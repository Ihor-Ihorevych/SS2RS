import os
import json, os, glob, zipfile, collections, re
root = os.environ["SS2_DATA_DIR"]
res = json.load(open("schemas.json"))
base_types = {}
for s,k,r in res:
    if s=="base" and "types" in r:
        for n,m in r["types"].items(): base_types.setdefault(n,set()).add(tuple(map(tuple,m)))
def entries(gros):
    d={}
    for g in gros:
        with zipfile.ZipFile(g) as z:
            for i in z.infolist():
                if not i.filename.endswith("/"): d[i.filename.lower().replace("\\","/")]=i.file_size
    return d
mods={"Sam2Renovation":entries([root+"/Content/Sam2Renovation/Sam2Renovation.gro"]),
      "InSamnity2":entries([root+"/Content/InSamnity2/InSamnity2.gro"]+sorted(glob.glob(root+"/Content/InSamnity2/z_*.gro")))}
tag={"Sam2Renovation":"renov","InSamnity2":"insam"}
ASSET={".mdl",".tex",".tga",".bmf",".wav",".ogg",".ogv",".fnt",".ans",".skl",".mtr",".mat",".pfx",".tfx",".crt",".dsc",".nfo",".mod",".txt",".ini",".dat"}
DATA={".ep",".rsc",".ctl",".cb",".mch",".dtn",".sfc",".pss",".sca",".mas",".scs",".scp",".bwh",".mah",".wld"}
out={}
for name,ents in mods.items():
    t=tag[name]
    schema={k:r for s,k,r in res if s==t and "types" in r}
    cnt=collections.Counter(); bytes_=collections.Counter(); ex=collections.defaultdict(collections.Counter)
    for k,size in ents.items():
        e=os.path.splitext(k)[1]
        if e in ASSET: c="A assets"
        elif e==".mac": c="C macro program"
        elif e in DATA:
            r=schema.get(k)
            if r is None: c="B? data (schema not read)"
            else:
                newt=[n for n in r["types"] if n not in base_types]
                chg=[n for n,m in r["types"].items() if n in base_types and tuple(map(tuple,m)) not in base_types[n]]
                c="B2 data, new class" if newt else ("B2 data, changed class" if chg else "B1 data, base schemas only")
        else: c="other "+e
        cnt[c]+=1; bytes_[c]+=size; ex[c][e]+=1
    out[name]=(cnt,bytes_,ex)
    print("==",name,"entries",sum(cnt.values()))
    for c in sorted(cnt): print("  %-30s %5d  %12d bytes  %s"%(c,cnt[c],bytes_[c],dict(ex[c].most_common(6))))
