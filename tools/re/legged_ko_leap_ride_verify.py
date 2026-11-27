"""Knock-out, leap attack and ride-and-rider records of the `.ep` files (T01-147, read-only, data only).

Prints the tables of `docs/re/puppet-legged-sequences.md`: the knock-out parameters of every legged parameter file,
the nine leap attacks with all members, the seat data and the ride-and-rider animation pairs, and evaluates the
formulas of the page (leap launch tempo, leap look angles, knock-out duration, ride lerp factor) for its test vectors.

Usage: python -I tools/re/legged_ko_leap_ride_verify.py [DATA_DIR]      (or set SS2_DATA_DIR)
Standard library only; read-only on the install; prints numbers and identifiers of the data.
"""
import collections, math, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, merged_files, parse  # noqa: E402

NULL = 0xFFFFFFFF


def val(o, suffix):
    for k, v in o:
        if k.endswith(suffix):
            return v
    return None


def walk(o, objs, seen, out):
    if id(o) in seen:
        return
    seen.add(id(o))
    out.append(o)
    for _, v in o:
        for x in (v if isinstance(v, list) else [v]):
            if isinstance(x, Obj):
                walk(x, objs, seen, out)
            elif isinstance(x, Ref) and x.oid != NULL and isinstance(objs.get(x.oid), Obj):
                walk(objs[x.oid], objs, seen, out)


def hist(vals, n=8):
    c = collections.Counter(vals)
    return ", ".join(f"{k} x{v}" for k, v in c.most_common(n)) + (f" ... ({len(c)} distinct)" if len(c) > n else "")


def ident(r, v):
    return r["idnt"][v] if isinstance(v, int) and 0 <= v < len(r["idnt"]) else "?"


def fl(v):
    return round(v, 4) if isinstance(v, float) else v


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder or set SS2_DATA_DIR")
    by = collections.defaultdict(list)
    files = merged_files(install, ".ep")
    for name, d in sorted(files.items()):
        r = parse(d)
        objs, _ = TreeDec(d, r).objects()
        seen, allo = set(), []
        for o in objs.values():
            if isinstance(o, Obj) and o.type != "CResourceFile":
                walk(o, objs, seen, allo)
        for o in allo:
            by[o.type].append((name.split("databases/")[-1], o, r, objs))
    print(len(files), ".ep files")

    print("\n== 1. SKnockOutParams (member of every SLeggedPuppetParams)")
    ko = [(n, o) for n, o, r, objs in by["SKnockOutParams"]]
    print("  records:", len(ko), "in", len({n for n, _ in ko}), "files")
    # the knock-out parameters are an inline struct of the legged parameter object: find them in the root objects
    rows = []
    for name, d in sorted(files.items()):
        r = parse(d)
        objs, _ = TreeDec(d, r).objects()
        for o in objs.values():
            if isinstance(o, Obj) and o.type.endswith("LeggedPuppetParams"):
                k = val(o, "lpp_kopKnockOutParams")
                if isinstance(k, Obj):
                    rows.append((name.split("databases/")[-1], [x for _, x in k]))
                break
    print("  legged parameter files with the member:", len(rows))
    print("  kop_bCanBeInKO:", hist([r_[1][0] for r_ in rows]))
    print("  kop_fKOKickImpulse (all):", hist([r_[1][1] for r_ in rows], 12))
    print("  kop_fKOSleep (all):", hist([fl(r_[1][2]) for r_ in rows]))
    can = [r_ for r_ in rows if r_[1][0]]
    print("  files that can be knocked out:", len(can), "| impulse:", hist([r_[1][1] for r_ in can], 12), "| sleep:", hist([fl(r_[1][2]) for r_ in can]))
    for n, v in can:
        print("    ", n, v)

    print("\n== 2. CLeapAttack (9 records)")
    names = ["la_idName", "la_pfPlayerFire", "la_fForwardVelocityMultiplier", "la_fUpVelocityImpulse", "la_idAnimation", "la_bLoopAnimation", "la_slInflictedDamage", "la_dtDamageType",
             "la_fLaunchHeading", "la_fLaunchPitch", "la_fFoeKickImpulse", "la_fRunAwayTime", "la_bRotateTowardsFoeDuringWait", "la_idBeforeRotateAnimation",
             "la_fBeforeAnimationProbability", "la_idAfterRotateAnimation", "la_fAfterAnimationProbability", "la_fWaitAfterAttackTime"]
    for n, o, r, objs in by["CLeapAttack"]:
        vals = [x for _, x in o]
        row = {}
        for nm, v in zip(names, vals):
            row[nm] = ident(r, v) if nm.startswith("la_id") else (getattr(v, "v", v) if not isinstance(v, float) else fl(v))
        print("  ", n, row)
    la = by["CLeapAttack"]
    print("  forward multiplier:", hist([fl(val(o, "la_fForwardVelocityMultiplier")) for _, o, _, _ in la]), "| up impulse:", hist([fl(val(o, "la_fUpVelocityImpulse")) for _, o, _, _ in la]),
          "| damage:", hist([val(o, "la_slInflictedDamage") for _, o, _, _ in la]), "| run away:", hist([fl(val(o, "la_fRunAwayTime")) for _, o, _, _ in la]),
          "| rotate during wait:", hist([val(o, "la_bRotateTowardsFoeDuringWait") for _, o, _, _ in la]), "| wait after:", hist([fl(val(o, "la_fWaitAfterAttackTime")) for _, o, _, _ in la]))

    print("\n== 3. Seat data, ways to seat, ride-and-rider pairs")
    sd = by["SPuppetSeatData"]
    print("  seat data records:", len(sd), "in", len({n for n, _, _, _ in sd}), "files")
    print("  seat flags:", hist([getattr(val(o, "psd_sfSeatFlags"), "v", val(o, "psd_sfSeatFlags")) for _, o, _, _ in sd]))
    print("  attachment:", hist([ident(r, val(o, "psd_idSeatAttachment")) for _, o, r, _ in sd]), "| rider mechanism:", hist([ident(r, val(o, "psd_idRiderMechanism")) for _, o, r, _ in sd]))
    print("  sitting animation:", hist([ident(r, val(o, "psd_idSittingAnim")) for _, o, r, _ in sd]), "| rider head turn:", hist([val(o, "psd_bRiderCanTurnHead") for _, o, _, _ in sd]))
    ws = by["SWayToSeat"]
    print("  ways to seat:", len(ws), "| enter point:", hist([ident(r, val(o, "wts_idEnterPointAttachment")) for _, o, r, _ in ws]))
    rr = by["SRideAndRiderAnim"]
    print("  ride-and-rider pairs:", len(rr), "in", len({n for n, _, _, _ in rr}), "files | rider anim:", hist([ident(r, val(o, "rra_idRiderAnim")) for _, o, r, _ in rr]),
          "| ride anim:", hist([ident(r, val(o, "rra_idRideAnim")) for _, o, r, _ in rr]), "| lerp placement:", hist([val(o, "rra_bLerpRiderPlacement") for _, o, _, _ in rr]),
          "| reversed:", hist([val(o, "rra_bPlayReversed") for _, o, _, _ in rr]))
    seq = collections.Counter()
    for n, o, r, objs in ws:
        for k in ("wts_csoEnteringAnimSequence", "wts_csoExitingAnimSequence"):
            c = val(o, k)
            items = []
            if isinstance(c, Obj):
                for _, x in c:
                    if isinstance(x, list):
                        items = x
            seq[(k.split("_")[1][3:], len(items))] += 1
    print("  pairs per sequence:", dict(seq))

    print("\n== 4. Formula checks (test vectors of the page)")
    def unit(v):
        n = math.sqrt(sum(c * c for c in v))
        return [c / n for c in v] if n else [0.0, 0.0, 0.0]
    # leap launch of GenericLeapAttackState: tempo = unit(to foe) * fwd, y += up ; look from the tempo direction
    def leap(to_foe, fwd, up):
        d = unit(to_foe)
        t = [d[0] * fwd, d[1] * fwd + up, d[2] * fwd]
        u = unit(t)
        pitch = math.asin(max(-1.0, min(1.0, u[1])))
        heading = 0.0 if abs(u[1]) > 0.99 else math.atan2(-u[0], -u[2])
        return t, math.degrees(heading), math.degrees(pitch)
    for rec, to_foe in (((1.5, 0.4), (0, 0, -10)), ((2.0, 0.5), (3, 0, -4)), ((4.0, 1.0), (0, 2, -8))):
        t, h, p = leap(to_foe, *rec)
        print(f"  leap multiplier {rec[0]}, up {rec[1]}, foe at {to_foe}: tempo ({t[0]:.3f}, {t[1]:.3f}, {t[2]:.3f}), look heading {h:.2f} deg, pitch {p:.2f} deg")
    print("  knock-out duration = max(animation length, kop_fKOSleep) when alive: length 0.8, sleep 2 ->", max(0.8, 2.0), "; length 3.1, sleep 2 ->", max(3.1, 2.0))
    print("  KnockOut direction for macKnockOutAbs(heading, pitch, force):")
    for H, P in ((0, 0), (90, 0), (0, 45)):
        h, p = math.radians(H), math.radians(P)
        print(f"    heading {H}, pitch {P} -> ({-math.cos(p) * math.sin(h):.4f}, {math.sin(p):.4f}, {-math.cos(p) * math.cos(h):.4f})")
    for el, ln in ((0.0, 2.0), (1.0, 2.0), (2.5, 2.0)):
        print(f"  ride lerp factor: elapsed {el} s, animation length {ln} s -> {min(el / ln, 1.0)}")


if __name__ == "__main__":
    main()
