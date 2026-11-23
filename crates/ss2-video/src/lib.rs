//! ss2-video — Theora cutscene playback (track T06)
//!
//! Today this crate holds the **format rule** only (T06-021); the Theora decoder is T06-010 and
//! the player is T06-011. The rule: a movie is played only if it is Ogg Theora (`.ogv`). The one
//! Xbox movie in the retail data (`.xmv`, see `docs/re/cutscenes.md`) is **ignored by design**:
//! looking it up yields [`MovieOutcome::Skipped`] and a `tracing` warning, never an error, a
//! panic or an attempt to decode it as Theora. The game goes on without that cutscene.
//!
//! Not yet connected to anything: no caller resolves level movies through
//! [`resolve_movie`] / [`level_movies`] yet (the cutscene player does not exist); they are the
//! lookup the player must use.

use ss2_vfs::{EntryKind, Nfo, Vfs, VfsPath};

/// The `.nfo` keys that name a movie (levels and episodes).
pub const MOVIE_KEYS: [&str; 3] = ["INTRO_MOVIE", "OUTRO_MOVIE", "EPISODE_MOVIE"];

/// Why `.xmv` is not played.
pub const XMV_IGNORE_REASON: &str = ".xmv is the Xbox-only video format (the PC levels use .ogv); ignored by design, see docs/VIDEO_FORMATS.md";

/// What a video file name says about its format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VideoKind {
    /// Ogg Theora/Vorbis: the format every PC movie uses.
    Ogv,
    /// Xbox video: ignored by design.
    Xmv,
    /// Anything else (no extension, another extension, a hidden file such as `.xmv`).
    Unknown,
}

impl VideoKind {
    /// Classify a file name or path (`/` and `\` both separate; only the last segment counts)
    /// by its extension, ASCII case-insensitively. `a.b.xmv` is `Xmv`; `.xmv` (a name that is
    /// only an extension) and `movie.xmv.bak` are `Unknown`.
    pub fn from_name(name: &str) -> Self {
        let file = name.rsplit(['/', '\\']).next().unwrap_or(name);
        let Some((stem, ext)) = file.rsplit_once('.') else {
            return Self::Unknown;
        };
        if stem.is_empty() {
            Self::Unknown
        } else if ext.eq_ignore_ascii_case("ogv") {
            Self::Ogv
        } else if ext.eq_ignore_ascii_case("xmv") {
            Self::Xmv
        } else {
            Self::Unknown
        }
    }

    /// Classify a normalized VFS path.
    pub fn from_path(path: &VfsPath) -> Self {
        Self::from_name(path.as_str())
    }

    /// `true` if the engine can play this kind (only `Ogv`).
    pub fn is_playable(self) -> bool {
        self == Self::Ogv
    }

    /// The reason this kind is ignored by design; `None` for `Ogv` and `Unknown`.
    pub fn ignore_reason(self) -> Option<&'static str> {
        (self == Self::Xmv).then_some(XMV_IGNORE_REASON)
    }
}

/// The result of looking a movie reference up. Never an error: every case lets the game go on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MovieOutcome {
    /// No movie is named (empty or blank reference).
    NotSet,
    /// An existing `.ogv`: play it.
    Play(VfsPath),
    /// A movie in an ignored format (`.xmv`): skip the cutscene. The file need not exist.
    Skipped {
        /// The normalized path as referenced.
        path: VfsPath,
        /// The format that is ignored.
        kind: VideoKind,
        /// Why (for the log and for tools).
        reason: &'static str,
    },
    /// An `.ogv` reference that names no file.
    Missing(VfsPath),
    /// A reference whose extension is neither `.ogv` nor `.xmv`.
    Unrecognized(VfsPath),
    /// A reference that is not a valid VFS path (`..`, `:`, control characters).
    BadPath(String),
}

impl MovieOutcome {
    /// `true` only for [`MovieOutcome::Play`].
    pub fn is_playable(&self) -> bool {
        matches!(self, Self::Play(_))
    }
}

/// Resolve a movie reference (`Content/SeriousSam2/Movies/Ntsc/1_1_intro.ogv`, any case or
/// separators) against `vfs`. A `.xmv` is skipped without touching the VFS and logged at
/// `warn`; other non-playable outcomes are logged at `warn` too.
pub fn resolve_movie(vfs: &dyn Vfs, reference: &str) -> MovieOutcome {
    let reference = reference.trim();
    if reference.is_empty() {
        return MovieOutcome::NotSet;
    }
    let path = match VfsPath::new(reference) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(reference, error = %e, "movie reference is not a valid path; cutscene skipped");
            return MovieOutcome::BadPath(reference.to_owned());
        }
    };
    match VideoKind::from_path(&path) {
        VideoKind::Xmv => {
            tracing::warn!(movie = %path, "{XMV_IGNORE_REASON}; cutscene skipped");
            MovieOutcome::Skipped {
                path,
                kind: VideoKind::Xmv,
                reason: XMV_IGNORE_REASON,
            }
        }
        VideoKind::Unknown => {
            tracing::warn!(movie = %path, "movie has an unrecognized video extension; cutscene skipped");
            MovieOutcome::Unrecognized(path)
        }
        VideoKind::Ogv => {
            if vfs.stat(&path).is_ok_and(|s| s.kind == EntryKind::File) {
                MovieOutcome::Play(path)
            } else {
                tracing::warn!(movie = %path, "movie file not found; cutscene skipped");
                MovieOutcome::Missing(path)
            }
        }
    }
}

/// Resolve every movie a level or episode `.nfo` names ([`MOVIE_KEYS`]), in key order, as
/// `(key, outcome)`; keys the `.nfo` does not have are left out.
pub fn level_movies(vfs: &dyn Vfs, nfo: &Nfo) -> Vec<(&'static str, MovieOutcome)> {
    MOVIE_KEYS
        .iter()
        .filter_map(|&key| nfo.get(key).map(|v| (key, resolve_movie(vfs, v))))
        .collect()
}

/// Every file under `dir` (recursively, sorted by path) whose name classifies as a video
/// (`Ogv` or `Xmv`), with its kind. A missing `dir` gives an empty list. This is how a mod
/// folder or the install is enumerated: an `.xmv` is listed as `Xmv` and nothing opens it.
pub fn scan_videos(vfs: &dyn Vfs, dir: &VfsPath) -> Vec<(VfsPath, VideoKind)> {
    let mut found = Vec::new();
    let mut stack = vec![dir.clone()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = vfs.list(&d) else { continue };
        for e in entries {
            let Ok(child) = d.join(&e.name) else { continue };
            match e.kind {
                EntryKind::Dir => stack.push(child),
                EntryKind::File => {
                    let kind = VideoKind::from_path(&child);
                    if kind != VideoKind::Unknown {
                        found.push((child, kind));
                    }
                }
            }
        }
    }
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss2_vfs::MemoryVfs;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct Sink(Arc<Mutex<Vec<u8>>>);
    impl std::io::Write for Sink {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Sink {
        type Writer = Sink;
        fn make_writer(&'a self) -> Sink {
            self.clone()
        }
    }

    /// Run `f` with a capturing subscriber; return its result and the log text.
    fn logged<T>(f: impl FnOnce() -> T) -> (T, String) {
        let sink = Sink::default();
        let sub = tracing_subscriber::fmt()
            .with_writer(sink.clone())
            .with_ansi(false)
            .finish();
        let out = tracing::subscriber::with_default(sub, f);
        let text = String::from_utf8(sink.0.lock().unwrap().clone()).unwrap();
        (out, text)
    }

    fn vfs() -> MemoryVfs {
        MemoryVfs::from_files([
            ("Content/Movies/Ntsc/a.ogv", &b"o"[..]),
            ("Content/Movies/Ntsc/demo_02.xmv", &b"x"[..]),
            ("Content/Movies/Ntsc/readme.txt", &b"t"[..]),
            ("Content/Mod/Deep/Er/b.OGV", &b"o"[..]),
            ("Content/Mod/Deep/Er/c.XMV", &b"x"[..]),
            ("Content/Mod/dir.xmv/inner.txt", &b"t"[..]),
        ])
        .unwrap()
    }

    #[test]
    fn classifier_table() {
        use VideoKind::*;
        let table = [
            ("a.ogv", Ogv),
            ("A.OGV", Ogv),
            ("a.OgV", Ogv),
            ("a.xmv", Xmv),
            ("A.XMV", Xmv),
            ("a.XmV", Xmv),
            ("a.b.xmv", Xmv),
            ("a.xmv.ogv", Ogv),
            ("a.ogv.xmv", Xmv),
            ("dir/sub/a.xmv", Xmv),
            ("dir\\sub\\a.OGV", Ogv),
            ("dir.xmv/a", Unknown),
            ("dir.xmv\\a.txt", Unknown),
            ("dir.xmv/", Unknown),
            ("dir\\.xmv", Unknown),
            ("a.xmv.bak", Unknown),
            ("a.xmv ", Unknown),
            ("a.xmvx", Unknown),
            ("a.mp4", Unknown),
            ("a", Unknown),
            ("ogv", Unknown),
            ("xmv", Unknown),
            (".xmv", Unknown),
            (".ogv", Unknown),
            ("dir/.xmv", Unknown),
            ("a.", Unknown),
            ("", Unknown),
            ("/", Unknown),
        ];
        for (name, want) in table {
            assert_eq!(VideoKind::from_name(name), want, "{name:?}");
        }
    }

    #[test]
    fn kind_helpers() {
        assert!(VideoKind::Ogv.is_playable());
        assert!(!VideoKind::Xmv.is_playable());
        assert!(!VideoKind::Unknown.is_playable());
        assert_eq!(VideoKind::Xmv.ignore_reason(), Some(XMV_IGNORE_REASON));
        assert_eq!(VideoKind::Ogv.ignore_reason(), None);
        assert_eq!(VideoKind::Unknown.ignore_reason(), None);
        assert!(XMV_IGNORE_REASON.contains("Xbox"));
        let p = VfsPath::new("X/Y.XMV").unwrap();
        assert_eq!(VideoKind::from_path(&p), VideoKind::Xmv);
    }

    #[test]
    fn xmv_is_skipped_with_a_typed_outcome_and_a_log_line() {
        let v = vfs();
        let (out, log) = logged(|| resolve_movie(&v, "Content\\Movies\\NTSC\\Demo_02.XMV"));
        let MovieOutcome::Skipped { path, kind, reason } = &out else {
            panic!("{out:?}");
        };
        assert_eq!(path.as_str(), "content/movies/ntsc/demo_02.xmv");
        assert_eq!(*kind, VideoKind::Xmv);
        assert_eq!(*reason, XMV_IGNORE_REASON);
        assert!(!out.is_playable());
        assert!(log.contains("WARN"), "{log}");
        assert!(log.contains("Xbox"), "{log}");
        assert!(log.contains("demo_02.xmv"), "{log}");
        assert!(log.contains("cutscene skipped"), "{log}");
    }

    #[test]
    fn xmv_is_skipped_even_when_the_file_is_absent() {
        let (out, _) = logged(|| resolve_movie(&MemoryVfs::new(), "nowhere/x.xmv"));
        assert!(matches!(out, MovieOutcome::Skipped { .. }), "{out:?}");
    }

    #[test]
    fn neighbouring_ogv_still_plays_and_does_not_log() {
        let v = vfs();
        let (out, log) = logged(|| resolve_movie(&v, "content/movies/ntsc/A.ogv"));
        assert_eq!(
            out,
            MovieOutcome::Play(VfsPath::new("content/movies/ntsc/a.ogv").unwrap())
        );
        assert!(out.is_playable());
        assert!(log.is_empty(), "{log}");
    }

    #[test]
    fn other_references_are_non_fatal_outcomes() {
        let v = vfs();
        assert_eq!(resolve_movie(&v, ""), MovieOutcome::NotSet);
        assert_eq!(resolve_movie(&v, "   "), MovieOutcome::NotSet);
        let (m, log) = logged(|| resolve_movie(&v, "content/movies/ntsc/gone.ogv"));
        assert!(matches!(m, MovieOutcome::Missing(_)), "{m:?}");
        assert!(log.contains("not found"), "{log}");
        let (m, log) = logged(|| resolve_movie(&v, "content/movies/ntsc/readme.txt"));
        assert!(matches!(m, MovieOutcome::Unrecognized(_)), "{m:?}");
        assert!(log.contains("unrecognized"), "{log}");
        let (m, log) = logged(|| resolve_movie(&v, "../escape.ogv"));
        assert_eq!(m, MovieOutcome::BadPath("../escape.ogv".into()));
        assert!(log.contains("not a valid path"), "{log}");
        // An .ogv name that is a directory is not a movie.
        let mut v2 = MemoryVfs::new();
        v2.insert("d.ogv/f", vec![1]).unwrap();
        assert!(matches!(
            resolve_movie(&v2, "d.ogv"),
            MovieOutcome::Missing(_)
        ));
    }

    #[test]
    fn level_nfo_referencing_xmv_skips_the_cutscene_and_keeps_the_rest() {
        let v = vfs();
        let nfo = Nfo::parse(
            b"LEVEL\r\nNAME=\"T\"\r\nINTRO_MOVIE=\"Content/Movies/Ntsc/demo_02.xmv\"\r\nOUTRO_MOVIE=\"Content/Movies/Ntsc/a.ogv\"\r\n",
        )
        .unwrap();
        let (res, log) = logged(|| level_movies(&v, &nfo));
        assert_eq!(res.len(), 2);
        assert_eq!(res[0].0, "INTRO_MOVIE");
        assert!(
            matches!(res[0].1, MovieOutcome::Skipped { .. }),
            "{:?}",
            res[0]
        );
        assert_eq!(res[1].0, "OUTRO_MOVIE");
        assert!(res[1].1.is_playable());
        assert_eq!(log.matches("cutscene skipped").count(), 1, "{log}");

        let ep = Nfo::parse(b"EPISODE\r\nEPISODE_MOVIE = \"x.xmv\"\r\n").unwrap();
        let (res, _) = logged(|| level_movies(&v, &ep));
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].0, "EPISODE_MOVIE");
        assert!(matches!(res[0].1, MovieOutcome::Skipped { .. }));

        let none = Nfo::parse(b"LEVEL\r\nNAME=\"T\"\r\n").unwrap();
        assert!(level_movies(&v, &none).is_empty());
    }

    #[test]
    fn mod_folder_scan_lists_xmv_as_ignored_and_never_opens_it() {
        let v = vfs();
        let found = scan_videos(&v, &VfsPath::new("content").unwrap());
        let got: Vec<(&str, VideoKind)> = found.iter().map(|(p, k)| (p.as_str(), *k)).collect();
        assert_eq!(
            got,
            [
                ("content/mod/deep/er/b.ogv", VideoKind::Ogv),
                ("content/mod/deep/er/c.xmv", VideoKind::Xmv),
                ("content/movies/ntsc/a.ogv", VideoKind::Ogv),
                ("content/movies/ntsc/demo_02.xmv", VideoKind::Xmv),
            ]
        );
        assert!(scan_videos(&v, &VfsPath::new("missing").unwrap()).is_empty());
        // A file is not a directory to scan.
        assert!(scan_videos(&v, &VfsPath::new("content/movies/ntsc/a.ogv").unwrap()).is_empty());
    }

    /// Data-gated: the real install has exactly the one `.xmv` the spec names
    /// (`docs/re/cutscenes.md`: 98 `.ogv`, 1 `.xmv`), and nothing is `Unknown`.
    #[test]
    fn real_install_video_counts_match_the_spec() {
        let dir = ss2_testkit::require_game_data!();
        let v = ss2_vfs::mount_install(&dir).unwrap();
        let found = scan_videos(&v, &VfsPath::root());
        let ogv: Vec<_> = found.iter().filter(|(_, k)| *k == VideoKind::Ogv).collect();
        let xmv: Vec<_> = found.iter().filter(|(_, k)| *k == VideoKind::Xmv).collect();
        eprintln!("real install: {} .ogv, {} .xmv", ogv.len(), xmv.len());
        assert_eq!(ogv.len(), 98);
        assert_eq!(xmv.len(), 1);
        assert!(
            xmv[0].0.as_str().ends_with("movies/ntsc/demo_02.xmv"),
            "{}",
            xmv[0].0
        );
        assert_eq!(found.len(), ogv.len() + xmv.len());
        // Resolving the real .xmv skips it; resolving a real .ogv plays it.
        let (out, _) = logged(|| resolve_movie(&v, xmv[0].0.as_str()));
        assert!(matches!(out, MovieOutcome::Skipped { .. }), "{out:?}");
        assert!(resolve_movie(&v, ogv[0].0.as_str()).is_playable());
    }
}
