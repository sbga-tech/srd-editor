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

fn dp4_f32_legacy(a_1: vec4<f32>, b_1: vec4<f32>) -> f32 {
    return fma(select(a_1.w, 0f, (b_1.w == 0f)), select(b_1.w, 0f, (a_1.w == 0f)), fma(select(a_1.z, 0f, (b_1.z == 0f)), select(b_1.z, 0f, (a_1.z == 0f)), fma(select(a_1.y, 0f, (b_1.y == 0f)), select(b_1.y, 0f, (a_1.y == 0f)), (select(a_1.x, 0f, (b_1.x == 0f)) * select(b_1.x, 0f, (a_1.x == 0f))))));
}

fn main_1() {
    let _e27: f32 = v0_position0_1[0u];
    let _e29: f32 = v0_position0_1[1u];
    let _e31: f32 = v0_position0_1[2u];
    let _e32: vec4<f32> = vec4<f32>(_e27, _e29, _e31, 1f);
    let _e35: vec4<f32> = cF.m[0u];
    let _e36: f32 = dp4_f32_legacy(_e32, _e35);
    let _e39: vec4<f32> = cF.m[3u];
    let _e40: f32 = dp4_f32_legacy(_e32, _e39);
    let _e43: vec4<f32> = cF.m[2u];
    let _e44: f32 = dp4_f32_legacy(_e32, _e43);
    let _e47: vec4<f32> = cF.m[1u];
    let _e48: f32 = dp4_f32_legacy(_e32, _e47);
    let _e49: vec4<f32> = vec4<f32>(_e36, _e48, _e44, _e40);
    let _e52: vec4<f32> = cF.m[13u];
    let _e53: f32 = dp4_f32_legacy(_e49, _e52);
    let _e56: vec4<f32> = cF.m[12u];
    let _e57: f32 = dp4_f32_legacy(_e49, _e56);
    let _e60: vec4<f32> = cF.m[10u];
    let _e61: f32 = dp4_f32_legacy(_e49, _e60);
    let _e64: vec4<f32> = cF.m[11u];
    let _e65: f32 = dp4_f32_legacy(_e49, _e64);
    let _e66: f32 = (_e53 * 0.5f);
    let _e71: vec4<f32> = cF.m[14u];
    let _e76: f32 = mul_legacy_f32_(_e53, (1f / _e71.x));
    let _e77: f32 = mul_legacy_f32_(_e53, (1f / _e71.y));
    let _e81: f32 = v2_color_1[0u];
    let _e83: f32 = v2_color_1[1u];
    let _e85: f32 = v2_color_1[2u];
    let _e87: f32 = v2_color_1[3u];
    let _e89: f32 = v3_specular0_1[0u];
    let _e91: f32 = v3_specular0_1[1u];
    let _e93: f32 = v3_specular0_1[2u];
    let _e95: f32 = v3_specular0_1[3u];
    let _e97: f32 = v1_texcoord0_1[0u];
    let _e99: f32 = v1_texcoord0_1[1u];
    let _e102: vec4<f32> = cF.m[8u];
    o0_position0_[0u] = _e61;
    o0_position0_[1u] = _e65;
    o0_position0_[2u] = _e57;
    o0_position0_[3u] = _e53;
    o1_color[0u] = _e81;
    o1_color[1u] = _e83;
    o1_color[2u] = _e85;
    o1_color[3u] = _e87;
    o2_specular0_[0u] = _e89;
    o2_specular0_[1u] = _e91;
    o2_specular0_[2u] = _e93;
    o2_specular0_[3u] = _e95;
    o3_texcoord0_[0u] = (_e97 + _e102.z);
    o3_texcoord0_[1u] = (_e99 + _e102.w);
    o3_texcoord0_[2u] = 0f;
    o3_texcoord0_[3u] = 0f;
    o4_texcoord2_[0u] = fma(_e76, 0.25f, fma(_e61, 0.5f, _e66));
    o4_texcoord2_[1u] = fma(_e77, 0.25f, fma(_e65, -0.5f, _e66));
    o4_texcoord2_[2u] = _e57;
    o4_texcoord2_[3u] = _e53;
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
