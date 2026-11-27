"""Evidence for docs/re/door-mover-rules.md (T01-167): what the data says about doors, animated movers and portals.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries).

It prints
  1. the doors (`SDoorProperties` / `SDoorEntity`): the names of the open and close animation, the mechanism name, closing delays, lock and
     detector flags, the stored state and flags, and for every distinct door model the animations it holds (name, first and last frame, frame
     rate) with the two lengths the engine can compute from them: (last - first) / fps (the length the door uses) and (last - first + 1) / fps
     (the length the animated mover's wait uses);
  2. the animated movers (`SAnimatedMoverProperties`): default animation names, relative-path flag, the animation sets they carry (animations,
     frames, frame rate, lengths, envelopes and keys), parents named by other entities;
  3. the portals (`CPortalEntity`): chapter links, models, animation names.

Usage: python -I tools/re/door_mover_verify.py [DATA_DIR] [--worlds N] [--jobs J]
       (DATA_DIR defaults to $SS2_DATA_DIR; --worlds N = only the N smallest worlds; --jobs J = J worker processes; the full run is about 40 minutes with one job, about 12 with four)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import zipfile  # noqa: E402
from ctse_values import Obj, Ref, TreeDec, _natural, merged_files, parse, u32  # noqa: E402

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


def load(d):
    r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects()
    try:
        extra, _ = dec.extra_objects(edob)
    except Exception:
        extra = {}
    allo = {**objs, **extra}
    rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}
    return r, allo, ex


def pick(install, names):
    """{lower-case path: bytes} of the wanted files only (later archives, the patches, override earlier ones); avoids loading every model of the install."""
    out = {}
    for f in sorted((f for f in os.listdir(install) if f.lower().endswith(".gro")), key=_natural):
        with zipfile.ZipFile(os.path.join(install, f)) as z:
            for i in z.infolist():
                if not i.is_dir() and i.filename.lower() in names:
                    d = z.read(i)
                    if d[:8] == b"CTSEMETA" and u32(d, 12) == 8:
                        out[i.filename.lower()] = d
    return out


def animations_of(d, install, depth=0):
    """CAnimation objects of a model or animation file (and of the .ans files it names): list of (name, first, last, spf, fps, envelopes, keys, events)."""
    r, allo, ex = load(d); idnt = r["idnt"]; out = []
    for o in allo.values():
        if isinstance(o, Obj) and o.type == "CAnimation":
            nm = idnt[member(o, "CAnimation.1")] if isinstance(member(o, "CAnimation.1"), int) and member(o, "CAnimation.1") < len(idnt) else "?"
            envs = member(o, "CAnimation.6") or []
            keys = 0
            for e in envs:
                eo = allo.get(e.oid) if isinstance(e, Ref) else None
                if isinstance(eo, Obj):
                    keys += member(eo, "CAnimEnvelope.4") or 0
            ev = member(o, "CAnimation.8")
            out.append((nm, member(o, "CAnimation.2"), member(o, "CAnimation.3"), member(o, "CAnimation.4"), member(o, "CAnimation.5"), len(envs), keys,
                        1 if isinstance(ev, Ref) and ev.oid != NULL else 0))
    if depth == 0:
        want = {path.lower() for path in set(ex.values()) if path.lower().endswith(".ans")}
        for d2 in pick(install, want).values():
            out += animations_of(d2, install, 1)
    return out


def merged_files_names(install):
    """[(lower-case path, size)] of every .wld in the install (later archives override earlier ones); no file is read."""
    out = {}
    for f in sorted((f for f in os.listdir(install) if f.lower().endswith(".gro")), key=_natural):
        with zipfile.ZipFile(os.path.join(install, f)) as z:
            for i in z.infolist():
                if not i.is_dir() and i.filename.lower().endswith(".wld"):
                    out[i.filename.lower()] = i.file_size
    return list(out.items())


def merge(C, door_models, mover_rows, c, dm, mr):
    for k, cn in c.items():
        C[k].update(cn)
    door_models.update(dm); mover_rows.extend(mr)


def scan_world(arg):
    """One world: (counters, door models, mover animation rows, short name). Top-level so that a process pool can run it."""
    install, wname = arg
    d = pick(install, {wname})[wname]
    r, allo, ex = load(d); idnt = r["idnt"]
    C = collections.defaultdict(collections.Counter); door_models = collections.Counter(); mover_rows = []
    short = wname.rsplit("/", 1)[-1]

    def idname(v):
        return idnt[v] if isinstance(v, int) and 0 <= v < len(idnt) else str(v)

    for oid, o in allo.items():
        if not isinstance(o, Obj):
            continue
        t = o.type
        if t == "SDoorProperties":
            P = "SDoorProperties."
            mref = member(o, P + "1"); model = ex.get(mref.oid, "inline") if isinstance(mref, Ref) and mref.oid != NULL else "null"
            door_models[model] += 1
            C["door open animation"][idname(member(o, P + "9"))] += 1
            C["door close animation (0 = none)"][idname(member(o, P + "10")) or "(none)"] += 1
            C["door mechanism name"][idname(member(o, P + "12"))] += 1
            C["door closing delay"][fmt(member(o, P + "8"))] += 1
            C["door locked at start x players only"][(member(o, P + "13"), member(o, P + "17"))] += 1
            C["door detector front x back"][(member(o, P + "2"), member(o, P + "3"))] += 1
            C["door extra area"][fmt(member(o, P + "5"))] += 1
            C["door safety offset"][fmt(member(o, P + "18"))] += 1
        elif t == "SDoorEntity":
            C["door state / flags stored"][(str(member(o, "SDoorEntity.16")), str(member(o, "SDoorEntity.en_dfFlags")))] += 1
            C["door detection hull set"][("set" if isinstance(member(o, "SDoorEntity.7"), Ref) and member(o, "SDoorEntity.7").oid != NULL else "null")] += 1
        elif t == "SAnimatedMoverProperties":
            P = "SAnimatedMoverProperties."
            cfg = member(o, P + "10"); an = idname(member(o, P + "1"))
            C["mover default animation"][an] += 1
            C["mover relative path"][member(o, P + "2")] += 1
            sets = []
            if isinstance(cfg, Ref) and cfg.oid in allo:
                host = allo[cfg.oid]
                hr = member(host, "CEntityAnimConfiguration.1")
                h = allo.get(hr.oid) if isinstance(hr, Ref) else None
                for inst in (member(h, "CAnimHost.1") or []) if isinstance(h, Obj) else []:
                    si = member(inst, "CAnimSetInstance.1")
                    aset = allo.get(si.oid) if isinstance(si, Ref) else None
                    if isinstance(aset, Obj):
                        for a in member(aset, "CAnimSet.1") or []:
                            ao = allo.get(a.oid) if isinstance(a, Ref) else None
                            if isinstance(ao, Obj):
                                envs = member(ao, "CAnimation.6") or []
                                keys = sum((member(allo[e.oid], "CAnimEnvelope.4") or 0) for e in envs if isinstance(e, Ref) and e.oid in allo and isinstance(allo[e.oid], Obj))
                                sets.append((idname(member(ao, "CAnimation.1")), member(ao, "CAnimation.2"), member(ao, "CAnimation.3"), member(ao, "CAnimation.4"), member(ao, "CAnimation.5"), len(envs), keys))
            C["mover animation sets x animations"][(len(sets),)] += 1
            for (nm, f0, f1, spf, fps, ne, nk) in sets:
                C["mover animation names"][nm] += 1
                C["mover animation frame rate"][fmt(fps)] += 1
                C["mover animation envelopes"][ne] += 1
                if fps:
                    mover_rows.append((short, nm, f0, f1, fps, (f1 - f0 + 1) / fps, nk))
        elif t == "CPortalEntity":
            mref = member(o, "CPortalEntity.en_pmcModel")
            C["portal model"][ex.get(mref.oid, "inline") if isinstance(mref, Ref) and mref.oid != NULL else "null"] += 1
            C["portal chapter link"]["set" if isinstance(member(o, "SBaseEntity.1"), Ref) and member(o, "SBaseEntity.1").oid != NULL else "null"] += 1
            C["portal current animation"][idname(member(o, "CPortalEntity.en_idCurrentAnimation"))] += 1
        elif t in ("SPlatformEntity", "SPlatformProperties"):
            C["platform objects (retail: none expected)"][t] += 1
    return dict(C), door_models, mover_rows, short


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
    names = sorted(merged_files_names(install), key=lambda kv: kv[1])
    C = collections.defaultdict(collections.Counter)
    door_models = collections.Counter(); mover_rows = []; nworlds = 0
    names = names[:limit] if limit else names
    if jobs > 1:
        import multiprocessing
        with multiprocessing.Pool(jobs) as pool:
            results = pool.imap_unordered(scan_world, [(install, n) for n, _ in names])
            for i, (c, dm, mr, short) in enumerate(results):
                merge(C, door_models, mover_rows, c, dm, mr); nworlds += 1
                print(f"[{i + 1}/{len(names)}] {short}", file=sys.stderr, flush=True)
    else:
        for i, (n, _) in enumerate(names):
            c, dm, mr, short = scan_world((install, n))
            merge(C, door_models, mover_rows, c, dm, mr); nworlds += 1
            print(f"[{i + 1}/{len(names)}] {short}", file=sys.stderr, flush=True)

    print(f"census of {nworlds} worlds\n")
    print("1. doors")
    for k in sorted(C):
        if k.startswith(("door", "portal")) and k.startswith("door"):
            print(f"   {k}: " + hist(C[k], 12))
    print(f"   door models ({len(door_models)} distinct): " + hist(door_models, 25))
    print("   animations of the door models (name, first..last frame, fps -> (last-first)/fps and (last-first+1)/fps in seconds):")
    mdls = pick(install, {m.lower() for m in door_models if m not in ("inline", "null")})
    for model, cnt in door_models.most_common():
        if model in ("inline", "null"):
            continue
        d = mdls.get(model.lower())
        if d is None:
            print(f"      {model.rsplit('/', 1)[-1]:40s} (not found as .mdl)"); continue
        rows = animations_of(d, install)
        print(f"      {model.rsplit('/', 1)[-1]:40s} x{cnt}: " + "; ".join(
            f"{nm} {f0}..{f1} @{fmt(fps)} -> {fmt((f1 - f0) / fps)} / {fmt((f1 - f0 + 1) / fps)} s" for (nm, f0, f1, spf, fps, ne, nk, ev) in rows if fps) or "(no animation)")
    print("\n2. animated movers")
    for k in sorted(C):
        if k.startswith("mover"):
            print(f"   {k}: " + hist(C[k], 12))
    if mover_rows:
        lens = sorted(r[5] for r in mover_rows)
        print(f"   animations: {len(mover_rows)}; length (last - first + 1)/fps: min {lens[0]:.3g}, median {lens[len(lens) // 2]:.3g}, max {lens[-1]:.3g} s")
        top = sorted(mover_rows, key=lambda r: -r[5])[:5]
        print("   longest: " + "; ".join(f"{r[0]} {r[1]} {r[2]}..{r[3]} @{fmt(r[4])} = {r[5]:.3g} s" for r in top))
    print("\n3. portals and platforms")
    for k in sorted(C):
        if k.startswith(("portal", "platform")):
            print(f"   {k}: " + hist(C[k], 12))


if __name__ == "__main__":
    main()
