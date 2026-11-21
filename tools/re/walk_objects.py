"""Decodes the OBJS section of every format-8 CTSEMETA file with the rules in docs/re/ctsemeta-objects.md.

Read-only. For each file it checks that decoding the object records lands exactly on the `EDOB` marker
and reports how many files do, which rules were exercised and why the others fail.

Usage: python -I walk_objects.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR)
"""
import collections, os, sys, zipfile

import struct, zipfile, os
u32 = lambda d,p: struct.unpack_from("<I", d, p)[0]
i32 = lambda d,p: struct.unpack_from("<i", d, p)[0]
f32 = lambda d,p: struct.unpack_from("<f", d, p)[0]
class Type:
    def __init__(s, **k): s.__dict__.update(k)
def parse(d):
    r = {}
    p = 20 + u32(d,16); r["version"] = d[20:p].decode("latin1")
    r["info"] = [u32(d,p+4+4*i) for i in range(5)]; p += 24
    n = u32(d,p+4); p += 8; r["rfil"] = []
    for _ in range(n):
        idx, fl, ln = u32(d,p), u32(d,p+4), u32(d,p+8); r["rfil"].append((idx, fl, d[p+12:p+12+ln].decode("latin1"))); p += 12+ln
    n = u32(d,p+4); p += 8; r["idnt"] = []
    for _ in range(n):
        idx, ln = u32(d,p), u32(d,p+4); r["idnt"].append(d[p+8:p+8+ln].decode("latin1")); p += 8+ln
    n = u32(d,p+4); p += 8; r["exty"] = {}
    for _ in range(n):
        t, ln = u32(d,p), u32(d,p+4); r["exty"][t] = d[p+8:p+8+ln].decode("latin1"); p += 8+ln
    r["inty"] = u32(d,p+4); p += 8; r["types"] = {}; r["adim"] = []
    while True:
        t = d[p:p+4]
        if t == b"DTTY":
            tid, ln = u32(d,p+4), u32(d,p+8); name = d[p+12:p+12+ln].decode("latin1"); q = p+12+ln
            kind = u32(d,q); T = Type(id=tid, name=name, kind=kind, start=p)
            if d[q+12:q+16] == b"STMB":
                T.flags = u32(d,q+4); T.parent = i32(d,q+8); m = u32(d,q+16); r2 = q+20; T.members = []
                for _ in range(m):
                    ml = u32(d,r2); T.members.append((d[r2+4:r2+4+ml].decode("latin1"), u32(d,r2+4+ml))); r2 += 8+ml
                T.words = None; p = r2
            else:
                nxt = min(x for x in (d.find(b"DTTY",q), d.find(b"ADIM",q), d.find(b"EXOB",q)) if x >= 0)
                T.words = [u32(d,x) for x in range(q+4, nxt, 4)]; T.flags = T.parent = None; T.members = None; p = nxt
            r["types"][tid] = T
        elif t == b"ADIM":
            n = u32(d,p+4); r["adim"].append((p, [u32(d,p+8+4*i) for i in range(n)])); p += 8+4*n
        else: break
    assert t == b"EXOB"; n = u32(d,p+4); r["exob"] = [struct.unpack_from("<IIII",d,p+8+16*i) for i in range(n)]; p += 8+16*n
    assert d[p:p+4] == b"OBTY"; n = u32(d,p+4); r["obty"] = [struct.unpack_from("<II",d,p+8+8*i) for i in range(n)]; p += 8+8*n
    assert d[p:p+4] == b"EDTY"; n = u32(d,p+4); r["edty"] = [struct.unpack_from("<II",d,p+8+8*i) for i in range(n)]; p += 8+8*n
    assert d[p:p+4] == b"OBJS"; r["objs_pos"] = p; r["objs_count"] = u32(d,p+4); r["data_start"] = p+8
    e = d.rfind(b"EDOB"); r["edob"] = e if d[-8:] == b"METAEND " else None
    return r
def tname(r, tid):
    if tid in r["types"]: return r["types"][tid].name
    if tid in r["exty"]: return "ext:"+r["exty"][tid]
    return f"?{tid}"

class Unknown(Exception): pass
def chain(r, tid):
    t = r["types"][tid]; out = []
    if t.parent is not None and t.parent != 0xffffffff and t.parent != -1 and t.parent in r["types"]: out += chain(r, t.parent)
    elif t.parent not in (None, -1, 0xffffffff): raise Unknown(f"parent {t.parent} missing for {t.name}")
    return out + [t]
def is_object_class(r, tid):
    t = r["types"].get(tid)
    while t is not None:
        if t.name == "CObject": return True
        if t.parent in (None, -1, 0xffffffff): return False
        t = r["types"].get(t.parent)
    return False
class Dec:
    def __init__(s, d, r): s.d, s.r, s.log = d, r, collections.Counter()
    def value(s, tid, pos):
        r, d = s.r, s.d
        t = r["types"].get(tid)
        if t is None:
            en = r["exty"].get(tid)
            EXT = {"UBYTE": 1, "SBYTE": 1, "UWORD": 2, "SWORD": 2, "ULONG": 4, "SLONG": 4, "FLOAT": 4, "IDENT": 4, "INDEX": 4, "BOOL": 4}
            if en in EXT: s.log[("extprim", en)] += 1; return pos + EXT[en]
            raise Unknown(f"external/unknown type {tid} {en}")
        if t.members is not None:
            if t.name == "CMetaHandle":
                # declared with no members, but stored by value as one u32 (docs/re/ctsemeta-objects.md, "CMetaHandle")
                s.log[("prim", "CMetaHandle", 4)] += 1; return pos + 4
            if not is_object_class(r, tid) and not any(ct.members for ct in chain(r, tid)):
                s.log[("empty-class-by-value", t.name)] += 1          # these really are 0 bytes: alignment holds in every file
            if t.name == "CTransString":
                if d[pos:pos+4] != b"DTRS": raise Unknown(f"CTransString expected DTRS got {d[pos:pos+4].hex()}")
                s.log[("marker", "struct", "DTRS")] += 1; pos += 4
            for ct in chain(r, tid):
                for name, mt in ct.members:
                    try: pos = s.value(mt, pos)
                    except Unknown as e:
                        msg = str(e)
                        if not msg.startswith("@"): msg = f"@{ct.name}.{name}: " + msg
                        raise Unknown(msg)
            return pos
        nm = t.name
        if t.kind == 0:
            cat, ref = t.words[0], t.words[1]
            MAGIC = {6: b"STAR", 7: b"SSAR", 8: b"DCON"}
            if cat in MAGIC:
                found = d[pos:pos+4]
                if found not in (b"STAR", b"SSAR", b"DCON"): raise Unknown(f"{nm} expected a container marker got {found.hex()}")
                s.log[("marker", cat, found.decode())] += 1
                cnt = u32(d,pos+4); pos += 8
                for _ in range(cnt): pos = (pos + 4) if cat == 8 else s.value(ref, pos)   # DCON: object ids; STAR/SSAR: inline values
                s.log[("container", cat)] += 1
                return pos
            if cat in (2, 11, 12):  # raw pointer X*, Ptr<X>, Handle<X>
                s.log[("ptr", cat)] += 1; return pos + 4
            if cat == 1:        # enum
                sz = ref; s.log[("enum", sz)] += 1
                if sz not in (1, 2, 4): raise Unknown(f"enum {nm} size {sz}")
                return pos + sz
            if cat == 4:   # fixed array X[N]: words = [4, elementTypeId]; the count N is only in the name
                import re as _re
                m = _re.search(r"\[(\d+)\]$", nm)
                if not m: raise Unknown(f"fixed array without [N]: {nm}")
                s.log[("fixedarray",)] += 1
                for _ in range(int(m.group(1))): pos = s.value(ref, pos)
                return pos
            raise Unknown(f"short type {nm} words {t.words}")
        if t.kind == 1 and t.members is None and nm == "CString":
            ln = u32(d, pos); s.log[("cstring",)] += 1
            if ln > 100000: raise Unknown(f"CString length {ln} implausible")
            return pos + 4 + ln
        if t.kind == 1 and t.members is None:
            if len(t.words) == 3:
                sz = t.words[1]; s.log[("prim", nm, sz)] += 1
                if sz not in (1, 2, 4, 8): raise Unknown(f"primitive {nm} size {sz}")
                return pos + sz
            if len(t.words) == 2 and t.words[1] in r["types"]: return s.value(t.words[1], pos)
        raise Unknown(f"type {nm} kind {t.kind} words {t.words}")
    def element(s, et, pos):
        if is_object_class(s.r, et): return pos + 4          # container holds object references
        return s.value(et, pos)
    def decode(s):
        r, d = s.r, s.d; pos = r["data_start"]
        for (oid_t, tid_t) in r["obty"]:
            oid, tid = u32(d,pos), u32(d,pos+4)
            if (oid, tid) != (oid_t, tid_t): raise Unknown(f"object header {(oid,tid)} != OBTY {(oid_t,tid_t)} at {pos}")
            pos += 8; pos = s.value(tid, pos)
        return pos

def main():
    arch = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("SS2_DATA_DIR", "")
    if not arch or not os.path.isdir(arch):
        sys.exit("pass the install folder (the one containing All_PC_01.gro) or set SS2_DATA_DIR")
    res = collections.Counter(); fails = collections.Counter(); ex = {}; markers = collections.Counter(); empties = collections.Counter()
    for f in sorted(os.listdir(arch)):
        if not f.lower().endswith(".gro"): continue
        with zipfile.ZipFile(os.path.join(arch, f)) as z:
            for i in z.infolist():
                if i.is_dir(): continue
                head = z.open(i).read(16)
                if head[:8] != b"CTSEMETA" or u32(head, 12) != 8: continue
                d = z.read(i)
                try: r = parse(d)
                except Exception: res["frame did not parse"] += 1; continue
                dec = Dec(d, r)
                try:
                    end = dec.decode()
                    if d[end:end+4] == b"EDOB":
                        count = u32(d, end + 4); pos = end + 8
                        for _ in range(count):                       # extra records: (object id, type id, value), see ctsemeta-objects.md
                            pos = dec.value(u32(d, pos + 4), pos + 8)
                        tail = d[pos:] == b"METAEND "
                        res["exact; EDOB count 0; METAEND" if count == 0 and tail else
                            "exact; EDOB extra records decode; METAEND" if tail else "EDOB extra records do not end at METAEND"] += 1
                        markers.update({k[1:]: v for k, v in dec.log.items() if k[0] == "marker"})
                        empties.update({k[1]: v for k, v in dec.log.items() if k[0] == "empty-class-by-value"})
                    else:
                        k = f"ended at {d[end:end+4]!r}"; fails[k] += 1; ex.setdefault(k, i.filename); res["decode ended elsewhere"] += 1
                except Unknown as e:
                    k = str(e)[:100]; fails[k] += 1; ex.setdefault(k, i.filename); res["unknown rule"] += 1
    print(dict(res))
    print("container/struct markers seen in files that decode:", dict(markers))
    print("empty classes met by value, encoded as 0 bytes (CMetaHandle is the exception, see its rule):", dict(empties))
    for k, c in fails.most_common(6): print(f"  {c:4d} {k}   e.g. {ex[k]}")

if __name__ == "__main__":
    main()
