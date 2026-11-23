//! The type system and the object decoder (roadmap T02-007; rules: `docs/re/ctsemeta-objects.md`).
//!
//! A [`TypeSystem`] is built from the type table of one file. It knows, for every type id, how a
//! value of that type is laid out in the object data, and decodes it into a [`Value`] without any
//! knowledge of the game classes. Unknown shapes make the decode fail; nothing is guessed.

use std::collections::HashMap;

use ss2_core::{Error, Result};

use crate::container::{Container, ShortForm, TypeShape};
use crate::value::{Object, Struct, Value};

const WHAT: &str = "CTSEMETA object data";
/// Deepest nesting of by-value structs and arrays accepted (the real files stay far below).
const MAX_DEPTH: usize = 64;
/// Most elements accepted in a container whose elements occupy no bytes.
const MAX_EMPTY_ELEMENTS: usize = 1_000_000;
const NULL_ID: u32 = 0xFFFF_FFFF;

/// A scalar stored as `1`, `2`, `4` or `8` raw little-endian bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Prim {
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    U64,
    I64,
    F32,
    F64,
    Ident,
}

impl Prim {
    fn size(self) -> usize {
        match self {
            Self::U8 | Self::I8 => 1,
            Self::U16 | Self::I16 => 2,
            Self::U32 | Self::I32 | Self::F32 | Self::Ident => 4,
            Self::U64 | Self::I64 | Self::F64 => 8,
        }
    }

    /// The scalar for a primitive of `size` bytes called `name`: the name decides signedness and
    /// floating point, the size the width.
    fn from_name(name: &str, size: u32) -> Option<Self> {
        Some(match (name, size) {
            ("FLOAT", 4) => Self::F32,
            ("DOUBLE", 8) => Self::F64,
            ("IDENT", 4) => Self::Ident,
            ("SBYTE", 1) => Self::I8,
            ("SWORD", 2) => Self::I16,
            ("SLONG" | "INDEX", 4) => Self::I32,
            ("SQUAD", 8) => Self::I64,
            (_, 1) => Self::U8,
            (_, 2) => Self::U16,
            (_, 4) => Self::U32,
            (_, 8) => Self::U64,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug)]
enum Codec {
    Prim(Prim),
    Enum {
        size: usize,
    },
    Ref,
    String,
    /// A class instance; `marker` is `DTRS` for `CTransString`.
    Struct {
        marker: Option<[u8; 4]>,
    },
    /// `marker`, count, then `elem` values inline.
    List {
        marker: [u8; 4],
        elem: u32,
    },
    /// `DCON`, count, then object ids.
    Objects,
    /// `n` values inline, no marker and no count.
    Fixed {
        n: usize,
        elem: u32,
    },
    Unsupported(String),
}

/// A member of a class after flattening the base classes in front of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlatMember {
    /// Type id of the class that declares the member.
    pub owner: u32,
    /// Name of that class.
    pub owner_name: String,
    /// Member name (often a decimal number).
    pub name: String,
    /// Type id of the member.
    pub type_id: u32,
}

/// The decoding rules of one file, built from its type table.
#[derive(Clone, Debug, Default)]
pub struct TypeSystem {
    codecs: HashMap<u32, Codec>,
    layouts: HashMap<u32, Vec<FlatMember>>,
    min_sizes: HashMap<u32, usize>,
    names: HashMap<u32, String>,
}

fn external_primitive(name: &str) -> Option<Prim> {
    Some(match name {
        "UBYTE" => Prim::U8,
        "SBYTE" => Prim::I8,
        "UWORD" => Prim::U16,
        "SWORD" => Prim::I16,
        "ULONG" | "BOOL" => Prim::U32,
        "SLONG" | "INDEX" => Prim::I32,
        "FLOAT" => Prim::F32,
        "IDENT" => Prim::Ident,
        _ => return None,
    })
}

fn fixed_length(name: &str) -> Option<usize> {
    let inner = name.strip_suffix(']')?;
    inner.rsplit_once('[')?.1.parse().ok()
}

impl TypeSystem {
    /// Build the decoding rules from the type table of `container` (internal and external types).
    pub fn new(container: &Container<'_>) -> Self {
        let mut ts = Self::default();
        for t in &container.types {
            ts.names.insert(t.id, t.name.to_string());
        }
        for t in &container.external_types {
            ts.names.entry(t.id).or_insert_with(|| t.name.to_string());
        }
        // Struct layouts first (they only depend on parents).
        for t in &container.types {
            if matches!(t.shape, TypeShape::Struct { .. }) {
                match Self::flatten(container, t.id) {
                    Ok(layout) => {
                        ts.layouts.insert(t.id, layout);
                    }
                    Err(e) => {
                        ts.codecs.insert(t.id, Codec::Unsupported(e));
                    }
                }
            }
        }
        for t in &container.types {
            if ts.codecs.contains_key(&t.id) {
                continue;
            }
            let codec = ts.resolve(container, t.id, 0);
            ts.codecs.insert(t.id, codec);
        }
        for t in &container.external_types {
            let codec = match external_primitive(&t.name) {
                Some(p) => Codec::Prim(p),
                None => {
                    Codec::Unsupported(format!("external type `{}` has no known encoding", t.name))
                }
            };
            ts.codecs.entry(t.id).or_insert(codec);
        }
        let ids: Vec<u32> = ts.codecs.keys().copied().collect();
        for id in ids {
            let mut stack = Vec::new();
            let size = ts.compute_min_size(id, &mut stack);
            ts.min_sizes.insert(id, size);
        }
        ts
    }

    /// The members of class `type_id`, **base class first**, as decoded into [`Struct::values`].
    pub fn members(&self, type_id: u32) -> Option<&[FlatMember]> {
        self.layouts.get(&type_id).map(Vec::as_slice)
    }

    /// The name of type `type_id`.
    pub fn type_name(&self, type_id: u32) -> Option<&str> {
        self.names.get(&type_id).map(String::as_str)
    }

    /// The value of member `name` declared by class `owner_name` in `s` (member names alone are
    /// not unique: a derived class may reuse `"1"`).
    pub fn member<'v>(&self, s: &'v Struct, owner_name: &str, name: &str) -> Option<&'v Value> {
        let layout = self.layouts.get(&s.type_id)?;
        let i = layout
            .iter()
            .position(|m| m.owner_name == owner_name && m.name == name)?;
        s.values.get(i)
    }

    fn flatten(c: &Container<'_>, type_id: u32) -> std::result::Result<Vec<FlatMember>, String> {
        let mut chain = Vec::new();
        let mut cursor = Some(type_id);
        while let Some(id) = cursor {
            if chain.len() > MAX_DEPTH {
                return Err("class inheritance is too deep or circular".to_string());
            }
            let t = c
                .type_def(id)
                .ok_or_else(|| format!("base class {id} is not in the type table"))?;
            let TypeShape::Struct { parent, .. } = &t.shape else {
                return Err(format!("base `{}` is not a class", t.name));
            };
            chain.push(t);
            cursor = *parent;
        }
        let mut out = Vec::new();
        for t in chain.iter().rev() {
            if let TypeShape::Struct { members, .. } = &t.shape {
                for m in members {
                    out.push(FlatMember {
                        owner: t.id,
                        owner_name: t.name.to_string(),
                        name: m.name.to_string(),
                        type_id: m.type_id,
                    });
                }
            }
        }
        Ok(out)
    }

    fn resolve(&self, c: &Container<'_>, id: u32, depth: usize) -> Codec {
        if depth > MAX_DEPTH {
            return Codec::Unsupported("alias chain is too deep or circular".to_string());
        }
        let Some(t) = c.type_def(id) else {
            return match c.external_type_name(id).and_then(external_primitive) {
                Some(p) => Codec::Prim(p),
                None => Codec::Unsupported(format!(
                    "external type {id} `{}` has no known encoding",
                    c.external_type_name(id).unwrap_or("?")
                )),
            };
        };
        match &t.shape {
            TypeShape::Struct { .. } => match t.name.as_ref() {
                // Declared without members but stored as one object id.
                "CMetaHandle" => Codec::Ref,
                "CTransString" => Codec::Struct {
                    marker: Some(*b"DTRS"),
                },
                _ => Codec::Struct { marker: None },
            },
            TypeShape::Short { .. } => {
                if t.name == "CString" {
                    return Codec::String;
                }
                match t.short_form() {
                    Some(ShortForm::Enumeration { size }) => Codec::Enum {
                        size: size as usize,
                    },
                    Some(
                        ShortForm::Pointer { .. }
                        | ShortForm::SmartPointer { .. }
                        | ShortForm::Handle { .. },
                    ) => Codec::Ref,
                    Some(ShortForm::FixedArray { element }) => match fixed_length(&t.name) {
                        Some(n) => Codec::Fixed { n, elem: element },
                        None => Codec::Unsupported(format!(
                            "fixed array `{}` has no length in its name",
                            t.name
                        )),
                    },
                    Some(ShortForm::StaticArray { element }) => Codec::List {
                        marker: *b"STAR",
                        elem: element,
                    },
                    Some(ShortForm::StaticStackArray { element }) => Codec::List {
                        marker: *b"SSAR",
                        elem: element,
                    },
                    Some(ShortForm::DynamicContainer { .. }) => Codec::Objects,
                    Some(ShortForm::Primitive { size, .. }) => {
                        match Prim::from_name(&t.name, size) {
                            Some(p) => Codec::Prim(p),
                            None => Codec::Unsupported(format!(
                                "primitive `{}` of {size} bytes",
                                t.name
                            )),
                        }
                    }
                    Some(ShortForm::Alias { target }) => self.resolve(c, target, depth + 1),
                    None => Codec::Unsupported(format!("type `{}`: unknown block form", t.name)),
                }
            }
        }
    }

    fn compute_min_size(&self, id: u32, stack: &mut Vec<u32>) -> usize {
        if let Some(&s) = self.min_sizes.get(&id) {
            return s;
        }
        if stack.contains(&id) || stack.len() > MAX_DEPTH {
            return 0;
        }
        stack.push(id);
        let size = match self.codecs.get(&id) {
            Some(Codec::Prim(p)) => p.size(),
            Some(Codec::Enum { size }) => *size,
            Some(Codec::Ref | Codec::String) => 4,
            Some(Codec::List { .. } | Codec::Objects) => 8,
            Some(Codec::Fixed { n, elem }) => n.saturating_mul(self.compute_min_size(*elem, stack)),
            Some(Codec::Struct { marker }) => {
                let members: Vec<u32> = self
                    .layouts
                    .get(&id)
                    .map(|l| l.iter().map(|m| m.type_id).collect())
                    .unwrap_or_default();
                let body: usize = members
                    .iter()
                    .map(|&m| self.compute_min_size(m, stack))
                    .sum();
                body + marker.map_or(0, |m| m.len())
            }
            Some(Codec::Unsupported(_)) | None => 0,
        };
        stack.pop();
        size
    }

    fn fail<T>(pos: usize, detail: impl std::fmt::Display) -> Result<T> {
        Err(Error::invalid(WHAT, format!("at byte {pos}: {detail}")))
    }

    fn take<'d>(data: &'d [u8], pos: &mut usize, n: usize) -> Result<&'d [u8]> {
        match data.get(*pos..).and_then(|rest| rest.get(..n)) {
            Some(b) => {
                *pos += n;
                Ok(b)
            }
            None => Self::fail(
                *pos,
                format!("{n} bytes wanted, the data ends (truncated file)"),
            ),
        }
    }

    fn read_u32(data: &[u8], pos: &mut usize) -> Result<u32> {
        let b = Self::take(data, pos, 4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// An element count, checked against the bytes left so a hostile count cannot allocate.
    fn read_count(&self, data: &[u8], pos: &mut usize, elem: u32) -> Result<usize> {
        let at = *pos;
        let n = Self::read_u32(data, pos)? as usize;
        let min = self.min_sizes.get(&elem).copied().unwrap_or(0);
        let remaining = data.len().saturating_sub(*pos);
        let ok = if min == 0 {
            n <= MAX_EMPTY_ELEMENTS
        } else {
            n.checked_mul(min).is_some_and(|b| b <= remaining)
        };
        if !ok {
            return Self::fail(
                at,
                format!("element count {n} does not fit in the {remaining} bytes left"),
            );
        }
        Ok(n)
    }

    fn prim(p: Prim, data: &[u8], pos: &mut usize) -> Result<Value> {
        let b = Self::take(data, pos, p.size())?;
        Ok(match p {
            Prim::U8 => Value::U8(b[0]),
            Prim::I8 => Value::I8(b[0] as i8),
            Prim::U16 => Value::U16(u16::from_le_bytes([b[0], b[1]])),
            Prim::I16 => Value::I16(i16::from_le_bytes([b[0], b[1]])),
            Prim::U32 => Value::U32(u32::from_le_bytes([b[0], b[1], b[2], b[3]])),
            Prim::I32 => Value::I32(i32::from_le_bytes([b[0], b[1], b[2], b[3]])),
            Prim::F32 => Value::F32(f32::from_le_bytes([b[0], b[1], b[2], b[3]])),
            Prim::Ident => Value::Ident(u32::from_le_bytes([b[0], b[1], b[2], b[3]])),
            Prim::U64 | Prim::I64 | Prim::F64 => {
                let a: [u8; 8] = b.try_into().expect("8 bytes were taken");
                match p {
                    Prim::U64 => Value::U64(u64::from_le_bytes(a)),
                    Prim::I64 => Value::I64(i64::from_le_bytes(a)),
                    _ => Value::F64(f64::from_le_bytes(a)),
                }
            }
        })
    }

    /// Decode one value of type `type_id` from `data` at `*pos`, advancing `*pos` past it.
    ///
    /// # Errors
    /// [`Error::Invalid`] on truncated data, an unknown type shape, a container marker that does
    /// not match the type (a sign that alignment was lost), an implausible count, or nesting deeper
    /// than 64 levels.
    pub fn decode_value(&self, data: &[u8], type_id: u32, pos: &mut usize) -> Result<Value> {
        self.decode_at(data, type_id, pos, 0)
    }

    fn decode_at(&self, data: &[u8], type_id: u32, pos: &mut usize, depth: usize) -> Result<Value> {
        if depth > MAX_DEPTH {
            return Self::fail(*pos, "values are nested too deeply");
        }
        let Some(codec) = self.codecs.get(&type_id) else {
            return Self::fail(*pos, format!("type {type_id} is not defined"));
        };
        match codec {
            Codec::Prim(p) => Self::prim(*p, data, pos),
            Codec::Enum { size } => {
                let b = Self::take(data, pos, *size)?;
                if !matches!(*size, 1 | 2 | 4) {
                    return Self::fail(*pos, format!("enumeration stored in {size} bytes"));
                }
                let mut value = 0u32;
                for (i, byte) in b.iter().enumerate() {
                    value |= u32::from(*byte) << (8 * i);
                }
                Ok(Value::Enum { type_id, value })
            }
            Codec::Ref => {
                let id = Self::read_u32(data, pos)?;
                Ok(Value::Ref((id != NULL_ID).then_some(id)))
            }
            Codec::String => {
                let len = Self::read_u32(data, pos)? as usize;
                let b = Self::take(data, pos, len)?;
                Ok(Value::String(b.iter().map(|&c| char::from(c)).collect()))
            }
            Codec::Struct { marker } => {
                if let Some(m) = marker {
                    let at = *pos;
                    if Self::take(data, pos, 4)? != m {
                        return Self::fail(
                            at,
                            format!("expected marker `{}`", String::from_utf8_lossy(m)),
                        );
                    }
                }
                let Some(layout) = self.layouts.get(&type_id) else {
                    return Self::fail(
                        *pos,
                        format!(
                            "class {type_id} `{}` has no usable layout",
                            self.type_name(type_id).unwrap_or("?")
                        ),
                    );
                };
                let mut values = Vec::with_capacity(layout.len());
                for m in layout {
                    values.push(self.decode_at(data, m.type_id, pos, depth + 1)?);
                }
                Ok(Value::Struct(Struct { type_id, values }))
            }
            Codec::List { marker, elem } => {
                let at = *pos;
                if Self::take(data, pos, 4)? != marker {
                    return Self::fail(
                        at,
                        format!("expected marker `{}`", String::from_utf8_lossy(marker)),
                    );
                }
                let n = self.read_count(data, pos, *elem)?;
                if matches!(self.codecs.get(elem), Some(Codec::Prim(Prim::U8))) {
                    return Ok(Value::Bytes(Self::take(data, pos, n)?.to_vec()));
                }
                let mut items = Vec::with_capacity(n);
                for _ in 0..n {
                    items.push(self.decode_at(data, *elem, pos, depth + 1)?);
                }
                Ok(Value::List(items))
            }
            Codec::Objects => {
                let at = *pos;
                if Self::take(data, pos, 4)? != b"DCON" {
                    return Self::fail(at, "expected marker `DCON`");
                }
                let n = self.read_count_of(data, pos, 4)?;
                let mut items = Vec::with_capacity(n);
                for _ in 0..n {
                    let id = Self::read_u32(data, pos)?;
                    items.push(Value::Ref((id != NULL_ID).then_some(id)));
                }
                Ok(Value::List(items))
            }
            Codec::Fixed { n, elem } => {
                let mut items = Vec::with_capacity((*n).min(4096));
                for _ in 0..*n {
                    items.push(self.decode_at(data, *elem, pos, depth + 1)?);
                }
                Ok(Value::List(items))
            }
            Codec::Unsupported(why) => {
                Self::fail(*pos, format!("unsupported type {type_id}: {why}"))
            }
        }
    }

    fn read_count_of(&self, data: &[u8], pos: &mut usize, elem_size: usize) -> Result<usize> {
        let at = *pos;
        let n = Self::read_u32(data, pos)? as usize;
        let remaining = data.len().saturating_sub(*pos);
        if n.checked_mul(elem_size).is_none_or(|b| b > remaining) {
            return Self::fail(
                at,
                format!("element count {n} does not fit in the {remaining} bytes left"),
            );
        }
        Ok(n)
    }
}

impl TypeSystem {
    fn mismatch<T>(type_id: u32, what: &str, value: &Value) -> Result<T> {
        Err(Error::invalid(
            "CTSEMETA value to write",
            format!("type {type_id} expects {what}, got {value:?}"),
        ))
    }

    /// Append the encoding of `value` as a value of type `type_id` to `out`: the exact inverse of
    /// [`TypeSystem::decode_value`] (writing what was decoded gives back the same bytes).
    ///
    /// # Errors
    /// [`Error::Invalid`] if `value` does not fit the type: another variant, a fixed array of the
    /// wrong length, a struct of another class or with the wrong number of members, an enumeration
    /// value too big for its size, a string with a character above U+00FF (the files are Latin-1),
    /// or an unsupported type.
    pub fn encode_value(&self, out: &mut Vec<u8>, type_id: u32, value: &Value) -> Result<()> {
        self.encode_at(out, type_id, value, 0)
    }

    fn put_str(out: &mut Vec<u8>, s: &str) -> Result<()> {
        let mut bytes = Vec::with_capacity(s.len());
        for c in s.chars() {
            match u8::try_from(u32::from(c)) {
                Ok(b) => bytes.push(b),
                Err(_) => {
                    return Err(Error::invalid(
                        "CTSEMETA value to write",
                        format!("the character {c:?} cannot be stored (the files are Latin-1)"),
                    ));
                }
            }
        }
        let len = u32::try_from(bytes.len())
            .map_err(|_| Error::invalid("CTSEMETA value to write", "string longer than 4 GiB"))?;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&bytes);
        Ok(())
    }

    fn put_count(out: &mut Vec<u8>, n: usize) -> Result<()> {
        let n = u32::try_from(n).map_err(|_| {
            Error::invalid("CTSEMETA value to write", "more than 4 billion elements")
        })?;
        out.extend_from_slice(&n.to_le_bytes());
        Ok(())
    }

    fn encode_at(
        &self,
        out: &mut Vec<u8>,
        type_id: u32,
        value: &Value,
        depth: usize,
    ) -> Result<()> {
        if depth > MAX_DEPTH {
            return Err(Error::invalid(
                "CTSEMETA value to write",
                "values are nested too deeply",
            ));
        }
        let Some(codec) = self.codecs.get(&type_id) else {
            return Err(Error::invalid(
                "CTSEMETA value to write",
                format!("type {type_id} is not defined"),
            ));
        };
        match (codec, value) {
            (Codec::Prim(Prim::U8), Value::U8(v)) => out.push(*v),
            (Codec::Prim(Prim::I8), Value::I8(v)) => out.push(*v as u8),
            (Codec::Prim(Prim::U16), Value::U16(v)) => out.extend_from_slice(&v.to_le_bytes()),
            (Codec::Prim(Prim::I16), Value::I16(v)) => out.extend_from_slice(&v.to_le_bytes()),
            (Codec::Prim(Prim::U32), Value::U32(v))
            | (Codec::Prim(Prim::Ident), Value::Ident(v)) => {
                out.extend_from_slice(&v.to_le_bytes());
            }
            (Codec::Prim(Prim::I32), Value::I32(v)) => out.extend_from_slice(&v.to_le_bytes()),
            (Codec::Prim(Prim::U64), Value::U64(v)) => out.extend_from_slice(&v.to_le_bytes()),
            (Codec::Prim(Prim::I64), Value::I64(v)) => out.extend_from_slice(&v.to_le_bytes()),
            (Codec::Prim(Prim::F32), Value::F32(v)) => out.extend_from_slice(&v.to_le_bytes()),
            (Codec::Prim(Prim::F64), Value::F64(v)) => out.extend_from_slice(&v.to_le_bytes()),
            (Codec::Prim(p), v) => return Self::mismatch(type_id, &format!("a {p:?} scalar"), v),
            (Codec::Enum { size }, Value::Enum { value: v, .. }) => {
                if *size < 4 && u64::from(*v) >> (8 * *size) != 0 {
                    return Self::mismatch(
                        type_id,
                        &format!("a value that fits {size} byte(s)"),
                        value,
                    );
                }
                out.extend_from_slice(&v.to_le_bytes()[..*size]);
            }
            (Codec::Ref, Value::Ref(id)) => {
                out.extend_from_slice(&id.unwrap_or(NULL_ID).to_le_bytes());
            }
            (Codec::String, Value::String(s)) => Self::put_str(out, s)?,
            (Codec::Struct { marker }, Value::Struct(s)) => {
                let layout = self.layouts.get(&type_id);
                if s.type_id != type_id || layout.is_none_or(|l| l.len() != s.values.len()) {
                    return Self::mismatch(
                        type_id,
                        "a struct of this class with all its members",
                        value,
                    );
                }
                if let Some(m) = marker {
                    out.extend_from_slice(m);
                }
                for (m, v) in layout.into_iter().flatten().zip(&s.values) {
                    self.encode_at(out, m.type_id, v, depth + 1)?;
                }
            }
            (Codec::List { marker, elem }, Value::Bytes(b)) => {
                if !matches!(self.codecs.get(elem), Some(Codec::Prim(Prim::U8))) {
                    return Self::mismatch(type_id, "a list", value);
                }
                out.extend_from_slice(marker);
                Self::put_count(out, b.len())?;
                out.extend_from_slice(b);
            }
            (Codec::List { marker, elem }, Value::List(items)) => {
                out.extend_from_slice(marker);
                Self::put_count(out, items.len())?;
                for v in items {
                    self.encode_at(out, *elem, v, depth + 1)?;
                }
            }
            (Codec::Objects, Value::List(items)) => {
                out.extend_from_slice(b"DCON");
                Self::put_count(out, items.len())?;
                for v in items {
                    let Value::Ref(id) = v else {
                        return Self::mismatch(type_id, "object ids", v);
                    };
                    out.extend_from_slice(&id.unwrap_or(NULL_ID).to_le_bytes());
                }
            }
            (Codec::Fixed { n, elem }, Value::List(items)) => {
                if items.len() != *n {
                    return Self::mismatch(type_id, &format!("{n} elements"), value);
                }
                for v in items {
                    self.encode_at(out, *elem, v, depth + 1)?;
                }
            }
            (Codec::Unsupported(why), _) => {
                return Err(Error::invalid(
                    "CTSEMETA value to write",
                    format!("unsupported type {type_id}: {why}"),
                ));
            }
            (_, v) => return Self::mismatch(type_id, "another kind of value", v),
        }
        Ok(())
    }
}

/// All objects of a file.
#[derive(Clone, Debug)]
pub struct Decoded {
    /// The decoding rules of the file.
    pub types: TypeSystem,
    /// The records of `OBJS`, in file order.
    pub objects: Vec<Object>,
    /// The extra records after `EDOB` (editor and bake data), in file order.
    pub extra: Vec<Object>,
}

impl Container<'_> {
    /// Decode every object of the file into [`Value`] trees: the `OBJS` records, then the extra
    /// records after `EDOB`.
    ///
    /// Checks the rules of the spec: each record header equals its `OBTY` (or `EDTY`) entry, the
    /// main records end exactly at the `EDOB` tag, and the extra records end exactly at the
    /// footer.
    ///
    /// # Errors
    /// [`Error::Invalid`] naming the byte offset of the first problem.
    pub fn decode_objects(&self) -> Result<Decoded> {
        let types = TypeSystem::new(self);
        let data = self.data;
        let mut pos = self.object_data.start;
        let mut objects = Vec::with_capacity(self.object_table.len());
        for entry in &self.object_table {
            objects.push(Self::record(
                &types,
                data,
                &mut pos,
                entry.object_id,
                entry.type_id,
            )?);
        }
        let edob_at = pos;
        if data.get(pos..pos + 4) != Some(b"EDOB") {
            return TypeSystem::fail(
                pos,
                "the object records do not end at the `EDOB` tag (alignment lost?)",
            );
        }
        pos += 4;
        let count = TypeSystem::read_u32(data, &mut pos)? as usize;
        if count != self.extra_table.len() {
            return TypeSystem::fail(
                edob_at,
                format!(
                    "EDOB says {count} extra records but EDTY has {}",
                    self.extra_table.len()
                ),
            );
        }
        let mut extra = Vec::with_capacity(count);
        for entry in &self.extra_table {
            extra.push(Self::record(
                &types,
                data,
                &mut pos,
                entry.object_id,
                entry.type_id,
            )?);
        }
        if data.get(pos..) != Some(b"METAEND ") {
            return TypeSystem::fail(pos, "the extra records do not end at the footer `METAEND `");
        }
        Ok(Decoded {
            types,
            objects,
            extra,
        })
    }

    fn record(
        types: &TypeSystem,
        data: &[u8],
        pos: &mut usize,
        id: u32,
        type_id: u32,
    ) -> Result<Object> {
        let at = *pos;
        let got_id = TypeSystem::read_u32(data, pos)?;
        let got_type = TypeSystem::read_u32(data, pos)?;
        if (got_id, got_type) != (id, type_id) {
            return TypeSystem::fail(
                at,
                format!(
                    "record header ({got_id}, {got_type}) differs from the table entry ({id}, {type_id})"
                ),
            );
        }
        let value = types.decode_value(data, type_id, pos)?;
        Ok(Object { id, type_id, value })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, SampleIds, TypeBlock};

    fn decode(bytes: &[u8]) -> Result<Decoded> {
        Container::parse(bytes)?.decode_objects()
    }

    #[test]
    fn decodes_the_sample_objects() {
        let bytes = MetaBuilder::sample().build().unwrap();
        let d = decode(&bytes).unwrap();
        assert_eq!(d.objects.len(), 2);
        let item = d.objects[0].value.as_struct().unwrap();
        assert_eq!(item.values.len(), 5);
        assert_eq!(item.values[0], Value::I32(7));
        assert_eq!(item.values[1], Value::F32(30.0));
        assert_eq!(item.values[2], Value::Ident(1));
        assert_eq!(
            item.values[3],
            Value::Enum {
                type_id: SampleIds::MODE,
                value: 2
            }
        );
        assert_eq!(
            item.values[4],
            Value::List(vec![Value::F32(1.0), Value::F32(2.0), Value::F32(3.0)])
        );
        assert_eq!(
            d.types.member(item, "CTestItem", "2"),
            Some(&Value::F32(30.0))
        );
        assert_eq!(d.types.member(item, "CTestItem", "9"), None);

        // The holder lists the base class members first, then its own.
        let holder = d.objects[1].value.as_struct().unwrap();
        assert_eq!(holder.values.len(), 7);
        let members = d.types.members(SampleIds::HOLDER).unwrap();
        assert_eq!(members[0].owner_name, "CTestItem");
        assert_eq!(members[5].owner_name, "CTestHolder");
        assert_eq!(
            d.types.member(holder, "CTestHolder", "1"),
            Some(&Value::Ref(Some(0)))
        );
        assert_eq!(
            d.types.member(holder, "CTestHolder", "2"),
            Some(&Value::List(vec![Value::Ref(Some(0))]))
        );
        assert_eq!(d.types.type_name(SampleIds::HOLDER), Some("CTestHolder"));
        assert!(d.extra.is_empty());
    }

    fn one_class(members: &[(&str, u32)], prims: &[TypeBlock], body: Vec<u8>) -> Vec<u8> {
        let mut b = MetaBuilder::new();
        for p in prims {
            b.type_block(p.clone());
        }
        b.type_block(TypeBlock::class(100, "CThing", 1, 5, None, members));
        b.object(0, 100, body);
        b.build().unwrap()
    }

    #[test]
    fn scalar_kinds_follow_their_names() {
        let prims = [
            TypeBlock::primitive(0, "FLOAT", 4, 4),
            TypeBlock::primitive(1, "SLONG", 4, 4),
            TypeBlock::primitive(2, "UBYTE", 1, 1),
            TypeBlock::primitive(3, "SBYTE", 1, 1),
            TypeBlock::primitive(4, "UWORD", 2, 2),
            TypeBlock::primitive(5, "SQUAD", 8, 8),
            TypeBlock::primitive(6, "DOUBLE", 8, 8),
            TypeBlock::alias(7, "BOOL", 8 + 100),
            TypeBlock::primitive(108, "ULONG", 4, 4),
        ];
        let bytes = one_class(
            &[
                ("a", 0),
                ("b", 1),
                ("c", 2),
                ("d", 3),
                ("e", 4),
                ("f", 5),
                ("g", 6),
                ("h", 7),
            ],
            &prims,
            ObjectBytes::new()
                .f32(1.5)
                .i32(-2)
                .u8(250)
                .u8(0xFE)
                .u16(65_000)
                .u64(u64::MAX)
                .u64(2.5f64.to_bits())
                .u32(1)
                .finish(),
        );
        let d = decode(&bytes).unwrap();
        let s = d.objects[0].value.as_struct().unwrap();
        assert_eq!(
            s.values,
            [
                Value::F32(1.5),
                Value::I32(-2),
                Value::U8(250),
                Value::I8(-2),
                Value::U16(65_000),
                Value::I64(-1),
                Value::F64(2.5),
                Value::U32(1),
            ]
        );
        assert_eq!(s.values[1].as_i64(), Some(-2));
        assert_eq!(s.values[6].as_f64(), Some(2.5));
    }

    #[test]
    fn strings_trans_strings_and_byte_arrays() {
        let mut b = MetaBuilder::new();
        b.type_block(TypeBlock::primitive(0, "UBYTE", 1, 1));
        b.type_block(TypeBlock::primitive(1, "CString", 4, 4));
        b.type_block(TypeBlock::class(
            2,
            "CTransString",
            1,
            5,
            None,
            &[("1", 1), ("2", 1)],
        ));
        b.type_block(TypeBlock::short(3, "CStaticArray<UBYTE>", 0, 6, 0));
        b.type_block(TypeBlock::class(
            4,
            "CHolder",
            1,
            5,
            None,
            &[("1", 1), ("2", 2), ("3", 3)],
        ));
        b.object(
            0,
            4,
            ObjectBytes::new()
                .cstring("caf\u{e9}")
                .trans_string("Menu.Key", "Text")
                .array_header(b"STAR", 3)
                .raw(&[1, 2, 3])
                .finish(),
        );
        let d = decode(&b.build().unwrap()).unwrap();
        let s = d.objects[0].value.as_struct().unwrap();
        assert_eq!(
            s.values[0].as_str(),
            Some("caf\u{c3}\u{a9}"),
            "bytes are Latin-1 chars"
        );
        let ts = s.values[1].as_struct().unwrap();
        assert_eq!(
            ts.values,
            [
                Value::String("Menu.Key".into()),
                Value::String("Text".into())
            ]
        );
        assert_eq!(s.values[2], Value::Bytes(vec![1, 2, 3]));
    }

    #[test]
    fn a_wrong_marker_is_reported_as_lost_alignment() {
        let mut b = MetaBuilder::new();
        b.type_block(TypeBlock::primitive(0, "SLONG", 4, 4));
        b.type_block(TypeBlock::short(1, "CStaticArray<SLONG>", 0, 6, 0));
        b.type_block(TypeBlock::class(2, "CThing", 1, 5, None, &[("1", 1)]));
        b.object(0, 2, ObjectBytes::new().array_header(b"SSAR", 0).finish());
        let err = decode(&b.build().unwrap()).unwrap_err().to_string();
        assert!(err.contains("expected marker `STAR`"), "{err}");
    }

    #[test]
    fn cmetahandle_is_four_bytes_although_declared_empty() {
        let mut b = MetaBuilder::new();
        b.type_block(TypeBlock::class(0, "CMetaHandle", 1, 5, None, &[]));
        b.type_block(TypeBlock::class(1, "CEmpty", 1, 5, None, &[]));
        b.type_block(TypeBlock::primitive(2, "SLONG", 4, 4));
        b.type_block(TypeBlock::class(
            3,
            "CThing",
            1,
            5,
            None,
            &[("1", 0), ("2", 1), ("3", 2)],
        ));
        b.object(0, 3, ObjectBytes::new().u32(7).i32(9).finish());
        let d = decode(&b.build().unwrap()).unwrap();
        let s = d.objects[0].value.as_struct().unwrap();
        assert_eq!(s.values[0], Value::Ref(Some(7)));
        assert_eq!(
            s.values[1],
            Value::Struct(Struct {
                type_id: 1,
                values: vec![]
            })
        );
        assert_eq!(s.values[2], Value::I32(9));
    }

    #[test]
    fn hostile_counts_and_nesting_are_rejected() {
        let mut b = MetaBuilder::new();
        b.type_block(TypeBlock::primitive(0, "SLONG", 4, 4));
        b.type_block(TypeBlock::short(1, "CStaticArray<SLONG>", 0, 6, 0));
        b.type_block(TypeBlock::class(2, "CThing", 1, 5, None, &[("1", 1)]));
        b.object(
            0,
            2,
            ObjectBytes::new().array_header(b"STAR", u32::MAX).finish(),
        );
        let err = decode(&b.build().unwrap()).unwrap_err().to_string();
        assert!(err.contains("does not fit"), "{err}");

        // A class that contains itself by value cannot be decoded and must not overflow the stack.
        let mut b = MetaBuilder::new();
        b.type_block(TypeBlock::class(0, "CLoop", 1, 5, None, &[("1", 0)]));
        b.object(0, 0, Vec::new());
        let err = decode(&b.build().unwrap()).unwrap_err().to_string();
        assert!(err.contains("nested too deeply"), "{err}");
    }

    #[test]
    fn unknown_shapes_fail_loudly() {
        let mut b = MetaBuilder::new();
        b.external_type(50, "CMysteryClass");
        b.type_block(TypeBlock::class(0, "CThing", 1, 5, None, &[("1", 50)]));
        b.object(0, 0, Vec::new());
        let err = decode(&b.build().unwrap()).unwrap_err().to_string();
        assert!(err.contains("CMysteryClass"), "{err}");
    }

    #[test]
    fn record_headers_and_the_edob_tag_are_checked() {
        let bytes = MetaBuilder::sample().build().unwrap();
        let c = Container::parse(&bytes).unwrap();
        // Change the object id in the first record header.
        let mut bad = bytes.clone();
        bad[c.object_data.start] ^= 0x40;
        assert!(
            decode(&bad)
                .unwrap_err()
                .to_string()
                .contains("differs from the table entry")
        );
        // One byte too many in the object data: the cursor misses EDOB.
        let mut long = bytes.clone();
        long.insert(c.object_data.end, 0);
        assert!(decode(&long).is_err());
    }

    #[test]
    fn extra_records_after_edob_decode_and_must_match_edty() {
        let build = |edty_id: u32| {
            let mut b = MetaBuilder::new();
            b.type_block(TypeBlock::primitive(0, "SLONG", 4, 4));
            b.type_block(TypeBlock::class(1, "CEditThing", 1, 5, None, &[("1", 0)]));
            b.object(0, 1, ObjectBytes::new().i32(1).finish());
            b.edty_pair(edty_id, 1);
            b.after_edob(1, &ObjectBytes::new().u32(50).u32(1).i32(77).finish());
            b.build().unwrap()
        };
        let d = decode(&build(50)).unwrap();
        assert_eq!(d.objects.len(), 1);
        assert_eq!(d.extra.len(), 1);
        assert_eq!(d.extra[0].id, 50);
        assert_eq!(
            d.extra[0].value.as_struct().unwrap().values,
            [Value::I32(77)]
        );
        let err = decode(&build(51)).unwrap_err().to_string();
        assert!(err.contains("differs from the table entry"), "{err}");
    }

    #[test]
    fn truncations_never_panic() {
        let bytes = MetaBuilder::sample().build().unwrap();
        for len in 0..bytes.len() {
            let _ = decode(&bytes[..len]);
        }
    }

    /// Data-gated: every object of every file of the retail install decodes, ends where the spec
    /// says, and the extra records end on the footer. Skips without `SS2_DATA_DIR`.
    #[test]
    fn every_object_of_the_real_install_decodes() {
        use ss2_vfs::{EntryKind, Vfs, VfsPath};
        let dir = ss2_testkit::require_game_data!();
        let vfs = ss2_vfs::mount_install(&dir).unwrap();
        let mut stack = vec![VfsPath::root()];
        let (mut files, mut objects, mut extra) = (0usize, 0usize, 0usize);
        let mut failures = Vec::new();
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
                    || &head != b"CTSEMETA"
                {
                    continue;
                }
                let bytes = vfs.read(&path).unwrap();
                match Container::parse(&bytes).and_then(|c| c.decode_objects()) {
                    Ok(d) => {
                        files += 1;
                        objects += d.objects.len();
                        extra += d.extra.len();
                    }
                    Err(e) => failures.push(format!("{}: {e}", path.as_str())),
                }
            }
        }
        eprintln!(
            "decoded {files} files, {objects} objects, {extra} extra records; {} failures",
            failures.len()
        );
        assert!(
            failures.is_empty(),
            "first failures: {:?}",
            &failures[..failures.len().min(5)]
        );
        assert!(files > 8000, "only {files} files");
    }
}
