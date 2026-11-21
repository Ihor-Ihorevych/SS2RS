//! [`LayeredVfs`]: a stack of [`Vfs`] layers where later mounts override earlier ones.
//!
//! This is how the game's `.gro` archives combine: base archives first, then each patch on top
//! (roadmap T02-002). Any [`Vfs`] can be a layer: an archive, a loose directory, a mod, memory.

use std::collections::BTreeMap;

use ss2_core::{Error, Result};

use crate::{DirEntry, EntryKind, ReadSeek, Stat, Vfs, VfsPath};

struct Layer {
    name: String,
    vfs: Box<dyn Vfs>,
}

/// Several [`Vfs`] layers presented as one. **The most recently mounted layer wins**.
///
/// - A file present in several layers comes from the topmost one.
/// - A directory is the union of that directory in every layer where it is a directory, so
///   listings merge; a name present in several layers appears once.
/// - If the topmost layer that knows a path has a **file** there, that file shadows a directory
///   of the same name in lower layers (and the directory's contents become unreachable).
/// - The root always exists, even with no layers.
#[derive(Default)]
pub struct LayeredVfs {
    layers: Vec<Layer>,
}

impl LayeredVfs {
    /// An empty stack (an empty root directory).
    pub fn new() -> Self {
        Self::default()
    }

    /// Mount `layer` on top of everything mounted so far. `name` identifies it in
    /// [`source_of`](Self::source_of) and diagnostics (for archives: the file name).
    pub fn mount(&mut self, name: impl Into<String>, layer: impl Vfs + 'static) {
        self.layers.push(Layer {
            name: name.into(),
            vfs: Box::new(layer),
        });
    }

    /// Layer names from the bottom (lowest priority) to the top (wins).
    pub fn layer_names(&self) -> Vec<&str> {
        self.layers.iter().map(|l| l.name.as_str()).collect()
    }

    /// Name of the topmost layer that provides `path` (a file or a directory), if any.
    pub fn source_of(&self, path: &VfsPath) -> Option<&str> {
        self.find(path).map(|(layer, _)| layer.name.as_str())
    }

    /// The topmost layer that has anything at `path`, with what it has there.
    fn find(&self, path: &VfsPath) -> Option<(&Layer, Stat)> {
        self.layers
            .iter()
            .rev()
            .find_map(|layer| layer.vfs.stat(path).ok().map(|stat| (layer, stat)))
    }
}

impl Vfs for LayeredVfs {
    fn stat(&self, path: &VfsPath) -> Result<Stat> {
        if path.is_root() {
            return Ok(Stat {
                kind: EntryKind::Dir,
                size: 0,
            });
        }
        self.find(path)
            .map(|(_, stat)| stat)
            .ok_or_else(|| Error::NotFound(path.to_string()))
    }

    fn open(&self, path: &VfsPath) -> Result<Box<dyn ReadSeek>> {
        match self.find(path) {
            Some((
                layer,
                Stat {
                    kind: EntryKind::File,
                    ..
                },
            )) => layer.vfs.open(path),
            _ => Err(Error::NotFound(path.to_string())),
        }
    }

    fn read(&self, path: &VfsPath) -> Result<Vec<u8>> {
        match self.find(path) {
            Some((
                layer,
                Stat {
                    kind: EntryKind::File,
                    ..
                },
            )) => layer.vfs.read(path),
            _ => Err(Error::NotFound(path.to_string())),
        }
    }

    fn list(&self, dir: &VfsPath) -> Result<Vec<DirEntry>> {
        if !dir.is_root()
            && !matches!(
                self.find(dir),
                Some((
                    _,
                    Stat {
                        kind: EntryKind::Dir,
                        ..
                    }
                ))
            )
        {
            return Err(Error::NotFound(dir.to_string()));
        }
        // Topmost layer first, so the first sighting of a name decides its kind.
        let mut merged: BTreeMap<String, EntryKind> = BTreeMap::new();
        for layer in self.layers.iter().rev() {
            match layer.vfs.list(dir) {
                Ok(entries) => {
                    for e in entries {
                        merged.entry(e.name).or_insert(e.kind);
                    }
                }
                Err(Error::NotFound(_)) => {}
                Err(e) => return Err(e),
            }
        }
        Ok(merged
            .into_iter()
            .map(|(name, kind)| DirEntry { name, kind })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EntryKind, MemoryVfs};
    use ss2_core::Error;

    fn p(s: &str) -> VfsPath {
        VfsPath::new(s).unwrap()
    }

    fn mem(files: &[(&str, &str)]) -> MemoryVfs {
        MemoryVfs::from_files(files.iter().map(|(n, d)| (*n, d.as_bytes()))).unwrap()
    }

    fn names(v: &LayeredVfs, dir: &str) -> Vec<(String, EntryKind)> {
        v.list(&p(dir))
            .unwrap()
            .into_iter()
            .map(|e| (e.name, e.kind))
            .collect()
    }

    #[test]
    fn later_layer_overrides_file_and_source_is_reported() {
        let mut v = LayeredVfs::new();
        v.mount("base", mem(&[("a.txt", "base")]));
        v.mount("patch", mem(&[("A.TXT", "patch")]));
        assert_eq!(v.read(&p("a.txt")).unwrap(), b"patch");
        assert_eq!(v.stat(&p("a.txt")).unwrap().size, 5);
        assert_eq!(v.source_of(&p("a.txt")), Some("patch"));
    }

    #[test]
    fn lower_layer_is_used_when_higher_has_no_such_file() {
        let mut v = LayeredVfs::new();
        v.mount("base", mem(&[("a.txt", "base"), ("b.txt", "b")]));
        v.mount("patch", mem(&[("a.txt", "patch")]));
        assert_eq!(v.read(&p("b.txt")).unwrap(), b"b");
        assert_eq!(v.source_of(&p("b.txt")), Some("base"));
    }

    #[test]
    fn listing_merges_layers_sorted_and_deduplicated() {
        let mut v = LayeredVfs::new();
        v.mount(
            "base",
            mem(&[("d/a.txt", "1"), ("d/c.txt", "1"), ("top.txt", "1")]),
        );
        v.mount("patch", mem(&[("d/b.txt", "2"), ("d/c.txt", "2")]));
        assert_eq!(
            names(&v, "d"),
            [
                ("a.txt".to_string(), EntryKind::File),
                ("b.txt".to_string(), EntryKind::File),
                ("c.txt".to_string(), EntryKind::File)
            ]
        );
        assert_eq!(
            names(&v, ""),
            [
                ("d".to_string(), EntryKind::Dir),
                ("top.txt".to_string(), EntryKind::File)
            ]
        );
    }

    #[test]
    fn missing_paths_are_not_found_everywhere() {
        let mut v = LayeredVfs::new();
        v.mount("base", mem(&[("a.txt", "x")]));
        assert!(matches!(v.stat(&p("nope")), Err(Error::NotFound(_))));
        assert!(matches!(v.open(&p("nope")), Err(Error::NotFound(_))));
        assert!(matches!(v.read(&p("nope")), Err(Error::NotFound(_))));
        assert!(matches!(v.list(&p("nope")), Err(Error::NotFound(_))));
        assert!(!v.exists(&p("nope")));
        assert_eq!(v.source_of(&p("nope")), None);
    }

    #[test]
    fn opening_a_directory_and_listing_a_file_are_not_found() {
        let mut v = LayeredVfs::new();
        v.mount("base", mem(&[("d/a.txt", "x")]));
        assert!(matches!(v.open(&p("d")), Err(Error::NotFound(_))));
        assert!(matches!(v.list(&p("d/a.txt")), Err(Error::NotFound(_))));
    }

    #[test]
    fn a_file_in_a_higher_layer_shadows_a_directory_below() {
        let mut v = LayeredVfs::new();
        v.mount("base", mem(&[("x/y.txt", "deep")]));
        v.mount("patch", mem(&[("x", "file now")]));
        assert_eq!(v.stat(&p("x")).unwrap().kind, EntryKind::File);
        assert!(matches!(v.list(&p("x")), Err(Error::NotFound(_))));
        assert_eq!(names(&v, ""), [("x".to_string(), EntryKind::File)]);
    }

    #[test]
    fn empty_stack_has_an_empty_root() {
        let v = LayeredVfs::new();
        assert_eq!(v.stat(&VfsPath::root()).unwrap().kind, EntryKind::Dir);
        assert!(v.list(&VfsPath::root()).unwrap().is_empty());
    }

    #[test]
    fn layer_names_are_listed_bottom_to_top() {
        let mut v = LayeredVfs::new();
        v.mount("a", MemoryVfs::new());
        v.mount("b", MemoryVfs::new());
        assert_eq!(v.layer_names(), ["a", "b"]);
    }

    #[test]
    fn streaming_open_comes_from_the_winning_layer() {
        use std::io::Read;
        let mut v = LayeredVfs::new();
        v.mount("base", mem(&[("a.txt", "base")]));
        v.mount("patch", mem(&[("a.txt", "patch")]));
        let mut s = String::new();
        v.open(&p("a.txt")).unwrap().read_to_string(&mut s).unwrap();
        assert_eq!(s, "patch");
    }
}
