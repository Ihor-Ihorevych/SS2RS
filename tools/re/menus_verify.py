"""Re-runs the measurements behind docs/re/gui.md on an install (read-only).

For every menu screen layout (`Content/SeriousSam2/Menus/*.rsc`, 99 files): the screen class, its underlay and overlay,
the gadgets (class, name, platform mask, text key and default text), the link gadgets and the screens they open, the
button bars and their controller buttons; the menu graph (which screens open which), screens that no link opens, the
fonts used. Writes the per-screen table with --table.

Usage: python -I tools/re/menus_verify.py [DATA_DIR] [--table OUT.md]
       (DATA_DIR defaults to $SS2_DATA_DIR; about 1 minute)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Blob, Enum, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402

NULL = 0xFFFFFFFF


def trans(o):
    """(key, default text) of a CTransString object."""
    if isinstance(o, Obj) and o.type == "CTransString": return (o.get("CTransString.1"), o.get("CTransString.2"))
    return (None, None)


def main():
    args = sys.argv[1:]; table = args[args.index("--table") + 1] if "--table" in args else None
    if table: args = [a for a in args if a not in ("--table", table)]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    screens = {}; gclass = collections.Counter(); masks = collections.Counter(); fonts = collections.Counter(); btn_types = collections.Counter()
    for n, d in sorted(merged_files(install, ".rsc").items()):
        if "/menus/" not in n.lower(): continue
        r = parse(d); objs, _ = TreeDec(d, r).objects(); idnt = r["idnt"]
        rs = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CResourceFile")
        rt = objs[[x.oid for x in rs.get("CResourceFile.rf_crsShared")][0]]
        if rt.type != "CMenuScreenLayout": continue
        cls = idnt[rt.get("CMenuScreenLayout.1")]; under = rt.get("CMenuScreenLayout.3"); over = rt.get("CMenuScreenLayout.4")
        gad = []; links = []; bars = []
        for g in rt.get("CMenuScreenLayout.2"):
            go = objs[g.oid] if isinstance(g, Ref) else g
            if not isinstance(go, Obj): continue
            nm = idnt[go.get("CMenuGadgetLayout.1")]; mask = go.get("CMenuGadgetLayout.7"); mask = mask.v if isinstance(mask, Enum) else mask
            txt = next((v for k, v in go if k.endswith("CMenuGadgetLayout.4") and isinstance(v, Obj)), None)
            key, default = trans(next((v for k, v in txt if k.endswith("CMenuText.1")), None)) if txt is not None else (None, None)
            font = idnt[txt.get("CMenuText.2")] if txt is not None else ""
            gclass[go.type] += 1; masks[mask] += 1
            if font: fonts[font] += 1
            gad.append((go.type, nm, mask, key, default, font))
            if go.type == "CMenuGadgetLinkLayout":
                t = go.get("CMenuGadgetLinkLayout.1"); links.append((nm, idnt[t] if isinstance(t, int) else ""))
            if go.type == "CMenuGadgetButtonBarLayout":
                for lst in ("CMenuGadgetButtonBarLayout.2", "CMenuGadgetButtonBarLayout.3"):
                    items = []
                    for b in go.get(lst):
                        bo = objs[b.oid] if isinstance(b, Ref) else b
                        t = bo.get("CMenuControllerButton.1"); t = t.v if isinstance(t, Enum) else t
                        k, dflt = trans(bo.get("CMenuControllerButton.2")); items.append((t, k, dflt)); btn_types[t] += 1
                    bars.append((lst[-1], items))
        screens[n.rsplit("/", 1)[-1][:-4]] = dict(cls=cls, under=under, over=over, gad=gad, links=links, bars=bars)
    print("menu screens:", len(screens), "| gadgets:", sum(len(s["gad"]) for s in screens.values()))
    print("gadget classes:", dict(gclass.most_common(30)))
    print("platform masks:", dict(masks), "| fonts:", dict(fonts.most_common(8)))
    print("underlays:", dict(collections.Counter(s["under"] for s in screens.values())), "| overlays:", dict(collections.Counter(s["over"] for s in screens.values())))
    byclass = {s["cls"]: f for f, s in screens.items()}
    edges = [(s["cls"], t, nm) for f, s in screens.items() for nm, t in s["links"]]
    stems = {f.lower(): s["cls"] for f, s in screens.items()}
    def resolve(t): return t if t in byclass else stems.get(t.lower())
    print("link gadgets:", len(edges), "| distinct targets:", len({t for _, t, _ in edges}), "| resolved by screen class:", len({t for _, t, _ in edges if t in byclass}), "| resolved by file name only:", len({t for _, t, _ in edges if t not in byclass and t.lower() in stems}), "| not found:", sorted({t for _, t, _ in edges if resolve(t) is None}))
    tgt = {resolve(t) for _, t, _ in edges}
    print("screens that no link opens:", len([c for c in byclass if c not in tgt]), sorted(c for c in byclass if c not in tgt)[:70])
    for m in (1, 2):
        print(f"gadgets for platform mask {m} only:", sorted({(f.lower() if False else g[1]) for f, s in screens.items() for g in s["gad"] if g[2] == m})[:40])
    print("button types (EMenuButtonType):", dict(sorted(btn_types.items(), key=lambda kv: str(kv[0]))))
    print("button bars:", sum(len(s["bars"]) for s in screens.values()))
    print("screens with most gadgets:", sorted(((len(s["gad"]), f) for f, s in screens.items()), reverse=True)[:6])
    print("outdegree:", sorted(((len(s["links"]), s["cls"]) for s in screens.values()), reverse=True)[:6])
    if table:
        L = ["# Menu screens (generated)", "",
             "Generated by `tools/re/menus_verify.py --table` from the 99 `Menus/*.rsc` layouts of the retail install. See `gui.md`. "
             "Columns: the file, the screen class (`CMenuScreenLayout.1`), underlay and overlay screens, the number of gadgets, the screens the link gadgets open (gadget name → screen class), and the gadget kinds.", "",
             "| File | Screen class | Underlay | Overlay | Gadgets | Links (gadget → screen) | Kinds |", "|---|---|---|---|---|---|---|"]
        for f, s in sorted(screens.items()):
            kinds = collections.Counter(g[0].replace("CMenuGadget", "").replace("CMG", "").replace("Layout", "") for g in s["gad"])
            L.append(f"| `{f}` | `{s['cls']}` | {s['under']} | {s['over']} | {len(s['gad'])} | " + ", ".join(f"{nm} → `{t}`" for nm, t in s["links"]) + " | " + ", ".join(f"{k} {v}" for k, v in kinds.most_common()) + " |")
        L += ["", "## Link graph (screen → opened screens)", ""]
        for c in sorted({e[0] for e in edges}):
            L.append(f"- `{c}` → " + ", ".join(f"`{t}`" for _, t, _ in sorted(e for e in edges if e[0] == c)))
        with open(table, "w", encoding="utf-8", newline="\n") as fh: fh.write("\n".join(L) + "\n")
        print("wrote", table)


if __name__ == "__main__":
    main()
