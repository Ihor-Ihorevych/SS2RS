"""Re-runs the measurements behind docs/re/camera-view.md on an install (read-only).

1. `Databases/Cameras.rsc`: every camera (name, pitch range, the two distance graphs as control points).
2. The 16 weapon rendering parameter files (`EntityParams/*weaponrendering.ep`): where the weapon model sits in the
   first-person view for one and for two weapons (position, angle, field of view).
3. The camera names that every puppet parameter file uses, checked against the database.
4. The player's view parameters (`CGlobalPlayerParams`, `SPlayerPuppetParams`) and the sniper zoom parameters.
Writes the tables with --table.

Usage: python -I tools/re/camera_verify.py [DATA_DIR] [--table OUT.md]
       (DATA_DIR defaults to $SS2_DATA_DIR; about 3 minutes)
"""
import collections, math, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Blob, Enum, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402


def root_of(d):
    r = parse(d); objs, _ = TreeDec(d, r).objects()
    rs = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CResourceFile")
    return r, objs, objs[[x.oid for x in rs.get("CResourceFile.rf_crsShared")][0]]


def points(o, objs, out):
    """All Vector2f control points below a graph object, in order."""
    for k, v in o:
        if isinstance(v, Ref) and v.oid in objs: v = objs[v.oid]
        if isinstance(v, Obj):
            if v.type == "Vector2f": out.append((v.get("Vector2f.1"), v.get("Vector2f.2")))
            else: points(v, objs, out)
        elif isinstance(v, list) and not isinstance(v, Blob):
            for x in v:
                x = objs[x.oid] if isinstance(x, Ref) and x.oid in objs else x
                if isinstance(x, Obj):
                    if x.type == "Vector2f": out.append((x.get("Vector2f.1"), x.get("Vector2f.2")))
                    else: points(x, objs, out)


def nested(o, typ):
    for k, v in o:
        if isinstance(v, Obj):
            if v.type == typ: yield v
            yield from nested(v, typ)


def vec(v): return [v.get(f"Vector3f.{i}") for i in (1, 2, 3)]


def main():
    args = sys.argv[1:]; table = args[args.index("--table") + 1] if "--table" in args else None
    if table: args = [a for a in args if a not in ("--table", table)]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    out = {}
    # 1. cameras
    rsc = merged_files(install, ".rsc"); name = next(n for n in rsc if n.lower().endswith("databases/cameras.rsc"))
    r, objs, root = root_of(rsc[name]); cams = []
    for k, v in root:
        if k.endswith("cd_cchCameraHolders"):
            for h in v:
                cc = objs[objs[h.oid].get("CCameraHolder.ch_pccCameraControl").oid]; g = {}
                for key in ("cc_pgrBehindDistanceOverPitch", "cc_pgrAboveDistanceOverPitch"):
                    go = objs[cc.get("CCameraControl." + key).oid]; pts = []; points(go, objs, pts); g[key] = (go.type, pts, go.get("CGraph.12"))
                cams.append((r["idnt"][cc.get("CCameraControl.cc_idName")], cc.get("CCameraControl.cc_fMinPitch"), cc.get("CCameraControl.cc_fMaxPitch"), g))
    names = {c[0] for c in cams}
    print(f"cameras: {len(cams)}; distinct names {len(names)}; graph classes", dict(collections.Counter(g[0] for c in cams for g in c[3].values())))
    for c in cams: print("  ", c[0].ljust(28), c[1], c[2], c[3]["cc_pgrBehindDistanceOverPitch"][0], len(c[3]["cc_pgrBehindDistanceOverPitch"][1]), "pts;", c[3]["cc_pgrAboveDistanceOverPitch"][0], len(c[3]["cc_pgrAboveDistanceOverPitch"][1]), "pts")
    out["cams"] = cams
    # 2-4 over the .ep files
    weapons = []; used = collections.defaultdict(collections.Counter); player = {}; sniper = {}
    for n, d in sorted(merged_files(install, ".ep").items()):
        r, objs, rt = root_of(d); rel = n.split("databases/")[-1]
        for o in objs.values():
            if not isinstance(o, Obj) or o.type == "CResourceFile": continue
            if any(k.startswith("SBaseWeaponRenderingParams.") for k, _ in o):
                g = lambda m: o.get("SBaseWeaponRenderingParams." + m)
                weapons.append((rel.rsplit("/", 1)[-1], o.type, vec(g("1")), vec(g("2")), g("3"), g("5"), vec(g("bwrp_vPositionDoubleWeapon")), vec(g("bwrp_vAngleDoubleWeapon")), g("bwrp_fFOVDoubleWeapon"), g("4")))
            if o is rt:
                for cam in nested(o, "SCameras"):
                    for k, v in cam:
                        if k.endswith(("pp_id3rdPersonRideCamera", "pp_id3rdPersonCamera", "pp_id3rdPersonDeathCamera")): used[k.split("pp_id")[-1]][r["idnt"][v]] += 1
            if o.type == "SPlayerPuppetParams" and o is rt:
                player[rel] = {k.split(".", 1)[-1]: (v.v if isinstance(v, Enum) else v) for k, v in o if isinstance(v, (int, float)) and not isinstance(v, bool)}
            if o.type == "SSniperWeaponParams":
                sniper[rel] = {k.split(".", 1)[-1]: v for k, v in o if isinstance(v, (int, float)) and k.split(".", 1)[-1].startswith("swp_f")}
    print(f"\nweapon rendering files: {len(weapons)}")
    print("  .3 equals the double-weapon field of view in", sum(1 for w in weapons if abs(w[4] - w[8]) < 1e-6), "of", len(weapons), "| angle equals double angle in", sum(1 for w in weapons if w[3] == w[7]), "| .4 values", sorted({round(w[9], 3) for w in weapons}), "| .5 range", min(w[5] for w in weapons), max(w[5] for w in weapons))
    print("  .3 in degrees: %.1f .. %.1f" % (min(math.degrees(w[4]) for w in weapons), max(math.degrees(w[4]) for w in weapons)))
    for w in weapons: print("  ", w[0].ljust(34), [round(x, 3) for x in w[2]], [round(x, 1) for x in w[3]], round(w[4], 3), round(w[5], 2), "| double", [round(x, 3) for x in w[6]])
    print("\ncamera names used by puppet files (not in the database are marked *):")
    for slot, c in used.items(): print("  ", slot, {k + ("" if k in names else "*"): v for k, v in c.most_common()})
    print("\nplayer puppet parameter files:", len(player))
    for rel, v in player.items(): print("  ", rel, {k: round(x, 3) for k, x in v.items() if any(t in k for t in ("View", "Pitch", "Bank", "Height", "Fov", "FOV", "Center", "Damping"))})
    print("sniper:", sniper)
    out.update(weapons=weapons, used=used)
    if table:
        L = ["# Camera database and weapon view placement (generated)", "",
             "Generated by `tools/re/camera_verify.py --table` from `Databases/Cameras.rsc` and the 16 `*weaponrendering.ep` files. See `camera-view.md`. "
             "Graph points are (pitch in degrees, distance); a constant graph (`CValueGraph`) shows its value.", "",
             "## Cameras", "", "| Camera | Pitch range | Behind distance over pitch | Above distance over pitch |", "|---|---|---|---|"]
        def fmt(g):
            cls, pts, const = g
            return f"{cls[1:].replace('Graph', '')}: " + ("; ".join(f"({x:g}, {y:g})" for x, y in pts) if pts else f"{const:g}")
        for c in sorted(cams): L.append(f"| `{c[0]}` | {c[1]:g} .. {c[2]:g} | {fmt(c[3]['cc_pgrBehindDistanceOverPitch'])} | {fmt(c[3]['cc_pgrAboveDistanceOverPitch'])} |")
        L += ["", "## Weapon view placement (first person)", "", "Position (x, y, z) and angle (three values, degrees) of the weapon model for one weapon, the field of view (radians; `.3`, equal to the double-weapon value in all files that have one), the member `.5`, and the position for two weapons (the second hand).", "",
              "| File | Position | Angle | FOV (rad / deg) | `.5` | Position, two weapons |", "|---|---|---|---|---|---|"]
        for w in sorted(weapons): L.append(f"| `{w[0]}` | {', '.join(f'{x:g}' for x in w[2])} | {', '.join(f'{x:.1f}' for x in w[3])} | {w[4]:.3f} / {math.degrees(w[4]):.1f} | {w[5]:g} | {', '.join(f'{x:g}' for x in w[6])} |")
        with open(table, "w", encoding="utf-8", newline="\n") as f: f.write("\n".join(L) + "\n")
        print("wrote", table)


if __name__ == "__main__":
    main()
