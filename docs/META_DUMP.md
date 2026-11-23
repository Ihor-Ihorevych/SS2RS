# meta-dump: print any CTSEMETA file

`tools/meta-dump` (T11-002) prints a CTSEMETA file (almost every asset of the game: `.tex`, `.mdl`, `.wld`, `.ep`, `.rsc`, ...) as **annotated text**, **JSON** or **RON**: the header, the chunk
list, the type table, the references and the decoded objects, each with its **byte offset and size**, optionally with the **raw bytes**, and with a coverage report of **which bytes of the file some
decoder explained** (`--unknown`). It needs no game data to run on a file you give it; with `--install` it reads from the install through the VFS.

It is a report layer over the existing readers: the frame comes from `ss2_meta::Container::parse` and the values from `ss2_meta::TypeSystem::decode_value`. Nothing in the format is parsed a second
time, and the public API of `ss2-meta` and `ss2-vfs` is unchanged. The format itself is specified in `docs/re/ctsemeta-container.md`, `ctsemeta-types.md` and `ctsemeta-objects.md`.

```
cargo run -p meta-dump -- FILE                          annotated text of a file on disk
cargo run -p meta-dump -- --install DIR content/x.ep    a file of the install (DIR holds All_PC_01.gro; the newest patch wins)
cargo run -p meta-dump -- --format json|ron FILE        structured output
cargo run -p meta-dump -- --raw FILE                    add the first bytes of every region
cargo run -p meta-dump -- --unknown FILE                only the coverage and the undecoded ranges
cargo run -p meta-dump -- --skip-values FILE            leave the object values out (offsets, sizes and types stay)
cargo run -p meta-dump -- --preview N FILE              how many bytes a hex preview shows (default 32)
```

Unlike `gro-tool`, `meta-dump` does not read `SS2_DATA_DIR`: a path on disk is the default mode and `--install DIR` is explicit.

**Exit codes:** `0` success · `1` only with `--unknown`: some bytes are undecoded · `2` the file cannot be read or its frame is damaged (the report is still printed to standard output, the message to
standard error). A damaged file never panics the tool.

## The annotated text

Offsets are file offsets in hex, `+n` is a size in bytes. The first lines of the report on the small test fixture (the testkit `MetaBuilder::sample()`, not game data):

```
# meta-dump (annotated text; offsets are file offsets, +n is a size in bytes)
file: sample.meta
size: 860 bytes
status: ok

== header 0x00000000..0x0000004b (75 bytes)
magic: CTSEMETA   byte-order marker: 0x1234ABCD
format version: 8
version string: "$Version: Serious Sam 2; 2.062.00:57284; 2005-Gold-RC5$"
build: product "Serious Sam 2", release 2.062.00, build 57284, tag 2005-Gold-RC5

== chunks
header   0x00000000..0x0000004b  75 bytes
INFO     0x0000004b..0x00000063  24 bytes
RFIL     0x00000063..0x000000bd  90 bytes  2 entries
IDNT     0x000000bd..0x000000dd  32 bytes  2 entries
EXTY     0x000000dd..0x00000100  35 bytes  1 entries
INTY     0x00000100..0x000002ba  442 bytes  10 entries
EXOB     0x000002ba..0x000002d2  24 bytes  1 entries
OBTY     0x000002d2..0x000002ea  24 bytes  2 entries
EDTY     0x000002ea..0x000002f2  8 bytes  0 entries
OBJS     0x000002f2..0x0000034c  90 bytes  2 entries
EDOB     0x0000034c..0x00000354  8 bytes
METAEND  0x00000354..0x0000035c  8 bytes

== RFIL file references (2)
0x0000006b +40  #0 flags 0x00000000  "Content/Test/Models/Cube.mdl"
0x00000093 +42  #1 flags 0x00040000  "Content/Test/Textures/Wall.tex"
```

Then `INFO` (with a `mismatch:` line for every declared count that disagrees with the tables), `IDNT`, `EXTY`, the type table (every `DTTY` block with its form: `struct`, `primitive`, `alias`,
`enumeration`, `pointer`, `fixed-array`, `static-array`, `dynamic-container`, ...; structs list their members; `ADIM` blocks in file order), `EXOB`, `OBTY`, `EDTY`, the objects and the extra records
after `EDOB`:

```
== INTY type table (10 types, 1 ADIM blocks)
0x00000108 +33  type 0 SLONG  kind 1  primitive size 4 align 4
0x00000129 +29  type 1 INDEX  kind 1  alias of 0 SLONG
0x000001a5 +32  type 5 FLOAT[3]  kind 0  fixed-array of 2 FLOAT
0x000001c5 +12  ADIM after 6 types, dimensions [3]
0x000001d1 +86  type 6 CTestItem  kind 2  struct
    flags 5, base class none, 5 members
      [0] "1" : 1 INDEX
      [1] "2" : 2 FLOAT
...
== OBJS objects (2 of 2 decoded)
0x000002fa +33  object 0  type 6 CTestItem
    CTestItem {
      CTestItem.1 = 7
      CTestItem.2 = 30.0
      CTestItem.3 = {ident: 1, text: "TestItem"}
      CTestItem.4 = {enum: "EMode", value: 2}
      CTestItem.5 = [1.0, 2.0, 3.0]
    }
0x0000031b +49  object 1  type 9 CTestHolder
    CTestHolder {
      ...
      CTestHolder.1 = {ref: 0, to: "object", type: "CTestItem"}
      CTestHolder.2 = [{ref: 0, to: "object", type: "CTestItem"}]
    }
```

Values are named `Owner.member` (the type that declares the member, then the member name as the file spells it, usually a number): a derived class lists its base-class members first. A **reference**
prints as `{ref: id, to: "object" | "external" | "unresolved", type: ..., file: ...}`: `external` objects live in another file of `RFIL` and print that path, `null` is a null pointer. An identifier prints its
text, an enumeration its type name and stored value, raw byte arrays (`CStaticArray<UBYTE>`: pixels, vertex data) print as `bytes[len] <first bytes> ...`. A value longer than 100 columns is broken over
lines. Strings are the file's Latin-1 bytes shown as the characters with those code points; `NaN` and infinite floats print as the strings `"NaN"`, `"inf"`, `"-inf"`.

`--raw` adds a `| 43 54 53 45 ...` line with the first `--preview` bytes under every region (header, chunk, table entry, object).

## `--unknown`: what no decoder explained

`--unknown` prints only the header lines and the coverage block, and exits `1` if any byte is undecoded. On the testkit fixture `broken` (object 1 has a member of a class with no known encoding, so
objects 1 and 2 cannot be decoded):

```
$ meta-dump --unknown broken.meta
# meta-dump (annotated text; offsets are file offsets, +n is a size in bytes)
file: broken.meta
size: 406 bytes
status: problems

== coverage
file size   406 bytes
explained   377 bytes (92.8571%)
  of which opaque (a word of unknown meaning or raw byte arrays): 4 bytes
undecoded   29 bytes in 1 range(s)
  0x00000169..0x00000186  29 bytes  in OBJS
    reason: object 1 (type 2) at byte 361 does not decode: invalid CTSEMETA object data: at byte 373: unsupported type 50: external type `CMysteryClass` has no known encoding
    | 01 00 00 00 02 00 00 00 01 00 00 00 01 02 03 04 05 02 00 00 00 01 00 00 00 21 00 00 00

== problems
- object 1 (type 2) at byte 361 does not decode: invalid CTSEMETA object data: at byte 373: unsupported type 50: external type `CMysteryClass` has no known encoding
- 2 of 3 object records were not decoded
```

How the numbers are made. Every byte of the file starts as unexplained. The tool marks the header, every frame section the container parser consumed (`INFO` to `OBTY`/`EDTY`, the `OBJS` tag and count,
`EDOB`, the footer) and every object record that **decoded and ended exactly where the next record starts**. What stays unmarked is reported as a range with the section it lies in, the reason (the first
decode error when the range starts where decoding stopped) and a hex preview. Object records are decoded in file order, so one record that fails hides **everything after it** up to `EDOB` (the
decoder cannot find the next record without the previous one); that is a limit of the format, not of the tool. The ranges can be as small as one byte (a stray byte before `EDOB`).

**"Explained" is a statement about structure, not about meaning.** Bytes of raw byte arrays (`CStaticArray<UBYTE>`: texture pixels, vertex and index buffers, animation data) and the first `INFO`
word (a flag of unknown meaning) are *read* by the decoder and so count as explained, but nobody knows what is in them. The report prints that share separately (`of which opaque`). The number to
look at for "how much of the file do we understand" is `explained - opaque`.

A file whose frame does not parse (bad magic, truncated, ...) gets `status: damaged`, the parser's message, the header if it is readable, and the rest of the file as one undecoded range.

## JSON and RON

The same document in two syntaxes, with a `schema` field (`meta-dump/1`, bumped when a field changes meaning). Records keep their field order. Excerpt (`--format json`; each region is
`offset`, `size`, then its fields):

```json
{
  "schema": "meta-dump/1",
  "file": "sample.meta",
  "size": 860,
  "status": "ok",
  "header": { "offset": 0, "size": 75, "magic": "CTSEMETA", "format_version": 8, "...": "..." },
  "info": { "...": "..." },
  "chunks": [ { "offset": 0, "size": 75, "tag": "header", "entries": null }, "..." ],
  "references": [ { "offset": 107, "size": 40, "index": 0, "flags": 0, "path": "Content/Test/Models/Cube.mdl" }, "..." ],
  "identifiers": [], "external_types": [], "types": [], "array_dims": [],
  "external_objects": [], "object_table": [], "extra_table": [],
  "objects": [ { "offset": 762, "size": 33, "id": 0, "type": 6, "type_name": "CTestItem",
                 "value": { "type": "CTestItem", "members": [ { "name": "CTestItem.1", "value": 7 }, "..." ] } } ],
  "extra_objects": [],
  "problems": [],
  "coverage": { "total": 860, "decoded": 860, "decoded_percent": "100.0000%", "opaque": 4,
                "undecoded": 0, "undecoded_ranges": [] }
}
```

(The arrays shown empty here are filled in the real output.) With `--raw` every region also has `hex` and `hex_truncated`. A byte-array value is `{len, hex, truncated}`. `--unknown` keeps only
`schema`, `file`, `size`, `status`, `problems` and `coverage`.

RON is the same tree: records are anonymous structs `(offset: 762, size: 33, ...)`, lists `[...]`, `null` is `None`. The document starts with `#![enable(implicit_some)]`, so a present optional value is
written without `Some(...)`. Strings use RON's escapes (`\u{7}`).

**Why no serialization crates.** The tree is built once (`node.rs`) and printed by two small hand-written printers (about 100 lines). RON is not in the dependency tree of the repository, `serde` needs
derives on types that live in `ss2-meta`, and a hand-written printer gives byte-for-byte deterministic output (field order as listed, shortest round-trip floats, no hash order, no timestamps) with no
`cargo deny` question to answer. The tests parse the JSON with `serde_json` (a dev-dependency only) and parse the RON with a small test-only parser and require that both give the same document.
This does **not** prove that the `ron` crate reads the output; see "Not covered".

## Never a panic

Damaged input becomes a report or a message with exit code 2: bad magic, truncation anywhere, counts of 4 billion, a string length past the end of the file, garbage after a valid header. Offsets are
computed from the sizes the format prescribes and **checked against the section ends the container reports**; a disagreement is listed under `problems`, never a wrong silent offset.

## How it was tested

- **Unit tests** of the printers (`node.rs`: JSON and RON escaping, floats, shapes; `text.rs`: line breaking and percentages).
- **Golden files** (`tools/meta-dump/tests/golden/`): the text, JSON, RON, `--unknown` and `--raw` output of three hand-made fixtures (`UPDATE_GOLDEN=1 cargo test -p meta-dump --test dump` rewrites
  them; review the diff).
- **Offsets**: every printed offset is checked against the real bytes (the tag at a chunk offset, the index at a reference offset, the type id at a type offset, the record header at an object offset),
  for format 8 and for the old formats 4, 5 and 7; the chunks must tile the file.
- **Coverage**: on a fixture in which exactly 29 bytes are undecoded the report must give exactly that range, its section and its hex; one stray byte before `EDOB` must give one undecoded byte.
- **Damage**: every truncation (all rejected) and three single-byte corruptions of every byte of five fixtures, in all three formats and with `--raw --unknown`; absurd counts in every counted section;
  noise. No panic, `decoded + undecoded == size`, ranges inside the file and in order.
- **Binary**: the real `meta-dump` executable on files and on a synthetic install (the newest patch wins), exit codes 0/1/2, determinism between runs.
- **Mutation check** of `src/` (see below).
- **Data-gated** (`tests/real_install.rs`, skips without `SS2_DATA_DIR`): runs `analyze` and all three printers over CTSEMETA files of the real install and requires no frame failure, no problem and no
  undecoded byte. By default a deterministic sample (every 20th file by path plus the five largest, files of at most 4 MiB, so no worlds): **410 files, 59,527,064 bytes, 100.0000% explained, 0 problems,
  about 65 s in a debug build**. `SS2_META_DUMP_FULL=1 cargo test --release -p meta-dump --test real_install -- --nocapture` walks everything.

### The whole install

`SS2_META_DUMP_FULL=1 cargo test --release -p meta-dump --test real_install -- --nocapture`, retail install with all patches mounted (the VFS shows 8141 CTSEMETA files; `docs/re/ctsemeta-container.md`
counts 9,424 over the archives before the patches override files):

| | |
|---|---|
| CTSEMETA files | 8,141 (26 extensions; magic checked, none skipped by error) |
| Bytes | 1,391,671,739 |
| Frame failures, problems, undecoded bytes | 0, 0, 0 (no file has an unexplained byte) |
| "Explained" | 1,391,671,739 bytes = **100.0000%** |
| of which **opaque** (raw byte arrays and the `INFO` flag word) | 1,036,988,286 bytes = **74.5139%** of all bytes |
| So bytes of understood structure | 354,683,453 bytes = 25.4861% |
| Time (release, 5,699 files also printed with values, the rest with `--skip-values`) | 198.7 s |

**Caveat, stated exactly:** "explained" means a decoder read the bytes and the object records end exactly where the next one starts. It includes the raw byte arrays (`CStaticArray<UBYTE>`:
texture pixels, vertex and index data, animation data), which are three quarters of the bytes of the install and whose meaning this tool does not interpret. 100% therefore says "the structure of
every file is understood", not "the content of every file is understood". Every extension (`.ans .bmf .cb .ctl .ctr .ep .fnt .lfx .mac .mas .mat .mch .mdl .mtr .pfx .pss .rsc .sca .scp .scs
.sfc .skl .tex .tfx .vfx .wld`) is at 100.0000%. Because of that the `--unknown` mode finds nothing to highlight in the retail install; it is for files the loaders have not met yet (modded or
future files, and the hand-made fixtures).

### Mutation check

`tools/meta-dump/src` was mutated one line at a time with `C:/Games/agents/tools/mutg.py` (an exact one-occurrence replacement) driven by a script that tries operator breakages
(`+`/`-`, `<`/`<=`, `==`/`!=`, `&&`/`||`, `true`/`false`, constants of the offset arithmetic, `min`/`max`, `!`) on every non-test, non-comment line of `analyze.rs`, `text.rs`, `render.rs`,
`node.rs`, `lib.rs` and `main.rs`, runs the unit, `dump` and `cli` tests, and restores the file. A mutant whose tests hang is killed by a timeout and counts as killed.

**170 mutants: 7 did not compile (not counted), 163 ran: 155 killed, 8 survived, all 8 judged equivalent.** The first pass had 23 survivors; tests were added for the 15 that mattered (the
trailing `ADIM` offsets, the `EDOB` header bytes when an extra record fails, the decoded count of a damaged file, `Span` helpers, string escaping, boolean printing, the line-width boundary, long
maps, `hex_truncated` at the exact preview length, the `--unknown` JSON of a damaged file) and the survivors re-run.

The 8 equivalent survivors, with the reason: `Painter::paint` `start < end` to `<=` (painting an empty range changes nothing); the section lookup of a gap (`<=` to `<` and `<` to `<=`, twice) and the
"reason belongs to this gap" test (`<` to `<=` and `&&` to `||`): a gap never starts exactly on a section boundary and no input produces more than one gap that contains a decode failure, so the
boundary cases cannot be reached (a future change that can produce several gaps must revisit this); `String::with_capacity` arguments in `hex` and `hex_spaced` (a capacity hint); `v > 0.0` to
`>=` in the non-finite float name (only infinities reach it, and `0.0` is finite). The script is not part of the repository.

## Not covered

- **The meaning of the bytes.** "Explained" means the generic decoder read them; raw byte arrays (about three quarters of the bytes of the install) and the `INFO` flag word are opaque. `meta-dump` does not
  interpret them (that is what the per-format loaders and `T11-003` are for).
- **Values are generic.** Members are named by the file's own numbers (`CTestItem.1`), not by game field names, and no class-specific view exists.
- **Object records after a failed one** are not decoded, so on a file where decoding stops everything after it up to `EDOB` is one range. Resynchronizing would need knowledge the format does not give.
- **A hex-only mode for files whose frame does not parse** exists only as the damaged report (the readable header and one undecoded range); no table is recovered from a half-readable frame.
- **RON is checked against this repository's own parser only**, not the `ron` crate (not in the dependency tree). The subset used (anonymous structs, lists, strings, numbers, `None`, the `implicit_some`
  extension) is plain RON, but that was not machine-verified against the crate.
- **Large files are big.** A world prints about 200 MB of JSON with values (about 39 s in a release build); use `--skip-values` (about 5 MB, about 1 s) or `--unknown`.
- **No comparison between files** (`meta-diff`, `meta-survey`: T11-003) and no editing or writing back.
- **Linux and gitleaks** were not run for this task; see the pull request.
- Follow-ups are listed as `T11-002b` in `docs/roadmap/T11-tools.md`.
