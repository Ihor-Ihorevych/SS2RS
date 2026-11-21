//! The [`Renderer`] trait: what the simulation/app calls, whatever draws underneath.

use ss2_core::{Error, Result};

use crate::resources::{MockResources, RenderResources};
use crate::scene::RenderWorld;

/// What one [`Renderer::render`] call drew; for tests, overlays and profiling.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameStats {
    /// Tick of the newest world that was drawn (`None` if nothing was ever submitted).
    pub tick: Option<u64>,
    /// 3D objects drawn.
    pub renderables: usize,
    /// Dynamic lights used.
    pub lights: usize,
    /// UI quads drawn.
    pub ui_quads: usize,
    /// UI text runs drawn.
    pub ui_texts: usize,
}

/// The renderer as seen by the rest of the engine. The wgpu implementation lives in `ss2-render`
/// (T04-026); [`NullRenderer`] is the headless/test implementation.
///
/// ## Threading and frame flow
/// The simulation runs at a fixed tick rate; rendering runs as fast as the display allows. Each
/// tick the sim builds a [`RenderWorld`] and [`submit`](Self::submit)s it (by value, so it can be
/// moved to a render thread). Each display frame calls [`render`](Self::render) with `alpha` in
/// `[0, 1]`: how far display time has advanced from the *previous* submitted world (`0`) to the
/// *newest* one (`1`). The renderer therefore shows the game at most one tick behind the sim, in
/// exchange for smooth motion at any frame rate.
///
/// **Interpolation rule:** with previous world `P` and newest world `N`, an object present in
/// both (same [`RenderId`](crate::RenderId)) is drawn at `P.transform.interpolate(N.transform,
/// alpha)`; an object only in `N` is drawn at its `N` transform; one only in `P` is not drawn.
/// The camera is interpolated the same way. UI is always taken from `N`.
pub trait Renderer {
    /// Resolve asset paths to ids.
    fn resources(&mut self) -> &mut dyn RenderResources;

    /// Hand over the extract for a new tick. The newest submission wins; ticks must strictly
    /// increase.
    ///
    /// # Errors
    /// [`Error::Invalid`] if `world` fails [`RenderWorld::validate`] or its tick does not
    /// increase; the world is dropped and the previous state is kept. Never panics.
    fn submit(&mut self, world: RenderWorld) -> Result<()>;

    /// Draw one display frame, blending the last two submitted worlds by `alpha` in `[0, 1]`.
    ///
    /// # Errors
    /// [`Error::Invalid`] if `alpha` is not finite. Out-of-range finite values are clamped.
    /// Implementations with a GPU may return other errors (surface lost, device lost) which the
    /// caller recovers from per the platform contract (T00-016).
    fn render(&mut self, alpha: f32) -> Result<FrameStats>;
}

/// A [`Renderer`] that validates and counts but draws nothing: for the headless dedicated server,
/// and for tests of code that feeds a renderer.
///
/// ```
/// use ss2_render_api::{NullRenderer, RenderId, RenderWorld, Renderable, Renderer, Transform};
///
/// let mut renderer = NullRenderer::new();
/// let model = renderer.resources().model("models/player.mdl");
///
/// let mut world = RenderWorld::new(1);
/// world.renderables.push(Renderable::new(RenderId(1), model, Transform::IDENTITY));
/// renderer.submit(world).unwrap();
///
/// let stats = renderer.render(0.5).unwrap();
/// assert_eq!((stats.tick, stats.renderables), (Some(1), 1));
/// ```
#[derive(Debug, Default)]
pub struct NullRenderer {
    resources: MockResources,
    previous: Option<RenderWorld>,
    newest: Option<RenderWorld>,
}

impl NullRenderer {
    /// A renderer with nothing submitted.
    pub fn new() -> Self {
        Self::default()
    }

    /// The newest accepted world.
    pub fn newest(&self) -> Option<&RenderWorld> {
        self.newest.as_ref()
    }

    /// The world before the newest one (what interpolation blends from).
    pub fn previous(&self) -> Option<&RenderWorld> {
        self.previous.as_ref()
    }

    /// The mock resource registry, for looking paths up in tests.
    pub fn mock_resources(&self) -> &MockResources {
        &self.resources
    }
}

impl Renderer for NullRenderer {
    fn resources(&mut self) -> &mut dyn RenderResources {
        &mut self.resources
    }

    fn submit(&mut self, world: RenderWorld) -> Result<()> {
        world.validate()?;
        if let Some(newest) = &self.newest
            && world.tick <= newest.tick
        {
            return Err(Error::invalid(
                "render world",
                format!("tick {} does not increase past {}", world.tick, newest.tick),
            ));
        }
        self.previous = self.newest.replace(world);
        Ok(())
    }

    fn render(&mut self, alpha: f32) -> Result<FrameStats> {
        if !alpha.is_finite() {
            return Err(Error::invalid(
                "render alpha",
                format!("{alpha} is not finite"),
            ));
        }
        Ok(match &self.newest {
            Some(w) => FrameStats {
                tick: Some(w.tick),
                renderables: w.renderables.len(),
                lights: w.lights.len(),
                ui_quads: w.ui.quads.len(),
                ui_texts: w.ui.texts.len(),
            },
            None => FrameStats::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use ss2_ui::{Color, Quad, Rect};

    use super::*;
    use crate::ids::RenderId;
    use crate::scene::{Renderable, Transform};

    fn world(tick: u64, objects: u64) -> RenderWorld {
        let mut w = RenderWorld::new(tick);
        let model = crate::ModelId::from_raw(1).unwrap();
        for i in 0..objects {
            w.renderables
                .push(Renderable::new(RenderId(i), model, Transform::IDENTITY));
        }
        w
    }

    #[test]
    fn nothing_submitted_draws_nothing() {
        let mut r = NullRenderer::new();
        assert_eq!(r.render(0.0).unwrap(), FrameStats::default());
        assert!(r.newest().is_none());
    }

    #[test]
    fn keeps_the_last_two_worlds() {
        let mut r = NullRenderer::new();
        r.submit(world(1, 1)).unwrap();
        assert!(r.previous().is_none());
        r.submit(world(2, 2)).unwrap();
        r.submit(world(3, 3)).unwrap();
        assert_eq!(r.previous().unwrap().tick, 2);
        assert_eq!(r.newest().unwrap().tick, 3);
        assert_eq!(r.render(0.3).unwrap().renderables, 3);
    }

    #[test]
    fn rejects_invalid_and_stale_worlds_and_keeps_state() {
        let mut r = NullRenderer::new();
        r.submit(world(5, 1)).unwrap();
        assert!(r.submit(world(5, 1)).is_err(), "same tick");
        assert!(r.submit(world(4, 1)).is_err(), "older tick");
        let mut bad = world(6, 2);
        bad.renderables[1].id = bad.renderables[0].id;
        assert!(r.submit(bad).is_err(), "duplicate ids");
        assert_eq!(r.newest().unwrap().tick, 5);
        assert!(r.previous().is_none());
    }

    #[test]
    fn alpha_must_be_finite_but_is_otherwise_clamped_not_rejected() {
        let mut r = NullRenderer::new();
        r.submit(world(1, 1)).unwrap();
        assert!(r.render(f32::NAN).is_err());
        assert!(r.render(f32::INFINITY).is_err());
        assert!(r.render(-3.0).is_ok());
        assert!(r.render(7.0).is_ok());
    }

    #[test]
    fn frame_stats_count_ui() {
        let mut r = NullRenderer::new();
        let mut w = world(1, 0);
        w.ui.quads.push(Quad {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                w: 10.0,
                h: 10.0,
            },
            color: Color([1.0; 4]),
        });
        r.submit(w).unwrap();
        assert_eq!(r.render(0.0).unwrap().ui_quads, 1);
    }

    #[test]
    fn usable_as_trait_object_with_resources() {
        let mut boxed: Box<dyn Renderer> = Box::new(NullRenderer::new());
        let m = boxed.resources().model("a.mdl");
        assert_eq!(m.raw(), 1);
    }
}
