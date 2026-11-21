"""Re-runs the measurements behind docs/re/wld-root.md on an install (read-only).

For every format-8 `.wld` (smallest first): the `CWorld` root, the entity list and the property
records of the extra section, the domains, the placement hierarchy of aspects (renderables and
hulls), the grids, the state manager and the `Time` values.

Usage: python -I tools/re/wld_root_verify.py [DATA_DIR] [LIMIT]
       (DATA_DIR defaults to $SS2_DATA_DIR; LIMIT = only the LIMIT smallest worlds, default all, about 20 minutes)
"""
import collections, math, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Blob, Obj, Ref, TreeDec, merged_files, parse, u32  # noqa: E402
from walk_objects import chain  # noqa: E402

NULL = 0xFFFFFFFF


def ancestry(r, tid):
    return [c.name for c in chain(r, tid)]


def ids(refs): return [x.oid for x in refs if isinstance(x, Ref) and x.oid != NULL]


def matrix(placement):
    """(rotation 3x3 rows, translation) of a QuatVect value."""
    q = placement.get("QuatVect.1"); t = placement.get("QuatVect.2")
    x, y, z, w = (q.get(f"Quaternion4f.{i}") for i in (1, 2, 3, 4))
    rot = [[1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
           [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
           [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)]]
    return rot, [t.get(f"Vector3f.{i}") for i in (1, 2, 3)]


def compose(a, b):
    ra, ta = a; rb, tb = b
    rot = [[sum(ra[i][k] * rb[k][j] for k in range(3)) for j in range(3)] for i in range(3)]
    return rot, [sum(ra[i][k] * tb[k] for k in range(3)) + ta[i] for i in range(3)]


def close(a, b, eps=1e-3):
    return all(abs(a[0][i][j] - b[0][i][j]) < eps for i in range(3) for j in range(3)) and all(abs(a[1][i] - b[1][i]) < eps for i in range(3))


CLASS_CODES = collections.defaultdict(collections.Counter)      # EXOB flags of the entity module reference -> entity classes (over all worlds)


def check_world(name, d, st, dist, times):
    r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects(); extra, _ = dec.extra_objects(edob)
    allo = {**objs, **extra}; kinds = {oid: ancestry(r, tid) for oid, tid in r["obty"]}
    obty_t = dict(r["obty"])
    worlds = [i for i, o in objs.items() if isinstance(o, Obj) and o.type == "CWorld"]
    st["worlds"] += 1; st["exactly one CWorld"] += len(worlds) == 1
    w = objs[worlds[0]]; rf = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CResourceFile")
    shared = ids(rf.get("CResourceFile.rf_crsShared"))
    st["CWorld is in CResourceFile.rf_crsShared"] += worlds[0] in shared
    for x in shared: dist["rf_crsShared classes"][objs[x].type] += 1
    for x in ids(rf.get("CResourceFile.rf_crsExposed")): dist["exposed objects"][objs[x].type] += 1
    # entities
    ents = [i for i in objs if "CEntity" in kinds[i]]
    listed = ids(w.get("CWorld.1"))
    st["CWorld.1 lists exactly the CEntity objects"] += sorted(listed) == sorted(ents); st["entities"] += len(ents)
    st["entity list has no duplicates"] += len(listed) == len(set(listed))
    serials = [objs[i].get("CEntity.1") for i in listed if objs[i].get("CEntity.1") != 0xFFFFFFFF]
    st["serials (CEntity.1 != 0xFFFFFFFF) are unique"] += len(serials) == len(set(serials))
    st["CWorld.9 == highest serial"] += w.get("CWorld.9") == (max(serials) if serials else 0)
    st["CWorld.9 >= highest serial"] += w.get("CWorld.9") >= (max(serials) if serials else 0)
    props = ids(w.get("CWorld.5"))
    st["CWorld.5 are extra records"] += all(p in extra for p in props)
    targets = [extra[p].get("CEntityProperties.2").oid for p in props if p in extra]
    st["every properties record names an entity of the world"] += set(targets) <= set(listed) and len(targets) == len(set(targets))
    no_props = [i for i in listed if i not in set(targets)]
    for i in no_props: dist["entities without a properties record"][objs[i].type] += 1
    NO_SERIAL = 0xFFFFFFFF
    st["entities without properties record <-> serial 0xFFFFFFFF (CEntity.1)"] += {i for i in listed if objs[i].get("CEntity.1") == NO_SERIAL} == set(no_props)
    pairs = collections.Counter()
    for p in props:
        if p in extra:
            e = objs[extra[p].get("CEntityProperties.2").oid]; pairs[(e.type, extra[p].type)] += 1
    for (e, pr), n in pairs.items():
        st["properties class name == entity class name with 'Entity' -> 'Properties'"] += n * (e.replace("Entity", "Properties") == pr)
        if e.replace("Entity", "Properties") != pr: dist["entity/properties names that differ"][(e, pr)] += n
    st["entity properties records"] += len(props)
    infos = [i for i in ents if objs[i].type == "CWorldInfoEntity"]
    st["exactly one CWorldInfoEntity, = CWorld.7"] += infos == [w.get("CWorld.7").oid]
    st["CEntity.1 == CEntity.4 for every entity"] += all(objs[i].get("CEntity.1") == objs[i].get("CEntity.4") for i in listed)
    for i in ents:
        e = objs[i]; dist["CEntity.5/.6"][(e.get("CEntity.5"), e.get("CEntity.6"))] += 1
    # domains
    for k, cls in ((2, "CRenderingDomain"), (3, "CPhysicsDomain"), (4, "CCollisionDomain"), (8, "CScriptingDomain")):
        st[f"CWorld.{k} -> {cls}"] += objs[w.get(f"CWorld.{k}").oid].type == cls
    st["CWorld.wo_pmdMacro -> CMacroDomain"] += objs[w.get("CWorld.wo_pmdMacro").oid].type == "CMacroDomain"
    rd = objs[w.get("CWorld.2").oid]; cd = objs[w.get("CWorld.4").oid]
    renderables = sorted(i for i in objs if "CRenderable" in kinds[i]); hulls = sorted(i for i in objs if "CHull" in kinds[i])
    st["CRenderingDomain.1 lists exactly the CRenderable objects"] += sorted(ids(rd.get("CRenderingDomain.1"))) == renderables
    st["CRenderingDomain.8 equals .1"] += ids(rd.get("CRenderingDomain.8")) == ids(rd.get("CRenderingDomain.1"))
    st["CCollisionDomain.1 lists exactly the CHull objects"] += sorted(ids(cd.get("CCollisionDomain.1"))) == hulls
    r1, r8 = ids(rd.get("CRenderingDomain.1")), ids(rd.get("CRenderingDomain.8"))
    st["CRenderingDomain.8 is a subset of .1"] += set(r8) <= set(r1)
    for i in set(r1) - set(r8): dist["renderables in .1 but not in .8"][objs[i].type] += 1
    for i in ids(rd.get("CRenderingDomain.3")): dist["CRenderingDomain.3 (dynamic lights?) classes"][objs[i].type] += 1
    for i in ids(rd.get("CRenderingDomain.9")): dist["CRenderingDomain.9 classes"][objs[i].type] += 1
    st["CRenderingDomain.5 -> CLightGrid"] += objs[rd.get("CRenderingDomain.5").oid].type == "CLightGrid"
    st["CRenderingDomain.6 and .7 -> two different CRenderableGrid"] += (objs[rd.get("CRenderingDomain.6").oid].type, objs[rd.get("CRenderingDomain.7").oid].type) == ("CRenderableGrid",) * 2 and rd.get("CRenderingDomain.6").oid != rd.get("CRenderingDomain.7").oid
    pd = objs[w.get("CWorld.3").oid]; p1, p2 = ids(pd.get("CPhysicsDomain.1")), ids(pd.get("CPhysicsDomain.2"))
    st["CPhysicsDomain.1 and .2 hold the same bodies"] += sorted(p1) == sorted(p2)
    if sorted(p1) != sorted(p2): dist["PD.1 != PD.2 (world: len .1, len .2, common)"][(name.rsplit('/', 1)[-1], len(p1), len(p2), len(set(p1) & set(p2)))] += 1
    st["CPhysicsDomain.1 is a subset of .2 or the reverse"] += set(p1) <= set(p2) or set(p2) <= set(p1)
    for i in p1: dist["CPhysicsDomain.1 classes"][objs[i].type] += 1
    c1, c3 = ids(cd.get("CCollisionDomain.1")), ids(cd.get("CCollisionDomain.3"))
    st["CCollisionDomain.3 is a subset of .1"] += set(c3) <= set(c1)
    for i in c3: dist["CCollisionDomain.3 classes"][objs[i].type] += 1
    sd = objs[w.get("CWorld.8").oid]; steppers = sd.get("CScriptingDomain.2")
    dist["CScriptingDomain.3 vs number of steppers"][(sd.get("CScriptingDomain.3"), len(steppers))] += 1
    st["stepper entities are entities of the world"] += all(x[0][1].oid in set(listed) for x in steppers if x) if steppers else 1
    st["CWorld.18 is null"] += w.get("CWorld.18").oid == NULL
    for k in (9, 10, 11, 16, 17, 19, 6): dist[f"CWorld.{k}"][repr(w.get(f"CWorld.{k}")) if not isinstance(w.get(f"CWorld.{k}"), (list, Obj, Ref)) else type(w.get(f"CWorld.{k}")).__name__] += 1
    g = w.get("CWorld.10"); dist["gravity-like CWorld.10"][tuple(round(x[1], 3) for x in g)] += 1
    # entity class codes (EXOB flags of CEntity.2), base members
    rfil = {e[0]: e[2] for e in r["rfil"]}; exm = {e[0]: (rfil[e[1]], e[2]) for e in r["exob"]}
    for i in listed:
        e = objs[i]; m = e.get("CEntity.2")
        st["entity module reference is external, Bin/Sam2Game.module"] += m is not None and m.oid in exm and exm[m.oid][0] == "Bin/Sam2Game.module"
        if m is not None and m.oid in exm: CLASS_CODES[exm[m.oid][1]][e.type] += 1
        dist["CBaseEntity.1 flags (class, value)"][(e.type, hex(e.get("CBaseEntity.1").v))] += e.get("CBaseEntity.1").v != 0
        if e.get("SBaseEntity.1") is None: dist["entity classes without SBaseEntity.1"][e.type] += 1
    ridx = [objs[i].get("CRenderable.1") for i in ids(rd.get("CRenderingDomain.1"))]
    st["CRenderable.1 == position in CRenderingDomain.1"] += ridx == list(range(len(ridx)))
    dist["len(CWorld.6)"][len(w.get("CWorld.6"))] += 1
    # aspects: placement hierarchy
    aspects = [i for i in objs if "CAspect" in kinds[i]]
    for i in aspects:
        a = objs[i]; owner = a.get("CAspect.5").oid; st["aspects"] += 1
        st["aspect owner (CAspect.5) is an entity of the world or null"] += owner in set(listed) or owner == NULL
        st["... of which null"] += owner == NULL
        par = a.get("CAspect.1").oid; loc = matrix(a.get("CAspect.4")); wor = matrix(a.get("CAspect.7"))
        if par == NULL:
            st["root aspects"] += 1; st["root aspect: world placement (.7) == local (.4)"] += close(loc, wor)
        else:
            st["child aspects"] += 1; p = objs[par]
            st["child aspect: .7 == parent .7 x local .4"] += close(compose(matrix(p.get("CAspect.7")), loc), wor)
            st["parent's child link (.2) or sibling chain leads to the child"] += i in chain_ids(objs, par)
        dist["CAspect.6 flags"][hex(a.get("CAspect.6").v)] += 1
    # grids: pools of nodes / links / entries with free lists
    def pool_ok(items, free):
        return len(set(free)) == len(free) and all(0 <= f < len(items) for f in free)
    hull_set = set(hulls)
    for i, o in objs.items():
        if not isinstance(o, Obj): continue
        if "CGenericGrid2D" in kinds[i]:
            nodes, free_n, links, free_l, entries, free_e = (o.get(f"CGenericGrid2D.{k}") for k in (1, 2, 3, 4, 5, 6))
            st["2D grids"] += 1
            st["grid node child ids are -1 or valid node indices"] += all(-1 <= c < len(nodes) for node in nodes for c in node.get("CGridNode2D.1")) and all(-1 <= node.get("CGridNode2D.2") < len(links) for node in nodes)
            st["grid free lists (.2 nodes, .4 links, .6 entries) are unique valid indices"] += pool_ok(nodes, free_n) and pool_ok(links, free_l) and pool_ok(entries, free_e)
            st["grid extent (.8) == CWorld.17"] += o.get("CGenericGrid2D.8") == w.get("CWorld.17")
            dist["grid depth (.7) minus CWorld.16"][o.get("CGenericGrid2D.7") - w.get("CWorld.16")] += 1
            dist["grid (class, depth, extent)"][(o.type, o.get("CGenericGrid2D.7"), o.get("CGenericGrid2D.8"))] += 1
            members = ids(o.get("CRenderableGrid.1")) if o.type == "CRenderableGrid" else ids(o.get("CLightGrid.1")) if o.type == "CLightGrid" else []
            st["2D grid: used entries (pool - free) == number of objects in its own list"] += len(entries) - len(free_e) == len(members)
            if o.type == "CRenderableGrid": st["renderable grid: its own list holds only renderables of the world"] += all(m in set(renderables) for m in members)
            else: st["light grid: its own list holds only light sources (class name ends in LightSource)"] += all(objs[m].type.endswith("LightSource") for m in members)
            dist["2D grid own-list classes"][(o.type, tuple(sorted({objs[m].type for m in members})[:4]))] += 1
        if o.type == "CCollisionGrid":
            nodes, free_n, links, free_l = (o.get(f"CCollisionGrid.{k}") for k in (1, 2, 3, 4)); st["collision grids"] += 1
            st["collision grid node child ids are -1 or valid node indices"] += all(-1 <= c < len(nodes) for node in nodes for c in node.get("CGridNode.1")) and all(-1 <= node.get("CGridNode.2") < len(links) for node in nodes)
            st["collision grid free lists (.2 nodes, .4 links) are unique valid indices"] += pool_ok(nodes, free_n) and pool_ok(links, free_l)
            used = [k for k in range(len(links)) if k not in set(free_l)]
            targets = {links[k].get("CGridLink.1").oid for k in used}
            st["collision grid: used links point at hulls of the world (or are null)"] += all(t == NULL or t in hull_set for t in targets)
            st["collision grid: every hull appears in at least one used link"] += {t for t in targets if t != NULL} == hull_set
            st["collision grid extent (.6) == CWorld.17"] += o.get("CCollisionGrid.6") == w.get("CWorld.17")
            dist["collision grid depth (.5) minus CWorld.16"][o.get("CCollisionGrid.5") - w.get("CWorld.16")] += 1
    # state manager and Time values
    sms = [i for i, o in objs.items() if isinstance(o, Obj) and o.type == "CStateManager"]
    st["exactly one CStateManager, = CScriptingDomain.4"] += sms == [objs[w.get("CWorld.8").oid].get("CScriptingDomain.4").oid]
    sm = objs[sms[0]]
    lst = sm.get("CStateManager.10")
    st["CStateManager.10 is a CEntryList"] += lst.type == "CEntryList"
    entries = lst.get("CEntryList.1")
    st["state manager entries refer to an entity of the world (or null)"] += all(x.get("CSMEntry.1").oid in set(listed) or x.get("CSMEntry.1").oid == NULL for x in entries)
    dist["state manager entries (bucketed)"][min(len(entries) // 50, 10)] += 1
    for k in (3, 4, 7):
        dist[f"CSoundSyncList.{k} length"][len(objs[w.get('CWorld.12').oid].get(f'CSoundSyncList.{k}'))] += 1
    walk_times(objs, times); walk_times(extra, times)


def chain_ids(objs, parent):
    """All aspects reachable from `parent` through member .2 (first child) and .3 (next sibling)."""
    out = []; seen = set(); stack = [objs[parent].get("CAspect.2").oid]
    while stack:
        c = stack.pop()
        if c == NULL or c in seen: continue
        seen.add(c); out.append(c); a = objs[c]; stack.append(a.get("CAspect.3").oid)
    return out


def walk_times(objs, times):
    def visit(v):
        if isinstance(v, Obj):
            if v.type == "Time": times.append(v.get("Time.1"))
            else:
                for _, x in v: visit(x)
        elif isinstance(v, list) and not isinstance(v, Blob):
            for x in v: visit(x)
    for o in objs.values(): visit(o)


def main():
    args = [a for a in sys.argv[1:]]
    install = args[0] if args and os.path.isdir(args[0]) else os.environ.get("SS2_DATA_DIR", "")
    limit = int(args[-1]) if args and args[-1].isdigit() else None
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder (the one containing All_PC_01.gro) or set SS2_DATA_DIR")
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter); times = []
    for name, d in sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))[:limit]:
        check_world(name, d, st, dist, times)
    st.pop("CRenderingDomain.8 equals .1", None); st.pop("CRenderable.1 == position in CRenderingDomain.1", None)
    for k, v in sorted(st.items()): print(f"{v:9d}  {k}")
    codes = {f: dict(c) for f, c in sorted(CLASS_CODES.items())}
    print(f"entity class codes: {len(codes)} distinct codes; codes shared by several classes: {sum(1 for c in codes.values() if len(c) > 1)}; classes with several codes: "
          f"{sum(1 for cl in {k for c in codes.values() for k in c} if sum(1 for c in codes.values() if cl in c) > 1)}")
    print("class code table (first 24):", {f: list(c)[0] for f, c in list(codes.items())[:24]})
    for k, v in sorted(dist.items()): print(k, dict(v.most_common(8)))
    if times:
        whole = sum(1 for t in times if t % (1 << 32) == 0)
        print(f"Time.1 values: {len(times)}; whole multiples of 2^32: {whole}; distinct {len(set(times))}; most common:", collections.Counter(times).most_common(5))


if __name__ == "__main__":
    main()
