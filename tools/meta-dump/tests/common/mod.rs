//! Hand-made fixtures shared by the integration tests (built with the testkit; no game data).
#![allow(dead_code)] // each test binary uses a different subset

use std::path::PathBuf;

mod old;
mod ron;

#[allow(unused_imports)] // each test binary uses a different subset
pub use old::old_file;
#[allow(unused_imports)]
pub use ron::parse_ron;

use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};

/// The testkit's small sample: header, one reference, identifiers, types of every common shape,
/// an `ADIM`, one external object, two objects (a class and a class with a base, a pointer and a
/// container).
pub fn sample() -> Vec<u8> {
    MetaBuilder::sample().build().unwrap()
}

/// Ids of the types of [`rich`].
pub const BLOB: u32 = 6;

/// A file with every feature the dump prints: two references (one with the parameter flag), an
/// identifier, an external type and object, a raw byte array (`CStaticArray<UBYTE>`), a `NaN`, a
/// non-ASCII string, and one extra record after `EDOB`.
pub fn rich() -> Vec<u8> {
    let mut b = MetaBuilder::new();
    let model = b.file_ref("Content/Test/Models/Cube.mdl", 0);
    b.file_ref("Content/Test/Textures/Wall.tex", 0x0004_0000);
    let name = b.identifier("Blob");
    b.external_type(20, "CModelConfiguration");
    b.type_block(TypeBlock::primitive(0, "UBYTE", 1, 1));
    b.type_block(TypeBlock::primitive(1, "SLONG", 4, 4));
    b.type_block(TypeBlock::primitive(2, "FLOAT", 4, 4));
    b.type_block(TypeBlock::primitive(3, "CString", 4, 4));
    b.type_block(TypeBlock::primitive(4, "IDENT", 4, 4));
    b.type_block(TypeBlock::short(5, "CStaticArray<UBYTE>", 0, 6, 0));
    b.type_block(TypeBlock::class(
        BLOB,
        "CBlob",
        1,
        5,
        None,
        &[("1", 5), ("2", 1), ("3", 2), ("4", 3), ("5", 4), ("6", 7)],
    ));
    b.type_block(TypeBlock::pointer(7, "CBlob*", BLOB));
    b.external_object(100, model, 1, 20);
    let record = |bytes: ObjectBytes| bytes.finish();
    b.object(
        0,
        BLOB,
        record(
            ObjectBytes::new()
                .array_header(b"STAR", 5)
                .raw(&[0xde, 0xad, 0xbe, 0xef, 0x01])
                .i32(-9)
                .f32(f32::NAN)
                .cstring("caf\u{e9}")
                .u32(name)
                .object_ref(Some(100)),
        ),
    );
    b.object(
        1,
        BLOB,
        record(
            ObjectBytes::new()
                .array_header(b"STAR", 0)
                .i32(2)
                .f32(1.5)
                .cstring("")
                .u32(0)
                .object_ref(Some(0)),
        ),
    );
    b.edty_pair(50, BLOB);
    b.after_edob(
        1,
        &ObjectBytes::new()
            .u32(50)
            .u32(BLOB)
            .array_header(b"STAR", 1)
            .raw(&[7])
            .i32(3)
            .f32(0.25)
            .cstring("x")
            .u32(0)
            .object_ref(None)
            .finish(),
    );
    b.build().unwrap()
}

/// Three objects of one class; the second has a member of a type the decoder has no encoding for
/// (`CMysteryClass`) and 9 bytes of body, so objects 1 and 2 cannot be decoded. Everything from
/// the start of record 1 up to the `EDOB` tag is undecoded.
pub fn broken() -> Vec<u8> {
    let mut b = MetaBuilder::new();
    b.external_type(50, "CMysteryClass");
    b.type_block(TypeBlock::primitive(0, "SLONG", 4, 4));
    b.type_block(TypeBlock::class(1, "CGood", 1, 5, None, &[("1", 0)]));
    b.type_block(TypeBlock::class(
        2,
        "CBad",
        1,
        5,
        None,
        &[("1", 0), ("2", 50)],
    ));
    b.object(0, 1, ObjectBytes::new().i32(11).finish());
    b.object(
        1,
        2,
        ObjectBytes::new().i32(1).raw(&[1, 2, 3, 4, 5]).finish(),
    );
    b.object(2, 1, ObjectBytes::new().i32(33).finish());
    b.build().unwrap()
}

/// Bytes of the records of [`broken`] that are not decoded: the 8-byte header and 9 bytes body of
/// record 1, and the 12 bytes of record 2.
pub const BROKEN_UNDECODED: usize = (8 + 9) + (8 + 4);

pub fn golden_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
        .join(name)
}

/// Compare `actual` with the golden file (`UPDATE_GOLDEN=1` rewrites it).
pub fn check_golden(name: &str, actual: &str) {
    let path = golden_path(name);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e} (run with UPDATE_GOLDEN=1)", path.display()))
        .replace("\r\n", "\n");
    assert_eq!(
        actual, expected,
        "golden file {name} differs (UPDATE_GOLDEN=1 to rewrite)"
    );
}
