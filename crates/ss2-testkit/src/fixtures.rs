//! Synthetic fixtures: tiny, hand-made, license-clean test inputs that need no game data.
//!
//! Everything here is generated from scratch (see `testdata/README.md` for the policy). Nothing is
//! derived from Croteam files. Output is **deterministic**: the same input always yields the same
//! bytes, so tests may compare against fixed byte strings.
//!
//! Currently provided:
//! - [`ZipBuilder`] / [`write_minimal_zip`]: a minimal ZIP (stored entries only) writer, written
//!   from the public ZIP file format description. The `.gro` archives are standard ZIP files.
//! - [`testdata_dir`] / [`testdata_path`]: locate the committed `testdata/` folder.
//!
//! A CTSEMETA writer is intentionally **not** here yet: it must follow the container spec from
//! the RE track (`docs/re/ctsemeta-container.md`, roadmap T01-004/T01-005), see T00-032.

use std::collections::BTreeSet;
use std::io;
use std::path::{Path, PathBuf};

/// Local file header signature (`PK\x03\x04`).
const SIG_LOCAL: u32 = 0x0403_4b50;
/// Central directory file header signature (`PK\x01\x02`).
const SIG_CENTRAL: u32 = 0x0201_4b50;
/// End of central directory signature (`PK\x05\x06`).
const SIG_EOCD: u32 = 0x0605_4b50;
/// "Version needed to extract" / "version made by": 2.0.
const VERSION: u16 = 20;
/// General purpose flag bit 11: file names are UTF-8.
const FLAG_UTF8: u16 = 1 << 11;
/// DOS date for 1980-01-01, the earliest representable; fixed so output is reproducible.
const DOS_DATE: u16 = (1 << 5) | 1;
/// DOS time 00:00:00.
const DOS_TIME: u16 = 0;

const CRC_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
};

/// CRC-32 (IEEE 802.3, as used by ZIP) of `data`.
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc = CRC_TABLE[((crc ^ u32::from(b)) & 0xFF) as usize] ^ (crc >> 8);
    }
    !crc
}

fn invalid(msg: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, msg.into())
}

/// Builds a minimal, deterministic ZIP archive in memory.
///
/// Limits (this is a test fixture builder, not a general archiver): entries are **stored**
/// (no compression), no ZIP64, no encryption, at most 65 535 entries, every entry and the whole
/// archive under 4 GiB, names unique, non-empty and at most 65 535 bytes.
#[derive(Debug, Default, Clone)]
pub struct ZipBuilder {
    entries: Vec<(String, Vec<u8>)>,
}

impl ZipBuilder {
    /// An empty archive builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a file. Names use `/` as separator; a trailing `/` makes an (empty) directory entry.
    /// Errors are reported by [`finish`](Self::finish), so calls can be chained.
    #[must_use]
    pub fn add(mut self, name: impl Into<String>, data: impl Into<Vec<u8>>) -> Self {
        self.entries.push((name.into(), data.into()));
        self
    }

    /// Serialize the archive.
    ///
    /// # Errors
    /// `InvalidInput` for an empty/duplicate/oversized name, too many entries, or sizes beyond
    /// the non-ZIP64 limits.
    pub fn finish(self) -> io::Result<Vec<u8>> {
        if self.entries.len() > usize::from(u16::MAX) {
            return Err(invalid("too many entries for a non-ZIP64 archive"));
        }
        let mut seen = BTreeSet::new();
        for (name, _) in &self.entries {
            if name.is_empty() {
                return Err(invalid("empty entry name"));
            }
            if name.len() > usize::from(u16::MAX) {
                return Err(invalid(format!(
                    "entry name too long ({} bytes)",
                    name.len()
                )));
            }
            if !seen.insert(name.as_str()) {
                return Err(invalid(format!("duplicate entry name {name:?}")));
            }
        }

        let mut out: Vec<u8> = Vec::new();
        let mut central: Vec<u8> = Vec::new();
        for (name, data) in &self.entries {
            let offset = u32::try_from(out.len()).map_err(|_| invalid("archive exceeds 4 GiB"))?;
            let size = u32::try_from(data.len()).map_err(|_| invalid("entry exceeds 4 GiB"))?;
            let name_len = name.len() as u16; // checked above
            let crc = crc32(data);
            let flags = if name.is_ascii() { 0 } else { FLAG_UTF8 };

            put_u32(&mut out, SIG_LOCAL);
            put_u16(&mut out, VERSION);
            put_u16(&mut out, flags);
            put_u16(&mut out, 0); // method: stored
            put_u16(&mut out, DOS_TIME);
            put_u16(&mut out, DOS_DATE);
            put_u32(&mut out, crc);
            put_u32(&mut out, size); // compressed size
            put_u32(&mut out, size); // uncompressed size
            put_u16(&mut out, name_len);
            put_u16(&mut out, 0); // extra field length
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(data);

            put_u32(&mut central, SIG_CENTRAL);
            put_u16(&mut central, VERSION); // version made by
            put_u16(&mut central, VERSION); // version needed
            put_u16(&mut central, flags);
            put_u16(&mut central, 0); // method
            put_u16(&mut central, DOS_TIME);
            put_u16(&mut central, DOS_DATE);
            put_u32(&mut central, crc);
            put_u32(&mut central, size);
            put_u32(&mut central, size);
            put_u16(&mut central, name_len);
            put_u16(&mut central, 0); // extra
            put_u16(&mut central, 0); // comment
            put_u16(&mut central, 0); // disk number start
            put_u16(&mut central, 0); // internal attributes
            put_u32(&mut central, 0); // external attributes
            put_u32(&mut central, offset);
            central.extend_from_slice(name.as_bytes());
        }

        let cd_offset = u32::try_from(out.len()).map_err(|_| invalid("archive exceeds 4 GiB"))?;
        let cd_size =
            u32::try_from(central.len()).map_err(|_| invalid("central directory too large"))?;
        out.extend_from_slice(&central);
        let count = self.entries.len() as u16; // checked above
        put_u32(&mut out, SIG_EOCD);
        put_u16(&mut out, 0); // this disk
        put_u16(&mut out, 0); // disk with central directory
        put_u16(&mut out, count);
        put_u16(&mut out, count);
        put_u32(&mut out, cd_size);
        put_u32(&mut out, cd_offset);
        put_u16(&mut out, 0); // comment length
        if u32::try_from(out.len()).is_err() {
            return Err(invalid("archive exceeds 4 GiB"));
        }
        Ok(out)
    }
}

fn put_u16(buf: &mut Vec<u8>, v: u16) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn put_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

/// Write a minimal ZIP containing `entries` (`(name, bytes)` pairs) to `path`.
///
/// # Errors
/// Anything [`ZipBuilder::finish`] rejects, or an I/O error writing `path`.
pub fn write_minimal_zip(path: impl AsRef<Path>, entries: &[(&str, &[u8])]) -> io::Result<()> {
    let mut b = ZipBuilder::new();
    for &(name, data) in entries {
        b = b.add(name, data);
    }
    std::fs::write(path, b.finish()?)
}

/// The committed `testdata/` folder at the workspace root.
pub fn testdata_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../testdata")
}

/// A path inside `testdata/` (the file is not checked for existence).
pub fn testdata_path(relative: impl AsRef<Path>) -> PathBuf {
    testdata_dir().join(relative)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u16_at(b: &[u8], at: usize) -> u16 {
        u16::from_le_bytes([b[at], b[at + 1]])
    }
    fn u32_at(b: &[u8], at: usize) -> u32 {
        u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
    }

    /// Independent minimal reader: walks EOCD -> central directory -> local headers.
    fn read_back(zip: &[u8]) -> Vec<(String, Vec<u8>)> {
        let eocd = zip.len() - 22;
        assert_eq!(u32_at(zip, eocd), SIG_EOCD);
        let count = usize::from(u16_at(zip, eocd + 10));
        let mut pos = u32_at(zip, eocd + 16) as usize;
        let mut out = Vec::new();
        for _ in 0..count {
            assert_eq!(u32_at(zip, pos), SIG_CENTRAL);
            let crc = u32_at(zip, pos + 16);
            let size = u32_at(zip, pos + 20) as usize;
            let name_len = usize::from(u16_at(zip, pos + 28));
            let local = u32_at(zip, pos + 42) as usize;
            let name = String::from_utf8(zip[pos + 46..pos + 46 + name_len].to_vec()).unwrap();
            assert_eq!(u32_at(zip, local), SIG_LOCAL);
            let l_name_len = usize::from(u16_at(zip, local + 26));
            let l_extra_len = usize::from(u16_at(zip, local + 28));
            let start = local + 30 + l_name_len + l_extra_len;
            let data = zip[start..start + size].to_vec();
            assert_eq!(crc32(&data), crc, "crc mismatch for {name}");
            out.push((name, data));
            pos += 46 + name_len;
        }
        out
    }

    #[test]
    fn crc32_known_vectors() {
        assert_eq!(crc32(b""), 0);
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(
            crc32(b"The quick brown fox jumps over the lazy dog"),
            0x414F_A339
        );
    }

    #[test]
    fn empty_archive_is_just_an_eocd() {
        let zip = ZipBuilder::new().finish().unwrap();
        assert_eq!(zip.len(), 22);
        assert_eq!(&zip[..4], b"PK\x05\x06");
    }

    #[test]
    fn round_trips_entries_in_order() {
        let zip = ZipBuilder::new()
            .add("a.txt", b"hello".to_vec())
            .add("dir/", Vec::new())
            .add("dir/b.bin", vec![0u8, 1, 2, 255])
            .add("ünï.txt", b"utf8".to_vec())
            .finish()
            .unwrap();
        let got = read_back(&zip);
        assert_eq!(
            got,
            vec![
                ("a.txt".to_owned(), b"hello".to_vec()),
                ("dir/".to_owned(), Vec::new()),
                ("dir/b.bin".to_owned(), vec![0, 1, 2, 255]),
                ("ünï.txt".to_owned(), b"utf8".to_vec()),
            ]
        );
    }

    #[test]
    fn output_is_deterministic() {
        let make = || ZipBuilder::new().add("x", b"1".to_vec()).finish().unwrap();
        assert_eq!(make(), make());
    }

    #[test]
    fn utf8_flag_only_for_non_ascii_names() {
        let zip = ZipBuilder::new()
            .add("a", b"".to_vec())
            .add("é", b"".to_vec())
            .finish()
            .unwrap();
        let first = 0;
        let second = 30 + 1; // header + 1-byte name + empty data
        assert_eq!(u16_at(&zip, first + 6) & FLAG_UTF8, 0);
        assert_eq!(u16_at(&zip, second + 6) & FLAG_UTF8, FLAG_UTF8);
    }

    #[test]
    fn rejects_bad_names() {
        assert!(ZipBuilder::new().add("", b"".to_vec()).finish().is_err());
        assert!(
            ZipBuilder::new()
                .add("dup", b"1".to_vec())
                .add("dup", b"2".to_vec())
                .finish()
                .is_err()
        );
        let long = "n".repeat(usize::from(u16::MAX) + 1);
        assert!(ZipBuilder::new().add(long, b"".to_vec()).finish().is_err());
    }

    #[test]
    fn write_minimal_zip_writes_a_readable_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("t.gro");
        write_minimal_zip(&path, &[("a/b.txt", b"data"), ("c", b"")]).unwrap();
        let got = read_back(&std::fs::read(&path).unwrap());
        assert_eq!(got.len(), 2);
        assert_eq!(got[0], ("a/b.txt".to_owned(), b"data".to_vec()));
    }

    #[test]
    fn testdata_dir_is_the_workspace_folder() {
        assert!(testdata_dir().join("README.md").is_file());
        assert_eq!(testdata_path("x/y"), testdata_dir().join("x/y"));
    }
}
