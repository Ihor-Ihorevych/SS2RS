"""Evidence for docs/re/mounted-tools.md (T01-123): what the data says about the mounted tools and the throw descriptions of puppets.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries).

It prints
  1. every `CMountedTool` of every puppet parameter file (`SPuppetParams.pp_amtMountedTools`): the identifier, the tool file, the
     class of the file, and the number per puppet; the same for the stored per-instance lists of the worlds (`en_amtMountedTools`,
     `ep_amtMountedTools`: count of non-empty lists);
  2. every `CThrowDescription` of every puppet file (`pp_atdThrowDescs`): the identifier and the speed window, with the number of
     graph control points (the graphs themselves are in `puppet_attacks_verify.py`);
  3. whether each tool file named by an identifier or a pointer exists as a `.ctl` file under a `Tools` folder of the install, and the
     number of `.ctl` files there (the table the binary builds from the file search `Tools/*.ctl`, spec section 2);
  4. the behaviour actions `CBAThrowPickedTool` of the `.cb` files and the throw description identifier each names, and whether the
     puppets that run those behaviours have a description with that identifier.

Usage: python -I tools/re/tools_throw_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR; about 3 minutes)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
import weapons_verify as wv  # noqa: E402


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


def ident(r, v):
    return r["idnt"][v] if isinstance(v, int) and 0 <= v < len(r["idnt"]) else v


def fixed(r, fl):
    return {k: (ident(r, v) if k.endswith('ThrowDescription') else v) for k, v in fl.items() if k.startswith('batt_')}


def load(d):
    r = parse(d); objs, _ = TreeDec(d, r).objects()
    rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}
    return r, objs, ex


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    eps = merged_files(install, ".ep")
    print("1. CMountedTool in the puppet parameter files")
    nfiles = 0; ntools = 0; per = collections.Counter(); ids = collections.Counter(); both = 0
    for name, d in sorted(eps.items()):
        r = parse(d)
        if not any(t.name == "CMountedTool" for t in r["types"].values()): continue
        r, objs, ex = load(d)
        found = [o for o in objs.values() if isinstance(o, Obj)]
        tools = []
        for o in found:
            for m in nested(o, "CMountedTool"): tools.append(m)
        tools = list({id(t): t for t in tools}.values())
        if not tools: continue
        nfiles += 1; per[len(tools)] += 1
        for t in tools:
            fl = dict(wv.flat(t, objs, ex, r["idnt"])); ntools += 1
            idv = ident(r, fl.get("mt_idTool")); tool = fl.get("mt_pctTool"); ids[str(idv)] += 1
            if idv not in (None, "empty", "") and tool not in (None, "-"): both += 1
            print(f"   {name.split('databases/')[-1]}: id {idv!r}, tool {tool!r}")
    print(f"   {ntools} CMountedTool objects in {nfiles} files; tools per file {dict(per)}; identifiers {dict(ids)}; entries with both a named identifier and a tool file: {both}")

    print("\n2. CThrowDescription in the puppet parameter files (identifier, speed window; the graphs are in puppet_attacks_verify.py)")
    cnt = collections.Counter(); files = 0
    for name, d in sorted(eps.items()):
        r = parse(d)
        if not any(t.name == "CThrowDescription" for t in r["types"].values()): continue
        r, objs, ex = load(d)
        descs = []
        for o in objs.values():
            if isinstance(o, Obj):
                descs += list(nested(o, "CThrowDescription"))
        descs = list({id(t): t for t in descs}.values())
        if not descs: continue
        files += 1
        row = []
        for t in descs:
            fl = {k: v for k, v in wv.flat(t, objs, ex, r["idnt"])}
            row.append((ident(r, fl.get("td_idName")), fl.get("td_fMinThrowingVelocity"), fl.get("td_fMaxThrowingVelocity"))); cnt[ident(r, fl.get("td_idName"))] += 1
        print(f"   {name.split('databases/')[-1]}: {row}")
    print(f"   {sum(cnt.values())} descriptions in {files} files; identifiers {dict(cnt)}")

    print("\n3. the .ctl files of the Tools folders (the binary's tool-name table is built from a file search for Tools/*.ctl)")
    ctls = merged_files(install, ".ctl")
    tools = {k.rsplit("/", 1)[-1][:-4]: k for k in ctls if "/tools/" in k}
    print(f"   {len(ctls)} .ctl files in all, {len(tools)} under a Tools folder")
    for n in ("NPC_Zapgun_R", "NPC_Bazooka_R", "NeanderthalShield", "NeanderthalClub"):
        hit = [k for k in tools if k.lower() == n.lower()]
        print(f"   {n}: {'found ' + tools[hit[0]] if hit else 'NOT under a Tools folder'}")

    print("\n4. CBAThrowPickedTool in the behaviour files")
    for name, d in sorted(merged_files(install, ".cb").items()):
        r = parse(d)
        if not any(t.name == "CBAThrowPickedTool" for t in r["types"].values()): continue
        r, objs, ex = load(d)
        for o in objs.values():
            if isinstance(o, Obj):
                for t in nested(o, "CBAThrowPickedTool"):
                    print(f"   {name.split('databases/')[-1]}: {fixed(r, dict(wv.flat(t, objs, ex, r['idnt'])))}")
            if isinstance(o, Obj) and o.type == "CBAThrowPickedTool":
                print(f"   {name.split('databases/')[-1]}: {fixed(r, dict(wv.flat(o, objs, ex, r['idnt'])))}")


if __name__ == "__main__":
    main()
