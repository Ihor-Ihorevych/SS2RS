"""Re-runs the relation checks behind docs/re/wld-entities.md on an install (read-only).

For every format-8 `.wld` (smallest first): the world's property records and the "owned" records that are not
in `CWorld.5`, the spawner structure (`SSpawnerEntity` and the records it points at), the parent / first child /
next sibling links between property records (`CEntityProperties.5/.6/.7`) and their agreement with the aspect
tree, and the links to `.ep` files (puppets, items, weapon pickups, resource holders).
(`python -I tools/re/wld_entity_schema.py` generates the member reference and counts the classes.)

Usage: python -I tools/re/wld_entity_verify.py [DATA_DIR] [LIMIT]
       (DATA_DIR defaults to $SS2_DATA_DIR; LIMIT = only the LIMIT smallest worlds, default all, about 15 minutes)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Blob, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
from ep_schema import load as load_eps, root_of  # noqa: E402
from wld_root_verify import ids  # noqa: E402

NULL = 0xFFFFFFFF
NO_SERIAL = 0xFFFFFFFF


def lineage(parent, name):
    out = []
    while name is not None and name not in out: out.append(name); name = parent.get(name)
    return out


def norm(v, allo):
    if isinstance(v, Ref): return ("ref", allo[v.oid].type if v.oid != NULL and isinstance(allo.get(v.oid), Obj) else v.oid)
    if isinstance(v, Obj): return (v.type, tuple((k, norm(x, allo)) for k, x in v))
    if isinstance(v, list) and not isinstance(v, Blob): return tuple(norm(x, allo) for x in v)
    return v


def check_world(name, d, eps, st, dist):
    r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects(); extra, _ = dec.extra_objects(edob); allo = {**objs, **extra}
    T = r["types"]; parent = {t.name: (T[t.parent].name if t.parent in T else None) for t in T.values() if t.members is not None}
    rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}
    w = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CWorld")
    world_recs = {p: extra[p] for p in ids(w.get("CWorld.5")) if p in extra}
    own = {}                                   # entity oid -> its record in CWorld.5
    for p, rec in world_recs.items(): own[rec.get("CEntityProperties.2").oid] = p
    ents = [i for i in ids(w.get("CWorld.1"))]
    st["worlds"] += 1; st["entities"] += len(ents)
    recs_all = [i for i, o in extra.items() if isinstance(o, Obj) and "CEntityProperties" in lineage(parent, o.type)]
    st["property records (extra section)"] += len(recs_all); st["... named by an entity (CWorld.5)"] += len(world_recs); st["... owned (not in CWorld.5)"] += len(recs_all) - len(world_recs)
    st["owned records have CEntityProperties.2 = null"] += sum(1 for i in recs_all if i not in world_recs and extra[i].get("CEntityProperties.2").oid == NULL)
    # --- spawners
    spawners = [i for i in ents if objs[i].type == "SSpawnerEntity"]
    st["spawner entities"] += len(spawners)
    st["spawners without serial (0xFFFFFFFF)"] += sum(1 for i in spawners if objs[i].get("CEntity.1") == NO_SERIAL)
    sp_props = {i for i in recs_all if extra[i].type == "SSpawnerProperties"}
    st["SSpawnerProperties records"] += len(sp_props); st["... owned (CEntityProperties.2 null)"] += sum(1 for i in sp_props if extra[i].get("CEntityProperties.2").oid == NULL)
    refs = collections.Counter(); ref_from = collections.Counter()
    for i in recs_all:
        for k, v in extra[i]:
            if isinstance(v, Ref) and v.oid in sp_props: refs[v.oid] += 1; ref_from[(k, "world record" if i in world_recs else "owned record")] += 1
    st["SSpawnerProperties records referenced exactly once"] += sum(1 for i in sp_props if refs[i] == 1)
    for k, n in ref_from.items(): dist["SSpawnerProperties referenced by (member, record kind)"][k] += n
    for i in spawners:
        s = objs[i]; tgt = s.get("SSpawnerEntity.120").oid; rec = s.get("SSpawnerEntity.21").oid
        spawnee = objs.get(tgt); fam = lineage(parent, spawnee.type) if spawnee is not None else []
        kind = "puppet" if "SPuppetEntity" in fam else "item" if "SBaseItemEntity" in fam else spawnee.type if spawnee is not None else "none"
        dist["spawnee (.120) family"][kind] += 1
        st["spawnee (.120) is an entity with a record in CWorld.5"] += tgt in own
        st["spawnee record (.21) is not in CWorld.5"] += rec not in world_recs
        mine = allo.get(own.get(tgt))
        same_class = mine is not None and allo[rec].type == mine.type
        st["spawnee record (.21) has the class of the spawnee's own record"] += same_class
        if same_class:
            a, b = allo[rec], mine; eq = 0; n = 0
            for (k, x), (k2, y) in zip(a, b):
                n += 1; eq += norm(x, allo) == norm(y, allo)
                if norm(x, allo) != norm(y, allo): dist["members of the spawnee record that differ from the entity's own record"][k] += 1
            st["spawnee record members compared"] += n; st["... equal to the entity's own record"] += eq
    # --- hierarchy links between records
    ok_parent = ok_child = 0; n_parent = 0
    kids = collections.defaultdict(list)
    for p, rec in world_recs.items():
        par = rec.get("CEntityProperties.5").oid
        if par != NULL: kids[par].append(p); n_parent += 1
    st["records with a parent link (.5)"] += n_parent
    st["parent link (.5) targets a record of CWorld.5"] += sum(1 for par in kids if par in world_recs)
    cyc = 0
    for par, lst in kids.items():
        if par not in world_recs: continue
        got = []; cur = world_recs[par].get("CEntityProperties.6").oid; seen = set()
        while cur != NULL and cur in world_recs and cur not in seen: got.append(cur); seen.add(cur); cur = world_recs[cur].get("CEntityProperties.7").oid
        cyc += cur != NULL
        ok_child += sorted(got) == sorted(lst)
    st["parents whose first-child/next-sibling chain (.6/.7) lists exactly the records that name them as parent (.5)"] += ok_child
    st["parents (.5 targets)"] += len(kids); st["child/sibling chains that do not end (cycle or bad link)"] += cyc
    # does the entity hierarchy agree with the aspect tree?
    asp_owner = collections.defaultdict(list)
    for i, o in objs.items():
        if isinstance(o, Obj) and o.get("CAspect.5") is not None and o.get("CAspect.5").oid != NULL: asp_owner[o.get("CAspect.5").oid].append(i)
    agree = tot = direct = 0
    for p, rec in world_recs.items():
        par = rec.get("CEntityProperties.5").oid
        if par == NULL: continue
        ce = rec.get("CEntityProperties.2").oid; pe = world_recs[par].get("CEntityProperties.2").oid if par in world_recs else None
        for a in asp_owner.get(ce, []):
            pa = objs[a].get("CAspect.1").oid
            if pa == NULL: continue
            tot += 1; direct += objs[pa].get("CAspect.5").oid == pe
            up = pa; hit = False; guard = 0
            while up != NULL and guard < 200:
                if objs[up].get("CAspect.5").oid == pe: hit = True; break
                up = objs[up].get("CAspect.1").oid; guard += 1
            agree += hit
    st["child aspects with a parent aspect (child entity has a parent record)"] += tot
    st["... whose parent aspect is owned by the parent entity"] += direct
    st["... with an ancestor aspect owned by the parent entity"] += agree
    # --- .ep links
    def ep_path(v): return ex.get(v.oid) if isinstance(v, Ref) and v.oid in ex and ex[v.oid].lower().endswith(".ep") else None
    for i in ents:
        e = objs[i]; fam = lineage(parent, e.type)
        if "SPuppetEntity" in fam:
            st["puppet entities"] += 1; mine = allo.get(own.get(i)); a = ep_path(e.get("SPuppetEntity.en_pppPuppetParams")); b = ep_path(mine.get("SPuppetProperties.ep_pppPuppetParams")) if mine is not None else None
            st["puppet: entity .ep link == its record's .ep link (both present)"] += a is not None and a == b
            if a is not None: dist["puppet params root class (by entity class)"][(e.type, eps.get(a.lower(), "MISSING"))] += 1
        if "SBaseItemEntity" in fam:
            st["item entities"] += 1; a = ep_path(e.get("SBaseItemEntity.503"))
            st["item entities with a .ep link (.503)"] += a is not None
            if a is not None: dist["item params root class"][eps.get(a.lower(), "MISSING")] += 1
        if "SWeaponItemEntity" in fam:
            st["weapon pickups"] += 1
            a = ep_path(e.get("SWeaponItemEntity.en_pbwpWeaponParams")); b = ep_path(e.get("SWeaponItemEntity.en_pbwpWeaponRenderParams"))
            want = e.type[:-len("ItemEntity")]
            st["weapon pickup: weapon params root class == <entity class without ItemEntity> + WeaponParams"] += a is not None and eps.get(a.lower()) == want + "WeaponParams"
            st["weapon pickup: render params root class == ... + WeaponRenderingParams"] += b is not None and eps.get(b.lower()) == want + "WeaponRenderingParams"
        if e.type in ("CWorldInfoEntity", "CChapterInfoEntity"):
            for k, v in e:
                if isinstance(v, list) and not isinstance(v, Blob):
                    for h in v:
                        if isinstance(h, Obj) and h.type == "CResourceOnDemandHolder":
                            a = ep_path(h.get("CResourceOnDemandHolder.rodh_prsResource"))
                            st["resource holders in world/chapter info"] += 1; st["... with a .ep link"] += a is not None
                            if a is not None: dist["resource holder .ep root class"][eps.get(a.lower(), "MISSING")] += 1


def main():
    install = sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].isdigit() else os.environ.get("SS2_DATA_DIR", "")
    limit = int(sys.argv[-1]) if sys.argv[-1].isdigit() else None
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    eps = {}
    for n, (d, r, objs, edob) in load_eps(install, (".ep",)).items(): eps[n.lower()] = objs[root_of(objs)[0]].type
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter)
    files = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))
    for n, (name, d) in enumerate(files):
        if limit and n >= limit: break
        check_world(name, d, eps, st, dist); print(f"[{n + 1}/{len(files)}] {name.rsplit('/', 1)[-1]}", file=sys.stderr, flush=True)
    for k, v in sorted(st.items()): print(f"{v:9d}  {k}")
    for k, v in sorted(dist.items()): print(k, dict(v.most_common(16)))


if __name__ == "__main__":
    main()
