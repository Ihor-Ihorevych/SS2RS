"""Re-runs the measurements behind docs/re/mdl.md on an install (read-only).

For every format-8 `.mdl`: the top-level object structure, the embedded mesh (checked with the
`.bmf` rules from bmf_verify.py), the skeleton, the animation host, the attachment children, the
bone palettes against the model's skeleton, and the kinds of external files a model refers to.

Usage: python -I tools/re/mdl_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR)
"""
import collections, os, struct, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from bmf_verify import check_mesh, g, path  # noqa: E402
from ctse_values import Obj, Ref, TreeDec, merged_files, parse  # noqa: E402

NULL = 0xFFFFFFFF


def kind(v, objs, ext):
    if v.oid == NULL: return "null"
    if v.oid in ext: return "external"
    return objs[v.oid].type if v.oid in objs else "id not in main section"


def bone_ids(skeleton):
    return [{g(b, "SkeletonBone.1") for b in g(lod, "CSkeletonLOD.2")} for lod in g(skeleton, "CSkeleton.1")]


def surfaces(mesh):
    for lod in g(mesh, "CRenderMesh.5"):
        for sec in g(lod, "CRenderMeshLOD.5"):
            yield from g(sec, "CRenderMeshSection.1")


def main():
    install = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder (the one containing All_PC_01.gro) or set SS2_DATA_DIR")
    st = collections.Counter(); mesh_stats = collections.Counter(); covered = {"index": collections.defaultdict(list), "vertex": collections.defaultdict(list)}
    members = collections.defaultdict(collections.Counter); rfil_ext = collections.Counter(); child = collections.Counter(); anim = collections.Counter()
    pal = collections.Counter(); skin = collections.Counter(); errors = collections.Counter()
    for name, d in sorted(merged_files(install, ".mdl").items()):
        r = parse(d); objs, _ = TreeDec(d, r).objects(); st["files"] += 1
        ext = {e[0] for e in r["exob"]}; ext_path = {e[0]: next(f[2] for f in r["rfil"] if f[0] == e[1]) for e in r["exob"]}
        for f in r["rfil"]: rfil_ext[f[2].rsplit(".", 1)[-1].lower()] += 1
        cfgs = [o for o in objs.values() if isinstance(o, Obj) and o.type == "CModelConfiguration"]
        main_cfgs = [o for o in cfgs if g(o, "CResource.1") == 1]
        st["exactly one resource CModelConfiguration (CResource.1 = 1)"] += len(main_cfgs) == 1
        st["files that also embed a helper CModelConfiguration (CResource.1 = -1)"] += len(cfgs) > len(main_cfgs)
        cfg = main_cfgs[0]; hosts = [objs[g(cfg, "CModelConfiguration.9").oid]]
        for k in ("2", "9", "10", "14", "16"): members["CModelConfiguration." + k][kind(g(cfg, "CModelConfiguration." + k), objs, ext)] += 1
        members["mesh instances per model"][len(g(cfg, "CModelConfiguration.3"))] += 1
        members["children per model (bucketed)"][min(len(g(cfg, "CModelConfiguration.8")), 4)] += 1
        # skeleton
        sk_ref = g(cfg, "CModelConfiguration.2"); ids = None
        if sk_ref.oid in objs: ids = bone_ids(objs[sk_ref.oid])
        # meshes
        for mi in g(cfg, "CModelConfiguration.3"):
            m = g(mi, "CMeshInstance.1"); members["CMeshInstance.1"][kind(m, objs, ext)] += 1
            members["CMeshInstance.2"][kind(g(mi, "CMeshInstance.2"), objs, ext)] += 1
            if m.oid not in objs: continue
            mesh = objs[g(objs[m.oid], "CMesh.1").oid]
            try: check_mesh(name, d, r, objs, mesh_stats, covered, mesh)
            except Exception as e: errors[str(e)[:50]] += 1
            for sf in surfaces(mesh):
                palette = set(g(sf, "CRenderMeshSurface.18"))
                if not palette: continue
                pal["surfaces with a bone palette"] += 1
                if ids is None: pal["... model has no embedded skeleton"] += 1
                elif palette <= ids[0]: pal["... palette is a subset of the bone ids of skeleton LOD 0"] += 1
                elif palette <= set().union(*ids): pal["... palette is a subset of the bone ids of some skeleton LOD"] += 1
                else: pal["... palette has ids that are no bone of the skeleton"] += 1
        # animation host
        for inst in g(hosts[0], "CAnimHost.1"):
            a = g(inst, "CAnimSetInstance.1"); anim[kind(a, objs, ext)] += 1
            if a.oid in ext: anim["external file extension ." + ext_path[a.oid].rsplit(".", 1)[-1].lower()] += 1
        # children
        children = [objs[x.oid] for x in g(cfg, "CModelConfiguration.8")]
        own_ids = [g(c, "CModelConfigChild.1") for c in children]
        child["models with children"] += bool(children); child["... with unique child ids"] += bool(children) and len(own_ids) == len(set(own_ids))
        for c in children:
            child["children"] += 1; inst = g(c, "CModelConfigChild.4"); target = g(inst, "CModelInstance.3")
            child["... instance refers to an external " + ext_path[target.oid].rsplit(".", 1)[-1].lower() if target.oid in ext else "... empty attachment slot (no model)"] += target.oid != NULL or True
            if ids is not None:
                child["children of models with an embedded skeleton"] += 1
                child["... parent (member 2) is a bone id of LOD 0"] += g(c, "CModelConfigChild.2") in ids[0]
                child["... own id (member 1) is larger than every bone id"] += g(c, "CModelConfigChild.1") > max(ids[0])
    for k, v in sorted(st.items()): print(f"{v:8d}  {k}")
    for k, v in sorted(members.items()): print(f"{k}: {dict(sorted(v.items(), key=lambda kv: -kv[1]))}")
    print("animation host:", dict(anim)); print("children:", {k: v for k, v in sorted(child.items())})
    print("bone palettes:", dict(pal)); print("external files per type (entries):", dict(rfil_ext.most_common(14)))
    print("embedded meshes, same checks as .bmf:", {k: v for k, v in sorted(mesh_stats.items()) if not k.startswith("weight sum")}, "errors:", dict(errors))


if __name__ == "__main__":
    main()
