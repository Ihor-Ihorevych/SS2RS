# Puppet damage: health, armour, death, kick and score (T01-140)

- **Roadmap task:** T01-140 (the unread rest of `CPuppetEntity::OnReceiveDamage`, `OnHullTouched`,
  `ApplyDamageImpact`; `puppet-entity-classes.md` section 7 open question 1)
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **medium** throughout (every function below was read once in this build, no other
  build compared, nothing run on the original game); a few specific numbers are **low** and said so
  individually; the formulas and enum values this page builds on (`GetReceiveDamageMultiplier`,
  `EDamageType`, the parameter members) are already **high/medium** on their own pages and are not
  re-derived here, only located.

Prerequisites: [`puppet-entity-classes.md`](puppet-entity-classes.md) section 7 (the already-read
first half of `OnReceiveDamage`, `ReceivesDamageFrom`, `GetReceiveDamageMultiplier`, health pickups),
[`puppet-record-members.md`](puppet-record-members.md) (health, death and misc records; the
explosion-threshold rule), `player-rules.md` (`gpp_dmDamageMultipliers`,
`ggp_fEnemyVSEnemyDamageMultiplier`, the difficulty table), [`players-session-classes.md`](players-session-classes.md)
section 7.2 (the player's own damage-multiplier overrides), `game-modes.md` (score
per kill), [`trigger-area-rules.md`](trigger-area-rules.md) section 1 (`EDamageType`, confirmed here
a second time from `ApplyDamageImpact`'s own exemption list), [`puppet-attacks.md`](puppet-attacks.md)
section 7 (the melee kick, open question "twice"), [`player-movement-constants.md`](player-movement-constants.md)
(fall height without damage, 2.5).

## Summary

1. **The order of a hit, in `OnReceiveDamage`**, after the already-documented player-cheat/invulnerability
   and shield-tool checks: compute a scaled integer damage amount (the enemy-vs-enemy and the
   co-op/difficulty multiplier enter here, never later); update bookkeeping (sniper multiplier,
   accumulated-damage counter, force-impulse accumulator) using a mix of the *original* and the
   *scaled* amount; if the puppet was **already** dead when the call started, only the bookkeeping
   above had any effect — the new hit cannot lower health/armour further and the function either
   triggers a delayed explosion, returns quietly, or logs a warning; otherwise it records the hit,
   fires a one-shot script event, applies the physical kick, and only then changes health/armour by
   the **armour-absorption formula** (section 2); if that brought health to 0 or below, it records
   the death, exits any ride, calls `DoDying`, and — if the dealer is itself a puppet — hands kill
   credit to the dealer's player brain.
2. **Armour absorbs two thirds of every hit, health the other third** (exact integer rounding in
   section 2); once armour cannot cover its two-thirds share, the shortfall spills onto health, so
   the *total* of health+armour lost always equals the scaled damage exactly.
3. **The co-op divisor (`GetReceiveDamageMultiplier`) and the enemy-vs-enemy multiplier are both
   applied to the damage amount inside `OnReceiveDamage`**, before the armour split, and in that
   order; the **accumulated-damage counter that decides whether a puppet explodes** instead tracks
   the **original, pre-multiplier** amount, not the scaled one.
4. **The sniper-damage multiplier's reader is found**: `CMiscPupeptParams.mpp_fSniperDamageMultiplier`
   (`puppet-record-members.md` section 7, previously "reader not found") is applied in `OnReceiveDamage`
   itself, to the already co-op/difficulty-scaled amount, only for hits of type `DMT_SNIPER_BULLET`.
5. **No fall-height or fall-speed damage formula was found** in `CPuppetEntity::OnHullTouched`,
   `ApplyDamageImpact` or `ApplyMaterialDamage`, all read in full for this page, nor in the one
   `CLeggedPuppetEntity::OnHullTouchedInternal` override (gated on a pose that is not the walking/falling
   one). This is a **negative result, not a proof of absence** — see Open questions.
6. **Material (touch) damage** is a separate per-step function, `ApplyMaterialDamage`, not called from
   inside `OnHullTouched`; it reads the touched material's own configured amount and period and
   re-enters the normal damage pipeline.
7. **A "kicker" (world launcher) touch inflicts real damage**, type 7 `DMT_KICKING`, built and
   dispatched from inside `OnHullTouched` itself, on top of its own physics launch impulse.
8. **`ApplyDamageImpact`'s exemption list (types 24, 9, 14, 15) matches `trigger-area-rules.md`'s
   already-published list exactly** (killer, falling, heat, cold never produce a kick) — an
   independent second reading of the same fact.
9. **The melee "twice" question of `puppet-attacks.md` section 7 is resolved for the non-deathmatch
   case**: the melee code's own body impulse and the damage pipeline's own impulse (through the
   running force accumulator that `ApplyDamageImpact`, almost certainly, reads) are two different
   impulses, both applied — **except in deathmatch**, where `OnReceiveDamage` skips feeding the
   accumulator at all, so only the melee code's own impulse lands.
10. **Score is not computed inside `OnReceiveDamage`.** Its only contribution is the kill-credit
    hand-off: on a kill, if the dealer is a puppet, the dealer's own player brain is notified
    (`OnIKilledPuppet`); `GetKillScore`/`ReceiveScore` happen inside that call, not read here.

## Evidence

- **Read in full** (`Sam2Game.dll`, the same build as the other T01 puppet pages; no other build
  compared; nothing run): `CPuppetEntity::OnReceiveDamage` (the part after the shield-tool check —
  the part before it is already documented in `puppet-entity-classes.md` section 7),
  `CPuppetEntity::ApplyDamageImpact`, `CPuppetEntity::OnHullTouched`, `CPuppetEntity::ApplyMaterialDamage`,
  `CPuppetEntity::GetPiercingProjectileResilience` and `ForcesPiercingProjectileToExplode` (read again,
  to cross-check which members are the health and armour getters: both already-published as
  `trunc((armour + health) × multiplier)` in `puppet-record-members.md`, which only holds if the
  two getters this page leans on for the armour formula are indeed health and armour — confirmed by
  this match), `CLeggedPuppetEntity::OnHullTouchedInternal` (read to check for a fall-height rule; it
  is gated on a pose value that is not the ordinary ground/fall pose and does not contain one).
- **Not found in this build**: an exported, non-empty `HandleFallingDamage` for `CPuppetEntity` or
  `CLeggedPuppetEntity` (searched by name across the whole decompiled tree of `Sam2Game.dll`); a
  generic per-`EDamageType` health-damage multiplier table lookup inside `OnReceiveDamage` itself (the
  only per-type health-damage scaling found there is the sniper multiplier).
- **Cross-checks.** The `ApplyDamageImpact` exemption list (types 24, 9, 14, 15) was compared against
  `trigger-area-rules.md` section 1, read independently by a different task: **exact match**. The
  bullet/sniper-bullet/slug accumulation-skip condition (types 19–21, gated by
  `pdp_bDisableBulletDamageAccumulation`) was compared against `puppet-record-members.md` section
  5.1: **exact match**. The health/armour getters used by the armour-absorption formula were
  cross-checked against `GetPiercingProjectileResilience`'s already-published formula: **consistent**.
- **Not checked / not resolved.** The exact float values of two per-type impulse-scale constants in
  `ApplyDamageImpact` (candidates for entries of `gpp_dmDamageMultipliers`/`CDamageMultipliers`, but
  not read from the data section in this pass); which of `ggp_fEnemyVSEnemyDamageMultiplier` or
  `gpp_dmDamageMultipliers`'s "puppet vs puppet"/"non-player vs model" entry the enemy-vs-enemy branch
  actually reads (both are documented as the value 6, so the *number* is not in doubt, only the
  *name*); the exact virtual method names behind two call points described here by behaviour only
  (the deathmatch-gated notification hook in step 4, and the kick-applying hook in step 6 of section
  1 — candidates `OnDamageTaken` and `ApplyDamageImpact` respectively, by elimination and signature
  match, not by a confirmed slot-to-name table); the damage *type* field of the material-damage
  record built by `ApplyMaterialDamage` (every other field was set explicitly; this one was not, and
  its default was not traced); where `OnKicked`/`en_tmLastPuppetKicked`/`en_tmLastNonPuppetKicked` are
  actually written (not inside any function read for this page — likely the melee code of
  `puppet-attacks.md`, out of this page's scope). None of this was run on the original game.

## Layout / rules

### 1. The order of operations in `OnReceiveDamage` (medium)

Continuing from where `puppet-entity-classes.md` section 7 left off (host-only; the player
god-cheat/invulnerability/rider-seat checks; the mechanism-part shield-tool redirect, which returns
before any of the below ever runs):

1. **Dealer and alignment gate, and the two damage-amount multipliers.** With a valid dealer that
   differs from this puppet and from its own last-dealt-to-self guard, and unless the amount is the
   "kill" sentinel (2,147,483,647): the puppet's `ReceivesDamageFrom`-style gate runs (already
   documented); if it refuses, the hit is dropped entirely. Otherwise, **if both the dealer and this
   puppet are alignment `BA_EVIL` (1)**, the amount is scaled by the **enemy-vs-enemy damage
   multiplier** — data value **6** (`player-rules.md`'s `ggp_fEnemyVSEnemyDamageMultiplier`, gated by
   `gpp_bEnemiesCanFightEachother`; the structurally identical `gpp_dmDamageMultipliers` "puppet vs
   puppet"/"non-player vs model" entries carry the same value 6, so which exact member is read here
   is not disambiguated — the number is not in doubt). Then, unless the amount is the kill sentinel,
   the amount is run through the puppet's own virtual **`GetReceiveDamageMultiplier`** — for a
   generic evil puppet in co-op, the already-published formula of `puppet-entity-classes.md` section
   7; for a **player** puppet, the overridden version is the **difficulty's `dif_fPlayerDamageMultiplier`**
   in a non-deathmatch session, or **1.0** in deathmatch (`players-session-classes.md` section 7.2) —
   **the player's own incoming-damage multiplier does not depend on the number of co-op players at
   all**, only on difficulty. The result is then **floored to a minimum of 1** (never zero, never
   negative, unless it started negative — see below). Call this scaled amount **D**.
2. **Bookkeeping, on the sign of D:**
   - **D is negative (a healing record, `puppet-record-members.md` section 3.2's `-n` convention):**
     the puppet's healing-accumulator member adds the *original* (pre-multiplier) amount's magnitude.
     None of the sniper, accumulation or force-impulse bookkeeping below runs. Health alone is raised
     later (step 5); armour is never touched by a negative amount.
   - **D is zero or positive:**
     - If the hit's damage type is **`DMT_SNIPER_BULLET` (20)**, D is further scaled by the
       *victim's own* `CMiscPupeptParams.mpp_fSniperDamageMultiplier` (default 1.0; 0.25/0.5/0.75 in
       12 of 126 files — `puppet-record-members.md` section 7, whose reader this resolves).
     - A virtual notification call fires **only when the session is deathmatch-type**; its return
       value is discarded. The best candidate by elimination is `OnDamageTaken` (an already-documented
       override point for the player puppet, `players-session-classes.md` section 3); its purpose
       beyond notification was not resolved here (**low**).
     - Unless the death parameters set `pdp_bDisableBulletDamageAccumulation` **and** the hit's type
       is one of `DMT_BULLET` (19), `DMT_SNIPER_BULLET` (20) or `DMT_SLUG` (21): the **original**
       (pre-multiplier) amount is added to the accumulated-damage counter that the explosion check
       (step 3) and `DoDying` read — **or**, if the original amount was the kill sentinel, the
       counter is set to 1,000,000 outright instead of adding to it (both already published in
       `puppet-record-members.md` section 5.1; confirmed again here as the same counter `OnReceiveDamage`
       itself maintains).
     - **Outside deathmatch only**, the hit's own force-impulse vector (the three components a
       `CDamage` record carries alongside its direction and position) is added to a running
       per-puppet force accumulator. **In deathmatch this step does not run at all** — see section 5
       for why this matters for the melee "twice" question.
3. **Already-dead check (health as it stood *before* this hit).** If the puppet's current health is
   0 or below when the call reaches this point (a second hit landing on an already-dying puppet in
   the same tick, or simply a repeat call): **the new hit's health/armour are never touched** — only
   its contribution to the accumulated-damage counter in step 2 already happened. Then: if the death
   parameters say the puppet explodes, the local flag `PLF_EXPLODED` is not yet set, and the
   accumulated-damage counter has now reached the exploding-damage threshold, the puppet enters the
   exploding state (`puppet-record-members.md` section 5.1) and the function returns. Otherwise, if
   the puppet is not alive (`en_bAlive` clear) it returns quietly; if it is somehow still alive with
   non-positive health it logs the diagnostic `CPuppetEntity::OnReceiveDamage(): Health is below 0,
   but puppet is not marked as dead.` and returns either way. **Everything below this point in this
   section only runs when the puppet was still alive and above 0 health when the call started.**
4. **Record the hit and apply the kick, before changing health/armour.** When D is strictly positive:
   the puppet's "last damage" members (time, amount, direction/position, dealer handle) are updated;
   a one-shot script macro event fires if and only if the local flag `PLF_REPORT_DAMAGE` (16) is set,
   and that flag is cleared immediately after (so it fires at most once per time the flag is
   re-armed by a script — the likely "damage effect" notification the roadmap line names; the exact
   event name was not resolved, **low**). A virtual hook is then called unconditionally with the same
   `CDamage` handle; by elimination (same handle, and the only other virtual override whose whole
   purpose is to push the body from a `CDamage`) this is almost certainly `ApplyDamageImpact`
   (section 4) — **medium**, not a confirmed slot-to-name match.
5. **Change health and armour.** See section 2 for the exact formula. In short: a kill-sentinel amount
   zeroes both outright; a negative D raises health only (through the setter — armour's setter is
   never called for a negative amount); otherwise the armour-absorption split runs. The health and
   armour setters are each called only when the computed value actually differs from the stored one.
6. **Death and kill credit.** If, after step 5, health is now 0 or below: the puppet's death damage
   type and current placement are recorded (already documented as `en_dmtDeathDamageType`/
   `en_qvDeathPlacement` in `puppet-entity-classes.md`'s member table — this page locates *where*
   they are written); the puppet leaves any ride it is in first, then `DoDying` runs (its own rules
   are `puppet-entity-classes.md` section 8, unchanged by this page). If the dealer is itself a
   puppet entity (as opposed to the world, an environment hazard, or no dealer), its own operating
   player brain — if it has one — is notified through `CPlayerBrainEntity::OnIKilledPuppet`. This is
   the only scoring-related call this function makes; `GetKillScore` and `ReceiveScore` are inside
   that call and were not read for this page.

### 2. The armour-absorption formula (medium-high: the arithmetic itself was read and checked by hand)

Given the scaled integer damage **D** (≥ 0, from section 1), the current health **H** and armour
**A**:

```
armorShare  = floor(D × 2 / 3)      // truncating integer division, as in C
healthShare = D − armorShare        // the remainder; armorShare + healthShare == D exactly

if A >= armorShare:
    newArmour = A − armorShare
    newHealth = H − healthShare
else:                                 // armour cannot cover its own two-thirds share
    shortfall = armorShare − A
    newArmour = 0
    newHealth = H − healthShare − shortfall   // == H − (D − A)
```

So **armour absorbs two thirds of a hit and health the other third**, until armour is exhausted, at
which point the part armour could not cover spills onto health — the **total** of health+armour lost
is always exactly D, never more, never less. The kill-sentinel amount (2,147,483,647) bypasses this
formula entirely and sets both health and armour to 0 directly; a negative D (healing) raises health
by `−D` through the setter alone, clamped to the already-documented rules (maximum, and half-maximum
in deathmatch unless forced: `puppet-entity-classes.md` section 7).

### 3. Where each multiplier enters (summary table)

| Multiplier | Value / source | Applies to | Where |
|---|---|---|---|
| Enemy-vs-enemy (evil hits evil) | 6 (`ggp_fEnemyVSEnemyDamageMultiplier`, data) | the damage amount | `OnReceiveDamage`, before the co-op/difficulty multiplier |
| Co-op / extra-enemy-strength (`GetReceiveDamageMultiplier`) | the already-published formula, for a generic **evil** puppet only | the damage amount, floored to a minimum of 1 | `OnReceiveDamage`, right after the enemy-vs-enemy step |
| Difficulty (`dif_fPlayerDamageMultiplier`), the **player's** override of the same virtual call | 0.2 (Tourist) to 2 (Serious), 1 in deathmatch | the damage amount to a **player** puppet; **no player-count term** | same call site, dispatched virtually to the player's own override |
| Sniper (`mpp_fSniperDamageMultiplier`) | the victim's own parameter, 0.25–1.0 | the damage amount, only for `DMT_SNIPER_BULLET` hits | `OnReceiveDamage`, after the co-op/difficulty step |
| "Serious damage" power-up (`GetApplyDamageMultiplier`) | 4.0 (SP/co-op) / 2.5 (deathmatch) | the **dealer's outgoing** damage, before it ever reaches `OnReceiveDamage` | not in this function (already documented, `players-session-classes.md` 7.2) |
| Per-type impulse scale (two constants, exact values unresolved) | candidates for `gpp_dmDamageMultipliers`/`CDamageMultipliers` entries | the **physical kick**, not health/armour | `ApplyDamageImpact` only (section 4) |

**`CDamageMultipliers` (`gpp_dmDamageMultipliers`) was not found applied to the health-damage amount
anywhere in `OnReceiveDamage`.** The only place a per-`EDamageType` scale was found at all is the
impulse magnitude inside `ApplyDamageImpact` (section 4), where two different constants are used for
`{DMT_EXPLOSION (11), DMT_IMPACT (8)}` and for `{DMT_DROWNING (13), DMT_CUTTING (3)}` — a pairing that
is consistent with, but does not confirm, `gpp_dmDamageMultipliers`'s documented `cutting 0.25` /
`explosion 1.5` entries (the exact floats were not read from the data section). If `CDamageMultipliers`
really does scale incoming health damage by type, it must be applied by the weapon/attack code before
the `CDamage` record ever reaches `OnReceiveDamage` (out of this page's scope), not inside the puppet's
own damage entry point.

### 4. `ApplyDamageImpact` — the physical kick (medium)

Skipped entirely, before anything else, for damage types **`DMT_KILLER` (24)**, **`DMT_FALLING` (9)**,
**`DMT_HEAT` (14)** and **`DMT_COLD` (15)** — this list is read independently here from
`ApplyDamageImpact`'s own code and **matches `trigger-area-rules.md` section 1's already-published
list exactly**, a second confirmation of that fact. For every other type:

- the hit's raw amount is clamped into an impulse-magnitude range of **0 to 5,000**;
- for `DMT_EXPLOSION` (11) and `DMT_IMPACT` (8) this is scaled by one constant, for `DMT_DROWNING`
  (13) and `DMT_CUTTING` (3) by a different one (section 3; exact values unresolved), and left
  unscaled for every other type;
- once the puppet is **already dead** (a corpse), a fixed, much smaller scale (≈ 66.7) replaces the
  live-computed one, so corpses react consistently regardless of what killed them;
- the function maintains a **running force vector** on the puppet (fed, outside deathmatch, by
  `OnReceiveDamage`'s own force-impulse bookkeeping of section 1 step 2): each call **removes** the
  physics impulse that corresponded to the vector's value *before* this hit and **applies a fresh
  one** for its value *after* — so several hits landing in the same simulation step compose into one
  physically consistent push instead of stacking additively, and a hit whose vector ends up
  unchanged produces no net impulse at all.

### 5. The melee "twice" question (`puppet-attacks.md` section 7), resolved for non-deathmatch (medium)

`puppet-attacks.md` left open whether a melee victim takes the blast's impulse twice: once from
`CBody::AddImpulse` applied directly by the melee code, and once more because the same impulse is
carried inside the `CDamage` record's own force-impulse fields. This page's reading answers it:

- **Outside deathmatch:** yes. `OnReceiveDamage` adds the record's force-impulse into the running
  accumulator (section 1 step 2), and the kick-applying hook of section 1 step 4 (almost certainly
  `ApplyDamageImpact`, section 4) turns that running accumulator into a *second*, independent physics
  impulse. The melee code's own direct `AddImpulse` and this second one are both real, separate calls
  to `CBody::AddImpulse`.
- **In deathmatch:** no (or at least, not from this mechanism) — `OnReceiveDamage` does not feed the
  force-impulse accumulator at all in a deathmatch-type session (section 1 step 2), so the only
  impulse that lands is the melee code's own direct one.

This does not resolve the **hull-category** half of that same open question (`puppet-attacks.md`
open question 2, T01-261), which is unrelated to the puppet's own damage code.

### 6. `OnHullTouched`: material damage, falling damage and world "kickers" (medium)

`OnHullTouched`, read in full, is mostly **bookkeeping**: for each hull touch it updates which
floor/wall/material the puppet is in contact with this step, and tracks the **strongest touch of the
step** (the contact whose relative-velocity-against-normal is the largest seen so far this step —
`en_hhuStrongestTouch`/`en_vStrongestTouchNormal` of `puppet-entity-classes.md`'s member table), and
sets `en_bTouchedAnything` at the very start of every call. Two things in it actually deal damage:

**6.1 World "kickers" (medium).** When the touched object/material carries a pair of flags that mark
it as a launcher ("kicker"), `OnHullTouched` computes a launch direction from the kicker's own
placement angles blended with the puppet's relative velocity against the surface, applies a physics
impulse to the puppet's body for it (debug-tagged `"Kicker launch impulse"` when the debug-dump
config is on and watching this entity), and **separately** builds a fresh `CDamage` record of type
**7 `DMT_KICKING`** with an amount computed from the kicker's own configuration, then dispatches it
back through the same entry point a normal hit uses — so a kicker touch goes through the full
multiplier/armour-split pipeline of sections 1–2 like any other hit, on top of its own impulse.

**6.2 Material (touch) damage — a separate per-step function, `ApplyMaterialDamage` (medium).** Not
called from inside `OnHullTouched` itself; it runs once per step and reads the data `OnHullTouched`
left behind. It fires only when **all** of: the puppet touched something this step
(`en_bTouchedAnything`); the touched material object has a non-zero configured damage amount;
`CanReceiveDamageFromMaterial` (already named in `players-session-classes.md`'s method list) allows
it; and enough time has passed since the last tick of this same material (a material-specific period,
tracked by a per-puppet "last damage applied by material" clock — `en_tmLastDamageAppliedByMaterial`).
When all hold, it builds a fresh `CDamage` record with the material's own configured amount, directed
**away from the surface** (the negated floor normal), no dealer, and dispatches it through the normal
entry point exactly like the kicker above; its damage *type* was not explicitly set in this reading
(every other field was) and its effective default was not traced (**low**). The clock is stamped
immediately after dispatch.

**6.3 Falling damage: not found (medium — a negative result, see Open questions).** `OnHullTouched`,
`ApplyDamageImpact` and `ApplyMaterialDamage`, all read in full above, contain **no fall-height or
fall-speed-derived damage calculation**. `CanReceiveFallingDamage`/`CanReceiveRangeDamage` simply
answer "yes" from a shared stub (already documented); the player's own override additionally refuses
falling damage while "serious jump" is active (`players-session-classes.md` 7.2); `HandleFallingDamage`
has no body for `CPuppetEntity` in this build (already noted as an open question in
`puppet-entity-classes.md`, confirmed here by failing to find *any* exported, non-empty override by
name across the whole decompiled tree of `Sam2Game.dll`); the one `CLeggedPuppetEntity::OnHullTouchedInternal`
override that exists is gated on a pose value that is not the ordinary ground/falling pose (it reads
as the diving/swimming touch handler, consistent with the "Diving" scheme sound already documented)
and contains no fall-height logic either. `player-movement-constants.md`'s **fall height without
damage of 2.5** (`ma_fAllowedFallHeight`) and `puppet-record-members.md`'s **`php_ghFallingHitDamage`
graph** (a step function: input 20 gives 0, input 60 gives 100, in 29 of 126 health-record files)
remain data with no confirmed reader.

## Invariants

1. The armour-absorption split (section 2) always removes exactly D from the health+armour total,
   never more or less, for any D ≥ 0 and any non-negative starting health/armour (checked by hand on
   the examples of the Test vectors; not checked by a script against the data, since this is pure
   code arithmetic with no data-side invariant to reproduce).
2. The accumulated-damage counter that gates an explosion (`puppet-record-members.md` 5.1) is driven
   by the **original**, pre-multiplier damage amount, never the scaled one computed in section 1 —
   so a hit that the co-op or sniper multiplier shrinks to a smaller health loss still counts its
   *full, unscaled* amount towards an exploding puppet's threshold.
3. `ApplyDamageImpact`'s four exempted damage types (24, 9, 14, 15) are exactly
   `trigger-area-rules.md`'s already-published list — checked by comparing the two independently
   written lists, not by a script (both are prose read from code, not data).
4. A puppet whose health is already 0 or below when `OnReceiveDamage` is called a second time in the
   same tick never has its health or armour changed by that second call; only the accumulated-damage
   counter can still move (and only if the hit's type is not one of the three accumulation-skip
   kinds).

## Open questions

Ranked by what blocks an implementer.

1. **Falling damage may simply not be implemented for puppets in this build**, or it lives somewhere
   this reading did not reach (an engine-level physics callback outside `Sam2Game.dll`, or code this
   page's name search missed because it shares a body with another empty virtual override and so
   carries a different exported name). A reviewer should re-run the whole-tree name search with a
   wider net (anything that reads `ma_fAllowedFallHeight`/`en_fFallHeightAllowed`/
   `en_fCustomFallHeightAllowed`, or that writes a damage record from inside `OnStep`) before an
   implementer assumes "no fall damage" is correct. Follow-up: a new T01 task, or fold into T01-141
   (the legged step) since that is where any such formula most plausibly still hides.
2. **The two `ApplyDamageImpact` impulse-scale constants' exact values**, and whether they are really
   `gpp_dmDamageMultipliers` entries or an unrelated, impulse-only tuning table: read the raw floats
   from the data section and compare against the documented `cutting 0.25`/`explosion 1.5`/`kicking 2`
   values. **T01-162** or a live capture (see below).
3. **Two call points named by elimination, not by a confirmed slot**: the deathmatch-only
   notification hook (candidate `OnDamageTaken`) and the kick-applying hook (candidate
   `ApplyDamageImpact`) of section 1. A second reading that resolves the vtable slot to its exported
   name directly (rather than by signature/behaviour match) would raise both from medium to high.
4. **The material-damage record's damage type** was not explicitly set in the reading of
   `ApplyMaterialDamage`; its effective default (whatever the constructor leaves it as) was not
   traced.
5. **`en_tmLastPuppetKicked`/`en_tmLastNonPuppetKicked` and `OnKicked` itself** are not written by
   any function read for this page; they most likely belong to the melee attack code of
   `puppet-attacks.md` (out of this page's scope) or to `CLeggedPuppetEntity`'s own override.
6. **Live confirmation (T01-146, already listed by `puppet-entity-classes.md`), extended by this
   page**: the oracle harness can now read a player's health and armour live
   (`docs/ORACLE_HARNESS.md`). A capture that fires a known, single-type hit (ideally a world kicker
   or a scripted `InflictDamage` of a fixed amount and type) at a player of known health/armour, on
   Normal and on Serious, solo and in a 2+ player co-op session, and reads the resulting health/armour
   back, would directly confirm or refute: the armour-absorption formula of section 2; that the
   player's own received-damage multiplier really has no co-op player-count term (test vector A
   below); and the sniper multiplier's existence and value for a chosen character. A second capture
   that lets an accumulating-damage puppet survive many small hits and then one big one, reading its
   health each time, would confirm the explosion-threshold accounting of invariant 2.
7. **Which exact member (`ggp_fEnemyVSEnemyDamageMultiplier` vs. a `gpp_dmDamageMultipliers` entry)**
   the enemy-vs-enemy branch of section 1 step 1 reads: both are documented as the value 6, so no
   test vector can currently distinguish them without reading the raw bytes.

## Test vectors

All computed by hand from the formulas of sections 1–2; none of these were run on the original game
(see Open questions item 6 for how a live capture would confirm them).

**A. The roadmap's own example — a 50-damage bullet (`DMT_BULLET`, type 19) on a player with 100
health and 50 armour, Normal difficulty:**

- **Solo (single player):** the player's own `GetReceiveDamageMultiplier` override reads only the
  difficulty (`dif_fPlayerDamageMultiplier(Normal)` = **1.0**, `player-rules.md`) — no co-op term
  applies to a player's *own* received damage. D = 50 × 1.0 = **50**.
  `armorShare = floor(50×2/3) = 33`, `healthShare = 50 − 33 = 17`. Armour (50) covers its share:
  **new armour = 17, new health = 83.**
- **4-player co-op, same difficulty:** the formula above has **no player-count term for a player
  puppet** — D is still **50**, and the result is **identical: armour 17, health 83.** This is the
  page's own answer to "where the co-op divisor is applied for a player": it is not applied to a
  player's own received damage at all; the co-op player-count term only reduces the damage an **evil**
  puppet (an enemy) receives, making enemies tougher as more players join, never making players take
  less damage.

**B. The same 50-damage, non-bullet hit (so the accumulation-skip condition does not interfere) on an
*evil* puppet with 100 health and 50 armour, to show where the co-op divisor actually bites:**

- **Solo (P = 1), no extra enemy strength (E = 0):** `GetReceiveDamageMultiplier` = 1.0 (the formula's
  own already-published test vector). D = 50. Same split as above: **armour 17, health 83.**
- **4-player co-op (P = 4), extra enemy strength 50, enemy strength per player 25** (the same example
  numbers `puppet-entity-classes.md` already uses): `m = 1/(1+0.01×(4−1)×25) × 1/(1+0.01×50)
  = 1/1.75 × 1/1.5 ≈ 0.381`. D = floor(50 × 0.381) = **19** (floored to at least 1, well above it).
  `armorShare = floor(19×2/3) = 12`, `healthShare = 19 − 12 = 7`. **New armour = 38, new health = 93**
  — much less lost than solo, because of the co-op toughness scaling (the generic enemy formula, not
  anything player-specific).

**C. Armour exhausted — 100 health, 10 armour, 50 damage (post-multiplier), any type:**

`armorShare = 33`, `healthShare = 17`. Armour (10) cannot cover 33: `shortfall = 33 − 10 = 23`.
**New armour = 0, new health = 100 − 17 − 23 = 60.** Check: 10 (armour) + 40 (health) = 50 = D exactly.

**D. Sniper hit — a puppet with `mpp_fSniperDamageMultiplier` 0.5, 0 armour, hit for the base sniper
damage of 75 (`hitscan-shot.md`), outside deathmatch, no other multiplier applying:**

D after the sniper multiplier = floor(75 × 0.5) = **37**. `armorShare = floor(37×2/3) = 24`,
`healthShare = 13`. No armour to absorb: shortfall = 24. **Health loses the full 37** (13 + 24),
exactly D, since there was nothing to absorb it.

**E. Kill-sentinel amount (2,147,483,647) of type `DMT_ABYSS` (25) on an invulnerable player:** per
`puppet-entity-classes.md`'s own test vector, invulnerability does **not** block this (only
`DMT_KILL_ALL_CHEAT`/`DMT_SERIOUS_BOMB` respect it). Health and armour are both set to **0** directly
(the armour-absorption formula is bypassed); the accumulated-damage counter is set to **1,000,000**
rather than adding to it.

**F. Healing tick, amount −10 (a damage record `ApplyHealing` builds, `puppet-record-members.md`
3.2):** D = −10. The healing accumulator gains **10** (the magnitude of the *original* amount); later,
health alone is raised by 10 through the setter (clamped to maximum, and to half-maximum in
deathmatch unless forced); armour's setter is never called.
