//! Server-side lag compensation (roadmap T08-010): let a hitscan or melee attack be judged against the
//! world **as the shooter saw it**, within a limit, so that a player with a slow connection is not
//! always shooting at where a target used to be.
//!
//! A shooter's screen shows the other entities a little in the past (the network delay plus the
//! client's interpolation delay). When the shot reaches the server the targets have moved on. The
//! server keeps a short **history of the moving colliders**, one entry per simulated tick, and for a shot
//! it asks [`LagCompensator::rewind`] which tick to use and answers the query against that tick's
//! entry ([`LagCompensator::raycast`]; the entry itself is [`LagCompensator::world_at`] for casts and overlaps).
//! **Only the moving colliders are rewound**; the static world (walls) is the live one, so a rewound target never
//! gets hit through a wall that exists now.
//!
//! **Fairness is a limit, not a promise.** The rewind is capped at `max_rewind_ticks` (the threat model's "a lag
//! switch cannot reach arbitrarily far back"); a claimed view tick in the future is ignored; a request older than the history
//! is answered with the oldest entry; and every answer says which of these happened ([`RewindOutcome`]) so the server can
//! count a client that keeps asking for more than it may have. Whether to rewind at all (a shooter with 400 ms of lag
//! against a 200 ms cap simply has to lead the target) is a game-design decision the numbers here make explicit.
//!
//! Pure and deterministic: ticks are integers, no clocks, no hash maps.

use std::collections::VecDeque;
use std::fmt;

use crate::geometry::Ray;
use crate::query::{DynamicCollider, Hit, QueryFilter, QueryWorld, SpatialQuery};

/// The longest rewind window that can be configured, in ticks (about four seconds at 60 Hz); it
/// bounds the memory of the history whatever a configuration file says.
pub const MAX_REWIND_TICKS: u32 = 256;

/// Why a history could not be configured or recorded into.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LagCompError {
    /// The window is longer than [`MAX_REWIND_TICKS`].
    WindowTooLong {
        /// What was asked for.
        given: u32,
    },
    /// A tick that is not newer than the newest one recorded.
    NotNewer {
        /// The newest tick in the history.
        newest: u32,
        /// The tick that was given.
        given: u32,
    },
}

impl fmt::Display for LagCompError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WindowTooLong { given } => write!(
                f,
                "a rewind window of {given} ticks is longer than the limit of {MAX_REWIND_TICKS}"
            ),
            Self::NotNewer { newest, given } => write!(
                f,
                "tick {given} is not newer than the newest recorded tick {newest}"
            ),
        }
    }
}

impl std::error::Error for LagCompError {}

/// How far back the server may look.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LagCompConfig {
    max_rewind_ticks: u32,
}

impl LagCompConfig {
    /// A window of `max_rewind_ticks` ticks (0 turns lag compensation off); at most [`MAX_REWIND_TICKS`].
    pub fn new(max_rewind_ticks: u32) -> Result<Self, LagCompError> {
        if max_rewind_ticks > MAX_REWIND_TICKS {
            return Err(LagCompError::WindowTooLong {
                given: max_rewind_ticks,
            });
        }
        Ok(Self { max_rewind_ticks })
    }

    /// The window, in ticks.
    pub fn max_rewind_ticks(&self) -> u32 {
        self.max_rewind_ticks
    }
}

/// Which limit, if any, shaped a rewind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RewindOutcome {
    /// The shooter's view tick was used as it is.
    Exact,
    /// The view was older than the window; the oldest allowed tick is used.
    Clamped,
    /// The view tick was ahead of the server's: ignored, the current tick is used.
    FutureIgnored,
    /// The history does not reach back that far (a new match, or a gap): its oldest entry is used.
    HistoryShort,
}

/// The tick to judge a shot at, and why.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rewind {
    /// The tick of the history entry to use.
    pub tick: u32,
    /// How many ticks back the shooter's view was.
    pub requested_ticks: u32,
    /// How many ticks back the server went.
    pub applied_ticks: u32,
    /// The most limiting reason the two differ, or `Exact`.
    pub outcome: RewindOutcome,
}

/// A bounded history of the moving colliders, one entry per recorded tick.
///
/// ```
/// use ss2_core::math::Vec3;
/// use ss2_physics::geometry::Ray;
/// use ss2_physics::lagcomp::{LagCompConfig, LagCompensator};
/// use ss2_physics::query::{ColliderId, DynamicCollider, PhysMaterialId, QueryFilter, QueryWorld, Shape};
/// use ss2_physics::spatial::Layers;
///
/// let mut lc = LagCompensator::new(LagCompConfig::new(10).unwrap());
/// for tick in 90..=100u32 {
///     lc.record(tick, vec![DynamicCollider {
///         id: ColliderId(7),
///         layers: Layers::ALL,
///         shape: Shape::Sphere { radius: 1.0 },
///         position: Vec3::new((tick - 90) as f32 * 3.0, 0.0, -20.0), // runs along x, 3 units per tick
///         material: PhysMaterialId(0),
///     }]).unwrap();
/// }
/// // The shooter, at tick 100, saw the target as it was at tick 96 and aims there.
/// let rewind = lc.rewind(100, 96);
/// let ray = Ray::new(Vec3::ZERO, Vec3::new(18.0, 0.0, -20.0)).unwrap(); // x at tick 96
/// let live = QueryWorld::new(); // the static world
/// assert!(lc.raycast(&live, &rewind, &ray, 500.0, &QueryFilter::default()).is_some());
/// // Judged at the current tick, the same shot misses: the target has moved on.
/// assert!(lc.raycast(&live, &lc.rewind(100, 100), &ray, 500.0, &QueryFilter::default()).is_none());
/// ```
pub struct LagCompensator {
    config: LagCompConfig,
    history: VecDeque<(u32, QueryWorld)>,
}

impl LagCompensator {
    /// An empty history.
    pub fn new(config: LagCompConfig) -> Self {
        Self {
            config,
            history: VecDeque::new(),
        }
    }

    /// Record the moving colliders of `tick`, which must be newer than the last one recorded
    /// (gaps are allowed). Entries older than the window are dropped.
    pub fn record(
        &mut self,
        tick: u32,
        colliders: Vec<DynamicCollider>,
    ) -> Result<(), LagCompError> {
        if let Some(newest) = self.newest_tick() {
            if tick <= newest {
                return Err(LagCompError::NotNewer {
                    newest,
                    given: tick,
                });
            }
        }
        let mut world = QueryWorld::new();
        world.set_dynamic(colliders);
        self.history.push_back((tick, world));
        let window = u64::from(self.config.max_rewind_ticks);
        while self
            .history
            .front()
            .is_some_and(|(t, _)| u64::from(*t) + window < u64::from(tick))
        {
            self.history.pop_front();
        }
        Ok(())
    }

    /// How many ticks are kept.
    pub fn len(&self) -> usize {
        self.history.len()
    }

    /// Whether nothing is recorded.
    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }

    /// The oldest tick kept.
    pub fn oldest_tick(&self) -> Option<u32> {
        self.history.front().map(|(t, _)| *t)
    }

    /// The newest tick kept.
    pub fn newest_tick(&self) -> Option<u32> {
        self.history.back().map(|(t, _)| *t)
    }

    /// Which tick to judge a shot at. `current_tick` is the server's tick when the shot is processed and
    /// `view_tick` the tick of the snapshot the shooter was showing when firing (the network delay and
    /// the client's interpolation delay already inside it). See [`RewindOutcome`].
    pub fn rewind(&self, current_tick: u32, view_tick: u32) -> Rewind {
        if view_tick > current_tick {
            return Rewind {
                tick: current_tick,
                requested_ticks: 0,
                applied_ticks: 0,
                outcome: RewindOutcome::FutureIgnored,
            };
        }
        let requested = current_tick - view_tick;
        if requested == 0 {
            return Rewind {
                tick: current_tick,
                requested_ticks: 0,
                applied_ticks: 0,
                outcome: RewindOutcome::Exact,
            };
        }
        let allowed = requested.min(self.config.max_rewind_ticks);
        let mut tick = current_tick - allowed;
        let mut outcome = if allowed < requested {
            RewindOutcome::Clamped
        } else {
            RewindOutcome::Exact
        };
        match self.oldest_tick() {
            Some(oldest) if oldest <= tick => {}
            Some(oldest) => {
                tick = oldest.min(current_tick);
                outcome = RewindOutcome::HistoryShort;
            }
            None => {
                tick = current_tick;
                outcome = RewindOutcome::HistoryShort;
            }
        }
        Rewind {
            tick,
            requested_ticks: requested,
            applied_ticks: current_tick - tick,
            outcome,
        }
    }

    /// The moving colliders as they were at `tick`: the last entry recorded at or before it, `None`
    /// when `tick` is older than the history. The world has **no static geometry**; ask it
    /// with a filter that leaves statics out, or use [`raycast`](Self::raycast).
    pub fn world_at(&self, tick: u32) -> Option<&QueryWorld> {
        let after = self.history.partition_point(|(t, _)| *t <= tick);
        after.checked_sub(1).map(|i| &self.history[i].1)
    }

    /// A ray judged against the live static world `live` and the moving colliders as they were at
    /// `rewind.tick`. The nearest hit wins; a moving collider at the same distance as the static world
    /// wins (as in [`QueryWorld`]). `live` is queried for statics only.
    pub fn raycast<S: SpatialQuery>(
        &self,
        live: &S,
        rewind: &Rewind,
        ray: &Ray,
        max_distance: f32,
        filter: &QueryFilter,
    ) -> Option<Hit> {
        let static_hit = if filter.statics {
            live.raycast(
                ray,
                max_distance,
                &QueryFilter {
                    dynamics: false,
                    ..*filter
                },
            )
        } else {
            None
        };
        let limit = static_hit.map_or(max_distance, |h| h.distance);
        let moving_hit = if filter.dynamics {
            self.world_at(rewind.tick).and_then(|w| {
                w.raycast(
                    ray,
                    limit,
                    &QueryFilter {
                        statics: false,
                        ..*filter
                    },
                )
            })
        } else {
            None
        };
        moving_hit.or(static_hit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Ray;
    use crate::query::{
        ColliderId, DynamicCollider, PhysMaterialId, QueryFilter, QueryWorld, Shape, SpatialQuery,
    };
    use crate::spatial::{Layers, TriMesh};
    use ss2_core::math::Vec3;

    const TARGET: ColliderId = ColliderId(7);
    const SHOOTER: ColliderId = ColliderId(1);

    fn body(id: ColliderId, at: Vec3) -> DynamicCollider {
        DynamicCollider {
            id,
            layers: Layers::ALL,
            shape: Shape::Sphere { radius: 1.0 },
            position: at,
            material: PhysMaterialId(3),
        }
    }

    /// Where the target is at a tick: it runs along x at three units per tick.
    fn x_at(tick: u32) -> f32 {
        (tick as f32 - 90.0) * 3.0
    }

    /// A target that runs along x at three units per tick, 20 units in front of the shooter (z = -20),
    /// recorded for ticks `from..=to`; the shooter stands still at the origin.
    fn recorded(config: LagCompConfig, from: u32, to: u32) -> LagCompensator {
        let mut lc = LagCompensator::new(config);
        for tick in from..=to {
            lc.record(
                tick,
                vec![
                    body(SHOOTER, Vec3::ZERO),
                    body(TARGET, Vec3::new(x_at(tick), 0.0, -20.0)),
                ],
            )
            .unwrap();
        }
        lc
    }

    fn config(max: u32) -> LagCompConfig {
        LagCompConfig::new(max).unwrap()
    }

    fn aim_at_x(x: f32) -> Ray {
        Ray::new(Vec3::ZERO, Vec3::new(x, 0.0, -20.0)).unwrap()
    }

    fn filter() -> QueryFilter {
        QueryFilter {
            exclude: Some(SHOOTER),
            ..QueryFilter::default()
        }
    }

    // ---- which tick ---------------------------------------------------------------------------

    #[test]
    fn a_view_within_the_window_is_used_as_it_is() {
        let lc = recorded(config(10), 90, 100);
        let r = lc.rewind(100, 97);
        assert_eq!(
            r,
            Rewind {
                tick: 97,
                requested_ticks: 3,
                applied_ticks: 3,
                outcome: RewindOutcome::Exact
            }
        );
        assert_eq!(
            lc.rewind(100, 100).outcome,
            RewindOutcome::Exact,
            "no lag is no rewind"
        );
        assert_eq!(lc.rewind(100, 100).applied_ticks, 0);
    }

    #[test]
    fn a_view_older_than_the_window_is_clamped_to_the_window() {
        let lc = recorded(config(10), 90, 100);
        let r = lc.rewind(100, 80);
        assert_eq!(
            (r.tick, r.requested_ticks, r.applied_ticks, r.outcome),
            (90, 20, 10, RewindOutcome::Clamped)
        );
        // Exactly at the limit is allowed.
        assert_eq!(lc.rewind(100, 90).outcome, RewindOutcome::Exact);
        assert_eq!(lc.rewind(100, 89).outcome, RewindOutcome::Clamped);
    }

    #[test]
    fn a_view_in_the_future_is_ignored_and_the_current_tick_is_used() {
        let lc = recorded(config(10), 90, 100);
        let r = lc.rewind(100, 105);
        assert_eq!(
            (r.tick, r.requested_ticks, r.applied_ticks, r.outcome),
            (100, 0, 0, RewindOutcome::FutureIgnored)
        );
        assert_eq!(
            lc.rewind(100, u32::MAX).outcome,
            RewindOutcome::FutureIgnored
        );
    }

    #[test]
    fn a_history_that_is_not_long_enough_answers_with_its_oldest_entry() {
        let lc = recorded(config(10), 96, 100); // only five ticks recorded so far
        let r = lc.rewind(100, 92);
        assert_eq!(
            (r.tick, r.requested_ticks, r.applied_ticks, r.outcome),
            (96, 8, 4, RewindOutcome::HistoryShort)
        );
        let empty = LagCompensator::new(config(10));
        let r = empty.rewind(100, 98);
        assert_eq!(
            (r.applied_ticks, r.outcome),
            (0, RewindOutcome::HistoryShort)
        );
    }

    #[test]
    fn a_zero_window_never_rewinds() {
        let lc = recorded(config(0), 100, 100);
        let r = lc.rewind(100, 97);
        assert_eq!(
            (r.tick, r.applied_ticks, r.outcome),
            (100, 0, RewindOutcome::Clamped)
        );
    }

    #[test]
    fn tick_arithmetic_does_not_overflow_near_the_ends_of_the_range() {
        let mut lc = LagCompensator::new(config(10));
        lc.record(u32::MAX - 1, vec![body(TARGET, Vec3::ZERO)])
            .unwrap();
        lc.record(u32::MAX, vec![body(TARGET, Vec3::ZERO)]).unwrap();
        assert_eq!(lc.rewind(u32::MAX, u32::MAX - 1).tick, u32::MAX - 1);
        let early = recorded(config(10), 0, 3);
        assert_eq!(
            early.rewind(3, 0).tick,
            0,
            "a window that reaches below tick 0 does not wrap"
        );
        assert_eq!(early.rewind(3, 0).outcome, RewindOutcome::Exact);
    }

    // ---- recording ----------------------------------------------------------------------------

    #[test]
    fn only_the_window_is_kept_however_long_the_game_runs() {
        let mut lc = LagCompensator::new(config(8));
        for tick in 0..10_000 {
            lc.record(tick, vec![body(TARGET, Vec3::new(tick as f32, 0.0, 0.0))])
                .unwrap();
        }
        assert_eq!(lc.len(), 9, "the window and the current tick");
        assert_eq!(lc.oldest_tick(), Some(9_991));
        assert_eq!(lc.newest_tick(), Some(9_999));
        assert!(lc.world_at(9_990).is_none(), "older than the window");
        assert!(lc.world_at(9_991).is_some());
    }

    #[test]
    fn a_tick_that_is_not_newer_is_refused_and_changes_nothing() {
        let mut lc = recorded(config(10), 90, 95);
        assert_eq!(
            lc.record(95, vec![]),
            Err(LagCompError::NotNewer {
                newest: 95,
                given: 95
            })
        );
        assert_eq!(
            lc.record(80, vec![]),
            Err(LagCompError::NotNewer {
                newest: 95,
                given: 80
            })
        );
        assert_eq!((lc.len(), lc.newest_tick()), (6, Some(95)));
    }

    #[test]
    fn the_window_is_limited_so_that_memory_stays_bounded() {
        assert!(LagCompConfig::new(MAX_REWIND_TICKS).is_ok());
        assert!(LagCompConfig::new(MAX_REWIND_TICKS + 1).is_err());
        assert!(LagCompConfig::new(u32::MAX).is_err());
        assert!(LagCompConfig::new(0).is_ok());
    }

    #[test]
    fn a_gap_in_the_ticks_uses_the_last_state_at_or_before() {
        let mut lc = LagCompensator::new(config(10));
        lc.record(90, vec![body(TARGET, Vec3::new(0.0, 0.0, -20.0))])
            .unwrap();
        lc.record(93, vec![body(TARGET, Vec3::new(30.0, 0.0, -20.0))])
            .unwrap();
        // Tick 91 and 92 were never recorded: the state of tick 90 holds.
        let w = lc.world_at(92).unwrap();
        let hit = w
            .raycast(&aim_at_x(0.0), 100.0, &QueryFilter::default())
            .unwrap();
        assert_eq!(hit.collider, Some(TARGET));
        assert!(
            lc.world_at(89).is_none(),
            "nothing older than the oldest entry"
        );
        let at_93 = lc.world_at(93).unwrap();
        assert!(
            at_93
                .raycast(&aim_at_x(0.0), 100.0, &QueryFilter::default())
                .is_none()
        );
    }

    // ---- shots --------------------------------------------------------------------------------

    #[test]
    fn a_shot_at_where_the_target_was_hits_only_with_the_rewind() {
        let lc = recorded(config(10), 90, 100);
        let live = QueryWorld::new();
        // The shooter saw the target at tick 96.
        let aim = aim_at_x(x_at(96));
        let rewound = lc.rewind(100, 96);
        let hit = lc
            .raycast(&live, &rewound, &aim, 200.0, &filter())
            .expect("hit as it was seen");
        assert_eq!(hit.collider, Some(TARGET));
        // Without the rewind (the live tick) the same shot misses: the target has moved four units on.
        let no_rewind = lc.rewind(100, 100);
        assert!(
            lc.raycast(&live, &no_rewind, &aim, 200.0, &filter())
                .is_none()
        );
    }

    #[test]
    fn a_shot_cannot_reach_further_back_than_the_window() {
        let lc = recorded(config(5), 80, 100);
        let live = QueryWorld::new();
        let aim = aim_at_x(x_at(85)); // where the target was at tick 85, 15 ticks ago
        let r = lc.rewind(100, 85);
        assert_eq!(r.outcome, RewindOutcome::Clamped);
        assert!(
            lc.raycast(&live, &r, &aim, 200.0, &filter()).is_none(),
            "the window ends at tick 95"
        );
        let aim_in_window = aim_at_x(x_at(95));
        assert!(
            lc.raycast(&live, &r, &aim_in_window, 200.0, &filter())
                .is_some()
        );
    }

    #[test]
    fn the_live_wall_still_blocks_a_rewound_target() {
        let lc = recorded(config(10), 90, 100);
        // A wall at z = -10, 10 units in front of the shooter and in front of the target (z = -20).
        let v = [
            Vec3::new(-200.0, -10.0, -10.0),
            Vec3::new(200.0, -10.0, -10.0),
            Vec3::new(200.0, 10.0, -10.0),
            Vec3::new(-200.0, 10.0, -10.0),
        ];
        let mut live = QueryWorld::new();
        live.set_static(
            TriMesh::new(&v, &[0, 1, 2, 0, 2, 3]).unwrap(),
            vec![PhysMaterialId(9), PhysMaterialId(9)],
        )
        .unwrap();
        let aim = aim_at_x(x_at(96));
        let r = lc.rewind(100, 96);
        let hit = lc
            .raycast(&live, &r, &aim, 200.0, &filter())
            .expect("the wall is hit");
        assert_eq!(
            hit.collider, None,
            "the static world, not the target behind it"
        );
        assert!(
            (hit.distance - (10.0 * (x_at(96) * x_at(96) + 400.0).sqrt() / 20.0)).abs() < 0.01,
            "{}",
            hit.distance
        );
    }

    #[test]
    fn a_target_in_front_of_the_wall_is_hit_in_preference_to_it() {
        let lc = recorded(config(10), 90, 100);
        // The wall is behind the target (z = -30).
        let v = [
            Vec3::new(-200.0, -10.0, -30.0),
            Vec3::new(200.0, -10.0, -30.0),
            Vec3::new(200.0, 10.0, -30.0),
            Vec3::new(-200.0, 10.0, -30.0),
        ];
        let mut live = QueryWorld::new();
        live.set_static(
            TriMesh::new(&v, &[0, 1, 2, 0, 2, 3]).unwrap(),
            vec![PhysMaterialId(9), PhysMaterialId(9)],
        )
        .unwrap();
        let r = lc.rewind(100, 96);
        let hit = lc
            .raycast(&live, &r, &aim_at_x(x_at(96)), 200.0, &filter())
            .unwrap();
        assert_eq!(hit.collider, Some(TARGET));
    }

    #[test]
    fn the_shooters_own_body_and_the_layer_mask_are_respected() {
        let lc = recorded(config(10), 90, 100);
        let live = QueryWorld::new();
        let r = lc.rewind(100, 96);
        // Aim at the shooter's own position (the target is on the line at x = 0 only at tick 0): shoot along +z from inside the shooter's sphere.
        let own = Ray::new(Vec3::ZERO, Vec3::Z).unwrap();
        assert!(
            lc.raycast(&live, &r, &own, 10.0, &filter()).is_none(),
            "the shooter is excluded"
        );
        let with_self = QueryFilter::default();
        let hit = lc.raycast(&live, &r, &own, 10.0, &with_self).unwrap();
        assert_eq!(hit.collider, Some(SHOOTER));
        let nothing = QueryFilter {
            mask: Layers::NONE,
            ..QueryFilter::default()
        };
        assert!(
            lc.raycast(&live, &r, &aim_at_x(x_at(96)), 200.0, &nothing)
                .is_none()
        );
        let statics_only = QueryFilter {
            dynamics: false,
            ..QueryFilter::default()
        };
        assert!(
            lc.raycast(&live, &r, &aim_at_x(x_at(96)), 200.0, &statics_only)
                .is_none()
        );
    }

    #[test]
    fn the_live_moving_colliders_are_not_consulted_only_the_live_static_world() {
        let lc = recorded(config(10), 90, 100);
        // The live world also holds the target where it is NOW (tick 100).
        let mut live = QueryWorld::new();
        live.set_dynamic(vec![body(TARGET, Vec3::new(x_at(100), 0.0, -20.0))]);
        let r = lc.rewind(100, 96);
        let now_position = aim_at_x(x_at(100));
        assert!(
            lc.raycast(&live, &r, &now_position, 200.0, &filter())
                .is_none(),
            "the rewound target is not at its current position"
        );
        // ...and the rewound position still hits.
        assert!(
            lc.raycast(&live, &r, &aim_at_x(x_at(96)), 200.0, &filter())
                .is_some()
        );
    }

    #[test]
    fn a_current_tick_older_than_the_history_never_goes_forward() {
        let lc = recorded(config(10), 96, 100);
        // A shot processed for tick 95 (before anything recorded): the oldest entry is later than "now".
        let r = lc.rewind(95, 90);
        assert_eq!(
            (r.tick, r.applied_ticks, r.outcome),
            (95, 0, RewindOutcome::HistoryShort)
        );
    }

    #[test]
    fn the_rewound_world_answers_other_queries_too() {
        // A melee swing is an overlap or a cast: world_at gives the historical moving colliders.
        let lc = recorded(config(10), 90, 100);
        let r = lc.rewind(100, 94);
        let w = lc.world_at(r.tick).unwrap();
        let mut out = Vec::new();
        w.overlap(
            &Shape::Sphere { radius: 0.5 },
            Vec3::new(x_at(94), 0.0, -20.0),
            &QueryFilter::default(),
            &mut out,
        )
        .unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].collider, Some(TARGET));
        out.clear();
        w.overlap(
            &Shape::Sphere { radius: 0.5 },
            Vec3::new(x_at(100), 0.0, -20.0),
            &QueryFilter::default(),
            &mut out,
        )
        .unwrap();
        assert!(out.is_empty(), "where the target is now it was not then");
    }

    #[test]
    fn the_answer_does_not_depend_on_the_order_the_colliders_were_recorded_in() {
        let a = body(SHOOTER, Vec3::ZERO);
        let b = body(TARGET, Vec3::new(5.0, 0.0, -20.0));
        let mut one = LagCompensator::new(config(4));
        one.record(1, vec![a, b]).unwrap();
        let mut two = LagCompensator::new(config(4));
        two.record(1, vec![b, a]).unwrap();
        let live = QueryWorld::new();
        let r = one.rewind(1, 1);
        let ray = Ray::new(Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, -1.0)).unwrap();
        assert_eq!(
            one.raycast(&live, &r, &ray, 100.0, &QueryFilter::default()),
            two.raycast(&live, &r, &ray, 100.0, &QueryFilter::default())
        );
    }
}
