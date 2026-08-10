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
    let _e37: f32 = v0_color_1[0u];
    let _e39: f32 = v0_color_1[1u];
    let _e41: f32 = v0_color_1[2u];
    let _e43: f32 = v0_color_1[3u];
    let _e44: f32 = mul_legacy_f32_(_e31.w, _e43);
    let _e46: f32 = v3_texcoord2_1[0u];
    let _e48: f32 = v3_texcoord2_1[1u];
    let _e50: f32 = v3_texcoord2_1[2u];
    let _e52: f32 = v3_texcoord2_1[3u];
    let _e58: vec4<f32> = sampleTexture_1_(vec4<f32>((_e46 / _e52), (_e48 / _e52), (_e50 / _e52), (_e52 / _e52)));
    let _e64: f32 = v1_specular0_1[0u];
    let _e66: f32 = v1_specular0_1[1u];
    let _e68: f32 = v1_specular0_1[2u];
    let _e69: f32 = mad_legacy_f32_(_e31.x, _e37, _e64);
    let _e70: f32 = mad_legacy_f32_(_e31.y, _e39, _e66);
    let _e71: f32 = mad_legacy_f32_(_e31.z, _e41, _e68);
    let _e84: f32 = mul_legacy_f32_((1f - _e58.x), (1f - _e69));
    let _e85: f32 = mul_legacy_f32_((1f - _e58.y), (1f - _e70));
    let _e86: f32 = mul_legacy_f32_((1f - _e58.z), (1f - _e71));
    let _e87: f32 = mul_legacy_f32_((1f - _e58.w), (1f - _e44));
    let _e88: f32 = mul_legacy_f32_(_e58.x, _e69);
    let _e89: f32 = mul_legacy_f32_(_e58.y, _e70);
    let _e90: f32 = mul_legacy_f32_(_e58.z, _e71);
    let _e91: f32 = mul_legacy_f32_(_e58.w, _e44);
    let _e118: vec4<f32> = cF.m[0u];
    let _e120: f32 = mad_legacy_f32_(select((_e88 * 2f), fma(-(_e84), 2f, 1f), ((_e58.x - 0.5f) >= 0f)), _e118.z, -(_e58.x));
    let _e121: f32 = mad_legacy_f32_(select((_e89 * 2f), fma(-(_e85), 2f, 1f), ((_e58.y - 0.5f) >= 0f)), _e118.z, -(_e58.y));
    let _e122: f32 = mad_legacy_f32_(select((_e90 * 2f), fma(-(_e86), 2f, 1f), ((_e58.z - 0.5f) >= 0f)), _e118.z, -(_e58.z));
    let _e123: f32 = mad_legacy_f32_(select((_e91 * 2f), fma(-(_e87), 2f, 1f), ((_e58.w - 0.5f) >= 0f)), _e118.z, -(_e58.w));
    let _e124: f32 = mad_legacy_f32_(_e44, _e120, _e58.x);
    let _e125: f32 = mad_legacy_f32_(_e44, _e121, _e58.y);
    let _e126: f32 = mad_legacy_f32_(_e44, _e122, _e58.z);
    let _e127: f32 = mad_legacy_f32_(_e44, _e123, _e58.w);
    let _e129: f32 = mul_legacy_f32_(_e127, (1f / _e118.z));
    let _e153: bool = (f32(clamp((clamp((clamp((f16(select(1f, 0f, ((999.9f - _e124) >= 0f))) + f16(select(1f, 0f, ((999.9f - _e125) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e126) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e129) >= 0f)))), 0h, 1h)) <= 0f);
    oC0_color[0u] = select(0f, _e124, _e153);
    oC0_color[1u] = select(0f, _e125, _e153);
    oC0_color[2u] = select(0f, _e126, _e153);
    oC0_color[3u] = select(0f, _e129, _e153);
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
