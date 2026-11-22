# Hit effects, blood, gibs and decals: what spawns when (T01-043)

- **Roadmap task:** T01-043 (feeds the effects and gameplay tasks; the renderer part is T04)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for what the data stores: which effect file a surface plays for which event, the puppet death parameters, the prop and placed-effect usage, and **the absence of decals and gib objects** (all 35 materials, 126 puppet files, 94 projectile blocks and all 59 worlds, `tools/re/effects_verify.py`); **medium** for which gameplay moment raises which event (the event names and the entity that owns them are in the data, the triggers are in code); **low** for lifetimes, limits and blending. Scope: the 10,933 files of the install, 604 `.pfx` effect files.

Prerequisites: `mat.md` (physical materials, schemes), `pfx.md` (particle effects), `destruction.md` (debris and effects of breakable models), `ep.md` and `puppet-stats.md` (puppets), `wld-sound-effects.md` and `wld-entities.md` (effect entities), `audio.md` (sound schemes).

## Summary

- **There are no decals.** No file of the install is a decal, and `classes-catalog.md` lists no decal class; no file is a bullet hole, scorch, crater or blood splat texture: the only archive names matching `decal`, `gib`, `splat`, `bullethole`, `scorch` or `crater` are six files: three `splatNN.wav` sounds and `chickengrindersplat.wav`, a level object model `crater.mdl` and its mask texture. **A surface hit leaves nothing behind except what a particle effect draws while it lives.**
- **Blood is a particle effect on a material.** Flesh surfaces have their own effect files (`Flesh_Bullet.pfx`, `DeadFlesh_Bullet.pfx`, `Flesh_Saw.pfx`, `Flesh_Beam.pfx`, `FleshMechanism_Bullet.pfx`, `FleshMetal_Bullet.pfx`); the effect played for a hit depends only on the **physical material of the surface that was hit** and the **kind of hit**.
- **Gibs are debris models.** There is no gib class: bodies and objects that break spawn **debris** (`CDFXDebris`, 2,140 effects, models such as `Flesh_02.mdl`, with particle trails) and explosion effects from the model's destruction configuration (`destruction.md`); a puppet's death parameters decide whether the corpse explodes and with what impulse.
- **Where effects come from:** (1) the physical material of the hit surface (`CParticleScheme`: event → `.pfx`); (2) the model's destruction configuration; (3) the puppet's death and effect parameters; (4) props (one splash and one dust effect); (5) effect entities placed in the world (2,352).

## Evidence

`tools/re/effects_verify.py` (read-only, `SS2_DATA_DIR`; about 15 minutes) over the retail install with all patches; the per-material table is `effects-tables.md` (generated):

- **Names:** 10,933 archive files, 604 `.pfx` (77 in `Effects/Materials`); the decal-like names are the six listed above.
- **Materials:** 35 `.mat` files; 135 particle scheme entries (34 materials have at least one; `playerflesh` has none); **every entry has exactly one alternative** (weight 1) and **all 135 effect files exist**; 74 distinct effect files.
- **Puppets:** 126 `SPuppetDeathParams` blocks; 94 projectile parameter blocks; **2,352 effect entities** in 57 of 59 worlds with 117 distinct effect files (58 have no effect pointer in the record); **1,726 props**, all with the same two effect pointers.

## Surface hits: the material particle scheme

A physical material (`.mat`, `mat.md`) owns a **particle scheme** (`sm_ppsParticleScheme`, class `CParticleScheme`): a list of events, each with a list of weighted `.pfx` alternatives (`CPFXInScheme.1` the event name in the file's `IDNT` table, `.2` the `CWeightedPFXInScheme` list: weight, effect, a third member 0). Its second member (a base scheme) is **null in all 35**. The sound scheme (`sm_pssSoundScheme`) has the same event names (`mat.md`). Events in the 35 particle schemes:

| Event | Materials | What it is (reading) | Effect names |
|---|---|---|---|
| `BulletHit` | 33 | a bullet or hitscan shot strikes the surface | `<Kind>_Bullet.pfx` (`Metal`, `Stone`, `Wood`, `Flesh`, …) |
| `ChainSaw` | 33 | the circular saw (chain saw) weapon touches the surface | `<Kind>_Saw.pfx` |
| `BeamHit` | 30 | the beam weapon hits the surface | `Stone_Beam.pfx` (most), `Flesh_Beam.pfx` (flesh, bones, dead flesh, mechanical flesh) |
| `Step` | 16 | a character steps on the surface | `<Kind>_Step.pfx` |
| `Thrust` | 12 | a jump, a landing or a thrust off the surface (probably) | `<Kind>_Thrust.pfx` |
| `Dust` | 11 | dust kicked up by a projectile (`bp_idEffectType` = `Dust` in 3 projectile blocks; 91 are empty) | `<Kind>_Dust.pfx` |

- Several materials **share** effect names (all metals use `Metal_*`, all woods `Wood_*`, both stones `Stone_*`): 135 entries use 74 files. Some materials have fewer events: `energy`, `magic`, `fire` have only bullet and saw; `shallowwater` only a step effect; `flower` plays `Pumpkin_Explode.pfx` for five of its six events (everything but the beam; the pumpkin splatter); `playerflesh` has no particle events (a player hit shows nothing here).
- The **surface a hit lands on** is the physical material of the visual material of the mesh surface (`mat.md`, `mtr.md`); the sound scheme named by the material picks the sound for the same event.

## Death, corpses and gibbing: `SPuppetDeathParams`

126 files (one block each): `pdp_slExplodingDamage` **0 in 103**, 80 to 200 in 23 (100: 7, 120: 6, 140: 3); `pdp_fExplosionForceImpulse` 0 (74), **2,000 (33)**, 1,500 (14), 1,000 (5) with an up ratio 0.5 in 59 files; `pdp_bDisableBulletDamageAccumulation` 1 in 7; `.1` BOOL (1 in 114); `.2`, `.3`, `.4` (25 / 10 / 5 in the common case: 27 / 42 / 43 files); `pdp_fStretchOverride` (−1 in 119, 1 to 3.5 in 7). Reading: a puppet with a non-zero exploding damage **explodes into pieces when the damage that kills it (or hits the corpse) reaches that amount**, applying the explosion impulse to its parts; the debris and blood come from the **model's destruction configuration** (`destruction.md`: for example the bear's `TankExplosion` configuration has `Flesh_Explode.pfx` particles and `Flesh_02.mdl` debris). The numbers `.2` to `.4` could be distances or times; their names are numbers.

**Corpse sink.** A dead puppet stays, then sinks into the ground and disappears: `CCorpseSinkConfig` in `CGlobalGameParams` (steady 1.0 s, sink delay 0.1 s, sink period 0.45 s, a disappear effect and sound; `rsc-ctl.md`); one puppet file embeds its own configuration. There are no persistent blood pools: **corpse removal and effects are the only after-death state**.

## Other effect sources

- **Props** (`SPropEntity`, 1,726 in the worlds): every prop points to the same `propsplash.pfx` (`en_pfxSplash`) and `propdust.pfx` (`en_pfxDust`): the effects played when a prop falls into water and when it hits the ground (reading).
- **Placed effect entities** (`CEffectEntity`, **2,352** in 57 worlds, `wld-sound-effects.md`): 117 distinct files; the commonest are `fire_small.pfx` (955), `firefly_white.pfx` (215), `mdgbtemple_firesmall.pfx` (72), `lavasmoke_plainmedium.pfx` (63), `smoke_ventilator3.pfx` (57), `smoke_dust&leaves_big.pfx` (56), `firefly_yellow.pfx` (55), `stadium_flash.pfx` (54), `treevillage_fire_01.pfx` (53), `fire_medium.pfx` (50), `dust_flying_small.pfx` (50): ambience (fires, smoke, fireflies, dust), not combat effects.
- **Puppet effects** (`SPuppetEffectParams`, `ep.md`): footstep effect stretch and power, sound hot spots and fall-offs, alive effect stretch and power (an effect that runs while the puppet lives; `en_hfxAlive`).
- **Projectile impact:** projectile parameters give impact and range impulses, damage and the effect type (`bp_idEffectType`); they hold **no effect file pointer**, so the explosion effect of a projectile comes from elsewhere (its model's destruction configuration is the likely source: `destruction.md`; not verified).
- **Spawn effects:** `CSpawnEffectConfiguration` (11 entries, `sfxc_pefEffect`; `misc-types.md`): the effect when a character is spawned.

## What spawns when (summary)

| Moment | Source | Spawns |
|---|---|---|
| Bullet, saw or beam hits a surface | the surface's physical material, event `BulletHit` / `ChainSaw` / `BeamHit` | one `.pfx` and the material's sound |
| A character steps, jumps or lands | event `Step` / `Thrust` | one `.pfx` and sound |
| A projectile with effect type `Dust` hits ground | event `Dust` | one `.pfx` |
| A model breaks | its destruction configuration | debris models, particles, sound, shaker; next phase (`destruction.md`) |
| A puppet dies | death parameters + the model's destruction configuration | explosion impulse, debris, blood particles; then the corpse sink |
| A prop hits water or ground | `en_pfxSplash`, `en_pfxDust` | `propsplash.pfx`, `propdust.pfx` |
| A level loads | placed effect entities | ambience effects (auto-start entities) |

## Reader invariants

1. Look up a hit effect by the **physical material** of the surface and the **event name** (compare strings); a missing event means **no effect** (not an error); `playerflesh` and a few materials simply have none.
2. Pick among alternatives by weight (all 135 entries have one alternative: a weighted choice with one item).
3. Do not create decals, bullet holes or blood pools: the data has none; if a port adds them it is a **new feature**, not parity.
4. Gibs are debris models from destruction configurations; explode a corpse only according to the puppet's death parameters.
5. Fail on a particle scheme member that is not a `CPFXInScheme` list; the base scheme (`CParticleScheme.2`) is null in the retail data.

## Test vector

`Databases/Materials/Flesh.mat` (`IDNT` = `['', 'BulletHit', 'ChainSaw', 'BeamHit']`): attributes 1200, 0.8472, 0.7722, 0.2; particle scheme: `BulletHit` → `Effects/Materials/Flesh_Bullet.pfx`, `ChainSaw` → `Flesh_Saw.pfx`, `BeamHit` → `Flesh_Beam.pfx` (weight 1 each); sound scheme base `Sounds/Materials/Flesh.scs`. `Stone.mat`: `BulletHit` → `Stone_Bullet.pfx`, `Step` → `Stone_Step.pfx`, `Thrust` → `Stone_Thrust.pfx`, `Dust` → `Stone_Dust.pfx`. `Flower.mat`: `BulletHit`, `ChainSaw`, `Step`, `Thrust` and `Dust` → `Pumpkin_Explode.pfx`, `BeamHit` → `Stone_Beam.pfx`.

## Open questions

- **Run-time rules** (task T01-074): which code raises `Thrust` and `Dust` exactly, the lifetime and limits of effects (how many at once), how the hit position and normal orient the effect, damage thresholds for exploding corpses (units of `.2` to `.4`), and which effect (if any) the player's own hits show (`playerflesh` has none).
- Whether any effect draws geometry that stays after the effect ends (the effect format is in `pfx.md`; the effect classes of the retail files have no persistent mode: not verified for all 604).
- Where sounds for `Thrust` and `Dust` come from (`mat.md`, `audio.md`).
