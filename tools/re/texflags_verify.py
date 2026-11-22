"""Re-runs the measurements behind docs/re/tex-flags.md on an install (read-only).

For every format-8 `.tex` (and, for the lightmap check, the embedded textures of the smallest worlds):
 * cross-tabs of `TexFlags` against the format code, mip chain, size class, folder and alpha format,
 * the wrap codes of both axes against folder, and a **seam test** on a sample of textures: how much
   the left and right (and top and bottom) edge columns differ, compared with two neighbouring columns,
   and how many edge pixels are transparent or black. A texture meant to repeat has seams as smooth as
   its interior; a texture meant to clamp usually does not.
 * the flags of embedded world textures that are lightmaps (reached through a texture oven modifier)
   against the other embedded textures.

Usage: python -I tools/re/texflags_verify.py [DATA_DIR] [--worlds N] [--sample N] [--tables docs/re/tex-flags-tables.md]
       (DATA_DIR defaults to $SS2_DATA_DIR; --worlds = how many of the smallest worlds to read, default 10;
        --sample = textures per wrap class for the seam test, default 60; about 10 minutes)
"""
import collections, os, statistics, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
from texpng import decode_static_texture  # noqa: E402

BITS = (0x01, 0x02, 0x04, 0x10, 0x20, 0x40)
ALPHA_FORMATS = {1, 3, 4, 5, 8, 9, 11}


def folder_of(name):
    parts = name.split("/")
    return parts[parts.index("textures") + 1] if "textures" in parts[:-1] and parts.index("textures") + 2 < len(parts) else "(other)"


def seam(w, h, px):
    """(u_ratio, v_ratio, transparent-or-black edge share) for an RGBA bytearray (rows top to bottom)."""
    def pix(x, y): i = (y * w + x) * 4; return px[i:i + 4]

    def diff(a, b): return sum(abs(a[k] - b[k]) for k in range(3)) / 3.0
    ys = range(0, h, max(1, h // 64)); xs = range(0, w, max(1, w // 64))
    u_seam = statistics.fmean(diff(pix(0, y), pix(w - 1, y)) for y in ys)
    u_in = statistics.fmean(diff(pix(w // 2, y), pix(w // 2 + 1, y)) for y in ys) + 0.5
    v_seam = statistics.fmean(diff(pix(x, 0), pix(x, h - 1)) for x in xs)
    v_in = statistics.fmean(diff(pix(x, h // 2), pix(x, h // 2 + 1)) for x in xs) + 0.5
    edge = [pix(x, 0) for x in xs] + [pix(x, h - 1) for x in xs] + [pix(0, y) for y in ys] + [pix(w - 1, y) for y in ys]
    dead = sum(1 for p in edge if p[3] < 16 or (p[0] + p[1] + p[2]) < 24) / len(edge)
    return u_seam / u_in, v_seam / v_in, dead


def main():
    args = sys.argv[1:]
    def opt(name, default):
        if name in args:
            i = args.index(name); v = args[i + 1]; del args[i:i + 2]; return v
        return default
    out = opt("--tables", None); worlds = int(opt("--worlds", 10)); sample = int(opt("--sample", 60))
    install = args[0] if args and os.path.isdir(args[0]) else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder (the one containing All_PC_01.gro) or set SS2_DATA_DIR")
    lines = []

    def emit(s=""): lines.append(s); print(s)

    rows = []   # (name, flags, fmt, w, h, wu, wv, color, folder, obj)
    for name, d in sorted(merged_files(install, ".tex").items()):
        try:
            r = parse(d); objs, end = TreeDec(d, r).objects()
            tex = next(v for v in objs.values() if v.type == "CStaticTexture")
        except Exception:
            continue
        g = lambda k: tex.get("CBaseTexture." + k)
        rows.append((name, g("4").v, g("6").v, g("1"), g("2"), g("9").v, g("10").v, g("11"), folder_of(name), tex))
    emit(f"textures read: {len(rows)}")

    # ---- flags against everything cheap
    by_flags = collections.defaultdict(list)
    for r_ in rows: by_flags[r_[1]].append(r_)
    emit("\n== flags x format code")
    for fl, rs in sorted(by_flags.items()):
        emit(f"  flags {fl:#04x} ({len(rs)}): formats " + str(dict(sorted(collections.Counter(x[2] for x in rs).items()))))
    emit("\n== bit presence (textures with the bit set / all)")
    for b in BITS:
        rs = [x for x in rows if x[1] & b]
        folders = collections.Counter(x[8] for x in rs).most_common(5)
        emit(f"  bit {b:#04x}: {len(rs)}/{len(rows)}; top folders {folders}; formats {dict(sorted(collections.Counter(x[2] for x in rs).items()))}")
    emit("\n== textures with rare bits (0x40 or 0x10 set), first 8")
    for x in [x for x in rows if x[1] & 0x50][:8]:
        emit(f"  {x[1]:#04x} fmt {x[2]} {x[3]}x{x[4]} wrap {x[5]:#x}/{x[6]:#x} {x[0][-70:]}")
    emit("\n== bit 0x01 (0x21 against 0x20 and 0x25 against 0x24): size, mip chain, alpha, folder")
    for lo, hi in ((0x20, 0x21), (0x24, 0x25)):
        for fl in (lo, hi):
            rs = by_flags.get(fl, [])
            if not rs: continue
            big = sum(1 for x in rs if max(x[3], x[4]) >= 512)
            alpha = sum(1 for x in rs if x[2] in ALPHA_FORMATS)
            dxt = sum(1 for x in rs if x[2] in (7, 8, 9))
            emit(f"  {fl:#04x}: n {len(rs)}; >=512 px {big}; alpha formats {alpha}; DXT (mips) {dxt}; top folders {collections.Counter(x[8] for x in rs).most_common(4)}")
    emit("\n== bit 0x04 (0x24, 0x25 against the rest): folders")
    ui = sum(1 for x in rows if x[1] & 4 and x[8] in ("interface", "menu")); emit(f"  bit 0x04 textures in interface or menu: {ui} of {sum(1 for x in rows if x[1] & 4)}")

    # ---- wrap codes
    emit("\n== wrap (u, v) x folder (top 5 folders per pair)")
    pairs = collections.defaultdict(list)
    for x in rows: pairs[(x[5], x[6])].append(x)
    for k, rs in sorted(pairs.items(), key=lambda kv: -len(kv[1])):
        emit(f"  {k[0]:#x}/{k[1]:#x}: {len(rs)}; folders {collections.Counter(x[8] for x in rs).most_common(5)}; border colours {collections.Counter(x[7] for x in rs).most_common(2)}")

    # ---- seam test on a sample
    emit("\n== seam test (ratio = edge-to-edge difference / neighbouring-column difference; ~1 means the seam looks like the interior)")
    for axis, idx in (("U", 5), ("V", 6)):
        for code in (0x50, 0x51, 0x52):
            cand = [x for x in rows if x[idx] == code and max(x[3], x[4]) <= 256 and x[2] in (0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 11)]
            step = max(1, len(cand) // sample)
            pick = cand[::step][:sample]
            ratios, deads, live = [], [], []
            for x in pick:
                try:
                    w, h, px = decode_static_texture(x[9])
                    ru, rv, dead = seam(w, h, px)
                except Exception:
                    continue
                ratios.append(ru if axis == "U" else rv); deads.append(dead)
                if dead < 0.5: live.append(ru if axis == "U" else rv)
            if ratios:
                tile = sum(1 for v in ratios if v < 2.0) / len(ratios)
                emit(f"  {axis} {code:#x}: {len(ratios)} sampled of {len(cand)}; median ratio {statistics.median(ratios):.2f}; seam as smooth as the interior (ratio < 2) in {tile:.0%}; edge pixels transparent or black {statistics.fmean(deads):.0%}")
                if live:
                    emit(f"      only textures whose edges are not mostly transparent or black: {len(live)}; median ratio {statistics.median(live):.2f}; seam smooth (ratio < 2) in {sum(1 for v in live if v < 2.0) / len(live):.0%}")

    # ---- format codes that share a layout: what the pixels look like
    emit("\n== format codes that share a layout (sample per code: what the decoded pixels look like)")
    for codes in ((0, 2, 6), (1, 3, 4, 5, 11)):
        for code in codes:
            cand = [x for x in rows if x[2] == code and max(x[3], x[4]) <= 256]
            pick = cand[::max(1, len(cand) // sample)][:sample]
            gray = alpha_var = alpha_zero = n = 0; folders = collections.Counter(); sizes = collections.Counter()
            for x in pick:
                try:
                    w, h, px = decode_static_texture(x[9])
                except Exception:
                    continue
                n += 1; folders[x[8]] += 1; sizes[(x[3], x[4])] += 1
                step = max(1, (w * h) // 2000)
                pix = [px[i * 4:i * 4 + 4] for i in range(0, w * h, step)]
                gray += all(abs(p[0] - p[1]) < 6 and abs(p[1] - p[2]) < 6 for p in pix)
                alphas = {p[3] for p in pix}; alpha_var += len(alphas) > 1; alpha_zero += any(a < 8 for a in alphas)
            if n:
                emit(f"  code {code}: {len(cand)} textures up to 256 px, {n} sampled; grayscale {gray / n:.0%}; alpha varies {alpha_var / n:.0%}; some transparent {alpha_zero / n:.0%}; folders {folders.most_common(3)}")

    # ---- lightmaps
    emit(f"\n== embedded world textures written by a texture oven modifier ({worlds} smallest worlds)")
    combos = collections.Counter(); bakers = collections.Counter(); preset = collections.defaultdict(collections.Counter)
    sizes = collections.defaultdict(collections.Counter)
    for name, d in sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))[:worlds]:
        r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects(); extra, _ = dec.extra_objects(edob)
        for o in objs.values():
            if not (isinstance(o, Obj) and o.type == "CTextureOvenShaderModifier"): continue
            tex = None
            for k, v in o:
                if isinstance(v, Obj):   # `.4` embeds a CShaderArgValTexture whose member holds the reference
                    for _, t in v:
                        if isinstance(t, Ref) and t.oid in objs and objs[t.oid].type == "CStaticTexture": tex = objs[t.oid]
            baker = o.get("CTextureOvenShaderModifier.6")
            if tex is None or not (isinstance(baker, Ref) and baker.oid in extra): continue
            fl = tex.get("CBaseTexture.4").v; fmt = tex.get("CBaseTexture.6").v; wrap = tex.get("CBaseTexture.9").v
            combos[(hex(fl), fmt, hex(wrap))] += 1; bakers[(hex(fl), extra[baker.oid].type)] += 1
            sizes[hex(fl)][max(tex.get("CBaseTexture.1"), tex.get("CBaseTexture.2"))] += 1
            for k in ("10", "11", "12"):
                v = extra[baker.oid].get("CTextureBakerLightMap." + k)
                preset["with 0x40" if fl & 0x40 else "without 0x40"][(k, getattr(v, "v", v))] += 1
    emit("  (flags, format code, wrap U) -> textures:")
    for k, v in sorted(combos.items()): emit(f"    {k}: {v}")
    emit(f"  baker record class by flags: {dict(sorted(bakers.items()))}")
    emit("  longest side by flags: " + str({k: dict(sorted(v.items())) for k, v in sorted(sizes.items())}))
    for group, c in sorted(preset.items()):
        emit(f"  baker members .10/.11/.12, textures {group}: " + str(dict(sorted(c.items(), key=lambda kv: -kv[1])[:9])))

    text = "\n".join(lines)
    if out:
        body = "# `.tex` flags and wrap modes: measured tables (T01-054)\n\nGenerated by `tools/re/texflags_verify.py`; see `tex-flags.md` for what the numbers mean. Do not edit by hand.\n\n```text\n" + text + "\n```\n"
        open(out, "w", encoding="utf-8").write(body)


if __name__ == "__main__":
    main()
