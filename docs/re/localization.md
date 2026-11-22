# Localization: strings, languages, overlays (T01-041)

- **Roadmap task:** T01-041
- **Author:** @lxstyo/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for what the install contains (every `CTransString` in the menu, parameter and tool files counted; every file under `Locales/` listed); **medium** for how translations are applied (inferred from the data layout and from names in the engine modules); **open** for the translation tables themselves: **none ships in this install**, only English default texts and a handful of German files.

Prerequisites: `misc-types.md` (`.dsc`, `.srt`, `.crt`, `.nfo`), `rsc-ctl.md` (menu layouts), `mods.md` (overlays).

## Summary

User-visible text is stored in two ways:

1. **Short UI strings** are `CTransString` values inside the data (menu screens, parameters): a **key** plus the **default (English) text**. A translation system maps the key to the text of the chosen language, and falls back to the default text.
2. **Long texts** (level and model descriptions, credits, subtitles) are **separate text files** (`.dsc`, `.crt`, `.srt`) that a language folder can replace file by file.

The retail install carries **only English** (the defaults and the base-tree text files) and **a partial German overlay** (`Locales/deu/`, 135 files: descriptions and credits). There are no translation-table files, so the other retail languages (the game was published in more than English and German) are not present in this install.

## Evidence

`tools/re/l10n_verify.py` decodes the objects of all `.rsc`, `.ep`, `.ctl`, `.mdl` and `.mac` files and counts `CTransString` values; the archives were listed for `Locales/`. `Core.dll` and `Sam2Game.dll` contain the names of the string class and of translation tables (names only).

## `CTransString`

A by-value class with **two members**: `.1` the **key** (a string) and `.2` the **default text** (a string, possibly empty). Over the install:

| Where | Values | With a non-empty default text | Key style |
|---|---|---|---|
| `.rsc` (menus, HUD and global databases) | 2,125 | 1,388 | 2,083 **dotted names** `<Screen>.<gadget>.<property>` (for example a button bar label `…bbrButtonBar.Left00` with text `back`, a caption `…lblCaption.Name` with `CHAPTERS`, option labels `…opslInvertLook.Option00` with `YES`), 42 eight-digit **hex hashes** |
| `.ep` (character parameters) | 126 | 6 | all **8 hex digits** (for example `93E51DA2`, the boss name of the player puppet's misc parameters, empty) |

**Distinct keys: 2,153; 1,373 with a non-empty default text.** The dotted keys are the menu's own names; the hex keys (character names such as boss names) are **hashes of the text** or of its id and carry the default text only when the entry was filled in (6 of 126 characters). An empty default means the text is produced elsewhere (for example from the game module or the translation table).

Behavior in the original engine (names only): `Core.dll` has `TranslationTable` and `TranslationTables`; `Sam2Game.dll` has a `Translate` function and `CTransString` constructors from a plain string and a `CString`, plus a conversion to a character pointer. This points to a **lookup of the key in the active language's table, falling back to the default text**; the table files and their format are not in the install.

## Language folders (`Content/SeriousSam2/Locales/<lang>/`)

135 entries, all under **`deu`**: **132 `.dsc`**: 48 level descriptions and 84 model descriptions (enemies and so on). Of the 132, **54 translate a description that exists in the base tree** (66 there), **78 have no English counterpart file** (the enemy descriptions and two demo levels: the English texts for those are apparently stored elsewhere, for example in the translation tables) and **12 base descriptions (multiplayer levels) have no German version**; the `.nfo` of each subject stays in the base tree, **2 `.crt`** (credits) and one `copyright_xbox.txt`. The **path below the language folder repeats the full original path** (`Locales/deu/Content/SeriousSam2/Models/Vehicles/…`), which makes the language folder an **overlay**: a loader that is set to `deu` resolves a request for `Content/…/X.dsc` first in `Locales/deu/Content/…/X.dsc`, then in the base tree. This is the same replace-by-path rule as mod overlays (`mods.md`).

What is **not** localized in the install: the subtitles (`.srt`: English only, all 72 in the base tree), the voice-overs (English `.wav`), the movies, and all menu strings except through the missing translation tables.

## Subtitles and voices

`.srt` files carry English text and the English voice file per cue (`misc-types.md`, `cutscenes.md`); a localized game would need a localized `.srt` per language (the path rule above) and localized voice files (the cue's `sound` path can point to a language-specific file).

## What is not in the data

1. The **format and location of the translation tables**, and the list of languages the game supports.
2. The **language selection** (profile setting or command line) and the fallback chain.
3. **Fonts** per language: `Content/SeriousSam2/Fonts` has the default fonts (`SeriousZurich`, `SeriousVerdana`, HD variants: `fnt.md`); how Asian or Cyrillic text is rendered is not visible.
4. Text expansion and layout rules for longer translations.

For a reimplementation: keep the **key → default text** mapping, load translation tables from a **new, documented format** (the original one is unavailable), and apply the locale-folder overlay rule to the long texts.

## Reader invariants

1. `CTransString`: two strings; an empty default text is valid.
2. A language folder mirrors the full path of the files it replaces; resolve through the overlay rule (`vfs`: replace by path, case-insensitive).
3. Text files may start with a UTF-8 byte-order mark (`misc-types.md`).

## Open questions

The "What is not in the data" list; where the English texts of the 78 descriptions without a base file are stored, and how the key hashes are computed.

## Test vectors

`tools/re/l10n_verify.py` prints: `.rsc` 2,125 values (1,388 non-empty), `.ep` 126 (6 non-empty), 2,153 distinct keys (1,373 non-empty); `.rsc` keys 2,083 dotted and 42 hex; Locales: `deu` 135 entries (132 `.dsc`, 2 `.crt`, 1 `.txt`).
