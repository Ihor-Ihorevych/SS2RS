//! Helpers for tests that need the player's own Serious Sam 2 install ("data-gated" tests).
//!
//! Plain CI never has game data, so such tests call [`require_game_data!`], which skips the test
//! with a printed reason when `SS2_DATA_DIR` is missing or wrong.

use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;

/// Environment variable pointing at the folder that contains the `.gro` archives.
pub const DATA_DIR_ENV: &str = "SS2_DATA_DIR";

/// Why a data-gated test is being skipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    /// `SS2_DATA_DIR` is unset or empty.
    NotSet,
    /// `SS2_DATA_DIR` does not point at a directory.
    NotADirectory(PathBuf),
    /// The directory contains no `.gro` archive directly inside it.
    NoArchives(PathBuf),
}

impl fmt::Display for SkipReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotSet => write!(f, "{DATA_DIR_ENV} is not set"),
            Self::NotADirectory(p) => {
                write!(f, "{DATA_DIR_ENV}={} is not a directory", p.display())
            }
            Self::NoArchives(p) => write!(
                f,
                "no .gro archives in {}: point {DATA_DIR_ENV} at the folder that contains \
                 All_PC_01.gro",
                p.display()
            ),
        }
    }
}

impl std::error::Error for SkipReason {}

/// Validate a candidate data directory. Pure: reads no environment variables.
pub fn resolve_game_data_dir(value: Option<OsString>) -> Result<PathBuf, SkipReason> {
    let value = value.filter(|v| !v.is_empty()).ok_or(SkipReason::NotSet)?;
    let dir = PathBuf::from(value);
    if !dir.is_dir() {
        return Err(SkipReason::NotADirectory(dir));
    }
    let has_gro = std::fs::read_dir(&dir)
        .map(|entries| {
            entries.flatten().any(|e| {
                e.path()
                    .extension()
                    .is_some_and(|x| x.eq_ignore_ascii_case("gro"))
            })
        })
        .unwrap_or(false);
    if has_gro {
        Ok(dir)
    } else {
        Err(SkipReason::NoArchives(dir))
    }
}

/// The game data directory from `SS2_DATA_DIR`, or the reason there is none.
pub fn game_data_dir() -> Result<PathBuf, SkipReason> {
    resolve_game_data_dir(std::env::var_os(DATA_DIR_ENV))
}

/// Evaluates to the game data `PathBuf`; otherwise prints `SKIPPED (no game data): <reason>` to
/// stderr and returns from the calling test function.
#[macro_export]
macro_rules! require_game_data {
    () => {
        match $crate::game_data_dir() {
            Ok(dir) => dir,
            Err(reason) => {
                eprintln!("SKIPPED (no game data): {reason}");
                return;
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::fs;

    #[test]
    fn unset_is_not_set() {
        assert!(matches!(
            resolve_game_data_dir(None),
            Err(SkipReason::NotSet)
        ));
        assert!(matches!(
            resolve_game_data_dir(Some(OsString::new())),
            Err(SkipReason::NotSet)
        ));
    }

    #[test]
    fn missing_dir_is_not_a_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("nope");
        match resolve_game_data_dir(Some(missing.clone().into_os_string())) {
            Err(SkipReason::NotADirectory(p)) => assert_eq!(p, missing),
            other => panic!("expected NotADirectory, got {other:?}"),
        }
    }

    #[test]
    fn empty_dir_has_no_archives() {
        let tmp = tempfile::tempdir().unwrap();
        let r = resolve_game_data_dir(Some(tmp.path().as_os_str().to_owned()));
        assert!(matches!(r, Err(SkipReason::NoArchives(_))), "{r:?}");
    }

    #[test]
    fn parent_of_install_is_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let install = tmp.path().join("Serious Sam 2");
        fs::create_dir(&install).unwrap();
        fs::write(install.join("All_PC_01.gro"), b"PK").unwrap();
        let r = resolve_game_data_dir(Some(tmp.path().as_os_str().to_owned()));
        assert!(matches!(r, Err(SkipReason::NoArchives(_))), "{r:?}");
    }

    #[test]
    fn dir_with_gro_is_accepted() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("All_PC_01.GRO"), b"PK").unwrap();
        let r = resolve_game_data_dir(Some(tmp.path().as_os_str().to_owned()));
        assert_eq!(r.unwrap(), tmp.path());
    }

    #[test]
    fn path_with_spaces_is_accepted() {
        let tmp = tempfile::Builder::new()
            .prefix("Serious Sam 2 ")
            .tempdir()
            .unwrap();
        fs::write(tmp.path().join("a.gro"), b"PK").unwrap();
        let r = resolve_game_data_dir(Some(tmp.path().as_os_str().to_owned()));
        assert_eq!(r.unwrap(), tmp.path());
    }

    #[test]
    fn no_archives_message_is_actionable() {
        let msg = SkipReason::NoArchives("C:/x".into()).to_string();
        assert!(
            msg.contains("SS2_DATA_DIR") && msg.contains("All_PC_01.gro"),
            "{msg}"
        );
    }

    #[test]
    fn macro_skips_or_runs() {
        let dir = crate::require_game_data!();
        assert!(dir.is_dir());
    }
}
