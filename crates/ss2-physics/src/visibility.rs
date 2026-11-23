//! Visibility queries for AI and gameplay (roadmap T07-024): **can A see B**, and **how far does a sound carry**.
//!
//! * **Sight.** [`line_of_sight`] casts a segment between two points against the world
//!   ([`SpatialQuery`]) and says whether something opaque is in the way. The target's own collider does not
//!   block (a ray to the centre of a body first meets the body's surface; hitting the target *is* seeing
//!   it), and which layers are opaque is the caller's filter mask (glass, triggers and water volumes are
//!   simply not in it). [`exposure`] samples several points of a target (head, chest, feet) and counts how many are
//!   visible, for partial cover. [`in_cone`] and [`Perception`] add a range and a field of view.
//!   **The field of view is a cosine, not an angle**: the physics crate keeps trigonometry out
//!   so that results are the same bits on every platform (`docs/GEOMETRY.md`); convert degrees once, in configuration.
//! * **Sound.** A [`SoundGraph`] is the map of sectors joined by portals (doors, windows, openings). A sound of a
//!   given loudness (the distance it carries in open air) travels through portals; each costs its length, a closed
//!   one costs an extra penalty (a door muffles). [`SoundGraph::propagate`] returns, for every sector the sound
//!   reaches, how much loudness is left there. Where the sectors and portals come from (the level data, T07-015)
//!   is not this module's business: it takes the graph as data.
//!
//! Pure and deterministic: no clocks, no hash maps, ties broken by sector number.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};
use std::fmt;

use crate::geometry::Ray;
use crate::query::{ColliderId, QueryFilter, SpatialQuery};
use ss2_core::math::Vec3;

/// What stands between two points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sight {
    /// Nothing opaque is in the way (or the target itself is the first thing met).
    Clear,
    /// Something opaque blocks the line.
    Blocked {
        /// How far from the start the line meets it.
        distance: f32,
        /// What blocks it; `None` for the static world.
        by: Option<ColliderId>,
    },
    /// The points are farther apart than the maximum distance.
    OutOfRange,
    /// A point is not finite.
    Invalid,
}

/// How a line of sight is cast.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SightParams {
    /// The farthest a line may reach.
    pub max_distance: f32,
    /// Which layers are opaque, and a collider to ignore (the looker's own body).
    pub filter: QueryFilter,
    /// How far short of the target the line stops, so that a surface the target stands on or against does not block.
    pub margin: f32,
}

impl Default for SightParams {
    /// 100 units, everything opaque, a margin of 0.01.
    fn default() -> Self {
        Self {
            max_distance: 100.0,
            filter: QueryFilter::default(),
            margin: 0.01,
        }
    }
}

/// Whether the segment from `from` to `to` is free of opaque things. A hit on `target` counts as clear.
pub fn line_of_sight<S: SpatialQuery>(
    world: &S,
    from: Vec3,
    to: Vec3,
    target: Option<ColliderId>,
    params: &SightParams,
) -> Sight {
    if !from.is_finite() || !to.is_finite() {
        return Sight::Invalid;
    }
    let delta = to - from;
    let length = delta.length();
    if length > params.max_distance {
        return Sight::OutOfRange;
    }
    let reach = length - params.margin.max(0.0);
    let Some(ray) = Ray::new(from, delta) else {
        return Sight::Clear; // the same point
    };
    if reach <= 0.0 {
        return Sight::Clear;
    }
    match world.raycast(&ray, reach, &params.filter) {
        None => Sight::Clear,
        Some(hit) if target.is_some() && hit.collider == target => Sight::Clear,
        Some(hit) => Sight::Blocked {
            distance: hit.distance,
            by: hit.collider,
        },
    }
}

/// How many of a target's sample points are visible.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Exposure {
    /// Points with a clear line.
    pub visible: usize,
    /// Points tried.
    pub total: usize,
}

impl Exposure {
    /// At least one point is visible.
    pub fn any(&self) -> bool {
        self.visible > 0
    }

    /// Every point is visible.
    pub fn full(&self) -> bool {
        self.total > 0 && self.visible == self.total
    }
}

/// Cast a line from `eye` to each of `points` and count the clear ones.
pub fn exposure<S: SpatialQuery>(
    world: &S,
    eye: Vec3,
    points: &[Vec3],
    target: Option<ColliderId>,
    params: &SightParams,
) -> Exposure {
    let visible = points
        .iter()
        .filter(|p| line_of_sight(world, eye, **p, target, params) == Sight::Clear)
        .count();
    Exposure {
        visible,
        total: points.len(),
    }
}

/// Whether `to` lies within a cone from `from` along the **unit** vector `forward`: the cosine of the
/// angle between `forward` and the direction to `to` is at least `min_cos` (1 is a line, 0 a half space,
/// -1 everything). The same point is always inside; a non-finite input is never.
pub fn in_cone(from: Vec3, forward: Vec3, min_cos: f32, to: Vec3) -> bool {
    if !from.is_finite() || !forward.is_finite() || !to.is_finite() || !min_cos.is_finite() {
        return false;
    }
    let delta = to - from;
    let length = delta.length();
    if length <= 1e-6 {
        return true;
    }
    forward.dot(delta) / length >= min_cos
}

/// Why a target was not seen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Unseen {
    /// Too far.
    OutOfRange,
    /// Outside the field of view.
    OutsideCone,
    /// Blocked (by what, how far along).
    Blocked(Sight),
    /// An input was not finite.
    Invalid,
}

/// What a looker saw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seen {
    /// Every sample point is visible.
    Full,
    /// Some sample points are visible.
    Partial(Exposure),
    /// Nothing.
    No(Unseen),
}

/// A looker's range and field of view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Perception {
    /// The cosine of half the field of view (see [`in_cone`]).
    pub min_cos: f32,
    /// How the lines are cast, including the range.
    pub sight: SightParams,
}

impl Perception {
    /// Whether a looker at `eye` facing the unit vector `forward` sees a target made of `points` (the first is
    /// taken as its centre for the field of view test).
    pub fn look<S: SpatialQuery>(
        &self,
        world: &S,
        eye: Vec3,
        forward: Vec3,
        points: &[Vec3],
        target: Option<ColliderId>,
    ) -> Seen {
        let Some(centre) = points.first() else {
            return Seen::No(Unseen::Invalid);
        };
        if !eye.is_finite() || !centre.is_finite() {
            return Seen::No(Unseen::Invalid);
        }
        if (*centre - eye).length() > self.sight.max_distance {
            return Seen::No(Unseen::OutOfRange);
        }
        if !in_cone(eye, forward, self.min_cos, *centre) {
            return Seen::No(Unseen::OutsideCone);
        }
        let e = exposure(world, eye, points, target, &self.sight);
        if e.full() {
            Seen::Full
        } else if e.any() {
            Seen::Partial(e)
        } else {
            // Report what blocked the first line.
            Seen::No(Unseen::Blocked(line_of_sight(
                world,
                eye,
                *centre,
                target,
                &self.sight,
            )))
        }
    }
}

/// A sector of the level, for sound.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SectorId(pub u32);

/// A way sound gets from one sector to another.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Portal {
    /// One side.
    pub a: SectorId,
    /// The other side.
    pub b: SectorId,
    /// The distance sound travels to cross it (between the sectors' centres, say).
    pub length: f32,
    /// What it costs in extra distance when it is closed (a door, a window).
    pub closed_penalty: f32,
    /// Whether it is open.
    pub open: bool,
}

/// The index of a portal in its graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PortalId(pub usize);

/// Why a portal could not be added.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphError {
    /// A length or penalty is negative or not finite.
    BadCost,
    /// The graph already has the most portals it may.
    TooManyPortals,
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadCost => write!(
                f,
                "a portal's length and penalty must be finite and not negative"
            ),
            Self::TooManyPortals => write!(f, "too many portals"),
        }
    }
}

impl std::error::Error for GraphError {}

/// The most portals a graph holds.
pub const MAX_PORTALS: usize = 1 << 20;

/// Sectors joined by portals, for carrying sound.
///
/// ```
/// use ss2_physics::visibility::{Portal, SectorId, SoundGraph};
///
/// let mut graph = SoundGraph::new();
/// let door = graph.add_portal(Portal { a: SectorId(0), b: SectorId(1), length: 10.0, closed_penalty: 30.0, open: false }).unwrap();
/// // A sound of loudness 50 in sector 0: through the closed door only 10 is left in sector 1.
/// assert_eq!(graph.propagate(SectorId(0), 50.0)[&SectorId(1)], 10.0);
/// graph.set_open(door, true);
/// assert_eq!(graph.propagate(SectorId(0), 50.0)[&SectorId(1)], 40.0);
/// ```
#[derive(Clone, Debug, Default)]
pub struct SoundGraph {
    portals: Vec<Portal>,
    adjacent: BTreeMap<SectorId, Vec<usize>>,
}

impl SoundGraph {
    /// An empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a portal.
    pub fn add_portal(&mut self, portal: Portal) -> Result<PortalId, GraphError> {
        let ok = |x: f32| x.is_finite() && x >= 0.0;
        if !ok(portal.length) || !ok(portal.closed_penalty) {
            return Err(GraphError::BadCost);
        }
        if self.portals.len() >= MAX_PORTALS {
            return Err(GraphError::TooManyPortals);
        }
        let index = self.portals.len();
        self.portals.push(portal);
        self.adjacent.entry(portal.a).or_default().push(index);
        if portal.b != portal.a {
            self.adjacent.entry(portal.b).or_default().push(index);
        }
        Ok(PortalId(index))
    }

    /// Open or close a portal; `false` if there is no such portal.
    pub fn set_open(&mut self, id: PortalId, open: bool) -> bool {
        match self.portals.get_mut(id.0) {
            Some(p) => {
                p.open = open;
                true
            }
            None => false,
        }
    }

    /// How many portals there are.
    pub fn portal_count(&self) -> usize {
        self.portals.len()
    }

    /// The loudness left in every sector a sound reaches. `loudness` is the distance the sound carries in open air; the
    /// source sector has all of it, and a sector `d` away through portals (with their penalties) has
    /// `loudness - d`, if that is not negative. A loudness that is not finite or is negative reaches nowhere (an empty map).
    pub fn propagate(&self, source: SectorId, loudness: f32) -> BTreeMap<SectorId, f32> {
        let mut reached = BTreeMap::new();
        if !loudness.is_finite() || loudness < 0.0 {
            return reached;
        }
        // Dijkstra on the distance travelled. Costs are not negative, so their bit patterns order like the numbers.
        let mut best: BTreeMap<SectorId, f32> = BTreeMap::new();
        let mut queue: BinaryHeap<Reverse<(u32, SectorId)>> = BinaryHeap::new();
        best.insert(source, 0.0);
        queue.push(Reverse((0.0f32.to_bits(), source)));
        while let Some(Reverse((bits, sector))) = queue.pop() {
            let cost = f32::from_bits(bits);
            if best.get(&sector).is_some_and(|b| *b < cost) {
                continue; // a shorter way was found since this entry was queued
            }
            reached.insert(sector, loudness - cost);
            for &index in self.adjacent.get(&sector).map(Vec::as_slice).unwrap_or(&[]) {
                let p = &self.portals[index];
                let other = if p.a == sector { p.b } else { p.a };
                let step = p.length + if p.open { 0.0 } else { p.closed_penalty };
                let next = cost + step;
                if next > loudness {
                    continue;
                }
                if best.get(&other).is_none_or(|b| next < *b) {
                    best.insert(other, next);
                    queue.push(Reverse((next.to_bits(), other)));
                }
            }
        }
        reached
    }

    /// The loudness left in `listener`'s sector, if the sound reaches it.
    pub fn heard_at(&self, source: SectorId, loudness: f32, listener: SectorId) -> Option<f32> {
        self.propagate(source, loudness).get(&listener).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::{DynamicCollider, PhysMaterialId, QueryWorld, Shape};
    use crate::spatial::{Layers, TriMesh};

    fn wall_at(z: f32) -> QueryWorld {
        let v = [
            Vec3::new(-50.0, -50.0, z),
            Vec3::new(50.0, -50.0, z),
            Vec3::new(50.0, 50.0, z),
            Vec3::new(-50.0, 50.0, z),
        ];
        let mut w = QueryWorld::new();
        w.set_static(
            TriMesh::new(&v, &[0, 1, 2, 0, 2, 3]).unwrap(),
            vec![PhysMaterialId(0); 2],
        )
        .unwrap();
        w
    }

    fn body(id: u64, at: Vec3, layers: Layers) -> DynamicCollider {
        DynamicCollider {
            id: ColliderId(id),
            layers,
            shape: Shape::Sphere { radius: 1.0 },
            position: at,
            material: PhysMaterialId(0),
        }
    }

    fn params() -> SightParams {
        SightParams::default()
    }

    // ---- line of sight ------------------------------------------------------------------------

    #[test]
    fn nothing_in_the_way_is_clear_and_a_wall_blocks_with_its_distance() {
        let empty = QueryWorld::new();
        assert_eq!(
            line_of_sight(
                &empty,
                Vec3::ZERO,
                Vec3::new(0.0, 0.0, -20.0),
                None,
                &params()
            ),
            Sight::Clear
        );
        let wall = wall_at(-10.0);
        match line_of_sight(
            &wall,
            Vec3::ZERO,
            Vec3::new(0.0, 0.0, -20.0),
            None,
            &params(),
        ) {
            Sight::Blocked { distance, by } => {
                assert!((distance - 10.0).abs() < 1e-3, "{distance}");
                assert_eq!(by, None, "the static world");
            }
            other => panic!("{other:?}"),
        }
        // The same wall, but the target is in front of it.
        assert_eq!(
            line_of_sight(
                &wall,
                Vec3::ZERO,
                Vec3::new(0.0, 0.0, -9.0),
                None,
                &params()
            ),
            Sight::Clear
        );
    }

    #[test]
    fn sight_is_symmetric_for_the_static_world() {
        let wall = wall_at(-10.0);
        let (a, b) = (Vec3::new(1.0, 2.0, 0.0), Vec3::new(-3.0, 0.5, -20.0));
        let forward = matches!(
            line_of_sight(&wall, a, b, None, &params()),
            Sight::Blocked { .. }
        );
        let backward = matches!(
            line_of_sight(&wall, b, a, None, &params()),
            Sight::Blocked { .. }
        );
        assert!(forward && backward);
        let c = Vec3::new(1.0, 2.0, -5.0);
        assert_eq!(line_of_sight(&wall, a, c, None, &params()), Sight::Clear);
        assert_eq!(line_of_sight(&wall, c, a, None, &params()), Sight::Clear);
    }

    #[test]
    fn a_moving_collider_blocks_and_the_target_itself_does_not() {
        let mut w = QueryWorld::new();
        w.set_dynamic(vec![
            body(5, Vec3::new(0.0, 0.0, -10.0), Layers::ALL),
            body(9, Vec3::new(0.0, 0.0, -20.0), Layers::ALL),
        ]);
        let to = Vec3::new(0.0, 0.0, -20.0);
        // Aiming at body 9 through body 5.
        assert_eq!(
            line_of_sight(&w, Vec3::ZERO, to, Some(ColliderId(9)), &params()),
            Sight::Blocked {
                distance: 9.0,
                by: Some(ColliderId(5))
            }
        );
        // With nothing between, hitting the target is seeing it.
        let mut w2 = QueryWorld::new();
        w2.set_dynamic(vec![body(9, to, Layers::ALL)]);
        assert_eq!(
            line_of_sight(&w2, Vec3::ZERO, to, Some(ColliderId(9)), &params()),
            Sight::Clear
        );
        // Without naming the target, its surface blocks.
        assert!(matches!(
            line_of_sight(&w2, Vec3::ZERO, to, None, &params()),
            Sight::Blocked {
                by: Some(ColliderId(9)),
                ..
            }
        ));
        // The looker's own body is ignored through the filter.
        let own = QueryFilter {
            exclude: Some(ColliderId(5)),
            ..QueryFilter::default()
        };
        assert_eq!(
            line_of_sight(
                &w,
                Vec3::new(0.0, 0.0, -10.0),
                to,
                Some(ColliderId(9)),
                &SightParams {
                    filter: own,
                    ..params()
                }
            ),
            Sight::Clear
        );
    }

    #[test]
    fn only_the_layers_in_the_mask_are_opaque() {
        let glass = Layers::bit(3);
        let mut w = QueryWorld::new();
        w.set_dynamic(vec![body(5, Vec3::new(0.0, 0.0, -10.0), glass)]);
        let to = Vec3::new(0.0, 0.0, -20.0);
        assert!(matches!(
            line_of_sight(&w, Vec3::ZERO, to, None, &params()),
            Sight::Blocked { .. }
        ));
        let opaque = QueryFilter {
            mask: Layers(!(1 << 3)),
            ..QueryFilter::default()
        };
        assert_eq!(
            line_of_sight(
                &w,
                Vec3::ZERO,
                to,
                None,
                &SightParams {
                    filter: opaque,
                    ..params()
                }
            ),
            Sight::Clear,
            "glass does not block"
        );
    }

    #[test]
    fn range_the_margin_and_bad_input_are_handled() {
        let wall = wall_at(-10.0);
        // The wall is exactly at the target: the margin keeps it from blocking.
        assert_eq!(
            line_of_sight(
                &wall,
                Vec3::ZERO,
                Vec3::new(0.0, 0.0, -10.0),
                None,
                &params()
            ),
            Sight::Clear
        );
        assert!(matches!(
            line_of_sight(
                &wall,
                Vec3::ZERO,
                Vec3::new(0.0, 0.0, -10.0),
                None,
                &SightParams {
                    margin: 0.0,
                    ..params()
                }
            ),
            Sight::Blocked { .. }
        ));
        // Range.
        let empty = QueryWorld::new();
        let p = SightParams {
            max_distance: 10.0,
            ..params()
        };
        assert_eq!(
            line_of_sight(&empty, Vec3::ZERO, Vec3::new(0.0, 0.0, -10.0), None, &p),
            Sight::Clear,
            "exactly at the range"
        );
        assert_eq!(
            line_of_sight(&empty, Vec3::ZERO, Vec3::new(0.0, 0.0, -10.5), None, &p),
            Sight::OutOfRange
        );
        // Same point and a very short line.
        assert_eq!(
            line_of_sight(
                &wall,
                Vec3::new(1.0, 1.0, 1.0),
                Vec3::new(1.0, 1.0, 1.0),
                None,
                &params()
            ),
            Sight::Clear
        );
        assert_eq!(
            line_of_sight(
                &wall,
                Vec3::ZERO,
                Vec3::new(0.0, 0.0, -0.005),
                None,
                &params()
            ),
            Sight::Clear
        );
        // Bad input.
        for bad in [
            Vec3::new(f32::NAN, 0.0, 0.0),
            Vec3::new(0.0, f32::INFINITY, 0.0),
        ] {
            assert_eq!(
                line_of_sight(&empty, bad, Vec3::ZERO, None, &params()),
                Sight::Invalid
            );
            assert_eq!(
                line_of_sight(&empty, Vec3::ZERO, bad, None, &params()),
                Sight::Invalid
            );
        }
    }

    #[test]
    fn a_line_no_longer_than_the_margin_is_clear_even_from_inside_something() {
        let mut w = QueryWorld::new();
        w.set_dynamic(vec![body(5, Vec3::ZERO, Layers::ALL)]);
        let p = SightParams {
            margin: 0.5,
            ..params()
        };
        assert_eq!(
            line_of_sight(&w, Vec3::ZERO, Vec3::new(0.0, 0.0, -0.5), None, &p),
            Sight::Clear,
            "as long as the margin"
        );
        assert_eq!(
            line_of_sight(&w, Vec3::ZERO, Vec3::new(0.0, 0.0, -0.25), None, &p),
            Sight::Clear,
            "shorter than the margin"
        );
    }

    #[test]
    fn exposure_counts_the_visible_points_of_a_target_behind_low_cover() {
        // A wall at z = -10 that only reaches up to y = 1.0: a half-height cover.
        let v = [
            Vec3::new(-50.0, -5.0, -10.0),
            Vec3::new(50.0, -5.0, -10.0),
            Vec3::new(50.0, 1.0, -10.0),
            Vec3::new(-50.0, 1.0, -10.0),
        ];
        let mut w = QueryWorld::new();
        w.set_static(
            TriMesh::new(&v, &[0, 1, 2, 0, 2, 3]).unwrap(),
            vec![PhysMaterialId(0); 2],
        )
        .unwrap();
        let eye = Vec3::new(0.0, 1.8, 0.0);
        let feet = Vec3::new(0.0, 0.2, -20.0);
        let chest = Vec3::new(0.0, 1.2, -20.0);
        let head = Vec3::new(0.0, 1.8, -20.0);
        let e = exposure(&w, eye, &[head, chest, feet], None, &params());
        assert_eq!(
            (e.visible, e.total),
            (2, 3),
            "the feet are behind the cover"
        );
        assert!(e.any() && !e.full());
        let none = exposure(&w, eye, &[], None, &params());
        assert_eq!(
            (none.visible, none.total, none.any(), none.full()),
            (0, 0, false, false)
        );
        let all = exposure(
            &QueryWorld::new(),
            eye,
            &[head, chest, feet],
            None,
            &params(),
        );
        assert!(all.full());
    }

    #[test]
    fn the_cone_is_a_cosine_test() {
        let (from, fwd) = (Vec3::ZERO, Vec3::NEG_Z);
        let half = 0.5f32.sqrt(); // cos 45 degrees: a 90 degree field of view
        assert!(in_cone(from, fwd, half, Vec3::new(0.0, 0.0, -5.0)));
        assert!(
            in_cone(from, fwd, half, Vec3::new(3.0, 0.0, -5.0)),
            "inside 45 degrees"
        );
        assert!(
            !in_cone(from, fwd, half, Vec3::new(6.0, 0.0, -5.0)),
            "outside"
        );
        assert!(
            in_cone(from, fwd, half - 1e-4, Vec3::new(5.0, 0.0, -5.0)),
            "on the edge, with a little room"
        );
        assert!(
            !in_cone(from, fwd, half, Vec3::new(0.0, 0.0, 5.0)),
            "behind"
        );
        assert!(
            in_cone(from, fwd, -1.0, Vec3::new(0.0, 0.0, 5.0)),
            "a cosine of -1 sees everything"
        );
        assert!(
            !in_cone(from, fwd, 1.0, Vec3::new(0.5, 0.0, -5.0)),
            "a cosine of 1 sees only the line"
        );
        assert!(in_cone(from, fwd, 0.9, from), "the same point is inside");
        assert!(!in_cone(from, fwd, f32::NAN, Vec3::NEG_Z));
        assert!(!in_cone(
            from,
            Vec3::new(f32::NAN, 0.0, 0.0),
            0.0,
            Vec3::NEG_Z
        ));
    }

    #[test]
    fn a_looker_reports_what_it_saw_or_why_not() {
        let looker = Perception {
            min_cos: 0.5,
            sight: SightParams {
                max_distance: 30.0,
                ..params()
            },
        };
        let eye = Vec3::new(0.0, 1.8, 0.0);
        let fwd = Vec3::NEG_Z;
        let target = [Vec3::new(0.0, 1.0, -20.0), Vec3::new(0.0, 0.2, -20.0)];
        let empty = QueryWorld::new();
        assert_eq!(looker.look(&empty, eye, fwd, &target, None), Seen::Full);
        assert_eq!(
            looker.look(&empty, eye, Vec3::Z, &target, None),
            Seen::No(Unseen::OutsideCone)
        );
        let at_the_limit = [Vec3::new(0.0, 1.8, -30.0)];
        assert_eq!(
            looker.look(&empty, eye, fwd, &at_the_limit, None),
            Seen::Full,
            "exactly at the range is in range"
        );
        let far = [Vec3::new(0.0, 1.0, -50.0)];
        assert_eq!(
            looker.look(&empty, eye, fwd, &far, None),
            Seen::No(Unseen::OutOfRange)
        );
        assert_eq!(
            looker.look(&empty, eye, fwd, &[], None),
            Seen::No(Unseen::Invalid)
        );
        assert_eq!(
            looker.look(&empty, Vec3::new(f32::NAN, 0.0, 0.0), fwd, &target, None),
            Seen::No(Unseen::Invalid)
        );
        match looker.look(&wall_at(-10.0), eye, fwd, &target, None) {
            Seen::No(Unseen::Blocked(Sight::Blocked { by: None, .. })) => {}
            other => panic!("{other:?}"),
        }
        let partly = [Vec3::new(0.0, 1.0, -20.0), Vec3::new(0.0, 1.0, -5.0)];
        let mut w = QueryWorld::new();
        w.set_dynamic(vec![body(5, Vec3::new(0.0, 1.0, -12.0), Layers::ALL)]);
        let points = [partly[1], partly[0]];
        assert!(matches!(
            looker.look(&w, eye, fwd, &points, None),
            Seen::Partial(Exposure {
                visible: 1,
                total: 2
            })
        ));
    }

    // ---- sound ------------------------------------------------------------------------------------

    fn portal(a: u32, b: u32, length: f32, penalty: f32, open: bool) -> Portal {
        Portal {
            a: SectorId(a),
            b: SectorId(b),
            length,
            closed_penalty: penalty,
            open,
        }
    }

    #[test]
    fn sound_loses_loudness_by_distance_and_stops_when_it_runs_out() {
        let mut g = SoundGraph::new();
        g.add_portal(portal(0, 1, 10.0, 0.0, true)).unwrap();
        g.add_portal(portal(1, 2, 15.0, 0.0, true)).unwrap();
        g.add_portal(portal(2, 3, 20.0, 0.0, true)).unwrap();
        let reached = g.propagate(SectorId(0), 30.0);
        assert_eq!(
            reached.get(&SectorId(0)),
            Some(&30.0),
            "the source sector has all of it"
        );
        assert_eq!(reached.get(&SectorId(1)), Some(&20.0));
        assert_eq!(reached.get(&SectorId(2)), Some(&5.0));
        assert_eq!(
            reached.get(&SectorId(3)),
            None,
            "it does not reach the fourth"
        );
        assert_eq!(g.heard_at(SectorId(0), 30.0, SectorId(2)), Some(5.0));
        assert_eq!(g.heard_at(SectorId(0), 30.0, SectorId(3)), None);
        // Exactly enough to arrive with nothing left is still arriving.
        assert_eq!(g.heard_at(SectorId(0), 25.0, SectorId(2)), Some(0.0));
        assert_eq!(g.heard_at(SectorId(0), 24.9, SectorId(2)), None);
    }

    #[test]
    fn a_closed_portal_costs_its_penalty_and_opening_it_removes_it() {
        let mut g = SoundGraph::new();
        let door = g.add_portal(portal(0, 1, 10.0, 30.0, false)).unwrap();
        assert_eq!(g.heard_at(SectorId(0), 50.0, SectorId(1)), Some(10.0));
        assert_eq!(
            g.heard_at(SectorId(0), 39.0, SectorId(1)),
            None,
            "a closed door muffles it away"
        );
        assert!(g.set_open(door, true));
        assert_eq!(g.heard_at(SectorId(0), 50.0, SectorId(1)), Some(40.0));
        assert!(!g.set_open(PortalId(99), true), "no such portal");
    }

    #[test]
    fn the_shortest_way_is_used_and_cycles_are_harmless() {
        let mut g = SoundGraph::new();
        g.add_portal(portal(0, 1, 10.0, 0.0, true)).unwrap();
        g.add_portal(portal(1, 3, 10.0, 0.0, true)).unwrap(); // 0 - 1 - 3: 20
        g.add_portal(portal(0, 2, 5.0, 0.0, true)).unwrap();
        g.add_portal(portal(2, 3, 5.0, 0.0, true)).unwrap(); // 0 - 2 - 3: 10
        g.add_portal(portal(3, 0, 1.0, 100.0, false)).unwrap(); // a closed shortcut back: 101
        assert_eq!(g.heard_at(SectorId(0), 100.0, SectorId(3)), Some(90.0));
        // The long way is found first and the short one later: 0 - 1 - 3 is 101, 0 - 2 - 3 is 20.
        let mut late = SoundGraph::new();
        late.add_portal(portal(0, 1, 1.0, 0.0, true)).unwrap();
        late.add_portal(portal(1, 3, 100.0, 0.0, true)).unwrap();
        late.add_portal(portal(0, 2, 10.0, 0.0, true)).unwrap();
        late.add_portal(portal(2, 3, 10.0, 0.0, true)).unwrap();
        assert_eq!(late.heard_at(SectorId(0), 200.0, SectorId(3)), Some(180.0));
        // A door that is only on the long way does not change the short one.
        let mut h = SoundGraph::new();
        h.add_portal(portal(0, 1, 10.0, 0.0, true)).unwrap();
        h.add_portal(portal(0, 1, 10.0, 0.0, false)).unwrap();
        h.add_portal(portal(0, 1, 3.0, 50.0, false)).unwrap();
        assert_eq!(
            h.heard_at(SectorId(0), 100.0, SectorId(1)),
            Some(90.0),
            "the open portal beats the closed ones"
        );
    }

    #[test]
    fn a_sector_with_no_portals_is_alone_and_bad_loudness_reaches_nowhere() {
        let g = SoundGraph::new();
        assert_eq!(g.propagate(SectorId(7), 10.0).len(), 1);
        assert_eq!(g.heard_at(SectorId(7), 10.0, SectorId(8)), None);
        let mut h = SoundGraph::new();
        h.add_portal(portal(0, 1, 1.0, 0.0, true)).unwrap();
        for bad in [-1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert!(h.propagate(SectorId(0), bad).is_empty(), "{bad}");
        }
        assert_eq!(
            h.propagate(SectorId(0), 0.0).len(),
            1,
            "a silent sound stays where it is"
        );
        h.add_portal(portal(0, 2, 0.0, 0.0, true)).unwrap();
        assert!(
            h.propagate(SectorId(0), 0.0).contains_key(&SectorId(2)),
            "a zero-length portal costs nothing"
        );
    }

    #[test]
    fn bad_portals_are_refused() {
        let mut g = SoundGraph::new();
        for (len, pen) in [
            (-1.0, 0.0),
            (0.0, -0.5),
            (f32::NAN, 0.0),
            (1.0, f32::INFINITY),
        ] {
            assert_eq!(
                g.add_portal(portal(0, 1, len, pen, true)),
                Err(GraphError::BadCost),
                "{len} {pen}"
            );
        }
        assert_eq!(g.portal_count(), 0);
        // A portal from a sector to itself is allowed and harmless.
        assert!(g.add_portal(portal(4, 4, 1.0, 0.0, true)).is_ok());
        assert_eq!(g.propagate(SectorId(4), 5.0).len(), 1);
    }

    #[test]
    fn a_long_chain_and_the_order_of_portals_do_not_change_the_answer() {
        let build = |reverse: bool| {
            let mut g = SoundGraph::new();
            let mut order: Vec<u32> = (0..2000).collect();
            if reverse {
                order.reverse();
            }
            for i in order {
                g.add_portal(portal(i, i + 1, 1.0, 0.0, true)).unwrap();
            }
            g
        };
        let (a, b) = (build(false), build(true));
        let (ra, rb) = (
            a.propagate(SectorId(0), 1500.0),
            b.propagate(SectorId(0), 1500.0),
        );
        assert_eq!(ra, rb);
        assert_eq!(ra.len(), 1501);
        assert_eq!(ra[&SectorId(1500)], 0.0);
        assert_eq!(
            ra.keys().copied().collect::<Vec<_>>(),
            (0..=1500).map(SectorId).collect::<Vec<_>>(),
            "sorted by sector"
        );
    }
}
