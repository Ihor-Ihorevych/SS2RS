#![no_main]

use libfuzzer_sys::fuzz_target;
use ss2_format_skl::Skeleton;

fuzz_target!(|data: &[u8]| {
    if let Ok(skeleton) = Skeleton::parse(data) {
        for lod in skeleton.lods() {
            for bone in lod.bones() {
                assert_eq!(lod.bone(bone.id), Some(bone));
                assert!(bone.local_rotation.is_normalized());
                assert!(bone.parent.is_none_or(|id| lod.bone(id).is_some()));
            }
        }
    }
});
