"""Re-runs the measurements behind docs/re/misc-types.md on an install (read-only).

Structured types (.cb .sfc .pss .ctr .mch .mac): root classes, decode, extra records, who refers to them.
Text types (.nfo .dsc .srt .crt .preload .cfg) and the binary .bin: grammar checks and statistics.

Usage: python -I tools/re/misc_verify.py [DATA_DIR]
"""
import collections, os, re, sys, zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, TreeDec, merged_files, parse, u32, _natural  # noqa: E402

STRUCTURED = (".cb", ".sfc", ".pss", ".ctr", ".mch", ".mac")
REFERRERS = (".ep", ".wld", ".rsc", ".mdl", ".ctl", ".cb", ".pfx", ".mch", ".sfc", ".pss", ".mac", ".scs", ".sca", ".scp", ".mas")


def all_entries(install):
    """{lower path: bytes} of every entry, later archives override earlier ones; sizes only for big files."""
    out = {}; sizes = {}
    for f in sorted((f for f in os.listdir(install) if f.lower().endswith(".gro")), key=_natural):
        with zipfile.ZipFile(os.path.join(install, f)) as z:
            for i in z.infolist():
                if i.is_dir(): continue
                k = i.filename.lower(); sizes[k] = i.file_size
                if k.endswith((".nfo", ".dsc", ".srt", ".crt", ".preload", ".cfg", ".bin", ".txt")): out[k] = z.read(i)
    return out, sizes


def text(d):
    bom = d[:3] == b"\xef\xbb\xbf"
    try: s = d[3:].decode("utf-8") if bom else d.decode("utf-8"); utf8 = True
    except UnicodeDecodeError: s = d.decode("latin1"); utf8 = False
    return s, bom, utf8


def structured(install, st, dist):
    refs = collections.defaultdict(set); cache = {}
    for ext in STRUCTURED:
        files = merged_files(install, ext)
        for name, d in sorted(files.items()):
            r = parse(d); objs, edob = TreeDec(d, r).objects(); st[f"{ext} files"] += 1
            st[f"{ext} decode to the METAEND footer"] += TreeDec(d, r).extra_objects(edob)[1] + 8 == len(d)
            roots = [o for o in objs.values() if isinstance(o, Obj) and o.type != "CResourceFile"]
            dist[f"{ext} classes present"].update(set(o.type for o in roots))
            st[f"{ext} extra records after EDOB (files)"] += u32(d, edob + 4) > 0
    for ext in REFERRERS:
        for name, d in merged_files(install, ext).items():
            for e in parse(d).get("rfil", []):
                p = e[2].lower()
                if p.endswith(STRUCTURED): refs[p[-4:] if p[-4] == "." else p[-3:]].add((os.path.splitext(name)[1], p))
    for t, s in sorted(refs.items()):
        c = collections.Counter(r for r, _ in s); dist[f"{t} referenced by (referrer extension: distinct target files)"].update({k: len({p for r2, p in s if r2 == k}) for k in c})
        dist[f"{t} distinct referenced targets"][len({p for _, p in s})] += 1


def nfo(entries, st, dist):
    keys = collections.Counter(); kinds = collections.Counter()
    for k, d in sorted(entries.items()):
        if not k.endswith(".nfo"): continue
        s, bom, utf8 = text(d); st["nfo files"] += 1; st["nfo with BOM"] += bom; st["nfo valid UTF-8"] += utf8
        ks = tuple(m.group(1).upper() for m in re.finditer(r"(?m)^\s*([A-Za-z_]+)\s*=", s))
        kinds[ks] += 1; keys.update(ks)
        for kind in ("LEVEL", "EPISODE"):
            st[f"nfo with the kind word {kind} on its own line"] += bool(re.search(rf"(?m)^\s*{kind}\s*(;|$)", s))
        dist["nfo folder"][os.path.dirname(k).rsplit("/", 1)[-1] if "/" in k else k] += 1
    dist["nfo key sets"] = collections.Counter({" ".join(k): v for k, v in kinds.most_common(10)})
    dist["nfo keys"] = keys


def dsc(entries, st, dist):
    n = 0; langs = collections.Counter()
    for k, d in entries.items():
        if not k.endswith(".dsc"): continue
        s, bom, utf8 = text(d); st["dsc files"] += 1; st["dsc valid UTF-8"] += utf8; st["dsc with BOM"] += bom
        st["dsc single paragraph (no blank line)"] += "\n\n" not in s.replace("\r", "")
        m = re.search(r"locales/([a-z]+)/", k); langs[m.group(1) if m else "(base)"] += 1
    dist["dsc by language folder"] = langs


CUE = re.compile(r"(\d+)\r?\n(\d\d):(\d\d):(\d\d),(\d\d\d) --> (\d\d):(\d\d):(\d\d),(\d\d\d)\r?\n")


def cues(entries, st, dist, ext):
    attrs = collections.Counter(); langs = collections.Counter(); n = 0; mono = 0; files = 0
    for k, d in entries.items():
        if not k.endswith(ext): continue
        files += 1; s, bom, utf8 = text(d); st[f"{ext} valid UTF-8"] += utf8; st[f"{ext} with BOM"] += bom
        blocks = [b for b in re.split(r"\r?\n\r?\n", s.strip("﻿\r\n")) if b.strip()]
        ok = 0; prev = -1; ordered = True
        for b in blocks:
            m = CUE.match(b + "\n")
            if m:
                ok += 1; t0 = ((int(m[2]) * 60 + int(m[3])) * 60 + int(m[4])) * 1000 + int(m[5]); t1 = ((int(m[6]) * 60 + int(m[7])) * 60 + int(m[8])) * 1000 + int(m[9])
                st[f"{ext} cue ends after it starts"] += t1 > t0; st[f"{ext} cues"] += 1; ordered &= t0 >= prev; prev = t0
                attrs.update(a for a in re.findall(r"<[A-Za-z]+\s+([a-z]+)=", b))
                dist[f"{ext} tags"].update(re.findall(r"<(/?[A-Za-z]+)", b))
        st[f"{ext} files whose every block is a cue"] += ok == len(blocks); st[f"{ext} files with cues in time order"] += ordered
        m = re.search(r"locales/([a-z]+)/", k); langs[m.group(1) if m else "(base)"] += 1
    st[f"{ext} files"] = files; dist[f"{ext} tag attributes"] = attrs; dist[f"{ext} by language folder"] = langs


def preload(entries, sizes, st, dist):
    for k, d in entries.items():
        if not k.endswith(".preload"): continue
        s, bom, utf8 = text(d); st["preload files"] += 1
        lines = [l for l in s.splitlines() if l.strip()]
        ok = all(re.fullmatch(r"(\d+:[^:*?\"<>|]+|-1:)", l) for l in lines); st["preload: every line is <size>:<path>"] += ok
        base = k[:-len(".preload")]; st["preload belongs to an existing .wld"] += base in sizes
        exist = size_eq = 0
        for l in lines:
            n, p = l.split(":", 1); q = p.lower()
            if q in sizes: exist += 1; size_eq += sizes[q] == int(n)
        st["preload lines"] += len(lines); st["preload lines naming an existing file"] += exist; st["preload lines whose size equals the entry's size"] += size_eq
        dist["preload extensions listed"].update(os.path.splitext(l.split(":", 1)[1])[1].lower() for l in lines)
        real = [l for l in lines if not l.startswith("-1:")]
        dist["preload duplicate paths in a file (separators excluded)"][len(real) - len(set(real))] += 1


def other(entries, st, dist):
    for k, d in entries.items():
        if k.endswith(".bin"):
            dist[".bin"][(k, len(d), d.count(0), len(set(d)))] += 1
        if k.endswith(".cfg"):
            s, bom, utf8 = text(d); dist[".cfg sections"].update(re.findall(r"(?m)^\[(\w+)\]", s)); st["cfg files"] += 1


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter)
    entries, sizes = all_entries(install)
    structured(install, st, dist); nfo(entries, st, dist); dsc(entries, st, dist)
    cues(entries, st, dist, ".srt"); cues(entries, st, dist, ".crt"); preload(entries, sizes, st, dist); other(entries, st, dist)
    for k, v in sorted(st.items()): print(f"{v:6d}  {k}")
    for k, v in sorted(dist.items()): print(k, dict(v.most_common(10)))


if __name__ == "__main__":
    main()
