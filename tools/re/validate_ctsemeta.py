"""Walks every CTSEMETA file of an install with the frame rules from docs/re/ctsemeta-container.md and reports how many parse to the METAEND footer.

Usage: python -I validate_ctsemeta.py [DATA_DIR]   (DATA_DIR defaults to $SS2_DATA_DIR; read-only, no game content is written)
"""
import os, sys
import zipfile, os, struct, collections
arch_dir = (sys.argv[1] if len(sys.argv) > 1 else os.environ.get("SS2_DATA_DIR", ""))
if not arch_dir or not os.path.isdir(arch_dir):
    sys.exit("pass the install folder (the one containing All_PC_01.gro) or set SS2_DATA_DIR")
u32 = lambda d,p: struct.unpack_from("<I", d, p)[0]
res = collections.Counter(); fmt_by_ext = collections.defaultdict(collections.Counter); big = []
def parse(d):
    p = 20 + u32(d,16)
    assert d[p:p+4] == b"INFO"; p += 24
    assert d[p:p+4] == b"RFIL"; n = u32(d,p+4); p += 8
    for _ in range(n): p += 12 + u32(d,p+8)
    assert d[p:p+4] == b"IDNT"; n = u32(d,p+4); p += 8
    for _ in range(n): p += 8 + u32(d,p+4)
    assert d[p:p+4] == b"EXTY"; n = u32(d,p+4); p += 8
    for _ in range(n): p += 8 + u32(d,p+4)
    assert d[p:p+4] == b"INTY"; inty = u32(d,p+4); p += 8
    ntypes = 0
    while True:
        t = d[p:p+4]
        if t == b"DTTY":
            ln = u32(d,p+8); q = p+12+ln
            if d[q+12:q+16] == b"STMB":
                m = u32(d,q+16); r = q+20
                for _ in range(m): r += 8 + u32(d,r)
                p = r
            else:
                nxt = min(x for x in (d.find(b"DTTY",q), d.find(b"ADIM",q), d.find(b"EXOB",q)) if x >= 0); p = nxt
            ntypes += 1
        elif t == b"ADIM":
            p += 8 + 4*u32(d,p+4)
        else: break
    assert t == b"EXOB", t
    assert ntypes == inty, (ntypes, inty)
    n = u32(d,p+4); p += 8 + 16*n
    assert d[p:p+4] == b"OBTY", d[p:p+4]; n = u32(d,p+4); p += 8 + 8*n
    assert d[p:p+4] == b"EDTY"; n = u32(d,p+4); p += 8 + 8*n
    assert d[p:p+4] == b"OBJS"; nobj = u32(d,p+4)
    assert d[-8:] == b"METAEND "
    if d[-16:-12] == b"EDOB" and d[-12:-8] == bytes(4): return "EDOB+0+METAEND at end"
    return "EDOB then more data" if d.find(b"EDOB", p) >= 0 else "no EDOB after OBJS"
for f in sorted(os.listdir(arch_dir)):
    if not f.lower().endswith(".gro"): continue
    with zipfile.ZipFile(os.path.join(arch_dir, f)) as z:
        for i in z.infolist():
            if i.is_dir(): continue
            ext = os.path.splitext(i.filename.lower())[1]
            head = z.open(i).read(24)
            if head[:8] != b"CTSEMETA": continue
            fmt = u32(head,12); fmt_by_ext[ext][fmt] += 1
            if fmt != 8: res["format!=8"] += 1; continue
            if i.file_size > 40_000_000: big.append((i.filename, i.file_size)); continue
            try: res["OK/"+parse(z.read(i))] += 1; res["OK"] += 1
            except AssertionError as e: res["FAIL "+str(e)[:40]] += 1
            except Exception as e: res["ERR "+type(e).__name__] += 1
print(dict(res)); print("big skipped:", big[:5])
print("format numbers by ext (non-8 only):", {e:dict(c) for e,c in fmt_by_ext.items() if any(k!=8 for k in c)})
print("exts that are CTSEMETA:", sorted(fmt_by_ext)[:60])
