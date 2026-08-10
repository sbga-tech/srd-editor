// Generated from the exact embedded Ceylon SM3 bytecode; see tools/package_simple_shaders.rs.
struct cF_buf {
    m: array<vec4<f32>, 256>,
}

struct VertexOutput {
    @builtin(position) @invariant member: vec4<f32>,
    @location(9) member_1: vec4<f32>,
    @location(10) member_2: vec4<f32>,
    @location(1) member_3: vec4<f32>,
    @location(3) member_4: vec4<f32>,
}

var<private> o0_position0_: vec4<f32> = vec4<f32>(0f, 0f, 0f, 1f);
var<private> o1_color: vec4<f32>;
var<private> o2_specular0_: vec4<f32>;
var<private> o3_texcoord0_: vec4<f32>;
var<private> o4_texcoord2_: vec4<f32>;
var<private> v0_position0_1: vec4<f32>;
var<private> v1_texcoord0_1: vec4<f32>;
var<private> v2_color_1: vec4<f32>;
var<private> v3_specular0_1: vec4<f32>;
@group(0) @binding(0) 
var<uniform> cF: cF_buf;

fn mul_legacy_f32_(a: f32, b: f32) -> f32 {
    return (select(a, 0f, (b == 0f)) * select(b, 0f, (a == 0f)));
}

fn mad_legacy_f32_(a_1: f32, b_1: f32, c: f32) -> f32 {
    return fma(select(a_1, 0f, (b_1 == 0f)), select(b_1, 0f, (a_1 == 0f)), c);
}

fn dp4_f32_legacy(a_2: vec4<f32>, b_2: vec4<f32>) -> f32 {
    return fma(select(a_2.w, 0f, (b_2.w == 0f)), select(b_2.w, 0f, (a_2.w == 0f)), fma(select(a_2.z, 0f, (b_2.z == 0f)), select(b_2.z, 0f, (a_2.z == 0f)), fma(select(a_2.y, 0f, (b_2.y == 0f)), select(b_2.y, 0f, (a_2.y == 0f)), (select(a_2.x, 0f, (b_2.x == 0f)) * select(b_2.x, 0f, (a_2.x == 0f))))));
}

fn main_1() {
    let _e24: f32 = v0_position0_1[0u];
    let _e26: f32 = v0_position0_1[1u];
    let _e28: f32 = v0_position0_1[2u];
    let _e29: vec4<f32> = vec4<f32>(_e24, _e26, _e28, 1f);
    let _e32: vec4<f32> = cF.m[1u];
    let _e33: f32 = dp4_f32_legacy(_e29, _e32);
    let _e36: vec4<f32> = cF.m[10u];
    let _e41: f32 = mad_legacy_f32_((0.5f - _e33), (1f / _e36.y), 1f);
    let _e44: vec4<f32> = cF.m[3u];
    let _e45: f32 = dp4_f32_legacy(_e29, _e44);
    let _e48: vec4<f32> = cF.m[0u];
    let _e49: f32 = dp4_f32_legacy(_e29, _e48);
    let _e51: f32 = (1f / _e36.x);
    let _e53: f32 = mad_legacy_f32_((_e49 - 0.5f), _e51, -1f);
    let _e54: f32 = (_e45 * 0.5f);
    let _e57: f32 = mul_legacy_f32_(_e45, _e51);
    let _e58: f32 = mul_legacy_f32_(_e45, (1f / _e36.y));
    let _e62: f32 = v2_color_1[0u];
    let _e64: f32 = v2_color_1[1u];
    let _e66: f32 = v2_color_1[2u];
    let _e68: f32 = v2_color_1[3u];
    let _e70: f32 = v3_specular0_1[0u];
    let _e72: f32 = v3_specular0_1[1u];
    let _e74: f32 = v3_specular0_1[2u];
    let _e76: f32 = v3_specular0_1[3u];
    let _e78: f32 = v1_texcoord0_1[0u];
    let _e80: f32 = v1_texcoord0_1[1u];
    let _e83: vec4<f32> = cF.m[8u];
    o0_position0_[0u] = _e53;
    o0_position0_[1u] = _e41;
    o0_position0_[2u] = 0f;
    o0_position0_[3u] = _e45;
    o1_color[0u] = _e62;
    o1_color[1u] = _e64;
    o1_color[2u] = _e66;
    o1_color[3u] = _e68;
    o2_specular0_[0u] = _e70;
    o2_specular0_[1u] = _e72;
    o2_specular0_[2u] = _e74;
    o2_specular0_[3u] = _e76;
    o3_texcoord0_[0u] = (_e78 + _e83.z);
    o3_texcoord0_[1u] = (_e80 + _e83.w);
    o3_texcoord0_[2u] = 0f;
    o3_texcoord0_[3u] = 0f;
    o4_texcoord2_[0u] = fma(_e57, 0.25f, fma(_e53, 0.5f, _e54));
    o4_texcoord2_[1u] = fma(_e58, 0.25f, fma(_e41, -0.5f, _e54));
    o4_texcoord2_[2u] = 0f;
    o4_texcoord2_[3u] = _e45;
    return;
}

@vertex 
fn main(@location(0) v0_position0_: vec4<f32>, @location(1) v1_texcoord0_: vec4<f32>, @location(2) v2_color: vec4<f32>, @location(3) v3_specular0_: vec4<f32>) -> VertexOutput {
    v0_position0_1 = v0_position0_;
    v1_texcoord0_1 = v1_texcoord0_;
    v2_color_1 = v2_color;
    v3_specular0_1 = v3_specular0_;
    main_1();
    let _e16: vec4<f32> = o0_position0_;
    let _e17: vec4<f32> = o1_color;
    let _e18: vec4<f32> = o2_specular0_;
    let _e19: vec4<f32> = o3_texcoord0_;
    let _e20: vec4<f32> = o4_texcoord2_;
    return VertexOutput(_e16, _e17, _e18, _e19, _e20);
}
