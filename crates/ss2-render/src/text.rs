//! Text drawing with `glyphon` and the embedded Inter font.

use glyphon::cosmic_text::{Align, fontdb};
use glyphon::{
    Attrs, Buffer, Cache, Color as GlyphColor, Family, FontSystem, Metrics, Resolution, Shaping,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};
use ss2_ui::{Canvas, TextAlign, TextRun};

use crate::RenderError;

const INTER: &[u8] = include_bytes!("../assets/Inter-Regular.ttf");

/// Wide enough that placeholder labels never wrap.
const BOX_WIDTH: f32 = 1400.0;

pub(crate) struct TextPainter {
    font_system: FontSystem,
    swash: SwashCache,
    viewport: Viewport,
    atlas: TextAtlas,
    renderer: TextRenderer,
}

impl TextPainter {
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Self {
        // Only the embedded font: no system font scan, so output is the same on every machine.
        let mut db = fontdb::Database::new();
        db.load_font_data(INTER.to_vec());
        let font_system = FontSystem::new_with_locale_and_db("en-US".to_string(), db);
        let cache = Cache::new(device);
        let viewport = Viewport::new(device, &cache);
        let mut atlas = TextAtlas::new(device, queue, &cache, format);
        let renderer =
            TextRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), None);
        Self {
            font_system,
            swash: SwashCache::new(),
            viewport,
            atlas,
            renderer,
        }
    }

    pub(crate) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: (u32, u32),
        canvas: &Canvas,
        texts: &[TextRun],
    ) -> Result<(), RenderError> {
        self.viewport.update(
            queue,
            Resolution {
                width: size.0,
                height: size.1,
            },
        );
        let attrs = Attrs::new().family(Family::Name("Inter"));
        let scale = canvas.scale;
        let mut buffers = Vec::with_capacity(texts.len());
        for run in texts {
            let px = run.size * scale;
            let mut buffer = Buffer::new(&mut self.font_system, Metrics::new(px, px * 1.25));
            buffer.set_size(Some(BOX_WIDTH * scale), Some(px * 2.0));
            let align = match run.align {
                TextAlign::Left => None,
                TextAlign::Center => Some(Align::Center),
            };
            buffer.set_text(&run.text, &attrs, Shaping::Advanced, align);
            buffer.shape_until_scroll(&mut self.font_system, false);
            buffers.push(buffer);
        }
        let areas: Vec<TextArea<'_>> = buffers
            .iter()
            .zip(texts)
            .map(|(buffer, run)| {
                let p = canvas.to_screen(run.pos);
                let left = match run.align {
                    TextAlign::Left => p.x,
                    TextAlign::Center => p.x - BOX_WIDTH * scale / 2.0,
                };
                let c = run.color.0;
                TextArea {
                    buffer,
                    left,
                    top: p.y,
                    scale: 1.0,
                    bounds: TextBounds {
                        left: 0,
                        top: 0,
                        right: size.0 as i32,
                        bottom: size.1 as i32,
                    },
                    default_color: GlyphColor::rgba(
                        (c[0] * 255.0).round() as u8,
                        (c[1] * 255.0).round() as u8,
                        (c[2] * 255.0).round() as u8,
                        (c[3] * 255.0).round() as u8,
                    ),
                    custom_glyphs: &[],
                }
            })
            .collect();
        self.renderer
            .prepare(
                device,
                queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                areas,
                &mut self.swash,
            )
            .map_err(|e| RenderError::Device(format!("text prepare: {e}")))
    }

    pub(crate) fn render<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
    ) -> Result<(), RenderError> {
        self.renderer
            .render(&self.atlas, &self.viewport, pass)
            .map_err(|e| RenderError::Device(format!("text render: {e}")))
    }

    pub(crate) fn trim(&mut self) {
        self.atlas.trim();
    }
}
