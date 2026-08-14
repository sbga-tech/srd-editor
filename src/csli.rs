use std::fmt;

use crate::vtbf::{Block, Property, SrdFile};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsliError(pub String);

impl fmt::Display for CsliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CsliError {}

#[derive(Debug, Clone, PartialEq)]
pub struct SlicCell {
    pub flags: u32,
    pub explicit_width: f32,
    pub explicit_height: f32,
    pub field_3a: Option<[u8; 4]>,
    pub field_33: Option<[u8; 4]>,
    pub field_44: Vec<[u8; 4]>,
    pub cref_index: i16,
}

impl SlicCell {
    pub fn runtime_color_3a(&self) -> [u8; 4] {
        self.field_3a.unwrap_or([0; 4])
    }

    pub fn runtime_color_33(&self) -> [u8; 4] {
        self.field_33.unwrap_or([0; 4])
    }

    pub fn runtime_vertex_colors(&self) -> Result<[[u8; 4]; 4], CsliError> {
        if self.field_44.len() > 4 {
            return Err(CsliError("SLIC cell has more than four 0x44 colors".into()));
        }
        let mut colors = [[0; 4]; 4];
        colors[..self.field_44.len()].copy_from_slice(&self.field_44);
        Ok(colors)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrefEntry {
    pub image_index: i16,
    pub rectangle_index: i16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CsliDefinition {
    pub field_80: u32,
    pub width: f32,
    pub height: f32,
    pub custom_origin: [f32; 2],
    pub field_44: [[u8; 4]; 4],
    pub origin_mode: u8,
    pub columns: u16,
    pub rows: u16,
    pub explicit_width_cell_count: u16,
    pub explicit_height_cell_count: u16,
    pub cref_count: u16,
    pub crefs: Vec<CrefEntry>,
    pub node_index: i32,
    pub cells: Vec<SlicCell>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeneratedCellRect {
    pub active: bool,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SliceQuad {
    pub cell_index: usize,
    pub positions: [[f32; 3]; 4],
    pub normalized_cell_coordinates: [[f32; 2]; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SliceVertexColors {
    pub primary: [u8; 4],
    pub secondary: [u8; 4],
}

impl CsliDefinition {
    pub fn from_block(file: &SrdFile, block: &Block) -> Result<Self, CsliError> {
        if !block.is_tag(b"CSLI") {
            return Err(CsliError("block is not CSLI".into()));
        }

        let mut result = Self {
            field_80: 0,
            width: 0.0,
            height: 0.0,
            custom_origin: [0.0, 0.0],
            field_44: [[0xff; 4]; 4],
            origin_mode: 0,
            columns: 0,
            rows: 0,
            explicit_width_cell_count: 0,
            explicit_height_cell_count: 0,
            cref_count: 0,
            crefs: Vec::new(),
            node_index: -1,
            cells: Vec::new(),
        };
        let mut field_44_index = 0usize;
        for property in &block.properties {
            match property.code {
                0x80 => result.field_80 = unsigned_scalar(file, property, "CSLI 0x80")?,
                0x40 => result.width = float_scalar(file, property, "CSLI 0x40")?,
                0x41 => result.height = float_scalar(file, property, "CSLI 0x41")?,
                0x42 => result.custom_origin[0] = float_scalar(file, property, "CSLI 0x42")?,
                0x43 => result.custom_origin[1] = float_scalar(file, property, "CSLI 0x43")?,
                0x44 => {
                    let destination = result.field_44.get_mut(field_44_index).ok_or_else(|| {
                        CsliError("CSLI has more than four 0x44 properties".into())
                    })?;
                    *destination = reordered_four_bytes(file, property, "CSLI 0x44")?;
                    field_44_index += 1;
                }
                0x4b => result.origin_mode = unsigned_scalar(file, property, "CSLI 0x4b")? as u8,
                0x81 => result.columns = unsigned_scalar(file, property, "CSLI 0x81")? as u16,
                0x82 => result.rows = unsigned_scalar(file, property, "CSLI 0x82")? as u16,
                0x84 => {
                    result.explicit_width_cell_count =
                        unsigned_scalar(file, property, "CSLI 0x84")? as u16
                }
                0x85 => {
                    result.explicit_height_cell_count =
                        unsigned_scalar(file, property, "CSLI 0x85")? as u16
                }
                0x45 => result.cref_count = unsigned_scalar(file, property, "CSLI 0x45")? as u16,
                0x51 => result.node_index = unsigned_scalar(file, property, "CSLI 0x51")? as i32,
                _ => {}
            }
        }

        let slic_blocks = block
            .children
            .iter()
            .filter(|child| child.is_tag(b"SLIC"))
            .collect::<Vec<_>>();
        if slic_blocks.len() > 1 {
            return Err(CsliError("CSLI has more than one SLIC child".into()));
        }
        if let Some(slic) = slic_blocks.first() {
            result.cells = parse_slic(file, slic)?;
        }

        let cref_blocks = block
            .children
            .iter()
            .filter(|child| child.is_tag(b"CREF"))
            .collect::<Vec<_>>();
        if cref_blocks.len() > 1 {
            return Err(CsliError("CSLI has more than one CREF child".into()));
        }
        if result.cref_count != 0
            && let Some(cref) = cref_blocks.first()
        {
            result.crefs = parse_cref(file, cref)?;
        }

        Ok(result)
    }

    pub fn expected_cell_count(&self) -> usize {
        usize::from(self.columns) * usize::from(self.rows)
    }

    pub fn runtime_origin_offset(&self) -> [f32; 2] {
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
        [factors[0] * self.width, factors[1] * self.height]
    }

    #[allow(clippy::assign_op_pattern)]
    pub fn generate_cell_rects(&self) -> Result<Vec<GeneratedCellRect>, CsliError> {
        self.generate_cell_rects_with_size([self.width, self.height])
    }

    /// Reproduces `srd_get_cast_csli_cell_rects ->
    /// srd_generate_csli_cell_rects` with the current SrImage width/height at
    /// CAST `+0x184/+0x188`. The parsed explicit cell widths/heights and their
    /// first-row/first-column sums stay unchanged while animation channels
    /// 11/12 can replace the remaining distributable extent.
    #[allow(clippy::assign_op_pattern)]
    pub fn generate_cell_rects_with_size(
        &self,
        runtime_size: [f32; 2],
    ) -> Result<Vec<GeneratedCellRect>, CsliError> {
        let expected = self.expected_cell_count();
        if self.cells.len() != expected {
            return Err(CsliError(format!(
                "CSLI grid is {}x{} but SLIC contains {} records",
                self.columns,
                self.rows,
                self.cells.len()
            )));
        }
        if self.columns == 0 || self.rows == 0 {
            return Ok(Vec::new());
        }

        let mut first_row_explicit_width = 0.0f32;
        let mut first_column_explicit_height = 0.0f32;
        for row in 0..usize::from(self.rows) {
            for column in 0..usize::from(self.columns) {
                let cell = &self.cells[row * usize::from(self.columns) + column];
                if column == 0 && cell.flags & 0x02 != 0 {
                    // The game loads the new value first, then adds the accumulator.
                    first_column_explicit_height =
                        cell.explicit_height + first_column_explicit_height;
                }
                if row == 0 && cell.flags & 0x01 != 0 {
                    // Preserve the same operand order for signed zero and NaN payloads.
                    first_row_explicit_width = cell.explicit_width + first_row_explicit_width;
                }
            }
        }

        let denominator_x =
            ((i32::from(self.columns) - i32::from(self.explicit_width_cell_count)) as f32).max(1.0);
        let denominator_y =
            ((i32::from(self.rows) - i32::from(self.explicit_height_cell_count)) as f32).max(1.0);
        let default_width = (runtime_size[0] - first_row_explicit_width) / denominator_x;
        let default_height = (runtime_size[1] - first_column_explicit_height) / denominator_y;

        let mut generated = Vec::with_capacity(expected);
        let mut y = 0.0f32;
        for row in 0..usize::from(self.rows) {
            let row_start = row * usize::from(self.columns);
            let first_cell = &self.cells[row_start];
            let row_height = if first_cell.flags & 0x02 != 0 {
                first_cell.explicit_height
            } else {
                default_height
            };
            let mut x = 0.0f32;
            for column in 0..usize::from(self.columns) {
                let source = &self.cells[row_start + column];
                let width = if source.flags & 0x01 != 0 {
                    source.explicit_width
                } else {
                    default_width
                };
                generated.push(GeneratedCellRect {
                    active: (source.flags >> 8) & 1 != 0,
                    x,
                    y,
                    width,
                    height: row_height,
                });
                x += width;
            }
            y += row_height;
        }
        Ok(generated)
    }

    pub fn generate_active_quads(&self, axis_mode: bool) -> Result<Vec<SliceQuad>, CsliError> {
        self.generate_active_quads_with_geometry(
            axis_mode,
            [self.width, self.height],
            self.runtime_origin_offset(),
        )
    }

    /// Builds the same active cell quads using the current SrImage geometry.
    /// `runtime_origin` is passed separately because origin modes are
    /// recomputed when animation replaces width/height, while custom origins
    /// remain literal.
    pub fn generate_active_quads_with_geometry(
        &self,
        axis_mode: bool,
        runtime_size: [f32; 2],
        runtime_origin: [f32; 2],
    ) -> Result<Vec<SliceQuad>, CsliError> {
        let inverse_width = 1.0f32 / runtime_size[0];
        let inverse_height = 1.0f32 / runtime_size[1];
        let mut quads = Vec::new();

        for (cell_index, cell) in self
            .generate_cell_rects_with_size(runtime_size)?
            .into_iter()
            .enumerate()
        {
            if !cell.active {
                continue;
            }

            // Preserve the operation grouping used by SrSliceCast's virtual
            // quad builder at 0xADB2B0.
            let left = cell.x - runtime_origin[0];
            let right = left + cell.width;
            let far_y = (cell.height - runtime_origin[1]) + cell.y;
            let near_y = cell.y - runtime_origin[1];
            let (first_y, second_y) = if axis_mode {
                (near_y, far_y)
            } else {
                let second_y = -far_y;
                let first_y = cell.height - far_y;
                (first_y, second_y)
            };

            let x0 = inverse_width * cell.x;
            let y0 = inverse_height * cell.y;
            let x1 = (cell.x + cell.width) * inverse_width;
            let y1 = (cell.y + cell.height) * inverse_height;

            quads.push(SliceQuad {
                cell_index,
                positions: [
                    [left, first_y, 0.0],
                    [left, second_y, 0.0],
                    [right, first_y, 0.0],
                    [right, second_y, 0.0],
                ],
                normalized_cell_coordinates: [[x0, y0], [x0, y1], [x1, y0], [x1, y1]],
            });
        }

        Ok(quads)
    }

    pub fn cell_cref(&self, cell_index: usize) -> Option<CrefEntry> {
        let selector = usize::try_from(self.cells.get(cell_index)?.cref_index).ok()?;
        if selector >= usize::from(self.cref_count) {
            return None;
        }
        self.crefs.get(selector).copied()
    }
}

pub fn slice_texture_coordinates(rectangle: [f32; 4], flags: u32) -> [[f32; 2]; 4] {
    let mut x0 = rectangle[0];
    let mut y0 = rectangle[1];
    let mut x1 = rectangle[2];
    let mut y1 = rectangle[3];

    if flags & 0x10 != 0 {
        std::mem::swap(&mut x0, &mut x1);
    }
    if flags & 0x20 != 0 {
        std::mem::swap(&mut y0, &mut y1);
    }

    match flags & 0xc0 {
        0x40 => [[x1, y0], [x0, y0], [x1, y1], [x0, y1]],
        0x80 => [[x1, y1], [x1, y0], [x0, y1], [x0, y0]],
        0xc0 => [[x0, y1], [x1, y1], [x0, y0], [x1, y0]],
        _ => [[x0, y0], [x0, y1], [x1, y0], [x1, y1]],
    }
}

pub fn slice_vertex_colors(
    definition: &CsliDefinition,
    cell: &SlicCell,
    normalized_cell_coordinates: [[f32; 2]; 4],
    multiplicative_tint: [u8; 4],
    additive_tint: [u8; 4],
) -> Result<[SliceVertexColors; 4], CsliError> {
    let cell_multiply = cell.runtime_color_3a();
    let cell_add = cell.runtime_color_33();
    let per_vertex = cell.runtime_vertex_colors()?;

    Ok(std::array::from_fn(|vertex_index| {
        let [x, y] = normalized_cell_coordinates[vertex_index];
        let first_edge = lerp_color_game(definition.field_44[0], definition.field_44[1], y);
        let second_edge = lerp_color_game(definition.field_44[2], definition.field_44[3], y);
        let base = lerp_color_game(first_edge, second_edge, x);
        let primary = multiply_color_game(
            multiply_color_game(
                multiply_color_game(per_vertex[vertex_index], cell_multiply),
                base,
            ),
            multiplicative_tint,
        );
        let secondary = add_color_saturating_game(additive_tint, cell_add);
        SliceVertexColors { primary, secondary }
    }))
}

pub fn lerp_color_game(first: [u8; 4], second: [u8; 4], factor: f32) -> [u8; 4] {
    let inverse = 1.0f32 - factor;
    std::array::from_fn(|index| {
        let value = f32::from(first[index]) * inverse + f32::from(second[index]) * factor;
        if value <= 255.0 {
            value.max(0.0) as u8
        } else {
            255
        }
    })
}

pub fn multiply_color_game(first: [u8; 4], second: [u8; 4]) -> [u8; 4] {
    std::array::from_fn(|index| {
        let product = u32::from(first[index]) * u32::from(second[index]);
        (product / 255) as u8
    })
}

pub fn add_color_saturating_game(first: [u8; 4], second: [u8; 4]) -> [u8; 4] {
    std::array::from_fn(|index| {
        let sum = u16::from(first[index]) + u16::from(second[index]);
        if sum >= 255 { 255 } else { sum as u8 }
    })
}

pub fn parent_cell_center_offset(
    cell_index: i32,
    cells: &[GeneratedCellRect],
    parent_size: [f32; 2],
    axis_mode: bool,
) -> [f32; 2] {
    let Ok(index) = usize::try_from(cell_index) else {
        return [0.0, 0.0];
    };
    let Some(cell) = cells.get(index) else {
        return [0.0, 0.0];
    };
    if !cell.active {
        return [0.0, 0.0];
    }

    let left = cell.x - parent_size[0];
    let right = cell.width + left;
    let x = (right + left) * 0.5;

    let mut first = cell.y - parent_size[1];
    let mut second = (cell.height - parent_size[1]) + cell.y;
    if !axis_mode {
        second = -second;
        first = cell.height + second;
    }
    let y = (first + second) * 0.5;
    [x, y]
}

fn parse_slic(file: &SrdFile, block: &Block) -> Result<Vec<SlicCell>, CsliError> {
    let records = split_records(&block.properties);
    records
        .iter()
        .map(|properties| {
            let mut cell = SlicCell {
                flags: 0,
                explicit_width: 0.0,
                explicit_height: 0.0,
                field_3a: None,
                field_33: None,
                field_44: Vec::new(),
                cref_index: 0,
            };
            for property in properties {
                match property.code {
                    0x83 => cell.flags = unsigned_scalar(file, property, "SLIC 0x83")?,
                    0x40 => cell.explicit_width = float_scalar(file, property, "SLIC 0x40")?,
                    0x41 => cell.explicit_height = float_scalar(file, property, "SLIC 0x41")?,
                    0x3a => {
                        cell.field_3a = Some(reordered_four_bytes(file, property, "SLIC 0x3a")?)
                    }
                    0x33 => {
                        cell.field_33 = Some(reordered_four_bytes(file, property, "SLIC 0x33")?)
                    }
                    0x44 => cell
                        .field_44
                        .push(reordered_four_bytes(file, property, "SLIC 0x44")?),
                    0x46 => cell.cref_index = signed_scalar(file, property, "SLIC 0x46")? as i16,
                    _ => {}
                }
            }
            if cell.flags & 0x200 == 0 {
                cell.flags |= 0x100;
            }
            Ok(cell)
        })
        .collect()
}

fn parse_cref(file: &SrdFile, block: &Block) -> Result<Vec<CrefEntry>, CsliError> {
    let mut entries = Vec::new();
    for property in block.properties_with_code(0x4a) {
        let image_index = property
            .read_signed_scalar_at(file, 0)
            .ok_or_else(|| CsliError("invalid CREF 0x4a image index".into()))?
            as i16;
        let rectangle_index = property
            .read_signed_scalar_at(file, 1)
            .ok_or_else(|| CsliError("invalid CREF 0x4a rectangle index".into()))?
            as i16;
        entries.push(CrefEntry {
            image_index,
            rectangle_index,
        });
    }
    Ok(entries)
}

fn split_records(properties: &[Property]) -> Vec<Vec<&Property>> {
    let mut records = vec![Vec::new()];
    for property in properties {
        if property.code == 0xfe {
            records.push(Vec::new());
        } else {
            records.last_mut().unwrap().push(property);
        }
    }
    if records.last().is_some_and(Vec::is_empty) {
        records.pop();
    }
    records
}

fn unsigned_scalar(file: &SrdFile, property: &Property, label: &str) -> Result<u32, CsliError> {
    property
        .read_unsigned_scalar(file)
        .ok_or_else(|| CsliError(format!("invalid {label}")))
}

fn signed_scalar(file: &SrdFile, property: &Property, label: &str) -> Result<i32, CsliError> {
    property
        .read_signed_scalar(file)
        .ok_or_else(|| CsliError(format!("invalid {label}")))
}

fn float_scalar(file: &SrdFile, property: &Property, label: &str) -> Result<f32, CsliError> {
    property
        .read_scalar_as_f32(file)
        .ok_or_else(|| CsliError(format!("invalid {label}")))
}

fn reordered_four_bytes(
    file: &SrdFile,
    property: &Property,
    label: &str,
) -> Result<[u8; 4], CsliError> {
    let bytes = property.value_bytes(file);
    if bytes.len() < 4 {
        return Err(CsliError(format!("{label} has fewer than four bytes")));
    }
    Ok([bytes[1], bytes[2], bytes[3], bytes[0]])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(flags: u32, width: f32, height: f32) -> SlicCell {
        SlicCell {
            flags,
            explicit_width: width,
            explicit_height: height,
            field_3a: None,
            field_33: None,
            field_44: Vec::new(),
            cref_index: 0,
        }
    }

    #[test]
    fn cell_generation_uses_first_cell_height_for_each_row() {
        let definition = CsliDefinition {
            field_80: 0,
            width: 11.0,
            height: 23.0,
            custom_origin: [0.0, 0.0],
            field_44: [[0xff; 4]; 4],
            origin_mode: 0,
            columns: 2,
            rows: 2,
            explicit_width_cell_count: 0,
            explicit_height_cell_count: 0,
            cref_count: 0,
            crefs: Vec::new(),
            node_index: 0,
            cells: vec![
                cell(0x102, 99.0, 7.0),
                cell(0x103, 3.0, 88.0),
                cell(0x100, 77.0, 66.0),
                cell(0x101, 4.0, 55.0),
            ],
        };
        let result = definition.generate_cell_rects().unwrap();
        assert_eq!(
            result,
            vec![
                GeneratedCellRect {
                    active: true,
                    x: 0.0,
                    y: 0.0,
                    width: 4.0,
                    height: 7.0,
                },
                GeneratedCellRect {
                    active: true,
                    x: 4.0,
                    y: 0.0,
                    width: 3.0,
                    height: 7.0,
                },
                GeneratedCellRect {
                    active: true,
                    x: 0.0,
                    y: 7.0,
                    width: 4.0,
                    height: 8.0,
                },
                GeneratedCellRect {
                    active: true,
                    x: 4.0,
                    y: 7.0,
                    width: 4.0,
                    height: 8.0,
                },
            ]
        );
    }

    #[test]
    fn center_offset_matches_both_binary_branches() {
        let cells = [GeneratedCellRect {
            active: true,
            x: 13.0,
            y: 17.0,
            width: 6.0,
            height: 8.0,
        }];
        assert_eq!(
            parent_cell_center_offset(0, &cells, [10.0, 12.0], true),
            [6.0, 9.0]
        );
        assert_eq!(
            parent_cell_center_offset(0, &cells, [10.0, 12.0], false),
            [6.0, -9.0]
        );
        assert_eq!(
            parent_cell_center_offset(-1, &cells, [10.0, 12.0], true),
            [0.0, 0.0]
        );
        assert_eq!(
            parent_cell_center_offset(1, &cells, [10.0, 12.0], true),
            [0.0, 0.0]
        );
        let inactive = [GeneratedCellRect {
            active: false,
            ..cells[0]
        }];
        assert_eq!(
            parent_cell_center_offset(0, &inactive, [10.0, 12.0], true),
            [0.0, 0.0]
        );
    }

    #[test]
    fn runtime_origin_uses_the_binary_nine_mode_table_and_custom_fallback() {
        let mut definition = CsliDefinition {
            field_80: 0,
            width: 20.0,
            height: 30.0,
            custom_origin: [7.0, 9.0],
            field_44: [[0xff; 4]; 4],
            origin_mode: 4,
            columns: 0,
            rows: 0,
            explicit_width_cell_count: 0,
            explicit_height_cell_count: 0,
            cref_count: 0,
            crefs: Vec::new(),
            node_index: 0,
            cells: Vec::new(),
        };
        assert_eq!(definition.runtime_origin_offset(), [10.0, 15.0]);
        definition.origin_mode = 8;
        assert_eq!(definition.runtime_origin_offset(), [20.0, 30.0]);
        definition.origin_mode = 9;
        assert_eq!(definition.runtime_origin_offset(), [7.0, 9.0]);
    }

    #[test]
    fn active_quad_matches_binary_vertex_order_and_cell_coordinates() {
        let definition = CsliDefinition {
            field_80: 0,
            width: 16.0,
            height: 8.0,
            custom_origin: [0.0, 0.0],
            field_44: [[0xff; 4]; 4],
            origin_mode: 4,
            columns: 2,
            rows: 1,
            explicit_width_cell_count: 2,
            explicit_height_cell_count: 1,
            cref_count: 0,
            crefs: Vec::new(),
            node_index: 0,
            cells: vec![cell(0x103, 4.0, 8.0), cell(0x103, 12.0, 8.0)],
        };

        let quads_2d = definition.generate_active_quads(true).unwrap();
        assert_eq!(quads_2d.len(), 2);
        assert_eq!(
            quads_2d[1],
            SliceQuad {
                cell_index: 1,
                positions: [
                    [-4.0, -4.0, 0.0],
                    [-4.0, 4.0, 0.0],
                    [8.0, -4.0, 0.0],
                    [8.0, 4.0, 0.0],
                ],
                normalized_cell_coordinates: [[0.25, 0.0], [0.25, 1.0], [1.0, 0.0], [1.0, 1.0],],
            }
        );

        let quads_3d = definition.generate_active_quads(false).unwrap();
        assert_eq!(
            quads_3d[1].positions,
            [
                [-4.0, 4.0, 0.0],
                [-4.0, -4.0, 0.0],
                [8.0, 4.0, 0.0],
                [8.0, -4.0, 0.0],
            ]
        );
    }

    #[test]
    fn runtime_extent_reflows_default_cells_and_normalized_coordinates() {
        let definition = CsliDefinition {
            field_80: 0,
            width: 10.0,
            height: 8.0,
            custom_origin: [0.0, 0.0],
            field_44: [[0xff; 4]; 4],
            origin_mode: 4,
            columns: 2,
            rows: 1,
            explicit_width_cell_count: 1,
            explicit_height_cell_count: 0,
            cref_count: 0,
            crefs: Vec::new(),
            node_index: 0,
            cells: vec![cell(0x101, 4.0, 0.0), cell(0x100, 0.0, 0.0)],
        };

        let rects = definition
            .generate_cell_rects_with_size([20.0, 12.0])
            .unwrap();
        assert_eq!(rects[0].width, 4.0);
        assert_eq!(rects[1].x, 4.0);
        assert_eq!(rects[1].width, 16.0);
        assert_eq!(rects[1].height, 12.0);

        let quads = definition
            .generate_active_quads_with_geometry(true, [20.0, 12.0], [10.0, 6.0])
            .unwrap();
        assert_eq!(
            quads[1],
            SliceQuad {
                cell_index: 1,
                positions: [
                    [-6.0, -6.0, 0.0],
                    [-6.0, 6.0, 0.0],
                    [10.0, -6.0, 0.0],
                    [10.0, 6.0, 0.0],
                ],
                normalized_cell_coordinates: [[0.2, 0.0], [0.2, 1.0], [1.0, 0.0], [1.0, 1.0],],
            }
        );
    }

    #[test]
    fn inactive_cells_do_not_emit_render_quads() {
        let definition = CsliDefinition {
            field_80: 0,
            width: 4.0,
            height: 2.0,
            custom_origin: [0.0, 0.0],
            field_44: [[0xff; 4]; 4],
            origin_mode: 0,
            columns: 1,
            rows: 1,
            explicit_width_cell_count: 1,
            explicit_height_cell_count: 1,
            cref_count: 0,
            crefs: Vec::new(),
            node_index: 0,
            cells: vec![cell(0x203, 4.0, 2.0)],
        };

        assert!(definition.generate_active_quads(true).unwrap().is_empty());
    }

    #[test]
    fn cref_selector_uses_signed_slic_index_and_declared_count() {
        let mut definition = CsliDefinition {
            field_80: 0,
            width: 0.0,
            height: 0.0,
            custom_origin: [0.0, 0.0],
            field_44: [[0xff; 4]; 4],
            origin_mode: 0,
            columns: 1,
            rows: 1,
            explicit_width_cell_count: 0,
            explicit_height_cell_count: 0,
            cref_count: 1,
            crefs: vec![CrefEntry {
                image_index: 3,
                rectangle_index: 7,
            }],
            node_index: 0,
            cells: vec![cell(0x100, 0.0, 0.0)],
        };

        assert_eq!(definition.cell_cref(0), Some(definition.crefs[0]));
        definition.cells[0].cref_index = -1;
        assert_eq!(definition.cell_cref(0), None);
        definition.cells[0].cref_index = 1;
        assert_eq!(definition.cell_cref(0), None);
    }

    #[test]
    fn texture_coordinate_flags_match_all_binary_ordering_branches() {
        let rectangle = [1.0, 2.0, 3.0, 4.0];
        assert_eq!(
            slice_texture_coordinates(rectangle, 0),
            [[1.0, 2.0], [1.0, 4.0], [3.0, 2.0], [3.0, 4.0]]
        );
        assert_eq!(
            slice_texture_coordinates(rectangle, 0x10),
            [[3.0, 2.0], [3.0, 4.0], [1.0, 2.0], [1.0, 4.0]]
        );
        assert_eq!(
            slice_texture_coordinates(rectangle, 0x20),
            [[1.0, 4.0], [1.0, 2.0], [3.0, 4.0], [3.0, 2.0]]
        );
        assert_eq!(
            slice_texture_coordinates(rectangle, 0x40),
            [[3.0, 2.0], [1.0, 2.0], [3.0, 4.0], [1.0, 4.0]]
        );
        assert_eq!(
            slice_texture_coordinates(rectangle, 0x80),
            [[3.0, 4.0], [3.0, 2.0], [1.0, 4.0], [1.0, 2.0]]
        );
        assert_eq!(
            slice_texture_coordinates(rectangle, 0xc0),
            [[1.0, 4.0], [3.0, 4.0], [1.0, 2.0], [3.0, 2.0]]
        );
    }

    #[test]
    fn color_helpers_match_binary_truncation_multiply_and_saturating_add() {
        assert_eq!(
            lerp_color_game([0, 10, 250, 255], [255, 20, 10, 0], 0.5),
            [127, 15, 130, 127]
        );
        assert_eq!(
            multiply_color_game([255, 128, 1, 0], [128, 128, 255, 255]),
            [128, 64, 1, 0]
        );
        assert_eq!(
            add_color_saturating_game([200, 254, 255, 0], [54, 1, 1, 0]),
            [254, 255, 255, 0]
        );
    }

    #[test]
    fn slice_vertex_color_chain_uses_bilinear_base_and_two_game_combiners() {
        let definition = CsliDefinition {
            field_80: 0,
            width: 0.0,
            height: 0.0,
            custom_origin: [0.0, 0.0],
            field_44: [
                [0, 0, 0, 0],
                [100, 100, 100, 100],
                [200, 200, 200, 200],
                [255, 255, 255, 255],
            ],
            origin_mode: 0,
            columns: 1,
            rows: 1,
            explicit_width_cell_count: 0,
            explicit_height_cell_count: 0,
            cref_count: 0,
            crefs: Vec::new(),
            node_index: 0,
            cells: Vec::new(),
        };
        let cell = SlicCell {
            flags: 0,
            explicit_width: 0.0,
            explicit_height: 0.0,
            field_3a: Some([255, 255, 255, 255]),
            field_33: Some([5, 6, 7, 8]),
            field_44: vec![[255, 255, 255, 255]; 4],
            cref_index: 0,
        };
        let colors = slice_vertex_colors(
            &definition,
            &cell,
            [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]],
            [255, 255, 255, 255],
            [10, 20, 30, 40],
        )
        .unwrap();

        assert_eq!(colors[0].primary, [0, 0, 0, 0]);
        assert_eq!(colors[1].primary, [100, 100, 100, 100]);
        assert_eq!(colors[2].primary, [200, 200, 200, 200]);
        assert_eq!(colors[3].primary, [255, 255, 255, 255]);
        assert_eq!(colors[0].secondary, [15, 26, 37, 48]);
        assert_eq!(colors[3].secondary, [15, 26, 37, 48]);
    }

    #[test]
    fn omitted_slic_colors_use_zeroed_project_allocation_defaults() {
        let cell = SlicCell {
            flags: 0,
            explicit_width: 0.0,
            explicit_height: 0.0,
            field_3a: None,
            field_33: None,
            field_44: vec![[1, 2, 3, 4]],
            cref_index: 0,
        };
        assert_eq!(cell.runtime_color_3a(), [0; 4]);
        assert_eq!(cell.runtime_color_33(), [0; 4]);
        assert_eq!(
            cell.runtime_vertex_colors().unwrap(),
            [[1, 2, 3, 4], [0; 4], [0; 4], [0; 4]]
        );
    }
}
