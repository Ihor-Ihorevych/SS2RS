"""Sweeps every CTSEMETA class's own type table, across every format-8 file in the install, for a
class that declares the same member name twice in its own `STMB` list (read-only; rules:
docs/re/ctsemeta-types.md). This is the general case of what T01-424 found by accident for
`CBloomProperties` (its own table declares a member named `"21"` twice; `environment-schema.md`'s
generated table aggregates by name alone, so its one `CBloomProperties.21` row silently merges two
unrelated fields). This script says whether any *other* already-documented class has the same
property, which would mean any doc that aggregates a `*-schema.md`-style member table **by name**
rather than by file position has the same silently-merged-range bug for that class too.

Read-only; builds on `class_catalog.py`'s own file enumeration and type-table parse (never the
game's binaries).

Usage: python -I tools/re/ctse_duplicate_members_verify.py [DATA_DIR]
DATA_DIR defaults to $SS2_DATA_DIR.
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from class_catalog import classes_of  # noqa: E402


def duplicates(members):
    """Member names (in file order) that occur more than once; {name: [0-based positions]}."""
    pos = collections.defaultdict(list)
    for i, m in enumerate(members):
        pos[m].append(i)
    return {m: ps for m, ps in pos.items() if len(ps) > 1}


def main():
    args = sys.argv[1:]
    install = args[0] if args and os.path.isdir(args[0]) else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder or set SS2_DATA_DIR")
    cat, nfiles = classes_of(install)
    hits = {}
    for name, rec in cat.items():
        # Check every variant (older patch levels can have a different member list), not just the
        # most common one: a class could gain or lose its duplicate across patches.
        variant_hits = {}
        for (parent, flags, members), count in rec["variants"].items():
            dups = duplicates(members)
            if dups:
                variant_hits[(parent, flags, members)] = (count, dups)
        if variant_hits:
            hits[name] = variant_hits

    print(f"{len(cat)} classes with members, over {nfiles} CTSEMETA files (all extensions, all patches merged).")
    print(f"{len(hits)} class(es) declare the same member name twice in at least one of their own variants:\n")
    for name in sorted(hits):
        for (parent, flags, members), (count, dups) in hits[name].items():
            total_files = sum(c for _, c in cat[name]["variants"].items())
            dup_desc = ", ".join(f'"{m}" at positions {ps}' for m, ps in sorted(dups.items()))
            print(f"  {name} (parent={parent}, {len(members)} members, this variant in {count}/{total_files} files): {dup_desc}")
    if not hits:
        print("  (none found)")

    known = {"CBloomProperties"}
    new = sorted(set(hits) - known)
    print(f"\nAlready known (T01-424): {sorted(known & set(hits))}")
    print(f"Newly found by this sweep: {new if new else '(none)'}")


if __name__ == "__main__":
    main()
