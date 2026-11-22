"""Re-runs the measurements behind docs/re/fx.md on an install (read-only).

For every format-8 `.lfx` (flare configurations), `.tfx` (text effects) and `.vfx` (Xbox vibration
effects): the object classes, the member ranges, the graph quadruples of the text effects and, with
`--refs`, which files refer to them (reads every .ep/.wld/.rsc/.mdl/.pfx/.ctl; a few minutes).

Usage: python -I tools/re/fx_verify.py [DATA_DIR] [--refs]
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, merged_files, parse, u32  # noqa: E402


def load(d):
    r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects()
    return r, objs, edob


def members(o, cls):
    return {k.split(".", 1)[1]: v for k, v in o if k.startswith(cls + ".")}


def rng(values):
    values = list(values)
    return (min(values), max(values)) if values else None


def lfx(install, st, dist):
    files = merged_files(install, ".lfx")
    parts_all = []
    for name, d in sorted(files.items()):
        r, objs, edob = load(d); st["lfx files"] += 1
        classes = sorted(o.type for o in objs.values() if isinstance(o, Obj))
        st["lfx classes are exactly CResourceFile, CFlareConfiguration"] += classes == ["CFlareConfiguration", "CResourceFile"]
        st["lfx have no extra records after EDOB"] += u32(d, edob + 4) == 0
        cfg = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CFlareConfiguration")
        parts = cfg.get("CFlareConfiguration.1"); dist["lfx parts per file"][len(parts)] += 1
        textures = {p[0][1].oid for p in parts}
        st["lfx part texture pointers are RFIL entries (ids not in OBTY)"] += all(t not in objs for t in textures)
        st["lfx: number of RFIL entries == number of distinct texture pointers"] += len(r["rfil"]) == len(textures)
        st["lfx RFIL paths are all .tex"] += all(e[2].lower().endswith(".tex") for e in r["rfil"])
        for p in parts:
            m = {k.split(".")[1]: v for k, v in p}; parts_all.append(m)
    n = len(parts_all)
    st["lfx parts in all"] = n
    dist["lfx colour alpha byte (.10 >> 24)"].update(m["10"] >> 24 for m in parts_all)
    dist["lfx flags (.9)"].update(m["9"].v for m in parts_all)
    for k, label in (("2", "offset"), ("3", "size"), ("4", "aspect"), ("5", "amplification"), ("6", "desaturation"), ("7", "falloff"), ("8", "rotation")):
        dist[f"lfx part member .{k} ({label}) range"][tuple(round(x, 3) for x in rng(m[k] for m in parts_all))] += 1
    dist["lfx rotation (.8) values"].update(round(m["8"], 1) for m in parts_all)
    dist["lfx part member .5 == 1"][sum(1 for m in parts_all if m["5"] == 1.0)] += 1
    dist["lfx part member .6 == 0.5"][sum(1 for m in parts_all if m["6"] == 0.5)] += 1


def tfx(install, st, dist):
    files = merged_files(install, ".tfx")
    for name, d in sorted(files.items()):
        r, objs, edob = load(d); st["tfx files"] += 1
        cfg = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CTextEffectConfiguration")
        st["tfx have no extra records after EDOB"] += u32(d, edob + 4) == 0
        cm = members(cfg, "CTextEffectConfiguration")
        dist["tfx effect lists (.1) sizes"][len(cm["1"])] += 1; dist["tfx letter-effect lists (.2) sizes"][len(cm["2"])] += 1
        dist["tfx timing members (.3, .4, .5)"][(round(cm["3"], 3), round(cm["4"], 3), round(cm["5"], 3))] += 1
        dist["tfx font reference is an RFIL path"][r["rfil"][0][2].rsplit("/", 1)[-1]] += 1
        dist["tfx CFontInstance flags (.3)"][members(cm["7"], "CFontInstance")["3"].v] += 1
        dist["tfx alignment H/V"][(cm["8"].v, cm["9"].v)] += 1; dist["tfx blend type (.10)"][cm["10"].v] += 1
        for ref in cm["1"] + cm["2"]:
            e = objs[ref.oid]; base = "CTextEffect" if e.type in ("CBlendTextEffect", "CMoveTextEffect", "CStretchTextEffect") else "CLetterEffect"
            g = tuple(objs[members(e, base)[k].oid].type for k in "1234")
            dist[f"tfx {e.type}: classes of the four graphs (.1-.4)"][g] += 1
            dist[f"tfx {e.type}: enabled flag"][members(e, base)["5" if base == "CTextEffect" else "6"]] += 1
        for o in objs.values():
            if isinstance(o, Obj) and o.type.endswith("Graph"):
                gm = members(o, "CGraph"); dist["tfx graph .7/.8 (all graph classes)"][(gm["7"], round(gm["8"], 2))] += 1
                pts = members(o, o.type)
                key = "1" if o.type != "CLinearGraph" else "2"
                if o.type != "CValueGraph":
                    xs = [members(p, "CControlPoint")["3"] for p in pts[key]]
                    dist[f"tfx {o.type} point count"][len(xs)] += 1


def vfx(install, st, dist):
    for name, d in sorted(merged_files(install, ".vfx").items()):
        r, objs, edob = load(d); st["vfx files"] += 1
        fx = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CXboxVibroFX")
        m = members(fx, "CXboxVibroFX")
        which = tuple(("graph" if isinstance(m[k], Ref) and m[k].oid != 0xFFFFFFFF else "none") for k in ("4", "5"))
        dist["vfx (.4 low?, .5 high?) graph present"][which] += 1
        dist["vfx duration member (.6)"][m["6"]] += 1
        st["vfx have no RFIL entries"] += len(r["rfil"]) == 0
        for k in ("4", "5"):
            if isinstance(m[k], Ref) and m[k].oid != 0xFFFFFFFF:
                g = objs[m[k].oid]; dist["vfx graph classes"][g.type] += 1


def refs(install, st, dist):
    want = (".lfx", ".tfx", ".vfx"); allfiles = {}
    for ext in (".ep", ".wld", ".rsc", ".mdl", ".pfx", ".ctl"):
        allfiles.update(merged_files(install, ext))
    targets = collections.defaultdict(set)
    for name, d in allfiles.items():
        for e in parse(d).get("rfil", []):
            p = e[2].lower()
            if p.endswith(want):
                dist["referenced from (ext of referrer, ext of target)"][(os.path.splitext(name)[1], p[-4:])] += 1; targets[p[-4:]].add(p)
    for k, v in sorted(targets.items()): dist["distinct targets referenced"][k] = len(v)


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter)
    lfx(install, st, dist); tfx(install, st, dist); vfx(install, st, dist)
    if "--refs" in sys.argv: refs(install, st, dist)
    for k, v in sorted(st.items()): print(f"{v:6d}  {k}")
    for k, v in sorted(dist.items()): print(k, dict(v.most_common(8)))


if __name__ == "__main__":
    main()
