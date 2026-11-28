"""Evidence for docs/re/wld-lighting.md, "Baked lightmaps" / T01-421 (read-only, SS2_DATA_DIR).

T01-420 added a second tie-break stage to the lightmap UV-stream rule: `frac_lit`, the fraction of
a candidate UV id's own rasterized footprint that is "lit" (departs from near-grey by more than a
tolerance) baked pixel data in the instance's own lightmap texture, rather than landing on unused,
near-grey gutter texels. That earlier task used the test informally ("departs from near-grey by
more than the existing tolerance") without pinning down the reference grey, the tolerance value,
its per-channel metric, the mip level, or the exact texel rasterization -- which this task
(T01-421) fixes in prose (`wld-lighting.md`, reader invariant 3) and checks here.

The test, exactly as `wld_lightmap_streamselect_verify.py`'s own `id_signals`/`GREY_TOL` already
implement it (unchanged by this task):

- reference grey: pure (127, 127, 127);
- metric: per-channel absolute deviation from the reference, R/G/B independently (alpha ignored);
  "lit" means the *largest* of the three deviations exceeds the tolerance (a max-over-channels
  test, not luminance and not a single fixed channel);
- tolerance (`GREY_TOL`): 10, out of a 0-255 channel range;
- mip level: 0, the only level `texpng.decode_static_texture` decodes -- a real choice, not a
  trivial one, since DXT-format lightmaps do carry a full mip chain after level 0 (`tex.md`);
- UV footprint -> texel grid: texel-*centre* sampling on the lightmap texture's own actual
  width x height grid (not the fixed 256-cell grid the overlap stage measures on), with a plain
  inclusive edge test and **no top-left tie-break** -- `rasterize_coverage` is a boolean "is this
  texel covered by at least one triangle of this id" test, so a texel on an edge shared by two
  triangles of the same chart being "inside" both is harmless (the result is already `True`); V is
  not flipped (row y of the decoded, top-to-bottom buffer is v = y / height directly), matching
  reader invariant 4;
- DXT1/BC1 decoding: exact per the public description (`texpng._bc1`): four-colour (opaque)
  interpolation when color0 > color1 as a packed RGB565 u16, three-colour (punch-through)
  interpolation with a transparent fourth palette entry otherwise.

This script adds three checks the earlier tasks did not run:

1. (default) **Does the DXT1 punch-through (3-colour) mode ever matter for a lightmap?** Decoding
   it correctly (instead of, say, always treating color0/color1 blocks as 4-colour) only changes a
   pixel when a block's own index stream actually selects the transparent 4th palette entry (index
   3) under a 3-colour block. Over every DXT1 lightmap texture of the N smallest worlds (default
   20; about 2 minutes), this counts 3-colour blocks and, of those, how many index-3 texels are
   actually selected.
2. `--sensitivity`: how much the `platform_03/platform.mdl` hard case's frac_lit margin (slot 2/UV
   id 5 ahead of slot 0/UV id 3) depends on the exact tolerance and reference grey, over the
   `1_7_city_metropolis` instances T01-420/421 already name in the spec.
3. `--platform-margin`: the same margin, but over *every* instance of `platform_03/platform.mdl`
   in all 59 worlds whose mesh is in the "several candidates, ids {3, 5}" bucket -- not just the
   named world -- to see whether the direction of the margin (id 5's frac_lit at or above id 3's)
   ever reverses anywhere it has actually been baked.

Usage:
  python -I tools/re/wld_lightmap_littexel_verify.py [DATA_DIR] [N]   # DXT1 punch-through check, N smallest worlds (default 20)
  python -I tools/re/wld_lightmap_littexel_verify.py [DATA_DIR] --sensitivity
  python -I tools/re/wld_lightmap_littexel_verify.py [DATA_DIR] --platform-margin
"""
import collections
import math
import os
import struct
import sys
import zlib

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, TreeDec, merged_files, parse  # noqa: E402
from wld_lightmap_stream_verify import (  # noqa: E402
    decode_lightmap_texture, rasterize_coverage, resolved_mesh_for_instance, ModelFileIndex,
)
from wld_lightmap_streamselect_verify import bucket_of, exact_analyze_mesh, id_triangles  # noqa: E402

NULL = 0xFFFFFFFF
GREY_TOL = 10  # the tolerance this task pins down (wld_lightmap_streamselect_verify.GREY_TOL)
REF_GREY = (127, 127, 127)  # the reference grey this task pins down


# --- check 1: does DXT1's punch-through mode ever select a texel? -----------------------------

def _dxt1_block_modes(raw, w, h):
    """(n_blocks, n_three_colour_blocks, n_punch_through_texels_selected)."""
    n_blocks = n_three = n_pt = 0
    p = 0
    bw = max(1, (w + 3) // 4)
    bh = max(1, (h + 3) // 4)
    for by in range(bh):
        for bx in range(bw):
            c0, c1 = struct.unpack_from("<HH", raw, p)
            idx = struct.unpack_from("<I", raw, p + 4)[0]
            p += 8
            n_blocks += 1
            if c0 <= c1:  # the public BC1 three-colour (punch-through) mode
                n_three += 1
                for i in range(16):
                    x, y = bx * 4 + i % 4, by * 4 + i // 4
                    if x < w and y < h and (idx >> (2 * i)) & 3 == 3:
                        n_pt += 1
    return n_blocks, n_three, n_pt


def check_dxt1_modes(install, limit):
    worlds = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))[:limit]
    tot_tex = tot_blocks = tot_three = tot_pt = tot_tex_with_pt = 0
    for name, d in worlds:
        r = parse(d)
        objs, edob = TreeDec(d, r).objects()
        extra, _ = TreeDec(d, r).extra_objects(edob)
        allo = {**objs, **extra}
        seen = set()
        for o in objs.values():
            if not (isinstance(o, Obj) and o.type == "CTextureOvenShaderModifier"):
                continue
            tex_ref = o.get("CTextureOvenShaderModifier.4").get("CShaderArgValTexture.1")
            if tex_ref.oid in seen:
                continue
            seen.add(tex_ref.oid)
            tex = allo.get(tex_ref.oid)
            if tex is None or tex.get("CBaseTexture.6").v != 7:  # 7 = DXT1
                continue
            w, h = tex.get("CBaseTexture.1"), tex.get("CBaseTexture.2")
            raw = zlib.decompress(bytes(tex.get("CBaseTexture.18")))
            nb, nt, npt = _dxt1_block_modes(raw, w, h)
            tot_tex += 1
            tot_blocks += nb
            tot_three += nt
            tot_pt += npt
            tot_tex_with_pt += npt > 0
    print(f"DXT1 lightmap textures checked (over the {limit} smallest worlds): {tot_tex}")
    print(f"total 4x4 blocks: {tot_blocks}")
    if tot_blocks:
        print(f"three-colour (punch-through-eligible) blocks: {tot_three} ({100 * tot_three / tot_blocks:.2f}%)")
    print(f"of those, texels that actually select the punch-through (index-3) palette entry: {tot_pt}")
    print(f"textures with at least one such texel: {tot_tex_with_pt}")
    print("-> if 0, which BC1 mode a decoder implements never changes a decoded lightmap pixel in this data,"
          " though a correct decoder should still implement both per the public spec.")


# --- check 2: sensitivity of the platform_03 margin to the tolerance / reference grey ----------

def _frac_lit(cov, rgba, w, h, ref=REF_GREY, tol=GREY_TOL):
    n_cov = sum(cov)
    if n_cov == 0:
        return None
    n_lit = sum(1 for i in range(w * h)
                if cov[i] and not all(abs(rgba[i * 4 + c] - ref[c]) <= tol for c in range(3)))
    return n_lit / n_cov


def _platform03_candidates(install, world_substr):
    """Yield (instance id, w, h, rgba, tris3, tris5) for platform_03's several(3,5) instances of
    the named world."""
    worlds = dict(merged_files(install, ".wld").items())
    key = next(k for k in worlds if world_substr.lower() in k)
    d = worlds[key]
    r = parse(d)
    objs, edob = TreeDec(d, r).objects()
    extra, _ = TreeDec(d, r).extra_objects(edob)
    allo = {**objs, **extra}
    index = ModelFileIndex(install)
    mdl_cache = {}
    for i, o in objs.items():
        if not (isinstance(o, Obj) and o.type == "CModelInstance" and o.get("CModelInstance.4").oid != NULL):
            continue
        ml = objs[o.get("CModelInstance.4").oid]
        m = next((allo.get(x.oid) for x in ml.get("CShaderModifierList.1")
                  if allo.get(x.oid) is not None and allo[x.oid].type == "CTextureOvenShaderModifier"), None)
        if m is None:
            continue
        mpath, resolved = resolved_mesh_for_instance(r, o, index, mdl_cache)
        if mpath is None or "platform_03" not in mpath.lower():
            continue
        mobjs, mesh = resolved
        stats = exact_analyze_mesh(mobjs, mesh)
        if stats is None:
            continue
        bucket, cand = bucket_of(stats)
        if bucket != "several" or set(cand) != {3, 5}:
            continue
        w, h, rgba = decode_lightmap_texture(objs, extra, m)
        if w is None:
            continue
        yield i, w, h, rgba, id_triangles(mobjs, mesh, 3), id_triangles(mobjs, mesh, 5)


def check_sensitivity(install):
    tol_values = [1, 2, 3, 5, 8, 10, 15, 20, 30, 40, 50, 63]
    ref_values = [(127, 127, 127), (128, 128, 128), (126, 126, 126), (120, 120, 120)]
    rows = list(_platform03_candidates(install, "1_7_city_metropolis"))
    print(f"{len(rows)} 1_7_city_metropolis instances of platform_03/platform.mdl in the several(3, 5) bucket\n")
    print("tolerance sweep (reference grey fixed at (127, 127, 127)):")
    print(f"{'inst':>6} " + " ".join(f"{'tol=' + str(t):>16}" for t in tol_values))
    reversals = 0
    for i, w, h, rgba, tris3, tris5 in rows:
        cov3 = rasterize_coverage(tris3, w, h)
        cov5 = rasterize_coverage(tris5, w, h)
        cells = []
        for tol in tol_values:
            f3 = _frac_lit(cov3, rgba, w, h, tol=tol)
            f5 = _frac_lit(cov5, rgba, w, h, tol=tol)
            if f3 > f5:
                reversals += 1
            cells.append(f"{f3:.3f}/{f5:.3f}")
        print(f"{i:>6} " + " ".join(f"{c:>16}" for c in cells))
    print(f"\nid3 > id5 (reversal of the adopted pick) at any tolerance above: {reversals} of {len(rows) * len(tol_values)} cells")
    print("\nreference-grey sweep (tolerance fixed at 10):")
    for i, w, h, rgba, tris3, tris5 in rows:
        cov3 = rasterize_coverage(tris3, w, h)
        cov5 = rasterize_coverage(tris5, w, h)
        for ref in ref_values:
            f3 = _frac_lit(cov3, rgba, w, h, ref=ref)
            f5 = _frac_lit(cov5, rgba, w, h, ref=ref)
            print(f"  inst {i} ref={ref}: id3={f3:.4f} id5={f5:.4f} margin={f5 - f3:+.4f}")


# --- check 3: does the margin ever reverse over every real occurrence? -------------------------

def check_platform_margin(install):
    worlds = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))
    f3s, f5s = [], []
    reversals = ties = total = 0
    index = ModelFileIndex(install)
    mdl_cache = {}
    mesh_stats_cache = {}
    for name, d in worlds:
        r = parse(d)
        objs, edob = TreeDec(d, r).objects()
        extra, _ = TreeDec(d, r).extra_objects(edob)
        allo = {**objs, **extra}
        for i, o in objs.items():
            if not (isinstance(o, Obj) and o.type == "CModelInstance" and o.get("CModelInstance.4").oid != NULL):
                continue
            ml = objs[o.get("CModelInstance.4").oid]
            m = next((allo.get(x.oid) for x in ml.get("CShaderModifierList.1")
                      if allo.get(x.oid) is not None and allo[x.oid].type == "CTextureOvenShaderModifier"), None)
            if m is None:
                continue
            mpath, resolved = resolved_mesh_for_instance(r, o, index, mdl_cache)
            if mpath is None or "platform_03" not in mpath.lower():
                continue
            mobjs, mesh = resolved
            key = mpath.lower()
            if key not in mesh_stats_cache:
                mesh_stats_cache[key] = exact_analyze_mesh(mobjs, mesh)
            stats = mesh_stats_cache[key]
            if stats is None:
                continue
            bucket, cand = bucket_of(stats)
            if bucket != "several" or set(cand) != {3, 5}:
                continue
            w, h, rgba = decode_lightmap_texture(objs, extra, m)
            if w is None:
                continue
            cov3 = rasterize_coverage(id_triangles(mobjs, mesh, 3), w, h)
            cov5 = rasterize_coverage(id_triangles(mobjs, mesh, 5), w, h)
            f3 = _frac_lit(cov3, rgba, w, h)
            f5 = _frac_lit(cov5, rgba, w, h)
            if f3 is None or f5 is None:
                continue
            total += 1
            f3s.append(f3)
            f5s.append(f5)
            if f3 > f5:
                reversals += 1
            elif f3 == f5:
                ties += 1
    print(f"platform_03/platform.mdl instances in the several(3, 5) bucket, all 59 worlds: {total}")
    if f3s:
        print(f"id 3 (slot 0) frac_lit range: {min(f3s) * 100:.1f}% .. {max(f3s) * 100:.1f}%")
        print(f"id 5 (slot 2) frac_lit range: {min(f5s) * 100:.1f}% .. {max(f5s) * 100:.1f}%")
    print(f"instances where id 3's frac_lit exceeds id 5's (the margin reverses): {reversals}")
    print(f"instances where they tie exactly: {ties}")
    print(f"instances where id 5 is at or above id 3 (the adopted pick's direction holds): {total - reversals}")


def main():
    args = sys.argv[1:]
    install = args[0] if args and os.path.isdir(args[0]) else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder (the one containing All_PC_01.gro) or set SS2_DATA_DIR")
    if "--sensitivity" in args:
        check_sensitivity(install)
        return
    if "--platform-margin" in args:
        check_platform_margin(install)
        return
    limit = next((int(a) for a in args[1:] if a.isdigit()), 20)
    check_dxt1_modes(install, limit)


if __name__ == "__main__":
    main()
