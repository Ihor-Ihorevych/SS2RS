# tools/re — reproducible evidence for the specs in docs/re/

Plain Python 3 scripts (standard library only) that read an install's `.gro` archives **read-only**. They exist so a reviewer can re-run the numbers quoted in the specs.

```
python -I tools/re/validate_ctsemeta.py "C:\path\to\Serious Sam 2"        # frame validation counts
python -I tools/re/walk_objects.py "C:\path\to\Serious Sam 2"             # object-data decode: how many files decode to EDOB
python -I tools/re/inventory.py "C:\path\to\Serious Sam 2" inventory.md   # regenerate docs/re/inventory.md
```

They never write game content. Scripts that analyze the original *binaries* do not belong here (CLEANROOM.md).

Texture evidence (docs/re/tex.md):

```
python -I tools/re/tex_verify.py "C:\path\to\Serious Sam 2"       # size checks for every .tex, flag/wrap/dimension statistics
```

`ctse_values.py` is the value decoder these scripts share (objects -> Python values).

Mesh and extra-record evidence (docs/re/bmf.md, ctsemeta-objects.md):

```
python -I tools/re/bmf_verify.py "C:\path\to\Serious Sam 2"    # per-surface layout checks for every .bmf
python -I tools/re/edob_verify.py "C:\path\to\Serious Sam 2"   # the EDOB extra-record rule for every non-world file
```

CTSEMETA leftovers (docs/re/ctsemeta-leftovers.md):

```
python -I tools/re/ctse_leftovers_verify.py "C:\path\to\Serious Sam 2"                  # old formats 4-7, INFO[0], EDTY, RFIL/EXOB flags (about 4 minutes)
python -I tools/re/ctse_leftovers_verify.py "C:\path\to\Serious Sam 2" --only codes     # entity-class codes of the 59 worlds (slow)
python -I tools/re/dtty_verify.py "C:\path\to\Serious Sam 2"                            # docs/re/dtty-kinds.md: DTTY kind and flags statistics
```

Skeleton evidence (docs/re/skl.md):

```
python -I tools/re/skl_verify.py "C:\path\to\Serious Sam 2"    # hierarchy rules and the local-transform / inverse-bind check
```

Model evidence (docs/re/mdl.md):

```
python -I tools/re/mdl_verify.py "C:\path\to\Serious Sam 2"    # structure, embedded meshes, palettes, children, references (about 5 minutes)
```

Animation evidence (docs/re/ans.md):

```
python -I tools/re/ans_verify.py "C:\path\to\Serious Sam 2"    # keyframe rules for every envelope, bone ids, events (about 5 minutes)
```

Material evidence (docs/re/mtr.md):

```
python -I tools/re/mtr_verify.py "C:\path\to\Serious Sam 2"    # preset structure, shader class codes, texture slots, modifiers (about 5 minutes)
```

World evidence (docs/re/ctsemeta-objects.md, "CMetaHandle" and "The EDOB section"):

```
python -I tools/re/wld_verify.py "C:\path\to\Serious Sam 2"    # all 59 worlds: objects, extra records, CMetaHandle values (about 20 minutes)
```

`walk_objects.py` now reports 9,269 of 9,269 file entries decoding (about 11 minutes).

World root evidence (docs/re/wld-root.md):

```
python -I tools/re/wld_root_verify.py "C:\path\to\Serious Sam 2" [N]    # relations of the CWorld root in all 59 worlds (about 20 minutes); N = only the N smallest
```

World geometry evidence (docs/re/wld-geometry.md):

```
python -I tools/re/wld_geometry_verify.py "C:\path\to\Serious Sam 2" [N]    # static models, hulls, collision grid cells in all 59 worlds (about 20 minutes)
```

Texture flag evidence (docs/re/tex-flags.md, tex-flags-tables.md):

```
python -I tools/re/texflags_verify.py "C:\path\to\Serious Sam 2" [--worlds N] [--sample N] [--tables docs/re/tex-flags-tables.md]    # flag bits against format, folder and size; wrap codes with a seam test; format-code content classes; embedded lightmaps and their baker record (about 10 minutes)
```

Lighting evidence (docs/re/lighting.md, lighting-tables.md):

```
python -I tools/re/shading_verify.py "C:\path\to\Serious Sam 2" [--tables docs/re/lighting-tables.md] [LIMIT]    # light kinds and flags, baked shadow buffers (CShadowBuffer), the GI object, EntityShadowType against lightmaps, modifiers next to lightmaps (about 20 minutes)
```

World lighting evidence (docs/re/wld-lighting.md):

```
python -I tools/re/wld_lighting_verify.py "C:\path\to\Serious Sam 2" [N]    # lights, baked lightmaps, flares in all 59 worlds (about 20 minutes)
```

Entity parameter evidence (docs/re/ep.md, docs/re/ep-schema.md):

```
python -I tools/re/ep_schema.py "C:\path\to\Serious Sam 2" --check [--worlds N]    # structure, references, entity links (all worlds: about 20 minutes)
python -I tools/re/ep_schema.py "C:\path\to\Serious Sam 2" --markdown docs/re/ep-schema.md    # regenerate the schema reference
```

Font evidence (docs/re/fnt.md):

```
python -I tools/re/fnt_verify.py "C:\path\to\Serious Sam 2" [--png OUT_DIR]    # glyph tables, code coverage; optional rendered samples
```

`texpng.py` decodes a `CStaticTexture` (tex.md) to RGBA and writes PNG files; the other scripts import it.

Physical material and scheme evidence (docs/re/mat.md):

```
python -I tools/re/mat_verify.py "C:\path\to\Serious Sam 2"    # .mat, .scs, .scp, .sca, .mas and who refers to them
```

Resource database and character tool evidence (docs/re/rsc-ctl.md, docs/re/rsc-ctl-schema.md):

```
python -I tools/re/ep_schema.py "C:\path\to\Serious Sam 2" --check --ext .rsc,.ctl    # roots and references of all .rsc and .ctl files
python -I tools/re/ep_schema.py "C:\path\to\Serious Sam 2" --markdown docs/re/rsc-ctl-schema.md --ext .rsc,.ctl --title "..."    # regenerate the schema
```

Particle effect evidence (docs/re/pfx.md, docs/re/pfx-schema.md):

```
python -I tools/re/pfx_verify.py "C:\path\to\Serious Sam 2"    # structure, emitter shapes, graph holders, enumerations
python -I tools/re/ep_schema.py "C:\path\to\Serious Sam 2" --check --ext .pfx    # roots and references
```

World visibility evidence (docs/re/wld-visibility.md):

```
python -I tools/re/wld_vis_verify.py "C:\path\to\Serious Sam 2" [LIMIT]    # vis boxes and groups, the active list, the 2D grids (about 15 minutes for all worlds)

Flare, text-effect and vibration-effect evidence (docs/re/fx.md):

```
python -I tools/re/fx_verify.py "C:\path\to\Serious Sam 2" [--refs]    # .lfx / .tfx / .vfx classes, ranges, graph patterns (seconds; --refs adds a few minutes)
```

Smaller file types (docs/re/misc-types.md, misc-schema.md):

```
python -X utf8 tools/re/misc_verify.py "C:\path\to\Serious Sam 2"    # .nfo .dsc .srt .crt .preload .cfg .bin and the .cb .sfc .pss .ctr .mch .mac files
python -I tools/re/ep_schema.py "C:\path\to\Serious Sam 2" --markdown docs/re/misc-schema.md --ext .cb,.sfc,.pss,.ctr,.mch,.mac,.sca,.mas --title "..."
```

Shader program evidence (docs/re/shaders.md):

```
python -X utf8 tools/re/shader_verify.py "C:\path\to\Serious Sam 2"    # GPU-Programs.asm: programs, families, version tags, fragments (prints numbers only)
```

Class catalogue (docs/re/classes.md, classes-catalog.md):

```
python -X utf8 tools/re/class_catalog.py "C:\path\to\Serious Sam 2" --markdown docs/re/classes-catalog.md [--modules modules.json]    # about 30 seconds
```

AI evidence (docs/re/ai.md):

```
python -X utf8 tools/re/ai_verify.py "C:\path\to\Serious Sam 2" [--worlds 6] [--table]    # .cb behaviors; with --worlds also spawners, tactics, markers, nav mesh
```

Player and global rules (docs/re/player-rules.md):

```
python -X utf8 tools/re/rules_verify.py "C:\path\to\Serious Sam 2"    # difficulty tables, power-up durations, damage multipliers, item numbers, player puppet values
```

Audio assets (docs/re/audio.md):

```
python -X utf8 tools/re/audio_verify.py "C:\path\to\Serious Sam 2"    # .wav and .ogg formats, rates, channels, folders (about a minute)
```

Movies (docs/re/cutscenes.md):

```
python -X utf8 tools/re/movie_verify.py "C:\path\to\Serious Sam 2"    # Theora/Vorbis headers of all movies, .nfo references, .srt pairing (about a minute)
```

World entity evidence (docs/re/wld-entities.md, docs/re/wld-entity-schema.md):

```
python -I tools/re/wld_entity_schema.py "C:\path\to\Serious Sam 2" --markdown docs/re/wld-entity-schema.md [--json OUT.json]    # entity and record classes, members, census (about 15 minutes)
python -I tools/re/wld_entity_verify.py "C:\path\to\Serious Sam 2" [LIMIT]    # spawners, record hierarchy, .ep links (about 15 minutes)
```

Per-character health and score table (docs/re/puppet-stats.md, game-modes.md):

```
python -X utf8 tools/re/puppet_stats.py "C:\path\to\Serious Sam 2" --markdown docs/re/puppet-stats.md
```

Localization (docs/re/localization.md):

```
python -X utf8 tools/re/l10n_verify.py "C:\path\to\Serious Sam 2"    # CTransString counts and key styles, Locales/ files (about a minute)
```

World and `.mac` scripting evidence (docs/re/wld-macro.md, wld-macro-schema.md, wld-macro-vocab.md):

```
python -I tools/re/wld_macro_verify.py "C:\path\to\Serious Sam 2" [--vocab docs/re/wld-macro-vocab.md] [LIMIT]    # programs, trees, text, entities, variables (about 15 minutes)
python -I tools/re/wld_entity_schema.py "C:\path\to\Serious Sam 2" --markdown docs/re/wld-macro-schema.md --match "Macro|OnHandler|Variant|HintedVar|LatentID|SingleEvent|EntityVariable|EntityPropertyVariable|Expression|VarAssignment|VarTypeHint|ObjectAction" --also-ext .mac --title "..."    # the class reference
```

World physics evidence (docs/re/wld-physics.md, wld-physics-schema.md):

```
python -I tools/re/wld_physics_verify.py "C:\path\to\Serious Sam 2" [LIMIT]    # mechanisms, parts, bodies, hulls, collision grid cells, joints, motion handlers (about 15 minutes)
python -I tools/re/wld_entity_schema.py "C:\path\to\Serious Sam 2" --markdown docs/re/wld-physics-schema.md --match "Mechanism|Body|Hull|Joint|MotionHandler|CMotionHandler|CollisionMesh|CollisionLayer|CollisionDomain|CollisionGrid|PhysicsDomain|FluidHull|ForceField|JointLimit|JointValues|PrimitiveDesc" --title "..."    # the class reference
```

World sound and effect evidence (docs/re/wld-sound-effects.md, wld-sound-effects-schema.md):

```
python -I tools/re/wld_sound_verify.py "C:\path\to\Serious Sam 2" [LIMIT]    # sources, channels, sync list, environments, effect instances against the .pfx (about 15 to 30 minutes)
python -I tools/re/wld_entity_schema.py "C:\path\to\Serious Sam 2" --markdown docs/re/wld-sound-effects-schema.md --match "Sound|Feeder|DynamicMusic|SfxEnv|EffectInstance|EffectRenderable|ParticleEffectInstance|ParticleEffectSound|EffectEntity|EffectProperties|EnvSFX|ShakerEffect|ShakerRenderable|TextEffect" --title "..."    # the class reference
```

World navigation evidence (docs/re/wld-navigation.md, wld-navigation-schema.md):

```
python -I tools/re/wld_nav_verify.py "C:\path\to\Serious Sam 2" [LIMIT]    # nav mesh nodes, links and grid, path marker chains, tactics and movers, formations (about 15 minutes)
python -I tools/re/wld_entity_schema.py "C:\path\to\Serious Sam 2" --markdown docs/re/wld-navigation-schema.md --match "NavMesh|PathFinding|PathMarker|Formation|Beacon|MarkersMoving|CMC[A-Z]|Nav[A-Z]|SMoveCharacter|Locator|SpawnMarker|Tactic" --title "..."    # the class reference
```

Level logic evidence (docs/re/level-logic.md, level-logic-api.md):

```
python -I tools/re/wld_logic_verify.py "C:\path\to\Serious Sam 2" [--api docs/re/level-logic-api.md] [LIMIT]    # what scripts do to which entity class, chapter chain, variable names (about 20 minutes)
```

Environment evidence (docs/re/environment.md, environment-levels.md, environment-schema.md):

```
python -I tools/re/wld_env_verify.py "C:\path\to\Serious Sam 2" [--table docs/re/environment-levels.md] [LIMIT]    # sky, haze, bloom, fluids per level and record/renderable copies (about 15 minutes)
python -I tools/re/wld_entity_schema.py "C:\path\to\Serious Sam 2" --markdown docs/re/environment-schema.md --match "Haze|Bloom|CubeRenderable|CubeBackground|SimpleBackground|FluidEntity|FluidHull|ShakerEffect|ShakerRenderable|EnvironmentDamager|ForceField|CShakingParams" --title "..."    # the class reference
```

Destruction evidence (docs/re/destruction.md, destruction-models.md):

```
python -I tools/re/destruction_verify.py "C:\path\to\Serious Sam 2" [--table docs/re/destruction-models.md] [LIMIT]    # holders, configurations, effects, substitute chains in all models; use in worlds (about 12 minutes)
```

Vehicle and turret evidence (docs/re/vehicles.md, vehicles-table.md):

```
python -I tools/re/vehicles_verify.py "C:\path\to\Serious Sam 2" [--table docs/re/vehicles-table.md] [LIMIT]    # aircraft, turret, hovercraft and rolling-ball parameter files and their placed entities in all worlds (about 15 minutes)
```

Camera and weapon view evidence (docs/re/camera-view.md, camera-tables.md):

```
python -I tools/re/camera_verify.py "C:\path\to\Serious Sam 2" [--table docs/re/camera-tables.md]    # the 18-camera database, the 16 weapon view placements, camera names used by puppets, the player's view parameters (about 3 minutes)
```

Hit effects, blood and gib evidence (docs/re/effects-pipeline.md, effects-tables.md):

```
python -I tools/re/effects_verify.py "C:\path\to\Serious Sam 2" [--table docs/re/effects-tables.md] [LIMIT]    # material particle schemes, puppet death parameters, prop and placed effects, decal/gib name search (about 15 minutes)
```

Weapon evidence (docs/re/weapons.md, weapons-tables.md):

```
python -I tools/re/weapons_verify.py "C:\path\to\Serious Sam 2" [--table docs/re/weapons-tables.md]    # the 17 weapon parameter files (single player and deathmatch) and the projectile files they use (about 1 minute)
```

Character evidence (docs/re/enemies.md, enemies-tables.md):

```
python -I tools/re/enemies_verify.py "C:\path\to\Serious Sam 2" [--table docs/re/enemies-tables.md]    # the 124 puppet parameter files: health, score, speeds, attacks, and the 73 projectile files of the shooting blasts (about 1 minute)
```

Boss fight evidence (docs/re/bosses.md, bosses-tables.md):

```
python -I tools/re/bosses_verify.py "C:\path\to\Serious Sam 2" [--table docs/re/bosses-tables.md] [LIMIT]    # the seven boss characters, their behavior states and the events, actions and calls the arena scripts address to each boss (about 15 minutes)
```

Menu evidence (docs/re/gui.md, menus-tables.md):

```
python -I tools/re/menus_verify.py "C:\path\to\Serious Sam 2" [--table docs/re/menus-tables.md]    # the 99 menu screen layouts: gadgets, link graph, platform masks, button bars (about 1 minute)
```

Main loop and time evidence (docs/re/main-loop.md):

```
python -I tools/re/mainloop_verify.py "C:\path\to\Serious Sam 2"    # the configuration scripts of the install and every line that sets a timing variable (a few seconds); the loop itself is specified from the binaries and has no script
```

Save-game evidence (docs/re/save-game.md):

```
python -I tools/re/savegame_verify.py "C:\path\to\Serious Sam 2" [--skip-worlds]    # the save-data pointer and save counter of every world version (about 6 minutes), the classes no data file defines, the 46 .preload files, the 3 shipped demos and their .nfo; a save game itself cannot be reproduced from the install (there is none)
```

Replication evidence (docs/re/replication.md):

```
python -I tools/re/replication_verify.py "C:\path\to\Serious Sam 2"    # decodes the 3 shipped demos to the last byte (message counts, the opening snapshot, per-step traffic, the share of empty updates) and counts configuration lines that set net_, ser_ or cli_ variables (a few seconds); the replication rules themselves are specified from the binaries and have no script
```

Player, session and inventory classes (docs/re/players-session-classes.md):

```
python -I tools/re/players_session_classes_verify.py "C:\path\to\Serious Sam 2"    # which of the 52 exported classes of the family are in the data and under which name, that the other 42 are in no type table, the files that carry the resource classes, and the member order of SPlayerPuppetParams against the 24 names the binary declares (about 2 minutes; the class names it checks were read from the binary's export table, the script reads data only)
```
