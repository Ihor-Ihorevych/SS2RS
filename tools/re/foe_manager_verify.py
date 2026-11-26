"""Evidence for docs/re/foe-manager-numbers.md (T01-156): the data that the foe manager's numbers read, and the sleep-time rule evaluated on it.
Data only, read-only on the install (CLEANROOM.md: this script reads DATA files, never the game's binaries).

It prints
  1. the 82 `.cb` files: (active range, threat range) pairs with the foe manager's sleep time at the two ends and the middle of its slope,
     the initial reaction period (`cb_fInitialReactionPeriod`) and the auto-change chance (`fad_fAutoChangeFoeChance`; > 0 switches
     `en_bAutoChangesFoes` on), and the resulting reaction delay per difficulty;
  2. the 124 puppet `.ep` files under `Databases/Puppets`: the foe-assigning table (`pp_faaFoeAssigning`: interpreter and table length);
  3. (worlds, slow: about 20 minutes, `--no-worlds` skips) the placed puppets' `en_bCanAttackOwnAligment`, the alignment and the threat sensitivity of
     the puppets that have it set (the flag the hostility branch of `OnBrainDamaged` reads);
  4. the test vectors of the sleep-time rule and of the candidate sort (section 4, computed here, not read from the game).

Usage: python -I tools/re/foe_manager_verify.py [DATA_DIR] [--no-worlds] [LIMIT]   (DATA_DIR defaults to $SS2_DATA_DIR)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Enum, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
import ai_verify  # noqa: E402

# Constants of CFoeManagerEntity::CalculateSleepTime (spec sections 2 and 5): seconds
SLEEP_NO_FOE_PUPPET = 1.0   # no puppet that may be this brain's foe exists
SLEEP_FAR = 5.0             # nearest foe puppet farther than the active range
SLEEP_MIN = 0.1             # nearest foe puppet nearer than the threat range
SLEEP_SLOPE = 0.4           # added over the linear part, so the slope runs 0.1 .. 0.5
DEFAULT_INITIAL_DELAY = 0.5  # ggp_fDefaultInitialDelay (docs/re/rsc-ctl-schema.md)
# dif_fInitialDelayMultiplier of the five PC difficulty levels (docs/re/player-rules.md, generated from the rsc-ctl data)
DIFFICULTY_DELAY = {"Tourist": 1.5, "Easy": 1.25, "Normal": 1.0, "Hard": 0.5, "Serious": 0.25}
AL = {0: "good", 1: "evil", 2: "neutral", 3: "semi-neutral"}
TS = {1: "TS_DEADHORSE", 2: "TS_DAMAGEONLY", 3: "TS_DAMAGEANDSENSE", 4: "TS_SHORTSIGHTED", 5: "TS_STANDARD", 6: "TS_LONGSIGHTED", 7: "TS_HEARTHROUGHWALLS", 8: "TS_SEETHROUGHWALLS"}


def val(v):
    return v.v if isinstance(v, Enum) else v


def sleep_time(d, active, threat):
    """CalculateSleepTime for a nearest-foe distance d (None = no foe puppet), the .cb active range and threat range."""
    if d is None: return SLEEP_NO_FOE_PUPPET
    if d > active: return SLEEP_FAR
    if d < threat: return SLEEP_MIN
    t = (d - threat) / (active - threat) if active != threat else float("nan")
    if t < 0: t = 0.0
    if t >= 1: t = 1.0
    return t * SLEEP_SLOPE + SLEEP_MIN


def lineage(parent, name):
    out = []
    while name is not None and name not in out: out.append(name); name = parent.get(name)
    return out


def cb_part(install):
    pairs = collections.Counter(); react = collections.Counter(); chance = collections.Counter(); files = 0; auto_on = 0
    for name, d in sorted(merged_files(install, ".cb").items()):
        r, dec, objs, edob = ai_verify.load(d)
        cb = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CCharacterBehavior")
        m = ai_verify.members(cb, "CCharacterBehavior")
        files += 1
        pairs[(m["cb_fActiveRange"], m["cb_fThreatRange"])] += 1
        react[m["cb_fInitialReactionPeriod"]] += 1
        fad = m["cb_fadFoeAsigningDescriptor"]
        fad = objs[fad.oid] if isinstance(fad, Ref) else fad
        c = ai_verify.members(fad, "CFoeAsigingDescriptor")["fad_fAutoChangeFoeChance"]
        chance[c] += 1; auto_on += c > 0
    print(f"1. the {files} `.cb` files")
    print("   (active range, threat range) -> files; sleep time at d = threat range, the middle, active range; (A < d: 5.0, d < T: 0.1):")
    for (a, t), n in sorted(pairs.items(), key=lambda kv: -kv[1]):
        mid = (a + t) / 2
        print(f"   ({a:g}, {t:g}): {n} files; sleep {sleep_time(t, a, t):.3f} / {sleep_time(mid, a, t):.3f} / {sleep_time(a, a, t):.3f}")
    print("   cb_fInitialReactionPeriod (-1 = use the global 0.5):", dict(sorted(react.items())))
    print("   fad_fAutoChangeFoeChance:", dict(sorted(chance.items())), f"-> en_bAutoChangesFoes on for {auto_on} files, off for {files - auto_on}")
    print("   reaction delay in seconds, per difficulty (period or 0.5 when negative, times dif_fInitialDelayMultiplier):")
    for lvl, mult in DIFFICULTY_DELAY.items():
        dist = collections.Counter(round((p if p >= 0 else DEFAULT_INITIAL_DELAY) * mult, 4) for p, n in react.items() for _ in range(n))
        print(f"     {lvl:8s} x{mult}: {dict(sorted(dist.items()))}")


def ep_part(install):
    eps = merged_files(install, ".ep")
    low = {k.replace("\\", "/").lower(): k for k in eps}
    puppets = sorted(k for k in low if "/databases/puppets/" in k)
    interp = collections.Counter(); length = collections.Counter(); n = 0
    for k in puppets:
        d = eps[low[k]]
        objs, _ = TreeDec(d, parse(d)).objects()
        root = next((o for o in objs.values() if isinstance(o, Obj) and o.type != "CResourceFile"), None)
        if root is None: continue
        for key, v in root:
            if key.endswith(".pp_faaFoeAssigning"):
                v = objs[v.oid] if isinstance(v, Ref) else v
                mm = ai_verify.members(v, "CFoeAssigning")
                interp[val(mm["fa_ftiFoeTableInterpreter"])] += 1; length[len(mm["fa_aidFoeTable"])] += 1; n += 1
    print(f"\n2. the {len(puppets)} puppet `.ep` files: {n} with a foe-assigning table")
    print("   fa_ftiFoeTableInterpreter (0 FTI_EXCLUDE, 1 FTI_INCLUDE):", dict(interp), "; table length:", dict(length))


def world_part(install, limit):
    worlds = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))
    if limit: worlds = worlds[:limit]
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter)
    for wi, (name, d) in enumerate(worlds):
        r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects()
        T = r["types"]; parent = {t.name: (T[t.parent].name if t.parent in T else None) for t in T.values() if t.members is not None}
        for o in objs.values():
            if not isinstance(o, Obj) or "SPuppetEntity" not in lineage(parent, o.type): continue
            st["placed puppets"] += 1
            own = o.get("SPuppetEntity.en_bCanAttackOwnAligment")
            if own:
                st["en_bCanAttackOwnAligment set"] += 1
                dist["alignment of those"][AL.get(val(o.get("SPuppetEntity.en_baAlignment")), "?")] += 1
                ts = val(o.get("SPuppetEntity.1300")); dist["default threat sensitivity of those"][TS.get(ts, ts)] += 1
        print(f"\r{wi + 1}/{len(worlds)} worlds", end="", file=sys.stderr)
    print(f"\n3. placed puppets of {len(worlds)} worlds")
    for k, v in st.items(): print(f"   {k}: {v}")
    for k, c in dist.items(): print(f"   {k}: {dict(c.most_common(12))}")


def vectors():
    print("\n4. test vectors (computed here)")
    print("   sleep_time(None, 250, 10) =", sleep_time(None, 250, 10), "(no foe puppet)")
    for d in (5, 10, 130, 250, 251):
        print(f"   sleep_time({d}, 250, 10) = {sleep_time(d, 250, 10):.4f}")
    print("   sleep_time(2400, 2500, 10) =", f"{sleep_time(2400, 2500, 10):.4f}")
    # the sort comparator truncates (a - b) to an integer: distances closer than 1 unit compare equal
    cmp = lambda a, b: int(a - b)
    print("   sort comparator int(a - b): (3.2, 3.9) ->", cmp(3.2, 3.9), "(a tie); (3.2, 4.3) ->", cmp(3.2, 4.3), "; (10.0, 2.5) ->", cmp(10.0, 2.5))


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    cb_part(install)
    ep_part(install)
    if "--no-worlds" not in sys.argv: world_part(install, int(args[1]) if len(args) > 1 else None)
    vectors()


if __name__ == "__main__":
    main()
