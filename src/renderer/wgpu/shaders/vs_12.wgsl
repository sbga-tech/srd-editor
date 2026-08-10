// Generated from the exact embedded Ceylon SM3 bytecode; see tools/package_simple_shaders.rs.
struct cF_buf {
    m: array<vec4<f32>, 256>,
}

struct VertexOutput {
    @builtin(position) @invariant member: vec4<f32>,
    @location(9) member_1: vec4<f32>,
    @location(10) member_2: vec4<f32>,
    @location(1) member_3: vec4<f32>,
}

var<private> o0_position0_: vec4<f32> = vec4<f32>(0f, 0f, 0f, 1f);
var<private> o1_color: vec4<f32>;
var<private> o2_specular0_: vec4<f32>;
var<private> o3_texcoord0_: vec4<f32>;
var<private> v0_position0_1: vec4<f32>;
var<private> v1_texcoord0_1: vec4<f32>;
var<private> v2_texcoord1_1: vec4<f32>;
var<private> v3_color_1: vec4<f32>;
var<private> v4_specular0_1: vec4<f32>;
@group(0) @binding(0) 
var<uniform> cF: cF_buf;

fn mad_legacy_f32_(a: f32, b: f32, c: f32) -> f32 {
    return fma(select(a, 0f, (b == 0f)), select(b, 0f, (a == 0f)), c);
}

fn dp4_f32_legacy(a_1: vec4<f32>, b_1: vec4<f32>) -> f32 {
    return fma(select(a_1.w, 0f, (b_1.w == 0f)), select(b_1.w, 0f, (a_1.w == 0f)), fma(select(a_1.z, 0f, (b_1.z == 0f)), select(b_1.z, 0f, (a_1.z == 0f)), fma(select(a_1.y, 0f, (b_1.y == 0f)), select(b_1.y, 0f, (a_1.y == 0f)), (select(a_1.x, 0f, (b_1.x == 0f)) * select(b_1.x, 0f, (a_1.x == 0f))))));
}

fn main_1() {
    let _e23: f32 = v0_position0_1[0u];
    let _e25: f32 = v0_position0_1[1u];
    let _e27: f32 = v0_position0_1[2u];
    let _e28: vec4<f32> = vec4<f32>(_e23, _e25, _e27, 1f);
    let _e31: vec4<f32> = cF.m[1u];
    let _e32: f32 = dp4_f32_legacy(_e28, _e31);
    let _e35: vec4<f32> = cF.m[10u];
    let _e39: f32 = mad_legacy_f32_((0.5f - _e32), (1f / _e35.y), 1f);
    let _e42: vec4<f32> = cF.m[0u];
    let _e43: f32 = dp4_f32_legacy(_e28, _e42);
    let _e47: f32 = mad_legacy_f32_((_e43 - 0.5f), (1f / _e35.x), -1f);
    let _e50: vec4<f32> = cF.m[3u];
    let _e51: f32 = dp4_f32_legacy(_e28, _e50);
    let _e53: f32 = v3_color_1[0u];
    let _e55: f32 = v3_color_1[1u];
    let _e57: f32 = v3_color_1[2u];
    let _e59: f32 = v3_color_1[3u];
    let _e61: f32 = v4_specular0_1[0u];
    let _e63: f32 = v4_specular0_1[1u];
    let _e65: f32 = v4_specular0_1[2u];
    let _e67: f32 = v4_specular0_1[3u];
    let _e69: f32 = v2_texcoord1_1[0u];
    let _e71: f32 = v2_texcoord1_1[1u];
    let _e74: vec4<f32> = cF.m[9u];
    let _e80: f32 = v1_texcoord0_1[0u];
    let _e82: f32 = v1_texcoord0_1[1u];
    let _e85: vec4<f32> = cF.m[8u];
    o0_position0_[0u] = _e47;
    o0_position0_[1u] = _e39;
    o0_position0_[2u] = 0f;
    o0_position0_[3u] = _e51;
    o1_color[0u] = _e53;
    o1_color[1u] = _e55;
    o1_color[2u] = _e57;
    o1_color[3u] = _e59;
    o2_specular0_[0u] = _e61;
    o2_specular0_[1u] = _e63;
    o2_specular0_[2u] = _e65;
    o2_specular0_[3u] = _e67;
    o3_texcoord0_[0u] = (_e80 + _e85.z);
    o3_texcoord0_[1u] = (_e82 + _e85.w);
    o3_texcoord0_[2u] = (_e69 + _e74.x);
    o3_texcoord0_[3u] = (_e71 + _e74.y);
    return;
}

@vertex 
fn main(@location(0) v0_position0_: vec4<f32>, @location(1) v1_texcoord0_: vec4<f32>, @location(2) v2_texcoord1_: vec4<f32>, @location(3) v3_color: vec4<f32>, @location(4) v4_specular0_: vec4<f32>) -> VertexOutput {
    v0_position0_1 = v0_position0_;
    v1_texcoord0_1 = v1_texcoord0_;
    v2_texcoord1_1 = v2_texcoord1_;
    v3_color_1 = v3_color;
    v4_specular0_1 = v4_specular0_;
    main_1();
    let _e17: vec4<f32> = o0_position0_;
    let _e18: vec4<f32> = o1_color;
    let _e19: vec4<f32> = o2_specular0_;
    let _e20: vec4<f32> = o3_texcoord0_;
    return VertexOutput(_e17, _e18, _e19, _e20);
}
