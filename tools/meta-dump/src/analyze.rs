//! Turn the bytes of one CTSEMETA file into a [`Report`]: every region with its byte span, the
//! decoded tables and objects, and a coverage map of which bytes some decoder explained.
//!
//! Nothing here parses the format on its own: the frame comes from [`ss2_meta::Container::parse`]
//! and the values from [`ss2_meta::TypeSystem::decode_value`]. What this module adds is
//! bookkeeping: the byte offset of every table entry and object record (computed from the sizes
//! the format prescribes and checked against the section ends the container reports), and the
//! coverage map.

use std::collections::BTreeMap;
use std::ops::Range;

use ss2_meta::{Build, Container, ObjectEntry, ShortForm, TypeDef, TypeShape, TypeSystem, Value};

use crate::Options;
use crate::node::Node;

/// A half-open byte range of the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    /// First byte.
    pub start: usize,
    /// One past the last byte.
    pub end: usize,
}

impl Span {
    /// Length in bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// `true` for an empty span.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The bytes of `data` this span covers (clamped to the data; never panics).
    #[must_use]
    pub fn bytes<'d>(&self, data: &'d [u8]) -> &'d [u8] {
        let end = self.end.min(data.len());
        data.get(self.start.min(end)..end).unwrap_or(&[])
    }
}

impl From<Range<usize>> for Span {
    fn from(r: Range<usize>) -> Self {
        Self {
            start: r.start,
            end: r.end,
        }
    }
}

/// The file header.
#[derive(Clone, Debug)]
pub struct HeaderInfo {
    /// Magic, byte-order marker, format version and version string.
    pub span: Span,
    /// The format version word.
    pub format_version: u32,
    /// The raw `$Version: …$` text.
    pub version_string: String,
    /// The parsed build, when the text has the usual shape.
    pub build: Option<Build>,
}

/// The `INFO` section.
#[derive(Clone, Debug)]
pub struct InfoInfo {
    /// Bytes of the section.
    pub span: Span,
    /// The first word (versions 5 to 8 only); its meaning is not known.
    pub flag: Option<u32>,
    /// Declared number of `RFIL` entries.
    pub file_count: u32,
    /// Declared number of `IDNT` entries.
    pub ident_count: u32,
    /// Declared number of types.
    pub type_count: u32,
    /// Declared number of objects.
    pub object_count: u32,
    /// Declared counts that disagree with the tables.
    pub mismatches: Vec<String>,
}

/// One chunk (section) of the file.
#[derive(Clone, Debug)]
pub struct Chunk {
    /// `header`, a four-letter tag, or `METAEND`.
    pub tag: String,
    /// Bytes of the chunk, tag included.
    pub span: Span,
    /// Number of entries, for the sections that hold a counted table.
    pub entries: Option<u64>,
}

/// An `RFIL` entry.
#[derive(Clone, Debug)]
pub struct FileEntry {
    /// Bytes of the entry.
    pub span: Span,
    /// Index in the table.
    pub index: u32,
    /// Flag word.
    pub flags: u32,
    /// The referenced path.
    pub path: String,
}

/// An `IDNT` entry.
#[derive(Clone, Debug)]
pub struct IdentEntry {
    /// Bytes of the entry.
    pub span: Span,
    /// Identifier id.
    pub id: u32,
    /// The text.
    pub text: String,
}

/// An `EXTY` entry.
#[derive(Clone, Debug)]
pub struct ExternalTypeEntry {
    /// Bytes of the entry.
    pub span: Span,
    /// Type id.
    pub id: u32,
    /// Class name.
    pub name: String,
}

/// A member of a struct type.
#[derive(Clone, Debug)]
pub struct MemberEntry {
    /// Member name.
    pub name: String,
    /// Type id of the member.
    pub type_id: u32,
    /// Name of that type, when it is defined.
    pub type_name: Option<String>,
}

/// A `DTTY` block.
#[derive(Clone, Debug)]
pub struct TypeEntry {
    /// Bytes of the block.
    pub span: Span,
    /// Type id.
    pub id: u32,
    /// Type name.
    pub name: String,
    /// The `kind` word.
    pub kind: u32,
    /// `struct`, `primitive`, `alias`, `enumeration`, `pointer`, `fixed-array`, `static-array`,
    /// `static-stack-array`, `dynamic-container`, `smart-pointer` or `handle`.
    pub form: &'static str,
    /// Size in bytes (primitives and enumerations).
    pub size: Option<u32>,
    /// Alignment (primitives).
    pub align: Option<u32>,
    /// Pointee, element or alias target.
    pub target: Option<u32>,
    /// Name of the target type.
    pub target_name: Option<String>,
    /// The `flags` word of a struct.
    pub flags: Option<u32>,
    /// Base class of a struct.
    pub parent: Option<u32>,
    /// Name of the base class.
    pub parent_name: Option<String>,
    /// Members of a struct, in file order.
    pub members: Vec<MemberEntry>,
}

/// An `ADIM` block.
#[derive(Clone, Debug)]
pub struct DimsEntry {
    /// Bytes of the block.
    pub span: Span,
    /// How many `DTTY` blocks precede it.
    pub after_type: usize,
    /// The dimensions.
    pub dims: Vec<u32>,
}

/// An `EXOB` entry.
#[derive(Clone, Debug)]
pub struct ExternalObjectEntry {
    /// Bytes of the entry.
    pub span: Span,
    /// Object id.
    pub object_id: u32,
    /// Index into `RFIL`.
    pub file_ref: u32,
    /// The referenced path.
    pub path: String,
    /// Flag word.
    pub flags: u32,
    /// Type id.
    pub type_id: u32,
    /// Type name.
    pub type_name: Option<String>,
}

/// An `OBTY` or `EDTY` entry.
#[derive(Clone, Debug)]
pub struct PairEntry {
    /// Bytes of the entry.
    pub span: Span,
    /// Object id.
    pub object_id: u32,
    /// Type id.
    pub type_id: u32,
    /// Type name.
    pub type_name: Option<String>,
}

/// An object record that decoded.
#[derive(Clone, Debug)]
pub struct ObjectRec {
    /// Bytes of the record (the 8-byte header included).
    pub span: Span,
    /// Object id.
    pub id: u32,
    /// Type id.
    pub type_id: u32,
    /// Type name.
    pub type_name: Option<String>,
    /// The decoded value (`None` with [`Options::skip_values`]).
    pub value: Option<Node>,
}

/// A run of bytes no decoder explained.
#[derive(Clone, Debug)]
pub struct Gap {
    /// The bytes.
    pub span: Span,
    /// The section of the frame it lies in (`(file)` when the frame could not be read).
    pub section: String,
    /// Why nothing explains them.
    pub reason: String,
}

/// Which bytes of the file some decoder explained.
#[derive(Clone, Debug)]
pub struct Coverage {
    /// File size.
    pub total: usize,
    /// Bytes explained (`total - undecoded`).
    pub decoded: usize,
    /// Of the explained bytes: bytes the decoder reads but whose meaning is unknown (the `INFO`
    /// flag word and raw byte-array payloads).
    pub opaque: usize,
    /// The unexplained runs, in file order.
    pub undecoded: Vec<Gap>,
}

impl Coverage {
    /// Total size of the unexplained runs.
    #[must_use]
    pub fn undecoded_bytes(&self) -> usize {
        self.undecoded.iter().map(|g| g.span.len()).sum()
    }
}

/// A file that parsed.
#[derive(Clone, Debug)]
pub struct Report<'a> {
    /// What the user called the file.
    pub label: String,
    /// The file.
    pub data: &'a [u8],
    /// Header.
    pub header: HeaderInfo,
    /// `INFO`.
    pub info: InfoInfo,
    /// The chunks in file order.
    pub chunks: Vec<Chunk>,
    /// `RFIL`.
    pub files: Vec<FileEntry>,
    /// `IDNT`.
    pub idents: Vec<IdentEntry>,
    /// `EXTY`.
    pub external_types: Vec<ExternalTypeEntry>,
    /// `DTTY` blocks.
    pub types: Vec<TypeEntry>,
    /// `ADIM` blocks.
    pub array_dims: Vec<DimsEntry>,
    /// `EXOB`.
    pub external_objects: Vec<ExternalObjectEntry>,
    /// `OBTY`.
    pub object_table: Vec<PairEntry>,
    /// `EDTY`.
    pub extra_table: Vec<PairEntry>,
    /// The object records that decoded, in file order.
    pub objects: Vec<ObjectRec>,
    /// The extra records after `EDOB` that decoded.
    pub extra: Vec<ObjectRec>,
    /// Everything that went wrong or does not add up: a record that did not decode, a count that
    /// disagrees with a table, a span that could not be verified.
    pub problems: Vec<String>,
    /// The coverage map.
    pub coverage: Coverage,
}

/// A file whose frame could not be read: what is known about it.
#[derive(Clone, Debug)]
pub struct Damaged<'a> {
    /// What the user called the file.
    pub label: String,
    /// The file.
    pub data: &'a [u8],
    /// Why the frame was rejected (the message of [`Container::parse`]).
    pub error: String,
    /// The header, when it could be read.
    pub header: Option<HeaderInfo>,
    /// Coverage: the header bytes that read, everything else undecoded.
    pub coverage: Coverage,
}

/// The result of [`analyze`].
#[derive(Clone, Debug)]
pub enum Outcome<'a> {
    /// The frame parsed.
    Parsed(Box<Report<'a>>),
    /// The frame did not parse.
    Damaged(Box<Damaged<'a>>),
}

impl Outcome<'_> {
    /// The coverage map, either way.
    #[must_use]
    pub fn coverage(&self) -> &Coverage {
        match self {
            Outcome::Parsed(r) => &r.coverage,
            Outcome::Damaged(d) => &d.coverage,
        }
    }

    /// `true` if the frame parsed.
    #[must_use]
    pub fn is_parsed(&self) -> bool {
        matches!(self, Outcome::Parsed(_))
    }
}

fn le32(data: &[u8], pos: usize) -> Option<u32> {
    let b = data.get(pos..pos.checked_add(4)?)?;
    Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// The coverage bitmap: one flag per byte of the file.
struct Painter {
    painted: Vec<bool>,
}

impl Painter {
    fn new(len: usize) -> Self {
        Self {
            painted: vec![false; len],
        }
    }

    fn paint(&mut self, start: usize, end: usize) {
        let end = end.min(self.painted.len());
        if start < end {
            self.painted[start..end].fill(true);
        }
    }

    fn gaps(&self) -> Vec<Span> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < self.painted.len() {
            if self.painted[i] {
                i += 1;
                continue;
            }
            let start = i;
            while i < self.painted.len() && !self.painted[i] {
                i += 1;
            }
            out.push(Span { start, end: i });
        }
        out
    }
}

/// Header bytes that read, as far as they read. Returns the painted end and the header if all of
/// it was readable.
fn salvage_header(data: &[u8], painter: &mut Painter) -> Option<HeaderInfo> {
    if data.get(..8) != Some(b"CTSEMETA") {
        return None;
    }
    painter.paint(0, 8);
    if le32(data, 8) != Some(0x1234_ABCD) {
        return None;
    }
    painter.paint(8, 12);
    let format_version = le32(data, 12)?;
    painter.paint(12, 16);
    let len = le32(data, 16)? as usize;
    let end = 20usize.checked_add(len)?;
    let text = data.get(20..end)?;
    painter.paint(16, end);
    let version_string: String = text.iter().map(|&b| char::from(b)).collect();
    Some(HeaderInfo {
        span: Span { start: 0, end },
        format_version,
        build: Build::parse(&version_string),
        version_string,
    })
}

/// Analyze one file. Never panics: a file whose frame does not parse yields
/// [`Outcome::Damaged`].
#[must_use]
pub fn analyze<'a>(data: &'a [u8], label: &str, opts: &Options) -> Outcome<'a> {
    match Container::parse(data) {
        Ok(c) => Outcome::Parsed(Box::new(build_report(&c, data, label, opts))),
        Err(e) => {
            let mut painter = Painter::new(data.len());
            let header = salvage_header(data, &mut painter);
            let error = e.to_string();
            let undecoded = painter
                .gaps()
                .into_iter()
                .map(|span| Gap {
                    span,
                    section: "(file)".into(),
                    reason: format!("the container frame could not be read: {error}"),
                })
                .collect::<Vec<_>>();
            let undecoded_bytes: usize = undecoded.iter().map(|g| g.span.len()).sum();
            Outcome::Damaged(Box::new(Damaged {
                label: label.to_string(),
                data,
                error,
                header,
                coverage: Coverage {
                    total: data.len(),
                    decoded: data.len() - undecoded_bytes,
                    opaque: 0,
                    undecoded,
                },
            }))
        }
    }
}

/// What the value converter needs to know about the file.
struct ValueCtx<'c> {
    container: &'c Container<'c>,
    types: &'c TypeSystem,
    /// Object id to `(type id, referenced file)`.
    targets: BTreeMap<u32, (u32, Option<String>)>,
    preview: usize,
    opaque: usize,
}

impl ValueCtx<'_> {
    fn type_name(&self, id: u32) -> Option<String> {
        self.container.type_name(id).map(str::to_string)
    }

    fn node(&mut self, v: &Value) -> Node {
        match v {
            Value::U8(x) => Node::int(*x),
            Value::I8(x) => Node::int(*x),
            Value::U16(x) => Node::int(*x),
            Value::I16(x) => Node::int(*x),
            Value::U32(x) => Node::int(*x),
            Value::I32(x) => Node::int(*x),
            Value::U64(x) => Node::int(*x),
            Value::I64(x) => Node::int(*x),
            Value::F32(x) => Node::f32(*x),
            Value::F64(x) => Node::f64(*x),
            Value::Ident(id) => Node::map([
                ("ident", Node::int(*id)),
                (
                    "text",
                    self.container.ident(*id).map_or(Node::Null, Node::str),
                ),
            ]),
            Value::Enum { type_id, value } => Node::map([
                (
                    "enum",
                    self.type_name(*type_id).map_or(Node::Null, Node::Str),
                ),
                ("value", Node::int(*value)),
            ]),
            Value::Ref(None) => Node::Null,
            Value::Ref(Some(id)) => {
                let mut pairs = vec![("ref".to_string(), Node::int(*id))];
                match self.targets.get(id) {
                    Some((type_id, file)) => {
                        pairs.push((
                            "to".into(),
                            Node::str(if file.is_some() { "external" } else { "object" }),
                        ));
                        pairs.push((
                            "type".into(),
                            self.type_name(*type_id).map_or(Node::Null, Node::Str),
                        ));
                        if let Some(f) = file {
                            pairs.push(("file".into(), Node::str(f.clone())));
                        }
                    }
                    None => pairs.push(("to".into(), Node::str("unresolved"))),
                }
                Node::Map(pairs)
            }
            Value::String(s) => Node::str(s.clone()),
            Value::Struct(s) => {
                let layout = self.types.members(s.type_id);
                let members = s
                    .values
                    .iter()
                    .enumerate()
                    .map(|(i, v)| {
                        let label = match layout {
                            Some(l) if l.len() == s.values.len() => {
                                format!("{}.{}", l[i].owner_name, l[i].name)
                            }
                            _ => format!("#{i}"),
                        };
                        (label, self.node(v))
                    })
                    .collect();
                Node::Struct {
                    name: self
                        .types
                        .type_name(s.type_id)
                        .map_or_else(|| format!("type {}", s.type_id), str::to_string),
                    members,
                }
            }
            Value::List(items) => Node::List(items.iter().map(|i| self.node(i)).collect()),
            Value::Bytes(b) => {
                self.opaque += b.len();
                Node::Bytes {
                    len: b.len(),
                    preview: b[..b.len().min(self.preview)].to_vec(),
                }
            }
        }
    }
}

/// Bytes held by raw byte arrays inside `v`.
fn opaque_bytes(v: &Value) -> usize {
    match v {
        Value::Bytes(b) => b.len(),
        Value::List(items) => items.iter().map(opaque_bytes).sum(),
        Value::Struct(s) => s.values.iter().map(opaque_bytes).sum(),
        _ => 0,
    }
}

fn describe_type(c: &Container<'_>, t: &TypeDef<'_>, span: Span) -> TypeEntry {
    let name_of = |id: u32| c.type_name(id).map(str::to_string);
    let mut e = TypeEntry {
        span,
        id: t.id,
        name: t.name.to_string(),
        kind: t.kind,
        form: "struct",
        size: None,
        align: None,
        target: None,
        target_name: None,
        flags: None,
        parent: None,
        parent_name: None,
        members: Vec::new(),
    };
    match &t.shape {
        TypeShape::Struct {
            flags,
            parent,
            members,
        } => {
            e.flags = Some(*flags);
            e.parent = *parent;
            e.parent_name = parent.and_then(name_of);
            e.members = members
                .iter()
                .map(|m| MemberEntry {
                    name: m.name.to_string(),
                    type_id: m.type_id,
                    type_name: name_of(m.type_id),
                })
                .collect();
        }
        TypeShape::Short { .. } => {
            let (form, target) = match t.short_form() {
                Some(ShortForm::Enumeration { size }) => {
                    e.size = Some(size);
                    ("enumeration", None)
                }
                Some(ShortForm::Primitive { size, align }) => {
                    e.size = Some(size);
                    e.align = Some(align);
                    ("primitive", None)
                }
                Some(ShortForm::Pointer { target }) => ("pointer", Some(target)),
                Some(ShortForm::FixedArray { element }) => ("fixed-array", Some(element)),
                Some(ShortForm::StaticArray { element }) => ("static-array", Some(element)),
                Some(ShortForm::StaticStackArray { element }) => {
                    ("static-stack-array", Some(element))
                }
                Some(ShortForm::DynamicContainer { element }) => {
                    ("dynamic-container", Some(element))
                }
                Some(ShortForm::SmartPointer { target }) => ("smart-pointer", Some(target)),
                Some(ShortForm::Handle { target }) => ("handle", Some(target)),
                Some(ShortForm::Alias { target }) => ("alias", Some(target)),
                None => ("unknown", None),
            };
            e.form = form;
            e.target = target;
            e.target_name = target.and_then(name_of);
        }
    }
    e
}

/// Size in bytes of the `DTTY` block of `t` (tag included) as the format lays it out.
fn type_block_size(t: &TypeDef<'_>, format_version: u32) -> usize {
    let head = 4 + 4 + (4 + t.name.chars().count()) + 4;
    head + match &t.shape {
        TypeShape::Struct { members, .. } => {
            let per_member_extra = if format_version == 4 { 4 } else { 0 };
            8 + 4
                + 4
                + members
                    .iter()
                    .map(|m| 4 + m.name.chars().count() + 4 + per_member_extra)
                    .sum::<usize>()
        }
        TypeShape::Short { words } => 4 * words.len(),
    }
}

fn section_range(c: &Container<'_>, tag: &[u8; 4]) -> Option<Range<usize>> {
    c.sections
        .iter()
        .find(|s| &s.tag == tag)
        .map(|s| s.range.clone())
}

/// Check that `computed` (the end of the last entry) is where the section ends.
fn verify(problems: &mut Vec<String>, tag: &str, computed: usize, section_end: usize) {
    if computed != section_end {
        problems.push(format!(
            "entry offsets of {tag} could not be verified: the entries end at byte {computed}, \
             the section ends at byte {section_end}"
        ));
    }
}

fn pair_table(
    c: &Container<'_>,
    tag: &[u8; 4],
    entries: &[ObjectEntry],
    problems: &mut Vec<String>,
) -> Vec<PairEntry> {
    let Some(range) = section_range(c, tag) else {
        return Vec::new();
    };
    let mut pos = range.start + 8;
    let out = entries
        .iter()
        .map(|e| {
            let span = Span {
                start: pos,
                end: pos + 8,
            };
            pos += 8;
            PairEntry {
                span,
                object_id: e.object_id,
                type_id: e.type_id,
                type_name: c.type_name(e.type_id).map(str::to_string),
            }
        })
        .collect();
    verify(problems, &String::from_utf8_lossy(tag), pos, range.end);
    out
}

/// Decode one record at `*pos`, checking its header against the table entry.
fn record(
    types: &TypeSystem,
    data: &[u8],
    pos: &mut usize,
    entry: &ObjectEntry,
) -> Result<(Span, Value), String> {
    let at = *pos;
    let (Some(id), Some(type_id)) = (le32(data, at), le32(data, at.wrapping_add(4))) else {
        return Err(format!("the record header at byte {at} is truncated"));
    };
    if (id, type_id) != (entry.object_id, entry.type_id) {
        return Err(format!(
            "the record header ({id}, {type_id}) at byte {at} differs from the table entry ({}, {})",
            entry.object_id, entry.type_id
        ));
    }
    let mut p = at + 8;
    match types.decode_value(data, type_id, &mut p) {
        Ok(v) => {
            *pos = p;
            Ok((Span { start: at, end: p }, v))
        }
        Err(e) => Err(format!(
            "object {id} (type {type_id}) at byte {at} does not decode: {e}"
        )),
    }
}

#[allow(clippy::too_many_lines)] // one straight pass over the sections; splitting it hides the order
fn build_report<'a>(c: &Container<'a>, data: &'a [u8], label: &str, opts: &Options) -> Report<'a> {
    let mut problems = Vec::new();
    let mut painter = Painter::new(data.len());
    let name_of = |id: u32| c.type_name(id).map(str::to_string);

    // Header.
    let header_end = 20 + le32(data, 16).unwrap_or(0) as usize;
    let header = HeaderInfo {
        span: Span {
            start: 0,
            end: header_end,
        },
        format_version: c.header.format_version,
        version_string: c.header.version_string.to_string(),
        build: c.header.build.clone(),
    };
    painter.paint(0, header_end);

    // INFO.
    let info_range = section_range(c, b"INFO").unwrap_or(header_end..header_end);
    let info = InfoInfo {
        span: info_range.clone().into(),
        flag: c.info.flag,
        file_count: c.info.file_count,
        ident_count: c.info.ident_count,
        type_count: c.info.type_count,
        object_count: c.info.object_count,
        mismatches: c.info_mismatches(),
    };

    // RFIL.
    let mut files = Vec::new();
    if let Some(range) = section_range(c, b"RFIL") {
        let mut pos = range.start + 8;
        for f in &c.files {
            let size = 12 + f.path.chars().count();
            files.push(FileEntry {
                span: Span {
                    start: pos,
                    end: pos + size,
                },
                index: f.index,
                flags: f.flags,
                path: f.path.to_string(),
            });
            pos += size;
        }
        verify(&mut problems, "RFIL", pos, range.end);
    }

    // IDNT.
    let mut idents = Vec::new();
    if let Some(range) = section_range(c, b"IDNT") {
        let mut pos = range.start + 8;
        for (i, text) in c.idents.iter().enumerate() {
            let size = 8 + text.chars().count();
            idents.push(IdentEntry {
                span: Span {
                    start: pos,
                    end: pos + size,
                },
                id: u32::try_from(i).unwrap_or(u32::MAX),
                text: text.to_string(),
            });
            pos += size;
        }
        verify(&mut problems, "IDNT", pos, range.end);
    }

    // EXTY.
    let mut external_types = Vec::new();
    if let Some(range) = section_range(c, b"EXTY") {
        let mut pos = range.start + 8;
        for t in &c.external_types {
            let size = 8 + t.name.chars().count();
            external_types.push(ExternalTypeEntry {
                span: Span {
                    start: pos,
                    end: pos + size,
                },
                id: t.id,
                name: t.name.to_string(),
            });
            pos += size;
        }
        verify(&mut problems, "EXTY", pos, range.end);
    }

    // INTY: type blocks with the ADIM blocks interleaved.
    let mut types = Vec::new();
    let mut array_dims = Vec::new();
    if let Some(range) = section_range(c, b"INTY") {
        let mut pos = range.start + 8;
        let mut dims = c.array_dims.iter().peekable();
        for (i, t) in c.types.iter().enumerate() {
            while let Some(d) = dims.next_if(|d| d.after_type <= i) {
                let size = 8 + 4 * d.dims.len();
                array_dims.push(DimsEntry {
                    span: Span {
                        start: pos,
                        end: pos + size,
                    },
                    after_type: d.after_type,
                    dims: d.dims.clone(),
                });
                pos += size;
            }
            let size = type_block_size(t, c.header.format_version);
            types.push(describe_type(
                c,
                t,
                Span {
                    start: pos,
                    end: pos + size,
                },
            ));
            pos += size;
        }
        for d in dims {
            let size = 8 + 4 * d.dims.len();
            array_dims.push(DimsEntry {
                span: Span {
                    start: pos,
                    end: pos + size,
                },
                after_type: d.after_type,
                dims: d.dims.clone(),
            });
            pos += size;
        }
        verify(&mut problems, "INTY", pos, range.end);
    }

    // EXOB.
    let mut external_objects = Vec::new();
    if let Some(range) = section_range(c, b"EXOB") {
        let mut pos = range.start + 8;
        for e in &c.external_objects {
            external_objects.push(ExternalObjectEntry {
                span: Span {
                    start: pos,
                    end: pos + 16,
                },
                object_id: e.object_id,
                file_ref: e.file_ref,
                path: c
                    .files
                    .get(e.file_ref as usize)
                    .map_or_else(String::new, |f| f.path.to_string()),
                flags: e.flags,
                type_id: e.type_id,
                type_name: name_of(e.type_id),
            });
            pos += 16;
        }
        verify(&mut problems, "EXOB", pos, range.end);
    }

    let object_table = pair_table(c, b"OBTY", &c.object_table, &mut problems);
    let extra_table = pair_table(c, b"EDTY", &c.extra_table, &mut problems);

    // Everything the frame parser consumed is explained, except the object data.
    for s in &c.sections {
        if &s.tag != b"OBJS" {
            painter.paint(s.range.start, s.range.end);
        }
    }
    let footer_start = data.len().saturating_sub(8);
    painter.paint(footer_start, data.len());

    // Objects.
    let objs_range = section_range(c, b"OBJS").unwrap_or(c.object_data.clone());
    painter.paint(objs_range.start, c.object_data.start);
    let types_sys = TypeSystem::new(c);
    let mut targets = BTreeMap::new();
    for e in c.object_table.iter().chain(&c.extra_table) {
        targets.insert(e.object_id, (e.type_id, None));
    }
    for e in &c.external_objects {
        targets.insert(
            e.object_id,
            (
                e.type_id,
                c.files.get(e.file_ref as usize).map(|f| f.path.to_string()),
            ),
        );
    }
    let mut vctx = ValueCtx {
        container: c,
        types: &types_sys,
        targets,
        preview: opts.preview,
        opaque: 0,
    };
    let mut objects = Vec::new();
    let mut extra = Vec::new();
    let mut fail: Option<(usize, String)> = None;
    let mut pos = c.object_data.start;
    let make =
        |vctx: &mut ValueCtx<'_>, span: Span, entry: &ObjectEntry, value: &Value| -> ObjectRec {
            // Without `--skip-values` no tree is built: only the opaque bytes are counted.
            let node = if opts.skip_values {
                vctx.opaque += opaque_bytes(value);
                None
            } else {
                Some(vctx.node(value))
            };
            ObjectRec {
                span,
                id: entry.object_id,
                type_id: entry.type_id,
                type_name: vctx.type_name(entry.type_id),
                value: node,
            }
        };
    for entry in &c.object_table {
        match record(&types_sys, data, &mut pos, entry) {
            Ok((span, value)) => {
                painter.paint(span.start, span.end);
                objects.push(make(&mut vctx, span, entry, &value));
            }
            Err(message) => {
                fail = Some((pos, message));
                break;
            }
        }
    }
    let mut edob_at = None;
    if fail.is_none() {
        if c.has_extra_records {
            if data.get(pos..pos + 4) == Some(b"EDOB") {
                edob_at = Some(pos);
                painter.paint(pos, pos + 8);
                let declared = le32(data, pos + 4);
                if declared == u32::try_from(c.extra_table.len()).ok() {
                    let mut p = pos + 8;
                    for entry in &c.extra_table {
                        match record(&types_sys, data, &mut p, entry) {
                            Ok((span, value)) => {
                                painter.paint(span.start, span.end);
                                extra.push(make(&mut vctx, span, entry, &value));
                            }
                            Err(message) => {
                                fail = Some((p, message));
                                break;
                            }
                        }
                    }
                    if fail.is_none() && p != footer_start {
                        fail = Some((
                            p,
                            format!(
                                "the extra records end at byte {p}, not at the footer (byte {footer_start})"
                            ),
                        ));
                    }
                } else {
                    fail = Some((
                        pos,
                        format!(
                            "EDOB at byte {pos} says {} extra records but EDTY has {}",
                            declared.map_or_else(|| "?".to_string(), |d| d.to_string()),
                            c.extra_table.len()
                        ),
                    ));
                }
            } else {
                fail = Some((
                    pos,
                    format!("the object records end at byte {pos}, which is not the EDOB tag"),
                ));
            }
        } else {
            if pos != c.object_data.end {
                fail = Some((
                    pos,
                    format!(
                        "the object records end at byte {pos} but the object data ends at byte {}",
                        c.object_data.end
                    ),
                ));
            }
            if !c.extra_table.is_empty() {
                problems.push(format!(
                    "EDTY lists {} extra records but the file has none after EDOB",
                    c.extra_table.len()
                ));
            }
        }
    }
    if let Some((_, message)) = &fail {
        problems.push(message.clone());
    }
    if objects.len() < c.object_table.len() {
        problems.push(format!(
            "{} of {} object records were not decoded",
            c.object_table.len() - objects.len(),
            c.object_table.len()
        ));
    }

    // Chunks.
    let mut chunks = vec![Chunk {
        tag: "header".into(),
        span: header.span,
        entries: None,
    }];
    for s in &c.sections {
        let entries = match &s.tag {
            b"RFIL" => Some(c.files.len()),
            b"IDNT" => Some(c.idents.len()),
            b"EXTY" => Some(c.external_types.len()),
            b"INTY" => Some(c.types.len()),
            b"EXOB" => Some(c.external_objects.len()),
            b"OBTY" => Some(c.object_table.len()),
            b"EDTY" => Some(c.extra_table.len()),
            b"OBJS" => Some(c.object_record_count as usize),
            _ => None,
        }
        .map(|n| n as u64);
        let span = if &s.tag == b"OBJS" {
            Span {
                start: s.range.start,
                end: edob_at.unwrap_or(s.range.end),
            }
        } else {
            s.range.clone().into()
        };
        chunks.push(Chunk {
            tag: String::from_utf8_lossy(&s.tag).into_owned(),
            span,
            entries,
        });
    }
    if let Some(at) = edob_at {
        chunks.push(Chunk {
            tag: "EDOB".into(),
            span: Span {
                start: at,
                end: footer_start,
            },
            entries: Some(c.extra_table.len() as u64),
        });
    }
    chunks.push(Chunk {
        tag: "METAEND".into(),
        span: Span {
            start: footer_start,
            end: data.len(),
        },
        entries: None,
    });

    // Coverage.
    let undecoded: Vec<Gap> = painter
        .gaps()
        .into_iter()
        .map(|span| {
            let section = c
                .sections
                .iter()
                .find(|s| s.range.start <= span.start && span.start < s.range.end)
                .map_or_else(
                    || "(frame)".to_string(),
                    |s| String::from_utf8_lossy(&s.tag).into_owned(),
                );
            let reason = match &fail {
                Some((at, message)) if span.start <= *at && *at < span.end => message.clone(),
                _ => "no decoder explains these bytes".to_string(),
            };
            Gap {
                span,
                section,
                reason,
            }
        })
        .collect();
    let undecoded_bytes: usize = undecoded.iter().map(|g| g.span.len()).sum();
    let coverage = Coverage {
        total: data.len(),
        decoded: data.len() - undecoded_bytes,
        opaque: vctx.opaque + if c.info.flag.is_some() { 4 } else { 0 },
        undecoded,
    };

    Report {
        label: label.to_string(),
        data,
        header,
        info,
        chunks,
        files,
        idents,
        external_types,
        types,
        array_dims,
        external_objects,
        object_table,
        extra_table,
        objects,
        extra,
        problems,
        coverage,
    }
}
