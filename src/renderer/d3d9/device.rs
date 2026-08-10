use std::ptr;

use super::bindings::{
    BOOL, D3DADAPTER_DEFAULT, D3DCLEAR_STENCIL, D3DCLEAR_TARGET, D3DCLEAR_ZBUFFER,
    D3DCREATE_HARDWARE_VERTEXPROCESSING, D3DCREATE_SOFTWARE_VERTEXPROCESSING, D3DDEVTYPE_HAL,
    D3DFMT_D24S8, D3DFMT_UNKNOWN, D3DLOCKED_RECT, D3DMULTISAMPLE_NONE, D3DPOOL_SYSTEMMEM,
    D3DPRESENT_INTERVAL_ONE, D3DPRESENT_PARAMETERS, D3DSURFACE_DESC, D3DSWAPEFFECT_DISCARD, Error,
    HRESULT, HWND, IDirect3D9Ex, IDirect3DDevice9, IDirect3DDevice9Ex, Interface, RECT, Result,
};

use super::dxvk::DxvkProvider;
use crate::shader_bytecode::EmbeddedSimpleShaderPair;

const E_FAIL: HRESULT = HRESULT(0x8000_4005_u32 as i32);
const D3DERR_DEVICELOST: HRESULT = HRESULT(0x8876_0868_u32 as i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum D3d9ExFrameStatus {
    Presented,
    DeviceLost,
    Minimized,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum D3d9ExDeviceStatus {
    Ready,
    NeedsReset,
    DeviceLost,
    Minimized,
}

/// Owns a DXVK-backed Direct3D 9Ex interface, device, provider lifetime, and
/// reset parameters. DXVK is loaded explicitly on every supported platform;
/// this module never links or falls back to the Windows system `d3d9.dll`.
pub struct D3d9ExDevice {
    device: IDirect3DDevice9Ex,
    base_device: IDirect3DDevice9,
    _direct3d: IDirect3D9Ex,
    present: D3DPRESENT_PARAMETERS,
    hwnd: HWND,
    size: [u32; 2],
    reset_pending: bool,
    provider_label: String,
    _provider: DxvkProvider,
}

impl D3d9ExDevice {
    pub fn new_offscreen(size: [u32; 2]) -> Result<Self> {
        let provider = DxvkProvider::load().map_err(|message| Error::new(E_FAIL, message))?;
        let provider_label = provider.label();
        let hwnd = provider.window_handle();
        let size = [size[0].max(1), size[1].max(1)];
        let mut present = make_present_parameters(hwnd, size);
        let direct3d = provider
            .create_direct3d9_ex()
            .map_err(|message| Error::new(E_FAIL, message))?;

        let device = match create_device(
            &direct3d,
            hwnd,
            &mut present,
            D3DCREATE_HARDWARE_VERTEXPROCESSING as u32,
        ) {
            Ok(device) => device,
            Err(hardware_error) => create_device(
                &direct3d,
                hwnd,
                &mut present,
                D3DCREATE_SOFTWARE_VERTEXPROCESSING as u32,
            )
            .map_err(|software_error| {
                Error::new(software_error.code(), format!(
                    "DXVK IDirect3D9Ex::CreateDeviceEx failed with hardware vertex processing ({hardware_error}) and software vertex processing ({software_error})"
                ))
            })?,
        };
        let base_device = device.cast()?;

        Ok(Self {
            _direct3d: direct3d,
            device,
            base_device,
            present,
            hwnd,
            size,
            reset_pending: false,
            provider_label,
            _provider: provider,
        })
    }

    pub fn provider_label(&self) -> &str {
        &self.provider_label
    }

    pub fn device(&self) -> &IDirect3DDevice9 {
        &self.base_device
    }

    pub fn validate_shader_pair(&self, pair: &EmbeddedSimpleShaderPair) -> Result<()> {
        unsafe {
            let _vertex_shader = self
                .device
                .CreateVertexShader(pair.vertex_shader.as_ptr())?;
            let _pixel_shader = self.device.CreatePixelShader(pair.pixel_shader.as_ptr())?;
        }
        Ok(())
    }

    pub fn resize(&mut self, size: [u32; 2]) {
        self.size = size;
        if size[0] != 0 && size[1] != 0 {
            self.present.BackBufferWidth = size[0];
            self.present.BackBufferHeight = size[1];
            self.reset_pending = true;
        }
    }

    pub fn render_clear_frame(&mut self, clear_argb: u32) -> Result<D3d9ExFrameStatus> {
        match self.status()? {
            D3d9ExDeviceStatus::Ready => {}
            D3d9ExDeviceStatus::NeedsReset => self.reset()?,
            D3d9ExDeviceStatus::DeviceLost => return Ok(D3d9ExFrameStatus::DeviceLost),
            D3d9ExDeviceStatus::Minimized => return Ok(D3d9ExFrameStatus::Minimized),
        }

        self.clear_and_begin_scene(clear_argb)?;
        self.end_scene_and_present()
    }

    pub fn status(&mut self) -> Result<D3d9ExDeviceStatus> {
        if self.size[0] == 0 || self.size[1] == 0 {
            return Ok(D3d9ExDeviceStatus::Minimized);
        }

        if self.reset_pending {
            return Ok(D3d9ExDeviceStatus::NeedsReset);
        }
        match unsafe { self.device.CheckDeviceState(self.hwnd) } {
            Ok(()) => Ok(D3d9ExDeviceStatus::Ready),
            Err(error) if error.code() == D3DERR_DEVICELOST => Ok(D3d9ExDeviceStatus::DeviceLost),
            Err(error) => Err(error),
        }
    }

    pub fn reset(&mut self) -> Result<()> {
        unsafe { self.device.ResetEx(&mut self.present, ptr::null_mut())? };
        self.reset_pending = false;
        Ok(())
    }

    pub fn clear_and_begin_scene(&self, clear_argb: u32) -> Result<()> {
        unsafe {
            self.device.Clear(
                0,
                ptr::null(),
                (D3DCLEAR_TARGET | D3DCLEAR_ZBUFFER | D3DCLEAR_STENCIL) as u32,
                clear_argb,
                1.0,
                0,
            )?;
            self.device.BeginScene()
        }
    }

    pub fn begin_scene(&self) -> Result<()> {
        unsafe { self.device.BeginScene() }
    }

    pub fn end_scene(&self) -> Result<()> {
        unsafe { self.device.EndScene() }
    }

    /// Reads one raw BGRA/XRGB backbuffer texel through the documented
    /// render-target -> SYSTEMMEM transfer path. This is used only by the
    /// invisible SRD draw smoke test, after EndScene and before PresentEx.
    pub fn read_backbuffer_pixel(&self, x: u32, y: u32) -> Result<[u8; 4]> {
        let render_target = unsafe { self.device.GetRenderTarget(0)? };
        let mut description = D3DSURFACE_DESC::default();
        unsafe { render_target.GetDesc(&mut description)? };
        if x >= description.Width || y >= description.Height {
            return Err(Error::new(
                E_FAIL,
                format!(
                    "backbuffer pixel ({x}, {y}) is outside {}x{}",
                    description.Width, description.Height
                ),
            ));
        }

        let mut staging = None;
        unsafe {
            self.device.CreateOffscreenPlainSurface(
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
                "CreateOffscreenPlainSurface returned null for backbuffer readback",
            )
        })?;
        unsafe { self.device.GetRenderTargetData(&render_target, &staging)? };

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

    pub fn end_scene_and_present(&mut self) -> Result<D3d9ExFrameStatus> {
        self.end_scene()?;

        match unsafe {
            self.device
                .PresentEx(ptr::null(), ptr::null(), HWND::default(), ptr::null(), 0)
        } {
            Ok(()) => Ok(D3d9ExFrameStatus::Presented),
            Err(error) if error.code() == D3DERR_DEVICELOST => Ok(D3d9ExFrameStatus::DeviceLost),
            Err(error) => Err(error),
        }
    }
}

fn make_present_parameters(hwnd: HWND, size: [u32; 2]) -> D3DPRESENT_PARAMETERS {
    D3DPRESENT_PARAMETERS {
        BackBufferWidth: size[0].max(1),
        BackBufferHeight: size[1].max(1),
        BackBufferFormat: D3DFMT_UNKNOWN,
        BackBufferCount: 1,
        MultiSampleType: D3DMULTISAMPLE_NONE,
        MultiSampleQuality: 0,
        SwapEffect: D3DSWAPEFFECT_DISCARD,
        hDeviceWindow: hwnd,
        Windowed: BOOL(1),
        EnableAutoDepthStencil: BOOL(1),
        AutoDepthStencilFormat: D3DFMT_D24S8,
        Flags: 0,
        FullScreen_RefreshRateInHz: 0,
        PresentationInterval: D3DPRESENT_INTERVAL_ONE as u32,
    }
}

fn create_device(
    direct3d: &IDirect3D9Ex,
    hwnd: HWND,
    present: &mut D3DPRESENT_PARAMETERS,
    behavior_flags: u32,
) -> Result<IDirect3DDevice9Ex> {
    let mut device = None;
    unsafe {
        direct3d.CreateDeviceEx(
            D3DADAPTER_DEFAULT,
            D3DDEVTYPE_HAL,
            hwnd,
            behavior_flags,
            present,
            ptr::null_mut(),
            &mut device,
        )?;
    }
    device.ok_or_else(|| Error::new(E_FAIL, "CreateDeviceEx returned a null IDirect3DDevice9Ex"))
}
