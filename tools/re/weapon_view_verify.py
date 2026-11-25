"""Evidence for docs/re/weapon-view.md (T01-121): what the data says about the weapon view and rendering parameters.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries).

It prints
  1. for each of the 17 weapons, single player and deathmatch: the six sway and kick members (`bwp_fOffsetX`, `bwp_fOffsetY`,
     `bwp_fOffsetZ`, `bwp_fFrequencyX`, `bwp_fFrequencyY`, `bwp_fOffsetZExponent`) and `bwp_fDoubleWeaponFireDelay`, with the weapons
     marked whose charging time is above 0 (the only ones that use the six members, spec section 2);
  2. the 16 rendering parameter files with the eight members of `SBaseWeaponRenderingParams` by their binary names, the horizontal
     field of view in degrees (the member is in radians), and the difference between the one- and two-weapon values;
  3. the sniper's zoom members and the zoom multiplier curve they give (`1 / zoom factor`, interpolated over the full zooming time);
  4. the view parameters of the player puppet file (`ppp_fViewDampingK`, `ppp_fViewDampingD`, `ppp_fMaxMoveBanking`,
     `ppp_fMaxSideStepBanking`).

Usage: python -I tools/re/weapon_view_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR; about 1 minute)
"""
import math, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import merged_files  # noqa: E402
import weapons_verify as wv  # noqa: E402

# SBaseWeaponParams: data member -> binary name (weapon-classes.md section 4.1)
SWAY = [("b.7", "bwp_fOffsetX"), ("b.9", "bwp_fOffsetY"), ("b.11", "bwp_fOffsetZ"), ("b.8", "bwp_fFrequencyX"), ("b.10", "bwp_fFrequencyY"),
        ("b.12", "bwp_fOffsetZExponent"), ("b.34", "bwp_fDoubleWeaponFireDelay"), ("b.13", "bwp_fChargingTime")]
# SBaseWeaponRenderingParams in the binary's order (positions 0 to 7)
REND = ["bwrp_vPosition", "bwrp_vAngle", "bwrp_fFOV", "bwrp_fBobbingMul", "bwrp_vPositionDoubleWeapon", "bwrp_vAngleDoubleWeapon",
        "bwrp_fFOVDoubleWeapon", "bwrp_fFrontClipDist"]


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    eps = merged_files(install, ".ep"); by = {n.split("databases/")[-1]: d for n, d in eps.items()}
    weapons = {}; rend = {}
    for rel in sorted(by):
        if not rel.startswith("entityparams/") or "items/" in rel or "copy" in rel: continue
        base = rel.rsplit("/", 1)[-1]
        if base.endswith("weaponrendering.ep"):
            r, objs, ex, rt = wv.load(by[rel]); rend[base.replace("weaponrendering.ep", "")] = (rt.type, [v for k, v in wv.flat(rt, objs, ex, r["idnt"])], dict(wv.flat(rt, objs, ex, r["idnt"])))
        elif base.endswith("weapon.ep") or base.endswith("weapon_dm.ep"):
            r, objs, ex, rt = wv.load(by[rel]); fl = wv.flat(rt, objs, ex, r["idnt"])
            key = base.replace("weapon_dm.ep", "").replace("weapon.ep", ""); dm = base.endswith("_dm.ep")
            weapons.setdefault(key, {})["dm" if dm else "sp"] = (rt.type, dict(fl))

    print("1. sway and kick members and the double-weapon fire delay (seconds; offsets in metres; frequencies in cycles per second)")
    print("   weapon sp/dm: " + ", ".join(b for _, b in SWAY) + "   [* = charging weapon]")
    users = []
    for key in sorted(weapons):
        for v in ("sp", "dm"):
            w = weapons[key].get(v)
            if not w: continue
            d = w[1]; ch = d.get("b.13") or 0
            if ch > 0 and v == "sp": users.append(key)
            print(f"   {key:16s} {v}{'*' if ch > 0 else ' '}: " + ", ".join(f"{d.get(a)}" for a, _ in SWAY))
    print("   weapons with a charging time above 0 (the only ones that use the six sway members):", users)

    print("\n2. rendering parameter files (members by the binary's names; angles in degrees as stored; FOV stored in radians)")
    print(f"   {len(rend)} files; class names: {sorted({t for t, _, _ in rend.values()})}")
    for key in sorted(rend):
        t, vals, d = rend[key]
        items = [(k, v) for k, v in d.items()]
        # the flat member order is the file order: positions 0..7 of the binary table
        seq = [v for _, v in items]
        fov = seq[2] if len(seq) > 2 else None
        fov2 = seq[6] if len(seq) > 6 else None
        deg = f"{math.degrees(fov):.1f}" if isinstance(fov, float) else "?"
        print(f"   {key:16s} " + "; ".join(f"{REND[i][5:]} {seq[i]}" for i in range(min(8, len(seq)))) + f"   (FOV {deg} deg"
              + (f", two weapons {math.degrees(fov2):.1f}" if isinstance(fov2, float) else "") + ")")
    same = sum(1 for _, _, d in rend.values() if list(d.values())[:3] == list(d.values())[4:7])
    print(f"   files whose two-weapon position, angle and FOV equal the one-weapon values: {same} of {len(rend)}")

    print("\n3. sniper zoom (swp_fMinZoomFactor, swp_fMaxZoomFactor, swp_fFullZoomingTime) and the field-of-view multiplier over the zoom")
    for v in ("sp", "dm"):
        w = weapons.get("sniper", {}).get(v)
        if not w: continue
        d = w[1]; mn = d.get("w.3") ; mx = d.get("w.4"); tm = d.get("w.5")
        names = {k: val for k, val in d.items() if k.startswith(("w.", "swp_"))}
        print(f"   sniper {v}: {names}")
        zmin = next((val for k, val in d.items() if k.endswith("fMinZoomFactor")), mn)
        zmax = next((val for k, val in d.items() if k.endswith("fMaxZoomFactor")), mx)
        ztime = next((val for k, val in d.items() if k.endswith("fFullZoomingTime")), tm)
        if all(isinstance(x, (int, float)) and x for x in (zmin, zmax, ztime)):
            print("      multiplier = lerp(1/min, 1/max, t / time): " + ", ".join(
                f"t={f * ztime:g}s: {(1 / zmin) + (1 / zmax - 1 / zmin) * f:.4f} (horizontal FOV {80 * ((1 / zmin) + (1 / zmax - 1 / zmin) * f):.1f} deg)"
                for f in (0, .25, .5, 1)))

    print("\n4. player puppet view members")
    for rel in sorted(by):
        if not rel.endswith("playerpuppet.ep") and not rel.endswith("playerpuppet_dm.ep"): continue
        r, objs, ex, rt = wv.load(by[rel]); d = dict(wv.flat(rt, objs, ex, r["idnt"]))
        print(f"   {rel}: " + ", ".join(f"{k} {d.get(k)}" for k in ("ppp_fViewDampingK", "ppp_fViewDampingD", "ppp_fMaxMoveBanking",
                                                                   "ppp_fMaxSideStepBanking", "ppp_fViewAutoCenterDelay", "ppp_fViewAutoCenterMultiplier")))


if __name__ == "__main__":
    main()
