"""Re-runs the measurements behind docs/re/mat.md on an install (read-only).

Physical materials (`.mat`, root `CSamMaterial`), the scheme files (`.scs` sound, `.scp` particle, `.sca`
animation, `.mas` moving animation) and how materials and models refer to them.

Usage: python -I tools/re/mat_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR)
"""
import collections, os, sys, zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Blob, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402

NULL = 0xFFFFFFFF


def load(install, ext):
    out = {}
    for name, d in sorted(merged_files(install, ext).items()):
        r = parse(d); objs, edob = TreeDec(d, r).objects(); out[name] = (d, r, objs)
    return out


def ext_map(r):
    rf = {e[0]: e[2] for e in r["rfil"]}
    return {e[0]: rf[e[1]] for e in r["exob"]}


def root_class(objs):
    rf = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CResourceFile")
    return [objs[x.oid].type for x in rf.get("CResourceFile.rf_crsShared")]


def main():
    install = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    paths = set()
    for f in os.listdir(install):
        if f.lower().endswith(".gro"):
            with zipfile.ZipFile(os.path.join(install, f)) as z: paths |= {i.filename.lower() for i in z.infolist() if not i.is_dir()}
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter)
    for e in (".mat", ".scs", ".scp", ".sca", ".mas"):
        files = load(install, e)
        st[f"{e} files"] += len(files)
        st[f"{e}: one shared root of class " + {".mat": "CSamMaterial", ".scs": "CSoundScheme", ".scp": "CParticleScheme", ".sca": "CAnimationScheme", ".mas": "CMovingAnimationScheme"}[e]] += sum(
            root_class(o) == [{".mat": "CSamMaterial", ".scs": "CSoundScheme", ".scp": "CParticleScheme", ".sca": "CAnimationScheme", ".mas": "CMovingAnimationScheme"}[e]] for _, _, o in files.values())
        for name, (d, r, objs) in files.items():
            for x in r["rfil"]:
                st[f"{e}: external references"] += 1; st[f"{e}: ... resolve to a file of the install"] += x[2].lower() in paths or x[2].lower().startswith("bin/")
        if e == ".mat":
            for name, (d, r, objs) in files.items():
                ex = ext_map(r); idn = r["idnt"]; m = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CSamMaterial")
                for k in (1, 2, 3, 4): dist[f"CMaterialAttributes.{k}"][round(m.get(f"CMaterialAttributes.{k}"), 3)] += 1
                dist["speed multiplier"][round(m.get("CSamMaterial.sm_fMovementSpeedMultiplier"), 2)] += 1
                if m.get("CSamMaterial.sm_slApplyDamageAmount"): dist["damaging materials (name, damage, period)"][(name.rsplit("/", 1)[-1], m.get("CSamMaterial.sm_slApplyDamageAmount"), m.get("CSamMaterial.sm_fDamageApplyPeriod"))] += 1
                for key, member in (("particle", "CSamMaterial.sm_ppsParticleScheme"), ("sound", "CSamMaterial.sm_pssSoundScheme")):
                    ref = m.get(member); dist[f"{key} scheme"]["embedded" if ref.oid in objs else "external " + ext_map(r).get(ref.oid, "?").rsplit(".", 1)[-1] if ref.oid in ex else "null"] += 1
                    if ref.oid in objs:
                        s = objs[ref.oid]; par = s.get(s.type + ".2")
                        dist[f"{key} scheme: member .2"]["null" if par.oid == NULL else "external ." + ex[par.oid].rsplit(".", 1)[-1] if par.oid in ex else "object"] += 1
                        for item in s.get("CParticleScheme.1" if key == "particle" else "CSoundScheme.1"):
                            ev = idn[item[0][1]]; dist[f"{key} event names"][ev] += 1; st["scheme event ids are valid IDNT indices"] += 1
        if e == ".scs":
            for name, (d, r, objs) in files.items():
                s = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CSoundScheme"); idn = r["idnt"]
                st["scs: scheme has no parent (.2 null)"] += s.get("CSoundScheme.2").oid == NULL
                for item in s.get("CSoundScheme.1"):
                    dist["scs event names"][idn[item[0][1]]] += 1; dist["sounds per event"][min(len(item[1][1]), 6)] += 1
                    for w in item[1][1]:
                        m = dict(w); dist["SndFlags"][hex(m["CWeightedSoundInScheme.3"].v)] += 1
                        for k in "456": dist[f"CWeightedSoundInScheme.{k} range"][(round(m["CWeightedSoundInScheme." + k], 2))] += 0
    # who refers to materials and schemes
    for ext in (".mtr", ".bmf", ".mdl"):
        for name, (d, r, objs) in load(install, ext).items():
            ex = ext_map(r)
            for o in objs.values():
                if isinstance(o, Obj) and o.type == "CShaderPreset":
                    p = o.get("CShaderPreset.2"); dist["preset physical material (.2)"]["external .mat" if p.oid in ex else "embedded" if p.oid in objs else "null"] += 1
                    if p.oid in ex: dist["most used .mat files"][ex[p.oid].rsplit("/", 1)[-1].lower()] += 1
                if isinstance(o, Obj) and o.type == "CSamCustomModelData":
                    for k, v in o:
                        if isinstance(v, Ref) and v.oid != NULL: dist["CSamCustomModelData member -> target"][(k, "external ." + ex[v.oid].rsplit(".", 1)[-1].lower() if v.oid in ex else objs[v.oid].type)] += 1
    for k, v in sorted(st.items()): print(f"{v:7d}  {k}")
    for k, v in sorted(dist.items()):
        v = collections.Counter({a: b for a, b in v.items() if b})
        if v: print(k, dict(v.most_common(10)))


if __name__ == "__main__":
    main()
