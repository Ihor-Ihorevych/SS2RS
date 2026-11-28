//! The bloom chain (T04-043, RFC 0013 amendment 6, `docs/re/environment.md` "Bloom", T01-417/T01-423/T01-424).
//!
//! With any active bloom the world is drawn into an intermediate texture of the frame's format ([`BloomGpu::scene`]);
//! each [`Bloom`] then runs the original's chain on it in turn, adding onto it, so a later bloom sees an earlier one's
//! result; finally the intermediate is copied to the frame, under the 2D overlay. Per bloom:
//!
//! 1. `intensity = clamp(fade * strength * BLOOM_BIAS, 0, 2)`; below [`MIN_INTENSITY`] or on a frame smaller than
//!    32x32 nothing is done.
//! 2. Quarter-size steps (`ceil(side / 4)`) until both sides are at most 128: each step is a 4x4 box filter drawn
//!    twice at two blur taps and averaged. The taps are 1.5 source texels times `blur`, shared between the two by
//!    `balance` (equal at 1), alternating horizontal and vertical from step to step.
//! 3. One pass: the high-pass (`k = min(high_pass * sqrt(intensity), 1)`; what is brighter than `1 - k` survives,
//!    stretched back to 0..1) and the tint by the fixed three-colour ramp at `ramp`, normalised to its brightest
//!    channel.
//! 4. Added onto the frame times `min(colour.rgb * fade, 1) * intensity`; the frame's alpha is not touched.
//!
//! The spec reads the pass order and the members from the original but not every formula: the high-pass and tint
//! arithmetic, the tap directions and the bias's default are this module's readings, provisional until checked by
//! eye against the original.

use ss2_render_api::scene_data::Bloom;
use wgpu::util::DeviceExt;

/// The video option scaling every bloom (`efx_fBloomBias`); its default is not in the spec (provisional).
pub const BLOOM_BIAS: f32 = 1.0;
/// Below this intensity a bloom is skipped.
pub const MIN_INTENSITY: f32 = 0.01;
/// The largest side of the smallest step.
const SMALLEST: u32 = 128;
/// The fixed colour ramp, at `ramp` 0, 1 and 2 (`docs/re/environment.md`; the component order is not confirmed).
const RAMP: [[f32; 3]; 3] = [[0.66, 0.10, 0.05], [0.24, 0.21, 0.10], [0.16, 0.15, 0.14]];
const STEP_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// The ramp colour at `ramp` (clamped to 0..=2), scaled so its brightest channel is 1.
pub fn ramp_colour(ramp: f32) -> [f32; 3] {
    let r = if ramp.is_finite() {
        ramp.clamp(0.0, 2.0)
    } else {
        0.0
    };
    let i = (r.floor() as usize).min(1);
    let t = r - i as f32;
    let c: [f32; 3] = std::array::from_fn(|j| RAMP[i][j] + (RAMP[i + 1][j] - RAMP[i][j]) * t);
    let top = c.iter().copied().fold(0.0f32, f32::max).max(1e-6);
    c.map(|v| v / top)
}

/// The step sizes from a `(w, h)` frame: quartered (rounding up) until both sides are at most 128; at least one.
pub fn step_sizes((w, h): (u32, u32)) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    let (mut w, mut h) = (w.max(1), h.max(1));
    loop {
        w = w.div_ceil(4);
        h = h.div_ceil(4);
        out.push((w, h));
        if w <= SMALLEST && h <= SMALLEST {
            return out;
        }
    }
}

/// One bloom's intensity, or `None` when it adds nothing.
pub fn intensity(b: &Bloom) -> Option<f32> {
    let i = (b.fade * b.strength * BLOOM_BIAS).clamp(0.0, 2.0);
    (i >= MIN_INTENSITY).then_some(i)
}

struct Target {
    view: wgpu::TextureView,
    size: (u32, u32),
}

/// The bloom pipelines and their textures on one device.
pub(crate) struct BloomGpu {
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    down: wgpu::RenderPipeline,
    tint: wgpu::RenderPipeline,
    add: wgpu::RenderPipeline,
    copy: wgpu::RenderPipeline,
    /// The world is drawn here when a bloom is active.
    scene: Option<(wgpu::Texture, Target)>,
    /// One texture per quarter-size step, and one more at the last step's size for the tint pass.
    steps: Vec<Target>,
    tinted: Option<Target>,
    format: wgpu::TextureFormat,
}

impl BloomGpu {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bloom"),
            source: wgpu::ShaderSource::Wgsl(include_str!("bloom.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bloom layout"),
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
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bloom pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let make = |label: &str, entry: &str, format, blend: Option<wgpu::BlendState>, writes| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend,
                        write_mask: writes,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let additive = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::OVER,
        };
        BloomGpu {
            down: make(
                "bloom down",
                "fs_down",
                STEP_FORMAT,
                None,
                wgpu::ColorWrites::ALL,
            ),
            tint: make(
                "bloom tint",
                "fs_tint",
                STEP_FORMAT,
                None,
                wgpu::ColorWrites::ALL,
            ),
            add: make(
                "bloom add",
                "fs_add",
                format,
                Some(additive),
                wgpu::ColorWrites::COLOR,
            ),
            copy: make(
                "bloom copy",
                "fs_copy",
                format,
                None,
                wgpu::ColorWrites::ALL,
            ),
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("bloom sampler"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
            layout,
            scene: None,
            steps: Vec::new(),
            tinted: None,
            format,
        }
    }

    /// The texture the world is drawn into this frame, made or remade for `size`.
    pub(crate) fn scene(&mut self, device: &wgpu::Device, size: (u32, u32)) -> &wgpu::TextureView {
        if self.scene.as_ref().is_none_or(|(_, t)| t.size != size) {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("bloom scene"),
                size: extent(size),
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: self.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.scene = Some((texture, Target { view, size }));
        }
        &self.scene.as_ref().expect("made above").1.view
    }

    /// Run every bloom on the scene texture (drawn by [`BloomGpu::scene`]) and copy the result to `frame`.
    pub(crate) fn encode(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        frame: &wgpu::TextureView,
        size: (u32, u32),
        blooms: &[Bloom],
    ) {
        self.scene(device, size);
        let sizes = step_sizes(size);
        if self.steps.iter().map(|t| t.size).ne(sizes.iter().copied()) {
            self.steps = sizes.iter().map(|s| step_target(device, *s)).collect();
            self.tinted = Some(step_target(device, *sizes.last().expect("at least one")));
        }
        let big_enough = size.0 >= 32 && size.1 >= 32;
        for bloom in blooms.iter().filter(|_| big_enough) {
            let Some(intensity) = intensity(bloom) else {
                continue;
            };
            self.encode_one(device, encoder, bloom, intensity, size);
        }
        let scene = &self.scene.as_ref().expect("made above").1.view;
        let group = self.group(
            device,
            scene,
            params(size, [0.0; 2], [0.0; 2], 0.0, [0.0; 3]),
        );
        draw(encoder, frame, &self.copy, &group, Some(wgpu::Color::BLACK));
    }

    fn encode_one(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        bloom: &Bloom,
        intensity: f32,
        size: (u32, u32),
    ) {
        let scene = &self.scene.as_ref().expect("made by encode").1.view;
        let reach = 1.5 * bloom.blur;
        let share = bloom.balance.max(1e-3);
        let (a, b) = (
            reach * 2.0 * share / (1.0 + share),
            -reach * 2.0 / (1.0 + share),
        );
        let mut source = (scene, size);
        for (i, step) in self.steps.iter().enumerate() {
            let (tap_a, tap_b) = if i % 2 == 0 {
                ([a, 0.0], [b, 0.0])
            } else {
                ([0.0, a], [0.0, b])
            };
            let group = self.group(
                device,
                source.0,
                params(source.1, tap_a, tap_b, 0.0, [0.0; 3]),
            );
            draw(
                encoder,
                &step.view,
                &self.down,
                &group,
                Some(wgpu::Color::BLACK),
            );
            source = (&step.view, step.size);
        }
        let tinted = self.tinted.as_ref().expect("made by encode");
        let k = (bloom.high_pass * intensity.sqrt()).min(1.0);
        let group = self.group(
            device,
            source.0,
            params(source.1, [0.0; 2], [0.0; 2], k, ramp_colour(bloom.ramp)),
        );
        draw(
            encoder,
            &tinted.view,
            &self.tint,
            &group,
            Some(wgpu::Color::BLACK),
        );
        let add: [f32; 3] =
            std::array::from_fn(|j| (bloom.colour[j] * bloom.fade).clamp(0.0, 1.0) * intensity);
        let group = self.group(
            device,
            &tinted.view,
            params(tinted.size, [0.0; 2], [0.0; 2], 0.0, add),
        );
        draw(encoder, scene, &self.add, &group, None);
    }

    fn group(
        &self,
        device: &wgpu::Device,
        view: &wgpu::TextureView,
        params: [f32; 12],
    ) -> wgpu::BindGroup {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bloom params"),
            contents: bytemuck::cast_slice(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloom pass"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buffer.as_entire_binding(),
                },
            ],
        })
    }
}

fn params(
    source: (u32, u32),
    tap_a: [f32; 2],
    tap_b: [f32; 2],
    k: f32,
    tint: [f32; 3],
) -> [f32; 12] {
    [
        1.0 / source.0.max(1) as f32,
        1.0 / source.1.max(1) as f32,
        tap_a[0],
        tap_a[1],
        tap_b[0],
        tap_b[1],
        k,
        0.0,
        tint[0],
        tint[1],
        tint[2],
        1.0,
    ]
}

fn extent((w, h): (u32, u32)) -> wgpu::Extent3d {
    wgpu::Extent3d {
        width: w.max(1),
        height: h.max(1),
        depth_or_array_layers: 1,
    }
}

fn step_target(device: &wgpu::Device, size: (u32, u32)) -> Target {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bloom step"),
        size: extent(size),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: STEP_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    Target {
        view: texture.create_view(&wgpu::TextureViewDescriptor::default()),
        size,
    }
}

/// One full-screen pass into `view`; `clear` clears it first, `None` keeps what it holds.
fn draw(
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    group: &wgpu::BindGroup,
    clear: Option<wgpu::Color>,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("bloom pass"),
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
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, group, &[]);
    pass.draw(0..3, 0..1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_quarter_until_both_sides_are_at_most_128() {
        assert_eq!(step_sizes((1920, 1080)), [(480, 270), (120, 68)]);
        assert_eq!(step_sizes((3840, 2160)), [(960, 540), (240, 135), (60, 34)]);
        assert_eq!(step_sizes((64, 64)), [(16, 16)]);
        assert_eq!(step_sizes((0, 0)), [(1, 1)]);
    }

    #[test]
    fn the_ramp_runs_from_warm_to_grey_and_is_normalised() {
        assert_eq!(ramp_colour(0.0), [1.0, 0.10 / 0.66, 0.05 / 0.66]);
        let grey = ramp_colour(2.0);
        assert!(grey.iter().all(|c| *c > 0.85 && *c <= 1.0), "{grey:?}");
        assert_eq!(ramp_colour(5.0), ramp_colour(2.0));
        assert_eq!(ramp_colour(f32::NAN), ramp_colour(0.0));
        let mid = ramp_colour(1.5);
        assert!(mid[0] >= mid[1] && mid[1] >= mid[2]);
    }

    #[test]
    fn the_intensity_is_fade_times_strength_clamped_and_skipped_when_tiny() {
        let b = Bloom {
            colour: [0.2; 4],
            fade: 0.5,
            strength: 1.2,
            blur: 1.0,
            balance: 1.0,
            high_pass: 0.5,
            ramp: 1.0,
        };
        assert_eq!(intensity(&b), Some(0.6));
        assert_eq!(
            intensity(&Bloom {
                strength: 10.0,
                ..b
            }),
            Some(2.0)
        );
        assert_eq!(intensity(&Bloom { fade: 0.0, ..b }), None);
    }
}
