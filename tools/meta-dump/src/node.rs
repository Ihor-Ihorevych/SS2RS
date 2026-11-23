//! A tiny document model and the JSON and RON printers for it.
//!
//! The report of `meta-dump` is first turned into a [`Node`] tree; [`Node::to_json`] and
//! [`Node::to_ron`] print that tree. Writing the two printers by hand (instead of pulling in a
//! serialization stack) keeps the output byte-for-byte deterministic: map keys stay in the order
//! the report lists them, floats print with Rust's shortest round-trip form, and nothing depends on
//! a hash order or the clock. RON has no crate in the dependency tree of the repository, and the
//! subset used here (anonymous structs, lists, strings, numbers, `None`) is a few dozen lines.

use std::fmt::Write as _;

/// A value of the report.
#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    /// No value (JSON `null`, RON `None`).
    Null,
    /// A truth value.
    Bool(bool),
    /// An integer.
    Int(i128),
    /// A number already formatted (a finite float). JSON and RON both accept Rust's `{:?}` form.
    Num(String),
    /// Text.
    Str(String),
    /// A list.
    List(Vec<Node>),
    /// An ordered record. Keys are identifiers, so RON can print them as struct fields.
    Map(Vec<(String, Node)>),
    /// A class instance: its type name and `(member, value)` pairs, base class first.
    Struct {
        /// The type name.
        name: String,
        /// The members, as `(label, value)`.
        members: Vec<(String, Node)>,
    },
    /// Raw bytes: the total length and a preview (the first bytes).
    Bytes {
        /// Length of the whole array.
        len: usize,
        /// The first bytes (at most the preview limit).
        preview: Vec<u8>,
    },
}

/// Lower-case hex of `bytes`, two digits per byte, no separator.
#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Hex with a space between bytes (`01 02 ff`).
#[must_use]
pub fn hex_spaced(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        let _ = write!(s, "{b:02x}");
    }
    s
}

impl Node {
    /// An unsigned or signed integer.
    pub fn int(v: impl Into<i128>) -> Self {
        Self::Int(v.into())
    }

    /// A string.
    pub fn str(v: impl Into<String>) -> Self {
        Self::Str(v.into())
    }

    /// An `f32`: finite values print in the shortest form that round-trips, `NaN` and infinities
    /// (which neither JSON nor a plain RON number can hold) become the strings `"NaN"`, `"inf"`
    /// and `"-inf"`.
    #[must_use]
    pub fn f32(v: f32) -> Self {
        if v.is_finite() {
            Self::Num(format!("{v:?}"))
        } else {
            Self::Str(non_finite(f64::from(v)))
        }
    }

    /// An `f64`, as [`Node::f32`].
    #[must_use]
    pub fn f64(v: f64) -> Self {
        if v.is_finite() {
            Self::Num(format!("{v:?}"))
        } else {
            Self::Str(non_finite(v))
        }
    }

    /// A record from `(key, value)` pairs.
    pub fn map<K: Into<String>>(pairs: impl IntoIterator<Item = (K, Node)>) -> Self {
        Self::Map(pairs.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }

    /// Replace [`Node::Struct`] and [`Node::Bytes`] by the plain maps they print as in JSON and
    /// RON, so that the two printers only have to know seven shapes.
    #[must_use]
    pub fn expanded(&self) -> Node {
        match self {
            Node::List(items) => Node::List(items.iter().map(Node::expanded).collect()),
            Node::Map(pairs) => Node::Map(
                pairs
                    .iter()
                    .map(|(k, v)| (k.clone(), v.expanded()))
                    .collect(),
            ),
            Node::Struct { name, members } => Node::map([
                ("type", Node::str(name.clone())),
                (
                    "members",
                    Node::List(
                        members
                            .iter()
                            .map(|(k, v)| {
                                Node::map([("name", Node::str(k.clone())), ("value", v.expanded())])
                            })
                            .collect(),
                    ),
                ),
            ]),
            Node::Bytes { len, preview } => Node::map([
                ("len", Node::int(*len as u64)),
                ("hex", Node::str(hex(preview))),
                ("truncated", Node::Bool(preview.len() < *len)),
            ]),
            other => other.clone(),
        }
    }

    /// Print as JSON (two-space indent, a final newline).
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        print(&self.expanded(), Dialect::Json, 0, &mut out);
        out.push('\n');
        out
    }

    /// Print as RON (two-space indent, a final newline). Records become anonymous structs
    /// `(key: value, …)`, `Null` becomes `None`; the document starts with
    /// `#![enable(implicit_some)]` so that a present optional value needs no `Some(…)`.
    #[must_use]
    pub fn to_ron(&self) -> String {
        let mut out = String::from(RON_HEADER);
        print(&self.expanded(), Dialect::Ron, 0, &mut out);
        out.push('\n');
        out
    }
}

fn non_finite(v: f64) -> String {
    if v.is_nan() {
        "NaN".into()
    } else if v > 0.0 {
        "inf".into()
    } else {
        "-inf".into()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Dialect {
    Json,
    Ron,
}

/// First line of every RON document.
pub const RON_HEADER: &str = "#![enable(implicit_some)]\n";

/// Longest list of scalars printed on one line.
const INLINE_LIST: usize = 16;

fn is_scalar(n: &Node) -> bool {
    !matches!(n, Node::List(_) | Node::Map(_))
}

fn indent(out: &mut String, level: usize) {
    for _ in 0..level {
        out.push_str("  ");
    }
}

fn quote(s: &str, d: Dialect, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => match d {
                Dialect::Json => {
                    let _ = write!(out, "\\u{:04x}", c as u32);
                }
                Dialect::Ron => {
                    let _ = write!(out, "\\u{{{:x}}}", c as u32);
                }
            },
            c => out.push(c),
        }
    }
    out.push('"');
}

fn print(n: &Node, d: Dialect, level: usize, out: &mut String) {
    match n {
        Node::Null => out.push_str(if d == Dialect::Json { "null" } else { "None" }),
        Node::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Node::Int(v) => {
            let _ = write!(out, "{v}");
        }
        Node::Num(s) => out.push_str(s),
        Node::Str(s) => quote(s, d, out),
        Node::List(items) if items.is_empty() => out.push_str("[]"),
        Node::List(items) if items.len() <= INLINE_LIST && items.iter().all(is_scalar) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                print(item, d, level, out);
            }
            out.push(']');
        }
        Node::List(items) => {
            out.push_str("[\n");
            for (i, item) in items.iter().enumerate() {
                indent(out, level + 1);
                print(item, d, level + 1, out);
                if i + 1 < items.len() || d == Dialect::Ron {
                    out.push(',');
                }
                out.push('\n');
            }
            indent(out, level);
            out.push(']');
        }
        Node::Map(pairs) if pairs.is_empty() => {
            out.push_str(if d == Dialect::Json { "{}" } else { "()" });
        }
        Node::Map(pairs) => {
            out.push_str(if d == Dialect::Json { "{\n" } else { "(\n" });
            for (i, (k, v)) in pairs.iter().enumerate() {
                indent(out, level + 1);
                match d {
                    Dialect::Json => {
                        quote(k, d, out);
                        out.push_str(": ");
                    }
                    Dialect::Ron => {
                        out.push_str(k);
                        out.push_str(": ");
                    }
                }
                print(v, d, level + 1, out);
                if i + 1 < pairs.len() || d == Dialect::Ron {
                    out.push(',');
                }
                out.push('\n');
            }
            indent(out, level);
            out.push(if d == Dialect::Json { '}' } else { ')' });
        }
        // `expanded` removed these; print their expansion if one is ever passed in directly.
        other => print(&other.expanded(), d, level, out),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Node {
        Node::map([
            ("name", Node::str("a \"quoted\" \\ line\n\u{1}\u{e9}")),
            ("count", Node::int(-3)),
            ("big", Node::int(u64::MAX)),
            ("ratio", Node::f32(0.1)),
            ("small", Node::f64(1e-9)),
            ("flag", Node::Bool(true)),
            ("none", Node::Null),
            ("nums", Node::List(vec![Node::int(1), Node::int(2)])),
            ("empty", Node::List(vec![])),
            ("rec", Node::map([("a", Node::int(1))])),
            ("norec", Node::Map(vec![])),
            (
                "rows",
                Node::List(vec![Node::map([("x", Node::int(1))]), Node::Null]),
            ),
        ])
    }

    #[test]
    fn json_is_valid_and_has_the_values() {
        let text = sample().to_json();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["name"], "a \"quoted\" \\ line\n\u{1}\u{e9}");
        assert_eq!(v["count"], -3);
        assert_eq!(v["big"], u64::MAX);
        assert_eq!(v["ratio"], 0.1);
        assert_eq!(v["small"], 1e-9);
        assert_eq!(v["flag"], true);
        assert!(v["none"].is_null());
        assert_eq!(v["nums"], serde_json::json!([1, 2]));
        assert_eq!(v["empty"], serde_json::json!([]));
        assert_eq!(v["norec"], serde_json::json!({}));
        assert_eq!(v["rows"][0]["x"], 1);
        assert!(text.ends_with("}\n"));
    }

    #[test]
    fn json_golden_shape() {
        let n = Node::map([
            ("a", Node::int(1)),
            ("l", Node::List(vec![Node::int(1), Node::str("x")])),
            ("m", Node::map([("k", Node::Null)])),
            ("r", Node::List(vec![Node::map([("z", Node::Bool(false))])])),
        ]);
        assert_eq!(
            n.to_json(),
            "{\n  \"a\": 1,\n  \"l\": [1, \"x\"],\n  \"m\": {\n    \"k\": null\n  },\n  \"r\": [\n    {\n      \"z\": false\n    }\n  ]\n}\n"
        );
    }

    #[test]
    fn ron_golden_shape() {
        let n = Node::map([
            ("a", Node::int(1)),
            ("l", Node::List(vec![Node::int(1), Node::str("x")])),
            ("m", Node::map([("k", Node::Null)])),
            ("r", Node::List(vec![Node::map([("z", Node::Bool(false))])])),
            ("e", Node::Map(vec![])),
        ]);
        assert_eq!(
            n.to_ron(),
            "#![enable(implicit_some)]\n(\n  a: 1,\n  l: [1, \"x\"],\n  m: (\n    k: None,\n  ),\n  r: [\n    (\n      z: false,\n    ),\n  ],\n  e: (),\n)\n"
        );
    }

    #[test]
    fn ron_escapes_control_characters_the_ron_way() {
        let n = Node::str("tab\t nl\n bell\u{7} quote\" bs\\ del\u{7f} caf\u{e9}");
        assert_eq!(
            n.to_ron(),
            "#![enable(implicit_some)]\n\"tab\\t nl\\n bell\\u{7} quote\\\" bs\\\\ del\\u{7f} caf\u{e9}\"\n"
        );
        assert_eq!(
            n.to_json(),
            "\"tab\\t nl\\n bell\\u0007 quote\\\" bs\\\\ del\\u007f caf\u{e9}\"\n"
        );
    }

    #[test]
    fn long_scalar_lists_are_multi_line_and_short_ones_inline() {
        let short = Node::List((0..16).map(Node::int).collect());
        assert!(!short.to_json().trim_end().contains('\n'));
        let long = Node::List((0..17).map(Node::int).collect());
        let text = long.to_json();
        assert_eq!(text.lines().count(), 19);
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 17);
    }

    #[test]
    fn floats_print_shortest_and_non_finite_become_strings() {
        assert_eq!(Node::f32(30.0), Node::Num("30.0".into()));
        assert_eq!(Node::f32(0.1), Node::Num("0.1".into()));
        assert_eq!(Node::f32(-0.0), Node::Num("-0.0".into()));
        assert_eq!(Node::f32(f32::NAN), Node::Str("NaN".into()));
        assert_eq!(Node::f32(f32::INFINITY), Node::Str("inf".into()));
        assert_eq!(Node::f64(f64::NEG_INFINITY), Node::Str("-inf".into()));
        assert_eq!(Node::f64(2.5), Node::Num("2.5".into()));
    }

    #[test]
    fn structs_and_bytes_expand_to_maps() {
        let s = Node::Struct {
            name: "CThing".into(),
            members: vec![("A.1".into(), Node::int(7))],
        };
        let v: serde_json::Value = serde_json::from_str(&s.to_json()).unwrap();
        assert_eq!(v["type"], "CThing");
        assert_eq!(v["members"][0]["name"], "A.1");
        assert_eq!(v["members"][0]["value"], 7);
        let b = Node::Bytes {
            len: 5,
            preview: vec![1, 0xab],
        };
        let v: serde_json::Value = serde_json::from_str(&b.to_json()).unwrap();
        assert_eq!(v["len"], 5);
        assert_eq!(v["hex"], "01ab");
        assert_eq!(v["truncated"], true);
        let whole = Node::Bytes {
            len: 1,
            preview: vec![9],
        };
        assert!(whole.to_ron().contains("truncated: false"));
    }

    #[test]
    fn hex_helpers() {
        assert_eq!(hex(&[0, 1, 0xff]), "0001ff");
        assert_eq!(hex_spaced(&[0, 1, 0xff]), "00 01 ff");
        assert_eq!(hex_spaced(&[]), "");
    }
}
