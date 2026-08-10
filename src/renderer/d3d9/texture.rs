use std::ffi::c_void;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::ptr;
use std::slice;

use super::bindings::{
    D3DFMT_A8R8G8B8, D3DFORMAT, D3DLOCKED_RECT, D3DPOOL_DEFAULT, D3DPOOL_SYSTEMMEM,
    IDirect3DDevice9, IDirect3DTexture9, POINT, RECT,
};

use crate::dds::{D3d9Direct2dUpload, D3d9TextureCreation, DdsDescriptor, DdsLoadPolicy};
use crate::renderer::assets::{FennelAtlasSourceSet, SrdTextureSourceSet};
use crate::ruhuna::RuhunaD3d9SamplerState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct D3d9TextureError(pub String);

impl fmt::Display for D3d9TextureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for D3d9TextureError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum D3d9TextureUploadPath {
    GameNativeLayout,
    IndependentRgba8Decode,
}

pub struct D3d9Texture2d {
    pub texture: IDirect3DTexture9,
    pub width: u32,
    pub height: u32,
    pub mip_levels: u32,
    pub upload_path: D3d9TextureUploadPath,
}

pub struct SrdD3d9TextureSet {
    sources: SrdTextureSourceSet,
    textures: Vec<Option<D3d9Texture2d>>,
}

/// Device-reset-safe D3D9 textures for the DDS pages embedded in one Ruhuna
/// RFZ font. Page order is the AVTS/database page order already validated by
/// the platform-neutral atlas source set.
pub struct RuhunaD3d9AtlasSet {
    sources: FennelAtlasSourceSet,
    textures: Vec<Option<D3d9Texture2d>>,
}

impl RuhunaD3d9AtlasSet {
    pub fn from_sources(
        device: &IDirect3DDevice9,
        sources: FennelAtlasSourceSet,
    ) -> Result<Self, D3d9TextureError> {
        let mut result = Self {
            textures: (0..sources.page_count()).map(|_| None).collect(),
            sources,
        };
        result.create_device_objects(device)?;
        Ok(result)
    }

    pub fn invalidate_device_objects(&mut self) {
        for texture in &mut self.textures {
            *texture = None;
        }
    }

    pub fn create_device_objects(
        &mut self,
        device: &IDirect3DDevice9,
    ) -> Result<(), D3d9TextureError> {
        for (index, source) in self.sources.pages().enumerate() {
            self.textures[index] = Some(D3d9Texture2d::from_dds_bytes(device, source)?);
        }
        Ok(())
    }

    pub fn get(&self, page_index: usize) -> Option<&D3d9Texture2d> {
        self.textures.get(page_index)?.as_ref()
    }

    pub fn page_count(&self) -> usize {
        self.sources.page_count()
    }

    pub fn sampler(&self) -> RuhunaD3d9SamplerState {
        self.sources.sampler()
    }
}

impl SrdD3d9TextureSet {
    pub fn from_sources(
        device: &IDirect3DDevice9,
        sources: SrdTextureSourceSet,
    ) -> Result<Self, D3d9TextureError> {
        let mut set = Self {
            textures: (0..sources.len()).map(|_| None).collect(),
            sources,
        };
        set.create_device_objects(device)?;
        Ok(set)
    }

    pub fn invalidate_device_objects(&mut self) {
        for texture in &mut self.textures {
            *texture = None;
        }
    }

    pub fn create_device_objects(
        &mut self,
        device: &IDirect3DDevice9,
    ) -> Result<(), D3d9TextureError> {
        for (index, source) in self.sources.iter().enumerate() {
            self.textures[index] = source
                .map(|bytes| D3d9Texture2d::from_dds_bytes(device, bytes))
                .transpose()?;
        }
        Ok(())
    }

    pub fn get(&self, index: usize) -> Option<&D3d9Texture2d> {
        self.textures.get(index)?.as_ref()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct D3d9TextureAudit {
    pub file_count: usize,
    pub game_native_count: usize,
    pub independent_decode_count: usize,
    pub mip_level_count: usize,
}

pub fn audit_dds_device_uploads(
    device: &IDirect3DDevice9,
    root: &Path,
) -> Result<D3d9TextureAudit, D3d9TextureError> {
    let mut files = Vec::new();
    collect_dds_files(root, &mut files)?;
    files.sort();
    let mut audit = D3d9TextureAudit::default();
    for path in files {
        let bytes = fs::read(&path).map_err(|error| {
            D3d9TextureError(format!("failed to read {}: {error}", path.display()))
        })?;
        let texture = D3d9Texture2d::from_dds_bytes(device, &bytes)
            .map_err(|error| D3d9TextureError(format!("{}: {error}", path.display())))?;
        audit.file_count += 1;
        audit.mip_level_count += texture.mip_levels as usize;
        match texture.upload_path {
            D3d9TextureUploadPath::GameNativeLayout => audit.game_native_count += 1,
            D3d9TextureUploadPath::IndependentRgba8Decode => audit.independent_decode_count += 1,
        }
    }
    Ok(audit)
}

impl D3d9Texture2d {
    pub fn from_dds_bytes(
        device: &IDirect3DDevice9,
        bytes: &[u8],
    ) -> Result<Self, D3d9TextureError> {
        let descriptor = DdsDescriptor::parse(bytes).map_err(dds_error)?;
        descriptor.validate_data_len(bytes).map_err(dds_error)?;
        match descriptor.creation_plan(DdsLoadPolicy::default()) {
            D3d9TextureCreation::Direct2d {
                width,
                height,
                mip_levels,
                format,
                ..
            } => {
                let texture =
                    create_texture(device, width, height, mip_levels, D3DFORMAT(format.0))?;
                upload_game_native_levels(device, &texture, &descriptor, bytes)?;
                Ok(Self {
                    texture,
                    width,
                    height,
                    mip_levels,
                    upload_path: D3d9TextureUploadPath::GameNativeLayout,
                })
            }
            D3d9TextureCreation::D3dx2d { .. } => {
                let levels = descriptor
                    .decode_rgba8_levels_with_library(bytes)
                    .map_err(dds_error)?;
                let mip_levels = u32::try_from(levels.len()).map_err(|_| {
                    D3d9TextureError("decoded mip level count does not fit u32".into())
                })?;
                let texture = create_texture(
                    device,
                    descriptor.width,
                    descriptor.height,
                    mip_levels,
                    D3DFMT_A8R8G8B8,
                )?;
                for level in &levels {
                    upload_rgba8_level(
                        device,
                        &texture,
                        level.mip_index,
                        level.width,
                        level.height,
                        &level.rgba,
                    )?;
                }
                Ok(Self {
                    texture,
                    width: descriptor.width,
                    height: descriptor.height,
                    mip_levels,
                    upload_path: D3d9TextureUploadPath::IndependentRgba8Decode,
                })
            }
            D3d9TextureCreation::DirectCube { .. } | D3d9TextureCreation::D3dxCube { .. } => Err(
                D3d9TextureError("D3D9 editor texture upload currently accepts only 2D DDS".into()),
            ),
        }
    }
}

fn create_texture(
    device: &IDirect3DDevice9,
    width: u32,
    height: u32,
    mip_levels: u32,
    format: D3DFORMAT,
) -> Result<IDirect3DTexture9, D3d9TextureError> {
    let mut texture = None;
    unsafe {
        device
            .CreateTexture(
                width,
                height,
                mip_levels,
                0,
                format,
                D3DPOOL_DEFAULT,
                &mut texture,
                ptr::null_mut(),
            )
            .map_err(|error| {
                D3d9TextureError(format!(
                    "CreateTexture {width}x{height} mips={mip_levels} format={:#x} failed: {error}",
                    format.0
                ))
            })?;
    }
    texture.ok_or_else(|| D3d9TextureError("CreateTexture returned null".into()))
}

fn upload_game_native_levels(
    device: &IDirect3DDevice9,
    destination: &IDirect3DTexture9,
    descriptor: &DdsDescriptor,
    bytes: &[u8],
) -> Result<(), D3d9TextureError> {
    let uploads = descriptor.direct_2d_upload_plan().map_err(dds_error)?;
    for upload in uploads {
        match upload {
            D3d9Direct2dUpload::UpdateSurface(upload) => {
                let source_start = upload.source_offset as usize;
                let source_end = source_start + upload.source_byte_len as usize;
                let source = bytes.get(source_start..source_end).ok_or_else(|| {
                    D3d9TextureError(format!(
                        "DDS mip {} source range {source_start}..{source_end} is outside {} bytes",
                        upload.destination_mip_level,
                        bytes.len()
                    ))
                })?;
                upload_raw_level(
                    device,
                    destination,
                    upload.destination_mip_level,
                    upload.width,
                    upload.height,
                    D3DFORMAT(upload.staging_format.0),
                    upload.source_row_pitch,
                    upload.row_count,
                    source,
                    false,
                )?;
            }
            D3d9Direct2dUpload::CompressedLevelBelowFourSkipped {
                mip_level,
                width,
                height,
            } => {
                return Err(D3d9TextureError(format!(
                    "game-native DDS plan leaves compressed mip {mip_level} ({width}x{height}) without an upload"
                )));
            }
        }
    }
    Ok(())
}

fn upload_rgba8_level(
    device: &IDirect3DDevice9,
    destination: &IDirect3DTexture9,
    mip_level: u32,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Result<(), D3d9TextureError> {
    let row_pitch = width
        .checked_mul(4)
        .ok_or_else(|| D3d9TextureError("RGBA row pitch overflows u32".into()))?;
    let expected = row_pitch
        .checked_mul(height)
        .ok_or_else(|| D3d9TextureError("RGBA mip byte count overflows u32".into()))?
        as usize;
    if rgba.len() != expected {
        return Err(D3d9TextureError(format!(
            "decoded mip {mip_level} has {} RGBA bytes, expected {expected}",
            rgba.len()
        )));
    }
    upload_raw_level(
        device,
        destination,
        mip_level,
        width,
        height,
        D3DFMT_A8R8G8B8,
        row_pitch,
        height,
        rgba,
        true,
    )
}

#[allow(clippy::too_many_arguments)]
fn upload_raw_level(
    device: &IDirect3DDevice9,
    destination: &IDirect3DTexture9,
    mip_level: u32,
    width: u32,
    height: u32,
    format: D3DFORMAT,
    source_row_pitch: u32,
    row_count: u32,
    source: &[u8],
    rgba_to_bgra: bool,
) -> Result<(), D3d9TextureError> {
    let mut staging = None;
    unsafe {
        device
            .CreateTexture(
                width,
                height,
                1,
                0,
                format,
                D3DPOOL_SYSTEMMEM,
                &mut staging,
                ptr::null_mut(),
            )
            .map_err(|error| {
                D3d9TextureError(format!(
                    "CreateTexture SYSTEMMEM {width}x{height} format={:#x} failed: {error}",
                    format.0
                ))
            })?;
    }
    let staging =
        staging.ok_or_else(|| D3d9TextureError("CreateTexture SYSTEMMEM returned null".into()))?;
    let mut locked = D3DLOCKED_RECT::default();
    unsafe {
        staging
            .LockRect(0, &mut locked, ptr::null(), 0)
            .map_err(|error| D3d9TextureError(format!("LockRect staging failed: {error}")))?;
    }
    let copy_result = copy_rows(
        locked.pBits,
        locked.Pitch,
        source,
        source_row_pitch,
        row_count,
        rgba_to_bgra,
    );
    let unlock_result = unsafe { staging.UnlockRect(0) }
        .map_err(|error| D3d9TextureError(format!("UnlockRect staging failed: {error}")));
    copy_result?;
    unlock_result?;

    let source_surface = unsafe { staging.GetSurfaceLevel(0) }
        .map_err(|error| D3d9TextureError(format!("GetSurfaceLevel staging failed: {error}")))?;
    let destination_surface =
        unsafe { destination.GetSurfaceLevel(mip_level) }.map_err(|error| {
            D3d9TextureError(format!(
                "GetSurfaceLevel destination mip {mip_level} failed: {error}"
            ))
        })?;
    let source_rect = RECT {
        left: 0,
        top: 0,
        right: width as i32,
        bottom: height as i32,
    };
    let destination_point = POINT { x: 0, y: 0 };
    unsafe {
        device
            .UpdateSurface(
                &source_surface,
                &source_rect,
                &destination_surface,
                &destination_point,
            )
            .map_err(|error| {
                D3d9TextureError(format!(
                    "UpdateSurface destination mip {mip_level} failed: {error}"
                ))
            })?;
    }
    Ok(())
}

fn copy_rows(
    destination: *mut c_void,
    destination_pitch: i32,
    source: &[u8],
    source_pitch: u32,
    row_count: u32,
    rgba_to_bgra: bool,
) -> Result<(), D3d9TextureError> {
    if destination.is_null() || destination_pitch < 0 {
        return Err(D3d9TextureError(
            "staging LockRect returned an invalid pointer or pitch".into(),
        ));
    }
    let source_pitch = source_pitch as usize;
    let destination_pitch = destination_pitch as usize;
    let required = source_pitch
        .checked_mul(row_count as usize)
        .ok_or_else(|| D3d9TextureError("source row range overflows usize".into()))?;
    if source.len() != required {
        return Err(D3d9TextureError(format!(
            "source contains {} bytes, expected {required}",
            source.len()
        )));
    }
    if destination_pitch < source_pitch {
        return Err(D3d9TextureError(format!(
            "destination pitch {destination_pitch} is smaller than source pitch {source_pitch}"
        )));
    }
    for row in 0..row_count as usize {
        let source_row = &source[row * source_pitch..][..source_pitch];
        let destination_row = unsafe {
            slice::from_raw_parts_mut(
                destination.cast::<u8>().add(row * destination_pitch),
                source_pitch,
            )
        };
        if rgba_to_bgra {
            for (source_pixel, destination_pixel) in source_row
                .chunks_exact(4)
                .zip(destination_row.chunks_exact_mut(4))
            {
                destination_pixel.copy_from_slice(&[
                    source_pixel[2],
                    source_pixel[1],
                    source_pixel[0],
                    source_pixel[3],
                ]);
            }
        } else {
            destination_row.copy_from_slice(source_row);
        }
    }
    Ok(())
}

fn dds_error(error: crate::dds::DdsError) -> D3d9TextureError {
    D3d9TextureError(error.to_string())
}

fn collect_dds_files(root: &Path, output: &mut Vec<PathBuf>) -> Result<(), D3d9TextureError> {
    if root.is_file() {
        if root
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("dds"))
        {
            output.push(root.to_path_buf());
        }
        return Ok(());
    }
    let entries = fs::read_dir(root).map_err(|error| {
        D3d9TextureError(format!("failed to enumerate {}: {error}", root.display()))
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            D3d9TextureError(format!("failed to enumerate {}: {error}", root.display()))
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_dds_files(&path, output)?;
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("dds"))
        {
            output.push(path);
        }
    }
    Ok(())
}
