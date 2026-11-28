//! The skinning path on the player's own data (T04-006): a real skinned mesh, its skeleton and one of its animations,
//! posed at a few times. Needs `SS2_DATA_DIR`; skips cleanly without it. Nothing from the game is written anywhere.
//!
//! The model is found, not named: the first skeleton (in path order) whose model folder holds a skinned PC surface
//! whose bone palette names bones of the skeleton's first LOD (`docs/re/mdl.md`: palette entries are 1-based bone
//! ids) and an animation that turns one of the bones moving that surface, its tracks bound to bones **by name**
//! (`docs/re/ans.md`, "Target ids" and reader invariant 3, T04-045: a track's `target_id` indexes its own file's
//! identifier table; the skeleton's table holds each bone's name at the bone's id).
//!
//! A second test checks the binding on the spec's test vectors ("Test vectors for the binding rule").

use ss2_core::math::{Quat, Vec3};
use ss2_format_ans::{AnimSet, Animation, LoopMode};
use ss2_format_bmf::{Mesh, Surface};
use ss2_format_skl::{Skeleton, SkeletonLod};
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
    lod_data(skl.lods().first()?)
}

/// A skeleton LOD as boundary data, with its bone names.
fn lod_data(lod: &SkeletonLod) -> Option<SkeletonData> {
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
    .ok()?
    .with_bone_names(lod.bones().iter().map(|b| b.name.clone()))
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

/// How many of the animation's rotation tracks name bones of the skeleton.
fn driven_bones(animation: &Animation, skeleton: &SkeletonData) -> usize {
    animation
        .rotations
        .iter()
        .filter(|t| skeleton.index_of_name(&t.target).is_some())
        .count()
}

/// How many of the animation's translation tracks carry the reciprocal of their bone's length as their length hint
/// (`docs/re/ans.md`, "Translation member `len`"): a sign of whether the tracks are bound to the right bones.
fn fitting_length_hints(animation: &Animation, lod: &SkeletonLod) -> usize {
    animation
        .translations
        .iter()
        .filter(|t| {
            lod.bone_by_name(&t.target)
                .is_some_and(|b| (t.inverse_bone_length * b.length - 1.0).abs() < 1e-3)
        })
        .count()
}

/// A local pose of `animation` at `time`, its tracks bound to `skeleton`'s bones by name.
fn pose_at(
    skeleton: &SkeletonData,
    animation: &Animation,
    time: f64,
) -> Vec<ss2_render_api::skinning::BoneTransform> {
    let pose = animation.sample(time, LoopMode::Once);
    skeleton.pose_from_named_tracks(
        animation
            .translations
            .iter()
            .map(|t| t.target.as_str())
            .zip(pose.translations.iter().copied()),
        animation
            .rotations
            .iter()
            .map(|t| t.target.as_str())
            .zip(pose.rotations.iter().copied()),
    )
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
        skeleton.index_of_name(&t.target).is_some_and(|i| {
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
                return Some(Found {
                    what: format!(
                        "{} + {} + animation {:?} ({} of {} translation length hints fit the bone bound by name)",
                        skl_path.as_str(),
                        bmf_path.as_str(),
                        animation.name,
                        fitting_length_hints(&animation, &skl.lods()[0]),
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
        let local = pose_at(&skeleton, &animation, time);
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
    // The animation turns a bone of the surface, so the surface moves; that the tracks reach the right bones is checked
    // on the spec's vectors below.
    eprintln!("largest move from the bind pose: {moved:.4}");
    assert!(moved > 1e-3, "the animation moved no vertex");
}

/// One row of `docs/re/ans.md`, "Test vectors for the binding rule (T01-430)".
struct Vector {
    skeleton: &'static str,
    animations: &'static str,
    animation: &'static str,
    bones: usize,
    envelopes: usize,
    /// Envelopes whose raw `target_id` happens to be a bone id of the skeleton (the spec's "direct-id matches").
    direct: usize,
    /// The animation was exported against this skeleton, so every length hint fits its bone (`ans.md`, "Translation
    /// member `len`"); a shared animation carries the hints of the skeleton it was exported with.
    own: bool,
}

const VECTORS: [Vector; 3] = [
    Vector {
        skeleton: "Content/SeriousSam2/Models/Enemies/ScorpSoldier/Sources/Skeleton.skl",
        animations: "Content/SeriousSam2/Models/Enemies/ScorpSoldier/Animations/Scorp.ans",
        animation: "ScorpSoldier_Idle",
        bones: 40,
        envelopes: 80,
        direct: 56,
        own: true,
    },
    Vector {
        skeleton: "Content/SeriousSam2/Models/Enemies/Tank/Sources/Tank.skl",
        animations: "Content/SeriousSam2/Models/Enemies/Tank/Animations/Animations.ans",
        animation: "Walk",
        bones: 15,
        envelopes: 30,
        direct: 4,
        own: true,
    },
    Vector {
        skeleton: "Content/SeriousSam2/Models/Npcs/Brimland/Sources/Skeletons/Female.skl",
        // The spec's table names the shared `Biped_Rides.ans`, which has no `BackPedal`; the animation (48 envelopes,
        // 0 direct-id matches, as the table says) is in the shared `Biped_Movement.ans` next to it.
        animations: "Content/SeriousSam2/Models/Animations/Biped_Movement.ans",
        animation: "BackPedal",
        bones: 24,
        envelopes: 48,
        direct: 0,
        own: false,
    },
];

fn read_vector(vfs: &dyn Vfs, v: &Vector) -> (Skeleton, Animation) {
    let read = |p: &str| {
        vfs.read(&VfsPath::new(p).unwrap())
            .unwrap_or_else(|e| panic!("{p}: {e}"))
    };
    let skl = Skeleton::parse(&read(v.skeleton)).unwrap();
    // A shared file may hold several animation sets: look in each.
    let bytes = read(v.animations);
    let container = ss2_meta::Container::parse(&bytes).unwrap();
    let decoded = container.decode_objects().unwrap();
    let animation = decoded
        .objects
        .iter()
        .filter(|o| decoded.types.type_name(o.type_id) == Some("CAnimSet"))
        .filter_map(|o| AnimSet::from_object(&container, &decoded, o.id).ok())
        .find_map(|set| set.animation(v.animation).cloned())
        .unwrap_or_else(|| panic!("{}: no animation {}", v.animations, v.animation));
    (skl, animation)
}

#[test]
fn tracks_bind_to_bones_by_name_on_the_spec_vectors() {
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    for v in &VECTORS {
        let (skl, animation) = read_vector(&vfs, v);
        let lod = &skl.lods()[0];
        assert_eq!(lod.bones().len(), v.bones, "{}", v.skeleton);
        let skeleton = lod_data(lod).unwrap();
        let names: Vec<&str> = animation
            .translations
            .iter()
            .map(|t| t.target.as_str())
            .chain(animation.rotations.iter().map(|t| t.target.as_str()))
            .collect();
        let ids: Vec<u32> = animation
            .translations
            .iter()
            .map(|t| t.target_id)
            .chain(animation.rotations.iter().map(|t| t.target_id))
            .collect();
        let by_name = names
            .iter()
            .filter(|n| skeleton.index_of_name(n).is_some())
            .count();
        // The old reading: the raw `target_id` as a bone id. It is a bone id for a minority, and the bone it names is
        // rarely the track's own.
        let direct = ids.iter().filter(|id| lod.bone(**id).is_some()).count();
        let same_bone = names
            .iter()
            .zip(&ids)
            .filter(|(n, id)| lod.bone(**id).is_some_and(|b| b.name == **n))
            .count();
        let hints = fitting_length_hints(&animation, lod);
        eprintln!(
            "{} + {}: {by_name} of {} envelopes bound by name; raw id a bone id for {direct}, the same bone for {same_bone}; {hints} of {} length hints fit",
            v.skeleton,
            v.animation,
            names.len(),
            animation.translations.len()
        );
        assert_eq!(names.len(), v.envelopes);
        assert_eq!(by_name, v.envelopes, "every envelope names a bone");
        assert_eq!(direct, v.direct, "direct-id matches");
        assert!(
            same_bone < v.envelopes,
            "the raw id is not the bone id here"
        );
        if v.own {
            assert_eq!(
                hints,
                animation.translations.len(),
                "every length hint fits the bone bound by name"
            );
        }

        // The bound pose stays plausible: a bound translation (other than a root's, which may carry the model) keeps
        // about its bone's rest offset length across the animation. Most hold it to 5 % (bones barely stretch;
        // `ans.md`: the first key equals the rest translation for 96 % of envelopes), none strays past half or
        // double (a gait may shift a hip, as the scorpion's do by up to 27 %). A shared animation keeps the offsets
        // of the skeleton it was exported with, whose bone length its hint gives (retargeting it is not done here),
        // so its translations are measured against that length instead.
        let span = animation.span_seconds();
        let (mut checked, mut close) = (0usize, 0usize);
        for k in 0..=8 {
            let local = pose_at(&skeleton, &animation, span * f64::from(k) / 8.0);
            let palette = skeleton.model_transforms(&local).unwrap();
            assert!(palette.iter().all(|m| m.is_finite()));
            for t in &animation.translations {
                let i = skeleton.index_of_name(&t.target).unwrap();
                let bone = &skeleton.bones()[i];
                // A hint of exactly 1 is the source's length for a zero offset (`skl.md`), not a length.
                let rest = if v.own {
                    bone.rest.translation.length()
                } else if t.inverse_bone_length == 1.0 {
                    0.0
                } else {
                    1.0 / t.inverse_bone_length
                };
                if bone.parent.is_none() || rest < 1e-3 {
                    continue;
                }
                let ratio = local[i].translation.length() / rest;
                assert!(
                    (0.5..=2.0).contains(&ratio),
                    "{}: bone {} offset scaled by {ratio}",
                    v.animation,
                    t.target
                );
                checked += 1;
                close += usize::from((ratio - 1.0).abs() < 0.05);
            }
        }
        eprintln!(
            "  {close} of {checked} sampled bone offsets within 5 % of their reference length"
        );
        assert!(
            checked > 0 && close * 4 >= checked * 3,
            "{close} of {checked}"
        );
    }
}

#[test]
fn the_worked_example_reaches_bone_10() {
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let (skl, animation) = read_vector(&vfs, &VECTORS[0]);
    let lod = &skl.lods()[0];
    let skeleton = lod_data(lod).unwrap();
    let first = &animation.translations[0];
    assert_eq!((first.target_id, first.target.as_str()), (13, "RF_Foot"));
    let index = skeleton.index_of_name(&first.target).unwrap();
    assert_eq!(skeleton.bones()[index].id, 10, "RF_Foot is bone 10");
    assert_eq!(
        lod.bone(13).unwrap().name,
        "RB_Foot",
        "bone 13 is another leg"
    );
    assert!((first.inverse_bone_length * lod.bone(10).unwrap().length - 1.0).abs() < 1e-3);
    // The pose moves bone 10, not bone 13.
    let pose = animation.sample(0.0, LoopMode::Once);
    let local = pose_at(&skeleton, &animation, 0.0);
    assert_eq!(local[index].translation, Vec3::from(pose.translations[0]));
}
