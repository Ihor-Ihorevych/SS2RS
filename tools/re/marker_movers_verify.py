"""Evidence for docs/re/behavior-marker-movers.md and docs/re/behavior-navigation-mover.md (T01-155): the data side of the three marker
movers (`CMCMarkersMovingOrdered`, `CMCMarkersMovingClosestToFoe`, `CMCMarkersMovingRandom`) and of the path markers they walk.
Data only, read-only on the install (CLEANROOM.md: this script reads DATA files, never the game's binaries).

It prints, over every `.wld` of the install,
  1. the marker position the movers use: the editor primitive that every path marker owns (the movers read its placement) sits **0.25 m along the
     marker's own up axis** above the placement of the marker's record in all but a few markers (the exceptions are listed by world);
  2. the link graph: next and back links, the length of every next-link segment (3-D distance), segments shorter than 1.0 m (the path-length quirk of
     the closest-to-foe mover), the "single path" of a marker (next chain, then back chain) and its size, open chains against closed loops;
  3. the ordered movers: initial direction, direction-change period, the wait members, the behaviour on the last marker, each against whether the
     tactic's marker lies on an open chain or a closed loop and where on the chain it lies; the stored run-time members;
  4. the random and closest-to-foe movers: wait members, flags, period, size of the marker's single path;
  5. test material for the specification: the probability table of the random mover (index 0 is re-mapped to index 1).

Usage: python -I tools/re/marker_movers_verify.py [DATA_DIR] [LIMIT]
       (DATA_DIR defaults to $SS2_DATA_DIR; LIMIT = only the LIMIT smallest worlds; about 15 minutes for all 59 worlds)
"""
import collections, math, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Enum, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
from wld_root_verify import ids  # noqa: E402

NULL = 0xFFFFFFFF


def num(v):
    return v.v if isinstance(v, Enum) else v


def vec(o):
    return tuple(o.get(f"Vector3f.{i}") for i in (1, 2, 3))


def up_axis(q):
    """The rotated local Y axis of a quaternion (x, y, z, w): the column (R01, R11, R21) of its rotation matrix."""
    x, y, z, w = (q.get(f"Quaternion4f.{i}") for i in (1, 2, 3, 4))
    return (2 * (x * y - w * z), 1 - 2 * (x * x + z * z), 2 * (y * z + w * x))


def single_path(start, nxt, prv):
    """`CPathMarkerEntity::GetSinglePath`: the start, then the next chain (stops at null or a marker already collected),
    then the back chain of the start (the same stop rule)."""
    out, seen = [], set()
    cur = start
    while cur != NULL and cur in nxt and cur not in seen:
        seen.add(cur); out.append(cur); cur = nxt[cur]
    cur = prv.get(start, NULL)
    while cur != NULL and cur in nxt and cur not in seen:
        seen.add(cur); out.append(cur); cur = prv[cur]
    return out


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    limit = int(args[1]) if len(args) > 1 else None
    worlds = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))
    if limit: worlds = worlds[:limit]
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter)
    maxdev = 0.0; seglens = []
    for wi, (name, d) in enumerate(worlds):
        r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects()
        try: extra, _ = dec.extra_objects(edob)
        except Exception: extra = {}
        allo = {**objs, **extra}
        w = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CWorld"); ents = ids(w.get("CWorld.1"))
        recs = {extra[p].get("CEntityProperties.2").oid: extra[p] for p in ids(w.get("CWorld.5")) if p in extra}
        st["worlds"] += 1
        pm = {i: objs[i] for i in ents if objs[i].type == "SPathMarkerEntity"}
        nxt = {i: o.get("SPathMarkerEntity.2").oid for i, o in pm.items()}
        prv = {i: o.get("SPathMarkerEntity.4").oid for i, o in pm.items()}
        pos = {}
        for i, o in pm.items():
            st["path markers"] += 1
            rend = allo.get(o.get("SPathMarkerEntity.1").oid); rec = recs.get(i)
            if rend is None or rec is None: st["markers without primitive or record"] += 1; continue
            q = rend.get("CAspect.7"); p = vec(q.get("QuatVect.2")); a4 = vec(rend.get("CAspect.4").get("QuatVect.2"))
            rq = rec.get("CEntityProperties.3"); rp = vec(rq.get("QuatVect.2")); ax = up_axis(rq.get("QuatVect.1"))
            dev = max(abs(p[k] - (rp[k] + 0.25 * ax[k])) for k in range(3))
            maxdev = max(maxdev, dev); pos[i] = p
            st["markers whose primitive sits 0.25 m along the up axis above the record position (tolerance 0.001)"] += dev < 1e-3
            st["markers whose two stored primitive placements agree"] += p == a4
            if dev >= 1e-3: dist["worlds with markers whose primitive is elsewhere (record position is not the marker position): markers"][name.rsplit("/", 1)[-1]] += 1
        for i in pm:
            n = nxt[i]
            if n != NULL and n in pos and i in pos:
                L = math.dist(pos[i], pos[n]); seglens.append(L)
                st["next-link segments"] += 1; st["... shorter than 1.0 m"] += L < 1.0; st["... shorter than 0.01 m"] += L < 0.01
        # single paths of every marker
        for i in pm:
            sp = single_path(i, nxt, prv)
            dist["size of the single path of a marker"][min(len(sp), 40)] += 1
        # tactics and movers
        tac = {i: o for i, o in objs.items() if isinstance(o, Obj) and o.type == "STacticEntity"}
        for ti, t in tac.items():
            mvr = t.get("STacticEntity.en_hmcCharacterMover"); mk = t.get("STacticEntity.en_henTacticMarker")
            mv = objs.get(mvr.oid) if isinstance(mvr, Ref) else None
            if mv is None or mv.type not in ("CMCMarkersMovingOrdered", "CMCMarkersMovingClosestToFoe", "CMCMarkersMovingRandom"): continue
            cls = mv.type; short = cls.replace("CMCMarkersMoving", "")
            has = isinstance(mk, Ref) and mk.oid != NULL and mk.oid in pm
            st[f"tactics with the {short} mover"] += 1
            st[f"{short}: tactic has a marker"] += has
            g = lambda m: mv.get(f"{cls}.{m}")
            if cls == "CMCMarkersMovingOrdered":
                init = num(g("mm_imdInitMarkersDir")); per = g("mm_fChangeDirectionPeriod"); wmin = g("mm_fWaitOnMarkerMin"); wmax = g("mm_fWaitOnMarkerMax")
                bolm = num(g("mm_bolmBehaviorOnLastMarker"))
                dist["ordered: initial direction"][init] += 1; dist["ordered: behaviour on last marker"][bolm] += 1
                dist["ordered: change period"][per] += 1; dist["ordered: wait min"][wmin] += 1; dist["ordered: wait max"][wmax] += 1
                st["ordered: wait min above wait max"] += wmin > wmax; st["ordered: wait min below 0"] += wmin < 0; st["ordered: wait max below 0"] += wmax < 0
                st["ordered: wait max <= 0 (never waits)"] += wmax <= 0
                st["ordered: wait max > 0 and wait min <= 0"] += wmax > 0 >= wmin
                st["ordered: change period above 0"] += per > 0
                dist["ordered: run-time members (toward next, marker reached, last reached, wait)"][(g("mm_bMovingTowardNextMarker"), g("mm_bMarkerReached"), g("mm_bLastMarkerReached"), g("mm_fWaitOnMarker"))] += 1
                dist["ordered: mover speed multiplier"][round(mv.get("SMoveCharacter.mc_fSpeedMultiplier"), 3)] += 1
                if has:
                    m = mk.oid; sp = single_path(m, nxt, prv)
                    # loop: following next from the marker comes back to it
                    cur = nxt[m]; seen = {m}; loop = False
                    while cur != NULL and cur in pm:
                        if cur == m: loop = True; break
                        if cur in seen: break
                        seen.add(cur); cur = nxt[cur]
                    fwd_end = nxt[m] == NULL; bwd_end = prv[m] == NULL or prv[m] not in pm
                    kind = "closed loop" if loop else ("open chain, marker is the last (no next)" if fwd_end else ("open chain, marker is the first (no back)" if bwd_end else "open chain, marker in the middle"))
                    dist["ordered: marker lies on"][kind] += 1
                    dist["ordered: behaviour on last marker x marker lies on"][(bolm, "loop" if loop else "open")] += 1
                    dist["ordered: initial direction x marker lies on"][(init, kind)] += 1
                    dist["ordered: size of the single path of the tactic marker"][min(len(sp), 40)] += 1
                    # markers of the forward chain that have no next (the last markers) in that single path
                else:
                    dist["ordered: behaviour on last marker, tactic without a marker"][bolm] += 1
            elif cls == "CMCMarkersMovingClosestToFoe":
                los = g("mmctf_bTestLOS"); follow = g("mmctf_bMustFollowPath"); per = g("mmctf_fBestMarkerCheckPeriod")
                dist["closest: (test LOS, must follow path)"][(los, follow)] += 1; dist["closest: best marker check period"][per] += 1
                dist["closest: run-time members (best marker set, last reached set, moving forward)"][
                    (isinstance(g("mmctf_henCurrentBestMarker"), Ref) and g("mmctf_henCurrentBestMarker").oid != NULL,
                     isinstance(g("mmctf_henLastReachedMarker"), Ref) and g("mmctf_henLastReachedMarker").oid != NULL, g("mmctf_bMovingForwardAlongPath"))] += 1
                if has:
                    sp = single_path(mk.oid, nxt, prv); m = mk.oid
                    cur = nxt[m]; seen = {m}; loop = False
                    while cur != NULL and cur in pm:
                        if cur == m: loop = True; break
                        if cur in seen: break
                        seen.add(cur); cur = nxt[cur]
                    dist["closest: marker lies on"]["closed loop" if loop else "open chain"] += 1
                    dist["closest: size of the single path of the tactic marker"][min(len(sp), 40)] += 1
            else:
                wmin = g("mmr_fWaitOnMarkerMin"); wmax = g("mmr_fWaitOnMarkerMax")
                dist["random: wait min"][wmin] += 1; dist["random: wait max"][wmax] += 1
                st["random: wait min above wait max"] += wmin > wmax
                dist["random: run-time members (wait, marker reached)"][(g("mmr_fWaitOnMarker"), g("mmr_bMarkerReached"))] += 1
                if has:
                    sp = single_path(mk.oid, nxt, prv)
                    dist["random: size of the single path of the tactic marker"][min(len(sp), 40)] += 1
        print(f"\r{wi + 1}/{len(worlds)} worlds", end="", file=sys.stderr)
    print()
    print("1. marker position: largest deviation of the primitive from (record position + 0.25 x up axis): %.6f" % maxdev)
    print("2./3./4. counts")
    for k, v in st.items():
        if v or "markers" in k: print(f"   {k}: {v}")
    if seglens:
        sl = sorted(seglens); print(f"   next-link segment length: min {sl[0]:.3f}, median {sl[len(sl)//2]:.3f}, max {sl[-1]:.3f}")
    for k, v in dist.items():
        items = sorted(v.items(), key=lambda kv: (-kv[1], str(kv[0])))
        print(f"   {k}: {dict(items) if len(items) <= 14 else dict(items[:14])}{' ...' if len(items) > 14 else ''}")
    print("5. random mover pick (single path of n markers, index 0 is the current marker): probability of index k")
    for n in (1, 2, 3, 4, 5):
        pr = [0.0] * n
        for idx in range(n):
            pr[(idx + 1) % n if idx == 0 else idx] += 1.0 / n
        print(f"   n = {n}: {[round(p, 4) for p in pr]}")


if __name__ == "__main__":
    main()
