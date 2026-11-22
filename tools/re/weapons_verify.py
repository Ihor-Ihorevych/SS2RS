"""Re-runs the measurements behind docs/re/weapons.md on an install (read-only).

For the 16 weapons (`Databases/EntityParams/<weapon>weapon.ep` and the deathmatch variant `<weapon>weapon_dm.ep`):
the class, the 22 base members, the named members, the model, the tools, the sounds and the projectile file; the
single-player versus deathmatch differences; and the members of every projectile parameter file a weapon refers to.
Writes the tables with --table.

Usage: python -I tools/re/weapons_verify.py [DATA_DIR] [--table OUT.md]
       (DATA_DIR defaults to $SS2_DATA_DIR; about 1 minute)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Blob, Enum, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402

NULL = 0xFFFFFFFF


def load(d):
    r = parse(d); objs, _ = TreeDec(d, r).objects()
    rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}
    rs = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CResourceFile")
    return r, objs, ex, objs[[x.oid for x in rs.get("CResourceFile.rf_crsShared")][0]]


def flat(o, objs, ex, idnt, prefix=""):
    out = []
    for k, v in o:
        if k.startswith(("CResource.", "CResourceFile.")): continue
        cls, _, mem = k.partition(".")
        # a numbered member is prefixed by its class level so that equal numbers of different classes stay apart:
        # b = SBaseWeaponParams, s = SBaseScatterWeaponParams, p = SBaseProjectileParams, w = the weapon's own class
        name = prefix + ((({"SBaseWeaponParams": "b", "SBaseScatterWeaponParams": "s", "SBaseProjectileParams": "p"}.get(cls, "w")) + "." + mem) if mem.isdigit() else mem)
        if isinstance(v, Ref): out.append((name, "-" if v.oid == NULL else (ex[v.oid].rsplit("/", 1)[-1] if v.oid in ex else "(embedded " + (objs[v.oid].type if v.oid in objs else "?") + ")")))
        elif isinstance(v, Obj):
            if v.type in ("Vector2f", "Vector3f"): out.append((name, tuple(round(x, 4) for _, x in v)))
            else: out += flat(v, objs, ex, idnt, name + ".")
        elif isinstance(v, list) and not isinstance(v, Blob): out.append((name, f"list[{len(v)}]"))
        elif isinstance(v, Enum): out.append((name, "enum " + str(v.v)))
        elif isinstance(v, float): out.append((name, round(v, 4)))
        elif isinstance(v, (int, str)): out.append((name, v))
    return out


def main():
    args = sys.argv[1:]; table = args[args.index("--table") + 1] if "--table" in args else None
    if table: args = [a for a in args if a not in ("--table", table)]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    eps = merged_files(install, ".ep"); by = {n.split("databases/")[-1]: d for n, d in eps.items()}
    weapons = {}; proj = {}
    for rel in sorted(by):
        if not rel.startswith("entityparams/") or "items/" in rel or "rendering" in rel or "copy" in rel: continue
        if not (rel.endswith("weapon.ep") or rel.endswith("weapon_dm.ep")): continue
        r, objs, ex, rt = load(by[rel]); fl = flat(rt, objs, ex, r["idnt"])
        key = rel.rsplit("/", 1)[-1].replace("weapon_dm.ep", "").replace("weapon.ep", ""); dm = rel.endswith("_dm.ep")
        weapons.setdefault(key, {})["dm" if dm else "sp"] = (rt.type, dict(fl), fl)
    print(f"weapons: {len(weapons)}; with a deathmatch variant: {sum(1 for w in weapons.values() if 'dm' in w and 'sp' in w)}")
    pj = collections.Counter(); projfiles = {}
    for key, w in sorted(weapons.items()):
        for v in ("sp", "dm"):
            if v not in w: continue
            p = w[v][1].get("b.23")
            if isinstance(p, str) and p != "-": pj[p] += 1; projfiles[p] = True
    print("projectile files referenced by the weapons:", len(pj), dict(pj))
    prows = {}
    for p in sorted(projfiles):
        rel = next((n for n in by if n.endswith("/" + p.lower()) or n == p.lower()), None)
        if rel is None: print("  not found:", p); continue
        r, objs, ex, rt = load(by[rel]); prows[p] = (rel, rt.type, flat(rt, objs, ex, r["idnt"]))
    # differences sp vs dm
    diffs = {}
    for key, w in sorted(weapons.items()):
        if "sp" in w and "dm" in w:
            a, b = w["sp"][1], w["dm"][1]; diffs[key] = [(k, a.get(k), b.get(k)) for k in sorted(set(a) | set(b), key=str) if a.get(k) != b.get(k)]
    print("members that differ between the single-player and the deathmatch file:", {k: len(v) for k, v in diffs.items()})
    print("classes:", {k: w["sp"][0] for k, w in weapons.items() if "sp" in w})
    named = collections.Counter();
    for key, w in weapons.items():
        for n in w.get("sp", (0, {}))[1]:
            if "." not in n: named[n] += 1
    print("named members across weapons (count of weapons):", dict(named.most_common(40)))
    # base member value table across weapons
    for m in sorted((f"{c}.{i}" for c in "bsw" for i in range(1, 40)), key=lambda x: (x[0], int(x[2:]))):
        vals = {k: w["sp"][1].get(m) for k, w in weapons.items() if "sp" in w and m in w["sp"][1]}
        if vals and all(isinstance(v, (int, float)) for v in vals.values()): print(f" {m}: n={len(vals)} min={min(vals.values())} max={max(vals.values())} distinct={len(set(vals.values()))}")
    if table:
        L = ["# Weapon parameters (generated)", "",
             "Generated by `tools/re/weapons_verify.py --table` from the `<weapon>weapon.ep` and `<weapon>weapon_dm.ep` files of the retail install. See `weapons.md`. "
             "Members are by name when the type table has one, otherwise by number with the class level as a prefix (`b.30` = member 30 of `SBaseWeaponParams`, `s.1` of `SBaseScatterWeaponParams`, `w.1` of the weapon's own class, `p.2` of `SBaseProjectileParams`). `-` is a null reference. The deathmatch file is shown only where it differs.", ""]
        for key, w in sorted(weapons.items()):
            if "sp" not in w: continue
            cls, d, fl = w["sp"]
            L += [f"## {key} (`{cls}`)", "", "| Member | Single player | Deathmatch |", "|---|---|---|"]
            dmd = w.get("dm", (None, {}, []))[1]
            for k, v in fl:
                dv = dmd.get(k, v)
                L.append(f"| `{k}` | {v} | {'' if dv == v else '**' + str(dv) + '**'} |")
            L.append("")
        L += ["# Projectile parameter files referenced by the weapons", ""]
        for p, (rel, cls, fl) in sorted(prows.items()):
            L += [f"## `{p}` (`{cls}`)", "", "| Member | Value |", "|---|---|"] + [f"| `{k}` | {v} |" for k, v in fl] + [""]
        with open(table, "w", encoding="utf-8", newline="\n") as f: f.write("\n".join(L) + "\n")
        print("wrote", table)


if __name__ == "__main__":
    main()
