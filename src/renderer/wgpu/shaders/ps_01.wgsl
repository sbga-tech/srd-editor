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
var<private> oC0_color: vec4<f32>;
@group(0) @binding(16) 
var<uniform> cF: cF_buf;

fn mul_legacy_f32_(a: f32, b: f32) -> f32 {
    return (select(a, 0f, (b == 0f)) * select(b, 0f, (a == 0f)));
}

fn sampleTexture_0_(texCoord: vec4<f32>) -> vec4<f32> {
    let _e22: vec4<f32> = textureSample(s0_2d, s0_, vec2<f32>(texCoord.x, texCoord.y));
    return _e22;
}

fn sampleTexture_1_(texCoord_1: vec4<f32>) -> vec4<f32> {
    let _e22: vec4<f32> = textureSample(s1_2d, s1_, vec2<f32>(texCoord_1.x, texCoord_1.y));
    return _e22;
}

fn main_1() {
    let _e19: f32 = v1_texcoord0_1[2u];
    let _e21: f32 = v1_texcoord0_1[3u];
    let _e23: vec4<f32> = sampleTexture_1_(vec4<f32>(_e19, _e21, _e19, _e21));
    let _e29: f32 = v1_texcoord0_1[0u];
    let _e31: f32 = v1_texcoord0_1[1u];
    let _e33: vec4<f32> = sampleTexture_0_(vec4<f32>(_e29, _e31, _e19, _e21));
    let _e38: f32 = mul_legacy_f32_(_e33.x, _e23.x);
    let _e39: f32 = mul_legacy_f32_(_e33.y, _e23.y);
    let _e40: f32 = mul_legacy_f32_(_e33.z, _e23.z);
    let _e41: f32 = mul_legacy_f32_(_e33.w, _e23.w);
    let _e43: f32 = v0_color_1[0u];
    let _e45: f32 = v0_color_1[1u];
    let _e47: f32 = v0_color_1[2u];
    let _e49: f32 = v0_color_1[3u];
    let _e50: f32 = mul_legacy_f32_(_e38, _e43);
    let _e51: f32 = mul_legacy_f32_(_e39, _e45);
    let _e52: f32 = mul_legacy_f32_(_e40, _e47);
    let _e53: f32 = mul_legacy_f32_(_e41, _e49);
    let _e56: vec4<f32> = cF.m[0u];
    let _e58: f32 = mul_legacy_f32_(_e50, _e56.z);
    let _e59: f32 = mul_legacy_f32_(_e51, _e56.z);
    let _e60: f32 = mul_legacy_f32_(_e52, _e56.z);
    let _e84: bool = (f32(clamp((clamp((clamp((f16(select(1f, 0f, ((999.9f - _e58) >= 0f))) + f16(select(1f, 0f, ((999.9f - _e59) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e60) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e53) >= 0f)))), 0h, 1h)) <= 0f);
    oC0_color[0u] = select(0f, _e58, _e84);
    oC0_color[1u] = select(0f, _e59, _e84);
    oC0_color[2u] = select(0f, _e60, _e84);
    oC0_color[3u] = select(0f, _e53, _e84);
    return;
}

@fragment 
fn main(@location(9) v0_color: vec4<f32>, @location(1) v1_texcoord0_: vec4<f32>) -> @location(0) vec4<f32> {
    v0_color_1 = v0_color;
    v1_texcoord0_1 = v1_texcoord0_;
    main_1();
    let _e5: vec4<f32> = oC0_color;
    return _e5;
}
