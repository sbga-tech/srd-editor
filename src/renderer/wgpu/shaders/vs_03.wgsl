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
var<private> v2_color_1: vec4<f32>;
var<private> v3_specular0_1: vec4<f32>;
@group(0) @binding(0) 
var<uniform> cF: cF_buf;

fn dp4_f32_legacy(a: vec4<f32>, b: vec4<f32>) -> f32 {
    return fma(select(a.w, 0f, (b.w == 0f)), select(b.w, 0f, (a.w == 0f)), fma(select(a.z, 0f, (b.z == 0f)), select(b.z, 0f, (a.z == 0f)), fma(select(a.y, 0f, (b.y == 0f)), select(b.y, 0f, (a.y == 0f)), (select(a.x, 0f, (b.x == 0f)) * select(b.x, 0f, (a.x == 0f))))));
}

fn main_1() {
    let _e22: f32 = v0_position0_1[0u];
    let _e24: f32 = v0_position0_1[1u];
    let _e26: f32 = v0_position0_1[2u];
    let _e27: vec4<f32> = vec4<f32>(_e22, _e24, _e26, 1f);
    let _e30: vec4<f32> = cF.m[3u];
    let _e31: f32 = dp4_f32_legacy(_e27, _e30);
    let _e34: vec4<f32> = cF.m[2u];
    let _e35: f32 = dp4_f32_legacy(_e27, _e34);
    let _e38: vec4<f32> = cF.m[0u];
    let _e39: f32 = dp4_f32_legacy(_e27, _e38);
    let _e42: vec4<f32> = cF.m[1u];
    let _e43: f32 = dp4_f32_legacy(_e27, _e42);
    let _e44: vec4<f32> = vec4<f32>(_e39, _e43, _e35, _e31);
    let _e47: vec4<f32> = cF.m[13u];
    let _e48: f32 = dp4_f32_legacy(_e44, _e47);
    let _e51: vec4<f32> = cF.m[12u];
    let _e52: f32 = dp4_f32_legacy(_e44, _e51);
    let _e55: vec4<f32> = cF.m[11u];
    let _e56: f32 = dp4_f32_legacy(_e44, _e55);
    let _e59: vec4<f32> = cF.m[10u];
    let _e60: f32 = dp4_f32_legacy(_e44, _e59);
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
    o0_position0_[0u] = _e60;
    o0_position0_[1u] = _e56;
    o0_position0_[2u] = _e52;
    o0_position0_[3u] = _e48;
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
    return;
}

@vertex 
fn main(@location(0) v0_position0_: vec4<f32>, @location(1) v1_texcoord0_: vec4<f32>, @location(2) v2_color: vec4<f32>, @location(3) v3_specular0_: vec4<f32>) -> VertexOutput {
    v0_position0_1 = v0_position0_;
    v1_texcoord0_1 = v1_texcoord0_;
    v2_color_1 = v2_color;
    v3_specular0_1 = v3_specular0_;
    main_1();
    let _e15: vec4<f32> = o0_position0_;
    let _e16: vec4<f32> = o1_color;
    let _e17: vec4<f32> = o2_specular0_;
    let _e18: vec4<f32> = o3_texcoord0_;
    return VertexOutput(_e15, _e16, _e17, _e18);
}
