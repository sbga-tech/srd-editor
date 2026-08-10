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

fn sampleTexture_0_(texCoord: vec4<f32>) -> vec4<f32> {
    let _e24: vec4<f32> = textureSample(s0_2d, s0_, vec2<f32>(texCoord.x, texCoord.y));
    return _e24;
}

fn sampleTexture_1_(texCoord_1: vec4<f32>) -> vec4<f32> {
    let _e24: vec4<f32> = textureSample(s1_2d, s1_, vec2<f32>(texCoord_1.x, texCoord_1.y));
    return _e24;
}

fn main_1() {
    let _e21: f32 = v2_texcoord0_1[2u];
    let _e23: f32 = v2_texcoord0_1[3u];
    let _e25: vec4<f32> = sampleTexture_1_(vec4<f32>(_e21, _e23, _e21, _e23));
    let _e28: f32 = v2_texcoord0_1[0u];
    let _e30: f32 = v2_texcoord0_1[1u];
    let _e32: vec4<f32> = sampleTexture_0_(vec4<f32>(_e28, _e30, _e21, _e23));
    let _e37: f32 = v0_color_1[0u];
    let _e39: f32 = v0_color_1[1u];
    let _e41: f32 = v0_color_1[2u];
    let _e43: f32 = v0_color_1[3u];
    let _e44: f32 = mul_legacy_f32_(_e25.x, _e43);
    let _e46: f32 = v1_specular0_1[0u];
    let _e48: f32 = v1_specular0_1[1u];
    let _e50: f32 = v1_specular0_1[2u];
    let _e51: f32 = mad_legacy_f32_(_e32.x, _e37, _e46);
    let _e52: f32 = mad_legacy_f32_(_e32.y, _e39, _e48);
    let _e53: f32 = mad_legacy_f32_(_e32.z, _e41, _e50);
    let _e57: f32 = mad_legacy_f32_(_e44, (_e51 - 1f), 1f);
    let _e58: f32 = mad_legacy_f32_(_e44, (_e52 - 1f), 1f);
    let _e59: f32 = mad_legacy_f32_(_e44, (_e53 - 1f), 1f);
    let _e62: vec4<f32> = cF.m[0u];
    let _e64: f32 = mul_legacy_f32_(_e57, _e62.z);
    let _e65: f32 = mul_legacy_f32_(_e58, _e62.z);
    let _e66: f32 = mul_legacy_f32_(_e59, _e62.z);
    let _e90: bool = (f32(clamp((clamp((clamp((f16(select(1f, 0f, ((999.9f - _e64) >= 0f))) + f16(select(1f, 0f, ((999.9f - _e65) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e66) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((-1f + 999.9f) >= 0f)))), 0h, 1h)) <= 0f);
    oC0_color[0u] = select(0f, _e64, _e90);
    oC0_color[1u] = select(0f, _e65, _e90);
    oC0_color[2u] = select(0f, _e66, _e90);
    oC0_color[3u] = select(0f, 1f, _e90);
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
