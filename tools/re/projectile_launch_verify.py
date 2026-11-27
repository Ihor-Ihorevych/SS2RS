"""Evidence for docs/re/projectile-launch.md (T01-119): what the data says about launching and charging projectiles.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries).

It prints
  1. for the seven projectile weapons (rocket launcher, grenade launcher, plasma rifle, Klodovik gun, cannon, zap gun, flamer), single
     player and deathmatch: the projectile file, `bwp_fChargingTime`, `bwp_fChargingAutoFireTime`, the minimum and maximum recoil
     duration, the two launch velocities, `bwp_ctFiresToReload`, and for the zap gun `zgwp_ctMaxFirePowerWorthBullets`;
  2. the numbers of every projectile file a weapon names that matter to the launch: flying speed, life time, and the (fire power 0,
     fire power 1) pairs of direct damage, area damage, hot spot, fall-off and stretch, with the value a launch of the weapon's real
     fire power gives (rule of the spec: the weapon passes fire power 0, the charge, or the zap gun's discrete 0 or 1);
  3. the launch speed at fire power 0, 0.25, 0.5, 0.75 and 1 of the three weapons that scale it, and the zap gun's fired-bullet count
     after a shot of fire power 0 and 1 (the formula of the spec);
  4. the hand grenade of the player puppet (`ppp_pgppHandGrenade`) with its throw numbers and projectile file;
  5. which of the 94 projectile files are named by a player weapon, and how many of the 36 files with a maximum of 0 next to a positive
     minimum are among them (the answer to "why does the maximum look wrong").

Usage: python -I tools/re/projectile_launch_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR; about 1 minute)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import merged_files, parse  # noqa: E402
import weapons_verify as wv  # noqa: E402

LAUNCH = ["rocketlauncher", "grenadelauncher", "plasmarifle", "klodovikgun", "cannon", "zapgun", "flamer"]
# the fire power each weapon passes to the projectile (spec section 3): 0, the charge, or the zap gun's discrete 0 or 1
PASSES = {"rocketlauncher": "0", "grenadelauncher": "0", "plasmarifle": "0", "klodovikgun": "0", "flamer": "0", "cannon": "charge",
          "zapgun": "discrete"}
# data number -> binary name of SBaseProjectileParams (projectile-classes.md section 4)
PAIRS = [("direct damage", "p.2", "p.11"), ("area damage", "p.3", "p.12"), ("hot spot", "p.7", "p.13"), ("fall-off", "p.8", "p.14"),
         ("stretch", "p.10", "p.20")]


def lerp(a, b, t):
    return a + (b - a) * t


def num(v, default=0.0):
    return v if isinstance(v, (int, float)) else default


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    eps = merged_files(install, ".ep"); by = {n.split("databases/")[-1]: d for n, d in eps.items()}
    weapons = {}
    for rel in sorted(by):
        if not rel.startswith("entityparams/") or "items/" in rel or "rendering" in rel or "copy" in rel: continue
        if not (rel.endswith("weapon.ep") or rel.endswith("weapon_dm.ep")): continue
        r, objs, ex, rt = wv.load(by[rel]); fl = wv.flat(rt, objs, ex, r["idnt"])
        key = rel.rsplit("/", 1)[-1].replace("weapon_dm.ep", "").replace("weapon.ep", ""); dm = rel.endswith("_dm.ep")
        weapons.setdefault(key, {})["dm" if dm else "sp"] = (rt.type, dict(fl))

    projs = {}
    for rel in sorted(by):
        if "genericprojectiles/" not in rel: continue
        r, objs, ex, rt = wv.load(by[rel]); projs[rel.rsplit("/", 1)[-1]] = dict(wv.flat(rt, objs, ex, r["idnt"]))

    print("1. the seven projectile weapons (b.N = SBaseWeaponParams member N; seconds, metres per second)")
    print("   columns: projectile file | charging time b.13 | auto fire time b.33 | recoil min b.30 | recoil max b.31 | launch min b.15 | "
          "launch max b.16 | fires to reload b.5 | own members")
    for key in LAUNCH:
        for v in ("sp", "dm"):
            w = weapons.get(key, {}).get(v)
            if not w: continue
            d = w[1]
            own = {k: val for k, val in d.items() if k.startswith("w.")}
            print(f"   {key:16s} {v}: {d.get('b.23')} | {d.get('b.13')} | {d.get('b.33')} | {d.get('b.30')} | {d.get('b.31')} | {d.get('b.15')} | "
                  f"{d.get('b.16')} | {d.get('b.5')} | {own}")

    print("\n2. the projectile files the seven weapons name: speed, life time, and the pairs (value at fire power 0, at 1) and the value at the passed power")
    seen = set()
    for key in LAUNCH:
        for v in ("sp", "dm"):
            w = weapons.get(key, {}).get(v)
            if not w: continue
            f = w[1].get("b.23")
            if not isinstance(f, str) or f == "-" or (key, v, f) in seen: continue
            seen.add((key, v, f)); p = projs.get(f.lower())
            if p is None: print(f"   {key} {v}: {f} not found"); continue
            print(f"   {key} {v}: {f}: flying speed {p.get('p.1')}, life time {p.get('p.5')}, flags {p.get('p.100')}, adds shooter velocity {p.get('bp_bAddShooterVelocity')}, nature {p.get('gpp_pnProjectileNature')}")
            for name, lo, hi in PAIRS:
                a, b = num(p.get(lo)), num(p.get(hi))
                note = ""
                if PASSES[key] == "0": note = f" -> the weapon passes 0: {a:g} always"
                elif PASSES[key] == "charge": note = f" -> at charge 0 / 0.5 / 1: {lerp(a, b, 0):g} / {lerp(a, b, .5):g} / {lerp(a, b, 1):g}"
                else: note = f" -> zap gun passes 0 or 1: {a:g} or {b:g}"
                print(f"      {name:14s} min {a:g}  max {b:g}{note}")

    print("\n3. launch speed lerp(min, max, fire power) for the three weapons that scale it; the zap gun's bullet count")
    for key in ("grenadelauncher", "cannon", "zapgun"):
        for v in ("sp", "dm"):
            w = weapons.get(key, {}).get(v)
            if not w: continue
            lo, hi = num(w[1].get("b.15")), num(w[1].get("b.16"))
            print(f"   {key:16s} {v}: " + ", ".join(f"p={t:g}: {lerp(lo, hi, t):g}" for t in (0, .25, .5, .75, 1)))
    zg = weapons.get("zapgun", {})
    for v in ("sp", "dm"):
        if v not in zg: continue
        worth = zg[v][1].get("w.1")
        if worth is None: continue
        for t in (0, 1):
            count = 0
            for _ in range(3):
                count = int((worth - 1) * t + count + 1)
            print(f"   zapgun {v}: ctMaxFirePowerWorthBullets {worth}; three shots of fire power {t}: fired-bullet count after each shot "
                  f"{[int((worth - 1) * t * 0 + (worth - 1) * t * 0) or 0][0]}", end="")
            c = 0; seq = []
            for _ in range(3):
                c = int((worth - 1) * t + c + 1); seq.append(c)
            print(" ->", seq)

    print("\n4. the hand grenade of the player puppet (SPlayerPuppetParams)")
    for rel in sorted(by):
        r = parse(by[rel])
        if not any(t.name == "SPlayerPuppetParams" for t in r["types"].values()): continue
        r, objs, ex, rt = wv.load(by[rel]); d = dict(wv.flat(rt, objs, ex, r["idnt"]))
        gp = d.get("ppp_pgppHandGrenade")
        print(f"   {rel}: hand grenade file {gp}; w.1..w.5 = {[d.get('w.%d' % i) for i in range(1, 6)]}; added pitch {d.get('ppp_fAddedPitchDeg')}")
        if isinstance(gp, str) and gp != "-" and gp.lower() in projs:
            p = projs[gp.lower()]
            print(f"      {gp}: speed {p.get('p.1')}, life {p.get('p.5')}, direct {p.get('p.2')}/{p.get('p.11')}, area {p.get('p.3')}/{p.get('p.12')}, "
                  f"hot spot {p.get('p.7')}/{p.get('p.13')}, fall-off {p.get('p.8')}/{p.get('p.14')}")

    print("\n5. the 94 projectile files and the player weapons")
    named = collections.Counter()
    for key, w in weapons.items():
        for v in ("sp", "dm"):
            f = w.get(v, ("", {}))[1].get("b.23")
            if isinstance(f, str) and f != "-": named[f.lower()] += 1
    zero_max = {n for n, p in projs.items() if any(num(p.get(lo)) > 0 and num(p.get(hi)) == 0 for _, lo, hi in PAIRS[:4])}
    print(f"   projectile files: {len(projs)}; named by a player weapon file: {len(named)} ({sorted(named)})")
    print(f"   files with a maximum of 0 next to a positive minimum (direct, area, hot spot or fall-off): {len(zero_max)}; "
          f"of them named by a player weapon: {sorted(zero_max & set(named))}")
    for n in sorted(named):
        p = projs.get(n)
        if p: print(f"      {n}: " + ", ".join(f"{name} {num(p.get(lo)):g}/{num(p.get(hi)):g}" for name, lo, hi in PAIRS))


if __name__ == "__main__":
    main()
