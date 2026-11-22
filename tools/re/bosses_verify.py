"""Re-runs the measurements behind docs/re/bosses.md on an install (read-only).

1. The boss puppet parameter files (`Puppets/Bosses/*.ep`): health, healing members, attacks (shooting processes,
   melee blasts), the behavior file's states, areas and actions (`.cb`).
2. The worlds: which entities use a boss parameter file, and for every macro program that runs in such a world the events,
   actions, calls and sent events that the scripts address to the boss (through the variables bound to the boss entities),
   the functions of the program and the `RegisterBoss` calls on the world info.
Writes the tables with --table.

Usage: python -I tools/re/bosses_verify.py [DATA_DIR] [LIMIT] [--table OUT.md]
       (DATA_DIR defaults to $SS2_DATA_DIR; LIMIT = only the LIMIT smallest worlds, default all, about 15 minutes)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Blob, Enum, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
from wld_root_verify import ids  # noqa: E402
from wld_logic_verify import kids  # noqa: E402
import enemies_verify as E  # noqa: E402

NULL = 0xFFFFFFFF
BOSSES = ("dragon", "kleerofski", "kwongo", "princchan", "pyramidslow", "transformer", "zumzum")


def base(p): return p.rsplit("/", 1)[-1]


def main():
    args = sys.argv[1:]; table = args[args.index("--table") + 1] if "--table" in args else None
    if table: args = [a for a in args if a not in ("--table", table)]
    install = args[0] if args and not args[0].isdigit() else os.environ.get("SS2_DATA_DIR", "")
    limit = int(args[-1]) if args and args[-1].isdigit() else None
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    # 1a. behavior files
    cbs = {}
    for n, d in sorted(merged_files(install, ".cb").items()):
        r = parse(d); objs, _ = TreeDec(d, r).objects(); idnt = r["idnt"]
        rs = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CResourceFile")
        rt = objs[[x.oid for x in rs.get("CResourceFile.rf_crsShared")][0]]
        name = base(n)[:-3]; states = []
        for cs in rt.get("CCharacterBehavior.cb_acsCharacterStates"):
            cs = objs[cs.oid] if isinstance(cs, Ref) else cs; acts = collections.Counter(); areas = 0
            for o in E.members(cs, objs, "CCustomBehaviorArea"): areas += 1
            for o in objs.values():
                pass
            sub = []
            for typ in ("SBAShoot", "SBAMelee", "SBALocalMove", "SBAAnimation", "SBARunOverFoe", "CBAChangeMovingAbilities", "SBALeap"):
                acts[typ] = len(E.members(cs, objs, typ))
            states.append((idnt[cs.get("CCharacterState.cs_idState")], areas, {k: v for k, v in acts.items() if v}))
        cbs[name.lower()] = (name, rt.get("CCharacterBehavior.cb_fActiveRange"), rt.get("CCharacterBehavior.cb_fSenseRange"), rt.get("CCharacterBehavior.cb_fThreatRange"), states)
    print("boss behavior files:")
    for b in BOSSES:
        k = "pyramid" if b == "pyramidslow" else ("princchan" if b == "princchan" else b)
        if k in cbs: print("  ", cbs[k][0].ljust(12), "ranges (active, sense, threat)", cbs[k][1:4], "| states:", [(s[0], s[1], s[2]) for s in cbs[k][4]])
    # 1b. boss puppet files
    prow = {}
    eps = merged_files(install, ".ep")
    for n, d in sorted(eps.items()):
        rel = n.split("databases/")[-1]
        if not rel.startswith("puppets/bosses/") or base(n)[:-3] not in BOSSES: continue
        r, objs, ex, rt = E.load(d)
        hp = next(v for k, v in rt if k.endswith("SPuppetParams.1") and isinstance(v, Obj)); hv = {k.split(".", 1)[-1]: (round(x, 2) if isinstance(x, float) else x) for k, x in hp}
        prow[base(n)[:-3]] = hv
    print("boss health members (`SPuppetHealthParams`):")
    for b, hv in sorted(prow.items()): print("  ", b.ljust(12), hv)
    # 2. worlds
    wb = {}   # world -> {boss: [entity ids]}
    api = collections.defaultdict(lambda: collections.defaultdict(collections.Counter))   # world -> boss -> (kind, name) -> count
    funcs = collections.defaultdict(list); reg = collections.defaultdict(int); events_w = collections.defaultdict(collections.Counter)
    files = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))
    mac_progs = {}
    for n, d in sorted(merged_files(install, ".mac").items()):
        r = parse(d); objs, _ = TreeDec(d, r).objects(); rs = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CResourceFile")
        mac_progs[n.lower()] = (objs, r["idnt"], objs[[x.oid for x in rs.get("CResourceFile.rf_crsShared")][0]])
    for ni, (name, d) in enumerate(files):
        if limit and ni >= limit: break
        r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects()
        try: extra, _ = dec.extra_objects(edob)
        except Exception: extra = {}
        rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}; idnt = r["idnt"]
        w = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CWorld"); ents = ids(w.get("CWorld.1")); wn = name.rsplit("/", 1)[-1][:-4]
        recs = {extra[p].get("CEntityProperties.2").oid: extra[p] for p in ids(w.get("CWorld.5")) if p in extra}
        boss_ent = {}
        for i in ents:
            e = objs[i]; pp = next((v for k, v in e if k.endswith("en_pppPuppetParams")), None)
            if isinstance(pp, Ref) and pp.oid in ex and base(ex[pp.oid])[:-3].lower() in BOSSES: boss_ent[i] = base(ex[pp.oid])[:-3].lower()
        print(f"[{ni + 1}/{len(files)}] {wn}" + (f"  BOSS {sorted(set(boss_ent.values()))} x{len(boss_ent)}" if boss_ent else ""), file=sys.stderr, flush=True)
        if not boss_ent: continue
        wb[wn] = collections.Counter(boss_ent.values())
        for ei in ents:
            e = objs[ei]
            if e.type == "CWorldInfoEntity": pass
        for ei in ents:
            e = objs[ei]
            if e.type != "CMacroEntity": continue
            pr = e.get("CMacroEntity.3").oid
            if pr in objs and isinstance(objs[pr], Obj) and objs[pr].type == "CMacroProgram": prog = objs[pr]; pobjs, pidnt = objs, idnt
            else:
                entry = mac_progs.get(ex.get(pr, "").lower())
                if entry is None: continue
                pobjs, pidnt, prog = entry
            bossvar = {}
            for v in e.get("CMacroEntity.2"):
                vv = dict(v); nm = idnt[vv["CEntityVariable.1"]]
                if not nm: continue
                imp = objs.get([x for _, x in vv["CEntityVariable.3"]][0].oid)
                hs = []
                if isinstance(imp, Obj) and imp.type == "CVariantImpHANDLE": hs = [imp.get("CVariantImpHANDLE.1").oid]
                elif isinstance(imp, Obj) and imp.type == "CVariantImpMetaHandleArray": hs = [h.oid for h in imp.get("CVariantImpMetaHandleArray.1")]
                bs = {boss_ent[h] for h in hs if h in boss_ent}
                if bs: bossvar[nm] = sorted(bs)[0]
            if not bossvar: continue
            for fn in [pobjs[x] for x in ids(prog.get("CMacroProgram.2")) if pobjs[x].type == "CMacroFunction"]:
                fname = pidnt[fn.get("CMacroFunction.2")] if fn.get("CMacroFunction.2") is not None else ""
                stack = [fn.get("CMacroFunction.1").oid]; seen = set(); nodes = []
                while stack:
                    i = stack.pop()
                    if i == NULL or i in seen or i not in pobjs or not isinstance(pobjs[i], Obj): continue
                    seen.add(i); o = pobjs[i]; nodes.append(o); stack += kids(o)
                def who(oid):
                    o = pobjs.get(oid)
                    if isinstance(o, Obj) and o.type == "CVarObjectExpression": return bossvar.get(pidnt[o.get("CVarObjectExpression.1")])
                    return None
                hit = collections.Counter()
                for o in nodes:
                    t = o.type
                    if t in ("CObjectActionStatement", "CObjectActionAsyncStatement"):
                        b = who(o.get("CObjectActionStatement.1").oid)
                        if b: hit[(b, "action" + (" (async)" if "Async" in t else ""), pidnt[o.get("CObjectActionStatement.2")])] += 1
                    elif t == "CEventBoolExpression":
                        for ev in o.get("CEventBoolExpression.5"):
                            b = who(ev.get("CSingleEvent.1").oid)
                            if b: hit[(b, "event", pidnt[ev.get("CSingleEvent.2")])] += 1
                    elif t == "CMacroStatementNotify":
                        b = who(o.get("CMacroStatementNotify.1").oid)
                        if b: hit[(b, "send event", pidnt[o.get("CMacroStatementNotify.2")])] += 1
                    elif t == "CFunctionObjectExpression":
                        b = who(o.get("CFunctionObjectExpression.1").oid)
                        if b: hit[(b, "call", pidnt[o.get("CFunctionObjectExpression.2")])] += 1
                    elif t == "CObjectActionStatement" and pidnt[o.get("CObjectActionStatement.2")] == "RegisterBoss": reg[wn] += 1
                for (b, kind, nm), c in hit.items(): api[wn][b][(kind, nm)] += c
                if hit: funcs[wn].append(fname)
    print("\nworlds with bosses:", {w: dict(c) for w, c in wb.items()})
    for wn, bs in api.items():
        for b, c in bs.items():
            print(f"\n{wn} / {b}: functions that address it: {sorted(set(funcs[wn]))[:12]}")
            for kind in ("event", "action", "action (async)", "send event", "call"):
                row = sorted(((nm, n) for (k, nm), n in c.items() if k == kind), key=lambda t: -t[1])
                if row: print("   ", kind.ljust(15), ", ".join(f"{nm} {n}" for nm, n in row[:22]))
    if table:
        L = ["# Boss fights (generated)", "", "Generated by `tools/re/bosses_verify.py --table`. For every world that places a boss parameter file: the number of boss entities and, for the scripts of that world, the events, actions, calls and sent events addressed to the boss through macro variables bound to the boss entities (name and number of uses). See `bosses.md`.", ""]
        for wn, bs in sorted(api.items()):
            for b, c in sorted(bs.items()):
                L += [f"## {wn} — `{b}` ({wb[wn][b]} entity)", ""]
                for kind in ("event", "action", "action (async)", "send event", "call"):
                    row = sorted(((nm, n) for (k, nm), n in c.items() if k == kind), key=lambda t: (-t[1], t[0]))
                    if row: L.append(f"- **{kind}:** " + ", ".join(f"`{nm}` ×{n}" for nm, n in row))
                L.append("")
        with open(table, "w", encoding="utf-8", newline="\n") as f: f.write("\n".join(L) + "\n")
        print("wrote", table)


if __name__ == "__main__":
    main()
