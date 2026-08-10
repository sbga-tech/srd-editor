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

fn mad_legacy_f32_(a_1: f32, b_1: f32, c: f32) -> f32 {
    return fma(select(a_1, 0f, (b_1 == 0f)), select(b_1, 0f, (a_1 == 0f)), c);
}

fn main_1() {
    let _e16: f32 = v1_specular0_1[0u];
    let _e18: f32 = v1_specular0_1[1u];
    let _e20: f32 = v1_specular0_1[2u];
    let _e22: f32 = v0_color_1[0u];
    let _e24: f32 = v0_color_1[1u];
    let _e26: f32 = v0_color_1[2u];
    let _e34: f32 = v0_color_1[3u];
    let _e35: f32 = mad_legacy_f32_(_e34, ((_e22 + _e16) - 1f), 1f);
    let _e36: f32 = mad_legacy_f32_(_e34, ((_e24 + _e18) - 1f), 1f);
    let _e37: f32 = mad_legacy_f32_(_e34, ((_e26 + _e20) - 1f), 1f);
    let _e40: vec4<f32> = cF.m[0u];
    let _e42: f32 = mul_legacy_f32_(_e35, _e40.z);
    let _e43: f32 = mul_legacy_f32_(_e36, _e40.z);
    let _e44: f32 = mul_legacy_f32_(_e37, _e40.z);
    let _e68: bool = (f32(clamp((clamp((clamp((f16(select(1f, 0f, ((999.9f - _e42) >= 0f))) + f16(select(1f, 0f, ((999.9f - _e43) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e44) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((-1f + 999.9f) >= 0f)))), 0h, 1h)) <= 0f);
    oC0_color[0u] = select(0f, _e42, _e68);
    oC0_color[1u] = select(0f, _e43, _e68);
    oC0_color[2u] = select(0f, _e44, _e68);
    oC0_color[3u] = select(0f, 1f, _e68);
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
