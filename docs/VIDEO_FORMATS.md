# Video formats: `.ogv` plays, `.xmv` is ignored

Status: implemented as a lookup rule in `ss2-video` (roadmap task T06-021). The decoder (T06-010) and the cutscene
player (T06-011) do not exist yet, so nothing in the running game calls the rule today.

## The rule

| Extension | Kind | What the engine does |
|---|---|---|
| `.ogv` | Ogg Theora/Vorbis, the PC movie format | played (once the player exists) if the file is in the VFS |
| `.xmv` | Xbox video | **ignored by design**: the cutscene is skipped, with one `warn` log line, no error |
| anything else | unknown | skipped with a `warn` log line (`Unrecognized`) |

The extension is compared ASCII case-insensitively on the last path segment only: `A.XMV` and `a.b.xmv` are `.xmv`;
`.xmv` (a name that is only an extension), `movie.xmv.bak` and a folder called `x.xmv/` are not.

## Why `.xmv` is ignored

`.xmv` is the Xbox console video format. The retail PC install has exactly **one** such file, `demo_02.xmv` (27 MB,
the Xbox demo), next to `demo_02.ogv`, which is the PC movie. No level or episode `.nfo` names it: all 94 movie
references name `.ogv` files (`docs/re/cutscenes.md`). Supporting it would need a proprietary Xbox decoder for no
playable content, so the engine does not try.

## What the engine does when it meets one

`ss2_video::resolve_movie(vfs, reference)` returns a typed, non-fatal `MovieOutcome`:

- a `.xmv` reference gives `MovieOutcome::Skipped { path, kind: VideoKind::Xmv, reason }` and logs
  `WARN ... .xmv is the Xbox-only video format ...; cutscene skipped`. The file need not exist and is never opened,
  so a corrupt or huge one cannot cause harm;
- `level_movies(vfs, nfo)` does this for `INTRO_MOVIE`, `OUTRO_MOVIE` and `EPISODE_MOVIE` of a level or episode
  `.nfo`: the cutscene whose movie is a `.xmv` is skipped and the other movies of the same `.nfo` still resolve;
- `scan_videos(vfs, dir)` lists a folder (the install, a mod folder) and reports each `.xmv` as `VideoKind::Xmv`
  without opening it;
- `ss2-doctor --mod` already accepts only `.ogv` for movie keys (`ss2-vfs` `modcheck`), so a mod that names a `.xmv`
  is reported there as a wrong extension; this task did not change that.

## Real-install count

On the player's install (all patches mounted): **98 `.ogv` and 1 `.xmv`**, nothing else with a video extension, as
the spec says. The data-gated test `real_install_video_counts_match_the_spec` checks this and that the real `.xmv`
resolves to `Skipped` and a real `.ogv` to `Play`.

## Changing this later

To support `.xmv`: add a decoder crate, make `VideoKind::Xmv` playable (`is_playable`, `resolve_movie`), drop
`ignore_reason`, change the `modcheck` extension lists, and replace this page. The classifier and the outcome type
stay, so callers do not change.
