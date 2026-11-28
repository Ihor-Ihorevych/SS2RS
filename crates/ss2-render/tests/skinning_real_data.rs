//! The skinning path on the player's own data (T04-006): a real skinned mesh, its skeleton and one of its animations,
//! posed at a few times. Needs `SS2_DATA_DIR`; skips cleanly without it. Nothing from the game is written anywhere.
//!
//! The model is found, not named: the first skeleton (in path order) whose model folder holds a skinned PC surface
//! whose bone palette names bones of the skeleton's first LOD (`docs/re/mdl.md`: palette entries are 1-based bone
//! ids) and an animation that turns one of the bones moving that surface, its tracks bound to bones by id
//! (`docs/re/ans.md`: tracks address bone ids).

use ss2_core::math::{Quat, Vec3};
use ss2_format_ans::{AnimSet, Animation, LoopMode};
use ss2_format_bmf::{Mesh, Surface};
use ss2_format_skl::Skeleton;
use ss2_render_api::skinning::{
    BoneTransform, SkeletonBone, SkeletonData, SkinnedMeshData, SkinnedVertex,
};
use ss2_vfs::{EntryKind, Vfs, VfsPath};

fn files_under(vfs: &dyn Vfs, dir: &VfsPath, extension: &str) -> Vec<VfsPath> {
    let mut found = Vec::new();
    let mut dirs = vec![dir.clone()];
    while let Some(d) = dirs.pop() {
        let Ok(entries) = vfs.list(&d) else {
            continue;
        };
        for e in entries {
            let Ok(path) = d.join(&e.name) else {
                continue;
            };
            if e.kind == EntryKind::Dir {
                dirs.push(path);
            } else if path
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case(extension))
            {
                found.push(path);
            }
        }
    }
    found.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    found
}

/// The model folder of a skeleton: the folder above `Sources`, or the skeleton's own folder.
fn model_folder(skl: &VfsPath) -> VfsPath {
    let mut p = skl.parent().unwrap_or_else(VfsPath::root);
    let own = p.clone();
    while let Some(parent) = p.parent() {
        if p.file_name()
            .is_some_and(|n| n.eq_ignore_ascii_case("Sources"))
        {
            return parent;
        }
        p = parent;
    }
    own
}

fn skeleton_data(skl: &Skeleton) -> Option<SkeletonData> {
    let lod = skl.lods().first()?;
    SkeletonData::new(
        lod.bones()
            .iter()
            .map(|b| SkeletonBone {
                id: b.id,
                parent: b.parent,
                rest: BoneTransform::new(b.local_rotation, b.local_translation),
                inverse_bind: b.inverse_bind,
            })
            .collect(),
    )
    .ok()
}

/// A skinned surface as boundary data: palette entries become skeleton bone indices; an entry that is no bone of the
/// skeleton drops its weight. `None` unless every weighted palette entry is a bone.
fn skinned_surface(surface: &Surface, skeleton: &SkeletonData) -> Option<SkinnedMeshData> {
    let (weights, joints) = (
        surface.bone_weights.as_ref()?,
        surface.bone_indices.as_ref()?,
    );
    if surface.bone_palette.is_empty() {
        return None;
    }
    let bones: Vec<Option<u16>> = surface
        .bone_palette
        .iter()
        .map(|id| {
            u32::try_from(*id)
                .ok()
                .and_then(|id| skeleton.index_of(id))
                .and_then(|i| u16::try_from(i).ok())
        })
        .collect();
    let mut vertices = Vec::with_capacity(surface.positions.len());
    for (i, position) in surface.positions.iter().enumerate() {
        let mut v = SkinnedVertex {
            position: *position,
            normal: surface.normals.get(i).copied().unwrap_or([0.0, 1.0, 0.0]),
            uv: surface
                .uv_sets
                .first()
                .and_then(|uvs| uvs.get(i))
                .copied()
                .unwrap_or_default(),
            ..SkinnedVertex::default()
        };
        for k in 0..4 {
            let w = weights[i][k];
            if w == 0 {
                continue;
            }
            v.joints[k] = (*bones.get(usize::from(joints[i][k]))?)?;
            v.weights[k] = f32::from(w) / 255.0;
        }
        vertices.push(v);
    }
    let mesh = SkinnedMeshData {
        vertices,
        indices: surface.indices.iter().map(|i| u32::from(*i)).collect(),
        inverse_bind: skeleton.inverse_binds(),
    };
    mesh.validate().ok()?;
    Some(mesh)
}

/// How many of the animation's rotation tracks address bones of the skeleton.
fn driven_bones(animation: &Animation, skeleton: &SkeletonData) -> usize {
    animation
        .rotations
        .iter()
        .filter(|t| skeleton.index_of(t.target_id).is_some())
        .count()
}

/// How many of the animation's translation tracks carry the reciprocal of their bone's length as their length hint
/// (`docs/re/ans.md`, "Translation member `len`"): a sign of whether the tracks are bound to the right bones.
fn fitting_length_hints(animation: &Animation, lengths: &[(u32, f32)]) -> usize {
    animation
        .translations
        .iter()
        .filter(|t| {
            lengths
                .iter()
                .find(|(id, _)| *id == t.target_id)
                .is_some_and(|(_, len)| (t.inverse_bone_length * len - 1.0).abs() < 1e-3)
        })
        .count()
}

/// `true` if `animation` turns, away from its rest rotation, a bone that moves `mesh`: one of its weighted joints or an
/// ancestor of one.
fn moves(animation: &Animation, skeleton: &SkeletonData, mesh: &SkinnedMeshData) -> bool {
    let bones = skeleton.bones();
    let mut affected = vec![false; bones.len()];
    for v in &mesh.vertices {
        for (j, w) in v.joints.iter().zip(v.weights) {
            let mut at = (w > 0.0).then_some(usize::from(*j));
            while let Some(i) = at {
                affected[i] = true;
                at = bones[i].parent.and_then(|p| skeleton.index_of(p));
            }
        }
    }
    animation.rotations.iter().any(|t| {
        skeleton.index_of(t.target_id).is_some_and(|i| {
            affected[i]
                && t.keys
                    .iter()
                    .any(|k| Quat::from_array(k.value).dot(bones[i].rest.rotation).abs() < 0.9999)
        })
    })
}

struct Found {
    what: String,
    skeleton: SkeletonData,
    mesh: SkinnedMeshData,
    animation: Animation,
}

fn find(vfs: &dyn Vfs) -> Option<Found> {
    for skl_path in files_under(vfs, &VfsPath::root(), "skl") {
        let Some(skl) = vfs
            .read(&skl_path)
            .ok()
            .and_then(|b| Skeleton::parse(&b).ok())
        else {
            continue;
        };
        let Some(skeleton) = skeleton_data(&skl) else {
            continue;
        };
        if skeleton.bones().len() < 3 {
            continue;
        }
        let folder = model_folder(&skl_path);
        let animations: Vec<Animation> = files_under(vfs, &folder, "ans")
            .into_iter()
            .filter_map(|p| vfs.read(&p).ok())
            .filter_map(|b| AnimSet::from_bytes(&b).ok())
            .flat_map(|set| set.animations)
            .filter(|a| driven_bones(a, &skeleton) >= 3 && a.last_frame > a.first_frame)
            .collect();
        if animations.is_empty() {
            continue;
        }
        for bmf_path in files_under(vfs, &folder, "bmf") {
            let Some(bmf) = vfs
                .read(&bmf_path)
                .ok()
                .and_then(|b| Mesh::from_bytes(&b).ok())
            else {
                continue;
            };
            let found = bmf
                .lods
                .iter()
                .filter(|l| l.is_for_pc())
                .flat_map(|l| &l.surfaces)
                .filter(|s| s.is_skinned() && !s.indices.is_empty())
                .filter_map(|s| skinned_surface(s, &skeleton))
                .find_map(|mesh| {
                    let animation = animations.iter().find(|a| moves(a, &skeleton, &mesh))?;
                    Some((mesh, animation.clone()))
                });
            if let Some((mesh, animation)) = found {
                let lengths: Vec<(u32, f32)> = skl.lods()[0]
                    .bones()
                    .iter()
                    .map(|b| (b.id, b.length))
                    .collect();
                return Some(Found {
                    what: format!(
                        "{} + {} + animation {:?} ({} of {} translation length hints fit the bone bound by id)",
                        skl_path.as_str(),
                        bmf_path.as_str(),
                        animation.name,
                        fitting_length_hints(&animation, &lengths),
                        animation.translations.len()
                    ),
                    skeleton,
                    mesh,
                    animation,
                });
            }
        }
    }
    None
}

#[test]
fn a_real_model_poses_to_a_finite_palette_and_stays_in_bounds() {
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let found = find(&vfs).expect("no skinned model with a matching skeleton and animation");
    eprintln!("posing {}", found.what);
    let Found {
        skeleton,
        mesh,
        animation,
        ..
    } = found;
    let bind = mesh.bounds().unwrap();
    let radius = bind.min.length().max(bind.max.length());

    // The rest pose undoes the inverse binds: the mesh stays where it was authored (`docs/re/skl.md`: composing the
    // local transforms and inverting gives the stored matrix to 1e-4, for all but one bone).
    let rest = skeleton.model_transforms(&skeleton.rest_pose()).unwrap();
    let mut worst = 0.0f32;
    for (i, v) in mesh.vertices.iter().enumerate() {
        let p = mesh.skinned_position(i, &rest).unwrap();
        worst = worst.max((p - Vec3::from(v.position)).length());
    }
    eprintln!("rest pose: largest vertex offset {worst}");
    assert!(
        worst < 0.05 * radius.max(1.0),
        "the rest pose moved a vertex by {worst}"
    );

    let span = animation.span_seconds();
    let mut moved = 0.0f32;
    for k in 0..5 {
        let time = span * f64::from(k) / 4.0;
        let pose = animation.sample(time, LoopMode::Once);
        let local = skeleton.pose_from_tracks(
            animation
                .translations
                .iter()
                .map(|t| t.target_id)
                .zip(pose.translations.iter().copied()),
            animation
                .rotations
                .iter()
                .map(|t| t.target_id)
                .zip(pose.rotations.iter().copied()),
        );
        let palette = skeleton.model_transforms(&local).unwrap();
        assert!(palette.iter().all(|m| m.is_finite()), "palette at {time} s");
        // Root motion may carry the model, so the bound is generous: within a few model sizes of the origin.
        let bound = 4.0 * radius + 10.0;
        let mut farthest = 0.0f32;
        for i in 0..mesh.vertices.len() {
            let p = mesh.skinned_position(i, &palette).unwrap();
            assert!(p.is_finite(), "vertex {i} at {time} s");
            farthest = farthest.max(p.length());
            moved = moved.max((p - Vec3::from(mesh.vertices[i].position)).length());
        }
        eprintln!("t = {time:.3} s: farthest vertex {farthest:.3} (bind radius {radius:.3})");
        assert!(
            farthest <= bound,
            "a vertex went {farthest} from the origin at {time} s"
        );
    }
    // The animation turns a bone of the surface, so the surface moves. Whether it moves the right way is not checked:
    // binding tracks to bones by `target_id` does not hold for every file (T04-045).
    eprintln!("largest move from the bind pose: {moved:.4}");
    assert!(moved > 1e-3, "the animation moved no vertex");
}
