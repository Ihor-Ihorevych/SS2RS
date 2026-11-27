"""The moving-animation schemes and the chooser of docs/re/puppet-moving-anim.md (T01-148, read-only, data only).

Reads every `.mas` file and every scheme embedded in a `.mdl` file of the install (all patches) and prints
  1. the schemes: records per list, base-scheme chains, value tables of the three record kinds;
  2. which models use which scheme;
  3. a reference implementation of the chooser rules of the page (the "all" animation, the body override and
     the pose transition) run over the whole selector grid of every scheme, so the coverage statements of the
     page are reproducible: how often the chosen record fits the pose, direction and speed exactly;
  4. the test vectors of the page (selector from entity state, chosen record).

Usage: python -I tools/re/moving_anim_verify.py [DATA_DIR]      (or set SS2_DATA_DIR)
Standard library only; read-only on the install; prints numbers and the animation identifiers of the schemes.
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, merged_files, parse  # noqa: E402

NULL = 0xFFFFFFFF
ANY, INVALID = -2, -1
POSE = "stand crouch dive swim fly fall roll ko ladder parachute death".split()
DIRN = "forward backward left right turnleft turnright up down notmoving".split()
SPEED = "fast slow idle".split()
EMO = "frightened normal brave enrage charge".split()


def sg(v):
    v = getattr(v, "v", v)
    return v - (1 << 32) if isinstance(v, int) and v >= (1 << 31) else v


def lvl(rec, sel, wild):
    return 2 if rec == sel else (1 if rec == wild else 0)


def idstr(r, v):
    return r["idnt"][v] if isinstance(v, int) and 0 <= v < len(r["idnt"]) else "?"


class Scheme:
    def __init__(self, name, r, obj, objs, ext):
        self.name, self.r = name, r
        m = [v for k, v in obj if k.startswith("CMovingAnimationScheme.")]
        self.alls = [self.all_rec(r, x) for x in m[0]]
        self.bodies = [self.body_rec(r, x) for x in m[1]]
        self.poses = [self.pose_rec(r, x) for x in m[2]]
        b = m[3]
        self.base_path = None
        self.base_obj = None
        if isinstance(b, Ref) and b.oid != NULL:
            if b.oid in ext:
                self.base_path = r["rfil"][ext[b.oid]][2].lower()
            else:
                self.base_obj = objs.get(b.oid)

    @staticmethod
    def all_rec(r, o):
        v = [x for _, x in o]
        return dict(id=idstr(r, v[0]), term=bool(v[1]), nominal=v[2], rev=bool(v[3]), pose=sg(v[4]), dir=sg(v[5]), speed=sg(v[6]),
                    tags=[idstr(r, t) for t in v[7]], ready=sg(v[8]), shield=sg(v[9]), emo=sg(v[10]), nobody=bool(v[11]))

    @staticmethod
    def body_rec(r, o):
        v = [x for _, x in o]
        return dict(id=idstr(r, v[0]), tags=[idstr(r, t) for t in v[1]], pose=sg(v[2]), ready=sg(v[3]), shield=sg(v[4]), emo=sg(v[5]))

    @staticmethod
    def pose_rec(r, o):
        v = [x for _, x in o]
        return dict(id=idstr(r, v[0]), rev=bool(v[1]), frm=sg(v[2]), to=sg(v[3]), dir=sg(v[4]), speed=sg(v[5]), tags=[idstr(r, t) for t in v[6]],
                    ready=sg(v[7]), shield=sg(v[8]), emo=sg(v[9]), nobody=bool(v[10]))


def load(install):
    schemes = {}      # lower path -> Scheme (the .mas files)
    models = {}       # model path -> Scheme or path
    for name, d in sorted(merged_files(install, ".mas").items()):
        r = parse(d)
        objs, _ = TreeDec(d, r).objects()
        ext = {ob: fi for ob, fi, _, _ in r["exob"]}
        for o in objs.values():
            if isinstance(o, Obj) and o.type == "CMovingAnimationScheme":
                schemes[name.lower()] = Scheme(name, r, o, objs, ext)
                break
    for name, d in sorted(merged_files(install, ".mdl").items()):
        r = parse(d)
        objs, _ = TreeDec(d, r).objects()
        ext = {ob: fi for ob, fi, _, _ in r["exob"]}
        for o in objs.values():
            if isinstance(o, Obj) and o.type == "CSamCustomModelData":
                v = [x for k, x in o if k.startswith("CSamCustomModelData.")][3]
                if isinstance(v, Ref) and v.oid != NULL:
                    if v.oid in ext:
                        models[name] = ("ext", r["rfil"][ext[v.oid]][2].lower())
                    else:
                        t = objs.get(v.oid)
                        if isinstance(t, Obj) and t.type == "CMovingAnimationScheme":
                            models[name] = ("emb", Scheme(name, r, t, objs, ext))
    return schemes, models


def chain(s, schemes):
    out, seen = [], set()
    while s is not None and id(s) not in seen:
        seen.add(id(s))
        out.append(s)
        if s.base_path:
            s = schemes.get(s.base_path)
        else:
            s = None
    return out


def tag_score(rec_tags, sel_tags):
    score, any_used = 0, 0
    for t in rec_tags:
        if t == "":
            continue
        if t in sel_tags:
            score += 3
        elif t == "<Any>" and any_used < len(sel_tags):
            any_used += 1
            score += 2
        else:
            return -1
    return score


def choose_body(ch, sel):
    best, bkey = None, None
    for s in ch:
        for b in s.bodies:
            sc = tag_score(b["tags"], sel["tags"])
            key = (sc, lvl(b["pose"], sel["pose"], ANY), lvl(b["ready"], sel["ready"], ANY), lvl(b["shield"], sel["shield"], INVALID), lvl(b["emo"], sel["emo"], ANY))
            if bkey is None or key > bkey:
                best, bkey = b, key
    if best is not None and bkey[0] != -1 and all(x != 0 for x in bkey[1:]):
        return best["id"]
    return None


def choose_all(ch, sel):
    """The 'all' animation: returns (record, body id or None, how) with how in exact/partial/none."""
    n = len(sel["tags"])
    best, bkey = None, None
    for s in ch:
        for a in s.alls:
            sc = tag_score(a["tags"], sel["tags"])
            if sc == -1:
                continue
            key = (lvl(a["pose"], sel["pose"], ANY), lvl(a["dir"], sel["dir"], ANY), lvl(a["speed"], sel["speed"], ANY), sc,
                   lvl(a["ready"], sel["ready"], ANY), lvl(a["shield"], sel["shield"], INVALID), lvl(a["emo"], sel["emo"], ANY))
            if bkey is None or key > bkey:
                best, bkey = a, key
        if best is not None and (best["pose"], best["dir"], best["speed"], bkey[3], best["ready"], best["shield"], best["emo"]) == \
                (sel["pose"], sel["dir"], sel["speed"], 3 * n, sel["ready"], sel["shield"], sel["emo"]):
            return best, None, "exact"
    if best is None:
        return None, None, "none"
    body = None
    if not best["nobody"] and (3 * n != bkey[3] or sel["ready"] != best["ready"] or sel["shield"] != best["shield"] or sel["emo"] != best["emo"]):
        body = choose_body(ch, sel)
    return best, body, ("fit" if bkey[:3] == (2, 2, 2) else "partial")


def choose_pose(ch, sel, to):
    n = len(sel["tags"])
    best, bkey = None, None
    for s in ch:
        for p in s.poses:
            if lvl(p["frm"], sel["pose"], ANY) == 0 or lvl(p["to"], to, ANY) == 0 or lvl(p["dir"], sel["dir"], ANY) == 0 or lvl(p["speed"], sel["speed"], ANY) == 0 \
                    or lvl(p["ready"], sel["ready"], ANY) == 0 or lvl(p["shield"], sel["shield"], INVALID) == 0 or lvl(p["emo"], sel["emo"], ANY) == 0:
                continue
            sc = tag_score(p["tags"], sel["tags"])
            if sc == -1:
                continue
            key = (lvl(p["to"], to, ANY), lvl(p["frm"], sel["pose"], ANY), lvl(p["dir"], sel["dir"], ANY), lvl(p["speed"], sel["speed"], ANY), sc,
                   lvl(p["ready"], sel["ready"], ANY), lvl(p["shield"], sel["shield"], INVALID), lvl(p["emo"], sel["emo"], ANY))
            if bkey is None or key >= bkey:
                best, bkey = p, key
    if best is None:
        return None, None
    body = None
    if not best["nobody"] and (3 * n != bkey[4] or sel["ready"] != best["ready"] or sel["shield"] != best["shield"] or sel["emo"] != best["emo"]):
        body = choose_body(ch, sel)
    return best, body


def prepare_selector(pose_epp, speed, slow_threshold, move_dir_md, desired_ready, has_foe, shield, emotion, tools):
    """PrepareMovingAnimData of the page: the entity state -> the six selectors and the tag list."""
    pmap = {1: 0, 2: 1, 3: 2, 4: 3, 5: 4, 6: 5, 7: 6, 8: 7}
    pose = pmap.get(pose_epp, 0)          # 0 and 9 (and any other value) give MAP_STAND
    sp = 2 if speed == 0 else (1 if speed < slow_threshold else 0)
    dmap = {1: 2, 2: 0, 3: 3, 4: 1, 5: 8, 6: 4, 7: 5, 8: 6, 9: 7}
    d = dmap.get(move_dir_md)             # other values keep the previous direction (None here)
    ready = (1 if has_foe else 0) if desired_ready == -1 else desired_ready
    return dict(pose=pose, speed=sp, dir=d, ready=ready, shield=shield, emo=emotion, tags=[t for t in tools[:4]])


def hist(vals, n=10):
    c = collections.Counter(vals)
    return ", ".join(f"{k} x{v}" for k, v in c.most_common(n)) + (f" ... ({len(c)} distinct)" if len(c) > n else "")


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder or set SS2_DATA_DIR")
    schemes, models = load(install)
    emb = [m[1] for m in models.values() if m[0] == "emb"]
    ext = [m[1] for m in models.values() if m[0] == "ext"]
    print(f"{len(schemes)} .mas files; {len(models)} models name a scheme ({len(ext)} external, {len(emb)} embedded); 'custom model data' with none: see the page")
    print("models per external scheme:", hist([p.split("/")[-1] for p in ext], 30))
    allsch = list(schemes.values()) + emb
    print("\n== 1. Schemes (", len(schemes), "files +", len(emb), "embedded )")
    for name, s in sorted(schemes.items()):
        ch = chain(s, schemes)
        print(f"  {name.split('/')[-1]}: all {len(s.alls)}, body {len(s.bodies)}, pose {len(s.poses)}, base {s.base_path.split('/')[-1] if s.base_path else None}, chain length {len(ch)}")
    print("  embedded schemes: all", hist([len(s.alls) for s in emb]), "| base", hist([("file" if s.base_path else ("embedded" if s.base_obj else None)) for s in emb]))
    A = [a for s in allsch for a in s.alls]
    B = [b for s in allsch for b in s.bodies]
    P = [p for s in allsch for p in s.poses]
    print(f"  records over the {len(allsch)} schemes: all {len(A)}, body {len(B)}, pose transition {len(P)}")
    print("\n== 2. 'All' records")
    print("  term flag:", hist([a["term"] for a in A]), "| reverse flag:", hist([a["rev"] for a in A]), "| no-body-override:", hist([a["nobody"] for a in A]))
    print("  pose:", hist([POSE[a["pose"]] if a["pose"] >= 0 else "ANY" for a in A]))
    print("  direction:", hist([DIRN[a["dir"]] if a["dir"] >= 0 else "ANY" for a in A]))
    print("  speed:", hist([SPEED[a["speed"]] if a["speed"] >= 0 else "ANY" for a in A]))
    print("  readiness:", hist([a["ready"] for a in A]), "| shield:", hist([a["shield"] for a in A]), "| emotion:", hist([EMO[a["emo"]] if a["emo"] >= 0 else "ANY" for a in A]))
    print("  tags per record:", hist([len(a["tags"]) for a in A]), "| tag names:", hist([t for a in A for t in a["tags"]]))
    nv = [a["nominal"] for a in A]
    print(f"  nominal velocity: min {min(nv)}, max {max(nv)}, zero in {sum(1 for v in nv if v == 0)}, negative in {sum(1 for v in nv if v < 0)}; records with speed idle and nominal > 0: {sum(1 for a in A if a['speed'] == 2 and a['nominal'] > 0)}")
    print("  animation identifiers (distinct):", len({a['id'] for a in A}))
    print("\n== 3. Body records")
    print("  pose:", hist([POSE[b["pose"]] if b["pose"] >= 0 else "ANY" for b in B]), "| readiness:", hist([b["ready"] for b in B]), "| tags:", hist([t for b in B for t in b["tags"]]))
    print("  shield:", hist([b["shield"] for b in B]), "| emotion:", hist([b["emo"] for b in B]))
    print("\n== 4. Pose transitions")
    print("  (from, to) pairs:", hist([(POSE[p["frm"]] if p["frm"] >= 0 else "ANY", POSE[p["to"]] if p["to"] >= 0 else "ANY") for p in P], 20))
    print("  reverse flag:", hist([p["rev"] for p in P]), "| tags:", hist([t for p in P for t in p["tags"]]), "| dir/speed wildcards:", hist([(p["dir"], p["speed"]) for p in P]))

    print("\n== 5. Chooser coverage (reference implementation of the page over the selector grid, no tags)")
    tot = collections.Counter()
    per = {}
    for name, s in sorted(schemes.items()):
        ch = chain(s, schemes)
        c = collections.Counter()
        for pose in range(8):
            for d in range(9):
                for sp in range(3):
                    for rd in (0, 1):
                        for sh in (0, 1):
                            for em in range(5):
                                sel = dict(pose=pose, dir=d, speed=sp, ready=rd, shield=sh, emo=em, tags=[])
                                rec, body, how = choose_all(ch, sel)
                                c[how] += 1
                                if rec is not None and body is not None:
                                    c["body"] += 1
        per[name] = c
        tot.update(c)
    print("  selectors per scheme: 8 poses x 9 directions x 3 speeds x 2 readiness x 2 shield x 5 emotions = 4320")
    print("  total over", len(schemes), "schemes:", dict(tot))
    for name, c in sorted(per.items()):
        print(f"   {name.split('/')[-1]}: exact {c['exact']}, fit {c['fit']}, partial {c['partial']}, none {c['none']}, with body override {c['body']}")
    print("  the 18 selectors (stand, forward/backward/left/right/turnleft/turnright, fast/slow/idle? see below) per scheme with a fitting record (stand pose, not ready, normal emotion, no shield):")
    for name, s in sorted(schemes.items()):
        ch = chain(s, schemes)
        out = []
        for d in (0, 1, 2, 3, 8):
            for sp in (0, 1, 2):
                rec, body, how = choose_all(ch, dict(pose=0, dir=d, speed=sp, ready=0, shield=0, emo=1, tags=[]))
                if rec is not None and how in ("exact", "fit"):
                    out.append(1)
        print(f"   {name.split('/')[-1]}: {len(out)} of 15 (direction x speed) fit exactly")

    print("\n== 6. Test vectors")
    bip = schemes.get("content/serioussam2/misc/movinganimationschemes/bipedmoving.mas")
    ch = chain(bip, schemes)
    def show(title, sel):
        rec, body, how = choose_all(ch, sel)
        print(f"  {title}: -> {None if rec is None else rec['id']} ({how}), nominal {None if rec is None else round(rec['nominal'], 3)}, body {body}")
    show("stand, forward, fast, not ready", dict(pose=0, dir=0, speed=0, ready=0, shield=0, emo=1, tags=[]))
    show("stand, forward, slow, ready", dict(pose=0, dir=0, speed=1, ready=1, shield=0, emo=1, tags=[]))
    show("stand, left, fast", dict(pose=0, dir=2, speed=0, ready=0, shield=0, emo=1, tags=[]))
    show("stand, backward, fast", dict(pose=0, dir=1, speed=0, ready=0, shield=0, emo=1, tags=[]))
    show("stand, not moving, idle", dict(pose=0, dir=8, speed=2, ready=0, shield=0, emo=1, tags=[]))
    show("crouch, forward, slow", dict(pose=1, dir=0, speed=1, ready=0, shield=0, emo=1, tags=[]))
    for to in (1, 0):
        sel = dict(pose=0 if to == 1 else 1, dir=8, speed=2, ready=0, shield=0, emo=1, tags=[])
        pr, body = choose_pose(ch, sel, to)
        print(f"  pose change {POSE[sel['pose']]} -> {POSE[to]}: {None if pr is None else (pr['id'], 'reverse' if pr['rev'] else 'forward')}, body {body}")
    print("  selector from entity state:")
    for args_, note in (((1, 0.0, 5.0, 5, -1, False, 0, 1, []), "standing, not moving (MD_STOP)"),
                        ((1, 3.0, 5.0, 2, -1, True, 0, 1, []), "walking 3 m/s (threshold 5), forward, has a foe"),
                        ((1, 5.0, 5.0, 4, -1, False, 0, 1, []), "speed equal to the threshold, backward"),
                        ((9, 8.0, 5.0, 3, 1, False, 1, 4, ["Minigun", "Shield"]), "riding wheel pose, right, forced ready, shielded, charge, two tools")):
        print("   ", note, "->", prepare_selector(*args_))


if __name__ == "__main__":
    main()
