# `.ans` animation sets

- **Roadmap task:** T01-010
- **Author:** @Ihor-Ihorevych/agent · **Reviewer:** _none yet (T01-049)_
- **Confidence:** **high** for the object layout, the keyframe rules and the relation to the skeleton (checked on every envelope); **medium** for the meaning of the values (parent-relative local transforms) and for the event structure; **low** for `CAnimSet.3`, `CAnimation.1`, the float envelopes, interpolation and loop behaviour. Scope: the **88** distinct format-8 `.ans` files and the **354** animation sets embedded in `.mdl` files (1,647 animations in all, 852 of them in `.ans` files).

Prerequisites: `ctsemeta-container.md`, `ctsemeta-types.md`, `ctsemeta-objects.md`; `skl.md` (bones), `mdl.md` (how a model refers to a set).

## Summary

An animation set (`CAnimSet`) is a list of animations. An animation has a frame range, a frame rate, a list of **envelopes** and optional **events**. There is one translation envelope and one rotation envelope per animated bone, and (in some sets) extra float envelopes. An envelope is a **sparse list of keyframes**: frame number plus value. The data is not compressed. Sets appear either as standalone `.ans` files (a model's animation host points to them) or embedded in the model.

## Evidence

`tools/re/ans_verify.py` (read-only, `SS2_DATA_DIR`) over the retail install with all patches. For **all 64,927 envelopes** of the 1,647 animations: the key count in the envelope equals the list length; frame numbers strictly ascend; the first and last key sit exactly on the animation's first and last frame; the envelope's own first/last frame equal the animation's; member 5 equals `1 / (last − first + 1)`. All 29,555 rotation envelopes have unit-length quaternions (error below 1e-4), and every bone that has one of the two kinds has both (translation and rotation: 29,555 bones, none with only one). For the 342 embedded sets whose model has an embedded skeleton: 29,344 of 29,452 translation/rotation envelopes address a bone id of that skeleton; the translation member `len` times the bone's length is 1 in 14,468 of 14,665 envelopes.

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

`CAnimation.1` equals `CAnimation.2` in only 8 animations and is not the index in the list: its meaning is open.

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

In `CAnimation.6` the envelopes are grouped by kind: all translation envelopes first, then all rotation envelopes, in the same bone order (sample below). Keys are **sparse**: a static bone has two keys (first and last frame), a moving bone has a key at every changed frame. How to interpolate between keys (linear for translation, spherical for rotation is the likely answer) **cannot be told from the data** and is not specified here.

**Translation member `len`.** `CVec3LenAnimEnvelope.1` is the **reciprocal of the skeleton's bone length** (`skl.md`, member 6) of the target bone: `len × boneLength = 1` for 14,468 of 14,665 envelopes (98.7 %); the other 197 are in models whose skeleton differs from the one the animation was exported with. It is not the path length, nor the largest key length (tested). It most likely supports retargeting translations to a skeleton with different bone lengths.

**Meaning of the values.** Keys are in the same space as the skeleton's local transforms (`skl.md`, member 4: parent-relative translation and quaternion): the first translation key equals the bone's rest translation in 14,094 of 14,665 cases (96 %), as expected for bones that hardly move; the first rotation key equals the rest rotation in 5,494 (37 %), the rest are bones that really rotate. Values are therefore absolute local transforms, **not deltas** (deltas would be near zero for static bones).

**Target ids.** Translation and rotation envelopes address bone ids of the model's skeleton (99.6 % of the 29,452 checked; the exceptions are not analysed). The float envelopes address ids that are **not** bones: all 743 checked ids lie above the largest bone id of the skeleton (for example 104..107 with 91 bones in `boss_princchan.mdl`) and are not attachment child ids (`mdl.md`) either. What they animate is **unknown**; the same may hold for the palette ids beyond the skeleton found in `mdl.md`.

### Time

`CAnimation.4 × CAnimation.5 = 1` for all 1,647 animations. Key frames are integers on the animation's frame grid; events use float frames. Whether the animation's duration is `(last − first) / fps` or `(last − first + 1) / fps` (the envelope member `.5` suggests a frame count of `last − first + 1`) and how looping wraps are **not determined**.

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
3. Look bones up by id; ids not present in the skeleton (and all float envelope ids) must be skipped, not indexed.
4. Never allocate from a count before checking it against the bytes remaining (a set reaches 2.5 MB).
5. Treat events as optional: an unknown event class should fail the set or be skipped explicitly, never guessed.

## Test vector

`Content/SeriousSam2/Models/Items/Powerups/Animations/boss_animations.ans` (3,301 bytes): one `CAnimSet` (`.3` = 1) with one animation: `.1` = 2, first frame 2, last frame 25, 0.0333333 s per frame, 30.0 fps, no events, eight envelopes in the order translation bones 3, 4, 5, 6, then rotation bones 3, 4, 5, 6, every envelope with member `.5` = 0.0416667 (= 1/24):

| Bone | Translation envelope | Rotation envelope |
|---|---|---|
| 3 | 20 keys (frames 2, 7, 8, 9, …); first (0, 0.42, 0.84); `len` 1.0648 | 2 keys, identity |
| 4 | 2 keys (2, 25), both (2.1e−7, −0.85, −2.4); `len` 0.3928 | 2 keys, identity |
| 5 | 2 keys, both (0, 1.65, −1.75); `len` 0.4158 | 23 keys (frames 2, 3, 4, 6, …); first (3.2e−7, 0.9668, −0.2554, 1.2e−6) |
| 6 | 2 keys, both (0, 0, 0); `len` 1.0 | 2 keys, identity |

## Open questions

- Meaning of `CAnimSet.3` (values 4 to 85; not the bone count, the largest bone id or the animation count) and `CAnimation.1` → **T01-058**.
- Interpolation between sparse keys, duration and looping, how animations are named and selected (the names live in animation schemes, `.mas`, T01-019/T01-057) → T01-058.
- What the float envelopes (ids above the bones) animate; the 108 translation/rotation envelopes whose id is not a bone of the model's skeleton → T01-058.
- Exact behaviour of each event class → gameplay specs.
