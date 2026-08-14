use std::fmt;

use crate::animation::{Evaluation, Key20, KeyData, ScalarValue, Track, cvtt_f32_to_i32};
use crate::csli::{CrefEntry, CsliDefinition, multiply_color_game, slice_texture_coordinates};
use crate::text::TextDefinition;
use crate::texture::{TextureList, TextureSamplerState};
use crate::vtbf::{Block, Property, SrdFile};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageError(pub String);

impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ImageError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageReferenceChannel {
    Cref,
    Cre1,
}

impl ImageReferenceChannel {
    fn index(self) -> usize {
        match self {
            Self::Cref => 0,
            Self::Cre1 => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageCoordinateState {
    pub vertex_colors: [[u8; 4]; 4],
    pub reference_index: i16,
    pub explicit_image_index: i16,
    pub uses_explicit_rectangle: bool,
    pub explicit_rectangle: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedImageCoordinates {
    pub image_index: i16,
    pub coordinates: [[f32; 2]; 4],
    pub selected_sampler: Option<TextureSamplerState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SrdTextureBindingSource {
    ExplicitOverride,
    TextureList(usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedImageTextureSlots {
    pub channels: [ResolvedImageCoordinates; 2],
    pub slots: [Option<SrdTextureBindingSource>; 3],
}

impl ResolvedImageTextureSlots {
    pub fn texture_present(self) -> [bool; 3] {
        self.slots.map(|slot| slot.is_some())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageQuad {
    pub positions: [[f32; 3]; 4],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageGeometryState {
    pub size: [f32; 2],
    pub origin: [f32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeImageState {
    pub geometry: ImageGeometryState,
    pub coordinates: [ImageCoordinateState; 2],
    pub field_0c: u32,
    pub field_10: i32,
    pub field_14: u32,
    pub field_18: u8,
    pub render_preset_override: i32,
    pub field_1c: i32,
}

impl RuntimeImageState {
    pub fn coordinate_state(&self, channel: ImageReferenceChannel) -> ImageCoordinateState {
        self.coordinates[channel.index()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageVertexColors {
    pub primary: [u8; 4],
    pub secondary: [u8; 4],
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageDefinition {
    pub flags: u32,
    pub width: f32,
    pub height: f32,
    pub custom_origin: [f32; 2],
    pub origin_mode: u8,
    pub vertex_colors: [[u8; 4]; 4],
    pub cref_index: i16,
    pub cref_count: u16,
    pub crefs: Vec<CrefEntry>,
    pub field_4c: u16,
    pub cre1_index: i16,
    pub cre1_count: u16,
    pub cre1s: Vec<CrefEntry>,
    pub coordinate_offsets: [[f32; 2]; 2],
    pub field_a1: u32,
    pub node_index: i32,
    pub has_text_child: bool,
    pub text: Option<TextDefinition>,
}

impl ImageDefinition {
    pub const INITIAL_COORDINATE_OFFSET_SCALE: f32 = f32::from_bits(0x4cbe_bc20);

    pub fn from_block(file: &SrdFile, block: &Block) -> Result<Self, ImageError> {
        if !block.is_tag(b"CIMG") {
            return Err(ImageError("block is not CIMG".into()));
        }

        let mut result = Self {
            flags: 0,
            width: 128.0,
            height: 128.0,
            custom_origin: [0.0, 0.0],
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
            node_index: -1,
            has_text_child: false,
            text: None,
        };
        let mut color_index = 0usize;
        for property in &block.properties {
            match property.code {
                0x40 => result.width = float_scalar(file, property, "CIMG 0x40")?,
                0x41 => result.height = float_scalar(file, property, "CIMG 0x41")?,
                0x42 => result.custom_origin[0] = float_scalar(file, property, "CIMG 0x42")?,
                0x43 => result.custom_origin[1] = float_scalar(file, property, "CIMG 0x43")?,
                0x44 => {
                    let destination =
                        result.vertex_colors.get_mut(color_index).ok_or_else(|| {
                            ImageError("CIMG has more than four 0x44 properties".into())
                        })?;
                    *destination = reordered_four_bytes(file, property, "CIMG 0x44")?;
                    color_index += 1;
                }
                0x45 => result.cref_count = unsigned_scalar(file, property, "CIMG 0x45")? as u16,
                0x46 => result.cref_index = signed_scalar(file, property, "CIMG 0x46")? as i16,
                0x49 => result.flags = unsigned_scalar(file, property, "CIMG 0x49")?,
                0x4b => result.origin_mode = unsigned_scalar(file, property, "CIMG 0x4b")? as u8,
                0x4c => result.field_4c = unsigned_scalar(file, property, "CIMG 0x4c")? as u16,
                0x4d => result.cre1_count = unsigned_scalar(file, property, "CIMG 0x4d")? as u16,
                0x4e => result.cre1_index = signed_scalar(file, property, "CIMG 0x4e")? as i16,
                0x51 => result.node_index = unsigned_scalar(file, property, "CIMG 0x51")? as i32,
                0x83 => {
                    result.coordinate_offsets[0][0] = float_scalar(file, property, "CIMG 0x83")?
                }
                0x84 => {
                    result.coordinate_offsets[1][0] = float_scalar(file, property, "CIMG 0x84")?
                }
                0x85 => {
                    result.coordinate_offsets[0][1] = float_scalar(file, property, "CIMG 0x85")?
                }
                0x86 => {
                    result.coordinate_offsets[1][1] = float_scalar(file, property, "CIMG 0x86")?
                }
                0xa1 => result.field_a1 = unsigned_scalar(file, property, "CIMG 0xa1")?,
                _ => {}
            }
        }

        for child in &block.children {
            if child.is_tag(b"CREF") && result.cref_count != 0 {
                result.crefs = parse_reference_table(file, child, result.cref_count, "CREF")?;
            } else if child.is_tag(b"CRE1") && result.cre1_count != 0 {
                result.cre1s = parse_reference_table(file, child, result.cre1_count, "CRE1")?;
            } else if child.is_tag(b"TEXT") {
                result.has_text_child = true;
                result.text = Some(
                    TextDefinition::from_block(file, child)
                        .map_err(|error| ImageError(error.to_string()))?,
                );
            }
        }

        Ok(result)
    }

    pub fn runtime_origin_offset(&self) -> [f32; 2] {
        self.origin_for_size([self.width, self.height])
    }

    pub fn point_sampled(&self) -> bool {
        self.flags & 0x0100_0000 != 0
    }

    pub fn creates_text_cast(&self) -> bool {
        self.has_text_child && self.flags & 0x100 != 0
    }

    pub fn initial_coordinate_state(&self, channel: ImageReferenceChannel) -> ImageCoordinateState {
        ImageCoordinateState {
            vertex_colors: self.vertex_colors,
            reference_index: match channel {
                ImageReferenceChannel::Cref => self.cref_index,
                ImageReferenceChannel::Cre1 => self.cre1_index,
            },
            explicit_image_index: 0,
            uses_explicit_rectangle: false,
            explicit_rectangle: [0.0; 4],
        }
    }

    pub fn initial_runtime_state(&self) -> RuntimeImageState {
        RuntimeImageState {
            geometry: self.initial_geometry_state(),
            coordinates: [
                self.initial_coordinate_state(ImageReferenceChannel::Cref),
                self.initial_coordinate_state(ImageReferenceChannel::Cre1),
            ],
            field_0c: u32::from(self.field_4c),
            field_10: 0,
            field_14: 0,
            field_18: 0,
            render_preset_override: -1,
            field_1c: -1,
        }
    }

    /// Reproduces the state left by `srd_srimage_construct` before a cast-specific
    /// initializer copies CIMG, CNUM, or CSLI data into the embedded SrImage.
    pub fn srimage_constructor_base() -> Self {
        Self {
            flags: 0,
            width: 0.0,
            height: 0.0,
            custom_origin: [0.0; 2],
            origin_mode: 4,
            vertex_colors: [[0; 4]; 4],
            cref_index: 0,
            cref_count: 0,
            crefs: Vec::new(),
            field_4c: 0,
            cre1_index: 0,
            cre1_count: 0,
            cre1s: Vec::new(),
            coordinate_offsets: [[0.0; 2]; 2],
            field_a1: 0,
            node_index: -1,
            has_text_child: false,
            text: None,
        }
    }

    /// Reproduces `srd_init_srimage_from_csli`. CSLI initializes only channel 0's
    /// reference table; both selectors retain the zero written by the constructor.
    pub fn from_csli_runtime_base(definition: &CsliDefinition) -> Self {
        Self {
            flags: definition.field_80,
            width: definition.width,
            height: definition.height,
            custom_origin: definition.custom_origin,
            origin_mode: definition.origin_mode,
            vertex_colors: definition.field_44,
            cref_index: 0,
            cref_count: definition.cref_count,
            crefs: definition.crefs.clone(),
            field_4c: 0,
            cre1_index: 0,
            cre1_count: 0,
            cre1s: Vec::new(),
            coordinate_offsets: [[0.0; 2]; 2],
            field_a1: 0,
            node_index: definition.node_index,
            has_text_child: false,
            text: None,
        }
    }

    pub fn build_quad(&self, axis_mode: bool) -> ImageQuad {
        self.build_quad_with_geometry(self.initial_geometry_state(), axis_mode)
    }

    pub fn initial_geometry_state(&self) -> ImageGeometryState {
        ImageGeometryState {
            size: [self.width, self.height],
            origin: self.runtime_origin_offset(),
        }
    }

    pub fn apply_size_track(
        &self,
        state: &mut ImageGeometryState,
        track: &Track,
        frame: f32,
    ) -> bool {
        let component = match track.target {
            11 => 0,
            12 => 1,
            _ => return false,
        };
        let bits = match track.evaluate(frame) {
            Evaluation::Value(ScalarValue::F32(value)) => value.to_bits(),
            Evaluation::Value(ScalarValue::I32(value)) => value as u32,
            Evaluation::Value(ScalarValue::Bytes4(value)) => u32::from_le_bytes(value),
            Evaluation::Unchanged | Evaluation::Unsupported => return false,
        };
        state.size[component] = f32::from_bits(bits);
        state.origin = self.origin_for_size(state.size);
        true
    }

    pub fn build_quad_with_geometry(
        &self,
        geometry: ImageGeometryState,
        axis_mode: bool,
    ) -> ImageQuad {
        let left = -geometry.origin[0];
        let right = geometry.size[0] - geometry.origin[0];
        let mut first_y = -geometry.origin[1];
        let mut second_y = geometry.size[1] - geometry.origin[1];
        if !axis_mode {
            first_y = -first_y;
            second_y = -second_y;
        }
        ImageQuad {
            positions: [
                [left, first_y, 0.0],
                [left, second_y, 0.0],
                [right, first_y, 0.0],
                [right, second_y, 0.0],
            ],
        }
    }

    fn origin_for_size(&self, size: [f32; 2]) -> [f32; 2] {
        const FACTORS: [[f32; 2]; 9] = [
            [0.0, 0.0],
            [0.5, 0.0],
            [1.0, 0.0],
            [0.0, 0.5],
            [0.5, 0.5],
            [1.0, 0.5],
            [0.0, 1.0],
            [0.5, 1.0],
            [1.0, 1.0],
        ];
        let Some(factors) = FACTORS.get(usize::from(self.origin_mode)) else {
            return self.custom_origin;
        };
        [factors[0] * size[0], factors[1] * size[1]]
    }

    pub fn vertex_colors(
        &self,
        state: ImageCoordinateState,
        vertex_index: usize,
        multiplicative_tint: [u8; 4],
        additive_tint: [u8; 4],
    ) -> Option<ImageVertexColors> {
        let color = *state.vertex_colors.get(vertex_index)?;
        Some(ImageVertexColors {
            primary: multiply_color_game(color, multiplicative_tint),
            secondary: premultiply_additive_color_game(additive_tint),
        })
    }

    pub fn apply_coordinate_track(
        &self,
        channel: ImageReferenceChannel,
        state: &mut ImageCoordinateState,
        track: &Track,
        frame: f32,
        textures: &TextureList,
    ) -> Result<bool, ImageError> {
        if track.format & 3 != 3 {
            let bits = match track.evaluate(frame) {
                Evaluation::Value(ScalarValue::F32(value)) => value.to_bits(),
                Evaluation::Value(ScalarValue::I32(value)) => value as u32,
                Evaluation::Value(ScalarValue::Bytes4(value)) => u32::from_le_bytes(value),
                Evaluation::Unchanged | Evaluation::Unsupported => return Ok(false),
            };
            let bytes = bits.to_le_bytes();
            state.reference_index = i16::from_le_bytes([bytes[0], bytes[1]]);
            state.explicit_image_index = i16::from_le_bytes([bytes[2], bytes[3]]);
            return Ok(true);
        }

        let references = match channel {
            ImageReferenceChannel::Cref => self.crefs.as_slice(),
            ImageReferenceChannel::Cre1 => self.cre1s.as_slice(),
        };
        let float_keys;
        let keys = match &track.keys {
            KeyData::Key20I32(keys) => keys.as_slice(),
            KeyData::Key20F32(keys) => {
                float_keys = keys
                    .iter()
                    .map(|key| Key20 {
                        frame: key.frame,
                        value: key.value.to_bits() as i32,
                        mode: key.mode,
                        slope_in: key.slope_in,
                        slope_out: key.slope_out,
                    })
                    .collect::<Vec<_>>();
                float_keys.as_slice()
            }
            _ => {
                state.uses_explicit_rectangle = false;
                return Ok(false);
            }
        };
        if references.is_empty() || keys.is_empty() {
            state.uses_explicit_rectangle = false;
            return Ok(false);
        }

        let frame = track.wrapped_frame(frame);
        let mut explicit_image_index = -1i16;
        match reference_key_segment(keys, frame) {
            ReferenceKeySegment::Value(key) => {
                state.reference_index = key.value as i16;
                if let Some(rectangle) = lookup_reference_rectangle(
                    references,
                    key.value,
                    textures,
                    &mut explicit_image_index,
                )? {
                    state.explicit_rectangle = rectangle;
                }
            }
            ReferenceKeySegment::Pair(left, _right) if left.mode == 0 => {
                state.reference_index = left.value as i16;
                if let Some(rectangle) = lookup_reference_rectangle(
                    references,
                    left.value,
                    textures,
                    &mut explicit_image_index,
                )? {
                    state.explicit_rectangle = rectangle;
                }
            }
            ReferenceKeySegment::Pair(left, right) => {
                let t = (frame - left.frame as f32) / (right.frame - left.frame) as f32;
                let inverse = 1.0 - t;
                state.reference_index =
                    cvtt_f32_to_i32(left.value as f32 * inverse + right.value as f32 * t) as i16;
                let left_rectangle = lookup_reference_rectangle(
                    references,
                    left.value,
                    textures,
                    &mut explicit_image_index,
                )?;
                let right_rectangle = lookup_reference_rectangle(
                    references,
                    right.value,
                    textures,
                    &mut explicit_image_index,
                )?;
                if let (Some(left), Some(right)) = (left_rectangle, right_rectangle) {
                    state.explicit_rectangle = [
                        right[0] * t + left[0] * inverse,
                        left[1] * inverse + right[1] * t,
                        left[2] * inverse + right[2] * t,
                        left[3] * inverse + right[3] * t,
                    ];
                }
            }
        }
        state.explicit_image_index = explicit_image_index;
        state.uses_explicit_rectangle = true;
        Ok(true)
    }

    pub fn apply_vertex_color_track(
        &self,
        state: &mut ImageCoordinateState,
        track: &Track,
        frame: f32,
    ) -> bool {
        let vertex_index = match track.target {
            13 => 0,
            14 => 2,
            15 => 1,
            16 => 3,
            _ => return false,
        };
        let bytes = match track.evaluate(frame) {
            Evaluation::Value(ScalarValue::F32(value)) => value.to_bits().to_le_bytes(),
            Evaluation::Value(ScalarValue::I32(value)) => (value as u32).to_le_bytes(),
            Evaluation::Value(ScalarValue::Bytes4(value)) => value,
            Evaluation::Unchanged | Evaluation::Unsupported => return false,
        };
        state.vertex_colors[vertex_index] = [bytes[2], bytes[1], bytes[0], bytes[3]];
        true
    }

    pub fn apply_runtime_track(
        &self,
        state: &mut RuntimeImageState,
        track: &Track,
        frame: f32,
        textures: &TextureList,
    ) -> Result<bool, ImageError> {
        match track.target {
            11 | 12 => Ok(self.apply_size_track(&mut state.geometry, track, frame)),
            13..=16 => Ok(self.apply_vertex_color_track(
                &mut state.coordinates[ImageReferenceChannel::Cref.index()],
                track,
                frame,
            )),
            17 => self.apply_coordinate_track(
                ImageReferenceChannel::Cref,
                &mut state.coordinates[ImageReferenceChannel::Cref.index()],
                track,
                frame,
                textures,
            ),
            20 => self.apply_coordinate_track(
                ImageReferenceChannel::Cre1,
                &mut state.coordinates[ImageReferenceChannel::Cre1.index()],
                track,
                frame,
                textures,
            ),
            _ => Ok(false),
        }
    }

    #[allow(clippy::assign_op_pattern)]
    pub fn resolve_coordinates(
        &self,
        channel: ImageReferenceChannel,
        state: ImageCoordinateState,
        textures: &TextureList,
        offset_scale: f32,
    ) -> Result<ResolvedImageCoordinates, ImageError> {
        let channel_index = channel.index();
        let (declared_count, references) = match channel {
            ImageReferenceChannel::Cref => (self.cref_count, self.crefs.as_slice()),
            ImageReferenceChannel::Cre1 => (self.cre1_count, self.cre1s.as_slice()),
        };
        let selector = state.reference_index;
        let mut image_index = -1i16;
        let mut rectangle = [0.0f32; 4];

        if selector >= 0 && u32::from(selector as u16) < u32::from(declared_count) {
            if state.uses_explicit_rectangle {
                image_index = state.explicit_image_index;
                rectangle = state.explicit_rectangle;
            } else if !references.is_empty() {
                let reference = references.get(selector as usize).ok_or_else(|| {
                    ImageError(format!(
                        "{channel:?} selector {selector} is below declared count {declared_count} but outside its allocated table"
                    ))
                })?;
                image_index = reference.image_index;
                if reference.image_index >= 0 && reference.rectangle_index >= 0 {
                    let texture = textures
                        .textures
                        .get(reference.image_index as usize)
                        .ok_or_else(|| {
                            ImageError(format!(
                                "{channel:?} image index {} is outside {} TEX records",
                                reference.image_index,
                                textures.textures.len()
                            ))
                        })?;
                    rectangle = texture
                        .crops
                        .get(reference.rectangle_index as usize)
                        .ok_or_else(|| {
                            ImageError(format!(
                                "{channel:?} rectangle index {} is outside {} CROP records for TEX {}",
                                reference.rectangle_index,
                                texture.crops.len(),
                                reference.image_index
                            ))
                        })?
                        .normalized_rectangle;
                }
            }
        }

        let mut coordinates = slice_texture_coordinates(rectangle, self.flags);
        let offset = self.coordinate_offsets[channel_index];
        for coordinate in &mut coordinates {
            coordinate[0] = offset[0] * offset_scale + coordinate[0];
            coordinate[1] = offset[1] * offset_scale + coordinate[1];
        }
        let selected_sampler = usize::try_from(image_index)
            .ok()
            .and_then(|index| textures.textures.get(index))
            .map(|texture| texture.sampler_pair().select(self.point_sampled()));
        Ok(ResolvedImageCoordinates {
            image_index,
            coordinates,
            selected_sampler,
        })
    }

    /// Reproduces `srd_select_render_texture_pair` and the embedded draw-packet
    /// layout: CREF selects packet texture slot 0, CRE1 selects slot 1, and SRD
    /// leaves slot 2 null. A non-null explicit override wins over the TEXL index.
    pub fn resolve_texture_slots(
        &self,
        state: &RuntimeImageState,
        textures: &TextureList,
        offset_scale: f32,
        explicit_override_present: [bool; 2],
    ) -> Result<ResolvedImageTextureSlots, ImageError> {
        let channels = [
            self.resolve_coordinates(
                ImageReferenceChannel::Cref,
                state.coordinate_state(ImageReferenceChannel::Cref),
                textures,
                offset_scale,
            )?,
            self.resolve_coordinates(
                ImageReferenceChannel::Cre1,
                state.coordinate_state(ImageReferenceChannel::Cre1),
                textures,
                offset_scale,
            )?,
        ];
        let slots = std::array::from_fn(|slot| {
            if slot >= 2 {
                return None;
            }
            if explicit_override_present[slot] {
                return Some(SrdTextureBindingSource::ExplicitOverride);
            }
            usize::try_from(channels[slot].image_index)
                .ok()
                .filter(|index| *index < textures.textures.len())
                .map(SrdTextureBindingSource::TextureList)
        });
        Ok(ResolvedImageTextureSlots { channels, slots })
    }
}

pub fn premultiply_additive_color_game(color: [u8; 4]) -> [u8; 4] {
    let alpha = u32::from(color[3]);
    [
        (u32::from(color[0]) * alpha / 255) as u8,
        (u32::from(color[1]) * alpha / 255) as u8,
        (u32::from(color[2]) * alpha / 255) as u8,
        0,
    ]
}

enum ReferenceKeySegment<'a> {
    Value(&'a Key20<i32>),
    Pair(&'a Key20<i32>, &'a Key20<i32>),
}

fn reference_key_segment(keys: &[Key20<i32>], frame: f32) -> ReferenceKeySegment<'_> {
    if keys.len() == 1 || frame <= keys[0].frame as f32 {
        return ReferenceKeySegment::Value(&keys[0]);
    }
    if frame >= keys[keys.len() - 1].frame as f32 {
        return ReferenceKeySegment::Value(&keys[keys.len() - 1]);
    }
    let upper = keys.partition_point(|key| key.frame as f32 <= frame);
    ReferenceKeySegment::Pair(&keys[upper - 1], &keys[upper])
}

fn lookup_reference_rectangle(
    references: &[CrefEntry],
    selector: i32,
    textures: &TextureList,
    explicit_image_index: &mut i16,
) -> Result<Option<[f32; 4]>, ImageError> {
    if selector < 0 || selector as usize >= references.len() {
        return Ok(None);
    }
    let reference = references[selector as usize];
    if reference.image_index < 0 || reference.rectangle_index < 0 {
        return Ok(None);
    }
    *explicit_image_index = reference.image_index;
    let texture = textures
        .textures
        .get(reference.image_index as usize)
        .ok_or_else(|| {
            ImageError(format!(
                "animated image index {} is outside {} TEX records",
                reference.image_index,
                textures.textures.len()
            ))
        })?;
    let rectangle = texture
        .crops
        .get(reference.rectangle_index as usize)
        .ok_or_else(|| {
            ImageError(format!(
                "animated rectangle index {} is outside {} CROP records for TEX {}",
                reference.rectangle_index,
                texture.crops.len(),
                reference.image_index
            ))
        })?
        .normalized_rectangle;
    Ok(Some(rectangle))
}

fn parse_reference_table(
    file: &SrdFile,
    block: &Block,
    declared_count: u16,
    label: &str,
) -> Result<Vec<CrefEntry>, ImageError> {
    let mut entries = vec![
        CrefEntry {
            image_index: 0,
            rectangle_index: 0,
        };
        usize::from(declared_count)
    ];
    for (index, property) in block.properties_with_code(0x4a).enumerate() {
        let destination = entries.get_mut(index).ok_or_else(|| {
            ImageError(format!(
                "CIMG declares {declared_count} {label} records but the child has more"
            ))
        })?;
        destination.image_index = property
            .read_signed_scalar_at(file, 0)
            .ok_or_else(|| ImageError(format!("invalid {label} 0x4a image index")))?
            as i16;
        destination.rectangle_index = property
            .read_signed_scalar_at(file, 1)
            .ok_or_else(|| ImageError(format!("invalid {label} 0x4a rectangle index")))?
            as i16;
    }
    Ok(entries)
}

fn unsigned_scalar(file: &SrdFile, property: &Property, label: &str) -> Result<u32, ImageError> {
    property
        .read_unsigned_scalar(file)
        .ok_or_else(|| ImageError(format!("invalid {label}")))
}

fn signed_scalar(file: &SrdFile, property: &Property, label: &str) -> Result<i32, ImageError> {
    property
        .read_signed_scalar(file)
        .ok_or_else(|| ImageError(format!("invalid {label}")))
}

fn float_scalar(file: &SrdFile, property: &Property, label: &str) -> Result<f32, ImageError> {
    property
        .read_scalar_as_f32(file)
        .ok_or_else(|| ImageError(format!("invalid {label}")))
}

fn reordered_four_bytes(
    file: &SrdFile,
    property: &Property,
    label: &str,
) -> Result<[u8; 4], ImageError> {
    let bytes = property.value_bytes(file);
    if bytes.len() < 4 {
        return Err(ImageError(format!("{label} has fewer than four bytes")));
    }
    Ok([bytes[1], bytes[2], bytes[3], bytes[0]])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::texture::{TextureCrop, TextureDefinition};

    fn definition() -> ImageDefinition {
        ImageDefinition {
            flags: 0,
            width: 8.0,
            height: 6.0,
            custom_origin: [1.0, 2.0],
            origin_mode: 9,
            vertex_colors: [[0xff; 4]; 4],
            cref_index: 0,
            cref_count: 1,
            crefs: vec![CrefEntry {
                image_index: 0,
                rectangle_index: 0,
            }],
            field_4c: 0,
            cre1_index: -1,
            cre1_count: 0,
            cre1s: Vec::new(),
            coordinate_offsets: [[0.0; 2]; 2],
            field_a1: 0,
            node_index: 0,
            has_text_child: false,
            text: None,
        }
    }

    fn textures() -> TextureList {
        TextureList {
            declared_count: 1,
            textures: vec![TextureDefinition {
                crops: vec![TextureCrop {
                    normalized_rectangle: [0.1, 0.2, 0.7, 0.9],
                }],
                crop_count: 1,
                ..TextureDefinition::default()
            }],
        }
    }

    fn reference_animation_track(mode: u32) -> Track {
        Track {
            target: 17,
            key_count: 2,
            format: 0x23,
            range_start: 0,
            range_end: 10,
            keys: KeyData::Key20I32(vec![
                Key20 {
                    frame: 0,
                    value: 0,
                    mode,
                    slope_in: 0.0,
                    slope_out: 0.0,
                },
                Key20 {
                    frame: 10,
                    value: 1,
                    mode: 0,
                    slope_in: 0.0,
                    slope_out: 0.0,
                },
            ]),
        }
    }

    #[test]
    fn cref_and_cre1_are_independent_channels() {
        let mut definition = definition();
        definition.cre1_index = 0;
        definition.cre1_count = 1;
        definition.cre1s = vec![CrefEntry {
            image_index: 0,
            rectangle_index: 0,
        }];
        definition.coordinate_offsets = [[0.25, 0.5], [1.0, 2.0]];

        let first = definition
            .resolve_coordinates(
                ImageReferenceChannel::Cref,
                definition.initial_coordinate_state(ImageReferenceChannel::Cref),
                &textures(),
                2.0,
            )
            .unwrap();
        let second = definition
            .resolve_coordinates(
                ImageReferenceChannel::Cre1,
                definition.initial_coordinate_state(ImageReferenceChannel::Cre1),
                &textures(),
                2.0,
            )
            .unwrap();
        assert_eq!(first.coordinates[0], [0.6, 1.2]);
        assert_eq!(second.coordinates[0], [2.1, 4.2]);
        assert_eq!(first.selected_sampler, second.selected_sampler);
    }

    #[test]
    fn cref_and_cre1_map_to_packet_texture_slots_zero_and_one() {
        let mut definition = definition();
        definition.cre1_index = 0;
        definition.cre1_count = 1;
        definition.cre1s = vec![CrefEntry {
            image_index: 1,
            rectangle_index: 0,
        }];
        let mut textures = textures();
        textures.declared_count = 2;
        textures.textures.push(textures.textures[0].clone());
        let state = definition.initial_runtime_state();

        let resolved = definition
            .resolve_texture_slots(&state, &textures, 1.0, [false, false])
            .unwrap();
        assert_eq!(
            resolved.slots,
            [
                Some(SrdTextureBindingSource::TextureList(0)),
                Some(SrdTextureBindingSource::TextureList(1)),
                None,
            ]
        );
        assert_eq!(resolved.texture_present(), [true, true, false]);

        let overridden = definition
            .resolve_texture_slots(&state, &textures, 1.0, [false, true])
            .unwrap();
        assert_eq!(
            overridden.slots[1],
            Some(SrdTextureBindingSource::ExplicitOverride)
        );

        let mut out_of_range_state = state;
        out_of_range_state.coordinates[ImageReferenceChannel::Cre1.index()]
            .uses_explicit_rectangle = true;
        out_of_range_state.coordinates[ImageReferenceChannel::Cre1.index()].explicit_image_index =
            2;
        let out_of_range = definition
            .resolve_texture_slots(&out_of_range_state, &textures, 1.0, [false, false])
            .unwrap();
        assert_eq!(
            out_of_range.slots,
            [Some(SrdTextureBindingSource::TextureList(0)), None, None]
        );
    }

    #[test]
    fn explicit_rectangle_still_requires_selector_inside_declared_count() {
        let definition = definition();
        let state = ImageCoordinateState {
            reference_index: -1,
            explicit_image_index: 7,
            uses_explicit_rectangle: true,
            explicit_rectangle: [0.2, 0.3, 0.4, 0.5],
            ..definition.initial_coordinate_state(ImageReferenceChannel::Cref)
        };
        let resolved = definition
            .resolve_coordinates(ImageReferenceChannel::Cref, state, &textures(), 1.0)
            .unwrap();
        assert_eq!(resolved.image_index, -1);
        assert_eq!(resolved.coordinates, [[0.0; 2]; 4]);
        assert_eq!(resolved.selected_sampler, None);
    }

    #[test]
    fn image_reference_tracks_interpolate_explicit_rectangles_like_the_game() {
        let mut definition = definition();
        definition.cref_count = 2;
        definition.crefs = vec![
            CrefEntry {
                image_index: 0,
                rectangle_index: 0,
            },
            CrefEntry {
                image_index: 1,
                rectangle_index: 0,
            },
        ];
        let textures = TextureList {
            declared_count: 2,
            textures: vec![
                TextureDefinition {
                    crops: vec![TextureCrop {
                        normalized_rectangle: [0.0, 0.2, 0.4, 0.6],
                    }],
                    crop_count: 1,
                    ..TextureDefinition::default()
                },
                TextureDefinition {
                    crops: vec![TextureCrop {
                        normalized_rectangle: [0.2, 0.4, 0.8, 1.0],
                    }],
                    crop_count: 1,
                    ..TextureDefinition::default()
                },
            ],
        };
        let mut state = definition.initial_coordinate_state(ImageReferenceChannel::Cref);
        assert!(
            definition
                .apply_coordinate_track(
                    ImageReferenceChannel::Cref,
                    &mut state,
                    &reference_animation_track(1),
                    5.0,
                    &textures,
                )
                .unwrap()
        );
        assert_eq!(state.reference_index, 0);
        assert_eq!(state.explicit_image_index, 1);
        assert!(state.uses_explicit_rectangle);
        assert_eq!(state.explicit_rectangle, [0.1, 0.3, 0.6, 0.8]);
        let resolved = definition
            .resolve_coordinates(
                ImageReferenceChannel::Cref,
                state,
                &textures,
                ImageDefinition::INITIAL_COORDINATE_OFFSET_SCALE,
            )
            .unwrap();
        assert_eq!(resolved.image_index, 1);
        assert_eq!(resolved.coordinates[0], [0.1, 0.3]);

        let mut held = definition.initial_coordinate_state(ImageReferenceChannel::Cref);
        definition
            .apply_coordinate_track(
                ImageReferenceChannel::Cref,
                &mut held,
                &reference_animation_track(0),
                5.0,
                &textures,
            )
            .unwrap();
        assert_eq!(held.reference_index, 0);
        assert_eq!(held.explicit_image_index, 0);
        assert_eq!(held.explicit_rectangle, [0.0, 0.2, 0.4, 0.6]);
    }

    #[test]
    fn image_quad_preserves_both_axis_orders() {
        let definition = definition();
        assert_eq!(
            definition.build_quad(true).positions,
            [
                [-1.0, -2.0, 0.0],
                [-1.0, 4.0, 0.0],
                [7.0, -2.0, 0.0],
                [7.0, 4.0, 0.0],
            ]
        );
        assert_eq!(
            definition.build_quad(false).positions,
            [
                [-1.0, 2.0, 0.0],
                [-1.0, -4.0, 0.0],
                [7.0, 2.0, 0.0],
                [7.0, -4.0, 0.0],
            ]
        );

        let mut centered = definition.clone();
        centered.origin_mode = 4;
        let mut geometry = centered.initial_geometry_state();
        let width_track = Track {
            target: 11,
            key_count: 1,
            format: 0x13,
            range_start: 0,
            range_end: 0,
            keys: KeyData::Key20F32(vec![Key20 {
                frame: 0,
                value: 20.0,
                mode: 0,
                slope_in: 0.0,
                slope_out: 0.0,
            }]),
        };
        assert!(centered.apply_size_track(&mut geometry, &width_track, 0.0));
        assert_eq!(geometry.size, [20.0, 6.0]);
        assert_eq!(geometry.origin, [10.0, 3.0]);
        assert_eq!(
            centered.build_quad_with_geometry(geometry, true).positions,
            [
                [-10.0, -3.0, 0.0],
                [-10.0, 3.0, 0.0],
                [10.0, -3.0, 0.0],
                [10.0, 3.0, 0.0],
            ]
        );
    }

    #[test]
    fn runtime_srimage_state_preserves_constructor_and_csli_initializers() {
        let constructor = ImageDefinition::srimage_constructor_base();
        let constructor_state = constructor.initial_runtime_state();
        assert_eq!(constructor.origin_mode, 4);
        assert_eq!(constructor_state.geometry.size, [0.0, 0.0]);
        assert_eq!(constructor_state.geometry.origin, [0.0, 0.0]);
        assert_eq!(constructor_state.coordinates[0].vertex_colors, [[0; 4]; 4]);
        assert_eq!(constructor_state.coordinates[0].reference_index, 0);
        assert_eq!(constructor_state.coordinates[1].reference_index, 0);
        assert_eq!(constructor_state.field_10, 0);
        assert_eq!(constructor_state.field_14, 0);
        assert_eq!(constructor_state.field_18, 0);
        assert_eq!(constructor_state.render_preset_override, -1);
        assert_eq!(constructor_state.field_1c, -1);

        let csli = CsliDefinition {
            field_80: 0x0100_0000,
            width: 40.0,
            height: 20.0,
            custom_origin: [7.0, 9.0],
            field_44: [[1, 2, 3, 4]; 4],
            origin_mode: 8,
            columns: 0,
            rows: 0,
            explicit_width_cell_count: 0,
            explicit_height_cell_count: 0,
            cref_count: 1,
            crefs: vec![CrefEntry {
                image_index: 2,
                rectangle_index: 3,
            }],
            node_index: 6,
            cells: Vec::new(),
        };
        let base = ImageDefinition::from_csli_runtime_base(&csli);
        let state = base.initial_runtime_state();
        assert!(base.point_sampled());
        assert_eq!(state.geometry.size, [40.0, 20.0]);
        assert_eq!(state.geometry.origin, [40.0, 20.0]);
        assert_eq!(state.coordinates[0].vertex_colors, csli.field_44);
        assert_eq!(state.coordinates[0].reference_index, 0);
        assert_eq!(state.coordinates[1].reference_index, 0);
        assert_eq!(base.crefs, csli.crefs);
        assert!(base.cre1s.is_empty());
    }

    #[test]
    fn runtime_dispatch_updates_exact_srimage_subobjects() {
        let definition = definition();
        let mut state = definition.initial_runtime_state();
        let width = Track {
            target: 11,
            key_count: 1,
            format: 0x13,
            range_start: 0,
            range_end: 0,
            keys: KeyData::Key20F32(vec![Key20 {
                frame: 0,
                value: 12.0,
                mode: 0,
                slope_in: 0.0,
                slope_out: 0.0,
            }]),
        };
        assert!(
            definition
                .apply_runtime_track(&mut state, &width, 0.0, &textures())
                .unwrap()
        );
        assert_eq!(state.geometry.size, [12.0, 6.0]);
        assert_eq!(state.geometry.origin, [1.0, 2.0]);

        let color = Track {
            target: 14,
            key_count: 1,
            format: 0x51,
            range_start: 0,
            range_end: 0,
            keys: KeyData::Key8Bytes4(vec![crate::animation::Key8 {
                frame: 0,
                value: [10, 20, 30, 40],
            }]),
        };
        assert!(
            definition
                .apply_runtime_track(&mut state, &color, 0.0, &textures())
                .unwrap()
        );
        assert_eq!(state.coordinates[0].vertex_colors[2], [30, 20, 10, 40]);
        assert_eq!(state.coordinates[1].vertex_colors[2], [0xff; 4]);
    }

    #[test]
    fn image_colors_multiply_primary_and_premultiply_additive_secondary() {
        let mut definition = definition();
        definition.vertex_colors[2] = [255, 128, 64, 32];
        let colors = definition
            .vertex_colors(
                definition.initial_coordinate_state(ImageReferenceChannel::Cref),
                2,
                [128, 255, 32, 255],
                [64, 128, 255, 128],
            )
            .unwrap();
        assert_eq!(colors.primary, [128, 128, 8, 32]);
        assert_eq!(colors.secondary, [32, 64, 128, 0]);
        assert!(
            definition
                .vertex_colors(
                    definition.initial_coordinate_state(ImageReferenceChannel::Cref),
                    4,
                    [0; 4],
                    [0; 4],
                )
                .is_none()
        );

        let track = Track {
            target: 14,
            key_count: 1,
            format: 0x51,
            range_start: 0,
            range_end: 0,
            keys: KeyData::Key8Bytes4(vec![crate::animation::Key8 {
                frame: 0,
                value: [1, 2, 3, 4],
            }]),
        };
        let mut state = definition.initial_coordinate_state(ImageReferenceChannel::Cref);
        assert!(definition.apply_vertex_color_track(&mut state, &track, 0.0));
        assert_eq!(state.vertex_colors[2], [3, 2, 1, 4]);
    }

    #[test]
    fn image_quad_and_color_inputs_are_backend_neutral() {
        let definition = definition();
        let quad = definition.build_quad_with_geometry(definition.initial_geometry_state(), true);
        assert_eq!(quad.positions[0], [-1.0, -2.0, 0.0]);
        assert_eq!(quad.positions[1], [-1.0, 4.0, 0.0]);
        assert_eq!(quad.positions[2], [7.0, -2.0, 0.0]);
        assert_eq!(quad.positions[3], [7.0, 4.0, 0.0]);

        let colors = definition
            .vertex_colors(
                definition.initial_coordinate_state(ImageReferenceChannel::Cref),
                0,
                [255; 4],
                [20, 40, 60, 128],
            )
            .unwrap();
        assert_eq!(colors.primary, [255; 4]);
        assert_eq!(colors.secondary, [10, 20, 30, 0]);
    }
}
