//! `meta-dump`: print any CTSEMETA file as annotated text, JSON or RON.
//!
//! The tool is a thin report layer over [`ss2_meta`]: the container frame comes from
//! [`ss2_meta::Container::parse`], the values from [`ss2_meta::TypeSystem::decode_value`]. It adds
//! the byte offset and size of every region it prints, optional raw bytes, and a **coverage map**
//! that says which bytes of the file some decoder explained (`--unknown` prints only that, and
//! lists the rest with a hex preview).
//!
//! ```
//! use meta_dump::{Format, Options, dump};
//! use ss2_testkit::fixtures::ctsemeta::MetaBuilder;
//!
//! let bytes = MetaBuilder::sample().build().unwrap();
//! let out = dump(&bytes, "sample.meta", Format::Text, &Options::default());
//! assert!(out.parsed);
//! assert_eq!(out.undecoded_bytes, 0);
//! assert!(out.text.contains("== header 0x00000000..0x"));
//! ```
//!
//! Exit-code contract of the binary: `0` success, `1` (only with `--unknown`) some bytes are
//! undecoded, `2` the file could not be read or its frame is damaged.

pub mod analyze;
pub mod node;
mod render;
mod text;

pub use analyze::{Coverage, Damaged, Gap, Outcome, Report, Span, analyze};
pub use node::Node;
pub use render::{SCHEMA, to_node};
pub use text::render_text;

/// The output format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Annotated text (default).
    Text,
    /// JSON.
    Json,
    /// RON (anonymous structs, see `docs/META_DUMP.md`).
    Ron,
}

/// What to print.
#[derive(Clone, Debug)]
pub struct Options {
    /// Bytes shown by a hex preview (raw spans, undecoded ranges, byte-array values).
    pub preview: usize,
    /// Add the first `preview` bytes of every region it prints (`--raw`).
    pub raw: bool,
    /// Leave the decoded values out; keep every offset, size and type (`--skip-values`).
    pub skip_values: bool,
    /// Print only the coverage and the undecoded ranges (`--unknown`).
    pub unknown_only: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            preview: 32,
            raw: false,
            skip_values: false,
            unknown_only: false,
        }
    }
}

/// The result of [`dump`].
#[derive(Clone, Debug)]
pub struct Dumped {
    /// The document to print.
    pub text: String,
    /// `true` if the container frame parsed (`false` for a damaged file).
    pub parsed: bool,
    /// Bytes no decoder explained.
    pub undecoded_bytes: usize,
    /// Total bytes of the file.
    pub total_bytes: usize,
}

/// Analyze `data` and print it. Never panics, whatever `data` is; a damaged file gives a report
/// with `parsed == false`.
#[must_use]
pub fn dump(data: &[u8], label: &str, format: Format, opts: &Options) -> Dumped {
    let outcome = analyze(data, label, opts);
    let text = match format {
        Format::Text => render_text(&outcome, opts),
        Format::Json => to_node(&outcome, opts).to_json(),
        Format::Ron => to_node(&outcome, opts).to_ron(),
    };
    Dumped {
        text,
        parsed: outcome.is_parsed(),
        undecoded_bytes: outcome.coverage().undecoded_bytes(),
        total_bytes: data.len(),
    }
}
