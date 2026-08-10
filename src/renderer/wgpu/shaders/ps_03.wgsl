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
    let _e24: vec4<f32> = textureSample(s1_2d, s1_, vec2<f32>(texCoord.x, texCoord.y));
    return _e24;
}

fn sampleTexture_0_(texCoord_1: vec4<f32>) -> vec4<f32> {
    let _e24: vec4<f32> = textureSample(s0_2d, s0_, vec2<f32>(texCoord_1.x, texCoord_1.y));
    return _e24;
}

fn main_1() {
    let _e21: f32 = v2_texcoord0_1[0u];
    let _e23: f32 = v2_texcoord0_1[1u];
    let _e25: f32 = v2_texcoord0_1[2u];
    let _e27: f32 = v2_texcoord0_1[3u];
    let _e29: vec4<f32> = sampleTexture_0_(vec4<f32>(_e21, _e23, _e25, _e27));
    let _e35: f32 = v0_color_1[0u];
    let _e37: f32 = v0_color_1[1u];
    let _e39: f32 = v0_color_1[2u];
    let _e41: f32 = v0_color_1[3u];
    let _e42: f32 = mul_legacy_f32_(_e29.w, _e41);
    let _e44: f32 = v3_texcoord2_1[0u];
    let _e46: f32 = v3_texcoord2_1[1u];
    let _e48: f32 = v3_texcoord2_1[2u];
    let _e50: f32 = v3_texcoord2_1[3u];
    let _e56: vec4<f32> = sampleTexture_1_(vec4<f32>((_e44 / _e50), (_e46 / _e50), (_e48 / _e50), (_e50 / _e50)));
    let _e62: f32 = v1_specular0_1[0u];
    let _e64: f32 = v1_specular0_1[1u];
    let _e66: f32 = v1_specular0_1[2u];
    let _e67: f32 = mad_legacy_f32_(_e29.x, _e35, _e62);
    let _e68: f32 = mad_legacy_f32_(_e29.y, _e37, _e64);
    let _e69: f32 = mad_legacy_f32_(_e29.z, _e39, _e66);
    let _e82: f32 = mad_legacy_f32_((_e56.x - 1f), (1f - _e67), 1f);
    let _e83: f32 = mad_legacy_f32_((_e56.y - 1f), (1f - _e68), 1f);
    let _e84: f32 = mad_legacy_f32_((_e56.z - 1f), (1f - _e69), 1f);
    let _e85: f32 = mad_legacy_f32_((_e56.w - 1f), (1f - _e42), 1f);
    let _e88: vec4<f32> = cF.m[0u];
    let _e90: f32 = mad_legacy_f32_(_e82, _e88.z, -(_e56.x));
    let _e91: f32 = mad_legacy_f32_(_e83, _e88.z, -(_e56.y));
    let _e92: f32 = mad_legacy_f32_(_e84, _e88.z, -(_e56.z));
    let _e93: f32 = mad_legacy_f32_(_e85, _e88.z, -(_e56.w));
    let _e94: f32 = mad_legacy_f32_(_e42, _e90, _e56.x);
    let _e95: f32 = mad_legacy_f32_(_e42, _e91, _e56.y);
    let _e96: f32 = mad_legacy_f32_(_e42, _e92, _e56.z);
    let _e97: f32 = mad_legacy_f32_(_e42, _e93, _e56.w);
    let _e99: f32 = mul_legacy_f32_(_e97, (1f / _e88.z));
    let _e123: bool = (f32(clamp((clamp((clamp((f16(select(1f, 0f, ((999.9f - _e94) >= 0f))) + f16(select(1f, 0f, ((999.9f - _e95) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e96) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e99) >= 0f)))), 0h, 1h)) <= 0f);
    oC0_color[0u] = select(0f, _e94, _e123);
    oC0_color[1u] = select(0f, _e95, _e123);
    oC0_color[2u] = select(0f, _e96, _e123);
    oC0_color[3u] = select(0f, _e99, _e123);
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
