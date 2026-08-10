use std::borrow::Cow;
use std::collections::BTreeMap;
use std::mem;
use std::num::NonZeroU64;
use std::ops::Range;
use std::slice;

use crate::fennel::{
    FennelRenderVertex, fennel_default_draw_packet, fennel_default_raster_state,
    fennel_default_shader_key,
};
use crate::render::{
    CeylonAlphaStencilState, CeylonDepthState, CeylonRasterState, CeylonSrdFixedShaderConstants,
    D3d9BlendFactor, D3d9BlendOperation, D3d9ComparisonFunction, D3d9CullMode, D3d9FillMode,
    D3d9StencilOperation, SrdD3d9BlendPreset, SrdRenderVertex, ceylon_d3d9_blend_preset,
};
use crate::renderer::assets::{FennelAtlasSourceSet, SrdTextureSourceSet};
use crate::renderer::backend::{
    CompositionReadback, FennelAtlasHandle, FennelRenderBatch, RenderBackendError,
    SrdExternalRenderState, SrdRenderBackend, SrdTextureSetHandle,
};
use crate::ruhuna::RuhunaD3d9SamplerState;
use crate::shader::CEYLON_SIMPLE_SHADER_KEY_LENGTH;
use crate::srd_draw::{EvidenceCompleteSrdDraw, EvidenceSrdTextureBinding};
use crate::texture::{TextureAddressMode, TextureFilter, TextureSamplerState};

use super::shaders::{
    WgpuShaderPair, WgpuVertexInput, WgpuVertexSemantic, embedded_wgpu_shader_pair,
    pixel_source_with_alpha_test, pixel_source_with_sampler_biases, pixel_sources,
    vertex_source_with_d3d_color_swizzle, vertex_sources,
};
use super::texture::{WgpuFennelAtlas, WgpuTexture2d, WgpuTextureSet};

const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const DEPTH_STENCIL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24PlusStencil8;
const VERTEX_UNIFORM_SIZE: u64 = 256 * 16;
const PIXEL_UNIFORM_SIZE: u64 = 224 * 16;
const UNIFORM_DRAW_SIZE: u64 = VERTEX_UNIFORM_SIZE + PIXEL_UNIFORM_SIZE;
const GPU_ARENA_CHUNK_SIZE: u64 = 2 * 1024 * 1024;

pub struct WgpuSrdRenderBackend {
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter_diagnostics: String,
    enabled_features: wgpu::Features,
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    pipelines: BTreeMap<PipelineKey, wgpu::RenderPipeline>,
    samplers: BTreeMap<SamplerKey, wgpu::Sampler>,
    fallback_texture: WgpuTexture2d,
    texture_sets: BTreeMap<SrdTextureSetHandle, WgpuTextureSet>,
    fennel_atlases: BTreeMap<FennelAtlasHandle, WgpuFennelAtlas>,
    next_texture_set: u64,
    next_fennel_atlas: u64,
    composition: Option<CompositionTarget>,
    encoder: Option<wgpu::CommandEncoder>,
    uniform_arena: GpuArena,
    vertex_arena: GpuArena,
}

struct CompositionTarget {
    #[allow(dead_code)]
    color: wgpu::Texture,
    color_view: wgpu::TextureView,
    #[allow(dead_code)]
    depth_stencil: wgpu::Texture,
    depth_stencil_view: wgpu::TextureView,
    size: [u32; 2],
}

struct GpuArena {
    chunks: Vec<GpuArenaChunk>,
    usage: wgpu::BufferUsages,
    label: &'static str,
}

struct GpuArenaChunk {
    buffer: wgpu::Buffer,
    size: u64,
    used: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum InputLayout {
    Srd,
    Fennel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum DrawTopology {
    TriangleStrip,
    TriangleList,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct PipelineKey {
    shader_key: [u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH],
    input_layout: InputLayout,
    topology: DrawTopology,
    sampler_bias_bits: [u32; 2],
    alpha_comparison: u32,
    alpha_reference: u8,
    blend_enabled: bool,
    source_blend: u32,
    destination_blend: u32,
    blend_operation: u32,
    separate_alpha_blend: bool,
    source_blend_alpha: u32,
    destination_blend_alpha: u32,
    blend_operation_alpha: u32,
    cull_mode: u32,
    fill_mode: u32,
    color_write_mask: u32,
    depth_enabled: bool,
    depth_write_enabled: bool,
    depth_comparison: u32,
    stencil_enabled: bool,
    stencil_comparison: u32,
    stencil_fail: u32,
    stencil_depth_fail: u32,
    stencil_pass: u32,
    stencil_read_mask: u32,
    stencil_write_mask: u32,
    depth_bias: i32,
}

#[derive(Debug, Clone, Copy)]
struct PipelineSpec {
    key: PipelineKey,
    blend: SrdD3d9BlendPreset,
    raster: CeylonRasterState,
    depth: CeylonDepthState,
    alpha_stencil: CeylonAlphaStencilState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct SamplerKey {
    address_u: u32,
    address_v: u32,
    min_filter: u32,
    mag_filter: u32,
    mip_filter: u32,
    lod_min_bits: u32,
    lod_max_bits: u32,
    anisotropy: u16,
}

#[derive(Debug, Clone, Copy)]
enum TextureSelection {
    Fallback,
    Srd {
        handle: SrdTextureSetHandle,
        index: usize,
    },
    Fennel {
        handle: FennelAtlasHandle,
        page: usize,
    },
}

#[derive(Debug, Clone, Copy)]
struct DrawTextureBinding {
    texture: TextureSelection,
    sampler: SamplerKey,
}

impl WgpuSrdRenderBackend {
    pub fn new() -> Result<Self, RenderBackendError> {
        pollster::block_on(Self::new_async())
    }

    pub async fn new_async() -> Result<Self, RenderBackendError> {
        #[cfg(target_os = "macos")]
        let backends = wgpu::Backends::METAL;
        #[cfg(not(target_os = "macos"))]
        let backends = wgpu::Backends::from_env().unwrap_or(wgpu::Backends::all());

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
            ..wgpu::InstanceDescriptor::default()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .map_err(|error| {
                RenderBackendError(format!(
                    "WebGPU found no hardware graphics adapter: {error}"
                ))
            })?;
        let adapter_features = adapter.features();
        if !adapter_features.contains(wgpu::Features::SHADER_F16) {
            let info = adapter.get_info();
            return Err(RenderBackendError(format!(
                "WebGPU adapter {} ({:?}) cannot run the translated Ceylon shaders because SHADER_F16 is unavailable",
                info.name, info.backend
            )));
        }
        let optional_features =
            wgpu::Features::POLYGON_MODE_LINE | wgpu::Features::POLYGON_MODE_POINT;
        let enabled_features = wgpu::Features::SHADER_F16 | (adapter_features & optional_features);
        let info = adapter.get_info();
        let adapter_diagnostics = format!("{} / {:?}", info.name, info.backend);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("SRD WebGPU preview device"),
                required_features: enabled_features,
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
            })
            .await
            .map_err(|error| {
                RenderBackendError(format!(
                    "WebGPU device creation failed on {adapter_diagnostics}: {error}"
                ))
            })?;

        validate_embedded_shader_modules(&device)?;
        let bind_group_layout = create_bind_group_layout(&device);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Ceylon WebGPU pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let fallback_texture = WgpuTexture2d::opaque_black(&device, &queue);

        Ok(Self {
            device,
            queue,
            adapter_diagnostics,
            enabled_features,
            bind_group_layout,
            pipeline_layout,
            pipelines: BTreeMap::new(),
            samplers: BTreeMap::new(),
            fallback_texture,
            texture_sets: BTreeMap::new(),
            fennel_atlases: BTreeMap::new(),
            next_texture_set: 1,
            next_fennel_atlas: 1,
            composition: None,
            encoder: None,
            uniform_arena: GpuArena::new(
                wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                "Ceylon WebGPU uniform arena",
            ),
            vertex_arena: GpuArena::new(
                wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                "Ceylon WebGPU vertex arena",
            ),
        })
    }

    pub fn adapter_diagnostics(&self) -> &str {
        &self.adapter_diagnostics
    }

    pub fn release_srd_textures(&mut self, handle: SrdTextureSetHandle) {
        self.texture_sets.remove(&handle);
    }

    pub fn release_fennel_atlas(&mut self, handle: FennelAtlasHandle) {
        self.fennel_atlases.remove(&handle);
    }

    fn render_srd_vertices(
        &mut self,
        draw: &EvidenceCompleteSrdDraw,
        vertices: &[SrdRenderVertex],
        external: SrdExternalRenderState,
        textures: Option<SrdTextureSetHandle>,
    ) -> Result<(), RenderBackendError> {
        if vertices.len() < 3 {
            return Err(RenderBackendError(
                "WebGPU SRD triangle strip has fewer than three vertices".into(),
            ));
        }
        debug_assert_eq!(mem::size_of::<SrdRenderVertex>(), SrdRenderVertex::STRIDE);
        let byte_len = vertices
            .len()
            .checked_mul(SrdRenderVertex::STRIDE)
            .ok_or_else(|| RenderBackendError("WebGPU SRD vertex upload size overflow".into()))?;
        // SAFETY: SrdRenderVertex is repr(C), its asserted stride includes every byte, and
        // the slice cannot outlive the source vertices.
        let bytes = unsafe { slice::from_raw_parts(vertices.as_ptr().cast::<u8>(), byte_len) };

        let mut alpha_stencil = external.alpha_stencil;
        alpha_stencil.alpha_test_enabled = draw.blend.alpha_test_enabled;
        alpha_stencil.apply_draw_packet(draw.packet);
        let spec = PipelineSpec::new(
            draw.shader_key,
            InputLayout::Srd,
            DrawTopology::TriangleStrip,
            [0, 0],
            draw.blend,
            draw.raster,
            draw.depth,
            alpha_stencil,
            draw.packet.field_2c.saturating_neg(),
        )?;
        let bindings = self.resolve_srd_textures(draw.texture_bindings, textures)?;
        self.submit_draw(
            spec,
            bytes,
            vertices.len(),
            draw.fixed_constants,
            draw.is_2d,
            bindings,
            external,
        )
    }

    fn render_fennel_batch(
        &mut self,
        batch: &FennelRenderBatch<'_>,
        external: SrdExternalRenderState,
        atlas_handle: FennelAtlasHandle,
    ) -> Result<(), RenderBackendError> {
        if batch.vertices.is_empty() {
            return Ok(());
        }
        if batch.vertices.len() % 6 != 0 {
            return Err(RenderBackendError(
                "WebGPU Fennel batch vertex count is not a multiple of six".into(),
            ));
        }
        let sampler = self
            .fennel_atlases
            .get(&atlas_handle)
            .ok_or_else(|| {
                RenderBackendError(format!(
                    "WebGPU Fennel atlas handle {atlas_handle:?} is not loaded"
                ))
            })?
            .sampler();
        if self
            .fennel_atlases
            .get(&atlas_handle)
            .and_then(|atlas| atlas.get(batch.page_index))
            .is_none()
        {
            return Err(RenderBackendError(format!(
                "WebGPU Fennel atlas page {} is not loaded",
                batch.page_index
            )));
        }

        let shader_key = fennel_default_shader_key(batch.is_2d)
            .srd_simple_shader_direct_contributions()
            .map(|bits| bits.compact_key())
            .map_err(|error| {
                RenderBackendError(format!("unsupported WebGPU Fennel shader key: {error:?}"))
            })?;
        let packet = fennel_default_draw_packet(batch.is_2d);
        let blend = ceylon_d3d9_blend_preset(i32::from(packet.table_preset_id()));
        let raster = fennel_default_raster_state(batch.is_2d);
        let depth = CeylonDepthState::from_draw_flags(packet.draw_flags_00);
        let mut alpha_stencil = external.alpha_stencil;
        alpha_stencil.alpha_test_enabled = blend.alpha_test_enabled;
        alpha_stencil.apply_draw_packet(packet);
        let spec = PipelineSpec::new(
            shader_key,
            InputLayout::Fennel,
            DrawTopology::TriangleList,
            [sampler.mip_lod_bias_bits, 0],
            blend,
            raster,
            depth,
            alpha_stencil,
            packet.field_2c.saturating_neg(),
        )?;
        let sampler_key = SamplerKey::from_fennel(sampler);
        self.ensure_sampler(sampler_key)?;
        let fallback_sampler = SamplerKey::fallback();
        self.ensure_sampler(fallback_sampler)?;
        let bindings = [
            DrawTextureBinding {
                texture: TextureSelection::Fennel {
                    handle: atlas_handle,
                    page: batch.page_index,
                },
                sampler: sampler_key,
            },
            DrawTextureBinding {
                texture: TextureSelection::Fallback,
                sampler: fallback_sampler,
            },
        ];

        debug_assert_eq!(
            mem::size_of::<FennelRenderVertex>(),
            FennelRenderVertex::STRIDE
        );
        let byte_len = batch
            .vertices
            .len()
            .checked_mul(FennelRenderVertex::STRIDE)
            .ok_or_else(|| {
                RenderBackendError("WebGPU Fennel vertex upload size overflow".into())
            })?;
        // SAFETY: FennelRenderVertex is repr(C), its asserted stride includes every byte,
        // and the slice cannot outlive the source vertices.
        let bytes =
            unsafe { slice::from_raw_parts(batch.vertices.as_ptr().cast::<u8>(), byte_len) };
        self.submit_draw(
            spec,
            bytes,
            batch.vertices.len(),
            batch.fixed_constants,
            batch.is_2d,
            bindings,
            external,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn submit_draw(
        &mut self,
        spec: PipelineSpec,
        vertex_bytes: &[u8],
        vertex_count: usize,
        constants: CeylonSrdFixedShaderConstants,
        is_2d: bool,
        bindings: [DrawTextureBinding; 2],
        external: SrdExternalRenderState,
    ) -> Result<(), RenderBackendError> {
        let composition = self.composition.as_ref().ok_or_else(|| {
            RenderBackendError("WebGPU composition target is not configured".into())
        })?;
        if self.encoder.is_none() {
            return Err(RenderBackendError(
                "WebGPU draw was submitted outside a composition pass".into(),
            ));
        }
        let Some(scissor) = resolve_scissor(external, composition.size)? else {
            return Ok(());
        };

        self.ensure_pipeline(spec)?;
        for binding in bindings {
            self.ensure_sampler(binding.sampler)?;
        }

        let uniform_allocation =
            self.uniform_arena
                .allocate(&self.device, UNIFORM_DRAW_SIZE, 256)?;
        let vertex_allocation =
            self.vertex_arena
                .allocate(&self.device, vertex_bytes.len() as u64, 4)?;
        let (vertex_constants, pixel_constants) = serialize_constants(constants, is_2d);
        let uniform_chunk = &self.uniform_arena.chunks[uniform_allocation.chunk];
        self.queue.write_buffer(
            &uniform_chunk.buffer,
            uniform_allocation.range.start,
            &vertex_constants,
        );
        self.queue.write_buffer(
            &uniform_chunk.buffer,
            uniform_allocation.range.start + VERTEX_UNIFORM_SIZE,
            &pixel_constants,
        );
        let vertex_chunk = &self.vertex_arena.chunks[vertex_allocation.chunk];
        self.queue.write_buffer(
            &vertex_chunk.buffer,
            vertex_allocation.range.start,
            vertex_bytes,
        );

        let bind_group = self.create_bind_group(uniform_allocation.chunk, bindings)?;
        let pipeline = self
            .pipelines
            .get(&spec.key)
            .expect("pipeline was inserted by ensure_pipeline");
        let composition = self
            .composition
            .as_ref()
            .expect("composition was checked above");
        let encoder = self.encoder.as_mut().expect("encoder was checked above");
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Ceylon WebGPU draw"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &composition.color_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &composition.depth_stencil_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(
            0,
            &bind_group,
            &[
                u32::try_from(uniform_allocation.range.start).map_err(|_| {
                    RenderBackendError("WebGPU vertex uniform offset exceeds u32".into())
                })?,
                u32::try_from(uniform_allocation.range.start + VERTEX_UNIFORM_SIZE).map_err(
                    |_| RenderBackendError("WebGPU pixel uniform offset exceeds u32".into()),
                )?,
            ],
        );
        pass.set_vertex_buffer(0, vertex_chunk.buffer.slice(vertex_allocation.range));
        pass.set_scissor_rect(scissor[0], scissor[1], scissor[2], scissor[3]);
        pass.set_stencil_reference(spec.alpha_stencil.stencil_reference);
        pass.set_blend_constant(wgpu::Color::WHITE);
        pass.draw(
            0..u32::try_from(vertex_count)
                .map_err(|_| RenderBackendError("WebGPU vertex count exceeds u32".into()))?,
            0..1,
        );
        Ok(())
    }

    fn resolve_srd_textures(
        &mut self,
        source_bindings: [Option<EvidenceSrdTextureBinding>; 3],
        handle: Option<SrdTextureSetHandle>,
    ) -> Result<[DrawTextureBinding; 2], RenderBackendError> {
        if let Some(binding) = source_bindings[2] {
            return Err(RenderBackendError(format!(
                "WebGPU translated Ceylon shaders expose two texture slots, but SRD texture {} is bound to slot 2",
                binding.texture_index
            )));
        }
        let fallback_sampler = SamplerKey::fallback();
        self.ensure_sampler(fallback_sampler)?;
        let mut resolved = [
            DrawTextureBinding {
                texture: TextureSelection::Fallback,
                sampler: fallback_sampler,
            },
            DrawTextureBinding {
                texture: TextureSelection::Fallback,
                sampler: fallback_sampler,
            },
        ];
        for (slot, binding) in source_bindings[..2].iter().copied().enumerate() {
            let Some(binding) = binding else {
                continue;
            };
            let handle = handle.ok_or_else(|| {
                RenderBackendError(format!(
                    "WebGPU SRD texture {} is bound to slot {slot}, but no texture set was supplied",
                    binding.texture_index
                ))
            })?;
            let texture_set = self.texture_sets.get(&handle).ok_or_else(|| {
                RenderBackendError(format!(
                    "WebGPU SRD texture-set handle {handle:?} is not loaded"
                ))
            })?;
            if texture_set.get(binding.texture_index).is_none() {
                return Err(RenderBackendError(format!(
                    "WebGPU SRD texture index {} is not loaded for slot {slot}",
                    binding.texture_index
                )));
            }
            let sampler = SamplerKey::from_srd(binding.sampler);
            self.ensure_sampler(sampler)?;
            resolved[slot] = DrawTextureBinding {
                texture: TextureSelection::Srd {
                    handle,
                    index: binding.texture_index,
                },
                sampler,
            };
        }
        Ok(resolved)
    }

    fn ensure_sampler(&mut self, key: SamplerKey) -> Result<(), RenderBackendError> {
        if self.samplers.contains_key(&key) {
            return Ok(());
        }
        let descriptor = key.descriptor()?;
        let sampler = self.device.create_sampler(&descriptor);
        self.samplers.insert(key, sampler);
        Ok(())
    }

    fn create_bind_group(
        &self,
        uniform_chunk: usize,
        bindings: [DrawTextureBinding; 2],
    ) -> Result<wgpu::BindGroup, RenderBackendError> {
        let uniform = &self.uniform_arena.chunks[uniform_chunk].buffer;
        let views = [
            self.texture_view(bindings[0].texture)?,
            self.texture_view(bindings[1].texture)?,
        ];
        let samplers = [
            self.samplers.get(&bindings[0].sampler).ok_or_else(|| {
                RenderBackendError("WebGPU sampler slot 0 was not created".into())
            })?,
            self.samplers.get(&bindings[1].sampler).ok_or_else(|| {
                RenderBackendError("WebGPU sampler slot 1 was not created".into())
            })?,
        ];
        Ok(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Ceylon WebGPU draw bindings"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: uniform,
                        offset: 0,
                        size: NonZeroU64::new(VERTEX_UNIFORM_SIZE),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 16,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: uniform,
                        offset: 0,
                        size: NonZeroU64::new(PIXEL_UNIFORM_SIZE),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 32,
                    resource: wgpu::BindingResource::TextureView(views[0]),
                },
                wgpu::BindGroupEntry {
                    binding: 33,
                    resource: wgpu::BindingResource::Sampler(samplers[0]),
                },
                wgpu::BindGroupEntry {
                    binding: 34,
                    resource: wgpu::BindingResource::TextureView(views[1]),
                },
                wgpu::BindGroupEntry {
                    binding: 35,
                    resource: wgpu::BindingResource::Sampler(samplers[1]),
                },
            ],
        }))
    }

    fn texture_view(
        &self,
        selection: TextureSelection,
    ) -> Result<&wgpu::TextureView, RenderBackendError> {
        match selection {
            TextureSelection::Fallback => Ok(&self.fallback_texture.view),
            TextureSelection::Srd { handle, index } => self
                .texture_sets
                .get(&handle)
                .and_then(|textures| textures.get(index))
                .map(|texture| &texture.view)
                .ok_or_else(|| {
                    RenderBackendError(format!(
                        "WebGPU SRD texture {index} from handle {handle:?} is unavailable"
                    ))
                }),
            TextureSelection::Fennel { handle, page } => self
                .fennel_atlases
                .get(&handle)
                .and_then(|atlas| atlas.get(page))
                .map(|texture| &texture.view)
                .ok_or_else(|| {
                    RenderBackendError(format!(
                        "WebGPU Fennel page {page} from handle {handle:?} is unavailable"
                    ))
                }),
        }
    }

    fn ensure_pipeline(&mut self, spec: PipelineSpec) -> Result<(), RenderBackendError> {
        if self.pipelines.contains_key(&spec.key) {
            return Ok(());
        }
        let pipeline = self.create_pipeline(spec)?;
        self.pipelines.insert(spec.key, pipeline);
        Ok(())
    }

    fn create_pipeline(
        &self,
        spec: PipelineSpec,
    ) -> Result<wgpu::RenderPipeline, RenderBackendError> {
        let pair = embedded_wgpu_shader_pair(&spec.key.shader_key).ok_or_else(|| {
            RenderBackendError(format!(
                "WebGPU draw requested an untranslated Ceylon shader key {:?}",
                spec.key.shader_key
            ))
        })?;
        let vertex_source =
            vertex_source_with_d3d_color_swizzle(pair.vertex_source, pair.vertex_inputs)?;
        let pixel_source =
            pixel_source_with_sampler_biases(pair.pixel_source, spec.key.sampler_bias_bits)?;
        let pixel_source = pixel_source_with_alpha_test(
            &pixel_source,
            spec.alpha_stencil.alpha_function().ok_or_else(|| {
                RenderBackendError("WebGPU draw has an invalid alpha comparison".into())
            })?,
            spec.alpha_stencil.alpha_reference.min(255) as u8,
        )?;
        let attributes = vertex_attributes(pair, spec.key.input_layout)?;
        let stride = match spec.key.input_layout {
            InputLayout::Srd => SrdRenderVertex::STRIDE,
            InputLayout::Fennel => FennelRenderVertex::STRIDE,
        } as u64;
        let blend = blend_state(spec.blend);
        let polygon_mode = polygon_mode(spec.raster.fill_mode(), self.enabled_features)?;
        let (front_face, cull_mode) = cull_state(spec.raster.cull_mode().ok_or_else(|| {
            RenderBackendError("WebGPU draw has an invalid Ceylon cull mode".into())
        })?);
        let depth_compare = if spec.depth.z_enabled {
            compare_function(spec.depth.z_function().ok_or_else(|| {
                RenderBackendError("WebGPU draw has an invalid Ceylon depth comparison".into())
            })?)
        } else {
            wgpu::CompareFunction::Always
        };
        let stencil = stencil_state(spec.alpha_stencil)?;
        let write_mask = color_writes(spec.raster.color_write_mask);

        self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let vertex_module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("translated Ceylon vertex shader"),
                source: wgpu::ShaderSource::Wgsl(Cow::Owned(vertex_source)),
            });
        let pixel_module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("translated Ceylon pixel shader"),
                source: wgpu::ShaderSource::Wgsl(Cow::Owned(pixel_source)),
            });
        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Ceylon WebGPU render pipeline"),
                layout: Some(&self.pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &vertex_module,
                    entry_point: Some("main"),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: stride,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &attributes,
                    }],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &pixel_module,
                    entry_point: Some("main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: COLOR_FORMAT,
                        blend,
                        write_mask,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: match spec.key.topology {
                        DrawTopology::TriangleStrip => wgpu::PrimitiveTopology::TriangleStrip,
                        DrawTopology::TriangleList => wgpu::PrimitiveTopology::TriangleList,
                    },
                    strip_index_format: None,
                    front_face,
                    cull_mode,
                    unclipped_depth: false,
                    polygon_mode,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_STENCIL_FORMAT,
                    depth_write_enabled: spec.depth.z_enabled && spec.depth.z_write_enabled,
                    depth_compare,
                    stencil,
                    bias: wgpu::DepthBiasState {
                        constant: spec.key.depth_bias,
                        slope_scale: 0.0,
                        clamp: 0.0,
                    },
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });
        if let Some(error) = pollster::block_on(self.device.pop_error_scope()) {
            return Err(RenderBackendError(format!(
                "WebGPU rejected translated Ceylon shader pair VS{} / PS{}: {error}",
                pair.vertex_index, pair.pixel_index
            )));
        }
        Ok(pipeline)
    }
}

impl SrdRenderBackend for WgpuSrdRenderBackend {
    fn name(&self) -> &'static str {
        "WebGPU"
    }

    fn upload_srd_textures(
        &mut self,
        sources: SrdTextureSourceSet,
    ) -> Result<SrdTextureSetHandle, RenderBackendError> {
        let handle = SrdTextureSetHandle::new(self.next_texture_set);
        self.next_texture_set = self.next_texture_set.checked_add(1).ok_or_else(|| {
            RenderBackendError("WebGPU SRD texture-set handle space exhausted".into())
        })?;
        let textures = WgpuTextureSet::from_sources(&self.device, &self.queue, sources)?;
        self.texture_sets.insert(handle, textures);
        Ok(handle)
    }

    fn upload_fennel_atlas(
        &mut self,
        sources: FennelAtlasSourceSet,
    ) -> Result<FennelAtlasHandle, RenderBackendError> {
        let handle = FennelAtlasHandle::new(self.next_fennel_atlas);
        self.next_fennel_atlas = self.next_fennel_atlas.checked_add(1).ok_or_else(|| {
            RenderBackendError("WebGPU Fennel atlas handle space exhausted".into())
        })?;
        let atlas = WgpuFennelAtlas::from_sources(&self.device, &self.queue, sources)?;
        self.fennel_atlases.insert(handle, atlas);
        Ok(handle)
    }

    fn clear_uploaded_resources(&mut self) {
        self.texture_sets.clear();
        self.fennel_atlases.clear();
    }

    fn configure_composition_target(&mut self, size: [u32; 2]) -> Result<(), RenderBackendError> {
        if self.encoder.is_some() {
            return Err(RenderBackendError(
                "WebGPU composition target cannot be resized during a composition pass".into(),
            ));
        }
        if size[0] == 0 || size[1] == 0 {
            return Err(RenderBackendError(
                "WebGPU composition dimensions must be non-zero".into(),
            ));
        }
        self.composition = Some(CompositionTarget::new(&self.device, size));
        Ok(())
    }

    fn composition_size(&self) -> Option<[u32; 2]> {
        self.composition.as_ref().map(|target| target.size)
    }

    fn begin_composition(&mut self, clear_argb: u32) -> Result<(), RenderBackendError> {
        if self.encoder.is_some() {
            return Err(RenderBackendError(
                "WebGPU composition target is already active".into(),
            ));
        }
        let target = self.composition.as_ref().ok_or_else(|| {
            RenderBackendError("WebGPU composition target is not configured".into())
        })?;
        self.uniform_arena.reset();
        self.vertex_arena.reset();
        self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Ceylon WebGPU composition encoder"),
            });
        {
            let _clear = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Ceylon WebGPU composition clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.color_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(argb_color(clear_argb)),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &target.depth_stencil_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0),
                        store: wgpu::StoreOp::Store,
                    }),
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        self.encoder = Some(encoder);
        Ok(())
    }

    fn end_composition(&mut self) -> Result<(), RenderBackendError> {
        let encoder = self
            .encoder
            .take()
            .ok_or_else(|| RenderBackendError("WebGPU composition target is not active".into()))?;
        let submission = self.queue.submit([encoder.finish()]);
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .map_err(|error| {
                RenderBackendError(format!("WebGPU submission wait failed: {error}"))
            })?;
        if let Some(error) = pollster::block_on(self.device.pop_error_scope()) {
            return Err(RenderBackendError(format!(
                "WebGPU composition submission failed: {error}"
            )));
        }
        Ok(())
    }

    fn read_composition(&self) -> Result<CompositionReadback, RenderBackendError> {
        if self.encoder.is_some() {
            return Err(RenderBackendError(
                "WebGPU composition target is still active at readback".into(),
            ));
        }
        let target = self.composition.as_ref().ok_or_else(|| {
            RenderBackendError("WebGPU composition target is not configured".into())
        })?;
        let unpadded_row = target.size[0]
            .checked_mul(4)
            .ok_or_else(|| RenderBackendError("WebGPU readback row size overflow".into()))?;
        let padded_row = unpadded_row
            .checked_add(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT - 1)
            .map(|value| value & !(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT - 1))
            .ok_or_else(|| RenderBackendError("WebGPU padded row size overflow".into()))?;
        let buffer_size = u64::from(padded_row)
            .checked_mul(u64::from(target.size[1]))
            .ok_or_else(|| RenderBackendError("WebGPU readback buffer size overflow".into()))?;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Ceylon WebGPU composition readback"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Ceylon WebGPU readback encoder"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target.color,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row),
                    rows_per_image: Some(target.size[1]),
                },
            },
            wgpu::Extent3d {
                width: target.size[0],
                height: target.size[1],
                depth_or_array_layers: 1,
            },
        );
        let submission = self.queue.submit([encoder.finish()]);
        let buffer_slice = buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .map_err(|error| RenderBackendError(format!("WebGPU readback wait failed: {error}")))?;
        receiver
            .recv()
            .map_err(|error| RenderBackendError(format!("WebGPU map callback failed: {error}")))?
            .map_err(|error| {
                RenderBackendError(format!("WebGPU readback mapping failed: {error}"))
            })?;
        let mapped = buffer_slice.get_mapped_range();
        let pixel_count = usize::try_from(target.size[0])
            .ok()
            .and_then(|width| {
                usize::try_from(target.size[1])
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .ok_or_else(|| RenderBackendError("WebGPU readback pixel count overflow".into()))?;
        let byte_count = pixel_count
            .checked_mul(4)
            .ok_or_else(|| RenderBackendError("WebGPU readback byte count overflow".into()))?;
        let rgba = if padded_row == unpadded_row {
            mapped[..byte_count].to_vec()
        } else {
            let mut rgba = Vec::with_capacity(byte_count);
            for row in mapped
                .chunks_exact(padded_row as usize)
                .take(target.size[1] as usize)
            {
                rgba.extend_from_slice(&row[..unpadded_row as usize]);
            }
            rgba
        };
        drop(mapped);
        buffer.unmap();
        Ok(CompositionReadback {
            width: target.size[0],
            height: target.size[1],
            rgba,
        })
    }

    fn render_srd(
        &mut self,
        draws: &[EvidenceCompleteSrdDraw],
        external: SrdExternalRenderState,
        textures: Option<SrdTextureSetHandle>,
    ) -> Result<(), RenderBackendError> {
        for draw in draws {
            self.render_srd_vertices(draw, &draw.quad.vertices, external, textures)?;
        }
        Ok(())
    }

    fn render_srd_triangle_strip(
        &mut self,
        state: &EvidenceCompleteSrdDraw,
        vertices: &[SrdRenderVertex],
        external: SrdExternalRenderState,
        textures: Option<SrdTextureSetHandle>,
    ) -> Result<(), RenderBackendError> {
        self.render_srd_vertices(state, vertices, external, textures)
    }

    fn render_fennel(
        &mut self,
        batches: &[FennelRenderBatch<'_>],
        external: SrdExternalRenderState,
        atlas: FennelAtlasHandle,
    ) -> Result<(), RenderBackendError> {
        for batch in batches {
            self.render_fennel_batch(batch, external, atlas)?;
        }
        Ok(())
    }
}

impl CompositionTarget {
    fn new(device: &wgpu::Device, size: [u32; 2]) -> Self {
        let extent = wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        };
        let color = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Ceylon WebGPU composition color"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: COLOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let color_view = color.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_stencil = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Ceylon WebGPU composition depth/stencil"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_STENCIL_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_stencil_view = depth_stencil.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            color,
            color_view,
            depth_stencil,
            depth_stencil_view,
            size,
        }
    }
}

#[derive(Debug, Clone)]
struct GpuAllocation {
    chunk: usize,
    range: Range<u64>,
}

impl GpuArena {
    fn new(usage: wgpu::BufferUsages, label: &'static str) -> Self {
        Self {
            chunks: Vec::new(),
            usage,
            label,
        }
    }

    fn reset(&mut self) {
        for chunk in &mut self.chunks {
            chunk.used = 0;
        }
    }

    fn allocate(
        &mut self,
        device: &wgpu::Device,
        size: u64,
        alignment: u64,
    ) -> Result<GpuAllocation, RenderBackendError> {
        let aligned_size = align_up(size, alignment)?;
        for (index, chunk) in self.chunks.iter_mut().enumerate() {
            let start = align_up(chunk.used, alignment)?;
            let end = start
                .checked_add(aligned_size)
                .ok_or_else(|| RenderBackendError("WebGPU arena range overflow".into()))?;
            if end <= chunk.size {
                chunk.used = end;
                return Ok(GpuAllocation {
                    chunk: index,
                    range: start..start + size,
                });
            }
        }
        let chunk_size = align_up(aligned_size.max(GPU_ARENA_CHUNK_SIZE), alignment)?;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(self.label),
            size: chunk_size,
            usage: self.usage,
            mapped_at_creation: false,
        });
        self.chunks.push(GpuArenaChunk {
            buffer,
            size: chunk_size,
            used: aligned_size,
        });
        Ok(GpuAllocation {
            chunk: self.chunks.len() - 1,
            range: 0..size,
        })
    }
}

impl PipelineSpec {
    #[allow(clippy::too_many_arguments)]
    fn new(
        shader_key: [u8; CEYLON_SIMPLE_SHADER_KEY_LENGTH],
        input_layout: InputLayout,
        topology: DrawTopology,
        sampler_bias_bits: [u32; 2],
        blend: SrdD3d9BlendPreset,
        raster: CeylonRasterState,
        depth: CeylonDepthState,
        alpha_stencil: CeylonAlphaStencilState,
        depth_bias: i32,
    ) -> Result<Self, RenderBackendError> {
        let alpha_comparison = alpha_stencil.alpha_function().ok_or_else(|| {
            RenderBackendError("WebGPU draw has an invalid alpha comparison".into())
        })?;
        let depth_comparison = depth.z_function().ok_or_else(|| {
            RenderBackendError("WebGPU draw has an invalid depth comparison".into())
        })?;
        let cull_mode = raster
            .cull_mode()
            .ok_or_else(|| RenderBackendError("WebGPU draw has an invalid cull mode".into()))?;
        let (stencil_comparison, stencil_fail, stencil_depth_fail, stencil_pass) = if alpha_stencil
            .stencil_enabled
        {
            (
                alpha_stencil.stencil_function().ok_or_else(|| {
                    RenderBackendError("WebGPU draw has an invalid stencil comparison".into())
                })? as u32,
                alpha_stencil.stencil_fail().ok_or_else(|| {
                    RenderBackendError("WebGPU draw has an invalid stencil-fail operation".into())
                })? as u32,
                alpha_stencil.stencil_z_fail().ok_or_else(|| {
                    RenderBackendError(
                        "WebGPU draw has an invalid stencil depth-fail operation".into(),
                    )
                })? as u32,
                alpha_stencil.stencil_pass().ok_or_else(|| {
                    RenderBackendError("WebGPU draw has an invalid stencil-pass operation".into())
                })? as u32,
            )
        } else {
            (
                D3d9ComparisonFunction::Always as u32,
                D3d9StencilOperation::Keep as u32,
                D3d9StencilOperation::Keep as u32,
                D3d9StencilOperation::Keep as u32,
            )
        };
        let alpha_reference = if alpha_stencil.alpha_test_enabled {
            alpha_stencil.alpha_reference.min(255) as u8
        } else {
            0
        };
        let alpha_comparison = if alpha_stencil.alpha_test_enabled {
            alpha_comparison
        } else {
            D3d9ComparisonFunction::Always
        };
        Ok(Self {
            key: PipelineKey {
                shader_key,
                input_layout,
                topology,
                sampler_bias_bits,
                alpha_comparison: alpha_comparison as u32,
                alpha_reference,
                blend_enabled: blend.alpha_blend_enabled,
                source_blend: blend.source_blend as u32,
                destination_blend: blend.destination_blend as u32,
                blend_operation: blend.blend_operation as u32,
                separate_alpha_blend: blend.separate_alpha_blend_enabled,
                source_blend_alpha: blend.source_blend_alpha as u32,
                destination_blend_alpha: blend.destination_blend_alpha as u32,
                blend_operation_alpha: blend.blend_operation_alpha as u32,
                cull_mode: cull_mode as u32,
                fill_mode: raster.fill_mode() as u32,
                color_write_mask: raster.color_write_mask,
                depth_enabled: depth.z_enabled,
                depth_write_enabled: depth.z_write_enabled,
                depth_comparison: depth_comparison as u32,
                stencil_enabled: alpha_stencil.stencil_enabled,
                stencil_comparison,
                stencil_fail,
                stencil_depth_fail,
                stencil_pass,
                stencil_read_mask: alpha_stencil.stencil_mask,
                stencil_write_mask: alpha_stencil.stencil_write_mask,
                depth_bias,
            },
            blend,
            raster,
            depth,
            alpha_stencil,
        })
    }
}

impl SamplerKey {
    fn fallback() -> Self {
        Self {
            address_u: TextureAddressMode::Clamp as u32,
            address_v: TextureAddressMode::Clamp as u32,
            min_filter: TextureFilter::Point as u32,
            mag_filter: TextureFilter::Point as u32,
            mip_filter: TextureFilter::Point as u32,
            lod_min_bits: 0.0f32.to_bits(),
            lod_max_bits: 0.0f32.to_bits(),
            anisotropy: 1,
        }
    }

    fn from_srd(state: TextureSamplerState) -> Self {
        Self {
            address_u: state.address_u as u32,
            address_v: state.address_v as u32,
            min_filter: state.min_filter as u32,
            mag_filter: state.mag_filter as u32,
            // The isolated D3D9 renderer leaves MIPFILTER at the device default,
            // whose value is NONE. Clamping WebGPU to LOD 0 preserves that contract.
            mip_filter: TextureFilter::Point as u32,
            lod_min_bits: 0.0f32.to_bits(),
            lod_max_bits: 0.0f32.to_bits(),
            anisotropy: 1,
        }
    }

    fn from_fennel(state: RuhunaD3d9SamplerState) -> Self {
        let anisotropy = u16::try_from(state.max_anisotropy.max(1)).unwrap_or(u16::MAX);
        let all_linear = state.base.min_filter == TextureFilter::Linear
            && state.base.mag_filter == TextureFilter::Linear
            && state.mip_filter == TextureFilter::Linear;
        Self {
            address_u: state.base.address_u as u32,
            address_v: state.base.address_v as u32,
            min_filter: state.base.min_filter as u32,
            mag_filter: state.base.mag_filter as u32,
            mip_filter: state.mip_filter as u32,
            lod_min_bits: (state.max_mip_level as f32).to_bits(),
            lod_max_bits: 32.0f32.to_bits(),
            anisotropy: if all_linear { anisotropy } else { 1 },
        }
    }

    fn descriptor(self) -> Result<wgpu::SamplerDescriptor<'static>, RenderBackendError> {
        Ok(wgpu::SamplerDescriptor {
            label: Some("Ceylon WebGPU sampler"),
            address_mode_u: address_mode(self.address_u)?,
            address_mode_v: address_mode(self.address_v)?,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: filter_mode(self.mag_filter)?,
            min_filter: filter_mode(self.min_filter)?,
            mipmap_filter: filter_mode(self.mip_filter)?,
            lod_min_clamp: f32::from_bits(self.lod_min_bits),
            lod_max_clamp: f32::from_bits(self.lod_max_bits),
            compare: None,
            anisotropy_clamp: self.anisotropy,
            border_color: None,
        })
    }
}

fn validate_embedded_shader_modules(device: &wgpu::Device) -> Result<(), RenderBackendError> {
    device.push_error_scope(wgpu::ErrorFilter::Validation);
    let _vertex_modules: Vec<_> = vertex_sources()
        .enumerate()
        .map(|(index, source)| {
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(&format!("translated Ceylon VS {index}")),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(source)),
            })
        })
        .collect();
    let _pixel_modules: Vec<_> = pixel_sources()
        .enumerate()
        .map(|(index, source)| {
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(&format!("translated Ceylon PS {index}")),
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(source)),
            })
        })
        .collect();
    if let Some(error) = pollster::block_on(device.pop_error_scope()) {
        return Err(RenderBackendError(format!(
            "WebGPU rejected the embedded Ceylon shader corpus: {error}"
        )));
    }
    Ok(())
}

fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Ceylon WebGPU bindings"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: NonZeroU64::new(VERTEX_UNIFORM_SIZE),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 16,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: NonZeroU64::new(PIXEL_UNIFORM_SIZE),
                },
                count: None,
            },
            texture_layout_entry(32),
            sampler_layout_entry(33),
            texture_layout_entry(34),
            sampler_layout_entry(35),
        ],
    })
}

fn texture_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn sampler_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

fn vertex_attributes(
    pair: WgpuShaderPair,
    input_layout: InputLayout,
) -> Result<Vec<wgpu::VertexAttribute>, RenderBackendError> {
    pair.vertex_inputs
        .iter()
        .copied()
        .map(|input| vertex_attribute(input, input_layout))
        .collect()
}

fn vertex_attribute(
    input: WgpuVertexInput,
    input_layout: InputLayout,
) -> Result<wgpu::VertexAttribute, RenderBackendError> {
    let (offset, format) = match (input_layout, input.semantic) {
        (_, WgpuVertexSemantic::Position) => (0, wgpu::VertexFormat::Float32x3),
        (InputLayout::Srd, WgpuVertexSemantic::Color0)
        | (InputLayout::Fennel, WgpuVertexSemantic::Color0) => (12, wgpu::VertexFormat::Unorm8x4),
        (InputLayout::Srd, WgpuVertexSemantic::Color1)
        | (InputLayout::Fennel, WgpuVertexSemantic::Color1) => (16, wgpu::VertexFormat::Unorm8x4),
        (InputLayout::Srd, WgpuVertexSemantic::TexCoord0)
        | (InputLayout::Fennel, WgpuVertexSemantic::TexCoord0) => {
            (20, wgpu::VertexFormat::Float32x2)
        }
        (InputLayout::Srd, WgpuVertexSemantic::TexCoord1) => (28, wgpu::VertexFormat::Float32x2),
        (layout, semantic) => {
            return Err(RenderBackendError(format!(
                "translated Ceylon vertex shader requires {semantic:?}, which {layout:?} vertices do not carry"
            )));
        }
    };
    Ok(wgpu::VertexAttribute {
        format,
        offset,
        shader_location: input.location,
    })
}

fn blend_state(blend: SrdD3d9BlendPreset) -> Option<wgpu::BlendState> {
    blend.alpha_blend_enabled.then(|| {
        let color = blend_component(
            blend.source_blend,
            blend.destination_blend,
            blend.blend_operation,
        );
        let alpha = if blend.separate_alpha_blend_enabled {
            blend_component(
                blend.source_blend_alpha,
                blend.destination_blend_alpha,
                blend.blend_operation_alpha,
            )
        } else {
            color
        };
        wgpu::BlendState { color, alpha }
    })
}

fn blend_component(
    source: D3d9BlendFactor,
    destination: D3d9BlendFactor,
    operation: D3d9BlendOperation,
) -> wgpu::BlendComponent {
    wgpu::BlendComponent {
        src_factor: blend_factor(source),
        dst_factor: blend_factor(destination),
        operation: blend_operation(operation),
    }
}

fn blend_factor(factor: D3d9BlendFactor) -> wgpu::BlendFactor {
    match factor {
        D3d9BlendFactor::Zero => wgpu::BlendFactor::Zero,
        D3d9BlendFactor::One => wgpu::BlendFactor::One,
        D3d9BlendFactor::SourceColor => wgpu::BlendFactor::Src,
        D3d9BlendFactor::InverseSourceColor => wgpu::BlendFactor::OneMinusSrc,
        D3d9BlendFactor::SourceAlpha => wgpu::BlendFactor::SrcAlpha,
        D3d9BlendFactor::InverseSourceAlpha => wgpu::BlendFactor::OneMinusSrcAlpha,
        D3d9BlendFactor::DestinationAlpha => wgpu::BlendFactor::DstAlpha,
        D3d9BlendFactor::InverseDestinationAlpha => wgpu::BlendFactor::OneMinusDstAlpha,
        D3d9BlendFactor::DestinationColor => wgpu::BlendFactor::Dst,
        D3d9BlendFactor::InverseDestinationColor => wgpu::BlendFactor::OneMinusDst,
        D3d9BlendFactor::SourceAlphaSaturate => wgpu::BlendFactor::SrcAlphaSaturated,
        D3d9BlendFactor::BlendFactor => wgpu::BlendFactor::Constant,
        D3d9BlendFactor::InverseBlendFactor => wgpu::BlendFactor::OneMinusConstant,
    }
}

fn blend_operation(operation: D3d9BlendOperation) -> wgpu::BlendOperation {
    match operation {
        D3d9BlendOperation::Add => wgpu::BlendOperation::Add,
        D3d9BlendOperation::Subtract => wgpu::BlendOperation::Subtract,
        D3d9BlendOperation::ReverseSubtract => wgpu::BlendOperation::ReverseSubtract,
        D3d9BlendOperation::Minimum => wgpu::BlendOperation::Min,
        D3d9BlendOperation::Maximum => wgpu::BlendOperation::Max,
    }
}

fn compare_function(function: D3d9ComparisonFunction) -> wgpu::CompareFunction {
    match function {
        D3d9ComparisonFunction::Never => wgpu::CompareFunction::Never,
        D3d9ComparisonFunction::Less => wgpu::CompareFunction::Less,
        D3d9ComparisonFunction::Equal => wgpu::CompareFunction::Equal,
        D3d9ComparisonFunction::LessEqual => wgpu::CompareFunction::LessEqual,
        D3d9ComparisonFunction::Greater => wgpu::CompareFunction::Greater,
        D3d9ComparisonFunction::NotEqual => wgpu::CompareFunction::NotEqual,
        D3d9ComparisonFunction::GreaterEqual => wgpu::CompareFunction::GreaterEqual,
        D3d9ComparisonFunction::Always => wgpu::CompareFunction::Always,
    }
}

fn stencil_state(state: CeylonAlphaStencilState) -> Result<wgpu::StencilState, RenderBackendError> {
    if !state.stencil_enabled {
        return Ok(wgpu::StencilState::default());
    }
    let face = wgpu::StencilFaceState {
        compare: compare_function(state.stencil_function().ok_or_else(|| {
            RenderBackendError("WebGPU draw has an invalid stencil comparison".into())
        })?),
        fail_op: stencil_operation(state.stencil_fail().ok_or_else(|| {
            RenderBackendError("WebGPU draw has an invalid stencil-fail operation".into())
        })?),
        depth_fail_op: stencil_operation(state.stencil_z_fail().ok_or_else(|| {
            RenderBackendError("WebGPU draw has an invalid stencil depth-fail operation".into())
        })?),
        pass_op: stencil_operation(state.stencil_pass().ok_or_else(|| {
            RenderBackendError("WebGPU draw has an invalid stencil-pass operation".into())
        })?),
    };
    Ok(wgpu::StencilState {
        front: face,
        back: face,
        read_mask: state.stencil_mask,
        write_mask: state.stencil_write_mask,
    })
}

fn stencil_operation(operation: D3d9StencilOperation) -> wgpu::StencilOperation {
    match operation {
        D3d9StencilOperation::Keep => wgpu::StencilOperation::Keep,
        D3d9StencilOperation::Zero => wgpu::StencilOperation::Zero,
        D3d9StencilOperation::Replace => wgpu::StencilOperation::Replace,
        D3d9StencilOperation::IncrementSaturate => wgpu::StencilOperation::IncrementClamp,
        D3d9StencilOperation::DecrementSaturate => wgpu::StencilOperation::DecrementClamp,
        D3d9StencilOperation::Invert => wgpu::StencilOperation::Invert,
        D3d9StencilOperation::Increment => wgpu::StencilOperation::IncrementWrap,
        D3d9StencilOperation::Decrement => wgpu::StencilOperation::DecrementWrap,
    }
}

fn cull_state(mode: D3d9CullMode) -> (wgpu::FrontFace, Option<wgpu::Face>) {
    match mode {
        D3d9CullMode::None => (wgpu::FrontFace::Ccw, None),
        D3d9CullMode::Clockwise => (wgpu::FrontFace::Ccw, Some(wgpu::Face::Back)),
        D3d9CullMode::CounterClockwise => (wgpu::FrontFace::Cw, Some(wgpu::Face::Back)),
    }
}

fn polygon_mode(
    mode: D3d9FillMode,
    enabled_features: wgpu::Features,
) -> Result<wgpu::PolygonMode, RenderBackendError> {
    match mode {
        D3d9FillMode::Solid => Ok(wgpu::PolygonMode::Fill),
        D3d9FillMode::Wireframe if enabled_features.contains(wgpu::Features::POLYGON_MODE_LINE) => {
            Ok(wgpu::PolygonMode::Line)
        }
        D3d9FillMode::Point if enabled_features.contains(wgpu::Features::POLYGON_MODE_POINT) => {
            Ok(wgpu::PolygonMode::Point)
        }
        D3d9FillMode::Wireframe => Err(RenderBackendError(
            "WebGPU adapter does not support wireframe polygon mode".into(),
        )),
        D3d9FillMode::Point => Err(RenderBackendError(
            "WebGPU adapter does not support point polygon mode".into(),
        )),
    }
}

fn color_writes(mask: u32) -> wgpu::ColorWrites {
    let mut writes = wgpu::ColorWrites::empty();
    if mask & 1 != 0 {
        writes |= wgpu::ColorWrites::RED;
    }
    if mask & 2 != 0 {
        writes |= wgpu::ColorWrites::GREEN;
    }
    if mask & 4 != 0 {
        writes |= wgpu::ColorWrites::BLUE;
    }
    if mask & 8 != 0 {
        writes |= wgpu::ColorWrites::ALPHA;
    }
    writes
}

fn address_mode(value: u32) -> Result<wgpu::AddressMode, RenderBackendError> {
    match value {
        value if value == TextureAddressMode::Wrap as u32 => Ok(wgpu::AddressMode::Repeat),
        value if value == TextureAddressMode::Clamp as u32 => Ok(wgpu::AddressMode::ClampToEdge),
        _ => Err(RenderBackendError(format!(
            "WebGPU sampler uses unsupported D3D9 address mode {value}"
        ))),
    }
}

fn filter_mode(value: u32) -> Result<wgpu::FilterMode, RenderBackendError> {
    match value {
        value if value == TextureFilter::Point as u32 => Ok(wgpu::FilterMode::Nearest),
        value if value == TextureFilter::Linear as u32 => Ok(wgpu::FilterMode::Linear),
        _ => Err(RenderBackendError(format!(
            "WebGPU sampler uses unsupported D3D9 filter mode {value}"
        ))),
    }
}

fn serialize_constants(
    constants: CeylonSrdFixedShaderConstants,
    is_2d: bool,
) -> (
    [u8; VERTEX_UNIFORM_SIZE as usize],
    [u8; PIXEL_UNIFORM_SIZE as usize],
) {
    let mut vertex = [0u8; VERTEX_UNIFORM_SIZE as usize];
    let mut pixel = [0u8; PIXEL_UNIFORM_SIZE as usize];
    write_matrix_registers(&mut vertex, 0, constants.vertex_c0_c3_world.rows);
    write_matrix_registers(&mut vertex, 4, constants.vertex_c4_c7.rows);
    write_register(&mut vertex, 8, constants.vertex_c8_fixed_param0);
    write_register(&mut vertex, 9, constants.vertex_c9_fixed_param1);
    if is_2d {
        write_register(&mut vertex, 10, constants.vertex_c10_screen_param);
    } else {
        write_matrix_registers(
            &mut vertex,
            10,
            constants.vertex_c10_c13_projection_view.rows,
        );
    }
    write_register(&mut pixel, 0, constants.pixel_c0_fixed_param0);
    (vertex, pixel)
}

fn write_matrix_registers(destination: &mut [u8], first: usize, rows: [[f32; 4]; 4]) {
    for (index, row) in rows.into_iter().enumerate() {
        write_register(destination, first + index, row);
    }
}

fn write_register(destination: &mut [u8], register: usize, values: [f32; 4]) {
    let start = register * 16;
    for (index, value) in values.into_iter().enumerate() {
        destination[start + index * 4..start + index * 4 + 4]
            .copy_from_slice(&value.to_bits().to_le_bytes());
    }
}

fn argb_color(argb: u32) -> wgpu::Color {
    wgpu::Color {
        r: f64::from((argb >> 16) & 0xff) / 255.0,
        g: f64::from((argb >> 8) & 0xff) / 255.0,
        b: f64::from(argb & 0xff) / 255.0,
        a: f64::from(argb >> 24) / 255.0,
    }
}

fn resolve_scissor(
    external: SrdExternalRenderState,
    size: [u32; 2],
) -> Result<Option<[u32; 4]>, RenderBackendError> {
    if !external.scissor.enabled {
        return Ok(Some([0, 0, size[0], size[1]]));
    }
    let rectangle = external.scissor.rectangle;
    if rectangle.right < rectangle.left || rectangle.bottom < rectangle.top {
        return Err(RenderBackendError(format!(
            "WebGPU scissor is inverted: ({}, {})..({}, {})",
            rectangle.left, rectangle.top, rectangle.right, rectangle.bottom
        )));
    }
    let left = rectangle.left.max(0) as u32;
    let top = rectangle.top.max(0) as u32;
    let right = (rectangle.right.max(0) as u32).min(size[0]);
    let bottom = (rectangle.bottom.max(0) as u32).min(size[1]);
    if right <= left || bottom <= top {
        return Ok(None);
    }
    Ok(Some([left, top, right - left, bottom - top]))
}

fn align_up(value: u64, alignment: u64) -> Result<u64, RenderBackendError> {
    debug_assert!(alignment.is_power_of_two());
    value
        .checked_add(alignment - 1)
        .map(|value| value & !(alignment - 1))
        .ok_or_else(|| RenderBackendError("WebGPU aligned size overflow".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::identity_matrix4x4_game;
    use crate::reference_runtime::ReferenceLayerParent;
    use crate::render::{CeylonDrawPacketPresetState, D3d9PrimitiveType, SrdQuadDraw};
    use crate::renderer::backend::render_to_composition;
    use crate::scene::ReferenceTarget;

    #[test]
    fn translated_untextured_shader_renders_through_webgpu() {
        std::thread::Builder::new()
            .name("WebGPU shader smoke".into())
            .stack_size(16 * 1024 * 1024)
            .spawn(translated_untextured_shader_renders_on_large_stack)
            .unwrap()
            .join()
            .unwrap();
    }

    fn translated_untextured_shader_renders_on_large_stack() {
        let Ok(mut backend) = WgpuSrdRenderBackend::new() else {
            eprintln!("WebGPU adapter unavailable; skipping hardware smoke test");
            return;
        };
        #[cfg(target_os = "macos")]
        assert!(
            backend.adapter_diagnostics().ends_with(" / Metal"),
            "macOS preview must use Metal, got {}",
            backend.adapter_diagnostics()
        );
        backend.configure_composition_target([64, 64]).unwrap();
        let color = [30, 20, 10, 255];
        let vertex = |position| SrdRenderVertex {
            position,
            primary_color: color,
            secondary_color: [0; 4],
            texture_coordinates: [[0.0; 2]; 2],
        };
        let draw = EvidenceCompleteSrdDraw {
            owner: ReferenceLayerParent::ProjectLayer(ReferenceTarget {
                scene_index: 0,
                layer_index: 0,
            }),
            scene_index: 0,
            layer_index: 0,
            node_index: 0,
            is_2d: false,
            renderer_layer_key: 0,
            shader_key: *b"AACAABBAAAGAAAAAAA",
            quad: SrdQuadDraw {
                primitive_type: D3d9PrimitiveType::TriangleStrip,
                vertices: [
                    vertex([-0.5, -0.5, 0.5]),
                    vertex([-0.5, 0.5, 0.5]),
                    vertex([0.5, -0.5, 0.5]),
                    vertex([0.5, 0.5, 0.5]),
                ],
            },
            packet: CeylonDrawPacketPresetState::default(),
            fixed_constants: CeylonSrdFixedShaderConstants::initial_for_target(
                identity_matrix4x4_game(),
                [64, 64],
            ),
            blend: ceylon_d3d9_blend_preset(1),
            raster: CeylonRasterState {
                cull_mode_internal: 2,
                ..CeylonRasterState::default()
            },
            depth: CeylonDepthState::from_draw_flags(0),
            texture_bindings: [None; 3],
        };

        render_to_composition(&mut backend, 0xFF00_0000, |backend| {
            backend.render_srd(&[draw], SrdExternalRenderState::without_scissor(), None)
        })
        .unwrap();
        let readback = backend.read_composition().unwrap();
        let center = ((32 * readback.width + 32) * 4) as usize;
        assert_eq!(
            &readback.rgba[center..center + 4],
            &[color[2], color[1], color[0], color[3]]
        );
    }
}
