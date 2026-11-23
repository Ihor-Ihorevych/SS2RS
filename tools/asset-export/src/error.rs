//! The typed errors of the exporter and their exit-code classes.

use std::fmt;

/// Why one export failed. Nothing in this crate panics on damaged input: every problem is one of
/// these, and a batch records it in the manifest and goes on.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    /// The input could not be found or read.
    #[error("cannot read {path}: {detail}")]
    Read {
        /// The path or VFS path that was read.
        path: String,
        /// What went wrong.
        detail: String,
    },
    /// A reader of the engine refused the file (damaged or not what its name says).
    #[error("{0}")]
    Format(String),
    /// A kind of file this tool (or the readers it uses) does not handle yet, for example the
    /// development-build textures of format versions 4 to 7.
    #[error("{0}")]
    Unsupported(String),
    /// An option does not fit the file (a mip level or face the texture does not have, a LOD that
    /// does not exist).
    #[error("{0}")]
    Option(String),
    /// The output could not be produced (a value that glTF or PNG cannot hold, an image too large).
    #[error("{0}")]
    Encode(String),
    /// The output could not be written.
    #[error("cannot write {path}: {detail}")]
    Write {
        /// The output path.
        path: String,
        /// What went wrong.
        detail: String,
    },
}

impl ExportError {
    /// A short stable class name for manifests: `read`, `format`, `unsupported`, `option`,
    /// `encode` or `write`.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Read { .. } => "read",
            Self::Format(_) => "format",
            Self::Unsupported(_) => "unsupported",
            Self::Option(_) => "option",
            Self::Encode(_) => "encode",
            Self::Write { .. } => "write",
        }
    }

    /// Wrap an error of a format reader: the "format version N" refusals (old development-build
    /// files) become [`ExportError::Unsupported`], everything else [`ExportError::Format`].
    pub fn from_reader(e: &ss2_core::Error) -> Self {
        let text = e.to_string();
        if text.contains("format version") {
            Self::Unsupported(text)
        } else {
            Self::Format(text)
        }
    }

    /// A read failure of `path`.
    pub fn read(path: impl fmt::Display, detail: impl fmt::Display) -> Self {
        Self::Read {
            path: path.to_string(),
            detail: detail.to_string(),
        }
    }

    /// A write failure of `path`.
    pub fn write(path: impl fmt::Display, detail: impl fmt::Display) -> Self {
        Self::Write {
            path: path.to_string(),
            detail: detail.to_string(),
        }
    }
}

/// `Result` of this crate.
pub type Result<T> = std::result::Result<T, ExportError>;
