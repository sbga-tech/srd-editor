use std::fmt;

use crate::projection::{Matrix4x4, identity_matrix4x4_game, mul_matrix4x4_game};
use crate::reference_runtime::ReferenceLayerParent;
use crate::target_pass::SrdQueueState;
use crate::texture::TextureSamplerState;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct SrdVertex {
    pub position: [f32; 3],
    pub primary_color: [u8; 4],
    pub secondary_color: [u8; 4],
    pub texture_coordinates: [[f32; 2]; 2],
}

impl SrdVertex {
    pub const STRIDE: usize = 36;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SrdQuad {
    pub vertices: [SrdVertex; 4],
}

impl SrdQuad {
    pub const fn new(vertices: [SrdVertex; 4]) -> Self {
        Self { vertices }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawOrigin {
    pub owner: ReferenceLayerParent,
    pub scene_index: usize,
    pub layer_index: usize,
    pub node_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawOrder {
    pub renderer_layer_key: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SrdTransform {
    /// Original SimpleShader `mtxWorld` input.
    pub world: Matrix4x4,
    /// Original SimpleShader `mtxPrjView` input. It remains separate so WGSL
    /// executes the same matrix operations as the recovered Cg program.
    pub projection_view: Matrix4x4,
    /// ShapeEnv2D `screenParam.xy`: half the filter source dimensions.
    pub screen_param: [f32; 2],
}

impl SrdTransform {
    pub fn from_scene(world: Matrix4x4, projection_view: Matrix4x4, target_size: [u32; 2]) -> Self {
        Self {
            world,
            projection_view,
            screen_param: [target_size[0] as f32 * 0.5, target_size[1] as f32 * 0.5],
        }
    }

    pub fn identity_2d(target_size: [u32; 2]) -> Self {
        Self::from_scene(
            identity_matrix4x4_game(),
            identity_matrix4x4_game(),
            target_size,
        )
    }

    pub fn clip_from_vertex(self, transform_mode: SimpleTransformMode) -> Matrix4x4 {
        if transform_mode == SimpleTransformMode::TwoDimensional {
            let screen = Matrix4x4 {
                rows: [
                    [
                        1.0 / self.screen_param[0],
                        0.0,
                        0.0,
                        -1.0 - 0.5 / self.screen_param[0],
                    ],
                    [
                        0.0,
                        -1.0 / self.screen_param[1],
                        0.0,
                        1.0 + 0.5 / self.screen_param[1],
                    ],
                    [0.0, 0.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ],
            };
            mul_matrix4x4_game(&screen, &self.world)
        } else {
            mul_matrix4x4_game(&self.projection_view, &self.world)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum SimpleVertexLayout {
    Format13 = 13,
    Format14 = 14,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum SimpleTransformMode {
    ThreeDimensional = 0,
    TwoDimensional = 1,
}
impl SimpleTransformMode {
    pub const fn from_is_2d(is_2d: bool) -> Self {
        if is_2d {
            Self::TwoDimensional
        } else {
            Self::ThreeDimensional
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum SrdMultiTextureMode {
    None = 0,
    ReplaceWithSecondary = 9,
    BlendWithSecondaryAlpha = 10,
    SecondaryRedToAlpha = 11,
    MultiplyAlphaBySecondaryRed = 12,
}

impl SrdMultiTextureMode {
    pub const fn from_image_field(value: u32) -> Self {
        // The native draw path maps clamped CIMG/CNUM values 1..4 to the
        // SimpleShader's consecutive Surfride modes 9..12.
        match value {
            1 => Self::ReplaceWithSecondary,
            2 => Self::BlendWithSecondaryAlpha,
            3 => Self::SecondaryRedToAlpha,
            4 => Self::MultiplyAlphaBySecondaryRed,
            _ => Self::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SimpleTextureCount(u8);

impl SimpleTextureCount {
    pub fn from_slots(
        textures: [Option<SrdTextureBinding>; 3],
    ) -> Result<Self, UnsupportedSimpleTextureLayout> {
        if textures[2].is_some() {
            return Err(UnsupportedSimpleTextureLayout::ThirdSlot);
        }
        match (textures[0].is_some(), textures[1].is_some()) {
            (false, false) => Ok(Self(0)),
            (true, false) => Ok(Self(1)),
            (true, true) => Ok(Self(2)),
            (false, true) => Err(UnsupportedSimpleTextureLayout::SparseSecondSlot),
        }
    }

    pub const fn get(self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SimpleBlendMode(u8);

impl SimpleBlendMode {
    const fn from_render_preset(
        render_preset: i32,
    ) -> Result<Self, UnsupportedSurfaceRenderPreset> {
        match render_preset {
            9 => Ok(Self(9)),
            0..=21 => Ok(Self(0)),
            33..=61 => Ok(Self(render_preset as u8)),
            _ => Err(UnsupportedSurfaceRenderPreset(render_preset)),
        }
    }

    pub const fn get(self) -> u8 {
        self.0
    }
}

/// Pipeline-specialized Ceylon SimpleShader feature set. Geometry layout is
/// selected independently because both format 13 and format 14 execute this
/// same program with different vertex declarations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SimpleShaderProgram {
    pub transform_mode: SimpleTransformMode,
    pub texture_count: SimpleTextureCount,
    pub multi_texture_mode: SrdMultiTextureMode,
    pub blend_mode: SimpleBlendMode,
}

impl SimpleShaderProgram {
    pub const fn requires_backdrop(self) -> bool {
        self.blend_mode.0 >= 33 && self.blend_mode.0 <= 60
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SimpleShaderProfile {
    pub vertex_layout: SimpleVertexLayout,
    pub transform_mode: SimpleTransformMode,
    pub texture_count: SimpleTextureCount,
    pub multi_texture_mode: SrdMultiTextureMode,
    pub alpha_blend: bool,
    pub blend_mode: SimpleBlendMode,
}

impl SimpleShaderProfile {
    pub fn surface(
        transform_mode: SimpleTransformMode,
        textures: [Option<SrdTextureBinding>; 3],
        image_field_0c: u32,
        render_preset: i32,
    ) -> Result<Self, SimpleShaderProfileError> {
        Ok(Self {
            vertex_layout: SimpleVertexLayout::Format14,
            transform_mode,
            texture_count: SimpleTextureCount::from_slots(textures)?,
            multi_texture_mode: SrdMultiTextureMode::from_image_field(image_field_0c),
            alpha_blend: render_preset_alpha_blend(render_preset)?,
            blend_mode: SimpleBlendMode::from_render_preset(render_preset)?,
        })
    }

    pub const fn fennel(transform_mode: SimpleTransformMode) -> Self {
        Self {
            vertex_layout: SimpleVertexLayout::Format13,
            transform_mode,
            texture_count: SimpleTextureCount(1),
            multi_texture_mode: SrdMultiTextureMode::None,
            alpha_blend: true,
            blend_mode: SimpleBlendMode(3),
        }
    }

    pub const fn program(self) -> SimpleShaderProgram {
        SimpleShaderProgram {
            transform_mode: self.transform_mode,
            texture_count: self.texture_count,
            multi_texture_mode: self.multi_texture_mode,
            blend_mode: self.blend_mode,
        }
    }

    pub const fn requires_backdrop(self) -> bool {
        self.program().requires_backdrop()
    }

    /// Normalized values for the SimpleShader features an independent SRD draw
    /// can select: transform, colors, UVs, distance update, alpha blend, maps,
    /// base blend, and first multi-texture blend.
    pub fn feature_values(self) -> [u8; 10] {
        [
            self.transform_mode as u8,
            2,
            if self.vertex_layout == SimpleVertexLayout::Format14 {
                2
            } else {
                1
            },
            1,
            self.alpha_blend as u8,
            (self.texture_count.0 >= 1) as u8,
            (self.texture_count.0 >= 2) as u8,
            0,
            self.blend_mode.0,
            self.multi_texture_mode as u8,
        ]
    }

    /// Ceylon's 18-byte diagnostic key for the normalized feature subset.
    /// This is evidence/testing output only; runtime pipeline selection uses
    /// the typed profile directly.
    pub fn compact_key(self) -> [u8; 18] {
        let mut bits = [false; 71];
        bits[2] = self.transform_mode as u32 != 0;
        let color_count = 2;
        bits[9] = color_count & 1 != 0;
        bits[10] = color_count & 2 != 0;
        let texcoord_count = if self.vertex_layout == SimpleVertexLayout::Format14 {
            2
        } else {
            1
        };
        bits[11] = texcoord_count & 1 != 0;
        bits[12] = texcoord_count & 2 != 0;
        bits[20] = self.vertex_layout == SimpleVertexLayout::Format14;
        bits[24] = self.alpha_blend;
        bits[36] = self.texture_count.0 >= 1;
        bits[37] = self.texture_count.0 >= 2;
        let mut bit = 0;
        while bit < 6 {
            bits[41 + bit] = self.blend_mode.0 & (1 << bit) != 0;
            bit += 1;
        }
        bit = 0;
        while bit < 4 {
            bits[47 + bit] = self.multi_texture_mode as u8 & (1 << bit) != 0;
            bit += 1;
        }
        let mut key = [b'A'; 18];
        let mut nibble = 0;
        while nibble < 18 {
            let mut value = 0;
            bit = 0;
            while bit < 4 {
                let position = nibble * 4 + bit;
                if position < 71 && bits[position] {
                    value |= 1 << bit;
                }
                bit += 1;
            }
            key[nibble] += value;
            nibble += 1;
        }
        key
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnsupportedSurfaceRenderPreset(pub i32);

impl fmt::Display for UnsupportedSurfaceRenderPreset {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "render preset {} is outside the proven native SimpleShader contract (0..=21 or 33..=61)",
            self.0
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsupportedSimpleTextureLayout {
    SparseSecondSlot,
    ThirdSlot,
}

impl fmt::Display for UnsupportedSimpleTextureLayout {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SparseSecondSlot => formatter
                .write_str("SimpleShader texture slot 1 is bound while base slot 0 is absent"),
            Self::ThirdSlot => formatter
                .write_str("SimpleShader exposes two textures, but texture slot 2 is bound"),
        }
    }
}

impl std::error::Error for UnsupportedSimpleTextureLayout {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimpleShaderProfileError {
    RenderPreset(UnsupportedSurfaceRenderPreset),
    TextureLayout(UnsupportedSimpleTextureLayout),
}

impl fmt::Display for SimpleShaderProfileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RenderPreset(error) => error.fmt(formatter),
            Self::TextureLayout(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for SimpleShaderProfileError {}

impl From<UnsupportedSurfaceRenderPreset> for SimpleShaderProfileError {
    fn from(error: UnsupportedSurfaceRenderPreset) -> Self {
        Self::RenderPreset(error)
    }
}

impl From<UnsupportedSimpleTextureLayout> for SimpleShaderProfileError {
    fn from(error: UnsupportedSimpleTextureLayout) -> Self {
        Self::TextureLayout(error)
    }
}
impl std::error::Error for UnsupportedSurfaceRenderPreset {}

const fn render_preset_alpha_blend(
    render_preset: i32,
) -> Result<bool, UnsupportedSurfaceRenderPreset> {
    match render_preset {
        0..=21 => Ok(matches!(render_preset, 2..=10 | 12..=19)),
        33..=60 => Ok(false),
        61 => Ok(true),
        _ => Err(UnsupportedSurfaceRenderPreset(render_preset)),
    }
}
pub const fn select_srd_image_render_preset(
    image_flags: u32,
    render_preset_override: i32,
    renderer_special_mode: bool,
) -> Option<i32> {
    if render_preset_override >= 0 {
        return Some(render_preset_override);
    }
    match image_flags & 0x0f {
        0 if renderer_special_mode => match image_flags & 0x600 {
            0x200 => Some(20),
            0x400 => Some(21),
            _ => Some(3),
        },
        0 => Some(3),
        1 => Some(4),
        2 => Some(5),
        3 => Some(9),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum BlendFactor {
    Zero,
    One,
    SourceColor,
    OneMinusSourceColor,
    SourceAlpha,
    OneMinusSourceAlpha,
    DestinationColor,
    OneMinusDestinationColor,
    DestinationAlpha,
    OneMinusDestinationAlpha,
    SourceAlphaSaturated,
    Constant,
    OneMinusConstant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum BlendOperation {
    Add,
    Subtract,
    ReverseSubtract,
    Minimum,
    Maximum,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlendComponent {
    pub source: BlendFactor,
    pub destination: BlendFactor,
    pub operation: BlendOperation,
}

impl BlendComponent {
    const fn new(source: BlendFactor, destination: BlendFactor, operation: BlendOperation) -> Self {
        Self {
            source,
            destination,
            operation,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlendState {
    pub enabled: bool,
    pub color: BlendComponent,
    pub alpha: BlendComponent,
}

const BASE_COLOR_BLEND: BlendComponent = BlendComponent::new(
    BlendFactor::SourceAlpha,
    BlendFactor::OneMinusSourceAlpha,
    BlendOperation::Add,
);

impl BlendState {
    fn for_render_preset(preset: i32) -> (Self, bool) {
        debug_assert!((0..=21).contains(&preset) || (33..=61).contains(&preset));
        let index = match preset {
            33..=60 => 0,
            61 => 3,
            _ => preset,
        };
        let mut state = Self {
            enabled: false,
            color: BASE_COLOR_BLEND,
            alpha: BASE_COLOR_BLEND,
        };
        let mut alpha_test = false;
        match index {
            1 | 21 => alpha_test = true,
            2 => {
                state.enabled = true;
                alpha_test = true;
            }
            3 => state.enabled = true,
            4 => {
                state.enabled = true;
                state.color.destination = BlendFactor::One;
            }
            5 | 15 => {
                state.enabled = true;
                state.color.destination = BlendFactor::One;
                state.color.operation = BlendOperation::ReverseSubtract;
            }
            6 | 16 => {
                state.enabled = true;
                state.color.source = BlendFactor::Zero;
                state.color.destination = BlendFactor::SourceColor;
            }
            7 | 17 => {
                state.enabled = true;
                state.color.source = BlendFactor::OneMinusDestinationColor;
                state.color.destination = BlendFactor::Zero;
            }
            8 => {
                state.enabled = true;
                state.color.source = BlendFactor::DestinationColor;
                state.color.destination = BlendFactor::One;
            }
            9 | 19 => {
                state.enabled = true;
                state.color.source = BlendFactor::Zero;
                state.color.destination = BlendFactor::SourceColor;
                alpha_test = true;
            }
            10 => {
                state.enabled = true;
                state.color =
                    BlendComponent::new(BlendFactor::One, BlendFactor::Zero, BlendOperation::Add);
            }
            11 => {
                state.color =
                    BlendComponent::new(BlendFactor::One, BlendFactor::Zero, BlendOperation::Add);
            }
            12 => {
                state.enabled = true;
                state.alpha = BlendComponent::new(
                    BlendFactor::Zero,
                    BlendFactor::OneMinusSourceAlpha,
                    BlendOperation::Add,
                );
                alpha_test = true;
            }
            13 => {
                state.enabled = true;
                state.alpha = BlendComponent::new(
                    BlendFactor::Zero,
                    BlendFactor::OneMinusSourceAlpha,
                    BlendOperation::Add,
                );
            }
            14 => {
                state.enabled = true;
                state.color.destination = BlendFactor::One;
                state.alpha =
                    BlendComponent::new(BlendFactor::Zero, BlendFactor::One, BlendOperation::Add);
            }
            18 => {
                state.enabled = true;
                state.color.source = BlendFactor::DestinationColor;
                state.color.destination = BlendFactor::OneMinusSourceAlpha;
            }
            _ => {}
        }
        // D3D9 applies the color factors and operation to render-target alpha
        // whenever SEPARATEALPHABLENDENABLE is false. WebGPU always specifies
        // color and alpha components independently, so copy the effective
        // color equation for every non-separate preset.
        if !matches!(index, 12..=14) {
            state.alpha = state.color;
        }
        (state, alpha_test)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum CompareFunction {
    Never,
    Less,
    Equal,
    LessEqual,
    Greater,
    NotEqual,
    GreaterEqual,
    Always,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlphaTest {
    pub comparison: CompareFunction,
    pub reference: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum StencilOperation {
    Keep,
    Zero,
    Replace,
    IncrementClamp,
    DecrementClamp,
    Invert,
    IncrementWrap,
    DecrementWrap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StencilFaceState {
    pub comparison: CompareFunction,
    pub fail: StencilOperation,
    pub depth_fail: StencilOperation,
    pub pass: StencilOperation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StencilState {
    pub enabled: bool,
    pub face: StencilFaceState,
    pub reference: u8,
    pub read_mask: u8,
    pub write_mask: u8,
}

impl StencilState {
    pub const DISABLED: Self = Self {
        enabled: false,
        face: StencilFaceState {
            comparison: CompareFunction::Always,
            fail: StencilOperation::Keep,
            depth_fail: StencilOperation::Keep,
            pass: StencilOperation::Keep,
        },
        reference: 0,
        read_mask: 0,
        write_mask: 0,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DepthState {
    pub enabled: bool,
    pub write_enabled: bool,
    pub comparison: CompareFunction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum CullMode {
    None,
    Clockwise,
    CounterClockwise,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum FillMode {
    Solid,
    Wireframe,
    Point,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RasterState {
    pub cull: CullMode,
    pub fill: FillMode,
    pub color_write_mask: u8,
    pub depth_bias: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SrdPipelineState {
    pub blend: BlendState,
    pub raster: RasterState,
    pub depth: DepthState,
    pub stencil: StencilState,
    pub alpha_test: Option<AlphaTest>,
}

impl SrdPipelineState {
    fn surface(
        render_preset: i32,
        image_field_10: i32,
        image_field_14: u32,
        image_field_18: u8,
        renderer_counter: &mut u8,
    ) -> Self {
        let (blend, preset_alpha_test) = BlendState::for_render_preset(render_preset);
        let mut alpha_test = preset_alpha_test.then_some(AlphaTest {
            comparison: CompareFunction::Greater,
            reference: 0,
        });
        let shift = image_field_14 & 0x1f;
        let mask = 1u8.checked_shl(shift).unwrap_or(0);
        let stencil = match image_field_10 {
            1 | 2 => {
                *renderer_counter = renderer_counter.wrapping_sub(1);
                alpha_test = Some(AlphaTest {
                    comparison: CompareFunction::Greater,
                    reference: 128,
                });
                StencilState {
                    enabled: true,
                    face: StencilFaceState {
                        comparison: CompareFunction::Always,
                        fail: StencilOperation::Keep,
                        depth_fail: StencilOperation::Keep,
                        pass: StencilOperation::Replace,
                    },
                    reference: mask,
                    read_mask: mask,
                    write_mask: mask,
                }
            }
            3 if image_field_18 == 0 => stencil_test(mask, CompareFunction::Equal),
            4 if image_field_18 == 0 => stencil_test(mask, CompareFunction::NotEqual),
            _ => StencilState::DISABLED,
        };
        Self {
            blend,
            raster: RasterState {
                cull: CullMode::None,
                fill: FillMode::Solid,
                color_write_mask: 0x0f,
                depth_bias: 0,
            },
            depth: DepthState {
                enabled: false,
                write_enabled: false,
                comparison: CompareFunction::LessEqual,
            },
            stencil,
            alpha_test,
        }
    }

    pub const fn fennel() -> Self {
        Self {
            blend: BlendState {
                enabled: true,
                color: BASE_COLOR_BLEND,
                alpha: BASE_COLOR_BLEND,
            },
            raster: RasterState {
                cull: CullMode::Clockwise,
                fill: FillMode::Solid,
                color_write_mask: 0x0f,
                depth_bias: 0,
            },
            depth: DepthState {
                enabled: true,
                write_enabled: true,
                comparison: CompareFunction::LessEqual,
            },
            stencil: StencilState::DISABLED,
            alpha_test: None,
        }
    }
}

const fn stencil_test(mask: u8, comparison: CompareFunction) -> StencilState {
    StencilState {
        enabled: true,
        face: StencilFaceState {
            comparison,
            fail: StencilOperation::Keep,
            depth_fail: StencilOperation::Keep,
            pass: StencilOperation::Keep,
        },
        reference: mask,
        read_mask: mask,
        write_mask: mask,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SrdTextureBinding {
    pub texture_index: usize,
    pub sampler: TextureSamplerState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SrdMaterial {
    pub textures: [Option<SrdTextureBinding>; 3],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SrdSurfaceStateInput {
    pub transform: SrdTransform,
    pub transform_mode: SimpleTransformMode,
    pub render_preset: i32,
    pub image_field_0c: u32,
    pub image_field_10: i32,
    pub image_field_14: u32,
    pub image_field_18: u8,
    pub textures: [Option<SrdTextureBinding>; 3],
}

impl SrdMaterial {
    pub fn texture_count(self) -> usize {
        self.textures
            .iter()
            .filter(|texture| texture.is_some())
            .count()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SrdDrawState {
    pub transform: SrdTransform,
    pub profile: SimpleShaderProfile,
    pub pipeline: SrdPipelineState,
    pub material: SrdMaterial,
    pub queue: SrdQueueState,
}

impl SrdDrawState {
    pub fn surface(
        input: SrdSurfaceStateInput,
        renderer_counter: &mut u8,
    ) -> Result<Self, SimpleShaderProfileError> {
        let SrdSurfaceStateInput {
            transform,
            transform_mode,
            render_preset,
            image_field_0c,
            image_field_10,
            image_field_14,
            image_field_18,
            textures,
        } = input;

        let profile =
            SimpleShaderProfile::surface(transform_mode, textures, image_field_0c, render_preset)?;
        let pipeline = SrdPipelineState::surface(
            render_preset,
            image_field_10,
            image_field_14,
            image_field_18,
            renderer_counter,
        );
        Ok(Self {
            transform,
            profile,
            pipeline,
            material: SrdMaterial { textures },
            queue: SrdQueueState::surface(render_preset > 32 || pipeline.blend.enabled),
        })
    }

    pub const fn fennel(transform: SrdTransform, transform_mode: SimpleTransformMode) -> Self {
        Self {
            transform,
            profile: SimpleShaderProfile::fennel(transform_mode),
            pipeline: SrdPipelineState::fennel(),
            material: SrdMaterial {
                textures: [None; 3],
            },
            queue: SrdQueueState::fennel(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_fields_select_consecutive_surfride_texture_modes() {
        let expected = [
            SrdMultiTextureMode::None,
            SrdMultiTextureMode::ReplaceWithSecondary,
            SrdMultiTextureMode::BlendWithSecondaryAlpha,
            SrdMultiTextureMode::SecondaryRedToAlpha,
            SrdMultiTextureMode::MultiplyAlphaBySecondaryRed,
        ];
        for (field, expected) in (0u32..=4).zip(expected) {
            assert_eq!(SrdMultiTextureMode::from_image_field(field), expected);
        }
        assert_eq!(
            SrdMultiTextureMode::from_image_field(257),
            SrdMultiTextureMode::None
        );
        assert_eq!(
            SrdMultiTextureMode::from_image_field(u32::MAX),
            SrdMultiTextureMode::None
        );
    }

    #[test]
    fn surface_state_rejects_only_unproven_shader_presets_without_mutation() {
        for preset in [-1, 22, 32, 62, i32::MAX] {
            let mut renderer_counter = 7;
            let result = SrdDrawState::surface(
                SrdSurfaceStateInput {
                    transform: SrdTransform::identity_2d([64, 64]),
                    transform_mode: SimpleTransformMode::TwoDimensional,
                    render_preset: preset,
                    image_field_0c: 0,
                    image_field_10: 1,
                    image_field_14: 2,
                    image_field_18: 0,
                    textures: [None; 3],
                },
                &mut renderer_counter,
            );
            assert_eq!(
                result.unwrap_err(),
                SimpleShaderProfileError::RenderPreset(UnsupportedSurfaceRenderPreset(preset))
            );
            assert_eq!(renderer_counter, 7);
        }
    }

    #[test]
    fn supported_surface_presets_preserve_shader_visible_modes() {
        for preset in (0..=21).chain(33..=61) {
            let mut renderer_counter = 0;
            let state = SrdDrawState::surface(
                SrdSurfaceStateInput {
                    transform: SrdTransform::identity_2d([64, 64]),
                    transform_mode: SimpleTransformMode::TwoDimensional,
                    render_preset: preset,
                    image_field_0c: 0,
                    image_field_10: 0,
                    image_field_14: 0,
                    image_field_18: 0,
                    textures: [None; 3],
                },
                &mut renderer_counter,
            )
            .unwrap();
            assert_eq!(
                state.profile.blend_mode.get(),
                if preset == 9 || preset >= 33 {
                    preset as u8
                } else {
                    0
                }
            );
            assert_eq!(
                state.profile.requires_backdrop(),
                (33..=60).contains(&preset)
            );
            assert_eq!(
                state.profile.alpha_blend,
                preset == 61 || matches!(preset, 2..=10 | 12..=19)
            );
            assert_eq!(state.profile.alpha_blend, state.pipeline.blend.enabled);
            assert_eq!(
                state.queue.target_attributes,
                if preset >= 33 || state.pipeline.blend.enabled {
                    0xf7
                } else {
                    0xff
                }
            );
        }
    }

    #[test]
    fn normalized_profiles_reproduce_known_ceylon_keys() {
        assert_eq!(
            SimpleShaderProfile::fennel(SimpleTransformMode::ThreeDimensional).compact_key(),
            *b"AAMAAABAABGAAAAAAA"
        );
        assert_eq!(
            SimpleShaderProfile::fennel(SimpleTransformMode::TwoDimensional).compact_key(),
            *b"EAMAAABAABGAAAAAAA"
        );
        let textured_surface = SimpleShaderProfile {
            vertex_layout: SimpleVertexLayout::Format14,
            transform_mode: SimpleTransformMode::TwoDimensional,
            texture_count: SimpleTextureCount(1),
            multi_texture_mode: SrdMultiTextureMode::None,
            alpha_blend: true,
            blend_mode: SimpleBlendMode(3),
        };
        assert_eq!(textured_surface.compact_key(), *b"EAEBABBAABGAAAAAAA");
    }

    #[test]
    fn program_identity_excludes_vertex_layout_and_fixed_alpha_blend() {
        let mut profile = SimpleShaderProfile::fennel(SimpleTransformMode::TwoDimensional);
        let program = profile.program();
        profile.vertex_layout = SimpleVertexLayout::Format14;
        profile.alpha_blend = false;

        assert_eq!(profile.program(), program);
    }

    #[test]
    fn program_identity_includes_every_shader_specialization() {
        let base = SimpleShaderProfile::fennel(SimpleTransformMode::TwoDimensional);
        let mut changed = [base; 4];
        changed[0].transform_mode = SimpleTransformMode::ThreeDimensional;
        changed[1].texture_count = SimpleTextureCount(2);
        changed[2].multi_texture_mode = SrdMultiTextureMode::ReplaceWithSecondary;
        changed[3].blend_mode = SimpleBlendMode(9);

        assert!(
            changed
                .into_iter()
                .all(|profile| profile.program() != base.program())
        );
    }

    #[test]
    fn target_color_requirement_excludes_color_fill_mode() {
        let mut profile = SimpleShaderProfile::fennel(SimpleTransformMode::TwoDimensional);
        profile.blend_mode = SimpleBlendMode(60);
        assert!(profile.program().requires_backdrop());
        profile.blend_mode = SimpleBlendMode(61);
        assert!(!profile.program().requires_backdrop());
    }

    #[test]
    fn simple_texture_profile_rejects_sparse_and_third_slots() {
        let binding = SrdTextureBinding {
            texture_index: 0,
            sampler: TextureSamplerState {
                address_u: crate::texture::TextureAddressMode::Clamp,
                address_v: crate::texture::TextureAddressMode::Clamp,
                min_filter: crate::texture::TextureFilter::Point,
                mag_filter: crate::texture::TextureFilter::Point,
            },
        };
        assert_eq!(
            SimpleTextureCount::from_slots([None, Some(binding), None]),
            Err(UnsupportedSimpleTextureLayout::SparseSecondSlot)
        );
        assert_eq!(
            SimpleTextureCount::from_slots([Some(binding), Some(binding), Some(binding)]),
            Err(UnsupportedSimpleTextureLayout::ThirdSlot)
        );
    }

    #[test]
    fn nonseparate_preset_blending_applies_color_equation_to_alpha() {
        for preset in [2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 15, 16, 17, 18, 19, 61] {
            let (blend, _) = BlendState::for_render_preset(preset);
            assert_eq!(blend.alpha, blend.color, "preset {preset}");
        }

        let (disabled, _) = BlendState::for_render_preset(11);
        assert!(!disabled.enabled);

        let (separate, _) = BlendState::for_render_preset(14);
        assert_ne!(separate.alpha, separate.color);
    }

    #[test]
    fn stencil_write_is_explicit_pipeline_state() {
        let mut order = 7;
        let state = SrdPipelineState::surface(3, 1, 2, 0, &mut order);
        assert_eq!(order, 6);
        assert_eq!(state.stencil.reference, 4);
        assert_eq!(state.stencil.face.pass, StencilOperation::Replace);
        assert_eq!(state.alpha_test.unwrap().reference, 128);
    }
}
