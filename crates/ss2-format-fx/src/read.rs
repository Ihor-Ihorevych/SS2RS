//! Shared reading helpers: a view of one decoded class instance with typed member access.

use ss2_core::{Error, Result};
use ss2_meta::{Container, Decoded, FromValue, ObjectRef, Struct, Value};

/// The decoded file and the name used in error messages.
pub(crate) struct Ctx<'a> {
    pub container: &'a Container<'a>,
    pub decoded: &'a Decoded,
    pub what: &'static str,
}

impl<'a> Ctx<'a> {
    /// Parse and decode a format-8 file.
    pub fn decode(bytes: &'a [u8], what: &'static str) -> Result<(Container<'a>, Decoded)> {
        let container = Container::parse(bytes)?;
        if container.header.format_version != 8 {
            return Err(Error::invalid(
                what,
                format!("format version {}", container.header.format_version),
            ));
        }
        let decoded = container.decode_objects()?;
        Ok((container, decoded))
    }

    pub fn bad(&self, detail: impl Into<String>) -> Error {
        Error::invalid(self.what, detail)
    }

    /// The object `id` as a class instance.
    pub fn object(&'a self, id: u32) -> Result<Obj<'a>> {
        let o = self.decoded.object(id).ok_or_else(|| {
            self.bad(format!(
                "a reference names object {id}, which does not exist"
            ))
        })?;
        let s = o
            .value
            .as_struct()
            .ok_or_else(|| self.bad(format!("object {id} is not a class instance")))?;
        Ok(Obj { ctx: self, s })
    }

    /// The file's single shared root (`CResourceFile.rf_crsShared`), which must be of class
    /// `class`.
    pub fn root(&'a self, class: &str) -> Result<Obj<'a>> {
        let file = self
            .decoded
            .objects
            .iter()
            .find(|o| self.decoded.types.type_name(o.type_id) == Some("CResourceFile"))
            .and_then(|o| o.value.as_struct())
            .ok_or_else(|| self.bad("the file has no CResourceFile"))?;
        let shared = match self
            .decoded
            .types
            .member(file, "CResourceFile", "rf_crsShared")
        {
            Some(Value::List(items)) => items
                .iter()
                .map(Value::as_ref_id)
                .collect::<Option<Vec<_>>>(),
            _ => None,
        }
        .ok_or_else(|| self.bad("the CResourceFile has no rf_crsShared list"))?;
        let [root_id] = shared.as_slice() else {
            return Err(self.bad(format!(
                "{} shared roots, exactly one expected",
                shared.len()
            )));
        };
        let root = self.object(*root_id)?;
        if root.class() != class {
            return Err(self.bad(format!(
                "the root is a `{}`, a `{class}` is expected",
                root.class()
            )));
        }
        Ok(root)
    }

    /// The path of an external file object (`None` for a null reference).
    pub fn file(&self, r: ObjectRef, what: &str) -> Result<Option<String>> {
        let Some(id) = r.0 else {
            return Ok(None);
        };
        self.container
            .external_objects
            .iter()
            .find(|e| e.object_id == id)
            .and_then(|e| self.container.files.get(e.file_ref as usize))
            .map(|f| Some(f.path.to_string()))
            .ok_or_else(|| {
                self.bad(format!(
                    "{what} refers to object {id}, which is not an external file"
                ))
            })
    }

    /// Like [`Ctx::file`] but a null reference is an error.
    pub fn required_file(&self, r: ObjectRef, what: &str) -> Result<String> {
        self.file(r, what)?
            .ok_or_else(|| self.bad(format!("{what} is null")))
    }
}

/// One class instance with typed access to its members by `(owner, name)`.
#[derive(Clone, Copy)]
pub(crate) struct Obj<'a> {
    pub ctx: &'a Ctx<'a>,
    pub s: &'a Struct,
}

impl<'a> Obj<'a> {
    pub fn class(&self) -> &'a str {
        self.ctx
            .decoded
            .types
            .type_name(self.s.type_id)
            .unwrap_or("?")
    }

    /// The raw value of member `owner.name`.
    pub fn value(&self, owner: &str, name: &str) -> Result<&'a Value> {
        self.ctx
            .decoded
            .types
            .member(self.s, owner, name)
            .ok_or_else(|| {
                self.ctx.bad(format!(
                    "class `{}` has no member `{owner}.{name}`",
                    self.class()
                ))
            })
    }

    /// Member `owner.name` converted to `T`.
    pub fn get<T: FromValue>(&self, owner: &str, name: &str) -> Result<T> {
        T::from_value(self.value(owner, name)?, &self.ctx.decoded.types).map_err(|e| match e {
            Error::Invalid { what, detail } => {
                Error::invalid(what, format!("`{owner}.{name}`: {detail}"))
            }
            other => other,
        })
    }

    /// A class instance stored by value in member `owner.name`.
    pub fn sub(&self, owner: &str, name: &str) -> Result<Obj<'a>> {
        match self.value(owner, name)? {
            Value::Struct(s) => Ok(Obj { ctx: self.ctx, s }),
            other => Err(self.ctx.bad(format!(
                "`{owner}.{name}` is not a class instance: {other:?}"
            ))),
        }
    }

    /// The elements of a list member as class instances stored by value.
    pub fn subs(&self, owner: &str, name: &str) -> Result<Vec<Obj<'a>>> {
        match self.value(owner, name)? {
            Value::List(items) => items
                .iter()
                .map(|v| match v {
                    Value::Struct(s) => Ok(Obj { ctx: self.ctx, s }),
                    other => Err(self
                        .ctx
                        .bad(format!("`{owner}.{name}` holds a non-instance: {other:?}"))),
                })
                .collect(),
            other => Err(self
                .ctx
                .bad(format!("`{owner}.{name}` is not a list: {other:?}"))),
        }
    }

    /// The objects a pointer container (`CDynamicContainer<T>`) member lists.
    pub fn objects(&self, owner: &str, name: &str) -> Result<Vec<Obj<'a>>> {
        let refs: Vec<ObjectRef> = self.get(owner, name)?;
        refs.into_iter()
            .map(|r| {
                let id = r.0.ok_or_else(|| {
                    self.ctx
                        .bad(format!("`{owner}.{name}` holds a null pointer"))
                })?;
                self.ctx.object(id)
            })
            .collect()
    }

    /// The object a pointer member names (`None` for null).
    pub fn pointee(&self, owner: &str, name: &str) -> Result<Option<Obj<'a>>> {
        let r: ObjectRef = self.get(owner, name)?;
        r.0.map(|id| self.ctx.object(id)).transpose()
    }

    /// An external file named by pointer member `owner.name` (`None` for null).
    pub fn file(&self, owner: &str, name: &str) -> Result<Option<String>> {
        let r: ObjectRef = self.get(owner, name)?;
        self.ctx.file(r, &format!("`{owner}.{name}`"))
    }
}
