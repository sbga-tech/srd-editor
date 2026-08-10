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
    let _e43: vec4<f32> = textureSample(s0_2d, s0_, vec2<f32>(texCoord.x, texCoord.y));
    return _e43;
}

fn main_1() {
    let _e40: f32 = v1_texcoord2_1[3u];
    let _e41: f32 = (1f / _e40);
    let _e43: f32 = v1_texcoord2_1[0u];
    let _e45: f32 = v1_texcoord2_1[1u];
    let _e46: f32 = mul_legacy_f32_(_e43, _e41);
    let _e47: f32 = mul_legacy_f32_(_e45, _e41);
    let _e50: vec4<f32> = cF.m[2u];
    let _e52: f32 = (1f / _e50.y);
    let _e54: f32 = (1f / _e50.x);
    let _e56: f32 = v0_color_1[3u];
    let _e57: f32 = mul_legacy_f32_(_e56, _e54);
    let _e58: f32 = mul_legacy_f32_(_e56, _e52);
    let _e59: f32 = fma(_e57, -4f, _e46);
    let _e60: f32 = fma(_e58, -3f, _e47);
    let _e62: vec4<f32> = sampleTexture_0_(vec4<f32>(_e59, _e60, f32(), f32()));
    let _e66: f32 = fma(_e58, -4f, _e47);
    let _e71: vec4<f32> = sampleTexture_0_(vec4<f32>(_e59, _e66, _e57, _e58));
    let _e78: f32 = fma(_e58, -2f, _e47);
    let _e80: vec4<f32> = sampleTexture_0_(vec4<f32>(_e59, _e78, _e71.z, f32()));
    let _e86: f32 = fma(_e80.z, 0.00326599f, fma(_e71.z, 0.00072873f, (_e62.z * 0.00174817f)));
    let _e87: f32 = (_e47 - _e58);
    let _e89: vec4<f32> = sampleTexture_0_(vec4<f32>(_e59, _e87, _e57, _e58));
    let _e97: vec4<f32> = sampleTexture_0_(vec4<f32>(_e59, _e47, _e86, f32()));
    let _e103: f32 = fma(_e97.z, 0.0060052f, fma(_e89.z, 0.00475201f, _e86));
    let _e104: f32 = (_e58 + _e47);
    let _e106: vec4<f32> = sampleTexture_0_(vec4<f32>(_e59, _e104, _e57, _e58));
    let _e113: f32 = fma(_e58, 2f, _e47);
    let _e115: vec4<f32> = sampleTexture_0_(vec4<f32>(_e59, _e113, _e103, f32()));
    let _e121: f32 = fma(_e115.z, 0.00326599f, fma(_e106.z, 0.00475201f, _e103));
    let _e122: f32 = fma(_e58, 3f, _e47);
    let _e124: vec4<f32> = sampleTexture_0_(vec4<f32>(_e59, _e122, _e57, _e58));
    let _e131: f32 = fma(_e58, 4f, _e47);
    let _e133: vec4<f32> = sampleTexture_0_(vec4<f32>(_e59, _e131, _e121, f32()));
    let _e139: f32 = fma(_e133.z, 0.00072873f, fma(_e124.z, 0.00174817f, _e121));
    let _e140: f32 = fma(_e57, -3f, _e46);
    let _e142: vec4<f32> = sampleTexture_0_(vec4<f32>(_e140, _e66, _e57, _e58));
    let _e150: vec4<f32> = sampleTexture_0_(vec4<f32>(_e140, _e60, _e139, f32()));
    let _e156: f32 = fma(_e150.z, 0.00419373f, fma(_e142.z, 0.00174817f, _e139));
    let _e158: vec4<f32> = sampleTexture_0_(vec4<f32>(_e140, _e78, _e57, _e58));
    let _e166: vec4<f32> = sampleTexture_0_(vec4<f32>(_e140, _e87, _e156, f32()));
    let _e172: f32 = fma(_e166.z, 0.01139972f, fma(_e158.z, 0.00783487f, _e156));
    let _e174: vec4<f32> = sampleTexture_0_(vec4<f32>(_e140, _e47, _e57, _e58));
    let _e182: vec4<f32> = sampleTexture_0_(vec4<f32>(_e140, _e104, _e172, f32()));
    let _e188: f32 = fma(_e182.z, 0.01139972f, fma(_e174.z, 0.01440603f, _e172));
    let _e190: vec4<f32> = sampleTexture_0_(vec4<f32>(_e140, _e113, _e57, _e58));
    let _e198: vec4<f32> = sampleTexture_0_(vec4<f32>(_e140, _e122, _e188, f32()));
    let _e204: f32 = fma(_e198.z, 0.00419373f, fma(_e190.z, 0.00783487f, _e188));
    let _e206: vec4<f32> = sampleTexture_0_(vec4<f32>(_e140, _e131, _e57, _e58));
    let _e213: f32 = fma(_e57, -2f, _e46);
    let _e215: vec4<f32> = sampleTexture_0_(vec4<f32>(_e213, _e66, _e204, f32()));
    let _e221: f32 = fma(_e215.z, 0.00326599f, fma(_e206.z, 0.00174817f, _e204));
    let _e223: vec4<f32> = sampleTexture_0_(vec4<f32>(_e213, _e60, _e57, _e58));
    let _e231: vec4<f32> = sampleTexture_0_(vec4<f32>(_e213, _e78, _e221, f32()));
    let _e237: f32 = fma(_e231.z, 0.01463737f, fma(_e223.z, 0.00783487f, _e221));
    let _e239: vec4<f32> = sampleTexture_0_(vec4<f32>(_e213, _e87, _e57, _e58));
    let _e247: vec4<f32> = sampleTexture_0_(vec4<f32>(_e213, _e47, _e237, f32()));
    let _e253: f32 = fma(_e247.z, 0.02691384f, fma(_e239.z, 0.02129735f, _e237));
    let _e255: vec4<f32> = sampleTexture_0_(vec4<f32>(_e213, _e104, _e57, _e58));
    let _e263: vec4<f32> = sampleTexture_0_(vec4<f32>(_e213, _e113, _e253, f32()));
    let _e269: f32 = fma(_e263.z, 0.01463737f, fma(_e255.z, 0.02129735f, _e253));
    let _e271: vec4<f32> = sampleTexture_0_(vec4<f32>(_e213, _e122, _e57, _e58));
    let _e279: vec4<f32> = sampleTexture_0_(vec4<f32>(_e213, _e131, _e269, f32()));
    let _e285: f32 = fma(_e279.z, 0.00326599f, fma(_e271.z, 0.00783487f, _e269));
    let _e286: f32 = (_e46 - _e57);
    let _e288: vec4<f32> = sampleTexture_0_(vec4<f32>(_e286, _e66, _e57, _e58));
    let _e296: vec4<f32> = sampleTexture_0_(vec4<f32>(_e286, _e60, _e285, f32()));
    let _e302: f32 = fma(_e296.z, 0.01139972f, fma(_e288.z, 0.00475201f, _e285));
    let _e304: vec4<f32> = sampleTexture_0_(vec4<f32>(_e286, _e78, _e57, _e58));
    let _e311: f32 = -(_e56);
    let _e312: f32 = mad_legacy_f32_(_e311, _e54, _e46);
    let _e313: f32 = mad_legacy_f32_(_e311, _e52, _e47);
    let _e315: vec4<f32> = sampleTexture_0_(vec4<f32>(_e312, _e313, _e302, f32()));
    let _e321: f32 = fma(_e315.z, 0.03098762f, fma(_e304.z, 0.02129735f, _e302));
    let _e323: vec4<f32> = sampleTexture_0_(vec4<f32>(_e286, _e47, _e57, _e58));
    let _e331: vec4<f32> = sampleTexture_0_(vec4<f32>(_e286, _e104, _e321, f32()));
    let _e337: f32 = fma(_e331.z, 0.03098762f, fma(_e323.z, 0.0391596f, _e321));
    let _e339: vec4<f32> = sampleTexture_0_(vec4<f32>(_e286, _e113, _e57, _e58));
    let _e347: vec4<f32> = sampleTexture_0_(vec4<f32>(_e286, _e122, _e337, f32()));
    let _e353: f32 = fma(_e347.z, 0.01139972f, fma(_e339.z, 0.02129735f, _e337));
    let _e355: vec4<f32> = sampleTexture_0_(vec4<f32>(_e286, _e131, _e57, _e58));
    let _e363: vec4<f32> = sampleTexture_0_(vec4<f32>(_e46, _e66, _e353, f32()));
    let _e369: f32 = fma(_e363.z, 0.0060052f, fma(_e355.z, 0.00475201f, _e353));
    let _e371: vec4<f32> = sampleTexture_0_(vec4<f32>(_e46, _e60, _e57, _e58));
    let _e379: vec4<f32> = sampleTexture_0_(vec4<f32>(_e46, _e78, _e369, f32()));
    let _e387: vec4<f32> = sampleTexture_0_(vec4<f32>(_e46, _e87, _e57, _e58));
    let _e395: vec4<f32> = sampleTexture_0_(vec4<f32>(_e46, _e47, _e54, _e52));
    let _e403: vec4<f32> = sampleTexture_0_(vec4<f32>(_e46, _e104, _e395.z, f32()));
    let _e409: f32 = fma(_e403.z, 0.0391596f, fma(_e395.z, 0.04948667f, fma(_e387.z, 0.0391596f, fma(_e379.z, 0.02691384f, fma(_e371.z, 0.01440603f, _e369)))));
    let _e411: vec4<f32> = sampleTexture_0_(vec4<f32>(_e46, _e113, _e57, _e58));
    let _e419: vec4<f32> = sampleTexture_0_(vec4<f32>(_e46, _e122, _e409, f32()));
    let _e425: f32 = fma(_e419.z, 0.01440603f, fma(_e411.z, 0.02691384f, _e409));
    let _e427: vec4<f32> = sampleTexture_0_(vec4<f32>(_e46, _e131, _e57, _e58));
    let _e434: f32 = (_e57 + _e46);
    let _e436: vec4<f32> = sampleTexture_0_(vec4<f32>(_e434, _e66, _e425, f32()));
    let _e442: f32 = fma(_e436.z, 0.00475201f, fma(_e427.z, 0.0060052f, _e425));
    let _e444: vec4<f32> = sampleTexture_0_(vec4<f32>(_e434, _e60, _e57, _e58));
    let _e452: vec4<f32> = sampleTexture_0_(vec4<f32>(_e434, _e78, _e442, f32()));
    let _e460: vec4<f32> = sampleTexture_0_(vec4<f32>(_e434, _e87, _e57, _e58));
    let _e468: vec4<f32> = sampleTexture_0_(vec4<f32>(_e434, _e47, _e460.z, f32()));
    let _e474: f32 = fma(_e468.z, 0.0391596f, fma(_e460.z, 0.03098762f, fma(_e452.z, 0.02129735f, fma(_e444.z, 0.01139972f, _e442))));
    let _e475: f32 = mad_legacy_f32_(_e56, _e54, _e46);
    let _e476: f32 = mad_legacy_f32_(_e56, _e52, _e47);
    let _e478: vec4<f32> = sampleTexture_0_(vec4<f32>(_e475, _e476, _e57, _e58));
    let _e486: vec4<f32> = sampleTexture_0_(vec4<f32>(_e434, _e113, _e474, f32()));
    let _e492: f32 = fma(_e486.z, 0.02129735f, fma(_e478.z, 0.03098762f, _e474));
    let _e494: vec4<f32> = sampleTexture_0_(vec4<f32>(_e434, _e122, _e57, _e58));
    let _e502: vec4<f32> = sampleTexture_0_(vec4<f32>(_e434, _e131, _e492, f32()));
    let _e508: f32 = fma(_e502.z, 0.00475201f, fma(_e494.z, 0.01139972f, _e492));
    let _e509: f32 = fma(_e57, 2f, _e46);
    let _e511: vec4<f32> = sampleTexture_0_(vec4<f32>(_e509, _e66, _e57, _e58));
    let _e519: vec4<f32> = sampleTexture_0_(vec4<f32>(_e509, _e60, _e508, f32()));
    let _e525: f32 = fma(_e519.z, 0.00783487f, fma(_e511.z, 0.00326599f, _e508));
    let _e527: vec4<f32> = sampleTexture_0_(vec4<f32>(_e509, _e78, _e57, _e58));
    let _e535: vec4<f32> = sampleTexture_0_(vec4<f32>(_e509, _e87, _e525, f32()));
    let _e541: f32 = fma(_e535.z, 0.02129735f, fma(_e527.z, 0.01463737f, _e525));
    let _e543: vec4<f32> = sampleTexture_0_(vec4<f32>(_e509, _e47, _e57, _e58));
    let _e551: vec4<f32> = sampleTexture_0_(vec4<f32>(_e509, _e104, _e541, f32()));
    let _e557: f32 = fma(_e551.z, 0.02129735f, fma(_e543.z, 0.02691384f, _e541));
    let _e559: vec4<f32> = sampleTexture_0_(vec4<f32>(_e509, _e113, _e57, _e58));
    let _e567: vec4<f32> = sampleTexture_0_(vec4<f32>(_e509, _e122, _e557, f32()));
    let _e573: f32 = fma(_e567.z, 0.00783487f, fma(_e559.z, 0.01463737f, _e557));
    let _e575: vec4<f32> = sampleTexture_0_(vec4<f32>(_e509, _e131, _e57, _e58));
    let _e582: f32 = fma(_e57, 3f, _e46);
    let _e584: vec4<f32> = sampleTexture_0_(vec4<f32>(_e582, _e66, _e573, f32()));
    let _e590: f32 = fma(_e584.z, 0.00174817f, fma(_e575.z, 0.00326599f, _e573));
    let _e592: vec4<f32> = sampleTexture_0_(vec4<f32>(_e582, _e60, _e57, _e58));
    let _e600: vec4<f32> = sampleTexture_0_(vec4<f32>(_e582, _e78, _e590, f32()));
    let _e606: f32 = fma(_e600.z, 0.00783487f, fma(_e592.z, 0.00419373f, _e590));
    let _e608: vec4<f32> = sampleTexture_0_(vec4<f32>(_e582, _e87, _e57, _e58));
    let _e616: vec4<f32> = sampleTexture_0_(vec4<f32>(_e582, _e47, _e606, f32()));
    let _e622: f32 = fma(_e616.z, 0.01440603f, fma(_e608.z, 0.01139972f, _e606));
    let _e624: vec4<f32> = sampleTexture_0_(vec4<f32>(_e582, _e104, _e57, _e58));
    let _e632: vec4<f32> = sampleTexture_0_(vec4<f32>(_e582, _e113, _e622, f32()));
    let _e638: f32 = fma(_e632.z, 0.00783487f, fma(_e624.z, 0.01139972f, _e622));
    let _e640: vec4<f32> = sampleTexture_0_(vec4<f32>(_e582, _e122, _e57, _e58));
    let _e648: vec4<f32> = sampleTexture_0_(vec4<f32>(_e582, _e131, _e638, f32()));
    let _e654: f32 = fma(_e648.z, 0.00174817f, fma(_e640.z, 0.00419373f, _e638));
    let _e655: f32 = fma(_e57, 4f, _e46);
    let _e657: vec4<f32> = sampleTexture_0_(vec4<f32>(_e655, _e66, _e57, _e58));
    let _e665: vec4<f32> = sampleTexture_0_(vec4<f32>(_e655, _e60, _e654, f32()));
    let _e671: f32 = fma(_e665.z, 0.00174817f, fma(_e657.z, 0.00072873f, _e654));
    let _e673: vec4<f32> = sampleTexture_0_(vec4<f32>(_e655, _e78, _e57, _e58));
    let _e681: vec4<f32> = sampleTexture_0_(vec4<f32>(_e655, _e87, _e671, f32()));
    let _e687: f32 = fma(_e681.z, 0.00475201f, fma(_e673.z, 0.00326599f, _e671));
    let _e689: vec4<f32> = sampleTexture_0_(vec4<f32>(_e655, _e47, _e57, _e58));
    let _e697: vec4<f32> = sampleTexture_0_(vec4<f32>(_e655, _e104, _e687, f32()));
    let _e705: vec4<f32> = sampleTexture_0_(vec4<f32>(_e655, _e113, _e57, _e58));
    let _e713: vec4<f32> = sampleTexture_0_(vec4<f32>(_e655, _e122, _e705.z, f32()));
    let _e721: vec4<f32> = sampleTexture_0_(vec4<f32>(_e655, _e131, _e54, _e52));
    let _e730: vec4<f32> = cF.m[0u];
    let _e732: f32 = mul_legacy_f32_(fma(_e721.x, 0.00072873f, fma(_e713.x, 0.00174817f, fma(_e705.x, 0.00326599f, fma(_e697.x, 0.00475201f, fma(_e689.x, 0.0060052f, fma(_e681.x, 0.00475201f, fma(_e673.x, 0.00326599f, fma(_e665.x, 0.00174817f, fma(_e657.x, 0.00072873f, fma(_e648.x, 0.00174817f, fma(_e640.x, 0.00419373f, fma(_e632.x, 0.00783487f, fma(_e624.x, 0.01139972f, fma(_e616.x, 0.01440603f, fma(_e608.x, 0.01139972f, fma(_e600.x, 0.00783487f, fma(_e592.x, 0.00419373f, fma(_e584.x, 0.00174817f, fma(_e575.x, 0.00326599f, fma(_e567.x, 0.00783487f, fma(_e559.x, 0.01463737f, fma(_e551.x, 0.02129735f, fma(_e543.x, 0.02691384f, fma(_e535.x, 0.02129735f, fma(_e527.x, 0.01463737f, fma(_e519.x, 0.00783487f, fma(_e511.x, 0.00326599f, fma(_e502.x, 0.00475201f, fma(_e494.x, 0.01139972f, fma(_e486.x, 0.02129735f, fma(_e478.x, 0.03098762f, fma(_e468.x, 0.0391596f, fma(_e460.x, 0.03098762f, fma(_e452.x, 0.02129735f, fma(_e444.x, 0.01139972f, fma(_e436.x, 0.00475201f, fma(_e427.x, 0.0060052f, fma(_e419.x, 0.01440603f, fma(_e411.x, 0.02691384f, fma(_e403.x, 0.0391596f, fma(_e395.x, 0.04948667f, fma(_e387.x, 0.0391596f, fma(_e379.x, 0.02691384f, fma(_e371.x, 0.01440603f, fma(_e363.x, 0.0060052f, fma(_e355.x, 0.00475201f, fma(_e347.x, 0.01139972f, fma(_e339.x, 0.02129735f, fma(_e331.x, 0.03098762f, fma(_e323.x, 0.0391596f, fma(_e315.x, 0.03098762f, fma(_e304.x, 0.02129735f, fma(_e296.x, 0.01139972f, fma(_e288.x, 0.00475201f, fma(_e279.x, 0.00326599f, fma(_e271.x, 0.00783487f, fma(_e263.x, 0.01463737f, fma(_e255.x, 0.02129735f, fma(_e247.x, 0.02691384f, fma(_e239.x, 0.02129735f, fma(_e231.x, 0.01463737f, fma(_e223.x, 0.00783487f, fma(_e215.x, 0.00326599f, fma(_e206.x, 0.00174817f, fma(_e198.x, 0.00419373f, fma(_e190.x, 0.00783487f, fma(_e182.x, 0.01139972f, fma(_e174.x, 0.01440603f, fma(_e166.x, 0.01139972f, fma(_e158.x, 0.00783487f, fma(_e150.x, 0.00419373f, fma(_e142.x, 0.00174817f, fma(_e133.x, 0.00072873f, fma(_e124.x, 0.00174817f, fma(_e115.x, 0.00326599f, fma(_e106.x, 0.00475201f, fma(_e97.x, 0.0060052f, fma(_e89.x, 0.00475201f, fma(_e80.x, 0.00326599f, fma(_e71.x, 0.00072873f, (_e62.x * 0.00174817f))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))), _e730.z);
    let _e733: f32 = mul_legacy_f32_(fma(_e721.y, 0.00072873f, fma(_e713.y, 0.00174817f, fma(_e705.y, 0.00326599f, fma(_e697.y, 0.00475201f, fma(_e689.y, 0.0060052f, fma(_e681.y, 0.00475201f, fma(_e673.y, 0.00326599f, fma(_e665.y, 0.00174817f, fma(_e657.y, 0.00072873f, fma(_e648.y, 0.00174817f, fma(_e640.y, 0.00419373f, fma(_e632.y, 0.00783487f, fma(_e624.y, 0.01139972f, fma(_e616.y, 0.01440603f, fma(_e608.y, 0.01139972f, fma(_e600.y, 0.00783487f, fma(_e592.y, 0.00419373f, fma(_e584.y, 0.00174817f, fma(_e575.y, 0.00326599f, fma(_e567.y, 0.00783487f, fma(_e559.y, 0.01463737f, fma(_e551.y, 0.02129735f, fma(_e543.y, 0.02691384f, fma(_e535.y, 0.02129735f, fma(_e527.y, 0.01463737f, fma(_e519.y, 0.00783487f, fma(_e511.y, 0.00326599f, fma(_e502.y, 0.00475201f, fma(_e494.y, 0.01139972f, fma(_e486.y, 0.02129735f, fma(_e478.y, 0.03098762f, fma(_e468.y, 0.0391596f, fma(_e460.y, 0.03098762f, fma(_e452.y, 0.02129735f, fma(_e444.y, 0.01139972f, fma(_e436.y, 0.00475201f, fma(_e427.y, 0.0060052f, fma(_e419.y, 0.01440603f, fma(_e411.y, 0.02691384f, fma(_e403.y, 0.0391596f, fma(_e395.y, 0.04948667f, fma(_e387.y, 0.0391596f, fma(_e379.y, 0.02691384f, fma(_e371.y, 0.01440603f, fma(_e363.y, 0.0060052f, fma(_e355.y, 0.00475201f, fma(_e347.y, 0.01139972f, fma(_e339.y, 0.02129735f, fma(_e331.y, 0.03098762f, fma(_e323.y, 0.0391596f, fma(_e315.y, 0.03098762f, fma(_e304.y, 0.02129735f, fma(_e296.y, 0.01139972f, fma(_e288.y, 0.00475201f, fma(_e279.y, 0.00326599f, fma(_e271.y, 0.00783487f, fma(_e263.y, 0.01463737f, fma(_e255.y, 0.02129735f, fma(_e247.y, 0.02691384f, fma(_e239.y, 0.02129735f, fma(_e231.y, 0.01463737f, fma(_e223.y, 0.00783487f, fma(_e215.y, 0.00326599f, fma(_e206.y, 0.00174817f, fma(_e198.y, 0.00419373f, fma(_e190.y, 0.00783487f, fma(_e182.y, 0.01139972f, fma(_e174.y, 0.01440603f, fma(_e166.y, 0.01139972f, fma(_e158.y, 0.00783487f, fma(_e150.y, 0.00419373f, fma(_e142.y, 0.00174817f, fma(_e133.y, 0.00072873f, fma(_e124.y, 0.00174817f, fma(_e115.y, 0.00326599f, fma(_e106.y, 0.00475201f, fma(_e97.y, 0.0060052f, fma(_e89.y, 0.00475201f, fma(_e80.y, 0.00326599f, fma(_e71.y, 0.00072873f, (_e62.y * 0.00174817f))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))), _e730.z);
    let _e734: f32 = mul_legacy_f32_(fma(_e721.z, 0.00072873f, fma(_e713.z, 0.00174817f, fma(_e705.z, 0.00326599f, fma(_e697.z, 0.00475201f, fma(_e689.z, 0.0060052f, _e687))))), _e730.z);
    let _e758: bool = (f32(clamp((clamp((clamp((f16(select(1f, 0f, ((999.9f - _e732) >= 0f))) + f16(select(1f, 0f, ((999.9f - _e733) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((999.9f - _e734) >= 0f)))), 0h, 1h) + f16(select(1f, 0f, ((-1f + 999.9f) >= 0f)))), 0h, 1h)) <= 0f);
    oC0_color[0u] = select(0f, _e732, _e758);
    oC0_color[1u] = select(0f, _e733, _e758);
    oC0_color[2u] = select(0f, _e734, _e758);
    oC0_color[3u] = select(0f, 1f, _e758);
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
