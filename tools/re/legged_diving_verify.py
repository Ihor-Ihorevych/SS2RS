"""Breath period of the legged puppets and the drowning numbers of docs/re/puppet-legged-diving.md (T01-149, read-only, data only).

Reads every `.ep` file of the install and prints the `pbp_fKeepBreathPeriod` of each puppet parameter file (the time a
character can stay under water), the health of those files, and evaluates the formulas of the page (the dive timers, the
inhale sound by the held fraction, the drowning damage) for its test vectors.

Usage: python -I tools/re/legged_diving_verify.py [DATA_DIR]      (or set SS2_DATA_DIR)
Standard library only; read-only on the install.
"""
import collections, math, os, sys
from fractions import Fraction

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, merged_files, parse  # noqa: E402


def find(o, suffix, out, objs, seen=None):
    seen = seen if seen is not None else set()
    if id(o) in seen:
        return
    seen.add(id(o))
    for k, v in o:
        if k.endswith(suffix):
            out.append(v)
        for x in (v if isinstance(v, list) else [v]):
            if isinstance(x, Obj):
                find(x, suffix, out, objs, seen)


def hist(vals, n=10):
    c = collections.Counter(vals)
    return ", ".join(f"{k} x{v}" for k, v in c.most_common(n))


def fl(v):
    return round(v, 4) if isinstance(v, float) else v


def inhale(r):
    """The inhale sound by the held fraction r = clamp(elapsed / breath period, 0, 1) (strict less-than)."""
    return "InhaleEasy" if r < 0.2 else ("InhaleMedium" if r < 0.8 else "InhaleHard")


def ticks(e, dt, k):
    """True when the periodic event of frequency k (per second) fires in the step that ends at elapsed time e (step length dt)."""
    return int(e * k) > int((e - dt) * k)


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder or set SS2_DATA_DIR")
    rows = []
    for name, d in sorted(merged_files(install, ".ep").items()):
        r = parse(d)
        objs, _ = TreeDec(d, r).objects()
        for o in objs.values():
            if isinstance(o, Obj) and o.type.endswith("PuppetParams") and o.type != "CResourceFile":
                bp, hp = [], []
                find(o, "lpp_lpbBody", bp, objs)
                find(o, "SPuppetParams.1", hp, objs)
                health = [x for _, x in hp[0]][0] if hp and isinstance(hp[0], Obj) else None
                if bp and isinstance(bp[0], Obj):
                    vals = [x for _, x in bp[0]]
                    rows.append((name.split("databases/")[-1], o.type, fl(vals[-1]), health))
                break
    print(len(rows), "puppet parameter files with a breath period")
    print("pbp_fKeepBreathPeriod:", hist([r_[2] for r_ in rows]))
    by = collections.defaultdict(list)
    for n, t, b, h in rows:
        by[b].append((n, t, h))
    for b, lst in sorted(by.items()):
        print(f"  {b} s: {len(lst)} files; root classes {hist([t for _, t, _ in lst])}; examples {[n for n, _, _ in lst[:6]]}")
    print("  files with a breath period of 0 or less:", sum(1 for r_ in rows if r_[2] <= 0))
    print("\n== formulas (test vectors)")
    for r_ in (0.0, 0.19, 0.2, 0.79, 0.8, 1.0):
        print(f"  held fraction {r_}: {inhale(r_)}")
    B, dt = Fraction(5), Fraction(1, 30)
    print(f"  breath period {float(B)} s, step {float(dt):.4f} s; events during a 12 s dive (steps where the periodic test fires):")
    n = int(12 / dt) + 1
    bub = [float(i * dt) for i in range(1, n) if ticks(i * dt, dt, Fraction(1, 3))]
    war = [float(i * dt) for i in range(1, n) if i * dt > Fraction(6666, 10000) * B and ticks(i * dt, dt, Fraction(1, 2))]
    dmg = [float(i * dt) for i in range(1, n) if i * dt > B and ticks(i * dt - B, dt, 1)]
    print("   bubbles (every 3 s of dive):", bub)
    print("   warning sound and effect (after 0.6666 x B = 3.333 s, every 2 s of dive):", war)
    print("   drowning damage (after B, once per second of overtime):", dmg)
    for mh in (100, 96, 5000, 9):
        print(f"  drowning damage per hit at max health {mh}: {int(mh / 10)}")


if __name__ == "__main__":
    main()
