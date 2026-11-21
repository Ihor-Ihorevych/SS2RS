//! [`MemoryVfs`]: an in-memory [`Vfs`] for tests and mocks.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;

use ss2_core::{Error, Result};

use crate::{DirEntry, EntryKind, ReadSeek, Stat, Vfs, VfsPath};

/// A [`Vfs`] backed by a map of path → bytes. Directories exist implicitly for every prefix of
/// an inserted file. Inserting an existing path replaces it (the "later layer wins" rule of the
/// real mount stack, T02-002).
///
/// ```
/// use ss2_vfs::{MemoryVfs, Vfs, VfsPath};
///
/// let vfs = MemoryVfs::from_files([("a/b.txt", &b"hi"[..])]).unwrap();
/// assert!(vfs.exists(&VfsPath::new("A").unwrap()));      // implicit directory
/// assert!(!vfs.exists(&VfsPath::new("a/c.txt").unwrap()));
/// ```
#[derive(Clone, Debug, Default)]
pub struct MemoryVfs {
    files: BTreeMap<VfsPath, Vec<u8>>,
}

impl MemoryVfs {
    /// An empty file system.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build from `(path, bytes)` pairs.
    ///
    /// # Errors
    /// [`Error::Invalid`] if a path is invalid (see [`VfsPath::new`]) or names the root.
    pub fn from_files<'a>(files: impl IntoIterator<Item = (&'a str, &'a [u8])>) -> Result<Self> {
        let mut vfs = Self::new();
        for (path, data) in files {
            vfs.insert(path, data.to_vec())?;
        }
        Ok(vfs)
    }

    /// Add or replace a file.
    ///
    /// # Errors
    /// [`Error::Invalid`] if `path` is invalid, names the root, or would turn an existing
    /// directory into a file (or a file into a directory).
    pub fn insert(&mut self, path: &str, data: Vec<u8>) -> Result<()> {
        let path = VfsPath::new(path)?;
        if path.is_root() {
            return Err(Error::invalid(
                "vfs path",
                "cannot store a file at the root",
            ));
        }
        if self.is_dir(&path) {
            return Err(Error::invalid(
                "vfs path",
                format!("{path} is a directory, cannot replace it with a file"),
            ));
        }
        let mut ancestor = path.parent();
        while let Some(dir) = ancestor.filter(|d| !d.is_root()) {
            if self.files.contains_key(&dir) {
                return Err(Error::invalid(
                    "vfs path",
                    format!("{dir} is a file, cannot use it as a directory"),
                ));
            }
            ancestor = dir.parent();
        }
        self.files.insert(path, data);
        Ok(())
    }

    fn is_dir(&self, path: &VfsPath) -> bool {
        if path.is_root() {
            return true;
        }
        let prefix = format!("{path}/");
        self.files.keys().any(|k| k.as_str().starts_with(&prefix))
    }
}

impl Vfs for MemoryVfs {
    fn stat(&self, path: &VfsPath) -> Result<Stat> {
        if let Some(data) = self.files.get(path) {
            Ok(Stat {
                kind: EntryKind::File,
                size: data.len() as u64,
            })
        } else if self.is_dir(path) {
            Ok(Stat {
                kind: EntryKind::Dir,
                size: 0,
            })
        } else {
            Err(Error::NotFound(path.to_string()))
        }
    }

    fn open(&self, path: &VfsPath) -> Result<Box<dyn ReadSeek>> {
        match self.files.get(path) {
            Some(data) => Ok(Box::new(Cursor::new(data.clone()))),
            None => Err(Error::NotFound(path.to_string())),
        }
    }

    fn list(&self, dir: &VfsPath) -> Result<Vec<DirEntry>> {
        if self.files.contains_key(dir) || !self.is_dir(dir) {
            return Err(Error::NotFound(dir.to_string()));
        }
        let prefix = if dir.is_root() {
            String::new()
        } else {
            format!("{dir}/")
        };
        let mut entries = BTreeSet::new();
        for key in self.files.keys() {
            let Some(rest) = key.as_str().strip_prefix(&prefix) else {
                continue;
            };
            entries.insert(match rest.split_once('/') {
                Some((name, _)) => DirEntry {
                    name: name.to_owned(),
                    kind: EntryKind::Dir,
                },
                None => DirEntry {
                    name: rest.to_owned(),
                    kind: EntryKind::File,
                },
            });
        }
        Ok(entries.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Seek, SeekFrom};

    use super::*;

    fn p(s: &str) -> VfsPath {
        VfsPath::new(s).unwrap()
    }

    fn sample() -> MemoryVfs {
        MemoryVfs::from_files([
            ("Models/Player.mdl", &b"mdl"[..]),
            ("models/enemies/gnaar.mdl", b"gnaar"),
            ("models/enemies/beast.mdl", b"beast"),
            ("readme.txt", b"hello"),
        ])
        .unwrap()
    }

    #[test]
    fn stat_files_dirs_and_missing() {
        let vfs = sample();
        assert_eq!(
            vfs.stat(&p("MODELS/player.MDL")).unwrap(),
            Stat {
                kind: EntryKind::File,
                size: 3
            }
        );
        assert_eq!(
            vfs.stat(&p("models")).unwrap(),
            Stat {
                kind: EntryKind::Dir,
                size: 0
            }
        );
        assert_eq!(vfs.stat(&VfsPath::root()).unwrap().kind, EntryKind::Dir);
        assert!(matches!(vfs.stat(&p("nope")), Err(Error::NotFound(_))));
        assert!(vfs.exists(&p("models/enemies")));
        assert!(
            !vfs.exists(&p("models/enem")),
            "prefix of a name is not a directory"
        );
    }

    #[test]
    fn read_and_open_with_seek() {
        let vfs = sample();
        assert_eq!(vfs.read(&p("readme.txt")).unwrap(), b"hello");
        let mut f = vfs.open(&p("readme.txt")).unwrap();
        f.seek(SeekFrom::Start(3)).unwrap();
        let mut rest = String::new();
        f.read_to_string(&mut rest).unwrap();
        assert_eq!(rest, "lo");
        assert!(matches!(vfs.read(&p("models")), Err(Error::NotFound(_))));
        assert!(matches!(vfs.open(&p("missing")), Err(Error::NotFound(_))));
    }

    #[test]
    fn list_is_sorted_deduplicated_and_typed() {
        let vfs = sample();
        let names = |dir: &str| {
            vfs.list(&p(dir))
                .unwrap()
                .into_iter()
                .map(|e| (e.name, e.kind))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            names(""),
            [
                ("models".to_owned(), EntryKind::Dir),
                ("readme.txt".to_owned(), EntryKind::File)
            ]
        );
        assert_eq!(
            names("models"),
            [
                ("enemies".to_owned(), EntryKind::Dir),
                ("player.mdl".to_owned(), EntryKind::File)
            ]
        );
        assert_eq!(
            names("models/enemies"),
            [
                ("beast.mdl".to_owned(), EntryKind::File),
                ("gnaar.mdl".to_owned(), EntryKind::File)
            ]
        );
        assert!(matches!(
            vfs.list(&p("readme.txt")),
            Err(Error::NotFound(_))
        ));
        assert!(matches!(vfs.list(&p("nope")), Err(Error::NotFound(_))));
    }

    #[test]
    fn insert_replaces_and_rejects_file_dir_conflicts() {
        let mut vfs = sample();
        vfs.insert("README.TXT", b"v2".to_vec()).unwrap();
        assert_eq!(vfs.read(&p("readme.txt")).unwrap(), b"v2");
        assert!(vfs.insert("models", b"x".to_vec()).is_err(), "dir -> file");
        assert!(
            vfs.insert("readme.txt/inner", b"x".to_vec()).is_err(),
            "file -> dir"
        );
        assert!(vfs.insert("", b"x".to_vec()).is_err(), "root");
        assert!(vfs.insert("../x", b"x".to_vec()).is_err(), "escape");
    }

    #[test]
    fn dyn_vfs_is_object_safe_and_thread_safe() {
        fn assert_send_sync<T: Send + Sync + ?Sized>() {}
        assert_send_sync::<dyn Vfs>();
        let boxed: Box<dyn Vfs> = Box::new(sample());
        assert_eq!(boxed.read(&p("readme.txt")).unwrap(), b"hello");
    }
}
