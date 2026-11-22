# `.mat` physical materials and the scheme files (`.scs`, `.scp`, `.sca`, `.mas`)

- **Roadmap task:** T01-013 (also identifies `.scp`, `.sca`, `.mas` for T01-019)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the structure of all five file types, the scheme layout (events with weighted choices), how materials and models refer to schemes, and for the gameplay members of a material (they match the material names); **medium** for the physical attributes `CMaterialAttributes.1` (density) and `.2` (friction-like) and for the roles of the sound parameters; **low** for `CMaterialAttributes.3`/`.4` and the flag bits. Scope: **35** `.mat`, **37** `.scs`, **7** `.scp`, **4** `.sca` and **20** `.mas` files (all format 8, all decode).

Prerequisites: `ctsemeta-container.md`, `ctsemeta-types.md`, `ctsemeta-objects.md`; `mtr.md` (presets name a physical material), `mdl.md` (`CSamCustomModelData`).

## Summary

A **physical material** (`.mat`, root class `CSamMaterial`) describes what a surface is *made of* for gameplay and physics: a density and friction-like numbers, a movement-speed multiplier, damage-over-time, and two **schemes**: what particle effects and what sounds to play for events such as `BulletHit`, `Step` or `ChainSaw` on that surface. A **scheme** is a list of *events*; each event has a list of weighted alternatives (a particle effect or a sound) of which one is picked at random. The same scheme classes also appear on their own in files: `.scs` = sound scheme, `.scp` = particle scheme, `.sca` = animation scheme, `.mas` = moving animation scheme (identified here by their root classes). Visual materials (`mtr.md`) point at the physical material through `CShaderPreset.2`.

## Evidence

`tools/re/mat_verify.py` (read-only, `SS2_DATA_DIR`) over the retail install with all patches:

- All 35 + 37 + 7 + 4 + 20 files decode; each has exactly **one shared root** of the class named in the table below: `.mat` → `CSamMaterial`, `.scs` → `CSoundScheme`, `.scp` → `CParticleScheme`, `.sca` → `CAnimationScheme`, `.mas` → `CMovingAnimationScheme`.
- **All external references resolve** to a file of the install: `.mat` 187, `.scs` 597 (all `.wav`), `.scp` 7, `.mas` 15, `.sca` 1.
- All 141 event ids in the materials' schemes are valid indices into the file's `IDNT` table.
- The gameplay members match the material names: only `lava` and `swampdeadly` deal damage, only the slow materials (`hardwood_moveslow`, `lava`, `swamp`, `swampdeadly`) have a speed multiplier other than 1.
- 5,274 material presets in models, meshes and `.mtr` files name one of the `.mat` files (25 of the 35 are used).

## The `.mat` file

```
CSamMaterial                           (root; base class CMaterialAttributes)
  CMaterialAttributes.1   float        density-like: 550 .. 8,500 (see the table)
  CMaterialAttributes.2   float        friction-like: 0.173 (ice) .. 0.983 (the "high friction" materials)
  CMaterialAttributes.3   float        0.448 .. 0.772 (0.548 in 17 materials)
  CMaterialAttributes.4   float        0.01 .. 0.548
  CSamMaterial.sm_fMovementSpeedMultiplier        float   1.0 in 31; 0.6, 0.75, 0.7, 0.35
  CSamMaterial.sm_bApplySpeedMultiplierOnPlayersOnly   BOOL
  CSamMaterial.sm_fDamageApplyPeriod              float   seconds between damage ticks (1.0; 0.5 in swampdeadly)
  CSamMaterial.sm_slApplyDamageAmount             int     0 in 33; lava 50, swampdeadly 5
  CSamMaterial.sm_bApplyDamageOnPlayersOnly       BOOL
  CSamMaterial.sm_ppsParticleScheme               -> CParticleScheme   (embedded in 34, null in 1)
  CSamMaterial.sm_pssSoundScheme                  -> CSoundScheme      (embedded in 34, external .scs in 1)
```

**The attributes.** `.1` behaves as a density in kg/m³ (copper and steel 8,500, iron 7,800, aluminium 2,700, stone 2,500, glass 2,200, sand and lava 1,500, water 1,000, ice 900, hardwood 850, lightwood 550). `.2` is friction-like: the materials named `*_highfriction` have 0.983, ice 0.173, snow 0.2. `.3` and `.4` are not identified (restitution, damping or a second friction are candidates; sand, lava and swamp have the high `.4` = 0.548, glass, ice and water the low 0.01). The names come from the file names; the data has no unit.

| Material | `.1` | `.2` | `.3` | `.4` | Notes |
|---|---|---|---|---|---|
| default | 1000 | 0.707 | 0.548 | 0.1 | |
| stone, stone_highfriction | 2500 | 0.775, 0.983 | 0.548 | 0.173 | the most used: 1,855 presets |
| hardwood, hardwood_highfriction, hardwood_moveslow | 850, 1000, 850 | 0.447, 0.983, 0.447 | 0.548, 0.448, 0.548 | 0.15, 0.173, 0.15 | `moveslow`: speed ×0.6 |
| lightwood | 550 | 0.347 | 0.548 | 0.15 | |
| steel, copper | 8500 | 0.648 | 0.548 | 0.12 | |
| iron | 7800 | 0.648 | 0.548 | 0.12 | |
| aluminium | 2700 | 0.648 | 0.548 | 0.12 | |
| glass | 2200 | 0.447 | 0.548 | 0.01 | |
| ice | 900 | 0.173 | 0.548 | 0.01 | |
| snow | 900 | 0.2 | 0.54 | 0.1 | |
| sand | 1500 | 0.848 | 0.548 | 0.548 | |
| grass, flower | 1000 | 0.983 | 0.448 | 0.173 | |
| water, shallowwater | 1000 | 0.5 | 0.5 | 0.01 | |
| lava | 1500 | 0.248 | 0.548 | 0.548 | speed ×0.75, 50 damage per 1.0 s |
| swamp | 1500 | 0.648 | 0.548 | 0.548 | speed ×0.7 |
| swampdeadly | 1500 | 0.248 | 0.548 | 0.548 | speed ×0.35, 5 damage per 0.5 s |
| acid, bones, deadflesh, energy, feathers, fire, flesh, fleshmechanism, fleshmetal, magic, mechanism, rubber | 1200 | 0.847 | 0.772 | 0.2 | the common "organic/effect" attributes |
| playerflesh | 1200 | 0.707 | 0.548 | 0.1 | |

## Schemes

```
CSoundScheme                               CParticleScheme
  .1  list of CSoundInScheme                 .1  list of CPFXInScheme
       .1  event id: an index into IDNT           .1  event id: an index into IDNT
       .2  list of CWeightedSoundInScheme         .2  list of CWeightedPFXInScheme
            .1  int      weight                        .1  int      weight
            .2  -> .wav (external)                     .2  -> .pfx (external)
            .3  SndFlags (0x2 427, 0x6 161,            .3  int      (0 in the samples)
                 0x3 22, 0x7 5)
            .4  float    0.5 .. 1.5 (1.0 in 510)
            .5  float    0 .. 0.5 (0.05 in 384)
            .6  float    0.35 .. 1.0 (1.0 in 508)
            .7  EPlatform
       .3  EPlatform
  .2  -> base scheme (a CSoundScheme or external .scs) or null     (CParticleScheme.2: null in all 34)
```

- **Events.** An event is **named by a string in the file's `IDNT` table**: the names in the sound schemes of `.scs` files are `Sight` (14 files), `Fire` (13), `Wound` (12), `Explode` (11), `BulletHit` (11), `Step` (9), `Death` (8), `ChainSaw` (8), `BeamHit` (7), `Moving` (6), `Taunt`, `Floor`, `Alive`, `ObjectGrab`, `ObjectThrow`, `DiveIn`, `DiveOut`, … The materials' particle schemes use `BulletHit` (33 materials), `ChainSaw` (33), `BeamHit` (30), `Step` (16), `Thrust` (12), `Dust` (11); their embedded sound schemes only `BulletHit` (6 materials). Event names are matched by the **string**, never by the number (the table differs per file).
- **Weighted choices.** An event has 1 to 14 alternatives (mean 2.7); `weight` is 1 in 611 of the 615 alternatives (6, 7, 16 and 30 once each): a relative probability. Selecting by weight is the obvious reading; the data cannot prove it.
- **Sound parameters** (names are not in the data): `.4` looks like a pitch or speed factor (1.0 = normal), `.6` a volume (1.0 = full), `.5` a random variation (0.05 or 0.1 mostly). `SndFlags` is a bit set: bit `0x2` is set in **all 615** alternatives, `0x4` in 166 and `0x1` in 27.
- **Inheritance.** In a material the sound scheme's `.2` is a **base scheme**: 27 of the 34 embedded sound schemes name an external `.scs` named after the surface kind (`Metal.scs` 7, `Wood.scs` 5, `Fire.scs` 4, `Flesh.scs`, `Stone.scs`, `Grass.scs` 2 each, `Default.scs`, `Energy.scs`, `Sand.scs`, `Snow.scs`, `Water.scs`); 6 have none, 1 an object. So a material defines only `BulletHit` itself and probably takes the other surface sounds (`Step`, `Floor`, …) from the base `.scs`: events not found in the scheme being looked up in the base is the natural reading of the `.2` member, but the data cannot prove the lookup order (not verified that each base file defines those events). The `.scs` files themselves never have a base (37 of 37 null).

## Scheme files and who uses them

| Extension | Root class | Files | Identified by |
|---|---|---|---|
| `.scs` | `CSoundScheme` | 37 | root class; 597 `.wav` references |
| `.scp` | `CParticleScheme` | 7 | root class |
| `.sca` | `CAnimationScheme` | 4 | root class |
| `.mas` | `CMovingAnimationScheme` | 20 | root class |
| `.mat` | `CSamMaterial` | 35 | root class |

The schemes are attached by:

- **Material presets** (`mtr.md`): `CShaderPreset.2` → a `.mat` (5,274 external, 1 embedded, 2,207 null). The physical material belongs to the **visual material of a mesh surface**, so a surface decides its impact effects, footsteps and friction.
- **Models** (`CSamCustomModelData` of `mdl.md`): `.1` a particle scheme (embedded 171, external `.scp` 78), `.2` a sound scheme (embedded 373, external `.scs` 14), `.3` an animation scheme (embedded 92, external `.sca` 27), `.4` a moving animation scheme (external `.mas` 70, embedded 40).
- **The world info entity**: `CWorldInfoEntity.en_pmaShallowWater` → `ShallowWater.mat`.
- **Animation events** (`ans.md`): the sound and effect events name scheme events by identifier (`ssae_idSound`); the link was not verified.

The `.sca` and `.mas` classes (`CAnimationScheme`, `CMovingAnimationScheme`) are not specified here; they are the animation-side counterpart (which animation to play for a named event) and are left to T01-019/T01-029.

## Reader invariants

1. Require one shared root of the class that matches the extension.
2. Match event names by the string from the file's own `IDNT` table.
3. For a sound or particle event: look it up in the scheme, then in its base scheme (sound), then give up silently.
4. Resolve every external reference through the virtual file system (all resolve in the retail data).
5. Apply the speed multiplier and damage per period while an entity stands on a surface whose material has them (the `…OnPlayersOnly` flags restrict this to players).

## Test vectors

`Content/SeriousSam2/Databases/Materials/Acid.mat` (3,067 bytes): `IDNT` = `['', 'BulletHit', 'ChainSaw', 'BeamHit']`; `CSamMaterial` with `CMaterialAttributes` = (1200.0, 0.8472, 0.7722, 0.2), speed multiplier 1.0, damage 0 every 1.0 s, both `…OnPlayersOnly` = 0; a `CParticleScheme` with three events (ids 1, 2, 3 = `BulletHit`, `ChainSaw`, `BeamHit`), each with one alternative of weight 1 → an external `.pfx` (`.3` = 0), base null; a `CSoundScheme` with one event (id 1 = `BulletHit`) with three alternatives (weight 1, `SndFlags 0x2`, `.4` = 1.0, `.5` = 0.1, `.6` = 1.0, `EPlatform 0xFFFFFFFF`) → three external `.wav` files, base null.

`Content/SeriousSam2/Sounds/Vehicles/SpawnPods/SpawnPods.scs` (1,460 bytes): `IDNT` = `['', 'Alive']`; one event `Alive` (id 1) with one alternative (weight 1, `SndFlags 0x3`, `.4` = 1.0, `.5` = 0.0, `.6` = 1.0).

## Open questions

- The meaning of `CMaterialAttributes.3` and `.4`, the units, and how the physics uses all four (T01-023 physics middleware).
- The bits of `SndFlags` and the third member of `CWeightedPFXInScheme`; the exact use of `.4`/`.5`/`.6` (pitch, variation, volume).
- Event resolution order across a scheme and its base, and how `Step` events on a material are chosen between its particle scheme and the entity's own scheme (gameplay, T01-024).
- The classes `CAnimationScheme` and `CMovingAnimationScheme` (`.sca`, `.mas`).
