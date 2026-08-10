// Generated from the exact embedded Ceylon SM3 bytecode; see tools/package_simple_shaders.rs.
struct cF_buf {
    m: array<vec4<f32>, 256>,
}

struct VertexOutput {
    @builtin(position) @invariant member: vec4<f32>,
    @location(9) member_1: vec4<f32>,
}

var<private> o0_position0_: vec4<f32> = vec4<f32>(0f, 0f, 0f, 1f);
var<private> o1_color: vec4<f32>;
var<private> v0_position0_1: vec4<f32>;
var<private> v1_color_1: vec4<f32>;
@group(0) @binding(0) 
var<uniform> cF: cF_buf;

fn mad_legacy_f32_(a: f32, b: f32, c: f32) -> f32 {
    return fma(select(a, 0f, (b == 0f)), select(b, 0f, (a == 0f)), c);
}

fn dp4_f32_legacy(a_1: vec4<f32>, b_1: vec4<f32>) -> f32 {
    return fma(select(a_1.w, 0f, (b_1.w == 0f)), select(b_1.w, 0f, (a_1.w == 0f)), fma(select(a_1.z, 0f, (b_1.z == 0f)), select(b_1.z, 0f, (a_1.z == 0f)), fma(select(a_1.y, 0f, (b_1.y == 0f)), select(b_1.y, 0f, (a_1.y == 0f)), (select(a_1.x, 0f, (b_1.x == 0f)) * select(b_1.x, 0f, (a_1.x == 0f))))));
}

fn main_1() {
    let _e16: f32 = v0_position0_1[0u];
    let _e18: f32 = v0_position0_1[1u];
    let _e20: f32 = v0_position0_1[2u];
    let _e21: vec4<f32> = vec4<f32>(_e16, _e18, _e20, 1f);
    let _e24: vec4<f32> = cF.m[1u];
    let _e25: f32 = dp4_f32_legacy(_e21, _e24);
    let _e28: vec4<f32> = cF.m[10u];
    let _e32: f32 = mad_legacy_f32_((0.5f - _e25), (1f / _e28.y), 1f);
    let _e35: vec4<f32> = cF.m[0u];
    let _e36: f32 = dp4_f32_legacy(_e21, _e35);
    let _e40: f32 = mad_legacy_f32_((_e36 - 0.5f), (1f / _e28.x), -1f);
    let _e43: vec4<f32> = cF.m[3u];
    let _e44: f32 = dp4_f32_legacy(_e21, _e43);
    let _e46: f32 = v1_color_1[0u];
    let _e48: f32 = v1_color_1[1u];
    let _e50: f32 = v1_color_1[2u];
    let _e52: f32 = v1_color_1[3u];
    o0_position0_[0u] = _e40;
    o0_position0_[1u] = _e32;
    o0_position0_[2u] = 0f;
    o0_position0_[3u] = _e44;
    o1_color[0u] = _e46;
    o1_color[1u] = _e48;
    o1_color[2u] = _e50;
    o1_color[3u] = _e52;
    return;
}

@vertex 
fn main(@location(0) v0_position0_: vec4<f32>, @location(1) v1_color: vec4<f32>) -> VertexOutput {
    v0_position0_1 = v0_position0_;
    v1_color_1 = v1_color;
    main_1();
    let _e9: vec4<f32> = o0_position0_;
    let _e10: vec4<f32> = o1_color;
    return VertexOutput(_e9, _e10);
}
