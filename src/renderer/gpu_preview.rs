use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::camera::CameraDefinition;
use crate::document::EditorDocument;
use crate::fennel::FennelFontSlotRegistry;
use crate::game_host::{
    CHUSAN_ADVERTISE_LOGO_PLAYER, CHUSAN_BG_SCENE, CHUSAN_COMMON_BACKGROUND_PLAYER,
    CHUSAN_LINKED_VERSE_GATE_PLAYER, CHUSAN_MAIN_SCENE, ChusanAirSceneTargetProfile,
};
use crate::projection::{mul_matrix4x4_game, project_point_to_screen_game, viewport_matrix_game};
use crate::reference_runtime::{ReferenceLayerParent, ReferenceRuntimePlan};
use crate::renderer::assets::{FennelAtlasSourceSet, SrdTextureSourceSet};
use crate::renderer::backend::{
    FennelAtlasHandle, FennelAtlasRoute, RuntimeRenderLayer, RuntimeSrdSourceFilter,
    SrdExternalRenderState, SrdRenderBackend, SrdTextureSetHandle, render_preview_runtime_layers,
    render_to_composition,
};
use crate::ruhuna::{RuhunaFont, RuhunaRuntimeFont};
use crate::srd_draw::{
    EvidenceCompleteRuntimeCastDraw, EvidenceCompleteSrdDraw, EvidenceMergedRuntimeTargetCommand,
    EvidenceRuntimeTargetCommandSource, FennelFontResourceAssignment, SrdHostDrawContext,
    assign_fennel_font_resource_requests,
    build_evidence_complete_animation_assignment_runtime_cast_draws,
    build_evidence_complete_initial_runtime_cast_draws,
    build_evidence_filtered_merged_runtime_target_submission,
    collect_fennel_font_resource_requests,
};
use crate::target_pass::EvidenceSrdType1TargetFilter;

use super::{
    PreviewCastSelection, PreviewFrame, PreviewHighlightBounds, PreviewHighlightRequest,
    PreviewLayerRequest, PreviewProfile, PreviewRequest,
};

const CLEAR_ARGB: u32 = 0xFF06_0B17;
const GAME_PRESENT_SIZE: [u32; 2] = [1080, 1920];
const GAME_SCREEN_SIZE: [u32; 2] = [1920, 1080];

pub(super) trait ManagedSrdRenderBackend: SrdRenderBackend {
    fn release_srd_textures(&mut self, handle: SrdTextureSetHandle);
    fn release_fennel_atlas(&mut self, handle: FennelAtlasHandle);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GpuRenderPhase {
    Begin,
    End,
}

pub(super) struct GpuPreviewState<B> {
    backend: B,
    foreground_resources: PreviewDocumentResources,
    background_resources: PreviewDocumentResources,
}

#[derive(Default)]
struct PreviewDocumentResources {
    asset_path: Option<PathBuf>,
    texture_filenames: Vec<Vec<u8>>,
    fennel_assignments: Vec<FennelFontResourceAssignment>,
    texture_indices: BTreeSet<usize>,
    textures: Option<SrdTextureSetHandle>,
    fennel_resources_loaded: bool,
    runtime_fonts: BTreeMap<Vec<u8>, RuhunaRuntimeFont>,
    fennel_atlas_routes: BTreeMap<u32, FennelAtlasRoute>,
}
struct PreparedRuntimePreviewLayer {
    draws: Vec<EvidenceCompleteRuntimeCastDraw>,
    submission: Vec<EvidenceMergedRuntimeTargetCommand>,
    reference_plan: ReferenceRuntimePlan,
}

impl<B: ManagedSrdRenderBackend> GpuPreviewState<B> {
    pub(super) fn new(backend: B) -> Self {
        Self {
            backend,
            foreground_resources: PreviewDocumentResources::default(),
            background_resources: PreviewDocumentResources::default(),
        }
    }

    #[cfg(any(
        target_os = "windows",
        all(target_os = "linux", feature = "dxvk-native")
    ))]
    pub(super) fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    pub(super) fn render(
        &mut self,
        request: PreviewRequest<'_>,
        mut phase: impl FnMut(GpuRenderPhase) -> Result<(), String>,
    ) -> Result<PreviewFrame, String> {
        let background = request
            .background
            .as_ref()
            .map(|background| (background.document, background));
        let foreground_document = request.foreground.document;
        let foreground_request = &request.foreground;
        let composition_size =
            preview_layer_composition_size(foreground_document, foreground_request)?;
        if let Some((background_document, background_request)) = background {
            let background_size =
                preview_layer_composition_size(background_document, background_request)?;
            if background_size != composition_size {
                return Err(format!(
                    "Common background composition size {}x{} does not match foreground {}x{}; the game binary does not provide an implicit editor scaling rule",
                    background_size[0],
                    background_size[1],
                    composition_size[0],
                    composition_size[1],
                ));
            }
        }

        let (foreground_host, foreground_filter, target) =
            preview_host_for_profile(request.profile)?;
        let foreground_host = preview_host_with_project_camera(
            foreground_host,
            foreground_document.project.camera,
            composition_size,
        );
        let foreground = prepare_runtime_preview_layer(
            &mut self.backend,
            &mut self.foreground_resources,
            foreground_document,
            foreground_request,
            foreground_host,
            target,
            foreground_filter,
        )?;
        let background = background
            .map(|(document, request)| {
                let host = CHUSAN_COMMON_BACKGROUND_PLAYER
                    .host_context_for_target(
                        target,
                        GAME_PRESENT_SIZE[0],
                        GAME_PRESENT_SIZE[1],
                        GAME_SCREEN_SIZE,
                    )
                    .map_err(|error| error.to_string())?;
                let host = preview_host_with_project_camera(
                    host,
                    document.project.camera,
                    composition_size,
                );
                prepare_runtime_preview_layer(
                    &mut self.backend,
                    &mut self.background_resources,
                    document,
                    request,
                    host,
                    target,
                    CHUSAN_COMMON_BACKGROUND_PLAYER.initial_srd_target_filter(target),
                )
            })
            .transpose()?;
        let (selection_bounds, highlight_bounds) = runtime_cast_bounds(
            &foreground_document.project,
            &foreground.reference_plan,
            &foreground.draws,
            &foreground.submission,
            request.selection,
            request.highlights,
            composition_size,
        );
        if self.backend.composition_size() != Some(composition_size) {
            self.backend
                .configure_composition_target(composition_size)
                .map_err(|error| format!("failed to create GPU composition target: {error}"))?;
        }

        let foreground_layer = RuntimeRenderLayer {
            draws: &foreground.draws,
            submission: &foreground.submission,
            textures: self.foreground_resources.textures,
            fennel_atlas_routes: &self.foreground_resources.fennel_atlas_routes,
        };
        let background_layer = background.as_ref().map(|background| RuntimeRenderLayer {
            draws: &background.draws,
            submission: &background.submission,
            textures: self.background_resources.textures,
            fennel_atlas_routes: &self.background_resources.fennel_atlas_routes,
        });

        phase(GpuRenderPhase::Begin)?;
        let render_result = render_to_composition(&mut self.backend, CLEAR_ARGB, |backend| {
            render_preview_runtime_layers(
                backend,
                background_layer,
                foreground_layer,
                SrdExternalRenderState::without_scissor(),
                true,
                true,
                RuntimeSrdSourceFilter::All,
            )
        });
        let end_result = phase(GpuRenderPhase::End);
        match (render_result, end_result) {
            (Err(render_error), Ok(())) => {
                return Err(format!("GPU preview draw failed: {render_error}"));
            }
            (Ok(()), Err(end_error)) => return Err(end_error),
            (Err(render_error), Err(end_error)) => {
                return Err(format!(
                    "GPU preview draw failed: {render_error}; additionally failed to end the host scene: {end_error}"
                ));
            }
            (Ok(()), Ok(())) => {}
        }

        let readback = self
            .backend
            .read_composition()
            .map_err(|error| format!("GPU preview readback failed: {error}"))?;
        let mut frame = PreviewFrame::from_rgba(readback.width, readback.height, readback.rgba)?;
        frame.highlight_bounds = highlight_bounds;
        frame.selection_bounds = selection_bounds;
        Ok(frame)
    }
}

fn preview_layer_composition_size(
    document: &EditorDocument,
    request: &PreviewLayerRequest<'_>,
) -> Result<[u32; 2], String> {
    let scene = document
        .project
        .scenes
        .get(request.scene_index)
        .ok_or_else(|| format!("scene {} is not available", request.scene_index))?;
    checked_scene_size(scene.width, scene.height)
}

fn preview_host_with_project_camera(
    mut host: SrdHostDrawContext,
    camera: CameraDefinition,
    composition_size: [u32; 2],
) -> SrdHostDrawContext {
    let aspect = (composition_size[0] as f32) / (composition_size[1] as f32);
    host.target_projection_view = camera.runtime_matrices(aspect).projection_view;
    host
}

fn preview_host_for_profile(
    profile: PreviewProfile,
) -> Result<
    (
        SrdHostDrawContext,
        EvidenceSrdType1TargetFilter,
        ChusanAirSceneTargetProfile,
    ),
    String,
> {
    let (host, filter, target) = match profile {
        PreviewProfile::AdvertiseLogoMain => {
            let target = CHUSAN_MAIN_SCENE;
            (
                CHUSAN_ADVERTISE_LOGO_PLAYER.host_context_for_target(
                    target,
                    GAME_PRESENT_SIZE[0],
                    GAME_PRESENT_SIZE[1],
                    GAME_SCREEN_SIZE,
                ),
                CHUSAN_ADVERTISE_LOGO_PLAYER.initial_srd_target_filter(target),
                target,
            )
        }
        PreviewProfile::CommonBackgroundMain => {
            let target = CHUSAN_MAIN_SCENE;
            (
                CHUSAN_COMMON_BACKGROUND_PLAYER.host_context_for_target(
                    target,
                    GAME_PRESENT_SIZE[0],
                    GAME_PRESENT_SIZE[1],
                    GAME_SCREEN_SIZE,
                ),
                CHUSAN_COMMON_BACKGROUND_PLAYER.initial_srd_target_filter(target),
                target,
            )
        }
        PreviewProfile::LinkedVerseGateMain => {
            let target = CHUSAN_MAIN_SCENE;
            (
                CHUSAN_LINKED_VERSE_GATE_PLAYER.host_context_for_target(
                    target,
                    GAME_PRESENT_SIZE[0],
                    GAME_PRESENT_SIZE[1],
                    GAME_SCREEN_SIZE,
                ),
                CHUSAN_LINKED_VERSE_GATE_PLAYER.initial_srd_target_filter(target),
                target,
            )
        }
        PreviewProfile::AdvertiseLogoBackground => {
            let target = CHUSAN_BG_SCENE;
            (
                CHUSAN_ADVERTISE_LOGO_PLAYER.host_context_for_target(
                    target,
                    GAME_PRESENT_SIZE[0],
                    GAME_PRESENT_SIZE[1],
                    GAME_SCREEN_SIZE,
                ),
                CHUSAN_ADVERTISE_LOGO_PLAYER.initial_srd_target_filter(target),
                target,
            )
        }
    };
    Ok((host.map_err(|error| error.to_string())?, filter, target))
}

fn prepare_runtime_preview_layer<B: ManagedSrdRenderBackend>(
    backend: &mut B,
    resources: &mut PreviewDocumentResources,
    document: &EditorDocument,
    request: &PreviewLayerRequest<'_>,
    host: SrdHostDrawContext,
    target: ChusanAirSceneTargetProfile,
    filter: EvidenceSrdType1TargetFilter,
) -> Result<PreparedRuntimePreviewLayer, String> {
    let mut font_registry = FennelFontSlotRegistry::default();
    let assignments = assign_fennel_font_resource_requests(
        &mut font_registry,
        collect_fennel_font_resource_requests(&document.project)
            .map_err(|error| format!("failed to collect Fennel font resources: {error}"))?,
    );
    prepare_document_resources(backend, resources, document, &assignments);
    ensure_fennel_resources(backend, resources, document.asset_path(), &assignments)?;
    let reference_plan = document
        .project
        .build_reference_runtime_plan()
        .map_err(|error| format!("failed to resolve Surfride reference layers: {error}"))?;
    let draws = if let Some(animation_assignment) = request.animation_assignment {
        build_evidence_complete_animation_assignment_runtime_cast_draws(
            &document.project,
            &document.textures,
            request.scene_index,
            animation_assignment,
            request.frame as f32,
            host,
            &font_registry,
            &resources.runtime_fonts,
            false,
            request.runtime_text_inputs,
        )
    } else {
        build_evidence_complete_initial_runtime_cast_draws(
            &document.project,
            &document.textures,
            request.scene_index,
            host,
            &font_registry,
            &resources.runtime_fonts,
            false,
            request.runtime_text_inputs,
        )
    }
    .map_err(|error| format!("failed to evaluate Surfride runtime draws: {error}"))?;
    let draws = filter_editor_preview_draws(
        draws,
        &document.project,
        &reference_plan,
        request.scene_index,
        request.hidden_layers,
        request.solo_layers,
        request.hidden_casts,
        request.solo_casts,
    )?;
    let target_profile = target
        .scene_pass_profile()
        .map_err(|error| error.to_string())?;
    let submission =
        build_evidence_filtered_merged_runtime_target_submission(&draws, &target_profile, filter)
            .map_err(|error| format!("failed to build Ceylon target submission: {error}"))?;
    let required_indices = draws
        .iter()
        .filter_map(runtime_srd_draw)
        .flat_map(|draw| draw.texture_bindings.iter().flatten())
        .map(|binding| binding.texture_index)
        .collect::<BTreeSet<_>>();
    ensure_textures(backend, resources, document, &required_indices)?;
    Ok(PreparedRuntimePreviewLayer {
        draws,
        submission,
        reference_plan,
    })
}

#[derive(Clone, Copy)]
struct RuntimeBoundsRequest {
    selection: PreviewCastSelection,
    kind: Option<super::PreviewHighlightKind>,
}

fn runtime_cast_bounds(
    project: &crate::scene::Project,
    reference_plan: &ReferenceRuntimePlan,
    draws: &[EvidenceCompleteRuntimeCastDraw],
    submission: &[EvidenceMergedRuntimeTargetCommand],
    selection: Option<PreviewCastSelection>,
    highlights: &[PreviewHighlightRequest],
    composition_size: [u32; 2],
) -> (Option<[u32; 4]>, Vec<PreviewHighlightBounds>) {
    let mut requests = Vec::with_capacity(highlights.len() + selection.is_some() as usize);
    let selection_index = selection.map(|selection| {
        let index = requests.len();
        requests.push(RuntimeBoundsRequest {
            selection,
            kind: None,
        });
        index
    });
    requests.extend(highlights.iter().map(|highlight| RuntimeBoundsRequest {
        selection: highlight.selection,
        kind: Some(highlight.kind),
    }));

    let mut nodes_by_project_owner =
        BTreeMap::<(crate::scene::ReferenceTarget, usize), Vec<usize>>::new();
    let mut requests_by_reference_owner = BTreeMap::<usize, Vec<usize>>::new();
    for (request_index, request) in requests.iter().enumerate() {
        let target = crate::scene::ReferenceTarget {
            scene_index: request.selection.scene_index,
            layer_index: request.selection.layer_index,
        };
        let Some(layer) = project
            .scenes
            .get(request.selection.scene_index)
            .and_then(|scene| scene.layers.get(request.selection.layer_index))
        else {
            continue;
        };
        if request.selection.node_index >= layer.nodes.len() {
            continue;
        }

        let mut descendants = vec![false; layer.nodes.len()];
        descendants[request.selection.node_index] = true;
        if let Ok(hierarchy) = layer.build_hierarchy() {
            let mut stack = vec![request.selection.node_index];
            while let Some(node_index) = stack.pop() {
                for &child in hierarchy
                    .children
                    .get(node_index)
                    .map(Vec::as_slice)
                    .unwrap_or_default()
                {
                    if !descendants[child] {
                        descendants[child] = true;
                        stack.push(child);
                    }
                }
            }
        }
        for (node_index, included) in descendants.iter().copied().enumerate() {
            if included {
                nodes_by_project_owner
                    .entry((target, node_index))
                    .or_default()
                    .push(request_index);
            }
        }

        let mut included_instances = vec![false; reference_plan.instances.len()];
        for (instance_index, instance) in reference_plan.instances.iter().enumerate() {
            included_instances[instance_index] = match instance.parent {
                ReferenceLayerParent::ProjectLayer(parent) => {
                    parent == target
                        && descendants
                            .get(instance.reference_node_index)
                            .copied()
                            .unwrap_or(false)
                }
                ReferenceLayerParent::ReferenceInstance(parent_index) => included_instances
                    .get(parent_index)
                    .copied()
                    .unwrap_or(false),
            };
            if included_instances[instance_index] {
                requests_by_reference_owner
                    .entry(instance_index)
                    .or_default()
                    .push(request_index);
            }
        }
    }

    let mut admitted = vec![false; draws.len()];
    for source in submission
        .iter()
        .flat_map(|command| command.sources.iter().copied())
    {
        let runtime_draw_index = match source {
            EvidenceRuntimeTargetCommandSource::Image { runtime_draw_index }
            | EvidenceRuntimeTargetCommandSource::SliceCell { runtime_draw_index }
            | EvidenceRuntimeTargetCommandSource::NumberGlyph { runtime_draw_index }
            | EvidenceRuntimeTargetCommandSource::FennelBatch {
                runtime_draw_index, ..
            } => runtime_draw_index,
        };
        if let Some(admitted) = admitted.get_mut(runtime_draw_index) {
            *admitted = true;
        }
    }

    let mut bounds = vec![
        [
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        ];
        requests.len()
    ];
    for (runtime_draw_index, draw) in draws.iter().enumerate() {
        if !admitted[runtime_draw_index] {
            continue;
        }
        let request_indices = match draw.owner() {
            ReferenceLayerParent::ProjectLayer(owner) => nodes_by_project_owner
                .get(&(owner, draw.node_index()))
                .map(Vec::as_slice),
            ReferenceLayerParent::ReferenceInstance(instance_index) => requests_by_reference_owner
                .get(&instance_index)
                .map(Vec::as_slice),
        };
        let Some(request_indices) = request_indices else {
            continue;
        };
        let Some(draw_bounds) = runtime_draw_bounds(draw, composition_size) else {
            continue;
        };
        for &request_index in request_indices {
            let request_bounds = &mut bounds[request_index];
            request_bounds[0] = request_bounds[0].min(draw_bounds[0]);
            request_bounds[1] = request_bounds[1].min(draw_bounds[1]);
            request_bounds[2] = request_bounds[2].max(draw_bounds[2]);
            request_bounds[3] = request_bounds[3].max(draw_bounds[3]);
        }
    }

    let selection_bounds =
        selection_index.and_then(|index| clip_bounds(bounds[index], composition_size));
    let highlight_bounds = requests
        .iter()
        .zip(bounds)
        .filter_map(|(request, bounds)| {
            Some(PreviewHighlightBounds {
                kind: request.kind?,
                bounds: clip_bounds(bounds, composition_size)?,
            })
        })
        .collect();
    (selection_bounds, highlight_bounds)
}

fn runtime_draw_bounds(
    draw: &EvidenceCompleteRuntimeCastDraw,
    composition_size: [u32; 2],
) -> Option<[f32; 4]> {
    let mut bounds = [
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    ];
    match draw {
        EvidenceCompleteRuntimeCastDraw::Image(draw) => accumulate_vertex_bounds(
            draw.quad.vertices.iter().map(|vertex| vertex.position),
            draw.is_2d,
            draw.fixed_constants,
            composition_size,
            &mut bounds,
        ),
        EvidenceCompleteRuntimeCastDraw::SliceCell(cell) => accumulate_vertex_bounds(
            cell.draw.quad.vertices.iter().map(|vertex| vertex.position),
            cell.draw.is_2d,
            cell.draw.fixed_constants,
            composition_size,
            &mut bounds,
        ),
        EvidenceCompleteRuntimeCastDraw::NumberGlyph(glyph) => accumulate_vertex_bounds(
            glyph
                .draw
                .quad
                .vertices
                .iter()
                .map(|vertex| vertex.position),
            glyph.draw.is_2d,
            glyph.draw.fixed_constants,
            composition_size,
            &mut bounds,
        ),
        EvidenceCompleteRuntimeCastDraw::Fennel(draw) => accumulate_vertex_bounds(
            draw.batches
                .iter()
                .flat_map(|batch| batch.vertices.iter())
                .map(|vertex| vertex.position),
            draw.is_2d,
            draw.fixed_constants,
            composition_size,
            &mut bounds,
        ),
    }
    bounds
        .iter()
        .all(|value| value.is_finite())
        .then_some(bounds)
}

fn clip_bounds(bounds: [f32; 4], composition_size: [u32; 2]) -> Option<[u32; 4]> {
    if !bounds.iter().all(|value| value.is_finite()) {
        return None;
    }
    let left = bounds[0].floor().clamp(0.0, composition_size[0] as f32) as u32;
    let top = bounds[1].floor().clamp(0.0, composition_size[1] as f32) as u32;
    let right = bounds[2].ceil().clamp(0.0, composition_size[0] as f32) as u32;
    let bottom = bounds[3].ceil().clamp(0.0, composition_size[1] as f32) as u32;
    (right > left && bottom > top).then_some([left, top, right - left, bottom - top])
}

fn accumulate_vertex_bounds(
    positions: impl IntoIterator<Item = [f32; 3]>,
    is_2d: bool,
    constants: crate::render::CeylonSrdFixedShaderConstants,
    composition_size: [u32; 2],
    bounds: &mut [f32; 4],
) {
    let screen_matrix = (!is_2d).then(|| {
        let projection_world = mul_matrix4x4_game(
            &constants.vertex_c10_c13_projection_view,
            &constants.vertex_c0_c3_world,
        );
        mul_matrix4x4_game(
            &viewport_matrix_game(composition_size[0] as i32, composition_size[1] as i32),
            &projection_world,
        )
    });
    for position in positions {
        let point = screen_matrix
            .as_ref()
            .map_or([position[0], position[1]], |matrix| {
                project_point_to_screen_game(position, matrix)
            });
        if !point.iter().all(|value| value.is_finite()) {
            continue;
        }
        bounds[0] = bounds[0].min(point[0]);
        bounds[1] = bounds[1].min(point[1]);
        bounds[2] = bounds[2].max(point[0]);
        bounds[3] = bounds[3].max(point[1]);
    }
}

fn prepare_document_resources<B: ManagedSrdRenderBackend>(
    backend: &mut B,
    resources: &mut PreviewDocumentResources,
    document: &EditorDocument,
    assignments: &[FennelFontResourceAssignment],
) {
    if resources.asset_path.as_deref() == Some(document.asset_path())
        && resources.texture_filenames.len() == document.textures.textures.len()
        && resources
            .texture_filenames
            .iter()
            .zip(&document.textures.textures)
            .all(|(cached, texture)| cached == &texture.filename)
        && resources.fennel_assignments.as_slice() == assignments
    {
        return;
    }
    release_document_resources(backend, resources);
    resources.asset_path = Some(document.asset_path().to_path_buf());
    resources.texture_filenames = document
        .textures
        .textures
        .iter()
        .map(|texture| texture.filename.clone())
        .collect();
    resources.fennel_assignments.extend_from_slice(assignments);
}

fn ensure_fennel_resources<B: ManagedSrdRenderBackend>(
    backend: &mut B,
    resources: &mut PreviewDocumentResources,
    document_path: &Path,
    assignments: &[FennelFontResourceAssignment],
) -> Result<(), String> {
    if resources.fennel_resources_loaded {
        return Ok(());
    }
    let loaded = if assignments.is_empty() {
        Ok((BTreeMap::new(), BTreeMap::new()))
    } else {
        let root = find_game_data_root(document_path).ok_or_else(|| {
            format!(
                "could not locate a data directory above {} for RFZ resources",
                document_path.display()
            )
        })?;
        load_fennel_resources(backend, &root, assignments)
    };
    match loaded {
        Ok((runtime_fonts, fennel_atlas_routes)) => {
            resources.runtime_fonts = runtime_fonts;
            resources.fennel_atlas_routes = fennel_atlas_routes;
            resources.fennel_resources_loaded = true;
            Ok(())
        }
        Err(error) => {
            release_document_resources(backend, resources);
            Err(error)
        }
    }
}

fn ensure_textures<B: ManagedSrdRenderBackend>(
    backend: &mut B,
    resources: &mut PreviewDocumentResources,
    document: &EditorDocument,
    required: &BTreeSet<usize>,
) -> Result<(), String> {
    if required.is_subset(&resources.texture_indices) {
        return Ok(());
    }
    if let Some(textures) = resources.textures.take() {
        backend.release_srd_textures(textures);
    }
    resources.texture_indices.clear();
    let root = find_game_data_root(document.asset_path()).ok_or_else(|| {
        format!(
            "could not locate a data directory above {} for DDS resources",
            document.asset_path().display()
        )
    })?;
    let sources =
        SrdTextureSourceSet::load_required(&root, &document.textures, required.iter().copied())
            .map_err(|error| format!("failed to load SRD texture sources: {error}"))?;
    let textures = backend
        .upload_srd_textures(sources)
        .map_err(|error| format!("failed to upload SRD textures: {error}"))?;
    resources.textures = Some(textures);
    resources.texture_indices.clone_from(required);
    Ok(())
}

fn release_document_resources<B: ManagedSrdRenderBackend>(
    backend: &mut B,
    resources: &mut PreviewDocumentResources,
) {
    if let Some(textures) = resources.textures.take() {
        backend.release_srd_textures(textures);
    }
    let atlases = resources
        .fennel_atlas_routes
        .values()
        .map(|route| route.atlas)
        .collect::<BTreeSet<_>>();
    for atlas in atlases {
        backend.release_fennel_atlas(atlas);
    }
    *resources = PreviewDocumentResources::default();
}

fn filter_editor_preview_draws(
    draws: Vec<EvidenceCompleteRuntimeCastDraw>,
    project: &crate::scene::Project,
    reference_plan: &ReferenceRuntimePlan,
    selected_scene_index: usize,
    hidden_layers: &BTreeSet<usize>,
    solo_layers: &BTreeSet<usize>,
    hidden_casts: &BTreeSet<PreviewCastSelection>,
    solo_casts: &BTreeSet<PreviewCastSelection>,
) -> Result<Vec<EvidenceCompleteRuntimeCastDraw>, String> {
    let mut visible = Vec::with_capacity(draws.len());
    let mut hierarchy_parents = BTreeMap::new();
    for draw in draws {
        let root = reference_plan
            .root_project_layer(draw.owner())
            .map_err(|error| format!("failed to resolve runtime draw owner ancestry: {error}"))?;
        if root.scene_index != selected_scene_index {
            return Err(format!(
                "runtime draw owner {:?} roots in scene {}, not selected scene {selected_scene_index}",
                draw.owner(),
                root.scene_index
            ));
        }
        if hidden_layers.contains(&root.layer_index)
            || (!solo_layers.is_empty() && !solo_layers.contains(&root.layer_index))
            || draw_matches_cast_selection(
                project,
                reference_plan,
                &draw,
                hidden_casts,
                &mut hierarchy_parents,
            )?
        {
            continue;
        }
        if !solo_casts.is_empty()
            && !draw_matches_cast_selection(
                project,
                reference_plan,
                &draw,
                solo_casts,
                &mut hierarchy_parents,
            )?
        {
            continue;
        }
        visible.push(draw);
    }
    Ok(visible)
}

fn draw_matches_cast_selection(
    project: &crate::scene::Project,
    reference_plan: &ReferenceRuntimePlan,
    draw: &EvidenceCompleteRuntimeCastDraw,
    selections: &BTreeSet<PreviewCastSelection>,
    hierarchy_parents: &mut BTreeMap<crate::scene::ReferenceTarget, Vec<Option<usize>>>,
) -> Result<bool, String> {
    let mut owner = draw.owner();
    let mut node_index = draw.node_index();
    let mut visited_instances = BTreeSet::new();
    loop {
        let (source, parent) = match owner {
            ReferenceLayerParent::ProjectLayer(source) => (source, None),
            ReferenceLayerParent::ReferenceInstance(instance_index) => {
                if !visited_instances.insert(instance_index) {
                    return Err(format!(
                        "reference instance ancestry cycles at {instance_index} while filtering CAST controls"
                    ));
                }
                let instance = reference_plan.instances.get(instance_index).ok_or_else(|| {
                    format!(
                        "reference instance {instance_index} is outside the runtime plan while filtering CAST controls"
                    )
                })?;
                (
                    instance.target,
                    Some((instance.parent, instance.reference_node_index)),
                )
            }
        };
        if selections.iter().any(|selection| {
            selection.scene_index == source.scene_index
                && selection.layer_index == source.layer_index
        }) {
            let parents = hierarchy_parent_indices(project, source, hierarchy_parents)?;
            if selections
                .iter()
                .filter(|selection| {
                    selection.scene_index == source.scene_index
                        && selection.layer_index == source.layer_index
                })
                .any(|selection| node_is_in_subtree(parents, selection.node_index, node_index))
            {
                return Ok(true);
            }
        }
        let Some((parent_owner, parent_node_index)) = parent else {
            return Ok(false);
        };
        owner = parent_owner;
        node_index = parent_node_index;
    }
}

fn hierarchy_parent_indices<'a>(
    project: &crate::scene::Project,
    target: crate::scene::ReferenceTarget,
    cache: &'a mut BTreeMap<crate::scene::ReferenceTarget, Vec<Option<usize>>>,
) -> Result<&'a [Option<usize>], String> {
    if !cache.contains_key(&target) {
        let layer = project
            .scenes
            .get(target.scene_index)
            .and_then(|scene| scene.layers.get(target.layer_index))
            .ok_or_else(|| {
                format!(
                    "runtime draw source layer ({}, {}) is missing while filtering CAST controls",
                    target.scene_index, target.layer_index
                )
            })?;
        let parents = layer
            .build_hierarchy()
            .map_err(|error| {
                format!("failed to build CAST hierarchy for preview filtering: {error}")
            })?
            .parents;
        cache.insert(target, parents);
    }
    Ok(cache
        .get(&target)
        .expect("inserted hierarchy parent indices")
        .as_slice())
}

fn node_is_in_subtree(parents: &[Option<usize>], ancestor: usize, mut node_index: usize) -> bool {
    for _ in 0..=parents.len() {
        if node_index == ancestor {
            return true;
        }
        let Some(parent) = parents.get(node_index).copied().flatten() else {
            return false;
        };
        node_index = parent;
    }
    false
}

fn runtime_srd_draw(draw: &EvidenceCompleteRuntimeCastDraw) -> Option<&EvidenceCompleteSrdDraw> {
    match draw {
        EvidenceCompleteRuntimeCastDraw::Image(draw) => Some(draw),
        EvidenceCompleteRuntimeCastDraw::SliceCell(cell) => Some(&cell.draw),
        EvidenceCompleteRuntimeCastDraw::NumberGlyph(glyph) => Some(&glyph.draw),
        EvidenceCompleteRuntimeCastDraw::Fennel(_) => None,
    }
}

fn load_fennel_resources<B: ManagedSrdRenderBackend>(
    backend: &mut B,
    game_data_root: &Path,
    assignments: &[FennelFontResourceAssignment],
) -> Result<
    (
        BTreeMap<Vec<u8>, RuhunaRuntimeFont>,
        BTreeMap<u32, FennelAtlasRoute>,
    ),
    String,
> {
    let mut uploaded_atlases = BTreeSet::new();
    let resources = (|| {
        let mut runtime_fonts = BTreeMap::new();
        let mut atlas_routes = BTreeMap::new();
        let mut next_texture_token = 1u32;
        for assignment in assignments {
            if !assignment.slot.first_request
                || !assignment.slot.registered
                || runtime_fonts.contains_key(assignment.request.name.as_slice())
                || !assignment
                    .request
                    .name
                    .get(assignment.request.name.len().saturating_sub(4)..)
                    .is_some_and(|suffix| suffix.eq_ignore_ascii_case(b".rfz"))
            {
                continue;
            }
            let name = std::str::from_utf8(&assignment.request.name)
                .map_err(|error| format!("RFZ font name is not UTF-8: {error}"))?;
            let path = game_data_root.join("A000/font").join(name);
            let parsed = RuhunaFont::from_rfz(
                &fs::read(&path)
                    .map_err(|error| format!("failed to read {}: {error}", path.display()))?,
            )
            .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
            let sources = FennelAtlasSourceSet::from_font(&parsed)
                .map_err(|error| format!("failed to prepare {}: {error}", path.display()))?;
            let page_count = sources.page_count();
            let atlas = backend
                .upload_fennel_atlas(sources)
                .map_err(|error| format!("failed to upload {}: {error}", path.display()))?;
            uploaded_atlases.insert(atlas);
            let mut page_tokens = Vec::with_capacity(page_count);
            for page_index in 0..page_count {
                let texture_token = next_texture_token;
                next_texture_token = next_texture_token.checked_add(1).ok_or_else(|| {
                    "GPU preview Fennel texture-token space exhausted while loading atlases"
                        .to_string()
                })?;
                atlas_routes.insert(texture_token, FennelAtlasRoute { atlas, page_index });
                page_tokens.push(texture_token);
            }
            let owner_token = u32::try_from(assignment.slot.resource_handle).map_err(|_| {
                format!(
                    "Fennel resource handle {} does not fit the runtime glyph token",
                    assignment.slot.resource_handle
                )
            })?;
            let runtime = parsed
                .build_runtime_font(owner_token, |page| {
                    page_tokens.get(usize::from(page)).copied().unwrap_or(0)
                })
                .map_err(|error| format!("failed to build runtime {}: {error}", path.display()))?;
            runtime_fonts.insert(assignment.request.name.clone(), runtime);
        }
        Ok((runtime_fonts, atlas_routes))
    })();
    if resources.is_err() {
        for atlas in uploaded_atlases {
            backend.release_fennel_atlas(atlas);
        }
    }
    resources
}

fn checked_scene_size(width: f32, height: f32) -> Result<[u32; 2], String> {
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return Err(format!(
            "scene composition size is invalid: {width} × {height}"
        ));
    }
    if width.fract() != 0.0 || height.fract() != 0.0 {
        return Err(format!(
            "fractional scene composition sizes are not evidence-complete: {width} × {height}"
        ));
    }
    if width > u32::MAX as f32 || height > u32::MAX as f32 {
        return Err(format!(
            "scene composition size exceeds renderer dimensions: {width} × {height}"
        ));
    }
    Ok([width as u32, height as u32])
}

fn find_game_data_root(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find(|ancestor| {
            ancestor
                .file_name()
                .is_some_and(|name| name.eq_ignore_ascii_case("data"))
        })
        .map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::identity_matrix4x4_game;
    use crate::reference_runtime::ReferenceLayerInstance;
    use crate::render::{
        CeylonDepthState, CeylonDrawPacketPresetState, CeylonRasterState,
        CeylonSrdFixedShaderConstants, SrdQuadDraw, SrdRenderVertex, ceylon_d3d9_blend_preset,
    };
    use crate::renderer::backend::{CompositionReadback, FennelRenderBatch, RenderBackendError};
    use crate::scene::{Layer, NodeRecord, Project, RawTransform, ReferenceTarget, Scene};
    use crate::shader::CEYLON_SIMPLE_SHADER_KEY_LENGTH;
    use crate::transform::SpatialTransform;

    #[derive(Default)]
    struct RecordingBackend {
        composition_size: Option<[u32; 2]>,
        rgba: Vec<u8>,
        draw_calls: usize,
    }

    impl RecordingBackend {
        fn mark_draw(&mut self) {
            self.draw_calls += 1;
            if let Some(pixel) = self.rgba.get_mut(..4) {
                pixel.copy_from_slice(&[0xE1, 0x43, 0x27, 0xFF]);
            }
        }
    }

    impl SrdRenderBackend for RecordingBackend {
        fn name(&self) -> &'static str {
            "recording"
        }

        fn upload_srd_textures(
            &mut self,
            _sources: SrdTextureSourceSet,
        ) -> Result<SrdTextureSetHandle, RenderBackendError> {
            Ok(SrdTextureSetHandle::new(1))
        }

        fn upload_fennel_atlas(
            &mut self,
            _sources: FennelAtlasSourceSet,
        ) -> Result<FennelAtlasHandle, RenderBackendError> {
            Ok(FennelAtlasHandle::new(2))
        }

        fn clear_uploaded_resources(&mut self) {}

        fn configure_composition_target(
            &mut self,
            size: [u32; 2],
        ) -> Result<(), RenderBackendError> {
            self.composition_size = Some(size);
            Ok(())
        }

        fn composition_size(&self) -> Option<[u32; 2]> {
            self.composition_size
        }

        fn begin_composition(&mut self, clear_argb: u32) -> Result<(), RenderBackendError> {
            let [width, height] = self
                .composition_size
                .ok_or_else(|| RenderBackendError("composition is not configured".into()))?;
            let pixel_count = usize::try_from(width).unwrap() * usize::try_from(height).unwrap();
            let clear = [
                (clear_argb >> 16) as u8,
                (clear_argb >> 8) as u8,
                clear_argb as u8,
                (clear_argb >> 24) as u8,
            ];
            self.rgba.resize(pixel_count * 4, 0);
            for pixel in self.rgba.chunks_exact_mut(4) {
                pixel.copy_from_slice(&clear);
            }
            Ok(())
        }

        fn end_composition(&mut self) -> Result<(), RenderBackendError> {
            Ok(())
        }

        fn read_composition(&self) -> Result<CompositionReadback, RenderBackendError> {
            let [width, height] = self
                .composition_size
                .ok_or_else(|| RenderBackendError("composition is not configured".into()))?;
            Ok(CompositionReadback {
                width,
                height,
                rgba: self.rgba.clone(),
            })
        }

        fn render_srd(
            &mut self,
            draws: &[EvidenceCompleteSrdDraw],
            _external: SrdExternalRenderState,
            _textures: Option<SrdTextureSetHandle>,
        ) -> Result<(), RenderBackendError> {
            if !draws.is_empty() {
                self.mark_draw();
            }
            Ok(())
        }

        fn render_srd_triangle_strip(
            &mut self,
            _state: &EvidenceCompleteSrdDraw,
            _vertices: &[SrdRenderVertex],
            _external: SrdExternalRenderState,
            _textures: Option<SrdTextureSetHandle>,
        ) -> Result<(), RenderBackendError> {
            self.mark_draw();
            Ok(())
        }

        fn render_fennel(
            &mut self,
            batches: &[FennelRenderBatch<'_>],
            _external: SrdExternalRenderState,
            _atlas: FennelAtlasHandle,
        ) -> Result<(), RenderBackendError> {
            if !batches.is_empty() {
                self.mark_draw();
            }
            Ok(())
        }
    }

    impl ManagedSrdRenderBackend for RecordingBackend {
        fn release_srd_textures(&mut self, _handle: SrdTextureSetHandle) {}

        fn release_fennel_atlas(&mut self, _handle: FennelAtlasHandle) {}
    }

    #[test]
    fn layerless_first_scene_renders_the_clear_composition() {
        let mut document = EditorDocument::empty();
        document.project.declared_scene_count = 1;
        document.project.scenes.push(Scene {
            name: b"Scene 1".to_vec(),
            declared_layer_count: 0,
            declared_animation_set_count: 0,
            width: 1920.0,
            height: 1080.0,
            layers: Vec::new(),
            animation_sets: Vec::new(),
        });
        let empty_layers = BTreeSet::new();
        let empty_casts = BTreeSet::new();
        let runtime_text_inputs = BTreeMap::new();
        let mut state = GpuPreviewState::new(RecordingBackend::default());
        let mut phases = Vec::new();

        let frame = state
            .render(
                PreviewRequest {
                    foreground: PreviewLayerRequest {
                        document: &document,
                        document_revision: 1,
                        scene_index: 0,
                        animation_assignment: None,
                        frame: 0,
                        hidden_layers: &empty_layers,
                        solo_layers: &empty_layers,
                        hidden_casts: &empty_casts,
                        solo_casts: &empty_casts,
                        runtime_text_inputs: &runtime_text_inputs,
                    },
                    background: None,
                    profile: PreviewProfile::AdvertiseLogoMain,
                    selection: None,
                    highlights: &[],
                },
                |phase| {
                    phases.push(phase);
                    Ok(())
                },
            )
            .unwrap();

        assert_eq!(phases, [GpuRenderPhase::Begin, GpuRenderPhase::End]);
        assert_eq!(state.backend.draw_calls, 0);
        assert_eq!([frame.width, frame.height], [1920, 1080]);
        assert_eq!(&frame.rgba[..4], &[0x06, 0x0B, 0x17, 0xFF]);
        assert_eq!(frame.selection_bounds, None);
        assert!(frame.highlight_bounds.is_empty());
    }

    #[test]
    fn base_pose_preview_renders_submitted_fixture_geometry_without_an_animation_set() {
        let Some(path) =
            crate::test_support::game_data_path("surfboard/advertise/chu_ui_advertise_00_v10.srd")
        else {
            return;
        };
        let document = EditorDocument::load(path).unwrap();
        let hidden_layers = BTreeSet::new();
        let runtime_text_inputs = BTreeMap::new();
        let hidden_casts = BTreeSet::new();
        let solo_casts = BTreeSet::new();
        let mut state = GpuPreviewState::new(RecordingBackend::default());
        let mut phases = Vec::new();

        let frame = state
            .render(
                PreviewRequest {
                    foreground: PreviewLayerRequest {
                        document: &document,
                        document_revision: 0,
                        scene_index: 0,
                        animation_assignment: None,
                        frame: 0,
                        hidden_layers: &hidden_layers,
                        solo_layers: &hidden_layers,
                        hidden_casts: &hidden_casts,
                        solo_casts: &solo_casts,
                        runtime_text_inputs: &runtime_text_inputs,
                    },
                    background: None,
                    profile: PreviewProfile::AdvertiseLogoMain,
                    selection: Some(PreviewCastSelection {
                        scene_index: 0,
                        layer_index: 0,
                        node_index: 0,
                    }),
                    highlights: &[],
                },
                |phase| {
                    phases.push(phase);
                    Ok(())
                },
            )
            .unwrap();

        assert_eq!(phases, [GpuRenderPhase::Begin, GpuRenderPhase::End]);
        assert!(state.backend.draw_calls > 0);
        assert_eq!([frame.width, frame.height], [1920, 1080]);
        assert_eq!(&frame.rgba[..4], &[0xE1, 0x43, 0x27, 0xFF]);
        let [left, top, width, height] = frame
            .selection_bounds
            .expect("the selected NullCast includes its submitted C_black child");
        assert!(width > 0 && height > 0);
        assert!(left + width <= frame.width && top + height <= frame.height);
    }

    fn test_node(first_child_index: i16, next_sibling_index: i16) -> NodeRecord {
        NodeRecord {
            name: None,
            type_flags: Some(1),
            parent_csli_cell_index: None,
            first_child_index,
            next_sibling_index,
            field_a0: None,
        }
    }

    fn test_layer(nodes: Vec<NodeRecord>) -> Layer {
        let node_count = nodes.len();
        Layer {
            name: Vec::new(),
            flags: 0x100,
            animation_count: 0,
            animations: Vec::new(),
            field_23: Vec::new(),
            nodes,
            transforms: vec![RawTransform::Trs2(SpatialTransform::default()); node_count],
            image_by_node: vec![None; node_count],
            number_by_node: vec![None; node_count],
            reference_by_node: vec![None; node_count],
            csli_by_node: vec![None; node_count],
            cast_attribute_lists: Vec::new(),
            cast_attribute_list_by_node: vec![None; node_count],
        }
    }

    fn selection_project() -> Project {
        Project {
            name: Vec::new(),
            declared_scene_count: 1,
            declared_font_count: 0,
            camera: CameraDefinition::ZERO_FALLBACK,
            scenes: vec![Scene {
                name: Vec::new(),
                declared_layer_count: 2,
                declared_animation_set_count: 0,
                width: 100.0,
                height: 100.0,
                layers: vec![
                    test_layer(vec![test_node(1, -1), test_node(-1, 2), test_node(-1, -1)]),
                    test_layer(vec![test_node(-1, -1)]),
                ],
                animation_sets: Vec::new(),
            }],
            fonts: Vec::new(),
        }
    }

    fn test_vertex(x: f32, y: f32) -> SrdRenderVertex {
        SrdRenderVertex {
            position: [x, y, 0.0],
            primary_color: [255; 4],
            secondary_color: [0; 4],
            texture_coordinates: [[0.0; 2]; 2],
        }
    }

    fn test_draw(
        owner: ReferenceLayerParent,
        node_index: usize,
        rectangle: [f32; 4],
    ) -> EvidenceCompleteRuntimeCastDraw {
        let [left, top, right, bottom] = rectangle;
        EvidenceCompleteRuntimeCastDraw::Image(EvidenceCompleteSrdDraw {
            owner,
            scene_index: 0,
            layer_index: 0,
            node_index,
            is_2d: true,
            renderer_layer_key: 0,
            shader_key: [0; CEYLON_SIMPLE_SHADER_KEY_LENGTH],
            quad: SrdQuadDraw::new([
                test_vertex(left, top),
                test_vertex(left, bottom),
                test_vertex(right, top),
                test_vertex(right, bottom),
            ]),
            packet: CeylonDrawPacketPresetState::default(),
            fixed_constants: CeylonSrdFixedShaderConstants::initial_for_target(
                identity_matrix4x4_game(),
                [100, 100],
            ),
            blend: ceylon_d3d9_blend_preset(0),
            raster: CeylonRasterState::default(),
            depth: CeylonDepthState::from_draw_flags(0),
            texture_bindings: [None; 3],
        })
    }

    fn submitted(runtime_draw_index: usize) -> EvidenceMergedRuntimeTargetCommand {
        EvidenceMergedRuntimeTargetCommand {
            sources: vec![EvidenceRuntimeTargetCommandSource::Image { runtime_draw_index }],
            packet: CeylonDrawPacketPresetState::default(),
            renderer_layer_key: 0,
            vertex_format: SrdRenderVertex::BINARY_FORMAT_ID,
            primitive_type: 5,
            vertex_count: 4,
        }
    }

    #[test]
    fn cast_filters_apply_hidden_precedence_solo_unions_and_reference_descendants() {
        let root = ReferenceTarget {
            scene_index: 0,
            layer_index: 0,
        };
        let referenced = ReferenceTarget {
            scene_index: 0,
            layer_index: 1,
        };
        let plan = ReferenceRuntimePlan {
            instances: vec![ReferenceLayerInstance {
                parent: ReferenceLayerParent::ProjectLayer(root),
                reference_node_index: 1,
                target: referenced,
                is_2d: true,
                flip_y: false,
            }],
            unresolved: Vec::new(),
        };
        let draws = vec![
            test_draw(
                ReferenceLayerParent::ProjectLayer(root),
                0,
                [0.0, 0.0, 1.0, 1.0],
            ),
            test_draw(
                ReferenceLayerParent::ProjectLayer(root),
                2,
                [0.0, 0.0, 1.0, 1.0],
            ),
            test_draw(
                ReferenceLayerParent::ReferenceInstance(0),
                0,
                [0.0, 0.0, 1.0, 1.0],
            ),
        ];
        let mut hidden_casts = BTreeSet::new();
        let mut solo_casts = BTreeSet::new();
        let parent = PreviewCastSelection {
            scene_index: 0,
            layer_index: 0,
            node_index: 0,
        };

        hidden_casts.insert(parent);
        solo_casts.insert(parent);
        assert!(
            filter_editor_preview_draws(
                draws.clone(),
                &selection_project(),
                &plan,
                0,
                &BTreeSet::new(),
                &BTreeSet::new(),
                &hidden_casts,
                &solo_casts,
            )
            .unwrap()
            .is_empty()
        );

        hidden_casts.clear();
        solo_casts.clear();
        solo_casts.insert(PreviewCastSelection {
            scene_index: 0,
            layer_index: 0,
            node_index: 1,
        });
        solo_casts.insert(PreviewCastSelection {
            scene_index: 0,
            layer_index: 0,
            node_index: 2,
        });
        let soloed = filter_editor_preview_draws(
            draws.clone(),
            &selection_project(),
            &plan,
            0,
            &BTreeSet::new(),
            &BTreeSet::new(),
            &hidden_casts,
            &solo_casts,
        )
        .unwrap();
        assert_eq!(soloed.len(), 2);
        assert!(soloed.iter().any(|draw| {
            draw.owner() == ReferenceLayerParent::ProjectLayer(root) && draw.node_index() == 2
        }));
        assert!(
            soloed
                .iter()
                .any(|draw| { draw.owner() == ReferenceLayerParent::ReferenceInstance(0) })
        );

        hidden_casts.insert(PreviewCastSelection {
            scene_index: 0,
            layer_index: 0,
            node_index: 1,
        });
        solo_casts.clear();
        solo_casts.insert(parent);
        let reference_hidden = filter_editor_preview_draws(
            draws,
            &selection_project(),
            &plan,
            0,
            &BTreeSet::new(),
            &BTreeSet::new(),
            &hidden_casts,
            &solo_casts,
        )
        .unwrap();
        assert_eq!(reference_hidden.len(), 2);
        assert!(
            reference_hidden
                .iter()
                .all(|draw| { draw.owner() == ReferenceLayerParent::ProjectLayer(root) })
        );
    }

    #[test]
    fn selection_and_highlight_bounds_include_descendants_and_omit_no_geometry() {
        let selected_layer = ReferenceTarget {
            scene_index: 0,
            layer_index: 0,
        };
        let other_layer = ReferenceTarget {
            scene_index: 0,
            layer_index: 1,
        };
        let plan = ReferenceRuntimePlan {
            instances: vec![
                ReferenceLayerInstance {
                    parent: ReferenceLayerParent::ProjectLayer(selected_layer),
                    reference_node_index: 1,
                    target: other_layer,
                    is_2d: true,
                    flip_y: false,
                },
                ReferenceLayerInstance {
                    parent: ReferenceLayerParent::ReferenceInstance(0),
                    reference_node_index: 0,
                    target: other_layer,
                    is_2d: true,
                    flip_y: false,
                },
                ReferenceLayerInstance {
                    parent: ReferenceLayerParent::ProjectLayer(other_layer),
                    reference_node_index: 0,
                    target: selected_layer,
                    is_2d: true,
                    flip_y: false,
                },
            ],
            unresolved: Vec::new(),
        };
        let draws = vec![
            test_draw(
                ReferenceLayerParent::ProjectLayer(selected_layer),
                0,
                [10.0, 20.0, 30.0, 40.0],
            ),
            test_draw(
                ReferenceLayerParent::ProjectLayer(selected_layer),
                1,
                [40.0, 50.0, 60.0, 70.0],
            ),
            test_draw(
                ReferenceLayerParent::ReferenceInstance(0),
                0,
                [70.0, 80.0, 90.0, 95.0],
            ),
            test_draw(
                ReferenceLayerParent::ReferenceInstance(1),
                0,
                [5.0, 15.0, 20.0, 25.0],
            ),
            test_draw(
                ReferenceLayerParent::ReferenceInstance(2),
                0,
                [0.0, 0.0, 100.0, 100.0],
            ),
        ];
        let submission = (0..draws.len()).map(submitted).collect::<Vec<_>>();
        let selection = PreviewCastSelection {
            scene_index: 0,
            layer_index: 0,
            node_index: 0,
        };
        let highlights = [
            PreviewHighlightRequest {
                selection,
                kind: crate::renderer::PreviewHighlightKind::Direct,
            },
            PreviewHighlightRequest {
                selection: PreviewCastSelection {
                    scene_index: 0,
                    layer_index: 1,
                    node_index: 0,
                },
                kind: crate::renderer::PreviewHighlightKind::Inherited,
            },
            PreviewHighlightRequest {
                selection: PreviewCastSelection {
                    scene_index: 0,
                    layer_index: 0,
                    node_index: 2,
                },
                kind: crate::renderer::PreviewHighlightKind::Direct,
            },
        ];

        assert_eq!(
            runtime_cast_bounds(
                &selection_project(),
                &plan,
                &draws,
                &submission,
                Some(selection),
                &highlights,
                [100, 100],
            ),
            (
                Some([5, 15, 85, 80]),
                vec![
                    PreviewHighlightBounds {
                        kind: crate::renderer::PreviewHighlightKind::Direct,
                        bounds: [5, 15, 85, 80],
                    },
                    PreviewHighlightBounds {
                        kind: crate::renderer::PreviewHighlightKind::Inherited,
                        bounds: [0, 0, 100, 100],
                    },
                ],
            ),
        );
    }

    #[test]
    fn selection_bounds_ignore_unsubmitted_and_unrelated_draws_then_clip() {
        let selected_layer = ReferenceTarget {
            scene_index: 0,
            layer_index: 0,
        };
        let other_layer = ReferenceTarget {
            scene_index: 0,
            layer_index: 1,
        };
        let draws = vec![
            test_draw(
                ReferenceLayerParent::ProjectLayer(selected_layer),
                0,
                [10.0, 20.0, 120.0, 80.0],
            ),
            test_draw(
                ReferenceLayerParent::ProjectLayer(selected_layer),
                1,
                [-50.0, -50.0, 200.0, 200.0],
            ),
            test_draw(
                ReferenceLayerParent::ProjectLayer(other_layer),
                0,
                [-25.0, -25.0, 125.0, 125.0],
            ),
        ];
        let submission = vec![submitted(0), submitted(2)];

        assert_eq!(
            runtime_cast_bounds(
                &selection_project(),
                &ReferenceRuntimePlan::default(),
                &draws,
                &submission,
                Some(PreviewCastSelection {
                    scene_index: 0,
                    layer_index: 0,
                    node_index: 0,
                }),
                &[],
                [100, 100],
            )
            .0,
            Some([10, 20, 90, 60]),
        );
    }

    #[test]
    fn embedded_camera_replaces_only_the_host_projection_view() {
        let (host, _, _) = preview_host_for_profile(PreviewProfile::CommonBackgroundMain).unwrap();
        let camera = CameraDefinition {
            position: [0.0, 0.0, 1000.0],
            target: [0.0; 3],
            angle_units: 8191,
            near: 10.0,
            far: 100_000.0,
        };
        let mut expected = host;
        expected.target_projection_view = camera.runtime_matrices(1920.0 / 1080.0).projection_view;

        assert_eq!(
            preview_host_with_project_camera(host, camera, [1920, 1080]),
            expected,
        );
    }

    #[test]
    fn missing_camera_zero_fallback_is_applied_without_a_host_matrix_substitute() {
        let (host, _, _) = preview_host_for_profile(PreviewProfile::AdvertiseLogoMain).unwrap();
        let result =
            preview_host_with_project_camera(host, CameraDefinition::ZERO_FALLBACK, [1920, 1080]);

        let expected = CameraDefinition::ZERO_FALLBACK
            .runtime_matrices(1920.0 / 1080.0)
            .projection_view;
        assert_eq!(
            result
                .target_projection_view
                .rows
                .map(|row| row.map(f32::to_bits)),
            expected.rows.map(|row| row.map(f32::to_bits)),
        );
        assert_ne!(
            result
                .target_projection_view
                .rows
                .map(|row| row.map(f32::to_bits)),
            host.target_projection_view
                .rows
                .map(|row| row.map(f32::to_bits)),
        );
    }
}
