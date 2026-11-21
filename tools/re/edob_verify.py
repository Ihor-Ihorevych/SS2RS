"""Checks the rule for the data after `EDOB` (docs/re/ctsemeta-objects.md, "The EDOB section").

For every format-8 file whose main object section decodes: if the u32 after `EDOB` is non-zero,
decode that many extra (object id, type id, value) records and require the cursor to land exactly
on the 8-byte `METAEND ` footer, with no id clashing with the main section. World files (`.wld`)
are skipped: their main section does not decode yet (T01-053).

Usage: python -I tools/re/edob_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, TreeDec, merged_files, parse, u32  # noqa: E402


def main():
    install = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder (the one containing All_PC_01.gro) or set SS2_DATA_DIR")
    per_ext = collections.defaultdict(collections.Counter); classes = collections.defaultdict(collections.Counter)
    for name, d in sorted(merged_files(install, "").items()):
        ext = name.rsplit(".", 1)[-1]
        if ext == "wld": continue
        try:
            r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects()
        except Exception:
            per_ext[ext]["main section does not decode"] += 1; continue
        if u32(d, edob + 4) == 0:
            per_ext[ext]["no extra data"] += 1; continue
        try:
            extra, end = dec.extra_objects(edob)
        except Exception as e:
            per_ext[ext][f"extra failed: {str(e)[:40]}"] += 1; continue
        good = d[end:] == b"METAEND " and not set(extra) & set(objs)
        per_ext[ext]["extra decodes exactly to METAEND" if good else "extra misaligned"] += 1
        classes[ext].update(o.type for o in extra.values() if isinstance(o, Obj))
    for ext, c in sorted(per_ext.items()):
        if len(c) > 1 or "no extra data" not in c: print(f".{ext}: {dict(c)}")
    print(f"(extensions with only 'no extra data' are omitted; {sum(c['no extra data'] for c in per_ext.values())} files have none)")
    for ext, c in sorted(classes.items()): print(f".{ext} extra classes:", dict(c.most_common(5)))


if __name__ == "__main__":
    main()
