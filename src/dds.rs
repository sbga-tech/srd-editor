use std::fmt;

const DDS_MAGIC: u32 = u32::from_le_bytes(*b"DDS ");
const DDS_HEADER_SIZE: usize = 128;
const DDS_PALETTE_SIZE: usize = 1024;

const DDPF_ALPHAPIXELS: u32 = 0x0000_0001;
const DDPF_FOURCC: u32 = 0x0000_0004;
// The game tests this raw bit as a third top-level format family. Its source
// enum name has not been proven, so keep the numeric meaning explicit.
const DDS_PIXEL_FORMAT_FLAG_0X20: u32 = 0x0000_0020;
const DDPF_RGB: u32 = 0x0000_0040;

const DDSCAPS2_CUBEMAP: u32 = 0x0000_0200;
const DDSCAPS2_CUBEMAP_FACES: [u32; 6] = [
    0x0000_0400,
    0x0000_0800,
    0x0000_1000,
    0x0000_2000,
    0x0000_4000,
    0x0000_8000,
];

const FOURCC_DXT1: u32 = u32::from_le_bytes(*b"DXT1");
const FOURCC_DXT2: u32 = u32::from_le_bytes(*b"DXT2");
const FOURCC_DXT3: u32 = u32::from_le_bytes(*b"DXT3");
const FOURCC_DXT4: u32 = u32::from_le_bytes(*b"DXT4");
const FOURCC_DXT5: u32 = u32::from_le_bytes(*b"DXT5");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DdsError(pub String);

impl fmt::Display for DdsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for DdsError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct GameTextureFormat(pub u32);

impl GameTextureFormat {
    pub const R5_G6_B5: Self = Self(0);
    pub const A8_R8_G8_B8: Self = Self(1);
    pub const X8_R8_G8_B8: Self = Self(2);
    pub const R8_G8_B8: Self = Self(3);
    pub const A4_R4_G4_B4: Self = Self(4);
    pub const A1_R5_G5_B5: Self = Self(5);
    pub const A2_R10_G10_B10: Self = Self(6);
    pub const A8_B8_G8_R8: Self = Self(16);
    pub const EMBEDDED_PALETTE_16: Self = Self(32);
    pub const EMBEDDED_PALETTE_8: Self = Self(33);
    pub const EMBEDDED_PALETTE_8_ALPHA: Self = Self(34);
    pub const DXT1: Self = Self(48);
    pub const DXT2: Self = Self(49);
    pub const DXT3: Self = Self(50);
    pub const DXT4: Self = Self(51);
    pub const DXT5: Self = Self(52);
    pub const R16F: Self = Self(64);
    pub const G16_R16F: Self = Self(65);
    pub const A16_B16_G16_R16F: Self = Self(66);
    pub const R32F: Self = Self(80);
    pub const G32_R32F: Self = Self(81);
    pub const A32_B32_G32_R32F: Self = Self(82);

    pub fn is_block_compressed(self) -> bool {
        (48..=52).contains(&self.0)
    }

    pub fn has_embedded_palette(self) -> bool {
        (32..=34).contains(&self.0)
    }

    pub fn direct_creation_format(self) -> Self {
        match self.0 {
            3 | 32 | 33 => Self::A8_R8_G8_B8,
            _ => self,
        }
    }

    pub fn d3d9_format(self) -> D3d9Format {
        D3d9Format(game_texture_format_to_d3d9(self.0))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct D3d9Format(pub u32);

impl D3d9Format {
    pub const UNKNOWN: Self = Self(0);
}

pub const fn game_texture_format_to_d3d9(format: u32) -> u32 {
    match format {
        0 => 23,
        1 => 21,
        2 => 22,
        3 => 20,
        4 => 26,
        5 => 25,
        6 => 35,
        16 => 34,
        32 | 35 => 51,
        33 => 50,
        34 => 28,
        48 => FOURCC_DXT1,
        49 => FOURCC_DXT2,
        50 => FOURCC_DXT3,
        51 => FOURCC_DXT4,
        52 => FOURCC_DXT5,
        64 => 111,
        65 => 112,
        66 => 113,
        80 => 114,
        81 => 115,
        82 => 116,
        96 => 80,
        97 => 77,
        98 => 75,
        99 => 71,
        100 => u32::from_le_bytes(*b"INTZ"),
        101 => u32::from_le_bytes(*b"RAWZ"),
        _ => 0,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DdsSurfaceLevel {
    pub face_index: u32,
    pub mip_index: u32,
    pub data_offset: u32,
    pub width: u32,
    pub height: u32,
    pub byte_len: u32,
}

impl DdsSurfaceLevel {
    pub fn data_range(self) -> Result<std::ops::Range<usize>, DdsError> {
        let start = usize::try_from(self.data_offset)
            .map_err(|_| DdsError("DDS level offset does not fit usize".into()))?;
        let len = usize::try_from(self.byte_len)
            .map_err(|_| DdsError("DDS level length does not fit usize".into()))?;
        let end = start
            .checked_add(len)
            .ok_or_else(|| DdsError("DDS level data range overflows usize".into()))?;
        Ok(start..end)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DdsDescriptor {
    pub format: GameTextureFormat,
    pub bits_per_pixel: u32,
    pub width: u32,
    pub height: u32,
    pub mip_count: u32,
    pub is_cube: bool,
    pub cube_face_mask: u32,
    pub cube_face_count: u32,
    pub cube_face_indices: [u32; 6],
    pub palette_offset: Option<u32>,
    pub levels: Vec<DdsSurfaceLevel>,
    pixel_format_flags: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DdsLoadPolicy {
    pub device_special_mode: bool,
    pub force_d3dx: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum D3d9TextureCreation {
    Direct2d {
        width: u32,
        height: u32,
        mip_levels: u32,
        usage: u32,
        format: D3d9Format,
        pool: u32,
    },
    DirectCube {
        edge_length: u32,
        mip_levels: u32,
        usage: u32,
        format: D3d9Format,
        pool: u32,
    },
    D3dx2d {
        width: u32,
        height: u32,
        mip_levels: u32,
        usage: u32,
        format: D3d9Format,
        pool: u32,
        filter: u32,
        mip_filter: u32,
        color_key: u32,
        palette_offset: Option<u32>,
    },
    D3dxCube {
        edge_length: u32,
        mip_levels: u32,
        usage: u32,
        format: D3d9Format,
        pool: u32,
        filter: u32,
        mip_filter: u32,
        color_key: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum D3d9Direct2dUpload {
    UpdateSurface(D3d9SystemMemoryUpload),
    CompressedLevelBelowFourSkipped {
        mip_level: u32,
        width: u32,
        height: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct D3d9SystemMemoryUpload {
    pub destination_mip_level: u32,
    pub source_offset: u32,
    pub source_byte_len: u32,
    pub width: u32,
    pub height: u32,
    pub source_row_pitch: u32,
    pub row_count: u32,
    pub staging_mip_levels: u32,
    pub staging_usage: u32,
    pub staging_format: D3d9Format,
    pub staging_pool: u32,
    pub staging_lock_flags: u32,
    pub source_rect: [u32; 4],
    pub destination_point: [u32; 2],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedDdsRgba8Level {
    pub mip_index: u32,
    pub width: u32,
    pub height: u32,
    /// Canonical RGBA byte order produced by the independent decoder.
    pub rgba: Vec<u8>,
}

impl DdsDescriptor {
    pub fn parse(bytes: &[u8]) -> Result<Self, DdsError> {
        if bytes.len() < DDS_HEADER_SIZE {
            return Err(DdsError(format!(
                "DDS header requires {DDS_HEADER_SIZE} bytes, got {}",
                bytes.len()
            )));
        }
        let word = |offset| {
            u32::from_le_bytes(
                bytes[offset..offset + 4]
                    .try_into()
                    .expect("checked header"),
            )
        };
        if word(0) != DDS_MAGIC {
            return Err(DdsError("DDS magic is not 'DDS '".into()));
        }

        let pixel_format_flags = word(80);
        let fourcc = word(84);
        let bit_count = word(88);
        let mask_at_100 = word(100);
        let (format, bits_per_pixel) =
            parse_game_format(pixel_format_flags, fourcc, bit_count, mask_at_100)?;

        let caps2 = word(112);
        let is_cube = caps2 & DDSCAPS2_CUBEMAP != 0;
        let mut cube_face_mask = 0u32;
        let mut cube_face_count = 0u32;
        let mut cube_face_indices = [0u32; 6];
        if is_cube {
            for (face, flag) in DDSCAPS2_CUBEMAP_FACES.into_iter().enumerate() {
                if caps2 & flag == 0 {
                    continue;
                }
                cube_face_mask |= 1 << face;
                cube_face_indices[face] = cube_face_count;
                cube_face_count += 1;
            }
        } else {
            cube_face_count = 1;
        }

        let mip_count = match word(28) {
            0 => 1,
            count => count,
        };
        let level_count = cube_face_count
            .checked_mul(mip_count)
            .ok_or_else(|| DdsError("DDS face/mipmap count overflows u32".into()))?;
        let capacity = usize::try_from(level_count)
            .map_err(|_| DdsError("DDS face/mipmap count does not fit usize".into()))?;
        let palette_offset = format
            .has_embedded_palette()
            .then_some(DDS_HEADER_SIZE as u32);
        let mut data_offset = if palette_offset.is_some() {
            (DDS_HEADER_SIZE + DDS_PALETTE_SIZE) as u32
        } else {
            DDS_HEADER_SIZE as u32
        };
        let width = word(16);
        let height = word(12);
        let mut levels = Vec::with_capacity(capacity);
        let mut previous_width = 0u32;
        let mut previous_height = 0u32;

        for index in 0..level_count {
            let mip_index = index % mip_count;
            let face_index = index / mip_count;
            let (level_width, level_height) = if mip_index == 0 {
                (width, height)
            } else {
                ((previous_width >> 1).max(1), (previous_height >> 1).max(1))
            };
            let area = if format.is_block_compressed() {
                round_up_to_four(level_width)?
                    .checked_mul(round_up_to_four(level_height)?)
                    .ok_or_else(|| DdsError("DDS padded level area overflows u32".into()))?
            } else {
                level_width
                    .checked_mul(level_height)
                    .ok_or_else(|| DdsError("DDS level area overflows u32".into()))?
            };
            let byte_len = bits_per_pixel
                .checked_mul(area)
                .ok_or_else(|| DdsError("DDS level bit count overflows u32".into()))?
                >> 3;
            levels.push(DdsSurfaceLevel {
                face_index,
                mip_index,
                data_offset,
                width: level_width,
                height: level_height,
                byte_len,
            });
            data_offset = data_offset
                .checked_add(byte_len)
                .ok_or_else(|| DdsError("DDS cumulative data size overflows u32".into()))?;
            previous_width = level_width;
            previous_height = level_height;
        }

        Ok(Self {
            format,
            bits_per_pixel,
            width,
            height,
            mip_count,
            is_cube,
            cube_face_mask,
            cube_face_count,
            cube_face_indices,
            palette_offset,
            levels,
            pixel_format_flags,
        })
    }

    pub fn required_data_len(&self) -> Result<usize, DdsError> {
        let Some(last) = self.levels.last().copied() else {
            return Ok(if self.palette_offset.is_some() {
                DDS_HEADER_SIZE + DDS_PALETTE_SIZE
            } else {
                DDS_HEADER_SIZE
            });
        };
        Ok(last.data_range()?.end)
    }

    pub fn validate_data_len(&self, bytes: &[u8]) -> Result<(), DdsError> {
        let required = self.required_data_len()?;
        if bytes.len() < required {
            return Err(DdsError(format!(
                "DDS surface data requires {required} bytes, got {}",
                bytes.len()
            )));
        }
        Ok(())
    }

    pub fn creation_plan(&self, policy: DdsLoadPolicy) -> D3d9TextureCreation {
        let direct_compatible = !self.format.is_block_compressed()
            || (game_power_of_two(self.width) && game_power_of_two(self.height));
        let use_d3dx = !direct_compatible || policy.device_special_mode || policy.force_d3dx;
        if use_d3dx {
            if self.is_cube {
                D3d9TextureCreation::D3dxCube {
                    edge_length: u32::MAX,
                    mip_levels: self.mip_count,
                    usage: 0,
                    format: D3d9Format::UNKNOWN,
                    pool: 0,
                    filter: u32::MAX,
                    mip_filter: u32::MAX,
                    color_key: 0,
                }
            } else {
                D3d9TextureCreation::D3dx2d {
                    width: self.width,
                    height: self.height,
                    mip_levels: self.mip_count,
                    usage: 0,
                    format: D3d9Format::UNKNOWN,
                    pool: 0,
                    filter: u32::MAX,
                    mip_filter: u32::MAX,
                    color_key: 0,
                    palette_offset: (self.pixel_format_flags & 0x1838 != 0)
                        .then_some(DDS_HEADER_SIZE as u32),
                }
            }
        } else {
            let format = self.direct_creation_format().d3d9_format();
            let mip_levels = self.direct_mip_count();
            if self.is_cube {
                D3d9TextureCreation::DirectCube {
                    edge_length: self.width,
                    mip_levels,
                    usage: 0,
                    format,
                    pool: 0,
                }
            } else {
                D3d9TextureCreation::Direct2d {
                    width: self.width,
                    height: self.height,
                    mip_levels,
                    usage: 0,
                    format,
                    pool: 0,
                }
            }
        }
    }

    pub fn direct_2d_upload_plan(&self) -> Result<Vec<D3d9Direct2dUpload>, DdsError> {
        if self.is_cube {
            return Err(DdsError(
                "the SRD DDS cube upload path is not yet evidence-closed".into(),
            ));
        }
        if self.format != self.direct_creation_format() {
            return Err(DdsError(format!(
                "DDS internal format {} is converted to {} before direct creation; its pixel conversion path is not yet evidence-closed",
                self.format.0,
                self.direct_creation_format().0
            )));
        }

        let mip_count = self.direct_mip_count();
        let capacity = usize::try_from(mip_count)
            .map_err(|_| DdsError("DDS direct mip count does not fit usize".into()))?;
        let mut uploads = Vec::with_capacity(capacity);
        for level in self.levels.iter().take(capacity) {
            if self.format.is_block_compressed() && (level.width < 4 || level.height < 4) {
                uploads.push(D3d9Direct2dUpload::CompressedLevelBelowFourSkipped {
                    mip_level: level.mip_index,
                    width: level.width,
                    height: level.height,
                });
                continue;
            }

            let base_row_pitch = level
                .width
                .checked_mul(self.bits_per_pixel)
                .ok_or_else(|| DdsError("DDS upload row bit count overflows u32".into()))?
                >> 3;
            let (source_row_pitch, row_count) = if self.format.is_block_compressed() {
                (
                    base_row_pitch.checked_mul(4).ok_or_else(|| {
                        DdsError("DDS compressed upload row pitch overflows u32".into())
                    })?,
                    level.height >> 2,
                )
            } else {
                (base_row_pitch, level.height)
            };
            let copied_byte_len = source_row_pitch
                .checked_mul(row_count)
                .ok_or_else(|| DdsError("DDS upload byte count overflows u32".into()))?;
            if copied_byte_len != level.byte_len {
                return Err(DdsError(format!(
                    "DDS mip {} staging copy length {} differs from parsed surface length {}",
                    level.mip_index, copied_byte_len, level.byte_len
                )));
            }
            uploads.push(D3d9Direct2dUpload::UpdateSurface(D3d9SystemMemoryUpload {
                destination_mip_level: level.mip_index,
                source_offset: level.data_offset,
                source_byte_len: copied_byte_len,
                width: level.width,
                height: level.height,
                source_row_pitch,
                row_count,
                staging_mip_levels: 1,
                staging_usage: 0,
                staging_format: self.direct_creation_format().d3d9_format(),
                staging_pool: 2,
                staging_lock_flags: 0,
                source_rect: [0, 0, level.width, level.height],
                destination_point: [0, 0],
            }));
        }
        Ok(uploads)
    }

    /// Decodes the complete 2D mip chain through the independent `image_dds`
    /// library. This is an editor compatibility path for files whose exact
    /// game branch uses D3DX; it is not evidence for Surfride semantics.
    pub fn decode_rgba8_levels_with_library(
        &self,
        bytes: &[u8],
    ) -> Result<Vec<DecodedDdsRgba8Level>, DdsError> {
        self.validate_data_len(bytes)?;
        if self.is_cube || self.cube_face_count != 1 {
            return Err(DdsError(
                "the independent decoder path currently accepts only 2D DDS".into(),
            ));
        }
        let image_format = match self.format {
            GameTextureFormat::A4_R4_G4_B4 => image_dds::ImageFormat::Bgra4Unorm,
            GameTextureFormat::A8_R8_G8_B8 => image_dds::ImageFormat::Bgra8Unorm,
            GameTextureFormat::R8_G8_B8 => image_dds::ImageFormat::Bgr8Unorm,
            GameTextureFormat::A8_B8_G8_R8 => image_dds::ImageFormat::Rgba8Unorm,
            GameTextureFormat::DXT1 => image_dds::ImageFormat::BC1RgbaUnorm,
            GameTextureFormat::DXT2 | GameTextureFormat::DXT3 => {
                image_dds::ImageFormat::BC2RgbaUnorm
            }
            GameTextureFormat::DXT4 | GameTextureFormat::DXT5 => {
                image_dds::ImageFormat::BC3RgbaUnorm
            }
            GameTextureFormat::R16F => image_dds::ImageFormat::R16Float,
            GameTextureFormat::G16_R16F => image_dds::ImageFormat::Rg16Float,
            GameTextureFormat::A16_B16_G16_R16F => image_dds::ImageFormat::Rgba16Float,
            GameTextureFormat::R32F => image_dds::ImageFormat::R32Float,
            GameTextureFormat::G32_R32F => image_dds::ImageFormat::Rg32Float,
            GameTextureFormat::A32_B32_G32_R32F => image_dds::ImageFormat::Rgba32Float,
            format => {
                return Err(DdsError(format!(
                    "independent DDS decoder has no proven mapping for game format {}",
                    format.0
                )));
            }
        };
        let payload_start = self
            .levels
            .first()
            .ok_or_else(|| DdsError("DDS has no surface levels".into()))?
            .data_range()?
            .start;
        let payload_end = self.required_data_len()?;
        let surface = image_dds::Surface {
            width: self.width,
            height: self.height,
            depth: 1,
            layers: 1,
            mipmaps: self.mip_count,
            image_format,
            data: &bytes[payload_start..payload_end],
        };
        let mut levels = Vec::with_capacity(self.mip_count as usize);
        for mip_index in 0..self.mip_count {
            let decoded = surface
                .decode_layers_mipmaps_rgba8(0..1, mip_index..mip_index + 1)
                .map_err(|error| {
                    DdsError(format!(
                        "image_dds failed to decode mip {mip_index}: {error}"
                    ))
                })?;
            let expected_len = decoded
                .width
                .checked_mul(decoded.height)
                .and_then(|pixels| pixels.checked_mul(4))
                .and_then(|bytes| usize::try_from(bytes).ok())
                .ok_or_else(|| DdsError("decoded RGBA mip length overflows usize".into()))?;
            if decoded.data.len() != expected_len {
                return Err(DdsError(format!(
                    "image_dds mip {mip_index} returned {} bytes, expected {expected_len}",
                    decoded.data.len()
                )));
            }
            levels.push(DecodedDdsRgba8Level {
                mip_index,
                width: decoded.width,
                height: decoded.height,
                rgba: decoded.data,
            });
        }
        Ok(levels)
    }

    fn direct_creation_format(&self) -> GameTextureFormat {
        self.format.direct_creation_format()
    }

    fn direct_mip_count(&self) -> u32 {
        if !self.format.is_block_compressed() || self.mip_count <= 1 {
            return self.mip_count;
        }
        let mut width = self.width;
        let mut height = self.height;
        let mut accepted = 1u32;
        while accepted < self.mip_count {
            width >>= 1;
            height >>= 1;
            if width < 4 || (!self.is_cube && height < 4) {
                break;
            }
            accepted += 1;
        }
        accepted
    }
}

fn parse_game_format(
    flags: u32,
    fourcc: u32,
    bit_count: u32,
    mask_at_100: u32,
) -> Result<(GameTextureFormat, u32), DdsError> {
    if flags & DDPF_RGB != 0 {
        let format = match bit_count {
            16 => {
                if flags & DDPF_ALPHAPIXELS != 0 {
                    if mask_at_100 == 31 {
                        GameTextureFormat::A1_R5_G5_B5
                    } else {
                        GameTextureFormat::A4_R4_G4_B4
                    }
                } else {
                    GameTextureFormat::R5_G6_B5
                }
            }
            24 => GameTextureFormat::R8_G8_B8,
            32 => {
                if flags & DDPF_ALPHAPIXELS != 0 {
                    GameTextureFormat::A8_R8_G8_B8
                } else if mask_at_100 == 255 {
                    GameTextureFormat::X8_R8_G8_B8
                } else {
                    GameTextureFormat::A8_B8_G8_R8
                }
            }
            _ => return Err(unsupported_format(flags, fourcc, bit_count, mask_at_100)),
        };
        return Ok((format, bit_count));
    }

    if flags & DDPF_FOURCC != 0 {
        let result = match fourcc {
            FOURCC_DXT1 => (GameTextureFormat::DXT1, 4),
            FOURCC_DXT2 => (GameTextureFormat::DXT2, 8),
            FOURCC_DXT3 => (GameTextureFormat::DXT3, 8),
            FOURCC_DXT4 => (GameTextureFormat::DXT4, 8),
            FOURCC_DXT5 => (GameTextureFormat::DXT5, 8),
            111 => (GameTextureFormat::R16F, 16),
            112 => (GameTextureFormat::G16_R16F, 32),
            113 => (GameTextureFormat::A16_B16_G16_R16F, 64),
            114 => (GameTextureFormat::R32F, 32),
            115 => (GameTextureFormat::G32_R32F, 64),
            116 => (GameTextureFormat::A32_B32_G32_R32F, 128),
            _ => return Err(unsupported_format(flags, fourcc, bit_count, mask_at_100)),
        };
        return Ok(result);
    }

    if flags & DDS_PIXEL_FORMAT_FLAG_0X20 != 0 {
        let format = match bit_count {
            8 => {
                if flags & DDPF_ALPHAPIXELS != 0 {
                    GameTextureFormat::EMBEDDED_PALETTE_8_ALPHA
                } else {
                    GameTextureFormat::EMBEDDED_PALETTE_8
                }
            }
            16 => GameTextureFormat::EMBEDDED_PALETTE_16,
            _ => return Err(unsupported_format(flags, fourcc, bit_count, mask_at_100)),
        };
        return Ok((format, bit_count));
    }

    Err(unsupported_format(flags, fourcc, bit_count, mask_at_100))
}

fn unsupported_format(flags: u32, fourcc: u32, bit_count: u32, mask: u32) -> DdsError {
    DdsError(format!(
        "DDS pixel format is unsupported by the game parser: flags={flags:#010x}, fourcc={fourcc:#010x}, bit_count={bit_count}, mask@100={mask:#010x}"
    ))
}

fn round_up_to_four(value: u32) -> Result<u32, DdsError> {
    let remainder = value & 3;
    if remainder == 0 {
        Ok(value)
    } else {
        value
            .checked_add(4 - remainder)
            .ok_or_else(|| DdsError("DDS padded dimension overflows u32".into()))
    }
}

const fn game_power_of_two(value: u32) -> bool {
    (value.wrapping_sub(1) & value) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    struct HeaderFields {
        width: u32,
        height: u32,
        mip_count: u32,
        pixel_flags: u32,
        fourcc: u32,
        bit_count: u32,
        mask_at_100: u32,
        caps2: u32,
    }

    fn dds_header(fields: HeaderFields) -> Vec<u8> {
        let mut bytes = vec![0u8; DDS_HEADER_SIZE];
        let mut write = |offset: usize, value: u32| {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes())
        };
        write(0, DDS_MAGIC);
        write(12, fields.height);
        write(16, fields.width);
        write(28, fields.mip_count);
        write(80, fields.pixel_flags);
        write(84, fields.fourcc);
        write(88, fields.bit_count);
        write(100, fields.mask_at_100);
        write(112, fields.caps2);
        bytes
    }

    #[test]
    fn d3d9_format_mapping_matches_the_backend_switch() {
        let expected = [
            (0, 23),
            (1, 21),
            (2, 22),
            (3, 20),
            (4, 26),
            (5, 25),
            (6, 35),
            (16, 34),
            (32, 51),
            (35, 51),
            (33, 50),
            (34, 28),
            (48, FOURCC_DXT1),
            (49, FOURCC_DXT2),
            (50, FOURCC_DXT3),
            (51, FOURCC_DXT4),
            (52, FOURCC_DXT5),
            (64, 111),
            (65, 112),
            (66, 113),
            (80, 114),
            (81, 115),
            (82, 116),
            (96, 80),
            (97, 77),
            (98, 75),
            (99, 71),
            (100, u32::from_le_bytes(*b"INTZ")),
            (101, u32::from_le_bytes(*b"RAWZ")),
        ];
        for (format, d3d9) in expected {
            assert_eq!(game_texture_format_to_d3d9(format), d3d9);
        }
        assert_eq!(game_texture_format_to_d3d9(7), 0);
        assert_eq!(game_texture_format_to_d3d9(102), 0);
    }

    #[test]
    fn dxt_levels_use_game_four_by_four_padding_and_face_major_order() {
        let mut bytes = dds_header(HeaderFields {
            width: 8,
            height: 8,
            mip_count: 3,
            pixel_flags: DDPF_FOURCC,
            fourcc: FOURCC_DXT5,
            bit_count: 0,
            mask_at_100: 0,
            caps2: DDSCAPS2_CUBEMAP | DDSCAPS2_CUBEMAP_FACES[0] | DDSCAPS2_CUBEMAP_FACES[2],
        });
        bytes.resize(128 + 2 * (64 + 16 + 16), 0);
        let descriptor = DdsDescriptor::parse(&bytes).unwrap();
        descriptor.validate_data_len(&bytes).unwrap();
        assert_eq!(descriptor.format, GameTextureFormat::DXT5);
        assert_eq!(descriptor.cube_face_mask, 0b000101);
        assert_eq!(descriptor.cube_face_count, 2);
        assert_eq!(descriptor.cube_face_indices, [0, 0, 1, 0, 0, 0]);
        assert_eq!(
            descriptor.levels,
            vec![
                DdsSurfaceLevel {
                    face_index: 0,
                    mip_index: 0,
                    data_offset: 128,
                    width: 8,
                    height: 8,
                    byte_len: 64,
                },
                DdsSurfaceLevel {
                    face_index: 0,
                    mip_index: 1,
                    data_offset: 192,
                    width: 4,
                    height: 4,
                    byte_len: 16,
                },
                DdsSurfaceLevel {
                    face_index: 0,
                    mip_index: 2,
                    data_offset: 208,
                    width: 2,
                    height: 2,
                    byte_len: 16,
                },
                DdsSurfaceLevel {
                    face_index: 1,
                    mip_index: 0,
                    data_offset: 224,
                    width: 8,
                    height: 8,
                    byte_len: 64,
                },
                DdsSurfaceLevel {
                    face_index: 1,
                    mip_index: 1,
                    data_offset: 288,
                    width: 4,
                    height: 4,
                    byte_len: 16,
                },
                DdsSurfaceLevel {
                    face_index: 1,
                    mip_index: 2,
                    data_offset: 304,
                    width: 2,
                    height: 2,
                    byte_len: 16,
                },
            ]
        );
    }

    #[test]
    fn direct_and_d3dx_plans_match_the_binary_branch_and_arguments() {
        let rgba = DdsDescriptor::parse(&dds_header(HeaderFields {
            width: 720,
            height: 266,
            mip_count: 10,
            pixel_flags: DDPF_RGB | DDPF_ALPHAPIXELS,
            fourcc: 0,
            bit_count: 32,
            mask_at_100: 0,
            caps2: 0,
        }))
        .unwrap();
        assert_eq!(
            rgba.creation_plan(DdsLoadPolicy::default()),
            D3d9TextureCreation::Direct2d {
                width: 720,
                height: 266,
                mip_levels: 10,
                usage: 0,
                format: D3d9Format(21),
                pool: 0,
            }
        );

        let dxt5 = DdsDescriptor::parse(&dds_header(HeaderFields {
            width: 1152,
            height: 1024,
            mip_count: 4,
            pixel_flags: DDPF_FOURCC,
            fourcc: FOURCC_DXT5,
            bit_count: 0,
            mask_at_100: 0,
            caps2: 0,
        }))
        .unwrap();
        assert_eq!(
            dxt5.creation_plan(DdsLoadPolicy::default()),
            D3d9TextureCreation::D3dx2d {
                width: 1152,
                height: 1024,
                mip_levels: 4,
                usage: 0,
                format: D3d9Format::UNKNOWN,
                pool: 0,
                filter: u32::MAX,
                mip_filter: u32::MAX,
                color_key: 0,
                palette_offset: None,
            }
        );

        let power_of_two = DdsDescriptor::parse(&dds_header(HeaderFields {
            width: 16,
            height: 16,
            mip_count: 8,
            pixel_flags: DDPF_FOURCC,
            fourcc: FOURCC_DXT1,
            bit_count: 0,
            mask_at_100: 0,
            caps2: 0,
        }))
        .unwrap();
        assert_eq!(
            power_of_two.creation_plan(DdsLoadPolicy::default()),
            D3d9TextureCreation::Direct2d {
                width: 16,
                height: 16,
                mip_levels: 3,
                usage: 0,
                format: D3d9Format(FOURCC_DXT1),
                pool: 0,
            }
        );
        assert!(matches!(
            power_of_two.creation_plan(DdsLoadPolicy {
                device_special_mode: true,
                force_d3dx: false,
            }),
            D3d9TextureCreation::D3dx2d { .. }
        ));
    }

    #[test]
    fn palette_formats_skip_the_exact_1024_byte_table() {
        let mut bytes = dds_header(HeaderFields {
            width: 2,
            height: 2,
            mip_count: 1,
            pixel_flags: DDS_PIXEL_FORMAT_FLAG_0X20,
            fourcc: 0,
            bit_count: 8,
            mask_at_100: 0,
            caps2: 0,
        });
        bytes.resize(1152 + 4, 0);
        let descriptor = DdsDescriptor::parse(&bytes).unwrap();
        assert_eq!(descriptor.format, GameTextureFormat::EMBEDDED_PALETTE_8);
        assert_eq!(descriptor.palette_offset, Some(128));
        assert_eq!(descriptor.levels[0].data_offset, 1152);
        assert_eq!(descriptor.levels[0].byte_len, 4);
        descriptor.validate_data_len(&bytes).unwrap();
    }

    #[test]
    fn direct_2d_uploads_use_system_memory_staging_and_update_surface_geometry() {
        let rgba = DdsDescriptor::parse(&dds_header(HeaderFields {
            width: 4,
            height: 2,
            mip_count: 2,
            pixel_flags: DDPF_RGB | DDPF_ALPHAPIXELS,
            fourcc: 0,
            bit_count: 32,
            mask_at_100: 0,
            caps2: 0,
        }))
        .unwrap();
        assert_eq!(
            rgba.direct_2d_upload_plan().unwrap(),
            vec![
                D3d9Direct2dUpload::UpdateSurface(D3d9SystemMemoryUpload {
                    destination_mip_level: 0,
                    source_offset: 128,
                    source_byte_len: 32,
                    width: 4,
                    height: 2,
                    source_row_pitch: 16,
                    row_count: 2,
                    staging_mip_levels: 1,
                    staging_usage: 0,
                    staging_format: D3d9Format(21),
                    staging_pool: 2,
                    staging_lock_flags: 0,
                    source_rect: [0, 0, 4, 2],
                    destination_point: [0, 0],
                }),
                D3d9Direct2dUpload::UpdateSurface(D3d9SystemMemoryUpload {
                    destination_mip_level: 1,
                    source_offset: 160,
                    source_byte_len: 8,
                    width: 2,
                    height: 1,
                    source_row_pitch: 8,
                    row_count: 1,
                    staging_mip_levels: 1,
                    staging_usage: 0,
                    staging_format: D3d9Format(21),
                    staging_pool: 2,
                    staging_lock_flags: 0,
                    source_rect: [0, 0, 2, 1],
                    destination_point: [0, 0],
                }),
            ]
        );

        let dxt5 = DdsDescriptor::parse(&dds_header(HeaderFields {
            width: 8,
            height: 8,
            mip_count: 3,
            pixel_flags: DDPF_FOURCC,
            fourcc: FOURCC_DXT5,
            bit_count: 0,
            mask_at_100: 0,
            caps2: 0,
        }))
        .unwrap();
        assert_eq!(
            dxt5.direct_2d_upload_plan().unwrap(),
            vec![
                D3d9Direct2dUpload::UpdateSurface(D3d9SystemMemoryUpload {
                    destination_mip_level: 0,
                    source_offset: 128,
                    source_byte_len: 64,
                    width: 8,
                    height: 8,
                    source_row_pitch: 32,
                    row_count: 2,
                    staging_mip_levels: 1,
                    staging_usage: 0,
                    staging_format: D3d9Format(FOURCC_DXT5),
                    staging_pool: 2,
                    staging_lock_flags: 0,
                    source_rect: [0, 0, 8, 8],
                    destination_point: [0, 0],
                }),
                D3d9Direct2dUpload::UpdateSurface(D3d9SystemMemoryUpload {
                    destination_mip_level: 1,
                    source_offset: 192,
                    source_byte_len: 16,
                    width: 4,
                    height: 4,
                    source_row_pitch: 16,
                    row_count: 1,
                    staging_mip_levels: 1,
                    staging_usage: 0,
                    staging_format: D3d9Format(FOURCC_DXT5),
                    staging_pool: 2,
                    staging_lock_flags: 0,
                    source_rect: [0, 0, 4, 4],
                    destination_point: [0, 0],
                }),
            ]
        );
    }

    #[test]
    fn compressed_base_levels_below_four_skip_the_staging_upload() {
        let descriptor = DdsDescriptor::parse(&dds_header(HeaderFields {
            width: 2,
            height: 2,
            mip_count: 1,
            pixel_flags: DDPF_FOURCC,
            fourcc: FOURCC_DXT1,
            bit_count: 0,
            mask_at_100: 0,
            caps2: 0,
        }))
        .unwrap();
        assert_eq!(
            descriptor.direct_2d_upload_plan().unwrap(),
            vec![D3d9Direct2dUpload::CompressedLevelBelowFourSkipped {
                mip_level: 0,
                width: 2,
                height: 2,
            }]
        );
    }
}
