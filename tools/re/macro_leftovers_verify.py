"""Evidence for docs/re/wld-macro.md open question 2 (T01-065): the macro language leftovers.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's
binaries).

It answers, from the data alone:
  1. The 12 `Run`/`RunAsync` targets that name no function of their own program: which function
     name, and whether another program anywhere in the install has a function of that name (the
     "runs on another object's program" hypothesis of wld-macro.md section "Macro entities").
  2. The 23 macro entities whose function (`.11`) is not a function of their own program: which
     name, and the same cross-program search.
  3. Samples of marker kind 1 and kind 15 (`CMacroSourceMarker.5`): the exact source substring each
     one highlights, so a reader can classify them directly.
  4. `CMacroSource.2`, `CMacroProgram.5` and `CMacroEntity.4`: value distributions and a check of
     each against other known fields (source line count, function count, entity record presence).
  5. The "else" clause hypothesis for `CMacroStatementIf.4`: per program, the count of bare
     `else {` occurrences in the stored text (not `else If`) against the count of `.4`-set
     `CMacroStatementIf` objects reachable in that program.

Usage: python -I tools/re/macro_leftovers_verify.py [DATA_DIR]   (DATA_DIR defaults to $SS2_DATA_DIR; ~15 minutes)
"""
import collections, os, re, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, merged_files, parse  # noqa: E402

NULL = 0xFFFFFFFF


def ids(v):
    return [x.oid for x in (v or []) if isinstance(x, Ref) and x.oid != NULL]


def children(o):
    out = []
    for k, v in o:
        if isinstance(v, Ref):
            out.append(v.oid)
        elif isinstance(v, Obj):
            out += children(v)
        elif isinstance(v, list):
            for x in v:
                if isinstance(x, Ref):
                    out.append(x.oid)
                elif isinstance(x, Obj):
                    out += children(x)
    return out


def reach_from(objs, roots):
    reach = set()
    stack = list(roots)
    while stack:
        i = stack.pop()
        if i == NULL or i in reach or i not in objs or not isinstance(objs[i], Obj):
            continue
        reach.add(i)
        stack += children(objs[i])
    return reach


def program_functions(objs, idnt, prog):
    items = [objs[x] for x in ids(prog.get("CMacroProgram.2")) if x in objs]
    funcs = [f for f in items if isinstance(f, Obj) and f.type == "CMacroFunction"]
    return funcs, {idnt[f.get("CMacroFunction.2")] for f in funcs}


def main():
    install = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("SS2_DATA_DIR")
    if not install:
        print("usage: macro_leftovers_verify.py <install>  (or set SS2_DATA_DIR)")
        return 1

    all_func_names = collections.defaultdict(set)  # function name -> {(file, program oid)} that has it
    programs = []  # (file, oid, Obj, objs, idnt)

    files = sorted(merged_files(install, ".mac").items()) + sorted(merged_files(install, ".wld").items())
    for n, (name, d) in enumerate(files):
        r = parse(d)
        dec = TreeDec(d, r)
        objs, edob = dec.objects()
        idnt = r["idnt"]
        for oid, o in objs.items():
            if isinstance(o, Obj) and o.type == "CMacroProgram":
                programs.append((name, oid, o, objs, idnt))
                _, names = program_functions(objs, idnt, o)
                for fn in names:
                    all_func_names[fn].add((name.rsplit("/", 1)[-1], oid))
        print(f"[{n + 1}/{len(files)}] {name.rsplit('/', 1)[-1]}", file=sys.stderr, flush=True)

    print(f"Indexed {len(programs)} programs, {len(all_func_names)} distinct function names.\n")

    # ---- 1. run targets not in their own program ----
    print("-- 1. Run/RunAsync targets naming no function of their own program --")
    for name, pid, prog, objs, idnt in programs:
        _, own = program_functions(objs, idnt, prog)
        reach = reach_from(objs, [f.get("CMacroFunction.1").oid for f in program_functions(objs, idnt, prog)[0]])
        for i in reach:
            o = objs[i]
            if o.type not in ("CMacroStatementRunMacro", "CMacroStatementRunAsyncMacro"):
                continue
            tgt = idnt[o.get("CMacroStatementRunMacro.3")]
            if tgt in own:
                continue
            elsewhere = sorted(all_func_names.get(tgt, []))
            print(f"  {name.rsplit('/', 1)[-1]} prog {pid}: Run target {tgt!r} -> elsewhere: {elsewhere[:5]}" + (" ..." if len(elsewhere) > 5 else ""))

    # ---- 2. macro entities whose function is not in their own program ----
    print("\n-- 2. Macro entities whose function (.11) is not a function of their own program --")
    for name, d in sorted(merged_files(install, ".wld").items()):
        r = parse(d)
        dec = TreeDec(d, r)
        objs, edob = dec.objects()
        idnt = r["idnt"]
        prog_by_oid = {oid: o for oid, o in objs.items() if isinstance(o, Obj) and o.type == "CMacroProgram"}
        w = next((o for o in objs.values() if isinstance(o, Obj) and o.type == "CWorld"), None)
        if w is None:
            continue
        for i in ids(w.get("CWorld.1")):
            e = objs.get(i)
            if not isinstance(e, Obj) or e.type != "CMacroEntity":
                continue
            pr = e.get("CMacroEntity.3").oid
            prog = prog_by_oid.get(pr)
            if prog is None:
                continue  # external .mac, not checked here (wld_macro_verify.py already does)
            _, own = program_functions(objs, idnt, prog)
            fn = idnt[e.get("CMacroEntity.11")]
            if fn in own:
                continue
            elsewhere = sorted(all_func_names.get(fn, []))
            print(f"  {name.rsplit('/', 1)[-1]} entity {i}: function {fn!r}, own program has {sorted(own)} -> elsewhere: {elsewhere[:5]}" + (" ..." if len(elsewhere) > 5 else ""))

    # ---- 3. marker kind 1 and 15 samples ----
    print("\n-- 3. Marker kind 1 and 15 samples (exact source substring) --")
    kind_samples = collections.defaultdict(list)
    for name, pid, prog, objs, idnt in programs:
        src = prog.get("CMacroProgram.6")
        if src is None:
            continue
        text = src.get("CMacroSource.1") or ""
        lines = text.split("\n")
        for mref in src.get("CMacroSource.5") or []:
            m = objs.get(mref.oid) if isinstance(mref, Ref) else None
            if not isinstance(m, Obj):
                continue
            l0, l1, c0, c1, kind, flag = [m.get(f"CMacroSourceMarker.{k}") for k in range(1, 7)]
            if kind not in (1, 15):
                continue
            if l0 is None or l0 != l1 or l0 >= len(lines):
                continue
            line = lines[l0]
            if c0 is None or c1 is None or not (0 <= c0 <= c1 < len(line) + 1):
                continue
            snippet = line[c0:c1 + 1]
            if len(kind_samples[kind]) < 15:
                kind_samples[kind].append((name.rsplit("/", 1)[-1], snippet, flag))
    for kind in (1, 15):
        print(f"  kind {kind}:")
        for fname, snippet, flag in kind_samples[kind]:
            print(f"    {fname}: {snippet!r} (flag={flag})")

    # ---- 4. CMacroSource.2 / CMacroProgram.5 / CMacroEntity.4 ----
    print("\n-- 4. CMacroSource.2, CMacroProgram.5, CMacroEntity.4 --")
    src2_vs_lines = collections.Counter()
    prog5_vs_funcs = collections.defaultdict(collections.Counter)
    for name, pid, prog, objs, idnt in programs:
        src = prog.get("CMacroProgram.6")
        funcs, _ = program_functions(objs, idnt, prog)
        p5 = prog.get("CMacroProgram.5")
        prog5_vs_funcs[p5]["n_functions"] += len(funcs)
        prog5_vs_funcs[p5]["n_programs"] += 1
        if src is not None:
            text = src.get("CMacroSource.1") or ""
            nlines = text.count("\n")
            s2 = src.get("CMacroSource.2")
            src2_vs_lines[(s2 == nlines, s2 == nlines + 1, s2 == len(text))] += 1
    print("  CMacroSource.2 vs line count: (==linebreaks, ==linebreaks+1, ==byte length) counts:", dict(src2_vs_lines))
    print("  CMacroProgram.5 vs function/program counts:", {k: dict(v) for k, v in prog5_vs_funcs.items()})

    entity4_vs = collections.Counter()
    for name, d in sorted(merged_files(install, ".wld").items()):
        r = parse(d)
        dec = TreeDec(d, r)
        objs, edob = dec.objects()
        try:
            extra, _ = dec.extra_objects(edob)
        except Exception:
            extra = {}
        allo = {**objs, **extra}
        w = next((o for o in objs.values() if isinstance(o, Obj) and o.type == "CWorld"), None)
        if w is None:
            continue
        recs = {extra[p].get("CEntityProperties.2").oid: extra[p] for p in ids(w.get("CWorld.5")) if p in extra}
        for i in ids(w.get("CWorld.1")):
            e = objs.get(i)
            if not isinstance(e, Obj) or e.type != "CMacroEntity":
                continue
            e4 = e.get("CMacroEntity.4")
            has_rec = i in recs
            entity4_vs[(e4, has_rec)] += 1
    print("  CMacroEntity.4 vs has-a-world-record: (.4 value, has record) counts:", dict(entity4_vs))

    # ---- 5. the "else" hypothesis ----
    print("\n-- 5. CMacroStatementIf.4 vs bare 'else {' count in the stored text --")
    totals = collections.Counter()
    mismatches = 0
    for name, pid, prog, objs, idnt in programs:
        src = prog.get("CMacroProgram.6")
        if src is None:
            continue
        text = src.get("CMacroSource.1") or ""
        clean = re.sub(r'"[^"\n]*"', '""', text)
        clean = re.sub(r"//[^\n]*", "", clean)
        bare_else = len(re.findall(r"\belse\s*\{", clean, re.I))
        funcs, _ = program_functions(objs, idnt, prog)
        reach = reach_from(objs, [f.get("CMacroFunction.1").oid for f in funcs])
        flagged = sum(1 for i in reach if objs[i].type == "CMacroStatementIf" and isinstance(objs[i].get("CMacroStatementIf.4"), Ref) and objs[i].get("CMacroStatementIf.4").oid != NULL)
        totals["bare_else_in_text"] += bare_else
        totals["flag_4_set"] += flagged
        if bare_else != flagged:
            mismatches += 1
    print(f"  total bare 'else {{' in text: {totals['bare_else_in_text']}, total CMacroStatementIf.4 set: {totals['flag_4_set']}, programs where they differ: {mismatches} of {len(programs)}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
