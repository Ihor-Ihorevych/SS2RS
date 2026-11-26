"""Evidence for docs/re/puppet-params-loading.md (T01-139, T01-150): where the puppet parameter files are, which identifiers the two
file tables of the binary would hold, and what the placed puppets carry.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries).

It prints
  1. the `.ep` files under the `Puppets` folder (the table of the generic branch, spec section 2): count, folders, the identifiers (file
     name without folder and extension) and the identifiers shared by two files;
  2. the `.ep` files under the `EntityParams` folder (the table of the non-generic branch): count, the `_DM` files and which base
     files have one, the files whose name is the stem of a puppet entity class name;
  3. (worlds, slow: about 10 minutes, `--no-worlds` skips) the placed puppet entities: how many carry a parameter file pointer, how many have
     a non-empty `en_idPuppet` and whether that identifier equals the stem of the parameter file they point at; the placed property
     records and whether their parameter pointer and obsolete identifier are set;
  4. the dummy foe: the `Puppets/Objects` files and whether any world places an entity of the dummy foe class.

Usage: python -I tools/re/puppet_params_verify.py [DATA_DIR] [--no-worlds] [LIMIT]   (DATA_DIR defaults to $SS2_DATA_DIR)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, merged_files, parse  # noqa: E402

NULL = 0xFFFFFFFF


def stem(path):
    return path.replace("\\", "/").rsplit("/", 1)[-1].rsplit(".", 1)[0]


def lineage(parent, name):
    out = []
    while name is not None and name not in out: out.append(name); name = parent.get(name)
    return out


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    no_worlds = "--no-worlds" in sys.argv
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    limit = int(args[1]) if len(args) > 1 else None
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    eps = merged_files(install, ".ep")
    low = {k.replace("\\", "/").lower(): k for k in eps}
    puppets = sorted(k for k in low if "/databases/puppets/" in k)
    print(f"1. the Puppets folder: {len(puppets)} .ep files (of {len(eps)} .ep files in all)")
    folders = collections.Counter(k.split("/databases/puppets/")[1].rsplit("/", 1)[0] if "/" in k.split("/databases/puppets/")[1] else "(top)" for k in puppets)
    print("   by sub-folder:", dict(sorted(folders.items())))
    ids = collections.defaultdict(list)
    for k in puppets: ids[stem(k)].append(k.split("/databases/puppets/")[1])
    dup_ci = collections.defaultdict(list)
    for i, v in ids.items(): dup_ci[i.lower()] += v
    print(f"   distinct identifiers (case-sensitive): {len(ids)}; (case-insensitive): {len(dup_ci)}")
    print("   identifiers shared by several files:", {i: v for i, v in dup_ci.items() if len(v) > 1} or "none")
    print("   Objects folder:", sorted(k.split("/databases/puppets/")[1] for k in puppets if k.split("/databases/puppets/")[1].startswith("objects/")))
    ent = sorted(k for k in low if "/databases/entityparams/" in k)
    names = [k.split("/databases/entityparams/")[1] for k in ent]
    print(f"\n2. the EntityParams folder: {len(ent)} .ep files")
    dm = [n for n in names if n.lower().endswith("_dm.ep")]
    base_dm = [n for n in names if n.lower()[:-3] + "_dm.ep" in {x.lower() for x in names}]
    print(f"   _DM files: {len(dm)}; base files that have a _DM file: {len(base_dm)}")
    print("   files named like puppet classes:", [n for n in names if "puppet" in n.lower()])
    print("   _DM files:", dm)
    if no_worlds:
        return
    worlds = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))
    if limit: worlds = worlds[:limit]
    by_class = collections.Counter(); has_ptr = collections.Counter(); id_set = collections.Counter(); id_match = collections.Counter()
    prop = collections.Counter(); mismatch = collections.Counter(); dummy = 0
    for wi, (name, d) in enumerate(worlds):
        r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects(); extra, _ = dec.extra_objects(edob)
        T = r["types"]; parent = {t.name: (T[t.parent].name if t.parent in T else None) for t in T.values() if t.members is not None}
        rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}
        idn = lambda v, r=r: r["idnt"][v] if isinstance(v, int) and 0 <= v < len(r["idnt"]) else v
        for o in extra.values():
            if isinstance(o, Obj) and "SPuppetProperties" in lineage(parent, o.type):
                prop["property records"] += 1
                p = o.get("SPuppetProperties.ep_pppPuppetParams")
                prop["with a parameter pointer"] += isinstance(p, Ref) and p.oid != NULL
                prop["with an obsolete identifier"] += idn(o.get("SPuppetProperties.ep_idPuppet")) not in ("empty", "", None)
        for o in objs.values():
            if not isinstance(o, Obj): continue
            if o.type == "SDummyFoePuppetEntity" or "DummyFoe" in o.type: dummy += 1
            if "SPuppetEntity" not in lineage(parent, o.type): continue
            by_class[o.type] += 1
            p = o.get("SPuppetEntity.en_pppPuppetParams")
            path = ex.get(p.oid) if isinstance(p, Ref) and p.oid in ex else None
            has_ptr[bool(path)] += 1
            i = idn(o.get("SPuppetEntity.en_idPuppet"))
            if i not in ("empty", "", None):
                id_set["set"] += 1
                id_match[(stem(path).lower() == str(i).lower()) if path else "no pointer"] += 1
                if path and stem(path).lower() != str(i).lower(): mismatch[(str(i), stem(path))] += 1
            else:
                id_set["empty"] += 1
        print(f"\r{wi + 1}/{len(worlds)} worlds", end="", file=sys.stderr)
    print(f"\n3. placed puppets: {sum(by_class.values())} in {len(worlds)} worlds; by class: {dict(by_class)}")
    print(f"   with a parameter file pointer: {dict(has_ptr)}; en_idPuppet: {dict(id_set)}; the identifier equals the stem of the pointed file (ignoring case): {dict(id_match)}")
    print("   the differing pairs (identifier, file stem):", dict(mismatch.most_common(12)))
    print(f"   placed property records: {dict(prop)}")
    print(f"\n4. entities of a dummy foe class in the worlds: {dummy}")


if __name__ == "__main__":
    main()
