//! The physics query contract (roadmap T07-004, `docs/rfcs/0007-physics-query-api.md`): how
//! gameplay, the character controller, hit scanning and AI ask the world "what is there?".
//!
//! Three questions: [`SpatialQuery::raycast`] (a ray), [`SpatialQuery::shapecast`] (move a shape
//! along a line and find where it first touches something) and [`SpatialQuery::overlap`] (what does a
//! shape at a place touch). Every answer is a [`Hit`] or an [`Overlap`] that says **where**, with
//! what **normal**, on what **material** and which **collider** (an entity's body; `None` is the
//! static world). A [`QueryFilter`] chooses layers, one collider to ignore and whether to look at
//! the static world, the moving colliders, or both.
//!
//! [`QueryWorld`] is the reference implementation: a static triangle mesh plus the moving colliders
//! of this tick, rebuilt each tick ([`QueryWorld::set_dynamic`]). It does not support every
//! shape against every other yet; what it cannot do it **refuses** with
//! [`QueryError::Unsupported`] rather than answering wrongly (the matrix is in
//! `docs/PHYSICS_QUERIES.md`, the gaps are task T07-004b).

use std::collections::BTreeSet;
use std::fmt;

use crate::geometry::{Aabb, Capsule, Ray, Sphere};
use crate::spatial::{Bvh, Layers, TriMesh};
use ss2_core::math::Vec3;

/// The identity of a moving collider (an entity's body); assigned by the simulation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ColliderId(pub u64);

/// A physical material: what a surface is made of (grass, metal), for footstep sounds, impact
/// effects and friction. The table that says what each id means is T07-009.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysMaterialId(pub u16);

/// What a query looks at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueryFilter {
    /// Only things on a layer of this mask are considered.
    pub mask: Layers,
    /// A collider to ignore (a shooter's own body).
    pub exclude: Option<ColliderId>,
    /// Look at the static world.
    pub statics: bool,
    /// Look at the moving colliders.
    pub dynamics: bool,
}

impl Default for QueryFilter {
    /// Everything: all layers, nothing excluded, static and moving.
    fn default() -> Self {
        Self {
            mask: Layers::ALL,
            exclude: None,
            statics: true,
            dynamics: true,
        }
    }
}

/// A shape to cast or to test for overlap.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    /// A sphere.
    Sphere {
        /// The radius.
        radius: f32,
    },
    /// A capsule standing along the Y axis: a cylinder of length `2 * half_height` with a
    /// hemisphere at each end (the character controller's shape).
    Capsule {
        /// Half the length of the straight part.
        half_height: f32,
        /// The radius.
        radius: f32,
    },
    /// An axis-aligned box.
    Box {
        /// Half the size along each axis.
        half_extents: Vec3,
    },
}

impl Shape {
    fn validate(&self) -> Result<(), QueryError> {
        let ok = match *self {
            Self::Sphere { radius } => radius.is_finite() && radius >= 0.0,
            Self::Capsule {
                half_height,
                radius,
            } => {
                half_height.is_finite() && half_height >= 0.0 && radius.is_finite() && radius >= 0.0
            }
            Self::Box { half_extents } => {
                half_extents.is_finite() && half_extents.cmpge(Vec3::ZERO).all()
            }
        };
        if ok {
            Ok(())
        } else {
            Err(QueryError::BadInput(
                "a shape has a negative or non-finite size",
            ))
        }
    }

    /// The box around the shape placed at `at`.
    fn bounds_at(&self, at: Vec3) -> Aabb {
        match *self {
            Self::Sphere { radius } => Aabb::from_center_half(at, Vec3::splat(radius)),
            Self::Capsule {
                half_height,
                radius,
            } => Aabb::from_center_half(at, Vec3::new(radius, half_height + radius, radius)),
            Self::Box { half_extents } => Aabb::from_center_half(at, half_extents),
        }
    }
}

fn capsule_at(at: Vec3, half_height: f32, radius: f32) -> Capsule {
    Capsule::new(
        at - Vec3::new(0.0, half_height, 0.0),
        at + Vec3::new(0.0, half_height, 0.0),
        radius,
    )
}

/// What a ray or a cast met.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    /// How far along the ray, or how far the shape travelled (not the fraction), to the hit.
    pub distance: f32,
    /// The contact point on the thing that was hit (for a box cast, the box's centre at the hit).
    pub point: Vec3,
    /// The surface normal, unit, pointing away from the thing that was hit.
    pub normal: Vec3,
    /// The material of the surface.
    pub material: PhysMaterialId,
    /// The collider that was hit; `None` is the static world.
    pub collider: Option<ColliderId>,
    /// The ray started inside, or the shape already overlapped the thing: `distance` is 0 and the
    /// normal is the way out.
    pub started_overlapping: bool,
}

/// One thing a shape overlaps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Overlap {
    /// The collider; `None` is the static world (one entry per distinct material).
    pub collider: Option<ColliderId>,
    /// The material.
    pub material: PhysMaterialId,
}

/// Why a query was not answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueryError {
    /// This combination of shapes is not implemented yet; the answer would be wrong, so there is none.
    Unsupported(&'static str),
    /// A size or position is negative, NaN or infinite.
    BadInput(&'static str),
}

impl fmt::Display for QueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(what) => write!(f, "not supported: {what}"),
            Self::BadInput(what) => write!(f, "bad input: {what}"),
        }
    }
}

impl std::error::Error for QueryError {}

/// The questions the rest of the engine asks of the physical world.
pub trait SpatialQuery {
    /// The nearest thing the ray meets within `max_distance`.
    fn raycast(&self, ray: &Ray, max_distance: f32, filter: &QueryFilter) -> Option<Hit>;

    /// Move `shape` from `from` by `delta` and find the first thing it touches.
    fn shapecast(
        &self,
        shape: &Shape,
        from: Vec3,
        delta: Vec3,
        filter: &QueryFilter,
    ) -> Result<Option<Hit>, QueryError>;

    /// **Append** what `shape` placed at `at` touches to `out`: the static world's materials
    /// first (sorted), then the moving colliders by id.
    fn overlap(
        &self,
        shape: &Shape,
        at: Vec3,
        filter: &QueryFilter,
        out: &mut Vec<Overlap>,
    ) -> Result<(), QueryError>;
}

/// A moving collider for one tick.
///
/// ```
/// use ss2_core::math::Vec3;
/// use ss2_physics::geometry::Ray;
/// use ss2_physics::query::{ColliderId, DynamicCollider, PhysMaterialId, QueryFilter, QueryWorld, Shape, SpatialQuery};
/// use ss2_physics::spatial::Layers;
///
/// let mut world = QueryWorld::new();
/// world.set_dynamic(vec![DynamicCollider {
///     id: ColliderId(7),
///     layers: Layers::ALL,
///     shape: Shape::Sphere { radius: 1.0 },
///     position: Vec3::new(0.0, 0.0, -10.0),
///     material: PhysMaterialId(3),
/// }]);
/// let ray = Ray::new(Vec3::ZERO, Vec3::NEG_Z).unwrap();
/// let hit = world.raycast(&ray, 100.0, &QueryFilter::default()).unwrap();
/// assert_eq!((hit.distance, hit.collider), (9.0, Some(ColliderId(7))));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DynamicCollider {
    /// Its identity.
    pub id: ColliderId,
    /// The layers it is on.
    pub layers: Layers,
    /// Its shape.
    pub shape: Shape,
    /// Where it is (the centre of the shape).
    pub position: Vec3,
    /// What it is made of.
    pub material: PhysMaterialId,
}

/// The reference implementation of [`SpatialQuery`]: a static triangle mesh and the moving
/// colliders of this tick.
pub struct QueryWorld {
    statics: Option<TriMesh>,
    materials: Vec<PhysMaterialId>,
    dynamics: Vec<DynamicCollider>,
    tree: Bvh<u32>,
}

impl Default for QueryWorld {
    fn default() -> Self {
        Self::new()
    }
}

struct Candidate {
    distance: f32,
    point: Vec3,
    normal: Vec3,
    started_overlapping: bool,
}

impl QueryWorld {
    /// An empty world.
    pub fn new() -> Self {
        Self {
            statics: None,
            materials: Vec::new(),
            dynamics: Vec::new(),
            tree: Bvh::build(Vec::new()),
        }
    }

    /// Set the static world: a mesh and **one material per triangle**.
    pub fn set_static(
        &mut self,
        mesh: TriMesh,
        materials: Vec<PhysMaterialId>,
    ) -> Result<(), QueryError> {
        if materials.len() != mesh.triangle_count() {
            return Err(QueryError::BadInput("one material per triangle is needed"));
        }
        self.statics = Some(mesh);
        self.materials = materials;
        Ok(())
    }

    /// Replace the moving colliders (call it each tick). They are kept sorted by id, so the
    /// order they are given in never changes an answer.
    pub fn set_dynamic(&mut self, mut colliders: Vec<DynamicCollider>) {
        colliders.sort_by_key(|c| c.id);
        let items = colliders
            .iter()
            .enumerate()
            .map(|(i, c)| (c.shape.bounds_at(c.position), c.layers, i as u32))
            .collect();
        self.tree = Bvh::build(items);
        self.dynamics = colliders;
    }

    fn has_static_triangles(&self) -> bool {
        self.statics
            .as_ref()
            .is_some_and(|m| m.triangle_count() > 0)
    }

    fn ray_candidate(c: &DynamicCollider, ray: &Ray, limit: f32) -> Option<Candidate> {
        let hit = match c.shape {
            Shape::Sphere { radius } => Sphere::new(c.position, radius).raycast(ray, limit),
            Shape::Capsule {
                half_height,
                radius,
            } => capsule_at(c.position, half_height, radius).raycast(ray, limit),
            Shape::Box { half_extents } => {
                Aabb::from_center_half(c.position, half_extents).raycast(ray, limit)
            }
        }?;
        Some(Candidate {
            distance: hit.t,
            point: hit.point,
            normal: hit.normal,
            started_overlapping: hit.inside,
        })
    }

    /// A moving sphere against one collider.
    fn sphere_cast_candidate(
        c: &DynamicCollider,
        from: Vec3,
        radius: f32,
        delta: Vec3,
    ) -> Option<Candidate> {
        let len = delta.length();
        let mine = Sphere::new(from, radius);
        let swept = match c.shape {
            Shape::Sphere { radius: other } => {
                mine.sweep_sphere(delta, &Sphere::new(c.position, other))
            }
            Shape::Box { half_extents } => {
                mine.sweep_aabb(delta, &Aabb::from_center_half(c.position, half_extents))
            }
            Shape::Capsule {
                half_height,
                radius: other,
            } => {
                let capsule = capsule_at(c.position, half_height, other);
                if capsule.signed_distance(from) < radius {
                    let nearest = capsule.a + {
                        let ab = capsule.b - capsule.a;
                        let len2 = ab.length_squared();
                        if len2 <= 1e-12 {
                            Vec3::ZERO
                        } else {
                            ab * ((from - capsule.a).dot(ab) / len2).clamp(0.0, 1.0)
                        }
                    };
                    let normal = (from - nearest).try_normalize().unwrap_or(Vec3::Y);
                    return Some(Candidate {
                        distance: 0.0,
                        point: from - normal * radius,
                        normal,
                        started_overlapping: true,
                    });
                }
                let ray = Ray::new(from, delta).filter(|_| len > 1e-12)?;
                let hit = Capsule::new(capsule.a, capsule.b, other + radius).raycast(&ray, len)?;
                return Some(Candidate {
                    distance: hit.t,
                    point: hit.point - hit.normal * radius,
                    normal: hit.normal,
                    started_overlapping: false,
                });
            }
        }?;
        Some(Candidate {
            distance: swept.t * len,
            point: swept.position - swept.normal * radius,
            normal: swept.normal,
            started_overlapping: swept.overlapping_at_start,
        })
    }

    fn box_cast_candidate(
        c: &DynamicCollider,
        from: Vec3,
        half: Vec3,
        delta: Vec3,
    ) -> Option<Candidate> {
        let Shape::Box { half_extents } = c.shape else {
            return None;
        };
        let swept = Aabb::from_center_half(from, half)
            .sweep_aabb(delta, &Aabb::from_center_half(c.position, half_extents))?;
        Some(Candidate {
            distance: swept.t * delta.length(),
            point: swept.position,
            normal: swept.normal,
            started_overlapping: swept.overlapping_at_start,
        })
    }

    fn overlaps_pair(probe: &Shape, at: Vec3, c: &DynamicCollider) -> bool {
        match (*probe, c.shape) {
            (Shape::Sphere { radius }, Shape::Sphere { radius: r2 }) => {
                Sphere::new(at, radius).overlaps_sphere(&Sphere::new(c.position, r2))
            }
            (Shape::Sphere { radius }, Shape::Box { half_extents }) => Sphere::new(at, radius)
                .overlaps_aabb(&Aabb::from_center_half(c.position, half_extents)),
            (
                Shape::Sphere { radius },
                Shape::Capsule {
                    half_height,
                    radius: r2,
                },
            ) => capsule_at(c.position, half_height, r2).overlaps_sphere(&Sphere::new(at, radius)),
            (Shape::Box { half_extents }, Shape::Box { half_extents: h2 }) => {
                Aabb::from_center_half(at, half_extents)
                    .overlaps(&Aabb::from_center_half(c.position, h2))
            }
            (Shape::Box { half_extents }, Shape::Sphere { radius }) => {
                Sphere::new(c.position, radius)
                    .overlaps_aabb(&Aabb::from_center_half(at, half_extents))
            }
            (
                Shape::Box { half_extents },
                Shape::Capsule {
                    half_height,
                    radius,
                },
            ) => capsule_at(c.position, half_height, radius)
                .overlaps_aabb(&Aabb::from_center_half(at, half_extents)),
            (
                Shape::Capsule {
                    half_height,
                    radius,
                },
                Shape::Sphere { radius: r2 },
            ) => capsule_at(at, half_height, radius).overlaps_sphere(&Sphere::new(c.position, r2)),
            (
                Shape::Capsule {
                    half_height,
                    radius,
                },
                Shape::Capsule {
                    half_height: h2,
                    radius: r2,
                },
            ) => capsule_at(at, half_height, radius)
                .overlaps_capsule(&capsule_at(c.position, h2, r2)),
            (
                Shape::Capsule {
                    half_height,
                    radius,
                },
                Shape::Box { half_extents },
            ) => capsule_at(at, half_height, radius)
                .overlaps_aabb(&Aabb::from_center_half(c.position, half_extents)),
        }
    }
}

impl SpatialQuery for QueryWorld {
    fn raycast(&self, ray: &Ray, max_distance: f32, filter: &QueryFilter) -> Option<Hit> {
        if !max_distance.is_finite() || max_distance < 0.0 {
            return None;
        }
        let mut best: Option<Hit> = None;
        if filter.statics
            && let Some(mesh) = &self.statics
            && let Some(h) = mesh.raycast(ray, max_distance, filter.mask)
        {
            best = Some(Hit {
                distance: h.t,
                point: h.point,
                normal: h.normal,
                material: self.materials[h.triangle as usize],
                collider: None,
                started_overlapping: false,
            });
        }
        if filter.dynamics {
            let limit = best.map_or(max_distance, |h| h.distance);
            let mut found: Option<(Candidate, usize)> = None;
            self.tree
                .query_ray(ray, limit, filter.mask, 0.0, &mut |index, best_t| {
                    let c = &self.dynamics[*index as usize];
                    if filter.exclude == Some(c.id) {
                        return None;
                    }
                    let cand = Self::ray_candidate(c, ray, best_t)?;
                    let distance = cand.distance;
                    // Keep `found` in step with the tree's rule (a hit is taken only if it is the
                    // first or nearer than the best): a tie must not replace the winner.
                    if found.is_none() || distance < best_t {
                        found = Some((cand, *index as usize));
                    }
                    Some(distance)
                });
            if let Some((cand, index)) = found
                && best.is_none_or(|b| cand.distance < b.distance)
            {
                let c = &self.dynamics[index];
                best = Some(Hit {
                    distance: cand.distance,
                    point: cand.point,
                    normal: cand.normal,
                    material: c.material,
                    collider: Some(c.id),
                    started_overlapping: cand.started_overlapping,
                });
            }
        }
        best
    }

    fn shapecast(
        &self,
        shape: &Shape,
        from: Vec3,
        delta: Vec3,
        filter: &QueryFilter,
    ) -> Result<Option<Hit>, QueryError> {
        shape.validate()?;
        if !from.is_finite() || !delta.is_finite() {
            return Err(QueryError::BadInput(
                "a position or a movement is not finite",
            ));
        }
        let (radius, half) = match *shape {
            Shape::Sphere { radius } => (radius, Vec3::splat(radius)),
            Shape::Box { half_extents } => {
                if filter.statics && self.has_static_triangles() {
                    return Err(QueryError::Unsupported(
                        "a box cast against the static world (T07-004b)",
                    ));
                }
                if filter.dynamics
                    && self.dynamics.iter().any(|c| {
                        filter.mask.intersects(c.layers) && !matches!(c.shape, Shape::Box { .. })
                    })
                {
                    return Err(QueryError::Unsupported(
                        "a box cast against a sphere or capsule collider (T07-004b)",
                    ));
                }
                (0.0, half_extents)
            }
            Shape::Capsule { .. } => {
                return Err(QueryError::Unsupported("capsule casts (T07-004b)"));
            }
        };
        let len = delta.length();
        let mut best: Option<Hit> = None;
        if filter.statics
            && let (Some(mesh), Shape::Sphere { .. }) = (&self.statics, shape)
            && let Some((swept, triangle)) = mesh.sweep_sphere(from, radius, delta, filter.mask)
        {
            best = Some(Hit {
                distance: swept.t * len,
                point: swept.position - swept.normal * radius,
                normal: swept.normal,
                material: self.materials[triangle as usize],
                collider: None,
                started_overlapping: swept.overlapping_at_start,
            });
        }
        if filter.dynamics {
            let region = Aabb::from_center_half(from, half)
                .union(&Aabb::from_center_half(from + delta, half));
            let mut nearest: Option<(Candidate, usize)> = None;
            self.tree.query_aabb(&region, filter.mask, &mut |index| {
                let c = &self.dynamics[*index as usize];
                if filter.exclude == Some(c.id) {
                    return;
                }
                let cand = match shape {
                    Shape::Sphere { .. } => Self::sphere_cast_candidate(c, from, radius, delta),
                    _ => Self::box_cast_candidate(c, from, half, delta),
                };
                if let Some(cand) = cand
                    && nearest.as_ref().is_none_or(|(n, i)| {
                        cand.distance < n.distance
                            || (cand.distance == n.distance && c.id < self.dynamics[*i].id)
                    })
                {
                    nearest = Some((cand, *index as usize));
                }
            });
            if let Some((cand, index)) = nearest
                && best.is_none_or(|b| cand.distance < b.distance)
            {
                let c = &self.dynamics[index];
                best = Some(Hit {
                    distance: cand.distance,
                    point: cand.point,
                    normal: cand.normal,
                    material: c.material,
                    collider: Some(c.id),
                    started_overlapping: cand.started_overlapping,
                });
            }
        }
        Ok(best)
    }

    fn overlap(
        &self,
        shape: &Shape,
        at: Vec3,
        filter: &QueryFilter,
        out: &mut Vec<Overlap>,
    ) -> Result<(), QueryError> {
        shape.validate()?;
        if !at.is_finite() {
            return Err(QueryError::BadInput("a position is not finite"));
        }
        if filter.statics && self.has_static_triangles() {
            let Shape::Sphere { radius } = *shape else {
                return Err(QueryError::Unsupported(
                    "a box or capsule overlap with the static world (T07-004b)",
                ));
            };
            let mut materials = BTreeSet::new();
            if let Some(mesh) = &self.statics {
                mesh.query_sphere(at, radius, filter.mask, &mut |triangle| {
                    materials.insert(self.materials[triangle as usize]);
                });
            }
            out.extend(materials.into_iter().map(|material| Overlap {
                collider: None,
                material,
            }));
        }
        if filter.dynamics {
            let mut touching: Vec<(ColliderId, PhysMaterialId)> = Vec::new();
            self.tree
                .query_aabb(&shape.bounds_at(at), filter.mask, &mut |index| {
                    let c = &self.dynamics[*index as usize];
                    if filter.exclude != Some(c.id) && Self::overlaps_pair(shape, at, c) {
                        touching.push((c.id, c.material));
                    }
                });
            touching.sort_by_key(|(id, _)| *id);
            out.extend(touching.into_iter().map(|(id, material)| Overlap {
                collider: Some(id),
                material,
            }));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Aabb, Capsule, Ray, Sphere};
    use crate::spatial::{Layers, TriMesh};
    use ss2_core::math::Vec3;

    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> f32 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((self.0 >> 33) as f32) / (1_u64 << 31) as f32
        }
        fn range(&mut self, lo: f32, hi: f32) -> f32 {
            lo + (hi - lo) * self.next()
        }
        fn vec(&mut self, extent: f32) -> Vec3 {
            Vec3::new(
                self.range(-extent, extent),
                self.range(-extent, extent),
                self.range(-extent, extent),
            )
        }
    }

    const GRASS: PhysMaterialId = PhysMaterialId(1);
    const METAL: PhysMaterialId = PhysMaterialId(2);

    /// A 10 by 10 floor at y = 0 made of two triangles: the left half grass, the right half metal.
    fn floor_world() -> QueryWorld {
        let v = [
            Vec3::new(-5.0, 0.0, -5.0),
            Vec3::new(5.0, 0.0, -5.0),
            Vec3::new(5.0, 0.0, 5.0),
            Vec3::new(-5.0, 0.0, 5.0),
        ];
        let mesh = TriMesh::new(&v, &[0, 2, 1, 0, 3, 2]).unwrap();
        let mut w = QueryWorld::new();
        w.set_static(mesh, vec![GRASS, METAL]).unwrap();
        w
    }

    fn collider(id: u64, shape: Shape, position: Vec3, layers: Layers) -> DynamicCollider {
        DynamicCollider {
            id: ColliderId(id),
            layers,
            shape,
            position,
            material: METAL,
        }
    }

    fn down(from: Vec3) -> Ray {
        Ray::new(from, Vec3::NEG_Y).unwrap()
    }

    #[test]
    fn a_ray_hits_the_floor_with_its_material_and_no_collider() {
        let w = floor_world();
        let hit = w
            .raycast(
                &down(Vec3::new(1.0, 3.0, 1.0)),
                100.0,
                &QueryFilter::default(),
            )
            .unwrap();
        assert!((hit.distance - 3.0).abs() < 1e-5 && hit.normal == Vec3::Y);
        assert_eq!(hit.collider, None, "the static world");
        assert!(hit.material == GRASS || hit.material == METAL);
        assert!(
            w.raycast(
                &down(Vec3::new(1.0, 3.0, 1.0)),
                2.0,
                &QueryFilter::default()
            )
            .is_none(),
            "too short"
        );
        assert!(
            w.raycast(
                &down(Vec3::new(50.0, 3.0, 1.0)),
                100.0,
                &QueryFilter::default()
            )
            .is_none(),
            "off the floor"
        );
        // The triangle that is hit decides the material.
        let a = w
            .raycast(
                &down(Vec3::new(-3.0, 1.0, 3.0)),
                10.0,
                &QueryFilter::default(),
            )
            .unwrap()
            .material;
        let b = w
            .raycast(
                &down(Vec3::new(3.0, 1.0, -3.0)),
                10.0,
                &QueryFilter::default(),
            )
            .unwrap()
            .material;
        assert_ne!(a, b, "the two halves have different materials");
    }

    #[test]
    fn moving_colliders_are_hit_with_their_id_and_the_nearest_thing_wins() {
        let mut w = floor_world();
        w.set_dynamic(vec![
            collider(
                7,
                Shape::Sphere { radius: 1.0 },
                Vec3::new(0.0, 2.0, 0.0),
                Layers::bit(1),
            ),
            collider(
                9,
                Shape::Box {
                    half_extents: Vec3::splat(0.5),
                },
                Vec3::new(0.0, 6.0, 0.0),
                Layers::bit(1),
            ),
            collider(
                4,
                Shape::Capsule {
                    half_height: 1.0,
                    radius: 0.5,
                },
                Vec3::new(3.0, 1.5, 0.0),
                Layers::bit(1),
            ),
        ]);
        let f = QueryFilter::default();
        let from_above = w
            .raycast(&down(Vec3::new(0.0, 10.0, 0.0)), 100.0, &f)
            .unwrap();
        assert_eq!(
            from_above.collider,
            Some(ColliderId(9)),
            "the box is nearer than the sphere and the floor"
        );
        assert!((from_above.distance - 3.5).abs() < 1e-4 && from_above.normal == Vec3::Y);
        let capsule = w
            .raycast(
                &Ray::new(Vec3::new(8.0, 1.5, 0.0), Vec3::NEG_X).unwrap(),
                100.0,
                &f,
            )
            .unwrap();
        assert_eq!(capsule.collider, Some(ColliderId(4)));
        assert!((capsule.distance - 4.5).abs() < 1e-4, "{capsule:?}");
        let sphere = w
            .raycast(
                &Ray::new(Vec3::new(-5.0, 2.0, 0.0), Vec3::X).unwrap(),
                100.0,
                &f,
            )
            .unwrap();
        assert_eq!(sphere.collider, Some(ColliderId(7)));
        assert!((sphere.distance - 4.0).abs() < 1e-4);
    }

    #[test]
    fn filters_choose_layers_one_excluded_collider_and_static_or_dynamic() {
        let mut w = floor_world();
        w.set_dynamic(vec![collider(
            7,
            Shape::Sphere { radius: 1.0 },
            Vec3::new(0.0, 2.0, 0.0),
            Layers::bit(1),
        )]);
        let ray = down(Vec3::new(0.0, 10.0, 0.0));
        let all = QueryFilter::default();
        assert_eq!(
            w.raycast(&ray, 100.0, &all).unwrap().collider,
            Some(ColliderId(7))
        );
        let other_layer = QueryFilter {
            mask: Layers::bit(2),
            ..all
        };
        assert_eq!(
            w.raycast(&ray, 100.0, &other_layer).unwrap().collider,
            None,
            "the sphere is on another layer; the floor is on all"
        );
        let skip_it = QueryFilter {
            exclude: Some(ColliderId(7)),
            ..all
        };
        assert_eq!(
            w.raycast(&ray, 100.0, &skip_it).unwrap().collider,
            None,
            "ignore the shooter's own body"
        );
        let only_dynamic = QueryFilter {
            statics: false,
            ..all
        };
        assert_eq!(
            w.raycast(&ray, 100.0, &only_dynamic).unwrap().collider,
            Some(ColliderId(7))
        );
        let only_static = QueryFilter {
            dynamics: false,
            ..all
        };
        assert_eq!(w.raycast(&ray, 100.0, &only_static).unwrap().collider, None);
        let nothing = QueryFilter {
            statics: false,
            dynamics: false,
            ..all
        };
        assert!(w.raycast(&ray, 100.0, &nothing).is_none());
    }

    #[test]
    fn a_sphere_cast_stops_on_the_floor_a_radius_above_it() {
        let w = floor_world();
        let hit = w
            .shapecast(
                &Shape::Sphere { radius: 0.5 },
                Vec3::new(1.0, 4.0, 1.0),
                Vec3::new(0.0, -10.0, 0.0),
                &QueryFilter::default(),
            )
            .unwrap()
            .unwrap();
        assert!((hit.distance - 3.5).abs() < 1e-4, "{hit:?}");
        assert_eq!(hit.normal, Vec3::Y);
        assert!(
            (hit.point - Vec3::new(1.0, 0.0, 1.0)).length() < 1e-4,
            "the contact point is on the floor"
        );
        assert!(!hit.started_overlapping);
        let miss = w
            .shapecast(
                &Shape::Sphere { radius: 0.5 },
                Vec3::new(1.0, 4.0, 1.0),
                Vec3::new(0.0, 1.0, 0.0),
                &QueryFilter::default(),
            )
            .unwrap();
        assert!(miss.is_none(), "moving away");
        let inside = w
            .shapecast(
                &Shape::Sphere { radius: 0.5 },
                Vec3::new(1.0, 0.2, 1.0),
                Vec3::new(0.0, 1.0, 0.0),
                &QueryFilter::default(),
            )
            .unwrap()
            .unwrap();
        assert!(
            inside.started_overlapping && inside.distance == 0.0,
            "already in the floor: {inside:?}"
        );
    }

    #[test]
    fn a_cast_distance_is_the_travel_not_the_fraction() {
        let mut w = QueryWorld::new();
        w.set_dynamic(vec![collider(
            1,
            Shape::Box {
                half_extents: Vec3::splat(1.0),
            },
            Vec3::new(10.0, 0.0, 0.0),
            Layers::ALL,
        )]);
        let hit = w
            .shapecast(
                &Shape::Sphere { radius: 1.0 },
                Vec3::ZERO,
                Vec3::new(40.0, 0.0, 0.0),
                &QueryFilter::default(),
            )
            .unwrap()
            .unwrap();
        assert!((hit.distance - 8.0).abs() < 1e-4, "{hit:?}");
        assert_eq!(hit.normal, Vec3::NEG_X);
        let boxed = w
            .shapecast(
                &Shape::Box {
                    half_extents: Vec3::splat(1.0),
                },
                Vec3::ZERO,
                Vec3::new(40.0, 0.0, 0.0),
                &QueryFilter::default(),
            )
            .unwrap()
            .unwrap();
        assert!(
            (boxed.distance - 8.0).abs() < 1e-4 && boxed.collider == Some(ColliderId(1)),
            "{boxed:?}"
        );
    }

    /// The signed distance to a moving collider (the reference for the sampled checks).
    fn sdf(c: &DynamicCollider, p: Vec3) -> f32 {
        match c.shape {
            Shape::Sphere { radius } => Sphere::new(c.position, radius).signed_distance(p),
            Shape::Box { half_extents } => {
                Aabb::from_center_half(c.position, half_extents).signed_distance(p)
            }
            Shape::Capsule {
                half_height,
                radius,
            } => Capsule::new(
                c.position - Vec3::new(0.0, half_height, 0.0),
                c.position + Vec3::new(0.0, half_height, 0.0),
                radius,
            )
            .signed_distance(p),
        }
    }

    fn random_world(rng: &mut Lcg, count: u64) -> (QueryWorld, Vec<DynamicCollider>) {
        let colliders: Vec<DynamicCollider> = (0..count)
            .map(|i| {
                let shape = match i % 3 {
                    0 => Shape::Sphere {
                        radius: rng.range(0.3, 1.2),
                    },
                    1 => Shape::Box {
                        half_extents: Vec3::new(
                            rng.range(0.3, 1.2),
                            rng.range(0.3, 1.2),
                            rng.range(0.3, 1.2),
                        ),
                    },
                    _ => Shape::Capsule {
                        half_height: rng.range(0.2, 1.0),
                        radius: rng.range(0.3, 0.8),
                    },
                };
                collider(i, shape, rng.vec(12.0), Layers::ALL)
            })
            .collect();
        let mut w = QueryWorld::new();
        w.set_dynamic(colliders.clone());
        (w, colliders)
    }

    #[test]
    fn sphere_casts_against_moving_colliders_match_sampled_distance() {
        let mut rng = Lcg(3);
        let (w, colliders) = random_world(&mut rng, 60);
        let (mut hits, mut misses) = (0, 0);
        for _ in 0..800 {
            let radius = rng.range(0.1, 0.8);
            let from = rng.vec(14.0);
            let delta = rng.vec(12.0);
            let len = delta.length();
            let nearest = |p: Vec3| colliders.iter().map(|c| sdf(c, p)).fold(f32::MAX, f32::min);
            match w
                .shapecast(
                    &Shape::Sphere { radius },
                    from,
                    delta,
                    &QueryFilter::default(),
                )
                .unwrap()
            {
                Some(h) if !h.started_overlapping => {
                    hits += 1;
                    let at = from + delta * (h.distance / len);
                    assert!(
                        (nearest(at) - radius).abs() < 4e-3,
                        "touching at the hit: {}",
                        nearest(at) - radius
                    );
                    for i in 0..48 {
                        assert!(
                            nearest(from + delta * (h.distance / len * i as f32 / 48.0))
                                > radius - 4e-3,
                            "overlapping before the hit"
                        );
                    }
                    assert!(h.normal.dot(delta) <= 1e-3 && (h.normal.length() - 1.0).abs() < 1e-3);
                }
                Some(h) => {
                    hits += 1;
                    assert!(h.distance == 0.0 && nearest(from) < radius + 1e-3);
                }
                None => {
                    misses += 1;
                    for i in 0..=200 {
                        assert!(
                            nearest(from + delta * (i as f32 / 200.0)) > radius - 4e-3,
                            "a miss, yet overlapping"
                        );
                    }
                }
            }
        }
        assert!(hits > 100 && misses > 100, "hits {hits}, misses {misses}");
    }

    #[test]
    fn overlap_lists_the_touching_colliders_by_id_and_the_materials_of_the_static_world() {
        let mut rng = Lcg(5);
        let (mut w, colliders) = random_world(&mut rng, 80);
        let v = [
            Vec3::new(-15.0, -1.0, -15.0),
            Vec3::new(15.0, -1.0, -15.0),
            Vec3::new(15.0, -1.0, 15.0),
            Vec3::new(-15.0, -1.0, 15.0),
        ];
        w.set_static(
            TriMesh::new(&v, &[0, 2, 1, 0, 3, 2]).unwrap(),
            vec![GRASS, METAL],
        )
        .unwrap();
        for _ in 0..400 {
            let at = rng.vec(13.0);
            let radius = rng.range(0.4, 2.5);
            let probe = Shape::Sphere { radius };
            let mut out = Vec::new();
            w.overlap(
                &probe,
                at,
                &QueryFilter {
                    statics: false,
                    ..QueryFilter::default()
                },
                &mut out,
            )
            .unwrap();
            let got: Vec<u64> = out
                .iter()
                .map(|o| o.collider.expect("only moving colliders").0)
                .collect();
            let want: Vec<u64> = colliders
                .iter()
                .filter(|c| sdf(c, at) <= radius)
                .map(|c| c.id.0)
                .collect();
            let mut sorted = got.clone();
            sorted.sort_unstable();
            assert_eq!(got, sorted, "ordered by id");
            // Compare away from the boundary, where sampling the signed distance and the exact test could differ by rounding.
            let near_edge = colliders.iter().any(|c| (sdf(c, at) - radius).abs() < 1e-3);
            if !near_edge {
                assert_eq!(got, want);
            }
        }
        // The static world: one entry per distinct material, sorted, with no collider.
        let mut out = Vec::new();
        w.overlap(
            &Shape::Sphere { radius: 3.0 },
            Vec3::new(0.0, -0.5, 0.0),
            &QueryFilter {
                dynamics: false,
                ..QueryFilter::default()
            },
            &mut out,
        )
        .unwrap();
        assert_eq!(
            out,
            vec![
                Overlap {
                    collider: None,
                    material: GRASS
                },
                Overlap {
                    collider: None,
                    material: METAL
                }
            ]
        );
    }

    #[test]
    fn boxes_and_capsules_overlap_moving_colliders() {
        let mut w = QueryWorld::new();
        w.set_dynamic(vec![
            collider(
                1,
                Shape::Sphere { radius: 1.0 },
                Vec3::new(0.0, 0.0, 0.0),
                Layers::ALL,
            ),
            collider(
                2,
                Shape::Box {
                    half_extents: Vec3::splat(1.0),
                },
                Vec3::new(5.0, 0.0, 0.0),
                Layers::ALL,
            ),
            collider(
                3,
                Shape::Capsule {
                    half_height: 1.0,
                    radius: 0.5,
                },
                Vec3::new(10.0, 0.0, 0.0),
                Layers::ALL,
            ),
        ]);
        let f = QueryFilter {
            statics: false,
            ..QueryFilter::default()
        };
        let ids = |shape: Shape, at: Vec3| {
            let mut out = Vec::new();
            w.overlap(&shape, at, &f, &mut out).unwrap();
            out.iter()
                .map(|o| o.collider.unwrap().0)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            ids(
                Shape::Box {
                    half_extents: Vec3::splat(0.5)
                },
                Vec3::new(1.4, 0.0, 0.0)
            ),
            [1]
        );
        assert_eq!(
            ids(
                Shape::Box {
                    half_extents: Vec3::splat(2.5)
                },
                Vec3::new(2.5, 0.0, 0.0)
            ),
            [1, 2]
        );
        assert_eq!(
            ids(
                Shape::Capsule {
                    half_height: 1.0,
                    radius: 0.5
                },
                Vec3::new(10.0, 1.8, 0.0)
            ),
            [3]
        );
        assert_eq!(
            ids(
                Shape::Capsule {
                    half_height: 1.0,
                    radius: 0.5
                },
                Vec3::new(7.5, 0.0, 0.0)
            ),
            Vec::<u64>::new()
        );
        assert_eq!(ids(Shape::Sphere { radius: 100.0 }, Vec3::ZERO), [1, 2, 3]);
    }

    #[test]
    fn what_is_not_supported_is_refused_and_bad_input_is_named() {
        let w = floor_world();
        let f = QueryFilter::default();
        assert!(matches!(
            w.shapecast(
                &Shape::Capsule {
                    half_height: 1.0,
                    radius: 0.5
                },
                Vec3::Y,
                Vec3::NEG_Y,
                &f
            ),
            Err(QueryError::Unsupported(_))
        ));
        assert!(
            matches!(
                w.shapecast(
                    &Shape::Box {
                        half_extents: Vec3::ONE
                    },
                    Vec3::Y * 3.0,
                    Vec3::NEG_Y,
                    &f
                ),
                Err(QueryError::Unsupported(_))
            ),
            "a box against the static world"
        );
        let mut out = Vec::new();
        assert!(matches!(
            w.overlap(
                &Shape::Box {
                    half_extents: Vec3::ONE
                },
                Vec3::ZERO,
                &f,
                &mut out
            ),
            Err(QueryError::Unsupported(_))
        ));
        // The same box is fine when only moving colliders are asked about.
        assert!(
            w.overlap(
                &Shape::Box {
                    half_extents: Vec3::ONE
                },
                Vec3::ZERO,
                &QueryFilter {
                    statics: false,
                    ..f
                },
                &mut out
            )
            .is_ok()
        );
        assert!(matches!(
            w.shapecast(
                &Shape::Sphere { radius: 0.5 },
                Vec3::new(f32::NAN, 0.0, 0.0),
                Vec3::X,
                &f
            ),
            Err(QueryError::BadInput(_))
        ));
        assert!(matches!(
            w.shapecast(&Shape::Sphere { radius: -1.0 }, Vec3::ZERO, Vec3::X, &f),
            Err(QueryError::BadInput(_))
        ));
        assert!(
            w.raycast(&down(Vec3::new(0.0, 5.0, 0.0)), f32::NAN, &f)
                .is_none(),
            "a bad length is no hit"
        );
    }

    #[test]
    fn casts_honour_the_excluded_collider_and_refuse_boxes_against_round_colliders() {
        let mut w = QueryWorld::new();
        w.set_dynamic(vec![
            collider(
                1,
                Shape::Box {
                    half_extents: Vec3::splat(1.0),
                },
                Vec3::new(10.0, 0.0, 0.0),
                Layers::bit(1),
            ),
            collider(
                2,
                Shape::Sphere { radius: 1.0 },
                Vec3::new(20.0, 0.0, 0.0),
                Layers::bit(2),
            ),
        ]);
        let sphere = Shape::Sphere { radius: 0.5 };
        let go = Vec3::new(40.0, 0.0, 0.0);
        let f = QueryFilter::default();
        assert_eq!(
            w.shapecast(&sphere, Vec3::ZERO, go, &f)
                .unwrap()
                .unwrap()
                .collider,
            Some(ColliderId(1))
        );
        let skip = QueryFilter {
            exclude: Some(ColliderId(1)),
            ..f
        };
        assert_eq!(
            w.shapecast(&sphere, Vec3::ZERO, go, &skip)
                .unwrap()
                .unwrap()
                .collider,
            Some(ColliderId(2)),
            "the next one along"
        );
        // A box cast is supported against box colliders only; a sphere collider on the mask makes it refuse.
        let boxed = Shape::Box {
            half_extents: Vec3::splat(0.5),
        };
        assert!(matches!(
            w.shapecast(&boxed, Vec3::ZERO, go, &f),
            Err(QueryError::Unsupported(_))
        ));
        let only_boxes = QueryFilter {
            mask: Layers::bit(1),
            ..f
        };
        assert_eq!(
            w.shapecast(&boxed, Vec3::ZERO, go, &only_boxes)
                .unwrap()
                .unwrap()
                .collider,
            Some(ColliderId(1)),
            "the sphere is on another layer, so nothing unsupported is in play"
        );
    }

    #[test]
    fn two_colliders_at_the_same_distance_give_the_lower_id_whatever_the_input_order() {
        let ray = Ray::new(Vec3::new(-10.0, 0.0, 0.0), Vec3::X).unwrap();
        let same = |id| collider(id, Shape::Sphere { radius: 1.0 }, Vec3::ZERO, Layers::ALL);
        for order in [[5, 2], [2, 5]] {
            let mut w = QueryWorld::new();
            w.set_dynamic(order.iter().map(|id| same(*id)).collect());
            assert_eq!(
                w.raycast(&ray, 100.0, &QueryFilter::default())
                    .unwrap()
                    .collider,
                Some(ColliderId(2)),
                "input order {order:?}"
            );
        }
    }

    #[test]
    fn the_static_materials_must_match_the_triangles() {
        let v = [Vec3::ZERO, Vec3::X, Vec3::Y];
        let mut w = QueryWorld::new();
        let err = w.set_static(TriMesh::new(&v, &[0, 1, 2]).unwrap(), vec![GRASS, METAL]);
        assert_eq!(
            err,
            Err(QueryError::BadInput("one material per triangle is needed"))
        );
    }

    #[test]
    fn the_order_the_colliders_are_given_in_does_not_change_the_answers() {
        let mut rng = Lcg(8);
        let (mut forward, colliders) = random_world(&mut rng, 50);
        let mut backward = QueryWorld::new();
        let mut reversed = colliders.clone();
        reversed.reverse();
        backward.set_dynamic(reversed);
        forward.set_dynamic(colliders);
        for _ in 0..200 {
            let at = rng.vec(13.0);
            let probe = Shape::Sphere {
                radius: rng.range(0.5, 3.0),
            };
            let f = QueryFilter::default();
            let (mut a, mut b) = (Vec::new(), Vec::new());
            forward.overlap(&probe, at, &f, &mut a).unwrap();
            backward.overlap(&probe, at, &f, &mut b).unwrap();
            assert_eq!(a, b);
        }
    }

    #[test]
    fn the_contract_can_be_used_through_a_trait_object() {
        fn can_stand_on(world: &dyn SpatialQuery, at: Vec3) -> bool {
            world
                .raycast(
                    &Ray::new(at, Vec3::NEG_Y).unwrap(),
                    2.0,
                    &QueryFilter::default(),
                )
                .is_some_and(|h| h.normal.y > 0.7)
        }
        let w = floor_world();
        assert!(can_stand_on(&w, Vec3::new(0.0, 1.0, 0.0)));
        assert!(!can_stand_on(&w, Vec3::new(0.0, 5.0, 0.0)));
    }
}
