//! Public rendering facade.
//!
//! Backend-neutral draw compilation lives in `pipeline`, preview orchestration in `preview`,
//! and native GPU ownership behind the `backend` contract. Those modules stay private so editor
//! code consumes one rendering vocabulary instead of backend implementation details.

use std::collections::{BTreeMap, BTreeSet};

use crate::document::EditorDocument;
use crate::scene::AnimationSetDefinition;

mod backend;
mod pipeline;
pub use pipeline::{
    FennelDraw, FennelFontResourceAssignment, FennelFontResourceRequest, FennelRenderResources,
    FennelRuntimeTextCastKey, FennelSrdRuntimeTextInput, FennelTextFontRole, NumberGlyphDraw,
    RuntimeCastDraw, RuntimeFennelList, RuntimeSrdStrip, RuntimeTargetCommand,
    RuntimeTargetCommandSource, SliceCellDraw, SrdDraw, SrdDrawError, SrdTextureBinding,
    assign_fennel_font_resource_requests, build_animation_assignment_runtime_cast_draws,
    build_animation_set_image_draws, build_animation_set_reference_image_draws,
    build_animation_set_runtime_cast_draws, build_animation_set_runtime_srd_draws,
    build_base_pose_fennel_draws, build_base_pose_image_draws,
    build_base_pose_reference_fennel_draws, build_base_pose_reference_image_draws,
    build_base_pose_runtime_cast_draws, build_filtered_runtime_target_submission,
    build_runtime_srd_draws_from_runtime, build_runtime_target_commands,
    build_runtime_target_submission, collect_fennel_font_resource_requests,
    materialize_runtime_fennel_list, materialize_runtime_srd_strip, select_srd_image_render_preset,
};
mod preview;
mod resources;
mod wgpu;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PreviewProfile {
    AdvertiseLogoMain,
    CommonBackgroundMain,
    LinkedVerseGateMain,
    AdvertiseLogoBackground,
}

/// One SRD composition layer of a preview submission. Foreground and
/// background carry fully independent documents, revisions, scene/optional
/// animation assignment/frame selections, layer visibility, and SrTextCast
/// inputs.
pub struct PreviewLayerRequest<'a> {
    pub document: &'a EditorDocument,
    pub document_revision: u64,
    pub scene_index: usize,
    /// `None` renders the serialized Base Pose without applying an ANMS
    /// assignment. A present assignment may be a scratch definition that is
    /// not stored in the scene.
    pub animation_assignment: Option<&'a AnimationSetDefinition>,
    pub frame: i32,
    pub hidden_layers: &'a BTreeSet<usize>,
    /// Editor-only layer solo selection. When nonempty, only draws rooted in
    /// one of these scene layers are retained.
    pub solo_layers: &'a BTreeSet<usize>,
    /// Editor-only CAST visibility overrides. A hidden CAST suppresses its
    /// source-hierarchy and reference-instance descendants.
    pub hidden_casts: &'a BTreeSet<PreviewCastSelection>,
    /// Editor-only CAST solo selection. When nonempty, the union of its
    /// source-hierarchy and reference-instance subtrees is retained.
    pub solo_casts: &'a BTreeSet<PreviewCastSelection>,
    /// Explicit SrTextCast host inputs. Absent entries keep the strict initial
    /// behavior and reject `$[0]..$[7]` rather than inventing values.
    pub runtime_text_inputs: &'a BTreeMap<FennelRuntimeTextCastKey, FennelSrdRuntimeTextInput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PreviewCastSelection {
    pub scene_index: usize,
    pub layer_index: usize,
    pub node_index: usize,
}

/// Identifies how the editor presents a cast highlight. Geometry selection is
/// otherwise identical: the selected CAST and its emitted descendants are
/// included for both kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PreviewHighlightKind {
    Direct,
    Inherited,
}

/// Requests bounds for a CAST with the supplied presentation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewHighlightRequest {
    pub selection: PreviewCastSelection,
    pub kind: PreviewHighlightKind,
}

/// Pixel-space emitted geometry bounds for one requested highlight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewHighlightBounds {
    pub kind: PreviewHighlightKind,
    pub bounds: [u32; 4],
}

/// A full preview submission. The renderer clears once, submits `background`
/// with `CHUSAN_COMMON_BACKGROUND_PLAYER` against the same target scene, then
/// submits `foreground` with `profile`'s player. A background whose
/// composition size differs from the foreground is an error: the game binary
/// provides no implicit editor scaling rule.
pub struct PreviewRequest<'a> {
    pub foreground: PreviewLayerRequest<'a>,
    pub background: Option<PreviewLayerRequest<'a>>,
    pub profile: PreviewProfile,
    /// Existing selected-cast bounds, retained for selection semantics.
    pub selection: Option<PreviewCastSelection>,
    /// Additional cast bounds tagged for direct/inherited editor highlights.
    pub highlights: &'a [PreviewHighlightRequest],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    /// Pixel-space `[x, y, width, height]` of the selected CAST's emitted
    /// geometry, including emitted descendants, clipped to this frame.
    pub selection_bounds: Option<[u32; 4]>,
    /// Highlight bounds in request order, excluding requests without submitted
    /// geometry after target-pass filtering.
    pub highlight_bounds: Vec<PreviewHighlightBounds>,
}

impl PreviewFrame {
    pub fn from_rgba(width: u32, height: u32, rgba: Vec<u8>) -> Result<Self, String> {
        let expected = usize::try_from(width)
            .ok()
            .and_then(|width| {
                usize::try_from(height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| format!("preview dimensions overflow: {width}×{height}"))?;
        if rgba.len() != expected {
            return Err(format!(
                "preview readback contains {} bytes; expected {expected} for {width}×{height}",
                rgba.len()
            ));
        }
        Ok(Self {
            width,
            height,
            rgba,
            selection_bounds: None,
            highlight_bounds: Vec::new(),
        })
    }
}

#[cfg(test)]
mod preview_frame_tests {
    use super::PreviewFrame;

    #[test]
    fn rgba_readback_preserves_every_channel() {
        let frame = PreviewFrame::from_rgba(1, 1, vec![0x11, 0x22, 0x33, 0x44]).unwrap();
        assert_eq!(frame.rgba, [0x11, 0x22, 0x33, 0x44]);
    }

    #[test]
    fn rgba_readback_rejects_wrong_pixel_count() {
        assert!(PreviewFrame::from_rgba(2, 1, vec![0; 4]).is_err());
    }
}

pub trait PreviewRenderer {
    fn name(&self) -> &str;

    /// Whether the backend implements the editor's complete preview contract.
    fn is_reference_accurate(&self) -> bool {
        false
    }

    fn omissions(&self) -> &str;

    fn render(&mut self, request: PreviewRequest<'_>) -> Result<PreviewFrame, String>;
}

pub struct PreviewCoordinator {
    renderer: Box<dyn PreviewRenderer>,
}

impl PreviewCoordinator {
    pub fn new(renderer: impl PreviewRenderer + 'static) -> Self {
        Self {
            renderer: Box::new(renderer),
        }
    }

    pub fn system_default() -> Self {
        match wgpu::WgpuPreviewRenderer::try_new() {
            Ok(renderer) => Self::new(renderer),
            Err(error) => Self::new(UnavailablePreviewRenderer::new(error)),
        }
    }

    pub fn backend_name(&self) -> &str {
        self.renderer.name()
    }

    pub fn is_reference_accurate(&self) -> bool {
        self.renderer.is_reference_accurate()
    }

    pub fn omissions(&self) -> &str {
        self.renderer.omissions()
    }

    pub fn render(&mut self, request: PreviewRequest<'_>) -> Result<PreviewFrame, String> {
        self.renderer.render(request)
    }
}

struct UnavailablePreviewRenderer {
    error: String,
}

impl UnavailablePreviewRenderer {
    fn new(error: String) -> Self {
        Self { error }
    }
}

impl PreviewRenderer for UnavailablePreviewRenderer {
    fn name(&self) -> &str {
        "GPU preview unavailable"
    }

    fn omissions(&self) -> &str {
        &self.error
    }

    fn render(&mut self, _request: PreviewRequest<'_>) -> Result<PreviewFrame, String> {
        Err(self.error.clone())
    }
}

#[cfg(all(test, not(any(target_os = "windows", target_os = "linux"))))]
mod tests {
    use super::PreviewCoordinator;

    #[test]
    fn system_default_prefers_webgpu() {
        std::thread::Builder::new()
            .name("WebGPU default selection".into())
            .stack_size(16 * 1024 * 1024)
            .spawn(|| {
                let preview = PreviewCoordinator::system_default();

                assert!(preview.backend_name().starts_with("WebGPU · "));
                assert!(!preview.is_reference_accurate());
                assert_eq!(
                    preview.omissions(),
                    "Render presets 22..32, unresolved scene/pass providers, and legacy .sbfont text remain unsupported"
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
