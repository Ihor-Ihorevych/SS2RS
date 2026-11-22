"""Re-runs the measurements behind docs/re/wld-geometry.md on an install (read-only).

For every format-8 `.wld` (smallest first): how a world places models (`CStaticModelEntity`,
`CModelRenderable`, `CModelInstance`), the collision hulls and their cell rectangles in the
collision grid, the mechanisms, the entity parent links and placement rules, and which entity
classes own which geometry classes.

Usage: python -I tools/re/wld_geometry_verify.py [DATA_DIR] [LIMIT]
       (DATA_DIR defaults to $SS2_DATA_DIR; LIMIT = only the LIMIT smallest worlds, default all, about 20 minutes)
"""
import collections, math, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
from wld_root_verify import ancestry, close, ids, matrix  # noqa: E402

NULL = 0xFFFFFFFF
GEOMETRY = ("CModelRenderable", "CEditorModelRenderable", "CEditorPrimitiveRenderable", "CModelHull", "CPrimitiveHull",
            "CEffectRenderable", "CCrumbsRenderable", "CCubeRenderable", "CFlareRenderable", "CVisBoxRenderable")


def check_world(name, d, mdls, st, dist):
    r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects(); extra, _ = dec.extra_objects(edob)
    kinds = {oid: ancestry(r, tid) for oid, tid in r["obty"]}
    rf = {e[0]: e[2] for e in r["rfil"]}; exm = {e[0]: rf[e[1]] for e in r["exob"]}
    w = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CWorld"); ext = w.get("CWorld.17")
    props = {p.get("CEntityProperties.2").oid: p for p in extra.values() if isinstance(p, Obj) and p.get("CEntityProperties.2") is not None}
    prop_id = {id(p): k for k, p in extra.items()}
    asp = {i: o for i, o in objs.items() if isinstance(o, Obj) and "CAspect" in kinds[i]}
    hulls = {i: o for i, o in asp.items() if "CHull" in kinds[i]}
    owned = collections.defaultdict(list)
    for i, o in asp.items(): owned[o.get("CAspect.5").oid].append(i)

    def model_of(inst):
        m = inst.get("CModelInstance.3")
        return exm.get(m.oid) if m.oid in exm else ("embedded" if m.oid in objs else None)

    def top(i):
        own = asp[i].get("CAspect.5").oid
        while True:
            p = asp[i].get("CAspect.1").oid
            if p == NULL or asp[p].get("CAspect.5").oid != own: return i
            i = p

    # who owns which geometry class
    for i, o in asp.items():
        if o.type in GEOMETRY:
            ow = o.get("CAspect.5").oid; dist[f"owners of {o.type}"][objs[ow].type if ow in objs else "null"] += 1
    # static model entities
    cg = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CCollisionGrid"); free = set(cg.get("CCollisionGrid.4")); links = cg.get("CCollisionGrid.3")
    used = collections.Counter(links[k].get("CGridLink.1").oid for k in range(len(links)) if k not in free)
    for i, e in objs.items():
        if not (isinstance(e, Obj) and e.type == "CStaticModelEntity"): continue
        st["static model entities"] += 1
        R = objs[e.get("CStaticModelEntity.1").oid]; inst = objs[R.get("CModelRenderable.1").oid]
        st["renderable's owner is the entity"] += R.get("CAspect.5").oid == i
        em = e.get("CStaticModelEntity.6"); emp = exm.get(em.oid) if em.oid in exm else ("embedded" if em.oid in objs else None)
        mp = model_of(inst)
        st["entity model (.6) == renderable's instance model"] += mp == emp
        if mp and mp != "embedded":
            st["external model references"] += 1; st["... resolve to a .mdl of the install"] += mp.lower() in mdls
            dist["distinct external models"][mp.lower()] += 1
        else: st["embedded model configurations"] += 1
        sc = tuple(round(inst.get("CModelInstance.1").get(f"Vector3f.{k}"), 4) for k in (1, 2, 3))
        dist["instance scale kind"]["identity" if sc == (1, 1, 1) else "uniform" if sc[0] == sc[1] == sc[2] and sc[0] > 0 else "has a negative axis" if min(sc) < 0 else "non-uniform"] += 1
        st["instance has a shader modifier list (CModelInstance.4)"] += inst.get("CModelInstance.4").oid != NULL
        mo = e.get("CStaticModelEntity.10").oid
        own_asp = owned[i]; own_hulls = [a for a in own_asp if a in hulls]
        st["single aspect subtree (one top)"] += len({top(a) for a in own_asp}) == 1
        if mo != NULL:
            M = objs[mo]; st["entities with a mechanism"] += 1
            st["mechanism's renderable (CMechanism.6) is the entity's renderable"] += M.get("CMechanism.6").oid == e.get("CStaticModelEntity.1").oid
            st["entity has a mechanism <-> it has at least one hull"] += bool(own_hulls)
            st["mechanism parts (CMechanism.1) == number of hulls"] += len(M.get("CMechanism.1")) == len(own_hulls)
            st["hulls of entities with a mechanism"] += len(own_hulls)
            for h in own_hulls: st["hull's mechanism (CHull.9) is the entity's mechanism"] += hulls[h].get("CHull.9").oid == mo
        else: st["entity without mechanism has no hull"] += not own_hulls
        pe = e.get("CStaticModelEntity.21").oid; pr = props.get(i)
        t = top(e.get("CStaticModelEntity.1").oid)
        if pe == NULL:
            st["no parent entity: top aspect is a root"] += asp[t].get("CAspect.1").oid == NULL
            st["no parent entity: property placement == top world placement"] += pr is not None and close(matrix(pr.get("CEntityProperties.3")), matrix(asp[t].get("CAspect.7")))
        else:
            st["parent entities (.21 set)"] += 1
            st["parent entity: CEntityProperties.5 is the parent's property record"] += pr is not None and pe in props and pr.get("CEntityProperties.5").oid == prop_id.get(id(props[pe]))
            par = asp[t].get("CAspect.1").oid
            st["parent entity: top's parent aspect belongs to the parent entity"] += par != NULL and asp[par].get("CAspect.5").oid == pe
            st["parent entity: property placement == top local placement"] += pr is not None and close(matrix(pr.get("CEntityProperties.3")), matrix(asp[t].get("CAspect.4")))
        dist["hulls per static model entity"][min(len(own_hulls), 3)] += 1
        st["entity .11 (collision flag) == 1 exactly when the entity has a hull"] += (e.get("CStaticModelEntity.11") == 1) == bool(own_hulls)
        if pr is not None:
            st["entity .11 == properties .4, entity .12 == properties .12, entity .20 == properties .8"] += (
                e.get("CStaticModelEntity.11") == pr.get("CStaticModelProperties.4") and e.get("CStaticModelEntity.12") == pr.get("CStaticModelProperties.12")
                and e.get("CStaticModelEntity.20") == pr.get("CStaticModelProperties.8"))
            st["properties record has a model reference (.1) equal to the entity's (.6)"] += (exm.get(pr.get("CStaticModelProperties.1").oid) == exm.get(em.oid)) if em.oid in exm else 1
            dist["CStaticModelProperties.9 shadow type"][pr.get("CStaticModelProperties.9").v] += 1
            dist["CStaticModelEntity.12 values"][e.get("CStaticModelEntity.12")] += 1
    # model hulls
    for h, o in hulls.items():
        st["hulls"] += 1
        xa, xb, ya, yb = (o.get(f"CHull.{k}") for k in (3, 4, 5, 6)); lvl = o.get("CHull.7")
        n = (xb - xa + 1) * (yb - ya + 1) if xb >= xa and yb >= ya else -1
        st["hull cell rectangle (CHull.3..6 = xmin, xmax, ymin, ymax) is well-formed"] += n > 0
        st["hull: used collision-grid links == rectangle cells"] += used.get(h, 0) == n
        st["hull: level (CHull.7) <= collision grid depth"] += lvl <= cg.get("CCollisionGrid.5")
        _, t = matrix(o.get("CAspect.7")); size = ext / (2 ** lvl)
        cx, cz = math.floor((t[0] + ext / 2) / size), math.floor((t[2] + ext / 2) / size)
        st["hull: position (x, z) cell lies inside the rectangle"] += xa <= cx <= xb and ya <= cz <= yb
        b = o.get("CHull.1").oid; dist["body class behind hulls"][(o.type, objs[b].type if b in objs else "null")] += 1
        if o.type == "CPrimitiveHull":
            pd = o.get("CPrimitiveHull.1"); dist["primitive hull descriptor kinds (PrimitiveType)"][pd.get("CPrimitiveDesc.1").v] += 1
            dist["primitive hull of type 0: (second, third) parameters"][(round(pd.get("CPrimitiveDesc.3"), 3), round(pd.get("CPrimitiveDesc.4"), 3)) if pd.get("CPrimitiveDesc.1").v == 0 else "n/a"] += 1
        if o.type == "CModelHull":
            ow = o.get("CAspect.5").oid
            if ow in objs and objs[ow].type == "CStaticModelEntity":
                st["model hull: its model instance names the entity's model"] += model_of(objs[o.get("CModelHull.1").oid]) == exm.get(objs[ow].get("CStaticModelEntity.6").oid, "embedded")
                st["model hulls of static model entities"] += 1
    for i, o in asp.items():
        if o.type == "CEditorPrimitiveRenderable":
            dist["editor primitive types"][tuple(x for x in [o.get("CPrimitiveRenderable.1").get("CPrimitiveDesc.1").v])] += 1


def main():
    args = sys.argv[1:]
    install = args[0] if args and os.path.isdir(args[0]) else os.environ.get("SS2_DATA_DIR", "")
    limit = int(args[-1]) if args and args[-1].isdigit() else None
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder (the one containing All_PC_01.gro) or set SS2_DATA_DIR")
    mdls = merged_files(install, ".mdl"); st = collections.Counter(); dist = collections.defaultdict(collections.Counter)
    for name, d in sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))[:limit]:
        check_world(name, d, mdls, st, dist); st["worlds"] += 1
    for k, v in sorted(st.items()): print(f"{v:9d}  {k}")
    for k, v in sorted(dist.items()):
        print(k, f"({len(v)} distinct)" if k == "distinct external models" else dict(v.most_common(8)))


if __name__ == "__main__":
    main()
