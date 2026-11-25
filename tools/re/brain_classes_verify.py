"""Evidence for docs/re/brain-classes.md (T01-104): what the data says about the brain classes of Sam2Game.dll.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries).

It prints
  1. that none of the 61 exported brain classes (names taken from the exported names of Sam2Game.dll, quoted as-is) is in any type table,
     neither under its own name nor as an `S` twin;
  2. the data classes that carry a member whose name contains `Brain` (the handles that point at a brain, and the world info's list of
     brains without a foe), with the number of files that define each class;
  3. which of the exported puppet entity classes (the kinds that pair with the brain kinds by name) are in the data;
  4. the identifiers of the AI moments that the 82 `.cb` behavior files use, with the number of action lists per moment, next to the
     moment names that the brain code raises (names read from the binary and quoted as-is), so that the two lists can be compared.

Usage: python -I tools/re/brain_classes_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR; about 3 minutes)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ai_verify  # noqa: E402
import class_catalog  # noqa: E402

# The 61 classes of the family: 20 entity classes, 20 properties classes, 21 parameter classes (exported names).
ENTITIES = ["CAIBrainEntity", "CAirBikeBrainEntity", "CAircraftBrainEntity", "CAutoTurretBrainEntity", "CBrainEntity",
            "CBuggyBrainEntity", "CCharacterBrainEntity", "CDummyFoeBrainEntity", "CFlyingKleerBrainEntity",
            "CGenericLeggedBrainEntity", "CHovercraftBrainEntity", "CKleerBrainEntity", "CManifestedSpawnerBrainEntity",
            "CMilitaryCarBrainEntity", "CNPCBrainEntity", "CSciFiSoldierBrainEntity", "CTurretBrainEntity", "CWereBullBrainEntity",
            "CWheeledVehicleBrainEntity", "CZombieSoldierBrainEntity"]
PROPERTIES = [n.replace("Entity", "Properties") for n in ENTITIES]
PARAMS = ["CAIBrainParams", "CAirBikeBrainParams", "CAircraftBrainParams", "CAutoTurretBrainParams", "CBrainParams", "CBuggyBrainParams",
          "CFlyingKleerBrainParams", "CGenericLeggedBrainParams", "CHovercraftBrainParams", "CKleerBrainParams",
          "CManifestedSpawnerBrainParams", "CMilitaryCarBrainParams", "CNPCBomberBrainParams", "CNPCBrainParams",
          "CNPCCivilianBrainParams", "CNPCKamikazeBrainParams", "CNPCShooterBrainParams", "CSciFiSoldierBrainParams",
          "CTurretBrainParams", "CWereBullBrainParams", "CWheeledVehicleBrainParams"]
# Exported puppet entity classes (the brain kind X pairs with the puppet kind X by name).
PUPPETS = ["CAirBikePuppetEntity", "CAircraftPuppetEntity", "CAutoTurretPuppetEntity", "CBuggyPuppetEntity", "CDummyFoePuppetEntity",
           "CFlyingKleerPuppetEntity", "CGenericLeggedPuppetEntity", "CHovercraftPuppetEntity", "CKleerPuppetEntity",
           "CLeggedPuppetEntity", "CManifestedSpawnerPuppetEntity", "CMilitaryCarPuppetEntity", "CNPCPuppetEntity",
           "CPlayerPuppetEntity", "CPuppetEntity", "CRollingBallPuppetEntity", "CSciFiSoldierPuppetEntity", "CTurretPuppetEntity",
           "CWereBullPuppetEntity", "CWheeledVehiclePuppetEntity", "CZombieSoldierPuppetEntity"]
# Moment names that the brain code raises or reports (strings the binary's brain methods refer to; `Action` is the action set a new brain starts with).
BRAIN_MOMENTS = ["Action", "BeginState", "EndState", "FoeAssigned", "Damage", "AllFoesDied", "ToolPicked", "StartRiding", "StopRiding",
                 "IKilledSomeFoe", "IKilledMyFoe", "MyFoeDied", "TurnedOn", "TurnedOff", "FoeTouched", "CantPossesRide"]


def twin(name):
    """The data name an exported class would have: the same, or the `S` twin of a `C` name."""
    return [name] + (["S" + name[1:]] if name[0] == "C" else [])


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    cat, nfiles = class_catalog.classes_of(install)
    print(f"class tables of {nfiles} format-8 files, {len(cat)} classes")

    family = ENTITIES + PROPERTIES + PARAMS
    print(f"\n1. the {len(family)} classes of the family ({len(ENTITIES)} entity, {len(PROPERTIES)} properties, {len(PARAMS)} parameter classes; "
          "CCharacterBrainParams and CDummyFoeBrainParams do not exist)")
    present = [c for c in family if any(n in cat for n in twin(c))]
    print(f"   {len(family) - len(present)} of {len(family)} are absent from every type table (own name and `S` twin); present: {present or 'none'}")
    stems = tuple(c[1:] for c in family)
    stray = sorted(n for n in cat if n[1:] in stems)
    print(f"   data classes whose name equals a family name with another first letter: {stray or 'none'}")

    print("\n2. data classes with a member whose name contains 'Brain'")
    for name in sorted(cat):
        parent, flags, members = class_catalog.best(cat[name])
        hits = [m for m in members if "brain" in m.lower()]
        if hits: print(f"   {name:34s} in {cat[name]['files']:4d} file(s): {', '.join(hits)}")

    print("\n3. exported puppet entity classes (own name or `S` twin) that are in the data")
    for c in PUPPETS:
        found = [n for n in twin(c) if n in cat]
        print(f"   {c:34s} {('saved as ' + found[0] + ' in ' + str(cat[found[0]]['files']) + ' file(s)') if found else 'not in the data'}")

    print("\n4. AI moments of the .cb files")
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter)
    ai_verify.behaviors(install, st, dist, False)
    moments = dist["moments with a handler list"]
    print(f"   {st['cb files']} .cb files; moment identifiers with a handler list (action lists): {dict(moments.most_common())}")
    in_cb = sorted(moments); both = [m for m in BRAIN_MOMENTS if m in moments]
    print(f"   raised by the brain code and used by some .cb file: {both}")
    print(f"   raised by the brain code and used by no .cb file: {[m for m in BRAIN_MOMENTS if m not in moments]}")
    print(f"   used by .cb files and not in the list of the brain code's own moment names: {[m for m in in_cb if m not in BRAIN_MOMENTS]}")


if __name__ == "__main__":
    main()
