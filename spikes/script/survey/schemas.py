import os
import zipfile, glob, os, json, subprocess, sys, tempfile, concurrent.futures as cf, collections
root = os.environ["SS2_DATA_DIR"]
M = os.environ["META_DUMP"]  # target/debug/meta-dump(.exe) built from tools/meta-dump
EXTS = {".ep",".rsc",".ctl",".cb",".mac",".sfc",".pss",".mch",".sca",".mas",".dtn",".mat",".mtr",".skl",".tfx",".scs",".mah",".bwh",".wld",".pfx",".scp",".crt"}
def entries(gros):
    d={}
    for g in gros:
        with zipfile.ZipFile(g) as z:
            for i in z.infolist():
                if i.filename.endswith("/"): continue
                k=i.filename.lower().replace("\\","/")
                d[k]=(g,i.filename)
    return d
base = entries(sorted(glob.glob(root+"/All_PC_0*.gro"))+sorted(glob.glob(root+"/Patch_*.gro")))
mods = {
 "renov": entries([root+"/Content/Sam2Renovation/Sam2Renovation.gro"]),
 "insam": entries([root+"/Content/InSamnity2/InSamnity2.gro"]+sorted(glob.glob(root+"/Content/InSamnity2/z_*.gro"))),
}
tmpd = os.path.join(os.environ.get("SURVEY_OUT", "."), "scratch"); os.makedirs(tmpd, exist_ok=True)
def run(job):
    src, key, (g, fn) = job
    ext = os.path.splitext(key)[1]
    tp = os.path.join(tmpd, f"{abs(hash((src,key)))}{ext}")
    try:
        with zipfile.ZipFile(g) as z:
            data = z.read(fn)
        with open(tp,"wb") as f: f.write(data)
        r = subprocess.run([M,"--format","json","--skip-values",tp],capture_output=True)
        try: d=json.loads(r.stdout)
        except Exception: return (src,key,{"status":"unparsed","rc":r.returncode,"size":len(data)})
        types = {t["name"]:[ (m["name"],m.get("type_name")) for m in (t.get("members") or [])] for t in d.get("types",[]) if t.get("form")=="struct"}
        objs = collections.Counter(o["type_name"] for o in d.get("objects",[]))
        refs = len(d.get("references",[]))
        return (src,key,{"status":d.get("status","?"),"keys":len(d),"size":len(data),"types":types,"objs":dict(objs),"refs":refs})
    finally:
        try: os.remove(tp)
        except OSError: pass
jobs=[]
for k,v in base.items():
    if os.path.splitext(k)[1] in EXTS: jobs.append(("base",k,v))
for m,e in mods.items():
    for k,v in e.items():
        if os.path.splitext(k)[1] in EXTS: jobs.append((m,k,v))
print(len(jobs),"jobs",flush=True)
res=[]
with cf.ThreadPoolExecutor(8) as ex:
    for n,r in enumerate(ex.map(run,jobs)):
        res.append(r)
        if n%500==0: print(n,flush=True)
json.dump(res,open("schemas.json","w"))
print("done")
