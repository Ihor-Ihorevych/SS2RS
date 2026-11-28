//! Animated models (T04-044, RFC 0013 amendment 8): a model's skeleton, skinned surfaces and animations turned into
//! the format-free skinning types of `ss2_render_api::skinning` and [`SkinnedModelData`]. The rules are in the
//! parent module's documentation, "Animated models".

use std::collections::BTreeMap;

use ss2_format_ans::{AnimSet, Animation};
use ss2_format_bmf::Surface;
use ss2_format_skl::SkeletonLod;
use ss2_render_api::skinning::{
    BoneTransform, SkeletonBone, SkeletonData, SkinnedMeshData, SkinnedVertex,
};

use crate::scene::{AnimKey, BoneTrack, SkeletalAnimation, SkinnedPart};

/// The format-free skeleton of one skeleton level of detail, with its bone names (`docs/re/skl.md`; names bind the
/// animation tracks, `docs/re/ans.md` reader invariant 3).
///
/// # Errors
/// Why it cannot be used (no bones, more than `MAX_BONES`, a non-finite transform).
pub(super) fn skeleton_data(lod: &SkeletonLod) -> Result<SkeletonData, String> {
    let bones = lod
        .bones()
        .iter()
        .map(|b| SkeletonBone {
            id: b.id,
            parent: b.parent,
            rest: BoneTransform::new(b.local_rotation, b.local_translation),
            inverse_bind: b.inverse_bind,
        })
        .collect();
    SkeletonData::new(bones)
        .and_then(|s| s.with_bone_names(lod.bones().iter().map(|b| b.name.clone())))
        .map_err(|e| e.to_string())
}

/// `animation` bound to `skeleton` by bone name; `None` if it has a frame rate that is not positive, a last frame
/// before its first, or no track that names a bone of `skeleton`.
///
/// Key times are seconds since the first frame (`(frame - first) / fps`) and one pass lasts the frame count
/// `last - first + 1` over the frame rate (`docs/re/ans.md`, "Duration and looping": the loop wraps by the frame
/// count). Tracks whose name is no bone are skipped; of two tracks for one bone the later wins; scalar tracks are not
/// carried.
pub(super) fn bind_animation(
    animation: &Animation,
    skeleton: &SkeletonData,
) -> Option<SkeletalAnimation> {
    let fps = animation.fps;
    let first = i64::from(animation.first_frame);
    let frames = i64::from(animation.last_frame) - first + 1;
    if !(fps.is_finite() && fps > 0.0) || frames < 1 {
        return None;
    }
    let time = |frame: i32| (i64::from(frame) - first) as f32 / fps;
    let mut tracks: BTreeMap<usize, BoneTrack> = BTreeMap::new();
    for t in &animation.translations {
        if let Some(bone) = skeleton.index_of_name(&t.target) {
            entry(&mut tracks, bone).translations = t
                .keys
                .iter()
                .map(|k| AnimKey {
                    time: time(k.frame),
                    value: k.value,
                })
                .collect();
        }
    }
    for t in &animation.rotations {
        if let Some(bone) = skeleton.index_of_name(&t.target) {
            entry(&mut tracks, bone).rotations = t
                .keys
                .iter()
                .map(|k| AnimKey {
                    time: time(k.frame),
                    value: k.value,
                })
                .collect();
        }
    }
    if tracks.is_empty() {
        return None;
    }
    Some(SkeletalAnimation {
        name: animation.name.clone(),
        duration: frames as f32 / fps,
        tracks: tracks.into_values().collect(),
    })
}

/// The track of `bone` in `tracks`, added empty if it has none yet.
fn entry(tracks: &mut BTreeMap<usize, BoneTrack>, bone: usize) -> &mut BoneTrack {
    tracks.entry(bone).or_insert_with(|| BoneTrack {
        bone: u32::try_from(bone).unwrap_or(u32::MAX),
        ..BoneTrack::default()
    })
}

/// Every animation of `sets` (in host order, then set order) bound to `skeleton`, and the one to play
/// ([`default_animation`]).
pub(super) fn bind_animations<'a>(
    sets: impl IntoIterator<Item = &'a AnimSet>,
    skeleton: &SkeletonData,
) -> (Vec<SkeletalAnimation>, u32) {
    let animations: Vec<SkeletalAnimation> = sets
        .into_iter()
        .flat_map(|s| &s.animations)
        .filter_map(|a| bind_animation(a, skeleton))
        .collect();
    let default = default_animation(animations.iter().map(|a| a.name.as_str()));
    (animations, u32::try_from(default).unwrap_or(0))
}

/// The animation a placement plays, of animations named `names` in order: the first whose name contains `idle`
/// (ignoring case), else the first named `default` (ignoring case), else the first. The specs do not say what the
/// original plays on a placed model, so this is a choice: idle loops are what a level shows at rest; mechanisms
/// (doors, lifts) whose only animations are actions fall back to the first one, and the placement's player holds
/// its first frame rather than looping it (`crate::scene::SkinnedModelData::default_player`, T04-048).
pub(super) fn default_animation<'a>(names: impl Iterator<Item = &'a str> + Clone) -> usize {
    names
        .clone()
        .position(|n| n.to_ascii_lowercase().contains("idle"))
        .or_else(|| {
            names
                .clone()
                .position(|n| n.eq_ignore_ascii_case("default"))
        })
        .unwrap_or(0)
}

/// Whether `surface` is skinned: it has bone weights, bone indices and a non-empty palette (`docs/re/bmf.md`,
/// "Skinning").
pub(super) fn is_skinned(surface: &Surface) -> bool {
    surface.bone_weights.is_some()
        && surface.bone_indices.is_some()
        && !surface.bone_palette.is_empty()
}

/// For each palette slot of `surface`, the index of its bone in `skeleton`: palette entries are bone ids of the
/// skeleton's first level of detail (`docs/re/mdl.md`, "Bone palettes in models"); an entry that is no bone of it is
/// "no bone" (reader invariant 4), `None`.
fn palette_joints(surface: &Surface, skeleton: &SkeletonData) -> Vec<Option<u16>> {
    surface
        .bone_palette
        .iter()
        .map(|id| {
            u32::try_from(*id)
                .ok()
                .and_then(|id| skeleton.index_of(id))
                .and_then(|i| u16::try_from(i).ok())
        })
        .collect()
}

/// Append one surface to a skinned `mesh` (whose bone list is `skeleton`'s), with the base UVs from the stream of map
/// id `uv_map` (else the first) and, when `lightmap_stream` names one, that position's stream as the lightmap UVs
/// (the loader's module docs, "Lightmaps": the same rule as a static mesh's). Weights are the stored bytes over 255
/// (the renderer normalises them); a weight whose palette slot is no bone of the skeleton, or a surface without
/// skinning, gets weight 0, so such vertices stay at their bind position. Returns how many weights were dropped for
/// want of a bone.
pub(super) fn append_skinned_surface(
    mesh: &mut SkinnedMeshData,
    surface: &Surface,
    uv_map: Option<u32>,
    lightmap_stream: Option<usize>,
    skeleton: &SkeletonData,
) -> usize {
    let base = u32::try_from(mesh.vertices.len()).unwrap_or(u32::MAX);
    let joints = if is_skinned(surface) {
        palette_joints(surface, skeleton)
    } else {
        Vec::new()
    };
    if let Some(k) = lightmap_stream {
        let lightmap = surface.uv_sets.get(k);
        let uvs = mesh
            .lightmap_uvs
            .get_or_insert_with(|| vec![[0.0, 0.0]; mesh.vertices.len()]);
        uvs.extend((0..surface.positions.len()).map(|i| {
            lightmap
                .and_then(|u| u.get(i))
                .copied()
                .filter(|u| u.iter().all(|c| c.is_finite()))
                .unwrap_or([0.0, 0.0])
        }));
    }
    let stream = uv_map
        .and_then(|id| surface.uv_map_ids.iter().position(|m| *m == id))
        .filter(|i| *i < surface.uv_sets.len())
        .unwrap_or(0);
    let uvs = surface.uv_sets.get(stream);
    let mut dropped = 0;
    for (i, position) in surface.positions.iter().enumerate() {
        let normal = surface
            .normals
            .get(i)
            .copied()
            .filter(|n| n.iter().all(|c| c.is_finite()))
            .unwrap_or([0.0, 1.0, 0.0]);
        let uv = uvs
            .and_then(|u| u.get(i))
            .copied()
            .filter(|u| u.iter().all(|c| c.is_finite()))
            .unwrap_or([0.0, 0.0]);
        let mut vertex = SkinnedVertex {
            position: *position,
            normal,
            uv,
            joints: [0; 4],
            weights: [0.0; 4],
        };
        if !joints.is_empty()
            && let (Some(w), Some(j)) = (
                surface.bone_weights.as_ref().and_then(|w| w.get(i)),
                surface.bone_indices.as_ref().and_then(|j| j.get(i)),
            )
        {
            for k in 0..4 {
                if w[k] == 0 {
                    continue;
                }
                match joints.get(usize::from(j[k])).copied().flatten() {
                    Some(joint) => {
                        vertex.joints[k] = joint;
                        vertex.weights[k] = f32::from(w[k]) / 255.0;
                    }
                    None => dropped += 1,
                }
            }
        }
        mesh.vertices.push(vertex);
    }
    mesh.indices
        .extend(surface.indices.iter().map(|i| base + u32::from(*i)));
    dropped
}

/// The skinned parts of one level of detail's `surfaces`, one per material: `material_of(i)` gives surface `i`'s scene
/// material and base-colour UV map. With `lightmap_stream`, every part also gets lightmap UVs from that position
/// (the loader's module docs, "Lightmaps"; chosen by `lightmap::choose`, the same rule as a static mesh's). Parts
/// that are empty or fail validation are left out (their errors returned).
pub(super) fn skinned_parts(
    surfaces: &[Surface],
    skeleton: &SkeletonData,
    lightmap_stream: Option<usize>,
    mut material_of: impl FnMut(usize) -> (u32, Option<u32>),
) -> (Vec<SkinnedPart>, Vec<String>, usize) {
    let mut parts: Vec<SkinnedPart> = Vec::new();
    let mut part_of: BTreeMap<u32, usize> = BTreeMap::new();
    let mut dropped = 0;
    for (i, surface) in surfaces.iter().enumerate() {
        let (material, uv_map) = material_of(i);
        let p = *part_of.entry(material).or_insert_with(|| {
            parts.push(SkinnedPart {
                mesh: SkinnedMeshData {
                    vertices: Vec::new(),
                    indices: Vec::new(),
                    inverse_bind: skeleton.inverse_binds(),
                    lightmap_uvs: None,
                },
                material,
            });
            parts.len() - 1
        });
        dropped += append_skinned_surface(
            &mut parts[p].mesh,
            surface,
            uv_map,
            lightmap_stream,
            skeleton,
        );
    }
    let mut kept = Vec::with_capacity(parts.len());
    let mut left_out = Vec::new();
    for part in parts {
        match part.mesh.validate() {
            Ok(()) if !part.mesh.indices.is_empty() => kept.push(part),
            Ok(()) => {}
            Err(e) => left_out.push(e.to_string()),
        }
    }
    (kept, left_out, dropped)
}

#[cfg(test)]
mod tests {
    use ss2_core::math::{Mat4, Quat, Vec3};
    use ss2_format_ans::{Key, RotationTrack, TranslationTrack};
    use ss2_format_bmf::MaterialRef;

    use super::*;

    fn skeleton() -> SkeletonData {
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
        .with_bone_names(["Root", "Tip"])
        .unwrap()
    }

    fn animation(name: &str, targets: &[&str]) -> Animation {
        Animation {
            name: name.to_string(),
            first_frame: 10,
            last_frame: 40,
            seconds_per_frame: 1.0 / 30.0,
            fps: 30.0,
            translations: targets
                .iter()
                .map(|t| TranslationTrack {
                    target: (*t).to_string(),
                    target_id: 99,
                    inverse_bone_length: 1.0,
                    keys: vec![
                        Key {
                            frame: 10,
                            value: [0.0, 1.0, 0.0],
                        },
                        Key {
                            frame: 40,
                            value: [0.0, 2.0, 0.0],
                        },
                    ],
                })
                .collect(),
            rotations: targets
                .iter()
                .map(|t| RotationTrack {
                    target: (*t).to_string(),
                    target_id: 99,
                    keys: vec![Key {
                        frame: 25,
                        value: [0.0, 0.0, 0.0, 1.0],
                    }],
                })
                .collect(),
            scalars: Vec::new(),
            events: Vec::new(),
        }
    }

    #[test]
    fn tracks_bind_by_name_with_times_from_the_first_frame() {
        let s = skeleton();
        let bound = bind_animation(&animation("Walk", &["Tip", "L_Hand"]), &s).unwrap();
        assert_eq!(bound.name, "Walk");
        // 31 frames at 30 per second: the loop wraps by the frame count.
        assert!((bound.duration - 31.0 / 30.0).abs() < 1e-6);
        assert_eq!(bound.tracks.len(), 1, "L_Hand is no bone");
        let t = &bound.tracks[0];
        assert_eq!(t.bone, 1);
        assert_eq!(t.translations[0].time, 0.0);
        assert!((t.translations[1].time - 1.0).abs() < 1e-6);
        assert!((t.rotations[0].time - 0.5).abs() < 1e-6);
        // Halfway, the tip is half way from 1 to 2 up.
        let pose = bound.local_pose(&s, 0.5);
        assert!((pose[1].translation.y - 1.5).abs() < 1e-5);
        assert_eq!(pose[0], s.rest_pose()[0]);
        // Nothing binds, or a bad frame rate: no animation.
        assert!(bind_animation(&animation("Other", &["Nope"]), &s).is_none());
        let mut bad = animation("Walk", &["Tip"]);
        bad.fps = 0.0;
        assert!(bind_animation(&bad, &s).is_none());
        bad.fps = 30.0;
        bad.last_frame = 5;
        assert!(bind_animation(&bad, &s).is_none());
    }

    #[test]
    fn idle_then_default_then_the_first_is_played() {
        assert_eq!(default_animation(["Open", "Close"].into_iter()), 0);
        assert_eq!(
            default_animation(["Open", "Default", "Idle_01"].into_iter()),
            2
        );
        assert_eq!(default_animation(["Open", "DEFAULT"].into_iter()), 1);
        assert_eq!(default_animation(["Run", "crouchidle"].into_iter()), 1);
        let sets = [AnimSet {
            name: "Animations".to_string(),
            animations: vec![
                animation("Open", &["Tip"]),
                animation("Unbound_Idle", &["Nope"]),
                animation("Idle_01", &["Root"]),
            ],
        }];
        let (bound, default) = bind_animations(&sets, &skeleton());
        assert_eq!(bound.len(), 2, "the unbound idle is dropped");
        assert_eq!(bound[default as usize].name, "Idle_01");
    }

    fn surface(palette: Vec<i64>) -> Surface {
        Surface {
            indices: vec![0, 1, 2],
            positions: vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 2.0, 0.0]],
            normals: vec![[0.0, 0.0, 1.0]; 3],
            tangents: None,
            uv_sets: vec![vec![[0.0; 2]; 3]],
            uv_map_ids: vec![0],
            bone_weights: Some(vec![[255, 0, 0, 0], [128, 127, 0, 0], [0, 0, 0, 0]]),
            bone_indices: Some(vec![[0, 0, 0, 0], [0, 1, 0, 0], [1, 0, 0, 0]]),
            bone_palette: palette,
            material: MaterialRef::None,
        }
    }

    #[test]
    fn palette_slots_become_skeleton_bones_and_unknown_ids_weigh_nothing() {
        let s = skeleton();
        // Palette slot 0 is bone id 2 (index 1), slot 1 is bone id 9: no bone.
        let (parts, left_out, dropped) =
            skinned_parts(&[surface(vec![2, 9])], &s, None, |_| (3, None));
        assert!(left_out.is_empty());
        assert_eq!(dropped, 1);
        let mesh = &parts[0].mesh;
        assert_eq!(parts[0].material, 3);
        assert_eq!(mesh.inverse_bind, s.inverse_binds());
        assert_eq!(mesh.vertices[0].joints[0], 1);
        assert!((mesh.vertices[0].weights[0] - 1.0).abs() < 1e-6);
        // The weight on the unknown bone is dropped; the other stays on bone index 1.
        assert_eq!(mesh.vertices[1].joints[..2], [1, 0]);
        assert_eq!(mesh.vertices[1].weights[1], 0.0);
        assert_eq!(mesh.vertices[2].weights, [0.0; 4], "unweighted stays put");
        // A surface without skinning is carried unweighted.
        let mut plain = surface(Vec::new());
        plain.bone_weights = None;
        assert!(!is_skinned(&plain));
        let (parts, _, dropped) = skinned_parts(&[plain], &s, None, |_| (0, None));
        assert_eq!(dropped, 0);
        assert!(parts[0].mesh.vertices.iter().all(|v| v.weights == [0.0; 4]));
    }

    #[test]
    fn a_lightmap_stream_gives_every_part_lightmap_uvs_at_that_position() {
        let s = skeleton();
        let mut two_streams = surface(vec![2]);
        // Slot 0 is the base colour UVs, slot 1 the lightmap's.
        two_streams
            .uv_sets
            .push(vec![[0.1, 0.2], [0.3, 0.4], [0.5, 0.6]]);
        // Without a chosen stream, no lightmap UVs at all.
        let (parts, _, _) = skinned_parts(&[two_streams.clone()], &s, None, |_| (0, Some(0)));
        assert_eq!(parts[0].mesh.lightmap_uvs, None);
        // With one, every vertex gets slot 1's coordinates, independent of the base-colour UV map.
        let (parts, _, _) = skinned_parts(&[two_streams], &s, Some(1), |_| (0, Some(0)));
        assert_eq!(
            parts[0].mesh.lightmap_uvs,
            Some(vec![[0.1, 0.2], [0.3, 0.4], [0.5, 0.6]])
        );
        assert_eq!(parts[0].mesh.vertices[0].uv, [0.0, 0.0], "slot 0, as named");
    }
}
