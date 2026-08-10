// Generated from the exact embedded Ceylon SM3 bytecode; see tools/package_simple_shaders.rs.
struct cF_buf {
    m: array<vec4<f32>, 256>,
}

struct VertexOutput {
    @builtin(position) @invariant member: vec4<f32>,
    @location(9) member_1: vec4<f32>,
    @location(1) member_2: vec4<f32>,
    @location(3) member_3: vec4<f32>,
}

var<private> o0_position0_: vec4<f32> = vec4<f32>(0f, 0f, 0f, 1f);
var<private> o1_color: vec4<f32>;
var<private> o2_texcoord0_: vec4<f32>;
var<private> o3_texcoord2_: vec4<f32>;
var<private> v0_position0_1: vec4<f32>;
var<private> v1_texcoord0_1: vec4<f32>;
var<private> v2_color_1: vec4<f32>;
@group(0) @binding(0) 
var<uniform> cF: cF_buf;

fn mul_legacy_f32_(a: f32, b: f32) -> f32 {
    return (select(a, 0f, (b == 0f)) * select(b, 0f, (a == 0f)));
}

fn dp4_f32_legacy(a_1: vec4<f32>, b_1: vec4<f32>) -> f32 {
    return fma(select(a_1.w, 0f, (b_1.w == 0f)), select(b_1.w, 0f, (a_1.w == 0f)), fma(select(a_1.z, 0f, (b_1.z == 0f)), select(b_1.z, 0f, (a_1.z == 0f)), fma(select(a_1.y, 0f, (b_1.y == 0f)), select(b_1.y, 0f, (a_1.y == 0f)), (select(a_1.x, 0f, (b_1.x == 0f)) * select(b_1.x, 0f, (a_1.x == 0f))))));
}

fn main_1() {
    let _e25: f32 = v0_position0_1[0u];
    let _e27: f32 = v0_position0_1[1u];
    let _e29: f32 = v0_position0_1[2u];
    let _e30: vec4<f32> = vec4<f32>(_e25, _e27, _e29, 1f);
    let _e33: vec4<f32> = cF.m[0u];
    let _e34: f32 = dp4_f32_legacy(_e30, _e33);
    let _e37: vec4<f32> = cF.m[3u];
    let _e38: f32 = dp4_f32_legacy(_e30, _e37);
    let _e41: vec4<f32> = cF.m[2u];
    let _e42: f32 = dp4_f32_legacy(_e30, _e41);
    let _e45: vec4<f32> = cF.m[1u];
    let _e46: f32 = dp4_f32_legacy(_e30, _e45);
    let _e47: vec4<f32> = vec4<f32>(_e34, _e46, _e42, _e38);
    let _e50: vec4<f32> = cF.m[13u];
    let _e51: f32 = dp4_f32_legacy(_e47, _e50);
    let _e54: vec4<f32> = cF.m[12u];
    let _e55: f32 = dp4_f32_legacy(_e47, _e54);
    let _e58: vec4<f32> = cF.m[10u];
    let _e59: f32 = dp4_f32_legacy(_e47, _e58);
    let _e62: vec4<f32> = cF.m[11u];
    let _e63: f32 = dp4_f32_legacy(_e47, _e62);
    let _e64: f32 = (_e51 * 0.5f);
    let _e69: vec4<f32> = cF.m[14u];
    let _e74: f32 = mul_legacy_f32_(_e51, (1f / _e69.x));
    let _e75: f32 = mul_legacy_f32_(_e51, (1f / _e69.y));
    let _e79: f32 = v2_color_1[0u];
    let _e81: f32 = v2_color_1[1u];
    let _e83: f32 = v2_color_1[2u];
    let _e85: f32 = v2_color_1[3u];
    let _e87: f32 = v1_texcoord0_1[0u];
    let _e89: f32 = v1_texcoord0_1[1u];
    let _e92: vec4<f32> = cF.m[8u];
    o0_position0_[0u] = _e59;
    o0_position0_[1u] = _e63;
    o0_position0_[2u] = _e55;
    o0_position0_[3u] = _e51;
    o1_color[0u] = _e79;
    o1_color[1u] = _e81;
    o1_color[2u] = _e83;
    o1_color[3u] = _e85;
    o2_texcoord0_[0u] = (_e87 + _e92.z);
    o2_texcoord0_[1u] = (_e89 + _e92.w);
    o2_texcoord0_[2u] = 0f;
    o2_texcoord0_[3u] = 0f;
    o3_texcoord2_[0u] = fma(_e74, 0.25f, fma(_e59, 0.5f, _e64));
    o3_texcoord2_[1u] = fma(_e75, 0.25f, fma(_e63, -0.5f, _e64));
    o3_texcoord2_[2u] = _e55;
    o3_texcoord2_[3u] = _e51;
    return;
}

@vertex 
fn main(@location(0) v0_position0_: vec4<f32>, @location(1) v1_texcoord0_: vec4<f32>, @location(2) v2_color: vec4<f32>) -> VertexOutput {
    v0_position0_1 = v0_position0_;
    v1_texcoord0_1 = v1_texcoord0_;
    v2_color_1 = v2_color;
    main_1();
    let _e13: vec4<f32> = o0_position0_;
    let _e14: vec4<f32> = o1_color;
    let _e15: vec4<f32> = o2_texcoord0_;
    let _e16: vec4<f32> = o3_texcoord2_;
    return VertexOutput(_e13, _e14, _e15, _e16);
}
