//! ss2-meta — CTSEMETA container and reflection reader (track T02).
//!
//! Almost every asset of the game (textures, models, worlds, parameter files, menus, …) is a
//! **CTSEMETA** file: a self-describing container with a table of referenced files, a string
//! pool, the definitions of the data types it uses, and the objects serialized with them.
//!
//! Two layers:
//!
//! 1. **The container frame** ([`Container::parse`], roadmap T02-006): header and `$Version`
//!    string, the `INFO` counts, the file references (`RFIL`), identifiers (`IDNT`), external types
//!    (`EXTY`), the type table (`DTTY` blocks), the external-object, object-type and extra-type
//!    tables (`EXOB`, `OBTY`, `EDTY`) and the byte range of the object data.
//! 2. **The type system and value decoder** ([`TypeSystem`], [`Container::decode_objects`],
//!    T02-007): decodes every object into a generic [`Value`] tree using only the file's own type
//!    table. A typed layer on top of the tree is the job of later tasks (T02-008b).
//!
//! Format versions 4 to 8 are read (versions 4 to 7 only occur in 155 old `.tex` files of the
//! patches). The specs: `docs/re/ctsemeta-container.md`, `ctsemeta-types.md`,
//! `ctsemeta-objects.md`, `ctsemeta-leftovers.md`, `dtty-kinds.md`.
//!
//! ```
//! use ss2_meta::{Container, Value};
//! use ss2_testkit::fixtures::ctsemeta::MetaBuilder;
//!
//! let bytes = MetaBuilder::sample().build().unwrap();
//! let meta = Container::parse(&bytes).unwrap();
//! assert_eq!(meta.header.format_version, 8);
//! assert_eq!(meta.files[0].path, "Content/Test/Models/Cube.mdl");
//! assert_eq!(meta.object_table.len(), 2);
//!
//! let decoded = meta.decode_objects().unwrap();
//! let item = decoded.objects[0].value.as_struct().unwrap();
//! assert_eq!(decoded.types.member(item, "CTestItem", "2"), Some(&Value::F32(30.0)));
//! ```

mod container;
mod decode;
mod value;

pub use container::{
    ArrayDims, Build, Container, ExternalObject, ExternalType, FLAG_PARAMETER_REFERENCE, FileRef,
    Header, Info, Member, ObjectEntry, Section, ShortForm, TypeDef, TypeShape,
};
pub use decode::{Decoded, FlatMember, TypeSystem};
pub use value::{Object, Struct, Value};
