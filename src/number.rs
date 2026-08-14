use std::fmt;

use crate::csli::CrefEntry;
use crate::image::{
    ImageCoordinateState, ImageDefinition, ImageReferenceChannel, ResolvedImageCoordinates,
};
use crate::vtbf::{Block, Property, SrdFile};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberError(pub String);

impl fmt::Display for NumberError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for NumberError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NumberSpecialGlyphs {
    pub plus: i16,
    pub minus: i16,
    pub comma: i16,
    pub decimal_point: i16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumberGlyphQuad {
    pub positions: [[f32; 3]; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberGlyphSegment {
    Sign,
    Integer,
    DecimalPoint,
    Fraction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberFormattedText {
    pub sign: Vec<u8>,
    pub integer: Vec<u8>,
    pub decimal_point: Vec<u8>,
    pub fraction: Vec<u8>,
    pub combined: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumberGlyphRecord {
    pub glyph_index: i16,
    pub is_digit: bool,
    pub segment: NumberGlyphSegment,
    pub quad: NumberGlyphQuad,
}

impl NumberGlyphRecord {
    pub fn drawable(self) -> bool {
        self.glyph_index >= 0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NumberGlyphHistoryRecord {
    pub sign: Vec<NumberGlyphRecord>,
    pub integer: Vec<NumberGlyphRecord>,
    pub decimal_point: Vec<NumberGlyphRecord>,
    pub fraction: Vec<NumberGlyphRecord>,
    pub total_value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NumberGlyphFirstUvCrop {
    None,
    MoveTopTowardBottom { amount: f32 },
    MoveBottomTowardTop { amount: f32 },
}

impl NumberGlyphFirstUvCrop {
    pub fn apply(self, coordinates: &mut ResolvedImageCoordinates) {
        let span = coordinates.coordinates[1][1] - coordinates.coordinates[0][1];
        match self {
            Self::None => {}
            Self::MoveTopTowardBottom { amount } => {
                let offset = span * amount;
                coordinates.coordinates[0][1] += offset;
                coordinates.coordinates[2][1] += offset;
            }
            Self::MoveBottomTowardTop { amount } => {
                let offset = span * amount;
                coordinates.coordinates[1][1] -= offset;
                coordinates.coordinates[3][1] -= offset;
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumberGlyphMode1Draw {
    pub record: NumberGlyphRecord,
    pub first_uv_crop: NumberGlyphFirstUvCrop,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NumberGlyphMode1RenderPlan {
    pub increasing: bool,
    pub draws: Vec<NumberGlyphMode1Draw>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NumberDefinition {
    pub flags: u32,
    pub width: f32,
    pub height: f32,
    pub custom_origin: [f32; 2],
    pub origin_mode: u8,
    pub vertex_colors: [[u8; 4]; 4],
    pub cref_count: u16,
    pub crefs: Vec<CrefEntry>,
    pub field_4c: u16,
    pub cre1_count: u16,
    pub cre1s: Vec<CrefEntry>,
    pub format_flags: u32,
    pub field_78: u32,
    pub initial_integer: i32,
    pub initial_fraction: f32,
    pub fields_83_8b: [i16; 9],
    pub field_8c: [f32; 2],
    pub fields_8d_94: [i16; 8],
    pub node_index: i32,
}

impl NumberDefinition {
    pub const INITIAL_COORDINATE_OFFSET_SCALE: f32 = 0.0;

    pub fn from_block(file: &SrdFile, block: &Block) -> Result<Self, NumberError> {
        if !block.is_tag(b"CNUM") {
            return Err(NumberError("block is not CNUM".into()));
        }

        let mut result = Self {
            flags: 0,
            width: 128.0,
            height: 128.0,
            custom_origin: [0.0, 0.0],
            origin_mode: 0,
            vertex_colors: [[0xff; 4]; 4],
            cref_count: 0,
            crefs: Vec::new(),
            field_4c: 0,
            cre1_count: 0,
            cre1s: Vec::new(),
            format_flags: 0,
            field_78: 0,
            initial_integer: 0,
            initial_fraction: 0.0,
            fields_83_8b: [0; 9],
            field_8c: [0.0; 2],
            fields_8d_94: [0; 8],
            node_index: -1,
        };
        let mut color_index = 0usize;
        for property in &block.properties {
            match property.code {
                0x40 => result.width = float_scalar(file, property, "CNUM 0x40")?,
                0x41 => result.height = float_scalar(file, property, "CNUM 0x41")?,
                0x42 => result.custom_origin[0] = float_scalar(file, property, "CNUM 0x42")?,
                0x43 => result.custom_origin[1] = float_scalar(file, property, "CNUM 0x43")?,
                0x44 => {
                    let destination =
                        result.vertex_colors.get_mut(color_index).ok_or_else(|| {
                            NumberError("CNUM has more than four 0x44 properties".into())
                        })?;
                    *destination = reordered_four_bytes(file, property, "CNUM 0x44")?;
                    color_index += 1;
                }
                0x45 => result.cref_count = unsigned_scalar(file, property, "CNUM 0x45")? as u16,
                0x49 => result.flags = unsigned_scalar(file, property, "CNUM 0x49")?,
                0x4b => result.origin_mode = unsigned_scalar(file, property, "CNUM 0x4b")? as u8,
                0x4c => result.field_4c = unsigned_scalar(file, property, "CNUM 0x4c")? as u16,
                0x4d => result.cre1_count = unsigned_scalar(file, property, "CNUM 0x4d")? as u16,
                0x51 => result.node_index = unsigned_scalar(file, property, "CNUM 0x51")? as i32,
                0x78 => result.field_78 = unsigned_scalar(file, property, "CNUM 0x78")?,
                0x80 => result.format_flags = unsigned_scalar(file, property, "CNUM 0x80")?,
                0x81 => result.initial_integer = signed_scalar(file, property, "CNUM 0x81")?,
                0x82 => result.initial_fraction = float_scalar(file, property, "CNUM 0x82")?,
                0x83..=0x8b => {
                    result.fields_83_8b[usize::from(property.code - 0x83)] =
                        signed_scalar(file, property, "CNUM 0x83..0x8b")? as i16
                }
                0x8c => {
                    for (index, value) in result.field_8c.iter_mut().enumerate() {
                        *value = property.read_scalar_as_f32_at(file, index).ok_or_else(|| {
                            NumberError(format!("invalid CNUM 0x8c value {index}"))
                        })?;
                    }
                }
                0x8d..=0x94 => {
                    result.fields_8d_94[usize::from(property.code - 0x8d)] =
                        signed_scalar(file, property, "CNUM 0x8d..0x94")? as i16
                }
                _ => {}
            }
        }

        for child in &block.children {
            if child.is_tag(b"CREF") && result.cref_count != 0 {
                result.crefs = parse_reference_table(file, child, result.cref_count, "CREF")?;
            } else if child.is_tag(b"CRE1") && result.cre1_count != 0 {
                result.cre1s = parse_reference_table(file, child, result.cre1_count, "CRE1")?;
            }
        }
        Ok(result)
    }

    pub fn initial_value(&self) -> f64 {
        f64::from(self.initial_integer) + f64::from(self.initial_fraction)
    }

    pub fn digit_advance(&self) -> i16 {
        self.fields_83_8b[0]
    }

    pub fn digit_height(&self) -> i16 {
        self.fields_83_8b[1]
    }

    pub fn punctuation_advance(&self) -> i16 {
        self.fields_83_8b[2]
    }

    pub fn punctuation_height(&self) -> i16 {
        self.fields_83_8b[3]
    }

    pub fn comma_vertical_offset(&self) -> i16 {
        self.fields_83_8b[4]
    }

    pub fn grouping_interval(&self) -> i16 {
        self.fields_83_8b[5]
    }

    pub fn integer_digit_count(&self) -> i16 {
        self.fields_83_8b[6]
    }

    pub fn digit_spacing(&self) -> i16 {
        self.fields_83_8b[7]
    }

    pub fn fraction_digit_count(&self) -> i16 {
        self.fields_83_8b[8]
    }

    pub fn fraction_scale(&self) -> [f32; 2] {
        self.field_8c
    }

    pub fn fraction_spacing(&self) -> i16 {
        self.fields_8d_94[0]
    }

    pub fn fraction_vertical_offset(&self) -> i16 {
        self.fields_8d_94[1]
    }

    pub fn decimal_point_vertical_offset(&self) -> i16 {
        self.fields_8d_94[2]
    }

    pub fn value_animation_mode(&self) -> i16 {
        self.fields_8d_94[3]
    }

    pub fn special_glyphs(&self) -> NumberSpecialGlyphs {
        NumberSpecialGlyphs {
            plus: self.fields_8d_94[4],
            minus: self.fields_8d_94[5],
            comma: self.fields_8d_94[6],
            decimal_point: self.fields_8d_94[7],
        }
    }

    pub fn glyph_index_for_ascii(&self, byte: u8) -> Option<i16> {
        match byte {
            b'0'..=b'9' => Some(i16::from(byte - b'0')),
            b'+' => Some(self.special_glyphs().plus),
            b'-' => Some(self.special_glyphs().minus),
            b',' => Some(self.special_glyphs().comma),
            b'.' => Some(self.special_glyphs().decimal_point),
            _ => None,
        }
    }

    pub fn format_value_parts(&self, integer: i32, fraction: f64) -> NumberFormattedText {
        let total = f64::from(integer) + fraction;
        let sign = if total < 0.0 {
            vec![b'-']
        } else if self.format_flags & 0x01 != 0 {
            vec![b'+']
        } else {
            Vec::new()
        };
        let integer = self.format_integer_digits(integer);
        let fraction = self.format_fraction_digits(fraction);
        let decimal_point = if fraction.is_empty() {
            Vec::new()
        } else {
            vec![b'.']
        };
        let mut combined =
            Vec::with_capacity(sign.len() + integer.len() + decimal_point.len() + fraction.len());
        combined.extend_from_slice(&sign);
        combined.extend_from_slice(&integer);
        combined.extend_from_slice(&decimal_point);
        combined.extend_from_slice(&fraction);
        NumberFormattedText {
            sign,
            integer,
            decimal_point,
            fraction,
            combined,
        }
    }

    pub fn initial_formatted_text(&self) -> NumberFormattedText {
        self.format_value_parts(self.initial_integer, f64::from(self.initial_fraction))
    }

    pub fn build_glyph_records(
        &self,
        formatted: &NumberFormattedText,
        axis_mode: bool,
    ) -> Vec<NumberGlyphRecord> {
        let mut records = Vec::new();
        for (segment, text) in [
            (NumberGlyphSegment::Sign, formatted.sign.as_slice()),
            (NumberGlyphSegment::Integer, formatted.integer.as_slice()),
            (
                NumberGlyphSegment::DecimalPoint,
                formatted.decimal_point.as_slice(),
            ),
            (NumberGlyphSegment::Fraction, formatted.fraction.as_slice()),
        ] {
            for byte in text.iter().copied() {
                let Some(glyph_index) = self.glyph_index_for_ascii(byte) else {
                    continue;
                };
                if i32::from(glyph_index) >= i32::from(self.cref_count) {
                    continue;
                }
                records.push(NumberGlyphRecord {
                    glyph_index,
                    is_digit: !matches!(byte, b'+' | b'-' | b',' | b'.'),
                    segment,
                    quad: NumberGlyphQuad {
                        positions: [[0.0; 3]; 4],
                    },
                });
            }
        }

        let positions = self.build_glyph_positions(&formatted.combined, axis_mode);
        for (record, quad) in records.iter_mut().zip(positions) {
            record.quad = quad;
        }
        records
    }

    /// Reproduces the `history_count <= 1` branch of
    /// `srd_render_number_glyph_history`. A fresh SrNumberCast appends one
    /// current 56-byte history record, then renders sign forward, integer
    /// backward, decimal point forward, and fraction forward with alpha 1.0.
    pub fn first_history_render_records(
        &self,
        formatted: &NumberFormattedText,
        axis_mode: bool,
    ) -> Vec<NumberGlyphRecord> {
        let records = self.build_glyph_records(formatted, axis_mode);
        let mut ordered = Vec::with_capacity(records.len());
        ordered.extend(
            records
                .iter()
                .copied()
                .filter(|record| record.segment == NumberGlyphSegment::Sign),
        );
        ordered.extend(
            records
                .iter()
                .rev()
                .copied()
                .filter(|record| record.segment == NumberGlyphSegment::Integer),
        );
        ordered.extend(
            records
                .iter()
                .copied()
                .filter(|record| record.segment == NumberGlyphSegment::DecimalPoint),
        );
        ordered.extend(
            records
                .iter()
                .copied()
                .filter(|record| record.segment == NumberGlyphSegment::Fraction),
        );
        ordered
    }

    pub fn build_history_record(
        &self,
        integer: i32,
        fraction: f64,
        axis_mode: bool,
    ) -> NumberGlyphHistoryRecord {
        let formatted = self.format_value_parts(integer, fraction);
        let records = self.build_glyph_records(&formatted, axis_mode);
        NumberGlyphHistoryRecord {
            sign: records
                .iter()
                .copied()
                .filter(|record| record.segment == NumberGlyphSegment::Sign)
                .collect(),
            integer: records
                .iter()
                .copied()
                .filter(|record| record.segment == NumberGlyphSegment::Integer)
                .collect(),
            decimal_point: records
                .iter()
                .copied()
                .filter(|record| record.segment == NumberGlyphSegment::DecimalPoint)
                .collect(),
            fraction: records
                .iter()
                .copied()
                .filter(|record| record.segment == NumberGlyphSegment::Fraction)
                .collect(),
            total_value: f64::from(integer) + fraction,
        }
    }

    /// Reproduces the shipped mode-1 branch of
    /// `srd_render_number_glyph_history` and `srd_render_number_glyph` for two
    /// history records. `progress` is `SrNumberCast+0x238`: one selects the
    /// older record and zero selects the newer record.
    pub fn mode1_history_render_plan(
        &self,
        older: &NumberGlyphHistoryRecord,
        newer: &NumberGlyphHistoryRecord,
        progress: f32,
        previous_increasing: bool,
    ) -> NumberGlyphMode1RenderPlan {
        let increasing = if older.total_value != newer.total_value {
            newer.total_value > older.total_value
        } else {
            previous_increasing
        };
        let factor = if progress <= 1.0 {
            progress.max(0.0)
        } else {
            1.0
        };

        let mut older_sign = older.sign.clone();
        while older_sign.len() < newer.sign.len() {
            older_sign.push(number_glyph_sentinel(NumberGlyphSegment::Sign));
        }

        let mut older_integer = older.integer.clone();
        let mut newer_integer = newer.integer.clone();
        while older_integer.len() < newer_integer.len() {
            older_integer.insert(0, number_glyph_sentinel(NumberGlyphSegment::Integer));
        }
        if older_integer.len() > newer_integer.len() {
            let missing = older_integer.len() - newer_integer.len();
            for record in older_integer.iter().take(missing).rev().copied() {
                newer_integer.insert(
                    0,
                    NumberGlyphRecord {
                        glyph_index: -1,
                        ..record
                    },
                );
            }
        }

        let mut older_decimal_point = older.decimal_point.clone();
        while older_decimal_point.len() < newer.decimal_point.len() {
            older_decimal_point.push(number_glyph_sentinel(NumberGlyphSegment::DecimalPoint));
        }
        let mut older_fraction = older.fraction.clone();
        while older_fraction.len() < newer.fraction.len() {
            older_fraction.push(number_glyph_sentinel(NumberGlyphSegment::Fraction));
        }

        let mut draws = Vec::new();
        let mut emitted_pair = false;
        for (older_record, newer_record) in older_sign
            .iter()
            .zip(&newer.sign)
            .chain(older_integer.iter().zip(&newer_integer))
            .chain(older_decimal_point.iter().zip(&newer.decimal_point))
            .chain(older_fraction.iter().zip(&newer.fraction))
        {
            let mut current = *newer_record;
            if current.glyph_index >= 0
                && older_record.glyph_index >= 0
                && !current.is_digit
                && !older_record.is_digit
            {
                for (current_point, older_point) in current
                    .quad
                    .positions
                    .iter_mut()
                    .zip(older_record.quad.positions)
                {
                    for (current_value, older_value) in current_point.iter_mut().zip(older_point) {
                        *current_value = (older_value - *current_value) * factor + *current_value;
                    }
                }
            }

            if !current.is_digit
                || (current.glyph_index == older_record.glyph_index && !emitted_pair)
            {
                if current.drawable() {
                    draws.push(NumberGlyphMode1Draw {
                        record: current,
                        first_uv_crop: NumberGlyphFirstUvCrop::None,
                    });
                }
                continue;
            }

            if current.drawable() {
                draws.push(mode1_cropped_draw(current, increasing, !increasing, factor));
            }
            current.glyph_index = older_record.glyph_index;
            if current.drawable() {
                draws.push(mode1_cropped_draw(current, increasing, increasing, factor));
            }
            emitted_pair = true;
        }

        NumberGlyphMode1RenderPlan { increasing, draws }
    }

    pub fn glyph_coordinate_state(
        &self,
        glyph_index: i16,
        channel: ImageReferenceChannel,
    ) -> ImageCoordinateState {
        let mut state = self.image_base().initial_coordinate_state(channel);
        if channel == ImageReferenceChannel::Cref {
            state.reference_index = glyph_index;
        }
        state
    }

    #[allow(clippy::assign_op_pattern)]
    pub fn measure_string_width(&self, text: &[u8]) -> f32 {
        let mut width = 0.0f32;
        let mut after_decimal_point = false;
        for (index, byte) in text.iter().copied().enumerate() {
            if byte == b',' {
                width = width + f32::from(self.punctuation_advance());
                continue;
            }
            if byte == b'.' {
                after_decimal_point = true;
                width = width + f32::from(self.punctuation_advance());
                continue;
            }

            let mut advance = f32::from(self.digit_advance());
            if after_decimal_point {
                advance = advance * self.fraction_scale()[0];
            }
            width = width + advance;
            if index + 1 < text.len() {
                let spacing = if !after_decimal_point || self.format_flags & 0x20 != 0 {
                    self.digit_spacing()
                } else {
                    self.fraction_spacing()
                };
                width = width + f32::from(spacing);
            }
        }
        width.ceil()
    }

    #[allow(clippy::assign_op_pattern)]
    pub fn build_glyph_positions(&self, text: &[u8], axis_mode: bool) -> Vec<NumberGlyphQuad> {
        let measured_width = self.measure_string_width(text);
        let mut left = -self.custom_origin[0];
        match self.field_78 & 0x0c {
            0x04 => left = (self.width - measured_width) * 0.5 + left,
            0x08 => left = self.width - measured_width + left,
            _ => {}
        }

        let base_top = -self.custom_origin[1];
        let digit_height = f32::from(self.digit_height());
        let mut after_decimal_point = false;
        let mut result = Vec::with_capacity(text.len());
        for (index, byte) in text.iter().copied().enumerate() {
            let (advance, mut top, mut height) = if byte == b',' {
                let height = f32::from(self.punctuation_height());
                (
                    f32::from(self.punctuation_advance()),
                    (digit_height - height) + base_top - f32::from(self.comma_vertical_offset()),
                    height,
                )
            } else if byte == b'.' {
                let height = f32::from(self.punctuation_height());
                (
                    f32::from(self.punctuation_advance()),
                    (digit_height - height) + base_top
                        - f32::from(self.decimal_point_vertical_offset()),
                    height,
                )
            } else if after_decimal_point {
                let scale = self.fraction_scale();
                let height = digit_height * scale[1];
                (
                    f32::from(self.digit_advance()) * scale[0],
                    (digit_height - height) + base_top - f32::from(self.fraction_vertical_offset()),
                    height,
                )
            } else {
                (f32::from(self.digit_advance()), base_top, digit_height)
            };

            if !axis_mode {
                top = -top;
                height = -height;
            }
            let right = left + advance;
            result.push(NumberGlyphQuad {
                positions: [
                    [left, top, 0.0],
                    [left, top + height, 0.0],
                    [right, top, 0.0],
                    [right, top + height, 0.0],
                ],
            });
            left = right;

            if index + 1 < text.len() {
                let spacing = if after_decimal_point {
                    if self.format_flags & 0x20 != 0 {
                        Some(self.digit_spacing())
                    } else {
                        Some(self.fraction_spacing())
                    }
                } else if matches!(text[index + 1], b'.' | b',') {
                    None
                } else {
                    Some(self.digit_spacing())
                };
                if let Some(spacing) = spacing {
                    left = left + f32::from(spacing);
                }
            }
            if byte == b'.' {
                after_decimal_point = true;
            }
        }
        result
    }

    fn format_integer_digits(&self, integer: i32) -> Vec<u8> {
        let digit_count = i32::from(self.integer_digit_count());
        let mut limit = 1i32;
        if digit_count > 0 {
            for _ in 0..digit_count {
                limit = limit.wrapping_mul(10);
            }
        }
        limit = limit.wrapping_sub(1);
        let absolute = integer.wrapping_abs();
        let displayed = if absolute < limit { absolute } else { limit };

        let text = if self.format_flags & 0x04 == 0 || digit_count == 0 {
            displayed.to_string()
        } else if digit_count > 0 {
            format!("{displayed:0width$}", width = digit_count as usize)
        } else {
            format!(
                "{displayed:<width$}",
                width = digit_count.unsigned_abs() as usize
            )
        };
        if self.format_flags & 0x02 == 0 {
            return text.into_bytes();
        }
        insert_group_separators(text.as_bytes(), self.grouping_interval())
    }

    fn format_fraction_digits(&self, fraction: f64) -> Vec<u8> {
        if self.format_flags & 0x08 == 0 {
            return Vec::new();
        }

        let mut text = format!("{:.6}", fraction.abs()).replace("0.", "");
        while text.len() > 1 && text.ends_with('0') {
            text.pop();
        }
        let requested = i32::from(self.fraction_digit_count());
        if (text.len() as i32) < requested && self.format_flags & 0x10 != 0 {
            text.extend(std::iter::repeat_n(
                '0',
                (requested - text.len() as i32) as usize,
            ));
        } else if (text.len() as i32) > requested && requested > -1 {
            text.truncate(requested as usize);
        }
        text.into_bytes()
    }

    pub fn image_base(&self) -> ImageDefinition {
        ImageDefinition {
            flags: self.flags,
            width: self.width,
            height: self.height,
            custom_origin: self.custom_origin,
            origin_mode: self.origin_mode,
            vertex_colors: self.vertex_colors,
            cref_index: 0,
            cref_count: self.cref_count,
            crefs: self.crefs.clone(),
            field_4c: self.field_4c,
            cre1_index: 0,
            cre1_count: self.cre1_count,
            cre1s: self.cre1s.clone(),
            coordinate_offsets: [
                [f32::from_bits(self.initial_integer as u32), 0.0],
                [self.initial_fraction, 0.0],
            ],
            field_a1: 0,
            node_index: self.node_index,
            has_text_child: false,
            text: None,
        }
    }
}

fn number_glyph_sentinel(segment: NumberGlyphSegment) -> NumberGlyphRecord {
    NumberGlyphRecord {
        glyph_index: -1,
        is_digit: false,
        segment,
        quad: NumberGlyphQuad {
            positions: [[0.0; 3]; 4],
        },
    }
}

fn mode1_cropped_draw(
    mut record: NumberGlyphRecord,
    use_progress: bool,
    keep_top: bool,
    progress: f32,
) -> NumberGlyphMode1Draw {
    let amount = if use_progress {
        progress
    } else {
        1.0 - progress
    };
    let height = record.quad.positions[1][1] - record.quad.positions[0][1];
    let first_uv_crop = if keep_top {
        record.quad.positions[1][1] = record.quad.positions[0][1] + amount * height;
        record.quad.positions[3][1] = record.quad.positions[2][1] + amount * height;
        NumberGlyphFirstUvCrop::MoveTopTowardBottom {
            amount: 1.0 - amount,
        }
    } else {
        record.quad.positions[0][1] += amount * height;
        record.quad.positions[2][1] += amount * height;
        NumberGlyphFirstUvCrop::MoveBottomTowardTop { amount }
    };
    NumberGlyphMode1Draw {
        record,
        first_uv_crop,
    }
}

fn insert_group_separators(text: &[u8], interval: i16) -> Vec<u8> {
    let mut result = Vec::with_capacity(text.len());
    let mut group_size = 0i32;
    for (index, byte) in text.iter().copied().enumerate().rev() {
        result.insert(0, byte);
        group_size += 1;
        if index > 0 && group_size == i32::from(interval) {
            result.insert(0, b',');
            group_size = 0;
        }
    }
    result
}

fn parse_reference_table(
    file: &SrdFile,
    block: &Block,
    declared_count: u16,
    label: &str,
) -> Result<Vec<CrefEntry>, NumberError> {
    let mut entries = vec![
        CrefEntry {
            image_index: 0,
            rectangle_index: 0,
        };
        usize::from(declared_count)
    ];
    for (index, property) in block.properties_with_code(0x4a).enumerate() {
        let destination = entries.get_mut(index).ok_or_else(|| {
            NumberError(format!(
                "CNUM declares {declared_count} {label} records but the child has more"
            ))
        })?;
        destination.image_index = property
            .read_signed_scalar_at(file, 0)
            .ok_or_else(|| NumberError(format!("invalid {label} 0x4a image index")))?
            as i16;
        destination.rectangle_index = property
            .read_signed_scalar_at(file, 1)
            .ok_or_else(|| NumberError(format!("invalid {label} 0x4a rectangle index")))?
            as i16;
    }
    Ok(entries)
}

fn unsigned_scalar(file: &SrdFile, property: &Property, label: &str) -> Result<u32, NumberError> {
    property
        .read_unsigned_scalar(file)
        .ok_or_else(|| NumberError(format!("invalid {label}")))
}

fn signed_scalar(file: &SrdFile, property: &Property, label: &str) -> Result<i32, NumberError> {
    property
        .read_signed_scalar(file)
        .ok_or_else(|| NumberError(format!("invalid {label}")))
}

fn float_scalar(file: &SrdFile, property: &Property, label: &str) -> Result<f32, NumberError> {
    property
        .read_scalar_as_f32(file)
        .ok_or_else(|| NumberError(format!("invalid {label}")))
}

fn reordered_four_bytes(
    file: &SrdFile,
    property: &Property,
    label: &str,
) -> Result<[u8; 4], NumberError> {
    let bytes = property.value_bytes(file);
    if bytes.len() < 4 {
        return Err(NumberError(format!("{label} has fewer than four bytes")));
    }
    Ok([bytes[1], bytes[2], bytes[3], bytes[0]])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn definition() -> NumberDefinition {
        NumberDefinition {
            flags: 0,
            width: 128.0,
            height: 128.0,
            custom_origin: [0.0; 2],
            origin_mode: 0,
            vertex_colors: [[0xff; 4]; 4],
            cref_count: 1,
            crefs: vec![CrefEntry {
                image_index: 0,
                rectangle_index: 0,
            }],
            field_4c: 0,
            cre1_count: 1,
            cre1s: vec![CrefEntry {
                image_index: 0,
                rectangle_index: 0,
            }],
            format_flags: 0,
            field_78: 0,
            initial_integer: -12,
            initial_fraction: -0.5,
            fields_83_8b: [10, 20, 4, 6, 2, 3, 5, 1, 2],
            field_8c: [0.5, 0.25],
            fields_8d_94: [3, 4, 5, 6, 10, 11, 12, 13],
            node_index: 0,
        }
    }

    #[test]
    fn value_and_special_glyph_mapping_match_runtime_inputs() {
        let definition = definition();
        assert_eq!(definition.initial_value(), -12.5);
        assert_eq!(definition.integer_digit_count(), 5);
        assert_eq!(definition.grouping_interval(), 3);
        assert_eq!(definition.fraction_digit_count(), 2);
        assert_eq!(definition.value_animation_mode(), 6);
        assert_eq!(definition.glyph_index_for_ascii(b'7'), Some(7));
        assert_eq!(definition.glyph_index_for_ascii(b'+'), Some(10));
        assert_eq!(definition.glyph_index_for_ascii(b'-'), Some(11));
        assert_eq!(definition.glyph_index_for_ascii(b','), Some(12));
        assert_eq!(definition.glyph_index_for_ascii(b'.'), Some(13));
        assert_eq!(definition.glyph_index_for_ascii(b'x'), None);
    }

    #[test]
    fn glyph_measurement_and_positions_preserve_the_two_binary_spacing_paths() {
        let definition = definition();
        let text = b"1,2.3";
        assert_eq!(definition.measure_string_width(text), 35.0);
        let quads = definition.build_glyph_positions(text, true);
        assert_eq!(quads.len(), 5);
        assert_eq!(
            quads[0].positions,
            [
                [0.0, 0.0, 0.0],
                [0.0, 20.0, 0.0],
                [10.0, 0.0, 0.0],
                [10.0, 20.0, 0.0],
            ]
        );
        assert_eq!(
            quads[1].positions,
            [
                [10.0, 12.0, 0.0],
                [10.0, 18.0, 0.0],
                [14.0, 12.0, 0.0],
                [14.0, 18.0, 0.0],
            ]
        );
        assert_eq!(quads[2].positions[0], [15.0, 0.0, 0.0]);
        assert_eq!(
            quads[3].positions,
            [
                [25.0, 9.0, 0.0],
                [25.0, 15.0, 0.0],
                [29.0, 9.0, 0.0],
                [29.0, 15.0, 0.0],
            ]
        );
        assert_eq!(
            quads[4].positions,
            [
                [30.0, 11.0, 0.0],
                [30.0, 16.0, 0.0],
                [35.0, 11.0, 0.0],
                [35.0, 16.0, 0.0],
            ]
        );
        assert_eq!(
            definition.build_glyph_positions(b"1", false)[0].positions,
            [
                [0.0, -0.0, 0.0],
                [0.0, -20.0, 0.0],
                [10.0, -0.0, 0.0],
                [10.0, -20.0, 0.0],
            ]
        );
    }

    #[test]
    fn formatting_and_glyph_filtering_follow_the_number_cast_build_order() {
        let mut definition = definition();
        definition.format_flags = 0x01 | 0x02 | 0x04 | 0x08 | 0x10;
        definition.initial_integer = 12345;
        definition.initial_fraction = 0.5;
        let formatted = definition.initial_formatted_text();
        assert_eq!(formatted.sign, b"+");
        assert_eq!(formatted.integer, b"12,345");
        assert_eq!(formatted.decimal_point, b".");
        assert_eq!(formatted.fraction, b"50");
        assert_eq!(formatted.combined, b"+12,345.50");

        definition.cref_count = 11;
        definition.fields_8d_94[4] = 10;
        definition.fields_8d_94[6] = 20;
        definition.fields_8d_94[7] = -1;
        let records = definition.build_glyph_records(&formatted, true);
        assert_eq!(records.len(), 9);
        assert_eq!(records[0].glyph_index, 10);
        assert!(!records[0].is_digit);
        assert!(records[0].drawable());
        assert_eq!(records[1].glyph_index, 1);
        assert!(records[1].is_digit);
        assert_eq!(records[3].glyph_index, 3);
        assert_eq!(records[5].glyph_index, 5);
        assert_eq!(records[6].glyph_index, -1);
        assert!(!records[6].drawable());
        assert_eq!(records[7].glyph_index, 5);
        assert_eq!(records[8].glyph_index, 0);
        let positions = definition.build_glyph_positions(&formatted.combined, true);
        assert_eq!(records[3].quad, positions[3]);
        assert_eq!(records[7].quad, positions[7]);

        let first_history = definition.first_history_render_records(&formatted, true);
        assert_eq!(
            first_history
                .iter()
                .map(|record| record.glyph_index)
                .collect::<Vec<_>>(),
            [10, 5, 4, 3, 2, 1, -1, 5, 0]
        );
    }

    #[test]
    fn number_image_base_uses_both_tables_with_zero_initial_selectors() {
        let definition = definition();
        let base = definition.image_base();
        assert_eq!(base.cref_index, 0);
        assert_eq!(base.cre1_index, 0);
        assert_eq!(base.crefs, definition.crefs);
        assert_eq!(base.cre1s, definition.cre1s);
        assert_eq!(
            base.coordinate_offsets[0][0].to_bits(),
            definition.initial_integer as u32
        );
        assert_eq!(base.coordinate_offsets[1][0], -0.5);
        assert_eq!(NumberDefinition::INITIAL_COORDINATE_OFFSET_SCALE, 0.0);
        assert_eq!(
            definition
                .glyph_coordinate_state(7, ImageReferenceChannel::Cref)
                .reference_index,
            7
        );
        assert_eq!(
            definition
                .glyph_coordinate_state(7, ImageReferenceChannel::Cre1)
                .reference_index,
            0
        );
    }

    #[test]
    fn mode1_history_scrolls_changed_digits_in_the_binary_pair_order() {
        let mut definition = definition();
        definition.cref_count = 14;
        definition.format_flags = 0;
        let older = definition.build_history_record(11, 0.0, true);
        let newer = definition.build_history_record(12, 0.0, true);
        let plan = definition.mode1_history_render_plan(&older, &newer, 0.25, false);
        assert!(plan.increasing);
        assert_eq!(
            plan.draws
                .iter()
                .map(|draw| draw.record.glyph_index)
                .collect::<Vec<_>>(),
            [1, 2, 1]
        );
        assert_eq!(plan.draws[1].record.quad.positions[0][1], 5.0);
        assert_eq!(plan.draws[1].record.quad.positions[1][1], 20.0);
        assert_eq!(plan.draws[2].record.quad.positions[0][1], 0.0);
        assert_eq!(plan.draws[2].record.quad.positions[1][1], 5.0);

        let coordinates = ResolvedImageCoordinates {
            image_index: 0,
            coordinates: [[0.0, 0.1], [0.0, 0.9], [1.0, 0.1], [1.0, 0.9]],
            selected_sampler: None,
        };
        let mut newer_coordinates = coordinates;
        plan.draws[1].first_uv_crop.apply(&mut newer_coordinates);
        assert_eq!(newer_coordinates.coordinates[1][1], 0.7);
        assert_eq!(newer_coordinates.coordinates[3][1], 0.7);
        let mut older_coordinates = coordinates;
        plan.draws[2].first_uv_crop.apply(&mut older_coordinates);
        assert_eq!(older_coordinates.coordinates[0][1], 0.7);
        assert_eq!(older_coordinates.coordinates[2][1], 0.7);
    }

    #[test]
    fn mode1_history_interpolates_punctuation_and_preserves_equal_direction() {
        let mut definition = definition();
        definition.cref_count = 14;
        definition.format_flags = 1;
        let mut older = definition.build_history_record(-1, 0.0, true);
        let mut newer = definition.build_history_record(1, 0.0, true);
        for position in &mut older.sign[0].quad.positions {
            position[0] += 100.0;
        }
        let plan = definition.mode1_history_render_plan(&older, &newer, 0.25, false);
        assert!(plan.increasing);
        assert_eq!(
            plan.draws[0].record.glyph_index,
            definition.special_glyphs().plus
        );
        assert_eq!(plan.draws[0].record.quad.positions[0][0], 25.0);
        assert_eq!(plan.draws[0].first_uv_crop, NumberGlyphFirstUvCrop::None);

        newer.total_value = older.total_value;
        assert!(
            definition
                .mode1_history_render_plan(&older, &newer, 0.5, true)
                .increasing
        );
        assert!(
            !definition
                .mode1_history_render_plan(&older, &newer, 0.5, false)
                .increasing
        );
    }
}
