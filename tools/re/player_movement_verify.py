"""The player's stored movement constants (T01-410; read-only on the install).

Reads the two live player puppet parameter files (`EntityParams/PlayerPuppet.ep` for single
player and co-op, `PlayerPuppet_DM.ep` for multiplayer; the third file the install carries,
`Databases/Puppets/Player/Player.ep`, is not read by `CPlayerPuppetEntity` in any reading this
repository has done and is left out) and prints, for each:

  - the three `SPuppetEnvironmentParams` records (land, air-for-the-fly-pose, water): ground
    acceleration/deceleration, the nominal speed range, the walk/run animation threshold and the
    turn rates;
  - `SMovingAbilities`: the step-up and step-down heights and the allowed fall-without-damage
    height;
  - `SLeggedPuppetBodyParams`: the maximum slope a jump may start from, the jump-up velocity, the
    crouch/swim/dive view heights and the keep-breath period;
  - `SPuppetBodyParams`: the standing and sitting eye points (view origin) and the two aim-target
    vectors, plus the model stretch factor;
  - the power-up and freezing numbers that scale movement (`ppp_fSpeedPowerUpMultiplier`,
    `ppp_fFreezeTime`).

It then checks the fixed numbers quoted in `docs/re/player-movement-constants.md` against the
data (exit status 1 on a mismatch) so the table stays reproducible.

Usage: python -I tools/re/player_movement_verify.py [DATA_DIR]      (default: $SS2_DATA_DIR)
Standard library only.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, merged_files, parse, TreeDec  # noqa: E402

FILES = {
    "single player / co-op": "content/serioussam2/databases/entityparams/playerpuppet.ep",
    "multiplayer (deathmatch)": "content/serioussam2/databases/entityparams/playerpuppet_dm.ep",
}

# Expected values (this page's table); (path of qualified member names from the root, expected).
# A numbered member is read by position against the declared order of puppet-record-members.md.
ENV_ORDER = ["pep_fAcceleration", "pep_fDecceleration", "pep_fSpeedMin", "pep_fSpeedMax",
             "pep_fWalkVelocity", "pep_fMovingRotationMin", "pep_fMovingRotationMax",
             "pep_fStandingRotationMin", "pep_fStandingRotationMax"]
BODY_ORDER = ["pbp_strModel", "pbp_pmcModel", "pbp_fStretch", "pbp_fDefaultAnimationLerpTime",
              "pbp_vViewOrigin", "pbp_qvShootOrigin", "pbp_idAimOriginAttachment",
              "pbp_vViewOriginSitting", "pbp_qvShootOriginSitting", "pbp_vViewTarget",
              "pbp_vShootTarget", "pbp_bBonesMoveHulls", "pbp_bForcesPiercingProjectileToExplode",
              "pbp_fPiercingProjectileResilianceMultiplier", "pbp_asdShields", "pbp_bAutoAimable"]

EXPECT = {
    "single player / co-op": {
        "env.land": dict(zip(ENV_ORDER, [75.0, 45.0, 7.5, 7.5, 3.0, None, None, None, None])),
        "env.water": dict(zip(ENV_ORDER, [50.0, 10.0, 6.0, 6.0, 3.0, 600.0, 800.0, 180.0, 180.0])),
        "fall_height": 2.5, "step_up": 2.0, "step_down": 8.0,
        "slope_deg": 60.0, "jump_speed": 10.0,
        "view_crouch": 0.7, "view_swim": 1.5, "view_dive": 1.1, "breath": 60.0,
        "view_origin_y": 1.95, "view_origin_sitting_y": 1.2, "stretch": 0.9,
        "speed_powerup_mult": 2.0, "freeze_time": 60.0,
    },
    "multiplayer (deathmatch)": {
        "env.land": dict(zip(ENV_ORDER, [110.0, 80.0, 12.0, 12.0, 3.0, None, None, None, None])),
        "env.water": dict(zip(ENV_ORDER, [50.0, 10.0, 6.0, 6.0, 3.0, 600.0, 800.0, 180.0, 180.0])),
        "fall_height": 2.5, "step_up": 2.0, "step_down": 8.0,
        "slope_deg": 70.0, "jump_speed": 11.0,
        "view_crouch": 0.7, "view_swim": 1.5, "view_dive": 1.1, "breath": 60.0,
        "view_origin_y": 1.95, "view_origin_sitting_y": 1.2, "stretch": 0.9,
        "speed_powerup_mult": 2.0, "freeze_time": 60.0,
    },
}


def numbered(obj, order):
    """Map an Obj's stored members onto `order` by position (puppet-record-members.md section
    1: the data stores the declared members in declaration order, skipping a declared member the
    file does not write). The player's own records store every member of these two classes (no
    skips), so position i is always `order[i]`; this is checked by the length match below."""
    values = [v for _, v in obj]
    if len(values) != len(order):
        raise ValueError("expected %d stored members, found %d (%s)" % (len(order), len(values), obj.type))
    return dict(zip(order, values))


def env_record(obj):
    return {k: (round(v, 3) if isinstance(v, float) and v < 1e30 else (None if v is None else v))
            for k, v in numbered(obj, ENV_ORDER).items()}


def vec_y(v):
    if isinstance(v, Obj):
        for k, vv in v:
            if k.endswith(".2"):
                return round(vv, 3)
    return None


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder or set SS2_DATA_DIR")
    eps = merged_files(install, ".ep")
    fail = 0
    for label, path in FILES.items():
        if path not in eps:
            sys.exit("missing %s (%s)" % (path, label))
        r = parse(eps[path])
        objs, _ = TreeDec(eps[path], r).objects()
        root = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "SPlayerPuppetParams")

        def resolve(v):
            return objs.get(v.oid, v) if isinstance(v, Ref) else v

        envs = [resolve(v) for k, v in root if k in ("SLeggedPuppetParams.1", "SLeggedPuppetParams.2", "SLeggedPuppetParams.3")]
        land, air, water = (env_record(e) for e in envs)
        ma = resolve(dict((k, v) for k, v in root)["SPuppetParams.7"])
        lpb = resolve(dict((k, v) for k, v in root)["SLeggedPuppetParams.lpp_lpbBody"])
        body = resolve(dict((k, v) for k, v in root)["SPuppetParams.2"])
        body_named = numbered(body, BODY_ORDER)

        got = {
            "env.land": land, "env.air (fly pose only)": air, "env.water": water,
            "fall_height": ma.get("SMovingAbilities.ma_fAllowedFallHeight"),
            "step_up": ma.get("SMovingAbilities.ma_fStepUpHeight2"),
            "step_down": ma.get("SMovingAbilities.ma_fStepDownHeight"),
            "slope_deg": lpb.get("SLeggedPuppetBodyParams.lpbp_fMaxValidSlopeForJump"),
            "jump_speed": lpb.get("SLeggedPuppetBodyParams.lpbp_fJumpUpVelocity"),
            "view_crouch": lpb.get("SLeggedPuppetBodyParams.pbp_fViewHeightCrouch"),
            "view_swim": lpb.get("SLeggedPuppetBodyParams.pbp_fViewHeightSwim"),
            "view_dive": lpb.get("SLeggedPuppetBodyParams.pbp_fViewHeightDive"),
            "breath": lpb.get("SLeggedPuppetBodyParams.pbp_fKeepBreathPeriod"),
            "view_origin_y": vec_y(body_named.get("pbp_vViewOrigin")),
            "view_origin_sitting_y": vec_y(body_named.get("pbp_vViewOriginSitting")),
            "stretch": body_named.get("pbp_fStretch"),
            "speed_powerup_mult": dict((k, v) for k, v in root).get("SPlayerPuppetParams.ppp_fSpeedPowerUpMultiplier"),
            "freeze_time": dict((k, v) for k, v in root).get("SPlayerPuppetParams.ppp_fFreezeTime"),
        }

        print("== %s (%s) ==" % (label, path))
        for k in ("fall_height", "step_up", "step_down", "slope_deg", "jump_speed", "view_crouch",
                  "view_swim", "view_dive", "breath", "view_origin_y", "view_origin_sitting_y",
                  "stretch", "speed_powerup_mult", "freeze_time"):
            print("  %-24s %s" % (k, got[k]))
        for name, rec in (("land", land), ("air (fly pose only)", air), ("water", water)):
            print("  env.%-20s %s" % (name, rec))

        exp = EXPECT[label]
        for k, v in exp.items():
            if k.startswith("env."):
                want = v
                have = got[k]
                for field, w in want.items():
                    h = have.get(field)
                    if w is None:
                        continue  # "no limit" sentinel (a very large float); not checked exactly
                    if h is None or abs(h - w) > 1e-3:
                        print("  MISMATCH %s.%s: data %s, expected %s" % (k, field, h, w))
                        fail += 1
            else:
                h = got.get(k)
                if h is None or abs(h - v) > 1e-3:
                    print("  MISMATCH %s: data %s, expected %s" % (k, h, v))
                    fail += 1
    print("\nresult:", "FAIL (%d)" % fail if fail else "ok")
    return 1 if fail else 0


if __name__ == "__main__":
    sys.exit(main())
