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
    let _e31: f32 = v2_texcoord0_1[0u];
    let _e33: f32 = v2_texcoord0_1[1u];
    let _e35: vec4<f32> = sampleTexture_0_(vec4<f32>(_e31, _e33, _e21, _e23));
    let _e39: f32 = v0_color_1[0u];
    let _e41: f32 = v0_color_1[1u];
    let _e43: f32 = v0_color_1[2u];
    let _e45: f32 = v0_color_1[3u];
    let _e46: f32 = mul_legacy_f32_(fma(_e35.w, 0.00001f, _e25.w), _e45);
    let _e48: f32 = v1_specular0_1[0u];
    let _e50: f32 = v1_specular0_1[1u];
    let _e52: f32 = v1_specular0_1[2u];
    let _e53: f32 = mad_legacy_f32_(_e25.x, _e39, _e48);
    let _e54: f32 = mad_legacy_f32_(_e25.y, _e41, _e50);
    let _e55: f32 = mad_legacy_f32_(_e25.z, _e43, _e52);
    let _e58: vec4<f32> = cF.m[0u];
    let _e60: f32 = mul_legacy_f32_(_e53, _e58.z);
    let _e61: f32 = mul_legacy_f32_(_e54, _e58.z);
    let _e62: f32 = mul_legacy_f32_(_e55, _e58.z);
    let _e86: bool = (f32(clamp((clamp((clamp((f16(select(1f, 0f, ((999.9f - _e60) >= 0f))) + f16(select(1f, 0f, ((999.9f - _e61) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e62) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e46) >= 0f)))), 0h, 1h)) <= 0f);
    oC0_color[0u] = select(0f, _e60, _e86);
    oC0_color[1u] = select(0f, _e61, _e86);
    oC0_color[2u] = select(0f, _e62, _e86);
    oC0_color[3u] = select(0f, _e46, _e86);
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
