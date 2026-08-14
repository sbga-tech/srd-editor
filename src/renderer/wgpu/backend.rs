use std::collections::BTreeMap;
use std::mem;
use std::num::NonZeroU64;
use std::ops::Range;
use std::slice;

use crate::fennel::FennelRenderVertex;
use crate::renderer::backend::{
    CompositionReadback, FennelAtlasHandle, RenderBackendError, ScissorRect, SimpleDraw,
    SimpleTextureSource, SimpleVertices, SrdExternalRenderState, SrdRenderBackend,
    SrdTextureSetHandle,
};
use crate::renderer::pipeline::{
    BlendComponent, BlendFactor, BlendOperation, CompareFunction, CullMode, DepthState,
    DrawTopology, FillMode, RasterState, SimpleShaderProgram, SimpleVertexLayout, SrdDrawState,
    SrdTextureBinding, SrdVertex, StencilOperation, StencilState,
};
use crate::renderer::resources::{FennelAtlasSourceSet, SrdTextureSourceSet};
use crate::ruhuna::RuhunaSamplerState;
use crate::texture::{TextureAddressMode, TextureFilter, TextureSamplerState};

use super::texture::{WgpuFennelAtlas, WgpuTexture2d, WgpuTextureSet};
const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const DEPTH_STENCIL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24PlusStencil8;
const DRAW_UNIFORM_SIZE: u64 = mem::size_of::<DrawUniforms>() as u64;
const GPU_ARENA_CHUNK_SIZE: u64 = 2 * 1024 * 1024;

const SIMPLE_SHADER: &str = include_str!("shaders/simple.wgsl");

pub struct WgpuSrdRenderBackend {
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter_diagnostics: String,
    enabled_features: wgpu::Features,
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    simple_shader: wgpu::ShaderModule,
    backdrop_sampler: wgpu::Sampler,
    pipelines: BTreeMap<PipelineKey, wgpu::RenderPipeline>,
    samplers: BTreeMap<SamplerKey, wgpu::Sampler>,
    bind_groups: BTreeMap<BindGroupKey, wgpu::BindGroup>,
    fallback_texture: WgpuTexture2d,
    texture_sets: BTreeMap<SrdTextureSetHandle, WgpuTextureSet>,
    fennel_atlases: BTreeMap<FennelAtlasHandle, WgpuFennelAtlas>,
    next_texture_set: u64,
    next_fennel_atlas: u64,
    composition: Option<CompositionTarget>,
    encoder: Option<wgpu::CommandEncoder>,
    clear_color: Option<wgpu::Color>,
    pending_draws: Vec<PendingDraw>,
    uniform_arena: GpuArena,
    vertex_arena: GpuArena,
}

struct CompositionTarget {
    color: wgpu::Texture,
    color_view: wgpu::TextureView,
    backdrop: wgpu::Texture,
    backdrop_view: wgpu::TextureView,
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
struct PipelineKey {
    program: SimpleShaderProgram,
    vertex_layout: SimpleVertexLayout,
    topology: DrawTopology,
    blend: crate::renderer::pipeline::BlendState,
    raster: RasterState,
    depth: DepthState,
    stencil: StencilState,
}

impl PipelineKey {
    fn from_state(state: SrdDrawState, topology: DrawTopology) -> Self {
        Self {
            program: state.profile.program(),
            vertex_layout: state.profile.vertex_layout,
            topology,
            blend: state.pipeline.blend,
            raster: state.pipeline.raster,
            depth: state.pipeline.depth,
            stencil: state.pipeline.stencil,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct SamplerKey {
    address_u: TextureAddressMode,
    address_v: TextureAddressMode,
    min_filter: TextureFilter,
    mag_filter: TextureFilter,
    mip_filter: TextureFilter,
    lod_min_bits: u32,
    lod_max_bits: u32,
    anisotropy: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct DrawTextureBinding {
    texture: TextureSelection,
    sampler: SamplerKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct BindGroupKey {
    uniform_chunk: usize,
    bindings: [DrawTextureBinding; 2],
}

struct PreparedDraw<'a> {
    state: SrdDrawState,
    topology: DrawTopology,
    vertex_bytes: &'a [u8],
    vertex_count: usize,
    bindings: [DrawTextureBinding; 2],
    sampler_biases: [f32; 2],
    external: SrdExternalRenderState,
}

/// GPU resources and dynamic state for one draw in the composition's single
/// ordinary render pass. Pipelines, samplers, bind groups, uniforms, and
/// vertices are all prepared before the pass starts.
struct PendingDraw {
    pipeline: PipelineKey,
    bind_group: BindGroupKey,
    dynamic_offset: u32,
    vertex_chunk: usize,
    vertex_range: Range<u64>,
    vertex_count: u32,
    scissor: [u32; 4],
    stencil_reference: u32,
    requires_backdrop: bool,
}

fn target_color_segments(
    requires_backdrop: impl ExactSizeIterator<Item = bool>,
) -> impl Iterator<Item = Range<usize>> {
    let draw_count = requires_backdrop.len();
    requires_backdrop
        .enumerate()
        .filter_map(|(index, requires_backdrop)| requires_backdrop.then_some(index))
        .chain(std::iter::once(draw_count))
        .scan(0, |start, end| {
            let range = *start..end;
            *start = end;
            Some(range)
        })
}

/// Original Ceylon constant groups plus host-only fragment inputs. Profile
/// features are pipeline constants, never per-draw shader branches.
#[repr(C, align(16))]
struct DrawUniforms {
    /// Original SimpleShader `mtxWorld`, column-major for WGSL.
    world: [[f32; 4]; 4],
    /// Original SimpleShader `mtxPrjView`, column-major for WGSL.
    projection_view: [[f32; 4]; 4],
    /// `screenParam.xy`, followed by composition target dimensions.
    screen_target: [f32; 4],
    /// Sampler LOD biases, alpha reference, alpha comparison.
    params: [f32; 4],
}

impl DrawUniforms {
    fn from_draw(state: SrdDrawState, sampler_biases: [f32; 2], target_size: [u32; 2]) -> Self {
        let (alpha_comparison, alpha_reference) = match state.pipeline.alpha_test {
            Some(alpha) => (alpha.comparison as u32, alpha.reference),
            None => (CompareFunction::Always as u32, 0),
        };
        Self {
            world: matrix_columns(state.transform.world),
            projection_view: matrix_columns(state.transform.projection_view),
            screen_target: [
                state.transform.screen_param[0],
                state.transform.screen_param[1],
                target_size[0] as f32,
                target_size[1] as f32,
            ],
            params: [
                sampler_biases[0],
                sampler_biases[1],
                alpha_reference as f32,
                alpha_comparison as f32,
            ],
        }
    }

    fn as_bytes(&self) -> &[u8] {
        // SAFETY: DrawUniforms is repr(C), contains only plain numeric arrays, and
        // the returned bytes cannot outlive this value.
        unsafe {
            slice::from_raw_parts(
                (self as *const DrawUniforms).cast::<u8>(),
                mem::size_of::<DrawUniforms>(),
            )
        }
    }
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
        let optional_features =
            wgpu::Features::POLYGON_MODE_LINE | wgpu::Features::POLYGON_MODE_POINT;
        let enabled_features = adapter_features & optional_features;
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

        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let simple_shader = create_shader(&device, "SRD Ceylon SimpleShader", SIMPLE_SHADER);
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .map_err(|error| {
                RenderBackendError(format!("WebGPU shader validation wait failed: {error}"))
            })?;
        if let Some(error) = device.pop_error_scope().await {
            return Err(RenderBackendError(format!(
                "native WebGPU shader validation failed: {error}"
            )));
        }

        let bind_group_layout = create_bind_group_layout(&device);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SRD native pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        let fallback_texture = WgpuTexture2d::opaque_black(&device, &queue);
        let backdrop_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("SRD target-color sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 0.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });

        Ok(Self {
            device,
            queue,
            adapter_diagnostics,
            enabled_features,
            bind_group_layout,
            pipeline_layout,
            simple_shader,
            backdrop_sampler,
            pipelines: BTreeMap::new(),
            samplers: BTreeMap::new(),
            bind_groups: BTreeMap::new(),
            fallback_texture,
            texture_sets: BTreeMap::new(),
            fennel_atlases: BTreeMap::new(),
            next_texture_set: 1,
            next_fennel_atlas: 1,
            composition: None,
            encoder: None,
            clear_color: None,
            pending_draws: Vec::new(),
            uniform_arena: GpuArena::new(
                wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                "SRD native uniform arena",
            ),
            vertex_arena: GpuArena::new(
                wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                "SRD native vertex arena",
            ),
        })
    }

    pub fn adapter_diagnostics(&self) -> &str {
        &self.adapter_diagnostics
    }

    pub fn release_srd_textures(&mut self, handle: SrdTextureSetHandle) {
        self.bind_groups.retain(|key, _| {
            !key.bindings.iter().any(|binding| {
                matches!(binding.texture, TextureSelection::Srd { handle: bound, .. } if bound == handle)
            })
        });
        self.texture_sets.remove(&handle);
    }

    pub fn release_fennel_atlas(&mut self, handle: FennelAtlasHandle) {
        self.bind_groups.retain(|key, _| {
            !key.bindings.iter().any(|binding| {
                matches!(binding.texture, TextureSelection::Fennel { handle: bound, .. } if bound == handle)
            })
        });
        self.fennel_atlases.remove(&handle);
    }

    fn prepare_simple_draw<'a>(
        &self,
        draw: SimpleDraw<'a>,
    ) -> Result<PreparedDraw<'a>, RenderBackendError> {
        match (draw.topology, draw.vertices, draw.textures) {
            (
                DrawTopology::TriangleStrip,
                SimpleVertices::Format14(vertices),
                SimpleTextureSource::Srd(textures),
            ) => {
                if vertices.len() < 3 {
                    return Err(RenderBackendError(
                        "WebGPU format-14 triangle strip needs at least three vertices".into(),
                    ));
                }
                debug_assert_eq!(mem::size_of::<SrdVertex>(), SrdVertex::STRIDE);
                let byte_len = vertices
                    .len()
                    .checked_mul(SrdVertex::STRIDE)
                    .ok_or_else(|| {
                        RenderBackendError("WebGPU format-14 vertex upload size overflow".into())
                    })?;
                // SAFETY: SrdVertex is repr(C), its asserted stride covers every byte,
                // and this byte slice is bounded by the source vertex slice.
                let vertex_bytes =
                    unsafe { slice::from_raw_parts(vertices.as_ptr().cast::<u8>(), byte_len) };
                let bindings = self.resolve_srd_textures(draw.state.material.textures, textures)?;
                Ok(PreparedDraw {
                    state: draw.state,
                    topology: draw.topology,
                    vertex_bytes,
                    vertex_count: vertices.len(),
                    bindings,
                    sampler_biases: [0.0; 2],
                    external: draw.external,
                })
            }
            (
                DrawTopology::TriangleList,
                SimpleVertices::Format13(vertices),
                SimpleTextureSource::Fennel { atlas, page_index },
            ) => {
                if vertices.len() < 3 || !vertices.len().is_multiple_of(3) {
                    return Err(RenderBackendError(format!(
                        "WebGPU format-13 triangle list has invalid vertex count {}",
                        vertices.len()
                    )));
                }
                let atlas_resource = self.fennel_atlases.get(&atlas).ok_or_else(|| {
                    RenderBackendError(format!(
                        "WebGPU Fennel atlas handle {atlas:?} is not loaded"
                    ))
                })?;
                if atlas_resource.get(page_index).is_none() {
                    return Err(RenderBackendError(format!(
                        "WebGPU Fennel atlas page {page_index} is not loaded"
                    )));
                }
                let atlas_sampler = atlas_resource.sampler();
                let bindings = [
                    DrawTextureBinding {
                        texture: TextureSelection::Fennel {
                            handle: atlas,
                            page: page_index,
                        },
                        sampler: SamplerKey::from_fennel(atlas_sampler),
                    },
                    DrawTextureBinding {
                        texture: TextureSelection::Fallback,
                        sampler: SamplerKey::fallback(),
                    },
                ];
                debug_assert_eq!(
                    mem::size_of::<FennelRenderVertex>(),
                    FennelRenderVertex::STRIDE
                );
                let byte_len = vertices
                    .len()
                    .checked_mul(FennelRenderVertex::STRIDE)
                    .ok_or_else(|| {
                        RenderBackendError("WebGPU format-13 vertex upload size overflow".into())
                    })?;
                // SAFETY: FennelRenderVertex is repr(C), its asserted stride covers every
                // byte, and this byte slice is bounded by the source vertex slice.
                let vertex_bytes =
                    unsafe { slice::from_raw_parts(vertices.as_ptr().cast::<u8>(), byte_len) };
                Ok(PreparedDraw {
                    state: draw.state,
                    topology: draw.topology,
                    vertex_bytes,
                    vertex_count: vertices.len(),
                    bindings,
                    sampler_biases: [f32::from_bits(atlas_sampler.mip_lod_bias_bits), 0.0],
                    external: draw.external,
                })
            }
            _ => Err(RenderBackendError(
                "SimpleShader topology, vertex layout, and texture source do not match".into(),
            )),
        }
    }

    fn submit_draw(&mut self, draw: PreparedDraw<'_>) -> Result<(), RenderBackendError> {
        let composition_size = self
            .composition
            .as_ref()
            .ok_or_else(|| {
                RenderBackendError("WebGPU composition target is not configured".into())
            })?
            .size;
        if self.encoder.is_none() {
            return Err(RenderBackendError(
                "WebGPU draw was submitted outside a composition pass".into(),
            ));
        }
        let Some(scissor) = resolve_scissor(draw.external.scissor, composition_size)? else {
            return Ok(());
        };
        let pipeline = PipelineKey::from_state(draw.state, draw.topology);
        self.ensure_pipeline(pipeline)?;
        for binding in draw.bindings {
            self.ensure_sampler(binding.sampler)?;
        }

        let uniform_allocation =
            self.uniform_arena
                .allocate(&self.device, DRAW_UNIFORM_SIZE, 256)?;
        let vertex_allocation = self.vertex_arena.allocate(
            &self.device,
            u64::try_from(draw.vertex_bytes.len())
                .map_err(|_| RenderBackendError("WebGPU vertex byte count exceeds u64".into()))?,
            4,
        )?;
        let uniforms = DrawUniforms::from_draw(draw.state, draw.sampler_biases, composition_size);
        let uniform_chunk = &self.uniform_arena.chunks[uniform_allocation.chunk];
        self.queue.write_buffer(
            &uniform_chunk.buffer,
            uniform_allocation.range.start,
            uniforms.as_bytes(),
        );
        self.queue.write_buffer(
            &self.vertex_arena.chunks[vertex_allocation.chunk].buffer,
            vertex_allocation.range.start,
            draw.vertex_bytes,
        );

        let bind_group = BindGroupKey {
            uniform_chunk: uniform_allocation.chunk,
            bindings: draw.bindings,
        };
        self.ensure_bind_group(bind_group)?;
        self.pending_draws.push(PendingDraw {
            pipeline,
            bind_group,
            dynamic_offset: u32::try_from(uniform_allocation.range.start)
                .map_err(|_| RenderBackendError("WebGPU uniform offset exceeds u32".into()))?,
            vertex_chunk: vertex_allocation.chunk,
            vertex_range: vertex_allocation.range,
            vertex_count: u32::try_from(draw.vertex_count)
                .map_err(|_| RenderBackendError("WebGPU vertex count exceeds u32".into()))?,
            scissor,
            stencil_reference: u32::from(draw.state.pipeline.stencil.reference),
            requires_backdrop: draw.state.profile.requires_backdrop(),
        });
        Ok(())
    }

    fn resolve_srd_textures(
        &self,
        source_bindings: [Option<SrdTextureBinding>; 3],
        handle: Option<SrdTextureSetHandle>,
    ) -> Result<[DrawTextureBinding; 2], RenderBackendError> {
        if let Some(binding) = source_bindings[2] {
            return Err(RenderBackendError(format!(
                "native surface shader exposes two texture slots, but texture {} is bound to slot 2",
                binding.texture_index
            )));
        }
        let fallback = DrawTextureBinding {
            texture: TextureSelection::Fallback,
            sampler: SamplerKey::fallback(),
        };
        let mut resolved = [fallback; 2];
        for (slot, binding) in source_bindings[..2].iter().copied().enumerate() {
            let Some(binding) = binding else {
                continue;
            };
            let handle = handle.ok_or_else(|| {
                RenderBackendError(format!(
                    "SRD texture {} is bound to slot {slot}, but no texture set was supplied",
                    binding.texture_index
                ))
            })?;
            let texture_set = self.texture_sets.get(&handle).ok_or_else(|| {
                RenderBackendError(format!("SRD texture-set handle {handle:?} is not loaded"))
            })?;
            if texture_set.get(binding.texture_index).is_none() {
                return Err(RenderBackendError(format!(
                    "SRD texture index {} is not loaded for slot {slot}",
                    binding.texture_index
                )));
            }
            resolved[slot] = DrawTextureBinding {
                texture: TextureSelection::Srd {
                    handle,
                    index: binding.texture_index,
                },
                sampler: SamplerKey::from_srd(binding.sampler),
            };
        }
        Ok(resolved)
    }

    fn ensure_sampler(&mut self, key: SamplerKey) -> Result<(), RenderBackendError> {
        if self.samplers.contains_key(&key) {
            return Ok(());
        }
        let sampler = self.device.create_sampler(&key.descriptor());
        self.samplers.insert(key, sampler);
        Ok(())
    }

    fn ensure_bind_group(&mut self, key: BindGroupKey) -> Result<(), RenderBackendError> {
        if self.bind_groups.contains_key(&key) {
            return Ok(());
        }
        let uniform = &self.uniform_arena.chunks[key.uniform_chunk].buffer;
        let views = [
            self.texture_view(key.bindings[0].texture)?,
            self.texture_view(key.bindings[1].texture)?,
        ];
        let samplers = [
            self.samplers
                .get(&key.bindings[0].sampler)
                .expect("sampler created before bind group"),
            self.samplers
                .get(&key.bindings[1].sampler)
                .expect("sampler created before bind group"),
        ];
        let backdrop_view = &self
            .composition
            .as_ref()
            .ok_or_else(|| {
                RenderBackendError("WebGPU composition target is not configured".into())
            })?
            .backdrop_view;
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SRD native draw bindings"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: uniform,
                        offset: 0,
                        size: NonZeroU64::new(DRAW_UNIFORM_SIZE),
                    }),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(views[0]),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(samplers[0]),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(views[1]),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(samplers[1]),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(backdrop_view),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(&self.backdrop_sampler),
                },
            ],
        });
        self.bind_groups.insert(key, bind_group);
        Ok(())
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
                        "SRD texture {index} from handle {handle:?} is unavailable"
                    ))
                }),
            TextureSelection::Fennel { handle, page } => self
                .fennel_atlases
                .get(&handle)
                .and_then(|atlas| atlas.get(page))
                .map(|texture| &texture.view)
                .ok_or_else(|| {
                    RenderBackendError(format!(
                        "Fennel atlas page {page} from handle {handle:?} is unavailable"
                    ))
                }),
        }
    }

    fn ensure_pipeline(&mut self, key: PipelineKey) -> Result<(), RenderBackendError> {
        if self.pipelines.contains_key(&key) {
            return Ok(());
        }
        let (entry_point, attributes, stride) = match key.vertex_layout {
            SimpleVertexLayout::Format14 => (
                "vs_format_14",
                &SURFACE_ATTRIBUTES[..],
                SrdVertex::STRIDE as u64,
            ),
            SimpleVertexLayout::Format13 => (
                "vs_format_13",
                &FENNEL_ATTRIBUTES[..],
                FennelRenderVertex::STRIDE as u64,
            ),
        };
        let (front_face, cull_mode) = cull_state(key.raster.cull);
        let polygon_mode = polygon_mode(key.raster.fill, self.enabled_features)?;
        let depth_stencil = depth_stencil_state(key.depth, key.stencil, key.raster.depth_bias);
        let constants = program_constants(key.program);
        let compilation_options = || wgpu::PipelineCompilationOptions {
            constants: &constants,
            zero_initialize_workgroup_memory: true,
        };
        self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let pipeline = self
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("SRD native render pipeline"),
                layout: Some(&self.pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &self.simple_shader,
                    entry_point: Some(entry_point),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: stride,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes,
                    }],
                    compilation_options: compilation_options(),
                },
                primitive: wgpu::PrimitiveState {
                    topology: match key.topology {
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
                depth_stencil: Some(depth_stencil),
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &self.simple_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: COLOR_FORMAT,
                        blend: blend_state(key.blend),
                        write_mask: color_writes(key.raster.color_write_mask),
                    })],
                    compilation_options: compilation_options(),
                }),
                multiview: None,
                cache: None,
            });
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .map_err(|error| RenderBackendError(format!("WebGPU pipeline wait failed: {error}")))?;
        if let Some(error) = pollster::block_on(self.device.pop_error_scope()) {
            return Err(RenderBackendError(format!(
                "native WebGPU pipeline creation failed: {error}"
            )));
        }
        self.pipelines.insert(key, pipeline);
        Ok(())
    }

    fn encode_draw_range(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &CompositionTarget,
        range: Range<usize>,
        clear_color: Option<wgpu::Color>,
    ) {
        let clears = clear_color.is_some();
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("SRD native composition segment"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target.color_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: match clear_color {
                        Some(color) => wgpu::LoadOp::Clear(color),
                        None => wgpu::LoadOp::Load,
                    },
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &target.depth_stencil_view,
                depth_ops: Some(wgpu::Operations {
                    load: if clears {
                        wgpu::LoadOp::Clear(1.0)
                    } else {
                        wgpu::LoadOp::Load
                    },
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: Some(wgpu::Operations {
                    load: if clears {
                        wgpu::LoadOp::Clear(0)
                    } else {
                        wgpu::LoadOp::Load
                    },
                    store: wgpu::StoreOp::Store,
                }),
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_blend_constant(wgpu::Color::WHITE);
        for draw in &self.pending_draws[range] {
            pass.set_pipeline(
                self.pipelines
                    .get(&draw.pipeline)
                    .expect("pending draw pipeline was prepared"),
            );
            pass.set_bind_group(
                0,
                self.bind_groups
                    .get(&draw.bind_group)
                    .expect("pending draw bind group was prepared"),
                &[draw.dynamic_offset],
            );
            pass.set_vertex_buffer(
                0,
                self.vertex_arena.chunks[draw.vertex_chunk]
                    .buffer
                    .slice(draw.vertex_range.clone()),
            );
            pass.set_scissor_rect(
                draw.scissor[0],
                draw.scissor[1],
                draw.scissor[2],
                draw.scissor[3],
            );
            pass.set_stencil_reference(draw.stencil_reference);
            pass.draw(0..draw.vertex_count, 0..1);
        }
    }
}

impl SrdRenderBackend for WgpuSrdRenderBackend {
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

    fn configure_composition_target(&mut self, size: [u32; 2]) -> Result<(), RenderBackendError> {
        if self.encoder.is_some() {
            return Err(RenderBackendError(
                "WebGPU composition target cannot be resized during a pass".into(),
            ));
        }
        if size.contains(&0) {
            return Err(RenderBackendError(
                "WebGPU composition dimensions must be non-zero".into(),
            ));
        }
        self.bind_groups.clear();
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
        self.composition.as_ref().ok_or_else(|| {
            RenderBackendError("WebGPU composition target is not configured".into())
        })?;
        self.uniform_arena.reset();
        self.vertex_arena.reset();
        self.pending_draws.clear();
        self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        self.encoder = Some(
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("SRD native composition encoder"),
                }),
        );
        self.clear_color = Some(argb_color(clear_argb));
        Ok(())
    }

    fn end_composition(&mut self) -> Result<(), RenderBackendError> {
        let mut encoder = self
            .encoder
            .take()
            .ok_or_else(|| RenderBackendError("WebGPU composition target is not active".into()))?;
        let clear_color = self.clear_color.take().ok_or_else(|| {
            RenderBackendError("WebGPU composition clear state is not active".into())
        })?;
        let target = self.composition.as_ref().ok_or_else(|| {
            RenderBackendError("WebGPU composition target is not configured".into())
        })?;

        let extent = wgpu::Extent3d {
            width: target.size[0],
            height: target.size[1],
            depth_or_array_layers: 1,
        };
        let mut first_segment = true;
        for range in
            target_color_segments(self.pending_draws.iter().map(|draw| draw.requires_backdrop))
        {
            let snapshot_before_next_draw = range.end < self.pending_draws.len();
            self.encode_draw_range(
                &mut encoder,
                target,
                range,
                first_segment.then_some(clear_color),
            );
            first_segment = false;
            if snapshot_before_next_draw {
                encoder.copy_texture_to_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &target.color,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::TexelCopyTextureInfo {
                        texture: &target.backdrop,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    extent,
                );
            }
        }
        self.pending_draws.clear();

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
            label: Some("SRD native composition readback"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("SRD native readback encoder"),
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
        let byte_count = usize::try_from(target.size[0])
            .ok()
            .and_then(|width| {
                usize::try_from(target.size[1])
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .and_then(|pixels| pixels.checked_mul(4))
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

    fn render_simple(&mut self, draw: SimpleDraw<'_>) -> Result<(), RenderBackendError> {
        let prepared = self.prepare_simple_draw(draw)?;
        self.submit_draw(prepared)
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
            label: Some("SRD native composition color"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: COLOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let color_view = color.create_view(&wgpu::TextureViewDescriptor::default());
        let backdrop = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("SRD native target-color snapshot"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: COLOR_FORMAT,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let backdrop_view = backdrop.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_stencil = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("SRD native composition depth/stencil"),
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
            backdrop,
            backdrop_view,
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

impl SamplerKey {
    const fn fallback() -> Self {
        Self {
            address_u: TextureAddressMode::Clamp,
            address_v: TextureAddressMode::Clamp,
            min_filter: TextureFilter::Point,
            mag_filter: TextureFilter::Point,
            mip_filter: TextureFilter::Point,
            lod_min_bits: 0.0f32.to_bits(),
            lod_max_bits: 0.0f32.to_bits(),
            anisotropy: 1,
        }
    }

    const fn from_srd(state: TextureSamplerState) -> Self {
        Self {
            address_u: state.address_u,
            address_v: state.address_v,
            min_filter: state.min_filter,
            mag_filter: state.mag_filter,
            mip_filter: TextureFilter::Point,
            lod_min_bits: 0.0f32.to_bits(),
            lod_max_bits: 0.0f32.to_bits(),
            anisotropy: 1,
        }
    }

    fn from_fennel(state: RuhunaSamplerState) -> Self {
        let anisotropy = u16::try_from(state.max_anisotropy.max(1)).unwrap_or(u16::MAX);
        let all_linear = state.base.min_filter == TextureFilter::Linear
            && state.base.mag_filter == TextureFilter::Linear
            && state.mip_filter == TextureFilter::Linear;
        Self {
            address_u: state.base.address_u,
            address_v: state.base.address_v,
            min_filter: state.base.min_filter,
            mag_filter: state.base.mag_filter,
            mip_filter: state.mip_filter,
            lod_min_bits: (state.max_mip_level as f32).to_bits(),
            lod_max_bits: 32.0f32.to_bits(),
            anisotropy: if all_linear { anisotropy } else { 1 },
        }
    }

    fn descriptor(self) -> wgpu::SamplerDescriptor<'static> {
        wgpu::SamplerDescriptor {
            label: Some("SRD native sampler"),
            address_mode_u: address_mode(self.address_u),
            address_mode_v: address_mode(self.address_v),
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: filter_mode(self.mag_filter),
            min_filter: filter_mode(self.min_filter),
            mipmap_filter: filter_mode(self.mip_filter),
            lod_min_clamp: f32::from_bits(self.lod_min_bits),
            lod_max_clamp: f32::from_bits(self.lod_max_bits),
            compare: None,
            anisotropy_clamp: self.anisotropy,
            border_color: None,
        }
    }
}

fn create_shader(device: &wgpu::Device, label: &str, source: &'static str) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    })
}
fn matrix_columns(matrix: crate::projection::Matrix4x4) -> [[f32; 4]; 4] {
    let mut columns = [[0.0; 4]; 4];
    for (column, values) in columns.iter_mut().enumerate() {
        for (row, value) in values.iter_mut().enumerate() {
            *value = matrix.rows[row][column];
        }
    }
    columns
}

fn program_constants(program: SimpleShaderProgram) -> [(&'static str, f64); 4] {
    [
        ("SIMPLE_2D_TRANSFORM", program.transform_mode as u32 as f64),
        (
            "SIMPLE_TEXTURE_COUNT",
            f64::from(program.texture_count.get()),
        ),
        (
            "SIMPLE_MULTI_TEXTURE_MODE",
            program.multi_texture_mode as u32 as f64,
        ),
        ("SIMPLE_BLEND_MODE", f64::from(program.blend_mode.get())),
    ]
}

fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("SRD native bind-group layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: NonZeroU64::new(DRAW_UNIFORM_SIZE),
                },
                count: None,
            },
            texture_layout_entry(1),
            sampler_layout_entry(2),
            texture_layout_entry(3),
            sampler_layout_entry(4),
            texture_layout_entry(5),
            sampler_layout_entry(6),
        ],
    })
}

const fn texture_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
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

const fn sampler_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

const SURFACE_ATTRIBUTES: [wgpu::VertexAttribute; 5] = [
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 0,
        shader_location: 0,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Unorm8x4,
        offset: 12,
        shader_location: 1,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Unorm8x4,
        offset: 16,
        shader_location: 2,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x2,
        offset: 20,
        shader_location: 3,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x2,
        offset: 28,
        shader_location: 4,
    },
];

const FENNEL_ATTRIBUTES: [wgpu::VertexAttribute; 4] = [
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 0,
        shader_location: 0,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Unorm8x4,
        offset: 12,
        shader_location: 1,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Unorm8x4,
        offset: 16,
        shader_location: 2,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x2,
        offset: 20,
        shader_location: 3,
    },
];

fn blend_state(blend: crate::renderer::pipeline::BlendState) -> Option<wgpu::BlendState> {
    blend.enabled.then(|| wgpu::BlendState {
        color: blend_component(blend.color),
        alpha: blend_component(blend.alpha),
    })
}

fn blend_component(equation: BlendComponent) -> wgpu::BlendComponent {
    wgpu::BlendComponent {
        src_factor: blend_factor(equation.source),
        dst_factor: blend_factor(equation.destination),
        operation: blend_operation(equation.operation),
    }
}

const fn blend_factor(factor: BlendFactor) -> wgpu::BlendFactor {
    match factor {
        BlendFactor::Zero => wgpu::BlendFactor::Zero,
        BlendFactor::One => wgpu::BlendFactor::One,
        BlendFactor::SourceColor => wgpu::BlendFactor::Src,
        BlendFactor::OneMinusSourceColor => wgpu::BlendFactor::OneMinusSrc,
        BlendFactor::SourceAlpha => wgpu::BlendFactor::SrcAlpha,
        BlendFactor::OneMinusSourceAlpha => wgpu::BlendFactor::OneMinusSrcAlpha,
        BlendFactor::DestinationAlpha => wgpu::BlendFactor::DstAlpha,
        BlendFactor::OneMinusDestinationAlpha => wgpu::BlendFactor::OneMinusDstAlpha,
        BlendFactor::DestinationColor => wgpu::BlendFactor::Dst,
        BlendFactor::OneMinusDestinationColor => wgpu::BlendFactor::OneMinusDst,
        BlendFactor::SourceAlphaSaturated => wgpu::BlendFactor::SrcAlphaSaturated,
        BlendFactor::Constant => wgpu::BlendFactor::Constant,
        BlendFactor::OneMinusConstant => wgpu::BlendFactor::OneMinusConstant,
    }
}

const fn blend_operation(operation: BlendOperation) -> wgpu::BlendOperation {
    match operation {
        BlendOperation::Add => wgpu::BlendOperation::Add,
        BlendOperation::Subtract => wgpu::BlendOperation::Subtract,
        BlendOperation::ReverseSubtract => wgpu::BlendOperation::ReverseSubtract,
        BlendOperation::Minimum => wgpu::BlendOperation::Min,
        BlendOperation::Maximum => wgpu::BlendOperation::Max,
    }
}

const fn compare_function(comparison: CompareFunction) -> wgpu::CompareFunction {
    match comparison {
        CompareFunction::Never => wgpu::CompareFunction::Never,
        CompareFunction::Less => wgpu::CompareFunction::Less,
        CompareFunction::Equal => wgpu::CompareFunction::Equal,
        CompareFunction::LessEqual => wgpu::CompareFunction::LessEqual,
        CompareFunction::Greater => wgpu::CompareFunction::Greater,
        CompareFunction::NotEqual => wgpu::CompareFunction::NotEqual,
        CompareFunction::GreaterEqual => wgpu::CompareFunction::GreaterEqual,
        CompareFunction::Always => wgpu::CompareFunction::Always,
    }
}

fn depth_stencil_state(
    depth: DepthState,
    stencil: StencilState,
    depth_bias: i32,
) -> wgpu::DepthStencilState {
    let stencil = if stencil.enabled {
        let face = wgpu::StencilFaceState {
            compare: compare_function(stencil.face.comparison),
            fail_op: stencil_operation(stencil.face.fail),
            depth_fail_op: stencil_operation(stencil.face.depth_fail),
            pass_op: stencil_operation(stencil.face.pass),
        };
        wgpu::StencilState {
            front: face,
            back: face,
            read_mask: u32::from(stencil.read_mask),
            write_mask: u32::from(stencil.write_mask),
        }
    } else {
        wgpu::StencilState::default()
    };
    wgpu::DepthStencilState {
        format: DEPTH_STENCIL_FORMAT,
        depth_write_enabled: depth.enabled && depth.write_enabled,
        depth_compare: if depth.enabled {
            compare_function(depth.comparison)
        } else {
            wgpu::CompareFunction::Always
        },
        stencil,
        bias: wgpu::DepthBiasState {
            constant: depth_bias,
            slope_scale: 0.0,
            clamp: 0.0,
        },
    }
}

const fn stencil_operation(operation: StencilOperation) -> wgpu::StencilOperation {
    match operation {
        StencilOperation::Keep => wgpu::StencilOperation::Keep,
        StencilOperation::Zero => wgpu::StencilOperation::Zero,
        StencilOperation::Replace => wgpu::StencilOperation::Replace,
        StencilOperation::IncrementClamp => wgpu::StencilOperation::IncrementClamp,
        StencilOperation::DecrementClamp => wgpu::StencilOperation::DecrementClamp,
        StencilOperation::Invert => wgpu::StencilOperation::Invert,
        StencilOperation::IncrementWrap => wgpu::StencilOperation::IncrementWrap,
        StencilOperation::DecrementWrap => wgpu::StencilOperation::DecrementWrap,
    }
}

const fn cull_state(mode: CullMode) -> (wgpu::FrontFace, Option<wgpu::Face>) {
    match mode {
        CullMode::None => (wgpu::FrontFace::Ccw, None),
        CullMode::Clockwise => (wgpu::FrontFace::Ccw, Some(wgpu::Face::Back)),
        CullMode::CounterClockwise => (wgpu::FrontFace::Cw, Some(wgpu::Face::Back)),
    }
}

fn polygon_mode(
    mode: FillMode,
    enabled_features: wgpu::Features,
) -> Result<wgpu::PolygonMode, RenderBackendError> {
    match mode {
        FillMode::Solid => Ok(wgpu::PolygonMode::Fill),
        FillMode::Wireframe if enabled_features.contains(wgpu::Features::POLYGON_MODE_LINE) => {
            Ok(wgpu::PolygonMode::Line)
        }
        FillMode::Point if enabled_features.contains(wgpu::Features::POLYGON_MODE_POINT) => {
            Ok(wgpu::PolygonMode::Point)
        }
        FillMode::Wireframe => Err(RenderBackendError(
            "WebGPU adapter does not support wireframe polygon mode".into(),
        )),
        FillMode::Point => Err(RenderBackendError(
            "WebGPU adapter does not support point polygon mode".into(),
        )),
    }
}

fn color_writes(mask: u8) -> wgpu::ColorWrites {
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

const fn address_mode(mode: TextureAddressMode) -> wgpu::AddressMode {
    match mode {
        TextureAddressMode::Wrap => wgpu::AddressMode::Repeat,
        TextureAddressMode::Clamp => wgpu::AddressMode::ClampToEdge,
    }
}

const fn filter_mode(filter: TextureFilter) -> wgpu::FilterMode {
    match filter {
        TextureFilter::Point => wgpu::FilterMode::Nearest,
        TextureFilter::Linear => wgpu::FilterMode::Linear,
    }
}

fn resolve_scissor(
    scissor: Option<ScissorRect>,
    size: [u32; 2],
) -> Result<Option<[u32; 4]>, RenderBackendError> {
    let Some(rectangle) = scissor else {
        return Ok(Some([0, 0, size[0], size[1]]));
    };
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

fn argb_color(argb: u32) -> wgpu::Color {
    wgpu::Color {
        r: f64::from((argb >> 16) & 0xff) / 255.0,
        g: f64::from((argb >> 8) & 0xff) / 255.0,
        b: f64::from(argb & 0xff) / 255.0,
        a: f64::from(argb >> 24) / 255.0,
    }
}

fn align_up(value: u64, alignment: u64) -> Result<u64, RenderBackendError> {
    debug_assert!(alignment.is_power_of_two());
    value
        .checked_add(alignment - 1)
        .map(|value| value & !(alignment - 1))
        .ok_or_else(|| RenderBackendError("WebGPU aligned size overflow".into()))
}

#[cfg(all(test, target_os = "macos"))]
mod tests {

    #[test]
    fn target_color_segments_snapshot_immediately_before_each_dependent_draw() {
        let segments =
            target_color_segments([false, true, true, false].into_iter()).collect::<Vec<_>>();

        assert_eq!(segments, [0..1, 1..2, 2..4]);
    }

    #[test]
    fn target_color_segments_preserve_empty_prefix_for_first_dependent_draw() {
        let segments = target_color_segments([true, false].into_iter()).collect::<Vec<_>>();

        assert_eq!(segments, [0..0, 0..2]);
    }

    use super::*;
    use crate::reference_runtime::ReferenceLayerParent;
    use crate::renderer::backend::render_to_composition;
    use crate::renderer::pipeline::{
        DrawOrder, DrawOrigin, SrdDraw, SrdQuad, SrdTransform, build_base_pose_image_draws,
    };
    use crate::scene::ReferenceTarget;

    #[test]
    fn recovered_material_formulas_execute_on_gpu() {
        std::thread::Builder::new()
            .name("native WebGPU material formula regression".into())
            .stack_size(16 * 1024 * 1024)
            .spawn(recovered_material_formulas_execute_on_gpu_on_large_stack)
            .unwrap()
            .join()
            .unwrap();
    }

    fn recovered_material_formulas_execute_on_gpu_on_large_stack() {
        const RESULT_COUNT: u64 = 37;
        const VECTOR_SIZE: u64 = 16;
        let backend = WgpuSrdRenderBackend::new().unwrap();
        let source = format!(
            "{SIMPLE_SHADER}\n\
             @group(0) @binding(7)\n\
             var<storage, read_write> test_output: array<vec4<f32>, 37>;\n\
             @compute @workgroup_size(1)\n\
             fn test_material(@builtin(global_invocation_id) id: vec3<u32>) {{\n\
                 if id.x < 13u {{\n\
                     test_output[id.x] = combine_textures(\n\
                         vec4(0.2, 0.3, 0.4, 0.5),\n\
                         vec4(0.8, 0.7, 0.6, 0.25),\n\
                         id.x,\n\
                     );\n\
                 }} else if id.x < 37u {{\n\
                     test_output[id.x] = photoshop_layer_blend(\n\
                         vec4(0.2, 0.4, 0.7, 0.9),\n\
                         vec4(0.9, 0.2, 0.45, 0.6),\n\
                         id.x + 20u,\n\
                     );\n\
                 }}\n\
             }}\n"
        );
        backend
            .device
            .push_error_scope(wgpu::ErrorFilter::Validation);
        let module = backend
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("SRD multi-texture formula regression"),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
        let pipeline = backend
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("SRD multi-texture formula regression"),
                layout: None,
                module: &module,
                entry_point: Some("test_material"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });
        backend
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .unwrap();
        if let Some(error) = pollster::block_on(backend.device.pop_error_scope()) {
            panic!("multi-texture formula pipeline validation failed: {error}");
        }

        let byte_len = RESULT_COUNT * VECTOR_SIZE;
        let output = backend.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SRD multi-texture formula output"),
            size: byte_len,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback = backend.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SRD multi-texture formula readback"),
            size: byte_len,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let bind_group = backend
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("SRD multi-texture formula bind group"),
                layout: &pipeline.get_bind_group_layout(0),
                entries: &[wgpu::BindGroupEntry {
                    binding: 7,
                    resource: output.as_entire_binding(),
                }],
            });
        let mut encoder = backend
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("SRD multi-texture formula encoder"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("SRD multi-texture formula pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(RESULT_COUNT as u32, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, byte_len);
        let submission = backend.queue.submit([encoder.finish()]);
        let readback_slice = readback.slice(..);
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        readback_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        backend
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .unwrap();
        receiver.recv().unwrap().unwrap();
        let mapped = readback_slice.get_mapped_range();
        let actual = mapped
            .chunks_exact(VECTOR_SIZE as usize)
            .map(|vector| {
                std::array::from_fn(|component| {
                    let offset = component * 4;
                    f32::from_le_bytes(vector[offset..offset + 4].try_into().unwrap())
                })
            })
            .collect::<Vec<[f32; 4]>>();
        drop(mapped);
        readback.unmap();

        let expected = [
            [0.2, 0.3, 0.4, 0.5],
            [0.2, 0.3, 0.4, 0.5],
            [0.8, 0.7, 0.6, 0.25],
            [0.35, 0.4, 0.45, 0.4375],
            [1.0, 1.0, 1.0, 0.75],
            [-0.6, -0.4, -0.2, 0.25],
            [0.16, 0.21, 0.24, 0.250005],
            [0.8, 0.7, 0.6, 0.5],
            [0.2, 0.3, 0.4, 0.5],
            [0.8, 0.7, 0.6, 0.250005],
            [0.35, 0.4, 0.45, 0.5],
            [0.2, 0.3, 0.4, 0.8],
            [0.2, 0.3, 0.4, 0.4],
            [0.62, 0.28, 0.55, 0.72],
            [0.62, 0.4, 0.7, 0.9],
            [0.2, 0.28, 0.55, 0.72],
            [0.62, 0.28, 0.55, 0.72],
            [0.2, 0.4, 0.7, 0.9],
            [0.146_666_66, 0.16, 0.48, 0.86],
            [0.14, 0.16, 0.37, 0.66],
            [0.68, 0.46, 0.88, 0.96],
            [0.68, 0.52, 0.88, 0.96],
            [0.632, 0.472, 0.781, 0.936],
            [0.296, 0.256, 0.682, 0.912],
            [0.318_662_52, 0.3136, 0.6874, 0.905_842],
            [0.584, 0.256, 0.658, 0.912],
            [0.68, 0.16, 0.68, 0.96],
            [0.68, 0.16, 0.64, 0.96],
            [0.56, 0.4, 0.7, 0.9],
            [0.68, 0.16, 0.88, 0.96],
            [0.5, 0.28, 0.43, 0.54],
            [0.524, 0.424, 0.592, 0.612],
            [0.5, 0.28, 0.507_142_84, 0.72],
            [0.14, 0.388, 0.76, 0.72],
            [0.56, 0.22, 0.49, 0.72],
            [0.234_285_71, 0.468_571_42, 0.82, 0.72],
            [0.74, 0.52, 0.97, 1.0],
        ];
        assert_eq!(actual.len(), expected.len());
        for (case, (actual, expected)) in actual.iter().zip(expected).enumerate() {
            for (component, (actual, expected)) in actual.iter().zip(expected).enumerate() {
                assert!(
                    (actual - expected).abs() <= 2.0e-5,
                    "case {case} component {component}: expected {expected}, got {actual}"
                );
            }
        }
    }

    #[test]
    fn zero_multi_texture_mode_ignores_secondary_texture_on_gpu() {
        std::thread::Builder::new()
            .name("native WebGPU zero multi-texture regression".into())
            .stack_size(16 * 1024 * 1024)
            .spawn(zero_multi_texture_mode_ignores_secondary_texture_on_gpu_on_large_stack)
            .unwrap()
            .join()
            .unwrap();
    }

    fn zero_multi_texture_mode_ignores_secondary_texture_on_gpu_on_large_stack() {
        let mut backend = WgpuSrdRenderBackend::new().unwrap();
        backend.configure_composition_target([64, 64]).unwrap();
        let mut draw = solid_surface_draw(4, [255, 255, 255, 128], 0);
        let sampler = TextureSamplerState {
            address_u: TextureAddressMode::Clamp,
            address_v: TextureAddressMode::Clamp,
            min_filter: TextureFilter::Point,
            mag_filter: TextureFilter::Point,
        };
        draw.state.material.textures = [
            Some(SrdTextureBinding {
                texture_index: 0,
                sampler,
            }),
            Some(SrdTextureBinding {
                texture_index: 1,
                sampler,
            }),
            None,
        ];
        draw.state.profile = crate::renderer::pipeline::SimpleShaderProfile::surface(
            crate::renderer::pipeline::SimpleTransformMode::TwoDimensional,
            draw.state.material.textures,
            0,
            4,
        )
        .unwrap();
        let texture_handle = SrdTextureSetHandle::new(1);
        backend.texture_sets.insert(
            texture_handle,
            WgpuTextureSet::from_test_colors(
                &backend.device,
                &backend.queue,
                [[255, 0, 0, 255], [0, 0, 0, 0]],
            ),
        );

        render_to_composition(&mut backend, 0xff00_0000, |backend| {
            render_surface_draw(
                backend,
                &draw,
                &draw.geometry.vertices,
                Some(texture_handle),
            )
        })
        .unwrap();

        let frame = backend.read_composition().unwrap();
        let center = (32 * 64 + 32) * 4;
        let actual = &frame.rgba[center..center + 4];
        for (component, (actual, expected)) in actual.iter().zip([128u8, 0, 0, 255]).enumerate() {
            assert!(
                actual.abs_diff(expected) <= 1,
                "component {component}: expected {expected}, got {actual}"
            );
        }
    }

    #[test]
    fn raw_image_field_four_multiplies_base_alpha_by_secondary_red_on_gpu() {
        std::thread::Builder::new()
            .name("native WebGPU SrRmulA regression".into())
            .stack_size(16 * 1024 * 1024)
            .spawn(
                raw_image_field_four_multiplies_base_alpha_by_secondary_red_on_gpu_on_large_stack,
            )
            .unwrap()
            .join()
            .unwrap();
    }

    fn raw_image_field_four_multiplies_base_alpha_by_secondary_red_on_gpu_on_large_stack() {
        let mut backend = WgpuSrdRenderBackend::new().unwrap();
        backend.configure_composition_target([64, 64]).unwrap();
        let mut draw = solid_surface_draw(4, [255; 4], 0);
        let sampler = TextureSamplerState {
            address_u: TextureAddressMode::Clamp,
            address_v: TextureAddressMode::Clamp,
            min_filter: TextureFilter::Point,
            mag_filter: TextureFilter::Point,
        };
        draw.state.material.textures = [
            Some(SrdTextureBinding {
                texture_index: 0,
                sampler,
            }),
            Some(SrdTextureBinding {
                texture_index: 1,
                sampler,
            }),
            None,
        ];
        draw.state.profile = crate::renderer::pipeline::SimpleShaderProfile::surface(
            crate::renderer::pipeline::SimpleTransformMode::TwoDimensional,
            draw.state.material.textures,
            4,
            4,
        )
        .unwrap();
        let texture_handle = SrdTextureSetHandle::new(2);
        backend.texture_sets.insert(
            texture_handle,
            WgpuTextureSet::from_test_colors(
                &backend.device,
                &backend.queue,
                [[255, 0, 0, 255], [128, 0, 0, 255]],
            ),
        );

        render_to_composition(&mut backend, 0xff00_0000, |backend| {
            render_surface_draw(
                backend,
                &draw,
                &draw.geometry.vertices,
                Some(texture_handle),
            )
        })
        .unwrap();

        let frame = backend.read_composition().unwrap();
        let center = (32 * 64 + 32) * 4;
        let actual = &frame.rgba[center..center + 4];
        for (component, (actual, expected)) in actual.iter().zip([128u8, 0, 0, 255]).enumerate() {
            assert!(
                actual.abs_diff(expected) <= 1,
                "component {component}: expected {expected}, got {actual}"
            );
        }
    }

    #[test]
    fn native_surface_pipeline_renders_and_reads_back() {
        std::thread::Builder::new()
            .name("native WebGPU surface smoke".into())
            .stack_size(16 * 1024 * 1024)
            .spawn(native_surface_pipeline_renders_and_reads_back_on_large_stack)
            .unwrap()
            .join()
            .unwrap();
    }

    fn native_surface_pipeline_renders_and_reads_back_on_large_stack() {
        let mut backend = WgpuSrdRenderBackend::new().unwrap();
        backend.configure_composition_target([64, 64]).unwrap();

        let vertex = |position| SrdVertex {
            position,
            // Independent SRD vertices carry semantic RGBA components.
            primary_color: [255, 0, 0, 255],
            secondary_color: [0; 4],
            texture_coordinates: [[0.0; 2]; 2],
        };
        let vertices = [
            vertex([16.0, 16.0, 0.0]),
            vertex([16.0, 48.0, 0.0]),
            vertex([48.0, 16.0, 0.0]),
            vertex([48.0, 48.0, 0.0]),
        ];
        let mut renderer_counter = 0;
        let draw = SrdDraw {
            origin: DrawOrigin {
                owner: ReferenceLayerParent::ProjectLayer(ReferenceTarget {
                    scene_index: 0,
                    layer_index: 0,
                }),
                scene_index: 0,
                layer_index: 0,
                node_index: 0,
            },
            order: DrawOrder {
                renderer_layer_key: 0,
            },
            geometry: SrdQuad::new(vertices),
            state: SrdDrawState::surface(
                crate::renderer::pipeline::SrdSurfaceStateInput {
                    transform: SrdTransform::identity_2d([64, 64]),
                    transform_mode: crate::renderer::pipeline::SimpleTransformMode::TwoDimensional,
                    render_preset: 0,
                    image_field_0c: 0,
                    image_field_10: 0,
                    image_field_14: 0,
                    image_field_18: 0,
                    textures: [None; 3],
                },
                &mut renderer_counter,
            )
            .unwrap(),
        };

        render_to_composition(&mut backend, 0xff06_0b17, |backend| {
            render_surface_draw(backend, &draw, &vertices, None)
        })
        .unwrap();

        let frame = backend.read_composition().unwrap();
        assert_eq!(&frame.rgba[..4], &[6, 11, 23, 255]);
        let center = (32 * 64 + 32) * 4;
        assert_eq!(&frame.rgba[center..center + 4], &[255, 0, 0, 255]);
    }

    #[test]
    fn native_surface_pipeline_always_adds_secondary_vertex_color() {
        std::thread::Builder::new()
            .name("native WebGPU secondary vertex color regression".into())
            .stack_size(16 * 1024 * 1024)
            .spawn(native_surface_pipeline_always_adds_secondary_vertex_color_on_large_stack)
            .unwrap()
            .join()
            .unwrap();
    }

    fn native_surface_pipeline_always_adds_secondary_vertex_color_on_large_stack() {
        let mut backend = WgpuSrdRenderBackend::new().unwrap();
        backend.configure_composition_target([64, 64]).unwrap();
        let mut draw = solid_surface_draw(0, [64, 32, 16, 255], 0);
        for vertex in &mut draw.geometry.vertices {
            vertex.secondary_color = [40, 30, 20, 0];
        }

        render_to_composition(&mut backend, 0xff00_0000, |backend| {
            render_surface_draw(backend, &draw, &draw.geometry.vertices, None)
        })
        .unwrap();

        let frame = backend.read_composition().unwrap();
        let center = (32 * 64 + 32) * 4;
        let actual = &frame.rgba[center..center + 4];
        for (component, (actual, expected)) in actual.iter().zip([104u8, 62, 36, 255]).enumerate() {
            assert!(
                actual.abs_diff(expected) <= 1,
                "component {component}: expected {expected}, got {actual}"
            );
        }
    }

    #[test]
    fn render_preset_four_matches_ceylon_additive_blend_equation() {
        std::thread::Builder::new()
            .name("native WebGPU preset-4 blend regression".into())
            .stack_size(16 * 1024 * 1024)
            .spawn(render_preset_four_matches_ceylon_additive_blend_equation_on_large_stack)
            .unwrap()
            .join()
            .unwrap();
    }

    fn render_preset_four_matches_ceylon_additive_blend_equation_on_large_stack() {
        let mut backend = WgpuSrdRenderBackend::new().unwrap();
        backend.configure_composition_target([64, 64]).unwrap();
        let draw = solid_surface_draw(4, [128, 64, 32, 128], 0);

        render_to_composition(&mut backend, 0xff14_283c, |backend| {
            render_surface_draw(backend, &draw, &draw.geometry.vertices, None)
        })
        .unwrap();

        let frame = backend.read_composition().unwrap();
        let center = (32 * 64 + 32) * 4;
        let actual = &frame.rgba[center..center + 4];
        // Ceylon preset 4: src * SRCALPHA + dst * ONE. Because separate-alpha
        // blending is disabled, the same factors apply to alpha.
        let expected = [84u8, 72, 76, 255];
        for (component, (actual, expected)) in actual.iter().zip(expected).enumerate() {
            assert!(
                actual.abs_diff(expected) <= 1,
                "component {component}: expected {expected}, got {actual}"
            );
        }
    }

    #[test]
    fn opaque_fennel_atlas_preserves_glyph_alpha() {
        let actual = std::thread::Builder::new()
            .name("native WebGPU opaque Fennel regression".into())
            .stack_size(16 * 1024 * 1024)
            .spawn(|| render_fennel_solid_atlas_pixel_on_large_stack([255; 4]))
            .unwrap()
            .join()
            .unwrap();

        assert_eq!(actual, [0, 255, 0, 255]);
    }

    #[test]
    fn transparent_fennel_atlas_preserves_composition_alpha() {
        let actual = std::thread::Builder::new()
            .name("native WebGPU transparent Fennel regression".into())
            .stack_size(16 * 1024 * 1024)
            .spawn(|| render_fennel_solid_atlas_pixel_on_large_stack([255, 255, 255, 0]))
            .unwrap()
            .join()
            .unwrap();

        assert_eq!(actual, [6, 11, 23, 255]);
    }

    fn render_fennel_solid_atlas_pixel_on_large_stack(atlas_rgba: [u8; 4]) -> [u8; 4] {
        let mut backend = WgpuSrdRenderBackend::new().unwrap();
        backend.configure_composition_target([64, 64]).unwrap();
        let vertex = |position| FennelRenderVertex {
            position,
            primary_color_bgra: [0, 255, 0, 255],
            secondary_color_bgra: [0; 4],
            texture_coordinates: [0.0; 2],
        };
        let vertices = [
            vertex([16.0, 16.0, 0.0]),
            vertex([16.0, 48.0, 0.0]),
            vertex([48.0, 16.0, 0.0]),
            vertex([48.0, 16.0, 0.0]),
            vertex([16.0, 48.0, 0.0]),
            vertex([48.0, 48.0, 0.0]),
        ];
        let state = SrdDrawState::fennel(
            SrdTransform::identity_2d([64, 64]),
            crate::renderer::pipeline::SimpleTransformMode::TwoDimensional,
        );
        let atlas = FennelAtlasHandle::new(7);
        let sampler = RuhunaSamplerState {
            base: TextureSamplerState {
                address_u: TextureAddressMode::Clamp,
                address_v: TextureAddressMode::Clamp,
                min_filter: TextureFilter::Point,
                mag_filter: TextureFilter::Point,
            },
            mip_filter: TextureFilter::Point,
            max_mip_level: 0,
            max_anisotropy: 1,
            mip_lod_bias_bits: 0,
            border_color: 0,
        };
        backend.fennel_atlases.insert(
            atlas,
            WgpuFennelAtlas::from_test_texture(
                WgpuTexture2d::solid_rgba(&backend.device, &backend.queue, atlas_rgba),
                sampler,
            ),
        );

        render_to_composition(&mut backend, 0xff06_0b17, |backend| {
            backend.render_simple(SimpleDraw {
                state,
                topology: DrawTopology::TriangleList,
                vertices: SimpleVertices::Format13(&vertices),
                textures: SimpleTextureSource::Fennel {
                    atlas,
                    page_index: 0,
                },
                external: SrdExternalRenderState::without_scissor(),
            })
        })
        .unwrap();

        let frame = backend.read_composition().unwrap();
        let center = (32 * 64 + 32) * 4;
        frame.rgba[center..center + 4].try_into().unwrap()
    }

    #[test]
    fn dynamic_surface_draws_sample_the_ordered_backdrop() {
        std::thread::Builder::new()
            .name("native WebGPU target-color regression".into())
            .stack_size(16 * 1024 * 1024)
            .spawn(dynamic_surface_draws_sample_the_ordered_backdrop_on_large_stack)
            .unwrap()
            .join()
            .unwrap();
    }

    fn dynamic_surface_draws_sample_the_ordered_backdrop_on_large_stack() {
        let mut backend = WgpuSrdRenderBackend::new().unwrap();
        backend.configure_composition_target([64, 64]).unwrap();
        let draws = [
            solid_surface_draw(0, [51, 102, 204, 255], 0),
            solid_surface_draw(34, [204, 51, 102, 128], 1),
            solid_surface_draw(35, [26, 230, 77, 128], 2),
        ];

        render_to_composition(&mut backend, 0xff00_0000, |backend| {
            for draw in &draws {
                render_surface_draw(backend, draw, &draw.geometry.vertices, None)?;
            }
            Ok(())
        })
        .unwrap();

        let frame = backend.read_composition().unwrap();
        let center = (32 * 64 + 32) * 4;
        let actual = &frame.rgba[center..center + 4];
        let expected = [77u8, 102, 140, 191];
        for (component, (actual, expected)) in actual.iter().zip(expected).enumerate() {
            assert!(
                actual.abs_diff(expected) <= 2,
                "component {component}: expected {expected}, got {actual}"
            );
        }

        let first_draw = solid_surface_draw(37, [255, 0, 0, 255], 0);
        render_to_composition(&mut backend, 0xff03_0507, |backend| {
            render_surface_draw(backend, &first_draw, &first_draw.geometry.vertices, None)
        })
        .unwrap();
        let frame = backend.read_composition().unwrap();
        assert_eq!(&frame.rgba[center..center + 4], &[3, 5, 7, 255]);
    }

    #[test]
    fn target_color_effect_modes_execute_on_gpu() {
        std::thread::Builder::new()
            .name("native WebGPU target-color effects".into())
            .stack_size(16 * 1024 * 1024)
            .spawn(target_color_effect_modes_execute_on_gpu_on_large_stack)
            .unwrap()
            .join()
            .unwrap();
    }

    fn target_color_effect_modes_execute_on_gpu_on_large_stack() {
        let mut backend = WgpuSrdRenderBackend::new().unwrap();
        backend.configure_composition_target([64, 64]).unwrap();
        let sample_offset = (32 * 64 + 31) * 4;

        for (mode, alpha, expected) in [
            (57, 255, 99u8),
            (58, 255, 99),
            (59, 255, 128),
            (57, 128, 77),
            (59, 128, 128),
        ] {
            let backdrop = solid_surface_rect_draw(0, [255; 4], 0, [32.5, 0.5, 64.5, 64.5]);
            let effect = solid_surface_draw(mode, [255, 0, 0, alpha], 1);
            render_to_composition(&mut backend, 0xff00_0000, |backend| {
                for draw in [&backdrop, &effect] {
                    render_surface_draw(backend, draw, &draw.geometry.vertices, None)?;
                }
                Ok(())
            })
            .unwrap();
            let frame = backend.read_composition().unwrap();
            let actual = &frame.rgba[sample_offset..sample_offset + 4];
            for (component, actual) in actual[..3].iter().enumerate() {
                assert!(
                    actual.abs_diff(expected) <= 2,
                    "Gaussian mode {mode} alpha {alpha} component {component}: expected {expected}, got {actual}"
                );
            }
            assert_eq!(actual[3], 255);
        }

        let backdrop = solid_surface_rect_draw(0, [255; 4], 0, [32.5, 0.5, 64.5, 64.5]);
        let refraction = solid_surface_draw(60, [128, 128, 0, 255], 1);
        render_to_composition(&mut backend, 0xff00_0000, |backend| {
            for draw in [&backdrop, &refraction] {
                render_surface_draw(backend, draw, &draw.geometry.vertices, None)?;
            }
            Ok(())
        })
        .unwrap();
        let frame = backend.read_composition().unwrap();
        let refracted = &frame.rgba[sample_offset..sample_offset + 4];
        for (component, actual) in refracted[..3].iter().enumerate() {
            assert!(
                actual.abs_diff(64) <= 2,
                "refraction component {component}: expected 64, got {actual}"
            );
        }
        assert_eq!(refracted[3], 255);

        let backdrop = solid_surface_draw(0, [51, 102, 204, 255], 0);
        let fill = solid_surface_draw(61, [0, 0, 0, 128], 1);
        render_to_composition(&mut backend, 0xff00_0000, |backend| {
            for draw in [&backdrop, &fill] {
                render_surface_draw(backend, draw, &draw.geometry.vertices, None)?;
            }
            Ok(())
        })
        .unwrap();
        let frame = backend.read_composition().unwrap();
        let center = (32 * 64 + 32) * 4;
        let filled = &frame.rgba[center..center + 4];
        for (component, (actual, expected)) in filled.iter().zip([153u8, 179, 230, 191]).enumerate()
        {
            assert!(
                actual.abs_diff(expected) <= 2,
                "color-fill component {component}: expected {expected}, got {actual}"
            );
        }
    }

    #[test]
    fn renders_complete_dynamic_shader_fixture() {
        std::thread::Builder::new()
            .name("native WebGPU dynamic fixture".into())
            .stack_size(16 * 1024 * 1024)
            .spawn(renders_complete_dynamic_shader_fixture_on_large_stack)
            .unwrap()
            .join()
            .unwrap();
    }

    fn renders_complete_dynamic_shader_fixture_on_large_stack() {
        let Some(root) = std::env::var_os("GAME_DATA_CORPUS").map(std::path::PathBuf::from) else {
            eprintln!("skipping: GAME_DATA_CORPUS is not set");
            return;
        };
        let document = crate::document::EditorDocument::load(
            root.join("surfboard/shader/chu_ui_shader_extparam_00_v12.srd"),
        )
        .unwrap();
        let target_size = [1920, 1080];
        let snapshot = crate::game_host::WorldSnapshot::new(
            crate::transform::Affine3x4::IDENTITY,
            crate::game_host::SRD_RENDERER_INITIAL_LAYER_KEY,
            Some(crate::game_host::ProjectTargetSnapshot::new(
                crate::projection::identity_matrix4x4_game(),
                target_size,
            )),
            crate::projection::identity_matrix4x4_game(),
            target_size,
        );
        let mut draws =
            build_base_pose_image_draws(&document.project, &document.textures, 0, snapshot)
                .unwrap();
        assert_eq!(draws.len(), 54);
        let mut dynamic_modes = draws
            .iter()
            .map(|draw| draw.state.profile.blend_mode.get())
            .filter(|mode| *mode >= 33)
            .collect::<Vec<_>>();
        dynamic_modes.sort_unstable();
        dynamic_modes.dedup();
        assert_eq!(dynamic_modes, (34..=58).collect::<Vec<_>>());
        // The fixture's authored layer transform depends on a game-only host. Arrange the
        // recovered material inputs in a deterministic gallery so every mode reaches pixels.
        for (index, draw) in draws.iter_mut().enumerate() {
            let left = (index % 9) as f32 * 200.0 + 0.5;
            let top = (index / 9) as f32 * 180.0 + 0.5;
            let positions = [
                [left, top, 0.0],
                [left, top + 160.0, 0.0],
                [left + 180.0, top, 0.0],
                [left + 180.0, top + 160.0, 0.0],
            ];
            for (vertex, position) in draw.geometry.vertices.iter_mut().zip(positions) {
                vertex.position = position;
                vertex.primary_color = [255; 4];
                vertex.secondary_color = [0; 4];
            }
            draw.state.transform = SrdTransform::identity_2d(target_size);
        }

        let required_texture_indices = draws.iter().flat_map(|draw| {
            draw.state
                .material
                .textures
                .iter()
                .flatten()
                .map(|binding| binding.texture_index)
        });
        let texture_sources =
            SrdTextureSourceSet::load_required(&root, &document.textures, required_texture_indices)
                .unwrap();
        let mut backend = WgpuSrdRenderBackend::new().unwrap();
        let textures = backend.upload_srd_textures(texture_sources).unwrap();
        backend.configure_composition_target(target_size).unwrap();
        render_to_composition(&mut backend, 0xff06_0b17, |backend| {
            for draw in &draws {
                render_surface_draw(backend, draw, &draw.geometry.vertices, Some(textures))?;
            }
            Ok(())
        })
        .unwrap();

        let frame = backend.read_composition().unwrap();
        assert!(
            frame
                .rgba
                .chunks_exact(4)
                .any(|pixel| pixel != [6, 11, 23, 255]),
            "dynamic shader fixture rendered only the clear color"
        );
    }

    fn render_surface_draw(
        backend: &mut WgpuSrdRenderBackend,
        draw: &SrdDraw,
        vertices: &[SrdVertex],
        textures: Option<SrdTextureSetHandle>,
    ) -> Result<(), RenderBackendError> {
        backend.render_simple(SimpleDraw {
            state: draw.state,
            topology: DrawTopology::TriangleStrip,
            vertices: SimpleVertices::Format14(vertices),
            textures: SimpleTextureSource::Srd(textures),
            external: SrdExternalRenderState::without_scissor(),
        })
    }

    fn solid_surface_draw(preset: i32, rgba: [u8; 4], node_index: usize) -> SrdDraw {
        solid_surface_rect_draw(preset, rgba, node_index, [0.5, 0.5, 64.5, 64.5])
    }

    fn solid_surface_rect_draw(
        preset: i32,
        rgba: [u8; 4],
        node_index: usize,
        rectangle: [f32; 4],
    ) -> SrdDraw {
        let vertex = |position| SrdVertex {
            position,
            primary_color: rgba,
            secondary_color: [0; 4],
            texture_coordinates: [[0.0; 2]; 2],
        };
        let vertices = [
            vertex([rectangle[0], rectangle[1], 0.0]),
            vertex([rectangle[0], rectangle[3], 0.0]),
            vertex([rectangle[2], rectangle[1], 0.0]),
            vertex([rectangle[2], rectangle[3], 0.0]),
        ];
        let mut renderer_counter = 0;
        SrdDraw {
            origin: DrawOrigin {
                owner: ReferenceLayerParent::ProjectLayer(ReferenceTarget {
                    scene_index: 0,
                    layer_index: 0,
                }),
                scene_index: 0,
                layer_index: 0,
                node_index,
            },
            order: DrawOrder {
                renderer_layer_key: node_index as u32,
            },
            geometry: SrdQuad::new(vertices),
            state: SrdDrawState::surface(
                crate::renderer::pipeline::SrdSurfaceStateInput {
                    transform: SrdTransform::identity_2d([64, 64]),
                    transform_mode: crate::renderer::pipeline::SimpleTransformMode::TwoDimensional,
                    render_preset: preset,
                    image_field_0c: 0,
                    image_field_10: 0,
                    image_field_14: 0,
                    image_field_18: 0,
                    textures: [None; 3],
                },
                &mut renderer_counter,
            )
            .unwrap(),
        }
    }
}
