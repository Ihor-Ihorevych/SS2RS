# Behaviour family: data leftovers (T01-159)

Answers `behavior-classes.md` open question 8. Evidence: `tools/re/behavior_leftovers_verify.py`
(data only, run against the base game plus the two mods installed on this machine, InSamnity2 and
Sam2Renovation — neither is clean-room content, both are read-only test data). Binary reading is
**medium** confidence (nothing was run); the data counts below are **high** (reproduced by the
script) and every one closes exactly against the invariants already published in
`behavior-classes.md`.

## 1. Who owns the 564 world-stored `CAIMomentHandler`s

Not `SPuppetEntity`/`SPuppetProperties` directly: the `Handle<CAIMomentHandler>` member
(`en_hmhMomentHandler` / `ep_hmhMomentHandler`) is declared on each of the five concrete "character"
record pairs a puppet's entity and properties specialise into —
`S{Aircraft,Hovercraft,Legged,RollingBall,Turret}Character{Entity,Properties}` — not on the generic
puppet base. In the base game's 59 worlds, non-null / total:

| Record | Entity | Properties |
|---|---|---|
| `SAircraftCharacter*` | 17 / 254 | 17 / 253 |
| `SHovercraftCharacter*` | 0 / 41 | 0 / 41 |
| `SLeggedCharacter*` | 250 / 1,825 | 280 / 1,852 |
| `SRollingBallCharacter*` | 0 / 44 | 0 / 44 |
| `STurretCharacter*` | 0 / 164 | 0 / 135 |

**267 (entity side) + 297 (properties side) = 564** — the exact count `behavior-classes.md` section
2 already reports for the data's "564 counted, all in worlds" `CAIMomentHandler` objects. So that
564 is not 564 distinct objects; it is the entity's own cached handle and its paired properties
record's handle **both** counted (the original survey script's object walk has no cross-path
dedup), one shared handler object reached twice per legged or aircraft puppet that carries an
override: legged puppets alone account for 530 of the 564 (250 + 280), aircraft for the other 34 (17
+ 17). Hovercraft, rolling-ball and turret puppets never carry a moment-handler override in the base
game. (Mods: InSamnity2 has 3 legged-properties overrides and no others; Sam2Renovation's split is
197/0/477/0/0 entity-side — same shape as base, no new owning class.)

Reading the chain: a placed puppet's moment handler is a **per-instance override** of its `.cb`
brain file's own moments (`behavior-classes.md` section 2, `CAIMomentHandler` = "the lists of a
behaviour, state or band") — a level designer can give one specific placed `Legged`/`Aircraft`
character a custom weighted-action list (most commonly a custom `SBAShoot`/`SBAAnimation` moment)
without writing a new `.cb` file. This is exactly what the 614 `SBAShoot` and 16 `SBAAnimation`
"inside the moment handlers of 23 worlds" (section 6) are: per-placement shoot/animation overrides on
individual enemies.

## 2. Mod data and the two obsolete classes

`behavior-classes.md` Invariant 5: `CTurretBehaviorEntity` and `CRideFoeManagerEntity` print the
engine's "Class (%1) is obsolete and should be removed from level ASAP." error (a shared stub at
`CAIBrainEntity`'s `OnCreate` address, `0x72af0` in the Ghidra decompile — the same address carries
the exported name of roughly thirty other never-overridden `OnCreate`s across unrelated families,
e.g. `CKleerBrainEntity`, `CBuggyBrainEntity`; this is linker identical-code-folding, not evidence
that those other classes are themselves obsolete) and do nothing else.

Scanned every `.wld`/`.cb`/`.ep` of the base game **and** of both mods installed here (InSamnity2:
5/81/260 files; Sam2Renovation: 31/79/323 files) for the type merely being declared in a file's type
table, and for any stored instance: **zero in every case, base and both mods.** No installed mod (nor
the base game) ever places or even declares either obsolete class; they are pure runtime leftovers
with no data-side path to them.

## 3. How a tactic without a marker moves

Already answered precisely by T01-155 (`behavior-marker-movers.md` section 5.2, `ObtainFirstPathMarker`):
the tactic's own `en_henTacticMarker`, else the puppet's `en_ulAutoPatrolMarkerEntityID`, else
**`GetClosestMarker`** — the nearest `CPathMarkerEntity` (or subclass) to the puppet's own position by
plain 3-D distance, no limit — else an empty marker, at which point the mover fails its step at once
(`MoveCharacter` returns 1). `GetGoalMarker` is a dead end for this: that doc already notes it is "not
used by the movers". `en_henTacticMarker` is therefore an optional direct link, not a requirement:
**65% of `CMCMarkersMovingOrdered` tactics (491 of 755) carry no marker reference at all** and still
patrol, via `GetClosestMarker`, in both base and mods (base: ordered 264 marker / 491 no marker,
closest-to-foe 21/15, random 18/6; InSamnity2 24/56, 5/5, 0/2; Sam2Renovation 201/331, 17/10, 10/5 —
the same shape everywhere, confirming the mechanism holds for mod-authored levels too, not only the
base game's own).

This T01-159 scan adds one binary-reading detail on top of T01-155's data reading:
`CBrainEntity::StartPatrolingMarkers(Handle<CPathMarkerEntity>, int)` is a plain no-op on a null
handle (returns without touching the puppet's cached current-marker field or changing state) — so
whatever resolves the marker (`ObtainFirstPathMarker`) must run, and succeed, *before*
`StartPatrolingMarkers` is called; nothing inside it manufactures a fallback marker itself.

## Reproduces

`python -I tools/re/behavior_leftovers_verify.py "<install>"` prints, per source (base + each
installed mod): the `Handle<CAIMomentHandler>` member paths with non-null/total counts; the
`STacticEntity` marker-vs-mover-class cross-tab; and the obsolete-class type/instance scan. About 10
minutes across base + two mods.
