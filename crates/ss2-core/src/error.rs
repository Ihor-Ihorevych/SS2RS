//! Shared error type. Library code returns [`Result`] and never panics on bad input.

use std::path::PathBuf;

/// Errors shared across SS2RS crates.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An I/O operation on `path` failed.
    #[error("I/O error on {path}: {source}")]
    Io {
        /// The path being accessed.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// Input (usually game data) was malformed.
    #[error("invalid {what}: {detail}")]
    Invalid {
        /// What was being parsed or validated, e.g. `"CTSEMETA header"`.
        what: &'static str,
        /// Human-readable explanation.
        detail: String,
    },
    /// A requested item does not exist.
    #[error("not found: {0}")]
    NotFound(String),
}

/// `Result` alias using [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Build an [`Error::Io`] for `path`.
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    /// Build an [`Error::Invalid`].
    pub fn invalid(what: &'static str, detail: impl Into<String>) -> Self {
        Self::Invalid {
            what,
            detail: detail.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_displays_what_and_detail() {
        let e = Error::invalid("CTSEMETA header", "bad magic");
        assert_eq!(e.to_string(), "invalid CTSEMETA header: bad magic");
    }

    #[test]
    fn io_error_names_the_path() {
        let source = std::io::Error::new(std::io::ErrorKind::NotFound, "no such file");
        let e = Error::io("Content/a.gro", source);
        let msg = e.to_string();
        assert!(msg.contains("Content/a.gro"), "{msg}");
        assert!(msg.contains("no such file"), "{msg}");
    }
}
