"""Evidence for the data task of T01-166 (`mechanism-classes-spawners.md` open question 2,
`mechanism-classes-spawner-tweaks.md` open question 1): which spawner each of the 129 owner ids
of `spawnertweaks.rsc` and the 33 of `spawnerboottweaks.rsc` selects.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's
binaries).

Method: an owner-id condition names the spawnee's own entity id (`CBaseProperties.ep_ulEntityIDVis`,
equal to `CEntity.1`; `wld-visibility.md`). For every world this walks every `SSpawnerEntity`,
resolves `.120` (`en_henOwner`, the spawnee) and records the spawnee's own id against its class
name and the world it is in. It then prints, for every owner id in the two tweak files, the level
table it is under, the tweak kind(s) and reason text, and the matching spawnee(s) found (or "not
found" when the id names no placed entity in the 46 spawner worlds — most likely an id of a level
the install does not ship, or of an entity later removed by a patch).

Usage: python -I tools/re/spawner_multiplier_verify.py [DATA_DIR]   (DATA_DIR defaults to $SS2_DATA_DIR; a few minutes)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
from ep_schema import load as load_eps  # noqa: E402

NULL = 0xFFFFFFFF


def main():
    install = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("SS2_DATA_DIR")
    if not install:
        print("usage: spawner_multiplier_verify.py <install>  (or set SS2_DATA_DIR)")
        return 1

    # world stem (lowercase, no extension/path) -> {entity id: [spawnee class, ...]}
    # Entity ids are per-world serials, not global, so a tweak table's level name picks the one world to search.
    by_world_id = {}
    worlds = sorted(merged_files(install, ".wld").items())
    for n, (name, d) in enumerate(worlds):
        r = parse(d)
        objs, _ = TreeDec(d, r).objects()
        stem = name.rsplit("/", 1)[-1].rsplit(".", 1)[0].lower()
        table = by_world_id.setdefault(stem, collections.defaultdict(list))
        for o in objs.values():
            if not isinstance(o, Obj) or o.type != "SSpawnerEntity":
                continue
            owner = o.get("SSpawnerEntity.120")
            if not isinstance(owner, Ref) or owner.oid == NULL:
                continue
            spawnee = objs.get(owner.oid)
            if not isinstance(spawnee, Obj):
                continue
            eid = spawnee.get("CEntity.1")
            if eid is None:
                continue
            table[eid].append(spawnee.type)
        print(f"[{n + 1}/{len(worlds)}] {stem}", file=sys.stderr, flush=True)

    total = sum(len(t) for t in by_world_id.values())
    print(f"Indexed spawner owners in {len(worlds)} worlds, {total} distinct (world, entity id) pairs.\n")

    # ---- the two tweak files
    eps = load_eps(install, (".rsc",))
    for fname, (d, r, objs, edob) in sorted(eps.items()):
        has_owner_cond = any(isinstance(o, Obj) and o.type in ("CSpawnerTweakConditionOwnerID", "CSpawnerBootTweakCondition") for o in objs.values())
        if not has_owner_cond:
            continue
        print(f"== {fname} ==")
        levels = {o_id: o for o_id, o in objs.items() if isinstance(o, Obj) and o.type in ("CSpawnerTweakParams", "CSpawnerBootTweaks")}
        for lvl_id, lvl in sorted(levels.items(), key=lambda kv: kv[1].get("CSpawnerTweakParams.ctp_strLevelName") or kv[1].get("CSpawnerBootTweaks.sbt_strLevelName") or ""):
            level_name = lvl.get("CSpawnerTweakParams.ctp_strLevelName") or lvl.get("CSpawnerBootTweaks.sbt_strLevelName")
            conds = lvl.get("CSpawnerTweakParams.ctp_astcTweakCondition") or lvl.get("CSpawnerBootTweaks.sbt_asbtcTweakCondition") or []
            for cref in conds:
                cond = objs.get(cref.oid)
                if not isinstance(cond, Obj) or cond.type not in ("CSpawnerTweakConditionOwnerID", "CSpawnerBootTweakCondition"):
                    continue
                ids = cond.get("CSpawnerTweakConditionOwnerID.stoid_aiOwnerIDs") or cond.get("CSpawnerBootTweakCondition.sbt_aiOwnerIDs") or []
                reason = cond.get("CSpawnerTweakCondition.stc_strReason") or cond.get("CSpawnerBootTweakCondition.sbtc_strReason") or ""
                tw = cond.get("CSpawnerTweakCondition.stp_sttTweakType") or cond.get("CSpawnerBootTweakCondition.sbtc_sbttTweakType") or []
                tw = tw if isinstance(tw, list) else [tw]
                kinds = [objs[t.oid].type for t in tw if isinstance(t, Ref) and t.oid in objs]
                print(f"  {level_name}  tweaks={kinds}  reason={reason!r}")
                world_table = by_world_id.get(str(level_name).lower(), {})
                for eid in ids:
                    hits = world_table.get(eid, [])
                    if not hits:
                        print(f"    id {eid}: no spawner in {level_name}'s own world owns this id")
                    else:
                        print(f"    id {eid}: {sorted(set(hits))}" + (f" (x{len(hits)})" if len(hits) > 1 else ""))
        print()
    return 0


if __name__ == "__main__":
    sys.exit(main())
