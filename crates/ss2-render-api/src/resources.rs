//! Turning asset paths into the cheap ids a [`RenderWorld`](crate::RenderWorld) carries.

use std::collections::BTreeMap;

use crate::ids::{ClipId, MaterialId, ModelId, TextureId};

/// How the simulation side asks the renderer for something to draw.
///
/// Resolve names **once** (when an entity spawns), keep the returned id, and put the id into every
/// frame's extract. Asking is cheap and idempotent: the same path always yields the same id.
///
/// The returned id is valid immediately even if the asset is still loading or failed to load: the
/// renderer substitutes a placeholder (or skips the object) and never makes the caller wait or
/// handle an error. Load failures are reported by the asset manager (`ss2-assets`), not here.
///
/// `path` is a game-data path such as `"models/player.mdl"`; implementations normalize it exactly
/// like `ss2_vfs::VfsPath` (case-insensitive, `/` or `\`), so callers need not.
pub trait RenderResources {
    /// The model at `path`.
    fn model(&mut self, path: &str) -> ModelId;
    /// The material at `path`.
    fn material(&mut self, path: &str) -> MaterialId;
    /// The texture at `path`.
    fn texture(&mut self, path: &str) -> TextureId;
    /// The animation clip at `path`.
    fn clip(&mut self, path: &str) -> ClipId;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Model,
    Material,
    Texture,
    Clip,
}

/// A [`RenderResources`] for tests and headless runs: hands out sequential ids (starting at 1,
/// separately per kind) and remembers which path each id came from.
///
/// ```
/// use ss2_render_api::{MockResources, RenderResources};
///
/// let mut res = MockResources::new();
/// let a = res.model("Models/Gnaar.mdl");
/// assert_eq!(res.model(r"models\gnaar.MDL"), a);   // same file, same id
/// assert_eq!(res.path_of_model(a), Some("models/gnaar.mdl"));
/// ```
#[derive(Clone, Debug, Default)]
pub struct MockResources {
    ids: BTreeMap<(Kind, String), u32>,
    next: BTreeMap<Kind, u32>,
}

impl MockResources {
    /// No resources registered yet.
    pub fn new() -> Self {
        Self::default()
    }

    fn intern(&mut self, kind: Kind, path: &str) -> u32 {
        let key = (kind, normalize(path));
        if let Some(&id) = self.ids.get(&key) {
            return id;
        }
        let counter = self.next.entry(kind).or_insert(0);
        *counter += 1;
        let id = *counter;
        self.ids.insert(key, id);
        id
    }

    fn path_of(&self, kind: Kind, raw: u32) -> Option<&str> {
        self.ids
            .iter()
            .find(|((k, _), id)| *k == kind && **id == raw)
            .map(|((_, path), _)| path.as_str())
    }

    /// The normalized path a model id was created from.
    pub fn path_of_model(&self, id: ModelId) -> Option<&str> {
        self.path_of(Kind::Model, id.raw())
    }

    /// The normalized path a material id was created from.
    pub fn path_of_material(&self, id: MaterialId) -> Option<&str> {
        self.path_of(Kind::Material, id.raw())
    }

    /// The normalized path a texture id was created from.
    pub fn path_of_texture(&self, id: TextureId) -> Option<&str> {
        self.path_of(Kind::Texture, id.raw())
    }

    /// The normalized path a clip id was created from.
    pub fn path_of_clip(&self, id: ClipId) -> Option<&str> {
        self.path_of(Kind::Clip, id.raw())
    }
}

/// Minimal stand-in for `VfsPath` normalization (`\` → `/`, lower case, no empty segments).
fn normalize(path: &str) -> String {
    path.split(['/', '\\'])
        .filter(|s| !s.is_empty() && *s != ".")
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join("/")
}

impl RenderResources for MockResources {
    fn model(&mut self, path: &str) -> ModelId {
        ModelId::from_raw(self.intern(Kind::Model, path)).expect("ids start at 1")
    }

    fn material(&mut self, path: &str) -> MaterialId {
        MaterialId::from_raw(self.intern(Kind::Material, path)).expect("ids start at 1")
    }

    fn texture(&mut self, path: &str) -> TextureId {
        TextureId::from_raw(self.intern(Kind::Texture, path)).expect("ids start at 1")
    }

    fn clip(&mut self, path: &str) -> ClipId {
        ClipId::from_raw(self.intern(Kind::Clip, path)).expect("ids start at 1")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_path_same_id_regardless_of_spelling() {
        let mut r = MockResources::new();
        let a = r.model("Models/A.mdl");
        assert_eq!(r.model(r"models\a.MDL"), a);
        assert_eq!(r.model("./models//a.mdl"), a);
        assert_ne!(r.model("models/b.mdl"), a);
    }

    #[test]
    fn kinds_have_independent_id_spaces() {
        let mut r = MockResources::new();
        assert_eq!(r.model("x").raw(), 1);
        assert_eq!(r.texture("x").raw(), 1);
        assert_eq!(r.material("x").raw(), 1);
        assert_eq!(r.clip("x").raw(), 1);
        assert_eq!(r.model("y").raw(), 2);
    }

    #[test]
    fn path_lookup_round_trips_and_is_kind_specific() {
        let mut r = MockResources::new();
        let m = r.model("Models/A.mdl");
        let t = r.texture("Tex/Wall.tex");
        assert_eq!(r.path_of_model(m), Some("models/a.mdl"));
        assert_eq!(r.path_of_texture(t), Some("tex/wall.tex"));
        assert_eq!(r.path_of_material(MaterialId::from_raw(1).unwrap()), None);
    }

    #[test]
    fn usable_as_trait_object() {
        let mut r = MockResources::new();
        let dynamic: &mut dyn RenderResources = &mut r;
        assert_eq!(dynamic.clip("a.ans").raw(), 1);
    }
}
