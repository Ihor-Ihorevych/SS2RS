use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, SeekFrom};

use ss2_core::Error;
use ss2_vfs::{DirEntry, DirectoryVfs, EntryKind, LayeredVfs, MemoryVfs, Stat, Vfs, VfsPath};

fn path(raw: &str) -> VfsPath {
    VfsPath::new(raw).unwrap()
}

#[test]
fn mixed_case_loose_files_can_be_listed_and_streamed() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("Assets")).unwrap();
    fs::create_dir(root.path().join("Assets/Empty")).unwrap();
    fs::write(root.path().join("Assets/Zebra.TXT"), b"abcdef").unwrap();
    fs::write(root.path().join("Assets/Alpha.txt"), b"other").unwrap();
    let vfs = DirectoryVfs::new(root.path()).unwrap();

    assert_eq!(
        vfs.list(&path("ASSETS")).unwrap(),
        [
            DirEntry {
                name: "alpha.txt".into(),
                kind: EntryKind::File
            },
            DirEntry {
                name: "empty".into(),
                kind: EntryKind::Dir
            },
            DirEntry {
                name: "zebra.txt".into(),
                kind: EntryKind::File
            },
        ]
    );
    assert!(vfs.list(&path("assets/empty")).unwrap().is_empty());
    let zebra = path(r"Assets\zebra.txt");
    assert_eq!(
        vfs.stat(&zebra).unwrap(),
        Stat {
            kind: EntryKind::File,
            size: 6
        }
    );
    let mut file = vfs.open(&zebra).unwrap();
    file.seek(SeekFrom::Start(2)).unwrap();
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, b"cdef");
}

#[test]
fn mounted_directory_reflects_new_updated_and_removed_overrides() {
    let root = tempfile::tempdir().unwrap();
    let mut vfs = LayeredVfs::new();
    vfs.mount(
        "base",
        MemoryVfs::from_files([("config.txt", &b"base"[..])]).unwrap(),
    );
    vfs.mount("loose", DirectoryVfs::new(root.path()).unwrap());
    let key = path("CONFIG.TXT");
    let file = root.path().join("Config.txt");

    assert_eq!(vfs.read(&key).unwrap(), b"base");
    fs::write(&file, b"override").unwrap();
    assert_eq!(vfs.source_of(&key), Some("loose"));
    assert_eq!(vfs.read(&key).unwrap(), b"override");
    fs::write(&file, b"updated override").unwrap();
    assert_eq!(vfs.stat(&key).unwrap().size, 16);
    assert_eq!(vfs.read(&key).unwrap(), b"updated override");
    fs::remove_file(&file).unwrap();
    assert_eq!(vfs.source_of(&key), Some("base"));
    assert_eq!(vfs.read(&key).unwrap(), b"base");
}

#[test]
fn missing_paths_and_wrong_entry_kinds_are_reported() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("File.txt");
    fs::write(&file, b"contents").unwrap();
    assert!(matches!(
        DirectoryVfs::new(&file),
        Err(Error::Invalid { .. })
    ));
    assert!(matches!(
        DirectoryVfs::new(root.path().join("missing")),
        Err(Error::NotFound(_))
    ));
    let vfs = DirectoryVfs::new(root.path()).unwrap();
    assert!(matches!(
        vfs.stat(&path("missing")),
        Err(Error::NotFound(_))
    ));
    assert!(matches!(
        vfs.stat(&path("file.txt/child")),
        Err(Error::NotFound(_))
    ));
    assert!(matches!(
        vfs.list(&path("file.txt")),
        Err(Error::NotFound(_))
    ));
    assert!(matches!(
        vfs.open(&VfsPath::root()),
        Err(Error::NotFound(_))
    ));
}

#[test]
fn replacing_the_mounted_root_with_a_file_makes_it_unavailable() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("mount");
    fs::create_dir(&root).unwrap();
    let vfs = DirectoryVfs::new(&root).unwrap();
    assert_eq!(vfs.stat(&VfsPath::root()).unwrap().kind, EntryKind::Dir);

    fs::remove_dir(&root).unwrap();
    fs::write(&root, b"regular file").unwrap();
    assert!(matches!(
        vfs.stat(&VfsPath::root()),
        Err(Error::NotFound(_))
    ));
}

#[test]
fn colliding_host_names_are_reported_through_the_layered_mount() {
    let root = tempfile::tempdir().unwrap();
    // Lowercasing U+0130 (İ) produces two code points: `i` with a combining dot.
    // Filesystems can allow both spellings even when they reject ASCII case variants.
    fs::write(root.path().join("İtem.txt"), b"one").unwrap();
    let normalized = "i\u{307}tem.txt";
    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.path().join(normalized))
    {
        Ok(_) => {}
        Err(error) if error.kind() == ErrorKind::AlreadyExists => return,
        Err(error) => panic!("could not create the case-colliding file: {error}"),
    }
    let vfs = DirectoryVfs::new(root.path()).unwrap();
    assert!(matches!(
        vfs.stat(&path(normalized)),
        Err(Error::Invalid { .. })
    ));
    assert!(matches!(
        vfs.list(&VfsPath::root()),
        Err(Error::Invalid { .. })
    ));

    let mut layers = LayeredVfs::new();
    layers.mount(
        "base",
        MemoryVfs::from_files([(normalized, &b"base"[..])]).unwrap(),
    );
    layers.mount("loose", vfs);
    let key = path(normalized);
    assert!(matches!(layers.stat(&key), Err(Error::Invalid { .. })));
    assert!(matches!(layers.open(&key), Err(Error::Invalid { .. })));
    assert!(matches!(layers.read(&key), Err(Error::Invalid { .. })));
}

#[cfg(any(unix, windows))]
#[test]
fn symlinks_and_unrepresentable_host_names_are_hidden() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("outside.txt"), b"outside").unwrap();
    fs::write(root.path().join("Visible.txt"), b"visible").unwrap();
    #[cfg(unix)]
    {
        fs::write(root.path().join(r"bad\name.txt"), b"backslash").unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("Link")).unwrap();
    }
    #[cfg(windows)]
    match std::os::windows::fs::symlink_dir(outside.path(), root.path().join("Link")) {
        Ok(()) => {}
        Err(error) if error.raw_os_error() == Some(1314) => {
            eprintln!("skipping symlink check: Windows account cannot create symlinks");
            return;
        }
        Err(error) => panic!("could not create a symlink: {error}"),
    }
    let vfs = DirectoryVfs::new(root.path()).unwrap();
    assert_eq!(
        vfs.list(&VfsPath::root()).unwrap(),
        [DirEntry {
            name: "visible.txt".into(),
            kind: EntryKind::File
        }]
    );
    assert!(matches!(
        vfs.open(&path("link/outside.txt")),
        Err(Error::NotFound(_))
    ));
}
