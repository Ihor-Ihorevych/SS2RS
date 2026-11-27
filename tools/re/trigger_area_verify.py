"""Evidence for docs/re/trigger-area-rules.md (T01-169): what the data says about detector areas, killers, locators and environment damagers.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries).

It prints
  1. detector areas (`SDetectorAreaProperties` / entity): custom flag x players-only flag, target record kinds, parent kinds, colour values;
  2. killers (`SKillerProperties`): the cross-table kill-instantly x damaging period x damage amount x abyss flag, and what the code does for each
     row (docs/re/trigger-area-rules.md section 3);
  3. environment damagers (`CEnvironmentDamagerProperties`): per world the boxes (size in each axis), type and healing flag;
  4. locators;
  5. the player puppet parameter files (`.ep`) that carry the freezing numbers used by the environment damager.

Usage: python -I tools/re/trigger_area_verify.py [DATA_DIR] [--worlds N] [--jobs J]
       (DATA_DIR defaults to $SS2_DATA_DIR; --worlds N = only the N smallest worlds; --jobs J = worker processes)
"""
import collections, os, sys, zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, _natural, parse, u32  # noqa: E402
from ep_schema import load as load_eps  # noqa: E402

NULL = 0xFFFFFFFF


def member(o, name):
    for k, v in o:
        if k == name or ("." not in name and k.rsplit(".", 1)[-1] == name):
            return v
    return None


def fmt(v):
    return f"{v:.4g}" if isinstance(v, float) else str(v)


def hist(c, top=10):
    return "; ".join(f"{k} x{n}" for k, n in c.most_common(top)) + (" ..." if len(c) > top else "")


def wld_names(install):
    out = {}
    for f in sorted((f for f in os.listdir(install) if f.lower().endswith(".gro")), key=_natural):
        with zipfile.ZipFile(os.path.join(install, f)) as z:
            for i in z.infolist():
                if not i.is_dir() and i.filename.lower().endswith(".wld"):
                    out[i.filename.lower()] = i.file_size
    return sorted(out.items(), key=lambda kv: kv[1])


def read_world(install, wname):
    d = None
    for f in sorted((f for f in os.listdir(install) if f.lower().endswith(".gro")), key=_natural):
        with zipfile.ZipFile(os.path.join(install, f)) as z:
            for i in z.infolist():
                if not i.is_dir() and i.filename.lower() == wname:
                    x = z.read(i)
                    if x[:8] == b"CTSEMETA" and u32(x, 12) == 8:
                        d = x
    return d


def box_of(v):
    """Six floats of a Box3f value (minimum x y z, maximum x y z), or None."""
    try:
        vals = [x for _, x in v]
        return vals if len(vals) == 6 and all(isinstance(x, float) for x in vals) else None
    except Exception:
        return None


def scan_world(arg):
    install, wname = arg
    d = read_world(install, wname)
    r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects()
    try:
        extra, _ = dec.extra_objects(edob)
    except Exception:
        extra = {}
    allo = {**objs, **extra}
    rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}
    typeof = {oid: o.type for oid, o in allo.items() if isinstance(o, Obj)}
    C = collections.defaultdict(collections.Counter); rows = []
    short = wname.rsplit("/", 1)[-1]

    def kind(v):
        if not isinstance(v, Ref) or v.oid == NULL:
            return "null"
        return ("ext:" + os.path.basename(ex[v.oid])) if v.oid in ex else typeof.get(v.oid, "?")

    for oid, o in allo.items():
        if not isinstance(o, Obj):
            continue
        t = o.type
        if t == "CDetectorAreaProperties":
            po = member(o, "ep_bPlayersOnly")
            C["detector custom x players only"][(member(o, "ep_bCustomEventDetectorArea"), po if po is not None else member(o, "CDetectorAreaProperties.3"))] += 1
            tg = member(o, "ep_hepTarget")
            C["detector target record kind"][kind(tg if tg is not None else member(o, "CDetectorAreaProperties.2"))] += 1
        elif t == "CDetectorAreaEntity":
            C["detector parent kind"][kind(member(o, "en_henParent"))] += 1
            C["detector activated flag stored"][member(o, "CDetectorAreaEntity.13")] += 1
        elif t == "SKillerProperties":
            P = "SKillerProperties."
            k = (member(o, "ep_bKillInstantly"), fmt(member(o, "ep_fDamagingPeriod")), member(o, "ep_slDamageAmount"), member(o, "ep_bAbyss"))
            C["killer (kill instantly, period, amount, abyss)"][k] += 1
            ki, per, abyss = k[0], member(o, "ep_fDamagingPeriod"), k[3]
            branch = "periodic damage of the amount (type DMT_KILLER 24), any entity" if (not ki and per and per > 0) else \
                     ("lethal damage to puppets, type " + ("DMT_ABYSS 25" if abyss else "DMT_KILLER 24"))
            C["killer: what the code does (section 3)"][(branch,)] += 1
        elif t == "SKillerEntity":
            C["killer parent kind"][kind(member(o, "en_henParent"))] += 1
        elif t == "CEnvironmentDamagerProperties":
            b = box_of(member(o, "ep_boxEnvironment") or [])
            rows.append((short, member(o, "ep_edtType"), member(o, "ep_bHealing"), b))
        elif t == "SLocatorEntity":
            C["locator sphere x parent kind"][(member(o, "en_bSphere"), kind(member(o, "en_penParent")))] += 1
    return dict(C), rows, short


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    limit = int(sys.argv[sys.argv.index("--worlds") + 1]) if "--worlds" in sys.argv else None
    if limit is not None:
        args = [a for a in args if a != str(limit)]
    jobs = int(sys.argv[sys.argv.index("--jobs") + 1]) if "--jobs" in sys.argv else 1
    if "--jobs" in sys.argv:
        args = [a for a in args if a != str(jobs)]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder or set SS2_DATA_DIR")
    names = wld_names(install)
    names = names[:limit] if limit else names
    C = collections.defaultdict(collections.Counter); rows = []
    work = [(install, n) for n, _ in names]

    def take(res, i):
        c, rw, short = res
        for k, cn in c.items():
            C[k].update(cn)
        rows.extend(rw)
        print(f"[{i + 1}/{len(work)}] {short}", file=sys.stderr, flush=True)

    if jobs > 1:
        import multiprocessing
        with multiprocessing.Pool(jobs) as pool:
            for i, res in enumerate(pool.imap_unordered(scan_world, work)):
                take(res, i)
    else:
        for i, w in enumerate(work):
            take(scan_world(w), i)

    print(f"census of {len(work)} worlds\n")
    for title, sel in (("1. detector areas", "detector"), ("2. killers", "killer"), ("4. locators", "locator")):
        print(title)
        for k in sorted(C):
            if k.startswith(sel):
                print(f"   {k}: " + hist(C[k], 14))
    print("\n3. environment damagers (world, type, healing, box min x y z, max x y z, size x y z)")
    for short, edt, heal, b in sorted(rows, key=lambda r: (r[0], str(r[2]))):
        if b:
            size = [round(b[i + 3] - b[i], 1) for i in range(3)]
            print(f"   {short:30s} type {edt} healing {heal} min {[round(x, 1) for x in b[:3]]} max {[round(x, 1) for x in b[3:]]} size {size}")
        else:
            print(f"   {short:30s} type {edt} healing {heal} (box not decoded)")
    print("\n5. freezing numbers in the player puppet parameter files (.ep)")
    eps = load_eps(install, (".ep",))
    for name, (d, r, objs, edob) in sorted(eps.items()):
        for o in objs.values():
            if isinstance(o, Obj) and o.type == "SPlayerPuppetParams":
                vals = {k.split(".", 1)[-1]: fmt(v) for k, v in o if "Freez" in k or "Unfre" in k}
                if vals:
                    print(f"   {name.rsplit('/', 1)[-1]:24s} {vals}")


if __name__ == "__main__":
    main()
