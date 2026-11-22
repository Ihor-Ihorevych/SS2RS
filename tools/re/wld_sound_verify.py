"""Re-runs the measurements behind docs/re/wld-sound-effects.md on an install (read-only).

For every format-8 `.wld`: the sound sources and channels (what owns them, which sound they play, flags and
ranges), the sync list, feeders, environment boxes, the dynamic music, and the effect instances (entities,
renderables, the `.pfx` they run, and how many particle effect instances each holds compared with the `.pfx`).

Usage: python -I tools/re/wld_sound_verify.py [DATA_DIR] [LIMIT]
       (DATA_DIR defaults to $SS2_DATA_DIR; LIMIT = only the LIMIT smallest worlds, default all, about 15 minutes)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Blob, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
from wld_root_verify import ids  # noqa: E402

NULL = 0xFFFFFFFF
KINDS = {"CMachineParticleEffect": "CMachineParticleEffectInstance", "CCloudsParticleEffect": "CCloudsParticleEffectInstance", "CAtomicParticleEffect": "CAtomicParticleEffectInstance"}


def load_pfx(install):
    out = {}
    for name, d in merged_files(install, ".pfx").items():
        r = parse(d); objs, _ = TreeDec(d, r).objects()
        e = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CEffect")
        out[name.lower()] = ([objs[x.oid].type for x in e.get("CEffect.5") if isinstance(x, Ref)], e.get("CEffect.12").v)
    return out


def refs_in(o):
    for k, v in o:
        if isinstance(v, Ref): yield k, v
        elif isinstance(v, Obj):
            for kk, x in refs_in(v): yield kk, x
        elif isinstance(v, list) and not isinstance(v, Blob):
            for x in v:
                if isinstance(x, Ref): yield k, x
                elif isinstance(x, Obj):
                    for kk, y in refs_in(x): yield kk, y


def check_world(name, d, pfx, st, dist):
    r = parse(d); dec = TreeDec(d, r); objs, edob = dec.objects()
    try: extra, _ = dec.extra_objects(edob)
    except Exception: extra = {}
    allo = {**objs, **extra}; rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}; idnt = r["idnt"]
    w = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CWorld")
    ents = set(ids(w.get("CWorld.1")))
    of = lambda t: {i: o for i, o in objs.items() if isinstance(o, Obj) and o.type == t}
    sources, channels = of("CSoundSource"), of("CSoundChannel")
    st["worlds"] += 1; st["sound sources"] += len(sources); st["sound channels"] += len(channels)
    st["sources whose CSoundSource.1 is a channel of the world"] += sum(1 for s in sources.values() if s.get("CSoundSource.1").oid in channels)
    used = collections.Counter(s.get("CSoundSource.1").oid for s in sources.values())
    st["channels used by exactly one source"] += sum(1 for c in channels if used[c] == 1)
    # who refers to a source (any member of an entity)
    owners = collections.defaultdict(list)
    for i in ents:
        e = objs[i]
        for k, v in refs_in(e):
            if v.oid in sources: owners[v.oid].append((e.type, k))
    for sid, lst in owners.items():
        for (t, k) in lst: dist["entity member that holds a sound source"][(t, k)] += 1
    st["sources held by an entity member"] += len(owners); st["sources held by no entity"] += len(sources) - len(owners)
    st["sources held by two or more entity members"] += sum(1 for l in owners.values() if len(l) > 1)
    for sid, s in sources.items():
        o = s.get("CAspect.5").oid
        dist["source owner (CAspect.5) class"][objs[o].type if o in objs and isinstance(objs[o], Obj) else ("(none)" if o == NULL else "?")] += 1
    # channels
    for cid, c in channels.items():
        snd = c.get("CSoundChannel.3"); fd = c.get("CSoundChannel.2"); sy = c.get("CSoundChannel.4")
        fl = c.get("CSoundChannel.1").v
        dist["SndFlags"][hex(fl)] += 1
        if snd.oid != NULL:
            st["channels with a sound (.3)"] += 1
            dist["sound target"][("external ." + ex[snd.oid].rsplit(".", 1)[-1].lower()) if snd.oid in ex else (allo[snd.oid].type if snd.oid in allo and isinstance(allo[snd.oid], Obj) else "?")] += 1
            dist["with a sound: volume"][round(c.get("CSoundChannel.10"), 2)] += 1; dist["with a sound: hot spot"][round(c.get("CSoundChannel.20"), 1)] += 1; dist["with a sound: fall-off"][round(c.get("CSoundChannel.21"), 1)] += 1
            dist["with a sound: cone angles (.33, .34)"][(round(c.get("CSoundChannel.33")), round(c.get("CSoundChannel.34")))] += 1
            dist["with a sound: .22"][c.get("CSoundChannel.22")] += 1; dist["with a sound: SndFlags"][hex(fl)] += 1
        else: dist["without a sound: SndFlags"][hex(fl)] += 1
        if fd.oid != NULL: st["channels with a feeder (.2)"] += 1
        st["channels with a sync list (.4)"] += sy.oid != NULL
    st["channels with a sound that belong to a source held by an entity"] += sum(1 for sid, s in sources.items() if sid in owners and channels[s.get("CSoundSource.1").oid].get("CSoundChannel.3").oid != NULL)
    # sync list, feeders, environments, music
    for sy in of("CSoundSyncList").values():
        dist["sync list list lengths (.3, .4, .7), bucketed"][tuple(min(len(sy.get(f"CSoundSyncList.{k}")), 9) if len(sy.get(f"CSoundSyncList.{k}")) < 10 else "10+" for k in (3, 4, 7))] += 1
    st["sync lists"] += len(of("CSoundSyncList")); st["feeders (CStaticSoundFeeder)"] += len(of("CStaticSoundFeeder"))
    withs = {i for i, c in channels.items() if c.get("CSoundChannel.3").oid != NULL}; envs = set(of("CSoundEnvironment")); feeders = set(of("CStaticSoundFeeder"))
    for sy in of("CSoundSyncList").values():
        l3 = set(ids(sy.get("CSoundSyncList.3"))); l7 = set(ids(sy.get("CSoundSyncList.7")))
        st["sync list .3 (channels) == the channels that have a sound"] += l3 == withs
        st["sync list .7 == the sound environments of the world"] += l7 == envs
        st["sync list .4 (listeners) is empty"] += len(sy.get("CSoundSyncList.4")) == 0
    st["channels with a feeder have a sound, and the feeder is a CStaticSoundFeeder of the world"] += sum(1 for c in channels.values() if c.get("CSoundChannel.2").oid in feeders and c.get("CSoundChannel.3").oid != NULL)
    st["channels with a sound have a feeder"] += sum(1 for i in withs if channels[i].get("CSoundChannel.2").oid != NULL)
    st["sound environments"] += len(of("CSoundEnvironment"))
    for e in of("CSoundEnvironment").values():
        for k, v in e:
            if k.endswith(".2") and hasattr(v, "v"): dist["environment preset (SfxEnvironment)"][v.v] += 1
    st["dynamic music"] += len(of("CDynamicMusic")); st["dynamic music sources"] += len(of("CDynamicMusicSource"))
    for m in of("CDynamicMusic").values():
        t = m.get("CDynamicMusic.dm_pbsMusic")
        if isinstance(t, Ref) and t.oid in ex: dist["music track (external)"][ex[t.oid].rsplit("/", 1)[-1].lower()] += 1
    # effects
    eents = [i for i in ents if objs[i].type == "CEffectEntity"]; erend = of("CEffectRenderable")
    st["effect entities"] += len(eents); st["effect renderables"] += len(erend)
    for ei in eents:
        e = objs[ei]; rr = e.get("CEffectEntity.4").oid; ss = e.get("CEffectEntity.en_hss").oid
        st["effect entities whose CEffectEntity.4 is an effect renderable that names the entity (CAspect.5)"] += rr in erend and erend[rr].get("CAspect.5").oid == ei
        st["effect entities with a sound source (en_hss)"] += ss in sources
        dist["effect entity: EffectLifeType (.6)"][e.get("CEffectEntity.6").v] += 1
        dist["effect entity: en_pesEffectState"][e.get("CEffectEntity.en_pesEffectState").v] += 1
        dist["effect entity: (en_bAutoStart, en_bEmitting)"][(e.get("CEffectEntity.en_bAutoStart"), e.get("CEffectEntity.en_bEmitting"))] += 1
        dist["effect entity: has own sound (en_pbsSound)"][e.get("CEffectEntity.en_pbsSound").oid != NULL] += 1
    for ri, rd in erend.items():
        own = rd.get("CAspect.5").oid
        dist["effect renderable owner class"][objs[own].type if own in objs else "(none)"] += 1
        inst = rd.get("CEffectRenderable.1"); eff = inst.get("CEffectInstance.7"); lst = [x for x in inst.get("CEffectInstance.6")]
        st["effect instances"] += 1
        path = ex.get(eff.oid, "").lower()
        if not path: st["effect instances without an external effect"] += 1; continue
        st["effect instances naming an external .pfx"] += path.endswith(".pfx")
        if path in pfx:
            kinds, life = pfx[path]
            st["instances: number of particle effect instances == number of particle effects of the .pfx"] += len(lst) == len(kinds)
            got = [objs[x.oid].type for x in lst if isinstance(x, Ref) and x.oid in objs]
            st["instances: instance class matches the particle effect class (Machine, Clouds, Atomic), in order"] += got == [KINDS[k] for k in kinds]
            dist["distinct .pfx used (count of instances)"]["n"] += 0
            dist["effect entity EffectLifeType equals the .pfx CEffect.12"][("entity" if own in eents else "other owner", None)] += 0
    for ei in eents:
        e = objs[ei]; rr = erend.get(e.get("CEffectEntity.4").oid)
        if rr is None: continue
        eff = rr.get("CEffectRenderable.1").get("CEffectInstance.7"); path = ex.get(eff.oid, "").lower()
        if path in pfx: st["effect entities: EffectLifeType equals the .pfx CEffect.12"] += e.get("CEffectEntity.6").v == pfx[path][1]; st["... entities compared"] += 1
        dist["pfx used by effect entities (top)"][path.rsplit("/", 1)[-1]] += 1


def main():
    install = sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].isdigit() else os.environ.get("SS2_DATA_DIR", "")
    limit = int(sys.argv[-1]) if sys.argv[-1].isdigit() else None
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    pfx = load_pfx(install); st = collections.Counter(); dist = collections.defaultdict(collections.Counter)
    files = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))
    for n, (name, d) in enumerate(files):
        if limit and n >= limit: break
        check_world(name, d, pfx, st, dist); print(f"[{n + 1}/{len(files)}] {name.rsplit('/', 1)[-1]}", file=sys.stderr, flush=True)
    for k, v in sorted(st.items()):
        if v: print(f"{v:9d}  {k}")
    for k, v in sorted(dist.items()):
        v = collections.Counter({a: b for a, b in v.items() if b})
        if v: print(k, dict(v.most_common(14)))


if __name__ == "__main__":
    main()
