"""Reproduce the T01-055 mesh measurements without reading engine binaries."""

import argparse
from collections import Counter
import os
from pathlib import Path
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
from bmf_verify import buffer_bytes, path
from ctse_values import TreeDec, merged_files, parse


NULL = 0xFFFFFFFF
LEAF = 0xFFFF


def member(obj, number):
    return obj.get(f"{obj.type}.{number}")


def stream(buffers, element_path, count, layout):
    _, buffer, offset = path(element_path)
    size = struct.calcsize(layout) * count
    data = buffers[buffer][offset:offset + size]
    assert len(data) == size
    return tuple(struct.iter_unpack(layout, data))


def dot(a, b):
    return sum(x * y for x, y in zip(a, b, strict=True))


def cross(a, b):
    return (a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0])


def surface_details(name, surface, surfaces, indices, vertices, stats):
    positions = stream(vertices, member(surface, 7), member(surface, 3), "<3f")
    normals = stream(vertices, member(surface, 8), member(surface, 3), "<3f")
    triangles = stream(indices, member(surface, 6), member(surface, 2), "<3H")
    for a, b, c in triangles:
        ab = tuple(x - y for x, y in zip(positions[b], positions[a], strict=True))
        ac = tuple(x - y for x, y in zip(positions[c], positions[a], strict=True))
        face = cross(ab, ac)
        facing = sum(dot(face, normals[index]) for index in (a, b, c))
        alignment = "aligned" if facing > 0 else "opposed" if facing < 0 else "orthogonal"
        stats["winding", "degenerate" if dot(face, face) == 0 else alignment] += 1
    if name.endswith("/grass_14.bmf"):
        uv = stream(vertices, member(surface, 14)[0], len(positions), "<2f")
        for height in (0.0, 1.0):
            values = sorted({round(v, 2) for (_, y, _), (_, v) in zip(positions, uv, strict=True)
                             if y == height})
            print(f"Grass card at local Y={height:g}: V={values}")
    for morph in member(surface, 22):
        affected = member(morph, 3)
        tangents = member(morph, 6)
        assert len(set(affected)) == len(affected) and all(i < len(positions) for i in affected)
        assert len(member(morph, 4)) == len(member(morph, 5)) == len(affected)
        assert len(tangents) in (0, len(affected))
        stats["relative morphs", member(morph, 2)] += 1
        stats["morph tangent deltas", bool(tangents)] += 1
        for index, delta in zip(affected, member(morph, 5), strict=True):
            offset = tuple(value for _, value in delta)
            modified = tuple(a + b for a, b in zip(normals[index], offset, strict=True))
            if abs(dot(modified, modified) ** 0.5 - 1) < abs(dot(offset, offset) ** 0.5 - 1):
                stats["normal deltas closer to unit after addition", True] += 1
            stats["normal deltas", True] += 1
    if member(surface, 16).oid == NULL:
        master = member(surface, 20)
        if master < 0:
            stats["null material", "no master"] += 1
        else:
            assert 0 <= master < len(surfaces) and surfaces[master] is not surface
            target = member(surfaces[master], 16)
            stats["null material", "master has preset" if target.oid != NULL else "master has no preset"] += 1


def collision_details(name, mesh, vertices, stats):
    indices16, indices32 = member(mesh, 13), member(mesh, 14)
    nodes, refs = member(mesh, 16), member(mesh, 17)
    stats["collision uses render streams", bool(member(mesh, 19))] += 1
    assert not member(mesh, 12)
    if not member(mesh, 19):
        assert not (indices16 or indices32 or refs or nodes)
        return
    assert bool(indices16) != bool(indices32)
    indices = indices16 or indices32
    stats["collision index width", 16 if indices16 else 32] += 1
    count = member(mesh, 22)
    buffer = vertices[member(mesh, 20)]
    position_offset, normal_offset = member(mesh, 21), member(mesh, 23)
    assert max(position_offset, normal_offset) + 12 * count <= len(buffer)
    assert len(indices) % 3 == 0 and min(indices) >= 0 and max(indices) < count
    tri_count = len(indices) // 3
    assert len(member(mesh, 9)) == len(member(mesh, 26)) == tri_count
    assert set(refs) == set(range(tri_count))
    stats["collision triangles", True] += tri_count
    stats["collision references", True] += len(refs)
    if any(i >= len(member(mesh, 11)) for i in member(mesh, 9)):
        print("Collision surface IDs outside table:", name.rsplit("/", 1)[-1])
    bounds = tuple(value for _, value in member(mesh, 18))
    positions = struct.unpack_from(f"<{count * 3}f", buffer, position_offset)
    tri_bounds = []
    for i in range(0, len(indices), 3):
        points = [positions[j * 3:j * 3 + 3] for j in indices[i:i + 3]]
        tri_bounds.append((tuple(map(min, zip(*points))), tuple(map(max, zip(*points)))))
    pending = [(0, bounds[:3], bounds[3:])]
    seen, spans = set(), []
    while pending:
        index, lower, upper = pending.pop()
        assert index not in seen
        seen.add(index)
        plane, first, size, start = (value for _, value in nodes[index])
        assert start + size <= len(refs)
        spans.extend(range(start, start + size))
        for triangle in refs[start:start + size]:
            minimum, maximum = tri_bounds[triangle]
            overlap = all(a <= d + 1e-4 and b >= c - 1e-4
                          for a, b, c, d in zip(minimum, maximum, lower, upper, strict=True))
            stats["triangle AABB overlaps node box", overlap] += 1
        axis = plane >> 14
        stats["collision node", "leaf" if axis == 3 else "internal"] += 1
        if axis == 3:
            assert plane == first == LEAF
        else:
            assert index < first < first + 1 < len(nodes)
            plane_at = bounds[axis] + (bounds[axis + 3] - bounds[axis]) * (plane & 0x3FFF) / 16384
            lower_max, upper_min = list(upper), list(lower)
            lower_max[axis] = upper_min[axis] = plane_at
            pending.extend(((first, lower, lower_max), (first + 1, upper_min, upper)))
    assert len(seen) == len(nodes) and sorted(spans) == list(range(len(refs)))


def scan(install):
    stats = Counter()
    meshes = merged_files(install, ".bmf")
    if len(meshes) != 76:
        raise SystemExit(f"Expected 76 effective retail meshes, got {len(meshes)}")
    for name, data in sorted(meshes.items()):
        objects, _ = TreeDec(data, parse(data)).objects()
        render = next(obj for obj in objects.values() if obj.type == "CRenderMesh")
        indices = [buffer_bytes(objects, holder) for holder in member(render, 12)]
        vertices = [buffer_bytes(objects, holder) for holder in member(render, 13)]
        for lod in member(render, 5):
            stats["platform mask", f"{member(lod, 7).v:#010x}"] += 1
            for section in member(lod, 5):
                surfaces = member(section, 1)
                for surface in surfaces:
                    surface_details(name, surface, surfaces, indices, vertices, stats)
        for mesh in objects.values():
            if mesh.type == "CCollisionMesh":
                collision_details(name, mesh, vertices, stats)
    for (category, value), count in sorted(stats.items()):
        print(f"{category}, {value}: {count}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("install", nargs="?", type=Path, default=os.environ.get("SS2_DATA_DIR"))
    args = parser.parse_args()
    if args.install is None or not args.install.is_dir():
        parser.error("pass a retail install directory or set SS2_DATA_DIR")
    scan(args.install)
