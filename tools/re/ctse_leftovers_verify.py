"""Checks of the CTSEMETA leftovers (docs/re/ctsemeta-leftovers.md, T01-051). Read-only; data files only.

Sections (all by default; pick with --only a,b,...):
  old     the 155 files of format versions 4-7: convert to the version-8 frame (see the spec) and decode them to EDOB
  info    INFO[0] against the presence of extra records
  edty    EDTY pairs against the extra records after EDOB
  flags   RFIL.flags and EXOB.flags tables
  codes   entity-class codes of the worlds (slow: decodes all worlds; not run by default)

Usage: python -I tools/re/ctse_leftovers_verify.py [DATA_DIR] [--only old,info,edty,flags,codes]
"""
import collections, os, re, struct, sys, zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, _natural, merged_files, parse  # noqa: E402


def u32(d, p):
    return struct.unpack_from("<I", d, p)[0]


def to_v8(d, version):
    """Rewrite a version 4-7 file as version 8: insert INFO[0]; for version 4 drop the leading member-id word of every member."""
    p = 20 + u32(d, 16) + 4                                  # just after the INFO tag
    d = d[:p] + struct.pack("<I", 1) + d[p:]
    if version != 4:
        return d
    i = d.find(b"INTY"); n = u32(d, i + 4); q = i + 8; out = bytearray(d[:q])
    for _ in range(n):
        assert d[q:q + 4] == b"DTTY"
        r = q + 12 + u32(d, q + 8)
        if d[r + 12:r + 16] == b"STMB":
            m = u32(d, r + 16); r2 = r + 20; out += d[q:r2]
            for _ in range(m):
                ln = u32(d, r2 + 4); out += d[r2 + 4:r2 + 12 + ln]; r2 += 12 + ln   # skip the id word (4 bytes), keep len+name+type
            q = r2
        else:
            nxt = min(x for x in (d.find(b"DTTY", r), d.find(b"ADIM", r), d.find(b"EXOB", r)) if x >= 0)
            out += d[q:nxt]; q = nxt
        while d[q:q + 4] == b"ADIM":
            k = u32(d, q + 4); out += d[q:q + 8 + 4 * k]; q += 8 + 4 * k
    out += d[q:]
    return bytes(out)


def old_files(install):
    for f in sorted((f for f in os.listdir(install) if f.lower().endswith(".gro")), key=_natural):
        with zipfile.ZipFile(os.path.join(install, f)) as z:
            for i in z.infolist():
                if i.is_dir() or i.file_size < 24: continue
                with z.open(i) as fh: head = fh.read(16)
                if head[:8] == b"CTSEMETA" and u32(head, 12) != 8:
                    yield i.filename, u32(head, 12), z.read(i)


def section_old(install):
    print("== old formats (4-7)")
    c = collections.Counter(); classes = collections.Counter(); first = {}
    for name, v, d in old_files(install):
        c[("files", v)] += 1; first.setdefault(v, d[:0])
        d8 = to_v8(d, v); r = parse(d8); objs, end = TreeDec(d8, r).objects()
        c[("decodes to EDOB exactly", end == r["edob"])] += 1
        c[("extra records", u32(d8, r["edob"] + 4))] += 1
        c[("rfil entries>0", bool(r["rfil"]))] += 1
        for o in objs.values():
            if isinstance(o, Obj): classes[o.type] += 1
    for k in sorted(c, key=str): print("  ", k, c[k])
    print("   object classes:", dict(classes))
    last = {}                                                 # path -> (archive, version) of the copy the game reads (later archives win)
    for f in sorted((f for f in os.listdir(install) if f.lower().endswith(".gro")), key=_natural):
        with zipfile.ZipFile(os.path.join(install, f)) as z:
            for i in z.infolist():
                if i.is_dir() or i.file_size < 24: continue
                with z.open(i) as fh: head = fh.read(16)
                if head[:8] == b"CTSEMETA": last[i.filename.lower()] = (f, u32(head, 12))
    eff = collections.Counter(v for _, v in last.values() if v != 8)
    print("   effective copies (not overridden by a later version-8 file):", sum(eff.values()), dict(eff))


def has_extras(d, r):
    return u32(d, r["edob"] + 4) > 0


def section_info(files):
    print("== INFO[0] against extra records")
    c = collections.Counter()
    for name, d in files.items():
        r = parse(d); c[(r["info"][0], "extras" if has_extras(d, r) else "no extras")] += 1
    for k in sorted(c): print("  ", k, c[k])


def section_edty(files):
    print("== EDTY against EDOB")
    c = collections.Counter(); ids = collections.Counter()
    for name, d in files.items():
        r = parse(d); n = u32(d, r["edob"] + 4)
        c[("EDTY count == EDOB count", len(r["edty"]) == n)] += 1
        if name.endswith(".wld") or not r["edty"]: continue
        dec = TreeDec(d, r); pos = r["edob"] + 8; pairs = []
        for _ in range(n):
            oid, tid = u32(d, pos), u32(d, pos + 4); pairs.append((oid, tid)); _, pos = dec.val(tid, pos + 8)
        c[("non-world EDTY == (id, type) of the records, in order", pairs == r["edty"])] += 1
        c[("extra ids disjoint from OBTY ids", not ({a for a, _ in pairs} & {a for a, _ in r["obty"]}))] += 1
    for k in sorted(c, key=str): print("  ", k, c[k])


def section_flags(files):
    print("== RFIL.flags and EXOB.flags")
    rf = collections.Counter(); ef = collections.defaultdict(collections.Counter); ent = set()
    for name, d in files.items():
        ext = name.rsplit(".", 1)[-1]; r = parse(d)
        for idx, fl, path in r["rfil"]: rf[(ext, fl)] += 1
        for oid, fref, fl, tid in r["exob"]:
            tn = r["exty"].get(tid) or (r["types"][tid].name if tid in r["types"] else "?")
            ef[fl if fl in (1,) or 1000 <= fl < 1100 else ("entity class" if tn == "CEntityClass" else "other")][tn] += 1
            if tn == "CEntityClass": ent.add(fl)
    print("   RFIL.flags per container extension (flag 0 / 0x40000):")
    for ext in sorted({e for e, _ in rf}):
        print("     %-4s %6d / %d" % (ext, rf[(ext, 0)], rf[(ext, 0x40000)]))
    for k in sorted(ef, key=str):
        print("   EXOB.flags", k, dict(ef[k].most_common(6)))
    print("   distinct CEntityClass codes:", len(ent))


def section_codes(files):
    print("== entity-class codes of the worlds")

    def walk(o, f):
        for k, v in o:
            if isinstance(v, Ref): f(v)
            elif isinstance(v, Obj): walk(v, f)
            elif isinstance(v, list):
                for x in v:
                    if isinstance(x, Ref): f(x)
                    elif isinstance(x, Obj): walk(x, f)

    mp = collections.defaultdict(set)
    for name, d in files.items():
        if not name.endswith(".wld"): continue
        r = parse(d)
        ec = {oid: fl for oid, fref, fl, tid in r["exob"] if r["exty"].get(tid) == "CEntityClass"}
        objs, _ = TreeDec(d, r).objects()
        for o in objs.values():
            if isinstance(o, Obj):
                walk(o, lambda v, o=o: mp[o.type].add(ec[v.oid]) if v.oid in ec else None)
    one = sum(1 for s in mp.values() if len(s) == 1)
    print("  classes seen:", len(mp), "with exactly one code:", one)
    for t in sorted(mp): print("   ", t, sorted(mp[t]))


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    only = sys.argv[sys.argv.index("--only") + 1].split(",") if "--only" in sys.argv else ["old", "info", "edty", "flags"]
    args = [a for a in args if a not in (",".join(only),)]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    if "old" in only: section_old(install)
    if set(only) & {"info", "edty", "flags", "codes"}:
        files = merged_files(install, "")
        print(len(files), "format-8 files (later archives override earlier ones)")
        if "info" in only: section_info(files)
        if "edty" in only: section_edty(files)
        if "flags" in only: section_flags(files)
        if "codes" in only: section_codes(files)


if __name__ == "__main__":
    main()
