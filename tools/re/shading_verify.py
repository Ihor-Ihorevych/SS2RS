"""Re-runs the measurements behind docs/re/lighting.md on an install (read-only).

Questions of T01-045 that the data files can answer, over every format-8 `.wld`:
 * directional lights and their baked shadow buffers (`CShadowBuffer`: size, matrix, depth array),
 * the one global-illumination object (`CCubeGISHC`) and who refers to it,
 * point light kinds (omnidirectional or spot), enabled flags, ranges, parents,
 * `EntityShadowType` of static models against baked lightmaps and against other flags,
 * which shader modifiers sit next to a baked lightmap on a model instance (full-bright?).

Usage: python -I tools/re/shading_verify.py [DATA_DIR] [LIMIT] [--tables docs/re/lighting-tables.md]
       (DATA_DIR defaults to $SS2_DATA_DIR; LIMIT = only the LIMIT smallest worlds, default all, about 20 minutes)
"""
import collections, math, os, struct, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Blob, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
from wld_root_verify import ancestry  # noqa: E402

NULL = 0xFFFFFFFF
TWO_PI = 2 * math.pi


def refs(v, out):
    if isinstance(v, Ref): out.add(v.oid)
    elif isinstance(v, Obj):
        for _, x in v: refs(x, out)
    elif isinstance(v, (list, tuple)) and not isinstance(v, (Blob, bytes)):
        for x in v: refs(x, out)


def all_members(o, key):
    """Every value stored under `key` (a record can hold two members with the same number)."""
    return [v for k, v in o if k == key]


def num(v):
    v = getattr(v, "v", v)
    return v if isinstance(v, (int, float)) else None


def check_world(name, d, st, dist, notes):
    r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects(); extra, _ = dec.extra_objects(edob)
    allo = {**objs, **extra}
    props = {p.get("CEntityProperties.2").oid: p for p in extra.values()
             if isinstance(p, Obj) and p.get("CEntityProperties.2") is not None}
    st["worlds"] += 1

    # ---- directional lights and their shadow buffers
    for i, e in objs.items():
        if not (isinstance(e, Obj) and e.type == "CDirectionalLightEntity"): continue
        pr = props[i]; st["directional lights"] += 1
        twelve = all_members(pr, "CDirectionalLightProperties.12")
        dist["directional .12 members (kinds)"][tuple(type(x).__name__ for x in twelve)] += 1
        sb = next((x for x in twelve if isinstance(x, Ref)), None)
        flag12 = next((num(x) for x in twelve if not isinstance(x, Ref)), None)
        dist["directional .12 BOOL x shadow buffer"][(flag12, "buffer" if isinstance(sb, Ref) and sb.oid in allo else "none")] += 1
        size_member = pr.get("CDirectionalLightProperties.20")
        dist["directional .20 (INDEX)"][num(size_member)] += 1
        buf = allo.get(sb.oid) if isinstance(sb, Ref) else None
        dist["directional shadow buffer"]["present" if buf is not None else "none"] += 1
        dist["directional (.7, .8) BOOLs"][(num(pr.get("CDirectionalLightProperties.7")), num(pr.get("CDirectionalLightProperties.8")))] += 1
        dist["directional .8 BOOL x shadow buffer"][(num(pr.get("CDirectionalLightProperties.8")), "buffer" if isinstance(sb, Ref) and sb.oid in allo else "none")] += 1
        dist["directional .8 BOOL x .12 BOOL"][(num(pr.get("CDirectionalLightProperties.8")), flag12)] += 1
        dist["directional .21 (FLOAT)"][num(pr.get("CDirectionalLightProperties.21"))] += 1
        dist["directional .14 (FLOAT)"][num(pr.get("CDirectionalLightProperties.14"))] += 1
        ext = tuple(num(pr.get(f"CDirectionalLightProperties.{k}")) for k in (2, 3, 4))
        dist["directional influence box (.2, .3, .4) bucket"][tuple(("<=100" if (x or 0) <= 100 else "<=1000" if x <= 1000 else "<=10000" if x <= 10000 else ">10000") for x in ext)] += 1
        wbuf = num(allo[sb.oid].get("CShadowBuffer.1")) if isinstance(sb, Ref) and sb.oid in allo else None
        dist["directional (.12 BOOL, .20, shadow buffer width)"][(flag12, num(size_member), wbuf)] += 1
        gi = pr.get("CDirectionalLightProperties.19")
        if isinstance(gi, Ref) and gi.oid in allo:
            g = allo[gi.oid]; st["directional lights that refer to the CCubeGISHC"] += 1
            notes.append(f"CCubeGISHC in {name}: #{gi.oid} members " + ", ".join(f"{k}={brief(v)}" for k, v in g))
            for k, v in g:
                if isinstance(v, list): notes.append(f"CCubeGISHC {k} items: " + "; ".join(brief_deep(x) for x in v))
        tex = pr.get("CDirectionalLightProperties.9")
        if isinstance(tex, Ref) and tex.oid in allo:
            notes.append(f"directional light texture in {name}: #{tex.oid} {allo[tex.oid].type}")
        if buf is not None:
            w, h = num(buf.get("CShadowBuffer.1")), num(buf.get("CShadowBuffer.2"))
            dist["shadow buffer size"][(w, h)] += 1
            st["shadow buffer size equals directional .20"] += w == num(size_member) == h
            arr = buf.get("CShadowBuffer.4")
            st["shadow buffer array length == width*height"] += len(arr) == w * h
            m = buf.get("CShadowBuffer.3")
            vals = [num(x) for x in (list(m) if not isinstance(m, Obj) else [x for _, x in m])]
            if len(vals) == 16 and None not in vals:
                dist["shadow matrix: last row (members 12..15)"][tuple(round(x, 3) for x in vals[12:16])] += 1
                dist["shadow matrix: last column (members 3,7,11,15)"][tuple(round(x, 3) for x in vals[3::4])] += 1
            if len(arr):
                kinds = collections.Counter(type(x).__name__ for x in arr); dist["shadow buffer element kind"][tuple(sorted(kinds))] += 1
                nums = [x for x in arr if isinstance(x, int)]
                if nums:
                    lo, hi = min(nums), max(nums); used = sum(1 for x in nums if x != 65535)
                    dist["shadow buffer value range"][(lo >= 0, hi <= 65535)] += 1
                    dist["shadow buffer share of cells that are not 65535"][("0%" if used == 0 else "<1%" if used * 100 < len(nums) else "<10%" if used * 10 < len(nums) else ">=10%")] += 1
    st["CShadowBuffer objects in the main section"] += sum(1 for o in objs.values() if isinstance(o, Obj) and o.type == "CShadowBuffer")
    st["CShadowBuffer objects in the extra section"] += sum(1 for o in extra.values() if isinstance(o, Obj) and o.type == "CShadowBuffer")

    # ---- point lights
    for i, e in objs.items():
        if not (isinstance(e, Obj) and e.type == "CPointLightEntity"): continue
        src = objs[e.get("CPointLightEntity.2").oid]; pr = props[i]; st["point lights"] += 1
        c1, c2 = num(src.get("CPointLightSource.6")), num(src.get("CPointLightSource.7"))
        omni = c1 is not None and c1 >= TWO_PI - 1e-3
        dist["point light kind"]["omnidirectional" if omni else "spot"] += 1
        dist["point light enabled (CLightSource.5)"][num(src.get("CLightSource.5"))] += 1
        dist["point light parent"]["parent entity" if e.get("CPointLightEntity.3").oid != NULL else "none"] += 1
        rng = num(src.get("CPointLightSource.2"))
        if rng is not None: dist["point light range bucket"][("<=10" if rng <= 10 else "<=30" if rng <= 30 else "<=100" if rng <= 100 else ">100")] += 1
        if not omni: dist["spot outer angle (degrees, rounded)"][round(math.degrees(c1))] += 1
        dist["point light (.7 dark, .13, .14 BOOLs)"][tuple(num(pr.get(f"CPointLightProperties.{k}")) for k in (7, 13, 14))] += 1
        dist["point light .16 (FLOAT)"][num(pr.get("CPointLightProperties.16"))] += 1
        dist["point light .21 (FLOAT)"][num(pr.get("CPointLightProperties.21"))] += 1
        dist["point light .13 x parent"][(num(pr.get("CPointLightProperties.13")), "parent" if e.get("CPointLightEntity.3").oid != NULL else "none")] += 1
        dist["point light cone/soft angle pairs (degrees)"][(num(pr.get("CPointLightProperties.18")), num(pr.get("CPointLightProperties.19")))] += 1
        h, f = num(pr.get("CPointLightProperties.3")), num(pr.get("CPointLightProperties.4"))
        dist["point light hot spot .3 / fall-off .4 (is hot spot 0)"][("hot spot 0" if not h else "hot spot > 0", "fall-off <= 10" if f <= 10 else "fall-off > 10")] += 1
        dist["point light CLightSource.3/.4"][(num(src.get("CLightSource.3")), num(src.get("CLightSource.4")))] += 1

    # ---- shadow type of static models against baked lightmaps
    extra_ovens = {i for i, o in extra.items() if isinstance(o, Obj) and o.type == "CTextureOvenShaderModifier"}
    for pid, p in extra.items():
        if not (isinstance(p, Obj) and p.type == "CStaticModelProperties"): continue
        found = set(); refs(p, found)
        baked = bool(found & extra_ovens)
        shadow = num(p.get("CStaticModelProperties.9"))
        dist["static model: EntityShadowType x baked lightmap"][(shadow, "baked" if baked else "not baked")] += 1
        ent = p.get("CEntityProperties.2")
        if isinstance(ent, Ref) and ent.oid in objs and objs[ent.oid].type == "CStaticModelEntity":
            dist["static model: EntityShadowType x CStaticModelEntity.12"][(shadow, num(objs[ent.oid].get("CStaticModelEntity.12")))] += 1

    # ---- shader modifiers next to a baked lightmap
    for i, o in objs.items():
        if not (isinstance(o, Obj) and o.type == "CModelInstance"): continue
        lst = o.get("CModelInstance.4")
        if not isinstance(lst, Ref) or lst.oid not in objs: continue
        mods = objs[lst.oid].get("CShaderModifierList.1") or []
        kinds = tuple(sorted(objs[m.oid].type for m in mods if isinstance(m, Ref) and m.oid in objs))
        has_oven = "CTextureOvenShaderModifier" in kinds
        dist["modifier classes on an instance with a baked lightmap" if has_oven else "modifier classes on an instance without one"][kinds] += 1


def brief_deep(v):
    if isinstance(v, Obj): return v.type + "(" + ", ".join(f"{k}={brief_deep(x)}" for k, x in v) + ")"
    if isinstance(v, (list, tuple)): return "[" + ", ".join(brief_deep(x) for x in v) + "]"
    return repr(getattr(v, "v", v))


def brief(v):
    if isinstance(v, Ref): return f"ref#{v.oid}"
    if isinstance(v, Blob): return f"<{len(v)} bytes>"
    if isinstance(v, Obj): return f"{v.type}{{{len(v)}}}"
    if isinstance(v, (list, tuple)): return f"[{len(v)} items]"
    return repr(v)


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    out = None
    if "--tables" in sys.argv: out = sys.argv[sys.argv.index("--tables") + 1]; args = [a for a in args if a != out]
    install = args[0] if args and os.path.isdir(args[0]) else os.environ.get("SS2_DATA_DIR", "")
    limit = int(args[-1]) if args and args[-1].isdigit() else None
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder (the one containing All_PC_01.gro) or set SS2_DATA_DIR")
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter); notes = []
    for name, d in sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))[:limit]:
        check_world(name, d, st, dist, notes)
    lines = [f"{v:9d}  {k}" for k, v in sorted(st.items())]
    for k, v in sorted(dist.items()): lines.append(f"{k}: " + str(dict(v.most_common(14))))
    lines += notes[:16]
    lines.append(f'(notes shown: {min(len(notes), 16)} of {len(notes)})')
    text = "\n".join(lines)
    print(text)
    if out:
        body = text if not out.endswith(".md") else (
            "# Lighting: measured tables (T01-045)\n\nGenerated by `tools/re/shading_verify.py` over all 59 worlds; "
            "see `lighting.md` for what the numbers mean. Do not edit by hand.\n\n```text\n" + text + "\n```")
        open(out, "w", encoding="utf-8").write(body + "\n")


if __name__ == "__main__":
    main()
