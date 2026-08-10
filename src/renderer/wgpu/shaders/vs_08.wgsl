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
    var phi_501_: f32;
    var phi_502_: f32;
    var phi_503_: f32;
    var phi_504_: f32;
    var phi_505_: f32;
    var phi_506_: f32;
    var phi_532_: f32;
    var phi_533_: f32;
    var phi_534_: f32;
    var phi_535_: f32;
    var phi_536_: f32;
    var phi_561_: f32;
    var phi_562_: f32;
    var phi_563_: f32;
    var phi_564_: f32;
    var phi_565_: f32;
    var phi_594_: f32;
    var phi_595_: f32;
    var phi_634_: f32;
    var phi_635_: f32;
    var phi_636_: f32;
    var phi_637_: f32;
    var phi_638_: f32;
    var phi_639_: f32;
    var phi_661_: f32;
    var phi_662_: f32;
    var phi_663_: f32;
    var phi_664_: f32;
    var phi_665_: f32;
    var phi_666_: f32;
    var phi_699_: f32;
    var phi_700_: f32;
    var phi_701_: f32;
    var phi_702_: f32;
    var phi_703_: f32;
    var phi_704_: f32;
    var phi_751_: f32;
    var phi_752_: f32;
    var phi_753_: f32;
    var phi_754_: f32;
    var phi_755_: f32;
    var phi_756_: f32;
    var phi_797_: f32;
    var phi_798_: f32;
    var phi_799_: f32;
    var phi_800_: f32;
    var phi_801_: f32;
    var phi_802_: f32;
    var phi_843_: f32;
    var phi_844_: f32;
    var phi_845_: f32;
    var phi_846_: f32;
    var phi_847_: f32;
    var phi_848_: f32;
    var phi_895_: f32;
    var phi_896_: f32;
    var phi_897_: f32;
    var phi_898_: f32;
    var phi_899_: f32;
    var phi_900_: f32;
    var phi_930_: f32;
    var phi_931_: f32;
    var phi_932_: f32;
    var phi_933_: f32;
    var phi_934_: f32;
    var phi_935_: f32;
    var phi_965_: f32;
    var phi_966_: f32;
    var phi_967_: f32;
    var phi_968_: f32;
    var phi_969_: f32;
    var phi_970_: f32;
    var phi_1016_: f32;
    var phi_1017_: f32;
    var phi_1018_: f32;
    var phi_1019_: f32;
    var phi_1020_: f32;
    var phi_1021_: f32;
    var phi_1022_: f32;
    var phi_1023_: f32;
    var phi_1024_: f32;
    var phi_1025_: f32;
    var phi_1026_: f32;
    var phi_1027_: f32;
    var phi_1028_: f32;
    var phi_1077_: f32;
    var phi_1078_: f32;
    var phi_1079_: f32;
    var phi_1080_: f32;
    var phi_1081_: f32;
    var phi_1082_: f32;
    var phi_1083_: f32;
    var phi_1084_: f32;
    var phi_1085_: f32;
    var phi_1086_: f32;
    var phi_1087_: f32;
    var phi_1088_: f32;
    var phi_1089_: f32;
    var phi_1127_: f32;
    var phi_1128_: f32;
    var phi_1129_: f32;
    var phi_1142_: bool;
    var phi_1143_: bool;
    var phi_1145_: bool;
    var phi_1146_: bool;
    var phi_1175_: f32;
    var phi_1195_: f32;
    var phi_1196_: f32;
    var phi_1197_: f32;
    var phi_1198_: f32;
    var phi_1199_: f32;
    var phi_1219_: f32;
    var phi_1220_: f32;
    var phi_1221_: f32;
    var phi_1222_: f32;
    var phi_1223_: f32;
    var phi_1243_: f32;
    var phi_1244_: f32;
    var phi_1245_: f32;
    var phi_1246_: f32;
    var phi_1247_: f32;
    var phi_1266_: f32;
    var phi_1267_: f32;
    var phi_1268_: f32;
    var phi_1269_: f32;
    var phi_1353_: f32;
    var phi_1354_: f32;
    var phi_1355_: f32;
    var phi_1356_: f32;
    var phi_1357_: f32;
    var phi_1358_: f32;
    var phi_1359_: f32;
    var phi_1360_: f32;
    var phi_1361_: f32;
    var phi_1406_: f32;
    var phi_1407_: f32;
    var phi_1408_: f32;
    var phi_1409_: f32;
    var phi_1410_: f32;
    var phi_1411_: f32;
    var phi_1412_: f32;
    var phi_1413_: f32;
    var phi_1414_: f32;
    var phi_1459_: f32;
    var phi_1460_: f32;
    var phi_1461_: f32;
    var phi_1462_: f32;
    var phi_1463_: f32;
    var phi_1464_: f32;
    var phi_1465_: f32;
    var phi_1466_: f32;
    var phi_1467_: f32;
    var phi_1512_: f32;
    var phi_1513_: f32;
    var phi_1514_: f32;
    var phi_1515_: f32;
    var phi_1516_: f32;
    var phi_1517_: f32;
    var phi_1518_: f32;
    var phi_1519_: f32;
    var phi_1520_: f32;
    var phi_1567_: f32;
    var phi_1568_: f32;
    var phi_1569_: f32;
    var phi_1570_: f32;
    var phi_1571_: f32;
    var phi_1572_: f32;
    var phi_1573_: f32;
    var phi_1574_: f32;
    var phi_1606_: f32;
    var phi_1608_: f32;
    var phi_1672_: f32;
    var phi_1673_: f32;
    var phi_1674_: f32;
    var phi_1675_: f32;
    var phi_1676_: f32;
    var phi_1677_: f32;
    var phi_1898_: f32;
    var phi_1899_: f32;
    var phi_1900_: f32;
    var phi_1912_: f32;
    var phi_1913_: f32;
    var phi_1914_: f32;
    var phi_1915_: f32;
    var phi_1916_: f32;
    var phi_1917_: f32;
    var phi_1918_: f32;
    var phi_1919_: f32;
    var phi_1920_: f32;
    var phi_1921_: f32;
    var phi_1922_: f32;
    var phi_1923_: f32;
    var phi_1924_: f32;

    let _e123: vec4<f32> = cF.m[19u];
    let _e125: f32 = (_e123.y * _e123.y);
    let _e128: f32 = mul_legacy_f32_(_e123.w, _e123.x);
    let _e130: f32 = mad_legacy_f32_(_e123.y, _e123.z, _e128);
    let _e133: vec4<f32> = cF.m[62u];
    let _e138: f32 = -((_e123.x * _e123.x));
    let _e142: f32 = mul_legacy_f32_(_e123.w, _e123.y);
    let _e144: f32 = mad_legacy_f32_(_e123.x, _e123.z, -(_e142));
    let _e147: vec4<f32> = cF.m[14u];
    let _e156: f32 = v1_tangent_1[0u];
    let _e158: f32 = v1_tangent_1[1u];
    let _e160: f32 = v1_tangent_1[2u];
    let _e161: vec3<f32> = vec3<f32>(_e156, _e158, _e160);
    let _e162: f32 = dp3_f32_legacy(vec3<f32>((_e144 * 2f), (_e130 * 2f), (fma(_e138, 2f, -((_e125 * 2f))) + 1f)), _e161);
    let _e165: f32 = -(((_e123.z * _e123.z) * 2f));
    let _e168: f32 = mad_legacy_f32_(_e123.x, _e123.z, _e142);
    let _e170: f32 = mul_legacy_f32_(_e123.w, _e123.z);
    let _e172: f32 = mad_legacy_f32_(_e123.x, _e123.y, -(_e170));
    let _e176: f32 = dp3_f32_legacy(_e161, vec3<f32>((fma(-(_e125), 2f, _e165) + 1f), (_e172 * 2f), (_e168 * 2f)));
    let _e178: f32 = mad_legacy_f32_(_e123.y, _e123.z, -(_e128));
    let _e179: f32 = mad_legacy_f32_(_e123.x, _e123.y, _e170);
    let _e184: f32 = dp3_f32_legacy(_e161, vec3<f32>((_e179 * 2f), (fma(_e138, 2f, _e165) + 1f), (_e178 * 2f)));
    let _e193: vec4<f32> = cF.m[20u];
    let _e196: f32 = v1_tangent_1[3u];
    let _e198: f32 = mul_legacy_f32_(_e196, _e147.y);
    let _e199: f32 = mad_legacy_f32_(fract((_e176 * 700.634f)), _e193.z, 1f);
    let _e200: f32 = (_e147.x - _e198);
    let _e201: f32 = mul_legacy_f32_(_e200, _e199);
    let _e206: f32 = v3_color_1[0u];
    let _e208: f32 = v3_color_1[1u];
    let _e210: f32 = v3_color_1[2u];
    let _e212: f32 = v3_color_1[3u];
    let _e214: f32 = v4_specular0_1[0u];
    let _e216: f32 = v4_specular0_1[1u];
    let _e218: f32 = v4_specular0_1[2u];
    let _e220: f32 = v4_specular0_1[3u];
    let _e223: vec4<f32> = cF.m[11u];
    let _e228: f32 = v2_texcoord0_1[0u];
    let _e230: f32 = v2_texcoord0_1[1u];
    let _e238: f32 = select(0f, 1f, ((_e198 < (_e147.x + _e133.w)) && (_e198 >= (_e147.x + _e133.z))));
    let _e241: vec4<f32> = cF.m[63u];
    let _e259: f32 = (select(_e238, (_e238 + 1f), ((_e198 >= (_e147.x + _e133.x)) && (_e198 < (_e147.x + _e133.y)))) + select(0f, 1f, ((_e198 < (_e147.x + _e241.y)) && (_e198 >= (_e147.x + _e241.x)))));
    let _e260: f32 = mad_legacy_f32_(_e176, fract((_e184 * 800.634f)), fract((_e162 * 900.634f)));
    let _e264: f32 = mad_legacy_f32_((fract(_e260) - 1f), _e193.w, 1f);
    let _e267: bool = (_e264 < 0.5f);
    let _e271: bool = ((!(_e267) && (select(_e259, (_e259 + 1f), ((_e198 < (_e147.x + _e241.w)) && (_e198 >= (_e147.x + _e241.z)))) <= 0f)) || _e267);
    let _e272: f32 = select(f32(), -100000000f, _e271);
    let _e276: vec4<f32> = cF.m[12u];
    phi_1912_ = _e230;
    phi_1913_ = _e228;
    phi_1914_ = _e220;
    phi_1915_ = _e218;
    phi_1916_ = _e216;
    phi_1917_ = _e214;
    phi_1918_ = _e212;
    phi_1919_ = _e210;
    phi_1920_ = _e208;
    phi_1921_ = _e206;
    phi_1922_ = _e272;
    phi_1923_ = _e272;
    phi_1924_ = _e272;
    if !(_e271) {
        let _e279: vec4<f32> = cF.m[17u];
        let _e281: f32 = mul_legacy_f32_(_e176, _e279.w);
        let _e282: f32 = mul_legacy_f32_(_e184, _e279.w);
        let _e283: f32 = mul_legacy_f32_(_e162, _e279.w);
        let _e293: f32 = mul_legacy_f32_(_e281, (1f / _e279.x));
        let _e294: f32 = mul_legacy_f32_(_e282, (1f / _e279.y));
        let _e295: f32 = mul_legacy_f32_(_e283, (1f / _e279.z));
        let _e308: bool = (abs(select(0f, 1f, (_e281 < 0f))) > 0f);
        let _e310: bool = (abs(select(0f, 1f, (_e282 < 0f))) > 0f);
        let _e312: bool = (abs(select(0f, 1f, (_e283 < 0f))) > 0f);
        let _e317: f32 = mul_legacy_f32_(fract(abs(_e293)), abs(_e279.x));
        let _e318: f32 = mul_legacy_f32_(fract(abs(_e294)), abs(_e279.y));
        let _e319: f32 = mul_legacy_f32_(fract(abs(_e295)), abs(_e279.z));
        let _e323: f32 = mul_legacy_f32_(_e317, (1f - select(0f, 1f, _e308)));
        let _e324: f32 = mul_legacy_f32_(_e318, (1f - select(0f, 1f, _e310)));
        let _e325: f32 = mul_legacy_f32_(_e319, (1f - select(0f, 1f, _e312)));
        let _e327: f32 = select(_e323, (_e323 - _e317), _e308);
        let _e329: f32 = select(_e324, (_e324 - _e318), _e310);
        let _e331: f32 = select(_e325, (_e325 - _e319), _e312);
        let _e335: f32 = v4_specular0_1[3u];
        let _e337: f32 = v4_specular0_1[1u];
        let _e339: f32 = v4_specular0_1[0u];
        let _e341: f32 = v4_specular0_1[2u];
        let _e343: f32 = v0_position0_1[2u];
        let _e345: f32 = v0_position0_1[1u];
        let _e347: f32 = v0_position0_1[0u];
        phi_504_ = _e327;
        phi_505_ = _e329;
        phi_506_ = _e331;
        if (_e147.z >= 0.5f) {
            phi_501_ = _e327;
            phi_502_ = _e329;
            phi_503_ = _e331;
            if (_e147.z >= 1.5f) {
                let _e349: f32 = (_e335 * 2f);
                let _e350: f32 = (_e337 * 2f);
                let _e351: f32 = mul_legacy_f32_(_e339, _e349);
                let _e352: f32 = mul_legacy_f32_(_e337, _e349);
                let _e353: f32 = (_e339 * 2f);
                let _e354: f32 = mul_legacy_f32_(_e337, _e350);
                let _e355: f32 = -(_e353);
                let _e357: f32 = mad_legacy_f32_(_e339, _e355, -(_e354));
                let _e359: f32 = mul_legacy_f32_(_e341, (_e341 * 2f));
                let _e361: f32 = mad_legacy_f32_(_e341, _e353, -(_e352));
                let _e362: f32 = mad_legacy_f32_(_e341, _e350, _e351);
                let _e365: vec4<f32> = vec4<f32>(_e327, _e329, _e331, 1f);
                let _e366: f32 = dp4_f32_legacy(vec4<f32>(_e361, _e362, (_e357 + 1f), _e343), _e365);
                let _e367: f32 = mul_legacy_f32_(_e341, _e349);
                let _e368: f32 = -(_e359);
                let _e369: f32 = mad_legacy_f32_(_e339, _e355, _e368);
                let _e371: f32 = mad_legacy_f32_(_e341, _e350, -(_e351));
                let _e373: f32 = mad_legacy_f32_(_e337, -(_e350), _e368);
                let _e374: f32 = mad_legacy_f32_(_e337, _e353, _e367);
                let _e377: f32 = dp4_f32_legacy(_e365, vec4<f32>(_e374, (_e369 + 1f), _e371, _e345));
                let _e380: f32 = mad_legacy_f32_(_e337, _e353, -(_e367));
                let _e381: f32 = mad_legacy_f32_(_e341, _e353, _e352);
                let _e383: f32 = dp4_f32_legacy(_e365, vec4<f32>((_e373 + 1f), _e380, _e381, _e347));
                phi_501_ = _e383;
                phi_502_ = _e377;
                phi_503_ = _e366;
            }
            let _e385: f32 = phi_501_;
            let _e387: f32 = phi_502_;
            let _e389: f32 = phi_503_;
            phi_504_ = _e385;
            phi_505_ = _e387;
            phi_506_ = _e389;
        }
        let _e391: f32 = phi_504_;
        let _e393: f32 = phi_505_;
        let _e395: f32 = phi_506_;
        let _e396: bool = (_e201 > 0f);
        phi_534_ = _e201;
        phi_535_ = 1f;
        phi_536_ = 0f;
        if _e396 {
            let _e399: vec4<f32> = cF.m[59u];
            let _e402: f32 = mad_legacy_f32_(_e200, _e199, -(_e399.w));
            if (_e201 < _e399.w) {
                let _e405: f32 = (_e399.x - 1f);
                let _e407: f32 = mul_legacy_f32_(_e201, (1f / _e399.w));
                let _e408: f32 = clamp(_e407, 0f, 1f);
                let _e409: f32 = mul_legacy_f32_(_e408, _e405);
                let _e411: f32 = mad_legacy_f32_(_e408, _e405, 1f);
                let _e412: f32 = mul_legacy_f32_(_e201, fma(_e409, 0.5f, 1f));
                phi_532_ = _e412;
                phi_533_ = _e411;
            } else {
                let _e415: f32 = mul_legacy_f32_(fma((_e399.x - 1f), 0.5f, 1f), _e399.w);
                phi_532_ = _e415;
                phi_533_ = _e399.x;
            }
            let _e417: f32 = phi_532_;
            let _e419: f32 = phi_533_;
            phi_534_ = _e402;
            phi_535_ = _e419;
            phi_536_ = _e417;
        }
        let _e421: f32 = phi_534_;
        let _e423: f32 = phi_535_;
        let _e425: f32 = phi_536_;
        phi_563_ = _e423;
        phi_564_ = _e421;
        phi_565_ = _e425;
        if (_e421 > 0f) {
            let _e429: vec4<f32> = cF.m[60u];
            if (_e421 < _e429.w) {
                let _e435: f32 = (_e429.x - _e423);
                let _e436: f32 = mul_legacy_f32_(_e421, (1f / _e429.w));
                let _e437: f32 = clamp(_e436, 0f, 1f);
                let _e438: f32 = mul_legacy_f32_(_e437, _e435);
                let _e440: f32 = mad_legacy_f32_(_e437, _e435, _e423);
                let _e441: f32 = mad_legacy_f32_(_e421, fma(_e438, 0.5f, _e423), _e425);
                phi_561_ = _e441;
                phi_562_ = _e440;
            } else {
                let _e444: f32 = mad_legacy_f32_(fma((_e429.x - _e423), 0.5f, _e423), _e429.w, _e425);
                phi_561_ = _e444;
                phi_562_ = _e429.x;
            }
            let _e446: f32 = phi_561_;
            let _e448: f32 = phi_562_;
            phi_563_ = _e448;
            phi_564_ = (_e421 - _e429.w);
            phi_565_ = _e446;
        }
        let _e450: f32 = phi_563_;
        let _e452: f32 = phi_564_;
        let _e454: f32 = phi_565_;
        phi_595_ = _e454;
        if (_e452 > 0f) {
            let _e458: vec4<f32> = cF.m[61u];
            if (_e452 < _e458.w) {
                let _e465: f32 = mul_legacy_f32_(_e452, (1f / _e458.w));
                let _e467: f32 = mul_legacy_f32_(clamp(_e465, 0f, 1f), (_e458.x - _e450));
                let _e469: f32 = mad_legacy_f32_(_e452, fma(_e467, 0.5f, _e450), _e454);
                phi_594_ = _e469;
            } else {
                let _e472: f32 = mul_legacy_f32_(_e452, (1f / _e458.w));
                let _e474: f32 = mul_legacy_f32_(clamp(_e472, 0f, 1f), (_e458.x - _e450));
                let _e476: f32 = mad_legacy_f32_(fma(_e474, 0.5f, _e450), _e458.w, _e454);
                let _e477: f32 = mad_legacy_f32_((_e452 - _e458.w), _e458.x, _e476);
                phi_594_ = _e477;
            }
            let _e479: f32 = phi_594_;
            phi_595_ = _e479;
        }
        let _e481: f32 = phi_595_;
        let _e485: f32 = fract((_e176 * 700.234f));
        let _e486: f32 = fract((_e184 * 800.234f));
        let _e487: f32 = fract((_e162 * 900.234f));
        let _e490: vec4<f32> = cF.m[36u];
        let _e496: vec4<f32> = cF.m[37u];
        let _e500: f32 = mad_legacy_f32_(_e485, _e490.x, _e496.x);
        let _e501: f32 = mad_legacy_f32_(_e486, _e490.y, _e496.y);
        let _e502: f32 = mad_legacy_f32_(_e487, _e490.z, _e496.z);
        phi_637_ = _e395;
        phi_638_ = _e393;
        phi_639_ = _e391;
        if (_e490.w < _e481) {
            if (_e481 >= _e496.w) {
                let _e507: f32 = (_e496.w - _e490.w);
                let _e508: f32 = mad_legacy_f32_(_e500, _e507, _e391);
                let _e509: f32 = mad_legacy_f32_(_e501, _e507, _e393);
                let _e510: f32 = mad_legacy_f32_(_e502, _e507, _e395);
                phi_634_ = _e508;
                phi_635_ = _e509;
                phi_636_ = _e510;
            } else {
                let _e511: f32 = (_e481 - _e490.w);
                let _e512: f32 = mad_legacy_f32_(_e500, _e511, _e391);
                let _e513: f32 = mad_legacy_f32_(_e501, _e511, _e393);
                let _e514: f32 = mad_legacy_f32_(_e502, _e511, _e395);
                phi_634_ = _e512;
                phi_635_ = _e513;
                phi_636_ = _e514;
            }
            let _e516: f32 = phi_634_;
            let _e518: f32 = phi_635_;
            let _e520: f32 = phi_636_;
            phi_637_ = _e520;
            phi_638_ = _e518;
            phi_639_ = _e516;
        }
        let _e522: f32 = phi_637_;
        let _e524: f32 = phi_638_;
        let _e526: f32 = phi_639_;
        let _e529: vec4<f32> = cF.m[38u];
        let _e533: f32 = mad_legacy_f32_(_e485, _e529.x, _e529.x);
        let _e534: f32 = mad_legacy_f32_(_e486, _e529.y, _e529.y);
        let _e535: f32 = mad_legacy_f32_(_e487, _e529.z, _e529.z);
        phi_664_ = _e522;
        phi_665_ = _e524;
        phi_666_ = _e526;
        if (_e529.w < _e481) {
            phi_661_ = _e526;
            phi_662_ = _e524;
            phi_663_ = _e522;
            if !((_e481 >= _e529.w)) {
                let _e540: f32 = (_e481 - _e529.w);
                let _e541: f32 = mad_legacy_f32_(_e533, _e540, _e526);
                let _e542: f32 = mad_legacy_f32_(_e534, _e540, _e524);
                let _e543: f32 = mad_legacy_f32_(_e535, _e540, _e522);
                phi_661_ = _e541;
                phi_662_ = _e542;
                phi_663_ = _e543;
            }
            let _e545: f32 = phi_661_;
            let _e547: f32 = phi_662_;
            let _e549: f32 = phi_663_;
            phi_664_ = _e549;
            phi_665_ = _e547;
            phi_666_ = _e545;
        }
        let _e551: f32 = phi_664_;
        let _e553: f32 = phi_665_;
        let _e555: f32 = phi_666_;
        let _e558: vec4<f32> = cF.m[40u];
        let _e564: vec4<f32> = cF.m[39u];
        let _e568: f32 = mad_legacy_f32_(_e485, _e558.x, _e564.x);
        let _e569: f32 = mad_legacy_f32_(_e486, _e558.y, _e564.y);
        let _e570: f32 = mad_legacy_f32_(_e487, _e558.z, _e564.z);
        phi_702_ = _e551;
        phi_703_ = _e553;
        phi_704_ = _e555;
        if (_e558.w < _e481) {
            if (_e481 >= _e564.w) {
                let _e575: f32 = (_e564.w - _e558.w);
                let _e576: f32 = mad_legacy_f32_(_e568, _e575, _e555);
                let _e577: f32 = mad_legacy_f32_(_e569, _e575, _e553);
                let _e578: f32 = mad_legacy_f32_(_e570, _e575, _e551);
                phi_699_ = _e576;
                phi_700_ = _e577;
                phi_701_ = _e578;
            } else {
                let _e579: f32 = (_e481 - _e558.w);
                let _e580: f32 = mad_legacy_f32_(_e568, _e579, _e555);
                let _e581: f32 = mad_legacy_f32_(_e569, _e579, _e553);
                let _e582: f32 = mad_legacy_f32_(_e570, _e579, _e551);
                phi_699_ = _e580;
                phi_700_ = _e581;
                phi_701_ = _e582;
            }
            let _e584: f32 = phi_699_;
            let _e586: f32 = phi_700_;
            let _e588: f32 = phi_701_;
            phi_702_ = _e588;
            phi_703_ = _e586;
            phi_704_ = _e584;
        }
        let _e590: f32 = phi_702_;
        let _e592: f32 = phi_703_;
        let _e594: f32 = phi_704_;
        let _e598: f32 = fract((_e176 * 700.345f));
        let _e599: f32 = fract((_e184 * 800.345f));
        let _e600: f32 = fract((_e162 * 900.345f));
        let _e603: vec4<f32> = cF.m[41u];
        let _e609: vec4<f32> = cF.m[42u];
        let _e613: f32 = mad_legacy_f32_(_e598, _e603.x, _e609.x);
        let _e614: f32 = mad_legacy_f32_(_e599, _e603.y, _e609.y);
        let _e615: f32 = mad_legacy_f32_(_e600, _e603.z, _e609.z);
        phi_754_ = _e590;
        phi_755_ = _e592;
        phi_756_ = _e594;
        if (_e603.w < _e481) {
            if (_e481 >= _e609.w) {
                let _e620: f32 = (_e609.w - _e603.w);
                let _e621: f32 = (_e620 * _e620);
                let _e622: f32 = mul_legacy_f32_(_e613, _e621);
                let _e623: f32 = mul_legacy_f32_(_e614, _e621);
                let _e624: f32 = mul_legacy_f32_(_e615, _e621);
                phi_751_ = fma(_e622, 0.5f, _e594);
                phi_752_ = fma(_e623, 0.5f, _e592);
                phi_753_ = fma(_e624, 0.5f, _e590);
            } else {
                let _e628: f32 = (_e481 - _e603.w);
                let _e629: f32 = (_e628 * _e628);
                let _e630: f32 = mul_legacy_f32_(_e613, _e629);
                let _e631: f32 = mul_legacy_f32_(_e614, _e629);
                let _e632: f32 = mul_legacy_f32_(_e615, _e629);
                phi_751_ = fma(_e630, 0.5f, _e594);
                phi_752_ = fma(_e631, 0.5f, _e592);
                phi_753_ = fma(_e632, 0.5f, _e590);
            }
            let _e637: f32 = phi_751_;
            let _e639: f32 = phi_752_;
            let _e641: f32 = phi_753_;
            phi_754_ = _e641;
            phi_755_ = _e639;
            phi_756_ = _e637;
        }
        let _e643: f32 = phi_754_;
        let _e645: f32 = phi_755_;
        let _e647: f32 = phi_756_;
        let _e650: vec4<f32> = cF.m[43u];
        let _e656: vec4<f32> = cF.m[44u];
        let _e660: f32 = mad_legacy_f32_(_e598, _e650.x, _e656.x);
        let _e661: f32 = mad_legacy_f32_(_e599, _e650.y, _e656.y);
        let _e662: f32 = mad_legacy_f32_(_e600, _e650.z, _e656.z);
        phi_800_ = _e643;
        phi_801_ = _e645;
        phi_802_ = _e647;
        if (_e650.w < _e481) {
            if (_e481 >= _e656.w) {
                let _e667: f32 = (_e656.w - _e650.w);
                let _e668: f32 = (_e667 * _e667);
                let _e669: f32 = mul_legacy_f32_(_e660, _e668);
                let _e670: f32 = mul_legacy_f32_(_e661, _e668);
                let _e671: f32 = mul_legacy_f32_(_e662, _e668);
                phi_797_ = fma(_e669, 0.5f, _e647);
                phi_798_ = fma(_e670, 0.5f, _e645);
                phi_799_ = fma(_e671, 0.5f, _e643);
            } else {
                let _e675: f32 = (_e481 - _e650.w);
                let _e676: f32 = (_e675 * _e675);
                let _e677: f32 = mul_legacy_f32_(_e660, _e676);
                let _e678: f32 = mul_legacy_f32_(_e661, _e676);
                let _e679: f32 = mul_legacy_f32_(_e662, _e676);
                phi_797_ = fma(_e677, 0.5f, _e647);
                phi_798_ = fma(_e678, 0.5f, _e645);
                phi_799_ = fma(_e679, 0.5f, _e643);
            }
            let _e684: f32 = phi_797_;
            let _e686: f32 = phi_798_;
            let _e688: f32 = phi_799_;
            phi_800_ = _e688;
            phi_801_ = _e686;
            phi_802_ = _e684;
        }
        let _e690: f32 = phi_800_;
        let _e692: f32 = phi_801_;
        let _e694: f32 = phi_802_;
        let _e697: vec4<f32> = cF.m[45u];
        let _e703: vec4<f32> = cF.m[46u];
        let _e707: f32 = mad_legacy_f32_(_e598, _e697.x, _e703.x);
        let _e708: f32 = mad_legacy_f32_(_e599, _e697.y, _e703.y);
        let _e709: f32 = mad_legacy_f32_(_e600, _e697.z, _e703.z);
        phi_846_ = _e690;
        phi_847_ = _e692;
        phi_848_ = _e694;
        if (_e697.w < _e481) {
            if (_e481 >= _e703.w) {
                let _e714: f32 = (_e703.w - _e697.w);
                let _e715: f32 = (_e714 * _e714);
                let _e716: f32 = mul_legacy_f32_(_e707, _e715);
                let _e717: f32 = mul_legacy_f32_(_e708, _e715);
                let _e718: f32 = mul_legacy_f32_(_e709, _e715);
                phi_843_ = fma(_e716, 0.5f, _e694);
                phi_844_ = fma(_e717, 0.5f, _e692);
                phi_845_ = fma(_e718, 0.5f, _e690);
            } else {
                let _e722: f32 = (_e481 - _e697.w);
                let _e723: f32 = (_e722 * _e722);
                let _e724: f32 = mul_legacy_f32_(_e707, _e723);
                let _e725: f32 = mul_legacy_f32_(_e708, _e723);
                let _e726: f32 = mul_legacy_f32_(_e709, _e723);
                phi_843_ = fma(_e724, 0.5f, _e694);
                phi_844_ = fma(_e725, 0.5f, _e692);
                phi_845_ = fma(_e726, 0.5f, _e690);
            }
            let _e731: f32 = phi_843_;
            let _e733: f32 = phi_844_;
            let _e735: f32 = phi_845_;
            phi_846_ = _e735;
            phi_847_ = _e733;
            phi_848_ = _e731;
        }
        let _e737: f32 = phi_846_;
        let _e739: f32 = phi_847_;
        let _e741: f32 = phi_848_;
        let _e749: f32 = fract((_e176 * 1000.456f));
        let _e750: f32 = (fract((_e176 * 700.456f)) - 0.5f);
        let _e751: f32 = (fract((_e184 * 800.456f)) - 0.5f);
        let _e752: f32 = (fract((_e162 * 900.456f)) - 0.5f);
        let _e753: vec3<f32> = vec3<f32>(_e750, _e751, _e752);
        let _e754: f32 = dp3_f32_(_e753, _e753);
        let _e757: vec4<f32> = cF.m[47u];
        let _e759: f32 = inverseSqrt(_e754);
        let _e760: f32 = mul_legacy_f32_(_e759, _e750);
        let _e761: f32 = mul_legacy_f32_(_e759, _e751);
        let _e762: f32 = mul_legacy_f32_(_e759, _e752);
        let _e765: vec4<f32> = cF.m[48u];
        let _e767: f32 = mad_legacy_f32_(_e749, _e757.x, _e765.x);
        let _e768: f32 = mul_legacy_f32_(_e760, _e767);
        let _e769: f32 = mul_legacy_f32_(_e761, _e767);
        let _e770: f32 = mul_legacy_f32_(_e762, _e767);
        phi_898_ = _e737;
        phi_899_ = _e739;
        phi_900_ = _e741;
        if (_e757.w < _e481) {
            if (_e481 >= _e765.w) {
                let _e775: f32 = (_e765.w - _e757.w);
                let _e776: f32 = mad_legacy_f32_(_e768, _e775, _e741);
                let _e777: f32 = mad_legacy_f32_(_e769, _e775, _e739);
                let _e778: f32 = mad_legacy_f32_(_e770, _e775, _e737);
                phi_895_ = _e776;
                phi_896_ = _e777;
                phi_897_ = _e778;
            } else {
                let _e779: f32 = (_e481 - _e757.w);
                let _e780: f32 = mad_legacy_f32_(_e768, _e779, _e741);
                let _e781: f32 = mad_legacy_f32_(_e769, _e779, _e739);
                let _e782: f32 = mad_legacy_f32_(_e770, _e779, _e737);
                phi_895_ = _e780;
                phi_896_ = _e781;
                phi_897_ = _e782;
            }
            let _e784: f32 = phi_895_;
            let _e786: f32 = phi_896_;
            let _e788: f32 = phi_897_;
            phi_898_ = _e788;
            phi_899_ = _e786;
            phi_900_ = _e784;
        }
        let _e790: f32 = phi_898_;
        let _e792: f32 = phi_899_;
        let _e794: f32 = phi_900_;
        let _e797: vec4<f32> = cF.m[49u];
        let _e801: vec4<f32> = cF.m[50u];
        let _e803: f32 = mad_legacy_f32_(_e749, _e797.x, _e801.x);
        let _e804: f32 = mul_legacy_f32_(_e760, _e803);
        let _e805: f32 = mul_legacy_f32_(_e761, _e803);
        let _e806: f32 = mul_legacy_f32_(_e762, _e803);
        phi_933_ = _e790;
        phi_934_ = _e792;
        phi_935_ = _e794;
        if (_e797.w < _e481) {
            if (_e481 >= _e801.w) {
                let _e811: f32 = (_e801.w - _e797.w);
                let _e812: f32 = mad_legacy_f32_(_e804, _e811, _e794);
                let _e813: f32 = mad_legacy_f32_(_e805, _e811, _e792);
                let _e814: f32 = mad_legacy_f32_(_e806, _e811, _e790);
                phi_930_ = _e812;
                phi_931_ = _e813;
                phi_932_ = _e814;
            } else {
                let _e815: f32 = (_e481 - _e797.w);
                let _e816: f32 = mad_legacy_f32_(_e804, _e815, _e794);
                let _e817: f32 = mad_legacy_f32_(_e805, _e815, _e792);
                let _e818: f32 = mad_legacy_f32_(_e806, _e815, _e790);
                phi_930_ = _e816;
                phi_931_ = _e817;
                phi_932_ = _e818;
            }
            let _e820: f32 = phi_930_;
            let _e822: f32 = phi_931_;
            let _e824: f32 = phi_932_;
            phi_933_ = _e824;
            phi_934_ = _e822;
            phi_935_ = _e820;
        }
        let _e826: f32 = phi_933_;
        let _e828: f32 = phi_934_;
        let _e830: f32 = phi_935_;
        let _e833: vec4<f32> = cF.m[51u];
        let _e837: vec4<f32> = cF.m[52u];
        let _e839: f32 = mad_legacy_f32_(_e749, _e833.x, _e837.x);
        let _e840: f32 = mul_legacy_f32_(_e760, _e839);
        let _e841: f32 = mul_legacy_f32_(_e761, _e839);
        let _e842: f32 = mul_legacy_f32_(_e762, _e839);
        phi_968_ = _e826;
        phi_969_ = _e828;
        phi_970_ = _e830;
        if (_e833.w < _e481) {
            if (_e481 >= _e837.w) {
                let _e847: f32 = (_e837.w - _e833.w);
                let _e848: f32 = mad_legacy_f32_(_e840, _e847, _e830);
                let _e849: f32 = mad_legacy_f32_(_e841, _e847, _e828);
                let _e850: f32 = mad_legacy_f32_(_e842, _e847, _e826);
                phi_965_ = _e848;
                phi_966_ = _e849;
                phi_967_ = _e850;
            } else {
                let _e851: f32 = (_e481 - _e833.w);
                let _e852: f32 = mad_legacy_f32_(_e840, _e851, _e830);
                let _e853: f32 = mad_legacy_f32_(_e841, _e851, _e828);
                let _e854: f32 = mad_legacy_f32_(_e842, _e851, _e826);
                phi_965_ = _e852;
                phi_966_ = _e853;
                phi_967_ = _e854;
            }
            let _e856: f32 = phi_965_;
            let _e858: f32 = phi_966_;
            let _e860: f32 = phi_967_;
            phi_968_ = _e860;
            phi_969_ = _e858;
            phi_970_ = _e856;
        }
        let _e862: f32 = phi_968_;
        let _e864: f32 = phi_969_;
        let _e866: f32 = phi_970_;
        phi_1022_ = _e481;
        phi_1023_ = 0f;
        phi_1024_ = 0f;
        phi_1025_ = 0f;
        phi_1026_ = _e862;
        phi_1027_ = _e864;
        phi_1028_ = _e866;
        if (_e481 > 0f) {
            let _e876: vec4<f32> = cF.m[53u];
            let _e886: vec4<f32> = cF.m[54u];
            let _e890: f32 = mad_legacy_f32_(fract((_e176 * 700.567f)), _e876.x, _e886.x);
            let _e891: f32 = mad_legacy_f32_(fract((_e184 * 800.567f)), _e876.y, _e886.y);
            let _e892: f32 = mad_legacy_f32_(fract((_e162 * 900.567f)), _e876.z, _e886.z);
            if (clamp(select(0f, 1f, (_e481 < _e876.w)), 0f, 1f) > 0f) {
                let _e896: f32 = mul_legacy_f32_(_e481, (1f / _e876.w));
                let _e897: f32 = clamp(_e896, 0f, 1f);
                let _e898: f32 = mul_legacy_f32_(_e897, _e890);
                let _e899: f32 = mul_legacy_f32_(_e897, _e891);
                let _e900: f32 = mul_legacy_f32_(_e897, _e892);
                let _e901: f32 = mad_legacy_f32_(_e898, _e481, _e866);
                let _e902: f32 = mad_legacy_f32_(_e899, _e481, _e864);
                let _e903: f32 = mad_legacy_f32_(_e900, _e481, _e862);
                phi_1016_ = _e901;
                phi_1017_ = _e902;
                phi_1018_ = _e903;
                phi_1019_ = _e898;
                phi_1020_ = _e899;
                phi_1021_ = _e900;
            } else {
                let _e904: f32 = mad_legacy_f32_(_e890, _e876.w, _e866);
                let _e905: f32 = mad_legacy_f32_(_e891, _e876.w, _e864);
                let _e906: f32 = mad_legacy_f32_(_e892, _e876.w, _e862);
                phi_1016_ = _e904;
                phi_1017_ = _e905;
                phi_1018_ = _e906;
                phi_1019_ = _e890;
                phi_1020_ = _e891;
                phi_1021_ = _e892;
            }
            let _e908: f32 = phi_1016_;
            let _e910: f32 = phi_1017_;
            let _e912: f32 = phi_1018_;
            let _e914: f32 = phi_1019_;
            let _e916: f32 = phi_1020_;
            let _e918: f32 = phi_1021_;
            phi_1022_ = (_e481 - _e876.w);
            phi_1023_ = _e918;
            phi_1024_ = _e916;
            phi_1025_ = _e914;
            phi_1026_ = _e912;
            phi_1027_ = _e910;
            phi_1028_ = _e908;
        }
        let _e920: f32 = phi_1022_;
        let _e922: f32 = phi_1023_;
        let _e924: f32 = phi_1024_;
        let _e926: f32 = phi_1025_;
        let _e928: f32 = phi_1026_;
        let _e930: f32 = phi_1027_;
        let _e932: f32 = phi_1028_;
        phi_1083_ = _e922;
        phi_1084_ = _e924;
        phi_1085_ = _e926;
        phi_1086_ = _e920;
        phi_1087_ = _e928;
        phi_1088_ = _e930;
        phi_1089_ = _e932;
        if (_e920 > 0f) {
            let _e942: vec4<f32> = cF.m[55u];
            let _e948: vec4<f32> = cF.m[56u];
            let _e952: f32 = mad_legacy_f32_(fract((_e176 * 700.567f)), _e942.x, _e948.x);
            let _e953: f32 = mad_legacy_f32_(fract((_e184 * 800.567f)), _e942.y, _e948.y);
            let _e954: f32 = mad_legacy_f32_(fract((_e162 * 900.567f)), _e942.z, _e948.z);
            if (clamp(select(0f, 1f, (_e920 < _e942.w)), 0f, 1f) > 0f) {
                let _e965: f32 = mul_legacy_f32_(_e920, (1f / _e942.w));
                let _e966: f32 = clamp(_e965, 0f, 1f);
                let _e967: f32 = mad_legacy_f32_(_e966, (_e952 - _e926), _e926);
                let _e968: f32 = mad_legacy_f32_(_e966, (_e953 - _e924), _e924);
                let _e969: f32 = mad_legacy_f32_(_e966, (_e954 - _e922), _e922);
                let _e970: f32 = mad_legacy_f32_(_e967, _e920, _e932);
                let _e971: f32 = mad_legacy_f32_(_e968, _e920, _e930);
                let _e972: f32 = mad_legacy_f32_(_e969, _e920, _e928);
                phi_1077_ = _e970;
                phi_1078_ = _e971;
                phi_1079_ = _e972;
                phi_1080_ = _e967;
                phi_1081_ = _e968;
                phi_1082_ = _e969;
            } else {
                let _e973: f32 = mad_legacy_f32_(_e952, _e942.w, _e932);
                let _e974: f32 = mad_legacy_f32_(_e953, _e942.w, _e930);
                let _e975: f32 = mad_legacy_f32_(_e954, _e942.w, _e928);
                phi_1077_ = _e973;
                phi_1078_ = _e974;
                phi_1079_ = _e975;
                phi_1080_ = _e952;
                phi_1081_ = _e953;
                phi_1082_ = _e954;
            }
            let _e977: f32 = phi_1077_;
            let _e979: f32 = phi_1078_;
            let _e981: f32 = phi_1079_;
            let _e983: f32 = phi_1080_;
            let _e985: f32 = phi_1081_;
            let _e987: f32 = phi_1082_;
            phi_1083_ = _e987;
            phi_1084_ = _e985;
            phi_1085_ = _e983;
            phi_1086_ = (_e920 - _e942.w);
            phi_1087_ = _e981;
            phi_1088_ = _e979;
            phi_1089_ = _e977;
        }
        let _e989: f32 = phi_1083_;
        let _e991: f32 = phi_1084_;
        let _e993: f32 = phi_1085_;
        let _e995: f32 = phi_1086_;
        let _e997: f32 = phi_1087_;
        let _e999: f32 = phi_1088_;
        let _e1001: f32 = phi_1089_;
        phi_1127_ = _e1001;
        phi_1128_ = _e999;
        phi_1129_ = _e997;
        if (_e995 > 0f) {
            let _e1011: vec4<f32> = cF.m[57u];
            let _e1017: vec4<f32> = cF.m[58u];
            let _e1021: f32 = mad_legacy_f32_(fract((_e176 * 700.567f)), _e1011.x, _e1017.x);
            let _e1022: f32 = mad_legacy_f32_(fract((_e184 * 800.567f)), _e1011.y, _e1017.y);
            let _e1023: f32 = mad_legacy_f32_(fract((_e162 * 900.567f)), _e1011.z, _e1017.z);
            let _e1029: f32 = mul_legacy_f32_(_e995, (1f / _e1011.w));
            let _e1030: f32 = clamp(_e1029, 0f, 1f);
            let _e1031: f32 = mad_legacy_f32_(_e1030, (_e1021 - _e993), _e993);
            let _e1032: f32 = mad_legacy_f32_(_e1030, (_e1022 - _e991), _e991);
            let _e1033: f32 = mad_legacy_f32_(_e1030, (_e1023 - _e989), _e989);
            let _e1034: f32 = mad_legacy_f32_(_e1031, _e995, _e1001);
            let _e1035: f32 = mad_legacy_f32_(_e1032, _e995, _e999);
            let _e1036: f32 = mad_legacy_f32_(_e1033, _e995, _e997);
            phi_1127_ = _e1034;
            phi_1128_ = _e1035;
            phi_1129_ = _e1036;
        }
        let _e1038: f32 = phi_1127_;
        let _e1040: f32 = phi_1128_;
        let _e1042: f32 = phi_1129_;
        let _e1044: f32 = v3_color_1[3u];
        phi_1145_ = true;
        phi_1146_ = true;
        if !((_e1044 < 0.15f)) {
            phi_1142_ = true;
            phi_1143_ = true;
            if !((_e1044 < 0.45f)) {
                phi_1142_ = !((_e1044 < 0.75f));
                phi_1143_ = false;
            }
            let _e1052: bool = phi_1142_;
            let _e1054: bool = phi_1143_;
            phi_1145_ = _e1054;
            phi_1146_ = !(_e1052);
        }
        let _e1057: bool = phi_1145_;
        let _e1059: bool = phi_1146_;
        let _e1063: f32 = (select(1f, 0f, _e1057) - 0.5f);
        let _e1074: vec4<f32> = cF.m[15u];
        let _e1077: f32 = mad_legacy_f32_(fract((_e176 * 700.123f)), _e1074.x, _e1074.y);
        let _e1078: f32 = mul_legacy_f32_((select(1f, 0f, _e1059) - 0.5f), _e1077);
        if (_e1074.z < 0f) {
            let _e1081: f32 = mul_legacy_f32_(_e1063, _e1077);
            phi_1175_ = _e1081;
        } else {
            let _e1083: f32 = mad_legacy_f32_(fract((_e184 * 800.123f)), _e1074.z, _e1074.w);
            let _e1084: f32 = mul_legacy_f32_(_e1063, _e1083);
            phi_1175_ = _e1084;
        }
        let _e1086: f32 = phi_1175_;
        phi_1197_ = _e201;
        phi_1198_ = 1f;
        phi_1199_ = 1f;
        if _e396 {
            let _e1089: vec4<f32> = cF.m[32u];
            let _e1092: f32 = mad_legacy_f32_(_e200, _e199, -(_e1089.w));
            phi_1195_ = _e1089.x;
            phi_1196_ = _e1089.y;
            if (_e201 < _e1089.w) {
                let _e1099: f32 = mul_legacy_f32_(_e201, (1f / _e1089.w));
                let _e1100: f32 = mad_legacy_f32_(_e1099, (_e1089.x - 1f), 1f);
                let _e1101: f32 = mad_legacy_f32_(_e1099, (_e1089.y - 1f), 1f);
                phi_1195_ = _e1100;
                phi_1196_ = _e1101;
            }
            let _e1103: f32 = phi_1195_;
            let _e1105: f32 = phi_1196_;
            phi_1197_ = _e1092;
            phi_1198_ = _e1105;
            phi_1199_ = _e1103;
        }
        let _e1107: f32 = phi_1197_;
        let _e1109: f32 = phi_1198_;
        let _e1111: f32 = phi_1199_;
        phi_1221_ = _e1107;
        phi_1222_ = _e1109;
        phi_1223_ = _e1111;
        if (_e1107 > 0f) {
            let _e1115: vec4<f32> = cF.m[33u];
            phi_1219_ = _e1115.x;
            phi_1220_ = _e1115.y;
            if (_e1107 < _e1115.w) {
                let _e1124: f32 = mul_legacy_f32_(_e1107, (1f / _e1115.w));
                let _e1125: f32 = mad_legacy_f32_(_e1124, (_e1115.x - _e1111), _e1111);
                let _e1126: f32 = mad_legacy_f32_(_e1124, (_e1115.y - _e1109), _e1109);
                phi_1219_ = _e1125;
                phi_1220_ = _e1126;
            }
            let _e1128: f32 = phi_1219_;
            let _e1130: f32 = phi_1220_;
            phi_1221_ = (_e1107 - _e1115.w);
            phi_1222_ = _e1130;
            phi_1223_ = _e1128;
        }
        let _e1132: f32 = phi_1221_;
        let _e1134: f32 = phi_1222_;
        let _e1136: f32 = phi_1223_;
        phi_1245_ = _e1132;
        phi_1246_ = _e1134;
        phi_1247_ = _e1136;
        if (_e1132 > 0f) {
            let _e1140: vec4<f32> = cF.m[34u];
            phi_1243_ = _e1140.x;
            phi_1244_ = _e1140.y;
            if (_e1132 < _e1140.w) {
                let _e1149: f32 = mul_legacy_f32_(_e1132, (1f / _e1140.w));
                let _e1150: f32 = mad_legacy_f32_(_e1149, (_e1140.x - _e1136), _e1136);
                let _e1151: f32 = mad_legacy_f32_(_e1149, (_e1140.y - _e1134), _e1134);
                phi_1243_ = _e1150;
                phi_1244_ = _e1151;
            }
            let _e1153: f32 = phi_1243_;
            let _e1155: f32 = phi_1244_;
            phi_1245_ = (_e1132 - _e1140.w);
            phi_1246_ = _e1155;
            phi_1247_ = _e1153;
        }
        let _e1157: f32 = phi_1245_;
        let _e1159: f32 = phi_1246_;
        let _e1161: f32 = phi_1247_;
        phi_1268_ = _e1159;
        phi_1269_ = _e1161;
        if (_e1157 > 0f) {
            let _e1165: vec4<f32> = cF.m[35u];
            phi_1266_ = _e1165.x;
            phi_1267_ = _e1165.y;
            if (_e1157 < _e1165.w) {
                let _e1173: f32 = mul_legacy_f32_(_e1157, (1f / _e1165.w));
                let _e1174: f32 = mad_legacy_f32_(_e1173, (_e1165.x - _e1161), _e1161);
                let _e1175: f32 = mad_legacy_f32_(_e1173, (_e1165.y - _e1159), _e1159);
                phi_1266_ = _e1174;
                phi_1267_ = _e1175;
            }
            let _e1177: f32 = phi_1266_;
            let _e1179: f32 = phi_1267_;
            phi_1268_ = _e1179;
            phi_1269_ = _e1177;
        }
        let _e1181: f32 = phi_1268_;
        let _e1183: f32 = phi_1269_;
        let _e1186: vec4<f32> = cF.m[16u];
        let _e1189: f32 = mad_legacy_f32_(fract((_e176 * 1000.123f)), _e1186.z, _e1186.w);
        let _e1192: f32 = mad_legacy_f32_(fract((_e162 * 900.123f)), _e1186.x, _e1186.y);
        let _e1193: f32 = mul_legacy_f32_(_e1189, _e201);
        let _e1198: f32 = fma(fract(fma(_e1193, 0.1591549f, 0.5f)), 6.283185f, -3.141593f);
        let _e1199: f32 = fma(fract(fma(_e1192, 0.1591549f, 0.5f)), 6.283185f, -3.141593f);
        let _e1200: f32 = mul_legacy_f32_(_e1078, _e1183);
        let _e1201: f32 = mul_legacy_f32_(_e1086, _e1181);
        let _e1202: f32 = cos(_e1199);
        let _e1203: f32 = sin(_e1199);
        let _e1206: vec4<f32> = cF.m[65u];
        let _e1208: f32 = mul_legacy_f32_(_e1200, _e1206.w);
        let _e1209: f32 = mul_legacy_f32_(_e1201, _e1206.w);
        let _e1210: f32 = cos(_e1198);
        let _e1211: f32 = sin(_e1198);
        let _e1212: f32 = mul_legacy_f32_(_e1202, _e1209);
        let _e1213: f32 = mad_legacy_f32_(_e1203, _e1208, _e1212);
        let _e1214: f32 = mul_legacy_f32_(_e1211, _e1213);
        let _e1215: f32 = mul_legacy_f32_(_e1203, _e1209);
        let _e1217: f32 = mad_legacy_f32_(_e1202, _e1208, -(_e1215));
        let _e1218: f32 = mul_legacy_f32_(_e1210, _e1213);
        let _e1219: f32 = mad_legacy_f32_(_e1211, _e1217, _e1218);
        let _e1221: f32 = mad_legacy_f32_(_e1210, _e1217, -(_e1214));
        let _e1224: vec4<f32> = cF.m[21u];
        phi_1357_ = _e201;
        phi_1358_ = 0f;
        phi_1359_ = 1f;
        phi_1360_ = 1f;
        phi_1361_ = 1f;
        if _e396 {
            let _e1235: vec4<f32> = cF.m[22u];
            let _e1242: vec4<f32> = cF.m[23u];
            let _e1247: f32 = mad_legacy_f32_(fract((_e176 * 700.789f)), _e1235.x, _e1242.x);
            let _e1248: f32 = mad_legacy_f32_(fract((_e184 * 800.789f)), _e1235.y, _e1242.y);
            let _e1249: f32 = mad_legacy_f32_(fract((_e162 * 900.789f)), _e1235.z, _e1242.z);
            let _e1250: f32 = mad_legacy_f32_(fract((_e176 * 1000.789f)), _e1235.w, _e1242.w);
            phi_1353_ = _e1247;
            phi_1354_ = _e1248;
            phi_1355_ = _e1249;
            phi_1356_ = _e1250;
            if (_e201 < _e1224.x) {
                let _e1258: f32 = mul_legacy_f32_(_e201, (1f / _e1224.x));
                let _e1259: f32 = mad_legacy_f32_(_e1258, (_e1247 - 1f), 1f);
                let _e1260: f32 = mad_legacy_f32_(_e1258, (_e1248 - 1f), 1f);
                let _e1261: f32 = mad_legacy_f32_(_e1258, (_e1249 - 1f), 1f);
                let _e1262: f32 = mul_legacy_f32_(_e1258, _e1250);
                phi_1353_ = _e1259;
                phi_1354_ = _e1260;
                phi_1355_ = _e1261;
                phi_1356_ = _e1262;
            }
            let _e1264: f32 = phi_1353_;
            let _e1266: f32 = phi_1354_;
            let _e1268: f32 = phi_1355_;
            let _e1270: f32 = phi_1356_;
            phi_1357_ = (_e201 - _e1224.x);
            phi_1358_ = _e1270;
            phi_1359_ = _e1268;
            phi_1360_ = _e1266;
            phi_1361_ = _e1264;
        }
        let _e1272: f32 = phi_1357_;
        let _e1274: f32 = phi_1358_;
        let _e1276: f32 = phi_1359_;
        let _e1278: f32 = phi_1360_;
        let _e1280: f32 = phi_1361_;
        phi_1410_ = _e1272;
        phi_1411_ = _e1274;
        phi_1412_ = _e1276;
        phi_1413_ = _e1278;
        phi_1414_ = _e1280;
        if (_e1272 > 0f) {
            let _e1292: vec4<f32> = cF.m[24u];
            let _e1299: vec4<f32> = cF.m[25u];
            let _e1304: f32 = mad_legacy_f32_(fract((_e176 * 700.789f)), _e1292.x, _e1299.x);
            let _e1305: f32 = mad_legacy_f32_(fract((_e184 * 800.789f)), _e1292.y, _e1299.y);
            let _e1306: f32 = mad_legacy_f32_(fract((_e162 * 900.789f)), _e1292.z, _e1299.z);
            let _e1307: f32 = mad_legacy_f32_(fract((_e176 * 1000.789f)), _e1292.w, _e1299.w);
            phi_1406_ = _e1304;
            phi_1407_ = _e1305;
            phi_1408_ = _e1306;
            phi_1409_ = _e1307;
            if (_e1272 < _e1224.y) {
                let _e1316: f32 = mul_legacy_f32_(_e1272, (1f / _e1224.y));
                let _e1317: f32 = mad_legacy_f32_(_e1316, (_e1304 - _e1280), _e1280);
                let _e1318: f32 = mad_legacy_f32_(_e1316, (_e1305 - _e1278), _e1278);
                let _e1319: f32 = mad_legacy_f32_(_e1316, (_e1306 - _e1276), _e1276);
                let _e1320: f32 = mad_legacy_f32_(_e1316, (_e1307 - _e1274), _e1274);
                phi_1406_ = _e1317;
                phi_1407_ = _e1318;
                phi_1408_ = _e1319;
                phi_1409_ = _e1320;
            }
            let _e1322: f32 = phi_1406_;
            let _e1324: f32 = phi_1407_;
            let _e1326: f32 = phi_1408_;
            let _e1328: f32 = phi_1409_;
            phi_1410_ = (_e1272 - _e1224.y);
            phi_1411_ = _e1328;
            phi_1412_ = _e1326;
            phi_1413_ = _e1324;
            phi_1414_ = _e1322;
        }
        let _e1330: f32 = phi_1410_;
        let _e1332: f32 = phi_1411_;
        let _e1334: f32 = phi_1412_;
        let _e1336: f32 = phi_1413_;
        let _e1338: f32 = phi_1414_;
        phi_1463_ = _e1330;
        phi_1464_ = _e1332;
        phi_1465_ = _e1334;
        phi_1466_ = _e1336;
        phi_1467_ = _e1338;
        if (_e1330 > 0f) {
            let _e1350: vec4<f32> = cF.m[26u];
            let _e1357: vec4<f32> = cF.m[27u];
            let _e1362: f32 = mad_legacy_f32_(fract((_e176 * 700.789f)), _e1350.x, _e1357.x);
            let _e1363: f32 = mad_legacy_f32_(fract((_e184 * 800.789f)), _e1350.y, _e1357.y);
            let _e1364: f32 = mad_legacy_f32_(fract((_e162 * 900.789f)), _e1350.z, _e1357.z);
            let _e1365: f32 = mad_legacy_f32_(fract((_e176 * 1000.789f)), _e1350.w, _e1357.w);
            phi_1459_ = _e1362;
            phi_1460_ = _e1363;
            phi_1461_ = _e1364;
            phi_1462_ = _e1365;
            if (_e1330 < _e1224.z) {
                let _e1374: f32 = mul_legacy_f32_(_e1330, (1f / _e1224.z));
                let _e1375: f32 = mad_legacy_f32_(_e1374, (_e1362 - _e1338), _e1338);
                let _e1376: f32 = mad_legacy_f32_(_e1374, (_e1363 - _e1336), _e1336);
                let _e1377: f32 = mad_legacy_f32_(_e1374, (_e1364 - _e1334), _e1334);
                let _e1378: f32 = mad_legacy_f32_(_e1374, (_e1365 - _e1332), _e1332);
                phi_1459_ = _e1375;
                phi_1460_ = _e1376;
                phi_1461_ = _e1377;
                phi_1462_ = _e1378;
            }
            let _e1380: f32 = phi_1459_;
            let _e1382: f32 = phi_1460_;
            let _e1384: f32 = phi_1461_;
            let _e1386: f32 = phi_1462_;
            phi_1463_ = (_e1330 - _e1224.z);
            phi_1464_ = _e1386;
            phi_1465_ = _e1384;
            phi_1466_ = _e1382;
            phi_1467_ = _e1380;
        }
        let _e1388: f32 = phi_1463_;
        let _e1390: f32 = phi_1464_;
        let _e1392: f32 = phi_1465_;
        let _e1394: f32 = phi_1466_;
        let _e1396: f32 = phi_1467_;
        phi_1516_ = _e1388;
        phi_1517_ = _e1390;
        phi_1518_ = _e1392;
        phi_1519_ = _e1394;
        phi_1520_ = _e1396;
        if (_e1388 > 0f) {
            let _e1408: vec4<f32> = cF.m[28u];
            let _e1415: vec4<f32> = cF.m[29u];
            let _e1420: f32 = mad_legacy_f32_(fract((_e176 * 700.789f)), _e1408.x, _e1415.x);
            let _e1421: f32 = mad_legacy_f32_(fract((_e184 * 800.789f)), _e1408.y, _e1415.y);
            let _e1422: f32 = mad_legacy_f32_(fract((_e162 * 900.789f)), _e1408.z, _e1415.z);
            let _e1423: f32 = mad_legacy_f32_(fract((_e176 * 1000.789f)), _e1408.w, _e1415.w);
            phi_1512_ = _e1420;
            phi_1513_ = _e1421;
            phi_1514_ = _e1422;
            phi_1515_ = _e1423;
            if (_e1388 < _e1224.w) {
                let _e1432: f32 = mul_legacy_f32_(_e1388, (1f / _e1224.w));
                let _e1433: f32 = mad_legacy_f32_(_e1432, (_e1420 - _e1396), _e1396);
                let _e1434: f32 = mad_legacy_f32_(_e1432, (_e1421 - _e1394), _e1394);
                let _e1435: f32 = mad_legacy_f32_(_e1432, (_e1422 - _e1392), _e1392);
                let _e1436: f32 = mad_legacy_f32_(_e1432, (_e1423 - _e1390), _e1390);
                phi_1512_ = _e1433;
                phi_1513_ = _e1434;
                phi_1514_ = _e1435;
                phi_1515_ = _e1436;
            }
            let _e1438: f32 = phi_1512_;
            let _e1440: f32 = phi_1513_;
            let _e1442: f32 = phi_1514_;
            let _e1444: f32 = phi_1515_;
            phi_1516_ = (_e1388 - _e1224.w);
            phi_1517_ = _e1444;
            phi_1518_ = _e1442;
            phi_1519_ = _e1440;
            phi_1520_ = _e1438;
        }
        let _e1446: f32 = phi_1516_;
        let _e1448: f32 = phi_1517_;
        let _e1450: f32 = phi_1518_;
        let _e1452: f32 = phi_1519_;
        let _e1454: f32 = phi_1520_;
        phi_1571_ = _e1448;
        phi_1572_ = _e1454;
        phi_1573_ = _e1452;
        phi_1574_ = _e1450;
        if (_e1446 > 0f) {
            let _e1466: vec4<f32> = cF.m[30u];
            let _e1473: vec4<f32> = cF.m[31u];
            let _e1478: f32 = mad_legacy_f32_(fract((_e176 * 700.789f)), _e1466.x, _e1473.x);
            let _e1479: f32 = mad_legacy_f32_(fract((_e184 * 800.789f)), _e1466.y, _e1473.y);
            let _e1480: f32 = mad_legacy_f32_(fract((_e162 * 900.789f)), _e1466.z, _e1473.z);
            let _e1481: f32 = mad_legacy_f32_(fract((_e176 * 1000.789f)), _e1466.w, _e1473.w);
            let _e1484: vec4<f32> = cF.m[20u];
            phi_1567_ = _e1481;
            phi_1568_ = _e1478;
            phi_1569_ = _e1479;
            phi_1570_ = _e1480;
            if (_e1446 < _e1484.x) {
                let _e1492: f32 = mul_legacy_f32_(_e1446, (1f / _e1484.x));
                let _e1493: f32 = mad_legacy_f32_(_e1492, (_e1478 - _e1454), _e1454);
                let _e1494: f32 = mad_legacy_f32_(_e1492, (_e1479 - _e1452), _e1452);
                let _e1495: f32 = mad_legacy_f32_(_e1492, (_e1480 - _e1450), _e1450);
                let _e1496: f32 = mad_legacy_f32_(_e1492, (_e1481 - _e1448), _e1448);
                phi_1567_ = _e1496;
                phi_1568_ = _e1493;
                phi_1569_ = _e1494;
                phi_1570_ = _e1495;
            }
            let _e1498: f32 = phi_1567_;
            let _e1500: f32 = phi_1568_;
            let _e1502: f32 = phi_1569_;
            let _e1504: f32 = phi_1570_;
            phi_1571_ = _e1498;
            phi_1572_ = _e1500;
            phi_1573_ = _e1502;
            phi_1574_ = _e1504;
        }
        let _e1506: f32 = phi_1571_;
        let _e1508: f32 = phi_1572_;
        let _e1510: f32 = phi_1573_;
        let _e1512: f32 = phi_1574_;
        let _e1517: vec4<f32> = cF.m[64u];
        let _e1520: f32 = mul_legacy_f32_(_e1517.z, _e1517.w);
        let _e1521: f32 = mul_legacy_f32_(_e1520, fract((_e162 * 900.1793f)));
        let _e1524: vec4<f32> = cF.m[65u];
        let _e1527: f32 = mad_legacy_f32_(_e201, (1f / _e1524.x), _e1521);
        let _e1528: f32 = floor(_e1527);
        if (_e1524.y != 0f) {
            let _e1532: f32 = mul_legacy_f32_(_e1528, (1f / _e1517.w));
            let _e1536: f32 = mul_legacy_f32_(fract(abs(_e1532)), abs(_e1517.w));
            phi_1606_ = _e1536;
            if (_e1528 < 0f) {
                phi_1606_ = -(_e1536);
            }
            let _e1540: f32 = phi_1606_;
            phi_1608_ = _e1540;
        } else {
            phi_1608_ = min(_e1528, _e1517.w);
        }
        let _e1543: f32 = phi_1608_;
        let _e1545: f32 = mul_legacy_f32_(_e1543, _e1517.x);
        let _e1546: f32 = fract(_e1545);
        let _e1550: f32 = mad_legacy_f32_(floor(_e1545), _e1517.y, select(_e1517.y, 0f, _e1057));
        let _e1554: vec4<f32> = vec4<f32>(_e1038, _e1040, _e1042, 1f);
        let _e1557: vec4<f32> = cF.m[2u];
        let _e1560: vec4<f32> = cF.m[1u];
        let _e1563: vec4<f32> = cF.m[0u];
        if (_e147.z < 0.5f) {
            let _e1564: f32 = dp4_f32_legacy(_e1554, _e1557);
            let _e1565: f32 = dp4_f32_legacy(_e1554, _e1560);
            let _e1566: f32 = dp4_f32_legacy(_e1554, _e1563);
            phi_1675_ = _e1564;
            phi_1676_ = _e1565;
            phi_1677_ = _e1566;
        } else {
            phi_1672_ = _e1038;
            phi_1673_ = _e1040;
            phi_1674_ = _e1042;
            if (_e147.z < 1.5f) {
                let _e1568: f32 = (_e335 * 2f);
                let _e1569: f32 = (_e337 * 2f);
                let _e1570: f32 = mul_legacy_f32_(_e339, _e1568);
                let _e1571: f32 = mul_legacy_f32_(_e337, _e1568);
                let _e1572: f32 = (_e339 * 2f);
                let _e1573: f32 = mul_legacy_f32_(_e337, _e1569);
                let _e1574: f32 = -(_e1572);
                let _e1576: f32 = mad_legacy_f32_(_e339, _e1574, -(_e1573));
                let _e1578: f32 = mul_legacy_f32_(_e341, (_e341 * 2f));
                let _e1580: f32 = -(_e1578);
                let _e1581: f32 = mad_legacy_f32_(_e337, -(_e1569), _e1580);
                let _e1583: f32 = mad_legacy_f32_(_e341, _e1572, -(_e1571));
                let _e1584: f32 = mad_legacy_f32_(_e341, _e1569, _e1570);
                let _e1587: f32 = dp4_f32_legacy(vec4<f32>(_e1583, _e1584, (_e1576 + 1f), _e343), _e1554);
                let _e1588: f32 = mul_legacy_f32_(_e341, _e1568);
                let _e1589: f32 = mad_legacy_f32_(_e339, _e1574, _e1580);
                let _e1590: f32 = mad_legacy_f32_(_e337, _e1572, _e1588);
                let _e1593: f32 = mad_legacy_f32_(_e341, _e1569, -(_e1570));
                let _e1595: f32 = dp4_f32_legacy(_e1554, vec4<f32>(_e1590, (_e1589 + 1f), _e1593, _e345));
                let _e1598: f32 = mad_legacy_f32_(_e337, _e1572, -(_e1588));
                let _e1599: f32 = mad_legacy_f32_(_e341, _e1572, _e1571);
                let _e1601: f32 = dp4_f32_legacy(_e1554, vec4<f32>((_e1581 + 1f), _e1598, _e1599, _e347));
                phi_1672_ = _e1601;
                phi_1673_ = _e1595;
                phi_1674_ = _e1587;
            }
            let _e1603: f32 = phi_1672_;
            let _e1605: f32 = phi_1673_;
            let _e1607: f32 = phi_1674_;
            phi_1675_ = _e1607;
            phi_1676_ = _e1605;
            phi_1677_ = _e1603;
        }
        let _e1609: f32 = phi_1675_;
        let _e1611: f32 = phi_1676_;
        let _e1613: f32 = phi_1677_;
        let _e1616: vec4<f32> = cF.m[66u];
        let _e1631: vec4<f32> = cF.m[67u];
        let _e1633: f32 = (1f / _e1631.x);
        let _e1635: f32 = (1f / _e1631.z);
        let _e1637: f32 = (1f / _e1631.y);
        let _e1638: f32 = mul_legacy_f32_((_e1613 - _e1616.x), _e1633);
        let _e1639: f32 = mul_legacy_f32_((_e1611 - _e1616.y), _e1637);
        let _e1640: f32 = mul_legacy_f32_((_e1609 - _e1616.z), _e1635);
        let _e1650: f32 = abs(_e1631.x);
        let _e1651: f32 = abs(_e1631.y);
        let _e1652: f32 = abs(_e1631.z);
        let _e1653: bool = (abs(select(0f, 1f, (_e1613 < _e1616.x))) > 0f);
        let _e1655: bool = (abs(select(0f, 1f, (_e1611 < _e1616.y))) > 0f);
        let _e1657: bool = (abs(select(0f, 1f, (_e1609 < _e1616.z))) > 0f);
        let _e1659: f32 = mul_legacy_f32_(fract(abs(_e1638)), _e1650);
        let _e1660: f32 = mul_legacy_f32_(fract(abs(_e1639)), _e1651);
        let _e1661: f32 = mul_legacy_f32_(fract(abs(_e1640)), _e1652);
        let _e1665: f32 = mul_legacy_f32_(_e1659, (1f - select(0f, 1f, _e1653)));
        let _e1666: f32 = mul_legacy_f32_(_e1660, (1f - select(0f, 1f, _e1655)));
        let _e1667: f32 = mul_legacy_f32_(_e1661, (1f - select(0f, 1f, _e1657)));
        let _e1669: f32 = select(_e1665, (_e1665 - _e1659), _e1653);
        let _e1671: f32 = select(_e1666, (_e1666 - _e1660), _e1655);
        let _e1673: f32 = select(_e1667, (_e1667 - _e1661), _e1657);
        let _e1676: vec4<f32> = cF.m[68u];
        let _e1683: f32 = mul_legacy_f32_(_e1633, (_e1669 + _e1676.x));
        let _e1684: f32 = mul_legacy_f32_(_e1637, (_e1671 + _e1676.y));
        let _e1685: f32 = mul_legacy_f32_(_e1635, (_e1673 + _e1676.z));
        let _e1698: bool = (abs(select(0f, 1f, (_e1669 < -(_e1676.x)))) > 0f);
        let _e1700: bool = (abs(select(0f, 1f, (_e1671 < -(_e1676.y)))) > 0f);
        let _e1702: bool = (abs(select(0f, 1f, (_e1673 < -(_e1676.z)))) > 0f);
        let _e1710: f32 = mul_legacy_f32_(_e1650, fract(abs(_e1683)));
        let _e1711: f32 = mul_legacy_f32_(_e1651, fract(abs(_e1684)));
        let _e1712: f32 = mul_legacy_f32_(_e1652, fract(abs(_e1685)));
        let _e1716: f32 = mul_legacy_f32_(_e1710, (1f - select(0f, 1f, _e1698)));
        let _e1717: f32 = mul_legacy_f32_(_e1711, (1f - select(0f, 1f, _e1700)));
        let _e1718: f32 = mul_legacy_f32_(_e1712, (1f - select(0f, 1f, _e1702)));
        let _e1725: f32 = (select(_e1716, (_e1716 - _e1710), _e1698) + _e1616.x);
        let _e1726: f32 = (select(_e1717, (_e1717 - _e1711), _e1700) + _e1616.y);
        let _e1727: f32 = (select(_e1718, (_e1718 - _e1712), _e1702) + _e1616.z);
        let _e1730: vec4<f32> = cF.m[69u];
        let _e1737: vec3<f32> = vec3<f32>((_e1725 - _e1730.x), (_e1726 - _e1730.y), (_e1727 - _e1730.z));
        let _e1738: f32 = dp3_f32_(_e1737, _e1737);
        let _e1739: f32 = sqrt(_e1738);
        let _e1742: vec4<f32> = cF.m[67u];
        let _e1748: vec4<f32> = cF.m[66u];
        let _e1751: f32 = mul_legacy_f32_(_e1739, (1f / _e1748.w));
        let _e1752: f32 = clamp(_e1751, 0f, 1f);
        let _e1755: f32 = mul_legacy_f32_((_e1739 - _e1742.w), (1f / fma(_e1742.w, 0.5f, -(_e1742.w))));
        let _e1756: f32 = clamp(_e1755, 0f, 1f);
        let _e1760: f32 = mul_legacy_f32_((_e1752 * _e1752), fma(-(_e1752), 2f, 3f));
        let _e1761: f32 = mul_legacy_f32_(_e1506, _e1760);
        let _e1765: f32 = mul_legacy_f32_((_e1756 * _e1756), fma(-(_e1756), 2f, 3f));
        let _e1766: f32 = mul_legacy_f32_(_e1761, _e1765);
        if (_e147.w > 0.5f) {
            let _e1772: bool = (_e147.w > 1.5f);
            let _e1773: f32 = select(_e223.z, _e1557.y, _e1772);
            let _e1774: f32 = select(_e223.y, _e1560.y, _e1772);
            let _e1775: f32 = select(_e223.x, _e1563.y, _e1772);
            let _e1776: vec3<f32> = vec3<f32>(_e1775, _e1774, _e1773);
            let _e1777: f32 = dp3_f32_(_e1776, _e1776);
            let _e1778: f32 = inverseSqrt(_e1777);
            let _e1782: vec3<f32> = vec3<f32>(_e276.x, _e276.y, _e276.z);
            let _e1783: f32 = dp3_f32_(_e1782, _e1782);
            let _e1784: f32 = inverseSqrt(_e1783);
            let _e1785: f32 = mul_legacy_f32_(_e1778, _e1775);
            let _e1786: f32 = mul_legacy_f32_(_e1778, _e1774);
            let _e1787: f32 = mul_legacy_f32_(_e1778, _e1773);
            let _e1788: f32 = mul_legacy_f32_(_e1784, _e276.x);
            let _e1789: f32 = mul_legacy_f32_(_e1784, _e276.y);
            let _e1790: f32 = mul_legacy_f32_(_e1784, _e276.z);
            let _e1791: f32 = mul_legacy_f32_(_e1787, _e1789);
            let _e1792: f32 = mul_legacy_f32_(_e1785, _e1790);
            let _e1793: f32 = mul_legacy_f32_(_e1786, _e1788);
            let _e1797: f32 = mad_legacy_f32_(_e1786, _e1790, -(_e1791));
            let _e1798: f32 = mad_legacy_f32_(_e1787, _e1788, -(_e1792));
            let _e1799: f32 = mad_legacy_f32_(_e1785, _e1789, -(_e1793));
            let _e1800: vec3<f32> = vec3<f32>(_e1797, _e1798, _e1799);
            let _e1801: f32 = dp3_f32_(_e1800, _e1800);
            let _e1802: f32 = inverseSqrt(_e1801);
            let _e1803: f32 = mul_legacy_f32_(_e1219, _e1785);
            let _e1804: f32 = mul_legacy_f32_(_e1219, _e1786);
            let _e1805: f32 = mul_legacy_f32_(_e1219, _e1787);
            let _e1806: f32 = mul_legacy_f32_(_e1802, _e1797);
            let _e1807: f32 = mul_legacy_f32_(_e1802, _e1798);
            let _e1808: f32 = mul_legacy_f32_(_e1802, _e1799);
            let _e1809: f32 = mad_legacy_f32_(_e1221, _e1806, _e1803);
            let _e1810: f32 = mad_legacy_f32_(_e1221, _e1807, _e1804);
            let _e1811: f32 = mad_legacy_f32_(_e1221, _e1808, _e1805);
            phi_1898_ = (_e1809 + _e1725);
            phi_1899_ = (_e1810 + _e1726);
            phi_1900_ = (_e1811 + _e1727);
        } else {
            let _e1815: vec3<f32> = vec3<f32>(_e1221, _e1219, 0f);
            let _e1819: f32 = dp3_f32_legacy(_e1815, vec3<f32>(_e1557.x, _e1557.y, _e1557.z));
            let _e1823: f32 = dp3_f32_legacy(_e1815, vec3<f32>(_e1563.x, _e1563.y, _e1563.z));
            let _e1827: f32 = dp3_f32_legacy(_e1815, vec3<f32>(_e1560.x, _e1560.y, _e1560.z));
            phi_1898_ = (_e1725 + _e1823);
            phi_1899_ = (_e1726 + _e1827);
            phi_1900_ = (_e1727 + _e1819);
        }
        let _e1832: f32 = phi_1898_;
        let _e1834: f32 = phi_1899_;
        let _e1836: f32 = phi_1900_;
        let _e1839: vec4<f32> = cF.m[18u];
        let _e1844: f32 = mul_legacy_f32_(_e1508, _e1839.x);
        let _e1845: f32 = mul_legacy_f32_(_e1510, _e1839.y);
        let _e1846: f32 = mul_legacy_f32_(_e1512, _e1839.z);
        let _e1847: f32 = mul_legacy_f32_(_e1766, _e1839.w);
        phi_1912_ = _e1550;
        phi_1913_ = select((_e1517.x + _e1546), _e1546, _e1059);
        phi_1914_ = 0f;
        phi_1915_ = 0f;
        phi_1916_ = 0f;
        phi_1917_ = 0f;
        phi_1918_ = _e1847;
        phi_1919_ = _e1846;
        phi_1920_ = _e1845;
        phi_1921_ = _e1844;
        phi_1922_ = _e1836;
        phi_1923_ = _e1834;
        phi_1924_ = _e1832;
    }
    let _e1849: f32 = phi_1912_;
    let _e1851: f32 = phi_1913_;
    let _e1853: f32 = phi_1914_;
    let _e1855: f32 = phi_1915_;
    let _e1857: f32 = phi_1916_;
    let _e1859: f32 = phi_1917_;
    let _e1861: f32 = phi_1918_;
    let _e1863: f32 = phi_1919_;
    let _e1865: f32 = phi_1920_;
    let _e1867: f32 = phi_1921_;
    let _e1869: f32 = phi_1922_;
    let _e1871: f32 = phi_1923_;
    let _e1873: f32 = phi_1924_;
    let _e1874: vec4<f32> = vec4<f32>(_e1873, _e1871, _e1869, 1f);
    let _e1877: vec4<f32> = cF.m[13u];
    let _e1878: f32 = dp4_f32_legacy(_e1874, _e1877);
    let _e1879: f32 = dp4_f32_legacy(_e1874, _e276);
    let _e1882: vec4<f32> = cF.m[10u];
    let _e1883: f32 = dp4_f32_legacy(_e1874, _e1882);
    let _e1884: f32 = dp4_f32_legacy(_e1874, _e223);
    let _e1885: f32 = (_e1878 * 0.5f);
    let _e1890: vec4<f32> = cF.m[70u];
    let _e1895: f32 = mul_legacy_f32_(_e1878, (1f / _e1890.x));
    let _e1896: f32 = mul_legacy_f32_(_e1878, (1f / _e1890.y));
    let _e1901: vec4<f32> = cF.m[8u];
    o0_position0_[0u] = _e1883;
    o0_position0_[1u] = _e1884;
    o0_position0_[2u] = _e1879;
    o0_position0_[3u] = _e1878;
    o1_color[0u] = _e1867;
    o1_color[1u] = _e1865;
    o1_color[2u] = _e1863;
    o1_color[3u] = _e1861;
    o2_specular0_[0u] = _e1859;
    o2_specular0_[1u] = _e1857;
    o2_specular0_[2u] = _e1855;
    o2_specular0_[3u] = _e1853;
    o3_texcoord0_[0u] = (_e1851 + _e1901.z);
    o3_texcoord0_[1u] = (_e1849 + _e1901.w);
    o3_texcoord0_[2u] = 0f;
    o3_texcoord0_[3u] = 0f;
    o4_texcoord2_[0u] = fma(_e1895, 0.25f, fma(_e1883, 0.5f, _e1885));
    o4_texcoord2_[1u] = fma(_e1896, 0.25f, fma(_e1884, -0.5f, _e1885));
    o4_texcoord2_[2u] = _e1879;
    o4_texcoord2_[3u] = _e1878;
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
    let _e18: vec4<f32> = o0_position0_;
    let _e19: vec4<f32> = o1_color;
    let _e20: vec4<f32> = o2_specular0_;
    let _e21: vec4<f32> = o3_texcoord0_;
    let _e22: vec4<f32> = o4_texcoord2_;
    return VertexOutput(_e18, _e19, _e20, _e21, _e22);
}
