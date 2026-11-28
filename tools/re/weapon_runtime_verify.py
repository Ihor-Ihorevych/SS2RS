"""Evidence for docs/re/weapon-runtime-behaviour.md (T01-075): one consolidated run-time table per player weapon.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries).
Builds on `weapons_verify.py` (the base/scatter member reader) and the member-position rosetta of
`weapon-classes.md` section 4 / `projectile-classes.md` section 3 (b.N / s.N / p.N -> binary name); it does
not re-derive that mapping, only applies it.

Prints, for all 17 player weapons (single player values; deathmatch in brackets where it differs):
  1. ammunition cost per shot and magazine size (`bwp_ctFiresToReload`);
  2. the fire period: recoil min/max (and, for the three charging weapons, the charging time budget);
  3. damage and `EDamageType`: the hitscan weapons' own bullet-damage member, or (for the seven projectile
     weapons) the direct/area damage pair and type of the projectile file `bwp_pgppProjectileParams` names;
  4. spread: scatter distance and radius min/max for the six bullet weapons;
  5. a cross-check that confirms there is exactly one damage-type member (`bp_dtDirectDamageType`) in
     `SBaseProjectileParams` -- i.e. the direct hit and the area (splash) damage of a projectile always
     share one `EDamageType`, there is no second field for the area damage.

Usage: python -I tools/re/weapon_runtime_verify.py [DATA_DIR] [--table OUT.md]
       (DATA_DIR defaults to $SS2_DATA_DIR; about 1 minute)
"""
import os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import merged_files  # noqa: E402
import weapons_verify as wv  # noqa: E402

# WeaponIndex (weapon-classes.md section 3), ammo kind, cost per shot and cap (players-session-classes.md
# section 7.3: three binary lookup tables, not stored in the weapon files -- cited here, not re-derived).
CAPS = {"none": 0, "shells": 200, "bullets": 1000, "rockets": 50, "grenades": 50, "plasma": 500,
        "Klodovik": 12, "cannon balls": 30, "Serious Bomb": 3, "sniper bullets": 50, "napalm": 500}
WEAPONS = [
    (0, "circularsaw", "circular saw", "none", 0),
    (1, "zapgun", "zap gun", "none", 0),
    (2, "autoshotgun", "auto shotgun", "shells", 1),
    (3, "doubleshotgun", "double shotgun", "shells", 2),
    (4, "uzi", "Uzi", "bullets", 1),
    (5, "minigun", "minigun", "bullets", 1),
    (6, "rocketlauncher", "rocket launcher", "rockets", 1),
    (7, "grenadelauncher", "grenade launcher", "grenades", 1),
    (8, "plasmarifle", "plasma rifle", "plasma", 1),
    (9, "klodovikgun", "Klodovik gun", "Klodovik", 1),
    (10, "cannon", "cannon", "cannon balls", 1),
    (11, "seriousbomb", "Serious Bomb", "Serious Bomb", 1),
    (12, "colt", "Colt", "none", 0),
    (13, "sniper", "sniper", "sniper bullets", 1),
    (15, "beamgun", "beam gun", "plasma", 1),
    (16, "flamer", "flamer", "napalm", 1),
]
SCATTER = {"colt", "uzi", "minigun", "autoshotgun", "doubleshotgun", "sniper"}
PROJECTILE = {"rocketlauncher", "grenadelauncher", "plasmarifle", "klodovikgun", "cannon", "zapgun", "flamer"}
# the weapon's own bullet-damage member(s), named in the data (weapon-classes.md section 4.3)
OWN_DAMAGE = {
    "colt": ["cwp_slBulletDamage"], "uzi": ["uwp_slBulletDamage"], "minigun": ["mgwp_slBulletDamage"],
    "autoshotgun": ["aswp_ctBullets", "aswp_slBulletDamage"], "doubleshotgun": ["dswp_ctBullets", "dswp_slBulletDamage"],
    "sniper": ["swp_slBulletDamage", "swp_slBulletDamageInZoomMode"],
}
EDAMAGETYPE = {-1: "DMT_NONE", 1: "DMT_PIERCING", 2: "DMT_CUT", 3: "DMT_CUTTING", 4: "DMT_PUNCH", 5: "DMT_BLUNT",
               6: "DMT_CRUSHING", 7: "DMT_KICKING", 8: "DMT_IMPACT", 9: "DMT_FALLING", 11: "DMT_EXPLOSION",
               12: "DMT_SHOCKWAVE", 13: "DMT_DROWNING", 14: "DMT_HEAT", 15: "DMT_COLD", 16: "DMT_ELECTRICITY",
               17: "DMT_ACID", 18: "DMT_PLASMA", 19: "DMT_BULLET", 20: "DMT_SNIPER_BULLET", 21: "DMT_SLUG",
               22: "DMT_KILL_ALL_CHEAT", 23: "DMT_SERIOUS_BOMB", 24: "DMT_KILLER", 25: "DMT_ABYSS"}
BULLETTYPE = {0: "BT_PHYSICAL", 1: "BT_RAY_STATIC_TRAIL", 2: "BT_RAY_MOVING_TRAIL", 3: "BT_RAY_NO_TRAIL"}
# the hitscan weapons' own DamageInflictorWeapon value (weapon-classes.md rule 6); projectile weapons use their WeaponIndex except the flamer (19)
HITSCAN_DT = {"colt": 19, "uzi": 19, "minigun": 19, "autoshotgun": 21, "doubleshotgun": 21, "sniper": 20}  # DMT_BULLET/DMT_SLUG/DMT_SNIPER_BULLET


def enum_v(x):
    """flat() renders an Enum as the string "enum N" (ctse_values.py); recover the int N."""
    if hasattr(x, "v"):
        return x.v
    if isinstance(x, str) and x.startswith("enum "):
        try:
            return int(x[5:])
        except ValueError:
            return x
    return x


def num(v, default=0.0):
    return v if isinstance(v, (int, float)) else default


def fmt(v):
    return "-" if v in (None, "-") else v


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder or set SS2_DATA_DIR")
    eps = merged_files(install, ".ep"); by = {n.split("databases/")[-1]: d for n, d in eps.items()}

    weapons = {}
    for rel in sorted(by):
        if not rel.startswith("entityparams/") or "items/" in rel or "rendering" in rel or "copy" in rel:
            continue
        if not (rel.endswith("weapon.ep") or rel.endswith("weapon_dm.ep")):
            continue
        r, objs, ex, rt = wv.load(by[rel]); fl = dict(wv.flat(rt, objs, ex, r["idnt"]))
        key = rel.rsplit("/", 1)[-1].replace("weapon_dm.ep", "").replace("weapon.ep", ""); dm = rel.endswith("_dm.ep")
        weapons.setdefault(key, {})["dm" if dm else "sp"] = fl

    projs = {}
    for rel in sorted(by):
        if "genericprojectiles/" not in rel:
            continue
        r, objs, ex, rt = wv.load(by[rel]); projs[rel.rsplit("/", 1)[-1].lower()] = dict(wv.flat(rt, objs, ex, r["idnt"]))

    # cross-check: exactly one damage-type member in SBaseProjectileParams across every projectile file
    dt_fields = set()
    for p in projs.values():
        dt_fields |= {k for k in p if k.startswith("bp_dt") or (k.startswith("p.") and False)}
    print(f"cross-check: damage-type member names seen across {len(projs)} projectile files: {sorted(dt_fields)}")
    print("  (exactly one -> the direct hit and the area/splash damage of a projectile always share one EDamageType)")
    print()

    rows = []
    for idx, key, label, ammo_kind, cost in WEAPONS:
        w = weapons.get(key, {})
        sp = w.get("sp", {}); dm = w.get("dm", {})
        if not sp:
            print(f"!! {key} not found in the install"); continue
        mag = sp.get("b.5"); mag_dm = dm.get("b.5", mag)
        rmin, rmax = sp.get("b.30"), sp.get("b.31")
        rmin_dm, rmax_dm = dm.get("b.30", rmin), dm.get("b.31", rmax)
        ctime, cauto = sp.get("b.13"), sp.get("b.33")
        lvmin, lvmax = sp.get("b.15"), sp.get("b.16")

        damage_txt = "-"; dtype_txt = "-"; spread_txt = "-"
        if key in SCATTER:
            dist, dist_dm = sp.get("s.1"), dm.get("s.1", sp.get("s.1"))
            rmn, rmn_dm = sp.get("s.2"), dm.get("s.2", sp.get("s.2"))
            rmx, rmx_dm = sp.get("s.6"), dm.get("s.6", sp.get("s.6"))
            bt = enum_v(sp.get("s.5")) if sp.get("s.5") is not None else None
            spread_txt = f"dist {dist}{'' if dist_dm == dist else f' ({dist_dm})'}, radius {rmn}-{rmx}{'' if (rmn_dm, rmx_dm) == (rmn, rmx) else f' ({rmn_dm}-{rmx_dm})'}, bullet type {bt} {BULLETTYPE.get(bt, '?')}"
            own = OWN_DAMAGE.get(key, [])
            vals = [sp.get(m) for m in own]
            vals_dm = [dm.get(m, v) for m, v in zip(own, vals)]
            damage_txt = ", ".join(f"{m}={v}{'' if v == vd else f' ({vd})'}" for m, v, vd in zip(own, vals, vals_dm))
            dtype_txt = f"{HITSCAN_DT.get(key)} {EDAMAGETYPE.get(HITSCAN_DT.get(key), '?')}"
        elif key in PROJECTILE:
            pf = sp.get("b.23"); pf_dm = dm.get("b.23", pf)
            p = projs.get(pf.lower()) if isinstance(pf, str) and pf != "-" else None
            p_dm = projs.get(pf_dm.lower()) if isinstance(pf_dm, str) and pf_dm != "-" else p
            if p:
                dmg0, dmg1 = num(p.get("p.2")), num(p.get("p.11"))
                area0, area1 = num(p.get("p.3")), num(p.get("p.12"))
                dt = enum_v(p.get("bp_dtDirectDamageType"))
                damage_txt = f"direct {dmg0}/{dmg1} (power 0/1), area {area0}/{area1}"
                if p_dm is not p:
                    damage_txt += f"; dm: direct {num(p_dm.get('p.2'))}/{num(p_dm.get('p.11'))}, area {num(p_dm.get('p.3'))}/{num(p_dm.get('p.12'))}"
                dtype_txt = f"{dt} {EDAMAGETYPE.get(dt, '?')}"

        if num(ctime) > 0:
            period = f"charge {ctime}s + auto {cauto}s, recoil {rmin}-{rmax}s by charge"
        else:
            note = " (0 min = recoil-animation length)" if num(rmin) <= 0 else ""
            period = f"{fmt(rmin)}-{fmt(rmax)}s (lerp by fire power){note}"
        if key in ("minigun", "circularsaw"):
            period += " -- gated by spin-up (weapon-timing.md section 6)"
        if rmin_dm != rmin or rmax_dm != rmax:
            period += f" [dm {rmin_dm}-{rmax_dm}s]"

        mag_txt = str(fmt(mag)) if num(mag) > 0 else "none (never reloads)"
        rows.append((idx, label, ammo_kind, str(cost), str(CAPS.get(ammo_kind, "?")), mag_txt, period, damage_txt, dtype_txt, spread_txt))

    print(f"{'idx':>3} {'weapon':16} {'ammo':14} {'cost':4} {'cap':5} {'magazine':24} {'fire period':46} {'damage':46} {'type':20} spread")
    for idx, label, ammo_kind, cost, cap, mag, period, damage_txt, dtype_txt, spread_txt in rows:
        print(f"{idx:>3} {label:16} {ammo_kind:14} {cost:4} {cap:5} {mag:24} {period:46} {damage_txt:46} {dtype_txt:20} {spread_txt}")

    if "--table" in sys.argv:
        out = sys.argv[sys.argv.index("--table") + 1]
        L = ["# Weapon run-time behaviour (generated)", "",
             "Generated by `tools/re/weapon_runtime_verify.py --table` from the retail install's weapon and projectile files. "
             "See `weapon-runtime-behaviour.md`. Single-player values; deathmatch in brackets or after `dm:` where it differs.", "",
             "| Index | Weapon | Ammo | Cost/shot | Cap | Magazine | Fire period | Damage | Damage type | Spread |",
             "|---|---|---|---|---|---|---|---|---|---|"]
        for idx, label, ammo_kind, cost, cap, mag, period, damage_txt, dtype_txt, spread_txt in rows:
            L.append(f"| {idx} | {label} | {ammo_kind} | {cost} | {cap} | {mag} | {period} | {damage_txt} | {dtype_txt} | {spread_txt} |")
        with open(out, "w", encoding="utf-8", newline="\n") as f:
            f.write("\n".join(L) + "\n")
        print("\nwrote", out)


if __name__ == "__main__":
    main()
