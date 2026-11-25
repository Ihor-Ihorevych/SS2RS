//! ss2-format-ep — the `.ep` entity parameter loader (roadmap T02-022; spec `docs/re/ep.md`).
//!
//! An `.ep` file is the parameter set of one kind of game object (a projectile, an enemy, a
//! pickup, a weapon): a `CResourceFile` and one **root object** whose class derives from
//! `CEntityParams`, plus the sub-objects it owns (attacks, blasts, graphs). **Inheritance is the
//! class chain of the file's own type table**: every member of the chain, base class first, is
//! stored in the file; nothing is inherited from another file.
//!
//! [`EntityParams::from_bytes`] returns that data as a tree ([`ParamObject`]) in which
//!
//! - every member keeps its qualified name (`Class.member`, [`ParamMember::owner`] and
//!   [`ParamMember::name`]: names alone are not unique along the chain);
//! - identifiers are resolved to their text, enumerations carry their type name,
//! - references to other files become paths ([`ParamValue::File`]: models, controls, textures,
//!   sounds, other `.ep` files), listed by [`EntityParams::referenced_files`], and
//! - owned sub-objects are inlined ([`ParamValue::Object`]), polymorphic ones by their own class.
//!
//! Which numbers mean what (damage, speed, ranges) belongs to the gameplay specs; typed structs
//! for one class can be built on top with [`ParamObject::get`].
//!
//! ```ignore
//! let ep = ss2_format_ep::EntityParams::from_bytes(&bytes)?;
//! assert!(ep.class_chain.contains(&"CEntityParams".to_string()));
//! let model = ep.root.get("SBaseProjectileParams", "bp_pmcModel");
//! for path in ep.referenced_files() { /* load_any(path) */ }
//! ```

use ss2_assets::{AssetLoader, LoadContext};
use ss2_core::{Error, Result};
use ss2_meta::{Container, Decoded, Struct, TypeSystem, Value};

const WHAT: &str = ".ep entity parameters";
/// Deepest nesting of owned objects followed (the real files nest 4 to 5 levels).
const MAX_DEPTH: usize = 32;

/// One value of a parameter.
#[derive(Clone, Debug, PartialEq)]
pub enum ParamValue {
    /// A signed or unsigned integer of any width.
    Int(i64),
    /// A float (`FLOAT`, `DOUBLE`).
    Float(f64),
    /// An `IDENT`, resolved through the file's identifier table (`""` is the empty identifier).
    Ident(String),
    /// An enumeration: the name of its type (`EDamageType`, `ProjectileFlags`, …) and the value.
    Enum {
        /// The type name.
        type_name: String,
        /// The stored value.
        value: u32,
    },
    /// A string.
    Text(String),
    /// A null reference.
    Null,
    /// A reference to another file, as the path listed in this file's `RFIL`.
    File(String),
    /// An owned object, or a class instance stored by value.
    Object(Box<ParamObject>),
    /// A list or fixed array of values (also the elements of an object container).
    List(Vec<ParamValue>),
    /// Raw bytes (`CStaticArray<UBYTE>`).
    Bytes(Vec<u8>),
}

impl ParamValue {
    /// The value as `f64` if it is a number.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Int(v) => Some(*v as f64),
            Self::Float(v) => Some(*v),
            _ => None,
        }
    }

    /// The value as an integer.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Int(v) => Some(*v),
            Self::Enum { value, .. } => Some(i64::from(*value)),
            _ => None,
        }
    }

    /// The path if this is a reference to another file.
    pub fn as_file(&self) -> Option<&str> {
        match self {
            Self::File(p) => Some(p),
            _ => None,
        }
    }

    /// The object if this is an owned object.
    pub fn as_object(&self) -> Option<&ParamObject> {
        match self {
            Self::Object(o) => Some(o),
            _ => None,
        }
    }

    /// The elements if this is a list.
    pub fn as_list(&self) -> Option<&[ParamValue]> {
        match self {
            Self::List(l) => Some(l),
            _ => None,
        }
    }
}

/// One member of an object, with the class that declares it.
#[derive(Clone, Debug, PartialEq)]
pub struct ParamMember {
    /// The class that declares the member (a class of the chain).
    pub owner: String,
    /// The member name (often a number as text).
    pub name: String,
    /// The value.
    pub value: ParamValue,
}

/// A class instance: its class and its members, base class first.
#[derive(Clone, Debug, PartialEq)]
pub struct ParamObject {
    /// The class name.
    pub class: String,
    /// All members of the class chain, base class first.
    pub members: Vec<ParamMember>,
}

impl ParamObject {
    /// The value of member `name` declared by class `owner`.
    pub fn get(&self, owner: &str, name: &str) -> Option<&ParamValue> {
        self.members
            .iter()
            .find(|m| m.owner == owner && m.name == name)
            .map(|m| &m.value)
    }

    /// The first member called `name` whatever its owner (use [`ParamObject::get`] when the name
    /// is not unique along the chain).
    pub fn find(&self, name: &str) -> Option<&ParamValue> {
        self.members
            .iter()
            .find(|m| m.name == name)
            .map(|m| &m.value)
    }

    /// Every file referenced from this object or the objects it owns, in order of appearance
    /// without repeats (compared without regard to case).
    pub fn referenced_files(&self) -> Vec<&str> {
        let mut all = Vec::new();
        self.visit_files(&mut all);
        let mut seen = std::collections::BTreeSet::new();
        all.retain(|p| seen.insert(p.to_lowercase()));
        all
    }

    fn visit_files<'a>(&'a self, out: &mut Vec<&'a str>) {
        for m in &self.members {
            m.value.visit_files(out);
        }
    }
}

impl ParamValue {
    fn visit_files<'a>(&'a self, out: &mut Vec<&'a str>) {
        match self {
            Self::File(p) => out.push(p),
            Self::Object(o) => o.visit_files(out),
            Self::List(l) => l.iter().for_each(|v| v.visit_files(out)),
            _ => {}
        }
    }
}

/// A decoded `.ep` file.
#[derive(Clone, Debug, PartialEq)]
pub struct EntityParams {
    /// The class names from the root class up to its oldest base (`SGenericProjectileParams`,
    /// `SBaseProjectileParams`, `CEntityParams`).
    pub class_chain: Vec<String>,
    /// The root object.
    pub root: ParamObject,
}

/// The object ids of an `rf_crsShared` container.
fn shared_roots(v: &Value) -> Option<Vec<u32>> {
    match v {
        Value::List(items) => items.iter().map(Value::as_ref_id).collect(),
        _ => None,
    }
}

fn bad(detail: impl Into<String>) -> Error {
    Error::invalid(WHAT, detail)
}

struct Walker<'a> {
    container: &'a Container<'a>,
    decoded: &'a Decoded,
    what: &'static str,
}

impl Walker<'_> {
    fn bad(&self, detail: impl Into<String>) -> Error {
        Error::invalid(self.what, detail)
    }

    fn types(&self) -> &TypeSystem {
        &self.decoded.types
    }

    fn object(&self, id: u32, depth: usize) -> Result<ParamValue> {
        if let Some(e) = self
            .container
            .external_objects
            .iter()
            .find(|e| e.object_id == id)
        {
            let file = self
                .container
                .files
                .get(e.file_ref as usize)
                .ok_or_else(|| self.bad("a reference points past the file table"))?;
            return Ok(ParamValue::File(file.path.to_string()));
        }
        let o = self.decoded.object(id).ok_or_else(|| {
            self.bad(format!(
                "a reference names object {id}, which does not exist"
            ))
        })?;
        let s = o
            .value
            .as_struct()
            .ok_or_else(|| self.bad("an object is not a class instance"))?;
        // Every object points back to the file's own `CResourceFile` (`CResource.2`), which in
        // turn lists the root: that bookkeeping link is not a parameter, so it reads as null.
        if self.types().type_name(s.type_id) == Some("CResourceFile") {
            return Ok(ParamValue::Null);
        }
        Ok(ParamValue::Object(Box::new(
            self.struct_value(s, depth + 1)?,
        )))
    }

    fn struct_value(&self, s: &Struct, depth: usize) -> Result<ParamObject> {
        if depth > MAX_DEPTH {
            return Err(self.bad("objects are nested too deeply (a reference cycle?)"));
        }
        let class = self.types().type_name(s.type_id).unwrap_or("?").to_string();
        let layout = self
            .types()
            .members(s.type_id)
            .ok_or_else(|| self.bad(format!("class `{class}` has no layout")))?;
        let mut members = Vec::with_capacity(layout.len());
        for (m, v) in layout.iter().zip(&s.values) {
            members.push(ParamMember {
                owner: m.owner_name.clone(),
                name: m.name.clone(),
                value: self.value(v, depth)?,
            });
        }
        Ok(ParamObject { class, members })
    }

    fn value(&self, v: &Value, depth: usize) -> Result<ParamValue> {
        Ok(match v {
            Value::U8(x) => ParamValue::Int(i64::from(*x)),
            Value::I8(x) => ParamValue::Int(i64::from(*x)),
            Value::U16(x) => ParamValue::Int(i64::from(*x)),
            Value::I16(x) => ParamValue::Int(i64::from(*x)),
            Value::U32(x) => ParamValue::Int(i64::from(*x)),
            Value::I32(x) => ParamValue::Int(i64::from(*x)),
            Value::U64(x) => ParamValue::Int(i64::try_from(*x).unwrap_or(i64::MAX)),
            Value::I64(x) => ParamValue::Int(*x),
            Value::F32(x) => ParamValue::Float(f64::from(*x)),
            Value::F64(x) => ParamValue::Float(*x),
            Value::Ident(i) => ParamValue::Ident(
                self.container
                    .ident(*i)
                    .ok_or_else(|| {
                        self.bad(format!("identifier {i} is not in the identifier table"))
                    })?
                    .to_string(),
            ),
            Value::Enum { type_id, value } => ParamValue::Enum {
                type_name: self.types().type_name(*type_id).unwrap_or("?").to_string(),
                value: *value,
            },
            Value::String(s) => ParamValue::Text(s.clone()),
            Value::Ref(None) => ParamValue::Null,
            Value::Ref(Some(id)) => self.object(*id, depth)?,
            Value::Struct(s) => ParamValue::Object(Box::new(self.struct_value(s, depth + 1)?)),
            Value::List(items) => ParamValue::List(
                items
                    .iter()
                    .map(|i| self.value(i, depth + 1))
                    .collect::<Result<_>>()?,
            ),
            Value::Bytes(b) => ParamValue::Bytes(b.clone()),
        })
    }
}

/// A decoded resource file of the shared `.ep`/`.rsc`/`.ctl` layout: the class chain of its one
/// shared root and the root as a [`ParamObject`] tree.
///
/// This is the generic part of [`EntityParams`] (the other loaders built on the same container,
/// such as `ss2-format-rsc`, reuse it); the `.ep` rule that the root derives from `CEntityParams`
/// is **not** checked here.
#[derive(Clone, Debug, PartialEq)]
pub struct ParamTree {
    /// The class names from the root class up to its oldest base.
    pub class_chain: Vec<String>,
    /// The root object.
    pub root: ParamObject,
}

impl ParamTree {
    /// Decode a format-8 resource file with exactly one shared root.
    ///
    /// # Errors
    /// [`Error::Invalid`] (with `what` as the subject) when the file is not a decodable
    /// resource file, has no or several shared roots, or has a bad reference.
    pub fn from_bytes(bytes: &[u8], what: &'static str) -> Result<Self> {
        let bad = |detail: String| Error::invalid(what, detail);
        let container = Container::parse(bytes)?;
        if container.header.format_version != 8 {
            return Err(bad(format!(
                "format version {}",
                container.header.format_version
            )));
        }
        let decoded = container.decode_objects()?;
        let file = decoded
            .objects
            .iter()
            .find(|o| decoded.types.type_name(o.type_id) == Some("CResourceFile"))
            .and_then(|o| o.value.as_struct())
            .ok_or_else(|| bad("the file has no CResourceFile".into()))?;
        let shared = decoded
            .types
            .member(file, "CResourceFile", "rf_crsShared")
            .and_then(shared_roots)
            .ok_or_else(|| bad("the CResourceFile has no rf_crsShared list".into()))?;
        let [root_id] = shared.as_slice() else {
            return Err(bad(format!(
                "{} shared roots, exactly one expected",
                shared.len()
            )));
        };
        let root_object = decoded
            .object(*root_id)
            .ok_or_else(|| bad("the root object does not exist".into()))?;
        let root_struct = root_object
            .value
            .as_struct()
            .ok_or_else(|| bad("the root is not a class instance".into()))?;
        // The chain by parent links (classes without members, such as CEntityParams, count).
        let mut cursor = Some(root_struct.type_id);
        let mut class_chain = Vec::new();
        while let Some(id) = cursor {
            class_chain.push(decoded.types.type_name(id).unwrap_or("?").to_string());
            cursor = container.type_def(id).and_then(|t| match &t.shape {
                ss2_meta::TypeShape::Struct { parent, .. } => *parent,
                ss2_meta::TypeShape::Short { .. } => None,
            });
            if class_chain.len() > MAX_DEPTH {
                return Err(bad("the class chain is too long or circular".into()));
            }
        }
        let walker = Walker {
            container: &container,
            decoded: &decoded,
            what,
        };
        let root = walker.struct_value(root_struct, 0)?;
        Ok(Self { class_chain, root })
    }

    /// Every file the tree refers to, in order of appearance without repeats (compared without
    /// regard to case).
    pub fn referenced_files(&self) -> Vec<&str> {
        self.root.referenced_files()
    }
}

impl EntityParams {
    /// Read an `.ep` file.
    ///
    /// Checks the rules of `ep.md`: exactly one shared root (`rf_crsShared`), its class chain
    /// contains `CEntityParams`, every reference is external (a file path) or an object of the
    /// file, owned objects nest no deeper than 32 levels.
    ///
    /// # Errors
    /// [`Error::Invalid`] naming what is wrong.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let ParamTree { class_chain, root } = ParamTree::from_bytes(bytes, WHAT)?;
        if !class_chain.iter().any(|c| c == "CEntityParams") {
            return Err(bad(format!(
                "the root class `{}` does not derive from CEntityParams",
                class_chain[0]
            )));
        }
        Ok(Self { class_chain, root })
    }

    /// The root class name.
    pub fn root_class(&self) -> &str {
        &self.root.class
    }

    /// Every file the parameters refer to, in order of appearance without repeats: the models,
    /// controls, textures, sounds and other `.ep` files to request from the asset manager.
    pub fn referenced_files(&self) -> Vec<&str> {
        self.root.referenced_files()
    }
}

/// The `.ep` loader: register it with `AssetManager::register_loader` to load [`EntityParams`].
/// [`EntityParams::referenced_files`] lists what to request next.
#[derive(Clone, Copy, Debug, Default)]
pub struct EpLoader;

impl AssetLoader for EpLoader {
    type Asset = EntityParams;

    fn extensions(&self) -> &[&str] {
        &["ep"]
    }

    fn load(&self, ctx: &mut LoadContext<'_>) -> Result<EntityParams> {
        let params = EntityParams::from_bytes(&ctx.read()?)?;
        for path in params.referenced_files() {
            // The references are dependencies of the parameter set. A reference whose format has
            // no loader registered yet is simply not recorded.
            let _ = ctx.load_any(path);
        }
        Ok(params)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ss2_assets::{AssetManager, LoadState};
    use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};
    use ss2_vfs::MemoryVfs;

    use super::*;

    /// A projectile-like file: root `SGenericProjectileParams` (base `SBaseProjectileParams`, base
    /// `CEntityParams`) with a model reference, an identifier, an enumeration, a float, and one
    /// owned sub-object (a blast).
    fn params_file(root_derives_from_entity_params: bool) -> Vec<u8> {
        let mut b = MetaBuilder::new();
        let model = b.file_ref("Content/Test/Models/axe.mdl", 0x4_0000);
        let name = b.identifier("Axe");
        b.external_type(100, "CModelConfiguration");
        b.type_block(TypeBlock::primitive(0, "SLONG", 4, 4));
        b.type_block(TypeBlock::alias(1, "INDEX", 0));
        b.type_block(TypeBlock::primitive(2, "FLOAT", 4, 4));
        b.type_block(TypeBlock::primitive(3, "IDENT", 4, 4));
        b.type_block(TypeBlock::enumeration(4, "EDamageType", 4));
        b.type_block(TypeBlock::pointer(5, "CModelConfiguration*", 100));
        b.type_block(TypeBlock::short(6, "CDynamicContainer<CResource>", 0, 8, 7));
        b.type_block(TypeBlock::class(7, "CResource", 4, 5, None, &[("1", 1)]));
        b.type_block(TypeBlock::class(
            8,
            "CResourceFile",
            3,
            5,
            None,
            &[("rf_crsShared", 6)],
        ));
        b.type_block(TypeBlock::class(
            9,
            "CEntityParams",
            1,
            5,
            if root_derives_from_entity_params {
                None
            } else {
                Some(7)
            },
            &[],
        ));
        b.type_block(TypeBlock::class(
            10,
            "CSingleBlast",
            1,
            5,
            None,
            &[("sb_fRadius", 2)],
        ));
        b.type_block(TypeBlock::pointer(11, "CSingleBlast*", 10));
        b.type_block(TypeBlock::class(
            12,
            "SBaseProjectileParams",
            1,
            5,
            Some(9),
            &[("bp_pmcModel", 5), ("bp_idName", 3)],
        ));
        let parent_of_root = if root_derives_from_entity_params {
            12
        } else {
            7
        };
        b.type_block(TypeBlock::class(
            13,
            "SGenericProjectileParams",
            1,
            5,
            Some(parent_of_root),
            &[("gpp_edtDamage", 4), ("gpp_pblast", 11), ("gpp_fSpeed", 2)],
        ));
        b.external_object(5, model, 1, 100);
        b.object(0, 8, ObjectBytes::new().dynamic_container(&[1]).finish());
        let mut members = ObjectBytes::new();
        if !root_derives_from_entity_params {
            members = members.i32(1);
        } else {
            members = members.object_ref(Some(5)).u32(name);
        }
        b.object(1, 13, members.u32(2).object_ref(Some(2)).f32(30.5).finish());
        b.object(2, 10, ObjectBytes::new().f32(2.5).finish());
        b.build().unwrap()
    }

    #[test]
    fn reads_the_class_chain_members_files_and_owned_objects() {
        let ep = EntityParams::from_bytes(&params_file(true)).unwrap();
        assert_eq!(ep.root_class(), "SGenericProjectileParams");
        assert_eq!(
            ep.class_chain,
            [
                "SGenericProjectileParams",
                "SBaseProjectileParams",
                "CEntityParams"
            ]
        );
        let r = &ep.root;
        assert_eq!(r.members.len(), 5, "base members first");
        assert_eq!(r.members[0].owner, "SBaseProjectileParams");
        assert_eq!(
            r.get("SBaseProjectileParams", "bp_pmcModel")
                .unwrap()
                .as_file(),
            Some("Content/Test/Models/axe.mdl")
        );
        assert_eq!(
            r.get("SBaseProjectileParams", "bp_idName"),
            Some(&ParamValue::Ident("Axe".into()))
        );
        assert_eq!(
            r.find("gpp_edtDamage"),
            Some(&ParamValue::Enum {
                type_name: "EDamageType".into(),
                value: 2
            })
        );
        assert_eq!(r.find("gpp_fSpeed").unwrap().as_f64(), Some(30.5));
        let blast = r.find("gpp_pblast").unwrap().as_object().unwrap();
        assert_eq!(blast.class, "CSingleBlast");
        assert_eq!(blast.find("sb_fRadius").unwrap().as_f64(), Some(2.5));
        assert_eq!(ep.referenced_files(), ["Content/Test/Models/axe.mdl"]);
        assert!(r.get("CEntityParams", "x").is_none());
    }

    #[test]
    fn a_root_that_does_not_derive_from_entity_params_is_refused() {
        let err = EntityParams::from_bytes(&params_file(false))
            .unwrap_err()
            .to_string();
        assert!(err.contains("does not derive from CEntityParams"), "{err}");
        assert!(EntityParams::from_bytes(b"junk").is_err());
    }

    #[test]
    fn nothing_panics_on_damaged_files() {
        let good = params_file(true);
        for len in (0..good.len()).step_by(7) {
            let _ = EntityParams::from_bytes(&good[..len]);
        }
        for i in (0..good.len()).step_by(3) {
            let mut bad = good.clone();
            bad[i] ^= 0x33;
            let _ = EntityParams::from_bytes(&bad);
        }
    }

    #[test]
    fn the_loader_plugs_into_the_asset_manager_and_records_dependencies() {
        let file = params_file(true);
        let vfs = MemoryVfs::from_files([("databases/axe.ep", file.as_slice())]).unwrap();
        let assets = AssetManager::new(Arc::new(vfs));
        assets.register_loader(EpLoader).unwrap();
        let h = assets.load::<EntityParams>("Databases/Axe.EP").unwrap();
        assert_eq!(h.wait(), LoadState::Ready);
        assert_eq!(h.get().unwrap().root_class(), "SGenericProjectileParams");
        // No `.mdl` loader is registered here, so the model is not a recorded dependency.
        assert!(h.dependencies().is_empty());
    }

    /// Data-gated: every `.ep` of the retail install loads (322 files, 43 root classes), every
    /// root derives from `CEntityParams`, and the references are the 596 of `ep.md`. Skips
    /// without `SS2_DATA_DIR`.
    #[test]
    fn every_parameter_file_of_the_real_install_loads() {
        use ss2_vfs::{EntryKind, Vfs, VfsPath};
        use std::collections::BTreeMap;
        let dir = ss2_testkit::require_game_data!();
        let vfs = ss2_vfs::mount_install(&dir).unwrap();
        let mut stack = vec![VfsPath::root()];
        let mut roots: BTreeMap<String, usize> = BTreeMap::new();
        let (mut files, mut references) = (0usize, 0usize);
        let mut by_ext: BTreeMap<String, usize> = BTreeMap::new();
        let mut failures = Vec::new();
        while let Some(d) = stack.pop() {
            for e in vfs.list(&d).unwrap() {
                let path = d.join(&e.name).unwrap();
                if e.kind == EntryKind::Dir {
                    stack.push(path);
                    continue;
                }
                if path.extension() != Some("ep") {
                    continue;
                }
                match EntityParams::from_bytes(&vfs.read(&path).unwrap()) {
                    Ok(ep) => {
                        files += 1;
                        *roots.entry(ep.root_class().to_string()).or_default() += 1;
                        let refs = ep.referenced_files();
                        references += refs.len();
                        for r in refs {
                            *by_ext
                                .entry(r.rsplit('.').next().unwrap_or("").to_lowercase())
                                .or_default() += 1;
                        }
                    }
                    Err(e) => failures.push(format!("{}: {e}", path.as_str())),
                }
            }
        }
        eprintln!(
            "{files} files, {} root classes, {references} distinct references per file; by extension {by_ext:?}; {} failures",
            roots.len(),
            failures.len()
        );
        assert!(
            failures.is_empty(),
            "first failures: {:?}",
            &failures[..failures.len().min(2)]
        );
        assert_eq!(files, 322);
        assert_eq!(roots.len(), 43);
        assert_eq!(references, 596, "the references of ep.md");
    }
}
