"""Re-runs the measurements behind docs/re/tex.md on an install (read-only).

Checks, for every format-8 `.tex`: the object classes, the pixel-blob zlib stream, and that the
inflated size equals the size predicted by the format code (24-bit, 32-bit, DXT1/3/5 with mip
chain, cube maps = 6 DXT1 faces). Also prints the flag, wrap and dimension statistics.

Usage: python -I tex_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR)
"""
import collections, os, sys, zlib

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import TreeDec, merged_files, parse  # noqa: E402


def mip_chain(w, h):
    out = []
    while True:
        out.append((w, h))
        if w == 1 and h == 1: return out
        w, h = max(1, w // 2), max(1, h // 2)


def block_bytes(w, h, per_block):
    return max(1, (w + 3) // 4) * max(1, (h + 3) // 4) * per_block


def expected(fmt, flags, w, h):
    """Raw size predicted by the format code, and a label."""
    if fmt in (0, 2, 6): return w * h * 3, "24-bit BGR, 1 level"
    if fmt in (1, 3, 4, 5, 11): return w * h * 4, "32-bit BGRA, 1 level"
    if fmt in (7, 8, 9):
        per, name = {7: (8, "DXT1"), 8: (16, "DXT3"), 9: (16, "DXT5")}[fmt]
        one = sum(block_bytes(a, b, per) for a, b in mip_chain(w, h))
        return (one * 6, f"{name} cube map, 6 faces x mip chain") if flags & 2 else (one, f"{name}, full mip chain")
    return None, "unknown format code"


def main():
    install = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder (the one containing All_PC_01.gro) or set SS2_DATA_DIR")
    ok = collections.Counter(); bad = []; fmts = collections.Counter(); flags = collections.Counter()
    wraps = collections.Counter(); classes = collections.Counter(); dims = []; failed = 0
    ui = collections.defaultdict(lambda: [0, 0])      # flags -> [interface or menu textures, all]
    for name, d in sorted(merged_files(install, ".tex").items()):
        try:
            r = parse(d); objs, end = TreeDec(d, r).objects(); assert end == r["edob"]
        except Exception:
            failed += 1; continue
        classes[tuple(r["types"][t].name for _, t in r["obty"])] += 1
        tex = next(v for v in objs.values() if v.type == "CStaticTexture")
        g = lambda k: tex.get("CBaseTexture." + k)
        w, h, fl, fmt, blob = g("1"), g("2"), g("4").v, g("6").v, g("18")
        parts = name.split("/"); folder = parts[parts.index("textures") + 1] if "textures" in parts[:-1] else ""
        ui[hex(fl)][0] += folder in ("interface", "menu"); ui[hex(fl)][1] += 1
        fmts[fmt] += 1; flags[hex(fl)] += 1; wraps[(hex(g("9").v), hex(g("10").v))] += 1; dims.append((w, h))
        raw = zlib.decompress(bytes(blob))
        size, label = expected(fmt, fl, w, h)
        if size == len(raw): ok[label] += 1
        else: bad.append((name, fmt, w, h, len(raw), size))
    print(f"decoded {sum(fmts.values())} format-8 textures ({failed} failed); object classes: {dict(classes)}")
    print("format codes:", dict(sorted(fmts.items())))
    print("flags:", dict(flags.most_common())); print("wrap (u, v):", dict(wraps.most_common(6)))
    print(f"dimensions: width {min(w for w, _ in dims)}..{max(w for w, _ in dims)}, "
          f"height {min(h for _, h in dims)}..{max(h for _, h in dims)}; all powers of two: "
          f"{all(w & (w-1) == 0 and h & (h-1) == 0 for w, h in dims)}")
    nonsq = [max(w, h) // min(w, h) for w, h in dims if w != h]
    print(f"non-square: {len(nonsq)} (aspect ratios up to {max(nonsq)}:1)")
    print("share under textures/interface or textures/menu, by flags:", {k: f"{a}/{b}" for k, (a, b) in sorted(ui.items())})
    print("inflated size == predicted size:")
    for k, v in sorted(ok.items()): print(f"   {v:5d}  {k}")
    print(f"mismatches: {len(bad)}", bad[:3])


if __name__ == "__main__":
    main()
