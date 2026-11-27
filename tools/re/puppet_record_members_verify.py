"""Names and values of the numbered members of the puppet record classes (T01-151; read-only on the install).

The DLL declares, for each record of a puppet parameter file, a member table with the real name of every
member in declaration order. The data stores some members by name and the rest by a bare number. This script
transcribes the declared names (words only, no offsets) and checks, over every puppet parameter file of the
install, that

  1. the members stored for each record are the declared members in the declared order (a declared member the
     data does not store is skipped) and every stored name agrees with the declared name at that position,
     so each bare number is the declared name at its position;
  2. the value types fit the declared types;
  3. the class constructors' default values (transcribed below) agree with the data (how many files keep the
     default), and the relations between members that the page states (for example that a minimum never
     exceeds its maximum).

Usage: python -I tools/re/puppet_record_members_verify.py [DATA_DIR]      (default: environment SS2_DATA_DIR)
Prints a summary; exit status 1 when a stated invariant fails. Standard library only.
"""
import collections
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, TreeDec, merged_files, parse  # noqa: E402

# Declared members in declaration order: (name, type word, class default or None). Names are the binary's own.
# Defaults are the values written by each class's default constructor (read from the DLL), None = no plain number.
DECLARED = {
    "SPuppetHealthParams": [
        ("php_slMaxHealth", "int", 100), ("php_slMaxArmor", "int", 0),
        ("php_slHealingStart", "int", 0), ("php_slHealingEnd", "int", 0), ("php_slHealingPerSec", "int", 0),
        ("php_fHealthBarSize", "float", 2.0), ("php_fHealthBarCullingDistance", "float", 1000.0),
        ("php_fHealthBarOffsetY", "float", 0.0), ("php_ghFallingHitDamage", "obj", None)],
    "SPuppetBodyParams": [
        ("pbp_strModel", "str", ""), ("pbp_pmcModel", "ref", None), ("pbp_fStretch", "float", 1.0),
        ("pbp_fDefaultAnimationLerpTime", "float", -1.0), ("pbp_vViewOrigin", "obj", None),
        ("pbp_qvShootOrigin", "obj", None), ("pbp_idAimOriginAttachment", "int", None),
        ("pbp_vViewOriginSitting", "obj", None), ("pbp_qvShootOriginSitting", "obj", None),
        ("pbp_vViewTarget", "obj", None), ("pbp_vShootTarget", "obj", None),
        ("pbp_bBonesMoveHulls", "int", 0), ("pbp_bForcesPiercingProjectileToExplode", "int", 0),
        ("pbp_fPiercingProjectileResilianceMultiplier", "float", 1.0), ("pbp_asdShields", "list", None),
        ("pbp_bAutoAimable", "int", 1)],
    "SPuppetDeathParams": [
        ("pdp_bExplodes", "int", 0), ("pdp_bDisableBulletDamageAccumulation", "int", 0),
        ("pdp_slExplodingDamage", "int", 0), ("pdp_fExplosionDamage", "float", 25.0),
        ("pdp_fExplosionFallOff", "float", 10.0), ("pdp_fExplosionHotSpot", "float", 5.0),
        ("pdp_fExplosionForceImpulse", "float", 2000.0), ("pdp_fExplosionForceImpulseUpRatio", "float", 0.5),
        ("pdp_pcscCorpseSinkConfig", "ref", None), ("pdp_fStretchOverride", "float", -1.0)],
    "SPuppetEffectParams": [
        ("pfx_fStepEffectStretch", "float", 1.0), ("pfx_fStepEffectPower", "float", 1.0),
        ("pfx_ulStepEffectPlatforms", "int", 0xFFFFFFFD),
        ("pfx_fPuppetSoundsFallOff", "float", -1.0), ("pfx_fPuppetSoundsHotSpot", "float", -1.0),
        ("pfx_fMovingSoundsFallOff", "float", -1.0), ("pfx_fMovingSoundsHotSpot", "float", -1.0),
        ("pfx_fStepSoundsFallOff", "float", -1.0), ("pfx_fStepSoundsHotSpot", "float", -1.0),
        ("pfx_ulStepSoundPlatforms", "int", 0xFFFFFFFD),
        ("pfx_fAliveEffectStretch", "float", 1.0), ("pfx_fAliveEffectPower", "float", 1.0)],
    "CMiscPupeptParams": [
        ("mpp_slScore", "int", 100), ("mpp_ptxRideIcon", "ref", None), ("mpp_ptxRideAmmoIcon", "ref", None),
        ("mpp_ptxBossIcon", "ref", None), ("mpp_strBossName", "obj", None),
        ("mpp_fSniperDamageMultiplier", "float", 1.0), ("mpp_esEnemySize", "int", 1)],
    "SPuppetEnvironmentParams": [
        ("pep_fAcceleration", "float", 200.0), ("pep_fDecceleration", "float", 40.0),
        ("pep_fSpeedMin", "float", 6.0), ("pep_fSpeedMax", "float", 6.0), ("pep_fWalkVelocity", "float", 2.5),
        ("pep_fMovingRotationMin", "float", 360.0), ("pep_fMovingRotationMax", "float", 360.0),
        ("pep_fStandingRotationMin", "float", 180.0), ("pep_fStandingRotationMax", "float", 180.0)],
    "SKnockOutParams": [
        ("kop_bCanBeInKO", "int", 1), ("kop_fKOKickImpulse", "int", 100000), ("kop_fKOSleep", "float", 2.0)],
}
# Where each record sits in a parameter file: (member tag of the owning class, record class).
SLOTS = [
    ("SPuppetParams.1", "SPuppetHealthParams"), ("SPuppetParams.2", "SPuppetBodyParams"),
    ("SPuppetParams.6", "SPuppetDeathParams"), ("SPuppetParams.pp_pfxEffectParams", "SPuppetEffectParams"),
    ("SPuppetParams.pp_mppMiscPuppetParams", "CMiscPupeptParams"),
    ("SLeggedPuppetParams.1", "SPuppetEnvironmentParams"), ("SLeggedPuppetParams.2", "SPuppetEnvironmentParams"),
    ("SLeggedPuppetParams.3", "SPuppetEnvironmentParams"), ("SLeggedPuppetParams.lpp_kopKnockOutParams", "SKnockOutParams"),
]


def kind(v):
    if isinstance(v, Obj):
        return "obj"
    if isinstance(v, bool) or isinstance(v, int):
        return "int"
    if isinstance(v, float):
        return "float"
    if isinstance(v, str):
        return "str"
    if isinstance(v, list):
        return "list"
    if v is None:
        return "ref"
    s = type(v).__name__
    return "ref" if s in ("Ref", "NullRef") else "int" if hasattr(v, "value") or "Platform" in str(v)[:9] or "MoveAbilities" in str(v) else "ref"


def num(v):
    """A plain number from a stored value (EPlatform and other flag types print as Name(0xHEX))."""
    if isinstance(v, (int, float)):
        return v
    s = str(v)
    if s.endswith(")") and "(0x" in s:
        return int(s[s.index("(0x") + 1:-1], 16)
    return None


GRAPHS = collections.Counter()


def collect(install):
    rows = []  # (file, root class, [(slot, recordclass, [(tag, value)])])
    global GRAPHS
    GRAPHS = collections.Counter()
    for name, d in sorted(merged_files(install, ".ep").items()):
        r = parse(d)
        objs, _ = TreeDec(d, r).objects()
        for o in objs.values():
            if not isinstance(o, Obj) or o.type == "CResourceFile":
                continue
            if not any(k.endswith("SPuppetParams.1") for k, _ in o):
                continue
            recs = []
            for k, v in o:
                for slot, cls in SLOTS:
                    if k == slot and isinstance(v, Obj):
                        recs.append((slot, cls, [(kk.split(".", 1)[1], vv) for kk, vv in v]))
                        for kk, vv in v:
                            if cls == "SPuppetHealthParams" and kk.endswith(".3") and isinstance(vv, Obj):
                                for k3, v3 in vv:
                                    g = objs.get(getattr(v3, "oid", -1))
                                    if isinstance(g, Obj):
                                        pts = []
                                        for a, b in g:
                                            if a.endswith(".1") and isinstance(b, list):
                                                for cp in b:
                                                    for a2, b2 in cp:
                                                        if a2.endswith(".3"):
                                                            pts.append(tuple(round(x[1], 3) for x in b2))
                                        GRAPHS[(g.type, tuple(pts))] += 1
            rows.append((name, o.type, recs))
            break
    return rows


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder or set SS2_DATA_DIR")
    rows = collect(install)
    fail = 0
    print("puppet parameter files:", len(rows))
    order_ok = collections.Counter()
    order_n = collections.Counter()
    type_bad = collections.Counter()
    vals = collections.defaultdict(list)       # (cls, declared name) -> values (all slots)
    bynum = collections.defaultdict(set)       # (cls, tag) -> declared names seen at that tag
    variants = collections.Counter()
    for name, root, recs in rows:
        for slot, cls, members in recs:
            decl = DECLARED[cls]
            order_n[cls] += 1
            # walk the stored members in order against the declared list (a declared member may be skipped)
            i = 0
            ok = True
            for tag, v in members:
                while i < len(decl) and tag.isdigit() is False and decl[i][0] != tag:
                    i += 1
                if i >= len(decl):
                    ok = False
                    break
                dn, dt, _ = decl[i]
                if tag.isdigit():
                    bynum[(cls, tag)].add(dn)
                k = kind(v)
                if k != dt and not (dt == "int" and num(v) is not None):
                    type_bad[(cls, dn)] += 1
                vals[(cls, dn)].append(num(v) if num(v) is not None else v)
                i += 1
            if ok:
                order_ok[cls] += 1
            variants[(cls, len(members))] += 1
    print("\n1. order and names (files x records whose stored members equal the declared order):")
    for cls in DECLARED:
        print("  %-26s %4d of %4d records; stored-member counts %s" % (cls, order_ok[cls], order_n[cls],
              sorted((n, c) for (c2, n), c in variants.items() if c2 == cls)))
        if order_ok[cls] != order_n[cls]:
            fail += 1
    print("\n   numbered tag -> declared name (every record, must be one name per tag):")
    for (cls, tag), names in sorted(bynum.items(), key=lambda x: (x[0][0], int(x[0][1]))):
        flag = "" if len(names) == 1 else "   <-- AMBIGUOUS"
        if flag:
            fail += 1
        print("  %-26s .%-3s = %s%s" % (cls, tag, ", ".join(sorted(names)), flag))
    print("\n2. type fit (stored values whose kind differs from the declared kind):", dict(type_bad) or "none")
    if type_bad:
        fail += 1
    print("\n3. class default against data (files keeping the default / records):")
    for cls, decl in DECLARED.items():
        for dn, dt, dflt in decl:
            vs = vals[(cls, dn)]
            if dflt is None or not vs:
                continue
            if isinstance(dflt, str):
                eq = sum(1 for x in vs if x == dflt)
                print("  %-26s %-44s default %-12r keeps %4d of %4d" % (cls, dn, dflt, eq, len(vs)))
                continue
            eq = sum(1 for x in vs if isinstance(x, (int, float)) and abs(x - dflt) < 1e-6)
            nums = [x for x in vs if isinstance(x, (int, float))]
            rng = "%g..%g" % (min(nums), max(nums)) if nums else "-"
            fmt = (lambda x: "0x%08X" % x) if isinstance(dflt, int) and dflt > 1 << 20 else (lambda x: "%g" % x)
            if isinstance(dflt, int) and dflt > 1 << 20 and nums:
                rng = "%s..%s" % (fmt(min(nums)), fmt(max(nums)))
            print("  %-26s %-44s default %-12s keeps %4d of %4d   range %s" % (cls, dn, fmt(dflt), eq, len(vs), rng))
    print("\n4. relations stated by the page (records checked / violations):")

    def rel(label, cls, a, b, op):
        va, vb = vals[(cls, a)], vals[(cls, b)]
        n = min(len(va), len(vb))
        bad = sum(1 for x, y in zip(va, vb) if isinstance(x, (int, float)) and isinstance(y, (int, float)) and not op(x, y))
        print("  %-62s %4d records, %d violations" % (label, n, bad))
        return bad

    fail += rel("env: pep_fSpeedMin <= pep_fSpeedMax", "SPuppetEnvironmentParams", "pep_fSpeedMin", "pep_fSpeedMax", lambda x, y: x <= y)
    fail += rel("env: pep_fMovingRotationMin <= pep_fMovingRotationMax", "SPuppetEnvironmentParams", "pep_fMovingRotationMin", "pep_fMovingRotationMax", lambda x, y: x <= y)
    fail += rel("env: pep_fStandingRotationMin <= pep_fStandingRotationMax", "SPuppetEnvironmentParams", "pep_fStandingRotationMin", "pep_fStandingRotationMax", lambda x, y: x <= y)
    rel("env: pep_fWalkVelocity <= pep_fSpeedMax (informational)", "SPuppetEnvironmentParams", "pep_fWalkVelocity", "pep_fSpeedMax", lambda x, y: x <= y)
    rel("death: pdp_fExplosionHotSpot <= pdp_fExplosionFallOff (informational)", "SPuppetDeathParams", "pdp_fExplosionHotSpot", "pdp_fExplosionFallOff", lambda x, y: x <= y)
    rel("health: php_slHealingEnd >= php_slHealingStart (informational)", "SPuppetHealthParams", "php_slHealingEnd", "php_slHealingStart", lambda x, y: x >= y)
    rel("health: php_slMaxArmor <= php_slMaxHealth (informational)", "SPuppetHealthParams", "php_slMaxArmor", "php_slMaxHealth", lambda x, y: x <= y)
    # exploding: files with pdp_bExplodes set
    ex = [(a, b, c) for a, b, c in zip(vals[("SPuppetDeathParams", "pdp_bExplodes")], vals[("SPuppetDeathParams", "pdp_slExplodingDamage")], vals[("SPuppetDeathParams", "pdp_fExplosionDamage")])]
    print("  death: pdp_bExplodes set in %d records; of those slExplodingDamage > 0 in %d, fExplosionDamage > 0 in %d; "
          "bExplodes clear but slExplodingDamage > 0 in %d" % (
              sum(1 for a, _, _ in ex if a), sum(1 for a, b, _ in ex if a and b > 0), sum(1 for a, _, c in ex if a and c > 0),
              sum(1 for a, b, _ in ex if not a and b > 0)))
    fl = vals[("SPuppetDeathParams", "pdp_bDisableBulletDamageAccumulation")]
    th = vals[("SPuppetDeathParams", "pdp_slExplodingDamage")]
    be = vals[("SPuppetDeathParams", "pdp_bExplodes")]
    print("  death: pdp_bDisableBulletDamageAccumulation set in %d records; of those bExplodes set in %d, slExplodingDamage > 0 in %d" % (
        sum(1 for f in fl if f), sum(1 for f, b in zip(fl, be) if f and b), sum(1 for f, t in zip(fl, th) if f and t > 0)))
    print("  death: slExplodingDamage values among the records with bExplodes set:", dict(sorted(collections.Counter(t for t, b in zip(th, be) if b).items())))
    print("  death: pdp_fExplosionDamage values among the records with bExplodes set:", dict(sorted(collections.Counter(x for x, b in zip(vals[("SPuppetDeathParams", "pdp_fExplosionDamage")], be) if b).items())))
    plat = collections.Counter(("0x%08X" % x) for k in ("pfx_ulStepEffectPlatforms", "pfx_ulStepSoundPlatforms") for x in vals[("SPuppetEffectParams", k)])
    print("  effects: platform masks (both members):", dict(plat))
    sizes = collections.Counter(vals[("CMiscPupeptParams", "mpp_esEnemySize")])
    print("  misc: mpp_esEnemySize stored in %d of %d records, values %s" % (len(vals[("CMiscPupeptParams", "mpp_esEnemySize")]), order_n["CMiscPupeptParams"], dict(sorted(sizes.items()))))
    hs = collections.Counter((a, b) for a, b in zip(vals[("SPuppetHealthParams", "php_slMaxArmor")], vals[("SPuppetHealthParams", "php_slMaxHealth")]) if a)
    print("  health: (php_slMaxArmor, php_slMaxHealth) pairs with armor > 0:", dict(sorted(hs.items())))
    heal = sorted({(a, b, c) for a, b, c in zip(vals[("SPuppetHealthParams", "php_slHealingStart")], vals[("SPuppetHealthParams", "php_slHealingEnd")], vals[("SPuppetHealthParams", "php_slHealingPerSec")]) if c})
    print("  health: (start, end, per second) triples with a rate > 0:", heal)
    print("  health: the falling-hit-damage graph of the records that store it (%d):" % sum(GRAPHS.values()), dict(GRAPHS))
    print("\nresult:", "FAIL (%d)" % fail if fail else "ok")
    return 1 if fail else 0


if __name__ == "__main__":
    sys.exit(main())
