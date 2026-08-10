use std::collections::BTreeMap;
use std::ffi::c_void;
use std::mem;
use std::ptr;
use std::slice;

use super::bindings::{
    D3DCLEAR_TARGET, D3DLOCK_DISCARD, D3DLOCKED_RECT, D3DPOOL_DEFAULT, D3DPOOL_SYSTEMMEM,
    D3DPT_TRIANGLESTRIP, D3DRS_ALPHABLENDENABLE, D3DRS_ALPHAFUNC, D3DRS_ALPHAREF,
    D3DRS_ALPHATESTENABLE, D3DRS_BLENDOP, D3DRS_BLENDOPALPHA, D3DRS_COLORWRITEENABLE,
    D3DRS_CULLMODE, D3DRS_DESTBLEND, D3DRS_DESTBLENDALPHA, D3DRS_FILLMODE, D3DRS_SCISSORTESTENABLE,
    D3DRS_SEPARATEALPHABLENDENABLE, D3DRS_SRCBLEND, D3DRS_SRCBLENDALPHA, D3DRS_STENCILENABLE,
    D3DRS_ZENABLE, D3DRS_ZFUNC, D3DRS_ZWRITEENABLE, D3DSAMP_ADDRESSU, D3DSAMP_ADDRESSV,
    D3DSAMP_MAGFILTER, D3DSAMP_MINFILTER, D3DSBT_ALL, D3DUSAGE_DYNAMIC, D3DUSAGE_RENDERTARGET,
    D3DUSAGE_WRITEONLY, D3DVERTEXELEMENT9, D3DVIEWPORT9, Error, HRESULT, IDirect3DBaseTexture9,
    IDirect3DDevice9, IDirect3DPixelShader9, IDirect3DStateBlock9, IDirect3DSurface9,
    IDirect3DTexture9, IDirect3DVertexBuffer9, IDirect3DVertexDeclaration9, IDirect3DVertexShader9,
    Interface, RECT, Result,
};

use crate::render::{SRD_D3D9_VERTEX_DECLARATION, SrdRenderVertex};
use crate::renderer::backend::SrdExternalRenderState;
use crate::renderer::d3d9::texture::SrdD3d9TextureSet;
use crate::shader::CEYLON_SIMPLE_SHADER_KEY_LENGTH;
use crate::shader_bytecode::{
    EMBEDDED_SIMPLE_SHADER_KEYS, EmbeddedSimpleShaderPair, embedded_simple_shader_pair,
};
use crate::srd_draw::EvidenceCompleteSrdDraw;

const E_FAIL: HRESULT = HRESULT(0x8000_4005_u32 as i32);
const E_INVALIDARG: HRESULT = HRESULT(0x8007_0057_u32 as i32);

/// Native D3D9 submission for the evidence-complete SRD subset.
///
/// The dynamic vertex buffer is a DEFAULT-pool resource and must be released
/// before ResetEx. Shader objects and the vertex declaration are recreated as
/// well so reset behavior does not depend on undocumented editor assumptions.
pub struct SrdDx9Renderer {
    device: IDirect3DDevice9,
    vertex_declaration: Option<IDirect3DVertexDeclaration9>,
    shaders: BTreeMap<[u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH], ShaderObjects>,
    vertex_buffer: Option<IDirect3DVertexBuffer9>,
    vertex_capacity: usize,
    composition_size: Option<[u32; 2]>,
    composition_target: Option<CompositionTarget>,
}

struct ShaderObjects {
    vertex: IDirect3DVertexShader9,
    pixel: IDirect3DPixelShader9,
}

struct CompositionTarget {
    texture: IDirect3DTexture9,
    surface: IDirect3DSurface9,
    size: [u32; 2],
}

pub struct D3d9BgraSurfaceReadback {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}

impl SrdDx9Renderer {
    pub fn new(device: &IDirect3DDevice9) -> Result<Self> {
        let mut renderer = Self {
            device: device.clone(),
            vertex_declaration: None,
            shaders: BTreeMap::new(),
            vertex_buffer: None,
            vertex_capacity: 0,
            composition_size: None,
            composition_target: None,
        };
        renderer.create_device_objects()?;
        Ok(renderer)
    }

    pub fn invalidate_device_objects(&mut self) {
        self.composition_target = None;
        self.vertex_buffer = None;
        self.vertex_capacity = 0;
        self.shaders.clear();
        self.vertex_declaration = None;
    }

    pub fn create_device_objects(&mut self) -> Result<()> {
        let elements = SRD_D3D9_VERTEX_DECLARATION.map(|element| D3DVERTEXELEMENT9 {
            Stream: element.stream,
            Offset: element.offset,
            Type: element.declaration_type,
            Method: element.method,
            Usage: element.usage,
            UsageIndex: element.usage_index,
        });
        unsafe {
            self.vertex_declaration = Some(self.device.CreateVertexDeclaration(elements.as_ptr())?);
            for key in EMBEDDED_SIMPLE_SHADER_KEYS {
                let pair = embedded_simple_shader_pair(&key)
                    .ok_or_else(|| Error::new(E_FAIL, "embedded SRD shader pair is missing"))?;
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
        self.vertex_buffer = Some(create_vertex_buffer(&self.device, 4)?);
        self.vertex_capacity = 4;
        if let Some([width, height]) = self.composition_size {
            self.composition_target = Some(create_composition_target(&self.device, width, height)?);
        }
        Ok(())
    }

    pub fn configure_composition_target(&mut self, width: u32, height: u32) -> Result<()> {
        let size = [width.max(1), height.max(1)];
        self.composition_size = Some(size);
        self.composition_target = Some(create_composition_target(&self.device, size[0], size[1])?);
        Ok(())
    }

    pub fn composition_size(&self) -> Option<[u32; 2]> {
        self.composition_target.as_ref().map(|target| target.size)
    }

    pub fn composition_texture(&self) -> Result<IDirect3DBaseTexture9> {
        self.composition_target
            .as_ref()
            .ok_or_else(|| Error::new(E_FAIL, "SRD composition target is not available"))?
            .texture
            .cast()
    }

    pub fn read_composition_pixel(&self, x: u32, y: u32) -> Result<[u8; 4]> {
        let target = self
            .composition_target
            .as_ref()
            .ok_or_else(|| Error::new(E_FAIL, "SRD composition target is not available"))?;
        read_render_target_pixel(&self.device, &target.surface, x, y)
    }

    pub fn read_composition_bgra(&self) -> Result<D3d9BgraSurfaceReadback> {
        let target = self
            .composition_target
            .as_ref()
            .ok_or_else(|| Error::new(E_FAIL, "SRD composition target is not available"))?;
        read_render_target_bgra(&self.device, &target.surface)
    }

    pub(crate) fn bind_composition_target(&self, clear_argb: u32) -> Result<RenderTargetGuard> {
        let target = self
            .composition_target
            .as_ref()
            .ok_or_else(|| Error::new(E_FAIL, "SRD composition target is not available"))?;
        let guard = RenderTargetGuard::bind(&self.device, &target.surface, target.size)?;
        unsafe {
            self.device
                .Clear(0, ptr::null(), D3DCLEAR_TARGET as u32, clear_argb, 1.0, 0)?;
        }
        Ok(guard)
    }

    pub fn render(
        &mut self,
        draws: &[EvidenceCompleteSrdDraw],
        external: SrdExternalRenderState,
        textures: Option<&SrdD3d9TextureSet>,
    ) -> Result<()> {
        if draws.is_empty() {
            return Ok(());
        }
        let state = StateBlockGuard::capture(&self.device)?;
        for draw in draws {
            self.render_vertices(draw, &draw.quad.vertices, external, textures)?;
        }
        state.restore()
    }

    pub fn render_triangle_strip(
        &mut self,
        draw: &EvidenceCompleteSrdDraw,
        vertices: &[SrdRenderVertex],
        external: SrdExternalRenderState,
        textures: Option<&SrdD3d9TextureSet>,
    ) -> Result<()> {
        let state = StateBlockGuard::capture(&self.device)?;
        self.render_vertices(draw, vertices, external, textures)?;
        state.restore()
    }

    fn render_vertices(
        &mut self,
        draw: &EvidenceCompleteSrdDraw,
        vertices: &[SrdRenderVertex],
        external: SrdExternalRenderState,
        textures: Option<&SrdD3d9TextureSet>,
    ) -> Result<()> {
        if vertices.len() < 3 {
            return Err(Error::new(
                E_INVALIDARG,
                "SRD triangle strip has fewer than three vertices",
            ));
        }
        if draw.packet.flags_0c & 0x100 != 0 {
            return Err(Error::new(
                E_INVALIDARG,
                "SRD draw requires the unported packet stencil override",
            ));
        }
        let pair = embedded_simple_shader_pair(&draw.shader_key).ok_or_else(|| {
            Error::new(
                E_INVALIDARG,
                "SRD draw requested a shader key without embedded D3D9 bytecode",
            )
        })?;
        self.require_loaded_pair(&pair)?;
        self.ensure_vertex_capacity(vertices.len())?;
        let declaration = self
            .vertex_declaration
            .as_ref()
            .ok_or_else(|| Error::new(E_FAIL, "SRD D3D9 vertex declaration is not available"))?;
        let shaders = self.shaders.get(&draw.shader_key).ok_or_else(|| {
            Error::new(
                E_FAIL,
                "SRD D3D9 shader objects are not available for this key",
            )
        })?;
        let vertex_buffer = self
            .vertex_buffer
            .as_ref()
            .ok_or_else(|| Error::new(E_FAIL, "SRD D3D9 vertex buffer is not available"))?;

        upload_vertices(vertex_buffer, vertices)?;
        let constants = draw.fixed_constants;
        let cull = draw
            .raster
            .cull_mode()
            .ok_or_else(|| Error::new(E_INVALIDARG, "invalid internal SRD cull mode"))?;
        let z_function = draw
            .depth
            .z_function()
            .ok_or_else(|| Error::new(E_INVALIDARG, "invalid internal SRD Z comparison"))?;
        let mut alpha_stencil = external.alpha_stencil;
        alpha_stencil.alpha_test_enabled = draw.blend.alpha_test_enabled;
        alpha_stencil.apply_draw_packet(draw.packet);
        let alpha_function = alpha_stencil
            .alpha_function()
            .ok_or_else(|| Error::new(E_INVALIDARG, "invalid internal SRD alpha comparison"))?;

        unsafe {
            self.device.SetVertexDeclaration(declaration)?;
            self.device
                .SetStreamSource(0, vertex_buffer, 0, SrdRenderVertex::STRIDE as u32)?;
            self.device.SetVertexShader(&shaders.vertex)?;
            self.device.SetPixelShader(&shaders.pixel)?;

            for (slot, binding) in draw.texture_bindings.iter().enumerate() {
                if let Some(binding) = binding {
                    let texture = textures
                        .and_then(|textures| textures.get(binding.texture_index))
                        .ok_or_else(|| {
                            Error::new(
                                E_FAIL,
                                format!(
                                    "SRD texture index {} is not loaded for slot {slot}",
                                    binding.texture_index
                                ),
                            )
                        })?;
                    self.device.SetTexture(slot as u32, &texture.texture)?;
                    self.device.SetSamplerState(
                        slot as u32,
                        D3DSAMP_ADDRESSU,
                        binding.sampler.address_u as u32,
                    )?;
                    self.device.SetSamplerState(
                        slot as u32,
                        D3DSAMP_ADDRESSV,
                        binding.sampler.address_v as u32,
                    )?;
                    self.device.SetSamplerState(
                        slot as u32,
                        D3DSAMP_MINFILTER,
                        binding.sampler.min_filter as u32,
                    )?;
                    self.device.SetSamplerState(
                        slot as u32,
                        D3DSAMP_MAGFILTER,
                        binding.sampler.mag_filter as u32,
                    )?;
                } else {
                    self.device.SetTexture(slot as u32, None)?;
                }
            }

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
            if draw.is_2d {
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

            self.device.SetRenderState(
                D3DRS_ALPHABLENDENABLE,
                u32::from(draw.blend.alpha_blend_enabled),
            )?;
            self.device
                .SetRenderState(D3DRS_SRCBLEND, draw.blend.source_blend as u32)?;
            self.device
                .SetRenderState(D3DRS_DESTBLEND, draw.blend.destination_blend as u32)?;
            self.device
                .SetRenderState(D3DRS_BLENDOP, draw.blend.blend_operation as u32)?;
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
                u32::from(draw.blend.separate_alpha_blend_enabled),
            )?;
            self.device
                .SetRenderState(D3DRS_SRCBLENDALPHA, draw.blend.source_blend_alpha as u32)?;
            self.device.SetRenderState(
                D3DRS_DESTBLENDALPHA,
                draw.blend.destination_blend_alpha as u32,
            )?;
            self.device
                .SetRenderState(D3DRS_BLENDOPALPHA, draw.blend.blend_operation_alpha as u32)?;
            self.device.SetRenderState(D3DRS_CULLMODE, cull as u32)?;
            self.device
                .SetRenderState(D3DRS_FILLMODE, draw.raster.fill_mode() as u32)?;
            self.device
                .SetRenderState(D3DRS_COLORWRITEENABLE, draw.raster.color_write_mask)?;
            self.device
                .SetRenderState(D3DRS_ZENABLE, u32::from(draw.depth.z_enabled))?;
            self.device
                .SetRenderState(D3DRS_ZWRITEENABLE, u32::from(draw.depth.z_write_enabled))?;
            self.device.SetRenderState(D3DRS_ZFUNC, z_function as u32)?;

            // This first subset has no packet stencil override. Scissor is a
            // proven external material context input and is therefore passed
            // explicitly instead of being inferred from SRD.
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
                D3DPT_TRIANGLESTRIP,
                0,
                u32::try_from(vertices.len() - 2).map_err(|_| {
                    Error::new(
                        E_INVALIDARG,
                        "SRD triangle-strip primitive count overflows u32",
                    )
                })?,
            )?;
        }
        Ok(())
    }

    fn ensure_vertex_capacity(&mut self, required: usize) -> Result<()> {
        if self.vertex_buffer.is_some() && self.vertex_capacity >= required {
            return Ok(());
        }
        let capacity = required.max(4).next_power_of_two();
        self.vertex_buffer = Some(create_vertex_buffer(&self.device, capacity)?);
        self.vertex_capacity = capacity;
        Ok(())
    }

    fn require_loaded_pair(&self, pair: &EmbeddedSimpleShaderPair) -> Result<()> {
        if self.shaders.is_empty() {
            return Err(Error::new(
                E_FAIL,
                "SRD D3D9 shader objects are not available",
            ));
        }
        if pair.vertex_shader.is_empty() || pair.pixel_shader.is_empty() {
            return Err(Error::new(
                E_INVALIDARG,
                "SRD D3D9 shader bytecode is empty",
            ));
        }
        Ok(())
    }
}

fn create_vertex_buffer(
    device: &IDirect3DDevice9,
    vertex_capacity: usize,
) -> Result<IDirect3DVertexBuffer9> {
    let byte_len = vertex_capacity
        .checked_mul(SrdRenderVertex::STRIDE)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| Error::new(E_INVALIDARG, "SRD vertex buffer size overflows u32"))?;
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
    buffer.ok_or_else(|| Error::new(E_FAIL, "CreateVertexBuffer returned null for SRD format 14"))
}

fn create_composition_target(
    device: &IDirect3DDevice9,
    width: u32,
    height: u32,
) -> Result<CompositionTarget> {
    let backbuffer = unsafe { device.GetRenderTarget(0)? };
    let mut description = Default::default();
    unsafe { backbuffer.GetDesc(&mut description)? };
    let mut texture = None;
    unsafe {
        device.CreateTexture(
            width,
            height,
            1,
            D3DUSAGE_RENDERTARGET as u32,
            description.Format,
            D3DPOOL_DEFAULT,
            &mut texture,
            ptr::null_mut(),
        )?;
    }
    let texture = texture
        .ok_or_else(|| Error::new(E_FAIL, "CreateTexture returned null for SRD composition"))?;
    let surface = unsafe { texture.GetSurfaceLevel(0)? };
    Ok(CompositionTarget {
        texture,
        surface,
        size: [width, height],
    })
}

fn read_render_target_pixel(
    device: &IDirect3DDevice9,
    render_target: &IDirect3DSurface9,
    x: u32,
    y: u32,
) -> Result<[u8; 4]> {
    let mut description = Default::default();
    unsafe { render_target.GetDesc(&mut description)? };
    if x >= description.Width || y >= description.Height {
        return Err(Error::new(
            E_INVALIDARG,
            format!(
                "composition pixel ({x}, {y}) is outside {}x{}",
                description.Width, description.Height
            ),
        ));
    }
    let mut staging = None;
    unsafe {
        device.CreateOffscreenPlainSurface(
            description.Width,
            description.Height,
            description.Format,
            D3DPOOL_SYSTEMMEM,
            &mut staging,
            ptr::null_mut(),
        )?;
    }
    let staging = staging.ok_or_else(|| {
        Error::new(
            E_FAIL,
            "CreateOffscreenPlainSurface returned null for composition readback",
        )
    })?;
    unsafe { device.GetRenderTargetData(render_target, &staging)? };
    let rectangle = RECT {
        left: x as i32,
        top: y as i32,
        right: x as i32 + 1,
        bottom: y as i32 + 1,
    };
    let mut locked = D3DLOCKED_RECT::default();
    unsafe { staging.LockRect(&mut locked, &rectangle, 0)? };
    let pixel = unsafe { *(locked.pBits.cast::<[u8; 4]>()) };
    unsafe { staging.UnlockRect()? };
    Ok(pixel)
}

fn read_render_target_bgra(
    device: &IDirect3DDevice9,
    render_target: &IDirect3DSurface9,
) -> Result<D3d9BgraSurfaceReadback> {
    let mut description = Default::default();
    unsafe { render_target.GetDesc(&mut description)? };
    let mut staging = None;
    unsafe {
        device.CreateOffscreenPlainSurface(
            description.Width,
            description.Height,
            description.Format,
            D3DPOOL_SYSTEMMEM,
            &mut staging,
            ptr::null_mut(),
        )?;
    }
    let staging = staging.ok_or_else(|| {
        Error::new(
            E_FAIL,
            "CreateOffscreenPlainSurface returned null for composition readback",
        )
    })?;
    unsafe { device.GetRenderTargetData(render_target, &staging)? };
    let mut locked = D3DLOCKED_RECT::default();
    unsafe { staging.LockRect(&mut locked, ptr::null(), 0)? };
    let row_bytes = description.Width as usize * 4;
    let mut bgra = vec![0u8; row_bytes * description.Height as usize];
    for row in 0..description.Height as usize {
        let source = unsafe {
            slice::from_raw_parts(
                locked.pBits.cast::<u8>().add(row * locked.Pitch as usize),
                row_bytes,
            )
        };
        bgra[row * row_bytes..][..row_bytes].copy_from_slice(source);
    }
    unsafe { staging.UnlockRect()? };
    Ok(D3d9BgraSurfaceReadback {
        width: description.Width,
        height: description.Height,
        bgra,
    })
}

fn upload_vertices(buffer: &IDirect3DVertexBuffer9, vertices: &[SrdRenderVertex]) -> Result<()> {
    debug_assert_eq!(mem::size_of::<SrdRenderVertex>(), SrdRenderVertex::STRIDE);
    let byte_len = vertices
        .len()
        .checked_mul(SrdRenderVertex::STRIDE)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| Error::new(E_INVALIDARG, "SRD vertex upload size overflows u32"))?;
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
    block: IDirect3DStateBlock9,
    restored: bool,
}

impl StateBlockGuard {
    fn capture(device: &IDirect3DDevice9) -> Result<Self> {
        let block = unsafe { device.CreateStateBlock(D3DSBT_ALL)? };
        unsafe { block.Capture()? };
        Ok(Self {
            block,
            restored: false,
        })
    }

    fn restore(mut self) -> Result<()> {
        unsafe { self.block.Apply()? };
        self.restored = true;
        Ok(())
    }
}

impl Drop for StateBlockGuard {
    fn drop(&mut self) {
        if !self.restored {
            let _ = unsafe { self.block.Apply() };
        }
    }
}

pub(crate) struct RenderTargetGuard {
    device: IDirect3DDevice9,
    render_target: IDirect3DSurface9,
    depth_stencil: Option<IDirect3DSurface9>,
    viewport: D3DVIEWPORT9,
}

impl RenderTargetGuard {
    fn bind(
        device: &IDirect3DDevice9,
        surface: &IDirect3DSurface9,
        size: [u32; 2],
    ) -> Result<Self> {
        let render_target = unsafe { device.GetRenderTarget(0)? };
        let depth_stencil = unsafe { device.GetDepthStencilSurface().ok() };
        let mut viewport = D3DVIEWPORT9::default();
        unsafe {
            device.GetViewport(&mut viewport)?;
            device.SetRenderTarget(0, surface)?;
            device.SetDepthStencilSurface(None)?;
            device.SetViewport(&D3DVIEWPORT9 {
                X: 0,
                Y: 0,
                Width: size[0],
                Height: size[1],
                MinZ: 0.0,
                MaxZ: 1.0,
            })?;
        }
        Ok(Self {
            device: device.clone(),
            render_target,
            depth_stencil,
            viewport,
        })
    }
}

impl Drop for RenderTargetGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.SetRenderTarget(0, &self.render_target);
            let _ = self
                .device
                .SetDepthStencilSurface(self.depth_stencil.as_ref());
            let _ = self.device.SetViewport(&self.viewport);
        }
    }
}
