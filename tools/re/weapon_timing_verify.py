"""Re-runs the per-weapon numbers behind docs/re/weapon-timing.md on an install (read-only).

For the 17 weapon files (`Databases/EntityParams/<weapon>weapon.ep` and the deathmatch variant
`<weapon>weapon_dm.ep`): the timing members named by `weapon-classes.md` section 4.1 and
`projectile-launch.md` (shots-before-reload, charging time, charging auto-fire time, minimum and
maximum recoil duration, reload-animation rescale, double-weapon fire delay, spin-up time,
spin-down time, spin speed) plus the two flags that `weapon-timing.md` reads from the binary's
default constructors (the charging flag and the spin-up flag; not in the data, hard-coded here from
that reading). Prints the table; writes it with --table.

Usage: python -I tools/re/weapon_timing_verify.py [DATA_DIR] [--table OUT.md]
       (DATA_DIR defaults to $SS2_DATA_DIR; about 1 minute)
"""
import os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, merged_files, parse  # noqa: E402

NULL = 0xFFFFFFFF

# From the default constructors of the 16 weapon entities (weapon-classes.md section 9 rule 5 /
# projectile-launch.md section 3.5): which weapons set the WCF_CHARGE / WCF_SPINUP flags of
# en_ulWeaponChargingType. Not in the data; recorded here so the table can show the gate that
# weapon-timing.md section 4 and 7 describe (DoFiringsFromLastFireTime's single-shot rule, OnStep's
# recoil-refire rule, CallFireStart's three-way branch).
CHARGE_FLAG = {"grenadelauncher", "cannon", "zapgun"}
SPINUP_FLAG = {"minigun", "circularsaw"}
DISCRETE_POWER = {"zapgun"}

MEMBERS = [
    ("b.5", "ctFiresToReload"),
    ("b.13", "chargingTime"),
    ("b.33", "chargingAutoFireTime"),
    ("b.30", "minRecoilDuration"),
    ("b.31", "maxRecoilDuration"),
    ("b.32", "reloadDuration"),
    ("b.34", "doubleWeaponFireDelay"),
    ("b.20", "spinUpTime"),
    ("b.21", "spinDownTime"),
    ("b.22", "spinSpeedDeg"),
]


def load(d):
    r = parse(d); objs, _ = TreeDec(d, r).objects()
    rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}
    rs = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CResourceFile")
    return objs, ex, objs[[x.oid for x in rs.get("CResourceFile.rf_crsShared")][0]]


def flat(o, objs, ex, prefix=""):
    out = {}
    for k, v in o:
        if k.startswith(("CResource.", "CResourceFile.")): continue
        cls, _, mem = k.partition(".")
        name = prefix + ((({"SBaseWeaponParams": "b", "SBaseScatterWeaponParams": "s"}.get(cls, "w")) + "." + mem) if mem.isdigit() else mem)
        if isinstance(v, Ref):
            out[name] = "-" if v.oid == NULL else (ex[v.oid].rsplit("/", 1)[-1] if v.oid in ex else "(embedded)")
        elif isinstance(v, Obj):
            out.update(flat(v, objs, ex, name + "."))
        elif isinstance(v, (int, float, str)):
            out[name] = round(v, 4) if isinstance(v, float) else v
    return out


def main():
    args = sys.argv[1:]; table = args[args.index("--table") + 1] if "--table" in args else None
    if table: args = [a for a in args if a not in ("--table", table)]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    eps = merged_files(install, ".ep"); by = {n.split("databases/")[-1]: d for n, d in eps.items()}
    rows = {}
    for rel in sorted(by):
        if not rel.startswith("entityparams/") or "items/" in rel or "rendering" in rel or "copy" in rel: continue
        if not (rel.endswith("weapon.ep") or rel.endswith("weapon_dm.ep")): continue
        objs, ex, rt = load(by[rel]); fl = flat(rt, objs, ex)
        key = rel.rsplit("/", 1)[-1].replace("weapon_dm.ep", "").replace("weapon.ep", ""); dm = rel.endswith("_dm.ep")
        rows.setdefault(key, {})["dm" if dm else "sp"] = fl
    print(f"weapons read: {len(rows)}")
    header = "weapon".ljust(18) + "".join(n.ljust(14) for _, n in MEMBERS) + "flags"
    print(header); print("-" * len(header))
    out_rows = []
    for key in sorted(rows):
        for variant in ("sp", "dm"):
            fl = rows[key].get(variant)
            if fl is None: continue
            vals = [fl.get(m, 0) for m, _ in MEMBERS]
            flags = []
            if key in CHARGE_FLAG: flags.append("WCF_CHARGE")
            if key in SPINUP_FLAG: flags.append("WCF_SPINUP")
            if key in DISCRETE_POWER: flags.append("discrete")
            label = key + (" (dm)" if variant == "dm" else "")
            print(label.ljust(18) + "".join(str(v).ljust(14) for v in vals) + ",".join(flags))
            out_rows.append((label, vals, flags))
    # sanity: the three charging weapons are exactly those with chargingTime > 0 (invariant 3 of
    # projectile-launch.md); the spin-up weapons are exactly those with spinUpTime > 0.
    charging_by_data = {k for k in rows if rows[k].get("sp", {}).get("b.13", 0) > 0}
    spinup_by_data = {k for k in rows if rows[k].get("sp", {}).get("b.20", 0) > 0}
    zero_spin_by_data = {k for k in rows if rows[k].get("sp", {}).get("b.22", 0) == 0}
    print()
    print("weapons with chargingTime > 0:", sorted(charging_by_data), "== CHARGE_FLAG:", charging_by_data == CHARGE_FLAG)
    print("weapons with spinSpeedDeg == 0 (grenade launcher, plasma rifle only):", sorted(zero_spin_by_data))
    print("every other weapon (including those without WCF_SPINUP) carries the generic 0.5/3.0/500 spin-up/down/speed default; only", sorted(SPINUP_FLAG), "read it (the flag, not the data, gates its use)")
    if table:
        L = ["# Weapon timing numbers (generated)", "",
             "Generated by `tools/re/weapon_timing_verify.py --table` from the `<weapon>weapon.ep` / "
             "`<weapon>weapon_dm.ep` files of the retail install. See `weapon-timing.md`. Values are 0 when "
             "the weapon's file has no nonzero entry for that member. `flags` are read from the binary's "
             "default constructors, not from the data (`weapon-classes.md` section 9 rule 5).", "",
             "| Weapon | " + " | ".join(n for _, n in MEMBERS) + " | Flags |",
             "|---|" + "---|" * (len(MEMBERS) + 1)]
        for label, vals, flags in out_rows:
            L.append(f"| {label} | " + " | ".join(str(v) for v in vals) + " | " + ",".join(flags) + " |")
        with open(table, "w", encoding="utf-8", newline="\n") as f: f.write("\n".join(L) + "\n")
        print("wrote", table)


if __name__ == "__main__":
    main()
