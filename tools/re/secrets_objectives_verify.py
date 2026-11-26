"""Evidence for docs/re/secrets-objectives-beacons.md (T01-170): what the data says about secrets, mission objectives and navigation beacons.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries).

For every world (.wld, the last patch wins) it counts the placed entities of the three classes and their property records and prints:
  1. per class: entities, records, worlds, and for every member the most common values (the same census the mechanism pages use, limited to
     `CSecretEntity`, `SMissionObjectiveEntity`, `SNavigationalBeaconEntity` and their `...Properties`);
  2. the per-world counts: secrets per world (the level's secret total, because every secret adds 1 to the total), objectives per world and how
     many of them are visible on the scanner, beacons per world;
  3. (inside 1) the text and notify members of the objectives, the chapter links (`SBaseEntity.1`) and the stored states of the three classes
     (the binary announces an accomplished or failed objective only when its notify flag is set and its text is not empty).

Usage: python -I tools/re/secrets_objectives_verify.py [DATA_DIR] [--worlds N]
       (DATA_DIR defaults to $SS2_DATA_DIR; --worlds N = only the N smallest worlds; the full run is about 10 minutes)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, merged_files, parse  # noqa: E402

NULL = 0xFFFFFFFF
CLASSES = {"CSecretEntity", "CSecretProperties", "SMissionObjectiveEntity", "SMissionObjectiveProperties",
           "SNavigationalBeaconEntity", "SNavigationalBeaconProperties"}


def fmt(v):
    if isinstance(v, float):
        return f"{v:.4g}"
    s = str(v)
    return s if len(s) <= 40 else s[:37] + "..."


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    limit = int(sys.argv[sys.argv.index("--worlds") + 1]) if "--worlds" in sys.argv else None
    if limit is not None:
        args = [a for a in args if a != str(limit)]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder or set SS2_DATA_DIR")
    files = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))
    tab = {c: {"n": 0, "worlds": set(), "vals": collections.defaultdict(collections.Counter)} for c in CLASSES}
    perworld = collections.defaultdict(lambda: collections.Counter())
    nworlds = 0
    for n, (wname, d) in enumerate(files):
        if limit and n >= limit:
            break
        r = parse(d)
        dec = TreeDec(d, r)
        objs, edob = dec.objects()
        try:
            extra, _ = dec.extra_objects(edob)
        except Exception:
            extra = {}
        allo = {**objs, **extra}
        nworlds += 1
        typeof = {oid: o.type for oid, o in allo.items() if isinstance(o, Obj)}
        short = wname.rsplit("/", 1)[-1]
        for oid, o in allo.items():
            if not isinstance(o, Obj) or o.type not in CLASSES:
                continue
            t = tab[o.type]
            t["n"] += 1
            t["worlds"].add(short)
            for k, v in o:
                cls, _, mem = k.partition(".")
                if cls in ("CEntity", "CBaseEntity", "CEntityProperties", "CBaseProperties"):
                    continue
                if isinstance(v, Ref):
                    key = "null" if v.oid == NULL else "->" + typeof.get(v.oid, "ref?")
                elif isinstance(v, list) and v and isinstance(v[0], tuple) and v[0][0] == "CTransString.1":
                    key = "text empty" if not v[1][1] else "text set"
                elif isinstance(v, list):
                    key = f"list{len(v)}"
                elif isinstance(v, Obj):
                    key = "obj:" + v.type
                else:
                    key = fmt(v)
                t["vals"][k][key] += 1
            if o.type.endswith("Entity"):
                perworld[short][o.type] += 1
        print(f"[{n + 1}/{len(files)}] {short}", file=sys.stderr, flush=True)

    print(f"{nworlds} worlds")
    for c in sorted(CLASSES):
        t = tab[c]
        print(f"\n{c}: {t['n']} in {len(t['worlds'])} worlds")
        for k, cn in t["vals"].items():
            print(f"   {k:46s} " + "; ".join(f"{a} x{b}" for a, b in cn.most_common(8)) + (" ..." if len(cn) > 8 else ""))
    print("\nper-world counts (secrets, objectives, beacons), worlds that have any:")
    hist = {name: collections.Counter() for name in ("CSecretEntity", "SMissionObjectiveEntity", "SNavigationalBeaconEntity")}
    for w, c in sorted(perworld.items()):
        for name in hist:
            if c[name]:
                hist[name][c[name]] += 1
    for name, h in hist.items():
        print(f"   {name}: entities per world (count: worlds) {dict(sorted(h.items()))}; worlds {sum(h.values())}; entities {sum(k * v for k, v in h.items())}")


if __name__ == "__main__":
    main()
