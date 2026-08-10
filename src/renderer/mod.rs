use std::collections::{BTreeMap, BTreeSet};

use crate::document::EditorDocument;
use crate::scene::AnimationSetDefinition;
use crate::srd_draw::{FennelRuntimeTextCastKey, FennelSrdRuntimeTextInput};

pub mod assets;
pub mod backend;
#[cfg(any(
    target_os = "windows",
    all(target_os = "linux", feature = "dxvk-native")
))]
mod d3d9;
mod gpu_preview;
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

    /// Whether the backend is the reference D3D9 path, rather than a
    /// compatibility renderer with declared omissions.
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
        #[cfg(any(
            target_os = "windows",
            all(target_os = "linux", feature = "dxvk-native")
        ))]
        if std::env::var("SRD_PREVIEW_BACKEND")
            .is_ok_and(|backend| backend.eq_ignore_ascii_case("d3d9"))
        {
            return match d3d9::D3d9PreviewRenderer::try_new() {
                Ok(renderer) => Self::new(renderer),
                Err(error) => Self::new(UnavailablePreviewRenderer::new(error)),
            };
        }

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

/// Applies reference-backend settings only when D3D9 was explicitly selected.
pub fn prepare_process_environment() {
    #[cfg(any(
        target_os = "windows",
        all(target_os = "linux", feature = "dxvk-native")
    ))]
    if std::env::var("SRD_PREVIEW_BACKEND")
        .is_ok_and(|backend| backend.eq_ignore_ascii_case("d3d9"))
    {
        d3d9::prepare_process_environment();
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
                    "Legacy .sbfont text is omitted; an arbitrary SRD does not itself prove its host target profile"
                );
            })
            .unwrap()
            .join()
            .unwrap();
    }
}

#[cfg(all(
    test,
    any(
        target_os = "windows",
        all(target_os = "linux", feature = "dxvk-native")
    )
))]
mod differential_tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs::File;
    use std::io::BufWriter;
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::document::EditorDocument;

    #[derive(Clone, Copy)]
    struct DiffTolerance {
        maximum_delta: u8,
        p999_delta: u8,
        mean_delta: f64,
    }
    #[test]
    fn webgpu_matches_reference_d3d9_corpus_captures() {
        std::thread::Builder::new()
            .name("D3D9-WebGPU differential".into())
            .stack_size(16 * 1024 * 1024)
            .spawn(webgpu_matches_reference_d3d9_corpus_captures_on_large_stack)
            .unwrap()
            .join()
            .unwrap();
    }

    fn webgpu_matches_reference_d3d9_corpus_captures_on_large_stack() {
        let Some(root) = std::env::var_os("GAME_DATA_CORPUS").map(PathBuf::from) else {
            eprintln!("skipping: GAME_DATA_CORPUS is not set");
            return;
        };
        d3d9::prepare_process_environment();
        let mut reference = d3d9::D3d9PreviewRenderer::try_new().unwrap();
        let mut candidate = wgpu::WgpuPreviewRenderer::try_new().unwrap();
        let cases = [
            (
                "common-yellow-loop",
                "surfboard/common/commonBackGround/CHU_UI_Common_BK_00_v11.srd",
                PreviewProfile::CommonBackgroundMain,
                0,
                0,
                DiffTolerance {
                    maximum_delta: 32,
                    p999_delta: 10,
                    mean_delta: 0.1,
                },
            ),
            (
                "common-alpha-test",
                "surfboard/common/commonBackGround/CHU_UI_Common_BK_00_v11.srd",
                PreviewProfile::CommonBackgroundMain,
                1,
                1,
                DiffTolerance {
                    maximum_delta: 2,
                    p999_delta: 1,
                    mean_delta: 0.002,
                },
            ),
            (
                "advertise-fennel",
                "surfboard/advertise/CHU_UI_Advertise_00_v10.srd",
                PreviewProfile::AdvertiseLogoMain,
                0,
                24,
                DiffTolerance {
                    maximum_delta: 192,
                    p999_delta: 160,
                    mean_delta: 1.6,
                },
            ),
            (
                "linked-verse",
                "surfboard/play/linkedVerse/CHU_UI_LinkedVERSE_Gate_00.srd",
                PreviewProfile::LinkedVerseGateMain,
                0,
                0,
                DiffTolerance {
                    maximum_delta: 0,
                    p999_delta: 0,
                    mean_delta: 0.0,
                },
            ),
        ];
        for (revision, (label, relative, profile, animation_set_index, frame, tolerance)) in
            cases.into_iter().enumerate()
        {
            compare_case(
                &mut reference,
                &mut candidate,
                &root.join(relative),
                profile,
                animation_set_index,
                frame,
                revision as u64,
                label,
                tolerance,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn compare_case(
        reference: &mut d3d9::D3d9PreviewRenderer,
        candidate: &mut wgpu::WgpuPreviewRenderer,
        path: &Path,
        profile: PreviewProfile,
        animation_set_index: usize,
        frame: i32,
        revision: u64,
        label: &str,
        tolerance: DiffTolerance,
    ) {
        let document = EditorDocument::load(path).unwrap();
        let hidden_layers = BTreeSet::new();
        let hidden_casts = BTreeSet::new();
        let solo_casts = BTreeSet::new();
        let runtime_text_inputs = BTreeMap::new();
        let animation_assignment = document
            .project
            .scenes
            .get(0)
            .and_then(|scene| scene.animation_sets.get(animation_set_index))
            .unwrap_or_else(|| panic!("{label} animation assignment is unavailable"));
        let render = |renderer: &mut dyn PreviewRenderer| {
            renderer
                .render(PreviewRequest {
                    foreground: PreviewLayerRequest {
                        document: &document,
                        document_revision: revision,
                        scene_index: 0,
                        animation_assignment: Some(animation_assignment),
                        frame,
                        hidden_layers: &hidden_layers,
                        solo_layers: &hidden_layers,
                        hidden_casts: &hidden_casts,
                        solo_casts: &solo_casts,
                        runtime_text_inputs: &runtime_text_inputs,
                    },
                    background: None,
                    profile,
                    selection: None,
                    highlights: &[],
                })
                .unwrap_or_else(|error| panic!("{label} render failed: {error}"))
        };
        let reference = render(reference);
        let candidate = render(candidate);
        assert_eq!(candidate.width, reference.width, "{label} width");
        assert_eq!(candidate.height, reference.height, "{label} height");
        assert_eq!(candidate.rgba.len(), reference.rgba.len(), "{label} bytes");

        let mut differing_pixels = 0usize;
        let mut maximum_delta = 0u8;
        let mut total_delta = 0u64;
        let mut delta_histogram = [0u64; 256];
        for (reference, candidate) in reference
            .rgba
            .chunks_exact(4)
            .zip(candidate.rgba.chunks_exact(4))
        {
            if reference != candidate {
                differing_pixels += 1;
            }
            for (&reference, &candidate) in reference.iter().zip(candidate) {
                let delta = reference.abs_diff(candidate);
                maximum_delta = maximum_delta.max(delta);
                total_delta += u64::from(delta);
                delta_histogram[usize::from(delta)] += 1;
            }
        }
        let pixels =
            usize::try_from(reference.width).unwrap() * usize::try_from(reference.height).unwrap();
        let mean_delta = total_delta as f64 / (pixels * 4) as f64;
        let p99_delta = percentile(&delta_histogram, 99, 100);
        let p999_delta = percentile(&delta_histogram, 999, 1000);
        eprintln!(
            "{label}: differing={differing_pixels}/{pixels}, max_delta={maximum_delta}, p99={p99_delta}, p99.9={p999_delta}, mean_delta={mean_delta:.6}"
        );
        write_differential_captures(label, &reference, &candidate);
        assert!(
            maximum_delta <= tolerance.maximum_delta,
            "{label} maximum channel delta {maximum_delta} exceeds {}",
            tolerance.maximum_delta
        );
        assert!(
            p999_delta <= tolerance.p999_delta,
            "{label} p99.9 channel delta {p999_delta} exceeds {}",
            tolerance.p999_delta
        );
        assert!(
            mean_delta <= tolerance.mean_delta,
            "{label} mean channel delta {mean_delta:.6} exceeds {}",
            tolerance.mean_delta
        );
    }

    fn percentile(histogram: &[u64; 256], numerator: u64, denominator: u64) -> u8 {
        let total = histogram.iter().sum::<u64>();
        let target = total.saturating_mul(numerator).div_ceil(denominator);
        let mut accumulated = 0u64;
        for (value, count) in histogram.iter().enumerate() {
            accumulated += count;
            if accumulated >= target {
                return value as u8;
            }
        }
        u8::MAX
    }

    fn write_differential_captures(
        label: &str,
        reference: &PreviewFrame,
        candidate: &PreviewFrame,
    ) {
        let Some(directory) = std::env::var_os("SRD_DIFFERENTIAL_OUTPUT").map(PathBuf::from) else {
            return;
        };
        std::fs::create_dir_all(&directory).unwrap();
        write_png(&directory.join(format!("{label}-d3d9.png")), reference);
        write_png(&directory.join(format!("{label}-webgpu.png")), candidate);
        let mut difference = Vec::with_capacity(reference.rgba.len());
        for (reference, candidate) in reference
            .rgba
            .chunks_exact(4)
            .zip(candidate.rgba.chunks_exact(4))
        {
            difference.extend_from_slice(&[
                reference[0].abs_diff(candidate[0]).saturating_mul(4),
                reference[1].abs_diff(candidate[1]).saturating_mul(4),
                reference[2].abs_diff(candidate[2]).saturating_mul(4),
                0xFF,
            ]);
        }
        write_png(
            &directory.join(format!("{label}-delta-x4.png")),
            &PreviewFrame {
                width: reference.width,
                height: reference.height,
                rgba: difference,
                selection_bounds: None,
                highlight_bounds: Vec::new(),
            },
        );
    }

    fn write_png(path: &Path, frame: &PreviewFrame) {
        let mut encoder = png::Encoder::new(
            BufWriter::new(File::create(path).unwrap()),
            frame.width,
            frame.height,
        );
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&frame.rgba)
            .unwrap();
    }
}
