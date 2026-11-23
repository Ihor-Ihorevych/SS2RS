//! Where the input files come from: a plain folder on disk, or an install read through the VFS
//! (the newest patch wins, as in the game).

use std::path::{Path, PathBuf};

use ss2_vfs::{EntryKind, Vfs, VfsPath, mount_install};

use crate::error::{ExportError, Result};
use crate::mesh::Resolver;

/// One input file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The name of the file: its lower-case install path, or for a folder on disk its path
    /// relative to the listed folder (lower case, `/` separated). Output files mirror it.
    pub key: String,
    real: Option<PathBuf>,
}

impl Entry {
    /// The folder part of the key (`""` if none), without a trailing slash.
    pub fn dir(&self) -> &str {
        self.key.rsplit_once('/').map_or("", |(d, _)| d)
    }

    /// The file name without its extension.
    pub fn stem(&self) -> &str {
        let name = self.key.rsplit('/').next().unwrap_or(&self.key);
        name.rsplit_once('.').map_or(name, |(s, _)| s)
    }

    /// The key with its extension replaced by `ext` (no dot).
    pub fn with_extension(&self, ext: &str) -> String {
        let base = match self.key.rsplit_once('/') {
            Some((d, n)) => (format!("{d}/"), n),
            None => (String::new(), self.key.as_str()),
        };
        let stem = base.1.rsplit_once('.').map_or(base.1, |(s, _)| s);
        format!("{}{stem}.{ext}", base.0)
    }
}

/// The input side of an export.
pub enum Source {
    /// Files on disk.
    Disk,
    /// An install mounted through the VFS.
    Install(Box<dyn Vfs>),
}

impl Source {
    /// Mount the install in `dir` (the folder with `All_PC_01.gro`).
    ///
    /// # Errors
    /// [`ExportError::Read`] if the folder cannot be mounted.
    pub fn install(dir: &Path) -> Result<Self> {
        let vfs = mount_install(dir).map_err(|e| {
            ExportError::read(dir.display(), format!("cannot mount the install: {e}"))
        })?;
        Ok(Self::Install(Box::new(vfs)))
    }

    /// One input named on the command line: a VFS path with an install, a file path without.
    ///
    /// # Errors
    /// [`ExportError::Read`] if the VFS path is malformed.
    pub fn entry(&self, arg: &str) -> Result<Entry> {
        match self {
            Self::Disk => Ok(Entry {
                key: arg.replace('\\', "/").to_lowercase(),
                real: Some(PathBuf::from(arg)),
            }),
            Self::Install(_) => {
                let p = VfsPath::new(arg).map_err(|e| ExportError::read(arg, e))?;
                Ok(Entry {
                    key: p.as_str().to_string(),
                    real: None,
                })
            }
        }
    }

    /// The bytes of `entry`.
    ///
    /// # Errors
    /// [`ExportError::Read`] naming the file.
    pub fn read(&self, entry: &Entry) -> Result<Vec<u8>> {
        match self {
            Self::Disk => {
                let path = entry.real.as_deref().unwrap_or(Path::new(&entry.key));
                std::fs::read(path).map_err(|e| ExportError::read(path.display(), e))
            }
            Self::Install(vfs) => {
                let p = VfsPath::new(&entry.key).map_err(|e| ExportError::read(&entry.key, e))?;
                vfs.read(&p).map_err(|e| ExportError::read(&entry.key, e))
            }
        }
    }

    /// Every file below `folder` whose extension is one of `exts` (lower case, no dot), in
    /// sorted key order. With an install `folder` is a VFS folder (`""` or `/` for the root);
    /// on disk it is a directory and the keys are relative to it.
    ///
    /// # Errors
    /// [`ExportError::Read`] if the folder cannot be listed.
    pub fn list(&self, folder: &str, exts: &[&str]) -> Result<Vec<Entry>> {
        let wanted = |name: &str| {
            name.rsplit_once('.')
                .is_some_and(|(_, e)| exts.contains(&e.to_lowercase().as_str()))
        };
        let mut out = Vec::new();
        match self {
            Self::Disk => {
                let root = PathBuf::from(folder);
                let mut stack = vec![(root.clone(), String::new())];
                while let Some((dir, prefix)) = stack.pop() {
                    let rd =
                        std::fs::read_dir(&dir).map_err(|e| ExportError::read(dir.display(), e))?;
                    for item in rd {
                        let item = item.map_err(|e| ExportError::read(dir.display(), e))?;
                        let name = item.file_name().to_string_lossy().to_lowercase();
                        let key = format!("{prefix}{name}");
                        let path = item.path();
                        if path.is_dir() {
                            stack.push((path, format!("{key}/")));
                        } else if wanted(&name) {
                            out.push(Entry {
                                key,
                                real: Some(path),
                            });
                        }
                    }
                }
            }
            Self::Install(vfs) => {
                let trimmed = folder.trim_matches(['/', '\\']);
                let root = if trimmed.is_empty() {
                    VfsPath::root()
                } else {
                    VfsPath::new(trimmed).map_err(|e| ExportError::read(folder, e))?
                };
                let mut stack = vec![root];
                while let Some(dir) = stack.pop() {
                    let children = vfs
                        .list(&dir)
                        .map_err(|e| ExportError::read(dir.as_str(), e))?;
                    for child in children {
                        let path = dir
                            .join(&child.name)
                            .map_err(|e| ExportError::read(&child.name, e))?;
                        if child.kind == EntryKind::Dir {
                            stack.push(path);
                        } else if wanted(&child.name) {
                            out.push(Entry {
                                key: path.as_str().to_string(),
                                real: None,
                            });
                        }
                    }
                }
            }
        }
        out.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(out)
    }
}

impl Resolver for Source {
    fn read(&self, path: &str) -> Option<Vec<u8>> {
        match self {
            Self::Disk => None,
            Self::Install(vfs) => {
                let p = VfsPath::new(path).ok()?;
                vfs.read(&p).ok()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(key: &str) -> Entry {
        Entry {
            key: key.to_string(),
            real: None,
        }
    }

    #[test]
    fn key_parts() {
        let e = entry("content/a/b/body.tex");
        assert_eq!((e.dir(), e.stem()), ("content/a/b", "body"));
        assert_eq!(e.with_extension("png"), "content/a/b/body.png");
        let e = entry("body.tex");
        assert_eq!((e.dir(), e.stem()), ("", "body"));
        assert_eq!(e.with_extension("glb"), "body.glb");
        let e = entry("a.b/noext");
        assert_eq!(e.stem(), "noext");
        assert_eq!(e.with_extension("png"), "a.b/noext.png");
    }

    #[test]
    fn disk_listing_is_sorted_lower_case_and_filtered() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("Sub/Deep")).unwrap();
        for f in ["B.tex", "a.TEX", "skip.txt", "Sub/c.tex", "Sub/Deep/d.bmf"] {
            std::fs::write(root.join(f), b"x").unwrap();
        }
        let src = Source::Disk;
        let tex = src.list(root.to_str().unwrap(), &["tex"]).unwrap();
        let keys: Vec<_> = tex.iter().map(|e| e.key.as_str()).collect();
        assert_eq!(keys, ["a.tex", "b.tex", "sub/c.tex"]);
        let all = src.list(root.to_str().unwrap(), &["tex", "bmf"]).unwrap();
        assert_eq!(all.len(), 4);
        assert_eq!(src.read(&tex[0]).unwrap(), b"x");
        assert!(
            src.list(root.join("nope").to_str().unwrap(), &["tex"])
                .is_err()
        );
    }
}
