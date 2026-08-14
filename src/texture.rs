use std::fmt;
use std::path::{Path, PathBuf};

use crate::csli::{CrefEntry, CsliDefinition, slice_texture_coordinates};
use crate::vtbf::{Block, SrdFile};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureError(pub String);

impl fmt::Display for TextureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for TextureError {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextureCrop {
    pub normalized_rectangle: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u32)]
pub enum TextureAddressMode {
    Wrap = 1,
    Clamp = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u32)]
pub enum TextureFilter {
    Point = 1,
    Linear = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureSamplerState {
    pub address_u: TextureAddressMode,
    pub address_v: TextureAddressMode,
    pub min_filter: TextureFilter,
    pub mag_filter: TextureFilter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureSamplerPair {
    pub linear: TextureSamplerState,
    pub point: TextureSamplerState,
}

impl TextureSamplerPair {
    pub fn select(self, point_sampled: bool) -> TextureSamplerState {
        if point_sampled {
            self.point
        } else {
            self.linear
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextureDefinition {
    pub filename: Vec<u8>,
    pub width: u16,
    pub height: u16,
    pub field_62: u32,
    pub crop_count: u32,
    pub crops: Vec<TextureCrop>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextureList {
    pub declared_count: u16,
    pub textures: Vec<TextureDefinition>,
}

pub const SURFBOARD_TEXTURE_RELATIVE_ROOT: &str = "surfboard/texture";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedSliceTexture {
    pub image_index: usize,
    pub rectangle_index: usize,
    pub coordinates: [[f32; 2]; 4],
    pub samplers: TextureSamplerPair,
}

impl TextureDefinition {
    pub fn sampler_pair(&self) -> TextureSamplerPair {
        let address_u = if self.field_62 & 0x00f0 != 0 {
            TextureAddressMode::Clamp
        } else {
            TextureAddressMode::Wrap
        };
        let address_v = if self.field_62 & 0x0f00 != 0 {
            TextureAddressMode::Clamp
        } else {
            TextureAddressMode::Wrap
        };
        let state = |filter| TextureSamplerState {
            address_u,
            address_v,
            min_filter: filter,
            mag_filter: filter,
        };
        TextureSamplerPair {
            linear: state(TextureFilter::Linear),
            point: state(TextureFilter::Point),
        }
    }

    /// Resolves the Chusan Surfride texture root proven by
    /// `chusan_get_surfboard_texture_root_path`. The TEX value remains a base
    /// name and receives the same unconditional `.dds` suffix as the binary.
    pub fn external_dds_path(&self, game_data_root: &Path) -> Result<PathBuf, TextureError> {
        let end = self
            .filename
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(self.filename.len());
        let base = std::str::from_utf8(&self.filename[..end])
            .map_err(|error| TextureError(format!("TEX filename is not UTF-8: {error}")))?;
        let mut path = game_data_root.join(SURFBOARD_TEXTURE_RELATIVE_ROOT);
        path.push(format!("{base}.dds"));
        Ok(path)
    }
}

impl TextureList {
    pub fn from_block(file: &SrdFile, block: &Block) -> Result<Self, TextureError> {
        if !block.is_tag(b"TEXL") {
            return Err(TextureError("block is not TEXL".into()));
        }

        let declared_count = block
            .last_property(0x60)
            .and_then(|property| property.read_unsigned_scalar(file))
            .unwrap_or(0) as u16;
        let mut textures = vec![TextureDefinition::default(); usize::from(declared_count)];
        let mut texture_index = 0usize;
        for child in &block.children {
            if !child.is_tag(b"TEX ") {
                continue;
            }
            let destination = textures.get_mut(texture_index).ok_or_else(|| {
                TextureError(format!(
                    "TEXL declares {declared_count} textures but has more TEX children"
                ))
            })?;
            *destination = parse_texture(file, child)?;
            texture_index += 1;
        }

        Ok(Self {
            declared_count,
            textures,
        })
    }

    pub fn from_file(file: &SrdFile) -> Result<Option<Self>, TextureError> {
        let mut blocks = file
            .blocks_depth_first()
            .filter(|block| block.is_tag(b"TEXL"));
        let Some(block) = blocks.next() else {
            return Ok(None);
        };
        if blocks.next().is_some() {
            return Err(TextureError("SRD has more than one TEXL block".into()));
        }
        Self::from_block(file, block).map(Some)
    }

    pub fn resolve_slice_cell(
        &self,
        definition: &CsliDefinition,
        cell_index: usize,
    ) -> Option<ResolvedSliceTexture> {
        let cell = definition.cells.get(cell_index)?;
        let CrefEntry {
            image_index,
            rectangle_index,
        } = definition.cell_cref(cell_index)?;
        let image_index = usize::try_from(image_index).ok()?;
        let rectangle_index = usize::try_from(rectangle_index).ok()?;
        let texture = self.textures.get(image_index)?;
        let rectangle = texture.crops.get(rectangle_index)?.normalized_rectangle;
        Some(ResolvedSliceTexture {
            image_index,
            rectangle_index,
            coordinates: slice_texture_coordinates(rectangle, cell.flags),
            samplers: texture.sampler_pair(),
        })
    }
}

fn parse_texture(file: &SrdFile, block: &Block) -> Result<TextureDefinition, TextureError> {
    let mut texture = TextureDefinition::default();
    for property in &block.properties {
        match property.code {
            0x40 => {
                texture.width = property
                    .read_unsigned_scalar(file)
                    .ok_or_else(|| TextureError("invalid TEX 0x40 width".into()))?
                    as u16
            }
            0x41 => {
                texture.height = property
                    .read_unsigned_scalar(file)
                    .ok_or_else(|| TextureError("invalid TEX 0x41 height".into()))?
                    as u16
            }
            0x61 => {
                let bytes = property
                    .string_bytes(file)
                    .ok_or_else(|| TextureError("invalid TEX 0x61 filename".into()))?;
                texture.filename = bytes[..bytes.len().min(255)].to_vec();
            }
            0x62 => {
                texture.field_62 = property
                    .read_unsigned_scalar(file)
                    .ok_or_else(|| TextureError("invalid TEX 0x62".into()))?
            }
            0x63 => {
                texture.crop_count = property
                    .read_unsigned_scalar(file)
                    .ok_or_else(|| TextureError("invalid TEX 0x63 crop count".into()))?
            }
            _ => {}
        }
    }

    let crop_count = usize::try_from(texture.crop_count)
        .map_err(|_| TextureError("TEX crop count does not fit usize".into()))?;
    texture.crops = vec![
        TextureCrop {
            normalized_rectangle: [0.0; 4],
        };
        crop_count
    ];
    let mut crop_index = 0usize;
    for child in &block.children {
        if !child.is_tag(b"CROP") {
            continue;
        }
        parse_crop(file, child, &mut texture, &mut crop_index)?;
    }
    Ok(texture)
}

fn parse_crop(
    file: &SrdFile,
    block: &Block,
    texture: &mut TextureDefinition,
    crop_index: &mut usize,
) -> Result<(), TextureError> {
    for property in block.properties_with_code(0x65) {
        let destination = texture.crops.get_mut(*crop_index).ok_or_else(|| {
            TextureError(format!(
                "TEX declares {} crops but CROP has more records",
                texture.crop_count
            ))
        })?;
        let mut values = [0.0f32; 4];
        let value_count = usize::try_from(property.count)
            .ok()
            .and_then(|count| {
                usize::try_from(property.multiplier)
                    .ok()
                    .and_then(|multiplier| count.checked_mul(multiplier))
            })
            .ok_or_else(|| TextureError("CROP 0x65 value count overflow".into()))?;
        if value_count > 4 {
            return Err(TextureError(format!(
                "CROP 0x65 has {value_count} values; game stack record only has four"
            )));
        }
        for (index, value) in values.iter_mut().take(value_count).enumerate() {
            *value = property
                .read_scalar_as_f32_at(file, index)
                .ok_or_else(|| TextureError("invalid CROP 0x65 scalar".into()))?;
        }

        let inverse_width = 1.0f32 / f32::from(texture.width);
        let inverse_height = 1.0f32 / f32::from(texture.height);
        destination.normalized_rectangle = [
            inverse_width * values[0],
            inverse_height * values[1],
            inverse_width * values[2],
            inverse_height * values[3],
        ];
        *crop_index += 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolved_slice_texture_uses_crop_and_slic_ordering() {
        let textures = TextureList {
            declared_count: 1,
            textures: vec![TextureDefinition {
                filename: b"test.dds".to_vec(),
                width: 256,
                height: 128,
                field_62: 0,
                crop_count: 1,
                crops: vec![TextureCrop {
                    normalized_rectangle: [0.25, 0.5, 0.75, 1.0],
                }],
            }],
        };
        let definition = CsliDefinition {
            field_80: 0,
            width: 0.0,
            height: 0.0,
            custom_origin: [0.0, 0.0],
            field_44: [[0; 4]; 4],
            origin_mode: 0,
            columns: 1,
            rows: 1,
            explicit_width_cell_count: 0,
            explicit_height_cell_count: 0,
            cref_count: 1,
            crefs: vec![CrefEntry {
                image_index: 0,
                rectangle_index: 0,
            }],
            node_index: 0,
            cells: vec![crate::csli::SlicCell {
                flags: 0x10,
                explicit_width: 0.0,
                explicit_height: 0.0,
                field_3a: None,
                field_33: None,
                field_44: Vec::new(),
                cref_index: 0,
            }],
        };

        assert_eq!(
            textures.resolve_slice_cell(&definition, 0),
            Some(ResolvedSliceTexture {
                image_index: 0,
                rectangle_index: 0,
                coordinates: [[0.75, 0.5], [0.75, 1.0], [0.25, 0.5], [0.25, 1.0]],
                samplers: TextureSamplerPair {
                    linear: TextureSamplerState {
                        address_u: TextureAddressMode::Wrap,
                        address_v: TextureAddressMode::Wrap,
                        min_filter: TextureFilter::Linear,
                        mag_filter: TextureFilter::Linear,
                    },
                    point: TextureSamplerState {
                        address_u: TextureAddressMode::Wrap,
                        address_v: TextureAddressMode::Wrap,
                        min_filter: TextureFilter::Point,
                        mag_filter: TextureFilter::Point,
                    },
                },
            })
        );
    }

    #[test]
    fn sampler_pair_matches_tex_flags_and_slice_selector() {
        let texture = TextureDefinition {
            field_62: 0x0110,
            ..TextureDefinition::default()
        };

        let pair = texture.sampler_pair();
        assert_eq!(pair.linear.address_u, TextureAddressMode::Clamp);
        assert_eq!(pair.linear.address_v, TextureAddressMode::Clamp);
        assert_eq!(pair.select(false).min_filter, TextureFilter::Linear);
        assert_eq!(pair.select(false).mag_filter, TextureFilter::Linear);
        assert_eq!(pair.select(true).min_filter, TextureFilter::Point);
        assert_eq!(pair.select(true).mag_filter, TextureFilter::Point);
        assert_eq!(TextureAddressMode::Wrap as u32, 1);
        assert_eq!(TextureAddressMode::Clamp as u32, 3);
        assert_eq!(TextureFilter::Point as u32, 1);
        assert_eq!(TextureFilter::Linear as u32, 2);

        let wrap = TextureDefinition::default().sampler_pair();
        assert_eq!(wrap.linear.address_u, TextureAddressMode::Wrap);
        assert_eq!(wrap.linear.address_v, TextureAddressMode::Wrap);
    }

    #[test]
    fn external_dds_path_uses_the_proven_surfboard_texture_root() {
        let texture = TextureDefinition {
            filename: b"CHU_UI_Advertise_00_v250".to_vec(),
            ..TextureDefinition::default()
        };
        assert_eq!(
            texture
                .external_dds_path(Path::new("fixture-root"))
                .unwrap(),
            Path::new("fixture-root")
                .join("surfboard")
                .join("texture")
                .join("CHU_UI_Advertise_00_v250.dds")
        );
    }
}
