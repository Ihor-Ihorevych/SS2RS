//! The 3D world pass (RFC 0013, slice of T04-005/T04-009): textured static meshes drawn with one matrix per
//! instance, reverse-Z depth, flat lighting from the camera and alpha cutoff. The 2D pass draws on top.
//!
//! The renderer keeps no CPU copy of world data. When the GPU device is rebuilt (or [`crate::Renderer::clear_world`]
//! is called) every mesh, texture and batch is dropped and [`crate::Renderer::world_generation`] changes; the
//! owner of the level uploads it again. Handles from an older generation are refused.

use std::ops::Range;

use ss2_core::math::{Mat4, Quat, Vec3};
use ss2_render_api::Camera;
use ss2_render_api::scene_data::{CubeFace, MeshData, MeshVertex, TexelFormat, TextureData};
use wgpu::util::DeviceExt;

use crate::context::RenderError;

/// The depth buffer's format.
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// A mesh uploaded with [`crate::Renderer::create_mesh`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MeshHandle {
    index: u32,
    generation: u64,
}

/// A texture uploaded with [`crate::Renderer::create_texture`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextureHandle {
    index: u32,
    generation: u64,
}

/// One mesh drawn with one look at many places.
#[derive(Clone, Debug, PartialEq)]
pub struct WorldBatch {
    /// What to draw.
    pub mesh: MeshHandle,
    /// The base-colour texture; `None` draws `tint` alone.
    pub texture: Option<TextureHandle>,
    /// Multiplies the texture (linear RGBA).
    pub tint: [f32; 4],
    /// Texels whose alpha (after the tint) is below this are not drawn; `None` draws all.
    pub alpha_cutoff: Option<f32>,
    /// Model-to-world matrices, one per place.
    pub instances: Vec<Mat4>,
    /// A lightmap: the batch is drawn full bright (base colour times the lightmap sampled with the mesh's lightmap
    /// UVs, no light term; RFC 0013 amendment 2). Each lightmap belongs to one placement, so a lightmapped batch
    /// normally has one instance. `None`: lit by the flat light as before.
    pub lightmap: Option<TextureHandle>,
}

/// The level's sky (RFC 0013 amendment 1): one texture per [`CubeFace`] in its order, each image in the face's
/// orientation convention ([`CubeFace::basis`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkyBatch {
    /// The face textures in [`CubeFace::ALL`] order; `None` draws the face in `tint` alone.
    pub faces: [Option<TextureHandle>; 6],
    /// Linear RGBA multiplier on every face.
    pub tint: [f32; 4],
    /// How the cube is turned in the world; it keeps this orientation whichever way the camera looks.
    pub rotation: Quat,
    /// The level's cube edge length (only the ratio to `viewer_height` matters: the drawn cube has its own size).
    pub size: f32,
    /// The viewer's height inside the cube relative to its centre, in the units of `size` (RFC 0013 amendment 3).
    pub viewer_height: f32,
}

/// Half the size of the sky cube drawn around the camera (it never writes depth, so its size only has to clear the
/// near plane).
const SKY_HALF_SIZE: f32 = 10.0;

struct GpuSky {
    /// One material per face, in [`CubeFace::ALL`] order.
    faces: Vec<wgpu::BindGroup>,
    rotation: Quat,
    /// The viewer's height inside the drawn cube (the level's offset scaled to [`SKY_HALF_SIZE`]).
    viewer_offset: f32,
}

struct GpuMesh {
    vertices: wgpu::Buffer,
    /// One lightmap UV per vertex (zeros for a mesh without them).
    lightmap_uvs: wgpu::Buffer,
    indices: wgpu::Buffer,
    index_count: u32,
}

struct GpuTexture {
    view: wgpu::TextureView,
    repeat_u: bool,
    repeat_v: bool,
}

struct GpuBatch {
    mesh: u32,
    material: wgpu::BindGroup,
    instances: Range<u32>,
}

/// Everything the world pass owns on one device.
pub(crate) struct WorldGpu {
    pipeline: wgpu::RenderPipeline,
    /// The world shader without depth: the sky.
    sky_pipeline: wgpu::RenderPipeline,
    /// The sky cube: six quads, one per face in [`CubeFace::ALL`] order (6 indices each).
    sky_mesh: GpuMesh,
    /// The sky cube's matrix, written every frame (it follows the camera).
    sky_instance: wgpu::Buffer,
    sky: Option<GpuSky>,
    camera: wgpu::Buffer,
    camera_group: wgpu::BindGroup,
    material_layout: wgpu::BindGroupLayout,
    /// Indexed by `repeat_u as usize + 2 * repeat_v as usize`.
    samplers: [wgpu::Sampler; 4],
    white: wgpu::TextureView,
    meshes: Vec<GpuMesh>,
    textures: Vec<GpuTexture>,
    batches: Vec<GpuBatch>,
    instances: Option<wgpu::Buffer>,
    depth: Option<(wgpu::TextureView, (u32, u32))>,
    generation: u64,
    srgb: bool,
    bc: bool,
}

impl WorldGpu {
    /// Build the pipeline for a `format` target. `generation` starts the handle generation.
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        generation: u64,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("world"),
            source: wgpu::ShaderSource::Wgsl(include_str!("world.wgsl").into()),
        });
        let camera = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("world camera"),
            size: 80,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("world camera layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let camera_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("world camera"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.as_entire_binding(),
            }],
        });
        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("world material layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("world layout"),
            bind_group_layouts: &[Some(&camera_layout), Some(&material_layout)],
            immediate_size: 0,
        });
        let make_pipeline = |label: &str, depth_write: bool, compare: wgpu::CompareFunction| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[
                    Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<MeshVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2],
                    }),
                    Some(wgpu::VertexBufferLayout {
                        array_stride: 64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4],
                    }),
                    Some(wgpu::VertexBufferLayout {
                        array_stride: 8,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![7 => Float32x2],
                    }),
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            // No face culling: the winding of the game's meshes is not verified yet, and mirrored placements
            // reverse it.
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(depth_write),
                depth_compare: Some(compare),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
            })
        };
        let pipeline = make_pipeline("world", true, wgpu::CompareFunction::Greater);
        // The sky is drawn first, behind everything, and leaves the depth buffer to the world.
        let sky_pipeline = make_pipeline("sky", false, wgpu::CompareFunction::Always);
        let sky_mesh = sky_cube(device);
        let sky_instance = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sky instance"),
            size: 64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sampler = |repeat_u: bool, repeat_v: bool| {
            let mode = |repeat| {
                if repeat {
                    wgpu::AddressMode::Repeat
                } else {
                    wgpu::AddressMode::ClampToEdge
                }
            };
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("world sampler"),
                address_mode_u: mode(repeat_u),
                address_mode_v: mode(repeat_v),
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::MipmapFilterMode::Linear,
                anisotropy_clamp: 8,
                ..Default::default()
            })
        };
        let samplers = [
            sampler(false, false),
            sampler(true, false),
            sampler(false, true),
            sampler(true, true),
        ];
        let white = device
            .create_texture_with_data(
                queue,
                &wgpu::TextureDescriptor {
                    label: Some("world white texel"),
                    size: wgpu::Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                },
                wgpu::util::TextureDataOrder::LayerMajor,
                &[255, 255, 255, 255],
            )
            .create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            pipeline,
            sky_pipeline,
            sky_mesh,
            sky_instance,
            sky: None,
            camera,
            camera_group,
            material_layout,
            samplers,
            white,
            meshes: Vec::new(),
            textures: Vec::new(),
            batches: Vec::new(),
            instances: None,
            depth: None,
            generation,
            srgb: format.is_srgb(),
            bc: device
                .features()
                .contains(wgpu::Features::TEXTURE_COMPRESSION_BC),
        }
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn supports(&self, format: TexelFormat) -> bool {
        match format {
            TexelFormat::Rgba8 => true,
            TexelFormat::Bc1 | TexelFormat::Bc2 | TexelFormat::Bc3 => self.bc,
        }
    }

    /// Drop every mesh, texture and batch; older handles stop working.
    pub(crate) fn clear(&mut self) {
        self.meshes.clear();
        self.textures.clear();
        self.batches.clear();
        self.instances = None;
        self.sky = None;
        self.generation += 1;
    }

    pub(crate) fn create_mesh(
        &mut self,
        device: &wgpu::Device,
        mesh: &MeshData,
    ) -> Result<MeshHandle, RenderError> {
        mesh.validate()
            .map_err(|e| RenderError::World(e.to_string()))?;
        if mesh.indices.is_empty() {
            return Err(RenderError::World(
                "a mesh needs at least one triangle".into(),
            ));
        }
        let index_count = u32::try_from(mesh.indices.len())
            .map_err(|_| RenderError::World("too many indices".into()))?;
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("world mesh vertices"),
            contents: bytemuck::cast_slice(&mesh.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("world mesh indices"),
            contents: bytemuck::cast_slice(&mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let lightmap_uvs =
            lightmap_uv_buffer(device, mesh.lightmap_uvs.as_deref(), mesh.vertices.len());
        let index = u32::try_from(self.meshes.len())
            .map_err(|_| RenderError::World("too many meshes".into()))?;
        self.meshes.push(GpuMesh {
            vertices,
            lightmap_uvs,
            indices,
            index_count,
        });
        Ok(MeshHandle {
            index,
            generation: self.generation,
        })
    }

    pub(crate) fn create_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture: &TextureData,
    ) -> Result<TextureHandle, RenderError> {
        texture
            .validate()
            .map_err(|e| RenderError::World(e.to_string()))?;
        if !self.supports(texture.format) {
            return Err(RenderError::World(format!(
                "{:?} textures are not supported by this GPU",
                texture.format
            )));
        }
        let (w, h) = (texture.width, texture.height);
        let limit = device.limits().max_texture_dimension_2d;
        if w > limit || h > limit {
            return Err(RenderError::World(format!(
                "a {w}x{h} texture exceeds the limit of {limit}"
            )));
        }
        let block = texture.format != TexelFormat::Rgba8;
        if block && (!w.is_multiple_of(4) || !h.is_multiple_of(4)) {
            return Err(RenderError::World(format!(
                "a block-compressed texture must be a multiple of 4 in size, not {w}x{h}"
            )));
        }
        let format = match (texture.format, self.srgb) {
            (TexelFormat::Rgba8, false) => wgpu::TextureFormat::Rgba8Unorm,
            (TexelFormat::Rgba8, true) => wgpu::TextureFormat::Rgba8UnormSrgb,
            (TexelFormat::Bc1, false) => wgpu::TextureFormat::Bc1RgbaUnorm,
            (TexelFormat::Bc1, true) => wgpu::TextureFormat::Bc1RgbaUnormSrgb,
            (TexelFormat::Bc2, false) => wgpu::TextureFormat::Bc2RgbaUnorm,
            (TexelFormat::Bc2, true) => wgpu::TextureFormat::Bc2RgbaUnormSrgb,
            (TexelFormat::Bc3, false) => wgpu::TextureFormat::Bc3RgbaUnorm,
            (TexelFormat::Bc3, true) => wgpu::TextureFormat::Bc3RgbaUnormSrgb,
        };
        let mip_level_count = u32::try_from(texture.mips.len())
            .map_err(|_| RenderError::World("too many mip levels".into()))?;
        let data: Vec<u8> = texture.mips.concat();
        let gpu = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("world texture"),
                size: wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                mip_level_count,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &data,
        );
        let index = u32::try_from(self.textures.len())
            .map_err(|_| RenderError::World("too many textures".into()))?;
        self.textures.push(GpuTexture {
            view: gpu.create_view(&wgpu::TextureViewDescriptor::default()),
            repeat_u: texture.repeat_u,
            repeat_v: texture.repeat_v,
        });
        Ok(TextureHandle {
            index,
            generation: self.generation,
        })
    }

    fn mesh_index(&self, h: MeshHandle) -> Result<u32, RenderError> {
        if h.generation != self.generation || h.index as usize >= self.meshes.len() {
            return Err(RenderError::World(format!(
                "mesh {h:?} is stale or unknown (world generation {})",
                self.generation
            )));
        }
        Ok(h.index)
    }

    fn texture(&self, h: TextureHandle) -> Result<&GpuTexture, RenderError> {
        if h.generation != self.generation {
            return Err(RenderError::World(format!(
                "texture {h:?} is stale (world generation {})",
                self.generation
            )));
        }
        self.textures
            .get(h.index as usize)
            .ok_or_else(|| RenderError::World(format!("texture {h:?} is unknown")))
    }

    /// Replace what is drawn. Nothing changes if it returns an error.
    pub(crate) fn set_world(
        &mut self,
        device: &wgpu::Device,
        batches: Vec<WorldBatch>,
    ) -> Result<(), RenderError> {
        let mut matrices: Vec<f32> = Vec::new();
        let mut planned = Vec::with_capacity(batches.len());
        for (i, b) in batches.iter().enumerate() {
            let mesh = self.mesh_index(b.mesh)?;
            let texture = b.texture.map(|t| self.texture(t)).transpose()?;
            let lightmap = b.lightmap.map(|t| self.texture(t)).transpose()?;
            let cutoff = b.alpha_cutoff.unwrap_or(0.0);
            if !b.tint.iter().all(|c| c.is_finite()) || !cutoff.is_finite() {
                return Err(RenderError::World(format!(
                    "batch {i} has a non-finite tint or cutoff"
                )));
            }
            if !b.instances.iter().all(Mat4::is_finite) {
                return Err(RenderError::World(format!(
                    "batch {i} has a non-finite matrix"
                )));
            }
            let first = u32::try_from(matrices.len() / 16)
                .map_err(|_| RenderError::World("too many instances".into()))?;
            for m in &b.instances {
                matrices.extend_from_slice(&m.to_cols_array());
            }
            let last = u32::try_from(matrices.len() / 16)
                .map_err(|_| RenderError::World("too many instances".into()))?;
            if first == last {
                continue;
            }
            let (view, sampler) = match texture {
                Some(t) => (
                    &t.view,
                    &self.samplers[usize::from(t.repeat_u) + 2 * usize::from(t.repeat_v)],
                ),
                None => (&self.white, &self.samplers[3]),
            };
            let material = self.material(
                device,
                view,
                sampler,
                b.tint,
                cutoff,
                false,
                lightmap.map(|l| &l.view),
            );
            planned.push(GpuBatch {
                mesh,
                material,
                instances: first..last,
            });
        }
        self.instances = (!matrices.is_empty()).then(|| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("world instances"),
                contents: bytemuck::cast_slice(&matrices),
                usage: wgpu::BufferUsages::VERTEX,
            })
        });
        self.batches = planned;
        Ok(())
    }

    /// A material bind group: `view` sampled with `sampler`, times `tint`, texels below `cutoff` discarded; unlit,
    /// lightmapped (full bright with `lightmap`) or lit by the flat light.
    #[allow(clippy::too_many_arguments)]
    fn material(
        &self,
        device: &wgpu::Device,
        view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        tint: [f32; 4],
        cutoff: f32,
        unlit: bool,
        lightmap: Option<&wgpu::TextureView>,
    ) -> wgpu::BindGroup {
        let mut uniform = [0f32; 8];
        uniform[..4].copy_from_slice(&tint);
        uniform[4] = cutoff;
        uniform[5] = if unlit { 1.0 } else { 0.0 };
        uniform[6] = if lightmap.is_some() { 1.0 } else { 0.0 };
        let lightmap = lightmap.unwrap_or(&self.white);
        let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("world material"),
            contents: bytemuck::cast_slice(&uniform),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("world material"),
            layout: &self.material_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(lightmap),
                },
                // A lightmap belongs to one placement and never repeats.
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&self.samplers[0]),
                },
            ],
        })
    }

    /// Replace the sky (`None`: no sky, the clear colour shows). Nothing changes if it returns an error.
    pub(crate) fn set_sky(
        &mut self,
        device: &wgpu::Device,
        sky: Option<SkyBatch>,
    ) -> Result<(), RenderError> {
        let Some(sky) = sky else {
            self.sky = None;
            return Ok(());
        };
        if !sky.tint.iter().all(|c| c.is_finite()) {
            return Err(RenderError::World(format!("sky tint {:?}", sky.tint)));
        }
        if !sky.rotation.is_finite()
            || (sky.rotation.length_squared() - 1.0).abs() > 1e-3
            || !(sky.size.is_finite() && sky.size > 0.0)
            || !sky.viewer_height.is_finite()
        {
            return Err(RenderError::World(format!(
                "sky placement: rotation {:?}, size {}, viewer height {}",
                sky.rotation, sky.size, sky.viewer_height
            )));
        }
        let mut faces = Vec::with_capacity(6);
        for face in sky.faces {
            // Clamped, so a face's edge does not pick up the opposite edge.
            let view = match face {
                Some(t) => &self.texture(t)?.view,
                None => &self.white,
            };
            faces.push(self.material(device, view, &self.samplers[0], sky.tint, 0.0, true, None));
        }
        self.sky = Some(GpuSky {
            faces,
            rotation: sky.rotation.normalize(),
            viewer_offset: sky.viewer_height / sky.size * 2.0 * SKY_HALF_SIZE,
        });
        Ok(())
    }

    /// Clear `view` (a `size`-pixel target) to `clear` and draw the world as `camera` sees it.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn encode(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        size: (u32, u32),
        camera: &Camera,
        clear: wgpu::Color,
    ) {
        if self.depth.as_ref().is_none_or(|(_, s)| *s != size) {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("world depth"),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.depth = Some((
                texture.create_view(&wgpu::TextureViewDescriptor::default()),
                size,
            ));
        }
        let aspect = size.0 as f32 / size.1.max(1) as f32;
        let mut uniform = [0f32; 20];
        uniform[..16].copy_from_slice(&camera.view_projection(aspect).to_cols_array());
        uniform[16..19].copy_from_slice(&camera.forward().to_array());
        queue.write_buffer(&self.camera, 0, bytemuck::cast_slice(&uniform));
        if let Some(sky) = &self.sky {
            // Centred on the camera, except that the viewer sits `viewer_offset` above the cube's centre; turned by the
            // level's rotation; never closer whatever the camera does.
            let around_camera = Mat4::from_scale_rotation_translation(
                Vec3::splat(2.0 * SKY_HALF_SIZE),
                sky.rotation,
                camera.position - Vec3::Y * sky.viewer_offset,
            );
            queue.write_buffer(
                &self.sky_instance,
                0,
                bytemuck::cast_slice(&around_camera.to_cols_array()),
            );
        }
        let Some((depth, _)) = &self.depth else {
            return;
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("world"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    // Reverse-Z: far is 0.
                    load: wgpu::LoadOp::Clear(0.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        if let Some(sky) = &self.sky {
            pass.set_pipeline(&self.sky_pipeline);
            pass.set_bind_group(0, &self.camera_group, &[]);
            pass.set_vertex_buffer(0, self.sky_mesh.vertices.slice(..));
            pass.set_vertex_buffer(1, self.sky_instance.slice(..));
            pass.set_vertex_buffer(2, self.sky_mesh.lightmap_uvs.slice(..));
            pass.set_index_buffer(self.sky_mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            for (i, material) in (0u32..).zip(&sky.faces) {
                pass.set_bind_group(1, material, &[]);
                pass.draw_indexed(i * 6..i * 6 + 6, 0, 0..1);
            }
        }
        let Some(instances) = &self.instances else {
            return;
        };
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.camera_group, &[]);
        pass.set_vertex_buffer(1, instances.slice(..));
        for b in &self.batches {
            let mesh = &self.meshes[b.mesh as usize];
            pass.set_bind_group(1, &b.material, &[]);
            pass.set_vertex_buffer(0, mesh.vertices.slice(..));
            pass.set_vertex_buffer(2, mesh.lightmap_uvs.slice(..));
            pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count, 0, b.instances.clone());
        }
    }
}

/// A unit cube (from -0.5 to 0.5) seen from inside: one quad per [`CubeFace`] in [`CubeFace::ALL`] order, its UVs
/// following the face's orientation convention (texel (0, 0) top-left, u along `right`, v along `down`).
fn sky_cube(device: &wgpu::Device) -> GpuMesh {
    let mut vertices = Vec::with_capacity(24);
    let mut indices: Vec<u32> = Vec::with_capacity(36);
    for face in CubeFace::ALL {
        let b = face.basis();
        let base = u32::try_from(vertices.len()).unwrap_or(0);
        for (u, v) in [(0.0f32, 0.0f32), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)] {
            let p = b.forward * 0.5 + b.right * (u - 0.5) + b.down * (v - 0.5);
            vertices.push(MeshVertex {
                position: p.to_array(),
                normal: (-b.forward).to_array(),
                uv: [u, v],
            });
        }
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    GpuMesh {
        lightmap_uvs: lightmap_uv_buffer(device, None, vertices.len()),
        vertices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sky cube vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        }),
        indices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sky cube indices"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        }),
        index_count: 36,
    }
}

/// The lightmap UV stream of a mesh: `uvs`, or zeros for `vertex_count` vertices when it has none.
fn lightmap_uv_buffer(
    device: &wgpu::Device,
    uvs: Option<&[[f32; 2]]>,
    vertex_count: usize,
) -> wgpu::Buffer {
    let zeros;
    let data = match uvs {
        Some(uvs) => uvs,
        None => {
            zeros = vec![[0.0f32; 2]; vertex_count.max(1)];
            &zeros[..]
        }
    };
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("world mesh lightmap UVs"),
        contents: bytemuck::cast_slice(data),
        usage: wgpu::BufferUsages::VERTEX,
    })
}
