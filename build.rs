use std::env;
use std::path::PathBuf;

const D3D9_BINDINGS: &[&str] = &[
    "HWND",
    "POINT",
    "RECT",
    "GetDesktopWindow",
    "D3D_SDK_VERSION",
    "D3DADAPTER_DEFAULT",
    "D3DCLEAR_STENCIL",
    "D3DCLEAR_TARGET",
    "D3DCLEAR_ZBUFFER",
    "D3DCREATE_HARDWARE_VERTEXPROCESSING",
    "D3DCREATE_SOFTWARE_VERTEXPROCESSING",
    "D3DDEVTYPE_HAL",
    "D3DDISPLAYMODEEX",
    "D3DFMT_A8R8G8B8",
    "D3DFMT_D24S8",
    "D3DFMT_UNKNOWN",
    "D3DFORMAT",
    "D3DLOCK_DISCARD",
    "D3DLOCKED_RECT",
    "D3DMULTISAMPLE_NONE",
    "D3DPOOL_DEFAULT",
    "D3DPOOL_SYSTEMMEM",
    "D3DPRESENT_INTERVAL_ONE",
    "D3DRECT",
    "D3DPRESENT_PARAMETERS",
    "D3DPT_TRIANGLELIST",
    "D3DPT_TRIANGLESTRIP",
    "D3DRS_ALPHABLENDENABLE",
    "D3DRS_ALPHAFUNC",
    "D3DRS_ALPHAREF",
    "D3DRS_ALPHATESTENABLE",
    "D3DRS_BLENDOP",
    "D3DRS_BLENDOPALPHA",
    "D3DRS_COLORWRITEENABLE",
    "D3DRS_CULLMODE",
    "D3DRS_DESTBLEND",
    "D3DRS_DESTBLENDALPHA",
    "D3DRS_FILLMODE",
    "D3DRS_SCISSORTESTENABLE",
    "D3DRS_SEPARATEALPHABLENDENABLE",
    "D3DRS_SRCBLEND",
    "D3DRS_SRCBLENDALPHA",
    "D3DRS_STENCILENABLE",
    "D3DRS_ZENABLE",
    "D3DRS_ZFUNC",
    "D3DRS_ZWRITEENABLE",
    "D3DSAMP_ADDRESSU",
    "D3DSAMP_ADDRESSV",
    "D3DSAMP_BORDERCOLOR",
    "D3DSAMP_MAGFILTER",
    "D3DSAMP_MAXANISOTROPY",
    "D3DSAMP_MAXMIPLEVEL",
    "D3DSAMP_MINFILTER",
    "D3DSAMP_MIPFILTER",
    "D3DSAMP_MIPMAPLODBIAS",
    "D3DSBT_ALL",
    "D3DSCANLINEORDERING",
    "D3DSURFACE_DESC",
    "D3DSWAPEFFECT_DISCARD",
    "D3DUSAGE_DYNAMIC",
    "D3DUSAGE_RENDERTARGET",
    "D3DUSAGE_WRITEONLY",
    "D3DVERTEXELEMENT9",
    "D3DVIEWPORT9",
    "IDirect3D9Ex",
    "IDirect3DBaseTexture9",
    "IDirect3DDevice9",
    "IDirect3DDevice9Ex",
    "IDirect3DPixelShader9",
    "IDirect3DStateBlock9",
    "IDirect3DSurface9",
    "IDirect3DTexture9",
    "IDirect3DVertexBuffer9",
    "IDirect3DVertexDeclaration9",
    "IDirect3DVertexShader9",
    "RGNDATA",
];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("Cargo did not set target OS");
    let needs_d3d9 = target_os == "windows"
        || (target_os == "linux" && env::var_os("CARGO_FEATURE_DXVK_NATIVE").is_some());
    if !needs_d3d9 {
        return;
    }

    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo did not set OUT_DIR"))
        .join("d3d9_bindings.rs");
    let mut args = vec![
        "--out".to_owned(),
        output.to_string_lossy().into_owned(),
        "--flat".to_owned(),
        "--no-allow".to_owned(),
        "--filter".to_owned(),
    ];
    args.extend(D3D9_BINDINGS.iter().map(|name| (*name).to_owned()));

    let _warnings = windows_bindgen::bindgen(args);
}
