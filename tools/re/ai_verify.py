"""Re-runs the measurements behind docs/re/ai.md on an install (read-only).

Character behaviors (`.cb`): states, distance bands, movers, actions per moment, ranges, alignment.
World data (`.wld`, the N smallest worlds
with `--worlds N`): spawners, tactics, path markers, formations, navigation mesh objects.

Usage: python -I tools/re/ai_verify.py [DATA_DIR] [--worlds N] [--table]
       (--table prints one row per behavior, as used in the spec)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Enum, Obj, Ref, TreeDec, merged_files, parse, u32  # noqa: E402

NULL = 0xFFFFFFFF


def members(o, cls):
    return {k.split(".", 1)[1]: v for k, v in o if k.startswith(cls + ".")}


def load(d):
    r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects(); return r, dec, objs, edob


def behaviors(install, st, dist, table):
    rows = []
    for name, d in sorted(merged_files(install, ".cb").items()):
        r, dec, objs, edob = load(d); st["cb files"] += 1
        idn = lambda v, r=r: r["idnt"][v] if isinstance(v, int) and 0 <= v < len(r["idnt"]) else v
        cb = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CCharacterBehavior")
        m = members(cb, "CCharacterBehavior")
        states = [objs[x.oid] if isinstance(x, Ref) else x for x in m["cb_acsCharacterStates"]]
        areas = 0; movers = collections.Counter(); acts = collections.Counter(); moments = collections.Counter()
        def walk_actions(h):
            for lst in members(h, "CAIMomentHandler")["caimh_amhMomentHandlers"]:
                lm = members(lst, "CBehaviorActionList")
                moments[idn(lm["bal_idMoment"])] += 1
                for a in lm.get("bal_abaActions", []) or []:
                    o = objs[a.oid] if isinstance(a, Ref) and a.oid != NULL else a
                    if isinstance(o, Obj): acts[o.type] += 1
        walk_actions(m["gdc_ahActionHandler"])
        for s in states:
            sm = members(s, "CCharacterState")
            for medium in ("cs_dcLand", "cs_dcAir", "cs_dcWater"):
                dca = members(sm[medium], "CDipContentAreas")
                for a in dca["dca_ccbaCustomBehaviorAreas"]:
                    ao = objs[a.oid] if isinstance(a, Ref) else a; am = members(ao, "CCustomBehaviorArea"); areas += 1
                    mv = am["cba_pmcMoveCharacter"]
                    movers[objs[mv.oid].type if isinstance(mv, Ref) and mv.oid != NULL else "(none)"] += 1
                    walk_actions(am["cba_mhAIMomentHandler"])
            walk_actions(sm["cs_mhAIMomentHandler"])
        dist["behavior alignment (cb_baAligment)"][m["cb_baAligment"].v if isinstance(m["cb_baAligment"], Enum) else m["cb_baAligment"]] += 1
        dist["active range"][m["cb_fActiveRange"]] += 1; dist["sense range"][m["cb_fSenseRange"]] += 1; dist["threat range"][m["cb_fThreatRange"]] += 1
        dist["states per behavior"][len(states)] += 1; dist["movers used in bands"].update(movers); dist["action classes (all moments)"].update(acts); dist["moments with a handler list"].update(moments)
        dist["based-on pointer null"][isinstance(m["cb_pcbBasedOnCharacter"], Ref) and m["cb_pcbBasedOnCharacter"].oid == NULL] += 1
        rows.append((idn(m["cb_idName"]), name.rsplit("/", 1)[-1], m["cb_baAligment"].v if isinstance(m["cb_baAligment"], Enum) else m["cb_baAligment"], m["cb_fActiveRange"], m["cb_fSenseRange"], m["cb_fThreatRange"], len(states), areas, sum(acts.values())))
    if table:
        print("| name | file | align | active | sense | threat | states | bands | actions |"); print("|---|---|---|---|---|---|---|---|---|")
        for r_ in rows: print("| " + " | ".join(str(x) for x in r_) + " |")


def worlds(install, n, st, dist):
    want = {"SSpawnerProperties", "STacticProperties", "SPathMarkerProperties", "CNavMesh", "CMarkersFormation", "CLineFormation", "CMCMarkersMovingOrdered", "SMultiSpawnerProperties"}
    ws = merged_files(install, ".wld")
    for name in sorted(ws, key=lambda k: len(ws[k]))[:n]:
        r, dec, objs, edob = load(ws[name]); extra, _ = dec.extra_objects(edob); st["worlds read"] += 1
        for o in list(objs.values()) + list(extra.values()):
            if isinstance(o, Obj) and o.type in want:
                st[f"world objects {o.type}"] += 1


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    wn = int(sys.argv[sys.argv.index("--worlds") + 1]) if "--worlds" in sys.argv else 0
    args = [a for a in args if not a.isdigit()]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter)
    behaviors(install, st, dist, "--table" in sys.argv)
    if wn: worlds(install, wn, st, dist)
    for k, v in sorted(st.items()): print(f"{v:6d}  {k}")
    for k, v in sorted(dist.items()): print(k, dict(v.most_common(14)))


if __name__ == "__main__":
    main()
