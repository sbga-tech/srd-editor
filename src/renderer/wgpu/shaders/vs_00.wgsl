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

fn dp4_f32_legacy(a: vec4<f32>, b: vec4<f32>) -> f32 {
    return fma(select(a.w, 0f, (b.w == 0f)), select(b.w, 0f, (a.w == 0f)), fma(select(a.z, 0f, (b.z == 0f)), select(b.z, 0f, (a.z == 0f)), fma(select(a.y, 0f, (b.y == 0f)), select(b.y, 0f, (a.y == 0f)), (select(a.x, 0f, (b.x == 0f)) * select(b.x, 0f, (a.x == 0f))))));
}

fn main_1() {
    let _e17: f32 = v0_position0_1[0u];
    let _e19: f32 = v0_position0_1[1u];
    let _e21: f32 = v0_position0_1[2u];
    let _e22: vec4<f32> = vec4<f32>(_e17, _e19, _e21, 1f);
    let _e25: vec4<f32> = cF.m[3u];
    let _e26: f32 = dp4_f32_legacy(_e22, _e25);
    let _e29: vec4<f32> = cF.m[2u];
    let _e30: f32 = dp4_f32_legacy(_e22, _e29);
    let _e33: vec4<f32> = cF.m[0u];
    let _e34: f32 = dp4_f32_legacy(_e22, _e33);
    let _e37: vec4<f32> = cF.m[1u];
    let _e38: f32 = dp4_f32_legacy(_e22, _e37);
    let _e39: vec4<f32> = vec4<f32>(_e34, _e38, _e30, _e26);
    let _e42: vec4<f32> = cF.m[13u];
    let _e43: f32 = dp4_f32_legacy(_e39, _e42);
    let _e46: vec4<f32> = cF.m[12u];
    let _e47: f32 = dp4_f32_legacy(_e39, _e46);
    let _e50: vec4<f32> = cF.m[11u];
    let _e51: f32 = dp4_f32_legacy(_e39, _e50);
    let _e54: vec4<f32> = cF.m[10u];
    let _e55: f32 = dp4_f32_legacy(_e39, _e54);
    let _e57: f32 = v1_color_1[0u];
    let _e59: f32 = v1_color_1[1u];
    let _e61: f32 = v1_color_1[2u];
    let _e63: f32 = v1_color_1[3u];
    o0_position0_[0u] = _e55;
    o0_position0_[1u] = _e51;
    o0_position0_[2u] = _e47;
    o0_position0_[3u] = _e43;
    o1_color[0u] = _e57;
    o1_color[1u] = _e59;
    o1_color[2u] = _e61;
    o1_color[3u] = _e63;
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
