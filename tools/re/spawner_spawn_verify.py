"""Evidence for docs/re/spawner-state-machine.md and docs/re/spawner-spawn-side.md (T01-165): what the data says about the spawn side of the spawner.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries).

It prints
  1. the 11 external spawn effect configurations (`*.sfc`, member by member) and the branch of the spawn effect that each one takes;
  2. a census of the `SSpawnerProperties` records of the worlds: count / group / delay combinations, the group-spawner flag against the group size,
     the spawn effect configurations (shared or inline) by which parts they set, spawn source, launcher and formation links, spawnee count limit type;
  3. the launchers (`SRelativeRandomConeLauncher`, `SMarkersLauncher`) with their members;
  4. the formations (`CLineFormation`, `CRectFormation`, `CVFormation`, `CPieSliceFormation`, `CMarkersFormation`): kind number, the five flags of
     the base class, members, the weighted marker lists (items, weights, the two flags);
  5. the multi-spawners: template providers, template counts, weights, the randomise flag, and the group of the spawner they own.

Usage: python -I tools/re/spawner_spawn_verify.py [DATA_DIR] [--worlds N]
       (DATA_DIR defaults to $SS2_DATA_DIR; --worlds N = only the N smallest worlds; the full run is about 10 minutes)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, merged_files, parse  # noqa: E402

NULL = 0xFFFFFFFF
SFC_CLASS = "CSpawnEffectConfiguration"
FORMATIONS = ("CLineFormation", "CRectFormation", "CVFormation", "CPieSliceFormation", "CMarkersFormation")
LAUNCHERS = ("SRelativeRandomConeLauncher", "SMarkersLauncher")


def member(o, suffix):
    """Value of the member of Obj `o` with the qualified name `suffix` (`Class.member`), or whose member name is `suffix` when no class is given."""
    for k, v in o:
        if k == suffix or ("." not in suffix and k.rsplit(".", 1)[-1] == suffix):
            return v
    return None


def fmt(v):
    if isinstance(v, float):
        return f"{v:.4g}"
    return str(v)


def hist(c, top=8):
    return "; ".join(f"{k} x{n}" for k, n in c.most_common(top)) + (" ..." if len(c) > top else "")


def sfc_branch(o):
    """Which way DoSpawnEffect goes for this configuration (docs/re/spawner-spawn-side.md section 4)."""
    eff = member(o, "sfxc_pefEffect"); snd = member(o, "sfxc_pbsSound")
    has_eff = isinstance(eff, Ref) and eff.oid != NULL
    has_snd = isinstance(snd, Ref) and snd.oid != NULL
    proj = member(o, "sfxc_pgppProjectileParams")
    has_proj = isinstance(proj, Ref) and proj.oid != NULL
    delay = member(o, "sfx_fSpawnDelay")
    first = "effect or sound: effect entity, then delayer" if (has_eff or has_snd) else "no effect, no sound: finish at once"
    second = "projectile carries the spawnee" if has_proj else "spawnee made directly"
    return first, second, delay


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    limit = int(sys.argv[sys.argv.index("--worlds") + 1]) if "--worlds" in sys.argv else None
    if limit is not None:
        args = [a for a in args if a != str(limit)]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder or set SS2_DATA_DIR")

    # ---- 1. external .sfc files
    print("1. external spawn effect configurations (*.sfc)")
    ext_members = None
    for name, d in sorted(merged_files(install, ".sfc").items()):
        r = parse(d); dec = TreeDec(d, r); objs, _ = dec.objects()
        rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}
        for t in r["types"].values():
            if t.name == SFC_CLASS and t.members is not None and ext_members is None:
                ext_members = [m for m, _ in t.members]
        for o in objs.values():
            if not isinstance(o, Obj) or o.type != SFC_CLASS:
                continue
            row = []
            for k, v in o:
                if isinstance(v, Ref):
                    v = "null" if v.oid == NULL else os.path.basename(ex.get(v.oid, "inline")) if v.oid in ex else "inline"
                row.append(f"{k.rsplit('.', 1)[-1].replace('sfxc_', '').replace('sfx_', '')}={fmt(v)}")
            b1, b2, delay = sfc_branch(o)
            print(f"   {os.path.basename(name):34s} " + " ".join(row))
            print(f"   {'':34s} -> {b1}; {b2}; delay {fmt(delay)} s")
    if ext_members:
        print(f"   member order of {SFC_CLASS} ({len(ext_members)}): " + ", ".join(ext_members))

    # ---- census of worlds
    files = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))
    C = collections.defaultdict(collections.Counter)
    nworlds = 0; nrec = 0
    for n, (wname, d) in enumerate(files):
        if limit and n >= limit:
            break
        r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects()
        try:
            extra, _ = dec.extra_objects(edob)
        except Exception:
            extra = {}
        allo = {**objs, **extra}; nworlds += 1
        rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}
        typeof = {oid: o.type for oid, o in allo.items() if isinstance(o, Obj)}

        def kind(v):
            if not isinstance(v, Ref) or v.oid == NULL:
                return "null"
            if v.oid in ex:
                return "ext:" + os.path.basename(ex[v.oid])
            return typeof.get(v.oid, "?")

        for oid, o in allo.items():
            if not isinstance(o, Obj):
                continue
            t = o.type
            if t == "SSpawnerProperties":
                nrec += 1
                tot = member(o, "SSpawnerProperties.2"); grp = member(o, "SSpawnerProperties.1")
                gs = member(o, "ep_bGroupSpawner")
                C["count x group"][(tot, grp)] += 1
                C["group spawner flag x (group > 1, total > group)"][(gs, grp > 1, tot > grp)] += 1
                C["initial delay"][fmt(member(o, "SSpawnerProperties.3"))] += 1
                C["single delay"][fmt(member(o, "SSpawnerProperties.4"))] += 1
                C["group delay"][fmt(member(o, "SSpawnerProperties.5"))] += 1
                C["spawnee death delay"][fmt(member(o, "SSpawnerProperties.14"))] += 1
                C["delay combination (initial>0, single>0, group>0, death>0)"][tuple(bool(member(o, "SSpawnerProperties." + k)) for k in ("3", "4", "5", "14"))] += 1
                src = kind(member(o, "SSpawnerProperties.11")); sfc = member(o, "ep_sfcSpawnEffectConfig")
                lau = kind(member(o, "SSpawnerProperties.12")); frm = kind(member(o, "SSpawnerProperties.13"))
                skind = "null" if not isinstance(sfc, Ref) or sfc.oid == NULL else ("inline" if sfc.oid in allo and oid != sfc.oid and sfc.oid not in ex else "ext:" + os.path.basename(ex.get(sfc.oid, "?")))
                C["spawn source linked x effect configuration kind"][("source" if src != "null" else "no source", skind if skind.startswith("ext") is False else "shared file")] += 1
                C["launcher x formation"][(lau, frm)] += 1
                C["auto fall down x launcher"][(member(o, "SSpawnerProperties.7"), lau)] += 1
                C["group spawner x formation"][(gs, frm)] += 1
                if isinstance(sfc, Ref) and sfc.oid != NULL and sfc.oid in allo and isinstance(allo[sfc.oid], Obj):
                    b1, b2, delay = sfc_branch(allo[sfc.oid])
                    C["inline configuration: branch"][(b1, b2)] += 1
                    C["inline configuration: spawn delay"][fmt(delay)] += 1
                    C["inline configuration: stop chance"][fmt(member(allo[sfc.oid], "sfxc_fSpawnSourceStopChance"))] += 1
            elif t in LAUNCHERS:
                for k, v in o:
                    if k.startswith(("SLauncher",)):
                        continue
                    cls, _, m = k.partition(".")
                    if isinstance(v, (int, float)) or v is None:
                        C[f"{t}.{m}"][fmt(v)] += 1
                    elif isinstance(v, Ref):
                        C[f"{t}.{m}"][kind(v)] += 1
                    elif isinstance(v, Obj):
                        lst = member(v, "CWeightedFormationMarkers.1") or []
                        C[f"{t}.{m} (weighted markers: items)"][len(lst)] += 1
                        C[f"{t}.{m} (weighted markers: flag .2)"][fmt(member(v, "CWeightedFormationMarkers.2"))] += 1
                        C[f"{t}.{m} (weighted markers: closest to player)"][fmt(member(v, "wfm_bClosestToPlayer"))] += 1
                        for it in lst:
                            C[f"{t} marker weight"][fmt(member(it, "CWeightedFormationMarker.3"))] += 1
            elif t in FORMATIONS:
                C["formation kind number (CFormation.6) by class"][(t, member(o, "CFormation.6"))] += 1
                C["formation five flags (CFormation.1..5) by class"][(t, tuple(member(o, "CFormation.%d" % i) for i in range(1, 6)))] += 1
                for k, v in o:
                    cls, _, m = k.partition(".")
                    if cls == "CFormation":
                        continue
                    if isinstance(v, (int, float)):
                        C[f"{t}.{m}"][fmt(v)] += 1
                    elif isinstance(v, Ref):
                        C[f"{t}.{m}"][kind(v)] += 1
                    elif isinstance(v, Obj):
                        lst = member(v, "CWeightedFormationMarkers.1") or []
                        C[f"{t}.{m} (weighted markers: items)"][len(lst)] += 1
                        C[f"{t} weighted markers: flag .2 x closest-to-player flag"][(fmt(member(v, "CWeightedFormationMarkers.2")), fmt(member(v, "wfm_bClosestToPlayer")))] += 1
                        tw = 0
                        for it in lst:
                            w = member(it, "CWeightedFormationMarker.3"); C[f"{t} marker weight"][fmt(w)] += 1; tw += w or 0
                            C[f"{t} marker item: marker kind x own record"][(kind(member(it, "CWeightedFormationMarker.1")), kind(member(it, "CWeightedFormationMarker.2")))] += 1
                        C[f"{t} total weight"][tw] += 1
            elif t == "SMultiSpawnerProperties":
                prov = member(o, "SMultiSpawnerProperties.2")
                C["multi-spawner randomise flag (SMultiSpawnerProperties.3)"][fmt(member(o, "SMultiSpawnerProperties.3"))] += 1
                if isinstance(prov, Ref) and prov.oid in allo and isinstance(allo[prov.oid], Obj):
                    lst = member(allo[prov.oid], "CSpawnTemplateProvider.1") or []
                    C["multi-spawner templates"][len(lst)] += 1
                    ws = []
                    for e in lst:
                        w = allo[e.oid] if isinstance(e, Ref) and e.oid in allo else None
                        if isinstance(w, Obj):
                            ws.append(member(w, "CWeightedSpawnTemplate.3"))
                            C["template weight"][fmt(member(w, "CWeightedSpawnTemplate.3"))] += 1
                    C["multi-spawner total weight"][sum(x or 0 for x in ws)] += 1
        print(f"[{n + 1}/{len(files)}] {wname.rsplit('/', 1)[-1]}", file=sys.stderr, flush=True)

    print(f"\ncensus of {nworlds} worlds ({nrec} SSpawnerProperties records)")
    sections = [
        ("2. spawner records", lambda k: not k.startswith(("SRelative", "SMarkers", "CLine", "CRect", "CV", "CPie", "CMarkers", "formation", "multi", "template"))),
        ("3. launchers", lambda k: k.startswith(("SRelative", "SMarkers"))),
        ("4. formations", lambda k: k.startswith(("CLine", "CRect", "CV", "CPie", "CMarkers", "formation"))),
        ("5. multi-spawners", lambda k: k.startswith(("multi", "template"))),
    ]
    for title, sel in sections:
        print(f"\n{title}")
        for k in sorted(C):
            if sel(k):
                print(f"   {k}: " + hist(C[k], 12))


if __name__ == "__main__":
    main()
