"""Re-runs the measurements behind the world part of docs/re/ctsemeta-objects.md (read-only).

For every format-8 `.wld`: the main object section decodes with the rules (including the
`CMetaHandle` rule) and ends on `EDOB`; the extra records after `EDOB` decode and end on
`METAEND `; every `CMetaHandle` value is an object id of the file (main, extra or external) or null.

Usage: python -I tools/re/wld_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Blob, Obj, Ref, TreeDec, merged_files, parse, u32  # noqa: E402

NULL = 0xFFFFFFFF


def handles(v, out):
    """Collect every Ref that comes from a CMetaHandle: they are found through the holders' member names."""
    if isinstance(v, Obj):
        for k, x in v:
            if v.type in ("CVariantImpHANDLE", "CSMEntry") and k.endswith(".1") and isinstance(x, Ref): out.append(x.oid)
            handles(x, out)
    elif isinstance(v, list) and not isinstance(v, Blob):
        for x in v: handles(x, out)


def main():
    install = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder (the one containing All_PC_01.gro) or set SS2_DATA_DIR")
    st = collections.Counter(); classes = collections.Counter(); extra_classes = collections.Counter(); handle_where = collections.Counter()
    for name, d in sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1])):
        r = parse(d); dec = TreeDec(d, r); st["worlds"] += 1
        objs, edob = dec.objects(); st["main section ends on EDOB"] += d[edob:edob + 4] == b"EDOB"
        count = u32(d, edob + 4); extra, end = dec.extra_objects(edob)
        st["extra records decode"] += 1; st["extra section ends on METAEND"] += d[end:] == b"METAEND "
        st["extra record ids clash with OBTY"] += bool(set(extra) & set(objs)); st["extra records (total)"] += count
        classes.update(o.type for o in objs.values() if isinstance(o, Obj)); extra_classes.update(o.type for o in extra.values() if isinstance(o, Obj))
        known = set(objs) | set(extra) | {e[0] for e in r["exob"]}
        found = []
        for o in list(objs.values()) + list(extra.values()): handles(o, found)
        st["CMetaHandle values in holder members"] += len(found); st["... null (0xFFFFFFFF)"] += sum(1 for x in found if x == NULL)
        st["... an object id of the file (main, extra or external)"] += sum(1 for x in found if x != NULL and x in known)
        st["... neither"] += sum(1 for x in found if x != NULL and x not in known)
        st["objects in main sections"] += len(objs)
    for k, v in sorted(st.items()): print(f"{v:10d}  {k}")
    print("most frequent classes (main sections):", dict(classes.most_common(12)))
    print("most frequent classes (extra records):", dict(extra_classes.most_common(8)))


if __name__ == "__main__":
    main()
