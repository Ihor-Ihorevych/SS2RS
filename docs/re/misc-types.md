# The smaller file types: `.nfo .dsc .srt .crt .preload .cfg .bin .cb .sfc .pss .ctr .mch .mac .sca .mas`

- **Roadmap task:** T01-019
- **Author:** @lxstyo/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for what each type is, its file structure and who uses it (every file of every type was read); **medium** for the member meanings that carry readable names (`.cb`, `.sfc`, `.pss`, schemes); **low** for the unnamed numeric members and for `dsstdfx.bin`.

Prerequisites: `ctsemeta-container.md`, `ctsemeta-types.md`, `ctsemeta-objects.md`. The full member reference of the structured types is generated: **`misc-schema.md`** (116 classes, 379 members, by `tools/re/ep_schema.py`).

## Summary

| Type | Files (distinct) | Kind | What it is | Used by |
|---|---|---|---|---|
| `.nfo` | 79 | text | description of a mod, an episode, a level, a player model or a recorded demo | menus, mod list, loading screens |
| `.dsc` | 198 | text | description paragraph that goes with an `.nfo` | menus (player model and level pages) |
| `.srt` | 72 | text | timed subtitles with sound cues for the story movies | movie player |
| `.crt` | 5 | text | the same cue format used for **credits** (positioned text) | credits movies |
| `.preload` | 46 | text | list of the resources a level needs, to load before it starts | level loader (one per `.wld`) |
| `.cfg` | 2 | text script | engine start-up scripts in the console language | engine boot |
| `.bin` | 1 | binary | `Content/Engine/dsstdfx.bin`, an unidentified 19,824-byte table | unknown |
| `.cb` | 82 | CTSEMETA | **character behavior**: enemy AI decision tables | worlds (81 of them) |
| `.sfc` | 11 | CTSEMETA | **spawn effect configuration**: how a spawner presents a spawn (effect, sound, projectile, animation) | worlds |
| `.pss` | 7 | CTSEMETA | **puppet spine data**: which bones bend toward the aim and how much | `.ep` (the legged puppet parameters) |
| `.ctr` | 3 | CTSEMETA | **editor custom trees**: the entity, shader and weapon palettes of the editor | editor only |
| `.mch` | 8 | CTSEMETA | **mechanism templates**: body, joint and hull templates for physics | models (`.mdl`) |
| `.mac` | 8 | CTSEMETA | **macro programs** stored as files (most macros live inside worlds) | worlds |
| `.sca` | 4 | CTSEMETA | **animation schemes**: term → weighted animation names (`mat.md`) | models, `.ep` |
| `.mas` | 20 | CTSEMETA | **moving animation schemes**: which locomotion animation to play for a pose, direction and speed | models, `.ep` |

Files already specified elsewhere (`.scs`, `.scp`, `.mat`, `.tfx`, `.lfx`, `.vfx`, `.pfx`, `.fnt`, `.rsc`, `.ctl`, `.ep`, `.mtr`) are not repeated. The structured types are all format-8 CTSEMETA files with a `CResourceFile` root and **no extra records after `EDOB`**; all 119 decode to the `METAEND` footer.

## Evidence

`tools/re/misc_verify.py` (read-only, standard library) re-runs every count below. The help file (`help-digest.md`) names the `.nfo` keys and the chapter/level structure; everything else is from the data. No binaries were used.

## Text types

General rules found in all text files: UTF-8 (one `.nfo` and four `.dsc` are not valid UTF-8: Latin-1 text), a UTF-8 **byte-order mark is present in some** (2 `.nfo`, 139 of 198 `.dsc`, 2 `.srt`, all 5 `.crt`) and must be skipped; line ends are CRLF or LF. Localized copies live under `Content/SeriousSam2/Locales/<lang>/` with the **full original path repeated below it** (for example `Locales/deu/Content/SeriousSam2/Models/Vehicles/…`); only `deu` appears in these types (66 `.dsc` in the base tree, 132 under `deu`; 3 `.crt` in the base tree and 2 under `deu`).

### `.nfo`

An optional **kind word on a line of its own** (`LEVEL` in 58 files, `EPISODE` in 7; the mod, player-model and demo files have none), then a list of `KEY = "value"` assignments. The terminating `;` is optional (present in the mod files, absent in the others); a value may continue on the next line after `KEY=`; keys are upper case; the order is free. Key sets observed over the 79 files:

| Kind | Where | Keys |
|---|---|---|
| Mod / game description | `Content/*.nfo` (3: the game, `InSamnity2`, `Sam2Renovation`) | `NAME`, `AUTHOR`, `THUMBNAIL` |
| Player model | `Content/SeriousSam2/Models/Player/` (8) | `NAME`, `MODEL` (`.mdl`), `THUMBNAIL` (`.tex`), `DESCRIPTION` (with the text in `.dsc` of the same name) |
| Episode | a folder under `Levels` (kind word `EPISODE`) | `NAME`, `THUMBNAIL` |
| Level | next to its `.wld` (kind word `LEVEL`) | `NAME`, `MAP_TYPE` (`SINGLE; COOP; DEATHMATCH;` semicolon separated), optional `PLAYERS`, `LOADING_MODEL`, `MODEL_ANIMATION`, `MODEL_ROTATE`, `MODEL_DISTANCE`, `MODEL_NAME`, `LOADING_SCREEN`, `THUMBNAIL`, `ETA`, `EPISODE_MOVIE`, `INTRO_MOVIE`, `OUTRO_MOVIE` |
| Recorded demo | `Content/SeriousSam2/Demos/` (2) | `NAME`, `TIMESTAMP` |

This agrees with the help file (`help-digest.md`). 59 files have `MAP_TYPE`, 58 `LOADING_MODEL`, 47 `OUTRO_MOVIE`. A reader must ignore unknown keys.

### `.dsc`

Plain text: a description of one subject, matched to the `.nfo` of the same base name in the same folder (all 66 base-tree `.dsc` have a sibling `.nfo`). 147 of 198 are a single paragraph; the others contain blank lines.

### `.srt` (subtitles) and `.crt` (credits)

SubRip-style cues, blank-line separated:

```
<index>
HH:MM:SS,mmm --> HH:MM:SS,mmm
<B sound="Content/…/Some.wav">text, possibly spanning several lines
```

641 cues in 72 files (all in `Content/SeriousSam2/Movies/`, named like the movie: `1_1_intro.srt`). Every cue ends after it starts and cues are in time order in every file. The `<B …>` tag is the only markup (about 520 occurrences) and its only attribute is **`sound`**, the voice-over `.wav` (an install path) that plays with the cue, so **the subtitle file also schedules the movie's voice lines**. A cue's text can itself contain a blank line (one cue in `end.srt`): parse by the time line, not by blank lines.

`.crt` credits use the same grammar with **`pos="x,y"`** instead (104 cues in 5 files; positions in the 640 × 480 virtual screen, such as `100,100`), the first line of the text being a heading ("CEO/PRODUCER") and the rest names. Cue numbers can repeat in `.crt` (two blocks both numbered 6 when two columns appear at once).

### `.preload`

One file per world, named `<world>.wld.preload` next to it (46 files, 56,471 lines). A line is **`<number>:<path>`**; the path is an install path (all 56,214 non-separator lines name a file that exists), the number is **not** the archive size except for `.wav` (all 15,963 sound lines carry the exact size) and a few others; for `.mdl`, `.tex`, `.pfx`, `.ep` and the rest it differs from every archive version (so it is something else, possibly a loaded size; unknown). **Lines with number −1 and an empty path** (257, 1 to 9 per file, mostly 5 to 7) separate groups (five of them in the first Jungle world; the first group is the HUD, fonts and text effects, the next ones weapon models, weapon sounds and effects, then level content).

The list is in load order, has **no duplicate path** within a file (checked in all 46) and mixes every resource type: in `1_1_jungle` 399 `.tex`, 379 `.wav`, 236 `.mdl`, 122 `.pfx`, 45 `.ep`, 28 `.ogg`, down to single `.sfc`, `.lfx`, `.fnt`. Over all files: `.wav` 15,963, `.tex` 15,466, `.mdl` 9,005, `.pfx` 5,674, `.ep` 2,230, `.ans` 1,094, `.ctl` 855, `.bmf` 840, `.mat` 733, `.mtr` 673.

Implementation note: it is a **hint for loading and streaming** (preload everything on the list behind a loading screen), not a dependency map: the world's own references remain the authority.

### `.cfg`

Two engine start-up scripts in `Content/SeriousSam2/Config/` (`autoexec.cfg`, and the Xbox variant), written in the console language: `[Section]` headers (`Core`, `Engine`, `ProcRender`, `Sam2Game`), `//` comments, `if(!var) { … }` blocks and calls such as `svcStartService(ident("Service.RConServer_Local"))`. Specified with the console variables in T01-033.

### `.bin`

`Content/Engine/dsstdfx.bin`: 19,824 bytes, 10,584 of them zero, all 256 byte values occur. Its name suggests a **DirectSound standard-effects table**; no structure was determined (the bytes do not start with a magic number). Not referenced by name from any data file. Open.

## Structured types

All classes and members are in `misc-schema.md`; below, what each file is for and the rules a reader needs.

### `.cb` character behavior (82 files)

Root `CCharacterBehavior` (`CResource` base). Despite the file count (82) there are only 76 distinct names (`cb_idName` such as `Fighter01_Mental`, `empty`); worlds refer to 81 of the files. Structure:

- **Header:** `cb_idName`, `cb_idBasedOnCharacter` (always `empty`) and `cb_pcbBasedOnCharacter` (null in all 82: **no inheritance is in use**), `cb_baAligment` (`EBrainAligment`: values 0 to 3), the **ranges** `cb_fActiveRange` (250, 400, 2,500, 25,000), `cb_fSenseRange` and `cb_fThreatRange` (0 to 5,000), `cb_fInitialReactionPeriod` (−1), optional graphs `cb_pgrThinkPeriodOveride`, `cb_pgrAltitudeMin/Max` (flying characters), `CFoeAsigingDescriptor.fad_fAutoChangeFoeChance` (0 to 10).
- **`cb_acsCharacterStates`**: 0 to 8 `CCharacterState`s (134 in all; 36 distinct names such as `Default`, `Agressive`, `RunAndGun`). A state has three `CDipContentAreas` (**land, air, water**: the behavior depends on the medium the character is in) and a `CAIMomentHandler` (events → actions).
- **Areas:** each `CDipContentAreas` holds 0 to 5 **`CCustomBehaviorArea`** (381 in all): a named range band (`Far`, `Middle`, `Close`, …, 31 names), its outer distance `cba_fDistance` (2 … 10⁸), the **movement to use** (`cba_pmcMoveCharacter`: `CMCTowardsFoe`, `CMCCircleAroundFoe`, `CMCStayInArea`, `CMCPatternMoving`, or null: 324 set, 57 null), a graph `cba_pgrActionFrequencyOverAreaRatio` (how often to act as a function of the position inside the band), a random multiplier `cba_fActionPeriodRandomizeMultiplier` (0 to 0.5), a `CAIMomentHandler` and a velocity adjuster (`SAreaVelocityAdjuster`: acceleration, deceleration, speed 0.5 to 2, rotation multipliers). This is the "movers chosen by distance to the foe" of the help file (`help-digest.md`).
- **Actions** (`CBehaviorAction` and derived, 853 in all): each has a `ba_fWeight` (0 to 100,000), an optional **validity term** (`ba_pcValidityTerm`: 104 set; `SBAVTHealth` (51: observation kind and a health value up to 10,000), `CBAVTFoeAngle`, `CBAVTPlayerDistance`, `CBAVTHasTool`, `CBAVTCurrentStateDuration`, combiners via `CBAVTTermCombiner`, each with a `vt_bNotTrue`) and three optional weight multiplier graphs (over health, distance and area ratio; all null in the shipped files). The weights are presumably used to pick an action at random in proportion to the weight. Action classes seen: `SBAShoot` (112; `as_idShoot` names a shot such as `Shoot`, `Laser`, `TankRocket` (38 names), min/max shots 1 to 200, "shoot while moving" graphs), `SBAAnimation` (112; animation name and whether to move during it), `SBAMelee` (67; attack name: `Melee`, `HandKick`, `Kick`, …), `CBAPlaySound` (47; sound term in a scheme: `Sight`, `War`, …; volume, pitch, pitch random), `CBANoAction`, `CBADefaultAction` and, in other files, `SBAChangeState`, `SBALeapOntoFoe`, `SBARunOverFoe`, `SBAShieldStatus`, `SBAEmotion`, `SBALocalMove`, `SBAThrowMacroEvent`, `CBAChangeMovingAbilities`, `CBAForcePlayerFoe`, `CBAPlayerAssigning`, `CBAThrowPickedTool`, `CBAWait`, and lists (`CBAList`, `SBAList`).

How the game combines the weights, ranges and graphs at run time (the think loop) is **not** in the files: that is T01-025.

### `.sfc` spawn effect configuration (11 files)

Root `CSpawnEffectConfiguration`: how a spawner shows a spawn: the chance to stop the source (`sfxc_fSpawnSourceStopChance` 0, 0.25, 1), whether the spawnee matches the throw direction, the source's prepare and spawn animations (`Spawn`, `Vomit`, … or `empty`), a **particle effect** (`sfxc_pefEffect`, a `.pfx`: 10 of 11), stretch and power (always 1), a **sound** (5), a **delay** (0 to 2.2 s), an optional **projectile** (`sfxc_idProjectile`, `sfxc_pgppProjectileParams`: 4) with stretch and life after touch, and the spawnee's start animation (`SpawnGetup`, `Fly_Forward`, …). Referenced from worlds by 11 files (the spawner entities).

### `.pss` puppet spine data (7 files)

Root `CPuppetSpineData`: `psd_mSwizzling` (a 3 × 3 matrix mapping the model's axes to the aim axes) and 2 to 7 `CPuppetSpineBone` (`psb_idName`: `Chest`, `Neck02`, `Head`, … 16 names; `psb_fBendRatio` 0.1 to 0.7): **the share of the aim rotation each spine bone takes**, so a character twists the upper body to face a target. Referenced by 78 entries of `.ep` files (7 distinct targets); the layout is also in `ep.md`.

### `.ctr` editor custom trees (3 files)

`Content/SeriousEditor2/CustomTrees/{Entities,Shaders,Weapons}.ctr`: `CCustomTreeFile` root with one `CCustomTree` (name, own path, a root `CCustomTreeDirectory`; directories hold lists of sub-directories and of `CCustomTreeFile` entries (142 in all, each two strings)). They are the palettes of the editor and are **not used by the game**; a runtime need not read them.

### `.mch` mechanism templates (8 files)

Root `CMechanismTemplateHolder` with 1 to 8 `CMechanismTemplate` (29 in all; 10 names such as `Default`, `Ragdoll`, `RagdollSimple`): a tree of `CMechanismPartTemplate` with body templates (`CDynamicBodyTemplate` and `CHibridBodyTemplate` (sic: that is how the game spells it); no kinematic template class occurs), hull templates (`CPrimitiveHullTemplate`) and joint templates (`CBallJointTemplate`, `CConeJointTemplate`, `CHingeJointTemplate`, `CLinearMotionJointTemplate`, `CAircraftJointTemplate`). Matches the help's body types (dynamic, hybrid, kinematic) and joint kinds. Referenced by 8 models. The detailed layout and units belong to T01-057 (physics templates) and T01-023.

### `.mac` macro programs (8 files)

Root `CMacroProgram` (functions, `On` handlers, statements, expressions, variants): the **same classes that make up the scripts embedded in worlds**, stored as separate files. Eight files, each referenced by a world. Their layout is covered with the macro language in T01-012g and the semantics in T01-030; they are smaller samples than any world and are good first test cases.

### `.sca` animation schemes (4) and `.mas` moving animation schemes (20)

The animation counterparts of the sound and particle schemes of `mat.md` (same inheritance rule: a base scheme pointer, followed when a term is not found).

- **`CAnimationScheme`**: `.1` a list of `CAnimsInScheme` (term such as `Wound`, `Alive`, `Wound_Fly`: 11 names; then a list of **`CWeightedAnimInScheme`**: weight INDEX (always 1) and animation name), `.2` the base scheme (1 of 4 set).
- **`CMovingAnimationScheme`**: `.1` a list of `CAllMovingAnimation` (up to 24: the animation for the full body), `.2` `CBodyMovingAnimation` (up to 56: the upper-body override animations, 30 names such as `Body_Minigun_IdleFight`), `.3` `CPoseTransitionAnimation` (up to 12: transitions such as `Knockdown_Front`, `Crouch`, `JumpEnd`), `.4` base scheme (15 of 20 set). Each entry is a **selector plus a name**: pose, direction, speed, readiness, shield status and emotion (`MovingAnimationPose`, `MovingAnimationDirection`, `MovingAnimationSpeed`, `MovingAnimationReadiness`, `CharacterShieldStatus`, `CharacterEmotion`), where the value **`0xFFFFFFFF` means "any"** (it shows as 4.29497e+09 in the schema), plus optional extra names and a flag `ama_bDisableBodyOverride`. The player looks the animation up by matching the current state of the character against the selectors; how ties are broken is open (T01-029).

## Invariants for a reader

1. A text reader strips an optional UTF-8 BOM, accepts CRLF and LF and falls back to Latin-1 if UTF-8 is invalid.
2. `.nfo`: ignore unknown keys; `;` is optional; values may be on the next line; quoted strings contain no escapes in any file.
3. `.srt` / `.crt`: split cues on the time line; accept repeated indices; the `<B …>` tag has the attributes `sound` (`.srt`) or `pos` (`.crt`) and no closing tag.
4. `.preload`: split each line at the first `:`; skip an empty path (number −1); ignore the number except as a size hint.
5. CTSEMETA types: parse with the generic reader; all roots are `CResource`s with `CResourceFile` as first object; unknown derived classes (behavior actions, validity terms, moves) must be **dispatched on the object's class**, and an unknown class is an error.
6. Treat `0xFFFFFFFF` in the scheme selectors as a wildcard, and `CControlPoint.4` in any graph as garbage (`rsc-ctl.md`).

## Open questions

1. `dsstdfx.bin`: structure and use (T01-033 or T01-034).
2. The number in `.preload` lines (not the archive size except for sounds) and what the −1 separator groups mean (T01-022 loading).
3. The run-time use of `.cb` data: weight combination, action period, the think period, the meaning of the three `EBrainAligment` values (T01-025).
4. The tie-breaking and priority of the moving-animation selectors (T01-029).
5. `.mch` units and the rest of the physics templates (T01-057).
6. Whether `.dsc` is chosen by language folder before the base file (the overlay rule of `Locales`); a data-only question for T01-041.

## Test vectors

`Content/SeriousSam2/Models/Player/DancingDenzel.nfo` (first lines): `NAME="Dancing Denzell"`, `MODEL="Content/SeriousSam2/Models/Player/Denzel/Denzel.mdl"`, `THUMBNAIL="Content/SeriousSam2/Models/Player/Denzel.tex"`: three keys, no semicolons.

A `.srt` cue and the values it must decode to:

```
1
00:00:13,914 --> 00:00:17,584
<B sound="Content/SeriousSam2/Sounds/Netricsa/NG_Hi_Sam_No_No.wav">Hi Sam! No no, don't look around.
```

start 13,914 ms, end 17,584 ms, voice-over `Content/SeriousSam2/Sounds/Netricsa/NG_Hi_Sam_No_No.wav`, text "Hi Sam! No no, don't look around.".

A `.preload` line: `158491:Content/SeriousSam2/Fonts/SeriousVerdana.fnt` (first line of the Jungle world's list): number 158,491, path as given; the archive entry is 153,707 bytes, so the number is not its size.
