use crate::dds::DdsDescriptor;
use crate::renderer::backend::RenderBackendError;
use crate::renderer::resources::{FennelAtlasSourceSet, SrdTextureSourceSet};
use crate::ruhuna::RuhunaSamplerState;

pub(super) struct WgpuTexture2d {
    #[allow(dead_code)]
    texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}

pub(super) struct WgpuTextureSet {
    textures: Vec<Option<WgpuTexture2d>>,
}

pub(super) struct WgpuFennelAtlas {
    pages: Vec<WgpuTexture2d>,
    sampler: RuhunaSamplerState,
}

impl WgpuTexture2d {
    pub fn from_dds(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str,
    ) -> Result<Self, RenderBackendError> {
        let descriptor = DdsDescriptor::parse(bytes).map_err(texture_error)?;
        descriptor.validate_data_len(bytes).map_err(texture_error)?;
        let levels = descriptor
            .decode_rgba8_levels_with_library(bytes)
            .map_err(texture_error)?;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: descriptor.width,
                height: descriptor.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: descriptor.mip_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for level in levels {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: level.mip_index,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &level.rgba,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(level.width * 4),
                    rows_per_image: Some(level.height),
                },
                wgpu::Extent3d {
                    width: level.width,
                    height: level.height,
                    depth_or_array_layers: 1,
                },
            );
        }
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Ok(Self { texture, view })
    }

    pub fn solid_rgba(device: &wgpu::Device, queue: &wgpu::Queue, rgba: [u8; 4]) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Ceylon 1x1 texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self { texture, view }
    }

    pub fn opaque_black(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        Self::solid_rgba(device, queue, [0, 0, 0, 255])
    }
}

#[cfg(all(test, target_os = "macos"))]
impl WgpuTextureSet {
    pub(super) fn from_test_colors(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        colors: [[u8; 4]; 2],
    ) -> Self {
        Self {
            textures: colors
                .into_iter()
                .map(|color| Some(WgpuTexture2d::solid_rgba(device, queue, color)))
                .collect(),
        }
    }
}

impl WgpuTextureSet {
    pub fn from_sources(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        sources: SrdTextureSourceSet,
    ) -> Result<Self, RenderBackendError> {
        let textures = sources
            .iter()
            .enumerate()
            .map(|(index, source)| {
                source
                    .map(|bytes| {
                        WgpuTexture2d::from_dds(
                            device,
                            queue,
                            bytes,
                            &format!("SRD texture {index}"),
                        )
                    })
                    .transpose()
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { textures })
    }

    pub fn get(&self, index: usize) -> Option<&WgpuTexture2d> {
        self.textures.get(index)?.as_ref()
    }
}

impl WgpuFennelAtlas {
    pub fn from_sources(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        sources: FennelAtlasSourceSet,
    ) -> Result<Self, RenderBackendError> {
        let sampler = sources.sampler();
        let pages = sources
            .pages()
            .enumerate()
            .map(|(index, bytes)| {
                WgpuTexture2d::from_dds(device, queue, bytes, &format!("Fennel atlas page {index}"))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { pages, sampler })
    }

    pub fn get(&self, page: usize) -> Option<&WgpuTexture2d> {
        self.pages.get(page)
    }

    pub fn sampler(&self) -> RuhunaSamplerState {
        self.sampler
    }
}

#[cfg(all(test, target_os = "macos"))]
impl WgpuFennelAtlas {
    pub(super) fn from_test_texture(texture: WgpuTexture2d, sampler: RuhunaSamplerState) -> Self {
        Self {
            pages: vec![texture],
            sampler,
        }
    }
}

fn texture_error(error: impl std::fmt::Display) -> RenderBackendError {
    RenderBackendError(format!("WebGPU DDS upload failed: {error}"))
}
