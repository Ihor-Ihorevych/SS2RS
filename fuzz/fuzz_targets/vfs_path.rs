//! Any path text is normalized or refused; a normalized path is stable and never escapes upward.
#![no_main]

use libfuzzer_sys::fuzz_target;
use ss2_vfs::VfsPath;

fuzz_target!(|data: &str| {
    if let Ok(path) = VfsPath::new(data) {
        let text = path.to_string();
        let again = VfsPath::new(&text).expect("a normalized path is valid");
        assert_eq!(path, again, "normalizing twice changes the path");
        // A `..` segment is refused; a name that merely contains two dots is fine.
        assert!(
            text.split('/')
                .all(|s| s != ".." && s != "." && !s.is_empty() || text.is_empty()),
            "{text:?} has a bad segment"
        );
        assert_eq!(text, text.to_lowercase(), "paths are lower case");
        assert!(!text.contains('\\'), "{text:?} has a backslash");
        let _ = path.parent();
        let _ = path.join(data);
    }
});
