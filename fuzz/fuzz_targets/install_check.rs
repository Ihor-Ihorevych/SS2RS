//! `ss2-doctor` on a folder whose archives hold arbitrary bytes: always a report, never a panic.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let dir = std::env::temp_dir().join(format!("ss2-fuzz-install-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    // The first byte picks which of a few names to write; the rest is the content, so both
    // damaged base archives and strange patch or foreign names are reached.
    let Some((&pick, content)) = data.split_first() else {
        return;
    };
    let names = [
        "All_PC_01.gro",
        "Patch_02_100_800000.gro",
        "mod.GRO",
        "Patch_x_y.gro",
        "Movies_PC_01.gro",
    ];
    let name = names[usize::from(pick) % names.len()];
    let file = dir.join(name);
    if std::fs::write(&file, content).is_err() {
        return;
    }
    let report = ss2_vfs::check_install(&dir);
    let _ = report.render();
    let _ = report.exit_code();
    let _ = std::fs::remove_file(&file);
});
