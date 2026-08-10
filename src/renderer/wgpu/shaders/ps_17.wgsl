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

fn sampleTexture_0_(texCoord: vec4<f32>) -> vec4<f32> {
    let _e24: vec4<f32> = textureSample(s0_2d, s0_, vec2<f32>(texCoord.x, texCoord.y));
    return _e24;
}

fn sampleTexture_1_(texCoord_1: vec4<f32>) -> vec4<f32> {
    let _e24: vec4<f32> = textureSample(s1_2d, s1_, vec2<f32>(texCoord_1.x, texCoord_1.y));
    return _e24;
}

fn main_1() {
    let _e21: f32 = v3_texcoord2_1[0u];
    let _e23: f32 = v3_texcoord2_1[1u];
    let _e25: f32 = v3_texcoord2_1[2u];
    let _e27: f32 = v3_texcoord2_1[3u];
    let _e33: vec4<f32> = sampleTexture_1_(vec4<f32>((_e21 / _e27), (_e23 / _e27), (_e25 / _e27), (_e27 / _e27)));
    let _e36: f32 = v2_texcoord0_1[0u];
    let _e38: f32 = v2_texcoord0_1[1u];
    let _e40: f32 = v2_texcoord0_1[2u];
    let _e42: f32 = v2_texcoord0_1[3u];
    let _e44: vec4<f32> = sampleTexture_0_(vec4<f32>(_e36, _e38, _e40, _e42));
    let _e50: f32 = v0_color_1[0u];
    let _e52: f32 = v0_color_1[1u];
    let _e54: f32 = v0_color_1[2u];
    let _e56: f32 = v0_color_1[3u];
    let _e57: f32 = mul_legacy_f32_(_e44.w, _e56);
    let _e60: vec4<f32> = cF.m[2u];
    let _e65: f32 = v1_specular0_1[0u];
    let _e67: f32 = v1_specular0_1[1u];
    let _e69: f32 = v1_specular0_1[2u];
    let _e70: f32 = mad_legacy_f32_(_e44.x, _e50, _e65);
    let _e71: f32 = mad_legacy_f32_(_e44.y, _e52, _e67);
    let _e72: f32 = mad_legacy_f32_(_e44.z, _e54, _e69);
    let _e75: vec4<f32> = cF.m[0u];
    let _e80: f32 = mad_legacy_f32_((1f / (_e33.x + _e60.z)), _e60.w, -(_e27));
    let _e81: f32 = mul_legacy_f32_(_e80, (1f / _e75.x));
    let _e83: f32 = mul_legacy_f32_(_e57, clamp(_e81, 0f, 1f));
    let _e85: f32 = mul_legacy_f32_(_e70, _e75.z);
    let _e86: f32 = mul_legacy_f32_(_e71, _e75.z);
    let _e87: f32 = mul_legacy_f32_(_e72, _e75.z);
    let _e111: bool = (f32(clamp((clamp((clamp((f16(select(1f, 0f, ((999.9f - _e85) >= 0f))) + f16(select(1f, 0f, ((999.9f - _e86) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e87) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e83) >= 0f)))), 0h, 1h)) <= 0f);
    oC0_color[0u] = select(0f, _e85, _e111);
    oC0_color[1u] = select(0f, _e86, _e111);
    oC0_color[2u] = select(0f, _e87, _e111);
    oC0_color[3u] = select(0f, _e83, _e111);
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
