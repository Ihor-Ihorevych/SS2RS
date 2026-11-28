//! Read-only skeleton assets from CTSEMETA `.skl` files (T02-015).
//!
//! Register [`SkeletonLoader`] with the asset manager or call [`Skeleton::parse`] on bytes.
//! Geometry uses the file's coordinate basis and units. The stored inverse bind matrix is
//! preserved independently of the local pose. Each LOD has its own bone IDs and hierarchy.
//!
//! Each bone also carries its **name**: the file's own identifier (`IDNT`) table stores a bone's
//! name at the index equal to the bone's id (`docs/re/ans.md`, "Target ids"). Animation tracks
//! name their bones by string, so a track binds to the bone of the same name
//! ([`SkeletonLod::bone_by_name`]), never by treating its identifier index as a bone id.

use std::collections::BTreeMap;

use ss2_assets::{AssetLoader, LoadContext};
use ss2_core::math::{Mat4, Quat, Vec3};
use ss2_core::{Error, Result};
use ss2_meta::{Container, Struct, TypeSystem, Value};

/// A skeleton, with its LODs in source order.
#[derive(Debug, Clone, PartialEq)]
pub struct Skeleton {
    lods: Vec<SkeletonLod>,
}

/// A self-contained skeleton LOD.
#[derive(Debug, Clone, PartialEq)]
pub struct SkeletonLod {
    /// Source distance at which this LOD can be selected.
    pub switch_distance: f32,
    /// Source platform mask, with its bits preserved.
    pub platform_mask: u32,
    bones: Vec<Bone>,
    bone_indices: BTreeMap<u32, usize>,
    name_indices: BTreeMap<String, usize>,
}

impl SkeletonLod {
    /// Bones in source order. Each parent occurs before its children.
    pub fn bones(&self) -> &[Bone] {
        &self.bones
    }

    /// Look up a bone by source ID. IDs can have gaps.
    pub fn bone(&self, id: u32) -> Option<&Bone> {
        self.bone_indices.get(&id).map(|&index| &self.bones[index])
    }

    /// Look up a bone by its name (exact match). If two bones share a name, the first in source
    /// order is returned. Bones without a name are never found.
    pub fn bone_by_name(&self, name: &str) -> Option<&Bone> {
        self.name_indices.get(name).map(|&index| &self.bones[index])
    }
}

/// One bone in a skeleton LOD.
#[derive(Debug, Clone, PartialEq)]
pub struct Bone {
    /// The nonzero source bone ID.
    pub id: u32,
    /// The bone's name: the file's identifier-table entry at index [`Self::id`]. Empty if the
    /// table has no entry there.
    pub name: String,
    /// A parent bone ID in the same LOD. Roots have no parent.
    pub parent: Option<u32>,
    /// Source member 3, whose meaning is open.
    pub raw_member_3: f32,
    /// Parent-relative rotation as a unit quaternion.
    pub local_rotation: Quat,
    /// Parent-relative translation in source units.
    pub local_translation: Vec3,
    /// Stored affine inverse bind transform. Applies to a column vector.
    pub inverse_bind: Mat4,
    /// Source bone length, including the source convention for a zero translation.
    pub length: f32,
    /// Source lower angle limits in radians.
    pub angle_min: Vec3,
    /// Source upper angle limits in radians.
    pub angle_max: Vec3,
    /// Source IK flags. Their bit meanings are open.
    pub ik_flags: u32,
}

impl Skeleton {
    /// Read a skeleton from a complete CTSEMETA file in source coordinate space.
    ///
    /// # Errors
    /// An error if the container or skeleton fields are invalid, a quaternion is unusable, or
    /// a bone is duplicated or references a parent that does not precede it in the same LOD.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let container = Container::parse(bytes)?;
        let decoded = container.decode_objects()?;
        let [resource, skeleton] = decoded.objects.as_slice() else {
            return Err(invalid(
                "expected two main objects: CResourceFile and CSkeleton",
            ));
        };
        let types = &decoded.types;
        Fields::new(types, &resource.value, "CResourceFile")?;
        let skeleton = Fields::new(types, &skeleton.value, "CSkeleton")?;
        let lods = skeleton
            .list("1")?
            .iter()
            .map(|value| read_lod(&container, types, value))
            .collect::<Result<_>>()?;
        Ok(Self { lods })
    }

    /// The LODs in file order.
    pub fn lods(&self) -> &[SkeletonLod] {
        &self.lods
    }
}

/// Asset-manager loader for `.skl` files.
pub struct SkeletonLoader;

impl AssetLoader for SkeletonLoader {
    type Asset = Skeleton;

    fn extensions(&self) -> &[&str] {
        &["skl"]
    }

    fn load(&self, ctx: &mut LoadContext<'_>) -> Result<Skeleton> {
        Skeleton::parse(&ctx.read()?)
    }
}

fn read_lod(container: &Container<'_>, types: &TypeSystem, value: &Value) -> Result<SkeletonLod> {
    let lod = Fields::new(types, value, "CSkeletonLOD")?;
    let mut bones = Vec::new();
    let mut bone_indices = BTreeMap::new();
    let mut name_indices = BTreeMap::new();
    for value in lod.list("2")? {
        let fields = Fields::new(types, value, "SkeletonBone")?;
        let id = fields.uint("1")?;
        if id == 0 {
            return Err(invalid("bone ID must be positive"));
        }
        if bone_indices.contains_key(&id) {
            return Err(invalid(format!("duplicate bone ID {id}")));
        }
        let parent = fields.uint("2")?;
        let parent = (parent != 0).then_some(parent);
        if let Some(parent) = parent
            && !bone_indices.contains_key(&parent)
        {
            return Err(invalid(format!(
                "bone {id} must reference an earlier parent in this LOD (got {parent})"
            )));
        }

        let local = fields.nested("4", "QuatVect")?;
        let rotation = Quat::from_array(local.nested("1", "Quaternion4f")?.components()?);
        if (rotation.length_squared() - 1.0).abs() > 0.001 {
            return Err(invalid(format!(
                "bone {id} has an invalid local quaternion"
            )));
        }
        let [a, b, c, x, d, e, f, y, g, h, i, z] = fields.nested("5", "Matrix34f")?.components()?;
        let inverse_bind =
            Mat4::from_cols_array(&[a, d, g, 0.0, b, e, h, 0.0, c, f, i, 0.0, x, y, z, 1.0]);

        let name = container.ident(id).unwrap_or_default().to_owned();
        if !name.is_empty() {
            name_indices.entry(name.clone()).or_insert(bones.len());
        }
        bones.push(Bone {
            id,
            name,
            parent,
            raw_member_3: fields.float("3")?,
            local_rotation: rotation.normalize(),
            local_translation: local.vector("2")?,
            inverse_bind,
            length: fields.float("6")?,
            angle_min: fields.vector("7")?,
            angle_max: fields.vector("8")?,
            ik_flags: fields.uint("9")?,
        });
        bone_indices.insert(id, bones.len() - 1);
    }

    Ok(SkeletonLod {
        switch_distance: lod.float("1")?,
        platform_mask: lod.uint("4")?,
        bones,
        bone_indices,
        name_indices,
    })
}

fn invalid(detail: impl Into<String>) -> Error {
    Error::invalid("SKL skeleton", detail)
}

/// Resolves member names in the class that declares each field, including when a base class
/// has a field with the same number.
struct Fields<'a> {
    types: &'a TypeSystem,
    instance: &'a Struct,
    class: &'static str,
}

impl<'a> Fields<'a> {
    fn new(types: &'a TypeSystem, value: &'a Value, class: &'static str) -> Result<Self> {
        let instance = value
            .as_struct()
            .ok_or_else(|| invalid(format!("expected {class} object")))?;
        let actual = types.type_name(instance.type_id).unwrap_or("<unknown>");
        if actual != class {
            return Err(invalid(format!("expected {class}, got {actual}")));
        }
        Ok(Self {
            types,
            instance,
            class,
        })
    }

    fn get(&self, key: &str) -> Result<&'a Value> {
        self.types
            .member(self.instance, self.class, key)
            .ok_or_else(|| invalid(format!("missing {}.{key}", self.class)))
    }

    fn nested(&self, key: &str, class: &'static str) -> Result<Self> {
        Self::new(self.types, self.get(key)?, class)
    }

    fn list(&self, key: &str) -> Result<&'a [Value]> {
        match self.get(key)? {
            Value::List(values) => Ok(values),
            _ => Err(invalid(format!("{}.{key} must be a list", self.class))),
        }
    }

    fn uint(&self, key: &str) -> Result<u32> {
        self.get(key)?
            .as_i64()
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| {
                invalid(format!(
                    "{}.{key} must be a nonnegative integer or enum",
                    self.class
                ))
            })
    }

    fn float(&self, key: &str) -> Result<f32> {
        match self.get(key)? {
            Value::F32(value) if value.is_finite() => Ok(*value),
            _ => Err(invalid(format!(
                "{}.{key} must be a finite float",
                self.class
            ))),
        }
    }

    fn components<const N: usize>(&self) -> Result<[f32; N]> {
        let mut values = [0.0; N];
        for (index, value) in values.iter_mut().enumerate() {
            *value = self.float(&(index + 1).to_string())?;
        }
        Ok(values)
    }

    fn vector(&self, key: &str) -> Result<Vec3> {
        Ok(Vec3::from_array(
            self.nested(key, "Vector3f")?.components()?,
        ))
    }
}
