//! ss2-format-fx — the effect formats (roadmap T02-024; specs `docs/re/pfx.md`,
//! `docs/re/pfx-schema.md` and `docs/re/fx.md`).
//!
//! Four small CTSEMETA resource types describe effects:
//!
//! | Extension | Module | Asset type | What it is |
//! |---|---|---|---|
//! | `.pfx` | [`pfx`] | [`pfx::Effect`] | a **particle effect**: emitters (machine, clouds, atomic) with textures, blending and life graphs |
//! | `.lfx` | [`lfx`] | [`lfx::FlareConfiguration`] | a **lens flare**: textured parts laid out along the light-to-centre axis |
//! | `.tfx` | [`tfx`] | [`tfx::TextEffectConfiguration`] | an animated **on-screen text** style |
//! | `.vfx` | [`vfx`] | [`vfx::VibrationEffect`] | a **controller vibration** pattern: one graph per motor |
//!
//! Nearly every numeric parameter of a particle effect is a [`graph::GraphHolder`]: a constant that
//! a [`graph::Graph`] (control points over the particle's or the effect's life) may replace. The
//! files are pure data: this crate returns them as typed structs and does not simulate, evaluate
//! graphs or interpret the enumerations the specs leave open (blend type, render order, orientation,
//! flare flags): those stay raw numbers with their meaning in the field documentation. Members
//! whose role is unknown keep their number (`member_5`).
//!
//! Textures and fonts are returned as paths (as listed in the file's `RFIL`) for the asset manager
//! to load; the loaders request them as dependencies.
//!
//! ```ignore
//! let fx = ss2_format_fx::pfx::Effect::from_bytes(&bytes)?;
//! for p in &fx.particle_effects {
//!     // p.texture.texture is a `.tex` path; p.kind is Machine / Clouds / Atomic
//! }
//! let flare = ss2_format_fx::lfx::FlareConfiguration::from_bytes(&bytes)?;
//! ```

pub mod graph;
pub mod lfx;
pub mod pfx;
pub mod tfx;
pub mod vfx;

mod read;

use ss2_assets::{AssetLoader, LoadContext};
use ss2_core::Result;

pub use graph::{ControlPoint, Graph, GraphHolder, GraphKind};

macro_rules! loader {
    ($name:ident, $asset:ty, $ext:literal, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Debug, Default)]
        pub struct $name;

        impl AssetLoader for $name {
            type Asset = $asset;

            fn extensions(&self) -> &[&str] {
                &[$ext]
            }

            fn load(&self, ctx: &mut LoadContext<'_>) -> Result<$asset> {
                let asset = <$asset>::from_bytes(&ctx.read()?)?;
                for path in asset.referenced_files() {
                    // The references are dependencies of the effect. A reference whose format has
                    // no loader registered yet is simply not recorded.
                    let _ = ctx.load_any(path);
                }
                Ok(asset)
            }
        }
    };
}

loader!(
    PfxLoader,
    pfx::Effect,
    "pfx",
    "The `.pfx` loader (asset type [`pfx::Effect`]); the textures are recorded as dependencies."
);
loader!(
    LfxLoader,
    lfx::FlareConfiguration,
    "lfx",
    "The `.lfx` loader (asset type [`lfx::FlareConfiguration`]); the textures are recorded as dependencies."
);
loader!(
    TfxLoader,
    tfx::TextEffectConfiguration,
    "tfx",
    "The `.tfx` loader (asset type [`tfx::TextEffectConfiguration`]); the font is recorded as a dependency."
);
loader!(
    VfxLoader,
    vfx::VibrationEffect,
    "vfx",
    "The `.vfx` loader (asset type [`vfx::VibrationEffect`])."
);

/// Shared by the data-gated tests of every module: walk the install and read every file of one
/// extension.
#[cfg(test)]
pub(crate) mod testutil {
    use ss2_vfs::{EntryKind, Vfs, VfsPath};

    /// `(path, bytes)` of every file with extension `ext` in the install at `dir`.
    pub fn install_files(dir: &std::path::Path, ext: &str) -> Vec<(String, Vec<u8>)> {
        let vfs = ss2_vfs::mount_install(dir).unwrap();
        let mut stack = vec![VfsPath::root()];
        let mut out = Vec::new();
        while let Some(d) = stack.pop() {
            for e in vfs.list(&d).unwrap() {
                let path = d.join(&e.name).unwrap();
                if e.kind == EntryKind::Dir {
                    stack.push(path);
                } else if path.extension() == Some(ext) {
                    out.push((path.as_str().to_string(), vfs.read(&path).unwrap()));
                }
            }
        }
        out.sort();
        out
    }
}

#[cfg(test)]
mod loader_tests {
    use std::sync::Arc;

    use ss2_assets::{AssetManager, LoadState};
    use ss2_vfs::MemoryVfs;

    use super::*;

    #[test]
    fn the_loaders_plug_into_the_asset_manager() {
        let file = vfx::tests::vibro_file(true, true);
        let vfs = MemoryVfs::from_files([("vibro/test.vfx", file.as_slice())]).unwrap();
        let assets = AssetManager::new(Arc::new(vfs));
        assets.register_loader(VfxLoader).unwrap();
        assets.register_loader(PfxLoader).unwrap();
        assets.register_loader(LfxLoader).unwrap();
        assets.register_loader(TfxLoader).unwrap();
        let h = assets
            .load::<vfx::VibrationEffect>("Vibro/Test.VFX")
            .unwrap();
        assert_eq!(h.wait(), LoadState::Ready);
        assert_eq!(h.get().unwrap().duration, 5.0);
    }
}
