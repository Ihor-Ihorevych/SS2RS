"""Generates the schema of the entity classes and their property records of the `.wld` worlds and re-runs the
census behind docs/re/wld-entities.md (read-only).

For every format-8 `.wld` (one at a time, so memory stays small): every entity (class deriving from `CEntity`)
and every property record (class deriving from `CEntityProperties`, stored in the extra section), with the
structures nested in them; per member an aggregate of what the files contain.

Usage:
  python -I tools/re/wld_entity_schema.py [DATA_DIR] --markdown OUT.md [--json OUT.json] [--worlds N]
  python -I tools/re/wld_entity_schema.py [DATA_DIR] --markdown OUT.md --match REGEX [--also-ext .mac] [--title "..."] [--worlds N]
      (instead of the entity and record classes, the classes whose name or a base class name matches REGEX, plus the
       structures nested in them; --also-ext adds other CTSEMETA files, for example the `.mac` macro programs;
       docs/re/wld-macro-schema.md is generated with --match "Macro|OnHandler|Variant|HintedVar|LatentID|SingleEvent|EntityVariable|EntityPropertyVariable|Expression|VarAssignment|VarTypeHint|ObjectAction" --also-ext .mac)
DATA_DIR defaults to $SS2_DATA_DIR; N = only the N smallest worlds (default all, about 20 minutes).
"""
import collections, json, os, re, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Blob, Enum, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
from ep_schema import load as load_eps, positional_key, root_of, type_name  # noqa: E402

NULL = 0xFFFFFFFF
CAP = 40


class Agg:
    __slots__ = ("kinds", "nums", "lo", "hi", "many", "lens", "idents", "structs")

    def __init__(self):
        self.kinds = collections.Counter(); self.nums = collections.Counter(); self.lo = None; self.hi = None; self.many = False
        self.lens = [None, None]; self.idents = collections.Counter(); self.structs = collections.Counter()

    def add(self, v, ident=None):
        if ident is not None: self.kinds["ident"] += 1; (self.idents.__setitem__(ident, self.idents[ident] + 1) if len(self.idents) < 200 or ident in self.idents else None); return
        if isinstance(v, Ref): self.kinds["null" if v.oid == NULL else "ref"] += 1
        elif isinstance(v, Blob): self.kinds["bytes"] += 1
        elif isinstance(v, str): self.kinds["str"] += 1
        elif isinstance(v, (Enum, bool, int, float)):
            x = v.v if isinstance(v, Enum) else v; self.kinds["enum" if isinstance(v, Enum) else "num"] += 1
            self.lo = x if self.lo is None else min(self.lo, x); self.hi = x if self.hi is None else max(self.hi, x)
            if not self.many:
                self.nums[x] += 1
                if len(self.nums) > CAP: self.many = True; self.nums = collections.Counter()
        elif isinstance(v, Obj): self.kinds["struct"] += 1; self.structs[v.type] += 1
        elif isinstance(v, list):
            self.kinds["list"] += 1; n = len(v); self.lens[0] = n if self.lens[0] is None else min(self.lens[0], n); self.lens[1] = n if self.lens[1] is None else max(self.lens[1], n)

    def text(self):
        k = self.kinds; parts = []
        if self.lo is not None:
            f = lambda x: f"{x:.6g}"
            if self.many: parts.append(f"range {f(self.lo)} .. {f(self.hi)} ({CAP}+ distinct)")
            elif len(self.nums) <= 4: parts.append("values " + ", ".join(sorted({f(x) for x in self.nums})) + f" ({len(self.nums)} distinct)")
            else: parts.append(f"range {f(self.lo)} .. {f(self.hi)} ({len(self.nums)} distinct)")
        if self.lens[0] is not None: parts.append(f"list length {self.lens[0]} .. {self.lens[1]}")
        if self.idents:
            shown = ", ".join(f"`{a or 'empty'}`" for a, _ in self.idents.most_common(3)); parts.append(f"identifier {shown}" + (f" ... ({len(self.idents)} distinct)" if len(self.idents) > 3 else ""))
        if self.structs: parts.append("struct " + ", ".join(a for a, _ in self.structs.most_common(2)))
        for a in ("ref", "null", "str", "bytes"):
            if k[a]: parts.append(f"{a} x{k[a]}")
        return "; ".join(parts) or "-"

    def to_json(self): return {"kinds": dict(self.kinds), "lo": self.lo, "hi": self.hi, "many": self.many, "distinct": len(self.nums), "lens": self.lens, "idents": len(self.idents), "text": self.text()}


def lineage(parent, name):
    out = []
    while name is not None and name not in out: out.append(name); name = parent.get(name)
    return out


def main():
    args = sys.argv[1:]; install = next((a for a in args if os.path.isdir(a)), os.environ.get("SS2_DATA_DIR", ""))
    if not install or not os.path.isdir(install) or "--markdown" not in args: sys.exit(__doc__)
    out_md = args[args.index("--markdown") + 1]; out_json = args[args.index("--json") + 1] if "--json" in args else None
    limit = int(args[args.index("--worlds") + 1]) if "--worlds" in args else None
    match = re.compile(args[args.index("--match") + 1]) if "--match" in args else None
    also = args[args.index("--also-ext") + 1].split(",") if "--also-ext" in args else []
    title = args[args.index("--title") + 1] if "--title" in args else "`.wld` entity and property record schema (generated)"
    eps = {}
    for name, (d, r, objs, edob) in load_eps(install, (".ep",)).items():
        eps[name.lower()] = objs[root_of(objs)[0]].type
    agg = collections.defaultdict(Agg); variants = collections.defaultdict(collections.Counter); parent = {}
    inst = collections.Counter(); worlds_of = collections.defaultdict(set); pair = collections.Counter(); code_of = collections.defaultdict(collections.Counter)
    eplinks = collections.Counter(); epbad = collections.Counter(); st = collections.Counter(); kind_of = {}
    files = []
    for ext in also: files += sorted(merged_files(install, ext).items(), key=lambda kv: len(kv[1]))      # the small extra files first
    files += sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))
    for n, (wname, d) in enumerate(files):
        if limit and n >= limit: break
        r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects()
        try: extra, _ = dec.extra_objects(edob)
        except Exception: extra = {}
        T = r["types"]; rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: (rf[e[1]], e[3]) for e in r["exob"]}
        for tid, t in T.items():
            if t.members is None: continue
            variants[t.name][tuple((m, type_name(r, mt)) for m, mt in t.members)] += 1
            parent[t.name] = T[t.parent].name if t.parent in T else None
        idents = {f"{t.name}.{m}" for t in T.values() if t.members is not None for m, mt in t.members if type_name(r, mt) == "IDENT"}
        idnt = r["idnt"]; st["worlds"] += 1

        def collect(o, root):
            seen = collections.Counter()
            for k, v in o:
                pk = positional_key(seen, k)
                if k in idents and isinstance(v, int) and not isinstance(v, bool): agg[pk].add(v, idnt[v] if 0 <= v < len(idnt) else f"#{v}")
                else: agg[pk].add(v)
                if isinstance(v, Ref) and v.oid in ex and ex[v.oid][0].lower().endswith(".ep"):
                    p = ex[v.oid][0].lower(); eplinks[(root.type, k, eps.get(p, "MISSING"))] += 1
                    if p not in eps: epbad[p] += 1
                if isinstance(v, Obj): collect(v, root)
                elif isinstance(v, list) and not isinstance(v, Blob):
                    for x in v:
                        if isinstance(x, Obj): collect(x, root)
        for oid, o in list({**objs, **extra}.items()):
            if not isinstance(o, Obj): continue
            lin = lineage(parent, o.type)
            if match is not None:
                if match.search(o.type) or any(match.search(a) for a in lin): kind = "match"
                else: continue
            elif "CEntity" in lin: kind = "entity"
            elif "CEntityProperties" in lin: kind = "properties"
            else: continue
            kind_of[o.type] = kind; inst[o.type] += 1; worlds_of[o.type].add(wname); collect(o, o)
            if kind == "match": pass
            elif kind == "entity":
                m = o.get("CEntity.2")
                if isinstance(m, Ref) and m.oid in ex: code_of[o.type][ex[m.oid][1]] += 1
            else:
                e = o.get("CEntityProperties.2")
                if isinstance(e, Ref) and e.oid in objs: pair[(objs[e.oid].type, o.type)] += 1
        print(f"[{n + 1}/{len(files)}] {wname.rsplit('/', 1)[-1]}", file=sys.stderr, flush=True)
    # nested structure classes reachable from the entity/property classes
    nested = set()
    def reach(cls):
        v = variants.get(cls)
        if not v: return
        for members in v:
            for m, t in members:
                base = t.replace("CStaticStackArray<", "").replace("CDynamicContainer<", "").replace(">", "").rstrip("*")
                if t.endswith("*") or base in nested or base not in variants: continue
                nested.add(base); reach(base)
    for c, k in list(kind_of.items()): reach(c)
    for c in list(nested):
        for a in lineage(parent, c)[1:]: nested.add(a)
    ents = sorted(c for c, k in kind_of.items() if k == "entity"); props = sorted(c for c, k in kind_of.items() if k == "properties")
    classes = {c for c in variants if c in kind_of or c in nested or any(a in kind_of or a in nested for a in lineage(parent, c)[1:] if False)}
    bases = set()
    for c in list(kind_of) + list(nested):
        for a in lineage(parent, c)[1:]: bases.add(a)
    show = sorted((set(kind_of) | nested | bases) & set(variants))
    what = (f"It covers every class whose name or base class name matches `{match.pattern}`, the base classes they share and the structures nested in them. " if match is not None else
            "It covers every **entity class** (derives from `CEntity`; runtime members, prefix `en_`), every **property record class** (derives from `CEntityProperties`, stored in the extra section; editor/design members, prefix `ep_`), the base classes they share and the structures nested in them. ")
    lines = [f"# {title}", "",
             f"Generated by `tools/re/wld_entity_schema.py --markdown` from the type tables and the objects of the **{st['worlds']} distinct files** ({'worlds' if not also else 'worlds and ' + ', '.join(also) + ' files'}) of the retail install with all patches. "
             + what +
             "For every class: its base class, the number of instances and worlds, its **own** members in stored order (a class's full member list is its base classes' members first), the type of each member as named in the type table and what the files contain; "
             "`ref` = a reference to an object of the world or to an external resource, `null` = null reference. Classes whose definition differs between worlds list each variant with its world count. "
             "A member name that a class's own type table declares twice (docs/re/ctsemeta-types.md invariant 4) gets one row per occurrence, each keyed and summarized from its own file position alone; the second (or later) occurrence's member name is suffixed `#2`, `#3`, ... to tell it apart from the first (T01-426). The prose specification is next to this file in `docs/re/`.", ""]
    for cls in show:
        vs = variants[cls].most_common()
        if not vs or not any(k for k, _ in vs): continue
        tag = {"entity": "entity", "properties": "property record", "match": "matched class"}.get(kind_of.get(cls), "nested structure" if cls in nested else "base class")
        lines += [f"## `{cls}`", f"Base: `{parent.get(cls) or '-'}` · {tag} · instances: {inst.get(cls, 0)} in {len(worlds_of.get(cls, ()))} worlds", ""]
        for vi, (members, nw) in enumerate(vs):
            if len(vs) > 1: lines += [f"**Variant {vi + 1}** ({nw} worlds, {len(members)} members)", ""]
            lines += ["| Member | Type | Observed |", "|---|---|---|"]
            seen = collections.Counter()
            for m, t in members:
                pk = positional_key(seen, f"{cls}.{m}")
                lines.append(f"| `{pk}` | `{t}` | " + (agg[pk].text() if vi == 0 and pk in agg else "-") + " |")
            lines.append("")
    with open(out_md, "w", encoding="utf-8", newline="\n") as f: f.write("\n".join(lines) + "\n")
    print(f"wrote {out_md}: {len(show)} classes ({len(ents)} entity, {len(props)} property record, {len(nested)} nested)")
    if out_json:
        data = {"entities": {c: {"instances": inst[c], "worlds": len(worlds_of[c]), "base": parent.get(c), "codes": dict(code_of[c])} for c in ents},
                "properties": {c: {"instances": inst[c], "worlds": len(worlds_of[c]), "base": parent.get(c)} for c in props},
                "pairs": {f"{a} | {b}": v for (a, b), v in pair.items()},
                "eplinks": {f"{a} | {b} | {c}": v for (a, b, c), v in eplinks.items()}, "ep_unresolved": dict(epbad), "members": {k: a.to_json() for k, a in agg.items()}}
        with open(out_json, "w", encoding="utf-8") as f: json.dump(data, f, indent=1, default=str)
        print("wrote", out_json)
    print("entity -> .ep links:", sum(eplinks.values()), "unresolved:", sum(epbad.values()))


if __name__ == "__main__":
    main()
