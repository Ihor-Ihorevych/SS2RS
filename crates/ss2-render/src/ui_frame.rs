//! The 2D drawing commands of the renderer: quads, text, images, 9-slice images and clip rectangles.
//!
//! A [`UiFrame`] is a list of [`UiCommand`]s drawn **in the order they are in** (back to front), in canvas units
//! (`ss2_ui::Canvas`; render targets of [`Renderer::render_to_target`](crate::Renderer::render_to_target) use
//! pixels). It is additive to `ss2_ui::DrawList`, which still works unchanged and can be put in a frame with
//! [`UiFrame::push_draw_list`]. `ss2-ui` does not produce these commands yet (follow-up T04-017b).
//!
//! Everything here is plain data and needs no GPU.

use std::borrow::Cow;

use ss2_ui::{Color, DrawList, Quad, Rect, TextAlign, TextRun, Vec2};

use crate::nine_slice::{Fill, Insets, UvRect};

/// A picture registered with a [`Renderer`](crate::Renderer) (an uploaded image or a render target).
///
/// A handle stays valid across a rebuilt GPU device. It belongs to the renderer that made it: using it on another
/// one is an error or, worse, the other renderer's image with the same number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ImageHandle(u32);

impl ImageHandle {
    pub(crate) const fn new(id: u32) -> ImageHandle {
        ImageHandle(id)
    }

    /// The number of the handle (only useful in a log line).
    pub const fn id(self) -> u32 {
        self.0
    }
}

/// How an image is sampled when it is drawn at a size other than its own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Filter {
    /// The nearest texel: hard pixel edges, and exact at 1:1.
    #[default]
    Nearest,
    /// Bilinear blend of the four nearest texels.
    Linear,
}

/// Draw part of an image into a rectangle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImageDraw {
    /// What to draw.
    pub image: ImageHandle,
    /// Where, in canvas units.
    pub rect: Rect,
    /// Which part of the image (0..=1; [`UvRect::FULL`] for all of it).
    pub uv: UvRect,
    /// Multiplied with every texel (white leaves the image as it is; straight alpha).
    pub tint: Color,
    /// Sampling.
    pub filter: Filter,
}

/// Draw a bordered image into a rectangle (see [`nine_slice`](crate::nine_slice)).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NineSliceDraw {
    /// What to draw.
    pub image: ImageHandle,
    /// Where, in canvas units.
    pub rect: Rect,
    /// The borders, in texels of the image.
    pub insets: Insets,
    /// Canvas units per texel for the borders (1.0 draws a texel one unit wide).
    pub scale: f32,
    /// What the edges and the centre do when the rectangle is larger than the image.
    pub fill: Fill,
    /// Multiplied with every texel.
    pub tint: Color,
    /// Sampling.
    pub filter: Filter,
}

/// One drawing command.
#[derive(Clone, Debug, PartialEq)]
pub enum UiCommand {
    /// A filled rectangle.
    Quad(Quad),
    /// A line of text.
    Text(TextRun),
    /// An image.
    Image(ImageDraw),
    /// A 9-slice image.
    NineSlice(NineSliceDraw),
    /// Clip everything after this, until the matching [`UiCommand::PopClip`], to the rectangle and to every
    /// rectangle pushed before it (see [`ScissorStack`](crate::ScissorStack)).
    PushClip(Rect),
    /// Undo the last [`UiCommand::PushClip`]. One without a push is ignored.
    PopClip,
}

/// A borrowed command: what the planner reads, from a [`UiFrame`] or a `DrawList` alike.
#[derive(Clone, Copy, Debug)]
pub(crate) enum CommandRef<'a> {
    Quad(&'a Quad),
    Text(&'a TextRun),
    Image(&'a ImageDraw),
    NineSlice(&'a NineSliceDraw),
    PushClip(Rect),
    PopClip,
}

/// Something the renderer can draw: it hands out its commands in drawing order.
pub(crate) trait UiScene {
    fn visit<'a>(&'a self, f: &mut dyn FnMut(CommandRef<'a>));
}

impl UiScene for DrawList {
    /// Quads first, then text: the order the renderer always drew a `DrawList` in.
    fn visit<'a>(&'a self, f: &mut dyn FnMut(CommandRef<'a>)) {
        for quad in &self.quads {
            f(CommandRef::Quad(quad));
        }
        for text in &self.texts {
            f(CommandRef::Text(text));
        }
    }
}

/// A list of drawing commands, drawn back to front.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UiFrame {
    commands: Vec<UiCommand>,
}

impl UiScene for UiFrame {
    fn visit<'a>(&'a self, f: &mut dyn FnMut(CommandRef<'a>)) {
        for command in &self.commands {
            f(match command {
                UiCommand::Quad(q) => CommandRef::Quad(q),
                UiCommand::Text(t) => CommandRef::Text(t),
                UiCommand::Image(i) => CommandRef::Image(i),
                UiCommand::NineSlice(n) => CommandRef::NineSlice(n),
                UiCommand::PushClip(r) => CommandRef::PushClip(*r),
                UiCommand::PopClip => CommandRef::PopClip,
            });
        }
    }
}

impl From<&DrawList> for UiFrame {
    fn from(list: &DrawList) -> Self {
        let mut frame = UiFrame::default();
        frame.push_draw_list(list);
        frame
    }
}

impl UiFrame {
    /// An empty frame.
    pub fn new() -> UiFrame {
        UiFrame::default()
    }

    /// Empty the frame **keeping its capacity**, so filling it again each frame does not allocate.
    pub fn clear(&mut self) {
        self.commands.clear();
    }

    /// The commands, in drawing order.
    pub fn commands(&self) -> &[UiCommand] {
        &self.commands
    }

    /// Whether there is nothing to draw.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Add any command.
    pub fn push(&mut self, command: UiCommand) -> &mut Self {
        self.commands.push(command);
        self
    }

    /// A filled rectangle.
    pub fn quad(&mut self, rect: Rect, color: Color) -> &mut Self {
        self.push(UiCommand::Quad(Quad { rect, color }))
    }

    /// A line of text (its top edge at `pos.y`; `pos.x` is the left edge or the centre).
    pub fn text(
        &mut self,
        pos: Vec2,
        text: impl Into<Cow<'static, str>>,
        size: f32,
        color: Color,
        align: TextAlign,
    ) -> &mut Self {
        self.push(UiCommand::Text(TextRun {
            pos,
            text: text.into(),
            size,
            color,
            align,
        }))
    }

    /// All of an image, stretched to `rect`, unchanged by any tint, sampled with the nearest texel.
    pub fn image(&mut self, image: ImageHandle, rect: Rect) -> &mut Self {
        self.image_ex(image, rect, UvRect::FULL, Color([1.0; 4]), Filter::Nearest)
    }

    /// Part of an image, with a tint and a sampling mode.
    pub fn image_ex(
        &mut self,
        image: ImageHandle,
        rect: Rect,
        uv: UvRect,
        tint: Color,
        filter: Filter,
    ) -> &mut Self {
        self.push(UiCommand::Image(ImageDraw {
            image,
            rect,
            uv,
            tint,
            filter,
        }))
    }

    /// A bordered image stretched to `rect` (borders `scale` units per texel, no tint, nearest texel).
    pub fn nine_slice(
        &mut self,
        image: ImageHandle,
        rect: Rect,
        insets: Insets,
        scale: f32,
    ) -> &mut Self {
        self.push(UiCommand::NineSlice(NineSliceDraw {
            image,
            rect,
            insets,
            scale,
            fill: Fill::Stretch,
            tint: Color([1.0; 4]),
            filter: Filter::Nearest,
        }))
    }

    /// Clip what follows to `rect`; balance with [`UiFrame::pop_clip`].
    pub fn push_clip(&mut self, rect: Rect) -> &mut Self {
        self.push(UiCommand::PushClip(rect))
    }

    /// End the clip of the matching [`UiFrame::push_clip`].
    pub fn pop_clip(&mut self) -> &mut Self {
        self.push(UiCommand::PopClip)
    }

    /// Add what a `DrawList` draws (its quads, then its text), in that order, after what is already here.
    pub fn push_draw_list(&mut self, list: &DrawList) -> &mut Self {
        self.commands
            .extend(list.quads.iter().map(|q| UiCommand::Quad(*q)));
        self.commands
            .extend(list.texts.iter().map(|t| UiCommand::Text(t.clone())));
        self
    }

    /// Every image the frame draws, in command order (an image used twice is listed twice).
    pub fn images(&self) -> impl Iterator<Item = ImageHandle> + '_ {
        self.commands.iter().filter_map(|c| match c {
            UiCommand::Image(i) => Some(i.image),
            UiCommand::NineSlice(n) => Some(n.image),
            _ => None,
        })
    }

    /// Whether the frame draws `image` anywhere.
    pub fn uses_image(&self, image: ImageHandle) -> bool {
        self.images().any(|i| i == image)
    }

    /// How many GPU draw calls (one per batch, a batch of text counting as one) drawing this frame takes on a
    /// `target`-sized image: consecutive commands with the same image, sampling and clip merge, skipped commands
    /// (under an empty clip) do not count. `size_of` gives the size of an image in texels (needed to cut a
    /// 9-slice); commands that name an image it does not know are skipped.
    pub fn draw_call_count(
        &self,
        canvas: &ss2_ui::Canvas,
        target: (u32, u32),
        size_of: &dyn Fn(ImageHandle) -> Option<(u32, u32)>,
    ) -> usize {
        crate::plan::Plan::build(self, canvas, target, size_of)
            .batches
            .len()
    }
}

/// Whether drawing `used` images into `target` would make a picture that depends on itself: `target` is among
/// them, or one of them was itself rendered from something that (however indirectly) uses `target`.
/// `rendered_from` gives the images an image was last rendered from (none for an uploaded image).
pub(crate) fn would_loop(
    target: ImageHandle,
    used: &[ImageHandle],
    rendered_from: &dyn Fn(ImageHandle) -> Vec<ImageHandle>,
) -> bool {
    let mut seen: Vec<ImageHandle> = Vec::new();
    let mut stack: Vec<ImageHandle> = used.to_vec();
    while let Some(image) = stack.pop() {
        if image == target {
            return true;
        }
        if seen.contains(&image) {
            continue;
        }
        seen.push(image);
        stack.extend(rendered_from(image));
    }
    false
}

/// The order in which to render images again so that each comes after the images it was rendered from.
/// `images` lists every image with the images it depends on; ties keep the order of `images`.
pub(crate) fn replay_order(images: &[(ImageHandle, Vec<ImageHandle>)]) -> Vec<ImageHandle> {
    fn visit(
        image: ImageHandle,
        images: &[(ImageHandle, Vec<ImageHandle>)],
        done: &mut Vec<ImageHandle>,
        order: &mut Vec<ImageHandle>,
    ) {
        if done.contains(&image) {
            return;
        }
        done.push(image);
        if let Some((_, from)) = images.iter().find(|(h, _)| *h == image) {
            for &dep in from {
                visit(dep, images, done, order);
            }
            order.push(image);
        }
    }
    let mut done = Vec::new();
    let mut order = Vec::new();
    for (image, _) in images {
        visit(*image, images, &mut done, &mut order);
    }
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(id: u32) -> ImageHandle {
        ImageHandle::new(id)
    }

    fn r(x: f32) -> Rect {
        Rect {
            x,
            y: 0.0,
            w: 10.0,
            h: 10.0,
        }
    }

    #[test]
    fn builders_add_commands_in_order() {
        let mut f = UiFrame::new();
        f.quad(r(0.0), Color([1.0; 4]))
            .push_clip(r(1.0))
            .image(h(1), r(2.0))
            .nine_slice(h(2), r(3.0), Insets::uniform(2.0), 1.0)
            .text(
                Vec2 { x: 0.0, y: 0.0 },
                "hi",
                20.0,
                Color([1.0; 4]),
                TextAlign::Left,
            )
            .pop_clip();
        let kinds: Vec<&str> = f
            .commands()
            .iter()
            .map(|c| match c {
                UiCommand::Quad(_) => "quad",
                UiCommand::Text(_) => "text",
                UiCommand::Image(_) => "image",
                UiCommand::NineSlice(_) => "nine",
                UiCommand::PushClip(_) => "push",
                UiCommand::PopClip => "pop",
            })
            .collect();
        assert_eq!(kinds, ["quad", "push", "image", "nine", "text", "pop"]);
        assert!(!f.is_empty());
    }

    #[test]
    fn clear_keeps_the_capacity() {
        let mut f = UiFrame::new();
        for i in 0..50 {
            f.quad(r(i as f32), Color([1.0; 4]));
        }
        let cap = f.commands.capacity();
        f.clear();
        assert!(f.is_empty());
        assert_eq!(f.commands.capacity(), cap);
    }

    #[test]
    fn a_draw_list_becomes_its_quads_then_its_text() {
        let mut list = DrawList::default();
        list.quads.push(Quad {
            rect: r(0.0),
            color: Color([0.5; 4]),
        });
        list.quads.push(Quad {
            rect: r(5.0),
            color: Color([0.25; 4]),
        });
        list.texts.push(TextRun {
            pos: Vec2 { x: 1.0, y: 2.0 },
            text: "a".into(),
            size: 10.0,
            color: Color([1.0; 4]),
            align: TextAlign::Center,
        });
        let frame = UiFrame::from(&list);
        assert_eq!(frame.commands().len(), 3);
        assert!(matches!(frame.commands()[0], UiCommand::Quad(q) if q.color == Color([0.5; 4])));
        assert!(matches!(frame.commands()[1], UiCommand::Quad(q) if q.color == Color([0.25; 4])));
        assert!(matches!(&frame.commands()[2], UiCommand::Text(t) if t.text == "a"));
        // Both views of the list hand out the same commands in the same order.
        let mut a = Vec::new();
        list.visit(&mut |c| a.push(format!("{c:?}")));
        let mut b = Vec::new();
        frame.visit(&mut |c| b.push(format!("{c:?}")));
        assert_eq!(a, b);
    }

    #[test]
    fn images_lists_every_use_and_uses_image_finds_them() {
        let mut f = UiFrame::new();
        f.image(h(1), r(0.0))
            .quad(r(1.0), Color([1.0; 4]))
            .nine_slice(h(2), r(2.0), Insets::ZERO, 1.0)
            .image(h(1), r(3.0));
        assert_eq!(f.images().collect::<Vec<_>>(), [h(1), h(2), h(1)]);
        assert!(f.uses_image(h(2)));
        assert!(!f.uses_image(h(3)));
        assert!(!UiFrame::new().uses_image(h(1)));
    }

    #[test]
    fn a_picture_that_uses_itself_or_loops_through_another_is_detected() {
        let none = |_: ImageHandle| Vec::new();
        // Direct.
        assert!(would_loop(h(1), &[h(1)], &none));
        assert!(would_loop(h(1), &[h(2), h(1)], &none));
        // Not.
        assert!(!would_loop(h(1), &[h(2), h(3)], &none));
        assert!(!would_loop(h(1), &[], &none));
        // Indirect: image 2 was rendered from 3, and 3 from 1; drawing 2 into 1 is a loop.
        let from = |i: ImageHandle| match i.id() {
            2 => vec![h(3)],
            3 => vec![h(1)],
            _ => Vec::new(),
        };
        assert!(would_loop(h(1), &[h(2)], &from));
        assert!(!would_loop(h(4), &[h(2)], &from));
        // A loop that already exists elsewhere does not hang the search.
        let cyc = |i: ImageHandle| match i.id() {
            2 => vec![h(3)],
            3 => vec![h(2)],
            _ => Vec::new(),
        };
        assert!(!would_loop(h(1), &[h(2)], &cyc));
    }

    #[test]
    fn replay_renders_what_an_image_was_made_from_first() {
        // 5 was made from 3 and 4; 3 from 1; 4 from nothing; 1 from nothing; 2 from 5.
        let images = vec![
            (h(1), vec![]),
            (h(2), vec![h(5)]),
            (h(3), vec![h(1)]),
            (h(4), vec![]),
            (h(5), vec![h(3), h(4)]),
        ];
        let order = replay_order(&images);
        assert_eq!(order.len(), 5);
        let pos = |i: u32| order.iter().position(|&x| x == h(i)).unwrap();
        assert!(pos(1) < pos(3));
        assert!(pos(3) < pos(5));
        assert!(pos(4) < pos(5));
        assert!(pos(5) < pos(2));
        // Independent images keep their listed order.
        let flat = vec![(h(7), vec![]), (h(3), vec![]), (h(5), vec![])];
        assert_eq!(replay_order(&flat), [h(7), h(3), h(5)]);
    }

    #[test]
    fn replay_survives_a_loop_and_an_image_that_is_gone() {
        let looped = vec![(h(1), vec![h(2)]), (h(2), vec![h(1)])];
        assert_eq!(replay_order(&looped).len(), 2);
        // 1 was made from 9, which is no longer there: 1 is still replayed, 9 is not invented.
        let gone = vec![(h(1), vec![h(9)])];
        assert_eq!(replay_order(&gone), [h(1)]);
    }
}
