mod backend;
mod bindings;
mod device;
mod dxvk;
mod fennel;
mod srd;
mod texture;

use self::backend::D3d9SrdRenderBackend;
use self::device::{D3d9ExDevice, D3d9ExDeviceStatus};
use crate::renderer::backend::{FennelAtlasHandle, SrdTextureSetHandle};
use crate::renderer::gpu_preview::{GpuPreviewState, GpuRenderPhase, ManagedSrdRenderBackend};
use crate::renderer::{PreviewFrame, PreviewRenderer, PreviewRequest};

pub(super) fn prepare_process_environment() {
    dxvk::prepare_process_environment();
}

pub struct D3d9PreviewRenderer {
    state: PreviewState,
    name: String,
}

struct PreviewState {
    gpu: GpuPreviewState<D3d9SrdRenderBackend>,
    device: D3d9ExDevice,
    device_objects_invalidated: bool,
}

impl D3d9PreviewRenderer {
    pub(super) fn try_new() -> Result<Self, String> {
        let state = PreviewState::new()?;
        let name = state.device.provider_label().to_owned();
        Ok(Self { state, name })
    }
}

impl PreviewRenderer for D3d9PreviewRenderer {
    fn name(&self) -> &str {
        &self.name
    }

    fn is_reference_accurate(&self) -> bool {
        false
    }

    fn omissions(&self) -> &str {
        "Legacy .sbfont text is omitted; an arbitrary SRD does not itself prove its host target profile"
    }

    fn render(&mut self, request: PreviewRequest<'_>) -> Result<PreviewFrame, String> {
        self.state.ensure_device_ready()?;
        let device = &mut self.state.device;
        self.state.gpu.render(request, |phase| match phase {
            GpuRenderPhase::Begin => device
                .begin_scene()
                .map_err(|error| format!("failed to begin D3D9 preview scene: {error}")),
            GpuRenderPhase::End => device
                .end_scene()
                .map_err(|error| format!("failed to end D3D9 preview scene: {error}")),
        })
    }
}

impl PreviewState {
    fn new() -> Result<Self, String> {
        let device = D3d9ExDevice::new_offscreen([1, 1])
            .map_err(|error| format!("failed to create offscreen Direct3D 9Ex device: {error}"))?;
        let backend = D3d9SrdRenderBackend::new(device.device())
            .map_err(|error| format!("failed to initialize SRD D3D9 renderer: {error}"))?;
        Ok(Self {
            device,
            gpu: GpuPreviewState::new(backend),
            device_objects_invalidated: false,
        })
    }

    fn ensure_device_ready(&mut self) -> Result<(), String> {
        match self
            .device
            .status()
            .map_err(|error| format!("failed to query D3D9 preview device status: {error}"))?
        {
            D3d9ExDeviceStatus::Ready => {}
            D3d9ExDeviceStatus::NeedsReset => {
                self.gpu.backend_mut().invalidate_device_objects();
                self.device_objects_invalidated = true;
                self.device
                    .reset()
                    .map_err(|error| format!("failed to reset D3D9 preview device: {error}"))?;
            }
            D3d9ExDeviceStatus::DeviceLost => {
                return Err("D3D9 preview device is lost; retry after Windows restores it".into());
            }
            D3d9ExDeviceStatus::Minimized => {
                return Err("D3D9 preview device has no renderable backbuffer".into());
            }
        }
        if self.device_objects_invalidated {
            self.gpu
                .backend_mut()
                .create_device_objects()
                .map_err(|error| format!("failed to restore D3D9 preview resources: {error}"))?;
            self.device_objects_invalidated = false;
        }
        Ok(())
    }
}

impl ManagedSrdRenderBackend for D3d9SrdRenderBackend {
    fn release_srd_textures(&mut self, handle: SrdTextureSetHandle) {
        D3d9SrdRenderBackend::release_srd_textures(self, handle);
    }

    fn release_fennel_atlas(&mut self, handle: FennelAtlasHandle) {
        D3d9SrdRenderBackend::release_fennel_atlas(self, handle);
    }
}
