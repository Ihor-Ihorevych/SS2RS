# Playing a level (playable MVP)

`ss2-app` can load any level of your own Serious Sam 2 install and let you walk or fly through it. There are no
enemies, weapons, sounds or lighting yet; the movement numbers are provisional (tuned by feel, not measured from the
original game). See the [design](superpowers/specs/2026-11-27-playable-mvp-design.md) for what is in and out.

## Starting

```text
ss2-app                              # menu: New Game opens the level list
ss2-app --level 1_1_jungle           # straight into a level
ss2-app --data "D:/Games/Serious Sam 2" --level dm_dockingbay
ss2-app --forget-data                # forget the chosen game folder: the first-run wizard shows again
ss2-app --level dm_dockingbay --screenshot out.png --size 1920x1080   # offscreen picture, no window
```

The game data folder is `--data`, else `SS2_DATA_DIR`, else the folder you chose in the **first-run wizard** the last
time; each is checked again at every start. When none of them is usable (the first start, or the folder moved), the
window first shows the wizard: it lists the installs it finds (Steam, the registry, common folders), or you type the
folder that holds `All_PC_01.gro`, it checks the folder and remembers it (`data_location.toml` next to
`settings.toml`). On the path screen Ctrl+V pastes a copied path (Windows). **Choose Game Folder...** on the main menu
shows the wizard again to pick another folder without restarting ("Back to Menu" keeps the current one), and
`ss2-app --forget-data` forgets the choice so that the wizard shows at the next start; see [First run](FIRST_RUN.md). `--data` skips the wizard. The level list also offers the built-in **Test room**. A level is named by its
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

- Levels have their baked lightmaps (about 99 % of lit objects; the rest keep a flat light from the camera), their
  sky, distance haze and bloom. There are no water surfaces, animated models or dynamic lights yet.
- In a haze the sky stays visible. The original draws the whole sky in the flat fog colour there; to get that, start
  with `--set sky_fog=on` (or set `sky_fog = true` in `settings.toml`).
- Collision uses each model's own collision mesh; the few models without one (about 10 of 2,000) collide with
  their coarsest drawn shape.
- Single-player levels start where the original starts them (the world's first chapter).
- A few world-embedded models with nothing to draw are left out (listed in the log).
