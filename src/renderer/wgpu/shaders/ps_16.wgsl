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
var<private> v1_texcoord0_1: vec4<f32>;
var<private> v2_texcoord2_1: vec4<f32>;
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
    let _e23: vec4<f32> = textureSample(s0_2d, s0_, vec2<f32>(texCoord.x, texCoord.y));
    return _e23;
}

fn sampleTexture_1_(texCoord_1: vec4<f32>) -> vec4<f32> {
    let _e23: vec4<f32> = textureSample(s1_2d, s1_, vec2<f32>(texCoord_1.x, texCoord_1.y));
    return _e23;
}

fn main_1() {
    let _e20: f32 = v2_texcoord2_1[0u];
    let _e22: f32 = v2_texcoord2_1[1u];
    let _e24: f32 = v2_texcoord2_1[2u];
    let _e26: f32 = v2_texcoord2_1[3u];
    let _e32: vec4<f32> = sampleTexture_1_(vec4<f32>((_e20 / _e26), (_e22 / _e26), (_e24 / _e26), (_e26 / _e26)));
    let _e36: vec4<f32> = cF.m[2u];
    let _e40: f32 = v1_texcoord0_1[0u];
    let _e42: f32 = v1_texcoord0_1[1u];
    let _e44: f32 = v1_texcoord0_1[2u];
    let _e46: f32 = v1_texcoord0_1[3u];
    let _e48: vec4<f32> = sampleTexture_0_(vec4<f32>(_e40, _e42, _e44, _e46));
    let _e54: f32 = v0_color_1[0u];
    let _e56: f32 = v0_color_1[1u];
    let _e58: f32 = v0_color_1[2u];
    let _e60: f32 = v0_color_1[3u];
    let _e61: f32 = mul_legacy_f32_(_e48.x, _e54);
    let _e62: f32 = mul_legacy_f32_(_e48.y, _e56);
    let _e63: f32 = mul_legacy_f32_(_e48.z, _e58);
    let _e64: f32 = mul_legacy_f32_(_e48.w, _e60);
    let _e68: vec4<f32> = cF.m[0u];
    let _e73: f32 = mad_legacy_f32_((1f / (_e32.x + _e36.z)), _e36.w, -(_e26));
    let _e74: f32 = mul_legacy_f32_(_e73, (1f / _e68.x));
    let _e76: f32 = mul_legacy_f32_(_e64, clamp(_e74, 0f, 1f));
    let _e78: f32 = mul_legacy_f32_(_e61, _e68.z);
    let _e79: f32 = mul_legacy_f32_(_e62, _e68.z);
    let _e80: f32 = mul_legacy_f32_(_e63, _e68.z);
    let _e104: bool = (f32(clamp((clamp((clamp((f16(select(1f, 0f, ((999.9f - _e78) >= 0f))) + f16(select(1f, 0f, ((999.9f - _e79) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e80) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e76) >= 0f)))), 0h, 1h)) <= 0f);
    oC0_color[0u] = select(0f, _e78, _e104);
    oC0_color[1u] = select(0f, _e79, _e104);
    oC0_color[2u] = select(0f, _e80, _e104);
    oC0_color[3u] = select(0f, _e76, _e104);
    return;
}

@fragment 
fn main(@location(9) v0_color: vec4<f32>, @location(1) v1_texcoord0_: vec4<f32>, @location(3) v2_texcoord2_: vec4<f32>) -> @location(0) vec4<f32> {
    v0_color_1 = v0_color;
    v1_texcoord0_1 = v1_texcoord0_;
    v2_texcoord2_1 = v2_texcoord2_;
    main_1();
    let _e7: vec4<f32> = oC0_color;
    return _e7;
}
