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

fn dp4_f32_legacy(a: vec4<f32>, b: vec4<f32>) -> f32 {
    return fma(select(a.w, 0f, (b.w == 0f)), select(b.w, 0f, (a.w == 0f)), fma(select(a.z, 0f, (b.z == 0f)), select(b.z, 0f, (a.z == 0f)), fma(select(a.y, 0f, (b.y == 0f)), select(b.y, 0f, (a.y == 0f)), (select(a.x, 0f, (b.x == 0f)) * select(b.x, 0f, (a.x == 0f))))));
}

fn main_1() {
    let _e24: f32 = v0_position0_1[0u];
    let _e26: f32 = v0_position0_1[1u];
    let _e28: f32 = v0_position0_1[2u];
    let _e29: vec4<f32> = vec4<f32>(_e24, _e26, _e28, 1f);
    let _e32: vec4<f32> = cF.m[3u];
    let _e33: f32 = dp4_f32_legacy(_e29, _e32);
    let _e36: vec4<f32> = cF.m[2u];
    let _e37: f32 = dp4_f32_legacy(_e29, _e36);
    let _e40: vec4<f32> = cF.m[0u];
    let _e41: f32 = dp4_f32_legacy(_e29, _e40);
    let _e44: vec4<f32> = cF.m[1u];
    let _e45: f32 = dp4_f32_legacy(_e29, _e44);
    let _e46: vec4<f32> = vec4<f32>(_e41, _e45, _e37, _e33);
    let _e49: vec4<f32> = cF.m[13u];
    let _e50: f32 = dp4_f32_legacy(_e46, _e49);
    let _e53: vec4<f32> = cF.m[12u];
    let _e54: f32 = dp4_f32_legacy(_e46, _e53);
    let _e57: vec4<f32> = cF.m[11u];
    let _e58: f32 = dp4_f32_legacy(_e46, _e57);
    let _e61: vec4<f32> = cF.m[10u];
    let _e62: f32 = dp4_f32_legacy(_e46, _e61);
    let _e64: f32 = v3_color_1[0u];
    let _e66: f32 = v3_color_1[1u];
    let _e68: f32 = v3_color_1[2u];
    let _e70: f32 = v3_color_1[3u];
    let _e72: f32 = v4_specular0_1[0u];
    let _e74: f32 = v4_specular0_1[1u];
    let _e76: f32 = v4_specular0_1[2u];
    let _e78: f32 = v4_specular0_1[3u];
    let _e80: f32 = v2_texcoord1_1[0u];
    let _e82: f32 = v2_texcoord1_1[1u];
    let _e85: vec4<f32> = cF.m[9u];
    let _e91: f32 = v1_texcoord0_1[0u];
    let _e93: f32 = v1_texcoord0_1[1u];
    let _e96: vec4<f32> = cF.m[8u];
    o0_position0_[0u] = _e62;
    o0_position0_[1u] = _e58;
    o0_position0_[2u] = _e54;
    o0_position0_[3u] = _e50;
    o1_color[0u] = _e64;
    o1_color[1u] = _e66;
    o1_color[2u] = _e68;
    o1_color[3u] = _e70;
    o2_specular0_[0u] = _e72;
    o2_specular0_[1u] = _e74;
    o2_specular0_[2u] = _e76;
    o2_specular0_[3u] = _e78;
    o3_texcoord0_[0u] = (_e91 + _e96.z);
    o3_texcoord0_[1u] = (_e93 + _e96.w);
    o3_texcoord0_[2u] = (_e80 + _e85.x);
    o3_texcoord0_[3u] = (_e82 + _e85.y);
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
