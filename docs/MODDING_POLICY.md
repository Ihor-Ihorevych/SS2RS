# Modding policy: mod content, redistribution and licensing

This page says what a mod for SS2RS may contain, what it may never contain, how mods that change
original content are distributed, and how mod authors license their work. It applies to everything
in this repository and to anything the project links to or lists.

This is a project policy written in plain language. It is **not legal advice**. For borderline cases
the maintainers decide, and they may ask for content to be changed or removed.

## 1. Background

SS2RS is an independent engine. It does not contain Croteam's game content, and players supply their
own legally obtained copy of *Serious Sam 2* (see [NOTICE](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/NOTICE) and [LICENSE](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/LICENSE)).
Mods are how other people add content and behaviour on top of that engine.

The mod interface is described in [docs/re/mods.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/re/mods.md). Its facts that matter here:

- a mod has a descriptor (`.mod` names the game modules; `.nfo` holds the name, author and thumbnail);
- a mod's content is in overlay archives (`.gro` files, which are ordinary zip archives), and an overlay
  entry replaces a base entry when it has the same path;
- some mods ship a replacement native game module (a `.dll`). A Rust engine cannot load that DLL.

The load order and conflict rules are not settled yet (see T13-001 in [docs/roadmap/T13-mods.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/docs/roadmap/T13-mods.md)).

## 2. What a mod may contain and redistribute

A mod may contain and redistribute:

- **Original assets** made by the mod author: textures, models, sounds, music, text and levels that the
  author created themselves, without copying from the game.
- **Original code** and **scripts** written by the author.
- **Configuration and data** written by the author, including parameter values and tables.
- **Data that only references the player's own files by path**: for example a file that says "use the
  texture at this path in the player's install". The mod does not contain the referenced bytes.
- **Mod descriptors**: `.mod` and `.nfo` files written by the author, and a README.
- **Documentation** written by the author.

If an original asset is made to look like a game asset, it is still original, as long as it was not
traced, copied, converted or extracted from a game file.

## 3. What a mod must never contain or redistribute

A mod must never contain or redistribute any of the following:

- **Croteam or Devolver game assets or derivatives**, including:
  - the game's `.gro` archives, and any copy or part of them;
  - `.dll` and `.exe` files from the game or its installer;
  - textures, models, sounds, music, movies, level files and fonts from the game;
  - anything copied or converted from those files, including re-encoded, resized or re-packed copies;
- **Game text**, such as dialogue, subtitles, menus, help files or strings taken from binaries;
- **Decompiled or disassembled output** of any game binary, and project files made by decompilers;
- **Extracted or re-encoded copies** of any game file, even if the result looks different.

A mod that changes an original asset must not carry a copy of the original asset in any form.

## 4. Distributing mods that change original content

A mod that changes original content (for example a retexture or a new level version) must be
distributed in one of these two ways:

1. **Separate original files** that the engine loads over the player's own data. The files contain
   only the author's own content, and the player's copy of the game is not included.
2. **Patches or instructions** that need the player's own copy of the game. A patch must not contain
   any bytes of the original file, including bytes of a diff that reproduce the original. Instructions
   may name files and steps, but they must not include game content.

Game `.dll` and `.exe` files are never part of a mod (section 3). Whether a mod may ship its own native
module is not settled: the engine cannot load native game modules, and T13-002 decides the strategy.

This policy says nothing about the third-party mods that a game install may already contain, such as
the mods shipped in the `Content` folder. Those are not part of this repository.

## 5. Licensing guidance for mod authors

- **You choose the license for your own work.** The project does not license mods for you. Nothing in
  this repository's LICENSE gives a mod's license.
- **State the license** in your mod descriptor (the `.nfo` file, if your tools allow extra text) and in
  your README. Include the license name and, if you can, a link to its text.
- **Common options**, in neutral terms: MIT, Apache-2.0 and CC0-1.0 are permissive (they allow reuse
  with few conditions); CC-BY-4.0 requires attribution; CC-BY-SA-4.0 is share-alike (derivatives must
  use the same license). Read the license text before you choose one.
- **The engine is Apache-2.0** (see [LICENSE](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/LICENSE)). That license covers the engine's own code
  and documents. It does not license game content, and it does not decide the license of a mod.
- **A mod that uses only the engine's public mod interface** is a separate work from the engine, and
  its license is the author's choice. The repository's docs do not yet define a public mod API
  (T13-005; what exists today is in the [modder docs](modding/api.md)), and whether a given mod is a derivative work is a legal question. Mod authors
  who need certainty should ask their own advisor.
- **Original content still has to stay original**: the license of a mod does not make game content
  redistributable.

## 6. Relation to the clean-room rule and the repository guard

- [CLEANROOM.md](https://github.com/Ihor-Ihorevych/SS2RS/blob/main/CLEANROOM.md) governs how the project's own code and specs are written. Its rule
  against copying or translating decompiler output applies to contributors to this repository. The
  same concern applies to mods in section 3: a mod must not redistribute decompiler output.
- The repository itself must never contain game content. `cargo xtask guard` (the pre-commit hook and
  CI) rejects `.gro`, `.dll`, `.exe` and `.chm` files anywhere, rejects game-data extensions (`.wld`,
  `.tex`, `.mdl`, `.bmf`, `.ogv`, `.xmv`) outside small fixtures under `testdata/`, rejects paths inside
  a `Serious Sam 2` install folder, and rejects oversized files.
- **Example mods in this repository must be original or synthetic.** They must pass `cargo xtask guard`
  and must not reference game files as shipped content. A file that is needed only to test a loader is
  made by hand for the test, not copied from a game.

## 7. What to do when you find a mod or pull request with game content

- **Do not download, open, copy or quote the content**, and do not discuss how to get the game data.
- **Report it** by opening an issue with the Bug template (see [TRIAGE.md](TRIAGE.md) §1). Name the
  file or the pull request. Do not attach the content, and do not paste parts of it.
- **Maintainers** remove the content, comment where it was removed and decide next steps (TRIAGE §8).
  The reporter should not re-share the content in replies.
- For a **pull request**, the maintainers do not merge it. They ask the author to remove the content and
  to rewrite the branch if needed. They may ask for the earlier versions of the branch to be removed
  as well. Do not argue the case in public threads.

## 8. Checklist before you publish a mod

- [ ] Every file in the mod is either written by me or generated by my own tools.
- [ ] No file came from the game's `.gro`, `.dll`, `.exe` or other game files, and nothing is a copy or
      a re-encoded copy of one.
- [ ] Any text is my own. No dialogue, subtitles, menus or help text were copied from the game.
- [ ] A changed original asset is shipped as a separate file the engine loads over the player's data, or
      as a patch or instructions that need the player's own copy, as in section 4.
- [ ] The `.mod` and `.nfo` descriptors are present and correct, and the mod has a README.
- [ ] `ss2-doctor <install folder> --mod <Name>` reports no error for the mod; the warnings are read and
      understood ([Mod validation](MOD_VALIDATION.md)). It also warns about native code and `.gro`
      files inside a mod (MOD046), which section 3 forbids.
- [ ] The README states the license and links to its text.
- [ ] I have checked that nothing in the mod folder is a game file (compare the file list with the names
      in the install, and check the hashes of anything you are unsure about).
- [ ] I have not included a decompiler project, decompiled code or disassembly.

## 9. For maintainers

- **Takedown and removal requests.** The docs do not define a separate procedure for rights-holder
  requests. Handle them under TRIAGE §8: remove the material, record the decision in the issue, and do
  not discuss the material in public. Requests that involve legal questions are for the maintainers;
  this page does not decide them.
- **Listing mods.** Not yet. The repository has no mod list or site in its docs. When one is added, it
  should follow the rules in this page and link back here.
