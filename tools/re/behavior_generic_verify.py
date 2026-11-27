"""Evidence for docs/re/behavior-generic-puppet.md (T01-152): the data behind the puppet side of the generic behaviours.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries).

It prints
  1. the 13 `CRunOverFoeAttack` records of the `.ep` files (the numbers the run-over behaviour really uses) and, for the 10
     `SBARunOverFoe` actions of the `.cb` files, the action's own three numbers next to the record they point at (the action's
     numbers are copies that the engine does not read: the spec shows that they differ);
  2. the 7 `SBALocalMove` actions with every member, and the vector length the spec's formula gives for each (forward ratio
     clamped to `balm_fVelocityRatioClamp`);
  3. the `SBAAnimation`, `CBAWait`, `CBACrouch` and `SBALeapOntoFoe` actions (counts, flags, ids);
  4. `fad_fAutoChangeFoeChance` of the 82 character behaviours (the chance of re-choosing the foe after an attack behaviour ends).

Usage: python -I tools/re/behavior_generic_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR; about 1 minute)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Enum, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402

NULL = 0xFFFFFFFF


def members(o, cls):
    return {k.split(".", 1)[1]: v for k, v in o if k.startswith(cls + ".")}


def walk(o, out):
    if isinstance(o, Obj):
        out.append(o)
        for _, v in o:
            walk(v, out)
    elif isinstance(o, list):
        for x in o:
            walk(x, out)


def load(d):
    r = parse(d)
    objs, _ = TreeDec(d, r).objects()
    return r, objs


def val(v, idnt):
    if isinstance(v, Enum):
        return v.v
    if isinstance(v, float):
        return round(v, 4)
    return v


def main(install):
    print("== 1. CRunOverFoeAttack records (.ep) and SBARunOverFoe actions (.cb)")
    recs = {}
    for name, d in sorted(merged_files(install, ".ep").items()):
        r = parse(d)
        if not any(t.name == "CRunOverFoeAttack" for t in r["types"].values()):
            continue
        r, objs = load(d)
        allo = []
        for o in objs.values():
            walk(o, allo)
        seen = set()
        for o in allo:
            if isinstance(o, Obj) and o.type == "CRunOverFoeAttack" and id(o) not in seen:
                seen.add(id(o))
                m = members(o, "CRunOverFoeAttack")
                idn = lambda v: r["idnt"][v] if isinstance(v, int) and 0 <= v < len(r["idnt"]) else v
                row = dict(id=idn(m["rof_idName"]), speed=val(m["rof_fSpeedMultiplier"], None), tmin=val(m["rof_fRunOverFoeTimeMin"], None),
                           tmax=val(m["rof_fRunOverFoeTimeMax"], None), thru=val(m["rof_fRunTroughDistance"], None),
                           kickdist=val(m["rof_fKickAnimDistance"], None), kick=idn(m["rof_idKickAnim"]), wait=val(m["rof_fPostAttackWait"], None),
                           blasts=len(m["rof_cmbMeleeBlasts"]))
                recs.setdefault(row["id"], []).append((name.rsplit("/", 1)[-1], row))
                print(f"   {name.rsplit('/', 1)[-1]:34s} id={row['id']:12s} speed={row['speed']} time={row['tmin']}..{row['tmax']} "
                      f"runThrough={row['thru']} kickDist={row['kickdist']} kick={row['kick']} postWait={row['wait']} blasts={row['blasts']}")
    print("   SBARunOverFoe actions in .cb files:")
    for name, d in sorted(merged_files(install, ".cb").items()):
        r = parse(d)
        if not any(t.name == "SBARunOverFoe" for t in r["types"].values()):
            continue
        r, objs = load(d)
        allo = []
        for o in objs.values():
            walk(o, allo)
        seen = set()
        for o in allo:
            if isinstance(o, Obj) and o.type == "SBARunOverFoe" and id(o) not in seen:
                seen.add(id(o))
                m = members(o, "SBARunOverFoe")
                print(f"   {name.rsplit('/', 1)[-1]:34s} id={r['idnt'][m['rof_idRunOverFoe']]:12s} action time={val(m['rof_fRunOverFoeTimeMin'], None)}..{val(m['rof_fRunOverFoeTimeMax'], None)} "
                      f"runThrough={val(m['rof_fRunTroughDistance'], None)}")
    print("== 2. SBALocalMove actions and the move vector length")
    n = 0
    clamp_hits = collections.Counter()
    for name, d in sorted(merged_files(install, ".cb").items()):
        r = parse(d)
        if not any(t.name == "SBALocalMove" for t in r["types"].values()):
            continue
        r, objs = load(d)
        allo = []
        for o in objs.values():
            walk(o, allo)
        seen = set()
        for o in allo:
            if isinstance(o, Obj) and o.type == "SBALocalMove" and id(o) not in seen:
                seen.add(id(o))
                m = members(o, "SBALocalMove")
                n += 1
                fwd, up, clamp = m["balm_fFwdVelocityRatio"], m["balm_fUpVelocityRatio"], m["balm_fVelocityRatioClamp"]
                length = (fwd * fwd + up * up) ** 0.5          # forward and up are orthogonal axes of the formula
                eff = min(length, clamp)
                clamp_hits["clamped" if length > clamp else "not clamped"] += 1
                print(f"   {name.rsplit('/', 1)[-1]:24s} heading={val(m['balm_fHeading'], None)} fwd={val(fwd, None)} up={val(up, None)} clamp={val(clamp, None)} "
                      f"=> |v|={round(eff, 4)}  forcePose={val(m['balm_bForcePose'], None)} pose={val(m['balm_mapPose'], None)} "
                      f"forceReady={val(m['balm_bForceReadiness'], None)} noStrafe={val(m['balm_bDisableStrafing'], None)} lookAhead={val(m['balm_bLookAhead'], None)} "
                      f"waitBefore={val(m['balm_fWaitBeforeMove'], None)} step={val(m['balm_fAutoOffTime'], None)} waitAfter={val(m['balm_fWaitAfterMove'], None)}")
    print(f"   {n} SBALocalMove objects; {dict(clamp_hits)}")
    print("== 3. other generic actions")
    cnt = collections.Counter()
    ids = collections.defaultdict(collections.Counter)
    for name, d in sorted(merged_files(install, ".cb").items()):
        r = parse(d)
        wanted = {"SBAAnimation", "CBAWait", "CBACrouch", "SBALeapOntoFoe", "SBAMelee"}
        if not ({t.name for t in r["types"].values()} & wanted):
            continue
        r, objs = load(d)
        allo = []
        for o in objs.values():
            walk(o, allo)
        seen = set()
        for o in allo:
            if isinstance(o, Obj) and o.type in wanted and id(o) not in seen:
                seen.add(id(o))
                cnt[o.type] += 1
                m = members(o, o.type)
                if o.type == "SBAAnimation":
                    ids[o.type]["moveDuring=%s" % val(m["baa_bMoveDuringAnimation"], None)] += 1
                if o.type == "CBAWait":
                    ids[o.type]["moveDuring=%s" % val(m["baw_bMoveDuringWait"], None)] += 1
                    ids[o.type]["period %s..%s" % (val(m["baw_fMinPeriod"], None), val(m["baw_fMaxPeriod"], None))] += 1
                if o.type == "SBALeapOntoFoe":
                    ids[o.type][r["idnt"][m["lof_idLeapAttack"]]] += 1
    for k in sorted(cnt):
        print(f"   {k}: {cnt[k]} objects  {dict(ids[k])}")
    print("== 4. fad_fAutoChangeFoeChance of the character behaviours")
    ch = collections.Counter()
    for name, d in sorted(merged_files(install, ".cb").items()):
        r, objs = load(d)
        cb = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CCharacterBehavior")
        fad = members(cb, "CCharacterBehavior")["cb_fadFoeAsigningDescriptor"]
        ch[val(members(fad, "CFoeAsigingDescriptor")["fad_fAutoChangeFoeChance"], None)] += 1
    print(f"   values (files): {dict(sorted(ch.items()))}  total {sum(ch.values())}")


if __name__ == "__main__":
    inst = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("SS2_DATA_DIR")
    if not inst:
        sys.exit("usage: behavior_generic_verify.py DATA_DIR")
    main(inst)
