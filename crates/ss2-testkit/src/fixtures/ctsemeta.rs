//! A writer for synthetic **CTSEMETA** (format 8) files, for parser tests that must run without
//! game data.
//!
//! Written from the clean-room specs only: `docs/re/ctsemeta-container.md` (file frame),
//! `docs/re/ctsemeta-types.md` (type table) and `docs/re/ctsemeta-objects.md` (object values).
//! No game file was read to make it, and nothing it produces is a copy of one: every sample is
//! hand-built and tiny.
//!
//! The builder lays the sections out in the one order the format has, computes the `INFO` counts
//! and the `OBJS`/`OBTY` pairing itself, and refuses (in [`MetaBuilder::build`]) to write a file
//! that breaks an invariant the specs say a reader checks (duplicate type ids, a dangling
//! `fileRef`, an object of an undefined type, ...). Output is deterministic.
//!
//! ```
//! use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};
//!
//! let mut meta = MetaBuilder::new();
//! meta.type_block(TypeBlock::primitive(0, "SLONG", 4, 4));
//! meta.type_block(TypeBlock::class(1, "CCounter", 2, 5, None, &[("1", 0)]));
//! meta.object(0, 1, ObjectBytes::new().i32(42).finish());
//! let bytes = meta.build().unwrap();
//! assert_eq!(&bytes[..8], b"CTSEMETA");
//! assert!(bytes.ends_with(b"EDOB\0\0\0\0METAEND "));
//! ```

use std::collections::BTreeSet;
use std::io;

/// The 8 magic bytes every file starts with.
pub const MAGIC: &[u8; 8] = b"CTSEMETA";
/// The byte-order marker after the magic.
pub const BYTE_ORDER_MARKER: u32 = 0x1234_ABCD;
/// The only format version specified (`ctsemeta-container.md`).
pub const FORMAT_VERSION: u32 = 8;
/// The version string of the gold build, used when none is set.
pub const DEFAULT_VERSION: &str = "$Version: Serious Sam 2; 2.062.00:57284; 2005-Gold-RC5$";
/// The last 8 bytes of every file (note the trailing space).
pub const FOOTER: &[u8; 8] = b"METAEND ";
/// "None" for an object reference or a parent type.
pub const NULL_ID: u32 = 0xFFFF_FFFF;

fn invalid(msg: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, msg.into())
}

fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put_len(out: &mut Vec<u8>, len: usize) -> io::Result<()> {
    put_u32(
        out,
        u32::try_from(len).map_err(|_| invalid("length does not fit in a u32"))?,
    );
    Ok(())
}

/// A length-prefixed byte string: `u32 length`, then the bytes (no terminator).
fn put_str(out: &mut Vec<u8>, s: &str) -> io::Result<()> {
    put_len(out, s.len())?;
    out.extend_from_slice(s.as_bytes());
    Ok(())
}

/// One `DTTY` block of the type table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeBlock {
    id: u32,
    name: String,
    kind: u32,
    shape: Shape,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Shape {
    /// `kind`, then `[a, b, c]`.
    Primitive([u32; 3]),
    /// `kind`, then `[a, b]`.
    Short([u32; 2]),
    /// `kind`, then `flags`, `parent`, `STMB`, members.
    Class {
        flags: u32,
        parent: Option<u32>,
        members: Vec<(String, u32)>,
    },
}

impl TypeBlock {
    /// A primitive of `size` bytes: kind 1, words `[0, size, align]` (`FLOAT`, `SLONG`, ...).
    pub fn primitive(id: u32, name: &str, size: u32, align: u32) -> Self {
        Self {
            id,
            name: name.to_owned(),
            kind: 1,
            shape: Shape::Primitive([0, size, align]),
        }
    }

    /// An alias of another type: kind 1, words `[13, target]` (`INDEX`, `BOOL`, `COLOR`, ...).
    pub fn alias(id: u32, name: &str, target: u32) -> Self {
        Self::short(id, name, 1, 13, target)
    }

    /// An enumeration stored in `size` bytes: kind 0, words `[1, size]`.
    pub fn enumeration(id: u32, name: &str, size: u32) -> Self {
        Self::short(id, name, 0, 1, size)
    }

    /// A raw pointer `X*`: kind 0, words `[2, target]`.
    pub fn pointer(id: u32, name: &str, target: u32) -> Self {
        Self::short(id, name, 0, 2, target)
    }

    /// A fixed array: kind 0, words `[4, element]`. The length `N` lives in the *name*
    /// (`"SLONG[4]"`), as in the real files.
    pub fn fixed_array(id: u32, name: &str, element: u32) -> Self {
        Self::short(id, name, 0, 4, element)
    }

    /// `CDynamicContainer<T>`: kind 0, words `[8, element]` (encoded as `DCON` + object ids).
    pub fn dynamic_container(id: u32, name: &str, element: u32) -> Self {
        Self::short(id, name, 0, 8, element)
    }

    /// Any two-word ("short") block.
    pub fn short(id: u32, name: &str, kind: u32, a: u32, b: u32) -> Self {
        Self {
            id,
            name: name.to_owned(),
            kind,
            shape: Shape::Short([a, b]),
        }
    }

    /// A struct or class with `STMB` members `(name, typeId)`. `parent` is the base class's type
    /// id (`None` is written as `-1`). `kind` and `flags` are written as given: their meaning is
    /// still open in the specs (the real files show `flags = 5`).
    pub fn class(
        id: u32,
        name: &str,
        kind: u32,
        flags: u32,
        parent: Option<u32>,
        members: &[(&str, u32)],
    ) -> Self {
        Self {
            id,
            name: name.to_owned(),
            kind,
            shape: Shape::Class {
                flags,
                parent,
                members: members.iter().map(|(n, t)| ((*n).to_owned(), *t)).collect(),
            },
        }
    }

    /// The type id.
    pub fn id(&self) -> u32 {
        self.id
    }

    fn write(&self, out: &mut Vec<u8>) -> io::Result<()> {
        out.extend_from_slice(b"DTTY");
        put_u32(out, self.id);
        put_str(out, &self.name)?;
        put_u32(out, self.kind);
        match &self.shape {
            Shape::Primitive(words) => words.iter().for_each(|w| put_u32(out, *w)),
            Shape::Short(words) => words.iter().for_each(|w| put_u32(out, *w)),
            Shape::Class {
                flags,
                parent,
                members,
            } => {
                put_u32(out, *flags);
                put_u32(out, parent.unwrap_or(NULL_ID));
                out.extend_from_slice(b"STMB");
                put_len(out, members.len())?;
                for (name, type_id) in members {
                    put_str(out, name)?;
                    put_u32(out, *type_id);
                }
            }
        }
        Ok(())
    }
}

/// The bytes of one object's members, built value by value in the order the type table lists them
/// (base class first). No padding, no names, no sizes: see `ctsemeta-objects.md`.
#[derive(Clone, Debug, Default)]
pub struct ObjectBytes(Vec<u8>);

impl ObjectBytes {
    /// An empty object (also the encoding of a class with no members).
    pub fn new() -> Self {
        Self::default()
    }

    /// 1 raw byte (`UBYTE`, 1-byte enumerations).
    #[must_use]
    pub fn u8(mut self, v: u8) -> Self {
        self.0.push(v);
        self
    }

    /// 2 raw bytes (`UWORD`).
    #[must_use]
    pub fn u16(mut self, v: u16) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }

    /// 4 raw bytes (`ULONG`, `IDENT`, 4-byte enumerations).
    #[must_use]
    pub fn u32(mut self, v: u32) -> Self {
        put_u32(&mut self.0, v);
        self
    }

    /// 4 raw bytes, signed (`SLONG`, `INDEX`; `-1` = none).
    #[must_use]
    pub fn i32(self, v: i32) -> Self {
        self.u32(v as u32)
    }

    /// 8 raw bytes (`SQUAD`).
    #[must_use]
    pub fn u64(mut self, v: u64) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }

    /// An IEEE-754 binary32 (`FLOAT`).
    #[must_use]
    pub fn f32(self, v: f32) -> Self {
        self.u32(v.to_bits())
    }

    /// A pointer / `Ptr<X>` / `Handle<X>`: one object id, `None` = [`NULL_ID`].
    #[must_use]
    pub fn object_ref(self, id: Option<u32>) -> Self {
        self.u32(id.unwrap_or(NULL_ID))
    }

    /// A `CString`: `u32` length then the bytes.
    ///
    /// # Panics
    /// If `s` is longer than `u32::MAX` bytes.
    #[must_use]
    pub fn cstring(mut self, s: &str) -> Self {
        put_str(&mut self.0, s).expect("a fixture string fits in a u32");
        self
    }

    /// A `CTransString`: the marker `DTRS`, then the lookup key and the text as two `CString`s.
    #[must_use]
    pub fn trans_string(mut self, key: &str, text: &str) -> Self {
        self.0.extend_from_slice(b"DTRS");
        self.cstring(key).cstring(text)
    }

    /// A `CDynamicContainer`: `DCON`, the count, then one object id per element.
    #[must_use]
    pub fn dynamic_container(mut self, ids: &[u32]) -> Self {
        self.0.extend_from_slice(b"DCON");
        self = self.u32(u32::try_from(ids.len()).expect("a fixture container is small"));
        for id in ids {
            self = self.u32(*id);
        }
        self
    }

    /// The header of a `CStaticArray` (`STAR`) or `CStaticStackArray` (`SSAR`): the marker and the
    /// element count. Append the `count` elements afterwards.
    #[must_use]
    pub fn array_header(mut self, marker: &[u8; 4], count: u32) -> Self {
        self.0.extend_from_slice(marker);
        self.u32(count)
    }

    /// Raw bytes appended as they are (for shapes without a helper).
    #[must_use]
    pub fn raw(mut self, bytes: &[u8]) -> Self {
        self.0.extend_from_slice(bytes);
        self
    }

    /// The finished bytes.
    pub fn finish(self) -> Vec<u8> {
        self.0
    }
}

#[derive(Clone, Debug)]
enum TableEntry {
    Type(TypeBlock),
    Dims(Vec<u32>),
}

#[derive(Clone, Copy, Debug)]
struct ExternalObject {
    object_id: u32,
    file_ref: u32,
    flags: u32,
    type_id: u32,
}

/// Builds one CTSEMETA file. See the module docs.
#[derive(Clone, Debug)]
pub struct MetaBuilder {
    version: String,
    info0: u32,
    files: Vec<(String, u32)>,
    identifiers: Vec<String>,
    external_types: Vec<(u32, String)>,
    table: Vec<TableEntry>,
    external_objects: Vec<ExternalObject>,
    edty: Vec<(u32, u32)>,
    objects: Vec<(u32, u32, Vec<u8>)>,
    edob_word: u32,
    after_edob: Vec<u8>,
}

impl Default for MetaBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MetaBuilder {
    /// An empty, valid file: gold-build version string, no tables, no objects.
    pub fn new() -> Self {
        Self {
            version: DEFAULT_VERSION.to_owned(),
            info0: 1,
            files: Vec::new(),
            identifiers: vec![String::new()], // id 0 is always the empty string
            external_types: Vec::new(),
            table: Vec::new(),
            external_objects: Vec::new(),
            edty: Vec::new(),
            objects: Vec::new(),
            edob_word: 0,
            after_edob: Vec::new(),
        }
    }

    /// Set the `$Version: ...$` string (a loader must accept any).
    pub fn version(&mut self, version: &str) -> &mut Self {
        self.version = version.to_owned();
        self
    }

    /// Set `INFO[0]` (meaning unknown; `1` in most real files, `0` in a few).
    pub fn info0(&mut self, value: u32) -> &mut Self {
        self.info0 = value;
        self
    }

    /// Add an `RFIL` entry; returns its index (`0`, `1`, ... in order).
    pub fn file_ref(&mut self, path: &str, flags: u32) -> u32 {
        self.files.push((path.to_owned(), flags));
        (self.files.len() - 1) as u32
    }

    /// Add an `IDNT` entry; returns its id. The empty string is always id 0 and is never added
    /// twice.
    pub fn identifier(&mut self, name: &str) -> u32 {
        if name.is_empty() {
            return 0;
        }
        self.identifiers.push(name.to_owned());
        (self.identifiers.len() - 1) as u32
    }

    /// Add an `EXTY` entry: a class defined by the engine, not described in the file.
    pub fn external_type(&mut self, type_id: u32, name: &str) -> &mut Self {
        self.external_types.push((type_id, name.to_owned()));
        self
    }

    /// Add a `DTTY` block to the type table (in order).
    pub fn type_block(&mut self, block: TypeBlock) -> &mut Self {
        self.table.push(TableEntry::Type(block));
        self
    }

    /// Add an `ADIM` block (`n` array dimensions) at this point of the type table.
    pub fn array_dims(&mut self, dims: &[u32]) -> &mut Self {
        self.table.push(TableEntry::Dims(dims.to_vec()));
        self
    }

    /// Add an `EXOB` entry: object `object_id` of `type_id` lives in `RFIL` entry `file_ref`.
    pub fn external_object(
        &mut self,
        object_id: u32,
        file_ref: u32,
        flags: u32,
        type_id: u32,
    ) -> &mut Self {
        self.external_objects.push(ExternalObject {
            object_id,
            file_ref,
            flags,
            type_id,
        });
        self
    }

    /// Add an `EDTY` pair (meaning unknown).
    pub fn edty_pair(&mut self, a: u32, b: u32) -> &mut Self {
        self.edty.push((a, b));
        self
    }

    /// Add an object with an `OBTY` entry and its `OBJS` record, in order. `members` are the
    /// object's value bytes (see [`ObjectBytes`]).
    pub fn object(&mut self, object_id: u32, type_id: u32, members: Vec<u8>) -> &mut Self {
        self.objects.push((object_id, type_id, members));
        self
    }

    /// Set the `u32` after `EDOB` (0 in 9,090 real files) and any extra bytes that follow it
    /// before the footer (blob payload in 179 real files).
    pub fn after_edob(&mut self, word: u32, extra: &[u8]) -> &mut Self {
        self.edob_word = word;
        self.after_edob = extra.to_vec();
        self
    }

    /// Serialize the file.
    ///
    /// # Errors
    /// `InvalidInput` if the file would break an invariant of `ctsemeta-container.md`: a type id
    /// used twice (`EXTY` and `DTTY` share one id space), an `EXOB` `fileRef` past the end of
    /// `RFIL`, an `EXOB` or object of a type that is not defined, an object id used twice
    /// (`OBTY` and `EXOB` share one id space), or a length that does not fit a `u32`.
    pub fn build(&self) -> io::Result<Vec<u8>> {
        let types: Vec<&TypeBlock> = self
            .table
            .iter()
            .filter_map(|e| match e {
                TableEntry::Type(t) => Some(t),
                TableEntry::Dims(_) => None,
            })
            .collect();

        let mut type_ids = BTreeSet::new();
        for id in self
            .external_types
            .iter()
            .map(|(id, _)| *id)
            .chain(types.iter().map(|t| t.id))
        {
            if !type_ids.insert(id) {
                return Err(invalid(format!(
                    "type id {id} is defined twice (EXTY and DTTY share one id space)"
                )));
            }
        }
        let mut object_ids = BTreeSet::new();
        for eo in &self.external_objects {
            if eo.file_ref as usize >= self.files.len() {
                return Err(invalid(format!(
                    "EXOB object {} points at file {} but RFIL has {} entries",
                    eo.object_id,
                    eo.file_ref,
                    self.files.len()
                )));
            }
            if !type_ids.contains(&eo.type_id) {
                return Err(invalid(format!(
                    "EXOB object {} has undefined type {}",
                    eo.object_id, eo.type_id
                )));
            }
            if !object_ids.insert(eo.object_id) {
                return Err(invalid(format!("object id {} is used twice", eo.object_id)));
            }
        }
        for (id, type_id, _) in &self.objects {
            if !type_ids.contains(type_id) {
                return Err(invalid(format!("object {id} has undefined type {type_id}")));
            }
            if !object_ids.insert(*id) {
                return Err(invalid(format!("object id {id} is used twice")));
            }
        }

        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        put_u32(&mut out, BYTE_ORDER_MARKER);
        put_u32(&mut out, FORMAT_VERSION);
        put_str(&mut out, &self.version)?;

        let type_count = self.external_types.len() + types.len();
        let object_count = self.external_objects.len() + self.objects.len();
        out.extend_from_slice(b"INFO");
        for word in [
            self.info0,
            self.files.len() as u32,
            self.identifiers.len() as u32,
            u32::try_from(type_count).map_err(|_| invalid("too many types"))?,
            u32::try_from(object_count).map_err(|_| invalid("too many objects"))?,
        ] {
            put_u32(&mut out, word);
        }

        out.extend_from_slice(b"RFIL");
        put_len(&mut out, self.files.len())?;
        for (index, (path, flags)) in self.files.iter().enumerate() {
            put_u32(&mut out, index as u32);
            put_u32(&mut out, *flags);
            put_str(&mut out, path)?;
        }

        out.extend_from_slice(b"IDNT");
        put_len(&mut out, self.identifiers.len())?;
        for (id, name) in self.identifiers.iter().enumerate() {
            put_u32(&mut out, id as u32);
            put_str(&mut out, name)?;
        }

        out.extend_from_slice(b"EXTY");
        put_len(&mut out, self.external_types.len())?;
        for (id, name) in &self.external_types {
            put_u32(&mut out, *id);
            put_str(&mut out, name)?;
        }

        out.extend_from_slice(b"INTY");
        put_len(&mut out, types.len())?;
        for entry in &self.table {
            match entry {
                TableEntry::Type(t) => t.write(&mut out)?,
                TableEntry::Dims(dims) => {
                    out.extend_from_slice(b"ADIM");
                    put_len(&mut out, dims.len())?;
                    dims.iter().for_each(|d| put_u32(&mut out, *d));
                }
            }
        }

        out.extend_from_slice(b"EXOB");
        put_len(&mut out, self.external_objects.len())?;
        for eo in &self.external_objects {
            for word in [eo.object_id, eo.file_ref, eo.flags, eo.type_id] {
                put_u32(&mut out, word);
            }
        }

        out.extend_from_slice(b"OBTY");
        put_len(&mut out, self.objects.len())?;
        for (id, type_id, _) in &self.objects {
            put_u32(&mut out, *id);
            put_u32(&mut out, *type_id);
        }

        out.extend_from_slice(b"EDTY");
        put_len(&mut out, self.edty.len())?;
        for (a, b) in &self.edty {
            put_u32(&mut out, *a);
            put_u32(&mut out, *b);
        }

        out.extend_from_slice(b"OBJS");
        put_len(&mut out, self.objects.len())?;
        for (id, type_id, members) in &self.objects {
            put_u32(&mut out, *id);
            put_u32(&mut out, *type_id);
            out.extend_from_slice(members);
        }

        out.extend_from_slice(b"EDOB");
        put_u32(&mut out, self.edob_word);
        out.extend_from_slice(&self.after_edob);
        out.extend_from_slice(FOOTER);
        Ok(out)
    }

    /// A small, realistic, hand-made file for reader tests: one referenced file, two identifiers,
    /// one external type and object, a type table with a primitive, an alias, an enumeration, a
    /// fixed array (with its `ADIM`) and a class, and two objects (the second holds a reference to
    /// the first and a container). See [`SampleIds`] for the ids.
    pub fn sample() -> Self {
        let mut meta = Self::new();
        let model = meta.file_ref("Content/Test/Models/Cube.mdl", 0);
        let _ = meta.file_ref("Content/Test/Textures/Wall.tex", 0x0004_0000);
        let name = meta.identifier("TestItem");
        meta.external_type(SampleIds::EXTERNAL_MODEL, "CModelConfiguration");

        meta.type_block(TypeBlock::primitive(SampleIds::SLONG, "SLONG", 4, 4));
        meta.type_block(TypeBlock::alias(
            SampleIds::INDEX,
            "INDEX",
            SampleIds::SLONG,
        ));
        meta.type_block(TypeBlock::primitive(SampleIds::FLOAT, "FLOAT", 4, 4));
        meta.type_block(TypeBlock::primitive(SampleIds::IDENT, "IDENT", 4, 4));
        meta.type_block(TypeBlock::enumeration(SampleIds::MODE, "EMode", 1));
        meta.type_block(TypeBlock::fixed_array(
            SampleIds::FLOAT3,
            "FLOAT[3]",
            SampleIds::FLOAT,
        ));
        meta.array_dims(&[3]);
        meta.type_block(TypeBlock::class(
            SampleIds::ITEM,
            "CTestItem",
            2,
            5,
            None,
            &[
                ("1", SampleIds::INDEX),
                ("2", SampleIds::FLOAT),
                ("3", SampleIds::IDENT),
                ("4", SampleIds::MODE),
                ("5", SampleIds::FLOAT3),
            ],
        ));
        meta.type_block(TypeBlock::pointer(
            SampleIds::ITEM_PTR,
            "CTestItem*",
            SampleIds::ITEM,
        ));
        meta.type_block(TypeBlock::dynamic_container(
            SampleIds::ITEM_LIST,
            "CDynamicContainer<CTestItem>",
            SampleIds::ITEM,
        ));
        meta.type_block(TypeBlock::class(
            SampleIds::HOLDER,
            "CTestHolder",
            3,
            5,
            Some(SampleIds::ITEM),
            &[("1", SampleIds::ITEM_PTR), ("2", SampleIds::ITEM_LIST)],
        ));

        meta.external_object(SampleIds::MODEL_OBJECT, model, 1, SampleIds::EXTERNAL_MODEL);
        meta.edty_pair(0, 1);

        // Object 0: a CTestItem (INDEX 7, FLOAT 30.0, IDENT -> "TestItem", mode 2, FLOAT[3]).
        meta.object(
            0,
            SampleIds::ITEM,
            ObjectBytes::new()
                .i32(7)
                .f32(30.0)
                .u32(name)
                .u8(2)
                .f32(1.0)
                .f32(2.0)
                .f32(3.0)
                .finish(),
        );
        // Object 1: a CTestHolder: base CTestItem members first, then its own (a pointer to
        // object 0 and a container holding object 0).
        meta.object(
            1,
            SampleIds::HOLDER,
            ObjectBytes::new()
                .i32(-1)
                .f32(0.5)
                .u32(0)
                .u8(0)
                .f32(0.0)
                .f32(0.0)
                .f32(0.0)
                .object_ref(Some(0))
                .dynamic_container(&[0])
                .finish(),
        );
        meta
    }
}

/// The ids used by [`MetaBuilder::sample`].
pub struct SampleIds;

impl SampleIds {
    /// `EXTY` id of the external class `CModelConfiguration`.
    pub const EXTERNAL_MODEL: u32 = 20;
    /// `SLONG` (4-byte primitive).
    pub const SLONG: u32 = 0;
    /// `INDEX` (alias of `SLONG`).
    pub const INDEX: u32 = 1;
    /// `FLOAT`.
    pub const FLOAT: u32 = 2;
    /// `IDENT`.
    pub const IDENT: u32 = 3;
    /// `EMode`, a 1-byte enumeration.
    pub const MODE: u32 = 4;
    /// `FLOAT[3]`.
    pub const FLOAT3: u32 = 5;
    /// `CTestItem`.
    pub const ITEM: u32 = 6;
    /// `CTestItem*`.
    pub const ITEM_PTR: u32 = 7;
    /// `CDynamicContainer<CTestItem>`.
    pub const ITEM_LIST: u32 = 8;
    /// `CTestHolder` (derives from `CTestItem`).
    pub const HOLDER: u32 = 9;
    /// The external object (it lives in `Cube.mdl`).
    pub const MODEL_OBJECT: u32 = 100;
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    // ----- an independent walker, written from the specs, to check what the writer produced ----

    struct Cursor<'a> {
        bytes: &'a [u8],
        pos: usize,
    }

    impl<'a> Cursor<'a> {
        fn new(bytes: &'a [u8]) -> Self {
            Self { bytes, pos: 0 }
        }

        fn take(&mut self, n: usize) -> &'a [u8] {
            assert!(
                n <= self.bytes.len() - self.pos,
                "read past the end at {}",
                self.pos
            );
            let s = &self.bytes[self.pos..self.pos + n];
            self.pos += n;
            s
        }

        fn u32(&mut self) -> u32 {
            u32::from_le_bytes(self.take(4).try_into().unwrap())
        }

        fn tag(&mut self) -> [u8; 4] {
            self.take(4).try_into().unwrap()
        }

        fn expect(&mut self, tag: &[u8; 4]) {
            let got = self.tag();
            assert_eq!(
                &got,
                tag,
                "expected {} at {}",
                String::from_utf8_lossy(tag),
                self.pos - 4
            );
        }

        fn string(&mut self) -> String {
            let n = self.u32() as usize;
            String::from_utf8(self.take(n).to_vec()).unwrap()
        }

        fn peek_is(&self, tag: &[u8; 4]) -> bool {
            self.bytes.get(self.pos..self.pos + 4) == Some(tag.as_slice())
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    enum Def {
        Primitive {
            size: u32,
        },
        Alias {
            target: u32,
        },
        Enum {
            size: u32,
        },
        Pointer,
        FixedArray {
            element: u32,
            len: usize,
        },
        Container {
            element: u32,
        },
        Class {
            parent: Option<u32>,
            members: Vec<u32>,
        },
    }

    #[derive(Debug)]
    struct Walked {
        version: String,
        info: [u32; 5],
        files: Vec<(u32, u32, String)>,
        identifiers: Vec<(u32, String)>,
        external_types: Vec<(u32, String)>,
        defs: BTreeMap<u32, (String, Def)>,
        dims: Vec<Vec<u32>>,
        external_objects: Vec<[u32; 4]>,
        obty: Vec<(u32, u32)>,
        edty: Vec<(u32, u32)>,
        objects: Vec<(u32, u32, Vec<u8>)>,
        after_edob: (u32, Vec<u8>),
    }

    /// Decode the bytes of one value of `type_id`, advancing `c`; returns the raw bytes consumed.
    fn value_len(w: &Walked, c: &mut Cursor<'_>, type_id: u32) {
        let (name, def) = w
            .defs
            .get(&type_id)
            .unwrap_or_else(|| panic!("type {type_id} is not in the table"));
        match def {
            Def::Primitive { size } | Def::Enum { size } => {
                c.take(*size as usize);
            }
            Def::Alias { target } => value_len(w, c, *target),
            Def::Pointer => {
                c.u32();
            }
            Def::FixedArray { element, len } => {
                for _ in 0..*len {
                    value_len(w, c, *element);
                }
            }
            Def::Container { element } => {
                c.expect(b"DCON");
                let n = c.u32();
                for _ in 0..n {
                    c.u32(); // object ids
                }
                let _ = element;
            }
            Def::Class { parent, members } => {
                let mut chain = vec![type_id];
                let mut next = *parent;
                while let Some(p) = next {
                    chain.push(p);
                    next = match &w.defs[&p].1 {
                        Def::Class { parent, .. } => *parent,
                        other => panic!("{name}: parent {p} is not a class: {other:?}"),
                    };
                }
                let _ = members;
                for t in chain.iter().rev() {
                    if let Def::Class { members, .. } = &w.defs[t].1 {
                        for m in members {
                            value_len(w, c, *m);
                        }
                    }
                }
            }
        }
    }

    fn walk(bytes: &[u8]) -> Walked {
        let mut c = Cursor::new(bytes);
        assert_eq!(c.take(8), MAGIC);
        assert_eq!(c.u32(), BYTE_ORDER_MARKER);
        assert_eq!(c.u32(), FORMAT_VERSION);
        let version = c.string();
        c.expect(b"INFO");
        let info = [c.u32(), c.u32(), c.u32(), c.u32(), c.u32()];

        c.expect(b"RFIL");
        let n = c.u32();
        let files = (0..n)
            .map(|_| (c.u32(), c.u32(), c.string()))
            .collect::<Vec<_>>();
        c.expect(b"IDNT");
        let n = c.u32();
        let identifiers = (0..n).map(|_| (c.u32(), c.string())).collect::<Vec<_>>();
        c.expect(b"EXTY");
        let n = c.u32();
        let external_types = (0..n).map(|_| (c.u32(), c.string())).collect::<Vec<_>>();

        c.expect(b"INTY");
        let inty = c.u32();
        let mut defs = BTreeMap::new();
        let mut dims = Vec::new();
        let mut blocks = 0;
        while !c.peek_is(b"EXOB") {
            if c.peek_is(b"ADIM") {
                c.expect(b"ADIM");
                let n = c.u32();
                dims.push((0..n).map(|_| c.u32()).collect());
                continue;
            }
            c.expect(b"DTTY");
            blocks += 1;
            let id = c.u32();
            let name = c.string();
            let kind = c.u32();
            // Shape rule of ctsemeta-types.md: `STMB` 12 bytes after the start of `kind` (that is,
            // 8 bytes after the end of the `kind` word, past `flags` and `parent`) means
            // struct/class.
            let after = &bytes[c.pos..];
            let def = if after.len() >= 12 && &after[8..12] == b"STMB" {
                let _flags = c.u32();
                let parent = c.u32();
                c.expect(b"STMB");
                let count = c.u32();
                let members = (0..count)
                    .map(|_| {
                        let _n = c.string();
                        c.u32()
                    })
                    .collect();
                Def::Class {
                    parent: (parent != NULL_ID).then_some(parent),
                    members,
                }
            } else if kind == 1 && u32::from_le_bytes(after[0..4].try_into().unwrap()) == 0 {
                let (_zero, size, _align) = (c.u32(), c.u32(), c.u32());
                Def::Primitive { size }
            } else {
                let (a, b) = (c.u32(), c.u32());
                match (kind, a) {
                    (1, 13) => Def::Alias { target: b },
                    (0, 1) => Def::Enum { size: b },
                    (0, 2) => Def::Pointer,
                    (0, 4) => {
                        let len = name
                            .rsplit_once('[')
                            .and_then(|(_, n)| n.trim_end_matches(']').parse().ok())
                            .expect("fixed array length lives in the name");
                        Def::FixedArray { element: b, len }
                    }
                    (0, 8) => Def::Container { element: b },
                    other => panic!("unknown short shape {other:?} for {name}"),
                }
            };
            defs.insert(id, (name, def));
        }
        assert_eq!(blocks, inty, "the number of DTTY blocks equals INTY");

        c.expect(b"EXOB");
        let n = c.u32();
        let external_objects = (0..n)
            .map(|_| [c.u32(), c.u32(), c.u32(), c.u32()])
            .collect::<Vec<_>>();
        c.expect(b"OBTY");
        let n = c.u32();
        let obty = (0..n).map(|_| (c.u32(), c.u32())).collect::<Vec<_>>();
        c.expect(b"EDTY");
        let n = c.u32();
        let edty = (0..n).map(|_| (c.u32(), c.u32())).collect::<Vec<_>>();
        c.expect(b"OBJS");
        let n = c.u32();
        assert_eq!(n as usize, obty.len(), "OBJS count equals OBTY count");

        let mut w = Walked {
            version,
            info,
            files,
            identifiers,
            external_types,
            defs,
            dims,
            external_objects,
            obty: obty.clone(),
            edty,
            objects: Vec::new(),
            after_edob: (0, Vec::new()),
        };
        for (i, (want_id, want_type)) in obty.iter().enumerate() {
            let start = c.pos;
            let (id, type_id) = (c.u32(), c.u32());
            assert_eq!(
                (id, type_id),
                (*want_id, *want_type),
                "record {i} matches its OBTY entry"
            );
            value_len(&w, &mut c, type_id);
            w.objects
                .push((id, type_id, bytes[start + 8..c.pos].to_vec()));
        }
        c.expect(b"EDOB");
        let word = c.u32();
        let extra = bytes[c.pos..bytes.len() - 8].to_vec();
        c.take(extra.len());
        assert_eq!(c.take(8), FOOTER, "the file ends with METAEND");
        assert_eq!(c.pos, bytes.len());
        w.after_edob = (word, extra);
        w
    }

    // ----- tests -------------------------------------------------------------------------------

    #[test]
    fn empty_file_is_the_minimal_valid_frame() {
        let bytes = MetaBuilder::new().build().unwrap();
        let w = walk(&bytes);
        assert_eq!(w.version, DEFAULT_VERSION);
        assert_eq!(w.info, [1, 0, 1, 0, 0]);
        assert_eq!(
            w.identifiers,
            [(0, String::new())],
            "id 0 is the empty string"
        );
        assert!(w.files.is_empty() && w.objects.is_empty() && w.defs.is_empty());
        assert!(bytes.ends_with(b"EDOB\0\0\0\0METAEND "));
    }

    #[test]
    fn header_matches_the_spec_test_vector() {
        // docs/re/ctsemeta-container.md, "Test vectors": the start of a real header.
        let bytes = MetaBuilder::new().build().unwrap();
        let mut expected = Vec::new();
        expected.extend_from_slice(b"CTSEMETA");
        expected.extend_from_slice(&[0xcd, 0xab, 0x34, 0x12]);
        expected.extend_from_slice(&[0x08, 0, 0, 0]);
        expected.extend_from_slice(&[0x37, 0, 0, 0]);
        expected.extend_from_slice(b"$Version: Serious Sam 2; 2.062.00:57284; 2005-Gold-RC5$");
        expected.extend_from_slice(b"INFO");
        assert_eq!(&bytes[..expected.len()], expected.as_slice());
        assert_eq!(DEFAULT_VERSION.len(), 55, "the spec's L = 0x37");
    }

    #[test]
    fn the_sample_walks_cleanly_to_the_footer_and_info_is_consistent() {
        let bytes = MetaBuilder::sample().build().unwrap();
        let w = walk(&bytes);
        assert_eq!(w.info[1] as usize, w.files.len(), "INFO[1] = RFIL count");
        assert_eq!(
            w.info[2] as usize,
            w.identifiers.len(),
            "INFO[2] = IDNT count"
        );
        assert_eq!(
            w.info[3] as usize,
            w.external_types.len() + w.defs.len(),
            "INFO[3] = EXTY + INTY"
        );
        assert_eq!(
            w.info[4] as usize,
            w.external_objects.len() + w.objects.len(),
            "INFO[4] = EXOB + OBTY"
        );
        assert_eq!(
            w.files[1],
            (1, 0x0004_0000, "Content/Test/Textures/Wall.tex".to_owned())
        );
        assert_eq!(
            w.identifiers,
            [(0, String::new()), (1, "TestItem".to_owned())]
        );
        assert_eq!(w.external_types, [(20, "CModelConfiguration".to_owned())]);
        assert_eq!(
            w.dims,
            [vec![3]],
            "the ADIM block is interleaved in the table"
        );
        assert_eq!(w.external_objects, [[100, 0, 1, 20]]);
        assert_eq!(w.edty, [(0, 1)]);
        assert_eq!(w.after_edob, (0, Vec::new()));
        assert_eq!(w.obty, [(0, 6), (1, 9)]);
    }

    #[test]
    fn objects_decode_byte_for_byte_through_the_type_table() {
        let w = walk(&MetaBuilder::sample().build().unwrap());
        // CTestItem: INDEX(4) + FLOAT(4) + IDENT(4) + EMode(1) + FLOAT[3](12) = 25 bytes.
        let (_, _, item) = &w.objects[0];
        assert_eq!(item.len(), 25);
        assert_eq!(&item[0..4], &7_i32.to_le_bytes());
        assert_eq!(&item[4..8], &30.0_f32.to_le_bytes());
        assert_eq!(&item[8..12], &1_u32.to_le_bytes(), "IDENT -> IDNT id 1");
        assert_eq!(item[12], 2);
        // CTestHolder: the base class members first (25), then pointer(4) + DCON(4+4+4) = 41.
        let (_, _, holder) = &w.objects[1];
        assert_eq!(holder.len(), 25 + 4 + 12);
        assert_eq!(&holder[25..29], &0_u32.to_le_bytes(), "pointer to object 0");
        assert_eq!(&holder[29..33], b"DCON");
        assert_eq!(&holder[33..37], &1_u32.to_le_bytes());
        assert_eq!(&holder[37..41], &0_u32.to_le_bytes());
    }

    #[test]
    fn output_is_deterministic() {
        assert_eq!(
            MetaBuilder::sample().build().unwrap(),
            MetaBuilder::sample().build().unwrap()
        );
    }

    #[test]
    fn identifiers_are_sequential_and_the_empty_string_is_id_zero() {
        let mut m = MetaBuilder::new();
        assert_eq!(m.identifier(""), 0);
        assert_eq!(m.identifier("a"), 1);
        assert_eq!(m.identifier(""), 0, "never added twice");
        assert_eq!(m.identifier("b"), 2);
        assert_eq!(m.file_ref("x", 0), 0);
        assert_eq!(m.file_ref("y", 0), 1);
        let w = walk(&m.build().unwrap());
        assert_eq!(
            w.identifiers.iter().map(|(i, _)| *i).collect::<Vec<_>>(),
            [0, 1, 2]
        );
        assert_eq!(
            w.files.iter().map(|(i, _, _)| *i).collect::<Vec<_>>(),
            [0, 1]
        );
    }

    #[test]
    fn every_value_helper_has_the_documented_width() {
        let b = |o: ObjectBytes| o.finish();
        assert_eq!(b(ObjectBytes::new().u8(1)), [1]);
        assert_eq!(b(ObjectBytes::new().u16(0x0201)), [1, 2]);
        assert_eq!(b(ObjectBytes::new().u32(1)), [1, 0, 0, 0]);
        assert_eq!(b(ObjectBytes::new().i32(-1)), [0xff; 4]);
        assert_eq!(b(ObjectBytes::new().u64(1)).len(), 8);
        assert_eq!(b(ObjectBytes::new().f32(1.0)), 1.0_f32.to_le_bytes());
        assert_eq!(b(ObjectBytes::new().object_ref(None)), [0xff; 4]);
        assert_eq!(b(ObjectBytes::new().object_ref(Some(5))), [5, 0, 0, 0]);
        assert_eq!(
            b(ObjectBytes::new().cstring("ab")),
            [2, 0, 0, 0, b'a', b'b']
        );
        assert_eq!(
            b(ObjectBytes::new().cstring("")),
            [0, 0, 0, 0],
            "empty string is length 0"
        );
        assert_eq!(
            b(ObjectBytes::new().trans_string("K", "")),
            [b'D', b'T', b'R', b'S', 1, 0, 0, 0, b'K', 0, 0, 0, 0],
            "DTRS marker, key, empty text"
        );
        assert_eq!(
            b(ObjectBytes::new().dynamic_container(&[])),
            [b'D', b'C', b'O', b'N', 0, 0, 0, 0]
        );
        assert_eq!(
            b(ObjectBytes::new().array_header(b"STAR", 2)),
            [b'S', b'T', b'A', b'R', 2, 0, 0, 0]
        );
        assert_eq!(b(ObjectBytes::new().raw(&[9, 9])), [9, 9]);
    }

    #[test]
    fn a_class_with_no_members_takes_no_bytes() {
        let mut m = MetaBuilder::new();
        m.type_block(TypeBlock::class(0, "CObject", 1, 5, None, &[]));
        m.object(0, 0, Vec::new());
        let w = walk(&m.build().unwrap());
        assert_eq!(w.objects, [(0, 0, Vec::new())]);
    }

    #[test]
    fn after_edob_data_and_a_nonzero_word_are_written_before_the_footer() {
        let mut m = MetaBuilder::new();
        m.after_edob(2, &[1, 2, 3, 4, 5]);
        let bytes = m.build().unwrap();
        assert!(bytes.ends_with(b"EDOB\x02\0\0\0\x01\x02\x03\x04\x05METAEND "));
        assert_eq!(walk(&bytes).after_edob, (2, vec![1, 2, 3, 4, 5]));
    }

    #[test]
    fn a_custom_version_string_is_length_prefixed() {
        let mut m = MetaBuilder::new();
        m.version("$Version: Serious Sam 2; 2.100.00:1; 2021-PATCH-2-090-inc$")
            .info0(0);
        let w = walk(&m.build().unwrap());
        assert_eq!(
            w.version,
            "$Version: Serious Sam 2; 2.100.00:1; 2021-PATCH-2-090-inc$"
        );
        assert_eq!(w.info[0], 0);
    }

    #[test]
    fn invariant_violations_are_refused_not_written() {
        let err = |m: &MetaBuilder| m.build().unwrap_err().to_string();

        let mut m = MetaBuilder::new();
        m.external_type(3, "A")
            .type_block(TypeBlock::primitive(3, "B", 4, 4));
        assert!(
            err(&m).contains("type id 3 is defined twice"),
            "EXTY and DTTY share ids"
        );

        let mut m = MetaBuilder::new();
        m.type_block(TypeBlock::primitive(1, "A", 4, 4))
            .type_block(TypeBlock::primitive(1, "B", 4, 4));
        assert!(err(&m).contains("defined twice"));

        let mut m = MetaBuilder::new();
        m.external_type(1, "A");
        m.external_object(5, 0, 1, 1);
        assert!(err(&m).contains("RFIL has 0 entries"), "dangling fileRef");

        let mut m = MetaBuilder::new();
        m.file_ref("f", 0);
        m.external_object(5, 0, 1, 99);
        assert!(err(&m).contains("undefined type 99"));

        let mut m = MetaBuilder::new();
        m.object(0, 7, Vec::new());
        assert!(err(&m).contains("undefined type 7"));

        let mut m = MetaBuilder::new();
        m.file_ref("f", 0);
        m.type_block(TypeBlock::primitive(1, "A", 4, 4));
        m.external_object(2, 0, 1, 1);
        m.object(2, 1, vec![0; 4]);
        assert!(
            err(&m).contains("object id 2 is used twice"),
            "OBTY and EXOB share ids"
        );
    }

    #[test]
    fn the_sample_is_small_enough_to_commit_if_ever_needed() {
        let len = MetaBuilder::sample().build().unwrap().len();
        assert!(
            len < 2_000,
            "{len} bytes: the testdata policy allows 256 KiB, fixtures should be tiny"
        );
    }
}
