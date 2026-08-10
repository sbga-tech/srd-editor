// Generated from the exact embedded Ceylon SM3 bytecode; fixed-function alpha test is injected by the WebGPU backend.
enable f16;

struct cF_buf {
    m: array<vec4<f32>, 224>,
}

@group(0) @binding(33) 
var s0_: sampler;
@group(0) @binding(32) 
var s0_2d: texture_2d<f32>;
var<private> v0_color_1: vec4<f32>;
var<private> v1_specular0_1: vec4<f32>;
var<private> v2_texcoord0_1: vec4<f32>;
var<private> oC0_color: vec4<f32>;
@group(0) @binding(16) 
var<uniform> cF: cF_buf;

fn mad_legacy_f32_(a: f32, b: f32, c: f32) -> f32 {
    return fma(select(a, 0f, (b == 0f)), select(b, 0f, (a == 0f)), c);
}

fn mul_legacy_f32_(a_1: f32, b_1: f32) -> f32 {
    return (select(a_1, 0f, (b_1 == 0f)) * select(b_1, 0f, (a_1 == 0f)));
}

fn sampleTexture_0_(texCoord: vec4<f32>) -> vec4<f32> {
    let _e21: vec4<f32> = textureSample(s0_2d, s0_, vec2<f32>(texCoord.x, texCoord.y));
    return _e21;
}

fn main_1() {
    let _e18: f32 = v2_texcoord0_1[0u];
    let _e20: f32 = v2_texcoord0_1[1u];
    let _e22: f32 = v2_texcoord0_1[2u];
    let _e24: f32 = v2_texcoord0_1[3u];
    let _e26: vec4<f32> = sampleTexture_0_(vec4<f32>(_e18, _e20, _e22, _e24));
    let _e32: f32 = v0_color_1[0u];
    let _e34: f32 = v0_color_1[1u];
    let _e36: f32 = v0_color_1[2u];
    let _e38: f32 = v0_color_1[3u];
    let _e39: f32 = mul_legacy_f32_(_e26.w, _e38);
    let _e41: f32 = v1_specular0_1[0u];
    let _e43: f32 = v1_specular0_1[1u];
    let _e45: f32 = v1_specular0_1[2u];
    let _e46: f32 = mad_legacy_f32_(_e26.x, _e32, _e41);
    let _e47: f32 = mad_legacy_f32_(_e26.y, _e34, _e43);
    let _e48: f32 = mad_legacy_f32_(_e26.z, _e36, _e45);
    let _e51: vec4<f32> = cF.m[0u];
    let _e53: f32 = mul_legacy_f32_(_e46, _e51.z);
    let _e54: f32 = mul_legacy_f32_(_e47, _e51.z);
    let _e55: f32 = mul_legacy_f32_(_e48, _e51.z);
    let _e79: bool = (f32(clamp((clamp((clamp((f16(select(1f, 0f, ((999.9f - _e53) >= 0f))) + f16(select(1f, 0f, ((999.9f - _e54) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e55) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e39) >= 0f)))), 0h, 1h)) <= 0f);
    oC0_color[0u] = select(0f, _e53, _e79);
    oC0_color[1u] = select(0f, _e54, _e79);
    oC0_color[2u] = select(0f, _e55, _e79);
    oC0_color[3u] = select(0f, _e39, _e79);
    return;
}

@fragment 
fn main(@location(9) v0_color: vec4<f32>, @location(10) v1_specular0_: vec4<f32>, @location(1) v2_texcoord0_: vec4<f32>) -> @location(0) vec4<f32> {
    v0_color_1 = v0_color;
    v1_specular0_1 = v1_specular0_;
    v2_texcoord0_1 = v2_texcoord0_;
    main_1();
    let _e7: vec4<f32> = oC0_color;
    return _e7;
}
