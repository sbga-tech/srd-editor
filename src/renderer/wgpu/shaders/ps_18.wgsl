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
var<private> v1_texcoord2_1: vec4<f32>;
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
    let _e44: vec4<f32> = textureSample(s0_2d, s0_, vec2<f32>(texCoord.x, texCoord.y));
    return _e44;
}

fn main_1() {
    let _e41: f32 = v1_texcoord2_1[3u];
    let _e42: f32 = (1f / _e41);
    let _e44: f32 = v1_texcoord2_1[0u];
    let _e46: f32 = v1_texcoord2_1[1u];
    let _e47: f32 = mul_legacy_f32_(_e44, _e42);
    let _e48: f32 = mul_legacy_f32_(_e46, _e42);
    let _e51: vec4<f32> = cF.m[2u];
    let _e53: f32 = (1f / _e51.y);
    let _e55: f32 = (1f / _e51.x);
    let _e57: f32 = v0_color_1[3u];
    let _e58: f32 = (_e57 * 0.5f);
    let _e59: f32 = mul_legacy_f32_(_e58, _e55);
    let _e60: f32 = mul_legacy_f32_(_e58, _e53);
    let _e61: f32 = fma(_e59, -4f, _e47);
    let _e62: f32 = fma(_e60, -3f, _e48);
    let _e63: f32 = fma(_e60, -4f, _e48);
    let _e65: vec4<f32> = sampleTexture_0_(vec4<f32>(_e61, _e62, f32(), _e58));
    let _e69: f32 = fma(_e60, -2f, _e48);
    let _e71: vec4<f32> = sampleTexture_0_(vec4<f32>(_e61, _e63, f32(), f32()));
    let _e82: vec4<f32> = sampleTexture_0_(vec4<f32>(_e61, _e69, _e55, _e53));
    let _e89: f32 = (_e48 - _e60);
    let _e91: vec4<f32> = sampleTexture_0_(vec4<f32>(_e61, _e89, _e82.z, f32()));
    let _e99: vec4<f32> = sampleTexture_0_(vec4<f32>(_e61, _e48, _e55, _e53));
    let _e106: f32 = (_e60 + _e48);
    let _e108: vec4<f32> = sampleTexture_0_(vec4<f32>(_e61, _e106, _e99.z, f32()));
    let _e112: f32 = fma(_e60, 2f, _e48);
    let _e117: vec4<f32> = sampleTexture_0_(vec4<f32>(_e61, _e112, _e55, _e53));
    let _e124: f32 = fma(_e60, 3f, _e48);
    let _e126: vec4<f32> = sampleTexture_0_(vec4<f32>(_e61, _e124, _e117.z, f32()));
    let _e130: f32 = fma(_e60, 4f, _e48);
    let _e135: vec4<f32> = sampleTexture_0_(vec4<f32>(_e61, _e130, _e55, _e53));
    let _e142: f32 = fma(_e59, -3f, _e47);
    let _e144: vec4<f32> = sampleTexture_0_(vec4<f32>(_e142, _e63, _e135.z, f32()));
    let _e152: vec4<f32> = sampleTexture_0_(vec4<f32>(_e142, _e62, _e55, _e53));
    let _e160: vec4<f32> = sampleTexture_0_(vec4<f32>(_e142, _e69, _e152.z, f32()));
    let _e168: vec4<f32> = sampleTexture_0_(vec4<f32>(_e142, _e89, _e55, _e53));
    let _e176: vec4<f32> = sampleTexture_0_(vec4<f32>(_e142, _e48, _e168.z, f32()));
    let _e184: vec4<f32> = sampleTexture_0_(vec4<f32>(_e142, _e106, _e55, _e53));
    let _e192: vec4<f32> = sampleTexture_0_(vec4<f32>(_e142, _e112, _e184.z, f32()));
    let _e200: vec4<f32> = sampleTexture_0_(vec4<f32>(_e142, _e124, _e55, _e53));
    let _e208: vec4<f32> = sampleTexture_0_(vec4<f32>(_e142, _e130, _e200.z, f32()));
    let _e212: f32 = fma(_e59, -2f, _e47);
    let _e217: vec4<f32> = sampleTexture_0_(vec4<f32>(_e212, _e63, _e55, _e53));
    let _e225: vec4<f32> = sampleTexture_0_(vec4<f32>(_e212, _e62, _e217.z, f32()));
    let _e233: vec4<f32> = sampleTexture_0_(vec4<f32>(_e212, _e69, _e55, _e53));
    let _e241: vec4<f32> = sampleTexture_0_(vec4<f32>(_e212, _e89, _e233.z, f32()));
    let _e249: vec4<f32> = sampleTexture_0_(vec4<f32>(_e212, _e48, _e55, _e53));
    let _e257: vec4<f32> = sampleTexture_0_(vec4<f32>(_e212, _e106, _e249.z, f32()));
    let _e265: vec4<f32> = sampleTexture_0_(vec4<f32>(_e212, _e112, _e55, _e53));
    let _e273: vec4<f32> = sampleTexture_0_(vec4<f32>(_e212, _e124, _e265.z, f32()));
    let _e281: vec4<f32> = sampleTexture_0_(vec4<f32>(_e212, _e130, _e55, _e53));
    let _e288: f32 = (_e47 - _e59);
    let _e290: vec4<f32> = sampleTexture_0_(vec4<f32>(_e288, _e63, _e281.z, f32()));
    let _e298: vec4<f32> = sampleTexture_0_(vec4<f32>(_e288, _e62, _e55, _e53));
    let _e306: vec4<f32> = sampleTexture_0_(vec4<f32>(_e288, _e69, _e298.z, f32()));
    let _e310: f32 = -(_e58);
    let _e311: f32 = mad_legacy_f32_(_e310, _e55, _e47);
    let _e312: f32 = mad_legacy_f32_(_e310, _e53, _e48);
    let _e317: vec4<f32> = sampleTexture_0_(vec4<f32>(_e311, _e312, _e55, _e53));
    let _e325: vec4<f32> = sampleTexture_0_(vec4<f32>(_e288, _e48, _e55, _e53));
    let _e333: vec4<f32> = sampleTexture_0_(vec4<f32>(_e288, _e106, _e325.z, f32()));
    let _e341: vec4<f32> = sampleTexture_0_(vec4<f32>(_e288, _e112, _e55, _e53));
    let _e349: vec4<f32> = sampleTexture_0_(vec4<f32>(_e288, _e124, _e341.z, f32()));
    let _e357: vec4<f32> = sampleTexture_0_(vec4<f32>(_e288, _e130, _e55, _e53));
    let _e365: vec4<f32> = sampleTexture_0_(vec4<f32>(_e47, _e63, _e357.z, f32()));
    let _e373: vec4<f32> = sampleTexture_0_(vec4<f32>(_e47, _e62, _e55, _e53));
    let _e381: vec4<f32> = sampleTexture_0_(vec4<f32>(_e47, _e69, _e373.z, f32()));
    let _e389: vec4<f32> = sampleTexture_0_(vec4<f32>(_e47, _e89, _e55, _e53));
    let _e395: f32 = fma(_e389.z, 0.0391596f, fma(_e381.z, 0.02691384f, fma(_e373.z, 0.01440603f, fma(_e365.z, 0.0060052f, fma(_e357.z, 0.00475201f, fma(_e349.z, 0.01139972f, fma(_e341.z, 0.02129735f, fma(_e333.z, 0.03098762f, fma(_e325.z, 0.0391596f, fma(_e317.z, 0.03098762f, fma(_e306.z, 0.02129735f, fma(_e298.z, 0.01139972f, fma(_e290.z, 0.00475201f, fma(_e281.z, 0.00326599f, fma(_e273.z, 0.00783487f, fma(_e265.z, 0.01463737f, fma(_e257.z, 0.02129735f, fma(_e249.z, 0.02691384f, fma(_e241.z, 0.02129735f, fma(_e233.z, 0.01463737f, fma(_e225.z, 0.00783487f, fma(_e217.z, 0.00326599f, fma(_e208.z, 0.00174817f, fma(_e200.z, 0.00419373f, fma(_e192.z, 0.00783487f, fma(_e184.z, 0.01139972f, fma(_e176.z, 0.01440603f, fma(_e168.z, 0.01139972f, fma(_e160.z, 0.00783487f, fma(_e152.z, 0.00419373f, fma(_e144.z, 0.00174817f, fma(_e135.z, 0.00072873f, fma(_e126.z, 0.00174817f, fma(_e117.z, 0.00326599f, fma(_e108.z, 0.00475201f, fma(_e99.z, 0.0060052f, fma(_e91.z, 0.00475201f, fma(_e82.z, 0.00326599f, fma(_e71.z, 0.00072873f, (_e65.z * 0.00174817f))))))))))))))))))))))))))))))))))))))));
    let _e397: vec4<f32> = sampleTexture_0_(vec4<f32>(_e47, _e48, _e59, _e60));
    let _e405: vec4<f32> = sampleTexture_0_(vec4<f32>(_e47, _e106, _e395, f32()));
    let _e413: vec4<f32> = sampleTexture_0_(vec4<f32>(_e47, _e112, _e55, _e53));
    let _e421: vec4<f32> = sampleTexture_0_(vec4<f32>(_e47, _e124, _e413.z, f32()));
    let _e429: vec4<f32> = sampleTexture_0_(vec4<f32>(_e47, _e130, _e55, _e53));
    let _e436: f32 = (_e59 + _e47);
    let _e438: vec4<f32> = sampleTexture_0_(vec4<f32>(_e436, _e63, _e429.z, f32()));
    let _e446: vec4<f32> = sampleTexture_0_(vec4<f32>(_e436, _e62, _e55, _e53));
    let _e454: vec4<f32> = sampleTexture_0_(vec4<f32>(_e436, _e69, _e446.z, f32()));
    let _e462: vec4<f32> = sampleTexture_0_(vec4<f32>(_e436, _e89, _e55, _e53));
    let _e470: vec4<f32> = sampleTexture_0_(vec4<f32>(_e436, _e48, _e462.z, f32()));
    let _e474: f32 = mad_legacy_f32_(_e58, _e55, _e47);
    let _e475: f32 = mad_legacy_f32_(_e58, _e53, _e48);
    let _e480: vec4<f32> = sampleTexture_0_(vec4<f32>(_e474, _e475, _e55, _e53));
    let _e488: vec4<f32> = sampleTexture_0_(vec4<f32>(_e436, _e112, _e480.z, f32()));
    let _e496: vec4<f32> = sampleTexture_0_(vec4<f32>(_e436, _e124, _e55, _e53));
    let _e504: vec4<f32> = sampleTexture_0_(vec4<f32>(_e436, _e130, _e496.z, f32()));
    let _e508: f32 = fma(_e59, 2f, _e47);
    let _e513: vec4<f32> = sampleTexture_0_(vec4<f32>(_e508, _e63, _e55, _e53));
    let _e521: vec4<f32> = sampleTexture_0_(vec4<f32>(_e508, _e62, _e513.z, f32()));
    let _e529: vec4<f32> = sampleTexture_0_(vec4<f32>(_e508, _e69, _e55, _e53));
    let _e537: vec4<f32> = sampleTexture_0_(vec4<f32>(_e508, _e89, _e529.z, f32()));
    let _e545: vec4<f32> = sampleTexture_0_(vec4<f32>(_e508, _e48, _e55, _e53));
    let _e549: f32 = mad_legacy_f32_(_e57, _e55, _e47);
    let _e550: f32 = mad_legacy_f32_(_e57, _e53, _e48);
    let _e555: vec4<f32> = sampleTexture_0_(vec4<f32>(_e508, _e106, _e545.z, f32()));
    let _e561: f32 = fma(_e555.z, 0.02129735f, fma(_e545.z, 0.02691384f, fma(_e537.z, 0.02129735f, fma(_e529.z, 0.01463737f, fma(_e521.z, 0.00783487f, fma(_e513.z, 0.00326599f, fma(_e504.z, 0.00475201f, fma(_e496.z, 0.01139972f, fma(_e488.z, 0.02129735f, fma(_e480.z, 0.03098762f, fma(_e470.z, 0.0391596f, fma(_e462.z, 0.03098762f, fma(_e454.z, 0.02129735f, fma(_e446.z, 0.01139972f, fma(_e438.z, 0.00475201f, fma(_e429.z, 0.0060052f, fma(_e421.z, 0.01440603f, fma(_e413.z, 0.02691384f, fma(_e405.z, 0.0391596f, fma(_e397.z, 0.04948667f, _e395))))))))))))))))))));
    let _e563: vec4<f32> = sampleTexture_0_(vec4<f32>(_e549, _e550, _e55, _e53));
    let _e571: vec4<f32> = sampleTexture_0_(vec4<f32>(_e508, _e124, _e555.z, f32()));
    let _e579: vec4<f32> = sampleTexture_0_(vec4<f32>(_e508, _e130, _e561, _e58));
    let _e583: f32 = fma(_e59, 3f, _e47);
    let _e588: vec4<f32> = sampleTexture_0_(vec4<f32>(_e583, _e63, _e571.z, f32()));
    let _e596: vec4<f32> = sampleTexture_0_(vec4<f32>(_e583, _e62, _e579.z, _e58));
    let _e604: vec4<f32> = sampleTexture_0_(vec4<f32>(_e583, _e69, _e588.z, f32()));
    let _e612: vec4<f32> = sampleTexture_0_(vec4<f32>(_e583, _e89, _e596.z, _e58));
    let _e620: vec4<f32> = sampleTexture_0_(vec4<f32>(_e583, _e48, _e604.z, f32()));
    let _e628: vec4<f32> = sampleTexture_0_(vec4<f32>(_e583, _e106, _e612.z, _e58));
    let _e636: vec4<f32> = sampleTexture_0_(vec4<f32>(_e583, _e112, _e620.z, f32()));
    let _e644: vec4<f32> = sampleTexture_0_(vec4<f32>(_e583, _e124, _e628.z, _e58));
    let _e651: f32 = fma(_e59, 4f, _e47);
    let _e653: vec4<f32> = sampleTexture_0_(vec4<f32>(_e583, _e130, _e636.z, f32()));
    let _e661: vec4<f32> = sampleTexture_0_(vec4<f32>(_e651, _e63, _e644.z, _e58));
    let _e669: vec4<f32> = sampleTexture_0_(vec4<f32>(_e651, _e62, _e653.z, f32()));
    let _e677: vec4<f32> = sampleTexture_0_(vec4<f32>(_e651, _e69, _e661.z, _e58));
    let _e685: vec4<f32> = sampleTexture_0_(vec4<f32>(_e651, _e89, _e669.z, f32()));
    let _e693: vec4<f32> = sampleTexture_0_(vec4<f32>(_e651, _e48, _e677.z, _e58));
    let _e701: vec4<f32> = sampleTexture_0_(vec4<f32>(_e651, _e106, _e685.z, f32()));
    let _e706: vec4<f32> = sampleTexture_0_(vec4<f32>(_e651, _e112, _e693.z, _e58));
    let _e717: vec4<f32> = sampleTexture_0_(vec4<f32>(_e651, _e124, _e706.z, _e58));
    let _e725: vec4<f32> = sampleTexture_0_(vec4<f32>(_e651, _e130, _e59, _e60));
    let _e734: vec4<f32> = cF.m[0u];
    let _e736: f32 = mul_legacy_f32_(fma(_e725.x, 0.00072873f, fma(_e717.x, 0.00174817f, fma(_e706.x, 0.00326599f, fma(_e701.x, 0.00475201f, fma(_e693.x, 0.0060052f, fma(_e685.x, 0.00475201f, fma(_e677.x, 0.00326599f, fma(_e669.x, 0.00174817f, fma(_e661.x, 0.00072873f, fma(_e653.x, 0.00174817f, fma(_e644.x, 0.00419373f, fma(_e636.x, 0.00783487f, fma(_e628.x, 0.01139972f, fma(_e620.x, 0.01440603f, fma(_e612.x, 0.01139972f, fma(_e604.x, 0.00783487f, fma(_e596.x, 0.00419373f, fma(_e588.x, 0.00174817f, fma(_e579.x, 0.00326599f, fma(_e571.x, 0.00783487f, fma(_e563.x, 0.01463737f, fma(_e555.x, 0.02129735f, fma(_e545.x, 0.02691384f, fma(_e537.x, 0.02129735f, fma(_e529.x, 0.01463737f, fma(_e521.x, 0.00783487f, fma(_e513.x, 0.00326599f, fma(_e504.x, 0.00475201f, fma(_e496.x, 0.01139972f, fma(_e488.x, 0.02129735f, fma(_e480.x, 0.03098762f, fma(_e470.x, 0.0391596f, fma(_e462.x, 0.03098762f, fma(_e454.x, 0.02129735f, fma(_e446.x, 0.01139972f, fma(_e438.x, 0.00475201f, fma(_e429.x, 0.0060052f, fma(_e421.x, 0.01440603f, fma(_e413.x, 0.02691384f, fma(_e405.x, 0.0391596f, fma(_e397.x, 0.04948667f, fma(_e389.x, 0.0391596f, fma(_e381.x, 0.02691384f, fma(_e373.x, 0.01440603f, fma(_e365.x, 0.0060052f, fma(_e357.x, 0.00475201f, fma(_e349.x, 0.01139972f, fma(_e341.x, 0.02129735f, fma(_e333.x, 0.03098762f, fma(_e325.x, 0.0391596f, fma(_e317.x, 0.03098762f, fma(_e306.x, 0.02129735f, fma(_e298.x, 0.01139972f, fma(_e290.x, 0.00475201f, fma(_e281.x, 0.00326599f, fma(_e273.x, 0.00783487f, fma(_e265.x, 0.01463737f, fma(_e257.x, 0.02129735f, fma(_e249.x, 0.02691384f, fma(_e241.x, 0.02129735f, fma(_e233.x, 0.01463737f, fma(_e225.x, 0.00783487f, fma(_e217.x, 0.00326599f, fma(_e208.x, 0.00174817f, fma(_e200.x, 0.00419373f, fma(_e192.x, 0.00783487f, fma(_e184.x, 0.01139972f, fma(_e176.x, 0.01440603f, fma(_e168.x, 0.01139972f, fma(_e160.x, 0.00783487f, fma(_e152.x, 0.00419373f, fma(_e144.x, 0.00174817f, fma(_e135.x, 0.00072873f, fma(_e126.x, 0.00174817f, fma(_e117.x, 0.00326599f, fma(_e108.x, 0.00475201f, fma(_e99.x, 0.0060052f, fma(_e91.x, 0.00475201f, fma(_e82.x, 0.00326599f, fma(_e71.x, 0.00072873f, (_e65.x * 0.00174817f))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))), _e734.z);
    let _e737: f32 = mul_legacy_f32_(fma(_e725.y, 0.00072873f, fma(_e717.y, 0.00174817f, fma(_e706.y, 0.00326599f, fma(_e701.y, 0.00475201f, fma(_e693.y, 0.0060052f, fma(_e685.y, 0.00475201f, fma(_e677.y, 0.00326599f, fma(_e669.y, 0.00174817f, fma(_e661.y, 0.00072873f, fma(_e653.y, 0.00174817f, fma(_e644.y, 0.00419373f, fma(_e636.y, 0.00783487f, fma(_e628.y, 0.01139972f, fma(_e620.y, 0.01440603f, fma(_e612.y, 0.01139972f, fma(_e604.y, 0.00783487f, fma(_e596.y, 0.00419373f, fma(_e588.y, 0.00174817f, fma(_e579.y, 0.00326599f, fma(_e571.y, 0.00783487f, fma(_e563.y, 0.01463737f, fma(_e555.y, 0.02129735f, fma(_e545.y, 0.02691384f, fma(_e537.y, 0.02129735f, fma(_e529.y, 0.01463737f, fma(_e521.y, 0.00783487f, fma(_e513.y, 0.00326599f, fma(_e504.y, 0.00475201f, fma(_e496.y, 0.01139972f, fma(_e488.y, 0.02129735f, fma(_e480.y, 0.03098762f, fma(_e470.y, 0.0391596f, fma(_e462.y, 0.03098762f, fma(_e454.y, 0.02129735f, fma(_e446.y, 0.01139972f, fma(_e438.y, 0.00475201f, fma(_e429.y, 0.0060052f, fma(_e421.y, 0.01440603f, fma(_e413.y, 0.02691384f, fma(_e405.y, 0.0391596f, fma(_e397.y, 0.04948667f, fma(_e389.y, 0.0391596f, fma(_e381.y, 0.02691384f, fma(_e373.y, 0.01440603f, fma(_e365.y, 0.0060052f, fma(_e357.y, 0.00475201f, fma(_e349.y, 0.01139972f, fma(_e341.y, 0.02129735f, fma(_e333.y, 0.03098762f, fma(_e325.y, 0.0391596f, fma(_e317.y, 0.03098762f, fma(_e306.y, 0.02129735f, fma(_e298.y, 0.01139972f, fma(_e290.y, 0.00475201f, fma(_e281.y, 0.00326599f, fma(_e273.y, 0.00783487f, fma(_e265.y, 0.01463737f, fma(_e257.y, 0.02129735f, fma(_e249.y, 0.02691384f, fma(_e241.y, 0.02129735f, fma(_e233.y, 0.01463737f, fma(_e225.y, 0.00783487f, fma(_e217.y, 0.00326599f, fma(_e208.y, 0.00174817f, fma(_e200.y, 0.00419373f, fma(_e192.y, 0.00783487f, fma(_e184.y, 0.01139972f, fma(_e176.y, 0.01440603f, fma(_e168.y, 0.01139972f, fma(_e160.y, 0.00783487f, fma(_e152.y, 0.00419373f, fma(_e144.y, 0.00174817f, fma(_e135.y, 0.00072873f, fma(_e126.y, 0.00174817f, fma(_e117.y, 0.00326599f, fma(_e108.y, 0.00475201f, fma(_e99.y, 0.0060052f, fma(_e91.y, 0.00475201f, fma(_e82.y, 0.00326599f, fma(_e71.y, 0.00072873f, (_e65.y * 0.00174817f))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))), _e734.z);
    let _e738: f32 = mul_legacy_f32_(fma(_e725.z, 0.00072873f, fma(_e717.z, 0.00174817f, fma(_e706.z, 0.00326599f, fma(_e701.z, 0.00475201f, fma(_e693.z, 0.0060052f, fma(_e685.z, 0.00475201f, fma(_e677.z, 0.00326599f, fma(_e669.z, 0.00174817f, fma(_e661.z, 0.00072873f, fma(_e653.z, 0.00174817f, fma(_e644.z, 0.00419373f, fma(_e636.z, 0.00783487f, fma(_e628.z, 0.01139972f, fma(_e620.z, 0.01440603f, fma(_e612.z, 0.01139972f, fma(_e604.z, 0.00783487f, fma(_e596.z, 0.00419373f, fma(_e588.z, 0.00174817f, fma(_e579.z, 0.00326599f, fma(_e571.z, 0.00783487f, fma(_e563.z, 0.01463737f, _e561))))))))))))))))))))), _e734.z);
    let _e762: bool = (f32(clamp((clamp((clamp((f16(select(1f, 0f, ((999.9f - _e736) >= 0f))) + f16(select(1f, 0f, ((999.9f - _e737) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e738) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((-1f + 999.9f) >= 0f)))), 0h, 1h)) <= 0f);
    oC0_color[0u] = select(0f, _e736, _e762);
    oC0_color[1u] = select(0f, _e737, _e762);
    oC0_color[2u] = select(0f, _e738, _e762);
    oC0_color[3u] = select(0f, 1f, _e762);
    return;
}

@fragment 
fn main(@location(9) v0_color: vec4<f32>, @location(3) v1_texcoord2_: vec4<f32>) -> @location(0) vec4<f32> {
    v0_color_1 = v0_color;
    v1_texcoord2_1 = v1_texcoord2_;
    main_1();
    let _e5: vec4<f32> = oC0_color;
    return _e5;
}
