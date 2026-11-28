"""Evidence for docs/re/wld-lighting.md, "Baked lightmaps" / T01-414 (read-only, SS2_DATA_DIR).

T01-408 found that the generic per-draw vertex-stream table every shader class' `Exec` reads
through (`shaGetChannel`/`shaBindChannel`, `Engine.dll`) is built from **two** same-sized groups of
id'd slots per surface, and that only the first group -- the mesh's own declared UV ids
(`bmf.md` member `.12`, paired with element paths `.14`) -- had been checked against the oven
modifier's own (never-matching, T01-406) UV map id. This script checks the follow-up's first
hypothesis directly against the data: a fresh read of the per-surface stream-binding helper inside
`Engine.dll` (not committed; see `docs/re/wld-lighting.md`) shows the **second** group's ids and
element paths come from the exact byte offsets `CRenderMeshSurface`'s own layout gives to member
`.13` ("eight color-map identifiers", paired with element paths `.15`) -- the structural twin of
`.12`/`.14`, already named in `bmf.md` but, per that spec's own data, always empty (member `.5`, the
colour-stream count, is 0 in all 353 surfaces of the 76-file retail `.bmf` sample).

This script re-checks that emptiness directly on the meshes **actually referenced by lit world
instances** (the `.mdl`/`.bmf` chain, following one level of external `.bmf` reference, same
resolution as `wld_lightmap_stream_verify.py`), over all 59 worlds -- not just the general `.bmf`
population -- and checks whether the oven modifier's own UV map id (`.3`) ever occurs in a lit
surface's own `.13` array. No rasterization is needed (only integer id comparisons), so this runs
over every lit instance in every world, unlike the slower stream-candidate check.

Usage: python -I tools/re/wld_lightmap_colormap_verify.py [DATA_DIR]
       (DATA_DIR defaults to $SS2_DATA_DIR; always resolves every lit instance's mesh, all 59 worlds)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
from wld_lightmap_stream_verify import ModelFileIndex, load_render_mesh, pc_lod, lod_surfaces  # noqa: E402
from wld_lightmap_uv_verify import model_name_of_instance  # noqa: E402

NULL = 0xFFFFFFFF


def surface_colormap_ids(sf):
    """The surface's own 8-slot colour-map id array (`.13`) and its declared colour-stream count (`.5`)."""
    ids = sf.get("CRenderMeshSurface.13")
    count = sf.get("CRenderMeshSurface.5")
    return [i for i in ids if i != 0], count


def check_world(name, d, st, dist, index, mdl_cache, mesh_cache):
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
            uvid = m.get("CTextureOvenShaderModifier.3").get("CShaderArgValUVMap.1")
            mpath = model_name_of_instance(r, o)
            if mpath is None:
                st["instance model not resolved to an external .mdl (embedded or unresolved)"] += 1
                continue
            key = mpath.lower()
            if key not in mdl_cache:
                data = index.get(mpath)
                mdl_cache[key] = load_render_mesh(data, index) if data is not None else None
            resolved = mdl_cache[key]
            if resolved is None:
                st["model resolved but its mesh could not be loaded"] += 1
                continue
            mobjs, mesh = resolved
            if key not in mesh_cache:
                lod = pc_lod(mesh)
                surfs = lod_surfaces(lod) if lod is not None else []
                mesh_cache[key] = [surface_colormap_ids(sf) for sf in surfs]
            per_surface = mesh_cache[key]
            if not per_surface:
                st["mesh had no PC-eligible LOD or no surfaces"] += 1
                continue
            st["instances checked against their own mesh's colour-map id array"] += 1
            any_nonempty = any(ids for ids, _ in per_surface)
            any_count = any(count for _, count in per_surface)
            st["instances whose mesh has at least one non-empty .13 colour-map id slot (any surface)"] += any_nonempty
            st["instances whose mesh declares a non-zero .5 colour-stream count (any surface)"] += any_count
            all_ids = {i for ids, _ in per_surface for i in ids}
            dist["distinct non-zero colour-map ids declared (sample, over instances with any)"][tuple(sorted(all_ids))] += 1 if all_ids else 0
            st["lightmap UV id (.3) found among the mesh's own .13 colour-map ids (any surface)"] += uvid in all_ids


def run(install):
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter)
    index = ModelFileIndex(install)
    mdl_cache = {}; mesh_cache = {}
    worlds = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))
    t0 = __import__("time").time()
    for n, (name, d) in enumerate(worlds):
        check_world(name, d, st, dist, index, mdl_cache, mesh_cache)
        if os.environ.get("WLD_PROGRESS"):
            print(f"[{n + 1}/{len(worlds)}] {name} done at {__import__('time').time() - t0:.1f}s", file=sys.stderr)
    for k, v in sorted(st.items()):
        print(f"{v:9d}  {k}")
    print("distinct meshes resolved:", len(mesh_cache))
    for k, v in sorted(dist.items()):
        print(k, dict(v.most_common(12)))


def main():
    args = sys.argv[1:]
    install = args[0] if args and os.path.isdir(args[0]) else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder (the one containing All_PC_01.gro) or set SS2_DATA_DIR")
    run(install)


if __name__ == "__main__":
    main()
