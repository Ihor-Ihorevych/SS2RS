"""Evidence for docs/re/weapon-classes.md (T01-106): what the data says about the weapon classes of Sam2Game.dll.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries).

It prints
  1. for each of the 83 exported classes of the family (names quoted from the export table of Sam2Game.dll): whether it is in the
     data (own name, or the `S` twin of a `C` name), its base class, own member count, number of files and file extensions;
  2. the member-order check (section 4 of the spec): for every data class whose member table the binary also declares, the data's
     members in file order against the binary's declared names. A data member that has only a number is given the binary's name at
     the same position; the check is that the counts are equal, that every member the data does name sits at the position of the same
     name in the binary's table, and that the data type fits the Hungarian prefix of the binary name;
  3. the weapon files: which class every `*weapon.ep` / `*weapon_dm.ep` / `*weaponrendering.ep` file has, and a table of the 22
     base members with the binary's names (single player; the deathmatch value is shown where it differs);
  4. the files that carry `CThrowDescription` and `CMountedTool`.

Usage: python -I tools/re/weapon_classes_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR; about 3 minutes)
"""
import collections, os, sys, zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import _natural, merged_files, parse, u32  # noqa: E402
import weapons_verify  # noqa: E402

# The 83 classes exported by Sam2Game.dll that make up this family (T01-026 split, part T01-106).
WEAPONS = ["AutoShotgun", "BeamGun", "Cannon", "CircularSawBlades", "CircularSaw", "Colt", "DoubleShotgun", "Flamer", "GrenadeLauncher",
           "KlodovikGun", "MiniGun", "PlasmaRifle", "RocketLauncher", "SeriousBomb", "Sniper", "Uzi", "ZapGun"]
FAMILY = (["CBaseWeaponEntity", "CBaseWeaponParams", "CBaseWeaponProperties", "CBaseWeaponRenderingParams", "CBaseScatterWeaponParams",
           "CAdjustWeaponMode", "CMountedTool", "CThrowDescription", "CWeaponItemEntity", "CWeaponItemProperties",
           "CComboWeaponsItemEntity", "CComboWeaponsItemProperties", "CCannonItemEntity", "CCannonItemProperties",
           "CCannonBallsItemEntity", "CCannonBallsItemProperties"]
          + [f"C{w}Weapon{k}" for w in WEAPONS for k in ("Entity", "Params", "Properties", "RenderingParams")
             if not (w == "CircularSawBlades" and k == "RenderingParams")])

# Member names that the binary's member tables declare, in declaration order (names only; the binary's own identifiers).
BIN = {
    "SBaseWeaponParams": ["bwp_pmcModel", "bwp_pctLeftHandTool", "bwp_pctRightHandTool", "bwp_ctFiresToReload", "bwp_fChargingTime",
                          "bwp_fChargingAutoFireTime", "bwp_fMinRecoilDuration", "bwp_fMaxRecoilDuration", "bwp_fReloadDuration",
                          "bwp_fOffsetX", "bwp_fOffsetY", "bwp_fOffsetZ", "bwp_fFrequencyX", "bwp_fFrequencyY", "bwp_fOffsetZExponent",
                          "bwp_fLaunchVelocityMin", "bwp_fLaunchVelocityMax", "bwp_fDoubleWeaponFireDelay", "bwp_fSpinUpTime",
                          "bwp_fSpinDownTime", "bwp_fSpinSpeed", "bwp_pgppProjectileParams"],
    "SBaseScatterWeaponParams": ["bswp_btBulletType", "bswp_fScatterDistance", "bswp_fScatterRadiusMin", "bswp_fScatterRadiusMax",
                                 "bswp_vPatternStretch", "bswp_fBulletVelocity", "bswp_fBulletRange", "bswp_fBulletImpulse",
                                 "bswp_fBulletImpulseUpRatio"],
    "SBaseWeaponRenderingParams": ["bwrp_vPosition", "bwrp_vAngle", "bwrp_fFOV", "bwrp_fBobbingMul", "bwrp_vPositionDoubleWeapon",
                                   "bwrp_vAngleDoubleWeapon", "bwrp_fFOVDoubleWeapon", "bwrp_fFrontClipDist"],
    "SAutoShotgunWeaponParams": ["aswp_ctBullets", "aswp_slBulletDamage"],
    "SBeamGunWeaponParams": ["bwp_slDamage", "bwp_fDamageFrequency", "bwp_fDamageRadius", "bwp_fMaxRange", "bwp_fDrainAmmoFrequency",
                             "bwp_pmcModel", "bwp_idHitMaterialEffect", "bwp_pefEffect", "bwp_idFireAnim", "bwp_idSoundTerm",
                             "bwp_idFirePressedSoundTerm", "bwp_idFireReleaseSoundTerm"],
    "SCircularSawWeaponParams": ["cswp_fCuttingRange", "cswp_fCuttingWide", "cswp_fCuttingForceImpulse", "cswp_pbsEngineStart",
                                 "cswp_pbsEngineLoop", "cswp_pbsEngineStop", "cswp_pbsCuttingStart", "cswp_pbsCuttingLoop",
                                 "cswp_pbsCuttingStop"],
    "SColtWeaponParams": ["cwp_slBulletDamage"],
    "SDoubleShotgunWeaponParams": ["dswp_ctBullets", "dswp_slBulletDamage"],
    "SMiniGunWeaponParams": ["mgwp_slBulletDamage", "mgwp_pbsFire", "mgwp_pbsClick", "mgwp_pbsSpin", "mgwp_pbsSpinUp", "mgwp_pbsSpinDown"],
    "SPlasmaRifleWeaponParams": ["prwp_iAmpuleSegments", "prwp_fRefillLength", "prwp_fReleaseLength"],
    "SSniperWeaponParams": ["swp_slBulletDamage", "swp_slBulletDamageInZoomMode", "swp_fMinZoomFactor", "swp_fMaxZoomFactor",
                            "swp_fFullZoomingTime"],
    "SUziWeaponParams": ["uwp_slBulletDamage"],
    "SZapGunWeaponParams": ["zgwp_ctMaxFirePowerWorthBullets"],
    "SWeaponItemEntity": ["en_wiWeaponIndex", "en_pbwpWeaponParams", "en_pbwpWeaponRenderParams", "en_slAmmoQuantity"],
}

# Hungarian prefix of a binary member name -> acceptable data type names (the data type name is a name in the file's own type table).
FITS = {"pmc": ("Ptr<CModelConfiguration>",), "pct": ("Ptr<CCharacterTool>",), "pgpp": ("Ptr<CGenericProjectileParams>",),
        "ct": ("INDEX", "SLONG"), "sl": ("SLONG",), "i": ("SLONG", "INDEX"), "f": ("FLOAT",), "v": ("Vector2f", "Vector3f"),
        "id": ("IDENT",), "pbs": (), "pef": (), "bt": (), "wi": ("WeaponIndex",), "pbwp": (), "en": ()}


def prefix(name):
    """The Hungarian prefix of `<class>_<prefix><Name>` (the lower-case letters after the underscore)."""
    tail = name.split("_", 1)[1]
    i = 0
    while i < len(tail) and tail[i].islower(): i += 1
    return tail[:i]


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


def twin(name):
    return [name] + (["S" + name[1:]] if name[0] == "C" else [])


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    wanted = {n for c in FAMILY for n in twin(c)} | set(BIN) | {"CFlamerWeaponParams", "SBaseWeaponProperties", "SBaseWeaponEntity"}
    cat, nfiles = collect(install, wanted)
    print(f"type tables of {nfiles} format-8 files")

    print(f"\n1. the {len(FAMILY)} exported classes of the family and the data")
    indata = 0; rows = []
    for c in FAMILY:
        found = [n for n in twin(c) if n in cat]
        if not found: rows.append((c, None)); continue
        indata += 1; n = found[0]; rec = cat[n]
        (parent, mem), _ = rec["variants"].most_common(1)[0]
        exts = ",".join(f"{e.lstrip('.')}:{k}" for e, k in rec["exts"].most_common(3))
        rows.append((c, f"saved as {n:36s} base {str(parent):26s} {len(mem):2d} members, {rec['files']:4d} files ({exts}), {len(rec['variants'])} variant(s)"))
    for c, s in rows: print(f"   {c:40s} {s or 'not in the data'}")
    saved_c = [c for c, s in rows if s and s.split()[2] == c]
    print(f"   in the data: {indata}; binary only: {len(FAMILY) - indata}; saved under the C name of the export: {saved_c}; as the S twin: {indata - len(saved_c)}")

    print("\n2. member order: the data's members against the binary's declared names")
    ok_all = 0
    for cls, names in BIN.items():
        if cls not in cat: print(f"   {cls}: not in the data"); continue
        (parent, mem), _ = cat[cls]["variants"].most_common(1)[0]
        dn = [m for m, _ in mem]; dt = [t for _, t in mem]
        same_count = len(dn) == len(names)
        named_ok = all(m == names[i] for i, m in enumerate(dn) if not m.isdigit()) if same_count else False
        types_ok = []; bad = []
        for i, (m, t) in enumerate(mem):
            if i >= len(names): break
            fit = FITS.get(prefix(names[i]))
            if fit: ok = t in fit or t.startswith("Enum") or t.startswith("Ptr<")
            else: ok = True
            types_ok.append(ok)
            if not ok: bad.append((m, names[i], t))
        nnum = sum(1 for m in dn if m.isdigit())
        print(f"   {cls:30s} data {len(dn):2d} members ({nnum:2d} number-only), binary {len(names):2d}; same count: {same_count}; "
              f"named members at the binary's position: {named_ok}; types fit the prefix: {all(types_ok)} {bad if bad else ''}")
        ok_all += same_count and named_ok and all(types_ok)
    print(f"   classes where all three checks hold: {ok_all} of {len(BIN)}")
    if "CFlamerWeaponParams" in cat:
        print("   CFlamerWeaponParams (saved under its C name): definitions in the data as (members, files): "
              + str([([m for m, _ in mem], k) for (parent, mem), k in cat["CFlamerWeaponParams"]["variants"].most_common()])
              + "; the binary declares no member for the class")
    # name of the number-only members of the base weapon parameters
    (p, mem), _ = cat["SBaseWeaponParams"]["variants"].most_common(1)[0]
    mapping = {m: BIN["SBaseWeaponParams"][i] for i, (m, _) in enumerate(mem) if m.isdigit()}
    (p, mem), _ = cat["SBaseScatterWeaponParams"]["variants"].most_common(1)[0]
    smap = {m: BIN["SBaseScatterWeaponParams"][i] for i, (m, _) in enumerate(mem) if m.isdigit()}
    print("   SBaseWeaponParams number -> name:", mapping)
    print("   SBaseScatterWeaponParams number -> name:", smap)
    (p, mem), _ = cat["SBaseWeaponRenderingParams"]["variants"].most_common(1)[0]
    rmap = {m: BIN["SBaseWeaponRenderingParams"][i] for i, (m, _) in enumerate(mem) if m.isdigit()}
    print("   SBaseWeaponRenderingParams number -> name:", rmap)

    print("\n3. the weapon files")
    eps = merged_files(install, ".ep"); by = {n.split("databases/")[-1]: d for n, d in eps.items()}
    kinds = collections.defaultdict(collections.Counter)
    weapons = {}
    for rel in sorted(by):
        if not rel.startswith("entityparams/") or "items/" in rel or "copy" in rel: continue
        base = rel.rsplit("/", 1)[-1]
        if "weapon" not in base: continue
        r, objs, ex, rt = weapons_verify.load(by[rel])
        kind = "rendering" if "rendering" in base else ("deathmatch" if base.endswith("weapon_dm.ep") else "single player")
        kinds[kind][rt.type] += 1
        if kind != "rendering": weapons.setdefault(base.replace("weapon_dm.ep", "").replace("weapon.ep", ""), {})["dm" if kind == "deathmatch" else "sp"] = (rt.type, dict(weapons_verify.flat(rt, objs, ex, r["idnt"])))
    for k, c in kinds.items(): print(f"   {k}: {sum(c.values())} files, classes {dict(c)}")
    print("\n   base members by binary name: single player value (deathmatch value where it differs)")
    cols = ["ctFiresToReload", "ChargingTime", "ChargingAutoFireTime", "MinRecoilDuration", "MaxRecoilDuration", "ReloadDuration",
            "LaunchVelocityMin", "LaunchVelocityMax", "DoubleWeaponFireDelay", "SpinUpTime", "SpinDownTime", "SpinSpeed"]
    bn = {v: k for k, v in mapping.items()}
    print("   " + "weapon".ljust(18) + " ".join(c[:11].rjust(11) for c in cols))
    for key, w in sorted(weapons.items()):
        if "sp" not in w: continue
        cells = []
        for c in cols:
            num = bn["bwp_" + ("f" if not c.startswith("ct") else "") + c]
            sp = w["sp"][1].get("b." + num); dm = w.get("dm", (0, {}))[1].get("b." + num, sp)
            cells.append((f"{sp:g}" if isinstance(sp, (int, float)) else "-") + ("" if dm == sp else f"({dm:g})"))
        print("   " + key.ljust(18) + " ".join(c.rjust(11) for c in cells))
    print("\n   scatter members by binary name (bullet weapons): single player (deathmatch)")
    sb = {v: k for k, v in smap.items()}
    for key, w in sorted(weapons.items()):
        if "sp" not in w or not any(k.startswith("s.") for k in w["sp"][1]): continue
        cells = []
        for c in ("btBulletType", "fScatterDistance", "fScatterRadiusMin", "fScatterRadiusMax", "fBulletVelocity", "fBulletRange"):
            num = sb["bswp_" + c]; sp = w["sp"][1].get("s." + num); dm = w.get("dm", (0, {}))[1].get("s." + num, sp)
            cells.append(f"{c[1:] if c[0] == 'f' else c}={sp}" + ("" if dm == sp else f"({dm})"))
        print("   " + key.ljust(16) + "  ".join(cells))

    print("\n4. files with CThrowDescription and CMountedTool")
    for cls in ("CThrowDescription", "CMountedTool"):
        rec = cat.get(cls)
        print(f"   {cls}: {rec['files'] if rec else 0} files {dict(rec['exts']) if rec else ''}")


if __name__ == "__main__":
    main()
