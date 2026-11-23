//! The registry of images: what a [`Renderer`](crate::Renderer) keeps so that it can draw them and make them
//! again on a new GPU device.
//!
//! Two kinds of image live here:
//!
//! * an **uploaded image**: its pixels are kept on the CPU (shared, not copied) next to the GPU texture;
//! * a **render target**: a texture the renderer draws a [`UiFrame`] into. A GPU texture cannot be read back
//!   after the device is lost, so what is kept is the *recipe*: the frame and clear colour of its last render.
//!   After a rebuilt device the recipes are drawn again, each after the images it was drawn from.
//!
//! The entries are plain CPU data; the GPU objects are made and dropped by the renderer.

use std::collections::BTreeMap;
use std::sync::Arc;

use ss2_ui::Color;

use crate::ImageRgba8;
use crate::ui_frame::{ImageHandle, UiFrame};

/// What a render target was last drawn from.
#[derive(Clone, Debug)]
pub(crate) struct Recipe {
    pub frame: UiFrame,
    pub clear: Color,
}

/// Where an image's content comes from.
#[derive(Clone, Debug)]
pub(crate) enum Source {
    Pixels(Arc<ImageRgba8>),
    /// `None` until the first render: transparent black.
    Target(Option<Recipe>),
}

/// The GPU objects of one image.
pub(crate) struct GpuImage {
    pub texture: wgpu::Texture,
    pub bind_group: wgpu::BindGroup,
}

pub(crate) struct Entry {
    pub size: (u32, u32),
    pub source: Source,
    pub gpu: Option<GpuImage>,
}

impl Entry {
    /// A render target holds premultiplied alpha and is drawn with the premultiplied pipeline.
    pub(crate) fn is_target(&self) -> bool {
        matches!(self.source, Source::Target(_))
    }
}

/// All the images of a renderer.
#[derive(Default)]
pub(crate) struct ImageStore {
    next_id: u32,
    entries: BTreeMap<ImageHandle, Entry>,
}

impl ImageStore {
    /// Add an image without GPU objects (yet). `None` when the handles have run out (4 billion images).
    pub(crate) fn insert(&mut self, size: (u32, u32), source: Source) -> Option<ImageHandle> {
        let id = self.next_id.checked_add(1)?;
        self.next_id = id;
        let handle = ImageHandle::new(id);
        self.entries.insert(
            handle,
            Entry {
                size,
                source,
                gpu: None,
            },
        );
        Some(handle)
    }

    pub(crate) fn remove(&mut self, handle: ImageHandle) -> bool {
        self.entries.remove(&handle).is_some()
    }

    pub(crate) fn get(&self, handle: ImageHandle) -> Option<&Entry> {
        self.entries.get(&handle)
    }

    pub(crate) fn get_mut(&mut self, handle: ImageHandle) -> Option<&mut Entry> {
        self.entries.get_mut(&handle)
    }

    pub(crate) fn size_of(&self, handle: ImageHandle) -> Option<(u32, u32)> {
        self.entries.get(&handle).map(|e| e.size)
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn handles(&self) -> impl Iterator<Item = ImageHandle> + '_ {
        self.entries.keys().copied()
    }

    /// The images a render target was last drawn from (without repeats; none for anything else).
    pub(crate) fn rendered_from(&self, handle: ImageHandle) -> Vec<ImageHandle> {
        let mut from = Vec::new();
        if let Some(Entry {
            source: Source::Target(Some(recipe)),
            ..
        }) = self.entries.get(&handle)
        {
            for image in recipe.frame.images() {
                if !from.contains(&image) {
                    from.push(image);
                }
            }
        }
        from
    }

    /// The same images without their GPU objects, for a renderer on a new device.
    pub(crate) fn clone_cpu(&self) -> ImageStore {
        ImageStore {
            next_id: self.next_id,
            entries: self
                .entries
                .iter()
                .map(|(h, e)| {
                    (
                        *h,
                        Entry {
                            size: e.size,
                            source: e.source.clone(),
                            gpu: None,
                        },
                    )
                })
                .collect(),
        }
    }

    /// Every image with the images it was rendered from, in handle order.
    pub(crate) fn dependencies(&self) -> Vec<(ImageHandle, Vec<ImageHandle>)> {
        self.handles().map(|h| (h, self.rendered_from(h))).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss2_ui::Rect;

    fn pixels(w: u32, h: u32) -> Source {
        Source::Pixels(Arc::new(ImageRgba8 {
            width: w,
            height: h,
            pixels: vec![0; (w * h * 4) as usize],
        }))
    }

    fn recipe_using(images: &[ImageHandle]) -> Source {
        let mut frame = UiFrame::new();
        for &i in images {
            frame.image(
                i,
                Rect {
                    x: 0.0,
                    y: 0.0,
                    w: 1.0,
                    h: 1.0,
                },
            );
        }
        Source::Target(Some(Recipe {
            frame,
            clear: Color([0.0; 4]),
        }))
    }

    #[test]
    fn handles_are_unique_and_never_reused_after_a_removal() {
        let mut s = ImageStore::default();
        let a = s.insert((2, 2), pixels(2, 2)).unwrap();
        let b = s.insert((2, 2), pixels(2, 2)).unwrap();
        assert_ne!(a, b);
        assert!(s.remove(a));
        assert!(!s.remove(a));
        let c = s.insert((2, 2), pixels(2, 2)).unwrap();
        assert_ne!(c, a);
        assert_ne!(c, b);
        assert_eq!(s.len(), 2);
        assert_eq!(s.size_of(a), None);
        assert_eq!(s.size_of(b), Some((2, 2)));
    }

    #[test]
    fn running_out_of_handles_is_reported_not_wrapped() {
        let mut s = ImageStore {
            next_id: u32::MAX,
            ..ImageStore::default()
        };
        assert!(s.insert((1, 1), pixels(1, 1)).is_none());
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn a_target_knows_what_it_was_rendered_from() {
        let mut s = ImageStore::default();
        let a = s.insert((4, 4), pixels(4, 4)).unwrap();
        let b = s.insert((4, 4), pixels(4, 4)).unwrap();
        let t = s.insert((8, 8), recipe_using(&[a, b, a])).unwrap();
        let blank = s.insert((8, 8), Source::Target(None)).unwrap();
        assert_eq!(s.rendered_from(t), [a, b]);
        assert!(s.rendered_from(a).is_empty());
        assert!(s.rendered_from(blank).is_empty());
        assert!(s.get(t).unwrap().is_target());
        assert!(!s.get(a).unwrap().is_target());
        let deps = s.dependencies();
        assert_eq!(deps.len(), 4);
        assert_eq!(deps[2], (t, vec![a, b]));
    }

    #[test]
    fn cloning_for_a_new_device_keeps_the_content_and_the_numbering() {
        let mut s = ImageStore::default();
        let a = s.insert((4, 4), pixels(4, 4)).unwrap();
        let t = s.insert((8, 8), recipe_using(&[a])).unwrap();
        let mut copy = s.clone_cpu();
        assert_eq!(copy.size_of(a), Some((4, 4)));
        assert_eq!(copy.rendered_from(t), [a]);
        assert!(copy.get(a).unwrap().gpu.is_none());
        // Pixels are shared, not copied.
        let (Source::Pixels(x), Source::Pixels(y)) =
            (&s.get(a).unwrap().source, &copy.get(a).unwrap().source)
        else {
            panic!()
        };
        assert!(Arc::ptr_eq(x, y));
        // New images in the copy do not collide with the old ones.
        let n = copy.insert((1, 1), pixels(1, 1)).unwrap();
        assert!(n != a && n != t);
    }
}
