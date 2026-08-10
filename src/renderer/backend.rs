use std::collections::BTreeMap;
use std::fmt;

use crate::fennel::FennelRenderVertex;
use crate::render::{
    CeylonAlphaStencilState, CeylonRenderScissorState, CeylonSrdFixedShaderConstants, D3d9Rect,
    SrdRenderVertex,
};
use crate::renderer::assets::{FennelAtlasSourceSet, SrdTextureSourceSet};
use crate::srd_draw::{
    EvidenceCompleteFennelDraw, EvidenceCompleteRuntimeCastDraw, EvidenceCompleteSrdDraw,
    EvidenceMergedRuntimeTargetCommand, EvidenceRuntimeTargetCommandSource,
    build_evidence_merged_runtime_fennel_list, build_evidence_merged_runtime_srd_strip,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderBackendError(pub String);

impl fmt::Display for RenderBackendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for RenderBackendError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SrdTextureSetHandle(u64);

impl SrdTextureSetHandle {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FennelAtlasHandle(u64);

impl FennelAtlasHandle {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SrdExternalRenderState {
    pub scissor: CeylonRenderScissorState,
    pub alpha_stencil: CeylonAlphaStencilState,
}

impl SrdExternalRenderState {
    /// Explicit host input stating that the external material scissor is
    /// disabled. Callers must choose this state; it is not inferred from SRD.
    pub const fn without_scissor() -> Self {
        Self {
            scissor: CeylonRenderScissorState {
                enabled: false,
                rectangle: D3d9Rect {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                },
            },
            alpha_stencil: CeylonAlphaStencilState::default_material(),
        }
    }

    /// Explicit context used by standalone smoke harnesses. This is not
    /// inferred from SRD and is not claimed to be every game caller's state.
    pub const fn smoke_without_scissor() -> Self {
        Self::without_scissor()
    }
}

/// One already-laid-out Fennel texture batch. Backends submit one triangle
/// list per texture token, with six vertices per glyph.
pub struct FennelRenderBatch<'a> {
    pub page_index: usize,
    pub is_2d: bool,
    pub fixed_constants: CeylonSrdFixedShaderConstants,
    pub vertices: &'a [FennelRenderVertex],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositionReadback {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl CompositionReadback {
    pub fn from_bgra(width: u32, height: u32, mut bgra: Vec<u8>) -> Self {
        for pixel in bgra.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
        Self {
            width,
            height,
            rgba: bgra,
        }
    }
}

/// Minimal platform-neutral submission boundary used by the editor's retained
/// SRD planner. Asset parsing and draw construction stay outside the backend;
/// implementations only own GPU resources and translate completed draws.
pub trait SrdRenderBackend {
    fn name(&self) -> &'static str;

    fn upload_srd_textures(
        &mut self,
        sources: SrdTextureSourceSet,
    ) -> Result<SrdTextureSetHandle, RenderBackendError>;

    fn upload_fennel_atlas(
        &mut self,
        sources: FennelAtlasSourceSet,
    ) -> Result<FennelAtlasHandle, RenderBackendError>;

    fn clear_uploaded_resources(&mut self);

    fn configure_composition_target(&mut self, size: [u32; 2]) -> Result<(), RenderBackendError>;

    fn composition_size(&self) -> Option<[u32; 2]>;

    fn begin_composition(&mut self, clear_argb: u32) -> Result<(), RenderBackendError>;

    fn end_composition(&mut self) -> Result<(), RenderBackendError>;

    fn read_composition(&self) -> Result<CompositionReadback, RenderBackendError>;

    fn render_srd(
        &mut self,
        draws: &[EvidenceCompleteSrdDraw],
        external: SrdExternalRenderState,
        textures: Option<SrdTextureSetHandle>,
    ) -> Result<(), RenderBackendError>;

    fn render_srd_triangle_strip(
        &mut self,
        state: &EvidenceCompleteSrdDraw,
        vertices: &[SrdRenderVertex],
        external: SrdExternalRenderState,
        textures: Option<SrdTextureSetHandle>,
    ) -> Result<(), RenderBackendError>;

    fn render_fennel(
        &mut self,
        batches: &[FennelRenderBatch<'_>],
        external: SrdExternalRenderState,
        atlas: FennelAtlasHandle,
    ) -> Result<(), RenderBackendError>;
}

pub fn render_to_composition<B, F>(
    backend: &mut B,
    clear_argb: u32,
    render: F,
) -> Result<(), RenderBackendError>
where
    B: SrdRenderBackend + ?Sized,
    F: FnOnce(&mut B) -> Result<(), RenderBackendError>,
{
    backend.begin_composition(clear_argb)?;
    let render_result = render(backend);
    let end_result = backend.end_composition();
    match (render_result, end_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Err(render_error), Err(end_error)) => Err(RenderBackendError(format!(
            "{render_error}; additionally failed to restore the composition target: {end_error}"
        ))),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FennelAtlasRoute {
    pub atlas: FennelAtlasHandle,
    pub page_index: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct RuntimeRenderLayer<'a> {
    pub draws: &'a [EvidenceCompleteRuntimeCastDraw],
    pub submission: &'a [EvidenceMergedRuntimeTargetCommand],
    pub textures: Option<SrdTextureSetHandle>,
    pub fennel_atlas_routes: &'a BTreeMap<u32, FennelAtlasRoute>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSrdSourceFilter {
    All,
    AlphaTestOnly,
    DualTextureOnly,
}

impl RuntimeSrdSourceFilter {
    fn includes(
        self,
        draws: &[EvidenceCompleteRuntimeCastDraw],
        source: EvidenceRuntimeTargetCommandSource,
    ) -> bool {
        match self {
            Self::All => true,
            Self::AlphaTestOnly => runtime_source_is_alpha_test(draws, source),
            Self::DualTextureOnly => runtime_source_texture_mask(draws, source) == 0b011,
        }
    }
}

pub fn count_runtime_alpha_test_sources(
    draws: &[EvidenceCompleteRuntimeCastDraw],
    submission: &[EvidenceMergedRuntimeTargetCommand],
) -> usize {
    submission
        .iter()
        .flat_map(|command| command.sources.iter().copied())
        .filter(|source| runtime_source_is_alpha_test(draws, *source))
        .count()
}

pub fn count_runtime_dual_texture_sources(
    draws: &[EvidenceCompleteRuntimeCastDraw],
    submission: &[EvidenceMergedRuntimeTargetCommand],
) -> usize {
    submission
        .iter()
        .flat_map(|command| command.sources.iter().copied())
        .filter(|source| runtime_source_texture_mask(draws, *source) == 0b011)
        .count()
}

pub fn render_preview_runtime_layers(
    backend: &mut (impl SrdRenderBackend + ?Sized),
    common_background: Option<RuntimeRenderLayer<'_>>,
    foreground: RuntimeRenderLayer<'_>,
    external: SrdExternalRenderState,
    render_fennel: bool,
    render_number: bool,
    source_filter: RuntimeSrdSourceFilter,
) -> Result<(), RenderBackendError> {
    if let Some(background) = common_background {
        render_runtime_target_submission(
            backend,
            background,
            external,
            render_fennel,
            render_number,
            source_filter,
        )?;
    }
    render_runtime_target_submission(
        backend,
        foreground,
        external,
        render_fennel,
        render_number,
        source_filter,
    )
}

pub fn render_runtime_target_submission(
    backend: &mut (impl SrdRenderBackend + ?Sized),
    layer: RuntimeRenderLayer<'_>,
    external: SrdExternalRenderState,
    render_fennel: bool,
    render_number: bool,
    source_filter: RuntimeSrdSourceFilter,
) -> Result<(), RenderBackendError> {
    for command in layer.submission {
        if command.vertex_format == 14 && command.primitive_type == 4 {
            if let Some(strip) =
                build_evidence_merged_runtime_srd_strip(layer.draws, command, |source| {
                    (render_number
                        || !matches!(
                            source,
                            EvidenceRuntimeTargetCommandSource::NumberGlyph { .. }
                        ))
                        && source_filter.includes(layer.draws, source)
                })
                .map_err(|error| RenderBackendError(error.to_string()))?
            {
                backend.render_srd_triangle_strip(
                    strip.state,
                    &strip.vertices,
                    external,
                    layer.textures,
                )?;
            }
            continue;
        }
        if command.vertex_format == 13 && command.primitive_type == 3 {
            if !render_fennel {
                continue;
            }
            let list = build_evidence_merged_runtime_fennel_list(layer.draws, command)
                .map_err(|error| RenderBackendError(error.to_string()))?;
            let route = require_atlas_route(layer.fennel_atlas_routes, list.texture_token)?;
            let batch = FennelRenderBatch {
                page_index: route.page_index,
                is_2d: list.state.is_2d,
                fixed_constants: list.state.fixed_constants,
                vertices: &list.vertices,
            };
            backend.render_fennel(std::slice::from_ref(&batch), external, route.atlas)?;
            continue;
        }
        for source in &command.sources {
            if !source_filter.includes(layer.draws, *source) {
                continue;
            }
            match *source {
                EvidenceRuntimeTargetCommandSource::Image { runtime_draw_index } => {
                    let Some(EvidenceCompleteRuntimeCastDraw::Image(draw)) =
                        layer.draws.get(runtime_draw_index)
                    else {
                        return Err(RenderBackendError(
                            "runtime target Image source does not match its draw".to_string(),
                        ));
                    };
                    backend.render_srd(std::slice::from_ref(draw), external, layer.textures)?;
                }
                EvidenceRuntimeTargetCommandSource::SliceCell { runtime_draw_index } => {
                    let Some(EvidenceCompleteRuntimeCastDraw::SliceCell(cell)) =
                        layer.draws.get(runtime_draw_index)
                    else {
                        return Err(RenderBackendError(
                            "runtime target SliceCell source does not match its draw".to_string(),
                        ));
                    };
                    backend.render_srd(
                        std::slice::from_ref(&cell.draw),
                        external,
                        layer.textures,
                    )?;
                }
                EvidenceRuntimeTargetCommandSource::NumberGlyph { runtime_draw_index } => {
                    if !render_number {
                        continue;
                    }
                    let Some(EvidenceCompleteRuntimeCastDraw::NumberGlyph(glyph)) =
                        layer.draws.get(runtime_draw_index)
                    else {
                        return Err(RenderBackendError(
                            "runtime target NumberGlyph source does not match its draw".to_string(),
                        ));
                    };
                    backend.render_srd(
                        std::slice::from_ref(&glyph.draw),
                        external,
                        layer.textures,
                    )?;
                }
                EvidenceRuntimeTargetCommandSource::FennelBatch {
                    runtime_draw_index,
                    batch_index,
                } => {
                    if !render_fennel {
                        continue;
                    }
                    let Some(EvidenceCompleteRuntimeCastDraw::Fennel(draw)) =
                        layer.draws.get(runtime_draw_index)
                    else {
                        return Err(RenderBackendError(
                            "runtime target Fennel source does not match its draw".to_string(),
                        ));
                    };
                    let batch = draw.batches.get(batch_index).ok_or_else(|| {
                        RenderBackendError(
                            "runtime target Fennel batch index is outside its draw".to_string(),
                        )
                    })?;
                    let route =
                        require_atlas_route(layer.fennel_atlas_routes, batch.texture_token)?;
                    let routed_batch = FennelRenderBatch {
                        page_index: route.page_index,
                        is_2d: draw.is_2d,
                        fixed_constants: draw.fixed_constants,
                        vertices: &batch.vertices,
                    };
                    backend.render_fennel(
                        std::slice::from_ref(&routed_batch),
                        external,
                        route.atlas,
                    )?;
                }
            }
        }
    }
    Ok(())
}

pub fn render_fennel_draws(
    backend: &mut (impl SrdRenderBackend + ?Sized),
    draws: &[EvidenceCompleteFennelDraw],
    atlas_routes: &BTreeMap<u32, FennelAtlasRoute>,
    external: SrdExternalRenderState,
) -> Result<(), RenderBackendError> {
    for draw in draws {
        for batch in &draw.batches {
            let route = require_atlas_route(atlas_routes, batch.texture_token)?;
            let routed_batch = FennelRenderBatch {
                page_index: route.page_index,
                is_2d: draw.is_2d,
                fixed_constants: draw.fixed_constants,
                vertices: &batch.vertices,
            };
            backend.render_fennel(std::slice::from_ref(&routed_batch), external, route.atlas)?;
        }
    }
    Ok(())
}

fn require_atlas_route(
    routes: &BTreeMap<u32, FennelAtlasRoute>,
    texture_token: u32,
) -> Result<FennelAtlasRoute, RenderBackendError> {
    routes.get(&texture_token).copied().ok_or_else(|| {
        RenderBackendError(format!(
            "Fennel texture token {texture_token:#010x} has no atlas route"
        ))
    })
}

fn runtime_source_is_alpha_test(
    draws: &[EvidenceCompleteRuntimeCastDraw],
    source: EvidenceRuntimeTargetCommandSource,
) -> bool {
    match source {
        EvidenceRuntimeTargetCommandSource::Image { runtime_draw_index } => {
            matches!(
                &draws[runtime_draw_index],
                EvidenceCompleteRuntimeCastDraw::Image(draw) if draw.blend.alpha_test_enabled
            )
        }
        EvidenceRuntimeTargetCommandSource::SliceCell { runtime_draw_index } => {
            matches!(
                &draws[runtime_draw_index],
                EvidenceCompleteRuntimeCastDraw::SliceCell(cell)
                    if cell.draw.blend.alpha_test_enabled
            )
        }
        EvidenceRuntimeTargetCommandSource::NumberGlyph { runtime_draw_index } => {
            matches!(
                &draws[runtime_draw_index],
                EvidenceCompleteRuntimeCastDraw::NumberGlyph(glyph)
                    if glyph.draw.blend.alpha_test_enabled
            )
        }
        EvidenceRuntimeTargetCommandSource::FennelBatch { .. } => false,
    }
}

fn runtime_source_texture_mask(
    draws: &[EvidenceCompleteRuntimeCastDraw],
    source: EvidenceRuntimeTargetCommandSource,
) -> u8 {
    let state = match source {
        EvidenceRuntimeTargetCommandSource::Image { runtime_draw_index } => {
            let EvidenceCompleteRuntimeCastDraw::Image(draw) = &draws[runtime_draw_index] else {
                return 0;
            };
            draw
        }
        EvidenceRuntimeTargetCommandSource::SliceCell { runtime_draw_index } => {
            let EvidenceCompleteRuntimeCastDraw::SliceCell(cell) = &draws[runtime_draw_index]
            else {
                return 0;
            };
            &cell.draw
        }
        EvidenceRuntimeTargetCommandSource::NumberGlyph { runtime_draw_index } => {
            let EvidenceCompleteRuntimeCastDraw::NumberGlyph(glyph) = &draws[runtime_draw_index]
            else {
                return 0;
            };
            &glyph.draw
        }
        EvidenceRuntimeTargetCommandSource::FennelBatch { .. } => return 0,
    };
    state
        .texture_bindings
        .iter()
        .enumerate()
        .fold(0u8, |mask, (slot, binding)| {
            mask | (u8::from(binding.is_some()) << slot)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::CeylonDrawPacketPresetState;

    #[derive(Default)]
    struct CompositionLifecycleBackend {
        begin_values: Vec<u32>,
        end_calls: usize,
        end_error: Option<RenderBackendError>,
    }

    impl SrdRenderBackend for CompositionLifecycleBackend {
        fn name(&self) -> &'static str {
            "composition-lifecycle-test"
        }

        fn upload_srd_textures(
            &mut self,
            _sources: SrdTextureSourceSet,
        ) -> Result<SrdTextureSetHandle, RenderBackendError> {
            Err(RenderBackendError(
                "texture upload is outside the composition lifecycle contract".to_string(),
            ))
        }

        fn upload_fennel_atlas(
            &mut self,
            _sources: FennelAtlasSourceSet,
        ) -> Result<FennelAtlasHandle, RenderBackendError> {
            Err(RenderBackendError(
                "atlas upload is outside the composition lifecycle contract".to_string(),
            ))
        }

        fn clear_uploaded_resources(&mut self) {}

        fn configure_composition_target(
            &mut self,
            _size: [u32; 2],
        ) -> Result<(), RenderBackendError> {
            Ok(())
        }

        fn composition_size(&self) -> Option<[u32; 2]> {
            None
        }

        fn begin_composition(&mut self, clear_argb: u32) -> Result<(), RenderBackendError> {
            self.begin_values.push(clear_argb);
            Ok(())
        }

        fn end_composition(&mut self) -> Result<(), RenderBackendError> {
            self.end_calls += 1;
            self.end_error.clone().map_or(Ok(()), Err)
        }

        fn read_composition(&self) -> Result<CompositionReadback, RenderBackendError> {
            Err(RenderBackendError(
                "readback is outside the composition lifecycle contract".to_string(),
            ))
        }

        fn render_srd(
            &mut self,
            _draws: &[EvidenceCompleteSrdDraw],
            _external: SrdExternalRenderState,
            _textures: Option<SrdTextureSetHandle>,
        ) -> Result<(), RenderBackendError> {
            Err(RenderBackendError(
                "SRD submission is outside the composition lifecycle contract".to_string(),
            ))
        }

        fn render_srd_triangle_strip(
            &mut self,
            _state: &EvidenceCompleteSrdDraw,
            _vertices: &[SrdRenderVertex],
            _external: SrdExternalRenderState,
            _textures: Option<SrdTextureSetHandle>,
        ) -> Result<(), RenderBackendError> {
            Err(RenderBackendError(
                "SRD strip submission is outside the composition lifecycle contract".to_string(),
            ))
        }

        fn render_fennel(
            &mut self,
            _batches: &[FennelRenderBatch<'_>],
            _external: SrdExternalRenderState,
            _atlas: FennelAtlasHandle,
        ) -> Result<(), RenderBackendError> {
            Err(RenderBackendError(
                "Fennel submission is outside the composition lifecycle contract".to_string(),
            ))
        }
    }

    #[test]
    fn composition_target_is_restored_after_draw_failure() {
        let mut backend = CompositionLifecycleBackend::default();
        let error = {
            let erased: &mut dyn SrdRenderBackend = &mut backend;
            render_to_composition(erased, 0x1122_3344, |_backend| {
                Err(RenderBackendError("draw failed".to_string()))
            })
            .unwrap_err()
        };

        assert_eq!(error.0, "draw failed");
        assert_eq!(backend.begin_values, [0x1122_3344]);
        assert_eq!(backend.end_calls, 1);
    }

    #[test]
    fn composition_reports_draw_and_restore_failures() {
        let mut backend = CompositionLifecycleBackend {
            end_error: Some(RenderBackendError("restore failed".to_string())),
            ..CompositionLifecycleBackend::default()
        };
        let error = render_to_composition(&mut backend, 0, |_backend| {
            Err(RenderBackendError("draw failed".to_string()))
        })
        .unwrap_err();

        assert_eq!(
            error.0,
            "draw failed; additionally failed to restore the composition target: restore failed"
        );
        assert_eq!(backend.end_calls, 1);
    }

    #[test]
    fn preview_layers_submit_background_before_foreground() {
        let background_submission = [EvidenceMergedRuntimeTargetCommand {
            sources: vec![EvidenceRuntimeTargetCommandSource::Image {
                runtime_draw_index: 1,
            }],
            packet: CeylonDrawPacketPresetState::default(),
            renderer_layer_key: 0,
            vertex_format: 14,
            primitive_type: 4,
            vertex_count: 0,
        }];
        let foreground_submission = [EvidenceMergedRuntimeTargetCommand {
            sources: vec![EvidenceRuntimeTargetCommandSource::Image {
                runtime_draw_index: 2,
            }],
            packet: CeylonDrawPacketPresetState::default(),
            renderer_layer_key: 0,
            vertex_format: 14,
            primitive_type: 4,
            vertex_count: 0,
        }];
        let atlas_routes = BTreeMap::new();
        let background = RuntimeRenderLayer {
            draws: &[],
            submission: &background_submission,
            textures: None,
            fennel_atlas_routes: &atlas_routes,
        };
        let foreground = RuntimeRenderLayer {
            draws: &[],
            submission: &foreground_submission,
            textures: None,
            fennel_atlas_routes: &atlas_routes,
        };

        let error = render_preview_runtime_layers(
            &mut CompositionLifecycleBackend::default(),
            Some(background),
            foreground,
            SrdExternalRenderState::without_scissor(),
            false,
            true,
            RuntimeSrdSourceFilter::All,
        )
        .unwrap_err();

        assert_eq!(
            error.0,
            "runtime target Image source 1 does not match its draw"
        );
    }
}
