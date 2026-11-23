//! The generic value tree that CTSEMETA objects decode into (roadmap T02-007).

/// A decoded value. Class instances keep only their member values: the names come from the
/// [`crate::TypeSystem`] (so a file with 800,000 objects does not store 800,000 copies of every
/// member name).
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// `UBYTE` and any 1-byte primitive not known to be signed.
    U8(u8),
    /// `SBYTE`.
    I8(i8),
    /// `UWORD` and any 2-byte primitive not known to be signed.
    U16(u16),
    /// `SWORD`.
    I16(i16),
    /// `ULONG`, `BOOL`, `COLOR` and any 4-byte primitive not known to be signed or a float.
    U32(u32),
    /// `SLONG`, `INDEX` (−1 means none).
    I32(i32),
    /// An 8-byte primitive not known to be signed.
    U64(u64),
    /// `SQUAD`.
    I64(i64),
    /// `FLOAT`.
    F32(f32),
    /// `DOUBLE`.
    F64(f64),
    /// `IDENT`: an index into the file's identifier table (`0` is the empty identifier).
    Ident(u32),
    /// An enumeration: the id of its type and the stored value.
    Enum {
        /// Type id of the enumeration.
        type_id: u32,
        /// The stored value (1, 2 or 4 bytes, zero-extended).
        value: u32,
    },
    /// A raw pointer, `Ptr<T>`, `Handle<T>` or `CMetaHandle`: an object id, `None` for null.
    Ref(Option<u32>),
    /// `CString` (Latin-1 text).
    String(String),
    /// A class instance stored by value or as an object.
    Struct(Struct),
    /// The elements of a fixed array, `CStaticArray<T>` or `CStaticStackArray<T>`, or the object
    /// ids of a `CDynamicContainer<T>` (each element then a [`Value::Ref`]).
    List(Vec<Value>),
    /// An array of 1-byte elements (`CStaticArray<UBYTE>`), kept as raw bytes.
    Bytes(Vec<u8>),
}

impl Value {
    /// The object id if this is a non-null [`Value::Ref`].
    pub fn as_ref_id(&self) -> Option<u32> {
        match self {
            Self::Ref(id) => *id,
            _ => None,
        }
    }

    /// The value as `i64` if it is an integer, an identifier or an enumeration.
    pub fn as_i64(&self) -> Option<i64> {
        Some(match self {
            Self::U8(v) => i64::from(*v),
            Self::I8(v) => i64::from(*v),
            Self::U16(v) => i64::from(*v),
            Self::I16(v) => i64::from(*v),
            Self::U32(v) | Self::Ident(v) => i64::from(*v),
            Self::I32(v) => i64::from(*v),
            Self::Enum { value, .. } => i64::from(*value),
            Self::I64(v) => *v,
            Self::U64(v) => i64::try_from(*v).ok()?,
            _ => return None,
        })
    }

    /// The value as `f64` if it is a float.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::F32(v) => Some(f64::from(*v)),
            Self::F64(v) => Some(*v),
            _ => None,
        }
    }

    /// The text if this is a [`Value::String`].
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// The instance if this is a [`Value::Struct`].
    pub fn as_struct(&self) -> Option<&Struct> {
        match self {
            Self::Struct(s) => Some(s),
            _ => None,
        }
    }
}

/// A class instance: its type and the values of all its members, **base class first** (the order
/// of [`crate::TypeSystem::members`]).
#[derive(Clone, Debug, PartialEq)]
pub struct Struct {
    /// Type id of the instance.
    pub type_id: u32,
    /// One value per flattened member.
    pub values: Vec<Value>,
}

/// One record of `OBJS` (or an extra record after `EDOB`).
#[derive(Clone, Debug, PartialEq)]
pub struct Object {
    /// Object id.
    pub id: u32,
    /// Type id.
    pub type_id: u32,
    /// The decoded members.
    pub value: Value,
}
