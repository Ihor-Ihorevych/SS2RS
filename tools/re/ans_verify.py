"""Re-runs the measurements behind docs/re/ans.md on an install (read-only).

Checks, for every `CAnimSet` in the 88 `.ans` files and in the models that embed one: the
animation header members, the envelope members, the keyframe rules, the events, and (for models
with an embedded skeleton) that the envelopes address bones of that skeleton.

Usage: python -I tools/re/ans_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR)
"""
import collections, math, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, merged_files, parse  # noqa: E402

NULL = 0xFFFFFFFF


def g(o, name): return o.get(name)


def check_set(label, objs, ids, st, evclasses, bone_len=None):
    sets = [o for o in objs.values() if isinstance(o, Obj) and o.type == "CAnimSet"]
    for s in sets:
        st[f"{label}: animation sets"] += 1
        st["CAnimSet.3 values " + str(g(s, "CAnimSet.3"))] += 1
        for ref in g(s, "CAnimSet.1"):
            a = objs[ref.oid]; st["animations"] += 1
            first, last, step, fps = g(a, "CAnimation.2"), g(a, "CAnimation.3"), g(a, "CAnimation.4"), g(a, "CAnimation.5")
            st["step * fps = 1 (within 1e-4)"] += abs(step * fps - 1) < 1e-4
            envs = [objs[e.oid] for e in g(a, "CAnimation.6")]
            st["CAnimation.1 == CAnimation.2"] += g(a, "CAnimation.1") == first
            kinds = collections.Counter(e.type for e in envs); bones = collections.defaultdict(set)
            for e in envs:
                st["envelopes"] += 1; bone = g(e, "CAnimEnvelope.1"); bones[bone].add(e.type)
                keys = g(e, "CVec3LenAnimEnvelope.2" if e.type == "CVec3LenAnimEnvelope" else
                         "CQuatAnimEnvelope.1" if e.type == "CQuatAnimEnvelope" else "CFloatAnimEnvelope.1")
                frames = [g(k, "CAnimFrame.1") for k in keys]
                st["envelope first/last frame == animation first/last"] += (g(e, "CAnimEnvelope.2"), g(e, "CAnimEnvelope.3")) == (first, last)
                st["key count (member 4) == list length"] += g(e, "CAnimEnvelope.4") == len(keys)
                st["frames strictly ascending"] += all(x < y for x, y in zip(frames, frames[1:]))
                st["frames inside [first, last]"] += bool(frames) and first <= frames[0] and frames[-1] <= last
                st["first and last key at the animation's first and last frame"] += bool(frames) and (frames[0], frames[-1]) == (first, last)
                st["member 5 == 1 / (last - first + 1)"] += abs(g(e, "CAnimEnvelope.5") - 1 / (last - first + 1)) < 1e-6
                if e.type == "CQuatAnimEnvelope":
                    qs = [g(k, "CQuatAnimFrame.1") for k in keys]
                    if bone_len is not None and bone in bone_len:
                        q0 = [g(qs[0], f"Quaternion4f.{i}") for i in (1, 2, 3, 4)]; rq = bone_len[bone][1]
                        st["rotation envelopes with a skeleton bone"] += 1
                        st["first rotation key == skeleton rest rotation (up to sign, 1e-3)"] += min(math.dist(q0, rq), math.dist(q0, [-x for x in rq])) < 1e-3
                    st["quaternion keys unit length (1e-4)"] += all(abs(math.sqrt(sum(g(q, f"Quaternion4f.{i}") ** 2 for i in (1, 2, 3, 4))) - 1) < 1e-4 for q in qs)
                elif e.type == "CVec3LenAnimEnvelope":
                    vs = [g(k, "CVec3AnimFrame.1") for k in keys]
                    if bone_len is not None and bone in bone_len:
                        st["translation envelopes with a skeleton bone"] += 1
                        st["member 'len' x skeleton bone length == 1 (1e-3)"] += abs(g(e, "CVec3LenAnimEnvelope.1") * bone_len[bone][0] - 1) < 1e-3
                        first_v = [g(vs[0], f"Vector3f.{i}") for i in (1, 2, 3)]
                        st["first translation key == skeleton rest translation (1e-3)"] += math.dist(first_v, bone_len[bone][2]) < 1e-3
                if ids is not None:
                    tag = "float" if e.type == "CFloatAnimEnvelope" else "translation/rotation"
                    st[f"{tag} envelopes checked against a skeleton"] += 1; st[f"{tag} envelope id is a bone id of the model's skeleton"] += bone in ids
            st["bones with both a translation and a rotation envelope"] += sum(1 for b in bones.values() if {"CVec3LenAnimEnvelope", "CQuatAnimEnvelope"} <= b)
            st["bones with only one kind"] += sum(1 for b in bones.values() if len({"CVec3LenAnimEnvelope", "CQuatAnimEnvelope"} & b) == 1)
            ev = g(a, "CAnimation.8")
            if ev.oid != NULL:
                holder = objs[ev.oid]; st["animations with an event holder"] += 1
                for member, v in holder:
                    if isinstance(v, list):
                        for x in v:
                            o = objs[x.oid] if isinstance(x, Ref) else x
                            if isinstance(o, Obj):
                                evclasses[o.type] += 1; fr = g(o, "CAnimEvent.ae_fFrame")
                                st["events"] += 1; st["... event frame inside the animation's [first, last]"] += first <= fr <= last
                                st["... event stored inline in the holder list (not as a reference)"] += not isinstance(x, Ref)
    return len(sets)


def main():
    install = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder (the one containing All_PC_01.gro) or set SS2_DATA_DIR")
    st = collections.Counter(); ev = collections.Counter()
    for name, d in sorted(merged_files(install, ".ans").items()):
        r = parse(d); objs, _ = TreeDec(d, r).objects(); check_set(".ans", objs, None, st, ev)
    for name, d in sorted(merged_files(install, ".mdl").items()):
        r = parse(d); objs, _ = TreeDec(d, r).objects()
        if not any(isinstance(o, Obj) and o.type == "CAnimSet" for o in objs.values()): continue
        cfg = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CModelConfiguration" and g(o, "CResource.1") == 1)
        sk = g(cfg, "CModelConfiguration.2"); ids = None
        bl = None
        if sk.oid in objs:
            lods = g(objs[sk.oid], "CSkeleton.1"); ids = {g(b, "SkeletonBone.1") for lod in lods for b in g(lod, "CSkeletonLOD.2")}
            bl = {}
            for b in g(lods[0], "CSkeletonLOD.2"):
                qv = g(b, "SkeletonBone.4"); q = g(qv, "QuatVect.1"); t = g(qv, "QuatVect.2")
                bl[g(b, "SkeletonBone.1")] = (g(b, "SkeletonBone.6"), [g(q, f"Quaternion4f.{i}") for i in (1, 2, 3, 4)], [g(t, f"Vector3f.{i}") for i in (1, 2, 3)])
        check_set("embedded in .mdl", objs, ids, st, ev, bl)
    for k, v in sorted(st.items()): print(f"{v:9d}  {k}")
    print("event classes:", dict(ev.most_common()))


if __name__ == "__main__":
    main()
