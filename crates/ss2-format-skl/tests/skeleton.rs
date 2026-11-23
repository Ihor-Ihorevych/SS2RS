mod support;

use std::sync::Arc;

use ss2_assets::AssetManager;
use ss2_core::math::{Quat, Vec3};
use ss2_format_skl::{Skeleton, SkeletonLoader};
use ss2_vfs::{EntryKind, MemoryVfs, Vfs, VfsPath, mount_install};

#[test]
fn registered_loader_preserves_lod_order_sparse_ids_and_source_transforms() {
    let bytes = support::skeleton(
        &[&[(1, 0), (4, 1), (9, 0)], &[(4, 0), (9, 4)]],
        [0.0, 0.0, 0.600_12, 0.800_16],
        "CSkeleton",
    );
    let assets = AssetManager::new(Arc::new(
        MemoryVfs::from_files([("sample.skl", &bytes[..])]).unwrap(),
    ));
    assets.register_loader(SkeletonLoader).unwrap();
    let handle = assets.load::<Skeleton>("SAMPLE.SKL").unwrap();
    let skeleton = handle
        .get()
        .unwrap_or_else(|| panic!("{:?}", handle.error()));
    let lods = skeleton.lods();
    assert_eq!(lods.len(), 2);
    assert_eq!(
        (lods[0].switch_distance, lods[1].switch_distance),
        (10.0, 30.0)
    );
    assert_eq!(lods[0].platform_mask, u32::MAX);
    assert_eq!(
        lods[0]
            .bones()
            .iter()
            .map(|bone| bone.id)
            .collect::<Vec<_>>(),
        [1, 4, 9]
    );
    assert_eq!(lods[0].bone(4).unwrap().parent, Some(1));
    assert_eq!(lods[1].bone(4).unwrap().parent, None);
    assert_eq!(lods[1].bone(9).unwrap().parent, Some(4));
    assert!(lods[1].bone(1).is_none());
    let bone = lods[1].bone(9).unwrap();
    assert_eq!(bone.local_translation, Vec3::new(1.0, 2.0, 3.0));
    assert!(bone.local_rotation.is_normalized());
    assert!(
        bone.local_rotation
            .abs_diff_eq(Quat::from_xyzw(0.0, 0.0, 0.6, 0.8), 1e-6)
    );
    assert_eq!(
        bone.inverse_bind.transform_point3(Vec3::X),
        Vec3::new(-5.0, -5.0, -7.0)
    );
    assert_eq!(bone.length, 3.75);
    assert_eq!(bone.angle_min, Vec3::new(-3.0, -2.0, -1.0));
    assert_eq!(bone.angle_max, Vec3::new(1.0, 2.0, 3.0));
    assert_eq!(bone.ik_flags, 5);
    assert_eq!(bone.raw_member_3, 0.25);
}

#[test]
fn invalid_bones_report_the_problem_with_the_hierarchy_or_rotation() {
    for (bones, description) in [
        (&[(4, 0), (4, 0)][..], "duplicate"),
        (&[(4, 9), (9, 4)][..], "earlier"),
        (&[(4, 3)][..], "earlier"),
        (&[(0, 0)][..], "positive"),
    ] {
        let bytes = support::skeleton(&[bones], Quat::IDENTITY.to_array(), "CSkeleton");
        let error = Skeleton::parse(&bytes).unwrap_err().to_string();
        assert!(error.contains(description), "{error}");
    }
    let bytes = support::skeleton(&[&[(1, 0)]], [0.0; 4], "CSkeleton");
    assert!(
        Skeleton::parse(&bytes)
            .unwrap_err()
            .to_string()
            .contains("quaternion")
    );
}

#[test]
fn unrelated_ctsemeta_objects_are_rejected() {
    let bytes = support::skeleton(&[&[(1, 0)]], Quat::IDENTITY.to_array(), "CUnrelated");
    assert!(
        Skeleton::parse(&bytes)
            .unwrap_err()
            .to_string()
            .contains("CSkeleton")
    );
}

#[test]
fn loads_every_skeleton_in_the_local_install() {
    let dir = ss2_testkit::require_game_data!();
    let vfs = Arc::new(mount_install(dir).unwrap());
    let assets = AssetManager::new(vfs.clone());
    assets.register_loader(SkeletonLoader).unwrap();
    let mut dirs = vec![VfsPath::root()];
    let mut files = 0;
    let mut lods = 0;
    let mut bones = 0;

    while let Some(dir) = dirs.pop() {
        for entry in vfs.list(&dir).unwrap() {
            let path = dir.join(&entry.name).unwrap();
            if entry.kind == EntryKind::Dir {
                dirs.push(path);
            } else if path.extension() == Some("skl") {
                let handle = assets.load::<Skeleton>(path.as_str()).unwrap();
                let skeleton = handle
                    .get()
                    .unwrap_or_else(|| panic!("{path}: {:?}", handle.error()));
                files += 1;
                lods += skeleton.lods().len();
                bones += skeleton
                    .lods()
                    .iter()
                    .map(|lod| lod.bones().len())
                    .sum::<usize>();
            }
        }
    }
    assert!(files > 0 && lods > 0 && bones > 0, "no skeletons found");
    eprintln!("loaded {files} skeletons with {lods} LODs and {bones} bones");
}
