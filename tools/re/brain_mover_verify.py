"""Evidence for docs/re/brain-movement.md (T01-127): the distance bands of the `.cb` files (what the brain's mover choice looks up) and the
tactic and patrol data of the worlds (what the brain's mover choice and patrol functions consume). Data only, read-only on the install
(CLEANROOM.md: this script reads DATA files, never the game's binaries).

It prints
  1. the band lists of the 82 `.cb` files: for every state and medium (land, air, water) the number of bands, whether the distances are strictly
     ascending, the last distance, empty lists, bands without a mover, and the test vectors of the lookup (which band a distance selects);
  2. the velocity adjuster of the bands: the values stored in the five members;
  3. (with worlds) the tactic and patrol data of the placed puppets: default tactic link, the tactic's mover class, marker, next tactic and
     the auto-patrol members; `--cb-only` skips this part.

Usage: python -I tools/re/brain_mover_verify.py [DATA_DIR] [--cb-only] [LIMIT]
       (DATA_DIR defaults to $SS2_DATA_DIR; LIMIT = only the LIMIT smallest worlds; the `.cb` part takes about 1 minute, the worlds about 20)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Blob, Enum, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
import ai_verify  # noqa: E402

NULL = 0xFFFFFFFF
MEDIA = ("cs_dcLand", "cs_dcAir", "cs_dcWater")


def pick(o, objs):
    return objs[o.oid] if isinstance(o, Ref) and o.oid != NULL else (o if isinstance(o, Obj) else None)


def select(bands, d):
    """The lookup of the brain: the first band (list order) whose distance is greater than d; None when d < 0 or no band qualifies."""
    if d < 0: return None
    for b in bands:
        if b[0] > d: return b
    return None


def lineage(parent, name):
    out = []
    while name is not None and name not in out: out.append(name); name = parent.get(name)
    return out


def cb_part(install):
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter); vect = []
    for name, d in sorted(merged_files(install, ".cb").items()):
        r, dec, objs, edob = ai_verify.load(d)
        idn = lambda v, r=r: r["idnt"][v] if isinstance(v, int) and 0 <= v < len(r["idnt"]) else v
        cb = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CCharacterBehavior")
        m = ai_verify.members(cb, "CCharacterBehavior")
        for s in [pick(x, objs) for x in m["cb_acsCharacterStates"]]:
            sm = ai_verify.members(s, "CCharacterState"); st["states"] += 1
            for medium in MEDIA:
                areas = [pick(a, objs) for a in ai_verify.members(sm[medium], "CDipContentAreas")["dca_ccbaCustomBehaviorAreas"]]
                st["band lists"] += 1; dist["bands per list"][len(areas)] += 1
                if not areas: st["empty band lists"] += 1; continue
                rows = []
                for a in areas:
                    am = ai_verify.members(a, "CCustomBehaviorArea")
                    rows.append((am["cba_fDistance"], idn(am["cba_idArea"]), pick(am["cba_pmcMoveCharacter"], objs)))
                    st["bands"] += 1; st["bands without a mover"] += rows[-1][2] is None
                    for k, v in ai_verify.members(pick(am["cba_avaVelocityAdjuster"], objs) or am["cba_avaVelocityAdjuster"], "SAreaVelocityAdjuster").items():
                        dist["velocity adjuster " + k][round(v, 4) if isinstance(v, float) else v] += 1
                ds = [x[0] for x in rows]
                st["lists strictly ascending by distance"] += all(a < b for a, b in zip(ds, ds[1:]))
                st["lists whose last distance is 1e8 or more"] += ds[-1] >= 1e8
                dist["last distance"][ds[-1]] += 1
                dist["first distance"][ds[0]] += 1
                if name.endswith("kleer.cb") and medium == "cs_dcLand" and not vect:
                    vect = [(name.rsplit("/", 1)[-1], idn(sm["cs_idState"]), [(x[0], x[1]) for x in rows])]
    print(f"\n1. band lists of the {len(merged_files(install, '.cb'))} `.cb` files")
    for k, v in st.items(): print(f"   {k}: {v}")
    print("   bands per list:", dict(sorted(dist["bands per list"].items())))
    print("   first distance of a list (value: lists):", dict(sorted(dist["first distance"].items())))
    print("   last distance of a list (value: lists):", dict(sorted(dist["last distance"].items())))
    print("\n2. velocity adjuster values (member: value: bands)")
    for k in sorted(x for x in dist if x.startswith("velocity adjuster")): print(f"   {k[18:]}: {dict(sorted(dist[k].items()))}")
    if vect:
        f, s, rows = vect[0]
        print(f"\n   lookup example, {f}, state {s}, land bands (distance, id): {rows}")
        for q in (-1, 0, rows[0][0] - 0.01, rows[0][0], rows[0][0] + 0.01, rows[-1][0] - 1, 1e9):
            b = select(rows, q); print(f"      distance {q:g} -> {b[1] if b else 'no band (default mover)'}")


def world_part(install, limit):
    worlds = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))
    if limit: worlds = worlds[:limit]
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter)
    for wi, (name, d) in enumerate(worlds):
        r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects()
        T = r["types"]; parent = {t.name: (T[t.parent].name if t.parent in T else None) for t in T.values() if t.members is not None}
        tac = {i: o for i, o in objs.items() if isinstance(o, Obj) and o.type == "STacticEntity"}
        st["tactic entities"] += len(tac); st["worlds"] += 1
        nxt_targets = collections.Counter()
        for i, t in tac.items():
            mv = pick(t.get("STacticEntity.en_hmcCharacterMover"), objs)
            dist["tactic mover class"][mv.type if mv else "(none)"] += 1
            st["tactics with a marker"] += isinstance(t.get("STacticEntity.en_henTacticMarker"), Ref) and t.get("STacticEntity.en_henTacticMarker").oid != NULL
            n = t.get("STacticEntity.en_henNextTactic")
            if isinstance(n, Ref) and n.oid != NULL: nxt_targets[n.oid] += 1; st["tactics with a next tactic"] += 1
        st["tactics that are the next tactic of another"] += len(nxt_targets)
        st["tactics with a next tactic that is itself"] += sum(1 for i, t in tac.items() if isinstance(t.get("STacticEntity.en_henNextTactic"), Ref) and t.get("STacticEntity.en_henNextTactic").oid == i)
        used = collections.Counter()
        for i, o in objs.items():
            if not isinstance(o, Obj) or "SPuppetEntity" not in lineage(parent, o.type): continue
            st["placed puppets"] += 1
            t = o.get("SPuppetEntity.en_henDefaultTacticEntity")
            has_t = isinstance(t, Ref) and t.oid != NULL
            auto = o.get("SPuppetEntity.1102"); aid = o.get("SPuppetEntity.1101")
            st["puppets with a default tactic"] += has_t
            st["puppets with the auto-patrol flag"] += bool(auto)
            st["puppets with an auto-patrol marker id other than 0xFFFFFFFF"] += aid not in (None, NULL)
            st["puppets with both a default tactic and the auto-patrol flag"] += has_t and bool(auto)
            if has_t:
                used[t.oid] += 1
                tt = tac.get(t.oid)
                if tt is not None:
                    mk = tt.get("STacticEntity.en_henTacticMarker")
                    st["puppets whose default tactic has no marker"] += not (isinstance(mk, Ref) and mk.oid != NULL)
                    st["puppets whose default tactic has no marker and no auto-patrol flag"] += not (isinstance(mk, Ref) and mk.oid != NULL) and not auto
        st["tactics used as default tactic by two or more puppets"] += sum(1 for v in used.values() if v > 1)
        print(f"\r{wi + 1}/{len(worlds)} worlds", end="", file=sys.stderr)
    print(f"\n3. tactics and patrol data of {st['worlds']} worlds")
    for k, v in st.items(): print(f"   {k}: {v}")
    print("   tactic mover class:", dict(dist["tactic mover class"].most_common()))


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    cb_part(install)
    if "--cb-only" not in sys.argv: world_part(install, int(args[1]) if len(args) > 1 else None)


if __name__ == "__main__":
    main()
