"""Re-runs the measurements behind docs/re/wld-visibility.md on an install (read-only).

For every format-8 `.wld` (smallest first): the visibility boxes (`SVisBoxEntity`, `SVisBoxProperties`,
`CVisBoxRenderable`), the visibility groups (`CVisGroupRenderable.1` / `CRenderable.2`), the entity
visibility ids, whether the group members lie inside their box, and the 2D grids (`CRenderableGrid`,
`CLightGrid`: entry rectangles, levels, quadtree links). Also scans all class and member names for
portal / sector / occlusion concepts.

Usage: python -I tools/re/wld_vis_verify.py [DATA_DIR] [LIMIT]
       (DATA_DIR defaults to $SS2_DATA_DIR; LIMIT = only the LIMIT smallest worlds, default all, about 20 minutes)
"""
import collections, math, os, re, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Blob, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
from wld_root_verify import ancestry, ids, matrix  # noqa: E402

NULL = 0xFFFFFFFF
WORDS = re.compile(r"portal|sector|zone|occlu|visib|\bpvs\b|cull|antiportal", re.I)


def vec(v): return [v.get(f"Vector3f.{k}") for k in (1, 2, 3)]


def rows(seq): return [[getattr(v, "v", v) for _, v in n] for n in seq]


def cells(L, lo, hi, extent):
    size = extent / (1 << L)
    return int(math.floor((lo + extent / 2) / size)), int(math.floor((hi + extent / 2) / size))


def check_grid(g, kind, st, dist, objs):
    nodes = rows(g.get("CGenericGrid2D.1")); links = rows(g.get("CGenericGrid2D.3")); ents = rows(g.get("CGenericGrid2D.5"))
    fn, fl, fe = set(g.get("CGenericGrid2D.2")), set(g.get("CGenericGrid2D.4")), set(g.get("CGenericGrid2D.6"))
    depth = g.get("CGenericGrid2D.7"); extent = g.get("CGenericGrid2D.8")
    used_e = [i for i in range(len(ents)) if i not in fe]; used_l = {i for i in range(len(links)) if i not in fl}
    st[f"{kind}: grids"] += 1
    reached_links = collections.Counter(); reached_nodes = {0}
    ok_all = True; total_cells = 0

    def chain(n):
        out = []; l = nodes[n][1]
        while l != -1:
            out.append(l); l = links[l][1]
        return out
    for e in used_e:
        x0, x1, z0, z1, L = ents[e]
        dist[f"{kind}: entry level"][L] += 1
        dist[f"{kind}: entry cells per axis (x, z)"][(x1 - x0 + 1, z1 - z0 + 1)] += 1
        st[f"{kind}: entries with a rectangle of at most 2 x 2 cells inside the level's range"] += (0 <= x0 <= x1 < (1 << L)) and (0 <= z0 <= z1 < (1 << L)) and x1 - x0 < 2 and z1 - z0 < 2 and 0 <= L <= depth
        for cx in range(x0, x1 + 1):
            for cz in range(z0, z1 + 1):
                total_cells += 1; n = 0; reached_nodes.add(0); fail = False
                for s in range(1, L + 1):
                    slot = ((cx >> (L - s)) & 1) + 2 * ((cz >> (L - s)) & 1)
                    n = nodes[n][0][slot]
                    if n == -1: fail = True; break
                    reached_nodes.add(n)
                if fail: ok_all = False; continue
                found = [l for l in chain(n) if links[l][0] == e]
                if len(found) == 1: reached_links[found[0]] += 1
                else: ok_all = False
    st[f"{kind}: every (entry, cell) has exactly one link at the node reached by slot = bx + 2*bz"] += ok_all
    st[f"{kind}: used links == number of (entry, cell) pairs, each used exactly once"] += set(reached_links) == used_l and all(c == 1 for c in reached_links.values()) and len(used_l) == total_cells
    st[f"{kind}: the used nodes are exactly the nodes on the paths of the cells (plus node 0)"] += reached_nodes == {i for i in range(len(nodes)) if i not in fn}
    return ents, used_e, depth, extent


def check_world(name, d, st, dist, seen_members, names):
    r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects(); extra, _ = dec.extra_objects(edob)
    allo = {**objs, **extra}; st["worlds"] += 1
    for oid, tid in r["obty"]:
        for c in ancestry(r, tid): names.add(c)
    for o in allo.values():
        if isinstance(o, Obj) and o.type not in seen_members:
            seen_members[o.type] = {k for k, _ in o}
    w = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CWorld"); rd = objs[w.get("CWorld.2").oid]
    props = {}
    for p in ids(w.get("CWorld.5")):
        if p in extra: props[extra[p].get("CEntityProperties.2").oid] = extra[p]
    # entity visibility id
    for e, p in props.items():
        st["entity properties records"] += 1
        v = p.get("CBaseProperties.ep_ulEntityIDVis")
        if v is None: st["records without the ep_ulEntityIDVis member (older class variant)"] += 1; continue
        st["ep_ulEntityIDVis == CEntity.1 == CEntityProperties.8 (records that have the member)"] += v == objs[e].get("CEntity.1") == p.get("CEntityProperties.8")
        st["... records that have the member"] += 1
    vbe = {i: o for i, o in objs.items() if isinstance(o, Obj) and o.type == "SVisBoxEntity"}
    boxes = {i: o for i, o in objs.items() if isinstance(o, Obj) and o.type == "CVisBoxRenderable"}
    dist["vis boxes per world"][min(len(vbe), 40) // 5 * 5] += 1
    st["vis box entities"] += len(vbe); st["CVisBoxRenderable objects"] += len(boxes)
    st["worlds without vis boxes"] += not vbe
    st["each vis box entity owns exactly one CVisBoxRenderable that points back (CAspect.5)"] += sum(
        1 for i, e in vbe.items() if e.get("SVisBoxEntity.1").oid in boxes and boxes[e.get("SVisBoxEntity.1").oid].get("CAspect.5").oid == i)
    st["vis box entities that have a SVisBoxProperties record"] += sum(1 for i in vbe if i in props and props[i].type == "SVisBoxProperties")
    for i, e in vbe.items():
        p = props.get(i); b = boxes[e.get("SVisBoxEntity.1").oid]
        if p is None: continue
        st["properties (.1 size, .2, .3) equal the renderable's (.1, .2, .3)"] += (all(abs(a - c) < 1e-6 for a, c in zip(vec(p.get("SVisBoxProperties.1")), vec(b.get("CVisBoxRenderable.1")))) and p.get("SVisBoxProperties.2") == b.get("CVisBoxRenderable.2") and p.get("SVisBoxProperties.3") == b.get("CVisBoxRenderable.3"))
        dist["CVisBoxRenderable.2"][b.get("CVisBoxRenderable.2") if b.get("CVisBoxRenderable.2") > 1e30 else round(b.get("CVisBoxRenderable.2"), 1)] += 1
        dist["CVisBoxRenderable.3"][b.get("CVisBoxRenderable.3")] += 1
        dist["vis box aspect flags"][hex(b.get("CAspect.6").v)] += 1
        dist["vis box name given (CEntityProperties.4)"][bool(p.get("CEntityProperties.4"))] += 1
    # groups
    st["every CRenderable.2 that is set points at a CVisBoxRenderable"] += all(
        o.get("CRenderable.2").oid in boxes for o in objs.values() if isinstance(o, Obj) and o.get("CRenderable.2") is not None and o.get("CRenderable.2").oid != NULL)
    inv = True; members = {}
    for i, b in boxes.items():
        lst = ids(b.get("CVisGroupRenderable.1")); dist["group size"][min(len(lst), 400) // 50 * 50] += 1
        st["group lists: total members"] += len(lst)
        if len(set(lst)) != len(lst): inv = False
        for x in lst:
            if x in members: inv = False
            members[x] = i
            if objs[x].get("CRenderable.2").oid != i: inv = False
    for i, o in objs.items():
        if isinstance(o, Obj) and o.get("CRenderable.2") is not None and o.get("CRenderable.2").oid != NULL and i not in members: inv = False
    nest = collections.Counter()
    for i, b in boxes.items():
        depth_n = 0; cur = b; seen_b = {i}
        while cur.get("CRenderable.2").oid != NULL:
            cur = boxes[cur.get("CRenderable.2").oid]; depth_n += 1
            if id(cur) in {id(boxes[x]) for x in seen_b}: depth_n = -1; break
            seen_b.add(next(k for k, v in boxes.items() if v is cur))
        nest[depth_n] += 1
    for k, v in nest.items(): dist["vis box nesting depth (-1 = cycle)"][k] += v
    st["vis boxes that are themselves in a group"] += sum(v for k, v in nest.items() if k > 0)
    st["CPortalEntity instances (teleporter-like entity, not visibility)"] += sum(1 for o in objs.values() if isinstance(o, Obj) and o.type == "CPortalEntity")
    st["worlds with a CPortalEntity"] += any(isinstance(o, Obj) and o.type == "CPortalEntity" for o in objs.values())
    st["CRenderable.2 and CVisGroupRenderable.1 are exact inverses (each renderable in at most one group)"] += inv
    rend = [i for i, o in objs.items() if isinstance(o, Obj) and any(k == "CRenderable.1" for k, _ in o)]
    for i in rend:
        o = objs[i]; dist["renderables in a group, by class"][(o.type, i in members)] += 1
    for i in members:
        own = objs[i].get("CAspect.5").oid
        dist["group members: owning entity class"][objs[own].type if own in objs else "(none)"] += 1
        dist["group members: aspect flags"][hex(objs[i].get("CAspect.6").v)] += 1
    # containment of member origins in the box (centre = world translation, size = full edge lengths)
    mx = [0.0, 0.0, 0.0]; inside = outside = 0; unassigned_inside = 0; rot_id = 0
    bmat = {}
    for i, b in boxes.items():
        rot, t = matrix(b.get("CAspect.7")); sz = vec(b.get("CVisBoxRenderable.1")); bmat[i] = (rot, t, sz)
        rot_id += all(abs(rot[a][c] - (a == c)) < 1e-4 for a in range(3) for c in range(3))
    st["vis boxes with an identity rotation"] += rot_id; st["... of all vis boxes"] += len(boxes)

    def loc(i, bid):
        rot, t, sz = bmat[bid]; rm, tm = matrix(objs[i].get("CAspect.7")); dl = [tm[k] - t[k] for k in range(3)]
        return [sum(rot[a][k] * dl[a] for a in range(3)) / (sz[k] / 2) if sz[k] else 9e9 for k in range(3)]
    for i, bid in members.items():
        n = loc(i, bid); mx = [max(mx[k], abs(n[k])) for k in range(3)]
        if all(abs(c) <= 1 + 1e-3 for c in n): inside += 1
        else: outside += 1
    st["group members whose origin lies inside their box (|local| <= size/2)"] += inside; st["group members whose origin lies outside their box"] += outside
    dist["largest |local|/(size/2) over the members (x, y, z), per world"][tuple(round(c, 1) for c in mx)] += 1
    # non-members whose origin lies inside some box (second grid renderables)
    for i in rend:
        if i in members or i in boxes: continue
        if any(all(abs(c) <= 1 for c in loc(i, bid)) for bid in boxes): unassigned_inside += 1
    st["renderables without a group whose origin lies inside some box"] += unassigned_inside
    st["renderables (CRenderable.1) in all"] += len(rend); st["... in a group"] += len(members)
    # grids and the domain
    rg = [objs[rd.get("CRenderingDomain.6").oid], objs[rd.get("CRenderingDomain.7").oid]]
    for k, g in enumerate(rg):
        l = ids(g.get("CRenderableGrid.1")); cls = {objs[x].type for x in l}
        dist["renderable grid (.6/.7): holds vis boxes only / vis boxes none / empty"][(k + 6, "vis boxes only" if cls == {"CVisBoxRenderable"} and l else ("empty" if not l else "no vis boxes" if "CVisBoxRenderable" not in cls else "mixed"))] += 1
        ents, used, depth, extent = check_grid(g, "renderable grid", st, dist, objs)
        dist["grid depth / extent"][(depth, extent)] += 1
        # CRenderable.1 = entry index
        st["renderable grid: CRenderable.1 is the index of a used entry"] += all(objs[x].get("CRenderable.1") in set(used) for x in l)
        st["renderable grid: CRenderable.1 values unique within the grid"] += len({objs[x].get("CRenderable.1") for x in l}) == len(l)
        st["renderable grid: CRenderable.1 == list position"] += all(objs[x].get("CRenderable.1") == n for n, x in enumerate(l))
        for x in l:
            o = objs[x]; e = ents[o.get("CRenderable.1")]; rot, t = matrix(o.get("CAspect.7")); L = e[4]; c = cells(L, t[0], t[0], extent), cells(L, t[2], t[2], extent)
            st["renderable grid: the renderable's origin (x, z) lies inside its entry's rectangle"] += e[0] <= c[0][0] and c[0][1] <= e[1] and e[2] <= c[1][0] and c[1][1] <= e[3]
            st["renderable grid: renderables"] += 1
            if o.type == "CVisBoxRenderable":
                sz = vec(o.get("CVisBoxRenderable.1")); hx = sum(abs(rot[0][k]) * sz[k] / 2 for k in range(3)); hz = sum(abs(rot[2][k]) * sz[k] / 2 for k in range(3))
                # level = the finest level whose cell size (extent / 2^L) is at least the larger side of the box's rectangle
                side = max(2 * hx, 2 * hz); best = None
                for LL in range(0, depth + 1):
                    if extent / (1 << LL) >= side: best = LL
                if best is not None:
                    cx = cells(best, t[0] - hx, t[0] + hx, extent); cz = cells(best, t[2] - hz, t[2] + hz, extent); best = (cx[0], cx[1], cz[0], cz[1], best)
                st["vis box: entry == rectangle of the rotated box (centre +- size/2, x and z) at the finest level whose cell is at least as large as the box's larger side"] += best == tuple(e)
                st["vis box: entries"] += 1
    lg = objs[rd.get("CRenderingDomain.5").oid]; check_grid(lg, "light grid", st, dist, objs)
    # the "active" list CRenderingDomain.8 versus group membership
    act = set(ids(rd.get("CRenderingDomain.8"))); nogroup = {i for i in rend if i not in members}
    st["CRenderingDomain.8 holds no group member"] += not (act & set(members))
    st["CRenderingDomain.8 is a subset of the renderables without a group"] += act <= nogroup
    st["CRenderingDomain.8 == all renderables without a group"] += act == nogroup
    for i in nogroup - act: dist["renderables without a group that are not in CRenderingDomain.8, by class"][objs[i].type] += 1
    for i in act - nogroup: dist["CRenderingDomain.8 entries that are not CRenderable.1 objects or are in a group"][objs[i].type] += 1
    # which domain lists hold the vis boxes
    for key in ("CRenderingDomain.1", "CRenderingDomain.8"):
        have = set(ids(rd.get(key))); dist[f"vis boxes in {key}"][sum(1 for i in boxes if i in have) == len(boxes)] += 1


def main():
    install = sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].isdigit() else os.environ.get("SS2_DATA_DIR", "")
    limit = int(sys.argv[-1]) if sys.argv[-1].isdigit() else None
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter); seen = {}; names = set()
    files = merged_files(install, ".wld")
    for n, (name, d) in enumerate(sorted(files.items(), key=lambda kv: len(kv[1]))):
        if limit and n >= limit: break
        check_world(name, d, st, dist, seen, names); print(f"[{n + 1}/{len(files)}] {name.rsplit('/', 1)[-1]}", file=sys.stderr, flush=True)
    for k, v in sorted(st.items()): print(f"{v:9d}  {k}")
    for k, v in sorted(dist.items()): print(k, dict(v.most_common(14)))
    print("class names matching portal/sector/zone/occlusion/visibility words:", sorted(c for c in names if WORDS.search(c)))
    members = sorted({m for ms in seen.values() for m in ms if WORDS.search(m)})
    print("member names matching those words:", members)
    print("classes:", len(names))


if __name__ == "__main__":
    main()
