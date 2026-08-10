// Generated from the exact embedded Ceylon SM3 bytecode; fixed-function alpha test is injected by the WebGPU backend.
enable f16;

struct cF_buf {
    m: array<vec4<f32>, 224>,
}

@group(0) @binding(33) 
var s0_: sampler;
@group(0) @binding(32) 
var s0_2d: texture_2d<f32>;
@group(0) @binding(35) 
var s1_: sampler;
@group(0) @binding(34) 
var s1_2d: texture_2d<f32>;
var<private> v0_color_1: vec4<f32>;
var<private> v1_specular0_1: vec4<f32>;
var<private> v2_texcoord0_1: vec4<f32>;
var<private> v3_texcoord2_1: vec4<f32>;
var<private> oC0_color: vec4<f32>;
@group(0) @binding(16) 
var<uniform> cF: cF_buf;

fn mad_legacy_f32_(a: f32, b: f32, c: f32) -> f32 {
    return fma(select(a, 0f, (b == 0f)), select(b, 0f, (a == 0f)), c);
}

fn mul_legacy_f32_(a_1: f32, b_1: f32) -> f32 {
    return (select(a_1, 0f, (b_1 == 0f)) * select(b_1, 0f, (a_1 == 0f)));
}

fn sampleTexture_1_(texCoord: vec4<f32>) -> vec4<f32> {
    let _e36: vec4<f32> = textureSample(s1_2d, s1_, vec2<f32>(texCoord.x, texCoord.y));
    return _e36;
}

fn sampleTexture_0_(texCoord_1: vec4<f32>) -> vec4<f32> {
    let _e36: vec4<f32> = textureSample(s0_2d, s0_, vec2<f32>(texCoord_1.x, texCoord_1.y));
    return _e36;
}

fn main_1() {
    var phi_385_: f32;
    var phi_386_: f32;
    var phi_387_: f32;

    let _e33: f32 = v2_texcoord0_1[0u];
    let _e35: f32 = v2_texcoord0_1[1u];
    let _e37: f32 = v2_texcoord0_1[2u];
    let _e39: f32 = v2_texcoord0_1[3u];
    let _e41: vec4<f32> = sampleTexture_0_(vec4<f32>(_e33, _e35, _e37, _e39));
    let _e47: f32 = v0_color_1[0u];
    let _e49: f32 = v0_color_1[1u];
    let _e51: f32 = v0_color_1[2u];
    let _e53: f32 = v0_color_1[3u];
    let _e54: f32 = mul_legacy_f32_(_e41.w, _e53);
    let _e56: f32 = v1_specular0_1[0u];
    let _e58: f32 = v1_specular0_1[1u];
    let _e60: f32 = v1_specular0_1[2u];
    let _e61: f32 = mad_legacy_f32_(_e41.x, _e47, _e56);
    let _e62: f32 = mad_legacy_f32_(_e41.y, _e49, _e58);
    let _e63: f32 = mad_legacy_f32_(_e41.z, _e51, _e60);
    let _e66: f32 = min(_e61, min(_e62, _e63));
    let _e67: f32 = max(_e61, max(_e62, _e63));
    let _e68: f32 = (_e67 + _e66);
    let _e69: f32 = (_e67 - _e66);
    let _e76: f16 = f16(select(0f, 1f, (_e69 == 0f)));
    let _e84: f32 = mul_legacy_f32_(_e69, (1f / (2f - _e68)));
    let _e86: f32 = mul_legacy_f32_((1f / _e68), _e69);
    let _e89: f32 = select(_e84, _e86, (f32(select(0h, 1h, ((f16(select(1f, 0f, (fma(_e68, 0.5f, -0.5f) >= 0f))) == 0h) && (_e76 == 0h)))) <= 0f));
    let _e91: bool = (f32(abs(_e76)) <= 0f);
    let _e92: f32 = select(0f, _e89, _e91);
    let _e94: f32 = v3_texcoord2_1[0u];
    let _e96: f32 = v3_texcoord2_1[1u];
    let _e98: f32 = v3_texcoord2_1[2u];
    let _e100: f32 = v3_texcoord2_1[3u];
    let _e106: vec4<f32> = sampleTexture_1_(vec4<f32>((_e94 / _e100), (_e96 / _e100), (_e98 / _e100), (_e100 / _e100)));
    let _e113: f32 = max(_e106.x, max(_e106.y, _e106.z));
    let _e114: f32 = min(_e106.x, min(_e106.y, _e106.z));
    let _e115: f32 = (_e113 + _e114);
    let _e116: f32 = (_e115 * 0.5f);
    let _e118: f32 = mad_legacy_f32_(_e116, (1f - _e92), _e92);
    let _e124: f32 = mul_legacy_f32_(_e116, (_e92 + 1f));
    let _e129: f32 = select(_e124, _e118, (f32(abs(f16(select(1f, 0f, ((_e116 - 0.5f) >= 0f))))) <= 0f));
    let _e136: f32 = (1f / (_e113 - _e114));
    let _e140: bool = (f16(select(0f, 1f, ((_e106.x - _e113) == 0f))) == 0h);
    let _e141: f16 = f16(select(0f, 1f, ((_e106.y - _e113) == 0f)));
    let _e146: f32 = mul_legacy_f32_((_e106.y - _e106.z), _e136);
    let _e147: f32 = mad_legacy_f32_(_e136, (_e106.z - _e106.x), 2f);
    let _e153: f32 = mad_legacy_f32_(_e136, (_e106.x - _e106.y), 4f);
    let _e159: f32 = (_e115 - _e129);
    let _e160: f32 = (select(_e153, select(_e147, _e146, (f32(select(0h, _e141, _e140)) <= 0f)), (f32(select(0h, 1h, (_e140 && (_e141 == 0h)))) <= 0f)) * 0.16666667f);
    phi_385_ = _e116;
    phi_386_ = _e116;
    phi_387_ = _e116;
    if !((!(_e91) || (_e91 && (_e89 == 0f)))) {
        let _e166: f32 = (_e160 * 360f);
        let _e167: f32 = fma(_e160, 360f, -120f);
        let _e170: bool = ((360f - _e167) >= 0f);
        let _e171: f32 = select((_e167 - 360f), _e167, _e170);
        let _e182: f32 = select((_e171 + 360f), _e171, (f32(select(0h, f16(select(1f, 0f, (_e171 >= 0f))), (f16(select(1f, 0f, _e170)) == 0h))) <= 0f));
        let _e188: f16 = f16(select(1f, 0f, ((_e182 - 180f) >= 0f)));
        let _e192: bool = (f16(select(1f, 0f, ((_e182 - 60f) >= 0f))) == 0h);
        let _e194: bool = (_e192 && (_e188 == 0h));
        let _e198: f16 = f16(select(1f, 0f, ((_e182 - 240f) >= 0f)));
        let _e202: f32 = (_e129 + (_e129 - _e115));
        let _e204: f32 = mul_legacy_f32_(_e182, _e202);
        let _e205: f32 = mul_legacy_f32_(_e202, (240f - _e182));
        let _e218: bool = ((360f - _e166) >= 0f);
        let _e219: f32 = select((_e166 - 360f), _e166, _e218);
        let _e230: f32 = select((_e219 + 360f), _e219, (f32(select(0h, f16(select(1f, 0f, (_e219 >= 0f))), (f16(select(1f, 0f, _e218)) == 0h))) <= 0f));
        let _e235: f16 = f16(select(1f, 0f, ((_e230 - 180f) >= 0f)));
        let _e243: bool = (f16(select(1f, 0f, ((_e230 - 60f) >= 0f))) == 0h);
        let _e245: bool = (_e243 && (_e235 == 0h));
        let _e250: f16 = f16(select(1f, 0f, ((_e230 - 240f) >= 0f)));
        let _e252: f32 = mul_legacy_f32_(_e202, _e230);
        let _e253: f32 = mul_legacy_f32_(_e202, (240f - _e230));
        let _e262: f32 = fma(_e160, 360f, 120f);
        let _e268: bool = ((360f - _e262) >= 0f);
        let _e269: f32 = select((_e262 - 360f), _e262, _e268);
        let _e280: f32 = select((_e269 + 360f), _e269, (f32(select(0h, f16(select(1f, 0f, (_e269 >= 0f))), (f16(select(1f, 0f, _e268)) == 0h))) <= 0f));
        let _e289: f32 = mul_legacy_f32_(_e202, _e280);
        let _e292: f16 = f16(select(1f, 0f, ((_e280 - 180f) >= 0f)));
        let _e296: bool = (f16(select(1f, 0f, ((_e280 - 60f) >= 0f))) == 0h);
        let _e298: bool = (_e296 && (_e292 == 0h));
        let _e299: f16 = f16(select(1f, 0f, ((_e280 - 240f) >= 0f)));
        let _e304: f32 = mul_legacy_f32_(_e202, (240f - _e280));
        phi_385_ = select(_e159, select(fma(_e205, 0.01666667f, _e159), select(_e129, fma(_e204, 0.01666667f, _e159), (f32(select(0h, _e188, _e192)) <= 0f)), (f32(select(0h, _e198, _e194)) <= 0f)), (f32(select(0h, 1h, ((_e198 == 0h) && _e194))) <= 0f));
        phi_386_ = select(_e159, select(fma(_e253, 0.01666667f, _e159), select(_e129, fma(_e252, 0.01666667f, _e159), (f32(select(0h, _e235, _e243)) <= 0f)), (f32(select(0h, _e250, _e245)) <= 0f)), (f32(select(0h, 1h, ((_e250 == 0h) && _e245))) <= 0f));
        phi_387_ = select(_e159, select(fma(_e304, 0.01666667f, _e159), select(_e129, fma(_e289, 0.01666667f, _e159), (f32(select(0h, _e292, _e296)) <= 0f)), (f32(select(0h, _e299, _e298)) <= 0f)), (f32(select(0h, 1h, ((_e299 == 0h) && _e298))) <= 0f));
    }
    let _e319: f32 = phi_385_;
    let _e321: f32 = phi_386_;
    let _e323: f32 = phi_387_;
    let _e326: vec4<f32> = cF.m[0u];
    let _e329: f32 = mad_legacy_f32_(_e323, _e326.z, -(_e106.x));
    let _e330: f32 = mad_legacy_f32_(_e321, _e326.z, -(_e106.y));
    let _e331: f32 = mad_legacy_f32_(_e319, _e326.z, -(_e106.z));
    let _e332: f32 = mad_legacy_f32_(_e54, _e326.z, -(_e106.w));
    let _e333: f32 = mad_legacy_f32_(_e54, _e329, _e106.x);
    let _e334: f32 = mad_legacy_f32_(_e54, _e330, _e106.y);
    let _e335: f32 = mad_legacy_f32_(_e54, _e331, _e106.z);
    let _e336: f32 = mad_legacy_f32_(_e54, _e332, _e106.w);
    let _e338: f32 = mul_legacy_f32_(_e336, (1f / _e326.z));
    let _e362: bool = (f32(clamp((clamp((clamp((f16(select(1f, 0f, ((999.9f - _e333) >= 0f))) + f16(select(1f, 0f, ((999.9f - _e334) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e335) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e338) >= 0f)))), 0h, 1h)) <= 0f);
    oC0_color[0u] = select(0f, _e333, _e362);
    oC0_color[1u] = select(0f, _e334, _e362);
    oC0_color[2u] = select(0f, _e335, _e362);
    oC0_color[3u] = select(0f, _e338, _e362);
    return;
}

@fragment 
fn main(@location(9) v0_color: vec4<f32>, @location(10) v1_specular0_: vec4<f32>, @location(1) v2_texcoord0_: vec4<f32>, @location(3) v3_texcoord2_: vec4<f32>) -> @location(0) vec4<f32> {
    v0_color_1 = v0_color;
    v1_specular0_1 = v1_specular0_;
    v2_texcoord0_1 = v2_texcoord0_;
    v3_texcoord2_1 = v3_texcoord2_;
    main_1();
    let _e9: vec4<f32> = oC0_color;
    return _e9;
}
