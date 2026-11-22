"""Re-runs the measurements behind docs/re/wld-lighting.md on an install (read-only).

For every format-8 `.wld` (smallest first): light entities, their light sources and property
records, the light list and light grid, the baked lightmap chain (model instance -> oven
modifier -> lightmap texture, bake record, ambient mask), flares, and the single-instance
effect entities (bloom, haze, cube background).

Usage: python -I tools/re/wld_lighting_verify.py [DATA_DIR] [LIMIT]
       (DATA_DIR defaults to $SS2_DATA_DIR; LIMIT = only the LIMIT smallest worlds, default all, about 20 minutes)
"""
import collections, math, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
from wld_root_verify import ancestry, close, ids, matrix  # noqa: E402

NULL = 0xFFFFFFFF
# source member -> property member that holds the same value ("rad": the source stores radians, the record degrees)
POINT = [("CLightSource.1", "CPointLightProperties.1", "eq"), ("CLightSource.2", "CPointLightProperties.2", "eq"),
         ("CLightSource.5", "CPointLightProperties.14", "eq"), ("CLightSource.6", "CPointLightProperties.16", "eq"),
         ("CPointLightSource.1", "CPointLightProperties.3", "eq"), ("CPointLightSource.2", "CPointLightProperties.4", "eq"),
         ("CPointLightSource.6", "CPointLightProperties.18", "rad"), ("CPointLightSource.7", "CPointLightProperties.19", "rad"),
         ("CPointLightSource.9", "CPointLightProperties.21", "eq")]
DIRECTIONAL = [("CLightSource.1", "CDirectionalLightProperties.1", "eq"), ("CLightSource.2", "CDirectionalLightProperties.5", "eq"),
               ("CLightSource.3", "CDirectionalLightProperties.7", "eq"), ("CLightSource.4", "CDirectionalLightProperties.8", "eq"),
               ("CLightSource.5", "CDirectionalLightProperties.12", "eq"), ("CLightSource.6", "CDirectionalLightProperties.14", "eq")]
LIGHTS = {"CPointLightEntity": ("CPointLightEntity.2", POINT), "CDirectionalLightEntity": ("CDirectionalLightEntity.2", DIRECTIONAL)}


def same(a, b, mode):
    a = getattr(a, "v", a); b = getattr(b, "v", b)
    if not isinstance(a, (int, float)) or not isinstance(b, (int, float)): return False
    return abs(a - b) < 1e-4 if mode == "rad" and False else (abs(a - math.radians(b)) < 1e-4 if mode == "rad" else abs(a - b) < 1e-6 * max(1, abs(a)))


def check_world(name, d, st, dist):
    r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects(); extra, _ = dec.extra_objects(edob)
    allo = {**objs, **extra}; kinds = {oid: ancestry(r, tid) for oid, tid in r["obty"]}
    w = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CWorld"); rd = objs[w.get("CWorld.2").oid]
    rf = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CResourceFile")
    props = {p.get("CEntityProperties.2").oid: p for p in extra.values() if isinstance(p, Obj) and p.get("CEntityProperties.2") is not None}
    prop_id = {id(p): k for k, p in extra.items()}
    asp = {i: o for i, o in objs.items() if isinstance(o, Obj) and "CAspect" in kinds[i]}

    def top(i):
        own = asp[i].get("CAspect.5").oid
        while True:
            p = asp[i].get("CAspect.1").oid
            if p == NULL or asp[p].get("CAspect.5").oid != own: return i
            i = p

    # lights
    sources = set()
    for i, e in objs.items():
        if not (isinstance(e, Obj) and e.type in LIGHTS): continue
        member, mapping = LIGHTS[e.type]; sid = e.get(member).oid; src = objs[sid]; pr = props[i]; sources.add(sid)
        st[f"{e.type}s"] += 1
        st["light source has no owner (CAspect.5 is null)"] += src.get("CAspect.5").oid == NULL
        kids = []; c = src.get("CAspect.2").oid
        while c != NULL and c not in kids: kids.append(c); c = asp[c].get("CAspect.3").oid
        for k in kids: dist["children of a light source aspect: (class, owner class)"][(asp[k].type, objs[asp[k].get("CAspect.5").oid].type if asp[k].get("CAspect.5").oid in objs else "null")] += 1
        st["light source has a child aspect owned by its light entity (editor marker)"] += any(asp[k].get("CAspect.5").oid == i for k in kids)
        for sk, pk, mode in mapping: st[f"{e.type}: {sk} == {pk}" + (" (radians)" if mode == "rad" else "")] += same(src.get(sk), pr.get(pk), mode)
        par = e.get(f"{e.type[:-6]}Entity.3") if False else None
        pe = e.get("CPointLightEntity.3") if e.type == "CPointLightEntity" else e.get("CDirectionalLightEntity.3")
        pe = pe.oid if pe is not None else NULL
        t = top(sid)
        st["light source is a root aspect exactly when the light has no parent entity"] += (src.get("CAspect.1").oid == NULL) == (pe == NULL)
        if pe != NULL: st["light with parent: source's parent aspect belongs to the parent entity"] += asp[src.get("CAspect.1").oid].get("CAspect.5").oid == pe
        if pe == NULL:
            st["light without parent entity: property placement == world placement"] += close(matrix(pr.get("CEntityProperties.3")), matrix(asp[t].get("CAspect.7")))
        else:
            st["light with parent entity"] += 1; dist["parent entity classes of lights"][objs[pe].type] += 1
            st["light with parent: CEntityProperties.5 is the parent's property record"] += pr.get("CEntityProperties.5").oid == prop_id.get(id(props.get(pe)))
            st["light with parent: property placement == local placement"] += close(matrix(pr.get("CEntityProperties.3")), matrix(asp[t].get("CAspect.4")))
        dist["point light cone angles in degrees (CPointLightProperties.18)"][pr.get("CPointLightProperties.18") if e.type == "CPointLightEntity" else "-"] += 1
    st["light list (CRenderingDomain.3) == all light sources"] += set(ids(rd.get("CRenderingDomain.3"))) == sources
    lg = objs[rd.get("CRenderingDomain.5").oid]; lgl = set(ids(lg.get("CLightGrid.1")))
    st["light grid list is a subset of the light list"] += lgl <= sources; st["light grid list == light list"] += lgl == sources
    for i, o in objs.items():
        if isinstance(o, Obj) and o.type in ("CShadowBufferBaker",):
            dist["shadow buffer bakers owners"][objs[o.get("CAspect.5").oid].type if o.get("CAspect.5").oid in objs else "null"] += 1
    # baked lightmaps
    exposed = set(ids(rf.get("CResourceFile.rf_crsExposed"))); users = collections.Counter(); oven_main = {}
    for i, o in objs.items():
        if not (isinstance(o, Obj) and o.type == "CModelInstance" and o.get("CModelInstance.4").oid != NULL): continue
        ml = objs[o.get("CModelInstance.4").oid]
        mods = [allo[x.oid] for x in ml.get("CShaderModifierList.1") if x.oid in allo and allo[x.oid].type == "CTextureOvenShaderModifier"]
        if not mods: continue
        st["instances with oven modifiers"] += 1; st["... exactly one oven modifier"] += len(mods) == 1
        for x in ml.get("CShaderModifierList.1"):
            m = allo[x.oid]
            if m.type != "CTextureOvenShaderModifier": continue
            st["oven modifiers in the main section"] += x.oid in objs
            t = m.get("CTextureOvenShaderModifier.4").get("CShaderArgValTexture.1").oid; users[t] += 1
            tex = allo.get(t); bk = allo.get(m.get("CTextureOvenShaderModifier.6").oid)
            st["oven modifier -> baker record in the extra section"] += m.get("CTextureOvenShaderModifier.6").oid in extra and bk is not None and bk.type == "CTextureBakerLightMap"
            st["lightmap texture is a CStaticTexture of the main section"] += t in objs and tex.type == "CStaticTexture"
            st["lightmap texture is exposed (rf_crsExposed)"] += t in exposed
            if tex is not None and tex.type == "CStaticTexture":
                wd, ht = tex.get("CBaseTexture.1"), tex.get("CBaseTexture.2")
                st["lightmap is square, power of two"] += wd == ht and wd & (wd - 1) == 0
                dist["lightmap format code"][tex.get("CBaseTexture.6").v] += 1; dist["lightmap size"][wd] += 1
                dist["lightmap TexFlags"][hex(tex.get("CBaseTexture.4").v)] += 1
                dist["lightmap wrap (u, v)"][(hex(tex.get("CBaseTexture.9").v), hex(tex.get("CBaseTexture.10").v))] += 1
                amr = bk.get("CTextureBakerLightMap.9") if bk is not None else None
                if amr is None: st["bake records without member .9 (no ambient mask reference)"] += 1
                am = allo.get(amr.oid) if amr is not None else None
                st["baker -> ambient mask (extra), mask size == lightmap size / 2, bytes == w*h"] += (am is not None and am.type == "CAmbientMaskCache" and (am.get("CAmbientMaskCache.1"), am.get("CAmbientMaskCache.2")) == (wd // 2, ht // 2)
                                                                                                     and len(am.get("CAmbientMaskCache.3")) == (wd // 2) * (ht // 2))
            tg = (m.get("CTextureOvenShaderModifier.1"), m.get("CTextureOvenShaderModifier.2")); st["oven target ids are consecutive (.2 == .1 + 1)"] += tg[1] == tg[0] + 1
            dist["oven UV map id"][m.get("CTextureOvenShaderModifier.3").get("CShaderArgValUVMap.1")] += 1
            oven_main[x.oid] = m
    st["each lightmap texture used by exactly one instance"] += all(n == 1 for n in users.values()); st["lightmap textures"] += len(users)
    extra_ovens = {i: o for i, o in extra.items() if isinstance(o, Obj) and o.type == "CTextureOvenShaderModifier"}
    def refs(v, out):
        if isinstance(v, Ref): out.add(v.oid)
        elif isinstance(v, Obj):
            for _, x in v: refs(x, out)
        elif isinstance(v, list) and not hasattr(v, "decode"):
            for x in v: refs(x, out)
    holders = collections.Counter()
    for i, o in extra.items():
        if isinstance(o, Obj):
            found = set(); refs(o, found)
            for t in found & set(extra_ovens): holders[o.type] += 1
    for k, v in holders.items(): dist["records in the extra section that refer to an extra oven modifier"][k] += v
    st["oven modifiers in the extra section"] += len(extra_ovens); st["oven modifiers in the main section (per world)"] += len(oven_main)
    st["extra oven modifiers == main oven modifiers in number"] += len(extra_ovens) == len(oven_main)
    # flares
    for i, e in objs.items():
        if isinstance(e, Obj) and e.type == "SFlareEntity":
            st["flare entities"] += 1; fr = objs[e.get("SFlareEntity.1").oid]
            st["flare renderable is owned by its entity"] += fr.get("CAspect.5").oid == i
            pe = e.get("SFlareEntity.2").oid; dist["flare parent entity class"][objs[pe].type if pe in objs else "null"] += 1
            cf = fr.get("CFlareRenderable.1"); dist["flare configuration"]["embedded CFlareConfiguration" if cf.oid in objs else "external"] += 1
            pr = props[i]; l = pr.get("CEntityProperties.7").oid; dist["flare properties .7 links to"][extra[l].type if l in extra else "null"] += 1
            fpar = fr.get("CAspect.1").oid; dist["flare renderable's parent aspect"][objs[fpar].type if fpar in objs else "null"] += 1
            if l in extra:
                lent = extra[l].get("CEntityProperties.2").oid; le = objs[lent]
                ls = le.get("CPointLightEntity.2").oid if le.type == "CPointLightEntity" else None
                st["flare: next sibling aspect (CAspect.3) is the source of the light named in CEntityProperties.7"] += fr.get("CAspect.3").oid == ls
                st["flare: parent entity (.2) == the light's parent entity"] += e.get("SFlareEntity.2").oid == (le.get("CPointLightEntity.3").oid if le.type == "CPointLightEntity" else NULL)
    for cls in ("CBloomEntity", "CHazeEntity", "CCubeBackgroundEntity"):
        dist[f"{cls} per world"][sum(1 for o in objs.values() if isinstance(o, Obj) and o.type == cls)] += 1


def main():
    args = sys.argv[1:]
    install = args[0] if args and os.path.isdir(args[0]) else os.environ.get("SS2_DATA_DIR", "")
    limit = int(args[-1]) if args and args[-1].isdigit() else None
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder (the one containing All_PC_01.gro) or set SS2_DATA_DIR")
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter)
    for name, d in sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))[:limit]:
        check_world(name, d, st, dist); st["worlds"] += 1
    for k, v in sorted(st.items()): print(f"{v:9d}  {k}")
    for k, v in sorted(dist.items()): print(k, dict(v.most_common(8)))


if __name__ == "__main__":
    main()
