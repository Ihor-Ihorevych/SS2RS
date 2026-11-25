# Diving, breath and drowning of legged puppets (T01-149)

- **Roadmap task:** T01-149 (follow-up of [`puppet-legged-classes.md`](puppet-legged-classes.md) section 3 and open question 4)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the breath-period table (reproduced by `tools/re/legged_diving_verify.py` over all `.ep` files); **medium** for every rule read from `Sam2Game.dll` (one reading of each function, the periodic tests and the thresholds re-checked in the instruction listing); **medium-low** where marked. **Nothing was run on the original game.**

Prerequisites: [`puppet-legged-classes.md`](puppet-legged-classes.md) (members `en_fFluidRatio`, `en_fFluidDensity`, `en_tmDived`, `en_fLastFluidRatio`; the pose enumeration), [`puppet-entity-classes.md`](puppet-entity-classes.md) section 7 (the damage record and `GetNominalHealth`), [`puppet-moving-anim.md`](puppet-moving-anim.md) (the dive and swim poses).

## Summary

A legged puppet is **"diving" while the fluid ratio of its body is above 0.99**. The fluid ratio is written by the physics hook `OnFluidTouched`; the legged `OnStep` watches it and does four things: it **starts the dive** (the clock `en_tmDived`, the sounds `Diving` and `DiveIn`), **ends it** (the sound `DiveOut` and one of three inhale sounds chosen by how much of the breath was used), makes **periodic sounds and effects** while under water (bubbles every 3 s of the dive; a warning after two thirds of the breath, every 2 s), and **deals drowning damage once per second after the breath period is over: one tenth of the puppet's nominal health per hit** (damage type `DMT_DROWNING`, 13).

- **Breath period** = `pbp_fKeepBreathPeriod` of the body parameters (`SLeggedPuppetBodyParams`, last member): **5.0 s in 84 of 86 legged parameter files, 60.0 s in the two player files `EntityParams/PlayerPuppet.ep` and `PlayerPuppet_DM.ep`** (the cut-scene player `Player/Player.ep` and `Santa.ep` have 5.0). A value below 0 disables the whole dive logic of `OnStep` (no file stores one).
- The numbers are in section 2; every time constant below is in seconds of the dive (`e` = seconds since `en_tmDived`).
- **Correction to `puppet-legged-classes.md`:** `GetDipContent` does **not** read the fluid ratio; it answers by the **pose** (section 1).

## Evidence

- **Binary:** `Sam2Game.dll` of the install used for the other specs. Read in full: the legged `OnStep` (the whole dive and drowning part; the listing was read for the periodic tests and the thresholds), `OnStartedDiving`, `OnStoppedDiving`, `OnFluidTouched`, `GetDipContent`. The meaning of the virtual calls was taken from the puppet's virtual table (`GetNominalHealth`, `GetFreeSoundSource`, `GetPuppetModel`).
- **Data:** `tools/re/legged_diving_verify.py` reads all `.ep` files: the breath period of the 86 legged parameter files, and evaluates the periodic tests of the page (a 12 s dive at 30 steps/s) for the test vectors.
- **Not checked:** nothing was run; the fluid contact itself (which body part, how the ratio is computed, buoyancy: `physics.md`, T01-141 for the swim and dive poses of the movement step); the scheme sounds and effects named here (`Diving`, `DiveIn`, `DiveOut`, `InhaleEasy/Medium/Hard`, `UnderWaterBubbles`, `Drown`: their content is in the sound and effect schemes of the models).

## 1. Fluid ratio and `GetDipContent`

- **`OnFluidTouched(own hull, fluid hull, ratio)`** (physics hook, read): `en_fFluidRatio` := `ratio` (the submerged fraction the physics reports; **0 to 1** as the code uses it) and `en_fFluidDensity` := the density member of the fluid hull's entity (0 when the other hull is not such a fluid). The base entity's hook is empty, so only legged puppets keep these values.
- **At the end of every `OnStep`** both members are **reset to 0 when the puppet has no body or the body has a certain flag (bit value 2) set**. What that flag means was not resolved (T01-269): if it is the "no fluid touch was reported" state, the ratio is 0 in every step without a touch; if not, the ratio persists until the next touch (**medium-low**; the dive end test below needs the ratio to fall below 0.99, so the first reading is the likely one).
- **`GetDipContent`** (read): **pose `PP_FLY` (5) -> 1 (air), `PP_DIVE` (3) or `PP_SWIM` (4) -> 2 (water), every other pose -> 0 (land)**. The pose itself is set by the movement step (T01-141).

## 2. The dive in `OnStep` (read)

Applies **only when the puppet has legged parameters and its body and mechanism exist** (otherwise `OnStep` returns before). Let `B` = the breath period (above), `f` = `en_fFluidRatio`, `f0` = `en_fLastFluidRatio` (the value of the previous step), `dt` = the simulation step length in seconds (`simGetStep`). The dive block runs **only for `B >= 0`**:

1. `e` := seconds since `en_tmDived`; `r` := **clamp(e / B, 0, 1)** (the "held fraction"; for `B = 0` the quotient is taken as read, no division guard was seen).
2. **Start:** if **`f0 < 0.99` and `f > 0.99`**: `en_tmDived` := now and **`OnStartedDiving`** is called.
3. **End:** if **`f0 > 0.99` and `f < 0.99`**: **`OnStoppedDiving(r)`** is called with the held fraction `r` of this step (computed from the `en_tmDived` of the dive just ending).
4. `en_fLastFluidRatio` := `f`.
5. **While `f > 0.99`** (still under water this step; the start step counts):
   - **Bubbles:** `UnderWaterBubbles` (a scheme effect from the model's effect scheme) is spawned at the puppet's view origin and the scheme sound `UnderWaterBubbles` is played on a free sound source, **in the step in which `floor(e / 3)` is greater than `floor((e - dt) / 3)`: once every 3.0 s of the dive** (the constant is the float 0.33333334; the first at e = 3 s).
   - **Breath warning, when `r > 0.6666`:** in the step in which **`floor(e x 0.5) > floor((e - dt) x 0.5)`** (once every 2.0 s of the dive, at the even seconds) the scheme sound **`Drown`** is played on the puppet's "alive" sound source and the scheme effect **`Drown`** is spawned at the view origin. (`0.6666 x B` = 3.333 s for a 5 s breath, 40 s for the player's 60 s breath: the first warning is at the first even second after that.)
   - **Drowning damage, when `e > B`:** with `x = e - B`, in the step in which **`floor(x) > floor(x - dt)`** (once per second of overtime, at `e = B + 1, B + 2, ...`) **the puppet receives a damage record: type `DMT_DROWNING` (13), amount `GetNominalHealth() / 10` (integer division, truncated), no attacker, zero direction, zero hit point and zero impulse**, delivered through `OnReceiveDamage` (alignment, invulnerability and the god cheat apply as for any damage: `puppet-entity-classes.md` section 7). So a puppet with the nominal health `H` loses **`H / 10` per second** after the breath is used up and is dead after about 10 s if nothing heals it; an `H` below 10 gives 0 damage (a non-positive amount is ignored by the damage function: **medium**).
6. The `e > B` and `r > 0.6666` tests use the same `e`; **a long breath period (60 s) therefore warns from 40 s and hurts from 60 s**.

**`OnStartedDiving`** (read): plays the scheme sound **`Diving`** on the puppet's dedicated diving sound source (a looping sound; the handle member the creation also uses: `en_hss*`) and sets `en_fMovingSoundVolume` to that sound's volume (to 1.0 when the sound could not be played); then plays **`DiveIn`** on a free sound source (the mouth). **`OnStoppedDiving(r)`** (read): stops the sound of the diving source; plays **`DiveOut`** on a free sound source, then **one inhale sound on the puppet's "alive" sound source: `InhaleEasy` when `r < 0.2`, `InhaleMedium` when `0.2 <= r < 0.8`, `InhaleHard` when `r >= 0.8`** (the constants 0.2 and 0.8; the comparisons are "less than"). The scheme sound names are resolved through the puppet model's sound scheme (`GetPuppetModel`, `samPlaySchemeSound`).

## Invariants

1. `pbp_fKeepBreathPeriod` is 5.0 in 84 files and 60.0 in 2; none is 0 or less (script).
2. Drowning damage needs `e > B`: **no puppet is hurt before its breath period has passed**; at `B = 5 s` the first hit is at `e = 6 s` (the step that crosses it).
3. The three inhale sounds partition the held fraction `[0, 1]` into `[0, 0.2)`, `[0.2, 0.8)`, `[0.8, 1]`.
4. All periodic events fire at most once per step.

## Open questions

1. **The flag that clears the fluid ratio** at the end of `OnStep`, and the fluid contact itself (what the ratio measures, the density use, buoyancy of the dive and swim poses): **T01-269** (with T01-141 and `physics.md`).
2. **Run-time check** of the periods (3 s bubbles, 2 s warning, 1 s damage), the 0.99 threshold and the damage per second on the running original: **T01-270** (with T01-146).
3. The contents of the sound and effect schemes named here (`Diving`, `DiveIn`, `DiveOut`, `Inhale*`, `UnderWaterBubbles`, `Drown`): T01-057 (schemes) and the audio specs.
4. The co-op scaling of `GetNominalHealth` (`puppet-entity-classes.md` section 7): T01-140.

## Test vectors

From `tools/re/legged_diving_verify.py`, step 1/30 s, breath period 5 s, a puppet that goes under water at `e = 0` and stays:

- **Bubbles** at e = 3, 6, 9, 12 s. **Warning** (`r > 0.6666`, i.e. `e > 3.333`): at e = 4, 6, 8, 10, 12 s. **Drowning damage** at e = 6, 7, 8, 9, 10, 11, 12 s: at nominal health 100 -> **10 per hit** (dead at e = 15 s from 100 health if never healed: ten hits); 96 -> 9; 5000 (a boss) -> 500; 9 -> 0.
- **Player** (breath 60 s, health 100): warning from e = 40.0 s (the first even second at or above 39.996 is 40), damage from e = 61 s, 10 per second.
- **Inhale choice:** held fraction 0.0 -> `InhaleEasy`; 0.19 -> `InhaleEasy`; 0.2 -> `InhaleMedium`; 0.79 -> `InhaleMedium`; 0.8 -> `InhaleHard`; 1.0 -> `InhaleHard`. Surfacing after 1.5 s with `B` = 5 s: `r` = 0.3 -> `InhaleMedium`; after 4.5 s: 0.9 -> `InhaleHard`; after 0.5 s: 0.1 -> `InhaleEasy` (the exact boundaries 1.0 s and 4.0 s are float-rounding cases: not asserted).
- **Dip content:** `PP_STAND` -> 0; `PP_SWIM` -> 2; `PP_DIVE` -> 2; `PP_FLY` -> 1.
- **Thresholds:** ratios `f0 = 0.98`, `f = 0.995` start the dive; `f0 = 0.995`, `f = 0.99` (not below 0.99) do **not** end it; `f = 0.989` does.
