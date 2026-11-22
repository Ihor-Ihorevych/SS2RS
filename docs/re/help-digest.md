# Digest of `Help/SeriousEngineTools.chm`

- **Roadmap task:** T01-002
- **Author:** @lxstyo/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** that the summaries say what the help file says; **medium** as evidence about the *shipped game* (the help describes the Serious Engine 2 SDK, version 2.001 in 2006; the game is later and its data is patched to 2.062–2.080). Every claim used as evidence must be re-checked against the data, as the "Confirms" column does.

## Summary

The install ships one help file, `Serious Sam 2/Help/SeriousEngineTools.chm` (9.1 MB): the documentation of **Serious Engine 2 SDK** and its editor **Serious Editor 2** (SEd2), written by Croteam in 2006. It is a Doxygen page tree of 343 HTML pages and 179 screenshots. About 190 of the pages are one-paragraph descriptions of editor tools; the rest explain engine concepts, resource types, the macro language, the Serious Sam 2 entity set and 19 tutorials.

It is the only first-party description of the content pipeline that we have. It never gives byte layouts, so it cannot replace the specs in this folder, but it names things (entity classes, properties, defaults, terms) and explains intent, which tells us what to look for and what to call it.

## Evidence

- Extracted with a general archive tool into a temporary directory outside the repository; converted to plain text; read page by page for the 60-odd pages that describe the engine, the resource types and the game, and skimmed for the tool pages (every tool page is a one-paragraph description of an editor command and is summarized as a family below).
- Nothing from the help file is stored in the repository except this digest. **No page text, screenshot or tutorial level is copied**; the summaries are in our words (`CLEANROOM.md`: "Croteam's help files verbatim" are forbidden). The full per-page notes stay on the author's machine.
- Where a statement can be tested against the install, the "Confirms" column names the spec that already does so, or the task that must.

## How the help file is organized

Page counts per branch are approximate (the table of contents repeats some pages under several parents).

| Branch | Pages | What it holds |
|---|---|---|
| Serious Engine 2 | about 24 | overview, meta data, the resource types, the macro language, particle effects |
| Serious Editor 2 | about 80 | the editors (mesh, world, skeleton, animation, collision, destruction, model), the GUI and its controls, a "tips" section |
| Serious Sam 2 | 8 | characters, camera, enemy behavior, the entity catalogue, hovercraft, ragdoll |
| Plugins | 1 | export/import plug-ins for a third-party 3D package (geometry and animation exchange files) |
| Step-by-step tutorials | 19 | room, mechanism, door, terrain, spawning, particles, destruction, custom character, cutscene, custom level, chapters, distribution, macro basics, a keypad |
| Related pages | about 210 | resource structures, shader pages, and one page per editor tool |

## What the engine pages say

| Topic | Summary (our words) | Confirms / feeds |
|---|---|---|
| Meta data | Every piece of data carries a description of its own structure, so one generic loader and one generic property editor serve all types; adding a field to a class needs no extra I/O code. | The self-describing container: `ctsemeta-container.md`, `ctsemeta-types.md`. |
| Resources | A resource is anything that makes sense as its own file. A resource is either **shared** (own file) or **local** (embedded in a parent); the editor converts between them ("unshare"; "save as"). | Embedded resources and `RFIL` references: `ctsemeta-types.md`; embedded lightmap textures in worlds: `wld-lighting.md`. |
| Document types | Worlds are `.wld`, models `.mdl`, meshes `.bmf`, skeletons `.skl`, fonts `.fnt`; scripts are text. | Matches `inventory.md`. The first page of the model section names older extensions (`.msh`, `.dtn`) that are **not** in the install; the mechanism templates are `.mch` (8 files). |
| Textures | Created from 24- or 32-bit source images. Kinds: static texture, colour texture (editable operations), texture modifier (changes sampling without touching the source). Flags: mip-maps, "constant" (not reduced on weak machines), an alternate fast compressor. Formats: 32/16-bit opaque, 8-bit grayscale, 4-bit compressed, 32/16-bit translucent, grayscale+alpha, compressed translucent. Wrap per axis: repeat, clamp to edge, clamp to border colour. | `tex.md`; the flag bits and wrap codes still open (T01-054). |
| Mesh | What is edited is an *edit mesh*, compiled to a render mesh at edit time. Geometry is organized in **maps**: a vertex is a list of entries, at most one per map; map kinds are position (the one mandatory morph map), UV, weight, surface, layer. Several UV maps allowed (e.g. diffuse and shadow). Polygons with more than 3 vertices are triangulated on compile. Layers hold polygons; a *rendering* layer map makes each layer a level of detail with a **render distance** and a **fade length**; a *collision* layer map marks collision geometry. A surface is one batch with one or more shaders. | `bmf.md` (compiled form), `mdl.md` (LOD). Layer-distance semantics: T01-055. |
| World | One level is one world: all entities, plus the structures needed to run it (collision grid, light grid, navigation mesh). Exactly one `WorldInfoEntity` per world. | `wld-root.md`. |
| Collision mesh | Collisions work between any primitive types and between mesh and primitive; static scenery uses mesh collision, movers use primitives. | `wld-geometry.md`; T01-023 (middleware). |
| Navigation mesh | Baked from the world-info entity, starting at chapter markers or navigation-mesh markers; rays are cast in four directions from every valid spot to grow walkable cells, which are then merged into larger sections. Characters path on it and use "feelers" to avoid dynamic objects. Goal must be inside the mesh. | `CNavMesh` is inventoried for T01-012e. |
| Model | A model has aspects, each its own resource: mesh (required), skeleton (to move parts), animation set, mechanism (when it uses collision primitives), destruction holder (when it can break). Models have **children** attached to a bone or the root (the held weapon is the example). Four kinds of **scheme** hang off a model: effect, sound, animation, moving animation. | `mdl.md`, `skl.md`, `ans.md`; mechanisms and destruction are T01-057. |
| Animation set | An animated object has an *anim host* listing animation sets (just a grouping). An animation is a set of **envelopes** (for a model, one per bone); an envelope has **channels** (position x/y/z and rotation h/p/b for bones; field of view, fade, aspect for cameras). Frames are integers and need not start at 0; key frames hold values and the rest are interpolated, also at playback. **Events** fire at frames (e.g. a footstep that spawns effects according to surface) and carry rules such as "first loop only", "forward only", "reverse only". | `ans.md`; interpolation and events: T01-058. |
| Mechanism template holder | Collision and physics of a model is a tree of **parts**; a joint of several kinds (ball, cone, hinge, …) links a part to its parent; each part has any number of primitive hulls; its body type is **dynamic** (full physics, ragdoll parts), **hybrid** (no rotation, legged characters) or **kinematic** (pushes dynamic bodies, is pushed by nothing). A part named like a bone follows that bone and skips physics (for precise boss collision). | The classes `CDynamicBody`, `CHybridBody`, `CKinematicBody` found in worlds (T01-012i); templates T01-057. |
| Destruction holder | Named destruction configurations: particles, sounds, debris, substitute model, animations, and the conditions (e.g. damage needed) that trigger them. | T01-046, T01-057. |
| Schemes | A scheme maps a *term* (a state or event name such as "wound", "death", "alive") to a weighted list of variations of a resource (sounds, effects, animations). Schemes **inherit**: a missing term is searched in the base scheme, so male and female soldiers can share footsteps. | `mat.md` (`.scs`/`.scp`/`.sca`/`.mas`); how game code picks a variation is T01-043/T01-029. |
| Macro language | A thread-based visual script for game events: it waits on time or events and calls actions on entities. Variables: `Null` (global call), `This` (the object the function was called on), and *variants* typed by assignment (index, float, string, resource pointer, entity handle, entity-handle array, 3-vector, placement). Statements: action (sync/async), run (sync/async), send event, assign, declare (optionally global), wait, break, exit-all-threads, print, comment; if/else; for; for-each; operators and, or, not, comparisons, + − × ÷. A program is function blocks plus `On` handlers, each handler running as a new thread; a macro entity starts the function `Main` with itself as `This`. A page lists the built-in `mac…` helpers (random, integer divide/modulo, vector and quaternion construction and access, event sender). "Latent" actions take time and can be waited on. | `CMacroProgram`/`CMacroFunction`/`COnHandler`/`CVariantImp*` in `wld-root.md`; the stored form is T01-012g, the semantics T01-030. |
| Particle effects | An *effect* resource holds any number of particle effects of four kinds: atomic, snake, clouds (procedural), and *machine* (the general particle editor). Effect level: life type (once, looping with life −1, infinite), hot-spot and fall-off distance (distance fade), stretch over life and over distance (graphs), relative placement over time. Machine particles split into **emitter** (shape box/sphere/cylinder, pivot, launch direction, rate, life range, velocity, spin, size range, emit angle ranges) and **particle** (colour gradient, alpha, size/velocity/spin multipliers and damping, weight, rotation modifiers, rise speed, collision with bounce). Nearly every value can be driven by a graph over time. | `pfx.md` (T01-016, by another author) should list these; we have not re-checked. |
| Shaders and surfaces | Shaders are per surface (a surface holds an array of them): texture, UV map, stretch, offset, **blend type** (opaque, translucent, shade = ×2 modulate, add, add-alpha), depth compare (less, equal, less-or-equal, always), alpha test with a threshold, double-sided, full-bright. A **shader preset** is a resource that is an array of shaders. Surface properties: smoothing angle, double-sided, tangent UV map (for normal-map style shaders), modifiers. | `mtr.md` (shader presets). The blend-type meanings answer part of T01-059. |
| Shader modifiers | A separate system that overrides shader properties from above (surface or entity): constant, float graph, gradient, texture oven. Used to scroll textures, replace a texture with a local lightmap, force a flag. A per-object lightmap is the pair "local texture + full-bright flag". | `wld-lighting.md` (`CTextureOvenShaderModifier`); parameters T01-061. |
| Texture oven | Bakes a texture from the scene: gradient, height gradient (top projection), **light map** (blur, anti-aliasing, shadow precision, ambient brightness in −1…1, directional ambient occlusion with radius/blur/samples/fill, contrast) and noise variants. | Baked lightmaps in worlds: `wld-lighting.md`; T01-045. |

## What the Serious Sam 2 pages say

| Topic | Summary (our words) | Feeds |
|---|---|---|
| Characters | Players, enemies and vehicles are **puppets** configured by *puppet params* (an entity-parameter resource). Kinds: aircraft, hovercraft, legged, rolling ball, turret, wheeled vehicle. A model must face −z, have every needed animation, and defines named *children* (muzzle, foot) for interaction points. Animations carry event lists. State names ("terms") such as explosion, death, knock-out select weighted effect/sound/animation variations from the model's schemes. | `ep.md`, T01-026, T01-029, T01-042. |
| Camera | The puppet params name camera setups from `Databases/Cameras.rsc` for three cases: ridden by someone, third person, third-person death. Each camera has distance above/behind **as graphs over pitch**, a pitch range, and a name. | `rsc-ctl.md`; T01-042. |
| Enemy behavior | AI acts only when a character has a foe; *movers* are chosen by distance to the foe. Without a foe a character can **patrol** along linked path markers (a non-branching chain, may loop), either automatically at spawn ("auto patrol", first marker settable) or from a macro, which also gets an event at each marker. Default patrol speed and rotation multipliers are 0.35; 0.7 or less switches run to walk. Then tactics (cover, groups) follow. | `SPathMarkerEntity` in `wld-root.md`; T01-025. |
| Entities catalogue | The editor-visible entities, each with its properties: ammo and weapon pickups (the shipped ammo types and 15 weapon pickups), armor (5, 10, 25, 50, 100, 200 points) and health (5, 10, 25, 50, 100) pickups, power-ups (extra life, invisibility, invulnerability, 4× damage, jump, 5× score, 2× speed, strength), treasure sizes, and the non-gameplay ones: world info, chapter info, alarm, animated mover, barrier field, bloom, bouncer (with air-control and exit-speed numbers), camera, crumbs (batched decoration on surfaces), cube background, detector area, directional light, door, environment damager, external joint, flare, fluid, force field, formation marker, generic motor, haze, killer, locator, macro, mission objective, movie player, multi-spawner, navigation markers, particle effect, path marker, platform, point light, portal, prop. | `wld-root.md`, T01-012d, T01-026, T01-027. The pickup values are checkable against `.ep` data (T01-027). |
| Chapters | Spawn points are *chapter info* entities in a chain ("next chapter"); one is active at a time, set by a macro action; the world-info entity names the first. A chapter holds start inventory (weapons, custom ammo, ammo ratio, health, armor, preferred weapon), spawn limits per alignment and the next level; activating a chapter with a next level ends the current one, plays the outro movie from the level's `.nfo` and loads the next. | T01-030, T01-037, T01-040. |
| Level `.nfo` files | Episode and level descriptions: `EPISODE` / `LEVEL` kind, `NAME`, thumbnail, `MAP_TYPE` (single/coop/deathmatch), loading-screen model, animation, rotate flag, distance and name, and the intro/outro movies. The folder must carry the episode's name. | `inventory.md` (122 `.nfo`), T01-019, T01-040. |
| Distribution | A `.gro` is an ordinary zip whose paths start with `Content` and which must sit in a directory named `Serious Sam 2`; the editor can make one for the open document. | `gro-tool`; matches our reader. |
| Hovercraft, ragdoll | Setup recipes for a vehicle and a ragdoll (skeleton + mechanism). | T01-047, T01-057. |

## The editor pages (summarized by family, not page by page)

| Family | Pages | Gist |
|---|---|---|
| Mesh editor tools (modelling) | ~100 | Create, extrude, bevel, knife, subdivide, CSG, primitives, welding, selection, hiding, mirror, flip, path tools. |
| Mesh editor tools (UV and maps) | ~20 | Auto-map U/V/UV, fit, arrange, random offsets, create and merge maps, smoothing/surface/weight/collision/rendering map creation. |
| World editor tools | ~35 | Clone and update clones by name, select by class or radius, selection store and favorites, align, randomize, freeze, simulation start variants, list resources, environment snapshot, **bake** (all, view, selected, store, crumbs). |
| Simulation | ~10 | Start, reset, kill-all, go to time, spring, target tracker; "exclusive" simulation toggles. |
| Skeleton editor | ~8 | Create/rename/re-parent bones, import and re-import a skeleton, align bone to parent. |
| Animation editor | ~20 | Key-frame navigation, create/delete/set keys, auto-key, clear channel, view from envelope. |
| Collision editor | ~10 | Shapes, parts, joints, inter-collision table, default pairs, generate a mechanism from a model. |
| Destruction, model editors | ~5 | Interfaces for destruction configurations and for composing models. |
| GUI, controls, tips | ~40 | Panels, meta-tree/resource/array/colour/gradient/graph/macro/list controls, selection, guides, shortcuts, thumbnails, profiling, ambient occlusion, scrolling texture, color flicker. |
| Tutorials | 19 | Worked examples of the rules above; the interesting ones for specs are door/mechanism (joints and motors), spawning, destruction, custom character, cutscene camera, chapters, distribution. |

The tool pages matter for us only as a vocabulary: they name operations the editor performed on data we now read (for instance "Weight", "Surface", "Smoothing" and "Rendering" maps explain the map kinds seen in meshes), and none of them describes anything the game itself executes.

## What the help file confirms

1. **Self-describing meta data** is by design, not an accident of our parser (container and type specs).
2. **A resource is shared or local**; embedded resources therefore exist and are normal (`wld-lighting.md` embedded lightmaps).
3. **`.gro` is a zip with `Content/` at its root**, exactly as our archive reader treats it.
4. **Model = aspect resources + children + schemes**; the child list is how weapons attach (`mdl.md`).
5. **Animation = envelopes × channels × integer frames with interpolation and rule-gated events**; the frame numbering is not required to start at zero (check in `ans.md`).
6. **Physics bodies are dynamic, hybrid and kinematic**, matching the class names found in worlds.
7. **Scheme terms with weights and inheritance** exist; the `.scs/.scp/.sca/.mas` families in `mat.md` are their stored form.
8. **The macro language is thread-based with latent actions and `On` handlers**, matching the object classes in worlds.
9. **Blend types** are opaque, translucent, shade (×2 modulate), add, add-alpha.
10. **Navigation** is a baked ray-grown mesh with merged sections, plus path markers for patrols.

## What it does not tell us (open questions)

- Any byte layout, enum value or numeric default of the stored formats (those are the other specs).
- Version differences: the help is for SDK 2.001; the game data is 2.062 and later (T01-051).
- Anything about the game's runtime behavior beyond the entity property lists: tick rate, movement numbers, damage tables, AI timing (T01-022, T01-024, T01-028, T01-029, T01-038/039).
- The shader programs themselves (T01-020).
- The extensions `.msh` and `.dtn` it names do not exist in the install; whether they are older names of `.bmf` and a destruction resource is unconfirmed (T01-057).

## Test vectors

Not applicable (a digest). Two checkable statements, each verifiable from `docs/re/inventory.md` and the archive list: the install contains 8 `.mch` files and 122 `.nfo` files, and no `.msh` or `.dtn`.
