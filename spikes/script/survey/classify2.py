import os
import json, os, glob, re, collections
res = json.load(open("schemas.json"))
docs = ""
for f in glob.glob(os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../docs/re/*.md")):
    docs += open(f, encoding="utf-8", errors="replace").read()+"\n"
base_types=set()
for s,k,r in res:
    if s=="base" and "types" in r: base_types.update(r["types"])
print("base .dtn files:", sum(1 for s,k,r in res if s=="base" and k.endswith(".dtn")))
for mod in ("renov","insam"):
    new=collections.defaultdict(set)
    for s,k,r in res:
        if s!=mod or "types" not in r: continue
        for n in r["types"]:
            if n not in base_types: new[n].add(os.path.splitext(k)[1])
    indoc=[n for n in new if re.search(r"\b"+re.escape(n)+r"\b", docs)]
    notdoc=[n for n in new if n not in indoc]
    print(mod, "new types", len(new), "named in docs/re (i.e. in the base game's class inventory or data specs):", len(indoc), "not named:", len(notdoc))
    print("  NOT in docs/re:", sorted(notdoc))
    print("  in docs/re:", sorted(indoc))
