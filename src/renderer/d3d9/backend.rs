use std::collections::BTreeMap;

use super::bindings::IDirect3DDevice9;

use crate::render::SrdRenderVertex;
use crate::renderer::assets::{FennelAtlasSourceSet, SrdTextureSourceSet};
use crate::renderer::backend::{
    CompositionReadback, FennelAtlasHandle, FennelRenderBatch, RenderBackendError,
    SrdExternalRenderState, SrdRenderBackend, SrdTextureSetHandle,
};
use crate::renderer::d3d9::fennel::FennelDx9Renderer;
use crate::renderer::d3d9::srd::{RenderTargetGuard, SrdDx9Renderer};
use crate::renderer::d3d9::texture::{RuhunaD3d9AtlasSet, SrdD3d9TextureSet};
use crate::srd_draw::EvidenceCompleteSrdDraw;

/// Complete D3D9 implementation of the platform-neutral SRD submission
/// boundary. The editor owns this adapter, never the native SRD/Fennel
/// renderers or their GPU texture sets.
pub struct D3d9SrdRenderBackend {
    device: IDirect3DDevice9,
    srd: SrdDx9Renderer,
    fennel: Option<FennelDx9Renderer>,
    composition_binding: Option<RenderTargetGuard>,
    texture_sets: BTreeMap<SrdTextureSetHandle, SrdD3d9TextureSet>,
    fennel_atlases: BTreeMap<FennelAtlasHandle, RuhunaD3d9AtlasSet>,
    next_texture_set: u64,
    next_fennel_atlas: u64,
}

impl D3d9SrdRenderBackend {
    pub fn new(device: &IDirect3DDevice9) -> Result<Self, RenderBackendError> {
        Ok(Self {
            device: device.clone(),
            srd: SrdDx9Renderer::new(device).map_err(backend_error)?,
            fennel: None,
            texture_sets: BTreeMap::new(),
            composition_binding: None,
            fennel_atlases: BTreeMap::new(),
            next_texture_set: 1,
            next_fennel_atlas: 1,
        })
    }

    pub fn invalidate_device_objects(&mut self) {
        self.composition_binding = None;
        self.srd.invalidate_device_objects();
        if let Some(renderer) = &mut self.fennel {
            renderer.invalidate_device_objects();
        }
        for textures in self.texture_sets.values_mut() {
            textures.invalidate_device_objects();
        }
        for atlas in self.fennel_atlases.values_mut() {
            atlas.invalidate_device_objects();
        }
    }

    pub fn create_device_objects(&mut self) -> Result<(), RenderBackendError> {
        self.srd.create_device_objects().map_err(backend_error)?;
        if let Some(renderer) = &mut self.fennel {
            renderer.create_device_objects().map_err(backend_error)?;
        }
        for textures in self.texture_sets.values_mut() {
            textures
                .create_device_objects(&self.device)
                .map_err(backend_error)?;
        }
        for atlas in self.fennel_atlases.values_mut() {
            atlas
                .create_device_objects(&self.device)
                .map_err(backend_error)?;
        }
        Ok(())
    }

    pub fn configure_composition_target(
        &mut self,
        size: [u32; 2],
    ) -> Result<(), RenderBackendError> {
        self.srd
            .configure_composition_target(size[0], size[1])
            .map_err(backend_error)
    }

    pub fn release_srd_textures(&mut self, handle: SrdTextureSetHandle) {
        self.texture_sets.remove(&handle);
    }

    pub fn release_fennel_atlas(&mut self, handle: FennelAtlasHandle) {
        self.fennel_atlases.remove(&handle);
    }

    pub fn read_composition(&self) -> Result<CompositionReadback, RenderBackendError> {
        let readback = self.srd.read_composition_bgra().map_err(backend_error)?;
        Ok(CompositionReadback::from_bgra(
            readback.width,
            readback.height,
            readback.bgra,
        ))
    }

    fn ensure_fennel_renderer(&mut self) -> Result<&mut FennelDx9Renderer, RenderBackendError> {
        if self.fennel.is_none() {
            self.fennel = Some(FennelDx9Renderer::new(&self.device).map_err(backend_error)?);
        }
        Ok(self
            .fennel
            .as_mut()
            .expect("Fennel renderer was initialized above"))
    }
}

impl SrdRenderBackend for D3d9SrdRenderBackend {
    fn name(&self) -> &'static str {
        "Direct3D 9"
    }

    fn upload_srd_textures(
        &mut self,
        sources: SrdTextureSourceSet,
    ) -> Result<SrdTextureSetHandle, RenderBackendError> {
        let handle = SrdTextureSetHandle::new(self.next_texture_set);
        self.next_texture_set = self.next_texture_set.checked_add(1).ok_or_else(|| {
            RenderBackendError("D3D9 SRD texture-set handle space exhausted".to_string())
        })?;
        let textures =
            SrdD3d9TextureSet::from_sources(&self.device, sources).map_err(backend_error)?;
        self.texture_sets.insert(handle, textures);
        Ok(handle)
    }

    fn upload_fennel_atlas(
        &mut self,
        sources: FennelAtlasSourceSet,
    ) -> Result<FennelAtlasHandle, RenderBackendError> {
        self.ensure_fennel_renderer()?;
        let handle = FennelAtlasHandle::new(self.next_fennel_atlas);
        self.next_fennel_atlas = self.next_fennel_atlas.checked_add(1).ok_or_else(|| {
            RenderBackendError("D3D9 Fennel atlas handle space exhausted".to_string())
        })?;
        let atlas =
            RuhunaD3d9AtlasSet::from_sources(&self.device, sources).map_err(backend_error)?;
        self.fennel_atlases.insert(handle, atlas);
        Ok(handle)
    }

    fn clear_uploaded_resources(&mut self) {
        self.texture_sets.clear();
        self.fennel_atlases.clear();
    }

    fn configure_composition_target(&mut self, size: [u32; 2]) -> Result<(), RenderBackendError> {
        D3d9SrdRenderBackend::configure_composition_target(self, size)
    }

    fn composition_size(&self) -> Option<[u32; 2]> {
        self.srd.composition_size()
    }

    fn begin_composition(&mut self, clear_argb: u32) -> Result<(), RenderBackendError> {
        if self.composition_binding.is_some() {
            return Err(RenderBackendError(
                "D3D9 Composition target is already bound".to_string(),
            ));
        }
        self.composition_binding = Some(
            self.srd
                .bind_composition_target(clear_argb)
                .map_err(backend_error)?,
        );
        Ok(())
    }

    fn end_composition(&mut self) -> Result<(), RenderBackendError> {
        let binding = self.composition_binding.take().ok_or_else(|| {
            RenderBackendError("D3D9 Composition target is not bound".to_string())
        })?;
        drop(binding);
        Ok(())
    }

    fn read_composition(&self) -> Result<CompositionReadback, RenderBackendError> {
        D3d9SrdRenderBackend::read_composition(self)
    }

    fn render_srd(
        &mut self,
        draws: &[EvidenceCompleteSrdDraw],
        external: SrdExternalRenderState,
        textures: Option<SrdTextureSetHandle>,
    ) -> Result<(), RenderBackendError> {
        let Self {
            srd, texture_sets, ..
        } = self;
        let textures = textures
            .map(|handle| {
                texture_sets.get(&handle).ok_or_else(|| {
                    RenderBackendError(format!(
                        "D3D9 SRD texture-set handle {handle:?} is not loaded"
                    ))
                })
            })
            .transpose()?;
        srd.render(draws, external, textures).map_err(backend_error)
    }

    fn render_srd_triangle_strip(
        &mut self,
        state: &EvidenceCompleteSrdDraw,
        vertices: &[SrdRenderVertex],
        external: SrdExternalRenderState,
        textures: Option<SrdTextureSetHandle>,
    ) -> Result<(), RenderBackendError> {
        let Self {
            srd, texture_sets, ..
        } = self;
        let textures = textures
            .map(|handle| {
                texture_sets.get(&handle).ok_or_else(|| {
                    RenderBackendError(format!(
                        "D3D9 SRD texture-set handle {handle:?} is not loaded"
                    ))
                })
            })
            .transpose()?;
        srd.render_triangle_strip(state, vertices, external, textures)
            .map_err(backend_error)
    }

    fn render_fennel(
        &mut self,
        batches: &[FennelRenderBatch<'_>],
        external: SrdExternalRenderState,
        atlas: FennelAtlasHandle,
    ) -> Result<(), RenderBackendError> {
        let Self {
            fennel,
            fennel_atlases,
            ..
        } = self;
        let atlas_resource = fennel_atlases.get(&atlas).ok_or_else(|| {
            RenderBackendError(format!("D3D9 Fennel atlas handle {atlas:?} is not loaded"))
        })?;
        let renderer = fennel.as_mut().ok_or_else(|| {
            RenderBackendError("D3D9 Fennel renderer is not initialized".to_string())
        })?;
        renderer
            .render(batches, external, atlas_resource)
            .map_err(backend_error)
    }
}

fn backend_error(error: impl std::fmt::Display) -> RenderBackendError {
    RenderBackendError(error.to_string())
}
