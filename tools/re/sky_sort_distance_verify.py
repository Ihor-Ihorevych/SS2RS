"""Evidence for docs/re/environment.md (T01-428, follow-up to T01-422): whether the sky cube's own
render-command tie-break distance reliably sorts it ahead of nearby ordinary opaque draws across the
install, and so whether any draw could leave fog armed before the sky draws.

Read-only on the install's data (CLEANROOM.md): this script reads DATA files only, never the game's
binaries.

T01-422 settled (instruction level, `Engine.dll`) that `CViewRenCmd::Execute`'s `qsort` puts the sky's
own render command in the same near/opaque group as ordinary level geometry (never the back-to-front/
fading group a translucent or dithering-out material uses), and that within that shared group ties are
broken by ascending camera distance — for the sky, a point-to-oriented-box test against its own
`CCubeRenderable` bounding box (half-extent `en_fSize`, centred and oriented by the entity's own
`en_qvPlacement`), which T01-409 already found to be a largely arbitrary per-level value the renderer
never actually draws the cube at (the drawn cube always recentres on the camera). This task re-measures
that distance against real camera positions, to settle whether the sky's tie-break key is usually small
(so it beats nearby geometry and stays unfogged) or usually large (so nearby geometry's own, genuinely
short camera distance beats it, executes first, and so could arm `shaGetFogFactors` before the sky
draws).

For every world with a `CCubeBackgroundEntity`:
  - the sky's own point-to-box distance (world units) from every representative camera position this
    script can find for that world: the single-player/co-op start (the world's first chapter in the
    chapter chain, `player_start_verify.py`'s own definition) and every `CSpawnMarkerEntity` (deathmatch
    spawns).
  - a same-group-opaque proxy distance: the distance from that same camera position to the *nearest*
    `CStaticModelEntity`'s own placement (point distance, not box) — a lower bound on how close some
    ordinary opaque command's own tie-break distance is at that camera position, since retail levels are
    built from tiled static models and `chapter_start_ground_verify.py` already found static models
    routinely within a 15 m radius of a chapter start.

Usage: python -I tools/re/sky_sort_distance_verify.py [DATA_DIR] [--worlds N]
       (DATA_DIR defaults to $SS2_DATA_DIR; --worlds N = only the N smallest worlds; full run ~10 minutes)
"""
import collections
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, merged_files, parse  # noqa: E402

NULL = 0xFFFFFFFF


def quat_matrix(x, y, z, w):
    """Same formula as wld_env_verify.quat_matrix / skl_verify.quat_matrix / wld_root_verify.matrix."""
    return [[1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
            [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
            [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)]]


def decode(data):
    r = parse(data)
    dec = TreeDec(data, r)
    objs, edob = dec.objects()
    try:
        extra, _ = dec.extra_objects(edob)
    except Exception:
        extra = {}
    return {**objs, **extra}


def build_placements(allo):
    """{entity id: (x, y, z)}: the world placement of the first aspect the entity owns whose parent
    aspect is not owned by the same entity (same rule as chapter_start_ground_verify.py / the
    AspectIndex::placement rule `ss2-format-wld` already uses for every other placement)."""
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


def point_to_box_distance(point, center, rotation, half_extent):
    """Euclidean distance from `point` to the surface of an oriented box: `center` is the box's own
    world position, `rotation` its 3x3 (row) matrix, `half_extent` its half-size on every local axis
    (the sky cube is a cube, so one scalar). Zero when the point is inside. Same construction as the
    haze/bloom camera-distance test already used elsewhere in this codebase (`environment.md`,
    "Role and formulas of .5 .6 .7 .10"): transform into the box's local frame, clamp per axis, measure
    the residual."""
    d = [point[i] - center[i] for i in range(3)]
    # world-to-local: multiply by the rotation matrix's transpose (its inverse, since it is orthonormal)
    local = [sum(rotation[j][i] * d[j] for j in range(3)) for i in range(3)]
    clamped = [max(-half_extent, min(half_extent, c)) for c in local]
    residual = [local[i] - clamped[i] for i in range(3)]
    return math.sqrt(sum(r * r for r in residual))


def sky_box(allo):
    """(center, rotation, half_extent) of the one CCubeBackgroundEntity's own bounding box, or None."""
    skies = [o for o in allo.values() if isinstance(o, Obj) and o.type == "CCubeBackgroundEntity"]
    if not skies:
        return None
    e = skies[0]
    qv = e.get("CCubeBackgroundEntity.en_qvPlacement")
    q = qv.get("QuatVect.1")
    x, y, z, w = (q.get(f"Quaternion4f.{i}") for i in (1, 2, 3, 4))
    rot = quat_matrix(x, y, z, w)
    pos = qv.get("QuatVect.2")
    center = (pos.get("Vector3f.1"), pos.get("Vector3f.2"), pos.get("Vector3f.3"))
    half_extent = e.get("CCubeBackgroundEntity.en_fSize")
    return center, rot, half_extent


def nearest_static_model_distance(point, static_positions):
    if not static_positions:
        return None
    return min(math.dist(point, p) for p in static_positions)


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    limit = int(sys.argv[sys.argv.index("--worlds") + 1]) if "--worlds" in sys.argv else None
    if limit is not None:
        args = [a for a in args if a != str(limit)]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder or set SS2_DATA_DIR")
    files = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))

    worlds_with_sky = 0
    worlds_with_camera = 0
    rows = []  # (short, world_sample_count, min_sky_dist, max_sky_dist, worst_case_count, cases)

    for n, (wname, d) in enumerate(files):
        if limit and n >= limit:
            break
        short = wname.rsplit("/", 1)[-1][:-4]
        allo = decode(d)
        box = sky_box(allo)
        if box is None:
            print(f"[{n + 1}/{len(files)}] {short}: no sky", file=sys.stderr, flush=True)
            continue
        worlds_with_sky += 1
        center, rot, half_extent = box

        placements = build_placements(allo)
        static_positions = [placements[oid] for oid, o in allo.items()
                             if isinstance(o, Obj) and o.type == "CStaticModelEntity" and oid in placements]

        cameras = []  # (label, position)
        fc = first_chapter(allo)
        if fc is not None and fc in placements:
            cameras.append(("start", placements[fc]))
        markers = [oid for oid, o in allo.items() if isinstance(o, Obj) and o.type == "CSpawnMarkerEntity"]
        for i, oid in enumerate(markers):
            if oid in placements:
                cameras.append((f"spawn{i}", placements[oid]))

        if not cameras:
            print(f"[{n + 1}/{len(files)}] {short}: sky but no start/spawn camera found", file=sys.stderr, flush=True)
            rows.append((short, 0, None, None, None, []))
            continue
        worlds_with_camera += 1

        cases = []
        for label, cam in cameras:
            sky_d = point_to_box_distance(cam, center, rot, half_extent)
            near_d = nearest_static_model_distance(cam, static_positions)
            cases.append((label, round(sky_d, 1), round(near_d, 1) if near_d is not None else None))
        sky_ds = [c[1] for c in cases]
        beats_count = sum(1 for _, sd, nd in cases if nd is not None and nd < sd)
        rows.append((short, len(cases), min(sky_ds), max(sky_ds), beats_count, cases))
        print(f"[{n + 1}/{len(files)}] {short}: {len(cases)} camera(s), sky dist {min(sky_ds):.1f}-{max(sky_ds):.1f}, "
              f"{beats_count} case(s) where the nearest static model is closer", file=sys.stderr, flush=True)

    print(f"\nworlds with a sky entity: {worlds_with_sky}")
    print(f"worlds with at least one start/spawn camera found: {worlds_with_camera}")

    with_camera = [r for r in rows if r[1] > 0]
    zero_sky_dist = [r for r in with_camera if r[2] == 0.0]
    print(f"\nworlds where the sky's own distance is exactly 0 at every sampled camera (camera already "
          f"inside the sky's placement box): {len([r for r in with_camera if r[3] == 0.0])}")
    print(f"worlds where the sky's own distance is 0 at at least one sampled camera: {len(zero_sky_dist)}")

    beaten = [r for r in with_camera if r[4] and r[4] > 0]
    print(f"\nworlds with at least one camera where a nearby static model's own placement is CLOSER to "
          f"the camera than the sky's own box distance (so that model's render command sorts, and could "
          f"arm fog, before the sky's does): {len(beaten)} of {len(with_camera)}")
    for short, count, mind, maxd, beats, cases in sorted(rows, key=lambda r: -(r[3] or -1))[:20]:
        print(f"  {short}: sky dist {mind}-{maxd} over {count} camera(s); beaten in {beats} case(s): {cases}")

    print("\nfull per-world distances (sky box distance, nearest static model distance), sorted by world:")
    for short, count, mind, maxd, beats, cases in sorted(rows):
        print(f"  {short}: {cases}")


if __name__ == "__main__":
    main()
