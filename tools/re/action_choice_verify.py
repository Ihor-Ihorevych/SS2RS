"""Evidence for docs/re/behavior-action-choice.md (T01-153): what the data says about the choice among the actions of a moment.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries).

It reads the 82 `.cb` character behaviours and prints
  1. the handlers: the behaviour-wide one, one per state and one per distance band (82 + 134 + 381), the moment lists they hold,
     lists that repeat a moment inside one handler, empty lists;
  2. the actions: classes per handler level, the weights (zero, non-positive, the distinct values), the lists in which no action
     can ever be chosen (all weights zero), lists in which every action has a validity term;
  3. the validity terms: which action classes carry which term class, `vt_bNotTrue` per class, terms of the combiner and what they point at;
  4. the two special actions: `CBADefaultAction` ("Pass on handling") and `CBANoAction` ("No action") per level and moment, their weights,
     lists that hold only these two;
  5. the band graph `cba_pgrActionFrequencyOverAreaRatio` (the period of the `Action` moment, see the spec) and the band's
     `cba_fActionPeriodRandomizeMultiplier`: graph classes, number of control points, the distinct curves.

Usage: python -I tools/re/action_choice_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR; about 1 minute)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Enum, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402

NULL = 0xFFFFFFFF


def members(o, cls):
    return {k.split(".", 1)[1]: v for k, v in o if k.startswith(cls + ".")}


def load(d):
    r = parse(d)
    dec = TreeDec(d, r)
    objs, _edob = dec.objects()
    return r, objs


def deref(objs, v):
    if isinstance(v, Ref):
        return None if v.oid == NULL else objs[v.oid]
    return v


def ev(v):
    return v.v if isinstance(v, Enum) else v


def curve(objs, g):
    """A graph as (class, (scale `CGraph.7`, constant `CGraph.12`, x and y of its one control point)); the engine's value of a `CValueGraph` is constant x scale."""
    if g is None:
        return ("null", None)
    if g.type == "CValueGraph":
        base = members(g, "CGraph")
        cp = members(g, "CValueGraph")["1"]            # one CControlPoint
        xy = members(cp, "CControlPoint")["3"]            # Vector2f (x, y)
        return ("CValueGraph", (round(base["7"], 4), round(base["12"], 4), round(dict(xy)["Vector2f.1"], 4), round(dict(xy)["Vector2f.2"], 4)))
    return (g.type, None)


def classify(acts):
    """The shape of an `Action` list of a band: which special classes it holds."""
    if acts is None:
        return "no list"
    if not acts:
        return "empty list"
    cl = tuple(sorted(a.type for a in acts))
    if cl == ("CBANoAction",):
        return "NoAction only"
    if cl == ("CBADefaultAction",):
        return "DefaultAction only"
    if cl == ("CBADefaultAction", "CBANoAction"):
        return "DefaultAction + NoAction"
    return "other: " + ", ".join(cl)


def main(install):
    st = collections.Counter()
    lvl_lists = collections.defaultdict(collections.Counter)      # level -> moment -> lists
    lvl_actions = collections.defaultdict(collections.Counter)    # level -> class -> actions
    weights = collections.Counter()
    term_by_action = collections.Counter()
    term_notrue = collections.Counter()
    comb = collections.Counter()
    special = collections.defaultdict(collections.Counter)
    curves = collections.Counter()
    rnd = collections.Counter()
    freq_cls = collections.Counter()
    lists_only_special = collections.Counter()
    lists_all_termed = collections.Counter()
    lists_pos_free = collections.Counter()
    moments_by_level_file = collections.defaultdict(lambda: collections.defaultdict(set))
    all_actions = collections.Counter()
    all_terms = collections.Counter()
    all_weights = collections.Counter()
    band_shape = collections.Counter()
    pass_prob = collections.Counter()
    anomalies = []
    chain = collections.Counter()
    for name, d in sorted(merged_files(install, ".cb").items()):
        r, objs = load(d)
        st["cb files"] += 1
        idn = lambda v, r=r: r["idnt"][v] if isinstance(v, int) and 0 <= v < len(r["idnt"]) else v
        cb = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CCharacterBehavior")
        cm = members(cb, "CCharacterBehavior")
        handlers = [("behaviour", cm["gdc_ahActionHandler"], name)]
        for s in cm["cb_acsCharacterStates"]:
            so = deref(objs, s)
            sm = members(so, "CCharacterState")
            handlers.append(("state", sm["cs_mhAIMomentHandler"], name))
            for medium in ("cs_dcLand", "cs_dcAir", "cs_dcWater"):
                dca = members(sm[medium], "CDipContentAreas")
                for a in dca["dca_ccbaCustomBehaviorAreas"]:
                    ao = deref(objs, a)
                    am = members(ao, "CCustomBehaviorArea")
                    handlers.append(("band", am["cba_mhAIMomentHandler"], name))
                    c = curve(objs, deref(objs, am["cba_pgrActionFrequencyOverAreaRatio"]))
                    freq_cls[c[0]] += 1
                    curves[(c[0], c[1])] += 1
                    rnd[round(am["cba_fActionPeriodRandomizeMultiplier"], 4)] += 1
        for level, h, fname in handlers:
            st["handlers " + level] += 1
            lists = members(h, "CAIMomentHandler")["caimh_amhMomentHandlers"]
            if lists:
                st["handlers with lists " + level] += 1
            seen = collections.Counter()
            if level == "band":
                al = None
                for lst in lists:
                    lm0 = members(deref(objs, lst), "CBehaviorActionList")
                    if idn(lm0["bal_idMoment"]) == "Action":
                        al = [deref(objs, a) for a in (lm0.get("bal_abaActions") or [])]
                        break
                shape = classify(al)
                band_shape[shape] += 1
                if shape == "DefaultAction + NoAction":
                    wd = sum(members(a, "CBehaviorAction")["ba_fWeight"] for a in al if a.type == "CBADefaultAction")
                    wn = sum(members(a, "CBehaviorAction")["ba_fWeight"] for a in al if a.type == "CBANoAction")
                    pass_prob[round(wd / (wd + wn), 4)] += 1
                if shape in ("empty list", "no list") or (al and all(members(a, "CBehaviorAction")["ba_fWeight"] == 0 for a in al)):
                    anomalies.append((fname.rsplit("/", 1)[-1], shape if shape in ("empty list",) else "all weights 0" if al else "no list"))
            for lst in lists:
                lst = deref(objs, lst)
                lm = members(lst, "CBehaviorActionList")
                mom = idn(lm["bal_idMoment"])
                seen[mom] += 1
                lvl_lists[level][mom] += 1
                moments_by_level_file[level][fname].add(mom)
                acts = [deref(objs, a) for a in (lm.get("bal_abaActions") or [])]

                def count_all(alist):
                    for a_ in alist:
                        am_ = members(a_, "CBehaviorAction")
                        all_actions[a_.type] += 1
                        all_weights["zero" if am_["ba_fWeight"] == 0 else "positive"] += 1
                        t_ = deref(objs, am_["ba_pcValidityTerm"])
                        if t_ is not None:
                            all_terms[(t_.type, ev(members(t_, "CBAValidityTerm").get("vt_bNotTrue")))] += 1
                            if t_.type == "CBAVTTermCombiner":
                                tm_ = members(t_, "CBAVTTermCombiner")
                                for sub in ("tc_pTermA", "tc_pTermB"):
                                    u_ = deref(objs, tm_[sub])
                                    all_terms[("(sub-term) " + u_.type, ev(members(u_, "CBAValidityTerm").get("vt_bNotTrue")))] += 1
                        if a_.type == "SBAList":
                            count_all([deref(objs, x) for x in (members(a_, "CBAList").get("bal_abaActions") or [])])
                count_all(acts)
                if not acts:
                    st["empty lists " + level + " " + str(mom)] += 1
                    continue
                allw0 = True
                allterm = True
                onlysp = True
                for a in acts:
                    am = members(a, "CBehaviorAction")
                    lvl_actions[level][a.type] += 1
                    w = am["ba_fWeight"]
                    weights["zero" if w == 0 else "negative" if w < 0 else "positive"] += 1
                    if w > 0:
                        allw0 = False
                    t = deref(objs, am["ba_pcValidityTerm"])
                    if t is None:
                        allterm = False
                    else:
                        term_by_action[(a.type, t.type)] += 1
                        tb = members(t, "CBAValidityTerm")
                        term_notrue[(t.type, ev(tb.get("vt_bNotTrue")))] += 1
                        if t.type == "CBAVTTermCombiner":
                            tm = members(t, "CBAVTTermCombiner")
                            ta, tb2 = deref(objs, tm["tc_pTermA"]), deref(objs, tm["tc_pTermB"])
                            comb[(ev(tm["vtr_tcTermCombineOperation"]), ta.type, tb2.type,
                                  ev(members(ta, "CBAValidityTerm").get("vt_bNotTrue")),
                                  ev(members(tb2, "CBAValidityTerm").get("vt_bNotTrue")))] += 1
                    if a.type in ("CBADefaultAction", "CBANoAction"):
                        special[a.type][(level, mom, w)] += 1
                    else:
                        onlysp = False
                if allw0:
                    st["lists where every weight is 0 " + level] += 1
                if allterm:
                    lists_all_termed[(level, str(mom))] += 1
                if onlysp:
                    lists_only_special[(level, str(mom), tuple(sorted(a.type for a in acts)))] += 1
                if not any(members(a, "CBehaviorAction")["ba_pcValidityTerm"] for a in acts):
                    st["lists with no term " + level] += 1
                st["lists " + level] += 1
                st["actions " + level] += len(acts)
            for mom, n in seen.items():
                if n > 1:
                    st["handlers repeating a moment " + level + " " + str(mom)] += 1
    print("== 1. handlers and lists")
    for k in sorted(st):
        print(f"   {k}: {st[k]}")
    for lv in ("behaviour", "state", "band"):
        print(f"   lists per moment at level {lv}: {dict(lvl_lists[lv])}")
    print("== 2. actions per level and weights")
    for lv in ("behaviour", "state", "band"):
        print(f"   {lv}: {dict(lvl_actions[lv])}")
    print(f"   weights: {dict(weights)}")
    print("   lists whose actions all carry a term:", dict(lists_all_termed))
    print(f"   all actions including those inside SBAList: {sum(all_actions.values())}  weights: {dict(all_weights)}")
    print(f"   action classes (all): {dict(sorted(all_actions.items()))}")
    print("== 3. validity terms")
    print(f"   terms on actions in handler lists and inside SBAList (incl. combiner sub-terms): {sum(all_terms.values())}")
    for k, v in sorted(all_terms.items(), key=str):
        print(f"      (all) {k[0]} vt_bNotTrue={k[1]}: {v}")
    for k, v in sorted(term_by_action.items()):
        print(f"   action {k[0]} -> term {k[1]}: {v}")
    for k, v in sorted(term_notrue.items(), key=str):
        print(f"   term {k[0]} vt_bNotTrue={k[1]}: {v}")
    for k, v in sorted(comb.items(), key=str):
        print(f"   combiner op={k[0]} A={k[1]}(not {k[3]}) B={k[2]}(not {k[4]}): {v}")
    print("== 4. CBADefaultAction / CBANoAction (level, moment, weight): count")
    for cls in ("CBADefaultAction", "CBANoAction"):
        print(f"   {cls}:")
        for k, v in sorted(special[cls].items(), key=str):
            print(f"      {k}: {v}")
    print("   lists holding only those two classes:")
    for k, v in sorted(lists_only_special.items(), key=str):
        print(f"      {k}: {v}")
    print("== 4b. the `Action` list of the 381 bands, and what a pass-through reaches")
    for k, v in sorted(band_shape.items(), key=lambda kv: -kv[1]):
        print(f"   {k}: {v}")
    print(f"   P(pass on) of the DefaultAction + NoAction bands (Default weight / sum): {dict(sorted(pass_prob.items()))}")
    print(f"   bands with an empty list or all-zero weights: {collections.Counter(a for a in anomalies if a[1] != 'no list')}")
    print("== 5. band frequency graph and randomisation")
    print(f"   graph classes: {dict(freq_cls)}")
    print(f"   cba_fActionPeriodRandomizeMultiplier: {dict(sorted(rnd.items()))}")
    for (cls, pts), n in sorted(curves.items(), key=lambda kv: -kv[1]):
        print(f"   {n:4d} bands: {cls} {pts}")


if __name__ == "__main__":
    inst = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("SS2_DATA_DIR")
    if not inst:
        sys.exit("usage: action_choice_verify.py DATA_DIR")
    main(inst)
