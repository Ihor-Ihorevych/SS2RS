"""Evidence for docs/re/wld-lighting.md, "Baked lightmaps" / T01-414 (read-only, SS2_DATA_DIR).

T01-407 found the mesh's own "unwrap-like" UV stream (inside [0, 1], non-overlapping,
`wld_lightmap_stream_verify.py`) for 86.6 percent of lit instances (16,147 of 18,641 resolved), but
left the other 13.4 percent -- a mesh with none or several such streams -- without a rule. This
script tests two deterministic tie-breaks a loader could apply for those residual cases, over every
lit instance resolved in all 59 worlds (reusing `wld_lightmap_stream_verify.analyze_mesh`'s per-mesh
cache, so it costs the same one rasterization pass that script already does):

- **Several candidates:** pick the lowest-indexed declared UV slot among the candidates (consistent
  with the dominant pattern among the single-candidate cases, where slot 0 already wins 87 percent
  of the time).
- **No candidates:** fall back to the mesh's slot 0 (its first declared UV id) regardless of whether
  it is itself inside [0, 1] or non-overlapping -- the fallback `wld-lighting.md`'s reader invariants
  already suggest as "a reasonable default".

Reports, for each bucket (0, 1, 2+ candidates), how many instances/meshes the tie-break resolves to
an id, and -- for the "no candidates" bucket -- how close slot 0 itself comes to passing the [0, 1]
and non-overlap tests (so a reader can judge how much worse the fallback is than a real candidate).

Usage: python -I tools/re/wld_lightmap_tiebreak_verify.py [DATA_DIR] [N]
       (N = resolve+check meshes for only the N smallest worlds, default 59 = all; this reruns the
        same rasterization as wld_lightmap_stream_verify.py, about 2 minutes for all 59 worlds)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, TreeDec, merged_files, parse  # noqa: E402
from wld_lightmap_stream_verify import (  # noqa: E402
    ModelFileIndex, analyze_mesh, candidates, resolved_mesh_for_instance,
)

NULL = 0xFFFFFFFF


def tiebreak_choice(stats):
    """(bucket, chosen_uv_id_or_None, slot0_diagnostic_or_None) for one mesh's `analyze_mesh` stats."""
    cand = candidates(stats)
    if len(cand) == 1:
        return "one", cand[0], None
    if len(cand) > 1:
        chosen = min(cand, key=lambda uid: stats[uid]["slot"])
        return "several", chosen, None
    # zero candidates: fall back to slot 0's own declared id, whatever its own stats say
    slot0_id = next((uid for uid, s in stats.items() if s["slot"] == 0), None)
    diag = stats.get(slot0_id) if slot0_id is not None else None
    return "none", slot0_id, diag


def check_world(name, d, st, dist, index, mdl_cache, mesh_stats_cache, mesh_choice_cache):
    r = parse(d); objs, edob = TreeDec(d, r).objects()
    extra, _ = TreeDec(d, r).extra_objects(edob)
    allo = {**objs, **extra}
    st["worlds"] += 1
    for i, o in objs.items():
        if not (isinstance(o, Obj) and o.type == "CModelInstance" and o.get("CModelInstance.4").oid != NULL):
            continue
        ml = objs[o.get("CModelInstance.4").oid]
        for x in ml.get("CShaderModifierList.1"):
            m = allo.get(x.oid)
            if m is None or m.type != "CTextureOvenShaderModifier":
                continue
            st["oven modifiers (instances) seen"] += 1
            mpath, resolved = resolved_mesh_for_instance(r, o, index, mdl_cache)
            if mpath is None or resolved is None:
                st["instance mesh not resolved"] += 1
                continue
            mobjs, mesh = resolved
            key = mpath.lower()
            if key not in mesh_stats_cache:
                mesh_stats_cache[key] = analyze_mesh(mobjs, mesh)
            stats = mesh_stats_cache[key]
            if stats is None:
                st["mesh had no PC-eligible LOD or no surfaces"] += 1
                continue
            if key not in mesh_choice_cache:
                mesh_choice_cache[key] = tiebreak_choice(stats)
            bucket, chosen, diag = mesh_choice_cache[key]
            st["instances with a mesh stream analysis"] += 1
            st[f"bucket: {bucket} candidate(s)"] += 1
            st[f"bucket {bucket}: tie-break resolves to an id"] += chosen is not None
            st[f"bucket {bucket}: tie-break finds no UV id at all (mesh has none)"] += chosen is None
            if bucket == "none" and diag is not None:
                st["none-bucket: slot 0 is itself inside [0, 1]"] += diag["in01"]
                st["none-bucket: slot 0's own overlap fraction is available (was in01)"] += diag["overlap"] is not None
                if diag["overlap"] is not None:
                    dist["none-bucket: slot 0's own overlap fraction (rounded to 10%)"][round(10 * diag["overlap"])] += 1
                dist["none-bucket: slot 0's own [min_u,max_u]x[min_v,max_v] range (rounded)"][
                    (round(diag["min_u"], 1), round(diag["max_u"], 1), round(diag["min_v"], 1), round(diag["max_v"], 1))
                ] += 1


def run(install, limit):
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter)
    index = ModelFileIndex(install)
    mdl_cache = {}; mesh_stats_cache = {}; mesh_choice_cache = {}
    worlds = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))[:limit]
    t0 = __import__("time").time()
    for n, (name, d) in enumerate(worlds):
        check_world(name, d, st, dist, index, mdl_cache, mesh_stats_cache, mesh_choice_cache)
        if os.environ.get("WLD_PROGRESS"):
            print(f"[{n + 1}/{len(worlds)}] {name} done at {__import__('time').time() - t0:.1f}s", file=sys.stderr)
    for k, v in sorted(st.items()):
        print(f"{v:9d}  {k}")
    print("distinct meshes with a tie-break choice computed:", len(mesh_choice_cache))
    bucket_counts = collections.Counter(b for b, _, _ in mesh_choice_cache.values())
    print("distinct meshes by bucket:", dict(bucket_counts))
    for k, v in sorted(dist.items()):
        print(k, dict(v.most_common(12)))


def main():
    args = sys.argv[1:]
    install = args[0] if args and os.path.isdir(args[0]) else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder (the one containing All_PC_01.gro) or set SS2_DATA_DIR")
    limit = int(args[-1]) if args and args[-1].isdigit() else 59
    run(install, limit)


if __name__ == "__main__":
    main()
