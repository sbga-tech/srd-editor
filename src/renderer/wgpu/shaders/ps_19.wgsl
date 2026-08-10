// Generated from the exact embedded Ceylon SM3 bytecode; fixed-function alpha test is injected by the WebGPU backend.
enable f16;

struct cF_buf {
    m: array<vec4<f32>, 224>,
}

@group(0) @binding(33) 
var s0_: sampler;
@group(0) @binding(32) 
var s0_2d: texture_2d<f32>;
var<private> v0_color_1: vec4<f32>;
var<private> v1_specular0_1: vec4<f32>;
var<private> v2_texcoord2_1: vec4<f32>;
var<private> oC0_color: vec4<f32>;
@group(0) @binding(16) 
var<uniform> cF: cF_buf;

fn mad_legacy_f32_(a: f32, b: f32, c: f32) -> f32 {
    return fma(select(a, 0f, (b == 0f)), select(b, 0f, (a == 0f)), c);
}

fn mul_legacy_f32_(a_1: f32, b_1: f32) -> f32 {
    return (select(a_1, 0f, (b_1 == 0f)) * select(b_1, 0f, (a_1 == 0f)));
}

fn sampleTexture_0_(texCoord: vec4<f32>) -> vec4<f32> {
    let _e33: vec4<f32> = textureSample(s0_2d, s0_, vec2<f32>(texCoord.x, texCoord.y));
    return _e33;
}

fn main_1() {
    var phi_351_: f32;
    var phi_352_: f32;
    var phi_353_: f32;

    let _e30: f32 = v1_specular0_1[0u];
    let _e32: f32 = v1_specular0_1[1u];
    let _e34: f32 = v1_specular0_1[2u];
    let _e36: f32 = v0_color_1[0u];
    let _e38: f32 = v0_color_1[1u];
    let _e40: f32 = v0_color_1[2u];
    let _e41: f32 = (_e36 + _e30);
    let _e42: f32 = (_e38 + _e32);
    let _e43: f32 = (_e40 + _e34);
    let _e46: f32 = max(_e41, max(_e42, _e43));
    let _e47: f32 = min(_e41, min(_e42, _e43));
    let _e48: f32 = (_e46 + _e47);
    let _e49: f32 = (_e46 - _e47);
    let _e56: f16 = f16(select(0f, 1f, (_e49 == 0f)));
    let _e65: f32 = mul_legacy_f32_(_e49, (1f / (2f - _e48)));
    let _e66: f32 = mul_legacy_f32_(_e49, (1f / _e48));
    let _e69: f32 = select(_e65, _e66, (f32(select(0h, 1h, ((f16(select(1f, 0f, (fma(_e48, 0.5f, -0.5f) >= 0f))) == 0h) && (_e56 == 0h)))) <= 0f));
    let _e71: bool = (f32(abs(_e56)) <= 0f);
    let _e72: f32 = select(0f, _e69, _e71);
    let _e74: f32 = v2_texcoord2_1[0u];
    let _e76: f32 = v2_texcoord2_1[1u];
    let _e78: f32 = v2_texcoord2_1[2u];
    let _e80: f32 = v2_texcoord2_1[3u];
    let _e86: vec4<f32> = sampleTexture_0_(vec4<f32>((_e74 / _e80), (_e76 / _e80), (_e78 / _e80), (_e80 / _e80)));
    let _e93: f32 = (1f / _e49);
    let _e96: f32 = (max(_e86.x, max(_e86.y, _e86.z)) + min(_e86.x, min(_e86.y, _e86.z)));
    let _e97: f32 = (_e96 * 0.5f);
    let _e106: f32 = mul_legacy_f32_(_e97, (_e72 + 1f));
    let _e107: f32 = mad_legacy_f32_(_e97, (1f - _e72), _e72);
    let _e110: f32 = select(_e106, _e107, (f32(abs(f16(select(1f, 0f, ((_e97 - 0.5f) >= 0f))))) <= 0f));
    let _e120: bool = (f16(select(0f, 1f, ((_e41 - _e46) == 0f))) == 0h);
    let _e121: f16 = f16(select(0f, 1f, ((_e42 - _e46) == 0f)));
    let _e126: f32 = mad_legacy_f32_(_e93, (_e43 - _e41), 2f);
    let _e127: f32 = mul_legacy_f32_((_e42 - _e43), _e93);
    let _e131: f32 = mad_legacy_f32_(_e93, (_e41 - _e42), 4f);
    let _e135: f32 = (_e96 - _e110);
    let _e136: f32 = (select(_e131, select(_e126, _e127, (f32(select(0h, _e121, _e120)) <= 0f)), (f32(select(0h, 1h, (_e120 && (_e121 == 0h)))) <= 0f)) * 0.16666667f);
    phi_351_ = _e97;
    phi_352_ = _e97;
    phi_353_ = _e97;
    if !((!(_e71) || (_e71 && (_e69 == 0f)))) {
        let _e142: f32 = (_e136 * 360f);
        let _e143: f32 = fma(_e136, 360f, -120f);
        let _e146: bool = ((360f - _e143) >= 0f);
        let _e147: f32 = select((_e143 - 360f), _e143, _e146);
        let _e158: f32 = select((_e147 + 360f), _e147, (f32(select(0h, f16(select(1f, 0f, (_e147 >= 0f))), (f16(select(1f, 0f, _e146)) == 0h))) <= 0f));
        let _e166: f16 = f16(select(1f, 0f, ((_e158 - 180f) >= 0f)));
        let _e170: bool = (f16(select(1f, 0f, ((_e158 - 60f) >= 0f))) == 0h);
        let _e172: bool = (_e170 && (_e166 == 0h));
        let _e174: f16 = f16(select(1f, 0f, ((_e158 - 240f) >= 0f)));
        let _e178: f32 = (_e110 + (_e110 - _e96));
        let _e180: f32 = mul_legacy_f32_(_e158, _e178);
        let _e181: f32 = mul_legacy_f32_(_e178, (240f - _e158));
        let _e194: bool = ((360f - _e142) >= 0f);
        let _e195: f32 = select((_e142 - 360f), _e142, _e194);
        let _e206: f32 = select((_e195 + 360f), _e195, (f32(select(0h, f16(select(1f, 0f, (_e195 >= 0f))), (f16(select(1f, 0f, _e194)) == 0h))) <= 0f));
        let _e217: f16 = f16(select(1f, 0f, ((_e206 - 180f) >= 0f)));
        let _e221: bool = (f16(select(1f, 0f, ((_e206 - 60f) >= 0f))) == 0h);
        let _e223: bool = (_e221 && (_e217 == 0h));
        let _e224: f16 = f16(select(1f, 0f, ((_e206 - 240f) >= 0f)));
        let _e228: f32 = mul_legacy_f32_(_e178, _e206);
        let _e229: f32 = mul_legacy_f32_(_e178, (240f - _e206));
        let _e235: f32 = fma(_e136, 360f, 120f);
        let _e244: bool = ((360f - _e235) >= 0f);
        let _e245: f32 = select((_e235 - 360f), _e235, _e244);
        let _e256: f32 = select((_e245 + 360f), _e245, (f32(select(0h, f16(select(1f, 0f, (_e245 >= 0f))), (f16(select(1f, 0f, _e244)) == 0h))) <= 0f));
        let _e260: f32 = mul_legacy_f32_(_e178, _e256);
        let _e261: f16 = f16(select(1f, 0f, ((_e256 - 180f) >= 0f)));
        let _e272: bool = (f16(select(1f, 0f, ((_e256 - 60f) >= 0f))) == 0h);
        let _e274: bool = (_e272 && (_e261 == 0h));
        let _e276: f32 = mul_legacy_f32_(_e178, (240f - _e256));
        let _e277: f16 = f16(select(1f, 0f, ((_e256 - 240f) >= 0f)));
        phi_351_ = select(_e135, select(fma(_e181, 0.01666667f, _e135), select(_e110, fma(_e180, 0.01666667f, _e135), (f32(select(0h, _e166, _e170)) <= 0f)), (f32(select(0h, _e174, _e172)) <= 0f)), (f32(select(0h, 1h, ((_e174 == 0h) && _e172))) <= 0f));
        phi_352_ = select(_e135, select(fma(_e229, 0.01666667f, _e135), select(_e110, fma(_e228, 0.01666667f, _e135), (f32(select(0h, _e217, _e221)) <= 0f)), (f32(select(0h, _e224, _e223)) <= 0f)), (f32(select(0h, 1h, ((_e224 == 0h) && _e223))) <= 0f));
        phi_353_ = select(_e135, select(fma(_e276, 0.01666667f, _e135), select(_e110, fma(_e260, 0.01666667f, _e135), (f32(select(0h, _e261, _e272)) <= 0f)), (f32(select(0h, _e277, _e274)) <= 0f)), (f32(select(0h, 1h, ((_e277 == 0h) && _e274))) <= 0f));
    }
    let _e295: f32 = phi_351_;
    let _e297: f32 = phi_352_;
    let _e299: f32 = phi_353_;
    let _e301: f32 = v0_color_1[3u];
    let _e304: vec4<f32> = cF.m[0u];
    let _e310: f32 = mad_legacy_f32_(_e299, _e304.z, -(_e86.x));
    let _e311: f32 = mad_legacy_f32_(_e297, _e304.z, -(_e86.y));
    let _e312: f32 = mad_legacy_f32_(_e295, _e304.z, -(_e86.z));
    let _e313: f32 = mad_legacy_f32_(_e301, _e304.z, -(_e86.w));
    let _e314: f32 = mad_legacy_f32_(_e301, _e310, _e86.x);
    let _e315: f32 = mad_legacy_f32_(_e301, _e311, _e86.y);
    let _e316: f32 = mad_legacy_f32_(_e301, _e312, _e86.z);
    let _e317: f32 = mad_legacy_f32_(_e301, _e313, _e86.w);
    let _e319: f32 = mul_legacy_f32_(_e317, (1f / _e304.z));
    let _e343: bool = (f32(clamp((clamp((clamp((f16(select(1f, 0f, ((999.9f - _e314) >= 0f))) + f16(select(1f, 0f, ((999.9f - _e315) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e316) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e319) >= 0f)))), 0h, 1h)) <= 0f);
    oC0_color[0u] = select(0f, _e314, _e343);
    oC0_color[1u] = select(0f, _e315, _e343);
    oC0_color[2u] = select(0f, _e316, _e343);
    oC0_color[3u] = select(0f, _e319, _e343);
    return;
}

@fragment 
fn main(@location(9) v0_color: vec4<f32>, @location(10) v1_specular0_: vec4<f32>, @location(3) v2_texcoord2_: vec4<f32>) -> @location(0) vec4<f32> {
    v0_color_1 = v0_color;
    v1_specular0_1 = v1_specular0_;
    v2_texcoord2_1 = v2_texcoord2_;
    main_1();
    let _e7: vec4<f32> = oC0_color;
    return _e7;
}
