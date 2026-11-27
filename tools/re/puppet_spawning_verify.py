"""Spawn effect configurations as the spawn source / spawnee code of docs/re/puppet-spawning.md uses them (T01-145, read-only, data only).

Reads every `CSpawnEffectConfiguration` of the install: the shared `.sfc` files and the configurations embedded in the worlds,
and prints the value tables of the members the puppet code reads (stop chance, match throw direction, prepare / spawn / spawnee
animations), which spawners use a puppet as spawn source, and the formulas of the page for its test vectors.

Usage: python -I tools/re/puppet_spawning_verify.py [DATA_DIR]      (or set SS2_DATA_DIR)
Standard library only; read-only on the install; prints numbers and identifiers.
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, merged_files, parse  # noqa: E402


def hist(vals, n=10):
    c = collections.Counter(vals)
    return ", ".join(f"{k!r} x{v}" for k, v in c.most_common(n)) + (f" ... ({len(c)} distinct)" if len(c) > n else "")


def fl(v):
    return round(v, 4) if isinstance(v, float) else v


def collect(files, tag):
    out = []
    for name, d in sorted(files.items()):
        r = parse(d)
        objs, _ = TreeDec(d, r).objects()
        for o in objs.values():
            if isinstance(o, Obj) and o.type == "CSpawnEffectConfiguration":
                v = {k.split(".")[-1]: x for k, x in o}
                out.append((tag, name.split("/")[-1], r, v))
    return out


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder or set SS2_DATA_DIR")
    cfg = collect(merged_files(install, ".sfc"), "sfc")
    cfg += collect(merged_files(install, ".wld"), "wld")
    print("configurations:", hist([c[0] for c in cfg]))
    def idn(c, k):
        r, v = c[2], c[3][k]
        return r["idnt"][v] if isinstance(v, int) and 0 <= v < len(r["idnt"]) else "?"
    for tag in ("sfc", "wld"):
        cs = [c for c in cfg if c[0] == tag]
        print(f"\n== {tag} ({len(cs)})")
        print("  sfxc_fSpawnSourceStopChance:", hist([fl(c[3]["sfxc_fSpawnSourceStopChance"]) for c in cs]))
        print("  sfxc_bShouldMatchThrowDir:", hist([c[3]["sfxc_bShouldMatchThrowDir"] for c in cs]))
        print("  sfxc_idSpawnSourcePrepareAnimation:", hist([idn(c, "sfxc_idSpawnSourcePrepareAnimation") for c in cs]))
        print("  sfxc_idSpawnSourceAnimation:", hist([idn(c, "sfxc_idSpawnSourceAnimation") for c in cs]))
        print("  sfxc_idSpawneeAnimation:", hist([idn(c, "sfxc_idSpawneeAnimation") for c in cs]))
        print("  sfx_fSpawnDelay:", hist([fl(c[3]["sfx_fSpawnDelay"]) for c in cs]))
    print("  files of the .sfc configurations:", [c[1] for c in cfg if c[0] == "sfc"])
    print("\n== formulas")
    print("  spawn source stops at once when r <= chance (r uniform in [0, 1)): chance 0 -> only r = 0; 0.25 -> 25 percent; 1 -> always")
    print("  braking wait = max(|velocity| / deceleration, 1e-5): speed 6 m/s, deceleration 20 m/s^2 ->", max(6 / 20, 1e-5), "s; standing ->", 1e-5, "s")
    print("  spawnee animation wait = max(length - lerp, 1e-5): length 2.0, lerp 0.2 ->", round(max(2.0 - 0.2, 1e-5), 3), "s; length 0.1, lerp 0.2 ->", 1e-5, "s")


if __name__ == "__main__":
    main()
