"""Re-runs the measurements behind docs/re/wld-macro.md on an install (read-only).

For every macro program (`CMacroProgram`) of the format-8 `.wld` worlds and of the `.mac` files: the functions,
the syntax tree (statements, expressions, handlers), the stored source text with its token markers, the code
segment ids, and how the macro entities bind variables and pick a function; plus the vocabulary (actions, events,
calls) used by the programs.

Usage: python -I tools/re/wld_macro_verify.py [DATA_DIR] [LIMIT] [--vocab OUT.md]
       (DATA_DIR defaults to $SS2_DATA_DIR; LIMIT = only the LIMIT smallest files, default all, about 15 minutes)
"""
import collections, os, re, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Blob, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
from wld_root_verify import ids  # noqa: E402

NULL = 0xFFFFFFFF
MACFUNCS = {}                      # lowercase path of a .mac file -> names of its functions (the .mac files are read before the worlds)
MACRO =re.compile(r"Macro|OnHandler|Variant|HintedVar|SingleEvent|ObjectAction|VarAssignment|VarTypeHint|Expression|IfClause")
# keyword in the stored source text -> class of the syntax tree object it becomes
KEYWORDS = [
    ("Macro (function)", r"(?m)^\s*Macro\s", "CMacroFunction"),
    ("On (handler)", r"\bOn\b", "COnHandler"),
    ("Action", r"\bAction\s", "CObjectActionStatement"),
    ("ActionAsync", r"\bActionAsync\s", "CObjectActionAsyncStatement"),
    ("Run", r"\bRun\s", "CMacroStatementRunMacro"),
    ("RunAsync", r"\bRunAsync\s", "CMacroStatementRunAsyncMacro"),
    ("Wait", r"\bWait\b", "CMacroStatementWait"),
    ("If", r"\bIf\b", "CMacroStatementIf"),
    ("for", r"\bfor\s*\(", "CMacroStatementFor"),
    ("foreach", r"\bforeach\b", "CMacroStatementForeach"),
    ("Send event", r"\bSend event\b", "CMacroStatementNotify"),
    ("Break", r"\bBreak\b", "CMacroStatementBreak"),
    ("ExitThread", r"\bExitThread\b", "CMacroStatementExitThread"),
    ("ExitAllThreads", r"\bExitAllThreads\b", "CMacroStatementExitAllThreads"),
    ("Print", r"\bPrint\b", "CMacroStatementPrint"),
    ("var / global var", r"\bvar\s", "CVarTypeHintStatement"),
    ("assignment", r"(?m)^\s*[A-Za-z_]\w*\s*=(?!=)", "CVarAssignmentStatement"),
]


def children(o):
    out = []
    for k, v in o:
        if isinstance(v, Ref): out.append(v.oid)
        elif isinstance(v, Obj): out += children(v)          # (an Obj is also a list: test it first)
        elif isinstance(v, list) and not isinstance(v, Blob):
            for x in v:
                if isinstance(x, Ref): out.append(x.oid)
                elif isinstance(x, Obj): out += children(x)
    return out


def strip_text(text):
    text = re.sub(r'"[^"\n]*"', '""', text)
    return re.sub(r"//[^\n]*", "", text)


def check_file(name, d, st, dist, vocab, is_world):
    r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects()
    try: extra, _ = dec.extra_objects(edob)
    except Exception: extra = {}
    allo = {**objs, **extra}; idnt = r["idnt"]; short = name.rsplit("/", 1)[-1]
    rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}
    progs = {i: o for i, o in objs.items() if isinstance(o, Obj) and o.type == "CMacroProgram"}
    st["files"] += 1; st["programs"] += len(progs)
    macro_objs = {i for i, o in objs.items() if isinstance(o, Obj) and MACRO.search(o.type) and o.type not in ("CMacroProgram", "CMacroSource", "CMacroSourceMarker", "CMacroDomain", "CMacroLatentID", "CMacroEntity", "CMacroProperties", "CMSHintedVar", "CMacroFunction", "CMacroFunctionNewLine", "CMacroFunctionComment")}
    reached_all = set()
    prog_names = {}
    for pid, p in progs.items():
        items = [objs[x] for x in ids(p.get("CMacroProgram.2"))]; src = p.get("CMacroProgram.6")
        funcs = [f for f in items if f.type == "CMacroFunction"]          # the list also holds CMacroFunctionNewLine / CMacroFunctionComment items
        for f in items:
            if f.type != "CMacroFunction": dist["non-function items in CMacroProgram.2"][f.type] += 1
        names = [idnt[f.get("CMacroFunction.2")] for f in funcs]; prog_names[pid] = set(names)
        for f in funcs:
            dist["function name"][idnt[f.get("CMacroFunction.2")]] += 1; dist["function class restriction (empty = any)"][idnt[f.get("CMacroFunction.3")] or "(any)"] += 1
        st["functions"] += len(funcs)
        # reachable syntax tree
        if not is_world: MACFUNCS[name.lower()] = {idnt[f.get("CMacroFunction.2")] for f in funcs}
        reach = set(); stack = [f.get("CMacroFunction.1").oid for f in items]       # also the empty blocks of the newline items
        while stack:
            i = stack.pop()
            if i == NULL or i in reach or i not in objs or not isinstance(objs[i], Obj): continue
            o = objs[i]
            if not MACRO.search(o.type): continue
            reach.add(i)
            for c in children(o):
                if c != NULL and c in objs and isinstance(objs[c], Obj): stack.append(c)
        reached_all |= reach
        cls = collections.Counter(objs[i].type for i in reach); cls["CMacroFunction"] += len(funcs)
        for k, v in cls.items(): dist["syntax tree class"][k] += v
        # parent pointers
        for i in reach:
            o = objs[i]
            if o.type == "CMacroStatementBlock":
                for s in ids(o.get("CMacroStatementBlock.1")):
                    st["block statements"] += 1; st["... whose parent pointer (CMacroStatement.2) is the block"] += objs[s].get("CMacroStatement.2").oid == i
                for h in ids(o.get("CMacroStatementBlock.2")):
                    st["handlers listed in a block (CMacroStatementBlock.2)"] += 1
                st["function body blocks (flag .3 = 1)"] += o.get("CMacroStatementBlock.3") == 1
                st["blocks with handlers whose flag .3 is 1"] += bool(ids(o.get("CMacroStatementBlock.2"))) and o.get("CMacroStatementBlock.3") == 1
                st["blocks with handlers"] += bool(ids(o.get("CMacroStatementBlock.2")))
        for f in funcs:
            b = f.get("CMacroFunction.1").oid; st["function bodies with flag .3 = 1"] += objs[b].get("CMacroStatementBlock.3") == 1
        # code segment ids
        segs = [objs[i].get("CMacroCodeSegment.1") for i in reach if objs[i].get("CMacroCodeSegment.1") is not None]
        real = [s for s in segs if s >= 0]
        st["syntax tree objects"] += len(reach); st["... with a code segment id (>= 0)"] += len(real)
        st["programs whose code segment ids are unique"] += len(set(real)) == len(real)
        st["programs: CMacroProgram.4 == highest segment id + 1"] += bool(real) and p.get("CMacroProgram.4") == max(real) + 1
        st["programs whose segment ids start at 64 or later"] += bool(real) and min(real) >= 64
        dist["lowest segment id (programs; 64 = never edited)"][min(real) if real and min(real) < 100 else ">= 100"] += 1
        # source text and markers
        text = src.get("CMacroSource.1"); lines = text.split("\n"); ms = [objs[x] for x in ids(src.get("CMacroSource.5"))]
        st["source markers"] += len(ms)
        okm = 0
        for m in ms:
            l0, l1, c0, c1, kind, flag = [m.get(f"CMacroSourceMarker.{k}") for k in range(1, 7)]
            if 0 <= l0 <= l1 < len(lines) and 0 <= c0 <= c1 < len(lines[l1]) + 1 and l0 == l1: okm += 1
            dist["marker (kind, flag)"][(kind, flag)] += 1
        st["... single-line and inside the text (columns inclusive)"] += okm
        st["programs: source lines vs CMacroSource.2 (equal)"] += len(lines) - 1 == src.get("CMacroSource.2")
        hv = [h.get("CMSHintedVar.mshv_strSource") for h in src.get("CMacroSource.msrc_ahvHintedVars")]
        for h in src.get("CMacroSource.msrc_ahvHintedVars"): dist["hinted variable type"][idnt[h.get("CMSHintedVar.mshv_idHintedDataType")] or "(none)"] += 1
        # keywords in the text against classes in the tree
        clean = strip_text(text); allmatch = True
        for label, rx, c in KEYWORDS:
            a = len(re.findall(rx, clean)); b = cls.get(c, 0)
            st[f"text keyword '{label}' count == {c} count (programs)"] += a == b
            if a != b: dist["keyword mismatches (text, tree)"][(label, a, b)] += 1; dist["programs with a keyword mismatch (file, object, text length)"][(short, pid, len(text))] += 1; allmatch = False
        st["programs where every keyword count matches"] += allmatch
        # vocabulary
        for i in reach:
            o = objs[i]; t = o.type
            if t in ("CObjectActionStatement", "CObjectActionAsyncStatement"):
                vocab[("action" + (" (async)" if t.endswith("AsyncStatement") else ""), idnt[o.get("CObjectActionStatement.2")], len(ids(o.get("CObjectActionStatement.4"))))][short] += 1
            elif t == "CEventBoolExpression":
                dist["event expression type (CEventBoolExpression.4)"][o.get("CEventBoolExpression.4").v] += 1
                dist["events per event expression"][len(o.get("CEventBoolExpression.5"))] += 1
                for ev in o.get("CEventBoolExpression.5"): vocab[("event", idnt[ev.get("CSingleEvent.2")], 0)][short] += 1
            elif t == "CMacroStatementNotify": vocab[("send event", idnt[o.get("CMacroStatementNotify.2")], 0)][short] += 1
            elif t == "CFunctionObjectExpression": vocab[("call", idnt[o.get("CFunctionObjectExpression.2")], len(ids(o.get("CFunctionObjectExpression.3"))))][short] += 1
            elif t in ("CMacroStatementRunMacro", "CMacroStatementRunAsyncMacro"):
                tgt = idnt[o.get("CMacroStatementRunMacro.3")]; vocab[("run", tgt, 0)][short] += 1
                st["run targets"] += 1; st["... naming a function of the same program"] += tgt in prog_names[pid]
    st["syntax tree objects not reachable from a function"] += len(macro_objs - reached_all)
    for i in macro_objs - reached_all: dist["not reachable: class"][objs[i].type] += 1
    st["macro syntax tree objects in all"] += len(macro_objs)
    if not is_world: return
    # macro entities
    w = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CWorld")
    recs = {extra[p].get("CEntityProperties.2").oid: extra[p] for p in ids(w.get("CWorld.5")) if p in extra}
    users = collections.Counter(); ents = set(ids(w.get("CWorld.1"))); ents_set = ents
    for i in ids(w.get("CWorld.1")):
        e = objs[i]
        if e.type != "CMacroEntity": continue
        st["macro entities"] += 1; pr = e.get("CMacroEntity.3").oid
        kind = "embedded program" if pr in progs else "external .mac" if pr in ex else "other"
        dist["macro entity program"][kind] += 1
        if pr in progs: users[pr] += 1
        fn = idnt[e.get("CMacroEntity.11")]
        if pr in progs:
            st["macro entities whose function name (.11) is a function of the program (embedded)"] += fn in prog_names[pr]
            if fn not in prog_names[pr]: dist["entity function name that is not a function of its program (name, functions in program)"][(fn, len(prog_names[pr]))] += 1
        elif pr in ex and ex[pr].lower() in MACFUNCS:
            st["... external .mac entities checked"] += 1; st["macro entities whose function name (.11) is a function of the program (external .mac)"] += fn in MACFUNCS[ex[pr].lower()]
        dist["macro entity function (.11)"][fn if fn in ("Main",) else "(other)"] += 1
        vars_ = e.get("CMacroEntity.2")
        st["variable bindings (CEntityVariable)"] += len(vars_)
        for v in vars_:
            var = [x for k, x in v if k.endswith(".3")][0]
            imp = objs.get([x for k, x in var if True][0].oid) if len(var) else None
            dist["variable value class"][imp.type if isinstance(imp, Obj) else "?"] += 1
            if isinstance(imp, Obj) and imp.type == "CVariantImpHANDLE":
                h = imp.get("CVariantImpHANDLE.1")
                if h.oid != NULL: st["entity variables: handle values (non-null)"] += 1; st["... that resolve to an entity of the world"] += h.oid in ents_set
                else: st["entity variables: null handles"] += 1
            elif isinstance(imp, Obj) and imp.type == "CVariantImpMetaHandleArray":
                for h in imp.get("CVariantImpMetaHandleArray.1"):
                    st["entity variables: handle array elements"] += 1; st["... that resolve to an entity of the world"] += h.oid in ents_set
            elif isinstance(imp, Obj) and imp.type == "CVariantImpResourcePointer":
                t = imp.get("CVariantImpResourcePointer.1")
                dist["resource pointer target"][("external ." + ex[t.oid].rsplit(".", 1)[-1].lower()) if t.oid in ex else ("object " + objs[t.oid].type if t.oid in objs and isinstance(objs[t.oid], Obj) else "null" if t.oid == NULL else "?")] += 1
        if pr in progs:
            hv = [h.get("CMSHintedVar.mshv_strSource") for h in progs[pr].get("CMacroProgram.6").get("CMacroSource.msrc_ahvHintedVars")]
            bound = {idnt[dict(v).get("CEntityVariable.1")] for v in vars_}
            st["hinted variables of embedded programs"] += len(hv); st["... whose name is a variable bound by the macro entity"] += sum(1 for n in hv if n in bound)
        rec = recs.get(i)
        if rec is not None and rec.type == "CMacroProperties":
            rv = rec.get("CMacroProperties.2")
            st["macro entities with a record"] += 1
            ev = [dict(x).get("CEntityVariable.1") for x in vars_]; rn = [dict(x).get("CEntityPropertyVariable.1") for x in rv]
            voids = [objs[[y for _, y in dict(x).get("CEntityVariable.3")][0].oid].type == "CVariantImpVoid" for x in vars_]
            st["record and entity have the same number of variables"] += len(ev) == len(rn)
            st["... each entity variable has the record's name, or is an unbound void entry with an empty name"] += len(ev) == len(rn) and all(a == b or (v and idnt[a] == "") for a, b, v in zip(ev, rn, voids))
            st["void variables (empty name, CVariantImpVoid)"] += sum(voids)
            st["record function name (.5) == entity function name (.11)"] += rec.get("CMacroProperties.5") == e.get("CMacroEntity.11")
    for pid in progs: dist["macro entities per embedded program"][min(users.get(pid, 0), 5)] += 1


def main():
    args = sys.argv[1:]; vocab_out = args[args.index("--vocab") + 1] if "--vocab" in args else None
    if vocab_out: args = [a for a in args if a not in ("--vocab", vocab_out)]
    install = args[0] if args and not args[0].isdigit() else os.environ.get("SS2_DATA_DIR", "")
    limit = int(args[-1]) if args and args[-1].isdigit() else None
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter); vocab = collections.defaultdict(collections.Counter)
    files = [(n, d, False) for n, d in sorted(merged_files(install, ".mac").items(), key=lambda kv: len(kv[1]))]
    files += [(n, d, True) for n, d in sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))]
    for n, (name, d, is_world) in enumerate(files):
        if limit and n >= limit: break
        check_file(name, d, st, dist, vocab, is_world); print(f"[{n + 1}/{len(files)}] {name.rsplit('/', 1)[-1]}", file=sys.stderr, flush=True)
    for k, v in sorted(st.items()): print(f"{v:9d}  {k}")
    for k, v in sorted(dist.items()): print(k, dict(v.most_common(45 if k == "syntax tree class" else 14)))
    if vocab_out:
        by = collections.defaultdict(list)
        for (kind, nm, args_), files_ in vocab.items(): by[kind].append((nm, args_, sum(files_.values()), len(files_)))
        lines = ["# Macro vocabulary (generated)", "", "Generated by `tools/re/wld_macro_verify.py --vocab` from the macro programs of the retail install (worlds and `.mac` files). "
                 "Names are the identifiers stored in the programs; **uses** counts syntax tree objects, **files** the worlds or `.mac` files that use the name, **args** the number of arguments at the call (a name can appear with several counts). "
                 "The semantics of each name (what the entity does) belong to the level logic spec (T01-030).", ""]
        for kind in sorted(by):
            lines += [f"## {kind} ({len(by[kind])} distinct)", "", "| Name | args | uses | files |", "|---|---|---|---|"]
            for nm, a, uses, nf in sorted(by[kind], key=lambda t: (-t[2], t[0])): lines.append(f"| `{nm or '(empty)'}` | {a} | {uses} | {nf} |")
            lines.append("")
        with open(vocab_out, "w", encoding="utf-8", newline="\n") as f: f.write("\n".join(lines) + "\n")
        print("wrote", vocab_out)


if __name__ == "__main__":
    main()
