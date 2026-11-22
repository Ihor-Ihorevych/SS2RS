"""Re-runs the measurements behind docs/re/effects-pipeline.md on an install (read-only).

1. Physical materials (`.mat`): the particle scheme (event -> weighted `.pfx`) and the base schemes.
2. Puppet parameter files: `SPuppetDeathParams` (exploding damage, explosion impulse, corpse sink) and `SPuppetEffectParams`.
3. Projectile parameter files: `bp_idEffectType`.
4. Worlds: placed `CEffectEntity` (particle effects) and the `en_pfxSplash`/`en_pfxDust` of props.
5. Whether any decal, gib or blood-splat file or class exists (names in the archives, type names in all `.mdl`/`.wld`/`.ep`).
Writes the per-material table with --table.

Usage: python -I tools/re/effects_verify.py [DATA_DIR] [LIMIT] [--table OUT.md]
       (DATA_DIR defaults to $SS2_DATA_DIR; LIMIT = only the LIMIT smallest worlds, default all, about 15 minutes)
"""
import collections, os, sys, zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Blob, Enum, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
from wld_root_verify import ids  # noqa: E402


def root_of(d):
    r = parse(d); objs, _ = TreeDec(d, r).objects()
    rs = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CResourceFile")
    return r, objs, objs[[x.oid for x in rs.get("CResourceFile.rf_crsShared")][0]]


def nested(o, typ):
    for k, v in o:
        if isinstance(v, Obj):
            if v.type == typ: yield v
            yield from nested(v, typ)


def base(p): return p.rsplit("/", 1)[-1]


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
    # 5a. names
    hits = sorted(p for p in paths if any(w in base(p) for w in ("decal", "gib", "splat", "bullethole", "scorch", "crater")))
    print("files in the archives:", len(paths), "| names containing decal/gib/splat/bullethole/scorch/crater:", len(hits), collections.Counter(p.rsplit(".", 1)[-1] for p in hits), [base(p) for p in hits][:14])
    pfx = [p for p in paths if p.endswith(".pfx")]; print(".pfx files:", len(pfx), "| in Effects/Materials:", sum(1 for p in pfx if "/effects/materials/" in p))
    # 1. materials
    mats = []
    for n, d in sorted(merged_files(install, ".mat").items()):
        r, objs, rt = root_of(d); idnt = r["idnt"]; rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}
        pr = rt.get("CSamMaterial.sm_ppsParticleScheme"); ps = objs.get(pr.oid) if isinstance(pr, Ref) else None; ev = {}
        for it in (ps.get("CParticleScheme.1") if ps is not None else []):
            it = objs[it.oid] if isinstance(it, Ref) else it
            ev[idnt[it.get("CPFXInScheme.1")]] = [(base(ex.get(w.get("CWeightedPFXInScheme.2").oid, "(embedded)")) if isinstance(w.get("CWeightedPFXInScheme.2"), Ref) else "?", w.get("CWeightedPFXInScheme.1")) for w in (objs[x.oid] if isinstance(x, Ref) else x for x in it.get("CPFXInScheme.2"))]
        b = ps.get("CParticleScheme.2") if ps is not None else None; bn = base(ex.get(b.oid, "(null)")) if isinstance(b, Ref) else ""
        sr = rt.get("CSamMaterial.sm_pssSoundScheme"); ss = objs.get(sr.oid) if isinstance(sr, Ref) else None; sb = ss.get("CSoundScheme.2") if ss is not None else None; sbn = base(ex.get(sb.oid, "(null)")) if isinstance(sb, Ref) else ""
        mats.append((base(n)[:-4], ev, bn, sbn, rt.get("CSamMaterial.sm_fMovementSpeedMultiplier"), rt.get("CSamMaterial.sm_slApplyDamageAmount")))
    evc = collections.Counter(e for m in mats for e in m[1]); print(f"\nmaterials: {len(mats)}; events {dict(evc)}; with a base particle scheme: {sum(1 for m in mats if m[2] not in ('', '(null)'))}; with no particle events: {sum(1 for m in mats if not m[1])}")
    allfx = [fx for m in mats for v in m[1].values() for fx, _ in v]; ex_ok = sum(1 for fx in allfx if any(p.endswith("/" + fx.lower()) for p in pfx))
    print(f"particle scheme entries: {len(allfx)}, distinct files {len(set(allfx))}, existing in the archives {ex_ok}; alternatives per event: {dict(collections.Counter(len(v) for m in mats for v in m[1].values()))}")
    # 2-3. puppets and projectiles
    death = []; effp = collections.defaultdict(collections.Counter); proj = collections.Counter(); nproj = 0
    for n, d in sorted(merged_files(install, ".ep").items()):
        r, objs, rt = root_of(d); rel = n.split("databases/")[-1]
        for dp in nested(rt, "SPuppetDeathParams"):
            vals = {k.split(".", 1)[-1]: (v.v if isinstance(v, Enum) else v) for k, v in dp if isinstance(v, (int, float))}
            cs = [v for k, v in dp if k.endswith("pdp_pcscCorpseSinkConfig")]
            death.append((rel, vals, isinstance(cs[0], Ref) and cs[0].oid in objs if cs else False))
        for o in objs.values():
            if isinstance(o, Obj) and any(k.endswith("bp_idEffectType") for k, _ in o):
                nproj += 1; proj[r["idnt"][[v for k, v in o if k.endswith("bp_idEffectType")][0]]] += 1
    print(f"\npuppet death parameter blocks: {len(death)}")
    for key in ("pdp_slExplodingDamage", "pdp_fExplosionForceImpulse", "pdp_fExplosionForceImpulseUpRatio", "pdp_bDisableBulletDamageAccumulation", "1", "2", "3", "4", "pdp_fStretchOverride"):
        print("  ", key.ljust(38), dict(collections.Counter(round(v.get(key, -99), 2) for _, v, _ in death).most_common(8)))
    print("   corpse sink config embedded in", sum(1 for _, _, c in death if c), "files")
    print(f"projectile parameter blocks: {nproj}; bp_idEffectType {dict(proj)}")
    # 4. worlds
    effs = collections.Counter(); props = {"en_pfxSplash": collections.Counter(), "en_pfxDust": collections.Counter()}; perworld = {}; nef = 0
    files = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))
    for n, (name, d) in enumerate(files):
        if limit and n >= limit: break
        r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects(); extra, _ = dec.extra_objects(edob); rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}
        w = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CWorld"); wn = name.rsplit("/", 1)[-1][:-4]; c = collections.Counter()
        recs = {extra[q].get("CEntityProperties.2").oid: extra[q] for q in ids(w.get("CWorld.5")) if q in extra}
        for i in ids(w.get("CWorld.1")):
            e = objs[i]
            if e.type == "CEffectEntity":
                c["effect entities"] += 1; nef += 1
                rec = recs.get(i); pv = rec.get("CEffectProperties.1") if rec is not None else None
                effs[base(ex[pv.oid]).lower() if isinstance(pv, Ref) and pv.oid in ex else "(none)"] += 1
            elif e.type == "SPropEntity":
                c["props"] += 1
                for k, v in e:
                    for key in props:
                        if k.endswith(key) and isinstance(v, Ref): props[key][base(ex.get(v.oid, "(embedded)")).lower()] += 1
        perworld[wn] = c; print(f"[{n + 1}/{len(files)}] {wn}", file=sys.stderr, flush=True)
    print(f"\nplaced effect entities: {nef} in {sum(1 for c in perworld.values() if c['effect entities'])} of {len(perworld)} worlds; distinct effect files {len(effs)}; top: {effs.most_common(12)}")
    for key, c in props.items(): print(f"{key}: {sum(c.values())} refs, distinct {len(c)}; top {c.most_common(8)}")
    if table:
        evs = [e for e, _ in evc.most_common()]
        L = ["# Material hit effects and death parameters (generated)", "",
             "Generated by `tools/re/effects_verify.py --table` from the 35 `.mat` files and the puppet parameter files of the retail install. See `effects-pipeline.md`. "
             "Cells list the effect file of the material's particle scheme for the event (several alternatives are separated by `|`, with weights in brackets when they differ); a blank cell means the material has no effect for that event.", "",
             "## Particle schemes of the physical materials", "", "| Material | Base particle scheme | Sound scheme | " + " | ".join(f"`{e}`" for e in evs) + " |", "|---|---|---|" + "---|" * len(evs)]
        for m in sorted(mats):
            cells = [" \\| ".join(f"`{fx}`" + (f" ({wt:g})" if len(m[1][e]) > 1 else "") for fx, wt in m[1].get(e, [])) for e in evs]
            L.append(f"| `{m[0]}` | {m[2] if m[2] not in ('(null)', '') else ''} | {m[3] if m[3] not in ('(null)', '') else ''} | " + " | ".join(cells) + " |")
        L += ["", "## Puppet death parameters (`SPuppetDeathParams`)", "", "Only files with a non-zero exploding damage or explosion impulse are listed.", "", "| File | Exploding damage | Explosion impulse (up ratio) | `.2` | `.3` | `.4` | Corpse sink |", "|---|---|---|---|---|---|---|"]
        for rel, v, c in sorted(death):
            if v.get("pdp_slExplodingDamage", 0) or v.get("pdp_fExplosionForceImpulse", 0):
                L.append(f"| `{rel}` | {v.get('pdp_slExplodingDamage', 0):g} | {v.get('pdp_fExplosionForceImpulse', 0):g} ({v.get('pdp_fExplosionForceImpulseUpRatio', 0):g}) | {v.get('2', 0):g} | {v.get('3', 0):g} | {v.get('4', 0):g} | {'yes' if c else ''} |")
        with open(table, "w", encoding="utf-8", newline="\n") as f: f.write("\n".join(L) + "\n")
        print("wrote", table)


if __name__ == "__main__":
    main()
