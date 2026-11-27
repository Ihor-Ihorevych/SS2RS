# Playing a level (playable MVP)

`ss2-app` can load any level of your own Serious Sam 2 install and let you walk or fly through it. There are no
enemies, weapons, sounds or lighting yet; the movement numbers are provisional (tuned by feel, not measured from the
original game). See the [design](superpowers/specs/2026-11-27-playable-mvp-design.md) for what is in and out.

## Starting

```text
ss2-app                              # menu: New Game opens the level list
ss2-app --level 1_1_jungle           # straight into a level
ss2-app --data "D:/Games/Serious Sam 2" --level dm_dockingbay
ss2-app --level dm_dockingbay --screenshot out.png --size 1920x1080   # offscreen picture, no window
```

The game data folder is `--data`, else `SS2_DATA_DIR`, else the first install the search finds (Steam, the registry,
common folders). Without an install the level list still offers the built-in **Test room**. A level is named by its
folder (`1_1_jungle`, `dm_dockingbay`) or its world file name, in any case.

## Controls

| Action | Keyboard and mouse | Gamepad |
|---|---|---|
| Move | W A S D | left stick |
| Look | mouse | right stick |
| Jump (walking), rise (flying) | Space | South (A / Cross) |
| Sink (flying) | Ctrl or C | |
| Fly faster | Shift | |
| Switch walking / flying | F | North (Y / Triangle) |
| Pause menu (Resume, Options, Quit to Menu) | Esc | Start |
| Screenshot | F12 | |

The mouse is captured while you play; after switching to another window, click to take it back.

## What you will notice

- Lighting is a flat light from the camera; there are no lightmaps, sky, fog or water surfaces yet.
- Collision uses the coarsest drawn shape of each model, so it can be slightly off where the real collision mesh
  differs (T01-055).
- Single-player levels start at a chapter marker, because the real start rule is not specified yet (T01-210).
- 109 world-embedded models and some textures in old development formats are left out (listed in the log).
