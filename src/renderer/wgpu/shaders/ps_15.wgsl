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
var<private> v1_texcoord0_1: vec4<f32>;
var<private> oC0_color: vec4<f32>;
@group(0) @binding(16) 
var<uniform> cF: cF_buf;

fn mul_legacy_f32_(a: f32, b: f32) -> f32 {
    return (select(a, 0f, (b == 0f)) * select(b, 0f, (a == 0f)));
}

fn sampleTexture_0_(texCoord: vec4<f32>) -> vec4<f32> {
    let _e20: vec4<f32> = textureSample(s0_2d, s0_, vec2<f32>(texCoord.x, texCoord.y));
    return _e20;
}

fn main_1() {
    let _e17: f32 = v1_texcoord0_1[0u];
    let _e19: f32 = v1_texcoord0_1[1u];
    let _e21: f32 = v1_texcoord0_1[2u];
    let _e23: f32 = v1_texcoord0_1[3u];
    let _e25: vec4<f32> = sampleTexture_0_(vec4<f32>(_e17, _e19, _e21, _e23));
    let _e31: f32 = v0_color_1[0u];
    let _e33: f32 = v0_color_1[1u];
    let _e35: f32 = v0_color_1[2u];
    let _e37: f32 = v0_color_1[3u];
    let _e38: f32 = mul_legacy_f32_(_e25.x, _e31);
    let _e39: f32 = mul_legacy_f32_(_e25.y, _e33);
    let _e40: f32 = mul_legacy_f32_(_e25.z, _e35);
    let _e41: f32 = mul_legacy_f32_(_e25.w, _e37);
    let _e44: vec4<f32> = cF.m[0u];
    let _e46: f32 = mul_legacy_f32_(_e38, _e44.z);
    let _e47: f32 = mul_legacy_f32_(_e39, _e44.z);
    let _e48: f32 = mul_legacy_f32_(_e40, _e44.z);
    let _e72: bool = (f32(clamp((clamp((clamp((f16(select(1f, 0f, ((999.9f - _e46) >= 0f))) + f16(select(1f, 0f, ((999.9f - _e47) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e48) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e41) >= 0f)))), 0h, 1h)) <= 0f);
    oC0_color[0u] = select(0f, _e46, _e72);
    oC0_color[1u] = select(0f, _e47, _e72);
    oC0_color[2u] = select(0f, _e48, _e72);
    oC0_color[3u] = select(0f, _e41, _e72);
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
