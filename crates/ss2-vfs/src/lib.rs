//! ss2-vfs — virtual file system over .gro archives and loose dirs (track T02).
//!
//! This is the **v0 contract** (roadmap T00-014, `docs/rfcs/0001-vfs-and-asset-api.md`): the
//! [`Vfs`] trait every consumer codes against, the [`VfsPath`] type that names files, and
//! implementations for ZIP archives ([`ArchiveVfs`]), loose files ([`DirectoryVfs`]), and tests
//! ([`MemoryVfs`]). Combine them with [`LayeredVfs`].
//!
//! ```
//! use ss2_vfs::{MemoryVfs, Vfs, VfsPath};
//!
//! let mut vfs = MemoryVfs::new();
//! vfs.insert("Textures/Wall.tex", b"pixels".to_vec()).unwrap();
//!
//! // Lookups are case-insensitive and accept both separators.
//! let path = VfsPath::new(r"textures\WALL.TEX").unwrap();
//! assert_eq!(vfs.read(&path).unwrap(), b"pixels");
//! assert_eq!(vfs.stat(&path).unwrap().size, 6);
//!
//! let dir = VfsPath::new("textures").unwrap();
//! let names: Vec<_> = vfs.list(&dir).unwrap().into_iter().map(|e| e.name).collect();
//! assert_eq!(names, ["wall.tex"]);
//! ```

mod archive;
mod check;
mod directory;
mod install;
mod layered;
mod memory;
mod modcheck;
mod mods;
mod path;

use std::io::{Read, Seek};

use ss2_core::{Error, Result};

pub use archive::ArchiveVfs;
pub use check::{ArchiveInfo, Finding, InstallReport, PatchLevel, Severity, check_install};
pub use directory::DirectoryVfs;
pub use install::{archive_load_order, mount_install};
pub use layered::LayeredVfs;
pub use memory::MemoryVfs;
pub use modcheck::{
    Lint, ModCheckEnv, ModFinding, ModLayer, ModReport, ModSpec, ModSummary, ModUnderTest,
    check_install_mods, check_mods, installed_mod_names,
};
pub use mods::{
    ModInfo, ModManifest, ModModule, Nfo, discover_mods, mod_archive_names, mount_mod,
    parse_current_mod, selected_mod,
};
pub use path::VfsPath;

/// A readable, seekable byte stream returned by [`Vfs::open`].
///
/// Blanket-implemented for everything that is `Read + Seek + Send`, so a `Cursor`, a `File` or a
/// memory-mapped slice all qualify.
pub trait ReadSeek: Read + Seek + Send {}

impl<T: Read + Seek + Send> ReadSeek for T {}

/// What a path points at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EntryKind {
    /// A regular file.
    File,
    /// A directory (possibly merged from several mounts).
    Dir,
}

/// Result of [`Vfs::stat`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stat {
    /// File or directory.
    pub kind: EntryKind,
    /// Size in bytes for files; `0` for directories.
    pub size: u64,
}

/// One child returned by [`Vfs::list`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DirEntry {
    /// Normalized last path segment (lower-case, no separators).
    pub name: String,
    /// File or directory.
    pub kind: EntryKind,
}

/// Read-only view of the game's files, however they are stored (archives, loose directories,
/// mod layers, memory).
///
/// Implementations are `Send + Sync` and take `&self`: many threads read at once. All paths are
/// already-normalized [`VfsPath`]s, so implementations never deal with case or separators.
/// Failures are returned as [`ss2_core::Error`], never panics: [`Error::NotFound`] when the path
/// does not exist, [`Error::Io`] for read failures, [`Error::Invalid`] for corrupt archives.
pub trait Vfs: Send + Sync {
    /// Kind and size of `path`.
    ///
    /// # Errors
    /// [`Error::NotFound`] if nothing is mounted at `path`.
    fn stat(&self, path: &VfsPath) -> Result<Stat>;

    /// Open a file for streaming reads (use for large files such as `.wld` and `.ogv`).
    ///
    /// # Errors
    /// [`Error::NotFound`] if `path` is missing or is a directory; [`Error::Io`] on read failure.
    fn open(&self, path: &VfsPath) -> Result<Box<dyn ReadSeek>>;

    /// Children of directory `dir`, **sorted by name** and de-duplicated across mounts (a name
    /// present in several layers appears once). Sorted output keeps results deterministic.
    ///
    /// # Errors
    /// [`Error::NotFound`] if `dir` is missing or is a file.
    fn list(&self, dir: &VfsPath) -> Result<Vec<DirEntry>>;

    /// Read a whole file into memory. The default goes through [`Vfs::open`]; implementations
    /// that can do better (mmap, a single archive read) should override it.
    ///
    /// # Errors
    /// As [`Vfs::open`].
    fn read(&self, path: &VfsPath) -> Result<Vec<u8>> {
        let mut file = self.open(path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|e| Error::io(path.as_str(), e))?;
        Ok(bytes)
    }

    /// `true` if anything (file or directory) is mounted at `path`.
    fn exists(&self, path: &VfsPath) -> bool {
        self.stat(path).is_ok()
    }
}
