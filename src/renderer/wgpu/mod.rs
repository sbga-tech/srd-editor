mod backend;
mod shaders;
mod texture;

use backend::WgpuSrdRenderBackend;

use crate::renderer::backend::{FennelAtlasHandle, SrdTextureSetHandle};
use crate::renderer::gpu_preview::{GpuPreviewState, ManagedSrdRenderBackend};
use crate::renderer::{PreviewFrame, PreviewRenderer, PreviewRequest};

pub(super) struct WgpuPreviewRenderer {
    state: GpuPreviewState<WgpuSrdRenderBackend>,
    name: String,
}

impl WgpuPreviewRenderer {
    pub(super) fn try_new() -> Result<Self, String> {
        let backend = WgpuSrdRenderBackend::new()
            .map_err(|error| format!("failed to initialize WebGPU SRD renderer: {error}"))?;
        let name = format!("WebGPU · {}", backend.adapter_diagnostics());
        Ok(Self {
            state: GpuPreviewState::new(backend),
            name,
        })
    }
}

impl PreviewRenderer for WgpuPreviewRenderer {
    fn name(&self) -> &str {
        &self.name
    }

    fn omissions(&self) -> &str {
        "Legacy .sbfont text is omitted; an arbitrary SRD does not itself prove its host target profile"
    }

    fn render(&mut self, request: PreviewRequest<'_>) -> Result<PreviewFrame, String> {
        self.state.render(request, |_| Ok(()))
    }
}

impl ManagedSrdRenderBackend for WgpuSrdRenderBackend {
    fn release_srd_textures(&mut self, handle: SrdTextureSetHandle) {
        WgpuSrdRenderBackend::release_srd_textures(self, handle);
    }

    fn release_fennel_atlas(&mut self, handle: FennelAtlasHandle) {
        WgpuSrdRenderBackend::release_fennel_atlas(self, handle);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::document::EditorDocument;
    use crate::renderer::{PreviewLayerRequest, PreviewProfile};

    #[test]
    fn renders_representative_game_corpus_profiles() {
        std::thread::Builder::new()
            .name("WebGPU corpus smoke".into())
            .stack_size(16 * 1024 * 1024)
            .spawn(renders_representative_game_corpus_profiles_on_large_stack)
            .unwrap()
            .join()
            .unwrap();
    }

    fn renders_representative_game_corpus_profiles_on_large_stack() {
        let Some(root) = std::env::var_os("GAME_DATA_CORPUS").map(PathBuf::from) else {
            eprintln!("skipping: GAME_DATA_CORPUS is not set");
            return;
        };
        let cases = [
            (
                "common-yellow-loop",
                "surfboard/common/commonBackGround/CHU_UI_Common_BK_00_v11.srd",
                PreviewProfile::CommonBackgroundMain,
                0,
                0,
                true,
                false,
            ),
            (
                "common-alpha-test",
                "surfboard/common/commonBackGround/CHU_UI_Common_BK_00_v11.srd",
                PreviewProfile::CommonBackgroundMain,
                1,
                1,
                true,
                false,
            ),
            (
                "advertise-fennel",
                "surfboard/advertise/CHU_UI_Advertise_00_v10.srd",
                PreviewProfile::AdvertiseLogoMain,
                0,
                24,
                true,
                false,
            ),
            (
                "advertise-dual-texture",
                "surfboard/advertise/CHU_UI_Advertise_00_v10.srd",
                PreviewProfile::AdvertiseLogoMain,
                10,
                30,
                true,
                true,
            ),
            (
                "linked-verse",
                "surfboard/play/linkedVerse/CHU_UI_LinkedVERSE_Gate_00.srd",
                PreviewProfile::LinkedVerseGateMain,
                0,
                0,
                true,
                false,
            ),
        ];
        let mut renderer = WgpuPreviewRenderer::try_new().unwrap();
        for (
            revision,
            (
                label,
                relative,
                profile,
                animation_set_index,
                frame,
                expect_visible,
                expect_non_white_detail,
            ),
        ) in cases.into_iter().enumerate()
        {
            render_corpus_case(
                &mut renderer,
                &root.join(relative),
                profile,
                animation_set_index,
                frame,
                revision as u64,
                label,
                expect_visible,
                expect_non_white_detail,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_corpus_case(
        renderer: &mut WgpuPreviewRenderer,
        path: &Path,
        profile: PreviewProfile,
        animation_set_index: usize,
        frame: i32,
        revision: u64,
        label: &str,
        expect_visible: bool,
        expect_non_white_detail: bool,
    ) {
        let document = EditorDocument::load(path).unwrap();
        let hidden_layers = BTreeSet::new();
        let hidden_casts = BTreeSet::new();
        let solo_casts = BTreeSet::new();
        let runtime_text_inputs = BTreeMap::new();
        let animation_assignment = document
            .project
            .scenes
            .first()
            .and_then(|scene| scene.animation_sets.get(animation_set_index))
            .unwrap_or_else(|| panic!("{label} animation assignment is unavailable"));
        let frame = renderer
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
            .unwrap_or_else(|error| panic!("{label} WebGPU render failed: {error}"));
        let clear = [0x06, 0x0B, 0x17];
        let changed = frame
            .rgba
            .chunks_exact(4)
            .filter(|pixel| pixel[..3] != clear)
            .count();
        let non_white_detail = frame
            .rgba
            .chunks_exact(4)
            .filter(|pixel| pixel[0] < 240 || pixel[1] < 240)
            .count();
        let hash = frame
            .rgba
            .iter()
            .fold(0xCBF2_9CE4_8422_2325u64, |hash, byte| {
                (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01B3)
            });
        eprintln!(
            "{label}: {}x{}, changed={changed}, non_white_detail={non_white_detail}, rgba_fnv1a={hash:016X}",
            frame.width, frame.height
        );
        let mut ppm = format!("P6\n{} {}\n255\n", frame.width, frame.height).into_bytes();
        for pixel in frame.rgba.chunks_exact(4) {
            ppm.extend_from_slice(&pixel[..3]);
        }
        std::fs::write(std::env::temp_dir().join(format!("{label}.ppm")), ppm).unwrap();
        if expect_visible {
            assert!(changed > 0, "{label} rendered only the clear color");
        }
        if expect_non_white_detail {
            assert!(
                non_white_detail > 10_000,
                "{label} did not expose the title artwork"
            );
        }
    }
}
