"""Evidence for docs/re/chapter-restore-players.md section 2.10 (T01-411): why a few single-player
starts have no static ground close under them, and that a chapter's own parent link
(`en_henParent`) is not the cause.

Read-only on the install's data (CLEANROOM.md): this script reads DATA files only, never the
game's binaries.

For every world with a chapter chain it reports:
  1. Across all 307 `CChapterInfoEntity` records of the install: how many have `en_henParent` set
     (`mechanism-classes-world-info.md` already counts 14; this is an independent recount) and,
     specifically, whether the world's own first chapter (the single-player/co-op start,
     `chapter-restore-players.md` section 2.8) is one of them.
  2. For the two worlds named on the command line (default: `5_1_Village`, `5_2_SeasideTownApproaching`),
     every entity with its own placement within a given radius of the first chapter, classified as a
     static model or not — the non-static neighbours are candidates for "the ground" a player-start
     verifier that only tests `CStaticModelEntity` instances would miss.

Usage: python -I tools/re/chapter_start_ground_verify.py [DATA_DIR] [--radius N] [--worlds name1,name2,...]
       (DATA_DIR defaults to $SS2_DATA_DIR; the full parent-link census is about 3 minutes)
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, merged_files, parse  # noqa: E402

NULL = 0xFFFFFFFF


def build_placements(allo):
    """{entity id: (x, y, z)}: the world placement of the first aspect the entity owns whose
    parent aspect is not owned by the same entity — the same rule as `ss2-format-wld`'s
    `AspectIndex::placement`."""
    owned = {}
    aspects = {}
    for oid, o in allo.items():
        if not isinstance(o, Obj):
            continue
        world = o.get("CAspect.7")
        owner = o.get("CAspect.5")
        if world is None or not (isinstance(owner, Ref) and owner.oid != NULL):
            continue
        parent = o.get("CAspect.1")
        parent_id = parent.oid if isinstance(parent, Ref) and parent.oid != NULL else None
        owned.setdefault(owner.oid, []).append(oid)
        aspects[oid] = (parent_id, owner.oid, world)

    placements = {}
    for owner, aspect_ids in owned.items():
        for aid in aspect_ids:
            parent_id, _own, world = aspects[aid]
            parent_owner = aspects.get(parent_id, (None, None, None))[1] if parent_id else None
            if parent_owner != owner:
                vec = world.get("QuatVect.2")
                placements[owner] = (vec.get("Vector3f.1"), vec.get("Vector3f.2"), vec.get("Vector3f.3"))
                break
    return placements


def first_chapter(allo):
    wis = [o for o in allo.values() if isinstance(o, Obj) and o.type == "CWorldInfoEntity"]
    if not wis:
        return None
    ref = wis[0].get("CWorldInfoEntity.2")
    if isinstance(ref, Ref) and ref.oid != NULL and allo.get(ref.oid, Obj("")).type == "CChapterInfoEntity":
        return ref.oid
    return None


def decode(data):
    r = parse(data)
    dec = TreeDec(data, r)
    objs, edob = dec.objects()
    try:
        extra, _ = dec.extra_objects(edob)
    except Exception:
        extra = {}
    return {**objs, **extra}


def parent_census(files):
    total = 0
    with_parent = 0
    first_chapter_with_parent = []
    for wname, data in files.items():
        allo = decode(data)
        chapters = {oid: o for oid, o in allo.items() if isinstance(o, Obj) and o.type == "CChapterInfoEntity"}
        if not chapters:
            continue
        total += len(chapters)
        for oid, o in chapters.items():
            parent = o.get("CChapterInfoEntity.en_henParent")
            if isinstance(parent, Ref) and parent.oid != NULL:
                with_parent += 1
        fc = first_chapter(allo)
        if fc is not None and isinstance(chapters[fc].get("CChapterInfoEntity.en_henParent"), Ref) \
                and chapters[fc].get("CChapterInfoEntity.en_henParent").oid != NULL:
            first_chapter_with_parent.append(wname.rsplit("/", 1)[-1])
    print(f"chapter info entities: {total}; with en_henParent set: {with_parent}")
    print(f"world first chapters with en_henParent set: {first_chapter_with_parent or 'none'}")


def neighbourhood(wname, data, radius):
    allo = decode(data)
    placements = build_placements(allo)
    fc = first_chapter(allo)
    short = wname.rsplit("/", 1)[-1]
    if fc is None or fc not in placements:
        print(f"{short}: no first chapter with a placement")
        return
    chap = allo[fc]
    cx, cy, cz = placements[fc]
    parent = chap.get("CChapterInfoEntity.en_henParent")
    print(f"\n{short}: first chapter object {fc} '{chap.get('CChapterInfoEntity.7')}' at "
          f"({cx:.2f}, {cy:.2f}, {cz:.2f}); en_henParent={'null' if not (isinstance(parent, Ref) and parent.oid != NULL) else parent}")
    near = []
    for oid, (x, y, z) in placements.items():
        if oid == fc:
            continue
        dxz = math.hypot(x - cx, z - cz)
        if dxz <= radius:
            kind = allo[oid].type if isinstance(allo.get(oid), Obj) else "?"
            near.append((dxz, cy - y, oid, kind))
    near.sort()
    static_below = [n for n in near if n[3] == "CStaticModelEntity" and n[1] >= -0.5]
    non_static_same_height = [n for n in near if n[3] != "CStaticModelEntity" and abs(n[1]) < 1.0]
    print(f"  within {radius} m (xz): {len(near)} entities; "
          f"{len(static_below)} static models at/below; "
          f"{len(non_static_same_height)} non-static entities within 1 m of the chapter's own height")
    for dxz, dy, oid, kind in near[:12]:
        print(f"    dxz={dxz:6.2f} dy={dy:7.2f}  {kind:24s} object {oid}")


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    radius = 15.0
    if "--radius" in sys.argv:
        radius = float(sys.argv[sys.argv.index("--radius") + 1])
    worlds = ["5_1_village", "5_2_seasidetownapproaching"]
    if "--worlds" in sys.argv:
        worlds = sys.argv[sys.argv.index("--worlds") + 1].split(",")
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder or set SS2_DATA_DIR")

    files = merged_files(install, ".wld")
    parent_census(files)
    for key in worlds:
        matches = [n for n in files if key.lower() in n]
        for wname in matches:
            neighbourhood(wname, files[wname], radius)


if __name__ == "__main__":
    main()
