"""Re-runs the movie measurements behind docs/re/cutscenes.md on an install (read-only).

Reads the first bytes of every `.ogv` / `.xmv` in the archives (Theora and Vorbis identification headers: frame size,
frame rate, audio channels and rate), checks that every movie referenced by the level `.nfo` files exists and
that every `.srt` belongs to a movie. Prints numbers only.

Usage: python -I tools/re/movie_verify.py [DATA_DIR]
"""
import collections, os, re, struct, sys, zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import _natural  # noqa: E402


def theora(d):
    i = d.find(b"\x80theora")
    if i < 0: return None
    b = d[i + 7:]
    vmaj, vmin, vrev = b[0], b[1], b[2]
    fmbw, fmbh = struct.unpack_from(">HH", b, 3)
    picw = int.from_bytes(b[7:10], "big"); pich = int.from_bytes(b[10:13], "big")
    frn, frd = struct.unpack_from(">II", b, 15)
    return (vmaj, vmin, vrev), (picw, pich), (frn, frd)


def vorbis(d):
    i = d.find(b"\x01vorbis")
    if i < 0: return None
    _, ch, rate = struct.unpack_from("<IBI", d, i + 7)
    return ch, rate


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    names = {}; nfo = {}
    for f in sorted((f for f in os.listdir(install) if f.lower().endswith(".gro")), key=_natural):
        with zipfile.ZipFile(os.path.join(install, f)) as z:
            for i in z.infolist():
                n = i.filename.lower()
                if i.is_dir(): continue
                if n.endswith((".ogv", ".xmv", ".srt")): names[n] = (f, i.filename, i.file_size)
                if n.endswith(".nfo"): nfo[n] = z.read(i)
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter)
    by = collections.defaultdict(list)
    for n, (f, name, size) in names.items():
        if n.endswith((".ogv", ".xmv")): by[f].append((n, name, size))
    for f, items in by.items():
        with zipfile.ZipFile(os.path.join(install, f)) as z:
            for n, name, size in items:
                with z.open(name) as fh: head = fh.read(1 << 16)
                ext = n[-4:]; st[f"{ext} files"] += 1; dist[f"{ext} folder"][os.path.dirname(n).rsplit("/", 1)[-1]] += 1
                if ext == ".ogv":
                    t = theora(head); v = vorbis(head)
                    dist["theora version"][t[0] if t else None] += 1; dist["frame size"][t[1] if t else None] += 1; dist["frame rate (num/den)"][t[2] if t else None] += 1
                    dist["vorbis (channels, rate)"][v] += 1; st["ogv total MB"] += size / 1e6
    # references from level nfo files
    refs = []
    for n, d in nfo.items():
        s = d.decode("utf-8", "replace")
        refs += re.findall(r'(?i)(?:INTRO|OUTRO|EPISODE)_MOVIE\s*=\s*"([^"]+)"', s)
    st["movie references in .nfo"] = len(refs); have = {n.split("/movies/")[-1] if "/movies/" in n else n for n in names}
    missing = []
    for r in refs:
        key = r.replace("\\", "/").lower()
        ok = any(n.endswith("/" + key.split("/")[-1]) or n.endswith(key) for n in names)
        if not ok: missing.append(r)
    st["movie references that name no file"] = len(missing)
    dist["sample references"] = collections.Counter(refs[:6])
    srt = [n for n in names if n.endswith(".srt")]
    st["srt files"] = len(srt)
    st["srt with a same-named .ogv"] = sum(1 for n in srt if any(m.endswith("/" + os.path.basename(n)[:-4] + ".ogv") for m in names))
    for k, v in sorted(st.items()): print(f"{v:10.0f}  {k}")
    for k, v in sorted(dist.items()): print(k, dict(v.most_common(8)))
    if missing: print("missing:", missing[:5])


if __name__ == "__main__":
    main()
