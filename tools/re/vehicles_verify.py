"""Re-runs the measurements behind docs/re/vehicles.md on an install (read-only).

The aircraft, turret, hovercraft and rolling-ball puppet parameter files (`.ep`): class-specific movement
parameters, camera names, seats, ride icons and mounted tools; and how the worlds use them (entities per file,
per level). Writes the per-file table with --table.

Usage: python -I tools/re/vehicles_verify.py [DATA_DIR] [LIMIT] [--table OUT.md]
       (DATA_DIR defaults to $SS2_DATA_DIR; LIMIT = only the LIMIT smallest worlds, default all, about 15 minutes)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Blob, Enum, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
from wld_root_verify import ids  # noqa: E402

NULL = 0xFFFFFFFF
ROOTS = {"SAircraftPuppetParams": ("SAircraftCharacterEntity", ["app_fMaxFrontVel", "app_fMaxUpVel", "app_fMaxRightVel", "app_fAngVelH", "app_fFrontAcceleration"]),
         "STurretPuppetParams": ("STurretCharacterEntity", ["STurretPuppetParams.1", "STurretPuppetParams.tpp_fGunAcceleration", "STurretPuppetParams.tpp_fGunDecceleration"]),
         "SHovercraftPuppetParams": ("SHovercraftCharacterEntity", ["hpp_fMaxVelForward", "hpp_fMaxVelSide", "hpp_fMaxJumpVelocity", "hpp_fSpringCoef"]),
         "SRollingBallPuppetParams": ("SRollingBallCharacterEntity", ["rbpp_fMaxLinearVelocity", "rbpp_fLinearAccel", "rbpp_fMaxJumpVelocity"])}


def walk(o, f):
    for k, v in o:
        f(k, v)
        if isinstance(v, Obj): walk(v, f)
        elif isinstance(v, list) and not isinstance(v, Blob):
            for x in v:
                if isinstance(x, Obj): walk(x, f)


def collect(objs, root, ex, idnt):
    got = {}; tools = []; mounts = []
    def f(k, v):
        short = k.split(".", 1)[-1] if not k.startswith(("STurretPuppetParams", "SPuppetHealthParams")) else k
        if isinstance(v, Ref):
            return
        if isinstance(v, (int, float)) and not isinstance(v, bool): got.setdefault(short if short in sum([w[1] for w in ROOTS.values()], []) or k in sum([w[1] for w in ROOTS.values()], []) else k, v)
        elif isinstance(v, Enum): got.setdefault(k, v.v)
    walk(root, f)
    for k, v in root:
        if k.endswith("pp_amtMountedTools"):
            for t in v:
                nm = idnt[t.get("CMountedTool.mt_idTool")]; ref = t.get("CMountedTool.mt_pctTool")
                tools.append((nm, ex.get(ref.oid, "").rsplit("/", 1)[-1] if isinstance(ref, Ref) else ""))
    def g(k, v):
        if k.endswith(("pp_id3rdPersonRideCamera", "pp_id3rdPersonCamera", "pp_id3rdPersonDeathCamera")): got["cam:" + k.split("pp_id")[-1]] = idnt[v]
        if k.endswith("op_idOperatorSeat"): got["seat"] = idnt[v]
        if k.endswith("mpp_ptxRideIcon"): got["ride icon"] = isinstance(v, Ref) and v.oid != NULL
        if k.endswith("SPuppetHealthParams.1") and isinstance(v, (int, float)): got["health"] = v
    walk(root, g)
    for k, v in root:
        if isinstance(v, Obj) and v.type == "SPuppetHealthParams": got["health"] = [x for _, x in v][0]
    return got, tools


def main():
    args = sys.argv[1:]; table = args[args.index("--table") + 1] if "--table" in args else None
    if table: args = [a for a in args if a not in ("--table", table)]
    install = args[0] if args and not args[0].isdigit() else os.environ.get("SS2_DATA_DIR", "")
    limit = int(args[-1]) if args and args[-1].isdigit() else None
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    rows = {k: [] for k in ROOTS}; byfile = {}
    for name, d in sorted(merged_files(install, ".ep").items()):
        r = parse(d); objs, _ = TreeDec(d, r).objects(); rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}
        rt = objs[[x.oid for x in objs[next(i for i, o in objs.items() if isinstance(o, Obj) and o.type == "CResourceFile")].get("CResourceFile.rf_crsShared")][0]]
        if rt.type in ROOTS:
            got, tools = collect(objs, rt, ex, r["idnt"]); rel = name.split("databases/")[-1]
            rows[rt.type].append((rel, got, tools)); byfile[name.lower()] = (rt.type, rel)
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter)
    for k, rs in rows.items():
        print(f"{k}: {len(rs)} files; ride icon set in {sum(1 for _, g, _ in rs if g.get('ride icon'))}; seat names {collections.Counter(g.get('seat') for _, g, _ in rs).most_common(3)}; cameras {collections.Counter(g.get('cam:3rdPersonRideCamera') for _, g, _ in rs).most_common(4)}")
    use = collections.defaultdict(collections.Counter); perworld = {}
    files = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))
    for n, (name, d) in enumerate(files):
        if limit and n >= limit: break
        r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects(); rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}
        w = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CWorld"); wn = name.rsplit("/", 1)[-1][:-4]; c = collections.Counter()
        for i in ids(w.get("CWorld.1")):
            e = objs[i]
            for rootcls, (ecls, _) in ROOTS.items():
                if e.type == ecls:
                    c[ecls] += 1; ref = e.get("SPuppetEntity.en_pppPuppetParams"); p = ex.get(ref.oid, "").lower() if isinstance(ref, Ref) else ""
                    use[p][wn] += 1; st["entities"] += 1; st["... whose .ep is one of the files above"] += p in byfile and byfile[p][0] == rootcls
        perworld[wn] = c; print(f"[{n + 1}/{len(files)}] {wn}", file=sys.stderr, flush=True)
    for k, v in sorted(st.items()): print(f"{v:8d}  {k}")
    print("worlds with vehicles/turrets:", sum(1 for c in perworld.values() if c), "of", len(perworld))
    if table:
        L = ["# Vehicle, turret, hovercraft and rolling-ball parameters (generated)", "",
             "Generated by `tools/re/vehicles_verify.py --table` from the `.ep` files whose root class is `SAircraftPuppetParams`, `STurretPuppetParams`, `SHovercraftPuppetParams` or `SRollingBallPuppetParams` and from the placed entities of every world. "
             "Columns: health (`SPuppetHealthParams.1`), whether the file has a ride icon (the player can ride it), the operator seat, the ride camera, the class-specific numbers named in the heading and the number of entities and worlds that use the file. See `vehicles.md`.", ""]
        for k, rs in rows.items():
            keys = ROOTS[k][1]
            L += [f"## `{k}` ({len(rs)} files; entity class `{ROOTS[k][0]}`)", "", "| File | Health | Ride | Seat | Ride camera | " + " | ".join(f"`{x.split('.')[-1] if not x.split('.')[-1].isdigit() else 'turret .' + x.split('.')[-1]}`" for x in keys) + " | Entities / worlds |", "|---|---|---|---|---|" + "---|" * len(keys) + "---|"]
            for rel, g, tools in sorted(rs):
                path = ("content/serioussam2/databases/" + rel).lower(); u = use.get(path, collections.Counter())
                vals = " | ".join(f"{g.get(x, '-'):g}" if isinstance(g.get(x, '-'), (int, float)) else "-" for x in keys)
                L.append(f"| `{rel}` | {g.get('health', '-')} | {'yes' if g.get('ride icon') else ''} | {g.get('seat', '') or ''} | {g.get('cam:3rdPersonRideCamera', '') or ''} | {vals} | " + f"{sum(u.values())} / {len(u)} |")
            L.append("")
        with open(table, "w", encoding="utf-8", newline="\n") as f: f.write("\n".join(L) + "\n")
        print("wrote", table)


if __name__ == "__main__":
    main()
