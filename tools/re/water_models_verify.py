"""Which models use the water shader (`CWaterShaderArgs`, `mtr.md`), per level (read-only).

For every format-8 `.wld`, resolves each `CStaticModelEntity`'s external `.mdl`, then for every
mesh surface of that model resolves its material (`CRenderMeshSurface.16`, `bmf.md`): external
materials are looked up in the water-shader set built once over every `.mtr`/`.bmf`/`.mdl` file of
the install (any embedded `CShaderConfig.2` whose arguments object is a `CWaterShaderArgs`);
embedded materials are checked the same way inside the model's own objects.

Usage: python -I tools/re/water_models_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from bmf_verify import g  # noqa: E402
from ctse_values import Obj, TreeDec, merged_files, parse  # noqa: E402

NULL = 0xFFFFFFFF


def surfaces_of(objs, mdl_root_oid):
    """Every CRenderMeshSurface reachable from the model's main CModelConfiguration."""
    cfgs = [o for o in objs.values() if isinstance(o, Obj) and o.type == "CModelConfiguration" and g(o, "CResource.1") == 1]
    if not cfgs: return
    for mi in g(cfgs[0], "CModelConfiguration.3"):
        m = g(mi, "CMeshInstance.1")
        if m.oid not in objs: continue
        mesh = objs[g(objs[m.oid], "CMesh.1").oid]
        for lod in g(mesh, "CRenderMesh.5"):
            for sec in g(lod, "CRenderMeshLOD.5"):
                yield from g(sec, "CRenderMeshSection.1")


def preset_is_water(objs, preset_oid):
    if preset_oid not in objs: return False
    preset = objs[preset_oid]
    if not (isinstance(preset, Obj) and preset.type == "CShaderPreset"): return False
    for cf in g(preset, "CShaderPreset.1"):
        a = cf.get("CShaderConfig.2")
        if a.oid in objs and objs[a.oid].type == "CWaterShaderArgs": return True
    return False


def file_is_water(objs):
    """True if any embedded CShaderPreset of this file (.mtr/.bmf/.mdl) uses CWaterShaderArgs."""
    return any(isinstance(o, Obj) and o.type == "CShaderPreset" and preset_is_water(objs, oid) for oid, o in objs.items())


def model_uses_water(name, mdls, water_mtrs, cache):
    """None if the model could not be resolved; else True/False, cached across worlds."""
    if name in cache: return cache[name]
    d = mdls.get(name)
    if d is None: cache[name] = None; return None
    r = parse(d); objs, _ = TreeDec(d, r).objects()
    rf = {e[0]: e[2] for e in r["rfil"]}; exm = {e[0]: rf[e[1]] for e in r["exob"]}
    used_water = False
    for sf in surfaces_of(objs, None):
        ref = g(sf, "CRenderMeshSurface.16")
        if ref.oid == NULL: continue
        if ref.oid in exm:
            if exm[ref.oid].lower() in water_mtrs: used_water = True
        elif preset_is_water(objs, ref.oid):
            used_water = True
    cache[name] = used_water
    return used_water


def main():
    install = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder (the one containing All_PC_01.gro) or set SS2_DATA_DIR")

    water_mtrs = set()
    for ext in (".mtr", ".bmf", ".mdl"):
        for name, d in merged_files(install, ext).items():
            r = parse(d); objs, _ = TreeDec(d, r).objects()
            if file_is_water(objs): water_mtrs.add(name)
    print(f"materials (.mtr/.bmf/.mdl files) that embed a CWaterShaderArgs config: {len(water_mtrs)}")
    for n in sorted(water_mtrs): print("  ", n)

    mdls = merged_files(install, ".mdl")
    cache = {}
    per_world = {}
    st = collections.Counter()
    for name, d in sorted(merged_files(install, ".wld").items()):
        r = parse(d); objs, _ = TreeDec(d, r).objects()
        rf = {e[0]: e[2] for e in r["rfil"]}; exm = {e[0]: rf[e[1]] for e in r["exob"]}
        water_here = collections.Counter(); embedded = 0; unresolved = 0
        for e in objs.values():
            if not (isinstance(e, Obj) and e.type == "CStaticModelEntity"): continue
            em = g(e, "CStaticModelEntity.6")
            if em.oid == NULL: continue
            if em.oid not in exm:
                embedded += 1; continue
            mp = exm[em.oid].lower()
            uses = model_uses_water(mp, mdls, water_mtrs, cache)
            if uses is None: unresolved += 1; st["model file not found in install"] += 1
            elif uses: water_here[mp] += 1
        if water_here or embedded or unresolved:
            per_world[name] = (water_here, embedded, unresolved)
        st["worlds"] += 1
        st["worlds with at least one water-shader model instance"] += bool(water_here)

    print(f"\n{st['worlds']} worlds; {st['worlds with at least one water-shader model instance']} use the water shader on a static model instance")
    for name, (water_here, embedded, unresolved) in sorted(per_world.items()):
        if not water_here: continue
        models = "; ".join(f"{m} x{c}" for m, c in sorted(water_here.items()))
        print(f"  {name}: {models}")


if __name__ == "__main__":
    main()
