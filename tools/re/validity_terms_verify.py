"""Evidence for docs/re/behavior-validity-terms.md (T01-157): the data that the validity terms and the rarely used actions read.
Data only, read-only on the install (CLEANROOM.md: this script reads DATA files, never the game's binaries).

It prints
  1. over the 82 `.cb` files: every stored validity-term object by class (members and values, `vt_bNotTrue`), the action classes that carry a
     term, and a check that none of the five terms and six actions the roadmap calls unused occurs (counts 0);
  2. the 6 `CBAVTFoeAngle` objects in full, and for the 51 `SBAVTHealth` objects the observation mode, the value, the accumulate switch and the
     value relative to the maximum health of the puppet with the same identifier (`php_slMaxHealth`, `php_slMaxArmor` of the `.ep`);
  3. the 12 stored `CBAPlayerAssigning` objects (the data name of `CBAFoeAssigning`);
  4. the armor of the 124 puppet files (how often the damage split of the health terms matters);
  5. test vectors of the health-term modes and of the angle rule as the script computes them (not read from the game).

Usage: python -I tools/re/validity_terms_verify.py [DATA_DIR]   (DATA_DIR defaults to $SS2_DATA_DIR; about 1 second)
"""
import collections, math, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Enum, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
import ai_verify  # noqa: E402

NULL = 0xFFFFFFFF
UNUSED_TERMS = ("CBAVTHasFoe", "CBAVTHasFoeCandidate", "CBAVTKnockOut", "CBAVTMeleeSuccess", "CBAVTWarMeasure")
UNUSED_ACTIONS = ("CBATryRiding", "CBAChangeAligment", "CBASetActionSet", "CBAForceMovingType", "CBAStopTactic", "CBAUnforceMovingType",
                  "SBAChangeAligment", "SBASetActionSet", "SBAForceMovingType", "CBAFoeAssigning")
HO = {0: "HO_BELOW", 1: "HO_ABOVE", 2: "HO_DROPPED_BELOW", 3: "HO_HEALED_ABOVE", 4: "HO_DROPPED_EVERY", 5: "HO_HEALED_EVERY"}


def val(v):
    return v.v if isinstance(v, Enum) else v


def flat(o):
    return {k.split(".", 1)[1]: v for k, v in o}


# ---- the rules as the spec states them (sections 2 and 3 of the page) ----
def health_term(mode, v, h, d, nominal, accumulate=False, accu=0, same_instant=True):
    """CBAVTHealth::IsValid. h = current health, d = en_slLastDamage, nominal = nominal health, accu = en_slAccuDamage."""
    before = h + d
    if mode == 0: return h < v
    if mode == 1: return v < h
    if mode == 2: return same_instant and h < v and v < before
    if mode == 3: return v < h and v < before
    if mode == 4:
        if accumulate: return v < accu
        if nominal == before: return False
        return int((nominal - h) / v) > int((nominal - before) / v)
    if mode == 5: return math.fmod(h, v) > math.fmod(before, v)
    return False


def wrap_deg(a):
    """fmod-based wrap of (a + 180) into [0, 360) then minus 180 (the angle term's normalisation), degrees"""
    r = math.fmod(a + 180.0, 360.0)
    if r < 0: r += 360.0
    return r - 180.0


def foe_angle_term(rel_dir, own_euler, hmin, hmax, pmin, pmax):
    """CBAVTFoeAngle: rel_dir = (dx, dy, dz) from the puppet to its foe in world axes; own_euler = (heading, pitch) of the puppet in degrees."""
    x, y, z = rel_dir
    n = math.sqrt(x * x + y * y + z * z)
    x, y, z = x / n, y / n, z / n
    pitch = math.degrees(math.asin(max(-1.0, min(1.0, y))))
    heading = 0.0 if (y > 0.99 or y < -0.99) else math.degrees(math.atan2(-x, -z))
    rh = wrap_deg(heading - own_euler[0]); rp = wrap_deg(pitch - own_euler[1])
    return hmin <= rh <= hmax and pmin <= rp <= pmax, rh, rp


def main():
    install = sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].startswith("--") else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    eps = merged_files(install, ".ep")
    low = {k.replace("\\", "/").lower(): k for k in eps}
    health = {}
    armor = collections.Counter(); max_health = collections.Counter()
    for k in sorted(k for k in low if "/databases/puppets/" in k):
        d = eps[low[k]]
        objs, _ = TreeDec(d, parse(d)).objects()
        root = next((o for o in objs.values() if isinstance(o, Obj) and o.type != "CResourceFile"), None)
        for key, v in root:
            if key == "SPuppetParams.1":
                hp = objs[v.oid] if isinstance(v, Ref) else v
                m = flat(hp)
                stem = k.rsplit("/", 1)[-1].rsplit(".", 1)[0].lower()
                health[stem] = (m["1"], m["2"])
                armor[m["2"] > 0] += 1
    term_cls = collections.Counter(); term_members = collections.defaultdict(lambda: collections.defaultdict(collections.Counter))
    carried = collections.Counter(); unused = collections.Counter(); angle_rows = []; health_rows = []; assign_rows = []
    files = 0
    for name, d in sorted(merged_files(install, ".cb").items()):
        r, dec, objs, edob = ai_verify.load(d); files += 1
        idn = lambda v, r=r: r["idnt"][v] if isinstance(v, int) and 0 <= v < len(r["idnt"]) else v
        cb = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CCharacterBehavior")
        cbname = idn(ai_verify.members(cb, "CCharacterBehavior")["cb_idName"])
        for o in objs.values():
            if not isinstance(o, Obj): continue
            t = o.type
            if t in UNUSED_TERMS or t in UNUSED_ACTIONS: unused[t] += 1
            m = flat(o)
            if "ba_pcValidityTerm" in m:
                ref = m["ba_pcValidityTerm"]
                if isinstance(ref, Ref) and ref.oid != NULL and ref.oid in objs: carried[(t, objs[ref.oid].type)] += 1
            if t.startswith("CBAVT") or t.startswith("SBAVT"):
                term_cls[t] += 1
                for k, v in m.items():
                    if k.startswith("vt_") or k.startswith("tc_") or k.startswith("vth_") or k.startswith("fa_") or k.startswith("pd_") or k.startswith("csd_"):
                        term_members[t][k][val(v) if not isinstance(v, Ref) else "ref"] += 1
                if t == "CBAVTFoeAngle":
                    angle_rows.append((cbname, m["fa_fHeadingMin"], m["fa_fHeadingMax"], m["fa_fPitchMin"], m["fa_fPitchMax"], m.get("vt_bNotTrue")))
                if t == "SBAVTHealth":
                    health_rows.append((cbname, val(m["vth_hoObservation"]), m["vth_slValue"], m["vth_bAccumulative"], m.get("vt_bNotTrue")))
            if t == "CBAPlayerAssigning":
                assign_rows.append((cbname, m["pa_bAllowPlayerToBeFoe"], m["pa_bAllowNonPlayerToBeFoe"]))
    print(f"1. the {files} `.cb` files: validity-term objects by class")
    for t, n in sorted(term_cls.items(), key=lambda kv: -kv[1]):
        print(f"   {t}: {n} objects")
        for k, c in term_members[t].items(): print(f"      {k}: {dict(sorted(c.items(), key=lambda kv: str(kv[0])))}")
    print("   which action or term class carries a term (carrier -> term class: count):")
    for (a, b), n in sorted(carried.items(), key=lambda kv: -kv[1]): print(f"      {a} -> {b}: {n}")
    print("   objects of the classes the roadmap calls unused (all must be 0):", dict(unused) if unused else "none (0 of 5 terms, 0 of 10 action names)")
    print("\n2. CBAVTFoeAngle objects (file, heading min, max, pitch min, max, vt_bNotTrue):")
    for row in angle_rows: print("   ", row)
    print("   SBAVTHealth objects: (mode name, value, accumulate) counts:", dict(collections.Counter((HO[m], v, a) for _, m, v, a, _n in health_rows).most_common(12)))
    by_mode = collections.Counter(HO[m] for _, m, _v, _a, _n in health_rows)
    print("   by mode:", dict(by_mode), "; with vt_bNotTrue set:", sum(1 for *_x, n in health_rows if n))
    print("   mode 2 (HO_DROPPED_BELOW) objects, value against the puppet's maximum health and armor (same identifier):")
    for cbn, m, v, a, n in health_rows:
        if m == 2: print(f"      {cbn}: value {v}, max health/armor {health.get(str(cbn).lower(), '(no .ep of that id)')}")
    print("   mode 4 objects: value / maximum health (ratio), where a puppet of the same id exists:", dict(collections.Counter(
        round(v / health[str(cbn).lower()][0], 3) for cbn, m, v, a, n in health_rows if m == 4 and str(cbn).lower() in health and health[str(cbn).lower()][0]).most_common(8)))
    print("   mode 4 accumulate switch set in:", [(cbn, v) for cbn, m, v, a, n in health_rows if m == 4 and a])
    print(f"\n3. CBAPlayerAssigning (data name of CBAFoeAssigning): {len(assign_rows)} objects (file, allow player, allow non-player):")
    for row in assign_rows: print("   ", row)
    print(f"\n4. puppet parameter files: {len(health)}; maximum armor above 0 in {armor[True]}, 0 in {armor[False]}")
    print("   (when armor is 0 the whole damage goes to health and the health terms' health-before-the-hit is exact)")
    print("\n5. test vectors (computed here)")
    print("   mode 4, value 20, nominal 100, hit 85 -> 75 (d = 10): ", health_term(4, 20, 75, 10, 100), "; hit 79 -> 75 (d = 4):", health_term(4, 20, 75, 4, 100))
    print("   mode 4, first hit from full health (nominal 100, 100 -> 70, d = 30, value 20):", health_term(4, 20, 70, 30, 100), "(the equal-case quirk)")
    print("   mode 2, value 50, 60 -> 40 (d = 20) in the same instant:", health_term(2, 50, 40, 20, 100), "; the step after:", health_term(2, 50, 40, 20, 100, same_instant=False))
    print("   mode 3, value 50, health 80, d = 10:", health_term(3, 50, 80, 10, 100), "; health 40:", health_term(3, 50, 40, 10, 100))
    print("   mode 5, value 20, 25 -> 15 (d = 10):", health_term(5, 20, 15, 10, 100), "; 35 -> 25 (d = 10):", health_term(5, 20, 25, 10, 100))
    for label, vec, own in (("foe straight ahead (-Z), level", (0, 0, -10), (0, 0)), ("foe on the +X side", (10, 0, 0), (0, 0)),
                            ("foe on the -X side", (-10, 0, 0), (0, 0)), ("foe behind (+Z)", (0, 0, 10), (0, 0)),
                            ("foe 30 degrees up", (0, 5, -8.66), (0, 0)), ("foe ahead, puppet heading 90", (0, 0, -10), (90, 0))):
        ok, rh, rp = foe_angle_term(vec, own, -45, 45, -90, 90)
        print(f"   FoeAngle {label}: heading {rh:+.1f}, pitch {rp:+.1f} -> in (-45..45, -90..90): {ok}")


if __name__ == "__main__":
    main()
