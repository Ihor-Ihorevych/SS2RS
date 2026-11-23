//! **Instancing and batching** for hordes: turn a frame's list of [`Renderable`]s into as few draw calls as possible (roadmap T04-023, `docs/BATCHING.md`).
//!
//! Device-free. [`plan_batches`] groups renderables that can be drawn by **one instanced draw call** (same model, same material override, same flags, and
//! either not animated or animated with per-instance animation allowed), splits groups that exceed the device's per-draw limit, and orders everything so the result is
//! **identical on every run**: batches by model and material, instances by [`RenderId`]. [`pack_instances`] writes the per-instance data (matrix, tint,
//! animation state, flags) in batch order, ready to upload to one instance buffer; each [`Batch`] knows its slice of it.
//!
//! What this does **not** do is draw: the renderer draws each batch with `instance_count` instances starting at `first_instance`.

use crate::{MaterialId, ModelId, RenderFlags, RenderId, Renderable};

/// How to batch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BatchConfig {
    /// The most instances in one draw call (the device's limit for an instance buffer slice or a uniform array). 1 to 65,536.
    pub max_instances_per_batch: usize,
    /// Whether skinned renderables with different animation states may share a draw call (the renderer reads each instance's clip and time, for example from an
    /// animation texture). When `false`, an animated renderable is drawn alone.
    pub animated_instancing: bool,
}

impl Default for BatchConfig {
    /// 1,024 instances per draw, animated instancing on.
    fn default() -> Self {
        Self {
            max_instances_per_batch: 1024,
            animated_instancing: true,
        }
    }
}

impl BatchConfig {
    /// Whether the values are in range.
    pub fn validate(&self) -> Result<(), &'static str> {
        if (1..=65_536).contains(&self.max_instances_per_batch) {
            Ok(())
        } else {
            Err("max_instances_per_batch must be 1 to 65536")
        }
    }
}

/// One instanced draw call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Batch {
    /// The model every instance draws.
    pub model: ModelId,
    /// The material that replaces the model's own, if any (the same for every instance).
    pub material_override: Option<MaterialId>,
    /// The flags every instance has.
    pub flags: RenderFlags,
    /// Whether the instances are animated (they carry a clip and a time).
    pub animated: bool,
    /// Where this batch's instances start in the packed instance buffer.
    pub first_instance: usize,
    /// How many instances to draw.
    pub instance_count: usize,
}

/// What a plan achieved.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BatchStats {
    /// Renderables in.
    pub renderables: usize,
    /// Draw calls out.
    pub batches: usize,
    /// `renderables - batches`: the draw calls saved against drawing each one alone.
    pub draw_calls_saved: usize,
    /// The most instances in a single batch.
    pub largest_batch: usize,
    /// Batches of one instance.
    pub singletons: usize,
}

/// The result of [`plan_batches`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchPlan {
    /// The draw calls, in order.
    pub batches: Vec<Batch>,
    /// For each instance slot (in packed order), the index into the renderables that were planned.
    pub order: Vec<usize>,
    /// Counts.
    pub stats: BatchStats,
}

/// The grouping key: renderables with the same key may share a draw call.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    model: ModelId,
    material: Option<MaterialId>,
    flags: u32,
    animated: bool,
    /// For a renderable that cannot be instanced (animated with instancing off), its own id, so it never groups; `None` otherwise.
    solo: Option<RenderId>,
}

/// Group `renderables` into instanced draw calls.
///
/// Renderables with the same model, material override and flags share a draw call, in groups of at most `cfg.max_instances_per_batch`. Batches are ordered by
/// model, then material, then flags, then animated; the instances of a batch by [`RenderId`], and a group split by the limit keeps that order across its batches. Every renderable appears
/// **exactly once**, and the result does not depend on the order of the input.
///
/// ```
/// use ss2_core::math::Vec3;
/// use ss2_render_api::batching::{plan_batches, BatchConfig};
/// use ss2_render_api::{ModelId, RenderId, Renderable, Transform};
///
/// let grunt = ModelId::from_raw(1).unwrap();
/// let horde: Vec<Renderable> = (0..300)
///     .map(|i| Renderable::new(RenderId(i), grunt, Transform::from_translation(Vec3::new(i as f32, 0.0, 0.0))))
///     .collect();
/// let plan = plan_batches(&horde, &BatchConfig::default()).unwrap();
/// assert_eq!(plan.batches.len(), 1);
/// assert_eq!(plan.stats.draw_calls_saved, 299);
/// ```
pub fn plan_batches(
    renderables: &[Renderable],
    cfg: &BatchConfig,
) -> Result<BatchPlan, &'static str> {
    cfg.validate()?;
    let key_of = |r: &Renderable| {
        let animated = r.animation.is_some();
        Key {
            model: r.model,
            material: r.material_override,
            flags: r.flags.0,
            animated,
            solo: (animated && !cfg.animated_instancing).then_some(r.id),
        }
    };
    let mut idx: Vec<usize> = (0..renderables.len()).collect();
    idx.sort_by_key(|i| (key_of(&renderables[*i]), renderables[*i].id, *i));

    let mut batches: Vec<Batch> = Vec::new();
    let mut order = Vec::with_capacity(idx.len());
    let mut current: Option<(Key, usize)> = None; // (key of the open batch, its index)
    for i in idx {
        let r = &renderables[i];
        let key = key_of(r);
        let open = match current {
            Some((k, b)) if k == key && batches[b].instance_count < cfg.max_instances_per_batch => {
                Some(b)
            }
            _ => None,
        };
        match open {
            Some(b) => batches[b].instance_count += 1,
            None => {
                batches.push(Batch {
                    model: r.model,
                    material_override: r.material_override,
                    flags: r.flags,
                    animated: key.animated,
                    first_instance: order.len(),
                    instance_count: 1,
                });
                current = Some((key, batches.len() - 1));
            }
        }
        order.push(i);
    }
    let stats = BatchStats {
        renderables: renderables.len(),
        batches: batches.len(),
        draw_calls_saved: renderables.len() - batches.len(),
        largest_batch: batches.iter().map(|b| b.instance_count).max().unwrap_or(0),
        singletons: batches.iter().filter(|b| b.instance_count == 1).count(),
    };
    Ok(BatchPlan {
        batches,
        order,
        stats,
    })
}

/// How many `f32`s one instance takes in the packed buffer.
pub const INSTANCE_FLOATS: usize = 24;

/// The per-instance data, as the floats a shader reads.
///
/// Layout (indices into the 24 floats): `0..16` the world matrix, column-major; `16..20` the tint (linear RGBA); `20` the animation time in seconds (0 if none);
/// `21` the animation clip id and `22` the flag bits, both as the **bit pattern** of the integer stored in the float (use `floatBitsToUint` in a shader; it is not
/// a numeric conversion); `23` is zero padding.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InstanceData(pub [f32; INSTANCE_FLOATS]);

impl InstanceData {
    /// The data of one renderable. A clip id of 0 means "not animated" (real clip ids are never 0).
    pub fn from_renderable(r: &Renderable) -> Self {
        let mut f = [0.0f32; INSTANCE_FLOATS];
        f[..16].copy_from_slice(&r.transform.matrix().to_cols_array());
        f[16..20].copy_from_slice(&r.tint);
        if let Some(a) = r.animation {
            f[20] = a.time;
            f[21] = f32::from_bits(a.clip.raw());
        }
        f[22] = f32::from_bits(r.flags.0);
        Self(f)
    }
}

/// The instance data of every renderable in the plan, in packed order: instance `n` of the buffer is `order[n]` of `renderables`. The length is
/// `plan.order.len() x INSTANCE_FLOATS`.
///
/// # Panics
/// If `plan` was made from a different list (an index in `plan.order` is out of range).
pub fn pack_instances(renderables: &[Renderable], plan: &BatchPlan) -> Vec<f32> {
    let mut out = Vec::with_capacity(plan.order.len() * INSTANCE_FLOATS);
    for &i in &plan.order {
        out.extend_from_slice(&InstanceData::from_renderable(&renderables[i]).0);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AnimState, ClipId, Transform};
    use ss2_core::math::{Quat, Vec3};
    use std::collections::BTreeSet;

    fn model(n: u32) -> ModelId {
        ModelId::from_raw(n).unwrap()
    }

    fn material(n: u32) -> MaterialId {
        MaterialId::from_raw(n).unwrap()
    }

    fn r(id: u64, m: u32) -> Renderable {
        Renderable::new(
            RenderId(id),
            model(m),
            Transform::from_translation(Vec3::new(id as f32, 0.0, 0.0)),
        )
    }

    fn animated(id: u64, m: u32, clip: u32, time: f32) -> Renderable {
        Renderable {
            animation: Some(AnimState {
                clip: ClipId::from_raw(clip).unwrap(),
                time,
            }),
            ..r(id, m)
        }
    }

    fn plan(list: &[Renderable]) -> BatchPlan {
        plan_batches(list, &BatchConfig::default()).unwrap()
    }

    fn counts(p: &BatchPlan) -> Vec<usize> {
        p.batches.iter().map(|b| b.instance_count).collect()
    }

    // ---- grouping -------------------------------------------------------------------------------

    #[test]
    fn identical_things_share_one_draw_call_and_different_ones_do_not() {
        let list = [r(1, 1), r(2, 1), r(3, 2), r(4, 1), r(5, 2)];
        let p = plan(&list);
        assert_eq!(counts(&p), [3, 2]);
        assert_eq!(
            (p.batches[0].model, p.batches[1].model),
            (model(1), model(2)),
            "batches ordered by model"
        );
        assert_eq!(
            p.stats,
            BatchStats {
                renderables: 5,
                batches: 2,
                draw_calls_saved: 3,
                largest_batch: 3,
                singletons: 0
            }
        );
    }

    #[test]
    fn a_material_override_a_flag_or_animation_makes_a_different_batch() {
        let plain = r(1, 1);
        let with_material = Renderable {
            material_override: Some(material(7)),
            ..r(2, 1)
        };
        let other_material = Renderable {
            material_override: Some(material(8)),
            ..r(3, 1)
        };
        let viewmodel = Renderable {
            flags: RenderFlags::VIEWMODEL,
            ..r(4, 1)
        };
        let skinned = animated(5, 1, 3, 0.5);
        let p = plan(&[plain, with_material, other_material, viewmodel, skinned]);
        assert_eq!(p.batches.len(), 5);
        // A second of each kind joins its own batch.
        let p2 = plan(&[
            plain,
            r(10, 1),
            with_material,
            Renderable {
                id: RenderId(11),
                ..with_material
            },
            viewmodel,
            Renderable {
                id: RenderId(12),
                ..viewmodel
            },
        ]);
        assert_eq!(counts(&p2), [2, 2, 2]);
        assert!(
            p2.batches
                .iter()
                .any(|b| b.material_override == Some(material(7)) && b.instance_count == 2)
        );
        assert!(
            p2.batches
                .iter()
                .any(|b| b.flags == RenderFlags::VIEWMODEL && b.instance_count == 2)
        );
    }

    #[test]
    fn animated_things_with_different_clips_share_a_draw_call_when_animated_instancing_is_on() {
        let list = [
            animated(1, 1, 1, 0.0),
            animated(2, 1, 2, 3.0),
            animated(3, 1, 1, 9.0),
            r(4, 1),
        ];
        let p = plan(&list);
        assert_eq!(
            counts(&p),
            [1, 3],
            "the static one apart, the three animated together"
        );
        assert!(!p.batches[0].animated && p.batches[1].animated);
        // With instancing off every animated renderable is alone; the static ones still group.
        let off = plan_batches(
            &list,
            &BatchConfig {
                animated_instancing: false,
                ..BatchConfig::default()
            },
        )
        .unwrap();
        assert_eq!(counts(&off), [1, 1, 1, 1]);
        let list2 = [
            animated(1, 1, 1, 0.0),
            animated(2, 1, 2, 0.0),
            r(3, 1),
            r(4, 1),
            r(5, 1),
        ];
        let off2 = plan_batches(
            &list2,
            &BatchConfig {
                animated_instancing: false,
                ..BatchConfig::default()
            },
        )
        .unwrap();
        assert_eq!(counts(&off2), [3, 1, 1]);
        assert_eq!(off2.stats.singletons, 2);
    }

    // ---- limits -----------------------------------------------------------------------------------

    #[test]
    fn a_group_over_the_limit_is_split_keeping_instance_order_and_contiguity() {
        let list: Vec<Renderable> = (0..2500).map(|i| r(i, 1)).collect();
        let p = plan(&list);
        assert_eq!(counts(&p), [1024, 1024, 452]);
        let mut next = 0;
        for b in &p.batches {
            assert_eq!(b.first_instance, next, "slices are contiguous");
            next += b.instance_count;
        }
        assert_eq!(next, 2500);
        // The order is the id order straight across the batches.
        assert!(p.order.windows(2).all(|w| list[w[0]].id < list[w[1]].id));
        assert_eq!(p.stats.largest_batch, 1024);
        assert_eq!(p.stats.draw_calls_saved, 2500 - 3);
    }

    #[test]
    fn a_limit_of_one_draws_everything_alone_and_exactly_the_limit_fits() {
        let list: Vec<Renderable> = (0..5).map(|i| r(i, 1)).collect();
        let one = plan_batches(
            &list,
            &BatchConfig {
                max_instances_per_batch: 1,
                ..BatchConfig::default()
            },
        )
        .unwrap();
        assert_eq!(counts(&one), [1, 1, 1, 1, 1]);
        assert_eq!((one.stats.singletons, one.stats.draw_calls_saved), (5, 0));
        let exact = plan_batches(
            &list,
            &BatchConfig {
                max_instances_per_batch: 5,
                ..BatchConfig::default()
            },
        )
        .unwrap();
        assert_eq!(counts(&exact), [5]);
        let over = plan_batches(
            &list,
            &BatchConfig {
                max_instances_per_batch: 4,
                ..BatchConfig::default()
            },
        )
        .unwrap();
        assert_eq!(counts(&over), [4, 1]);
    }

    #[test]
    fn bad_limits_are_refused_and_an_empty_frame_is_fine() {
        for bad in [0, 65_537, usize::MAX] {
            let cfg = BatchConfig {
                max_instances_per_batch: bad,
                ..BatchConfig::default()
            };
            assert!(
                cfg.validate().is_err() && plan_batches(&[r(1, 1)], &cfg).is_err(),
                "{bad}"
            );
        }
        assert!(
            BatchConfig {
                max_instances_per_batch: 65_536,
                ..BatchConfig::default()
            }
            .validate()
            .is_ok()
        );
        let p = plan(&[]);
        assert!(p.batches.is_empty() && p.order.is_empty());
        assert_eq!(p.stats, BatchStats::default());
        assert!(pack_instances(&[], &p).is_empty());
    }

    // ---- determinism and completeness -----------------------------------------------------------------

    #[test]
    fn the_plan_does_not_depend_on_the_order_of_the_input_and_covers_every_renderable_once() {
        let mut seed = 0xDEAD_BEEF_1234_5678u64;
        let mut rnd = move |m: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % m
        };
        let base: Vec<Renderable> = (0..400)
            .map(|i| {
                let mut x = r(i, 1 + rnd(4) as u32);
                if rnd(5) == 0 {
                    x.material_override = Some(material(1 + rnd(2) as u32));
                }
                if rnd(6) == 0 {
                    x.animation = Some(AnimState {
                        clip: ClipId::from_raw(1 + rnd(3) as u32).unwrap(),
                        time: rnd(100) as f32,
                    });
                }
                if rnd(7) == 0 {
                    x.flags = RenderFlags::VIEWMODEL;
                }
                x
            })
            .collect();
        let cfg = BatchConfig {
            max_instances_per_batch: 37,
            animated_instancing: true,
        };
        let reference = plan_batches(&base, &cfg).unwrap();
        // Same set of renderables, shuffled: the batches and the instance ids in order are identical.
        let ids = |p: &BatchPlan, list: &[Renderable]| {
            p.order.iter().map(|i| list[*i].id).collect::<Vec<_>>()
        };
        let ref_ids = ids(&reference, &base);
        for _ in 0..10 {
            let mut shuffled = base.clone();
            for i in (1..shuffled.len()).rev() {
                shuffled.swap(i, rnd(i as u64 + 1) as usize);
            }
            let p = plan_batches(&shuffled, &cfg).unwrap();
            assert_eq!(p.batches, reference.batches);
            assert_eq!(ids(&p, &shuffled), ref_ids);
        }
        // Completeness: every input index exactly once, every batch within the limit, every batch uniform.
        assert_eq!(
            reference
                .order
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .len(),
            base.len()
        );
        assert_eq!(reference.order.len(), base.len());
        for b in &reference.batches {
            assert!(b.instance_count >= 1 && b.instance_count <= 37);
            for slot in b.first_instance..b.first_instance + b.instance_count {
                let x = &base[reference.order[slot]];
                assert_eq!(
                    (x.model, x.material_override, x.flags, x.animation.is_some()),
                    (b.model, b.material_override, b.flags, b.animated)
                );
            }
        }
        assert_eq!(
            reference
                .batches
                .iter()
                .map(|b| b.instance_count)
                .sum::<usize>(),
            base.len()
        );
    }

    #[test]
    fn a_horde_of_a_few_kinds_costs_a_few_draw_calls() {
        let list: Vec<Renderable> = (0..300)
            .map(|i| animated(i, 1 + (i % 3) as u32, 1 + (i % 2) as u32, i as f32 * 0.1))
            .collect();
        let p = plan(&list);
        assert_eq!(p.batches.len(), 3);
        assert_eq!(p.stats.draw_calls_saved, 297);
        assert!(
            p.batches
                .iter()
                .all(|b| b.instance_count == 100 && b.animated)
        );
    }

    // ---- packing ---------------------------------------------------------------------------------------------

    #[test]
    fn packed_data_follows_the_plan_and_carries_the_matrix_tint_animation_and_flags() {
        let a = Renderable {
            transform: Transform {
                translation: Vec3::new(1.0, 2.0, 3.0),
                rotation: Quat::from_rotation_y(0.5),
                scale: Vec3::new(2.0, 2.0, 2.0),
            },
            tint: [0.1, 0.2, 0.3, 0.4],
            animation: Some(AnimState {
                clip: ClipId::from_raw(9).unwrap(),
                time: 1.25,
            }),
            flags: RenderFlags::CAST_SHADOWS,
            ..r(20, 2)
        };
        let b = r(10, 2); // sorts before a (lower id), same model but a is animated: different batch
        let c = r(30, 1);
        let list = [a, b, c];
        let p = plan(&list);
        let data = pack_instances(&list, &p);
        assert_eq!(data.len(), 3 * INSTANCE_FLOATS);
        // Find each instance's slot through the plan rather than assuming the batch order.
        let slot_of = |id: u64| {
            p.order
                .iter()
                .position(|i| list[*i].id == RenderId(id))
                .unwrap()
        };
        let d = |id: u64| &data[slot_of(id) * INSTANCE_FLOATS..(slot_of(id) + 1) * INSTANCE_FLOATS];
        assert_eq!(&d(20)[..16], &a.transform.matrix().to_cols_array());
        assert_eq!(&d(20)[16..20], &[0.1, 0.2, 0.3, 0.4]);
        assert_eq!(d(20)[20], 1.25);
        assert_eq!(
            d(20)[21].to_bits(),
            9,
            "the clip id travels as a bit pattern"
        );
        assert_eq!(d(20)[22].to_bits(), RenderFlags::CAST_SHADOWS.0);
        assert_eq!(d(20)[23], 0.0);
        // A static renderable has no clip and no time.
        assert_eq!((d(10)[20], d(10)[21].to_bits()), (0.0, 0));
        assert_eq!(d(10)[16..20], [1.0; 4], "untinted");
        assert_eq!(
            d(30)[12],
            30.0,
            "translation x is in the last column of a column-major matrix"
        );
    }

    #[test]
    fn each_batch_slice_of_the_packed_buffer_holds_exactly_its_instances() {
        let list: Vec<Renderable> = (0..50).map(|i| r(i, 1 + (i % 2) as u32)).collect();
        let p = plan_batches(
            &list,
            &BatchConfig {
                max_instances_per_batch: 8,
                animated_instancing: true,
            },
        )
        .unwrap();
        let data = pack_instances(&list, &p);
        for b in &p.batches {
            for k in 0..b.instance_count {
                let slot = b.first_instance + k;
                let id = list[p.order[slot]].id.0;
                assert_eq!(
                    data[slot * INSTANCE_FLOATS + 12],
                    id as f32,
                    "translation x of the instance in slot {slot}"
                );
            }
        }
    }
}
