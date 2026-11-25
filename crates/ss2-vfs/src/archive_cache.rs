//! Best-effort, per-user index files for archives mounted by the install loader.

use std::fs::{self, File};
use std::hash::{DefaultHasher, Hash, Hasher as _};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use super::{Record, RecordKind};

// Bump the version when the index format or interpretation of ZIP entries changes.
const MAGIC: &[u8] = b"SS2ZIP\0\x03";
const MAX_INDEX_BYTES: u64 = 16 * 1024 * 1024;
const MAX_PATH_TREE_BYTES: usize = 32 * 1024 * 1024;
const SAMPLE_BYTES: usize = 4096;

#[derive(Serialize, Deserialize, PartialEq, Eq)]
struct Stamp {
    len: u64,
    modified: SystemTime,
    created: Option<SystemTime>,
    #[cfg(unix)]
    identity: (u64, u64, i64, i64),
    #[cfg(windows)]
    change_time: i64,
    samples: u32,
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn change_time(file: &File) -> io::Result<i64> {
    use std::os::windows::io::AsRawHandle;

    use windows_sys::Win32::Storage::FileSystem::{
        FILE_BASIC_INFO, FileBasicInfo, GetFileInformationByHandleEx,
    };

    let mut info = FILE_BASIC_INFO::default();
    // SAFETY: `file` owns the live handle, and FileBasicInfo writes a FILE_BASIC_INFO into
    // the aligned, initialized buffer of that exact size.
    let success = unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileBasicInfo,
            (&raw mut info).cast(),
            size_of::<FILE_BASIC_INFO>() as u32,
        )
    };
    if success == 0 {
        return Err(io::Error::last_os_error());
    }
    if info.ChangeTime == 0 {
        return Err(io::Error::other("archive change time is unavailable"));
    }
    Ok(info.ChangeTime)
}

impl Stamp {
    fn read(file: &mut File) -> io::Result<Self> {
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;

        // Read the change time independently of the restorable last-write time.
        #[cfg(windows)]
        let before_change = change_time(file)?;
        let metadata = file.metadata()?;
        let len = metadata.len();
        let modified = metadata.modified()?;
        let created = metadata.created().ok();
        #[cfg(unix)]
        let identity = (
            metadata.dev(),
            metadata.ino(),
            metadata.ctime(),
            metadata.ctime_nsec(),
        );
        let mut hash = crc32fast::Hasher::new();
        let mut buffer = [0; SAMPLE_BYTES];
        let head = len.min(SAMPLE_BYTES as u64) as usize;
        file.seek(SeekFrom::Start(0))?;
        file.read_exact(&mut buffer[..head])?;
        hash.update(&buffer[..head]);

        // Sample the trailer and comment without parsing them on a warm mount.
        let tail = (len - head as u64).min(SAMPLE_BYTES as u64) as usize;
        if tail != 0 {
            file.seek(SeekFrom::Start(len - tail as u64))?;
            file.read_exact(&mut buffer[..tail])?;
            hash.update(&buffer[..tail]);
        }
        let after = file.metadata()?;
        if after.len() != len || after.modified()? != modified || after.created().ok() != created {
            return Err(io::Error::other("archive changed while reading its stamp"));
        }
        #[cfg(unix)]
        if identity != (after.dev(), after.ino(), after.ctime(), after.ctime_nsec()) {
            return Err(io::Error::other("archive changed while reading its stamp"));
        }
        #[cfg(windows)]
        if change_time(file)? != before_change {
            return Err(io::Error::other("archive changed while reading its stamp"));
        }
        Ok(Self {
            len,
            modified,
            created,
            #[cfg(unix)]
            identity,
            #[cfg(windows)]
            change_time: before_change,
            samples: hash.finalize(),
        })
    }

    fn directory_sample(&self, file: &mut File, start: u64) -> io::Result<u32> {
        let remaining = self
            .len
            .checked_sub(start)
            .filter(|remaining| *remaining >= 22)
            .ok_or(io::ErrorKind::InvalidData)?;
        let mut buffer = [0; SAMPLE_BYTES];
        // The beginning of the directory can be outside the tail sample with a long ZIP comment.
        let sample = &mut buffer[..remaining.min(SAMPLE_BYTES as u64) as usize];
        file.seek(SeekFrom::Start(start))?;
        file.read_exact(sample)?;
        Ok(crc32fast::hash(sample))
    }
}

#[derive(Serialize, Deserialize)]
struct DirectorySample {
    start: u64,
    checksum: u32,
}

fn valid_records(records: &[Record], archive_len: u64, directory_start: u64) -> bool {
    if records.len() as u64 > archive_len / 46 {
        return false;
    }
    let mut remaining = MAX_PATH_TREE_BYTES;
    for (index, record) in records.iter().enumerate() {
        // Materializing every ancestor copies a prefix of the name. Bound the sum of these
        // copies as well as the encoded index size for a corrupt cache.
        let depth = record
            .name
            .bytes()
            .filter(|b| matches!(b, b'/' | b'\\'))
            .count()
            + 1;
        let Some(next) = record
            .name
            .len()
            .checked_mul(depth)
            .and_then(|cost| remaining.checked_sub(cost))
        else {
            return false;
        };
        remaining = next;
        if let RecordKind::File(entry) = &record.kind
            && (entry.index != index
                || entry
                    .payload
                    .as_ref()
                    .is_some_and(|payload| !payload.fits_archive(entry.size, directory_start)))
        {
            return false;
        }
    }
    true
}

pub(super) struct ArchiveCache {
    index_path: PathBuf,
    archive_path: String,
    stamp: Stamp,
}

impl ArchiveCache {
    pub(super) fn new(dir: &Path, path: &Path, file: &mut File) -> Option<Self> {
        let archive_path = fs::canonicalize(path)
            .ok()?
            .into_os_string()
            .into_string()
            .ok()?;
        let mut key = DefaultHasher::new();
        archive_path.hash(&mut key);
        Some(Self {
            index_path: dir.join(format!("{:016x}.index", key.finish())),
            archive_path,
            stamp: Stamp::read(file).ok()?,
        })
    }

    pub(super) fn load(&self, file: &mut File) -> Option<Vec<Record>> {
        let metadata = fs::symlink_metadata(&self.index_path).ok()?;
        if !metadata.is_file() || metadata.len() > MAX_INDEX_BYTES {
            return None;
        }
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        File::open(&self.index_path)
            .ok()?
            .take(MAX_INDEX_BYTES + 1)
            .read_to_end(&mut bytes)
            .ok()?;
        if bytes.len() as u64 > MAX_INDEX_BYTES {
            return None;
        }
        let (checksum, data) = bytes.strip_prefix(MAGIC)?.split_first_chunk::<4>()?;
        if crc32fast::hash(data) != u32::from_le_bytes(*checksum) {
            return None;
        }
        let (path, stamp, directory, records): (String, Stamp, DirectorySample, Vec<Record>) =
            serde_json::from_slice(data).ok()?;
        if path != self.archive_path
            || stamp != self.stamp
            || stamp.directory_sample(file, directory.start).ok()? != directory.checksum
            || !valid_records(&records, stamp.len, directory.start)
        {
            return None;
        }
        Some(records)
    }

    pub(super) fn save(
        &self,
        file: &mut File,
        directory_start: u64,
        records: &[Record],
    ) -> io::Result<()> {
        if !valid_records(records, self.stamp.len, directory_start) {
            return Ok(());
        }
        let directory = DirectorySample {
            start: directory_start,
            checksum: self.stamp.directory_sample(file, directory_start)?,
        };
        if Stamp::read(file)? != self.stamp {
            return Ok(());
        }
        let data = serde_json::to_vec(&(&self.archive_path, &self.stamp, directory, records))
            .map_err(io::Error::other)?;
        if (data.len() + MAGIC.len() + 4) as u64 > MAX_INDEX_BYTES {
            return Ok(());
        }
        let dir = self
            .index_path
            .parent()
            .expect("cache index is inside its directory");
        fs::create_dir_all(dir)?;
        let mut temp = tempfile::NamedTempFile::new_in(dir)?;
        temp.write_all(MAGIC)?;
        temp.write_all(&crc32fast::hash(&data).to_le_bytes())?;
        temp.write_all(&data)?;
        temp.persist(&self.index_path).map_err(|e| e.error)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use ss2_testkit::fixtures::ZipBuilder;

    use super::*;
    use crate::{ArchiveVfs, Vfs, VfsPath};

    #[test]
    fn checksum_valid_indexes_with_invalid_ranges_or_tree_sizes_are_rebuilt() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("a.gro");
        let contents = ZipBuilder::new()
            .add("a.txt", b"hello".to_vec())
            .finish()
            .unwrap();
        fs::write(&path, contents).unwrap();
        let cache_dir = temp.path().join("cache");
        let cache = ArchiveCache::new(&cache_dir, &path, &mut File::open(&path).unwrap()).unwrap();
        ArchiveVfs::from_cached_file(&path, Some(&cache_dir)).unwrap();
        let good = fs::read(&cache.index_path).unwrap();

        for bad_range in [Some(u64::MAX), Some(1), None] {
            let data = &good[MAGIC.len() + 4..];
            let (path_key, stamp, directory, mut records): (
                String,
                Stamp,
                DirectorySample,
                Vec<Record>,
            ) = serde_json::from_slice(data).unwrap();
            if let Some(distance_from_directory) = bad_range {
                let RecordKind::File(entry) = &mut records[0].kind else {
                    panic!("fixture has one file");
                };
                let payload = entry.payload.as_mut().unwrap();
                if distance_from_directory == u64::MAX {
                    payload.len = u64::MAX;
                } else {
                    payload.start = directory.start - distance_from_directory;
                }
            } else {
                records[0].name = "folder/".repeat(4096);
            }
            let data = serde_json::to_vec(&(path_key, stamp, directory, records)).unwrap();
            let damaged = [MAGIC, &crc32fast::hash(&data).to_le_bytes(), &data].concat();
            fs::write(&cache.index_path, &damaged).unwrap();

            let archive = ArchiveVfs::from_cached_file(&path, Some(&cache_dir)).unwrap();
            assert_eq!(
                archive.read(&VfsPath::new("a.txt").unwrap()).unwrap(),
                b"hello"
            );
            assert_eq!(fs::read(&cache.index_path).unwrap(), good);
        }
    }
}
