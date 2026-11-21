//! Opaque ids that name renderer-side resources without exposing any GPU type.

use std::num::NonZeroU32;

macro_rules! resource_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(NonZeroU32);

        impl $name {
            /// Wrap a raw id. `None` for `0`, which is never a valid id (so
            /// `Option<Self>` costs nothing extra).
            pub fn from_raw(raw: u32) -> Option<Self> {
                NonZeroU32::new(raw).map(Self)
            }

            /// The raw id.
            pub fn raw(self) -> u32 {
                self.0.get()
            }
        }
    };
}

resource_id! {
    /// A model (mesh + skeleton + default materials) known to the renderer.
    /// Obtained from [`RenderResources::model`](crate::RenderResources::model).
    ModelId
}
resource_id! {
    /// A material the renderer can draw with.
    MaterialId
}
resource_id! {
    /// A texture (for 3D materials and, later, UI images).
    TextureId
}
resource_id! {
    /// An animation clip that can drive a skinned [`ModelId`].
    ClipId
}

/// Stable identity of one drawable thing across frames (normally the owning entity's id).
///
/// The renderer matches `RenderId`s between consecutive [`RenderWorld`](crate::RenderWorld)s to
/// interpolate motion, so an entity must keep the same value for its whole life, and no two
/// renderables in one world may share one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RenderId(pub u64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_not_a_valid_id() {
        assert!(ModelId::from_raw(0).is_none());
        assert_eq!(ModelId::from_raw(7).unwrap().raw(), 7);
    }

    #[test]
    fn option_of_id_is_free() {
        assert_eq!(size_of::<Option<MaterialId>>(), size_of::<u32>());
    }

    #[test]
    fn distinct_id_types_do_not_mix() {
        // Compile-time property: `ModelId` and `TextureId` are different types. This only checks
        // that equal raw values stay equal within one type.
        assert_eq!(TextureId::from_raw(3), TextureId::from_raw(3));
        assert_ne!(ClipId::from_raw(3), ClipId::from_raw(4));
    }
}
