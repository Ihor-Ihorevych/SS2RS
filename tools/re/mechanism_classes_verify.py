"""Evidence for docs/re/mechanism-classes*.md (T01-109): what the data says about the level mechanism, spawner and info entity classes of Sam2Game.dll.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries).

It prints
  1. for each of the 82 exported classes of the family (names quoted from the export table of Sam2Game.dll): whether it is in the data
     (own name, or the `S` twin of a `C` name), its base class, own member count, number of files and extensions; the data class `CTestJointEntity`
     (the data name of the exported `CExternalJointEntity`) is listed too;
  2. the member-order check: for every data class whose member table the binary also declares (29 classes), the data's members in file order
     against the binary's declared names (counts equal, every member the data names at the position of the same name, every data type fitting
     the Hungarian prefix of the binary's name);
  3. the census of the worlds: placed entities and property records per class, worlds, and for every member the most common values
     (one table per class), plus the cross-tabulations quoted by the specs (spawner limit type against spawnee class, spawning type, door states,
     chapter links, chapter-info chains, world-info layout variants);
  4. the spawner tweak tables (`spawnertweaks.rsc`, `spawnerboottweaks.rsc`): objects per class, levels, condition kinds;
  5. the text file `ChapterMaxes.txt` of the patches: levels, keys, value histogram.

Usage: python -I tools/re/mechanism_classes_verify.py [DATA_DIR] [--worlds N] [--short] [--no-worlds]
       (DATA_DIR defaults to $SS2_DATA_DIR; --worlds N = only the N smallest worlds; --short prints only the first five values per member;
       --no-worlds skips the census of the worlds (sections 1, 2, 4 and 5 only, about 2 minutes); the full run is about 12 minutes: 2 minutes for the type tables of all format-8 files, about 8 minutes for the 59 worlds)
"""
import collections, os, re, sys, zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Blob, Enum, Obj, Ref, TreeDec, _natural, merged_files, parse, u32  # noqa: E402
from ep_schema import load as load_eps, root_of  # noqa: E402

NULL = 0xFFFFFFFF

# The 82 classes exported by Sam2Game.dll that make up this family (T01-026 split, part T01-109).
FAMILY = """CAlarmEntity CAlarmProperties CAnimatedMoverEntity CAnimatedMoverEnvelopeProvider CAnimatedMoverProperties CBarrierFieldEntity
CBarrierFieldProperties CBouncerEntity CBouncerProperties CCameraEntity CCameraEnvelopeProvider CCameraProperties CChapterInfoEntity
CChapterInfoProperties CDetectorAreaEntity CDetectorAreaProperties CDiamondHandEntity CDiamondHandProperties CDoorEntity CDoorProperties
CEnvironmentDamagerEntity CEnvironmentDamagerProperties CExternalJointEntity CExternalJointProperties CGenericMotorEntity CGenericMotorProperties
CGlobalSpawnerBootTweaks CGlobalSpawnerTweakParams CKillerEntity CKillerProperties CLocatorEntity CLocatorProperties CMissionObjectiveData
CMissionObjectiveEntity CMissionObjectiveProperties CMultiSpawnerEntity CMultiSpawnerProperties CNavigationalBeaconData CNavigationalBeaconEntity
CNavigationalBeaconProperties CPlatformEntity CPlatformProperties CPortalEntity CPortalProperties CSecretEntity CSecretEntityData CSecretProperties
CSpawnDelayerEntity CSpawnDelayerProperties CSpawnMarkerEntity CSpawnMarkerProperties CSpawnerBootTweakCondition CSpawnerBootTweakType
CSpawnerBootTweaks CSpawnerEntity CSpawnerProperties CSpawnerTweakBarrierCollision CSpawnerTweakChangeSenseRange
CSpawnerTweakChangeThreatSensitivity CSpawnerTweakCondition CSpawnerTweakConditionLocation CSpawnerTweakConditionOwnerID
CSpawnerTweakConditionOwnerName CSpawnerTweakEnemySize CSpawnerTweakIgnoreCantMove CSpawnerTweakIgnoreMax CSpawnerTweakIncreaseMulti
CSpawnerTweakIncreaseY CSpawnerTweakModifyDelays CSpawnerTweakMustBeShooter CSpawnerTweakNoAutoPatrol CSpawnerTweakNoMulti CSpawnerTweakNoRandom
CSpawnerTweakNullTactics CSpawnerTweakParams CSpawnerTweakPatrolSpeed CSpawnerTweakPlacement CSpawnerTweakRandomCone CSpawnerTweakReduce
CSpawnerTweakType CWorldInfoEntity CWorldInfoProperties""".split()
EXTRA = ["CTestJointEntity", "CTestJointProperties"]

# Member names that the binary's member tables declare, in declaration order (the binary's own identifiers).
BIN = {
    "CTestJointEntity": [
        "en_pjoJoint", "en_preJoint", "en_henSrc", "en_idSrcPart", "en_henDst", "en_idDstPart", "en_qvPlacement",
        "en_qvRelTemplatePlacement", "en_pjtJoint", "en_idParentPart", "en_fBreakForce", "en_qvSrc", "en_qvDst"],
    "CTestJointProperties": [
        "ep_hepSrc", "ep_idSrcPart", "ep_hepDst", "ep_idDstPart", "ep_pjtJoint", "ep_idParentPart", "ep_fBreakForce"],
    "SAlarmEntity": [
        "en_hre", "en_fHeardDistance"],
    "SAlarmProperties": [
        "ep_fHeardDistance"],
    "SAnimatedMoverEntity": [
        "en_pbo", "en_idOrgAnimation", "en_peaiAnimInstance", "en_qvLastStep", "en_qvPlacement", "en_paehAnimEventHandler",
        "en_fPlayedAnimationLength", "en_mlidMacroLatentID", "en_amsState", "en_bIsAnimLooping", "en_idCurrentAnimation",
        "en_fCurrentAnimOffset", "en_fClientOffsetError", "en_qvDefault", "en_bRelativePath", "en_peacAnimConfig",
        "en_bNetIsAnimLooping", "en_idNetCurrentAnimation", "en_fNetCurrentAnimOffset"],
    "SAnimatedMoverEnvelopeProvider": [
        "m_mOwner"],
    "SAnimatedMoverProperties": [
        "ep_idAnimation", "ep_bRelativePath", "ep_peacAnimConfig"],
    "SBarrierFieldEntity": [
        "en_hmrModel", "en_hre", "en_hhuHull", "en_penParent", "en_idParentPart", "en_bEnabled", "en_bNetEnabled", "en_bCustom"],
    "SBarrierFieldProperties": [
        "ep_pmcModel", "ep_idParentPart", "ep_vStretch", "ep_bIgnoredByNavigation", "ep_bCustom"],
    "SCameraEntity": [
        "en_hre", "en_qvDefault", "en_fDefaultFOVFactor", "en_fDefaultAspectRatio", "en_qvDefaultParent", "en_henParent",
        "en_idDefaultAnimation", "en_idAnimation", "en_bRelativePath", "en_fAutoStopTime", "en_bEndCameraOnUserBreak",
        "en_heaiAnimInstance", "en_fFOVFactor", "en_fAspectRatio", "en_fFade", "en_qvPlacement", "en_bUseMaster", "en_bUseTarget",
        "en_bUseParentsOrientation", "en_mlidMacroLatentID", "en_htqTextQueue", "en_bLastCameraInterrupted", "en_csState",
        "en_hpaehAnimEventHandler"],
    "SCameraEnvelopeProvider": [
        "m_henOwner"],
    "SCameraProperties": [
        "ep_idAnimation", "ep_bRelativePath", "ep_bUseMaster", "ep_bUseTarget", "ep_fDefaultFOV", "ep_fDefaultAspect",
        "ep_peacAnimConfig", "ep_bUseParentsOrientation"],
    "SDoorEntity": [
        "en_pre", "en_pmcMechanism", "en_phuDetectionArea", "en_fClosingDelayTime", "en_idOpenAnim", "en_idCloseAnim",
        "en_bInitiallyLocked", "en_idMechanismName", "en_fAnimLen", "en_dsState", "en_dfFlags", "en_ulCurrentAnimFlags",
        "en_idCurrentAnimation", "en_fCurrentAnimOffset", "en_fClientOffsetError", "en_hdaehAnimEventHandler", "en_hssAnimEvents",
        "en_hssMoving", "en_csoSoundAnimEventsToTerminate", "en_bLocked", "en_ulNetCurrentAnimFlags", "en_idNetCurrentAnimation",
        "en_fNetCurrentAnimOffset", "en_dsNetState"],
    "SDoorProperties": [
        "ep_pmcModel", "ep_vStretch", "ep_bDetectorFront", "ep_bDetectorBack", "ep_fExtraArea", "ep_fSafetyOffset",
        "ep_fClosingDelayTime", "ep_idOpenAnim", "ep_idCloseAnim", "ep_idMechanismName", "ep_bLocked", "ep_smlModifiers",
        "ep_fCullDistance", "ep_fFadeLength", "ep_bPlayersOnly", "ep_fDoorSoundFallOff", "ep_fDoorSoundHotSpot"],
    "SKillerEntity": [
        "en_hre", "en_hhu", "en_henParent", "en_idParentPart", "en_bKillInstantly", "en_fDamagingPeriod", "en_slDamageAmount",
        "en_tmLastDamageApplied", "en_bAbyss"],
    "SKillerProperties": [
        "ep_pdPrimitive", "ep_idParentPart", "ep_bKillInstantly", "ep_fDamagingPeriod", "ep_slDamageAmount", "ep_bAbyss"],
    "SLocatorEntity": [
        "en_hre", "en_penParent", "en_idParentPart", "en_vStretch", "en_bSphere"],
    "SMissionObjectiveEntity": [
        "en_hre", "en_mosMissionObjectiveState", "en_mosNetMissionObjectiveState", "en_bNeverReceivedUpdate",
        "en_iRenderingIndex", "en_bVisibleOnScanner", "en_strObjAdded", "en_strObjAddedToSay", "en_fObjAddedStay",
        "en_pbsObjAdded", "en_strObjAccomplishedToSay", "en_fObjAccomplishedStay", "en_pbsObjAccomplished",
        "en_strObjFailedToSay", "en_fObjFailedStay", "en_pbsObjFailed", "en_bPlayHappyMusic", "en_fHappyMusicPeriod",
        "en_bPlaySadMusic", "en_fSadMusicPeriod", "en_fNetricsaIconAppearingPeriod", "en_hss", "en_pbsHappyMusic",
        "en_pbsSadMusic", "en_bNotifyObjAccomplished", "en_bNotifyObjFailed"],
    "SMissionObjectiveProperties": [
        "ep_bVisibleOnScanner", "ep_strObjAdded", "ep_strObjAddedToSay", "ep_fObjAddedStay", "ep_pbsObjAdded",
        "ep_strObjAccomplishedToSay", "ep_fObjAccomplishedStay", "ep_pbsObjAccomplishedOverride", "ep_bNotifyObjAccomplished",
        "ep_bPlayHappyMusic", "ep_fHappyMusicPeriod", "ep_strObjFailedToSay", "ep_fObjFailedStay", "ep_pbsObjFailedOverride",
        "ep_bNotifyObjFailed", "ep_bPlaySadMusic", "ep_fSadMusicPeriod", "ep_fNetricsaIconAppearingPeriod"],
    "SMultiSpawnerEntity": [
        "en_hre", "en_henSpawner"],
    "SMultiSpawnerProperties": [
        "ep_hspSpawner", "ep_hstpTemplateProvider", "ep_bRandomizeTemplateOrder", "ep_fRndBase"],
    "SNavigationalBeaconEntity": [
        "en_hre", "en_bDirectionOnly", "en_bActive"],
    "SNavigationalBeaconProperties": [
        "ep_bDirectionOnly"],
    "SPlatformEntity": [
        "en_hbo", "en_hhu", "en_hre", "en_fTime", "en_vVelocity", "en_fSign", "en_bSynced", "en_tmMovementStarted",
        "en_qvNetPlacement", "en_fNetSign", "en_fFTimeLeft", "en_qvInitPlacement"],
    "SPlatformProperties": [
        "ep_colColor", "ep_pdPrimitive", "ep_fTime", "ep_vVelocity"],
    "SSpawnDelayerEntity": [
        "en_fSpawnDelayTime", "en_henSpawner", "en_iSpawnee", "en_qvSpawnLocation", "en_vVelocity"],
    "SSpawnDelayerProperties": [
        "ep_fSpawnDelayTime", "ep_henSpawner", "ep_iSpawnee", "ep_vVelocity"],
    "SSpawnerEntity": [
        "en_ctGroup", "en_ctTotal", "en_fDelayInitial", "en_fSpawneeDeathDelay", "en_fDelaySingle", "en_fDelayGroup",
        "en_ctSpawned", "en_ctGroupsToSpawn", "en_ctKilledSpawnees", "en_hepSpawnProperties", "en_henLastSpawned", "en_henOwner",
        "en_hre", "en_hfFormation", "en_henSpawnSource", "en_henSpawnerExploiter", "en_hlcLauncher", "en_fLaunchPitch",
        "en_sfcSpawnEffectConfig", "en_stSpawningType", "en_stMultiSpawningType", "en_qvSpawnOrigin", "en_vSpawnVelocity",
        "en_bRBGSpawnerIsSpawningGroup", "en_qvOwnerProperties", "en_bAutoFallDown", "en_bSpawningPaused", "en_fRndBase",
        "en_scltSpawneeCountLimitType", "en_bGroupSpawner", "en_ssState", "en_pbsSpawneeDelivered", "en_bDisableSpawnSound",
        "en_bDeathSpawner", "en_bIgnoreAmmoCheck", "en_bMultiplied", "en_iMultiplier", "en_bSetIntitialValues",
        "en_ctInitialTotal", "en_ctInitialGroup", "en_bChangeToSimple", "en_tmTriggered", "en_strSpawningPuppetName",
        "en_fPuppetLagFactor", "en_fMinDelay", "en_bUseRandomCone", "en_fMinPitch", "en_fMaxPitch", "en_fAngleH",
        "en_fMinVelocity", "en_fMaxVelocity", "en_iAddToY", "en_iHealth", "en_bCanFly", "en_bNoMulti", "en_fMultiMulti",
        "en_bCantMove", "en_bDeathSpawningEnded", "en_bIgnoreMax"],
    "SSpawnerProperties": [
        "ep_ctTotal", "ep_ctGroup", "ep_fDelayInitial", "ep_fDelaySingle", "ep_fDelayGroup", "ep_fSpawneeDeathDelay",
        "ep_hepSpawnSource", "ep_sfcSpawnEffectConfig", "ep_hlcLauncher", "ep_hfFormation", "ep_bAutoFallDown", "ep_fRndBase",
        "ep_bGroupSpawner", "ep_bDisableSpawnSound"],
}


def twin(name):
    return [name] + (["S" + name[1:]] if name[0] == "C" else [])


def fmt(v):
    if isinstance(v, Enum): return f"{v.name}={v.v if v.v < 2**31 else v.v - 2**32}"
    if isinstance(v, bool): return str(int(v))
    if isinstance(v, float): return f"{v:.6g}"
    if isinstance(v, Ref): return "null" if v.oid == NULL else "ref"
    if isinstance(v, int) and v >= 2**31: return str(v - 2**32) if v != NULL else str(v)
    if isinstance(v, list): return f"list{len(v)}"
    if isinstance(v, Blob): return f"blob{len(v)}"
    if isinstance(v, str): return "'" + v[:48] + "'"
    return str(v)


def collect_types(install, names):
    """One pass over every format-8 file: for each wanted class name the variants (base, members with type names), files, extensions."""
    merged = {}
    for f in sorted((f for f in os.listdir(install) if f.lower().endswith(".gro")), key=_natural):
        with zipfile.ZipFile(os.path.join(install, f)) as z:
            for i in z.infolist():
                if i.is_dir() or i.file_size < 64: continue
                with z.open(i) as fh: head = fh.read(16)
                if head[:8] == b"CTSEMETA" and u32(head, 12) == 8: merged[i.filename.lower()] = (f, i.filename)
    by_arch = collections.defaultdict(list)
    for f, name in merged.values(): by_arch[f].append(name)
    out = {}; nfiles = 0
    for f, ns in by_arch.items():
        with zipfile.ZipFile(os.path.join(install, f)) as z:
            for name in ns:
                r = parse(z.read(name)); nfiles += 1; ext = os.path.splitext(name)[1].lower()
                for t in r["types"].values():
                    if t.members is None or t.name not in names: continue
                    parent = r["types"][t.parent].name if t.parent in r["types"] else None
                    mem = tuple((m, r["types"][mt].name if mt in r["types"] else "?") for m, mt in t.members)
                    rec = out.setdefault(t.name, {"variants": collections.Counter(), "files": 0, "exts": collections.Counter()})
                    rec["variants"][(parent, mem)] += 1; rec["files"] += 1; rec["exts"][ext] += 1
    return out, nfiles


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    limit = int(sys.argv[sys.argv.index("--worlds") + 1]) if "--worlds" in sys.argv else None
    if limit is not None: args = [a for a in args if a != str(limit)]
    short = "--short" in sys.argv
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    names = {n for c in FAMILY + EXTRA for n in twin(c)}
    cat, nfiles = collect_types(install, names)
    print(f"type tables of {nfiles} format-8 files")

    print(f"\n1. the {len(FAMILY)} exported classes of the family and the data")
    indata = 0
    for c in FAMILY + EXTRA:
        found = [n for n in twin(c) if n in cat]
        if not found:
            print(f"   {c:40s} not in the data"); continue
        n = found[0]; rec = cat[n]
        if c in FAMILY: indata += 1
        (parent, mem), _ = rec["variants"].most_common(1)[0]
        exts = ",".join(f"{e.lstrip('.')}:{k}" for e, k in rec["exts"].most_common(3))
        counts = sorted({len(m) for (_, m) in rec["variants"]})
        print(f"   {c:40s} saved as {n:34s} base {str(parent):24s} {len(mem):2d} members (variants with {counts}), {rec['files']:3d} files ({exts})")
    print(f"   exported: {len(FAMILY)}; in the data: {indata}; binary only: {len(FAMILY) - indata}")
    ent = [c for c in FAMILY if c.endswith("Entity")]; prop = [c for c in FAMILY if c.endswith("Properties")]
    print(f"   entities {len(ent)} ({sum(1 for c in ent if any(n in cat for n in twin(c)))} in the data), properties {len(prop)} "
          f"({sum(1 for c in prop if any(n in cat for n in twin(c)))} in the data), other {len(FAMILY) - len(ent) - len(prop)} "
          f"({sum(1 for c in FAMILY if not c.endswith(('Entity', 'Properties')) and any(n in cat for n in twin(c)))} in the data)")

    print("\n2. member order: the data's members against the binary's declared names")
    for cls, bn in BIN.items():
        found = [n for n in (cls, "S" + cls[1:], "C" + cls[1:]) if n in cat]
        if not found: print(f"   {cls}: not in the data"); continue
        for (parent, mem), nv in cat[found[0]]["variants"].most_common():
            dn = [m for m, _ in mem]
            if len(dn) > len(bn): print(f"   {found[0]:30s} data {len(dn)} members > binary {len(bn)}: SKIPPED"); continue
            anchors = all(m == bn[i] for i, m in enumerate(dn) if not m.isdigit())
            it = iter(bn); subseq = all(m in it for m in dn if not m.isdigit())
            print(f"   {found[0]:30s} base {str(parent):22s} data {len(dn):2d} / binary {len(bn):2d} members ({nv} file(s)): "
                  f"count equal {len(dn) == len(bn)}; named members at their binary position {anchors}; named members in binary order {subseq}")

    # ---- census of the worlds
    files = [] if "--no-worlds" in sys.argv else sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))
    ents = collections.defaultdict(lambda: {"n": 0, "worlds": set(), "vals": collections.defaultdict(collections.Counter), "owned": 0})
    recs = collections.defaultdict(lambda: {"n": 0, "worlds": set(), "vals": collections.defaultdict(collections.Counter), "owned": 0})
    cross = collections.defaultdict(collections.Counter)
    nworlds = 0; layouts = collections.defaultdict(collections.Counter)
    for n, (wname, d) in enumerate(files):
        if limit and n >= limit: break
        r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects()
        try: extra, _ = dec.extra_objects(edob)
        except Exception: extra = {}
        T = r["types"]
        rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}
        allo = {**objs, **extra}; nworlds += 1
        typeof = {oid: o.type for oid, o in allo.items() if isinstance(o, Obj)}
        for t in T.values():
            if t.members is not None and t.name in names: layouts[t.name][len(t.members)] += 1
        for oid, o in allo.items():
            if not isinstance(o, Obj) or o.type not in names: continue
            isrec = o.type.endswith("Properties")
            rec = (recs if isrec else ents)[o.type]
            rec["n"] += 1; rec["worlds"].add(wname)
            for k, v in o:
                cls, _, mem = k.partition(".")
                if cls in ("CEntity", "CBaseEntity", "CEntityProperties", "CBaseProperties") or (cls in ("SBaseEntity", "SBaseProperties") and mem != "1"): continue
                if isinstance(v, Ref):
                    if v.oid == NULL: key = "null"
                    elif v.oid in ex: key = "ext:" + ex[v.oid].split("/")[-1]
                    elif v.oid in typeof: key = "->" + typeof[v.oid]
                    else: key = "ref?"
                    rec["vals"][k][key] += 1
                elif isinstance(v, list):
                    rec["vals"][k][f"list{len(v)}"] += 1
                elif isinstance(v, Obj): rec["vals"][k]["obj:" + v.type] += 1
                else: rec["vals"][k][fmt(v)] += 1
            if isrec and isinstance(o.get("CEntityProperties.2"), Ref) and o.get("CEntityProperties.2").oid == NULL: rec["owned"] += 1
            if o.type == "SSpawnerEntity":
                sp = o.get("SSpawnerEntity.120"); lim = o.get("SSpawnerEntity.200")
                spt = typeof.get(sp.oid, "?") if isinstance(sp, Ref) else "?"
                fam = ("character" if "Character" in spt else "item" if "Item" in spt else "multispawner" if "MultiSpawner" in spt else "prop" if "Prop" in spt else spt)
                cross["limit type x spawnee family"][(fmt(lim) if lim is not None else "none", fam)] += 1
                cross["spawnee class (top)"][spt] += 1
                if lim is None: cross["old layout"][wname.rsplit("/", 1)[-1]] += 1
            for ref_name in ("SBaseEntity.1",):
                v = o.get(ref_name)
                if isinstance(v, Ref) and not isrec:
                    cross["entity linked to a chapter info (SBaseEntity.1 set)"][(o.type, "set" if v.oid != NULL else "null")] += 1
        print(f"[{n + 1}/{len(files)}] {wname.rsplit('/', 1)[-1]}", file=sys.stderr, flush=True)

    print(f"\n3. census of {nworlds} worlds: placed entities and property records of the family (members: top values)")
    top = 5 if short else 9
    for c in sorted(set(ents) | {c.replace("Entity", "Properties") for c in ents}):
        for part, tab in (("entity", ents), ("record", recs)):
            e = tab.get(c)
            if not e: continue
            print(f"\n   {c} [{part}] {e['n']} in {len(e['worlds'])} worlds" + (f", {e['owned']} owned (no entity)" if e.get("owned") else ""))
            for k, cn in e["vals"].items():
                print(f"      {k.split('.', 1)[-1]:40s} " + "; ".join(f"{a} x{b}" for a, b in cn.most_common(top)) + (" ..." if len(cn) > top else ""))
    print("\n   member-count layouts of the type tables (members: worlds):")
    for c in sorted(layouts): print(f"      {c:36s} " + ", ".join(f"{k}: {v}" for k, v in sorted(layouts[c].items())))
    for title, cn in cross.items():
        print(f"\n   {title}:")
        for k, v in cn.most_common(40): print(f"      {k}: {v}")

    # ---- tweak tables
    print("\n4. spawner tweak tables (.rsc)")
    eps = load_eps(install, (".rsc",))
    for name, (d, r, objs, edob) in sorted(eps.items()):
        cnt = collections.Counter(o.type for o in objs.values() if isinstance(o, Obj))
        if not any("Spawner" in t for t in cnt): continue
        print(f"   {name}: {sum(cnt.values())} objects; " + ", ".join(f"{a} x{b}" for a, b in sorted(cnt.items())))
        lv = [o for o in objs.values() if isinstance(o, Obj) and o.type in ("CSpawnerTweakParams", "CSpawnerBootTweaks")]
        print(f"      levels with a table: {len(lv)}")
        ctypes = collections.Counter(); per = collections.Counter(); ttypes = collections.Counter(); ids = 0
        for o in objs.values():
            if not isinstance(o, Obj) or not o.type.startswith(("CSpawnerTweakCondition", "CSpawnerBootTweakCondition")): continue
            ctypes[(o.type, o.get("CSpawnerTweakCondition.stc_iConditionType"))] += 1
            lst = o.get("CSpawnerTweakCondition.stp_sttTweakType") or o.get("CSpawnerBootTweakCondition.sbtc_sbttTweakType") or []
            per[len(lst)] += 1
            for e in lst: ttypes[objs[e.oid].type] += 1
            owners = o.get("CSpawnerTweakConditionOwnerID.stoid_aiOwnerIDs") or o.get("CSpawnerBootTweakCondition.sbt_aiOwnerIDs") or []
            ids += len(owners)
        print(f"      conditions (class, stc_iConditionType): {dict(ctypes)}; tweaks per condition: {dict(sorted(per.items()))}; owner ids listed: {ids}")
        print("      tweaks by class: " + ", ".join(f"{a} x{b}" for a, b in ttypes.most_common()))
        vals = collections.defaultdict(collections.Counter)
        for o in objs.values():
            if isinstance(o, Obj) and o.type.startswith("CSpawnerTweak") and not o.type.startswith(("CSpawnerTweakCondition", "CSpawnerTweakParams")):
                for k, v in o:
                    if k.startswith(("CResource", "CSpawnerTweakType.", "CSpawnerBootTweakType.")): continue
                    vals[k][fmt(v)] += 1
        for k, cn in sorted(vals.items()): print(f"         {k:46s} " + "; ".join(f"{a} x{b}" for a, b in cn.most_common(7)))

    # ---- ChapterMaxes.txt
    print("\n5. ChapterMaxes.txt (patches)")
    seen = {}
    for f in sorted((f for f in os.listdir(install) if f.lower().endswith(".gro")), key=_natural):
        with zipfile.ZipFile(os.path.join(install, f)) as z:
            for i in z.infolist():
                if i.filename.lower().endswith("chaptermaxes.txt"): seen[f] = z.read(i).decode("latin1")
    for f, t in seen.items():
        levels = re.findall(r'^"([^"]+)"\s*\{', t, re.M)
        keys = collections.Counter(re.sub(r"\d+", "N", k) for k in re.findall(r"^\s*([A-Za-z_]+\d*[A-Za-z_]*)\s*=", t, re.M))
        vals = collections.Counter(re.findall(r'=\s*"([^"]*)"', t))
        print(f"   {f}: {len(t.splitlines())} lines, {len(levels)} levels; keys {dict(keys)}; values {dict(vals.most_common(14))}")


if __name__ == "__main__":
    main()
