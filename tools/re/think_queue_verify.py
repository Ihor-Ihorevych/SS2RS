"""Re-runs the measurements behind docs/re/main-loop.md section 7 and 5.1 (T01-087, read-only).

Over every world (`.wld`, all patches merged): the per-step entity list (`CScriptingDomain.2`,
items of class `CEntityStepper`) and the think queue (`CStateManager.10`, a `CEntryList` of
`CSMEntry`). Answers, from the data alone:

- what priority the shipped worlds actually register per-step entities with (`CEntityStepper.2`);
- whether the registration order already matches the world's own entity-list order (`CWorld.1`);
- whether any macro entity (`CMacroEntity`) is among the per-step entities (it never is: the macro
  interpreter's own per-step call is not reached through this list, `wld-macro.md`);
- the exact stored shape of a `CSMEntry` (which fields are live, which are always-`-1` sentinels);
- whether the stored queue is already sorted by due time, and whether any shipped world has two
  entries that share a due time (a tie, needed to exercise the tie-break rule `main-loop.md`/
  `foe-manager-numbers.md` section 9.4 already states from the binary).

Usage: python -I tools/re/think_queue_verify.py [DATA_DIR] [LIMIT]
       (DATA_DIR defaults to $SS2_DATA_DIR; LIMIT = only the LIMIT smallest worlds, default all,
       a few minutes)
"""
import collections
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, merged_files, parse  # noqa: E402

NULL = 0xFFFFFFFF
SENTINEL_FIELDS = (
    "CSMEntry.3", "CSMEntry.5", "CSMEntry.6", "CSMEntry.7",
    "CSMEntry.8", "CSMEntry.9", "CSMEntry.10", "CSMEntry.11",
)


def ids(refs):
    return [x.oid for x in refs if isinstance(x, Ref) and x.oid != NULL]


def check_world(name, d, st, dist):
    r = parse(d)
    dec = TreeDec(d, r)
    objs, _edob = dec.objects()
    worlds = [i for i, o in objs.items() if isinstance(o, Obj) and o.type == "CWorld"]
    if not worlds:
        return
    w = objs[worlds[0]]
    listed = ids(w.get("CWorld.1"))  # the world's own entity-list order
    pos_of = {eid: i for i, eid in enumerate(listed)}

    # --- 1. Step functions: CScriptingDomain.2, items of class CEntityStepper ---
    sd = objs[w.get("CWorld.8").oid]
    steppers = sd.get("CScriptingDomain.2")
    st["worlds"] += 1
    st["steppers"] += len(steppers)
    handles, classes = [], []
    for item in steppers:
        st["stepper item is a CEntityStepper"] += isinstance(item, Obj) and item.type == "CEntityStepper"
        hnd = item.get("CEntityStepper.1")
        pri = item.get("CEntityStepper.2")
        dist["CEntityStepper.2 (priority)"][pri] += 1
        h = hnd.oid if isinstance(hnd, Ref) else NULL
        handles.append(h)
        ent_cls = objs[h].type if h != NULL and h in objs else "?"
        classes.append(ent_cls)
        dist["stepper entity class -> priority"][(ent_cls, pri)] += 1
    positions = [pos_of.get(h, -1) for h in handles]
    st["stepper list order matches CWorld.1 entity-list order (non-decreasing position)"] += positions == sorted(positions)
    macro_entities = {i for i, o in objs.items() if isinstance(o, Obj) and o.type == "CMacroEntity"}
    st["macro entities of this world"] += len(macro_entities)
    st["macro entities among the steppers"] += sum(1 for h in handles if h in macro_entities)

    # --- 2. The think queue: CStateManager.10 (CEntryList of CSMEntry) ---
    sms = [i for i, o in objs.items() if isinstance(o, Obj) and o.type == "CStateManager"]
    if not sms:
        return
    sm = objs[sms[0]]
    lst = sm.get("CStateManager.10")
    entries = lst.get("CEntryList.1")
    st["think-queue entries"] += len(entries)
    dist["think-queue entries per world"][len(entries)] += 1
    due_times = []
    for e in entries:
        hnd = e.get("CSMEntry.1")
        h = hnd.oid if isinstance(hnd, Ref) else NULL
        st["CSMEntry.1 refers to an entity of the world (or null)"] += h in set(listed) or h == NULL
        t = e.get("CSMEntry.2")
        due_ticks = t.get("Time.1") if isinstance(t, Obj) else None
        due_times.append(due_ticks)
        dist["due time (seconds)"][due_ticks / 4294967296.0 if due_ticks is not None else None] += 1
        st["CSMEntry.4 is ever present (should never be)"] += e.get("CSMEntry.4", "ABSENT") != "ABSENT"
        for f in SENTINEL_FIELDS:
            st[f"{f} == -1"] += e.get(f) == -1
        sid = e.get("CSMEntry.12")
        dist["CSMEntry.12 (state/function id)"][sid] += 1
        ent_cls = objs[h].type if h != NULL and h in objs else "null/other"
        dist["entity class with a pending think"][ent_cls] += 1
    st["stored CEntryList.1 is sorted ascending by due time"] += due_times == sorted(due_times)
    c = collections.Counter(due_times)
    ties = sum(1 for _t, n in c.items() if n > 1)
    st["worlds with >=1 tie (two entries sharing a due time)"] += ties > 0
    st["tie groups found (over all worlds)"] += ties


def main():
    args = sys.argv[1:]
    install = args[0] if args and os.path.isdir(args[0]) else os.environ.get("SS2_DATA_DIR", "")
    limit = int(args[-1]) if args and args[-1].isdigit() else None
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder (the one containing All_PC_01.gro) or set SS2_DATA_DIR")
    st = collections.Counter()
    dist = collections.defaultdict(collections.Counter)
    for name, d in sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))[:limit]:
        check_world(name, d, st, dist)
    for k, v in sorted(st.items()):
        print(f"{v:9d}  {k}")
    print()
    for k, v in sorted(dist.items()):
        print(k, dict(v.most_common(20)))


if __name__ == "__main__":
    main()
