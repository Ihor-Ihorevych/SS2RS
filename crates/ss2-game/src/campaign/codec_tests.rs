use super::*;

const MAGIC: [u8; 4] = *b"TEST";

fn envelope(body: impl FnOnce(&mut Writer)) -> Vec<u8> {
    let mut w = Writer::new(MAGIC, 1);
    body(&mut w);
    w.finish()
}

#[test]
fn the_smallest_envelope_is_ten_bytes_and_anything_shorter_is_too_short() {
    let empty = envelope(|_| {});
    assert_eq!(empty.len(), 10);
    let r = Reader::open(&empty, MAGIC, 1).unwrap();
    r.finish().unwrap();
    for n in 0..10 {
        let bytes = vec![0u8; n];
        assert_eq!(
            Reader::open(&bytes, MAGIC, 1).err(),
            Some(BytesError::TooShort),
            "{n} bytes"
        );
    }
    // Just over the minimum still opens, and the body is there to read.
    let one = envelope(|w| w.u8(7));
    let mut r = Reader::open(&one, MAGIC, 1).unwrap();
    assert_eq!(r.u8().unwrap(), 7);
    r.finish().unwrap();
}

#[test]
fn the_envelope_checks_checksum_magic_and_version_in_that_order() {
    let good = envelope(|w| w.u32(5));
    let mut bad_crc = good.clone();
    *bad_crc.last_mut().unwrap() ^= 1;
    assert_eq!(
        Reader::open(&bad_crc, MAGIC, 1).err(),
        Some(BytesError::BadChecksum)
    );
    assert_eq!(
        Reader::open(&good, *b"NOPE", 1).err(),
        Some(BytesError::WrongMagic)
    );
    assert_eq!(
        Reader::open(&good, MAGIC, 2).err(),
        Some(BytesError::UnknownVersion { found: 1 })
    );
}

#[test]
fn integers_flags_and_strings_round_trip() {
    let bytes = envelope(|w| {
        w.u8(200);
        w.bool(true);
        w.bool(false);
        w.u16(65_000);
        w.u32(4_000_000_000);
        w.u64(u64::MAX - 1);
        w.str("hello");
        w.raw(&[9, 8]);
    });
    let mut r = Reader::open(&bytes, MAGIC, 1).unwrap();
    assert_eq!(r.u8().unwrap(), 200);
    assert!(r.bool().unwrap());
    assert!(!r.bool().unwrap());
    assert_eq!(r.u16().unwrap(), 65_000);
    assert_eq!(r.u32().unwrap(), 4_000_000_000);
    assert_eq!(r.u64().unwrap(), u64::MAX - 1);
    assert_eq!(r.string(5).unwrap(), "hello");
    assert_eq!(r.take(2).unwrap(), [9, 8]);
    r.finish().unwrap();
}

#[test]
fn a_string_may_be_exactly_as_long_as_the_limit_and_no_longer() {
    let bytes = envelope(|w| {
        w.str("abcde");
        w.str("");
    });
    assert_eq!(
        Reader::open(&bytes, MAGIC, 1).unwrap().string(5).unwrap(),
        "abcde"
    );
    assert_eq!(
        Reader::open(&bytes, MAGIC, 1)
            .unwrap()
            .string(4)
            .unwrap_err(),
        BytesError::Invalid("a text is too long")
    );
    let mut r = Reader::open(&bytes, MAGIC, 1).unwrap();
    r.string(5).unwrap();
    assert_eq!(r.string(0).unwrap(), "");
}

#[test]
fn a_count_must_fit_its_limit_and_the_bytes_that_are_left() {
    // A count of 3 and six more bytes.
    let bytes = envelope(|w| {
        w.u32(3);
        w.raw(&[0; 6]);
    });
    assert_eq!(
        Reader::open(&bytes, MAGIC, 1)
            .unwrap()
            .count(10, 2)
            .unwrap(),
        3
    );
    assert_eq!(
        Reader::open(&bytes, MAGIC, 1).unwrap().count(3, 2).unwrap(),
        3
    );
    assert_eq!(
        Reader::open(&bytes, MAGIC, 1)
            .unwrap()
            .count(2, 2)
            .unwrap_err(),
        BytesError::Invalid("a count is over its limit")
    );
    // 3 elements of 3 bytes need nine bytes, six are left.
    assert_eq!(
        Reader::open(&bytes, MAGIC, 1)
            .unwrap()
            .count(10, 3)
            .unwrap_err(),
        BytesError::Truncated
    );
    // The check counts from where the reader is, not from the start.
    let later = envelope(|w| {
        w.raw(&[0; 40]);
        w.u32(2);
        w.raw(&[0; 3]);
    });
    let mut r = Reader::open(&later, MAGIC, 1).unwrap();
    r.take(40).unwrap();
    assert_eq!(r.count(10, 2).unwrap_err(), BytesError::Truncated);
}

#[test]
fn reading_past_the_end_is_truncated_and_leftovers_are_trailing() {
    let bytes = envelope(|w| w.u16(1));
    let mut r = Reader::open(&bytes, MAGIC, 1).unwrap();
    assert_eq!(r.u32().unwrap_err(), BytesError::Truncated);
    assert_eq!(r.u64().unwrap_err(), BytesError::Truncated);
    assert_eq!(r.take(usize::MAX).unwrap_err(), BytesError::Truncated);
    assert_eq!(r.u8().unwrap(), 1);
    assert_eq!(r.finish().unwrap_err(), BytesError::Trailing);
    let flag = envelope(|w| w.u8(2));
    assert_eq!(
        Reader::open(&flag, MAGIC, 1).unwrap().bool().unwrap_err(),
        BytesError::Invalid("a flag is not 0 or 1")
    );
}
