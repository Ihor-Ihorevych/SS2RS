"""Evidence for docs/re/wld-lighting.md, "Baked lightmaps" / T01-420 (read-only, SS2_DATA_DIR).

T01-414 left two residual buckets of T01-407's candidate test (a UV stream inside [0, 1] with at
most 2 percent rasterized overlap) unresolved by a real signal, only by a positional tie-break:

- **several candidates** (209 lit models, this task's primary target): T01-414's "lowest index"
  tie-break, and T07-044's "not referenced by the surface's own material" rule, both fail on
  `1_7_city_metropolis`'s `platform_03/platform.mdl` -- its two candidates are slot 0 (UV id 3,
  the lower index) and slot 2 (UV id 5); *neither* is referenced by the material (which uses ids 2
  and 4), so that rule cannot tell them apart either; rendering through slot 0 is almost black,
  through slot 2 plausibly lit.
- **no candidates** (181 lit models): T07-044 already ships "the lowest declared id the material
  does not use, else slot 0" for these, but the coordinator's visual check found the same
  almost-black pattern on the crate stacks of `6_2_military_base` under that rule.

This script tests a signal T01-407/414 never measured: how the candidate's own UV footprint
samples the instance's *own, already-baked* lightmap texture (not a synthetic quality measure of
the UV chart itself, but the actual pixel data every lit instance already carries, `tex.md`). For
every UV id the mesh declares (not only the ones that pass the [0, 1]/overlap test, so the
no-candidate bucket is covered too), it rasterizes that id's triangles into the instance's own
lightmap texture's texel grid (same rasterizer as `wld_lightmap_stream_verify.py --texels`) and
measures:

- `coverage`: the fraction of the lightmap's total texel area the id's footprint covers (a real
  unwrap should fill most of its lightmap);
- `mean_lum`: the mean texel luminance under that footprint (0..255; a baked, lit chart should be
  brighter, on average, than the near-neutral-grey gaps between charts, `wld_lightmap_stream_verify`'s
  "lit" test, mode 0x7F7F7F);
- `frac_lit`: of the footprint's own texels, the fraction that are "lit" (depart from near-grey by
  more than the existing tolerance) rather than landing on unused gap/background texels;
- `aspect_err`: how far the UV bounding box's own aspect ratio (width/height in UV space) is from
  the lightmap texture's own aspect ratio (log ratio, 0 = exact match) -- a real unwrap for a
  lightmap baked at a given size is expected to roughly match that texture's own shape;
- `overlap`: T01-407's own rasterized-overlap fraction (only meaningful, i.e. computed, for ids
  that are themselves inside [0, 1]; else `None`).

It reports, for both buckets, how often each signal's own top pick (highest `coverage`, highest
`mean_lum`, highest `frac_lit`, lowest `aspect_err`) agrees with every other signal's pick and with
the existing rules (T01-414's lowest-index, T07-044's not-material-used), and prints the per-id
numbers for the platform.mdl test vector and for every instance whose mesh path contains "crate"
in `6_2_military_base` (the coordinator's new test case) so a human can read them directly.

Usage:
  python -I tools/re/wld_lightmap_streamselect_verify.py [DATA_DIR] [N]
       (N = resolve+check instances for only the N smallest worlds, default 59 = all; this decodes
        one lightmap texture per residual-bucket instance, in addition to the mesh rasterization
        wld_lightmap_stream_verify.py already does -- about 6 minutes for all 59 worlds)
  python -I tools/re/wld_lightmap_streamselect_verify.py [DATA_DIR] --case WORLD_SUBSTR
       (print every bucketed instance's own per-id table for worlds whose path contains WORLD_SUBSTR,
        case-insensitive -- e.g. "metropolis" or "military_base")
  python -I tools/re/wld_lightmap_streamselect_verify.py [DATA_DIR] --meshes [N]
       (distinct-mesh-level agreement counts only, no lightmap texture decode needed since overlap
        and aspect error are pure UV-space measurements shared by every instance of one mesh --
        about 2.5 minutes for all 59 worlds; lists every mesh where the overlap-based pick disagrees
        with the shipped not-used-by-material rule)
"""
import collections, math, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, TreeDec, merged_files, parse  # noqa: E402
from wld_lightmap_stream_verify import (  # noqa: E402
    ModelFileIndex, analyze_mesh, candidates, decode_lightmap_texture, lod_surfaces, pc_lod,
    rasterize_coverage, resolved_mesh_for_instance, surface_triangles, surface_uv_arrays,
)

NULL = 0xFFFFFFFF
GREY_TOL = 10  # wld_lightmap_stream_verify's own "lit" tolerance around 0x7F7F7F


def id_triangles(objs, mesh, uid):
    tris = []
    for sf in lod_surfaces(pc_lod(mesh)):
        for slot, (sid, uv) in surface_uv_arrays(objs, mesh, sf).items():
            if sid != uid:
                continue
            tri_idx = surface_triangles(objs, mesh, sf)
            for t in range(0, len(tri_idx), 3):
                a, b, c = tri_idx[t:t + 3]
                tris.append((uv[a], uv[b], uv[c]))
    return tris


def id_signals(objs, mesh, uid, stats_for_id, w, h, rgba, lit_mask):
    tris = id_triangles(objs, mesh, uid)
    if not tris:
        return None
    cov = rasterize_coverage(tris, w, h)
    n_cov = sum(cov)
    if n_cov == 0:
        return None
    lum = [sum(rgba[i * 4:i * 4 + 3]) / 3 for i in range(w * h)]
    mean_lum = sum(lum[i] for i in range(w * h) if cov[i]) / n_cov
    n_lit_cov = sum(1 for i in range(w * h) if cov[i] and lit_mask[i])
    s = stats_for_id
    uv_w = s["max_u"] - s["min_u"]; uv_h = s["max_v"] - s["min_v"]
    aspect_err = None
    if uv_w > 1e-6 and uv_h > 1e-6 and h > 0:
        aspect_err = abs(math.log((uv_w / uv_h) / (w / h)))
    return {
        "coverage": n_cov / (w * h),
        "mean_lum": mean_lum,
        "frac_lit": n_lit_cov / n_cov,
        "aspect_err": aspect_err,
        "overlap": s["overlap"],
        "in01": s["in01"],
        "used_by_material": s["used_by_material"],
        "slot": s["slot"],
    }


def bucket_of(stats):
    cand = candidates(stats)
    if len(cand) >= 2:
        return "several", cand
    if len(cand) == 0:
        return "none", list(stats.keys())
    return "one", cand


def picks(sig_by_id):
    """{signal: winning uv id}. coverage/mean_lum/frac_lit/aspect_err are restricted to ids that are
    themselves inside [0, 1] (`in01`) -- a tiling channel that runs outside [0, 1] gets its UV bbox
    clamped into the grid by the rasterizer, which would otherwise fake a high "coverage" for an
    ordinary, materially-used tiling channel that has nothing to do with the lightmap unwrap."""
    in01_ids = {uid: s for uid, s in sig_by_id.items() if s["in01"]}
    out = {}
    for key, better_high in (("coverage", True), ("mean_lum", True), ("frac_lit", True)):
        avail = {uid: s[key] for uid, s in in01_ids.items() if s[key] is not None}
        if avail:
            out[key] = max(avail, key=avail.get) if better_high else min(avail, key=avail.get)
    avail = {uid: s["aspect_err"] for uid, s in in01_ids.items() if s["aspect_err"] is not None}
    if avail:
        out["aspect_err"] = min(avail, key=avail.get)
    avail = {uid: s["overlap"] for uid, s in in01_ids.items() if s["overlap"] is not None}
    out["lowest_overlap"] = min(avail, key=avail.get) if avail else None
    not_used = [uid for uid, s in sig_by_id.items() if not s["used_by_material"]]
    out["not_used_by_material"] = min(not_used, key=lambda u: sig_by_id[u]["slot"]) if not_used else None
    out["lowest_index"] = min(sig_by_id, key=lambda u: sig_by_id[u]["slot"]) if sig_by_id else None
    return out


def check_world(name, d, st, dist, cases, case_filter, index, mdl_cache, mesh_stats_cache):
    r = parse(d); objs, edob = TreeDec(d, r).objects()
    extra, _ = TreeDec(d, r).extra_objects(edob)
    allo = {**objs, **extra}
    st["worlds"] += 1
    for i, o in objs.items():
        if not (isinstance(o, Obj) and o.type == "CModelInstance" and o.get("CModelInstance.4").oid != NULL):
            continue
        ml = objs[o.get("CModelInstance.4").oid]
        m = next((allo.get(x.oid) for x in ml.get("CShaderModifierList.1")
                  if allo.get(x.oid) is not None and allo[x.oid].type == "CTextureOvenShaderModifier"), None)
        if m is None:
            continue
        st["oven modifiers (instances) seen"] += 1
        mpath, resolved = resolved_mesh_for_instance(r, o, index, mdl_cache)
        if mpath is None or resolved is None:
            st["instance mesh not resolved"] += 1
            continue
        mobjs, mesh = resolved
        key = mpath.lower()
        if key not in mesh_stats_cache:
            mesh_stats_cache[key] = analyze_mesh(mobjs, mesh)
        stats = mesh_stats_cache[key]
        if stats is None:
            st["mesh had no PC-eligible LOD or no surfaces"] += 1
            continue
        bucket, ids = bucket_of(stats)
        st[f"bucket: {bucket}"] += 1
        if bucket == "one":
            continue
        w, h, rgba = decode_lightmap_texture(objs, extra, m)
        if w is None:
            st[f"{bucket}: lightmap texture not decoded"] += 1
            continue
        lit_mask = [not all(abs(rgba[i * 4 + c] - 127) <= GREY_TOL for c in range(3)) for i in range(w * h)]
        oven_id = m.get("CTextureOvenShaderModifier.3").get("CShaderArgValUVMap.1")
        sig_by_id = {}
        for uid in ids:
            sig = id_signals(mobjs, mesh, uid, stats[uid], w, h, rgba, lit_mask)
            if sig is not None:
                sig_by_id[uid] = sig
        if len(sig_by_id) < 2:
            st[f"{bucket}: fewer than two ids yielded signals"] += 1
            continue
        st[f"{bucket}: instances with full signal set"] += 1
        pk = picks(sig_by_id)
        agree_key = pk.get("lowest_overlap")
        for other in ("coverage", "mean_lum", "frac_lit", "aspect_err", "not_used_by_material", "lowest_index"):
            if pk.get(other) is not None and agree_key is not None:
                st[f"{bucket}: overlap-pick agrees with {other}-pick"] += (pk[other] == agree_key)
                st[f"{bucket}: {other}-pick total comparable"] += 1
        if oven_id is not None and agree_key is not None:
            dist[f"{bucket}: (oven id - overlap-pick id) per world (sample)"][oven_id - agree_key] += 1
        dist[f"{bucket}: overlap-pick's own slot index"][sig_by_id[agree_key]["slot"] if agree_key in sig_by_id else None] += 1
        st[f"{bucket}: overlap-pick exists"] += agree_key is not None
        if agree_key is not None:
            dist[f"{bucket}: overlap-pick's own overlap value (rounded %)"][round(100 * sig_by_id[agree_key]["overlap"], 1)] += 1
        if case_filter and case_filter in name.lower():
            cases.append((name, i, mpath, bucket, oven_id, sig_by_id, pk))


def run(install, limit, case_filter):
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter)
    cases = []
    index = ModelFileIndex(install)
    mdl_cache = {}; mesh_stats_cache = {}
    worlds = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))[:limit]
    t0 = __import__("time").time()
    for n, (name, d) in enumerate(worlds):
        if case_filter and case_filter not in name.lower():
            continue
        check_world(name, d, st, dist, cases, case_filter, index, mdl_cache, mesh_stats_cache)
        if os.environ.get("WLD_PROGRESS"):
            print(f"[{n + 1}/{len(worlds)}] {name} done at {__import__('time').time() - t0:.1f}s", file=sys.stderr)
    for k, v in sorted(st.items()):
        print(f"{v:9d}  {k}")
    for k, v in sorted(dist.items()):
        print(k, dict(v.most_common(20)))
    for name, iid, mpath, bucket, oven_id, sig_by_id, pk in cases:
        print(f"\n=== {name} instance #{iid} ({mpath}) bucket={bucket} oven_uv_id={oven_id} ===")
        for uid, s in sorted(sig_by_id.items()):
            print(f"  id {uid:3d} slot {s['slot']}: coverage={s['coverage']:.3f} mean_lum={s['mean_lum']:.1f} "
                  f"frac_lit={s['frac_lit']:.3f} aspect_err={s['aspect_err']} overlap={s['overlap']} "
                  f"used_by_material={s['used_by_material']}")
        print(f"  picks: {pk}")


def mesh_pick(stats):
    """Mesh-only picks (no lightmap texture needed): overlap/aspect_err are pure UV-space
    measurements from `analyze_mesh`, so this is identical for every instance sharing one mesh."""
    bucket, ids = bucket_of(stats)
    if bucket == "one":
        return bucket, None, None
    in01 = {uid: stats[uid] for uid in ids if stats[uid]["in01"]}
    overlap_pick = min(in01, key=lambda u: in01[u]["overlap"]) if in01 else None
    not_used = [uid for uid in ids if not stats[uid]["used_by_material"]]
    material_pick = min(not_used, key=lambda u: stats[u]["slot"]) if not_used else None
    index_pick = min(ids, key=lambda u: stats[u]["slot"]) if ids else None
    return bucket, overlap_pick, {"material": material_pick, "index": index_pick}


def run_meshes(install, limit):
    """Distinct-mesh-level agreement counts for the several/none buckets -- no lightmap texture
    decode, so much cheaper than `run()`; comparable to T01-414's own distinct-mesh counts."""
    index = ModelFileIndex(install)
    mdl_cache = {}; mesh_stats_cache = {}; mesh_bucket = {}
    worlds = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))[:limit]
    t0 = __import__("time").time()
    for n, (name, d) in enumerate(worlds):
        r = parse(d); objs, edob = TreeDec(d, r).objects()
        for i, o in objs.items():
            if not (isinstance(o, Obj) and o.type == "CModelInstance" and o.get("CModelInstance.4").oid != NULL):
                continue
            ml = objs[o.get("CModelInstance.4").oid]
            has_oven = any(objs.get(x.oid) is not None and objs[x.oid].type == "CTextureOvenShaderModifier"
                           for x in ml.get("CShaderModifierList.1"))
            if not has_oven:
                continue
            mpath, resolved = resolved_mesh_for_instance(r, o, index, mdl_cache)
            if mpath is None or resolved is None:
                continue
            mobjs, mesh = resolved
            key = mpath.lower()
            if key not in mesh_stats_cache:
                mesh_stats_cache[key] = analyze_mesh(mobjs, mesh)
            stats = mesh_stats_cache[key]
            if stats is None or key in mesh_bucket:
                continue
            mesh_bucket[key] = (mpath, *mesh_pick(stats))
        if os.environ.get("WLD_PROGRESS"):
            print(f"[{n + 1}/{len(worlds)}] {name} done at {__import__('time').time() - t0:.1f}s", file=sys.stderr)
    for bucket_name in ("several", "none"):
        rows = [(p, ov, old) for p, b, ov, old in mesh_bucket.values() if b == bucket_name]
        print(f"\n{bucket_name}: {len(rows)} distinct meshes")
        has_overlap_pick = sum(1 for _, ov, _ in rows if ov is not None)
        print(f"  overlap-pick exists (some id in01): {has_overlap_pick}")
        agree_mat = sum(1 for _, ov, old in rows if ov is not None and old["material"] == ov)
        agree_idx = sum(1 for _, ov, old in rows if ov is not None and old["index"] == ov)
        comparable_mat = sum(1 for _, ov, old in rows if ov is not None and old["material"] is not None)
        print(f"  overlap-pick agrees with lowest-index pick: {agree_idx}/{has_overlap_pick}")
        print(f"  overlap-pick agrees with not-used-by-material pick: {agree_mat}/{comparable_mat}")
        disagree = [(p, ov, old) for p, ov, old in rows if ov is not None and old["material"] != ov]
        print(f"  disagreements with not-used-by-material pick ({len(disagree)}):")
        for p, ov, old in disagree[:40]:
            print(f"    {p}: overlap-pick={ov} material-pick={old['material']} index-pick={old['index']}")


def main():
    args = sys.argv[1:]
    install = args[0] if args and os.path.isdir(args[0]) else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder (the one containing All_PC_01.gro) or set SS2_DATA_DIR")
    if "--meshes" in args:
        limit = int(args[-1]) if args and args[-1].isdigit() else 59
        run_meshes(install, limit)
        return
    case_filter = None
    if "--case" in args:
        case_filter = args[args.index("--case") + 1].lower()
        limit = 59
    else:
        limit = int(args[-1]) if args and args[-1].isdigit() else 59
    run(install, limit, case_filter)


if __name__ == "__main__":
    main()
