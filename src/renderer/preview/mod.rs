//! Backend-agnostic preview orchestration and resource lifetime management.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::camera::CameraDefinition;
use crate::document::EditorDocument;
use crate::fennel::FennelFontSlotRegistry;
use crate::game_host::{
    CHUSAN_ADVERTISE_LOGO_PLAYER, CHUSAN_BG_SCENE, CHUSAN_COMMON_BACKGROUND_PLAYER,
    CHUSAN_LINKED_VERSE_GATE_PLAYER, CHUSAN_MAIN_SCENE, ChusanAirSceneTargetProfile, WorldSnapshot,
};
use crate::projection::Matrix4x4;
use crate::reference_runtime::{ReferenceLayerParent, ReferenceRuntimePlan};
use crate::renderer::backend::{
    FennelAtlasHandle, FennelAtlasRoute, RuntimeRenderLayer, SrdExternalRenderState,
    SrdRenderBackend, SrdTextureSetHandle, render_preview_runtime_layers,
    render_runtime_target_submission, render_to_composition,
};
use crate::renderer::resources::{FennelAtlasSourceSet, SrdTextureSourceSet};
use crate::renderer::{
    FennelFontResourceAssignment, FennelRenderResources, RuntimeCastDraw, RuntimeTargetCommand,
    RuntimeTargetCommandSource, SrdDraw, assign_fennel_font_resource_requests,
    build_animation_assignment_runtime_cast_draws, build_base_pose_runtime_cast_draws,
    build_runtime_target_commands, collect_fennel_font_resource_requests,
};
use crate::ruhuna::{RuhunaFont, RuhunaRuntimeFont};
use crate::target_pass::SrdType1TargetFilter;

use super::{
    PreviewCastSelection, PreviewFrame, PreviewHighlightBounds, PreviewHighlightKind,
    PreviewHighlightRequest, PreviewLayerRequest, PreviewProfile, PreviewRequest,
};
mod resources;
use resources::{
    PreviewDocumentResources, ensure_fennel_resources, ensure_textures, prepare_document_resources,
};
mod selection;
use selection::{
    PreviewVisibilityFilter, filter_editor_preview_draws, runtime_cast_bounds, runtime_srd_draw,
};

const CLEAR_ARGB: u32 = 0xFF00_0000;
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

struct PreparedRuntimePreviewLayer {
    draws: Vec<RuntimeCastDraw>,
    submission: Vec<RuntimeTargetCommand>,
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
            preview_snapshot_for_profile(request.profile)?;
        let foreground_host = preview_snapshot_with_project_camera(
            foreground_host,
            foreground_document.project.camera,
            composition_size,
        );
        let (foreground_frame_fraction, trailing_frame_fraction) = preview_animation_frame_samples(
            request.profile,
            foreground_request.animation_assignment.is_some(),
        );
        let foreground = prepare_runtime_preview_layer(
            &mut self.backend,
            &mut self.foreground_resources,
            foreground_document,
            foreground_request,
            foreground_host,
            target,
            foreground_filter,
            foreground_frame_fraction,
        )?;
        let trailing_foreground = trailing_frame_fraction
            .map(|frame_fraction| {
                prepare_runtime_preview_layer(
                    &mut self.backend,
                    &mut self.foreground_resources,
                    foreground_document,
                    foreground_request,
                    foreground_host,
                    target,
                    foreground_filter,
                    frame_fraction,
                )
            })
            .transpose()?;
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
                let filter = CHUSAN_COMMON_BACKGROUND_PLAYER.initial_srd_target_filter(target);
                let host = preview_snapshot_with_project_camera(
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
                    filter,
                    0.0,
                )
            })
            .transpose()?;
        let foreground_bounds_sources = [
            Some((
                foreground.draws.as_slice(),
                foreground.submission.as_slice(),
            )),
            trailing_foreground
                .as_ref()
                .map(|layer| (layer.draws.as_slice(), layer.submission.as_slice())),
        ];
        let (selection_bounds, highlight_bounds) = runtime_cast_bounds(
            &foreground_document.project,
            &foreground.reference_plan,
            foreground_bounds_sources.into_iter().flatten(),
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
        let trailing_foreground_layer =
            trailing_foreground
                .as_ref()
                .map(|foreground| RuntimeRenderLayer {
                    draws: &foreground.draws,
                    submission: &foreground.submission,
                    textures: self.foreground_resources.textures,
                    fennel_atlas_routes: &self.foreground_resources.fennel_atlas_routes,
                });
        let background_layer = background.as_ref().map(|background| RuntimeRenderLayer {
            draws: &background.draws,
            submission: &background.submission,
            textures: self.background_resources.textures,
            fennel_atlas_routes: &self.background_resources.fennel_atlas_routes,
        });

        phase(GpuRenderPhase::Begin)?;
        let render_result = render_to_composition(&mut self.backend, CLEAR_ARGB, |backend| {
            let external = SrdExternalRenderState::without_scissor();
            render_preview_runtime_layers(
                backend,
                background_layer,
                foreground_layer,
                external,
                true,
                true,
            )?;
            if let Some(trailing_foreground_layer) = trailing_foreground_layer {
                render_runtime_target_submission(
                    backend,
                    trailing_foreground_layer,
                    external,
                    true,
                    true,
                )?;
            }
            Ok(())
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

fn preview_animation_frame_samples(profile: PreviewProfile, animated: bool) -> (f32, Option<f32>) {
    if animated && profile == PreviewProfile::AdvertiseLogoMain {
        // Native MainScene output retains two consecutive AdvertiseLogo
        // samples on one presented target. Matched animation-state probes at
        // N + 0.5 and N + 1.0 reproduce the observed target-color feedback
        // and ordinary alpha-blended edges; either sample alone does not.
        (0.5, Some(1.0))
    } else {
        (0.0, None)
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

fn preview_snapshot_with_project_camera(
    mut snapshot: WorldSnapshot,
    camera: CameraDefinition,
    composition_size: [u32; 2],
) -> WorldSnapshot {
    let aspect = (composition_size[0] as f32) / (composition_size[1] as f32);
    snapshot.target_projection_view = camera.runtime_matrices(aspect).projection_view;
    snapshot
}

fn preview_snapshot_for_profile(
    profile: PreviewProfile,
) -> Result<
    (
        WorldSnapshot,
        SrdType1TargetFilter,
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
    snapshot: WorldSnapshot,
    target: ChusanAirSceneTargetProfile,
    filter: SrdType1TargetFilter,
    frame_fraction: f32,
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
    let fennel = FennelRenderResources {
        font_slots: &font_registry,
        fonts: &resources.runtime_fonts,
        force_color_update: false,
        text_inputs: request.runtime_text_inputs,
    };
    let frame = request.frame as f32 + frame_fraction;
    let draws = if let Some(animation_assignment) = request.animation_assignment {
        build_animation_assignment_runtime_cast_draws(
            &document.project,
            &document.textures,
            request.scene_index,
            animation_assignment,
            frame,
            snapshot,
            fennel,
        )
    } else {
        build_base_pose_runtime_cast_draws(
            &document.project,
            &document.textures,
            request.scene_index,
            snapshot,
            fennel,
        )
    }
    .map_err(|error| format!("failed to evaluate Surfride runtime draws: {error}"))?;
    let draws = filter_editor_preview_draws(
        draws,
        &document.project,
        &reference_plan,
        PreviewVisibilityFilter {
            selected_scene_index: request.scene_index,
            hidden_layers: request.hidden_layers,
            solo_layers: request.solo_layers,
            hidden_casts: request.hidden_casts,
            solo_casts: request.solo_casts,
        },
    )?;
    let target_profile = target
        .scene_pass_profile()
        .map_err(|error| error.to_string())?;
    let submission = build_runtime_target_commands(&draws, &target_profile, filter)
        .map_err(|error| format!("failed to build Ceylon target submission: {error}"))?;
    let required_indices = draws
        .iter()
        .filter_map(runtime_srd_draw)
        .flat_map(|draw| draw.state.material.textures.iter().flatten())
        .map(|binding| binding.texture_index)
        .collect::<BTreeSet<_>>();
    ensure_textures(backend, resources, document, &required_indices)?;
    Ok(PreparedRuntimePreviewLayer {
        draws,
        submission,
        reference_plan,
    })
}

fn checked_scene_size(width: f32, height: f32) -> Result<[u32; 2], String> {
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return Err(format!(
            "scene composition size is invalid: {width} × {height}"
        ));
    }
    if width.fract() != 0.0 || height.fract() != 0.0 {
        return Err(format!(
            "fractional scene composition sizes are unsupported: {width} × {height}"
        ));
    }
    if width > u32::MAX as f32 || height > u32::MAX as f32 {
        return Err(format!(
            "scene composition size exceeds renderer dimensions: {width} × {height}"
        ));
    }
    Ok([width as u32, height as u32])
}

#[cfg(test)]
mod tests;
