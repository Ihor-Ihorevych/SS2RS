"""Evidence for docs/re/puppet-tools-carry-rides.md (T01-144): what the data says about the character tools (`CCharacterTool`, the `.ctl` files).

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries).

It prints
  1. the number of `.ctl` files and of `CCharacterTool` objects, and for every animation term (draw, store, pick, throw) how many
     tools leave it empty (the state machine then skips the animation step, spec section 1);
  2. the limbs and attachments the tools use (the limb decides which desired tool replaces which), how many tools stretch with
     the owner, are thrown when the owner dies, straighten up when thrown, name an entity creator, and the health values;
  3. the lerp flags of the four animations and the stretch factors;
  4. the tools of each limb (identifiers only) so a reader can see how many tools compete for the same limb.

Usage: python -I tools/re/tools_state_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR; about 20 seconds)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, TreeDec, merged_files, parse  # noqa: E402
import weapons_verify as wv  # noqa: E402


def ident(r, v):
    return r["idnt"][v] if isinstance(v, int) and 0 <= v < len(r["idnt"]) else v


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    ctls = merged_files(install, ".ctl")
    rows = []
    for name, d in sorted(ctls.items()):
        r = parse(d)
        objs, _ = TreeDec(d, r).objects()
        rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}
        for o in objs.values():
            if isinstance(o, Obj) and o.type == "CCharacterTool":
                fl = dict(wv.flat(o, objs, ex, r["idnt"]))
                fl = {k: (ident(r, v) if k.startswith("ct_id") else v) for k, v in fl.items()}
                rows.append((name.rsplit("/", 1)[-1][:-4], fl))
    print(f"1. {len(ctls)} .ctl files, {len(rows)} CCharacterTool objects")
    for key, label in (("ct_idDrawAnimationTerm", "draw"), ("ct_idStoreAnimationTerm", "store"),
                       ("ct_idPickAnimationTerm", "pick"), ("ct_idThrowAnimationTerm", "throw")):
        empty = sum(1 for _, f in rows if f[key] in ("empty", "", None))
        vals = collections.Counter(f[key] for _, f in rows)
        print(f"   {label} animation term empty in {empty} of {len(rows)}; {len(vals)} distinct values")
    e = lambda f, k: f[k] in ("empty", "", None)
    print("   store term empty and draw term set (the store plays the draw animation backwards):",
          sum(1 for _, f in rows if e(f, "ct_idStoreAnimationTerm") and not e(f, "ct_idDrawAnimationTerm")),
          "; both empty (the store is skipped):", sum(1 for _, f in rows if e(f, "ct_idStoreAnimationTerm") and e(f, "ct_idDrawAnimationTerm")),
          "; store term set:", sum(1 for _, f in rows if not e(f, "ct_idStoreAnimationTerm")))
    print("   tools with a throw animation:", sum(1 for _, f in rows if not e(f, "ct_idThrowAnimationTerm")))
    print("2. limbs, attachments and flags")
    limbs = collections.Counter(f["ct_idLimb"] for _, f in rows)
    print("   limb:", dict(limbs))
    print("   attachment:", dict(collections.Counter(f["ct_idAttachment"] for _, f in rows)))
    print("   target bone:", dict(collections.Counter(f["ct_idTargetBone"] for _, f in rows)))
    for key in ("ct_bStretchWithOwner", "ct_bThrownWhenOwnerDies", "ct_bStraightenUpWhenThrowing"):
        print(f"   {key}:", dict(collections.Counter(f[key] for _, f in rows)))
    print("   entity creator set:", sum(1 for _, f in rows if f["ct_ptecToolEntityCreator"] not in ("-", None)), "of", len(rows))
    print("   health:", dict(collections.Counter(f["ct_slHealth"] for _, f in rows)))
    print("   model pointer null:", sum(1 for _, f in rows if f["ct_pmcModel"] in ("-", None)))
    print("3. lerp flags (draw/store/pick/throw) and stretch")
    for key in ("ct_bLerpDrawAnimation", "ct_bLerpStoreAnimation", "ct_bLerpPickAnimation", "ct_bLerpThrowAnimation"):
        print(f"   {key}:", dict(collections.Counter(f[key] for _, f in rows)))
    print("   stretch:", dict(sorted(collections.Counter(f["ct_fStretch"] for _, f in rows).items())))
    print("   thrown-when-owner-dies tools:", [n for n, f in rows if f["ct_bThrownWhenOwnerDies"]])
    print("   tools with a pick animation:", [n for n, f in rows if f["ct_idPickAnimationTerm"] not in ("empty", "", None)])
    print("   tools with an entity creator:", [n for n, f in rows if f["ct_ptecToolEntityCreator"] not in ("-", None)])
    print("   tools with health above -1:", [(n, f["ct_slHealth"]) for n, f in rows if f["ct_slHealth"] != -1])
    print("4. tools per limb")
    for limb in sorted(limbs, key=str):
        print(f"   {limb}: {[n for n, f in rows if f['ct_idLimb'] == limb]}")

    print("5. shield descriptions (pbp_asdShields) of the puppet parameter files: (tool, part) pairs")
    cnt = collections.Counter(); files = 0; total = 0
    for name, d in sorted(merged_files(install, ".ep").items()):
        r = parse(d)
        if not any(t.name == "CShieldDescription" for t in r["types"].values()): continue
        objs, _ = TreeDec(d, r).objects()
        rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}
        found = {}
        for o in objs.values():
            if isinstance(o, Obj):
                for t in nested(o, "CShieldDescription"): found[id(t)] = t
                if o.type == "CShieldDescription": found[id(o)] = o
        pairs = []
        for t in found.values():
            fl = dict(wv.flat(t, objs, ex, r["idnt"]))
            pairs.append((ident(r, fl.get("sd_idShieldTool")), ident(r, fl.get("sd_idShieldPart"))))
        if pairs:
            files += 1; total += len(pairs)
            for pr in pairs: cnt[pr] += 1
            print(f"   {name.split('databases/')[-1]}: {sorted(pairs, key=str)}")
    print(f"   {total} shield descriptions in {files} files; distinct pairs {dict(cnt)}")

    print("6. default riders declared in the puppet parameter files (pp_hcmGenericMounts: CCharacterMount records)")
    total = 0
    for name, d in sorted(merged_files(install, ".ep").items()):
        r = parse(d)
        if not any(t.name == "CCharacterMount" for t in r["types"].values()): continue
        objs, _ = TreeDec(d, r).objects()
        rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}
        found = {}
        for o in objs.values():
            if isinstance(o, Obj):
                for t in nested(o, "CCharacterMount"): found[id(t)] = t
                if o.type == "CCharacterMount": found[id(o)] = o
        for t in found.values():
            fl = dict(wv.flat(t, objs, ex, r["idnt"]))
            total += 1
            print(f"   {name.split('databases/')[-1]}: seat {ident(r, fl.get('cm_idSeat'))!r}, character {ident(r, fl.get('cm_idCharacter'))!r}, "
                  f"behavior {ident(r, fl.get('cm_idBehavior'))!r}, params {fl.get('cm_pppPuppetParams')!r}")
    print(f"   {total} CCharacterMount records in the puppet parameter files")


def nested(o, typ):
    for k, v in o:
        if isinstance(v, Obj):
            if v.type == typ: yield v
            yield from nested(v, typ)
        elif isinstance(v, list):
            for x in v:
                if isinstance(x, Obj):
                    if x.type == typ: yield x
                    yield from nested(x, typ)


if __name__ == "__main__":
    main()
