"""Re-runs the measurements behind docs/re/mtr.md on an install (read-only).

Materials are `CShaderPreset` objects: the 91 `.mtr` files, and the ones embedded in `.bmf` and
`.mdl` files (all three are read). Checks: the preset/config structure, the shader class codes in
EXOB, the argument classes, the texture slots (what the referenced textures are), the UV map ids
against the mesh surfaces, the blend types, and the modifier lists.

Usage: python -I tools/re/mtr_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR)
"""
import collections, os, re, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Enum, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402

NULL = 0xFFFFFFFF
# texture slot member -> UV map member, per arguments class (from the member census)
LAYERS = {"CShadeShaderArgs": [(10, 11), (20, 21), (30, 31), (40, 41)]}
HAS_ALPHA = {1, 3, 4, 5, 8, 9, 11}


def val(o, name):
    """The single value held by a CShaderArgVal* member."""
    return o.get(name)[0][1]


def main():
    install = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder (the one containing All_PC_01.gro) or set SS2_DATA_DIR")
    tex = {}
    for n, d in merged_files(install, ".tex").items():
        r = parse(d); objs, _ = TreeDec(d, r).objects()
        t = next(v for v in objs.values() if isinstance(v, Obj) and v.type == "CStaticTexture")
        tex[n] = (t.get("CBaseTexture.6").v, t.get("CBaseTexture.4").v)
    st = collections.Counter(); where = collections.Counter(); nconf = collections.Counter(); platform = collections.Counter()
    flag_class = collections.defaultdict(collections.Counter); flag_extype = collections.defaultdict(collections.Counter)
    slot_names = collections.defaultdict(collections.Counter); slot_fmt = collections.defaultdict(collections.Counter)
    blend = collections.defaultdict(collections.Counter); alpha_flag = collections.Counter(); mods = collections.Counter()
    preset2 = collections.Counter(); args_cls = collections.Counter()

    def token(path):
        b = re.sub(r"\d+$", "", path.rsplit("/", 1)[-1].rsplit(".", 1)[0].lower()).rstrip("_-")
        return b.split("_")[-1] if "_" in b else "(plain)"

    for ext in (".mtr", ".bmf", ".mdl"):
        for name, d in sorted(merged_files(install, ext).items()):
            r = parse(d); objs, _ = TreeDec(d, r).objects()
            rf = {e[0]: e[2] for e in r["rfil"]}; exm = {e[0]: (rf[e[1]], e[2], e[3]) for e in r["exob"]}
            for o in objs.values():
                if isinstance(o, Obj) and o.type == "CShaderPreset":
                    where[ext] += 1; configs = o.get("CShaderPreset.1"); nconf[len(configs)] += 1
                    st["CShaderPreset.3 == 255"] += o.get("CShaderPreset.3") == 255
                    p2 = o.get("CShaderPreset.2")
                    if p2.oid in exm: preset2["external ." + exm[p2.oid][0].rsplit(".", 1)[-1].lower() + " as " + r["exty"].get(exm[p2.oid][2], "?")] += 1
                    else: preset2["null" if p2.oid == NULL else objs[p2.oid].type] += 1
                    for cf in configs:
                        platform[hex(cf.get("CShaderConfig.3").v)] += 1
                        sh = cf.get("CShaderConfig.1").oid; a = objs[cf.get("CShaderConfig.2").oid]; args_cls[a.type] += 1
                        st["shader reference is external (module)"] += sh in exm
                        if sh in exm:
                            path, flags, x = exm[sh]
                            st["module path is Bin/Shaders.module"] += path == "Bin/Shaders.module"
                            flag_class[flags][a.type] += 1; flag_extype[flags][r["exty"].get(x, "?")] += 1
                if not isinstance(o, Obj) or not o.type.endswith("ShaderArgs"): continue
                for k, v in o:
                    if isinstance(v, Obj) and v.type == "CShaderArgValTexture":
                        ref = v[0][1]
                        if ref.oid == NULL: continue
                        p = exm.get(ref.oid, (None,))[0]
                        if p is None: continue
                        slot_names[(o.type, k)][token(p)] += 1
                        t = tex.get(p.lower())
                        if t: slot_fmt[(o.type, k)][t[0]] += 1
                if o.type == "CShadeShaderArgs":
                    b = hex(val(o, "CShaderCommonParams.1").v)
                    p = exm.get(val(o, "CShadeShaderArgs.10").oid, (None,))[0]; t = tex.get(p.lower()) if p else None
                    kind = "no texture" if p is None else "unknown texture" if t is None else "alpha channel" if t[0] in HAS_ALPHA else "no alpha"
                    blend[b][kind] += 1
                    if b == "0x1f4" and t is not None:
                        alpha_flag[(val(o, "CShaderCommonParams.5"), "alpha channel" if t[0] in HAS_ALPHA else "no alpha")] += 1
                for m in o.get("CShaderArgs.1")[0][1]: mods[objs[m.oid].type] += 1
    print("presets by file type:", dict(where)); print("configs per preset:", dict(sorted(nconf.items())))
    print("platform masks:", dict(platform)); print("CShaderPreset.3 == 255:", st["CShaderPreset.3 == 255"], "of", sum(where.values()))
    print("CShaderPreset.2 targets:", dict(preset2)); print("shader references external:", st["shader reference is external (module)"], "of which Bin/Shaders.module:", st["module path is Bin/Shaders.module"])
    print("EXOB flags -> arguments class:", {k: dict(v) for k, v in sorted(flag_class.items())})
    print("EXOB flags -> named external type:", {k: {t: n for t, n in v.items() if t != "CShader"} for k, v in sorted(flag_extype.items())})
    print("arguments classes:", dict(args_cls))
    for key in sorted(slot_names):
        if sum(slot_names[key].values()) >= 25: print(key[0], key[1], "names", dict(slot_names[key].most_common(4)), "formats", dict(slot_fmt[key].most_common(3)))
    print("blend type (shade args) vs first texture:", {b: dict(v) for b, v in sorted(blend.items())})
    print("blend 0x1f4: common member 5 vs first texture alpha:", dict(alpha_flag)); print("modifier classes:", dict(mods))


if __name__ == "__main__":
    main()
