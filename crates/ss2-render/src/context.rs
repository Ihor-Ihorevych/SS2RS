//! The wgpu renderer: windowed or offscreen; quads, text, images, 9-slice, scissor and render targets.

use std::sync::Arc;
use std::time::Instant;

use ss2_render_api::Camera;
use ss2_render_api::scene_data::{MeshData, TexelFormat, TextureData};
use ss2_ui::{Canvas, Color, DrawList, Vec2};
use wgpu::util::DeviceExt;

use crate::batch::Vertex;
use crate::gpu::{self, Gpu, GpuConfig, LostSignal, RecoveryTracker, Verdict};
use crate::images::{GpuImage, ImageStore, Recipe, Source};
use crate::plan::{BatchKind, Plan};
use crate::text::TextPainter;
use crate::ui_frame::{ImageHandle, UiFrame, UiScene, replay_order, would_loop};
use crate::world::{MeshHandle, SkyBatch, TextureHandle, WorldBatch, WorldGpu};

/// Why the renderer could not be created or could not draw.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    /// No usable GPU adapter (or software fallback) exists.
    #[error("no suitable GPU adapter: {0}")]
    NoAdapter(String),
    /// Device creation, drawing or readback failed.
    #[error("device error: {0}")]
    Device(String),
    /// The window surface could not be created or used.
    #[error("surface error: {0}")]
    Surface(String),
    /// The GPU device was lost and could not be rebuilt (or has been lost too often in a row).
    #[error("GPU device lost: {0}")]
    DeviceLost(String),
    /// An image could not be made or used: a bad size, an unknown or destroyed handle, or a render target that
    /// would be drawn into itself.
    #[error("image error: {0}")]
    Image(String),
    /// World data could not be uploaded or used: bad mesh or texture data, an unsupported texture format, or a
    /// handle from before the world was dropped.
    #[error("world error: {0}")]
    World(String),
}

/// What recovery has done so far (for a log line, an overlay or a test).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RenderStats {
    /// How many times the device (and adapter) was made again after it was lost.
    pub device_rebuilds: u32,
    /// How many times the window surface was configured again.
    pub surface_reconfigures: u32,
    /// Frames not drawn because the surface was outdated or lost.
    pub frames_skipped: u32,
}

/// An RGBA8 image (row-major, tightly packed, 4 bytes per pixel).
#[derive(Debug, Clone, PartialEq)]
pub struct ImageRgba8 {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `width * height * 4` bytes.
    pub pixels: Vec<u8>,
}

enum Target {
    Window {
        surface: wgpu::Surface<'static>,
        config: wgpu::SurfaceConfiguration,
    },
    Offscreen {
        texture: wgpu::Texture,
    },
    /// Only while the device is being rebuilt.
    Detached,
}

/// Draws a [`DrawList`] to a window or to an offscreen image.
pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    target: Target,
    size: (u32, u32),
    format: wgpu::TextureFormat,
    /// Straight-alpha blending (uploaded images, flat quads).
    pipeline: wgpu::RenderPipeline,
    /// Premultiplied blending (render targets).
    pipeline_premultiplied: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    texture_layout: wgpu::BindGroupLayout,
    /// The sampler bind groups, indexed by `Filter` (nearest, linear).
    samplers: [wgpu::BindGroup; 2],
    /// A white texel, bound for flat quads.
    white: wgpu::BindGroup,
    images: ImageStore,
    uniform: wgpu::Buffer,
    vertices: wgpu::Buffer,
    vertex_capacity: u64,
    text: TextPainter,
    world: WorldGpu,
    instance: wgpu::Instance,
    config: GpuConfig,
    lost: LostSignal,
    report: gpu::DeviceReport,
    tracker: RecoveryTracker,
    stats: RenderStats,
    started: Instant,
}

fn to_wgpu_color(c: Color) -> wgpu::Color {
    wgpu::Color {
        r: c.0[0] as f64,
        g: c.0[1] as f64,
        b: c.0[2] as f64,
        a: c.0[3] as f64,
    }
}

impl Renderer {
    /// Create a renderer drawing into a window. `display` is the window system's display handle
    /// (for winit: `event_loop.owned_display_handle()`), `target` the window.
    pub fn new_windowed(
        display: Box<dyn wgpu::wgt::WgpuHasDisplayHandle>,
        target: impl Into<wgpu::SurfaceTarget<'static>>,
        size: (u32, u32),
        vsync: bool,
    ) -> Result<Renderer, RenderError> {
        let gpu_config = GpuConfig::from_env();
        let instance = gpu::make_instance(&gpu_config, Some(display));
        let surface = instance
            .create_surface(target)
            .map_err(|e| RenderError::Surface(e.to_string()))?;
        let gpu = pollster::block_on(gpu::request_gpu(
            &instance,
            Some(&surface),
            None,
            &gpu_config,
        ))?;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: gpu.format,
            width: size.0.max(1),
            height: size.1.max(1),
            present_mode: present_mode(vsync),
            alpha_mode: gpu.alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        if size.0 > 0 && size.1 > 0 {
            surface.configure(&gpu.device, &config);
        }
        Ok(Self::build(gpu, Target::Window { surface, config }, size))
    }

    /// Create a renderer drawing into an offscreen RGBA8 texture. Tries a hardware adapter and
    /// then a software fallback; returns [`RenderError::NoAdapter`] if neither exists.
    pub fn new_offscreen(size: (u32, u32)) -> Result<Renderer, RenderError> {
        let gpu_config = GpuConfig::from_env();
        let instance = gpu::make_instance(&gpu_config, None);
        let gpu = pollster::block_on(gpu::request_gpu(&instance, None, None, &gpu_config))?;
        let texture = capture_texture(&gpu.device, gpu.format, (size.0.max(1), size.1.max(1)))?;
        Ok(Self::build(gpu, Target::Offscreen { texture }, size))
    }

    fn build(gpu: Gpu, target: Target, size: (u32, u32)) -> Renderer {
        let Gpu {
            instance,
            device,
            queue,
            format,
            lost,
            report,
            config,
            alpha_mode: _,
        } = gpu;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ui"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewport"),
            contents: bytemuck::cast_slice(&[size.0.max(1) as f32, size.1.max(1) as f32, 0.0, 0.0]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("viewport layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("viewport"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("image layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let sampler_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sampler layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            }],
        });
        let sampler_group = |label: &str, filter: wgpu::FilterMode| {
            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some(label),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: filter,
                min_filter: filter,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                ..Default::default()
            });
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &sampler_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                }],
            })
        };
        let samplers = [
            sampler_group("nearest", wgpu::FilterMode::Nearest),
            sampler_group("linear", wgpu::FilterMode::Linear),
        ];
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ui layout"),
            bind_group_layouts: &[Some(&bgl), Some(&texture_layout), Some(&sampler_layout)],
            immediate_size: 0,
        });
        let make_pipeline = |label: &str, fragment: &str, blend: wgpu::BlendState| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4],
                    })],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fragment),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(blend),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let pipeline = make_pipeline("ui", "fs_main", wgpu::BlendState::ALPHA_BLENDING);
        let pipeline_premultiplied = make_pipeline(
            "ui premultiplied",
            "fs_premultiplied",
            wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING,
        );
        let white_texture = device.create_texture_with_data(
            &queue,
            &wgpu::TextureDescriptor {
                label: Some("white texel"),
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
        );
        let white = texture_group(&device, &texture_layout, &white_texture, "white texel");
        let vertex_capacity = 6 * 256;
        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("quad vertices"),
            size: vertex_capacity * std::mem::size_of::<Vertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let text = TextPainter::new(&device, &queue, format);
        let world = WorldGpu::new(&device, &queue, format, 0);
        Renderer {
            device,
            queue,
            target,
            size,
            format,
            pipeline,
            pipeline_premultiplied,
            bind_group,
            texture_layout,
            samplers,
            white,
            images: ImageStore::default(),
            uniform,
            vertices,
            vertex_capacity,
            text,
            world,
            instance,
            config,
            lost,
            report,
            tracker: RecoveryTracker::default(),
            stats: RenderStats::default(),
            started: Instant::now(),
        }
    }

    /// A renderer on an offscreen `texture` of any format (tests of the other formats).
    #[cfg(test)]
    pub(crate) fn from_gpu_for_tests(
        gpu: Gpu,
        texture: wgpu::Texture,
        size: (u32, u32),
    ) -> Renderer {
        Self::build(gpu, Target::Offscreen { texture }, size)
    }

    /// Destroy the GPU device, as a driver reset would (tests of recovery).
    #[cfg(test)]
    pub(crate) fn destroy_device_for_tests(&self) {
        self.device.destroy();
    }

    /// What the adapter and device turned out to be (also logged at start-up).
    pub fn report(&self) -> &gpu::DeviceReport {
        &self.report
    }

    /// What recovery has done so far.
    pub fn stats(&self) -> RenderStats {
        self.stats
    }

    fn now_ms(&self) -> u64 {
        u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    /// Notice a lost device (the callback only sets a flag) and rebuild it. `Ok(true)` if it was
    /// rebuilt now, `Err` if it was lost too often to try again.
    fn check_device(&mut self) -> Result<bool, RenderError> {
        // The loss is reported while the device is polled; this is a quick no-wait poll.
        let _ = self.device.poll(wgpu::PollType::Poll);
        let Some(reason) = self.lost.take() else {
            return Ok(false);
        };
        match self.tracker.device_lost(self.now_ms()) {
            Verdict::GiveUp => Err(RenderError::DeviceLost(reason)),
            _ => self.rebuild_device(&reason).map(|()| true),
        }
    }

    /// Ask for a new adapter and device, put the surface (or the offscreen texture) on it and
    /// rebuild every GPU object. Everything the renderer holds is made again from the draw lists, so
    /// nothing is lost but the frame in flight.
    fn rebuild_device(&mut self, reason: &str) -> Result<(), RenderError> {
        tracing::warn!("rebuilding the GPU device: {reason}");
        let surface = match &self.target {
            Target::Window { surface, .. } => Some(surface),
            _ => None,
        };
        let gpu = pollster::block_on(gpu::request_gpu(
            &self.instance,
            surface,
            None,
            &self.config,
        ))?;
        let offscreen = if matches!(self.target, Target::Offscreen { .. }) {
            let size = (self.size.0.max(1), self.size.1.max(1));
            Some(capture_texture(&gpu.device, gpu.format, size)?)
        } else {
            None
        };
        let target = match std::mem::replace(&mut self.target, Target::Detached) {
            Target::Window {
                surface,
                mut config,
            } => {
                config.format = gpu.format;
                if self.size.0 > 0 && self.size.1 > 0 {
                    surface.configure(&gpu.device, &config);
                }
                Target::Window { surface, config }
            }
            other => match offscreen {
                Some(texture) => Target::Offscreen { texture },
                None => other,
            },
        };
        let mut fresh = Self::build(gpu, target, self.size);
        // Every image is uploaded again, and every render target drawn again from its recipe.
        fresh.images = self.images.clone_cpu();
        fresh.restore_images()?;
        fresh.tracker = self.tracker.clone();
        fresh.stats = self.stats;
        fresh.stats.device_rebuilds += 1;
        fresh.started = self.started;
        // The world is not kept on the CPU: it is dropped, and its owner uploads it again.
        fresh.world = WorldGpu::new(
            &fresh.device,
            &fresh.queue,
            fresh.format,
            self.world.generation() + 1,
        );
        *self = fresh;
        Ok(())
    }

    fn reconfigure_surface(&mut self) {
        if let Target::Window { surface, config } = &self.target {
            surface.configure(&self.device, config);
            self.stats.surface_reconfigures += 1;
        }
    }

    /// The surface was outdated or lost on this frame: skip it, and reconfigure or rebuild.
    fn surface_unusable(&mut self) -> Result<(), RenderError> {
        self.stats.frames_skipped += 1;
        match self.tracker.surface_unusable(self.now_ms()) {
            Verdict::Reconfigure => {
                self.reconfigure_surface();
                Ok(())
            }
            Verdict::RebuildDevice => self.rebuild_device("the surface stayed unusable"),
            Verdict::GiveUp => Err(RenderError::Surface(
                "the surface stays unusable and the device was rebuilt too often to try again"
                    .into(),
            )),
        }
    }

    /// Resize the drawing surface. A 0x0 size (minimized window) is remembered but not applied.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.size = (width, height);
        if width == 0 || height == 0 {
            return;
        }
        if let Target::Window { surface, config } = &mut self.target {
            config.width = width;
            config.height = height;
            surface.configure(&self.device, config);
        }
    }

    /// Switch vsync on or off (windowed only).
    pub fn set_vsync(&mut self, vsync: bool) {
        if let Target::Window { surface, config } = &mut self.target {
            config.present_mode = present_mode(vsync);
            if self.size.0 > 0 && self.size.1 > 0 {
                surface.configure(&self.device, config);
            }
        }
    }

    /// Draw `list` into the window. Does nothing (returns `Ok`) while the window is 0x0 or
    /// occluded; reconfigures and skips the frame when the surface is outdated or lost.
    pub fn render(&mut self, list: &DrawList, clear: Color) -> Result<(), RenderError> {
        self.render_scene(list, clear, None)
    }

    /// Like [`Renderer::render`], with the world (see [`Renderer::set_world`]) drawn first as `camera` sees it
    /// and `overlay` on top.
    pub fn render_world(
        &mut self,
        camera: &Camera,
        overlay: &DrawList,
        clear: Color,
    ) -> Result<(), RenderError> {
        self.render_scene(overlay, clear, Some(camera))
    }

    /// Like [`Renderer::render`] for a [`UiFrame`] (images, 9-slice, clip rectangles and everything else a
    /// frame can hold).
    pub fn render_frame(&mut self, frame: &UiFrame, clear: Color) -> Result<(), RenderError> {
        self.render_scene(frame, clear, None)
    }

    fn render_scene(
        &mut self,
        scene: &dyn UiScene,
        clear: Color,
        world: Option<&Camera>,
    ) -> Result<(), RenderError> {
        self.check_device()?;
        if self.size.0 == 0 || self.size.1 == 0 {
            return Ok(());
        }
        let Target::Window { surface, .. } = &self.target else {
            return Err(RenderError::Surface(
                "render() needs a windowed renderer".into(),
            ));
        };
        let frame = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Suboptimal(_) => {
                self.reconfigure_surface();
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                return self.surface_unusable();
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(RenderError::Surface("surface validation error".into()));
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame"),
            });
        let size = (self.size.0.max(1), self.size.1.max(1));
        let canvas = Canvas::fit(size.0, size.1).expect("size is non-zero");
        let ui_clear = self.encode_world(&mut encoder, &view, size, world, clear);
        self.draw(&view, &mut encoder, scene, ui_clear, size, &canvas, true)?;
        self.queue.submit(Some(encoder.finish()));
        self.queue.present(frame);
        self.text.trim();
        self.tracker.frame_ok();
        Ok(())
    }

    /// Draw `list` into the offscreen texture and read it back as RGBA8.
    pub fn render_to_image(
        &mut self,
        list: &DrawList,
        clear: Color,
    ) -> Result<ImageRgba8, RenderError> {
        self.render_scene_to_image(list, clear)
    }

    /// Like [`Renderer::render_to_image`] for a [`UiFrame`].
    pub fn render_frame_to_image(
        &mut self,
        frame: &UiFrame,
        clear: Color,
    ) -> Result<ImageRgba8, RenderError> {
        self.render_scene_to_image(frame, clear)
    }

    fn render_scene_to_image(
        &mut self,
        scene: &dyn UiScene,
        clear: Color,
    ) -> Result<ImageRgba8, RenderError> {
        if !matches!(self.target, Target::Offscreen { .. }) {
            return Err(RenderError::Device(
                "render_to_image() needs an offscreen renderer".into(),
            ));
        }
        self.capture_image(
            Some(scene),
            clear,
            (self.size.0.max(1), self.size.1.max(1)),
            None,
        )
    }

    /// Draw the world as `camera` sees it, with `overlay` on top, and read it back as RGBA8 (offscreen or
    /// windowed, at the current size). Needs an 8-bit colour target.
    pub fn render_world_to_image(
        &mut self,
        camera: &Camera,
        overlay: Option<&DrawList>,
        clear: Color,
    ) -> Result<ImageRgba8, RenderError> {
        if self.size.0 == 0 || self.size.1 == 0 {
            return Err(RenderError::Device(
                "cannot capture a zero-size framebuffer".into(),
            ));
        }
        self.capture_image(
            overlay.map(|l| l as &dyn UiScene),
            clear,
            self.size,
            Some(camera),
        )
    }

    /// Render a capture with the current framebuffer size and return tightly packed RGBA8 pixels.
    /// Pass `None` to suppress quads and text, for example when taking a photo without the HUD.
    /// A windowed renderer draws into a separate texture. Non-8-bit targets use an RGBA8 capture
    /// pass on the same device. This waits for GPU readback and requires a nonzero size.
    pub fn capture_frame(
        &mut self,
        overlay: Option<&DrawList>,
        clear: Color,
    ) -> Result<ImageRgba8, RenderError> {
        self.capture_scene(overlay.map(|l| l as &dyn UiScene), clear)
    }

    /// Like [`Renderer::capture_frame`] with a [`UiFrame`] as the overlay.
    pub fn capture_ui_frame(
        &mut self,
        overlay: Option<&UiFrame>,
        clear: Color,
    ) -> Result<ImageRgba8, RenderError> {
        self.capture_scene(overlay.map(|f| f as &dyn UiScene), clear)
    }

    fn capture_scene(
        &mut self,
        overlay: Option<&dyn UiScene>,
        clear: Color,
    ) -> Result<ImageRgba8, RenderError> {
        if self.size.0 == 0 || self.size.1 == 0 {
            return Err(RenderError::Device(
                "cannot capture a zero-size framebuffer".into(),
            ));
        }
        self.capture_image(overlay, clear, self.size, None)
    }

    fn capture_image(
        &mut self,
        overlay: Option<&dyn UiScene>,
        clear: Color,
        size: (u32, u32),
        world: Option<&Camera>,
    ) -> Result<ImageRgba8, RenderError> {
        self.check_device()?;
        let format = match &self.target {
            Target::Window { config, .. } => config.format,
            Target::Offscreen { texture } => texture.format(),
            Target::Detached => {
                return Err(RenderError::Device("the renderer has no target".into()));
            }
        };
        match format {
            wgpu::TextureFormat::Rgba8Unorm
            | wgpu::TextureFormat::Rgba8UnormSrgb
            | wgpu::TextureFormat::Bgra8Unorm
            | wgpu::TextureFormat::Bgra8UnormSrgb => {
                self.read_frame(overlay, clear, size, format, world)
            }
            _ if world.is_some() => Err(RenderError::World(format!(
                "capturing the world needs an 8-bit target, not {format:?}"
            ))),
            _ => {
                let format = wgpu::TextureFormat::Rgba8Unorm;
                let texture = capture_texture(&self.device, format, size)?;
                let gpu = Gpu {
                    instance: self.instance.clone(),
                    device: self.device.clone(),
                    queue: self.queue.clone(),
                    format,
                    lost: self.lost.clone(),
                    report: self.report.clone(),
                    config: self.config.clone(),
                    alpha_mode: wgpu::CompositeAlphaMode::Auto,
                };
                let mut temporary = Self::build(gpu, Target::Offscreen { texture }, size);
                // The images are made again in the temporary renderer's own format.
                temporary.images = self.images.clone_cpu();
                temporary.restore_images()?;
                temporary.read_frame(overlay, clear, size, format, None)
            }
        }
    }

    fn read_frame(
        &mut self,
        overlay: Option<&dyn UiScene>,
        clear: Color,
        (w, h): (u32, u32),
        format: wgpu::TextureFormat,
        world: Option<&Camera>,
    ) -> Result<ImageRgba8, RenderError> {
        let swap_red_blue = matches!(
            format,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        );
        let texture = match &self.target {
            Target::Offscreen { texture } if (texture.width(), texture.height()) == (w, h) => {
                texture.clone()
            }
            _ => capture_texture(&self.device, format, (w, h))?,
        };
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("capture"),
            });
        let size = (self.size.0.max(1), self.size.1.max(1));
        let canvas = Canvas::fit(size.0, size.1).expect("size is non-zero");
        let ui_clear = self.encode_world(&mut encoder, &view, (w, h), world, clear);
        self.draw(
            &view,
            &mut encoder,
            overlay.unwrap_or(&DrawList::default()),
            ui_clear,
            size,
            &canvas,
            true,
        )?;
        let pixels = self.read_texture(encoder, &texture, (w, h), swap_red_blue)?;
        self.text.trim();
        Ok(ImageRgba8 {
            width: w,
            height: h,
            pixels,
        })
    }

    /// Finish `encoder` (which has drawn into `texture`), copy `texture` out and return its pixels as tightly
    /// packed RGBA8 (`swap_red_blue` for a BGRA texture).
    fn read_texture(
        &self,
        mut encoder: wgpu::CommandEncoder,
        texture: &wgpu::Texture,
        (w, h): (u32, u32),
        swap_red_blue: bool,
    ) -> Result<Vec<u8>, RenderError> {
        let row_bytes = w * 4;
        let bytes_per_row = row_bytes.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback_size = u64::from(bytes_per_row) * u64::from(h);
        if readback_size > self.device.limits().max_buffer_size {
            return Err(RenderError::Device(format!(
                "capture readback buffer is too large for {w}x{h}"
            )));
        }
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: readback_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));

        let (tx, rx) = std::sync::mpsc::channel();
        readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| RenderError::Device(format!("poll: {e}")))?;
        rx.recv()
            .map_err(|e| RenderError::Device(format!("readback channel: {e}")))?
            .map_err(|e| RenderError::Device(format!("readback map: {e}")))?;
        let data = readback
            .slice(..)
            .get_mapped_range()
            .map_err(|e| RenderError::Device(format!("readback range: {e}")))?;
        let pixels = pack_capture_rows(
            &data,
            row_bytes as usize,
            bytes_per_row as usize,
            swap_red_blue,
        );
        drop(data);
        readback.unmap();
        Ok(pixels)
    }

    /// With a camera, clear `view` and draw the world into it, then return `None` so the 2D pass draws on top;
    /// without one, return the clear colour for the 2D pass.
    fn encode_world(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        size: (u32, u32),
        camera: Option<&Camera>,
        clear: Color,
    ) -> Option<wgpu::Color> {
        let clear = to_wgpu_color(clear);
        let Some(camera) = camera else {
            return Some(clear);
        };
        self.world.encode(
            &self.device,
            &self.queue,
            encoder,
            view,
            size,
            camera,
            clear,
        );
        None
    }

    /// Draw `scene` into `view` (a texture of `size` pixels): plan the batches, upload the vertices and prepare
    /// the text, then one render pass that draws the batches in order, each with its own scissor rectangle. The
    /// pass clears to `clear`, or keeps what `view` holds when it is `None` (the world is under the 2D pass).
    /// With `strict` an image the scene names that is not registered is an error; without it the command is
    /// skipped (used to draw a render target again, whose images may have been destroyed since).
    #[allow(clippy::too_many_arguments)]
    fn draw(
        &mut self,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        scene: &dyn UiScene,
        clear: Option<wgpu::Color>,
        size: (u32, u32),
        canvas: &Canvas,
        strict: bool,
    ) -> Result<(), RenderError> {
        let images = &self.images;
        let plan = Plan::build(scene, canvas, size, &|h| images.size_of(h));
        if strict && let Some(missing) = plan.missing.first() {
            return Err(RenderError::Image(format!(
                "image {} is not registered (destroyed, or made by another renderer)",
                missing.id()
            )));
        }
        if plan.unbalanced_pops > 0 {
            tracing::warn!(
                "{} PopClip command(s) without a PushClip were ignored",
                plan.unbalanced_pops
            );
        }
        let needed = plan.vertices.len() as u64;
        if needed > self.vertex_capacity {
            self.vertex_capacity = needed.next_power_of_two();
            self.vertices = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("quad vertices"),
                size: self.vertex_capacity * std::mem::size_of::<Vertex>() as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !plan.vertices.is_empty() {
            self.queue
                .write_buffer(&self.vertices, 0, bytemuck::cast_slice(&plan.vertices));
        }
        self.queue.write_buffer(
            &self.uniform,
            0,
            bytemuck::cast_slice(&[size.0 as f32, size.1 as f32, 0.0, 0.0]),
        );
        self.text.begin(&self.queue, size);
        let mut slot = 0;
        for batch in &plan.batches {
            if let BatchKind::Text { runs } = &batch.kind {
                self.text.prepare(
                    slot,
                    &self.device,
                    &self.queue,
                    size,
                    canvas,
                    &plan.texts[runs.clone()],
                    batch.scissor,
                )?;
                slot += 1;
            }
        }

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ui"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: clear.map_or(wgpu::LoadOp::Load, wgpu::LoadOp::Clear),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        let mut slot = 0;
        for batch in &plan.batches {
            match batch.scissor {
                Some(r) => pass.set_scissor_rect(r.x, r.y, r.w, r.h),
                None => pass.set_scissor_rect(0, 0, size.0, size.1),
            }
            match &batch.kind {
                BatchKind::Geometry {
                    image,
                    filter,
                    vertices,
                } => {
                    let (pipeline, texture) = match image.and_then(|h| self.images.get(h)) {
                        Some(entry) => {
                            let Some(gpu) = &entry.gpu else {
                                continue;
                            };
                            let pipeline = if entry.is_target() {
                                &self.pipeline_premultiplied
                            } else {
                                &self.pipeline
                            };
                            (pipeline, &gpu.bind_group)
                        }
                        None => (&self.pipeline, &self.white),
                    };
                    pass.set_pipeline(pipeline);
                    pass.set_bind_group(0, &self.bind_group, &[]);
                    pass.set_bind_group(1, texture, &[]);
                    pass.set_bind_group(2, &self.samplers[*filter as usize], &[]);
                    pass.set_vertex_buffer(0, self.vertices.slice(..));
                    pass.draw(vertices.clone(), 0..1);
                }
                BatchKind::Text { .. } => {
                    self.text.render(slot, &mut pass)?;
                    slot += 1;
                }
            }
        }
        Ok(())
    }

    /// Whether textures in `format` can be uploaded ([`TexelFormat::Rgba8`] always; the BC formats when the
    /// GPU supports them).
    pub fn supports(&self, format: TexelFormat) -> bool {
        self.world.supports(format)
    }

    /// Upload a mesh for the world. The handle lasts until the world is dropped (see
    /// [`Renderer::world_generation`]).
    pub fn create_mesh(&mut self, mesh: &MeshData) -> Result<MeshHandle, RenderError> {
        self.world.create_mesh(&self.device, mesh)
    }

    /// Upload a texture for the world. Bytes are used as they are: on an sRGB target the texture is sampled as
    /// sRGB, so a texel drawn at full light shows its own bytes either way.
    pub fn create_texture(&mut self, texture: &TextureData) -> Result<TextureHandle, RenderError> {
        self.world
            .create_texture(&self.device, &self.queue, texture)
    }

    /// Replace what the world draws. Nothing changes if it returns an error (a stale or unknown handle, a
    /// non-finite matrix, tint or cutoff).
    pub fn set_world(&mut self, batches: Vec<WorldBatch>) -> Result<(), RenderError> {
        self.world.set_world(&self.device, batches)
    }

    /// Replace the sky drawn behind the world (`None`: no sky, the clear colour shows); it is dropped with the world.
    /// Nothing changes if it returns an error (a stale or unknown texture, a non-finite tint).
    pub fn set_sky(&mut self, sky: Option<SkyBatch>) -> Result<(), RenderError> {
        self.world.set_sky(&self.device, sky)
    }

    /// Drop every mesh, texture and batch of the world, and the sky.
    pub fn clear_world(&mut self) {
        self.world.clear();
    }

    /// Changes whenever the world is dropped: by [`Renderer::clear_world`], or by a GPU device rebuild after
    /// which the owner of the level must upload it again. Handles from another generation are refused.
    pub fn world_generation(&self) -> u64 {
        self.world.generation()
    }

    /// Make an image the renderer can draw: `image` is uploaded and a copy of its pixels is kept, so that it
    /// can be uploaded again after the GPU device is rebuilt. Nothing changes if it returns an error.
    ///
    /// Uploaded images have straight (not premultiplied) alpha. The texture is `Rgba8Unorm`, or `Rgba8UnormSrgb`
    /// when the target is an sRGB format, so that an image drawn 1:1 shows its own bytes in either case.
    pub fn create_image(&mut self, image: &ImageRgba8) -> Result<ImageHandle, RenderError> {
        let (w, h) = (image.width, image.height);
        self.check_image_size((w, h))?;
        let expected = w as usize * h as usize * 4;
        if image.pixels.len() != expected {
            return Err(RenderError::Image(format!(
                "a {w}x{h} image needs {expected} bytes of RGBA8, got {}",
                image.pixels.len()
            )));
        }
        let handle = self
            .images
            .insert((w, h), Source::Pixels(Arc::new(image.clone())))
            .ok_or_else(|| RenderError::Image("out of image handles".into()))?;
        self.make_gpu_image(handle);
        Ok(handle)
    }

    /// Make an empty (transparent) image of `width` x `height` pixels that [`Renderer::render_to_target`] can
    /// draw into and that can then be drawn like any other image.
    pub fn create_render_target(
        &mut self,
        width: u32,
        height: u32,
    ) -> Result<ImageHandle, RenderError> {
        self.check_image_size((width, height))?;
        let handle = self
            .images
            .insert((width, height), Source::Target(None))
            .ok_or_else(|| RenderError::Image("out of image handles".into()))?;
        self.make_gpu_image(handle);
        Ok(handle)
    }

    fn check_image_size(&self, (w, h): (u32, u32)) -> Result<(), RenderError> {
        let limit = self.device.limits().max_texture_dimension_2d;
        if w == 0 || h == 0 || w > limit || h > limit {
            return Err(RenderError::Image(format!(
                "an image must be between 1x1 and {limit}x{limit} pixels, not {w}x{h}"
            )));
        }
        Ok(())
    }

    /// Forget an image and free its GPU memory. `false` if there was no such image. Drawing it afterwards is an
    /// error. A render target that was drawn from it keeps its pixels; if the device is rebuilt it is drawn again
    /// without the destroyed image.
    pub fn destroy_image(&mut self, image: ImageHandle) -> bool {
        self.images.remove(image)
    }

    /// The size of an image, `None` if it does not exist.
    pub fn image_size(&self, image: ImageHandle) -> Option<(u32, u32)> {
        self.images.size_of(image)
    }

    /// How many images are registered.
    pub fn image_count(&self) -> usize {
        self.images.len()
    }

    /// Draw `frame` into the render target `target`, replacing what it held.
    ///
    /// Coordinates are the **pixels of the target** (origin top left; the 1920x1080 canvas is not applied), and
    /// the target is cleared to `clear` first. The target holds **premultiplied alpha**, so `clear` is
    /// premultiplied before it is used, and the target is drawn with premultiplied blending later: semi-transparent
    /// pictures come out the same as when drawn directly.
    ///
    /// **A picture cannot be drawn into itself**: it is an error if `frame` draws `target`, or draws an image
    /// that was rendered (however indirectly) from `target`. Drawing one target into another is fine, to any
    /// depth: render the inner one first.
    ///
    /// The frame is kept (a copy) as the recipe of the target, so that it can be drawn again after the GPU
    /// device is rebuilt; a target drawn from other targets is replayed after them, and so shows *their latest*
    /// content, not the content they had when it was rendered.
    pub fn render_to_target(
        &mut self,
        target: ImageHandle,
        frame: &UiFrame,
        clear: Color,
    ) -> Result<(), RenderError> {
        self.check_device()?;
        match self.images.get(target) {
            Some(entry) if entry.is_target() => {}
            Some(_) => {
                return Err(RenderError::Image(format!(
                    "image {} is an uploaded image, not a render target",
                    target.id()
                )));
            }
            None => {
                return Err(RenderError::Image(format!(
                    "image {} is not registered",
                    target.id()
                )));
            }
        }
        let used: Vec<ImageHandle> = frame.images().collect();
        let images = &self.images;
        if would_loop(target, &used, &|h| images.rendered_from(h)) {
            return Err(RenderError::Image(format!(
                "the frame would draw render target {} into itself",
                target.id()
            )));
        }
        self.draw_into_target(target, frame, clear, true)?;
        if let Some(entry) = self.images.get_mut(target) {
            match &mut entry.source {
                Source::Target(Some(recipe)) => {
                    recipe.frame.clone_from(frame);
                    recipe.clear = clear;
                }
                source => {
                    *source = Source::Target(Some(Recipe {
                        frame: frame.clone(),
                        clear,
                    }));
                }
            }
        }
        Ok(())
    }

    fn draw_into_target(
        &mut self,
        target: ImageHandle,
        frame: &UiFrame,
        clear: Color,
        strict: bool,
    ) -> Result<(), RenderError> {
        let (size, texture) = match self.images.get(target) {
            Some(entry) => match &entry.gpu {
                Some(gpu) => (entry.size, gpu.texture.clone()),
                None => {
                    return Err(RenderError::Image(format!(
                        "render target {} has no GPU texture",
                        target.id()
                    )));
                }
            },
            None => {
                return Err(RenderError::Image(format!(
                    "image {} is not registered",
                    target.id()
                )));
            }
        };
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render target"),
            });
        let a = f64::from(clear.0[3]);
        let premultiplied = wgpu::Color {
            r: f64::from(clear.0[0]) * a,
            g: f64::from(clear.0[1]) * a,
            b: f64::from(clear.0[2]) * a,
            a,
        };
        // Pixels of the target: no scale, no offset.
        let canvas = Canvas {
            scale: 1.0,
            offset: Vec2 { x: 0.0, y: 0.0 },
        };
        self.draw(
            &view,
            &mut encoder,
            frame,
            Some(premultiplied),
            size,
            &canvas,
            strict,
        )?;
        self.queue.submit(Some(encoder.finish()));
        self.text.trim();
        Ok(())
    }

    /// Read an image back as RGBA8. Slow (it waits for the GPU): meant for tests and tools. A render target
    /// holds premultiplied alpha; an uploaded image is returned as it was given.
    pub fn read_image(&mut self, image: ImageHandle) -> Result<ImageRgba8, RenderError> {
        self.check_device()?;
        let Some(entry) = self.images.get(image) else {
            return Err(RenderError::Image(format!(
                "image {} is not registered",
                image.id()
            )));
        };
        if let Source::Pixels(pixels) = &entry.source {
            return Ok((**pixels).clone());
        }
        let size = entry.size;
        let Some(gpu) = &entry.gpu else {
            return Err(RenderError::Image("the image has no GPU texture".into()));
        };
        let swap = match self.format {
            wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb => false,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => true,
            other => {
                return Err(RenderError::Image(format!(
                    "a render target of format {other:?} cannot be read back as RGBA8"
                )));
            }
        };
        let texture = gpu.texture.clone();
        let encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("read image"),
            });
        let pixels = self.read_texture(encoder, &texture, size, swap)?;
        Ok(ImageRgba8 {
            width: size.0,
            height: size.1,
            pixels,
        })
    }

    /// Make the GPU objects of image `handle` (its content is uploaded, or left transparent for a target).
    fn make_gpu_image(&mut self, handle: ImageHandle) {
        let Some(entry) = self.images.get(handle) else {
            return;
        };
        let (w, h) = entry.size;
        let extent = wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        };
        let texture = match &entry.source {
            Source::Pixels(image) => {
                let format = if self.format.is_srgb() {
                    wgpu::TextureFormat::Rgba8UnormSrgb
                } else {
                    wgpu::TextureFormat::Rgba8Unorm
                };
                self.device.create_texture_with_data(
                    &self.queue,
                    &wgpu::TextureDescriptor {
                        label: Some("image"),
                        size: extent,
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    },
                    wgpu::util::TextureDataOrder::LayerMajor,
                    &image.pixels,
                )
            }
            Source::Target(_) => self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("render target"),
                size: extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            }),
        };
        let bind_group = texture_group(&self.device, &self.texture_layout, &texture, "image");
        if let Some(entry) = self.images.get_mut(handle) {
            entry.gpu = Some(GpuImage {
                texture,
                bind_group,
            });
        }
    }

    /// Make the GPU objects of every image again (after the device was rebuilt) and draw the render targets
    /// again, each after the images it was drawn from.
    fn restore_images(&mut self) -> Result<(), RenderError> {
        let handles: Vec<ImageHandle> = self.images.handles().collect();
        for handle in handles {
            self.make_gpu_image(handle);
        }
        for handle in replay_order(&self.images.dependencies()) {
            let recipe = match self.images.get(handle) {
                Some(entry) => match &entry.source {
                    Source::Target(Some(recipe)) => Some((recipe.frame.clone(), recipe.clear)),
                    _ => None,
                },
                None => None,
            };
            if let Some((frame, clear)) = recipe {
                self.draw_into_target(handle, &frame, clear, false)?;
            }
        }
        Ok(())
    }
}

fn texture_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    texture: &wgpu::Texture,
    label: &str,
) -> wgpu::BindGroup {
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&view),
        }],
    })
}

pub(crate) fn capture_texture(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    (width, height): (u32, u32),
) -> Result<wgpu::Texture, RenderError> {
    let limit = device.limits().max_texture_dimension_2d;
    if width > limit || height > limit {
        return Err(RenderError::Device(format!(
            "capture size {width}x{height} exceeds the texture limit of {limit}"
        )));
    }
    Ok(device.create_texture(&wgpu::TextureDescriptor {
        label: Some("capture texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    }))
}

fn pack_capture_rows(data: &[u8], row_bytes: usize, pitch: usize, swap_red_blue: bool) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(data.len() / pitch * row_bytes);
    for row in data.chunks_exact(pitch) {
        pixels.extend_from_slice(&row[..row_bytes]);
    }
    if swap_red_blue {
        for pixel in pixels.as_chunks_mut::<4>().0 {
            pixel.swap(0, 2);
        }
    }
    pixels
}

fn present_mode(vsync: bool) -> wgpu::PresentMode {
    if vsync {
        wgpu::PresentMode::AutoVsync
    } else {
        wgpu::PresentMode::AutoNoVsync
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bgra_capture_rows_are_unpadded_and_swizzled() {
        let rows = [10, 20, 30, 40, 0, 0, 0, 0, 50, 60, 70, 80, 0, 0, 0, 0];
        assert_eq!(
            pack_capture_rows(&rows, 4, 8, true),
            [30, 20, 10, 40, 70, 60, 50, 80]
        );
        assert_eq!(
            pack_capture_rows(&rows, 4, 8, false),
            [10, 20, 30, 40, 50, 60, 70, 80]
        );
    }

    #[test]
    fn floating_point_target_can_be_captured_as_rgba8() {
        let _gpu = crate::gpu::gpu_test_guard();
        let config = GpuConfig::from_env();
        let instance = gpu::make_instance(&config, None);
        let format = wgpu::TextureFormat::Rgba16Float;
        let gpu = match pollster::block_on(gpu::request_gpu(&instance, None, Some(format), &config))
        {
            Ok(gpu) => gpu,
            Err(RenderError::NoAdapter(reason)) => {
                eprintln!("SKIPPED (no GPU adapter): {reason}");
                return;
            }
            Err(error) => panic!("renderer creation failed: {error}"),
        };
        let size = (37, 19);
        let texture = capture_texture(&gpu.device, format, size).unwrap();
        let mut renderer = Renderer::build(gpu, Target::Offscreen { texture }, size);
        let capture = renderer
            .capture_frame(None, Color([0.0, 0.0, 1.0, 1.0]))
            .unwrap();
        assert_eq!((capture.width, capture.height), size);
        assert_eq!(capture.pixels.len(), (size.0 * size.1 * 4) as usize);
        assert!(
            capture
                .pixels
                .as_chunks::<4>()
                .0
                .iter()
                .all(|pixel| *pixel == [0, 0, 255, 255])
        );
    }

    fn offscreen_or_skip(size: (u32, u32)) -> Option<Renderer> {
        match Renderer::new_offscreen(size) {
            Ok(renderer) => Some(renderer),
            Err(RenderError::NoAdapter(reason)) => {
                eprintln!("SKIPPED (no GPU adapter): {reason}");
                None
            }
            Err(error) => panic!("renderer creation failed: {error}"),
        }
    }

    fn one_red_square() -> DrawList {
        let mut list = DrawList::default();
        list.quads.push(ss2_ui::Quad {
            rect: ss2_ui::Rect {
                x: 100.0,
                y: 100.0,
                w: 400.0,
                h: 300.0,
            },
            color: Color([1.0, 0.0, 0.0, 1.0]),
        });
        list
    }

    #[test]
    fn a_device_that_is_destroyed_is_rebuilt_and_the_same_picture_comes_out() {
        let _gpu = crate::gpu::gpu_test_guard();
        let Some(mut renderer) = offscreen_or_skip((160, 90)) else {
            return;
        };
        let list = one_red_square();
        let clear = Color([0.0, 0.0, 0.2, 1.0]);
        let before = renderer.render_to_image(&list, clear).unwrap();
        assert_eq!(renderer.stats(), RenderStats::default());
        assert!(
            before
                .pixels
                .as_chunks::<4>()
                .0
                .iter()
                .any(|p| p[0] > 200 && p[1] < 50),
            "the square is drawn before the loss"
        );

        renderer.device.destroy();
        let after = renderer
            .render_to_image(&list, clear)
            .expect("the next frame rebuilds the device and draws");
        assert_eq!(after, before, "the rebuilt renderer draws the same picture");
        assert_eq!(renderer.stats().device_rebuilds, 1);
        assert!(!renderer.lost.is_set(), "the new device is not lost");
        let lines = renderer.report().lines();
        assert!(lines[0].starts_with("GPU: "), "{lines:?}");

        // And it can happen again.
        renderer.device.destroy();
        assert_eq!(renderer.render_to_image(&list, clear).unwrap(), before);
        assert_eq!(renderer.stats().device_rebuilds, 2);
    }

    #[test]
    fn a_device_that_keeps_being_lost_ends_in_an_error_not_an_endless_loop() {
        let _gpu = crate::gpu::gpu_test_guard();
        let Some(mut renderer) = offscreen_or_skip((32, 32)) else {
            return;
        };
        let list = DrawList::default();
        let clear = Color([0.0, 0.0, 0.0, 1.0]);
        let mut outcome = Ok(());
        for _ in 0..10 {
            renderer.device.destroy();
            outcome = renderer.render_to_image(&list, clear).map(|_| ());
            if outcome.is_err() {
                break;
            }
        }
        assert!(
            matches!(outcome, Err(RenderError::DeviceLost(_))),
            "after the allowed rebuilds it gives up: {outcome:?}"
        );
        assert_eq!(
            renderer.stats().device_rebuilds,
            3,
            "three rebuilds are allowed within the window"
        );
    }

    #[test]
    fn render_looks_for_a_lost_device_before_anything_else() {
        let _gpu = crate::gpu::gpu_test_guard();
        let Some(mut renderer) = offscreen_or_skip((32, 32)) else {
            return;
        };
        renderer.device.destroy();
        // An offscreen renderer cannot render() to a window, but the lost device is noticed first.
        let result = renderer.render(&DrawList::default(), Color([0.0, 0.0, 0.0, 1.0]));
        assert!(matches!(result, Err(RenderError::Surface(_))), "{result:?}");
        assert_eq!(renderer.stats().device_rebuilds, 1);
    }

    #[test]
    fn a_healthy_renderer_never_rebuilds() {
        let _gpu = crate::gpu::gpu_test_guard();
        let Some(mut renderer) = offscreen_or_skip((32, 32)) else {
            return;
        };
        let list = one_red_square();
        for _ in 0..20 {
            renderer
                .render_to_image(&list, Color([0.0, 0.0, 0.0, 1.0]))
                .unwrap();
        }
        assert_eq!(renderer.stats(), RenderStats::default());
    }
}
