import os
import json, os, collections
res=json.load(open("schemas.json"))
base={}
for s,k,r in res:
    if s=="base" and "types" in r:
        for n,m in r["types"].items(): base.setdefault(n,set()).add(tuple(map(tuple,m)))
chg=collections.defaultdict(lambda:[0,collections.Counter(),collections.Counter()])
for s,k,r in res:
    if s!="insam" or "types" not in r: continue
    for n,m in r["types"].items():
        if n in base and tuple(map(tuple,m)) not in base[n]:
            mm=set(map(tuple,m))
            best=min(base[n], key=lambda b: len(mm^set(b)))
            add=mm-set(best); rem=set(best)-mm
            c=chg[n]; c[0]+=1
            for a in add: c[1][a]+=1
            for a in rem: c[2][a]+=1
tot_add=0
for n,(c,a,rm) in sorted(chg.items(), key=lambda x:-x[1][0])[:14]:
    print(n,"files",c,"added members:",len(a),"removed:",len(rm), list(a.items())[:4])
print("types changed",len(chg))
