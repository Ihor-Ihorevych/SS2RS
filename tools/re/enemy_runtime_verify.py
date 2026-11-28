"""Re-runs the measurements behind docs/re/enemy-runtime-behavior.md on an install (read-only).

Joins, per character, the `.cb` behavior (states, land-medium bands, movers, and the action that each band or
the behavior-wide `Action` moment runs) with the `.ep` attack record the action names (`CBAShoot.as_idShoot`,
`CBAMelee.mel_idMeleeAttack`, `CBALeapOntoFoe.lof_idLeapAttack`, `CBARunOverFoe.rof_idRunOverFoe`), and applies
the difficulty table of `player-rules.md` to the health and speed members. Nothing here was run on the original
game: this only reads the data files.

Usage: python -I tools/re/enemy_runtime_verify.py [DATA_DIR] [--table OUT.md]
       (DATA_DIR defaults to $SS2_DATA_DIR; a few seconds)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Enum, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
import enemies_verify as EV  # noqa: E402  (load(), members() [recursive], g())

NULL = 0xFFFFFFFF

# difficulty: name -> (enemy health multiplier, enemy speed multiplier, player-damage-taken multiplier)
# source: docs/re/player-rules.md "Difficulty tables" (ggp_gdGameDifficulties, PC)
DIFFICULTY = {
    "Tourist": (0.5, 0.75, 0.2),
    "Easy": (1.0, 0.75, 0.5),
    "Normal": (1.0, 1.0, 1.0),
    "Hard": (1.0, 1.125, 1.5),
    "Serious": (1.0, 1.25, 2.0),
}


def flat(o, cls):
    """The direct members of one object of class `cls` (not recursive), by their bare name."""
    return {k.split(".", 1)[1]: v for k, v in o if k.startswith(cls + ".")}


def ident(r, v):
    return r["idnt"][v] if isinstance(v, int) and 0 <= v < len(r["idnt"]) else v


def load_cb(d, r):
    dec = TreeDec(d, r)
    objs, edob = dec.objects()
    idn = lambda v: ident(r, v)
    cb = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CCharacterBehavior")
    m = flat(cb, "CCharacterBehavior")
    name = idn(m["cb_idName"])
    align = m["cb_baAligment"].v if isinstance(m["cb_baAligment"], Enum) else m["cb_baAligment"]

    def action_entries(h):
        out = []
        for lst in flat(h, "CAIMomentHandler")["caimh_amhMomentHandlers"]:
            lm = flat(lst, "CBehaviorActionList")
            moment = idn(lm["bal_idMoment"])
            acts = []
            for a in lm.get("bal_abaActions", []) or []:
                o = objs[a.oid] if isinstance(a, Ref) and a.oid != NULL else a
                if isinstance(o, Obj):
                    acts.append((o.type, {k.split(".", 1)[1]: v for k, v in o}))
            out.append((moment, acts))
        return out

    behavior_wide = action_entries(flat(cb, "CCharacterBehavior")["gdc_ahActionHandler"])
    states = []
    for s in [objs[x.oid] if isinstance(x, Ref) else x for x in m["cb_acsCharacterStates"]]:
        sm = flat(s, "CCharacterState")
        land = flat(sm["cs_dcLand"], "CDipContentAreas")
        bands = []
        for a in land["dca_ccbaCustomBehaviorAreas"]:
            ao = objs[a.oid] if isinstance(a, Ref) else a
            am = flat(ao, "CCustomBehaviorArea")
            mv = am["cba_pmcMoveCharacter"]
            mover = objs[mv.oid].type if isinstance(mv, Ref) and mv.oid != NULL else "(none)"
            va = flat(am["cba_avaVelocityAdjuster"], "SAreaVelocityAdjuster")
            bands.append(dict(name=idn(am["cba_idArea"]), dist=am["cba_fDistance"], mover=mover,
                               speed_mult=va.get("ava_fSpeedMultiplier"), actions=action_entries(am["cba_mhAIMomentHandler"])))
        states.append(dict(name=idn(sm["cs_idState"]), bands=bands))
    return dict(name=name, align=align, behavior_wide=behavior_wide, states=states, idnt=r["idnt"])


def ep_attacks(rt, objs, r):
    """Attack records of one puppet parameter file, keyed by their own id name."""
    shoot = {}
    for sp in EV.members(rt, objs, "CShootingProcess"):
        f = flat(sp, "CShootingProcess")
        name = ident(r, f.get("sp_idName"))
        blasts = EV.members(sp, objs, "CSingleBlast")
        shoot[name] = dict(start_dist=f.get("sp_fStartAttackDistance"), max_wait=f.get("sp_fMaxWaitStartAttackPeriod"),
                            stop_angle=f.get("sp_fStopAttackAngle"), wait_before=f.get("sp_fWaitBeforeShooting"),
                            aim_time=f.get("sp_fAimTime"), wait_after=f.get("sp_fWaitAfterShooting"),
                            must_stop=f.get("sp_bMustStopToShoot"), blasts=len(blasts))
    melee = {}
    for ma in EV.members(rt, objs, "CMeleeAttack"):
        f = flat(ma, "CMeleeAttack")
        name = ident(r, f.get("ma_idName"))
        bl = [flat(b, "CMeleeBlast") for b in EV.members(ma, objs, "CMeleeBlast")]
        melee[name] = dict(player_fire=f.get("ma_pfPlayerFire"), blasts=[(b.get("mb_slDamage"), b.get("mb_fDamageRadius")) for b in bl])
    leap = {}
    for la in EV.members(rt, objs, "CLeapAttack"):
        f = flat(la, "CLeapAttack")
        name = ident(r, f.get("la_idName"))
        leap[name] = dict(damage=f.get("la_slInflictedDamage"), run_away=f.get("la_fRunAwayTime"), wait_after=f.get("la_fWaitAfterAttackTime"))
    runover = {}
    for ro in EV.members(rt, objs, "CRunOverFoeAttack"):
        f = flat(ro, "CRunOverFoeAttack")
        name = ident(r, f.get("rof_idName"))
        bl = [flat(b, "CMeleeBlast") for b in EV.members(ro, objs, "CMeleeBlast")]
        runover[name] = dict(speed_mult=f.get("rof_fSpeedMultiplier"), time_min=f.get("rof_fRunOverFoeTimeMin"),
                              time_max=f.get("rof_fRunOverFoeTimeMax"), through_dist=f.get("rof_fRunTroughDistance"),
                              blasts=[(b.get("mb_slDamage"), b.get("mb_fDamageRadius")) for b in bl])
    return dict(shoot=shoot, melee=melee, leap=leap, runover=runover)


def idn_list(idnt, v):
    return idnt[v] if isinstance(v, int) and 0 <= v < len(idnt) else v


def resolve(action_cls, fields, atk, idnt):
    if action_cls == "SBAShoot":
        name = idn_list(idnt, fields.get("as_idShoot")); rec = atk["shoot"].get(name)
        return "shoot", name, rec
    if action_cls == "SBAMelee":
        name = idn_list(idnt, fields.get("mel_idMeleeAttack")); rec = atk["melee"].get(name)
        return "melee", name, rec
    if action_cls == "SBALeapOntoFoe":
        name = idn_list(idnt, fields.get("lof_idLeapAttack")); rec = atk["leap"].get(name)
        return "leap", name, rec
    if action_cls == "SBARunOverFoe":
        name = idn_list(idnt, fields.get("rof_idRunOverFoe")); rec = atk["runover"].get(name)
        return "run_over", name, rec
    return None, None, None


def scaled(base, mult):
    return None if base is None else round(base * mult, 4)


def main():
    args = sys.argv[1:]
    table = args[args.index("--table") + 1] if "--table" in args else None
    if table: args = [a for a in args if a not in ("--table", table)]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")

    cbs = {}
    for name, d in merged_files(install, ".cb").items():
        r = parse(d)
        stem = name.rsplit("/", 1)[-1][:-3].lower()
        cbs[stem] = load_cb(d, r)

    eps = {}
    for n, d in sorted(merged_files(install, ".ep").items()):
        rel = n.split("databases/")[-1]
        if not rel.startswith("puppets/"): continue
        grp = rel.split("/")[1]
        if grp not in ("enemies", "bosses", "animals", "npcs", "enemies_cutsequences", "npcs_cutsequences"): continue
        r, objs, ex, rt = EV.load(d)
        hp = next((v for k, v in rt if k.endswith("SPuppetParams.1") and isinstance(v, Obj)), None)
        if hp is None: continue
        health = [x for _, x in hp][0]
        misc = next(v for k, v in rt if k.endswith("pp_mppMiscPuppetParams"))
        score = EV.g(misc, "mpp_slScore")
        envs = [v for k, v in rt if isinstance(v, Obj) and v.type == "SPuppetEnvironmentParams"]
        env = None
        if envs:
            e = {k.split(".", 1)[-1]: v for k, v in envs[0]}
            env = dict(speed_min=e.get("3"), speed_max=e.get("4"), walk_velocity=e.get("7"), accel=e.get("1"), decel=e.get("2"))
        ka = next((v for k, v in rt if isinstance(v, Obj) and v.type == "SKickingAbilities"), None)
        kick = flat(ka, "SKickingAbilities") if ka is not None else {}
        ko = next((v for k, v in rt if isinstance(v, Obj) and v.type == "SKnockOutParams"), None)
        knockout = flat(ko, "SKnockOutParams") if ko is not None else {}
        stem = rel.rsplit("/", 1)[-1][:-3].lower()
        eps[stem] = dict(grp=grp, health=health, score=score, env=env, kick=kick, knockout=knockout, atk=ep_attacks(rt, objs, r))

    matched = sorted(set(cbs) & set(eps))
    print(f"{len(cbs)} .cb files, {len(eps)} enemy/boss/animal/npc .ep files, {len(matched)} matched by file stem")
    unresolved = 0; rows = []
    for stem in matched:
        cb = cbs[stem]; ep = eps[stem]
        line_rows = []
        for st in cb["states"]:
            for b in st["bands"]:
                for moment, acts in b["actions"]:
                    if moment != "Action": continue
                    for cls, fields in acts:
                        kind, aname, rec = resolve(cls, fields, ep["atk"], cb["idnt"])
                        if kind is None: continue
                        if rec is None: unresolved += 1
                        line_rows.append((st["name"], b["name"], b["dist"], b["mover"], kind, aname, rec))
        for moment, acts in cb["behavior_wide"]:
            if moment != "Action": continue
            for cls, fields in acts:
                kind, aname, rec = resolve(cls, fields, ep["atk"], cb["idnt"])
                if kind is None: continue
                if rec is None: unresolved += 1
                line_rows.append(("(behavior-wide)", "", None, "", kind, aname, rec))
        rows.append((stem, cb, ep, line_rows))
    print("band/behavior-wide actions resolved to an attack record:", sum(len(r[3]) for r in rows) - unresolved, "unresolved:", unresolved)

    # difficulty scaling test vectors for a few common enemies
    for name in ("kleer", "albino", "scifi_grunt", "rhino"):
        if name not in eps: continue
        h = eps[name]["health"]; env = eps[name]["env"] or {}
        print(f"-- {name}: base health {h}, speed {env.get('speed_min')}/{env.get('speed_max')} --")
        for level, (hm, sm, _) in DIFFICULTY.items():
            print(f"   {level}: health {scaled(h, hm)}, speed {scaled(env.get('speed_min'), sm)}/{scaled(env.get('speed_max'), sm)}")

    if table:
        L = ["# Enemy run-time behaviour per character (generated)", "",
             "Generated by `tools/re/enemy_runtime_verify.py --table` from the matched `.cb` (`ai.md`) and `.ep` "
             "(`enemies.md`) files of the retail install (name matched by file stem). See `enemy-runtime-behavior.md`. "
             "Columns: state, land band (name at its outer distance), mover, the kind and id of the action the band "
             "runs at the `Action` moment, and — when the id resolves to a record of the character's own parameter "
             "file — its data (shoot: start distance / wait before / aim time / wait after, seconds, -1 = disabled, "
             "0 = immediate; melee: damage×radius per blast; leap: damage; run over: speed multiplier). A blank "
             "resolution means the id is not one of the character's own attack records (open question 1 of "
             "`enemy-runtime-behavior.md`).", ""]
        for stem, cb, ep, line_rows in rows:
            if not line_rows: continue
            L += [f"## `{stem}` (align {cb['align']}, {ep['grp']})", "",
                  "| State | Band | Dist | Mover | Action | Record |", "|---|---|---|---|---|---|"]
            for st, bname, dist, mover, kind, aname, rec in line_rows:
                if kind == "shoot" and rec:
                    r_ = f"start {rec['start_dist']:g} / wait {rec['wait_before']:g} / aim {rec['aim_time']:g} / after {rec['wait_after']:g} ({rec['blasts']} blast(s))"
                elif kind == "melee" and rec:
                    r_ = ", ".join(f"{d:g}×{rad:g}" for d, rad in rec["blasts"]) or "(no blast)"
                elif kind == "leap" and rec:
                    r_ = f"damage {rec['damage']:g}"
                elif kind == "run_over" and rec:
                    r_ = f"×{rec['speed_mult']:g}"
                else:
                    r_ = ""
                d_ = f"{dist:g}" if isinstance(dist, (int, float)) else ""
                L.append(f"| {st} | {bname} | {d_} | {mover} | {kind} `{aname}` | {r_} |")
            L.append("")
        with open(table, "w", encoding="utf-8", newline="\n") as f: f.write("\n".join(L) + "\n")
        print("wrote", table)


if __name__ == "__main__":
    main()
