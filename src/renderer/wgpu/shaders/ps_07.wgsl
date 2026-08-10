// Generated from the exact embedded Ceylon SM3 bytecode; fixed-function alpha test is injected by the WebGPU backend.
enable f16;

struct cF_buf {
    m: array<vec4<f32>, 224>,
}

var<private> v0_color_1: vec4<f32>;
var<private> v1_specular0_1: vec4<f32>;
var<private> oC0_color: vec4<f32>;
@group(0) @binding(16) 
var<uniform> cF: cF_buf;

fn mul_legacy_f32_(a: f32, b: f32) -> f32 {
    return (select(a, 0f, (b == 0f)) * select(b, 0f, (a == 0f)));
}

fn main_1() {
    let _e15: f32 = v1_specular0_1[0u];
    let _e17: f32 = v1_specular0_1[1u];
    let _e19: f32 = v1_specular0_1[2u];
    let _e21: f32 = v0_color_1[0u];
    let _e23: f32 = v0_color_1[1u];
    let _e25: f32 = v0_color_1[2u];
    let _e30: f32 = v0_color_1[3u];
    let _e33: vec4<f32> = cF.m[0u];
    let _e35: f32 = mul_legacy_f32_((_e21 + _e15), _e33.z);
    let _e36: f32 = mul_legacy_f32_((_e23 + _e17), _e33.z);
    let _e37: f32 = mul_legacy_f32_((_e25 + _e19), _e33.z);
    let _e61: bool = (f32(clamp((clamp((clamp((f16(select(1f, 0f, ((999.9f - _e35) >= 0f))) + f16(select(1f, 0f, ((999.9f - _e36) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e37) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e30) >= 0f)))), 0h, 1h)) <= 0f);
    oC0_color[0u] = select(0f, _e35, _e61);
    oC0_color[1u] = select(0f, _e36, _e61);
    oC0_color[2u] = select(0f, _e37, _e61);
    oC0_color[3u] = select(0f, _e30, _e61);
    return;
}

@fragment 
fn main(@location(9) v0_color: vec4<f32>, @location(10) v1_specular0_: vec4<f32>) -> @location(0) vec4<f32> {
    v0_color_1 = v0_color;
    v1_specular0_1 = v1_specular0_;
    main_1();
    let _e5: vec4<f32> = oC0_color;
    return _e5;
}
