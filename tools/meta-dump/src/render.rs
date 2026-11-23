//! The structured formats: a [`Node`] tree of the report, printed as JSON or RON.

use crate::Options;
use crate::analyze::{Coverage, Damaged, HeaderInfo, ObjectRec, Outcome, Report, Span, TypeEntry};
use crate::node::{Node, hex};
use crate::text::percent;

/// Schema tag of the JSON and RON documents; bumped when a field changes meaning.
pub const SCHEMA: &str = "meta-dump/1";

fn int(v: usize) -> Node {
    Node::int(v as u64)
}

fn opt_str(v: Option<&str>) -> Node {
    v.map_or(Node::Null, Node::str)
}

fn opt_int(v: Option<u32>) -> Node {
    v.map_or(Node::Null, Node::int)
}

/// `offset`, `size` and, with `--raw`, `hex` (the first bytes) of a span, followed by `rest`.
fn entry(span: Span, data: &[u8], opts: &Options, rest: Vec<(&'static str, Node)>) -> Node {
    let mut pairs = vec![
        ("offset".to_string(), int(span.start)),
        ("size".to_string(), int(span.len())),
    ];
    pairs.extend(rest.into_iter().map(|(k, v)| (k.to_string(), v)));
    if opts.raw {
        let bytes = span.bytes(data);
        let shown = &bytes[..bytes.len().min(opts.preview)];
        pairs.push(("hex".into(), Node::str(hex(shown))));
        pairs.push((
            "hex_truncated".into(),
            Node::Bool(shown.len() < bytes.len()),
        ));
    }
    Node::Map(pairs)
}

fn header_node(h: &HeaderInfo, data: &[u8], opts: &Options) -> Node {
    entry(
        h.span,
        data,
        opts,
        vec![
            ("magic", Node::str("CTSEMETA")),
            ("byte_order_marker", Node::int(0x1234_ABCDu32)),
            ("format_version", Node::int(h.format_version)),
            ("version_string", Node::str(h.version_string.clone())),
            (
                "build",
                h.build.as_ref().map_or(Node::Null, |b| {
                    Node::map([
                        ("product", Node::str(b.product.clone())),
                        ("release", Node::str(b.release.clone())),
                        ("build", Node::int(b.build)),
                        ("tag", Node::str(b.tag.clone())),
                    ])
                }),
            ),
        ],
    )
}

fn coverage_node(c: &Coverage, data: &[u8], opts: &Options) -> Node {
    Node::map([
        ("total", int(c.total)),
        ("decoded", int(c.decoded)),
        ("decoded_percent", Node::str(percent(c.decoded, c.total))),
        ("opaque", int(c.opaque)),
        ("undecoded", int(c.undecoded_bytes())),
        (
            "undecoded_ranges",
            Node::List(
                c.undecoded
                    .iter()
                    .map(|g| {
                        let bytes = g.span.bytes(data);
                        let shown = &bytes[..bytes.len().min(opts.preview)];
                        Node::map([
                            ("offset", int(g.span.start)),
                            ("size", int(g.span.len())),
                            ("section", Node::str(g.section.clone())),
                            ("reason", Node::str(g.reason.clone())),
                            ("hex", Node::str(hex(shown))),
                            ("hex_truncated", Node::Bool(shown.len() < bytes.len())),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
}

fn type_node(t: &TypeEntry, data: &[u8], opts: &Options) -> Node {
    let mut rest = vec![
        ("id", Node::int(t.id)),
        ("name", Node::str(t.name.clone())),
        ("kind", Node::int(t.kind)),
        ("form", Node::str(t.form)),
        ("size_bytes", opt_int(t.size)),
        ("align", opt_int(t.align)),
        ("target", opt_int(t.target)),
        ("target_name", opt_str(t.target_name.as_deref())),
    ];
    if t.form == "struct" {
        rest.push(("flags", opt_int(t.flags)));
        rest.push(("base", opt_int(t.parent)));
        rest.push(("base_name", opt_str(t.parent_name.as_deref())));
        rest.push((
            "members",
            Node::List(
                t.members
                    .iter()
                    .map(|m| {
                        Node::map([
                            ("name", Node::str(m.name.clone())),
                            ("type", Node::int(m.type_id)),
                            ("type_name", opt_str(m.type_name.as_deref())),
                        ])
                    })
                    .collect(),
            ),
        ));
    }
    entry(t.span, data, opts, rest)
}

fn object_node(o: &ObjectRec, data: &[u8], opts: &Options) -> Node {
    let mut rest = vec![
        ("id", Node::int(o.id)),
        ("type", Node::int(o.type_id)),
        ("type_name", opt_str(o.type_name.as_deref())),
    ];
    if let Some(v) = &o.value {
        rest.push(("value", v.clone()));
    }
    entry(o.span, data, opts, rest)
}

fn report_node(r: &Report<'_>, opts: &Options) -> Node {
    let data = r.data;
    let mut pairs = vec![
        ("schema", Node::str(SCHEMA)),
        ("file", Node::str(r.label.clone())),
        ("size", int(data.len())),
        (
            "status",
            Node::str(if r.problems.is_empty() {
                "ok"
            } else {
                "problems"
            }),
        ),
    ];
    if opts.unknown_only {
        pairs.push(("problems", problems(&r.problems)));
        pairs.push(("coverage", coverage_node(&r.coverage, data, opts)));
        return Node::map(pairs);
    }
    pairs.push(("header", header_node(&r.header, data, opts)));
    pairs.push((
        "info",
        entry(
            r.info.span,
            data,
            opts,
            vec![
                ("flag", opt_int(r.info.flag)),
                ("files", Node::int(r.info.file_count)),
                ("identifiers", Node::int(r.info.ident_count)),
                ("types", Node::int(r.info.type_count)),
                ("objects", Node::int(r.info.object_count)),
                (
                    "mismatches",
                    Node::List(r.info.mismatches.iter().cloned().map(Node::Str).collect()),
                ),
            ],
        ),
    ));
    pairs.push((
        "chunks",
        Node::List(
            r.chunks
                .iter()
                .map(|c| {
                    entry(
                        c.span,
                        data,
                        opts,
                        vec![
                            ("tag", Node::str(c.tag.clone())),
                            ("entries", c.entries.map_or(Node::Null, Node::int)),
                        ],
                    )
                })
                .collect(),
        ),
    ));
    pairs.push((
        "references",
        Node::List(
            r.files
                .iter()
                .map(|f| {
                    entry(
                        f.span,
                        data,
                        opts,
                        vec![
                            ("index", Node::int(f.index)),
                            ("flags", Node::int(f.flags)),
                            ("path", Node::str(f.path.clone())),
                        ],
                    )
                })
                .collect(),
        ),
    ));
    pairs.push((
        "identifiers",
        Node::List(
            r.idents
                .iter()
                .map(|i| {
                    entry(
                        i.span,
                        data,
                        opts,
                        vec![("id", Node::int(i.id)), ("text", Node::str(i.text.clone()))],
                    )
                })
                .collect(),
        ),
    ));
    pairs.push((
        "external_types",
        Node::List(
            r.external_types
                .iter()
                .map(|t| {
                    entry(
                        t.span,
                        data,
                        opts,
                        vec![("id", Node::int(t.id)), ("name", Node::str(t.name.clone()))],
                    )
                })
                .collect(),
        ),
    ));
    pairs.push((
        "types",
        Node::List(r.types.iter().map(|t| type_node(t, data, opts)).collect()),
    ));
    pairs.push((
        "array_dims",
        Node::List(
            r.array_dims
                .iter()
                .map(|d| {
                    entry(
                        d.span,
                        data,
                        opts,
                        vec![
                            ("after_types", int(d.after_type)),
                            (
                                "dims",
                                Node::List(d.dims.iter().map(|x| Node::int(*x)).collect()),
                            ),
                        ],
                    )
                })
                .collect(),
        ),
    ));
    pairs.push((
        "external_objects",
        Node::List(
            r.external_objects
                .iter()
                .map(|e| {
                    entry(
                        e.span,
                        data,
                        opts,
                        vec![
                            ("object", Node::int(e.object_id)),
                            ("file_ref", Node::int(e.file_ref)),
                            ("path", Node::str(e.path.clone())),
                            ("flags", Node::int(e.flags)),
                            ("type", Node::int(e.type_id)),
                            ("type_name", opt_str(e.type_name.as_deref())),
                        ],
                    )
                })
                .collect(),
        ),
    ));
    for (key, table) in [
        ("object_table", &r.object_table),
        ("extra_table", &r.extra_table),
    ] {
        pairs.push((
            key,
            Node::List(
                table
                    .iter()
                    .map(|p| {
                        entry(
                            p.span,
                            data,
                            opts,
                            vec![
                                ("object", Node::int(p.object_id)),
                                ("type", Node::int(p.type_id)),
                                ("type_name", opt_str(p.type_name.as_deref())),
                            ],
                        )
                    })
                    .collect(),
            ),
        ));
    }
    pairs.push((
        "objects",
        Node::List(
            r.objects
                .iter()
                .map(|o| object_node(o, data, opts))
                .collect(),
        ),
    ));
    pairs.push((
        "extra_objects",
        Node::List(r.extra.iter().map(|o| object_node(o, data, opts)).collect()),
    ));
    pairs.push(("problems", problems(&r.problems)));
    pairs.push(("coverage", coverage_node(&r.coverage, data, opts)));
    Node::map(pairs)
}

fn problems(p: &[String]) -> Node {
    Node::List(p.iter().cloned().map(Node::Str).collect())
}

fn damaged_node(d: &Damaged<'_>, opts: &Options) -> Node {
    let mut pairs = vec![
        ("schema", Node::str(SCHEMA)),
        ("file", Node::str(d.label.clone())),
        ("size", int(d.data.len())),
        ("status", Node::str("damaged")),
        ("error", Node::str(d.error.clone())),
    ];
    if !opts.unknown_only {
        pairs.push((
            "header",
            d.header
                .as_ref()
                .map_or(Node::Null, |h| header_node(h, d.data, opts)),
        ));
    }
    pairs.push(("coverage", coverage_node(&d.coverage, d.data, opts)));
    Node::map(pairs)
}

/// The document tree of an outcome (what JSON and RON print).
#[must_use]
pub fn to_node(outcome: &Outcome<'_>, opts: &Options) -> Node {
    match outcome {
        Outcome::Parsed(r) => report_node(r, opts),
        Outcome::Damaged(d) => damaged_node(d, opts),
    }
}
