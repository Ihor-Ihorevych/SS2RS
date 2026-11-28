"""Re-runs the T01-423 per-frame bloom camera gate and multi-entity worked example on an install (read-only).

For every format-8 `.wld` with two or more `CBloomEntity`s (the retail install has exactly one, `dm_hole`):
prints each bloom's world position (from its own `en_qvPlacement`), its renderable's local `Box3f` placed at
that position, its edge-fade distance (renderable `.2`), and whether the two boxes' *inset* (fade-shrunk)
regions can be simultaneously occupied by one camera position — the geometric precondition for
`CBloomRenderable::RenderEffect` (`Bin/ProcRender.dll`, `docs/re/environment.md` "Bloom") to give both
entities a non-zero fade in the same frame. This does not read any decompiled code; it only re-derives the
same box-inset-by-edge-fade arithmetic `environment.md` already documents in prose, from the shipped data.

Usage: python -I tools/re/wld_bloom_cameratest_verify.py [DATA_DIR] (DATA_DIR defaults to $SS2_DATA_DIR)
"""
import os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Enum, Obj, TreeDec, merged_files, parse  # noqa: E402
from wld_root_verify import ids  # noqa: E402


def nums(o, prefix):
    out = {}
    for k, v in o:
        if not k.startswith(prefix):
            continue
        if isinstance(v, Enum):
            out[k] = v.v
        elif isinstance(v, (int, float)) and not isinstance(v, bool):
            out[k] = v
    return out


def box(v):
    if isinstance(v, Obj) and v.type == "Box3f":
        return [v.get(f"Box3f.{i}") for i in range(1, 7)]
    return None


def position(qv):
    p = qv.get("QuatVect.2")
    return (p.get("Vector3f.1"), p.get("Vector3f.2"), p.get("Vector3f.3"))


def world_position(rend):
    """A renderable's own world placement: `CAspect.7` (inherited by every `CRenderable`, `wld_root_verify.py`
    "aspects: placement hierarchy"), not an `en_qvPlacement` on the entity (bloom/haze entities do not expose
    one; only `CCubeBackgroundEntity` does, for its own editor gizmo)."""
    return position(rend.get("CAspect.7"))


def inset(lo, hi, fade):
    fade = max(fade, 0.001)
    a, b = lo + fade, hi - fade
    if b < a:
        a = b = (a + b) / 2
    return a, b


def check_world(name, d):
    r = parse(d)
    dec = TreeDec(d, r)
    objs, edob = dec.objects()
    w = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CWorld")
    ents = ids(w.get("CWorld.1"))
    blooms = [i for i in ents if objs[i].type == "CBloomEntity"]
    if len(blooms) < 2:
        return None
    short = name.rsplit("/", 1)[-1][:-4]
    out = []
    for i in blooms:
        e = objs[i]
        rend = objs[e.get("CBloomEntity.1").oid]
        pos = world_position(rend)
        lb = box(rend.get("CBloomRenderable.1"))
        fade = rend.get("CBloomRenderable.2")
        world_box = [[pos[a] + lb[a], pos[a] + lb[a + 3]] for a in range(3)]
        insets = [inset(*world_box[a], fade) for a in range(3)]
        rn = nums(rend, "CBloomRenderable.")
        out.append({"pos": pos, "box": world_box, "fade": fade, "inset": insets,
                     "colour": hex(int(rn.get("CBloomRenderable.3", 0))),
                     "members": {k.split(".")[-1]: round(v, 3) for k, v in rn.items()
                                 if k.split(".")[-1] not in ("1", "2", "3")}})
    # a camera position can give both a non-zero fade simultaneously iff their outer (un-inset) boxes
    # overlap on every axis (their inset cores need not coincide: each bloom's own fade is evaluated
    # against its own box independently, per environment.md's formula).
    overlap = all(out[0]["box"][a][0] < out[1]["box"][a][1] and out[1]["box"][a][0] < out[0]["box"][a][1] for a in range(3))
    return short, out, overlap


def main():
    install = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder or set SS2_DATA_DIR")
    files = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))
    found = 0
    for name, d in files:
        res = check_world(name, d)
        if res is None:
            continue
        found += 1
        short, out, overlap = res
        print(f"{short}: {len(out)} bloom entities, outer boxes overlap on every axis: {overlap}")
        for b in out:
            print(f"  pos={tuple(round(c, 1) for c in b['pos'])} fade={b['fade']:.3g} colour={b['colour']} "
                  f"members={b['members']}")
            print(f"    box=[{', '.join(f'({lo:.1f},{hi:.1f})' for lo, hi in b['box'])}] "
                  f"inset=[{', '.join(f'({lo:.1f},{hi:.1f})' for lo, hi in b['inset'])}]")
    print(f"{found} world(s) with 2+ bloom entities out of {len(files)}")


if __name__ == "__main__":
    main()
