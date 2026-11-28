"""Re-runs the measurement behind docs/re/ans.md's "Target ids" correction (T01-430, follow-up to T04-006).

The bug this answers: `ss2-format-ans` bound a translation/rotation envelope's `target_id`
(`CAnimEnvelope.1`) straight to a `.skl` bone of that numeric id. That works for an animation set
*embedded* in the same `.mdl` file as its skeleton (because both share one file's `IDNT` table and
the bones happen to occupy its low, stable indices), but fails for a *standalone* `.ans` file bound
to an *external* `.skl` (each file has its own, independently-numbered `IDNT` table).

The real rule, confirmed here on data: `target_id` is an `IDENT` -- an index into the envelope's own
file's `IDNT` string table -- naming the bone **by its name string**, not by its numeric id. Every
`.skl` file's own `IDNT` table stores the bone's name at the same index as the bone's id (`IDNT[id]`
= that bone's name, for `id` in `1..boneCount`; index 0 is the empty string, matching "no bone" / no
match). So the loader must: 1) read the envelope's own file's `IDNT[target_id]` to get the bone's
name string; 2) find that string in the skeleton's own `IDNT` table; 3) the string's index there is
the bone id. For an animation set embedded in the same `.mdl` as its skeleton, both steps happen to
use the *same* table, and the bones were registered at the same low indices as their own bone ids --
so the old "treat target_id as a bone id" reading coincidentally also worked, which is why
`ans.md`'s 98.7%/99.6% held: those figures were measured only over animation sets embedded in a
model with an embedded skeleton (see `ans_verify.py`, which always passed `ids=None` -- skipping the
check entirely -- for standalone `.ans` files). This script checks name resolution over every
animated model and skeleton of the install, both embedded and standalone/external.

Usage: python -I tools/re/ans_bone_name_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR)
"""
import collections
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, TreeDec, merged_files, parse  # noqa: E402

NULL = 0xFFFFFFFF


def g(o, name):
    return o.get(name)


def skeleton_name_table(sk_r, sk_objs):
    """{bone name -> bone id} for LOD 0 of a decoded CSkeleton, using IDNT[bone id] = name."""
    sk = next(o for o in sk_objs.values() if isinstance(o, Obj) and o.type == "CSkeleton")
    lod0 = g(sk, "CSkeleton.1")[0]
    bones = g(lod0, "CSkeletonLOD.2")
    ids = {g(b, "SkeletonBone.1") for b in bones}
    idnt = sk_r["idnt"]
    name_ok = sum(1 for bid in ids if bid < len(idnt) and idnt[bid])
    by_name = {idnt[bid]: bid for bid in ids if bid < len(idnt) and idnt[bid]}
    bone_len = {}
    for b in bones:
        bone_len[g(b, "SkeletonBone.1")] = g(b, "SkeletonBone.6")
    return ids, by_name, bone_len, name_ok, len(ids)


def external_ref(r, ref):
    """None if `ref` is null; ('int', object) if embedded (resolve later); ('ext', path) if external."""
    if ref.oid == NULL:
        return None
    ext = {ob: fi for ob, fi, _, _ in r["exob"]}
    if ref.oid in ext:
        return "ext", r["rfil"][ext[ref.oid]][2].lower()
    return "int", ref.oid


def load_skeleton(cfg, r, objs, skl_files):
    ref = g(cfg, "CModelConfiguration.2")
    res = external_ref(r, ref)
    if res is None:
        return None
    kind, val = res
    if kind == "int":
        if val not in objs:
            return None
        return r, objs  # embedded: same tree
    if not val.endswith(".skl"):
        return None  # external .rsc skeleton: not specified (skl.md T01-015), skip
    d = skl_files.get(val)
    if d is None:
        return None
    sr = parse(d)
    sobjs, _ = TreeDec(d, sr).objects()
    return sr, sobjs


def iter_animsets(cfg, r, objs, ans_files):
    """Yield (anim_r, anim_objs, CAnimSet) for every embedded or external animation set of a model.

    CModelConfiguration.9 -> CModelAnimHost -> CAnimHost.1 (list of CAnimSetInstance -> CAnimSetInstance.1).
    """
    ahref = g(cfg, "CModelConfiguration.9")
    if ahref is None or ahref.oid == NULL or ahref.oid not in objs:
        return
    host = objs[ahref.oid]
    insts = g(host, "CAnimHost.1")  # CAnimSetInstance has no CObject base: inline values, not references
    for inst_obj in insts:
        setref = g(inst_obj, "CAnimSetInstance.1")
        res = external_ref(r, setref)
        if res is None:
            continue
        kind, val = res
        if kind == "int":
            if val not in objs or not isinstance(objs[val], Obj) or objs[val].type != "CAnimSet":
                continue
            yield r, objs, objs[val]
        else:
            if not val.endswith(".ans"):
                continue
            d = ans_files.get(val)
            if d is None:
                continue
            ar = parse(d)
            aobjs, _ = TreeDec(d, ar).objects()
            for o in aobjs.values():
                if isinstance(o, Obj) and o.type == "CAnimSet":
                    yield ar, aobjs, o


def check_model(name, cfg, r, objs, skl_files, ans_files, st, bucket_st, vectors, wanted_vectors):
    sk = load_skeleton(cfg, r, objs, skl_files)
    if sk is None:
        return
    sk_r, sk_objs = sk
    same_file = sk_r is r
    ids, by_name, bone_len, name_ok, n_bones = skeleton_name_table(sk_r, sk_objs)
    if n_bones:
        st["skeletons checked (reached from a model)"] += 1
        st["skeletons where IDNT[bone id] = a non-empty name for every bone"] += name_ok == n_bones

    for ar, aobjs, aset in iter_animsets(cfg, r, objs, ans_files):
        bucket = "embedded animation set + embedded skeleton (shared IDNT table)" if same_file and ar is r else \
            "embedded animation set + external skeleton" if ar is r else \
            "external .ans + embedded skeleton" if ar is not r and same_file else \
            "external .ans + external skeleton"
        for aref in g(aset, "CAnimSet.1"):
            a = aobjs[aref.oid]
            anim_name = ar["idnt"][g(a, "CAnimation.1")] if g(a, "CAnimation.1") < len(ar["idnt"]) else None
            envs = [aobjs[e.oid] for e in g(a, "CAnimation.6")]
            for e in envs:
                if e.type not in ("CVec3LenAnimEnvelope", "CQuatAnimEnvelope"):
                    continue
                tid = g(e, "CAnimEnvelope.1")
                st["translation/rotation envelopes checked"] += 1
                bucket_st[bucket]["envelopes"] += 1
                direct_hit = tid in ids
                st["old rule: target_id is directly a bone id of the skeleton"] += direct_hit
                bucket_st[bucket]["old rule hits"] += direct_hit
                bone_name = ar["idnt"][tid] if tid < len(ar["idnt"]) else None
                resolved = by_name.get(bone_name) if bone_name else None
                name_hit = resolved is not None
                st["new rule: IDNT[target_id] names a bone of the skeleton's own IDNT table"] += name_hit
                bucket_st[bucket]["name rule hits"] += name_hit
                if e.type == "CVec3LenAnimEnvelope" and resolved in bone_len:
                    st["translation envelopes checked (new-rule resolvable)"] += 1
                    lenval = g(e, "CVec3LenAnimEnvelope.1")
                    st["... len x skeleton bone length == 1 (1e-3) using the new rule's bone id"] += \
                        abs(lenval * bone_len[resolved] - 1) < 1e-3
            if name in wanted_vectors and anim_name in wanted_vectors[name]:
                trs = [e for e in envs if e.type in ("CVec3LenAnimEnvelope", "CQuatAnimEnvelope")]
                n_old = sum(1 for e in trs if g(e, "CAnimEnvelope.1") in ids)
                n_new = sum(1 for e in trs
                            if (ar["idnt"][g(e, "CAnimEnvelope.1")] if g(e, "CAnimEnvelope.1") < len(ar["idnt"]) else None) in by_name)
                vectors.append((name, anim_name, bucket, len(trs), n_old, n_new))


def main():
    install = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder (the one containing All_PC_01.gro) or set SS2_DATA_DIR")

    skl_files = merged_files(install, ".skl")
    ans_files = merged_files(install, ".ans")
    mdl_files = merged_files(install, ".mdl")

    # Standalone-.skl naming-convention check (independent of any model).
    st = collections.Counter()
    for name, d in sorted(skl_files.items()):
        r = parse(d)
        objs, _ = TreeDec(d, r).objects()
        sk = next((o for o in objs.values() if isinstance(o, Obj) and o.type == "CSkeleton"), None)
        if sk is None:
            continue
        idnt = r["idnt"]
        for lod in g(sk, "CSkeleton.1"):
            bones = g(lod, "CSkeletonLOD.2")
            ids = {g(b, "SkeletonBone.1") for b in bones}
            st["standalone .skl LODs"] += 1
            st["standalone .skl LODs where IDNT[bone id] is a non-empty name for every bone"] += \
                all(bid < len(idnt) and idnt[bid] for bid in ids)

    wanted_vectors = {
        "content/serioussam2/models/enemies/scorpsoldier/scorpsoldier.mdl": {"ScorpSoldier_Idle"},
        "content/serioussam2/models/enemies/tank/tank.mdl": {"Walk"},
        "content/serioussam2/models/npcs/brimland/female.mdl": {"BackPedal"},
    }
    bucket_st = collections.defaultdict(collections.Counter)
    vectors = []
    checked_models = 0
    for name, d in sorted(mdl_files.items()):
        r = parse(d)
        objs, _ = TreeDec(d, r).objects()
        cfg = next((o for o in objs.values() if isinstance(o, Obj) and o.type == "CModelConfiguration"
                    and g(o, "CResource.1") == 1), None)
        if cfg is None:
            continue
        ahref = g(cfg, "CModelConfiguration.9")
        if ahref is None or ahref.oid == NULL or ahref.oid not in objs:
            continue
        insts = g(objs[ahref.oid], "CAnimHost.1")
        if not any(g(i, "CAnimSetInstance.1").oid != NULL for i in insts):
            continue
        checked_models += 1
        check_model(name, cfg, r, objs, skl_files, ans_files, st, bucket_st, vectors, wanted_vectors)

    print(f"models with a non-null animation host, checked: {checked_models}")
    print("\n== standalone .skl files: does IDNT[bone id] = the bone's name? ==")
    for k, v in sorted(st.items()):
        if k.startswith("standalone"):
            print(f"{v:9d}  {k}")

    print("\n== models reached from a skeleton + animation set (both rules compared) ==")
    for k, v in sorted(st.items()):
        if not k.startswith("standalone"):
            print(f"{v:9d}  {k}")

    print("\n== by binding category ==")
    for bucket, c in sorted(bucket_st.items()):
        print(f"  {bucket}: envelopes {c['envelopes']}, old-rule hits {c['old rule hits']}, new-rule hits {c['name rule hits']}")

    print("\n== test vectors ==")
    for name, anim, bucket, n, n_old, n_new in vectors:
        print(f"  {name} :: {anim} ({bucket}): {n} translation/rotation envelopes, old rule matches {n_old}, new rule matches {n_new}")


if __name__ == "__main__":
    main()
