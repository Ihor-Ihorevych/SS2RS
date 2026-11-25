"""Puppet attacks: shooting processes, blasts, melee, run-over, leap and throw descriptions of the `.ep` files (T01-143, read-only).

Reads every `.ep` of the install (all patches) and prints the tables of `docs/re/puppet-attacks.md`:
counts of each attack record, the value tables of their members, which graphs are set, the pairs of
prediction ratios, the pitch graphs and the throw descriptions. It also evaluates the formulas of the
spec on the stored values (damage direction of a melee blast, the chain timings, the free-fall launch)
so the test vectors of the page can be re-checked.

Usage: python -I tools/re/puppet_attacks_verify.py [DATA_DIR]      (or set SS2_DATA_DIR)
Standard library only; read-only on the install; prints numbers only.
"""
import collections, math, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, merged_files, parse, TreeDec  # noqa: E402

NULL = 0xFFFFFFFF


def load(install):
    out = {}
    for name, d in sorted(merged_files(install, ".ep").items()):
        r = parse(d)
        objs, _ = TreeDec(d, r).objects()
        out[name] = (r, objs)
    return out


def projectile_flags(eps, r, oid):
    """Resolve an external object reference of an `.ep` to the projectile file and return (file name, bp_ulFlags)."""
    for ob, fi, _, _ in r["exob"]:
        if ob == oid:
            path = r["rfil"][fi][2].lower()
            if path in eps:
                r2, o2 = eps[path]
                for o in o2.values():
                    if isinstance(o, Obj) and o.type != "CResourceFile":
                        f = val(o, "SBaseProjectileParams.100")
                        return path.rsplit("/", 1)[-1], (f.v if hasattr(f, "v") else f)
            return path.rsplit("/", 1)[-1], None
    return None, None


def walk(o, objs, seen, out):
    """All class instances reachable from `o` (inline or through references of the same file)."""
    if id(o) in seen:
        return
    seen.add(id(o))
    out.append(o)
    for _, v in o:
        for x in (v if isinstance(v, list) else [v]):
            if isinstance(x, Obj):
                walk(x, objs, seen, out)
            elif isinstance(x, Ref) and x.oid != NULL and isinstance(objs.get(x.oid), Obj):
                walk(objs[x.oid], objs, seen, out)


def val(o, suffix):
    for k, v in o:
        if k.endswith(suffix):
            return v
    return None


def ref(o, suffix, objs):
    v = val(o, suffix)
    if isinstance(v, Ref) and v.oid != NULL:
        return objs.get(v.oid)
    return None


def graph_eval(g, x):
    """Clamped piecewise-linear evaluation of CValueGraph / CLinearGraph / CStepUpGraph / CStepDownGraph (scales 1)."""
    if g is None:
        return None
    pts = None
    for k, v in g:
        if isinstance(v, list) and v and k.split(".")[-1] in ("1", "2"):
            pts = v
    if pts is None:
        return None
    if g.type == "CValueGraph":
        pts = [pts] if isinstance(pts[0], tuple) else pts
    xy = []
    for p in pts:
        c = val(p, "CControlPoint.3") if isinstance(p, Obj) else None
        if c is not None:
            xy.append((val(c, "Vector2f.1"), val(c, "Vector2f.2")))
    if not xy:
        return None
    if len(xy) == 1 or x <= xy[0][0]:
        return xy[0][1]
    if x >= xy[-1][0]:
        return xy[-1][1]
    for (x0, y0), (x1, y1) in zip(xy, xy[1:]):
        if x <= x1:
            return y0 if x1 == x0 else y0 + (x - x0) * (y1 - y0) / (x1 - x0)
    return xy[-1][1]


def graph_points(g):
    for k, v in g:
        if isinstance(v, list) and v and isinstance(v[0], Obj):
            return [(round(val(val(p, "CControlPoint.3"), "Vector2f.1"), 3), round(val(val(p, "CControlPoint.3"), "Vector2f.2"), 3)) for p in v]
        if isinstance(v, Obj):
            c = val(v, "CControlPoint.3")
            if c is not None:
                return [(round(val(c, "Vector2f.1"), 3), round(val(c, "Vector2f.2"), 3))]
    return []


def hist(vals, n=8):
    c = collections.Counter(vals)
    return ", ".join(f"{k} x{v}" for k, v in c.most_common(n)) + (f" ... ({len(c)} distinct)" if len(c) > n else "")


def free_fall(O, pitch, T0, vT, g=30.0, vmin=0.0, vmax=3e38):
    """The engine's fixed-pitch free-fall launch solver as described in the spec (gravity (0,-g,0))."""
    T = list(T0)
    tprev = 0.0
    for _ in range(10):
        d = [T[i] - O[i] for i in range(3)]
        dlast = d
        h = d[1]
        R = math.hypot(d[0], d[2])
        if math.atan2(h, R) > pitch:
            return None
        t = math.sqrt((2.0 / g) * (R * math.tan(pitch) - h))
        T = [T0[i] + vT[i] * t for i in range(3)]
        done = abs(t - tprev) <= 0.01
        tprev = t
        if done:
            break
    d = dlast          # the offset of the last iteration, taken before the target was moved by its own velocity
    gv = (0.0, -g, 0.0)
    v = [(d[i] - 0.5 * gv[i] * t * t) / t for i in range(3)]
    return v, t, T


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder or set SS2_DATA_DIR")
    eps = load(install)
    byt = collections.defaultdict(list)       # type -> [(file, obj, objs, idnt)]
    files_with = collections.Counter()
    for name, (r, objs) in eps.items():
        seen = set()
        allo = []
        for o in objs.values():
            if isinstance(o, Obj) and o.type != "CResourceFile":
                walk(o, objs, seen, allo)
        kinds = set()
        for o in allo:
            byt[o.type].append((name, o, objs, r["idnt"]))
            kinds.add(o.type)
        for k in kinds:
            files_with[k] += 1
    print(f"{len(eps)} .ep files")
    print("\n== 1. Records (instances / files)")
    for t in ("CShootingProcess", "CShootAnimation", "CSingleBlast", "CMeleeAttack", "CMeleeBlast", "CRunOverFoeAttack", "CLeapAttack",
              "CThrowDescription", "CMountedTool", "CValueGraph", "CLinearGraph", "CStepUpGraph", "CStepDownGraph"):
        print(f"  {t}: {len(byt[t])} instances in {files_with[t]} files")

    print("\n== 2. CShootingProcess")
    sp = [o for _, o, _, _ in byt["CShootingProcess"]]
    num = lambda s: [val(o, s) for o in sp]
    print("  sp_fStartAttackDistance:", hist(num("sp_fStartAttackDistance")), "| above 0 in", sum(1 for v in num("sp_fStartAttackDistance") if v > 0))
    print("  sp_fMaxWaitStartAttackPeriod:", hist(num("sp_fMaxWaitStartAttackPeriod")))
    print("  sp_bAttackTargetIfCantBeSeen:", hist(num("sp_bAttackTargetIfCantBeSeen")))
    print("  sp_fStopAttackAngle:", hist(num("sp_fStopAttackAngle")))
    print("  sp_pfPlayerFire:", hist([str(v) for v in num("sp_pfPlayerFire")]))
    print("  sp_pmcCharacterMover set:", sum(1 for o in sp if isinstance(val(o, "sp_pmcCharacterMover"), Ref) and val(o, "sp_pmcCharacterMover").oid != NULL), "of", len(sp))
    print("  sp_fWaitBeforeShooting:", hist(num("sp_fWaitBeforeShooting")), "| above 0 in", sum(1 for v in num("sp_fWaitBeforeShooting") if v > 0))
    print("  sp_fAimTime:", hist(num("sp_fAimTime")), "| above 0 in", sum(1 for v in num("sp_fAimTime") if v > 0))
    print("  sp_fWaitAfterShooting:", hist(num("sp_fWaitAfterShooting")), "| above 0 in", sum(1 for v in num("sp_fWaitAfterShooting") if v > 0))
    for m in ("sp_bMustStopToShoot", "sp_bSkipLookMatchPause", "sp_bAdjustPicthToThrow", "sp_bUnprepareAsInvertedPrepare", "sp_bLerpInAimAnimation", "sp_bLerpOutAimAnimation"):
        print(f"  {m}:", hist(num(m)))
    # identifiers: 'empty' check through the file's identifier table
    def ident_empty(name_o, member):
        for fname, o, objs, idnt in byt["CShootingProcess"]:
            pass
    empt = collections.Counter()
    for fname, o, objs, idnt in byt["CShootingProcess"]:
        for m in ("sp_idPrepareAnimation", "sp_idAimAnimation", "sp_idConsecutiveShootReloadAnimation", "sp_idReloadAnimation"):
            v = val(o, m)
            s = idnt[v] if isinstance(v, int) and 0 <= v < len(idnt) else "?"
            empt[(m, "empty" if s == "" else "set")] += 1
    for k in sorted(empt):
        print("  ", k[0], k[1], empt[k])
    # aim time with no aim animation (the aim time is then ignored)
    n = 0
    for fname, o, objs, idnt in byt["CShootingProcess"]:
        a = val(o, "sp_idAimAnimation")
        if val(o, "sp_fAimTime") > 0 and idnt[a] == "":
            n += 1
    print("  processes with sp_fAimTime > 0 and no aim animation (aim time ignored):", n)
    n = sum(1 for fname, o, objs, idnt in byt["CShootingProcess"] if val(o, "sp_bUnprepareAsInvertedPrepare") and idnt[val(o, "sp_idPrepareAnimation")] == "")
    print("  processes with sp_bUnprepareAsInvertedPrepare set but no prepare animation:", n)
    print("  blasts per process:", hist([len(val(o, "sp_csbBlasts") or []) for o in sp]))
    print("  shoot animations per process:", hist([len(val(o, "sp_asaShootAnimations") or []) for o in sp]))

    print("\n== 3. CShootAnimation")
    sa = [o for _, o, _, _ in byt["CShootAnimation"]]
    for m in ("sa_bTerm", "sa_bLerpShootAnimation", "sa_bLoopShootAnimation", "sa_bScaleAnimationSpeed"):
        print(f"  {m}:", hist([val(o, m) for o in sa]))
    print("  sa_fShootPeriod:", hist([val(o, "sa_fShootPeriod") for o in sa]), "| above 0 in", sum(1 for o in sa if val(o, "sa_fShootPeriod") > 0))
    print("  looping with period <= 0:", sum(1 for o in sa if val(o, "sa_bLoopShootAnimation") and val(o, "sa_fShootPeriod") <= 0),
          "| scale-speed with period <= 0:", sum(1 for o in sa if val(o, "sa_bScaleAnimationSpeed") and val(o, "sa_fShootPeriod") <= 0))

    print("\n== 4. CSingleBlast (the shot)")
    bl = [(f, o, objs) for f, o, objs, _ in byt["CSingleBlast"]]
    fl = collections.Counter()
    fall_rows = []
    for f, o, objs, idnt in byt["CSingleBlast"]:
        pr = val(o, "sb_pgppProjectileParams")
        if isinstance(pr, Ref) and pr.oid != NULL:
            pf, flg = projectile_flags(eps, eps[f][0], pr.oid)
            has_graph = ref(o, "sb_pgrPitchOverDistance", objs) is not None
            fl[(flg, "pitch graph" if has_graph else "no pitch graph")] += 1
            if flg is not None and flg & 1:
                fall_rows.append((f.split("databases/")[-1], pf, val(o, "sb_fMinThrowingVelocity"), val(o, "sb_fMaxThrowingVelocity")))
    print("  blast projectile flags (bp_ulFlags) x pitch graph:", dict(fl))
    print("  blasts whose projectile has PF_FALLING (bit 1):", len(fall_rows), "| of them with min throwing velocity > 0:", sum(1 for r_ in fall_rows if r_[2] > 0), "| with max < 1000:", sum(1 for r_ in fall_rows if r_[3] < 1000))
    for r_ in fall_rows[:12]:
        print("    ", r_)
    print("  blasts with min throwing velocity > 0 or max < 1000 (any projectile):", sum(1 for _, o, _ in [(f, o, objs) for f, o, objs, _ in byt["CSingleBlast"]] if val(o, "sb_fMinThrowingVelocity") > 0 or val(o, "sb_fMaxThrowingVelocity") < 999))
    bo = [o for _, o, _ in bl]
    def bf(o, s):
        b = val(o, "sb_bfcBulletFireing")
        return val(b, s)
    print("  projectile set:", sum(1 for _, o, objs in bl if ref(o, "sb_pgppProjectileParams", objs) is not None or (isinstance(val(o, "sb_pgppProjectileParams"), Ref) and val(o, "sb_pgppProjectileParams").oid != NULL)), "of", len(bl))
    print("  bf_ctBullets:", hist([bf(o, "bf_ctBullets") for o in bo]))
    both = sum(1 for _, o, objs in bl if bf(o, "bf_ctBullets") > 0 and isinstance(val(o, "sb_pgppProjectileParams"), Ref) and val(o, "sb_pgppProjectileParams").oid != NULL)
    neither = sum(1 for _, o, objs in bl if bf(o, "bf_ctBullets") == 0 and not (isinstance(val(o, "sb_pgppProjectileParams"), Ref) and val(o, "sb_pgppProjectileParams").oid != NULL))
    print("  blasts with bullets AND a projectile:", both, "| with neither:", neither)
    print("  bf_btBulletType (all blasts):", hist([str(bf(o, "bf_btBulletType")) for o in bo]))
    print("  bf_btBulletType (bullets > 0):", hist([str(bf(o, "bf_btBulletType")) for o in bo if bf(o, "bf_ctBullets") > 0]))
    print("  bf_slBulletDamage (bullets > 0):", hist([bf(o, "bf_slBulletDamage") for o in bo if bf(o, "bf_ctBullets") > 0]))
    print("  bf_fBulletVelocity (bullets > 0):", hist([bf(o, "bf_fBulletVelocity") for o in bo if bf(o, "bf_ctBullets") > 0]))
    print("  bf_fBulletReach (bullets > 0):", hist([bf(o, "bf_fBulletReach") for o in bo if bf(o, "bf_ctBullets") > 0]))
    print("  physical bullets (type 0) with velocity -1 (global default):", sum(1 for o in bo if bf(o, "bf_ctBullets") > 0 and bf(o, "bf_btBulletType") == 0 and bf(o, "bf_fBulletVelocity") <= 0))
    for m in ("sb_fBarrelShootoutDeclineH", "sb_fBarrelShootoutDeclineP", "sb_fScatteringDistance", "sb_fMinScatteringRange", "sb_fMaxScatteringRange",
              "sb_fMinPredictionRatio", "sb_fMaxPredictionRatio", "sb_fMinThrowPitch", "sb_fMaxThrowPitch", "sb_fMinThrowingVelocity", "sb_fMaxThrowingVelocity"):
        print(f"  {m}:", hist([round(val(o, m), 4) for o in bo]))
    print("  (min, max) prediction ratio pairs:", hist([(round(val(o, "sb_fMinPredictionRatio"), 3), round(val(o, "sb_fMaxPredictionRatio"), 3)) for o in bo], 12))
    print("  prediction min == max in", sum(1 for o in bo if val(o, "sb_fMinPredictionRatio") == val(o, "sb_fMaxPredictionRatio")), "of", len(bo))
    for m in ("sb_pgrPitchOverDistance", "sb_pgrScateringOverConsecutiveShoots", "sb_pgrPredictionOverConsecutiveShoots", "sb_pgrUDMissOverConsecutiveShoots", "sb_pgrLRMissOverConsecutiveShoots"):
        kinds = collections.Counter()
        for _, o, objs in bl:
            g = ref(o, m, objs)
            kinds[g.type if g is not None else "null"] += 1
        print(f"  {m}:", dict(kinds))
    print("  pitch graphs of blasts with a projectile (control points):")
    pg = collections.Counter()
    for _, o, objs in bl:
        g = ref(o, "sb_pgrPitchOverDistance", objs)
        if g is not None:
            pg[(g.type, tuple(graph_points(g)))] += 1
    for k, v in pg.most_common(12):
        print(f"    x{v} {k[0]} {list(k[1])}")
    sc = collections.Counter()
    for _, o, objs in bl:
        for m in ("sb_pgrPitchOverDistance", "sb_pgrScateringOverConsecutiveShoots", "sb_pgrPredictionOverConsecutiveShoots", "sb_pgrUDMissOverConsecutiveShoots"):
            g = ref(o, m, objs)
            if g is not None:
                sc[(m.split("_")[1], val(g, "CGraph.7"), val(g, "CGraph.8"))] += 1
    print("  graph scale members (CGraph.7, CGraph.8) of the set graphs:", dict(sc))

    print("\n== 5. Melee attacks and blasts")
    ma = [o for _, o, _, _ in byt["CMeleeAttack"]]
    print("  CMeleeAttack:", len(ma), "| ma_pfPlayerFire:", hist([str(val(o, "ma_pfPlayerFire")) for o in ma]), "| blasts per attack:", hist([len(val(o, "sp_cmbMeleeBlasts") or []) for o in ma]))
    mb = [o for _, o, _, _ in byt["CMeleeBlast"]]
    for m in ("mb_fDamageRadius", "mb_slDamage", "mb_slEnvironmentDamageOverride", "mb_fLaunchHeading", "mb_fLaunchPitch", "mb_fFoeKickImpulse", "mb_bDamageChainStart", "mb_dtDamageType"):
        print(f"  {m}:", hist([str(v) if not isinstance(v, float) else round(v, 3) for v in (val(o, m) for o in mb)], 12))
    print("  melee blasts that start a damage chain:", sum(1 for o in mb if val(o, "mb_bDamageChainStart")), "of", len(mb))
    multi = []
    for f, o, objs, _ in byt["CMeleeAttack"]:
        lst = val(o, "sp_cmbMeleeBlasts") or []
        if len(lst) > 1:
            multi.append([val(objs[b.oid] if isinstance(b, Ref) else b, "mb_bDamageChainStart") for b in lst])
    print("  chain-start flags of the blasts of the", len(multi), "melee attacks with several blasts:", multi)
    zero = [(f.split("databases/")[-1]) for f, o, objs, _ in byt["CMeleeBlast"] if not val(o, "mb_bDamageChainStart")]
    print("  files with a melee blast whose chain-start flag is 0:", zero)
    ro = [o for _, o, _, _ in byt["CRunOverFoeAttack"]]
    print("  CRunOverFoeAttack:", len(ro), "| melee blasts per attack:", hist([len(val(o, "rof_cmbMeleeBlasts") or []) for o in ro]), "| rof_pfPlayerFire:", hist([str(val(o, "rof_pfPlayerFire")) for o in ro]))
    la = [o for _, o, _, _ in byt["CLeapAttack"]]
    print("  CLeapAttack:", len(la), "| la_pfPlayerFire:", hist([str(val(o, "la_pfPlayerFire")) for o in la]))

    print("\n== 6. Throw descriptions (tool launch)")
    td = [(f, o, objs) for f, o, objs, _ in byt["CThrowDescription"]]
    print("  descriptions:", len(td), "in", len({f for f, _, _ in td}), "files")
    print("  td_fMinThrowingVelocity:", hist([val(o, "td_fMinThrowingVelocity") for _, o, _ in td]), "| td_fMaxThrowingVelocity:", hist([round(val(o, "td_fMaxThrowingVelocity"), -30) if val(o, "td_fMaxThrowingVelocity") > 1e30 else val(o, "td_fMaxThrowingVelocity") for _, o, _ in td]))
    kinds = collections.Counter()
    for _, o, objs in td:
        kinds[tuple(ref(o, m, objs).type if ref(o, m, objs) is not None else "null" for m in ("td_pgrPitchOverDist", "td_pgrUnderhitOverDist", "td_pgrPredictionOverDist"))] += 1
    print("  graph kinds (pitch, underhit, prediction):", dict(kinds))
    byname = collections.Counter()
    for f, o, objs, idnt in byt["CThrowDescription"]:
        row = []
        for m in ("td_pgrPitchOverDist", "td_pgrUnderhitOverDist", "td_pgrPredictionOverDist"):
            g = ref(o, m, objs)
            row.append((g.type[1:-5], tuple(graph_points(g))))
        mx = val(o, "td_fMaxThrowingVelocity")
        byname[(idnt[val(o, "td_idName")], tuple(row), val(o, "td_fMinThrowingVelocity"), "3e38" if mx > 1e30 else mx)] += 1
    for k, v in sorted(byname.items(), key=lambda kv: -kv[1]):
        print(f"  x{v} name {k[0]!r} pitch {k[1][0]} underhit {k[1][1]} prediction {k[1][2]} min {k[2]} max {k[3]}")
    per_file = collections.Counter(ff for ff, _, _, _ in byt["CThrowDescription"])
    print("  descriptions per file:", hist(list(per_file.values())))

    print("\n== 7. Formula checks (test vectors of the page)")
    # melee launch direction: heading H, pitch P degrees -> local direction (-cos P sin H, sin P, -cos P cos H)
    def melee_dir(H, P):
        h, p = math.radians(H), math.radians(P)
        return (-math.cos(p) * math.sin(h), math.sin(p), -math.cos(p) * math.cos(h))
    for H, P in ((0, 0), (0, 30), (30, 15), (-20, 0), (90, 0)):
        d = melee_dir(H, P)
        print(f"  melee launch heading {H}, pitch {P} -> local direction ({d[0]:.4f}, {d[1]:.4f}, {d[2]:.4f})")
    k = melee_dir(0, 30)
    print("  Kleerofski melee blast (impulse 2000, heading 0, pitch 30): impulse vector ", tuple(round(2000 * c, 2) for c in k))
    for O, P, T, vT in (((0, 0, 0), math.radians(45), (20, 0, 0), (0, 0, 0)), ((0, 0, 0), math.radians(60), (30, 5, 0), (0, 0, 0)), ((0, 2, 0), math.radians(45 + 5), (0, -1, -40), (3, 0, -2))):
        r = free_fall(O, P, T, vT)
        if r is None:
            print("  free fall", O, round(math.degrees(P), 2), T, vT, "-> no solution")
        else:
            v, t, T2 = r
            print(f"  free fall from {O}, pitch {math.degrees(P):.1f} deg, target {T}, target velocity {vT}: v=({v[0]:.3f}, {v[1]:.3f}, {v[2]:.3f}) |v|={math.sqrt(sum(c * c for c in v)):.3f} t={t:.4f} aimed at ({T2[0]:.3f}, {T2[1]:.3f}, {T2[2]:.3f})")
    # LaunchTool: pitch = min(asin(dy/dist) + graph pitch, 81 deg); target moved back by underhit
    def tool_throw(O, T, vTgt, pitch_deg, underhit, pred, vmin, vmax):
        d = [T[i] - O[i] for i in range(3)]
        dist = math.sqrt(sum(c * c for c in d))
        aim = list(T)
        if dist > 1.2e-7:
            aim = [T[i] - d[i] / dist * underhit for i in range(3)]
        pitch = min(math.asin(max(-1.0, min(1.0, d[1] / dist))) + math.radians(pitch_deg), 1.4137167)
        vT = (pred * vTgt[0], 0.0, pred * vTgt[2])
        r = free_fall(O, pitch, aim, vT)
        if r is None:
            return dist, pitch, None
        v, t, _ = r
        sp = math.sqrt(sum(c * c for c in v))
        if sp < vmin or sp > vmax:
            k = (vmin if sp < vmin else vmax) / sp
            v = [c * k for c in v]
        return dist, pitch, v
    # ThrowFar of an NPC: pitch 45 deg, underhit 3 (constant), prediction 0, speed window 10..80
    for O, T, vT in (((0, 1.5, 0), (0, 0.5, -12), (0, 0, 0)), ((0, 1.5, 0), (20, 1.5, 0), (0, 0, 0)), ((0, 1.5, 0), (0, 1.0, -300), (0, 0, 0))):
        dist, pitch, v = tool_throw(O, T, vT, 45.0, 3.0, 0.0, 10.0, 80.0)
        print("  tool throw from", O, "to", T, "-> distance %.3f, launch pitch %.2f deg," % (dist, math.degrees(pitch)), "velocity", None if v is None else tuple(round(c, 3) for c in v), "" if v is None else "speed %.3f" % math.sqrt(sum(c * c for c in v)))
    # linear underhit graph (0 m -> -1, 10 m -> 3): value at 5 m and at 12 m
    print("  underhit graph (0,-1)-(10,3): at 5 m ->", -1 + (3 - -1) * 0.5, "; at 12 m -> 3 (clamped)")
    print("  81 degrees in radians:", round(math.radians(81), 7), "(the engine's constant 1.4137167)")
    print("  seconds -> ticks: 4294967296 per second (time is a 32.32 fixed point)")


if __name__ == "__main__":
    main()
