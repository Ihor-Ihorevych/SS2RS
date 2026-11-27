"""Evidence for docs/re/wld-lighting.md / bmf.md, T01-407 (read-only, SS2_DATA_DIR).

T01-406 found the lightmap UV map id (oven modifier member `.3`) is a per-world constant that is
never one of the model's own mesh UV ids, and could not find where the game actually samples the
lightmap. This script tests the leading hypothesis -- "the instance's ordinary base/diffuse UV
channel" -- directly against the data, using the property a lightmap unwrap must have that an
ordinary (tiling) UV channel does not: it packs into 0..1 without overlap.

Mode 1 (default): for every lit instance's resolved mesh (the `.mdl`/`.bmf` chain; external meshes
only, same resolution as wld_lightmap_uv_verify.py), and for every UV stream of its first PC-eligible
LOD (surface member 12/14, bmf.md), measures: the min/max of u and v; whether the stream lies in
[0, 1] (small epsilon); the fraction of rasterized UV-space area covered by more than one triangle
(a 256x256 grid, coarser for very dense meshes); and whether every surface of that LOD declares the
id (a lightmap unwrap packs the whole instance, not one material group). A stream that is in [0,1]
and (near-)non-overlapping is an "unwrap-like" candidate. Reports, per world and overall, how many
distinct meshes and how many instances have exactly one candidate, and which UV id/slot it is.

Mode 2 (--texels): for a handful of instances (the test vector by default), rasterizes the mesh's
triangles under its mesh's sole (or best) candidate stream into the lightmap texture's own texel
grid (decoded with texpng.decode_static_texture) and reports what fraction of the lightmap's "lit"
texels fall inside a triangle. A texel is "lit" when it departs from near-neutral grey (close to
0x7F7F7F, by more than a small tolerance) -- the unused gaps between charts decode to that grey, not
to black (checked on several lightmaps' colour histograms). Can write PNGs (mesh coverage mask +
lightmap) to a scratch folder outside the repo with --out DIR, for a human to look at; never writes
into the repo.

Usage:
  python -I tools/re/wld_lightmap_stream_verify.py [DATA_DIR] [N]
       (N = resolve+check meshes for only the N smallest worlds, default 8; the slow part, same as
        wld_lightmap_uv_verify.py)
  python -I tools/re/wld_lightmap_stream_verify.py [DATA_DIR] --texels [--out DIR] [world:instance ...]
       (default instance list: the dm_stadium.wld test vector plus three more picked from the same
        and one other small world)
"""
import collections, math, os, struct, sys, zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from bmf_verify import NO_BUFFER, buffer_bytes, path as elem_path  # noqa: E402
from ctse_values import Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
from wld_lightmap_uv_verify import model_name_of_instance  # noqa: E402
import texpng  # noqa: E402

NULL = 0xFFFFFFFF
EPS = 2e-3
OVERLAP_THRESHOLD = 0.02   # a candidate's rasterized overlap fraction must be at or below this


class ModelFileIndex:
    """Lazy {lowercase path: bytes} for .mdl and .bmf files across the install's .gro archives."""

    def __init__(self, install):
        self.loc = {}
        for f in sorted(x for x in os.listdir(install) if x.lower().endswith(".gro")):
            with zipfile.ZipFile(os.path.join(install, f)) as z:
                for i in z.infolist():
                    if not i.is_dir() and i.filename.lower().endswith((".mdl", ".bmf")):
                        self.loc[i.filename.lower()] = (f, i.filename)
        self.install = install
        self.cache = {}

    def get(self, path):
        key = path.lower()
        if key not in self.cache:
            arch, name = self.loc.get(key, (None, None))
            self.cache[key] = None if arch is None else zipfile.ZipFile(os.path.join(self.install, arch)).read(name)
        return self.cache[key]


def _ext_bmf_path(r, ref):
    """The external .bmf path a reference names via RFIL/EXOB, or None (null, embedded, or not a .bmf)."""
    if not isinstance(ref, Ref) or ref.oid == NULL:
        return None
    e = next((e for e in r["exob"] if e[0] == ref.oid), None)
    if e is None or e[2] != 1:          # flags==1: a plain external file reference
        return None
    rfil = next((f for f in r["rfil"] if f[0] == e[1]), None)
    if rfil is None:
        return None
    return rfil[2] if rfil[2].lower().endswith(".bmf") else None


def load_render_mesh(data, index):
    """A `.mdl` or `.bmf` file's bytes -> (objs, CRenderMesh Obj), following one level of external
    `.bmf` reference through `index` (a ModelFileIndex). None if unresolved (embedded config only,
    missing file, or no mesh instance)."""
    r = parse(data)
    objs, _ = TreeDec(data, r).objects()
    mesh = next((o for o in objs.values() if isinstance(o, Obj) and o.type == "CMesh"), None)
    if mesh is not None:
        rm = mesh.get("CMesh.1")
        if isinstance(rm, Ref) and rm.oid in objs:
            return objs, objs[rm.oid]
    cfg = next((o for o in objs.values()
                if isinstance(o, Obj) and o.type == "CModelConfiguration" and o.get("CResource.1") == 1), None)
    if cfg is None:
        return None
    for mi in cfg.get("CModelConfiguration.3"):
        ref = mi.get("CMeshInstance.1")
        if not isinstance(ref, Ref) or ref.oid == NULL:
            continue
        if ref.oid in objs and isinstance(objs[ref.oid], Obj) and objs[ref.oid].type == "CMesh":
            rm = objs[ref.oid].get("CMesh.1")
            if isinstance(rm, Ref) and rm.oid in objs:
                return objs, objs[rm.oid]
        ext = _ext_bmf_path(r, ref)
        if ext is not None and index is not None:
            bdata = index.get(ext)
            if bdata is not None:
                return load_render_mesh(bdata, index)
    return None


def _edge(ax, ay, bx, by, px, py):
    return (bx - ax) * (py - ay) - (by - ay) * (px - ax)


def choose_grid(n_tris, budget=3_000_000, lo=16, hi=256):
    """A rasterization grid size such that n_tris * grid^2 (the cost if every triangle's bounding
    box covered the whole grid -- the worst case for a tiling/duplicated UV stream) stays bounded."""
    return max(lo, min(hi, int(math.sqrt(budget / max(n_tris, 1)))))


def overlap_fraction(tris, grid):
    """(fraction of covered grid cells that >=2 triangles cover, number of covered cells)."""
    counts = [0] * (grid * grid)
    for (u0, v0), (u1, v1), (u2, v2) in tris:
        area2 = _edge(u0, v0, u1, v1, u2, v2)
        if abs(area2) < 1e-12:
            continue
        s = -1.0 if area2 < 0 else 1.0
        x0 = max(0, int(min(u0, u1, u2) * grid)); x1 = min(grid - 1, int(max(u0, u1, u2) * grid))
        y0 = max(0, int(min(v0, v1, v2) * grid)); y1 = min(grid - 1, int(max(v0, v1, v2) * grid))
        for gy in range(y0, y1 + 1):
            py = (gy + 0.5) / grid
            row = gy * grid
            for gx in range(x0, x1 + 1):
                px = (gx + 0.5) / grid
                w0 = s * _edge(u1, v1, u2, v2, px, py)
                if w0 < 0:
                    continue
                w1 = s * _edge(u2, v2, u0, v0, px, py)
                if w1 < 0:
                    continue
                w2 = s * _edge(u0, v0, u1, v1, px, py)
                if w2 < 0:
                    continue
                counts[row + gx] += 1
    covered = sum(1 for c in counts if c > 0)
    overlapped = sum(1 for c in counts if c > 1)
    return (overlapped / covered if covered else 0.0), covered


def pc_lod(mesh):
    """The mesh's first PC-eligible (`CRenderMeshLOD.7` bit 0) LOD, or None."""
    for lod in mesh.get("CRenderMesh.5"):
        plat = lod.get("CRenderMeshLOD.7")
        plat = plat.v if hasattr(plat, "v") else plat
        if plat & 1:
            return lod
    return None


def lod_surfaces(lod):
    return [sf for sec in lod.get("CRenderMeshLOD.5") for sf in sec.get("CRenderMeshSection.1")]


def surface_uv_arrays(objs, mesh, sf):
    """{slot: (uv_id, [(u, v), ...] of length V)} for the surface's present UV slots."""
    vbufs = [buffer_bytes(objs, h) for h in mesh.get("CRenderMesh.13")]
    V = sf.get("CRenderMeshSurface.3")
    ids = sf.get("CRenderMeshSurface.12")
    paths = sf.get("CRenderMeshSurface.14")
    out = {}
    for slot in range(8):
        uid = ids[slot]
        if uid == 0:
            continue
        c, b, o = elem_path(paths[slot])
        if b == NO_BUFFER or vbufs[b] is None or c != 132:
            continue
        out[slot] = (uid, [struct.unpack_from("<2f", vbufs[b], o + 8 * i) for i in range(V)])
    return out


def surface_triangles(objs, mesh, sf):
    ibufs = [buffer_bytes(objs, h) for h in mesh.get("CRenderMesh.12")]
    T = sf.get("CRenderMeshSurface.2")
    c, ib, io = elem_path(sf.get("CRenderMeshSurface.6"))
    if ib == NO_BUFFER or ibufs[ib] is None:
        return []
    return struct.unpack_from(f"<{T * 3}H", ibufs[ib], io)


def _collect_uvmap_ids(o, out):
    """Recursively collect every non-zero `CShaderArgValUVMap.1` found anywhere under `o` (a
    CShaderArgs object or a nested value within one) -- the UV ids the surface's *own* material
    argues for its texture groups (mtr.md), whatever shader class it is."""
    if isinstance(o, Obj):
        if o.type == "CShaderArgValUVMap":
            v = o[0][1]
            if v:
                out.add(v)
            return
        for _, v in o:
            _collect_uvmap_ids(v, out)
    elif isinstance(o, list):
        for item in o:
            _collect_uvmap_ids(item, out)


def material_uv_ids(objs, sf):
    """UV ids the surface's own embedded material (`CShaderPreset`) references in any texture
    group, for any of its `CShaderConfig`s. Empty for a null or external (.mtr) material."""
    out = set()
    mat = sf.get("CRenderMeshSurface.16")
    if isinstance(mat, Ref) and mat.oid != NULL and mat.oid in objs:
        preset = objs[mat.oid]
        if isinstance(preset, Obj) and preset.type == "CShaderPreset":
            for cfg in preset.get("CShaderPreset.1"):
                args = cfg.get("CShaderConfig.2")
                if isinstance(args, Ref):
                    args = objs.get(args.oid)
                _collect_uvmap_ids(args, out)
    return out


def analyze_mesh(objs, mesh):
    """{uv_id: stats} over the mesh's first PC-eligible LOD. stats: min_u/max_u/min_v/max_v, in01,
    overlap (None if not in01), covered_cells, shared (declared on every surface of the LOD), slot
    (the most common slot index it occupies), used_by_material (any surface's own material
    arguments for one of its texture groups name this id -- the "base/diffuse channel" test)."""
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
        in01 = (-EPS <= min_u) and (max_u <= 1 + EPS) and (-EPS <= min_v) and (max_v <= 1 + EPS)
        overlap = covered = None
        if in01:
            overlap, covered = overlap_fraction(e["tris"], choose_grid(len(e["tris"])))
        result[uid] = {
            "min_u": min_u, "max_u": max_u, "min_v": min_v, "max_v": max_v, "in01": in01,
            "overlap": overlap, "covered_cells": covered,
            "shared": e["surfaces"] == set(range(len(surfaces))),
            "n_surfaces": len(e["surfaces"]), "total_surfaces": len(surfaces),
            "slot": e["slots"].most_common(1)[0][0],
            "used_by_material": uid in material_ids,
        }
    return result


def candidates(stats):
    return [uid for uid, s in stats.items() if s["in01"] and s["overlap"] is not None and s["overlap"] <= OVERLAP_THRESHOLD]


def resolved_mesh_for_instance(r, inst, index, mdl_cache):
    """(objs, CRenderMesh Obj) for a CModelInstance's external model, or None. Cached by model path."""
    mpath = model_name_of_instance(r, inst)
    if mpath is None:
        return None, None
    key = mpath.lower()
    if key not in mdl_cache:
        data = index.get(mpath)
        mdl_cache[key] = load_render_mesh(data, index) if data is not None else None
    return mpath, mdl_cache[key]


def check_world(name, d, st, dist, index, mdl_cache, mesh_stats_cache):
    r = parse(d); objs, edob = TreeDec(d, r).objects()
    extra, _ = TreeDec(d, r).extra_objects(edob)
    allo = {**objs, **extra}
    st["worlds"] += 1
    for i, o in objs.items():
        if not (isinstance(o, Obj) and o.type == "CModelInstance" and o.get("CModelInstance.4").oid != NULL):
            continue
        ml = objs[o.get("CModelInstance.4").oid]
        for x in ml.get("CShaderModifierList.1"):
            m = allo.get(x.oid)
            if m is None or m.type != "CTextureOvenShaderModifier":
                continue
            st["oven modifiers (instances) seen"] += 1
            mpath, resolved = resolved_mesh_for_instance(r, o, index, mdl_cache)
            if mpath is None:
                st["instance model not resolved to an external .mdl (embedded or unresolved)"] += 1
                continue
            if resolved is None:
                st["model resolved but its mesh could not be loaded (external .bmf missing, embedded config, no PC LOD)"] += 1
                continue
            mobjs, mesh = resolved
            key = mpath.lower()
            if key not in mesh_stats_cache:
                mesh_stats_cache[key] = analyze_mesh(mobjs, mesh)
            stats = mesh_stats_cache[key]
            if stats is None:
                st["mesh had no PC-eligible LOD or no surfaces"] += 1
                continue
            st["instances with a mesh stream analysis"] += 1
            cand = candidates(stats)
            dist["candidate ('unwrap-like') streams per instance's mesh"][len(cand)] += 1
            if len(cand) == 1:
                st["instances whose mesh has exactly one candidate stream"] += 1
                dist["the sole candidate's slot index (0-based, instances)"][stats[cand[0]]["slot"]] += 1
                dist["the sole candidate's UV id (instances, sample)"][cand[0]] += 1
                st["... of those, the candidate is NOT referenced by any of the surface's own material texture groups (not the base/diffuse channel)"] += not stats[cand[0]]["used_by_material"]
                st["... of those, the candidate IS referenced by a material texture group (could be the base/diffuse channel)"] += stats[cand[0]]["used_by_material"]
            dist["distinct UV ids seen per mesh (instances, sample)"][len(stats)] += 1


def run_stats(install, limit):
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter)
    index = ModelFileIndex(install)
    mdl_cache = {}; mesh_stats_cache = {}
    worlds = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))[:limit]
    t0 = __import__("time").time()
    for n, (name, d) in enumerate(worlds):
        check_world(name, d, st, dist, index, mdl_cache, mesh_stats_cache)
        if os.environ.get("WLD_PROGRESS"):
            print(f"[{n + 1}/{len(worlds)}] {name} ({len(d)} bytes) done at {__import__('time').time() - t0:.1f}s, meshes cached {len(mdl_cache)}", file=sys.stderr)
    # per-mesh (not per-instance) view: how many *distinct* meshes have exactly one candidate
    mesh_cand_counts = collections.Counter()
    mesh_slot = collections.Counter()
    mesh_used_by_material = collections.Counter()
    for key, stats in mesh_stats_cache.items():
        if stats is None:
            continue
        cand = candidates(stats)
        mesh_cand_counts[len(cand)] += 1
        if len(cand) == 1:
            mesh_slot[stats[cand[0]]["slot"]] += 1
            mesh_used_by_material[stats[cand[0]]["used_by_material"]] += 1
    for k, v in sorted(st.items()):
        print(f"{v:9d}  {k}")
    print("distinct meshes resolved and analysed:", sum(mesh_cand_counts.values()))
    print("distinct meshes by candidate-stream count:", dict(mesh_cand_counts))
    print("distinct meshes with exactly one candidate, by its slot index:", dict(mesh_slot))
    print("distinct meshes with exactly one candidate, is it also referenced by that surface's own material (True) or a separate, otherwise-unused channel (False):", dict(mesh_used_by_material))
    for k, v in sorted(dist.items()):
        print(k, dict(v.most_common(12)))


# ---------------------------------------------------------------------------
# Mode 2: visual texel-coverage check
# ---------------------------------------------------------------------------

def decode_lightmap_texture(objs, extra, m):
    tex_ref = m.get("CTextureOvenShaderModifier.4").get("CShaderArgValTexture.1")
    allo = {**objs, **extra}
    tex = allo.get(tex_ref.oid)
    if tex is None:
        return None
    return texpng.decode_static_texture(tex)


def rasterize_coverage(tris, w, h):
    """Boolean w*h grid: True where a texel center falls inside some triangle (candidate UV stream)."""
    cov = [False] * (w * h)
    for (u0, v0), (u1, v1), (u2, v2) in tris:
        area2 = _edge(u0, v0, u1, v1, u2, v2)
        if abs(area2) < 1e-12:
            continue
        s = -1.0 if area2 < 0 else 1.0
        x0 = max(0, int(min(u0, u1, u2) * w)); x1 = min(w - 1, int(max(u0, u1, u2) * w))
        y0 = max(0, int(min(v0, v1, v2) * h)); y1 = min(h - 1, int(max(v0, v1, v2) * h))
        for gy in range(y0, y1 + 1):
            py = (gy + 0.5) / h
            row = gy * w
            for gx in range(x0, x1 + 1):
                px = (gx + 0.5) / w
                w0 = s * _edge(u1, v1, u2, v2, px, py)
                if w0 < 0:
                    continue
                w1 = s * _edge(u2, v2, u0, v0, px, py)
                if w1 < 0:
                    continue
                w2 = s * _edge(u0, v0, u1, v1, px, py)
                if w2 < 0:
                    continue
                cov[row + gx] = True
    return cov


def texel_check(install, targets, out_dir):
    index = ModelFileIndex(install)
    worlds = dict(merged_files(install, ".wld").items())
    mdl_cache = {}; mesh_stats_cache = {}
    for world_name, inst_id in targets:
        key = next((k for k in worlds if k.endswith(world_name.lower())), None)
        if key is None:
            print(f"{world_name}: not found"); continue
        d = worlds[key]
        r = parse(d); objs, edob = TreeDec(d, r).objects()
        extra, _ = TreeDec(d, r).extra_objects(edob)
        allo = {**objs, **extra}
        inst = objs.get(inst_id)
        if inst is None or inst.type != "CModelInstance":
            print(f"{world_name}:{inst_id}: not a CModelInstance"); continue
        ml = objs[inst.get("CModelInstance.4").oid]
        m = next((allo[x.oid] for x in ml.get("CShaderModifierList.1")
                   if allo.get(x.oid) is not None and allo[x.oid].type == "CTextureOvenShaderModifier"), None)
        if m is None:
            print(f"{world_name}:{inst_id}: no oven modifier"); continue
        mpath, resolved = resolved_mesh_for_instance(r, inst, index, mdl_cache)
        if mpath is None or resolved is None:
            print(f"{world_name}:{inst_id}: mesh not resolved ({mpath})"); continue
        mobjs, mesh = resolved
        mkey = mpath.lower()
        if mkey not in mesh_stats_cache:
            mesh_stats_cache[mkey] = analyze_mesh(mobjs, mesh)
        stats = mesh_stats_cache[mkey]
        cand = candidates(stats) if stats else []
        w, h, rgba = decode_lightmap_texture(objs, extra, m)
        if not cand:
            print(f"{world_name}:{inst_id} ({mpath}, lightmap {w}x{h}): no in-[0,1]-and-non-overlapping "
                  f"candidate stream found; skipping texel check")
            continue
        uid = cand[0]
        tris = []
        for sf in lod_surfaces(pc_lod(mesh)):
            for slot, (sid, uv) in surface_uv_arrays(mobjs, mesh, sf).items():
                if sid != uid:
                    continue
                tri_idx = surface_triangles(mobjs, mesh, sf)
                for t_idx in range(0, len(tri_idx), 3):
                    a, b, c = tri_idx[t_idx:t_idx + 3]
                    tris.append((uv[a], uv[b], uv[c]))
        cov = rasterize_coverage(tris, w, h)
        # Unused texels between charts are not black: they decode to a near-neutral grey (close to
        # 0x7F7F7F, the shader system's own documented "no value" default, mtr.md) -- confirmed by
        # inspection of several lightmaps' colour histograms (one dominant near-grey cluster, the
        # rest scattered). A real baked texel is "lit" when it departs from that grey by more than
        # a small tolerance.
        lit_mask = [not all(abs(rgba[i * 4 + c] - 127) <= 10 for c in range(3)) for i in range(w * h)]
        n_lit = sum(lit_mask)
        n_lit_covered = sum(1 for i in range(w * h) if lit_mask[i] and cov[i])
        frac = n_lit_covered / n_lit if n_lit else float("nan")
        n_cov = sum(cov)
        print(f"{world_name}:{inst_id} ({mpath}): candidate stream(s) {cand}, using id {uid} (slot {stats[uid]['slot']}), "
              f"lightmap {w}x{h}: {n_lit} lit texels ({n_lit / (w * h):.1%} of {w * h}), "
              f"{frac:.1%} of them inside a triangle ({n_cov} texels covered in total)")
        if out_dir:
            os.makedirs(out_dir, exist_ok=True)
            cov_rgba = bytearray(w * h * 4)
            for i in range(w * h):
                v = 255 if cov[i] else 0
                cov_rgba[i * 4:i * 4 + 4] = bytes((v, v, v, 255))
            texpng.write_png(os.path.join(out_dir, f"{world_name}_{inst_id}_lightmap.png"), w, h, rgba)
            texpng.write_png(os.path.join(out_dir, f"{world_name}_{inst_id}_coverage.png"), w, h, cov_rgba)


DEFAULT_TARGETS = [("dm_stadium.wld", 2435)]


def auto_pick_targets(install, world_name, want, exclude_ids):
    """A few more lit instances from `world_name` whose mesh resolves, for breadth beyond the test vector."""
    worlds = dict(merged_files(install, ".wld").items())
    key = next((k for k in worlds if k.endswith(world_name.lower())), None)
    if key is None:
        return []
    d = worlds[key]; r = parse(d); objs, edob = TreeDec(d, r).objects()
    extra, _ = TreeDec(d, r).extra_objects(edob); allo = {**objs, **extra}
    out = []
    for i, o in sorted(objs.items()):
        if len(out) >= want:
            break
        if i in exclude_ids or not (isinstance(o, Obj) and o.type == "CModelInstance" and o.get("CModelInstance.4").oid != NULL):
            continue
        ml = objs[o.get("CModelInstance.4").oid]
        if any(allo.get(x.oid) is not None and allo[x.oid].type == "CTextureOvenShaderModifier" for x in ml.get("CShaderModifierList.1")):
            if model_name_of_instance(r, o) is not None:
                out.append((world_name, i))
    return out


def main():
    args = sys.argv[1:]
    install = args[0] if args and os.path.isdir(args[0]) else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder (the one containing All_PC_01.gro) or set SS2_DATA_DIR")
    if "--texels" in args:
        out_dir = None
        if "--out" in args:
            out_dir = args[args.index("--out") + 1]
        named = [a for a in args[1:] if ":" in a]
        targets = [(w, int(i)) for w, i in (t.split(":") for t in named)] if named else None
        if targets is None:
            targets = list(DEFAULT_TARGETS)
            targets += auto_pick_targets(install, "dm_stadium.wld", 2, {i for _, i in targets})
            targets += auto_pick_targets(install, "dockingbay.wld", 1, set())
        texel_check(install, targets, out_dir)
        return
    limit = int(args[-1]) if args and args[-1].isdigit() else 8
    run_stats(install, limit)


if __name__ == "__main__":
    main()
