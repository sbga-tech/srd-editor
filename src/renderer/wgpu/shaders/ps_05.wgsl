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
var<private> v3_texcoord2_1: vec4<f32>;
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
    let _e26: vec4<f32> = textureSample(s1_2d, s1_, vec2<f32>(texCoord.x, texCoord.y));
    return _e26;
}

fn sampleTexture_0_(texCoord_1: vec4<f32>) -> vec4<f32> {
    let _e26: vec4<f32> = textureSample(s0_2d, s0_, vec2<f32>(texCoord_1.x, texCoord_1.y));
    return _e26;
}

fn main_1() {
    let _e23: f32 = v2_texcoord0_1[0u];
    let _e25: f32 = v2_texcoord0_1[1u];
    let _e27: f32 = v2_texcoord0_1[2u];
    let _e29: f32 = v2_texcoord0_1[3u];
    let _e31: vec4<f32> = sampleTexture_0_(vec4<f32>(_e23, _e25, _e27, _e29));
    let _e36: f32 = v0_color_1[0u];
    let _e38: f32 = v0_color_1[1u];
    let _e40: f32 = v0_color_1[3u];
    let _e41: f32 = mul_legacy_f32_(_e31.w, _e40);
    let _e43: f32 = v1_specular0_1[0u];
    let _e45: f32 = v1_specular0_1[1u];
    let _e46: f32 = mad_legacy_f32_(_e31.x, _e36, _e43);
    let _e47: f32 = mad_legacy_f32_(_e31.y, _e38, _e45);
    let _e52: vec4<f32> = cF.m[2u];
    let _e57: f32 = mul_legacy_f32_((_e46 - 0.5f), (1f / _e52.x));
    let _e58: f32 = mul_legacy_f32_((_e47 - 0.5f), (1f / _e52.y));
    let _e59: f32 = mul_legacy_f32_(_e57, _e41);
    let _e60: f32 = mul_legacy_f32_(_e58, _e41);
    let _e61: f32 = (_e59 * 64f);
    let _e62: f32 = (_e60 * 64f);
    let _e64: f32 = v3_texcoord2_1[3u];
    let _e65: f32 = (1f / _e64);
    let _e67: f32 = v3_texcoord2_1[0u];
    let _e69: f32 = v3_texcoord2_1[1u];
    let _e70: f32 = mad_legacy_f32_(_e67, _e65, _e61);
    let _e71: f32 = mad_legacy_f32_(_e69, _e65, _e62);
    let _e73: vec4<f32> = sampleTexture_1_(vec4<f32>(_e70, _e71, _e61, _e62));
    let _e80: vec4<f32> = cF.m[0u];
    let _e82: f32 = mul_legacy_f32_(_e73.x, _e80.z);
    let _e83: f32 = mul_legacy_f32_(_e73.y, _e80.z);
    let _e84: f32 = mul_legacy_f32_(_e73.z, _e80.z);
    let _e108: bool = (f32(clamp((clamp((clamp((f16(select(1f, 0f, ((999.9f - _e82) >= 0f))) + f16(select(1f, 0f, ((999.9f - _e83) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e84) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e73.w) >= 0f)))), 0h, 1h)) <= 0f);
    oC0_color[0u] = select(0f, _e82, _e108);
    oC0_color[1u] = select(0f, _e83, _e108);
    oC0_color[2u] = select(0f, _e84, _e108);
    oC0_color[3u] = select(0f, _e73.w, _e108);
    return;
}

@fragment 
fn main(@location(9) v0_color: vec4<f32>, @location(10) v1_specular0_: vec4<f32>, @location(1) v2_texcoord0_: vec4<f32>, @location(3) v3_texcoord2_: vec4<f32>) -> @location(0) vec4<f32> {
    v0_color_1 = v0_color;
    v1_specular0_1 = v1_specular0_;
    v2_texcoord0_1 = v2_texcoord0_;
    v3_texcoord2_1 = v3_texcoord2_;
    main_1();
    let _e9: vec4<f32> = oC0_color;
    return _e9;
}
