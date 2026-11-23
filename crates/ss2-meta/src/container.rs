//! The CTSEMETA container frame (roadmap T02-006).
//!
//! Layout (versions 5 to 8; see the module docs of the crate for the specs):
//!
//! ```text
//! header: "CTSEMETA" · 0x1234ABCD · format version · $Version string
//! INFO  4 words (versions 4-7) or 5 words (version 8)
//! RFIL  count · (index, flags, path)*      IDNT  count · (id, name)*
//! EXTY  count · (type id, name)*           INTY  n · n DTTY blocks (ADIM blocks interleaved)
//! EXOB  count · 16-byte entries            OBTY  count · 8-byte entries
//! EDTY  count · 8-byte entries             OBJS  count · object records
//! EDOB  count · extra records              "METAEND "
//! ```

use std::borrow::Cow;
use std::collections::HashMap;
use std::ops::Range;

use ss2_core::{Error, Result};

/// `RFIL.flags` bit 18: set on references made by the data-driven parameter layer (weapon, player,
/// HUD and character databases). A reader may ignore it (`docs/re/ctsemeta-leftovers.md`).
pub const FLAG_PARAMETER_REFERENCE: u32 = 0x0004_0000;

const MAGIC: &[u8; 8] = b"CTSEMETA";
const BYTE_ORDER_MARKER: u32 = 0x1234_ABCD;
const FOOTER: &[u8; 8] = b"METAEND ";
const NO_EXTRA_TAIL: &[u8; 16] = b"EDOB\0\0\0\0METAEND ";
const NULL_ID: u32 = 0xFFFF_FFFF;
const WHAT: &str = "CTSEMETA container";

/// Latin-1 text: borrowed when it is plain ASCII (every name in the install), converted otherwise.
fn text(bytes: &[u8]) -> Cow<'_, str> {
    match std::str::from_utf8(bytes) {
        Ok(s) if bytes.is_ascii() => Cow::Borrowed(s),
        _ => Cow::Owned(bytes.iter().map(|&b| char::from(b)).collect()),
    }
}

/// The build that wrote a file, parsed from the `$Version: …$` string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Build {
    /// The product (`Serious Sam 2`).
    pub product: String,
    /// `major.minor.revision` (`2.062.00`).
    pub release: String,
    /// The build number after the colon (`57284`).
    pub build: u32,
    /// The last field (`2005-Gold-RC5`, `2004-05-dev`, …).
    pub tag: String,
}

impl Build {
    /// Parse `$Version: Serious Sam 2; 2.062.00:57284; 2005-Gold-RC5$`; `None` if the text does
    /// not have that shape (a loader must accept any version string, so this is only a helper).
    pub fn parse(version: &str) -> Option<Self> {
        let inner = version
            .trim()
            .strip_prefix("$Version:")?
            .strip_suffix('$')?;
        let mut parts = inner.split(';').map(str::trim);
        let product = parts.next()?;
        let (release, build) = parts.next()?.split_once(':')?;
        let tag = parts.next()?;
        Some(Self {
            product: product.to_string(),
            release: release.to_string(),
            build: build.parse().ok()?,
            tag: tag.to_string(),
        })
    }
}

/// The file header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Header<'a> {
    /// Format version, 4 to 8 (8 in all but 155 old `.tex` files).
    pub format_version: u32,
    /// The raw `$Version: …$` string.
    pub version_string: Cow<'a, str>,
    /// The parsed build, if the string has the usual shape.
    pub build: Option<Build>,
}

/// The `INFO` words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Info {
    /// `INFO[0]`: meaning unknown (1 in most files; 0 in every file with extra records);
    /// `None` in versions 4 to 7, which have no such word. A reader ignores it.
    pub flag: Option<u32>,
    /// Number of `RFIL` entries.
    pub file_count: u32,
    /// Number of `IDNT` entries.
    pub ident_count: u32,
    /// Total number of types (`EXTY` plus `INTY`).
    pub type_count: u32,
    /// Total number of objects (`EXOB` plus `OBTY`).
    pub object_count: u32,
}

/// One `RFIL` entry: a file the container refers to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileRef<'a> {
    /// Position in the table (always `0..n` in order).
    pub index: u32,
    /// `0` or [`FLAG_PARAMETER_REFERENCE`].
    pub flags: u32,
    /// Engine-relative path with `/` separators; resolve it case-insensitively.
    pub path: Cow<'a, str>,
}

/// One `EXTY` entry: a class defined by the engine or the game, not described in the file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalType<'a> {
    /// Type id (one id space with the `DTTY` blocks).
    pub id: u32,
    /// Class name.
    pub name: Cow<'a, str>,
}

/// A struct member: a name (often a decimal number) and the id of its type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Member<'a> {
    /// Member name.
    pub name: Cow<'a, str>,
    /// Type id.
    pub type_id: u32,
    /// The member number written in front of the name by format version 4 only.
    pub number: Option<u32>,
}

/// The shape of a `DTTY` block (`docs/re/ctsemeta-types.md`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeShape<'a> {
    /// A struct or class: layout version in [`TypeDef::kind`], `flags` (always 5), the base class
    /// (`None` for none) and the members in file order.
    Struct {
        /// The `flags` word.
        flags: u32,
        /// Type id of the base class.
        parent: Option<u32>,
        /// The members.
        members: Vec<Member<'a>>,
    },
    /// A block without members: two words (a constructed type or an alias) or three (a primitive
    /// `0, size, align`). See [`TypeDef::short_form`].
    Short {
        /// The words after `kind`.
        words: Vec<u32>,
    },
}

/// What a short block is, by `(kind, first word)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShortForm {
    /// An enumeration stored in `size` bytes.
    Enumeration {
        /// Size in bytes.
        size: u32,
    },
    /// A raw pointer.
    Pointer {
        /// Pointee type id.
        target: u32,
    },
    /// A fixed array; the length is in the type name (`n[4]`).
    FixedArray {
        /// Element type id.
        element: u32,
    },
    /// `CStaticArray<T>`.
    StaticArray {
        /// Element type id.
        element: u32,
    },
    /// `CStaticStackArray<T>`.
    StaticStackArray {
        /// Element type id.
        element: u32,
    },
    /// `CDynamicContainer<T>`.
    DynamicContainer {
        /// Element type id.
        element: u32,
    },
    /// `Ptr<T>`.
    SmartPointer {
        /// Pointee type id.
        target: u32,
    },
    /// `Handle<T>`.
    Handle {
        /// Pointee type id.
        target: u32,
    },
    /// A primitive of `size` bytes with alignment `align`.
    Primitive {
        /// Size in bytes.
        size: u32,
        /// Alignment in bytes.
        align: u32,
    },
    /// An alias of another type.
    Alias {
        /// Target type id.
        target: u32,
    },
}

/// One `DTTY` block of the type table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeDef<'a> {
    /// Type id (one id space with `EXTY`).
    pub id: u32,
    /// Type name (`CResourceFile`, `SLONG`, `CDynamicContainer<CResource>`, `FLOAT[3]`).
    pub name: Cow<'a, str>,
    /// For a struct: the version number of the class layout; for a short block: `0` for a
    /// constructed type and `1` for a primitive or alias (`docs/re/dtty-kinds.md`).
    pub kind: u32,
    /// The rest of the block.
    pub shape: TypeShape<'a>,
}

impl TypeDef<'_> {
    /// Classify a short block; `None` for a struct, or for a combination that is not one of the ten
    /// forms of the install.
    pub fn short_form(&self) -> Option<ShortForm> {
        let TypeShape::Short { words } = &self.shape else {
            return None;
        };
        Some(match (self.kind, words.as_slice()) {
            (0, [1, size]) => ShortForm::Enumeration { size: *size },
            (0, [2, target]) => ShortForm::Pointer { target: *target },
            (0, [4, element]) => ShortForm::FixedArray { element: *element },
            (0, [6, element]) => ShortForm::StaticArray { element: *element },
            (0, [7, element]) => ShortForm::StaticStackArray { element: *element },
            (0, [8, element]) => ShortForm::DynamicContainer { element: *element },
            (0, [11, target]) => ShortForm::SmartPointer { target: *target },
            (0, [12, target]) => ShortForm::Handle { target: *target },
            (1, [0, size, align]) => ShortForm::Primitive {
                size: *size,
                align: *align,
            },
            (1, [13, target]) => ShortForm::Alias { target: *target },
            _ => return None,
        })
    }
}

/// An `ADIM` block of the type table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArrayDims {
    /// How many `DTTY` blocks precede it.
    pub after_type: usize,
    /// The dimensions.
    pub dims: Vec<u32>,
}

/// One `EXOB` entry: object `object_id` of type `type_id` lives in file `file_ref` of `RFIL`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExternalObject {
    /// Object id (one id space with `OBTY`).
    pub object_id: u32,
    /// Index into [`Container::files`] (checked).
    pub file_ref: u32,
    /// `1` for a resource, a shader class code (1000 to 1070) or an entity-class code
    /// (`docs/re/ctsemeta-leftovers.md`).
    pub flags: u32,
    /// Type id.
    pub type_id: u32,
}

/// One `OBTY` or `EDTY` entry: an object id and its type id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectEntry {
    /// Object id.
    pub object_id: u32,
    /// Type id.
    pub type_id: u32,
}

/// A section of the file: its tag and byte range (tag included).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    /// The four tag bytes (`INFO`, `RFIL`, `IDNT`, `EXTY`, `INTY`, `EXOB`, `OBTY`, `EDTY`, `OBJS`,
    /// and `EDOB` when it could be located).
    pub tag: [u8; 4],
    /// Bytes of the section in the file.
    pub range: Range<usize>,
}

/// A parsed CTSEMETA container; borrows the file bytes.
#[derive(Clone, Debug)]
pub struct Container<'a> {
    /// The whole file.
    pub data: &'a [u8],
    /// Header.
    pub header: Header<'a>,
    /// `INFO` words.
    pub info: Info,
    /// `RFIL` entries.
    pub files: Vec<FileRef<'a>>,
    /// `IDNT` strings; entry `i` has id `i`.
    pub idents: Vec<Cow<'a, str>>,
    /// `EXTY` entries.
    pub external_types: Vec<ExternalType<'a>>,
    /// `DTTY` blocks in file order.
    pub types: Vec<TypeDef<'a>>,
    /// `ADIM` blocks in file order.
    pub array_dims: Vec<ArrayDims>,
    /// `EXOB` entries.
    pub external_objects: Vec<ExternalObject>,
    /// `OBTY` entries: the objects that have a record in `OBJS`, in record order.
    pub object_table: Vec<ObjectEntry>,
    /// `EDTY` entries: id and type of each extra record after `EDOB`, in record order.
    pub extra_table: Vec<ObjectEntry>,
    /// The record count written after `OBJS`.
    pub object_record_count: u32,
    /// The bytes of the object records. When [`Container::has_extra_records`] is `false` this ends
    /// just before the `EDOB` tag; otherwise it runs to the footer and **includes** `EDOB`, its
    /// count and the extra records, because telling them apart needs the object decoder (T02-007).
    pub object_data: Range<usize>,
    /// `true` unless the file ends in exactly `EDOB`, `0`, `METAEND `.
    pub has_extra_records: bool,
    /// Every section with its byte range, in file order (the "chunk list").
    pub sections: Vec<Section>,
    type_index: HashMap<u32, usize>,
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn fail<T>(&self, detail: impl std::fmt::Display) -> Result<T> {
        Err(Error::invalid(
            WHAT,
            format!("at byte {}: {detail}", self.pos),
        ))
    }

    fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        if n > self.remaining() {
            return self.fail(format!(
                "{n} bytes wanted, {} left (truncated file)",
                self.remaining()
            ));
        }
        let out = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(out)
    }

    fn u32(&mut self) -> Result<u32> {
        let b = self.bytes(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn peek_tag(&self) -> Option<[u8; 4]> {
        let b = self.data.get(self.pos..self.pos + 4)?;
        Some([b[0], b[1], b[2], b[3]])
    }

    fn tag(&mut self, expected: &[u8; 4]) -> Result<usize> {
        let start = self.pos;
        let got = self.bytes(4)?;
        if got != expected {
            self.pos = start;
            return self.fail(format!(
                "expected section `{}`, found {got:02x?}",
                String::from_utf8_lossy(expected)
            ));
        }
        Ok(start)
    }

    /// A length-prefixed string.
    fn string(&mut self) -> Result<Cow<'a, str>> {
        let len = self.u32()? as usize;
        Ok(text(self.bytes(len)?))
    }

    /// An entry count, checked against the bytes left so a hostile count cannot allocate.
    fn count(&mut self, min_entry: usize) -> Result<usize> {
        let n = self.u32()? as usize;
        if n.checked_mul(min_entry)
            .is_none_or(|b| b > self.remaining())
        {
            return self.fail(format!(
                "count {n} of {min_entry}-byte entries does not fit in the {} bytes left",
                self.remaining()
            ));
        }
        Ok(n)
    }
}

impl<'a> Container<'a> {
    /// Parse the frame of a CTSEMETA file.
    ///
    /// Checks: magic and byte-order marker; format version 4 to 8; sections in the one order the
    /// format has; every count within the bytes left; `RFIL` indices and `IDNT` ids sequential;
    /// the `INTY` count equals the number of `DTTY` blocks; every `EXOB` `fileRef` is a valid
    /// `RFIL` index; the file ends with `METAEND `. A short block must be one of the ten known
    /// forms. `INFO` counts other than the first two are *not* enforced (see
    /// [`Container::info_mismatches`]).
    ///
    /// # Errors
    /// [`Error::Invalid`] naming the byte offset and the problem.
    pub fn parse(data: &'a [u8]) -> Result<Self> {
        let mut r = Reader { data, pos: 0 };
        if r.bytes(8)? != MAGIC {
            r.pos = 0;
            return r.fail("not a CTSEMETA file (bad magic)");
        }
        if r.u32()? != BYTE_ORDER_MARKER {
            r.pos = 8;
            return r.fail("bad byte-order marker");
        }
        let format_version = r.u32()?;
        if !(4..=8).contains(&format_version) {
            r.pos = 12;
            return r.fail(format!(
                "unsupported format version {format_version} (4 to 8 are known)"
            ));
        }
        let version_string = r.string()?;
        let header = Header {
            format_version,
            build: Build::parse(&version_string),
            version_string,
        };
        let mut sections = Vec::new();

        let start = r.tag(b"INFO")?;
        let words = if format_version == 8 { 5 } else { 4 };
        let mut w = [0u32; 5];
        for slot in w.iter_mut().take(words) {
            *slot = r.u32()?;
        }
        // The four counts are the last four words in both layouts; version 8 has the flag first.
        let counts = if words == 5 { &w[1..5] } else { &w[0..4] };
        let info = Info {
            flag: (words == 5).then_some(w[0]),
            file_count: counts[0],
            ident_count: counts[1],
            type_count: counts[2],
            object_count: counts[3],
        };
        sections.push(Section {
            tag: *b"INFO",
            range: start..r.pos,
        });

        let start = r.tag(b"RFIL")?;
        let n = r.count(12)?;
        let mut files = Vec::with_capacity(n);
        for i in 0..n {
            let index = r.u32()?;
            if index as usize != i {
                return r.fail(format!("RFIL entry {i} has index {index}"));
            }
            let flags = r.u32()?;
            let path = r.string()?;
            files.push(FileRef { index, flags, path });
        }
        sections.push(Section {
            tag: *b"RFIL",
            range: start..r.pos,
        });

        let start = r.tag(b"IDNT")?;
        let n = r.count(8)?;
        let mut idents = Vec::with_capacity(n);
        for i in 0..n {
            let id = r.u32()?;
            if id as usize != i {
                return r.fail(format!("IDNT entry {i} has id {id}"));
            }
            idents.push(r.string()?);
        }
        sections.push(Section {
            tag: *b"IDNT",
            range: start..r.pos,
        });

        let start = r.tag(b"EXTY")?;
        let n = r.count(8)?;
        let mut external_types = Vec::with_capacity(n);
        for _ in 0..n {
            let id = r.u32()?;
            external_types.push(ExternalType {
                id,
                name: r.string()?,
            });
        }
        sections.push(Section {
            tag: *b"EXTY",
            range: start..r.pos,
        });

        let start = r.tag(b"INTY")?;
        let declared = r.count(12)?;
        let mut types = Vec::with_capacity(declared);
        let mut array_dims = Vec::new();
        loop {
            match r.peek_tag() {
                Some(t) if &t == b"DTTY" => types.push(Self::type_block(&mut r, format_version)?),
                Some(t) if &t == b"ADIM" => {
                    r.pos += 4;
                    let n = r.count(4)?;
                    let mut dims = Vec::with_capacity(n);
                    for _ in 0..n {
                        dims.push(r.u32()?);
                    }
                    array_dims.push(ArrayDims {
                        after_type: types.len(),
                        dims,
                    });
                }
                _ => break,
            }
        }
        if types.len() != declared {
            return r.fail(format!(
                "INTY says {declared} type blocks, the table has {}",
                types.len()
            ));
        }
        sections.push(Section {
            tag: *b"INTY",
            range: start..r.pos,
        });
        let mut type_index = HashMap::with_capacity(types.len());
        for (i, t) in types.iter().enumerate() {
            if type_index.insert(t.id, i).is_some() {
                return r.fail(format!("type id {} is defined twice", t.id));
            }
        }

        let start = r.tag(b"EXOB")?;
        let n = r.count(16)?;
        let mut external_objects = Vec::with_capacity(n);
        for _ in 0..n {
            let e = ExternalObject {
                object_id: r.u32()?,
                file_ref: r.u32()?,
                flags: r.u32()?,
                type_id: r.u32()?,
            };
            if e.file_ref as usize >= files.len() {
                return r.fail(format!(
                    "EXOB object {} refers to file {} but RFIL has {} entries",
                    e.object_id,
                    e.file_ref,
                    files.len()
                ));
            }
            external_objects.push(e);
        }
        sections.push(Section {
            tag: *b"EXOB",
            range: start..r.pos,
        });

        let object_table = Self::pairs(&mut r, b"OBTY", &mut sections)?;
        let extra_table = Self::pairs(&mut r, b"EDTY", &mut sections)?;

        let start = r.tag(b"OBJS")?;
        let object_record_count = r.u32()?;
        if object_record_count as usize != object_table.len() {
            return r.fail(format!(
                "OBJS has {object_record_count} records but OBTY has {} entries",
                object_table.len()
            ));
        }
        let data_start = r.pos;
        if !data.ends_with(FOOTER) {
            return Err(Error::invalid(
                WHAT,
                "the file does not end with the footer `METAEND ` (truncated or damaged)",
            ));
        }
        if data_start > data.len() - FOOTER.len() {
            r.pos = data_start;
            return r.fail("the object data overlaps the footer");
        }
        let has_extra_records =
            !data.ends_with(NO_EXTRA_TAIL) || data.len() - NO_EXTRA_TAIL.len() < data_start;
        let object_end = if has_extra_records {
            data.len() - FOOTER.len()
        } else {
            data.len() - NO_EXTRA_TAIL.len()
        };
        sections.push(Section {
            tag: *b"OBJS",
            range: start..object_end,
        });
        if !has_extra_records {
            sections.push(Section {
                tag: *b"EDOB",
                range: object_end..data.len() - FOOTER.len(),
            });
        }

        Ok(Self {
            data,
            header,
            info,
            files,
            idents,
            external_types,
            types,
            array_dims,
            external_objects,
            object_table,
            extra_table,
            object_record_count,
            object_data: data_start..object_end,
            has_extra_records,
            sections,
            type_index,
        })
    }

    fn pairs(
        r: &mut Reader<'a>,
        tag: &[u8; 4],
        sections: &mut Vec<Section>,
    ) -> Result<Vec<ObjectEntry>> {
        let start = r.tag(tag)?;
        let n = r.count(8)?;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            out.push(ObjectEntry {
                object_id: r.u32()?,
                type_id: r.u32()?,
            });
        }
        sections.push(Section {
            tag: *tag,
            range: start..r.pos,
        });
        Ok(out)
    }

    fn type_block(r: &mut Reader<'a>, format_version: u32) -> Result<TypeDef<'a>> {
        r.tag(b"DTTY")?;
        let id = r.u32()?;
        let name = r.string()?;
        let kind = r.u32()?;
        // A struct block has `flags, parent` and then `STMB`.
        let is_struct = r.data.get(r.pos + 8..r.pos + 12) == Some(b"STMB");
        if is_struct {
            let flags = r.u32()?;
            let parent = r.u32()?;
            r.tag(b"STMB")?;
            let min_member = if format_version == 4 { 12 } else { 8 };
            let n = r.count(min_member)?;
            let mut members = Vec::with_capacity(n);
            for _ in 0..n {
                let number = (format_version == 4).then(|| r.u32()).transpose()?;
                let name = r.string()?;
                let type_id = r.u32()?;
                members.push(Member {
                    name,
                    type_id,
                    number,
                });
            }
            return Ok(TypeDef {
                id,
                name,
                kind,
                shape: TypeShape::Struct {
                    flags,
                    parent: (parent != NULL_ID).then_some(parent),
                    members,
                },
            });
        }
        // Short blocks: kind 1 with first word 0 is a primitive (three words), all others two.
        let first = r.u32()?;
        let mut words = vec![first];
        let more = if kind == 1 && first == 0 { 2 } else { 1 };
        for _ in 0..more {
            words.push(r.u32()?);
        }
        let def = TypeDef {
            id,
            name,
            kind,
            shape: TypeShape::Short { words },
        };
        if def.short_form().is_none() {
            return r.fail(format!(
                "type {} `{}`: unknown short block form (kind {kind}, words {:?})",
                def.id, def.name, def.shape
            ));
        }
        Ok(def)
    }

    /// The type with id `id` among the `DTTY` blocks.
    pub fn type_def(&self, id: u32) -> Option<&TypeDef<'a>> {
        self.type_index.get(&id).map(|&i| &self.types[i])
    }

    /// The name of external type `id`.
    pub fn external_type_name(&self, id: u32) -> Option<&str> {
        self.external_types
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.name.as_ref())
    }

    /// The name of type `id`, internal or external.
    pub fn type_name(&self, id: u32) -> Option<&str> {
        self.type_def(id)
            .map(|t| t.name.as_ref())
            .or_else(|| self.external_type_name(id))
    }

    /// The identifier with id `id`.
    pub fn ident(&self, id: u32) -> Option<&str> {
        self.idents.get(id as usize).map(Cow::as_ref)
    }

    /// The bytes of one section (tag included), if the section was located.
    pub fn section(&self, tag: &[u8; 4]) -> Option<&'a [u8]> {
        self.sections
            .iter()
            .find(|s| &s.tag == tag)
            .map(|s| &self.data[s.range.clone()])
    }

    /// The `INFO` words that disagree with the tables: the file and identifier counts are always
    /// checked by [`Container::parse`] only against the table sizes read here, the type and object
    /// totals are reported (a few real files differ in `INFO[4]`). Empty when everything agrees.
    pub fn info_mismatches(&self) -> Vec<String> {
        let mut out = Vec::new();
        let mut check = |what: &str, info: u32, real: usize| {
            if info as usize != real {
                out.push(format!("INFO says {info} {what}, the tables hold {real}"));
            }
        };
        check("file references", self.info.file_count, self.files.len());
        check("identifiers", self.info.ident_count, self.idents.len());
        check(
            "types",
            self.info.type_count,
            self.external_types.len() + self.types.len(),
        );
        check(
            "objects",
            self.info.object_count,
            self.external_objects.len() + self.object_table.len(),
        );
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, SampleIds, TypeBlock};

    fn sample() -> Vec<u8> {
        MetaBuilder::sample().build().unwrap()
    }

    #[test]
    fn parses_the_sample_frame() {
        let bytes = sample();
        let m = Container::parse(&bytes).unwrap();
        assert_eq!(m.header.format_version, 8);
        let b = m.header.build.as_ref().unwrap();
        assert_eq!(
            (
                b.product.as_str(),
                b.release.as_str(),
                b.build,
                b.tag.as_str()
            ),
            ("Serious Sam 2", "2.062.00", 57284, "2005-Gold-RC5")
        );
        assert_eq!(m.info.flag, Some(1));
        assert!(m.info_mismatches().is_empty(), "{:?}", m.info_mismatches());
        assert_eq!(m.files.len(), 2);
        assert_eq!(m.files[1].flags, FLAG_PARAMETER_REFERENCE);
        assert_eq!(m.idents.len(), 2);
        assert_eq!(m.ident(0), Some(""));
        assert_eq!(m.ident(1), Some("TestItem"));
        assert_eq!(m.external_types[0].name, "CModelConfiguration");
        assert_eq!(
            m.type_name(SampleIds::EXTERNAL_MODEL),
            Some("CModelConfiguration")
        );
        assert_eq!(m.type_name(SampleIds::SLONG), Some("SLONG"));
        assert_eq!(m.type_name(9999), None);
        assert_eq!(m.types.len(), 10);
        assert_eq!(m.array_dims.len(), 1);
        assert_eq!(m.array_dims[0].dims, [3]);
        assert_eq!(m.external_objects.len(), 1);
        assert_eq!(m.external_objects[0].file_ref, 0);
        assert_eq!(m.object_table.len(), 2);
        assert_eq!(
            m.extra_table,
            [ObjectEntry {
                object_id: 0,
                type_id: 1
            }]
        );
        assert_eq!(m.object_record_count, 2);
        assert!(!m.has_extra_records);
        assert!(m.object_data.start < m.object_data.end);
        assert!(m.sections.iter().any(|s| &s.tag == b"EDOB"));
        assert_eq!(m.section(b"EDOB").unwrap(), b"EDOB\0\0\0\0");
    }

    #[test]
    fn classifies_short_blocks_and_structs() {
        let bytes = sample();
        let m = Container::parse(&bytes).unwrap();
        let form = |id| m.type_def(id).unwrap().short_form();
        assert_eq!(
            form(SampleIds::SLONG),
            Some(ShortForm::Primitive { size: 4, align: 4 })
        );
        assert_eq!(
            form(SampleIds::INDEX),
            Some(ShortForm::Alias {
                target: SampleIds::SLONG
            })
        );
        assert_eq!(
            form(SampleIds::MODE),
            Some(ShortForm::Enumeration { size: 1 })
        );
        assert_eq!(
            form(SampleIds::FLOAT3),
            Some(ShortForm::FixedArray {
                element: SampleIds::FLOAT
            })
        );
        assert_eq!(
            form(SampleIds::ITEM_PTR),
            Some(ShortForm::Pointer {
                target: SampleIds::ITEM
            })
        );
        assert_eq!(
            form(SampleIds::ITEM_LIST),
            Some(ShortForm::DynamicContainer {
                element: SampleIds::ITEM
            })
        );
        let item = m.type_def(SampleIds::ITEM).unwrap();
        assert_eq!(form(SampleIds::ITEM), None);
        let TypeShape::Struct {
            flags,
            parent,
            members,
        } = &item.shape
        else {
            panic!()
        };
        assert_eq!((item.kind, *flags, *parent, members.len()), (2, 5, None, 5));
        assert_eq!(members[0].name, "1");
        assert_eq!(members[0].number, None);
        let holder = m.type_def(SampleIds::HOLDER).unwrap();
        let TypeShape::Struct { parent, .. } = &holder.shape else {
            panic!()
        };
        assert_eq!(*parent, Some(SampleIds::ITEM));
    }

    #[test]
    fn extra_records_are_reported_not_decoded() {
        let mut b = MetaBuilder::sample();
        b.after_edob(1, b"\x07\0\0\0\x01\0\0\0whatever");
        let bytes = b.build().unwrap();
        let m = Container::parse(&bytes).unwrap();
        assert!(m.has_extra_records);
        assert!(m.sections.iter().all(|s| &s.tag != b"EDOB"));
        assert!(
            bytes[m.object_data.clone()]
                .windows(4)
                .any(|w| w == b"EDOB")
        );
        assert_eq!(m.object_data.end, bytes.len() - 8);
    }

    #[test]
    fn accepts_any_version_string_and_latin1_names() {
        let mut b = MetaBuilder::new();
        b.version("not the usual shape");
        b.file_ref("Content/Caf\u{e9}.tex", 0);
        let mut bytes = b.build().unwrap();
        // Replace the "e acute" (UTF-8 C3 A9) by the Latin-1 byte E9 as the real files do.
        let pos = bytes.windows(2).position(|w| w == [0xC3, 0xA9]).unwrap();
        bytes.remove(pos);
        bytes[pos] = 0xE9;
        // The length field of the path is one smaller now.
        let len_pos = bytes
            .windows(4)
            .position(|w| w == b"Content/Caf".get(..4).unwrap())
            .unwrap()
            - 4;
        let len = u32::from_le_bytes(bytes[len_pos..len_pos + 4].try_into().unwrap()) - 1;
        bytes[len_pos..len_pos + 4].copy_from_slice(&len.to_le_bytes());
        let m = Container::parse(&bytes).unwrap();
        assert_eq!(m.header.version_string, "not the usual shape");
        assert_eq!(m.header.build, None);
        assert_eq!(m.files[0].path, "Content/Caf\u{e9}.tex");
    }

    #[test]
    fn rejects_bad_headers() {
        let good = sample();
        let mut bad = good.clone();
        bad[0] = b'X';
        assert!(
            Container::parse(&bad)
                .unwrap_err()
                .to_string()
                .contains("magic")
        );
        let mut bad = good.clone();
        bad[8] ^= 1;
        assert!(
            Container::parse(&bad)
                .unwrap_err()
                .to_string()
                .contains("byte-order")
        );
        for v in [0u32, 3, 9, 1000] {
            let mut bad = good.clone();
            bad[12..16].copy_from_slice(&v.to_le_bytes());
            let err = Container::parse(&bad).unwrap_err().to_string();
            assert!(err.contains("unsupported format version"), "{v}: {err}");
        }
    }

    #[test]
    fn every_truncation_is_an_error_and_never_a_panic() {
        let good = sample();
        for len in 0..good.len() {
            assert!(
                Container::parse(&good[..len]).is_err(),
                "prefix of {len} bytes parsed"
            );
        }
    }

    #[test]
    fn no_single_byte_change_panics() {
        let good = sample();
        for i in 0..good.len() {
            for value in [0u8, 0xFF, good[i].wrapping_add(1)] {
                let mut bad = good.clone();
                bad[i] = value;
                let _ = Container::parse(&bad);
            }
        }
    }

    #[test]
    fn hostile_counts_do_not_allocate() {
        let mut bytes = MetaBuilder::new().build().unwrap();
        // The RFIL count (right after the INFO words) becomes 4 billion.
        let pos = bytes.windows(4).position(|w| w == b"RFIL").unwrap() + 4;
        bytes[pos..pos + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        let err = Container::parse(&bytes).unwrap_err().to_string();
        assert!(err.contains("does not fit"), "{err}");
    }

    #[test]
    fn structural_invariants_are_enforced() {
        // A dangling EXOB fileRef is refused by the writer, so patch the bytes.
        let mut b = MetaBuilder::new();
        let f = b.file_ref("a.tex", 0);
        b.type_block(TypeBlock::primitive(0, "SLONG", 4, 4));
        b.external_object(5, f, 1, 0);
        let mut bytes = b.build().unwrap();
        let pos = bytes.windows(4).position(|w| w == b"EXOB").unwrap() + 4 + 4 + 4;
        bytes[pos..pos + 4].copy_from_slice(&9u32.to_le_bytes());
        let err = Container::parse(&bytes).unwrap_err().to_string();
        assert!(err.contains("refers to file 9"), "{err}");

        // RFIL index out of sequence.
        let mut b = MetaBuilder::new();
        b.file_ref("a.tex", 0);
        let mut bytes = b.build().unwrap();
        let pos = bytes.windows(4).position(|w| w == b"RFIL").unwrap() + 8;
        bytes[pos..pos + 4].copy_from_slice(&3u32.to_le_bytes());
        assert!(
            Container::parse(&bytes)
                .unwrap_err()
                .to_string()
                .contains("has index 3")
        );

        // INTY count disagreeing with the table.
        let mut b = MetaBuilder::new();
        b.type_block(TypeBlock::primitive(0, "SLONG", 4, 4));
        let mut bytes = b.build().unwrap();
        let pos = bytes.windows(4).position(|w| w == b"INTY").unwrap() + 4;
        bytes[pos..pos + 4].copy_from_slice(&2u32.to_le_bytes());
        assert!(Container::parse(&bytes).is_err());

        // An unknown short form.
        let mut b = MetaBuilder::new();
        b.type_block(TypeBlock::short(0, "Odd", 0, 99, 1));
        let err = Container::parse(&b.build().unwrap())
            .unwrap_err()
            .to_string();
        assert!(err.contains("unknown short block form"), "{err}");

        // The same type id twice cannot be built; patch it in.
        let mut b = MetaBuilder::new();
        b.type_block(TypeBlock::primitive(0, "A", 4, 4));
        b.type_block(TypeBlock::primitive(1, "B", 4, 4));
        let mut bytes = b.build().unwrap();
        let pos = bytes.windows(4).rposition(|w| w == b"DTTY").unwrap() + 4;
        bytes[pos..pos + 4].copy_from_slice(&0u32.to_le_bytes());
        assert!(
            Container::parse(&bytes)
                .unwrap_err()
                .to_string()
                .contains("defined twice")
        );
    }

    #[test]
    fn info_mismatch_is_reported_not_fatal() {
        let mut bytes = sample();
        // INFO[4] (the object total) sits 20 bytes after the INFO tag: tag + 5 words.
        let pos = bytes.windows(4).position(|w| w == b"INFO").unwrap() + 4 + 4 * 4;
        bytes[pos..pos + 4].copy_from_slice(&99u32.to_le_bytes());
        let m = Container::parse(&bytes).unwrap();
        let mismatches = m.info_mismatches();
        assert_eq!(mismatches.len(), 1, "{mismatches:?}");
        assert!(mismatches[0].contains("99 objects"), "{mismatches:?}");
    }

    /// Build a format 4 to 7 file by hand: four `INFO` words and, in version 4, a number before
    /// every member name (`docs/re/ctsemeta-leftovers.md`).
    fn old_file(version: u32) -> Vec<u8> {
        fn u(out: &mut Vec<u8>, v: u32) {
            out.extend_from_slice(&v.to_le_bytes());
        }
        fn s(out: &mut Vec<u8>, v: &str) {
            u(out, v.len() as u32);
            out.extend_from_slice(v.as_bytes());
        }
        let mut o = Vec::new();
        o.extend_from_slice(b"CTSEMETA");
        u(&mut o, BYTE_ORDER_MARKER);
        u(&mut o, version);
        s(
            &mut o,
            "$Version: Serious Sam 2; 2.005.00:24818; Development$",
        );
        o.extend_from_slice(b"INFO");
        for w in [0, 1, 2, 1] {
            u(&mut o, w); // RFIL 0, IDNT 1, types 2, objects 1
        }
        o.extend_from_slice(b"RFIL");
        u(&mut o, 0);
        o.extend_from_slice(b"IDNT");
        u(&mut o, 1);
        u(&mut o, 0);
        s(&mut o, "");
        o.extend_from_slice(b"EXTY");
        u(&mut o, 0);
        o.extend_from_slice(b"INTY");
        u(&mut o, 2);
        o.extend_from_slice(b"DTTY");
        u(&mut o, 0);
        s(&mut o, "SLONG");
        for w in [1, 0, 4, 4] {
            u(&mut o, w);
        }
        o.extend_from_slice(b"DTTY");
        u(&mut o, 1);
        s(&mut o, "CResourceFile");
        for w in [2, 5, NULL_ID] {
            u(&mut o, w);
        }
        o.extend_from_slice(b"STMB");
        u(&mut o, 2);
        for (number, name) in [(1u32, "rf_iNextFreeID"), (2, "rf_iDefaultID")] {
            if version == 4 {
                u(&mut o, number);
            }
            s(&mut o, name);
            u(&mut o, 0);
        }
        o.extend_from_slice(b"EXOB");
        u(&mut o, 0);
        o.extend_from_slice(b"OBTY");
        u(&mut o, 1);
        u(&mut o, 0);
        u(&mut o, 1);
        o.extend_from_slice(b"EDTY");
        u(&mut o, 0);
        o.extend_from_slice(b"OBJS");
        u(&mut o, 1);
        u(&mut o, 0);
        u(&mut o, 1);
        u(&mut o, 7);
        u(&mut o, 9);
        o.extend_from_slice(b"EDOB");
        u(&mut o, 0);
        o.extend_from_slice(b"METAEND ");
        o
    }

    #[test]
    fn reads_versions_4_to_7() {
        for version in 4..=7 {
            let bytes = old_file(version);
            let m = Container::parse(&bytes).unwrap_or_else(|e| panic!("version {version}: {e}"));
            assert_eq!(m.header.format_version, version);
            assert_eq!(m.info.flag, None);
            assert_eq!(m.info.type_count, 2);
            assert!(m.info_mismatches().is_empty(), "{:?}", m.info_mismatches());
            let TypeShape::Struct { members, .. } = &m.type_def(1).unwrap().shape else {
                panic!()
            };
            assert_eq!(members.len(), 2);
            assert_eq!(members[1].name, "rf_iDefaultID");
            assert_eq!(members[1].number, (version == 4).then_some(2));
            assert!(!m.has_extra_records);
            assert_eq!(m.object_data.len(), 8 + 8);
        }
    }

    /// Data-gated: every container of the retail install parses, and the version and counts are
    /// what the specs say. Skips without `SS2_DATA_DIR`.
    #[test]
    fn every_file_of_the_real_install_parses() {
        use ss2_vfs::{EntryKind, Vfs, VfsPath};
        let dir = ss2_testkit::require_game_data!();
        let vfs = ss2_vfs::mount_install(&dir).unwrap();
        let mut stack = vec![VfsPath::root()];
        let (mut parsed, mut old, mut skipped) = (0usize, 0usize, 0usize);
        let mut failures = Vec::new();
        let mut info_off = 0usize;
        let mut extras = 0usize;
        while let Some(d) = stack.pop() {
            for e in vfs.list(&d).unwrap() {
                let path = d.join(&e.name).unwrap();
                if e.kind == EntryKind::Dir {
                    stack.push(path);
                    continue;
                }
                if vfs.stat(&path).unwrap().size < 24 {
                    continue;
                }
                let mut head = [0u8; 8];
                if std::io::Read::read_exact(&mut vfs.open(&path).unwrap(), &mut head).is_err()
                    || &head != MAGIC
                {
                    continue;
                }
                if vfs.stat(&path).unwrap().size > 150 * 1024 * 1024 {
                    skipped += 1;
                    continue;
                }
                let bytes = vfs.read(&path).unwrap();
                match Container::parse(&bytes) {
                    Ok(m) => {
                        parsed += 1;
                        if m.header.format_version != 8 {
                            old += 1;
                            assert_eq!(m.info.flag, None);
                        }
                        let mismatches = m.info_mismatches();
                        if mismatches
                            .iter()
                            .any(|s| s.contains("file ref") || s.contains("identifiers"))
                        {
                            failures.push(format!("{}: {mismatches:?}", path.as_str()));
                        }
                        info_off += usize::from(!mismatches.is_empty());
                        extras += usize::from(m.has_extra_records);
                    }
                    Err(e) => failures.push(format!("{}: {e}", path.as_str())),
                }
            }
        }
        eprintln!(
            "parsed {parsed} containers ({old} old versions, {extras} with extra records, {info_off} with INFO count differences), skipped {skipped} over 150 MB"
        );
        assert!(
            failures.is_empty(),
            "{} failures, first: {:?}",
            failures.len(),
            &failures[..failures.len().min(5)]
        );
        assert!(parsed > 8000, "only {parsed} containers parsed");
        assert!(old >= 100, "only {old} old-version files");
    }

    #[test]
    fn builder_objects_and_bytes_helpers_round_trip() {
        let mut b = MetaBuilder::new();
        b.type_block(TypeBlock::primitive(0, "SLONG", 4, 4));
        b.object(3, 0, ObjectBytes::new().i32(1).finish());
        let bytes = b.build().unwrap();
        let m = Container::parse(&bytes).unwrap();
        assert_eq!(
            m.object_table,
            [ObjectEntry {
                object_id: 3,
                type_id: 0
            }]
        );
        assert_eq!(bytes[m.object_data.clone()].len(), 8 + 4);
    }
}
