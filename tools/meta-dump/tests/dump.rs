//! Golden outputs, offsets checked against the real bytes, coverage numbers on fixtures where the
//! undecoded bytes are known exactly, and the equality of the JSON and RON documents.

mod common;

use common::*;
use meta_dump::{Format, Options, Outcome, Report, analyze, dump};

fn le32(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(data[at..at + 4].try_into().unwrap())
}

fn parsed<'a>(data: &'a [u8], opts: &Options) -> Box<Report<'a>> {
    match analyze(data, "t", opts) {
        Outcome::Parsed(r) => r,
        Outcome::Damaged(d) => panic!("frame rejected: {}", d.error),
    }
}

fn json(data: &[u8], opts: &Options) -> serde_json::Value {
    serde_json::from_str(&dump(data, "f", Format::Json, opts).text).unwrap()
}

#[test]
fn golden_outputs() {
    let o = Options::default();
    let u = Options {
        unknown_only: true,
        ..Options::default()
    };
    for (name, bytes) in [("sample", sample()), ("rich", rich()), ("broken", broken())] {
        let label = format!("{name}.meta");
        let text = |f, o: &Options| dump(&bytes, &label, f, o).text;
        check_golden(&format!("{name}.txt"), &text(Format::Text, &o));
        check_golden(&format!("{name}.unknown.txt"), &text(Format::Text, &u));
        check_golden(&format!("{name}.json"), &text(Format::Json, &o));
        check_golden(&format!("{name}.ron"), &text(Format::Ron, &o));
    }
    let raw = Options {
        raw: true,
        preview: 12,
        ..Options::default()
    };
    check_golden(
        "sample.raw.txt",
        &dump(&sample(), "sample.meta", Format::Text, &raw).text,
    );
}

/// Every offset the report prints points at the bytes it claims to describe.
fn assert_offsets_are_true(data: &[u8], r: &Report<'_>) {
    assert!(r.problems.is_empty(), "{:?}", r.problems);
    for c in &r.chunks {
        match c.tag.as_str() {
            "header" => assert_eq!(&data[c.span.start..c.span.start + 8], b"CTSEMETA"),
            "METAEND" => assert_eq!(&data[c.span.start..c.span.end], b"METAEND "),
            tag => assert_eq!(
                &data[c.span.start..c.span.start + 4],
                tag.as_bytes(),
                "{tag}"
            ),
        }
    }
    // The chunks tile the file: each starts where the previous one ends.
    let mut at = 0;
    for c in &r.chunks {
        assert_eq!(c.span.start, at, "{}", c.tag);
        at = c.span.end;
    }
    assert_eq!(at, data.len());
    for f in &r.files {
        assert_eq!(le32(data, f.span.start), f.index);
        assert_eq!(le32(data, f.span.start + 4), f.flags);
        assert_eq!(le32(data, f.span.start + 8) as usize, f.path.len());
        assert_eq!(&data[f.span.start + 12..f.span.end], f.path.as_bytes());
    }
    for i in &r.idents {
        assert_eq!(le32(data, i.span.start), i.id);
        assert_eq!(&data[i.span.start + 8..i.span.end], i.text.as_bytes());
    }
    for t in &r.external_types {
        assert_eq!(le32(data, t.span.start), t.id);
        assert_eq!(&data[t.span.start + 8..t.span.end], t.name.as_bytes());
    }
    for t in &r.types {
        assert_eq!(&data[t.span.start..t.span.start + 4], b"DTTY");
        assert_eq!(le32(data, t.span.start + 4), t.id);
        assert_eq!(le32(data, t.span.start + 8) as usize, t.name.len());
    }
    for d in &r.array_dims {
        assert_eq!(&data[d.span.start..d.span.start + 4], b"ADIM");
        assert_eq!(le32(data, d.span.start + 4) as usize, d.dims.len());
    }
    for e in &r.external_objects {
        assert_eq!(le32(data, e.span.start), e.object_id);
        assert_eq!(le32(data, e.span.start + 4), e.file_ref);
        assert_eq!(le32(data, e.span.start + 8), e.flags);
        assert_eq!(le32(data, e.span.start + 12), e.type_id);
    }
    for p in r.object_table.iter().chain(&r.extra_table) {
        assert_eq!(le32(data, p.span.start), p.object_id);
        assert_eq!(le32(data, p.span.start + 4), p.type_id);
    }
    for o in r.objects.iter().chain(&r.extra) {
        assert_eq!(le32(data, o.span.start), o.id);
        assert_eq!(le32(data, o.span.start + 4), o.type_id);
    }
}

#[test]
fn every_printed_offset_points_at_the_bytes_it_describes() {
    let o = Options::default();
    for bytes in [sample(), rich(), old_file(4), old_file(5), old_file(7)] {
        assert_offsets_are_true(&bytes, &parsed(&bytes, &o));
    }
}

#[test]
fn fully_decoded_files_have_no_undecoded_bytes() {
    let o = Options::default();
    for bytes in [sample(), rich(), old_file(4), old_file(6)] {
        let r = parsed(&bytes, &o);
        assert_eq!(r.coverage.total, bytes.len());
        assert_eq!(r.coverage.decoded, bytes.len());
        assert!(r.coverage.undecoded.is_empty());
    }
}

#[test]
fn opaque_bytes_count_the_info_word_and_the_byte_arrays() {
    let o = Options::default();
    // rich: INFO word (4) + 5 + 0 + 1 bytes of CStaticArray<UBYTE>
    assert_eq!(parsed(&rich(), &o).coverage.opaque, 4 + 5 + 1);
    // sample: only the INFO word
    assert_eq!(parsed(&sample(), &o).coverage.opaque, 4);
    // old formats have no INFO flag word
    assert_eq!(parsed(&old_file(5), &o).coverage.opaque, 0);
}

#[test]
fn the_undecoded_range_of_the_broken_file_is_exactly_where_the_decoder_stopped() {
    let bytes = broken();
    let r = parsed(&bytes, &Options::default());
    assert_eq!(r.objects.len(), 1, "only object 0 decodes");
    let cov = &r.coverage;
    assert_eq!(cov.total, bytes.len());
    assert_eq!(cov.undecoded.len(), 1);
    assert_eq!(cov.undecoded_bytes(), BROKEN_UNDECODED);
    assert_eq!(cov.decoded, bytes.len() - BROKEN_UNDECODED);
    let gap = &cov.undecoded[0];
    // It starts right after record 0 ...
    assert_eq!(gap.span.start, r.objects[0].span.end);
    // ... and ends where the EDOB chunk begins.
    let edob = r.chunks.iter().find(|c| c.tag == "EDOB").unwrap();
    assert_eq!(gap.span.end, edob.span.start);
    assert_eq!(gap.section, "OBJS");
    assert!(gap.reason.contains("CMysteryClass"), "{}", gap.reason);
    assert_eq!(
        le32(&bytes, gap.span.start),
        1,
        "record 1 header: object id"
    );
    assert!(r.problems.iter().any(|p| p.contains("2 of 3")));
}

#[test]
fn the_unknown_report_lists_ranges_with_offsets_and_a_hex_preview() {
    let bytes = broken();
    let u = Options {
        unknown_only: true,
        preview: 8,
        ..Options::default()
    };
    let out = dump(&bytes, "b", Format::Text, &u);
    assert_eq!(out.undecoded_bytes, BROKEN_UNDECODED);
    assert!(out.text.contains("undecoded   29 bytes in 1 range(s)"));
    assert!(
        out.text
            .contains("0x00000169..0x00000186  29 bytes  in OBJS")
    );
    assert!(out.text.contains("| 01 00 00 00 02 00 00 00 ..."));
    assert!(
        !out.text.contains("== header"),
        "--unknown prints no tables"
    );
    assert!(!out.text.contains("== RFIL"));
    // With a preview as long as the range the dots go away.
    let u = Options { preview: 100, ..u };
    let out = dump(&bytes, "b", Format::Text, &u);
    assert!(!out.text.contains(" ...\n"));
}

#[test]
fn one_stray_byte_before_edob_is_one_undecoded_byte() {
    let mut bytes = sample();
    let r = parsed(&bytes, &Options::default());
    let at = r
        .chunks
        .iter()
        .find(|c| c.tag == "EDOB")
        .unwrap()
        .span
        .start;
    bytes.insert(at, 0xAA);
    let r = parsed(&bytes, &Options::default());
    assert!(
        r.problems.iter().any(|p| p.contains("object data ends")),
        "{:?}",
        r.problems
    );
    assert_eq!(r.coverage.undecoded_bytes(), 1);
    let gap = &r.coverage.undecoded[0];
    assert_eq!((gap.span.start, gap.span.len()), (at, 1));
}

#[test]
fn info_mismatches_are_printed() {
    let mut bytes = sample();
    // INFO tag at 75; five words follow; the objects count is the fifth.
    let at = 75 + 4 + 4 * 4;
    assert_eq!(le32(&bytes, at), 3);
    bytes[at] = 99;
    let text = dump(&bytes, "x", Format::Text, &Options::default()).text;
    assert!(
        text.contains("mismatch: INFO says 99 objects, the tables hold 3"),
        "{text}"
    );
}

#[test]
fn json_and_ron_hold_the_same_document() {
    let o = Options {
        raw: true,
        ..Options::default()
    };
    for bytes in [sample(), rich(), broken(), old_file(4)] {
        let j = dump(&bytes, "f", Format::Json, &o).text;
        let r = dump(&bytes, "f", Format::Ron, &o).text;
        let jv: serde_json::Value = serde_json::from_str(&j).unwrap();
        let rv = parse_ron(&r).unwrap();
        assert_eq!(jv, rv);
        assert_eq!(jv["schema"], "meta-dump/1");
    }
}

#[test]
fn json_has_offsets_values_and_references() {
    let bytes = rich();
    let v = json(&bytes, &Options::default());
    assert_eq!(v["status"], "ok");
    assert_eq!(v["size"], bytes.len());
    assert_eq!(v["header"]["format_version"], 8);
    assert_eq!(v["header"]["build"]["build"], 57284);
    assert_eq!(v["info"]["flag"], 1);
    assert_eq!(v["references"][1]["path"], "Content/Test/Textures/Wall.tex");
    assert_eq!(v["references"][1]["flags"], 0x0004_0000);
    assert_eq!(v["identifiers"][1]["text"], "Blob");
    assert_eq!(v["external_types"][0]["name"], "CModelConfiguration");
    assert_eq!(v["types"][6]["form"], "struct");
    assert_eq!(v["types"][6]["members"][5]["type_name"], "CBlob*");
    assert_eq!(v["types"][5]["form"], "static-array");
    assert_eq!(v["types"][0]["size_bytes"], 1);
    assert_eq!(
        v["external_objects"][0]["path"],
        "Content/Test/Models/Cube.mdl"
    );
    assert_eq!(v["objects"].as_array().unwrap().len(), 2);
    assert_eq!(v["extra_objects"].as_array().unwrap().len(), 1);
    let first = &v["objects"][0]["value"]["members"];
    assert_eq!(first[0]["name"], "CBlob.1");
    assert_eq!(first[0]["value"]["hex"], "deadbeef01");
    assert_eq!(first[0]["value"]["truncated"], false);
    assert_eq!(first[1]["value"], -9);
    assert_eq!(first[2]["value"], "NaN");
    assert_eq!(first[3]["value"], "caf\u{c3}\u{a9}");
    assert_eq!(first[4]["value"]["text"], "Blob");
    assert_eq!(first[5]["value"]["to"], "external");
    assert_eq!(first[5]["value"]["file"], "Content/Test/Models/Cube.mdl");
    let second = &v["objects"][1]["value"]["members"];
    assert_eq!(second[5]["value"]["to"], "object");
    assert_eq!(
        v["extra_objects"][0]["value"]["members"][5]["value"],
        serde_json::Value::Null
    );
    let objs_chunk = v["chunks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["tag"] == "OBJS")
        .unwrap();
    assert_eq!(
        v["objects"][0]["offset"].as_u64().unwrap(),
        objs_chunk["offset"].as_u64().unwrap() + 8
    );
    assert_eq!(v["coverage"]["decoded"], bytes.len());
    assert_eq!(v["coverage"]["decoded_percent"], "100.0000%");
    assert_eq!(v["coverage"]["undecoded_ranges"], serde_json::json!([]));
}

#[test]
fn raw_adds_hex_and_skip_values_removes_values() {
    let bytes = sample();
    let raw = Options {
        raw: true,
        preview: 4,
        ..Options::default()
    };
    let v = json(&bytes, &raw);
    assert_eq!(v["header"]["hex"], "43545345");
    assert_eq!(v["header"]["hex_truncated"], true);
    assert_eq!(v["chunks"][9]["tag"], "OBJS");
    assert_eq!(v["chunks"][9]["hex"], "4f424a53");
    assert_eq!(v["chunks"][9]["offset"], 0x2f2);
    assert_eq!(v["chunks"][8]["tag"], "EDTY");
    assert_eq!(v["chunks"][8]["hex_truncated"], true, "8 bytes, 4 shown");
    assert_eq!(v["chunks"][10]["tag"], "EDOB");
    let v = json(&bytes, &Options::default());
    assert!(v["header"].get("hex").is_none());
    assert!(v["objects"][0].get("value").is_some());
    let skip = Options {
        skip_values: true,
        ..Options::default()
    };
    let v = json(&bytes, &skip);
    assert!(v["objects"][0].get("value").is_none());
    assert_eq!(v["objects"][0]["type_name"], "CTestItem");
    let text = dump(&bytes, "f", Format::Text, &skip).text;
    assert!(!text.contains("CTestItem.2 = 30.0"));
    assert!(text.contains("object 0  type 6 CTestItem"));
}

#[test]
fn output_is_deterministic() {
    for bytes in [sample(), rich(), broken()] {
        for f in [Format::Text, Format::Json, Format::Ron] {
            let o = Options {
                raw: true,
                ..Options::default()
            };
            assert_eq!(dump(&bytes, "x", f, &o).text, dump(&bytes, "x", f, &o).text);
        }
    }
}

#[test]
fn old_format_versions_are_dumped() {
    for version in 4..=7 {
        let bytes = old_file(version);
        let r = parsed(&bytes, &Options::default());
        assert_eq!(r.header.format_version, version);
        assert_eq!(r.info.flag, None);
        assert_eq!(r.types[1].members.len(), 2);
        assert_eq!(r.objects.len(), 1);
        assert_offsets_are_true(&bytes, &r);
    }
    let text = dump(&old_file(4), "old", Format::Text, &Options::default()).text;
    assert!(text.contains("flag: (none in this format version)"));
    assert!(text.contains("format version: 4"));
}

#[test]
fn a_damaged_frame_gives_a_report_with_the_readable_header_and_all_else_undecoded() {
    let mut bytes = sample();
    bytes.truncate(bytes.len() - 1); // the footer is cut
    let out = dump(&bytes, "cut.meta", Format::Text, &Options::default());
    assert!(!out.parsed);
    assert!(out.text.contains("status: damaged"));
    assert!(out.text.contains("error: "));
    assert!(
        out.text
            .contains("== header 0x00000000..0x0000004b (readable)")
    );
    assert_eq!(out.undecoded_bytes, bytes.len() - 75);
    let u = Options {
        unknown_only: true,
        ..Options::default()
    };
    let out = dump(&bytes, "cut.meta", Format::Text, &u);
    assert!(!out.text.contains("== header"));
    assert!(out.text.contains("0x0000004b..0x0000035b"));
    // Bad magic: nothing is readable.
    let mut bad = sample();
    bad[0] = b'X';
    let out = dump(&bad, "bad", Format::Json, &Options::default());
    let v: serde_json::Value = serde_json::from_str(&out.text).unwrap();
    assert_eq!(v["status"], "damaged");
    assert!(v["header"].is_null());
    assert_eq!(v["coverage"]["undecoded"], bad.len());
    assert!(v["error"].as_str().unwrap().contains("bad magic"));
    // Empty input.
    let out = dump(&[], "empty", Format::Ron, &Options::default());
    assert!(!out.parsed);
    assert_eq!(out.undecoded_bytes, 0);
}

#[test]
fn skip_values_changes_neither_the_coverage_nor_the_opaque_count() {
    let skip = Options {
        skip_values: true,
        ..Options::default()
    };
    for bytes in [sample(), rich(), broken()] {
        let a = parsed(&bytes, &Options::default());
        let b = parsed(&bytes, &skip);
        assert_eq!(a.coverage.opaque, b.coverage.opaque);
        assert_eq!(a.coverage.decoded, b.coverage.decoded);
        assert_eq!(a.objects.len(), b.objects.len());
        assert!(b.objects.iter().all(|o| o.value.is_none()));
        assert!(a.objects.iter().all(|o| o.value.is_some()));
    }
}
