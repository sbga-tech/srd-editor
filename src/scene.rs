use std::fmt;

use crate::animation::AnimationDefinition;
use crate::attribute::{CastAttributeList, ExtParamData, FontParamData};
use crate::camera::CameraDefinition;
use crate::csli::{CsliDefinition, parent_cell_center_offset};
use crate::image::ImageDefinition;
use crate::number::NumberDefinition;
use crate::reference::ReferenceDefinition;
use crate::text::{FontDefinition, TextDefinition};
use crate::transform::{Affine3x4, SpatialTransform, build_local_matrix};
use crate::vtbf::{Block, Property, SrdFile};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneError(pub String);

impl fmt::Display for SceneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SceneError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeRecord {
    pub name: Option<Vec<u8>>,
    pub type_flags: Option<u32>,
    pub parent_csli_cell_index: Option<i32>,
    pub first_child_index: i16,
    pub next_sibling_index: i16,
    pub field_a0: Option<i32>,
}

impl NodeRecord {
    pub fn cast_type(&self) -> Option<u8> {
        self.type_flags.map(|value| value as u8)
    }

    /// Low byte loaded from parsed NODE `+0x58` by `srd_render_cast`.
    /// Missing property `0xA0` stays zero in the binary's parsed record.
    pub fn render_layer_offset(&self) -> u8 {
        self.field_a0.unwrap_or(0) as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SrCastKind {
    Null,
    Image,
    Text,
    Slice,
    Reference,
    Number,
}

impl SrCastKind {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Null => "NullCast",
            Self::Image => "ImageCast",
            Self::Text => "TextCast",
            Self::Slice => "SliceCast",
            Self::Reference => "ReferenceCast",
            Self::Number => "NumberCast",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CastPayloads(u8);

impl CastPayloads {
    pub const IMAGE: Self = Self(1 << 0);
    pub const SLICE: Self = Self(1 << 1);
    pub const REFERENCE: Self = Self(1 << 2);
    pub const NUMBER: Self = Self(1 << 3);

    pub fn contains(self, payload: Self) -> bool {
        self.0 & payload.0 != 0
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn record_name(self) -> &'static str {
        match self {
            Self::IMAGE => "CIMG",
            Self::SLICE => "CSLI",
            Self::REFERENCE => "CRFD",
            Self::NUMBER => "CNUM",
            _ => "payload",
        }
    }

    pub fn iter(self) -> impl Iterator<Item = Self> {
        [Self::IMAGE, Self::SLICE, Self::REFERENCE, Self::NUMBER]
            .into_iter()
            .filter(move |payload| self.contains(*payload))
    }
}

impl std::ops::BitOrAssign for CastPayloads {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CastClassification {
    pub kind: SrCastKind,
    pub serialized_type: Option<u8>,
    pub present_payloads: CastPayloads,
    pub inactive_text_payload: bool,
}

impl CastClassification {
    pub fn expected_payload(self) -> Option<CastPayloads> {
        match self.kind {
            SrCastKind::Null => None,
            SrCastKind::Image | SrCastKind::Text => Some(CastPayloads::IMAGE),
            SrCastKind::Slice => Some(CastPayloads::SLICE),
            SrCastKind::Reference => Some(CastPayloads::REFERENCE),
            SrCastKind::Number => Some(CastPayloads::NUMBER),
        }
    }

    pub fn missing_expected_payload(self) -> bool {
        self.expected_payload()
            .is_some_and(|expected| !self.present_payloads.contains(expected))
    }

    pub fn extra_payloads(self) -> CastPayloads {
        let expected = self.expected_payload();
        let mut extras = CastPayloads::default();
        for payload in self.present_payloads.iter() {
            if Some(payload) != expected {
                extras |= payload;
            }
        }
        extras
    }

    pub fn unsupported_serialized_type(self) -> Option<u8> {
        self.serialized_type.filter(|value| *value > 4)
    }

    pub fn has_diagnostics(self) -> bool {
        self.missing_expected_payload()
            || !self.extra_payloads().is_empty()
            || self.inactive_text_payload
            || self.unsupported_serialized_type().is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hierarchy {
    pub parents: Vec<Option<usize>>,
    pub children: Vec<Vec<usize>>,
    pub roots: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReferenceTarget {
    pub scene_index: usize,
    pub layer_index: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    pub name: Vec<u8>,
    pub declared_layer_count: u32,
    pub declared_animation_set_count: u32,
    pub width: f32,
    pub height: f32,
    pub layers: Vec<Layer>,
    pub animation_sets: Vec<AnimationSetDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneAnimationSlot {
    pub animation_name: Vec<u8>,
    pub enabled: i32,
}

impl SceneAnimationSlot {
    pub fn is_enabled(&self) -> bool {
        self.enabled != 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationSetDefinition {
    pub name: Vec<u8>,
    pub start_frame: i32,
    pub runtime_duration: i32,
    pub declared_slot_count: i32,
    pub slots: Vec<SceneAnimationSlot>,
}

impl Scene {
    pub fn from_block(file: &SrdFile, block: &Block) -> Result<Self, SceneError> {
        if !block.is_tag(b"SCN ") {
            return Err(SceneError("block is not SCN ".into()));
        }

        let name = fixed_name(file, block, 0x03, 64, "SCN  0x03")?;
        let declared_layer_count = read_unsigned(file, block, 0x10)?;
        let declared_animation_set_count = read_unsigned(file, block, 0x17)?;
        let width = optional_float(file, block, 0x40)?;
        let height = optional_float(file, block, 0x41)?;
        let layers = block
            .children
            .iter()
            .filter(|child| child.is_tag(b"LAYR"))
            .map(|child| Layer::from_block(file, child))
            .collect::<Result<Vec<_>, _>>()?;
        let animation_sets = block
            .children
            .iter()
            .filter(|child| child.is_tag(b"ANMS"))
            .map(|child| AnimationSetDefinition::from_block(file, child))
            .collect::<Result<Vec<_>, _>>()?;

        validate_declared_count(
            declared_layer_count,
            layers.len(),
            "SCN  LAYR",
            block.offset,
        )?;
        validate_declared_count(
            declared_animation_set_count,
            animation_sets.len(),
            "SCN  ANMS",
            block.offset,
        )?;

        Ok(Self {
            name,
            declared_layer_count,
            declared_animation_set_count,
            width,
            height,
            layers,
            animation_sets,
        })
    }
}

impl AnimationSetDefinition {
    pub fn from_block(file: &SrdFile, block: &Block) -> Result<Self, SceneError> {
        if !block.is_tag(b"ANMS") {
            return Err(SceneError("block is not ANMS".into()));
        }

        let name = optional_fixed_name(file, block, 0x03, 64, "ANMS 0x03")?;
        let start_frame = optional_signed(file, block, 0x18)?.unwrap_or(0);
        let runtime_duration = optional_signed(file, block, 0x19)?.unwrap_or(0);
        let declared_slot_count = optional_signed(file, block, 0x0e)?.unwrap_or(0);
        let slots = block
            .children
            .iter()
            .filter(|child| child.is_tag(b"SANM"))
            .map(|child| SceneAnimationSlot::from_block(file, child))
            .collect::<Result<Vec<_>, _>>()?;
        validate_signed_declared_count(
            declared_slot_count,
            slots.len(),
            "ANMS SANM",
            block.offset,
        )?;

        Ok(Self {
            name,
            start_frame,
            runtime_duration,
            declared_slot_count,
            slots,
        })
    }
}

impl SceneAnimationSlot {
    pub fn from_block(file: &SrdFile, block: &Block) -> Result<Self, SceneError> {
        if !block.is_tag(b"SANM") {
            return Err(SceneError("block is not SANM".into()));
        }
        Ok(Self {
            animation_name: optional_fixed_name(file, block, 0x03, 64, "SANM 0x03")?,
            enabled: optional_signed(file, block, 0x0f)?.unwrap_or(1),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    pub name: Vec<u8>,
    pub declared_scene_count: u32,
    pub declared_font_count: u32,
    pub camera: CameraDefinition,
    pub scenes: Vec<Scene>,
    pub fonts: Vec<FontDefinition>,
}

impl Project {
    pub fn from_file(file: &SrdFile) -> Result<Self, SceneError> {
        if &file.format != b"SRFF" {
            return Err(SceneError("VTBF file format is not SRFF".into()));
        }
        let mut selected = None;
        for srck in file.blocks.iter().filter(|block| block.is_tag(b"SRCK")) {
            for project in srck.children.iter().filter(|block| block.is_tag(b"PROJ")) {
                selected = Some(project);
            }
        }
        let block = selected.ok_or_else(|| SceneError("SRFF has no SRCK/PROJ block".into()))?;
        let name = block
            .last_property(0x03)
            .map(|property| {
                property
                    .string_bytes(file)
                    .ok_or_else(|| SceneError("PROJ 0x03 is not a string".into()))
                    .map(|bytes| bytes.iter().copied().take(64).collect())
            })
            .transpose()?
            .unwrap_or_default();
        let declared_scene_count = read_unsigned(file, block, 0x00)?;
        let declared_font_count = optional_unsigned(file, block, 0x05)?.unwrap_or(0);
        let camera = CameraDefinition::from_project_block(file, block)
            .map_err(|error| SceneError(error.to_string()))?;
        let scenes = block
            .children
            .iter()
            .filter(|child| child.is_tag(b"SCN "))
            .map(|child| Scene::from_block(file, child))
            .collect::<Result<Vec<_>, _>>()?;
        let fonts = block
            .children
            .iter()
            .filter(|child| child.is_tag(b"FONT"))
            .map(|child| {
                FontDefinition::from_block(file, child)
                    .map_err(|error| SceneError(error.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        validate_declared_count(
            declared_scene_count,
            scenes.len(),
            "PROJ SCN ",
            block.offset,
        )?;
        validate_declared_count(declared_font_count, fonts.len(), "PROJ FONT", block.offset)?;

        Ok(Self {
            name,
            declared_scene_count,
            declared_font_count,
            camera,
            scenes,
            fonts,
        })
    }

    pub fn resolve_text_font(&self, text: &TextDefinition) -> Option<&FontDefinition> {
        let index = usize::try_from(text.font_index?).ok()?;
        self.fonts.get(index)
    }

    pub fn resolve_reference(&self, reference: &ReferenceDefinition) -> Option<ReferenceTarget> {
        let scene_index = self
            .scenes
            .iter()
            .position(|scene| scene.name == reference.source_name)?;
        let layer_index = self.scenes[scene_index]
            .layers
            .iter()
            .position(|layer| layer.name == reference.layer_name)?;
        Some(ReferenceTarget {
            scene_index,
            layer_index,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RawTransform {
    Trs2(SpatialTransform),
    Trs3(SpatialTransform),
}

impl RawTransform {
    pub fn spatial(self) -> SpatialTransform {
        match self {
            Self::Trs2(value) | Self::Trs3(value) => value,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub name: Vec<u8>,
    pub flags: u32,
    pub animation_count: u32,
    pub animations: Vec<AnimationDefinition>,
    pub field_23: Vec<u8>,
    pub nodes: Vec<NodeRecord>,
    pub transforms: Vec<RawTransform>,
    pub image_by_node: Vec<Option<ImageDefinition>>,
    pub number_by_node: Vec<Option<NumberDefinition>>,
    pub reference_by_node: Vec<Option<ReferenceDefinition>>,
    pub csli_by_node: Vec<Option<CsliDefinition>>,
    pub cast_attribute_lists: Vec<CastAttributeList>,
    pub cast_attribute_list_by_node: Vec<Option<usize>>,
}

impl Layer {
    pub fn classify_cast(&self, node_index: usize) -> Option<CastClassification> {
        let node = self.nodes.get(node_index)?;
        let image = self.image_by_node.get(node_index).and_then(Option::as_ref);
        let mut present_payloads = CastPayloads::default();
        if image.is_some() {
            present_payloads |= CastPayloads::IMAGE;
        }
        if self
            .csli_by_node
            .get(node_index)
            .is_some_and(Option::is_some)
        {
            present_payloads |= CastPayloads::SLICE;
        }
        if self
            .reference_by_node
            .get(node_index)
            .is_some_and(Option::is_some)
        {
            present_payloads |= CastPayloads::REFERENCE;
        }
        if self
            .number_by_node
            .get(node_index)
            .is_some_and(Option::is_some)
        {
            present_payloads |= CastPayloads::NUMBER;
        }

        let serialized_type = node.cast_type();
        let kind = match serialized_type {
            Some(1) if image.is_some_and(ImageDefinition::creates_text_cast) => SrCastKind::Text,
            Some(1) => SrCastKind::Image,
            Some(2) => SrCastKind::Slice,
            Some(3) => SrCastKind::Reference,
            Some(4) => SrCastKind::Number,
            Some(0) | None | Some(_) => SrCastKind::Null,
        };
        let inactive_text_payload = serialized_type == Some(1)
            && image.is_some_and(|image| image.has_text_child && !image.creates_text_cast());

        Some(CastClassification {
            kind,
            serialized_type,
            present_payloads,
            inactive_text_payload,
        })
    }

    pub fn from_block(file: &SrdFile, block: &Block) -> Result<Self, SceneError> {
        if !block.is_tag(b"LAYR") {
            return Err(SceneError("block is not LAYR".into()));
        }

        let name = required_property(block, 0x03)?
            .string_bytes(file)
            .ok_or_else(|| SceneError("LAYR property 0x03 is not a string".into()))?
            .to_vec();
        let flags = read_unsigned(file, block, 0x20)?;
        let node_count = read_unsigned(file, block, 0x21)?;
        let animation_count = read_unsigned(file, block, 0x22)?;
        let animations = block
            .children
            .iter()
            .filter(|child| child.is_tag(b"ANIM"))
            .map(|child| {
                AnimationDefinition::from_block(file, child)
                    .map_err(|error| SceneError(error.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        validate_declared_count(animation_count, animations.len(), "LAYR ANIM", block.offset)?;
        let field_23 = block
            .properties_with_code(0x23)
            .map(|property| {
                property
                    .read_unsigned_scalar(file)
                    .ok_or_else(|| SceneError("invalid LAYR property 0x23".into()))
                    .and_then(|value| {
                        u8::try_from(value)
                            .map_err(|_| SceneError("LAYR property 0x23 exceeds u8".into()))
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let cast = block
            .children
            .iter()
            .rev()
            .find(|child| child.is_tag(b"CAST"))
            .ok_or_else(|| SceneError("LAYR has no CAST child".into()))?;
        let node_block = cast
            .children
            .iter()
            .rev()
            .find(|child| child.is_tag(b"NODE"))
            .ok_or_else(|| SceneError("CAST has no NODE child".into()))?;
        let node_count = usize::try_from(node_count)
            .map_err(|_| SceneError("LAYR node count does not fit usize".into()))?;
        let node_properties = split_records(&node_block.properties, node_count, "NODE")?;
        let nodes = node_properties
            .into_iter()
            .map(|record| parse_node(file, &record))
            .collect::<Result<Vec<_>, _>>()?;

        let is_2d = flags & 1 == 0;
        let transform_tag = if is_2d { b"TRS2" } else { b"TRS3" };
        let transform_block = cast
            .children
            .iter()
            .rev()
            .find(|child| child.is_tag(transform_tag))
            .ok_or_else(|| {
                SceneError(format!(
                    "CAST has no {} child selected by LAYR flags",
                    String::from_utf8_lossy(transform_tag)
                ))
            })?;
        let transform_properties = split_records(
            &transform_block.properties,
            node_count,
            std::str::from_utf8(transform_tag).unwrap(),
        )?;
        let transforms = transform_properties
            .into_iter()
            .map(|record| {
                if is_2d {
                    parse_trs2(file, &record).map(RawTransform::Trs2)
                } else {
                    parse_trs3(file, &record).map(RawTransform::Trs3)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        let data_children = cast
            .children
            .iter()
            .filter(|child| child.is_tag(b"DATA"))
            .flat_map(|data| data.children.iter());
        let mut image_by_node = vec![None; node_count];
        let mut number_by_node = vec![None; node_count];
        let mut reference_by_node = vec![None; node_count];
        let mut csli_by_node = vec![None; node_count];
        for data_block in data_children {
            if data_block.is_tag(b"CIMG") {
                let image = ImageDefinition::from_block(file, data_block)
                    .map_err(|error| SceneError(error.to_string()))?;
                let index = usize::try_from(image.node_index).map_err(|_| {
                    SceneError(format!(
                        "CIMG at {:#x} has negative NODE index {}",
                        data_block.offset, image.node_index
                    ))
                })?;
                let destination = image_by_node.get_mut(index).ok_or_else(|| {
                    SceneError(format!(
                        "CIMG at {:#x} references NODE {index} outside {node_count} nodes",
                        data_block.offset
                    ))
                })?;
                *destination = Some(image);
                continue;
            }
            if data_block.is_tag(b"CNUM") {
                let number = NumberDefinition::from_block(file, data_block)
                    .map_err(|error| SceneError(error.to_string()))?;
                let index = usize::try_from(number.node_index).map_err(|_| {
                    SceneError(format!(
                        "CNUM at {:#x} has negative NODE index {}",
                        data_block.offset, number.node_index
                    ))
                })?;
                let destination = number_by_node.get_mut(index).ok_or_else(|| {
                    SceneError(format!(
                        "CNUM at {:#x} references NODE {index} outside {node_count} nodes",
                        data_block.offset
                    ))
                })?;
                *destination = Some(number);
                continue;
            }
            if data_block.is_tag(b"CRFD") {
                let reference = ReferenceDefinition::from_block(file, data_block)
                    .map_err(|error| SceneError(error.to_string()))?;
                let index = usize::try_from(reference.node_index).map_err(|_| {
                    SceneError(format!(
                        "CRFD at {:#x} has negative NODE index {}",
                        data_block.offset, reference.node_index
                    ))
                })?;
                let destination = reference_by_node.get_mut(index).ok_or_else(|| {
                    SceneError(format!(
                        "CRFD at {:#x} references NODE {index} outside {node_count} nodes",
                        data_block.offset
                    ))
                })?;
                *destination = Some(reference);
                continue;
            }
            if !data_block.is_tag(b"CSLI") {
                continue;
            }
            let csli_block = data_block;
            let csli = CsliDefinition::from_block(file, csli_block)
                .map_err(|error| SceneError(error.to_string()))?;
            let index = usize::try_from(csli.node_index).map_err(|_| {
                SceneError(format!(
                    "CSLI at {:#x} has negative NODE index {}",
                    csli_block.offset, csli.node_index
                ))
            })?;
            let destination = csli_by_node.get_mut(index).ok_or_else(|| {
                SceneError(format!(
                    "CSLI at {:#x} references NODE {index} outside {node_count} nodes",
                    csli_block.offset
                ))
            })?;
            *destination = Some(csli);
        }

        let mut cast_attribute_lists = Vec::new();
        let mut cast_attribute_list_by_node = vec![None; node_count];
        for catr in cast
            .children
            .iter()
            .filter(|child| child.is_tag(b"CATL"))
            .flat_map(|catl| catl.children.iter().filter(|child| child.is_tag(b"CATR")))
        {
            let list = CastAttributeList::from_block(file, catr)
                .map_err(|error| SceneError(error.to_string()))?;
            let list_index = cast_attribute_lists.len();
            if let Some(node_index) = list.node_index.filter(|index| *index >= 0) {
                let node_index = usize::try_from(node_index).unwrap();
                let destination = cast_attribute_list_by_node.get_mut(node_index).ok_or_else(
                    || {
                        SceneError(format!(
                            "CATR at {:#x} references NODE {node_index} outside {node_count} nodes",
                            catr.offset
                        ))
                    },
                )?;
                *destination = Some(list_index);
            }
            cast_attribute_lists.push(list);
        }

        Ok(Self {
            name,
            flags,
            animation_count,
            animations,
            field_23,
            nodes,
            transforms,
            image_by_node,
            number_by_node,
            reference_by_node,
            csli_by_node,
            cast_attribute_lists,
            cast_attribute_list_by_node,
        })
    }

    pub fn is_2d(&self) -> bool {
        self.flags & 1 == 0
    }

    pub fn find_animation(&self, name: &[u8]) -> Option<(usize, &AnimationDefinition)> {
        self.animations
            .iter()
            .enumerate()
            .find(|(_, animation)| animation.name == name)
    }

    pub fn ext_param_for_node(&self, node_index: usize) -> Option<&ExtParamData> {
        let list_index = self.cast_attribute_list_by_node.get(node_index)?.as_ref()?;
        self.cast_attribute_lists.get(*list_index)?.ext_param()
    }

    pub fn font_param_for_node(&self, node_index: usize) -> Option<FontParamData> {
        let list_index = self.cast_attribute_list_by_node.get(node_index)?.as_ref()?;
        self.cast_attribute_lists.get(*list_index)?.font_param()
    }

    /// Reproduces the inherited key stored at runtime CAST `+0x50` by
    /// `srd_update_cast_tree`. NODE `0xA0` is deliberately not added here:
    /// the render wrapper applies that offset only after loading this key.
    pub fn compose_runtime_cast_layer_keys(&self, inherited: u32) -> Result<Vec<u32>, SceneError> {
        let hierarchy = self.build_hierarchy()?;
        let mut keys = vec![inherited; self.nodes.len()];
        let mut stack = hierarchy
            .roots
            .iter()
            .rev()
            .map(|&node_index| (node_index, inherited))
            .collect::<Vec<_>>();
        while let Some((node_index, parent_key)) = stack.pop() {
            let key = self
                .ext_param_for_node(node_index)
                .copied()
                .unwrap_or_default()
                .compose_layer_key(parent_key);
            keys[node_index] = key;
            for &child_index in hierarchy.children[node_index].iter().rev() {
                stack.push((child_index, key));
            }
        }
        Ok(keys)
    }

    pub fn build_hierarchy(&self) -> Result<Hierarchy, SceneError> {
        let count = self.nodes.len();
        let mut parents = vec![None; count];
        let mut children = vec![Vec::new(); count];

        for (parent, parent_children) in children.iter_mut().enumerate() {
            let mut child = self.nodes[parent].first_child_index;
            let mut traversed = 0usize;
            while child != -1 {
                if traversed >= count {
                    return Err(SceneError(format!(
                        "NODE sibling chain for parent {parent} does not terminate"
                    )));
                }
                let child_index = usize::try_from(child).map_err(|_| {
                    SceneError(format!(
                        "NODE parent {parent} has negative child index {child}"
                    ))
                })?;
                let child_node = self.nodes.get(child_index).ok_or_else(|| {
                    SceneError(format!(
                        "NODE parent {parent} references child {child_index} outside {count} nodes"
                    ))
                })?;
                parents[child_index] = Some(parent);
                parent_children.push(child_index);
                child = child_node.next_sibling_index;
                traversed += 1;
            }
        }

        let roots = parents
            .iter()
            .enumerate()
            .filter_map(|(index, parent)| parent.is_none().then_some(index))
            .collect();
        Ok(Hierarchy {
            parents,
            children,
            roots,
        })
    }

    pub fn compose_world_matrices(
        &self,
        transforms: &[SpatialTransform],
        root_matrix: Affine3x4,
        flip_y: bool,
        offsets: &[[f32; 2]],
    ) -> Result<Vec<Affine3x4>, SceneError> {
        self.compose_world_matrices_with_runtime_mode(
            transforms,
            root_matrix,
            self.is_2d(),
            flip_y,
            offsets,
        )
    }

    pub fn compose_world_matrices_with_runtime_mode(
        &self,
        transforms: &[SpatialTransform],
        root_matrix: Affine3x4,
        is_2d: bool,
        flip_y: bool,
        offsets: &[[f32; 2]],
    ) -> Result<Vec<Affine3x4>, SceneError> {
        let count = self.nodes.len();
        if transforms.len() != count || offsets.len() != count {
            return Err(SceneError(format!(
                "world composition needs {count} transforms and offsets, got {} and {}",
                transforms.len(),
                offsets.len()
            )));
        }
        let hierarchy = self.build_hierarchy()?;
        let mut worlds = vec![Affine3x4::IDENTITY; count];
        let mut visited = vec![false; count];
        for &root in &hierarchy.roots {
            compose_node(
                root,
                root_matrix,
                is_2d,
                flip_y,
                transforms,
                offsets,
                &hierarchy.children,
                &mut worlds,
                &mut visited,
            )?;
        }
        if let Some(index) = visited.iter().position(|value| !value) {
            return Err(SceneError(format!(
                "NODE {index} is not reachable from a root CAST"
            )));
        }
        Ok(worlds)
    }

    pub fn compute_parent_csli_offsets(&self) -> Result<Vec<[f32; 2]>, SceneError> {
        let count = self.nodes.len();
        let hierarchy = self.build_hierarchy()?;
        let generated = self
            .csli_by_node
            .iter()
            .map(|definition| {
                definition
                    .as_ref()
                    .map(CsliDefinition::generate_cell_rects)
                    .transpose()
                    .map_err(|error| SceneError(error.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut offsets = vec![[0.0, 0.0]; count];
        for (node_index, node) in self.nodes.iter().enumerate() {
            let Some(cell_index) = node.parent_csli_cell_index else {
                continue;
            };
            let Some(parent_index) = hierarchy.parents[node_index] else {
                continue;
            };
            let Some(parent_cells) = generated[parent_index].as_deref() else {
                continue;
            };
            let parent_definition = self.csli_by_node[parent_index].as_ref().unwrap();
            offsets[node_index] = parent_cell_center_offset(
                cell_index,
                parent_cells,
                parent_definition.runtime_origin_offset(),
                self.is_2d(),
            );
        }
        Ok(offsets)
    }

    pub fn compose_world_matrices_with_csli_layout(
        &self,
        transforms: &[SpatialTransform],
        root_matrix: Affine3x4,
        flip_y: bool,
    ) -> Result<Vec<Affine3x4>, SceneError> {
        self.compose_world_matrices_with_runtime_mode_and_csli_layout(
            transforms,
            root_matrix,
            self.is_2d(),
            flip_y,
        )
    }

    pub fn compose_world_matrices_with_runtime_mode_and_csli_layout(
        &self,
        transforms: &[SpatialTransform],
        root_matrix: Affine3x4,
        is_2d: bool,
        flip_y: bool,
    ) -> Result<Vec<Affine3x4>, SceneError> {
        let offsets = self.compute_parent_csli_offsets()?;
        self.compose_world_matrices_with_runtime_mode(
            transforms,
            root_matrix,
            is_2d,
            flip_y,
            &offsets,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn compose_node(
    index: usize,
    parent_world: Affine3x4,
    is_2d: bool,
    flip_y: bool,
    transforms: &[SpatialTransform],
    offsets: &[[f32; 2]],
    children: &[Vec<usize>],
    worlds: &mut [Affine3x4],
    visited: &mut [bool],
) -> Result<(), SceneError> {
    if visited[index] {
        return Err(SceneError(format!(
            "NODE {index} is reached more than once in the CAST hierarchy"
        )));
    }
    visited[index] = true;
    let local = build_local_matrix(&transforms[index], is_2d, flip_y, offsets[index]);
    let world = parent_world.mul_game(local);
    worlds[index] = world;
    for &child in &children[index] {
        compose_node(
            child, world, is_2d, flip_y, transforms, offsets, children, worlds, visited,
        )?;
    }
    Ok(())
}

fn split_records<'a>(
    properties: &'a [Property],
    count: usize,
    tag: &str,
) -> Result<Vec<Vec<&'a Property>>, SceneError> {
    let mut records = vec![Vec::new(); count];
    let mut index = 0usize;
    for property in properties {
        if property.code == 0xfe {
            index = index
                .checked_add(1)
                .ok_or_else(|| SceneError(format!("{tag} record index overflow")))?;
        } else {
            let record = records.get_mut(index).ok_or_else(|| {
                SceneError(format!("{tag} property follows the final record separator"))
            })?;
            record.push(property);
        }
    }
    Ok(records)
}

fn parse_node(file: &SrdFile, properties: &[&Property]) -> Result<NodeRecord, SceneError> {
    let mut record = NodeRecord {
        name: None,
        type_flags: None,
        parent_csli_cell_index: None,
        first_child_index: -1,
        next_sibling_index: -1,
        field_a0: None,
    };
    for property in properties {
        match property.code {
            0x03 => {
                record.name = Some(
                    property
                        .string_bytes(file)
                        .ok_or_else(|| SceneError("NODE property 0x03 is not a string".into()))?
                        .iter()
                        .copied()
                        .take(64)
                        .collect(),
                )
            }
            0x30 => record.type_flags = property.read_unsigned_scalar(file),
            0x32 => record.parent_csli_cell_index = property.read_signed_scalar(file),
            0x3c => {
                record.first_child_index = property
                    .read_signed_scalar(file)
                    .ok_or_else(|| SceneError("invalid NODE property 0x3c".into()))?
                    as i16
            }
            0x3d => {
                record.next_sibling_index = property
                    .read_signed_scalar(file)
                    .ok_or_else(|| SceneError("invalid NODE property 0x3d".into()))?
                    as i16
            }
            0xa0 => record.field_a0 = property.read_signed_scalar(file),
            _ => {}
        }
    }
    Ok(record)
}

fn parse_trs2(file: &SrdFile, properties: &[&Property]) -> Result<SpatialTransform, SceneError> {
    let mut transform = SpatialTransform::default();
    for property in properties {
        match property.code {
            0x34 => {
                transform.translation[..2].copy_from_slice(&read_f32_vector::<2>(file, property)?)
            }
            0x35 => transform.rotation[2] = signed_scalar(file, property, "TRS2 0x35")?,
            0x36 => transform.scale[..2].copy_from_slice(&read_f32_vector::<2>(file, property)?),
            0x3a => transform.multiply_color = read_runtime_color(file, property, "TRS2 0x3a")?,
            0x33 => transform.additive_color = read_runtime_color(file, property, "TRS2 0x33")?,
            0x3b => {
                transform.visibility_word =
                    u32::from(signed_scalar(file, property, "TRS2 0x3b")? != 0)
            }
            _ => {}
        }
    }
    Ok(transform)
}

fn parse_trs3(file: &SrdFile, properties: &[&Property]) -> Result<SpatialTransform, SceneError> {
    let mut transform = SpatialTransform::default();
    for property in properties {
        match property.code {
            0x37 => transform.translation = read_f32_vector::<3>(file, property)?,
            0x38 => transform.rotation = read_i32_vector::<3>(file, property)?,
            0x39 => transform.scale = read_f32_vector::<3>(file, property)?,
            0x3a => transform.multiply_color = read_runtime_color(file, property, "TRS3 0x3a")?,
            0x33 => transform.additive_color = read_runtime_color(file, property, "TRS3 0x33")?,
            0x3b => {
                transform.visibility_word =
                    u32::from(signed_scalar(file, property, "TRS3 0x3b")? != 0)
            }
            _ => {}
        }
    }
    Ok(transform)
}

fn read_runtime_color(
    file: &SrdFile,
    property: &Property,
    label: &str,
) -> Result<[u8; 4], SceneError> {
    let bytes = property.value_bytes(file);
    if bytes.len() < 4 {
        return Err(SceneError(format!("{label} has fewer than four bytes")));
    }
    Ok([bytes[3], bytes[2], bytes[1], bytes[0]])
}

fn read_f32_vector<const N: usize>(
    file: &SrdFile,
    property: &Property,
) -> Result<[f32; N], SceneError> {
    let values = scalar_chunks(property, file)?;
    if values.len() < N {
        return Err(SceneError(format!(
            "property {:#04x} has {} values, expected {N}",
            property.code,
            values.len()
        )));
    }
    let mut result = [0.0; N];
    for (destination, source) in result.iter_mut().zip(values) {
        *destination = scalar_as_f32(property.type_code, source)?;
    }
    Ok(result)
}

fn read_i32_vector<const N: usize>(
    file: &SrdFile,
    property: &Property,
) -> Result<[i32; N], SceneError> {
    let values = scalar_chunks(property, file)?;
    if values.len() < N {
        return Err(SceneError(format!(
            "property {:#04x} has {} values, expected {N}",
            property.code,
            values.len()
        )));
    }
    let mut result = [0; N];
    for (destination, source) in result.iter_mut().zip(values) {
        *destination = scalar_as_i32(property.type_code, source)?;
    }
    Ok(result)
}

fn scalar_chunks<'a>(property: &Property, file: &'a SrdFile) -> Result<Vec<&'a [u8]>, SceneError> {
    let width = match property.type_code {
        1 | 3 | 4 => 1,
        5..=7 => 2,
        8..=12 => 4,
        _ => {
            return Err(SceneError(format!(
                "unsupported scalar type {} in property {:#04x}",
                property.type_code, property.code
            )));
        }
    };
    let bytes = property.value_bytes(file);
    if !bytes.len().is_multiple_of(width) {
        return Err(SceneError(format!(
            "property {:#04x} byte count is not scalar-aligned",
            property.code
        )));
    }
    Ok(bytes.chunks_exact(width).collect())
}

fn scalar_as_f32(type_code: u8, bytes: &[u8]) -> Result<f32, SceneError> {
    Ok(match type_code {
        1 | 4 => f32::from(bytes[0]),
        3 => f32::from(bytes[0] as i8),
        5 | 7 => f32::from(i16::from_le_bytes(bytes.try_into().unwrap())),
        6 => f32::from(u16::from_le_bytes(bytes.try_into().unwrap())),
        8 | 9 | 11 | 12 => i32::from_le_bytes(bytes.try_into().unwrap()) as f32,
        10 => f32::from_bits(u32::from_le_bytes(bytes.try_into().unwrap())),
        _ => return Err(SceneError(format!("unsupported scalar type {type_code}"))),
    })
}

fn scalar_as_i32(type_code: u8, bytes: &[u8]) -> Result<i32, SceneError> {
    Ok(match type_code {
        1 | 4 => i32::from(bytes[0]),
        3 => i32::from(bytes[0] as i8),
        5 | 7 => i32::from(i16::from_le_bytes(bytes.try_into().unwrap())),
        6 => i32::from(u16::from_le_bytes(bytes.try_into().unwrap())),
        8 | 9 | 11 | 12 => i32::from_le_bytes(bytes.try_into().unwrap()),
        10 => cvtt_f32_to_i32(f32::from_bits(u32::from_le_bytes(
            bytes.try_into().unwrap(),
        ))),
        _ => return Err(SceneError(format!("unsupported scalar type {type_code}"))),
    })
}

fn signed_scalar(file: &SrdFile, property: &Property, label: &str) -> Result<i32, SceneError> {
    property
        .read_signed_scalar(file)
        .ok_or_else(|| SceneError(format!("invalid {label}")))
}

fn required_property(block: &Block, code: u8) -> Result<&Property, SceneError> {
    block
        .last_property(code)
        .ok_or_else(|| SceneError(format!("missing property {code:#04x}")))
}

fn fixed_name(
    file: &SrdFile,
    block: &Block,
    code: u8,
    capacity: usize,
    label: &str,
) -> Result<Vec<u8>, SceneError> {
    required_property(block, code)?
        .string_bytes(file)
        .ok_or_else(|| SceneError(format!("{label} is not a string")))
        .map(|bytes| bytes.iter().copied().take(capacity).collect())
}

fn optional_fixed_name(
    file: &SrdFile,
    block: &Block,
    code: u8,
    capacity: usize,
    label: &str,
) -> Result<Vec<u8>, SceneError> {
    block
        .last_property(code)
        .map(|property| {
            property
                .string_bytes(file)
                .ok_or_else(|| SceneError(format!("{label} is not a string")))
                .map(|bytes| bytes.iter().copied().take(capacity).collect())
        })
        .transpose()
        .map(Option::unwrap_or_default)
}

fn validate_declared_count(
    declared: u32,
    parsed: usize,
    label: &str,
    offset: usize,
) -> Result<(), SceneError> {
    let declared = usize::try_from(declared)
        .map_err(|_| SceneError(format!("{label} count does not fit usize")))?;
    if declared != parsed {
        return Err(SceneError(format!(
            "{label} count mismatch at {offset:#x}: declared {declared}, parsed {parsed}"
        )));
    }
    Ok(())
}

fn validate_signed_declared_count(
    declared: i32,
    parsed: usize,
    label: &str,
    offset: usize,
) -> Result<(), SceneError> {
    let declared = usize::try_from(declared)
        .map_err(|_| SceneError(format!("{label} count is negative: {declared}")))?;
    if declared != parsed {
        return Err(SceneError(format!(
            "{label} count mismatch at {offset:#x}: declared {declared}, parsed {parsed}"
        )));
    }
    Ok(())
}

fn read_unsigned(file: &SrdFile, block: &Block, code: u8) -> Result<u32, SceneError> {
    required_property(block, code)?
        .read_unsigned_scalar(file)
        .ok_or_else(|| SceneError(format!("invalid property {code:#04x}")))
}

fn optional_unsigned(file: &SrdFile, block: &Block, code: u8) -> Result<Option<u32>, SceneError> {
    block
        .last_property(code)
        .map(|property| {
            property
                .read_unsigned_scalar(file)
                .ok_or_else(|| SceneError(format!("invalid property {code:#04x}")))
        })
        .transpose()
}

fn optional_signed(file: &SrdFile, block: &Block, code: u8) -> Result<Option<i32>, SceneError> {
    block
        .last_property(code)
        .map(|property| {
            property
                .read_signed_scalar(file)
                .ok_or_else(|| SceneError(format!("invalid property {code:#04x}")))
        })
        .transpose()
}

fn optional_float(file: &SrdFile, block: &Block, code: u8) -> Result<f32, SceneError> {
    block
        .last_property(code)
        .map(|property| {
            property
                .read_scalar_as_f32(file)
                .ok_or_else(|| SceneError(format!("invalid property {code:#04x}")))
        })
        .transpose()
        .map(|value| value.unwrap_or(0.0))
}

fn cvtt_f32_to_i32(value: f32) -> i32 {
    if !value.is_finite() || !(-2_147_483_648.0..2_147_483_648.0).contains(&value) {
        i32::MIN
    } else {
        value.trunc() as i32
    }
}

#[cfg(test)]
mod cast_classification_tests {
    use super::*;

    fn image(flags: u32, has_text_child: bool) -> ImageDefinition {
        ImageDefinition {
            flags,
            width: 128.0,
            height: 128.0,
            custom_origin: [0.0; 2],
            origin_mode: 0,
            vertex_colors: [[0xff; 4]; 4],
            cref_index: -1,
            cref_count: 0,
            crefs: Vec::new(),
            field_4c: 0,
            cre1_index: -1,
            cre1_count: 0,
            cre1s: Vec::new(),
            coordinate_offsets: [[0.0; 2]; 2],
            field_a1: 0,
            node_index: 0,
            has_text_child,
            text: None,
        }
    }

    fn layer(serialized_type: Option<u8>, image: Option<ImageDefinition>) -> Layer {
        Layer {
            name: Vec::new(),
            flags: 0,
            animation_count: 0,
            animations: Vec::new(),
            field_23: Vec::new(),
            nodes: vec![NodeRecord {
                name: None,
                type_flags: serialized_type.map(u32::from),
                parent_csli_cell_index: None,
                first_child_index: -1,
                next_sibling_index: -1,
                field_a0: None,
            }],
            transforms: Vec::new(),
            image_by_node: vec![image],
            number_by_node: vec![None],
            reference_by_node: vec![None],
            csli_by_node: vec![None],
            cast_attribute_lists: Vec::new(),
            cast_attribute_list_by_node: vec![None],
        }
    }

    #[test]
    fn classification_matches_runtime_factory_for_every_node_type() {
        for (serialized_type, expected) in [
            (None, SrCastKind::Null),
            (Some(0), SrCastKind::Null),
            (Some(1), SrCastKind::Image),
            (Some(2), SrCastKind::Slice),
            (Some(3), SrCastKind::Reference),
            (Some(4), SrCastKind::Number),
            (Some(7), SrCastKind::Null),
        ] {
            assert_eq!(
                layer(serialized_type, None).classify_cast(0).unwrap().kind,
                expected
            );
        }
    }

    #[test]
    fn type_one_uses_the_exact_text_factory_condition() {
        let text = layer(Some(1), Some(image(0x100, true)))
            .classify_cast(0)
            .unwrap();
        assert_eq!(text.kind, SrCastKind::Text);
        assert!(!text.inactive_text_payload);
        assert!(!text.missing_expected_payload());

        let inactive = layer(Some(1), Some(image(0, true)))
            .classify_cast(0)
            .unwrap();
        assert_eq!(inactive.kind, SrCastKind::Image);
        assert!(inactive.inactive_text_payload);
    }

    #[test]
    fn diagnostics_preserve_runtime_kind_for_malformed_payloads() {
        let slice = layer(Some(2), Some(image(0, false)))
            .classify_cast(0)
            .unwrap();
        assert_eq!(slice.kind, SrCastKind::Slice);
        assert!(slice.missing_expected_payload());
        assert_eq!(slice.extra_payloads(), CastPayloads::IMAGE);

        let unsupported = layer(Some(7), Some(image(0, false)))
            .classify_cast(0)
            .unwrap();
        assert_eq!(unsupported.kind, SrCastKind::Null);
        assert_eq!(unsupported.unsupported_serialized_type(), Some(7));
        assert_eq!(unsupported.extra_payloads(), CastPayloads::IMAGE);
    }
}
