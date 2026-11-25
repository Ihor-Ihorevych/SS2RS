"""Evidence for docs/re/brain-assignment.md (T01-125): which entities the worlds place, which of them carry a character behavior (`.cb`),
and how a puppet's behavior relates to its parameter file and character name. Data only, read-only on the install
(CLEANROOM.md: this script reads DATA files, never the game's binaries).

It prints
  1. the puppet entity classes that the worlds place (class name, how many, how many with a behavior link, how many without) and how many
     of those class names contain the text `PuppetEntity` (the text the brain choice replaces, brain-assignment.md section 2);
  2. the placed puppets without a behavior, by parameter file;
  3. the behavior links by parameter file: how many distinct `.cb` files each parameter file is paired with, and whether the file stems agree;
  4. the `.cb` files: file stem against `cb_idName`, which stems are referenced by a world, and whether the puppet's `en_idDefaultState`
     is a state of its behavior.

Usage: python -I tools/re/brain_assignment_verify.py [DATA_DIR] [LIMIT]
       (DATA_DIR defaults to $SS2_DATA_DIR; LIMIT = only the LIMIT smallest worlds, default all; the full run takes about 10 minutes)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
import ai_verify  # noqa: E402

NULL = 0xFFFFFFFF


def lineage(parent, name):
    out = []
    while name is not None and name not in out: out.append(name); name = parent.get(name)
    return out


def stem(path):
    return path.replace("\\", "/").rsplit("/", 1)[-1].rsplit(".", 1)[0]


def cb_states(install):
    """{file stem: (cb_idName, [state names])} for the 82 behavior files."""
    out = {}
    for name, d in sorted(merged_files(install, ".cb").items()):
        r, dec, objs, edob = ai_verify.load(d)
        idn = lambda v, r=r: r["idnt"][v] if isinstance(v, int) and 0 <= v < len(r["idnt"]) else v
        cb = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CCharacterBehavior")
        m = ai_verify.members(cb, "CCharacterBehavior")
        states = [objs[x.oid] if isinstance(x, Ref) else x for x in m["cb_acsCharacterStates"]]
        out[stem(name).lower()] = (idn(m["cb_idName"]), [idn(ai_verify.members(s, "CCharacterState")["cs_idState"]) for s in states])
    return out


def seat_counts(install):
    """{parameter file stem (lower case): number of seats} for the puppet parameter files (`SPuppetParams.4`, a container of seat objects)."""
    out = {}
    for name, d in sorted(merged_files(install, ".ep").items()):
        r = parse(d); dec = TreeDec(d, r); objs, _ = dec.objects()
        try:
            root = objs[[x.oid for x in next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CResourceFile").get("CResourceFile.rf_crsShared")][0]]
        except Exception:
            continue
        v = root.get("SPuppetParams.4")
        if v is None: continue
        v = objs[v.oid] if isinstance(v, Ref) and v.oid != NULL else v
        lst = v.get("CScriptObjectContainer.1") if isinstance(v, Obj) else None
        out[stem(name).lower()] = len(lst) if isinstance(lst, list) else 0
    return out


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    limit = int(args[1]) if len(args) > 1 else None
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    cbs = cb_states(install); seats = seat_counts(install)
    worlds = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))
    if limit: worlds = worlds[:limit]
    by_class = collections.Counter(); with_cb = collections.Counter(); no_cb = collections.Counter()
    nocb_by_ep = collections.Counter(); pair = collections.Counter(); ep_cb = collections.defaultdict(set)
    spawn = collections.Counter(); state_in = collections.Counter(); bad_state = collections.Counter(); seat_cb = collections.Counter(); used_cb = collections.Counter(); rec_cb = collections.Counter()
    for wi, (name, d) in enumerate(worlds):
        r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects(); extra, _ = dec.extra_objects(edob)
        T = r["types"]; parent = {t.name: (T[t.parent].name if t.parent in T else None) for t in T.values() if t.members is not None}
        rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}
        idn = lambda v, r=r: r["idnt"][v] if isinstance(v, int) and 0 <= v < len(r["idnt"]) else v
        link = lambda v, suf: ex.get(v.oid) if isinstance(v, Ref) and v.oid in ex and ex[v.oid].lower().endswith(suf) else None
        for o in extra.values():
            if isinstance(o, Obj) and o.type == "SPuppetProperties" or (isinstance(o, Obj) and "SPuppetProperties" in lineage(parent, o.type)):
                rec_cb["records"] += 1
                rec_cb["records with a behavior"] += link(o.get("SPuppetProperties.ep_pcbCharacterBehavior"), ".cb") is not None
        for o in objs.values():
            if not isinstance(o, Obj) or "SPuppetEntity" not in lineage(parent, o.type): continue
            by_class[o.type] += 1
            # the data stores the spawner links of SPuppetEntity under the numbers .30 (en_henSpawner) and .31 (en_henSpawnerWhoSpawnedMe), puppet-classes.md section 4
            s30, s31 = o.get("SPuppetEntity.30"), o.get("SPuppetEntity.31")
            on = lambda v: isinstance(v, Ref) and v.oid != NULL
            spawn[("spawner link (en_henSpawner) " + ("set" if on(s30) else "empty"), "spawned-by link (en_henSpawnerWhoSpawnedMe) " + ("set" if on(s31) else "empty"), "behavior" if link(o.get("SPuppetEntity.en_pcbCharacterBehavior"), ".cb") else "no behavior")] += 1
            cb = link(o.get("SPuppetEntity.en_pcbCharacterBehavior"), ".cb"); ep = link(o.get("SPuppetEntity.en_pppPuppetParams"), ".ep")
            epn = stem(ep) if ep else "(none)"
            ns = seats.get(epn.lower(), -1)
            seat_cb[(o.type, "seats" if ns > 0 else "no seats" if ns == 0 else "no params", "behavior" if cb else "no behavior")] += 1
            if cb is None:
                no_cb[o.type] += 1; nocb_by_ep[(o.type, epn)] += 1; continue
            with_cb[o.type] += 1; cbn = stem(cb).lower(); used_cb[cbn] += 1; ep_cb[epn].add(cbn)
            pair[epn.lower() == cbn] += 1
            ds = idn(o.get("SPuppetEntity.en_idDefaultState"))
            state_in[(ds in cbs[cbn][1]) if cbn in cbs else "cb not found"] += 1
            if cbn in cbs and ds not in cbs[cbn][1]: bad_state[(cbn, ds)] += 1
        print(f"\r{wi + 1}/{len(worlds)} worlds", end="", file=sys.stderr)
    print(f"\nworlds read: {len(worlds)}; behaviors read: {len(cbs)} files")
    print("\n1. placed puppet entity classes (all entities whose class derives from SPuppetEntity)")
    for c in sorted(by_class):
        print(f"   {c:34s} {by_class[c]:5d} placed, {with_cb[c]:5d} with a behavior, {no_cb[c]:4d} without")
    print(f"   total {sum(by_class.values())}; class names containing 'PuppetEntity': {sum(n for c, n in by_class.items() if 'PuppetEntity' in c)}")
    print(f"   property records (SPuppetProperties and subclasses): {rec_cb['records']}, with a behavior link: {rec_cb['records with a behavior']}")
    print("\n2. placed puppets without a behavior, by (entity class, parameter file stem)")
    for (c, e), n in sorted(nocb_by_ep.items(), key=lambda kv: -kv[1]): print(f"   {n:4d}  {c} / {e}")
    print("\n3. behavior links")
    print(f"   puppets with a behavior whose .cb stem equals the .ep stem: {pair[True]}, differs: {pair[False]}")
    multi = {e: sorted(c) for e, c in ep_cb.items() if len(c) > 1}
    print(f"   parameter files paired with 1 behavior file: {sum(1 for c in ep_cb.values() if len(c) == 1)}, with several: {len(multi)}")
    for e, c in sorted(multi.items()): print(f"      {e}: {', '.join(c)}")
    print("\n4. behaviors")
    print(f"   .cb files: {len(cbs)}; referenced by a placed puppet: {len(used_cb)}; never referenced: {sorted(set(cbs) - set(used_cb))}")
    print(f"   file stem differs from cb_idName in {sum(1 for s, (n, _) in cbs.items() if s.lower() != n.lower())} of {len(cbs)} files")
    print(f"   the puppet's en_idDefaultState is a state of its behavior: {dict(state_in)}")
    print("   ... the 'no' cases by (behavior file stem, default state): " + ", ".join(f"{c}/{d} x{n}" for (c, d), n in sorted(bad_state.items(), key=lambda kv: -kv[1])))
    print("\n   placed puppets by their two spawner links and the behavior link: " + "; ".join(f"{k[0]}, {k[1]}, {k[2]}: {n}" for k, n in sorted(spawn.items())))
    print("\n5. seats (entries of SPuppetParams.4 in the parameter file) against the behavior link, by entity class")
    for k, n in sorted(seat_cb.items()): print(f"   {k[0]:30s} {k[1]:9s} {k[2]:12s} {n:5d}")
    print(f"   parameter files with at least one seat: {sorted(e for e, n in seats.items() if n > 0)}")


if __name__ == "__main__":
    main()
