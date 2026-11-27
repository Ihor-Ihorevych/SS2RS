"""Evidence for docs/re/chapter-restore.md (T01-172): what the data says about chapters, chapter chains and the entities tied to them.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries).

For every world (.wld, the last patch wins) it prints one line and then totals:
  - the chain of chapter info entities that starts at the world info's first chapter (`CWorldInfoEntity.2`) and follows the next-chapter link
    (`CChapterInfoEntity.2`): its length, whether it ends, whether it loops, the chapters that are not on it;
  - the chapter names (`CChapterInfoEntity.7`), the chapters with a next level path (`CChapterInfoEntity.4`) and whether such a chapter is on the
    chain or off it; the chapters named `EndGameChapter`;
  - the entities that carry a chapter link (`SBaseEntity.1`): by class, and by the position (0 = first chapter) of the chapter they point to;
    entities that point to a chapter that is not on the chain;
  - the placed pickup items of the classes `STreasureItem*` and `SLifeItem*` (the classes the binary reports as not resurrected by a chapter
    restore): how many, and how many carry a chapter link.

Usage: python -I tools/re/chapter_restore_verify.py [DATA_DIR] [--worlds N]
       (DATA_DIR defaults to $SS2_DATA_DIR; --worlds N = only the N smallest worlds; the full run is about 8 minutes)
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
    tot = collections.Counter()
    chain_len = collections.Counter()
    by_class = collections.Counter()
    by_pos = collections.Counter()
    names_all = collections.Counter()
    endgame_worlds = []
    nlink_rows = []
    worlds_no_first = []
    for n, (wname, d) in enumerate(files):
        if limit and n >= limit:
            break
        r = parse(d)
        dec = TreeDec(d, r)
        objs, edob = dec.objects()
        try:
            extra, _ = dec.extra_objects(edob)
        except Exception:
            extra = {}
        allo = {**objs, **extra}
        tot["worlds"] += 1
        chapters = {oid: o for oid, o in allo.items() if isinstance(o, Obj) and o.type == "CChapterInfoEntity"}
        wis = [o for o in allo.values() if isinstance(o, Obj) and o.type == "CWorldInfoEntity"]
        tot["chapter infos"] += len(chapters)
        first = None
        if wis:
            f = wis[0].get("CWorldInfoEntity.2")
            if isinstance(f, Ref) and f.oid != NULL and f.oid in chapters:
                first = f.oid
        pos = {}
        looped = False
        if first is None:
            if chapters:
                worlds_no_first.append(wname.rsplit("/", 1)[-1])
        else:
            cur = first
            while cur in chapters and cur not in pos:
                pos[cur] = len(pos)
                nxt = chapters[cur].get("CChapterInfoEntity.2")
                cur = nxt.oid if isinstance(nxt, Ref) and nxt.oid != NULL else None
            looped = cur in pos
            chain_len[len(pos)] += 1
        off = [c for c in chapters if c not in pos]
        tot["chapters on a chain"] += len(pos)
        tot["chapters off every chain"] += len(off)
        if looped:
            tot["looping chains"] += 1
        for oid, c in chapters.items():
            nm = c.get("CChapterInfoEntity.7")
            nl = c.get("CChapterInfoEntity.4")
            names_all[nm if nm in ("EndGameChapter", "Next Level Link") else "other"] += 1
            if nm == "EndGameChapter":
                endgame_worlds.append((wname.rsplit("/", 1)[-1], oid in pos, pos.get(oid)))
            if isinstance(nl, str) and nl:
                nxt = c.get("CChapterInfoEntity.2")
                has_next = isinstance(nxt, Ref) and nxt.oid != NULL
                nlink_rows.append((oid in pos, has_next, nm if nm in ("EndGameChapter", "Next Level Link") else "other"))
        for oid, o in allo.items():
            if not isinstance(o, Obj):
                continue
            if o.type.startswith(("STreasureItem", "SLifeItem")) and o.type.endswith("Entity"):
                tot["placed " + o.type] += 1
                v0 = o.get("SBaseEntity.1")
                tot["placed treasure/life items with a chapter link"] += isinstance(v0, Ref) and v0.oid != NULL
            v = o.get("SBaseEntity.1")
            if isinstance(v, Ref) and not o.type.endswith("Properties") and v.oid != NULL:
                tot["entities with a chapter link"] += 1
                by_class[o.type] += 1
                if v.oid in pos:
                    by_pos[pos[v.oid]] += 1
                else:
                    tot["links to a chapter off the chain"] += 1
        print(f"[{n + 1}/{len(files)}] {wname.rsplit('/', 1)[-1]}: {len(chapters)} chapters, chain {len(pos)}" + (" LOOP" if looped else "")
              + (f", {len(off)} off the chain" if off else ""), file=sys.stderr, flush=True)
    print(f"worlds {tot['worlds']}; chapter infos {tot['chapter infos']}; on a chain {tot['chapters on a chain']}; off every chain {tot['chapters off every chain']}")
    print("worlds without a first chapter that have chapter infos:", len(worlds_no_first), worlds_no_first)
    print("chain lengths (length: worlds):", dict(sorted(chain_len.items())), "; looping chains:", tot["looping chains"])
    print("chapter names:", dict(names_all))
    print("EndGameChapter (world, on chain, position):", endgame_worlds)
    print("chapters with a next level path (on chain, has next chapter, name kind): " + str(dict(collections.Counter(nlink_rows))))
    print("entities with a chapter link:", tot["entities with a chapter link"], "; pointing off the chain:", tot["links to a chapter off the chain"])
    print("   by class:", dict(by_class.most_common(40)))
    print("   by chapter position (0 = first):", dict(sorted(by_pos.items())))
    print("placed treasure and life items (class: count):", {k[7:]: v for k, v in sorted(tot.items()) if k.startswith("placed S")},
          "; with a chapter link:", tot["placed treasure/life items with a chapter link"])


if __name__ == "__main__":
    main()
