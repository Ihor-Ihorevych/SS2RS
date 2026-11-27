"""Evidence for docs/re/wld-lighting.md, "Baked lightmaps" / T01-406 (read-only, SS2_DATA_DIR).

For every lit model instance's oven modifier: checks whether the lightmap UV map id ever occurs
in the id list of the instance's own mesh (every surface, every LOD, all 8 UV slots -- not just the
slots a material actually uses) and reports how the ids are distributed (unique per instance? per
world? clustered in a small range?). Also counts the rare "UV map" editor classes (CUVMapLayout,
CForegroundUVMapList, CTexCoordVertexMap, CUVMapViewCustomData) per world, to check whether they
track the oven-modifier count (they do not -- see the doc).

Usage: python -I tools/re/wld_lightmap_uv_verify.py [DATA_DIR] [N]
       (DATA_DIR defaults to $SS2_DATA_DIR; N = resolve+check meshes for only the N smallest worlds,
        default 8 -- this part opens every referenced .mdl, so it is the slow part; the id-distribution
        and editor-class counts always run over all 59 worlds; about 5 minutes for N=8)
"""
import collections, os, sys, zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, merged_files, parse  # noqa: E402

NULL = 0xFFFFFFFF
UV_EDITOR_CLASSES = ("CUVMapLayout", "CForegroundUVMapList", "CTexCoordVertexMap", "CUVMapViewCustomData")


def model_mesh_uv_ids(d):
    """All UV map ids declared anywhere on the mesh (every LOD, every surface, all 8 slots), as a set."""
    r = parse(d); dec = TreeDec(d, r); objs, _ = dec.objects()
    out = set()
    for o in objs.values():
        if isinstance(o, Obj) and o.type == "CRenderMesh":
            for lod in o.get("CRenderMesh.5"):
                for section in lod.get("CRenderMeshLOD.5"):
                    for sf in section.get("CRenderMeshSection.1"):
                        out.update(sf.get("CRenderMeshSurface.12"))
    return out - {0}


class MdlIndex:
    """Lazy {lowercase path: bytes} for .mdl files across the install's .gro archives (loaded on demand)."""
    def __init__(self, install):
        self.loc = {}
        for f in sorted(x for x in os.listdir(install) if x.lower().endswith(".gro")):
            with zipfile.ZipFile(os.path.join(install, f)) as z:
                for i in z.infolist():
                    if not i.is_dir() and i.filename.lower().endswith(".mdl"):
                        self.loc[i.filename.lower()] = (f, i.filename)
        self.install = install; self.cache = {}

    def get(self, path):
        key = path.lower()
        if key not in self.cache:
            arch, name = self.loc.get(key, (None, None))
            self.cache[key] = None if arch is None else zipfile.ZipFile(os.path.join(self.install, arch)).read(name)
        return self.cache[key]


def model_name_of_instance(r, inst):
    """External .mdl path for a CModelInstance.3 reference, or None (embedded CModelConfiguration or unresolved)."""
    ref = inst.get("CModelInstance.3")
    if not isinstance(ref, Ref) or ref.oid == NULL:
        return None
    exob = {e[0]: e for e in r["exob"]}
    e = exob.get(ref.oid)
    if e is None or e[2] != 1:        # (object id, rfil index, flags, type id); flags==1: a plain external file reference
        return None
    idx = e[1]
    if not (0 <= idx < len(r["rfil"])):
        return None
    path = r["rfil"][idx][2]
    return path if path.lower().endswith(".mdl") else None


def check_world(name, d, st, dist, mdl_index):
    r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects(); extra, _ = dec.extra_objects(edob)
    allo = {**objs, **extra}
    uv_ids_this_world = []
    for i, o in objs.items():
        if not (isinstance(o, Obj) and o.type == "CModelInstance" and o.get("CModelInstance.4").oid != NULL):
            continue
        ml = objs[o.get("CModelInstance.4").oid]
        for x in ml.get("CShaderModifierList.1"):
            m = allo.get(x.oid)
            if m is None or m.type != "CTextureOvenShaderModifier":
                continue
            uvid = m.get("CTextureOvenShaderModifier.3").get("CShaderArgValUVMap.1")
            uv_ids_this_world.append(uvid)
            st["oven modifiers seen"] += 1
            if mdl_index is None:
                continue
            path = model_name_of_instance(r, o)
            if path is None:
                st["instance model not resolved to an external .mdl (embedded or unresolved)"] += 1
                continue
            data = mdl_index.get(path)
            if data is None:
                st["referenced .mdl not found in install"] += 1
                continue
            mesh_ids = model_mesh_uv_ids(data)
            st["instances checked against their own mesh's full UV id list"] += 1
            st["lightmap UV id found in the mesh's own UV id list (any slot, any LOD)"] += uvid in mesh_ids
            dist["mesh UV ids actually declared (sample)"][tuple(sorted(mesh_ids))] += 1
    c = collections.Counter(uv_ids_this_world)
    st["worlds"] += 1
    st["oven modifiers whose UV id is unique within their own world"] += sum(1 for v in c.values() if v == 1)
    st["... total oven modifiers (cross-check)"] += sum(c.values())
    if uv_ids_this_world:
        dist["min/max UV id per world"][(min(uv_ids_this_world), max(uv_ids_this_world))] += 1
        top_id, top_n = c.most_common(1)[0]
        st["modifiers whose UV id equals their world's single most common id"] += top_n
        dist["distinct UV ids used, per world"][len(c)] += 1
        dist["most common id's share of the world's modifiers (top_n / total, rounded to 10%)"][round(10 * top_n / len(uv_ids_this_world))] += 1
    counts = collections.Counter(o.type for o in allo.values() if isinstance(o, Obj))
    for cls in UV_EDITOR_CLASSES:
        if counts[cls]:
            dist[f"worlds with {cls} > 0, and its count"][counts[cls]] += 1
        st[f"worlds with at least one {cls}"] += counts[cls] > 0


def main():
    args = sys.argv[1:]
    install = args[0] if args and os.path.isdir(args[0]) else os.environ.get("SS2_DATA_DIR", "")
    limit = int(args[-1]) if args and args[-1].isdigit() else 8
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder (the one containing All_PC_01.gro) or set SS2_DATA_DIR")
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter)
    worlds = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))
    mdl_index = MdlIndex(install)
    for n, (name, d) in enumerate(worlds):
        check_world(name, d, st, dist, mdl_index if n < limit else None)
    for k, v in sorted(st.items()): print(f"{v:9d}  {k}")
    for k, v in sorted(dist.items()):
        if k == "mesh UV ids actually declared (sample)":
            print(k, dict(v.most_common(10)))
        else:
            print(k, dict(v.most_common(12)))


if __name__ == "__main__":
    main()
