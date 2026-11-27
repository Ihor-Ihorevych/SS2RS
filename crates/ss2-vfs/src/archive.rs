//! [`ArchiveVfs`]: one `.gro` archive (a standard ZIP file) as a read-only [`Vfs`] layer.
//!
//! The central directory is turned into an index by normalized path. Install and mod mounts
//! reuse a persistent index on later launches. File data is read on demand: **stored** entries
//! stream straight from the archive file, and **deflated** entries are decompressed into memory.
//! Whole-file reads verify their CRC.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, BufReader, Cursor, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use ss2_core::{Error, Result};
use zip::{CompressionMethod, ZipArchive};

use crate::{DirEntry, EntryKind, ReadSeek, Stat, Vfs, VfsPath};

#[path = "archive_cache.rs"]
mod cache;

#[derive(Serialize, Deserialize)]
enum RecordKind {
    File(Entry),
    Dir,
    Skipped,
}

/// Keep archive order, including directory and skipped entries, when rebuilding the VFS index.
#[derive(Serialize, Deserialize)]
struct Record {
    name: String,
    kind: RecordKind,
}

#[derive(Serialize, Deserialize)]
struct Entry {
    /// Index for the ZIP reader when the entry cannot be read directly.
    index: usize,
    /// Uncompressed size in bytes.
    size: u64,
    payload: Option<Payload>,
}

#[derive(Serialize, Deserialize)]
struct Payload {
    start: u64,
    len: u64,
    crc32: u32,
    deflated: bool,
}

impl Payload {
    fn fits_archive(&self, size: u64, archive_len: u64) -> bool {
        self.start >= 30
            && self
                .start
                .checked_add(self.len)
                .is_some_and(|end| end <= archive_len)
            && (self.deflated || self.len == size)
    }
}

/// A `.gro` archive mounted as a [`Vfs`] (see the module docs).
///
/// Entry names are normalized like every [`VfsPath`] (case-insensitive, `/` and `\` equal).
/// Entries whose names cannot be a valid path (`..` segments, drive letters, control
/// characters) are **not exposed**; they are listed in [`skipped_entries`](Self::skipped_entries).
/// If two entries normalize to the same path the **last one in the archive wins**.
pub struct ArchiveVfs {
    path: PathBuf,
    name: String,
    files: BTreeMap<VfsPath, Entry>,
    /// Directory path -> (child name -> kind). Contains the root and every ancestor directory.
    children: BTreeMap<VfsPath, BTreeMap<String, EntryKind>>,
    skipped: Vec<String>,
    duplicates: Vec<String>,
    file: Arc<Mutex<File>>,
}

fn zip_error(path: &Path, name: &str, e: zip::result::ZipError) -> Error {
    match e {
        zip::result::ZipError::Io(io) => Error::io(path, io),
        other => Error::invalid("archive", format!("{name}: {other}")),
    }
}

/// Make `path` and all its ancestors known as directories.
fn register_dirs(children: &mut BTreeMap<VfsPath, BTreeMap<String, EntryKind>>, dir: &VfsPath) {
    let mut current = dir.clone();
    children.entry(current.clone()).or_default();
    while let (Some(parent), Some(name)) =
        (current.parent(), current.file_name().map(str::to_owned))
    {
        children
            .entry(parent.clone())
            .or_default()
            .insert(name, EntryKind::Dir);
        current = parent;
    }
}

impl ArchiveVfs {
    /// Open the archive at `path` and index it from its ZIP headers.
    ///
    /// # Errors
    /// [`Error::Io`] if the file cannot be opened or read; [`Error::Invalid`] (naming the file)
    /// if it is not a valid ZIP archive.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        Self::from_cached_file(path.as_ref(), None)
    }

    pub(crate) fn cache_dir() -> Option<PathBuf> {
        directories::BaseDirs::new().map(|dirs| dirs.cache_dir().join("SS2RS/archive-index"))
    }

    pub(crate) fn from_cached_file(path: &Path, cache_dir: Option<&Path>) -> Result<Self> {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        let mut file = File::open(path).map_err(|e| Error::io(path, e))?;
        let cache = cache_dir.and_then(|dir| cache::ArchiveCache::new(dir, path, &mut file));
        let records = match cache.as_ref().and_then(|cache| cache.load(&mut file)) {
            Some(records) => records,
            None => {
                let (directory_start, records) = scan_records(path, &name, &mut file)?;
                if let Some(cache) = cache {
                    let _ = cache.save(&mut file, directory_start, &records);
                }
                records
            }
        };

        let mut files = BTreeMap::new();
        let mut children: BTreeMap<VfsPath, BTreeMap<String, EntryKind>> = BTreeMap::new();
        children.insert(VfsPath::root(), BTreeMap::new());
        let mut skipped = Vec::new();
        let mut duplicates = Vec::new();

        for Record {
            name: entry_name,
            kind,
        } in records
        {
            if matches!(kind, RecordKind::Skipped) {
                skipped.push(entry_name);
                continue;
            }
            let Ok(vpath) = VfsPath::new(&entry_name) else {
                skipped.push(entry_name);
                continue;
            };
            let RecordKind::File(entry) = kind else {
                if !vpath.is_root() {
                    register_dirs(&mut children, &vpath);
                }
                continue;
            };
            if vpath.is_root() {
                skipped.push(entry_name);
                continue;
            }
            if let Some(parent) = vpath.parent() {
                register_dirs(&mut children, &parent);
                if let Some(file_name) = vpath.file_name() {
                    children
                        .entry(parent)
                        .or_default()
                        .insert(file_name.to_owned(), EntryKind::File);
                }
            }
            if files.insert(vpath, entry).is_some() {
                duplicates.push(entry_name);
            }
        }

        Ok(Self {
            path: path.to_path_buf(),
            name,
            files,
            children,
            skipped,
            duplicates,
            file: Arc::new(Mutex::new(file)),
        })
    }

    /// The archive's file name (for example `Patch_02_090_593613.gro`).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Number of files (not directories) that are exposed.
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Raw names of entries that were not exposed because they are not valid VFS paths.
    pub fn skipped_entries(&self) -> &[String] {
        &self.skipped
    }

    /// Raw names of entries that replaced an earlier entry of the same archive with the same
    /// normalized path (the same name twice, or names that differ only by case or by `/` against
    /// `\`). The last one wins; the earlier ones are unreachable. Used by the mod checker.
    pub fn duplicate_entries(&self) -> &[String] {
        &self.duplicates
    }

    fn entry(&self, path: &VfsPath) -> Result<&Entry> {
        self.files
            .get(path)
            .ok_or_else(|| Error::NotFound(path.to_string()))
    }
}

fn scan_records(path: &Path, name: &str, file: &mut File) -> Result<(u64, Vec<Record>)> {
    let archive_len = file.metadata().map_err(|e| Error::io(path, e))?.len();
    let mut zip = ZipArchive::new(file).map_err(|e| zip_error(path, name, e))?;
    let directory_start = zip.central_directory_start();
    let records = (0..zip.len())
        .map(|index| {
            let raw = zip
                .by_index_raw(index)
                .map_err(|e| zip_error(path, name, e))?;
            let Ok(entry_name) = raw.name() else {
                return Ok(Record {
                    name: format!("<entry {index}: undecodable name>"),
                    kind: RecordKind::Skipped,
                });
            };
            let kind = if raw.is_dir() {
                RecordKind::Dir
            } else {
                let size = raw.size();
                let payload = match (raw.encrypted(), raw.compression(), raw.data_start()) {
                    (
                        false,
                        CompressionMethod::Stored | CompressionMethod::Deflated,
                        Some(start),
                    ) => Some(Payload {
                        start,
                        len: raw.compressed_size(),
                        crc32: raw.crc32(),
                        deflated: raw.compression() == CompressionMethod::Deflated,
                    })
                    .filter(|payload| payload.fits_archive(size, archive_len)),
                    _ => None,
                };
                RecordKind::File(Entry {
                    index,
                    size,
                    payload,
                })
            };
            Ok(Record {
                name: entry_name.into_owned(),
                kind,
            })
        })
        .collect::<Result<_>>()?;
    Ok((directory_start, records))
}

/// A read-only, seekable view of `len` bytes starting at `start` inside an open file.
struct FileWindow {
    file: Arc<Mutex<File>>,
    start: u64,
    len: u64,
    pos: u64,
}

impl Read for FileWindow {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pos >= self.len || buf.is_empty() {
            return Ok(0);
        }
        let want =
            usize::try_from(self.len - self.pos).map_or(buf.len(), |rest| buf.len().min(rest));
        let mut file = self
            .file
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        file.seek(SeekFrom::Start(self.start + self.pos))?;
        let n = file.read(&mut buf[..want])?;
        self.pos += n as u64;
        Ok(n)
    }
}

impl Seek for FileWindow {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let target = match from {
            SeekFrom::Start(n) => i128::from(n),
            SeekFrom::End(d) => i128::from(self.len) + i128::from(d),
            SeekFrom::Current(d) => i128::from(self.pos) + i128::from(d),
        };
        if target < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "seek before the start of the file",
            ));
        }
        self.pos = u64::try_from(target)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "seek position too large"))?;
        Ok(self.pos)
    }
}

impl Vfs for ArchiveVfs {
    fn stat(&self, path: &VfsPath) -> Result<Stat> {
        if let Some(entry) = self.files.get(path) {
            Ok(Stat {
                kind: EntryKind::File,
                size: entry.size,
            })
        } else if self.children.contains_key(path) {
            Ok(Stat {
                kind: EntryKind::Dir,
                size: 0,
            })
        } else {
            Err(Error::NotFound(path.to_string()))
        }
    }

    fn open(&self, path: &VfsPath) -> Result<Box<dyn ReadSeek>> {
        let entry = self.entry(path)?;
        if let Some(payload) = entry.payload.as_ref().filter(|payload| !payload.deflated) {
            return Ok(Box::new(FileWindow {
                file: Arc::clone(&self.file),
                start: payload.start,
                len: entry.size,
                pos: 0,
            }));
        }
        Ok(Box::new(Cursor::new(self.read(path)?)))
    }

    fn read(&self, path: &VfsPath) -> Result<Vec<u8>> {
        let entry = self.entry(path)?;
        let mut archive_file = self
            .file
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // Cap the up-front allocation even when the archive declares a very large file.
        let mut bytes = Vec::with_capacity(usize::try_from(entry.size).unwrap_or(0).min(64 << 20));
        if let Some(payload) = &entry.payload {
            payload
                .read(&mut archive_file, entry.size, &mut bytes)
                .map_err(|e| Error::io(&self.path, e))?;
            return Ok(bytes);
        }

        // Keep the ZIP library's format handling and errors for other archive entries.
        let mut zip = ZipArchive::new(&mut *archive_file)
            .map_err(|e| zip_error(&self.path, &self.name, e))?;
        let mut file = zip
            .by_index(entry.index)
            .map_err(|e| zip_error(&self.path, &self.name, e))?;
        file.read_to_end(&mut bytes)
            .map_err(|e| Error::io(&self.path, e))?;
        Ok(bytes)
    }

    fn list(&self, dir: &VfsPath) -> Result<Vec<DirEntry>> {
        match self.children.get(dir) {
            Some(kids) if !self.files.contains_key(dir) => Ok(kids
                .iter()
                .map(|(name, kind)| DirEntry {
                    name: name.clone(),
                    kind: *kind,
                })
                .collect()),
            _ => Err(Error::NotFound(dir.to_string())),
        }
    }
}

impl Payload {
    fn read(&self, file: &mut File, size: u64, bytes: &mut Vec<u8>) -> io::Result<()> {
        file.seek(SeekFrom::Start(self.start))?;
        let mut input = file.take(self.len);
        if self.deflated {
            flate2::bufread::DeflateDecoder::new(BufReader::new(input))
                .take(size.saturating_add(1))
                .read_to_end(bytes)?;
        } else {
            input.read_to_end(bytes)?;
        }
        if bytes.len() as u64 > size || crc32fast::hash(bytes) != self.crc32 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "archive entry size or checksum is invalid",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EntryKind, Vfs};
    use ss2_core::Error;
    use ss2_testkit::fixtures::ZipBuilder;
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::path::PathBuf;

    fn p(s: &str) -> VfsPath {
        VfsPath::new(s).unwrap()
    }

    /// Write a stored (uncompressed) archive made with the testkit builder.
    fn stored(dir: &tempfile::TempDir, name: &str, files: &[(&str, &[u8])]) -> PathBuf {
        let mut b = ZipBuilder::new();
        for (n, d) in files {
            b = b.add(*n, d.to_vec());
        }
        let path = dir.path().join(name);
        std::fs::write(&path, b.finish().unwrap()).unwrap();
        path
    }

    /// Write an archive with the given (name, data, deflate?) entries using the real zip writer.
    fn mixed(dir: &tempfile::TempDir, name: &str, files: &[(&str, &[u8], bool)]) -> PathBuf {
        mixed_with_comment(dir, name, files, "")
    }

    fn mixed_with_comment(
        dir: &tempfile::TempDir,
        name: &str,
        files: &[(&str, &[u8], bool)],
        comment: &str,
    ) -> PathBuf {
        let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        w.set_comment(comment).unwrap();
        for (n, d, deflate) in files {
            let method = if *deflate {
                zip::CompressionMethod::Deflated
            } else {
                zip::CompressionMethod::Stored
            };
            w.start_file(
                *n,
                zip::write::SimpleFileOptions::default().compression_method(method),
            )
            .unwrap();
            w.write_all(d).unwrap();
        }
        let path = dir.path().join(name);
        std::fs::write(&path, w.finish().unwrap().into_inner()).unwrap();
        path
    }

    fn sample() -> (tempfile::TempDir, ArchiveVfs) {
        let dir = tempfile::tempdir().unwrap();
        let big = "pixel ".repeat(500).into_bytes();
        let path = mixed(
            &dir,
            "All_PC_01.gro",
            &[
                ("Content/Textures/Wall.tex", b"stored-wall", false),
                ("Content/Textures/Big.tex", &big, true),
                ("Content/Models/Player.mdl", b"mdl", false),
                ("readme.txt", b"hi", false),
            ],
        );
        let vfs = ArchiveVfs::from_file(&path).unwrap();
        (dir, vfs)
    }

    fn cache_file(dir: &Path) -> PathBuf {
        let mut entries = std::fs::read_dir(dir).unwrap();
        let path = entries.next().unwrap().unwrap().path();
        assert!(entries.next().is_none());
        path
    }

    #[test]
    fn warm_mount_reuses_the_cache_and_can_read_both_compression_methods() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("cache");
        let path = mixed(
            &dir,
            "a.gro",
            &[
                ("A.TXT", b"first", false),
                ("a.txt", b"stored", false),
                ("deep/zip.txt", b"deflated", true),
                ("empty/", b"", false),
                ("../bad.txt", b"", false),
            ],
        );
        ArchiveVfs::from_cached_file(&path, Some(&cache)).unwrap();
        let cache_path = cache_file(&cache);
        File::options()
            .write(true)
            .open(&cache_path)
            .unwrap()
            .set_modified(std::time::UNIX_EPOCH)
            .unwrap();
        let modified = cache_path.metadata().unwrap().modified().unwrap();

        let archive = ArchiveVfs::from_cached_file(&path, Some(&cache)).unwrap();
        assert_eq!(cache_path.metadata().unwrap().modified().unwrap(), modified);
        assert_eq!(archive.file_count(), 2);
        assert!(archive.list(&p("empty")).unwrap().is_empty());
        assert_eq!(archive.skipped_entries(), ["../bad.txt"]);
        assert_eq!(archive.duplicate_entries(), ["a.txt"]);
        assert_eq!(archive.read(&p("a.txt")).unwrap(), b"stored");
        assert_eq!(archive.read(&p("deep/zip.txt")).unwrap(), b"deflated");
        let mut stream = archive.open(&p("a.txt")).unwrap();
        stream.seek(SeekFrom::End(-3)).unwrap();
        let mut tail = Vec::new();
        stream.read_to_end(&mut tail).unwrap();
        assert_eq!(tail, b"red");
    }

    #[test]
    fn stale_cache_is_rebuilt_even_for_same_size_and_timestamp() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("cache");
        // Padding before and after the changed entry keeps it outside all three samples.
        let first = [0; 8192];
        let padding_name = "p".repeat(8192);
        let comment = "x".repeat(8192);
        let path = mixed_with_comment(
            &dir,
            "a.gro",
            &[(&padding_name, &first, false), ("old.txt", b"one", false)],
            &comment,
        );
        ArchiveVfs::from_cached_file(&path, Some(&cache)).unwrap();
        let before = path.metadata().unwrap();
        mixed_with_comment(
            &dir,
            "a.gro",
            &[(&padding_name, &first, false), ("new.txt", b"two", false)],
            &comment,
        );
        File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(before.modified().unwrap())
            .unwrap();
        assert_eq!(path.metadata().unwrap().len(), before.len());

        let archive = ArchiveVfs::from_cached_file(&path, Some(&cache)).unwrap();
        assert!(!archive.exists(&p("old.txt")));
        assert_eq!(archive.read(&p("new.txt")).unwrap(), b"two");
    }

    #[test]
    fn stored_stream_reads_the_archive_that_was_mounted() {
        let dir = tempfile::tempdir().unwrap();
        let path = stored(&dir, "a.gro", &[("a.txt", b"old")]);
        let archive = ArchiveVfs::from_file(&path).unwrap();
        std::fs::rename(&path, dir.path().join("original.gro")).unwrap();
        stored(&dir, "a.gro", &[("a.txt", b"new")]);

        let mut stream = archive.open(&p("a.txt")).unwrap();
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"old");
    }

    #[test]
    fn a_corrupt_cache_is_rebuilt_and_an_unwritable_cache_is_optional() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("cache");
        let path = stored(&dir, "a.gro", &[("a.txt", b"data")]);
        ArchiveVfs::from_cached_file(&path, Some(&cache)).unwrap();
        let cache_path = cache_file(&cache);
        let mut index = std::fs::read(&cache_path).unwrap();
        let last = index.last_mut().unwrap();
        *last ^= 1;
        std::fs::write(&cache_path, &index).unwrap();

        let archive = ArchiveVfs::from_cached_file(&path, Some(&cache)).unwrap();
        assert_eq!(archive.read(&p("a.txt")).unwrap(), b"data");
        assert_ne!(std::fs::read(&cache_path).unwrap(), index);

        let archive = ArchiveVfs::from_cached_file(&path, Some(&cache_path)).unwrap();
        assert_eq!(archive.read(&p("a.txt")).unwrap(), b"data");
    }

    #[test]
    fn reads_stored_and_deflated_entries() {
        let (_d, v) = sample();
        assert_eq!(
            v.read(&p("content/textures/wall.tex")).unwrap(),
            b"stored-wall"
        );
        assert_eq!(
            v.read(&p("content/textures/big.tex")).unwrap(),
            "pixel ".repeat(500).into_bytes()
        );
    }

    #[test]
    fn lookups_ignore_case_and_both_separators_even_inside_the_archive() {
        let dir = tempfile::tempdir().unwrap();
        let path = stored(&dir, "a.gro", &[(r"Content\Models/Player.MDL", b"x")]);
        let v = ArchiveVfs::from_file(&path).unwrap();
        assert_eq!(v.read(&p("CONTENT/models/player.mdl")).unwrap(), b"x");
    }

    #[test]
    fn stat_reports_kind_and_size() {
        let (_d, v) = sample();
        assert_eq!(v.stat(&p("readme.txt")).unwrap().size, 2);
        assert_eq!(v.stat(&p("content/textures/big.tex")).unwrap().size, 3000);
        assert_eq!(v.stat(&p("content/textures")).unwrap().kind, EntryKind::Dir);
        assert_eq!(v.stat(&VfsPath::root()).unwrap().kind, EntryKind::Dir);
        assert!(matches!(v.stat(&p("nope")), Err(Error::NotFound(_))));
    }

    #[test]
    fn directories_are_derived_from_file_paths_and_listed_sorted() {
        let (_d, v) = sample();
        let root: Vec<_> = v
            .list(&VfsPath::root())
            .unwrap()
            .into_iter()
            .map(|e| (e.name, e.kind))
            .collect();
        assert_eq!(
            root,
            [
                ("content".to_string(), EntryKind::Dir),
                ("readme.txt".to_string(), EntryKind::File)
            ]
        );
        let tex: Vec<_> = v
            .list(&p("content/textures"))
            .unwrap()
            .into_iter()
            .map(|e| e.name)
            .collect();
        assert_eq!(tex, ["big.tex", "wall.tex"]);
    }

    #[test]
    fn explicit_directory_entries_create_empty_directories() {
        let dir = tempfile::tempdir().unwrap();
        let path = stored(&dir, "a.gro", &[("Empty/", b""), ("f.txt", b"1")]);
        let v = ArchiveVfs::from_file(&path).unwrap();
        assert_eq!(v.stat(&p("empty")).unwrap().kind, EntryKind::Dir);
        assert!(v.list(&p("empty")).unwrap().is_empty());
        assert_eq!(v.file_count(), 1);
    }

    #[test]
    fn unsafe_entry_names_are_skipped_and_reported_not_exposed() {
        let dir = tempfile::tempdir().unwrap();
        let path = stored(
            &dir,
            "a.gro",
            &[
                ("../evil.txt", b"x"),
                ("C:/win.txt", b"x"),
                ("ok.txt", b"1"),
            ],
        );
        let v = ArchiveVfs::from_file(&path).unwrap();
        assert_eq!(v.file_count(), 1);
        assert_eq!(v.skipped_entries().len(), 2);
        assert!(v.skipped_entries().iter().any(|n| n.contains("evil")));
        assert!(!v.exists(&p("evil.txt")));
    }

    #[test]
    fn duplicate_names_that_differ_only_by_case_keep_the_last_one() {
        let dir = tempfile::tempdir().unwrap();
        let path = mixed(
            &dir,
            "a.gro",
            &[("A.TXT", b"first", false), ("a.txt", b"second", false)],
        );
        let v = ArchiveVfs::from_file(&path).unwrap();
        assert_eq!(v.read(&p("a.txt")).unwrap(), b"second");
        assert_eq!(v.file_count(), 1);
    }

    #[test]
    fn stored_entries_stream_with_seek() {
        let (_d, v) = sample();
        let mut f = v.open(&p("content/textures/wall.tex")).unwrap();
        f.seek(SeekFrom::Start(7)).unwrap();
        let mut rest = String::new();
        f.read_to_string(&mut rest).unwrap();
        assert_eq!(rest, "wall");
        assert_eq!(f.seek(SeekFrom::End(-4)).unwrap(), 7);
        assert_eq!(f.seek(SeekFrom::Current(-2)).unwrap(), 5);
        let mut two = [0u8; 2];
        f.read_exact(&mut two).unwrap();
        assert_eq!(&two, b"d-");
    }

    #[test]
    fn seeking_past_the_end_reads_nothing_and_before_the_start_errors() {
        let (_d, v) = sample();
        let mut f = v.open(&p("readme.txt")).unwrap();
        assert_eq!(f.seek(SeekFrom::Start(100)).unwrap(), 100);
        let mut buf = [0u8; 4];
        assert_eq!(f.read(&mut buf).unwrap(), 0);
        assert!(f.seek(SeekFrom::Current(-1000)).is_err());
    }

    #[test]
    fn deflated_entries_stream_with_seek_too() {
        let (_d, v) = sample();
        let mut f = v.open(&p("content/textures/big.tex")).unwrap();
        f.seek(SeekFrom::Start(6)).unwrap();
        let mut w = [0u8; 5];
        f.read_exact(&mut w).unwrap();
        assert_eq!(&w, b"pixel");
    }

    #[test]
    fn empty_files_work_stored_and_deflated() {
        let dir = tempfile::tempdir().unwrap();
        let path = mixed(
            &dir,
            "a.gro",
            &[("s.bin", b"", false), ("d.bin", b"", true)],
        );
        let v = ArchiveVfs::from_file(&path).unwrap();
        for n in ["s.bin", "d.bin"] {
            assert_eq!(v.stat(&p(n)).unwrap().size, 0);
            assert!(v.read(&p(n)).unwrap().is_empty());
        }
    }

    #[test]
    fn opening_a_directory_or_listing_a_file_is_not_found() {
        let (_d, v) = sample();
        assert!(matches!(v.open(&p("content")), Err(Error::NotFound(_))));
        assert!(matches!(v.read(&p("content")), Err(Error::NotFound(_))));
        assert!(matches!(v.list(&p("readme.txt")), Err(Error::NotFound(_))));
        assert!(matches!(v.list(&p("nope")), Err(Error::NotFound(_))));
    }

    #[test]
    fn garbage_is_an_invalid_archive_error_that_names_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Broken_PC_01.gro");
        std::fs::write(&path, b"this is not a zip").unwrap();
        let err = ArchiveVfs::from_file(&path).err().expect("must fail");
        assert!(matches!(err, Error::Invalid { .. }), "{err:?}");
        assert!(err.to_string().contains("Broken_PC_01.gro"), "{err}");
    }

    #[test]
    fn an_empty_file_and_a_truncated_archive_are_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let empty = dir.path().join("empty.gro");
        std::fs::write(&empty, b"").unwrap();
        assert!(matches!(
            ArchiveVfs::from_file(&empty),
            Err(Error::Invalid { .. })
        ));
        let good = mixed(&dir, "good.gro", &[("a.txt", b"hello world", false)]);
        let bytes = std::fs::read(&good).unwrap();
        let cut = dir.path().join("cut.gro");
        std::fs::write(&cut, &bytes[..bytes.len() / 2]).unwrap();
        assert!(matches!(
            ArchiveVfs::from_file(&cut),
            Err(Error::Invalid { .. })
        ));
    }

    #[test]
    fn a_missing_file_is_an_io_error() {
        let dir = tempfile::tempdir().unwrap();
        let err = ArchiveVfs::from_file(dir.path().join("nope.gro"))
            .err()
            .expect("must fail");
        assert!(matches!(err, Error::Io { .. }), "{err:?}");
    }

    #[test]
    fn a_valid_archive_with_no_entries_is_an_empty_root() {
        let dir = tempfile::tempdir().unwrap();
        let path = stored(&dir, "none.gro", &[]);
        let v = ArchiveVfs::from_file(&path).unwrap();
        assert_eq!(v.file_count(), 0);
        assert!(v.list(&VfsPath::root()).unwrap().is_empty());
    }

    #[test]
    fn many_threads_can_read_stored_and_deflated_entries_at_once() {
        let (_d, v) = sample();
        let v = std::sync::Arc::new(v);
        let handles: Vec<_> = (0..8)
            .map(|i| {
                let v = v.clone();
                std::thread::spawn(move || {
                    for _ in 0..50 {
                        let name = if i % 2 == 0 {
                            "content/textures/big.tex"
                        } else {
                            "content/textures/wall.tex"
                        };
                        let data = v.read(&p(name)).unwrap();
                        assert!(!data.is_empty());
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
    }

    #[test]
    fn the_archive_reports_its_file_name() {
        let (_d, v) = sample();
        assert_eq!(v.name(), "All_PC_01.gro");
    }
}
