// Canonical Ceylon SimpleShader program. Pipeline overrides specialize the
// normalized profile; per-draw uniforms retain only numeric shader inputs.
override SIMPLE_2D_TRANSFORM: u32 = 0u;
override SIMPLE_TEXTURE_COUNT: u32 = 0u;
override SIMPLE_MULTI_TEXTURE_MODE: u32 = 0u;
override SIMPLE_BLEND_MODE: u32 = 0u;

struct DrawUniforms {
    mtx_world: mat4x4<f32>,
    mtx_prj_view: mat4x4<f32>,
    // xy: original screenParam half-size, zw: composition target size.
    screen_target: vec4<f32>,
    // xy: sampler LOD bias, z: alpha reference, w: alpha comparison.
    params: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> draw: DrawUniforms;
@group(0) @binding(1)
var texture0: texture_2d<f32>;
@group(0) @binding(2)
var sampler0: sampler;
@group(0) @binding(3)
var texture1: texture_2d<f32>;
@group(0) @binding(4)
var sampler1: sampler;
@group(0) @binding(5)
var texture_target_color: texture_2d<f32>;
@group(0) @binding(6)
var sampler_target_color: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) primary_color: vec4<f32>,
    @location(1) secondary_color: vec4<f32>,
    @location(2) uv0: vec2<f32>,
    @location(3) uv1: vec2<f32>,
}

fn simple_position(position: vec3<f32>) -> vec4<f32> {
    // Preserve recovered Cg operation order rather than uploading a combined matrix.
    let world_position = draw.mtx_world * vec4(position, 1.0);
    if SIMPLE_2D_TRANSFORM != 0u {
        return vec4(
            world_position.x / draw.screen_target.x - 1.0,
            1.0 - world_position.y / draw.screen_target.y,
            0.0,
            1.0,
        );
    }
    // D3D9 rasterizes at integer pixel centers; WebGPU uses half-integer centers.
    let projected = draw.mtx_prj_view * world_position;
    return vec4(
        projected.x + projected.w / draw.screen_target.z,
        projected.y - projected.w / draw.screen_target.w,
        projected.zw,
    );
}

@vertex
fn vs_format_14(
    @location(0) position: vec3<f32>,
    @location(1) primary_color: vec4<f32>,
    @location(2) secondary_color: vec4<f32>,
    @location(3) uv0: vec2<f32>,
    @location(4) uv1: vec2<f32>,
) -> VertexOutput {
    var output: VertexOutput;
    output.position = simple_position(position);
    // Independent SRD vertices carry semantic RGBA components.
    output.primary_color = primary_color;
    output.secondary_color = secondary_color;
    output.uv0 = uv0;
    output.uv1 = uv1;
    return output;
}

@vertex
fn vs_format_13(
    @location(0) position: vec3<f32>,
    @location(1) primary_color: vec4<f32>,
    @location(2) secondary_color: vec4<f32>,
    @location(3) uv0: vec2<f32>,
) -> VertexOutput {
    var output: VertexOutput;
    output.position = simple_position(position);
    output.primary_color = primary_color.zyxw;
    output.secondary_color = secondary_color.zyxw;
    output.uv0 = uv0;
    output.uv1 = uv0;
    return output;
}

fn combine_textures(destination: vec4<f32>, source: vec4<f32>, mode: u32) -> vec4<f32> {
    switch mode {
        case 0u: { return destination; }
        case 1u: { return destination; }
        case 2u: { return source; }
        case 3u: { return mix(destination, source, source.a); }
        case 4u: { return destination + source; }
        case 5u: { return destination - source; }
        case 6u: {
            return vec4(destination.rgb * source.rgb, source.a + destination.a * 0.00001);
        }
        case 7u: { return vec4(1.0) - destination; }
        case 8u: { return destination; }
        case 9u: { return vec4(source.rgb, source.a + destination.a * 0.00001); }
        case 10u: {
            return vec4(mix(destination.rgb, source.rgb, source.a), destination.a);
        }
        case 11u: { return vec4(destination.rgb, source.r); }
        case 12u: { return vec4(destination.rgb, destination.a * source.r); }
        default: { return destination; }
    }
}

fn alpha_test_passes(alpha: f32, comparison: u32, reference: u32) -> bool {
    let quantized = u32(round(clamp(alpha, 0.0, 1.0) * 255.0));
    switch comparison {
        case 0u: { return false; }
        case 1u: { return quantized < reference; }
        case 2u: { return quantized == reference; }
        case 3u: { return quantized <= reference; }
        case 4u: { return quantized > reference; }
        case 5u: { return quantized != reference; }
        case 6u: { return quantized >= reference; }
        default: { return true; }
    }
}

fn rgb_to_hsv(rgb: vec3<f32>) -> vec3<f32> {
    var hsv = vec3<f32>(0.0);
    let maximum = max(rgb.r, max(rgb.g, rgb.b));
    let minimum = min(rgb.r, min(rgb.g, rgb.b));
    let delta = maximum - minimum;
    hsv.z = maximum;
    if hsv.z != 0.0 {
        hsv.y = delta / maximum;
    }
    if rgb.r == maximum {
        hsv.x = (rgb.g - rgb.b) / delta;
    } else if rgb.g == maximum {
        hsv.x = 2.0 + (rgb.b - rgb.r) / delta;
    } else {
        hsv.x = 4.0 + (rgb.r - rgb.g) / delta;
    }
    hsv.x /= 6.0;
    if hsv.x < 0.0 {
        hsv.x += 1.0;
    }
    return hsv;
}

fn hsv_to_rgb(value: vec3<f32>) -> vec3<f32> {
    var hsv = value;
    if hsv.y == 0.0 {
        return vec3<f32>(hsv.z);
    }
    if hsv.x >= 1.0 {
        hsv.x -= 1.0;
    }
    hsv.x *= 6.0;
    let sector = floor(hsv.x);
    let fraction = hsv.x - sector;
    let minimum = hsv.z * (1.0 - hsv.y);
    let descending = hsv.z * (1.0 - hsv.y * fraction);
    let ascending = hsv.z * (1.0 - hsv.y * (1.0 - fraction));
    if sector < 1.0 {
        return vec3(hsv.z, ascending, minimum);
    }
    if sector < 2.0 {
        return vec3(descending, hsv.z, minimum);
    }
    if sector < 3.0 {
        return vec3(minimum, hsv.z, ascending);
    }
    if sector < 4.0 {
        return vec3(minimum, descending, hsv.z);
    }
    if sector < 5.0 {
        return vec3(ascending, minimum, hsv.z);
    }
    return vec3(hsv.z, minimum, descending);
}

fn rgb_to_hls(rgb: vec3<f32>) -> vec3<f32> {
    var hls = vec3<f32>(0.0);
    let maximum = max(rgb.r, max(rgb.g, rgb.b));
    let minimum = min(rgb.r, min(rgb.g, rgb.b));
    let delta = maximum - minimum;
    if rgb.r == maximum {
        hls.x = (rgb.g - rgb.b) / delta;
    } else if rgb.g == maximum {
        hls.x = 2.0 + (rgb.b - rgb.r) / delta;
    } else {
        hls.x = 4.0 + (rgb.r - rgb.g) / delta;
    }
    hls.x /= 6.0;
    hls.y = (maximum + minimum) * 0.5;
    if maximum == minimum {
        hls.z = 0.0;
    } else if hls.y < 0.5 {
        hls.z = delta / (maximum + minimum);
    } else {
        hls.z = delta / (2.0 - maximum - minimum);
    }
    return hls;
}

fn hls_value(first: f32, second: f32, input_hue: f32) -> f32 {
    var hue = input_hue;
    if hue > 360.0 {
        hue -= 360.0;
    } else if hue < 0.0 {
        hue += 360.0;
    }
    if hue < 60.0 {
        return first + (second - first) * hue / 60.0;
    }
    if hue < 180.0 {
        return second;
    }
    if hue < 240.0 {
        return first + (second - first) * (240.0 - hue) / 60.0;
    }
    return first;
}

fn hls_to_rgb(hls: vec3<f32>) -> vec3<f32> {
    let second = select(
        hls.y * (1.0 - hls.z) + hls.z,
        hls.y * (1.0 + hls.z),
        hls.y < 0.5,
    );
    let first = 2.0 * hls.y - second;
    if hls.z == 0.0 {
        return vec3<f32>(hls.y);
    }
    return vec3(
        hls_value(first, second, hls.x * 360.0 + 120.0),
        hls_value(first, second, hls.x * 360.0),
        hls_value(first, second, hls.x * 360.0 - 120.0),
    );
}

fn photoshop_layer_blend(backdrop: vec4<f32>, source: vec4<f32>, mode: u32) -> vec4<f32> {
    var result = source;
    switch mode {
        case 34u: { result = max(backdrop, source); }
        case 35u: { result = min(backdrop, source); }
        case 36u: {
            result = select(
                source,
                backdrop,
                source.r + source.g + source.b < backdrop.r + backdrop.g + backdrop.b,
            );
        }
        case 37u: {
            result = select(
                source,
                backdrop,
                source.r + source.g + source.b > backdrop.r + backdrop.g + backdrop.b,
            );
        }
        case 38u: {
            let burned = select(
                vec4<f32>(1.0),
                vec4<f32>(1.0) - (vec4<f32>(1.0) - backdrop) / source,
                source > vec4<f32>(0.0),
            );
            result = select(burned, vec4<f32>(0.0), source + backdrop < vec4<f32>(1.0));
        }
        case 39u: {
            result = select(
                backdrop + source - vec4<f32>(1.0),
                vec4<f32>(0.0),
                backdrop + source < vec4<f32>(1.0),
            );
        }
        case 40u: {
            let dodged = select(
                vec4<f32>(0.0),
                backdrop / (vec4<f32>(1.0) - source),
                backdrop > vec4<f32>(0.0),
            );
            result = select(dodged, vec4<f32>(1.0), backdrop + source > vec4<f32>(1.0));
        }
        case 41u: {
            result = select(backdrop + source, vec4<f32>(1.0), backdrop + source > vec4<f32>(1.0));
        }
        case 42u: {
            result = vec4<f32>(1.0) - (vec4<f32>(1.0) - backdrop) * (vec4<f32>(1.0) - source);
        }
        case 43u: {
            result = select(
                vec4<f32>(1.0) - 2.0 * (vec4<f32>(1.0) - backdrop) * (vec4<f32>(1.0) - source),
                2.0 * backdrop * source,
                backdrop < vec4<f32>(0.5),
            );
        }
        case 44u: {
            let lower = backdrop + (backdrop - backdrop * backdrop) * (2.0 * source - vec4<f32>(1.0));
            let upper_dark = backdrop
                + (backdrop - backdrop * backdrop)
                    * (2.0 * source - vec4<f32>(1.0))
                    * (vec4<f32>(3.0) - 8.0 * backdrop);
            let upper_light = backdrop
                + (sqrt(backdrop) - backdrop) * (2.0 * source - vec4<f32>(1.0));
            let upper = select(upper_light, upper_dark, backdrop <= vec4<f32>(32.0 / 255.0));
            result = select(upper, lower, source < vec4<f32>(0.5));
        }
        case 45u: {
            result = select(
                vec4<f32>(1.0) - 2.0 * (vec4<f32>(1.0) - backdrop) * (vec4<f32>(1.0) - source),
                2.0 * backdrop * source,
                source < vec4<f32>(0.5),
            );
        }
        case 46u: {
            let lower = select(
                (backdrop - (vec4<f32>(1.0) - 2.0 * source)) / (2.0 * source),
                vec4<f32>(0.0),
                backdrop <= vec4<f32>(1.0) - 2.0 * source,
            );
            let upper = select(
                vec4<f32>(1.0),
                backdrop / (vec4<f32>(2.0) - 2.0 * source),
                backdrop < vec4<f32>(2.0) - 2.0 * source,
            );
            result = select(upper, lower, source < vec4<f32>(0.5));
        }
        case 47u: {
            let lower = select(
                2.0 * source + backdrop - vec4<f32>(1.0),
                vec4<f32>(0.0),
                backdrop < vec4<f32>(1.0) - 2.0 * source,
            );
            let upper = select(
                vec4<f32>(1.0),
                2.0 * source + backdrop - vec4<f32>(1.0),
                backdrop < vec4<f32>(2.0) - 2.0 * source,
            );
            result = select(upper, lower, source < vec4<f32>(0.5));
        }
        case 48u: {
            result = select(
                max(2.0 * source - vec4<f32>(1.0), backdrop),
                min(2.0 * source, backdrop),
                source < vec4<f32>(0.5),
            );
        }
        case 49u: { result = step(vec4<f32>(254.0 / 255.0), backdrop + source); }
        case 50u: { result = abs(backdrop - source); }
        case 51u: {
            result = abs(
                2.0 * (vec4<f32>(0.5) - backdrop) * (vec4<f32>(0.5) - source)
                    - vec4<f32>(0.5),
            );
        }
        case 52u: {
            var background_hsv = rgb_to_hsv(backdrop.rgb);
            let source_hsv = rgb_to_hsv(source.rgb);
            background_hsv.x = source_hsv.x;
            result = vec4(hsv_to_rgb(background_hsv), source.a);
        }
        case 53u: {
            var background_hls = rgb_to_hls(backdrop.rgb);
            let source_hls = rgb_to_hls(source.rgb);
            background_hls.z = source_hls.z;
            result = vec4(hls_to_rgb(background_hls), source.a);
        }
        case 54u: {
            var background_hls = rgb_to_hls(backdrop.rgb);
            let source_hls = rgb_to_hls(source.rgb);
            background_hls.x = source_hls.x;
            background_hls.z = source_hls.z;
            result = vec4(hls_to_rgb(background_hls), source.a);
        }
        case 55u: {
            var background_hsv = rgb_to_hsv(backdrop.rgb);
            let source_hsv = rgb_to_hsv(source.rgb);
            background_hsv.z = source_hsv.z;
            result = vec4(hsv_to_rgb(background_hsv), source.a);
        }
        case 56u: {
            return clamp(backdrop + source * source.a, vec4<f32>(0.0), vec4<f32>(1.0));
        }
        default: {}
    }
    return mix(backdrop, result, source.a);
}

fn gaussian_backdrop(uv: vec2<f32>, source_alpha: f32, mode: u32) -> vec4<f32> {
    let weights = array<f32, 9>(
        0.026995,
        0.064759,
        0.120985,
        0.176033,
        0.222456,
        0.176033,
        0.120985,
        0.064759,
        0.026995,
    );
    var color = vec3<f32>(0.0);
    if mode == 59u {
        for (var i = 0u; i < 18u; i += 1u) {
            for (var j = 0u; j < 18u; j += 1u) {
                let offset = (vec2(f32(i), f32(j)) - vec2<f32>(8.0))
                    * source_alpha
                    / draw.screen_target.xy;
                color += textureSample(texture_target_color, sampler_target_color, uv + offset).rgb
                    * weights[i / 2u]
                    * weights[j / 2u];
            }
        }
        color /= 4.0;
    } else {
        let radius = select(1.0, 0.5, mode == 57u) * source_alpha;
        for (var i = 0u; i < 9u; i += 1u) {
            for (var j = 0u; j < 9u; j += 1u) {
                let offset = (vec2(f32(i), f32(j)) - vec2<f32>(4.0))
                    * radius
                    / draw.screen_target.xy;
                color += textureSample(texture_target_color, sampler_target_color, uv + offset).rgb
                    * weights[i]
                    * weights[j];
            }
        }
    }
    return vec4(color, 1.0);
}

fn apply_surface_blend(source: vec4<f32>, position: vec2<f32>, mode: u32) -> vec4<f32> {
    if mode == 61u {
        return vec4(vec3<f32>(1.0), source.a);
    }
    let uv = position / draw.screen_target.zw;
    if mode >= 57u && mode <= 59u {
        return gaussian_backdrop(uv, source.a, mode);
    }
    if mode == 60u {
        let offset = 64.0 * (source.rg - vec2<f32>(0.5)) / draw.screen_target.xy * source.a;
        return textureSample(texture_target_color, sampler_target_color, uv + offset);
    }
    let backdrop = textureSample(texture_target_color, sampler_target_color, uv);
    return photoshop_layer_blend(backdrop, source, mode);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    var color = vec4(1.0);
    if SIMPLE_TEXTURE_COUNT >= 1u {
        color = textureSampleBias(texture0, sampler0, input.uv0, draw.params.x);
    }
    if SIMPLE_TEXTURE_COUNT >= 2u && SIMPLE_MULTI_TEXTURE_MODE != 0u {
        let source = textureSampleBias(texture1, sampler1, input.uv1, draw.params.y);
        color = combine_textures(color, source, SIMPLE_MULTI_TEXTURE_MODE);
    }

    color *= input.primary_color;
    color = vec4(color.rgb + input.secondary_color.rgb, color.a);
    if SIMPLE_BLEND_MODE == 9u {
        color = vec4(mix(vec3(1.0), color.rgb, color.a), 1.0);
    } else if SIMPLE_BLEND_MODE >= 33u {
        color = apply_surface_blend(color, input.position.xy, SIMPLE_BLEND_MODE);
    }

    if !alpha_test_passes(color.a, u32(draw.params.w), u32(draw.params.z)) {
        discard;
    }
    return color;
}
