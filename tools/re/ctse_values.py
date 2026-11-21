"""Value-tree decoder for CTSEMETA objects (builds on walk_objects.py; rules: docs/re/ctsemeta-objects.md).

Turns the objects of a format-8 file into Python values: class instances become `Obj` (an ordered
list of (qualified member name, value)), containers become lists, byte arrays become `Blob`,
pointers `Ref`, enumerations `Enum`. Read-only; used by the evidence scripts (tex_verify.py).
"""
import os, re, struct, sys, zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from walk_objects import Unknown, chain, parse, u32  # noqa: E402


class Ref:
    def __init__(self, oid): self.oid = oid
    def __repr__(self): return "null" if self.oid == 0xFFFFFFFF else f"->obj{self.oid}"


class Enum:
    def __init__(self, name, value): self.name, self.v = name, value
    def __repr__(self): return f"{self.name}({self.v:#x})"


class Obj(list):
    """Ordered (member, value) pairs of a class instance; member names are `Class.member`."""
    def __init__(self, type_name):
        super().__init__(); self.type = type_name
    def get(self, qualified, default=None):
        for k, v in self:
            if k == qualified: return v
        return default


class Blob(bytes):
    def __repr__(self): return f"<{len(self)} bytes>"


PRIM = {"FLOAT": "<f", "SLONG": "<i", "ULONG": "<I", "UBYTE": "<B", "SBYTE": "<b", "UWORD": "<H",
        "SWORD": "<h", "SQUAD": "<q", "UQUAD": "<Q", "IDENT": "<I", "INDEX": "<i"}
EXT_SIZE = {"UBYTE": 1, "SBYTE": 1, "UWORD": 2, "SWORD": 2, "ULONG": 4, "SLONG": 4, "FLOAT": 4,
            "IDENT": 4, "INDEX": 4, "BOOL": 4}


class TreeDec:
    def __init__(self, d, r): self.d, self.r = d, r

    def prim(self, name, size, pos):
        fmt = PRIM.get(name) or {1: "<B", 2: "<H", 4: "<I", 8: "<Q"}[size]
        return struct.unpack(fmt, self.d[pos:pos + size])[0], pos + size

    def val(self, tid, pos):
        r, d = self.r, self.d
        t = r["types"].get(tid)
        if t is None:
            en = r["exty"].get(tid)
            if en in EXT_SIZE: return self.prim(en, EXT_SIZE[en], pos)
            raise Unknown(f"external type {tid} {en}")
        if t.members is not None:
            if t.name == "CTransString": pos += 4            # DTRS marker
            o = Obj(t.name)
            for ct in chain(r, tid):
                for mname, mt in ct.members:
                    v, pos = self.val(mt, pos); o.append((f"{ct.name}.{mname}", v))
            return o, pos
        if t.name == "CString":
            ln = u32(d, pos); return d[pos + 4:pos + 4 + ln].decode("latin1"), pos + 4 + ln
        if t.kind == 0:
            cat, ref = t.words[0], t.words[1]
            if cat in (6, 7, 8):
                cnt = u32(d, pos + 4); pos += 8; et = r["types"].get(ref)
                if cat != 8 and et is not None and et.members is None and et.kind == 1 \
                        and len(et.words) == 3 and et.words[1] == 1:
                    return Blob(d[pos:pos + cnt]), pos + cnt        # array of bytes
                items = []
                for _ in range(cnt):
                    if cat == 8: items.append(Ref(u32(d, pos))); pos += 4
                    else:
                        v, pos = self.val(ref, pos); items.append(v)
                return items, pos
            if cat in (2, 11, 12): return Ref(u32(d, pos)), pos + 4
            if cat == 1:
                v, p2 = self.prim("U", ref, pos); return Enum(t.name, v), p2
            if cat == 4:
                n = int(re.search(r"\[(\d+)\]$", t.name).group(1)); items = []
                for _ in range(n):
                    v, pos = self.val(ref, pos); items.append(v)
                return items, pos
        if t.kind == 1 and t.members is None:
            if len(t.words) == 3: return self.prim(t.name, t.words[1], pos)
            if len(t.words) == 2 and t.words[1] in r["types"]: return self.val(t.words[1], pos)
        raise Unknown(f"type {t.name}")

    def objects(self):
        pos = self.r["data_start"]; out = {}
        for (oid, tid) in self.r["obty"]:
            assert u32(self.d, pos) == oid and u32(self.d, pos + 4) == tid
            out[oid], pos = self.val(tid, pos + 8)
        return out, pos


def _natural(name):
    lower = name.lower()
    return (1 if lower.startswith("patch") else 0,
            [int(t) if t.isdigit() else t for t in re.split(r"(\d+)", lower)])


def merged_files(install, suffix):
    """{path: bytes} of every file ending in `suffix`; later archives (patches) override earlier ones."""
    out = {}
    for f in sorted((f for f in os.listdir(install) if f.lower().endswith(".gro")), key=_natural):
        with zipfile.ZipFile(os.path.join(install, f)) as z:
            for i in z.infolist():
                if not i.is_dir() and i.filename.lower().endswith(suffix):
                    d = z.read(i)
                    if d[:8] == b"CTSEMETA" and u32(d, 12) == 8: out[i.filename.lower()] = d
    return out
