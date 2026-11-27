"""Evidence for docs/re/chapter-restore-players.md section 2.8/2.9 (T01-210): where single player
and co-op start.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries).

For every world (.wld, the last patch wins) it checks:
  1. `CSpawnMarkerEntity`: how many worlds have at least one, and whether every such world's file
     name starts with "dm_" (the deathmatch levels) — i.e. whether a single-player or co-op level
     ever has a spawn marker to pick from.
  2. The chapter chain that starts at the world info's first chapter (`CWorldInfoEntity.2`) and
     follows the next-chapter link (`CChapterInfoEntity.2`, same rule as
     `chapter_restore_verify.py`): whether the chain's own first chapter is also the first
     `CChapterInfoEntity` encountered in the world's entity list (`CWorld.1`) — the order
     `ss2-format-wld`'s `World.chapters` keeps, and the order the playable MVP's loader currently
     falls back to (`crates/ss2-world/src/load.rs`).

Usage: python -I tools/re/player_start_verify.py [DATA_DIR] [--worlds N]
       (DATA_DIR defaults to $SS2_DATA_DIR; --worlds N = only the N smallest worlds; the full run is
       about 3 minutes)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, merged_files, parse  # noqa: E402

NULL = 0xFFFFFFFF


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    limit = int(sys.argv[sys.argv.index("--worlds") + 1]) if "--worlds" in sys.argv else None
    if limit is not None:
        args = [a for a in args if a != str(limit)]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder or set SS2_DATA_DIR")
    files = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))

    marker_worlds, non_dm_marker_worlds = [], []
    first_matches_entity_order = collections.Counter()
    off_chain_first_in_entity_order = []
    rank_examples = []

    for n, (wname, d) in enumerate(files):
        if limit and n >= limit:
            break
        short = wname.rsplit("/", 1)[-1]
        r = parse(d)
        dec = TreeDec(d, r)
        objs, edob = dec.objects()
        try:
            extra, _ = dec.extra_objects(edob)
        except Exception:
            extra = {}
        allo = {**objs, **extra}

        markers = [oid for oid, o in allo.items() if isinstance(o, Obj) and o.type == "CSpawnMarkerEntity"]
        if markers:
            marker_worlds.append((short, len(markers)))
            if not short.lower().startswith("dm_"):
                non_dm_marker_worlds.append(short)

        chapters = {oid: o for oid, o in allo.items() if isinstance(o, Obj) and o.type == "CChapterInfoEntity"}
        if not chapters:
            continue
        wis = [o for o in allo.values() if isinstance(o, Obj) and o.type == "CWorldInfoEntity"]
        first = None
        if wis:
            f = wis[0].get("CWorldInfoEntity.2")
            if isinstance(f, Ref) and f.oid != NULL and f.oid in chapters:
                first = f.oid
        if first is None:
            continue

        world_obj = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CWorld")
        entities = world_obj.get("CWorld.1")
        entity_order_chapters = [
            e.oid if isinstance(e, Ref) else e
            for e in entities
            if (e.oid if isinstance(e, Ref) else e) in chapters
        ]
        rank = entity_order_chapters.index(first) + 1 if first in entity_order_chapters else None
        total = len(entity_order_chapters)
        matches = rank == 1
        first_matches_entity_order[matches] += 1
        if rank and rank > 1:
            rank_examples.append((short, rank, total))
        first_in_entity_order = entity_order_chapters[0] if entity_order_chapters else None
        if first_in_entity_order is not None and first_in_entity_order != first:
            # Is the entity-order-first chapter even on the chain at all?
            cur, pos, looped = first, {}, False
            while cur in chapters and cur not in pos:
                pos[cur] = len(pos)
                nxt = chapters[cur].get("CChapterInfoEntity.2")
                cur = nxt.oid if isinstance(nxt, Ref) and nxt.oid != NULL else None
            if first_in_entity_order not in pos:
                off_chain_first_in_entity_order.append(short)

        print(
            f"[{n + 1}/{len(files)}] {short}: spawn markers {len(markers)}; chain first chapter "
            f"rank in entity order {rank} of {total}",
            file=sys.stderr,
            flush=True,
        )

    print(f"worlds with at least one CSpawnMarkerEntity: {len(marker_worlds)}", dict(marker_worlds))
    print("of those, not a dm_* file name:", non_dm_marker_worlds)
    print(
        "worlds with a chapter chain: chain's first chapter IS first in entity order:",
        first_matches_entity_order[True],
        "; is NOT:",
        first_matches_entity_order[False],
    )
    print("worlds where the entity-order-first chapter is off the chain entirely:", off_chain_first_in_entity_order)
    print("rank (chain position 0) examples where it is not 1st in entity order (world, rank, total chapters):")
    for row in rank_examples:
        print("  ", row)


if __name__ == "__main__":
    main()
