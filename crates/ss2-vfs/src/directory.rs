use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use ss2_core::{Error, Result};

use crate::{DirEntry, EntryKind, ReadSeek, Stat, Vfs, VfsPath};

struct HostEntry {
    path: PathBuf,
    kind: EntryKind,
}

/// A read-only [`Vfs`] rooted at a directory on disk.
///
/// Names use [`VfsPath`] normalization on every host, including case-sensitive filesystems.
/// Filesystem changes are visible on the next operation. Symbolic links, special files and names
/// that cannot be represented by one VFS path segment are omitted. If two host entries normalize
/// to the same name, operations that scan that directory return [`Error::Invalid`].
///
/// Mount a trusted local directory. As with other path-based filesystem operations, callers
/// should coordinate edits to a directory's structure with concurrent reads.
#[derive(Debug)]
pub struct DirectoryVfs {
    root: PathBuf,
}

impl DirectoryVfs {
    /// Mount `root` at the virtual root. Pass the mount to [`crate::LayeredVfs::mount`] to layer
    /// it with archives or other directories.
    ///
    /// # Errors
    /// [`Error::NotFound`] for a missing root, [`Error::Invalid`] if it is a file, or
    /// [`Error::Io`] if it cannot be accessed.
    pub fn new(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref();
        let root = fs::canonicalize(root).map_err(|error| host_error(root, error))?;
        if !fs::metadata(&root)
            .map_err(|error| host_error(&root, error))?
            .is_dir()
        {
            return Err(Error::invalid(
                "VFS directory mount",
                format!("{} is not a directory", root.display()),
            ));
        }
        Ok(Self { root })
    }

    fn resolve(&self, virtual_path: &VfsPath) -> Result<HostEntry> {
        let mut entry = HostEntry {
            path: self.contained_path(&self.root, virtual_path)?,
            kind: EntryKind::Dir,
        };
        for segment in virtual_path
            .as_str()
            .split('/')
            .filter(|part| !part.is_empty())
        {
            if entry.kind != EntryKind::Dir {
                return Err(Error::NotFound(virtual_path.to_string()));
            }
            entry = read_entries(&entry.path)?
                .remove(segment)
                .ok_or_else(|| Error::NotFound(virtual_path.to_string()))?;
            entry.path = self.contained_path(&entry.path, virtual_path)?;
        }
        Ok(entry)
    }

    fn contained_path(&self, host_path: &Path, virtual_path: &VfsPath) -> Result<PathBuf> {
        let actual = fs::canonicalize(host_path).map_err(|error| host_error(host_path, error))?;
        if !actual.starts_with(&self.root) {
            return Err(Error::NotFound(virtual_path.to_string()));
        }
        Ok(actual)
    }
}

impl Vfs for DirectoryVfs {
    fn stat(&self, path: &VfsPath) -> Result<Stat> {
        let entry = self.resolve(path)?;
        let meta =
            fs::symlink_metadata(&entry.path).map_err(|error| host_error(&entry.path, error))?;
        if meta.is_file() {
            Ok(Stat {
                kind: EntryKind::File,
                size: meta.len(),
            })
        } else if meta.is_dir() {
            Ok(Stat {
                kind: EntryKind::Dir,
                size: 0,
            })
        } else {
            Err(Error::NotFound(path.to_string()))
        }
    }

    fn open(&self, path: &VfsPath) -> Result<Box<dyn ReadSeek>> {
        let entry = self.resolve(path)?;
        if entry.kind != EntryKind::File {
            return Err(Error::NotFound(path.to_string()));
        }
        let file = File::open(&entry.path).map_err(|error| host_error(&entry.path, error))?;
        Ok(Box::new(file))
    }

    fn list(&self, dir: &VfsPath) -> Result<Vec<DirEntry>> {
        let entry = self.resolve(dir)?;
        if entry.kind != EntryKind::Dir {
            return Err(Error::NotFound(dir.to_string()));
        }
        Ok(read_entries(&entry.path)?
            .into_iter()
            .map(|(name, entry)| DirEntry {
                name,
                kind: entry.kind,
            })
            .collect())
    }
}

fn read_entries(dir: &Path) -> Result<BTreeMap<String, HostEntry>> {
    let mut entries = BTreeMap::<String, HostEntry>::new();
    for entry in fs::read_dir(dir).map_err(|error| host_error(dir, error))? {
        let entry = entry.map_err(|error| host_error(dir, error))?;
        let Ok(raw) = entry.file_name().into_string() else {
            continue;
        };
        // A Unix filename can contain a literal backslash. It must not create VFS subpaths.
        if raw.contains('\\') {
            continue;
        }
        let Ok(name) = VfsPath::new(&raw) else {
            continue;
        };
        let Some(name) = name.file_name() else {
            continue;
        };
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(host_error(&path, error)),
        };
        let kind = if file_type.is_dir() {
            EntryKind::Dir
        } else if file_type.is_file() {
            EntryKind::File
        } else {
            continue;
        };
        if entries
            .insert(name.to_owned(), HostEntry { path, kind })
            .is_some()
        {
            return Err(Error::invalid(
                "VFS directory mount",
                format!(
                    "{} has multiple entries named {name:?} after normalization",
                    dir.display()
                ),
            ));
        }
    }
    Ok(entries)
}

fn host_error(path: &Path, error: io::Error) -> Error {
    match error.kind() {
        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory => {
            Error::NotFound(path.display().to_string())
        }
        _ => Error::io(path, error),
    }
}
