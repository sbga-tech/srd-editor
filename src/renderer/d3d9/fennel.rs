use std::collections::BTreeMap;
use std::ffi::c_void;
use std::mem;
use std::ptr;

use super::bindings::{
    D3DLOCK_DISCARD, D3DPOOL_DEFAULT, D3DPT_TRIANGLELIST, D3DRS_ALPHABLENDENABLE, D3DRS_ALPHAFUNC,
    D3DRS_ALPHAREF, D3DRS_ALPHATESTENABLE, D3DRS_BLENDOP, D3DRS_BLENDOPALPHA,
    D3DRS_COLORWRITEENABLE, D3DRS_CULLMODE, D3DRS_DESTBLEND, D3DRS_DESTBLENDALPHA, D3DRS_FILLMODE,
    D3DRS_SCISSORTESTENABLE, D3DRS_SEPARATEALPHABLENDENABLE, D3DRS_SRCBLEND, D3DRS_SRCBLENDALPHA,
    D3DRS_STENCILENABLE, D3DRS_ZENABLE, D3DRS_ZFUNC, D3DRS_ZWRITEENABLE, D3DSAMP_ADDRESSU,
    D3DSAMP_ADDRESSV, D3DSAMP_BORDERCOLOR, D3DSAMP_MAGFILTER, D3DSAMP_MAXANISOTROPY,
    D3DSAMP_MAXMIPLEVEL, D3DSAMP_MINFILTER, D3DSAMP_MIPFILTER, D3DSAMP_MIPMAPLODBIAS, D3DSBT_ALL,
    D3DUSAGE_DYNAMIC, D3DUSAGE_WRITEONLY, D3DVERTEXELEMENT9, Error, HRESULT, IDirect3DDevice9,
    IDirect3DPixelShader9, IDirect3DStateBlock9, IDirect3DVertexBuffer9,
    IDirect3DVertexDeclaration9, IDirect3DVertexShader9, RECT, Result,
};

use crate::fennel::{
    FennelRenderVertex, fennel_default_draw_packet, fennel_default_raster_state,
    fennel_default_shader_key,
};
use crate::render::{CeylonDepthState, FENNEL_D3D9_VERTEX_DECLARATION, ceylon_d3d9_blend_preset};
use crate::renderer::backend::{FennelRenderBatch, SrdExternalRenderState};
use crate::renderer::d3d9::texture::RuhunaD3d9AtlasSet;
use crate::shader::CEYLON_SIMPLE_SHADER_KEY_LENGTH;
use crate::shader_bytecode::{EmbeddedSimpleShaderPair, embedded_simple_shader_pair};

const E_FAIL: HRESULT = HRESULT(0x8000_4005_u32 as i32);
const E_INVALIDARG: HRESULT = HRESULT(0x8007_0057_u32 as i32);

pub struct FennelDx9Renderer {
    device: IDirect3DDevice9,
    vertex_declaration: Option<IDirect3DVertexDeclaration9>,
    shaders: BTreeMap<[u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH], ShaderObjects>,
    vertex_buffer: Option<IDirect3DVertexBuffer9>,
    vertex_capacity: usize,
}

struct ShaderObjects {
    vertex: IDirect3DVertexShader9,
    pixel: IDirect3DPixelShader9,
}

impl FennelDx9Renderer {
    pub fn new(device: &IDirect3DDevice9) -> Result<Self> {
        let mut renderer = Self {
            device: device.clone(),
            vertex_declaration: None,
            shaders: BTreeMap::new(),
            vertex_buffer: None,
            vertex_capacity: 0,
        };
        renderer.create_device_objects()?;
        Ok(renderer)
    }

    pub fn invalidate_device_objects(&mut self) {
        self.vertex_buffer = None;
        self.vertex_capacity = 0;
        self.shaders.clear();
        self.vertex_declaration = None;
    }

    pub fn create_device_objects(&mut self) -> Result<()> {
        let elements = FENNEL_D3D9_VERTEX_DECLARATION.map(|element| D3DVERTEXELEMENT9 {
            Stream: element.stream,
            Offset: element.offset,
            Type: element.declaration_type,
            Method: element.method,
            Usage: element.usage,
            UsageIndex: element.usage_index,
        });
        unsafe {
            self.vertex_declaration = Some(self.device.CreateVertexDeclaration(elements.as_ptr())?);
            for is_2d in [false, true] {
                let key = fennel_compact_shader_key(is_2d)?;
                let pair = embedded_simple_shader_pair(&key)
                    .ok_or_else(|| Error::new(E_FAIL, "embedded Fennel shader pair is missing"))?;
                self.shaders.insert(
                    key,
                    ShaderObjects {
                        vertex: self
                            .device
                            .CreateVertexShader(pair.vertex_shader.as_ptr())?,
                        pixel: self.device.CreatePixelShader(pair.pixel_shader.as_ptr())?,
                    },
                );
            }
        }
        Ok(())
    }

    pub fn render(
        &mut self,
        batches: &[FennelRenderBatch<'_>],
        external: SrdExternalRenderState,
        atlas: &RuhunaD3d9AtlasSet,
    ) -> Result<()> {
        if batches.is_empty() {
            return Ok(());
        }
        let state = StateBlockGuard::capture(&self.device)?;
        for batch in batches {
            self.render_batch(batch, external, atlas)?;
        }
        state.restore()
    }

    fn render_batch(
        &mut self,
        batch: &FennelRenderBatch<'_>,
        external: SrdExternalRenderState,
        atlas: &RuhunaD3d9AtlasSet,
    ) -> Result<()> {
        if batch.vertices.is_empty() {
            return Ok(());
        }
        if batch.vertices.len() % 6 != 0 {
            return Err(Error::new(
                E_INVALIDARG,
                "Fennel batch vertex count is not a multiple of six",
            ));
        }
        let page = atlas.get(batch.page_index).ok_or_else(|| {
            Error::new(
                E_INVALIDARG,
                format!("Fennel atlas page {} is not loaded", batch.page_index),
            )
        })?;
        self.ensure_vertex_capacity(batch.vertices.len())?;
        let vertex_buffer = self
            .vertex_buffer
            .as_ref()
            .ok_or_else(|| Error::new(E_FAIL, "Fennel D3D9 vertex buffer is not available"))?;
        upload_vertices(vertex_buffer, batch.vertices)?;

        let key = fennel_compact_shader_key(batch.is_2d)?;
        let pair = embedded_simple_shader_pair(&key)
            .ok_or_else(|| Error::new(E_FAIL, "embedded Fennel shader pair is missing"))?;
        self.require_loaded_pair(&pair)?;
        let shaders = self
            .shaders
            .get(&key)
            .ok_or_else(|| Error::new(E_FAIL, "Fennel D3D9 shader objects are unavailable"))?;
        let declaration = self
            .vertex_declaration
            .as_ref()
            .ok_or_else(|| Error::new(E_FAIL, "Fennel D3D9 vertex declaration is unavailable"))?;

        let packet = fennel_default_draw_packet(batch.is_2d);
        let blend = ceylon_d3d9_blend_preset(i32::from(packet.table_preset_id()));
        let mut alpha_stencil = external.alpha_stencil;
        alpha_stencil.alpha_test_enabled = blend.alpha_test_enabled;
        alpha_stencil.apply_draw_packet(packet);
        let alpha_function = alpha_stencil
            .alpha_function()
            .ok_or_else(|| Error::new(E_INVALIDARG, "invalid internal Fennel alpha comparison"))?;
        let raster = fennel_default_raster_state(batch.is_2d);
        let depth = CeylonDepthState::from_draw_flags(packet.draw_flags_00);
        let cull = raster
            .cull_mode()
            .ok_or_else(|| Error::new(E_INVALIDARG, "invalid internal Fennel cull mode"))?;
        let z_function = depth
            .z_function()
            .ok_or_else(|| Error::new(E_INVALIDARG, "invalid internal Fennel Z comparison"))?;
        let sampler = atlas.sampler();
        let constants = batch.fixed_constants;

        unsafe {
            self.device.SetVertexDeclaration(declaration)?;
            self.device
                .SetStreamSource(0, vertex_buffer, 0, FennelRenderVertex::STRIDE as u32)?;
            self.device.SetVertexShader(&shaders.vertex)?;
            self.device.SetPixelShader(&shaders.pixel)?;
            self.device.SetTexture(0, &page.texture)?;
            self.device.SetTexture(1, None)?;
            self.device.SetTexture(2, None)?;
            self.device
                .SetSamplerState(0, D3DSAMP_ADDRESSU, sampler.base.address_u as u32)?;
            self.device
                .SetSamplerState(0, D3DSAMP_ADDRESSV, sampler.base.address_v as u32)?;
            self.device
                .SetSamplerState(0, D3DSAMP_MINFILTER, sampler.base.min_filter as u32)?;
            self.device
                .SetSamplerState(0, D3DSAMP_MAGFILTER, sampler.base.mag_filter as u32)?;
            self.device
                .SetSamplerState(0, D3DSAMP_MIPFILTER, sampler.mip_filter as u32)?;
            self.device
                .SetSamplerState(0, D3DSAMP_MAXMIPLEVEL, sampler.max_mip_level)?;
            self.device
                .SetSamplerState(0, D3DSAMP_MAXANISOTROPY, sampler.max_anisotropy)?;
            self.device
                .SetSamplerState(0, D3DSAMP_MIPMAPLODBIAS, sampler.mip_lod_bias_bits)?;
            self.device
                .SetSamplerState(0, D3DSAMP_BORDERCOLOR, sampler.border_color)?;

            self.device.SetVertexShaderConstantF(
                0,
                constants.vertex_c0_c3_world.rows.as_ptr().cast(),
                4,
            )?;
            self.device.SetVertexShaderConstantF(
                4,
                constants.vertex_c4_c7.rows.as_ptr().cast(),
                4,
            )?;
            self.device.SetVertexShaderConstantF(
                8,
                constants.vertex_c8_fixed_param0.as_ptr(),
                1,
            )?;
            self.device.SetVertexShaderConstantF(
                9,
                constants.vertex_c9_fixed_param1.as_ptr(),
                1,
            )?;
            if batch.is_2d {
                self.device.SetVertexShaderConstantF(
                    10,
                    constants.vertex_c10_screen_param.as_ptr(),
                    1,
                )?;
            } else {
                self.device.SetVertexShaderConstantF(
                    10,
                    constants
                        .vertex_c10_c13_projection_view
                        .rows
                        .as_ptr()
                        .cast(),
                    4,
                )?;
            }
            self.device
                .SetPixelShaderConstantF(0, constants.pixel_c0_fixed_param0.as_ptr(), 1)?;

            self.device
                .SetRenderState(D3DRS_ALPHABLENDENABLE, u32::from(blend.alpha_blend_enabled))?;
            self.device
                .SetRenderState(D3DRS_SRCBLEND, blend.source_blend as u32)?;
            self.device
                .SetRenderState(D3DRS_DESTBLEND, blend.destination_blend as u32)?;
            self.device
                .SetRenderState(D3DRS_BLENDOP, blend.blend_operation as u32)?;
            self.device.SetRenderState(
                D3DRS_ALPHATESTENABLE,
                u32::from(alpha_stencil.alpha_test_enabled),
            )?;
            self.device
                .SetRenderState(D3DRS_ALPHAREF, alpha_stencil.alpha_reference)?;
            self.device
                .SetRenderState(D3DRS_ALPHAFUNC, alpha_function as u32)?;
            self.device.SetRenderState(
                D3DRS_SEPARATEALPHABLENDENABLE,
                u32::from(blend.separate_alpha_blend_enabled),
            )?;
            self.device
                .SetRenderState(D3DRS_SRCBLENDALPHA, blend.source_blend_alpha as u32)?;
            self.device
                .SetRenderState(D3DRS_DESTBLENDALPHA, blend.destination_blend_alpha as u32)?;
            self.device
                .SetRenderState(D3DRS_BLENDOPALPHA, blend.blend_operation_alpha as u32)?;
            self.device.SetRenderState(D3DRS_CULLMODE, cull as u32)?;
            self.device
                .SetRenderState(D3DRS_FILLMODE, raster.fill_mode() as u32)?;
            self.device
                .SetRenderState(D3DRS_COLORWRITEENABLE, raster.color_write_mask)?;
            self.device
                .SetRenderState(D3DRS_ZENABLE, u32::from(depth.z_enabled))?;
            self.device
                .SetRenderState(D3DRS_ZWRITEENABLE, u32::from(depth.z_write_enabled))?;
            self.device.SetRenderState(D3DRS_ZFUNC, z_function as u32)?;
            self.device.SetRenderState(D3DRS_STENCILENABLE, 0)?;
            self.device
                .SetRenderState(D3DRS_SCISSORTESTENABLE, u32::from(external.scissor.enabled))?;
            if external.scissor.enabled {
                let rectangle = RECT {
                    left: external.scissor.rectangle.left,
                    top: external.scissor.rectangle.top,
                    right: external.scissor.rectangle.right,
                    bottom: external.scissor.rectangle.bottom,
                };
                self.device.SetScissorRect(&rectangle)?;
            }
            self.device.DrawPrimitive(
                D3DPT_TRIANGLELIST,
                0,
                u32::try_from(batch.vertices.len() / 3).map_err(|_| {
                    Error::new(E_INVALIDARG, "Fennel primitive count overflows u32")
                })?,
            )?;
        }
        Ok(())
    }

    fn ensure_vertex_capacity(&mut self, required: usize) -> Result<()> {
        if self.vertex_buffer.is_some() && self.vertex_capacity >= required {
            return Ok(());
        }
        let capacity = required.max(6).next_power_of_two();
        self.vertex_buffer = Some(create_vertex_buffer(&self.device, capacity)?);
        self.vertex_capacity = capacity;
        Ok(())
    }

    fn require_loaded_pair(&self, pair: &EmbeddedSimpleShaderPair) -> Result<()> {
        if self.shaders.is_empty() || pair.vertex_shader.is_empty() || pair.pixel_shader.is_empty()
        {
            return Err(Error::new(E_FAIL, "Fennel shader bytecode is unavailable"));
        }
        Ok(())
    }
}

fn fennel_compact_shader_key(is_2d: bool) -> Result<[u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH]> {
    fennel_default_shader_key(is_2d)
        .srd_simple_shader_direct_contributions()
        .map(|bits| bits.compact_key())
        .map_err(|error| {
            Error::new(
                E_INVALIDARG,
                format!("unsupported Fennel shader key: {error:?}"),
            )
        })
}

fn create_vertex_buffer(
    device: &IDirect3DDevice9,
    vertex_capacity: usize,
) -> Result<IDirect3DVertexBuffer9> {
    let byte_len = vertex_capacity
        .checked_mul(FennelRenderVertex::STRIDE)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| Error::new(E_INVALIDARG, "Fennel vertex buffer size overflows u32"))?;
    let mut buffer = None;
    unsafe {
        device.CreateVertexBuffer(
            byte_len,
            (D3DUSAGE_DYNAMIC | D3DUSAGE_WRITEONLY) as u32,
            0,
            D3DPOOL_DEFAULT,
            &mut buffer,
            ptr::null_mut(),
        )?;
    }
    buffer.ok_or_else(|| Error::new(E_FAIL, "CreateVertexBuffer returned null for Fennel"))
}

fn upload_vertices(buffer: &IDirect3DVertexBuffer9, vertices: &[FennelRenderVertex]) -> Result<()> {
    debug_assert_eq!(
        mem::size_of::<FennelRenderVertex>(),
        FennelRenderVertex::STRIDE
    );
    let byte_len = vertices
        .len()
        .checked_mul(FennelRenderVertex::STRIDE)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| Error::new(E_INVALIDARG, "Fennel vertex upload size overflows u32"))?;
    let mut destination: *mut c_void = ptr::null_mut();
    unsafe {
        buffer.Lock(0, byte_len, &mut destination, D3DLOCK_DISCARD as u32)?;
        ptr::copy_nonoverlapping(
            vertices.as_ptr().cast::<u8>(),
            destination.cast::<u8>(),
            byte_len as usize,
        );
        buffer.Unlock()?;
    }
    Ok(())
}

struct StateBlockGuard {
    state: IDirect3DStateBlock9,
}

impl StateBlockGuard {
    fn capture(device: &IDirect3DDevice9) -> Result<Self> {
        let state = unsafe { device.CreateStateBlock(D3DSBT_ALL)? };
        unsafe { state.Capture()? };
        Ok(Self { state })
    }

    fn restore(self) -> Result<()> {
        unsafe { self.state.Apply() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_batch_keys_are_embedded() {
        assert_eq!(
            fennel_compact_shader_key(false).unwrap(),
            *b"AAMAAABAABGAAAAAAA"
        );
        assert_eq!(
            fennel_compact_shader_key(true).unwrap(),
            *b"EAMAAABAABGAAAAAAA"
        );
        assert!(embedded_simple_shader_pair(&fennel_compact_shader_key(false).unwrap()).is_some());
        assert!(embedded_simple_shader_pair(&fennel_compact_shader_key(true).unwrap()).is_some());
    }
}
