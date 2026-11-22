"""Re-runs the measurements behind docs/re/fnt.md on an install (read-only).

For every format-8 `.fnt`: the object classes, the atlas texture, the glyph rectangles, the code table,
the relations of the header members, the covered code ranges and the extra records.

Usage: python -I tools/re/fnt_verify.py [DATA_DIR] [--png OUT_DIR]
       (--png renders the sample string "Hello, Serious Sam 2!" with every font into OUT_DIR)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, TreeDec, merged_files, parse, u32  # noqa: E402


def ranges(codes):
    out = []; start = prev = None
    for c in sorted(codes):
        if start is None: start = prev = c
        elif c == prev + 1: prev = c
        else: out.append((start, prev)); start = prev = c
    if start is not None: out.append((start, prev))
    return out


def render(font, objs, text, path):
    from texpng import decode_static_texture, write_png
    W, H, atlas = decode_static_texture(objs[font.get("CFont.5").oid]); C = [[v for _, v in c] for c in font.get("CFont.6")]; mp = font.get("CFont.7"); lh = font.get("CFont.3")
    glyph = lambda ch: C[mp[ord(ch)] if ord(ch) < len(mp) else 0]
    width = sum(glyph(ch)[6] for ch in text) + 12; img = bytearray([0, 0, 0, 255] * (width * (lh + 8))); x = 4
    for ch in text:
        gx, gy, gw, gh, ox, oy, adv, _ = glyph(ch)
        for yy in range(gh):
            for xx in range(gw):
                a = atlas[((gy + yy) * W + gx + xx) * 4 + 3]; px = ((4 + yy + oy) * width + x + xx + ox) * 4
                if 0 <= px < len(img) and a: img[px:px + 3] = bytes((a, a, a))
        x += adv
    write_png(path, width, lh + 8, img)


def main():
    args = sys.argv[1:]; install = next((a for a in args if os.path.isdir(a) and a != (args[args.index("--png") + 1] if "--png" in args else None)), os.environ.get("SS2_DATA_DIR", ""))
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    png = args[args.index("--png") + 1] if "--png" in args else None
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter); extra_cls = collections.Counter(); cover = {}
    for name, d in sorted(merged_files(install, ".fnt").items()):
        r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects(); st["fonts"] += 1
        st["classes are exactly CResourceFile, CFont, CStaticTexture"] += sorted(o.type for o in objs.values() if isinstance(o, Obj)) == ["CFont", "CResourceFile", "CStaticTexture"]
        f = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CFont"); tex = objs[f.get("CFont.5").oid]
        W, H = tex.get("CBaseTexture.1"), tex.get("CBaseTexture.2"); C = [[v for _, v in c] for c in f.get("CFont.6")]; mp = f.get("CFont.7"); n = len(C)
        st["glyph records have 8 members"] += all(len(c) == 8 for c in C)
        st["glyph rectangles (.1 x, .2 y, .3 w, .4 h) lie inside the atlas"] += all(c[0] >= 0 and c[1] >= 0 and c[0] + c[2] <= W and c[1] + c[3] <= H for c in C)
        nz = [v for v in mp if v]
        st["code table values are glyph indices below the glyph count"] += all(0 <= v < n for v in mp)
        st["non-zero table values are distinct and cover glyphs 1..n-1 (glyph 0 is the fallback)"] += sorted(nz) == list(range(1, n))
        st["CFont.4 == atlas width"] += f.get("CFont.4") == W
        st["every glyph height (.4) == CFont.3"] += all(c[3] == f.get("CFont.3") for c in C)
        st["every advance (.7) == CFont.2 (monospaced font)"] += all(c[6] == f.get("CFont.2") for c in C)
        st["glyph members .6 and .8 are 0 in every glyph"] += all(c[5] == 0 and c[7] == 0 for c in C)
        dist["x offset (.5) range"][(min(c[4] for c in C), max(c[4] for c in C))] += 1
        dist["atlas format code"][tex.get("CBaseTexture.6").v] += 1; dist["atlas TexFlags"][hex(tex.get("CBaseTexture.4").v)] += 1
        dist["code table length"][len(mp)] += 1; dist["glyph count"][n] += 1
        cover[name.rsplit("/", 1)[-1] + " (" + str(len(mp)) + ")"] = ranges([i for i, v in enumerate(mp) if v])
        st["fonts with extra records after EDOB"] += bool(u32(d, edob + 4))
        if u32(d, edob + 4):
            extra, end = dec.extra_objects(edob); st["... extra records end on METAEND"] += d[end:] == b"METAEND "
            extra_cls.update(o.type for o in extra.values() if isinstance(o, Obj))
        if png:
            os.makedirs(png, exist_ok=True); render(f, objs, "Hello, Serious Sam 2!", os.path.join(png, name.replace("/", "_") + ".png"))
        dist["folder"][name.rsplit("/", 1)[0]] += 1
    for k, v in sorted(st.items()): print(f"{v:6d}  {k}")
    for k, v in sorted(dist.items()): print(k, dict(v.most_common(8)))
    print("extra record classes:", dict(extra_cls))
    seen = set()
    for k, v in cover.items():
        key = tuple(v)
        if key in seen: continue
        seen.add(key); print("covered code ranges", k, [(hex(a), hex(b)) for a, b in v][:14], "..." if len(v) > 14 else "")


if __name__ == "__main__":
    main()
