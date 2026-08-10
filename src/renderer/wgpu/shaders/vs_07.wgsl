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
var<private> v1_tangent_1: vec4<f32>;
var<private> v2_texcoord0_1: vec4<f32>;
var<private> v3_color_1: vec4<f32>;
var<private> v4_specular0_1: vec4<f32>;
@group(0) @binding(0) 
var<uniform> cF: cF_buf;

fn dp3_f32_(a: vec3<f32>, b: vec3<f32>) -> f32 {
    return fma(a.z, b.z, fma(a.y, b.y, (a.x * b.x)));
}

fn dp4_f32_legacy(a_1: vec4<f32>, b_1: vec4<f32>) -> f32 {
    return fma(select(a_1.w, 0f, (b_1.w == 0f)), select(b_1.w, 0f, (a_1.w == 0f)), fma(select(a_1.z, 0f, (b_1.z == 0f)), select(b_1.z, 0f, (a_1.z == 0f)), fma(select(a_1.y, 0f, (b_1.y == 0f)), select(b_1.y, 0f, (a_1.y == 0f)), (select(a_1.x, 0f, (b_1.x == 0f)) * select(b_1.x, 0f, (a_1.x == 0f))))));
}

fn dp3_f32_legacy(a_2: vec3<f32>, b_2: vec3<f32>) -> f32 {
    return fma(select(a_2.z, 0f, (b_2.z == 0f)), select(b_2.z, 0f, (a_2.z == 0f)), fma(select(a_2.y, 0f, (b_2.y == 0f)), select(b_2.y, 0f, (a_2.y == 0f)), (select(a_2.x, 0f, (b_2.x == 0f)) * select(b_2.x, 0f, (a_2.x == 0f)))));
}

fn mad_legacy_f32_(a_3: f32, b_3: f32, c: f32) -> f32 {
    return fma(select(a_3, 0f, (b_3 == 0f)), select(b_3, 0f, (a_3 == 0f)), c);
}

fn mul_legacy_f32_(a_4: f32, b_4: f32) -> f32 {
    return (select(a_4, 0f, (b_4 == 0f)) * select(b_4, 0f, (a_4 == 0f)));
}

fn main_1() {
    var phi_497_: f32;
    var phi_498_: f32;
    var phi_499_: f32;
    var phi_500_: f32;
    var phi_501_: f32;
    var phi_502_: f32;
    var phi_528_: f32;
    var phi_529_: f32;
    var phi_530_: f32;
    var phi_531_: f32;
    var phi_532_: f32;
    var phi_557_: f32;
    var phi_558_: f32;
    var phi_559_: f32;
    var phi_560_: f32;
    var phi_561_: f32;
    var phi_590_: f32;
    var phi_591_: f32;
    var phi_630_: f32;
    var phi_631_: f32;
    var phi_632_: f32;
    var phi_633_: f32;
    var phi_634_: f32;
    var phi_635_: f32;
    var phi_657_: f32;
    var phi_658_: f32;
    var phi_659_: f32;
    var phi_660_: f32;
    var phi_661_: f32;
    var phi_662_: f32;
    var phi_695_: f32;
    var phi_696_: f32;
    var phi_697_: f32;
    var phi_698_: f32;
    var phi_699_: f32;
    var phi_700_: f32;
    var phi_747_: f32;
    var phi_748_: f32;
    var phi_749_: f32;
    var phi_750_: f32;
    var phi_751_: f32;
    var phi_752_: f32;
    var phi_793_: f32;
    var phi_794_: f32;
    var phi_795_: f32;
    var phi_796_: f32;
    var phi_797_: f32;
    var phi_798_: f32;
    var phi_839_: f32;
    var phi_840_: f32;
    var phi_841_: f32;
    var phi_842_: f32;
    var phi_843_: f32;
    var phi_844_: f32;
    var phi_891_: f32;
    var phi_892_: f32;
    var phi_893_: f32;
    var phi_894_: f32;
    var phi_895_: f32;
    var phi_896_: f32;
    var phi_926_: f32;
    var phi_927_: f32;
    var phi_928_: f32;
    var phi_929_: f32;
    var phi_930_: f32;
    var phi_931_: f32;
    var phi_961_: f32;
    var phi_962_: f32;
    var phi_963_: f32;
    var phi_964_: f32;
    var phi_965_: f32;
    var phi_966_: f32;
    var phi_1012_: f32;
    var phi_1013_: f32;
    var phi_1014_: f32;
    var phi_1015_: f32;
    var phi_1016_: f32;
    var phi_1017_: f32;
    var phi_1018_: f32;
    var phi_1019_: f32;
    var phi_1020_: f32;
    var phi_1021_: f32;
    var phi_1022_: f32;
    var phi_1023_: f32;
    var phi_1024_: f32;
    var phi_1073_: f32;
    var phi_1074_: f32;
    var phi_1075_: f32;
    var phi_1076_: f32;
    var phi_1077_: f32;
    var phi_1078_: f32;
    var phi_1079_: f32;
    var phi_1080_: f32;
    var phi_1081_: f32;
    var phi_1082_: f32;
    var phi_1083_: f32;
    var phi_1084_: f32;
    var phi_1085_: f32;
    var phi_1123_: f32;
    var phi_1124_: f32;
    var phi_1125_: f32;
    var phi_1138_: bool;
    var phi_1139_: bool;
    var phi_1141_: bool;
    var phi_1142_: bool;
    var phi_1171_: f32;
    var phi_1191_: f32;
    var phi_1192_: f32;
    var phi_1193_: f32;
    var phi_1194_: f32;
    var phi_1195_: f32;
    var phi_1215_: f32;
    var phi_1216_: f32;
    var phi_1217_: f32;
    var phi_1218_: f32;
    var phi_1219_: f32;
    var phi_1239_: f32;
    var phi_1240_: f32;
    var phi_1241_: f32;
    var phi_1242_: f32;
    var phi_1243_: f32;
    var phi_1262_: f32;
    var phi_1263_: f32;
    var phi_1264_: f32;
    var phi_1265_: f32;
    var phi_1349_: f32;
    var phi_1350_: f32;
    var phi_1351_: f32;
    var phi_1352_: f32;
    var phi_1353_: f32;
    var phi_1354_: f32;
    var phi_1355_: f32;
    var phi_1356_: f32;
    var phi_1357_: f32;
    var phi_1402_: f32;
    var phi_1403_: f32;
    var phi_1404_: f32;
    var phi_1405_: f32;
    var phi_1406_: f32;
    var phi_1407_: f32;
    var phi_1408_: f32;
    var phi_1409_: f32;
    var phi_1410_: f32;
    var phi_1455_: f32;
    var phi_1456_: f32;
    var phi_1457_: f32;
    var phi_1458_: f32;
    var phi_1459_: f32;
    var phi_1460_: f32;
    var phi_1461_: f32;
    var phi_1462_: f32;
    var phi_1463_: f32;
    var phi_1508_: f32;
    var phi_1509_: f32;
    var phi_1510_: f32;
    var phi_1511_: f32;
    var phi_1512_: f32;
    var phi_1513_: f32;
    var phi_1514_: f32;
    var phi_1515_: f32;
    var phi_1516_: f32;
    var phi_1563_: f32;
    var phi_1564_: f32;
    var phi_1565_: f32;
    var phi_1566_: f32;
    var phi_1567_: f32;
    var phi_1568_: f32;
    var phi_1569_: f32;
    var phi_1570_: f32;
    var phi_1602_: f32;
    var phi_1604_: f32;
    var phi_1668_: f32;
    var phi_1669_: f32;
    var phi_1670_: f32;
    var phi_1671_: f32;
    var phi_1672_: f32;
    var phi_1673_: f32;
    var phi_1894_: f32;
    var phi_1895_: f32;
    var phi_1896_: f32;
    var phi_1908_: f32;
    var phi_1909_: f32;
    var phi_1910_: f32;
    var phi_1911_: f32;
    var phi_1912_: f32;
    var phi_1913_: f32;
    var phi_1914_: f32;
    var phi_1915_: f32;
    var phi_1916_: f32;
    var phi_1917_: f32;
    var phi_1918_: f32;
    var phi_1919_: f32;
    var phi_1920_: f32;

    let _e119: vec4<f32> = cF.m[19u];
    let _e121: f32 = (_e119.y * _e119.y);
    let _e124: f32 = mul_legacy_f32_(_e119.w, _e119.x);
    let _e126: f32 = mad_legacy_f32_(_e119.y, _e119.z, _e124);
    let _e129: vec4<f32> = cF.m[62u];
    let _e134: f32 = -((_e119.x * _e119.x));
    let _e138: f32 = mul_legacy_f32_(_e119.w, _e119.y);
    let _e140: f32 = mad_legacy_f32_(_e119.x, _e119.z, -(_e138));
    let _e143: vec4<f32> = cF.m[14u];
    let _e152: f32 = v1_tangent_1[0u];
    let _e154: f32 = v1_tangent_1[1u];
    let _e156: f32 = v1_tangent_1[2u];
    let _e157: vec3<f32> = vec3<f32>(_e152, _e154, _e156);
    let _e158: f32 = dp3_f32_legacy(vec3<f32>((_e140 * 2f), (_e126 * 2f), (fma(_e134, 2f, -((_e121 * 2f))) + 1f)), _e157);
    let _e161: f32 = -(((_e119.z * _e119.z) * 2f));
    let _e164: f32 = mad_legacy_f32_(_e119.x, _e119.z, _e138);
    let _e166: f32 = mul_legacy_f32_(_e119.w, _e119.z);
    let _e168: f32 = mad_legacy_f32_(_e119.x, _e119.y, -(_e166));
    let _e172: f32 = dp3_f32_legacy(_e157, vec3<f32>((fma(-(_e121), 2f, _e161) + 1f), (_e168 * 2f), (_e164 * 2f)));
    let _e174: f32 = mad_legacy_f32_(_e119.y, _e119.z, -(_e124));
    let _e175: f32 = mad_legacy_f32_(_e119.x, _e119.y, _e166);
    let _e180: f32 = dp3_f32_legacy(_e157, vec3<f32>((_e175 * 2f), (fma(_e134, 2f, _e161) + 1f), (_e174 * 2f)));
    let _e189: vec4<f32> = cF.m[20u];
    let _e192: f32 = v1_tangent_1[3u];
    let _e194: f32 = mul_legacy_f32_(_e192, _e143.y);
    let _e195: f32 = mad_legacy_f32_(fract((_e172 * 700.634f)), _e189.z, 1f);
    let _e196: f32 = (_e143.x - _e194);
    let _e197: f32 = mul_legacy_f32_(_e196, _e195);
    let _e202: f32 = v3_color_1[0u];
    let _e204: f32 = v3_color_1[1u];
    let _e206: f32 = v3_color_1[2u];
    let _e208: f32 = v3_color_1[3u];
    let _e210: f32 = v4_specular0_1[0u];
    let _e212: f32 = v4_specular0_1[1u];
    let _e214: f32 = v4_specular0_1[2u];
    let _e216: f32 = v4_specular0_1[3u];
    let _e219: vec4<f32> = cF.m[11u];
    let _e224: f32 = v2_texcoord0_1[0u];
    let _e226: f32 = v2_texcoord0_1[1u];
    let _e234: f32 = select(0f, 1f, ((_e194 < (_e143.x + _e129.w)) && (_e194 >= (_e143.x + _e129.z))));
    let _e237: vec4<f32> = cF.m[63u];
    let _e255: f32 = (select(_e234, (_e234 + 1f), ((_e194 >= (_e143.x + _e129.x)) && (_e194 < (_e143.x + _e129.y)))) + select(0f, 1f, ((_e194 < (_e143.x + _e237.y)) && (_e194 >= (_e143.x + _e237.x)))));
    let _e256: f32 = mad_legacy_f32_(_e172, fract((_e180 * 800.634f)), fract((_e158 * 900.634f)));
    let _e260: f32 = mad_legacy_f32_((fract(_e256) - 1f), _e189.w, 1f);
    let _e263: bool = (_e260 < 0.5f);
    let _e267: bool = ((!(_e263) && (select(_e255, (_e255 + 1f), ((_e194 < (_e143.x + _e237.w)) && (_e194 >= (_e143.x + _e237.z)))) <= 0f)) || _e263);
    let _e268: f32 = select(f32(), -100000000f, _e267);
    let _e272: vec4<f32> = cF.m[12u];
    phi_1908_ = _e226;
    phi_1909_ = _e224;
    phi_1910_ = _e216;
    phi_1911_ = _e214;
    phi_1912_ = _e212;
    phi_1913_ = _e210;
    phi_1914_ = _e208;
    phi_1915_ = _e206;
    phi_1916_ = _e204;
    phi_1917_ = _e202;
    phi_1918_ = _e268;
    phi_1919_ = _e268;
    phi_1920_ = _e268;
    if !(_e267) {
        let _e275: vec4<f32> = cF.m[17u];
        let _e277: f32 = mul_legacy_f32_(_e172, _e275.w);
        let _e278: f32 = mul_legacy_f32_(_e180, _e275.w);
        let _e279: f32 = mul_legacy_f32_(_e158, _e275.w);
        let _e289: f32 = mul_legacy_f32_(_e277, (1f / _e275.x));
        let _e290: f32 = mul_legacy_f32_(_e278, (1f / _e275.y));
        let _e291: f32 = mul_legacy_f32_(_e279, (1f / _e275.z));
        let _e304: bool = (abs(select(0f, 1f, (_e277 < 0f))) > 0f);
        let _e306: bool = (abs(select(0f, 1f, (_e278 < 0f))) > 0f);
        let _e308: bool = (abs(select(0f, 1f, (_e279 < 0f))) > 0f);
        let _e313: f32 = mul_legacy_f32_(fract(abs(_e289)), abs(_e275.x));
        let _e314: f32 = mul_legacy_f32_(fract(abs(_e290)), abs(_e275.y));
        let _e315: f32 = mul_legacy_f32_(fract(abs(_e291)), abs(_e275.z));
        let _e319: f32 = mul_legacy_f32_(_e313, (1f - select(0f, 1f, _e304)));
        let _e320: f32 = mul_legacy_f32_(_e314, (1f - select(0f, 1f, _e306)));
        let _e321: f32 = mul_legacy_f32_(_e315, (1f - select(0f, 1f, _e308)));
        let _e323: f32 = select(_e319, (_e319 - _e313), _e304);
        let _e325: f32 = select(_e320, (_e320 - _e314), _e306);
        let _e327: f32 = select(_e321, (_e321 - _e315), _e308);
        let _e331: f32 = v4_specular0_1[3u];
        let _e333: f32 = v4_specular0_1[1u];
        let _e335: f32 = v4_specular0_1[0u];
        let _e337: f32 = v4_specular0_1[2u];
        let _e339: f32 = v0_position0_1[2u];
        let _e341: f32 = v0_position0_1[1u];
        let _e343: f32 = v0_position0_1[0u];
        phi_500_ = _e323;
        phi_501_ = _e325;
        phi_502_ = _e327;
        if (_e143.z >= 0.5f) {
            phi_497_ = _e323;
            phi_498_ = _e325;
            phi_499_ = _e327;
            if (_e143.z >= 1.5f) {
                let _e345: f32 = (_e331 * 2f);
                let _e346: f32 = (_e333 * 2f);
                let _e347: f32 = mul_legacy_f32_(_e335, _e345);
                let _e348: f32 = mul_legacy_f32_(_e333, _e345);
                let _e349: f32 = (_e335 * 2f);
                let _e350: f32 = mul_legacy_f32_(_e333, _e346);
                let _e351: f32 = -(_e349);
                let _e353: f32 = mad_legacy_f32_(_e335, _e351, -(_e350));
                let _e355: f32 = mul_legacy_f32_(_e337, (_e337 * 2f));
                let _e357: f32 = mad_legacy_f32_(_e337, _e349, -(_e348));
                let _e358: f32 = mad_legacy_f32_(_e337, _e346, _e347);
                let _e361: vec4<f32> = vec4<f32>(_e323, _e325, _e327, 1f);
                let _e362: f32 = dp4_f32_legacy(vec4<f32>(_e357, _e358, (_e353 + 1f), _e339), _e361);
                let _e363: f32 = mul_legacy_f32_(_e337, _e345);
                let _e364: f32 = -(_e355);
                let _e365: f32 = mad_legacy_f32_(_e335, _e351, _e364);
                let _e367: f32 = mad_legacy_f32_(_e337, _e346, -(_e347));
                let _e369: f32 = mad_legacy_f32_(_e333, -(_e346), _e364);
                let _e370: f32 = mad_legacy_f32_(_e333, _e349, _e363);
                let _e373: f32 = dp4_f32_legacy(_e361, vec4<f32>(_e370, (_e365 + 1f), _e367, _e341));
                let _e376: f32 = mad_legacy_f32_(_e333, _e349, -(_e363));
                let _e377: f32 = mad_legacy_f32_(_e337, _e349, _e348);
                let _e379: f32 = dp4_f32_legacy(_e361, vec4<f32>((_e369 + 1f), _e376, _e377, _e343));
                phi_497_ = _e379;
                phi_498_ = _e373;
                phi_499_ = _e362;
            }
            let _e381: f32 = phi_497_;
            let _e383: f32 = phi_498_;
            let _e385: f32 = phi_499_;
            phi_500_ = _e381;
            phi_501_ = _e383;
            phi_502_ = _e385;
        }
        let _e387: f32 = phi_500_;
        let _e389: f32 = phi_501_;
        let _e391: f32 = phi_502_;
        let _e392: bool = (_e197 > 0f);
        phi_530_ = _e197;
        phi_531_ = 1f;
        phi_532_ = 0f;
        if _e392 {
            let _e395: vec4<f32> = cF.m[59u];
            let _e398: f32 = mad_legacy_f32_(_e196, _e195, -(_e395.w));
            if (_e197 < _e395.w) {
                let _e401: f32 = (_e395.x - 1f);
                let _e403: f32 = mul_legacy_f32_(_e197, (1f / _e395.w));
                let _e404: f32 = clamp(_e403, 0f, 1f);
                let _e405: f32 = mul_legacy_f32_(_e404, _e401);
                let _e407: f32 = mad_legacy_f32_(_e404, _e401, 1f);
                let _e408: f32 = mul_legacy_f32_(_e197, fma(_e405, 0.5f, 1f));
                phi_528_ = _e408;
                phi_529_ = _e407;
            } else {
                let _e411: f32 = mul_legacy_f32_(fma((_e395.x - 1f), 0.5f, 1f), _e395.w);
                phi_528_ = _e411;
                phi_529_ = _e395.x;
            }
            let _e413: f32 = phi_528_;
            let _e415: f32 = phi_529_;
            phi_530_ = _e398;
            phi_531_ = _e415;
            phi_532_ = _e413;
        }
        let _e417: f32 = phi_530_;
        let _e419: f32 = phi_531_;
        let _e421: f32 = phi_532_;
        phi_559_ = _e419;
        phi_560_ = _e417;
        phi_561_ = _e421;
        if (_e417 > 0f) {
            let _e425: vec4<f32> = cF.m[60u];
            if (_e417 < _e425.w) {
                let _e431: f32 = (_e425.x - _e419);
                let _e432: f32 = mul_legacy_f32_(_e417, (1f / _e425.w));
                let _e433: f32 = clamp(_e432, 0f, 1f);
                let _e434: f32 = mul_legacy_f32_(_e433, _e431);
                let _e436: f32 = mad_legacy_f32_(_e433, _e431, _e419);
                let _e437: f32 = mad_legacy_f32_(_e417, fma(_e434, 0.5f, _e419), _e421);
                phi_557_ = _e437;
                phi_558_ = _e436;
            } else {
                let _e440: f32 = mad_legacy_f32_(fma((_e425.x - _e419), 0.5f, _e419), _e425.w, _e421);
                phi_557_ = _e440;
                phi_558_ = _e425.x;
            }
            let _e442: f32 = phi_557_;
            let _e444: f32 = phi_558_;
            phi_559_ = _e444;
            phi_560_ = (_e417 - _e425.w);
            phi_561_ = _e442;
        }
        let _e446: f32 = phi_559_;
        let _e448: f32 = phi_560_;
        let _e450: f32 = phi_561_;
        phi_591_ = _e450;
        if (_e448 > 0f) {
            let _e454: vec4<f32> = cF.m[61u];
            if (_e448 < _e454.w) {
                let _e461: f32 = mul_legacy_f32_(_e448, (1f / _e454.w));
                let _e463: f32 = mul_legacy_f32_(clamp(_e461, 0f, 1f), (_e454.x - _e446));
                let _e465: f32 = mad_legacy_f32_(_e448, fma(_e463, 0.5f, _e446), _e450);
                phi_590_ = _e465;
            } else {
                let _e468: f32 = mul_legacy_f32_(_e448, (1f / _e454.w));
                let _e470: f32 = mul_legacy_f32_(clamp(_e468, 0f, 1f), (_e454.x - _e446));
                let _e472: f32 = mad_legacy_f32_(fma(_e470, 0.5f, _e446), _e454.w, _e450);
                let _e473: f32 = mad_legacy_f32_((_e448 - _e454.w), _e454.x, _e472);
                phi_590_ = _e473;
            }
            let _e475: f32 = phi_590_;
            phi_591_ = _e475;
        }
        let _e477: f32 = phi_591_;
        let _e481: f32 = fract((_e172 * 700.234f));
        let _e482: f32 = fract((_e180 * 800.234f));
        let _e483: f32 = fract((_e158 * 900.234f));
        let _e486: vec4<f32> = cF.m[36u];
        let _e492: vec4<f32> = cF.m[37u];
        let _e496: f32 = mad_legacy_f32_(_e481, _e486.x, _e492.x);
        let _e497: f32 = mad_legacy_f32_(_e482, _e486.y, _e492.y);
        let _e498: f32 = mad_legacy_f32_(_e483, _e486.z, _e492.z);
        phi_633_ = _e391;
        phi_634_ = _e389;
        phi_635_ = _e387;
        if (_e486.w < _e477) {
            if (_e477 >= _e492.w) {
                let _e503: f32 = (_e492.w - _e486.w);
                let _e504: f32 = mad_legacy_f32_(_e496, _e503, _e387);
                let _e505: f32 = mad_legacy_f32_(_e497, _e503, _e389);
                let _e506: f32 = mad_legacy_f32_(_e498, _e503, _e391);
                phi_630_ = _e504;
                phi_631_ = _e505;
                phi_632_ = _e506;
            } else {
                let _e507: f32 = (_e477 - _e486.w);
                let _e508: f32 = mad_legacy_f32_(_e496, _e507, _e387);
                let _e509: f32 = mad_legacy_f32_(_e497, _e507, _e389);
                let _e510: f32 = mad_legacy_f32_(_e498, _e507, _e391);
                phi_630_ = _e508;
                phi_631_ = _e509;
                phi_632_ = _e510;
            }
            let _e512: f32 = phi_630_;
            let _e514: f32 = phi_631_;
            let _e516: f32 = phi_632_;
            phi_633_ = _e516;
            phi_634_ = _e514;
            phi_635_ = _e512;
        }
        let _e518: f32 = phi_633_;
        let _e520: f32 = phi_634_;
        let _e522: f32 = phi_635_;
        let _e525: vec4<f32> = cF.m[38u];
        let _e529: f32 = mad_legacy_f32_(_e481, _e525.x, _e525.x);
        let _e530: f32 = mad_legacy_f32_(_e482, _e525.y, _e525.y);
        let _e531: f32 = mad_legacy_f32_(_e483, _e525.z, _e525.z);
        phi_660_ = _e518;
        phi_661_ = _e520;
        phi_662_ = _e522;
        if (_e525.w < _e477) {
            phi_657_ = _e522;
            phi_658_ = _e520;
            phi_659_ = _e518;
            if !((_e477 >= _e525.w)) {
                let _e536: f32 = (_e477 - _e525.w);
                let _e537: f32 = mad_legacy_f32_(_e529, _e536, _e522);
                let _e538: f32 = mad_legacy_f32_(_e530, _e536, _e520);
                let _e539: f32 = mad_legacy_f32_(_e531, _e536, _e518);
                phi_657_ = _e537;
                phi_658_ = _e538;
                phi_659_ = _e539;
            }
            let _e541: f32 = phi_657_;
            let _e543: f32 = phi_658_;
            let _e545: f32 = phi_659_;
            phi_660_ = _e545;
            phi_661_ = _e543;
            phi_662_ = _e541;
        }
        let _e547: f32 = phi_660_;
        let _e549: f32 = phi_661_;
        let _e551: f32 = phi_662_;
        let _e554: vec4<f32> = cF.m[40u];
        let _e560: vec4<f32> = cF.m[39u];
        let _e564: f32 = mad_legacy_f32_(_e481, _e554.x, _e560.x);
        let _e565: f32 = mad_legacy_f32_(_e482, _e554.y, _e560.y);
        let _e566: f32 = mad_legacy_f32_(_e483, _e554.z, _e560.z);
        phi_698_ = _e547;
        phi_699_ = _e549;
        phi_700_ = _e551;
        if (_e554.w < _e477) {
            if (_e477 >= _e560.w) {
                let _e571: f32 = (_e560.w - _e554.w);
                let _e572: f32 = mad_legacy_f32_(_e564, _e571, _e551);
                let _e573: f32 = mad_legacy_f32_(_e565, _e571, _e549);
                let _e574: f32 = mad_legacy_f32_(_e566, _e571, _e547);
                phi_695_ = _e572;
                phi_696_ = _e573;
                phi_697_ = _e574;
            } else {
                let _e575: f32 = (_e477 - _e554.w);
                let _e576: f32 = mad_legacy_f32_(_e564, _e575, _e551);
                let _e577: f32 = mad_legacy_f32_(_e565, _e575, _e549);
                let _e578: f32 = mad_legacy_f32_(_e566, _e575, _e547);
                phi_695_ = _e576;
                phi_696_ = _e577;
                phi_697_ = _e578;
            }
            let _e580: f32 = phi_695_;
            let _e582: f32 = phi_696_;
            let _e584: f32 = phi_697_;
            phi_698_ = _e584;
            phi_699_ = _e582;
            phi_700_ = _e580;
        }
        let _e586: f32 = phi_698_;
        let _e588: f32 = phi_699_;
        let _e590: f32 = phi_700_;
        let _e594: f32 = fract((_e172 * 700.345f));
        let _e595: f32 = fract((_e180 * 800.345f));
        let _e596: f32 = fract((_e158 * 900.345f));
        let _e599: vec4<f32> = cF.m[41u];
        let _e605: vec4<f32> = cF.m[42u];
        let _e609: f32 = mad_legacy_f32_(_e594, _e599.x, _e605.x);
        let _e610: f32 = mad_legacy_f32_(_e595, _e599.y, _e605.y);
        let _e611: f32 = mad_legacy_f32_(_e596, _e599.z, _e605.z);
        phi_750_ = _e586;
        phi_751_ = _e588;
        phi_752_ = _e590;
        if (_e599.w < _e477) {
            if (_e477 >= _e605.w) {
                let _e616: f32 = (_e605.w - _e599.w);
                let _e617: f32 = (_e616 * _e616);
                let _e618: f32 = mul_legacy_f32_(_e609, _e617);
                let _e619: f32 = mul_legacy_f32_(_e610, _e617);
                let _e620: f32 = mul_legacy_f32_(_e611, _e617);
                phi_747_ = fma(_e618, 0.5f, _e590);
                phi_748_ = fma(_e619, 0.5f, _e588);
                phi_749_ = fma(_e620, 0.5f, _e586);
            } else {
                let _e624: f32 = (_e477 - _e599.w);
                let _e625: f32 = (_e624 * _e624);
                let _e626: f32 = mul_legacy_f32_(_e609, _e625);
                let _e627: f32 = mul_legacy_f32_(_e610, _e625);
                let _e628: f32 = mul_legacy_f32_(_e611, _e625);
                phi_747_ = fma(_e626, 0.5f, _e590);
                phi_748_ = fma(_e627, 0.5f, _e588);
                phi_749_ = fma(_e628, 0.5f, _e586);
            }
            let _e633: f32 = phi_747_;
            let _e635: f32 = phi_748_;
            let _e637: f32 = phi_749_;
            phi_750_ = _e637;
            phi_751_ = _e635;
            phi_752_ = _e633;
        }
        let _e639: f32 = phi_750_;
        let _e641: f32 = phi_751_;
        let _e643: f32 = phi_752_;
        let _e646: vec4<f32> = cF.m[43u];
        let _e652: vec4<f32> = cF.m[44u];
        let _e656: f32 = mad_legacy_f32_(_e594, _e646.x, _e652.x);
        let _e657: f32 = mad_legacy_f32_(_e595, _e646.y, _e652.y);
        let _e658: f32 = mad_legacy_f32_(_e596, _e646.z, _e652.z);
        phi_796_ = _e639;
        phi_797_ = _e641;
        phi_798_ = _e643;
        if (_e646.w < _e477) {
            if (_e477 >= _e652.w) {
                let _e663: f32 = (_e652.w - _e646.w);
                let _e664: f32 = (_e663 * _e663);
                let _e665: f32 = mul_legacy_f32_(_e656, _e664);
                let _e666: f32 = mul_legacy_f32_(_e657, _e664);
                let _e667: f32 = mul_legacy_f32_(_e658, _e664);
                phi_793_ = fma(_e665, 0.5f, _e643);
                phi_794_ = fma(_e666, 0.5f, _e641);
                phi_795_ = fma(_e667, 0.5f, _e639);
            } else {
                let _e671: f32 = (_e477 - _e646.w);
                let _e672: f32 = (_e671 * _e671);
                let _e673: f32 = mul_legacy_f32_(_e656, _e672);
                let _e674: f32 = mul_legacy_f32_(_e657, _e672);
                let _e675: f32 = mul_legacy_f32_(_e658, _e672);
                phi_793_ = fma(_e673, 0.5f, _e643);
                phi_794_ = fma(_e674, 0.5f, _e641);
                phi_795_ = fma(_e675, 0.5f, _e639);
            }
            let _e680: f32 = phi_793_;
            let _e682: f32 = phi_794_;
            let _e684: f32 = phi_795_;
            phi_796_ = _e684;
            phi_797_ = _e682;
            phi_798_ = _e680;
        }
        let _e686: f32 = phi_796_;
        let _e688: f32 = phi_797_;
        let _e690: f32 = phi_798_;
        let _e693: vec4<f32> = cF.m[45u];
        let _e699: vec4<f32> = cF.m[46u];
        let _e703: f32 = mad_legacy_f32_(_e594, _e693.x, _e699.x);
        let _e704: f32 = mad_legacy_f32_(_e595, _e693.y, _e699.y);
        let _e705: f32 = mad_legacy_f32_(_e596, _e693.z, _e699.z);
        phi_842_ = _e686;
        phi_843_ = _e688;
        phi_844_ = _e690;
        if (_e693.w < _e477) {
            if (_e477 >= _e699.w) {
                let _e710: f32 = (_e699.w - _e693.w);
                let _e711: f32 = (_e710 * _e710);
                let _e712: f32 = mul_legacy_f32_(_e703, _e711);
                let _e713: f32 = mul_legacy_f32_(_e704, _e711);
                let _e714: f32 = mul_legacy_f32_(_e705, _e711);
                phi_839_ = fma(_e712, 0.5f, _e690);
                phi_840_ = fma(_e713, 0.5f, _e688);
                phi_841_ = fma(_e714, 0.5f, _e686);
            } else {
                let _e718: f32 = (_e477 - _e693.w);
                let _e719: f32 = (_e718 * _e718);
                let _e720: f32 = mul_legacy_f32_(_e703, _e719);
                let _e721: f32 = mul_legacy_f32_(_e704, _e719);
                let _e722: f32 = mul_legacy_f32_(_e705, _e719);
                phi_839_ = fma(_e720, 0.5f, _e690);
                phi_840_ = fma(_e721, 0.5f, _e688);
                phi_841_ = fma(_e722, 0.5f, _e686);
            }
            let _e727: f32 = phi_839_;
            let _e729: f32 = phi_840_;
            let _e731: f32 = phi_841_;
            phi_842_ = _e731;
            phi_843_ = _e729;
            phi_844_ = _e727;
        }
        let _e733: f32 = phi_842_;
        let _e735: f32 = phi_843_;
        let _e737: f32 = phi_844_;
        let _e745: f32 = fract((_e172 * 1000.456f));
        let _e746: f32 = (fract((_e172 * 700.456f)) - 0.5f);
        let _e747: f32 = (fract((_e180 * 800.456f)) - 0.5f);
        let _e748: f32 = (fract((_e158 * 900.456f)) - 0.5f);
        let _e749: vec3<f32> = vec3<f32>(_e746, _e747, _e748);
        let _e750: f32 = dp3_f32_(_e749, _e749);
        let _e753: vec4<f32> = cF.m[47u];
        let _e755: f32 = inverseSqrt(_e750);
        let _e756: f32 = mul_legacy_f32_(_e755, _e746);
        let _e757: f32 = mul_legacy_f32_(_e755, _e747);
        let _e758: f32 = mul_legacy_f32_(_e755, _e748);
        let _e761: vec4<f32> = cF.m[48u];
        let _e763: f32 = mad_legacy_f32_(_e745, _e753.x, _e761.x);
        let _e764: f32 = mul_legacy_f32_(_e756, _e763);
        let _e765: f32 = mul_legacy_f32_(_e757, _e763);
        let _e766: f32 = mul_legacy_f32_(_e758, _e763);
        phi_894_ = _e733;
        phi_895_ = _e735;
        phi_896_ = _e737;
        if (_e753.w < _e477) {
            if (_e477 >= _e761.w) {
                let _e771: f32 = (_e761.w - _e753.w);
                let _e772: f32 = mad_legacy_f32_(_e764, _e771, _e737);
                let _e773: f32 = mad_legacy_f32_(_e765, _e771, _e735);
                let _e774: f32 = mad_legacy_f32_(_e766, _e771, _e733);
                phi_891_ = _e772;
                phi_892_ = _e773;
                phi_893_ = _e774;
            } else {
                let _e775: f32 = (_e477 - _e753.w);
                let _e776: f32 = mad_legacy_f32_(_e764, _e775, _e737);
                let _e777: f32 = mad_legacy_f32_(_e765, _e775, _e735);
                let _e778: f32 = mad_legacy_f32_(_e766, _e775, _e733);
                phi_891_ = _e776;
                phi_892_ = _e777;
                phi_893_ = _e778;
            }
            let _e780: f32 = phi_891_;
            let _e782: f32 = phi_892_;
            let _e784: f32 = phi_893_;
            phi_894_ = _e784;
            phi_895_ = _e782;
            phi_896_ = _e780;
        }
        let _e786: f32 = phi_894_;
        let _e788: f32 = phi_895_;
        let _e790: f32 = phi_896_;
        let _e793: vec4<f32> = cF.m[49u];
        let _e797: vec4<f32> = cF.m[50u];
        let _e799: f32 = mad_legacy_f32_(_e745, _e793.x, _e797.x);
        let _e800: f32 = mul_legacy_f32_(_e756, _e799);
        let _e801: f32 = mul_legacy_f32_(_e757, _e799);
        let _e802: f32 = mul_legacy_f32_(_e758, _e799);
        phi_929_ = _e786;
        phi_930_ = _e788;
        phi_931_ = _e790;
        if (_e793.w < _e477) {
            if (_e477 >= _e797.w) {
                let _e807: f32 = (_e797.w - _e793.w);
                let _e808: f32 = mad_legacy_f32_(_e800, _e807, _e790);
                let _e809: f32 = mad_legacy_f32_(_e801, _e807, _e788);
                let _e810: f32 = mad_legacy_f32_(_e802, _e807, _e786);
                phi_926_ = _e808;
                phi_927_ = _e809;
                phi_928_ = _e810;
            } else {
                let _e811: f32 = (_e477 - _e793.w);
                let _e812: f32 = mad_legacy_f32_(_e800, _e811, _e790);
                let _e813: f32 = mad_legacy_f32_(_e801, _e811, _e788);
                let _e814: f32 = mad_legacy_f32_(_e802, _e811, _e786);
                phi_926_ = _e812;
                phi_927_ = _e813;
                phi_928_ = _e814;
            }
            let _e816: f32 = phi_926_;
            let _e818: f32 = phi_927_;
            let _e820: f32 = phi_928_;
            phi_929_ = _e820;
            phi_930_ = _e818;
            phi_931_ = _e816;
        }
        let _e822: f32 = phi_929_;
        let _e824: f32 = phi_930_;
        let _e826: f32 = phi_931_;
        let _e829: vec4<f32> = cF.m[51u];
        let _e833: vec4<f32> = cF.m[52u];
        let _e835: f32 = mad_legacy_f32_(_e745, _e829.x, _e833.x);
        let _e836: f32 = mul_legacy_f32_(_e756, _e835);
        let _e837: f32 = mul_legacy_f32_(_e757, _e835);
        let _e838: f32 = mul_legacy_f32_(_e758, _e835);
        phi_964_ = _e822;
        phi_965_ = _e824;
        phi_966_ = _e826;
        if (_e829.w < _e477) {
            if (_e477 >= _e833.w) {
                let _e843: f32 = (_e833.w - _e829.w);
                let _e844: f32 = mad_legacy_f32_(_e836, _e843, _e826);
                let _e845: f32 = mad_legacy_f32_(_e837, _e843, _e824);
                let _e846: f32 = mad_legacy_f32_(_e838, _e843, _e822);
                phi_961_ = _e844;
                phi_962_ = _e845;
                phi_963_ = _e846;
            } else {
                let _e847: f32 = (_e477 - _e829.w);
                let _e848: f32 = mad_legacy_f32_(_e836, _e847, _e826);
                let _e849: f32 = mad_legacy_f32_(_e837, _e847, _e824);
                let _e850: f32 = mad_legacy_f32_(_e838, _e847, _e822);
                phi_961_ = _e848;
                phi_962_ = _e849;
                phi_963_ = _e850;
            }
            let _e852: f32 = phi_961_;
            let _e854: f32 = phi_962_;
            let _e856: f32 = phi_963_;
            phi_964_ = _e856;
            phi_965_ = _e854;
            phi_966_ = _e852;
        }
        let _e858: f32 = phi_964_;
        let _e860: f32 = phi_965_;
        let _e862: f32 = phi_966_;
        phi_1018_ = _e477;
        phi_1019_ = 0f;
        phi_1020_ = 0f;
        phi_1021_ = 0f;
        phi_1022_ = _e858;
        phi_1023_ = _e860;
        phi_1024_ = _e862;
        if (_e477 > 0f) {
            let _e872: vec4<f32> = cF.m[53u];
            let _e882: vec4<f32> = cF.m[54u];
            let _e886: f32 = mad_legacy_f32_(fract((_e172 * 700.567f)), _e872.x, _e882.x);
            let _e887: f32 = mad_legacy_f32_(fract((_e180 * 800.567f)), _e872.y, _e882.y);
            let _e888: f32 = mad_legacy_f32_(fract((_e158 * 900.567f)), _e872.z, _e882.z);
            if (clamp(select(0f, 1f, (_e477 < _e872.w)), 0f, 1f) > 0f) {
                let _e892: f32 = mul_legacy_f32_(_e477, (1f / _e872.w));
                let _e893: f32 = clamp(_e892, 0f, 1f);
                let _e894: f32 = mul_legacy_f32_(_e893, _e886);
                let _e895: f32 = mul_legacy_f32_(_e893, _e887);
                let _e896: f32 = mul_legacy_f32_(_e893, _e888);
                let _e897: f32 = mad_legacy_f32_(_e894, _e477, _e862);
                let _e898: f32 = mad_legacy_f32_(_e895, _e477, _e860);
                let _e899: f32 = mad_legacy_f32_(_e896, _e477, _e858);
                phi_1012_ = _e897;
                phi_1013_ = _e898;
                phi_1014_ = _e899;
                phi_1015_ = _e894;
                phi_1016_ = _e895;
                phi_1017_ = _e896;
            } else {
                let _e900: f32 = mad_legacy_f32_(_e886, _e872.w, _e862);
                let _e901: f32 = mad_legacy_f32_(_e887, _e872.w, _e860);
                let _e902: f32 = mad_legacy_f32_(_e888, _e872.w, _e858);
                phi_1012_ = _e900;
                phi_1013_ = _e901;
                phi_1014_ = _e902;
                phi_1015_ = _e886;
                phi_1016_ = _e887;
                phi_1017_ = _e888;
            }
            let _e904: f32 = phi_1012_;
            let _e906: f32 = phi_1013_;
            let _e908: f32 = phi_1014_;
            let _e910: f32 = phi_1015_;
            let _e912: f32 = phi_1016_;
            let _e914: f32 = phi_1017_;
            phi_1018_ = (_e477 - _e872.w);
            phi_1019_ = _e914;
            phi_1020_ = _e912;
            phi_1021_ = _e910;
            phi_1022_ = _e908;
            phi_1023_ = _e906;
            phi_1024_ = _e904;
        }
        let _e916: f32 = phi_1018_;
        let _e918: f32 = phi_1019_;
        let _e920: f32 = phi_1020_;
        let _e922: f32 = phi_1021_;
        let _e924: f32 = phi_1022_;
        let _e926: f32 = phi_1023_;
        let _e928: f32 = phi_1024_;
        phi_1079_ = _e918;
        phi_1080_ = _e920;
        phi_1081_ = _e922;
        phi_1082_ = _e916;
        phi_1083_ = _e924;
        phi_1084_ = _e926;
        phi_1085_ = _e928;
        if (_e916 > 0f) {
            let _e938: vec4<f32> = cF.m[55u];
            let _e944: vec4<f32> = cF.m[56u];
            let _e948: f32 = mad_legacy_f32_(fract((_e172 * 700.567f)), _e938.x, _e944.x);
            let _e949: f32 = mad_legacy_f32_(fract((_e180 * 800.567f)), _e938.y, _e944.y);
            let _e950: f32 = mad_legacy_f32_(fract((_e158 * 900.567f)), _e938.z, _e944.z);
            if (clamp(select(0f, 1f, (_e916 < _e938.w)), 0f, 1f) > 0f) {
                let _e961: f32 = mul_legacy_f32_(_e916, (1f / _e938.w));
                let _e962: f32 = clamp(_e961, 0f, 1f);
                let _e963: f32 = mad_legacy_f32_(_e962, (_e948 - _e922), _e922);
                let _e964: f32 = mad_legacy_f32_(_e962, (_e949 - _e920), _e920);
                let _e965: f32 = mad_legacy_f32_(_e962, (_e950 - _e918), _e918);
                let _e966: f32 = mad_legacy_f32_(_e963, _e916, _e928);
                let _e967: f32 = mad_legacy_f32_(_e964, _e916, _e926);
                let _e968: f32 = mad_legacy_f32_(_e965, _e916, _e924);
                phi_1073_ = _e966;
                phi_1074_ = _e967;
                phi_1075_ = _e968;
                phi_1076_ = _e963;
                phi_1077_ = _e964;
                phi_1078_ = _e965;
            } else {
                let _e969: f32 = mad_legacy_f32_(_e948, _e938.w, _e928);
                let _e970: f32 = mad_legacy_f32_(_e949, _e938.w, _e926);
                let _e971: f32 = mad_legacy_f32_(_e950, _e938.w, _e924);
                phi_1073_ = _e969;
                phi_1074_ = _e970;
                phi_1075_ = _e971;
                phi_1076_ = _e948;
                phi_1077_ = _e949;
                phi_1078_ = _e950;
            }
            let _e973: f32 = phi_1073_;
            let _e975: f32 = phi_1074_;
            let _e977: f32 = phi_1075_;
            let _e979: f32 = phi_1076_;
            let _e981: f32 = phi_1077_;
            let _e983: f32 = phi_1078_;
            phi_1079_ = _e983;
            phi_1080_ = _e981;
            phi_1081_ = _e979;
            phi_1082_ = (_e916 - _e938.w);
            phi_1083_ = _e977;
            phi_1084_ = _e975;
            phi_1085_ = _e973;
        }
        let _e985: f32 = phi_1079_;
        let _e987: f32 = phi_1080_;
        let _e989: f32 = phi_1081_;
        let _e991: f32 = phi_1082_;
        let _e993: f32 = phi_1083_;
        let _e995: f32 = phi_1084_;
        let _e997: f32 = phi_1085_;
        phi_1123_ = _e997;
        phi_1124_ = _e995;
        phi_1125_ = _e993;
        if (_e991 > 0f) {
            let _e1007: vec4<f32> = cF.m[57u];
            let _e1013: vec4<f32> = cF.m[58u];
            let _e1017: f32 = mad_legacy_f32_(fract((_e172 * 700.567f)), _e1007.x, _e1013.x);
            let _e1018: f32 = mad_legacy_f32_(fract((_e180 * 800.567f)), _e1007.y, _e1013.y);
            let _e1019: f32 = mad_legacy_f32_(fract((_e158 * 900.567f)), _e1007.z, _e1013.z);
            let _e1025: f32 = mul_legacy_f32_(_e991, (1f / _e1007.w));
            let _e1026: f32 = clamp(_e1025, 0f, 1f);
            let _e1027: f32 = mad_legacy_f32_(_e1026, (_e1017 - _e989), _e989);
            let _e1028: f32 = mad_legacy_f32_(_e1026, (_e1018 - _e987), _e987);
            let _e1029: f32 = mad_legacy_f32_(_e1026, (_e1019 - _e985), _e985);
            let _e1030: f32 = mad_legacy_f32_(_e1027, _e991, _e997);
            let _e1031: f32 = mad_legacy_f32_(_e1028, _e991, _e995);
            let _e1032: f32 = mad_legacy_f32_(_e1029, _e991, _e993);
            phi_1123_ = _e1030;
            phi_1124_ = _e1031;
            phi_1125_ = _e1032;
        }
        let _e1034: f32 = phi_1123_;
        let _e1036: f32 = phi_1124_;
        let _e1038: f32 = phi_1125_;
        let _e1040: f32 = v3_color_1[3u];
        phi_1141_ = true;
        phi_1142_ = true;
        if !((_e1040 < 0.15f)) {
            phi_1138_ = true;
            phi_1139_ = true;
            if !((_e1040 < 0.45f)) {
                phi_1138_ = !((_e1040 < 0.75f));
                phi_1139_ = false;
            }
            let _e1048: bool = phi_1138_;
            let _e1050: bool = phi_1139_;
            phi_1141_ = _e1050;
            phi_1142_ = !(_e1048);
        }
        let _e1053: bool = phi_1141_;
        let _e1055: bool = phi_1142_;
        let _e1059: f32 = (select(1f, 0f, _e1053) - 0.5f);
        let _e1070: vec4<f32> = cF.m[15u];
        let _e1073: f32 = mad_legacy_f32_(fract((_e172 * 700.123f)), _e1070.x, _e1070.y);
        let _e1074: f32 = mul_legacy_f32_((select(1f, 0f, _e1055) - 0.5f), _e1073);
        if (_e1070.z < 0f) {
            let _e1077: f32 = mul_legacy_f32_(_e1059, _e1073);
            phi_1171_ = _e1077;
        } else {
            let _e1079: f32 = mad_legacy_f32_(fract((_e180 * 800.123f)), _e1070.z, _e1070.w);
            let _e1080: f32 = mul_legacy_f32_(_e1059, _e1079);
            phi_1171_ = _e1080;
        }
        let _e1082: f32 = phi_1171_;
        phi_1193_ = _e197;
        phi_1194_ = 1f;
        phi_1195_ = 1f;
        if _e392 {
            let _e1085: vec4<f32> = cF.m[32u];
            let _e1088: f32 = mad_legacy_f32_(_e196, _e195, -(_e1085.w));
            phi_1191_ = _e1085.x;
            phi_1192_ = _e1085.y;
            if (_e197 < _e1085.w) {
                let _e1095: f32 = mul_legacy_f32_(_e197, (1f / _e1085.w));
                let _e1096: f32 = mad_legacy_f32_(_e1095, (_e1085.x - 1f), 1f);
                let _e1097: f32 = mad_legacy_f32_(_e1095, (_e1085.y - 1f), 1f);
                phi_1191_ = _e1096;
                phi_1192_ = _e1097;
            }
            let _e1099: f32 = phi_1191_;
            let _e1101: f32 = phi_1192_;
            phi_1193_ = _e1088;
            phi_1194_ = _e1101;
            phi_1195_ = _e1099;
        }
        let _e1103: f32 = phi_1193_;
        let _e1105: f32 = phi_1194_;
        let _e1107: f32 = phi_1195_;
        phi_1217_ = _e1103;
        phi_1218_ = _e1105;
        phi_1219_ = _e1107;
        if (_e1103 > 0f) {
            let _e1111: vec4<f32> = cF.m[33u];
            phi_1215_ = _e1111.x;
            phi_1216_ = _e1111.y;
            if (_e1103 < _e1111.w) {
                let _e1120: f32 = mul_legacy_f32_(_e1103, (1f / _e1111.w));
                let _e1121: f32 = mad_legacy_f32_(_e1120, (_e1111.x - _e1107), _e1107);
                let _e1122: f32 = mad_legacy_f32_(_e1120, (_e1111.y - _e1105), _e1105);
                phi_1215_ = _e1121;
                phi_1216_ = _e1122;
            }
            let _e1124: f32 = phi_1215_;
            let _e1126: f32 = phi_1216_;
            phi_1217_ = (_e1103 - _e1111.w);
            phi_1218_ = _e1126;
            phi_1219_ = _e1124;
        }
        let _e1128: f32 = phi_1217_;
        let _e1130: f32 = phi_1218_;
        let _e1132: f32 = phi_1219_;
        phi_1241_ = _e1128;
        phi_1242_ = _e1130;
        phi_1243_ = _e1132;
        if (_e1128 > 0f) {
            let _e1136: vec4<f32> = cF.m[34u];
            phi_1239_ = _e1136.x;
            phi_1240_ = _e1136.y;
            if (_e1128 < _e1136.w) {
                let _e1145: f32 = mul_legacy_f32_(_e1128, (1f / _e1136.w));
                let _e1146: f32 = mad_legacy_f32_(_e1145, (_e1136.x - _e1132), _e1132);
                let _e1147: f32 = mad_legacy_f32_(_e1145, (_e1136.y - _e1130), _e1130);
                phi_1239_ = _e1146;
                phi_1240_ = _e1147;
            }
            let _e1149: f32 = phi_1239_;
            let _e1151: f32 = phi_1240_;
            phi_1241_ = (_e1128 - _e1136.w);
            phi_1242_ = _e1151;
            phi_1243_ = _e1149;
        }
        let _e1153: f32 = phi_1241_;
        let _e1155: f32 = phi_1242_;
        let _e1157: f32 = phi_1243_;
        phi_1264_ = _e1155;
        phi_1265_ = _e1157;
        if (_e1153 > 0f) {
            let _e1161: vec4<f32> = cF.m[35u];
            phi_1262_ = _e1161.x;
            phi_1263_ = _e1161.y;
            if (_e1153 < _e1161.w) {
                let _e1169: f32 = mul_legacy_f32_(_e1153, (1f / _e1161.w));
                let _e1170: f32 = mad_legacy_f32_(_e1169, (_e1161.x - _e1157), _e1157);
                let _e1171: f32 = mad_legacy_f32_(_e1169, (_e1161.y - _e1155), _e1155);
                phi_1262_ = _e1170;
                phi_1263_ = _e1171;
            }
            let _e1173: f32 = phi_1262_;
            let _e1175: f32 = phi_1263_;
            phi_1264_ = _e1175;
            phi_1265_ = _e1173;
        }
        let _e1177: f32 = phi_1264_;
        let _e1179: f32 = phi_1265_;
        let _e1182: vec4<f32> = cF.m[16u];
        let _e1185: f32 = mad_legacy_f32_(fract((_e172 * 1000.123f)), _e1182.z, _e1182.w);
        let _e1188: f32 = mad_legacy_f32_(fract((_e158 * 900.123f)), _e1182.x, _e1182.y);
        let _e1189: f32 = mul_legacy_f32_(_e1185, _e197);
        let _e1194: f32 = fma(fract(fma(_e1189, 0.1591549f, 0.5f)), 6.283185f, -3.141593f);
        let _e1195: f32 = fma(fract(fma(_e1188, 0.1591549f, 0.5f)), 6.283185f, -3.141593f);
        let _e1196: f32 = mul_legacy_f32_(_e1074, _e1179);
        let _e1197: f32 = mul_legacy_f32_(_e1082, _e1177);
        let _e1198: f32 = cos(_e1195);
        let _e1199: f32 = sin(_e1195);
        let _e1202: vec4<f32> = cF.m[65u];
        let _e1204: f32 = mul_legacy_f32_(_e1196, _e1202.w);
        let _e1205: f32 = mul_legacy_f32_(_e1197, _e1202.w);
        let _e1206: f32 = cos(_e1194);
        let _e1207: f32 = sin(_e1194);
        let _e1208: f32 = mul_legacy_f32_(_e1198, _e1205);
        let _e1209: f32 = mad_legacy_f32_(_e1199, _e1204, _e1208);
        let _e1210: f32 = mul_legacy_f32_(_e1207, _e1209);
        let _e1211: f32 = mul_legacy_f32_(_e1199, _e1205);
        let _e1213: f32 = mad_legacy_f32_(_e1198, _e1204, -(_e1211));
        let _e1214: f32 = mul_legacy_f32_(_e1206, _e1209);
        let _e1215: f32 = mad_legacy_f32_(_e1207, _e1213, _e1214);
        let _e1217: f32 = mad_legacy_f32_(_e1206, _e1213, -(_e1210));
        let _e1220: vec4<f32> = cF.m[21u];
        phi_1353_ = _e197;
        phi_1354_ = 0f;
        phi_1355_ = 1f;
        phi_1356_ = 1f;
        phi_1357_ = 1f;
        if _e392 {
            let _e1231: vec4<f32> = cF.m[22u];
            let _e1238: vec4<f32> = cF.m[23u];
            let _e1243: f32 = mad_legacy_f32_(fract((_e172 * 700.789f)), _e1231.x, _e1238.x);
            let _e1244: f32 = mad_legacy_f32_(fract((_e180 * 800.789f)), _e1231.y, _e1238.y);
            let _e1245: f32 = mad_legacy_f32_(fract((_e158 * 900.789f)), _e1231.z, _e1238.z);
            let _e1246: f32 = mad_legacy_f32_(fract((_e172 * 1000.789f)), _e1231.w, _e1238.w);
            phi_1349_ = _e1243;
            phi_1350_ = _e1244;
            phi_1351_ = _e1245;
            phi_1352_ = _e1246;
            if (_e197 < _e1220.x) {
                let _e1254: f32 = mul_legacy_f32_(_e197, (1f / _e1220.x));
                let _e1255: f32 = mad_legacy_f32_(_e1254, (_e1243 - 1f), 1f);
                let _e1256: f32 = mad_legacy_f32_(_e1254, (_e1244 - 1f), 1f);
                let _e1257: f32 = mad_legacy_f32_(_e1254, (_e1245 - 1f), 1f);
                let _e1258: f32 = mul_legacy_f32_(_e1254, _e1246);
                phi_1349_ = _e1255;
                phi_1350_ = _e1256;
                phi_1351_ = _e1257;
                phi_1352_ = _e1258;
            }
            let _e1260: f32 = phi_1349_;
            let _e1262: f32 = phi_1350_;
            let _e1264: f32 = phi_1351_;
            let _e1266: f32 = phi_1352_;
            phi_1353_ = (_e197 - _e1220.x);
            phi_1354_ = _e1266;
            phi_1355_ = _e1264;
            phi_1356_ = _e1262;
            phi_1357_ = _e1260;
        }
        let _e1268: f32 = phi_1353_;
        let _e1270: f32 = phi_1354_;
        let _e1272: f32 = phi_1355_;
        let _e1274: f32 = phi_1356_;
        let _e1276: f32 = phi_1357_;
        phi_1406_ = _e1268;
        phi_1407_ = _e1270;
        phi_1408_ = _e1272;
        phi_1409_ = _e1274;
        phi_1410_ = _e1276;
        if (_e1268 > 0f) {
            let _e1288: vec4<f32> = cF.m[24u];
            let _e1295: vec4<f32> = cF.m[25u];
            let _e1300: f32 = mad_legacy_f32_(fract((_e172 * 700.789f)), _e1288.x, _e1295.x);
            let _e1301: f32 = mad_legacy_f32_(fract((_e180 * 800.789f)), _e1288.y, _e1295.y);
            let _e1302: f32 = mad_legacy_f32_(fract((_e158 * 900.789f)), _e1288.z, _e1295.z);
            let _e1303: f32 = mad_legacy_f32_(fract((_e172 * 1000.789f)), _e1288.w, _e1295.w);
            phi_1402_ = _e1300;
            phi_1403_ = _e1301;
            phi_1404_ = _e1302;
            phi_1405_ = _e1303;
            if (_e1268 < _e1220.y) {
                let _e1312: f32 = mul_legacy_f32_(_e1268, (1f / _e1220.y));
                let _e1313: f32 = mad_legacy_f32_(_e1312, (_e1300 - _e1276), _e1276);
                let _e1314: f32 = mad_legacy_f32_(_e1312, (_e1301 - _e1274), _e1274);
                let _e1315: f32 = mad_legacy_f32_(_e1312, (_e1302 - _e1272), _e1272);
                let _e1316: f32 = mad_legacy_f32_(_e1312, (_e1303 - _e1270), _e1270);
                phi_1402_ = _e1313;
                phi_1403_ = _e1314;
                phi_1404_ = _e1315;
                phi_1405_ = _e1316;
            }
            let _e1318: f32 = phi_1402_;
            let _e1320: f32 = phi_1403_;
            let _e1322: f32 = phi_1404_;
            let _e1324: f32 = phi_1405_;
            phi_1406_ = (_e1268 - _e1220.y);
            phi_1407_ = _e1324;
            phi_1408_ = _e1322;
            phi_1409_ = _e1320;
            phi_1410_ = _e1318;
        }
        let _e1326: f32 = phi_1406_;
        let _e1328: f32 = phi_1407_;
        let _e1330: f32 = phi_1408_;
        let _e1332: f32 = phi_1409_;
        let _e1334: f32 = phi_1410_;
        phi_1459_ = _e1326;
        phi_1460_ = _e1328;
        phi_1461_ = _e1330;
        phi_1462_ = _e1332;
        phi_1463_ = _e1334;
        if (_e1326 > 0f) {
            let _e1346: vec4<f32> = cF.m[26u];
            let _e1353: vec4<f32> = cF.m[27u];
            let _e1358: f32 = mad_legacy_f32_(fract((_e172 * 700.789f)), _e1346.x, _e1353.x);
            let _e1359: f32 = mad_legacy_f32_(fract((_e180 * 800.789f)), _e1346.y, _e1353.y);
            let _e1360: f32 = mad_legacy_f32_(fract((_e158 * 900.789f)), _e1346.z, _e1353.z);
            let _e1361: f32 = mad_legacy_f32_(fract((_e172 * 1000.789f)), _e1346.w, _e1353.w);
            phi_1455_ = _e1358;
            phi_1456_ = _e1359;
            phi_1457_ = _e1360;
            phi_1458_ = _e1361;
            if (_e1326 < _e1220.z) {
                let _e1370: f32 = mul_legacy_f32_(_e1326, (1f / _e1220.z));
                let _e1371: f32 = mad_legacy_f32_(_e1370, (_e1358 - _e1334), _e1334);
                let _e1372: f32 = mad_legacy_f32_(_e1370, (_e1359 - _e1332), _e1332);
                let _e1373: f32 = mad_legacy_f32_(_e1370, (_e1360 - _e1330), _e1330);
                let _e1374: f32 = mad_legacy_f32_(_e1370, (_e1361 - _e1328), _e1328);
                phi_1455_ = _e1371;
                phi_1456_ = _e1372;
                phi_1457_ = _e1373;
                phi_1458_ = _e1374;
            }
            let _e1376: f32 = phi_1455_;
            let _e1378: f32 = phi_1456_;
            let _e1380: f32 = phi_1457_;
            let _e1382: f32 = phi_1458_;
            phi_1459_ = (_e1326 - _e1220.z);
            phi_1460_ = _e1382;
            phi_1461_ = _e1380;
            phi_1462_ = _e1378;
            phi_1463_ = _e1376;
        }
        let _e1384: f32 = phi_1459_;
        let _e1386: f32 = phi_1460_;
        let _e1388: f32 = phi_1461_;
        let _e1390: f32 = phi_1462_;
        let _e1392: f32 = phi_1463_;
        phi_1512_ = _e1384;
        phi_1513_ = _e1386;
        phi_1514_ = _e1388;
        phi_1515_ = _e1390;
        phi_1516_ = _e1392;
        if (_e1384 > 0f) {
            let _e1404: vec4<f32> = cF.m[28u];
            let _e1411: vec4<f32> = cF.m[29u];
            let _e1416: f32 = mad_legacy_f32_(fract((_e172 * 700.789f)), _e1404.x, _e1411.x);
            let _e1417: f32 = mad_legacy_f32_(fract((_e180 * 800.789f)), _e1404.y, _e1411.y);
            let _e1418: f32 = mad_legacy_f32_(fract((_e158 * 900.789f)), _e1404.z, _e1411.z);
            let _e1419: f32 = mad_legacy_f32_(fract((_e172 * 1000.789f)), _e1404.w, _e1411.w);
            phi_1508_ = _e1416;
            phi_1509_ = _e1417;
            phi_1510_ = _e1418;
            phi_1511_ = _e1419;
            if (_e1384 < _e1220.w) {
                let _e1428: f32 = mul_legacy_f32_(_e1384, (1f / _e1220.w));
                let _e1429: f32 = mad_legacy_f32_(_e1428, (_e1416 - _e1392), _e1392);
                let _e1430: f32 = mad_legacy_f32_(_e1428, (_e1417 - _e1390), _e1390);
                let _e1431: f32 = mad_legacy_f32_(_e1428, (_e1418 - _e1388), _e1388);
                let _e1432: f32 = mad_legacy_f32_(_e1428, (_e1419 - _e1386), _e1386);
                phi_1508_ = _e1429;
                phi_1509_ = _e1430;
                phi_1510_ = _e1431;
                phi_1511_ = _e1432;
            }
            let _e1434: f32 = phi_1508_;
            let _e1436: f32 = phi_1509_;
            let _e1438: f32 = phi_1510_;
            let _e1440: f32 = phi_1511_;
            phi_1512_ = (_e1384 - _e1220.w);
            phi_1513_ = _e1440;
            phi_1514_ = _e1438;
            phi_1515_ = _e1436;
            phi_1516_ = _e1434;
        }
        let _e1442: f32 = phi_1512_;
        let _e1444: f32 = phi_1513_;
        let _e1446: f32 = phi_1514_;
        let _e1448: f32 = phi_1515_;
        let _e1450: f32 = phi_1516_;
        phi_1567_ = _e1444;
        phi_1568_ = _e1450;
        phi_1569_ = _e1448;
        phi_1570_ = _e1446;
        if (_e1442 > 0f) {
            let _e1462: vec4<f32> = cF.m[30u];
            let _e1469: vec4<f32> = cF.m[31u];
            let _e1474: f32 = mad_legacy_f32_(fract((_e172 * 700.789f)), _e1462.x, _e1469.x);
            let _e1475: f32 = mad_legacy_f32_(fract((_e180 * 800.789f)), _e1462.y, _e1469.y);
            let _e1476: f32 = mad_legacy_f32_(fract((_e158 * 900.789f)), _e1462.z, _e1469.z);
            let _e1477: f32 = mad_legacy_f32_(fract((_e172 * 1000.789f)), _e1462.w, _e1469.w);
            let _e1480: vec4<f32> = cF.m[20u];
            phi_1563_ = _e1477;
            phi_1564_ = _e1474;
            phi_1565_ = _e1475;
            phi_1566_ = _e1476;
            if (_e1442 < _e1480.x) {
                let _e1488: f32 = mul_legacy_f32_(_e1442, (1f / _e1480.x));
                let _e1489: f32 = mad_legacy_f32_(_e1488, (_e1474 - _e1450), _e1450);
                let _e1490: f32 = mad_legacy_f32_(_e1488, (_e1475 - _e1448), _e1448);
                let _e1491: f32 = mad_legacy_f32_(_e1488, (_e1476 - _e1446), _e1446);
                let _e1492: f32 = mad_legacy_f32_(_e1488, (_e1477 - _e1444), _e1444);
                phi_1563_ = _e1492;
                phi_1564_ = _e1489;
                phi_1565_ = _e1490;
                phi_1566_ = _e1491;
            }
            let _e1494: f32 = phi_1563_;
            let _e1496: f32 = phi_1564_;
            let _e1498: f32 = phi_1565_;
            let _e1500: f32 = phi_1566_;
            phi_1567_ = _e1494;
            phi_1568_ = _e1496;
            phi_1569_ = _e1498;
            phi_1570_ = _e1500;
        }
        let _e1502: f32 = phi_1567_;
        let _e1504: f32 = phi_1568_;
        let _e1506: f32 = phi_1569_;
        let _e1508: f32 = phi_1570_;
        let _e1513: vec4<f32> = cF.m[64u];
        let _e1516: f32 = mul_legacy_f32_(_e1513.z, _e1513.w);
        let _e1517: f32 = mul_legacy_f32_(_e1516, fract((_e158 * 900.1793f)));
        let _e1520: vec4<f32> = cF.m[65u];
        let _e1523: f32 = mad_legacy_f32_(_e197, (1f / _e1520.x), _e1517);
        let _e1524: f32 = floor(_e1523);
        if (_e1520.y != 0f) {
            let _e1528: f32 = mul_legacy_f32_(_e1524, (1f / _e1513.w));
            let _e1532: f32 = mul_legacy_f32_(fract(abs(_e1528)), abs(_e1513.w));
            phi_1602_ = _e1532;
            if (_e1524 < 0f) {
                phi_1602_ = -(_e1532);
            }
            let _e1536: f32 = phi_1602_;
            phi_1604_ = _e1536;
        } else {
            phi_1604_ = min(_e1524, _e1513.w);
        }
        let _e1539: f32 = phi_1604_;
        let _e1541: f32 = mul_legacy_f32_(_e1539, _e1513.x);
        let _e1542: f32 = fract(_e1541);
        let _e1546: f32 = mad_legacy_f32_(floor(_e1541), _e1513.y, select(_e1513.y, 0f, _e1053));
        let _e1550: vec4<f32> = vec4<f32>(_e1034, _e1036, _e1038, 1f);
        let _e1553: vec4<f32> = cF.m[2u];
        let _e1556: vec4<f32> = cF.m[1u];
        let _e1559: vec4<f32> = cF.m[0u];
        if (_e143.z < 0.5f) {
            let _e1560: f32 = dp4_f32_legacy(_e1550, _e1553);
            let _e1561: f32 = dp4_f32_legacy(_e1550, _e1556);
            let _e1562: f32 = dp4_f32_legacy(_e1550, _e1559);
            phi_1671_ = _e1560;
            phi_1672_ = _e1561;
            phi_1673_ = _e1562;
        } else {
            phi_1668_ = _e1034;
            phi_1669_ = _e1036;
            phi_1670_ = _e1038;
            if (_e143.z < 1.5f) {
                let _e1564: f32 = (_e331 * 2f);
                let _e1565: f32 = (_e333 * 2f);
                let _e1566: f32 = mul_legacy_f32_(_e335, _e1564);
                let _e1567: f32 = mul_legacy_f32_(_e333, _e1564);
                let _e1568: f32 = (_e335 * 2f);
                let _e1569: f32 = mul_legacy_f32_(_e333, _e1565);
                let _e1570: f32 = -(_e1568);
                let _e1572: f32 = mad_legacy_f32_(_e335, _e1570, -(_e1569));
                let _e1574: f32 = mul_legacy_f32_(_e337, (_e337 * 2f));
                let _e1576: f32 = -(_e1574);
                let _e1577: f32 = mad_legacy_f32_(_e333, -(_e1565), _e1576);
                let _e1579: f32 = mad_legacy_f32_(_e337, _e1568, -(_e1567));
                let _e1580: f32 = mad_legacy_f32_(_e337, _e1565, _e1566);
                let _e1583: f32 = dp4_f32_legacy(vec4<f32>(_e1579, _e1580, (_e1572 + 1f), _e339), _e1550);
                let _e1584: f32 = mul_legacy_f32_(_e337, _e1564);
                let _e1585: f32 = mad_legacy_f32_(_e335, _e1570, _e1576);
                let _e1586: f32 = mad_legacy_f32_(_e333, _e1568, _e1584);
                let _e1589: f32 = mad_legacy_f32_(_e337, _e1565, -(_e1566));
                let _e1591: f32 = dp4_f32_legacy(_e1550, vec4<f32>(_e1586, (_e1585 + 1f), _e1589, _e341));
                let _e1594: f32 = mad_legacy_f32_(_e333, _e1568, -(_e1584));
                let _e1595: f32 = mad_legacy_f32_(_e337, _e1568, _e1567);
                let _e1597: f32 = dp4_f32_legacy(_e1550, vec4<f32>((_e1577 + 1f), _e1594, _e1595, _e343));
                phi_1668_ = _e1597;
                phi_1669_ = _e1591;
                phi_1670_ = _e1583;
            }
            let _e1599: f32 = phi_1668_;
            let _e1601: f32 = phi_1669_;
            let _e1603: f32 = phi_1670_;
            phi_1671_ = _e1603;
            phi_1672_ = _e1601;
            phi_1673_ = _e1599;
        }
        let _e1605: f32 = phi_1671_;
        let _e1607: f32 = phi_1672_;
        let _e1609: f32 = phi_1673_;
        let _e1612: vec4<f32> = cF.m[66u];
        let _e1627: vec4<f32> = cF.m[67u];
        let _e1629: f32 = (1f / _e1627.x);
        let _e1631: f32 = (1f / _e1627.z);
        let _e1633: f32 = (1f / _e1627.y);
        let _e1634: f32 = mul_legacy_f32_((_e1609 - _e1612.x), _e1629);
        let _e1635: f32 = mul_legacy_f32_((_e1607 - _e1612.y), _e1633);
        let _e1636: f32 = mul_legacy_f32_((_e1605 - _e1612.z), _e1631);
        let _e1646: f32 = abs(_e1627.x);
        let _e1647: f32 = abs(_e1627.y);
        let _e1648: f32 = abs(_e1627.z);
        let _e1649: bool = (abs(select(0f, 1f, (_e1609 < _e1612.x))) > 0f);
        let _e1651: bool = (abs(select(0f, 1f, (_e1607 < _e1612.y))) > 0f);
        let _e1653: bool = (abs(select(0f, 1f, (_e1605 < _e1612.z))) > 0f);
        let _e1655: f32 = mul_legacy_f32_(fract(abs(_e1634)), _e1646);
        let _e1656: f32 = mul_legacy_f32_(fract(abs(_e1635)), _e1647);
        let _e1657: f32 = mul_legacy_f32_(fract(abs(_e1636)), _e1648);
        let _e1661: f32 = mul_legacy_f32_(_e1655, (1f - select(0f, 1f, _e1649)));
        let _e1662: f32 = mul_legacy_f32_(_e1656, (1f - select(0f, 1f, _e1651)));
        let _e1663: f32 = mul_legacy_f32_(_e1657, (1f - select(0f, 1f, _e1653)));
        let _e1665: f32 = select(_e1661, (_e1661 - _e1655), _e1649);
        let _e1667: f32 = select(_e1662, (_e1662 - _e1656), _e1651);
        let _e1669: f32 = select(_e1663, (_e1663 - _e1657), _e1653);
        let _e1672: vec4<f32> = cF.m[68u];
        let _e1679: f32 = mul_legacy_f32_(_e1629, (_e1665 + _e1672.x));
        let _e1680: f32 = mul_legacy_f32_(_e1633, (_e1667 + _e1672.y));
        let _e1681: f32 = mul_legacy_f32_(_e1631, (_e1669 + _e1672.z));
        let _e1694: bool = (abs(select(0f, 1f, (_e1665 < -(_e1672.x)))) > 0f);
        let _e1696: bool = (abs(select(0f, 1f, (_e1667 < -(_e1672.y)))) > 0f);
        let _e1698: bool = (abs(select(0f, 1f, (_e1669 < -(_e1672.z)))) > 0f);
        let _e1706: f32 = mul_legacy_f32_(_e1646, fract(abs(_e1679)));
        let _e1707: f32 = mul_legacy_f32_(_e1647, fract(abs(_e1680)));
        let _e1708: f32 = mul_legacy_f32_(_e1648, fract(abs(_e1681)));
        let _e1712: f32 = mul_legacy_f32_(_e1706, (1f - select(0f, 1f, _e1694)));
        let _e1713: f32 = mul_legacy_f32_(_e1707, (1f - select(0f, 1f, _e1696)));
        let _e1714: f32 = mul_legacy_f32_(_e1708, (1f - select(0f, 1f, _e1698)));
        let _e1721: f32 = (select(_e1712, (_e1712 - _e1706), _e1694) + _e1612.x);
        let _e1722: f32 = (select(_e1713, (_e1713 - _e1707), _e1696) + _e1612.y);
        let _e1723: f32 = (select(_e1714, (_e1714 - _e1708), _e1698) + _e1612.z);
        let _e1726: vec4<f32> = cF.m[69u];
        let _e1733: vec3<f32> = vec3<f32>((_e1721 - _e1726.x), (_e1722 - _e1726.y), (_e1723 - _e1726.z));
        let _e1734: f32 = dp3_f32_(_e1733, _e1733);
        let _e1735: f32 = sqrt(_e1734);
        let _e1738: vec4<f32> = cF.m[67u];
        let _e1744: vec4<f32> = cF.m[66u];
        let _e1747: f32 = mul_legacy_f32_(_e1735, (1f / _e1744.w));
        let _e1748: f32 = clamp(_e1747, 0f, 1f);
        let _e1751: f32 = mul_legacy_f32_((_e1735 - _e1738.w), (1f / fma(_e1738.w, 0.5f, -(_e1738.w))));
        let _e1752: f32 = clamp(_e1751, 0f, 1f);
        let _e1756: f32 = mul_legacy_f32_((_e1748 * _e1748), fma(-(_e1748), 2f, 3f));
        let _e1757: f32 = mul_legacy_f32_(_e1502, _e1756);
        let _e1761: f32 = mul_legacy_f32_((_e1752 * _e1752), fma(-(_e1752), 2f, 3f));
        let _e1762: f32 = mul_legacy_f32_(_e1757, _e1761);
        if (_e143.w > 0.5f) {
            let _e1768: bool = (_e143.w > 1.5f);
            let _e1769: f32 = select(_e219.z, _e1553.y, _e1768);
            let _e1770: f32 = select(_e219.y, _e1556.y, _e1768);
            let _e1771: f32 = select(_e219.x, _e1559.y, _e1768);
            let _e1772: vec3<f32> = vec3<f32>(_e1771, _e1770, _e1769);
            let _e1773: f32 = dp3_f32_(_e1772, _e1772);
            let _e1774: f32 = inverseSqrt(_e1773);
            let _e1778: vec3<f32> = vec3<f32>(_e272.x, _e272.y, _e272.z);
            let _e1779: f32 = dp3_f32_(_e1778, _e1778);
            let _e1780: f32 = inverseSqrt(_e1779);
            let _e1781: f32 = mul_legacy_f32_(_e1774, _e1771);
            let _e1782: f32 = mul_legacy_f32_(_e1774, _e1770);
            let _e1783: f32 = mul_legacy_f32_(_e1774, _e1769);
            let _e1784: f32 = mul_legacy_f32_(_e1780, _e272.x);
            let _e1785: f32 = mul_legacy_f32_(_e1780, _e272.y);
            let _e1786: f32 = mul_legacy_f32_(_e1780, _e272.z);
            let _e1787: f32 = mul_legacy_f32_(_e1783, _e1785);
            let _e1788: f32 = mul_legacy_f32_(_e1781, _e1786);
            let _e1789: f32 = mul_legacy_f32_(_e1782, _e1784);
            let _e1793: f32 = mad_legacy_f32_(_e1782, _e1786, -(_e1787));
            let _e1794: f32 = mad_legacy_f32_(_e1783, _e1784, -(_e1788));
            let _e1795: f32 = mad_legacy_f32_(_e1781, _e1785, -(_e1789));
            let _e1796: vec3<f32> = vec3<f32>(_e1793, _e1794, _e1795);
            let _e1797: f32 = dp3_f32_(_e1796, _e1796);
            let _e1798: f32 = inverseSqrt(_e1797);
            let _e1799: f32 = mul_legacy_f32_(_e1215, _e1781);
            let _e1800: f32 = mul_legacy_f32_(_e1215, _e1782);
            let _e1801: f32 = mul_legacy_f32_(_e1215, _e1783);
            let _e1802: f32 = mul_legacy_f32_(_e1798, _e1793);
            let _e1803: f32 = mul_legacy_f32_(_e1798, _e1794);
            let _e1804: f32 = mul_legacy_f32_(_e1798, _e1795);
            let _e1805: f32 = mad_legacy_f32_(_e1217, _e1802, _e1799);
            let _e1806: f32 = mad_legacy_f32_(_e1217, _e1803, _e1800);
            let _e1807: f32 = mad_legacy_f32_(_e1217, _e1804, _e1801);
            phi_1894_ = (_e1805 + _e1721);
            phi_1895_ = (_e1806 + _e1722);
            phi_1896_ = (_e1807 + _e1723);
        } else {
            let _e1811: vec3<f32> = vec3<f32>(_e1217, _e1215, 0f);
            let _e1815: f32 = dp3_f32_legacy(_e1811, vec3<f32>(_e1553.x, _e1553.y, _e1553.z));
            let _e1819: f32 = dp3_f32_legacy(_e1811, vec3<f32>(_e1559.x, _e1559.y, _e1559.z));
            let _e1823: f32 = dp3_f32_legacy(_e1811, vec3<f32>(_e1556.x, _e1556.y, _e1556.z));
            phi_1894_ = (_e1721 + _e1819);
            phi_1895_ = (_e1722 + _e1823);
            phi_1896_ = (_e1723 + _e1815);
        }
        let _e1828: f32 = phi_1894_;
        let _e1830: f32 = phi_1895_;
        let _e1832: f32 = phi_1896_;
        let _e1835: vec4<f32> = cF.m[18u];
        let _e1840: f32 = mul_legacy_f32_(_e1504, _e1835.x);
        let _e1841: f32 = mul_legacy_f32_(_e1506, _e1835.y);
        let _e1842: f32 = mul_legacy_f32_(_e1508, _e1835.z);
        let _e1843: f32 = mul_legacy_f32_(_e1762, _e1835.w);
        phi_1908_ = _e1546;
        phi_1909_ = select((_e1513.x + _e1542), _e1542, _e1055);
        phi_1910_ = 0f;
        phi_1911_ = 0f;
        phi_1912_ = 0f;
        phi_1913_ = 0f;
        phi_1914_ = _e1843;
        phi_1915_ = _e1842;
        phi_1916_ = _e1841;
        phi_1917_ = _e1840;
        phi_1918_ = _e1832;
        phi_1919_ = _e1830;
        phi_1920_ = _e1828;
    }
    let _e1845: f32 = phi_1908_;
    let _e1847: f32 = phi_1909_;
    let _e1849: f32 = phi_1910_;
    let _e1851: f32 = phi_1911_;
    let _e1853: f32 = phi_1912_;
    let _e1855: f32 = phi_1913_;
    let _e1857: f32 = phi_1914_;
    let _e1859: f32 = phi_1915_;
    let _e1861: f32 = phi_1916_;
    let _e1863: f32 = phi_1917_;
    let _e1865: f32 = phi_1918_;
    let _e1867: f32 = phi_1919_;
    let _e1869: f32 = phi_1920_;
    let _e1870: vec4<f32> = vec4<f32>(_e1869, _e1867, _e1865, 1f);
    let _e1873: vec4<f32> = cF.m[13u];
    let _e1874: f32 = dp4_f32_legacy(_e1870, _e1873);
    let _e1875: f32 = dp4_f32_legacy(_e1870, _e272);
    let _e1876: f32 = dp4_f32_legacy(_e1870, _e219);
    let _e1879: vec4<f32> = cF.m[10u];
    let _e1880: f32 = dp4_f32_legacy(_e1870, _e1879);
    let _e1883: vec4<f32> = cF.m[8u];
    o0_position0_[0u] = _e1880;
    o0_position0_[1u] = _e1876;
    o0_position0_[2u] = _e1875;
    o0_position0_[3u] = _e1874;
    o1_color[0u] = _e1863;
    o1_color[1u] = _e1861;
    o1_color[2u] = _e1859;
    o1_color[3u] = _e1857;
    o2_specular0_[0u] = _e1855;
    o2_specular0_[1u] = _e1853;
    o2_specular0_[2u] = _e1851;
    o2_specular0_[3u] = _e1849;
    o3_texcoord0_[0u] = (_e1847 + _e1883.z);
    o3_texcoord0_[1u] = (_e1845 + _e1883.w);
    o3_texcoord0_[2u] = 0f;
    o3_texcoord0_[3u] = 0f;
    return;
}

@vertex 
fn main(@location(0) v0_position0_: vec4<f32>, @location(1) v1_tangent: vec4<f32>, @location(2) v2_texcoord0_: vec4<f32>, @location(3) v3_color: vec4<f32>, @location(4) v4_specular0_: vec4<f32>) -> VertexOutput {
    v0_position0_1 = v0_position0_;
    v1_tangent_1 = v1_tangent;
    v2_texcoord0_1 = v2_texcoord0_;
    v3_color_1 = v3_color;
    v4_specular0_1 = v4_specular0_;
    main_1();
    let _e17: vec4<f32> = o0_position0_;
    let _e18: vec4<f32> = o1_color;
    let _e19: vec4<f32> = o2_specular0_;
    let _e20: vec4<f32> = o3_texcoord0_;
    return VertexOutput(_e17, _e18, _e19, _e20);
}
