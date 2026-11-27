"""Evidence for docs/re/weapon-pickups.md (T01-122): what the data says about the weapon pickups and the weapon tables.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries).

It prints
  1. the table that maps a `WeaponIndex` to its parameter file and its rendering parameter file (the file names are the binary's
     strings, quoted in the spec) and whether the single player file, the deathmatch variant (`_DM`) and the rendering file exist in
     the install, and which class the file has;
  2. the weapon files that no index names (leftovers);
  3. the combo weapons: whether any world places a combo pickup (`SComboWeaponsItemEntity` in a type table) and the value of
     `ggp_cwuComboWeaponsUsing` in the global parameter file.
The ammunition a pickup grants per weapon is `item_classes_verify.py` (section 4 of `item-classes-ammo-weapons.md`) and is not repeated.

Usage: python -I tools/re/weapon_pickups_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR; about 2 minutes)
"""
import os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import merged_files, parse  # noqa: E402
import weapons_verify as wv  # noqa: E402

# index -> file stem of EntityParams/<stem>Weapon.ep and EntityParams/<stem>WeaponRendering.ep (strings of the binary, spec section 1)
STEM = {0: "CircularSaw", 1: "ZapGun", 2: "AutoShotgun", 3: "DoubleShotgun", 4: "Uzi", 5: "MiniGun", 6: "RocketLauncher", 7: "GrenadeLauncher",
        8: "PlasmaRifle", 9: "KlodovikGun", 10: "Cannon", 11: "SeriousBomb", 12: "Colt", 13: "Sniper", 15: "BeamGun", 16: "Flamer"}


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    eps = merged_files(install, ".ep"); by = {n.split("databases/")[-1]: d for n, d in eps.items()}

    def cls(rel):
        r = parse(by[rel]); wv_r, objs, ex, rt = wv.load(by[rel]); return rt.type

    print("1. WeaponIndex -> parameter files (EntityParams/<stem>Weapon.ep, <stem>Weapon_DM.ep, <stem>WeaponRendering.ep)")
    named = set()
    for i in range(-1, 18):
        if i not in STEM:
            print(f"   {i:2d}: no files (the function returns the empty string)"); continue
        s = STEM[i].lower(); sp = f"entityparams/{s}weapon.ep"; dm = f"entityparams/{s}weapon_dm.ep"; rd = f"entityparams/{s}weaponrendering.ep"
        named |= {sp, dm, rd}
        print(f"   {i:2d} {STEM[i]:16s} sp {'yes' if sp in by else 'NO '} {cls(sp) if sp in by else '':34s} dm {'yes' if dm in by else 'NO '} "
              f"rendering {'yes' if rd in by else 'NO '} {cls(rd) if rd in by else ''}")
    print("\n2. weapon files in EntityParams that no index names:")
    for rel in sorted(by):
        if rel.startswith("entityparams/") and "weapon" in rel.rsplit("/", 1)[-1] and "items/" not in rel and rel not in named:
            print("   ", rel)

    print("\n3. global parameters and session defaults that govern the combo weapons and the co-op stay rules")
    for rel in sorted(merged_files(install, ".rsc")):
        if not rel.endswith("globalgameparameters.rsc"): continue
        d = merged_files(install, ".rsc")[rel]; r, objs, ex, rt = wv.load(d); fl = dict(wv.flat(rt, objs, ex, r["idnt"]))
        for k in ("ggp_cwuComboWeaponsUsing", "ggp_fWeaponItemStayTimeAfterDrop", "ggp_fWeaponFireThreatDistance"):
            print(f"   {k}: {fl.get(k)}")
    worlds = 0; combo = 0
    for rel, d in sorted(merged_files(install, ".wld").items()):
        worlds += 1
        r = parse(d)
        if any(t.name in ("SComboWeaponsItemEntity", "CComboWeaponsItemEntity") for t in r["types"].values()): combo += 1
    print(f"   worlds read: {worlds}; worlds whose type table has the combo-weapons pickup class: {combo}")


if __name__ == "__main__":
    main()
