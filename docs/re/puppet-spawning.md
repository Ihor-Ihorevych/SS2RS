# Puppet spawning and appearing: spawnee rising, spawn source, `Appear` and `Disappear` (T01-145)

- **Roadmap task:** T01-145 (follow-up of [`puppet-entity-classes.md`](puppet-entity-classes.md) open question 5; the spawner side is [`mechanism-classes-spawners.md`](mechanism-classes-spawners.md), T01-109)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the counts and value tables of the spawn effect configurations (reproduced by `tools/re/puppet_spawning_verify.py`); **medium** for the rules read from `Sam2Game.dll` (one reading of each function, the branch polarity of the stop test checked in the instruction listing); **medium-low / low** where marked. **Nothing was run on the original game.**

Prerequisites: [`puppet-entity-classes.md`](puppet-entity-classes.md) (the 16 puppet states, section 4), [`mechanism-classes-spawners.md`](mechanism-classes-spawners.md) (the spawner, its template record and the spawn effect configuration), [`brain-classes.md`](brain-classes.md), [`puppet-moving-anim.md`](puppet-moving-anim.md) (animation groups, `samRemapSchemeAnim`).

## Summary

A character can take part in spawning in three ways; each uses its own puppet states (`PS_BEING_SPAWN_SOURCE` 12, `PS_DISAPPEARING` 13, `PS_APPEARING` 14, `PS_SPAWNEE_RISING` 15):

1. **As a spawnee** that has just been created by a spawner: `SetSpawner(spawner)` records the spawner and, when the spawner's **spawn effect configuration** names a spawnee animation the model has, plays it (state 15, "rising") and hands control back to the brain afterwards (section 1).
2. **As a spawn source** that carries spawnees (a legged character, a turret or a prop named by `ep_hepSpawnSource`; 7 spawners in the data): the spawner asks it to release one spawnee at a time; the puppet may stop and turn to the throw direction, plays a prepare and a spawn animation, and **an animation event `Release spawnee (<attachment>)` hands the creation back to the spawner** at the attachment's placement (section 2).
3. **By script** with `Appear` and `Disappear` (hide or show a character, with an optional scheme animation named `Appear` / `Disappear`): section 3.

## Evidence

- **Binary:** `Sam2Game.dll` of the install used for the other specs. Read in full: `OnLookDirMatched`, `SetSpawner`, `YouWillBeMySpawnSource`, `PrepareForLaunch`, `DoSpawnSource`, `SpawnSourceStopped`, `OnSpawnSourcePrepareAnimationFinished`, `OnSpawnSourceAnimationFinished`, `OnAnimEventReleaseSpawnee`, `ManuallyReleaseSpawnee`, `SpaweeAnimationFinished`, `Appear`, `DoAppear`, `Disappear`, `DoDisappear`, `OnAnimEventAppear`; of the spawner `SpawnSourceAndSpawnEffect` (the part that talks to the source) and the puppet's `SetState` and the spawn-source test of its `OnStep`. The listing was read for the stop test (the polarity of the comparison), the play arguments of the three animation calls and the member order of the configuration. The brain and spawner methods that the puppet calls were named from their virtual tables.
- **Data:** `tools/re/puppet_spawning_verify.py` reads every `CSpawnEffectConfiguration` of the install (the `.sfc` files and the configurations embedded in the worlds) and prints the tables of section 4 (about 6 minutes: it parses all 59 worlds).
- **Not checked:** nothing was run; the spawner's own procedures and the creation of the spawnee (T01-109); the spawn effect itself (`.sfc` effect and sound, `DoSpawnEffect`); the launch (`LaunchEntity`, T01-107/T01-136); the end of the `Appear` and `Disappear` animations (open question 2).

Member names follow `puppet-entity-classes.md` section 3: `en_henSpawnerWhoSpawnedMe` (the spawner that made this puppet) and `en_henSpawnerThatUsesUs` (the spawner that uses this puppet as spawn source), `en_ctSpawnSourceRequests` (releases still requested), `en_henSpawner` (the spawner that owns this puppet as its template); matched to the code by role and by the declared order (**medium**).

## 1. The spawnee side

**`SetSpawner(spawner)`** (read; called by the spawner after it created the puppet):

1. `en_henSpawnerWhoSpawnedMe` := the spawner (null when the argument is not a spawner).
2. Take the spawner's **spawn effect configuration** (`ep_sfcSpawnEffectConfig`; none: nothing more happens). **If its `sfxc_idSpawneeAnimation` is set**: remap the id through the model's animation scheme (`puppet-moving-anim.md` section 7); **if the model has that animation**: a new cloned animation state with the lerp time `en_fAnimLerpTime`; the animation is **played once (mode 2), speed 1, strength 1, in group 3**; `L := max(length - lerp time, 1e-5)`; **`SetState(PS_SPAWNEE_RISING)`**; the brain is told **`PuppetDoesSpawneeAppearAnim`** (brain: sets the generic-action ability and enters `BS_EXECUTING_GENERIC_ACTION`: `brain-classes.md`); **think `SpaweeAnimationFinished` after `L`** (the binary's spelling).
3. If the identifier is not set or the model lacks the animation the puppet stays in its current state and the brain decides as usual.

**`SpaweeAnimationFinished`:** the brain is told `OnActionFinished`; then **`SetState(PS_START_AUTO_PATROLING)` (10) when the puppet has a brain and its auto-patrol switch is set** (`en_bAutoPatrolMarkers`, matched by role: **medium-low**), else **`SetState(PS_DEFAULT)`**.

There is no code for state 15 in `SetState` (no think routine of its own; the think point is the one set above): the state only marks the puppet as busy rising. **Medium.**

## 2. The spawn source side

**Request.** The spawner's `SpawnSourceAndSpawnEffect` (when the spawner has a source) computes the launch velocity and asks the source puppet through its virtual **`PrepareForLaunch(spawner)`**: `en_henSpawnerThatUsesUs` := the spawner (only when it is a spawner) and **`en_ctSpawnSourceRequests` += 1**. (`YouWillBeMySpawnSource(spawner)` just stores the spawner.) The spawner then waits for `SpawnReady`. A spawner that has a source but **no spawn effect configuration** prints "Spawner that uses spawn source doesn't have spawn effect configuration defined!" and spawns without the source.

**Entering the state.** The puppet's own `OnStep` (every step): **`en_ctSpawnSourceRequests` > 0 and the puppet state is `PS_DEFAULT` (1)** -> `SetState(PS_BEING_SPAWN_SOURCE)`; the state's entry calls `DoSpawnSource`.

**`DoSpawnSource`** (read; the stop test polarity checked in the listing): the configuration is the spawner's. A random number `r` in [0, 1) is drawn from the global generator. **`stop = (r <= sfxc_fSpawnSourceStopChance)`** (the decompiled text shows the opposite polarity; the instruction listing shows "less or equal": **medium**; so a chance of 1 always stops, 0 practically never, 0.25 in a quarter of the cases). The brain is told **`PuppetDoesSpawnSource(stop)`**. **If stop: `SpawnSourceStopped` is called at once.** Otherwise: `StopMoving`, `t := max(|velocity| / GetCurrentDeceleration(), 1e-5)` (the time to brake), **think `SpawnSourceStopped` after `t`**.

**`SpawnSourceStopped`:** if the configuration's `sfxc_bShouldMatchThrowDir` is clear: **`OnLookDirMatched` at once**. Otherwise, if the spawner's **last launch velocity** vector (the spawner's `en_vLastSpawnVelocity`-like member written by `SpawnSourceAndSpawnEffect`; **name by role**) is longer than 1e-5: the puppet's **forced look direction** := the heading and pitch of that vector (`heading = atan2(-x, -z)`, 0 when the vertical share exceeds 0.99; `pitch = asin(y / |v|)`; roll 0), the two look-forcing flags are set, **think `OnLookDirMatched` after `GetMatchDesiredLookTime()`** (the turn time of `puppet-attacks.md` section 4). A zero launch velocity gives the min delay 1e-5 s.

**`OnLookDirMatched`** (read): **no spawn animation (`sfxc_idSpawnSourceAnimation` empty): think `ManuallyReleaseSpawnee` after 1e-5 s**; else **no prepare animation (`sfxc_idSpawnSourcePrepareAnimation` empty): `OnSpawnSourcePrepareAnimationFinished` at once**; else the prepare animation is remapped and played (arguments not recovered) and `OnSpawnSourcePrepareAnimationFinished` is thought after `max(length - lerp time, 1e-5)`. (The prepare animation is `empty` in every stored configuration, so that step is a pass-through in the data.)

**`OnSpawnSourcePrepareAnimationFinished`:** if `sfxc_idSpawnSourceAnimation` is not set: **think `ManuallyReleaseSpawnee` after 1e-5 s**. Else the animation is remapped through the scheme and played (arguments not recovered), `L := max(length - lerp time, 1e-5)`, think **`OnSpawnSourceAnimationFinished` after `L`**. **The release is made by the animation event `Release spawnee (<attachment>)` (`OnAnimEventReleaseSpawnee`) inside that animation** (9 such events in the data: `helper-classes-events.md`).

**`OnAnimEventReleaseSpawnee(attachment)`** (read): needs the spawner, the model and **at least one pending request**; then **`en_ctSpawnSourceRequests` -= 1**, takes the **absolute placement of the model attachment** (orthonormalised: the two axes re-normalised and crossed) and calls the spawner's **`SpawnReady(placement)`**: the spawnee is created at that placement (launch: T01-109). **`ManuallyReleaseSpawnee`** does the same without an event: requests -= 1, `SpawnReady` with the puppet's own placement, then `OnSpawnSourceAnimationFinished`.

**`OnSpawnSourceAnimationFinished`:** **if no request is left (`en_ctSpawnSourceRequests` < 1): clear the two look-forcing flags, `SetState(PS_DEFAULT)`, the brain is told `OnActionFinished` and `ResetActionTime`.** Otherwise (more releases wanted): with no spawn animation think `ManuallyReleaseSpawnee` after 1e-5 s, else think `OnSpawnSourceAnimationFinished` after `max(length, 1e-5)` (the animation is played again for the next release).

So **a request count above 1 makes the source release several spawnees one after the other with one animation each**.

## 3. `Appear` and `Disappear`

Script functions of the puppet (`macAppear`, `macDisappear` in the level scripts). An entity that is already deleted is ignored.

**`Appear(skipAnimation = false)`:** **silently ignored for a puppet that is a spawner template** (`en_henSpawner` set; the console text is "SPuppet::Appear() called on a spawner.") **or a multi-spawner template** ("... on a multi spawner template."). Otherwise: when `skipAnimation` is false, the scheme animation **`Appear`** (remapped) is looked up; **if the model has it**: `AssureThatOnStepIsEnabled`, new cloned state, the animation is **played once (mode 2, speed 1, strength 1, group 3)** and **`SetState(PS_APPEARING)` (14)**; **otherwise `DoAppear` at once.**

**`DoAppear`** (also the body of the animation event `OnAnimEventAppear`; it is a 5-byte forwarder, **not empty** as `puppet-entity-methods.md` says; `CPuppetAppearAnimEvent` exists in no data file): **does nothing unless the renderable is hidden** ("called on a puppet that is already visible"); refuses with "... has mechanism what means that last called action wasn't disapear" when the puppet has a mechanism, and with "... marked as alive" when the alive flag is already set. Otherwise: the **world info is notified** (its "appeared" counter hook), **the renderable's hidden flag is cleared, the alive flag is set**, the last mechanism is restored, the brain is **enabled** (`DoEnable`), `AssureThatOnStepIsEnabled`, and the alive effects are created (`SpawnAlliveEffects`).

**`Disappear(skipAnimation = false)`:** the same scheme lookup with the animation **`Disappear`**; if present: played once (mode 2, group 3) and **`SetState(PS_DISAPPEARING)` (13)**; otherwise **`DoDisappear`**. **`DoDisappear`** (also the body of the `Disappear` animation event, 1 in the data): destroys the child entities, clears the next think, stops the alive effects; ignored for a spawner template, a multi-spawner template ("... called on a spawner / multi spawner template") and **for a puppet that is already hidden**; otherwise: the world info is notified, **the renderable's hidden flag is set**, the current mechanism is deleted, the brain **disabled** (`DoDisable`), **the alive flag cleared**, the look target cleared, the sound and effect handles released and the carried object dropped.

**The states 13 and 14 have no think routine and no hook** (`SetState` does nothing for them): **how a state ends after the `Appear` or `Disappear` animation is not resolved** (the animation event `Appear` calls `DoAppear`, which does not change the state). T01-271.

## 4. The data: spawn effect configurations (**high**)

There are **835 `CSpawnEffectConfiguration` objects: 11 in `.sfc` files** (`AlbinoVomitingZombieAgent`, `Comet`, `Ground`, `Materialisation`, `OtherDimension`, `PrinceChanMusic`, `SoulCollector`, `SpawnCapsule`, `SpawnSphere`, `Thunder`, `ZumZumFly`) **and 824 embedded in worlds** (the spawner's `ep_sfcSpawnEffectConfig`; `mechanism-classes-spawners.md` section 2). Members the puppet code reads:

| Member | `.sfc` (11) | worlds (824) |
|---|---|---|
| `sfxc_fSpawnSourceStopChance` | 0 x7, 1 x2, 0.25 x2 | 0 x729, 1 x57, 0.25 x38 |
| `sfxc_bShouldMatchThrowDir` | 0 x8, 1 x3 | 0 x769, 1 x55 |
| `sfxc_idSpawnSourcePrepareAnimation` | empty x11 | **empty x824** |
| `sfxc_idSpawnSourceAnimation` | empty x6, `Spawn` x3, `Vomit`, `Spawning` | empty x715, `Spawn` x71, `Vomit` x29, `Spawning` x9 |
| `sfxc_idSpawneeAnimation` | empty x4, `SpawnGetup` x2, `Fly_Forward` x2, `Knockdown_Getup_Back`, `Knockdown_Getup_Front`, `Getup` | `Fly_Forward` x411, `Knockdown_Getup_Front` x143, empty x111, `SpawnGetup` x73, `Getup` x47, `Knockdown_Getup_Back` x29, `Threat` x9, `Wound1` x1 |
| `sfx_fSpawnDelay` (the spawner's wait for the effect, T01-109) | 0 x5, 0.1 x2, 0.9, 0.5, 2.2, 1.0 | 0.9 x254, 0 x223, 0.5 x157, 0.1 x118, 2.2 x47, 1.0 x25 |

Consequences: **the prepare animation is never used in the data**; **a spawn-source animation exists in 109 of the 824 embedded configurations** (the rest have none: such a source releases with `ManuallyReleaseSpawnee` after the min delay); **713 of 824 embedded configurations name a spawnee animation** (the rising state is the common case: `Fly_Forward` for 411, the get-up animations for 219); **the stop chance is 0 in 729 of 824** (with the stop test `r <= chance` those sources brake before they spawn), 1 in 57 (stop at once) and 0.25 in 38. A spawner uses a puppet as source in only **7** records (`mechanism-classes-spawners.md` section 2: a legged character in 4, a turret in 2, a prop in 1); the configuration tables above are over all spawners, whatever their source.

## Invariants

1. The stop test is `r <= sfxc_fSpawnSourceStopChance`: chances 0, 0.25 and 1 are the only stored values.
2. A spawnee that has no spawnee animation (or whose model lacks it) never enters `PS_SPAWNEE_RISING`.
3. `en_ctSpawnSourceRequests` never goes below 0 through the animation event (`> 0` is required) and a request of the spawner always gives exactly one `SpawnReady`.
4. `Appear` on a visible puppet and `Disappear` on a hidden one do nothing (warnings).

## Open questions

1. **The end of the `Appear` and `Disappear` states** and the play arguments of the prepare and spawn animations: **T01-271**.
2. **The spawner's side of the launch** (`SpawnReady`, `LaunchEntity`, the spawn effect): T01-109 and T01-136.
3. **Run-time check** of the spawnee animation wait and the spawn-source chain on the running original (numbers only): **T01-272** (with T01-146).

## Test vectors

- **Stop test:** chance 0.25, draws 0.10 -> stop at once; 0.25 -> stop (`<=`); 0.26 -> brake first. Chance 1 -> always stop at once; chance 0 -> only for the draw exactly 0.
- **Brake wait:** speed 6 m/s, deceleration 20 m/s^2 -> think `SpawnSourceStopped` after 0.3 s; standing -> 1e-5 s.
- **Spawnee wait:** animation length 2.0 s, lerp 0.2 s -> `SpaweeAnimationFinished` after 1.8 s; length 0.1 s -> 1e-5 s. After it: brain with auto patrol -> state 10; no brain -> state 1.
- **Requests:** two requests, spawn animation 3 s with one release event: after the first animation ends with one request left, the animation is played again 3 s later for the second release; the state returns to `PS_DEFAULT` after the second animation.
- **`Appear`:** hidden puppet with an `Appear` animation: state 14 and the animation; `skipAnimation` true or no animation: visible at once, alive flag set, brain enabled.
