"""Evidence for docs/re/behavior-mover-steering.md (T01-154): the stored numbers of the steering movers.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries).

It prints, for the movers whose steering the spec describes (`CMCCircleAroundFoe`, `CMCStayInArea`, `CMCPatternMoving`, `CMCDiamond`,
`CMCMellon`, `CMCAirAttack`) in the `.cb`, `.ep` and `.wld` files:
  1. where each class is stored (files and objects per extension) and which of its members matter for the band-stored and for the
     tactic-stored copies (the ranges of `CMCCircleAroundFoe` and `CMCStayInArea` are read only for tactic movers);
  2. the values of every member (distinct values with counts);
  3. the graphs of `CMCPatternMoving` and `CMCAirAttack`: class, scale members, control points, and their values at sample inputs.

`.wld` files are slow to read: pass `--no-wld` to skip them (the `.cb` and `.ep` counts do not need them).

Usage: python -I tools/re/mover_steering_verify.py [DATA_DIR] [--no-wld]      (about 5 minutes with the worlds, 15 seconds without)
"""
import collections, math, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Enum, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402

NULL = 0xFFFFFFFF
WANTED = ["CCharacterBehavior", "CMCCircleAroundFoe", "CMCStayInArea", "CMCPatternMoving", "CMCDiamond", "CMCMellon", "CMCAirAttack"]
GRAPH_MEMBERS = {"CCharacterBehavior": ["cb_pgrAltitudeMin", "cb_pgrAltitudeMax", "cb_pgrThinkPeriodOveride"],
                 "CMCPatternMoving": ["pm_pgrHAngleMin", "pm_pgrHAngleMax", "pm_pgrPatternMultiplier"],
                 "CMCAirAttack": ["mc_pgrAltitude", "mc_pgrMaxAltitude", "mc_pgrHorizontalOffset", "mc_pgrMovingPeriodOverride"]}


def members(o, cls):
    return {k.split(".", 1)[1]: v for k, v in o if k.startswith(cls + ".")}


def points(objs, g):
    """Control points (x, y) of a graph object (list member of the graph class)."""
    pts = []
    for k, v in g:
        if k.split(".")[0] != g.type:
            continue
        if isinstance(v, Obj):                       # CValueGraph: one CControlPoint
            d = dict(dict(v).get("CControlPoint.3", []))
            if d:
                pts.append((round(d["Vector2f.1"], 3), round(d["Vector2f.2"], 3)))
        elif isinstance(v, list):
            for p in v:
                p = objs[p.oid] if isinstance(p, Ref) else p
                if isinstance(p, Obj):
                    xy = next((x for kk, x in p if kk.endswith(".3")), None)
                    if isinstance(xy, Obj):
                        d = dict(xy)
                        pts.append((round(d["Vector2f.1"], 3), round(d["Vector2f.2"], 3)))
    return pts


def value(cls, pts, x):
    """The graph value as the engine computes it for the stored graphs (scales are 1.0, checked below)."""
    if cls == "CValueGraph":
        return pts[0][1]
    if cls == "CSineGraph":
        (x0, y0), (x1, y1) = pts[0], pts[1]
        return (y1 - y0) * math.sin((x - x0) * 2 * math.pi / (x1 - x0)) + y0
    if cls in ("CStepUpGraph", "CStepDownGraph"):
        (x0, y0), (x1, y1) = pts[0], pts[1]
        return y0 if x < x0 else y1 if x >= x1 else (x - x0) * (y1 - y0) / (x1 - x0) + y0
    if cls == "CLinearGraph":
        if len(pts) == 1 or x < pts[0][0]:
            return pts[0][1]
        if x >= pts[-1][0]:
            return pts[-1][1]
        for (xa, ya), (xb, yb) in zip(pts, pts[1:]):
            if x <= xb:
                return (x - xb) * (ya - yb) / (xa - xb) + yb if xa != xb else (ya + yb) / 2
    return None


def main(install, with_wld):
    exts = [".cb", ".ep"] + ([".wld"] if with_wld else [])
    inst = collections.Counter()
    files = collections.defaultdict(set)
    vals = collections.defaultdict(lambda: collections.defaultdict(collections.Counter))
    graphs = collections.defaultdict(collections.Counter)
    scales = collections.Counter()
    for ext in exts:
        for name, d in sorted(merged_files(install, ext).items()):
            r = parse(d)
            if not ({t.name for t in r["types"].values()} & set(WANTED)):
                continue
            objs, _ = TreeDec(d, r).objects()
            idnt = r["idnt"]
            seen = set()

            def visit(o):
                if o.type in WANTED and id(o) not in seen:
                    seen.add(id(o))
                    inst[(o.type, ext)] += 1
                    files[(o.type, ext)].add(name)
                    m = members(o, o.type)
                    base = members(o, "SMoveCharacter")
                    for k, v in {**m, **base}.items():
                        if isinstance(v, Ref):
                            vals[o.type][k]["null" if v.oid == NULL else "ref"] += 1
                        elif isinstance(v, Enum):
                            vals[o.type][k]["enum %d" % v.v] += 1
                        elif isinstance(v, float):
                            vals[o.type][k][round(v, 4)] += 1
                        elif isinstance(v, (int, bool)):
                            vals[o.type][k][v] += 1
                    for gm in GRAPH_MEMBERS.get(o.type, []):
                        g = m.get(gm, base.get(gm))
                        if isinstance(g, Ref) and g.oid != NULL:
                            go = objs[g.oid]
                            cm = members(go, "CGraph")
                            scales[(round(cm.get("7", 0), 3), round(cm.get("8", 0), 3), round(cm.get("12", 0), 3) if go.type == "CValueGraph" else None)] += 1
                            pts = points(objs, go)
                            graphs[(o.type, gm)][(go.type, tuple(pts), round(cm.get('7', 0), 3))] += 1

            def walk(o):
                if isinstance(o, Obj):
                    visit(o)
                    for _, v in o:
                        walk(v)
                elif isinstance(o, list):
                    for x in o:
                        walk(x)
            for o in objs.values():
                walk(o)
    print("== 1. objects and files per extension")
    for (t, e), n in sorted(inst.items()):
        print(f"   {t:22s} {e:5s} {n:4d} objects in {len(files[(t, e)])} files")
    print("== 2. member values (all stored objects of the class; base members included)")
    for t in WANTED:
        print(f"   {t}")
        for k, c in sorted(vals[t].items()):
            if k in ("mc_tmStarted", "mc_vStarted", "mc_fRnd", "en_henBrain"):
                continue
            items = sorted(c.items(), key=lambda kv: -kv[1])[:8]
            print(f"      {k}: " + ", ".join(f"{a} x{b}" for a, b in items))
    print("== 3. graphs (class, control points: count of members), and values at sample inputs")
    for key, c in sorted(graphs.items()):
        print(f"   {key[0]}.{key[1]}")
        for (cls, pts, sc), n in sorted(c.items(), key=lambda kv: -kv[1]):
            samples = ""
            if cls in ("CSineGraph", "CStepUpGraph", "CStepDownGraph", "CValueGraph", "CLinearGraph") and pts:
                xs = [pts[0][0], (pts[0][0] + pts[-1][0]) / 2, pts[-1][0]] if len(pts) > 1 else [0.0]
                samples = "  values at x=" + ", ".join(f"{x:g}:{value(cls, pts, x):.3f}" for x in xs)
            print(f"      x{n}: {cls} scale(CGraph.7)={sc} {list(pts)}{samples}")
    print(f"   (CGraph.7, CGraph.8, CGraph.12 of a value graph) over all graphs above: {dict(scales)}")


if __name__ == "__main__":
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    inst = args[0] if args else os.environ.get("SS2_DATA_DIR")
    if not inst:
        sys.exit("usage: mover_steering_verify.py DATA_DIR [--no-wld]")
    main(inst, "--no-wld" not in sys.argv)
