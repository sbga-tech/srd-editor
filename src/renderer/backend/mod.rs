//! Backend-neutral preview submission contract and managed GPU resource handles.

use std::collections::BTreeMap;
use std::fmt;

use crate::fennel::FennelRenderVertex;
use crate::renderer::pipeline::{
    DrawTopology, RuntimeCastDraw, RuntimeTargetCommand, RuntimeTargetCommandSource, SrdDrawState,
    SrdVertex, materialize_runtime_fennel_list, materialize_runtime_srd_strip,
};
use crate::renderer::resources::{FennelAtlasSourceSet, SrdTextureSourceSet};

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
pub struct ScissorRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SrdExternalRenderState {
    pub scissor: Option<ScissorRect>,
}

impl SrdExternalRenderState {
    pub const fn without_scissor() -> Self {
        Self { scissor: None }
    }
}

/// Borrowed vertex payload for one Ceylon SimpleShader draw.
pub enum SimpleVertices<'a> {
    Format14(&'a [SrdVertex]),
    Format13(&'a [FennelRenderVertex]),
}

/// GPU texture source selected for one Ceylon SimpleShader draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimpleTextureSource {
    Srd(Option<SrdTextureSetHandle>),
    Fennel {
        atlas: FennelAtlasHandle,
        page_index: usize,
    },
}

/// Complete borrowed submission for one pipeline-specialized SimpleShader draw.
pub struct SimpleDraw<'a> {
    pub state: SrdDrawState,
    pub topology: DrawTopology,
    pub vertices: SimpleVertices<'a>,
    pub textures: SimpleTextureSource,
    pub external: SrdExternalRenderState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositionReadback {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Minimal platform-neutral submission boundary used by the editor's retained
/// SRD planner. Asset parsing and draw construction stay outside the backend;
/// implementations only own GPU resources and translate completed draws.
pub trait SrdRenderBackend {
    fn upload_srd_textures(
        &mut self,
        sources: SrdTextureSourceSet,
    ) -> Result<SrdTextureSetHandle, RenderBackendError>;

    fn upload_fennel_atlas(
        &mut self,
        sources: FennelAtlasSourceSet,
    ) -> Result<FennelAtlasHandle, RenderBackendError>;

    fn configure_composition_target(&mut self, size: [u32; 2]) -> Result<(), RenderBackendError>;

    fn composition_size(&self) -> Option<[u32; 2]>;

    fn begin_composition(&mut self, clear_argb: u32) -> Result<(), RenderBackendError>;

    fn end_composition(&mut self) -> Result<(), RenderBackendError>;

    fn read_composition(&self) -> Result<CompositionReadback, RenderBackendError>;

    fn render_simple(&mut self, draw: SimpleDraw<'_>) -> Result<(), RenderBackendError>;
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
    pub draws: &'a [RuntimeCastDraw],
    pub submission: &'a [RuntimeTargetCommand],
    pub textures: Option<SrdTextureSetHandle>,
    pub fennel_atlas_routes: &'a BTreeMap<u32, FennelAtlasRoute>,
}

pub fn render_preview_runtime_layers(
    backend: &mut (impl SrdRenderBackend + ?Sized),
    common_background: Option<RuntimeRenderLayer<'_>>,
    foreground: RuntimeRenderLayer<'_>,
    external: SrdExternalRenderState,
    render_fennel: bool,
    render_number: bool,
) -> Result<(), RenderBackendError> {
    if let Some(background) = common_background {
        render_runtime_target_submission(
            backend,
            background,
            external,
            render_fennel,
            render_number,
        )?;
    }
    render_runtime_target_submission(backend, foreground, external, render_fennel, render_number)?;
    if std::env::var_os("SRD_DOUBLE_PASS_PROBE").is_some() {
        render_runtime_target_submission(
            backend,
            foreground,
            external,
            render_fennel,
            render_number,
        )?;
    }
    Ok(())
}

pub fn render_runtime_target_submission(
    backend: &mut (impl SrdRenderBackend + ?Sized),
    layer: RuntimeRenderLayer<'_>,
    external: SrdExternalRenderState,
    render_fennel: bool,
    render_number: bool,
) -> Result<(), RenderBackendError> {
    for command in layer.submission {
        match command.topology {
            DrawTopology::TriangleStrip => {
                if let Some(strip) = materialize_runtime_srd_strip(layer.draws, command, |source| {
                    render_number
                        || !matches!(source, RuntimeTargetCommandSource::NumberGlyph { .. })
                })
                .map_err(|error| RenderBackendError(error.to_string()))?
                {
                    backend.render_simple(SimpleDraw {
                        state: strip.state.state,
                        topology: command.topology,
                        vertices: SimpleVertices::Format14(&strip.vertices),
                        textures: SimpleTextureSource::Srd(layer.textures),
                        external,
                    })?;
                }
            }
            DrawTopology::TriangleList => {
                if !render_fennel {
                    continue;
                }
                let list = materialize_runtime_fennel_list(layer.draws, command)
                    .map_err(|error| RenderBackendError(error.to_string()))?;
                let route = require_atlas_route(layer.fennel_atlas_routes, list.texture_token)?;
                backend.render_simple(SimpleDraw {
                    state: list.state.state,
                    topology: command.topology,
                    vertices: SimpleVertices::Format13(&list.vertices),
                    textures: SimpleTextureSource::Fennel {
                        atlas: route.atlas,
                        page_index: route.page_index,
                    },
                    external,
                })?;
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct CompositionLifecycleBackend {
        begin_values: Vec<u32>,
        end_calls: usize,
        end_error: Option<RenderBackendError>,
    }

    impl SrdRenderBackend for CompositionLifecycleBackend {
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

        fn render_simple(&mut self, _draw: SimpleDraw<'_>) -> Result<(), RenderBackendError> {
            Err(RenderBackendError(
                "SimpleShader submission is outside the composition lifecycle contract".to_string(),
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
        let state = SrdDrawState::fennel(
            crate::renderer::pipeline::SrdTransform::identity_2d([1, 1]),
            crate::renderer::pipeline::SimpleTransformMode::TwoDimensional,
        );
        let background_submission = [RuntimeTargetCommand {
            sources: vec![RuntimeTargetCommandSource::Image {
                runtime_draw_index: 1,
            }],
            state,
            order: crate::renderer::pipeline::DrawOrder {
                renderer_layer_key: 0,
            },
            topology: DrawTopology::TriangleStrip,
            vertex_count: 0,
        }];
        let foreground_submission = [RuntimeTargetCommand {
            sources: vec![RuntimeTargetCommandSource::Image {
                runtime_draw_index: 2,
            }],
            state,
            order: crate::renderer::pipeline::DrawOrder {
                renderer_layer_key: 0,
            },
            topology: DrawTopology::TriangleStrip,
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
        )
        .unwrap_err();

        assert_eq!(
            error.0,
            "runtime target Image source 1 does not match its draw"
        );
    }
}
