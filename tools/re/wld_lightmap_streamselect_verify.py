"""Evidence for docs/re/wld-lighting.md, "Baked lightmaps" / T01-420 (read-only, SS2_DATA_DIR).

T01-414 left two residual buckets of T01-407's candidate test (a UV stream inside [0, 1] with at
most 2 percent rasterized overlap) unresolved by a real signal, only by a positional tie-break:

- **several candidates** (this task's primary target): T01-414's "lowest index" tie-break, and
  T07-044's "not referenced by the surface's own material" rule, both fail on
  `1_7_city_metropolis`'s `platform_03/platform.mdl` -- its two candidates are slot 0 (UV id 3,
  the lower index) and slot 2 (UV id 5); *neither* is referenced by the material (which uses ids 2
  and 4), so that rule cannot tell them apart either; rendering through slot 0 is almost black,
  through slot 2 plausibly lit.
- **no candidates**: T07-044 already ships "the lowest declared id the material does not use,
  else slot 0" for these, but the coordinator's visual check found the same almost-black pattern
  on the crate-stack models of `6_2_military_base` under that rule.

**This task's own first pass got the hard case right for the wrong reason.** Its overlap numbers
did not reproduce the shipped loader's (`crates/ss2-world/src/load/lightmap.rs`): that code's
`rasterize`/`overlap` is a correct, fixed-256-grid, top-left-rule rasterizer, while this script
originally called `wld_lightmap_stream_verify.py`'s own `overlap_fraction` (T01-407/414's), which
sizes its grid dynamically by triangle count and tests a pixel centre against all three of a
triangle's edges with an inclusive ">= 0" test -- which **double-counts** every pixel centre that
falls exactly on an edge shared by two triangles of the same chart, both as "inside A" and "inside
B". A finely, evenly subdivided unwrap (many internal shared edges, often grid-aligned to simple
UV fractions, exactly `platform_03/platform.mdl`'s slot 0/UV id 3) hits this often enough at a 256
grid to read a few tenths of a percent of spurious "overlap" even though no two triangles'
interiors truly overlap anywhere -- which is why this script's own first-pass numbers (0.79
percent for id 3, 0.0 percent for id 5) looked like they discriminated the hard case, while the
shipped loader (and `--meshes` below, now a faithful, line-by-line port of `rasterize`/`overlap`,
confirmed against the loader's own unit test `shared_edges_and_the_small_overlap_allowance`)
reports **0.0 percent for both** -- a genuine tie, re-checked at both a 256 and a 1024 grid.

**Over real meshes, a correctly measured overlap fraction rarely discriminates several candidates
at all**: a real lightmap unwrap has no true triangle overlap anywhere, so every genuine candidate
typically reads exactly 0.0 percent once the shared-edge bug is fixed -- ties, not close calls, are
the normal case among several candidates, not a rare edge case. The adopted rule (`rule_pick`,
below) therefore uses overlap only as its first stage, and breaks a tie by a second, independent
signal measured against the instance's own *already-baked* lightmap texture (not a synthetic
measure of the UV chart, but the actual pixel data every lit instance already carries, `tex.md`):
for every UV id the mesh declares, it rasterizes that id's triangles into the instance's own
lightmap texture's texel grid (same rasterizer as `wld_lightmap_stream_verify.py --texels`) and
measures:

- `frac_lit`: of the footprint's own texels, the fraction that are "lit" (depart from near-grey by
  more than the existing tolerance) rather than landing on unused gap/background texels -- the
  tie-break stage the adopted rule actually uses;
- `coverage`, `mean_lum`, `aspect_err`: three further signals (texel-area coverage, mean texel
  luminance, UV/lightmap aspect-ratio match) tested and **not** adopted -- see "Baked lightmaps"
  in `wld-lighting.md` for why (coverage is confounded by a tiling-channel rasterization artifact,
  raw luminance is anti-correlated with the correct pick on the hard case).

The adopted rule, `rule_pick`: among a mesh's declared UV ids, restrict to the ones inside [0, 1]
(within the loader's own [`PRODUCT_RANGE_EPS`]); if exactly one has the lowest [`overlap_exact`]
fraction, pick it (stage `"overlap"`); on a tie, pick the one with the highest `frac_lit` (stage
`"frac_lit"`); on a further tie, fall back to the existing order -- lowest id unused by the
surface's own material, then lowest slot index (stage `"material/index"`).

Usage:
  python -I tools/re/wld_lightmap_streamselect_verify.py [DATA_DIR] [N]
       (the adopted rule, full agreement/stage statistics against the existing rules, over every
        residual-bucket instance; N = only the N smallest worlds, default 59 = all; decodes one
        lightmap texture per residual-bucket instance -- about 9 minutes for all 59 worlds)
  python -I tools/re/wld_lightmap_streamselect_verify.py [DATA_DIR] --case WORLD_SUBSTR
       (print every bucketed instance's own per-id table and the rule's own pick for worlds whose
        path contains WORLD_SUBSTR, case-insensitive -- e.g. "metropolis" or "military_base")
  python -I tools/re/wld_lightmap_streamselect_verify.py [DATA_DIR] --meshes [N]
       (cheap, no-texture-decode pass: distinct-mesh bucket sizes under the corrected overlap
        measure, and how often overlap alone already resolves several candidates to one versus
        ties -- it is not the adopted rule by itself wherever it reports a tie -- about 2.5 minutes
        for all 59 worlds)
"""
import collections, math, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, TreeDec, merged_files, parse  # noqa: E402
from wld_lightmap_stream_verify import (  # noqa: E402
    ModelFileIndex, decode_lightmap_texture, lod_surfaces, material_uv_ids, pc_lod,
    rasterize_coverage, resolved_mesh_for_instance, surface_triangles, surface_uv_arrays,
)

NULL = 0xFFFFFFFF
GREY_TOL = 10  # wld_lightmap_stream_verify's own "lit" tolerance around 0x7F7F7F

# --- exact, product-matching overlap measure -------------------------------------------------
# `tools/re/wld_lightmap_stream_verify.py`'s own `overlap_fraction` (used by T01-407/414) sizes
# its grid dynamically (16..256 by triangle count) and tests a pixel centre against each edge
# with an inclusive ">= 0" half-space test on all three edges at once -- which double-counts
# every pixel centre that happens to land exactly on an edge shared by two triangles of the same
# chart, both as "inside triangle A" and "inside triangle B". A finely, evenly subdivided unwrap
# (many internal shared edges, often grid-aligned to simple UV fractions) can hit this often
# enough at a 256 grid to read a few tenths of a percent of spurious "overlap" even though no two
# triangles' interiors truly overlap anywhere. This is why this task's own numbers initially did
# not reproduce the shipped loader's (`crates/ss2-world/src/load/lightmap.rs`, `rasterize`):
# that code is a correct, textbook top-left-rule rasterizer, which breaks every such tie in
# favour of exactly one of the two adjacent triangles. The two functions below are a line-by-line
# port of `rasterize`/`overlap`/`cross`/`cell`/`inside` from that file (same fixed grid of
# [`PRODUCT_GRID`] cells, same [`PRODUCT_RANGE_EPS`] inside-[0,1] tolerance, same `u32`/pixel-centre
# rasterization), used for every overlap number this script reports.
PRODUCT_GRID = 256
PRODUCT_RANGE_EPS = 1e-3  # lightmap.rs's RANGE_EPS (T01-407/414's own scripts use 2e-3)
MAX_OVERLAP = 0.02  # lightmap.rs's MAX_OVERLAP, unchanged


def _cross(a, b, c):
    return (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])


def _rasterize_exact(counts, grid, p):
    """Add one to every cell whose centre lies in triangle `p` (grid-unit coordinates), breaking
    every edge shared by two triangles in favour of exactly one of them (the top-left rule) --
    a direct port of `lightmap.rs`'s `rasterize`."""
    area = _cross(p[0], p[1], p[2])
    if abs(area) < 1e-12:
        return
    if area < 0.0:
        p = (p[0], p[2], p[1])
    xs = [q[0] for q in p]; ys = [q[1] for q in p]

    def cell(x):
        return max(0, min(grid, int(math.ceil(x - 0.5))))

    x0 = cell(min(xs)); x1 = min(cell(max(xs)), grid - 1)
    y0 = cell(min(ys)); y1 = min(cell(max(ys)), grid - 1)
    edges = ((p[1], p[2]), (p[2], p[0]), (p[0], p[1]))
    top_left = []
    for a, b in edges:
        dx = b[0] - a[0]; dy = b[1] - a[1]
        top_left.append(dy < 0.0 or (dy == 0.0 and dx < 0.0))
    for y in range(y0, y1 + 1):
        row = y * grid
        for x in range(x0, x1 + 1):
            c = (x + 0.5, y + 0.5)
            ok = True
            for (a, b), tl in zip(edges, top_left):
                w = _cross(a, b, c)
                if not (w > 0.0 or (w == 0.0 and tl)):
                    ok = False
                    break
            if ok:
                counts[row + x] += 1


def overlap_exact(tris, grid=PRODUCT_GRID):
    """The share of covered cells covered by two or more triangles, `lightmap.rs`'s `overlap`."""
    counts = [0] * (grid * grid)
    for (u0, v0), (u1, v1), (u2, v2) in tris:
        p = ((u0 * grid, v0 * grid), (u1 * grid, v1 * grid), (u2 * grid, v2 * grid))
        _rasterize_exact(counts, grid, p)
    covered = sum(1 for c in counts if c > 0)
    if covered == 0:
        return 0.0
    twice = sum(1 for c in counts if c > 1)
    return twice / covered


def exact_analyze_mesh(objs, mesh, grid=PRODUCT_GRID):
    """Per-UV-id stats for the mesh's first PC-eligible LOD, using [`overlap_exact`] -- the
    overlap-sensitive, product-matching twin of `wld_lightmap_stream_verify.analyze_mesh`."""
    lod = pc_lod(mesh)
    if lod is None:
        return None
    surfaces = lod_surfaces(lod)
    if not surfaces:
        return None
    per_id = collections.defaultdict(lambda: {"pts": [], "tris": [], "surfaces": set(), "slots": collections.Counter()})
    material_ids = set()
    for si, sf in enumerate(surfaces):
        material_ids |= material_uv_ids(objs, sf)
        tri_idx = surface_triangles(objs, mesh, sf)
        if not tri_idx:
            continue
        for slot, (uid, uv) in surface_uv_arrays(objs, mesh, sf).items():
            e = per_id[uid]
            e["pts"].extend(uv)
            for t in range(len(tri_idx) // 3):
                a, b, c = tri_idx[3 * t:3 * t + 3]
                e["tris"].append((uv[a], uv[b], uv[c]))
            e["surfaces"].add(si)
            e["slots"][slot] += 1
    result = {}
    for uid, e in per_id.items():
        pts = e["pts"]
        if not pts:
            continue
        us = [p[0] for p in pts]; vs = [p[1] for p in pts]
        min_u, max_u, min_v, max_v = min(us), max(us), min(vs), max(vs)
        in01 = (-PRODUCT_RANGE_EPS <= min_u <= 1 + PRODUCT_RANGE_EPS) and (-PRODUCT_RANGE_EPS <= max_u <= 1 + PRODUCT_RANGE_EPS) \
            and (-PRODUCT_RANGE_EPS <= min_v <= 1 + PRODUCT_RANGE_EPS) and (-PRODUCT_RANGE_EPS <= max_v <= 1 + PRODUCT_RANGE_EPS)
        shared = e["surfaces"] == set(range(len(surfaces)))
        overlap = overlap_exact(e["tris"], grid) if (in01 and shared) else None
        result[uid] = {
            "min_u": min_u, "max_u": max_u, "min_v": min_v, "max_v": max_v, "in01": in01,
            "overlap": overlap, "shared": shared,
            "slot": e["slots"].most_common(1)[0][0],
            "used_by_material": uid in material_ids,
        }
    return result


def exact_candidates(stats):
    return [uid for uid, s in stats.items()
            if s["shared"] and s["in01"] and s["overlap"] is not None and s["overlap"] <= MAX_OVERLAP]


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
    cand = exact_candidates(stats)
    if len(cand) >= 2:
        return "several", cand
    if len(cand) == 0:
        return "none", list(stats.keys())
    return "one", cand


TIE = 1e-9  # exact-equality tolerance for the staged rule below (overlap/frac_lit are both ratios
            # of integer cell/texel counts, so a genuine tie prints as an exact float equality)


def rule_pick(sig_by_id):
    """The staged rule this task adopts: (chosen uv id, the stage that decided it) or (None, "no-in01").
    Over real meshes, [`overlap_exact`] alone rarely discriminates among several candidates -- a
    genuine lightmap unwrap has **no** true triangle overlap anywhere, so every real candidate
    typically measures exactly 0.0 once the shared-edge double-count bug is fixed (see the module
    docstring), which is why the known `platform_03/platform.mdl` hard case *ties* at 0.0 for both
    of its candidates under the corrected measure. The second stage breaks such ties by the
    fraction of each candidate's own UV footprint that is actually "lit" baked pixel data in the
    instance's own lightmap texture (`frac_lit`), rather than landing on the unused, near-grey
    gutter between charts (or, for `platform_03`, right at the texture's own edge) -- the real
    unwrap should have none of its own footprint fall outside the baked chart. A final tie (not
    observed on the cases checked by hand) falls back to the existing order: lowest id unused by
    the surface's own material, then lowest slot index."""
    in01_ids = [uid for uid, s in sig_by_id.items() if s["in01"]]
    if not in01_ids:
        return None, "no-in01"
    best_overlap = min(sig_by_id[u]["overlap"] for u in in01_ids)
    tied = [u for u in in01_ids if abs(sig_by_id[u]["overlap"] - best_overlap) <= TIE]
    if len(tied) == 1:
        return tied[0], "overlap"
    best_frac = max(sig_by_id[u]["frac_lit"] for u in tied)
    tied2 = [u for u in tied if abs(sig_by_id[u]["frac_lit"] - best_frac) <= TIE]
    if len(tied2) == 1:
        return tied2[0], "frac_lit"
    not_used = [u for u in tied2 if not sig_by_id[u]["used_by_material"]]
    pool = not_used if not_used else tied2
    return min(pool, key=lambda u: sig_by_id[u]["slot"]), "material/index"


def picks(sig_by_id):
    """{signal: winning uv id} -- each signal's own top pick in isolation, for the agreement
    diagnostics against the adopted [`rule_pick`] and against the superseded rules."""
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
            mesh_stats_cache[key] = exact_analyze_mesh(mobjs, mesh)
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
        agree_key, stage = rule_pick(sig_by_id)
        st[f"{bucket}: rule resolves at stage '{stage}'"] += 1
        for other in ("coverage", "mean_lum", "frac_lit", "aspect_err", "not_used_by_material", "lowest_index"):
            if pk.get(other) is not None and agree_key is not None:
                st[f"{bucket}: rule-pick agrees with {other}-pick"] += (pk[other] == agree_key)
                st[f"{bucket}: {other}-pick total comparable"] += 1
        if oven_id is not None and agree_key is not None:
            dist[f"{bucket}: (oven id - rule-pick id) per world (sample)"][oven_id - agree_key] += 1
        dist[f"{bucket}: rule-pick's own slot index"][sig_by_id[agree_key]["slot"] if agree_key in sig_by_id else None] += 1
        st[f"{bucket}: rule-pick exists"] += agree_key is not None
        if agree_key is not None:
            dist[f"{bucket}: rule-pick's own overlap value (rounded %)"][round(100 * sig_by_id[agree_key]["overlap"], 1)] += 1
        if case_filter and case_filter in name.lower():
            cases.append((name, i, mpath, bucket, oven_id, sig_by_id, pk, (agree_key, stage)))


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
    for name, iid, mpath, bucket, oven_id, sig_by_id, pk, (rule_id, stage) in cases:
        print(f"\n=== {name} instance #{iid} ({mpath}) bucket={bucket} oven_uv_id={oven_id} ===")
        for uid, s in sorted(sig_by_id.items()):
            print(f"  id {uid:3d} slot {s['slot']}: coverage={s['coverage']:.3f} mean_lum={s['mean_lum']:.1f} "
                  f"frac_lit={s['frac_lit']:.3f} aspect_err={s['aspect_err']} overlap={s['overlap']} "
                  f"used_by_material={s['used_by_material']}")
        print(f"  RULE PICK: id {rule_id} (stage: {stage})")
        print(f"  signal argmax picks: {pk}")


def mesh_overlap_stage(stats):
    """Mesh-only (no lightmap texture needed) view of the rule's first stage: bucket, whether
    overlap alone already resolves the in-[0, 1] ids to one (no tie -- the rule stops at stage
    "overlap"), and, if so, which id and whether it agrees with the superseded rules. A tie here
    means the real rule (`rule_pick`) needs the per-instance `frac_lit` stage, which this cheap
    pass cannot compute -- it only counts how often that is the case."""
    bucket, ids = bucket_of(stats)
    if bucket == "one":
        return bucket, None, None
    in01 = {uid: stats[uid] for uid in ids if stats[uid]["in01"]}
    not_used = [uid for uid in ids if not stats[uid]["used_by_material"]]
    material_pick = min(not_used, key=lambda u: stats[u]["slot"]) if not_used else None
    index_pick = min(ids, key=lambda u: stats[u]["slot"]) if ids else None
    old = {"material": material_pick, "index": index_pick}
    if not in01:
        return bucket, ("none", None), old
    best = min(in01[u]["overlap"] for u in in01)
    tied = [u for u in in01 if abs(in01[u]["overlap"] - best) <= TIE]
    if len(tied) == 1:
        return bucket, ("overlap", tied[0]), old
    return bucket, ("tie", tuple(sorted(tied))), old


def run_meshes(install, limit):
    """Distinct-mesh-level bucket sizes and overlap-stage-only counts for the several/none
    buckets, under the corrected measure -- no lightmap texture decode, so much cheaper than
    `run()`, but it can only report how often `overlap_exact` alone resolves several candidates
    to one (or ties, needing the real rule's `frac_lit` stage); it is NOT the adopted rule by
    itself for any mesh where it reports a tie. Comparable to T01-414's own distinct-mesh counts."""
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
                mesh_stats_cache[key] = exact_analyze_mesh(mobjs, mesh)
            stats = mesh_stats_cache[key]
            if stats is None or key in mesh_bucket:
                continue
            mesh_bucket[key] = (mpath, *mesh_overlap_stage(stats))
        if os.environ.get("WLD_PROGRESS"):
            print(f"[{n + 1}/{len(worlds)}] {name} done at {__import__('time').time() - t0:.1f}s", file=sys.stderr)
    for bucket_name in ("several", "none"):
        rows = [(p, st, old) for p, b, st, old in mesh_bucket.values() if b == bucket_name]
        print(f"\n{bucket_name}: {len(rows)} distinct meshes (corrected overlap measure)")
        no_in01 = sum(1 for _, st, _ in rows if st[0] == "none")
        resolved = sum(1 for _, st, _ in rows if st[0] == "overlap")
        tied = sum(1 for _, st, _ in rows if st[0] == "tie")
        print(f"  no id inside [0, 1] at all (unchanged slot-0 fallback): {no_in01}")
        print(f"  overlap alone resolves (stage 'overlap'): {resolved}")
        print(f"  overlap ties (needs the frac_lit stage, this pass can't tell which): {tied}")
        resolved_rows = [(p, st[1], old) for p, st, old in rows if st[0] == "overlap"]
        agree_mat = sum(1 for _, ov, old in resolved_rows if old["material"] == ov)
        agree_idx = sum(1 for _, ov, old in resolved_rows if old["index"] == ov)
        comparable_mat = sum(1 for _, ov, old in resolved_rows if old["material"] is not None)
        if resolved_rows:
            print(f"  of those, agrees with lowest-index pick: {agree_idx}/{len(resolved_rows)}")
            print(f"  of those, agrees with not-used-by-material pick: {agree_mat}/{comparable_mat}")
        print(f"  meshes with an overlap tie ({tied}):")
        for p, st, old in rows:
            if st[0] == "tie":
                print(f"    {p}: tied ids={st[1]} material-pick={old['material']} index-pick={old['index']}")


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
