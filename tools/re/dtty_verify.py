"""Statistics of the `DTTY` kind and flags words (docs/re/dtty-kinds.md, T01-052). Read-only; data files only.

For every effective format-8 file: the short forms of the type table (kind, words), the `kind` and `flags` of struct blocks,
how many classes keep one (kind, flags) in all files, and whether the kind of a class grows with the writer build number
(the evidence that a struct `kind` is a class version number).

Usage: python -I tools/re/dtty_verify.py [DATA_DIR]
"""
import collections, os, re, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import merged_files, parse  # noqa: E402


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    files = merged_files(install, "")
    short = collections.Counter(); kinds = collections.Counter(); flags = collections.Counter()
    variants = collections.defaultdict(lambda: collections.defaultdict(list))     # class -> (kind, members) -> [build numbers]
    classes_of_kind = collections.defaultdict(set); where = collections.defaultdict(collections.Counter)
    for name, d in files.items():
        r = parse(d); ext = name.rsplit(".", 1)[-1]
        build = int(re.search(r":(\d+);", r["version"]).group(1))
        for t in r["types"].values():
            if t.members is None:
                short[(t.kind, t.words[0], len(t.words))] += 1
            else:
                kinds[t.kind] += 1; flags[t.flags] += 1; classes_of_kind[t.kind].add(t.name)
                variants[t.name][(t.kind, len(t.members))].append(build)
                if t.kind in (0, 99, 246, 252, 1000, 114): where[(t.name, t.kind)][ext] += 1
    print(len(files), "files;", sum(kinds.values()), "struct blocks;", len(variants), "classes")
    print("short forms (kind, first word, number of words): count")
    for k, v in sorted(short.items()): print("  ", k, v)
    print("struct flags:", dict(flags))
    print("struct kind: blocks / classes")
    for k in sorted(kinds): print("  kind %4d  %7d blocks  %4d classes" % (k, kinds[k], len(classes_of_kind[k])))
    multi = {n: v for n, v in variants.items() if len({k[0] for k in v}) > 1}
    mono = 0
    for n, v in multi.items():
        order = sorted(v.items(), key=lambda kv: min(kv[1]))
        ks = [k[0] for k, _ in order]
        mono += all(a <= b for a, b in zip(ks, ks[1:]))
    print("classes with more than one kind:", len(multi), "; kind never decreases with the earliest build:", mono)
    print("most versions of one class:", sorted(((len({k[0] for k in v}), n) for n, v in variants.items()), reverse=True)[:4])
    print("unusual kinds:", {k: dict(v) for k, v in where.items()})


if __name__ == "__main__":
    main()
