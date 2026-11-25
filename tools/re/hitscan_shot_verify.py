"""Evidence for docs/re/hitscan-shot.md (T01-118): what the data says about the hitscan shot.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries).

It prints
  1. for the six hitscan weapons (Colt, Uzi, minigun, auto shotgun, double shotgun, sniper), single player and deathmatch: the nine
     members of `SBaseScatterWeaponParams` that the weapon copies into its owned shooter (bullet type, scatter distance, minimum and
     maximum scatter radius, pattern stretch, bullet velocity, range, impulse, up ratio) and the weapon's own members (bullets,
     damage, zoom damage), and a check that the other eleven weapon files do not derive from the scatter class;
  2. the scatter angles that follow from them (`atan(radius / distance)` in degrees, the model of section 4 of the spec);
  3. the bullet descriptions of the enemy attacks (`CBulletFireing`, member of `CSingleBlast`): how many files and objects, the
     value counts of every member, which bullet types occur and in which files (file names only);
  4. the `BulletType` and `DamageInflictorWeapon` values of the weapon files and of the stored shooters of the worlds.

Usage: python -I tools/re/hitscan_shot_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR; about 2 minutes)
"""
import collections, math, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, TreeDec, merged_files, parse  # noqa: E402
import weapons_verify as wv  # noqa: E402

HITSCAN = {"colt": "cwp_slBulletDamage", "uzi": "uwp_slBulletDamage", "minigun": "mgwp_slBulletDamage", "autoshotgun": None,
           "doubleshotgun": None, "sniper": "swp_slBulletDamage"}
# the nine members of SBaseScatterWeaponParams in the binary's order (weapon-classes.md section 4.2): data name or number -> binary name
SCATTER = [("s.5", "bswp_btBulletType"), ("s.1", "bswp_fScatterDistance"), ("s.2", "bswp_fScatterRadiusMin"),
           ("s.6", "bswp_fScatterRadiusMax"), ("bswp_vPatternStretch", "bswp_vPatternStretch"), ("s.3", "bswp_fBulletVelocity"),
           ("s.4", "bswp_fBulletRange"), ("bswp_fBulletImpulse", "bswp_fBulletImpulseUpRatio")]


def angle(radius, distance):
    return math.degrees(math.atan(radius / distance)) if distance > 0 else 0.0


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    eps = merged_files(install, ".ep"); by = {n.split("databases/")[-1]: d for n, d in eps.items()}

    weapons = {}
    for rel in sorted(by):
        if not rel.startswith("entityparams/") or "items/" in rel or "rendering" in rel or "copy" in rel: continue
        if not (rel.endswith("weapon.ep") or rel.endswith("weapon_dm.ep")): continue
        r, objs, ex, rt = wv.load(by[rel]); fl = wv.flat(rt, objs, ex, r["idnt"])
        key = rel.rsplit("/", 1)[-1].replace("weapon_dm.ep", "").replace("weapon.ep", ""); dm = rel.endswith("_dm.ep")
        weapons.setdefault(key, {})["dm" if dm else "sp"] = (rt.type, dict(fl))
    scatter_classes = sorted(k for k, w in weapons.items() if w.get("sp", ("",))[0] == "SBaseScatterWeaponParams" or
                             (w.get("sp") and any(m in w["sp"][1] for m in ("s.1", "s.5"))))
    print(f"1. weapon files: {len(weapons)} weapons; weapons whose file has the scatter members: {scatter_classes}")
    print("   (member: bswp_btBulletType 1 = BT_RAY_STATIC_TRAIL, 2 = BT_RAY_MOVING_TRAIL; distances, radii and range in metres)")
    for key in HITSCAN:
        w = weapons.get(key)
        if not w: print(f"   {key}: file not found"); continue
        for v in ("sp", "dm"):
            if v not in w: continue
            cls, d = w[v]
            row = []
            for dn, bn in SCATTER:
                if bn == "bswp_fBulletImpulseUpRatio": row.append(f"impulse {d.get('bswp_fBulletImpulse')}, up {d.get('bswp_fBulletImpulseUpRatio')}")
                else:
                    val = d.get(dn)
                    if isinstance(val, str) and val.startswith("enum "): val = int(val[5:])
                    row.append(f"{bn[5:]} {val}")
            own = {k: val for k, val in d.items() if k.split(".")[0] in ("aswp_ctBullets", "aswp_slBulletDamage", "dswp_ctBullets", "dswp_slBulletDamage",
                                                                     "cwp_slBulletDamage", "uwp_slBulletDamage", "mgwp_slBulletDamage",
                                                                     "swp_slBulletDamage", "swp_slBulletDamageInZoomMode")}
            print(f"   {key:14s} {v}: {cls}; " + "; ".join(row) + "; own " + str(own))

    print("\n2. scatter angle in degrees, atan(radius / distance), minimum and maximum, per weapon file")
    for key in HITSCAN:
        for v in ("sp", "dm"):
            w = weapons.get(key, {}).get(v)
            if not w: continue
            d = w[1]; dist, lo, hi = d.get("s.1"), d.get("s.2"), d.get("s.6")
            if None in (dist, lo, hi): continue
            print(f"   {key:14s} {v}: distance {dist:g}, radii {lo:g} .. {hi:g}  ->  {angle(lo, dist):.4f} .. {angle(hi, dist):.4f} degrees")
    others = [k for k in weapons if k not in HITSCAN]
    print(f"   the other {len(others)} weapons have no scatter member in the single player file:",
          all(not any(m in weapons[k]["sp"][1] for m in ("s.1", "s.2", "s.5", "s.6")) for k in others if "sp" in weapons[k]))

    print("\n3. CBulletFireing (the bullet description of an enemy attack, member sb_bfcBulletFireing of CSingleBlast)")
    rows = []; vals = collections.defaultdict(collections.Counter); files = set(); per_type = collections.defaultdict(set)
    firing = collections.defaultdict(set)
    for name, d in sorted(eps.items()):
        r = parse(d)
        if not any(t.name == "CBulletFireing" for t in r["types"].values()): continue
        objs, _ = TreeDec(d, r).objects()
        rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}

        def walk(o):
            for k, v in o:
                if isinstance(v, Obj):
                    if v.type == "CBulletFireing":
                        fl = dict(wv.flat(v, objs, ex, r["idnt"]))
                        rows.append((name, fl)); files.add(name)
                        for kk, vv in fl.items():
                            vv = int(vv[5:]) if isinstance(vv, str) and vv.startswith("enum ") else vv
                            vals[kk.split(".")[-1]][vv] += 1
                        bt = fl.get("bf_btBulletType"); n = fl.get("bf_ctBullets")
                        per_type[bt].add(name.rsplit("/", 1)[-1])
                        if isinstance(n, int) and n > 0: firing[(bt, n)].add(name.rsplit("/", 1)[-1])
                    else: walk(v)
        for o in objs.values():
            if isinstance(o, Obj): walk(o)
    print(f"   {len(rows)} objects in {len(files)} .ep files")
    for k in ("bf_ctBullets", "bf_dtDamageType", "bf_btBulletType", "bf_slBulletDamage", "bf_fBulletVelocity", "bf_fBulletReach"):
        c = vals.get(k, {})
        print(f"   {k:20s} " + ", ".join(f"{v}({n})" for v, n in sorted(c.items(), key=lambda x: str(x[0]))))
    for t in sorted(per_type, key=str):
        names = sorted(per_type[t])
        print(f"   bullet type {t}: {len(names)} files" + (": " + ", ".join(names) if len(names) <= 12 else ""))
    print("   attacks that really fire (bf_ctBullets above 0): bullet type, bullets -> files")
    for (t, n), names in sorted(firing.items(), key=str):
        print(f"      type {t}, {n} bullet(s): {len(names)} files: " + ", ".join(sorted(names)))
    both = collections.Counter((fl.get("bf_btBulletType"), fl.get("bf_ctBullets")) for _, fl in rows)
    print("   (bullet type, bullets per attack): " + ", ".join(f"{k}x{n}" for k, n in sorted(both.items(), key=str)))

    print("\n4. the damage inflictor of the stored shooters (SBulletShooterEntity, worlds) is in projectile_classes_verify.py section 5")


if __name__ == "__main__":
    main()
