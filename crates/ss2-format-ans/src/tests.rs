//! Synthetic-file tests (no game data) and the data-gated full-install test.

use std::sync::Arc;

use ss2_assets::{AssetManager, LoadState};
use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};
use ss2_vfs::MemoryVfs;

use super::*;

/// Ways to break the synthetic file.
#[derive(Clone, Copy, Default)]
struct Flaws {
    /// The translation envelope says 4 keys but lists 3.
    count_mismatch: bool,
    /// The translation keys are not ascending.
    descending: bool,
    /// A translation key lies after the last frame.
    key_out_of_range: bool,
    /// A rotation key is the zero quaternion.
    zero_quat: bool,
    /// The rotation envelope has no keys.
    empty_track: bool,
    /// The frame rate is 0.
    zero_fps: bool,
    /// The first frame is after the last.
    reversed_range: bool,
    /// The event holder lists an object that does not exist.
    dangling_event: bool,
}

fn types(b: &mut MetaBuilder) {
    b.type_block(TypeBlock::primitive(0, "SLONG", 4, 4));
    b.type_block(TypeBlock::alias(1, "INDEX", 0));
    b.type_block(TypeBlock::primitive(2, "FLOAT", 4, 4));
    b.type_block(TypeBlock::primitive(3, "IDENT", 4, 4));
    b.type_block(TypeBlock::primitive(4, "ULONG", 4, 4));
    b.type_block(TypeBlock::alias(5, "BOOL", 4));
    b.type_block(TypeBlock::enumeration(6, "AnimEventFlags", 4));
    b.type_block(TypeBlock::class(
        7,
        "Vector3f",
        1,
        5,
        None,
        &[("1", 2), ("2", 2), ("3", 2)],
    ));
    b.type_block(TypeBlock::class(
        8,
        "Quaternion4f",
        1,
        5,
        None,
        &[("1", 2), ("2", 2), ("3", 2), ("4", 2)],
    ));
    b.type_block(TypeBlock::class(9, "CAnimFrame", 1, 5, None, &[("1", 1)]));
    b.type_block(TypeBlock::class(
        10,
        "CVec3AnimFrame",
        1,
        5,
        Some(9),
        &[("1", 7)],
    ));
    b.type_block(TypeBlock::class(
        11,
        "CQuatAnimFrame",
        1,
        5,
        Some(9),
        &[("1", 8)],
    ));
    b.type_block(TypeBlock::class(
        12,
        "CFloatAnimFrame",
        1,
        5,
        Some(9),
        &[("1", 2)],
    ));
    b.type_block(TypeBlock::short(
        13,
        "CStaticArray<CVec3AnimFrame>",
        0,
        6,
        10,
    ));
    b.type_block(TypeBlock::short(
        14,
        "CStaticArray<CQuatAnimFrame>",
        0,
        6,
        11,
    ));
    b.type_block(TypeBlock::short(
        15,
        "CStaticArray<CFloatAnimFrame>",
        0,
        6,
        12,
    ));
    b.type_block(TypeBlock::class(
        16,
        "CAnimEnvelope",
        1,
        5,
        None,
        &[("1", 3), ("2", 1), ("3", 1), ("4", 1), ("5", 2)],
    ));
    b.type_block(TypeBlock::class(
        17,
        "CVec3LenAnimEnvelope",
        1,
        5,
        Some(16),
        &[("1", 2), ("2", 13)],
    ));
    b.type_block(TypeBlock::class(
        18,
        "CQuatAnimEnvelope",
        1,
        5,
        Some(16),
        &[("1", 14)],
    ));
    b.type_block(TypeBlock::class(
        19,
        "CFloatAnimEnvelope",
        1,
        5,
        Some(16),
        &[("1", 15)],
    ));
    b.type_block(TypeBlock::dynamic_container(
        20,
        "CDynamicContainer<CAnimEnvelope>",
        16,
    ));
    b.type_block(TypeBlock::class(
        21,
        "CAnimEvent",
        1,
        5,
        None,
        &[
            ("ae_fFrame", 2),
            ("ae_ulFlags", 6),
            ("ae_bOnlyInFirstLoop", 5),
        ],
    ));
    b.type_block(TypeBlock::class(
        22,
        "CSamSoundAnimEvent",
        1,
        5,
        Some(21),
        &[("ssae_idSound", 3)],
    ));
    b.type_block(TypeBlock::class(23, "CPlainAnimEvent", 1, 5, Some(21), &[]));
    b.type_block(TypeBlock::pointer(24, "CStaticSound*", 100));
    b.type_block(TypeBlock::class(
        25,
        "CFileAnimEvent",
        1,
        5,
        Some(21),
        &[("fae_pSound", 24), ("fae_fVolume", 2), ("fae_slCount", 0)],
    ));
    b.type_block(TypeBlock::dynamic_container(
        26,
        "CDynamicContainer<CAnimEvent>",
        21,
    ));
    b.type_block(TypeBlock::class(
        27,
        "CAnimEventHolder",
        1,
        5,
        None,
        &[("aeh_caeEvents", 26)],
    ));
    b.type_block(TypeBlock::pointer(28, "CAnimEventHolder*", 27));
    b.type_block(TypeBlock::class(
        29,
        "CAnimation",
        1,
        5,
        None,
        &[
            ("1", 3),
            ("2", 1),
            ("3", 1),
            ("4", 2),
            ("5", 2),
            ("6", 20),
            ("8", 28),
        ],
    ));
    b.type_block(TypeBlock::dynamic_container(
        30,
        "CDynamicContainer<CAnimation>",
        29,
    ));
    b.type_block(TypeBlock::class(
        31,
        "CAnimSet",
        1,
        5,
        None,
        &[("1", 30), ("3", 3)],
    ));
}

fn vec3_key(frame: i32, v: [f32; 3]) -> Vec<u8> {
    ObjectBytes::new()
        .i32(frame)
        .f32(v[0])
        .f32(v[1])
        .f32(v[2])
        .finish()
}

fn quat_key(frame: i32, q: [f32; 4]) -> Vec<u8> {
    ObjectBytes::new()
        .i32(frame)
        .f32(q[0])
        .f32(q[1])
        .f32(q[2])
        .f32(q[3])
        .finish()
}

fn envelope_head(target: u32, first: i32, last: i32, count: i32) -> ObjectBytes {
    ObjectBytes::new()
        .u32(target)
        .i32(first)
        .i32(last)
        .i32(count)
        .f32(1.0 / (last - first + 1) as f32)
}

const SQ: f32 = std::f32::consts::FRAC_1_SQRT_2;

/// One set `Moves` with the animation `Walk` (frames 10 to 14 at 10 fps): translation of bone
/// `Root` (keys at 10, 12, 14), rotation of `Root` (identity at 10, 90 degrees about z at 14),
/// the float channel `Glow` (0 at 10, 1 at 14) and four events.
fn file(flaws: Flaws) -> Vec<u8> {
    let mut b = MetaBuilder::new();
    let wav = b.file_ref("Content/Test/Sounds/Click.wav", 0);
    b.external_type(100, "CStaticSound");
    types(&mut b);
    let moves = b.identifier("Moves");
    let walk = b.identifier("Walk");
    let root = b.identifier("Root");
    let glow = b.identifier("Glow");
    let step = b.identifier("step_sound");
    b.external_object(50, wav, 1, 100);

    let (first, last) = if flaws.reversed_range {
        (14, 10)
    } else {
        (10, 14)
    };
    b.object(
        0,
        31,
        ObjectBytes::new()
            .dynamic_container(&[1])
            .u32(moves)
            .finish(),
    );
    let fps = if flaws.zero_fps { 0.0 } else { 10.0 };
    b.object(
        1,
        29,
        ObjectBytes::new()
            .u32(walk)
            .i32(first)
            .i32(last)
            .f32(0.1)
            .f32(fps)
            .dynamic_container(&[2, 3, 4])
            .object_ref(Some(5))
            .finish(),
    );

    // Translation: (0,0,0) at 10, (2,0,0) at 12, (2,4,0) at 14.
    let mut keys = vec![
        vec3_key(10, [0.0; 3]),
        vec3_key(12, [2.0, 0.0, 0.0]),
        vec3_key(14, [2.0, 4.0, 0.0]),
    ];
    if flaws.descending {
        keys.swap(1, 2);
    }
    if flaws.key_out_of_range {
        keys[2] = vec3_key(15, [2.0, 4.0, 0.0]);
    }
    let listed = if flaws.count_mismatch { 4 } else { 3 };
    let mut t = envelope_head(root, 10, 14, listed)
        .f32(0.5)
        .array_header(b"STAR", 3);
    for k in &keys {
        t = t.raw(k);
    }
    b.object(2, 17, t.finish());

    // Rotation.
    let zero = if flaws.zero_quat {
        [0.0; 4]
    } else {
        [0.0, 0.0, SQ, SQ]
    };
    let r = if flaws.empty_track {
        envelope_head(root, 10, 14, 0).array_header(b"STAR", 0)
    } else {
        envelope_head(root, 10, 14, 2)
            .array_header(b"STAR", 2)
            .raw(&quat_key(10, [0.0, 0.0, 0.0, 1.0]))
            .raw(&quat_key(14, zero))
    };
    b.object(3, 18, r.finish());

    // Scalar channel.
    b.object(
        4,
        19,
        envelope_head(glow, 10, 14, 2)
            .array_header(b"STAR", 2)
            .i32(10)
            .f32(0.0)
            .i32(14)
            .f32(1.0)
            .finish(),
    );

    // Events: `Early` before the range, a sound at 11, a first-loop-only click at 13 and a file
    // event at 14.
    let holder_ids: &[u32] = if flaws.dangling_event {
        &[6, 99]
    } else {
        &[6, 7, 8, 9]
    };
    b.object(
        5,
        27,
        ObjectBytes::new().dynamic_container(holder_ids).finish(),
    );
    b.object(
        6,
        22,
        ObjectBytes::new()
            .f32(11.0)
            .u32(1)
            .u32(0)
            .u32(step)
            .finish(),
    );
    b.object(7, 23, ObjectBytes::new().f32(13.0).u32(4).u32(1).finish());
    b.object(
        8,
        25,
        ObjectBytes::new()
            .f32(14.0)
            .u32(0)
            .u32(0)
            .object_ref(Some(50))
            .f32(0.5)
            .i32(-3)
            .finish(),
    );
    b.object(9, 23, ObjectBytes::new().f32(9.0).u32(0).u32(0).finish());
    b.build().unwrap()
}

fn set() -> AnimSet {
    AnimSet::from_bytes(&file(Flaws::default())).unwrap()
}

fn err(flaws: Flaws) -> String {
    AnimSet::from_bytes(&file(flaws)).unwrap_err().to_string()
}

fn near(a: [f32; 3], b: [f32; 3]) -> bool {
    a.iter().zip(&b).all(|(x, y)| (x - y).abs() < 1e-5)
}

#[test]
fn reads_names_header_tracks_and_events() {
    let s = set();
    assert_eq!(s.name, "Moves");
    assert_eq!(s.animations.len(), 1);
    let a = s.animation("walk").unwrap();
    assert_eq!(a.name, "Walk");
    assert_eq!((a.first_frame, a.last_frame), (10, 14));
    assert_eq!((a.seconds_per_frame, a.fps), (0.1, 10.0));
    assert_eq!((a.frame_count(), a.span_seconds()), (5, 0.4));
    assert_eq!(a.translations.len(), 1);
    let t = &a.translations[0];
    assert_eq!((t.target.as_str(), t.inverse_bone_length), ("Root", 0.5));
    assert_eq!(t.keys.len(), 3);
    assert_eq!(t.keys[1].frame, 12);
    assert_eq!(t.keys[1].value, [2.0, 0.0, 0.0]);
    assert_eq!(a.rotations[0].keys.len(), 2);
    assert_eq!(a.scalars[0].target, "Glow");
    assert_eq!(a.translation_index("ROOT"), Some(0));
    assert_eq!(a.rotation_index("nope"), None);
    assert_eq!(a.scalar_index("glow"), Some(0));
    assert!(s.animation("Run").is_none());
}

#[test]
fn frame_numbers_at_the_extremes_do_not_overflow() {
    // Found by the fuzz sweep of T02-030: `last_frame - first_frame` was computed in 32 bits.
    let mut a = set().animations[0].clone();
    a.first_frame = i32::MIN;
    a.last_frame = i32::MAX;
    assert_eq!(a.span_seconds(), (u32::MAX as f64) / f64::from(a.fps));
    assert_eq!(a.loop_period_frames(), u32::MAX as f64);
    assert!(a.frame_at(1.0e9, LoopMode::Once) <= f64::from(i32::MAX));
    assert!(a.frame_at(1.0e9, LoopMode::Loop).is_finite());
    let _ = a.sample(5.0, LoopMode::Loop);
    let _ = a.events_between(0.0, 10.0, LoopMode::Loop);
}

#[test]
fn rotation_keys_are_normalised_on_load() {
    let s = set();
    let q = s.animations[0].rotations[0].keys[1].value;
    let n: f32 = q.iter().map(|c| c * c).sum();
    assert!((n - 1.0).abs() < 1e-6, "{q:?}");
}

#[test]
fn events_keep_their_class_members() {
    let a = set().animations.remove(0);
    assert_eq!(a.events.len(), 4);
    let sound = &a.events[0];
    assert_eq!(sound.class, "CSamSoundAnimEvent");
    assert_eq!(
        (sound.frame, sound.flags, sound.only_in_first_loop),
        (11.0, 1, false)
    );
    assert_eq!(sound.name_param("ssae_idSound"), Some("step_sound"));
    assert_eq!(sound.params[0].owner, "CSamSoundAnimEvent");
    let click = &a.events[1];
    assert_eq!(
        (click.class.as_str(), click.flags, click.only_in_first_loop),
        ("CPlainAnimEvent", 4, true)
    );
    assert!(click.params.is_empty());
    let file = &a.events[2];
    assert_eq!(
        file.param("fae_pSound"),
        Some(&EventValue::File("Content/Test/Sounds/Click.wav".into()))
    );
    assert_eq!(file.param("fae_fVolume"), Some(&EventValue::Float(0.5)));
    assert_eq!(file.param("fae_slCount"), Some(&EventValue::Int(-3)));
    assert_eq!(file.name_param("fae_fVolume"), None);
    assert_eq!(file.param("missing"), None);
}

#[test]
fn bad_files_are_refused_with_a_reason() {
    assert!(
        err(Flaws {
            count_mismatch: true,
            ..Flaws::default()
        })
        .contains("says 4 keys but lists 3")
    );
    assert!(
        err(Flaws {
            descending: true,
            ..Flaws::default()
        })
        .contains("strictly ascend")
    );
    assert!(
        err(Flaws {
            key_out_of_range: true,
            ..Flaws::default()
        })
        .contains("outside the animation's frames")
    );
    assert!(
        err(Flaws {
            zero_quat: true,
            ..Flaws::default()
        })
        .contains("non-zero quaternion")
    );
    assert!(
        err(Flaws {
            empty_track: true,
            ..Flaws::default()
        })
        .contains("no keys")
    );
    assert!(
        err(Flaws {
            zero_fps: true,
            ..Flaws::default()
        })
        .contains("frame rate")
    );
    assert!(
        err(Flaws {
            reversed_range: true,
            ..Flaws::default()
        })
        .contains("after last frame")
    );
    assert!(
        err(Flaws {
            dangling_event: true,
            ..Flaws::default()
        })
        .contains("not in the file")
    );
    // The messages name the animation and the track.
    assert!(
        err(Flaws {
            descending: true,
            ..Flaws::default()
        })
        .contains("animation `Walk`, track `Root`")
    );
}

#[test]
fn wrong_files_and_junk_are_refused() {
    assert!(AnimSet::from_bytes(b"junk").is_err());
    assert!(AnimSet::from_bytes(&[]).is_err());
    // A valid container with no CAnimSet.
    let mut b = MetaBuilder::new();
    b.type_block(TypeBlock::primitive(0, "SLONG", 4, 4));
    b.type_block(TypeBlock::class(1, "CCounter", 1, 5, None, &[("1", 0)]));
    b.object(0, 1, ObjectBytes::new().i32(1).finish());
    let e = AnimSet::from_bytes(&b.build().unwrap())
        .unwrap_err()
        .to_string();
    assert!(e.contains("no CAnimSet"), "{e}");
}

#[test]
fn nothing_panics_on_damaged_files() {
    let good = file(Flaws::default());
    for len in 0..good.len() {
        let _ = AnimSet::from_bytes(&good[..len]);
    }
    for i in 0..good.len() {
        for x in [0x01u8, 0x80, 0xFF] {
            let mut bad = good.clone();
            bad[i] ^= x;
            let _ = AnimSet::from_bytes(&bad);
        }
    }
}

#[test]
fn the_loader_plugs_into_the_asset_manager() {
    let bytes = file(Flaws::default());
    let vfs = MemoryVfs::from_files([("a/walk.ans", bytes.as_slice())]).unwrap();
    let assets = AssetManager::new(Arc::new(vfs));
    assets.register_loader(AnsLoader).unwrap();
    let h = assets.load::<AnimSet>("A/WALK.ANS").unwrap();
    assert_eq!(h.wait(), LoadState::Ready);
    assert_eq!(h.get().unwrap().animations[0].name, "Walk");
}

// ---- sampling ----

#[test]
fn translation_interpolates_linearly_between_sparse_keys() {
    let a = set().animations.remove(0);
    assert!(near(a.sample_frame(10.0).translations[0], [0.0, 0.0, 0.0]));
    assert!(near(a.sample_frame(11.0).translations[0], [1.0, 0.0, 0.0]));
    assert!(near(a.sample_frame(12.0).translations[0], [2.0, 0.0, 0.0]));
    assert!(near(a.sample_frame(13.0).translations[0], [2.0, 2.0, 0.0]));
    assert!(near(a.sample_frame(11.5).translations[0], [1.5, 0.0, 0.0]));
    assert!(near(a.sample_frame(13.75).translations[0], [2.0, 3.5, 0.0]));
}

#[test]
fn values_are_held_outside_the_keys_and_nan_holds_the_first() {
    let a = set().animations.remove(0);
    assert!(near(a.sample_frame(-100.0).translations[0], [0.0; 3]));
    assert!(near(a.sample_frame(99.0).translations[0], [2.0, 4.0, 0.0]));
    assert!(near(a.sample_frame(f32::NAN).translations[0], [0.0; 3]));
    assert!(near(
        a.sample_frame(f32::INFINITY).translations[0],
        [2.0, 4.0, 0.0]
    ));
    assert_eq!(a.sample_frame(f32::NEG_INFINITY).scalars[0], 0.0);
}

#[test]
fn rotation_uses_the_shortest_arc_and_stays_unit_length() {
    let a = set().animations.remove(0);
    let half = a.sample_frame(12.0).rotations[0];
    // 45 degrees about z.
    let (s, c) = (
        std::f32::consts::FRAC_PI_8.sin(),
        std::f32::consts::FRAC_PI_8.cos(),
    );
    assert!(half[0].abs() < 1e-5 && half[1].abs() < 1e-5);
    assert!(
        (half[2] - s).abs() < 1e-4 && (half[3] - c).abs() < 1e-4,
        "{half:?}"
    );
    for f in [10.0, 10.3, 11.0, 12.9, 14.0] {
        let q = a.sample_frame(f).rotations[0];
        let n: f32 = q.iter().map(|c| c * c).sum();
        assert!((n - 1.0).abs() < 1e-5, "{f}: {q:?}");
    }
}

#[test]
fn slerp_edge_cases() {
    let id = [0.0, 0.0, 0.0, 1.0];
    // Opposite signs are the same rotation: no long way round.
    let neg = [0.0, 0.0, 0.0, -1.0];
    assert_eq!(sampling::slerp(id, neg, 0.5), id);
    // Nearly equal quaternions take the normalised-lerp branch and stay unit length.
    let near_id = [0.0, 0.0, 1e-4, 1.0];
    let q = sampling::slerp(id, near_id, 0.5);
    assert!((q.iter().map(|c| c * c).sum::<f32>() - 1.0).abs() < 1e-6);
    // Ends are exact.
    let z90 = [0.0, 0.0, SQ, SQ];
    let q0 = sampling::slerp(id, z90, 0.0);
    let q1 = sampling::slerp(id, z90, 1.0);
    assert!(q0.iter().zip(&id).all(|(a, b)| (a - b).abs() < 1e-6));
    assert!(q1.iter().zip(&z90).all(|(a, b)| (a - b).abs() < 1e-6));
    // A flipped sign of the end key gives the same middle.
    let z90n = z90.map(|c| -c);
    let a = sampling::slerp(id, z90, 0.5);
    let b = sampling::slerp(id, z90n, 0.5);
    assert!(a.iter().zip(&b).all(|(x, y)| (x - y).abs() < 1e-5));
}

#[test]
fn a_single_key_track_is_constant() {
    let mut a = set().animations.remove(0);
    a.translations[0].keys = vec![Key {
        frame: 12,
        value: [7.0, 8.0, 9.0],
    }];
    for f in [0.0, 12.0, 100.0] {
        assert_eq!(a.sample_frame(f).translations[0], [7.0, 8.0, 9.0]);
    }
}

#[test]
fn time_maps_to_frames_with_once_and_loop() {
    let a = set().animations.remove(0);
    // 10 fps, frames 10..=14, span 0.4 s.
    assert_eq!(a.frame_at(0.0, LoopMode::Once), 10.0);
    assert!((a.frame_at(0.25, LoopMode::Once) - 12.5).abs() < 1e-9);
    assert_eq!(a.frame_at(100.0, LoopMode::Once), 14.0);
    assert_eq!(a.loop_period_frames(), 4.0);
    assert!((a.frame_at(0.5, LoopMode::Loop) - 11.0).abs() < 1e-9);
    assert!((a.frame_at(0.4, LoopMode::Loop) - 10.0).abs() < 1e-9);
    assert!((a.frame_at(1.0, LoopMode::Loop) - 12.0).abs() < 1e-9);
    // Bad times count as zero.
    for t in [-1.0, f64::NAN, f64::NEG_INFINITY] {
        assert_eq!(a.frame_at(t, LoopMode::Loop), 10.0);
        assert_eq!(a.frame_at(t, LoopMode::Once), 10.0);
    }
    assert_eq!(a.frame_at(f64::INFINITY, LoopMode::Once), 10.0);
}

#[test]
fn sample_wraps_in_loop_mode_and_holds_in_once_mode() {
    let a = set().animations.remove(0);
    let once = a.sample(5.0, LoopMode::Once);
    assert!(near(once.translations[0], [2.0, 4.0, 0.0]));
    assert_eq!(once.scalars[0], 1.0);
    let looped = a.sample(0.5, LoopMode::Loop); // frame 11
    assert!(near(looped.translations[0], [1.0, 0.0, 0.0]));
    assert!((looped.scalars[0] - 0.25).abs() < 1e-6);
    // sample_into reuses the pose and gives the same result.
    let mut pose = Pose::default();
    a.sample_into(0.5, LoopMode::Loop, &mut pose);
    a.sample_into(0.5, LoopMode::Loop, &mut pose);
    assert_eq!(pose, looped);
    assert_eq!(pose.translations.len(), 1);
}

#[test]
fn a_one_frame_animation_loops_without_dividing_by_zero() {
    let mut a = set().animations.remove(0);
    a.first_frame = 10;
    a.last_frame = 10;
    a.translations[0].keys = vec![Key {
        frame: 10,
        value: [1.0, 2.0, 3.0],
    }];
    a.rotations[0].keys = vec![Key {
        frame: 10,
        value: [0.0, 0.0, 0.0, 1.0],
    }];
    a.scalars[0].keys = vec![Key {
        frame: 10,
        value: 5.0,
    }];
    for t in [0.0, 0.05, 3.7] {
        let p = a.sample(t, LoopMode::Loop);
        assert_eq!(p.translations[0], [1.0, 2.0, 3.0]);
        assert_eq!(p.scalars[0], 5.0);
    }
    assert_eq!(a.span_seconds(), 0.0);
}

#[test]
fn poses_blend() {
    let a = set().animations.remove(0);
    let (p0, p1) = (a.sample_frame(10.0), a.sample_frame(14.0));
    let mid = p0.blend(&p1, 0.5);
    assert!(near(mid.translations[0], [1.0, 2.0, 0.0]));
    assert!((mid.scalars[0] - 0.5).abs() < 1e-6);
    assert_eq!(p0.blend(&p1, -3.0), p0);
    assert_eq!(p0.blend(&p1, f32::NAN), p0);
    assert!(near(p0.blend(&p1, 9.0).translations[0], p1.translations[0]));
}

fn classes(v: &[FiredEvent<'_>]) -> Vec<(f32, u64)> {
    v.iter().map(|e| (e.event.frame, e.loop_index)).collect()
}

#[test]
fn events_fire_once_per_time_range_with_half_open_ranges() {
    let a = set().animations.remove(0);
    // Frame 11 is 0.1 s after the start; 13 is 0.3 s; 14 is 0.4 s; frame 9 is before the start.
    let all = a.events_between(0.0, 10.0, LoopMode::Once);
    assert_eq!(classes(&all), vec![(11.0, 0), (13.0, 0), (14.0, 0)]);
    assert!((all[0].time - 0.1).abs() < 1e-9);
    // Consecutive ticks neither miss nor repeat an event.
    let mut seen = Vec::new();
    let mut t = 0.0;
    while t < 0.6 {
        seen.extend(classes(&a.events_between(t, t + 0.0625, LoopMode::Once)));
        t += 0.0625;
    }
    assert_eq!(seen, vec![(11.0, 0), (13.0, 0), (14.0, 0)]);
    // The end of the range is exclusive, the start inclusive.
    assert!(a.events_between(0.0, 0.1, LoopMode::Once).is_empty());
    assert_eq!(a.events_between(0.1, 0.1 + 1e-6, LoopMode::Once).len(), 1);
    // Empty and reversed ranges, bad numbers.
    assert!(a.events_between(0.3, 0.3, LoopMode::Once).is_empty());
    assert!(a.events_between(0.4, 0.1, LoopMode::Once).is_empty());
    assert!(
        a.events_between(f64::NAN, f64::NAN, LoopMode::Loop)
            .is_empty()
    );
}

#[test]
fn looping_repeats_events_except_the_first_loop_only_ones() {
    let a = set().animations.remove(0);
    // Period 4 frames = 0.4 s. In pass 0 the events at frames 11 (rel 1) and 13 (rel 3) fire;
    // the frame-14 event (rel 4) and the frame-9 event (rel -1) are outside the period.
    let ev = a.events_between(0.0, 0.4, LoopMode::Loop);
    assert_eq!(classes(&ev), vec![(11.0, 0), (13.0, 0)]);
    // Pass 1 (0.4..0.8): the click at 13 is first-loop-only and stays silent.
    let ev = a.events_between(0.4, 0.8, LoopMode::Loop);
    assert_eq!(classes(&ev), vec![(11.0, 1)]);
    assert!((ev[0].time - 0.5).abs() < 1e-9);
    // A range spanning the boundary: the end of pass 0 and the start of pass 1.
    let ev = a.events_between(0.25, 0.65, LoopMode::Loop);
    assert_eq!(classes(&ev), vec![(13.0, 0), (11.0, 1)]);
    // Several passes in order.
    let ev = a.events_between(0.0, 1.2, LoopMode::Loop);
    assert_eq!(
        classes(&ev),
        vec![(11.0, 0), (13.0, 0), (11.0, 1), (11.0, 2)]
    );
    assert!(ev.windows(2).all(|w| w[0].time <= w[1].time));
}

#[test]
fn huge_time_ranges_are_bounded() {
    let a = set().animations.remove(0);
    let ev = a.events_between(0.0, 1e12, LoopMode::Loop);
    assert!(ev.len() as u64 <= MAX_LOOPS_PER_QUERY * 2);
    assert!(ev.iter().all(|e| e.loop_index >= 1));
}

#[test]
fn root_motion_delta_is_the_displacement_of_a_track() {
    let a = set().animations.remove(0);
    // Once: plain difference of the sampled positions (0.2 s = frame 12, 0.3 s = frame 13).
    let d = a.translation_delta(0, 0.2, 0.3, LoopMode::Once).unwrap();
    assert!(near(d, [0.0, 2.0, 0.0]), "{d:?}");
    // Past the end the value is held, so no more motion.
    let d = a.translation_delta(0, 0.4, 9.0, LoopMode::Once).unwrap();
    assert!(near(d, [0.0; 3]));
    // Reversed times negate.
    let d = a.translation_delta(0, 0.3, 0.2, LoopMode::Once).unwrap();
    assert!(near(d, [0.0, -2.0, 0.0]));
    // Unknown track.
    assert!(a.translation_delta(5, 0.0, 1.0, LoopMode::Once).is_none());
}

#[test]
fn root_motion_across_loops_skips_the_wrap_jump() {
    let a = set().animations.remove(0);
    // One pass moves (2, 4, 0). From 0.3 s (pass 0, frame 13: (2, 2, 0)) to 0.5 s (pass 1,
    // frame 11: (1, 0, 0)): 2 left in y (to the end), then (1, 0, 0) from the start.
    let d = a.translation_delta(0, 0.3, 0.5, LoopMode::Loop).unwrap();
    assert!(near(d, [1.0, 2.0, 0.0]), "{d:?}");
    // Three whole passes move three times the displacement.
    let d = a.translation_delta(0, 0.0, 1.2, LoopMode::Loop).unwrap();
    assert!(near(d, [6.0, 12.0, 0.0]), "{d:?}");
    // The sum of ticks equals the delta of the whole range.
    let mut sum = [0.0f32; 3];
    for i in 0..30 {
        let (t0, t1) = (f64::from(i) * 0.04, f64::from(i + 1) * 0.04);
        let d = a.translation_delta(0, t0, t1, LoopMode::Loop).unwrap();
        for k in 0..3 {
            sum[k] += d[k];
        }
    }
    let whole = a.translation_delta(0, 0.0, 1.2, LoopMode::Loop).unwrap();
    assert!(near(sum, whole), "{sum:?} vs {whole:?}");
}

/// Data-gated: every `.ans` of the retail install loads (88 files, 852 animations, the counts of
/// `ans.md`), and every track and event satisfies the reader invariants of the spec. Skips
/// without `SS2_DATA_DIR`.
#[test]
fn every_animation_set_of_the_real_install_loads() {
    use std::collections::BTreeMap;

    use ss2_vfs::{EntryKind, Vfs, VfsPath};
    let dir = ss2_testkit::require_game_data!();
    let vfs = ss2_vfs::mount_install(&dir).unwrap();
    let mut stack = vec![VfsPath::root()];
    let mut failures = Vec::new();
    let mut st: BTreeMap<&str, usize> = BTreeMap::new();
    let mut classes: BTreeMap<String, usize> = BTreeMap::new();
    let mut with_events = 0usize;
    while let Some(d) = stack.pop() {
        for e in vfs.list(&d).unwrap() {
            let path = d.join(&e.name).unwrap();
            if e.kind == EntryKind::Dir {
                stack.push(path);
                continue;
            }
            if path.extension() != Some("ans") {
                continue;
            }
            let bytes = vfs.read(&path).unwrap();
            let set = match AnimSet::from_bytes(&bytes) {
                Ok(s) => s,
                Err(e) => {
                    failures.push(format!("{}: {e}", path.as_str()));
                    continue;
                }
            };
            *st.entry("files").or_default() += 1;
            for a in &set.animations {
                *st.entry("animations").or_default() += 1;
                *st.entry("at 30 fps").or_default() += usize::from((a.fps - 30.0).abs() < 0.01);
                *st.entry("step x fps = 1").or_default() +=
                    usize::from((a.seconds_per_frame * a.fps - 1.0).abs() < 1e-4);
                *st.entry("translation tracks").or_default() += a.translations.len();
                *st.entry("rotation tracks").or_default() += a.rotations.len();
                *st.entry("scalar tracks").or_default() += a.scalars.len();
                *st.entry("scalar keys").or_default() +=
                    a.scalars.iter().map(|t| t.keys.len()).sum::<usize>();
                *st.entry("events").or_default() += a.events.len();
                with_events += usize::from(!a.events.is_empty());
                for ev in &a.events {
                    *classes.entry(ev.class.clone()).or_default() += 1;
                    *st.entry("events inside the frame range").or_default() += usize::from(
                        ev.frame >= a.first_frame as f32 && ev.frame <= a.last_frame as f32,
                    );
                }
                // Every bone with one of the two kinds has both, in the same order.
                let t: Vec<_> = a.translations.iter().map(|t| &t.target).collect();
                let r: Vec<_> = a.rotations.iter().map(|t| &t.target).collect();
                *st.entry("bones with both kinds, same order").or_default() +=
                    usize::from(t == r) * t.len();
                for tr in &a.translations {
                    *st.entry("keys on the first and last frame").or_default() += usize::from(
                        tr.keys[0].frame == a.first_frame
                            && tr.keys.last().unwrap().frame == a.last_frame,
                    );
                    *st.entry("translation tracks seen").or_default() += 1;
                }
                // Sampling every animation at both ends and in the middle gives finite, unit
                // length results.
                for time in [0.0, a.span_seconds() / 2.0, a.span_seconds() * 3.3] {
                    for mode in [LoopMode::Once, LoopMode::Loop] {
                        let p = a.sample(time, mode);
                        assert_eq!(p.translations.len(), a.translations.len());
                        assert!(p.translations.iter().flatten().all(|v| v.is_finite()));
                        assert!(p.scalars.iter().all(|v| v.is_finite()));
                        assert!(p.rotations.iter().all(|q| {
                            (q.iter().map(|c| c * c).sum::<f32>() - 1.0).abs() < 1e-3
                        }));
                    }
                }
            }
        }
    }
    eprintln!(
        "{st:#?}
animations with events: {with_events}
{classes:#?}"
    );
    assert!(
        failures.is_empty(),
        "first failures: {:?}",
        &failures[..failures.len().min(10)]
    );
    // The counts of `ans.md` (files and animations; fps; float envelopes) and the counts that
    // follow from them for the `.ans` files alone (the spec's other totals include the sets
    // embedded in `.mdl` files).
    let n = |k: &str| st.get(k).copied().unwrap_or(0);
    assert_eq!(
        (n("files"), n("animations"), n("at 30 fps")),
        (88, 852, 709)
    );
    assert_eq!(n("step x fps = 1"), 852);
    assert_eq!(n("scalar tracks"), 5074);
    assert_eq!(
        (n("translation tracks"), n("rotation tracks")),
        (14810, 14810)
    );
    assert_eq!(n("bones with both kinds, same order"), 14810);
    assert_eq!(n("keys on the first and last frame"), 14810);
    assert_eq!(
        (n("events"), n("events inside the frame range"), with_events),
        (689, 687, 238)
    );
    assert_eq!(classes.len(), 22);
    assert_eq!(classes["SPuppetSoundAnimEvent"], 297);
    assert_eq!(classes["CSamEffectAnimEvent"], 63);
}
