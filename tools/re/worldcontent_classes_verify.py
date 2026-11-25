"""Evidence for docs/re/worldcontent-classes.md (T01-110): what the data says about the world content entity classes of Sam2Game.dll.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries).

It prints
  1. for each of the 55 exported classes of the family (names quoted from the export table of Sam2Game.dll): whether it is in the data
     (own name, the `S` twin of a `C` name, or the name the DLL registers it under: `CParticleEffectEntity` is saved as `CEffectEntity`),
     its base class, own member count, number of files and file extensions; and the data classes next to the family that no export covers;
  2. the member-order check: for every data class whose member table the binary also declares, the data's members in file order against
     the binary's declared names (counts equal, every member the data names at the position of the same name, every data type fitting the
     Hungarian prefix of the binary's name), which names the numbered members;
  3. the census of the worlds: placed entities and property records per class (and per world count), the values of the members that
     carry the rules of the spec (counts of null and set handles, flag values, ranges, the most common identifiers) and the relations
     between records and entities (path marker links, effect parents, flare parents, static model parents, death spawners).

Usage: python -I tools/re/worldcontent_classes_verify.py [DATA_DIR] [--worlds N]
       (DATA_DIR defaults to $SS2_DATA_DIR; --worlds N = only the N smallest worlds; the full run is about 15 minutes)
"""
import collections, os, sys, zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Blob, Enum, Obj, Ref, TreeDec, _natural, merged_files, parse, u32  # noqa: E402

NULL = 0xFFFFFFFF

# The 55 classes exported by Sam2Game.dll that make up this family (T01-026 split, part T01-110).
FAMILY = [
    "CBaseEntity", "CBaseProperties", "SBaseEntity", "SBaseProperties",
    "CStaticModelEntity", "CStaticModelProperties", "CPropEntity", "CPropProperties",
    "CParticleEffectEntity", "CParticleEffectProperties", "CParticleEffectSound", "CSimpleEffectEntity", "CSimpleEffectProperties",
    "CSoundEffectEntity", "CSoundEffectProperties", "CShakerEffectEntity", "CShakerEffectProperties",
    "CSpawnEffectEntity", "CSpawnEffectProperties", "CSpawnEffectCapsuleEntity", "CSpawnEffectCapsuleProperties",
    "CStaticSoundEntity", "CStaticSoundProperties", "CStaticVibrationEntity", "CStaticVibrationProperties",
    "CEnvSFXEntity", "CEnvSFXProperties", "CMoviePlayerEntity", "CMoviePlayerProperties",
    "CFlareEntity", "CFlareProperties", "CPathMarkerEntity", "CPathMarkerProperties", "CVisBoxEntity", "CVisBoxProperties",
    "CNavMeshMarkerEntity", "CNavMeshMarkerProperties",
    "CDebugAngle3f", "CDebugAngle", "CDebugBool", "CDebugDumpConfig", "CDebugFloat", "CDebugIndex", "CDebugObject", "CDebugQV",
    "CDebugString", "CDebugV3f", "CDebugViewAngleH", "CDebugViewAngleP", "CDebugViewBB", "CDebugViewEntityPath", "CDebugViewP",
    "CDebugViewPR", "CDebugViewPV", "CDebugViewQV"]

# Name under which the DLL registers an exported class that is not saved under its export name (read from the binary's class tables;
# names only). The classes of the right-hand side are the ones that appear in the type tables of the files.
REGISTERED = {"CParticleEffectEntity": "CEffectEntity", "CParticleEffectProperties": "CEffectProperties",
              "CSimpleEffectEntity": "SSimpleEffectEntity", "CSimpleEffectProperties": "SSimpleEffectProperties",
              "CSoundEffectEntity": "SSoundEffectEntity", "CSoundEffectProperties": "SSoundEffectProperties",
              "CMoviePlayerEntity": "SMoviePlayerentity", "CMoviePlayerProperties": "SMoviePlayerProperties"}

# Data classes of the same part of the hierarchy that no exported class of this family covers (read here for the comparison).
NEIGHBOURS = ["CEffectEntity", "CEffectProperties", "CTextEffectEntity", "CTextEffectProperties", "CEntity", "CEntityProperties",
              "SSimpleEffectEntity", "SSoundEffectEntity", "SMoviePlayerentity", "SMoviePlayerEntity", "SStaticVibrationEntity"]

# Member names that the binary's member tables declare, in declaration order (names only; the binary's own identifiers).
BIN = {
    "CBaseEntity": [
        "en_ulBaseFlags"],
    "SBaseEntity": [
        "en_henChapter"],
    "CBaseProperties": [
        "ep_ulEntityIDVis"],
    "SBaseProperties": [
        "ep_hepChapter"],
    "CStaticModelEntity": [
        "en_hmrModel", "en_hhuHull", "en_hmech", "en_bCollision", "en_idMechanism", "en_mlidMacroLatentID", "en_fSleepTime",
        "en_idAnimToStop", "en_pmcOrgModel", "en_idAutoStartAnim", "en_henParent", "en_idParentPart",
        "en_hsmaedAnimEventsData", "en_fSoundHotSpot", "en_fSoundFallOff", "en_bMoveHulls", "en_slAccuDamage",
        "en_tmLastDamage", "en_henLastDamager", "en_slLastDamage", "en_henDeathSpawner", "en_hssAlive", "en_hssDestruction",
        "en_qvOriginalPlacement", "en_smaaAnimAction", "en_idAnimName", "en_fAnimFadeInTime", "en_fAnimCutBackTime",
        "en_ulAnimFlags", "en_fAnimSpeedMultiplier", "en_fAnimStrength", "en_bAnimWait", "en_fAnimLoopsCount",
        "en_fAnimFadeInNewState", "en_fClientOffsetError", "en_bControlByMacro", "en_pmcCurrentModel", "en_bHidden",
        "en_mfFlags", "en_bAnimPlayedByMacro", "en_smaaNetAnimAction", "en_idNetAnimName", "en_fNetAnimFadeInTime",
        "en_fNetAnimCutBackTime", "en_fNetAnimFadeInNewState", "en_ulNetAnimFlags", "en_fNetAnimSpeedMultiplier",
        "en_fNetAnimStrength", "en_bNetAnimWait", "en_fNetAnimLoopsCount", "en_fNetCurrentAnimOffset"],
    "CStaticModelProperties": [
        "ep_pmcModel", "ep_vStretch", "ep_smlModifiers", "ep_bCollision", "ep_idMechanism", "ep_fCullDistance",
        "ep_fFadeLength", "ep_idParentPart", "ep_idAutoStartAnim", "ep_estShadows", "ep_fSoundFallOff", "ep_fSoundHotSpot",
        "ep_bMoveHulls", "ep_hepDeathSpawner", "ep_henDeathSpawner", "ep_bDisableDestruction", "ep_bIgnoredByNavigation",
        "ep_bControlByMacro"],
    "SPropEntity": [
        "en_hre", "en_smlModifiers", "en_hmcMechanism", "en_henParentProp", "en_idParentPart", "en_tmLastFluidTouch",
        "en_tmLastGroundTouch", "en_idPoseAnimation", "en_henSpawner", "en_henSpawnerWhoSpawnedMe",
        "en_bMultiSpawnerTemplate", "en_bNoGravity", "en_slAccuDamage", "en_tmLastDamage", "en_bDynamicallyCreated",
        "en_pfxSplash", "en_pfxDust", "en_pfFlags", "en_ulDefaultObjectManipulationFlags",
        "en_ulCurrentObjectManipulationFlags", "en_henLastDamager", "en_slLastDamage", "en_henDeathSpawner",
        "en_hssDestruction", "en_hsaehAnimEventHandler", "en_idMechanismName", "en_bSpawnEffects", "en_fMinCollisionSpeedFX",
        "en_fMaxCollisionSpeedFX", "en_qvOriginalPlacement", "en_idLoopingAnimation", "en_bActiveOnStart", "en_fStretch",
        "en_pmcDefaultModel", "en_fFadeLength", "en_fCullDistance", "en_estShadows", "en_bPhysicsEnabled",
        "en_qvNetPlacement", "en_vNetVelocity", "en_bNetActive", "en_pmcCurrentModel", "en_bHidden", "en_bPickedAsTool"],
    "SPropProperties": [
        "ep_pdPrimitive", "ep_pmcModel", "ep_vStretch", "ep_fStretch", "ep_smlModifiers", "ep_idMechanismName",
        "ep_henParentMechanism", "ep_idParentPart", "ep_fCullDistance", "ep_fFadeLength", "ep_estShadows",
        "ep_bPhysicsEnabled", "ep_bSpawnEffects", "ep_fMinCollisionSpeedFX", "ep_fMaxCollisionSpeedFX",
        "ep_idLoopingAnimation", "ep_idPoseAnimation", "ep_hspSpawner", "ep_bMultiSpawnerTemplate", "ep_bActiveOnStart",
        "ep_bNoGravity", "ep_bIsSpawned", "ep_hepDeathSpawner", "ep_henDeathSpawner", "ep_ulDefaultObjectManipulationFlags"],
    "CEffectEntity": [
        "en_hedr", "en_hre", "en_fxltLifeType", "en_bSpawnedEffect", "en_bAutoStart", "en_idParentPart",
        "en_idParentPartAttachment", "en_henParent", "en_hss", "en_ulSndFlags", "en_pbsSound", "en_fPitch", "en_fPitchRnd",
        "en_qvOrgPlacement", "en_pesEffectState", "en_pesNetEffectState", "en_bEmitting", "en_bNetEmitting"],
    "CEffectProperties": [
        "ep_pefEffect", "ep_pesSound", "ep_fStretch", "ep_idParentPart", "ep_idParentPartAttachment", "ep_bSpawnedEffect",
        "ep_bAutoStart", "ep_hepRelevantEntity"],
    "CParticleEffectSound": [
        "pes_pbsSound", "pes_fVolume", "pes_fPitch", "pes_fRndPitch", "pes_fHotSpot", "pes_fFallOff", "pes_ulFlags"],
    "SShakerEffectEntity": [
        "en_hreShaker", "en_hreEditorRenderable", "en_bCreatedInEditor"],
    "SShakerEffectProperties": [
        "ep_spShakingParams", "ep_bCreatedInEditor"],
    "SStaticSoundEntity": [
        "en_hss", "en_hre", "en_pbs", "en_ulSoundFlags", "en_bAutoPlay", "en_fPitch", "en_fPitchRnd", "en_fVolume",
        "en_fFallOff", "en_fSoundLength", "en_mlidMacroLatentID", "en_fFadeInLength", "en_fFadeOutLength", "en_tmStartTime",
        "en_sssState", "en_sssNetState", "en_henParent", "en_idParentPart", "en_fDelayPlay"],
    "SStaticSoundProperties": [
        "ep_pbsSound", "ep_fVolume", "ep_fPitch", "ep_fPitchRnd", "ep_fHotSpot", "ep_fFallOff", "ep_iConeAngle",
        "ep_iSoftAngle", "ep_fConeVolume", "ep_ulFlags", "ep_bAutoPlay", "ep_fDelayPlay", "ep_idParentPart"],
    "CEnvSFXEntity": [
        "en_hse", "en_hre", "en_henParent", "en_idParentPart"],
    "CEnvSFXProperties": [
        "ep_eEnvFX", "ep_fSizeX", "ep_fSizeY", "ep_fSizeZ", "ep_idParentPart"],
    "SFlareEntity": [
        "en_hre", "en_henParent", "en_idParentPart", "en_qvInitialPlacement"],
    "SFlareProperties": [
        "ep_pfcConfig", "ep_colColor", "ep_fHotSpot", "ep_fFallOff", "ep_fCoverage", "ep_fGlaring", "ep_idParentPart"],
    "SPathMarkerEntity": [
        "en_hre", "en_hreDir", "en_henNextMarker", "en_henPrevMarker", "en_strName"],
    "SPathMarkerProperties": [
        "ep_hepNextMarker", "ep_colColor"],
    "SVisBoxEntity": [
        "en_hre"],
    "SVisBoxProperties": [
        "ep_vSize", "ep_fCullDistance", "ep_bOcclusionTest"],
    "CNavMeshMarkerEntity": [
        "en_hre"],
    "CTextEffectEntity": [
        "en_ptrfxRenderingEffect2D", "en_ptrfxRenderingEffect3D", "en_pre", "en_ptfxConfiguration",
        "en_ptfxbcBackgroundConfig"],
    "CTextEffectProperties": [
        "ep_ptfxConfiguration", "ep_ptfxbcBackgroundConfig", "ep_strText", "ep_tretEffectType", "ep_fWidth",
        "ep_bFaceForward", "ep_bDoubleSided", "ep_bStayForever", "ep_bRenderTextRegion", "ep_idParentPart"],
    "SSimpleEffectEntity": [
        "en_hre", "en_hss", "en_fLifeTime", "en_nebNetBehavior", "en_ulSoundFlags", "en_pbsSound", "en_pfxEffect",
        "en_qvInitialPlacement", "en_fEffectStretch", "en_fEffectPower", "en_vCollisionNormal", "en_vCollisionPenetration",
        "en_fVolume", "en_fPitch", "en_fPitchRnd", "en_fHotSpot", "en_fFallOff", "en_iConeAngle", "en_iSoftAngle",
        "en_fConeVolume"],
    "SSimpleEffectProperties": [
        "ep_pfxEffect", "ep_fEffectStretch", "ep_fEffectPower", "ep_pbsSound", "ep_ulSoundFlags", "ep_fVolume", "ep_fPitch",
        "ep_fPitchRnd", "ep_fHotSpot", "ep_fFallOff", "ep_iConeAngle", "ep_iSoftAngle", "ep_fConeVolume",
        "ep_vCollisionPenetration", "ep_vCollisionNormal", "ep_nebNetBehavior"],
    "SSoundEffectEntity": [
        "en_hss", "en_pbs", "en_ulSoundFlags"],
    "SSoundEffectProperties": [
        "ep_pbsSound", "ep_fVolume", "ep_fPitch", "ep_fRndPitch", "ep_fHotSpot", "ep_fFallOff", "ep_iConeAngle",
        "ep_iSoftAngle", "ep_fConeVolume", "ep_ulFlags"],
    "SMoviePlayerentity": [
        "en_hre", "en_hmcMovieChannel", "en_pbmSource", "en_ptxTarget", "en_pmsTarget", "en_bAutoPlay"],
    "SMoviePlayerProperties": [
        "ep_pbmSource", "ep_ptxTarget", "ep_pmsTarget", "ep_bAutoPlay"],
}

# Hungarian prefix of a binary member name -> acceptable data type names (empty = not checked).
FITS = {"b": ("BOOL",), "ul": ("ULONG", "SndFlags", "AspectFlags"), "qv": ("QuatVect",), "v": ("Vector3f",), "str": ("CString",),
        "sl": ("SLONG",), "f": ("FLOAT",), "id": ("IDENT",), "tm": ("Time",), "col": ("COLOR",)}

# Runtime member (entity) -> record member (property record) pairs that the class's creation function copies; the script counts how many
# entity/record pairs of the world files agree (names are the binary's; the data's numbered members are found by the order rule).
PAIRS = {
    "CStaticModelEntity": ("CStaticModelProperties", [("en_bCollision", "ep_bCollision"), ("en_idMechanism", "ep_idMechanism"),
                           ("en_idAutoStartAnim", "ep_idAutoStartAnim"), ("en_idParentPart", "ep_idParentPart"), ("en_fSoundHotSpot", "ep_fSoundHotSpot"),
                           ("en_fSoundFallOff", "ep_fSoundFallOff"), ("en_bMoveHulls", "ep_bMoveHulls"), ("en_bControlByMacro", "ep_bControlByMacro")]),
    "SPropEntity": ("SPropProperties", [("en_idMechanismName", "ep_idMechanismName"), ("en_bSpawnEffects", "ep_bSpawnEffects"),
                    ("en_fMinCollisionSpeedFX", "ep_fMinCollisionSpeedFX"), ("en_fMaxCollisionSpeedFX", "ep_fMaxCollisionSpeedFX"),
                    ("en_idLoopingAnimation", "ep_idLoopingAnimation"), ("en_bActiveOnStart", "ep_bActiveOnStart"), ("en_fStretch", "ep_fStretch"),
                    ("en_fFadeLength", "ep_fFadeLength"), ("en_fCullDistance", "ep_fCullDistance"), ("en_estShadows", "ep_estShadows"),
                    ("en_bPhysicsEnabled", "ep_bPhysicsEnabled"), ("en_ulDefaultObjectManipulationFlags", "ep_ulDefaultObjectManipulationFlags"),
                    ("en_bNoGravity", "ep_bNoGravity"), ("en_idParentPart", "ep_idParentPart"), ("en_idPoseAnimation", "ep_idPoseAnimation")]),
    "CEffectEntity": ("CEffectProperties", [("en_bAutoStart", "ep_bAutoStart"), ("en_idParentPart", "ep_idParentPart"),
                      ("en_idParentPartAttachment", "ep_idParentPartAttachment"), ("en_bSpawnedEffect", "ep_bSpawnedEffect")]),
    "SStaticSoundEntity": ("SStaticSoundProperties", [("en_bAutoPlay", "ep_bAutoPlay"), ("en_fVolume", "ep_fVolume"), ("en_fPitch", "ep_fPitch"),
                           ("en_fPitchRnd", "ep_fPitchRnd"), ("en_ulSoundFlags", "ep_ulFlags"), ("en_fDelayPlay", "ep_fDelayPlay"),
                           ("en_fFallOff", "ep_fFallOff"), ("en_idParentPart", "ep_idParentPart")]),
    "SFlareEntity": ("SFlareProperties", [("en_idParentPart", "ep_idParentPart")]),
    "CEnvSFXEntity": ("CEnvSFXProperties", [("en_idParentPart", "ep_idParentPart")]),
    "SShakerEffectEntity": ("SShakerEffectProperties", [("en_bCreatedInEditor", "ep_bCreatedInEditor")]),
}

# Class-constructor defaults of runtime members that no creation function overwrites (read from the binary; values only). Every placed
# entity is saved in its construction state for these, which the script checks on all of them.
CTOR = {
    "CStaticModelEntity": {"en_fAnimFadeInTime": 0.2, "en_fNetAnimFadeInTime": 0.2, "en_fAnimFadeInNewState": 0.2, "en_fNetAnimFadeInNewState": 0.2,
                           "en_fAnimCutBackTime": 0, "en_fAnimSpeedMultiplier": 1, "en_fNetAnimSpeedMultiplier": 1, "en_fAnimStrength": 1,
                           "en_fNetAnimStrength": 1, "en_fAnimLoopsCount": -1, "en_fNetAnimLoopsCount": -1, "en_ulAnimFlags": 0,
                           "en_bAnimWait": 0, "en_smaaAnimAction": 0, "en_slAccuDamage": 0, "en_slLastDamage": 0, "en_fClientOffsetError": 0,
                           "en_bAnimPlayedByMacro": 0, "en_fNetCurrentAnimOffset": 0},
    "SPropEntity": {"en_slAccuDamage": 0, "en_slLastDamage": 0, "en_bDynamicallyCreated": 0, "en_bPickedAsTool": 0, "en_bNoGravity": 0,
                    "en_pfFlags": 0},
    "CEffectEntity": {"en_bEmitting": 1, "en_bNetEmitting": 1, "en_pesNetEffectState": 0},
}


def prefix(name):
    """The Hungarian prefix of `<class>_<prefix><Name>` (the lower-case letters after the underscore, then up to the first capital)."""
    tail = name.split("_", 1)[1]
    i = 0
    while i < len(tail) and tail[i].islower(): i += 1
    return tail[:i]


def twin(name):
    alt = REGISTERED.get(name)
    return [name] + (["S" + name[1:]] if name[0] == "C" else []) + ([alt] if alt else [])


def fmt(v):
    if isinstance(v, Enum): return f"{v.name}={v.v if v.v < 2**31 else v.v - 2**32}"
    if isinstance(v, float): return f"{v:.6g}"
    if isinstance(v, Ref): return "null" if v.oid == NULL else "ref"
    if isinstance(v, int) and not isinstance(v, bool) and v >= 2**31: return str(v - 2**32) if v == NULL else str(v)
    return str(v)


def collect(install, names):
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


def lineage(parent, name):
    out = []
    while name is not None and name not in out: out.append(name); name = parent.get(name)
    return out


def summarize(vals):
    """Short text for a Counter of formatted values: null/set split for handles, a range for numbers, else the most common values."""
    n = sum(vals.values())
    nums = []
    for k, c in vals.items():
        try: nums.append((float(k), c))
        except ValueError: nums = None; break
    if nums and len(vals) > 6:
        return f"range {min(x for x, _ in nums):.6g} .. {max(x for x, _ in nums):.6g} ({len(vals)} distinct) over {n}"
    return ", ".join(f"{a} x{b}" for a, b in vals.most_common(6)) + (f" ... ({len(vals)} distinct)" if len(vals) > 6 else "")


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    limit = int(sys.argv[sys.argv.index("--worlds") + 1]) if "--worlds" in sys.argv else None
    if limit is not None: args = [a for a in args if a != str(limit)]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    wanted = {n for c in FAMILY for n in twin(c)} | set(NEIGHBOURS) | set(BIN)
    cat, nfiles = collect(install, wanted)
    print(f"type tables of {nfiles} format-8 files")

    print(f"\n1. the {len(FAMILY)} exported classes of the family and the data")
    indata = 0; saved_c = []; saved_s = []; saved_alias = []
    for c in FAMILY:
        found = [n for n in twin(c) if n in cat]
        if not found:
            print(f"   {c:34s} not in the data"); continue
        indata += 1; n = found[0]; rec = cat[n]
        (parent, mem), _ = rec["variants"].most_common(1)[0]
        exts = ",".join(f"{e.lstrip('.')}:{k}" for e, k in rec["exts"].most_common(3))
        (saved_c if n == c else saved_alias if n == REGISTERED.get(c) else saved_s).append(c)
        print(f"   {c:34s} saved as {n:28s} base {str(parent):16s} {len(mem):2d} members, {rec['files']:3d} files ({exts}), {len(rec['variants'])} variant(s)")
    print(f"   in the data: {indata}; binary only: {len(FAMILY) - indata}; saved under the C name: {len(saved_c)}; as the S twin: {len(saved_s)}; "
          f"under the registered name: {len(saved_alias)} {saved_alias}")
    print("   neighbours in the data (no export of this family):")
    for n in NEIGHBOURS:
        if n in cat:
            (parent, mem), _ = cat[n]["variants"].most_common(1)[0]
            print(f"      {n:28s} base {str(parent):16s} {len(mem)} members, {cat[n]['files']} files")
        else: print(f"      {n:28s} not in the data")

    print("\n2. member order: the data's members against the binary's declared names")
    for cls, names in BIN.items():
        found = [n for n in (cls, "S" + cls[1:], "C" + cls[1:]) if n in cat]
        if not found: print(f"   {cls}: not in the data"); continue
        key = found[0]
        for (parent, mem), nv in cat[key]["variants"].most_common():
            dn = [m for m, _ in mem]
            same_count = len(dn) == len(names)
            named_ok = all(m == names[i] for i, m in enumerate(dn) if not m.isdigit() and i < len(names)) if len(dn) <= len(names) else False
            bad = []
            for i, (m, t) in enumerate(mem):
                if i >= len(names): break
                fit = FITS.get(prefix(names[i]), ())
                if fit and t not in fit and not t.startswith("Enum") and not t.startswith("Ptr<"): bad.append((names[i], t))
            print(f"   {key:26s} base {str(parent):14s} data {len(dn):2d} / binary {len(names):2d} members ({nv} file(s)): "
                  f"same count {same_count}; named members at their binary position {named_ok}; type misfits {bad or 'none'}")
            if len(mem) and same_count:
                print("      " + ", ".join(f"{m}={names[i]}" for i, (m, _) in enumerate(mem) if m != names[i])[:900])

    print("\n3. census of the worlds")
    files = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))
    fam_ent = {n for c in FAMILY if c.endswith("Entity") for n in twin(c)} | {"CTextEffectEntity"}
    fam_prop = {n for c in FAMILY if c.endswith("Properties") for n in twin(c)} | {"CTextEffectProperties"}
    ents = collections.defaultdict(lambda: {"n": 0, "worlds": set(), "vals": collections.defaultdict(collections.Counter)})
    props = collections.defaultdict(lambda: {"n": 0, "worlds": set(), "vals": collections.defaultdict(collections.Counter)})
    allcls = collections.Counter(); base_flags = collections.Counter(); chapter = collections.Counter(); vis_id = collections.Counter()
    rel = collections.Counter(); nworlds = 0; nrec_owned = collections.Counter()
    flag_by_class = collections.defaultdict(collections.Counter); pair_tally = collections.defaultdict(collections.Counter)
    ctor_tally = collections.defaultdict(collections.Counter); mapnames = {}
    for cls in BIN:
        key = next((n for n in (cls, "S" + cls[1:], "C" + cls[1:]) if n in cat), None)
        if key:
            (_, mem), _ = cat[key]["variants"].most_common(1)[0]
            if len(mem) == len(BIN[cls]): mapnames[key] = {BIN[cls][i]: m for i, (m, _) in enumerate(mem)}
    nav_ignored = collections.Counter(); hotfix = []
    rev = {c: {v: k for k, v in m.items()} for c, m in mapnames.items()}

    def nm(k):
        c, _, m = k.partition(".")
        b = rev.get(c, {}).get(m)
        return f"{k} [{b}]" if b and b != m else k
    for n, (wname, d) in enumerate(files):
        if limit and n >= limit: break
        r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects()
        try: extra, _ = dec.extra_objects(edob)
        except Exception: extra = {}
        T = r["types"]; parent = {t.name: (T[t.parent].name if t.parent in T else None) for t in T.values() if t.members is not None}
        allo = {**objs, **extra}; nworlds += 1
        rec_of = {}
        for x in extra.values():
            eo = x.get("CEntityProperties.2") if isinstance(x, Obj) else None
            if isinstance(eo, Ref) and eo.oid != NULL: rec_of[eo.oid] = x
        for oid, o in allo.items():
            if not isinstance(o, Obj): continue
            lin = lineage(parent, o.type)
            is_ent = oid in objs and "CBaseEntity" in lin
            if is_ent:
                allcls[o.type] += 1
                v = o.get("CBaseEntity.1"); base_flags[fmt(v) if v is not None else "absent"] += 1
                ch = o.get("SBaseEntity.1")
                if ch is not None: chapter["null" if isinstance(ch, Ref) and ch.oid == NULL else "set"] += 1
            if o.type in fam_ent and is_ent:
                rec = ents[o.type]; rec["n"] += 1; rec["worlds"].add(wname)
                fl = o.get("CBaseEntity.1"); flag_by_class[o.type][fmt(fl) if fl is not None else "absent"] += 1
                mp = mapnames.get(o.type, {})
                for bn, dv in CTOR.get(o.type, {}).items():
                    v = o.get(f"{o.type}.{mp.get(bn)}") if bn in mp else None
                    if v is None: ctor_tally[(o.type, bn)]["absent"] += 1
                    else:
                        v = v.v if isinstance(v, Enum) else v
                        ctor_tally[(o.type, bn)]["equal" if (abs(v - dv) < 1e-6 if isinstance(v, (int, float)) and not isinstance(v, bool) else v == dv) else "differ"] += 1
                rc = rec_of.get(oid)
                if o.type in PAIRS and rc is not None:
                    rcls, plist = PAIRS[o.type]; rmp = mapnames.get(rcls, {})
                    for bn, rn in plist:
                        if bn not in mp or rn not in rmp: continue
                        a = o.get(f"{o.type}.{mp[bn]}"); b = rc.get(f"{rcls}.{rmp[rn]}")
                        a = a.v if isinstance(a, Enum) else a; b = b.v if isinstance(b, Enum) else b
                        pair_tally[(o.type, bn, rn)]["equal" if a == b else "differ"] += 1
                    if o.type == "CStaticModelEntity":
                        bi = rc.get(f"{rcls}.{rmp['ep_bIgnoredByNavigation']}") if "ep_bIgnoredByNavigation" in rmp else None
                        nav_ignored[("record ignored-by-navigation", bi, "entity base flags", fmt(fl) if fl is not None else None)] += 1
                        dd = rc.get(f"{rcls}.{rmp['ep_bDisableDestruction']}")
                        mf = o.get(f"{o.type}.{mp['en_mfFlags']}"); mf = mf.v if isinstance(mf, Enum) else mf
                        nav_ignored[("record disable-destruction", dd, "entity en_mfFlags", mf)] += 1
                if o.type == "SPathMarkerEntity" and o.get("CEntity.1") == 3325 and "1_2_approachingvillage" in wname.lower():
                    qv = rc.get("CEntityProperties.3") if rc is not None else None
                    pos = [round(v, 3) for _, v in qv.get("QuatVect.2")] if isinstance(qv, Obj) else None
                    hotfix.append((wname.rsplit("/", 1)[-1], o.type, "record position", pos))
                if o.type == "CStaticModelEntity" and rc is not None:
                    col = rc.get(f"{rcls}.{rmp['ep_bCollision']}"); me = o.get(f"{o.type}.{mp['en_hmech']}")
                    rel[("static model: record collision flag", col, "mechanism set", isinstance(me, Ref) and me.oid != NULL)] += 1
                for k, v in o:
                    cls, _, mem = k.partition(".")
                    if cls in ("CEntity", "CBaseEntity", "SBaseEntity"): continue
                    if isinstance(v, Ref): rec["vals"][k]["null" if v.oid == NULL else "set"] += 1
                    elif isinstance(v, (Obj, list, Blob)): rec["vals"][k]["<structure>"] += 1
                    else: rec["vals"][k][fmt(v)] += 1
                for k, tgt in (("CStaticModelEntity.21", "static model parent"), ("SFlareEntity.2", "flare parent"), ("CEffectEntity.en_henParent", "effect parent")):
                    v = o.get(k)
                    if isinstance(v, Ref) and v.oid != NULL and isinstance(allo.get(v.oid), Obj): rel[(tgt, allo[v.oid].type)] += 1
                if o.type == "SPathMarkerEntity":
                    nx = o.get("SPathMarkerEntity.2"); bk = o.get("SPathMarkerEntity.4")
                    rel[("path marker next set", isinstance(nx, Ref) and nx.oid != NULL)] += 1
                    if isinstance(nx, Ref) and nx.oid != NULL and isinstance(allo.get(nx.oid), Obj):
                        rel[("path marker next is a path marker", allo[nx.oid].type)] += 1
                        back = allo[nx.oid].get("SPathMarkerEntity.4")
                        rel[("next marker's back link points here", isinstance(back, Ref) and back.oid == oid)] += 1
            elif o.type in fam_prop:
                rec = props[o.type]; rec["n"] += 1; rec["worlds"].add(wname)
                if o.type == "SFlareProperties":
                    fm = mapnames.get("SFlareProperties", {})
                    hs, fo, cv, gl = (o.get(f"SFlareProperties.{fm[k]}") for k in ("ep_fHotSpot", "ep_fFallOff", "ep_fCoverage", "ep_fGlaring"))
                    rel[("flare record: hot spot >= 0", hs >= 0, "fall-off >= hot spot", fo >= hs, "coverage >= 0.01", cv >= 0.01, "0 <= glare <= 1", 0 <= gl <= 1)] += 1
                eo = o.get("CEntityProperties.2")
                nrec_owned[(o.type, "owned (no entity)" if isinstance(eo, Ref) and eo.oid == NULL else "has entity")] += 1
                for k, v in o:
                    cls, _, mem = k.partition(".")
                    if cls in ("CEntityProperties",): continue
                    if cls == "CBaseProperties":
                        vis_id["set" if v else "zero"] += 1; continue
                    if cls == "SBaseProperties": continue
                    if isinstance(v, Ref): rec["vals"][k]["null" if v.oid == NULL else "set"] += 1
                    elif isinstance(v, (Obj, list, Blob)): rec["vals"][k]["<structure>"] += 1
                    else: rec["vals"][k][fmt(v)] += 1
        print(f"[{n + 1}/{len(files)}] {wname.rsplit('/', 1)[-1]}", file=sys.stderr, flush=True)

    print(f"   worlds read: {nworlds}; entity classes of the world files: {len(allcls)}; entities {sum(allcls.values())}")
    print(f"   `CBaseEntity.1` (flags) over all entities: {dict(base_flags)}; `SBaseEntity.1` (chapter info handle): {dict(chapter)}")
    print(f"   `CBaseProperties` member present on {sum(vis_id.values())} records of the family: {dict(vis_id)}")
    print(f"   family entities: {sum(v['n'] for v in ents.values())}; family property records: {sum(v['n'] for v in props.values())}")
    print("\n   class                             entities (worlds) | records (worlds)")
    for c in sorted(ents):
        e = ents[c]; pp = props.get(c.replace("Entity", "Properties"))
        print(f"   {c:32s} {e['n']:6d} ({len(e['worlds']):2d}) | {pp['n'] if pp else 0:6d} ({len(pp['worlds']) if pp else 0:2d})")
        for k, cnt in sorted(e["vals"].items()): print(f"        {nm(k):62s} {summarize(cnt)}")
        if pp:
            for k, cnt in sorted(pp["vals"].items()): print(f"        [record] {nm(k):53s} {summarize(cnt)}")
            print(f"        [record] owners: {dict((b, n2) for (a, b), n2 in nrec_owned.items() if a == c.replace('Entity', 'Properties'))}")
    print("\n   record classes without an entity class of the same stem:", [c for c in props if c.replace("Properties", "Entity") not in ents])
    print("   `CBaseEntity.1` (base flags) per class:", {c: dict(v) for c, v in sorted(flag_by_class.items())})
    print("   record flag against the entity flag it sets:")
    for k, v in sorted(nav_ignored.items(), key=str): print(f"      {k}: {v}")
    print("   entity member equals its record member (copied by the creation function), per pair:")
    for (c, a, b), v in sorted(pair_tally.items()): print(f"      {c}.{a} = {b}: {dict(v)}")
    print("   constructor default of a runtime member against every placed entity:")
    for (c, a), v in sorted(ctor_tally.items()): print(f"      {c}.{a} = {CTOR[c][a]}: {dict(v)}")
    print(f"   path marker with serial 3325 in a world whose name contains 1_2_ApproachingVillage (case-insensitive): {hotfix}")
    print("   relations:")
    for k, v in sorted(rel.items(), key=lambda kv: str(kv[0])): print(f"      {k}: {v}")


if __name__ == "__main__":
    main()
