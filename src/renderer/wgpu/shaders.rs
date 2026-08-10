use crate::render::D3d9ComparisonFunction;
use crate::renderer::backend::RenderBackendError;
use crate::shader::CEYLON_SIMPLE_SHADER_KEY_LENGTH;
use crate::shader_bytecode::canonical_embedded_simple_shader_key;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WgpuVertexSemantic {
    Position,
    Color0,
    Color1,
    TexCoord0,
    TexCoord1,
    Tangent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct WgpuVertexInput {
    pub location: u32,
    pub semantic: WgpuVertexSemantic,
}

#[derive(Debug, Clone, Copy)]
struct WgpuShaderEntry {
    key: [u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH],
    vertex_index: u8,
    pixel_index: u8,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct WgpuShaderPair {
    pub vertex_index: u8,
    pub pixel_index: u8,
    pub vertex_source: &'static str,
    pub pixel_source: &'static str,
    pub vertex_inputs: &'static [WgpuVertexInput],
}

// Stored vertex sources return Ceylon clip-space Y unchanged. A post-shader Y
// negation mirrors D3D9 output; the corpus differential test guards this invariant.
include!("shaders/generated.rs");

pub(super) fn embedded_wgpu_shader_pair(
    key: &[u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH],
) -> Option<WgpuShaderPair> {
    let canonical = canonical_embedded_simple_shader_key(key)?;
    let index = WGPU_SHADER_ENTRIES
        .binary_search_by_key(&canonical, |entry| entry.key)
        .ok()?;
    let entry = WGPU_SHADER_ENTRIES[index];
    Some(WgpuShaderPair {
        vertex_index: entry.vertex_index,
        pixel_index: entry.pixel_index,
        vertex_source: VERTEX_SOURCES[usize::from(entry.vertex_index)],
        pixel_source: PIXEL_SOURCES[usize::from(entry.pixel_index)],
        vertex_inputs: VERTEX_INPUTS[usize::from(entry.vertex_index)],
    })
}

pub(super) fn vertex_sources() -> impl ExactSizeIterator<Item = &'static str> {
    VERTEX_SOURCES.into_iter()
}

pub(super) fn pixel_sources() -> impl ExactSizeIterator<Item = &'static str> {
    PIXEL_SOURCES.into_iter()
}

pub(super) fn vertex_source_with_d3d_color_swizzle(
    source: &str,
    inputs: &[WgpuVertexInput],
) -> Result<String, RenderBackendError> {
    let vertex = source.rfind("@vertex").ok_or_else(|| {
        RenderBackendError("translated Ceylon vertex shader has no vertex entry point".into())
    })?;
    let relative_name = source[vertex..].find("fn main(").ok_or_else(|| {
        RenderBackendError("translated Ceylon vertex shader has no main entry point".into())
    })?;
    let name = vertex + relative_name + 3;
    let arguments_start = name + "main(".len();
    let arguments_end = matching_parenthesis(source, arguments_start - 1)?;
    let argument_names = split_arguments(&source[arguments_start..arguments_end])?;
    if argument_names.len() != inputs.len() {
        return Err(RenderBackendError(format!(
            "translated vertex entry has {} arguments but metadata describes {}",
            argument_names.len(),
            inputs.len()
        )));
    }

    let mut translated = source.to_owned();
    for (argument, input) in argument_names.iter().zip(inputs) {
        if !matches!(
            input.semantic,
            WgpuVertexSemantic::Color0 | WgpuVertexSemantic::Color1
        ) {
            continue;
        }
        let assignment = format!(" = {argument};");
        let start = translated[arguments_end..]
            .find(&assignment)
            .map(|relative| arguments_end + relative)
            .ok_or_else(|| {
                RenderBackendError(format!(
                    "translated vertex entry does not assign color argument {argument}"
                ))
            })?;
        translated.replace_range(
            start..start + assignment.len(),
            &format!(" = {argument}.bgra;"),
        );
    }
    Ok(translated)
}

pub(super) fn pixel_source_with_sampler_biases(
    source: &str,
    biases: [u32; 2],
) -> Result<String, RenderBackendError> {
    let mut translated = source.to_owned();
    for (sampler, bias_bits) in ["s0_2d, s0_", "s1_2d, s1_"].into_iter().zip(biases) {
        if bias_bits == 0 {
            continue;
        }
        let prefix = format!("textureSample({sampler},");
        let mut search_start = 0usize;
        while let Some(relative) = translated[search_start..].find(&prefix) {
            let call_start = search_start + relative;
            let opening = call_start + "textureSample".len();
            let closing = matching_parenthesis(&translated, opening)?;
            translated.insert_str(closing, &format!(", bitcast<f32>({bias_bits}u)"));
            translated.replace_range(
                call_start..call_start + "textureSample".len(),
                "textureSampleBias",
            );
            search_start = closing + "Bias".len() + 1;
        }
    }
    Ok(translated)
}
pub(super) fn pixel_source_with_alpha_test(
    source: &str,
    comparison: D3d9ComparisonFunction,
    reference: u8,
) -> Result<String, RenderBackendError> {
    if comparison == D3d9ComparisonFunction::Always {
        return Ok(source.to_owned());
    }

    let fragment = source.rfind("@fragment").ok_or_else(|| {
        RenderBackendError("translated Ceylon pixel shader has no fragment entry point".into())
    })?;
    let return_start = source[fragment..]
        .rfind("return ")
        .map(|relative| fragment + relative)
        .ok_or_else(|| {
            RenderBackendError("translated Ceylon fragment entry has no return statement".into())
        })?;
    let return_end = source[return_start..]
        .find(';')
        .map(|relative| return_start + relative + 1)
        .ok_or_else(|| {
            RenderBackendError("translated Ceylon fragment return is unterminated".into())
        })?;
    let expression = source[return_start + "return ".len()..return_end - 1].trim();
    let condition = match comparison {
        D3d9ComparisonFunction::Never => "false".to_owned(),
        D3d9ComparisonFunction::Less => format!("ceylon_alpha < {reference}u"),
        D3d9ComparisonFunction::Equal => format!("ceylon_alpha == {reference}u"),
        D3d9ComparisonFunction::LessEqual => format!("ceylon_alpha <= {reference}u"),
        D3d9ComparisonFunction::Greater => format!("ceylon_alpha > {reference}u"),
        D3d9ComparisonFunction::NotEqual => format!("ceylon_alpha != {reference}u"),
        D3d9ComparisonFunction::GreaterEqual => format!("ceylon_alpha >= {reference}u"),
        D3d9ComparisonFunction::Always => unreachable!("handled above"),
    };
    let replacement = format!(
        "let ceylon_color = {expression};\n    let ceylon_alpha = u32(round(clamp(ceylon_color.a, 0.0, 1.0) * 255.0));\n    if !({condition}) {{\n        discard;\n    }}\n    return ceylon_color;"
    );
    let mut translated = source.to_owned();
    translated.replace_range(return_start..return_end, &replacement);
    Ok(translated)
}

fn matching_parenthesis(source: &str, opening: usize) -> Result<usize, RenderBackendError> {
    let mut depth = 0u32;
    for (offset, byte) in source.as_bytes()[opening..].iter().copied().enumerate() {
        match byte {
            b'(' => depth += 1,
            b')' => {
                depth = depth.checked_sub(1).ok_or_else(|| {
                    RenderBackendError("translated shader has unbalanced entry arguments".into())
                })?;
                if depth == 0 {
                    return Ok(opening + offset);
                }
            }
            _ => {}
        }
    }
    Err(RenderBackendError(
        "translated shader has unterminated entry arguments".into(),
    ))
}

fn split_arguments(arguments: &str) -> Result<Vec<&str>, RenderBackendError> {
    arguments
        .split(',')
        .map(str::trim)
        .filter(|argument| !argument.is_empty())
        .map(|argument| {
            let declaration = argument.split_once(':').map_or(argument, |(left, _)| left);
            declaration.split_whitespace().last().ok_or_else(|| {
                RenderBackendError(format!(
                    "translated shader entry argument is malformed: {argument}"
                ))
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shader_bytecode::{
        EMBEDDED_SIMPLE_SHADER_KEYS, FENNEL_TEXTURED_2D_SIMPLE_KEY, FENNEL_TEXTURED_SIMPLE_KEY,
    };

    #[test]
    fn every_embedded_simple_key_has_a_translated_pair() {
        for key in EMBEDDED_SIMPLE_SHADER_KEYS {
            assert!(
                embedded_wgpu_shader_pair(&key).is_some(),
                "missing {:?}",
                key
            );
        }
        assert!(embedded_wgpu_shader_pair(&FENNEL_TEXTURED_SIMPLE_KEY).is_some());
        assert!(embedded_wgpu_shader_pair(&FENNEL_TEXTURED_2D_SIMPLE_KEY).is_some());
    }

    #[test]
    fn alpha_test_injects_fragment_discard_and_quantizes_to_d3d9_alpha() {
        let source = "@fragment\nfn main(@location(9) color: vec4<f32>, @location(1) uv: vec4<f32>) -> @location(0) vec4<f32> { return color; }\n";
        let injected =
            pixel_source_with_alpha_test(source, D3d9ComparisonFunction::Greater, 128).unwrap();
        assert!(injected.contains("@fragment"));
        assert!(injected.contains("let ceylon_color = color;"));
        assert!(injected.contains("ceylon_alpha > 128u"));
        assert!(injected.contains("discard;"));
    }
}
