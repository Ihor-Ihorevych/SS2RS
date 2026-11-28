# tools/re — reproducible evidence for the specs in docs/re/

Plain Python 3 scripts (standard library only) that read an install's `.gro` archives **read-only**. They exist so a reviewer can re-run the numbers quoted in the specs.

```
python -I tools/re/validate_ctsemeta.py "C:\path\to\Serious Sam 2"        # frame validation counts
python -I tools/re/walk_objects.py "C:\path\to\Serious Sam 2"             # object-data decode: how many files decode to EDOB
python -I tools/re/ctsemeta_independent_verify.py "C:\path\to\Serious Sam 2"  # T01-050d: independent OBJS alignment, zero-record EDOB
python -I tools/re/inventory.py "C:\path\to\Serious Sam 2" inventory.md   # regenerate docs/re/inventory.md
```

They never write game content. Scripts that analyze the original *binaries* do not belong here (CLEANROOM.md).

Texture evidence (docs/re/tex.md):

```
python -I tools/re/tex_verify.py "C:\path\to\Serious Sam 2"       # size checks for every .tex, flag/wrap/dimension statistics
python -I tools/re/tex_legacy_verify.py "C:\path\to\Serious Sam 2"  # T01-083: legacy payload layouts and effective path counts
```

`ctse_values.py` is the value decoder these scripts share (objects -> Python values).

Mesh and extra-record evidence (docs/re/bmf.md, ctsemeta-objects.md):

```
python -I tools/re/bmf_verify.py "C:\path\to\Serious Sam 2"    # per-surface layout checks for every .bmf
python -I tools/re/bmf_details_verify.py "C:\path\to\Serious Sam 2"  # T01-055: winding, morph, material and collision measurements
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
python -I tools/re/wld_lightmap_uv_verify.py "C:\path\to\Serious Sam 2" [N]    # T01-406: the lightmap UV map id is a per-world constant, never a mesh UV id (about 5 minutes)
python -I tools/re/wld_lightmap_stream_verify.py "C:\path\to\Serious Sam 2" [N]    # T01-407: which UV stream a lightmapped instance actually uses (min/max, [0,1] + non-overlap rasterized test, shared across surfaces, cross-checked against the surface's own material) over all 59 worlds (about 5 minutes); --texels [--out DIR] rasterizes a few instances' candidate stream into their lightmap's own texel grid and reports coverage of the lit texels
python -I tools/re/wld_lightmap_colormap_verify.py "C:\path\to\Serious Sam 2"    # T01-414: checks the engine's second, parallel group of id'd per-draw stream slots (bmf.md member .13/.15, the colour-map twin of .12/.14) against every lit instance's own mesh, over all 59 worlds -- confirms it is always empty, so the oven modifier's UV id cannot resolve through it either (about 2 minutes)
python -I tools/re/wld_lightmap_tiebreak_verify.py "C:\path\to\Serious Sam 2" [N]    # T01-414: tests two deterministic tie-breaks for T01-407's "no candidate" / "several candidates" residual (13.4% of lit instances) -- lowest-slot-index among several, slot 0 as a last-resort fallback for none -- over all 59 worlds (about 2.5 minutes)
python -I tools/re/wld_lightmap_streamselect_verify.py "C:\path\to\Serious Sam 2" [N]    # T01-420: replaces T01-414's two tie-breaks and T07-044's shipped no-candidate rule with a two-stage rule -- a line-for-line port of the shipped loader's own fixed-256-grid, top-left-rule overlap rasterizer (ids inside [0,1] only), then, on the tie that measure usually leaves, the fraction of a candidate's own footprint that is "lit" baked pixel data in the instance's own lightmap -- over all 59 worlds (about 12 minutes); --case WORLD_SUBSTR prints one world's per-id tables and the rule's own pick; --meshes gives the cheaper (about 6 minutes) distinct-mesh bucket sizes and overlap-tie frequency without decoding any lightmap texture
python -I tools/re/wld_lightmap_littexel_verify.py "C:\path\to\Serious Sam 2" [N]    # T01-421: pins down T01-420's "lit texel" test (reference grey, tolerance, per-channel metric, mip level, exact rasterization) without changing it; default mode counts DXT1 punch-through (3-colour) blocks against how many ever select a transparent texel, over the N smallest worlds (about 2 minutes for 20); --sensitivity sweeps the tolerance and reference grey over platform_03/platform.mdl's known hard case; --platform-margin reruns the margin over every occurrence of that hard case in all 59 worlds (about 10 minutes)
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
python -I tools/re/ctse_duplicate_members_verify.py "C:\path\to\Serious Sam 2"    # T01-425: every class's own STMB member list, swept for a repeated member name (docs/re/ctsemeta-types.md, invariant 4); about 30 seconds
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
python -I tools/re/wld_env_verify.py "C:\path\to\Serious Sam 2" [--table docs/re/environment-levels.md] [LIMIT]    # sky, haze, bloom, fluids per level, record/renderable copies, and the sky's en_qvPlacement rotation/position distribution (T01-409) (about 15 minutes)
python -I tools/re/wld_entity_schema.py "C:\path\to\Serious Sam 2" --markdown docs/re/environment-schema.md --match "Haze|Bloom|CubeRenderable|CubeBackground|SimpleBackground|FluidEntity|FluidHull|ShakerEffect|ShakerRenderable|EnvironmentDamager|ForceField|CShakingParams" --title "..."    # the class reference
python -I tools/re/sky_faces_verify.py "C:\path\to\Serious Sam 2" [--dump DIR] [--top3]    # which CCubeRenderable face member is which cube direction and how its image tiles with its neighbours, from decoded-pixel edge matching (about 3 minutes)
python -I tools/re/water_models_verify.py "C:\path\to\Serious Sam 2"    # T01-070: which .mtr/.bmf/.mdl files embed a CWaterShaderArgs config, and which static model instances of which world resolve to one (a few minutes)
python -I tools/re/wld_bloom_cameratest_verify.py "C:\path\to\Serious Sam 2"    # T01-423: every world with 2+ CBloomEntitys (dm_hole), each one's world position, box, edge-fade inset and numeric members, and whether their boxes/insets can both be occupied by one camera position at once (a few minutes)
python -I tools/re/wld_bloom_members_verify.py "C:\path\to\Serious Sam 2" [--worlds dm_hole,1_3_village]    # T01-424: every CBloomProperties/CBloomRenderable instance's members read by STMB file position (not by name), telling apart the record's two same-named "21" members and labelling every renderable member with its settled BloomEffect role (a few minutes for the whole install, instant with --worlds)
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

Hitscan shot (docs/re/hitscan-shot.md):

```
python -I tools/re/hitscan_shot_verify.py "C:\path\to\Serious Sam 2"    # the six hitscan weapons' scatter, range, damage and pellet members (single player and deathmatch), the scatter angles, and the bullet descriptions of the enemy attacks by bullet type (about 2 minutes; the formulas it prints come from the spec, the script reads data only)
```

Projectile launch (docs/re/projectile-launch.md):

```
python -I tools/re/projectile_launch_verify.py "C:\path\to\Serious Sam 2"    # the charging, recoil and launch-velocity members of the seven projectile weapons, the projectile files they name with the pairs at fire power 0 and 1, the launch speed per charge, the zap gun's bullet count, the player's hand grenade and which of the 94 projectile files a player weapon names (about 1 minute; the formulas come from the spec, the script reads data only)
```

Weapon view (docs/re/weapon-view.md):

```
python -I tools/re/weapon_view_verify.py "C:\path\to\Serious Sam 2"    # the six sway members and the double-weapon fire delay of the 17 weapons, the eight members of the 16 rendering files (with the field of view in degrees), the sniper zoom curve and the player's view-spring numbers (about 1 minute; the formulas come from the spec, the script reads data only)
```

Weapon pickups (docs/re/weapon-pickups.md):

```
python -I tools/re/weapon_pickups_verify.py "C:\path\to\Serious Sam 2"    # the weapon index to parameter file table checked against the install (single player, deathmatch and rendering files with their classes), the leftover weapon files, the combo-weapons setting and that no world has the combo pickup (about 2 minutes; the file names come from the binary and are quoted in the spec, the script reads data only)
```

Mounted tools and throw descriptions (docs/re/mounted-tools.md):

```
python -I tools/re/tools_throw_verify.py "C:\path\to\Serious Sam 2"    # the CMountedTool records of the puppet files (identifier or pointer, the named .ctl files), the CThrowDescription identifiers and speed windows, and the throw behaviour actions of the .cb files (about 3 minutes; the lookup and mounting rules come from the spec, the script reads data only)
```

Character tools, carried objects and rides (docs/re/puppet-tools.md and docs/re/puppet-carry-rides.md):

```
python -I tools/re/tools_state_verify.py "C:\path\to\Serious Sam 2"    # the 68 .ctl character tools (empty animation terms, limbs, flags, stretch, health), the shield descriptions of the puppet files (the two files that have any) and the default riders (the one CCharacterMount) (about 1 minute)
```

Puppet parameter file loading (docs/re/puppet-params-loading.md):

```
python -I tools/re/puppet_params_verify.py "C:\path\to\Serious Sam 2" [--no-worlds] [LIMIT]    # the .ep files of the Puppets and EntityParams folders (counts, identifiers, duplicates, the _DM files) and the placed puppets' parameter pointers and en_idPuppet against the pointed file's stem (about 12 minutes with the worlds, 20 seconds with --no-worlds)
```

The dummy foe (docs/re/puppet-dummy-foe.md):

```
python -I tools/re/dummy_foe_verify.py "C:\path\to\Serious Sam 2" [--grep-all]    # the files and editor palette entries that name the dummy foe, the Puppets/Objects folder (about 20 seconds, 70 seconds with --grep-all, which reads every file of the install for the text DummyFoe)
```

Brain classes (docs/re/brain-classes.md):

```
python -I tools/re/brain_classes_verify.py "C:\path\to\Serious Sam 2"    # that none of the 61 exported brain classes is in any type table, the data classes with a member named after a brain (and in how many files), which puppet entity kinds are in the data, and the AI moment names of the 82 .cb files against the moment names the brain code raises (about 3 minutes; the class and moment names it checks were read from the binary, the script reads data only)
python -I tools/re/brain_assignment_verify.py "C:\path\to\Serious Sam 2" [LIMIT]    # docs/re/brain-assignment.md: the puppet entity classes the worlds place, which carry a character behavior (.cb), the behavior against the parameter file, the default state against the behavior, the spawner links and the seats (about 20 minutes; LIMIT reads only the smallest worlds)
python -I tools/re/brain_mover_verify.py "C:\path\to\Serious Sam 2" [--cb-only] [LIMIT]    # docs/re/brain-movement.md: the distance band lists of the 82 .cb files (counts, order, last distance, velocity adjuster) and the tactic and patrol data of the worlds (about 1 minute with --cb-only, about 20 minutes with the worlds)
python -I tools/re/brain_senses_verify.py "C:\path\to\Serious Sam 2" [--cb-only] [LIMIT]    # docs/re/brain-senses.md: the three ranges and the alignment of the 82 .cb files, the threat sensitivities and sense overrides of the placed puppets, the alignment table (about 1 minute with --cb-only, about 25 minutes with the worlds)
python -I tools/re/brain_rides_verify.py "C:\path\to\Serious Sam 2" [--ep-only] [LIMIT]    # docs/re/brain-rides.md: the seat data of the puppet parameter files (names, flags, ways) and the instance mounts and ride links of the placed puppets (about 2 minutes with --ep-only, about 25 minutes with the worlds)
```

AI behaviour and action classes (docs/re/behavior-classes.md and its three companion pages):

```
python -I tools/re/behavior_classes_verify.py "C:\path\to\Serious Sam 2"    # which of the 95 exported AI behaviour and action classes are in the data (45 by own name or S twin), their base, members and files, the member order of 14 classes against the binary's declared names, the objects of every class in the 82 .cb, 322 .ep and 59 .wld files with the stored value of every member, the term class on each action class and the movers of the bands (about 10 minutes; the class names it checks come from the binary's export table)
python -I tools/re/action_choice_verify.py "C:\path\to\Serious Sam 2"    # docs/re/behavior-action-choice.md: the 597 handlers (behaviour-wide, state, band) of the 82 .cb files, moment lists per level, weights, terms and vt_bNotTrue, CBADefaultAction / CBANoAction per band, the band period graph (under a second)
python -I tools/re/behavior_generic_verify.py "C:\path\to\Serious Sam 2"    # docs/re/behavior-generic-puppet.md: the 13 run-over records and the 10 run-over actions, the 7 local-move actions, the other generic actions, the foe-change chance of the 82 behaviours (under a second)
python -I tools/re/mover_steering_verify.py "C:\path\to\Serious Sam 2" [--no-wld]    # docs/re/behavior-mover-steering.md: where the circle, stay-in-area, pattern, diamond, mellon and air-attack movers are stored, every member value, the pattern and air-attack graphs and the behaviours' altitude graphs (about 5 minutes with the worlds, 15 seconds with --no-wld)
python -I tools/re/marker_movers_verify.py "C:\path	o\Serious Sam 2" [LIMIT]    # docs/re/behavior-marker-movers.md: the position of the path markers (primitive = record + 0.25 up), the next-link segment lengths, the single paths, and every stored value of the ordered, random and closest-to-foe movers against the chain the tactic marker lies on (about 15 minutes for all 59 worlds)
python -I tools/re/validity_terms_verify.py "C:\path\to\Serious Sam 2"    # docs/re/behavior-validity-terms.md: the validity-term objects by class, the 6 foe-angle objects, the 51 health terms by mode (with the puppets health), the 12 foe-assigning objects, the unused classes (0), the puppets armor, the test vectors
python -I tools/re/line_of_sight_verify.py "C:\path\to\Serious Sam 2"    # docs/re/brain-line-of-sight.md: the 124 puppet .ep files: stretch, the eye-point and target vectors of the body record, the sitting variant and the legged puppets pose heights
python -I tools/re/foe_manager_verify.py "C:\path\to\Serious Sam 2" [--no-worlds] [LIMIT]    # docs/re/foe-manager-numbers.md: the (active, threat) range pairs of the 82 .cb files with the foe manager sleep time, the reaction periods and auto-change chances, the 124 empty foe-assigning tables, the placed puppets own-alignment flag (worlds about 20 min), the test vectors
```

Puppet classes (docs/re/puppet-classes.md and its four companion pages):

```
python -I tools/re/puppet_classes_verify.py "C:\path\to\Serious Sam 2"    # which of the 101 exported puppet classes are in the data (50), the member-order rule against the declared member tables (16 classes), the root classes of the 124 puppet parameter files, and what the install does not contain (about 5 minutes)
python -I tools/re/puppet_attacks_verify.py "C:\path\to\Serious Sam 2"    # docs/re/puppet-attacks.md: the 123 shooting processes, 168 blasts, 71 melee attacks, 75 melee blasts, 13 run-over and 30 throw-description records of the 322 .ep files (value tables, graphs, projectile flags of the blasts), and the formulas of the page evaluated for its test vectors (melee direction, free-fall launch, tool throw); under a second
python -I tools/re/moving_anim_verify.py "C:\path\to\Serious Sam 2"    # docs/re/puppet-moving-anim.md: the 20 .mas files and 40 embedded moving-animation schemes (167 all / 90 body / 58 pose-transition records, value tables, base chains, models per scheme) and a reference implementation of the chooser run over the whole selector grid, with the page's test vectors; under a second
python -I tools/re/legged_ko_leap_ride_verify.py "C:\path\to\Serious Sam 2"    # docs/re/puppet-legged-sequences.md: the knock-out parameters of the 82 legged parameter files (10 can be knocked out), the 9 leap attacks, the 22 seats / ways to seat / 33 ride-and-rider pairs, and the formulas of the page for its test vectors; under a second
python -I tools/re/legged_diving_verify.py "C:\path\to\Serious Sam 2"    # docs/re/puppet-legged-diving.md: the breath period of the 86 legged parameter files (5 s in 84, 60 s in the two player files) and the periodic dive and drowning events of the page for its test vectors; under a second
python -I tools/re/puppet_record_members_verify.py "C:\path\to\Serious Sam 2"    # T01-151 (docs/re/puppet-record-members.md): the declared names of the 25 numbered members of the seven puppet record classes against the data order and types of the 126 puppet files (258 environment, 86 knock-out records), the class defaults against the data, the stated relations, the healing triples, the explosion thresholds and the falling-damage graph
python -I tools/re/puppet_spawning_verify.py "C:\path\to\Serious Sam 2"    # docs/re/puppet-spawning.md: the 835 spawn effect configurations (11 .sfc + 824 in worlds): stop chance, throw-direction flag, prepare / spawn-source / spawnee animations, and the formulas of the page; about 6 minutes (parses all worlds)
python -I tools/re/player_movement_verify.py "C:\path\to\Serious Sam 2"    # T01-410 (docs/re/player-movement-constants.md): the player's ground/water/fly-pose environment records, step and fall heights, jump velocity and slope, eye heights and model stretch, and the speed-power-up and freeze-time numbers, read from the two live player files (PlayerPuppet.ep, PlayerPuppet_DM.ep) and checked against the page's table; under a second
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
python -I tools/re/spawner_spawn_verify.py "C:\path	o\Serious Sam 2" [--worlds N]    # T01-165 (docs/re/spawner-spawn-side.md): the 11 external `.sfc` spawn effect configurations member by member and the branch each takes, and a census of the 59 worlds: spawner record count/group/delay combinations, spawn effect configurations by branch, launchers, the five formation classes (kind number, flags, sizes, weighted marker lists) and the multi-spawner template lists; about 10 minutes
python -I tools/re/door_mover_verify.py "C:\path	o\Serious Sam 2" [--worlds N]    # T01-167 (docs/re/door-mover-rules.md): the doors (animation names, closing delays, lock and detector flags, stored state), the animations of every door model with the two length formulas, the animation sets of the animated movers and the portals; about 10 minutes
python -I tools/re/trigger_area_verify.py "C:\path	o\Serious Sam 2" [--worlds N] [--jobs J]    # T01-169 (docs/re/trigger-area-rules.md): detector areas (custom and players-only flags, targets, parents), the killer cross-table with the branch the code takes for each row, the environment damagers' boxes and healing flags, locators, and the freezing numbers of the player parameter files; about 25 minutes with four jobs
python -I tools/re/chapter_restore_verify.py "C:\path\to\Serious Sam 2" [--worlds N]    # T01-172 (docs/re/chapter-restore.md): the chapter chains of all worlds (lengths, names, chapters off every chain, next level paths, EndGameChapter), the entities that carry a chapter link by class and by chapter position, the placed treasure and extra life items; about 10 minutes
python -I tools/re/player_start_verify.py "C:\path\to\Serious Sam 2" [--worlds N]    # T01-210 (docs/re/chapter-restore-players.md section 2.8/2.9): every CSpawnMarkerEntity world is a dm_* deathmatch level (16 of 16), and whether each world's chapter chain's own first chapter is also the first CChapterInfoEntity in entity-list order (it is not in 31 of 51 worlds, 3 of those off the chain entirely); about 3 minutes
python -I tools/re/chapter_start_ground_verify.py "C:\path\to\Serious Sam 2" [--radius N] [--worlds name1,name2]    # T01-411 (docs/re/chapter-restore-players.md section 2.10): en_henParent over all 307 CChapterInfoEntity records (14 set; which worlds' own first chapter is one of them), and every entity placed near the first chapter of the two named worlds (default 5_1_Village, 5_2_SeasideTownApproaching), classified as a static model or not; about 3 minutes
python -I tools/re/secrets_objectives_verify.py "C:\path\to\Serious Sam 2" [--worlds N]    # T01-170 (docs/re/secrets-objectives-beacons.md): secrets, mission objectives and navigation beacons member by member (states, texts, notify flags, scanner flag, chapter links) and the per-world counts; about 10 minutes
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
