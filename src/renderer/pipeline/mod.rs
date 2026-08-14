//! Backend-neutral draw compilation and runtime command merging.

mod state;
pub use state::*;

use std::collections::BTreeMap;
use std::fmt;

use crate::attribute::CastAttributeValue;
use crate::csli::{SliceQuad, SliceVertexColors, multiply_color_game, slice_vertex_colors};
use crate::fennel::{
    FENNEL_DEFAULT_D_VALUE, FENNEL_DEFAULT_REPEAT_SPACE_COUNT, FennelFontSlotRegistry,
    FennelFontSlotRequest, FennelNormalDrawInput, FennelOwnedTextureBatch, FennelRenderVertex,
    FennelResolvedGlyph, FennelSrdDrawPreparation, FennelSrdDrawPreparationInput,
    FennelStaticTextProperties, build_fennel_normal_vertex_batches,
    build_fennel_plain_record_stream_with_font_slots, build_fennel_srd_repeated_text,
    fennel_font_param_effect_color, fennel_srd_font_style, layout_fennel_static_srd_explicit_flags,
    layout_fennel_static_srd_font_param, measure_fennel_srd_text_size_mode0,
    prepare_fennel_srd_draw, prepare_fennel_srd_runtime_text,
};
#[cfg(test)]
use crate::game_host::ProjectTargetSnapshot;
use crate::game_host::WorldSnapshot;
use crate::image::{
    ImageCoordinateState, ImageDefinition, ImageReferenceChannel, ResolvedImageCoordinates,
    SrdTextureBindingSource, premultiply_additive_color_game,
};
use crate::number::{NumberGlyphRecord, NumberGlyphSegment};
use crate::projection::{
    Matrix4x4, cast_overlaps_render_target_game, compose_screen_matrix_game,
    identity_matrix4x4_game, inverse_matrix4x4_game, mul_matrix4x4_game,
};
use crate::reference_runtime::{
    ProjectLayerRuntimeState, ProjectRuntime, ReferenceLayerParent, RuntimeWorldState,
};
use crate::ruhuna::RuhunaRuntimeFont;
use crate::scene::{AnimationSetDefinition, Layer, Project, ReferenceTarget};
use crate::target_pass::{
    ScenePassProfile, SrdQueueState, SrdType1TargetFilter, build_srd_scene_submission_indices,
};
use crate::texture::TextureList;
use crate::transform::Affine3x4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SrdDrawError(pub String);

impl fmt::Display for SrdDrawError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for SrdDrawError {}

#[derive(Clone, Copy)]
struct ImageRenderQuadInput {
    positions: [[f32; 3]; 4],
    color_state: ImageCoordinateState,
    coordinates: [ResolvedImageCoordinates; 2],
    multiplicative_tint: [u8; 4],
    additive_tint: [u8; 4],
}

fn build_image_render_quad(image: &ImageDefinition, input: ImageRenderQuadInput) -> SrdQuad {
    let ImageRenderQuadInput {
        positions,
        color_state,
        coordinates,
        multiplicative_tint,
        additive_tint,
    } = input;
    SrdQuad::new(std::array::from_fn(|vertex_index| {
        let mut colors = image
            .vertex_colors(
                color_state,
                vertex_index,
                multiplicative_tint,
                additive_tint,
            )
            .expect("quad vertex index is always inside the four SrImage colors");
        if let Ok(permutation) = std::env::var("SRD_COLOR_PERMUTATION") {
            let indices = permutation
                .bytes()
                .map(|value| usize::from(value.saturating_sub(b'0')))
                .collect::<Vec<_>>();
            if let [red, green, blue] = indices.as_slice() {
                let source = colors.primary;
                colors.primary[..3].copy_from_slice(&[source[*red], source[*green], source[*blue]]);
            }
        }
        if std::env::var_os("SRD_ALPHA_SCREEN_PROBE").is_some() {
            let inverse = u16::from(255 - colors.primary[3]);
            colors.primary[3] = (255 - inverse * inverse / 255) as u8;
        }
        SrdVertex {
            position: positions[vertex_index],
            primary_color: colors.primary,
            secondary_color: colors.secondary,
            texture_coordinates: [
                coordinates[0].coordinates[vertex_index],
                coordinates[1].coordinates[vertex_index],
            ],
        }
    }))
}

fn build_slice_render_quad(
    quad: SliceQuad,
    colors: [SliceVertexColors; 4],
    texture_coordinates: [[f32; 2]; 4],
) -> SrdQuad {
    SrdQuad::new(std::array::from_fn(|vertex_index| SrdVertex {
        position: quad.positions[vertex_index],
        primary_color: colors[vertex_index].primary,
        secondary_color: colors[vertex_index].secondary,
        texture_coordinates: [
            texture_coordinates[vertex_index],
            texture_coordinates[vertex_index],
        ],
    }))
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SrdDraw {
    pub origin: DrawOrigin,
    pub order: DrawOrder,
    pub geometry: SrdQuad,
    pub state: SrdDrawState,
}

impl SrdDraw {
    pub const fn owner(&self) -> ReferenceLayerParent {
        self.origin.owner
    }

    pub const fn node_index(&self) -> usize {
        self.origin.node_index
    }

    pub const fn renderer_layer_key(&self) -> u32 {
        self.order.renderer_layer_key
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FennelDraw {
    pub origin: DrawOrigin,
    pub order: DrawOrder,
    pub font_name: Vec<u8>,
    pub state: SrdDrawState,
    pub batches: Vec<FennelOwnedTextureBatch>,
}

/// One low-level quad emitted by `srd_render_slice_cast` for an active CSLI
/// cell. The binary executes `srd_begin_quad_draw` and the matching submission
/// once per active cell, so cells remain separate runtime draws even when they
/// belong to the same CAST.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SliceCellDraw {
    pub cell_index: usize,
    pub draw: SrdDraw,
}

/// One drawable glyph emitted by the first history record of a freshly
/// constructed SrNumberCast. The binary calls `srd_begin_quad_draw` once per
/// non-negative glyph mapping, so glyph identity remains part of the runtime
/// stream even when adjacent packet records later merge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumberGlyphDraw {
    pub glyph_index: i16,
    pub segment: NumberGlyphSegment,
    pub draw: SrdDraw,
}

/// Draw payloads produced at each CAST render invocation. This preserves the
/// runtime layer/CAST/RefCast recursion order, but it is not yet the later
/// renderer target-queue sort/submission order.
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeCastDraw {
    Image(SrdDraw),
    SliceCell(SliceCellDraw),
    NumberGlyph(NumberGlyphDraw),
    Fennel(FennelDraw),
}

impl RuntimeCastDraw {
    pub const fn owner(&self) -> ReferenceLayerParent {
        match self {
            Self::Image(draw) => draw.owner(),
            Self::SliceCell(draw) => draw.draw.owner(),
            Self::NumberGlyph(draw) => draw.draw.owner(),
            Self::Fennel(draw) => draw.origin.owner,
        }
    }

    pub const fn node_index(&self) -> usize {
        match self {
            Self::Image(draw) => draw.node_index(),
            Self::SliceCell(draw) => draw.draw.node_index(),
            Self::NumberGlyph(draw) => draw.draw.node_index(),
            Self::Fennel(draw) => draw.origin.node_index,
        }
    }

    pub const fn renderer_layer_key(&self) -> u32 {
        match self {
            Self::Image(draw) => draw.renderer_layer_key(),
            Self::SliceCell(draw) => draw.draw.renderer_layer_key(),
            Self::NumberGlyph(draw) => draw.draw.renderer_layer_key(),
            Self::Fennel(draw) => draw.order.renderer_layer_key,
        }
    }
}

/// One logical draw command before the adjacent-packet merge. Image, active
/// Slice cell, and drawable Number glyph each contribute one format-14
/// command; Fennel contributes one format-13 command per texture batch in the
/// exact `sub_7C7F90` traversal order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeTargetCommandSource {
    Image {
        runtime_draw_index: usize,
    },
    SliceCell {
        runtime_draw_index: usize,
    },
    NumberGlyph {
        runtime_draw_index: usize,
    },
    FennelBatch {
        runtime_draw_index: usize,
        batch_index: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DrawTopology {
    TriangleStrip,
    TriangleList,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeTargetCommand {
    pub sources: Vec<RuntimeTargetCommandSource>,
    pub state: SrdDrawState,
    pub order: DrawOrder,
    pub topology: DrawTopology,
    pub vertex_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeSrdStrip<'a> {
    pub state: &'a SrdDraw,
    pub vertices: Vec<SrdVertex>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeFennelList<'a> {
    pub state: &'a FennelDraw,
    pub texture_token: u32,
    pub vertices: Vec<FennelRenderVertex>,
}
mod compiler;
mod font_resources;
mod submission;
#[cfg(test)]
use compiler::fennel_textbox_transform;
#[cfg(test)]
use submission::{
    AdjacentMergeCommand, AdjacentMergeKey, append_srd_quad_strip_vertices, merge_adjacent_commands,
};

pub use compiler::*;
pub use font_resources::*;
pub use submission::*;

include!("tests.rs");
