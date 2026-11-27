"""Re-runs the measurements behind docs/re/environment.md on an install (read-only).

For every format-8 `.wld`: the sky cube (`CCubeBackgroundEntity`), haze volumes (`CHazeEntity`), bloom
(`CBloomEntity`), fluids (`CFluidEntity`), force fields, shakers and environment damagers: how many, their
property values, and which renderable member equals which property member. Writes the per-level table.

Also (T01-409): the sky cube's `en_qvPlacement` rotation (identity, pure yaw, or a general rotation) over
all 58 skies, to tell whether the loader may ignore it or must apply it (`docs/re/environment.md`, sky).

Usage: python -I tools/re/wld_env_verify.py [DATA_DIR] [LIMIT] [--table OUT.md]
       (DATA_DIR defaults to $SS2_DATA_DIR; LIMIT = only the LIMIT smallest worlds, default all, about 15 minutes)
"""
import collections, math, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Blob, Enum, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
from wld_root_verify import ids  # noqa: E402

NULL = 0xFFFFFFFF


def quat_matrix(x, y, z, w):
    """Rotation matrix (3x3, rows) of the quaternion (x, y, z, w); same formula as skl_verify.quat_matrix
    and wld_root_verify.matrix."""
    return [[1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
            [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
            [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)]]


def placement_rotation(qv):
    """Classifies a QuatVect's rotation: ('identity', 0.0), ('yaw', degrees) if it is a pure rotation
    about +Y (heading, GEOMETRY.md convention: 0 along -Z, positive turns towards -X/West), or
    ('general', None) if it also tilts the up axis (pitch and/or bank)."""
    q = qv.get("QuatVect.1"); x, y, z, w = (q.get(f"Quaternion4f.{i}") for i in (1, 2, 3, 4))
    r = quat_matrix(x, y, z, w)
    off_axis = max(abs(r[0][1]), abs(r[1][0]), abs(r[1][2]), abs(r[2][1]), abs(r[1][1] - 1))
    if off_axis > 1e-3: return "general", None
    # pure rotation about Y: r = [[cos,0,sin],[0,1,0],[-sin,0,cos]]; heading (0 along -Z, + towards -X)
    # is the angle from -Z to the rotated -Z axis, i.e. atan2(-r[0][2], -r[2][2]) negated to match the
    # engine's heading sign (behavior-validity-terms.md: heading = atan2(-x, -z) of the forward vector).
    fwd = (-r[0][2], -r[1][2], -r[2][2])
    heading = math.degrees(math.atan2(-fwd[0], -fwd[2]))
    if abs(heading) < 1e-2: return "identity", 0.0
    return "yaw", heading


def nums(o, skip=()):
    out = {}
    for k, v in o:
        if k in skip: continue
        if isinstance(v, Enum): out[k] = v.v
        elif isinstance(v, bool): out[k] = int(v)
        elif isinstance(v, (int, float)): out[k] = v
    return out


def box(v):
    if isinstance(v, Obj) and v.type == "Box3f": return [v.get(f"Box3f.{i}") for i in range(1, 7)]
    return None


def check_world(name, d, st, dist, pairs, rows):
    r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects(); extra, _ = dec.extra_objects(edob)
    rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}
    w = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CWorld"); ents = ids(w.get("CWorld.1"))
    recs = {extra[p].get("CEntityProperties.2").oid: extra[p] for p in ids(w.get("CWorld.5")) if p in extra}
    short = name.rsplit("/", 1)[-1][:-4]; row = {"world": short}
    st["worlds"] += 1
    of = lambda t: [i for i in ents if objs[i].type == t]
    # sky
    sky = of("CCubeBackgroundEntity"); row["sky"] = None
    st["worlds with a sky cube entity"] += bool(sky)
    for i in sky:
        e = objs[i]; rec = recs.get(i); rend = objs[e.get("CCubeBackgroundEntity.1").oid]
        tex = rec.get("CCubeBackgroundProperties.1") if rec is not None else None
        path = ex.get(tex.oid, "") if isinstance(tex, Ref) else ""
        faces = [ex.get(rend.get(f"CCubeRenderable.{k}").oid, "").rsplit("/", 1)[-1].lower() for k in (10, 11, 12, 13, 14, 15)]
        row["sky"] = (path.rsplit("/", 1)[-1].lower(), e.get("CCubeBackgroundEntity.en_fSize"), round(e.get("CCubeBackgroundEntity.en_fViewerY"), 2), hex(rec.get("CCubeBackgroundProperties.6")) if rec is not None else "", faces)
        qv = e.get("CCubeBackgroundEntity.en_qvPlacement")
        kind, heading = placement_rotation(qv)
        pos = qv.get("QuatVect.2"); pos = (pos.get("Vector3f.1"), pos.get("Vector3f.2"), pos.get("Vector3f.3"))
        row["sky_rotation"] = (kind, round(heading, 1) if heading is not None else None, tuple(round(c, 1) for c in pos))
        st["sky cube: records with en_qvPlacement read"] += 1
        dist["sky cube: en_qvPlacement rotation kind"][kind] += 1
        st["sky cube: en_qvPlacement position is (0, 0, 0)"] += all(abs(c) < 1e-3 for c in pos)
        if kind == "yaw":
            dist["sky cube: yaw heading, rounded to the nearest 5 degrees"][round(heading / 5) * 5] += 1
            st["sky cube: yaw is a multiple of 90 degrees (tolerance 0.5)"] += min(abs(heading % 90), 90 - abs(heading % 90)) < 0.5
        st["sky cube: record texture is an external .tex"] += path.lower().endswith(".tex")
        st["sky cube: six face textures on the renderable (.10-.15), distinct names"] += len({f for f in faces if f}) > 1
        dist["sky cube size (en_fSize)"][e.get("CCubeBackgroundEntity.en_fSize")] += 1
        st["sky cube: size equals the renderable's CCubeRenderable.1"] += e.get("CCubeBackgroundEntity.en_fSize") == rend.get("CCubeRenderable.1")
        st["sky cube: colour (record .6) equals the renderable's CCubeRenderable.3"] += rec is not None and rec.get("CCubeBackgroundProperties.6") == rend.get("CCubeRenderable.3")
        st["sky cube: record .4 equals the renderable's CCubeRenderable.2"] += rec is not None and rec.get("CCubeBackgroundProperties.4") == rend.get("CCubeRenderable.2")
    # haze / bloom: renderable vs record member equality
    for cls, rcls, pcls, key in (("CHazeEntity", "CHazeEffectRenderable", "CHazeProperties", "haze"), ("CBloomEntity", "CBloomRenderable", "CBloomProperties", "bloom")):
        lst = of(cls); row[key] = []
        st[f"{key} entities"] += len(lst)
        for i in lst:
            e = objs[i]; rec = recs.get(i); rend = objs[e.get(f"{cls}.1").oid]
            if rec is None: continue
            pn = nums(rec); rn = nums(rend); rb = box(rend.get(f"{rcls}.1"))
            row[key].append({"color": hex(int(pn.get(f"{pcls}.1", 0))), "props": {k.split(".")[-1]: round(v, 3) for k, v in pn.items() if k.startswith(pcls)}, "box": [round(x, 1) for x in rb] if rb else None})
            st[f"{key}: records matched with renderables"] += 1
            for rk, rv in rn.items():
                for pk, pv in pn.items():
                    if pk.startswith(pcls) and abs(rv - pv) < 1e-4 * max(1, abs(pv)): pairs[(key, rk, pk)] += 1
            if rb:
                dims = sorted([abs(rb[3] - rb[0]), abs(rb[4] - rb[1]), abs(rb[5] - rb[2])])
                for pk, pv in pn.items():
                    if pk.startswith(pcls):
                        for mult, lab in ((1, "extent"), (0.5, "half extent"), (2, "double extent")):
                            if any(abs(pv - d_ * mult) < 0.01 * max(1, abs(pv)) for d_ in dims): pairs[(key, "Box3f " + lab, pk)] += 1
    dist["haze per world"][len(of("CHazeEntity"))] += 1; dist["bloom per world"][len(of("CBloomEntity"))] += 1
    # fluids, force fields, shakers, damagers
    row["fluids"] = []
    for i in of("CFluidEntity"):
        e = objs[i]; rec = recs.get(i); h = objs[e.get("CFluidEntity.1").oid]
        row["fluids"].append((round(h.get("CFluidHull.1")), h.get("CFluidHull.2"), box(h.get("CFluidHull.3")) and [round(x) for x in box(h.get("CFluidHull.3"))]))
        if rec is not None:
            st["fluids: record density (.3) equals the hull's CFluidHull.1"] += abs(rec.get("CFluidEntityProperties.3") - h.get("CFluidHull.1")) < 1e-6
            st["fluids: record .4 equals the hull's CFluidHull.2"] += abs(rec.get("CFluidEntityProperties.4") - h.get("CFluidHull.2")) < 1e-6
    st["fluid entities"] += len(row["fluids"])
    for t, k in (("CForceFieldEntity", "force fields"), ("SShakerEffectEntity", "shakers"), ("CEnvironmentDamagerEntity", "environment damagers"), ("CPortalEntity", "portals")):
        row[k] = len(of(t)); st[f"{k}"] += len(of(t))
    # water shaded materials are in mtr.md; effects renderables of other kinds
    rows.append(row)


def main():
    args = sys.argv[1:]; table = args[args.index("--table") + 1] if "--table" in args else None
    if table: args = [a for a in args if a not in ("--table", table)]
    install = args[0] if args and not args[0].isdigit() else os.environ.get("SS2_DATA_DIR", "")
    limit = int(args[-1]) if args and args[-1].isdigit() else None
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter); pairs = collections.Counter(); rows = []
    files = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))
    for n, (name, d) in enumerate(files):
        if limit and n >= limit: break
        check_world(name, d, st, dist, pairs, rows); print(f"[{n + 1}/{len(files)}] {name.rsplit('/', 1)[-1]}", file=sys.stderr, flush=True)
    for k, v in sorted(st.items()):
        if v: print(f"{v:9d}  {k}")
    for k, v in sorted(dist.items()): print(k, dict(v.most_common(12)))
    rotated = [(r["world"], r["sky_rotation"]) for r in rows if r.get("sky_rotation") and r["sky_rotation"][0] != "identity"]
    print(f"sky cube: {len(rotated)} of {sum(1 for r in rows if r.get('sky_rotation'))} skies have a non-identity en_qvPlacement rotation:", rotated)
    nonzero_pos = [(r["world"], r["sky_rotation"][2]) for r in rows if r.get("sky_rotation") and any(abs(c) > 1e-3 for c in r["sky_rotation"][2])]
    print(f"sky cube: {len(nonzero_pos)} skies with a non-(0,0,0) en_qvPlacement position:", nonzero_pos)
    for key in ("haze", "bloom"):
        tot = st[f"{key}: records matched with renderables"]
        print(f"{key}: renderable member = property member (count of {tot} matches):", [(f"{rk}={pk}", c) for (kk, rk, pk), c in sorted(pairs.items(), key=lambda t: -t[1]) if kk == key and c >= tot * 0.9][:20])
    if table:
        L = ["# Environment per level: sky, haze, bloom, fluids (generated)", "",
             "Generated by `tools/re/wld_env_verify.py --table` from the `CCubeBackgroundEntity`, `CHazeEntity`, `CBloomEntity`, `CFluidEntity` and related entities of every world of the retail install. "
             "Colours are the stored 32-bit values (hex); haze and bloom list the property record's numeric members by number (`wld-entity-schema.md`, `environment.md`). A blank cell means none.", "",
             "## Sky cube", "", "| World | Sky texture | Size | Viewer Y | Colour | Faces (distinct) |", "|---|---|---|---|---|---|"]
        for r in sorted(rows, key=lambda r: r["world"]):
            s = r["sky"]
            L.append(f"| {r['world']} | " + (f"`{s[0]}` | {s[1]:g} | {s[2]} | `{s[3]}` | {len({f for f in s[4] if f})} |" if s else " | | | | |"))
        L += ["", "## Haze volumes", "", "| World | Count | Volumes (colour: members .2 .3 .4 .5 .6 .7 .10) |", "|---|---|---|"]
        for r in sorted(rows, key=lambda r: r["world"]):
            if r["haze"]: L.append(f"| {r['world']} | {len(r['haze'])} | " + "; ".join(f"`{h['color']}`: " + " ".join(f"{h['props'].get(str(k), '-'):g}" if isinstance(h['props'].get(str(k), '-'), (int, float)) else '-' for k in (2, 3, 4, 5, 6, 7, 10)) for h in r["haze"][:6]) + (" …" if len(r["haze"]) > 6 else "") + " |")
        L += ["", "## Bloom", "", "| World | Count | Colour and first members (.2 .3 .4 .5) |", "|---|---|---|"]
        for r in sorted(rows, key=lambda r: r["world"]):
            if r["bloom"]: L.append(f"| {r['world']} | {len(r['bloom'])} | " + "; ".join(f"`{h['color']}`: " + " ".join(f"{h['props'].get(str(k), 0):g}" for k in (2, 3, 4, 5)) for h in r["bloom"]) + " |")
        L += ["", "## Fluids, force fields, shakers, damagers, portals", "", "| World | Fluids (density, .2, box) | Force fields | Shakers | Environment damagers | Portals |", "|---|---|---|---|---|---|"]
        for r in sorted(rows, key=lambda r: r["world"]):
            if r["fluids"] or r["force fields"] or r["shakers"] or r["environment damagers"] or r["portals"]:
                L.append(f"| {r['world']} | " + "; ".join(f"{f[0]}, {f[1]:g}, {f[2]}" for f in r["fluids"][:4]) + (" …" if len(r["fluids"]) > 4 else "") + f" | {r['force fields'] or ''} | {r['shakers'] or ''} | {r['environment damagers'] or ''} | {r['portals'] or ''} |")
        with open(table, "w", encoding="utf-8", newline="\n") as f: f.write("\n".join(L) + "\n")
        print("wrote", table)


if __name__ == "__main__":
    main()
