//! Skinned meshes and bone palettes as plain CPU data (RFC 0013 amendment 7, T04-006).
//!
//! A [`SkinnedMeshData`] is a triangle list whose vertices follow up to four bones each
//! ([`SkinnedVertex`]). The mesh lists the **inverse bind matrix** of each bone it uses; the
//! renderer is given a **palette** per drawn instance: one bone-to-model matrix per entry of
//! [`SkinnedMeshData::inverse_bind`], in the same order. A vertex is moved by
//!
//! ```text
//! skinned = Σ wᵢ · palette[jᵢ] · inverse_bind[jᵢ] · bind_position      (weights normalised)
//! ```
//!
//! so the bind pose (each palette entry the inverse of its inverse bind) leaves the mesh where it
//! was authored.
//!
//! The palette comes from a [`SkeletonData`]: parent-relative bone transforms (the rest pose, or
//! a pose sampled from an animation) composed down the hierarchy by
//! [`SkeletonData::model_transforms`]. The types here name no file format: the caller copies a
//! skeleton's bones and their names into [`SkeletonData`] and binds an animation's tracks to bones
//! **by name** with [`SkeletonData::pose_from_named_tracks`]. A track's numeric target id is an
//! index into *its own file's* identifier table and is not a bone id of a skeleton from another
//! file (`docs/re/ans.md`, "Target ids" and reader invariant 3); only the name is comparable. With
//! the readers of the game's files that is:
//!
//! ```ignore
//! let lod = &skl.lods()[0];
//! let skeleton = SkeletonData::new(lod.bones().iter().map(|b| SkeletonBone {
//!     id: b.id, parent: b.parent, rest: BoneTransform::new(b.local_rotation, b.local_translation),
//!     inverse_bind: b.inverse_bind }).collect())?
//!     .with_bone_names(lod.bones().iter().map(|b| b.name.clone()))?;
//! let pose = animation.sample(time, LoopMode::Loop);                    // ss2_format_ans::Pose
//! let local = skeleton.pose_from_named_tracks(
//!     animation.translations.iter().map(|t| t.target.as_str()).zip(pose.translations.iter().copied()),
//!     animation.rotations.iter().map(|t| t.target.as_str()).zip(pose.rotations.iter().copied()));
//! let palette = skeleton.model_transforms(&local)?;
//! ```
//!
//! [`SkeletonData::pose_from_tracks`] binds by bone id instead, for callers whose track ids are
//! known to be bone ids of the same skeleton.
//!
//! A model attached to a bone (a weapon in a hand) is placed with [`attachment_transform`].
//!
//! ```
//! use ss2_core::math::{Mat4, Quat, Vec3};
//! use ss2_render_api::skinning::{BoneTransform, SkeletonBone, SkeletonData, skin_matrices};
//!
//! // A root at the origin and a child one metre up; inverse binds are the inverses of the rest pose.
//! let skeleton = SkeletonData::new(vec![
//!     SkeletonBone::at_rest(1, None, BoneTransform::IDENTITY, Mat4::IDENTITY),
//!     SkeletonBone::at_rest(2, Some(1), BoneTransform::new(Quat::IDENTITY, Vec3::Y),
//!                           Mat4::from_translation(-Vec3::Y)),
//! ]).unwrap();
//! let palette = skeleton.model_transforms(&skeleton.rest_pose()).unwrap();
//! let skin = skin_matrices(&palette, &skeleton.inverse_binds()).unwrap();
//! assert!(skin.iter().all(|m| m.abs_diff_eq(Mat4::IDENTITY, 1e-6)));
//! ```

use std::collections::BTreeMap;

use ss2_core::math::{Mat4, Quat, Vec3};
use ss2_core::{Error, Result};

use crate::Aabb;

/// The most bones one skinned mesh may use (the renderer's palette holds this many matrices).
pub const MAX_BONES: usize = 256;

/// One vertex of a skinned mesh: 56 bytes, laid out for direct upload.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SkinnedVertex {
    /// Position in the bind pose, in the mesh's own space.
    pub position: [f32; 3],
    /// Unit normal in the bind pose.
    pub normal: [f32; 3],
    /// Texture coordinate, as authored.
    pub uv: [f32; 2],
    /// Four indices into the mesh's bone list ([`SkinnedMeshData::inverse_bind`]).
    pub joints: [u16; 4],
    /// The weight of each joint, not negative. They are normalised by their sum when drawn; all
    /// zero means "not skinned": the vertex stays at its bind position.
    pub weights: [f32; 4],
}

/// An indexed triangle list skinned by a bone palette.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SkinnedMeshData {
    /// The vertices, in the bind pose.
    pub vertices: Vec<SkinnedVertex>,
    /// Three indices per triangle, into `vertices`.
    pub indices: Vec<u32>,
    /// The inverse bind matrix of each bone the mesh uses (model space to bone space at bind
    /// time); a vertex joint indexes this list, and a palette has one matrix per entry. 1 to
    /// [`MAX_BONES`] entries.
    pub inverse_bind: Vec<Mat4>,
}

impl SkinnedMeshData {
    /// Check that the mesh can be drawn: whole triangles, indices in range, finite values, 1 to
    /// [`MAX_BONES`] bones, weights not negative, and every joint with a weight inside the bone
    /// list.
    ///
    /// # Errors
    /// [`Error::Invalid`] describing the first problem found.
    pub fn validate(&self) -> Result<()> {
        let bad = |detail: String| Err(Error::invalid("skinned mesh", detail));
        if self.inverse_bind.is_empty() || self.inverse_bind.len() > MAX_BONES {
            return bad(format!(
                "{} bones, a skinned mesh has 1 to {MAX_BONES}",
                self.inverse_bind.len()
            ));
        }
        if let Some(i) = self.inverse_bind.iter().position(|m| !m.is_finite()) {
            return bad(format!("inverse bind {i} is not finite"));
        }
        if !self.indices.len().is_multiple_of(3) {
            return bad(format!(
                "{} indices is not whole triangles",
                self.indices.len()
            ));
        }
        if let Some(i) = self
            .indices
            .iter()
            .find(|i| **i as usize >= self.vertices.len())
        {
            return bad(format!("index {i} but {} vertices", self.vertices.len()));
        }
        for (i, v) in self.vertices.iter().enumerate() {
            let finite = v
                .position
                .iter()
                .chain(&v.normal)
                .chain(&v.uv)
                .chain(&v.weights)
                .all(|c| c.is_finite());
            if !finite || v.weights.iter().any(|w| *w < 0.0) {
                return bad(format!("vertex {i} is not finite or has a negative weight"));
            }
            if let Some((j, _)) = v
                .joints
                .iter()
                .zip(&v.weights)
                .find(|(j, w)| **w > 0.0 && usize::from(**j) >= self.inverse_bind.len())
            {
                return bad(format!(
                    "vertex {i} uses bone {j} but the mesh has {}",
                    self.inverse_bind.len()
                ));
            }
        }
        Ok(())
    }

    /// The box around all vertices in the bind pose, or `None` for a mesh without vertices.
    pub fn bounds(&self) -> Option<Aabb> {
        Aabb::from_points(self.vertices.iter().map(|v| Vec3::from(v.position)))
    }

    /// The skinned position of vertex `index` for `palette` (one bone-to-model matrix per bone),
    /// on the CPU, by the same rule the renderer uses; `None` for a bad index or palette length.
    pub fn skinned_position(&self, index: usize, palette: &[Mat4]) -> Option<Vec3> {
        let v = self.vertices.get(index)?;
        if palette.len() != self.inverse_bind.len() {
            return None;
        }
        Some(skin_point(
            Vec3::from(v.position),
            v.joints,
            v.weights,
            |j| {
                palette
                    .get(j)
                    .zip(self.inverse_bind.get(j))
                    .map(|(p, i)| *p * *i)
            },
        ))
    }
}

/// `point` moved by the weighted blend of `skin(joint)` matrices (`None`: the joint is ignored).
fn skin_point(
    point: Vec3,
    joints: [u16; 4],
    weights: [f32; 4],
    skin: impl Fn(usize) -> Option<Mat4>,
) -> Vec3 {
    let total: f32 = weights.iter().sum();
    if total <= 0.0 {
        return point;
    }
    joints
        .iter()
        .zip(weights)
        .filter(|(_, w)| *w > 0.0)
        .filter_map(|(j, w)| skin(usize::from(*j)).map(|m| m.transform_point3(point) * (w / total)))
        .sum()
}

/// A parent-relative rigid transform: rotate, then translate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoneTransform {
    /// Unit rotation.
    pub rotation: Quat,
    /// Translation.
    pub translation: Vec3,
}

impl BoneTransform {
    /// No rotation, no translation.
    pub const IDENTITY: Self = Self {
        rotation: Quat::IDENTITY,
        translation: Vec3::ZERO,
    };

    /// A transform from its parts.
    pub fn new(rotation: Quat, translation: Vec3) -> Self {
        Self {
            rotation,
            translation,
        }
    }

    /// The matrix (for column vectors).
    pub fn to_mat4(self) -> Mat4 {
        Mat4::from_rotation_translation(self.rotation, self.translation)
    }
}

/// One bone of a [`SkeletonData`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkeletonBone {
    /// The bone's id, unique in its skeleton (parent links and [`SkeletonData::index_of`] use it;
    /// animation tracks find bones by name, [`SkeletonData::with_bone_names`]).
    pub id: u32,
    /// The parent's id; `None` for a root. The parent comes earlier in the bone list.
    pub parent: Option<u32>,
    /// The rest pose, parent-relative.
    pub rest: BoneTransform,
    /// Model space to bone space at bind time.
    pub inverse_bind: Mat4,
}

impl SkeletonBone {
    /// A bone from its parts.
    pub fn at_rest(id: u32, parent: Option<u32>, rest: BoneTransform, inverse_bind: Mat4) -> Self {
        Self {
            id,
            parent,
            rest,
            inverse_bind,
        }
    }
}

/// A bone hierarchy: bones in an order where every parent precedes its children.
#[derive(Clone, Debug, PartialEq)]
pub struct SkeletonData {
    bones: Vec<SkeletonBone>,
    /// The index of each bone's parent in `bones`.
    parents: Vec<Option<usize>>,
    /// Each bone's name, in bone order (empty: no name).
    names: Vec<String>,
    /// Name to the first bone index with that name.
    name_indices: BTreeMap<String, usize>,
}

impl SkeletonData {
    /// A skeleton from `bones`, checked.
    ///
    /// # Errors
    /// [`Error::Invalid`] if there are no bones or more than [`MAX_BONES`], an id repeats, a
    /// parent is missing or comes later, or a transform is not finite.
    pub fn new(bones: Vec<SkeletonBone>) -> Result<Self> {
        let bad = |detail: String| Err(Error::invalid("skeleton", detail));
        if bones.is_empty() || bones.len() > MAX_BONES {
            return bad(format!(
                "{} bones, a skeleton has 1 to {MAX_BONES}",
                bones.len()
            ));
        }
        let mut parents = Vec::with_capacity(bones.len());
        for (i, b) in bones.iter().enumerate() {
            if bones[..i].iter().any(|o| o.id == b.id) {
                return bad(format!("bone id {} repeats", b.id));
            }
            let finite = b.rest.rotation.is_finite()
                && b.rest.translation.is_finite()
                && b.inverse_bind.is_finite();
            if !finite {
                return bad(format!("bone {} is not finite", b.id));
            }
            let parent = match b.parent {
                None => None,
                Some(p) => match bones[..i].iter().position(|o| o.id == p) {
                    Some(index) => Some(index),
                    None => {
                        return bad(format!("bone {}'s parent {p} is not an earlier bone", b.id));
                    }
                },
            };
            parents.push(parent);
        }
        let names = vec![String::new(); bones.len()];
        Ok(Self {
            bones,
            parents,
            names,
            name_indices: BTreeMap::new(),
        })
    }

    /// The same skeleton with a name for each bone, in bone order (an empty string leaves that
    /// bone unnamed). Animation tracks find their bone by this name
    /// ([`SkeletonData::pose_from_named_tracks`]). Names match exactly; if several bones share a
    /// name, the first in bone order is the one found.
    ///
    /// # Errors
    /// [`Error::Invalid`] if the number of names is not the number of bones.
    pub fn with_bone_names<S: Into<String>>(
        mut self,
        names: impl IntoIterator<Item = S>,
    ) -> Result<Self> {
        let names: Vec<String> = names.into_iter().map(Into::into).collect();
        if names.len() != self.bones.len() {
            return Err(Error::invalid(
                "skeleton",
                format!("{} names for {} bones", names.len(), self.bones.len()),
            ));
        }
        self.name_indices.clear();
        for (i, name) in names.iter().enumerate() {
            if !name.is_empty() {
                self.name_indices.entry(name.clone()).or_insert(i);
            }
        }
        self.names = names;
        Ok(self)
    }

    /// The name of the bone at `index` in [`SkeletonData::bones`]; `None` for a bad index or an
    /// unnamed bone.
    pub fn bone_name(&self, index: usize) -> Option<&str> {
        self.names
            .get(index)
            .map(String::as_str)
            .filter(|n| !n.is_empty())
    }

    /// The position in [`SkeletonData::bones`] of the (first) bone named `name`.
    pub fn index_of_name(&self, name: &str) -> Option<usize> {
        self.name_indices.get(name).copied()
    }

    /// The bones in order.
    pub fn bones(&self) -> &[SkeletonBone] {
        &self.bones
    }

    /// The position of the bone with `id` in [`SkeletonData::bones`].
    pub fn index_of(&self, id: u32) -> Option<usize> {
        self.bones.iter().position(|b| b.id == id)
    }

    /// Every bone's inverse bind matrix, in bone order (the mesh's bone list when a mesh uses the
    /// whole skeleton).
    pub fn inverse_binds(&self) -> Vec<Mat4> {
        self.bones.iter().map(|b| b.inverse_bind).collect()
    }

    /// Every bone's rest transform, in bone order.
    pub fn rest_pose(&self) -> Vec<BoneTransform> {
        self.bones.iter().map(|b| b.rest).collect()
    }

    /// A local pose from tracks that address bones by id: the rest pose, with the translation of
    /// each bone named in `translations` and the rotation of each bone named in `rotations` (quaternions
    /// `[x, y, z, w]`, normalised here) replaced. Tracks whose id is not a bone are skipped, as are
    /// non-finite values and zero quaternions.
    ///
    /// An `.ans` track's `target_id` is **not** a bone id of a skeleton from another file; bind
    /// such tracks by name with [`SkeletonData::pose_from_named_tracks`].
    pub fn pose_from_tracks(
        &self,
        translations: impl IntoIterator<Item = (u32, [f32; 3])>,
        rotations: impl IntoIterator<Item = (u32, [f32; 4])>,
    ) -> Vec<BoneTransform> {
        let mut pose = self.rest_pose();
        for (id, t) in translations {
            if let Some(i) = self.index_of(id) {
                set_translation(&mut pose[i], t);
            }
        }
        for (id, q) in rotations {
            if let Some(i) = self.index_of(id) {
                set_rotation(&mut pose[i], q);
            }
        }
        pose
    }

    /// A local pose from animation tracks that name their bones by string (`docs/re/ans.md`,
    /// reader invariant 3): the rest pose, with the translation of the bone named by each entry of
    /// `translations` and the rotation of the bone named by each entry of `rotations`
    /// (quaternions `[x, y, z, w]`, normalised here) replaced. A track whose name is no bone of
    /// this skeleton (an animation exported for another skeleton) is skipped, as are non-finite
    /// values and zero quaternions. If two tracks name the same bone, the later one wins.
    pub fn pose_from_named_tracks<'a>(
        &self,
        translations: impl IntoIterator<Item = (&'a str, [f32; 3])>,
        rotations: impl IntoIterator<Item = (&'a str, [f32; 4])>,
    ) -> Vec<BoneTransform> {
        let mut pose = self.rest_pose();
        for (name, t) in translations {
            if let Some(i) = self.index_of_name(name) {
                set_translation(&mut pose[i], t);
            }
        }
        for (name, q) in rotations {
            if let Some(i) = self.index_of_name(name) {
                set_rotation(&mut pose[i], q);
            }
        }
        pose
    }

    /// The palette: each bone's bone-to-model matrix for the parent-relative `local` pose (one per
    /// bone, in bone order), `model(bone) = model(parent) · local(bone)`.
    ///
    /// # Errors
    /// [`Error::Invalid`] if `local` does not have one transform per bone.
    pub fn model_transforms(&self, local: &[BoneTransform]) -> Result<Vec<Mat4>> {
        if local.len() != self.bones.len() {
            return Err(Error::invalid(
                "pose",
                format!("{} transforms for {} bones", local.len(), self.bones.len()),
            ));
        }
        let mut model: Vec<Mat4> = Vec::with_capacity(local.len());
        for (l, parent) in local.iter().zip(&self.parents) {
            let m = l.to_mat4();
            model.push(match parent {
                Some(p) => model[*p] * m,
                None => m,
            });
        }
        Ok(model)
    }
}

/// Replace `bone`'s translation with `t` if it is finite.
fn set_translation(bone: &mut BoneTransform, t: [f32; 3]) {
    let t = Vec3::from(t);
    if t.is_finite() {
        bone.translation = t;
    }
}

/// Replace `bone`'s rotation with `q` normalised, if it is finite and not zero.
fn set_rotation(bone: &mut BoneTransform, q: [f32; 4]) {
    let q = Quat::from_array(q);
    if q.is_finite() && q.length_squared() > 1e-12 {
        bone.rotation = q.normalize();
    }
}

/// The skin matrices `palette[j] · inverse_bind[j]` a vertex blends (the renderer computes the
/// same on upload).
///
/// # Errors
/// [`Error::Invalid`] if the two lists differ in length.
pub fn skin_matrices(palette: &[Mat4], inverse_bind: &[Mat4]) -> Result<Vec<Mat4>> {
    if palette.len() != inverse_bind.len() {
        return Err(Error::invalid(
            "bone palette",
            format!(
                "{} matrices for {} bones",
                palette.len(),
                inverse_bind.len()
            ),
        ));
    }
    Ok(palette
        .iter()
        .zip(inverse_bind)
        .map(|(p, i)| *p * *i)
        .collect())
}

/// Where a model attached to `bone` goes in the world: the skinned instance's model-to-world
/// `instance`, times the bone's bone-to-model palette matrix, times the attachment's own `offset`
/// in bone space. `None` if `bone` is not in the palette.
pub fn attachment_transform(
    instance: Mat4,
    palette: &[Mat4],
    bone: usize,
    offset: Mat4,
) -> Option<Mat4> {
    palette.get(bone).map(|b| instance * *b * offset)
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use super::*;

    fn vertex(position: [f32; 3], joints: [u16; 4], weights: [f32; 4]) -> SkinnedVertex {
        SkinnedVertex {
            position,
            normal: [0.0, 0.0, 1.0],
            uv: [0.0, 0.0],
            joints,
            weights,
        }
    }

    /// Two bones along +Y: the root at the origin, the child at y = 1.
    fn two_bones() -> SkeletonData {
        SkeletonData::new(vec![
            SkeletonBone::at_rest(1, None, BoneTransform::IDENTITY, Mat4::IDENTITY),
            SkeletonBone::at_rest(
                2,
                Some(1),
                BoneTransform::new(Quat::IDENTITY, Vec3::Y),
                Mat4::from_translation(-Vec3::Y),
            ),
        ])
        .unwrap()
    }

    /// A strip from y = 0 to y = 2: the lower vertex on the root, the upper one on the child.
    fn strip(skeleton: &SkeletonData) -> SkinnedMeshData {
        SkinnedMeshData {
            vertices: vec![
                vertex([0.0, 0.0, 0.0], [0, 0, 0, 0], [1.0, 0.0, 0.0, 0.0]),
                vertex([0.0, 2.0, 0.0], [1, 0, 0, 0], [1.0, 0.0, 0.0, 0.0]),
                vertex([1.0, 2.0, 0.0], [1, 0, 0, 0], [1.0, 0.0, 0.0, 0.0]),
            ],
            indices: vec![0, 1, 2],
            inverse_bind: skeleton.inverse_binds(),
        }
    }

    fn near(a: Vec3, b: Vec3) -> bool {
        a.abs_diff_eq(b, 1e-5)
    }

    #[test]
    fn a_skinned_vertex_is_56_bytes() {
        assert_eq!(size_of::<SkinnedVertex>(), 56);
    }

    #[test]
    fn an_identity_pose_gives_an_identity_palette() {
        let flat = SkeletonData::new(vec![
            SkeletonBone::at_rest(1, None, BoneTransform::IDENTITY, Mat4::IDENTITY),
            SkeletonBone::at_rest(2, Some(1), BoneTransform::IDENTITY, Mat4::IDENTITY),
            SkeletonBone::at_rest(3, Some(2), BoneTransform::IDENTITY, Mat4::IDENTITY),
        ])
        .unwrap();
        let palette = flat.model_transforms(&flat.rest_pose()).unwrap();
        assert!(palette.iter().all(|m| *m == Mat4::IDENTITY));
        // The rest pose of any skeleton gives identity skin matrices.
        let s = two_bones();
        let skin = skin_matrices(
            &s.model_transforms(&s.rest_pose()).unwrap(),
            &s.inverse_binds(),
        )
        .unwrap();
        assert!(skin.iter().all(|m| m.abs_diff_eq(Mat4::IDENTITY, 1e-6)));
    }

    #[test]
    fn a_rotated_bone_moves_only_its_vertices() {
        let s = two_bones();
        let mesh = strip(&s);
        mesh.validate().unwrap();
        // Bend the child 90 degrees about +Z at its joint (y = 1).
        let local = s.pose_from_tracks([], [(2, Quat::from_rotation_z(FRAC_PI_2).to_array())]);
        let palette = s.model_transforms(&local).unwrap();
        assert!(near(
            mesh.skinned_position(0, &palette).unwrap(),
            Vec3::ZERO
        ));
        // (0, 2) is one up from the joint: rotated to one along -X.
        assert!(near(
            mesh.skinned_position(1, &palette).unwrap(),
            Vec3::new(-1.0, 1.0, 0.0)
        ));
        assert!(near(
            mesh.skinned_position(2, &palette).unwrap(),
            Vec3::new(-1.0, 2.0, 0.0)
        ));
    }

    #[test]
    fn transforms_compose_down_the_hierarchy() {
        let s = two_bones();
        let mesh = strip(&s);
        // Turning the root turns the child with it; the child's own turn adds.
        let local = s.pose_from_tracks(
            [(1, [5.0, 0.0, 0.0])],
            [
                (1, Quat::from_rotation_z(FRAC_PI_2).to_array()),
                (2, Quat::from_rotation_z(FRAC_PI_2).to_array()),
            ],
        );
        let palette = s.model_transforms(&local).unwrap();
        // The child joint: root at (5, 0), its +Y turned to -X.
        assert!(near(
            palette[1].transform_point3(Vec3::ZERO),
            Vec3::new(4.0, 0.0, 0.0)
        ));
        // The tip (0, 2): one more along the child's axis, turned 180 degrees in all: -Y.
        assert!(near(
            mesh.skinned_position(1, &palette).unwrap(),
            Vec3::new(4.0, -1.0, 0.0)
        ));
        assert!(near(
            mesh.skinned_position(0, &palette).unwrap(),
            Vec3::new(5.0, 0.0, 0.0)
        ));
    }

    #[test]
    fn weights_blend_and_zero_weights_stay_put() {
        let s = two_bones();
        let mut mesh = strip(&s);
        mesh.vertices[1].joints = [0, 1, 0, 0];
        mesh.vertices[1].weights = [0.5, 0.5, 0.0, 0.0];
        mesh.vertices[2].weights = [0.0; 4];
        let local = s.pose_from_tracks([(2, [0.0, 3.0, 0.0])], []);
        let palette = s.model_transforms(&local).unwrap();
        // Half at rest (y 2), half moved up by 2 (y 4).
        assert!(near(
            mesh.skinned_position(1, &palette).unwrap(),
            Vec3::new(0.0, 3.0, 0.0)
        ));
        assert!(near(
            mesh.skinned_position(2, &palette).unwrap(),
            Vec3::new(1.0, 2.0, 0.0)
        ));
        // Unnormalised weights are normalised.
        mesh.vertices[1].weights = [100.0, 100.0, 0.0, 0.0];
        assert!(near(
            mesh.skinned_position(1, &palette).unwrap(),
            Vec3::new(0.0, 3.0, 0.0)
        ));
        assert_eq!(mesh.skinned_position(1, &palette[..1]), None);
    }

    #[test]
    fn unknown_track_ids_and_bad_values_are_skipped() {
        let s = two_bones();
        let local = s.pose_from_tracks(
            [(9, [1.0, 1.0, 1.0]), (2, [f32::NAN, 0.0, 0.0])],
            [(77, [0.0, 0.0, 1.0, 0.0]), (1, [0.0; 4])],
        );
        assert_eq!(local, s.rest_pose());
        // A quaternion that is not unit is normalised.
        let local = s.pose_from_tracks([], [(1, [0.0, 0.0, 0.0, 2.0])]);
        assert_eq!(local[0].rotation, Quat::IDENTITY);
    }

    #[test]
    fn named_tracks_bind_to_the_bone_of_that_name() {
        let s = two_bones().with_bone_names(["Pelvis", "Spine"]).unwrap();
        assert_eq!(s.index_of_name("Spine"), Some(1));
        assert_eq!(s.bone_name(0), Some("Pelvis"));
        let local = s.pose_from_named_tracks(
            [("Spine", [0.0, 3.0, 0.0])],
            [("Pelvis", Quat::from_rotation_z(FRAC_PI_2).to_array())],
        );
        assert_eq!(local[1].translation, Vec3::new(0.0, 3.0, 0.0));
        assert!(
            local[0]
                .rotation
                .abs_diff_eq(Quat::from_rotation_z(FRAC_PI_2), 1e-6)
        );
        assert_eq!(local[0].translation, Vec3::ZERO);
        assert_eq!(local[1].rotation, Quat::IDENTITY);
        // The child sits 3 up its rotated parent's Y axis, which now points along -X.
        let model = s.model_transforms(&local).unwrap();
        assert!(near(
            model[1].transform_point3(Vec3::ZERO),
            Vec3::new(-3.0, 0.0, 0.0)
        ));
    }

    #[test]
    fn unknown_and_unnamed_bones_are_skipped_and_names_match_exactly() {
        let s = two_bones().with_bone_names(["Pelvis", ""]).unwrap();
        assert_eq!(s.bone_name(1), None);
        assert_eq!(s.bone_name(2), None);
        assert_eq!(s.index_of_name(""), None);
        let local = s.pose_from_named_tracks(
            [
                ("L_Hand", [1.0, 1.0, 1.0]),
                ("", [2.0, 2.0, 2.0]),
                ("pelvis", [3.0; 3]),
            ],
            [("Pelvis", [0.0; 4]), ("Pelvis", [f32::NAN, 0.0, 0.0, 1.0])],
        );
        assert_eq!(local, s.rest_pose());
        // Without names nothing binds.
        let unnamed = two_bones();
        assert_eq!(
            unnamed.pose_from_named_tracks([("Pelvis", [1.0; 3])], []),
            unnamed.rest_pose()
        );
    }

    #[test]
    fn duplicate_names_find_the_first_bone_and_the_last_track_wins() {
        let s = two_bones().with_bone_names(["Bone", "Bone"]).unwrap();
        assert_eq!(s.index_of_name("Bone"), Some(0));
        let local =
            s.pose_from_named_tracks([("Bone", [1.0, 0.0, 0.0]), ("Bone", [2.0, 0.0, 0.0])], []);
        assert_eq!(local[0].translation, Vec3::new(2.0, 0.0, 0.0));
        assert_eq!(local[1], s.rest_pose()[1]);
        // Renaming replaces every name.
        let s = s.with_bone_names(["A", "Bone"]).unwrap();
        assert_eq!(s.index_of_name("Bone"), Some(1));
        assert_eq!(s.index_of_name("A"), Some(0));
        assert!(two_bones().with_bone_names(["only one"]).is_err());
        assert!(two_bones().with_bone_names(["a", "b", "c"]).is_err());
    }

    #[test]
    fn bad_skeletons_and_poses_are_refused() {
        let b =
            |id, parent| SkeletonBone::at_rest(id, parent, BoneTransform::IDENTITY, Mat4::IDENTITY);
        assert!(SkeletonData::new(vec![]).is_err());
        assert!(
            SkeletonData::new(vec![b(1, None), b(1, None)]).is_err(),
            "repeated id"
        );
        assert!(
            SkeletonData::new(vec![b(2, Some(1)), b(1, None)]).is_err(),
            "parent later"
        );
        assert!(
            SkeletonData::new(vec![b(1, Some(1))]).is_err(),
            "own parent"
        );
        let mut nan = b(1, None);
        nan.inverse_bind = Mat4::from_cols_array(&[f32::NAN; 16]);
        assert!(SkeletonData::new(vec![nan]).is_err());
        let too_many = (1..=MAX_BONES as u32 + 1).map(|i| b(i, None)).collect();
        assert!(SkeletonData::new(too_many).is_err());
        let s = two_bones();
        assert!(s.model_transforms(&[BoneTransform::IDENTITY]).is_err());
        assert!(skin_matrices(&[Mat4::IDENTITY], &s.inverse_binds()).is_err());
        assert_eq!(s.index_of(2), Some(1));
        assert_eq!(s.index_of(3), None);
    }

    #[test]
    fn bad_meshes_are_refused() {
        let s = two_bones();
        let mut m = strip(&s);
        m.vertices[1].joints[0] = 2;
        assert!(m.validate().is_err(), "joint outside the bone list");
        m.vertices[1].weights = [0.0, 1.0, 0.0, 0.0];
        m.validate().unwrap();
        let mut m = strip(&s);
        m.vertices[0].weights[1] = -0.5;
        assert!(m.validate().is_err(), "negative weight");
        let mut m = strip(&s);
        m.vertices[0].weights[1] = f32::NAN;
        assert!(m.validate().is_err(), "NaN weight");
        let mut m = strip(&s);
        m.inverse_bind.clear();
        assert!(m.validate().is_err(), "no bones");
        let mut m = strip(&s);
        m.inverse_bind = vec![Mat4::IDENTITY; MAX_BONES + 1];
        assert!(m.validate().is_err(), "too many bones");
        let mut m = strip(&s);
        m.indices.push(0);
        assert!(m.validate().is_err(), "partial triangle");
        let mut m = strip(&s);
        m.indices[0] = 3;
        assert!(m.validate().is_err(), "index out of range");
        assert_eq!(strip(&s).bounds().unwrap().max, Vec3::new(1.0, 2.0, 0.0));
    }

    #[test]
    fn an_attachment_follows_its_bone() {
        let s = two_bones();
        let local = s.pose_from_tracks([], [(2, Quat::from_rotation_z(FRAC_PI_2).to_array())]);
        let palette = s.model_transforms(&local).unwrap();
        let instance = Mat4::from_translation(Vec3::new(0.0, 0.0, -10.0));
        // Held half a metre along the child bone.
        let offset = Mat4::from_translation(Vec3::new(0.0, 0.5, 0.0));
        let at = attachment_transform(instance, &palette, 1, offset).unwrap();
        assert!(near(
            at.transform_point3(Vec3::ZERO),
            Vec3::new(-0.5, 1.0, -10.0)
        ));
        assert_eq!(attachment_transform(instance, &palette, 2, offset), None);
    }
}
