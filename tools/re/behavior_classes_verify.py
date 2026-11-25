"""Evidence for docs/re/behavior-classes.md (T01-105): what the data says about the AI behaviour and action classes of Sam2Game.dll.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries).

It prints
  1. for each of the 95 exported classes of the family (names quoted from the export table of Sam2Game.dll): whether it is in the data
     (own name, or the `S` twin of a `C` name), its base class, own member count, number of files and file extensions;
  2. the data classes of the same subject that the DLL does not export (`CBADefaultAction`, `CBANoAction`, `CBAValidityTerm`, ...);
  3. the member-order check: for the 14 data classes whose member table the binary also declares, the data's members in file order against
     the binary's declared names (names only, quoted from the binary): same count, same names, same order;
  4. the instances in the files: number of objects, number of files per extension, for every family class and the data classes of 2;
  5. the stored values of every data member (distinct values, range, commonest) for the classes of 1 and 2, with enumeration values and
     identifiers named, and the usage relations the spec states (which actions carry a validity term, which movers are used where).

Usage: python -I tools/re/behavior_classes_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR; about 6 minutes)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Enum, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
import weapon_classes_verify as wcv  # noqa: E402

# The 95 classes exported by Sam2Game.dll that make up this family (T01-026 split, part T01-105).
FAMILY = [
    "CBAAnimation", "CBAChangeAligment", "CBAChangeMovingAbilities", "CBAChangeState", "CBACrouch", "CBADie", "CBAEmotion", "CBAFoeAssigning",
    "CBAForceMovingType", "CBAForcePlayerFoe", "CBALeapOntoFoe", "CBALocalMove", "CBALoseFoe", "CBAMelee", "CBAPlaySound", "CBAReuseTactic",
    "CBARunOverFoe", "CBASetActionSet", "CBAShieldStatus", "CBAShoot", "CBAStopMoving", "CBAStopTactic", "CBAStopUsingTactic",
    "CBAThrowMacroEvent", "CBAThrowPickedTool", "CBATryRiding", "CBAUnforceMovingType", "CBAVTCurrentStateDuration", "CBAVTFoeAngle",
    "CBAVTFoeIsPlayer", "CBAVTHasFoe", "CBAVTHasFoeCandidate", "CBAVTHasTool", "CBAVTHealth", "CBAVTKnockOut", "CBAVTMeleeSuccess",
    "CBAVTPlayerDistance", "CBAVTWarMeasure", "CBAWait", "CBaseGenericBehaviorEntity", "CBaseGenericBehaviorProperties", "CBehaviorEntity",
    "CBehaviorProperties", "CExitFromRideEntity", "CExitFromRideProperties", "CFoeManagerEntity", "CFoeManagerProperties",
    "CGBAAnimationBehaviorEntity", "CGBAAnimationBehaviorProperties", "CGBALeapOntoFoeBehaviorEntity", "CGBALeapOntoFoeBehaviorProperties",
    "CGBALocalMoveBehaviorEntity", "CGBALocalMoveBehaviorProperties", "CGBAMeleeBehaviorEntity", "CGBAMeleeBehaviorProperties",
    "CGBARunOverFoeBehaviorEntity", "CGBARunOverFoeBehaviorProperties", "CGBAShootBehaviorEntity", "CGBAShootBehaviorProperties",
    "CGBAWaitBehaviorEntity", "CGBAWaitBehaviorProperties", "CGBActionListBehaviorEntity", "CGBActionListBehaviorProperties",
    "CGoToPointByPathEntity", "CGoToPointByPathProperties", "CGoToPointEntity", "CGoToPointProperties", "CMCAirAttack", "CMCAwayFromFoe",
    "CMCCircleAroundFoe", "CMCDiamond", "CMCDontMove", "CMCDummy", "CMCMarkersMovingClosestToFoe", "CMCMarkersMovingOrdered",
    "CMCMarkersMovingRandom", "CMCMellon", "CMCPatternMoving", "CMCStayInArea", "CMCTowardsFoe", "CMNavigationMovingType",
    "CPossessRideEntity", "CPossessRideProperties", "CRideFoeManagerEntity", "CRideFoeManagerProperties", "CTacticEntity", "CTacticProperties",
    "CThinkPointOscilatorEntity", "CThinkPointOscilatorProperties", "CTurretBehaviorEntity", "CTurretBehaviorProperties", "SBAList",
    "SBehaviorAction", "SDynamicMoveData", "SMoveCharacter"]
# Data classes of the same subject that the DLL does not export under any name of the family.
EXTRA = ["CBehaviorAction", "CBAValidityTerm", "CBAVTTermCombiner", "CBADefaultAction", "CBANoAction", "CBAList", "CBehaviorActionList",
         "CBAPlayerAssigning", "CMoveCharacter", "CAIMomentHandler", "CCustomBehaviorArea"]

# Member names that the binary's member tables declare, in declaration order (the binary's own identifiers), for the data classes
# whose table the binary also declares.
BIN = {
    "SBAAnimation": ["baa_idAnimation", "baa_bMoveDuringAnimation"], "SBAChangeState": ["bacs_idNewState"],
    "SBAEmotion": ["bae_ceEmotion", "bae_fAutoOffTime"], "SBAShieldStatus": ["bass_cssChieldStatus", "bass_fAutoOffTime"],
    "SBAThrowMacroEvent": ["tme_idMAcroEvent"], "SBALeapOntoFoe": ["lof_idLeapAttack"],
    "SBALocalMove": ["balm_fHeading", "balm_fFwdVelocityRatio", "balm_fUpVelocityRatio", "balm_fVelocityRatioClamp", "balm_bForcePose",
                     "balm_mapPose", "balm_bForceReadiness", "balm_marReadiness", "balm_bDisableStrafing", "balm_bLookAhead",
                     "balm_fWaitBeforeMove", "balm_fAutoOffTime", "balm_fWaitAfterMove"],
    "SBAMelee": ["mel_idMeleeAttack"], "CBAPlayerAssigning": ["pa_bAllowPlayerToBeFoe", "pa_bAllowNonPlayerToBeFoe"],
    "SBARunOverFoe": ["rof_idRunOverFoe", "rof_fRunOverFoeTimeMin", "rof_fRunOverFoeTimeMax", "rof_fRunTroughDistance"],
    "SBAShoot": ["as_idShoot", "sp_pgrShootWhileMovingOverDistanceRatio", "sp_pgrShootWhileMovingOverAreaRatio", "sp_ctMinShoots",
                 "sp_ctMaxShoots", "sp_pgrShootCountMultiplierOverDistanceRatio", "sp_pgrShootCountMultiplierOverAreaRatio"],
    "SBAVTHealth": ["vth_hoObservation", "vth_slValue", "vth_bAccumulative"],
    "STacticProperties": ["ep_hmcCharacterMover", "ep_hepNextTactic", "ep_hepTacticMarker", "ep_fAutoOffTime", "ep_fAutoOffDistance",
                          "ep_bStopTacticWhenHasFoe", "ep_colColor"],
    "STacticEntity": ["en_hre", "en_hreChild", "en_hmcCharacterMover", "en_fAutoOffTime", "en_fAutoOffDistance", "en_bStopTacticWhenHasFoe",
                      "en_henNextTactic", "en_henTacticMarker"],
}
# Classes the binary registers with a member table but that no file stores (member counts quoted for the spec).
BINARY_ONLY = {"SBehaviorProperties": 1, "SBehaviorEntity": 3, "GoToPointProperties": 4, "GoToPointEntity": 4, "GoToPointByPathProperties": 6,
               "GoToPointByPathEntity": 10, "SPossessRideProperties": 2, "SPossessRideEntity": 9, "SFoeManagerProperties": 1,
               "SFoeManagerEntity": 20, "SBASetActionSet": 1, "SBAChangeAligment": 1, "SGBActionListBehaviorProperties": 1,
               "SGBActionListBehaviorEntity": 2, "SGBAAnimationBehaviorProperties": 1, "SBaseGenericBehaviorEntity": 1,
               "SBAForceMovingType": 2, "SGBALeapOntoFoeBehaviorProperties": 1, "SGBALeapOntoFoeBehaviorEntity": 2,
               "SGBALocalMoveBehaviorProperties": 1, "SGBAMeleeBehaviorProperties": 1, "SGBARunOverFoeBehaviorProperties": 7,
               "SGBARunOverFoeBehaviorEntity": 13, "SGBAShootBehaviorProperties": 3}
NULL = 0xFFFFFFFF
EXTS = (".cb", ".ep", ".wld")


def twin(name):
    return [name] + (["S" + name[1:]] if name[0] == "C" else [])


def scan(install, wanted_types):
    """Every object of the wanted types in .cb/.ep/.wld: counts, files, member values (identifiers resolved), container sizes."""
    inst = collections.Counter(); byext = collections.defaultdict(collections.Counter); files = collections.defaultdict(collections.Counter); vals = collections.defaultdict(lambda: collections.defaultdict(collections.Counter))
    terms = collections.Counter(); nofiles = collections.Counter()
    for ext in EXTS:
        for name, d in sorted(merged_files(install, ext).items()):
            r = parse(d); nofiles[ext] += 1
            if not ({t.name for t in r["types"].values()} & wanted_types): continue
            objs, _ = TreeDec(d, r).objects(); idnt = r["idnt"]
            identk = {(t.name, m) for t in r["types"].values() if t.members for m, mt in t.members if r["types"].get(mt) and r["types"][mt].name == "IDENT"}
            seen = set()

            def visit(o):
                if o.type not in wanted_types: return
                inst[o.type] += 1; byext[o.type][ext] += 1; seen.add(o.type)
                for k, v in o:
                    cls, _, mem = k.partition(".")
                    if isinstance(v, Ref): vals[o.type][mem]["null" if v.oid == NULL else "ref"] += 1
                    elif isinstance(v, Obj): vals[o.type][mem]["<" + v.type + ">"] += 1
                    elif isinstance(v, list): vals[o.type][mem]["list[%d]" % len(v)] += 1
                    elif isinstance(v, Enum): vals[o.type][mem]["enum %d" % v.v] += 1
                    elif isinstance(v, float): vals[o.type][mem][round(v, 4)] += 1
                    elif isinstance(v, int):
                        vals[o.type][mem][idnt[v] if (cls, mem) in identk and 0 <= v < len(idnt) else v] += 1
                    elif isinstance(v, str): vals[o.type][mem][v] += 1
                if o.type == "CBAVTTermCombiner": pass

            def walk(o):
                for k, v in o:
                    for x in (v if isinstance(v, list) else [v]):
                        if isinstance(x, Obj): visit(x); walk(x)
            for o in objs.values():
                if isinstance(o, Obj): visit(o); walk(o)
            for t in seen: files[t][ext] += 1
    return inst, files, vals, nofiles, byext


def rng(c):
    nums = [v for v in c if isinstance(v, (int, float)) and not isinstance(v, bool)]
    return f"{min(nums):g} .. {max(nums):g}" if nums and len(nums) == len(c) else "-"


def action_relations(install):
    """In the .cb files: which term class each action class carries (`ba_pcValidityTerm`), the operands of the term combiners, the movers of the bands."""
    per = collections.defaultdict(collections.Counter); comb = collections.Counter(); movers = collections.Counter(); weights = collections.Counter()
    for name, d in sorted(merged_files(install, ".cb").items()):
        r = parse(d); objs, _ = TreeDec(d, r).objects()
        for o in objs.values():
            if not isinstance(o, Obj): continue
            for k, v in o:
                if k == "CBehaviorAction.ba_pcValidityTerm":
                    per[o.type]["null" if isinstance(v, Ref) and v.oid == NULL else (objs[v.oid].type if isinstance(v, Ref) and v.oid in objs else "?")] += 1
                elif k == "CBehaviorAction.ba_fWeight": weights[v] += 1
                elif k in ("CBAVTTermCombiner.tc_pTermA", "CBAVTTermCombiner.tc_pTermB") and isinstance(v, Ref) and v.oid in objs: comb[objs[v.oid].type] += 1
                elif k == "CCustomBehaviorArea.cba_pmcMoveCharacter":
                    movers["(null)" if isinstance(v, Ref) and v.oid == NULL else (objs[v.oid].type if isinstance(v, Ref) and v.oid in objs else "?")] += 1
    return per, comb, movers, weights


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    wanted = {n for c in FAMILY + EXTRA for n in twin(c)} | set(BIN) | set(BINARY_ONLY)
    cat, nfiles = wcv.collect(install, wanted)
    print(f"type tables of {nfiles} format-8 files")

    print(f"\n1. the {len(FAMILY)} exported classes of the family and the data")
    indata = 0
    for c in FAMILY:
        found = [n for n in twin(c) if n in cat]
        if not found: print(f"   {c:34s} not in the data"); continue
        n = found[0]; rec = cat[n]; (parent, mem), _ = rec["variants"].most_common(1)[0]
        exts = ",".join(f"{e.lstrip('.')}:{k}" for e, k in rec["exts"].most_common(3)); indata += 1
        print(f"   {c:34s} saved as {n:28s} base {str(parent):22s} {len(mem):2d} members, {rec['files']:3d} files ({exts}), {len(rec['variants'])} variant(s)")
    print(f"   in the data under the exported name or its S twin: {indata} of {len(FAMILY)}; "
          f"member tables the binary declares for classes no file stores: {len(BINARY_ONLY)}; any of them in the data: "
          f"{[n for n in BINARY_ONLY if n in cat]}")

    print("\n2. data classes of the same subject that the DLL does not export")
    for c in EXTRA:
        if c not in cat: print(f"   {c:26s} not in the data"); continue
        rec = cat[c]; (parent, mem), _ = rec["variants"].most_common(1)[0]
        exts = ",".join(f"{e.lstrip('.')}:{k}" for e, k in rec["exts"].most_common(3))
        print(f"   {c:26s} base {str(parent):20s} {len(mem):2d} members {[m for m, _ in mem]}, {rec['files']:3d} files ({exts})")

    print("\n3. member order: the data's members against the binary's declared names")
    ok = 0
    for cls, names in BIN.items():
        (parent, mem), _ = cat[cls]["variants"].most_common(1)[0]
        dn = [m for m, _ in mem]; same = dn == names; ok += same
        print(f"   {cls:22s} data {len(dn):2d}, binary {len(names):2d}: identical names in identical order: {same}; definitions in the data: {len(cat[cls]['variants'])}")
    print(f"   classes where the check holds: {ok} of {len(BIN)}")

    print("\n4. instances and files (objects found by walking .cb, .ep, .wld)")
    types = {n for c in FAMILY + EXTRA for n in twin(c) if n in cat}
    inst, files, vals, nofiles, byext = scan(install, types)
    print(f"   files read: {dict(nofiles)}")
    for t in sorted(types):
        print(f"   {t:34s} {inst.get(t, 0):6d} objects (" + ", ".join(f"{k.lstrip('.')}:{n}" for k, n in sorted(byext[t].items())) + ") in files: "
              + ", ".join(f"{k.lstrip('.')}:{n}" for k, n in sorted(files[t].items())))

    print("\n5. stored values per member: distinct values, range, commonest values (count)")
    for t in sorted(types):
        if not vals.get(t): continue
        print(f"   {t} ({inst[t]} objects)")
        for m, c in vals[t].items():
            if m.isdigit(): continue  # members of the base entity class that the files store by number only
            top = ", ".join(f"{v}({n})" for v, n in c.most_common(5))
            print(f"      {m:42s} {len(c):3d}  {rng(c):16s} {top}")

    print("\n6. relations in the .cb files")
    per, comb, movers, weights = action_relations(install)
    print("   term class carried by each action class (null = no term):")
    for a, c in sorted(per.items()): print(f"      {a:22s} {dict(c)}")
    print(f"   operands of the term combiners: {dict(comb)}")
    print(f"   movers of the distance bands: {dict(movers)}")
    print(f"   action weights: {dict(sorted(weights.items()))}")


if __name__ == "__main__":
    main()
