//! The byte envelope shared by every value of the campaign module: a four-byte tag, a format version (`u16`), the body (little endian integers,
//! strings as a `u16` length and UTF-8) and the CRC-32 of everything before it.

use super::BytesError;
use crate::level_progress::crc32;

/// Writes the envelope.
pub(super) struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    pub(super) fn new(magic: [u8; 4], version: u16) -> Self {
        let mut buf = Vec::new();
        buf.extend_from_slice(&magic);
        buf.extend_from_slice(&version.to_le_bytes());
        Self { buf }
    }

    pub(super) fn u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    pub(super) fn bool(&mut self, v: bool) {
        self.buf.push(u8::from(v));
    }

    pub(super) fn u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub(super) fn u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub(super) fn u64(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub(super) fn raw(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// A string: `u16` length, then the bytes. Callers validate the length first (every writer in this module only holds strings that passed
    /// their limits, all far below `u16::MAX`).
    pub(super) fn str(&mut self, s: &str) {
        self.u16(u16::try_from(s.len()).unwrap_or(u16::MAX));
        self.buf.extend_from_slice(s.as_bytes());
    }

    pub(super) fn finish(mut self) -> Vec<u8> {
        let crc = crc32(&self.buf);
        self.buf.extend_from_slice(&crc.to_le_bytes());
        self.buf
    }
}

/// Reads the envelope.
pub(super) struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    /// Check the length, the checksum, the tag and the version, and return a reader over the body.
    pub(super) fn open(bytes: &'a [u8], magic: [u8; 4], version: u16) -> Result<Self, BytesError> {
        if bytes.len() < 4 + 2 + 4 {
            return Err(BytesError::TooShort);
        }
        let (body, tail) = bytes.split_at(bytes.len() - 4);
        let stored = u32::from_le_bytes([tail[0], tail[1], tail[2], tail[3]]);
        if crc32(body) != stored {
            return Err(BytesError::BadChecksum);
        }
        let mut r = Reader { data: body, pos: 0 };
        if r.take(4)? != magic {
            return Err(BytesError::WrongMagic);
        }
        let found = r.u16()?;
        if found != version {
            return Err(BytesError::UnknownVersion { found });
        }
        Ok(r)
    }

    pub(super) fn take(&mut self, n: usize) -> Result<&'a [u8], BytesError> {
        let end = self
            .pos
            .checked_add(n)
            .filter(|e| *e <= self.data.len())
            .ok_or(BytesError::Truncated)?;
        let s = &self.data[self.pos..end];
        self.pos = end;
        Ok(s)
    }

    pub(super) fn u8(&mut self) -> Result<u8, BytesError> {
        Ok(self.take(1)?[0])
    }

    pub(super) fn bool(&mut self) -> Result<bool, BytesError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(BytesError::Invalid("a flag is not 0 or 1")),
        }
    }

    pub(super) fn u16(&mut self) -> Result<u16, BytesError> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    pub(super) fn u32(&mut self) -> Result<u32, BytesError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub(super) fn u64(&mut self) -> Result<u64, BytesError> {
        let b = self.take(8)?;
        Ok(u64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    /// A string of at most `max` bytes.
    pub(super) fn string(&mut self, max: usize) -> Result<String, BytesError> {
        let len = usize::from(self.u16()?);
        if len > max {
            return Err(BytesError::Invalid("a text is too long"));
        }
        let bytes = self.take(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| BytesError::Invalid("a text is not UTF-8"))
    }

    /// A count of elements that each take at least `min_each` bytes: refused when it is over `max` or cannot fit in the bytes that are left (so a
    /// hostile count never makes the reader allocate or loop for long).
    pub(super) fn count(&mut self, max: usize, min_each: usize) -> Result<usize, BytesError> {
        let n = usize::try_from(self.u32()?).unwrap_or(usize::MAX);
        if n > max {
            return Err(BytesError::Invalid("a count is over its limit"));
        }
        if n.saturating_mul(min_each) > self.data.len() - self.pos {
            return Err(BytesError::Truncated);
        }
        Ok(n)
    }

    /// The body must be used up.
    pub(super) fn finish(self) -> Result<(), BytesError> {
        if self.pos == self.data.len() {
            Ok(())
        } else {
            Err(BytesError::Trailing)
        }
    }
}

#[cfg(test)]
#[path = "codec_tests.rs"]
mod tests;
