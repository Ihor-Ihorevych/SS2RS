# Destruction: breakable objects, phases, debris and effects (T01-046)

- **Roadmap task:** T01-046 (the destruction part of T01-057 is specified here)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the stored structure (holder, configurations, custom data, effects, substitute models, platform masks), for the counts, for the phase chains and for how many placed objects can break (all 3,707 models and 59 worlds, `tools/re/destruction_verify.py`; members from the files' own type tables); **medium** for the roles of members that only have numbers; **low** for the run-time rules: when damage accumulates into a break, how debris is simulated, how the platform is chosen. Scope: all 3,707 `.mdl` files and all 59 worlds.

Prerequisites: `mdl.md` (the model file; `CModelConfiguration.14` is the holder), `pfx.md` (particle effects), `wld-sound-effects.md` (sound), `environment.md` (shakers), `level-logic.md` (the script actions), `wld-entities.md`.

## Summary

- **Destruction is data in the model.** A model that can break has a `CDestructionHolder` (`CModelConfiguration.14`) with one or two **destruction configurations** (`CDestructionConfig`): custom data (health and damage rules), a list of **effects** (`CDFX…`: debris, particle effects, sounds, a camera shaker, each placed by a **distribution**) and an optional **substitute model** that replaces the object.
- **Destruction goes in phases.** A substitute model can itself have a destruction holder: 147 of the 224 substitute models do (chains such as `phase_01` → `phase_02` → `phase_03`), which is what the script action `macDestroyOnePhase` (197 uses) steps through.
- 740 of 3,707 models have at least one configuration (754 configurations), and **10,526 of 28,641 placed model renderables** (37 %) use such a model: 8,981 static model entities, 1,487 props, 28 turret characters, 16 doors and 14 legged characters.

## Evidence

`tools/re/destruction_verify.py` (read-only, `SS2_DATA_DIR`; about 12 minutes) over the retail install with all patches:

- **Models:** 3,707 `.mdl`; 2,938 have a destruction holder (769 have none); 2,198 holders are empty, 726 have one configuration and 14 have two: **740 models** with at least one, **754 configurations**.
- **Configurations:** 468 have custom data (`CModelDestructionCustomData`); 224 name an external substitute model (all 224 exist), 4 embed one, 526 have none; 26 have no effect and the others 1 to 40 (5.1 on average).
- **Effects:** 3,832 effects: debris 2,140, particle 1,153, sound 504, shaker 35, each with a distribution (point 3,499, box 165, line 71, sphere 97).
- **References:** 2,142 debris model references, 1,151 particle effect references (1,148 are existing `.pfx` files), 504 sounds; 2,642 of the debris and sound references resolve to files of the install.
- **Worlds:** 28,641 model renderables name a model; 10,526 name one with destruction configurations (table below).

## The stored structure

```
CModelConfiguration.14 -> CDestructionHolder            (embedded in 2,938 models)
CDestructionHolder
  .1  list of CDestructionConfig (0, 1 or 2)       .2  an editor holder (null in the shipped files)
CDestructionConfig
  .1  IDENT name        Default 594, PC 44, FlowerDeath 16, Player 7, Generic_Flesh 6, Generic_Mech 6, HoverFighter 6, TankExplosion 5, ... (59 names)
  .9  EPlatform mask    0xFFFFFFFF (553), 0xFFFFFFFD (191), 0xFFFFFFFE (10)
  .4  -> CModelDestructionCustomData (468) | null (286)
  .2  -> CModelConfiguration: the substitute model (224 external .mdl, 4 embedded, null in 526)
  .3  list of -> CBaseDFX effects (0 to 40)
  .5  IDENT shake name ('' in 669; Shake 34, Shake_01 27, tree 6)      .8 IDENT animation ('' in 732; Idle_01 22)      .7 BOOL      .10 stale
CModelDestructionCustomData
  mdcd_slHealth       -1 (13), up to 100 (321), up to 1,000 (129), above 1,000 (5)
  mdcd_mbtBodyType    ModelBodyType 0 .. 6  (3: 184, 1: 162, 4: 70, 5: 30, 0: 16, 2: 5, 6: 1)
  mdcd_slRangeDamage  -1, 5, 150 or 4000        mdcd_fForceImpulse 10 or 2000 (+ UpRatio 0.5)
  mdcd_fDamageHotSpot 2, 4 or 10                mdcd_fDamageFalloff 4, 16 or 20
  mdcd_dtDamageType   EDamageType 11 (429), 19 (28), 7 (1), 0xFFFFFFFF (10)
CBaseDFX (the effect base)        .1 IDENT name (820 distinct: 'Debris_01', 'Sound', 'Particles', 'Shaker', ...)   .2 -> CDFXDistribution   bde_bTransferVelocity BOOL
  CDFXDebris (2,140)    .1 list of CWeightedModel: weight, the debris model (.mdl), size and speed ranges, a direction vector, a disturbance, a collision mode (DebrisCollision 0, 1 or 3), toughness (0 to 4), a trail effect (.pfx)
  CDFXParticles (1,153) .1 list of CWeightedParticlesEffect: weight, the .pfx (ref), a scale
  CDFXSound (504)       .1 the sound (.wav), volume, pitch, pitch randomness 0.01 to 0.1, hot spot 1 to 1,500, fall-off 15 to 2,000, cone angles 360, SndFlags 2
  CDFXShaker (35)       dfxs_spShakingParams: the camera shake of `environment.md` (hot spot, fall-off, damping, intensities, frequencies)
CDFXDistribution (placement of the effect): point (a QuatVect and a bone name, 25 bone names such as R_LowerArm), CDFXDistributionBox (size, count 2 to 30), ...Line (length, count 1 to 20), ...Sphere (radii, angle ranges, count 2 to 20)
```

- **Platform alternatives.** Ten of the 14 two-configuration holders pair a configuration masked `0xFFFFFFFD` with one masked `0xFFFFFFFE` (`Player`/`Generic_Flesh` 6, `TankExplosion`/`Generic_DeadFlesh` 2, `Helicopter`/`Generic_Mech`, `Default`/`XboxGeneric`); the other four pairs have both masks `0xFFFFFFFF` and differ by name (`PC`/`Xbox`, `Default`/`Xbox`, `Default`/`XboxGeneric`, `Default`/`Default`). The names `PC` (44 configurations), `XboxGeneric` (4) and `Xbox` say which build a configuration is meant for: the **mask and the name select the configuration for the running platform** (reading; which bit is the PC and how the choice is made are not stated by the data).
- **Phases.** `CDestructionConfig.2` replaces the object with the substitute model; 147 of the 224 substitutes have their own holder, so a break is a **chain of phases** (`phase_01` → `phase_02` → `phase_03`; `junglebush_07c` → `phase_01`). No substitute is its own model. Scripts advance one step with `macDestroyOnePhase` (149 + 48 uses on static models) and observe `Destroyed` (44 uses) and `Damaged` (18).
- **Damage rules.** `mdcd_slHealth` is the object's health (most are 1 to 100), `mdcd_dtDamageType` is probably the damage class that counts (a number: the names are in the game module); the hot spot and fall-off have the names of the two-radius model of `audio.md` and probably describe **damage around an explosion**; `mdcd_slRangeDamage` is probably the damage the destroyed object deals around itself (−1 none). The placed entity keeps an accumulated damage (`CStaticModelEntity.en_slAccuDamage`) and sound slots for the destruction (`en_hssDestruction`, `wld-sound-effects.md`).

## Which placed objects can break

| Owner of the model renderable | Renderables | With destruction configurations |
|---|---|---|
| `CStaticModelEntity` | 25,383 | 8,981 |
| `SPropEntity` | 1,690 | 1,487 |
| `SDoorEntity` | 176 | 16 |
| others (pickups, portals, characters, …) | 1,392 | 42 (`STurretCharacterEntity` 28, `SLeggedCharacterEntity` 14) |

First configuration names of the 10,526: `Default` 9,223, `PC` 1,241, `XboxGeneric` 17, `Generic_Acid` 10, `MachineGun_Static` 7, `FlowerDeath` 6, `TankExplosion` 6, `PlasmaLauncher` 5, `Catapult_Explode` 4, and a few more. `destruction-models.md` (generated) lists every model with its configurations, effect counts (`D` debris, `P` particles, `S` sound, `K` shaker) and health.

## Reader invariants

1. Read the holder from `CModelConfiguration.14`; null or an empty list means the model cannot break.
2. Choose the configuration for the running platform by mask and name (14 holders have two; in 10 the masks are exclusive; the rule for the rest is not determined).
3. A substitute model is a complete model with its own holder: follow the chain; no cycle exists in the data (no substitute is its own model).
4. Effects are independent: debris, particles, sound and shaker of one configuration start together; each is placed by its distribution (default a point relative to the model or a named bone).
5. Treat `mdcd_dtDamageType` and `mdcd_mbtBodyType` as opaque numbers until the game rules spec names them.
6. Fail on an unknown `CBaseDFX` or `CDFXDistribution` class; the four effect classes and four distribution classes above are the complete list of the retail data.

## Test vector

`Content/SeriousSam2/Models/Animals/Bear/Bear2.mdl` (57,667 bytes): `IDNT` begins `['', 'Rendering 1', 'Default', 'Bear', 'TankExplosion', 'Explosion', 'Sound_Explosion', 'Shaker', 'Flesh']`. Its `CDestructionHolder` has one configuration `TankExplosion` (mask `0xFFFFFFFD`, no substitute, no idle animation) with custom data health 100, body type 0, range damage −1, force impulse 2,000 (up ratio 0.5), damage hot spot 4, fall-off 16, damage type 11, and four effects: particles (`Flesh_Explode.pfx`, scale 1.2), a sound (`Impact_FleshLarge04.wav`, floats 1 and 1, pitch randomness 0.05, hot spot 10, fall-off 150), a shaker (hot spot 20, fall-off 50, damping 5, intensities 0.1, 0.1, 1) and debris (`Flesh_02.mdl`, a sphere distribution, trail `Flesh_Trail.pfx`).

## Open questions

- **Run-time rules:** how damage accumulates, the exact break test (health versus accumulated damage, the damage type match), how a phase transition swaps the model, the debris simulation (speeds, lifetime, collision modes 0, 1, 3 and toughness 0 to 4) and how the platform bit is chosen: measurement (T01-038/039) and the game module.
- The names of `ModelBodyType` (0 to 6) and `EDamageType` (7, 11, 19) and of the EPlatform bits.
- `CDestructionConfig.5` (shake names) and `.8` (an idle animation), `CBaseDFX.1` names, the weighted debris members with number-only names, and the 769 models without a holder (they cannot break).
- The editor classes (`CEdtDestructionConfig`, `CEdtDestructionEffect`, `CEdtDestructionHolder`) appear only as editor data (`mdl.md`).
