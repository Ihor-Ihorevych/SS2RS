"""Re-runs the measurements behind docs/re/bmf.md on an install (read-only).

For every format-8 `.bmf`: decodes the objects, then for every surface checks the stream layout
(sizes, bounds, unit normals, local indices, bone palette) and decodes the data after `EDOB`.

Usage: python -I tools/re/bmf_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR)
"""
import collections, math, os, struct, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, TreeDec, merged_files, parse, u32  # noqa: E402

ELEMENT_BYTES = {128: 4, 132: 8, 133: 12, 134: 16, 135: 2}      # element code -> bytes per element
NO_BUFFER = 255                                                 # element path "buffer" value meaning "stream absent"
NULL = 0xFFFFFFFF


def g(o, name): return o.get(name)


def path(o): return (g(o, "CRenderMeshElementPath.1"), g(o, "CRenderMeshElementPath.2"), g(o, "CRenderMeshElementPath.3"))


def buffer_bytes(objs, holder):
    ref = g(holder, "VBufferHolder.1")
    return None if ref.oid == NULL else g(objs[ref.oid], "GFXHANDLE.2")


def check_mesh(name, d, r, objs, stats, covered):
    mesh = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CRenderMesh")
    index_bufs = [buffer_bytes(objs, h) for h in g(mesh, "CRenderMesh.12")]
    vertex_bufs = [buffer_bytes(objs, h) for h in g(mesh, "CRenderMesh.13")]
    lods = g(mesh, "CRenderMesh.5")
    stats["lods"] += len(lods)
    dist = [g(l, "CRenderMeshLOD.6") for l in lods]
    stats["LOD distances ascending (files)"] += dist == sorted(dist)
    for lod in lods:
        box = g(lod, "CRenderMeshLOD.3")
        lo = [g(box, f"Box3f.{i}") for i in (1, 2, 3)]; hi = [g(box, f"Box3f.{i}") for i in (4, 5, 6)]
        for section in g(lod, "CRenderMeshLOD.5"):
            for sf in g(section, "CRenderMeshSection.1"):
                tris, verts = g(sf, "CRenderMeshSurface.2"), g(sf, "CRenderMeshSurface.3")
                stats["surfaces"] += 1; stats["triangles"] += tris; stats["vertices"] += verts
                streams = {k: path(g(sf, "CRenderMeshSurface." + k)) for k in ("7", "8", "9", "10", "11")}
                for i, p in enumerate(g(sf, "CRenderMeshSurface.14")): streams[f"uv{i}"] = path(p)
                streams = {k: v for k, v in streams.items() if v[1] != NO_BUFFER}
                code, ib, off = path(g(sf, "CRenderMeshSurface.6"))
                assert code == 135
                idx = struct.unpack_from(f"<{tris * 3}H", index_bufs[ib], off)       # raises if it does not fit
                covered["index"][(name, "i", ib)].append((off, off + tris * 6, len(index_bufs[ib])))
                data = {}
                for k, (c, b, o) in streams.items():
                    size = verts * ELEMENT_BYTES[c]
                    assert o + size <= len(vertex_bufs[b]), (name, k)
                    data[k] = (vertex_bufs[b], o, c)
                    covered["vertex"][(name, "v", b)].append((o, o + size, len(vertex_bufs[b])))
                stats["streams fit their buffers"] += 1
                stats["uv stream count == field 4"] += sum(k.startswith("uv") for k in streams) == g(sf, "CRenderMeshSurface.4")
                stats["indices local (< vertex count)"] += max(idx) < verts
                stats["every vertex referenced"] += len(set(idx)) == verts
                pos = [struct.unpack_from("<3f", data["7"][0], data["7"][1] + 12 * i) for i in range(verts)]
                stats["positions inside LOD box"] += all(all(lo[a] - 1e-3 <= p[a] <= hi[a] + 1e-3 for a in range(3)) for p in pos)
                nrm = [struct.unpack_from("<3f", data["8"][0], data["8"][1] + 12 * i) for i in range(verts)]
                stats["normals unit length (>=99% of vertices)"] += sum(abs(math.sqrt(sum(c * c for c in n)) - 1) < 0.02 for n in nrm) >= 0.99 * verts
                if "9" in data:
                    b, o, _ = data["9"]
                    stats["tangent surfaces"] += 1
                    stats["tangent w is +-1"] += all(abs(abs(struct.unpack_from("<4f", b, o + 16 * i)[3]) - 1) < 1e-6 for i in range(verts))
                if "10" in data:
                    palette = g(sf, "CRenderMeshSurface.18"); wb, wo, _ = data["10"]; ib_, io, _ = data["11"]
                    stats["skinned surfaces"] += 1
                    top = max(max(ib_[io + 4 * i: io + 4 * i + 4]) for i in range(verts))
                    stats["bone index < palette size"] += top < len(palette)
                    for i in range(verts):
                        stats[f"weight sum {sum(wb[wo + 4 * i: wo + 4 * i + 4])}"] += 1 if sum(wb[wo + 4 * i: wo + 4 * i + 4]) in (0, 254, 255, 256) else 1000000


def tiles(spans):
    """True if the (start, end, buffer size) spans cover [0, size) with no gap."""
    cur = 0
    for a, e, size in sorted(set(spans)):
        if a > cur: return False
        cur = max(cur, e)
    return cur == spans[0][2]


def main():
    install = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder (the one containing All_PC_01.gro) or set SS2_DATA_DIR")
    stats = collections.Counter(); covered = {"index": collections.defaultdict(list), "vertex": collections.defaultdict(list)}
    classes = collections.Counter(); extra_ok = 0; extra_files = 0; extra_classes = collections.Counter()
    for name, d in sorted(merged_files(install, ".bmf").items()):
        r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects(); stats["files"] += 1
        classes.update(o.type for o in objs.values() if isinstance(o, Obj))
        check_mesh(name, d, r, objs, stats, covered)
        if u32(d, edob + 4):
            extra_files += 1
            extra, end = dec.extra_objects(edob); extra_ok += d[end:] == b"METAEND "
            assert not set(extra) & set(objs)
            extra_classes.update(o.type for o in extra.values() if isinstance(o, Obj))
    for kind in ("index", "vertex"):
        spans = covered[kind]; stats[f"{kind} buffers fully covered by surface streams"] += sum(tiles(v) for v in spans.values())
        stats[f"{kind} buffers referenced"] += len(spans)
    for k, v in sorted(stats.items(), key=lambda kv: (not kv[0][0].isalpha(), kv[0])): print(f"{v:9d}  {k}")
    print(f"files with data after EDOB: {extra_files}; decoded exactly to METAEND: {extra_ok}")
    print("extra record classes:", dict(extra_classes.most_common(8)))


if __name__ == "__main__":
    main()
