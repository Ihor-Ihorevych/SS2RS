"""Generates the class/member schema of the `.ep` entity parameter files and re-runs the checks
behind docs/re/ep.md (read-only).

Usage:
  python -I tools/re/ep_schema.py [DATA_DIR] --check [--worlds N] [--ext .ep[,.rsc,.ctl,...]]
      prints the checks (decode, one root, class tree, references resolve, entity links in the N smallest worlds)
  python -I tools/re/ep_schema.py [DATA_DIR] --markdown OUT.md [--ext .ep[,.rsc,...]] [--title "..."]
      writes the schema reference (docs/re/ep-schema.md is generated this way; `--ext .rsc,.ctl --title "..."` writes
      docs/re/rsc-ctl-schema.md: any set of extensions can be given)
DATA_DIR defaults to $SS2_DATA_DIR.
"""
import collections, os, sys, zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Blob, Enum, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
from walk_objects import chain  # noqa: E402

NULL = 0xFFFFFFFF


def positional_key(seen, key):
    """Suffixes a repeated key's 2nd, 3rd, ... occurrence with `#2`, `#3`, ... .

    A class's own `STMB` member list is not guaranteed to have distinct names (docs/re/ctsemeta-types.md
    invariant 4: six classes, including `CBloomProperties`, declare one member name twice). Aggregating
    per-member statistics by the plain name string silently pools both occurrences' values into one
    accumulator even though the row-emission loop already (correctly) walks the member list by file
    position (T01-426). Keying by this function's return value instead keeps every member's statistics
    tied to its own file position, while leaving every non-repeated member's key exactly as before.
    `seen` is a `collections.Counter` the caller keeps for the duration of one walk (one object instance
    when collecting, one class's own member list when emitting rows) so repeats are counted correctly.
    """
    n = seen[key]; seen[key] += 1
    return key if n == 0 else f"{key}#{n + 1}"


def ids(v): return [x.oid for x in (v or []) if isinstance(x, Ref) and x.oid != NULL]


def load(install, exts=(".ep",)):
    out = {}
    for ext in exts:
        for name, d in sorted(merged_files(install, ext).items()):
            r = parse(d); objs, edob = TreeDec(d, r).objects(); out[name] = (d, r, objs, edob)
    return out


def root_of(objs):
    rf = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CResourceFile")
    return ids(rf.get("CResourceFile.rf_crsShared"))


def type_name(r, tid):
    t = r["types"].get(tid)
    return t.name if t is not None else "external " + str(r["exty"].get(tid, "?"))


def check(install, world_limit, exts=(".ep",)):
    files = load(install, exts); st = collections.Counter(); roots = collections.defaultdict(collections.Counter); tree = {}
    allpaths = set()
    for f in os.listdir(install):
        if f.lower().endswith(".gro"):
            with zipfile.ZipFile(os.path.join(install, f)) as z: allpaths |= {i.filename.lower() for i in z.infolist() if not i.is_dir()}
    refs = collections.defaultdict(collections.Counter); root_class = {}
    for name, (d, r, objs, edob) in files.items():
        ext = "." + name.rsplit(".", 1)[-1]; st[f"{ext} files"] += 1; sh = root_of(objs); st[f"{ext}: exactly one shared root"] += len(sh) == 1
        tid = dict(r["obty"])[sh[0]]; ch = [c.name for c in chain(r, tid)]
        if ext == ".ep": st[".ep: root derives from CEntityParams"] += "CEntityParams" in ch
        root_class[name] = objs[sh[0]].type; roots[ext][objs[sh[0]].type] += 1; tree[objs[sh[0]].type] = " > ".join(ch[3:] if len(ch) > 3 else ch)
        st[f"{ext}: no extra records after EDOB"] += d[edob + 4:edob + 8] == bytes(4)
        for e in r["rfil"]:
            x = e[2].rsplit(".", 1)[-1].lower(); refs[ext][x] += 1; st[f"{ext}: external references"] += 1
            st[f"{ext}: ... resolve to a file of the install"] += e[2].lower() in allpaths or e[2].lower().startswith("bin/")
    print(chr(10).join(f"{v:7d}  {k}" for k, v in sorted(st.items())))
    for ext in sorted(roots): print(f"{ext} root classes ({len(roots[ext])}):", dict(roots[ext].most_common(12))); print(f"{ext} external reference types:", dict(refs[ext].most_common()))
    for k, v in sorted(tree.items()): print("   ", k, "<-", v)
    if world_limit and ".ep" in exts:
        links = collections.Counter(); bad = collections.Counter()
        paths = {n.lower(): c for n, c in root_class.items()}
        for name, d in sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))[:world_limit]:
            r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects(); extra, _ = dec.extra_objects(edob)
            rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}

            def walk(rec, v, k):
                if isinstance(v, Ref) and v.oid in ex and ex[v.oid].lower().endswith(".ep"):
                    p = ex[v.oid].lower(); links[(rec.type, k, paths.get(p, "MISSING"))] += 1
                    if p not in paths: bad[p] += 1
                elif isinstance(v, list) and not isinstance(v, Blob):
                    for x in v: walk(rec, x, k)
                elif isinstance(v, Obj):
                    for kk, x in v: walk(rec, x, k)
            for o in list(objs.values()) + list(extra.values()):
                if isinstance(o, Obj) and (o.type.endswith("Properties") or o.type.endswith("Entity")):
                    for k, v in o: walk(o, v, k)
        print(f"entity -> .ep links in the {world_limit} smallest worlds: {sum(links.values())}; unresolved: {sum(bad.values())}")
        for k, v in links.most_common(12): print("   ", v, k)


def schema(install, exts=(".ep",)):
    eps = load(install, exts); variants = collections.defaultdict(collections.Counter); parent = {}; stats = collections.defaultdict(list); used = collections.Counter()
    for name, (d, r, objs, edob) in eps.items():
        T = r["types"]
        for tid, t in T.items():
            if t.members is None: continue
            variants[t.name][tuple((m, type_name(r, mt)) for m, mt in t.members)] += 1
            parent[t.name] = T[t.parent].name if t.parent in T else None
        ident_members = {}
        for tid, t in T.items():
            if t.members is not None:
                for m, mt in t.members:
                    if type_name(r, mt) == "IDENT": ident_members[f"{t.name}.{m}"] = True
        idnt = r["idnt"]

        def collect(o):
            seen = collections.Counter()
            for k, v in o:
                pk = positional_key(seen, k)
                if k in ident_members and isinstance(v, int) and not isinstance(v, bool):
                    stats[pk].append(("ident", idnt[v] if 0 <= v < len(idnt) else f"#{v}"))
                else: stats[pk].append(summarize(v))
                if isinstance(v, Obj): collect(v)
                elif isinstance(v, list) and not isinstance(v, Blob):
                    for x in v:
                        if isinstance(x, Obj): collect(x)
        for o in objs.values():
            if isinstance(o, Obj): used[o.type] += 1; collect(o)
    return variants, parent, stats, used


def summarize(v):
    if isinstance(v, Ref): return ("null" if v.oid == NULL else "ref",)
    if isinstance(v, Blob): return ("bytes", len(v))
    if isinstance(v, Enum): return ("enum", v.v)
    if isinstance(v, bool): return ("num", int(v))
    if isinstance(v, (int, float)): return ("num", v)
    if isinstance(v, str): return ("str", len(v))
    if isinstance(v, Obj): return ("struct", v.type)
    if isinstance(v, list): return ("list", len(v))
    return ("?",)


def observed(items):
    kinds = collections.Counter(i[0] for i in items); parts = []
    nums = [i[1] for i in items if i[0] in ("num", "enum")]
    if nums:
        lo, hi = min(nums), max(nums); f = lambda x: f"{x:.6g}"
        parts.append(f"{'values' if len(set(nums)) <= 4 else 'range'} " + (", ".join(sorted({f(x) for x in nums})) if len(set(nums)) <= 4 else f"{f(lo)} .. {f(hi)}") + f" ({len(set(nums))} distinct)")
    lens = [i[1] for i in items if i[0] == "list"]
    if lens: parts.append(f"list length {min(lens)} .. {max(lens)}")
    names = collections.Counter(i[1] for i in items if i[0] == "ident")
    if names:
        shown = ", ".join(f"`{k or 'empty'}`" for k, _ in names.most_common(3)); parts.append(f"identifier {shown}" + (f" ... ({len(names)} distinct)" if len(names) > 3 else ""))
    structs = collections.Counter(i[1] for i in items if i[0] == "struct")
    if structs: parts.append("struct " + ", ".join(f"{k}" for k, _ in structs.most_common(2)))
    for k in ("ref", "null", "str", "bytes"):
        if kinds[k]: parts.append(f"{k} x{kinds[k]}")
    return "; ".join(parts) or "-"


def write_markdown(install, path, exts=(".ep",), title="`.ep` entity parameter schema (generated)"):
    variants, parent, stats, used = schema(install, exts)
    nfiles = sum(len(merged_files(install, e)) for e in exts)
    lines = [f"# {title}", "",
             f"Generated by `tools/re/ep_schema.py --markdown --ext {','.join(exts)}` from the type tables and the objects of the **{nfiles} distinct files** (extensions {', '.join(exts)}) of the retail install with all patches. "
             "For every class: its base class, its **own** members in stored order (a class's full member list is its base classes' members first), the type of each member as named in the type table, and what the files contain.",
             "Numeric members without a readable name are shown by their number (`.5`); readable names are the engine's own (`bp_`, `gpp_`, `pp_`, `lpp_`, `sb_` prefixes). 'Observed' summarizes all instances in the install; `ref` = a reference (an object of the same file or an external file), `null` = null reference. Classes whose definition differs between files list each variant with its file count. "
             "A member name that a class's own type table declares twice (docs/re/ctsemeta-types.md invariant 4) gets one row per occurrence, each keyed and summarized from its own file position alone; the second (or later) occurrence's member name is suffixed `#2`, `#3`, ... to tell it apart from the first (T01-426). The prose specification is next to this file in `docs/re/`.", ""]
    for cls in sorted(variants):
        vs = variants[cls].most_common()
        if not vs or not any(k for k, _ in vs): continue
        lines += [f"## `{cls}`", f"Base: `{parent.get(cls) or '-'}` · instances in these files: {used.get(cls, 0)}", ""]
        for vi, (members, nfiles) in enumerate(vs):
            if len(vs) > 1: lines += [f"**Variant {vi + 1}** ({nfiles} files, {len(members)} members)", ""]
            lines += ["| Member | Type | Observed |", "|---|---|---|"]
            seen = collections.Counter()
            for m, t in members:
                pk = positional_key(seen, f"{cls}.{m}")
                lines.append(f"| `{pk}` | `{t}` | " + (observed(stats[pk]) if vi == 0 and stats.get(pk) else "-") + " |")
            lines.append("")
    with open(path, "w", encoding="utf-8", newline="\n") as f: f.write("\n".join(lines) + "\n")
    print(f"wrote {path}: {len([c for c in variants if any(k for k in variants[c])])} classes, {sum(len(variants[c].most_common(1)[0][0]) for c in variants)} members")


def main():
    args = sys.argv[1:]; install = next((a for a in args if os.path.isdir(a)), os.environ.get("SS2_DATA_DIR", ""))
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    if "--markdown" in args:
        exts = tuple(args[args.index("--ext") + 1].split(",")) if "--ext" in args else (".ep",)
        title = args[args.index("--title") + 1] if "--title" in args else "`.ep` entity parameter schema (generated)"
        write_markdown(install, args[args.index("--markdown") + 1], exts, title)
    else:
        n = int(args[args.index("--worlds") + 1]) if "--worlds" in args else 0
        check(install, n, tuple(args[args.index("--ext") + 1].split(",")) if "--ext" in args else (".ep",))


if __name__ == "__main__":
    main()
