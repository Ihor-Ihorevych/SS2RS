"""Evidence for docs/re/item-classes.md (T01-108): what the data says about the item and pickup classes of Sam2Game.dll.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries).

It prints
  1. for each of the 113 exported classes of the family (names quoted from the export table of Sam2Game.dll): whether it is in the
     data (own name, or the `S` twin of a `C` name), its base class, own member count, number of files and file extensions;
  2. the member-order check: for every data class whose member table the binary also declares, the data's members in file order
     against the binary's declared names (counts equal, every member the data names at the position of the same name, every data
     type fitting the Hungarian prefix of the binary's name);
  3. the 51 item parameter files (`CBaseItemParams`, one number each) and the `.ctl` tool files that carry item records;
  4. the census of the worlds: placed item entities and property records per class, the values of the class members
     (amounts, ammunition index, weapon index and pickup quantity, power-up index, money), the model path, the flags, the spawner and
     parameter-file links, and the contents of the ammunition containers (`CAmmoItemContainer`, `CAmmoItem`).

Usage: python -I tools/re/item_classes_verify.py [DATA_DIR] [--worlds N]
       (DATA_DIR defaults to $SS2_DATA_DIR; --worlds N = only the N smallest worlds; the full run is about 15 minutes)
"""
import collections, os, sys, zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Blob, Enum, Obj, Ref, TreeDec, _natural, merged_files, parse, u32  # noqa: E402
from ep_schema import load as load_eps, root_of  # noqa: E402

NULL = 0xFFFFFFFF

# The 113 classes exported by Sam2Game.dll that make up this family (T01-026 split, part T01-108).
FAMILY = [
    "CAmmoItemEntity", "CAmmoItemProperties", "CAmmoPackItemEntity", "CAmmoPackItemProperties", "CArmorItemEntity",
    "CArmorItemLEntity", "CArmorItemLProperties", "CArmorItemMEntity", "CArmorItemMProperties", "CArmorItemProperties",
    "CArmorItemSEntity", "CArmorItemSProperties", "CArmorItemXLEntity", "CArmorItemXLProperties", "CArmorItemXSEntity",
    "CArmorItemXSProperties", "CArmorItemXXLEntity", "CArmorItemXXLProperties", "CAutoShotgunItemEntity",
    "CAutoShotgunItemProperties", "CBaseItemEntity", "CBaseItemParams", "CBaseItemProperties", "CBeamGunItemEntity",
    "CBeamGunItemProperties", "CBladesItemEntity", "CBladesItemProperties", "CCircularSawItemEntity",
    "CCircularSawItemProperties", "CColtItemEntity", "CColtItemProperties", "CDoubleShotgunItemEntity",
    "CDoubleShotgunItemProperties", "CFlamerItemEntity", "CFlamerItemProperties", "CGenericItemEntity", "CGenericItemProperties",
    "CGrenadeLauncherItemEntity", "CGrenadeLauncherItemProperties", "CGrenadesItemEntity", "CGrenadesItemProperties",
    "CHandGrenadesItemEntity", "CHandGrenadesItemProperties", "CHealthItemEntity", "CHealthItemLEntity", "CHealthItemLProperties",
    "CHealthItemMEntity", "CHealthItemMProperties", "CHealthItemProperties", "CHealthItemSEntity", "CHealthItemSProperties",
    "CHealthItemXLEntity", "CHealthItemXLProperties", "CHealthItemXSEntity", "CHealthItemXSProperties",
    "CInvisibilityPowerUpItemEntity", "CInvisibilityPowerUpItemProperties", "CInvulnerabilityPowerUpItemEntity",
    "CInvulnerabilityPowerUpItemProperties", "CKlodovikGunItemEntity", "CKlodovikGunItemProperties", "CKlodovikItemEntity",
    "CKlodovikItemProperties", "CLifeItemEntity", "CLifeItemProperties", "CMiniGunItemEntity", "CMiniGunItemProperties",
    "CNapalmItemEntity", "CNapalmItemProperties", "CPlasmaItemEntity", "CPlasmaItemProperties", "CPlasmaRifleItemEntity",
    "CPlasmaRifleItemProperties", "CPowerUpItemEntity", "CPowerUpItemProperties", "CRepairItemEntity", "CRepairItemProperties",
    "CRocketLauncherItemEntity", "CRocketLauncherItemProperties", "CRocketsItemEntity", "CRocketsItemProperties",
    "CSeriousBombItemEntity", "CSeriousBombItemProperties", "CSeriousDamageItemEntity", "CSeriousDamageItemProperties",
    "CSeriousJumpItemEntity", "CSeriousJumpItemProperties", "CSeriousScoreItemEntity", "CSeriousScoreItemProperties",
    "CSerious_BombItemEntity", "CSerious_BombItemProperties", "CShellsItemEntity", "CShellsItemProperties",
    "CSniperBulletsItemEntity", "CSniperBulletsItemProperties", "CSniperItemEntity", "CSniperItemProperties",
    "CSpeedPowerUpItemEntity", "CSpeedPowerUpItemProperties", "CStrengthItemEntity", "CStrengthItemProperties",
    "CTreasureItemEntity", "CTreasureItemLEntity", "CTreasureItemLProperties", "CTreasureItemMEntity", "CTreasureItemMProperties",
    "CTreasureItemProperties", "CTreasureItemSEntity", "CTreasureItemSProperties", "CUziItemEntity", "CUziItemProperties",
    "CZapGunItemEntity", "CZapGunItemProperties"]

# Classes of the same hierarchy that belong to other parts of the inventory (T01-106 weapons, T01-107 projectiles); they are read here
# only as bases of the family.
ADJACENT = ["CWeaponItemEntity", "CWeaponItemProperties", "CCannonItemEntity", "CCannonItemProperties", "CCannonBallsItemEntity",
            "CCannonBallsItemProperties", "CComboWeaponsItemEntity", "CComboWeaponsItemProperties", "CBulletsItemEntity",
            "CBulletsItemProperties", "CAmmoItemContainer", "CAmmoItem"]

# Member names that the binary's member tables declare, in declaration order (names only; the binary's own identifiers).
BIN = {
    "SBaseItemEntity": ["en_bOnlyFirstPlayerPicks", "en_hre", "en_hfxAlive", "en_hmech", "en_hbo", "en_henSpawner",
                        "en_bMultiSpawnerTemplate", "en_henPicker", "en_bDynamicallyCreated", "en_ulVisible", "en_qvOriginalPlacement",
                        "en_henSpawnerWhoSpawnedMe", "en_bNoGravity", "en_vNetDesiredVelAbs", "en_qvNetPlacement", "en_pmcBody",
                        "en_strBodyPath", "en_bPickedAsTool", "en_pbipItemParams"],
    "SBaseItemProperties": ["ep_hspSpawner", "ep_bMultiSpawnerTemplate", "ep_bOnlyFirstPlayerPicks", "ep_bNoGravity"],
    "SAmmoItemEntity": ["ai_aiAmmoIndex"],
    "SAmmoPackItemEntity": ["en_hcaiPackAmmo"],
    "SAmmoPackItemProperties": ["ep_hcaiPackAmmo"],
    "SArmorItemEntity": ["en_slShieldAmmount", "en_bFillsAboveHalf"],
    "SHealthItemEntity": ["en_slHealAmmount", "en_bFillsAboveHalf"],
    "STreasureItemEntity": ["en_slMoney"],
    "CPowerUpItemEntity": ["en_puiPowerUp", "en_fWastedTime"],
    "CSeriousJumpItemEntity": ["en_fJumpMultiplier", "en_fAirControlPeriod", "en_fAirControlFactor", "en_fAirControlVelocityMultiplier"],
    "CSeriousJumpItemProperties": ["ep_fJumpMultiplier", "ep_fAirControlPeriod", "ep_fAirControlFactor", "ep_fAirControlVelocityMultiplier"],
    "CRepairItemEntity": ["en_slRepairAmount"],
    "CRepairItemProperties": ["ep_slRepairAmount"],
    "CGenericItemProperties": ["ep_strBodyPath"],
    "CBaseItemParams": ["bip_fRespawnTime"],
    "SWeaponItemEntity": ["en_wiWeaponIndex", "en_pbwpWeaponParams", "en_pbwpWeaponRenderParams", "en_slAmmoQuantity"],
}

# Hungarian prefix of a binary member name -> acceptable data type names.
FITS = {"b": ("BOOL",), "hre": (), "hfx": (), "hmech": (), "hbo": (), "hen": (), "hsp": (), "ul": ("ULONG",), "qv": ("QuatVect",),
        "v": ("Vector3f",), "pmc": ("Ptr<CModelConfiguration>",), "str": ("CString",), "pbip": ("Ptr<CBaseItemParams>",),
        "sl": ("SLONG",), "f": ("FLOAT",), "ai": ("AmmoIndex",), "pui": ("PowerUpIndex",), "wi": ("WeaponIndex",), "hcai": (),
        "pbwp": ()}

AMMO = {0: "AI_BLADES", 1: "AI_BULLETS", 2: "AI_CANNON_BALLS", 3: "AI_GRENADES", 4: "AI_HAND_GRENADES", 5: "AI_KLODOVIK", 6: "AI_PLASMA",
        7: "AI_ROCKETS", 8: "AI_SERIOUS_BOMB", 9: "AI_SHELLS", 10: "AI_SNIPER_BULLETS", 11: "AI_NAPALM", NULL: "AI_NONE"}
WEAPON = {0: "saw", 1: "zap", 2: "autoshotgun", 3: "doubleshotgun", 4: "uzi", 5: "minigun", 6: "rocket", 7: "grenade", 8: "plasma",
          9: "klodovik", 10: "cannon", 11: "bomb", 12: "colt", 13: "sniper", 14: "combo", 15: "beam", 16: "flamer"}
POWERUP = {0: "STRENGTH", 1: "SPEED", 2: "JUMP", 3: "DAMAGE", 4: "SCORE", 5: "INVULNERABILITY", 6: "INVISIBILITY", NULL: "NONE"}


def prefix(name):
    """The Hungarian prefix of `<class>_<prefix><Name>` (the lower-case letters after the underscore, then up to the first capital)."""
    tail = name.split("_", 1)[1]
    i = 0
    while i < len(tail) and tail[i].islower(): i += 1
    return tail[:i]


def twin(name):
    return [name] + (["S" + name[1:]] if name[0] == "C" else [])


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
    out = {}; nfiles = 0; ctl = {}
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
                    if ext == ".ctl" and "Item" in t.name and t.name.endswith("Properties"): ctl.setdefault(name, []).append(t.name)
    return out, nfiles, ctl


def lineage(parent, name):
    out = []
    while name is not None and name not in out: out.append(name); name = parent.get(name)
    return out


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    limit = int(sys.argv[sys.argv.index("--worlds") + 1]) if "--worlds" in sys.argv else None
    if limit is not None: args = [a for a in args if a != str(limit)]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    wanted = {n for c in FAMILY + ADJACENT for n in twin(c)} | set(BIN) | {"CPowerUpItemEntity", "CPowerUpItemProperties"}
    cat, nfiles, ctl = collect(install, wanted)
    print(f"type tables of {nfiles} format-8 files")

    print(f"\n1. the {len(FAMILY)} exported classes of the family and the data")
    indata = 0; rows = []
    for c in FAMILY:
        found = [n for n in twin(c) if n in cat]
        if not found: rows.append((c, None)); continue
        indata += 1; n = found[0]; rec = cat[n]
        (parent, mem), _ = rec["variants"].most_common(1)[0]
        exts = ",".join(f"{e.lstrip('.')}:{k}" for e, k in rec["exts"].most_common(3))
        rows.append((c, f"saved as {n:36s} base {str(parent):22s} {len(mem):2d} members, {rec['files']:3d} files ({exts}), {len(rec['variants'])} variant(s)"))
    for c, s in rows: print(f"   {c:40s} {s or 'not in the data'}")
    saved_c = [c for c, s in rows if s and s.split()[2] == c]
    print(f"   in the data: {indata}; binary only: {len(FAMILY) - indata}; saved under the C name of the export: {len(saved_c)} {saved_c}; as the S twin: {indata - len(saved_c)}")
    ent = [c for c in FAMILY if c.endswith("Entity")]; prop = [c for c in FAMILY if c.endswith("Properties")]
    print(f"   entities {len(ent)} ({sum(1 for c in ent if any(n in cat for n in twin(c)))} in the data), properties {len(prop)} "
          f"({sum(1 for c in prop if any(n in cat for n in twin(c)))} in the data), other {len(FAMILY) - len(ent) - len(prop)}")
    print("   adjacent classes (other parts):")
    for c in ADJACENT:
        found = [n for n in twin(c) if n in cat]
        if found:
            n = found[0]; (parent, mem), _ = cat[n]["variants"].most_common(1)[0]
            print(f"      {c:34s} saved as {n:30s} base {str(parent):22s} {len(mem)} members, {cat[n]['files']} files")
        else: print(f"      {c:34s} not in the data")

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
            print(f"   {key:28s} base {str(parent):22s} data {len(dn):2d} / binary {len(names):2d} members ({nv} file(s)): "
                  f"same count {same_count}; named members at their binary position {named_ok}; type misfits {bad or 'none'}")
            if len(mem) and same_count:
                print("      " + ", ".join(f"{m}={names[i]}" for i, (m, _) in enumerate(mem) if m != names[i])[:600])

    print("\n3. item parameter files and tool files")
    eps = load_eps(install, (".ep",))
    resp = collections.defaultdict(list); bytype = collections.Counter()
    for name, (d, r, objs, edob) in sorted(eps.items()):
        rid = root_of(objs)[0]; o = objs[rid]; bytype[o.type] += 1
        if o.type == "CBaseItemParams": resp[name.split("databases/entityparams/items/")[-1]] = [v for k, v in o if k == "CBaseItemParams.1"]
    print(f"   {sum(len(v) for v in resp.values())} files with CBaseItemParams (of {len(eps)} .ep files); the number is `bip_fRespawnTime` (the binary's name)")
    for k, v in sorted(resp.items()): print(f"      {k:44s} {fmt(v[0])}")
    print(f"   .ctl files with an item property record: {len(ctl)}")
    for k, v in sorted(ctl.items()): print(f"      {k.split('databases/')[-1]:50s} {', '.join(v)}")

    print("\n4. census of the worlds")
    files = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))
    ents = collections.defaultdict(lambda: {"n": 0, "worlds": set(), "vals": collections.defaultdict(collections.Counter)})
    props = collections.defaultdict(lambda: {"n": 0, "worlds": set(), "vals": collections.defaultdict(collections.Counter)})
    pairs = collections.Counter(); contents = collections.Counter(); totals = collections.defaultdict(collections.Counter)
    owners = collections.Counter(); eplink = collections.defaultdict(collections.Counter); nworlds = 0; ncont = 0
    spawners = collections.Counter(); generic = collections.defaultdict(collections.Counter); cross = collections.Counter(); perworld = collections.Counter()
    for n, (wname, d) in enumerate(files):
        if limit and n >= limit: break
        r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects()
        try: extra, _ = dec.extra_objects(edob)
        except Exception: extra = {}
        T = r["types"]; parent = {t.name: (T[t.parent].name if t.parent in T else None) for t in T.values() if t.members is not None}
        rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}
        allo = {**objs, **extra}; nworlds += 1
        users = collections.defaultdict(list)
        for oid, o in allo.items():
            if isinstance(o, Obj):
                for k, v in o:
                    if isinstance(v, Ref) and v.oid != NULL: users[v.oid].append(o.type)
        for oid, o in allo.items():
            if not isinstance(o, Obj): continue
            lin = lineage(parent, o.type)
            if o.type == "CAmmoItemContainer":
                ncont += 1
                owners[tuple(sorted(set(users.get(oid, []))))] += 1
                items = []
                for k, v in o:
                    if isinstance(v, list):
                        for e in v:
                            if isinstance(e, Ref) and e.oid in allo and isinstance(allo[e.oid], Obj):
                                ai = allo[e.oid].get("CAmmoItem.ai_aiAmmoIndex"); q = allo[e.oid].get("CAmmoItem.ai_slQuantity")
                                items.append((AMMO.get(ai.v if isinstance(ai, Enum) else ai, "?"), q))
                kind = "ammo pack" if any(u.startswith("SAmmoPack") for u in users.get(oid, [])) else "chapter info"
                contents[(kind, tuple(sorted(items)))] += 1
                for a, q in items: totals[(kind, a)][q] += 1
                continue
            if "SBaseItemEntity" in lin:
                rec = ents[o.type]; rec["n"] += 1; rec["worlds"].add(wname); perworld[wname] += 1
                for k, v in o:
                    cls, _, mem = k.partition(".")
                    if cls in ("CEntity", "CBaseEntity", "SBaseEntity"): continue
                    if cls == "SBaseItemEntity":
                        if mem == "503":
                            if isinstance(v, Ref) and v.oid in ex: eplink[o.type][ex[v.oid].split("/")[-1].lower()] += 1
                            rec["vals"]["params link"]["null" if isinstance(v, Ref) and v.oid == NULL else "set"] += 1
                        elif mem == "en_strBodyPath": rec["vals"]["model"][v.split("/Models/")[-1]] += 1
                        elif mem == "en_bOnlyFirstPlayerPicks": rec["vals"]["onlyFirst"][fmt(v)] += 1
                        elif mem in ("12", "15", "16", "30", "31", "en_hfxAlive", "en_henPicker", "20", "501", "502", "en_pmcBody"):
                            generic[mem]["null" if isinstance(v, Ref) and v.oid == NULL else "set"] += 1
                        else: generic[mem][fmt(v)] += 1
                        continue
                    rec["vals"][k.split(".", 1)[1]][fmt(v)] += 1
                sp = o.get("SBaseItemEntity.30")
                if isinstance(sp, Ref) and sp.oid != NULL and sp.oid in allo and isinstance(allo[sp.oid], Obj): spawners[allo[sp.oid].type] += 1
                has = lambda m: isinstance(o.get("SBaseItemEntity." + m), Ref) and o.get("SBaseItemEntity." + m).oid != NULL
                cross[("spawner set" if has("30") else "spawner null", "mechanism set" if has("15") else "mechanism null")] += 1
            elif "SBaseItemProperties" in lin:
                rec = props[o.type]; rec["n"] += 1; rec["worlds"].add(wname)
                for k, v in o:
                    cls, _, mem = k.partition(".")
                    if cls == "SBaseItemProperties": rec["vals"][mem][fmt(v) if not isinstance(v, Ref) else ("null" if v.oid == NULL else "set")] += 1
                    elif cls.endswith("Properties") and cls not in ("CEntityProperties", "CBaseProperties", "SBaseProperties"): rec["vals"][k][fmt(v)] += 1
                eo = o.get("CEntityProperties.2")
                if isinstance(eo, Ref):
                    rec["vals"]["owner entity"]["null" if eo.oid == NULL else "set"] += 1
                    if eo.oid != NULL and eo.oid in objs and isinstance(objs[eo.oid], Obj): pairs[(objs[eo.oid].type, o.type)] += 1
                    elif eo.oid != NULL: pairs[(None, o.type)] += 1
        print(f"[{n + 1}/{len(files)}] {wname.rsplit('/', 1)[-1]}", file=sys.stderr, flush=True)

    print(f"   worlds read: {nworlds}; item entity classes found: {len(ents)}; item property record classes found: {len(props)}")
    print(f"   placed item entities: {sum(v['n'] for v in ents.values())} in {len(perworld)} worlds; property records: {sum(v['n'] for v in props.values())}")
    same = sum(n for (a, b), n in pairs.items() if a is not None and a.replace("Entity", "Properties") == b)
    other = sum(n for (a, b), n in pairs.items() if a is not None and a.replace("Entity", "Properties") != b)
    owned = sum(v["vals"]["owner entity"]["null"] for v in props.values())
    print(f"   property records with an entity (CEntityProperties.2) of the matching class: {same}; of another class: {other}; "
          f"records without an entity (owned by a spawner, `CEntityProperties.2` null): {owned}")
    print(f"   entities {sum(v['n'] for v in ents.values())} + owned records {owned} = {sum(v['n'] for v in ents.values()) + owned} records in all: {sum(v['n'] for v in props.values())}")
    print("   spawner link x mechanism handle (entities):", dict(cross))
    for m, cnt in sorted(generic.items()): print(f"   SBaseItemEntity.{m:28s} " + ", ".join(f"{a} x{b}" for a, b in cnt.most_common(6)))
    print("\n   class                             entities (worlds) | records (worlds)")
    for c in sorted(ents):
        e = ents[c]; pp = props.get(c.replace("Entity", "Properties"))
        print(f"   {c:32s} {e['n']:5d} ({len(e['worlds']):2d}) | {pp['n'] if pp else 0:5d} ({len(pp['worlds']) if pp else 0:2d})")
        for k, cnt in sorted(e["vals"].items()):
            print(f"        {k:40s} " + ", ".join(f"{a} x{b}" for a, b in cnt.most_common(30 if k == "model" else 8)) + (" ..." if len(cnt) > (30 if k == "model" else 8) else ""))
        if eplink.get(c): print("        .ep link targets                         " + ", ".join(f"{a} x{b}" for a, b in eplink[c].most_common(4)))
        if pp:
            for k, cnt in sorted(pp["vals"].items()):
                print(f"        [record] {k:31s} " + ", ".join(f"{a} x{b}" for a, b in cnt.most_common(6)))
    print("\n   property record classes without an entity class of the same stem:", [c for c in props if c.replace("Properties", "Entity") not in ents])
    print(f"\n   ammunition containers: {ncont}; referenced by (set of referring classes): {dict(owners)}")
    print("   quantity per ammunition kind over all containers (kind: quantity x count):")
    for (kind, a), cnt in sorted(totals.items()): print(f"      {kind:12s} {a:20s} " + ", ".join(f"{q} x{n}" for q, n in sorted(cnt.items())))
    print(f"   distinct container contents: {len(contents)}; most common 10:")
    for (kind, k), n in contents.most_common(10): print(f"      x{n} ({kind}): " + ", ".join(f"{a} {q}" for a, q in k))
    byk = collections.Counter(); sizes = collections.Counter()
    for (kind, k), n in contents.items(): byk[kind] += n; sizes[(kind, len(k))] += n
    print("   containers by owner:", dict(byk), "; items per container (owner, count): ", dict(sorted(sizes.items())))
    print("   spawner classes behind the spawnee templates (SBaseItemEntity.30):", dict(spawners))


if __name__ == "__main__":
    main()
