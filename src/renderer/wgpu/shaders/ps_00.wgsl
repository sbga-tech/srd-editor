// Generated from the exact embedded Ceylon SM3 bytecode; fixed-function alpha test is injected by the WebGPU backend.
enable f16;

struct cF_buf {
    m: array<vec4<f32>, 224>,
}

var<private> v0_color_1: vec4<f32>;
var<private> oC0_color: vec4<f32>;
@group(0) @binding(16) 
var<uniform> cF: cF_buf;

fn mul_legacy_f32_(a: f32, b: f32) -> f32 {
    return (select(a, 0f, (b == 0f)) * select(b, 0f, (a == 0f)));
}

fn main_1() {
    let _e14: f32 = v0_color_1[3u];
    let _e16: f32 = v0_color_1[0u];
    let _e18: f32 = v0_color_1[1u];
    let _e20: f32 = v0_color_1[2u];
    let _e23: vec4<f32> = cF.m[0u];
    let _e25: f32 = mul_legacy_f32_(_e16, _e23.z);
    let _e26: f32 = mul_legacy_f32_(_e18, _e23.z);
    let _e27: f32 = mul_legacy_f32_(_e20, _e23.z);
    let _e51: bool = (f32(clamp((clamp((clamp((f16(select(1f, 0f, ((999.9f - _e25) >= 0f))) + f16(select(1f, 0f, ((999.9f - _e26) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e27) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e14) >= 0f)))), 0h, 1h)) <= 0f);
    oC0_color[0u] = select(0f, _e25, _e51);
    oC0_color[1u] = select(0f, _e26, _e51);
    oC0_color[2u] = select(0f, _e27, _e51);
    oC0_color[3u] = select(0f, _e14, _e51);
    return;
}

@fragment 
fn main(@location(9) v0_color: vec4<f32>) -> @location(0) vec4<f32> {
    v0_color_1 = v0_color;
    main_1();
    let _e3: vec4<f32> = oC0_color;
    return _e3;
}
