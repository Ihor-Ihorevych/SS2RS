# Animation sets and sampling

`ss2-format-ans` (T02-016) reads `.ans` animation sets (spec: `docs/re/ans.md`) and evaluates them. It does not depend on the skeleton loader: tracks are keyed by the **identifier name** stored in the file (`Kleerofski`, `Hammer_Top`, ...; also kept as the raw identifier-table index, `target_id`, which is the "bone id" of the spec), and the caller binds names to bones.

```
let set = AnimSet::from_bytes(&bytes)?;          // or the AnsLoader through the AssetManager
let anim = set.animation("Boss_Kleerofski_Alive").unwrap();
let pose = anim.sample(t, LoopMode::Loop);       // pose.translations[i] belongs to anim.translations[i]
for fired in anim.events_between(prev_t, t, LoopMode::Loop) { /* fired.event.class, fired.time */ }
let step = anim.translation_delta(root_track, prev_t, t, LoopMode::Loop);
```

## What is read

- `AnimSet` (`CAnimSet`): the set name (member `.3`, an identifier such as `Animations`, `Biped_Movement`, `Wizards3`) and its animations.
- `Animation`: name (member `.1` is an identifier, e.g. `Boss_Kleerofski_Alive`, not an integer), first/last frame, seconds per frame and fps, three track lists and the events.
- Tracks are sparse key lists: `TranslationTrack` (with the `len` retargeting hint), `RotationTrack` (quaternions `x, y, z, w`, normalised on load) and `ScalarTrack` (the float envelopes).
- `AnimEvent`: class name, frame, flags, `only_in_first_loop`, and the class-specific members as generic `EventParam`s (identifiers resolved to text, external files to paths). Nothing about what an event does is guessed.
- Reader invariants of the spec: key count equals the list length, frames strictly ascend and lie in the animation's range, no empty tracks, quaternions normalised; errors name the animation and track. The sets embedded in `.mdl` files can be read with `AnimSet::from_object` once the model loader (T02-017) has decoded the container.

## Sampling rules (choices, not facts of the format)

The spec leaves interpolation, looping and root motion open (T01-058). The crate makes these choices and documents them in the item docs; changing them is a one-place edit:

- **Interpolation:** translation and scalars linear, rotation spherical along the shortest arc (normalised lerp when nearly equal). Before the first and after the last key the value is held; a NaN frame holds the first key.
- **Time:** `frame = first_frame + t * fps`. `LoopMode::Once` stops at `last_frame`; `LoopMode::Loop` repeats every `last - first` frames (at least 1), so the last frame is the first frame of the next pass. Evidence for this choice: in the retail `.ans` files 479 of the 851 multi-frame animations have identical poses on their first and last frame. Bad times (negative, NaN, infinite) count as 0.
- **Events** fire when passed: a query covers the half-open range `[from, to)`, so consecutive ticks never repeat or miss an event. In a loop each pass fires its events again, except `only_in_first_loop` ones; events outside `[first, last)` never fire in a loop (events at the last frame fire only in a `Once` animation). At most 1,024 passes are looked at per query.
- **Root motion:** the spec names no root bone, so `translation_delta(track, from, to, mode)` gives the displacement of any chosen translation track. Crossing a loop boundary skips the jump from the last key back to the first (the per-pass displacement is `value(last) - value(first)`).

## Open (follow-up T01-058)

Real interpolation and loop behaviour, which track is the root-motion track, what the float envelopes animate, what each event class does, and the meaning of set names are still undetermined. The retail numbers reproduced by the data-gated test: 88 files, 852 animations (709 at 30 fps), 14,810 translation and 14,810 rotation tracks, 5,074 float tracks, 689 events in 238 animations (22 classes).
