"""Re-runs the measurements behind docs/re/destruction.md on an install (read-only).

Phase A: every `.mdl`: the destruction holder (`CModelConfiguration.14`), its configurations
(`CDestructionConfig`), the custom data (health, body type, damage), the effects (`CDFX…`: debris models,
particle effects, sounds, shakers) and whether every referenced file exists.
Phase B: every `.wld`: which placed model renderables use a model that has destruction configurations.

Usage: python -I tools/re/destruction_verify.py [DATA_DIR] [LIMIT] [--table OUT.md]
       (DATA_DIR defaults to $SS2_DATA_DIR; LIMIT = only the LIMIT smallest worlds in phase B, default all, about 10 minutes)
"""
import collections, os, sys, zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Blob, Enum, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
from wld_root_verify import ids  # noqa: E402

NULL = 0xFFFFFFFF


def cfg_summary(objs, r, ex, idnt, c):
    data = c.get("CDestructionConfig.4"); d = objs.get(data.oid) if isinstance(data, Ref) else None
    fx = [objs[x.oid] for x in c.get("CDestructionConfig.3") if isinstance(x, Ref) and x.oid in objs]
    return {"name": idnt[c.get("CDestructionConfig.1")], "platform": c.get("CDestructionConfig.9").v if hasattr(c.get("CDestructionConfig.9"), "v") else c.get("CDestructionConfig.9"),
            "sub": c.get("CDestructionConfig.2"), "data": d, "fx": fx, "shake": idnt[c.get("CDestructionConfig.5")], "anim": idnt[c.get("CDestructionConfig.8")], "flag7": c.get("CDestructionConfig.7")}


def main():
    args = sys.argv[1:]; table = args[args.index("--table") + 1] if "--table" in args else None
    if table: args = [a for a in args if a not in ("--table", table)]
    install = args[0] if args and not args[0].isdigit() else os.environ.get("SS2_DATA_DIR", "")
    limit = int(args[-1]) if args and args[-1].isdigit() else None
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    paths = set()
    for f in os.listdir(install):
        if f.lower().endswith(".gro"):
            with zipfile.ZipFile(os.path.join(install, f)) as z: paths |= {i.filename.lower() for i in z.infolist() if not i.is_dir()}
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter); has = {}; rows = []
    mdls = merged_files(install, ".mdl")
    for name, d in sorted(mdls.items()):
        r = parse(d); objs, edob = TreeDec(d, r).objects(); idnt = r["idnt"]
        rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}
        mc = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CModelConfiguration")
        st["models"] += 1
        h = mc.get("CModelConfiguration.14")
        if not isinstance(h, Ref) or h.oid not in objs: st["models without a destruction holder"] += 1; continue
        holder = objs[h.oid]; cfgs = [objs[x.oid] for x in holder.get("CDestructionHolder.1") if isinstance(x, Ref) and x.oid in objs]
        st["models with a holder"] += 1; dist["configs per holder"][len(cfgs)] += 1
        if not cfgs: continue
        st["models with at least one configuration"] += 1; has[name.lower()] = [idnt[c.get("CDestructionConfig.1")] for c in cfgs]
        row = {"model": name, "configs": []}
        for c in cfgs:
            s = cfg_summary(objs, r, ex, idnt, c); st["configurations"] += 1
            dist["configuration name"][s["name"]] += 1; dist["platform mask (EPlatform)"][hex(s["platform"]) if isinstance(s["platform"], int) else s["platform"]] += 1
            sub = s["sub"]; kind = "null" if sub.oid == NULL else ("external .mdl" if sub.oid in ex else "embedded model")
            dist["substitute model (.2)"][kind] += 1
            if sub.oid in ex: st["substitute models that exist"] += ex[sub.oid].lower() in paths; st["substitute models (external)"] += 1
            d_ = s["data"]
            if d_ is not None:
                st["configurations with custom data (.4)"] += 1; dist["custom data class"][d_.type] += 1
                if d_.type == "CModelDestructionCustomData":
                    dist["body type (ModelBodyType)"][d_.get("CModelDestructionCustomData.mdcd_mbtBodyType").v] += 1
                    dist["damage type (EDamageType)"][d_.get("CModelDestructionCustomData.mdcd_dtDamageType").v] += 1
                    hp = d_.get("CModelDestructionCustomData.mdcd_slHealth"); dist["health"][hp if hp in (-1, 0) else (">0: " + ("<=100" if hp <= 100 else "<=1000" if hp <= 1000 else ">1000"))] += 1
            fxc = collections.Counter(x.type for x in s["fx"]); row["configs"].append((s["name"], dict(fxc), d_.get("CModelDestructionCustomData.mdcd_slHealth") if d_ is not None and d_.type == "CModelDestructionCustomData" else None))
            for x in s["fx"]:
                dist["effect (CDFX) class"][x.type] += 1
                if x.type == "CDFXDebris":
                    for it in x.get("CDFXDebris.1"):
                        m = it.get("CWeightedModel.2")
                        st["debris model references"] += 1
                        if isinstance(m, Ref) and m.oid in ex: st["... external and existing"] += ex[m.oid].lower() in paths
                elif x.type == "CDFXParticles":
                    for it in x.get("CDFXParticles.1"):
                        e = it.get("CWeightedParticlesEffect.2")
                        st["particle effect references"] += 1
                        if isinstance(e, Ref) and e.oid in ex: st["... external and existing (.pfx)"] += ex[e.oid].lower() in paths and ex[e.oid].lower().endswith(".pfx")
                elif x.type == "CDFXSound":
                    sn = x.get("CDFXSound.1"); st["sound references"] += 1
                    if isinstance(sn, Ref) and sn.oid in ex: st["... external and existing"] += ex[sn.oid].lower() in paths
        rows.append(row)
    print(f"phase A: {st['models']} models")
    for k, v in sorted(st.items()): print(f"{v:9d}  {k}")
    for k, v in sorted(dist.items()): print(k, dict(v.most_common(14)))
    # phase B
    sb = collections.Counter(); db = collections.defaultdict(collections.Counter)
    files = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))
    for n, (name, d) in enumerate(files):
        if limit and n >= limit: break
        r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects(); rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}
        mr = {i: o for i, o in objs.items() if isinstance(o, Obj) and o.type == "CModelRenderable"}
        insts = {i: o for i, o in objs.items() if isinstance(o, Obj) and o.type == "CModelInstance"}
        for i, rd in mr.items():
            inst = None
            for k, v in rd:
                if isinstance(v, Ref) and v.oid in insts: inst = insts[v.oid]; break
                if isinstance(v, Obj):
                    for kk, vv in v:
                        if isinstance(vv, Ref) and vv.oid in insts: inst = insts[vv.oid]; break
            if inst is None: continue
            m = inst.get("CModelInstance.3"); path = ex.get(m.oid, "").lower() if isinstance(m, Ref) else ""
            own = rd.get("CAspect.5").oid; oc = objs[own].type if own in objs else "(none)"
            sb["model renderables with a model"] += 1; db["owner class: all"][oc] += 1
            if path in has: sb["... whose model has destruction configurations"] += 1; db["owner class: with destruction"][oc] += 1; db["first configuration name"][has[path][0]] += 1
        print(f"[{n + 1}/{len(files)}] {name.rsplit('/', 1)[-1]}", file=sys.stderr, flush=True)
    print("phase B")
    for k, v in sorted(sb.items()): print(f"{v:9d}  {k}")
    for k, v in sorted(db.items()): print(k, dict(v.most_common(16)))
    if table:
        L = ["# Models with destruction configurations (generated)", "", "Generated by `tools/re/destruction_verify.py --table`. One row per model that has at least one `CDestructionConfig`: the configuration names with the number of effects by class (`D` debris, `P` particles, `S` sound, `K` shaker) and the custom-data health (`-` none). See `destruction.md`.", "", "| Model | Configurations (name: effects, health) |", "|---|---|"]
        short = {"CDFXDebris": "D", "CDFXParticles": "P", "CDFXSound": "S", "CDFXShaker": "K"}
        for r_ in sorted(rows, key=lambda r: r["model"]):
            L.append(f"| `{r_['model'].replace('content/serioussam2/', '')}` | " + "; ".join(f"{n or '(empty)'}: " + " ".join(f"{short.get(k, k)}{v}" for k, v in sorted(fx.items())) + (f", {h}" if h is not None else ", -") for n, fx, h in r_["configs"]) + " |")
        with open(table, "w", encoding="utf-8", newline="\n") as f: f.write("\n".join(L) + "\n")
        print("wrote", table)


if __name__ == "__main__":
    main()
