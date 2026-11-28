# `.ans` animation sets

- **Roadmap task:** T01-010
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the object layout, the keyframe rules and the relation to the skeleton (checked on every envelope), and, as of T01-058 (`Engine.dll`'s `ani*` animation-evaluation API and `CAnimation`'s own constructor, read in full), for `CAnimation.1`'s meaning, the interpolation method of each envelope kind, and the mechanism behind looping/reverse/clamping; **medium** for the meaning of the stored transform values (parent-relative local transforms) and for the event structure, and for what the float envelopes actually drive (the evaluation API is confirmed generic; its caller was not traced); **low** for `CAnimSet.3`, the exact per-flag-bit names of the playback mode, and the `.mas` naming scheme that resolves an IDENT to a human-readable name. **High**, as of T01-430 (data count, `tools/re/ans_bone_name_verify.py`), for the corrected binding rule in "Target ids" below: a translation/rotation envelope's `target_id` is **not** usable as a `.skl` bone id across files — it must be resolved through the two files' own identifier tables by **name**. The original "relation to the skeleton (checked on every envelope)" claim above held only for the **342 embedded animation sets whose model also embeds the skeleton**; see "Target ids". Scope: the **88** distinct format-8 `.ans` files and the **354** animation sets embedded in `.mdl` files (1,647 animations in all, 852 of them in `.ans` files); T01-430's correction additionally covers **545** models with a non-null animation host (320,272 translation/rotation envelopes) and all **31** standalone `.skl` files (42 LODs).

Prerequisites: `ctsemeta-container.md`, `ctsemeta-types.md`, `ctsemeta-objects.md`; `skl.md` (bones), `mdl.md` (how a model refers to a set).

## Summary

An animation set (`CAnimSet`) is a list of animations. An animation has a frame range, a frame rate, a list of **envelopes** and optional **events**. There is one translation envelope and one rotation envelope per animated bone, and (in some sets) extra float envelopes. An envelope is a **sparse list of keyframes**: frame number plus value. The data is not compressed. Sets appear either as standalone `.ans` files (a model's animation host points to them) or embedded in the model.

## Evidence

`tools/re/ans_verify.py` (read-only, `SS2_DATA_DIR`) over the retail install with all patches; **T01-430** additionally ran `tools/re/ans_bone_name_verify.py` (read-only, `SS2_DATA_DIR`) over every model with a non-null animation host, every standalone `.skl`, and every `.ans` file reached from one, to settle how an envelope's `target_id` actually binds to a skeleton bone (see "Target ids" below); **T01-058** additionally decompiled and read, in `Engine.dll`: `CAnimation`'s constructors (for `.1`'s true type and default), `aniFindAnimationInAnimHost`, `aniUpdateLookupTable`, `aniCalcAnimFrame`, `aniEvaluate` and its four per-type evaluators (`aniEvaluatePosition`, `aniEvaluateRotation`, `aniEvaluateColor`, `aniEvaluateStrength`) and their inlined/called interpolation helpers. For **all 64,927 envelopes** of the 1,647 animations: the key count in the envelope equals the list length; frame numbers strictly ascend; the first and last key sit exactly on the animation's first and last frame; the envelope's own first/last frame equal the animation's; member 5 equals `1 / (last − first + 1)`. All 29,555 rotation envelopes have unit-length quaternions (error below 1e-4), and every bone that has one of the two kinds has both (translation and rotation: 29,555 bones, none with only one). For the 342 embedded sets whose model has an embedded skeleton: 29,344 of 29,452 translation/rotation envelopes address a bone id of that skeleton; the translation member `len` times the bone's length is 1 in 14,468 of 14,665 envelopes.

## Objects and members

```
CAnimSet
  CResource.{1,5,3,2}     as for the other resources
  .1   list of -> CAnimation
  .3   int                 4..85 in the data; unknown (see below)
CAnimation
  .1   int                 unknown (ascending in 87 of 88 sets, unique in 85)
  .2   int                 first frame
  .3   int                 last frame
  .4   float               seconds per frame (= 1 / .5)
  .5   float               frames per second (30.0 in 709 of the 852 `.ans` animations; also 50, 10, 100, 5, 14.29, ...)
  .6   list of -> envelope  see below
  .8   -> CAnimEventHolder  or null (465 of 1,647 animations have one)
```

**`CAnimation.1` is confirmed to be an `IDENT`** (T01-058: `CAnimation`'s default constructor sets this member to the engine's own "invalid identifier" sentinel — the same one used elsewhere, for example `ColorMap` in `mtr.md`), and it is the animation's **lookup key**: `SeriousEngine::aniFindAnimationInAnimHost(host, id)` (read in full) scans every `CAnimSet` of a host and returns the first `CAnimation` whose `.1` equals the given `id`; `aniUpdateLookupTable` builds a sorted per-set index keyed the same way. So an animation is selected by **IDENT**, not by its list position; how that IDENT is derived from (or resolved back to) the human-readable names used by `.mas` schemes was not found in this reading. `CAnimation.1` equals `CAnimation.2` in only 8 animations, confirming it is a name-like id, not a copy of the first frame.

### Envelopes

All envelope classes share the base members (`CAnimEnvelope`):

| Member | Meaning |
|---|---|
| `.1` | **target id**: a bone id for translation and rotation envelopes (see below) |
| `.2` | first frame (equals `CAnimation.2` in all 64,927) |
| `.3` | last frame (equals `CAnimation.3`) |
| `.4` | number of keys (equals the list length) |
| `.5` | `1 / (last − first + 1)` (all 64,927): the reciprocal of the frame count |

The three classes differ in their key list:

| Class | Extra members | Key class (`CAnimFrame.1` = frame number, an int, then the value) |
|---|---|---|
| `CVec3LenAnimEnvelope` | `.1` float, `.2` keys | `CVec3AnimFrame`: `Vector3f` (translation) |
| `CQuatAnimEnvelope` | `.1` keys | `CQuatAnimFrame`: `Quaternion4f`, stored **x, y, z, w** (w last), unit length |
| `CFloatAnimEnvelope` | `.1` keys | float keys (same frame rules): 5,074 in the `.ans` files |

A **fourth envelope class exists in the engine but was not seen in this format's data**: `CColorAnimEnvelope` (T01-058, `Engine.dll`), evaluated through its own `aniEvaluateColor` the same way the other three are; `ans_verify.py`'s class tally over all 64,927 envelopes never found one, so it is either unused by this game's shipped `.ans`/`.mdl` animations or stored some other way not checked here.

In `CAnimation.6` the envelopes are grouped by kind: all translation envelopes first, then all rotation envelopes, in the same bone order (sample below). Keys are **sparse**: a static bone has two keys (first and last frame), a moving bone has a key at every changed frame.

**Interpolation (T01-058: `Engine.dll`'s `aniEvaluate` and the per-type evaluator it dispatches to, `aniEvaluatePosition`/`aniEvaluateRotation`/`aniEvaluateStrength`/`aniEvaluateColor`, all read in full).** The answer differs by envelope kind, confirmed from the actual arithmetic:

- **Translation and float ("strength") envelopes interpolate linearly** between the two keys that bracket the current frame: `value = lerp(key1, key2, t)` with `t = (frame - key1.frame) / (key2.frame - key1.frame)`.
- **Rotation envelopes use proper spherical interpolation (slerp)**, not plain lerp-and-renormalize: the two keys' dot product is computed and the second key negated if it is negative (shortest-path correction, the standard slerp fix for quaternion double-cover); when the two keys are nearly identical (dot above a fixed threshold) it falls back to a **linear** blend of the four components instead of slerp, to avoid dividing by a near-zero `sin` — again the standard numerically-safe slerp. **Blending between several animations played at once also uses this same slerp for rotation** (a second, identical acos/sin pass blends the freshly-interpolated key value into the accumulator from earlier-evaluated layers) and plain lerp for translation/strength/colour — so layering respects quaternion geometry throughout, not just within one animation's own keys.
- All four evaluators (`aniEvaluatePosition`, `aniEvaluateRotation`, `aniEvaluateColor`, `aniEvaluateStrength`) are thin wrappers that queue one target id into a shared per-frame query and call the common `aniEvaluate`, which walks every currently-played animation list and layer-blends the result; this is also how several simultaneously playing animations (not just several keys) are combined.

**Translation member `len`.** `CVec3LenAnimEnvelope.1` is the **reciprocal of the skeleton's bone length** (`skl.md`, member 6) of the target bone: `len × boneLength = 1` for 14,468 of 14,665 envelopes checked against the *same file's* bone ids (98.7 %; see "Target ids" below for why that scoping matters); the other 197 are in models whose skeleton differs from the one the animation was exported with. It is not the path length, nor the largest key length (tested). It most likely supports retargeting translations to a skeleton with different bone lengths.

**Meaning of the values.** Keys are in the same space as the skeleton's local transforms (`skl.md`, member 4: parent-relative translation and quaternion): the first translation key equals the bone's rest translation in 14,094 of 14,665 cases (96 %), as expected for bones that hardly move; the first rotation key equals the rest rotation in 5,494 (37 %), the rest are bones that really rotate. Values are therefore absolute local transforms, **not deltas** (deltas would be near zero for static bones).

**Target ids (corrected by T01-430, follow-up to T04-006).** A translation or rotation envelope's `target_id` (`CAnimEnvelope.1`) is an `IDENT`: **an index into the envelope's *own file's* `IDNT` string table**, naming the target bone **by name**, not by a numeric id that is comparable across files. `ctsemeta-container.md` leaves open "whether `IDNT` ids are referenced from object data and how" — this is one answer, and it generalises a convention already visible in `skl.md`: every `.skl` file's own `IDNT` table stores **the bone's name at the index equal to the bone's id** (`IDNT[boneId]` = that bone's name, for `boneId` in `1..boneCount`; index 0 is the empty string). Checked on all **31** standalone `.skl` files (42 LODs, every bone): **high** confidence, data count.

So the actual binding rule a loader must apply is: 1) read the envelope's own file's `IDNT[target_id]` to get the bone's name string; 2) find that string in the skeleton's own `IDNT` table (the index there, if any, is the bone id); 3) skip the envelope (not index out of range) if the name is not found. For an animation set **embedded in the same `.mdl` file as its skeleton**, steps 1 and 2 read the *same* table, and the model's bones happen to occupy that table's low, stable indices (registered before the animation and other names) — so `target_id` *also* equals the bone's numeric id there, which is why `ans.md`'s original "99.6 % of the 29,452 checked" and the `len` check's "98.7 %" held: both were measured only over the 342 embedded sets whose model embeds its skeleton (`ans_verify.py` always skipped the check, passing `ids=None`, for standalone `.ans` files). Measured in full by `tools/re/ans_bone_name_verify.py` (T01-430) over every model with a non-null animation host (545 models, 320,272 translation/rotation envelopes):

| Binding (model's skeleton × animation set) | Envelopes | `target_id` is directly a bone id (the old/buggy reading) | `IDNT[target_id]` names a bone of the skeleton's own `IDNT` table (the real rule) |
|---|---:|---:|---:|
| embedded skeleton + embedded animation set (same `IDNT` table) | 29,452 | 29,330 (99.6 %) | 29,330 (99.6 %) |
| embedded skeleton + external `.ans` | 255,792 | 36,220 (14.2 %) | 248,810 (97.3 %) |
| external `.skl` + embedded animation set | 22 | 0 (0 %) | 22 (100 %) |
| external `.skl` + external `.ans` | 35,006 | 9,720 (27.8 %) | 33,758 (96.4 %) |
| **all** | **320,272** | **75,270 (23.5 %)** | **311,920 (97.4 %)** |

The real rule resolves 97.4 % of all checked envelopes, against 23.5 % for the id-as-id reading, and it is the *only* reading that is at all workable once the skeleton is external (0 %–28 % by id, 96 %–100 % by name). The remaining 8,352 envelopes (2.6 %) whose name is not found in the skeleton's own table are the same residual the direct-id check always had: animations shared with (or reused by) a model whose skeleton differs from the one the animation was exported against (for example `bullsoldierred.mdl`'s skeleton has no bone named `L_Hand`, a name some of its shared animations' envelopes still carry) — not a flaw in the name rule, a genuine retargeting mismatch the loader should skip over (reader invariant 3 below).

The float envelopes address ids that are **not** bones: all 743 checked ids lie above the largest bone id of the skeleton (for example 104..107 with 91 bones in `boss_princchan.mdl`) and are not attachment child ids (`mdl.md`) either. **What consumes them is confirmed to be generic** (T01-058): `aniEvaluateStrength(default, id, &out)` evaluates any float track by `IDENT` the same way `aniEvaluatePosition`/`aniEvaluateRotation` evaluate a bone's transform — a first-class scalar animation channel, not a transform — but which caller asks for which id (a blend-shape weight, a shader parameter, an attachment's visibility, …) was not traced past the animation system itself, so **what they animate in this game stays unknown**; the same may hold for the palette ids beyond the skeleton found in `mdl.md`.

### Time

`CAnimation.4 × CAnimation.5 = 1` for all 1,647 animations. Key frames are integers on the animation's frame grid; events use float frames.

**Duration and looping (T01-058: `SeriousEngine::aniCalcAnimFrame`, `Engine.dll`, read in full).** The current playback frame is computed from elapsed real time times the animation's own `fps` (`.5`) times a per-instance speed multiplier, added to a start offset, then one of three modes applies, selected by flag bits of the playing instance (the public enum names for these bits were not found, only their effect):

- **Loop** (one bit set): the frame wraps with a modulo of the frame count, `last − first + 1` — confirming the envelope member `.5`'s `1 / (last − first + 1)` is exactly this same frame count, used the same way by every envelope's own interpolation fraction.
- **Reverse** (another bit): the elapsed time is subtracted instead of added, with its own end-of-range wrap check against `first`.
- **Clamp** (neither bit, the default): playback stops at `last` (or `first` if reversed).
- Independently of these, when a further bit is set and a per-instance fade-period is non-zero, finishing the animation computes a **fade-out window** (time remaining scaled by that period, clamped to zero) rather than cutting off sharply — the data-side period for this is not identified here.

This resolves "how looping wraps" to an exact mechanism, though the engine's own names for the three modes and the fade bit stay open (would need the public header, not present in this reading).

## Events

`CAnimation.8` refers to a `CAnimEventHolder` with one member, `CAnimEventHolder.aeh_caeEvents`, a list of references to **event objects** (all 1,313 are references). Every event class starts with the members of `CAnimEvent`:

| Member | Meaning |
|---|---|
| `CAnimEvent.ae_fFrame` | float frame at which the event fires; inside the animation's frame range for 1,309 of 1,313 events |
| `CAnimEvent.ae_ulFlags` | `AnimEventFlags` (4-byte enum; 0x0, 0x1, 0x4 seen) |
| `CAnimEvent.ae_bOnlyInFirstLoop` | 0 or 1 |

followed by class-specific members, for example `CSamSoundAnimEvent.ssae_idSound` (an id), `SPuppetSoundAnimEvent.psae_psstSoundSourceType`, `SMeleeDamageAnimEvent.mdae_idBlast`, `CAttachToolAnimEvent.atae_fLerpPeriod` (0.4), `CStoreToolAnimEvent.stae_fStoringPeriod` (0.2). Event classes in the data (count over all sets): `SPuppetSoundAnimEvent` 575, `CSamEffectAnimEvent` 171, `SPuppetFireAnimEvent` 129, `SAnimEventStep` 99, `CSamSoundAnimEvent` 91, `CSamShakeEffectAnimEvent` 52, `CHidePlayerWeaponAttachmentAnimEvent` 44, `SMeleeDamageAnimEvent` 36, `CShowPlayerWeaponAttachmentAnimEvent` 26, `SRagdollAnimEvent` 20, `CSamMacroEvent` 12, `CDrawToolAnimEvent`, `SPuppetReleaseSpawneeAnimEvent`, `CRenderMuzzleFlashAnimEvent` 9 each, `CStoreToolAnimEvent` 7, `CThrowDeathToolsAnimEvent` 6, and a few more (`CDetachToolAnimEvent`, `CStandAloneSimpleEffectAnimEvent`, `CHideAttachmentAnimEvent`, `CShowAttachmentAnimEvent`, `CAttachToolAnimEvent`, `CPlayerWeaponEffectForOneHandAnimEvent`, `CPuppetDisappearAnimEvent`). The ids (sound, blast) refer to schemes of the model's `CSamCustomModelData` (`mdl.md`; not verified). What each event does is gameplay (T01-026…T01-029) and not specified here.

## Reader invariants

1. Per envelope: key count equals the list length; frames strictly ascend and lie inside the animation's range; reject otherwise.
2. Normalise quaternions on load (the data is already unit length to 1e-4).
3. **Look bones up by name, not by `target_id` as a raw bone id** (T01-430, "Target ids" above): resolve `target_id` through the envelope's own file's `IDNT` table to a name string, then find that name in the skeleton's own `IDNT` table (its index there is the bone id). Names not found in the skeleton (a retargeting mismatch) and all float envelope ids must be skipped, not indexed.
4. Never allocate from a count before checking it against the bytes remaining (a set reaches 2.5 MB).
5. Treat events as optional: an unknown event class should fail the set or be skipped explicitly, never guessed.
6. Interpolate translation and float envelopes linearly between the bracketing keys; interpolate rotation envelopes with slerp, negating the second key when the quaternion dot product is negative, falling back to a linear blend of the four components when the keys are nearly identical (T01-058, "Interpolation" above).
7. Select an animation by `CAnimation.1` (an `IDENT`), not by its position in the set's list (T01-058).

## Test vector

`Content/SeriousSam2/Models/Items/Powerups/Animations/boss_animations.ans` (3,301 bytes): one `CAnimSet` (`.3` = 1) with one animation: `.1` = 2, first frame 2, last frame 25, 0.0333333 s per frame, 30.0 fps, no events, eight envelopes in the order translation bones 3, 4, 5, 6, then rotation bones 3, 4, 5, 6, every envelope with member `.5` = 0.0416667 (= 1/24):

| Bone | Translation envelope | Rotation envelope |
|---|---|---|
| 3 | 20 keys (frames 2, 7, 8, 9, …); first (0, 0.42, 0.84); `len` 1.0648 | 2 keys, identity |
| 4 | 2 keys (2, 25), both (2.1e−7, −0.85, −2.4); `len` 0.3928 | 2 keys, identity |
| 5 | 2 keys, both (0, 1.65, −1.75); `len` 0.4158 | 23 keys (frames 2, 3, 4, 6, …); first (3.2e−7, 0.9668, −0.2554, 1.2e−6) |
| 6 | 2 keys, both (0, 0, 0); `len` 1.0 | 2 keys, identity |

## Test vectors for the binding rule (T01-430)

Three models whose skeleton is an external `.skl` (so the animation's own `IDNT` table differs from the skeleton's), each with a standalone or shared `.ans`. For every one, resolving `target_id` to a name and then to the skeleton's bone of that name fits every translation/rotation envelope of the animation named; treating `target_id` directly as the bone id fits only a minority:

| Model | Skeleton | Animation (`.ans`) | Envelopes | Direct-id matches | Name-resolved matches |
|---|---|---|---|---:|---:|
| `Content/SeriousSam2/Models/Enemies/ScorpSoldier/ScorpSoldier.mdl` | `.../ScorpSoldier/Sources/Skeleton.skl` (40 bones) | `ScorpSoldier_Idle` in `.../ScorpSoldier/Animations/Scorp.ans` | 80 (40 translation + 40 rotation) | 56 | 80 |
| `Content/SeriousSam2/Models/Enemies/Tank/Tank.mdl` | `.../Tank/Sources/Tank.skl` (15 bones) | `Walk` in `.../Tank/Animations/Animations.ans` | 30 | 4 | 30 |
| `Content/SeriousSam2/Models/Npcs/Brimland/Female.mdl` | `.../Brimland/Sources/Skeletons/Female.skl` (24 bones) | `BackPedal` in the shared `Content/SeriousSam2/Models/Animations/Biped_Rides.ans` | 48 | 0 | 48 |

For `ScorpSoldier_Idle`'s first translation envelope: `target_id` 13; `Scorp.ans`'s own `IDNT[13]` = `"RF_Foot"`; `Skeleton.skl`'s `IDNT` table has `"RF_Foot"` at index 10 — bone 10, not bone 13 (bone 13 of the skeleton is `RB_Foot`, a different leg). The `len` value of that envelope (`CVec3LenAnimEnvelope.1`) times bone 10's length (`SkeletonBone.6`) is 1 to within 1e-3, confirming the resolved id, not the raw `target_id`.

`tools/re/ans_bone_name_verify.py` reproduces this table (and the install-wide counts in "Target ids" above) from the install.

## Open questions

- Meaning of `CAnimSet.3` (values 4 to 85; not the bone count, the largest bone id or the animation count): **not resolved by T01-058** — no reader of this member was found in `Engine.dll`; it may be editor-only or consumed by a different module entirely.
- ~~Interpolation between sparse keys, duration and looping~~ **Answered by T01-058** (sections "Interpolation" and "Duration and looping" above: lerp for translation/strength, slerp with shortest-path and a linear-identical fallback for rotation; loop/reverse/clamp playback modes with an optional fade-out window). How animations are **named** and selected by a human-readable name (the `.mas` schemes, T01-019/T01-057) stays open; selection by `IDENT` is answered (`CAnimation.1`, above).
- What the float envelopes (ids above the bones) animate: **the delivery mechanism is answered** (`aniEvaluateStrength`, a generic scalar channel), but the actual consumer/purpose in this game is not; the 108 translation/rotation envelopes whose id is not a bone of the model's skeleton stay unanalysed.
- New from T01-058: a fourth envelope class, `CColorAnimEnvelope`, exists in the engine (evaluated via `aniEvaluateColor`) but was not observed in any of the 64,927 envelopes checked in this format's data.
- Exact behaviour of each event class → gameplay specs.
- ~~How a translation/rotation envelope's `target_id` binds to a `.skl` bone across files~~ **Answered by T01-430** (follow-up to T04-006; see "Target ids" above): by name, through each file's own `IDNT` table, not by treating `target_id` as a bone id. The residual 2.6 % (8,352 of 320,272 envelopes) whose resolved name is not a bone of the model's skeleton are animations shared with a model whose skeleton differs from the one they were exported against — not analysed further here, same as the pre-existing 197/14,665 `len` exceptions.
