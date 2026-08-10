// Generated from the exact embedded Ceylon SM3 bytecode; see tools/package_simple_shaders.rs.
struct cF_buf {
    m: array<vec4<f32>, 256>,
}

struct VertexOutput {
    @builtin(position) @invariant member: vec4<f32>,
    @location(9) member_1: vec4<f32>,
    @location(1) member_2: vec4<f32>,
}

var<private> o0_position0_: vec4<f32> = vec4<f32>(0f, 0f, 0f, 1f);
var<private> o1_color: vec4<f32>;
var<private> o2_texcoord0_: vec4<f32>;
var<private> v0_position0_1: vec4<f32>;
var<private> v1_texcoord0_1: vec4<f32>;
var<private> v2_color_1: vec4<f32>;
@group(0) @binding(0) 
var<uniform> cF: cF_buf;

fn mad_legacy_f32_(a: f32, b: f32, c: f32) -> f32 {
    return fma(select(a, 0f, (b == 0f)), select(b, 0f, (a == 0f)), c);
}

fn dp4_f32_legacy(a_1: vec4<f32>, b_1: vec4<f32>) -> f32 {
    return fma(select(a_1.w, 0f, (b_1.w == 0f)), select(b_1.w, 0f, (a_1.w == 0f)), fma(select(a_1.z, 0f, (b_1.z == 0f)), select(b_1.z, 0f, (a_1.z == 0f)), fma(select(a_1.y, 0f, (b_1.y == 0f)), select(b_1.y, 0f, (a_1.y == 0f)), (select(a_1.x, 0f, (b_1.x == 0f)) * select(b_1.x, 0f, (a_1.x == 0f))))));
}

fn main_1() {
    let _e19: f32 = v0_position0_1[0u];
    let _e21: f32 = v0_position0_1[1u];
    let _e23: f32 = v0_position0_1[2u];
    let _e24: vec4<f32> = vec4<f32>(_e19, _e21, _e23, 1f);
    let _e27: vec4<f32> = cF.m[1u];
    let _e28: f32 = dp4_f32_legacy(_e24, _e27);
    let _e31: vec4<f32> = cF.m[10u];
    let _e35: f32 = mad_legacy_f32_((0.5f - _e28), (1f / _e31.y), 1f);
    let _e38: vec4<f32> = cF.m[0u];
    let _e39: f32 = dp4_f32_legacy(_e24, _e38);
    let _e43: f32 = mad_legacy_f32_((_e39 - 0.5f), (1f / _e31.x), -1f);
    let _e46: vec4<f32> = cF.m[3u];
    let _e47: f32 = dp4_f32_legacy(_e24, _e46);
    let _e49: f32 = v2_color_1[0u];
    let _e51: f32 = v2_color_1[1u];
    let _e53: f32 = v2_color_1[2u];
    let _e55: f32 = v2_color_1[3u];
    let _e57: f32 = v1_texcoord0_1[0u];
    let _e59: f32 = v1_texcoord0_1[1u];
    let _e62: vec4<f32> = cF.m[8u];
    o0_position0_[0u] = _e43;
    o0_position0_[1u] = _e35;
    o0_position0_[2u] = 0f;
    o0_position0_[3u] = _e47;
    o1_color[0u] = _e49;
    o1_color[1u] = _e51;
    o1_color[2u] = _e53;
    o1_color[3u] = _e55;
    o2_texcoord0_[0u] = (_e57 + _e62.z);
    o2_texcoord0_[1u] = (_e59 + _e62.w);
    o2_texcoord0_[2u] = 0f;
    o2_texcoord0_[3u] = 0f;
    return;
}

@vertex 
fn main(@location(0) v0_position0_: vec4<f32>, @location(1) v1_texcoord0_: vec4<f32>, @location(2) v2_color: vec4<f32>) -> VertexOutput {
    v0_position0_1 = v0_position0_;
    v1_texcoord0_1 = v1_texcoord0_;
    v2_color_1 = v2_color;
    main_1();
    let _e12: vec4<f32> = o0_position0_;
    let _e13: vec4<f32> = o1_color;
    let _e14: vec4<f32> = o2_texcoord0_;
    return VertexOutput(_e12, _e13, _e14);
}
