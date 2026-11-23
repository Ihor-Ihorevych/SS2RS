//! The annotated text format (the default).
//!
//! One block per chunk of the file, every entry prefixed with its byte offset (`0x…`) and size
//! (`+n`); `--raw` adds the first bytes of every region on a `|` line; objects print their decoded
//! value. The coverage block closes the report, and `--unknown` prints only that block.

use std::fmt::Write as _;

use crate::Options;
use crate::analyze::{Coverage, Damaged, Outcome, Report, Span};
use crate::node::{Node, hex_spaced};

/// Width above which a value is broken over several lines.
const WIDTH: usize = 100;

fn range(span: Span) -> String {
    format!("0x{:08x}..0x{:08x}", span.start, span.end)
}

fn entry_at(span: Span) -> String {
    format!("0x{:08x} +{}", span.start, span.len())
}

fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                let _ = write!(out, "\\x{:02x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn named(id: u32, name: Option<&str>) -> String {
    match name {
        Some(n) => format!("{id} {n}"),
        None => format!("{id} (not defined)"),
    }
}

/// Append `n` on one line to `out`; `false` as soon as `out` is longer than `limit` (the caller
/// then breaks the value over lines). The cost of one call is bounded by `limit`, so a deeply
/// nested value of millions of members costs time proportional to its size, not to its depth.
fn inline_into(n: &Node, out: &mut String, limit: usize) -> bool {
    match n {
        Node::Null => out.push_str("null"),
        Node::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Node::Int(v) => {
            let _ = write!(out, "{v}");
        }
        Node::Num(s) => out.push_str(s),
        Node::Str(s) => out.push_str(&quote(s)),
        Node::List(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                if !inline_into(item, out, limit) {
                    return false;
                }
            }
            out.push(']');
        }
        Node::Map(pairs) => {
            out.push('{');
            for (i, (k, v)) in pairs.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                let _ = write!(out, "{k}: ");
                if !inline_into(v, out, limit) {
                    return false;
                }
            }
            out.push('}');
        }
        Node::Struct { name, members } => {
            if members.is_empty() {
                let _ = write!(out, "{name} {{}}");
            } else {
                let _ = write!(out, "{name} {{ ");
                for (i, (k, v)) in members.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    let _ = write!(out, "{k} = ");
                    if !inline_into(v, out, limit) {
                        return false;
                    }
                }
                out.push_str(" }");
            }
        }
        Node::Bytes { len, preview } => {
            let more = if preview.len() < *len { " ..." } else { "" };
            if preview.is_empty() {
                let _ = write!(out, "bytes[{len}]");
            } else {
                let _ = write!(out, "bytes[{len}] {}{more}", hex_spaced(preview));
            }
        }
    }
    out.len() <= limit
}

/// A value on one line (unbounded; for tests and short values).
#[cfg(test)]
fn inline(n: &Node) -> String {
    let mut s = String::new();
    inline_into(n, &mut s, usize::MAX);
    s
}

/// A value, broken over lines when it does not fit in [`WIDTH`] columns at `indent`.
fn block(n: &Node, indent: usize, out: &mut String) {
    let mut one = String::new();
    if inline_into(n, &mut one, WIDTH.saturating_sub(indent)) {
        out.push_str(&one);
        return;
    }
    let pad = " ".repeat(indent + 2);
    let close = " ".repeat(indent);
    match n {
        Node::List(items) => {
            out.push_str("[\n");
            for item in items {
                out.push_str(&pad);
                block(item, indent + 2, out);
                out.push('\n');
            }
            let _ = write!(out, "{close}]");
        }
        Node::Map(pairs) => {
            out.push_str("{\n");
            for (k, v) in pairs {
                let _ = write!(out, "{pad}{k}: ");
                block(v, indent + 2, out);
                out.push('\n');
            }
            let _ = write!(out, "{close}}}");
        }
        Node::Struct { name, members } => {
            let _ = writeln!(out, "{name} {{");
            for (k, v) in members {
                let _ = write!(out, "{pad}{k} = ");
                block(v, indent + 2, out);
                out.push('\n');
            }
            let _ = write!(out, "{close}}}");
        }
        // A scalar that is itself too long (a long string) stays on its line.
        _ => {
            let mut whole = String::new();
            inline_into(n, &mut whole, usize::MAX);
            out.push_str(&whole);
        }
    }
}

struct Printer<'a> {
    out: String,
    data: &'a [u8],
    opts: &'a Options,
}

impl Printer<'_> {
    fn line(&mut self, s: impl AsRef<str>) {
        self.out.push_str(s.as_ref());
        self.out.push('\n');
    }

    /// The first bytes of `span` on a `|` line, when `--raw` is on.
    fn raw(&mut self, span: Span, indent: usize) {
        if !self.opts.raw {
            return;
        }
        let bytes = span.bytes(self.data);
        let shown = &bytes[..bytes.len().min(self.opts.preview)];
        let more = if shown.len() < bytes.len() {
            " ..."
        } else {
            ""
        };
        let pad = " ".repeat(indent);
        self.line(format!("{pad}| {}{more}", hex_spaced(shown)));
    }

    fn coverage(&mut self, cov: &Coverage) {
        self.line("== coverage");
        self.line(format!("file size   {} bytes", cov.total));
        self.line(format!(
            "explained   {} bytes ({})",
            cov.decoded,
            percent(cov.decoded, cov.total)
        ));
        self.line(format!(
            "  of which opaque (a word of unknown meaning or raw byte arrays): {} bytes",
            cov.opaque
        ));
        self.line(format!(
            "undecoded   {} bytes in {} range(s)",
            cov.undecoded_bytes(),
            cov.undecoded.len()
        ));
        for g in &cov.undecoded {
            self.line(format!(
                "  {}  {} bytes  in {}",
                range(g.span),
                g.span.len(),
                g.section
            ));
            self.line(format!("    reason: {}", g.reason));
            let bytes = g.span.bytes(self.data);
            let shown = &bytes[..bytes.len().min(self.opts.preview)];
            let more = if shown.len() < bytes.len() {
                " ..."
            } else {
                ""
            };
            self.line(format!("    | {}{more}", hex_spaced(shown)));
        }
    }

    fn head(&mut self, label: &str, size: usize, status: &str) {
        self.line("# meta-dump (annotated text; offsets are file offsets, +n is a size in bytes)");
        self.line(format!("file: {label}"));
        self.line(format!("size: {size} bytes"));
        self.line(format!("status: {status}"));
    }

    fn report(&mut self, r: &Report<'_>) {
        self.head(
            &r.label,
            r.data.len(),
            if r.problems.is_empty() {
                "ok"
            } else {
                "problems"
            },
        );
        if self.opts.unknown_only {
            self.line("");
            self.coverage(&r.coverage);
            self.problems(r);
            return;
        }
        self.line("");
        self.line(format!(
            "== header {} ({} bytes)",
            range(r.header.span),
            r.header.span.len()
        ));
        self.line("magic: CTSEMETA   byte-order marker: 0x1234ABCD");
        self.line(format!("format version: {}", r.header.format_version));
        self.line(format!(
            "version string: {}",
            quote(&r.header.version_string)
        ));
        match &r.header.build {
            Some(b) => self.line(format!(
                "build: product {}, release {}, build {}, tag {}",
                quote(&b.product),
                b.release,
                b.build,
                b.tag
            )),
            None => self.line("build: (the version string does not have the usual shape)"),
        }
        self.raw(r.header.span, 0);

        self.line("");
        self.line("== chunks");
        for ch in &r.chunks {
            let n = ch
                .entries
                .map_or_else(String::new, |n| format!("  {n} entries"));
            self.line(format!(
                "{:<8} {}  {} bytes{n}",
                ch.tag,
                range(ch.span),
                ch.span.len()
            ));
            self.raw(ch.span, 2);
        }

        self.line("");
        self.line(format!("== INFO {}", range(r.info.span)));
        let flag = r.info.flag.map_or_else(
            || "(none in this format version)".to_string(),
            |f| format!("{f} (meaning unknown)"),
        );
        self.line(format!("flag: {flag}"));
        self.line(format!(
            "declared: {} file references, {} identifiers, {} types, {} objects",
            r.info.file_count, r.info.ident_count, r.info.type_count, r.info.object_count
        ));
        for m in &r.info.mismatches {
            self.line(format!("mismatch: {m}"));
        }
        self.raw(r.info.span, 2);

        self.line("");
        self.line(format!("== RFIL file references ({})", r.files.len()));
        for f in &r.files {
            self.line(format!(
                "{}  #{} flags 0x{:08x}  {}",
                entry_at(f.span),
                f.index,
                f.flags,
                quote(&f.path)
            ));
            self.raw(f.span, 4);
        }

        self.line("");
        self.line(format!("== IDNT identifiers ({})", r.idents.len()));
        for i in &r.idents {
            self.line(format!(
                "{}  #{} {}",
                entry_at(i.span),
                i.id,
                quote(&i.text)
            ));
            self.raw(i.span, 4);
        }

        self.line("");
        self.line(format!(
            "== EXTY external types ({})",
            r.external_types.len()
        ));
        for t in &r.external_types {
            self.line(format!("{}  type {} {}", entry_at(t.span), t.id, t.name));
            self.raw(t.span, 4);
        }

        self.line("");
        self.line(format!(
            "== INTY type table ({} types, {} ADIM blocks)",
            r.types.len(),
            r.array_dims.len()
        ));
        let mut dims = r.array_dims.iter().peekable();
        let mut types = r.types.iter().peekable();
        loop {
            // Print in file order: whichever starts first.
            let next_dim = dims.peek().map(|d| d.span.start);
            let next_type = types.peek().map(|t| t.span.start);
            match (next_dim, next_type) {
                (Some(d), t) if t.is_none_or(|t| d < t) => {
                    let Some(d) = dims.next() else { break };
                    self.line(format!(
                        "{}  ADIM after {} types, dimensions {:?}",
                        entry_at(d.span),
                        d.after_type,
                        d.dims
                    ));
                    self.raw(d.span, 4);
                }
                (_, Some(_)) => {
                    let Some(t) = types.next() else { break };
                    self.type_line(t);
                }
                _ => break,
            }
        }

        self.line("");
        self.line(format!(
            "== EXOB external objects ({})",
            r.external_objects.len()
        ));
        for e in &r.external_objects {
            self.line(format!(
                "{}  object {}  type {}  flags {}  file #{} {}",
                entry_at(e.span),
                e.object_id,
                named(e.type_id, e.type_name.as_deref()),
                e.flags,
                e.file_ref,
                quote(&e.path)
            ));
            self.raw(e.span, 4);
        }

        for (tag, table) in [("OBTY", &r.object_table), ("EDTY", &r.extra_table)] {
            self.line("");
            self.line(format!("== {tag} ({})", table.len()));
            for p in table {
                self.line(format!(
                    "{}  object {}  type {}",
                    entry_at(p.span),
                    p.object_id,
                    named(p.type_id, p.type_name.as_deref())
                ));
                self.raw(p.span, 4);
            }
        }

        self.line("");
        self.line(format!(
            "== OBJS objects ({} of {} decoded)",
            r.objects.len(),
            r.object_table.len()
        ));
        for o in &r.objects {
            self.object(o);
        }
        if !r.extra.is_empty() || !r.extra_table.is_empty() {
            self.line("");
            self.line(format!(
                "== EDOB extra records ({} of {} decoded)",
                r.extra.len(),
                r.extra_table.len()
            ));
            for o in &r.extra {
                self.object(o);
            }
        }

        self.problems(r);
        self.line("");
        self.coverage(&r.coverage);
    }

    fn problems(&mut self, r: &Report<'_>) {
        if r.problems.is_empty() {
            return;
        }
        self.line("");
        self.line("== problems");
        for p in &r.problems {
            self.line(format!("- {p}"));
        }
    }

    fn type_line(&mut self, t: &crate::analyze::TypeEntry) {
        let mut what = t.form.to_string();
        if let Some(s) = t.size {
            let _ = write!(what, " size {s}");
        }
        if let Some(a) = t.align {
            let _ = write!(what, " align {a}");
        }
        if let Some(target) = t.target {
            let _ = write!(what, " of {}", named(target, t.target_name.as_deref()));
        }
        self.line(format!(
            "{}  type {} {}  kind {}  {what}",
            entry_at(t.span),
            t.id,
            t.name,
            t.kind
        ));
        if let Some(f) = t.flags {
            let parent = t.parent.map_or_else(
                || "none".to_string(),
                |p| named(p, t.parent_name.as_deref()),
            );
            self.line(format!(
                "    flags {f}, base class {parent}, {} members",
                t.members.len()
            ));
            for (i, m) in t.members.iter().enumerate() {
                self.line(format!(
                    "      [{i}] {} : {}",
                    quote(&m.name),
                    named(m.type_id, m.type_name.as_deref())
                ));
            }
        }
        self.raw(t.span, 4);
    }

    fn object(&mut self, o: &crate::analyze::ObjectRec) {
        self.line(format!(
            "{}  object {}  type {}",
            entry_at(o.span),
            o.id,
            named(o.type_id, o.type_name.as_deref())
        ));
        if let Some(v) = &o.value {
            let mut s = String::from("    ");
            block(v, 4, &mut s);
            self.line(s);
        }
        self.raw(o.span, 4);
    }

    fn damaged(&mut self, d: &Damaged<'_>) {
        self.head(&d.label, d.data.len(), "damaged");
        self.line(format!("error: {}", d.error));
        if !self.opts.unknown_only
            && let Some(h) = &d.header
        {
            self.line("");
            self.line(format!("== header {} (readable)", range(h.span)));
            self.line(format!("format version: {}", h.format_version));
            self.line(format!("version string: {}", quote(&h.version_string)));
        }
        self.line("");
        self.coverage(&d.coverage);
    }
}

/// The share `part / whole` as `99.1234%`, in integer arithmetic.
pub(crate) fn percent(part: usize, whole: usize) -> String {
    if whole == 0 {
        return "n/a".into();
    }
    // Integer arithmetic: 4 decimals, no float rounding surprises.
    let scaled = (part as u128) * 1_000_000 / (whole as u128);
    format!("{}.{:04}%", scaled / 10_000, scaled % 10_000)
}

/// Render the annotated text.
#[must_use]
pub fn render_text(outcome: &Outcome<'_>, opts: &Options) -> String {
    let data = match outcome {
        Outcome::Parsed(r) => r.data,
        Outcome::Damaged(d) => d.data,
    };
    let mut p = Printer {
        out: String::new(),
        data,
        opts,
    };
    match outcome {
        Outcome::Parsed(r) => p.report(r),
        Outcome::Damaged(d) => p.damaged(d),
    }
    p.out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quote_escapes_control_characters_and_only_those() {
        assert_eq!(quote("a\u{1}b\u{7f}c"), "\"a\\x01b\\x7fc\"");
        assert_eq!(quote("tab\t\r\n\"\\"), "\"tab\\t\\r\\n\\\"\\\\\"");
        // A printable character that is neither below 0x20 nor 0x7f stays as it is (0x20, 0x7e).
        assert_eq!(quote(" ~\u{e9}"), "\" ~\u{e9}\"");
        // 0x7f alone and a control below 0x20 alone are each enough.
        assert_eq!(quote("\u{7f}"), "\"\\x7f\"");
        assert_eq!(quote("\u{1f}"), "\"\\x1f\"");
    }

    #[test]
    fn booleans_print_as_words() {
        assert_eq!(inline(&Node::Bool(true)), "true");
        assert_eq!(inline(&Node::Bool(false)), "false");
    }

    #[test]
    fn a_long_map_is_broken_over_lines_like_a_list() {
        let long = Node::map([("ref", Node::int(3)), ("file", Node::str("x".repeat(120)))]);
        let mut s = String::new();
        block(&long, 0, &mut s);
        assert!(s.starts_with("{\n  ref: 3\n  file: \""), "{s}");
        assert!(s.ends_with("\n}"), "{s}");
        // A struct member that is a long map breaks the struct too.
        let st = Node::Struct {
            name: "S".into(),
            members: vec![("S.1".into(), long)],
        };
        let mut s = String::new();
        block(&st, 0, &mut s);
        assert!(s.starts_with("S {\n  S.1 = {\n"), "{s}");
    }

    #[test]
    fn the_line_width_limit_is_inclusive() {
        // `[` + 98 digits + `]` is exactly 100 columns at indent 0: it stays on one line.
        let exact = Node::List(vec![Node::Num("1".repeat(98))]);
        let mut s = String::new();
        block(&exact, 0, &mut s);
        assert_eq!(s.len(), 100);
        assert!(!s.contains('\n'));
        // One column more and it is broken.
        let over = Node::List(vec![Node::Num("1".repeat(99))]);
        let mut s = String::new();
        block(&over, 0, &mut s);
        assert!(s.contains('\n'));
        // The indent counts against the width.
        let mut s = String::new();
        block(&exact, 1, &mut s);
        assert!(s.contains('\n'));
    }

    #[test]
    fn percent_is_exact_integer_arithmetic() {
        assert_eq!(percent(1, 3), "33.3333%");
        assert_eq!(percent(2, 3), "66.6666%");
        assert_eq!(percent(5, 5), "100.0000%");
        assert_eq!(percent(0, 5), "0.0000%");
        assert_eq!(percent(0, 0), "n/a");
    }

    #[test]
    fn values_break_over_lines_only_when_long() {
        let short = Node::Struct {
            name: "T".into(),
            members: vec![("A.1".into(), Node::int(1)), ("A.2".into(), Node::Null)],
        };
        let mut s = String::new();
        block(&short, 0, &mut s);
        assert_eq!(s, "T { A.1 = 1, A.2 = null }");

        let long = Node::List((0..40).map(|i| Node::int(i * 1000)).collect());
        let mut s = String::new();
        block(&long, 4, &mut s);
        assert!(s.starts_with("[\n"));
        assert_eq!(s.lines().count(), 42);
        assert!(s.ends_with("    ]"));

        let nested = Node::Struct {
            name: "Outer".into(),
            members: vec![("o.1".into(), long)],
        };
        let mut s = String::new();
        block(&nested, 0, &mut s);
        assert!(s.starts_with("Outer {\n  o.1 = [\n    0\n"), "{s}");
    }

    #[test]
    fn inline_shapes() {
        assert_eq!(inline(&Node::str("a\"b\n")), "\"a\\\"b\\n\"");
        assert_eq!(
            inline(&Node::Bytes {
                len: 5,
                preview: vec![1, 2]
            }),
            "bytes[5] 01 02 ..."
        );
        assert_eq!(
            inline(&Node::Bytes {
                len: 0,
                preview: vec![]
            }),
            "bytes[0]"
        );
        assert_eq!(
            inline(&Node::map([("k", Node::int(1)), ("l", Node::Null)])),
            "{k: 1, l: null}"
        );
        assert_eq!(
            inline(&Node::Struct {
                name: "E".into(),
                members: vec![]
            }),
            "E {}"
        );
    }
}
