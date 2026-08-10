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
var<private> v1_texcoord0_1: vec4<f32>;
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

fn sampleTexture_1_(texCoord: vec4<f32>) -> vec4<f32> {
    let _e46: vec4<f32> = textureSample(s1_2d, s1_, vec2<f32>(texCoord.x, texCoord.y));
    return _e46;
}

fn sampleTexture_0_(texCoord_1: vec4<f32>) -> vec4<f32> {
    let _e46: vec4<f32> = textureSample(s0_2d, s0_, vec2<f32>(texCoord_1.x, texCoord_1.y));
    return _e46;
}

fn main_1() {
    let _e43: f32 = v1_texcoord0_1[0u];
    let _e45: f32 = v1_texcoord0_1[1u];
    let _e47: f32 = v1_texcoord0_1[2u];
    let _e49: f32 = v1_texcoord0_1[3u];
    let _e51: vec4<f32> = sampleTexture_0_(vec4<f32>(_e43, _e45, _e47, _e49));
    let _e54: f32 = v2_texcoord2_1[3u];
    let _e55: f32 = (1f / _e54);
    let _e57: f32 = v2_texcoord2_1[0u];
    let _e59: f32 = v2_texcoord2_1[1u];
    let _e60: f32 = mul_legacy_f32_(_e57, _e55);
    let _e61: f32 = mul_legacy_f32_(_e59, _e55);
    let _e63: f32 = v0_color_1[3u];
    let _e64: f32 = mul_legacy_f32_(_e51.w, _e63);
    let _e67: vec4<f32> = cF.m[2u];
    let _e69: f32 = (1f / _e67.y);
    let _e71: f32 = (1f / _e67.x);
    let _e72: f32 = mul_legacy_f32_(_e64, _e71);
    let _e73: f32 = mul_legacy_f32_(_e64, _e69);
    let _e74: f32 = fma(_e72, -4f, _e60);
    let _e75: f32 = fma(_e73, -3f, _e61);
    let _e76: f32 = fma(_e73, -4f, _e61);
    let _e78: vec4<f32> = sampleTexture_1_(vec4<f32>(_e74, _e75, f32(), _e64));
    let _e82: f32 = fma(_e73, -2f, _e61);
    let _e84: vec4<f32> = sampleTexture_1_(vec4<f32>(_e74, _e76, f32(), f32()));
    let _e95: vec4<f32> = sampleTexture_1_(vec4<f32>(_e74, _e82, _e71, _e69));
    let _e102: f32 = (_e61 - _e73);
    let _e104: vec4<f32> = sampleTexture_1_(vec4<f32>(_e74, _e102, _e95.z, f32()));
    let _e112: vec4<f32> = sampleTexture_1_(vec4<f32>(_e74, _e61, _e71, _e69));
    let _e119: f32 = (_e73 + _e61);
    let _e121: vec4<f32> = sampleTexture_1_(vec4<f32>(_e74, _e119, _e112.z, f32()));
    let _e125: f32 = fma(_e73, 2f, _e61);
    let _e130: vec4<f32> = sampleTexture_1_(vec4<f32>(_e74, _e125, _e71, _e69));
    let _e137: f32 = fma(_e73, 3f, _e61);
    let _e139: vec4<f32> = sampleTexture_1_(vec4<f32>(_e74, _e137, _e130.z, f32()));
    let _e143: f32 = fma(_e73, 4f, _e61);
    let _e148: vec4<f32> = sampleTexture_1_(vec4<f32>(_e74, _e143, _e71, _e69));
    let _e155: f32 = fma(_e72, -3f, _e60);
    let _e157: vec4<f32> = sampleTexture_1_(vec4<f32>(_e155, _e76, _e148.z, f32()));
    let _e165: vec4<f32> = sampleTexture_1_(vec4<f32>(_e155, _e75, _e71, _e69));
    let _e173: vec4<f32> = sampleTexture_1_(vec4<f32>(_e155, _e82, _e165.z, f32()));
    let _e181: vec4<f32> = sampleTexture_1_(vec4<f32>(_e155, _e102, _e71, _e69));
    let _e189: vec4<f32> = sampleTexture_1_(vec4<f32>(_e155, _e61, _e181.z, f32()));
    let _e197: vec4<f32> = sampleTexture_1_(vec4<f32>(_e155, _e119, _e71, _e69));
    let _e205: vec4<f32> = sampleTexture_1_(vec4<f32>(_e155, _e125, _e197.z, f32()));
    let _e213: vec4<f32> = sampleTexture_1_(vec4<f32>(_e155, _e137, _e71, _e69));
    let _e221: vec4<f32> = sampleTexture_1_(vec4<f32>(_e155, _e143, _e213.z, f32()));
    let _e225: f32 = fma(_e72, -2f, _e60);
    let _e230: vec4<f32> = sampleTexture_1_(vec4<f32>(_e225, _e76, _e71, _e69));
    let _e238: vec4<f32> = sampleTexture_1_(vec4<f32>(_e225, _e75, _e230.z, f32()));
    let _e246: vec4<f32> = sampleTexture_1_(vec4<f32>(_e225, _e82, _e71, _e69));
    let _e254: vec4<f32> = sampleTexture_1_(vec4<f32>(_e225, _e102, _e246.z, f32()));
    let _e262: vec4<f32> = sampleTexture_1_(vec4<f32>(_e225, _e61, _e71, _e69));
    let _e270: vec4<f32> = sampleTexture_1_(vec4<f32>(_e225, _e119, _e262.z, f32()));
    let _e278: vec4<f32> = sampleTexture_1_(vec4<f32>(_e225, _e125, _e71, _e69));
    let _e286: vec4<f32> = sampleTexture_1_(vec4<f32>(_e225, _e137, _e278.z, f32()));
    let _e294: vec4<f32> = sampleTexture_1_(vec4<f32>(_e225, _e143, _e71, _e69));
    let _e301: f32 = (_e60 - _e72);
    let _e303: vec4<f32> = sampleTexture_1_(vec4<f32>(_e301, _e76, _e294.z, f32()));
    let _e311: vec4<f32> = sampleTexture_1_(vec4<f32>(_e301, _e75, _e71, _e69));
    let _e319: vec4<f32> = sampleTexture_1_(vec4<f32>(_e301, _e82, _e311.z, f32()));
    let _e323: f32 = -(_e64);
    let _e324: f32 = mad_legacy_f32_(_e323, _e71, _e60);
    let _e325: f32 = mad_legacy_f32_(_e323, _e69, _e61);
    let _e330: vec4<f32> = sampleTexture_1_(vec4<f32>(_e324, _e325, _e71, _e69));
    let _e338: vec4<f32> = sampleTexture_1_(vec4<f32>(_e301, _e61, _e71, _e69));
    let _e346: vec4<f32> = sampleTexture_1_(vec4<f32>(_e301, _e119, _e338.z, f32()));
    let _e354: vec4<f32> = sampleTexture_1_(vec4<f32>(_e301, _e125, _e71, _e69));
    let _e362: vec4<f32> = sampleTexture_1_(vec4<f32>(_e301, _e137, _e354.z, f32()));
    let _e370: vec4<f32> = sampleTexture_1_(vec4<f32>(_e301, _e143, _e71, _e69));
    let _e378: vec4<f32> = sampleTexture_1_(vec4<f32>(_e60, _e76, _e370.z, f32()));
    let _e386: vec4<f32> = sampleTexture_1_(vec4<f32>(_e60, _e75, _e71, _e69));
    let _e394: vec4<f32> = sampleTexture_1_(vec4<f32>(_e60, _e82, _e386.z, f32()));
    let _e402: vec4<f32> = sampleTexture_1_(vec4<f32>(_e60, _e102, _e71, _e69));
    let _e408: f32 = fma(_e402.z, 0.0391596f, fma(_e394.z, 0.02691384f, fma(_e386.z, 0.01440603f, fma(_e378.z, 0.0060052f, fma(_e370.z, 0.00475201f, fma(_e362.z, 0.01139972f, fma(_e354.z, 0.02129735f, fma(_e346.z, 0.03098762f, fma(_e338.z, 0.0391596f, fma(_e330.z, 0.03098762f, fma(_e319.z, 0.02129735f, fma(_e311.z, 0.01139972f, fma(_e303.z, 0.00475201f, fma(_e294.z, 0.00326599f, fma(_e286.z, 0.00783487f, fma(_e278.z, 0.01463737f, fma(_e270.z, 0.02129735f, fma(_e262.z, 0.02691384f, fma(_e254.z, 0.02129735f, fma(_e246.z, 0.01463737f, fma(_e238.z, 0.00783487f, fma(_e230.z, 0.00326599f, fma(_e221.z, 0.00174817f, fma(_e213.z, 0.00419373f, fma(_e205.z, 0.00783487f, fma(_e197.z, 0.01139972f, fma(_e189.z, 0.01440603f, fma(_e181.z, 0.01139972f, fma(_e173.z, 0.00783487f, fma(_e165.z, 0.00419373f, fma(_e157.z, 0.00174817f, fma(_e148.z, 0.00072873f, fma(_e139.z, 0.00174817f, fma(_e130.z, 0.00326599f, fma(_e121.z, 0.00475201f, fma(_e112.z, 0.0060052f, fma(_e104.z, 0.00475201f, fma(_e95.z, 0.00326599f, fma(_e84.z, 0.00072873f, (_e78.z * 0.00174817f))))))))))))))))))))))))))))))))))))))));
    let _e410: vec4<f32> = sampleTexture_1_(vec4<f32>(_e60, _e61, _e72, _e73));
    let _e418: vec4<f32> = sampleTexture_1_(vec4<f32>(_e60, _e119, _e408, f32()));
    let _e426: vec4<f32> = sampleTexture_1_(vec4<f32>(_e60, _e125, _e71, _e69));
    let _e434: vec4<f32> = sampleTexture_1_(vec4<f32>(_e60, _e137, _e426.z, f32()));
    let _e442: vec4<f32> = sampleTexture_1_(vec4<f32>(_e60, _e143, _e71, _e69));
    let _e449: f32 = (_e72 + _e60);
    let _e451: vec4<f32> = sampleTexture_1_(vec4<f32>(_e449, _e76, _e442.z, f32()));
    let _e459: vec4<f32> = sampleTexture_1_(vec4<f32>(_e449, _e75, _e71, _e69));
    let _e467: vec4<f32> = sampleTexture_1_(vec4<f32>(_e449, _e82, _e459.z, f32()));
    let _e475: vec4<f32> = sampleTexture_1_(vec4<f32>(_e449, _e102, _e71, _e69));
    let _e479: f32 = mad_legacy_f32_(_e64, _e71, _e60);
    let _e480: f32 = mad_legacy_f32_(_e64, _e69, _e61);
    let _e485: vec4<f32> = sampleTexture_1_(vec4<f32>(_e449, _e61, _e475.z, f32()));
    let _e491: f32 = fma(_e485.z, 0.0391596f, fma(_e475.z, 0.03098762f, fma(_e467.z, 0.02129735f, fma(_e459.z, 0.01139972f, fma(_e451.z, 0.00475201f, fma(_e442.z, 0.0060052f, fma(_e434.z, 0.01440603f, fma(_e426.z, 0.02691384f, fma(_e418.z, 0.0391596f, fma(_e410.z, 0.04948667f, _e408))))))))));
    let _e493: vec4<f32> = sampleTexture_1_(vec4<f32>(_e479, _e480, _e71, _e69));
    let _e501: vec4<f32> = sampleTexture_1_(vec4<f32>(_e449, _e125, _e485.z, f32()));
    let _e509: vec4<f32> = sampleTexture_1_(vec4<f32>(_e449, _e137, _e491, _e64));
    let _e516: f32 = fma(_e72, 2f, _e60);
    let _e518: vec4<f32> = sampleTexture_1_(vec4<f32>(_e449, _e143, _e501.z, f32()));
    let _e526: vec4<f32> = sampleTexture_1_(vec4<f32>(_e516, _e76, _e509.z, _e64));
    let _e534: vec4<f32> = sampleTexture_1_(vec4<f32>(_e516, _e75, _e518.z, f32()));
    let _e542: vec4<f32> = sampleTexture_1_(vec4<f32>(_e516, _e82, _e526.z, _e64));
    let _e550: vec4<f32> = sampleTexture_1_(vec4<f32>(_e516, _e102, _e534.z, f32()));
    let _e558: vec4<f32> = sampleTexture_1_(vec4<f32>(_e516, _e61, _e542.z, _e64));
    let _e566: vec4<f32> = sampleTexture_1_(vec4<f32>(_e516, _e119, _e550.z, f32()));
    let _e574: vec4<f32> = sampleTexture_1_(vec4<f32>(_e516, _e125, _e558.z, _e64));
    let _e582: vec4<f32> = sampleTexture_1_(vec4<f32>(_e516, _e137, _e566.z, f32()));
    let _e590: vec4<f32> = sampleTexture_1_(vec4<f32>(_e516, _e143, _e574.z, _e64));
    let _e594: f32 = fma(_e72, 3f, _e60);
    let _e599: vec4<f32> = sampleTexture_1_(vec4<f32>(_e594, _e76, _e582.z, f32()));
    let _e607: vec4<f32> = sampleTexture_1_(vec4<f32>(_e594, _e75, _e590.z, _e64));
    let _e615: vec4<f32> = sampleTexture_1_(vec4<f32>(_e594, _e82, _e599.z, f32()));
    let _e623: vec4<f32> = sampleTexture_1_(vec4<f32>(_e594, _e102, _e607.z, _e64));
    let _e631: vec4<f32> = sampleTexture_1_(vec4<f32>(_e594, _e61, _e615.z, f32()));
    let _e639: vec4<f32> = sampleTexture_1_(vec4<f32>(_e594, _e119, _e623.z, _e64));
    let _e647: vec4<f32> = sampleTexture_1_(vec4<f32>(_e594, _e125, _e631.z, f32()));
    let _e655: vec4<f32> = sampleTexture_1_(vec4<f32>(_e594, _e137, _e639.z, _e64));
    let _e662: f32 = fma(_e72, 4f, _e60);
    let _e664: vec4<f32> = sampleTexture_1_(vec4<f32>(_e594, _e143, _e647.z, f32()));
    let _e672: vec4<f32> = sampleTexture_1_(vec4<f32>(_e662, _e76, _e655.z, _e64));
    let _e680: vec4<f32> = sampleTexture_1_(vec4<f32>(_e662, _e75, _e664.z, f32()));
    let _e688: vec4<f32> = sampleTexture_1_(vec4<f32>(_e662, _e82, _e672.z, _e64));
    let _e696: vec4<f32> = sampleTexture_1_(vec4<f32>(_e662, _e102, _e680.z, f32()));
    let _e704: vec4<f32> = sampleTexture_1_(vec4<f32>(_e662, _e61, _e688.z, _e64));
    let _e712: vec4<f32> = sampleTexture_1_(vec4<f32>(_e662, _e119, _e696.z, f32()));
    let _e717: vec4<f32> = sampleTexture_1_(vec4<f32>(_e662, _e125, _e704.z, _e64));
    let _e728: vec4<f32> = sampleTexture_1_(vec4<f32>(_e662, _e137, _e717.z, _e64));
    let _e736: vec4<f32> = sampleTexture_1_(vec4<f32>(_e662, _e143, _e72, _e73));
    let _e745: vec4<f32> = cF.m[0u];
    let _e747: f32 = mul_legacy_f32_(fma(_e736.x, 0.00072873f, fma(_e728.x, 0.00174817f, fma(_e717.x, 0.00326599f, fma(_e712.x, 0.00475201f, fma(_e704.x, 0.0060052f, fma(_e696.x, 0.00475201f, fma(_e688.x, 0.00326599f, fma(_e680.x, 0.00174817f, fma(_e672.x, 0.00072873f, fma(_e664.x, 0.00174817f, fma(_e655.x, 0.00419373f, fma(_e647.x, 0.00783487f, fma(_e639.x, 0.01139972f, fma(_e631.x, 0.01440603f, fma(_e623.x, 0.01139972f, fma(_e615.x, 0.00783487f, fma(_e607.x, 0.00419373f, fma(_e599.x, 0.00174817f, fma(_e590.x, 0.00326599f, fma(_e582.x, 0.00783487f, fma(_e574.x, 0.01463737f, fma(_e566.x, 0.02129735f, fma(_e558.x, 0.02691384f, fma(_e550.x, 0.02129735f, fma(_e542.x, 0.01463737f, fma(_e534.x, 0.00783487f, fma(_e526.x, 0.00326599f, fma(_e518.x, 0.00475201f, fma(_e509.x, 0.01139972f, fma(_e501.x, 0.02129735f, fma(_e493.x, 0.03098762f, fma(_e485.x, 0.0391596f, fma(_e475.x, 0.03098762f, fma(_e467.x, 0.02129735f, fma(_e459.x, 0.01139972f, fma(_e451.x, 0.00475201f, fma(_e442.x, 0.0060052f, fma(_e434.x, 0.01440603f, fma(_e426.x, 0.02691384f, fma(_e418.x, 0.0391596f, fma(_e410.x, 0.04948667f, fma(_e402.x, 0.0391596f, fma(_e394.x, 0.02691384f, fma(_e386.x, 0.01440603f, fma(_e378.x, 0.0060052f, fma(_e370.x, 0.00475201f, fma(_e362.x, 0.01139972f, fma(_e354.x, 0.02129735f, fma(_e346.x, 0.03098762f, fma(_e338.x, 0.0391596f, fma(_e330.x, 0.03098762f, fma(_e319.x, 0.02129735f, fma(_e311.x, 0.01139972f, fma(_e303.x, 0.00475201f, fma(_e294.x, 0.00326599f, fma(_e286.x, 0.00783487f, fma(_e278.x, 0.01463737f, fma(_e270.x, 0.02129735f, fma(_e262.x, 0.02691384f, fma(_e254.x, 0.02129735f, fma(_e246.x, 0.01463737f, fma(_e238.x, 0.00783487f, fma(_e230.x, 0.00326599f, fma(_e221.x, 0.00174817f, fma(_e213.x, 0.00419373f, fma(_e205.x, 0.00783487f, fma(_e197.x, 0.01139972f, fma(_e189.x, 0.01440603f, fma(_e181.x, 0.01139972f, fma(_e173.x, 0.00783487f, fma(_e165.x, 0.00419373f, fma(_e157.x, 0.00174817f, fma(_e148.x, 0.00072873f, fma(_e139.x, 0.00174817f, fma(_e130.x, 0.00326599f, fma(_e121.x, 0.00475201f, fma(_e112.x, 0.0060052f, fma(_e104.x, 0.00475201f, fma(_e95.x, 0.00326599f, fma(_e84.x, 0.00072873f, (_e78.x * 0.00174817f))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))), _e745.z);
    let _e748: f32 = mul_legacy_f32_(fma(_e736.y, 0.00072873f, fma(_e728.y, 0.00174817f, fma(_e717.y, 0.00326599f, fma(_e712.y, 0.00475201f, fma(_e704.y, 0.0060052f, fma(_e696.y, 0.00475201f, fma(_e688.y, 0.00326599f, fma(_e680.y, 0.00174817f, fma(_e672.y, 0.00072873f, fma(_e664.y, 0.00174817f, fma(_e655.y, 0.00419373f, fma(_e647.y, 0.00783487f, fma(_e639.y, 0.01139972f, fma(_e631.y, 0.01440603f, fma(_e623.y, 0.01139972f, fma(_e615.y, 0.00783487f, fma(_e607.y, 0.00419373f, fma(_e599.y, 0.00174817f, fma(_e590.y, 0.00326599f, fma(_e582.y, 0.00783487f, fma(_e574.y, 0.01463737f, fma(_e566.y, 0.02129735f, fma(_e558.y, 0.02691384f, fma(_e550.y, 0.02129735f, fma(_e542.y, 0.01463737f, fma(_e534.y, 0.00783487f, fma(_e526.y, 0.00326599f, fma(_e518.y, 0.00475201f, fma(_e509.y, 0.01139972f, fma(_e501.y, 0.02129735f, fma(_e493.y, 0.03098762f, fma(_e485.y, 0.0391596f, fma(_e475.y, 0.03098762f, fma(_e467.y, 0.02129735f, fma(_e459.y, 0.01139972f, fma(_e451.y, 0.00475201f, fma(_e442.y, 0.0060052f, fma(_e434.y, 0.01440603f, fma(_e426.y, 0.02691384f, fma(_e418.y, 0.0391596f, fma(_e410.y, 0.04948667f, fma(_e402.y, 0.0391596f, fma(_e394.y, 0.02691384f, fma(_e386.y, 0.01440603f, fma(_e378.y, 0.0060052f, fma(_e370.y, 0.00475201f, fma(_e362.y, 0.01139972f, fma(_e354.y, 0.02129735f, fma(_e346.y, 0.03098762f, fma(_e338.y, 0.0391596f, fma(_e330.y, 0.03098762f, fma(_e319.y, 0.02129735f, fma(_e311.y, 0.01139972f, fma(_e303.y, 0.00475201f, fma(_e294.y, 0.00326599f, fma(_e286.y, 0.00783487f, fma(_e278.y, 0.01463737f, fma(_e270.y, 0.02129735f, fma(_e262.y, 0.02691384f, fma(_e254.y, 0.02129735f, fma(_e246.y, 0.01463737f, fma(_e238.y, 0.00783487f, fma(_e230.y, 0.00326599f, fma(_e221.y, 0.00174817f, fma(_e213.y, 0.00419373f, fma(_e205.y, 0.00783487f, fma(_e197.y, 0.01139972f, fma(_e189.y, 0.01440603f, fma(_e181.y, 0.01139972f, fma(_e173.y, 0.00783487f, fma(_e165.y, 0.00419373f, fma(_e157.y, 0.00174817f, fma(_e148.y, 0.00072873f, fma(_e139.y, 0.00174817f, fma(_e130.y, 0.00326599f, fma(_e121.y, 0.00475201f, fma(_e112.y, 0.0060052f, fma(_e104.y, 0.00475201f, fma(_e95.y, 0.00326599f, fma(_e84.y, 0.00072873f, (_e78.y * 0.00174817f))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))), _e745.z);
    let _e749: f32 = mul_legacy_f32_(fma(_e736.z, 0.00072873f, fma(_e728.z, 0.00174817f, fma(_e717.z, 0.00326599f, fma(_e712.z, 0.00475201f, fma(_e704.z, 0.0060052f, fma(_e696.z, 0.00475201f, fma(_e688.z, 0.00326599f, fma(_e680.z, 0.00174817f, fma(_e672.z, 0.00072873f, fma(_e664.z, 0.00174817f, fma(_e655.z, 0.00419373f, fma(_e647.z, 0.00783487f, fma(_e639.z, 0.01139972f, fma(_e631.z, 0.01440603f, fma(_e623.z, 0.01139972f, fma(_e615.z, 0.00783487f, fma(_e607.z, 0.00419373f, fma(_e599.z, 0.00174817f, fma(_e590.z, 0.00326599f, fma(_e582.z, 0.00783487f, fma(_e574.z, 0.01463737f, fma(_e566.z, 0.02129735f, fma(_e558.z, 0.02691384f, fma(_e550.z, 0.02129735f, fma(_e542.z, 0.01463737f, fma(_e534.z, 0.00783487f, fma(_e526.z, 0.00326599f, fma(_e518.z, 0.00475201f, fma(_e509.z, 0.01139972f, fma(_e501.z, 0.02129735f, fma(_e493.z, 0.03098762f, _e491))))))))))))))))))))))))))))))), _e745.z);
    let _e773: bool = (f32(clamp((clamp((clamp((f16(select(1f, 0f, ((999.9f - _e747) >= 0f))) + f16(select(1f, 0f, ((999.9f - _e748) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e749) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((-1f + 999.9f) >= 0f)))), 0h, 1h)) <= 0f);
    oC0_color[0u] = select(0f, _e747, _e773);
    oC0_color[1u] = select(0f, _e748, _e773);
    oC0_color[2u] = select(0f, _e749, _e773);
    oC0_color[3u] = select(0f, 1f, _e773);
    return;
}

@fragment 
fn main(@location(9) v0_color: vec4<f32>, @location(1) v1_texcoord0_: vec4<f32>, @location(3) v2_texcoord2_: vec4<f32>) -> @location(0) vec4<f32> {
    v0_color_1 = v0_color;
    v1_texcoord0_1 = v1_texcoord0_;
    v2_texcoord2_1 = v2_texcoord2_;
    main_1();
    let _e7: vec4<f32> = oC0_color;
    return _e7;
}
