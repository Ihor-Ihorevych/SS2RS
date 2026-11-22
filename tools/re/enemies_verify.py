"""Re-runs the measurements behind docs/re/enemies.md on an install (read-only).

For every puppet parameter file (`Databases/Puppets/**/*.ep`, 126 files): the root class, health, score, enemy size,
model, the speed members of the land environment, the attacks (shooting processes with their single blasts, melee
blasts, leaps, run-over attacks, throws), the kicking and knock-out abilities and the death parameters.
Writes the per-file table with --table.

Usage: python -I tools/re/enemies_verify.py [DATA_DIR] [--table OUT.md]
       (DATA_DIR defaults to $SS2_DATA_DIR; about 1 minute)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Blob, Enum, Obj, Ref, TreeDec, merged_files, parse  # noqa: E402
import weapons_verify as W  # noqa: E402  (flat(): members with class-level prefixes)

NULL = 0xFFFFFFFF


def load(d):
    r = parse(d); objs, _ = TreeDec(d, r).objects()
    rf = {e[0]: e[2] for e in r["rfil"]}; ex = {e[0]: rf[e[1]] for e in r["exob"]}
    rs = next(o for o in objs.values() if isinstance(o, Obj) and o.type == "CResourceFile")
    return r, objs, ex, objs[[x.oid for x in rs.get("CResourceFile.rf_crsShared")][0]]


def members(o, objs, typ):
    """All objects of class `typ` reachable from `o` (members, lists, references), each once, in order."""
    out = []; seen = set()
    def go(x):
        for k, v in x:
            if k.startswith(("CResource.", "CResourceFile.")): continue
            if isinstance(v, Ref):
                t = objs.get(v.oid)
                if isinstance(t, Obj) and id(t) not in seen:
                    seen.add(id(t))
                    if t.type == typ: out.append(t)
                    go(t)
            elif isinstance(v, Obj):
                if v.type == typ: out.append(v)
                go(v)
            elif isinstance(v, list) and not isinstance(v, Blob):
                for p in v:
                    t = objs.get(p.oid) if isinstance(p, Ref) else p
                    if isinstance(t, Obj) and id(t) not in seen:
                        seen.add(id(t))
                        if t.type == typ: out.append(t)
                        go(t)
    go(o); return out


def g(o, name):
    for k, v in o:
        if k.endswith("." + name) or k == name: return v.v if isinstance(v, Enum) else v
    return None


def main():
    args = sys.argv[1:]; table = args[args.index("--table") + 1] if "--table" in args else None
    if table: args = [a for a in args if a not in ("--table", table)]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    rows = []
    for n, d in sorted(merged_files(install, ".ep").items()):
        rel = n.split("databases/")[-1]
        if not rel.startswith("puppets/"): continue
        r, objs, ex, rt = load(d); idnt = r["idnt"]
        hp = next((v for k, v in rt if k.endswith("SPuppetParams.1") and isinstance(v, Obj)), None)
        if hp is None: continue
        health = [x for _, x in hp][0]
        misc = next(v for k, v in rt if k.endswith("pp_mppMiscPuppetParams"))
        score = g(misc, "mpp_slScore"); size = next((v.v for k, v in misc if k.endswith(".8") and isinstance(v, Enum)), None)
        body = next((v for k, v in rt if k.endswith("SPuppetParams.2") and isinstance(v, Obj)), None)
        model = ""
        if body is not None:
            mv = g(body, "pbp_pmcModel"); model = ex.get(mv.oid, "").rsplit("/", 1)[-1] if isinstance(mv, Ref) else ""
        envs = [v for k, v in rt if isinstance(v, Obj) and v.type == "SPuppetEnvironmentParams"]
        env = None
        if envs:
            e = {k.split(".", 1)[-1]: v for k, v in envs[0]}; env = (e.get("3"), e.get("4"), e.get("7"))
        ma = next((v for k, v in rt if isinstance(v, Obj) and v.type == "SMovingAbilities"), None)
        movement = ma.get("SMovingAbilities.ma_maMovingAbilities") if ma is not None else None
        movement = movement.v if isinstance(movement, Enum) else movement
        shoot = []
        for sp in members(rt, objs, "CShootingProcess"):
            blasts = []
            for sb in members(sp, objs, "CSingleBlast"):
                bf = next((v for k, v in sb if k.endswith("sb_bfcBulletFireing")), None); pj = g(sb, "sb_pgppProjectileParams")
                bullet = (g(bf, "bf_ctBullets"), g(bf, "bf_slBulletDamage"), g(bf, "bf_btBulletType"), g(bf, "bf_fBulletReach")) if bf is not None else None
                blasts.append((bullet, ex.get(pj.oid, "").rsplit("/", 1)[-1] if isinstance(pj, Ref) and pj.oid in ex else ("(embedded)" if isinstance(pj, Ref) and pj.oid != NULL else "")))
            shoot.append((idnt[g(sp, "sp_idName")] if g(sp, "sp_idName") is not None else "", g(sp, "sp_fStartAttackDistance"), blasts))
        melee = [(g(m, "mb_slDamage"), g(m, "mb_fDamageRadius"), g(m, "mb_fFoeKickImpulse")) for m in members(rt, objs, "CMeleeBlast")]
        leap = [g(l, "la_slInflictedDamage") for l in members(rt, objs, "CLeapAttack")]
        runover = [(g(x, "rof_fSpeedMultiplier"), len(members(x, objs, "CMeleeBlast"))) for x in members(rt, objs, "CRunOverFoeAttack")]
        ka = next((v for k, v in rt if isinstance(v, Obj) and v.type == "SKickingAbilities"), None)
        kick = (g(ka, "ka_bIsKicker"), g(ka, "ka_slMinKickDamage"), g(ka, "ka_slMaxKickDamage")) if ka is not None else None
        ko = next((v for k, v in rt if isinstance(v, Obj) and v.type == "SKnockOutParams"), None)
        death = next((v for k, v in rt if isinstance(v, Obj) and v.type == "SPuppetDeathParams"), None)
        rows.append(dict(rel=rel, cls=rt.type, health=health, score=score, size=size, model=model, env=env, movement=movement, shoot=shoot, melee=melee, leap=leap, runover=runover, kick=kick,
                         ko=(g(ko, "kop_bCanBeInKO") if ko is not None else None), explode=(g(death, "pdp_slExplodingDamage") if death is not None else None),
                         throws=sum(len(v) for k, v in rt if k.endswith("pp_atdThrowDescs") and isinstance(v, list))))
    print("puppet files:", len(rows), dict(collections.Counter(r["rel"].split("/")[1] for r in rows)))
    print("root classes:", dict(collections.Counter(r["cls"] for r in rows)))
    print("enemy size values:", dict(collections.Counter(r["size"] for r in rows)))
    en = [r for r in rows if r["rel"].split("/")[1] in ("enemies", "bosses", "animals")]
    for key, f in (("shooting", lambda r: bool(r["shoot"])), ("melee", lambda r: bool(r["melee"])), ("leap", lambda r: bool(r["leap"])), ("run over", lambda r: bool(r["runover"])), ("throw", lambda r: r["throws"] > 0), ("kicker", lambda r: bool(r["kick"] and r["kick"][0])), ("can be knocked out", lambda r: bool(r["ko"])), ("exploding damage>0", lambda r: bool(r["explode"]))):
        print(f"  enemies, bosses, animals ({len(en)}) with {key}: {sum(1 for r in en if f(r))}")
    print("health (enemies/bosses/animals): min", min(r["health"] for r in en), "median", sorted(r["health"] for r in en)[len(en) // 2], "max", max(r["health"] for r in en))
    bd = collections.Counter(); proj = collections.Counter()
    for r in rows:
        for sp in r["shoot"]:
            for b, p in sp[2]:
                if b and b[0] and b[1]: bd[b[1]] += 1
                if p: proj[p] += 1
    print("bullet damage values:", dict(sorted(bd.items())), "| projectile files used by shooting blasts:", len(proj), dict(proj.most_common(10)))
    md = collections.Counter(m[0] for r in rows for m in r["melee"]); print("melee blast damage values:", dict(sorted(md.items())))
    # the projectile files of the shooting blasts
    allp = {}
    for n2, d2 in merged_files(install, ".ep").items(): allp.setdefault(n2.rsplit("/", 1)[-1], d2)
    pr = {}
    for p_ in sorted(proj):
        if p_.lower() not in allp: continue
        r2, o2, e2, t2 = load(allp[p_.lower()]); fl = dict(W.flat(t2, o2, e2, r2["idnt"]))
        pr[p_] = (t2.type, fl, proj[p_], sorted({rr["rel"].rsplit("/", 1)[-1][:-3] for rr in rows for sp in rr["shoot"] for b, pp in sp[2] if pp == p_}))
    print("enemy projectile files:", len(pr), "of", len(proj), "| direct damage p.2 values:", dict(sorted(collections.Counter(v[1].get("p.2") for v in pr.values()).items(), key=lambda kv: str(kv[0]))))
    if table:
        def att(r):
            parts = []
            for nm, dist, bl in r["shoot"]:
                bs = []
                for b, p in bl:
                    bs.append((f"{b[0]}×{b[1]}" if b and b[0] and b[1] else "") + (f" `{p}`" if p else ""))
                parts.append(f"shoot `{nm}`" + (f" from {dist:g}" if dist and dist > 0 else "") + (": " + ", ".join(x.strip() for x in bs if x.strip()) if any(x.strip() for x in bs) else ""))
            if r["melee"]: parts.append("melee " + ", ".join(f"{m[0]:g} r{m[1]:g}" for m in r["melee"]))
            if r["leap"]: parts.append("leap " + ", ".join(str(x) for x in r["leap"]))
            if r["runover"]: parts.append("run over ×" + ", ".join(f"{x[0]:g}" for x in r["runover"]))
            if r["throws"]: parts.append(f"throws {r['throws']}")
            return "; ".join(parts) or "-"
        L = ["# Puppet parameters per character (generated)", "",
             "Generated by `tools/re/enemies_verify.py --table` from the 126 puppet parameter files of the retail install. See `enemies.md`. "
             "Columns: root class, health, score, enemy size (`CMiscPupeptParams.8`, a number), the three speed members of the land environment (`SPuppetEnvironmentParams.3`, `.4`, `.7`), "
             "the attacks (shoot: process name, start distance, per blast bullets × damage and projectile file; melee: damage and radius per blast; leap: damage; run over: speed multiplier), "
             "kick damage (min–max, for kickers) and exploding damage.", ""]
        by = collections.defaultdict(list)
        for r in rows: by[r["rel"].split("/")[1]].append(r)
        for grp, rs in sorted(by.items()):
            L += [f"## {grp} ({len(rs)})", "", "| File | Class | Health | Score | Size | Speeds | Attacks | Kick | Expl. |", "|---|---|---|---|---|---|---|---|---|"]
            for r in sorted(rs, key=lambda x: x["rel"]):
                sp = "/".join(f"{x:g}" for x in r["env"]) if r["env"] and all(isinstance(x, (int, float)) for x in r["env"]) else ""
                kk = f"{r['kick'][1]}–{r['kick'][2]}" if r["kick"] and r["kick"][0] else ""
                L.append(f"| `{r['rel'].split('/', 2)[-1]}` | `{r['cls'][1:].replace('PuppetParams', '')}` | {r['health']:g} | {r['score']} | {r['size']} | {sp} | {att(r)} | {kk} | {r['explode'] or ''} |")
            L.append("")
        L += ["# Projectile files of the shooting blasts", "", "Direct damage `p.2` and range damage `p.3` with the second set `p.11` and `p.12` (as in `weapons.md`), the damage radii `p.7` and `p.8`, the speed `p.1`, the number of blasts that use the file and the characters that fire it.", "",
              "| File | Class | Speed | Direct | Range | Radii | Blasts | Fired by |", "|---|---|---|---|---|---|---|---|"]
        for p_, (cls, fl, cnt, who) in sorted(pr.items()):
            L.append(f"| `{p_}` | `{cls[1:].replace('ProjectileParams', '')}` | {fl.get('p.1', '')} | {fl.get('p.2', '')} / {fl.get('p.11', '')} | {fl.get('p.3', '')} / {fl.get('p.12', '')} | {fl.get('p.7', '')}, {fl.get('p.8', '')} | {cnt} | {', '.join(who)[:80]} |")
        with open(table, "w", encoding="utf-8", newline="\n") as f: f.write("\n".join(L) + "\n")
        print("wrote", table)


if __name__ == "__main__":
    main()
