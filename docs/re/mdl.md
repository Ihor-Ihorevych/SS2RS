# `.mdl` models

- **Roadmap task:** T01-008
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the top-level structure (which objects exist, what each reference points to) and for the embedded mesh (same layout as `.bmf`, checked on 7,351 surfaces), and, as of T01-057 (`Engine.dll`'s `mdlFindChild`, `mdlShowChild`/`mdlHideChild`/`mdlIsChildHidden`, `CMechanismTemplate::GetPartByName` and `CModelRenderable::GetEffectiveCullDistance`, all read in full), for child member `.1`'s type and `.5`'s meaning, for the physics templates' named-lookup role, and for the general mechanism of the three distances; **medium** for the rest of the attachment children and the animation host; **low** for the destruction and custom-data parts (destruction is specified in `destruction.md`; custom data is only listed here), which bone-palette ids beyond the skeleton address, `CMeshModifierList` and `CAnimQueueImp`. Scope: all **3,707** distinct format-8 `.mdl` files of the retail install with patches (2,000 bytes to 3.0 MB).

Prerequisites: `ctsemeta-container.md`, `ctsemeta-types.md`, `ctsemeta-objects.md`; `bmf.md` (meshes), `skl.md` (skeletons).

## Summary

A model is the **top-level renderable thing**: weapon, enemy, prop, debris, level architecture. It is a CTSEMETA file with one `CModelConfiguration` that ties together: a **mesh** (usually embedded as a complete `CMesh`, sometimes an external `.bmf`), an optional **skeleton** (embedded, external `.skl`, or none), an **animation host** that points to animation sets (`.ans`), **attachment children** (slots on bones, optionally holding another model), and optional **physics templates**, **destruction configuration** and **custom game data**. Materials are inside the mesh (`CShaderPreset`, see `bmf.md`).

## Evidence

`tools/re/mdl_verify.py` (read-only, `SS2_DATA_DIR`) over the retail install with all patches:

- All **3,707** files decode with the object rules; each has exactly one `CModelConfiguration` with `CResource.1 = 1` (one file also embeds a second, helper configuration with `CResource.1 = −1`).
- The meshes embedded in **3,517** models (4,087 LODs, 7,351 surfaces, 2.85 million triangles) pass the `.bmf` checks: all streams fit their buffers, all indices are local, all positions lie inside the LOD box, tangent signs are ±1, the UV count equals surface member 4. Exceptions, not errors: 9 surfaces do not reference every vertex, 27 have fewer than 99 % unit normals.
- The reference census (below) counts every reference in every model.
- **T01-057** additionally read, in full, `Engine.dll`'s `mdlFindChild`, `mdlShowChild`, `mdlHideChild`, `mdlIsChildHidden`, `CMechanismTemplate::GetPartByName` and `CModelRenderable::GetEffectiveCullDistance`.

## Objects of a model

```
CResourceFile                        (object 0)
CModelConfiguration                  (CResource.1 = 1)
  .1   Box3f                         bounding box (6 floats: min x,y,z, max x,y,z)
  .2   -> CSkeleton                  embedded (518) | external (70: 60 `.skl`, 10 `.rsc`) | null (3,119)
  .3   list of CMeshInstance         0 (69 models), 1 (3,631), 2 (7)
  .6, .7, .12  float                 three distances (see below)
  .8   list of -> CModelConfigChild  attachment children (0 in 3,456 models)
  .9   -> CModelAnimHost             always present
  .10  -> CMechanismTemplateHolder   physics: embedded (2,608) | external (40) | null (1,059)
  .14  -> CDestructionHolder         embedded (2,938) | null (769)
  .16  -> CSamCustomModelData        embedded (438) | null (3,269)
  .13, CResource.4                   only in the 29 models with extra records: they refer to editor records
CMeshInstance
  .1   -> CMesh                      embedded (3,517) | external .bmf (127) | null (1)
  .2   -> CMeshModifierList          null in 3,594, present in 51 (meaning unknown)
```

`CResource` members are as for the other resources (`.1` = 1, `.5` = −1, `.3` = 0, `.2` = `CResourceFile`).

**Embedded mesh.** `CMesh.1` → `CRenderMesh`, `CMesh.2` → `CCollisionMesh` or null; the rules of `bmf.md` apply unchanged (buffers in `GFXHANDLE`, element paths, planar streams, bone palettes, materials). 3,023 models have a collision mesh. A model with an **external** mesh (127 references) names a `.bmf` file in `RFIL`.

**Skeleton.** An embedded `CSkeleton` has the structure of `skl.md`. An external one is a `.skl` file (60 models) or a `.rsc` file (10 models; not yet specified, see T01-015).

### The three distances

`.6`, `.7` and `.12` are floats with a few common values: `.7` and `.12`: 1000.0 (2,553 models), 200.0 (144), `3.0e38` (about 188: a "no limit" sentinel); `.6`: 990.0 (2,305), 0.0 (403), `3.0e38` (168). They look like view/fade distances (compare `CRenderMesh.7/.8/.9` in `bmf.md`); **the general mechanism is now confirmed** (T01-057: `SeriousEngine::CModelRenderable::GetEffectiveCullDistance`, read in full): the renderable's effective cull distance is `max(|bbox.x|, |bbox.y|, |bbox.z|)` of the model's own bounding box (`CModelConfiguration.1`) **times a float read from the model configuration**, clamped to an explicit per-instance override (`CModelRenderable::SetCullDistance`, `wld-geometry.md` T01-060) when one is set and smaller. So **at least one of `.6`/`.7`/`.12` is a scale factor applied to the bounding box's half-extent** to compute the default cull distance when no instance overrides it; which of the three it is, and the roles of the other two, were not pinned down to an exact offset in this reading.

### Animation host

`CModelAnimHost` has one member, `CAnimHost.1`, a list of `CAnimSetInstance`; each instance has one member `CAnimSetInstance.1`, a reference to the animation set. Over all models there are 1,045 instances: **681 external `.ans` files** (all external references are `.ans`), **354 embedded `CAnimSet`** objects (with `CAnimation`, `CQuatAnimEnvelope`, `CVec3LenAnimEnvelope`), and 10 null. The animation format is specified separately (T01-010).

### Attachment children

`CModelConfiguration.8` lists `CModelConfigChild` objects (1,028 in 251 models):

| Member | Meaning (observed) |
|---|---|
| `.1` | **confirmed an `IDENT`** (T01-057: `SeriousEngine::mdlFindChild(CModelConfiguration*, IDENT)`, read in full, scans the children comparing exactly this member against the given id) — a **name** game code uses to find one specific attachment slot, not a plain serial number; this is why it is usually unique within a model (248 of 251) and explains the 49 cases that happen to equal a bone id as incidental overlap between a slot's name and a bone's, not a deliberate reference. Larger than every bone id of LOD 0 in 754 of 803 children of models with an embedded skeleton |
| `.2` | int: **parent bone id**: a bone id of the embedded skeleton's LOD 0 in 799 of 803 cases |
| `.3` | `QuatVect`: local placement (quaternion x,y,z,w + translation), as in `skl.md` |
| `.4` | `CModelInstance`: `.1` scale (`Vector3f`, 1,1,1 in the sample), `.2` `CModelAnimQueue` (`CAnimQueue.1` → `CAnimQueueImp`, always), `.3` → **another model** (an external `.mdl`: 215 children, for example `Models/Effects/Flares/Flare02.mdl`) or null (813 children: an empty slot); `.4`–`.6` always null |
| `.5` | **confirmed a hide flag** (T01-057: `SeriousEngine::mdlShowChild`/`mdlHideChild`/`mdlIsChildHidden`, read in full, all three operate on exactly this member): 0 **shown** (the class's own construction default) or 1 **hidden**; a reader should not render a child's filled model slot while this is set. The data's majority value, 1 (720 of 1,028), means **most attachment slots start hidden** and are shown later by game code (muzzle flashes, reveal effects and similar), not drawn unconditionally |

So a child is a **named slot on a bone**, optionally filled with another model (flares, lights, weapon attachments) and independently toggled visible or hidden. `CAnimQueueImp` has six members (empty lists and zeros in the sample); not analysed (still open, T01-057).

### Physics, destruction, custom data (listed only)

- `CMechanismTemplateHolder.1` is a list of `CMechanismTemplate`; those own physics body and joint templates: `CDynamicBodyTemplate`, `CPrimitiveHullTemplate`, `CHingeJointTemplate`, `CLinearMotionJointTemplate`, `CHibridBodyTemplate` (seen in the enemy ragdoll). Number of models that contain them: 2,608 holders, 1,155 mechanism templates, 1,150 hulls, 925 dynamic bodies. **The role is confirmed to be a named blueprint** (T01-057: `CMechanismTemplate::GetPartByName`, read in full, looks a part up by identifier): when an entity is created from the model the game asks for a mechanism **by name** (`GetDefaultMechanism`, confirmed independently this session in `projectile-classes.md`'s `CBaseProjectileEntity::OnCreate`, which refuses to create the entity with the console message "Projectile's model ... doesn't have mechanism ..." when the named template is missing) and instantiates a live `CMechanism` from the matching template's body/hull/joint templates — the `.mdl`-side templates are the design-time blueprint, the entity-side `CMechanism` (`wld-geometry.md`) is the run-time instance.
- `CDestructionHolder.1` is a list of `CDestructionConfig` (740 models), with debris/effect descriptions (`CDFXDebris`, `CDFXParticles`, `CDFXSound`, `CDFXDistribution`) and `EPlatform` masks. Specified in full in `destruction.md` (T01-046); not revisited here.
- `CSamCustomModelData` has four members: a `CParticleScheme`, a `CSoundScheme`, a `CAnimationScheme` (embedded) and an external reference (in the `Kleer` sample). Used by 438 models. **Not resolved by T01-057**: no reader of this class was found in the reading done for this task.
- `CMeshModifierList` (present in 51 models, null in 3,594) and `CAnimQueueImp` (six members, empty in the sample): **not resolved by T01-057**; no distinct reader was found for either beyond their own serialization.

## Bone palettes in models

Skinned surfaces use the per-surface palette of `bmf.md`. In models with an **embedded skeleton** the palette resolves as follows (1,318 surfaces with a palette):

| Palette entries | Surfaces |
|---|---|
| all are bone ids of skeleton LOD 0 | 1,044 |
| all are bone ids of some skeleton LOD | 24 |
| contain ids that are **no bone of the skeleton** | 250 |

A further 424 palette surfaces belong to models without an embedded skeleton (external or no skeleton). 36 skinned surfaces have an **empty** palette. So palette entries are 1-based bone ids in most cases, but 250 surfaces use ids beyond the skeleton (for example palette `{1, 6}` with a one-bone skeleton in `refrigerator.mdl`); what they address (destruction or mechanism parts?) is **open** (T01-057). The model's own skeleton thus explains 1,068 of 1,318 palettes. The constant shifts of 1 to 3 found in `skl.md` for standalone `.bmf`/`.skl` pairs (mostly weapons) are not explained by this; they remain open.

## External references (`RFIL` entries of all models)

`.tex` 11,155 · `.mat` 4,009 · `.module` 3,514 (shader modules, one per model with a material) · `.mdl` 1,505 · `.wav` 1,497 · `.pfx` 1,256 · `.ans` 681 · `.mtr` 145 · `.bmf` 127 · `.scs` 116 · `.mas` 79 · `.scp` 78 · `.skl` 60 · `.mch` 40 · `.sca` 40 · `.rsc` 10. Entries are counted per model, not per distinct file. Models therefore pull in textures, physical materials (`.mat`), shader modules, other models, sounds and particle effects through `RFIL`.

## The data after `EDOB`

29 models carry extra records (`CEditChannel`, `CWeightVertexMap`, `CEdtDestructionEffect`, `CDFXDistribution`, `CDFXDebris`, `CPlacementData`, `CEditModelConfig`, `CCustomDataHolder`), specified in `ctsemeta-objects.md`. The configuration members `.13` and `CResource.4` point at them. A runtime reader can ignore them: the main-section members that refer to an extra record are exactly nine (`ctsemeta-leftovers.md`), in `.mdl` `CResource.4`, `CModelConfiguration.13`, `CMesh.3`, `CAnimHost.2`, `CDestructionHolder.2`, `CSkeleton.2`, `CMechanismTemplateHolder.2` and `CAnimSet.2`.

## Invariants for a reader

1. Require `CResourceFile` (object 0) and exactly one `CModelConfiguration` with `CResource.1 = 1`; ignore helper configurations (`CResource.1 = −1`).
2. For every reference: null, external (in `EXOB`; the path is in `RFIL`) or an object in the main section; an unknown id is an error.
3. Apply the mesh and skeleton invariants of `bmf.md` / `skl.md` to embedded objects.
4. Bone lookups: ids from the palette or from a child's `.2` may not exist in the skeleton: treat them as "no bone" rather than indexing out of range.
5. Cap every count by the bytes remaining; models reach 3 MB.

## Test vector

`Content/SeriousSam2/Models/Enemies/Kleer/Kleer.mdl` (30,075 bytes): `CModelConfiguration` with box (−1, −0.069, −2) to (1, 3.470, 1.721); `.6` = 0.0, `.7` = `.12` = 150.0; one mesh instance whose mesh is the external `Models/Enemies/Kleer/Sources/Meshes/Kleer.bmf`, with a `CMeshModifierList`; an embedded `CSkeleton` (33 bones in LOD 0); an animation host with one instance referring to the external `Models/Enemies/Kleer/Animations/Kleer.ans`; three children (ids 34, 35, 36 with parent bones 24, 27, 3; the first two with the placement translation (0, 0, −0.6), the third with rotation (−0.7071, 0, 0, 0.7071) and translation (0, −1.478, −1.162)), all with empty model slots; a mechanism holder with 3 templates; a destruction holder with 1 configuration; custom data with four references. `RFIL` lists 10 external files (the `.bmf`, `.ans`, a `.mas` scheme, `Bones.mat`, `.wav`, `.scs`, two `.pfx`, and two debris `.mdl`).

## Open questions

- ~~Physics templates~~: **answered** (T01-057, see "Physics, destruction, custom data" above: a named blueprint instantiated by `GetDefaultMechanism`-style lookup). Destruction configuration is `destruction.md`'s own task, not revisited. **Still open**: custom data (`CSamCustomModelData`'s schemes), `CMeshModifierList`, `CAnimQueueImp` — no reader found for any of them in this task's reading.
- ~~Children member `.1`~~ and ~~`.5`~~: **answered** (T01-057: `.1` is an `IDENT` used as a lookup name, `.5` a hide flag). What palette ids beyond the skeleton address, and the three distances beyond "one of them scales the bounding box" (which one, and the other two's roles): still open.
- Which LOD and which mesh instance the engine picks per distance (needs the render spec).
- How worlds place the level models (`Models/Levels/…`) is part of the world spec (T01-012).
