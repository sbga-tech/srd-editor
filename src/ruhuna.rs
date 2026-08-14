use std::fmt;
use std::ops::Range;

use crate::avts::{AvtsError, AvtsFile};
use crate::dds::{DdsDescriptor, DdsError};
use crate::texture::{TextureAddressMode, TextureFilter, TextureSamplerState};
use crate::yabx::{RfzYabxError, YabxFile, YabxObject};

const DATABASE_CLASS: &[u8] = b"ruhuna::Database";
const GLYPH_CLASS: &[u8] = b"ruhuna::Glyph";
const TEXTURE_RESOURCE_CLASS: &[u8] = b"ruhuna::TextureResource";
const STEVIA_TEXTURE_CLASS: &[u8] = b"stevia::Texture";
const SERIALIZED_OBJECT_ID_BASE: i32 = 10_001;

#[derive(Debug)]
pub enum RuhunaError {
    Container(RfzYabxError),
    Avts(AvtsError),
    Dds(DdsError),
    Invalid(String),
}

impl fmt::Display for RuhunaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Container(error) => error.fmt(formatter),
            Self::Avts(error) => error.fmt(formatter),
            Self::Dds(error) => error.fmt(formatter),
            Self::Invalid(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for RuhunaError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuhunaDatabase {
    pub object_index: usize,
    pub id: Vec<u8>,
    pub platform: Vec<u8>,
    pub library: Vec<u8>,
    pub name: Vec<u8>,
    pub comment: Vec<u8>,
    pub flags: u32,
    pub point: u16,
    pub max_ascent: u16,
    pub max_descent: u16,
    pub max_glyph_width: u16,
    pub max_glyph_height: u16,
    pub texture_page_count: u32,
    pub texture_width: u16,
    pub texture_height: u16,
    pub texture_last_height: u16,
    pub glyph_margin: u16,
    pub glyph_count: u16,
    pub glyph_object_indices: Vec<usize>,
    pub texture_object_indices: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuhunaGlyph {
    pub object_index: usize,
    pub code: u16,
    pub cell_increment_x: u16,
    pub cell_increment_y: u16,
    pub page: u16,
    pub origin_x: i16,
    pub origin_y: i16,
    pub box_x1: u16,
    pub box_y1: u16,
    pub box_x2: u16,
    pub box_y2: u16,
    pub kerning_info_count: u16,
}

/// Host-independent representation of the game's 128-byte x86 runtime glyph
/// record. Pointer fields are kept as caller-supplied 32-bit tokens; the editor
/// never casts them to host pointers.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuhunaRuntimeGlyphRecord {
    pub owner_token: u32,
    pub font_slot_id: u16,
    pub code: u16,
    pub field_08: u32,
    pub point_x: u16,
    pub point_y: u16,
    pub atlas_x: u32,
    pub atlas_y: u32,
    pub texture_token: u32,
    pub inverse_texture_width: f32,
    pub inverse_texture_height: f32,
    pub enabled: u32,
    pub bearing_x: i32,
    pub bearing_y: i32,
    pub width: u32,
    pub height: u32,
    pub line_height: u32,
    pub advance_x: u32,
    pub advance_y: u32,
    pub em_pixels_x: i32,
    pub em_pixels_y: i32,
    pub field_4c: i32,
    pub flag_mode: u32,
    pub field_54: u32,
    pub field_58: i16,
    pub field_5a: u16,
    pub uv0: [f32; 2],
    pub uv1: [f32; 2],
    pub uv2: [f32; 2],
    pub uv3: [f32; 2],
    pub rotated: u16,
    pub field_7e: u16,
}

impl Default for RuhunaRuntimeGlyphRecord {
    fn default() -> Self {
        Self {
            owner_token: 0,
            font_slot_id: 0,
            code: 0,
            field_08: 0,
            point_x: 0,
            point_y: 0,
            atlas_x: 0,
            atlas_y: 0,
            texture_token: 0,
            inverse_texture_width: 0.0,
            inverse_texture_height: 0.0,
            enabled: 0,
            bearing_x: 0,
            bearing_y: 0,
            width: 0,
            height: 0,
            line_height: 0,
            advance_x: 0,
            advance_y: 0,
            em_pixels_x: 0,
            em_pixels_y: 0,
            field_4c: 0,
            flag_mode: 0,
            field_54: 0,
            field_58: 0,
            field_5a: 0,
            uv0: [0.0; 2],
            uv1: [0.0; 2],
            uv2: [0.0; 2],
            uv3: [0.0; 2],
            rotated: 0,
            field_7e: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuhunaRuntimeFont {
    pub minimum_code: u16,
    pub maximum_code: u16,
    pub dense_glyph_indices: Vec<u16>,
    pub glyph_pages: Vec<Option<u16>>,
    pub glyphs: Vec<RuhunaRuntimeGlyphRecord>,
}

impl RuhunaRuntimeFont {
    /// Returns the exact dense-table entry built by `sub_F47700` for an in-range
    /// code. Zero is a valid entry and is also what the game leaves in holes.
    pub fn dense_glyph_index(&self, code: u16) -> Option<u16> {
        if code < self.minimum_code || code > self.maximum_code {
            return None;
        }
        self.dense_glyph_indices
            .get(usize::from(code - self.minimum_code))
            .copied()
    }

    /// Reproduces `sub_F41C50`: range-check, read the dense index, address the
    /// 128-byte record, then reject a zero-filled hole unless its code matches.
    pub fn glyph(&self, code: u16) -> Option<&RuhunaRuntimeGlyphRecord> {
        let glyph_index = usize::from(self.dense_glyph_index(code)?);
        self.glyphs
            .get(glyph_index)
            .filter(|glyph| glyph.code == code)
    }

    /// Reproduces the successful tail of `sub_F323B0`: call the resource's
    /// checked getter, then store the global Fennel font slot ID at glyph
    /// offset `+0x04` before returning the same runtime record.
    pub fn glyph_for_font_slot(
        &mut self,
        font_slot_id: u16,
        code: u16,
    ) -> Option<&mut RuhunaRuntimeGlyphRecord> {
        let glyph_index = usize::from(self.dense_glyph_index(code)?);
        let glyph = self
            .glyphs
            .get_mut(glyph_index)
            .filter(|glyph| glyph.code == code)?;
        glyph.font_slot_id = font_slot_id;
        Some(glyph)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuhunaTextureResource {
    pub object_index: usize,
    file_data: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuhunaAtlasPage {
    pub page_index: usize,
    pub entry_name: Vec<u8>,
    pub descriptor: DdsDescriptor,
    data: Range<usize>,
}

/// Exact sampler values carried by the `stevia::Texture` records inside the
/// font AVTS metadata and by the matching Ceylon texture defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuhunaSamplerState {
    pub base: TextureSamplerState,
    pub mip_filter: TextureFilter,
    pub max_mip_level: u32,
    pub max_anisotropy: u32,
    pub mip_lod_bias_bits: u32,
    pub border_color: u32,
}

#[derive(Debug)]
pub struct RuhunaFont {
    yabx: YabxFile,
    pub database: RuhunaDatabase,
    pub glyphs: Vec<RuhunaGlyph>,
    pub textures: Vec<RuhunaTextureResource>,
}

impl RuhunaFont {
    pub fn from_rfz(bytes: &[u8]) -> Result<Self, RuhunaError> {
        let yabx = YabxFile::from_rfz(bytes).map_err(RuhunaError::Container)?;
        Self::from_yabx(yabx)
    }

    pub fn from_yabx(yabx: YabxFile) -> Result<Self, RuhunaError> {
        let database_objects = objects_of_class(&yabx, DATABASE_CLASS);
        if database_objects.len() != 1 {
            return Err(RuhunaError::Invalid(format!(
                "ruhuna font needs exactly one Database object, found {}",
                database_objects.len()
            )));
        }
        let (database_index, database_object) = database_objects[0];
        let database = parse_database(&yabx, database_index, database_object)?;

        let glyphs = objects_of_class(&yabx, GLYPH_CLASS)
            .into_iter()
            .map(|(index, object)| parse_glyph(&yabx, index, object))
            .collect::<Result<Vec<_>, _>>()?;
        let textures = objects_of_class(&yabx, TEXTURE_RESOURCE_CLASS)
            .into_iter()
            .map(|(index, object)| parse_texture_resource(&yabx, index, object))
            .collect::<Result<Vec<_>, _>>()?;

        if usize::from(database.glyph_count) != database.glyph_object_indices.len() {
            return Err(RuhunaError::Invalid(format!(
                "Database glyph_count {} does not match {} serialized glyph references",
                database.glyph_count,
                database.glyph_object_indices.len()
            )));
        }
        validate_referenced_classes(&yabx, &database.glyph_object_indices, GLYPH_CLASS, "glyph")?;
        validate_referenced_classes(
            &yabx,
            &database.texture_object_indices,
            TEXTURE_RESOURCE_CLASS,
            "texture",
        )?;

        Ok(Self {
            yabx,
            database,
            glyphs,
            textures,
        })
    }

    pub fn yabx(&self) -> &YabxFile {
        &self.yabx
    }

    pub fn texture_file_bytes(&self, texture: &RuhunaTextureResource) -> &[u8] {
        &self.yabx.bytes()[texture.file_data.clone()]
    }

    pub fn texture_avts<'a>(
        &'a self,
        texture: &RuhunaTextureResource,
    ) -> Result<AvtsFile<'a>, AvtsError> {
        AvtsFile::parse(self.texture_file_bytes(texture))
    }

    pub fn atlas_pages(
        &self,
        texture: &RuhunaTextureResource,
    ) -> Result<Vec<RuhunaAtlasPage>, RuhunaError> {
        let avts = self.texture_avts(texture).map_err(RuhunaError::Avts)?;
        let metadata = avts
            .entries
            .first()
            .ok_or_else(|| RuhunaError::Invalid("font AVTS has no metadata entry".into()))?;
        if metadata.field_200 != 0
            || metadata.field_204 != 0
            || !avts.entry_data(metadata).starts_with(b"YABX")
        {
            return Err(RuhunaError::Invalid(
                "font AVTS entry 0 is not the binary-proven YABX metadata entry".into(),
            ));
        }

        let mut pages = Vec::new();
        for (entry_index, entry) in avts.entries.iter().enumerate().skip(1) {
            let page_index = entry_index - 1;
            if entry.field_200 != 1 || entry.field_204 != entry_index as u32 {
                return Err(RuhunaError::Invalid(format!(
                    "font AVTS entry {entry_index} has unexpected fields {:#x}/{:#x}",
                    entry.field_200, entry.field_204
                )));
            }
            let data = avts.entry_data(entry);
            let descriptor = DdsDescriptor::parse(data).map_err(RuhunaError::Dds)?;
            descriptor
                .validate_data_len(data)
                .map_err(RuhunaError::Dds)?;
            if descriptor.width != u32::from(self.database.texture_width) {
                return Err(RuhunaError::Invalid(format!(
                    "font atlas page {page_index} width {} does not match Database tex_w {}",
                    descriptor.width, self.database.texture_width
                )));
            }
            let expected_height = if entry_index + 1 == avts.entries.len() {
                self.database.texture_last_height
            } else {
                self.database.texture_height
            };
            if descriptor.height != u32::from(expected_height) {
                return Err(RuhunaError::Invalid(format!(
                    "font atlas page {page_index} height {} does not match expected {expected_height}",
                    descriptor.height
                )));
            }
            pages.push(RuhunaAtlasPage {
                page_index,
                entry_name: entry.name.clone(),
                descriptor,
                data: (texture.file_data.start + entry.data.start)
                    ..(texture.file_data.start + entry.data.end),
            });
        }
        if pages.len() != self.database.texture_page_count as usize {
            return Err(RuhunaError::Invalid(format!(
                "font AVTS has {} DDS pages but Database tex_page is {}",
                pages.len(),
                self.database.texture_page_count
            )));
        }
        Ok(pages)
    }

    /// Resolves the sampler serialized beside the DDS atlas pages.
    ///
    /// Every page has one `stevia::Texture` metadata object. Page-to-object
    /// ordering is immaterial here because this method requires every object
    /// to carry the same seven fixed sampler fields before returning one
    /// shared state. A differing archive is rejected instead of guessed.
    pub fn atlas_sampler_state(
        &self,
        texture: &RuhunaTextureResource,
    ) -> Result<RuhunaSamplerState, RuhunaError> {
        let avts = self.texture_avts(texture).map_err(RuhunaError::Avts)?;
        let metadata_entry = avts
            .entries
            .first()
            .ok_or_else(|| RuhunaError::Invalid("font AVTS has no metadata entry".into()))?;
        let metadata_bytes = avts.entry_data(metadata_entry);
        let metadata = YabxFile::parse(metadata_bytes.to_vec())
            .map_err(|error| RuhunaError::Invalid(error.to_string()))?;
        let (class_index, class) = metadata
            .classes
            .iter()
            .enumerate()
            .find(|(_, class)| class.name == STEVIA_TEXTURE_CLASS)
            .ok_or_else(|| {
                RuhunaError::Invalid("font AVTS metadata has no stevia::Texture class".into())
            })?;
        let expected_fields: [(&[u8], i16); 7] = [
            (b"_wrapU", 4),
            (b"_wrapV", 4),
            (b"_minFilter", 4),
            (b"_magFilter", 4),
            (b"_mipFilter", 4),
            (b"_anisoNumber", 4),
            (b"_lodBias", 4),
        ];
        if class.fields.len() < expected_fields.len()
            || !class
                .fields
                .iter()
                .zip(expected_fields)
                .all(|(actual, expected)| {
                    actual.name == expected.0 && actual.storage_size == expected.1
                })
        {
            return Err(RuhunaError::Invalid(
                "stevia::Texture fixed sampler field prefix does not match the binary-proven layout"
                    .into(),
            ));
        }

        let serialized_class_index = i16::try_from(class_index + 1).map_err(|_| {
            RuhunaError::Invalid("stevia::Texture class index does not fit i16".into())
        })?;
        let texture_objects = metadata
            .objects
            .iter()
            .filter(|object| object.class_index == serialized_class_index)
            .collect::<Vec<_>>();
        if texture_objects.len() != self.database.texture_page_count as usize {
            return Err(RuhunaError::Invalid(format!(
                "font AVTS metadata has {} stevia::Texture objects for {} atlas pages",
                texture_objects.len(),
                self.database.texture_page_count
            )));
        }

        let mut shared_fields = None;
        for object in texture_objects {
            let data = metadata.object_data(object);
            let prefix = data.get(..28).ok_or_else(|| {
                RuhunaError::Invalid("stevia::Texture sampler prefix is truncated".into())
            })?;
            let fields = std::array::from_fn(|index| {
                u32::from_le_bytes(prefix[index * 4..index * 4 + 4].try_into().unwrap())
            });
            match shared_fields {
                Some(previous) if previous != fields => {
                    return Err(RuhunaError::Invalid(
                        "font atlas pages use differing stevia::Texture sampler fields".into(),
                    ));
                }
                None => shared_fields = Some(fields),
                _ => {}
            }
        }
        let fields = shared_fields.ok_or_else(|| {
            RuhunaError::Invalid("font AVTS metadata contains no texture sampler".into())
        })?;
        if fields != [2, 2, 1, 1, 0, 1, 0] {
            return Err(RuhunaError::Invalid(format!(
                "unsupported stevia::Texture sampler fields {fields:?}"
            )));
        }

        let pages = self.atlas_pages(texture)?;
        let max_mip_level = pages
            .first()
            .map(|page| page.descriptor.mip_count.max(1))
            .ok_or_else(|| RuhunaError::Invalid("font atlas contains no DDS pages".into()))?;
        if pages
            .iter()
            .any(|page| page.descriptor.mip_count.max(1) != max_mip_level)
        {
            return Err(RuhunaError::Invalid(
                "font atlas pages use differing mip counts".into(),
            ));
        }

        Ok(RuhunaSamplerState {
            base: TextureSamplerState {
                address_u: TextureAddressMode::Clamp,
                address_v: TextureAddressMode::Clamp,
                min_filter: TextureFilter::Linear,
                mag_filter: TextureFilter::Linear,
            },
            mip_filter: TextureFilter::Point,
            max_mip_level,
            max_anisotropy: 1,
            mip_lod_bias_bits: 0,
            border_color: 0,
        })
    }

    pub fn atlas_page_bytes(&self, page: &RuhunaAtlasPage) -> &[u8] {
        &self.yabx.bytes()[page.data.clone()]
    }

    /// Reproduces `sub_F47700` and `sub_7CB9B0`. The callback mirrors the
    /// game's page-index-to-texture-handle lookup and must return zero when a
    /// page is unavailable. Tokens stay opaque so this remains valid on x86
    /// and x64 hosts.
    pub fn build_runtime_font<F>(
        &self,
        owner_token: u32,
        mut texture_token_for_page: F,
    ) -> Result<RuhunaRuntimeFont, RuhunaError>
    where
        F: FnMut(u16) -> u32,
    {
        let mut glyph_by_object = vec![None; self.yabx.objects.len()];
        for glyph in &self.glyphs {
            glyph_by_object[glyph.object_index] = Some(glyph);
        }
        let ordered_glyphs = self
            .database
            .glyph_object_indices
            .iter()
            .map(|&object_index| {
                glyph_by_object[object_index].ok_or_else(|| {
                    RuhunaError::Invalid(format!(
                        "Database glyph reference {object_index} has no parsed Glyph"
                    ))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let first = ordered_glyphs.first().ok_or_else(|| {
            RuhunaError::Invalid(
                "Database has no glyphs; the game lookup builder dereferences the first entry"
                    .into(),
            )
        })?;
        let last = ordered_glyphs.last().unwrap();
        if last.code < first.code {
            return Err(RuhunaError::Invalid(format!(
                "Database glyph endpoints are not ascending: {} then {}",
                first.code, last.code
            )));
        }

        let minimum_code = first.code;
        let maximum_code = last.code;
        let lookup_len = usize::from(maximum_code - minimum_code) + 1;
        let mut dense_glyph_indices = vec![0; lookup_len];
        for (glyph_index, glyph) in ordered_glyphs.iter().enumerate() {
            if glyph.code < minimum_code || glyph.code > maximum_code {
                return Err(RuhunaError::Invalid(format!(
                    "glyph {} code {} lies outside endpoint range {}..={}",
                    glyph.object_index, glyph.code, minimum_code, maximum_code
                )));
            }
            dense_glyph_indices[usize::from(glyph.code - minimum_code)] =
                u16::try_from(glyph_index).map_err(|_| {
                    RuhunaError::Invalid(format!(
                        "glyph index {glyph_index} does not fit the game's u16 lookup entry"
                    ))
                })?;
        }

        let mut glyph_pages = Vec::with_capacity(ordered_glyphs.len());
        let mut glyphs = Vec::with_capacity(ordered_glyphs.len());
        for glyph in ordered_glyphs {
            let page = (glyph.page != u16::MAX).then_some(glyph.page);
            let texture_token = page.map_or(0, &mut texture_token_for_page);
            glyph_pages.push(page);
            glyphs.push(build_runtime_glyph_record(
                &self.database,
                glyph,
                owner_token,
                texture_token,
            ));
        }

        Ok(RuhunaRuntimeFont {
            minimum_code,
            maximum_code,
            dense_glyph_indices,
            glyph_pages,
            glyphs,
        })
    }
}

fn build_runtime_glyph_record(
    database: &RuhunaDatabase,
    glyph: &RuhunaGlyph,
    owner_token: u32,
    texture_token: u32,
) -> RuhunaRuntimeGlyphRecord {
    let inverse_texture_width = 1.0 / f32::from(database.texture_width);
    let page_height = if u32::from(glyph.page) == database.texture_page_count.wrapping_sub(1) {
        database.texture_last_height
    } else {
        database.texture_height
    };
    let inverse_texture_height = 1.0 / f32::from(page_height);
    let em_pixels = (f32::from(database.point) * 1.333_333_4 + 0.5).trunc() as i32;
    let flag_mode = (database.flags & 1) * 2;

    let mut record = RuhunaRuntimeGlyphRecord {
        owner_token,
        code: glyph.code,
        point_x: database.point,
        point_y: database.point,
        atlas_x: u32::from(glyph.box_x1),
        atlas_y: u32::from(glyph.box_y1),
        texture_token,
        inverse_texture_width,
        inverse_texture_height,
        enabled: 1,
        line_height: u32::from(database.max_ascent) + u32::from(database.max_descent),
        advance_x: u32::from(glyph.cell_increment_x),
        advance_y: u32::from(glyph.cell_increment_y),
        em_pixels_x: em_pixels,
        em_pixels_y: em_pixels,
        field_4c: -1,
        flag_mode,
        field_58: -4096,
        ..Default::default()
    };

    if texture_token != 0 {
        if database.flags & 2 != 0 {
            record.width = u32::from(glyph.box_y2)
                .wrapping_sub(u32::from(glyph.box_y1))
                .wrapping_add(1);
            record.height = u32::from(glyph.box_x2)
                .wrapping_sub(u32::from(glyph.box_x1))
                .wrapping_add(1);
            record.bearing_x = i32::from(database.max_descent) + i32::from(glyph.origin_y);
            record.bearing_y = i32::from(glyph.origin_x);
            record.advance_x = u32::from(glyph.cell_increment_y);
            record.advance_y = u32::from(glyph.cell_increment_x).wrapping_add(1);
            record.rotated = 1;

            let x0 = (f32::from(glyph.box_x1) - 1.0) * inverse_texture_width;
            let x1 = (f32::from(glyph.box_x2) + 1.0) * inverse_texture_width;
            let y0 = (f32::from(glyph.box_y1) - 1.0) * inverse_texture_height;
            let y1 = (f32::from(glyph.box_y2) + 1.0) * inverse_texture_height;
            record.uv0 = [x0, y1];
            record.uv1 = [x0, y0];
            record.uv2 = [x1, y1];
            record.uv3 = [x1, y0];
        } else {
            record.width = u32::from(glyph.box_x2)
                .wrapping_sub(u32::from(glyph.box_x1))
                .wrapping_add(1);
            record.height = u32::from(glyph.box_y2)
                .wrapping_sub(u32::from(glyph.box_y1))
                .wrapping_add(1);
            record.bearing_x = i32::from(glyph.origin_x);
            record.bearing_y = i32::from(database.max_ascent) - i32::from(glyph.origin_y);

            let x0 = (f32::from(glyph.box_x1) - 1.0) * inverse_texture_width;
            let x1 = (glyph.box_x1 as f32 + record.width as f32 + 1.0) * inverse_texture_width;
            let y0 = (f32::from(glyph.box_y1) - 1.0) * inverse_texture_height;
            let y1 = (glyph.box_y1 as f32 + 1.0 + record.height as f32) * inverse_texture_height;
            record.uv0 = [x0, y0];
            record.uv1 = [x1, y0];
            record.uv2 = [x0, y1];
            record.uv3 = [x1, y1];
        }
    }

    // Assembly at 0x7CBE48..0x7CBE61 doubles flags bit 0 into +0x50,
    // doubles that value once more, then adds it to +0x2C.
    record.bearing_y += i32::try_from(flag_mode * 2).unwrap();
    record
}

fn objects_of_class<'a>(yabx: &'a YabxFile, expected_name: &[u8]) -> Vec<(usize, &'a YabxObject)> {
    yabx.objects
        .iter()
        .enumerate()
        .filter(|(_, object)| {
            yabx.object_class(object)
                .is_some_and(|class| class.name == expected_name)
        })
        .collect()
}

fn parse_database(
    yabx: &YabxFile,
    object_index: usize,
    object: &YabxObject,
) -> Result<RuhunaDatabase, RuhunaError> {
    let mut reader = ObjectReader::new(yabx.object_data(object), object_index);
    let database = RuhunaDatabase {
        object_index,
        id: reader.read_dynamic_string("id")?,
        platform: reader.read_dynamic_string("platform")?,
        library: reader.read_dynamic_string("library")?,
        name: reader.read_dynamic_string("name")?,
        comment: reader.read_dynamic_string("comment")?,
        flags: reader.read_u32("flags")?,
        point: reader.read_u16("point")?,
        max_ascent: reader.read_u16("max_ascent")?,
        max_descent: reader.read_u16("max_descent")?,
        max_glyph_width: reader.read_u16("max_glyph_w")?,
        max_glyph_height: reader.read_u16("max_glyph_h")?,
        texture_page_count: reader.read_u32("tex_page")?,
        texture_width: reader.read_u16("tex_w")?,
        texture_height: reader.read_u16("tex_h")?,
        texture_last_height: reader.read_u16("tex_last_h")?,
        glyph_margin: reader.read_u16("glyph_margin")?,
        glyph_count: reader.read_u16("glyph_cnt")?,
        glyph_object_indices: reader.read_dynamic_object_references("glyph", yabx.objects.len())?,
        texture_object_indices: reader
            .read_dynamic_object_references("texture", yabx.objects.len())?,
    };
    reader.finish()?;
    Ok(database)
}

fn parse_glyph(
    yabx: &YabxFile,
    object_index: usize,
    object: &YabxObject,
) -> Result<RuhunaGlyph, RuhunaError> {
    let mut reader = ObjectReader::new(yabx.object_data(object), object_index);
    let glyph = RuhunaGlyph {
        object_index,
        code: reader.read_u16("code")?,
        cell_increment_x: reader.read_u16("cell_inc_x")?,
        cell_increment_y: reader.read_u16("cell_inc_y")?,
        page: reader.read_u16("page")?,
        origin_x: reader.read_i16("origin_x")?,
        origin_y: reader.read_i16("origin_y")?,
        box_x1: reader.read_u16("box_x1")?,
        box_y1: reader.read_u16("box_y1")?,
        box_x2: reader.read_u16("box_x2")?,
        box_y2: reader.read_u16("box_y2")?,
        kerning_info_count: reader.read_u16("kerning_info_cnt")?,
    };
    let mut kerning = reader.read_dynamic_reader("kerning_info")?;
    let serialized_count = kerning.read_u32("kerning_info count")?;
    if serialized_count != u32::from(glyph.kerning_info_count) {
        return Err(reader.error(format!(
            "kerning_info count {serialized_count} does not match kerning_info_cnt {}",
            glyph.kerning_info_count
        )));
    }
    if serialized_count != 0 {
        return Err(reader.error(format!(
            "non-empty kerning_info arrays are not implemented without a proven element layout (count {serialized_count})"
        )));
    }
    kerning.finish()?;
    reader.finish()?;
    Ok(glyph)
}

fn parse_texture_resource(
    yabx: &YabxFile,
    object_index: usize,
    object: &YabxObject,
) -> Result<RuhunaTextureResource, RuhunaError> {
    let mut reader = ObjectReader::new(yabx.object_data(object), object_index);
    let local_range = reader.read_dynamic_range("file")?;
    reader.finish()?;
    let file_data = (object.data.start + local_range.start)..(object.data.start + local_range.end);
    let bytes = &yabx.bytes()[file_data.clone()];
    if !bytes.starts_with(b"AVTS") {
        return Err(RuhunaError::Invalid(format!(
            "TextureResource object {object_index} file does not start with AVTS"
        )));
    }
    Ok(RuhunaTextureResource {
        object_index,
        file_data,
    })
}

fn validate_referenced_classes(
    yabx: &YabxFile,
    object_indices: &[usize],
    expected_class: &[u8],
    label: &str,
) -> Result<(), RuhunaError> {
    for &object_index in object_indices {
        let object = &yabx.objects[object_index];
        let actual = yabx
            .object_class(object)
            .map(|class| class.name.as_slice())
            .unwrap_or(b"<unknown>");
        if actual != expected_class {
            return Err(RuhunaError::Invalid(format!(
                "Database {label} reference points to object {object_index} of class {:?}",
                String::from_utf8_lossy(actual)
            )));
        }
    }
    Ok(())
}

struct ObjectReader<'a> {
    bytes: &'a [u8],
    offset: usize,
    object_index: usize,
}

impl<'a> ObjectReader<'a> {
    fn new(bytes: &'a [u8], object_index: usize) -> Self {
        Self {
            bytes,
            offset: 0,
            object_index,
        }
    }

    fn error(&self, message: impl Into<String>) -> RuhunaError {
        RuhunaError::Invalid(format!(
            "ruhuna object {} at {:#x}: {}",
            self.object_index,
            self.offset,
            message.into()
        ))
    }

    fn read_bytes(&mut self, size: usize, label: &str) -> Result<&'a [u8], RuhunaError> {
        let end = self
            .offset
            .checked_add(size)
            .ok_or_else(|| self.error(format!("{label} size overflows")))?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| self.error(format!("{label} needs {size} bytes")))?;
        self.offset = end;
        Ok(bytes)
    }

    fn read_u16(&mut self, label: &str) -> Result<u16, RuhunaError> {
        Ok(u16::from_le_bytes(
            self.read_bytes(2, label)?.try_into().unwrap(),
        ))
    }

    fn read_i16(&mut self, label: &str) -> Result<i16, RuhunaError> {
        Ok(i16::from_le_bytes(
            self.read_bytes(2, label)?.try_into().unwrap(),
        ))
    }

    fn read_u32(&mut self, label: &str) -> Result<u32, RuhunaError> {
        Ok(u32::from_le_bytes(
            self.read_bytes(4, label)?.try_into().unwrap(),
        ))
    }

    fn read_dynamic_range(&mut self, label: &str) -> Result<Range<usize>, RuhunaError> {
        let size = self.read_u32(&format!("{label} byte size"))?;
        let size = usize::try_from(size)
            .map_err(|_| self.error(format!("{label} byte size does not fit usize")))?;
        let start = self.offset;
        self.read_bytes(size, label)?;
        Ok(start..self.offset)
    }

    fn read_dynamic_reader(&mut self, label: &str) -> Result<Self, RuhunaError> {
        let range = self.read_dynamic_range(label)?;
        Ok(Self::new(&self.bytes[range], self.object_index))
    }

    fn read_dynamic_string(&mut self, label: &str) -> Result<Vec<u8>, RuhunaError> {
        let mut field = self.read_dynamic_reader(label)?;
        let length = usize::from(field.read_u16(&format!("{label} string length"))?);
        if length == 0 {
            return Err(field.error(format!("{label} string length is zero")));
        }
        let bytes = field.read_bytes(length, label)?;
        if bytes.last() != Some(&0) {
            return Err(field.error(format!("{label} string is not NUL-terminated")));
        }
        field.finish()?;
        Ok(bytes[..bytes.len() - 1].to_vec())
    }

    fn read_dynamic_object_references(
        &mut self,
        label: &str,
        object_count: usize,
    ) -> Result<Vec<usize>, RuhunaError> {
        let mut field = self.read_dynamic_reader(label)?;
        let count = field.read_u32(&format!("{label} reference count"))?;
        let count = usize::try_from(count)
            .map_err(|_| field.error(format!("{label} reference count does not fit usize")))?;
        let mut references = Vec::with_capacity(count);
        for index in 0..count {
            let encoded = field.read_i16(&format!("{label}[{index}]"))?;
            references.push(
                decode_object_reference(encoded, object_count).map_err(|message| {
                    field.error(format!("invalid {label}[{index}] reference: {message}"))
                })?,
            );
        }
        field.finish()?;
        Ok(references)
    }

    fn finish(&self) -> Result<(), RuhunaError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(self.error(format!(
                "{} unread bytes remain",
                self.bytes.len() - self.offset
            )))
        }
    }
}

fn decode_object_reference(encoded: i16, object_count: usize) -> Result<usize, String> {
    if encoded <= 0 {
        return Err(format!(
            "encoded ID {encoded} is null or an external/local-library reference"
        ));
    }
    let index = i32::from(encoded) - SERIALIZED_OBJECT_ID_BASE;
    let index = usize::try_from(index)
        .map_err(|_| format!("encoded ID {encoded} is below the object ID base"))?;
    if index >= object_count {
        return Err(format!(
            "encoded ID {encoded} resolves to object {index}, outside {object_count} objects"
        ));
    }
    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime_database(flags: u32) -> RuhunaDatabase {
        RuhunaDatabase {
            object_index: 0,
            id: Vec::new(),
            platform: Vec::new(),
            library: Vec::new(),
            name: Vec::new(),
            comment: Vec::new(),
            flags,
            point: 18,
            max_ascent: 15,
            max_descent: 3,
            max_glyph_width: 5,
            max_glyph_height: 8,
            texture_page_count: 2,
            texture_width: 100,
            texture_height: 200,
            texture_last_height: 50,
            glyph_margin: 1,
            glyph_count: 1,
            glyph_object_indices: vec![1],
            texture_object_indices: vec![2],
        }
    }

    fn runtime_glyph() -> RuhunaGlyph {
        RuhunaGlyph {
            object_index: 1,
            code: 0x41,
            cell_increment_x: 6,
            cell_increment_y: 7,
            page: 1,
            origin_x: -2,
            origin_y: 4,
            box_x1: 10,
            box_y1: 20,
            box_x2: 14,
            box_y2: 27,
            kerning_info_count: 0,
        }
    }

    fn assert_uv_close(actual: [f32; 2], expected: [f32; 2]) {
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert!((actual - expected).abs() <= f32::EPSILON);
        }
    }

    #[test]
    fn decodes_game_object_id_base() {
        assert_eq!(decode_object_reference(10_001, 2).unwrap(), 0);
        assert_eq!(decode_object_reference(10_002, 2).unwrap(), 1);
        assert!(decode_object_reference(10_000, 2).is_err());
        assert!(decode_object_reference(-1, 2).is_err());
    }

    #[test]
    fn runtime_glyph_record_matches_binary_offsets() {
        assert_eq!(std::mem::size_of::<RuhunaRuntimeGlyphRecord>(), 128);
        assert_eq!(
            std::mem::offset_of!(RuhunaRuntimeGlyphRecord, font_slot_id),
            4
        );
        assert_eq!(std::mem::offset_of!(RuhunaRuntimeGlyphRecord, code), 6);
        assert_eq!(
            std::mem::offset_of!(RuhunaRuntimeGlyphRecord, texture_token),
            24
        );
        assert_eq!(
            std::mem::offset_of!(RuhunaRuntimeGlyphRecord, bearing_x),
            40
        );
        assert_eq!(
            std::mem::offset_of!(RuhunaRuntimeGlyphRecord, advance_x),
            60
        );
        assert_eq!(std::mem::offset_of!(RuhunaRuntimeGlyphRecord, uv0), 92);
        assert_eq!(std::mem::offset_of!(RuhunaRuntimeGlyphRecord, rotated), 124);
    }

    #[test]
    fn builds_normal_runtime_glyph_with_last_page_height() {
        let record = build_runtime_glyph_record(&runtime_database(0), &runtime_glyph(), 9, 17);
        assert_eq!(record.owner_token, 9);
        assert_eq!(record.texture_token, 17);
        assert_eq!(record.code, 0x41);
        assert_eq!(record.point_x, 18);
        assert_eq!(record.point_y, 18);
        assert_eq!(record.atlas_x, 10);
        assert_eq!(record.atlas_y, 20);
        assert_eq!(record.inverse_texture_width, 0.01);
        assert_eq!(record.inverse_texture_height, 0.02);
        assert_eq!(record.bearing_x, -2);
        assert_eq!(record.bearing_y, 11);
        assert_eq!(record.width, 5);
        assert_eq!(record.height, 8);
        assert_eq!(record.line_height, 18);
        assert_eq!(record.advance_x, 6);
        assert_eq!(record.advance_y, 7);
        assert_eq!(record.em_pixels_x, 24);
        assert_eq!(record.em_pixels_y, 24);
        assert_eq!(record.field_4c, -1);
        assert_eq!(record.field_58, -4096);
        assert_uv_close(record.uv0, [0.09, 0.38]);
        assert_uv_close(record.uv1, [0.16, 0.38]);
        assert_uv_close(record.uv2, [0.09, 0.58]);
        assert_uv_close(record.uv3, [0.16, 0.58]);
        assert_eq!(record.rotated, 0);
    }

    #[test]
    fn builds_rotated_runtime_glyph_and_applies_flag_one_offset() {
        let record = build_runtime_glyph_record(&runtime_database(3), &runtime_glyph(), 0, 1);
        assert_eq!(record.bearing_x, 7);
        assert_eq!(record.bearing_y, 2);
        assert_eq!(record.width, 8);
        assert_eq!(record.height, 5);
        assert_eq!(record.advance_x, 7);
        assert_eq!(record.advance_y, 7);
        assert_eq!(record.flag_mode, 2);
        assert_uv_close(record.uv0, [0.09, 0.56]);
        assert_uv_close(record.uv1, [0.09, 0.38]);
        assert_uv_close(record.uv2, [0.15, 0.56]);
        assert_uv_close(record.uv3, [0.15, 0.38]);
        assert_eq!(record.rotated, 1);
    }

    #[test]
    fn unavailable_texture_zeroes_geometry_before_common_flag_adjustment() {
        let record = build_runtime_glyph_record(&runtime_database(1), &runtime_glyph(), 0, 0);
        assert_eq!(record.bearing_x, 0);
        assert_eq!(record.bearing_y, 4);
        assert_eq!(record.width, 0);
        assert_eq!(record.height, 0);
        assert_eq!(record.uv0, [0.0; 2]);
        assert_eq!(record.uv1, [0.0; 2]);
        assert_eq!(record.uv2, [0.0; 2]);
        assert_eq!(record.uv3, [0.0; 2]);
    }

    #[test]
    fn runtime_lookup_rejects_zero_filled_dense_table_holes() {
        let glyph_a = RuhunaRuntimeGlyphRecord {
            code: 0x41,
            ..Default::default()
        };
        let glyph_c = RuhunaRuntimeGlyphRecord {
            code: 0x43,
            ..Default::default()
        };
        let font = RuhunaRuntimeFont {
            minimum_code: 0x41,
            maximum_code: 0x43,
            dense_glyph_indices: vec![0, 0, 1],
            glyph_pages: vec![Some(0), Some(0)],
            glyphs: vec![glyph_a, glyph_c],
        };

        assert_eq!(font.glyph(0x41).map(|glyph| glyph.code), Some(0x41));
        assert!(font.glyph(0x42).is_none());
        assert_eq!(font.glyph(0x43).map(|glyph| glyph.code), Some(0x43));
        assert!(font.glyph(0x40).is_none());
        assert!(font.glyph(0x44).is_none());
    }

    #[test]
    fn global_font_slot_getter_writes_slot_id_only_on_success() {
        let mut font = RuhunaRuntimeFont {
            minimum_code: 0x41,
            maximum_code: 0x43,
            dense_glyph_indices: vec![0, 0, 1],
            glyph_pages: vec![Some(0), Some(0)],
            glyphs: vec![
                RuhunaRuntimeGlyphRecord {
                    code: 0x41,
                    ..Default::default()
                },
                RuhunaRuntimeGlyphRecord {
                    code: 0x43,
                    ..Default::default()
                },
            ],
        };

        let glyph = font.glyph_for_font_slot(7, 0x43).unwrap();
        assert_eq!(glyph.code, 0x43);
        assert_eq!(glyph.font_slot_id, 7);
        assert!(font.glyph_for_font_slot(9, 0x42).is_none());
        assert_eq!(font.glyph(0x41).unwrap().font_slot_id, 0);
    }
}
