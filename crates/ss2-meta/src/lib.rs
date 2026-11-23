//! ss2-meta — CTSEMETA container and reflection reader (track T02).
//!
//! Almost every asset of the game (textures, models, worlds, parameter files, menus, …) is a
//! **CTSEMETA** file: a self-describing container with a table of referenced files, a string
//! pool, the definitions of the data types it uses, and the objects serialized with them.
//!
//! This crate reads the **container frame** ([`Container::parse`], roadmap T02-006): header and
//! `$Version` string, the `INFO` counts, the file references (`RFIL`), identifiers (`IDNT`),
//! external types (`EXTY`), the type table (`DTTY` blocks), the external-object, object-type and
//! extra-type tables (`EXOB`, `OBTY`, `EDTY`) and the byte range of the object data. It does not
//! decode object values: that is the type system and value decoder (T02-007) built on top.
//!
//! Format versions 4 to 8 are read (versions 4 to 7 only occur in 155 old `.tex` files of the
//! patches). The specs: `docs/re/ctsemeta-container.md`, `ctsemeta-types.md`,
//! `ctsemeta-leftovers.md`, `dtty-kinds.md`.
//!
//! ```
//! use ss2_meta::Container;
//! use ss2_testkit::fixtures::ctsemeta::MetaBuilder;
//!
//! let bytes = MetaBuilder::sample().build().unwrap();
//! let meta = Container::parse(&bytes).unwrap();
//! assert_eq!(meta.header.format_version, 8);
//! assert_eq!(meta.files[0].path, "Content/Test/Models/Cube.mdl");
//! assert_eq!(meta.object_table.len(), 2);
//! ```

mod container;

pub use container::{
    Build, Container, ExternalObject, ExternalType, FLAG_PARAMETER_REFERENCE, FileRef, Header,
    Info, Member, ObjectEntry, Section, ShortForm, TypeDef, TypeShape,
};
