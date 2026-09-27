use std::fs::{self, OpenOptions};
use std::io::Write;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::animation_persistence::{
    normalize_project_animation_counts, projects_equal_for_persistence,
    reconcile_project_animations, validate_animation_authoring,
};
use crate::scene::{Project, RawTransform, Scene};
use crate::texture::TextureList;
use crate::vtbf::{Block, OwnedBlock, OwnedProperty, OwnedSrdFile, Property, SrdFile};

/// A component of a parsed TRS record that can be safely changed in-place.
///
/// Transform edits preserve their existing VTBF records. Structural animation
/// edits use the owned-tree reconciler without rebuilding unrelated data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformComponent {
    Translation(usize),
    Rotation(usize),
    Scale(usize),
}

#[derive(Debug, Clone)]
pub struct SaveReport {
    pub path: PathBuf,
    pub bytes_written: usize,
}

#[derive(Debug, Clone)]
pub struct EditorDocument {
    path: Option<PathBuf>,
    /// Original source location used to resolve DDS, font, and other runtime
    /// assets. Loaded documents retain this across Save As; untitled documents
    /// adopt their first destination after a successful Save As.
    asset_path: PathBuf,
    /// Exact source backing the parsed VTBF tree. Writes begin from this data
    /// so blocks and fields outside an explicitly supported edit are lossless.
    source_bytes: Vec<u8>,
    pub file: SrdFile,
    /// The parsed state presented to the editor.
    pub project: Project,
    /// Parsed state matching `source_bytes`. It is advanced only after a
    /// verified, successful disk write and is the writeback comparison base.
    source_project: Project,
    /// Source SCN index retained by each edited scene. Newly authored scenes
    /// have no source index until a successful save establishes a new baseline.
    scene_origins: Vec<Option<usize>>,
    pub textures: TextureList,
    /// Parsed texture state matching `source_bytes`; texture authoring is not
    /// part of this persistence path and must remain structurally unchanged.
    source_textures: TextureList,
}

impl EditorDocument {
    /// Creates a valid, unsaved SRFF project with no scenes or external assets.
    pub fn empty() -> Self {
        let source_bytes = empty_project_bytes();
        let file = SrdFile::parse(source_bytes.clone())
            .expect("the canonical empty SRD must remain valid VTBF");
        let project =
            Project::from_file(&file).expect("the canonical empty SRD must remain a valid project");
        let textures = TextureList {
            declared_count: 0,
            textures: Vec::new(),
        };
        Self {
            path: None,
            asset_path: PathBuf::new(),
            source_bytes,
            file,
            source_project: project.clone(),
            scene_origins: Vec::new(),
            project,
            source_textures: textures.clone(),
            textures,
        }
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        let source_bytes = fs::read(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let file = SrdFile::parse(source_bytes.clone())
            .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
        let project = Project::from_file(&file)
            .map_err(|error| format!("failed to decode {} scene data: {error}", path.display()))?;
        let textures = TextureList::from_file(&file)
            .map_err(|error| format!("failed to decode {} texture list: {error}", path.display()))?
            .unwrap_or(TextureList {
                declared_count: 0,
                textures: Vec::new(),
            });
        Ok(Self {
            asset_path: path.clone(),
            path: Some(path),
            source_bytes,
            file,
            source_project: project.clone(),
            scene_origins: (0..project.scenes.len()).map(Some).collect(),
            project,
            source_textures: textures.clone(),
            textures,
        })
    }

    pub fn is_dirty(&self) -> bool {
        self.scene_structure_dirty()
            || !projects_equal_for_persistence(&self.project, &self.source_project)
            || self.textures != self.source_textures
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn asset_path(&self) -> &Path {
        &self.asset_path
    }

    pub(crate) fn scene_origins(&self) -> &[Option<usize>] {
        &self.scene_origins
    }

    pub(crate) fn push_scene(&mut self, scene: Scene) -> Result<usize, String> {
        let count = self
            .project
            .scenes
            .len()
            .checked_add(1)
            .ok_or_else(|| "SRD scene count overflow".to_owned())?;
        self.project.declared_scene_count =
            u32::try_from(count).map_err(|_| "SRD scene count exceeds u32".to_owned())?;
        self.project.scenes.push(scene);
        self.scene_origins.push(None);
        Ok(count - 1)
    }

    pub(crate) fn remove_scene(&mut self, index: usize) -> Option<Scene> {
        if index >= self.project.scenes.len() || index >= self.scene_origins.len() {
            return None;
        }
        self.scene_origins.remove(index);
        let scene = self.project.scenes.remove(index);
        self.project.declared_scene_count = self.project.scenes.len() as u32;
        Some(scene)
    }

    pub(crate) fn rename_scene(&mut self, index: usize, name: Vec<u8>) -> Result<bool, String> {
        validate_scene_name(&name)?;
        if self
            .project
            .scenes
            .iter()
            .enumerate()
            .any(|(other_index, scene)| other_index != index && scene.name == name)
        {
            return Err("another scene already uses that name".into());
        }
        let scene = self
            .project
            .scenes
            .get_mut(index)
            .ok_or_else(|| format!("scene {index} does not exist"))?;
        if scene.name == name {
            return Ok(false);
        }
        scene.name = name;
        Ok(true)
    }

    pub(crate) fn restore_scene_history(
        &mut self,
        project: Project,
        scene_origins: Vec<Option<usize>>,
    ) {
        assert_eq!(project.scenes.len(), scene_origins.len());
        self.project = project;
        self.scene_origins = scene_origins;
    }

    fn scene_structure_dirty(&self) -> bool {
        self.scene_origins.len() != self.source_project.scenes.len()
            || self
                .scene_origins
                .iter()
                .enumerate()
                .any(|(index, origin)| *origin != Some(index))
    }

    /// Ensures that an existing source property can represent a requested TRS
    /// component. This is called before the model mutates parsed state, which
    /// prevents an edit from becoming an unsaveable in-memory change.
    pub fn validate_transform_component(
        &self,
        scene_index: usize,
        layer_index: usize,
        node_index: usize,
        component: TransformComponent,
    ) -> Result<(), String> {
        let source_transform = self.source_transform(scene_index, layer_index, node_index)?;
        let (property_code, values, kind) = match component {
            TransformComponent::Translation(index) => {
                let values = match source_transform {
                    RawTransform::Trs2(_) if index < 2 => 2,
                    RawTransform::Trs3(_) if index < 3 => 3,
                    RawTransform::Trs2(_) => {
                        return Err("TRS2 has no serializable Z translation".into());
                    }
                    RawTransform::Trs3(_) => {
                        return Err(
                            "TRS3 has no serializable translation component at that index".into(),
                        );
                    }
                };
                (
                    if values == 2 { 0x34 } else { 0x37 },
                    values,
                    ValueKind::F32,
                )
            }
            TransformComponent::Rotation(index) => {
                let values = match source_transform {
                    RawTransform::Trs2(_) if index == 2 => 1,
                    RawTransform::Trs3(_) if index < 3 => 3,
                    RawTransform::Trs2(_) => {
                        return Err("TRS2 only serializes Z rotation".into());
                    }
                    RawTransform::Trs3(_) => {
                        return Err(
                            "TRS3 has no serializable rotation component at that index".into()
                        );
                    }
                };
                (
                    if values == 1 { 0x35 } else { 0x38 },
                    values,
                    ValueKind::I32,
                )
            }
            TransformComponent::Scale(index) => {
                let values = match source_transform {
                    RawTransform::Trs2(_) if index < 2 => 2,
                    RawTransform::Trs3(_) if index < 3 => 3,
                    RawTransform::Trs2(_) => {
                        return Err("TRS2 has no serializable Z scale".into());
                    }
                    RawTransform::Trs3(_) => {
                        return Err("TRS3 has no serializable scale component at that index".into());
                    }
                };
                (
                    if values == 2 { 0x36 } else { 0x39 },
                    values,
                    ValueKind::F32,
                )
            }
        };
        let property = record_property(
            &self.source_transform_record(scene_index, layer_index, node_index)?,
            property_code,
        )
        .ok_or_else(|| {
            format!("TRS record has no writable property {property_code:#04x} for this component")
        })?;
        validate_property(property, values, kind)
    }

    /// Validates both the VTBF slot and the prospective value. In particular,
    /// compact integer rotation fields reject values that would overflow on
    /// writeback instead of accepting an edit that Save could never encode.
    pub fn validate_transform_value(
        &self,
        scene_index: usize,
        layer_index: usize,
        node_index: usize,
        component: TransformComponent,
        value: f32,
    ) -> Result<(), String> {
        if !value.is_finite() {
            return Err("transform values must be finite".into());
        }
        self.validate_transform_component(scene_index, layer_index, node_index, component)?;
        if !matches!(component, TransformComponent::Rotation(_)) {
            return Ok(());
        }
        let rounded = value.round();
        if !(-2_147_483_648.0..2_147_483_648.0).contains(&rounded) {
            return Err("rotation is outside the SRD i32 range".into());
        }
        let property = record_property(
            &self.source_transform_record(scene_index, layer_index, node_index)?,
            match component {
                TransformComponent::Rotation(2)
                    if matches!(
                        self.source_transform(scene_index, layer_index, node_index)?,
                        RawTransform::Trs2(_)
                    ) =>
                {
                    0x35
                }
                TransformComponent::Rotation(_) => 0x38,
                _ => unreachable!("non-rotation returned above"),
            },
        )
        .ok_or_else(|| "TRS rotation property disappeared during validation".to_owned())?;
        validate_integer_value(property, rounded as i32)
    }

    /// Writes the selected source path. An unchanged source is explicitly not
    /// treated as a successful save: the editor did not serialize an edit.
    pub fn save(&mut self) -> Result<SaveReport, String> {
        let path = self
            .path
            .clone()
            .ok_or_else(|| "untitled SRD documents must be saved with Save As".to_owned())?;
        self.write_to(path, false)
    }

    /// Writes the document to a new path. A clean Save As is a deliberate,
    /// lossless copy operation; it preserves the exact source bytes and moves
    /// this document's path/baseline only after the copy succeeds.
    pub fn save_as(&mut self, path: impl AsRef<Path>) -> Result<SaveReport, String> {
        self.write_to(path.as_ref().to_path_buf(), true)
    }

    fn write_to(
        &mut self,
        path: PathBuf,
        allow_unchanged_copy: bool,
    ) -> Result<SaveReport, String> {
        if allow_unchanged_copy
            && !self.scene_structure_dirty()
            && self.textures == self.source_textures
            && projects_equal_for_persistence(&self.project, &self.source_project)
        {
            return self.copy_clean_source_to(path);
        }

        normalize_project_animation_counts(&mut self.project)?;
        if allow_unchanged_copy
            && !self.scene_structure_dirty()
            && self.textures == self.source_textures
            && projects_equal_for_persistence(&self.project, &self.source_project)
        {
            return self.copy_clean_source_to(path);
        }
        validate_animation_authoring(&self.source_project, &self.project, &self.scene_origins)?;
        let bytes = self.serialize_project_changes()?;
        if bytes == self.source_bytes && !allow_unchanged_copy {
            return Err(
                "no serializable SRD edits to write; refusing to report an unchanged source copy as saved"
                    .into(),
            );
        }
        if bytes == self.source_bytes && self.path.as_ref() == Some(&path) {
            return Err(
                "Save As needs a different destination when the SRD has no serialized edits".into(),
            );
        }

        let file = SrdFile::parse(bytes.clone())
            .map_err(|error| format!("refusing to write invalid VTBF output: {error}"))?;
        let written_project = Project::from_file(&file)
            .map_err(|error| format!("refusing to write unverifiable SRD output: {error}"))?;
        if !projects_equal_for_persistence(&written_project, &self.project) {
            return Err(
                "refusing to write: in-place SRD serialization did not round-trip the edited project"
                    .into(),
            );
        }

        write_atomically(&path, &bytes)?;
        self.adopt_saved_path(path.clone());
        self.source_bytes = bytes;
        self.file = file;
        self.source_textures = self.textures.clone();
        self.source_project = self.project.clone();
        self.scene_origins = (0..self.project.scenes.len()).map(Some).collect();
        Ok(SaveReport {
            path,
            bytes_written: self.source_bytes.len(),
        })
    }

    fn copy_clean_source_to(&mut self, path: PathBuf) -> Result<SaveReport, String> {
        if self.path.as_ref() == Some(&path) {
            return Err(
                "Save As needs a different destination when the SRD has no serialized edits".into(),
            );
        }
        write_atomically(&path, &self.source_bytes)?;
        self.adopt_saved_path(path.clone());
        Ok(SaveReport {
            path,
            bytes_written: self.source_bytes.len(),
        })
    }

    fn adopt_saved_path(&mut self, path: PathBuf) {
        if self.path.is_none() {
            self.asset_path.clone_from(&path);
        }
        self.path = Some(path);
    }

    fn serialize_project_changes(&self) -> Result<Vec<u8>, String> {
        ensure_only_supported_values_changed(
            &self.source_project,
            &self.project,
            &self.scene_origins,
        )?;
        if self.textures != self.source_textures {
            return Err("SRD writeback cannot mutate TEXL/TEX/CROP data".into());
        }
        if !self.scene_structure_dirty()
            && projects_equal_for_persistence(&self.project, &self.source_project)
        {
            return Ok(self.source_bytes.clone());
        }

        let mut bytes = self.source_bytes.clone();
        let mut string_patches = Vec::new();

        for (edited_scene_index, origin) in self.scene_origins.iter().copied().enumerate() {
            let Some(scene_index) = origin else {
                continue;
            };
            let source_scene = self.source_project.scenes.get(scene_index).ok_or_else(|| {
                format!("edited scene {edited_scene_index} has invalid source origin {scene_index}")
            })?;
            let edited_scene = self.project.scenes.get(edited_scene_index).ok_or_else(|| {
                format!("scene provenance has no edited scene {edited_scene_index}")
            })?;
            for (layer_index, (source_layer, edited_layer)) in source_scene
                .layers
                .iter()
                .zip(&edited_scene.layers)
                .enumerate()
            {
                let layer_block = source_layer_block(&self.file, scene_index, layer_index)?;
                if source_layer.name != edited_layer.name {
                    let property = layer_block
                        .last_property(0x03)
                        .ok_or_else(|| "source LAYR is missing name property 0x03".to_owned())?;
                    string_patches.push(StringPatch::new(
                        layer_block,
                        property,
                        &edited_layer.name,
                    )?);
                }
                patch_u32_field(
                    &mut bytes,
                    layer_block,
                    0x20,
                    source_layer.flags,
                    edited_layer.flags,
                )?;
                for (node_index, (source_node, edited_node)) in source_layer
                    .nodes
                    .iter()
                    .zip(&edited_layer.nodes)
                    .enumerate()
                {
                    let name_changed = source_node.name != edited_node.name;
                    let flags_changed = source_node.type_flags != edited_node.type_flags;
                    if !name_changed && !flags_changed {
                        continue;
                    }
                    let (node_block, record) =
                        self.source_node_record(scene_index, layer_index, node_index)?;
                    if name_changed {
                        let edited_name = edited_node.name.as_deref().ok_or_else(|| {
                            "SRD writeback cannot add or remove a NODE name".to_owned()
                        })?;
                        let property = record_property(&record, 0x03).ok_or_else(|| {
                            format!("source NODE {node_index} is missing name property 0x03")
                        })?;
                        string_patches.push(StringPatch::new(node_block, property, edited_name)?);
                    }
                    if flags_changed {
                        let (Some(_), Some(edited_flags)) =
                            (source_node.type_flags, edited_node.type_flags)
                        else {
                            return Err(
                                "SRD writeback cannot add or remove NODE property 0x30".into()
                            );
                        };
                        let property = record_property(&record, 0x30).ok_or_else(|| {
                            format!("source NODE {node_index} is missing flags property 0x30")
                        })?;
                        patch_u32_vector(&mut bytes, property, &[edited_flags])?;
                    }
                }
                for (node_index, (source, edited)) in source_layer
                    .transforms
                    .iter()
                    .zip(&edited_layer.transforms)
                    .enumerate()
                {
                    if source != edited {
                        let properties =
                            self.source_transform_record(scene_index, layer_index, node_index)?;
                        patch_transform(&mut bytes, properties, *source, *edited)?;
                    }
                }
                for (node_index, (source, edited)) in source_layer
                    .image_by_node
                    .iter()
                    .zip(&edited_layer.image_by_node)
                    .enumerate()
                {
                    let (Some(source), Some(edited)) = (source.as_ref(), edited.as_ref()) else {
                        if source != edited {
                            return Err("SRD writeback cannot add or remove CIMG records".into());
                        }
                        continue;
                    };
                    if source != edited {
                        let block = self.source_payload_block(
                            scene_index,
                            layer_index,
                            node_index,
                            b"CIMG",
                        )?;
                        patch_image_payload(&mut bytes, block, source, edited)?;
                    }
                    let source_text = source.text.as_ref();
                    let edited_text = edited.text.as_ref();
                    match (source_text, edited_text) {
                        (Some(source_text), Some(edited_text)) => {
                            if source_text.text != edited_text.text {
                                let (block, property) = self.source_text_property(
                                    scene_index,
                                    layer_index,
                                    node_index,
                                )?;
                                string_patches.push(StringPatch::new(
                                    block,
                                    property,
                                    &edited_text.text,
                                )?);
                            }
                        }
                        (None, None) => {}
                        _ => return Err("SRD writeback cannot add or remove a TEXT payload".into()),
                    }
                }
                for (node_index, (source, edited)) in source_layer
                    .csli_by_node
                    .iter()
                    .zip(&edited_layer.csli_by_node)
                    .enumerate()
                {
                    if source != edited {
                        let (Some(source), Some(edited)) = (source.as_ref(), edited.as_ref())
                        else {
                            return Err("SRD writeback cannot add or remove CSLI records".into());
                        };
                        let block = self.source_payload_block(
                            scene_index,
                            layer_index,
                            node_index,
                            b"CSLI",
                        )?;
                        patch_csli_payload(&mut bytes, block, source, edited)?;
                    }
                }
                for (node_index, (source, edited)) in source_layer
                    .number_by_node
                    .iter()
                    .zip(&edited_layer.number_by_node)
                    .enumerate()
                {
                    if source != edited {
                        let (Some(source), Some(edited)) = (source.as_ref(), edited.as_ref())
                        else {
                            return Err("SRD writeback cannot add or remove CNUM records".into());
                        };
                        let block = self.source_payload_block(
                            scene_index,
                            layer_index,
                            node_index,
                            b"CNUM",
                        )?;
                        patch_number_payload(&mut bytes, block, source, edited)?;
                    }
                }
                for (node_index, (source, edited)) in source_layer
                    .reference_by_node
                    .iter()
                    .zip(&edited_layer.reference_by_node)
                    .enumerate()
                {
                    if source != edited {
                        let (Some(source), Some(edited)) = (source.as_ref(), edited.as_ref())
                        else {
                            return Err("SRD writeback cannot add or remove CRFD records".into());
                        };
                        let block = self.source_payload_block(
                            scene_index,
                            layer_index,
                            node_index,
                            b"CRFD",
                        )?;
                        patch_reference_payload(
                            &mut bytes,
                            &mut string_patches,
                            block,
                            source,
                            edited,
                        )?;
                    }
                }
            }
        }

        string_patches.sort_unstable_by_key(|patch| std::cmp::Reverse(patch.range.start));
        for patch in string_patches {
            patch.apply(&mut bytes)?;
        }

        let patched = SrdFile::parse(bytes).map_err(|error| {
            format!("failed to parse patched SRD before animation reconciliation: {error}")
        })?;
        let mut owned = patched.to_owned();
        reconcile_project_animations(
            &mut owned,
            &self.source_project,
            &self.project,
            &self.scene_origins,
        )?;
        owned
            .encode()
            .map_err(|error| format!("failed to encode reconciled SRD: {error}"))
    }

    fn source_transform(
        &self,
        scene_index: usize,
        layer_index: usize,
        node_index: usize,
    ) -> Result<RawTransform, String> {
        self.source_project
            .scenes
            .get(scene_index)
            .and_then(|scene| scene.layers.get(layer_index))
            .and_then(|layer| layer.transforms.get(node_index))
            .copied()
            .ok_or_else(|| {
                format!(
                    "selected transform {scene_index}/{layer_index}/{node_index} is not in the source project"
                )
            })
    }

    fn source_transform_record(
        &self,
        scene_index: usize,
        layer_index: usize,
        node_index: usize,
    ) -> Result<Vec<&Property>, String> {
        let source_transform = self.source_transform(scene_index, layer_index, node_index)?;
        let layer = source_layer_block(&self.file, scene_index, layer_index)?;
        let cast = layer
            .children
            .iter()
            .rev()
            .find(|block| block.is_tag(b"CAST"))
            .ok_or_else(|| format!("source LAYR {scene_index}/{layer_index} has no CAST block"))?;
        let tag = match source_transform {
            RawTransform::Trs2(_) => b"TRS2",
            RawTransform::Trs3(_) => b"TRS3",
        };
        let transform = cast
            .children
            .iter()
            .rev()
            .find(|block| block.is_tag(tag))
            .ok_or_else(|| {
                format!(
                    "source CAST {scene_index}/{layer_index} has no {} block",
                    String::from_utf8_lossy(tag)
                )
            })?;
        transform_record(&transform.properties, node_index)
    }

    fn source_node_record(
        &self,
        scene_index: usize,
        layer_index: usize,
        node_index: usize,
    ) -> Result<(&Block, Vec<&Property>), String> {
        let layer = source_layer_block(&self.file, scene_index, layer_index)?;
        let node = layer
            .children
            .iter()
            .rev()
            .find(|block| block.is_tag(b"CAST"))
            .and_then(|cast| {
                cast.children
                    .iter()
                    .rev()
                    .find(|block| block.is_tag(b"NODE"))
            })
            .ok_or_else(|| format!("source LAYR {scene_index}/{layer_index} has no NODE block"))?;
        Ok((node, transform_record(&node.properties, node_index)?))
    }

    fn source_payload_block(
        &self,
        scene_index: usize,
        layer_index: usize,
        node_index: usize,
        tag: &[u8; 4],
    ) -> Result<&Block, String> {
        let layer = source_layer_block(&self.file, scene_index, layer_index)?;
        layer
            .children
            .iter()
            .rev()
            .find(|block| block.is_tag(b"CAST"))
            .into_iter()
            .flat_map(|cast| cast.children.iter().filter(|block| block.is_tag(b"DATA")))
            .flat_map(|data| data.children.iter())
            .filter(|block| block.is_tag(tag))
            .find(|block| {
                block
                    .last_property(0x51)
                    .and_then(|property| property.read_unsigned_scalar(&self.file))
                    == u32::try_from(node_index).ok()
            })
            .ok_or_else(|| {
                format!(
                    "source LAYR {scene_index}/{layer_index} has no {} for NODE {node_index}",
                    String::from_utf8_lossy(tag)
                )
            })
    }

    fn source_text_property(
        &self,
        scene_index: usize,
        layer_index: usize,
        node_index: usize,
    ) -> Result<(&Block, &Property), String> {
        let layer = source_layer_block(&self.file, scene_index, layer_index)?;
        let cast = layer
            .children
            .iter()
            .rev()
            .find(|block| block.is_tag(b"CAST"))
            .ok_or_else(|| format!("source LAYR {scene_index}/{layer_index} has no CAST block"))?;
        let image = cast
            .children
            .iter()
            .filter(|block| block.is_tag(b"DATA"))
            .flat_map(|data| &data.children)
            .filter(|block| block.is_tag(b"CIMG"))
            .find(|block| {
                block
                    .last_property(0x51)
                    .and_then(|property| property.read_unsigned_scalar(&self.file))
                    == u32::try_from(node_index).ok()
            })
            .ok_or_else(|| {
                format!("source CAST {scene_index}/{layer_index} has no CIMG for NODE {node_index}")
            })?;
        let text = image
            .children
            .iter()
            .rev()
            .find(|block| block.is_tag(b"TEXT"))
            .ok_or_else(|| format!("source CIMG for NODE {node_index} has no TEXT block"))?;
        let property = text
            .last_property(0x7a)
            .ok_or_else(|| format!("source TEXT for NODE {node_index} has no 0x7a content"))?;
        Ok((text, property))
    }
}

fn empty_project_bytes() -> Vec<u8> {
    let mut project = OwnedBlock::new(*b"PROJ");
    project.properties.push(OwnedProperty::u32(0x00, 0));
    let mut source = OwnedBlock::new(*b"SRCK");
    source.children.push(project);
    OwnedSrdFile {
        unknown_04: [0; 4],
        format: *b"SRFF",
        unknown_0e: [0; 2],
        blocks: vec![source],
        trailing: Vec::new(),
    }
    .encode()
    .expect("the canonical empty SRD tree must remain encodable")
}

#[derive(Debug, Clone, Copy)]
enum ValueKind {
    F32,
    I32,
}

struct StringPatch {
    block_size_offset: usize,
    range: Range<usize>,
    replacement: Vec<u8>,
}

impl StringPatch {
    fn new(block: &Block, property: &Property, value: &[u8]) -> Result<Self, String> {
        if property.type_code != 2 {
            return Err(format!(
                "TEXT property {:#04x} uses VTBF type {} rather than a string",
                property.code, property.type_code
            ));
        }
        let prefix = property
            .string_prefix
            .as_ref()
            .ok_or_else(|| "TEXT string property has no encoded length prefix".to_owned())?;
        let mut replacement = encode_vtbf_string_length(value.len())?;
        replacement.extend_from_slice(value);
        Ok(Self {
            block_size_offset: block.offset + 4,
            range: prefix.start..property.value.end,
            replacement,
        })
    }

    fn apply(self, bytes: &mut Vec<u8>) -> Result<(), String> {
        let removed = self.range.len();
        let added = self.replacement.len();
        let current_size = u32::from_le_bytes(
            bytes
                .get(self.block_size_offset..self.block_size_offset + 4)
                .ok_or_else(|| "string block size field is outside the source bytes".to_owned())?
                .try_into()
                .expect("four-byte range"),
        );
        let new_size = if added >= removed {
            current_size.checked_add(
                u32::try_from(added - removed)
                    .map_err(|_| "string growth does not fit the VTBF block size".to_owned())?,
            )
        } else {
            current_size.checked_sub(
                u32::try_from(removed - added)
                    .map_err(|_| "string shrink does not fit the VTBF block size".to_owned())?,
            )
        }
        .ok_or_else(|| "string edit overflows its VTBF block size".to_owned())?;
        bytes
            .get_mut(self.block_size_offset..self.block_size_offset + 4)
            .ok_or_else(|| "TEXT block size field is outside the source bytes".to_owned())?
            .copy_from_slice(&new_size.to_le_bytes());
        if self.range.end > bytes.len() {
            return Err("TEXT property range is outside the source bytes".into());
        }
        bytes.splice(self.range, self.replacement);
        Ok(())
    }
}

static TEMP_WRITE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Replaces a destination only after its complete new contents have reached a
/// same-directory temporary file. A failed write leaves the original SRD
/// intact; replacement has no truncate window on supported platforms.
fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .ok_or_else(|| format!("cannot write SRD without a file name: {}", path.display()))?;
    let existing_permissions = fs::metadata(path)
        .ok()
        .map(|metadata| metadata.permissions());
    let (temporary_path, mut temporary_file) = loop {
        let sequence = TEMP_WRITE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let mut temporary_name = file_name.to_os_string();
        temporary_name.push(format!(".srd-editor-{}-{sequence}.tmp", std::process::id()));
        let candidate = directory.join(temporary_name);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => break (candidate, file),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "failed to create temporary save file beside {}: {error}",
                    path.display()
                ));
            }
        }
    };
    if let Some(permissions) = existing_permissions
        && let Err(error) = fs::set_permissions(&temporary_path, permissions)
    {
        drop(temporary_file);
        let _ = fs::remove_file(&temporary_path);
        return Err(format!(
            "failed to preserve permissions while saving {}: {error}",
            path.display()
        ));
    }
    if let Err(error) = temporary_file.write_all(bytes) {
        drop(temporary_file);
        let _ = fs::remove_file(&temporary_path);
        return Err(format!(
            "failed to write temporary save for {}: {error}",
            path.display()
        ));
    }
    if let Err(error) = temporary_file.sync_all() {
        drop(temporary_file);
        let _ = fs::remove_file(&temporary_path);
        return Err(format!(
            "failed to flush temporary save for {}: {error}",
            path.display()
        ));
    }
    drop(temporary_file);
    if let Err(error) = replace_saved_file(&temporary_path, path) {
        let _ = fs::remove_file(&temporary_path);
        return Err(format!(
            "failed to replace {} with saved output: {error}",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn replace_saved_file(temporary_path: &Path, path: &Path) -> std::io::Result<()> {
    fs::rename(temporary_path, path)
}

#[cfg(target_os = "windows")]
fn replace_saved_file(temporary_path: &Path, path: &Path) -> std::io::Result<()> {
    if !path.exists() {
        return fs::rename(temporary_path, path);
    }

    use std::os::windows::ffi::OsStrExt;

    #[link(name = "Kernel32")]
    unsafe extern "system" {
        fn ReplaceFileW(
            replaced_file_name: *const u16,
            replacement_file_name: *const u16,
            backup_file_name: *const u16,
            replace_flags: u32,
            exclude: *mut std::ffi::c_void,
            reserved: *mut std::ffi::c_void,
        ) -> i32;
    }

    fn wide(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain(Some(0)).collect()
    }

    let destination = wide(path);
    let replacement = wide(temporary_path);
    let replaced = unsafe {
        ReplaceFileW(
            destination.as_ptr(),
            replacement.as_ptr(),
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if replaced == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn source_project_block(file: &SrdFile) -> Result<&Block, String> {
    file.blocks
        .iter()
        .filter(|block| block.is_tag(b"SRCK"))
        .flat_map(|srck| srck.children.iter().filter(|block| block.is_tag(b"PROJ")))
        .last()
        .ok_or_else(|| "source SRFF has no SRCK/PROJ block".into())
}

fn source_layer_block(
    file: &SrdFile,
    scene_index: usize,
    layer_index: usize,
) -> Result<&Block, String> {
    let scene = source_project_block(file)?
        .children
        .iter()
        .filter(|block| block.is_tag(b"SCN "))
        .nth(scene_index)
        .ok_or_else(|| format!("source SRD has no SCN at index {scene_index}"))?;
    scene
        .children
        .iter()
        .filter(|block| block.is_tag(b"LAYR"))
        .nth(layer_index)
        .ok_or_else(|| format!("source SRD scene {scene_index} has no LAYR at index {layer_index}"))
}

fn transform_record(properties: &[Property], node_index: usize) -> Result<Vec<&Property>, String> {
    let mut current = 0usize;
    let mut record = Vec::new();
    for property in properties {
        if property.code == 0xfe {
            if current == node_index {
                return Ok(record);
            }
            current = current
                .checked_add(1)
                .ok_or_else(|| "TRS record index overflow".to_owned())?;
            continue;
        }
        if current == node_index {
            record.push(property);
        }
    }
    if current == node_index {
        Ok(record)
    } else {
        Err(format!("source TRS has no record {node_index}"))
    }
}

fn record_property<'a>(properties: &[&'a Property], code: u8) -> Option<&'a Property> {
    properties
        .iter()
        .rev()
        .copied()
        .find(|property| property.code == code)
}

fn validate_property(property: &Property, values: usize, kind: ValueKind) -> Result<(), String> {
    match kind {
        ValueKind::F32 if property.type_code != 10 => Err(format!(
            "property {:#04x} uses VTBF type {} rather than an in-place f32 value",
            property.code, property.type_code
        )),
        ValueKind::I32 if !matches!(property.type_code, 1 | 3..=9 | 11 | 12) => Err(format!(
            "property {:#04x} uses unsupported VTBF integer type {}",
            property.code, property.type_code
        )),
        _ => {
            let width = match kind {
                ValueKind::F32 => 4,
                ValueKind::I32 => integer_width(property.type_code).ok_or_else(|| {
                    format!(
                        "property {:#04x} has unsupported VTBF integer type {}",
                        property.code, property.type_code
                    )
                })?,
            };
            let required = values
                .checked_mul(width)
                .ok_or_else(|| "TRS property length overflow".to_owned())?;
            if property.value.len() < required {
                Err(format!(
                    "property {:#04x} has {} value bytes; {required} are required",
                    property.code,
                    property.value.len()
                ))
            } else {
                Ok(())
            }
        }
    }
}

fn validate_integer_value(property: &Property, value: i32) -> Result<(), String> {
    let fits = match property.type_code {
        1 | 4 => u8::try_from(value).is_ok(),
        3 => i8::try_from(value).is_ok(),
        5 | 7 => i16::try_from(value).is_ok(),
        6 => u16::try_from(value).is_ok(),
        8 | 9 | 11 | 12 => true,
        _ => false,
    };
    if fits {
        Ok(())
    } else {
        Err(format!(
            "rotation value {value} does not fit source VTBF type {} property {:#04x}",
            property.type_code, property.code
        ))
    }
}

fn ensure_only_supported_values_changed(
    source: &Project,
    edited: &Project,
    scene_origins: &[Option<usize>],
) -> Result<(), String> {
    if scene_origins.len() != edited.scenes.len() {
        return Err("scene provenance does not match the edited scene count".into());
    }
    let structure_changed = scene_origins.len() != source.scenes.len()
        || scene_origins
            .iter()
            .enumerate()
            .any(|(index, origin)| *origin != Some(index));
    let expected_count = if structure_changed {
        u32::try_from(edited.scenes.len()).map_err(|_| "SRD scene count exceeds u32")?
    } else {
        source.declared_scene_count
    };
    if edited.declared_scene_count != expected_count {
        return Err("edited PROJ scene count does not match its scene structure".into());
    }

    let mut source_shell = source.clone();
    source_shell.scenes.clear();
    source_shell.declared_scene_count = 0;
    let mut edited_shell = edited.clone();
    edited_shell.scenes.clear();
    edited_shell.declared_scene_count = 0;
    if edited_shell != source_shell {
        return Err("scene authoring cannot change other project fields".into());
    }

    let mut seen = vec![false; source.scenes.len()];
    let mut retained_source = source.clone();
    retained_source.scenes.clear();
    let mut retained_edited = source.clone();
    retained_edited.scenes.clear();
    for (scene, origin) in edited.scenes.iter().zip(scene_origins.iter().copied()) {
        if let Some(source_index) = origin {
            let Some(used) = seen.get_mut(source_index) else {
                return Err(format!(
                    "scene provenance references missing source SCN {source_index}"
                ));
            };
            if std::mem::replace(used, true) {
                return Err(format!("scene provenance reuses source SCN {source_index}"));
            }
            retained_source
                .scenes
                .push(source.scenes[source_index].clone());
            retained_edited.scenes.push(scene.clone());
        } else {
            validate_new_scene(scene)?;
        }
    }
    retained_source.declared_scene_count = retained_source.scenes.len() as u32;
    retained_edited.declared_scene_count = retained_edited.scenes.len() as u32;
    let source = &retained_source;
    let mut normalized = retained_edited;
    for (source_scene, edited_scene) in source.scenes.iter().zip(&mut normalized.scenes) {
        validate_scene_name(&edited_scene.name)?;
        edited_scene.name.clone_from(&source_scene.name);
        if source_scene.layers.len() != edited_scene.layers.len() {
            return Err("SRD writeback cannot add, remove, or reorder layers".into());
        }
        // Structural animation authoring is permitted only in these direct
        // SCN/LAYR descendants. Reset it before the final equality guard so
        // every other Scene/Layer/CAST field remains protected.
        edited_scene.declared_animation_set_count = source_scene.declared_animation_set_count;
        edited_scene
            .animation_sets
            .clone_from(&source_scene.animation_sets);

        for (source_layer, edited_layer) in source_scene.layers.iter().zip(&mut edited_scene.layers)
        {
            if source_layer.transforms.len() != edited_layer.transforms.len()
                || source_layer.nodes.len() != edited_layer.nodes.len()
                || source_layer.image_by_node.len() != edited_layer.image_by_node.len()
                || source_layer.csli_by_node.len() != edited_layer.csli_by_node.len()
                || source_layer.number_by_node.len() != edited_layer.number_by_node.len()
                || source_layer.reference_by_node.len() != edited_layer.reference_by_node.len()
            {
                return Err("SRD writeback cannot add, remove, or reorder CAST records".into());
            }
            edited_layer.animation_count = source_layer.animation_count;
            edited_layer.animations.clone_from(&source_layer.animations);
            edited_layer.name.clone_from(&source_layer.name);
            edited_layer.flags = source_layer.flags;

            for (source_node, edited_node) in source_layer.nodes.iter().zip(&mut edited_layer.nodes)
            {
                match (source_node.type_flags, edited_node.type_flags) {
                    (Some(source), Some(edited)) if (source ^ edited) & !0x100 == 0 => {}
                    (Some(_), Some(_)) => {
                        return Err(
                            "SRD writeback can only change NODE property 0x30 active bit 0x100"
                                .into(),
                        );
                    }
                    (None, None) => {}
                    _ => {
                        return Err("SRD writeback cannot add or remove NODE property 0x30".into());
                    }
                }
                edited_node.name.clone_from(&source_node.name);
                edited_node.type_flags = source_node.type_flags;
            }
            for (source_transform, edited_transform) in source_layer
                .transforms
                .iter()
                .zip(&mut edited_layer.transforms)
            {
                if std::mem::discriminant(source_transform)
                    != std::mem::discriminant(edited_transform)
                {
                    return Err("SRD writeback cannot change TRS record kinds".into());
                }
                *edited_transform = *source_transform;
            }
            for (source_image, edited_image) in source_layer
                .image_by_node
                .iter()
                .zip(&mut edited_layer.image_by_node)
            {
                let (source_image, edited_image) = match (source_image, edited_image) {
                    (Some(source), Some(edited)) => (source, edited),
                    (None, None) => continue,
                    _ => return Err("SRD writeback cannot add or remove CIMG records".into()),
                };
                edited_image.flags = source_image.flags;
                edited_image.width = source_image.width;
                edited_image.height = source_image.height;
                edited_image.custom_origin = source_image.custom_origin;
                edited_image.origin_mode = source_image.origin_mode;
                edited_image.vertex_colors = source_image.vertex_colors;
                edited_image.cref_index = source_image.cref_index;
                edited_image.cre1_index = source_image.cre1_index;
                edited_image.coordinate_offsets = source_image.coordinate_offsets;
                match (source_image.text.as_ref(), edited_image.text.as_mut()) {
                    (Some(source), Some(edited)) => {
                        if edited.text.len() > 2047 {
                            return Err("TEXT content exceeds the runtime 2047-byte limit".into());
                        }
                        edited.text.clone_from(&source.text);
                        edited.field_78 = source.field_78;
                        edited.font_index = source.font_index;
                    }
                    (None, None) => {}
                    _ => return Err("SRD writeback cannot add or remove TEXT payloads".into()),
                }
            }
            for (source, edited) in source_layer
                .csli_by_node
                .iter()
                .zip(&mut edited_layer.csli_by_node)
            {
                let (source, edited) = match (source, edited) {
                    (Some(source), Some(edited)) => (source, edited),
                    (None, None) => continue,
                    _ => return Err("SRD writeback cannot add or remove CSLI records".into()),
                };
                edited.field_80 = source.field_80;
                edited.width = source.width;
                edited.height = source.height;
                edited.custom_origin = source.custom_origin;
                edited.field_44 = source.field_44;
                edited.origin_mode = source.origin_mode;
            }
            for (source, edited) in source_layer
                .number_by_node
                .iter()
                .zip(&mut edited_layer.number_by_node)
            {
                let (source, edited) = match (source, edited) {
                    (Some(source), Some(edited)) => (source, edited),
                    (None, None) => continue,
                    _ => return Err("SRD writeback cannot add or remove CNUM records".into()),
                };
                edited.flags = source.flags;
                edited.width = source.width;
                edited.height = source.height;
                edited.custom_origin = source.custom_origin;
                edited.origin_mode = source.origin_mode;
                edited.vertex_colors = source.vertex_colors;
                edited.format_flags = source.format_flags;
                edited.initial_integer = source.initial_integer;
                edited.initial_fraction = source.initial_fraction;
            }
            for (source, edited) in source_layer
                .reference_by_node
                .iter()
                .zip(&mut edited_layer.reference_by_node)
            {
                let (source, edited) = match (source, edited) {
                    (Some(source), Some(edited)) => (source, edited),
                    (None, None) => continue,
                    _ => return Err("SRD writeback cannot add or remove CRFD records".into()),
                };
                edited.source_name.clone_from(&source.source_name);
                edited.layer_name.clone_from(&source.layer_name);
                edited.animation_enabled = source.animation_enabled;
                edited.animation_name.clone_from(&source.animation_name);
                edited.default_frame = source.default_frame;
            }
        }
    }
    if !projects_equal_for_persistence(&normalized, source) {
        return Err(
            "SRD writeback cannot serialize a change outside animations, Inspector names and active states, existing TRS fields, existing CIMG/CSLI/CNUM/CRFD payload fields, and existing TEXT fields"
                .into(),
        );
    }
    Ok(())
}

fn validate_new_scene(scene: &Scene) -> Result<(), String> {
    if scene.declared_layer_count != 0 || !scene.layers.is_empty() {
        return Err("new scenes must remain layerless until layer authoring is implemented".into());
    }
    if scene.declared_animation_set_count != scene.animation_sets.len() as u32 {
        return Err("new scene ANMS count does not match its animation sets".into());
    }
    validate_scene_name(&scene.name)?;
    if !scene.width.is_finite()
        || !scene.height.is_finite()
        || scene.width <= 0.0
        || scene.height <= 0.0
        || scene.width.fract() != 0.0
        || scene.height.fract() != 0.0
    {
        return Err("new scenes need positive integral composition dimensions".into());
    }
    Ok(())
}

fn validate_scene_name(name: &[u8]) -> Result<(), String> {
    if name.is_empty() || name.len() > 64 || name.contains(&0) {
        return Err("scene names must contain 1–64 non-NUL bytes".into());
    }
    Ok(())
}

fn encode_vtbf_string_length(length: usize) -> Result<Vec<u8>, String> {
    if length < 0x80 {
        return Ok(vec![length as u8]);
    }
    if length > 0x7fff {
        return Err("VTBF string content exceeds the 32767-byte encoding limit".into());
    }
    Ok(vec![0x80 | ((length >> 8) as u8), length as u8])
}

fn patch_transform(
    bytes: &mut [u8],
    properties: Vec<&Property>,
    source: RawTransform,
    edited: RawTransform,
) -> Result<(), String> {
    match (source, edited) {
        (RawTransform::Trs2(source), RawTransform::Trs2(edited)) => {
            if source.translation[..2] != edited.translation[..2] {
                patch_f32_vector(
                    bytes,
                    record_property(&properties, 0x34)
                        .ok_or_else(|| "TRS2 is missing position property 0x34".to_owned())?,
                    &edited.translation[..2],
                )?;
            }
            if source.rotation[2] != edited.rotation[2] {
                patch_i32_vector(
                    bytes,
                    record_property(&properties, 0x35)
                        .ok_or_else(|| "TRS2 is missing rotation property 0x35".to_owned())?,
                    &edited.rotation[2..3],
                )?;
            }
            if source.scale[..2] != edited.scale[..2] {
                patch_f32_vector(
                    bytes,
                    record_property(&properties, 0x36)
                        .ok_or_else(|| "TRS2 is missing scale property 0x36".to_owned())?,
                    &edited.scale[..2],
                )?;
            }
            patch_spatial_fields(bytes, &properties, &source, &edited)?;
        }
        (RawTransform::Trs3(source), RawTransform::Trs3(edited)) => {
            if source.translation != edited.translation {
                patch_f32_vector(
                    bytes,
                    record_property(&properties, 0x37)
                        .ok_or_else(|| "TRS3 is missing position property 0x37".to_owned())?,
                    &edited.translation,
                )?;
            }
            if source.rotation != edited.rotation {
                patch_i32_vector(
                    bytes,
                    record_property(&properties, 0x38)
                        .ok_or_else(|| "TRS3 is missing rotation property 0x38".to_owned())?,
                    &edited.rotation,
                )?;
            }
            if source.scale != edited.scale {
                patch_f32_vector(
                    bytes,
                    record_property(&properties, 0x39)
                        .ok_or_else(|| "TRS3 is missing scale property 0x39".to_owned())?,
                    &edited.scale,
                )?;
            }
            patch_spatial_fields(bytes, &properties, &source, &edited)?;
        }
        _ => return Err("SRD writeback cannot change TRS record kinds".into()),
    }
    Ok(())
}
fn required_block_property(block: &Block, code: u8) -> Result<&Property, String> {
    block.last_property(code).ok_or_else(|| {
        format!(
            "source {} block is missing property {code:#04x}",
            String::from_utf8_lossy(&block.tag)
        )
    })
}

fn patch_f32_field(
    bytes: &mut [u8],
    block: &Block,
    code: u8,
    source: f32,
    edited: f32,
) -> Result<(), String> {
    if source != edited {
        patch_f32_vector(bytes, required_block_property(block, code)?, &[edited])?;
    }
    Ok(())
}

fn patch_i32_field(
    bytes: &mut [u8],
    block: &Block,
    code: u8,
    source: i32,
    edited: i32,
) -> Result<(), String> {
    if source != edited {
        patch_i32_vector(bytes, required_block_property(block, code)?, &[edited])?;
    }
    Ok(())
}

fn patch_u32_field(
    bytes: &mut [u8],
    block: &Block,
    code: u8,
    source: u32,
    edited: u32,
) -> Result<(), String> {
    if source != edited {
        patch_u32_vector(bytes, required_block_property(block, code)?, &[edited])?;
    }
    Ok(())
}

fn patch_common_image_fields(
    bytes: &mut [u8],
    block: &Block,
    flags_code: u8,
    source_size: [f32; 2],
    edited_size: [f32; 2],
    source_origin: [f32; 2],
    edited_origin: [f32; 2],
    source_flags: u32,
    edited_flags: u32,
    source_mode: u8,
    edited_mode: u8,
    source_colors: [[u8; 4]; 4],
    edited_colors: [[u8; 4]; 4],
) -> Result<(), String> {
    patch_f32_field(bytes, block, 0x40, source_size[0], edited_size[0])?;
    patch_f32_field(bytes, block, 0x41, source_size[1], edited_size[1])?;
    patch_f32_field(bytes, block, 0x42, source_origin[0], edited_origin[0])?;
    patch_f32_field(bytes, block, 0x43, source_origin[1], edited_origin[1])?;
    patch_u32_field(bytes, block, flags_code, source_flags, edited_flags)?;
    patch_u32_field(
        bytes,
        block,
        0x4b,
        u32::from(source_mode),
        u32::from(edited_mode),
    )?;
    for (index, (source, edited)) in source_colors.into_iter().zip(edited_colors).enumerate() {
        if source != edited {
            let property = block
                .properties_with_code(0x44)
                .nth(index)
                .ok_or_else(|| format!("source payload has no vertex color {index}"))?;
            patch_payload_color(bytes, property, edited)?;
        }
    }
    Ok(())
}

fn patch_image_payload(
    bytes: &mut [u8],
    block: &Block,
    source: &crate::image::ImageDefinition,
    edited: &crate::image::ImageDefinition,
) -> Result<(), String> {
    patch_common_image_fields(
        bytes,
        block,
        0x49,
        [source.width, source.height],
        [edited.width, edited.height],
        source.custom_origin,
        edited.custom_origin,
        source.flags,
        edited.flags,
        source.origin_mode,
        edited.origin_mode,
        source.vertex_colors,
        edited.vertex_colors,
    )?;
    patch_i32_field(
        bytes,
        block,
        0x46,
        i32::from(source.cref_index),
        i32::from(edited.cref_index),
    )?;
    patch_i32_field(
        bytes,
        block,
        0x4e,
        i32::from(source.cre1_index),
        i32::from(edited.cre1_index),
    )?;
    for (code, source, edited) in [
        (
            0x83,
            source.coordinate_offsets[0][0],
            edited.coordinate_offsets[0][0],
        ),
        (
            0x84,
            source.coordinate_offsets[1][0],
            edited.coordinate_offsets[1][0],
        ),
        (
            0x85,
            source.coordinate_offsets[0][1],
            edited.coordinate_offsets[0][1],
        ),
        (
            0x86,
            source.coordinate_offsets[1][1],
            edited.coordinate_offsets[1][1],
        ),
    ] {
        patch_f32_field(bytes, block, code, source, edited)?;
    }
    if let (Some(source), Some(edited)) = (source.text.as_ref(), edited.text.as_ref()) {
        let text = block
            .children
            .iter()
            .rev()
            .find(|child| child.is_tag(b"TEXT"))
            .ok_or_else(|| "source CIMG has no TEXT child".to_owned())?;
        if source.field_78 != edited.field_78 {
            let (Some(source), Some(edited)) = (source.field_78, edited.field_78) else {
                return Err("SRD writeback cannot add or remove TEXT property 0x78".into());
            };
            patch_u32_field(bytes, text, 0x78, source, edited)?;
        }
        if source.font_index != edited.font_index {
            let (Some(source), Some(edited)) = (source.font_index, edited.font_index) else {
                return Err("SRD writeback cannot add or remove TEXT property 0x79".into());
            };
            patch_i32_field(bytes, text, 0x79, source, edited)?;
        }
    }
    Ok(())
}

fn patch_csli_payload(
    bytes: &mut [u8],
    block: &Block,
    source: &crate::csli::CsliDefinition,
    edited: &crate::csli::CsliDefinition,
) -> Result<(), String> {
    patch_common_image_fields(
        bytes,
        block,
        0x80,
        [source.width, source.height],
        [edited.width, edited.height],
        source.custom_origin,
        edited.custom_origin,
        source.field_80,
        edited.field_80,
        source.origin_mode,
        edited.origin_mode,
        source.field_44,
        edited.field_44,
    )
}

fn patch_number_payload(
    bytes: &mut [u8],
    block: &Block,
    source: &crate::number::NumberDefinition,
    edited: &crate::number::NumberDefinition,
) -> Result<(), String> {
    patch_common_image_fields(
        bytes,
        block,
        0x49,
        [source.width, source.height],
        [edited.width, edited.height],
        source.custom_origin,
        edited.custom_origin,
        source.flags,
        edited.flags,
        source.origin_mode,
        edited.origin_mode,
        source.vertex_colors,
        edited.vertex_colors,
    )?;
    patch_u32_field(bytes, block, 0x80, source.format_flags, edited.format_flags)?;
    patch_i32_field(
        bytes,
        block,
        0x81,
        source.initial_integer,
        edited.initial_integer,
    )?;
    patch_f32_field(
        bytes,
        block,
        0x82,
        source.initial_fraction,
        edited.initial_fraction,
    )
}

fn patch_reference_payload(
    bytes: &mut [u8],
    string_patches: &mut Vec<StringPatch>,
    block: &Block,
    source: &crate::reference::ReferenceDefinition,
    edited: &crate::reference::ReferenceDefinition,
) -> Result<(), String> {
    for (code, source, edited) in [
        (
            0x80,
            source.source_name.as_slice(),
            edited.source_name.as_slice(),
        ),
        (
            0x81,
            source.layer_name.as_slice(),
            edited.layer_name.as_slice(),
        ),
        (
            0x83,
            source.animation_name.as_slice(),
            edited.animation_name.as_slice(),
        ),
    ] {
        if source != edited {
            string_patches.push(StringPatch::new(
                block,
                required_block_property(block, code)?,
                edited,
            )?);
        }
    }
    patch_u32_field(
        bytes,
        block,
        0x82,
        source.animation_enabled,
        edited.animation_enabled,
    )?;
    patch_f32_field(
        bytes,
        block,
        0x84,
        source.default_frame,
        edited.default_frame,
    )
}

fn patch_spatial_fields(
    bytes: &mut [u8],
    properties: &[&Property],
    source: &crate::transform::SpatialTransform,
    edited: &crate::transform::SpatialTransform,
) -> Result<(), String> {
    if source.multiply_color != edited.multiply_color {
        patch_runtime_color(
            bytes,
            record_property(properties, 0x3a)
                .ok_or_else(|| "TRS is missing multiply-color property 0x3a".to_owned())?,
            edited.multiply_color,
        )?;
    }
    if source.additive_color != edited.additive_color {
        patch_runtime_color(
            bytes,
            record_property(properties, 0x33)
                .ok_or_else(|| "TRS is missing additive-color property 0x33".to_owned())?,
            edited.additive_color,
        )?;
    }
    if source.visibility_word != edited.visibility_word {
        patch_i32_vector(
            bytes,
            record_property(properties, 0x3b)
                .ok_or_else(|| "TRS is missing visibility property 0x3b".to_owned())?,
            &[i32::from(edited.visibility_word != 0)],
        )?;
    }
    Ok(())
}

fn patch_runtime_color(
    bytes: &mut [u8],
    property: &Property,
    color: [u8; 4],
) -> Result<(), String> {
    let output = bytes
        .get_mut(property.value.start..property.value.start + 4)
        .ok_or_else(|| "TRS color patch is outside the source bytes".to_owned())?;
    output.copy_from_slice(&[color[3], color[2], color[1], color[0]]);
    Ok(())
}

fn patch_payload_color(
    bytes: &mut [u8],
    property: &Property,
    color: [u8; 4],
) -> Result<(), String> {
    let output = bytes
        .get_mut(property.value.start..property.value.start + 4)
        .ok_or_else(|| "payload color patch is outside the source bytes".to_owned())?;
    output.copy_from_slice(&[color[3], color[0], color[1], color[2]]);
    Ok(())
}

fn patch_f32_vector(bytes: &mut [u8], property: &Property, values: &[f32]) -> Result<(), String> {
    validate_property(property, values.len(), ValueKind::F32)?;
    for (index, value) in values.iter().enumerate() {
        if !value.is_finite() {
            return Err(format!(
                "property {:#04x} cannot serialize a non-finite transform value",
                property.code
            ));
        }
        let offset = property.value.start + index * 4;
        bytes
            .get_mut(offset..offset + 4)
            .ok_or_else(|| "TRS f32 patch is outside the source bytes".to_owned())?
            .copy_from_slice(&value.to_le_bytes());
    }
    Ok(())
}

fn patch_i32_vector(bytes: &mut [u8], property: &Property, values: &[i32]) -> Result<(), String> {
    validate_property(property, values.len(), ValueKind::I32)?;
    let width = integer_width(property.type_code).expect("validated integer type");
    for (index, value) in values.iter().enumerate() {
        let offset = property.value.start + index * width;
        let output = bytes
            .get_mut(offset..offset + width)
            .ok_or_else(|| "TRS integer patch is outside the source bytes".to_owned())?;
        match property.type_code {
            1 | 4 => {
                let value = u8::try_from(*value).map_err(|_| {
                    format!(
                        "rotation value {value} does not fit source VTBF u8 property {:#04x}",
                        property.code
                    )
                })?;
                output[0] = value;
            }
            3 => {
                let value = i8::try_from(*value).map_err(|_| {
                    format!(
                        "rotation value {value} does not fit source VTBF i8 property {:#04x}",
                        property.code
                    )
                })?;
                output[0] = value as u8;
            }
            5 | 7 => {
                let value = i16::try_from(*value).map_err(|_| {
                    format!(
                        "rotation value {value} does not fit source VTBF i16 property {:#04x}",
                        property.code
                    )
                })?;
                output.copy_from_slice(&value.to_le_bytes());
            }
            6 => {
                let value = u16::try_from(*value).map_err(|_| {
                    format!(
                        "rotation value {value} does not fit source VTBF u16 property {:#04x}",
                        property.code
                    )
                })?;
                output.copy_from_slice(&value.to_le_bytes());
            }
            8 | 9 | 11 | 12 => output.copy_from_slice(&value.to_le_bytes()),
            _ => unreachable!("validated integer type"),
        }
    }
    Ok(())
}

fn patch_u32_vector(bytes: &mut [u8], property: &Property, values: &[u32]) -> Result<(), String> {
    validate_property(property, values.len(), ValueKind::I32)?;
    let width = integer_width(property.type_code).expect("validated integer type");
    for (index, value) in values.iter().enumerate() {
        let offset = property.value.start + index * width;
        let output = bytes
            .get_mut(offset..offset + width)
            .ok_or_else(|| "unsigned integer patch is outside the source bytes".to_owned())?;
        match property.type_code {
            1 | 4 => {
                output[0] = u8::try_from(*value).map_err(|_| {
                    format!(
                        "value {value} does not fit VTBF u8 property {:#04x}",
                        property.code
                    )
                })?
            }
            3 => {
                output[0] = i8::try_from(*value).map_err(|_| {
                    format!(
                        "value {value} does not fit VTBF i8 property {:#04x}",
                        property.code
                    )
                })? as u8
            }
            5 | 7 => output.copy_from_slice(
                &i16::try_from(*value)
                    .map_err(|_| {
                        format!(
                            "value {value} does not fit VTBF i16 property {:#04x}",
                            property.code
                        )
                    })?
                    .to_le_bytes(),
            ),
            6 => output.copy_from_slice(
                &u16::try_from(*value)
                    .map_err(|_| {
                        format!(
                            "value {value} does not fit VTBF u16 property {:#04x}",
                            property.code
                        )
                    })?
                    .to_le_bytes(),
            ),
            8 | 9 | 11 | 12 => output.copy_from_slice(&value.to_le_bytes()),
            _ => unreachable!("validated integer type"),
        }
    }
    Ok(())
}

fn integer_width(type_code: u8) -> Option<usize> {
    match type_code {
        1 | 3 | 4 => Some(1),
        5..=7 => Some(2),
        8 | 9 | 11 | 12 => Some(4),
        _ => None,
    }
}

pub fn display_srd_name(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::animation::{AnimationDefinition, Key8, Key20, KeyData, Motion, Track};
    use crate::scene::{AnimationSetDefinition, Scene, SceneAnimationSlot};
    use crate::vtbf::{OwnedBlock, OwnedProperty, OwnedSrdFile};

    #[test]
    fn empty_document_is_valid_unsaved_srd() {
        let mut document = EditorDocument::empty();

        assert_eq!(document.path(), None);
        assert_eq!(document.asset_path(), Path::new(""));
        assert!(document.project.scenes.is_empty());
        assert_eq!(document.project.declared_scene_count, 0);
        assert!(!document.is_dirty());
        assert_eq!(
            SrdFile::parse(document.source_bytes.clone())
                .unwrap()
                .blocks
                .len(),
            1
        );
        assert!(document.save().unwrap_err().contains("Save As"));
    }

    #[test]
    fn first_scene_save_as_round_trips_from_empty_document() {
        let path = temporary_srd_path("first-scene-save-as");
        let mut document = EditorDocument::empty();
        document
            .push_scene(Scene {
                name: b"Scene 1".to_vec(),
                declared_layer_count: 0,
                declared_animation_set_count: 0,
                width: 1920.0,
                height: 1080.0,
                layers: Vec::new(),
                animation_sets: Vec::new(),
            })
            .unwrap();

        assert!(document.is_dirty());
        let report = document.save_as(&path).unwrap();
        assert_eq!(report.path, path);
        assert_eq!(document.path(), Some(path.as_path()));
        assert_eq!(document.asset_path(), path);
        assert!(!document.is_dirty());

        let reloaded = EditorDocument::load(&path).unwrap();
        assert_eq!(reloaded.project, document.project);
        assert_eq!(reloaded.project.scenes.len(), 1);
        assert!(reloaded.project.scenes[0].layers.is_empty());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn scene_add_rename_and_delete_round_trip_existing_document() {
        let path = temporary_srd_path("scene-structural-round-trip");
        let source = minimal_trs2_srd();
        fs::write(&path, &source).unwrap();
        let mut document = EditorDocument::load(&path).unwrap();
        let retained_layer_count = document.project.scenes[0].layers.len();

        document
            .rename_scene(0, b"Renamed Source".to_vec())
            .unwrap();
        document
            .push_scene(Scene {
                name: b"Authored Scene".to_vec(),
                declared_layer_count: 0,
                declared_animation_set_count: 0,
                width: 1280.0,
                height: 720.0,
                layers: Vec::new(),
                animation_sets: Vec::new(),
            })
            .unwrap();
        assert_eq!(document.scene_origins(), &[Some(0), None]);
        document.save().unwrap();

        let mut reloaded = EditorDocument::load(&path).unwrap();
        assert_eq!(reloaded.project.declared_scene_count, 2);
        assert_eq!(reloaded.project.scenes.len(), 2);
        assert_eq!(reloaded.project.scenes[0].name, b"Renamed Source");
        assert_eq!(
            reloaded.project.scenes[0].layers.len(),
            retained_layer_count
        );
        assert_eq!(reloaded.project.scenes[1].name, b"Authored Scene");
        assert_eq!(reloaded.project.scenes[1].width, 1280.0);

        reloaded.remove_scene(0).unwrap();
        reloaded.save().unwrap();
        let mut reloaded = EditorDocument::load(&path).unwrap();
        assert_eq!(reloaded.project.declared_scene_count, 1);
        assert_eq!(reloaded.project.scenes.len(), 1);
        assert_eq!(reloaded.project.scenes[0].name, b"Authored Scene");

        reloaded.remove_scene(0).unwrap();
        reloaded.save().unwrap();
        let reloaded = EditorDocument::load(&path).unwrap();
        assert_eq!(reloaded.project.declared_scene_count, 0);
        assert!(reloaded.project.scenes.is_empty());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn writes_existing_trs_values_without_rewriting_opaque_source() {
        let path = temporary_srd_path("write-transform");
        let source = minimal_trs2_srd();
        fs::write(&path, &source).unwrap();

        let mut document = EditorDocument::load(&path).unwrap();
        let RawTransform::Trs2(transform) = &mut document.project.scenes[0].layers[0].transforms[0]
        else {
            panic!("fixture must contain TRS2");
        };
        transform.translation[0] = 42.5;
        assert!(document.is_dirty());

        let report = document.save().unwrap();
        assert_eq!(report.path, path);
        assert!(!document.is_dirty());
        let written = fs::read(&path).unwrap();
        assert_ne!(written, source);
        assert!(written.ends_with(b"opaque source data"));
        let loaded = EditorDocument::load(&path).unwrap();
        assert_eq!(
            loaded.project.scenes[0].layers[0].transforms[0]
                .spatial()
                .translation[0],
            42.5
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn resizes_text_content_and_preserves_transform_and_opaque_bytes() {
        let path = temporary_srd_path("write-text");
        let source = minimal_text_srd(b"Hi");
        fs::write(&path, &source).unwrap();

        let mut document = EditorDocument::load(&path).unwrap();
        document.project.scenes[0].layers[0].image_by_node[0]
            .as_mut()
            .unwrap()
            .text
            .as_mut()
            .unwrap()
            .text = vec![b'X'; 180];
        let RawTransform::Trs2(transform) = &mut document.project.scenes[0].layers[0].transforms[0]
        else {
            panic!("fixture must contain TRS2");
        };
        transform.translation[1] = 24.0;
        document.save().unwrap();

        let written = fs::read(&path).unwrap();
        assert!(written.ends_with(b"opaque source data"));
        let mut loaded = EditorDocument::load(&path).unwrap();
        assert_eq!(
            loaded.project.scenes[0].layers[0].image_by_node[0]
                .as_ref()
                .unwrap()
                .text
                .as_ref()
                .unwrap()
                .text,
            vec![b'X'; 180]
        );
        assert_eq!(
            loaded.project.scenes[0].layers[0].transforms[0]
                .spatial()
                .translation[1],
            24.0
        );

        loaded.project.scenes[0].layers[0].image_by_node[0]
            .as_mut()
            .unwrap()
            .text
            .as_mut()
            .unwrap()
            .text = b"OK".to_vec();
        loaded.save().unwrap();
        let shrunk = EditorDocument::load(&path).unwrap();
        assert_eq!(
            shrunk.project.scenes[0].layers[0].image_by_node[0]
                .as_ref()
                .unwrap()
                .text
                .as_ref()
                .unwrap()
                .text,
            b"OK"
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn persists_existing_layer_and_cast_names() {
        let path = temporary_srd_path("inspector-names");
        fs::write(&path, minimal_two_layer_two_node_trs2_srd()).unwrap();
        let mut document = EditorDocument::load(&path).unwrap();
        let layer = &mut document.project.scenes[0].layers[0];
        layer.name = b"renamed_layer".to_vec();
        layer.nodes[0].name = Some(b"renamed_cast".to_vec());

        document.save().unwrap();
        let reloaded = EditorDocument::load(&path).unwrap();
        let layer = &reloaded.project.scenes[0].layers[0];
        assert_eq!(layer.name, b"renamed_layer");
        assert_eq!(
            layer.nodes[0].name.as_deref(),
            Some(b"renamed_cast".as_slice())
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_trs2_components_that_have_no_serialized_field() {
        let path = temporary_srd_path("unsupported-trs2-component");
        let source = minimal_trs2_srd();
        fs::write(&path, &source).unwrap();
        let mut document = EditorDocument::load(&path).unwrap();
        let RawTransform::Trs2(transform) = &mut document.project.scenes[0].layers[0].transforms[0]
        else {
            panic!("fixture must contain TRS2");
        };
        transform.translation[2] = 99.0;

        let error = document.save().unwrap_err();
        assert!(error.contains("did not round-trip the edited project"));
        assert_eq!(fs::read(&path).unwrap(), source);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn clean_save_as_copies_exact_source_and_moves_document_path() {
        let source_path = temporary_srd_path("clean-save-as-source");
        let copy_path = temporary_srd_path("clean-save-as-copy");
        let source = minimal_trs2_srd();
        fs::write(&source_path, &source).unwrap();

        let mut document = EditorDocument::load(&source_path).unwrap();
        let report = document.save_as(&copy_path).unwrap();

        assert_eq!(report.path, copy_path);
        assert_eq!(document.path(), Some(copy_path.as_path()));
        assert_eq!(document.asset_path(), source_path);
        assert_eq!(document.source_bytes, source);
        assert_eq!(document.file.bytes(), source);
        assert!(!document.is_dirty());
        assert_eq!(fs::read(&copy_path).unwrap(), source);
        assert!(
            document
                .save()
                .unwrap_err()
                .contains("no serializable SRD edits")
        );

        fs::remove_file(source_path).unwrap();
        fs::remove_file(copy_path).unwrap();
    }

    #[test]
    fn rejects_unrepresentable_changes_without_touching_the_source_file() {
        let path = temporary_srd_path("unsupported-change");
        let source = minimal_trs2_srd();
        fs::write(&path, &source).unwrap();

        let mut document = EditorDocument::load(&path).unwrap();
        document.project.scenes[0].width = 640.0;
        let error = document.save().unwrap_err();
        assert!(error.contains("cannot serialize a change outside"));
        assert_eq!(fs::read(&path).unwrap(), source);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn refuses_to_claim_an_unchanged_source_was_saved() {
        let path = temporary_srd_path("unchanged-source");
        fs::write(&path, minimal_trs2_srd()).unwrap();
        let mut document = EditorDocument::load(&path).unwrap();
        assert!(
            document
                .save()
                .unwrap_err()
                .contains("no serializable SRD edits")
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn creates_reloads_and_normalizes_all_supported_animation_key_families() {
        let path = temporary_srd_path("all-animation-key-families");
        fs::write(&path, minimal_two_layer_trs2_srd()).unwrap();

        let mut document = EditorDocument::load(&path).unwrap();
        install_all_key_families(&mut document, b"all", b"set_all");
        document.project.scenes[0].animation_sets[0].start_frame = 7;
        document.project.scenes[0].animation_sets[0].runtime_duration = 99;

        document.save().unwrap();
        assert_eq!(document.project.scenes[0].declared_animation_set_count, 1);
        assert_eq!(document.project.scenes[0].layers[0].animation_count, 1);
        let animation = &document.project.scenes[0].layers[0].animations[0];
        assert_eq!(animation.declared_motion_count, 1);
        assert!(
            animation.motions[0]
                .tracks
                .iter()
                .all(|track| track.key_count > 0)
        );
        let reloaded = EditorDocument::load(&path).unwrap();
        assert_eq!(reloaded.project, document.project);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn saves_as_animation_structure_with_original_asset_path() {
        let source_path = temporary_srd_path("animation-save-as-source");
        let destination_path = temporary_srd_path("animation-save-as-destination");
        fs::write(&source_path, minimal_trs2_srd()).unwrap();
        let mut document = EditorDocument::load(&source_path).unwrap();
        install_all_key_families(&mut document, b"all", b"set");

        let report = document.save_as(&destination_path).unwrap();
        assert_eq!(report.path, destination_path);
        assert_eq!(document.path(), Some(destination_path.as_path()));
        assert_eq!(document.asset_path(), source_path);
        let reloaded = EditorDocument::load(&destination_path).unwrap();
        assert_eq!(reloaded.project, document.project);
        assert_eq!(fs::read(&source_path).unwrap(), minimal_trs2_srd());
        fs::remove_file(source_path).unwrap();
        fs::remove_file(destination_path).unwrap();
    }

    #[test]
    fn duplicates_renames_and_deletes_animation_sets_and_animation_structure() {
        let path = temporary_srd_path("animation-structure-edits");
        fs::write(&path, minimal_trs2_srd()).unwrap();
        let mut document = EditorDocument::load(&path).unwrap();
        install_all_key_families(&mut document, b"all", b"set_a");
        let mut duplicate = document.project.scenes[0].animation_sets[0].clone();
        duplicate.name = b"set_b".to_vec();
        duplicate.start_frame = 3;
        duplicate.runtime_duration = 30;
        duplicate.slots[0].enabled = 0;
        document.project.scenes[0].animation_sets.push(duplicate);
        document.save().unwrap();

        document.project.scenes[0].animation_sets[1].name = b"set_renamed".to_vec();
        let track = &mut document.project.scenes[0].layers[0].animations[0].motions[0].tracks[3];
        track.range_start = -4;
        track.range_end = 44;
        let KeyData::Key20F32(keys) = &mut track.keys else {
            panic!("fixture track must use Key20F32");
        };
        keys.rotate_left(1);
        keys[0].frame = 2;
        keys[0].value = 9.5;
        keys[0].mode = 2;
        keys[0].slope_in = 0.25;
        keys[0].slope_out = 0.5;
        keys.push(Key20 {
            frame: 24,
            value: 1.25,
            mode: 1,
            slope_in: -0.5,
            slope_out: 0.75,
        });
        document.save().unwrap();

        document.project.scenes[0].animation_sets.remove(0);
        document.project.scenes[0].layers[0].animations[0].motions[0]
            .tracks
            .remove(0);
        document.project.scenes[0].layers[0].animations[0]
            .motions
            .clear();
        document.save().unwrap();

        document.project.scenes[0].animation_sets[0].slots[0]
            .animation_name
            .clear();
        document.project.scenes[0].layers[0].animations.clear();
        document.save().unwrap();
        let reloaded = EditorDocument::load(&path).unwrap();
        assert_eq!(reloaded.project, document.project);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn combines_transform_and_text_with_structural_animation_edits() {
        let path = temporary_srd_path("text-transform-animation");
        fs::write(&path, minimal_text_srd(b"old")).unwrap();
        let mut document = EditorDocument::load(&path).unwrap();
        document.project.scenes[0].layers[0]
            .animations
            .push(AnimationDefinition {
                name: b"text_in".to_vec(),
                flags: 1,
                declared_motion_count: 0,
                duration: -1,
                motions: vec![Motion {
                    target: -1,
                    tracks: Vec::new(),
                }],
            });
        document.project.scenes[0]
            .animation_sets
            .push(AnimationSetDefinition {
                name: b"text_set".to_vec(),
                start_frame: 1,
                runtime_duration: 12,
                declared_slot_count: 0,
                slots: vec![SceneAnimationSlot {
                    animation_name: b"text_in".to_vec(),
                    enabled: 1,
                }],
            });
        let RawTransform::Trs2(transform) = &mut document.project.scenes[0].layers[0].transforms[0]
        else {
            panic!("fixture must contain TRS2");
        };
        transform.translation[0] = 123.0;
        document.project.scenes[0].layers[0].image_by_node[0]
            .as_mut()
            .unwrap()
            .text
            .as_mut()
            .unwrap()
            .text = b"new text".to_vec();

        document.save().unwrap();
        let reloaded = EditorDocument::load(&path).unwrap();
        assert_eq!(reloaded.project, document.project);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn node_active_bit_persists_while_other_node_flags_remain_protected() {
        let path = temporary_srd_path("node-active");
        fs::write(&path, minimal_text_srd(b"active")).unwrap();
        let mut document = EditorDocument::load(&path).unwrap();
        document.project.scenes[0].layers[0].nodes[0].type_flags = Some(0x101);
        document.save().unwrap();

        let mut reloaded = EditorDocument::load(&path).unwrap();
        assert!(reloaded.project.scenes[0].layers[0].nodes[0].active());
        let saved = fs::read(&path).unwrap();
        reloaded.project.scenes[0].layers[0].nodes[0].type_flags = Some(0x301);
        assert!(
            reloaded
                .save()
                .unwrap_err()
                .contains("can only change NODE property 0x30 active bit 0x100")
        );
        assert_eq!(fs::read(&path).unwrap(), saved);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn preserves_unrelated_owned_properties_and_blocks_during_animation_reconciliation() {
        let parsed = SrdFile::parse(minimal_trs2_srd()).unwrap();
        let mut owned = parsed.to_owned();
        let scene = &mut owned.blocks[0].children[0].children[0];
        scene.properties.push(OwnedProperty::u16(0xee, 0x1234));
        scene.children.push(OwnedBlock::new(*b"UNKN"));
        let source = owned.encode().unwrap();
        let path = temporary_srd_path("animation-unknown-preservation");
        fs::write(&path, &source).unwrap();

        let mut document = EditorDocument::load(&path).unwrap();
        install_all_key_families(&mut document, b"all", b"set");
        document.save().unwrap();

        let written = SrdFile::parse(fs::read(&path).unwrap()).unwrap();
        let scene = source_project_block(&written)
            .unwrap()
            .children
            .first()
            .unwrap();
        let unknown_property = scene.last_property(0xee).unwrap();
        assert_eq!(unknown_property.value_bytes(&written), &[0x34, 0x12]);
        assert_eq!(
            unknown_property.encoded_bytes(&written),
            &[0xee, 0x06, 0x34, 0x12]
        );
        assert!(scene.children.iter().any(|child| child.is_tag(b"UNKN")));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_unsupported_key_synthesis_and_unrelated_structure_without_writing() {
        let path = temporary_srd_path("unsupported-key-synthesis");
        let source = minimal_trs2_srd();
        fs::write(&path, &source).unwrap();
        let mut document = EditorDocument::load(&path).unwrap();
        document.project.scenes[0].layers[0]
            .animations
            .push(AnimationDefinition {
                name: b"unsupported".to_vec(),
                flags: 0,
                declared_motion_count: 0,
                duration: 0,
                motions: vec![Motion {
                    target: 0,
                    tracks: vec![Track {
                        target: 0,
                        key_count: 0,
                        format: 0,
                        range_start: 0,
                        range_end: 0,
                        keys: KeyData::Unsupported,
                    }],
                }],
            });
        assert!(
            document
                .save()
                .unwrap_err()
                .contains("unsupported KEY data")
        );
        assert_eq!(fs::read(&path).unwrap(), source);

        let mut document = EditorDocument::load(&path).unwrap();
        let layer = &mut document.project.scenes[0].layers[0];
        let duplicate = layer.nodes[0].clone();
        layer.nodes.push(duplicate);
        assert!(
            document
                .save()
                .unwrap_err()
                .contains("cannot add, remove, or reorder CAST records")
        );
        assert_eq!(fs::read(&path).unwrap(), source);

        let mut document = EditorDocument::load(&path).unwrap();
        install_all_key_families(&mut document, b"all", b"set");
        document.project.scenes[0].animation_sets[0].slots[0].animation_name = b"missing".to_vec();
        assert!(
            document
                .save()
                .unwrap_err()
                .contains("does not resolve in layer[0]")
        );
        assert_eq!(fs::read(&path).unwrap(), source);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_deleting_an_animation_still_named_by_an_unchanged_animation_set() {
        let path = temporary_srd_path("dangling-unchanged-slot");
        fs::write(&path, minimal_trs2_srd()).unwrap();
        let mut document = EditorDocument::load(&path).unwrap();
        install_all_key_families(&mut document, b"all", b"set");
        document.save().unwrap();
        let source = fs::read(&path).unwrap();

        document.project.scenes[0].layers[0].animations.clear();
        assert!(
            document
                .save()
                .unwrap_err()
                .contains("does not resolve in layer[0]")
        );
        assert_eq!(fs::read(&path).unwrap(), source);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn removing_a_non_tail_animation_keeps_the_survivors_opaque_data() {
        let mut owned = SrdFile::parse(minimal_trs2_srd()).unwrap().to_owned();
        append_owned_animation(
            &mut owned,
            owned_animation(b"first".to_vec(), 0, Vec::new()),
        );
        let mut key = OwnedBlock::new(*b"KEY ");
        key.properties = vec![
            OwnedProperty::i32(0x5a, 0),
            OwnedProperty::f32(0x5b, 1.0),
            OwnedProperty::u16(0xee, 0x1111),
        ];
        let mut track = OwnedBlock::new(*b"TRK ");
        track.properties = vec![
            OwnedProperty::u16(0x53, 0),
            OwnedProperty::u16(0x57, 1),
            OwnedProperty::u32(0x54, 0x10),
            OwnedProperty::i32(0x58, 0),
            OwnedProperty::i32(0x59, 0),
            OwnedProperty::u16(0xed, 0x2222),
        ];
        track.children.push(key);
        let mut motion = OwnedBlock::new(*b"MOT ");
        motion.properties = vec![
            OwnedProperty::i16(0x51, 0),
            OwnedProperty::u16(0x52, 1),
            OwnedProperty::u16(0xec, 0x3333),
        ];
        motion.children.push(track);
        let mut survivor = owned_animation(b"survivor".to_vec(), 0, vec![motion]);
        survivor.properties.push(OwnedProperty::u16(0xee, 0x7654));
        survivor.children.push(OwnedBlock::new(*b"OPAQ"));
        append_owned_animation(&mut owned, survivor);
        let layer = owned_layer_mut(owned_scene_mut(&mut owned));
        let survivor_index = layer
            .children
            .iter()
            .rposition(|child| child.tag == *b"ANIM")
            .unwrap();
        layer
            .children
            .insert(survivor_index, OwnedBlock::new(*b"UNKN"));
        let path = temporary_srd_path("surviving-animation-opaque");
        fs::write(&path, owned.encode().unwrap()).unwrap();

        let mut document = EditorDocument::load(&path).unwrap();
        document.project.scenes[0].layers[0].animations.remove(0);
        document.save().unwrap();

        let written = SrdFile::parse(fs::read(&path).unwrap()).unwrap();
        let layer = source_project_block(&written).unwrap().children[0]
            .children
            .iter()
            .find(|child| child.is_tag(b"LAYR"))
            .unwrap();
        let animation = layer
            .children
            .iter()
            .find(|child| child.is_tag(b"ANIM"))
            .unwrap();
        assert!(animation.last_property(0xee).is_some());
        assert!(animation.children.iter().any(|child| child.is_tag(b"OPAQ")));
        let unknown_index = layer
            .children
            .iter()
            .position(|child| child.tag == *b"UNKN")
            .unwrap();
        let animation_index = layer
            .children
            .iter()
            .position(|child| child.tag == *b"ANIM")
            .unwrap();
        assert!(unknown_index < animation_index);
        let motion = animation
            .children
            .iter()
            .find(|child| child.is_tag(b"MOT "))
            .unwrap();
        let track = motion
            .children
            .iter()
            .find(|child| child.is_tag(b"TRK "))
            .unwrap();
        let key = track
            .children
            .iter()
            .find(|child| child.is_tag(b"KEY "))
            .unwrap();
        assert!(motion.last_property(0xec).is_some());
        assert!(track.last_property(0xed).is_some());
        assert!(key.last_property(0xee).is_some());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_new_motion_targets_outside_the_layers_casts() {
        let path = temporary_srd_path("invalid-motion-target");
        let source = minimal_trs2_srd();
        fs::write(&path, &source).unwrap();
        let mut document = EditorDocument::load(&path).unwrap();
        install_all_key_families(&mut document, b"all", b"set");
        document.project.scenes[0].layers[0].animations[0].motions[0].target = 1;
        assert!(
            document
                .save()
                .unwrap_err()
                .contains("must be -1 with zero tracks")
        );
        assert_eq!(fs::read(&path).unwrap(), source);

        document.project.scenes[0].layers[0].animations[0].motions[0].target = -1;
        assert!(
            document
                .save()
                .unwrap_err()
                .contains("must be -1 with zero tracks")
        );
        assert_eq!(fs::read(&path).unwrap(), source);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn clean_save_as_copies_loadable_non_authorable_animation_slots() {
        let mut owned = SrdFile::parse(minimal_trs2_srd()).unwrap().to_owned();
        append_owned_animation_set(&mut owned, owned_animation_set(b"legacy", Vec::new()));
        let source = owned.encode().unwrap();
        let path = temporary_srd_path("legacy-clean-save-as-source");
        let destination = temporary_srd_path("legacy-clean-save-as-destination");
        fs::write(&path, &source).unwrap();

        let mut document = EditorDocument::load(&path).unwrap();
        document.save_as(&destination).unwrap();
        assert_eq!(fs::read(&destination).unwrap(), source);
        fs::remove_file(path).unwrap();
        fs::remove_file(destination).unwrap();
    }

    #[test]
    fn unrelated_transform_edits_preserve_legacy_non_authorable_animation_sets() {
        let mut owned = SrdFile::parse(minimal_trs2_srd()).unwrap().to_owned();
        append_owned_animation_set(&mut owned, owned_animation_set(b"legacy", Vec::new()));
        let path = temporary_srd_path("legacy-dirty-transform");
        fs::write(&path, owned.encode().unwrap()).unwrap();

        let mut document = EditorDocument::load(&path).unwrap();
        let RawTransform::Trs2(transform) = &mut document.project.scenes[0].layers[0].transforms[0]
        else {
            panic!("fixture must contain TRS2");
        };
        transform.translation[0] = 77.0;
        document.save().unwrap();
        let reloaded = EditorDocument::load(&path).unwrap();
        assert_eq!(
            reloaded.project.scenes[0].layers[0].transforms[0]
                .spatial()
                .translation[0],
            77.0
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn changed_optional_animation_fields_are_synthesized_without_rewriting_defaults() {
        let mut owned = SrdFile::parse(minimal_trs2_srd()).unwrap().to_owned();
        append_owned_animation_set(
            &mut owned,
            owned_animation_set(b"set", vec![OwnedBlock::new(*b"SANM")]),
        );
        let path = temporary_srd_path("optional-animation-fields");
        fs::write(&path, owned.encode().unwrap()).unwrap();

        let mut document = EditorDocument::load(&path).unwrap();
        document.project.scenes[0].animation_sets[0].start_frame = 4;
        document.project.scenes[0].animation_sets[0].slots[0].enabled = 0;
        document.save().unwrap();

        let written = SrdFile::parse(fs::read(&path).unwrap()).unwrap();
        let scene = &source_project_block(&written).unwrap().children[0];
        let set = scene
            .children
            .iter()
            .find(|child| child.is_tag(b"ANMS"))
            .unwrap();
        let slot = set
            .children
            .iter()
            .find(|child| child.is_tag(b"SANM"))
            .unwrap();
        assert!(set.last_property(0x18).is_some());
        assert!(slot.last_property(0x03).is_none());
        assert!(slot.last_property(0x0f).is_some());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_tracks_added_to_an_implicit_padding_motion() {
        let mut owned = SrdFile::parse(minimal_trs2_srd()).unwrap().to_owned();
        append_owned_animation(
            &mut owned,
            owned_animation(b"padding".to_vec(), 0, vec![owned_empty_motion()]),
        );
        let source = owned.encode().unwrap();
        let path = temporary_srd_path("padding-motion-track");
        fs::write(&path, &source).unwrap();

        let mut document = EditorDocument::load(&path).unwrap();
        document.project.scenes[0].layers[0].animations[0].motions[0]
            .tracks
            .push(Track {
                target: 0,
                key_count: 0,
                format: 0x10,
                range_start: 0,
                range_end: 0,
                keys: KeyData::Key8F32(vec![Key8 {
                    frame: 0,
                    value: 1.0,
                }]),
            });
        assert!(
            document
                .save()
                .unwrap_err()
                .contains("must be -1 with zero tracks")
        );
        assert_eq!(fs::read(&path).unwrap(), source);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn explicit_empty_motion_survives_an_unrelated_animation_edit() {
        let mut owned = SrdFile::parse(minimal_trs2_srd()).unwrap().to_owned();
        let mut motion = owned_empty_motion();
        motion.properties.push(OwnedProperty::u16(0xee, 0x1234));
        motion.children.push(OwnedBlock::new(*b"OPAQ"));
        append_owned_animation(
            &mut owned,
            owned_animation(b"empty".to_vec(), 0, vec![motion]),
        );
        let path = temporary_srd_path("explicit-empty-motion");
        fs::write(&path, owned.encode().unwrap()).unwrap();

        let mut document = EditorDocument::load(&path).unwrap();
        document.project.scenes[0].layers[0].animations[0].flags = 1;
        document.save().unwrap();

        let written = SrdFile::parse(fs::read(&path).unwrap()).unwrap();
        let layer = source_project_block(&written).unwrap().children[0]
            .children
            .iter()
            .find(|child| child.is_tag(b"LAYR"))
            .unwrap();
        let motion = layer
            .children
            .iter()
            .find(|child| child.is_tag(b"ANIM"))
            .unwrap()
            .children
            .iter()
            .find(|child| child.is_tag(b"MOT "))
            .unwrap();
        assert!(motion.last_property(0xee).is_some());
        assert!(motion.children.iter().any(|child| child.is_tag(b"OPAQ")));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn unrelated_animation_edits_do_not_truncate_lossy_source_names() {
        let mut owned = SrdFile::parse(minimal_trs2_srd()).unwrap().to_owned();
        let long_name = vec![b'x'; 65];
        append_owned_animation(
            &mut owned,
            owned_animation(long_name.clone(), 0, Vec::new()),
        );
        let path = temporary_srd_path("lossy-animation-name");
        fs::write(&path, owned.encode().unwrap()).unwrap();

        let mut document = EditorDocument::load(&path).unwrap();
        assert_eq!(
            document.project.scenes[0].layers[0].animations[0]
                .name
                .len(),
            64
        );
        document.project.scenes[0].layers[0].animations[0].flags = 1;
        document.save().unwrap();

        let written = SrdFile::parse(fs::read(&path).unwrap()).unwrap();
        let layer = source_project_block(&written).unwrap().children[0]
            .children
            .iter()
            .find(|child| child.is_tag(b"LAYR"))
            .unwrap();
        let name = layer
            .children
            .iter()
            .find(|child| child.is_tag(b"ANIM"))
            .unwrap()
            .last_property(0x03)
            .unwrap()
            .value_bytes(&written);
        assert_eq!(name, long_name.as_slice());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn source_nan_keys_are_clean_but_new_nan_keys_are_rejected() {
        let path = temporary_srd_path("nan-animation-key");
        fs::write(&path, minimal_trs2_srd()).unwrap();
        let mut document = EditorDocument::load(&path).unwrap();
        install_all_key_families(&mut document, b"all", b"set");
        document.project.scenes[0].layers[0].animations[0].motions[0].tracks[0].keys =
            KeyData::Key8F32(vec![Key8 {
                frame: 0,
                value: f32::NAN,
            }]);
        assert!(document.save().unwrap_err().contains("non-finite"));

        document.project.scenes[0].layers[0].animations[0].motions[0].tracks[0].keys =
            KeyData::Key8F32(vec![Key8 {
                frame: 0,
                value: 1.0,
            }]);
        document.save().unwrap();
        let mut owned = SrdFile::parse(fs::read(&path).unwrap()).unwrap().to_owned();
        let layer = owned_layer_mut(owned_scene_mut(&mut owned));
        let key = layer
            .children
            .iter_mut()
            .find(|child| child.tag == *b"ANIM")
            .unwrap()
            .children
            .iter_mut()
            .find(|child| child.tag == *b"MOT ")
            .unwrap()
            .children
            .iter_mut()
            .find(|child| child.tag == *b"TRK ")
            .unwrap()
            .children
            .iter_mut()
            .find(|child| child.tag == *b"KEY ")
            .unwrap();
        key.properties
            .iter_mut()
            .find(|property| property.code == 0x5b)
            .unwrap()
            .value = f32::NAN.to_le_bytes().to_vec();
        fs::write(&path, owned.encode().unwrap()).unwrap();

        let mut document = EditorDocument::load(&path).unwrap();
        assert!(!document.is_dirty());
        let duplicate =
            document.project.scenes[0].layers[0].animations[0].motions[0].tracks[0].clone();
        document.project.scenes[0].layers[0].animations[0].motions[0]
            .tracks
            .push(duplicate);
        assert!(document.save().unwrap_err().contains("non-finite"));
        document.project.scenes[0].layers[0].animations[0].motions[0]
            .tracks
            .pop();
        document.project.scenes[0].layers[0].animations[0].flags = 7;
        document.save().unwrap();
        assert!(!document.is_dirty());
        fs::remove_file(path).unwrap();
    }

    fn install_all_key_families(
        document: &mut EditorDocument,
        animation_name: &[u8],
        set_name: &[u8],
    ) {
        document.project.scenes[0].layers[0]
            .animations
            .push(all_key_families_animation(animation_name));
        let slots = document.project.scenes[0]
            .layers
            .iter()
            .enumerate()
            .map(|(layer_index, _)| SceneAnimationSlot {
                animation_name: if layer_index == 0 {
                    animation_name.to_vec()
                } else {
                    Vec::new()
                },
                enabled: 1,
            })
            .collect();
        document.project.scenes[0]
            .animation_sets
            .push(AnimationSetDefinition {
                name: set_name.to_vec(),
                start_frame: 0,
                runtime_duration: 24,
                declared_slot_count: 0,
                slots,
            });
    }

    fn owned_scene_mut(owned: &mut OwnedSrdFile) -> &mut OwnedBlock {
        &mut owned.blocks[0].children[0].children[0]
    }

    fn owned_layer_mut(scene: &mut OwnedBlock) -> &mut OwnedBlock {
        scene
            .children
            .iter_mut()
            .find(|child| child.tag == *b"LAYR")
            .unwrap()
    }

    fn replace_owned_property(block: &mut OwnedBlock, replacement: OwnedProperty) {
        let code = replacement.code;
        *block
            .properties
            .iter_mut()
            .find(|property| property.code == code)
            .unwrap() = replacement;
    }

    fn owned_animation(name: Vec<u8>, flags: u32, motions: Vec<OwnedBlock>) -> OwnedBlock {
        let mut animation = OwnedBlock::new(*b"ANIM");
        animation.properties = vec![
            OwnedProperty::u16(0x50, motions.len() as u16),
            OwnedProperty::i16(0x56, 0),
            OwnedProperty::string(0x03, name).unwrap(),
            OwnedProperty::u32(0x5f, flags),
        ];
        animation.children = motions;
        animation
    }

    fn owned_empty_motion() -> OwnedBlock {
        let mut motion = OwnedBlock::new(*b"MOT ");
        motion.properties = vec![OwnedProperty::i16(0x51, -1), OwnedProperty::u16(0x52, 0)];
        motion
    }

    fn append_owned_animation(owned: &mut OwnedSrdFile, animation: OwnedBlock) {
        let scene = owned_scene_mut(owned);
        let layer = owned_layer_mut(scene);
        let count = layer
            .children
            .iter()
            .filter(|child| child.tag == *b"ANIM")
            .count();
        replace_owned_property(layer, OwnedProperty::u16(0x22, (count + 1) as u16));
        layer.children.push(animation);
    }

    fn append_owned_animation_set(owned: &mut OwnedSrdFile, animation_set: OwnedBlock) {
        let scene = owned_scene_mut(owned);
        replace_owned_property(scene, OwnedProperty::u16(0x17, 1));
        scene.children.push(animation_set);
    }

    fn owned_animation_set(name: &[u8], slots: Vec<OwnedBlock>) -> OwnedBlock {
        let mut animation_set = OwnedBlock::new(*b"ANMS");
        animation_set.properties = vec![
            OwnedProperty::string(0x03, name.to_vec()).unwrap(),
            OwnedProperty::i32(0x0e, slots.len() as i32),
        ];
        animation_set.children = slots;
        animation_set
    }

    fn all_key_families_animation(name: &[u8]) -> AnimationDefinition {
        AnimationDefinition {
            name: name.to_vec(),
            flags: 0x11,
            declared_motion_count: 0,
            duration: 24,
            motions: vec![Motion {
                target: 0,
                tracks: vec![
                    Track {
                        target: 0,
                        key_count: 0,
                        format: 0x10,
                        range_start: 0,
                        range_end: 10,
                        keys: KeyData::Key8F32(vec![
                            Key8 {
                                frame: 0,
                                value: 1.5,
                            },
                            Key8 {
                                frame: 10,
                                value: 3.5,
                            },
                        ]),
                    },
                    Track {
                        target: 10,
                        key_count: 0,
                        format: 0x40,
                        range_start: 0,
                        range_end: 10,
                        keys: KeyData::Key8I32(vec![
                            Key8 { frame: 0, value: 4 },
                            Key8 {
                                frame: 10,
                                value: -2,
                            },
                        ]),
                    },
                    Track {
                        target: 13,
                        key_count: 0,
                        format: 0x51,
                        range_start: 0,
                        range_end: 10,
                        keys: KeyData::Key8Bytes4(vec![
                            Key8 {
                                frame: 0,
                                value: [1, 2, 3, 4],
                            },
                            Key8 {
                                frame: 10,
                                value: [5, 6, 7, 8],
                            },
                        ]),
                    },
                    Track {
                        target: 21,
                        key_count: 0,
                        format: 0x13,
                        range_start: 0,
                        range_end: 10,
                        keys: KeyData::Key20F32(vec![
                            Key20 {
                                frame: 0,
                                value: 1.0,
                                mode: 2,
                                slope_in: 0.0,
                                slope_out: 0.5,
                            },
                            Key20 {
                                frame: 10,
                                value: 0.0,
                                mode: 0,
                                slope_in: 0.5,
                                slope_out: 0.0,
                            },
                        ]),
                    },
                    Track {
                        target: 17,
                        key_count: 0,
                        format: 0x43,
                        range_start: 0,
                        range_end: 10,
                        keys: KeyData::Key20I32(vec![
                            Key20 {
                                frame: 0,
                                value: 7,
                                mode: 2,
                                slope_in: 0.0,
                                slope_out: 0.25,
                            },
                            Key20 {
                                frame: 10,
                                value: 3,
                                mode: 1,
                                slope_in: 0.25,
                                slope_out: 0.0,
                            },
                        ]),
                    },
                ],
            }],
        }
    }

    fn temporary_srd_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "srd-editor-{label}-{}-{}.srd",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    pub(crate) fn minimal_trs2_srd() -> Vec<u8> {
        let node = block(*b"NODE", Vec::new(), Vec::new());
        let trs2 = block(
            *b"TRS2",
            vec![
                f32_vector_property(0x34, &[1.0, 2.0]),
                i32_vector_property(0x35, &[3]),
                f32_vector_property(0x36, &[1.0, 1.0]),
            ],
            Vec::new(),
        );
        let cast = block(*b"CAST", Vec::new(), vec![node, trs2]);
        let layer = block(
            *b"LAYR",
            vec![
                string_property(0x03, b"layer"),
                u32_property(0x20, 0),
                u16_property(0x21, 1),
                u16_property(0x22, 0),
            ],
            vec![cast],
        );
        let scene = block(
            *b"SCN ",
            vec![
                string_property(0x03, b"scene"),
                u16_property(0x10, 1),
                u16_property(0x17, 0),
            ],
            vec![layer],
        );
        let project = block(*b"PROJ", vec![u32_property(0x00, 1)], vec![scene]);
        let srck = block(*b"SRCK", Vec::new(), vec![project]);

        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"VTBF");
        bytes.extend_from_slice(&[0; 4]);
        bytes.extend_from_slice(b"SRFF");

        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&[0; 2]);
        bytes.extend_from_slice(&srck);
        bytes.extend_from_slice(b"opaque source data");
        bytes
    }
    fn minimal_two_layer_trs2_srd() -> Vec<u8> {
        let parsed = SrdFile::parse(minimal_trs2_srd()).unwrap();
        let mut owned = parsed.to_owned();
        let scene = &mut owned.blocks[0].children[0].children[0];
        let mut second_layer = scene.children[0].clone();
        second_layer
            .properties
            .iter_mut()
            .find(|property| property.code == 0x03)
            .unwrap()
            .value = b"layer_2".to_vec();
        scene.children.push(second_layer);
        scene
            .properties
            .iter_mut()
            .find(|property| property.code == 0x10)
            .unwrap()
            .value = 2u16.to_le_bytes().to_vec();
        owned.encode().unwrap()
    }
    pub(crate) fn minimal_two_layer_two_node_trs2_srd() -> Vec<u8> {
        let parsed = SrdFile::parse(minimal_two_layer_trs2_srd()).unwrap();
        let mut owned = parsed.to_owned();
        let scene = &mut owned.blocks[0].children[0].children[0];

        for (layer_index, layer) in scene.children.iter_mut().enumerate() {
            layer
                .properties
                .iter_mut()
                .find(|property| property.code == 0x03)
                .unwrap()
                .value = if layer_index == 0 {
                b"root".to_vec()
            } else {
                b"second".to_vec()
            };
            layer
                .properties
                .iter_mut()
                .find(|property| property.code == 0x20)
                .unwrap()
                .value = 0x100u32.to_le_bytes().to_vec();
        }

        let layer = &mut scene.children[0];
        layer
            .properties
            .iter_mut()
            .find(|property| property.code == 0x21)
            .unwrap()
            .value = 2u16.to_le_bytes().to_vec();
        let cast = layer
            .children
            .iter_mut()
            .find(|child| child.tag == *b"CAST")
            .unwrap();
        let node = cast
            .children
            .iter_mut()
            .find(|child| child.tag == *b"NODE")
            .unwrap();
        node.properties = vec![
            OwnedProperty::string(0x03, b"parent".to_vec()).unwrap(),
            OwnedProperty::u32(0x30, 1),
            OwnedProperty::i16(0x3c, 1),
            OwnedProperty::u8(0xfe, 0),
            OwnedProperty::string(0x03, b"child".to_vec()).unwrap(),
            OwnedProperty::u32(0x30, 1),
        ];
        let transform = cast
            .children
            .iter_mut()
            .find(|child| child.tag == *b"TRS2")
            .unwrap();
        let second_transform = transform.properties.clone();
        transform.properties.push(OwnedProperty::u8(0xfe, 0));
        transform.properties.extend(second_transform);
        owned.encode().unwrap()
    }

    pub(crate) fn minimal_text_srd(text_content: &[u8]) -> Vec<u8> {
        let node = block(*b"NODE", vec![u32_property(0x30, 1)], Vec::new());
        let trs2 = block(
            *b"TRS2",
            vec![
                f32_vector_property(0x34, &[1.0, 2.0]),
                i32_vector_property(0x35, &[3]),
                f32_vector_property(0x36, &[1.0, 1.0]),
            ],
            Vec::new(),
        );
        let text = block(
            *b"TEXT",
            vec![string_property(0x7a, text_content)],
            Vec::new(),
        );
        let image = block(
            *b"CIMG",
            vec![u32_property(0x49, 0x100), u32_property(0x51, 0)],
            vec![text],
        );
        let data = block(*b"DATA", Vec::new(), vec![image]);
        let cast = block(*b"CAST", Vec::new(), vec![node, trs2, data]);
        let layer = block(
            *b"LAYR",
            vec![
                string_property(0x03, b"layer"),
                u32_property(0x20, 0),
                u16_property(0x21, 1),
                u16_property(0x22, 0),
            ],
            vec![cast],
        );
        let scene = block(
            *b"SCN ",
            vec![
                string_property(0x03, b"scene"),
                u16_property(0x10, 1),
                u16_property(0x17, 0),
            ],
            vec![layer],
        );
        let project = block(*b"PROJ", vec![u32_property(0x00, 1)], vec![scene]);
        let srck = block(*b"SRCK", Vec::new(), vec![project]);

        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"VTBF");
        bytes.extend_from_slice(&[0; 4]);
        bytes.extend_from_slice(b"SRFF");
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&[0; 2]);
        bytes.extend_from_slice(&srck);
        bytes.extend_from_slice(b"opaque source data");
        bytes
    }

    fn block(tag: [u8; 4], properties: Vec<Vec<u8>>, children: Vec<Vec<u8>>) -> Vec<u8> {
        let property_count = u16::try_from(properties.len()).unwrap();
        let property_bytes = properties.into_iter().flatten().collect::<Vec<_>>();
        let size = 8 + property_bytes.len();
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"vtc0");
        bytes.extend_from_slice(&(size as u32).to_le_bytes());
        bytes.extend_from_slice(&tag);
        bytes.extend_from_slice(&(children.len() as u16).to_le_bytes());
        bytes.extend_from_slice(&property_count.to_le_bytes());
        bytes.extend_from_slice(&property_bytes);
        for child in children {
            bytes.extend_from_slice(&child);
        }
        bytes
    }

    fn u32_property(code: u8, value: u32) -> Vec<u8> {
        let mut bytes = vec![code, 8];
        bytes.extend_from_slice(&value.to_le_bytes());
        bytes
    }

    fn u16_property(code: u8, value: u16) -> Vec<u8> {
        let mut bytes = vec![code, 6];
        bytes.extend_from_slice(&value.to_le_bytes());
        bytes
    }

    fn i32_vector_property(code: u8, values: &[i32]) -> Vec<u8> {
        let mut bytes = vec![code, 0x88, 0x10];
        bytes.extend_from_slice(&u16::try_from(values.len() - 1).unwrap().to_le_bytes());
        for value in values {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn f32_vector_property(code: u8, values: &[f32]) -> Vec<u8> {
        let mut bytes = vec![code, 0x8a, 0x10];
        bytes.extend_from_slice(&u16::try_from(values.len() - 1).unwrap().to_le_bytes());
        for value in values {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn string_property(code: u8, value: &[u8]) -> Vec<u8> {
        let mut bytes = vec![code, 2, u8::try_from(value.len()).unwrap()];
        bytes.extend_from_slice(value);
        bytes
    }
}
