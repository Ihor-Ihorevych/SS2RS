"""Re-runs the measurements behind docs/re/skl.md on an install (read-only).

For every format-8 `.skl`: inventory the LODs and bones, compare stored and reconstructed inverse
bind matrices, check the parent-offset lengths, and count signed angle bounds by IK flag state.

Usage: python -I tools/re/skl_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR)
"""
import collections, math, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, TreeDec, merged_files, parse  # noqa: E402


def g(o, name): return o.get(name)


def quat_matrix(x, y, z, w):
    """Rotation matrix (3x3, rows) of the quaternion (x, y, z, w)."""
    return [[1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
            [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
            [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)]]


def compose(a, b):
    """a * b for 3x4 affine matrices given as 3 rows of 4."""
    return [[sum(a[i][k] * b[k][j] for k in range(3)) + (a[i][3] if j == 3 else 0) for j in range(4)] for i in range(3)]


def invert(m):
    """Inverse of a rigid or scaled 3x4 affine matrix (general 3x3 by the adjugate)."""
    a = [r[:3] for r in m]; t = [r[3] for r in m]
    det = (a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1]) - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
           + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0]))
    inv = [[(a[(j + 1) % 3][(i + 1) % 3] * a[(j + 2) % 3][(i + 2) % 3] - a[(j + 1) % 3][(i + 2) % 3] * a[(j + 2) % 3][(i + 1) % 3]) / det
            for j in range(3)] for i in range(3)]
    return [inv[i] + [-sum(inv[i][k] * t[k] for k in range(3))] for i in range(3)]


def local_matrix(bone, conjugate=False):
    qv = g(bone, "SkeletonBone.4"); q = g(qv, "QuatVect.1"); t = g(qv, "QuatVect.2")
    x, y, z, w = (g(q, f"Quaternion4f.{i}") for i in (1, 2, 3, 4))
    if conjugate: x, y, z = -x, -y, -z
    r = quat_matrix(x, y, z, w)
    return [r[i] + [g(t, f"Vector3f.{i + 1}")] for i in range(3)], (x, y, z, w), [g(t, f"Vector3f.{i + 1}") for i in range(3)]


def stored_inverse(bone):
    m = g(bone, "SkeletonBone.5")
    v = [g(m, f"Matrix34f.{i}") for i in range(1, 13)]
    return [v[0:4], v[4:8], v[8:12]]


def palette_pairing(install):
    """Compare mesh bone palettes with the skeleton in the same model folder (the folder above `Sources/`).

    Heuristic pairing: only meshes and skeletons whose path contains `/sources/` are paired.
    """
    skeletons = {}
    for name, d in merged_files(install, ".skl").items():
        r = parse(d); objs, _ = TreeDec(d, r).objects()
        sk = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CSkeleton")
        skeletons[name] = {g(b, "SkeletonBone.1") for b in g(g(sk, "CSkeleton.1")[0], "CSkeletonLOD.2")}
    folder = lambda n: n[:n.find("/sources/")] if "/sources/" in n else None
    result = collections.Counter()
    for name, d in sorted(merged_files(install, ".bmf").items()):
        if folder(name) is None: continue
        mates = [n for n in skeletons if folder(n) == folder(name)]
        if not mates: result["meshes without a skeleton in the same model folder"] += 1; continue
        r = parse(d); objs, _ = TreeDec(d, r).objects()
        mesh = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CRenderMesh")
        palette = set()
        for lod in g(mesh, "CRenderMesh.5"):
            for sec in g(lod, "CRenderMeshLOD.5"):
                for sf in g(sec, "CRenderMeshSection.1"): palette |= set(g(sf, "CRenderMeshSurface.18"))
        if not palette: result["static meshes (no palette)"] += 1; continue
        result["palettes that contain 0"] += 0 in palette
        shifts = sorted({k for n in mates for k in range(-6, 10) if all(p - k in skeletons[n] for p in palette)})
        result["skinned meshes paired"] += 1
        result["palette = skeleton ids" if 0 in shifts else
               f"palette - {shifts[0]} = skeleton ids" if shifts else "no shift in -6..9 fits"] += 1
    return result


def main():
    install = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder (the one containing All_PC_01.gro) or set SS2_DATA_DIR")
    st = collections.Counter(); worst = {False: 0.0, True: 0.0}; qerr = 0.0; lenrule = collections.Counter(); limits = collections.Counter(); m3 = []; bad_inv = []
    ik = collections.Counter(); plat = collections.Counter(); lodn = collections.Counter(); roots = collections.Counter()
    residuals = []
    for name, d in sorted(merged_files(install, ".skl").items()):
        r = parse(d); objs, _ = TreeDec(d, r).objects(); st["files"] += 1
        sk = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CSkeleton")
        lods = g(sk, "CSkeleton.1"); lodn[len(lods)] += 1
        dist = [g(l, "CSkeletonLOD.1") for l in lods]; st["LOD distances ascending (files)"] += dist == sorted(dist)
        for lod in lods:
            plat[hex(g(lod, "CSkeletonLOD.4").v)] += 1
            bones = g(lod, "CSkeletonLOD.2"); st["LODs"] += 1; st["bones"] += len(bones)
            ids = [g(b, "SkeletonBone.1") for b in bones]
            st["ids are 1..n in order"] += ids == list(range(1, len(bones) + 1))
            st["parent < id (parents first)"] += all(g(b, "SkeletonBone.2") < g(b, "SkeletonBone.1") for b in bones)
            roots[sum(1 for b in bones if g(b, "SkeletonBone.2") == 0)] += 1
            world = {0: [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0]]}
            for conj in (False, True):
                w = dict(world)
                for b in bones:
                    loc, _, _ = local_matrix(b, conj); w[g(b, "SkeletonBone.1")] = compose(w[g(b, "SkeletonBone.2")], loc)
                    inv = invert(w[g(b, "SkeletonBone.1")]); sv = stored_inverse(b)
                    err = max(abs(inv[i][j] - sv[i][j]) for i in range(3) for j in range(4))
                    worst[conj] = max(worst[conj], err)
                    if not conj:
                        st["bones whose member 5 is the inverse of the composed member 4 (< 1e-4)"] += err < 1e-4
                        if err >= 1e-4:
                            key = (name.rsplit("/", 1)[-1], g(b, "SkeletonBone.1"))
                            bad_inv.append(key)
                            residual = compose(w[g(b, "SkeletonBone.1")], sv)
                            residuals.append((key, [[round(value, 6) for value in row] for row in residual]))
            for b in bones:
                _, q, t = local_matrix(b)
                qerr = max(qerr, abs(math.sqrt(sum(c * c for c in q)) - 1))
                length = g(b, "SkeletonBone.6"); norm = math.sqrt(sum(c * c for c in t))
                lenrule["member 6 = |translation|" if abs(norm - length) < 1e-4 else
                        "member 6 = 1.0 and translation is zero" if norm == 0.0 and length == 1.0 else "other"] += 1
                m3.append(g(b, "SkeletonBone.3"))
                mask = g(b, "SkeletonBone.9").v
                ik[hex(mask)] += 1
                lo = g(b, "SkeletonBone.7"); hi = g(b, "SkeletonBone.8")
                for component in range(3):
                    member = f"Vector3f.{component + 1}"
                    limits[g(lo, member), g(hi, member), bool(mask & (1 << component))] += 1
                st["bones"] += 0
    print(dict(st)); print("LODs per file:", dict(sorted(lodn.items())), "| roots per LOD:", dict(roots), "| platform masks:", dict(plat))
    print(f"max |inverse(world from member 4) - member 5|: quaternion as stored {worst[False]:.2e}, conjugated {worst[True]:.2e}")
    print("bones that do not match:", bad_inv); print("member 6:", dict(lenrule)); print(f"max | |quaternion| - 1 | = {qerr:.2e}")
    print("rest hierarchy x stored inverse bind for mismatches:", residuals)
    print("IK flags (member 9):", dict(ik))
    for (lower, upper, enabled), count in sorted(limits.items()):
        print(f"angle limit pairs: {lower:.9f} .. {upper:.9f}, enabled={enabled}: {count}")
    print("mesh bone palettes vs the skeleton of the same model folder:", dict(palette_pairing(install)))
    print(f"member 3: min {min(m3):.3f}, max {max(m3):.3f}, distinct values {len(set(m3))}")


if __name__ == "__main__":
    main()
