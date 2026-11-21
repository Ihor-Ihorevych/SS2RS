//! [`VfsPath`]: the one path type the whole engine uses to name game files.

use std::fmt;
use std::str::FromStr;

use ss2_core::{Error, Result};

/// A normalized, case-insensitive, `/`-separated path inside the virtual file system.
///
/// The original game treats `Models\Player.mdl`, `models/player.MDL` and `./models//player.mdl`
/// as the same file. `VfsPath` bakes that in: both separators are accepted, `.` and empty
/// segments vanish, everything is lower-cased, and the result never starts or ends with `/`.
/// Because the form is canonical, `==`, `Ord` and `Hash` are exactly "same file".
///
/// Rejected (so a path can never escape the VFS or leak a host path): `..` segments, `:` (drive
/// letters, URLs) and control characters.
///
/// ```
/// use ss2_vfs::VfsPath;
///
/// let a: VfsPath = r"Models\Player.MDL".parse().unwrap();
/// let b = VfsPath::new("./models//player.mdl").unwrap();
/// assert_eq!(a, b);
/// assert_eq!(a.as_str(), "models/player.mdl");
/// assert_eq!(a.extension(), Some("mdl"));
/// assert!(VfsPath::new("../etc/passwd").is_err());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VfsPath(String);

impl VfsPath {
    /// The root directory (empty path).
    pub fn root() -> Self {
        Self::default()
    }

    /// Parse and normalize `raw`.
    ///
    /// # Errors
    /// [`Error::Invalid`] for `..` segments, `:` or control characters.
    pub fn new(raw: &str) -> Result<Self> {
        if let Some(c) = raw.chars().find(|c| c.is_control() || *c == ':') {
            return Err(Error::invalid(
                "vfs path",
                format!("{raw:?} contains forbidden character {c:?}"),
            ));
        }
        let mut parts: Vec<String> = Vec::new();
        for seg in raw.split(['/', '\\']) {
            match seg {
                "" | "." => {}
                ".." => {
                    return Err(Error::invalid(
                        "vfs path",
                        format!("{raw:?} contains a `..` segment"),
                    ));
                }
                s => parts.push(s.to_lowercase()),
            }
        }
        Ok(Self(parts.join("/")))
    }

    /// The normalized text, e.g. `"models/player.mdl"`. Empty for the root.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// `true` for the root directory.
    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }

    /// The last segment, or `None` for the root.
    pub fn file_name(&self) -> Option<&str> {
        if self.is_root() {
            None
        } else {
            Some(self.0.rsplit('/').next().unwrap_or(&self.0))
        }
    }

    /// The extension of the last segment without the dot (`"mdl"`), lower-case.
    /// `None` if there is none; a leading dot (`.hidden`) is not an extension.
    pub fn extension(&self) -> Option<&str> {
        let (stem, ext) = self.file_name()?.rsplit_once('.')?;
        (!stem.is_empty() && !ext.is_empty()).then_some(ext)
    }

    /// The containing directory, or `None` for the root.
    pub fn parent(&self) -> Option<Self> {
        if self.is_root() {
            return None;
        }
        Some(Self(
            self.0
                .rsplit_once('/')
                .map(|(dir, _)| dir.to_owned())
                .unwrap_or_default(),
        ))
    }

    /// Append a relative path (normalized like [`VfsPath::new`]).
    ///
    /// # Errors
    /// Same as [`VfsPath::new`].
    pub fn join(&self, relative: &str) -> Result<Self> {
        let rel = Self::new(relative)?;
        Ok(match (self.is_root(), rel.is_root()) {
            (_, true) => self.clone(),
            (true, false) => rel,
            (false, false) => Self(format!("{}/{}", self.0, rel.0)),
        })
    }
}

impl fmt::Display for VfsPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for VfsPath {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Self::new(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_separators_case_dots_and_empties() {
        for raw in [
            "Models/Player.mdl",
            r"models\player.MDL",
            "./models//player.mdl",
            "/models/player.mdl/",
            r"\Models\.\Player.Mdl",
        ] {
            assert_eq!(
                VfsPath::new(raw).unwrap().as_str(),
                "models/player.mdl",
                "{raw}"
            );
        }
    }

    #[test]
    fn rejects_escapes_and_host_paths() {
        for raw in [
            "..",
            "a/../b",
            r"a\..\b",
            "C:/Games/x",
            "a:b",
            "a\0b",
            "a\nb",
        ] {
            assert!(VfsPath::new(raw).is_err(), "{raw:?} should be rejected");
        }
    }

    #[test]
    fn root_behaviour() {
        for raw in ["", "/", ".", "./", r"\"] {
            let p = VfsPath::new(raw).unwrap();
            assert!(p.is_root(), "{raw:?}");
            assert_eq!(p, VfsPath::root());
            assert_eq!(p.file_name(), None);
            assert_eq!(p.parent(), None);
        }
    }

    #[test]
    fn file_name_extension_parent() {
        let p = VfsPath::new("a/b/c.tar.GZ").unwrap();
        assert_eq!(p.file_name(), Some("c.tar.gz"));
        assert_eq!(p.extension(), Some("gz"));
        assert_eq!(p.parent().unwrap().as_str(), "a/b");
        assert_eq!(VfsPath::new("a").unwrap().parent(), Some(VfsPath::root()));
        assert_eq!(VfsPath::new("a/.hidden").unwrap().extension(), None);
        assert_eq!(VfsPath::new("a/noext").unwrap().extension(), None);
        assert_eq!(VfsPath::new("a/trailing.").unwrap().extension(), None);
    }

    #[test]
    fn join_normalizes_and_checks() {
        let base = VfsPath::new("Textures").unwrap();
        assert_eq!(
            base.join(r"Wall\A.tex").unwrap().as_str(),
            "textures/wall/a.tex"
        );
        assert_eq!(VfsPath::root().join("x").unwrap().as_str(), "x");
        assert_eq!(base.join("").unwrap(), base);
        assert!(base.join("../x").is_err());
    }

    #[test]
    fn non_ascii_is_lowercased_consistently() {
        assert_eq!(
            VfsPath::new("ÜBER/Straße").unwrap(),
            VfsPath::new("über/straße").unwrap()
        );
    }
}
