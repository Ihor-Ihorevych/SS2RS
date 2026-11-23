//! Typed access to decoded values (roadmap T02-008b).
//!
//! [`FromValue`] turns a [`Value`] into a Rust type; `#[derive(MetaDeserialize)]` implements it
//! for a struct by reading named members of a class instance:
//!
//! ```
//! use ss2_meta::{Container, MetaDeserialize, ObjectRef};
//! use ss2_testkit::fixtures::ctsemeta::MetaBuilder;
//!
//! #[derive(MetaDeserialize)]
//! #[meta(class = "CTestItem")]
//! struct Item {
//!     #[meta(name = "1")]
//!     index: i32,
//!     #[meta(name = "2")]
//!     speed: f32,
//!     #[meta(name = "5")]
//!     position: [f32; 3],
//! }
//!
//! let bytes = MetaBuilder::sample().build().unwrap();
//! let decoded = Container::parse(&bytes).unwrap().decode_objects().unwrap();
//! let item: Item = decoded.typed(0).unwrap();
//! assert_eq!((item.index, item.speed, item.position), (7, 30.0, [1.0, 2.0, 3.0]));
//! # let _ = ObjectRef(None);
//! ```

use ss2_core::{Error, Result};

use crate::decode::{Decoded, TypeSystem};
use crate::value::{Struct, Value};

/// Build a Rust value from a decoded [`Value`].
pub trait FromValue: Sized {
    /// Convert `value`; `types` names the members of class instances.
    ///
    /// # Errors
    /// [`Error::Invalid`] if the value has the wrong shape or does not fit the type.
    fn from_value(value: &Value, types: &TypeSystem) -> Result<Self>;
}

fn bad<T>(want: &str, value: &Value) -> Result<T> {
    Err(Error::invalid(
        "typed value",
        format!("expected {want}, found {value:?}"),
    ))
}

macro_rules! from_integer {
    ($($t:ty),*) => {$(
        impl FromValue for $t {
            fn from_value(value: &Value, _: &TypeSystem) -> Result<Self> {
                match value.as_i64().and_then(|v| <$t>::try_from(v).ok()) {
                    Some(v) => Ok(v),
                    None => match value {
                        Value::U64(v) => <$t>::try_from(*v).or_else(|_| bad(stringify!($t), value)),
                        _ => bad(stringify!($t), value),
                    },
                }
            }
        }
    )*};
}
from_integer!(u8, u16, u32, u64, i8, i16, i32, i64);

impl FromValue for f32 {
    fn from_value(value: &Value, _: &TypeSystem) -> Result<Self> {
        match value {
            Value::F32(v) => Ok(*v),
            _ => bad("a FLOAT", value),
        }
    }
}

impl FromValue for f64 {
    fn from_value(value: &Value, _: &TypeSystem) -> Result<Self> {
        value.as_f64().map_or_else(|| bad("a float", value), Ok)
    }
}

/// `BOOL` is stored as a 4-byte integer: any non-zero value is `true`.
impl FromValue for bool {
    fn from_value(value: &Value, _: &TypeSystem) -> Result<Self> {
        value
            .as_i64()
            .map_or_else(|| bad("a BOOL", value), |v| Ok(v != 0))
    }
}

impl FromValue for String {
    fn from_value(value: &Value, _: &TypeSystem) -> Result<Self> {
        value
            .as_str()
            .map(str::to_string)
            .map_or_else(|| bad("a string", value), Ok)
    }
}

impl FromValue for Value {
    fn from_value(value: &Value, _: &TypeSystem) -> Result<Self> {
        Ok(value.clone())
    }
}

impl<T: FromValue> FromValue for Vec<T> {
    fn from_value(value: &Value, types: &TypeSystem) -> Result<Self> {
        match value {
            Value::List(items) => items.iter().map(|v| T::from_value(v, types)).collect(),
            _ => bad("a list", value),
        }
    }
}

impl<T: FromValue, const N: usize> FromValue for [T; N] {
    fn from_value(value: &Value, types: &TypeSystem) -> Result<Self> {
        let Value::List(items) = value else {
            return bad("a fixed array", value);
        };
        if items.len() != N {
            return bad(&format!("{N} elements"), value);
        }
        let v: Vec<T> = items
            .iter()
            .map(|v| T::from_value(v, types))
            .collect::<Result<_>>()?;
        v.try_into()
            .map_err(|_| Error::invalid("typed value", "array length changed"))
    }
}

/// An array of bytes (`CStaticArray<UBYTE>`).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Bytes(pub Vec<u8>);

impl FromValue for Bytes {
    fn from_value(value: &Value, _: &TypeSystem) -> Result<Self> {
        match value {
            Value::Bytes(b) => Ok(Self(b.clone())),
            Value::List(items) => items
                .iter()
                .map(|v| match v {
                    Value::U8(b) => Ok(*b),
                    other => bad("a byte", other),
                })
                .collect::<Result<Vec<u8>>>()
                .map(Self),
            _ => bad("bytes", value),
        }
    }
}

/// A pointer, handle or `CMetaHandle`: an object id, `None` for null.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ObjectRef(pub Option<u32>);

impl FromValue for ObjectRef {
    fn from_value(value: &Value, _: &TypeSystem) -> Result<Self> {
        match value {
            Value::Ref(id) => Ok(Self(*id)),
            _ => bad("an object reference", value),
        }
    }
}

impl Decoded {
    /// The object with id `id` (main records first, then the extra records).
    pub fn object(&self, id: u32) -> Option<&crate::Object> {
        self.objects.iter().chain(&self.extra).find(|o| o.id == id)
    }

    /// Convert the object `id` into `T` (usually a struct with `#[derive(MetaDeserialize)]`).
    ///
    /// # Errors
    /// [`Error::NotFound`] if there is no such object; [`Error::Invalid`] if it does not convert.
    pub fn typed<T: FromValue>(&self, id: u32) -> Result<T> {
        let object = self
            .object(id)
            .ok_or_else(|| Error::NotFound(format!("object {id}")))?;
        T::from_value(&object.value, &self.types)
    }
}

/// Support code for the derive macro. Not a stable interface.
#[doc(hidden)]
pub mod __private {
    use super::*;

    pub fn expect_struct<'v>(value: &'v Value, target: &str) -> Result<&'v Struct> {
        value.as_struct().map_or_else(
            || {
                Err(Error::invalid(
                    "typed value",
                    format!("{target}: expected a class instance, found {value:?}"),
                ))
            },
            Ok,
        )
    }

    pub fn check_class(types: &TypeSystem, s: &Struct, class: &str, target: &str) -> Result<()> {
        let found = types.type_name(s.type_id).unwrap_or("?");
        if found == class {
            Ok(())
        } else {
            Err(Error::invalid(
                "typed value",
                format!("{target} reads class `{class}`, the object is a `{found}`"),
            ))
        }
    }

    pub fn field<T: FromValue>(
        types: &TypeSystem,
        s: &Struct,
        owner: &str,
        member: &str,
        field: &str,
    ) -> Result<T> {
        let Some(v) = types.member(s, owner, member) else {
            return Err(Error::invalid(
                "typed value",
                format!("field `{field}`: the class has no member `{owner}.{member}`"),
            ));
        };
        T::from_value(v, types).map_err(|e| match e {
            Error::Invalid { what, detail } => Error::Invalid {
                what,
                detail: format!("field `{field}` (`{owner}.{member}`): {detail}"),
            },
            other => other,
        })
    }

    pub fn field_or_default<T: FromValue + Default>(
        types: &TypeSystem,
        s: &Struct,
        owner: &str,
        member: &str,
        field: &str,
    ) -> Result<T> {
        if types.member(s, owner, member).is_none() {
            return Ok(T::default());
        }
        self::field(types, s, owner, member, field)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Container, MetaDeserialize};
    use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, SampleIds};

    #[derive(Debug, PartialEq, MetaDeserialize)]
    #[meta(class = "CTestItem")]
    struct Item {
        #[meta(name = "1")]
        index: i32,
        #[meta(name = "2")]
        speed: f32,
        #[meta(name = "3")]
        name: u32,
        #[meta(name = "4")]
        mode: u8,
        #[meta(name = "5")]
        position: [f32; 3],
        #[meta(name = "99", default)]
        missing: u32,
    }

    #[derive(Debug, PartialEq, MetaDeserialize)]
    struct Holder {
        // A member of the base class and two of the class itself: owners tell them apart.
        #[meta(owner = "CTestItem", name = "2")]
        inherited_speed: f32,
        #[meta(owner = "CTestHolder", name = "1")]
        target: ObjectRef,
        #[meta(owner = "CTestHolder", name = "2")]
        list: Vec<ObjectRef>,
    }

    fn decoded() -> Decoded {
        let bytes = MetaBuilder::sample().build().unwrap();
        Container::parse(&bytes).unwrap().decode_objects().unwrap()
    }

    #[test]
    fn derives_a_struct_from_a_class_instance() {
        let d = decoded();
        let item: Item = d.typed(0).unwrap();
        assert_eq!(
            item,
            Item {
                index: 7,
                speed: 30.0,
                name: 1,
                mode: 2,
                position: [1.0, 2.0, 3.0],
                missing: 0
            }
        );
    }

    #[test]
    fn owners_select_members_of_base_and_derived_classes() {
        let d = decoded();
        let holder: Holder = d.typed(1).unwrap();
        assert_eq!(
            holder,
            Holder {
                inherited_speed: 0.5,
                target: ObjectRef(Some(0)),
                list: vec![ObjectRef(Some(0))],
            }
        );
        assert_eq!(d.types.type_name(SampleIds::HOLDER), Some("CTestHolder"));
    }

    #[test]
    fn errors_name_the_field_and_the_class() {
        let d = decoded();
        // Object 1 is a CTestHolder: the class check of `Item` refuses it.
        let err = d.typed::<Item>(1).unwrap_err().to_string();
        assert!(
            err.contains("reads class `CTestItem`") && err.contains("CTestHolder"),
            "{err}"
        );

        #[derive(Debug, MetaDeserialize)]
        #[allow(dead_code)]
        struct Wrong {
            #[meta(owner = "CTestItem", name = "2")]
            speed: String,
        }
        let err = d.typed::<Wrong>(0).unwrap_err().to_string();
        assert!(
            err.contains("field `speed`") && err.contains("CTestItem.2"),
            "{err}"
        );

        #[derive(Debug, MetaDeserialize)]
        #[allow(dead_code)]
        struct Absent {
            #[meta(owner = "CTestItem", name = "42")]
            nothing: u32,
        }
        let err = d.typed::<Absent>(0).unwrap_err().to_string();
        assert!(err.contains("no member `CTestItem.42`"), "{err}");
        assert!(matches!(d.typed::<Item>(99), Err(Error::NotFound(_))));
    }

    #[test]
    fn scalar_conversions_check_ranges() {
        let ts = TypeSystem::default();
        assert_eq!(u8::from_value(&Value::U32(200), &ts).unwrap(), 200);
        assert!(u8::from_value(&Value::U32(300), &ts).is_err());
        assert!(u32::from_value(&Value::I32(-1), &ts).is_err());
        assert_eq!(i32::from_value(&Value::I32(-1), &ts).unwrap(), -1);
        assert_eq!(
            u64::from_value(&Value::U64(u64::MAX), &ts).unwrap(),
            u64::MAX
        );
        assert!(bool::from_value(&Value::U32(5), &ts).unwrap());
        assert!(!bool::from_value(&Value::U32(0), &ts).unwrap());
        assert_eq!(f64::from_value(&Value::F32(0.5), &ts).unwrap(), 0.5);
        assert!(f32::from_value(&Value::I32(1), &ts).is_err());
        assert_eq!(
            Bytes::from_value(&Value::Bytes(vec![1, 2]), &ts).unwrap(),
            Bytes(vec![1, 2])
        );
        assert!(<[u8; 2]>::from_value(&Value::List(vec![Value::U8(1)]), &ts).is_err());
    }
}
