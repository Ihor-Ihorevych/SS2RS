"""Evidence for docs/re/brain-senses.md (T01-129): the data that the brain's sense rules read. Data only, read-only on the install
(CLEANROOM.md: this script reads DATA files, never the game's binaries).

It prints
  1. the three ranges and the alignment of the 82 `.cb` files (`cb_fActiveRange`, `cb_fSenseRange`, `cb_fThreatRange`, `cb_baAligment`);
  2. for the placed puppets of the worlds: the default threat sensitivity (`EThreatSensitivity` 1 to 8, stored as member 1300), the instance
     overrides of the field of view, the short-sight distance, the sense range, the allowed moving range, the sound-check switch and the alignment,
     with the number of puppets that fall into each rule branch of the brain (which of them ignore sounds, which see farther, and so on);
  3. the test vectors of `CanBeEnemyToMyAlignment` over the four alignments.

Usage: python -I tools/re/brain_senses_verify.py [DATA_DIR] [--cb-only] [LIMIT]
       (DATA_DIR defaults to $SS2_DATA_DIR; the `.cb` part takes about 1 minute, the worlds about 20)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Enum, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
import ai_verify  # noqa: E402

NULL = 0xFFFFFFFF
TS = {1: "TS_DEADHORSE", 2: "TS_DAMAGEONLY", 3: "TS_DAMAGEANDSENSE", 4: "TS_SHORTSIGHTED", 5: "TS_STANDARD", 6: "TS_LONGSIGHTED", 7: "TS_HEARTHROUGHWALLS", 8: "TS_SEETHROUGHWALLS"}
AL = {0: "good", 1: "evil", 2: "neutral", 3: "semi-neutral"}


def enemy_alignment(a, b):
    """CanBeEnemyToMyAlignment(a, b) as read from the binary (a = the asking puppet's alignment)."""
    if a == 2 or b == 2: return False
    if a == 3: return b == 1
    if b == 3: return False
    return a != b


def lineage(parent, name):
    out = []
    while name is not None and name not in out: out.append(name); name = parent.get(name)
    return out


def val(v):
    return v.v if isinstance(v, Enum) else v


def cb_part(install):
    rng = collections.defaultdict(collections.Counter); al = collections.Counter(); rows = collections.Counter()
    for name, d in sorted(merged_files(install, ".cb").items()):
        r, dec, objs, edob = ai_verify.load(d)
        cb = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CCharacterBehavior")
        m = ai_verify.members(cb, "CCharacterBehavior")
        for k in ("cb_fActiveRange", "cb_fSenseRange", "cb_fThreatRange"): rng[k][m[k]] += 1
        al[val(m["cb_baAligment"])] += 1
        rows[(m["cb_fActiveRange"], m["cb_fSenseRange"], m["cb_fThreatRange"])] += 1
    print("1. the 82 `.cb` files")
    for k, c in rng.items(): print(f"   {k}: {dict(sorted(c.items()))}")
    print("   alignment (0 good, 1 evil, 2 neutral, 3 semi-neutral):", dict(sorted(al.items())))
    print("   (active, sense, threat) triples:", dict(rows.most_common()))


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
            ts = val(o.get("SPuppetEntity.1300")); dist["default threat sensitivity"][TS.get(ts, ts)] += 1
            fov = o.get("SPuppetEntity.en_fFOVInstanceOverride"); ss = o.get("SPuppetEntity.en_fInstanceShortSightDistance")
            sr = o.get("SPuppetEntity.en_fInstanceSenseRange"); amr = o.get("SPuppetEntity.en_fAllowedMovingRange")
            nosound = o.get("SPuppetEntity.en_bDisableThreatSoundCheck"); al = val(o.get("SPuppetEntity.en_baAlignment"))
            dist["alignment"][AL.get(al, al)] += 1
            dist["field of view override"][fov] += 1; dist["instance short-sight distance"][ss] += 1
            dist["instance sense range"][sr] += 1; dist["allowed moving range"][amr] += 1; dist["disable threat sound check"][nosound] += 1
            st["FOV override used (>= 0)"] += fov is not None and fov >= 0
            st["instance sense range used (>= 0)"] += sr is not None and sr >= 0
            st["allowed moving range >= 0 (ignores sounds)"] += amr is not None and amr >= 0
            st["sound check disabled (ignores sounds)"] += bool(nosound)
            st["sensitivity 1 to 3 (ignores sounds and, 1 and 2, never looks for a foe)"] += ts in (1, 2, 3)
            st["sensitivity 4 and short-sight distance < 0 (global 40)"] += ts == 4 and (ss is None or ss < 0)
            st["sensitivity 4 and short-sight distance >= 0 (own value)"] += ts == 4 and ss is not None and ss >= 0
            st["sensitivity 6 (unlimited see distance)"] += ts == 6
            st["sensitivity 7 (hears through walls)"] += ts == 7
            st["sensitivity 8 (sees through walls)"] += ts == 8
        print(f"\r{wi + 1}/{len(worlds)} worlds", end="", file=sys.stderr)
    print(f"\n2. placed puppets of {len(worlds)} worlds")
    for k, v in st.items(): print(f"   {k}: {v}")
    for k, c in dist.items(): print(f"   {k}: {dict(c.most_common(12))}")


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    cb_part(install)
    if "--cb-only" not in sys.argv: world_part(install, int(args[1]) if len(args) > 1 else None)
    print("\n3. CanBeEnemyToMyAlignment (row asks, column is the other)")
    print("   " + " ".join(f"{AL[b]:>13}" for b in range(4)))
    for a in range(4): print(f"   {AL[a]:>12} " + " ".join(f"{str(enemy_alignment(a, b)):>13}" for b in range(4)))


if __name__ == "__main__":
    main()
