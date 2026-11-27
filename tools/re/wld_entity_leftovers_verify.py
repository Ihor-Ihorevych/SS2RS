"""Re-runs part of the measurement behind docs/re/wld-entities.md on an install (read-only).

Breaks down, by class, the three leftover groups wld_entity_verify.py counts but does not
classify: the owned property records that are neither SSpawnerProperties nor in CWorld.5, the
parent links (CEntityProperties.5) whose target is not a world record, and the entities of
CWorld.1 that have no record in CWorld.5.

Usage: python -I tools/re/wld_entity_leftovers_verify.py [DATA_DIR]   (DATA_DIR defaults to $SS2_DATA_DIR)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, merged_files, parse  # noqa: E402

NULL = 0xFFFFFFFF


def main():
    install = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("SS2_DATA_DIR")
    if not install:
        print("set SS2_DATA_DIR or pass the install directory")
        return 2

    owned_classes = collections.Counter()
    nonworld_parent_classes = collections.Counter()
    entities_without_record = collections.Counter()

    for _path, d in merged_files(install, ".wld").items():
        r = parse(d)
        dec = TreeDec(d, r)
        objs, edob = dec.objects()
        extra, _ = dec.extra_objects(edob)
        allo = {**objs, **extra}
        w = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CWorld")

        world_recs = {}
        for ref in w.get("CWorld.5") or []:
            if isinstance(ref, Ref) and ref.oid != NULL and ref.oid in extra:
                world_recs[ref.oid] = extra[ref.oid]

        recs_all = {oid: o for oid, o in extra.items()
                    if isinstance(o, Obj) and o.get("CEntityProperties.2") is not None}
        owned = {oid: o for oid, o in recs_all.items() if oid not in world_recs}
        own_entity = {}
        for oid, rec in world_recs.items():
            ce = rec.get("CEntityProperties.2")
            if isinstance(ce, Ref):
                own_entity[ce.oid] = oid

        for o in owned.values():
            if o.type != "SSpawnerProperties":
                owned_classes[o.type] += 1

        for rec in world_recs.values():
            par = rec.get("CEntityProperties.5")
            if isinstance(par, Ref) and par.oid != NULL and par.oid not in world_recs:
                tgt = allo.get(par.oid)
                nonworld_parent_classes[tgt.type if tgt is not None else "MISSING"] += 1

        for ref in w.get("CWorld.1") or []:
            if isinstance(ref, Ref) and ref.oid != NULL and ref.oid in objs and ref.oid not in own_entity:
                entities_without_record[objs[ref.oid].type] += 1

    print("Owned records (not SSpawnerProperties, not in CWorld.5), by class:")
    for k, v in owned_classes.most_common():
        print(f"  {k}: {v}")
    print("total:", sum(owned_classes.values()))
    print()
    print("Parent links (.5) whose target is not a world record, by target class:")
    for k, v in nonworld_parent_classes.most_common():
        print(f"  {k}: {v}")
    print("total:", sum(nonworld_parent_classes.values()))
    print()
    print("Entities without a record (CWorld.5), by class:")
    for k, v in entities_without_record.most_common():
        print(f"  {k}: {v}")
    print("total:", sum(entities_without_record.values()))
    return 0


if __name__ == "__main__":
    sys.exit(main())
