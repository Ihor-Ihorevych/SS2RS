"""Re-runs the measurements behind docs/re/level-logic.md on an install (read-only).

Resolves every object expression of every macro program (actions, events, calls, sent events) to the entity
class it addresses, using the variable bindings of the macro entity that runs the program, and writes the
resulting "what scripts do to which entity class" tables. Also checks how scripts name entities, the chapter
chain and the level-finish events.

Usage: python -I tools/re/wld_logic_verify.py [DATA_DIR] [LIMIT] [--api OUT.md]
       (DATA_DIR defaults to $SS2_DATA_DIR; LIMIT = only the LIMIT smallest files, default all, about 20 minutes)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Blob, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
from wld_root_verify import ids  # noqa: E402

NULL = 0xFFFFFFFF


def kids(o):
    out = []
    for k, v in o:
        if isinstance(v, Ref): out.append(v.oid)
        elif isinstance(v, Obj): out += kids(v)
        elif isinstance(v, list) and not isinstance(v, Blob):
            for x in v:
                if isinstance(x, Ref): out.append(x.oid)
                elif isinstance(x, Obj): out += kids(x)
    return out


def check_world(name, d, st, dist, api, mac_progs):
    r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects()
    try: extra, _ = dec.extra_objects(edob)
    except Exception: extra = {}
    allo = {**objs, **extra}; idnt = r["idnt"]; short = name.rsplit("/", 1)[-1]
    rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}
    w = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CWorld"); ents = ids(w.get("CWorld.1"))
    recs = {extra[p].get("CEntityProperties.2").oid: extra[p] for p in ids(w.get("CWorld.5")) if p in extra}
    cls = {i: objs[i].type for i in ents}
    st["worlds"] += 1
    # chapters
    chs = [i for i in ents if cls[i] == "CChapterInfoEntity"]; wi = [i for i in ents if cls[i] == "CWorldInfoEntity"][0]
    first = objs[wi].get("CWorldInfoEntity.2").oid
    nxt = {i: objs[i].get("CChapterInfoEntity.2").oid for i in chs}
    st["chapter info entities"] += len(chs)
    if chs:
        st["worlds with chapters"] += 1; st["... whose world info names the first chapter (.2)"] += first in objs and first in nxt
        heads = [i for i in chs if i not in set(nxt.values())]
        st["... chapter chains that are linear (every chapter is the next of at most one)"] += len([v for v in nxt.values() if v != NULL]) == len({v for v in nxt.values() if v != NULL})
        ch = 0; cur = first; seen = set()
        while cur in nxt and cur not in seen: seen.add(cur); ch += 1; cur = nxt[cur]
        st["... chapters reached from the first by the next handle"] += ch; dist["chapters per world"][len(chs)] += 1
    # macro entities and their bindings
    for ei in ents:
        e = objs[ei]
        if e.type != "CMacroEntity": continue
        pr = e.get("CMacroEntity.3").oid
        if pr in objs and isinstance(objs[pr], Obj) and objs[pr].type == "CMacroProgram": prog = objs[pr]; pobjs, pidnt = objs, idnt
        else:
            entry = mac_progs.get(ex.get(pr, "").lower())
            if entry is None: continue
            pobjs, pidnt, prog = entry
        bound = {}
        for v in e.get("CMacroEntity.2"):
            vv = dict(v); nm = idnt[vv["CEntityVariable.1"]]
            if not nm: continue
            imp = objs.get([x for _, x in vv["CEntityVariable.3"]][0].oid)
            cl = set(); names = set()
            if isinstance(imp, Obj) and imp.type == "CVariantImpHANDLE":
                h = imp.get("CVariantImpHANDLE.1").oid
                if h in objs: cl.add(cls.get(h, objs[h].type)); names.add(h)
            elif isinstance(imp, Obj) and imp.type == "CVariantImpMetaHandleArray":
                for h in imp.get("CVariantImpMetaHandleArray.1"):
                    if h.oid in objs: cl.add(cls.get(h.oid, objs[h.oid].type)); names.add(h.oid)
            elif isinstance(imp, Obj) and imp.type == "CVariantImpResourcePointer":
                t = imp.get("CVariantImpResourcePointer.1"); cl.add("(resource ." + ex[t.oid].rsplit(".", 1)[-1].lower() + ")" if t.oid in ex else "(resource)")
            if cl: bound[nm] = (cl, names)
            # does the variable name equal the editor name of the entity it names?
            if isinstance(imp, Obj) and imp.type == "CVariantImpHANDLE" and imp.get("CVariantImpHANDLE.1").oid in recs:
                st["single-handle variables"] += 1; st["... whose name equals the entity's editor name (CEntityProperties.4)"] += recs[imp.get("CVariantImpHANDLE.1").oid].get("CEntityProperties.4") == nm
        # walk the program
        for fn in [pobjs[x] for x in ids(prog.get("CMacroProgram.2")) if pobjs[x].type == "CMacroFunction"]:
            this_cls = pidnt[fn.get("CMacroFunction.3")]; this = {"~" + this_cls} if this_cls else {"CMacroEntity"}
            hints = {}; stack = [fn.get("CMacroFunction.1").oid]; seenx = set(); nodes = []
            while stack:
                i = stack.pop()
                if i == NULL or i in seenx or i not in pobjs or not isinstance(pobjs[i], Obj): continue
                seenx.add(i); o = pobjs[i]; nodes.append(o); stack += kids(o)
            for o in nodes:
                if o.type == "CVarTypeHintStatement": hints[pidnt[o.get("CVarTypeHintStatement.1")]] = pidnt[o.get("CVarTypeHintStatement.2")]
            foreach = {}
            for o in nodes:
                if o.type == "CMacroStatementForeach": foreach[pidnt[o.get("CMacroStatementForeach.1")]] = pidnt[o.get("CMacroStatementForeach.2")]

            def resolve(oid, depth=0):
                o = pobjs.get(oid)
                if not isinstance(o, Obj): return ({"(none)"}, "none")
                if o.type == "CThisObjectExpression": return (this, "this")
                if o.type == "CNullObjectExpression": return ({"NULL"}, "none")
                if o.type == "CVarObjectExpression":
                    nm = pidnt[o.get("CVarObjectExpression.1")]
                    if nm in bound: return (bound[nm][0], "bound")
                    if nm in foreach and foreach[nm] in bound and depth < 3: return (bound[foreach[nm]][0], "bound")
                    if nm in hints: return ({"~" + hints[nm]}, "hint")
                    return ({"?" + ("var" if nm else "empty")}, "unresolved")
                if o.type == "CFunctionObjectExpression": return ({"(result of a call)"}, "unresolved")
                return ({"(expression)"}, "unresolved")

            def add(kind, oid, nm, kindsrc):
                c, how = resolve(oid)
                dist[f"{kind}: how the object resolved"][how] += 1
                for cc in c: a = api[(cc, kind, nm)]; a[0] += 1; a[1].add(short)
            for o in nodes:
                t = o.type
                if t in ("CObjectActionStatement", "CObjectActionAsyncStatement"):
                    add("action" if t == "CObjectActionStatement" else "action (async)", o.get("CObjectActionStatement.1").oid, pidnt[o.get("CObjectActionStatement.2")], t)
                elif t == "CEventBoolExpression":
                    for ev in o.get("CEventBoolExpression.5"): add("event", ev.get("CSingleEvent.1").oid, pidnt[ev.get("CSingleEvent.2")], t)
                elif t == "CMacroStatementNotify": add("send event", o.get("CMacroStatementNotify.1").oid, pidnt[o.get("CMacroStatementNotify.2")], t)
                elif t == "CFunctionObjectExpression": add("call", o.get("CFunctionObjectExpression.1").oid, pidnt[o.get("CFunctionObjectExpression.2")], t)
                elif t in ("CMacroStatementRunMacro", "CMacroStatementRunAsyncMacro"): add("run", o.get("CMacroStatementRunMacro.2").oid, pidnt[o.get("CMacroStatementRunMacro.3")], t)


def main():
    args = sys.argv[1:]; api_out = args[args.index("--api") + 1] if "--api" in args else None
    if api_out: args = [a for a in args if a not in ("--api", api_out)]
    install = args[0] if args and not args[0].isdigit() else os.environ.get("SS2_DATA_DIR", "")
    limit = int(args[-1]) if args and args[-1].isdigit() else None
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter); api = collections.defaultdict(lambda: [0, set()])
    mac_progs = {}
    for name, d in merged_files(install, ".mac").items():
        r = parse(d); objs, _ = TreeDec(d, r).objects(); p = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CMacroProgram")
        mac_progs[name.lower()] = (objs, r["idnt"], p)
    files = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))
    for n, (name, d) in enumerate(files):
        if limit and n >= limit: break
        check_world(name, d, st, dist, api, mac_progs)
        print(f"[{n + 1}/{len(files)}] {name.rsplit('/', 1)[-1]}", file=sys.stderr, flush=True)
    for k, v in sorted(st.items()):
        if v: print(f"{v:9d}  {k}")
    for k, v in sorted(dist.items()): print(k, dict(v.most_common(10)))
    if api_out:
        by = collections.defaultdict(lambda: collections.defaultdict(list))
        for (c, kind, nm), (uses, ws) in api.items(): by[c][kind].append((nm, uses, len(ws)))
        tot = {c: sum(u for k in by[c].values() for _, u, _ in k) for c in by}
        lines = ["# What level scripts do to which entity class (generated)", "",
                 "Generated by `tools/re/wld_logic_verify.py --api` from the macro programs of the retail install (59 worlds and the 8 `.mac` files). Every object expression of an action, call, event, `Send event` and `Run` is resolved to the **entity class** it addresses through the variable bindings of the macro entity that runs the program (`wld-macro.md`). "
                 "A class written `~Name` is only a **declared** type of a script variable (a `var` hint or a function's class restriction); `(result of a call)`, `(expression)` and `?var` are expressions that were not resolved. "
                 "Each row: the name used, its number of uses and the number of worlds that use it. What a name does is the subject of `level-logic.md`.", ""]
        for c in sorted(by, key=lambda c: -tot[c]):
            lines += [f"## `{c}` ({tot[c]:,} uses)", ""]
            for kind in ("event", "action", "action (async)", "call", "send event", "run"):
                rows = sorted(by[c].get(kind, []), key=lambda t: (-t[1], t[0]))
                if not rows: continue
                lines.append(f"**{kind}** ({len(rows)} names): " + ", ".join(f"`{n or '(empty)'}` {u}/{w}" for n, u, w in rows[:40]) + (f", … {len(rows) - 40} more" if len(rows) > 40 else ""))
                lines.append("")
        with open(api_out, "w", encoding="utf-8", newline="\n") as f: f.write("\n".join(lines) + "\n")
        print("wrote", api_out, f"({len(by)} classes)")


if __name__ == "__main__":
    main()
