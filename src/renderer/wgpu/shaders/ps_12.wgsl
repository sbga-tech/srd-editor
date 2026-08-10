// Generated from the exact embedded Ceylon SM3 bytecode; fixed-function alpha test is injected by the WebGPU backend.
enable f16;

struct cF_buf {
    m: array<vec4<f32>, 224>,
}

@group(0) @binding(33) 
var s0_: sampler;
@group(0) @binding(32) 
var s0_2d: texture_2d<f32>;
@group(0) @binding(35) 
var s1_: sampler;
@group(0) @binding(34) 
var s1_2d: texture_2d<f32>;
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

fn sampleTexture_1_(texCoord: vec4<f32>) -> vec4<f32> {
    let _e23: vec4<f32> = textureSample(s1_2d, s1_, vec2<f32>(texCoord.x, texCoord.y));
    return _e23;
}

fn sampleTexture_0_(texCoord_1: vec4<f32>) -> vec4<f32> {
    let _e23: vec4<f32> = textureSample(s0_2d, s0_, vec2<f32>(texCoord_1.x, texCoord_1.y));
    return _e23;
}

fn main_1() {
    let _e20: f32 = v2_texcoord0_1[0u];
    let _e22: f32 = v2_texcoord0_1[1u];
    let _e24: f32 = v2_texcoord0_1[2u];
    let _e26: f32 = v2_texcoord0_1[3u];
    let _e28: vec4<f32> = sampleTexture_0_(vec4<f32>(_e20, _e22, _e24, _e26));
    let _e34: vec4<f32> = sampleTexture_1_(vec4<f32>(_e24, _e26, _e24, _e26));
    let _e36: f32 = mul_legacy_f32_(_e28.w, _e34.x);
    let _e38: f32 = v0_color_1[0u];
    let _e40: f32 = v0_color_1[1u];
    let _e42: f32 = v0_color_1[2u];
    let _e44: f32 = v0_color_1[3u];
    let _e45: f32 = mul_legacy_f32_(_e36, _e44);
    let _e47: f32 = v1_specular0_1[0u];
    let _e49: f32 = v1_specular0_1[1u];
    let _e51: f32 = v1_specular0_1[2u];
    let _e52: f32 = mad_legacy_f32_(_e28.x, _e38, _e47);
    let _e53: f32 = mad_legacy_f32_(_e28.y, _e40, _e49);
    let _e54: f32 = mad_legacy_f32_(_e28.z, _e42, _e51);
    let _e57: vec4<f32> = cF.m[0u];
    let _e59: f32 = mul_legacy_f32_(_e52, _e57.z);
    let _e60: f32 = mul_legacy_f32_(_e53, _e57.z);
    let _e61: f32 = mul_legacy_f32_(_e54, _e57.z);
    let _e85: bool = (f32(clamp((clamp((clamp((f16(select(1f, 0f, ((999.9f - _e59) >= 0f))) + f16(select(1f, 0f, ((999.9f - _e60) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e61) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e45) >= 0f)))), 0h, 1h)) <= 0f);
    oC0_color[0u] = select(0f, _e59, _e85);
    oC0_color[1u] = select(0f, _e60, _e85);
    oC0_color[2u] = select(0f, _e61, _e85);
    oC0_color[3u] = select(0f, _e45, _e85);
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
