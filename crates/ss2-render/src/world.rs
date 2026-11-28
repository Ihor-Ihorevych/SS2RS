//! The 3D world pass (RFC 0013, slice of T04-005/T04-009): textured static meshes drawn with one matrix per
//! instance, reverse-Z depth, flat lighting from the camera and alpha cutoff. The 2D pass draws on top.
//!
//! Skinned meshes (RFC 0013 amendment 7, T04-006) go through a second pipeline of the same shader: each skinned
//! instance owns a uniform buffer of [`MAX_BONES`] skin matrices (palette times inverse bind, multiplied on the CPU
//! when the palette is set) and is drawn with the world's lighting and fog.
//!
//! The renderer keeps no CPU copy of world data. When the GPU device is rebuilt (or [`crate::Renderer::clear_world`]
//! is called) every mesh, texture and batch is dropped and [`crate::Renderer::world_generation`] changes; the
//! owner of the level uploads it again. Handles from an older generation are refused.

use std::ops::Range;

use ss2_core::math::{Mat4, Quat, Vec3};
use ss2_render_api::scene_data::{CubeFace, Fog, MeshData, MeshVertex, TexelFormat, TextureData};
use ss2_render_api::skinning::{MAX_BONES, SkinnedMeshData, SkinnedVertex};
use ss2_render_api::{Aabb, Camera, Frustum, Visibility};
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

/// A skinned mesh uploaded with [`crate::Renderer::create_skinned_mesh`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SkinnedMeshHandle {
    index: u32,
    generation: u64,
}

/// One skinned mesh drawn once, posed by its own bone palette (RFC 0013 amendment 7).
#[derive(Clone, Debug, PartialEq)]
pub struct SkinnedDraw {
    /// What to draw.
    pub mesh: SkinnedMeshHandle,
    /// The base-colour texture; `None` draws `tint` alone.
    pub texture: Option<TextureHandle>,
    /// Multiplies the texture (linear RGBA).
    pub tint: [f32; 4],
    /// Texels whose alpha (after the tint) is below this are not drawn; `None` draws all.
    pub alpha_cutoff: Option<f32>,
    /// Model-to-world matrix.
    pub transform: Mat4,
    /// One bone-to-model matrix per bone of the mesh (its `inverse_bind` list, in order); see
    /// [`ss2_render_api::skinning`].
    pub palette: Vec<Mat4>,
    /// A box, in the mesh's own (model) space, this draw never draws outside of whichever animation plays (RFC
    /// 0013 amendment 9, T04-048): `transform.transformed(bounds)` is what the renderer culls the draw by, not
    /// resampling the animation itself every frame. The world loader's `SkinnedModelData::culling_bounds` is
    /// built to hold this.
    pub bounds: Aabb,
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
    /// The level's cube half-extent, centre to face (only the ratio to `viewer_height` matters: the drawn cube has its own size).
    pub size: f32,
    /// The viewer's height inside the cube relative to its centre, in the units of `size` (RFC 0013 amendment 3).
    pub viewer_height: f32,
}

/// Half the size of the sky cube drawn around the camera (it never writes depth, so its size only has to clear the
/// near plane).
const SKY_HALF_SIZE: f32 = 10.0;

struct GpuSky {
    /// One material per face, in [`CubeFace::ALL`] order; `None` is a face that is not drawn.
    faces: Vec<Option<wgpu::BindGroup>>,
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

struct GpuSkinnedMesh {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    index_count: u32,
    inverse_bind: Vec<Mat4>,
}

struct GpuSkinnedDraw {
    mesh: u32,
    material: wgpu::BindGroup,
    /// The model-to-world matrix (one instance).
    instance: wgpu::Buffer,
    /// [`MAX_BONES`] skin matrices.
    palette: wgpu::Buffer,
    palette_group: wgpu::BindGroup,
    /// A CPU copy of `instance`'s matrix, kept in step by [`WorldGpu::set_skinned`] and
    /// [`WorldGpu::set_skinned_transform`]: `wgpu::Buffer` cannot be read back, and culling (T04-048) needs it
    /// every frame without a GPU round trip.
    transform: Mat4,
    /// The draw's [`SkinnedDraw::bounds`], in the mesh's own space; culled against the camera's frustum in
    /// `transform`'s world space (T04-048).
    bounds: Aabb,
}

/// The size of a skinned instance's palette buffer.
const PALETTE_BYTES: u64 = (MAX_BONES * 64) as u64;

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
    /// The world shader with the skinning vertex stage.
    skinned_pipeline: wgpu::RenderPipeline,
    palette_layout: wgpu::BindGroupLayout,
    skinned_meshes: Vec<GpuSkinnedMesh>,
    skinned: Vec<GpuSkinnedDraw>,
    /// The sky cube: six quads, one per face in [`CubeFace::ALL`] order (6 indices each).
    sky_mesh: GpuMesh,
    /// The sky cube's matrix, written every frame (it follows the camera).
    sky_instance: wgpu::Buffer,
    sky: Option<GpuSky>,
    /// The fog of the next frames ([`WorldGpu::set_fog`]).
    fog: Option<Fog>,
    /// Whether the sky takes the flat fog colour while there is fog ([`WorldGpu::set_sky_fog`]).
    sky_fog: bool,
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
            size: 128,
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
        let palette_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("skinned palette layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(PALETTE_BYTES),
                },
                count: None,
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("world layout"),
            bind_group_layouts: &[Some(&camera_layout), Some(&material_layout)],
            immediate_size: 0,
        });
        let skinned_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("skinned layout"),
            bind_group_layouts: &[
                Some(&camera_layout),
                Some(&material_layout),
                Some(&palette_layout),
            ],
            immediate_size: 0,
        });
        let instance_attributes = wgpu::vertex_attr_array![3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4];
        let instance_buffer = wgpu::VertexBufferLayout {
            array_stride: 64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &instance_attributes,
        };
        let static_attributes =
            wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2];
        let lightmap_attributes = wgpu::vertex_attr_array![7 => Float32x2];
        let static_buffers = [
            Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<MeshVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &static_attributes,
            }),
            Some(instance_buffer.clone()),
            Some(wgpu::VertexBufferLayout {
                array_stride: 8,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &lightmap_attributes,
            }),
        ];
        let skinned_attributes = wgpu::vertex_attr_array![
            0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 8 => Uint16x4, 9 => Float32x4
        ];
        let skinned_buffers = [
            Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<SkinnedVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &skinned_attributes,
            }),
            Some(instance_buffer),
        ];
        let make_pipeline =
            |label: &str,
             depth_write: bool,
             compare: wgpu::CompareFunction,
             layout: &wgpu::PipelineLayout,
             entry_point: &str,
             buffers: &[Option<wgpu::VertexBufferLayout<'_>>]| {
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some(label),
                    layout: Some(layout),
                    vertex: wgpu::VertexState {
                        module: &shader,
                        entry_point: Some(entry_point),
                        compilation_options: Default::default(),
                        buffers,
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
        let pipeline = make_pipeline(
            "world",
            true,
            wgpu::CompareFunction::Greater,
            &layout,
            "vs_main",
            &static_buffers,
        );
        // The sky is drawn first, behind everything, and leaves the depth buffer to the world.
        let sky_pipeline = make_pipeline(
            "sky",
            false,
            wgpu::CompareFunction::Always,
            &layout,
            "vs_main",
            &static_buffers,
        );
        let skinned_pipeline = make_pipeline(
            "skinned",
            true,
            wgpu::CompareFunction::Greater,
            &skinned_layout,
            "vs_skinned",
            &skinned_buffers,
        );
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
            skinned_pipeline,
            palette_layout,
            skinned_meshes: Vec::new(),
            skinned: Vec::new(),
            sky_mesh,
            sky_instance,
            sky: None,
            fog: None,
            sky_fog: false,
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
        self.skinned_meshes.clear();
        self.skinned.clear();
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

    pub(crate) fn create_skinned_mesh(
        &mut self,
        device: &wgpu::Device,
        mesh: &SkinnedMeshData,
    ) -> Result<SkinnedMeshHandle, RenderError> {
        mesh.validate()
            .map_err(|e| RenderError::World(e.to_string()))?;
        if mesh.indices.is_empty() {
            return Err(RenderError::World(
                "a skinned mesh needs at least one triangle".into(),
            ));
        }
        let index_count = u32::try_from(mesh.indices.len())
            .map_err(|_| RenderError::World("too many indices".into()))?;
        let index = u32::try_from(self.skinned_meshes.len())
            .map_err(|_| RenderError::World("too many skinned meshes".into()))?;
        self.skinned_meshes.push(GpuSkinnedMesh {
            vertices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("skinned mesh vertices"),
                contents: bytemuck::cast_slice(&mesh.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }),
            indices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("skinned mesh indices"),
                contents: bytemuck::cast_slice(&mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            }),
            index_count,
            inverse_bind: mesh.inverse_bind.clone(),
        });
        Ok(SkinnedMeshHandle {
            index,
            generation: self.generation,
        })
    }

    fn skinned_mesh_index(&self, h: SkinnedMeshHandle) -> Result<u32, RenderError> {
        if h.generation != self.generation || h.index as usize >= self.skinned_meshes.len() {
            return Err(RenderError::World(format!(
                "skinned mesh {h:?} is stale or unknown (world generation {})",
                self.generation
            )));
        }
        Ok(h.index)
    }

    /// The skin matrices (`palette[j]` times the mesh's inverse bind `j`) as bytes, or an error for a palette of the
    /// wrong length or with a non-finite matrix.
    fn skin_bytes(mesh: &GpuSkinnedMesh, palette: &[Mat4]) -> Result<Vec<f32>, RenderError> {
        if palette.len() != mesh.inverse_bind.len() {
            return Err(RenderError::World(format!(
                "a palette of {} matrices for a mesh of {} bones",
                palette.len(),
                mesh.inverse_bind.len()
            )));
        }
        if !palette.iter().all(Mat4::is_finite) {
            return Err(RenderError::World(
                "the palette has a non-finite matrix".into(),
            ));
        }
        Ok(palette
            .iter()
            .zip(&mesh.inverse_bind)
            .flat_map(|(p, i)| (*p * *i).to_cols_array())
            .collect())
    }

    /// Replace the skinned draws; draw `i` of `draws` becomes skinned instance `i`. Nothing changes if it returns an
    /// error.
    pub(crate) fn set_skinned(
        &mut self,
        device: &wgpu::Device,
        draws: Vec<SkinnedDraw>,
    ) -> Result<(), RenderError> {
        let mut planned = Vec::with_capacity(draws.len());
        for (i, d) in draws.iter().enumerate() {
            let mesh = self.skinned_mesh_index(d.mesh)?;
            let texture = d.texture.map(|t| self.texture(t)).transpose()?;
            let cutoff = d.alpha_cutoff.unwrap_or(0.0);
            if !d.tint.iter().all(|c| c.is_finite())
                || !cutoff.is_finite()
                || !d.transform.is_finite()
                || !(d.bounds.min.is_finite() && d.bounds.max.is_finite())
            {
                return Err(RenderError::World(format!(
                    "skinned draw {i} has a non-finite tint, cutoff, transform or bounds"
                )));
            }
            let skin = Self::skin_bytes(&self.skinned_meshes[mesh as usize], &d.palette)
                .map_err(|e| RenderError::World(format!("skinned draw {i}: {e}")))?;
            let mut palette_data = vec![0f32; MAX_BONES * 16];
            palette_data[..skin.len()].copy_from_slice(&skin);
            let palette = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("skinned palette"),
                contents: bytemuck::cast_slice(&palette_data),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
            let palette_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("skinned palette"),
                layout: &self.palette_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: palette.as_entire_binding(),
                }],
            });
            let instance = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("skinned instance"),
                contents: bytemuck::cast_slice(&d.transform.to_cols_array()),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
            let (view, sampler) = match texture {
                Some(t) => (
                    &t.view,
                    &self.samplers[usize::from(t.repeat_u) + 2 * usize::from(t.repeat_v)],
                ),
                None => (&self.white, &self.samplers[3]),
            };
            let material = self.material(device, view, sampler, d.tint, cutoff, false, None);
            planned.push(GpuSkinnedDraw {
                mesh,
                material,
                instance,
                palette,
                palette_group,
                transform: d.transform,
                bounds: d.bounds,
            });
        }
        self.skinned = planned;
        Ok(())
    }

    fn skinned_draw(&self, instance: usize) -> Result<&GpuSkinnedDraw, RenderError> {
        self.skinned.get(instance).ok_or_else(|| {
            RenderError::World(format!(
                "skinned instance {instance} is unknown ({} are drawn)",
                self.skinned.len()
            ))
        })
    }

    fn skinned_draw_mut(&mut self, instance: usize) -> Result<&mut GpuSkinnedDraw, RenderError> {
        let len = self.skinned.len();
        self.skinned.get_mut(instance).ok_or_else(|| {
            RenderError::World(format!(
                "skinned instance {instance} is unknown ({len} are drawn)"
            ))
        })
    }

    /// Pose skinned instance `instance` with `palette` from the next frame on. Nothing changes if it returns an error.
    pub(crate) fn set_bone_palette(
        &mut self,
        queue: &wgpu::Queue,
        instance: usize,
        palette: &[Mat4],
    ) -> Result<(), RenderError> {
        let draw = self.skinned_draw(instance)?;
        let skin = Self::skin_bytes(&self.skinned_meshes[draw.mesh as usize], palette)?;
        queue.write_buffer(&draw.palette, 0, bytemuck::cast_slice(&skin));
        Ok(())
    }

    /// Move skinned instance `instance` to `transform` from the next frame on.
    pub(crate) fn set_skinned_transform(
        &mut self,
        queue: &wgpu::Queue,
        instance: usize,
        transform: Mat4,
    ) -> Result<(), RenderError> {
        if !transform.is_finite() {
            return Err(RenderError::World(format!(
                "skinned instance {instance}: a non-finite transform"
            )));
        }
        let draw = self.skinned_draw_mut(instance)?;
        draw.transform = transform;
        queue.write_buffer(
            &draw.instance,
            0,
            bytemuck::cast_slice(&transform.to_cols_array()),
        );
        Ok(())
    }

    /// The number of skinned instances drawn.
    pub(crate) fn skinned_count(&self) -> usize {
        self.skinned.len()
    }

    /// Replace the sky (`None`: no sky, the clear colour shows). Nothing changes if it returns an error.
    /// Fog the world from the next frame on (`None`: no fog); refused unless [`Fog::validate`] passes.
    pub(crate) fn set_fog(&mut self, fog: Option<Fog>) -> Result<(), RenderError> {
        if let Some(f) = &fog {
            f.validate()
                .map_err(|e| RenderError::World(e.to_string()))?;
        }
        self.fog = fog;
        Ok(())
    }

    /// Draw the sky as the flat fog colour while there is fog (the original's behaviour, T01-429), or keep it clear.
    pub(crate) fn set_sky_fog(&mut self, on: bool) {
        self.sky_fog = on;
    }

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
        // A face without a texture is drawn in the tint alone, except the floor, which is left open while the
        // ceiling has a texture (`docs/re/environment.md`, "Faces with no texture", T01-413).
        let open_floor = sky.faces[CubeFace::NegY.index()].is_none()
            && sky.faces[CubeFace::PosY.index()].is_some();
        let mut faces = Vec::with_capacity(6);
        for (face, texture) in CubeFace::ALL.into_iter().zip(sky.faces) {
            if face == CubeFace::NegY && open_floor {
                faces.push(None);
                continue;
            }
            // Clamped, so a face's edge does not pick up the opposite edge.
            let view = match texture {
                Some(t) => &self.texture(t)?.view,
                None => &self.white,
            };
            faces.push(Some(self.material(
                device,
                view,
                &self.samplers[0],
                sky.tint,
                0.0,
                true,
                None,
            )));
        }
        self.sky = Some(GpuSky {
            faces,
            rotation: sky.rotation.normalize(),
            viewer_offset: sky.viewer_height / sky.size * SKY_HALF_SIZE,
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
        // Culls the skinned draws below (T04-048); not the static batches, which have no per-instance bounds yet.
        let frustum = Frustum::from_camera(camera, aspect);
        let mut uniform = [0f32; 32];
        uniform[..16].copy_from_slice(&camera.view_projection(aspect).to_cols_array());
        uniform[16..19].copy_from_slice(&camera.forward().to_array());
        uniform[20..23].copy_from_slice(&camera.position.to_array());
        if let Some(fog) = self.fog {
            // On a target that is not sRGB the pass works in sRGB values (textures are sampled raw), so the fog colour
            // is encoded the same way to show the same colour on both.
            let colour = if self.srgb {
                fog.colour
            } else {
                fog.colour.map(linear_to_srgb)
            };
            uniform[24..27].copy_from_slice(&colour);
            uniform[27] = fog.strength;
            uniform[28] = fog.near;
            uniform[29] = fog.far;
            uniform[30] = if self.sky_fog { 1.0 } else { 0.0 };
        }
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
                let Some(material) = material else {
                    continue;
                };
                pass.set_bind_group(1, material, &[]);
                pass.draw_indexed(i * 6..i * 6 + 6, 0, 0..1);
            }
        }
        if let Some(instances) = &self.instances {
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
        if self.skinned.is_empty() {
            return;
        }
        pass.set_pipeline(&self.skinned_pipeline);
        pass.set_bind_group(0, &self.camera_group, &[]);
        for d in &self.skinned {
            if !skinned_draw_visible(&frustum, d.bounds, d.transform) {
                continue;
            }
            let mesh = &self.skinned_meshes[d.mesh as usize];
            pass.set_bind_group(1, &d.material, &[]);
            pass.set_bind_group(2, &d.palette_group, &[]);
            pass.set_vertex_buffer(0, mesh.vertices.slice(..));
            pass.set_vertex_buffer(1, d.instance.slice(..));
            pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count, 0, 0..1);
        }
    }
}

/// Whether a skinned draw whose own-space box is `bounds` and whose model-to-world matrix is `transform` should be
/// drawn against `frustum` (T04-048): `false` only when [`Frustum::test_aabb`] of the box in world space says
/// [`Visibility::Outside`], so a box the frustum test cannot rule out is still drawn (the frustum's own contract:
/// never `Outside` for something a pixel could show).
fn skinned_draw_visible(frustum: &Frustum, bounds: Aabb, transform: Mat4) -> bool {
    frustum.test_aabb(bounds.transformed(transform)) != Visibility::Outside
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

/// One linear channel in 0..=1 as sRGB.
fn linear_to_srgb(c: f32) -> f32 {
    let c = c.clamp(0.0, 1.0);
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

#[cfg(test)]
mod skinned_culling_tests {
    use ss2_render_api::{Camera, Projection};

    use super::*;

    /// A camera at the origin looking down -Z, matching [`ss2_render_api::culling`]'s own test camera.
    fn frustum() -> Frustum {
        let camera = Camera {
            projection: Projection::Perspective {
                fov_y: std::f32::consts::FRAC_PI_2,
                near: 1.0,
            },
            ..Camera::default()
        };
        Frustum::from_camera(&camera, 1.0)
    }

    fn unit_box() -> Aabb {
        Aabb::new(Vec3::splat(-0.5), Vec3::splat(0.5))
    }

    #[test]
    fn a_draw_placed_ahead_is_visible_and_one_placed_far_to_the_side_is_not() {
        let f = frustum();
        let ahead = Mat4::from_translation(Vec3::new(0.0, 0.0, -5.0));
        assert!(skinned_draw_visible(&f, unit_box(), ahead));
        let far_aside = Mat4::from_translation(Vec3::new(40.0, 0.0, -5.0));
        assert!(!skinned_draw_visible(&f, unit_box(), far_aside));
    }

    #[test]
    fn a_box_straddling_the_frustums_edge_is_not_culled() {
        let f = frustum();
        // At z = -5 the view half-extent is 5 (a 90 degree vertical fov, aspect 1): a box centred just past it
        // still reaches inside, so it must be drawn (the frustum's contract: never `Outside` for the visible).
        let straddling = Mat4::from_translation(Vec3::new(5.2, 0.0, -5.0));
        assert!(skinned_draw_visible(&f, unit_box(), straddling));
    }

    #[test]
    fn scale_and_rotation_grow_the_tested_box_conservatively() {
        let f = frustum();
        // A small box far to the side, scaled up enough that its grown (axis-aligned, rotation-conservative) bound
        // reaches back into view.
        let big = Mat4::from_scale_rotation_translation(
            Vec3::splat(20.0),
            Quat::from_rotation_y(0.3),
            Vec3::new(30.0, 0.0, -5.0),
        );
        assert!(skinned_draw_visible(&f, unit_box(), big));
    }

    #[test]
    fn an_invalid_camera_sees_nothing_not_everything() {
        let f = Frustum::from_view_projection(Mat4::from_cols_array(&[f32::NAN; 16]));
        assert!(!skinned_draw_visible(
            &f,
            unit_box(),
            Mat4::from_translation(Vec3::new(0.0, 0.0, -5.0))
        ));
    }
}
