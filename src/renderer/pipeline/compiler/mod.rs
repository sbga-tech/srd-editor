use super::*;
mod fennel;
mod image;
mod number;
mod slice;
mod world;

use fennel::build_fennel_draw_for_runtime_cast;
#[cfg(test)]
pub(super) use fennel::fennel_textbox_transform;
pub use fennel::{
    FennelRuntimeTextCastKey, FennelSrdRuntimeTextInput, build_base_pose_fennel_draws,
    build_base_pose_reference_fennel_draws,
};
use image::{build_image_draw_for_runtime_cast, build_image_draws};
use number::build_number_glyph_draws_for_runtime_cast;
use slice::build_slice_cell_draws_for_runtime_cast;
use world::*;
/// CPU-side RFZ/Fennel resources required to compile TextCast draws.
#[derive(Clone, Copy)]
pub struct FennelRenderResources<'a> {
    pub font_slots: &'a FennelFontSlotRegistry<Vec<u8>>,
    pub fonts: &'a BTreeMap<Vec<u8>, RuhunaRuntimeFont>,
    pub force_color_update: bool,
    pub text_inputs: &'a BTreeMap<FennelRuntimeTextCastKey, FennelSrdRuntimeTextInput>,
}
#[derive(Clone, Copy)]
struct FennelCastResources<'a> {
    font_slots: &'a FennelFontSlotRegistry<Vec<u8>>,
    fonts: &'a BTreeMap<Vec<u8>, RuhunaRuntimeFont>,
    force_color_update: bool,
    text_input: Option<&'a FennelSrdRuntimeTextInput>,
}

#[derive(Clone, Copy)]
struct RuntimeCastSite<'a> {
    owner: ReferenceLayerParent,
    source: ReferenceTarget,
    layer: &'a Layer,
    node_index: usize,
    renderer_layer_key: u32,
}

#[derive(Clone, Copy)]
struct RuntimeCastState {
    is_2d: bool,
    world: RuntimeWorldState,
    image: crate::image::RuntimeImageState,
}

#[derive(Clone, Copy)]
struct FrameGeometry<'a> {
    snapshot: WorldSnapshot,
    camera_bridge: Matrix4x4,
    project_screen: Option<&'a (Matrix4x4, [i32; 2])>,
}

/// Compiles the proven ImageCast packet fields directly into native geometry,
/// material, pipeline, queue, and transform state. Unsupported render presets
/// and unresolved host inputs are rejected rather than mapped to a nearby
/// shader variant.
pub fn build_base_pose_image_draws(
    project: &Project,
    textures: &TextureList,
    scene_index: usize,
    snapshot: WorldSnapshot,
) -> Result<Vec<SrdDraw>, SrdDrawError> {
    build_image_draws(project, textures, scene_index, snapshot, None)
}
pub fn build_animation_set_image_draws(
    project: &Project,
    textures: &TextureList,
    scene_index: usize,
    animation_set_index: usize,
    frame: f32,
    snapshot: WorldSnapshot,
) -> Result<Vec<SrdDraw>, SrdDrawError> {
    let mut runtime = ProjectRuntime::new(project)
        .map_err(|error| SrdDrawError(format!("failed to construct SRD runtime: {error}")))?;
    runtime
        .apply_animation_set(project, textures, scene_index, animation_set_index, frame)
        .map_err(|error| SrdDrawError(error.to_string()))?;
    let runtime_layers = runtime
        .project_layers
        .get(scene_index)
        .ok_or_else(|| SrdDrawError(format!("scene index {scene_index} is outside the runtime")))?;
    build_image_draws(
        project,
        textures,
        scene_index,
        snapshot,
        Some(runtime_layers),
    )
}
/// Builds ImageCast draws for original and independently copied reference
/// layers in the exact recursive CAST traversal order. The caller still must
/// merge TextCast draws before using this as a complete Composition stream.
pub fn build_base_pose_reference_image_draws(
    project: &Project,
    textures: &TextureList,
    scene_index: usize,
    snapshot: WorldSnapshot,
) -> Result<Vec<SrdDraw>, SrdDrawError> {
    validate_world_snapshot(snapshot)?;
    let runtime = ProjectRuntime::new(project)
        .map_err(|error| SrdDrawError(format!("failed to construct SRD runtime: {error}")))?;
    build_reference_image_draws_from_runtime(project, textures, scene_index, snapshot, &runtime)
}
pub fn build_animation_set_reference_image_draws(
    project: &Project,
    textures: &TextureList,
    scene_index: usize,
    animation_set_index: usize,
    frame: f32,
    snapshot: WorldSnapshot,
) -> Result<Vec<SrdDraw>, SrdDrawError> {
    validate_world_snapshot(snapshot)?;
    let mut runtime = ProjectRuntime::new(project)
        .map_err(|error| SrdDrawError(format!("failed to construct SRD runtime: {error}")))?;
    runtime
        .apply_animation_set(project, textures, scene_index, animation_set_index, frame)
        .map_err(|error| SrdDrawError(error.to_string()))?;
    build_reference_image_draws_from_runtime(project, textures, scene_index, snapshot, &runtime)
}
fn build_reference_image_draws_from_runtime(
    project: &Project,
    textures: &TextureList,
    scene_index: usize,
    snapshot: WorldSnapshot,
    runtime: &ProjectRuntime,
) -> Result<Vec<SrdDraw>, SrdDrawError> {
    let renderer_inverse_camera_view = inverse_camera_view(project, snapshot);
    let worlds = runtime
        .compose_world_states(
            project,
            snapshot.first_calc_matrix,
            renderer_inverse_camera_view,
        )
        .map_err(|error| SrdDrawError(error.to_string()))?;
    if project.scenes.get(scene_index).is_none() {
        return Err(SrdDrawError(format!(
            "scene index {scene_index} is outside the project"
        )));
    }

    let camera_bridge = project_camera_bridge(project, snapshot);
    let project_screen = project_screen_matrix(snapshot, camera_bridge)?;
    let geometry = FrameGeometry {
        snapshot,
        camera_bridge,
        project_screen: project_screen.as_ref(),
    };
    let mut draws = Vec::new();

    for entry in runtime
        .references
        .plan
        .structural_cast_draw_order(project, scene_index, snapshot.renderer_layer_key)
        .map_err(|error| SrdDrawError(error.to_string()))?
    {
        let layer = &project.scenes[entry.source.scene_index].layers[entry.source.layer_index];
        let layer_worlds = worlds.layer(entry.owner).ok_or_else(|| {
            SrdDrawError(format!(
                "runtime draw owner {:?} has no composed world state",
                entry.owner
            ))
        })?;
        let cast_world = *layer_worlds.casts.get(entry.node_index).ok_or_else(|| {
            SrdDrawError(format!(
                "SCN[{}]/LAYR[{}]/NODE[{}] has no composed CAST world state",
                entry.source.scene_index, entry.source.layer_index, entry.node_index
            ))
        })?;
        let image_state = runtime_image_state_for_owner(runtime, entry.owner, entry.node_index)?;
        let site = RuntimeCastSite {
            owner: entry.owner,
            source: entry.source,
            layer,
            node_index: entry.node_index,
            renderer_layer_key: entry.renderer_layer_key,
        };
        let state = RuntimeCastState {
            is_2d: layer_worlds.is_2d,
            world: cast_world,
            image: image_state,
        };
        if let Some(draw) = build_image_draw_for_runtime_cast(textures, site, state, geometry)? {
            draws.push(draw);
        }
    }
    Ok(draws)
}
pub fn build_base_pose_runtime_cast_draws(
    project: &Project,
    textures: &TextureList,
    scene_index: usize,
    snapshot: WorldSnapshot,
    fennel: FennelRenderResources<'_>,
) -> Result<Vec<RuntimeCastDraw>, SrdDrawError> {
    validate_world_snapshot(snapshot)?;
    let runtime = ProjectRuntime::new(project)
        .map_err(|error| SrdDrawError(format!("failed to construct SRD runtime: {error}")))?;
    build_runtime_cast_draws_from_runtime(
        project,
        textures,
        scene_index,
        snapshot,
        &runtime,
        Some(fennel),
    )
}
/// Builds a complete runtime CAST stream from an arbitrary dense ANMS
/// assignment. The assignment need not be stored in the scene and can be an
/// editor-owned scratch clone.
pub fn build_animation_assignment_runtime_cast_draws(
    project: &Project,
    textures: &TextureList,
    scene_index: usize,
    animation_assignment: &AnimationSetDefinition,
    frame: f32,
    snapshot: WorldSnapshot,
    fennel: FennelRenderResources<'_>,
) -> Result<Vec<RuntimeCastDraw>, SrdDrawError> {
    validate_world_snapshot(snapshot)?;
    let mut runtime = ProjectRuntime::new(project)
        .map_err(|error| SrdDrawError(format!("failed to construct SRD runtime: {error}")))?;
    runtime
        .apply_animation_set_definition(project, textures, scene_index, animation_assignment, frame)
        .map_err(|error| SrdDrawError(format!("failed to apply animation assignment: {error}")))?;
    build_runtime_cast_draws_from_runtime(
        project,
        textures,
        scene_index,
        snapshot,
        &runtime,
        Some(fennel),
    )
}
pub fn build_animation_set_runtime_cast_draws(
    project: &Project,
    textures: &TextureList,
    scene_index: usize,
    animation_set_index: usize,
    frame: f32,
    snapshot: WorldSnapshot,
    fennel: FennelRenderResources<'_>,
) -> Result<Vec<RuntimeCastDraw>, SrdDrawError> {
    validate_world_snapshot(snapshot)?;
    let mut runtime = ProjectRuntime::new(project)
        .map_err(|error| SrdDrawError(format!("failed to construct SRD runtime: {error}")))?;
    runtime
        .apply_animation_set(project, textures, scene_index, animation_set_index, frame)
        .map_err(|error| SrdDrawError(format!("failed to apply animation set: {error}")))?;
    build_runtime_cast_draws_from_runtime(
        project,
        textures,
        scene_index,
        snapshot,
        &runtime,
        Some(fennel),
    )
}
/// Builds the non-text SRD runtime stream without requiring RFZ/Fennel host
/// resources. This is useful for corpus-wide render-state audits: ImageCast,
/// SliceCast and NumberCast still follow the same recursive CAST order and
/// runtime animation state as the complete stream, while SrTextCast is omitted
/// explicitly instead of being confused with an unsupported image draw.
pub fn build_animation_set_runtime_srd_draws(
    project: &Project,
    textures: &TextureList,
    scene_index: usize,
    animation_set_index: usize,
    frame: f32,
    snapshot: WorldSnapshot,
) -> Result<Vec<RuntimeCastDraw>, SrdDrawError> {
    validate_world_snapshot(snapshot)?;
    let mut runtime = ProjectRuntime::new(project)
        .map_err(|error| SrdDrawError(format!("failed to construct SRD runtime: {error}")))?;
    runtime
        .apply_animation_set(project, textures, scene_index, animation_set_index, frame)
        .map_err(|error| SrdDrawError(format!("failed to apply animation set: {error}")))?;
    build_runtime_srd_draws_from_runtime(project, textures, scene_index, snapshot, &runtime)
}
/// State-reuse variant of [`build_animation_set_runtime_srd_draws`].
/// The supplied runtime must already contain the desired animation-set frame.
/// Keeping construction separate lets exhaustive corpus tools clone one proven
/// reference plan per file instead of reparsing it for every integer frame.
pub fn build_runtime_srd_draws_from_runtime(
    project: &Project,
    textures: &TextureList,
    scene_index: usize,
    snapshot: WorldSnapshot,
    runtime: &ProjectRuntime,
) -> Result<Vec<RuntimeCastDraw>, SrdDrawError> {
    validate_world_snapshot(snapshot)?;
    build_runtime_cast_draws_from_runtime(project, textures, scene_index, snapshot, runtime, None)
}
fn build_runtime_cast_draws_from_runtime(
    project: &Project,
    textures: &TextureList,
    scene_index: usize,
    snapshot: WorldSnapshot,
    runtime: &ProjectRuntime,
    fennel: Option<FennelRenderResources<'_>>,
) -> Result<Vec<RuntimeCastDraw>, SrdDrawError> {
    let renderer_inverse_camera_view = inverse_camera_view(project, snapshot);
    let worlds = runtime
        .compose_world_states(
            project,
            snapshot.first_calc_matrix,
            renderer_inverse_camera_view,
        )
        .map_err(|error| SrdDrawError(error.to_string()))?;
    if project.scenes.get(scene_index).is_none() {
        return Err(SrdDrawError(format!(
            "scene index {scene_index} is outside the project"
        )));
    }
    let camera_bridge = project_camera_bridge(project, snapshot);
    let project_screen = project_screen_matrix(snapshot, camera_bridge)?;
    let geometry = FrameGeometry {
        snapshot,
        camera_bridge,
        project_screen: project_screen.as_ref(),
    };
    let mut draws = Vec::new();

    for entry in runtime
        .references
        .plan
        .structural_cast_draw_order(project, scene_index, snapshot.renderer_layer_key)
        .map_err(|error| SrdDrawError(error.to_string()))?
    {
        let layer = &project.scenes[entry.source.scene_index].layers[entry.source.layer_index];
        let layer_worlds = worlds.layer(entry.owner).ok_or_else(|| {
            SrdDrawError(format!(
                "runtime draw owner {:?} has no composed world state",
                entry.owner
            ))
        })?;
        let cast_world = *layer_worlds.casts.get(entry.node_index).ok_or_else(|| {
            SrdDrawError(format!(
                "SCN[{}]/LAYR[{}]/NODE[{}] has no composed CAST world state",
                entry.source.scene_index, entry.source.layer_index, entry.node_index
            ))
        })?;
        let image_state = runtime_image_state_for_owner(runtime, entry.owner, entry.node_index)?;
        let site = RuntimeCastSite {
            owner: entry.owner,
            source: entry.source,
            layer,
            node_index: entry.node_index,
            renderer_layer_key: entry.renderer_layer_key,
        };
        let state = RuntimeCastState {
            is_2d: layer_worlds.is_2d,
            world: cast_world,
            image: image_state,
        };
        if let Some(draw) = build_image_draw_for_runtime_cast(textures, site, state, geometry)? {
            draws.push(RuntimeCastDraw::Image(draw));
            continue;
        }
        if layer.csli_by_node[entry.node_index].is_some() {
            let cells = build_slice_cell_draws_for_runtime_cast(textures, site, state, geometry)?;
            draws.extend(cells.into_iter().map(RuntimeCastDraw::SliceCell));
            continue;
        }
        if layer.number_by_node[entry.node_index].is_some() {
            let glyphs =
                build_number_glyph_draws_for_runtime_cast(textures, site, state, geometry)?;
            draws.extend(glyphs.into_iter().map(RuntimeCastDraw::NumberGlyph));
            continue;
        }
        let Some(fennel) = fennel else {
            continue;
        };
        let resources = FennelCastResources {
            font_slots: fennel.font_slots,
            fonts: fennel.fonts,
            force_color_update: fennel.force_color_update,
            text_input: fennel.text_inputs.get(&FennelRuntimeTextCastKey {
                owner: entry.owner,
                node_index: entry.node_index,
            }),
        };
        if let Some(draw) =
            build_fennel_draw_for_runtime_cast(project, site, state, geometry, resources)?
        {
            draws.push(RuntimeCastDraw::Fennel(draw));
        }
    }
    Ok(draws)
}
