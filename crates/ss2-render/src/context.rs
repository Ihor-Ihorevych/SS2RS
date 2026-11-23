//! The wgpu renderer: windowed or offscreen, quads + text.

use ss2_ui::{Canvas, Color, DrawList};
use wgpu::util::DeviceExt;

use crate::batch::{Vertex, quads_to_vertices};
use crate::text::TextPainter;

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
}

/// Draws a [`DrawList`] to a window or to an offscreen image.
pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    target: Target,
    size: (u32, u32),
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform: wgpu::Buffer,
    vertices: wgpu::Buffer,
    vertex_capacity: u64,
    text: TextPainter,
}

fn to_wgpu_color(c: Color) -> wgpu::Color {
    wgpu::Color {
        r: c.0[0] as f64,
        g: c.0[1] as f64,
        b: c.0[2] as f64,
        a: c.0[3] as f64,
    }
}

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    format: wgpu::TextureFormat,
}

async fn request_gpu(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'_>>,
    format_override: Option<wgpu::TextureFormat>,
) -> Result<(Gpu, wgpu::Adapter), RenderError> {
    let mut last_error = String::from("no adapter returned");
    for &fallback in crate::golden::adapter_modes(crate::golden::force_software_requested()) {
        let options = wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: fallback,
            compatible_surface: surface,
            ..Default::default()
        };
        match instance.request_adapter(&options).await {
            Ok(adapter) => {
                let (device, queue) = adapter
                    .request_device(&wgpu::DeviceDescriptor::default())
                    .await
                    .map_err(|e| RenderError::Device(e.to_string()))?;
                let format = match (format_override, surface) {
                    (Some(f), _) => f,
                    (None, Some(s)) => {
                        let caps = s.get_capabilities(&adapter);
                        caps.formats
                            .iter()
                            .copied()
                            .find(|f| !f.is_srgb())
                            .or_else(|| caps.formats.first().copied())
                            .ok_or_else(|| {
                                RenderError::Surface("surface reports no formats".into())
                            })?
                    }
                    (None, None) => wgpu::TextureFormat::Rgba8Unorm,
                };
                return Ok((
                    Gpu {
                        device,
                        queue,
                        format,
                    },
                    adapter,
                ));
            }
            Err(e) => last_error = e.to_string(),
        }
    }
    Err(RenderError::NoAdapter(last_error))
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
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(display));
        let surface = instance
            .create_surface(target)
            .map_err(|e| RenderError::Surface(e.to_string()))?;
        let (gpu, adapter) = pollster::block_on(request_gpu(&instance, Some(&surface), None))?;
        let caps = surface.get_capabilities(&adapter);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: gpu.format,
            width: size.0.max(1),
            height: size.1.max(1),
            present_mode: present_mode(vsync),
            alpha_mode: caps
                .alpha_modes
                .first()
                .copied()
                .unwrap_or(wgpu::CompositeAlphaMode::Auto),
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
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let (gpu, _adapter) = pollster::block_on(request_gpu(&instance, None, None))?;
        let texture = capture_texture(&gpu.device, gpu.format, (size.0.max(1), size.1.max(1)))?;
        Ok(Self::build(gpu, Target::Offscreen { texture }, size))
    }

    fn build(gpu: Gpu, target: Target, size: (u32, u32)) -> Renderer {
        let Gpu {
            device,
            queue,
            format,
        } = gpu;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("quads"),
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
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("quads layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("quads"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4],
                })],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let vertex_capacity = 6 * 256;
        let vertices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("quad vertices"),
            size: vertex_capacity * std::mem::size_of::<Vertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let text = TextPainter::new(&device, &queue, format);
        Renderer {
            device,
            queue,
            target,
            size,
            pipeline,
            bind_group,
            uniform,
            vertices,
            vertex_capacity,
            text,
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
        if self.size.0 == 0 || self.size.1 == 0 {
            return Ok(());
        }
        let Target::Window { surface, config } = &self.target else {
            return Err(RenderError::Surface(
                "render() needs a windowed renderer".into(),
            ));
        };
        let frame = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated
            | wgpu::CurrentSurfaceTexture::Suboptimal(_)
            | wgpu::CurrentSurfaceTexture::Lost => {
                surface.configure(&self.device, config);
                return Ok(());
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
        self.draw(&view, &mut encoder, list, clear)?;
        self.queue.submit(Some(encoder.finish()));
        self.queue.present(frame);
        self.text.trim();
        Ok(())
    }

    /// Draw `list` into the offscreen texture and read it back as RGBA8.
    pub fn render_to_image(
        &mut self,
        list: &DrawList,
        clear: Color,
    ) -> Result<ImageRgba8, RenderError> {
        if !matches!(self.target, Target::Offscreen { .. }) {
            return Err(RenderError::Device(
                "render_to_image() needs an offscreen renderer".into(),
            ));
        }
        self.capture_image(Some(list), clear, (self.size.0.max(1), self.size.1.max(1)))
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
        if self.size.0 == 0 || self.size.1 == 0 {
            return Err(RenderError::Device(
                "cannot capture a zero-size framebuffer".into(),
            ));
        }
        self.capture_image(overlay, clear, self.size)
    }

    fn capture_image(
        &mut self,
        overlay: Option<&DrawList>,
        clear: Color,
        size: (u32, u32),
    ) -> Result<ImageRgba8, RenderError> {
        let format = match &self.target {
            Target::Window { config, .. } => config.format,
            Target::Offscreen { texture } => texture.format(),
        };
        match format {
            wgpu::TextureFormat::Rgba8Unorm
            | wgpu::TextureFormat::Rgba8UnormSrgb
            | wgpu::TextureFormat::Bgra8Unorm
            | wgpu::TextureFormat::Bgra8UnormSrgb => self.read_frame(overlay, clear, size, format),
            _ => {
                let format = wgpu::TextureFormat::Rgba8Unorm;
                let texture = capture_texture(&self.device, format, size)?;
                let gpu = Gpu {
                    device: self.device.clone(),
                    queue: self.queue.clone(),
                    format,
                };
                Self::build(gpu, Target::Offscreen { texture }, size)
                    .read_frame(overlay, clear, size, format)
            }
        }
    }

    fn read_frame(
        &mut self,
        overlay: Option<&DrawList>,
        clear: Color,
        (w, h): (u32, u32),
        format: wgpu::TextureFormat,
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
        let row_bytes = w * 4;
        let bytes_per_row = row_bytes.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let readback_size = u64::from(bytes_per_row) * u64::from(h);
        if readback_size > self.device.limits().max_buffer_size {
            return Err(RenderError::Device(format!(
                "capture readback buffer is too large for {w}x{h}"
            )));
        }
        self.draw(
            &view,
            &mut encoder,
            overlay.unwrap_or(&DrawList::default()),
            clear,
        )?;
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: readback_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
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
        self.text.trim();
        Ok(ImageRgba8 {
            width: w,
            height: h,
            pixels,
        })
    }

    fn draw(
        &mut self,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        list: &DrawList,
        clear: Color,
    ) -> Result<(), RenderError> {
        let size = (self.size.0.max(1), self.size.1.max(1));
        let canvas = Canvas::fit(size.0, size.1).expect("size is non-zero");
        let vertices = quads_to_vertices(list, &canvas);
        let needed = vertices.len() as u64;
        if needed > self.vertex_capacity {
            self.vertex_capacity = needed.next_power_of_two();
            self.vertices = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("quad vertices"),
                size: self.vertex_capacity * std::mem::size_of::<Vertex>() as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !vertices.is_empty() {
            self.queue
                .write_buffer(&self.vertices, 0, bytemuck::cast_slice(&vertices));
        }
        self.queue.write_buffer(
            &self.uniform,
            0,
            bytemuck::cast_slice(&[size.0 as f32, size.1 as f32, 0.0, 0.0]),
        );
        self.text
            .prepare(&self.device, &self.queue, size, &canvas, &list.texts)?;

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ui"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(to_wgpu_color(clear)),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        if !vertices.is_empty() {
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.set_vertex_buffer(
                0,
                self.vertices
                    .slice(..needed * std::mem::size_of::<Vertex>() as u64),
            );
            pass.draw(0..vertices.len() as u32, 0..1);
        }
        self.text.render(&mut pass)
    }
}

fn capture_texture(
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
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let format = wgpu::TextureFormat::Rgba16Float;
        let gpu = match pollster::block_on(request_gpu(&instance, None, Some(format))) {
            Ok((gpu, _)) => gpu,
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
}
