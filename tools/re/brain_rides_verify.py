"""Evidence for docs/re/brain-rides.md (T01-128): the seat data of the puppet parameter files and the mounts and riders that the worlds place.
Data only, read-only on the install (CLEANROOM.md: this script reads DATA files, never the game's binaries).

It prints
  1. the parameter files that declare seats: per file the seat count and, per seat, its name, flags (`SeatFlags` bit values), the number of
     ways to the seat and the entering and exiting animation sequence lengths;
  2. the distribution of the seat flags over all seat objects, and how many seats carry the way-point or the puppets-can't-seat flag;
  3. (with worlds) the puppets whose properties carry instance mounts (`ep_hcpInstanceMounts`): the ride class and the mounts' seat names and
     characters, and the number of placed puppets with a stored ride link (`en_henRide`) or rider lists.

Usage: python -I tools/re/brain_rides_verify.py [DATA_DIR] [--ep-only] [LIMIT]
       (DATA_DIR defaults to $SS2_DATA_DIR; the `.ep` part takes about 2 minutes, the worlds about 25)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Blob, Enum, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402

NULL = 0xFFFFFFFF
FLAGS = {1: "RIDER_CAN_SHOOT_OWN_WEAPON", 2: "WAY_POINT_SEAT", 4: "PUPPETS_CANT_SEAT", 8: "RIDER_DIES_WITH_RIDE", 16: "PLAYER_CANT_SEAT",
         32: "PLAYER_CAN_BE_DAMAGED", 64: "NON_PLAYER_IS_INVOULNERABLE", 128: "PLAYER_CANT_EXIT"}


def stem(p): return p.replace("\\", "/").rsplit("/", 1)[-1].rsplit(".", 1)[0]


def members(o, cls): return {k.split(".", 1)[1]: v for k, v in o if k.startswith(cls + ".")}


def lineage(parent, name):
    out = []
    while name is not None and name not in out: out.append(name); name = parent.get(name)
    return out


def val(v): return v.v if isinstance(v, Enum) else v


def ep_part(install):
    files = {}; flagc = collections.Counter(); st = collections.Counter(); ways = collections.Counter()
    for name, d in sorted(merged_files(install, ".ep").items()):
        r = parse(d); dec = TreeDec(d, r); objs, _ = dec.objects()
        idn = lambda v, r=r: r["idnt"][v] if isinstance(v, int) and 0 <= v < len(r["idnt"]) else v
        try:
            root = objs[[x.oid for x in next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CResourceFile").get("CResourceFile.rf_crsShared")][0]]
        except Exception:
            continue
        c = root.get("SPuppetParams.4")
        c = objs[c.oid] if isinstance(c, Ref) and c.oid != NULL else c
        lst = c.get("CScriptObjectContainer.1") if isinstance(c, Obj) else None
        if not lst: continue
        seats = []
        for x in lst:
            o = objs[x.oid] if isinstance(x, Ref) else x
            if not isinstance(o, Obj) or o.type != "SPuppetSeatData": st["non-seat objects in the container"] += 1; continue
            m = members(o, "SPuppetSeatData"); fl = val(m["psd_sfSeatFlags"]); wc = m["psd_csoWaysToSeat"]
            wc = objs[wc.oid] if isinstance(wc, Ref) and wc.oid != NULL else wc
            wl = wc.get("CScriptObjectContainer.1") if isinstance(wc, Obj) else []
            nw = len(wl or [])
            seats.append((idn(m["psd_idSeatName"]), fl, nw))
            flagc[fl] += 1; ways[nw] += 1; st["seat objects"] += 1
            for b, n in FLAGS.items(): st["seats with " + n] += bool(fl & b)
        files[stem(name)] = seats
    print(f"1. parameter files with seats: {len(files)}; seat objects: {st['seat objects']}")
    for f, s in sorted(files.items()): print(f"   {f}: {len(s)} seat(s) " + ", ".join(f"{n}[flags {fl}, ways {w}]" for n, fl, w in s))
    print("\n2. seat flag values (value: seats):", dict(sorted(flagc.items())), "; ways to a seat (count: seats):", dict(sorted(ways.items())))
    for k, v in st.items(): print(f"   {k}: {v}")


def world_part(install, limit):
    worlds = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))
    if limit: worlds = worlds[:limit]
    st = collections.Counter(); mounts = collections.Counter()
    for wi, (name, d) in enumerate(worlds):
        r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects(); extra, _ = dec.extra_objects(edob); allo = {**objs, **extra}
        idn = lambda v, r=r: r["idnt"][v] if isinstance(v, int) and 0 <= v < len(r["idnt"]) else v
        T = r["types"]; parent = {t.name: (T[t.parent].name if t.parent in T else None) for t in T.values() if t.members is not None}
        for i, o in allo.items():
            if not isinstance(o, Obj): continue
            fam = lineage(parent, o.type)
            if "SPuppetProperties" in fam:
                st["puppet property records"] += 1
                h = o.get("SPuppetProperties.ep_hcpInstanceMounts")
                if isinstance(h, Ref) and h.oid != NULL and h.oid in allo:
                    st["records with instance mounts"] += 1
                    cm = allo[h.oid]
                    for mo in (cm.get("CCharacterMounts.cm_acmMounts") or []):
                        mo = allo[mo.oid] if isinstance(mo, Ref) else mo
                        if isinstance(mo, Obj):
                            mm = members(mo, "CCharacterMount"); mounts[(idn(mm["cm_idSeat"]), idn(mm["cm_idCharacter"]))] += 1
        for i, o in objs.items():
            if isinstance(o, Obj) and "SPuppetEntity" in lineage(parent, o.type):
                st["placed puppets"] += 1
                rd = o.get("SPuppetEntity.en_henRide"); st["placed puppets with a stored ride link"] += isinstance(rd, Ref) and rd.oid != NULL
        print(f"\r{wi + 1}/{len(worlds)} worlds", end="", file=sys.stderr)
    print(f"\n3. worlds ({len(worlds)})")
    for k, v in st.items(): print(f"   {k}: {v}")
    print("   instance mounts (seat, character): count:", dict(mounts))


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    ep_part(install)
    if "--ep-only" not in sys.argv: world_part(install, int(args[1]) if len(args) > 1 else None)


if __name__ == "__main__":
    main()
