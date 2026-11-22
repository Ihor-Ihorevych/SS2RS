"""Prints the values quoted in docs/re/player-rules.md (read-only).

Global game parameters (difficulty tables, power-up durations, damage multipliers, score rules), the player
puppet's health and a few movement/view values, and the single number in every `Items/**.ep` file.

Usage: python -I tools/re/rules_verify.py [DATA_DIR]
"""
import os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Enum, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402


def decode(d):
    r = parse(d); objs, _ = TreeDec(d, r).objects(); return r, objs


def flat(o, prefix=""):
    """{dotted member path without class prefix: value} for nested by-value structs."""
    out = {}
    for k, v in o:
        key = prefix + k.split(".", 1)[1]
        if isinstance(v, Obj): out.update(flat(v, key + "."))
        else: out[key] = v
    return out


def show(v):
    if isinstance(v, float): return f"{v:.5g}"
    if isinstance(v, Enum): return f"{v.name}({v.v:#x})"
    if isinstance(v, Ref): return "null" if v.oid == 0xFFFFFFFF else "ref"
    return str(v)


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    rsc = merged_files(install, ".rsc"); key = next(k for k in rsc if k.endswith("globalgameparameters.rsc"))
    r, objs = decode(rsc[key]); g = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CGlobalGameParams"); f = flat(g)
    print("== CGlobalGameParams (selected)")
    for k in sorted(f):
        if any(s in k for s in ("gpp_fFOV", "AutoJump", "ExtraLifeScore", "SeriousScoreMultiplier", "GiveAllWeapons", "StayTimeAfterDrop", "FightEachother", "EnemyVSEnemy", "msr_f", "pud_f", "dif_", "gd_dp", "ComboWeapons", "ChooseLeftWeapon", "UsageDistance", "bp_fDefault", "StandardSeeDistance", "ShortsightedDistance", "DefaultInitialDelay")):
            print(f"  {k} = {show(f[k])}")
    eps = merged_files(install, ".ep"); print("== Items/**.ep (CBaseItemParams.1)")
    for name in sorted(eps):
        if "/items/" in name or name.startswith("items/"):
            r2, o2 = decode(eps[name]); it = next((o for o in o2.values() if isinstance(o, Obj) and o.type == "CBaseItemParams"), None)
            if it: print(f"  {name.split('items/')[-1]} = {show(it.get('CBaseItemParams.1'))}")
    pk = next(k for k in eps if k.endswith("/playerpuppet.ep") or k == "playerpuppet.ep")
    r3, o3 = decode(eps[pk]); pp = next(o for o in o3.values() if isinstance(o, Obj) and o.type == "SPlayerPuppetParams"); pf = flat(pp)
    print("== PlayerPuppet.ep (selected)")
    for k in sorted(pf):
        if any(s in k for s in ("ppp_f", "php_sl", "SPuppetParams.1", "lpbp_fJumpUp", "lpbp_fMaxValidSlope", "pbp_fViewHeight", "pbp_fKeepBreath", "ma_fStep", "ma_fAllowedFall", "omp_fMaxCarryingMass")):
            print(f"  {k} = {show(pf[k])}")


if __name__ == "__main__":
    main()
