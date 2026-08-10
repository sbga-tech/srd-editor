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

fn dp4_f32_legacy(a: vec4<f32>, b: vec4<f32>) -> f32 {
    return fma(select(a.w, 0f, (b.w == 0f)), select(b.w, 0f, (a.w == 0f)), fma(select(a.z, 0f, (b.z == 0f)), select(b.z, 0f, (a.z == 0f)), fma(select(a.y, 0f, (b.y == 0f)), select(b.y, 0f, (a.y == 0f)), (select(a.x, 0f, (b.x == 0f)) * select(b.x, 0f, (a.x == 0f))))));
}

fn main_1() {
    let _e20: f32 = v0_position0_1[0u];
    let _e22: f32 = v0_position0_1[1u];
    let _e24: f32 = v0_position0_1[2u];
    let _e25: vec4<f32> = vec4<f32>(_e20, _e22, _e24, 1f);
    let _e28: vec4<f32> = cF.m[3u];
    let _e29: f32 = dp4_f32_legacy(_e25, _e28);
    let _e32: vec4<f32> = cF.m[2u];
    let _e33: f32 = dp4_f32_legacy(_e25, _e32);
    let _e36: vec4<f32> = cF.m[0u];
    let _e37: f32 = dp4_f32_legacy(_e25, _e36);
    let _e40: vec4<f32> = cF.m[1u];
    let _e41: f32 = dp4_f32_legacy(_e25, _e40);
    let _e42: vec4<f32> = vec4<f32>(_e37, _e41, _e33, _e29);
    let _e45: vec4<f32> = cF.m[13u];
    let _e46: f32 = dp4_f32_legacy(_e42, _e45);
    let _e49: vec4<f32> = cF.m[12u];
    let _e50: f32 = dp4_f32_legacy(_e42, _e49);
    let _e53: vec4<f32> = cF.m[11u];
    let _e54: f32 = dp4_f32_legacy(_e42, _e53);
    let _e57: vec4<f32> = cF.m[10u];
    let _e58: f32 = dp4_f32_legacy(_e42, _e57);
    let _e60: f32 = v2_color_1[0u];
    let _e62: f32 = v2_color_1[1u];
    let _e64: f32 = v2_color_1[2u];
    let _e66: f32 = v2_color_1[3u];
    let _e68: f32 = v1_texcoord0_1[0u];
    let _e70: f32 = v1_texcoord0_1[1u];
    let _e73: vec4<f32> = cF.m[8u];
    o0_position0_[0u] = _e58;
    o0_position0_[1u] = _e54;
    o0_position0_[2u] = _e50;
    o0_position0_[3u] = _e46;
    o1_color[0u] = _e60;
    o1_color[1u] = _e62;
    o1_color[2u] = _e64;
    o1_color[3u] = _e66;
    o2_texcoord0_[0u] = (_e68 + _e73.z);
    o2_texcoord0_[1u] = (_e70 + _e73.w);
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
