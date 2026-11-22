# `.lfx` flare configurations, `.tfx` text effects, `.vfx` vibration effects

- **Roadmap task:** T01-017
- **Author:** @lxstyo/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the file structure, the class layouts, the references between files and the value ranges (every file checked: 37 `.lfx`, 10 `.tfx`, 5 `.vfx`); **medium** for the meaning of the ten flare-part members (the editor help names them in the same order and the value ranges agree); **low** for the text-effect members and for how any graph is evaluated; **open** for how the game maps effects to events.

Prerequisites: `ctsemeta-container.md`, `ctsemeta-types.md`, `ctsemeta-objects.md`. Related: `wld-lighting.md` (flare entities), `rsc-ctl.md` and `rsc-ctl-schema.md` (graphs, HUD parameters that name text effects), `fnt.md`, `help-digest.md`.

## Summary

Three small resource types, all format-8 CTSEMETA files with the usual `CResourceFile` root and one resource object:

| Extension | Root class | What it is | Files |
|---|---|---|---|
| `.lfx` | `CFlareConfiguration` | a **lens flare**: a list of textured parts laid out along the line from the light to the screen centre | 37 (all in `Content/SeriousSam2/Effects/Flares/`) |
| `.tfx` | `CTextEffectConfiguration` | an **animated on-screen text** style (HUD messages, conversation subtitles): font, placement, timing and a list of effects driven by graphs | 10 distinct paths (28 archive entries because patches replace them) |
| `.vfx` | `CXboxVibroFX` | a **controller vibration** pattern: one graph per motor | 5 (all in `Content/SeriousSam2/Vibro/Xbox/`) |

All of them are tiny (1.3 to 4.6 KB), contain no extra records after `EDOB` and use the **graph** classes already described in `rsc-ctl.md`.

## Evidence

- `tools/re/fx_verify.py` (read-only) walks every file with the decoder of `ctsemeta-objects.md` and prints the counts and ranges quoted below; with `--refs` it reads the references of every `.ep`, `.wld`, `.rsc`, `.mdl`, `.pfx` and `.ctl` to find who uses these files.
- The meaning of the flare members comes from the order in which the editor help (`help-digest.md`) documents the properties of a flare, compared with the value ranges. No other source exists for `.tfx` and `.vfx`, so those members are described by structure and by the names of the files that use them (HUD parameter names in `rsc-ctl-schema.md`).
- Nothing here comes from binaries.

## `.lfx` flare configuration

One `CFlareConfiguration` (`CResource` base, then `.1`): a **static array of `CFlarePart`**. 191 parts in 37 files: 19 files have one part, 9 have ten, 7 have nine, one has eight and one eleven. Files with several parts are the big "sun" flares of the levels (`sun_flare_<world>…lfx`, `moon_flare_…`); single-part files are lamps, fires and crystals.

The same `CFlarePart` list also occurs **embedded** in worlds (`wld-lighting.md`: 1,074 embedded configurations versus 1,427 references to these external files); a reader handles both with one routine.

### `CFlarePart` (40 bytes, members in file order)

| Member | Type | Observed | Meaning | Confidence |
|---|---|---|---|---|
| `.1` | pointer to texture | an `RFIL` entry (`.tex`) in all 191; the file's `RFIL` count equals its number of distinct textures | the flare image; one texture can serve several parts (the sun flares reuse one "bubble" texture for 3 to 4 parts) | high |
| `.10` | `COLOR` (u32) | alpha byte is `0xFF` in all 191 | multiplicative tint; channel order as for other `COLOR`s (`wld-lighting.md`, open) | medium |
| `.2` | float | −2.0 … 1.061 | **offset** along the light→screen-centre axis: 1 = at the light, 0 = at the screen centre, negative = beyond the centre on the far side, above 1 = past the light. Part 0 of a sun flare is about 1.0, the following "ghosts" use values like 0.063, 0.665, −0.442, −1.092 | medium |
| `.3` | float | 0.012 … 0.961 | **size** relative to the screen (0 … 1) | medium |
| `.4` | float | 0.927 … 2.5 | **aspect** of the quad (1 = square; the window flare uses 2.5) | medium |
| `.5` | float | 0.126 … 6.271 (1.0 in 64 parts) | **light amplification** (brightness multiplier) | medium |
| `.6` | float | 0.0 … 1.0 (0.5 in 54 parts) | **light desaturation** | medium |
| `.7` | float | 0.05 … 8.96 (1.0 in most) | **falloff factor** (multiplies the falloff) | medium |
| `.8` | float, degrees | 90.0 in 171 parts; 50.9 in 16; 360.0 in 2; 0.0 and 120.0 once each | **rotation factor**: how much the texture rotates with its position on screen, maximal at the screen edge | medium |
| `.9` | `FlareFlags`, 4 bytes | 2 in 186 parts, 1 in 3, 3 in 2 | bit 0 fade by size, bit 1 fade by intensity (distance), a further "off-center fade" exists in the help; the exact bit for that is not seen (values use bits 0 and 1 only) | low |

The help describes the flare as an additive, always-camera-facing quad. Units, the angle convention of `.8` and the falloff formula are not stated anywhere we can read; see open questions.

## `.tfx` text effect configuration

`CTextEffectConfiguration` (`CResource` base with the extra `CResourceCustomData` member `.4`, null everywhere, then):

| Member | Type | Observed | Notes |
|---|---|---|---|
| `.1` | dynamic container of `CTextEffect*` | 0 effects in 3 files, 1 in 3, 2 in 2, 3 in 2 | whole-text effects (see below) |
| `.2` | dynamic container of `CLetterEffect*` | 0 in 7 files, 1 in 1, 2 in 2 | per-letter effects |
| `.3`, `.4`, `.5` | float | `(0,2,0)` ×3, `(1.2,1.2,0.5)` ×2, `(0.25,100,0.25)`, `(1.5,0,0)`, `(0.2,2,0.2)`, `(0.5,3,0.5)`, `(0.5,2,1)` | look like **time in, time held, time out** in seconds (0.5 / 3 / 0.5 for the player notifications; 100 = "until removed" for the weapon-change message). Low confidence |
| `.6` | `Vector2f` | e.g. (320, 358), (32, 128) | position in a **640 × 480 virtual screen** (the centre of that screen is (320, 240)); the HUD uses this size everywhere |
| `.14` | `Vector2f` | (640, 480) in 7 files, (400, 60) in the 3 conversation files | size of the text box |
| `.7` | `CFontInstance` | reference to a `.fnt` (`SeriousZurich_HD.fnt` ×7, `SeriousVerdana_HD.fnt` ×3), colour, flags (0 ×8, 1 ×2), size members, shadow colour | the same class as in HUD parameters (`rsc-ctl-schema.md`) |
| `.8`, `.9` | `TextAlignmentH`, `TextAlignmentV` | H: 1 ×9 (probably centred), 0 ×1 (probably left; the title-style `playernotifications` at x = 32); V: 1 ×10 | alignment codes are 0, 1 (a third value is plausible; not seen) |
| `.10` | `GfuBlendType` | `0x1F5` (501) in all 10 | the blend enumeration of `mtr.md` |
| `.12`, `.13`, `.15`, `.20` | BOOL, float, BOOL, COLOR | 0, 0.5, 1, and a colour (`0x78000000` in the three conversation files, else 0) | `.20` looks like a **background box colour** (alpha 0x78) for subtitles; low confidence |

The three **conversation** configurations (`playerconversation`, `puppetconversation`, `netricsaconversation`) are the plainest: no effects, a 400 × 60 box at (320, 386) and the 0x78 background. The other seven are HUD popups with effects.

### Effect classes

Both bases hold an enable flag and **four graph pointers** (`.1` to `.4`). In every effect of every file the four graphs are one of a few fixed patterns:

| Class | Own members | Graph pattern (`.1`, `.2`, `.3`, `.4`) seen |
|---|---|---|
| `CBlendTextEffect` (5) | `.1`, `.2` colours, observed `0x00FFFFFF` and `0xFFFFFFFF` (transparent and opaque white) | step-up, value, step-down, value (3) · value, value, step-down, value (2) |
| `CStretchTextEffect` (5) | four floats, e.g. `(0, 0.8, 0, 0.8)`, `(0.5, 0.5, 0.5, 0.5)`, `(0, 1, 0, 1)` | linear, value, step-down, value (2) · step-up, value, step-down, value (2) · linear, value, value, value (1) |
| `CMoveTextEffect` (3) | `.1` a `Vector2f` offset, e.g. (80, 80), (320, 280) | step-down, value, step-up, value (2) · linear, value, value, value (1) |
| `CSizeLetterTextEffect` (4, letter effect) | `.5` delay between letters (0.02 or 0), four floats `(0.9, 1, 0.9, 1)` or `(0, 1, 0, 1)` | value, sine, value, value (2) · step-up, value, step-down, value (2) |
| `CScreenOffsetAppearLetterTextEffect` (1, letter effect) | `.1` an INDEX, `.2` `TextSide` | step-up, value, step-down, value (1) |

Reading: the pair (`.1`, `.2`) shapes the **appearing** phase and the pair (`.3`, `.4`) the **disappearing** phase, one curve and one constant each, and the effect's own members are the **start and end values** that the curve blends between (transparent white to opaque white for the blend effect; a start/end scale for the stretch effect; an offset for the move effect). This reading fits every pattern but is **not proven**.

`CControlPoint.4` (an INDEX next to each point) is frequently `0xCCCCCCCC` or `0xCDCDCDCD` (uninitialised editor memory) and must be ignored, as in `rsc-ctl.md`.

## `.vfx` vibration effect

`CXboxVibroFX` (`CResource` base, then):

| Member | Type | Observed |
|---|---|---|
| `.4` | graph pointer | present in `lowfreqlinear`, `lowfreqsine`, `bothfreqmaxlinear`; null in the two `highfreq*` files |
| `.5` | graph pointer | present in `highfreqlinear`, `highfreqsine`, `bothfreqmaxlinear`; null in the two `lowfreq*` files |
| `.6` | float | **5.0 in all five** (almost certainly a duration in seconds) |

The file names make `.4` the **low-frequency (large) motor** and `.5` the **high-frequency (small) motor**. The graphs are `CLinearGraph` (4) or `CSineGraph` (2); the linear ones run from (0, 0) to (1, 1) (a ramp), `bothfreqmaxlinear` holds 1.0 on both motors, the sine ones have points (0, 0.5) and (1, 1). A graph's x axis is 0 to 1, presumably the normalized duration; y is the motor strength 0 to 1.

The files carry no references, and **nothing in the data refers to a `.vfx` by path** (a text search of every archive finds the class name `CXboxVibroFX` only in the five files themselves). Worlds contain vibration sync lists and channels (`CVibroSyncList`, `CVibroChannel`, `CVibroFeeler`; T01-012h), so the effects are probably selected by name in game code. They are Xbox-controller specific; a PC port plays them through its own gamepad rumble (T03-003).

## Graph members seen in these files

Common `CGraph` base (`rsc-ctl.md`), over the 72 graphs of the text effects and the 6 of the vibration files: `.7` is 1 in all; `.8` is 1.0, or 0.33 in the two sine graphs of `enemypurged`/`itempicked`; `.9` is **0 for value, step-up and step-down graphs, 3 for every linear graph and 4 for every sine graph** (looks like a type code of the graph kind); `.10` is 0, 1 or 2 for the simple graphs and an unexplained large number (136 … 2836) for the linear and sine graphs; `.11` 0 for simple graphs, small numbers for linear graphs; `.12` equals the constant of a value graph (as `rsc-ctl.md` says: 1 where the point is (0, 1), 0 where it is (0, 0)) and is 0 in every other class. Point counts: step-up 10 graphs × 2 points, step-down 14 × 2, linear 6 or 8 points in the text effects and 2 in the vibration files, sine 2.

## Invariants for a reader

1. Resolve the class of every pointer by its object; the graph pointers point to **derived** classes (`CLinearGraph`, `CSineGraph`, `CStepUpGraph`, `CStepDownGraph`, `CValueGraph`) and must dispatch on the object's class; an unknown graph class is an error (checked: only these five occur).
2. A `CFlareConfiguration` has at least one part; texture pointers always name `RFIL` entries (never an object of the file).
3. In every `.tfx` the `.1`/`.2` lists contain only effect classes listed above; pointers are never null.
4. All three types have an empty `EDOB` extra-record list.
5. A `.vfx` has no `RFIL` entries; a `.tfx` has exactly one (the font); a `.lfx` has one per distinct texture.
6. Ignore `CControlPoint.4`.

## Who uses them

`fx_verify.py --refs` over the install: `.lfx` is referenced only from worlds (62 `RFIL` entries to all 37 files); `.tfx` from `.rsc` HUD databases (8 `RFIL` entries; the member names are in `rsc-ctl-schema.md`: conversation, enemy purged, extra life, fragments counter, item picked, …) and from `.ep` (4 entries, among them the player puppet); `.vfx` by nobody (see above).

## Open questions

1. Units and formulas for the flare (`.2` to `.8` as used in rendering): needed by T04 renderer; best settled with a reference capture (T01-038) or by the shader/material specs (T01-020).
2. The `FlareFlags` bits beyond 0 and 1 and the byte order of `.10` (T01-061).
3. Evaluation of the graph classes (shared with `rsc-ctl.md`): interpolation, the sine parameters, the role of `CGraph.8` to `.11`, how the x axis maps to time in a text effect.
4. Whether the (`.1`, `.2`) / (`.3`, `.4`) reading of the text-effect graphs is right, and the roles of the three timing floats `.3` to `.5` of the configuration.
5. How game code chooses a vibration effect (by name, from which events) and what `CXboxVibroFX.6` means exactly (T01-033, T01-037).
6. Whether the `TextSide` value of `CScreenOffsetAppearLetterTextEffect` and the alignment codes have more values than were observed.

## Test vectors

**One flare part** (from `kleerofski_diamond.lfx`, the static array of parts: marker `STAR`, count 1, then 40 bytes):

```
53 54 41 52  01 00 00 00                                  "STAR", count 1
02 00 00 00                                               .1  texture pointer = object id 2 (the RFIL entry)
ff ff ff ff                                               .10 colour 0xFFFFFFFF
00 00 80 3f  b8 1e 05 3e  00 00 80 3f  00 00 80 3f        .2 = 1.0, .3 = 0.13, .4 = 1.0, .5 = 1.0
00 00 00 3f  cd cc 4c 3d  00 00 b4 42                     .6 = 0.5, .7 = 0.05, .8 = 90.0
01 00 00 00                                               .9  FlareFlags = 1
```

**One graph's control points** (from `highfreqlinear.vfx`, a `CLinearGraph`'s point list: marker `SSAR`, count 2, then 2 × 12 bytes):

```
53 53 41 52  02 00 00 00
00 00 00 00 00 00 00 00  cd cd cd cd      point (0, 0), .4 = 0xCDCDCDCD (ignore)
00 00 80 3f 00 00 80 3f  00 00 00 00      point (1, 1), .4 = 0
```

`highfreqlinear.vfx` as a whole decodes to: `CResourceFile` (object 0), `CXboxVibroFX` (object 1) with `.4` null, `.5` → object 2 and `.6` = 5.0, and `CLinearGraph` (object 2) with `.9` = 3 and the two points above; 1,315 bytes in all.
