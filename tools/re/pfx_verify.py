"""Re-runs the measurements behind docs/re/pfx.md on an install (read-only).

For every format-8 `.pfx`: the CEffect root, the particle effects it holds, the emitter shape of every
machine effect, the constant-or-graph parameters (CGraphHolder), the enumerations in use and the textures.
(`python -I tools/re/ep_schema.py --check --ext .pfx` checks the roots and the references.)

Usage: python -I tools/re/pfx_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR)
"""
import collections, os, sys, zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Blob, Enum, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402

NULL = 0xFFFFFFFF
EFFECT_KINDS = ("CMachineParticleEffect", "CCloudsParticleEffect", "CAtomicParticleEffect")


def ids(v): return [x.oid for x in (v or []) if isinstance(x, Ref) and x.oid != NULL]


def main():
    install = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter); holders = collections.Counter(); graph_kinds = collections.Counter()
    for name, d in sorted(merged_files(install, ".pfx").items()):
        r = parse(d); objs, _ = TreeDec(d, r).objects(); st["effects"] += 1
        eff = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CEffect")
        parts = ids(eff.get("CEffect.5")); kinds = [objs[i].type for i in parts]
        st["every held object is one of the three particle effect kinds"] += all(k in EFFECT_KINDS for k in kinds)
        dist["particle effects per CEffect"][min(len(parts), 8)] += 1
        for k in kinds: dist["particle effect kind"][k] += 1
        # every particle effect object of the file is held by the root effect
        st["all particle effect objects of the file are held by CEffect.5"] += sorted(parts) == sorted(i for i, o in objs.items() if isinstance(o, Obj) and o.type in EFFECT_KINDS)
        dist["EffectLifeType"][eff.get("CEffect.12").v] += 1
        for i in parts:
            p = objs[i]; tex = dict(p.get("CParticleEffect.3")); rend = dict(p.get("CParticleEffect.4"))
            dist["orientation"][p.get("CParticleEffect.11").v] += 1; dist["distance acceptance flags"][p.get("CParticleEffect.7").v] += 1
            dist["blend type (GfuBlendType)"][rend["CParticlesRendering.1"].v] += 1; dist["render order"][rend["CParticlesRendering.5"].v] += 1
            dist["atlas columns x rows (CParticleEffectTexture.2, .3)"][(tex["CParticleEffectTexture.2"], tex["CParticleEffectTexture.3"])] += 1
            t = tex["CParticleEffectTexture.1"]; st["particle texture references"] += 1; st["... null"] += t.oid == NULL
            if p.type == "CMachineParticleEffect":
                life = dict(p.get("CMachineParticleEffect.1")); shape = objs[life["CMachineParticleEffectLife.21"].oid]
                st["machine effects"] += 1; dist["emitter shape"][shape.type] += 1; dist["particle shape (ParticleShape)"][p.get("CMachineParticleEffect.5").v] += 1
                st["every machine effect has an emitter shape (sphere, box or cylinder)"] += shape.type in ("CSphereEmitterShape", "CBoxEmitterShape", "CCylinderEmitterShape")
        # constant-or-graph parameters
        def walk(v, owner):
            if isinstance(v, Obj):
                if v.type == "CGraphHolder":
                    g = v.get("CGraphHolder.gh_pgrGraph"); holders["graph holders"] += 1
                    if g.oid != NULL: holders["... with a graph"] += 1; graph_kinds[objs[g.oid].type] += 1
                else:
                    for _, x in v: walk(x, v)
            elif isinstance(v, list) and not isinstance(v, Blob):
                for x in v:
                    walk(x, owner)
        for o in objs.values():
            if isinstance(o, Obj) and o.type != "CResourceFile":
                for _, v in o: walk(v, o)
        for i, o in objs.items():
            if isinstance(o, Obj) and o.type.endswith("Graph") and o.type != "CGraph":
                for k, v in o:
                    pts = v if isinstance(v, list) and not isinstance(v, Blob) else ([v] if isinstance(v, Obj) and v.type == "CControlPoint" else [])
                    for pt in pts:
                        if isinstance(pt, Obj) and pt.type == "CControlPoint": st["graph control points"] += 1
    for k, v in sorted(st.items()): print(f"{v:8d}  {k}")
    print("graph holders:", dict(holders), "| graph kinds behind them:", dict(graph_kinds))
    for k, v in sorted(dist.items()): print(k, dict(v.most_common(10)))


if __name__ == "__main__":
    main()
