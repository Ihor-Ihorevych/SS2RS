"""Evidence for docs/re/helper-classes*.md (T01-113): what the data says about the helper, event and global parameter classes of Sam2Game.dll.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries).

It prints
  1. the 59 exported classes of the family (names as exported, with their exported function counts) and which of them are in a type table
     of the data, under the exported name or as the `S` twin (30), plus the two classes the data stores under a different name;
  2. the animation events: how many events of each class the 88 `.ans` files and the models that embed an animation set hold, the
     class-specific member values of the 18 event classes of this family, and how the engine's defaults compare with the data;
  3. the global parameter files: the member lists of `CGlobalGameParams` and its nested records, and the defaults that the classes'
     constructors set (numbers read once from the binary, held below as constants) against the values the files carry;
  4. the members of the three records that the data stores under other names (`CThrowingParams` = `CObjectManipulationParams`,
     `SPuppetEnvironmentParams` = `CLPEnvironmentParams`, the session parameters = `CSamProjectParams`).

Usage: python -I tools/re/helper_classes_verify.py [DATA_DIR] [--sections=1234]      (DATA_DIR defaults to $SS2_DATA_DIR; about 3 minutes)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import class_catalog  # noqa: E402
from ctse_values import Enum, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402

# The 59 exported classes of the family with their exported function counts (quoted as exported).
FAMILY = {
    "iterators": {"CAddAllWeaponsIterator": 8, "CAddPastChaptersWeaponsIterator": 8, "CCollectChaptersForSortingIterator": 8,
                  "CCollectEnvironmentDamagersIterator": 8, "CCollectMissionObjectivesIterator": 8, "CCollectNavigationalBeaconsIterator": 8,
                  "CCollectSecretsIterator": 8, "CContributeToGameTotalStatsIterator": 8, "CCurrentStateInfoIterator": 8,
                  "CDisableMacrosIterator": 8, "CExecuteMacrosIterator": 8, "CFindFirstEntityOfTypeIterator": 8,
                  "CFindSpawnMarkerIterator": 8, "COnBootIterator": 8, "COnShutDownIterator": 8, "CPuppetsHealthDebugIterator": 8,
                  "CStateCallsInfoIterator": 8},
    "animation events": {"CAnimEventStep": 9, "CAttachToolAnimEvent": 8, "CDetachToolAnimEvent": 8, "CDrawToolAnimEvent": 8,
                         "CStoreToolAnimEvent": 8, "CHideAttachmentAnimEvent": 8, "CShowAttachmentAnimEvent": 8,
                         "CHidePlayerWeaponAttachmentAnimEvent": 9, "CShowPlayerWeaponAttachmentAnimEvent": 9,
                         "CMeleeDamageAnimEvent": 9, "CPuppetFireAnimEvent": 9, "CPuppetReleaseSpawneeAnimEvent": 9,
                         "CPuppetSoundAnimEvent": 7, "CPuppetAppearAnimEvent": 8, "CPuppetDisappearAnimEvent": 8,
                         "CPuppetExplodeAnimEvent": 8, "CRagdollAnimEvent": 9, "CThrowDeathToolsAnimEvent": 8},
    "handlers and records": {"CDoorAnimEventHandler": 9, "CPuppetAnimEventHandler": 12, "CStaticModelAnimEventHandler": 12,
                             "CWeaponAnimEventHandler": 11, "CStaticModelAnimEventsData": 9, "CEffectAnimEventToTerminate": 7,
                             "CSoundAnimEventToTerminate": 7},
    "parameter classes": {"CBulletParameters": 4, "CEntityParams": 13, "CGlobalGameParams": 7, "CGlobalSpawningParams": 7,
                          "CLPEnvironmentParams": 7, "CMaterialSoundRanges": 4, "CObjectManipulation": 6,
                          "CObjectManipulationParams": 6, "COperatingParams": 6, "CSamNetworkParams": 4, "CSamProjectParams": 7},
    "arrays and handles": {"CEntityArray": 16, "CEntityHandle": 4},
    "other": {"CLauncher": 11, "CTextToSay": 6, "CToolEntityCreator": 7, "CFeelerSetup": 3},
}
ALL = {n: c for g in FAMILY.values() for n, c in g.items()}
# Named (non-standard) exported methods, grouped by identical lists (export names; 43 classes, 80 methods).
NAMED = [
    (['ProcessEntity'], ['CAddAllWeaponsIterator', 'CAddPastChaptersWeaponsIterator', 'CCollectChaptersForSortingIterator', 'CCollectEnvironmentDamagersIterator', 'CCollectMissionObjectivesIterator', 'CCollectNavigationalBeaconsIterator', 'CCollectSecretsIterator', 'CContributeToGameTotalStatsIterator', 'CCurrentStateInfoIterator', 'CDisableMacrosIterator', 'CExecuteMacrosIterator', 'CFindFirstEntityOfTypeIterator', 'CFindSpawnMarkerIterator', 'COnBootIterator', 'COnShutDownIterator', 'CPuppetsHealthDebugIterator', 'CStateCallsInfoIterator']),
    (['Execute', 'GetDescription'], ['CAnimEventStep', 'CMeleeDamageAnimEvent', 'CPuppetFireAnimEvent', 'CPuppetReleaseSpawneeAnimEvent', 'CRagdollAnimEvent']),
    (['Execute'], ['CAttachToolAnimEvent', 'CDetachToolAnimEvent', 'CDrawToolAnimEvent', 'CHideAttachmentAnimEvent', 'CPuppetAppearAnimEvent', 'CPuppetDisappearAnimEvent', 'CPuppetExplodeAnimEvent', 'CShowAttachmentAnimEvent', 'CStoreToolAnimEvent', 'CThrowDeathToolsAnimEvent']),
    (['ProduceSound', 'TerminateSound'], ['CDoorAnimEventHandler']),
    (['Add', 'Clear', 'ClearNullEntries', 'Count', 'GetCountOfNullEntries', 'GetMember', 'Index', 'IsMember', 'RemoveByValue', 'RemoveOrderedByValue'], ['CEntityArray']),
    (['Clone', 'Copy', 'GetName', 'mdGetName', 'mdPostRead', 'mdScriptPostRead'], ['CEntityParams']),
    (['Execute', 'mdGetName'], ['CHidePlayerWeaponAttachmentAnimEvent', 'CShowPlayerWeaponAttachmentAnimEvent']),
    (['GetInstanceMoveSpeed'], ['CLPEnvironmentParams']),
    (['BreakPropertiesLinks', 'GetLaunchVelocity', 'SetupPreview', 'UpdateLinks'], ['CLauncher']),
    (['ProduceEffect', 'ProduceShakeEffect', 'ProduceSound', 'TerminateEffect', 'TerminateSound'], ['CPuppetAnimEventHandler', 'CStaticModelAnimEventHandler']),
    (['GetNextSoundSource', 'Initialize'], ['CStaticModelAnimEventsData']),
    (['ProduceEffect', 'ProduceSound', 'TerminateEffect', 'TerminateSound'], ['CWeaponAnimEventHandler']),
]

STANDARD = 7  # two constructors, destructor, two operators, one special function, mdGetDataType (smaller for plain structures)
# Exported class -> the name the data uses when it is neither the same nor the S twin (default values match, section 4).
OTHER_NAME = {"CObjectManipulationParams": "CThrowingParams", "CLPEnvironmentParams": "SPuppetEnvironmentParams"}

# Defaults that the constructors set, in declaration order (numbers read once from the binary).
DEF_NETWORK = [0.05, 5, 5, 0.2, 3, 0.05, 0.5, 5]                                   # CSamNetworkParams, 8 members
DEF_BULLET = [80, 160, 250, 0.5, 1]                                                  # CBulletParameters, first five members
DEF_STATIC_TRAIL = [1.0, 0.1, 0.7, 0.02, 0.04, 10.0]                                # CStaticBulletTrail, 6 members
DEF_MOVING_TRAIL = [10.0, 30.0, 0.1]                                                 # CMovingBulletTrail, 3 members
DEF_MATERIAL = [20, 5, 10, 5, 30, 10, 5, 30]                                         # CMaterialSoundRanges, 8 members
DEF_THROWING = [100, 5, 20, 10, 1, 30]                                               # CObjectManipulationParams, 6 members
DEF_ENVIRONMENT = [200, 40, 6, 6, 2.5, 360, 360, 180, 180]                           # CLPEnvironmentParams, 9 members
DEF_OPERATING = [0, 0, None, 0]                                                      # COperatingParams: bool, float, id (invalid), bool
DEF_TOOL_EVENTS = {"CAttachToolAnimEvent": 1.0, "CDrawToolAnimEvent": 0.5, "CStoreToolAnimEvent": 0.5}  # the period member of each
# CSamProjectParams: the constructor sets these 32 numeric members and leaves the two strings empty.
DEF_SESSION = [("sgp_esgaStartAs", 0), ("sgp_bContinuePlaying", 0), ("sgp_bDedicatedServer", 0), ("sgp_cbtChapterBootType", 0),
               ("sgp_gtGameType", 1), ("sgp_gdGameDifficulty", 3), ("sgp_bFriendlyFire", 0), ("sgp_iExtraEnemyStrength", 0),
               ("sgp_iEnemyStrengthPerPlayer", 50), ("sgp_ctMaxLives", 3), ("sgp_ctMaxPlayers", 2), ("sgp_ctPrivateSlots", 0),
               ("sgp_bVoiceRequred", 0), ("sgp_bPlayForStats", 0), ("sgp_bInfiniteAmmo", 0), ("sgp_bDisableSeriousBombs", 0),
               ("sgp_bWeaponStaysInCoop", 1), ("sgp_bAmmoStaysInCoop", 1), ("sgp_bHealthAndArmorStayInCoop", 1),
               ("sgp_ctFragsLimit", 20), ("sgp_ctTimeLimit", 10), ("sgp_bUseLives", 1), ("sgp_bRespawnInPlaceInCoop", 1),
               ("sgp_bEnableSprinting", 0), ("sgp_iEnemyMultiplier", -1), ("sgp_iEnemyMultiplierPerPlayer", -1),
               ("sgp_iEnemyMaxMultiplier", 100), ("sgp_bEnableRocketJumping", 0), ("sgp_iRespawnInvulnerability", -1),
               ("sgp_bUseComboWeapons", 0), ("sgp_iScoreForExtraLife", -1), ("sgp_fSpawnerMultiplier", 1.0)]
SESSION_MEMBERS = 34  # the member table named CSamTestRunParams: 32 numeric members above + sgp_strStartSaveGame + sgp_strStartChapterName


def twin(name):
    return [name] + (["S" + name[1:]] if name[0] == "C" else [])


def flat(o, prefix=""):
    out = {}
    for k, v in o:
        key = prefix + k.split(".", 1)[1]
        if isinstance(v, Obj): out.update(flat(v, key + "."))
        else: out[key] = v
    return out


def num(v):
    return v.v if isinstance(v, Enum) else v


def close(a, b):
    return abs(num(a) - num(b)) < 1e-4 * max(1.0, abs(b))


def nested(o):
    """Every Obj below (and including) o, in order."""
    if isinstance(o, Obj):
        yield o
        for _, v in o:
            yield from nested(v)
    elif isinstance(o, list):
        for v in o:
            yield from nested(v)


def decode(d):
    r = parse(d); objs, _ = TreeDec(d, r).objects(); return r, objs


def section1(cat):
    print(f"\n1. the {len(ALL)} exported classes ({sum(ALL.values())} exported functions), by group, and the data")
    in_data = []; other = []
    for grp, names in FAMILY.items():
        found = [n for n in names if any(t in cat for t in twin(n))]
        print(f"   {grp:22s} {len(names):3d} classes, {sum(names.values()):3d} functions, {len(found):2d} in the data")
        in_data += found
    for n in ALL:
        if n in OTHER_NAME and OTHER_NAME[n] in cat and n not in in_data: other.append(n)
    print(f"   in the data under the exported name or its S twin: {len(in_data)}; under another name (section 4): {len(other)} ({', '.join(other)})")
    named = {n: m for ms, ns in NAMED for n in ns for m in ms}
    nmeth = sum(len(ms) * len(ns) for ms, ns in NAMED); ncls = sum(len(ns) for _, ns in NAMED)
    plain = sum(1 for c in ALL.values() if c < STANDARD)
    print(f"   exported functions: {sum(ALL.values())} = {sum(ALL.values()) - nmeth} standard (two constructors, destructor, two operators, special, mdGetDataType; fewer on the {plain} plain structures) + {nmeth} named methods on {ncls} classes")
    print(f"   classes without a named method: {len(ALL) - ncls}")
    for n in ALL:
        t = [x for x in twin(n) if x in cat]
        if t:
            rec = cat[t[0]]; v = next(iter(rec["variants"]))
            print(f"   - {n:38s} -> {t[0]:36s} parent {str(v[0]):22s} {len(v[2]):3d} members, {rec['files']:4d} files {dict(rec['exts'])}")
    return in_data


def section2(install):
    print("\n2. animation events in the 88 .ans files and the models that embed an animation set")
    cls = collections.Counter(); vals = collections.defaultdict(collections.Counter); flags = collections.Counter(); first_loop = collections.Counter()
    nholders = 0
    for ext in (".ans", ".mdl"):
        for name, d in sorted(merged_files(install, ext).items()):
            r = parse(d)
            if ext == ".mdl" and not any(t.name == "CAnimSet" for t in r["types"].values()): continue
            objs, _ = TreeDec(d, r).objects()
            for a in (o for o in objs.values() if isinstance(o, Obj) and o.type == "CAnimation"):
                ev = a.get("CAnimation.8")
                if ev is None or ev.oid == 0xFFFFFFFF: continue
                nholders += 1
                for _, v in objs[ev.oid]:
                    if not isinstance(v, list): continue
                    for x in v:
                        o = objs[x.oid] if isinstance(x, Ref) else x
                        if not isinstance(o, Obj): continue
                        cls[o.type] += 1; f = flat(o)
                        flags[(o.type, num(f.get("ae_ulFlags")))] += 1; first_loop[(o.type, num(f.get("ae_bOnlyInFirstLoop")))] += 1
                        for k, v2 in f.items():
                            if k not in ("ae_fFrame", "ae_ulFlags", "ae_bOnlyInFirstLoop"):
                                vals[o.type][(k, repr(v2) if not isinstance(v2, float) else round(v2, 4))] += 1
    print(f"   animations with an event holder: {nholders}; events: {sum(cls.values())}; classes: {len(cls)}")
    ours = [n for n in ALL if n in cls or "S" + n[1:] in cls]
    print(f"   event classes of this family seen in the data: {len(ours)} of the 18 event classes")
    for n in FAMILY["animation events"]:
        t = n if n in cls else "S" + n[1:]
        if t not in cls:
            print(f"   {n:40s} not in any .ans or .mdl"); continue
        shown = sorted(vals[t].items(), key=lambda kv: -kv[1])[:4]
        print(f"   {n:40s} as {t:38s} {cls[t]:4d} events; flags {sorted((k[1], c) for k, c in flags.items() if k[0] == t)}; "
              f"first-loop only {sum(c for k, c in first_loop.items() if k[0] == t and k[1] == 1)}; class members (top values) {shown}")
    allflags = collections.Counter()
    for (t, fl), c in flags.items(): allflags[fl] += c
    print("   ae_ulFlags over all", sum(cls.values()), "events (value: count):", dict(sorted(allflags.items())))
    st = collections.Counter({k[1]: c for k, c in vals["SPuppetSoundAnimEvent"].items() if k[0] == "psae_psstSoundSourceType"})
    print("   SPuppetSoundAnimEvent.psae_psstSoundSourceType (value: count):", dict(sorted(st.items())))
    others = {k: v for k, v in cls.items() if k not in ALL and "C" + k[1:] not in ALL}
    print("   event classes of the data that are not in this family (base events of other parts):", dict(sorted(others.items(), key=lambda kv: -kv[1])))
    for n, dflt in DEF_TOOL_EVENTS.items():
        t = n
        if t in vals:
            ps = collections.Counter({k[1]: c for k, c in vals[t].items()})
            print(f"   {n}: constructor default of the period {dflt}; data values {dict(ps)}")


def section3(install, cat):
    print("\n3. global parameter classes: members of the data and the constructors' defaults")
    rsc = merged_files(install, ".rsc")
    key = next(k for k in rsc if k.endswith("globalgameparameters.rsc"))
    r, objs = decode(rsc[key])
    g = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CGlobalGameParams")
    f = flat(g)
    own = [t for t in r["types"].values() if t.name == "CGlobalGameParams"][0]
    print(f"   CGlobalGameParams: {len(own.members)} members in the type table; nested records flattened: {len(f)} values; objects in the file: {len(objs)}")
    kinds = collections.Counter(type(v).__name__ for k, v in g)
    print("   top-level member value kinds:", dict(kinds))
    sp = next(k for k in rsc if k.endswith("spawningparams.rsc")) if any(k.endswith("spawningparams.rsc") for k in rsc) else None
    if sp:
        r2, o2 = decode(rsc[sp]); s = next(o for o in o2.values() if isinstance(o, Obj) and o.type == "CGlobalSpawningParams"); fs = flat(s)
        print("   CGlobalSpawningParams: 8 members (6 of them graphs); gsp_iMinHealthForMaintainGroup =", num(fs.get("gsp_iMinHealthForMaintainGroup")), "(constructor default 1000); gsp_fFlyingPuppetMultiAdjustment =", round(num(fs.get("gsp_fFlyingPuppetMultiAdjustment")), 4))
    # nested records stored by value inside the global parameters
    netobj = next(v for k, v in g if k.endswith(".ggp_snpNetworkParams"))
    netv = [num(v) for _, v in netobj]
    print(f"   CSamNetworkParams in the data (member order): {[round(x, 4) for x in netv]}; the constructor sets {DEF_NETWORK}; equal member by member: {sum(close(a, b) for a, b in zip(netv, DEF_NETWORK))} of {len(DEF_NETWORK)}")
    bp = next(v for k, v in g if k.endswith(".ggp_bpBulletParams"))
    bpv = [v for _, v in bp]
    first5 = [num(v) for v in bpv[:5]]
    st = [num(v) for _, v in bpv[5]]; mv = [num(v) for _, v in bpv[6]]
    print(f"   CBulletParameters: {len(bpv)} members; first five {first5} (constructor {DEF_BULLET}: {sum(close(a, b) for a, b in zip(first5, DEF_BULLET))} of 5 equal); "
          f"static trail {[round(x, 4) for x in st]} (constructor {DEF_STATIC_TRAIL}); moving trail {[round(x, 4) for x in mv]} (constructor {DEF_MOVING_TRAIL})")
    ms = next(v for k, v in g if k.endswith(".gpp_msr")); msv = [num(v) for _, v in ms]
    print(f"   CMaterialSoundRanges in the data {msv}; constructor defaults {DEF_MATERIAL}: equal {sum(close(a, b) for a, b in zip(msv, DEF_MATERIAL))} of 8 (the file overrides the class defaults)")
    return f


def section4(install, cat):
    print("\n4. records the data stores under other names")
    eps = merged_files(install, ".ep")
    thr = collections.defaultdict(collections.Counter); nthr = 0
    env = collections.defaultdict(collections.Counter); nenv = 0; env_order = None; min_le_max = [0, 0]
    opr = collections.defaultdict(collections.Counter); nopr = 0
    for name, d in sorted(eps.items()):
        r = parse(d)
        names = {t.name for t in r["types"].values()}
        if not ({"SPuppetEnvironmentParams", "CThrowingParams", "SOperatingParams"} & names): continue
        objs, _ = TreeDec(d, r).objects()
        for o in (x for top in objs.values() for x in nested(top)):
            if o.type == "CThrowingParams":
                nthr += 1
                for i, (k, v) in enumerate(o): thr[i][round(num(v), 4)] += 1
            elif o.type == "SPuppetEnvironmentParams":
                nenv += 1
                if env_order is None: env_order = [k.split(".", 1)[1] for k, _ in o]
                vs = [num(v) for _, v in o]
                for i, v in enumerate(vs): env[i][round(v, 4)] += 1
                min_le_max[1] += 1; min_le_max[0] += vs[2] <= vs[3]
            elif o.type == "SOperatingParams":
                nopr += 1
                for i, (k, v) in enumerate(o): opr[i][repr(v) if isinstance(v, Ref) else (num(v) if not isinstance(v, str) else v)] += 1
    print(f"   CThrowingParams (data) = CObjectManipulationParams (exported): {nthr} instances; defaults {DEF_THROWING}; per member: the default is among the data values for "
          f"{sum(DEF_THROWING[i] in thr[i] for i in range(6))} of 6 members; files where a member equals the default: {[thr[i].get(DEF_THROWING[i], 0) for i in range(6)]}")
    print(f"   SPuppetEnvironmentParams (data) = CLPEnvironmentParams (exported): {nenv} instances; declared order {env_order}; constructor defaults in that order {DEF_ENVIRONMENT}")
    for i in range(9):
        top = env[i].most_common(3)
        print(f"      member {i + 1} ({env_order[i]}): default {DEF_ENVIRONMENT[i]} in {env[i].get(DEF_ENVIRONMENT[i], 0)} of {nenv}; most common {top}")
    print(f"      member 3 <= member 4 in {min_le_max[0]} of {min_le_max[1]} instances (the pair the exported GetInstanceMoveSpeed interpolates)")
    print(f"   SOperatingParams: {nopr} instances; per member (value: count): " + "; ".join(f"{i + 1}: {dict(opr[i])}" for i in range(4)))
    # the session parameters
    sess = cat.get("CSamTestRunParams"); print(f"   CSamTestRunParams (the session parameters) in the data: {'yes' if sess else 'no (binary only)'}; CSamProjectParams defaults held here: {len(DEF_SESSION)} numeric members of {SESSION_MEMBERS}")


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    only = next((a.split("=")[1] for a in sys.argv[1:] if a.startswith("--sections=")), "1234")
    cat, nfiles = class_catalog.classes_of(install)
    print(f"class tables of {nfiles} format-8 files, {len(cat)} classes")
    if "1" in only: section1(cat)
    if "2" in only: section2(install)
    if "3" in only: section3(install, cat)
    if "4" in only: section4(install, cat)


if __name__ == "__main__":
    main()
