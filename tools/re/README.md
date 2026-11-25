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

Weapon classes (docs/re/weapon-classes.md):

```
python -I tools/re/weapon_classes_verify.py "C:\path\to\Serious Sam 2"    # which of the 83 exported weapon classes are in the data and under which name, the member order of 14 data classes against the names the binary declares (the rule that names the numbered weapon members), the weapon files per class and their base members by name (about 4 minutes; the class and member names it checks were read from the binary, the script reads data only)
```

Brain classes (docs/re/brain-classes.md):

```
python -I tools/re/brain_classes_verify.py "C:\path\to\Serious Sam 2"    # that none of the 61 exported brain classes is in any type table, the data classes with a member named after a brain (and in how many files), which puppet entity kinds are in the data, and the AI moment names of the 82 .cb files against the moment names the brain code raises (about 3 minutes; the class and moment names it checks were read from the binary, the script reads data only)
python -I tools/re/brain_assignment_verify.py "C:\path\to\Serious Sam 2" [LIMIT]    # docs/re/brain-assignment.md: the puppet entity classes the worlds place, which carry a character behavior (.cb), the behavior against the parameter file, the default state against the behavior, the spawner links and the seats (about 20 minutes; LIMIT reads only the smallest worlds)
```

AI behaviour and action classes (docs/re/behavior-classes.md and its three companion pages):

```
python -I tools/re/behavior_classes_verify.py "C:\path\to\Serious Sam 2"    # which of the 95 exported AI behaviour and action classes are in the data (45 by own name or S twin), their base, members and files, the member order of 14 classes against the binary's declared names, the objects of every class in the 82 .cb, 322 .ep and 59 .wld files with the stored value of every member, the term class on each action class and the movers of the bands (about 10 minutes; the class names it checks come from the binary's export table)
```

Puppet classes (docs/re/puppet-classes.md and its four companion pages):

```
python -I tools/re/puppet_classes_verify.py "C:\path\to\Serious Sam 2"    # which of the 101 exported puppet classes are in the data (50), the member-order rule against the declared member tables (16 classes), the root classes of the 124 puppet parameter files, and what the install does not contain (about 5 minutes)
Projectile classes (docs/re/projectile-classes.md):

```
python -I tools/re/projectile_classes_verify.py "C:\path\to\Serious Sam 2"    # which of the 32 exported projectile classes are in the data, the member order of 13 data classes against the names the binary declares, the value tables of the 94 projectile files, SPuppetProjectileParams, CLaunchVelocities and the global bullet and flame parameters, and the stored shooters, launchers and crumbs (about 6 minutes)
```

Item and pickup classes (docs/re/item-classes.md and its two companion pages):

```
python -I tools/re/item_classes_verify.py "C:\path\to\Serious Sam 2"    # which of the 113 exported item classes are in the data (97) and under which name, the member order of 16 data classes against the names the binary declares, the 51 CBaseItemParams respawn numbers and the 8 tool files with an item record, and the census of the 2,730 placed items (class members, models, flags, spawner and parameter links, 4,222 records) and of the 665 ammunition containers (about 12 minutes; the class and member names it checks were read from the binary, the script reads data only)
```

Level mechanism, spawner and info entity classes (docs/re/mechanism-classes.md and its five companion pages):

```
python -I tools/re/mechanism_classes_verify.py "C:\path\to\Serious Sam 2"    # which of the 82 exported classes are in the data (70) and under which name, the member order of 23 data classes against the binary's declared names, the census of the 59 worlds (7,651 placed entities and their records: member value tables, the spawner limit type against the spawnee class, the chapter links), the two spawner tweak files and ChapterMaxes.txt (about 12 minutes; --no-worlds skips the census, --short prints fewer values)
```

World content entity classes (docs/re/worldcontent-classes.md and its four companion pages):

```
python -I tools/re/worldcontent_classes_verify.py "C:\path\to\Serious Sam 2"    # which of the 55 exported world-content classes are in the data (25: 11 under the C name, 12 as the S twin, 2 under the registered name CEffectEntity/CEffectProperties), the member order of 28 data classes against the names the binary declares, and the census of the placed entities and records of the family (static models, props, particle effects, sounds, flares, markers, vis boxes, nav-mesh markers, shakers, text effects): member values, entity-versus-record copy checks, flag cross tables, construction defaults, link relations and the hard-coded path marker (about 15 minutes; the class and member names it checks were read from the binary, the script reads data only)
```

HUD classes (docs/re/hud-classes.md and its three companion pages):

```
python -I tools/re/hud_classes_verify.py "C:\path\to\Serious Sam 2" [--all]    # which of the 95 exported HUD classes (plus the 4 of the player part) are in the data (47 saved, each in 1 file; 2 renamed), the member order of 31 data classes against the binary's names (141 members), the 44 entries of Databases/HudParams.rsc (positions, stacks, configuration records, textures), SimpleHudBarParams.rsc and the ammunition and weapon icon files against the enumeration captions; --all scans all 8,063 format-8 files (about 4 minutes)
```

Menu screen classes (docs/re/menu-classes.md and its six companion pages):

```
python -I tools/re/menu_classes_verify.py "C:\path\to\Serious Sam 2"    # the 104 exported menu classes (1,412 functions, 684 named) against the 99 menu layouts: which layout belongs to which class, handler names against gadget names (263 of 372 name a gadget of their own layout), the binary's declared member names of 25 gadget classes against the data's type tables (22 match in full), option lists against the code's tables (enemy percentages, difficulties, quality levels, connection presets), the popup layouts, the mod trees' own Menus folders, the menu parameters (a fraction of a minute)
python -I tools/re/helper_classes_verify.py "C:\path\to\Serious Sam 2" [--sections=1234]    # the 59 exported helper classes (471 functions, 80 named methods) against the data: 30 in the type tables, 2 more under other names; the 1,313 animation events by class with flags and own members; the constructors' defaults against the global parameter files (CSamNetworkParams 8 of 8, CBulletParameters 13 of 14), CThrowingParams (252 values) and SPuppetEnvironmentParams (258 values, third member never above the fourth); about 3 minutes
python -I tools/re/class_inventory_check.py [REPO_DIR]    # reads only docs/re: the 932-row class inventory (docs/re/class-inventory-1.md and -2.md) against the pages of the twelve parts: one row per class, per-part class and function counts, the data column's totals, every covering page names the class or its family rule, every page is in SUMMARY.md; a few seconds
```
