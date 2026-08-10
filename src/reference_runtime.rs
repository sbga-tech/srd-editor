use std::collections::BTreeSet;
use std::fmt;

use crate::animation::RuntimeAnimationState;
use crate::csli::{add_color_saturating_game, multiply_color_game};
use crate::image::{ImageDefinition, RuntimeImageState};
use crate::projection::{Matrix4x4, inverse_matrix4x4_game};
use crate::reference::ReferenceAnimationRequest;
use crate::scene::{AnimationSetDefinition, Layer, Project, ReferenceTarget, SceneError};
use crate::texture::TextureList;
use crate::transform::{Affine3x4, SpatialTransform, build_local_matrix};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReferenceLayerParent {
    ProjectLayer(ReferenceTarget),
    ReferenceInstance(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceLayerInstance {
    pub parent: ReferenceLayerParent,
    pub reference_node_index: usize,
    pub target: ReferenceTarget,
    pub is_2d: bool,
    pub flip_y: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeWorldState {
    pub matrix: Affine3x4,
    pub multiply_color: [u8; 4],
    pub additive_color: [u8; 4],
    pub visible: bool,
    pub render_gate: bool,
}

impl Default for RuntimeWorldState {
    fn default() -> Self {
        Self {
            matrix: Affine3x4::IDENTITY,
            multiply_color: [255; 4],
            additive_color: [0; 4],
            visible: true,
            render_gate: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReferenceLayerLocalState {
    pub transform: SpatialTransform,
    pub enabled: bool,
}

impl Default for ReferenceLayerLocalState {
    fn default() -> Self {
        Self {
            transform: SpatialTransform::default(),
            enabled: true,
        }
    }
}

impl ReferenceLayerInstance {
    pub fn compose_world_state(
        &self,
        parent_cast: RuntimeWorldState,
        local: ReferenceLayerLocalState,
    ) -> RuntimeWorldState {
        let local_matrix =
            build_local_matrix(&local.transform, self.is_2d, self.flip_y, [0.0, 0.0]);
        RuntimeWorldState {
            matrix: parent_cast.matrix.mul_game(local_matrix),
            multiply_color: multiply_color_game(
                parent_cast.multiply_color,
                local.transform.multiply_color,
            ),
            additive_color: add_color_saturating_game(
                parent_cast.additive_color,
                local.transform.additive_color,
            ),
            visible: parent_cast.visible && local.enabled,
            render_gate: parent_cast.render_gate && local.enabled,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedReference {
    pub parent: ReferenceLayerParent,
    pub reference_node_index: usize,
    pub source_name: Vec<u8>,
    pub layer_name: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReferenceRuntimePlan {
    pub instances: Vec<ReferenceLayerInstance>,
    pub unresolved: Vec<UnresolvedReference>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceRuntimeAncestryError {
    InvalidReferenceInstance { instance_index: usize },
    ReferenceCycle { instance_index: usize },
}

impl fmt::Display for ReferenceRuntimeAncestryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidReferenceInstance { instance_index } => {
                write!(
                    formatter,
                    "reference instance {instance_index} is outside the runtime plan"
                )
            }
            Self::ReferenceCycle { instance_index } => {
                write!(
                    formatter,
                    "reference instance ancestry cycles at {instance_index}"
                )
            }
        }
    }
}

impl std::error::Error for ReferenceRuntimeAncestryError {}

/// One non-reference CAST in the exact structural traversal produced by
/// `srd_render_runtime_layer`: layer CAST vectors are visited forward, and a
/// resolved RefCast recursively expands its copied layer at that NODE
/// position before the parent vector continues.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeCastDrawOrderEntry {
    pub owner: ReferenceLayerParent,
    pub source: ReferenceTarget,
    pub node_index: usize,
    /// Effective `SrRenderer+0x198` value at the CAST's draw call, after the
    /// inherited ExtParamData key and NODE `+0x58` low-byte offset.
    pub renderer_layer_key: u32,
}

impl ReferenceRuntimePlan {
    /// Finds the original ProjectLayer whose RefCast expansion owns `owner`.
    ///
    /// A copied reference draw keeps its source layer identity for assets but
    /// inherits visibility and layer controls from this root ProjectLayer.
    pub fn root_project_layer(
        &self,
        mut owner: ReferenceLayerParent,
    ) -> Result<ReferenceTarget, ReferenceRuntimeAncestryError> {
        let mut visited_instances = BTreeSet::new();
        loop {
            match owner {
                ReferenceLayerParent::ProjectLayer(target) => return Ok(target),
                ReferenceLayerParent::ReferenceInstance(instance_index) => {
                    if !visited_instances.insert(instance_index) {
                        return Err(ReferenceRuntimeAncestryError::ReferenceCycle {
                            instance_index,
                        });
                    }
                    owner = self
                        .instances
                        .get(instance_index)
                        .ok_or(ReferenceRuntimeAncestryError::InvalidReferenceInstance {
                            instance_index,
                        })?
                        .parent;
                }
            }
        }
    }
    pub fn structural_cast_draw_order(
        &self,
        project: &Project,
        scene_index: usize,
        root_layer_key: u32,
    ) -> Result<Vec<RuntimeCastDrawOrderEntry>, SceneError> {
        let mut output = Vec::new();
        let Some(scene) = project.scenes.get(scene_index) else {
            return Ok(output);
        };
        for layer_index in 0..scene.layers.len() {
            let target = ReferenceTarget {
                scene_index,
                layer_index,
            };
            self.append_structural_cast_draw_order(
                project,
                ReferenceLayerParent::ProjectLayer(target),
                target,
                root_layer_key,
                &mut output,
            )?;
        }
        Ok(output)
    }

    fn append_structural_cast_draw_order(
        &self,
        project: &Project,
        owner: ReferenceLayerParent,
        source: ReferenceTarget,
        inherited_layer_key: u32,
        output: &mut Vec<RuntimeCastDrawOrderEntry>,
    ) -> Result<(), SceneError> {
        let layer = &project.scenes[source.scene_index].layers[source.layer_index];
        let cast_layer_keys = layer.compose_runtime_cast_layer_keys(inherited_layer_key)?;
        for node_index in 0..layer.nodes.len() {
            let cast_layer_key = cast_layer_keys[node_index];
            let node_offset = layer.nodes[node_index].render_layer_offset();
            if layer
                .reference_by_node
                .get(node_index)
                .and_then(Option::as_ref)
                .is_some()
            {
                if let Some((instance_index, instance)) =
                    self.instances.iter().enumerate().find(|(_, instance)| {
                        instance.parent == owner && instance.reference_node_index == node_index
                    })
                {
                    self.append_structural_cast_draw_order(
                        project,
                        ReferenceLayerParent::ReferenceInstance(instance_index),
                        instance.target,
                        replace_low_byte_with_wrapping_sum(cast_layer_key, node_offset),
                        output,
                    )?;
                }
                // A RefCast itself submits no primitive. An unresolved
                // reference has a null copied-layer pointer and contributes
                // no recursive entries.
                continue;
            }
            output.push(RuntimeCastDrawOrderEntry {
                owner,
                source,
                node_index,
                renderer_layer_key: cast_layer_key.wrapping_add(u32::from(node_offset)),
            });
        }
        Ok(())
    }
}

/// `srd_update_cast_tree` passes a RefCast's key into its copied layer by
/// replacing only the low byte with the wrapping sum. Unlike the later render
/// wrapper, a carry from that byte is deliberately discarded here.
const fn replace_low_byte_with_wrapping_sum(key: u32, offset: u8) -> u32 {
    (key & !0xff) | ((key as u8).wrapping_add(offset) as u32)
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceLayerRuntimeState {
    pub instance_index: usize,
    pub local: ReferenceLayerLocalState,
    pub cast_transforms: Vec<SpatialTransform>,
    pub image_bases: Vec<ImageDefinition>,
    pub image_states: Vec<RuntimeImageState>,
    pub animations: Vec<RuntimeAnimationState>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuntimeAnimationApplication {
    pub common_channels: usize,
    pub image_channels: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuntimeAnimationTreeApplication {
    pub animated_layers: usize,
    pub common_channels: usize,
    pub image_channels: usize,
    pub reference_requests: usize,
}

impl RuntimeAnimationTreeApplication {
    fn include_layer(&mut self, application: RuntimeAnimationApplication) {
        self.animated_layers += 1;
        self.common_channels += application.common_channels;
        self.image_channels += application.image_channels;
    }

    fn include_tree(&mut self, child: Self) {
        self.animated_layers += child.animated_layers;
        self.common_channels += child.common_channels;
        self.image_channels += child.image_channels;
        self.reference_requests += child.reference_requests;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceRuntime {
    pub plan: ReferenceRuntimePlan,
    pub layers: Vec<ReferenceLayerRuntimeState>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectLayerRuntimeState {
    pub target: ReferenceTarget,
    pub enabled: bool,
    pub cast_transforms: Vec<SpatialTransform>,
    pub image_bases: Vec<ImageDefinition>,
    pub image_states: Vec<RuntimeImageState>,
    pub animations: Vec<RuntimeAnimationState>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectRuntime {
    pub project_layers: Vec<Vec<ProjectLayerRuntimeState>>,
    pub references: ReferenceRuntime,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeLayerWorldStates {
    pub owner: ReferenceLayerParent,
    pub source: ReferenceTarget,
    pub is_2d: bool,
    pub layer: RuntimeWorldState,
    pub casts: Vec<RuntimeWorldState>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectRuntimeWorldStates {
    pub project_layers: Vec<Vec<RuntimeLayerWorldStates>>,
    pub references: Vec<RuntimeLayerWorldStates>,
}

impl ProjectRuntimeWorldStates {
    pub fn layer(&self, owner: ReferenceLayerParent) -> Option<&RuntimeLayerWorldStates> {
        match owner {
            ReferenceLayerParent::ProjectLayer(target) => self
                .project_layers
                .get(target.scene_index)?
                .get(target.layer_index),
            ReferenceLayerParent::ReferenceInstance(instance_index) => {
                self.references.get(instance_index)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceRuntimeError {
    pub repeating_layers: Vec<ReferenceTarget>,
}

impl fmt::Display for ReferenceRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("reference-layer construction does not converge through")?;
        for layer in &self.repeating_layers {
            write!(
                formatter,
                " SCN[{}]/LAYR[{}]",
                layer.scene_index, layer.layer_index
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for ReferenceRuntimeError {}

impl Project {
    pub fn build_reference_runtime_plan(
        &self,
    ) -> Result<ReferenceRuntimePlan, ReferenceRuntimeError> {
        let mut plan = ReferenceRuntimePlan::default();
        let mut lineages = Vec::new();

        // The binary resolves all CASTs in the original runtime scene table before it
        // drains the queue of newly constructed reference layers.
        for (scene_index, scene) in self.scenes.iter().enumerate() {
            for (layer_index, layer) in scene.layers.iter().enumerate() {
                let source = ReferenceTarget {
                    scene_index,
                    layer_index,
                };
                append_layer_references(
                    self,
                    source,
                    ReferenceLayerParent::ProjectLayer(source),
                    layer.is_2d(),
                    &[source],
                    &mut plan,
                    &mut lineages,
                )?;
            }
        }

        // Each appended instance corresponds to one fresh runtime layer. Processing
        // the growing vector in order reproduces the binary's successive queue waves.
        let mut instance_index = 0usize;
        while instance_index < plan.instances.len() {
            let instance = plan.instances[instance_index].clone();
            let lineage = lineages[instance_index].clone();
            append_layer_references(
                self,
                instance.target,
                ReferenceLayerParent::ReferenceInstance(instance_index),
                instance.is_2d,
                &lineage,
                &mut plan,
                &mut lineages,
            )?;
            instance_index += 1;
        }

        Ok(plan)
    }
}

impl ReferenceLayerRuntimeState {
    pub fn new(
        project: &Project,
        plan: &ReferenceRuntimePlan,
        instance_index: usize,
    ) -> Option<Self> {
        let instance = plan.instances.get(instance_index)?;
        let layer =
            &project.scenes[instance.target.scene_index].layers[instance.target.layer_index];
        let image_bases = runtime_image_bases(layer);
        let image_states = runtime_image_states(layer, &image_bases);
        Some(Self {
            instance_index,
            local: ReferenceLayerLocalState::default(),
            cast_transforms: layer
                .transforms
                .iter()
                .copied()
                .map(|transform| transform.spatial())
                .collect(),
            image_bases,
            image_states,
            animations: layer
                .animations
                .iter()
                .map(|animation| animation.initial_runtime_state())
                .collect(),
        })
    }

    pub fn apply_animation_common_channels(
        &mut self,
        project: &Project,
        plan: &ReferenceRuntimePlan,
        animation_name: &[u8],
        frame: f32,
    ) -> Result<Option<usize>, crate::animation::AnimationError> {
        let instance = &plan.instances[self.instance_index];
        let layer =
            &project.scenes[instance.target.scene_index].layers[instance.target.layer_index];
        let Some((animation_index, animation)) = layer.find_animation(animation_name) else {
            return Ok(None);
        };
        self.animations[animation_index].frame = frame;
        animation
            .apply_common_channels(&mut self.cast_transforms, frame)
            .map(Some)
    }

    pub fn apply_reference_request_common_channels(
        &mut self,
        project: &Project,
        plan: &ReferenceRuntimePlan,
        request: ReferenceAnimationRequest<'_>,
    ) -> Result<Option<usize>, crate::animation::AnimationError> {
        self.apply_animation_common_channels(project, plan, request.animation_name, request.frame)
    }

    pub fn apply_animation_channels(
        &mut self,
        project: &Project,
        plan: &ReferenceRuntimePlan,
        textures: &TextureList,
        animation_name: &[u8],
        frame: f32,
    ) -> Result<Option<RuntimeAnimationApplication>, crate::animation::AnimationError> {
        let instance = &plan.instances[self.instance_index];
        let layer =
            &project.scenes[instance.target.scene_index].layers[instance.target.layer_index];
        apply_runtime_layer_channels(
            layer,
            textures,
            animation_name,
            frame,
            &mut self.cast_transforms,
            &self.image_bases,
            &mut self.image_states,
            &mut self.animations,
        )
    }
}

fn runtime_image_bases(layer: &Layer) -> Vec<ImageDefinition> {
    layer
        .nodes
        .iter()
        .enumerate()
        .map(|(node_index, node)| match node.cast_type() {
            Some(1) => layer.image_by_node[node_index]
                .clone()
                .unwrap_or_else(ImageDefinition::srimage_constructor_base),
            Some(2) => layer.csli_by_node[node_index]
                .as_ref()
                .map(ImageDefinition::from_csli_runtime_base)
                .unwrap_or_else(ImageDefinition::srimage_constructor_base),
            Some(4) => layer.number_by_node[node_index]
                .as_ref()
                .map(crate::number::NumberDefinition::image_base)
                .unwrap_or_else(ImageDefinition::srimage_constructor_base),
            _ => ImageDefinition::srimage_constructor_base(),
        })
        .collect()
}

fn runtime_image_states(layer: &Layer, image_bases: &[ImageDefinition]) -> Vec<RuntimeImageState> {
    image_bases
        .iter()
        .enumerate()
        .map(|(node_index, image)| {
            let mut state = image.initial_runtime_state();
            if let Some(ext_param) = layer.ext_param_for_node(node_index) {
                state.render_preset_override = ext_param.render_preset_override;
            }
            state
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn apply_runtime_layer_channels(
    layer: &Layer,
    textures: &TextureList,
    animation_name: &[u8],
    frame: f32,
    cast_transforms: &mut [SpatialTransform],
    image_bases: &[ImageDefinition],
    image_states: &mut [RuntimeImageState],
    animations: &mut [RuntimeAnimationState],
) -> Result<Option<RuntimeAnimationApplication>, crate::animation::AnimationError> {
    let Some((animation_index, animation)) = layer.find_animation(animation_name) else {
        return Ok(None);
    };
    animations[animation_index].frame = frame;

    let mut application = RuntimeAnimationApplication::default();
    let cast_count = cast_transforms.len();
    for motion in &animation.motions {
        if motion.target < 0 {
            continue;
        }
        let node_index = usize::try_from(motion.target).map_err(|_| {
            crate::animation::AnimationError("MOT target does not fit usize".into())
        })?;
        if node_index >= cast_count {
            return Err(crate::animation::AnimationError(format!(
                "MOT target {node_index} is outside {cast_count} runtime CASTs"
            )));
        }
        application.common_channels +=
            motion.apply_proven_common_channels(&mut cast_transforms[node_index], frame);
        let base = &image_bases[node_index];
        let state = &mut image_states[node_index];
        for track in &motion.tracks {
            if base
                .apply_runtime_track(state, track, frame, textures)
                .map_err(|error| crate::animation::AnimationError(error.to_string()))?
            {
                application.image_channels += 1;
            }
        }
    }
    Ok(Some(application))
}

fn child_animation_requests(
    layer: &Layer,
    animation_name: &[u8],
    frame: f32,
    plan: &ReferenceRuntimePlan,
    parent: ReferenceLayerParent,
) -> Vec<(usize, Vec<u8>, f32)> {
    let Some((_, animation)) = layer.find_animation(animation_name) else {
        return Vec::new();
    };
    let mut requests = Vec::new();
    for motion in &animation.motions {
        let Ok(node_index) = usize::try_from(motion.target) else {
            continue;
        };
        let Some(reference) = layer
            .reference_by_node
            .get(node_index)
            .and_then(|definition| definition.as_ref())
        else {
            continue;
        };
        let child_instance = plan.instances.iter().position(|instance| {
            instance.parent == parent && instance.reference_node_index == node_index
        });
        let Some(child_instance) = child_instance else {
            continue;
        };
        for track in &motion.tracks {
            let Some(request) = reference.animation_request(track, frame) else {
                continue;
            };
            requests.push((
                child_instance,
                request.animation_name.to_vec(),
                request.frame,
            ));
        }
    }
    requests
}

impl ReferenceRuntime {
    pub fn new(project: &Project) -> Result<Self, ReferenceRuntimeError> {
        let plan = project.build_reference_runtime_plan()?;
        let layers = (0..plan.instances.len())
            .map(|instance_index| {
                ReferenceLayerRuntimeState::new(project, &plan, instance_index)
                    .expect("instance index came from the same reference runtime plan")
            })
            .collect();
        Ok(Self { plan, layers })
    }

    pub fn apply_instance_animation(
        &mut self,
        project: &Project,
        textures: &TextureList,
        instance_index: usize,
        animation_name: &[u8],
        frame: f32,
    ) -> Result<Option<RuntimeAnimationTreeApplication>, crate::animation::AnimationError> {
        if instance_index >= self.layers.len() {
            return Err(crate::animation::AnimationError(format!(
                "reference instance {instance_index} is outside {} runtime layers",
                self.layers.len()
            )));
        }

        let Some(application) = self.layers[instance_index].apply_animation_channels(
            project,
            &self.plan,
            textures,
            animation_name,
            frame,
        )?
        else {
            return Ok(None);
        };
        let mut tree = RuntimeAnimationTreeApplication::default();
        tree.include_layer(application);

        let target = self.plan.instances[instance_index].target;
        let layer = &project.scenes[target.scene_index].layers[target.layer_index];
        let child_requests = child_animation_requests(
            layer,
            animation_name,
            frame,
            &self.plan,
            ReferenceLayerParent::ReferenceInstance(instance_index),
        );

        for (child_instance, child_animation_name, child_frame) in child_requests {
            tree.reference_requests += 1;
            if let Some(child) = self.apply_instance_animation(
                project,
                textures,
                child_instance,
                &child_animation_name,
                child_frame,
            )? {
                tree.include_tree(child);
            }
        }
        Ok(Some(tree))
    }
}

impl ProjectLayerRuntimeState {
    fn new(target: ReferenceTarget, layer: &Layer) -> Self {
        let image_bases = runtime_image_bases(layer);
        let image_states = runtime_image_states(layer, &image_bases);
        Self {
            target,
            enabled: layer.flags & 0x100 != 0,
            cast_transforms: layer
                .transforms
                .iter()
                .copied()
                .map(|transform| transform.spatial())
                .collect(),
            image_bases,
            image_states,
            animations: layer
                .animations
                .iter()
                .map(|animation| animation.initial_runtime_state())
                .collect(),
        }
    }

    fn apply_animation_channels(
        &mut self,
        layer: &Layer,
        textures: &TextureList,
        animation_name: &[u8],
        frame: f32,
    ) -> Result<Option<RuntimeAnimationApplication>, crate::animation::AnimationError> {
        apply_runtime_layer_channels(
            layer,
            textures,
            animation_name,
            frame,
            &mut self.cast_transforms,
            &self.image_bases,
            &mut self.image_states,
            &mut self.animations,
        )
    }
}

impl ProjectRuntime {
    pub fn new(project: &Project) -> Result<Self, ReferenceRuntimeError> {
        let project_layers = project
            .scenes
            .iter()
            .enumerate()
            .map(|(scene_index, scene)| {
                scene
                    .layers
                    .iter()
                    .enumerate()
                    .map(|(layer_index, layer)| {
                        ProjectLayerRuntimeState::new(
                            ReferenceTarget {
                                scene_index,
                                layer_index,
                            },
                            layer,
                        )
                    })
                    .collect()
            })
            .collect();
        Ok(Self {
            project_layers,
            references: ReferenceRuntime::new(project)?,
        })
    }

    /// Reproduces the runtime update chain before render traversal: original
    /// layers use the host FirstCalcMatrix as their root, while every copied
    /// layer composes its independent local state under the owning RefCast.
    /// CAST colors and visibility follow `srd_compose_cast_world_state`; the
    /// render gate remains separate from transform visibility.
    pub fn compose_world_states(
        &self,
        project: &Project,
        first_calc_matrix: Affine3x4,
        renderer_inverse_camera_view: Affine3x4,
    ) -> Result<ProjectRuntimeWorldStates, SceneError> {
        let mut project_worlds = Vec::with_capacity(project.scenes.len());
        for (scene_index, scene) in project.scenes.iter().enumerate() {
            let runtime_scene = self.project_layers.get(scene_index).ok_or_else(|| {
                SceneError(format!(
                    "SCN[{scene_index}] is missing from the project runtime"
                ))
            })?;
            if runtime_scene.len() != scene.layers.len() {
                return Err(SceneError(format!(
                    "SCN[{scene_index}] has {} parsed layers but {} runtime layers",
                    scene.layers.len(),
                    runtime_scene.len()
                )));
            }
            let mut scene_worlds = Vec::with_capacity(scene.layers.len());
            for (layer_index, (layer, runtime_layer)) in
                scene.layers.iter().zip(runtime_scene).enumerate()
            {
                let source = ReferenceTarget {
                    scene_index,
                    layer_index,
                };
                let layer_world = RuntimeWorldState {
                    matrix: first_calc_matrix,
                    multiply_color: [255; 4],
                    additive_color: [0; 4],
                    visible: true,
                    render_gate: runtime_layer.enabled,
                };
                let casts = compose_runtime_cast_world_states(
                    layer,
                    &runtime_layer.cast_transforms,
                    layer_world,
                    layer.is_2d(),
                    renderer_inverse_camera_view,
                )?;
                scene_worlds.push(RuntimeLayerWorldStates {
                    owner: ReferenceLayerParent::ProjectLayer(source),
                    source,
                    is_2d: layer.is_2d(),
                    layer: layer_world,
                    casts,
                });
            }
            project_worlds.push(scene_worlds);
        }

        let mut result = ProjectRuntimeWorldStates {
            project_layers: project_worlds,
            references: Vec::with_capacity(self.references.layers.len()),
        };
        for (instance_index, runtime_layer) in self.references.layers.iter().enumerate() {
            let instance = self
                .references
                .plan
                .instances
                .get(instance_index)
                .ok_or_else(|| {
                    SceneError(format!(
                        "reference runtime layer {instance_index} has no construction plan entry"
                    ))
                })?;
            let parent_layer = result.layer(instance.parent).ok_or_else(|| {
                SceneError(format!(
                    "reference instance {instance_index} has an unavailable parent {:?}",
                    instance.parent
                ))
            })?;
            let parent_cast = *parent_layer
                .casts
                .get(instance.reference_node_index)
                .ok_or_else(|| {
                    SceneError(format!(
                        "reference instance {instance_index} owning NODE {} is outside its parent CAST vector",
                        instance.reference_node_index
                    ))
                })?;
            let layer_world = instance.compose_world_state(parent_cast, runtime_layer.local);
            let layer = project
                .scenes
                .get(instance.target.scene_index)
                .and_then(|scene| scene.layers.get(instance.target.layer_index))
                .ok_or_else(|| {
                    SceneError(format!(
                        "reference instance {instance_index} target SCN[{}]/LAYR[{}] is outside the project",
                        instance.target.scene_index, instance.target.layer_index
                    ))
                })?;
            let casts = compose_runtime_cast_world_states(
                layer,
                &runtime_layer.cast_transforms,
                layer_world,
                instance.is_2d,
                renderer_inverse_camera_view,
            )?;
            result.references.push(RuntimeLayerWorldStates {
                owner: ReferenceLayerParent::ReferenceInstance(instance_index),
                source: instance.target,
                is_2d: instance.is_2d,
                layer: layer_world,
                casts,
            });
        }
        Ok(result)
    }

    pub fn apply_layer_animation(
        &mut self,
        project: &Project,
        textures: &TextureList,
        target: ReferenceTarget,
        animation_name: &[u8],
        frame: f32,
    ) -> Result<Option<RuntimeAnimationTreeApplication>, crate::animation::AnimationError> {
        let layer = project
            .scenes
            .get(target.scene_index)
            .and_then(|scene| scene.layers.get(target.layer_index))
            .ok_or_else(|| {
                crate::animation::AnimationError(format!(
                    "SCN[{}]/LAYR[{}] is outside the project runtime",
                    target.scene_index, target.layer_index
                ))
            })?;
        let runtime_layer = self
            .project_layers
            .get_mut(target.scene_index)
            .and_then(|scene| scene.get_mut(target.layer_index))
            .expect("project runtime layers mirror the parsed project");
        let Some(application) =
            runtime_layer.apply_animation_channels(layer, textures, animation_name, frame)?
        else {
            return Ok(None);
        };
        let mut tree = RuntimeAnimationTreeApplication::default();
        tree.include_layer(application);

        let child_requests = child_animation_requests(
            layer,
            animation_name,
            frame,
            &self.references.plan,
            ReferenceLayerParent::ProjectLayer(target),
        );

        for (child_instance, child_animation_name, child_frame) in child_requests {
            tree.reference_requests += 1;
            if let Some(child) = self.references.apply_instance_animation(
                project,
                textures,
                child_instance,
                &child_animation_name,
                child_frame,
            )? {
                tree.include_tree(child);
            }
        }
        Ok(Some(tree))
    }

    pub fn apply_animation_set(
        &mut self,
        project: &Project,
        textures: &TextureList,
        scene_index: usize,
        animation_set_index: usize,
        frame: f32,
    ) -> Result<RuntimeAnimationTreeApplication, crate::animation::AnimationError> {
        let scene = project.scenes.get(scene_index).ok_or_else(|| {
            crate::animation::AnimationError(format!(
                "SCN[{scene_index}] is outside the project runtime"
            ))
        })?;
        let animation_set = scene
            .animation_sets
            .get(animation_set_index)
            .ok_or_else(|| {
                crate::animation::AnimationError(format!(
                    "SCN[{scene_index}]/ANMS[{animation_set_index}] is outside the scene"
                ))
            })?;
        self.apply_animation_set_definition(project, textures, scene_index, animation_set, frame)
    }

    /// Applies an arbitrary dense ANMS assignment to one scene's runtime state.
    ///
    /// Slot `n` controls layer `n`: it always overwrites that runtime layer's
    /// enabled flag, and a non-empty animation name evaluates that layer's
    /// animation at `frame`. Slots beyond the scene's layers are ignored. On a
    /// freshly constructed runtime, layers beyond the assignment retain base state.
    pub fn apply_animation_set_definition(
        &mut self,
        project: &Project,
        textures: &TextureList,
        scene_index: usize,
        animation_set: &AnimationSetDefinition,
        frame: f32,
    ) -> Result<RuntimeAnimationTreeApplication, crate::animation::AnimationError> {
        let layer_count = self
            .project_layers
            .get(scene_index)
            .map(Vec::len)
            .ok_or_else(|| {
                crate::animation::AnimationError(format!(
                    "SCN[{scene_index}] is outside the project runtime"
                ))
            })?;
        let mut result = RuntimeAnimationTreeApplication::default();

        for (layer_index, slot) in animation_set.slots.iter().take(layer_count).enumerate() {
            self.project_layers[scene_index][layer_index].enabled = slot.is_enabled();
            if slot.animation_name.is_empty() {
                continue;
            }
            if let Some(application) = self.apply_layer_animation(
                project,
                textures,
                ReferenceTarget {
                    scene_index,
                    layer_index,
                },
                &slot.animation_name,
                frame,
            )? {
                result.include_tree(application);
            }
        }
        Ok(result)
    }
}

fn compose_runtime_cast_world_states(
    layer: &Layer,
    transforms: &[SpatialTransform],
    layer_world: RuntimeWorldState,
    is_2d: bool,
    renderer_inverse_camera_view: Affine3x4,
) -> Result<Vec<RuntimeWorldState>, SceneError> {
    if transforms.len() != layer.nodes.len() {
        return Err(SceneError(format!(
            "world composition needs {} transforms, got {}",
            layer.nodes.len(),
            transforms.len()
        )));
    }
    let offsets = layer.compute_parent_csli_offsets()?;
    let hierarchy = layer.build_hierarchy()?;
    let mut result = vec![RuntimeWorldState::default(); layer.nodes.len()];
    for &root in &hierarchy.roots {
        compose_runtime_cast_world_state_node(
            layer,
            transforms,
            &offsets,
            &hierarchy.children,
            root,
            layer_world,
            layer_world,
            is_2d,
            renderer_inverse_camera_view,
            &mut result,
        );
    }
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
fn compose_runtime_cast_world_state_node(
    layer: &Layer,
    transforms: &[SpatialTransform],
    offsets: &[[f32; 2]],
    children: &[Vec<usize>],
    index: usize,
    layer_world: RuntimeWorldState,
    parent: RuntimeWorldState,
    is_2d: bool,
    renderer_inverse_camera_view: Affine3x4,
    output: &mut [RuntimeWorldState],
) {
    let flags = layer.nodes[index].type_flags.unwrap_or(0);
    let local = transforms[index];
    let local_matrix = build_local_matrix(&local, is_2d, false, offsets[index]);
    let matrix = compose_cast_matrix_game(
        parent.matrix,
        local_matrix,
        flags,
        is_2d,
        renderer_inverse_camera_view,
    );
    let visible =
        local.is_visible() && layer_world.visible && (flags & 0x400 == 0 || parent.visible);
    let world = RuntimeWorldState {
        matrix,
        multiply_color: if flags & 0x200 != 0 {
            multiply_color_game(parent.multiply_color, local.multiply_color)
        } else {
            local.multiply_color
        },
        additive_color: if flags & 0x0008_0000 != 0 {
            add_color_saturating_game(parent.additive_color, local.additive_color)
        } else {
            local.additive_color
        },
        visible,
        render_gate: visible && layer_world.render_gate,
    };
    output[index] = world;
    for &child in &children[index] {
        compose_runtime_cast_world_state_node(
            layer,
            transforms,
            offsets,
            children,
            child,
            layer_world,
            world,
            is_2d,
            renderer_inverse_camera_view,
            output,
        );
    }
    if flags & 0x0100_0000 != 0 && flags & 0x0007_0000 == 0x0001_0000 {
        output[index].matrix.rows[2][3] = 0.0;
    }
}

fn compose_cast_matrix_game(
    parent: Affine3x4,
    local: Affine3x4,
    flags: u32,
    is_2d: bool,
    renderer_inverse_camera_view: Affine3x4,
) -> Affine3x4 {
    let mut world = parent.mul_game(local);
    let matrix_kind = flags & 0x0007_0000;
    if matrix_kind == 0 {
        return world;
    }

    let translation = [world.rows[0][3], world.rows[1][3], world.rows[2][3]];
    let inverse_parent = inverse_affine_via_matrix4x4_game(parent);
    if is_2d {
        world = inverse_parent.mul_game(world);
        if flags & 0x0100_0000 != 0 {
            let scale_x = (((parent.rows[0][0] * parent.rows[0][0])
                + (parent.rows[1][0] * parent.rows[1][0]))
                + parent.rows[2][0] * parent.rows[2][0])
                .sqrt();
            let scale_y = (((parent.rows[0][1] * parent.rows[0][1])
                + (parent.rows[1][1] * parent.rows[1][1]))
                + parent.rows[2][1] * parent.rows[2][1])
                .sqrt();
            let scale = Affine3x4 {
                rows: [
                    [scale_x, 0.0, 0.0, 0.0],
                    [0.0, scale_y, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                ],
            };
            world = scale.mul_game(world);
        }
        world = renderer_inverse_camera_view.mul_game(world);
    } else {
        world = match matrix_kind {
            0x0002_0000 => world.mul_game(inverse_parent),
            0x0003_0000 | 0x0004_0000 | 0x0005_0000 => world,
            _ => renderer_inverse_camera_view.mul_game(inverse_parent.mul_game(world)),
        };
    }
    world.rows[0][3] = translation[0];
    world.rows[1][3] = translation[1];
    world.rows[2][3] = translation[2];
    world
}

fn inverse_affine_via_matrix4x4_game(matrix: Affine3x4) -> Affine3x4 {
    let inverse = inverse_matrix4x4_game(&Matrix4x4 {
        rows: [
            matrix.rows[0],
            matrix.rows[1],
            matrix.rows[2],
            [0.0, 0.0, 0.0, 1.0],
        ],
    });
    Affine3x4 {
        rows: [inverse.rows[0], inverse.rows[1], inverse.rows[2]],
    }
}

#[allow(clippy::too_many_arguments)]
fn append_layer_references(
    project: &Project,
    source: ReferenceTarget,
    parent: ReferenceLayerParent,
    is_2d: bool,
    lineage: &[ReferenceTarget],
    plan: &mut ReferenceRuntimePlan,
    lineages: &mut Vec<Vec<ReferenceTarget>>,
) -> Result<(), ReferenceRuntimeError> {
    let layer = &project.scenes[source.scene_index].layers[source.layer_index];
    for (reference_node_index, reference) in layer.reference_by_node.iter().enumerate() {
        let Some(reference) = reference else {
            continue;
        };
        let Some(target) = project.resolve_reference(reference) else {
            plan.unresolved.push(UnresolvedReference {
                parent,
                reference_node_index,
                source_name: reference.source_name.clone(),
                layer_name: reference.layer_name.clone(),
            });
            continue;
        };

        if let Some(repeated_at) = lineage.iter().position(|entry| *entry == target) {
            let mut repeating_layers = lineage[repeated_at..].to_vec();
            repeating_layers.push(target);
            return Err(ReferenceRuntimeError { repeating_layers });
        }

        plan.instances.push(ReferenceLayerInstance {
            parent,
            reference_node_index,
            target,
            is_2d,
            flip_y: project.scenes[target.scene_index].layers[target.layer_index].is_2d() && !is_2d,
        });
        let mut child_lineage = lineage.to_vec();
        child_lineage.push(target);
        lineages.push(child_lineage);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::animation::{AnimationDefinition, Key8, KeyData, Motion, Track};
    use crate::attribute::{CastAttribute, CastAttributeList, CastAttributeValue, ExtParamData};
    use crate::reference::ReferenceDefinition;
    use crate::scene::{
        AnimationSetDefinition, Layer, NodeRecord, RawTransform, Scene, SceneAnimationSlot,
    };
    use crate::transform::SpatialTransform;

    use super::*;

    fn reference(source_name: &[u8], layer_name: &[u8], node_index: i32) -> ReferenceDefinition {
        ReferenceDefinition {
            source_name: source_name.to_vec(),
            layer_name: layer_name.to_vec(),
            animation_enabled: 0,
            animation_name: Vec::new(),
            default_frame: 0.0,
            node_index,
        }
    }

    fn layer(name: &[u8], is_2d: bool, references: Vec<Option<ReferenceDefinition>>) -> Layer {
        let count = references.len();
        Layer {
            name: name.to_vec(),
            flags: u32::from(!is_2d),
            animation_count: 0,
            animations: Vec::new(),
            field_23: Vec::new(),
            nodes: vec![
                NodeRecord {
                    name: None,
                    type_flags: Some(3),
                    parent_csli_cell_index: None,
                    first_child_index: -1,
                    next_sibling_index: -1,
                    field_a0: None,
                };
                count
            ],
            transforms: vec![RawTransform::Trs2(SpatialTransform::default()); count],
            image_by_node: vec![None; count],
            number_by_node: vec![None; count],
            reference_by_node: references,
            csli_by_node: vec![None; count],
            cast_attribute_lists: Vec::new(),
            cast_attribute_list_by_node: vec![None; count],
        }
    }

    fn project(layers: Vec<Layer>) -> Project {
        Project {
            name: Vec::new(),
            declared_scene_count: 1,
            declared_font_count: 0,
            camera: crate::camera::CameraDefinition::default(),
            scenes: vec![Scene {
                name: b"scene".to_vec(),
                declared_layer_count: layers.len() as u32,
                declared_animation_set_count: 0,
                width: 0.0,
                height: 0.0,
                layers,
                animation_sets: Vec::new(),
            }],
            fonts: Vec::new(),
        }
    }

    fn scratch_assignment(slots: Vec<SceneAnimationSlot>) -> AnimationSetDefinition {
        AnimationSetDefinition {
            name: b"scratch".to_vec(),
            start_frame: 0,
            runtime_duration: 10,
            declared_slot_count: slots.len() as i32,
            slots,
        }
    }

    fn animation_layer(base_x: f32) -> Layer {
        let mut result = layer(b"animated", true, vec![None]);
        result.flags |= 0x100;
        result.transforms[0] = RawTransform::Trs2(SpatialTransform {
            translation: [base_x, 0.0, 0.0],
            ..SpatialTransform::default()
        });
        result.animation_count = 1;
        result.animations.push(AnimationDefinition {
            name: b"move".to_vec(),
            flags: 0,
            declared_motion_count: 1,
            duration: 10,
            motions: vec![Motion {
                target: 0,
                tracks: vec![Track {
                    target: 0,
                    key_count: 1,
                    format: 0x10,
                    range_start: 0,
                    range_end: 10,
                    keys: KeyData::Key8F32(vec![Key8 {
                        frame: 0,
                        value: 25.0,
                    }]),
                }],
            }],
        });
        result
    }

    #[test]
    fn scratch_assignment_overwrites_dense_enablement_and_leaves_uncovered_layers_at_base_state() {
        let mut first = layer(b"first", true, vec![None]);
        first.flags |= 0x100;
        let mut second = layer(b"second", true, vec![None]);
        second.flags |= 0x100;
        let mut third = layer(b"third", true, vec![None]);
        third.flags |= 0x100;
        let project = project(vec![first, second, third]);
        let assignment = scratch_assignment(vec![
            SceneAnimationSlot {
                animation_name: Vec::new(),
                enabled: 0,
            },
            SceneAnimationSlot {
                animation_name: Vec::new(),
                enabled: 1,
            },
        ]);
        let textures = TextureList {
            declared_count: 0,
            textures: Vec::new(),
        };
        let mut runtime = ProjectRuntime::new(&project).unwrap();

        runtime
            .apply_animation_set_definition(&project, &textures, 0, &assignment, 0.0)
            .unwrap();

        assert!(project.scenes[0].animation_sets.is_empty());
        assert_eq!(
            runtime.project_layers[0]
                .iter()
                .map(|layer| layer.enabled)
                .collect::<Vec<_>>(),
            vec![false, true, true],
        );
        assert!(
            !runtime
                .compose_world_states(&project, Affine3x4::IDENTITY, Affine3x4::IDENTITY)
                .unwrap()
                .project_layers[0][0]
                .layer
                .render_gate
        );
    }

    #[test]
    fn enabled_empty_assignment_slot_keeps_the_layer_base_pose() {
        let project = project(vec![animation_layer(7.0)]);
        let assignment = scratch_assignment(vec![SceneAnimationSlot {
            animation_name: Vec::new(),
            enabled: 1,
        }]);
        let textures = TextureList {
            declared_count: 0,
            textures: Vec::new(),
        };
        let mut runtime = ProjectRuntime::new(&project).unwrap();

        let application = runtime
            .apply_animation_set_definition(&project, &textures, 0, &assignment, 4.0)
            .unwrap();

        assert_eq!(application, RuntimeAnimationTreeApplication::default());
        assert!(runtime.project_layers[0][0].enabled);
        assert_eq!(
            runtime.project_layers[0][0].cast_transforms[0].translation[0],
            7.0
        );
    }

    #[test]
    fn named_assignment_slot_applies_the_layer_animation() {
        let project = project(vec![animation_layer(7.0)]);
        let assignment = scratch_assignment(vec![SceneAnimationSlot {
            animation_name: b"move".to_vec(),
            enabled: 1,
        }]);
        let textures = TextureList {
            declared_count: 0,
            textures: Vec::new(),
        };
        let mut runtime = ProjectRuntime::new(&project).unwrap();

        let application = runtime
            .apply_animation_set_definition(&project, &textures, 0, &assignment, 4.0)
            .unwrap();

        assert_eq!(application.animated_layers, 1);
        assert!(runtime.project_layers[0][0].enabled);
        assert_eq!(
            runtime.project_layers[0][0].cast_transforms[0].translation[0],
            25.0
        );
    }

    #[test]
    fn matrix_kind_10000_removes_parent_basis_and_keeps_world_translation() {
        let parent = Affine3x4 {
            rows: [
                [2.0, 0.0, 0.0, 10.0],
                [0.0, 3.0, 0.0, 20.0],
                [0.0, 0.0, 1.0, 30.0],
            ],
        };
        let local = Affine3x4 {
            rows: [
                [4.0, 0.0, 0.0, 1.0],
                [0.0, 5.0, 0.0, 2.0],
                [0.0, 0.0, 1.0, 3.0],
            ],
        };
        let renderer_inverse_camera_view = Affine3x4 {
            rows: [
                [7.0, 0.0, 0.0, 100.0],
                [0.0, 11.0, 0.0, 200.0],
                [0.0, 0.0, 1.0, 300.0],
            ],
        };
        assert_eq!(
            compose_cast_matrix_game(
                parent,
                local,
                0x0001_0000,
                true,
                renderer_inverse_camera_view,
            ),
            Affine3x4 {
                rows: [
                    [28.0, 0.0, 0.0, 12.0],
                    [0.0, 55.0, 0.0, 26.0],
                    [0.0, 0.0, 1.0, 33.0],
                ]
            }
        );
    }

    #[test]
    fn matrix_modifier_keeps_parent_axis_lengths_and_zeroes_only_own_final_z() {
        let mut special = layer(b"special", true, vec![None, None]);
        special.nodes[0].type_flags = Some(3 | 0x0001_0000 | 0x0100_0000);
        special.nodes[0].first_child_index = 1;
        special.transforms[0] = RawTransform::Trs2(SpatialTransform {
            translation: [0.0, 0.0, 5.0],
            ..SpatialTransform::default()
        });
        special.transforms[1] = RawTransform::Trs2(SpatialTransform {
            translation: [0.0, 0.0, 2.0],
            ..SpatialTransform::default()
        });
        let project = project(vec![special]);
        let runtime = ProjectRuntime::new(&project).unwrap();
        let worlds = runtime
            .compose_world_states(
                &project,
                Affine3x4 {
                    rows: [
                        [2.0, 0.0, 0.0, 10.0],
                        [0.0, 3.0, 0.0, 20.0],
                        [0.0, 0.0, 1.0, 30.0],
                    ],
                },
                Affine3x4::IDENTITY,
            )
            .unwrap();
        let casts = &worlds.project_layers[0][0].casts;
        assert_eq!(casts[0].matrix.rows[0][0], 2.0);
        assert_eq!(casts[0].matrix.rows[1][1], 3.0);
        assert_eq!(casts[0].matrix.rows[2][3], 0.0);
        assert_eq!(casts[1].matrix.rows[2][3], 37.0);
    }

    #[test]
    fn repeated_targets_create_distinct_instances_and_nested_copies() {
        let project = project(vec![
            layer(
                b"root",
                true,
                vec![
                    Some(reference(b"scene", b"middle", 0)),
                    Some(reference(b"scene", b"middle", 1)),
                ],
            ),
            layer(
                b"middle",
                false,
                vec![Some(reference(b"scene", b"leaf", 0))],
            ),
            layer(b"leaf", true, Vec::new()),
        ]);

        let plan = project.build_reference_runtime_plan().unwrap();
        assert_eq!(plan.unresolved, Vec::new());
        assert_eq!(plan.instances.len(), 5);
        assert_eq!(plan.instances[0].target.layer_index, 1);
        assert_eq!(plan.instances[1].target.layer_index, 1);
        assert_eq!(plan.instances[2].target.layer_index, 2);
        assert_eq!(
            plan.instances[2].parent,
            ReferenceLayerParent::ProjectLayer(ReferenceTarget {
                scene_index: 0,
                layer_index: 1,
            })
        );
        assert_eq!(
            plan.instances[3].parent,
            ReferenceLayerParent::ReferenceInstance(0)
        );
        assert_eq!(
            plan.instances[4].parent,
            ReferenceLayerParent::ReferenceInstance(1)
        );
        assert!(plan.instances[0].is_2d);
        assert!(plan.instances[1].is_2d);
        assert!(!plan.instances[2].is_2d);
        assert!(plan.instances[3].is_2d);
        assert!(plan.instances[4].is_2d);
        assert!(!plan.instances[0].flip_y);
        assert!(!plan.instances[1].flip_y);
        assert!(plan.instances[2].flip_y);
        assert!(!plan.instances[3].flip_y);
        assert!(!plan.instances[4].flip_y);
    }

    #[test]
    fn structural_draw_order_expands_reference_layers_at_the_refcast_position() {
        let project = project(vec![
            layer(
                b"root",
                true,
                vec![None, Some(reference(b"scene", b"middle", 1)), None],
            ),
            layer(
                b"middle",
                true,
                vec![None, Some(reference(b"scene", b"leaf", 1)), None],
            ),
            layer(b"leaf", true, vec![None, None]),
        ]);
        let plan = project.build_reference_runtime_plan().unwrap();
        assert_eq!(
            plan.structural_cast_draw_order(
                &project,
                0,
                crate::render::SRD_RENDERER_INITIAL_LAYER_KEY,
            )
            .unwrap()
            .into_iter()
            .map(|entry| (entry.owner, entry.source.layer_index, entry.node_index))
            .collect::<Vec<_>>(),
            vec![
                (
                    ReferenceLayerParent::ProjectLayer(ReferenceTarget {
                        scene_index: 0,
                        layer_index: 0,
                    }),
                    0,
                    0,
                ),
                (ReferenceLayerParent::ReferenceInstance(0), 1, 0),
                (ReferenceLayerParent::ReferenceInstance(2), 2, 0),
                (ReferenceLayerParent::ReferenceInstance(2), 2, 1),
                (ReferenceLayerParent::ReferenceInstance(0), 1, 2),
                (
                    ReferenceLayerParent::ProjectLayer(ReferenceTarget {
                        scene_index: 0,
                        layer_index: 0,
                    }),
                    0,
                    2,
                ),
                (
                    ReferenceLayerParent::ProjectLayer(ReferenceTarget {
                        scene_index: 0,
                        layer_index: 1,
                    }),
                    1,
                    0,
                ),
                (ReferenceLayerParent::ReferenceInstance(1), 2, 0),
                (ReferenceLayerParent::ReferenceInstance(1), 2, 1),
                (
                    ReferenceLayerParent::ProjectLayer(ReferenceTarget {
                        scene_index: 0,
                        layer_index: 1,
                    }),
                    1,
                    2,
                ),
                (
                    ReferenceLayerParent::ProjectLayer(ReferenceTarget {
                        scene_index: 0,
                        layer_index: 2,
                    }),
                    2,
                    0,
                ),
                (
                    ReferenceLayerParent::ProjectLayer(ReferenceTarget {
                        scene_index: 0,
                        layer_index: 2,
                    }),
                    2,
                    1,
                ),
            ]
        );
    }

    #[test]
    fn cast_layer_keys_inherit_ext_params_and_apply_node_offsets_at_render() {
        let mut tested = layer(b"tested", true, vec![None, None]);
        tested.nodes[0].first_child_index = 1;
        tested.cast_attribute_lists = vec![
            CastAttributeList {
                node_index: Some(0),
                declared_count: 1,
                attributes: vec![CastAttribute {
                    name: b"ExtParamData".to_vec(),
                    source_type_code: 2,
                    value: CastAttributeValue::ExtParam {
                        source: Vec::new(),
                        parsed: ExtParamData {
                            layer: 6,
                            flags: ExtParamData::LAYER_KIND | ExtParamData::ENABLE_LAYER,
                            ..ExtParamData::default()
                        },
                    },
                }],
            },
            CastAttributeList {
                node_index: Some(1),
                declared_count: 1,
                attributes: vec![CastAttribute {
                    name: b"ExtParamData".to_vec(),
                    source_type_code: 2,
                    value: CastAttributeValue::ExtParam {
                        source: Vec::new(),
                        parsed: ExtParamData {
                            layer_level: 0xf0,
                            flags: ExtParamData::LAYER_KIND | ExtParamData::ENABLE_LEVEL,
                            ..ExtParamData::default()
                        },
                    },
                }],
            },
        ];
        tested.cast_attribute_list_by_node = vec![Some(0), Some(1)];
        tested.nodes[0].field_a0 = Some(0x90);
        tested.nodes[1].field_a0 = Some(0x20);
        let project = project(vec![tested]);
        let plan = project.build_reference_runtime_plan().unwrap();
        let entries = plan
            .structural_cast_draw_order(&project, 0, crate::render::SRD_RENDERER_INITIAL_LAYER_KEY)
            .unwrap();
        assert_eq!(entries[0].renderer_layer_key, 0x8710);
        assert_eq!(entries[1].renderer_layer_key, 0x8710);
    }

    #[test]
    fn reference_layer_seed_wraps_only_the_low_byte_before_recursion() {
        let mut root = layer(b"root", true, vec![Some(reference(b"scene", b"child", 0))]);
        root.nodes[0].field_a0 = Some(0x90);
        let mut child = layer(b"child", true, vec![None]);
        child.nodes[0].field_a0 = Some(0x20);
        let project = project(vec![root, child]);
        let plan = project.build_reference_runtime_plan().unwrap();
        let entries = plan
            .structural_cast_draw_order(&project, 0, crate::render::SRD_RENDERER_INITIAL_LAYER_KEY)
            .unwrap();
        let copied = entries
            .iter()
            .find(|entry| entry.owner == ReferenceLayerParent::ReferenceInstance(0))
            .unwrap();
        assert_eq!(copied.renderer_layer_key, 0x8530);
    }

    #[test]
    fn unresolved_references_do_not_create_runtime_layers() {
        let project = project(vec![layer(
            b"root",
            true,
            vec![Some(reference(b"missing", b"layer", 0))],
        )]);
        let plan = project.build_reference_runtime_plan().unwrap();
        assert!(plan.instances.is_empty());
        assert_eq!(plan.unresolved.len(), 1);
    }

    #[test]
    fn cycles_are_reported_without_inventing_a_depth_limit() {
        let project = project(vec![layer(
            b"root",
            true,
            vec![Some(reference(b"scene", b"root", 0))],
        )]);
        let error = project.build_reference_runtime_plan().unwrap_err();
        assert_eq!(
            error.repeating_layers,
            vec![
                ReferenceTarget {
                    scene_index: 0,
                    layer_index: 0,
                },
                ReferenceTarget {
                    scene_index: 0,
                    layer_index: 0,
                }
            ]
        );
    }

    #[test]
    fn copied_layer_world_state_uses_the_owning_refcast_as_parent() {
        let instance = ReferenceLayerInstance {
            parent: ReferenceLayerParent::ProjectLayer(ReferenceTarget {
                scene_index: 0,
                layer_index: 0,
            }),
            reference_node_index: 0,
            target: ReferenceTarget {
                scene_index: 0,
                layer_index: 1,
            },
            is_2d: true,
            flip_y: false,
        };
        let parent = RuntimeWorldState {
            matrix: Affine3x4 {
                rows: [
                    [1.0, 0.0, 0.0, 10.0],
                    [0.0, 1.0, 0.0, 20.0],
                    [0.0, 0.0, 1.0, 30.0],
                ],
            },
            multiply_color: [200, 100, 50, 255],
            additive_color: [250, 20, 30, 40],
            visible: true,
            render_gate: true,
        };
        let mut local = ReferenceLayerLocalState::default();
        local.transform.translation = [2.0, 3.0, 4.0];
        local.transform.multiply_color = [128, 255, 0, 200];
        local.transform.additive_color = [10, 240, 20, 220];

        let world = instance.compose_world_state(parent, local);
        assert_eq!(world.matrix.rows[0][3], 12.0);
        assert_eq!(world.matrix.rows[1][3], 23.0);
        assert_eq!(world.matrix.rows[2][3], 34.0);
        assert_eq!(world.multiply_color, [100, 100, 0, 200]);
        assert_eq!(world.additive_color, [255, 255, 50, 255]);
        assert!(world.visible);
        assert!(world.render_gate);

        local.enabled = false;
        let disabled = instance.compose_world_state(parent, local);
        assert!(!disabled.visible);
        assert!(!disabled.render_gate);

        local.enabled = true;
        let gated_parent = RuntimeWorldState {
            render_gate: false,
            ..parent
        };
        let gated = instance.compose_world_state(gated_parent, local);
        assert!(gated.visible);
        assert!(!gated.render_gate);
    }

    #[test]
    fn project_runtime_world_states_follow_refcast_parent_mode_color_and_gate() {
        let mut root = layer(b"root", true, vec![Some(reference(b"scene", b"target", 0))]);
        root.flags |= 0x100;
        root.transforms[0] = RawTransform::Trs2(SpatialTransform {
            translation: [10.0, 20.0, 0.0],
            multiply_color: [200, 100, 50, 255],
            additive_color: [10, 20, 30, 40],
            ..SpatialTransform::default()
        });

        // The parsed target is 3D, but binding propagates the owning 2D
        // RefCast mode to every CAST in this independent copy.
        let mut target = layer(b"target", false, vec![None]);
        target.flags |= 0x100;
        target.nodes[0].type_flags = Some(3 | 0x200 | 0x0008_0000);
        target.transforms[0] = RawTransform::Trs3(SpatialTransform {
            translation: [4.0, 5.0, 0.0],
            rotation: [0, 0, 0x4000],
            multiply_color: [128, 255, 128, 200],
            additive_color: [1, 2, 3, 4],
            ..SpatialTransform::default()
        });

        let project = project(vec![root, target]);
        let mut runtime = ProjectRuntime::new(&project).unwrap();
        runtime.references.layers[0].local.transform.translation = [2.0, 3.0, 0.0];
        runtime.references.layers[0].local.transform.multiply_color = [128, 255, 0, 128];
        runtime.references.layers[0].local.transform.additive_color = [250, 240, 230, 220];

        let worlds = runtime
            .compose_world_states(&project, Affine3x4::IDENTITY, Affine3x4::IDENTITY)
            .unwrap();
        let copied = &worlds.references[0];
        assert!(copied.is_2d);
        assert_eq!(copied.layer.matrix.rows[0][3], 12.0);
        assert_eq!(copied.layer.matrix.rows[1][3], 23.0);
        assert_eq!(copied.layer.multiply_color, [100, 100, 0, 128]);
        assert_eq!(copied.layer.additive_color, [255, 255, 255, 255]);
        assert!(copied.layer.visible);
        assert!(copied.layer.render_gate);

        let expected_local = build_local_matrix(
            &runtime.references.layers[0].cast_transforms[0],
            true,
            false,
            [0.0, 0.0],
        );
        assert_eq!(
            copied.casts[0].matrix,
            copied.layer.matrix.mul_game(expected_local)
        );
        assert_eq!(copied.casts[0].multiply_color, [50, 100, 0, 100]);
        assert_eq!(copied.casts[0].additive_color, [255, 255, 255, 255]);
        assert!(copied.casts[0].visible);
        assert!(copied.casts[0].render_gate);

        runtime.project_layers[0][0].enabled = false;
        let gated = runtime
            .compose_world_states(&project, Affine3x4::IDENTITY, Affine3x4::IDENTITY)
            .unwrap();
        assert!(gated.references[0].layer.visible);
        assert!(!gated.references[0].layer.render_gate);
        assert!(gated.references[0].casts[0].visible);
        assert!(!gated.references[0].casts[0].render_gate);
    }

    #[test]
    fn copied_layer_flip_y_applies_once_at_the_layer_local_matrix() {
        let mut root = layer(
            b"root",
            false,
            vec![Some(reference(b"scene", b"target", 0))],
        );
        root.flags |= 0x100;
        let mut target = layer(b"target", true, vec![None]);
        target.flags |= 0x100;
        target.transforms[0] = RawTransform::Trs2(SpatialTransform {
            translation: [0.0, 5.0, 0.0],
            ..SpatialTransform::default()
        });
        let project = project(vec![root, target]);
        let mut runtime = ProjectRuntime::new(&project).unwrap();
        assert!(runtime.references.plan.instances[0].flip_y);
        assert!(!runtime.references.plan.instances[0].is_2d);
        runtime.references.layers[0].local.transform.translation[1] = 3.0;

        let copied = runtime
            .compose_world_states(&project, Affine3x4::IDENTITY, Affine3x4::IDENTITY)
            .unwrap()
            .references
            .remove(0);
        assert_eq!(copied.layer.matrix.rows[1][3], -3.0);
        // CAST local Y is not flipped a second time; the propagated runtime
        // mode is 3D and the copied-layer root already contains the flip.
        assert_eq!(copied.casts[0].matrix.rows[1][3], 2.0);
    }

    #[test]
    fn copied_layers_keep_independent_animation_frames_and_cast_transforms() {
        let mut project = project(vec![
            layer(
                b"root",
                true,
                vec![
                    Some(reference(b"scene", b"target", 0)),
                    Some(reference(b"scene", b"target", 1)),
                ],
            ),
            layer(b"target", true, vec![None]),
        ]);
        let animation = AnimationDefinition {
            name: b"move".to_vec(),
            flags: 0,
            declared_motion_count: 1,
            duration: 10,
            motions: vec![Motion {
                target: 0,
                tracks: vec![Track {
                    target: 0,
                    key_count: 1,
                    format: 0x10,
                    range_start: 0,
                    range_end: 10,
                    keys: KeyData::Key8F32(vec![Key8 {
                        frame: 0,
                        value: 25.0,
                    }]),
                }],
            }],
        };
        project.scenes[0].layers[1].animation_count = 1;
        project.scenes[0].layers[1].animations.push(animation);

        let plan = project.build_reference_runtime_plan().unwrap();
        let mut first = ReferenceLayerRuntimeState::new(&project, &plan, 0).unwrap();
        let second = ReferenceLayerRuntimeState::new(&project, &plan, 1).unwrap();
        assert_eq!(
            first
                .apply_animation_common_channels(&project, &plan, b"move", 7.0)
                .unwrap(),
            Some(1)
        );
        assert_eq!(first.animations[0].frame, 7.0);
        assert_eq!(first.animations[0].duration, 10.0);
        assert_eq!(first.animations[0].flags, 9);
        assert_eq!(first.cast_transforms[0].translation[0], 25.0);
        assert_eq!(second.animations[0].frame, 0.0);
        assert_eq!(second.cast_transforms[0].translation[0], 0.0);
        assert_eq!(
            first
                .apply_animation_common_channels(&project, &plan, b"missing", 4.0)
                .unwrap(),
            None
        );
    }

    #[test]
    fn channel_23_recursively_applies_the_named_child_instance_animation() {
        let mut middle_reference = reference(b"scene", b"leaf", 0);
        middle_reference.animation_enabled = 1;
        middle_reference.animation_name = b"leaf_animation".to_vec();
        middle_reference.default_frame = 8.0;
        let mut project = project(vec![
            layer(b"root", true, vec![Some(reference(b"scene", b"middle", 0))]),
            layer(b"middle", true, vec![Some(middle_reference)]),
            layer(b"leaf", true, vec![None]),
        ]);
        project.scenes[0].layers[1].animation_count = 1;
        project.scenes[0].layers[1]
            .animations
            .push(AnimationDefinition {
                name: b"drive_reference".to_vec(),
                flags: 0,
                declared_motion_count: 1,
                duration: 10,
                motions: vec![Motion {
                    target: 0,
                    tracks: vec![Track {
                        target: 23,
                        key_count: 1,
                        format: 0x13,
                        range_start: 0,
                        range_end: 10,
                        keys: KeyData::Key20F32(vec![crate::animation::Key20 {
                            frame: 0,
                            value: 4.0,
                            mode: 0,
                            slope_in: 0.0,
                            slope_out: 0.0,
                        }]),
                    }],
                }],
            });
        project.scenes[0].layers[2].animation_count = 1;
        project.scenes[0].layers[2]
            .animations
            .push(AnimationDefinition {
                name: b"leaf_animation".to_vec(),
                flags: 0,
                declared_motion_count: 1,
                duration: 10,
                motions: vec![Motion {
                    target: 0,
                    tracks: vec![Track {
                        target: 0,
                        key_count: 1,
                        format: 0x10,
                        range_start: 0,
                        range_end: 10,
                        keys: KeyData::Key8F32(vec![Key8 {
                            frame: 0,
                            value: 99.0,
                        }]),
                    }],
                }],
            });

        let mut runtime = ReferenceRuntime::new(&project).unwrap();
        let nested_leaf = runtime
            .plan
            .instances
            .iter()
            .position(|instance| {
                instance.parent == ReferenceLayerParent::ReferenceInstance(0)
                    && instance.target.layer_index == 2
            })
            .unwrap();
        let top_level_leaf = runtime
            .plan
            .instances
            .iter()
            .position(|instance| {
                instance.parent
                    == ReferenceLayerParent::ProjectLayer(ReferenceTarget {
                        scene_index: 0,
                        layer_index: 1,
                    })
                    && instance.target.layer_index == 2
            })
            .unwrap();
        let textures = TextureList {
            declared_count: 0,
            textures: Vec::new(),
        };
        let application = runtime
            .apply_instance_animation(&project, &textures, 0, b"drive_reference", 3.0)
            .unwrap()
            .unwrap();
        assert_eq!(application.animated_layers, 2);
        assert_eq!(application.reference_requests, 1);
        assert_eq!(runtime.layers[0].animations[0].frame, 3.0);
        assert_eq!(runtime.layers[nested_leaf].animations[0].frame, 4.0);
        assert_eq!(
            runtime.layers[nested_leaf].cast_transforms[0].translation[0],
            99.0
        );
        assert_eq!(
            runtime.layers[top_level_leaf].cast_transforms[0].translation[0],
            0.0
        );
    }
    #[test]
    fn root_project_layer_follows_nested_refcast_ancestry() {
        let root = ReferenceTarget {
            scene_index: 0,
            layer_index: 4,
        };
        let plan = ReferenceRuntimePlan {
            instances: vec![
                ReferenceLayerInstance {
                    parent: ReferenceLayerParent::ProjectLayer(root),
                    reference_node_index: 2,
                    target: ReferenceTarget {
                        scene_index: 1,
                        layer_index: 3,
                    },
                    is_2d: true,
                    flip_y: false,
                },
                ReferenceLayerInstance {
                    parent: ReferenceLayerParent::ReferenceInstance(0),
                    reference_node_index: 5,
                    target: ReferenceTarget {
                        scene_index: 2,
                        layer_index: 1,
                    },
                    is_2d: true,
                    flip_y: false,
                },
            ],
            unresolved: Vec::new(),
        };

        assert_eq!(
            plan.root_project_layer(ReferenceLayerParent::ReferenceInstance(1)),
            Ok(root)
        );
    }

    #[test]
    fn root_project_layer_rejects_invalid_or_cyclic_instances() {
        let invalid = ReferenceRuntimePlan::default();
        assert_eq!(
            invalid.root_project_layer(ReferenceLayerParent::ReferenceInstance(0)),
            Err(ReferenceRuntimeAncestryError::InvalidReferenceInstance { instance_index: 0 })
        );

        let cyclic = ReferenceRuntimePlan {
            instances: vec![ReferenceLayerInstance {
                parent: ReferenceLayerParent::ReferenceInstance(0),
                reference_node_index: 0,
                target: ReferenceTarget {
                    scene_index: 0,
                    layer_index: 0,
                },
                is_2d: true,
                flip_y: false,
            }],
            unresolved: Vec::new(),
        };
        assert_eq!(
            cyclic.root_project_layer(ReferenceLayerParent::ReferenceInstance(0)),
            Err(ReferenceRuntimeAncestryError::ReferenceCycle { instance_index: 0 })
        );
    }
}
