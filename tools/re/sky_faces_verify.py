"""Re-runs the measurements behind docs/re/environment.md on the sky cube's faces (read-only, `SS2_DATA_DIR`).

For every world's `CCubeBackgroundEntity`: decodes the renderable's face textures (`CCubeRenderable.10`
through `.15`) to RGBA and compares every pair of face **edges** (top, bottom, left, right; forward or
reversed) across all skies that have both faces. A face that is really adjacent to another on the cube is
drawn so the two edges meet without a seam, so its best-matching edge pair is the *same* one in nearly
every sky, with a near-zero colour difference; an unrelated (opposite) pair of faces has no reason to
match and the best-of-32 alignment differs sky to sky and stays noisy. This tells apart the four side
("ring") faces, which top face and optional bottom face border them, and, for the ring, which image edge
of one face touches which edge of its neighbour and whether either is mirrored. Also checks whether the
property record's own texture (`CCubeBackgroundProperties.1`) is one of the six renderable faces.

Usage: python -I tools/re/sky_faces_verify.py [DATA_DIR] [--dump DIR] [--top3]
       (DATA_DIR defaults to $SS2_DATA_DIR; --dump DIR writes the decoded face PNGs of the test-vector
       world there, named by member number, for a visual check; --top3 also prints each pair's three
       best-voted edge combos, for diagnosing a weak or ambiguous match)
"""
import collections, itertools, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
from texpng import decode_static_texture, write_png  # noqa: E402
from wld_root_verify import ids  # noqa: E402

MEMBERS = (10, 11, 12, 13, 14, 15)
EDGES = ("top", "bottom", "left", "right")
SAMPLES = 48           # samples per edge, nearest-neighbour resampled so edges of different sizes compare


def _edge(w, h, rgba, edge):
    def at(x, y):
        i = (y * w + x) * 4
        return rgba[i], rgba[i + 1], rgba[i + 2]
    length, get = {"top": (w, lambda i: at(i, 0)), "bottom": (w, lambda i: at(i, h - 1)),
                   "left": (h, lambda i: at(0, i)), "right": (h, lambda i: at(w - 1, i))}[edge]
    return [get(min(length - 1, i * length // SAMPLES)) for i in range(SAMPLES)]


def _err(a, b):
    return sum(abs(x - y) for pa, pb in zip(a, b) for x, y in zip(pa, pb)) / (3 * len(a))


def _best(edges_a, ea, edges_b, eb):
    """Lowest error between edge `ea` of A and edge `eb` of B, forward or reversed; (error, flipped)."""
    fwd = _err(edges_a[ea], edges_b[eb]); rev = _err(edges_a[ea], edges_b[eb][::-1])
    return (fwd, False) if fwd <= rev else (rev, True)


def sky_worlds(install, tex_cache=None):
    """Yields (world, {member: (w, h, name) or None}, {member: edges}, record_match) per world with a sky.

    `record_match` is the face member whose path equals `CCubeBackgroundProperties.1` (by path string, not
    pixels, so it also covers the 2 legacy-texture skies that are not decoded), or `None` if the record has
    no texture or it matches none of the six.
    """
    tex_cache = {} if tex_cache is None else tex_cache
    tex = merged_files(install, ".tex")
    for name, d in sorted(merged_files(install, ".wld").items()):
        r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects(); extra, _ = dec.extra_objects(edob)
        rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}
        w = next((o for o in objs.values() if isinstance(o, Obj) and o.type == "CWorld"), None)
        if w is None: continue
        ents = ids(w.get("CWorld.1"))
        recs = {extra[p].get("CEntityProperties.2").oid: extra[p] for p in ids(w.get("CWorld.5")) if p in extra}
        sky = [i for i in ents if objs[i].type == "CCubeBackgroundEntity"]
        if not sky: continue
        i = sky[0]; rec = recs.get(i)
        rec_tex = rec.get("CCubeBackgroundProperties.1") if rec is not None else None
        rec_path = ex.get(rec_tex.oid, "").lower() if isinstance(rec_tex, Ref) else ""
        rend = objs[objs[i].get("CCubeBackgroundEntity.1").oid]
        info, edges, record_match = {}, {}, None
        for m in MEMBERS:
            ref = rend.get(f"CCubeRenderable.{m}")
            path = ex.get(ref.oid, "").lower() if isinstance(ref, Ref) else ""
            info[m] = None
            if path and rec_path and path == rec_path: record_match = m
            if not path or path not in tex: continue
            if path not in tex_cache:
                try:
                    tr = parse(tex[path]); tobjs, _ = TreeDec(tex[path], tr).objects()
                    tobj = next(v for v in tobjs.values() if isinstance(v, Obj) and v.type == "CStaticTexture")
                    tex_cache[path] = decode_static_texture(tobj)
                except Exception:
                    tex_cache[path] = None
            if tex_cache[path] is None: continue
            tw, th, rgba = tex_cache[path]
            info[m] = (tw, th, path.rsplit("/", 1)[-1])
            edges[m] = {e: _edge(tw, th, rgba, e) for e in EDGES}
        short = name.rsplit("/", 1)[-1][:-4]
        yield short, info, edges, (record_match, bool(rec_path))


def main():
    args = sys.argv[1:]
    dump = args[args.index("--dump") + 1] if "--dump" in args else None
    if dump: args = [a for a in args if a not in ("--dump", dump)]
    args = [a for a in args if a != "--top3"]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")

    worlds_with_faces = 0
    present = collections.Counter()               # member -> worlds where it has a texture
    both_present = collections.Counter()           # (m1, m2) -> worlds where both are present
    # per (m1, m2): combo -> [errors]; combo = (edge1, edge2, flipped)
    combo_err = collections.defaultdict(lambda: collections.defaultdict(list))
    best_combo_votes = collections.defaultdict(collections.Counter)  # (m1,m2) -> combo -> count of worlds where it won
    example = None
    tex_cache = {}
    record_match = collections.Counter()   # member -> worlds where CCubeBackgroundProperties.1 is that face
    record_named = 0                        # worlds where the record has a non-null texture at all

    for world, info, edges, (match, named) in sky_worlds(install, tex_cache):
        record_named += named
        if match is not None: record_match[match] += 1
        have = [m for m in MEMBERS if info[m] is not None]
        if not have: continue
        worlds_with_faces += 1
        for m in have: present[m] += 1
        if world == "1_3_village": example = (info, edges)
        for m1, m2 in itertools.combinations(have, 2):
            both_present[(m1, m2)] += 1
            best = None
            for e1, e2 in itertools.product(EDGES, EDGES):
                err, flipped = _best(edges[m1], e1, edges[m2], e2)
                combo = (e1, e2, flipped)
                combo_err[(m1, m2)][combo].append(err)
                if best is None or err < best[0]: best = (err, combo)
            best_combo_votes[(m1, m2)][best[1]] += 1

    print(f"worlds with at least one sky face texture: {worlds_with_faces}")
    print("member presence (worlds):", dict(sorted(present.items())))
    print(f"\nrecord's own texture (CCubeBackgroundProperties.1): named in {record_named} worlds, "
          f"matches one of the 6 renderable faces (by path, so it covers the 2 legacy-texture skies too) in "
          f"{sum(record_match.values())} of them, by member: {dict(sorted(record_match.items()))}")

    # For each pair, the combo that wins most often, and how consistently / how cheaply.
    adjacency = {}
    print("\nmember pair -> best-agreeing edge combo (edge of m1, edge of m2, mirrored), "
          "votes/total worlds with both, mean error of that combo, mean error of all 32 combos:")
    for pair in sorted(both_present):
        total = both_present[pair]
        combo, votes = best_combo_votes[pair].most_common(1)[0]
        errs = combo_err[pair][combo]
        mean_winner = sum(errs) / len(errs)
        all_errs = [e for lst in combo_err[pair].values() for e in lst]
        mean_all = sum(all_errs) / len(all_errs)
        adjacent = mean_winner < 15.0 and mean_winner < 0.3 * mean_all
        adjacency[pair] = (adjacent, combo, votes, total, mean_winner, mean_all)
        print(f"  .{pair[0]:<2} .{pair[1]:<2} -> {combo}  {votes:3d}/{total:<3d}  "
              f"winner={mean_winner:6.2f}  all32={mean_all:6.2f}  {'ADJACENT' if adjacent else 'opposite/none'}")
        if "--top3" in sys.argv:
            for c, v in best_combo_votes[pair].most_common(3):
                e = sum(combo_err[pair][c]) / len(combo_err[pair][c])
                print(f"        {c}  votes={v}  mean_err={e:.2f}")

    # Degree of each member in the adjacency graph (how many of the others it borders).
    degree = collections.Counter()
    for (m1, m2), (adjacent, *_rest) in adjacency.items():
        if adjacent: degree[m1] += 1; degree[m2] += 1
    print("\nadjacency degree per member (4 = touches all others seen; the ring faces border 2 ring "
          "neighbours + the top, and the top/bottom border all 4 ring faces):", dict(sorted(degree.items())))

    if dump and example:
        os.makedirs(dump, exist_ok=True)
        info, edges = example
        tex = merged_files(install, ".tex")
        for m in MEMBERS:
            if info[m] is None: continue
            path = None
            # re-find the path for this member from a fresh pass (info only has name); search cache by name
            for p in tex_cache:
                if tex_cache[p] and p.rsplit("/", 1)[-1] == info[m][2]: path = p; break
            if path is None: continue
            tr = parse(tex[path]); tobjs, _ = TreeDec(tex[path], tr).objects()
            tobj = next(v for v in tobjs.values() if isinstance(v, Obj) and v.type == "CStaticTexture")
            w_, h_, rgba = decode_static_texture(tobj)
            write_png(os.path.join(dump, f"face_{m}_{info[m][2]}.png"), w_, h_, rgba)
        print(f"\nwrote test-vector (1_3_village) face PNGs to {dump}")

    if example:
        info, _ = example
        print("\n1_3_village faces (member: WxH name):")
        for m in MEMBERS:
            print(f"  .{m}: {info[m]}")


if __name__ == "__main__":
    main()
