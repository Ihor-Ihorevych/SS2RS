"""Re-runs the measurements behind docs/re/wld-physics.md on an install (read-only).

For every format-8 `.wld`: mechanisms and their parts, the bodies and hulls and how they point at each other,
the collision grid cells of every hull, the joints (stored as two cross-linked halves), the motion handlers,
the material attributes, the collision layer names and the physics domain lists.

Usage: python -I tools/re/wld_physics_verify.py [DATA_DIR] [LIMIT]
       (DATA_DIR defaults to $SS2_DATA_DIR; LIMIT = only the LIMIT smallest worlds, default all, about 15 minutes)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Blob, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
from wld_root_verify import ids  # noqa: E402

NULL = 0xFFFFFFFF
BODIES = ("CDynamicBody", "CHybridBody", "CKinematicBody")
HULLS = ("CPrimitiveHull", "CModelHull", "CFluidHull", "CForceFieldHull", "CDummyHull")


def rows(seq): return [[v for _, v in n] for n in seq]


def check_world(name, d, st, dist):
    r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects()
    try: extra, _ = dec.extra_objects(edob)
    except Exception: extra = {}
    allo = {**objs, **extra}; rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}; idnt = r["idnt"]
    w = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CWorld")
    pd = objs[w.get("CWorld.4").oid] if isinstance(objs.get(w.get("CWorld.4").oid), Obj) and objs[w.get("CWorld.4").oid].type == "CPhysicsDomain" else next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CPhysicsDomain")
    cd = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CCollisionDomain")
    bodies = {i: o for i, o in objs.items() if isinstance(o, Obj) and o.type in BODIES}
    hulls = {i: o for i, o in objs.items() if isinstance(o, Obj) and o.type in HULLS}
    mechs = {i: o for i, o in objs.items() if isinstance(o, Obj) and o.type == "CMechanism"}
    st["worlds"] += 1; st["bodies"] += len(bodies); st["hulls"] += len(hulls); st["mechanisms"] += len(mechs)
    for o in bodies.values(): dist["body class"][o.type] += 1
    for o in hulls.values(): dist["hull class"][o.type] += 1
    # --- mechanism parts
    parts_total = 0; hull_of_part = {}; body_of_part = {}; hull_part_pos = {}; body_part_pos = {}
    for mi, m in mechs.items():
        parts = list(m.get("CMechanism.1")); parts_total += len(parts)
        st["mechanisms with a renderable (.6)"] += m.get("CMechanism.6").oid in objs
        st["mechanism renderable: CAspect.5 owner is an entity that owns this mechanism (CStaticModelEntity.10 or similar)"] += 0
        rend_oid = m.get("CMechanism.6").oid
        owner_ent = objs[rend_oid].get("CAspect.5").oid if rend_oid in objs else NULL
        dist["T01-066: mechanism (via renderable CMechanism.6 -> CAspect.5) owner entity class"][allo[owner_ent].type if owner_ent in allo and isinstance(allo[owner_ent], Obj) else "(no owner)"] += 1
        for k, p in enumerate(parts):
            h = p.get("CMechanismPart.5"); b = p.get("CMechanismPart.3"); j = p.get("CMechanismPart.2")
            hn = h.oid != NULL; bn = b.oid != NULL
            st["parts"] += 1; st["... with a hull (.5) and no body"] += hn and not bn; st["... with a body (.3) and no hull"] += bn and not hn; st["... with both"] += hn and bn; st["... with neither"] += (not hn) and (not bn)
            if bn: body_of_part[b.oid] = mi; st["... body of a part is a body of the world"] += b.oid in bodies
            if hn: hull_of_part[h.oid] = mi; st["... hull of a part is a hull of the world"] += h.oid in hulls
            if j.oid != NULL: st["parts with a joint (.2)"] += 1
            pi = p.get("CMechanismPart.7")
            st["part .7 is -1 or the index of an earlier part of the same mechanism"] += pi == -1 or 0 <= pi < k
            st["... of which -1 (a root part)"] += pi == -1
            hull_part_pos[(mi, h.oid)] = k if hn else None
            if bn: body_part_pos[(mi, b.oid)] = k
            dist["part name (.1)"][idnt[p.get("CMechanismPart.1")] if idnt[p.get("CMechanismPart.1")] in ("", "Main", "Root") else "(other)"] += 1
    dist["parts per mechanism"][min(parts_total // max(len(mechs), 1), 4)] += 0
    # --- body <-> hull
    for bi, b in bodies.items():
        hs = [hi for hi, h in hulls.items() if h.get("CHull.1").oid == bi]
        st["bodies with exactly one hull pointing back (CHull.1)"] += len(hs) == 1
        child = b.get("CAspect.2").oid
        st["... and that hull is the body's first child aspect (CAspect.2)"] += len(hs) == 1 and child == hs[0]
        st["bodies in CPhysicsDomain.1"] += 0
    for hi, h in hulls.items():
        bp = h.get("CHull.1").oid
        if bp != NULL: st["hulls with a body (.1)"] += 1; st["... the body is a body of the world"] += bp in bodies
        mp = h.get("CHull.9").oid
        st["hulls with a mechanism (.9)"] += mp != NULL
        if mp != NULL:
            mpart = hull_of_part.get(hi) == mp or (bp != NULL and body_of_part.get(bp) == mp)
            st["... that mechanism lists the hull (directly, or through its body)"] += mpart
            if not mpart:
                owner_ent = objs[bp].get("CAspect.5").oid if bp != NULL and bp in objs else NULL
                dist["T01-066: 111 unlisted hulls: (hull class, owning entity class)"][(h.type, allo[owner_ent].type if owner_ent in allo and isinstance(allo[owner_ent], Obj) else "(no body/entity)")] += 1
            pos = hull_part_pos.get((mp, hi)) if (mp, hi) in hull_part_pos else (body_part_pos.get((mp, bp)) if bp != NULL else None)
            if pos is not None: st["hull .10 == position of its part in the mechanism's part list"] += h.get("CHull.10") == pos; st["... hulls compared"] += 1
        dist["collision layer (CHull.2)"][(idnt[h.get("CHull.2")])] += 1
    # --- physics domain lists
    p1 = set(ids(pd.get("CPhysicsDomain.1"))); p2 = set(ids(pd.get("CPhysicsDomain.2")))
    st["CPhysicsDomain.1 == all bodies"] += p1 == set(bodies); st["CPhysicsDomain.2 is a subset of .1"] += p2 <= p1
    for bi, b in bodies.items():
        dist["body in CPhysicsDomain.2 (active), by class"][(b.type, bi in p2)] += 1
        if b.type == "CDynamicBody":
            own = b.get("CAspect.5").oid
            dist["dynamic body: (in .2, owner entity class)"][(bi in p2, objs[own].type if own in objs else "(none)")] += 1
            dist["dynamic body: (in .2, CBody.11, CBody.13, CBody.14)"][(bi in p2, b.get("CBody.11"), b.get("CBody.13"), b.get("CBody.14"))] += 1
    # --- collision grid cells of every hull
    cg = objs[cd.get("CCollisionDomain.2").oid]; nodes = rows(cg.get("CCollisionGrid.1")); links = rows(cg.get("CCollisionGrid.3"))
    depth = cg.get("CCollisionGrid.5"); extent = cg.get("CCollisionGrid.6")
    fl = set(cg.get("CCollisionGrid.4")); used_l = {i for i in range(len(links)) if i not in fl}
    found = collections.Counter(); ok_all = True; total = 0
    def chain(n):
        out = []; l = nodes[n][1]
        while l != -1: out.append(links[l][0].oid if isinstance(links[l][0], Ref) else links[l][0]); l = links[l][1]
        return out
    for hi, h in hulls.items():
        x0, x1, z0, z1, L = [h.get(f"CHull.{k}") for k in (3, 4, 5, 6, 7)]
        dist["hull cells per axis (x, z)"][(x1 - x0 + 1, z1 - z0 + 1)] += 1
        for cx in range(x0, x1 + 1):
            for cz in range(z0, z1 + 1):
                total += 1; n = 0; fail = False
                for s in range(1, L + 1):
                    slot = ((cx >> (L - s)) & 1) + 2 * ((cz >> (L - s)) & 1); n = nodes[n][0][slot]
                    if n == -1: fail = True; break
                if fail: ok_all = False; continue
                if hi in chain(n): found[hi] += 1
                else: ok_all = False
    st["collision grids"] += 1; st["collision grid: every cell of every hull (CHull.3..7) has a link to the hull at the node reached by slot = bx + 2*bz"] += ok_all
    st["collision grid: used links == (hull, cell) pairs"] += len(used_l) == total
    st["collision grid: cells per hull at most 2 x 2"] += all(h.get("CHull.4") - h.get("CHull.3") < 2 and h.get("CHull.6") - h.get("CHull.5") < 2 for h in hulls.values())
    # --- joints: halves linked by CJoint.6
    joints = {i: o for i, o in objs.items() if isinstance(o, Obj) and any(a == "CJoint" for a in [o.type] + [])} if False else {}
    jcls = [i for i, o in objs.items() if isinstance(o, Obj) and o.type.endswith("Joint") and o.get("CJoint.2") is not None]
    st["joints (halves)"] += len(jcls)
    for i in jcls: dist["joint class"][objs[i].type] += 1
    for i in jcls:
        j = objs[i]; o = j.get("CJoint.6").oid
        if o != NULL:
            st["joint halves with a partner (.6)"] += 1; st["... partner points back"] += o in objs and objs[o].get("CJoint.6").oid == i
            st["... partner has the same class"] += o in objs and objs[o].type == j.type
        st["joint owner (.1) is an entity"] += j.get("CJoint.1").oid in objs; st["joint body (.3) is a body of the world"] += j.get("CJoint.3").oid in bodies
        b5 = j.get("CJoint.5").oid
        if b5 != NULL: st["joint with a second body (.5)"] += 1; st["... it is a body of the world"] += b5 in bodies
    for bi, b in bodies.items():
        first = b.get("CBody.1").oid
        if first != NULL:
            got = []; cur = first; seen = set()
            while cur != NULL and cur in objs and cur not in seen: got.append(cur); seen.add(cur); cur = objs[cur].get("CJoint.4").oid
            mine = [i for i in jcls if objs[i].get("CJoint.3").oid == bi]
            both = [i for i in jcls if objs[i].get("CJoint.3").oid == bi or objs[i].get("CJoint.5").oid == bi]
            st["bodies with joints: CBody.1 then CJoint.4 chain == joints whose CJoint.3 is the body"] += sorted(got) == sorted(mine); st["bodies with joints"] += 1
            st["... chain == joints whose CJoint.3 or CJoint.5 is the body"] += sorted(got) == sorted(both)
            st["... chain is a subset of the joints naming the body (.3 or .5)"] += set(got) <= set(both)
            st["... chain length 1"] += len(got) == 1
            # T01-066: CJoint::GetNextInBody alternates .4 (if this body is the joint's .3) / .6 (if .5) -- test against this body's joint set
            got2 = []; cur = first; seen2 = set()
            while cur != NULL and cur in objs and cur not in seen2:
                got2.append(cur); seen2.add(cur); j = objs[cur]
                if j.get("CJoint.3").oid == bi: cur = j.get("CJoint.4").oid
                elif j.get("CJoint.5").oid == bi: cur = j.get("CJoint.6").oid
                else: cur = NULL
            st["T01-066: CBody.1 then alternating .4 (if body==.3) / .6 (if body==.5) chain == joints naming the body (.3 or .5)"] += sorted(got2) == sorted(both)
            st["T01-066: ...is a superset-or-equal of the old .4-only chain"] += set(got) <= set(got2)
    # --- motion handlers
    mh = {i: o for i, o in objs.items() if isinstance(o, Obj) and o.type == "CMechanismMotionHandler"}
    for i, h in mh.items():
        mm = h.get("CMechanismMotionHandler.1").oid
        st["motion handlers"] += 1; st["... whose mechanism points back (CMechanism.3)"] += mm in mechs and mechs[mm].get("CMechanism.3").oid == i
    st["mechanisms with a motion handler (.3)"] += sum(1 for m in mechs.values() if m.get("CMechanism.3").oid != NULL)
    # --- material attributes
    for owner, key in ((mechs, "CMechanism.7"), (hulls, "CPrimitiveHull.2")):
        for o in owner.values():
            v = o.get(key)
            if v is None or v.oid == NULL: continue
            kind = "external ." + ex[v.oid].rsplit(".", 1)[-1].lower() if v.oid in ex else (allo[v.oid].type if v.oid in allo and isinstance(allo[v.oid], Obj) else "?")
            dist[f"material attributes target ({key})"][kind] += 1


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
    for k, v in sorted(dist.items()): print(k, dict(v.most_common(16)))


if __name__ == "__main__":
    main()
