# Player rules and global game parameters (T01-027)

- **Roadmap task:** T01-027 (the difficulty table also serves T01-037)
- **Author:** @lxstyo/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** that the values below are what the files contain (read from `GlobalGameParameters.rsc`, `PlayerPuppet.ep` and the `Items/` parameter files); **medium** for the role of members that have readable names; **low** for the members that are only numbered; **open** for everything that lives in game code: pickup amounts, ammunition caps, damage-over-time rules, respawn rules, Netricsa behavior.

Prerequisites: `rsc-ctl.md` / `rsc-ctl-schema.md` (the global databases), `ep.md` / `ep-schema.md` (puppet and item parameters), `help-digest.md`, `ai.md` (difficulty also changes AI).

## Summary

Most of the rules that define how the player and the pickups behave are **numbers in two databases**: the global game parameters (`Content/SeriousSam2/Databases/GlobalGameParameters.rsc`) and the player puppet parameters (`…/EntityParams/PlayerPuppet.ep`, plus `PlayerPuppet_DM.ep` for multiplayer), and in the per-item parameter files under `Items/`. This page records them. The **amounts** a pickup gives, the **ammunition caps** and the rules for dying and respawning are *not* in the data files (they are not among the members of any item, player or global class); they have to be measured (T01-038/039). Where the 2006 editor help states an amount, it is quoted with the warning that **it disagrees with the shipped item files in several cases**.

## Evidence

Read-only decoding of the files named above with the decoder of `ctsemeta-objects.md` (class members are in `rsc-ctl-schema.md` and `ep-schema.md`); item files are the 51 `.ep` files under `Items/` of the base install, their world entities were read in the five smallest worlds. Nothing from binaries.

## Global parameters (`CGlobalGameParams`)

| Member | Value | Notes |
|---|---|---|
| `ggp_gppPlayerParams`: `gpp_fFOV` | **80** (degrees) | default horizontal field of view |
| `gpp_bAutoJumpSP` / `…DM` | 1 / 0 | auto-jump on in single player, off in multiplayer |
| `ggp_slExtraLifeScore` | **10,000** | an extra life per 10,000 points |
| `ggp_slSeriousScoreMultiplier` | **5** | score multiplier while the "serious score" power-up is active |
| `ggp_wfGiveAllWeaponsFlags` | `0xBFFF` | the weapon set of the "give all weapons" cheat: 15 of 16 weapon bits |
| `ggp_cwuComboWeaponsUsing` | 2 | how the combo weapons (dual wielding) are used; enumeration names not in the data |
| `ggp_fTimeToChooseLeftWeapon` | 3 | seconds for choosing the left weapon (`changeleftweapon.tfx` shows the message) |
| `ggp_fWeaponItemStayTimeAfterDrop`, `ggp_fPowerUpItemStayTimeAfterDrop` | 10, 10 | seconds a dropped weapon or power-up stays |
| `ggp_fBeaconAndObjStayTime` / `…FadeOutTime` / hot spot / fall-off | 4 / 3 / 20 / 40 | navigation beacon and objective markers |
| `ggp_fSeriousBombGlarePeriod` | 1 | the serious bomb's white-out |
| `ggp_fUsageDistance` | 4 | how far the "use" action reaches |
| `ggp_fRayFromViewDistance` | 1,000 | the aim ray length from the view |
| `ggp_bpBulletParams` | default bullet velocity 80, reach 160, force impulse 250, up-impulse ratio 0.5, ray diameter 1; static trail life 0.25 s, widths 0.02 to 0.04; moving trail length 10, velocity 30, width 0.1 | defaults for hit-scan bullets with a visible trail |
| `gpp_dmDamageMultipliers` | cutting **0.25**, explosion **1.5**, kicking **2**, non-player vs. model 6, puppet vs. puppet 6 | global damage-type multipliers |
| `gpp_bEnemiesCanFightEachother` / `…AllEnemies…` / `ggp_fEnemyVSEnemyDamageMultiplier` | 1 / 0 / 6 | enemies hurt each other, six times as much |
| threat and sight distances | projectile 100, weapon fire 100, puppet 100; standard see 100, short-sighted 40; character FOV 360 | `ai.md` |

### Power-up durations (seconds)

| Power-up | Single player (`ggp_pudPowerUpDurations`) | Multiplayer (`…DM`) |
|---|---|---|
| Serious strength | 60 | 40 |
| Serious speed | 60 | 20 |
| Serious jump | 60 | 40 |
| Serious damage | 60 | 30 |
| Serious score | 60 | 40 |
| Invulnerability | 60 | 30 |
| Invisibility | 60 | 30 |

The speed power-up multiplies the player's speed by 2 (`ppp_fSpeedPowerUpMultiplier`); the help file also says damage ×4, jump multiplier, score ×5 (the 5 above), strength increased: only the speed and score numbers are in the data. A `CPowerUpEffect` database (`PowerUpEffects.rsc`) pairs each power-up index (1, 3, 5, 6 have effects) with a screen material and a sound.

### Difficulty tables (`ggp_gdGameDifficulties`, PC; the Xbox table follows)

| Level | Enemy health | Enemy speed | Think frequency | Player damage | Health recharges | Initial delay | Auto-aim |
|---|---|---|---|---|---|---|---|
| Tourist | 0.5 | 0.75 | 0.75 | 0.2 | **yes** | 1.5 | 1.25 |
| Easy | 1 | 0.75 | 0.75 | 0.5 | no | 1.25 | 1.15 |
| Normal | 1 | 1 | 1 | 1 | no | 1 | 1 |
| Hard | 1 | 1.125 | 1.125 | 1.5 | no | 0.5 | 0.9 |
| Serious | 1 | 1.25 | 1.75 | 2 | no | 0.25 | 0.8 |

(Multipliers; the member is `dif_fPlayerDamageMultiplier`: easier levels have the smaller values, so it is read as the damage the **player takes**, 0.2 in Tourist and 2 in Serious (inference). The Xbox table is gentler: Tourist as above with auto-aim 1.2; Easy 0.5 health, speed 0.75, think 0.75, damage 0.5, delay 1.5, auto-aim 1.1; Normal 0.75, 0.9, 0.75, 0.75, 1.25, 1; Hard 1, 1, 1, 1.5, 1, 0.9; Serious 1, 1.125, 1.125, 2, 0.5, 0.8.) "Think frequency" is the multiplier of the AI think rate (`ai.md`), "initial delay" scales the delay before an enemy reacts (`ggp_fDefaultInitialDelay` is 0.5), and a graph (`ggp_ghAutoAimingAngle`) gives the auto-aim cone.

## The player puppet (`PlayerPuppet.ep`)

`SPlayerPuppetParams` over `SLeggedPuppetParams` and `SPuppetParams`:

- **Health**: `SPuppetParams.1` has `.1 = 100` and `.2 = 100` (health and a second value, both 100; the second is presumably the maximum or the armor), no healing (`php_slHealingStart/End/PerSec` all 0), a health-bar size 2.
- **Camera and view**: 3rd-person cameras `3rdPersonCamera_Close`, `3rdPersonRideCamera`, `3rdPersonDeathCamera` (`Cameras.rsc`), view damping spring (K 160, D 10), maximum move banking 0.25 and side-step banking 1, auto-centre after 0.5 s with multiplier 3, an added pitch of 10°, view heights crouch 0.7, swim 1.5, dive 1.1, keep-breath period **60 s** (drowning starts after it).
- **Freezing**: freeze time 60 s in both `PlayerPuppet.ep` and `PlayerPuppet_DM.ep` (the "30 in multiplayer" of an earlier reading of this page was `Databases/Puppets/Player/Player.ep`, a third, unused-by-the-live-class file with its own value of 30; see [`player-movement-constants.md`](player-movement-constants.md)), unfreeze 5 s, 1.5 damage per second while frozen, exhale period 3 s.
- **Jumping and slopes**: the legged body has a jump-up velocity of **10** and a maximum slope for jumping of 60°, look-around limits of 60° horizontally and 40° vertically; step-up height 2 and step-down 8 and an allowed fall height of 2.5 in the generic moving abilities. The ground speed, acceleration/deceleration, air control defaults, eye heights and the multiplayer differences are in [`player-movement-constants.md`](player-movement-constants.md) (T01-410); the collision shape and a general walkable-slope limit are still open there.
- **Object manipulation**: the player can carry objects up to mass **201** (1,000 with the "proteins" effect), grab velocities 10 to 20, throw velocity 1 to 50.
- **Kicking and knock-out**: kicking is disabled for the player; a knock-out sets impulse 10,000 and sleep 2 s but is disabled.
- **Death**: `SPuppetDeathParams` holds an exploding-damage threshold (0), three further numbers 25, 10, 5 (unnamed), an explosion impulse 2,000 with up ratio 0.5.
- **Hand grenade** reference and effects: a pointer to the hand-grenade projectile parameters, a text-effect for notifications and one for changing the left weapon, an exhale effect and sound.

## Pickups (`Items/`, 51 parameter files)

Every item file holds one `CBaseItemParams` with **one number**; the world entity (`S…ItemEntity`) names the model and the weapon or power-up index and carries no amount:

| Group | Files | The number |
|---|---|---|
| Ammo (`ammo/`) | 11: ammo pack, bullets, cannonballs, grenades, hand grenades, klodovik, napalm, plasma, rockets, shells, sniper bullets | 30 in all |
| Health (`health/`) | XS, S, M, L, XL | 10, 10, 25, 60, 120 |
| Armor (`armor/`) | XS, S, M, L, XL, XXL | 10, 10, 25, 10, 60, 120 |
| Power-ups (`powerup/`) | invisibility, invulnerability, serious damage, serious jump, serious score, speed, strength | 40, 60, 40, 40, 40, 40, 60 |
| Weapons (`weapons/`) | 16: auto shotgun, beam gun, cannon, colt, combo weapons, double shotgun, flamer, grenade launcher, klodovik gun, minigun, plasma rifle, rocket launcher, serious bomb, sniper, uzi, zap gun | 10 in all except the cannon: 30 |
| Treasure, life, repair | treasure S/M/L, extra life, repair | 60 in all five |

The number looks like a **respawn delay in seconds** for multiplayer (it is the same for all ammo, all weapons but one, and follows the item size for health), but **no member names it**, so this is a guess. It is **not the amount given**: the editor help states health 5/10/25/50/100 and armor 5/10/25/50/100/200, which does not fit the numbers (health XL 120 versus 100; armor L 10). The amounts a pickup gives, the ammunition caps and the weapon-pickup ammo bonuses are therefore **not in the data** and need runtime observation (T01-038/039); a test plan is: pick up each item at low and at full values and record the changes.

## Netricsa

Only the HUD side is data: `CNetricsaHudElementParams` (96 members: fonts, the colours of the panel, the ring rotation speed 180, texture stretch factors, the layout of level description and objectives), the hint and objective texts from the localization tables, and the `ggp_fBeaconAndObj…` timings above. The objectives and beacon logic is driven by macros and entities (`Mission objective`, `Navigational beacon`; `help-digest.md`).

## Death and respawn

From the data: **chapter info** entities are the respawn points (`help-digest.md`); a chapter sets the player's start inventory (weapons, ammo, health, armor, preferred weapon, `Loose weapons`). `SPuppetDeathParams`, `3rdPersonDeathCamera` and the extra-life rule (10,000 points) exist; the rules for losing a life and the behavior of "serious" (single-hit) modes are not in the data.

## Open questions

1. Pickup amounts, ammunition caps and the ammo each weapon pickup gives (not in the data; measure).
2. The meaning of `CBaseItemParams.1` (respawn delay is a guess) and of the numbered `SPuppetDeathParams` and `SPuppetHealthParams.2` members.
3. How armor absorbs damage; how damage multipliers combine (type multiplier × difficulty × power-up); the order of application.
4. What "health recharging" in Tourist does (rate and delay): not in the data.
5. Multiplayer score and frag rules (T01-037).

## Test vectors

Values to check against a decoder: difficulty Normal = (1, 1, 1, 1, no, 1, 1); Serious = (1, 1.25, 1.75, 2, no, 0.25, 0.8); power-up durations single player 60 for all seven; multiplayer 40, 20, 40, 30, 40, 30, 30 in the order of the table above; extra life every 10,000 points; FOV 80; player health 100.
