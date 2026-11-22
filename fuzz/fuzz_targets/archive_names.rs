//! Archive file names of any shape are ordered and parsed for a patch level without a panic; the
//! order does not depend on the input order.
#![no_main]

use libfuzzer_sys::fuzz_target;
use ss2_vfs::{PatchLevel, archive_load_order};

fuzz_target!(|data: &str| {
    let names: Vec<String> = data.split('\n').take(32).map(str::to_owned).collect();
    let forward = archive_load_order(names.clone());
    let mut reversed = names.clone();
    reversed.reverse();
    let backward = archive_load_order(reversed);
    let lower = |v: &[String]| v.iter().map(|s| s.to_lowercase()).collect::<Vec<_>>();
    assert_eq!(
        lower(&forward),
        lower(&backward),
        "the load order depends on the input order"
    );
    for name in &names {
        let _ = PatchLevel::from_file_name(name);
    }
});
