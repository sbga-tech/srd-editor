use std::collections::BTreeMap;
use std::fmt;

use crate::attribute::CastAttributeValue;
use crate::csli::{SliceQuad, build_slice_render_quad, multiply_color_game, slice_vertex_colors};
use crate::fennel::{
    FENNEL_DEFAULT_D_VALUE, FENNEL_DEFAULT_REPEAT_SPACE_COUNT, FennelFontSlotRegistry,
    FennelFontSlotRequest, FennelNormalDrawInput, FennelOwnedTextureBatch, FennelRenderVertex,
    FennelResolvedGlyph, FennelSrdDrawPreparation, FennelSrdDrawPreparationInput,
    FennelStaticTextProperties, build_fennel_normal_vertex_batches,
    build_fennel_plain_record_stream_with_font_slots, build_fennel_srd_repeated_text,
    fennel_default_draw_packet, fennel_font_param_effect_color, fennel_srd_font_style,
    layout_fennel_static_srd_explicit_flags, layout_fennel_static_srd_font_param,
    measure_fennel_srd_text_size_mode0, prepare_fennel_srd_draw, prepare_fennel_srd_runtime_text,
};
use crate::image::{
    ImageDefinition, ImageReferenceChannel, SrdTextureBindingSource,
    premultiply_additive_color_game,
};
use crate::number::{NumberGlyphRecord, NumberGlyphSegment};
use crate::projection::{
    Matrix4x4, cast_overlaps_render_target_game, compose_screen_matrix_game,
    identity_matrix4x4_game, inverse_matrix4x4_game, mul_matrix4x4_game,
};
use crate::reference_runtime::{
    ProjectLayerRuntimeState, ProjectRuntime, ReferenceLayerParent, RuntimeWorldState,
};
use crate::render::{
    CeylonDepthState, CeylonDrawPacketPresetState, CeylonRasterState,
    CeylonSrdFixedShaderConstants, SrdD3d9BlendPreset, SrdQuadDraw, SrdRenderVertex,
    apply_srd_image_alpha_stencil_packet_fields, apply_srd_image_field_0c_shader_bits,
    apply_srd_special_depth_packet_fields, ceylon_d3d9_blend_preset,
    select_srd_image_render_preset,
};
use crate::ruhuna::RuhunaRuntimeFont;
use crate::scene::{AnimationSetDefinition, Layer, Project, ReferenceTarget};
use crate::shader::CEYLON_SIMPLE_SHADER_KEY_LENGTH;
use crate::shader_bytecode::embedded_simple_shader_pair;
use crate::target_pass::{
    EvidenceScenePassProfile, EvidenceSrdType1TargetFilter,
    build_evidence_srd_scene_submission_indices,
};
use crate::texture::{TextureList, TextureSamplerState};
use crate::transform::Affine3x4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SrdDrawError(pub String);

impl fmt::Display for SrdDrawError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for SrdDrawError {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EvidenceCompleteSrdDraw {
    pub owner: ReferenceLayerParent,
    pub scene_index: usize,
    pub layer_index: usize,
    pub node_index: usize,
    pub is_2d: bool,
    pub renderer_layer_key: u32,
    pub shader_key: [u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH],
    pub quad: SrdQuadDraw,
    pub packet: CeylonDrawPacketPresetState,
    pub fixed_constants: CeylonSrdFixedShaderConstants,
    pub blend: SrdD3d9BlendPreset,
    pub raster: CeylonRasterState,
    pub depth: CeylonDepthState,
    pub texture_bindings: [Option<EvidenceSrdTextureBinding>; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvidenceSrdTextureBinding {
    pub texture_index: usize,
    pub sampler: TextureSamplerState,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceCompleteFennelDraw {
    pub owner: ReferenceLayerParent,
    pub scene_index: usize,
    pub layer_index: usize,
    pub node_index: usize,
    pub font_name: Vec<u8>,
    pub is_2d: bool,
    pub renderer_layer_key: u32,
    pub packet: CeylonDrawPacketPresetState,
    pub fixed_constants: CeylonSrdFixedShaderConstants,
    pub batches: Vec<FennelOwnedTextureBatch>,
}

/// One low-level quad emitted by `srd_render_slice_cast` for an active CSLI
/// cell. The binary executes `srd_begin_quad_draw` and the matching submission
/// once per active cell, so cells remain separate runtime draws even when they
/// belong to the same CAST.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EvidenceCompleteSliceCellDraw {
    pub cell_index: usize,
    pub draw: EvidenceCompleteSrdDraw,
}

/// One drawable glyph emitted by the first history record of a freshly
/// constructed SrNumberCast. The binary calls `srd_begin_quad_draw` once per
/// non-negative glyph mapping, so glyph identity remains part of the runtime
/// stream even when adjacent packet records later merge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EvidenceCompleteNumberGlyphDraw {
    pub glyph_index: i16,
    pub segment: NumberGlyphSegment,
    pub draw: EvidenceCompleteSrdDraw,
}

/// Draw payloads produced at each CAST render invocation. This preserves the
/// runtime layer/CAST/RefCast recursion order, but it is not yet the later
/// renderer target-queue sort/submission order.
#[derive(Debug, Clone, PartialEq)]
pub enum EvidenceCompleteRuntimeCastDraw {
    Image(EvidenceCompleteSrdDraw),
    SliceCell(EvidenceCompleteSliceCellDraw),
    NumberGlyph(EvidenceCompleteNumberGlyphDraw),
    Fennel(EvidenceCompleteFennelDraw),
}

impl EvidenceCompleteRuntimeCastDraw {
    pub const fn owner(&self) -> ReferenceLayerParent {
        match self {
            Self::Image(draw) => draw.owner,
            Self::SliceCell(draw) => draw.draw.owner,
            Self::NumberGlyph(draw) => draw.draw.owner,
            Self::Fennel(draw) => draw.owner,
        }
    }

    pub const fn node_index(&self) -> usize {
        match self {
            Self::Image(draw) => draw.node_index,
            Self::SliceCell(draw) => draw.draw.node_index,
            Self::NumberGlyph(draw) => draw.draw.node_index,
            Self::Fennel(draw) => draw.node_index,
        }
    }

    pub const fn renderer_layer_key(&self) -> u32 {
        match self {
            Self::Image(draw) => draw.renderer_layer_key,
            Self::SliceCell(draw) => draw.draw.renderer_layer_key,
            Self::NumberGlyph(draw) => draw.draw.renderer_layer_key,
            Self::Fennel(draw) => draw.renderer_layer_key,
        }
    }
}

/// One logical draw command before the adjacent-packet merge. Image, active
/// Slice cell, and drawable Number glyph each contribute one format-14
/// command; Fennel contributes one format-13 command per texture batch in the
/// exact `sub_7C7F90` traversal order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceRuntimeTargetCommandSource {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceMergedRuntimeTargetCommand {
    pub sources: Vec<EvidenceRuntimeTargetCommandSource>,
    pub packet: CeylonDrawPacketPresetState,
    pub renderer_layer_key: u32,
    pub vertex_format: u32,
    pub primitive_type: u32,
    pub vertex_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceMergedRuntimeSrdStrip<'a> {
    pub state: &'a EvidenceCompleteSrdDraw,
    pub vertices: Vec<SrdRenderVertex>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceMergedRuntimeFennelList<'a> {
    pub state: &'a EvidenceCompleteFennelDraw,
    pub texture_token: u32,
    pub vertices: Vec<FennelRenderVertex>,
}

/// Materializes one merged format-14/type-4 command using the exact connector
/// copies at `ceylon_enqueue_draw_packet` `0x670E19..0x670E3D`. At every
/// boundary the previous last vertex and next first vertex are duplicated,
/// producing `last,last,first,first` around the join.
pub fn build_evidence_merged_runtime_srd_strip<'a>(
    draws: &'a [EvidenceCompleteRuntimeCastDraw],
    command: &EvidenceMergedRuntimeTargetCommand,
    mut include: impl FnMut(EvidenceRuntimeTargetCommandSource) -> bool,
) -> Result<Option<EvidenceMergedRuntimeSrdStrip<'a>>, SrdDrawError> {
    if command.vertex_format != 14 || command.primitive_type != 4 {
        return Err(SrdDrawError(format!(
            "runtime target command is format {}/primitive {}, not an SRD quad strip",
            command.vertex_format, command.primitive_type
        )));
    }

    let mut state = None;
    let mut vertices = Vec::new();
    let mut included_source_count = 0usize;
    for source in command
        .sources
        .iter()
        .copied()
        .filter(|source| include(*source))
    {
        let (runtime_draw_index, draw) = match source {
            EvidenceRuntimeTargetCommandSource::Image { runtime_draw_index } => {
                let Some(EvidenceCompleteRuntimeCastDraw::Image(draw)) =
                    draws.get(runtime_draw_index)
                else {
                    return Err(SrdDrawError(format!(
                        "runtime target Image source {runtime_draw_index} does not match its draw"
                    )));
                };
                (runtime_draw_index, draw)
            }
            EvidenceRuntimeTargetCommandSource::SliceCell { runtime_draw_index } => {
                let Some(EvidenceCompleteRuntimeCastDraw::SliceCell(cell)) =
                    draws.get(runtime_draw_index)
                else {
                    return Err(SrdDrawError(format!(
                        "runtime target SliceCell source {runtime_draw_index} does not match its draw"
                    )));
                };
                (runtime_draw_index, &cell.draw)
            }
            EvidenceRuntimeTargetCommandSource::NumberGlyph { runtime_draw_index } => {
                let Some(EvidenceCompleteRuntimeCastDraw::NumberGlyph(glyph)) =
                    draws.get(runtime_draw_index)
                else {
                    return Err(SrdDrawError(format!(
                        "runtime target NumberGlyph source {runtime_draw_index} does not match its draw"
                    )));
                };
                (runtime_draw_index, &glyph.draw)
            }
            EvidenceRuntimeTargetCommandSource::FennelBatch { .. } => {
                return Err(SrdDrawError(
                    "format-14 SRD strip contains a Fennel batch source".to_string(),
                ));
            }
        };
        if draw.packet != command.packet || draw.renderer_layer_key != command.renderer_layer_key {
            return Err(SrdDrawError(format!(
                "runtime draw {runtime_draw_index} no longer matches its merged command state"
            )));
        }
        if vertices.is_empty() {
            state = Some(draw);
        }
        append_srd_quad_strip_vertices(&mut vertices, &draw.quad.vertices);
        included_source_count += 1;
    }

    let Some(state) = state else {
        return Ok(None);
    };
    if included_source_count == command.sources.len() && vertices.len() != command.vertex_count {
        return Err(SrdDrawError(format!(
            "materialized SRD strip has {} vertices but planner recorded {}",
            vertices.len(),
            command.vertex_count
        )));
    }
    Ok(Some(EvidenceMergedRuntimeSrdStrip { state, vertices }))
}

fn append_srd_quad_strip_vertices(
    destination: &mut Vec<SrdRenderVertex>,
    quad: &[SrdRenderVertex; 4],
) {
    if let Some(previous_last) = destination.last().copied() {
        destination.push(previous_last);
        destination.push(quad[0]);
    }
    destination.extend_from_slice(quad);
}

/// Materializes one format-13/type-3 command. The binary merge branch at
/// `0x670E66` only adds the new vertex count because batch vertices are already
/// contiguous; no connector or reordering is introduced.
pub fn build_evidence_merged_runtime_fennel_list<'a>(
    draws: &'a [EvidenceCompleteRuntimeCastDraw],
    command: &EvidenceMergedRuntimeTargetCommand,
) -> Result<EvidenceMergedRuntimeFennelList<'a>, SrdDrawError> {
    if command.vertex_format != 13 || command.primitive_type != 3 {
        return Err(SrdDrawError(format!(
            "runtime target command is format {}/primitive {}, not a Fennel triangle list",
            command.vertex_format, command.primitive_type
        )));
    }

    let mut state = None;
    let mut texture_token = None;
    let mut vertices = Vec::with_capacity(command.vertex_count);
    for source in command.sources.iter().copied() {
        let EvidenceRuntimeTargetCommandSource::FennelBatch {
            runtime_draw_index,
            batch_index,
        } = source
        else {
            return Err(SrdDrawError(
                "format-13 Fennel list contains an SRD quad source".to_string(),
            ));
        };
        let Some(EvidenceCompleteRuntimeCastDraw::Fennel(draw)) = draws.get(runtime_draw_index)
        else {
            return Err(SrdDrawError(format!(
                "runtime target Fennel source {runtime_draw_index} does not match its draw"
            )));
        };
        let batch = draw.batches.get(batch_index).ok_or_else(|| {
            SrdDrawError(format!(
                "runtime target Fennel batch {runtime_draw_index}/{batch_index} is outside its draw"
            ))
        })?;
        if draw.packet != command.packet || draw.renderer_layer_key != command.renderer_layer_key {
            return Err(SrdDrawError(format!(
                "runtime Fennel draw {runtime_draw_index} no longer matches its merged command state"
            )));
        }
        if let Some(expected) = texture_token {
            if batch.texture_token != expected {
                return Err(SrdDrawError(format!(
                    "merged Fennel command changes texture token from {expected:#010x} to {:#010x}",
                    batch.texture_token
                )));
            }
        } else {
            state = Some(draw);
            texture_token = Some(batch.texture_token);
        }
        vertices.extend_from_slice(&batch.vertices);
    }

    let state = state.ok_or_else(|| SrdDrawError("merged Fennel command is empty".to_string()))?;
    let texture_token = texture_token.expect("non-empty Fennel command has a texture token");
    if vertices.len() != command.vertex_count {
        return Err(SrdDrawError(format!(
            "materialized Fennel list has {} vertices but planner recorded {}",
            vertices.len(),
            command.vertex_count
        )));
    }
    Ok(EvidenceMergedRuntimeFennelList {
        state,
        texture_token,
        vertices,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EvidenceMergeTextureIdentity {
    Srd(EvidenceSrdTextureBinding),
    Fennel(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EvidenceAdjacentMergeKey {
    packet: CeylonDrawPacketPresetState,
    renderer_layer_key: u32,
    vertex_format: u32,
    primitive_type: u32,
    textures: [Option<EvidenceMergeTextureIdentity>; 3],
    packet_matrix_prefix: Option<[u32; 15]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EvidenceAdjacentMergeCommand {
    source: EvidenceRuntimeTargetCommandSource,
    key: EvidenceAdjacentMergeKey,
    vertex_count: usize,
}

/// Applies an already selected target's ScenePass/SceneModel profile to the
/// runtime CAST stream. The caller remains responsible for proving that the
/// target filter admitted these commands; this function only closes the exact
/// target-local classification and stable flush order.
pub fn build_evidence_runtime_target_submission(
    draws: &[EvidenceCompleteRuntimeCastDraw],
    profile: &EvidenceScenePassProfile,
) -> Result<Vec<EvidenceRuntimeTargetCommandSource>, SrdDrawError> {
    build_evidence_runtime_target_submission_impl(draws, profile, |_| true)
}

/// Applies the proven type-1 global-queue target filter before target-local
/// pass classification. A disabled/non-dispatched target produces an empty
/// submission; rejected Fennel batches do not disturb the order of admitted
/// commands.
pub fn build_evidence_filtered_runtime_target_submission(
    draws: &[EvidenceCompleteRuntimeCastDraw],
    profile: &EvidenceScenePassProfile,
    filter: EvidenceSrdType1TargetFilter,
) -> Result<Vec<EvidenceRuntimeTargetCommandSource>, SrdDrawError> {
    build_evidence_runtime_target_submission_impl(draws, profile, |packet| filter.accepts(packet))
}

/// Reproduces the adjacent-record merge in `ceylon_enqueue_draw_packet` for
/// the evidence-complete ordinary SRD/Fennel path, then applies the target
/// filter and ScenePass order.
///
/// The input stream belongs to one SrPlayer, so its enqueue target pointer and
/// packet `+0x80..+0x8B` host block are shared. The current draw builders also
/// exclude explicit texture overrides and renderer special-depth mode. Stencil
/// packets are rejected here because their renderer `+0x198` sequence byte is
/// outside the current runtime draw record; silently under-merging them would
/// not reproduce the binary.
pub fn build_evidence_filtered_merged_runtime_target_submission(
    draws: &[EvidenceCompleteRuntimeCastDraw],
    profile: &EvidenceScenePassProfile,
    filter: EvidenceSrdType1TargetFilter,
) -> Result<Vec<EvidenceMergedRuntimeTargetCommand>, SrdDrawError> {
    let commands = build_evidence_adjacent_merge_commands(draws)?;
    let groups = merge_evidence_adjacent_commands(commands);

    let mut admitted_groups = Vec::new();
    let mut packets = Vec::new();
    for group in groups {
        if filter.accepts(group.packet) {
            packets.push(group.packet);
            admitted_groups.push(group);
        }
    }

    let submission_indices = build_evidence_srd_scene_submission_indices(&packets, profile)
        .map_err(|error| SrdDrawError(error.to_string()))?;
    Ok(submission_indices
        .into_iter()
        .map(|command_index| admitted_groups[command_index].clone())
        .collect())
}

fn build_evidence_adjacent_merge_commands(
    draws: &[EvidenceCompleteRuntimeCastDraw],
) -> Result<Vec<EvidenceAdjacentMergeCommand>, SrdDrawError> {
    let mut commands = Vec::new();
    for (runtime_draw_index, draw) in draws.iter().enumerate() {
        match draw {
            EvidenceCompleteRuntimeCastDraw::Image(draw) => {
                validate_evidence_merge_packet(draw.packet, runtime_draw_index)?;
                let textures = draw
                    .texture_bindings
                    .map(|binding| binding.map(EvidenceMergeTextureIdentity::Srd));
                commands.push(EvidenceAdjacentMergeCommand {
                    source: EvidenceRuntimeTargetCommandSource::Image { runtime_draw_index },
                    key: EvidenceAdjacentMergeKey {
                        packet: draw.packet,
                        renderer_layer_key: draw.renderer_layer_key,
                        vertex_format: 14,
                        primitive_type: 4,
                        textures,
                        packet_matrix_prefix: packet_matrix_prefix(
                            draw.is_2d,
                            draw.fixed_constants.vertex_c0_c3_world,
                        ),
                    },
                    vertex_count: 4,
                });
            }
            EvidenceCompleteRuntimeCastDraw::SliceCell(cell) => {
                let draw = &cell.draw;
                validate_evidence_merge_packet(draw.packet, runtime_draw_index)?;
                let textures = draw
                    .texture_bindings
                    .map(|binding| binding.map(EvidenceMergeTextureIdentity::Srd));
                commands.push(EvidenceAdjacentMergeCommand {
                    source: EvidenceRuntimeTargetCommandSource::SliceCell { runtime_draw_index },
                    key: EvidenceAdjacentMergeKey {
                        packet: draw.packet,
                        renderer_layer_key: draw.renderer_layer_key,
                        vertex_format: 14,
                        primitive_type: 4,
                        textures,
                        packet_matrix_prefix: packet_matrix_prefix(
                            draw.is_2d,
                            draw.fixed_constants.vertex_c0_c3_world,
                        ),
                    },
                    vertex_count: 4,
                });
            }
            EvidenceCompleteRuntimeCastDraw::NumberGlyph(glyph) => {
                let draw = &glyph.draw;
                validate_evidence_merge_packet(draw.packet, runtime_draw_index)?;
                let textures = draw
                    .texture_bindings
                    .map(|binding| binding.map(EvidenceMergeTextureIdentity::Srd));
                commands.push(EvidenceAdjacentMergeCommand {
                    source: EvidenceRuntimeTargetCommandSource::NumberGlyph { runtime_draw_index },
                    key: EvidenceAdjacentMergeKey {
                        packet: draw.packet,
                        renderer_layer_key: draw.renderer_layer_key,
                        vertex_format: 14,
                        primitive_type: 4,
                        textures,
                        packet_matrix_prefix: packet_matrix_prefix(
                            draw.is_2d,
                            draw.fixed_constants.vertex_c0_c3_world,
                        ),
                    },
                    vertex_count: 4,
                });
            }
            EvidenceCompleteRuntimeCastDraw::Fennel(draw) => {
                validate_evidence_merge_packet(draw.packet, runtime_draw_index)?;
                let packet_matrix_prefix =
                    packet_matrix_prefix(draw.is_2d, draw.fixed_constants.vertex_c0_c3_world);
                for (batch_index, batch) in draw.batches.iter().enumerate() {
                    commands.push(EvidenceAdjacentMergeCommand {
                        source: EvidenceRuntimeTargetCommandSource::FennelBatch {
                            runtime_draw_index,
                            batch_index,
                        },
                        key: EvidenceAdjacentMergeKey {
                            packet: draw.packet,
                            renderer_layer_key: draw.renderer_layer_key,
                            vertex_format: 13,
                            primitive_type: 3,
                            textures: [
                                Some(EvidenceMergeTextureIdentity::Fennel(batch.texture_token)),
                                None,
                                None,
                            ],
                            packet_matrix_prefix,
                        },
                        vertex_count: batch.vertices.len(),
                    });
                }
            }
        }
    }
    Ok(commands)
}

fn validate_evidence_merge_packet(
    packet: CeylonDrawPacketPresetState,
    runtime_draw_index: usize,
) -> Result<(), SrdDrawError> {
    if packet.flags_0c & 0x100 != 0 {
        return Err(SrdDrawError(format!(
            "runtime draw {runtime_draw_index} uses stencil state whose renderer sequence byte is not recorded for exact adjacent merging"
        )));
    }
    if packet.field_2c != 0 {
        return Err(SrdDrawError(format!(
            "runtime draw {runtime_draw_index} uses packet field_2c={} outside the ordinary merge profile",
            packet.field_2c
        )));
    }
    Ok(())
}

fn packet_matrix_prefix(is_2d: bool, matrix: Matrix4x4) -> Option<[u32; 15]> {
    if is_2d {
        return None;
    }
    let mut prefix = [0u32; 15];
    for (destination, value) in prefix.iter_mut().zip(matrix.rows.iter().flatten().take(15)) {
        *destination = value.to_bits();
    }
    Some(prefix)
}

fn merge_evidence_adjacent_commands(
    commands: Vec<EvidenceAdjacentMergeCommand>,
) -> Vec<EvidenceMergedRuntimeTargetCommand> {
    let mut groups: Vec<(EvidenceAdjacentMergeKey, EvidenceMergedRuntimeTargetCommand)> =
        Vec::new();
    for command in commands {
        if let Some((previous_key, previous)) = groups.last_mut()
            && *previous_key == command.key
        {
            previous.sources.push(command.source);
            previous.vertex_count += if previous.primitive_type == 4 {
                command.vertex_count + 2
            } else {
                command.vertex_count
            };
            continue;
        }

        let packet = command.key.packet;
        let renderer_layer_key = command.key.renderer_layer_key;
        let vertex_format = command.key.vertex_format;
        let primitive_type = command.key.primitive_type;
        groups.push((
            command.key,
            EvidenceMergedRuntimeTargetCommand {
                sources: vec![command.source],
                packet,
                renderer_layer_key,
                vertex_format,
                primitive_type,
                vertex_count: command.vertex_count,
            },
        ));
    }
    groups.into_iter().map(|(_, group)| group).collect()
}

fn build_evidence_runtime_target_submission_impl(
    draws: &[EvidenceCompleteRuntimeCastDraw],
    profile: &EvidenceScenePassProfile,
    mut admits: impl FnMut(CeylonDrawPacketPresetState) -> bool,
) -> Result<Vec<EvidenceRuntimeTargetCommandSource>, SrdDrawError> {
    let mut sources = Vec::new();
    let mut packets = Vec::new();
    for (runtime_draw_index, draw) in draws.iter().enumerate() {
        match draw {
            EvidenceCompleteRuntimeCastDraw::Image(draw) => {
                if admits(draw.packet) {
                    sources.push(EvidenceRuntimeTargetCommandSource::Image { runtime_draw_index });
                    packets.push(draw.packet);
                }
            }
            EvidenceCompleteRuntimeCastDraw::SliceCell(cell) => {
                if admits(cell.draw.packet) {
                    sources
                        .push(EvidenceRuntimeTargetCommandSource::SliceCell { runtime_draw_index });
                    packets.push(cell.draw.packet);
                }
            }
            EvidenceCompleteRuntimeCastDraw::NumberGlyph(glyph) => {
                if admits(glyph.draw.packet) {
                    sources.push(EvidenceRuntimeTargetCommandSource::NumberGlyph {
                        runtime_draw_index,
                    });
                    packets.push(glyph.draw.packet);
                }
            }
            EvidenceCompleteRuntimeCastDraw::Fennel(draw) => {
                for batch_index in 0..draw.batches.len() {
                    if admits(draw.packet) {
                        sources.push(EvidenceRuntimeTargetCommandSource::FennelBatch {
                            runtime_draw_index,
                            batch_index,
                        });
                        packets.push(draw.packet);
                    }
                }
            }
        }
    }

    let submission_indices = build_evidence_srd_scene_submission_indices(&packets, profile)
        .map_err(|error| SrdDrawError(error.to_string()))?;
    Ok(submission_indices
        .into_iter()
        .map(|command_index| sources[command_index])
        .collect())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FennelTextFontRole {
    Primary,
    Ruby,
    Outline,
    OutlineRuby,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FennelFontResourceRequest {
    pub scene_index: usize,
    pub layer_index: usize,
    pub node_index: usize,
    pub role: FennelTextFontRole,
    pub name: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FennelFontResourceAssignment {
    pub request: FennelFontResourceRequest,
    pub slot: FennelFontSlotRequest,
}

/// Collects the font-resource requests issued by `sub_AAE6C0` while it walks
/// the player's original runtime scene table.
///
/// The binary traverses scenes, layers, and each layer's CAST vector in their
/// construction order. For every SrTextCast it requests the primary font
/// first, then visits the selected CATR records in source order. Only the
/// exact case-sensitive string keys below call the same four-slot TextCast
/// font loader. Empty strings reach `sub_1088590` but do not issue a resource
/// request, so they are omitted here.
///
/// Independently constructed reference layers do not add requests here.
/// `srd_player_impl_load_project` resolves and constructs those copies before
/// this traversal, but `sub_AAE6C0` still walks only the original runtime scene
/// table. Every copied TextCast was rebuilt from a target LAYR already present
/// in that table and resolves its TextBox through the shared renderer resource
/// tree at draw time.
///
/// This is not an assertion that the process-global FontManager was empty
/// before this player loaded. Apply the returned requests to a registry that
/// already contains any host resources whose earlier lifetime is known.
pub fn collect_fennel_font_resource_requests(
    project: &Project,
) -> Result<Vec<FennelFontResourceRequest>, SrdDrawError> {
    let mut requests = Vec::new();
    for (scene_index, scene) in project.scenes.iter().enumerate() {
        for (layer_index, layer) in scene.layers.iter().enumerate() {
            for node_index in 0..layer.nodes.len() {
                let Some(image) = layer.image_by_node.get(node_index).and_then(Option::as_ref)
                else {
                    continue;
                };
                if !image.creates_text_cast() {
                    continue;
                }
                let Some(text) = image.text.as_ref() else {
                    return Err(SrdDrawError(format!(
                        "SCN[{scene_index}]/LAYR[{layer_index}]/NODE[{node_index}] creates an SrTextCast without a TEXT definition"
                    )));
                };
                let font_index = usize::try_from(text.font_index.unwrap_or(-1)).map_err(|_| {
                    SrdDrawError(format!(
                        "SCN[{scene_index}]/LAYR[{layer_index}]/NODE[{node_index}] has an invalid TextCast font index"
                    ))
                })?;
                let font = project.fonts.get(font_index).ok_or_else(|| {
                    SrdDrawError(format!(
                        "SCN[{scene_index}]/LAYR[{layer_index}]/NODE[{node_index}] TextCast font index {font_index} is outside PROJ"
                    ))
                })?;
                push_fennel_font_resource_request(
                    &mut requests,
                    scene_index,
                    layer_index,
                    node_index,
                    FennelTextFontRole::Primary,
                    &font.name,
                );

                let Some(attribute_list_index) = layer
                    .cast_attribute_list_by_node
                    .get(node_index)
                    .and_then(|index| *index)
                else {
                    continue;
                };
                let attribute_list = layer
                    .cast_attribute_lists
                    .get(attribute_list_index)
                    .ok_or_else(|| {
                        SrdDrawError(format!(
                            "SCN[{scene_index}]/LAYR[{layer_index}]/NODE[{node_index}] CATR index {attribute_list_index} is outside the parsed list table"
                        ))
                    })?;
                for attribute in &attribute_list.attributes {
                    let Some(role) = (match attribute.name.as_slice() {
                        b"rubyFont" => Some(FennelTextFontRole::Ruby),
                        b"rfzOutlineFont" => Some(FennelTextFontRole::Outline),
                        b"rfzOutlineRubyFont" => Some(FennelTextFontRole::OutlineRuby),
                        _ => None,
                    }) else {
                        continue;
                    };
                    let CastAttributeValue::String(name) = &attribute.value else {
                        return Err(SrdDrawError(format!(
                            "SCN[{scene_index}]/LAYR[{layer_index}]/NODE[{node_index}] CATR {:?} uses an unported non-string font-resource value",
                            String::from_utf8_lossy(&attribute.name)
                        )));
                    };
                    push_fennel_font_resource_request(
                        &mut requests,
                        scene_index,
                        layer_index,
                        node_index,
                        role,
                        name,
                    );
                }
            }
        }
    }
    Ok(requests)
}

fn push_fennel_font_resource_request(
    requests: &mut Vec<FennelFontResourceRequest>,
    scene_index: usize,
    layer_index: usize,
    node_index: usize,
    role: FennelTextFontRole,
    name: &[u8],
) {
    if !name.is_empty() {
        requests.push(FennelFontResourceRequest {
            scene_index,
            layer_index,
            node_index,
            role,
            name: name.to_vec(),
        });
    }
}

pub fn assign_fennel_font_resource_requests(
    registry: &mut FennelFontSlotRegistry<Vec<u8>>,
    requests: impl IntoIterator<Item = FennelFontResourceRequest>,
) -> Vec<FennelFontResourceAssignment> {
    requests
        .into_iter()
        .map(|request| {
            let slot = registry.request(request.name.clone());
            FennelFontResourceAssignment { request, slot }
        })
        .collect()
}

/// Inputs owned by the scene/target hosting an SrPlayer, rather than by the
/// SRD file itself. There is deliberately no `Default`: an independent SRD
/// does not identify a unique game target, Camera, or scene-node placement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SrdRendererProjectTargetContext {
    /// Projection*View of the target resolved from SrPlayer property 2
    /// `TargetScene` while `srd_renderer_configure_project_camera` runs.
    pub projection_view: Matrix4x4,
    /// Width/height read from that resolved target. The width is passed as the
    /// SRD Camera Aspect property by the original renderer.
    pub render_size: [u32; 2],
}

impl SrdRendererProjectTargetContext {
    pub const fn new(projection_view: Matrix4x4, render_size: [u32; 2]) -> Self {
        Self {
            projection_view,
            render_size,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SrdHostDrawContext {
    pub first_calc_matrix: Affine3x4,
    /// Root `SrRenderer+0x198` key established by the concrete SrPlayer host.
    pub renderer_layer_key: u32,
    /// Target resolved from the player's `TargetScene` property during
    /// renderer preparation. This is independent from the scene that later
    /// receives a globally queued packet. `None` models the proven null lookup
    /// used by the shipped Advertise/Common host profiles.
    pub renderer_project_target: Option<SrdRendererProjectTargetContext>,
    /// Projection*View of the scene that actually receives and submits the
    /// packet, including packets routed through the global queue.
    pub target_projection_view: Matrix4x4,
    pub target_screen_size: [u32; 2],
}

impl SrdHostDrawContext {
    pub const fn new(
        first_calc_matrix: Affine3x4,
        renderer_layer_key: u32,
        renderer_project_target: Option<SrdRendererProjectTargetContext>,
        target_projection_view: Matrix4x4,
        target_screen_size: [u32; 2],
    ) -> Self {
        Self {
            first_calc_matrix,
            renderer_layer_key,
            renderer_project_target,
            target_projection_view,
            target_screen_size,
        }
    }
}

/// Builds only the initial ImageCast subset whose complete Simple shader pair
/// and packet/device inputs are proven. TEXT, explicit texture overrides,
/// special CAST matrix branches, alpha-test/stencil base contexts and
/// unsupported shader keys are rejected or excluded explicitly.
pub fn build_evidence_complete_initial_image_draws(
    project: &Project,
    textures: &TextureList,
    scene_index: usize,
    host: SrdHostDrawContext,
) -> Result<Vec<EvidenceCompleteSrdDraw>, SrdDrawError> {
    build_evidence_complete_image_draws(project, textures, scene_index, host, None)
}

pub fn build_evidence_complete_animation_set_image_draws(
    project: &Project,
    textures: &TextureList,
    scene_index: usize,
    animation_set_index: usize,
    frame: f32,
    host: SrdHostDrawContext,
) -> Result<Vec<EvidenceCompleteSrdDraw>, SrdDrawError> {
    let mut runtime = ProjectRuntime::new(project)
        .map_err(|error| SrdDrawError(format!("failed to construct SRD runtime: {error}")))?;
    runtime
        .apply_animation_set(project, textures, scene_index, animation_set_index, frame)
        .map_err(|error| SrdDrawError(error.to_string()))?;
    let runtime_layers = runtime
        .project_layers
        .get(scene_index)
        .ok_or_else(|| SrdDrawError(format!("scene index {scene_index} is outside the runtime")))?;
    build_evidence_complete_image_draws(project, textures, scene_index, host, Some(runtime_layers))
}

/// Builds ImageCast draws for original and independently copied reference
/// layers in the exact recursive CAST traversal order. The caller still must
/// merge TextCast draws before using this as a complete Composition stream.
pub fn build_evidence_complete_initial_reference_image_draws(
    project: &Project,
    textures: &TextureList,
    scene_index: usize,
    host: SrdHostDrawContext,
) -> Result<Vec<EvidenceCompleteSrdDraw>, SrdDrawError> {
    validate_host_draw_context(host)?;
    let runtime = ProjectRuntime::new(project)
        .map_err(|error| SrdDrawError(format!("failed to construct SRD runtime: {error}")))?;
    build_evidence_complete_reference_image_draws_from_runtime(
        project,
        textures,
        scene_index,
        host,
        &runtime,
    )
}

pub fn build_evidence_complete_animation_set_reference_image_draws(
    project: &Project,
    textures: &TextureList,
    scene_index: usize,
    animation_set_index: usize,
    frame: f32,
    host: SrdHostDrawContext,
) -> Result<Vec<EvidenceCompleteSrdDraw>, SrdDrawError> {
    validate_host_draw_context(host)?;
    let mut runtime = ProjectRuntime::new(project)
        .map_err(|error| SrdDrawError(format!("failed to construct SRD runtime: {error}")))?;
    runtime
        .apply_animation_set(project, textures, scene_index, animation_set_index, frame)
        .map_err(|error| SrdDrawError(error.to_string()))?;
    build_evidence_complete_reference_image_draws_from_runtime(
        project,
        textures,
        scene_index,
        host,
        &runtime,
    )
}

fn build_evidence_complete_reference_image_draws_from_runtime(
    project: &Project,
    textures: &TextureList,
    scene_index: usize,
    host: SrdHostDrawContext,
    runtime: &ProjectRuntime,
) -> Result<Vec<EvidenceCompleteSrdDraw>, SrdDrawError> {
    let renderer_inverse_camera_view = renderer_inverse_camera_view(project, host);
    let worlds = runtime
        .compose_world_states(
            project,
            host.first_calc_matrix,
            renderer_inverse_camera_view,
        )
        .map_err(|error| SrdDrawError(error.to_string()))?;
    if project.scenes.get(scene_index).is_none() {
        return Err(SrdDrawError(format!(
            "scene index {scene_index} is outside the project"
        )));
    }

    let camera_bridge = renderer_project_camera_bridge(project, host);
    let project_screen = renderer_project_screen_matrix(host, camera_bridge)?;
    let identity = identity_matrix4x4_game();
    let mut packet_current_matrix = identity;
    let mut draws = Vec::new();

    for entry in runtime
        .references
        .plan
        .structural_cast_draw_order(project, scene_index, host.renderer_layer_key)
        .map_err(|error| SrdDrawError(error.to_string()))?
    {
        let layer = &project.scenes[entry.source.scene_index].layers[entry.source.layer_index];
        let layer_worlds = worlds.layer(entry.owner).ok_or_else(|| {
            SrdDrawError(format!(
                "runtime draw owner {:?} has no composed world state",
                entry.owner
            ))
        })?;
        let cast_world = *layer_worlds.casts.get(entry.node_index).ok_or_else(|| {
            SrdDrawError(format!(
                "SCN[{}]/LAYR[{}]/NODE[{}] has no composed CAST world state",
                entry.source.scene_index, entry.source.layer_index, entry.node_index
            ))
        })?;
        let image_state = runtime_image_state_for_owner(&runtime, entry.owner, entry.node_index)?;
        if let Some(draw) = build_evidence_complete_image_draw_for_runtime_cast(
            textures,
            entry.owner,
            entry.source,
            layer,
            entry.node_index,
            entry.renderer_layer_key,
            layer_worlds.is_2d,
            cast_world,
            image_state,
            host,
            camera_bridge,
            project_screen.as_ref(),
            identity,
            &mut packet_current_matrix,
        )? {
            draws.push(draw);
        }
    }
    Ok(draws)
}

/// Builds the initial RFZ TextCast subset whose layout, texture batching,
/// 2D/3D vertex and matrix paths, world transform and color inputs are all
/// closed. Legacy `.sbfont` stays out of this evidence-complete path.
///
/// `font_registry` is the process-global slot state after this player's
/// requests have been applied. It may already contain earlier host resources;
/// the function uses it for both the TextCast primary slot and explicit
/// `$F[n]` switches. Runtime fonts retain caller-defined opaque texture tokens,
/// allowing the renderer to route batches across different font atlases.
#[derive(Debug, Clone, PartialEq)]
pub struct FennelSrdRuntimeTextInput {
    pub substitutions: [Vec<u8>; 8],
    pub default_d: i32,
    pub repeat_space_count: i32,
    /// Exact current SrTextCast text-state field `+0xF4`. Callers may advance
    /// it with `FennelSrdScrollState::advance`; this API does not infer a game
    /// tick rate from editor animation frames.
    pub field_f4: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FennelRuntimeTextCastKey {
    pub owner: ReferenceLayerParent,
    pub node_index: usize,
}

impl Default for FennelSrdRuntimeTextInput {
    fn default() -> Self {
        // Empty slots are an editor form default, not a claim about an
        // arbitrary game's current SrTextCast host strings. The initial draw
        // wrapper still rejects serialized substitution tokens without an
        // explicit input entry.
        Self {
            substitutions: std::array::from_fn(|_| Vec::new()),
            default_d: FENNEL_DEFAULT_D_VALUE,
            repeat_space_count: FENNEL_DEFAULT_REPEAT_SPACE_COUNT,
            field_f4: 0.0,
        }
    }
}

pub fn build_evidence_complete_initial_fennel_draws(
    project: &Project,
    scene_index: usize,
    host: SrdHostDrawContext,
    font_registry: &FennelFontSlotRegistry<Vec<u8>>,
    runtime_fonts: &BTreeMap<Vec<u8>, RuhunaRuntimeFont>,
    force_color_update: bool,
) -> Result<Vec<EvidenceCompleteFennelDraw>, SrdDrawError> {
    build_evidence_complete_fennel_draws_impl(
        project,
        scene_index,
        host,
        font_registry,
        runtime_fonts,
        force_color_update,
        None,
        None,
    )
}

/// Builds the same RFZ draw subset with explicit per-node SrTextCast host
/// inputs. Keys are `(layer_index, node_index)` within `scene_index`; absent
/// entries retain the strict initial behavior and reject `$[0]..$[7]` rather
/// than inventing substitution values.
pub fn build_evidence_complete_fennel_draws_with_runtime_text(
    project: &Project,
    scene_index: usize,
    host: SrdHostDrawContext,
    font_registry: &FennelFontSlotRegistry<Vec<u8>>,
    runtime_fonts: &BTreeMap<Vec<u8>, RuhunaRuntimeFont>,
    force_color_update: bool,
    runtime_text_inputs: &BTreeMap<(usize, usize), FennelSrdRuntimeTextInput>,
) -> Result<Vec<EvidenceCompleteFennelDraw>, SrdDrawError> {
    build_evidence_complete_fennel_draws_impl(
        project,
        scene_index,
        host,
        font_registry,
        runtime_fonts,
        force_color_update,
        None,
        Some(runtime_text_inputs),
    )
}

/// Applies one ANMS frame through the shared ProjectRuntime, then builds RFZ
/// TextCast draws with the resulting layer enable, CAST transforms/colors and
/// SrImage geometry while keeping text-host substitutions/scroll state
/// explicit.
pub fn build_evidence_complete_animation_set_fennel_draws_with_runtime_text(
    project: &Project,
    textures: &TextureList,
    scene_index: usize,
    animation_set_index: usize,
    frame: f32,
    host: SrdHostDrawContext,
    font_registry: &FennelFontSlotRegistry<Vec<u8>>,
    runtime_fonts: &BTreeMap<Vec<u8>, RuhunaRuntimeFont>,
    force_color_update: bool,
    runtime_text_inputs: &BTreeMap<(usize, usize), FennelSrdRuntimeTextInput>,
) -> Result<Vec<EvidenceCompleteFennelDraw>, SrdDrawError> {
    let mut runtime = ProjectRuntime::new(project)
        .map_err(|error| SrdDrawError(format!("failed to construct SRD runtime: {error}")))?;
    runtime
        .apply_animation_set(project, textures, scene_index, animation_set_index, frame)
        .map_err(|error| SrdDrawError(format!("failed to apply animation set: {error}")))?;
    let runtime_layers = runtime
        .project_layers
        .get(scene_index)
        .ok_or_else(|| SrdDrawError(format!("scene index {scene_index} is outside the runtime")))?;
    build_evidence_complete_fennel_draws_impl(
        project,
        scene_index,
        host,
        font_registry,
        runtime_fonts,
        force_color_update,
        Some(runtime_layers),
        Some(runtime_text_inputs),
    )
}

pub fn build_evidence_complete_initial_reference_fennel_draws(
    project: &Project,
    scene_index: usize,
    host: SrdHostDrawContext,
    font_registry: &FennelFontSlotRegistry<Vec<u8>>,
    runtime_fonts: &BTreeMap<Vec<u8>, RuhunaRuntimeFont>,
    force_color_update: bool,
    runtime_text_inputs: &BTreeMap<FennelRuntimeTextCastKey, FennelSrdRuntimeTextInput>,
) -> Result<Vec<EvidenceCompleteFennelDraw>, SrdDrawError> {
    validate_host_draw_context(host)?;
    let runtime = ProjectRuntime::new(project)
        .map_err(|error| SrdDrawError(format!("failed to construct SRD runtime: {error}")))?;
    build_evidence_complete_reference_fennel_draws_from_runtime(
        project,
        scene_index,
        host,
        font_registry,
        runtime_fonts,
        force_color_update,
        runtime_text_inputs,
        &runtime,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_evidence_complete_animation_set_reference_fennel_draws(
    project: &Project,
    textures: &TextureList,
    scene_index: usize,
    animation_set_index: usize,
    frame: f32,
    host: SrdHostDrawContext,
    font_registry: &FennelFontSlotRegistry<Vec<u8>>,
    runtime_fonts: &BTreeMap<Vec<u8>, RuhunaRuntimeFont>,
    force_color_update: bool,
    runtime_text_inputs: &BTreeMap<FennelRuntimeTextCastKey, FennelSrdRuntimeTextInput>,
) -> Result<Vec<EvidenceCompleteFennelDraw>, SrdDrawError> {
    validate_host_draw_context(host)?;
    let mut runtime = ProjectRuntime::new(project)
        .map_err(|error| SrdDrawError(format!("failed to construct SRD runtime: {error}")))?;
    runtime
        .apply_animation_set(project, textures, scene_index, animation_set_index, frame)
        .map_err(|error| SrdDrawError(format!("failed to apply animation set: {error}")))?;
    build_evidence_complete_reference_fennel_draws_from_runtime(
        project,
        scene_index,
        host,
        font_registry,
        runtime_fonts,
        force_color_update,
        runtime_text_inputs,
        &runtime,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_evidence_complete_reference_fennel_draws_from_runtime(
    project: &Project,
    scene_index: usize,
    host: SrdHostDrawContext,
    font_registry: &FennelFontSlotRegistry<Vec<u8>>,
    runtime_fonts: &BTreeMap<Vec<u8>, RuhunaRuntimeFont>,
    force_color_update: bool,
    runtime_text_inputs: &BTreeMap<FennelRuntimeTextCastKey, FennelSrdRuntimeTextInput>,
    runtime: &ProjectRuntime,
) -> Result<Vec<EvidenceCompleteFennelDraw>, SrdDrawError> {
    let renderer_inverse_camera_view = renderer_inverse_camera_view(project, host);
    let worlds = runtime
        .compose_world_states(
            project,
            host.first_calc_matrix,
            renderer_inverse_camera_view,
        )
        .map_err(|error| SrdDrawError(error.to_string()))?;
    if project.scenes.get(scene_index).is_none() {
        return Err(SrdDrawError(format!(
            "scene index {scene_index} is outside the project"
        )));
    }
    let camera_bridge = renderer_project_camera_bridge(project, host);
    let project_screen = renderer_project_screen_matrix(host, camera_bridge)?;
    let mut draws = Vec::new();
    for entry in runtime
        .references
        .plan
        .structural_cast_draw_order(project, scene_index, host.renderer_layer_key)
        .map_err(|error| SrdDrawError(error.to_string()))?
    {
        let layer = &project.scenes[entry.source.scene_index].layers[entry.source.layer_index];
        let layer_worlds = worlds.layer(entry.owner).ok_or_else(|| {
            SrdDrawError(format!(
                "runtime draw owner {:?} has no composed world state",
                entry.owner
            ))
        })?;
        let cast_world = *layer_worlds.casts.get(entry.node_index).ok_or_else(|| {
            SrdDrawError(format!(
                "SCN[{}]/LAYR[{}]/NODE[{}] has no composed CAST world state",
                entry.source.scene_index, entry.source.layer_index, entry.node_index
            ))
        })?;
        let image_state = runtime_image_state_for_owner(runtime, entry.owner, entry.node_index)?;
        let runtime_text_input = runtime_text_inputs.get(&FennelRuntimeTextCastKey {
            owner: entry.owner,
            node_index: entry.node_index,
        });
        if let Some(draw) = build_evidence_complete_fennel_draw_for_runtime_cast(
            project,
            entry.owner,
            entry.source,
            layer,
            entry.node_index,
            entry.renderer_layer_key,
            layer_worlds.is_2d,
            cast_world,
            image_state,
            host,
            camera_bridge,
            project_screen.as_ref(),
            font_registry,
            runtime_fonts,
            force_color_update,
            runtime_text_input,
        )? {
            draws.push(draw);
        }
    }
    Ok(draws)
}

#[allow(clippy::too_many_arguments)]
pub fn build_evidence_complete_initial_runtime_cast_draws(
    project: &Project,
    textures: &TextureList,
    scene_index: usize,
    host: SrdHostDrawContext,
    font_registry: &FennelFontSlotRegistry<Vec<u8>>,
    runtime_fonts: &BTreeMap<Vec<u8>, RuhunaRuntimeFont>,
    force_color_update: bool,
    runtime_text_inputs: &BTreeMap<FennelRuntimeTextCastKey, FennelSrdRuntimeTextInput>,
) -> Result<Vec<EvidenceCompleteRuntimeCastDraw>, SrdDrawError> {
    validate_host_draw_context(host)?;
    let runtime = ProjectRuntime::new(project)
        .map_err(|error| SrdDrawError(format!("failed to construct SRD runtime: {error}")))?;
    build_evidence_complete_runtime_cast_draws_from_runtime(
        project,
        textures,
        scene_index,
        host,
        font_registry,
        runtime_fonts,
        force_color_update,
        runtime_text_inputs,
        &runtime,
        true,
    )
}

/// Builds a complete runtime CAST stream from an arbitrary dense ANMS
/// assignment. The assignment need not be stored in the scene and can be an
/// editor-owned scratch clone.
#[allow(clippy::too_many_arguments)]
pub fn build_evidence_complete_animation_assignment_runtime_cast_draws(
    project: &Project,
    textures: &TextureList,
    scene_index: usize,
    animation_assignment: &AnimationSetDefinition,
    frame: f32,
    host: SrdHostDrawContext,
    font_registry: &FennelFontSlotRegistry<Vec<u8>>,
    runtime_fonts: &BTreeMap<Vec<u8>, RuhunaRuntimeFont>,
    force_color_update: bool,
    runtime_text_inputs: &BTreeMap<FennelRuntimeTextCastKey, FennelSrdRuntimeTextInput>,
) -> Result<Vec<EvidenceCompleteRuntimeCastDraw>, SrdDrawError> {
    validate_host_draw_context(host)?;
    let mut runtime = ProjectRuntime::new(project)
        .map_err(|error| SrdDrawError(format!("failed to construct SRD runtime: {error}")))?;
    runtime
        .apply_animation_set_definition(project, textures, scene_index, animation_assignment, frame)
        .map_err(|error| SrdDrawError(format!("failed to apply animation assignment: {error}")))?;
    build_evidence_complete_runtime_cast_draws_from_runtime(
        project,
        textures,
        scene_index,
        host,
        font_registry,
        runtime_fonts,
        force_color_update,
        runtime_text_inputs,
        &runtime,
        true,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn build_evidence_complete_animation_set_runtime_cast_draws(
    project: &Project,
    textures: &TextureList,
    scene_index: usize,
    animation_set_index: usize,
    frame: f32,
    host: SrdHostDrawContext,
    font_registry: &FennelFontSlotRegistry<Vec<u8>>,
    runtime_fonts: &BTreeMap<Vec<u8>, RuhunaRuntimeFont>,
    force_color_update: bool,
    runtime_text_inputs: &BTreeMap<FennelRuntimeTextCastKey, FennelSrdRuntimeTextInput>,
) -> Result<Vec<EvidenceCompleteRuntimeCastDraw>, SrdDrawError> {
    validate_host_draw_context(host)?;
    let mut runtime = ProjectRuntime::new(project)
        .map_err(|error| SrdDrawError(format!("failed to construct SRD runtime: {error}")))?;
    runtime
        .apply_animation_set(project, textures, scene_index, animation_set_index, frame)
        .map_err(|error| SrdDrawError(format!("failed to apply animation set: {error}")))?;
    build_evidence_complete_runtime_cast_draws_from_runtime(
        project,
        textures,
        scene_index,
        host,
        font_registry,
        runtime_fonts,
        force_color_update,
        runtime_text_inputs,
        &runtime,
        true,
    )
}

/// Builds the non-text SRD runtime stream without requiring RFZ/Fennel host
/// resources. This is useful for corpus-wide render-state audits: ImageCast,
/// SliceCast and NumberCast still follow the same recursive CAST order and
/// runtime animation state as the complete stream, while SrTextCast is omitted
/// explicitly instead of being confused with an unsupported image draw.
pub fn build_evidence_complete_animation_set_runtime_srd_draws(
    project: &Project,
    textures: &TextureList,
    scene_index: usize,
    animation_set_index: usize,
    frame: f32,
    host: SrdHostDrawContext,
) -> Result<Vec<EvidenceCompleteRuntimeCastDraw>, SrdDrawError> {
    validate_host_draw_context(host)?;
    let mut runtime = ProjectRuntime::new(project)
        .map_err(|error| SrdDrawError(format!("failed to construct SRD runtime: {error}")))?;
    runtime
        .apply_animation_set(project, textures, scene_index, animation_set_index, frame)
        .map_err(|error| SrdDrawError(format!("failed to apply animation set: {error}")))?;
    build_evidence_complete_runtime_srd_draws_from_runtime(
        project,
        textures,
        scene_index,
        host,
        &runtime,
    )
}

/// State-reuse variant of [`build_evidence_complete_animation_set_runtime_srd_draws`].
/// The supplied runtime must already contain the desired animation-set frame.
/// Keeping construction separate lets exhaustive corpus tools clone one proven
/// reference plan per file instead of reparsing it for every integer frame.
pub fn build_evidence_complete_runtime_srd_draws_from_runtime(
    project: &Project,
    textures: &TextureList,
    scene_index: usize,
    host: SrdHostDrawContext,
    runtime: &ProjectRuntime,
) -> Result<Vec<EvidenceCompleteRuntimeCastDraw>, SrdDrawError> {
    validate_host_draw_context(host)?;
    build_evidence_complete_runtime_cast_draws_from_runtime(
        project,
        textures,
        scene_index,
        host,
        &FennelFontSlotRegistry::default(),
        &BTreeMap::new(),
        false,
        &BTreeMap::new(),
        runtime,
        false,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_evidence_complete_runtime_cast_draws_from_runtime(
    project: &Project,
    textures: &TextureList,
    scene_index: usize,
    host: SrdHostDrawContext,
    font_registry: &FennelFontSlotRegistry<Vec<u8>>,
    runtime_fonts: &BTreeMap<Vec<u8>, RuhunaRuntimeFont>,
    force_color_update: bool,
    runtime_text_inputs: &BTreeMap<FennelRuntimeTextCastKey, FennelSrdRuntimeTextInput>,
    runtime: &ProjectRuntime,
    include_fennel: bool,
) -> Result<Vec<EvidenceCompleteRuntimeCastDraw>, SrdDrawError> {
    let renderer_inverse_camera_view = renderer_inverse_camera_view(project, host);
    let worlds = runtime
        .compose_world_states(
            project,
            host.first_calc_matrix,
            renderer_inverse_camera_view,
        )
        .map_err(|error| SrdDrawError(error.to_string()))?;
    if project.scenes.get(scene_index).is_none() {
        return Err(SrdDrawError(format!(
            "scene index {scene_index} is outside the project"
        )));
    }
    let camera_bridge = renderer_project_camera_bridge(project, host);
    let project_screen = renderer_project_screen_matrix(host, camera_bridge)?;
    let identity = identity_matrix4x4_game();
    let mut packet_current_matrix = identity;
    let mut draws = Vec::new();

    for entry in runtime
        .references
        .plan
        .structural_cast_draw_order(project, scene_index, host.renderer_layer_key)
        .map_err(|error| SrdDrawError(error.to_string()))?
    {
        let layer = &project.scenes[entry.source.scene_index].layers[entry.source.layer_index];
        let layer_worlds = worlds.layer(entry.owner).ok_or_else(|| {
            SrdDrawError(format!(
                "runtime draw owner {:?} has no composed world state",
                entry.owner
            ))
        })?;
        let cast_world = *layer_worlds.casts.get(entry.node_index).ok_or_else(|| {
            SrdDrawError(format!(
                "SCN[{}]/LAYR[{}]/NODE[{}] has no composed CAST world state",
                entry.source.scene_index, entry.source.layer_index, entry.node_index
            ))
        })?;
        let image_state = runtime_image_state_for_owner(runtime, entry.owner, entry.node_index)?;
        if let Some(draw) = build_evidence_complete_image_draw_for_runtime_cast(
            textures,
            entry.owner,
            entry.source,
            layer,
            entry.node_index,
            entry.renderer_layer_key,
            layer_worlds.is_2d,
            cast_world,
            image_state,
            host,
            camera_bridge,
            project_screen.as_ref(),
            identity,
            &mut packet_current_matrix,
        )? {
            draws.push(EvidenceCompleteRuntimeCastDraw::Image(draw));
            continue;
        }
        if layer.csli_by_node[entry.node_index].is_some() {
            let cells = build_evidence_complete_slice_cell_draws_for_runtime_cast(
                textures,
                entry.owner,
                entry.source,
                layer,
                entry.node_index,
                entry.renderer_layer_key,
                layer_worlds.is_2d,
                cast_world,
                image_state,
                host,
                camera_bridge,
                project_screen.as_ref(),
                identity,
                &mut packet_current_matrix,
            )?;
            draws.extend(
                cells
                    .into_iter()
                    .map(EvidenceCompleteRuntimeCastDraw::SliceCell),
            );
            continue;
        }
        if layer.number_by_node[entry.node_index].is_some() {
            let glyphs = build_evidence_complete_number_glyph_draws_for_runtime_cast(
                textures,
                entry.owner,
                entry.source,
                layer,
                entry.node_index,
                entry.renderer_layer_key,
                layer_worlds.is_2d,
                cast_world,
                image_state,
                host,
                camera_bridge,
                project_screen.as_ref(),
                identity,
                &mut packet_current_matrix,
            )?;
            draws.extend(
                glyphs
                    .into_iter()
                    .map(EvidenceCompleteRuntimeCastDraw::NumberGlyph),
            );
            continue;
        }
        if !include_fennel {
            continue;
        }
        let runtime_text_input = runtime_text_inputs.get(&FennelRuntimeTextCastKey {
            owner: entry.owner,
            node_index: entry.node_index,
        });
        if let Some(draw) = build_evidence_complete_fennel_draw_for_runtime_cast(
            project,
            entry.owner,
            entry.source,
            layer,
            entry.node_index,
            entry.renderer_layer_key,
            layer_worlds.is_2d,
            cast_world,
            image_state,
            host,
            camera_bridge,
            project_screen.as_ref(),
            font_registry,
            runtime_fonts,
            force_color_update,
            runtime_text_input,
        )? {
            draws.push(EvidenceCompleteRuntimeCastDraw::Fennel(draw));
        }
    }
    Ok(draws)
}

fn build_evidence_complete_fennel_draws_impl(
    project: &Project,
    scene_index: usize,
    host: SrdHostDrawContext,
    font_registry: &FennelFontSlotRegistry<Vec<u8>>,
    runtime_fonts: &BTreeMap<Vec<u8>, RuhunaRuntimeFont>,
    force_color_update: bool,
    runtime_layers: Option<&[ProjectLayerRuntimeState]>,
    runtime_text_inputs: Option<&BTreeMap<(usize, usize), FennelSrdRuntimeTextInput>>,
) -> Result<Vec<EvidenceCompleteFennelDraw>, SrdDrawError> {
    validate_host_draw_context(host)?;
    let mut composition_runtime = ProjectRuntime::new(project)
        .map_err(|error| SrdDrawError(format!("failed to construct SRD runtime: {error}")))?;
    if let Some(runtime_layers) = runtime_layers {
        let destination = composition_runtime
            .project_layers
            .get_mut(scene_index)
            .ok_or_else(|| {
                SrdDrawError(format!("scene index {scene_index} is outside the runtime"))
            })?;
        if destination.len() != runtime_layers.len() {
            return Err(SrdDrawError(format!(
                "SCN[{scene_index}] has {} runtime layers, expected {}",
                runtime_layers.len(),
                destination.len()
            )));
        }
        destination.clone_from_slice(runtime_layers);
    }
    let renderer_inverse_camera_view = renderer_inverse_camera_view(project, host);
    let composed_worlds = composition_runtime
        .compose_world_states(
            project,
            host.first_calc_matrix,
            renderer_inverse_camera_view,
        )
        .map_err(|error| SrdDrawError(error.to_string()))?;
    let scene = project
        .scenes
        .get(scene_index)
        .ok_or_else(|| SrdDrawError(format!("scene index {scene_index} is outside the project")))?;
    let identity = identity_matrix4x4_game();
    let camera_bridge = renderer_project_camera_bridge(project, host);
    let project_screen = renderer_project_screen_matrix(host, camera_bridge)?;
    let mut draws = Vec::new();

    for (layer_index, layer) in scene.layers.iter().enumerate() {
        let runtime_layer = runtime_layers
            .map(|layers| {
                layers.get(layer_index).ok_or_else(|| {
                    SrdDrawError(format!(
                        "SCN[{scene_index}]/LAYR[{layer_index}] is missing from the runtime"
                    ))
                })
            })
            .transpose()?;
        let layer_enabled = runtime_layer
            .map(|runtime_layer| runtime_layer.enabled)
            .unwrap_or(layer.flags & 0x100 != 0);
        if !layer_enabled {
            continue;
        }
        let is_2d = layer.is_2d();
        let layer_worlds = composed_worlds
            .project_layers
            .get(scene_index)
            .and_then(|layers| layers.get(layer_index))
            .ok_or_else(|| {
                SrdDrawError(format!(
                    "SCN[{scene_index}]/LAYR[{layer_index}] has no composed runtime world state"
                ))
            })?;
        let cast_layer_keys = layer
            .compose_runtime_cast_layer_keys(host.renderer_layer_key)
            .map_err(|error| SrdDrawError(error.to_string()))?;

        for node_index in 0..layer.nodes.len() {
            let Some(image) = layer.image_by_node[node_index]
                .as_ref()
                .filter(|image| image.creates_text_cast())
            else {
                continue;
            };
            let Some(text) = image.text.as_ref() else {
                continue;
            };
            let cast_world = layer_worlds.casts[node_index];
            if !cast_world.render_gate {
                continue;
            }
            let font_index = usize::try_from(text.font_index.unwrap_or(-1)).map_err(|_| {
                SrdDrawError(format!(
                    "SCN[{scene_index}]/LAYR[{layer_index}]/NODE[{node_index}] has an invalid RFZ font index"
                ))
            })?;
            let font = project.fonts.get(font_index).ok_or_else(|| {
                SrdDrawError(format!(
                    "SCN[{scene_index}]/LAYR[{layer_index}]/NODE[{node_index}] RFZ font index {font_index} is outside PROJ"
                ))
            })?;
            if !font
                .name
                .iter()
                .map(u8::to_ascii_lowercase)
                .collect::<Vec<_>>()
                .ends_with(b".rfz")
            {
                continue;
            }
            if !runtime_fonts.contains_key(font.name.as_slice()) {
                return Err(SrdDrawError(format!(
                    "SCN[{scene_index}]/LAYR[{layer_index}]/NODE[{node_index}] runtime RFZ {:?} is not loaded",
                    String::from_utf8_lossy(&font.name)
                )));
            }
            let primary_slot = font_registry
                .request_for_key(&font.name)
                .filter(|request| request.registered)
                .ok_or_else(|| {
                    SrdDrawError(format!(
                        "SCN[{scene_index}]/LAYR[{layer_index}]/NODE[{node_index}] RFZ {:?} has no registered global Fennel slot",
                        String::from_utf8_lossy(&font.name)
                    ))
                })?
                .font_slot_id;
            let image_state = runtime_layer.map_or_else(
                || image.initial_runtime_state(),
                |runtime_layer| runtime_layer.image_states[node_index],
            );
            let positions = image
                .build_quad_with_geometry(image_state.geometry, is_2d)
                .positions
                .map(|point| cast_world.matrix.transform_point_game(point));
            if !runtime_cast_passes_renderer_visibility(positions, is_2d, project_screen.as_ref())?
            {
                continue;
            }
            let font_param = layer.font_param_for_node(node_index).unwrap_or_default();
            if font_param.vertical {
                return Err(SrdDrawError(format!(
                    "SCN[{scene_index}]/LAYR[{layer_index}]/NODE[{node_index}] uses the unported FontParamData vertical correction"
                )));
            }
            let properties = FennelStaticTextProperties::from_text_definition(
                text,
                image_state.geometry.size[0],
                image_state.geometry.size[1],
            )
            .map_err(|error| SrdDrawError(error.to_string()))?;
            let source_colors = image_state
                .coordinate_state(ImageReferenceChannel::Cref)
                .vertex_colors;
            let primary_rgba = [0usize, 2, 1, 3].map(|source_index| {
                multiply_color_game(source_colors[source_index], cast_world.multiply_color)
            });
            let secondary_rgba = premultiply_additive_color_game(cast_world.additive_color);
            if !force_color_update
                && !secondary_rgba[..3].iter().any(|component| *component != 0)
                && !primary_rgba.iter().any(|color| color[3] != 0)
            {
                // `sub_AC5740` skips the entire TextBox update/draw block when
                // its external +0x11C bit 0x100 is clear and all effective
                // primary alpha / secondary RGB channels are zero.
                continue;
            }
            let primary_colors = primary_rgba.map(pack_fennel_record_color);
            let secondary_color = pack_fennel_record_color(secondary_rgba);
            let font_style = fennel_srd_font_style(font_param);
            let effect_color = fennel_font_param_effect_color(font_param.shadow_color);
            let effect_offset = [font_param.shadow_x as f32, font_param.shadow_y as f32];
            let runtime_text_input =
                runtime_text_inputs.and_then(|inputs| inputs.get(&(layer_index, node_index)));
            if runtime_text_input.is_none() {
                for substitution_index in 0..8u8 {
                    let token = [b'$', b'[', b'0' + substitution_index, b']'];
                    if text
                        .text
                        .windows(token.len())
                        .any(|candidate| candidate == token)
                    {
                        return Err(SrdDrawError(format!(
                            "SCN[{scene_index}]/LAYR[{layer_index}]/NODE[{node_index}] uses runtime substitution {:?}, but this draw has no SrTextCast substitution host input",
                            String::from_utf8_lossy(&token)
                        )));
                    }
                }
            }
            let substitutions: [&[u8]; 8] = runtime_text_input
                .map_or([b"".as_slice(); 8], |input| {
                    std::array::from_fn(|index| input.substitutions[index].as_slice())
                });
            let default_d =
                runtime_text_input.map_or(FENNEL_DEFAULT_D_VALUE, |input| input.default_d);
            let repeat_space_count = runtime_text_input
                .map_or(FENNEL_DEFAULT_REPEAT_SPACE_COUNT, |input| {
                    input.repeat_space_count
                });
            let mut prepared_text =
                prepare_fennel_srd_runtime_text(&text.text, substitutions, default_d, font_param)
                    .map_err(|error| SrdDrawError(error.to_string()))?;
            if let Some(input) = runtime_text_input {
                prepared_text.scroll_state.field_f4 = input.field_f4;
            }
            let scroll = prepared_text.scroll_state.outputs();
            let build_layout = |source: &[u8], explicit_flags: Option<u32>| {
                let mut stream = build_fennel_plain_record_stream_with_font_slots(
                    source,
                    primary_slot,
                    properties.glyph_placement(0, font_style.record_flags, primary_colors),
                    2048,
                    |font_slot_id, code| {
                        let resource_name = font_registry.resource_for_slot(font_slot_id)?;
                        let runtime_font = runtime_fonts.get(resource_name.as_slice())?;
                        Some(FennelResolvedGlyph {
                            glyph_token: fennel_slot_code_token(font_slot_id, code),
                            glyph: *runtime_font.glyph(code)?,
                        })
                    },
                )
                .map_err(|error| SrdDrawError(error.to_string()))?;
                let layout = if let Some(textbox_flags) = explicit_flags {
                    layout_fennel_static_srd_explicit_flags(
                        &mut stream,
                        properties.layout,
                        textbox_flags,
                        |token| {
                            let (font_slot_id, code) = fennel_slot_code_from_token(token);
                            let resource_name = font_registry.resource_for_slot(font_slot_id)?;
                            let runtime_font = runtime_fonts.get(resource_name.as_slice())?;
                            runtime_font.glyph(code).map(Into::into)
                        },
                    )
                } else {
                    layout_fennel_static_srd_font_param(
                        &mut stream,
                        properties.layout,
                        font_param,
                        |token| {
                            let (font_slot_id, code) = fennel_slot_code_from_token(token);
                            let resource_name = font_registry.resource_for_slot(font_slot_id)?;
                            let runtime_font = runtime_fonts.get(resource_name.as_slice())?;
                            runtime_font.glyph(code).map(Into::into)
                        },
                    )
                }
                .map_err(|error| SrdDrawError(error.to_string()))?;
                Ok::<_, SrdDrawError>((stream, layout))
            };
            let measure = |stream: &_| {
                measure_fennel_srd_text_size_mode0(
                    stream,
                    scroll.maximum_glyphs,
                    [properties.layout.scale_x, properties.layout.scale_y],
                    effect_offset,
                    |token| {
                        let (font_slot_id, code) = fennel_slot_code_from_token(token);
                        let resource_name = font_registry.resource_for_slot(font_slot_id)?;
                        let runtime_font = runtime_fonts.get(resource_name.as_slice())?;
                        runtime_font.glyph(code).copied()
                    },
                )
                .map_err(|error| SrdDrawError(error.to_string()))
            };

            let (initial_stream, initial_layout) = build_layout(&prepared_text.text, None)?;
            let initial_text_size = measure(&initial_stream)?;
            let repeated_text = build_fennel_srd_repeated_text(
                &prepared_text.text,
                initial_layout.textbox_flags,
                repeat_space_count,
            )
            .map_err(|error| SrdDrawError(error.to_string()))?;
            let repeated = if let Some(repeated_text) = repeated_text {
                let (stream, layout) =
                    build_layout(&repeated_text, Some(initial_layout.textbox_flags))?;
                let size = measure(&stream)?;
                Some((stream, layout, size))
            } else {
                None
            };
            let preparation = prepare_fennel_srd_draw(FennelSrdDrawPreparationInput {
                textbox_flags: initial_layout.textbox_flags,
                text_size: initial_text_size,
                clip_size: [properties.layout.box_width, properties.layout.box_height],
                font_point_y: u16::from(font_style.point_y),
                scroll,
                repeated_text_size: repeated.as_ref().map(|(_, _, size)| *size),
            })
            .map_err(|error| SrdDrawError(error.to_string()))?;
            let draw_textbox_flags = initial_layout.textbox_flags;
            let (stream, layout, draw_offset) = match preparation {
                FennelSrdDrawPreparation::DrawOffset(draw_offset) => {
                    if let Some((stream, layout, _)) = repeated {
                        (stream, layout, draw_offset)
                    } else {
                        (initial_stream, initial_layout, draw_offset)
                    }
                }
                FennelSrdDrawPreparation::RelayoutWithoutScrollModes { layout_flags } => {
                    let (stream, layout) = build_layout(&prepared_text.text, Some(layout_flags))?;
                    (stream, layout, [0.0; 2])
                }
            };
            let vertex_build = build_fennel_normal_vertex_batches(
                &stream,
                scroll.maximum_glyphs,
                FennelNormalDrawInput {
                    is_2d,
                    textbox_position: [
                        -image_state.geometry.origin[0],
                        -image_state.geometry.origin[1],
                        0.0,
                    ],
                    textbox_scale: [properties.layout.scale_x, properties.layout.scale_y],
                    textbox_vertical_offset: layout.layout.textbox_vertical_offset,
                    textbox_transform: fennel_textbox_transform(
                        cast_world.matrix,
                        is_2d,
                        camera_bridge,
                    ),
                    secondary_color,
                    // `sub_7C04F0` restores the original word after a fit
                    // guard relayout and draws with that original clip state.
                    textbox_flags: draw_textbox_flags,
                    clip_size: [properties.layout.box_width, properties.layout.box_height],
                    draw_offset,
                    effect_colors: [effect_color; 4],
                    effect_offset,
                },
                |token| {
                    let (font_slot_id, code) = fennel_slot_code_from_token(token);
                    let resource_name = font_registry.resource_for_slot(font_slot_id)?;
                    let runtime_font = runtime_fonts.get(resource_name.as_slice())?;
                    runtime_font.glyph(code).copied()
                },
            )
            .map_err(|error| SrdDrawError(error.to_string()))?;
            if vertex_build.processed_glyph_count == 0 {
                continue;
            }
            let mut fixed_constants = CeylonSrdFixedShaderConstants::initial_for_target(
                host.target_projection_view,
                host.target_screen_size,
            );
            fixed_constants.vertex_c0_c3_world = if is_2d {
                identity
            } else {
                fennel_textbox_transform(cast_world.matrix, false, camera_bridge)
            };
            // A freshly constructed TextBox builder has identity as its
            // previous packet matrix. The exact shipped 3D Fennel VS reads
            // c0..c3 but has no c4..c7 source operands.
            fixed_constants.vertex_c4_c7 = identity;
            draws.push(EvidenceCompleteFennelDraw {
                owner: ReferenceLayerParent::ProjectLayer(crate::scene::ReferenceTarget {
                    scene_index,
                    layer_index,
                }),
                scene_index,
                layer_index,
                node_index,
                font_name: font.name.clone(),
                is_2d,
                renderer_layer_key: cast_layer_keys[node_index]
                    .wrapping_add(u32::from(layer.nodes[node_index].render_layer_offset())),
                packet: fennel_default_draw_packet(is_2d),
                fixed_constants,
                batches: vertex_build.batches,
            });
        }
    }
    Ok(draws)
}

#[allow(clippy::too_many_arguments)]
fn build_evidence_complete_fennel_draw_for_runtime_cast(
    project: &Project,
    owner: ReferenceLayerParent,
    source: ReferenceTarget,
    layer: &Layer,
    node_index: usize,
    renderer_layer_key: u32,
    is_2d: bool,
    world: RuntimeWorldState,
    image_state: crate::image::RuntimeImageState,
    host: SrdHostDrawContext,
    camera_bridge: Matrix4x4,
    project_screen: Option<&(Matrix4x4, [i32; 2])>,
    font_registry: &FennelFontSlotRegistry<Vec<u8>>,
    runtime_fonts: &BTreeMap<Vec<u8>, RuhunaRuntimeFont>,
    force_color_update: bool,
    runtime_text_input: Option<&FennelSrdRuntimeTextInput>,
) -> Result<Option<EvidenceCompleteFennelDraw>, SrdDrawError> {
    if !world.visible || !world.render_gate {
        return Ok(None);
    }
    let Some(image) = layer.image_by_node[node_index]
        .as_ref()
        .filter(|image| image.creates_text_cast())
    else {
        return Ok(None);
    };
    let Some(text) = image.text.as_ref() else {
        return Ok(None);
    };
    let font_index = usize::try_from(text.font_index.unwrap_or(-1)).map_err(|_| {
        SrdDrawError(format!(
            "SCN[{}]/LAYR[{}]/NODE[{node_index}] has an invalid RFZ font index",
            source.scene_index, source.layer_index
        ))
    })?;
    let font = project.fonts.get(font_index).ok_or_else(|| {
        SrdDrawError(format!(
            "SCN[{}]/LAYR[{}]/NODE[{node_index}] RFZ font index {font_index} is outside PROJ",
            source.scene_index, source.layer_index
        ))
    })?;
    if !font
        .name
        .iter()
        .map(u8::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .ends_with(b".rfz")
    {
        return Ok(None);
    }
    if !runtime_fonts.contains_key(font.name.as_slice()) {
        return Err(SrdDrawError(format!(
            "SCN[{}]/LAYR[{}]/NODE[{node_index}] runtime RFZ {:?} is not loaded",
            source.scene_index,
            source.layer_index,
            String::from_utf8_lossy(&font.name)
        )));
    }
    let primary_slot = font_registry
        .request_for_key(&font.name)
        .filter(|request| request.registered)
        .ok_or_else(|| {
            SrdDrawError(format!(
                "SCN[{}]/LAYR[{}]/NODE[{node_index}] RFZ {:?} has no registered global Fennel slot",
                source.scene_index,
                source.layer_index,
                String::from_utf8_lossy(&font.name)
            ))
        })?
        .font_slot_id;
    let positions = image
        .build_quad_with_geometry(image_state.geometry, is_2d)
        .positions
        .map(|point| world.matrix.transform_point_game(point));
    if !runtime_cast_passes_renderer_visibility(positions, is_2d, project_screen)? {
        return Ok(None);
    }
    let font_param = layer.font_param_for_node(node_index).unwrap_or_default();
    if font_param.vertical {
        return Err(SrdDrawError(format!(
            "SCN[{}]/LAYR[{}]/NODE[{node_index}] uses the unported FontParamData vertical correction",
            source.scene_index, source.layer_index
        )));
    }
    let properties = FennelStaticTextProperties::from_text_definition(
        text,
        image_state.geometry.size[0],
        image_state.geometry.size[1],
    )
    .map_err(|error| SrdDrawError(error.to_string()))?;
    let source_colors = image_state
        .coordinate_state(ImageReferenceChannel::Cref)
        .vertex_colors;
    let primary_rgba = [0usize, 2, 1, 3]
        .map(|source_index| multiply_color_game(source_colors[source_index], world.multiply_color));
    let secondary_rgba = premultiply_additive_color_game(world.additive_color);
    if !force_color_update
        && !secondary_rgba[..3].iter().any(|component| *component != 0)
        && !primary_rgba.iter().any(|color| color[3] != 0)
    {
        return Ok(None);
    }
    let primary_colors = primary_rgba.map(pack_fennel_record_color);
    let secondary_color = pack_fennel_record_color(secondary_rgba);
    let font_style = fennel_srd_font_style(font_param);
    let effect_color = fennel_font_param_effect_color(font_param.shadow_color);
    let effect_offset = [font_param.shadow_x as f32, font_param.shadow_y as f32];
    if runtime_text_input.is_none() {
        for substitution_index in 0..8u8 {
            let token = [b'$', b'[', b'0' + substitution_index, b']'];
            if text
                .text
                .windows(token.len())
                .any(|candidate| candidate == token)
            {
                return Err(SrdDrawError(format!(
                    "SCN[{}]/LAYR[{}]/NODE[{node_index}] uses runtime substitution {:?}, but this draw has no SrTextCast substitution host input",
                    source.scene_index,
                    source.layer_index,
                    String::from_utf8_lossy(&token)
                )));
            }
        }
    }
    let substitutions: [&[u8]; 8] = runtime_text_input.map_or([b"".as_slice(); 8], |input| {
        std::array::from_fn(|index| input.substitutions[index].as_slice())
    });
    let default_d = runtime_text_input.map_or(FENNEL_DEFAULT_D_VALUE, |input| input.default_d);
    let repeat_space_count = runtime_text_input
        .map_or(FENNEL_DEFAULT_REPEAT_SPACE_COUNT, |input| {
            input.repeat_space_count
        });
    let mut prepared_text =
        prepare_fennel_srd_runtime_text(&text.text, substitutions, default_d, font_param)
            .map_err(|error| SrdDrawError(error.to_string()))?;
    if let Some(input) = runtime_text_input {
        prepared_text.scroll_state.field_f4 = input.field_f4;
    }
    let scroll = prepared_text.scroll_state.outputs();
    let build_layout = |text_source: &[u8], explicit_flags: Option<u32>| {
        let mut stream = build_fennel_plain_record_stream_with_font_slots(
            text_source,
            primary_slot,
            properties.glyph_placement(0, font_style.record_flags, primary_colors),
            2048,
            |font_slot_id, code| {
                let resource_name = font_registry.resource_for_slot(font_slot_id)?;
                let runtime_font = runtime_fonts.get(resource_name.as_slice())?;
                Some(FennelResolvedGlyph {
                    glyph_token: fennel_slot_code_token(font_slot_id, code),
                    glyph: *runtime_font.glyph(code)?,
                })
            },
        )
        .map_err(|error| SrdDrawError(error.to_string()))?;
        let layout = if let Some(textbox_flags) = explicit_flags {
            layout_fennel_static_srd_explicit_flags(
                &mut stream,
                properties.layout,
                textbox_flags,
                |token| {
                    let (font_slot_id, code) = fennel_slot_code_from_token(token);
                    let resource_name = font_registry.resource_for_slot(font_slot_id)?;
                    let runtime_font = runtime_fonts.get(resource_name.as_slice())?;
                    runtime_font.glyph(code).map(Into::into)
                },
            )
        } else {
            layout_fennel_static_srd_font_param(
                &mut stream,
                properties.layout,
                font_param,
                |token| {
                    let (font_slot_id, code) = fennel_slot_code_from_token(token);
                    let resource_name = font_registry.resource_for_slot(font_slot_id)?;
                    let runtime_font = runtime_fonts.get(resource_name.as_slice())?;
                    runtime_font.glyph(code).map(Into::into)
                },
            )
        }
        .map_err(|error| SrdDrawError(error.to_string()))?;
        Ok::<_, SrdDrawError>((stream, layout))
    };
    let measure = |stream: &_| {
        measure_fennel_srd_text_size_mode0(
            stream,
            scroll.maximum_glyphs,
            [properties.layout.scale_x, properties.layout.scale_y],
            effect_offset,
            |token| {
                let (font_slot_id, code) = fennel_slot_code_from_token(token);
                let resource_name = font_registry.resource_for_slot(font_slot_id)?;
                let runtime_font = runtime_fonts.get(resource_name.as_slice())?;
                runtime_font.glyph(code).copied()
            },
        )
        .map_err(|error| SrdDrawError(error.to_string()))
    };

    let (initial_stream, initial_layout) = build_layout(&prepared_text.text, None)?;
    let initial_text_size = measure(&initial_stream)?;
    let repeated_text = build_fennel_srd_repeated_text(
        &prepared_text.text,
        initial_layout.textbox_flags,
        repeat_space_count,
    )
    .map_err(|error| SrdDrawError(error.to_string()))?;
    let repeated = if let Some(repeated_text) = repeated_text {
        let (stream, layout) = build_layout(&repeated_text, Some(initial_layout.textbox_flags))?;
        let size = measure(&stream)?;
        Some((stream, layout, size))
    } else {
        None
    };
    let preparation = prepare_fennel_srd_draw(FennelSrdDrawPreparationInput {
        textbox_flags: initial_layout.textbox_flags,
        text_size: initial_text_size,
        clip_size: [properties.layout.box_width, properties.layout.box_height],
        font_point_y: u16::from(font_style.point_y),
        scroll,
        repeated_text_size: repeated.as_ref().map(|(_, _, size)| *size),
    })
    .map_err(|error| SrdDrawError(error.to_string()))?;
    let draw_textbox_flags = initial_layout.textbox_flags;
    let (stream, layout, draw_offset) = match preparation {
        FennelSrdDrawPreparation::DrawOffset(draw_offset) => {
            if let Some((stream, layout, _)) = repeated {
                (stream, layout, draw_offset)
            } else {
                (initial_stream, initial_layout, draw_offset)
            }
        }
        FennelSrdDrawPreparation::RelayoutWithoutScrollModes { layout_flags } => {
            let (stream, layout) = build_layout(&prepared_text.text, Some(layout_flags))?;
            (stream, layout, [0.0; 2])
        }
    };
    let vertex_build = build_fennel_normal_vertex_batches(
        &stream,
        scroll.maximum_glyphs,
        FennelNormalDrawInput {
            is_2d,
            textbox_position: [
                -image_state.geometry.origin[0],
                -image_state.geometry.origin[1],
                0.0,
            ],
            textbox_scale: [properties.layout.scale_x, properties.layout.scale_y],
            textbox_vertical_offset: layout.layout.textbox_vertical_offset,
            textbox_transform: fennel_textbox_transform(world.matrix, is_2d, camera_bridge),
            secondary_color,
            textbox_flags: draw_textbox_flags,
            clip_size: [properties.layout.box_width, properties.layout.box_height],
            draw_offset,
            effect_colors: [effect_color; 4],
            effect_offset,
        },
        |token| {
            let (font_slot_id, code) = fennel_slot_code_from_token(token);
            let resource_name = font_registry.resource_for_slot(font_slot_id)?;
            let runtime_font = runtime_fonts.get(resource_name.as_slice())?;
            runtime_font.glyph(code).copied()
        },
    )
    .map_err(|error| SrdDrawError(error.to_string()))?;
    if vertex_build.processed_glyph_count == 0 {
        return Ok(None);
    }
    let identity = identity_matrix4x4_game();
    let mut fixed_constants = CeylonSrdFixedShaderConstants::initial_for_target(
        host.target_projection_view,
        host.target_screen_size,
    );
    fixed_constants.vertex_c0_c3_world = if is_2d {
        identity
    } else {
        fennel_textbox_transform(world.matrix, false, camera_bridge)
    };
    // See the constructor/VS boundary above: c4..c7 is the previous TextBox
    // matrix, identity for this freshly built runtime, and is shader-dead.
    fixed_constants.vertex_c4_c7 = identity;
    Ok(Some(EvidenceCompleteFennelDraw {
        owner,
        scene_index: source.scene_index,
        layer_index: source.layer_index,
        node_index,
        font_name: font.name.clone(),
        is_2d,
        renderer_layer_key,
        packet: fennel_default_draw_packet(is_2d),
        fixed_constants,
        batches: vertex_build.batches,
    }))
}

const fn fennel_slot_code_token(font_slot_id: u16, code: u16) -> u32 {
    (font_slot_id as u32) << 16 | code as u32
}

const fn fennel_slot_code_from_token(token: u32) -> (u16, u16) {
    ((token >> 16) as u16, token as u16)
}

fn affine_to_matrix4x4(matrix: Affine3x4) -> Matrix4x4 {
    Matrix4x4 {
        rows: [
            matrix.rows[0],
            matrix.rows[1],
            matrix.rows[2],
            [0.0, 0.0, 0.0, 1.0],
        ],
    }
}

/// Reproduces the matrix written to TextBoxObject `+0x2EC` by `sub_AC5740`.
/// The 2D path copies the CAST world matrix. The 3D path multiplies
/// `SrRenderer+0x08 * CAST world` and then negates the complete Y column.
fn fennel_textbox_transform(world: Affine3x4, is_2d: bool, camera_bridge: Matrix4x4) -> Matrix4x4 {
    let world = affine_to_matrix4x4(world);
    if is_2d {
        return world;
    }
    let mut result = mul_matrix4x4_game(&camera_bridge, &world);
    for row in &mut result.rows {
        row[1] *= -1.0;
    }
    result
}

fn pack_fennel_record_color([red, green, blue, alpha]: [u8; 4]) -> u32 {
    // `sub_AC5740 -> sub_F25DA0` stores record colors as AARRGGBB, whose
    // little-endian bytes are B,G,R,A.
    u32::from_le_bytes([blue, green, red, alpha])
}

fn validate_host_draw_context(host: SrdHostDrawContext) -> Result<(), SrdDrawError> {
    if let Some(target) = host.renderer_project_target
        && target.render_size.contains(&0)
    {
        return Err(SrdDrawError(format!(
            "renderer project target size must be non-zero, got {}x{}",
            target.render_size[0], target.render_size[1]
        )));
    }
    if host.target_screen_size.contains(&0) {
        return Err(SrdDrawError(format!(
            "target screen size must be non-zero, got {}x{}",
            host.target_screen_size[0], host.target_screen_size[1]
        )));
    }
    Ok(())
}

fn renderer_project_camera_bridge(project: &Project, host: SrdHostDrawContext) -> Matrix4x4 {
    let Some(target) = host.renderer_project_target else {
        return identity_matrix4x4_game();
    };
    let external_inverse = inverse_matrix4x4_game(&target.projection_view);
    let srd_projection_view = project
        .camera
        .runtime_matrices(target.render_size[0] as f32)
        .projection_view;
    mul_matrix4x4_game(&external_inverse, &srd_projection_view)
}

fn renderer_inverse_camera_view(project: &Project, host: SrdHostDrawContext) -> Affine3x4 {
    let Some(target) = host.renderer_project_target else {
        return Affine3x4::IDENTITY;
    };
    let view = project
        .camera
        .runtime_matrices(target.render_size[0] as f32)
        .view;
    Affine3x4 {
        rows: [view.rows[0], view.rows[1], view.rows[2]],
    }
    .inverse_game()
}

fn renderer_project_screen_matrix(
    host: SrdHostDrawContext,
    camera_bridge: Matrix4x4,
) -> Result<Option<(Matrix4x4, [i32; 2])>, SrdDrawError> {
    let Some(target) = host.renderer_project_target else {
        return Ok(None);
    };
    let width = i32::try_from(target.render_size[0]).map_err(|_| {
        SrdDrawError(format!(
            "renderer project target width {} exceeds the game's signed i32 domain",
            target.render_size[0]
        ))
    })?;
    let height = i32::try_from(target.render_size[1]).map_err(|_| {
        SrdDrawError(format!(
            "renderer project target height {} exceeds the game's signed i32 domain",
            target.render_size[1]
        ))
    })?;
    Ok(Some((
        compose_screen_matrix_game(width, height, &target.projection_view, &camera_bridge),
        [width, height],
    )))
}

fn runtime_cast_passes_renderer_visibility(
    positions: [[f32; 3]; 4],
    is_2d: bool,
    project_screen: Option<&(Matrix4x4, [i32; 2])>,
) -> Result<bool, SrdDrawError> {
    let Some((screen_matrix, [width, height])) = project_screen else {
        // The binary reads SrRenderer+0x24C even on this proven null-target
        // path, but those four rectangle floats were never initialized. Keep
        // the editor deterministic and memory-safe instead of inventing a
        // zero/present/receiving-target rectangle.
        return Ok(true);
    };
    cast_overlaps_render_target_game(
        &positions,
        is_2d,
        (!is_2d).then_some(screen_matrix),
        *width,
        *height,
    )
    .map_err(|error| SrdDrawError(error.to_string()))
}

fn runtime_image_state_for_owner(
    runtime: &ProjectRuntime,
    owner: ReferenceLayerParent,
    node_index: usize,
) -> Result<crate::image::RuntimeImageState, SrdDrawError> {
    let states = match owner {
        ReferenceLayerParent::ProjectLayer(target) => runtime
            .project_layers
            .get(target.scene_index)
            .and_then(|scene| scene.get(target.layer_index))
            .map(|layer| layer.image_states.as_slice()),
        ReferenceLayerParent::ReferenceInstance(instance_index) => runtime
            .references
            .layers
            .get(instance_index)
            .map(|layer| layer.image_states.as_slice()),
    }
    .ok_or_else(|| SrdDrawError(format!("runtime draw owner {owner:?} is unavailable")))?;
    states.get(node_index).copied().ok_or_else(|| {
        SrdDrawError(format!(
            "runtime draw owner {owner:?} has no image state for NODE[{node_index}]"
        ))
    })
}

#[allow(clippy::too_many_arguments)]
fn build_evidence_complete_number_glyph_draws_for_runtime_cast(
    textures: &TextureList,
    owner: ReferenceLayerParent,
    source: ReferenceTarget,
    layer: &Layer,
    node_index: usize,
    renderer_layer_key: u32,
    is_2d: bool,
    world: RuntimeWorldState,
    image_state: crate::image::RuntimeImageState,
    host: SrdHostDrawContext,
    camera_bridge: Matrix4x4,
    project_screen: Option<&(Matrix4x4, [i32; 2])>,
    identity: Matrix4x4,
    packet_current_matrix: &mut Matrix4x4,
) -> Result<Vec<EvidenceCompleteNumberGlyphDraw>, SrdDrawError> {
    let Some(definition) = layer.number_by_node[node_index].as_ref() else {
        return Ok(Vec::new());
    };
    if !world.visible || !world.render_gate {
        return Ok(Vec::new());
    }

    let image = definition.image_base();
    let cast_positions = image
        .build_quad_with_geometry(image_state.geometry, is_2d)
        .positions
        .map(|point| world.matrix.transform_point_game(point));
    if !runtime_cast_passes_renderer_visibility(cast_positions, is_2d, project_screen)? {
        return Ok(Vec::new());
    }

    let preset = select_srd_image_render_preset(
        image.flags,
        image_state.render_preset_override,
        false,
    )
    .ok_or_else(|| {
        SrdDrawError(format!(
            "SCN[{}]/LAYR[{}]/NODE[{node_index}] NumberCast does not select a proven render preset",
            source.scene_index, source.layer_index
        ))
    })?;

    // `srd_render_number_cast` selects the texture pair once from the current
    // two SrImage descriptors before `srd_render_number_glyph` replaces only
    // channel 0's selector for per-glyph UV construction.
    let slots = image
        .resolve_texture_slots(
            &image_state,
            textures,
            crate::number::NumberDefinition::INITIAL_COORDINATE_OFFSET_SCALE,
            [false; 2],
        )
        .map_err(|error| SrdDrawError(error.to_string()))?;
    let mut texture_bindings = [None; 3];
    for (slot_index, destination) in texture_bindings.iter_mut().enumerate().take(2) {
        match slots.slots[slot_index] {
            Some(SrdTextureBindingSource::TextureList(texture_index)) => {
                let sampler = slots.channels[slot_index].selected_sampler.ok_or_else(|| {
                    SrdDrawError(format!(
                        "SCN[{}]/LAYR[{}]/NODE[{node_index}] NumberCast texture slot {slot_index} has no proven sampler",
                        source.scene_index, source.layer_index
                    ))
                })?;
                *destination = Some(EvidenceSrdTextureBinding {
                    texture_index,
                    sampler,
                });
            }
            Some(SrdTextureBindingSource::ExplicitOverride) => {
                return Err(SrdDrawError(format!(
                    "SCN[{}]/LAYR[{}]/NODE[{node_index}] NumberCast uses an unrecorded explicit texture override",
                    source.scene_index, source.layer_index
                )));
            }
            None => {}
        }
    }

    let second_coordinates = image
        .resolve_coordinates(
            ImageReferenceChannel::Cre1,
            image_state.coordinate_state(ImageReferenceChannel::Cre1),
            textures,
            crate::number::NumberDefinition::INITIAL_COORDINATE_OFFSET_SCALE,
        )
        .map_err(|error| SrdDrawError(error.to_string()))?;
    let formatted = definition.initial_formatted_text();
    let records = definition.first_history_render_records(&formatted, is_2d);
    let renderer_layer_prefix = renderer_layer_key & !0xff;
    let mut renderer_counter = renderer_layer_key as u8;
    let mut draws = Vec::with_capacity(records.len());
    for NumberGlyphRecord {
        glyph_index,
        is_digit: _,
        segment,
        mut quad,
    } in records
    {
        if glyph_index < 0 {
            continue;
        }

        let mut first_state = image_state.coordinate_state(ImageReferenceChannel::Cref);
        first_state.reference_index = glyph_index;
        first_state.uses_explicit_rectangle = false;
        let first_coordinates = image
            .resolve_coordinates(
                ImageReferenceChannel::Cref,
                first_state,
                textures,
                crate::number::NumberDefinition::INITIAL_COORDINATE_OFFSET_SCALE,
            )
            .map_err(|error| SrdDrawError(error.to_string()))?;
        quad.positions = quad
            .positions
            .map(|point| world.matrix.transform_point_game(point));
        let render_quad = definition.build_glyph_render_quad(
            quad,
            first_state,
            first_coordinates,
            second_coordinates,
            world.multiply_color,
            world.additive_color,
        );

        let mut fixed_constants = CeylonSrdFixedShaderConstants::initial_for_target(
            host.target_projection_view,
            host.target_screen_size,
        );
        if is_2d {
            fixed_constants.vertex_c0_c3_world = identity;
            fixed_constants.vertex_c4_c7 = identity;
            *packet_current_matrix = identity;
        } else {
            fixed_constants.vertex_c4_c7 = *packet_current_matrix;
            fixed_constants.vertex_c0_c3_world = camera_bridge;
            *packet_current_matrix = camera_bridge;
        }

        let mut packet = CeylonDrawPacketPresetState::srd_renderer_initial();
        packet.set_render_preset_id(preset);
        packet.set_srd_quad_is_2d(is_2d);
        apply_srd_image_field_0c_shader_bits(&mut packet, image_state.field_0c as i32);
        apply_srd_image_alpha_stencil_packet_fields(
            &mut packet,
            image_state.field_10,
            image_state.field_14,
            image_state.field_18,
            0,
            &mut renderer_counter,
        );
        let glyph_renderer_layer_key = renderer_layer_prefix | u32::from(renderer_counter);
        apply_srd_special_depth_packet_fields(&mut packet, false, image_state.field_1c);

        let shader_key = packet
            .srd_quad_shader_key(slots.texture_present())
            .srd_simple_shader_direct_contributions()
            .map_err(|error| SrdDrawError(format!("unsupported Simple mapping: {error:?}")))?
            .compact_key();
        let blend = ceylon_d3d9_blend_preset(i32::from(packet.table_preset_id()));
        let mut raster = CeylonRasterState::default();
        raster.apply_draw_packet(packet);
        draws.push(EvidenceCompleteNumberGlyphDraw {
            glyph_index,
            segment,
            draw: EvidenceCompleteSrdDraw {
                owner,
                scene_index: source.scene_index,
                layer_index: source.layer_index,
                node_index,
                is_2d,
                renderer_layer_key: glyph_renderer_layer_key,
                shader_key,
                quad: render_quad,
                packet,
                fixed_constants,
                blend,
                raster,
                depth: CeylonDepthState::from_draw_flags(packet.draw_flags_00),
                texture_bindings,
            },
        });
    }
    Ok(draws)
}

#[allow(clippy::too_many_arguments)]
fn build_evidence_complete_slice_cell_draws_for_runtime_cast(
    textures: &TextureList,
    owner: ReferenceLayerParent,
    source: ReferenceTarget,
    layer: &Layer,
    node_index: usize,
    renderer_layer_key: u32,
    is_2d: bool,
    world: RuntimeWorldState,
    image_state: crate::image::RuntimeImageState,
    host: SrdHostDrawContext,
    camera_bridge: Matrix4x4,
    project_screen: Option<&(Matrix4x4, [i32; 2])>,
    identity: Matrix4x4,
    packet_current_matrix: &mut Matrix4x4,
) -> Result<Vec<EvidenceCompleteSliceCellDraw>, SrdDrawError> {
    let Some(definition) = layer.csli_by_node[node_index].as_ref() else {
        return Ok(Vec::new());
    };
    if !world.visible || !world.render_gate {
        return Ok(Vec::new());
    }

    let image = ImageDefinition::from_csli_runtime_base(definition);
    let cast_positions = image
        .build_quad_with_geometry(image_state.geometry, is_2d)
        .positions
        .map(|point| world.matrix.transform_point_game(point));
    if !runtime_cast_passes_renderer_visibility(cast_positions, is_2d, project_screen)? {
        return Ok(Vec::new());
    }

    let mut quads = definition
        .generate_active_quads_with_geometry(
            is_2d,
            image_state.geometry.size,
            image_state.geometry.origin,
        )
        .map_err(|error| SrdDrawError(error.to_string()))?;
    let preset = select_srd_image_render_preset(
        image.flags,
        image_state.render_preset_override,
        false,
    )
    .ok_or_else(|| {
        SrdDrawError(format!(
            "SCN[{}]/LAYR[{}]/NODE[{node_index}] SliceCast does not select a proven render preset",
            source.scene_index, source.layer_index
        ))
    })?;

    let renderer_layer_prefix = renderer_layer_key & !0xff;
    let mut renderer_counter = renderer_layer_key as u8;
    let mut draws = Vec::with_capacity(quads.len());
    for mut slice_quad in quads.drain(..) {
        let cell_index = slice_quad.cell_index;
        let cell = definition.cells.get(cell_index).ok_or_else(|| {
            SrdDrawError(format!(
                "SCN[{}]/LAYR[{}]/NODE[{node_index}] SliceCast cell {cell_index} is outside SLIC",
                source.scene_index, source.layer_index
            ))
        })?;

        let resolved_texture = textures.resolve_slice_cell(definition, cell_index);
        let (texture_coordinates, texture_binding) = match resolved_texture {
            Some(resolved) => (
                resolved.coordinates,
                Some(EvidenceSrdTextureBinding {
                    texture_index: resolved.image_index,
                    sampler: resolved.samplers.select(image.point_sampled()),
                }),
            ),
            None => ([[0.0; 2]; 4], None),
        };

        let colors = slice_vertex_colors(
            definition,
            cell,
            slice_quad.normalized_cell_coordinates,
            world.multiply_color,
            world.additive_color,
        )
        .map_err(|error| SrdDrawError(error.to_string()))?;
        slice_quad.positions = slice_quad
            .positions
            .map(|point| world.matrix.transform_point_game(point));
        let quad = build_slice_render_quad(
            SliceQuad {
                cell_index,
                ..slice_quad
            },
            colors,
            texture_coordinates,
        );

        let mut fixed_constants = CeylonSrdFixedShaderConstants::initial_for_target(
            host.target_projection_view,
            host.target_screen_size,
        );
        if is_2d {
            fixed_constants.vertex_c0_c3_world = identity;
            fixed_constants.vertex_c4_c7 = identity;
            *packet_current_matrix = identity;
        } else {
            fixed_constants.vertex_c4_c7 = *packet_current_matrix;
            fixed_constants.vertex_c0_c3_world = camera_bridge;
            *packet_current_matrix = camera_bridge;
        }

        let mut packet = CeylonDrawPacketPresetState::srd_renderer_initial();
        packet.set_render_preset_id(preset);
        packet.set_srd_quad_is_2d(is_2d);
        apply_srd_image_field_0c_shader_bits(&mut packet, image_state.field_0c as i32);
        apply_srd_image_alpha_stencil_packet_fields(
            &mut packet,
            image_state.field_10,
            image_state.field_14,
            image_state.field_18,
            0,
            &mut renderer_counter,
        );
        let cell_renderer_layer_key = renderer_layer_prefix | u32::from(renderer_counter);
        apply_srd_special_depth_packet_fields(&mut packet, false, image_state.field_1c);

        let texture_bindings = [texture_binding, None, None];
        let shader_key = packet
            .srd_quad_shader_key([texture_binding.is_some(), false, false])
            .srd_simple_shader_direct_contributions()
            .map_err(|error| SrdDrawError(format!("unsupported Simple mapping: {error:?}")))?
            .compact_key();
        let blend = ceylon_d3d9_blend_preset(i32::from(packet.table_preset_id()));
        let mut raster = CeylonRasterState::default();
        raster.apply_draw_packet(packet);
        draws.push(EvidenceCompleteSliceCellDraw {
            cell_index,
            draw: EvidenceCompleteSrdDraw {
                owner,
                scene_index: source.scene_index,
                layer_index: source.layer_index,
                node_index,
                is_2d,
                renderer_layer_key: cell_renderer_layer_key,
                shader_key,
                quad,
                packet,
                fixed_constants,
                blend,
                raster,
                depth: CeylonDepthState::from_draw_flags(packet.draw_flags_00),
                texture_bindings,
            },
        });
    }
    Ok(draws)
}

#[allow(clippy::too_many_arguments)]
fn build_evidence_complete_image_draw_for_runtime_cast(
    textures: &TextureList,
    owner: ReferenceLayerParent,
    source: ReferenceTarget,
    layer: &Layer,
    node_index: usize,
    mut renderer_layer_key: u32,
    is_2d: bool,
    world: RuntimeWorldState,
    image_state: crate::image::RuntimeImageState,
    host: SrdHostDrawContext,
    camera_bridge: Matrix4x4,
    project_screen: Option<&(Matrix4x4, [i32; 2])>,
    identity: Matrix4x4,
    packet_current_matrix: &mut Matrix4x4,
) -> Result<Option<EvidenceCompleteSrdDraw>, SrdDrawError> {
    let Some(image) = layer.image_by_node[node_index]
        .as_ref()
        .filter(|image| !image.creates_text_cast())
    else {
        return Ok(None);
    };
    if !world.visible || !world.render_gate {
        return Ok(None);
    }

    let mut fixed_constants = CeylonSrdFixedShaderConstants::initial_for_target(
        host.target_projection_view,
        host.target_screen_size,
    );
    if is_2d {
        fixed_constants.vertex_c0_c3_world = identity;
        fixed_constants.vertex_c4_c7 = identity;
        *packet_current_matrix = identity;
    } else {
        fixed_constants.vertex_c4_c7 = *packet_current_matrix;
        fixed_constants.vertex_c0_c3_world = camera_bridge;
        *packet_current_matrix = camera_bridge;
    }

    let preset =
        select_srd_image_render_preset(image.flags, image_state.render_preset_override, false)
            .ok_or_else(|| {
                SrdDrawError(format!(
                    "SCN[{}]/LAYR[{}]/NODE[{node_index}] does not select a proven render preset",
                    source.scene_index, source.layer_index
                ))
            })?;
    let slots = image
        .resolve_texture_slots(
            &image_state,
            textures,
            ImageDefinition::INITIAL_COORDINATE_OFFSET_SCALE,
            [false; 2],
        )
        .map_err(|error| SrdDrawError(error.to_string()))?;
    let mut texture_bindings = [None; 3];
    for (slot_index, destination) in texture_bindings.iter_mut().enumerate().take(2) {
        match slots.slots[slot_index] {
            Some(SrdTextureBindingSource::TextureList(texture_index)) => {
                let sampler = slots.channels[slot_index].selected_sampler.ok_or_else(|| {
                    SrdDrawError(format!(
                        "SCN[{}]/LAYR[{}]/NODE[{node_index}] texture slot {slot_index} has no proven sampler",
                        source.scene_index, source.layer_index
                    ))
                })?;
                *destination = Some(EvidenceSrdTextureBinding {
                    texture_index,
                    sampler,
                });
            }
            Some(SrdTextureBindingSource::ExplicitOverride) => continue,
            None => {}
        }
    }
    if slots
        .slots
        .iter()
        .any(|slot| matches!(slot, Some(SrdTextureBindingSource::ExplicitOverride)))
    {
        return Ok(None);
    }

    let mut packet = CeylonDrawPacketPresetState::srd_renderer_initial();
    packet.set_render_preset_id(preset);
    packet.set_srd_quad_is_2d(is_2d);
    apply_srd_image_field_0c_shader_bits(&mut packet, image_state.field_0c as i32);
    let mut renderer_counter = renderer_layer_key as u8;
    apply_srd_image_alpha_stencil_packet_fields(
        &mut packet,
        image_state.field_10,
        image_state.field_14,
        image_state.field_18,
        0,
        &mut renderer_counter,
    );
    renderer_layer_key = (renderer_layer_key & !0xff) | u32::from(renderer_counter);
    apply_srd_special_depth_packet_fields(&mut packet, false, image_state.field_1c);

    let shader_key = packet
        .srd_quad_shader_key(slots.texture_present())
        .srd_simple_shader_direct_contributions()
        .map_err(|error| SrdDrawError(format!("unsupported Simple mapping: {error:?}")))?
        .compact_key();
    let blend = ceylon_d3d9_blend_preset(i32::from(packet.table_preset_id()));

    let local_positions = image
        .build_quad_with_geometry(image_state.geometry, is_2d)
        .positions;
    let positions = local_positions.map(|point| world.matrix.transform_point_game(point));
    if !runtime_cast_passes_renderer_visibility(positions, is_2d, project_screen)? {
        return Ok(None);
    }
    let quad = image.build_render_quad_from_positions(
        positions,
        image_state.coordinate_state(ImageReferenceChannel::Cref),
        slots.channels[0],
        slots.channels[1],
        world.multiply_color,
        world.additive_color,
    );
    let mut raster = CeylonRasterState::default();
    raster.apply_draw_packet(packet);
    Ok(Some(EvidenceCompleteSrdDraw {
        owner,
        scene_index: source.scene_index,
        layer_index: source.layer_index,
        node_index,
        is_2d,
        renderer_layer_key,
        shader_key,
        quad,
        packet,
        fixed_constants,
        blend,
        raster,
        depth: CeylonDepthState::from_draw_flags(packet.draw_flags_00),
        texture_bindings,
    }))
}

fn build_evidence_complete_image_draws(
    project: &Project,
    textures: &TextureList,
    scene_index: usize,
    host: SrdHostDrawContext,
    runtime_layers: Option<&[ProjectLayerRuntimeState]>,
) -> Result<Vec<EvidenceCompleteSrdDraw>, SrdDrawError> {
    validate_host_draw_context(host)?;
    let mut composition_runtime = ProjectRuntime::new(project)
        .map_err(|error| SrdDrawError(format!("failed to construct SRD runtime: {error}")))?;
    if let Some(runtime_layers) = runtime_layers {
        let destination = composition_runtime
            .project_layers
            .get_mut(scene_index)
            .ok_or_else(|| {
                SrdDrawError(format!("scene index {scene_index} is outside the runtime"))
            })?;
        if destination.len() != runtime_layers.len() {
            return Err(SrdDrawError(format!(
                "SCN[{scene_index}] has {} runtime layers, expected {}",
                runtime_layers.len(),
                destination.len()
            )));
        }
        destination.clone_from_slice(runtime_layers);
    }
    let renderer_inverse_camera_view = renderer_inverse_camera_view(project, host);
    let composed_worlds = composition_runtime
        .compose_world_states(
            project,
            host.first_calc_matrix,
            renderer_inverse_camera_view,
        )
        .map_err(|error| SrdDrawError(error.to_string()))?;
    let scene = project
        .scenes
        .get(scene_index)
        .ok_or_else(|| SrdDrawError(format!("scene index {scene_index} is outside the project")))?;
    let camera_bridge = renderer_project_camera_bridge(project, host);
    let project_screen = renderer_project_screen_matrix(host, camera_bridge)?;
    let identity = identity_matrix4x4_game();
    let mut packet_current_matrix = identity;
    let mut draws = Vec::new();

    for (layer_index, layer) in scene.layers.iter().enumerate() {
        let cast_layer_keys = layer
            .compose_runtime_cast_layer_keys(host.renderer_layer_key)
            .map_err(|error| SrdDrawError(error.to_string()))?;
        let runtime_layer = runtime_layers
            .map(|layers| {
                layers.get(layer_index).ok_or_else(|| {
                    SrdDrawError(format!(
                        "SCN[{scene_index}]/LAYR[{layer_index}] is missing from the runtime"
                    ))
                })
            })
            .transpose()?;
        let layer_enabled = runtime_layer
            .map(|runtime_layer| runtime_layer.enabled)
            .unwrap_or(layer.flags & 0x100 != 0);
        if !layer_enabled {
            continue;
        }
        let is_2d = layer.is_2d();
        let layer_worlds = composed_worlds
            .project_layers
            .get(scene_index)
            .and_then(|layers| layers.get(layer_index))
            .ok_or_else(|| {
                SrdDrawError(format!(
                    "SCN[{scene_index}]/LAYR[{layer_index}] has no composed runtime world state"
                ))
            })?;

        for node_index in 0..layer.nodes.len() {
            let Some(image) = layer.image_by_node[node_index]
                .as_ref()
                .filter(|image| !image.creates_text_cast())
            else {
                continue;
            };
            let cast_world = layer_worlds.casts[node_index];
            if !cast_world.render_gate {
                continue;
            }

            let mut fixed_constants = CeylonSrdFixedShaderConstants::initial_for_target(
                host.target_projection_view,
                host.target_screen_size,
            );
            if is_2d {
                fixed_constants.vertex_c0_c3_world = identity;
                fixed_constants.vertex_c4_c7 = identity;
                packet_current_matrix = identity;
            } else {
                fixed_constants.vertex_c4_c7 = packet_current_matrix;
                fixed_constants.vertex_c0_c3_world = camera_bridge;
                packet_current_matrix = camera_bridge;
            }

            let image_state = runtime_layer.map_or_else(
                || {
                    let mut image_state = image.initial_runtime_state();
                    if let Some(ext_param) = layer.ext_param_for_node(node_index) {
                        image_state.render_preset_override = ext_param.render_preset_override;
                    }
                    image_state
                },
                |runtime_layer| runtime_layer.image_states[node_index],
            );
            let preset = select_srd_image_render_preset(
                image.flags,
                image_state.render_preset_override,
                false,
            )
            .ok_or_else(|| {
                SrdDrawError(format!(
                    "SCN[{scene_index}]/LAYR[{layer_index}]/NODE[{node_index}] does not select a proven render preset"
                ))
            })?;
            let slots = image
                .resolve_texture_slots(
                    &image_state,
                    textures,
                    ImageDefinition::INITIAL_COORDINATE_OFFSET_SCALE,
                    [false; 2],
                )
                .map_err(|error| SrdDrawError(error.to_string()))?;
            let mut texture_bindings = [None; 3];
            for (slot_index, destination) in texture_bindings.iter_mut().enumerate().take(2) {
                match slots.slots[slot_index] {
                    Some(SrdTextureBindingSource::TextureList(texture_index)) => {
                        let sampler = slots.channels[slot_index].selected_sampler.ok_or_else(|| {
                            SrdDrawError(format!(
                                "SCN[{scene_index}]/LAYR[{layer_index}]/NODE[{node_index}] texture slot {slot_index} has no proven sampler"
                            ))
                        })?;
                        *destination = Some(EvidenceSrdTextureBinding {
                            texture_index,
                            sampler,
                        });
                    }
                    Some(SrdTextureBindingSource::ExplicitOverride) => continue,
                    None => {}
                }
            }
            if slots
                .slots
                .iter()
                .any(|slot| matches!(slot, Some(SrdTextureBindingSource::ExplicitOverride)))
            {
                continue;
            }

            let mut renderer_layer_key = cast_layer_keys[node_index]
                .wrapping_add(u32::from(layer.nodes[node_index].render_layer_offset()));
            let mut packet = CeylonDrawPacketPresetState::srd_renderer_initial();
            packet.set_render_preset_id(preset);
            packet.set_srd_quad_is_2d(is_2d);
            apply_srd_image_field_0c_shader_bits(&mut packet, image_state.field_0c as i32);
            let mut renderer_counter = renderer_layer_key as u8;
            apply_srd_image_alpha_stencil_packet_fields(
                &mut packet,
                image_state.field_10,
                image_state.field_14,
                image_state.field_18,
                0,
                &mut renderer_counter,
            );
            renderer_layer_key = (renderer_layer_key & !0xff) | u32::from(renderer_counter);
            apply_srd_special_depth_packet_fields(&mut packet, false, image_state.field_1c);

            let shader_key = packet
                .srd_quad_shader_key(slots.texture_present())
                .srd_simple_shader_direct_contributions()
                .map_err(|error| SrdDrawError(format!("unsupported Simple mapping: {error:?}")))?
                .compact_key();
            if embedded_simple_shader_pair(&shader_key).is_none() {
                continue;
            }
            let blend = ceylon_d3d9_blend_preset(i32::from(packet.table_preset_id()));
            if blend.alpha_test_enabled || packet.flags_0c & 0x100 != 0 {
                continue;
            }

            let local_positions = image
                .build_quad_with_geometry(image_state.geometry, is_2d)
                .positions;
            let positions =
                local_positions.map(|point| cast_world.matrix.transform_point_game(point));
            if !runtime_cast_passes_renderer_visibility(positions, is_2d, project_screen.as_ref())?
            {
                continue;
            }
            let quad = image.build_render_quad_from_positions(
                positions,
                image_state.coordinate_state(ImageReferenceChannel::Cref),
                slots.channels[0],
                slots.channels[1],
                cast_world.multiply_color,
                cast_world.additive_color,
            );
            let mut raster = CeylonRasterState::default();
            raster.apply_draw_packet(packet);
            draws.push(EvidenceCompleteSrdDraw {
                owner: ReferenceLayerParent::ProjectLayer(crate::scene::ReferenceTarget {
                    scene_index,
                    layer_index,
                }),
                scene_index,
                layer_index,
                node_index,
                is_2d,
                renderer_layer_key,
                shader_key,
                quad,
                packet,
                fixed_constants,
                blend,
                raster,
                depth: CeylonDepthState::from_draw_flags(packet.draw_flags_00),
                texture_bindings,
            });
        }
    }
    Ok(draws)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attribute::{CastAttribute, CastAttributeList};
    use crate::camera::CameraDefinition;
    use crate::csli::{CrefEntry, CsliDefinition, SlicCell};
    use crate::image::ImageDefinition;
    use crate::reference::ReferenceDefinition;
    use crate::ruhuna::RuhunaRuntimeGlyphRecord;
    use crate::scene::{NodeRecord, RawTransform, Scene};
    use crate::text::{FontDefinition, TextDefinition};
    use crate::texture::{TextureCrop, TextureDefinition};
    use crate::transform::SpatialTransform;

    fn text_image(node_index: i32, font_index: i32) -> ImageDefinition {
        ImageDefinition {
            flags: 0x100,
            width: 128.0,
            height: 128.0,
            custom_origin: [0.0; 2],
            origin_mode: 0,
            vertex_colors: [[0xff; 4]; 4],
            cref_index: -1,
            cref_count: 0,
            crefs: Vec::new(),
            field_4c: 0,
            cre1_index: -1,
            cre1_count: 0,
            cre1s: Vec::new(),
            coordinate_offsets: [[0.0; 2]; 2],
            field_a1: 0,
            node_index,
            has_text_child: true,
            text: Some(TextDefinition {
                field_78: None,
                font_index: Some(font_index),
                text: Vec::new(),
                field_36: None,
                field_7b: None,
                field_7c: None,
                field_41: None,
            }),
        }
    }

    fn node() -> NodeRecord {
        NodeRecord {
            name: None,
            type_flags: Some(1),
            parent_csli_cell_index: None,
            first_child_index: -1,
            next_sibling_index: -1,
            field_a0: None,
        }
    }

    fn plain_image(node_index: i32) -> ImageDefinition {
        ImageDefinition {
            flags: 0,
            width: 100.0,
            height: 50.0,
            custom_origin: [0.0; 2],
            origin_mode: 0,
            vertex_colors: [[0xff; 4]; 4],
            cref_index: -1,
            cref_count: 0,
            crefs: Vec::new(),
            field_4c: 0,
            cre1_index: -1,
            cre1_count: 0,
            cre1s: Vec::new(),
            coordinate_offsets: [[0.0; 2]; 2],
            field_a1: 0,
            node_index,
            has_text_child: false,
            text: None,
        }
    }

    fn font(name: &[u8]) -> crate::text::FontDefinition {
        FontDefinition {
            name: name.to_vec(),
            flags_70: None,
            field_71: None,
            characters: Vec::new(),
        }
    }

    #[test]
    fn fennel_font_requests_follow_runtime_cast_then_catr_order() {
        let attribute_list = CastAttributeList {
            node_index: Some(0),
            declared_count: 4,
            attributes: vec![
                CastAttribute {
                    name: b"rubyFont".to_vec(),
                    source_type_code: 2,
                    value: CastAttributeValue::String(b"ruby.rfz".to_vec()),
                },
                CastAttribute {
                    name: b"ignoredFont".to_vec(),
                    source_type_code: 2,
                    value: CastAttributeValue::String(b"ignored.rfz".to_vec()),
                },
                CastAttribute {
                    name: b"rfzOutlineFont".to_vec(),
                    source_type_code: 2,
                    value: CastAttributeValue::String(b"outline.rfz".to_vec()),
                },
                CastAttribute {
                    name: b"rfzOutlineRubyFont".to_vec(),
                    source_type_code: 2,
                    value: CastAttributeValue::String(Vec::new()),
                },
            ],
        };
        let layer = Layer {
            name: b"layer".to_vec(),
            flags: 0,
            animation_count: 0,
            animations: Vec::new(),
            field_23: Vec::new(),
            nodes: vec![node(), node(), node()],
            transforms: Vec::new(),
            image_by_node: vec![
                Some(text_image(0, 1)),
                Some(text_image(1, 0)),
                Some(text_image(2, 1)),
            ],
            number_by_node: vec![None, None, None],
            reference_by_node: vec![None, None, None],
            csli_by_node: vec![None, None, None],
            cast_attribute_lists: vec![attribute_list],
            cast_attribute_list_by_node: vec![Some(0), None, None],
        };
        let project = Project {
            name: b"project".to_vec(),
            declared_scene_count: 1,
            declared_font_count: 2,
            camera: CameraDefinition::default(),
            scenes: vec![Scene {
                name: b"scene".to_vec(),
                declared_layer_count: 1,
                declared_animation_set_count: 0,
                width: 1920.0,
                height: 1080.0,
                layers: vec![layer],
                animation_sets: Vec::new(),
            }],
            fonts: vec![font(b"primary_a.rfz"), font(b"primary_b.rfz")],
        };

        let requests = collect_fennel_font_resource_requests(&project).unwrap();
        assert_eq!(
            requests
                .iter()
                .map(|request| (request.node_index, request.role, request.name.as_slice()))
                .collect::<Vec<_>>(),
            vec![
                (0, FennelTextFontRole::Primary, b"primary_b.rfz".as_slice()),
                (0, FennelTextFontRole::Ruby, b"ruby.rfz".as_slice()),
                (0, FennelTextFontRole::Outline, b"outline.rfz".as_slice()),
                (1, FennelTextFontRole::Primary, b"primary_a.rfz".as_slice()),
                (2, FennelTextFontRole::Primary, b"primary_b.rfz".as_slice()),
            ]
        );

        let mut registry = FennelFontSlotRegistry::default();
        let assignments = assign_fennel_font_resource_requests(&mut registry, requests);
        assert_eq!(
            assignments
                .iter()
                .map(|assignment| assignment.slot.font_slot_id)
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 3, 0]
        );
        assert_eq!(
            assignments[4].slot.resource_handle,
            assignments[0].slot.resource_handle
        );
        assert_eq!(assignments[4].slot.lease_count, 2);
    }

    #[test]
    fn fennel_runtime_text_input_manual_defaults_keep_slots_explicit_and_clock_zero() {
        let input = FennelSrdRuntimeTextInput::default();
        assert!(input.substitutions.iter().all(Vec::is_empty));
        assert_eq!(input.default_d, FENNEL_DEFAULT_D_VALUE);
        assert_eq!(input.repeat_space_count, FENNEL_DEFAULT_REPEAT_SPACE_COUNT);
        assert_eq!(input.field_f4.to_bits(), 0.0f32.to_bits());
    }

    #[test]
    fn reference_image_builder_draws_the_independent_copy_at_the_refcast_position() {
        let root = Layer {
            name: b"root".to_vec(),
            flags: 0x100,
            animation_count: 0,
            animations: Vec::new(),
            field_23: Vec::new(),
            nodes: vec![NodeRecord {
                type_flags: Some(3),
                ..node()
            }],
            transforms: vec![RawTransform::Trs2(SpatialTransform {
                translation: [10.0, 20.0, 0.0],
                ..SpatialTransform::default()
            })],
            image_by_node: vec![None],
            number_by_node: vec![None],
            reference_by_node: vec![Some(ReferenceDefinition {
                source_name: b"scene".to_vec(),
                layer_name: b"target".to_vec(),
                animation_enabled: 0,
                animation_name: Vec::new(),
                default_frame: 0.0,
                node_index: 0,
            })],
            csli_by_node: vec![None],
            cast_attribute_lists: Vec::new(),
            cast_attribute_list_by_node: vec![None],
        };
        // The source target is disabled in the original runtime scene table;
        // its copied layer is independently enabled by reference binding.
        let target = Layer {
            name: b"target".to_vec(),
            flags: 0,
            animation_count: 0,
            animations: Vec::new(),
            field_23: Vec::new(),
            nodes: vec![node()],
            transforms: vec![RawTransform::Trs2(SpatialTransform {
                translation: [5.0, 7.0, 0.0],
                ..SpatialTransform::default()
            })],
            image_by_node: vec![Some(plain_image(0))],
            number_by_node: vec![None],
            reference_by_node: vec![None],
            csli_by_node: vec![None],
            cast_attribute_lists: Vec::new(),
            cast_attribute_list_by_node: vec![None],
        };
        let project = Project {
            name: b"project".to_vec(),
            declared_scene_count: 1,
            declared_font_count: 0,
            camera: CameraDefinition::default(),
            scenes: vec![Scene {
                name: b"scene".to_vec(),
                declared_layer_count: 2,
                declared_animation_set_count: 0,
                width: 1920.0,
                height: 1080.0,
                layers: vec![root, target],
                animation_sets: Vec::new(),
            }],
            fonts: Vec::new(),
        };
        let draws = build_evidence_complete_initial_reference_image_draws(
            &project,
            &TextureList {
                declared_count: 0,
                textures: Vec::new(),
            },
            0,
            SrdHostDrawContext::new(
                Affine3x4::IDENTITY,
                crate::render::SRD_RENDERER_INITIAL_LAYER_KEY,
                Some(SrdRendererProjectTargetContext::new(
                    identity_matrix4x4_game(),
                    [1920, 1080],
                )),
                identity_matrix4x4_game(),
                [1920, 1080],
            ),
        )
        .unwrap();
        assert_eq!(draws.len(), 1);
        assert_eq!(draws[0].owner, ReferenceLayerParent::ReferenceInstance(0));
        assert_eq!((draws[0].scene_index, draws[0].layer_index), (0, 1));
        assert_eq!(draws[0].node_index, 0);
        assert_eq!(draws[0].quad.vertices[0].position, [15.0, 27.0, 0.0]);

        let culled = build_evidence_complete_initial_reference_image_draws(
            &project,
            &TextureList {
                declared_count: 0,
                textures: Vec::new(),
            },
            0,
            SrdHostDrawContext::new(
                Affine3x4::IDENTITY,
                crate::render::SRD_RENDERER_INITIAL_LAYER_KEY,
                Some(SrdRendererProjectTargetContext::new(
                    identity_matrix4x4_game(),
                    [10, 10],
                )),
                identity_matrix4x4_game(),
                [10, 10],
            ),
        )
        .unwrap();
        assert!(culled.is_empty());
    }

    #[test]
    fn copied_fennel_instances_use_independent_runtime_text_inputs() {
        let reference = |node_index| {
            Some(ReferenceDefinition {
                source_name: b"scene".to_vec(),
                layer_name: b"target".to_vec(),
                animation_enabled: 0,
                animation_name: Vec::new(),
                default_frame: 0.0,
                node_index,
            })
        };
        let root = Layer {
            name: b"root".to_vec(),
            flags: 0x100,
            animation_count: 0,
            animations: Vec::new(),
            field_23: Vec::new(),
            nodes: vec![
                node(),
                NodeRecord {
                    type_flags: Some(3),
                    ..node()
                },
                node(),
                NodeRecord {
                    type_flags: Some(3),
                    ..node()
                },
            ],
            transforms: vec![
                RawTransform::Trs2(SpatialTransform::default()),
                RawTransform::Trs2(SpatialTransform::default()),
                RawTransform::Trs2(SpatialTransform::default()),
                RawTransform::Trs2(SpatialTransform {
                    translation: [100.0, 0.0, 0.0],
                    ..SpatialTransform::default()
                }),
            ],
            image_by_node: vec![Some(plain_image(0)), None, Some(plain_image(2)), None],
            number_by_node: vec![None, None, None, None],
            reference_by_node: vec![None, reference(1), None, reference(3)],
            csli_by_node: vec![None, None, None, None],
            cast_attribute_lists: Vec::new(),
            cast_attribute_list_by_node: vec![None, None, None, None],
        };
        let mut target_text = text_image(0, 0);
        let text = target_text.text.as_mut().unwrap();
        text.text = b"$[0]".to_vec();
        text.field_78 = Some(0);
        text.field_36 = Some([1.0, 1.0]);
        text.field_7c = Some(0);
        text.field_41 = Some(0);
        let target = Layer {
            name: b"target".to_vec(),
            flags: 0,
            animation_count: 0,
            animations: Vec::new(),
            field_23: Vec::new(),
            nodes: vec![node()],
            transforms: vec![RawTransform::Trs2(SpatialTransform::default())],
            image_by_node: vec![Some(target_text)],
            number_by_node: vec![None],
            reference_by_node: vec![None],
            csli_by_node: vec![None],
            cast_attribute_lists: Vec::new(),
            cast_attribute_list_by_node: vec![None],
        };
        let project = Project {
            name: b"project".to_vec(),
            declared_scene_count: 1,
            declared_font_count: 1,
            camera: CameraDefinition::default(),
            scenes: vec![Scene {
                name: b"scene".to_vec(),
                declared_layer_count: 2,
                declared_animation_set_count: 0,
                width: 1920.0,
                height: 1080.0,
                layers: vec![root, target],
                animation_sets: Vec::new(),
            }],
            fonts: vec![font(b"test.rfz")],
        };
        let glyph = |code, texture_token| RuhunaRuntimeGlyphRecord {
            code,
            texture_token,
            enabled: 1,
            point_x: 10,
            point_y: 12,
            width: 10,
            height: 12,
            line_height: 12,
            advance_x: 10,
            uv0: [0.0, 0.0],
            uv1: [1.0, 0.0],
            uv2: [0.0, 1.0],
            uv3: [1.0, 1.0],
            ..Default::default()
        };
        let runtime_font = RuhunaRuntimeFont {
            minimum_code: u16::from(b'A'),
            maximum_code: u16::from(b'B'),
            dense_glyph_indices: vec![0, 1],
            glyph_pages: vec![Some(0), Some(0)],
            glyphs: vec![glyph(u16::from(b'A'), 11), glyph(u16::from(b'B'), 22)],
        };
        let mut runtime_fonts = BTreeMap::new();
        runtime_fonts.insert(b"test.rfz".to_vec(), runtime_font);
        let mut font_registry = FennelFontSlotRegistry::default();
        font_registry.request(b"test.rfz".to_vec());
        let mut inputs = BTreeMap::new();
        for (instance_index, substitution) in
            [b"A".as_slice(), b"B".as_slice()].into_iter().enumerate()
        {
            let mut input = FennelSrdRuntimeTextInput::default();
            input.substitutions[0] = substitution.to_vec();
            inputs.insert(
                FennelRuntimeTextCastKey {
                    owner: ReferenceLayerParent::ReferenceInstance(instance_index),
                    node_index: 0,
                },
                input,
            );
        }
        let draws = build_evidence_complete_initial_reference_fennel_draws(
            &project,
            0,
            SrdHostDrawContext::new(
                Affine3x4::IDENTITY,
                crate::render::SRD_RENDERER_INITIAL_LAYER_KEY,
                Some(SrdRendererProjectTargetContext::new(
                    identity_matrix4x4_game(),
                    [1920, 1080],
                )),
                identity_matrix4x4_game(),
                [1920, 1080],
            ),
            &font_registry,
            &runtime_fonts,
            false,
            &inputs,
        )
        .unwrap();
        assert_eq!(draws.len(), 2);
        assert_eq!(
            draws.iter().map(|draw| draw.owner).collect::<Vec<_>>(),
            vec![
                ReferenceLayerParent::ReferenceInstance(0),
                ReferenceLayerParent::ReferenceInstance(1),
            ]
        );
        assert_eq!(
            draws
                .iter()
                .map(|draw| draw.batches[0].texture_token)
                .collect::<Vec<_>>(),
            vec![11, 22]
        );

        let culled = build_evidence_complete_initial_reference_fennel_draws(
            &project,
            0,
            SrdHostDrawContext::new(
                Affine3x4::IDENTITY,
                crate::render::SRD_RENDERER_INITIAL_LAYER_KEY,
                Some(SrdRendererProjectTargetContext::new(
                    identity_matrix4x4_game(),
                    [10, 10],
                )),
                identity_matrix4x4_game(),
                [10, 10],
            ),
            &font_registry,
            &runtime_fonts,
            false,
            &inputs,
        )
        .unwrap();
        assert_eq!(culled.len(), 1);
        assert_eq!(culled[0].owner, ReferenceLayerParent::ReferenceInstance(0));

        let runtime_cast_draws = build_evidence_complete_initial_runtime_cast_draws(
            &project,
            &TextureList {
                declared_count: 0,
                textures: Vec::new(),
            },
            0,
            SrdHostDrawContext::new(
                Affine3x4::IDENTITY,
                crate::render::SRD_RENDERER_INITIAL_LAYER_KEY,
                Some(SrdRendererProjectTargetContext::new(
                    identity_matrix4x4_game(),
                    [1920, 1080],
                )),
                identity_matrix4x4_game(),
                [1920, 1080],
            ),
            &font_registry,
            &runtime_fonts,
            false,
            &inputs,
        )
        .unwrap();
        assert_eq!(runtime_cast_draws.len(), 4);
        assert!(matches!(
            &runtime_cast_draws[0],
            EvidenceCompleteRuntimeCastDraw::Image(draw)
                if draw.owner == ReferenceLayerParent::ProjectLayer(ReferenceTarget {
                    scene_index: 0,
                    layer_index: 0,
                }) && draw.node_index == 0
        ));
        assert!(matches!(
            &runtime_cast_draws[1],
            EvidenceCompleteRuntimeCastDraw::Fennel(draw)
                if draw.owner == ReferenceLayerParent::ReferenceInstance(0)
        ));
        assert!(matches!(
            &runtime_cast_draws[2],
            EvidenceCompleteRuntimeCastDraw::Image(draw) if draw.node_index == 2
        ));
        assert!(matches!(
            &runtime_cast_draws[3],
            EvidenceCompleteRuntimeCastDraw::Fennel(draw)
                if draw.owner == ReferenceLayerParent::ReferenceInstance(1)
        ));

        let expected_target_commands = runtime_cast_draws
            .iter()
            .enumerate()
            .flat_map(|(runtime_draw_index, draw)| match draw {
                EvidenceCompleteRuntimeCastDraw::Image(_) => {
                    vec![EvidenceRuntimeTargetCommandSource::Image { runtime_draw_index }]
                }
                EvidenceCompleteRuntimeCastDraw::SliceCell(_) => {
                    vec![EvidenceRuntimeTargetCommandSource::SliceCell { runtime_draw_index }]
                }
                EvidenceCompleteRuntimeCastDraw::NumberGlyph(_) => {
                    vec![EvidenceRuntimeTargetCommandSource::NumberGlyph { runtime_draw_index }]
                }
                EvidenceCompleteRuntimeCastDraw::Fennel(draw) => (0..draw.batches.len())
                    .map(
                        |batch_index| EvidenceRuntimeTargetCommandSource::FennelBatch {
                            runtime_draw_index,
                            batch_index,
                        },
                    )
                    .collect(),
            })
            .collect::<Vec<_>>();
        let profile = crate::game_host::CHUSAN_MAIN_SCENE
            .scene_pass_profile()
            .unwrap();
        assert_eq!(
            build_evidence_runtime_target_submission(&runtime_cast_draws, &profile).unwrap(),
            expected_target_commands
        );
        assert_eq!(
            build_evidence_filtered_runtime_target_submission(
                &runtime_cast_draws,
                &profile,
                crate::game_host::CHUSAN_ADVERTISE_LOGO_PLAYER
                    .initial_srd_target_filter(crate::game_host::CHUSAN_MAIN_SCENE),
            )
            .unwrap(),
            expected_target_commands
        );
        let background_profile = crate::game_host::CHUSAN_BG_SCENE
            .scene_pass_profile()
            .unwrap();
        assert!(
            build_evidence_filtered_runtime_target_submission(
                &runtime_cast_draws,
                &background_profile,
                crate::game_host::CHUSAN_ADVERTISE_LOGO_PLAYER
                    .initial_srd_target_filter(crate::game_host::CHUSAN_BG_SCENE),
            )
            .unwrap()
            .is_empty()
        );
        let merged = build_evidence_filtered_merged_runtime_target_submission(
            &runtime_cast_draws,
            &profile,
            crate::game_host::CHUSAN_ADVERTISE_LOGO_PLAYER
                .initial_srd_target_filter(crate::game_host::CHUSAN_MAIN_SCENE),
        )
        .unwrap();
        assert_eq!(
            merged
                .iter()
                .flat_map(|group| group.sources.iter().copied())
                .collect::<Vec<_>>(),
            expected_target_commands
        );
        for group in &merged {
            let source_vertex_count = group
                .sources
                .iter()
                .map(|source| match *source {
                    EvidenceRuntimeTargetCommandSource::Image { .. } => 4,
                    EvidenceRuntimeTargetCommandSource::SliceCell { .. } => 4,
                    EvidenceRuntimeTargetCommandSource::NumberGlyph { .. } => 4,
                    EvidenceRuntimeTargetCommandSource::FennelBatch {
                        runtime_draw_index,
                        batch_index,
                    } => match &runtime_cast_draws[runtime_draw_index] {
                        EvidenceCompleteRuntimeCastDraw::Fennel(draw) => {
                            draw.batches[batch_index].vertices.len()
                        }
                        EvidenceCompleteRuntimeCastDraw::Image(_)
                        | EvidenceCompleteRuntimeCastDraw::SliceCell(_)
                        | EvidenceCompleteRuntimeCastDraw::NumberGlyph(_) => unreachable!(),
                    },
                })
                .sum::<usize>();
            let connector_count = if group.primitive_type == 4 {
                (group.sources.len() - 1) * 2
            } else {
                0
            };
            assert_eq!(group.vertex_count, source_vertex_count + connector_count);
        }
        assert!(
            build_evidence_filtered_merged_runtime_target_submission(
                &runtime_cast_draws,
                &background_profile,
                crate::game_host::CHUSAN_ADVERTISE_LOGO_PLAYER
                    .initial_srd_target_filter(crate::game_host::CHUSAN_BG_SCENE),
            )
            .unwrap()
            .is_empty()
        );
        for draw in &runtime_cast_draws {
            let packet = match draw {
                EvidenceCompleteRuntimeCastDraw::Image(draw) => draw.packet,
                EvidenceCompleteRuntimeCastDraw::SliceCell(draw) => draw.draw.packet,
                EvidenceCompleteRuntimeCastDraw::NumberGlyph(draw) => draw.draw.packet,
                EvidenceCompleteRuntimeCastDraw::Fennel(draw) => draw.packet,
            };
            assert_eq!(packet.flags_60 & 0x2000, 0);
            assert_eq!(packet.flags_64, 0);
        }
    }

    #[test]
    fn slice_cast_emits_one_runtime_draw_per_active_cell() {
        let cell = |cref_index| SlicCell {
            flags: 0x100,
            explicit_width: 0.0,
            explicit_height: 0.0,
            field_3a: Some([0xff; 4]),
            field_33: None,
            field_44: vec![[0xff; 4]; 4],
            cref_index,
        };
        let csli = CsliDefinition {
            field_80: 0,
            width: 20.0,
            height: 10.0,
            custom_origin: [0.0, 0.0],
            field_44: [[0xff; 4]; 4],
            origin_mode: 0,
            columns: 2,
            rows: 1,
            explicit_width_cell_count: 0,
            explicit_height_cell_count: 0,
            cref_count: 1,
            crefs: vec![CrefEntry {
                image_index: 0,
                rectangle_index: 0,
            }],
            node_index: 0,
            cells: vec![cell(0), cell(-1)],
        };
        let layer = Layer {
            name: b"slice".to_vec(),
            flags: 0x100,
            animation_count: 0,
            animations: Vec::new(),
            field_23: Vec::new(),
            nodes: vec![NodeRecord {
                type_flags: Some(2),
                ..node()
            }],
            transforms: vec![RawTransform::Trs2(SpatialTransform::default())],
            image_by_node: vec![None],
            number_by_node: vec![None],
            reference_by_node: vec![None],
            csli_by_node: vec![Some(csli)],
            cast_attribute_lists: Vec::new(),
            cast_attribute_list_by_node: vec![None],
        };
        let project = Project {
            name: b"project".to_vec(),
            declared_scene_count: 1,
            declared_font_count: 0,
            camera: CameraDefinition::default(),
            scenes: vec![Scene {
                name: b"scene".to_vec(),
                declared_layer_count: 1,
                declared_animation_set_count: 0,
                width: 20.0,
                height: 10.0,
                layers: vec![layer],
                animation_sets: Vec::new(),
            }],
            fonts: Vec::new(),
        };
        let textures = TextureList {
            declared_count: 1,
            textures: vec![TextureDefinition {
                filename: b"slice".to_vec(),
                width: 32,
                height: 32,
                field_62: 0,
                crop_count: 1,
                crops: vec![TextureCrop {
                    normalized_rectangle: [0.25, 0.5, 0.75, 1.0],
                }],
            }],
        };
        let draws = build_evidence_complete_initial_runtime_cast_draws(
            &project,
            &textures,
            0,
            SrdHostDrawContext::new(
                Affine3x4::IDENTITY,
                crate::render::SRD_RENDERER_INITIAL_LAYER_KEY,
                Some(SrdRendererProjectTargetContext::new(
                    identity_matrix4x4_game(),
                    [20, 10],
                )),
                identity_matrix4x4_game(),
                [20, 10],
            ),
            &FennelFontSlotRegistry::default(),
            &BTreeMap::new(),
            false,
            &BTreeMap::new(),
        )
        .unwrap();

        assert_eq!(draws.len(), 2);
        let cells = draws
            .iter()
            .map(|draw| match draw {
                EvidenceCompleteRuntimeCastDraw::SliceCell(cell) => cell,
                _ => panic!("SliceCast emitted a non-slice runtime draw"),
            })
            .collect::<Vec<_>>();
        assert_eq!([cells[0].cell_index, cells[1].cell_index], [0, 1]);
        assert_eq!(
            cells[0].draw.quad.vertices.map(|vertex| vertex.position),
            [
                [0.0, 0.0, 0.0],
                [0.0, 10.0, 0.0],
                [10.0, 0.0, 0.0],
                [10.0, 10.0, 0.0],
            ]
        );
        assert_eq!(
            cells[0]
                .draw
                .quad
                .vertices
                .map(|vertex| vertex.texture_coordinates),
            [
                [[0.25, 0.5]; 2],
                [[0.25, 1.0]; 2],
                [[0.75, 0.5]; 2],
                [[0.75, 1.0]; 2],
            ]
        );
        assert!(cells[0].draw.texture_bindings[0].is_some());
        assert_eq!(cells[1].draw.texture_bindings, [None; 3]);
        assert!(
            cells[1]
                .draw
                .quad
                .vertices
                .iter()
                .all(|vertex| vertex.texture_coordinates == [[0.0; 2]; 2])
        );

        let profile = crate::game_host::CHUSAN_MAIN_SCENE
            .scene_pass_profile()
            .unwrap();
        assert_eq!(
            build_evidence_runtime_target_submission(&draws, &profile).unwrap(),
            vec![
                EvidenceRuntimeTargetCommandSource::SliceCell {
                    runtime_draw_index: 0,
                },
                EvidenceRuntimeTargetCommandSource::SliceCell {
                    runtime_draw_index: 1,
                },
            ]
        );
        assert_eq!(
            build_evidence_filtered_merged_runtime_target_submission(
                &draws,
                &profile,
                crate::game_host::CHUSAN_ADVERTISE_LOGO_PLAYER
                    .initial_srd_target_filter(crate::game_host::CHUSAN_MAIN_SCENE),
            )
            .unwrap()
            .len(),
            2
        );
    }

    #[test]
    fn adjacent_merge_adds_strip_connectors_but_not_triangle_list_vertices() {
        let packet = CeylonDrawPacketPresetState::srd_renderer_initial();
        let strip_key = EvidenceAdjacentMergeKey {
            packet,
            renderer_layer_key: crate::render::SRD_RENDERER_INITIAL_LAYER_KEY,
            vertex_format: 14,
            primitive_type: 4,
            textures: [None; 3],
            packet_matrix_prefix: None,
        };
        let list_key = EvidenceAdjacentMergeKey {
            packet,
            renderer_layer_key: crate::render::SRD_RENDERER_INITIAL_LAYER_KEY,
            vertex_format: 13,
            primitive_type: 3,
            textures: [Some(EvidenceMergeTextureIdentity::Fennel(7)), None, None],
            packet_matrix_prefix: None,
        };
        let commands = vec![
            EvidenceAdjacentMergeCommand {
                source: EvidenceRuntimeTargetCommandSource::Image {
                    runtime_draw_index: 0,
                },
                key: strip_key.clone(),
                vertex_count: 4,
            },
            EvidenceAdjacentMergeCommand {
                source: EvidenceRuntimeTargetCommandSource::Image {
                    runtime_draw_index: 1,
                },
                key: strip_key,
                vertex_count: 4,
            },
            EvidenceAdjacentMergeCommand {
                source: EvidenceRuntimeTargetCommandSource::FennelBatch {
                    runtime_draw_index: 2,
                    batch_index: 0,
                },
                key: list_key.clone(),
                vertex_count: 6,
            },
            EvidenceAdjacentMergeCommand {
                source: EvidenceRuntimeTargetCommandSource::FennelBatch {
                    runtime_draw_index: 3,
                    batch_index: 0,
                },
                key: list_key,
                vertex_count: 12,
            },
        ];

        let groups = merge_evidence_adjacent_commands(commands);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].vertex_count, 10);
        assert_eq!(groups[0].sources.len(), 2);
        assert_eq!(groups[1].vertex_count, 18);
        assert_eq!(groups[1].sources.len(), 2);
    }

    #[test]
    fn materialized_strip_duplicates_previous_last_then_next_first() {
        let vertex = |x| SrdRenderVertex {
            position: [x, 0.0, 0.0],
            primary_color: [0; 4],
            secondary_color: [0; 4],
            texture_coordinates: [[0.0; 2]; 2],
        };
        let first = [vertex(0.0), vertex(1.0), vertex(2.0), vertex(3.0)];
        let second = [vertex(4.0), vertex(5.0), vertex(6.0), vertex(7.0)];
        let mut vertices = Vec::new();
        append_srd_quad_strip_vertices(&mut vertices, &first);
        append_srd_quad_strip_vertices(&mut vertices, &second);
        assert_eq!(
            vertices
                .iter()
                .map(|vertex| vertex.position[0])
                .collect::<Vec<_>>(),
            [0.0, 1.0, 2.0, 3.0, 3.0, 4.0, 4.0, 5.0, 6.0, 7.0]
        );
    }

    #[test]
    fn adjacent_merge_keeps_different_renderer_layer_keys_separate() {
        let packet = CeylonDrawPacketPresetState::srd_renderer_initial();
        let base_key = EvidenceAdjacentMergeKey {
            packet,
            renderer_layer_key: 0x8580,
            vertex_format: 14,
            primitive_type: 4,
            textures: [None; 3],
            packet_matrix_prefix: None,
        };
        let mut different_key = base_key.clone();
        different_key.renderer_layer_key = 0x8680;
        let groups = merge_evidence_adjacent_commands(vec![
            EvidenceAdjacentMergeCommand {
                source: EvidenceRuntimeTargetCommandSource::Image {
                    runtime_draw_index: 0,
                },
                key: base_key,
                vertex_count: 4,
            },
            EvidenceAdjacentMergeCommand {
                source: EvidenceRuntimeTargetCommandSource::Image {
                    runtime_draw_index: 1,
                },
                key: different_key,
                vertex_count: 4,
            },
        ]);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].renderer_layer_key, 0x8580);
        assert_eq!(groups[1].renderer_layer_key, 0x8680);
    }

    #[test]
    fn three_dimensional_fennel_textbox_matrix_multiplies_then_negates_y_column() {
        let world = Affine3x4 {
            rows: [
                [2.0, 0.0, 0.0, 1.0],
                [0.0, 3.0, 0.0, 2.0],
                [0.0, 0.0, 4.0, 3.0],
            ],
        };
        let camera_bridge = Matrix4x4 {
            rows: [
                [1.0, 2.0, 3.0, 4.0],
                [5.0, 6.0, 7.0, 8.0],
                [9.0, 10.0, 11.0, 12.0],
                [13.0, 14.0, 15.0, 16.0],
            ],
        };
        assert_eq!(
            fennel_textbox_transform(world, true, camera_bridge).rows,
            [
                [2.0, 0.0, 0.0, 1.0],
                [0.0, 3.0, 0.0, 2.0],
                [0.0, 0.0, 4.0, 3.0],
                [0.0, 0.0, 0.0, 1.0],
            ]
        );
        assert_eq!(
            fennel_textbox_transform(world, false, camera_bridge).rows,
            [
                [2.0, -6.0, 12.0, 18.0],
                [10.0, -18.0, 28.0, 46.0],
                [18.0, -30.0, 44.0, 74.0],
                [26.0, -42.0, 60.0, 102.0],
            ]
        );
    }
}
