"""Evidence for docs/re/hud-classes.md and its three companion pages (T01-111): what the data says about the HUD classes of Sam2Game.dll.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries).

It prints
  1. for each exported class of the HUD family (names quoted from the export table of Sam2Game.dll; 95 classes of the T01-026 split plus the
     four player-death and voting classes that `players-session-classes.md` counts): the name under which the data stores it (own name, or
     the name the DLL registers it under), whether it is in the data, its base class, its own member count, and in how many files;
  2. the member-order check: for every data class whose member table the binary also declares, the data's members in file order against the
     binary's declared names (counts equal, every member the data names at the position of the same name, every data type fitting the
     Hungarian prefix of the binary's name), and the number of data members that only the binary can name;
  3. the census of `Databases/HudParams.rsc` (the HUD parameter database): its 44 entries, their base classes, positions (the 640 x 480
     canvas), render modes, the stacking flag, the shared text, icon and bar configurations, the textures and fonts the entries refer to;
  4. `Databases/SimpleHudBarParams.rsc` (the bar parameters);
  5. the ammunition and weapon icon files (`Textures/Interface/AmmoIcons`, `Textures/Interface/WeaponIcons`) against the enumeration captions.

Usage: python -I tools/re/hud_classes_verify.py [DATA_DIR] [--all]
       (DATA_DIR defaults to $SS2_DATA_DIR; without --all only the 112 `.rsc` files are scanned for the class table (a few seconds);
        --all scans all format-8 files (about 4 minutes) and gives the "files" column over the whole install)
"""
import collections, os, re, sys, zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Enum, Obj, Ref, TreeDec, _natural, merged_files, parse, u32  # noqa: E402

NULL = 0xFFFFFFFF
HUDPARAMS = "content/serioussam2/databases/hudparams.rsc"
BARPARAMS = "content/serioussam2/databases/simplehudbarparams.rsc"

# The 95 classes of the T01-026 split (family "hud") plus the four classes of players-session-classes.md section 5.3 / 9 (exported names).
ELEMENTS_SIMPLE = ["Adrenaline", "Air", "Armor", "Bombs", "Boss", "ComboWeapons", "Counter", "Frozen", "Health", "Lives", "Points", "Time",
                   "Vehicle", "VehicleAmmo"]
ELEMENTS_POWERUP = ["SeriousDamage", "SeriousJump", "SeriousScore", "SeriousSpeed", "SeriousStrength", "Invisibility", "Invulnerability"]
ELEMENTS_OTHER = ["Ammo", "LeftAmmo", "RightAmmo", "AutoAim", "BackTemplate", "Conversation", "Damage", "EnemyPurged", "ExtraLife",
                  "FragsCounter", "Freezing", "HorizontalWeaponsList", "ItemPicked", "Netricsa", "NetricsaIcon", "ObjectAction",
                  "OnlineNotification", "Saving", "Scanner", "TimeRemain", "WeaponsList", "WeaponsMenu"]


def element_class(stem):
    suffix = "SimpleHudElement" if stem in ELEMENTS_SIMPLE else "HudElement"
    return "C" + stem + suffix


FAMILY_ELEMENTS = ["CHudElement", "CSimpleHudElement", "CPowerUpHudElement"] + [element_class(s) for s in
                   ELEMENTS_SIMPLE + ELEMENTS_POWERUP + ELEMENTS_OTHER]
FAMILY_PARAMS = [c + "Params" for c in FAMILY_ELEMENTS if c != "CPowerUpHudElement"]
FAMILY = FAMILY_ELEMENTS + FAMILY_PARAMS + ["CHudElementsParamsDatabase", "CSimpleHudBarParams", "CAmmoIcons", "CWeaponIcons"]
OTHER_PART = ["CPlayerDeathHudElement", "CPlayerDeathHudElementParams", "CVotingInfoHudElement", "CVotingInfoHudElementParams"]
# classes the DLL registers but does not export (table names only): the HUD container and the three configuration records
NOT_EXPORTED = ["CHUD", "SHUD", "CHudTextConfig", "SHudNumberConfig", "CHudIconConfig", "SHudIconConfig", "CHudBarConfig", "SHudBarConfig",
                "CCommonSimpleHudElmParams"]

# Name under which the data stores a parameter class when it is not the exported name (read from the DLL's class registrations):
# the S twin for most, and two renamed classes. Classes absent here are saved under their own name (when saved at all).
S_TWIN = {c + "Params": "S" + c[1:] + "Params" for c in
          ["CAdrenalineSimpleHudElement", "CAirSimpleHudElement", "CArmorSimpleHudElement", "CBombsSimpleHudElement", "CBossSimpleHudElement",
           "CComboWeaponsSimpleHudElement", "CCounterSimpleHudElement", "CFrozenSimpleHudElement", "CHealthSimpleHudElement",
           "CLivesSimpleHudElement", "CPointsSimpleHudElement", "CTimeSimpleHudElement", "CVehicleSimpleHudElement", "CHudElement",
           "CSimpleHudElement", "CAmmoHudElement", "CLeftAmmoHudElement", "CRightAmmoHudElement", "CBackTemplateHudElement",
           "CDamageHudElement", "CHorizontalWeaponsListHudElement", "CScannerHudElement", "CWeaponsListHudElement", "CWeaponsMenuHudElement"]}
RENAMED = {"CSeriousStrengthHudElementParams": "CStrengthSimpleHudElementParams", "CSeriousJumpHudElementParams": "CJumpSimpleHudElementParams"}

# Binary-declared member names (the DLL's static member tables, in declaration order; names only), keyed by the data class.
BIN = {
    "SHudElementParams": [
        "hep_vPosition", "hep_rheRender", "hep_bRenderOnCamera"],
    "SSimpleHudElementParams": [
        "shep_bRelToPrevious", "shep_hhtcTextCfg", "shep_hhicIconCfg", "shep_hhbcBarCfg", "shep_pshupBarParams"],
    "SAmmoHudElementParams": [
        "ahep_vIconRelPos", "ahep_fIconStretch", "ahep_bShowIcon", "ahep_vTextRelPos", "ahep_fTextSize", "ahep_bShowText",
        "ahep_haHorizontalAlign"],
    "SLivesSimpleHudElementParams": [
        "lshep_fStretchingDelay", "lshep_fStretchingPeriod", "lshep_fExtraStretch"],
    "SPointsSimpleHudElementParams": [
        "pshep_fStretchingPeriod", "pshep_fAdditionalStretch"],
    "COnlineNotificationHudElementParams": [
        "onhep_uwIconSize"],
    "SScannerHudElementParams": [
        "shep_fStretch", "shep_fRange", "shep_fStretchPoints", "shep_fStretchArrows", "shep_colEnemy", "shep_colNPC",
        "shep_colNeutral", "shep_colObjectives", "shep_colObjFailed", "shep_colBeacon", "shep_ptxScanner", "shep_ptxScannerPoint",
        "shep_ptxOutline", "shep_ptxScannerDir", "shep_ptxScannerArrow", "shep_vRightTop", "shep_vRightMiddle", "shep_vRightBottom",
        "shep_vCenterBottom", "shep_vLeftMiddle"],
    "SBackTemplateHudElementParams": [
        "bthep_ptxBackTemplate"],
    "CNetricsaIconHudElementParams": [
        "nihep_ptxNetricsa", "nihep_fStretch"],
    "CConversationHudElementParams": [
        "chep_ptfxcPlayerTextFXCfg", "chep_ptfxcPuppetTextFXCfg", "chep_ptfxcNetricsaTextFXCfg", "chep_bRenderTextRegions"],
    "SDamageHudElementParams": [
        "dhep_colDamageMin", "dhep_colDamageMax", "dhep_colDamageDir", "dhep_fDuration", "dhep_fWidth", "dhep_ptxDamage",
        "dhep_ptxDamageDir"],
    "SWeaponsListHudElementParams": [
        "wlhep_fStretchY", "wlhep_fWidth", "wlhep_fWeaponOffsetX", "wlhep_fQuantityOffsetX", "wlhep_fStayTime",
        "wlhep_ptxBarCorner", "wlhep_ptxBarTile", "wlhep_ptxQuantityBack", "wlhep_ptxNoAmmo", "wlhep_ptxQuantityFill",
        "wlhep_ptxBarCornerGlow", "wlhep_ptxBarTileGlow"],
    "SHorizontalWeaponsListHudElementParams": [
        "hwlhep_fStretchY", "hwlhep_fWidth", "hwlhep_fBarOffsetX", "hwlhep_fWeaponOffsetX", "hwlhep_fQuantityOffsetX",
        "hwlhep_fStayTime", "hwlhep_fNumberBarStretch", "hwlhep_fTextOffset", "hwlhep_colBackground", "hwlhep_fBackgroundHeight",
        "hwlhe_ptxBarCorner", "hwlhe_ptxBarTile", "hwlhe_ptxQuantityBack", "hwlhe_ptxNoAmmo", "hwlhe_ptxQuantityFill",
        "hwlhe_ptxBarCornerGlow", "hwlhe_ptxBarTileGlow", "hwlhe_ptxNumberBarCorner", "hwlhe_ptxNumberBarTile",
        "hwlhe_ptxNumberBarCornerGlow", "hwlhe_ptxNumberBarTileGlow", "hwlhe_pfntFont"],
    "SWeaponsMenuHudElementParams": [
        "wmhep_fStretch", "wmhep_fOffset0FromC", "wmhep_fOffset1FromC", "wmhep_fArrowLighting"],
    "CFreezingHudElementParams": [
        "fhep_ptxTexture", "fhep_ctMaxCount", "fhep_fMinSize", "fhep_fMaxSize", "fhep_gradColorOverLife"],
    "CAutoAimHudElementParams": [
        "aahep_ptxTexture"],
    "CObjectActionHudElementParams": [
        "oahep_fiActionMessage"],
    "CSavingHudElementParams": [
        "shep_fiMessage"],
    "CVotingInfoHudElementParams": [
        "shep_fiMessage"],
    "CExtraLifeHudElementParams": [
        "elhep_ptfxcExtraLife"],
    "CItemPickedHudElementParams": [
        "iphep_ptfxcPickedItem"],
    "CEnemyPurgedHudElementParams": [
        "hep_ptfxcEnemyPurged"],
    "CFragsCounterHudElementParams": [
        "fchep_ptfxcMessage"],
    "CTimeRemainHudElementParams": [
        "trhep_ptfxcMessage"],
    "CPlayerDeathHudElementParams": [
        "pdhep_fiMessage", "pdhep_fStayTime", "pdhep_fFadeOutTime", "pdhep_ctMaxDeaths"],
    "SHudNumberConfig": [
        "htc_vRelPos", "htc_haHorizontalAlign", "htc_fShakePower"],
    "SHudIconConfig": [
        "hic_vRelPos", "hic_fStretch", "hic_ptxTexture"],
    "CHudElementsParamsDatabase": [
        "hed_chpCommonSimpleHudElmParams", "hed_cepHudElementsParams"],
    "CCommonSimpleHudElmParams": [
        "cshep_fShakeFreqX", "cshep_fShakeFreqY", "cshep_fShakeDamp", "cshep_fBarStretchY"],
    "CSimpleHudBarParams": [
        "shbp_ptxBarFillTile", "shbp_ptxBarFillBoxCorner", "shbp_ptxBarFillCorner", "shbp_ptxBarTile", "shbp_ptxBarCorner",
        "shbp_ptxBarFillCorner2", "shbp_ptxBarFillTile2", "shbp_ptxBarHolder", "shbp_ptxBarHolderTile", "shbp_ptxBarHighlight",
        "shbp_fiFont"],
    "SHudBarConfig": [
        "hbc_vRelPos", "hbc_fTilingHolderLength", "hbc_fBarLength", "hbc_colHolder", "hbc_bFlipH", "hbc_grPhase1", "hbc_grPhase2"],
}
# The 96 members of CNetricsaHudElementParams are named in the data itself; they are checked against the binary's list by count only here
# (the data names every one of them: section 2 prints the number of named members).

FITS = {"b": ("BOOL",), "f": ("FLOAT",), "col": ("COLOR",), "v": ("Vector2f",), "ptx": ("Ptr<CTexture>", "Ptr<CStaticTexture>"),
        "pfnt": ("Ptr<CFont>",), "ptfxc": ("Ptr<CTextEffectConfiguration>",), "ct": ("INDEX",), "fi": ("CFontInstance",),
        "uw": ("UWORD",), "ha": ("HorizontalAlign",), "rhe": ("RenderHudElement",), "gr": ("CGradient",), "grad": ("CGradient",),
        "h": ("Handle<CHudTextConfig>", "Handle<CHudIconConfig>", "Handle<CHudBarConfig>")}
# The one binary name whose Hungarian prefix disagrees with the data type (a Vector2f declared with the prefix of a float); the data type wins.
PREFIX_EXCEPTIONS = {("SHorizontalWeaponsListHudElementParams", "hwlhep_fTextOffset"): "Vector2f"}

# Enumeration captions that the DLL declares (value, caption); the icon loader builds a file name from a caption by replacing blanks with "_".
AMMO = ["Blades", "Bullets", "Cannon balls", "Grenades", "Hand grenades", "Klodovik", "Plasma", "Rockets", "Serious bomb", "Shells",
        "Sniper bullets", "Napalm"]
WEAPONS = ["Circular_Saw", "Zap_Gun", "Auto_Shotgun", "Double_Shotgun", "Uzi", "Mini_Gun", "Rocket_Launcher", "Grenade_Launcher", "Plasma_Rifle",
           "Klodovik_Gun", "Cannon", "Serious_Bomb", "Colt", "Sniper", "Combo_Weapons", "Beam_Gun", "Flamer"]


def prefix(name):
    tail = name.split("_", 1)[1]
    i = 0
    while i < len(tail) and tail[i].islower(): i += 1
    return tail[:i]


def data_name(cls):
    return RENAMED.get(cls) or S_TWIN.get(cls) or cls


def scan_types(install, names, rsc_only):
    """{type name: {variants: Counter((parent, members)), files, exts}} over the format-8 files."""
    merged = {}
    for f in sorted((f for f in os.listdir(install) if f.lower().endswith(".gro")), key=_natural):
        with zipfile.ZipFile(os.path.join(install, f)) as z:
            for i in z.infolist():
                if i.is_dir() or i.file_size < 64: continue
                if rsc_only and not i.filename.lower().endswith(".rsc"): continue
                with z.open(i) as fh: head = fh.read(16)
                if head[:8] == b"CTSEMETA" and u32(head, 12) == 8: merged[i.filename.lower()] = (f, i.filename)
    by_arch = collections.defaultdict(list)
    for f, name in merged.values(): by_arch[f].append(name)
    out = {}
    for f, ns in by_arch.items():
        with zipfile.ZipFile(os.path.join(install, f)) as z:
            for name in ns:
                r = parse(z.read(name)); ext = os.path.splitext(name)[1].lower()
                for t in r["types"].values():
                    if t.members is None or t.name not in names: continue
                    parent = r["types"][t.parent].name if t.parent in r["types"] else None
                    mem = tuple((m, r["types"][mt].name if mt in r["types"] else "?") for m, mt in t.members)
                    rec = out.setdefault(t.name, {"variants": collections.Counter(), "files": set(), "exts": collections.Counter()})
                    rec["variants"][(parent, mem)] += 1; rec["files"].add(name.lower()); rec["exts"][ext] += 1
    return out, len(merged)


def decode(files, path):
    d = files[path]; r = parse(d); objs, _ = TreeDec(d, r).objects()
    return d, r, objs


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR")
    if not install: sys.exit("usage: hud_classes_verify.py DATA_DIR [--all]")
    scan_all = "--all" in sys.argv
    names = set(data_name(c) for c in FAMILY + OTHER_PART) | set(NOT_EXPORTED)
    found, nscanned = scan_types(install, names, rsc_only=not scan_all)
    print(f"== 1. classes (scanned {nscanned} format-8 files{'' if scan_all else ', .rsc only: use --all for the whole install'})")
    saved = collections.Counter()
    for group, items in [("elements (runtime, never saved)", FAMILY_ELEMENTS), ("parameter classes", FAMILY_PARAMS),
                         ("database, bar parameters, icon tables", ["CHudElementsParamsDatabase", "CSimpleHudBarParams", "CAmmoIcons", "CWeaponIcons"]),
                         ("players-session part", OTHER_PART)]:
        print(f"-- {group} ({len(items)})")
        for c in items:
            dn = data_name(c); rec = found.get(dn)
            if rec is None:
                print(f"  {c:42s} not in the data"); continue
            (parent, mem), _n = rec["variants"].most_common(1)[0]
            named = sum(1 for m, _t in mem if not m.isdigit())
            print(f"  {c:42s} saved as {dn:38s} base {str(parent):22s} members {len(mem):3d} (named {named:3d}) files {len(rec['files'])} variants {len(rec['variants'])}")
            if c in FAMILY: saved[c] += 1
    print(f"exported classes of the family: {len(FAMILY)}  (elements {len(FAMILY_ELEMENTS)}, parameter classes {len(FAMILY_PARAMS)}, "
          f"2 resources, 2 icon tables); saved in the data: {sum(saved.values())}")
    print("renamed in the data:", ", ".join(f"{k} -> {v}" for k, v in RENAMED.items()))
    print("-- classes the DLL registers but does not export")
    for c in NOT_EXPORTED:
        rec = found.get(c)
        print(f"  {c:30s} " + ("not in the data" if rec is None else f"in the data, members {len(next(iter(rec['variants']))[1])}, files {len(rec['files'])}"))

    print("== 2. member-order check (data members in file order against the binary's declared names)")
    ok = tot = numbered = exceptions = 0
    for dn, binnames in BIN.items():
        rec = found.get(dn)
        if rec is None: print(f"  {dn}: not in the data"); continue
        (parent, mem), _n = rec["variants"].most_common(1)[0]
        tot += 1
        same_count = len(mem) == len(binnames)
        names_ok = all(m == b for (m, _t), b in zip(mem, binnames) if not m.isdigit())
        types_ok = True
        for (m, t), b in zip(mem, binnames):
            p = prefix(b); fit = FITS.get(p)
            if PREFIX_EXCEPTIONS.get((dn, b)) == t:
                exceptions += 1; continue
            if fit and t not in fit:
                types_ok = False; print(f"    type mismatch {dn}.{m} {t} vs {b}")
        good = same_count and names_ok and types_ok
        ok += good; numbered += sum(1 for m, _t in mem if m.isdigit())
        print(f"  {dn:42s} members data {len(mem):3d} binary {len(binnames):3d}  count-equal {same_count}  named-at-position {names_ok}  types-fit {types_ok}"
              f"  numbered-in-data {sum(1 for m, _t in mem if m.isdigit())}")
    print(f"agree on count, position of named members and types: {ok} of {tot} classes; data members named only by the binary: {numbered}; "
          f"members whose binary name has a prefix that disagrees with the data type: {exceptions}")
    print("binary members checked:", sum(len(v) for v in BIN.values()))

    files = merged_files(install, ".rsc")
    print("== 3. HudParams.rsc")
    d, r, objs = decode(files, HUDPARAMS)
    rfil = {i: p for i, _fl, p in r["rfil"]}
    ext = {o: rfil[fr] for o, fr, _fl, _t in r["exob"]}
    db = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CHudElementsParamsDatabase")
    entries = [objs[x.oid] for x in db.get("CHudElementsParamsDatabase.1")]
    common = db.get("CHudElementsParamsDatabase.2")
    print("common simple-element parameters (shake frequency x, y, damping, bar stretch y):", [round(v, 4) for _k, v in common])
    print(f"entries {len(entries)}, distinct classes {len({e.type for e in entries})}")
    parent_of = {t.name: (r["types"][t.parent].name if t.parent in r["types"] else None) for t in r["types"].values() if t.members is not None}
    bases = collections.Counter(parent_of[e.type] for e in entries)
    print("entry base classes:", dict(bases))
    print("order of the entries:", " ".join(e.type for e in entries))
    render = collections.Counter(str(e.get("SHudElementParams.hep_rheRender")) for e in entries)
    oncam = [e.type for e in entries if e.get("SHudElementParams.hep_bRenderOnCamera")]
    print("render mode values:", dict(render), "| render-on-camera entries:", len(oncam), oncam)
    pos = {e.type: (e.get("SHudElementParams.1")[0][1], e.get("SHudElementParams.1")[1][1]) for e in entries}
    print("positions inside 0..640 x 0..480:", all(0 <= x <= 640 and 0 <= y <= 480 for x, y in pos.values()),
          "| distinct positions:", len(set(pos.values())), "| at (0,0):", sum(1 for p in pos.values() if p == (0.0, 0.0)))
    simple = [e for e in entries if parent_of[e.type] == "SSimpleHudElementParams"]
    print(f"simple-element entries {len(simple)}: bRelToPrevious set {sum(1 for e in simple if e.get('SSimpleHudElementParams.5'))}, "
          f"text cfg {sum(1 for e in simple if e.get('SSimpleHudElementParams.4').oid != NULL)}, icon cfg {sum(1 for e in simple if e.get('SSimpleHudElementParams.2').oid != NULL)}, "
          f"bar cfg {sum(1 for e in simple if e.get('SSimpleHudElementParams.3').oid != NULL)}, bar params ref {sum(1 for e in simple if e.get('SSimpleHudElementParams.6').oid != NULL)}")
    barrefs = {e.get("SSimpleHudElementParams.6").oid for e in simple if e.get("SSimpleHudElementParams.6").oid != NULL}
    print("distinct bar-params objects:", len(barrefs), "->", sorted({ext.get(o) for o in barrefs}))
    stacks = collections.defaultdict(list)
    for e in simple:
        if e.get("SSimpleHudElementParams.5"): stacks[pos[e.type]].append(e.type)
    print("stacks (entries with bRelToPrevious, by position):")
    for p, ts in sorted(stacks.items()): print(f"  {p}: {' '.join(ts)}")
    cfg = collections.Counter(objs[o].type for o in objs if isinstance(objs[o], Obj) and objs[o].type in ("SHudNumberConfig", "SHudIconConfig", "SHudBarConfig"))
    print("configuration records:", dict(cfg))
    aligns = collections.Counter(str(objs[e.get("SSimpleHudElementParams.4").oid].get("SHudNumberConfig.3")) for e in simple if e.get("SSimpleHudElementParams.4").oid != NULL)
    print("text alignments (0 left, 1 centre, 2 right):", dict(aligns))
    ext_by_dir = collections.Counter(os.path.dirname(p).rsplit("/", 1)[-1] + "/" + os.path.splitext(p)[1] for p in rfil.values())
    print("files referenced:", dict(ext_by_dir))
    print("render-order check: entries per class in the file", len(entries))
    netr = next(e for e in entries if e.type == "CNetricsaHudElementParams")
    kinds = collections.Counter(type(v).__name__ if not isinstance(v, (int, float)) else ("float" if isinstance(v, float) else "int") for k, v in netr
                                if not k.startswith("SHudElementParams"))
    print("CNetricsaHudElementParams members by value kind:", dict(kinds), "members:", sum(1 for k, _ in netr if k.startswith("CNetricsaHudElementParams.")))

    print("== 4. SimpleHudBarParams.rsc")
    d, r, objs = decode(files, BARPARAMS)
    bp = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CSimpleHudBarParams")
    rf2 = {i: p for i, _fl, p in r["rfil"]}; ex2 = {o: rf2[fr] for o, fr, _fl, _t in r["exob"]}
    print("textures in member order:", [os.path.basename(ex2[v.oid]) for k, v in bp if k.startswith("CSimpleHudBarParams.") and isinstance(v, Ref)])
    fi = bp.get("CSimpleHudBarParams.12")
    print("font instance: font", os.path.basename(ex2[fi.get("CFontInstance.1").oid]), "| scale", round(fi.get("CFontInstance.6"), 4))

    print("== 5. icon files")
    allpaths = set()
    for f in os.listdir(install):
        if f.lower().endswith(".gro"):
            with zipfile.ZipFile(os.path.join(install, f)) as z: allpaths |= {i.filename.lower() for i in z.infolist() if not i.is_dir()}
    for label, caps, d_ in [("AmmoIcons", AMMO, "content/serioussam2/textures/interface/ammoicons/"),
                            ("WeaponIcons", WEAPONS, "content/serioussam2/textures/interface/weaponicons/")]:
        have = sorted(p[len(d_):] for p in allpaths if p.startswith(d_))
        want = [c.replace(" ", "_").lower() + ".tex" for c in caps]
        print(f"  {label}: {len(have)} files; enumeration values {len(caps)}; values with a file {sum(1 for w in want if w in have)}; "
              f"files no value names: {[h for h in have if h not in want]}; values without file: {[c for c, w in zip(caps, want) if w not in have]}")


if __name__ == "__main__":
    main()
