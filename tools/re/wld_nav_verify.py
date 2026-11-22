"""Re-runs the measurements behind docs/re/wld-navigation.md on an install (read-only).

For every format-8 `.wld`: the navigation mesh (nodes, links, link chains, the navigation grid), the path markers
(next/previous handles, chains), formation markers and formations, tactics, navigation markers and beacons.

Usage: python -I tools/re/wld_nav_verify.py [DATA_DIR] [LIMIT]
       (DATA_DIR defaults to $SS2_DATA_DIR; LIMIT = only the LIMIT smallest worlds, default all, about 15 minutes)
"""
import collections, math, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Blob, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
from wld_root_verify import ids  # noqa: E402

NULL = 0xFFFFFFFF


def named(seq): return [{k.split(".")[-1]: v for k, v in x} for x in seq]


def cells(L, lo, hi, extent):
    size = extent / (1 << L)
    return int(math.floor((lo + extent / 2) / size)), int(math.floor((hi + extent / 2) / size))


def check_mesh(m, st, dist):
    nodes = named(m.get("CNavMesh.26")); ls = named(m.get("CNavMesh.28")); lk = named(m.get("CNavMesh.29")); grid = m.get("CNavMesh.30")
    st["nav meshes"] += 1; st["... non-empty"] += bool(nodes); st["nodes"] += len(nodes); st["links"] += len(lk)
    dist["nodes per mesh"][min(len(nodes) // 500, 8) * 500] += 1
    if not nodes: return
    st["link, link-search and link lists have the same length"] += len(ls) == len(lk)
    rev = [l["9"] for l in lk]
    st["links: reverse link (.9) is a link and is its own reverse"] += all(0 <= rev[j] < len(lk) and rev[rev[j]] == j for j in range(len(lk)))
    st["links: reverse link has the opposite direction ((.4 + 2) mod 4)"] += all(lk[rev[j]]["4"] == (lk[j]["4"] + 2) % 4 for j in range(len(lk)))
    target = [ls[j]["1"] for j in range(len(lk))]; owner = [target[rev[j]] for j in range(len(lk))]
    st["links: target node (CNMLinkSearch.1) is a node"] += all(0 <= t < len(nodes) for t in target)
    # adjacency geometry: dir 0 = +z, 1 = +x, 2 = -z, 3 = -x
    ok = 0
    for j, l in enumerate(lk):
        A = nodes[owner[j]]; B = nodes[target[j]]; d = l["4"]
        ax0, ax1, az0, az1 = A["5"], A["5"] + A["7"], A["6"], A["6"] + A["8"]; bx0, bx1, bz0, bz1 = B["5"], B["5"] + B["7"], B["6"], B["6"] + B["8"]
        if d == 0: ok += az1 == bz0 and max(ax0, bx0) < min(ax1, bx1)
        elif d == 1: ok += ax1 == bx0 and max(az0, bz0) < min(az1, bz1)
        elif d == 2: ok += az0 == bz1 and max(ax0, bx0) < min(ax1, bx1)
        elif d == 3: ok += ax0 == bx1 and max(az0, bz0) < min(az1, bz1)
    st["links: the owner and target rectangles touch on the side of the direction (0 +z, 1 +x, 2 -z, 3 -x), overlapping"] += ok
    # link point: the cell (.5, .7) and length (.8) along the edge; the point (.10) is the centre of the segment
    okc = oky = 0
    for j, l in enumerate(lk):
        c = l["10"]; cx, cy, cz = [c.get(f"Vector3f.{q}") for q in (1, 2, 3)]; d = l["4"]; n = l["8"]
        want = {0: ("x", l["5"] + n / 2), 1: ("z", l["7"] + 1 - n / 2), 2: ("x", l["5"] + 1 - n / 2), 3: ("z", l["7"] + n / 2)}[d]
        okc += abs((cx if want[0] == "x" else cz) - want[1]) < 1e-3
        oky += abs(l["6"] - cy) < 0.02
    st["links: along-edge centre of the point (.10): dir 0 .5 + n/2 (x), dir 1 .7 + 1 - n/2 (z), dir 2 .5 + 1 - n/2 (x), dir 3 .7 + n/2 (z), n = .8"] += okc
    st["links: .6 equals the point's height"] += oky
    # chains
    mine = collections.defaultdict(list)
    for j in range(len(lk)): mine[owner[j]].append(j)
    first_ok = chain_ok = 0
    for i, nd in enumerate(nodes):
        f = nd["1"]; got = []; cur = f; seen = set()
        while cur != -1 and cur not in seen and 0 <= cur < len(lk): got.append(cur); seen.add(cur); cur = ls[cur]["2"]
        first_ok += (f == -1 and i not in mine) or (f != -1 and owner[f] == i)
        chain_ok += sorted(got) == sorted(mine.get(i, []))
    st["nodes: first link (.1) is owned by the node (or -1 without links)"] += first_ok
    st["nodes: the chain through CNMLinkSearch.2 lists exactly the node's links"] += chain_ok
    st["nodes with no links"] += sum(1 for i in range(len(nodes)) if i not in mine)
    st["nodes: .2 equals the node's own index"] += sum(1 for i, nd in enumerate(nodes) if nd["2"] == i)
    st["nodes: .9 is a node index or -1"] += sum(1 for nd in nodes if -1 <= nd["9"] < len(nodes))
    for nd in nodes: dist["node .10 (low 6 bits)"][nd["10"]] += 0
    for nd in nodes: dist["node .10"][nd["10"]] += 1
    # grid: entry e belongs to node CNavigationGrid.4[e]; the entry's cells contain the node's rectangle
    ents = [[v for _, v in e] for e in grid.get("CGenericGrid2D.5")]; fe = set(grid.get("CGenericGrid2D.6")); used = [i for i in range(len(ents)) if i not in fe]
    depth = grid.get("CGenericGrid2D.7"); extent = grid.get("CGenericGrid2D.8"); lst = list(grid.get("CNavigationGrid.4"))
    st["grid: used entries == nodes"] += len(used) == len(nodes)
    inside = lvl = lvl1 = mapped = 0; seen = collections.Counter()
    for e in used:
        nid = lst[e] if e < len(lst) else -1
        if not (0 <= nid < len(nodes)): continue
        mapped += 1; seen[nid] += 1; nd = nodes[nid]; ent = ents[e]; L = ent[4]; sz = extent / (1 << L)
        rx0 = ent[0] * sz - extent / 2; rx1 = (ent[1] + 1) * sz - extent / 2; rz0 = ent[2] * sz - extent / 2; rz1 = (ent[3] + 1) * sz - extent / 2
        inside += rx0 <= nd["5"] and nd["5"] + nd["7"] <= rx1 and rz0 <= nd["6"] and nd["6"] + nd["8"] <= rz1
        side = max(nd["7"], nd["8"]); lvl += max(LL for LL in range(depth + 1) if extent / (1 << LL) >= side) == L
        lvl1 += max(LL for LL in range(depth + 1) if extent / (1 << LL) >= side + 1) == L
        dist["entry level minus the 'cell >= larger side' level"][L - max(LL for LL in range(depth + 1) if extent / (1 << LL) >= side)] += 1
        st["grid: entries covering at most 2 x 2 cells"] += ent[1] - ent[0] < 2 and ent[3] - ent[2] < 2
    st["grid: used entries whose CNavigationGrid.4 value is a node"] += mapped
    st["grid: every node is named by exactly one used entry"] += all(seen[i] == 1 for i in range(len(nodes)))
    st["grid: the entry's cells contain the node's rectangle (x .5 .. .5 + .7, z .6 .. .6 + .8)"] += inside
    st["grid: entry level == finest level whose cell is at least the larger side of the node"] += lvl
    st["grid: entry level == finest level whose cell is at least the larger side plus 1"] += lvl1
    dist["grid depth / extent"][(depth, extent)] += 1


def check_world(name, d, st, dist):
    r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects()
    try: extra, _ = dec.extra_objects(edob)
    except Exception: extra = {}
    allo = {**objs, **extra}; w = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CWorld"); ents = ids(w.get("CWorld.1"))
    recs = {extra[p].get("CEntityProperties.2").oid: extra[p] for p in ids(w.get("CWorld.5")) if p in extra}
    st["worlds"] += 1
    meshes = {i: o for i, o in objs.items() if isinstance(o, Obj) and o.type == "CNavMesh"}
    st["worlds with a nav mesh object"] += bool(meshes)
    for m in meshes.values(): check_mesh(m, st, dist)
    for o in objs.values():
        if isinstance(o, Obj) and o.type == "CNavMeshRenderable": st["nav mesh renderables"] += 1; st["... pointing at a nav mesh of the world"] += o.get("CNavMeshRenderable.2").oid in meshes
    wi = [objs[i] for i in ents if objs[i].type == "CWorldInfoEntity"][0]
    # the path finding service is a nested struct of the world info entity
    pf = [v for k, v in wi if isinstance(v, Obj) and v.type == "CPathFindingService"]
    st["world info entities with a CPathFindingService"] += bool(pf)
    for p in pf:
        refs = [v for k, v in p if isinstance(v, Ref) and v.oid in meshes]
        st["... whose pointer is the nav mesh of the world"] += bool(refs)
    # --- path markers
    pm = {i: objs[i] for i in ents if objs[i].type == "SPathMarkerEntity"}
    st["path markers"] += len(pm)
    nxt = {i: o.get("SPathMarkerEntity.2").oid for i, o in pm.items()}; prv = {i: o.get("SPathMarkerEntity.4").oid for i, o in pm.items()}
    st["path markers with a next marker (.2)"] += sum(1 for v in nxt.values() if v != NULL); st["path markers with a handle .4"] += sum(1 for v in prv.values() if v != NULL)
    st["... .2 targets a path marker of the world"] += sum(1 for v in nxt.values() if v in pm); st["... .4 targets a path marker of the world"] += sum(1 for v in prv.values() if v in pm)
    indeg = collections.Counter(v for v in nxt.values() if v in pm)
    st["markers that are the next of two or more markers (.2 in-degree > 1)"] += sum(1 for v in indeg.values() if v > 1)
    st["path markers: .4 target's .2 points back (A.4 = B  and  B.2 = A)"] += sum(1 for i, v in prv.items() if v in pm and nxt[v] == i)
    st["path markers: .2 target's .4 points back (A.2 = B  and  B.4 = A)"] += sum(1 for i, v in nxt.items() if v in pm and prv[v] == i)
    st["path markers: .2 target equals the target of the record's handle (SPathMarkerProperties.1)"] += sum(1 for i, o in pm.items() if i in recs and (nxt[i] != NULL) == (recs[i].get("SPathMarkerProperties.1").oid != NULL))
    # chains: follow .2 from markers that are nobody's next
    heads = [i for i in pm if i not in indeg]; lens = []
    inchain = set()
    for h in heads:
        n = 0; cur = h; seen = set()
        while cur in pm and cur not in seen: seen.add(cur); n += 1; cur = nxt[cur]
        lens.append(n); inchain |= seen
    st["path chains (from markers that are nobody's next)"] += len(heads); st["... markers on those chains"] += len(inchain)
    st["markers on cycles (not reachable from a chain head)"] += len(pm) - len(inchain)
    for n in lens: dist["path chain length"][min(n, 21) if n < 21 else "21+"] += 1
    # --- tactics
    tac = [objs[i] for i in ents if objs[i].type == "STacticEntity"]
    st["tactic entities"] += len(tac)
    st["... with a marker (en_henTacticMarker)"] += sum(1 for t in tac if t.get("STacticEntity.en_henTacticMarker").oid != NULL)
    st["... whose marker is a path marker of the world"] += sum(1 for t in tac if t.get("STacticEntity.en_henTacticMarker").oid in pm)
    mover = collections.Counter()
    for t in tac:
        m = t.get("STacticEntity.en_hmcCharacterMover")
        if isinstance(m, Ref) and m.oid in allo and isinstance(allo[m.oid], Obj): mover[allo[m.oid].type] += 1
    for k, v in mover.items(): dist["tactic mover class"][k] += v
    # --- formations
    fm = [objs[i] for i in ents if objs[i].type == "CFormationMarkerEntity"]
    st["formation marker entities"] += len(fm)
    st["... with their own formation (.2)"] += sum(1 for o in fm if o.get("CFormationMarkerEntity.2").oid != NULL)
    for o in fm:
        f = o.get("CFormationMarkerEntity.2").oid
        if f != NULL and f in allo and isinstance(allo[f], Obj): dist["formation class (marker .2)"][allo[f].type] += 1
    for t in ("CMarkersFormation", "CLineFormation", "CRectFormation", "CPieSliceFormation", "CVFormation"):
        st[f"formation objects: {t}"] += sum(1 for o in allo.values() if isinstance(o, Obj) and o.type == t)
    okw = totw = 0; fmset = {i for i in ents if objs[i].type == "CFormationMarkerEntity"}
    for o in allo.values():
        if isinstance(o, Obj) and o.type == "CMarkersFormation":
            for k, v in o:
                if isinstance(v, Obj) and v.type == "CWeightedFormationMarkers":
                    for item in v.get("CWeightedFormationMarkers.1"):
                        e = item.get("CWeightedFormationMarker.1"); p = item.get("CWeightedFormationMarker.2")
                        if e.oid == NULL: continue
                        totw += 1; okw += e.oid in fmset and p.oid in recs.values() or (e.oid in fmset and recs.get(e.oid) is not None and extra and p.oid == next((q for q, rr in extra.items() if rr is recs[e.oid]), None))
    st["weighted formation markers naming a marker entity"] += totw; st["... whose record handle is that entity's record"] += okw
    st["navigation markers (CNavMeshMarkerEntity)"] += sum(1 for i in ents if objs[i].type == "CNavMeshMarkerEntity")
    st["navigational beacons"] += sum(1 for i in ents if objs[i].type == "SNavigationalBeaconEntity")
    # movers anywhere
    for o in allo.values():
        if isinstance(o, Obj) and o.type.startswith("CMC"): dist["mover (CMC...) objects"][o.type] += 1


def main():
    install = sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].isdigit() else os.environ.get("SS2_DATA_DIR", "")
    limit = int(sys.argv[-1]) if sys.argv[-1].isdigit() else None
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter)
    files = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))
    for n, (name, d) in enumerate(files):
        if limit and n >= limit: break
        check_world(name, d, st, dist); print(f"[{n + 1}/{len(files)}] {name.rsplit('/', 1)[-1]}", file=sys.stderr, flush=True)
    for k, v in sorted(st.items()):
        if v: print(f"{v:9d}  {k}")
    for k, v in sorted(dist.items()):
        v = collections.Counter({a: b for a, b in v.items() if b})
        if v: print(k, dict(v.most_common(16)))


if __name__ == "__main__":
    main()
