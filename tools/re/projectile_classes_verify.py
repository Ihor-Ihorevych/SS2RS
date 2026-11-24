"""Evidence for docs/re/projectile-classes.md (T01-107): what the data says about the projectile classes of Sam2Game.dll.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries).

It prints
  1. for each of the 32 exported classes of the family (names quoted from the export table of Sam2Game.dll) and for the
     data names the binary registers for classes whose exported name differs: whether it is in the data, its base class,
     member count, number of files and file extensions;
  2. the member-order check (section 4 of the spec): for every data class whose member table the binary also declares, the
     data's members in file order against the binary's declared names (names only, quoted from the binary). A data member
     that has only a number is given the binary's name at the same position; the check is that the counts are equal (the
     binary's editor-only members, which the data never stores, are left out), that every member the data does name sits at the
     position of the same name, and that the data type fits the Hungarian prefix of the binary name;
  3. the 94 projectile parameter files (`GenericProjectiles/`): class, enumerations, and every member by binary name with the
     number of distinct values, the range and the commonest value; relations the spec states (flags against seeking members, ...);
  5. the stored instances in the worlds (`.wld`): shooters, launchers and crumbs, with the owners of the shooters and the
     named-member values (counts only);
  4. `SPuppetProjectileParams` (126 files), the bullet and flame parameters of the global parameter file, and the launch
     velocities of the character tools (`CLaunchVelocities`, 68 files).

Usage: python -I tools/re/projectile_classes_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR; about 4 minutes)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Enum, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
import weapon_classes_verify as wcv  # noqa: E402
import weapons_verify as wv  # noqa: E402

# The 32 classes exported by Sam2Game.dll that make up this family (T01-026 split, part T01-107).
FAMILY = ["CBaseProjectileEntity", "CBaseProjectileParams", "CBaseProjectileProperties", "CBulletEntity", "CBulletProperties",
          "CBulletTrailEntity", "CBulletTrailProperties", "CBulletTrailRenderable", "CBulletsItemEntity", "CBulletsItemProperties",
          "CCrumbsEntity", "CCrumbsProperties", "CDebrisEntity", "CDebrisProperties", "CFlameEntity", "CFlameHitData", "CFlameProperties",
          "CFlamerFlameParams", "CFlamerProjectileEntity", "CFlamerProjectileParams", "CFlamerProjectileProperties",
          "CGenericProjectileEntity", "CGenericProjectileParams", "CGenericProjectileProperties", "CLaunchVelocities", "CMarkersLauncher",
          "CMovingBulletTrail", "CPuppetProjectileParams", "CRelativeRandomConeLauncher", "CShooterEntity", "CShooterProperties",
          "CStaticBulletTrail"]
# Data names the binary registers although they are not the S twin of the exported name (the shooter is registered as the "bullet shooter").
OTHER_NAMES = {"CShooterEntity": "SBulletShooterEntity", "CShooterProperties": "SBulletShooterProperties",
               "CBaseProjectileParams": "SBaseProjectileParams"}

# Member names that the binary's member tables declare, in declaration order (the binary's own identifiers).
# `*` marks a member the binary flags as not stored in files (editor only); the data never has it.
BIN = {
    "SBaseProjectileParams": [
        "bp_pmcModel", "bp_ulFlags", "bp_fFlyingSpeed", "bp_bAddShooterVelocity", "bp_bHorizontalProjectile", "bp_vMinRandomRotation",
        "bp_vMaxRandomRotation", "bp_fStretch", "bp_fStretchMax", "bp_slPiercingPowerMin", "bp_slPiercingPowerMax", "bp_dtDirectDamageType",
        "bp_slDirectDamage", "bp_slDirectDamageMax", "bp_fImpactForceImpulse", "bp_fImpactForceImpulseUpRatio", "bp_slAreaDamage",
        "bp_slAreaDamageMax", "bp_fRangeForceImpulse", "bp_fRangeForceImpulseUpRatio", "bp_fDamageHotspot", "bp_fDamageHotspotMax",
        "bp_fDamageFalloff", "bp_fDamageFalloffMax", "bp_fLifeTime", "bp_fIgnoreLauncherPeriod", "bp_fDontExplodePeriod",
        "bp_bExplodesOnFirstTouch", "bp_bExplodesOnPuppets", "bp_bUpwardProjectile", "bp_fSeekFrequency", "bp_fSeekPredictionRatio",
        "bp_fSeekingConeDeg", "bp_fSeekingAngVSDistRatio", "bp_fSeekingMaxDistance", "bp_fStopSeekingDistance", "bp_fSeekingAngularVelocity",
        "bp_fSeekMovedCheckPeriod", "bp_fSeekMovedCheckDistanceRatio", "bp_bSeekMoreTimes", "bp_fMinPowerSeekingRatio", "bp_idEffectType",
        "bp_fFlyingSoundFallOff", "bp_fFlyingSoundHotSpot", "bp_fExplodeSoundFallOff", "bp_fExplodeSoundHotSpot",
        "bp_fRangeDamageOwnerMultiplier", "bp_fDirectDamageOwnerMultiplier"],
    "SGenericProjectileParams": ["gpp_pnProjectileNature", "gpp_slHealth"],
    "SPuppetProjectileParams": ["ppp_fProjectileDamageMultiplier", "ppp_fMinThrowPitchDeg", "ppp_fMaxThrowPitchDeg",
                                "ppp_fMinThrowPitchDistance", "ppp_fMaxThrowPitchDistance", "ppp_fMinPredictRatio", "ppp_fMaxPredictRatio",
                                "ppp_fMinUndershootRatio", "ppp_fMaxUndershootRatio"],
    "CFlamerFlameParams": ["ffp_pefPuppetFlame", "ffp_pefModelFlame", "ffp_pbsFlameBurning", "ffp_fMinFlameSize", "ffp_fMaxFlameSize",
                           "ffp_ghFlameSizeMultiplierOverPuppetSize", "ffp_fApplyDamageIncrement", "ffp_slDamageAmount", "ffp_fFlameLifetime",
                           "ffp_fFlameEntityLifetime", "ffp_ctMaxFlames"],
    "CStaticBulletTrail": ["sbt_fLifeTime", "sbt_fTailDelayRatio", "sbt_fHeadDelayRatio", "sbt_fStartWidth", "sbt_fEndWidth", "sbt_fTextureLength"],
    "CMovingBulletTrail": ["mbt_fTrailLength", "mbt_fTrailVelocity", "mbt_fTrailWidth"],
    "CBulletParameters": ["bp_fDefaultBulletVelocity", "bp_fDefaultBulletReach", "bp_fDefaultBulletForceImpulse", "bp_fBulletUpImpulseRatio",
                          "bp_fBulletRayDiameter", "bp_sbtStaticBulletTrail", "bp_mbtMovingBulletTrail"],
    "CLaunchVelocities": ["lv_bRelativeToAttachment", "lv_fMinVelocity", "lv_fMaxVelocity", "lv_fMinHeading", "lv_fMaxHeading", "lv_fMinPitch",
                          "lv_fMaxPitch", "lv_fMinRotationH", "lv_fMaxRotationH", "lv_fMinRotationP", "lv_fMaxRotationP", "lv_fMinRotationB",
                          "lv_fMaxRotationB"],
    "SMarkersLauncher": ["ml_fLaunchPower", "ml_bClosestToPlayer", "ml_bCTPFlagConverted", "ml_wfmWeightedFormationMarkers",
                         "ml_pgrSpawnSourceLaunchPitchOverDistance"],
    "SRelativeRandomConeLauncher": ["rcl_fMinPitch", "rcl_fMaxPitch", "rcl_fAngleH", "rcl_fMinVelocity", "rcl_fMaxVelocity"],
    "SCrumbsProperties": ["ep_fGapSize", "ep_fRandomGapMargin", "ep_fMaxDistance", "ep_fFadeOutLength", "ep_pccConfig", "ep_pclLayout",
                          "ep_hepTarget", "ep_hepLight", "*ep_bPreviewTiles"],
    "SCrumbsEntity": ["en_hre", "en_hreEdt"],
    "SBulletShooterEntity": ["en_qvShootOrigin", "en_fBulletShootDistance", "en_slBulletDamage", "en_dtDamageType", "en_ctBulletsToFire",
                             "en_henOwner", "en_btBulletType", "en_fBulletVelocity", "en_fBulletImpulse", "en_fBulletImpulseUpRatio",
                             "en_vPatternStretch", "en_fScatteringDistance", "en_fScatteringRangeMin", "en_fScatteringRangeMax",
                             "en_fPredictionRatio", "en_fRayRadius", "en_diwDamageInflictorWeapon"],
}
# Classes of the family that the binary registers with a member table but that no file stores (member counts quoted for the spec).
BINARY_ONLY = {"SBaseProjectileEntity": 35, "SBaseProjectileProperties": 11, "SBulletEntity": 12, "SBulletProperties": 9,
               "SBulletTrailEntity": 3, "SBulletTrailProperties": 2, "SDebrisEntity": 27, "SDebrisProperties": 11,
               "SGenericProjectileEntity": 1, "SGenericProjectileProperties": 1}
# Hungarian prefix of a member name -> acceptable data type names (the data type name is a name in the file's own type table).
FITS = {"pmc": ("Ptr<CModelConfiguration>",), "ul": ("ProjectileFlags",), "f": ("FLOAT",), "b": ("BOOL",), "v": ("Vector2f", "Vector3f"),
        "sl": ("SLONG",), "ct": ("INDEX", "SLONG"), "id": ("IDENT",), "dt": ("EDamageType",), "pn": ("ProjectileNature",),
        "en": (), "qv": ("QuatVect",), "bt": ("EBulletType",), "diw": ("DamageInflictorWeapon",)}


def prefix(name):
    """The Hungarian prefix of `<class>_<prefix><Name>` (the lower-case letters after the underscore)."""
    tail = name.lstrip("*").split("_", 1)[1]
    i = 0
    while i < len(tail) and tail[i].islower(): i += 1
    return tail[:i]


def twin(name):
    return [name] + (["S" + name[1:]] if name[0] == "C" else [])


def ranges(values):
    nums = [v for v in values if isinstance(v, (int, float))]
    return f"{min(nums):g} .. {max(nums):g}" if nums and len(nums) == len(values) else "-"


def struct_values(install, ext, struct):
    """Every `struct` member value over all files of the extension: {member: Counter(values)}, number of files, number of struct objects."""
    vals = collections.defaultdict(collections.Counter); files = objn = 0
    for name, d in sorted(merged_files(install, ext).items()):
        r = parse(d)
        if not any(t.name == struct for t in r["types"].values()): continue
        objs, _ = TreeDec(d, r).objects()
        rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}
        seen = False
        def walk(o):
            nonlocal objn, seen
            for k, v in o:
                if isinstance(v, Obj):
                    if v.type == struct:
                        objn += 1; seen = True
                        for kk, vv in wv.flat(v, objs, ex, r["idnt"]): vals[kk.split(".")[-1]][vv] += 1
                    else: walk(v)
        for o in objs.values():
            if isinstance(o, Obj): walk(o)
        files += seen
    return vals, files, objn


def world_instances(install):
    """Instances of the world-stored classes of the family: count, number of worlds, and value counts of the named members."""
    want = {"SMarkersLauncher", "SRelativeRandomConeLauncher", "SBulletShooterEntity", "SCrumbsEntity"}
    vals = {w: collections.defaultdict(collections.Counter) for w in want}; cnt = collections.Counter(); worlds = collections.Counter()
    owners = collections.Counter()
    for name, d in sorted(merged_files(install, ".wld").items()):
        r = parse(d)
        if not ({t.name for t in r["types"].values()} & want): continue
        objs, _ = TreeDec(d, r).objects()
        rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}
        seen = set()
        def visit(o):
            if o.type in want:
                cnt[o.type] += 1; seen.add(o.type)
                for k, v in o:
                    if k.partition(".")[2] and not k.partition(".")[2].isdigit() and not isinstance(v, Obj):
                        vals[o.type][k.partition(".")[2]][str(v)] += 1
                    if o.type == "SBulletShooterEntity" and k.endswith(".6") and isinstance(v, Ref) and v.oid in objs: owners[objs[v.oid].type] += 1
        def walk(o):
            for k, v in o:
                if isinstance(v, Obj): visit(v); walk(v)
                elif isinstance(v, list):
                    for x in v:
                        if isinstance(x, Obj): visit(x); walk(x)
        for o in objs.values():
            if isinstance(o, Obj): visit(o); walk(o)
        for t in seen: worlds[t] += 1
    return cnt, worlds, vals, owners


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    wanted = {n for c in FAMILY for n in twin(c)} | set(OTHER_NAMES.values()) | set(BIN) | set(BINARY_ONLY) | {"CBulletFireing", "CCharacterTool"}
    cat, nfiles = wcv.collect(install, wanted)
    print(f"type tables of {nfiles} format-8 files")

    print(f"\n1. the {len(FAMILY)} exported classes of the family and the data")
    indata = 0; rows = []
    for c in FAMILY:
        found = [n for n in twin(c) + ([OTHER_NAMES[c]] if c in OTHER_NAMES else []) if n in cat]
        if not found: rows.append((c, None, False)); continue
        n = found[0]; rec = cat[n]
        (parent, mem), _ = rec["variants"].most_common(1)[0]
        exts = ",".join(f"{e.lstrip('.')}:{k}" for e, k in rec["exts"].most_common(3))
        by_twin = n in twin(c)
        indata += by_twin
        rows.append((c, f"saved as {n:28s} base {str(parent):24s} {len(mem):2d} members, {rec['files']:3d} files ({exts}), {len(rec['variants'])} variant(s)", by_twin))
    for c, s, _ in rows: print(f"   {c:30s} {s or 'not in the data'}")
    other = [(c, OTHER_NAMES[c]) for c, s, t in rows if s and not t]
    print(f"   in the data under the exported name or its S twin: {indata} of {len(FAMILY)}; under another registered name: {other}; "
          f"neither: {sum(1 for c, s, t in rows if not s)}")
    print("   classes the binary registers with a member table but no file stores:",
          {n: (k, n in cat) for n, k in BINARY_ONLY.items()})
    print(f"   none of them in the data: {not any(n in cat for n in BINARY_ONLY)}")

    print("\n2. member order: the data's members against the binary's declared names")
    ok_all = 0
    for cls, names in BIN.items():
        if cls not in cat: print(f"   {cls}: not in the data"); continue
        (parent, mem), _ = cat[cls]["variants"].most_common(1)[0]
        stored = [n for n in names if not n.startswith("*")]
        dn = [m for m, _ in mem]
        same_count = len(dn) == len(stored)
        named_ok = all(m == stored[i] for i, m in enumerate(dn) if not m.isdigit()) if same_count else False
        types_ok = True; bad = []
        for i, (m, t) in enumerate(mem):
            if i >= len(stored): break
            fit = FITS.get(prefix(stored[i]))
            ok = (not fit) or t in fit or t.startswith("Enum") or t.startswith("Ptr<") or t.startswith("Handle<") or t.startswith("C") or t.startswith("S")
            if fit and t not in fit and not t.startswith(("Enum", "Ptr<", "Handle<")): ok = False
            if not ok: types_ok = False; bad.append((m, stored[i], t))
        nnum = sum(1 for m in dn if m.isdigit())
        print(f"   {cls:30s} data {len(dn):2d} members ({nnum:2d} number-only), binary {len(stored):2d} stored (+{len(names) - len(stored)} editor-only); "
              f"same count: {same_count}; named members at the binary's position: {named_ok}; types fit the prefix: {types_ok} {bad if bad else ''}; "
              f"definitions in the data: {len(cat[cls]['variants'])}")
        ok_all += same_count and named_ok and types_ok
    print(f"   classes where all three checks hold: {ok_all} of {len(BIN)}")
    (p, mem), _ = cat["SBaseProjectileParams"]["variants"].most_common(1)[0]
    print("   SBaseProjectileParams number -> name:", {m: BIN["SBaseProjectileParams"][i] for i, (m, _) in enumerate(mem) if m.isdigit()})

    print("\n3. the projectile parameter files (Databases/GenericProjectiles)")
    eps = merged_files(install, ".ep")
    bn = {}  # name of the number-only members at class level p
    for i, (m, _) in enumerate(mem):
        if m.isdigit(): bn["p." + m] = BIN["SBaseProjectileParams"][i]
    gm = {"gpp_pnProjectileNature": "gpp_pnProjectileNature", "gpp_slHealth": "gpp_slHealth"}
    files = {}; folders = collections.Counter(); classes = collections.Counter()
    for name, d in sorted(eps.items()):
        if "genericprojectiles" not in name: continue
        r, objs, ex, rt = wv.load(d); classes[rt.type] += 1
        folders[name.split("genericprojectiles/")[1].split("/")[0]] += 1
        files[name] = {bn.get(k, k): v for k, v in wv.flat(rt, objs, ex, r["idnt"])}
    print(f"   {sum(classes.values())} files, classes {dict(classes)}, folders {dict(folders)}")
    cols = collections.defaultdict(collections.Counter)
    for f in files.values():
        for k, v in f.items(): cols[k][str(v) if isinstance(v, tuple) else v] += 1
    print("   member (binary name): distinct values, range, commonest values (count)")
    for k in BIN["SBaseProjectileParams"] + BIN["SGenericProjectileParams"]:
        c = cols.get(k)
        if not c: print(f"   {k}: absent"); continue
        top = ", ".join(f"{v}({n})" for v, n in c.most_common(4))
        print(f"   {k:34s} {len(c):3d}  {ranges(list(c)):16s} {top}")
    flags = collections.Counter(f["bp_ulFlags"] for f in files.values())
    nat = collections.Counter(f["gpp_pnProjectileNature"] for f in files.values())
    seek = [f for f in files.values() if f["bp_ulFlags"] == "enum 2"]
    print(f"   flags {dict(flags)} (1 = PF_FALLING, 2 = PF_SEEK_TARGET); nature {dict(nat)} (0 = PN_PHYSICAL, 1 = PN_MAGIC)")
    print(f"   files with PF_SEEK_TARGET: {len(seek)}; of them with seek frequency 0: {sum(1 for f in seek if f['bp_fSeekFrequency'] == 0)}; "
          f"files without it but with a seek frequency above 0: {sum(1 for f in files.values() if f['bp_ulFlags'] != 'enum 2' and f['bp_fSeekFrequency'] > 0)}")
    print(f"   piercing: files with PiercingPowerMin above 0: {sum(1 for f in files.values() if f['bp_slPiercingPowerMin'] > 0)}; "
          f"min -1: {sum(1 for f in files.values() if f['bp_slPiercingPowerMin'] == -1)}")
    pairs = [("slDirectDamage", "slDirectDamageMax"), ("slAreaDamage", "slAreaDamageMax"), ("fDamageHotspot", "fDamageHotspotMax"),
             ("fDamageFalloff", "fDamageFalloffMax"), ("fStretch", "fStretchMax")]
    for lo, hi in pairs:
        a, b = "bp_" + lo, "bp_" + hi
        print(f"   pair {lo}/{hi}: max equal to min in {sum(1 for f in files.values() if f[a] == f[b])}, max 0 with min above 0 in "
              f"{sum(1 for f in files.values() if f[b] == 0 and f[a] > 0)}, max above min in {sum(1 for f in files.values() if f[b] > f[a])}, "
              f"max below min but not 0 in {sum(1 for f in files.values() if 0 < f[b] < f[a])} of {len(files)}")
    print(f"   -1 means 'use the global default' (impulse, flying and explode sound fall-off and hot spot): "
          + ", ".join(f"{k[3:]} {sum(1 for f in files.values() if f[k] == -1)}" for k in
                      ("bp_fImpactForceImpulse", "bp_fRangeForceImpulse", "bp_fFlyingSoundFallOff", "bp_fFlyingSoundHotSpot",
                       "bp_fExplodeSoundFallOff", "bp_fExplodeSoundHotSpot")))
    print(f"   health: -1 in {sum(1 for f in files.values() if f['gpp_slHealth'] == -1)}, above 0 in {sum(1 for f in files.values() if f['gpp_slHealth'] > 0)}; "
          f"magic nature with health above 0: {sum(1 for f in files.values() if f['gpp_pnProjectileNature'] == 'enum 1' and f['gpp_slHealth'] > 0)}")
    flamer = [(n, f) for n, f in files.items() if n.endswith("flamer.ep")]
    print("   the flamer's projectile file(s):", [(n.rsplit("/", 1)[1], f["bp_fLifeTime"], f["bp_slDirectDamage"], f["bp_fFlyingSpeed"]) for n, f in flamer],
          "class", [r for r in classes if r.startswith("CFlamer")])

    print("\n4. other data of the family")
    print("   SBaseWeaponParams.bwp_pgppProjectileParams (weapon-classes.md): see weapon_classes_verify.py")
    vals, nf, no = struct_values(install, ".ep", "SPuppetProjectileParams")
    (p, pm), _ = cat["SPuppetProjectileParams"]["variants"].most_common(1)[0]
    print(f"   SPuppetProjectileParams: {no} objects in {nf} .ep files (member pp_pppProjectileParams of the puppet parameters); "
          "data number -> binary name: distinct values, range, commonest")
    for i, (m, _) in enumerate(pm):
        c = vals.get(m, {})
        print(f"      .{m:2s} {BIN['SPuppetProjectileParams'][i]:34s} {len(c):2d}  {ranges(list(c)):12s} "
              + ", ".join(f"{v}({n})" for v, n in collections.Counter(c).most_common(4)))
    rsc = [(n, d) for n, d in merged_files(install, ".rsc").items() if n.endswith("globalgameparameters.rsc")]
    if rsc:
        r, objs, ex, rt = wv.load(rsc[0][1]); fl = dict(wv.flat(rt, objs, ex, r["idnt"]))
        print("   global parameters (globalgameparameters.rsc), bullets:",
              {k.split(".", 1)[1]: v for k, v in fl.items() if k.startswith("ggp_bpBulletParams.")})
        print("   global parameters, flames (data number -> binary name):",
              {BIN["CFlamerFlameParams"][i]: fl.get(k) for i, k in enumerate(
                  ["ggp_ffpFlamerFlameParams.w.%s" % n for n in ("1", "11", "2", "3", "4", "5.gh_fValue", "6", "7", "8", "9", "10")])})
        print("   global parameters, projectile defaults:",
              {k: v for k, v in fl.items() if k in ("ggp_fProjectileThreatDistance", "ggp_fWeaponFireThreatDistance",
                                                      "ggp_fProjectileDefFlyingSoundFallOff", "ggp_fProjectileDefFlyingSoundHotSpot",
                                                      "ggp_fProjectileDefExplodeSoundFallOff", "ggp_fProjectileDefExplodeSoundHotSpot")})
    vals, nf, no = struct_values(install, ".ctl", "CLaunchVelocities")
    print(f"   CLaunchVelocities: {no} objects in {nf} .ctl files (member ct_lvLaunchVelocities of CCharacterTool); member: distinct values, range, commonest")
    for k in BIN["CLaunchVelocities"]:
        c = vals.get(k, {})
        print(f"      {k:26s} {len(c):2d}  {ranges(list(c)):14s} " + ", ".join(f"{v}({n})" for v, n in collections.Counter(c).most_common(3)))
    cnt, worlds, wv_, owners = world_instances(install)
    print("\n5. stored instances in the worlds: instances / worlds")
    for t in sorted(cnt): print(f"   {t:30s} {cnt[t]:4d} / {worlds[t]}")
    for t in ("SBulletShooterEntity", "SMarkersLauncher"):
        for k, c in wv_[t].items(): print(f"      {t}.{k}: " + ", ".join(f"{v}({n})" for v, n in c.most_common(4)))
    print("   owners of the stored shooters (type of the embedded puppet):", dict(owners))
    print("   CBulletFireing (the bullet description inside blasts, .ep):", cat["CBulletFireing"]["files"], "files")


if __name__ == "__main__":
    main()
