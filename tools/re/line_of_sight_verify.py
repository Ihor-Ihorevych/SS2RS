"""Evidence for docs/re/brain-line-of-sight.md (T01-244): the data that the line-of-sight rays read.
Data only, read-only on the install (CLEANROOM.md: this script reads DATA files, never the game's binaries).

It prints, for the puppet parameter files under `Databases/Puppets` (124 `.ep`):
  1. the body parameters that decide where a puppet's eye point and its target point are: the stretch (`pbp_fStretch`), the first
     vector member of `SPuppetBodyParams` (member `.3`, identified as `pbp_vViewOrigin` by its position in the member table: **medium-low**),
     its sitting variant (`.11`), and the two other vector members that hold the target heights (`.7` and `.6`, which of them is the view
     target and which the shoot target is not decided by the data);
  2. for the legged puppets (SLeggedPuppetParams and subclasses that carry them): `pbp_fViewHeightCrouch`, `pbp_fViewHeightSwim` and
     `pbp_fViewHeightDive` (named members, **high** for the values), the three pose-dependent eye heights;
  3. the ray numbers of the spec's section 3 as the script computes them for three puppets (test vectors, not read from the game).

Usage: python -I tools/re/line_of_sight_verify.py [DATA_DIR]   (DATA_DIR defaults to $SS2_DATA_DIR)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, merged_files, parse  # noqa: E402


def vec(v):
    return tuple(x for _, x in v) if isinstance(v, Obj) else None


def main():
    install = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    eps = merged_files(install, ".ep")
    low = {k.replace("\\", "/").lower(): k for k in eps}
    puppets = sorted(k for k in low if "/databases/puppets/" in k)
    stretch = collections.Counter(); vo_y = collections.Counter(); vs_y = collections.Counter(); vo_x = collections.Counter()
    t7 = collections.Counter(); t6 = collections.Counter(); eq = collections.Counter()
    crouch = collections.Counter(); swim = collections.Counter(); dive = collections.Counter(); legged = 0; n = 0
    examples = {}
    for k in puppets:
        d = eps[low[k]]
        objs, _ = TreeDec(d, parse(d)).objects()
        root = next((o for o in objs.values() if isinstance(o, Obj) and o.type != "CResourceFile"), None)
        if root is None: continue
        body = None
        for key, v in root:
            if key == "SPuppetParams.2": body = objs[v.oid] if isinstance(v, Ref) else v
            if key.endswith(".lpp_lpbBody"):
                lb = objs[v.oid] if isinstance(v, Ref) else v
                m = {kk.split(".", 1)[1]: x for kk, x in lb}
                if "pbp_fViewHeightCrouch" in m:
                    legged += 1; crouch[round(m["pbp_fViewHeightCrouch"], 3)] += 1; swim[round(m["pbp_fViewHeightSwim"], 3)] += 1; dive[round(m["pbp_fViewHeightDive"], 3)] += 1
        if body is None: continue
        n += 1
        m = {kk.split(".", 1)[1]: x for kk, x in body}
        s = m.get("1"); a = vec(m.get("3")); b = vec(m.get("11")); c = vec(m.get("7")); e = vec(m.get("6"))
        stretch[round(s, 3)] += 1
        if a: vo_y[round(a[1], 3)] += 1; vo_x[(round(a[0], 3), round(a[2], 3))] += 1
        if b: vs_y[round(b[1], 3)] += 1
        if c: t7[round(c[1], 3)] += 1
        if e: t6[round(e[1], 3)] += 1
        if a and c: eq["member .3 == member .7"] += a == c
        examples[k.rsplit("/", 1)[-1]] = (s, a, c, e)
    print(f"1. the {len(puppets)} puppet `.ep` files, {n} with a body-parameter record")
    print("   pbp_fStretch:", dict(sorted(stretch.items())))
    print("   member .3 (view origin) y:", dict(sorted(vo_y.items())), "; x and z pairs:", dict(vo_x))
    print("   member .11 (sitting variant) y:", dict(sorted(vs_y.items())))
    print("   member .7 y:", dict(sorted(t7.items())))
    print("   member .6 y:", dict(sorted(t6.items())))
    print("   puppets whose member .3 equals member .7:", eq["member .3 == member .7"])
    print(f"\n2. legged puppets with the three pose heights: {legged}")
    print("   pbp_fViewHeightCrouch:", dict(sorted(crouch.items())))
    print("   pbp_fViewHeightSwim:", dict(sorted(swim.items())))
    print("   pbp_fViewHeightDive:", dict(sorted(dive.items())))
    print("\n3. test vectors (computed here): eye point of a standing puppet = position + rotate(heading, (0, y, 0) * stretch)")
    for name in ("chicken.ep", "kleerofski.ep", "kwongo.ep"):
        if name in examples:
            s, a, c, e = examples[name]
            print(f"   {name}: stretch {s}, member .3 {a} -> eye height above the placement = {a[1] * s:.3f}")


if __name__ == "__main__":
    main()
