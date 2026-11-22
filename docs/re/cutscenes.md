# Cutscenes and movies (T01-035)

- **Roadmap task:** T01-035
- **Author:** @lxstyo/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the movie files, their formats, naming and how levels refer to them, and for the subtitle and credits files (`misc-types.md`); **medium** for the in-engine camera cutscene classes (member names and the editor help); **open** for the exact trigger and skip rules, which are game logic (macros and the game module).

Prerequisites: `misc-types.md` (`.nfo`, `.srt`, `.crt`), `help-digest.md` (camera entity, chapters), `classes.md`.

## Summary

There are two kinds of cutscenes. **Pre-rendered movies** (`.ogv`, Ogg Theora with Vorbis audio) bracket every level and episode; each has a same-named **subtitle file** (`.srt`) that also schedules the voice lines. **In-engine cutscenes** are driven by **camera entities** that follow an animation, started and awaited from the level's macro program. The credits are a text overlay (`.crt`) over a movie.

## Evidence

`tools/re/movie_verify.py` reads the Theora and Vorbis identification headers of every movie and cross-checks the `.nfo` references. The camera entity classes come from the type tables of the worlds that contain them (28 worlds). Nothing from binaries.

## Movie files (`Content/SeriousSam2/Movies/Ntsc/`)

- **98 `.ogv` files** (1,365 MB) in one folder, `Ntsc`; they live in the movie archives (`Movies_PC_01.gro`, `Movies_PC_02.gro`) and in patches for `boot` and `demo_02`.
- **Codec:** Ogg container; **Theora** (version 3.2.0 in 97 files, 3.2.1 in one) and **Vorbis** stereo: **44,100 Hz in 97 files, 48,000 Hz in one**.
- **Frame size 640 × 368** in 97 files (a widescreen crop of 640 × 480) and **1,920 × 1,080** in one: `boot.ogv`, which exists twice: `All_PC_03` has a 640 × 368 version at 30 fps and the last patch replaces it with a 1080p version at 59.94 fps and 48 kHz audio. **Frame rate** 29.97 fps in 87 files, **25 fps in 10** (`3_1_intro`, `3_1_outro`, `4_1_intro`, `5_1_intro`, `5_3_outro`, `5_4_outro`, `6_2_intro`, `6_3_outro`, `7_5_intro`, `7_6_intro`: the movies made at the PAL rate), 59.94 fps in the patched boot movie.
- **Names** (the roles come from the names and the `.nfo` references):
  - `<chapter>_<level>_intro.ogv` and `…_outro.ogv`: the movie before and after each level (for example `1_1_intro`, `7_7_outro`; one extra `6_5_boss_outro`); a few levels have no intro or outro;
  - `episode_1.ogv` to `episode_7.ogv`: the movie that opens each episode; `end.ogv` (66 MB, the longest): the ending with credits;
  - `medallion_1.ogv` to `medallion_5.ogv`: five very short (1 to 2 MB) clips; `boot.ogv` (the patched replacement is the 1080p file above); `demo_01.ogv`, `demo_02.ogv` (identical sizes: 35,921,165 bytes) and `demo_02.xmv` (the Xbox video format, 27 MB; the only non-Ogg movie).
- **94 movie references** are written in the level and episode `.nfo` files (`INTRO_MOVIE`, `OUTRO_MOVIE`, `EPISODE_MOVIE`), as `Content/SeriousSam2/Movies/Ntsc/<NAME>.ogv`; **all 94 name an existing file** (the path is case-insensitive).

## Subtitles and credits

- **72 `.srt` files** (in `Movies/`, one level up), **each named like one movie** (72 of 72 have a same-named `.ogv`): the cue grammar and the `sound="…"` voice-over attribute are in `misc-types.md`. The subtitle file **drives the voice**: when a cue starts, its `.wav` plays, so the movie's audio track may be only music and effects. (Check one movie against its `.srt` before relying on this.)
- **5 `.crt` files** (credits: `demo_01`, `end`, and localized copies under `Locales/deu/…`): the same cue grammar with `pos="x,y"` positions in the 640 × 480 virtual screen; they scroll or page over the movie.

## In-engine cutscenes (camera entities)

Per the editor help, a **camera entity** is used for cutscenes and "needs to be activated through a macro function". In the data: `SCameraEntity` (28 world files; 24 members) with its properties record `SCameraProperties` (8 members):

| Member (entity / properties) | Type | Role |
|---|---|---|
| `SCameraProperties.1` | IDENT | the animation name played along the camera path |
| `.10` | animation configuration pointer | the camera animation (envelope data with position, rotation, FOV, aspect channels: `help-digest.md` "Animation set") |
| `.3`, `.5`, `.6`, `.11` | BOOL | "relative path", "use master parent", "use view target", "use parent orientation" (the help names these options; the mapping to members is by order only) |
| `.4`, `.7` | FLOAT | default FOV and default aspect (used when the animation has no FOV or aspect channel) |
| entity `en_csState` | `CameraState` | the state machine of the camera (idle, playing, …; values not named in the data) |
| entity `en_bLastCameraInterrupted` | BOOL | whether the last camera was **interrupted** (a cutscene can be cut short) |
| entity `.8` | `CMacroLatentID` | the latent id of the macro that started it, so the macro **waits** until the camera finishes (a latent action, `help-digest.md`) |
| entity `.21` | text queue handle | subtitles shown during the shot |
| entity `.25`, `en_hpaehAnimEventHandler` | handles | the target entity and the animation event handler |

The world also owns `CMovieSyncList` (59 of 59 worlds) with a list of `CMovieChannel`: the places where **movies are played inside a level** (a movie player entity in the editor, `help-digest.md`), so a level can show a video on a surface or full screen. Cutscene cameras use an `SCameraEnvelopeProvider` object (28 worlds) that gives the animation host the camera it controls.

## Chapters and movies

When a chapter with a "next level" ends, the level is finished: the **outro movie** named in the level's `.nfo` plays and the next level loads (`help-digest.md`). The **episode movie** is shown when an episode starts (menus: extras/movies/select episode, per the custom-level tutorial). `medallion_N` is presumably the clip shown on collecting an episode's medallion; unconfirmed.

## What is not in the data

1. Which player input skips a movie or a camera cutscene, and whether the engine requires a minimum play time.
2. How the engine decodes and presents the video: scaling and letterboxing 640 × 368 content, audio sync with the `.srt` voices, frame pacing at 29.97 fps. A reimplementation should decode Theora and Vorbis with permissive libraries and present at the original aspect.
3. The macro that starts each cutscene (T01-012g/T01-030) and the `CameraState` values.
4. The `.xmv` format: the Xbox demo only; out of scope for the PC game.

## Reader invariants

1. Movies are standard Ogg Theora/Vorbis; no custom container; read frame size and rate from the Theora identification header.
2. A movie reference in an `.nfo` is a path under the install, case-insensitive; resolve it through the archive reader.
3. A `.srt` belongs to the movie of the same base name in `Ntsc/`.
4. `.crt` credits use the same parser with positions.

## Open questions

The "What is not in the data" list; whether the 25 fps movies differ in any way from their 29.97 fps neighbours (they are not a separate locale: each name exists once); the roles of the five medallion clips.

## Test vectors

`tools/re/movie_verify.py` prints: 98 `.ogv`, 1 `.xmv`; 94 `.nfo` references and none missing; 72 `.srt` with same-named `.ogv`; Theora 3.2.0 ×97 and 3.2.1 ×1; frame size 640 × 368 ×97 and 1920 × 1080 ×1; rates 29.97 ×87, 25 ×10, 59.94 ×1; Vorbis stereo 44,100 Hz ×97 and 48,000 Hz ×1.
