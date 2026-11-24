"""Evidence for docs/re/players-session-classes.md (T01-102): what the data says about the player, session and inventory classes.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries).

It prints
  1. for the 10 classes of the family that ARE in the data: definition (base class, own members), number of files, file extensions, number of
     definition variants, and under which data name (the `C` name that the code exports, or its `S` twin) the class is saved;
  2. that none of the 42 other classes of the family (names taken from the exported names of Sam2Game.dll, quoted as-is) appears in any
     type table, neither under its own name nor as an `S` twin;
  3. the files that carry the family's resource classes (`.ep` and `.rsc` only) and the member-name comparison of `SPlayerPuppetParams`
     with the 24 member names that the binary's member table declares.

Usage: python -I tools/re/players_session_classes_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR; about 2 minutes)
"""
import collections, os, sys, zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import _natural, parse, u32  # noqa: E402
import class_catalog  # noqa: E402

# Names exported by Sam2Game.dll (the family of T01-102), split by whether the data has the class.
IN_DATA = ["CCameras", "CDifficultyParams", "CGameDifficulties", "CGlobalPlayerParams", "CKnockOutParams",
           "CPlayerDeathHudElementParams", "CPlayerPuppetCrosshairs", "CPlayerPuppetParams", "CPowerUpDurations",
           "CVotingInfoHudElementParams"]
BINARY_ONLY = [
    "CAmmoInventoryItem", "CAutoShotgunInventoryItem", "CBaseInventoryItemObsolete", "CBeamGunInventoryItem", "CBladesInventoryItem",
    "CBulletsInventoryItem", "CCannonBallsInventoryItem", "CCannonInventoryItem", "CCircularSawInventoryItem",
    "CDoubleShotgunInventoryItem", "CGameInfoEntity", "CGameInfoProperties", "CGameStats", "CGrenadeLauncherInventoryItem",
    "CGrenadesInventoryItem", "CHandGrenadesInventoryItem", "CInventoryBase", "CKlodovikGunInventoryItem", "CKlodovikInventoryItem",
    "CMiniGunInventoryItem", "CPlasmaInventoryItem", "CPlasmaRifleInventoryItem", "CPlayerBrainEntity", "CPlayerBrainProperties",
    "CPlayerDeathHudElement", "CPlayerInventory", "CPlayerPuppetEntity", "CPlayerPuppetProperties", "CPlayerSave",
    "CRocketLauncherInventoryItem", "CRocketsInventoryItem", "CSeriousBombInventoryItem", "CSerious_BombInventoryItem",
    "CShellsInventoryItem", "CStoredChapterStatus", "CStoredLevelStatus", "CUziInventoryItem", "CVotingInfoEntity",
    "CVotingInfoHudElement", "CVotingInfoProperties", "CWeaponInventoryItem", "CZapGunInventoryItem"]
# Member names of the player puppet parameter class as the binary's member table declares them (names only).
PPP_DLL = ["ppp_pgppHandGrenade", "ppp_fMinBombThrowVelocity", "ppp_fMaxBombThrowVelocity", "ppp_fAddedPitchDeg",
           "ppp_fMinBombThrowPitchDeg", "ppp_fMaxBombThrowPitchDeg", "ppp_fHandGreandeRecoil", "ppp_fFreezeTime", "ppp_fUnfrezeTime",
           "ppp_fFreezingDamagePerSec", "ppp_fFreezingExhalePeriod", "ppp_fViewDampingK", "ppp_fViewDampingD", "ppp_fMaxMoveBanking",
           "ppp_fMaxSideStepBanking", "ppp_fViewAutoCenterDelay", "ppp_fViewAutoCenterMultiplier", "ppp_fSpeedPowerUpMultiplier",
           "ppp_pfntOverlay", "ppp_ptecNotification", "ppp_ptecChangeLeftWeapon", "ppp_pefExhalation", "ppp_pbsExhalation",
           "ppp_ppcCrosshairs"]


def twin(name):
    """The data name an exported class would have: the same, or the `S` twin of a `C` name."""
    return [name] + (["S" + name[1:]] if name[0] == "C" else [])


def resource_entries(install, exts):
    """(archive, entry) of the format-8 files with the given extensions, patches overriding base archives (as class_catalog does)."""
    merged = {}
    for f in sorted((f for f in os.listdir(install) if f.lower().endswith(".gro")), key=_natural):
        with zipfile.ZipFile(os.path.join(install, f)) as z:
            for i in z.infolist():
                if i.is_dir() or i.file_size < 64 or not i.filename.lower().endswith(exts): continue
                with z.open(i) as fh: head = fh.read(16)
                if head[:8] == b"CTSEMETA" and u32(head, 12) == 8: merged[i.filename.lower()] = (f, i.filename)
    return sorted(merged.values(), key=lambda v: v[1].lower())


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    cat, nfiles = class_catalog.classes_of(install)
    print(f"class tables of {nfiles} format-8 files, {len(cat)} classes")

    print("\n1. the family's classes that are in the data")
    for c in IN_DATA:
        found = [n for n in twin(c) if n in cat]
        if not found: print(f"   {c}: NOT FOUND"); continue
        n = found[0]; rec = cat[n]; parent, flags, members = class_catalog.best(rec)
        exts = ",".join(f"{e.lstrip('.')}:{k}" for e, k in rec["exts"].most_common(3))
        print(f"   {c:30s} saved as {n:30s} base {str(parent):22s} {len(members):3d} own members, {rec['files']:4d} files ({exts}), {len(rec['variants'])} variant(s)")

    print("\n2. the other classes of the family are in no type table")
    present = [c for c in BINARY_ONLY if any(n in cat for n in twin(c))]
    print(f"   {len(BINARY_ONLY) - len(present)} of {len(BINARY_ONLY)} are absent (own name and `S` twin); present: {present or 'none'}")
    prefixes = ("CPlayerPuppet", "CPlayerBrain", "CGameInfo", "CVotingInfo", "CPlayerInventory", "CPlayerSave", "CStored", "CGameStats")
    stray = sorted(n for n in cat if n[1:].startswith(tuple(p[1:] for p in prefixes)))
    print(f"   data classes whose name continues one of those stems (own or twin form): {stray}")

    print("\n3. resource files of the family (.ep and .rsc)")
    holders = collections.defaultdict(list); ppp_members = collections.Counter()
    targets = {n for c in IN_DATA for n in twin(c)}
    for arch, name in resource_entries(install, (".ep", ".rsc")):
        with zipfile.ZipFile(os.path.join(install, arch)) as z: r = parse(z.read(name))
        for t in r["types"].values():
            if t.members is None or t.name not in targets: continue
            holders[t.name].append(name)
            if t.name == "SPlayerPuppetParams": ppp_members[tuple(m for m, _ in t.members)] += 1
    for n in sorted(holders):
        files = holders[n]
        shown = ", ".join(files) if len(files) <= 4 else f"{files[0]} ... {files[-1]}"
        print(f"   {n:30s} in {len(files):3d} file(s): {shown}")
    print("\n   SPlayerPuppetParams member sets in the data (own members; a number-only member is shown as `.N`):")
    allnames = set()
    for names, k in ppp_members.most_common():
        allnames |= set(names)
        pos = [PPP_DLL.index(n) for n in names if not n.isdigit() and n in PPP_DLL]
        in_order = all(a < b for a, b in zip(pos, pos[1:]))
        numbered = [n for n in names if n.isdigit()]
        print(f"     {k} file(s): {len(names)} members, {len(numbered)} number-only, {len(pos)} named and declared by the binary; "
              f"named members in the binary's order: {in_order}")
        for i, n in enumerate(names):
            if not n.isdigit(): continue
            before = max((PPP_DLL.index(m) for m in names[:i] if m in PPP_DLL), default=-1)
            after = min((PPP_DLL.index(m) for m in names[i + 1:] if m in PPP_DLL), default=len(PPP_DLL))
            print(f"       .{n} sits between named neighbours; the binary declares there: {PPP_DLL[before + 1:after]}")
    print(f"   binary names never seen as a member name in any data file: {sorted(n for n in PPP_DLL if n not in allnames)}; "
          f"data names the binary does not declare: {sorted(n for n in allnames if not n.isdigit() and n not in PPP_DLL)}")


if __name__ == "__main__":
    main()
