# Localization QA pass

- **Roadmap task:** T12-013 (`roadmap/T12-qa-release.md`), issue #347
- **Status:** procedure. The engine has no translation tables yet (T10-011), so the first part of the pass
  (font coverage and overflow on English text) can run now and the per-language part waits for T10-011.
- **Evidence rule:** only text, CSV or numbers that you produced yourself. No game files, no screenshots
  of game art, no game text (see [TRIAGE.md](TRIAGE.md) section 8 and [PARITY.md](PARITY.md) section 2).

## 1. What the specs say (and do not say)

Sources: `docs/re/localization.md` (strings, languages, overlays), `docs/re/fnt.md` (fonts and code tables),
`docs/re/cutscenes.md` and `docs/re/misc-types.md` (`.srt`, `.crt`, `.nfo`, `.dsc`), `docs/re/rsc-ctl.md`
(menu layouts), `docs/re/mods.md` (overlays by mod), [ACCESSIBILITY.md](ACCESSIBILITY.md) section 4.2.

- **Short UI strings** are `CTransString` values: a key and a default (English) text. Lookup of the key in the
  active language's table, with fallback to the default text, is inferred; the table format is **not** in the
  install (`docs/re/localization.md`).
- **Long texts** (level and model descriptions `.dsc`, credits `.crt`, subtitles `.srt`) are separate files.
  A language folder `Content/SeriousSam2/Locales/<lang>/` repeats the full original path and replaces the base
  file by path (case-insensitive).
- **Installed languages:** `docs/re/localization.md` states that the retail install carries English and a
  partial German overlay (`Locales/deu/`), and that no translation tables ship. It does not give the number of
  languages the retail game supports. Do not state a count.

## 2. Font coverage (procedure)

Source: `docs/re/fnt.md`. A `.fnt` file has a code table `CFont.7`: entry `c` is the glyph index for character
code `c`; 0 means no glyph, and glyph 0 is drawn instead.

1. Run `python -I tools/re/fnt_verify.py [DATA_DIR]` (needs `SS2_DATA_DIR` or the directory argument). It prints
   the covered code ranges per font. Record the result per language as pass or fail, not the full range list.
2. For each font the language's text uses, take the set of code points of the language's strings
   (decoded from UTF-8) and check `c < len(table)` and `table[c] != 0`.
3. Every uncovered code point is an issue: it will draw as the fallback glyph (a visible box in the console font).
4. The docs state the following facts that bound the result:
   - the 127-entry tables cover ASCII only (5 fonts); the 8,231- and 8,483-entry tables cover further ranges;
   - the highest mapped code is `0x2026` (8,231-entry tables) or `0x2122` (8,483-entry tables);
   - Latin-1 and Latin Extended-A are mapped (per `fnt.md`); the docs do not say whether Cyrillic, Greek or CJK are mapped.
     Check them with step 2. A code point above `0x2122` is not mapped in any font named in `fnt.md`.
5. No kerning data exists in the font file (`fnt.md`). Do not treat spacing differences from kerning as a
   font defect without a reference.

## 3. Checklist template (copy per language)

Result column values: `match` (checked, correct), `issue` (checked, problem: give the issue number), `not checked`
(say why in the Evidence column). Evidence is a checklist row, a CSV row or a test string length; never game text.

```markdown
## Localization QA: <language code> (SS2RS commit <sha>, install patch level: <Patch_* names>)

| ID | Check | Result | Issue | Evidence |
|---|---|---|---|---|
| LQ-F1 | Every code point of the language is mapped (`table[c] != 0`) in each font the screen uses | | | |
| LQ-F2 | No fallback glyph (box) appears in any menu, HUD or subtitle string | | | |
| LQ-F3 | Font choice per gadget is known (how fonts are selected is an open question, see section 7) | | | |
| LQ-O1 | Menu button bar labels, captions and option labels fit their gadget (section 4) | | | |
| LQ-O2 | HUD, pickup and objective messages fit their area | | | |
| LQ-O3 | Netricsa messages fit their area | | | |
| LQ-O4 | Loading-screen tips and pause, death and summary screens fit | | | |
| LQ-O5 | Co-op overlay text (scoreboard, chat, player names) fits | | | |
| LQ-O6 | Text effect (`.tfx`) conversation boxes fit their background | | | |
| LQ-O7 | Expansion budget (PROPOSED rule, section 4) is met or the string is flagged | | | |
| LQ-P1 | Format arguments and placeholders are present and in the same count as the default | | | |
| LQ-N1 | Number, date and plural forms are correct | | | |
| LQ-S1 | Each `.srt` cue text is readable at the default subtitle size | | | |
| LQ-S2 | Each cue has start before end, cues are in time order, and the voice starts with its cue | | | |
| LQ-S3 | A localized `.srt` replaces the base file by path under `Locales/<lang>/` | | | |
| LQ-C1 | Each `.crt` credit position stays inside the 640 x 480 virtual screen | | | |
| LQ-C2 | Credit headings and names are complete and the BOM is skipped | | | |
| LQ-D1 | Each `.dsc` description has a language file, or the base file is used (fallback) | | | |
| LQ-B1 | Text baked into textures (`.tex`, font atlases) is listed, since it cannot be translated by key | | | |
| LQ-E1 | Text files are UTF-8 (BOM allowed); Latin-1 files are read as Latin-1 | | | |
| LQ-L1 | Line breaking, right-to-left and CJK layout (open question, section 7) | | | |
```

## 4. Overflow and clipping

What to check (from the specs and T10): text in menus (`.rsc` gadgets: button bars, captions, option
labels, labels, scroll lists), HUD and pickup messages (T10-003), Netricsa messages (T10-008), co-op overlays
(T10-014), loading tips (T10-009), pause and summary screens (T10-020), credits (T10-016), and `.tfx` text
styles (T10-013). Clipping means text outside its gadget or its background box, or cut at the screen edge.

- **Resolutions and aspect ratios:** T10-015 asks for UI golden images at three resolutions and two aspect
  ratios; the docs do not name the three resolutions. The existing menu goldens are 1280 x 720
  (`docs/TESTING.md` section 3.4). The virtual canvas is 1920 x 1080 (T10-030, done), with aspect ratios from
  16:9 to 21:9 (T10-017). Record the resolution and aspect ratio of every screenshot you check.
- **Expansion budget (PROPOSED, not in the specs):** a translated string may be up to 30 percent wider than its
  English default before it needs a layout review. Measure the width with the font's glyph advances (`CFontCharacter`
  member 7, `docs/re/fnt.md`) or with the running build, and state which one you used.
- **Test strings:** use your own strings of the target length (for example, a repeated neutral word of the
  same character count). Do not paste the English or translated game text.
- **Subtitle length:** no line-length limit is given in the specs (open question, section 7). Record the number of
  lines and characters per cue; do not invent a limit.

## 5. Which screens to run

Menu screens are described in `docs/re/rsc-ctl.md` by category, not by name:

- 99 menu screen layouts (`CMenuScreenLayout`), with an underlay class (popup, main menu or the default
  underlay) and an optional overlay class;
- the gadget kinds in those layouts (button bars, labels, images, option select boxes, links, sliders,
  scroll lists, progress bars, image buttons, arrangers);
- the HUD and global databases (`HudParams.rsc`, `SimpleHudBarParams.rsc`, `MenuLayout.rsc`);
- movies with subtitles and credits (72 subtitle files, 5 credit files, `docs/re/cutscenes.md`);
- level and model descriptions (`.dsc`), and the mod list (`.nfo`).

Run the pass on one screen of each category first. Then run all screens of the language.

## 6. How to record and triage

1. One line per checklist ID in the PR or issue that completes it:
   `ID | match / issue / not checked | method (one line) | commit`, as in
   [ACCESSIBILITY.md](ACCESSIBILITY.md) section 5.
2. Measurements may be pasted into the PR as one line per row. A CSV (UTF-8, LF, header
   `row,screen,language,check,result,measure,method,tolerance`) is PROPOSED; it holds numbers and labels only,
   never game text, and its location is to be agreed by a maintainer.
3. Defects: use `.github/ISSUE_TEMPLATE/bug.md`. A rendering difference from the original uses
   `visual-parity.md`. Put the checklist ID in the report.
4. Severity and priority follow [TRIAGE.md](TRIAGE.md) sections 2 and 3, set from the effect on the player.
   No table in the docs maps a text defect to a severity.
5. Never attach `.gro`, `.tex`, `.fnt`, `.srt`, `.crt`, `.dsc` or any other game file, and never paste game
   text (TRIAGE section 8). Describe the defect in words and give the string length and the font name.

## 7. Gaps (the specs do not cover these)

- **Translation tables:** their format, location and the list of supported languages (`docs/re/localization.md`).
  Until T10-011 defines a documented format, there is nothing to load for a language.
- **Language selection** (profile setting or command line) and the fallback chain.
- **Placeholders and format arguments** in strings: no format is documented.
- **Plural, number and date formats:** not documented.
- **Line breaking, right-to-left and CJK layout:** not documented in the specs. Treat them as open questions;
  do not declare them supported or unsupported.
- **Text expansion and layout rules** for longer translations (`docs/re/localization.md` lists them as not in the data).
- **How a font is selected for a GUI element**, and whether a font has fallback fonts (`docs/re/fnt.md`).
- **Subtitle line length, size and placement** limits (ACCESSIBILITY SU-3 and SU-7 are open).
- **Localized voice files:** the `.srt` cue `sound` path can point to a language file, but no localized voice
  exists in the install (`docs/re/cutscenes.md`).
- **Text in textures** cannot be translated by key; which screens bake text into art is not inventoried.
- **Key hashes:** the eight-digit keys in `.ep` files are not explained (`docs/re/localization.md`).
