"""Re-runs the measurements behind docs/re/wld-navigation-leftovers.md on an install (read-only):
the roles of CNMNode.2/.4/.9/.10/.11 and CNMLink.3, the length of CNavigationGrid.4 against the
entry pool, and the three tactics without a mover (data side; wld-navigation.md and brain-movement.md
already cover everything else the task asks for).

Usage: python -I tools/re/wld_nav_leftovers_verify.py [DATA_DIR] [LIMIT]
       (DATA_DIR defaults to $SS2_DATA_DIR; LIMIT = only the LIMIT smallest worlds, default all)
"""
import collections, math, os, statistics, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
from wld_root_verify import ids  # noqa: E402

NULL = 0xFFFFFFFF


def named(seq):
    return [{k.split(".")[-1]: v for k, v in x} for x in seq]


def check_mesh(m, st, dist, diffs):
    nodes = named(m.get("CNavMesh.26"))
    ls = named(m.get("CNavMesh.28"))
    lk = named(m.get("CNavMesh.29"))
    grid = m.get("CNavMesh.30")
    if not nodes:
        return
    rev = [l["9"] for l in lk]
    target = [ls[j]["1"] for j in range(len(lk))]
    owner = [target[rev[j]] for j in range(len(lk))]
    mine = collections.defaultdict(list)
    for j in range(len(lk)):
        mine[owner[j]].append(j)

    # --- CNMNode.2 ---
    for i, nd in enumerate(nodes):
        st["CNMNode.2 total"] += 1
        t = nd["2"]
        if t == i:
            st["CNMNode.2 == self"] += 1
            continue
        st["CNMNode.2 != self"] += 1
        st["CNMNode.2 != self: target index is higher than own"] += t > i
        st["CNMNode.2 != self: target is a valid node index of this mesh"] += 0 <= t < len(nodes)
        st["CNMNode.2 != self: target is a valid LINK index of this mesh (CNMLink/.29)"] += 0 <= t < len(lk)

    # --- CNMNode.9 / .10 / .11 ---
    for i, nd in enumerate(nodes):
        a, b = nd["9"] != -1, nd["10"] != 0
        st["CNMNode.9 total"] += 1
        st["CNMNode.9 != -1 <=> CNMNode.10 != 0 (node by node)"] += a == b
        st["CNMNode.11 != 0"] += nd["11"] != 0
        if a:
            st["CNMNode.9 != -1: link count of the node"] += 1
            dist["CNMNode.9 != -1: link-count histogram"][min(len(mine.get(i, [])), 8)] += 1

    # --- CNMLink.3 ---
    pair_sum_nonzero = 0
    for j, l in enumerate(lk):
        A = nodes[owner[j]]
        B = nodes[target[j]]
        d_ = l["4"]
        axisA = A["4"][2] if d_ in (0, 2) else A["4"][1]
        sign = 1 if d_ in (0, 1) else -1
        want = sign * axisA
        diffs.append(abs(l["3"] - want))
        st["CNMLink.3 total"] += 1
        r = rev[j]
        if round(l["3"] + lk[r]["3"], 6) != 0:
            pair_sum_nonzero += 1
    st["CNMLink.3: links whose .3 is NOT the exact negative of their reverse's .3"] += pair_sum_nonzero

    # --- CNavigationGrid.4 length vs the entry pool and the grid-node array ---
    ents = [[v for _, v in e] for e in grid.get("CGenericGrid2D.5")]
    gridnodes = grid.get("CGenericGrid2D.1")
    fe = set(grid.get("CGenericGrid2D.6"))
    used = [i for i in range(len(ents)) if i not in fe]
    lst = list(grid.get("CNavigationGrid.4"))
    st["meshes"] += 1
    st["CNavigationGrid.4 length == entry pool length (CGenericGrid2D.5)"] += len(lst) == len(ents)
    st["CNavigationGrid.4 length == grid-node array length (CGenericGrid2D.1)"] += len(lst) == len(gridnodes)
    hi = (max(used) + 1) if used else 0
    st["CNavigationGrid.4 length == 1 + highest used entry index"] += len(lst) == hi
    st["CNavigationGrid.4 length > 1 + highest used entry index"] += len(lst) > hi
    st["CNavigationGrid.4 length < node/used-entry count (should never happen)"] += len(lst) < len(used)
    dist["CNavigationGrid.4 length, entry pool, grid-node array, nodes (one row per mesh)"][
        (len(lst), len(ents), len(gridnodes), len(nodes))
    ] += 1


def check_world(name, d, st, dist, diffs):
    r = parse(d)
    dec = TreeDec(d, r)
    objs, edob = dec.objects()
    w = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CWorld")
    ents = ids(w.get("CWorld.1"))
    st["worlds"] += 1
    for o in objs.values():
        if isinstance(o, Obj) and o.type == "CNavMesh":
            check_mesh(o, st, dist, diffs)
    # the tactics without a mover
    for i in ents:
        o = objs[i]
        if o.type == "STacticEntity":
            mv = o.get("STacticEntity.en_hmcCharacterMover")
            if not isinstance(mv, Ref) or mv.oid == NULL:
                print(
                    f"no-mover tactic: {name.rsplit('/', 1)[-1]} obj {i} "
                    f"marker={o.get('STacticEntity.en_henTacticMarker')} "
                    f"next={o.get('STacticEntity.en_henNextTactic')}",
                    file=sys.stderr,
                )
                st["tactics without a mover (en_hmcCharacterMover empty)"] += 1


def main():
    install = sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].isdigit() else os.environ.get("SS2_DATA_DIR", "")
    limit = int(sys.argv[-1]) if sys.argv[-1].isdigit() else None
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder or set SS2_DATA_DIR")
    st = collections.Counter()
    dist = collections.defaultdict(collections.Counter)
    diffs = []
    files = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))
    for n, (name, d) in enumerate(files):
        if limit and n >= limit:
            break
        check_world(name, d, st, dist, diffs)
        print(f"[{n + 1}/{len(files)}] {name.rsplit('/', 1)[-1]}", file=sys.stderr, flush=True)
    for k, v in sorted(st.items()):
        if v:
            print(f"{v:9d}  {k}")
    if diffs:
        print("\nCNMLink.3 vs the owner node plane's slope component along the link direction:")
        print("  median abs diff:", statistics.median(diffs))
        print("  90th pct abs diff:", statistics.quantiles(diffs, n=10)[-2])
    for k, v in sorted(dist.items()):
        v = collections.Counter({a: b for a, b in v.items() if b})
        if v:
            print(k, dict(v.most_common(16)))


if __name__ == "__main__":
    main()
