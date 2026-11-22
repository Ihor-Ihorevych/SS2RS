//! Any bytes presented as a `.gro` archive are opened or refused, and what opens can be listed and
//! read without a panic.
#![no_main]

use libfuzzer_sys::fuzz_target;
use ss2_vfs::{ArchiveVfs, EntryKind, Vfs, VfsPath};

fuzz_target!(|data: &[u8]| {
    let path = std::env::temp_dir().join(format!("ss2-fuzz-archive-{}.gro", std::process::id()));
    if std::fs::write(&path, data).is_err() {
        return;
    }
    let Ok(archive) = ArchiveVfs::from_file(&path) else {
        return;
    };
    let _ = archive.file_count();
    let _ = archive.skipped_entries();
    let Ok(root) = VfsPath::new("") else { return };
    let mut stack = vec![root];
    let mut visited = 0;
    while let Some(dir) = stack.pop() {
        visited += 1;
        if visited > 64 {
            break;
        }
        let Ok(entries) = archive.list(&dir) else {
            continue;
        };
        for entry in entries.into_iter().take(16) {
            let Ok(child) = dir.join(&entry.name) else {
                continue;
            };
            match entry.kind {
                EntryKind::Dir => stack.push(child),
                EntryKind::File => {
                    let _ = archive.stat(&child);
                    if let Ok(bytes) = archive.read(&child) {
                        let _ = bytes.len();
                    }
                }
            }
        }
    }
});
