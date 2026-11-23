//! Spatial acceleration: a bounding volume hierarchy ([`Bvh`]) for rays, sweeps and overlaps, layer
//! filters ([`Layers`]) and a triangle mesh ([`TriMesh`]) built on it (roadmap T07-003).
//!
//! The same structure serves the static world (build once from the world's triangles) and the
//! **dynamic broadphase** (build again each tick from the bounds of the moving things: a median
//! split build is `O(n log n)` and has no state to keep in step). Everything is deterministic:
//! the build breaks every tie by the order of the input, queries visit in tree order, and nothing
//! uses a hash map, so the same input gives the same tree, the same visiting order and the same
//! answers on every machine.
//!
//! The BVH only finds **candidates**: the caller's closure does the exact test (a triangle, a
//! capsule, a character) and the BVH prunes the rest. The queries are checked against brute force.

use std::fmt;

use crate::geometry::{Aabb, Capsule, Ray, SweepHit};
use ss2_core::math::Vec3;

/// A set of up to 32 layers, as bits. Things are on layers and a query has a mask: a thing is
/// considered when its layers and the mask share a bit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layers(pub u32);

impl Layers {
    /// Every layer.
    pub const ALL: Self = Self(u32::MAX);
    /// No layer.
    pub const NONE: Self = Self(0);

    /// The single layer `n` (0 to 31); a larger number is no layer.
    pub fn bit(n: u32) -> Self {
        if n < 32 { Self(1 << n) } else { Self::NONE }
    }

    /// Whether the two sets share a layer.
    pub fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }

    /// Whether every layer of `other` is in this set.
    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// The layers of both.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

const LEAF_SIZE: usize = 4;
/// How much every box is padded in a ray query, so flat boxes are not missed.
const BOX_PADDING: f32 = 1e-4;

struct Item<T> {
    bounds: Aabb,
    layers: Layers,
    value: T,
}

enum Kind {
    Leaf { first: u32, count: u32 },
    Inner { left: u32, right: u32 },
}

struct Node {
    bounds: Aabb,
    layers: Layers,
    kind: Kind,
}

/// A bounding volume hierarchy over boxes that carry a value and layers; see the module
/// documentation.
pub struct Bvh<T> {
    nodes: Vec<Node>,
    items: Vec<Item<T>>,
    dropped: usize,
}

impl<T> Bvh<T> {
    /// Build a tree over `items` (bounds, layers, value). The build splits at the median of the
    /// longest axis of the centres, breaking ties by the order of the input, so it is
    /// deterministic. Items whose bounds are not finite are left out and counted
    /// ([`Bvh::dropped`]).
    pub fn build(items: Vec<(Aabb, Layers, T)>) -> Self {
        let total = items.len();
        let mut slots: Vec<Option<Item<T>>> = Vec::with_capacity(total);
        let mut order: Vec<usize> = Vec::with_capacity(total);
        for (bounds, layers, value) in items {
            if bounds.min.is_finite() && bounds.max.is_finite() {
                order.push(slots.len());
                slots.push(Some(Item {
                    bounds,
                    layers,
                    value,
                }));
            }
        }
        let dropped = total - slots.len();
        let mut tree = Self {
            nodes: Vec::new(),
            items: Vec::with_capacity(slots.len()),
            dropped,
        };
        if order.is_empty() {
            return tree;
        }
        let bounds: Vec<Aabb> = slots
            .iter()
            .map(|s| s.as_ref().expect("filled").bounds)
            .collect();
        let layers: Vec<Layers> = slots
            .iter()
            .map(|s| s.as_ref().expect("filled").layers)
            .collect();
        tree.build_node(&mut order, 0, &bounds, &layers);
        for index in order {
            tree.items
                .push(slots[index].take().expect("each item once"));
        }
        tree
    }

    /// Build the subtree over `order[..]` (which sits at `offset` in the final item order) and
    /// return its node index.
    fn build_node(
        &mut self,
        order: &mut [usize],
        offset: usize,
        bounds: &[Aabb],
        layers: &[Layers],
    ) -> u32 {
        let node_bounds = order
            .iter()
            .skip(1)
            .fold(bounds[order[0]], |u, i| u.union(&bounds[*i]));
        let node_layers = order.iter().fold(Layers::NONE, |u, i| u.union(layers[*i]));
        let index = self.nodes.len() as u32;
        if order.len() <= LEAF_SIZE {
            self.nodes.push(Node {
                bounds: node_bounds,
                layers: node_layers,
                kind: Kind::Leaf {
                    first: offset as u32,
                    count: order.len() as u32,
                },
            });
            return index;
        }
        self.nodes.push(Node {
            bounds: node_bounds,
            layers: node_layers,
            kind: Kind::Leaf { first: 0, count: 0 },
        });
        // The longest axis of the centres.
        let (mut lo, mut hi) = (bounds[order[0]].center(), bounds[order[0]].center());
        for i in order.iter() {
            let c = bounds[*i].center();
            lo = lo.min(c);
            hi = hi.max(c);
        }
        let extent = hi - lo;
        let axis = if extent.x >= extent.y && extent.x >= extent.z {
            0
        } else if extent.y >= extent.z {
            1
        } else {
            2
        };
        order.sort_by(|a, b| {
            bounds[*a].center()[axis]
                .total_cmp(&bounds[*b].center()[axis])
                .then(a.cmp(b))
        });
        let mid = order.len() / 2;
        let (left_order, right_order) = order.split_at_mut(mid);
        let left = self.build_node(left_order, offset, bounds, layers);
        let right = self.build_node(right_order, offset + mid, bounds, layers);
        self.nodes[index as usize].kind = Kind::Inner { left, right };
        index
    }

    /// How many items are in the tree.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the tree holds nothing.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// How many input items were left out for non-finite bounds.
    pub fn dropped(&self) -> usize {
        self.dropped
    }

    /// The box around everything.
    pub fn bounds(&self) -> Option<Aabb> {
        self.nodes.first().map(|n| n.bounds)
    }

    /// The number of levels (0 for an empty tree).
    pub fn depth(&self) -> usize {
        fn go<T>(tree: &Bvh<T>, index: u32) -> usize {
            match tree.nodes[index as usize].kind {
                Kind::Leaf { .. } => 1,
                Kind::Inner { left, right } => 1 + go(tree, left).max(go(tree, right)),
            }
        }
        if self.nodes.is_empty() {
            0
        } else {
            go(self, 0)
        }
    }

    /// Call `visit(depth, bounds, is_leaf)` for every node, parents before children and the left child
    /// first (the root is depth 0): for drawing the tree and for statistics.
    pub fn visit_nodes(&self, visit: &mut impl FnMut(usize, &Aabb, bool)) {
        let mut stack = if self.nodes.is_empty() {
            Vec::new()
        } else {
            vec![(0_u32, 0_usize)]
        };
        while let Some((index, depth)) = stack.pop() {
            let node = &self.nodes[index as usize];
            match node.kind {
                Kind::Leaf { .. } => visit(depth, &node.bounds, true),
                Kind::Inner { left, right } => {
                    visit(depth, &node.bounds, false);
                    stack.push((right, depth + 1));
                    stack.push((left, depth + 1));
                }
            }
        }
    }

    /// Verify the structure: every node's box contains what is under it, and every item is in
    /// exactly one leaf. For tests and debugging.
    pub fn check(&self) -> Result<(), String> {
        let mut seen = vec![false; self.items.len()];
        let mut stack = if self.nodes.is_empty() {
            Vec::new()
        } else {
            vec![0_u32]
        };
        while let Some(i) = stack.pop() {
            let node = &self.nodes[i as usize];
            match node.kind {
                Kind::Leaf { first, count } => {
                    for k in first..first + count {
                        let item = self
                            .items
                            .get(k as usize)
                            .ok_or("a leaf reaches past the items")?;
                        if seen[k as usize] {
                            return Err(format!("item {k} is in two leaves"));
                        }
                        seen[k as usize] = true;
                        if !(node.bounds.contains_point(item.bounds.min)
                            && node.bounds.contains_point(item.bounds.max))
                        {
                            return Err(format!("a leaf does not contain its item {k}"));
                        }
                        if !node.layers.contains(item.layers) {
                            return Err(format!("a leaf's layers miss those of item {k}"));
                        }
                    }
                }
                Kind::Inner { left, right } => {
                    for child in [left, right] {
                        let c = &self.nodes[child as usize];
                        if !(node.bounds.contains_point(c.bounds.min)
                            && node.bounds.contains_point(c.bounds.max))
                        {
                            return Err(format!("node {i} does not contain its child {child}"));
                        }
                        if !node.layers.contains(c.layers) {
                            return Err(format!("node {i} misses the layers of its child {child}"));
                        }
                        stack.push(child);
                    }
                }
            }
        }
        if seen.iter().all(|s| *s) {
            Ok(())
        } else {
            Err("an item is in no leaf".into())
        }
    }

    /// Call `visit` with the value of every item whose box overlaps `region` and whose layers
    /// share a bit with `mask`, in tree order.
    pub fn query_aabb(&self, region: &Aabb, mask: Layers, visit: &mut impl FnMut(&T)) {
        let mut stack = if self.nodes.is_empty() {
            Vec::new()
        } else {
            vec![0_u32]
        };
        while let Some(i) = stack.pop() {
            let node = &self.nodes[i as usize];
            if !node.layers.intersects(mask) || !node.bounds.overlaps(region) {
                continue;
            }
            match node.kind {
                Kind::Leaf { first, count } => {
                    for item in &self.items[first as usize..(first + count) as usize] {
                        if item.layers.intersects(mask) && item.bounds.overlaps(region) {
                            visit(&item.value);
                        }
                    }
                }
                Kind::Inner { left, right } => {
                    stack.push(right);
                    stack.push(left);
                }
            }
        }
    }

    /// Find the nearest hit along `ray` up to `max_t`. Every item whose box, **grown by `inflate`
    /// on all sides**, the ray reaches before the best hit so far is offered to `hit(value,
    /// best_t)`, which does the exact test and returns the distance of its hit if that is no
    /// farther than `best_t`. Growing the boxes by a radius turns this into a sweep of a sphere
    /// (the caller's exact test then measures the sphere, not the ray). Nodes are visited nearest
    /// first, so most of the tree is never touched.
    pub fn query_ray(
        &self,
        ray: &Ray,
        max_t: f32,
        mask: Layers,
        inflate: f32,
        hit: &mut impl FnMut(&T, f32) -> Option<f32>,
    ) -> Option<(f32, &T)> {
        if self.nodes.is_empty() {
            return None;
        }
        // A box with no thickness (a floor or a wall triangle that is axis aligned) would be
        // missed by the slab test, since the ray enters and leaves it at the same instant: pad
        // every box a little. The caller's exact test makes the final decision.
        let pad = inflate.max(BOX_PADDING);
        let entry = |b: &Aabb, limit: f32| b.expanded(pad).raycast(ray, limit).map(|h| h.t);
        let mut best_t = max_t;
        let mut best: Option<(f32, &T)> = None;
        let root_t = entry(&self.nodes[0].bounds, best_t)?;
        let mut stack: Vec<(u32, f32)> = vec![(0, root_t)];
        while let Some((i, t_enter)) = stack.pop() {
            if t_enter > best_t {
                continue;
            }
            let node = &self.nodes[i as usize];
            if !node.layers.intersects(mask) {
                continue;
            }
            match node.kind {
                Kind::Leaf { first, count } => {
                    for item in &self.items[first as usize..(first + count) as usize] {
                        if !item.layers.intersects(mask) {
                            continue;
                        }
                        let Some(t_item) = entry(&item.bounds, best_t) else {
                            continue;
                        };
                        if t_item > best_t {
                            continue;
                        }
                        if let Some(t) = hit(&item.value, best_t)
                            && (best.is_none() || t < best_t)
                            && t <= best_t
                        {
                            best_t = t;
                            best = Some((t, &item.value));
                        }
                    }
                }
                Kind::Inner { left, right } => {
                    let l = entry(&self.nodes[left as usize].bounds, best_t);
                    let r = entry(&self.nodes[right as usize].bounds, best_t);
                    match (l, r) {
                        (Some(lt), Some(rt)) => {
                            // Push the farther child first so the nearer pops first.
                            if lt <= rt {
                                stack.push((right, rt));
                                stack.push((left, lt));
                            } else {
                                stack.push((left, lt));
                                stack.push((right, rt));
                            }
                        }
                        (Some(lt), None) => stack.push((left, lt)),
                        (None, Some(rt)) => stack.push((right, rt)),
                        (None, None) => {}
                    }
                }
            }
        }
        best
    }
}

/// A triangle.
///
/// ```
/// use ss2_core::math::Vec3;
/// use ss2_physics::geometry::Ray;
/// use ss2_physics::spatial::{Layers, TriMesh};
///
/// // A floor: two triangles. A ray from above hits it where it lands.
/// let floor = [Vec3::new(-5.0, 0.0, -5.0), Vec3::new(5.0, 0.0, -5.0), Vec3::new(5.0, 0.0, 5.0), Vec3::new(-5.0, 0.0, 5.0)];
/// let mesh = TriMesh::new(&floor, &[0, 2, 1, 0, 3, 2]).unwrap();
/// let down = Ray::new(Vec3::new(1.0, 3.0, 1.0), Vec3::NEG_Y).unwrap();
/// let hit = mesh.raycast(&down, 100.0, Layers::ALL).unwrap();
/// assert!((hit.t - 3.0).abs() < 1e-5 && hit.normal == Vec3::Y);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Triangle {
    /// The first corner.
    pub a: Vec3,
    /// The second corner.
    pub b: Vec3,
    /// The third corner.
    pub c: Vec3,
}

/// A ray's hit on a [`Triangle`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TriHit {
    /// Distance along the ray.
    pub t: f32,
    /// The point.
    pub point: Vec3,
    /// The face normal, unit, turned to face the ray (triangles are two-sided).
    pub normal: Vec3,
}

impl Triangle {
    /// A triangle.
    pub fn new(a: Vec3, b: Vec3, c: Vec3) -> Self {
        Self { a, b, c }
    }

    /// The box around the triangle.
    pub fn aabb(&self) -> Aabb {
        Aabb::new(
            self.a.min(self.b).min(self.c),
            self.a.max(self.b).max(self.c),
        )
    }

    /// Whether the corners are (nearly) on a line, so the triangle has no area to hit.
    pub fn is_degenerate(&self) -> bool {
        let (ab, ac) = (self.b - self.a, self.c - self.a);
        ab.cross(ac).length_squared()
            <= 1e-12 * (ab.length_squared() * ac.length_squared()).max(1e-12)
    }

    /// Cast a ray at the triangle, at most `max_t` long (the Möller-Trumbore method, two-sided).
    pub fn raycast(&self, ray: &Ray, max_t: f32) -> Option<TriHit> {
        const TOLERANCE: f32 = 1e-6; // lets a ray through the seam between two triangles
        let (e1, e2) = (self.b - self.a, self.c - self.a);
        let p = ray.dir.cross(e2);
        let det = e1.dot(p);
        if det.abs() < 1e-12 {
            return None;
        }
        let inv = 1.0 / det;
        let s = ray.origin - self.a;
        let u = s.dot(p) * inv;
        if !(-TOLERANCE..=1.0 + TOLERANCE).contains(&u) {
            return None;
        }
        let q = s.cross(e1);
        let v = ray.dir.dot(q) * inv;
        if v < -TOLERANCE || u + v > 1.0 + TOLERANCE {
            return None;
        }
        let t = e2.dot(q) * inv;
        if !(0.0..=max_t).contains(&t) {
            return None;
        }
        let mut normal = e1.cross(e2).try_normalize()?;
        if normal.dot(ray.dir) > 0.0 {
            normal = -normal;
        }
        Some(TriHit {
            t,
            point: ray.point_at(t),
            normal,
        })
    }

    /// Move a sphere from `center` by `delta` and find when it first touches this triangle. The
    /// sphere's centre is a ray against the rounded triangle slab: the triangle moved by the radius
    /// up and down its normal (the faces) and the three edges as capsules (whose round ends are
    /// the corners). A sphere that already overlaps the triangle is reported at 0 with the
    /// direction to push it out.
    pub fn sweep_sphere(&self, center: Vec3, radius: f32, delta: Vec3) -> Option<SweepHit> {
        if !center.is_finite() || !delta.is_finite() || !radius.is_finite() || radius < 0.0 {
            return None;
        }
        let normal = (self.b - self.a).cross(self.c - self.a).try_normalize()?;
        let nearest = self.closest_point(center);
        if (center - nearest).length_squared() < radius * radius {
            let push = (center - nearest)
                .try_normalize()
                .unwrap_or(if normal.dot(delta) <= 0.0 {
                    normal
                } else {
                    -normal
                });
            return Some(SweepHit {
                t: 0.0,
                position: center,
                normal: push,
                overlapping_at_start: true,
            });
        }
        let len = delta.length();
        let ray = Ray::new(center, delta).filter(|_| len > 1e-12)?;
        let mut best = f32::MAX;
        for sign in [1.0_f32, -1.0] {
            let lift = normal * (radius * sign);
            let shifted = Triangle::new(self.a + lift, self.b + lift, self.c + lift);
            if let Some(h) = shifted.raycast(&ray, len) {
                best = best.min(h.t);
            }
        }
        for (p, q) in [(self.a, self.b), (self.b, self.c), (self.c, self.a)] {
            if let Some(h) = Capsule::new(p, q, radius).raycast(&ray, len) {
                best = best.min(h.t);
            }
        }
        if best == f32::MAX {
            return None;
        }
        let t = best / len;
        let position = center + delta * t;
        let contact_normal = (position - self.closest_point(position))
            .try_normalize()
            .unwrap_or(if normal.dot(delta) <= 0.0 {
                normal
            } else {
                -normal
            });
        Some(SweepHit {
            t,
            position,
            normal: contact_normal,
            overlapping_at_start: false,
        })
    }

    /// The point of the triangle nearest to `p`.
    pub fn closest_point(&self, p: Vec3) -> Vec3 {
        let (a, b, c) = (self.a, self.b, self.c);
        let (ab, ac, ap) = (b - a, c - a, p - a);
        let (d1, d2) = (ab.dot(ap), ac.dot(ap));
        if d1 <= 0.0 && d2 <= 0.0 {
            return a;
        }
        let bp = p - b;
        let (d3, d4) = (ab.dot(bp), ac.dot(bp));
        if d3 >= 0.0 && d4 <= d3 {
            return b;
        }
        let vc = d1 * d4 - d3 * d2;
        if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
            return a + ab * (d1 / (d1 - d3));
        }
        let cp = p - c;
        let (d5, d6) = (ab.dot(cp), ac.dot(cp));
        if d6 >= 0.0 && d5 <= d6 {
            return c;
        }
        let vb = d5 * d2 - d1 * d6;
        if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
            return a + ac * (d2 / (d2 - d6));
        }
        let va = d3 * d6 - d5 * d4;
        if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
            return b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)));
        }
        let denom = 1.0 / (va + vb + vc);
        a + ab * (vb * denom) + ac * (vc * denom)
    }
}

/// Why a mesh could not be built.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MeshError {
    /// The index list is not made of triples.
    IndexCountNotMultipleOfThree(usize),
    /// An index points past the vertices.
    IndexOutOfRange {
        /// The index.
        index: u32,
        /// How many vertices there are.
        vertices: usize,
    },
    /// A vertex is NaN or infinite.
    NonFiniteVertex(usize),
}

impl fmt::Display for MeshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IndexCountNotMultipleOfThree(n) => {
                write!(f, "{n} indices is not a whole number of triangles")
            }
            Self::IndexOutOfRange { index, vertices } => {
                write!(f, "index {index} with only {vertices} vertices")
            }
            Self::NonFiniteVertex(i) => write!(f, "vertex {i} is not finite"),
        }
    }
}

impl std::error::Error for MeshError {}

/// A ray's hit on a [`TriMesh`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshHit {
    /// Distance along the ray.
    pub t: f32,
    /// The point.
    pub point: Vec3,
    /// The face normal, unit, facing the ray.
    pub normal: Vec3,
    /// Which triangle (its index in the mesh).
    pub triangle: u32,
}

/// A triangle mesh with a BVH over its triangles: the static world's collision geometry.
pub struct TriMesh {
    triangles: Vec<Triangle>,
    layers: Vec<Layers>,
    bvh: Bvh<u32>,
    dropped_degenerate: usize,
}

impl TriMesh {
    /// Build a mesh from vertices and triangle indices. Triangles with no area are left out and
    /// counted ([`TriMesh::dropped_degenerate`]); every triangle starts on all layers.
    pub fn new(vertices: &[Vec3], indices: &[u32]) -> Result<Self, MeshError> {
        if !indices.len().is_multiple_of(3) {
            return Err(MeshError::IndexCountNotMultipleOfThree(indices.len()));
        }
        if let Some(i) = vertices.iter().position(|v| !v.is_finite()) {
            return Err(MeshError::NonFiniteVertex(i));
        }
        if let Some(index) = indices
            .iter()
            .copied()
            .find(|i| *i as usize >= vertices.len())
        {
            return Err(MeshError::IndexOutOfRange {
                index,
                vertices: vertices.len(),
            });
        }
        let mut triangles = Vec::new();
        let mut dropped_degenerate = 0;
        for tri in indices.as_chunks::<3>().0 {
            let t = Triangle::new(
                vertices[tri[0] as usize],
                vertices[tri[1] as usize],
                vertices[tri[2] as usize],
            );
            if t.is_degenerate() {
                dropped_degenerate += 1;
            } else {
                triangles.push(t);
            }
        }
        let layers = vec![Layers::ALL; triangles.len()];
        let mut mesh = Self {
            triangles,
            layers,
            bvh: Bvh::build(Vec::new()),
            dropped_degenerate,
        };
        mesh.rebuild();
        Ok(mesh)
    }

    fn rebuild(&mut self) {
        let items = self
            .triangles
            .iter()
            .zip(&self.layers)
            .enumerate()
            .map(|(i, (t, l))| (t.aabb(), *l, i as u32))
            .collect();
        self.bvh = Bvh::build(items);
    }

    /// How many triangles the mesh has.
    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    /// Triangle `i`.
    pub fn triangle(&self, i: u32) -> Triangle {
        self.triangles[i as usize]
    }

    /// How many degenerate triangles were left out.
    pub fn dropped_degenerate(&self) -> usize {
        self.dropped_degenerate
    }

    /// The box around the whole mesh.
    pub fn bounds(&self) -> Option<Aabb> {
        self.bvh.bounds()
    }

    /// Put every triangle on `layers` (for instance, the world on a "static" layer).
    pub fn set_all_layers(&mut self, layers: Layers) {
        self.layers.iter_mut().for_each(|l| *l = layers);
        self.rebuild();
    }

    /// The nearest triangle the ray hits within `max_t`, among triangles on a layer of `mask`.
    pub fn raycast(&self, ray: &Ray, max_t: f32, mask: Layers) -> Option<MeshHit> {
        let mut found: Option<TriHit> = None;
        let (t, triangle) = self.bvh.query_ray(ray, max_t, mask, 0.0, &mut |i, best| {
            let hit = self.triangles[*i as usize].raycast(ray, best)?;
            // The tree takes a hit only if it is nearer than the best so far (or the first): keep
            // `found` in step with that rule, so a tie never replaces the winner's details.
            if found.is_none() || hit.t < best {
                found = Some(hit);
            }
            Some(hit.t)
        })?;
        // `found` is the details of the hit the tree accepted: the winner.
        let hit = found?;
        Some(MeshHit {
            t,
            point: hit.point,
            normal: hit.normal,
            triangle: *triangle,
        })
    }

    /// Move a sphere from `center` by `delta` and find the first triangle it touches, among
    /// triangles on a layer of `mask`: the sweep, and the triangle's index. A sphere that already
    /// overlaps a triangle is reported at `t = 0`. A zero `delta` checks for overlap only.
    pub fn sweep_sphere(
        &self,
        center: Vec3,
        radius: f32,
        delta: Vec3,
        mask: Layers,
    ) -> Option<(SweepHit, u32)> {
        if !center.is_finite() || !delta.is_finite() || !radius.is_finite() || radius < 0.0 {
            return None;
        }
        let len = delta.length();
        let mut found: Option<(SweepHit, u32)> = None;
        if len <= 1e-12 {
            let region = Aabb::from_center_half(center, Vec3::splat(radius));
            self.bvh.query_aabb(&region, mask, &mut |i| {
                if found.is_none()
                    && let Some(h) =
                        self.triangles[*i as usize].sweep_sphere(center, radius, Vec3::ZERO)
                {
                    found = Some((h, *i));
                }
            });
            return found;
        }
        let ray = Ray::new(center, delta)?;
        self.bvh
            .query_ray(&ray, len, mask, radius, &mut |i, best| {
                let hit = self.triangles[*i as usize].sweep_sphere(center, radius, delta)?;
                let distance = hit.t * len;
                if distance > best {
                    return None;
                }
                if found.is_none() || distance < best {
                    found = Some((hit, *i));
                }
                Some(distance)
            })?;
        found
    }

    /// Call `visit` with every triangle (by index) within `radius` of `center`, among triangles on
    /// a layer of `mask`, in tree order.
    pub fn query_sphere(
        &self,
        center: Vec3,
        radius: f32,
        mask: Layers,
        visit: &mut impl FnMut(u32),
    ) {
        let region = Aabb::from_center_half(center, Vec3::splat(radius));
        self.bvh.query_aabb(&region, mask, &mut |i| {
            let nearest = self.triangles[*i as usize].closest_point(center);
            if (nearest - center).length_squared() <= radius * radius {
                visit(*i);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Aabb, Ray};
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
        fn below(&mut self, n: u32) -> u32 {
            (self.next() * n as f32) as u32 % n.max(1)
        }
    }

    fn random_boxes(rng: &mut Lcg, count: usize) -> Vec<(Aabb, Layers, u32)> {
        (0..count)
            .map(|i| {
                let half = Vec3::new(
                    rng.range(0.1, 1.5),
                    rng.range(0.1, 1.5),
                    rng.range(0.1, 1.5),
                );
                let layers = Layers::bit(rng.below(4));
                (
                    Aabb::from_center_half(rng.vec(20.0), half),
                    layers,
                    i as u32,
                )
            })
            .collect()
    }

    #[test]
    fn layers_are_bit_masks() {
        assert!(Layers::ALL.intersects(Layers::bit(5)));
        assert!(!Layers::NONE.intersects(Layers::ALL));
        assert!(!Layers::bit(1).intersects(Layers::bit(2)));
        let both = Layers::bit(1).union(Layers::bit(2));
        assert!(
            both.intersects(Layers::bit(2))
                && both.contains(Layers::bit(1))
                && !both.contains(Layers::bit(3))
        );
        assert_eq!(
            Layers::bit(40),
            Layers::NONE,
            "a layer past 31 does not exist and is not a wrap-around"
        );
    }

    #[test]
    fn an_empty_tree_answers_nothing() {
        let tree: Bvh<u32> = Bvh::build(Vec::new());
        assert!(tree.is_empty() && tree.bounds().is_none() && tree.depth() == 0);
        let mut seen = 0;
        tree.query_aabb(
            &Aabb::new(Vec3::splat(-1e9), Vec3::splat(1e9)),
            Layers::ALL,
            &mut |_| seen += 1,
        );
        assert_eq!(seen, 0);
        let ray = Ray::new(Vec3::ZERO, Vec3::X).unwrap();
        assert!(
            tree.query_ray(&ray, 100.0, Layers::ALL, 0.0, &mut |_, _| Some(0.0))
                .is_none()
        );
    }

    #[test]
    fn the_tree_is_well_formed_for_any_size() {
        let mut rng = Lcg(1);
        for count in [1_usize, 2, 3, 4, 5, 17, 100, 1000] {
            let items = random_boxes(&mut rng, count);
            let tree = Bvh::build(items.clone());
            assert_eq!(tree.len(), count);
            tree.check()
                .unwrap_or_else(|e| panic!("{count} items: {e}"));
            let union = items.iter().fold(items[0].0, |u, i| u.union(&i.0));
            assert_eq!(tree.bounds(), Some(union));
            let log = (count as f32).log2().ceil() as usize + 2;
            assert!(
                tree.depth() <= log + 2,
                "{count} items: depth {} for log2 {}",
                tree.depth(),
                log
            );
        }
    }

    #[test]
    fn visiting_the_nodes_gives_every_node_once_parents_first_with_their_depth() {
        let mut rng = Lcg(13);
        let tree = Bvh::build(random_boxes(&mut rng, 100));
        let (mut count, mut leaves, mut max_depth, mut first) = (0, 0, 0, None);
        tree.visit_nodes(&mut |depth, bounds, is_leaf| {
            first.get_or_insert((depth, *bounds));
            count += 1;
            max_depth = max_depth.max(depth);
            if is_leaf {
                leaves += 1;
            }
        });
        assert_eq!(
            first,
            Some((0, tree.bounds().unwrap())),
            "the root comes first, at depth 0"
        );
        assert_eq!(
            count,
            2 * leaves - 1,
            "a binary tree with {leaves} leaves has that many nodes"
        );
        assert_eq!(max_depth + 1, tree.depth());
        assert!(leaves >= 100 / 4, "leaves hold at most four items each");
        let empty: Bvh<u32> = Bvh::build(Vec::new());
        empty.visit_nodes(&mut |_, _, _| panic!("an empty tree has no nodes"));
    }

    #[test]
    fn overlap_queries_return_exactly_the_brute_force_set() {
        let mut rng = Lcg(2);
        let items = random_boxes(&mut rng, 600);
        let tree = Bvh::build(items.clone());
        for _ in 0..300 {
            let region = Aabb::from_center_half(
                rng.vec(22.0),
                Vec3::new(
                    rng.range(0.2, 6.0),
                    rng.range(0.2, 6.0),
                    rng.range(0.2, 6.0),
                ),
            );
            let mask = Layers(rng.below(16));
            let mut got: Vec<u32> = Vec::new();
            tree.query_aabb(&region, mask, &mut |v| got.push(*v));
            got.sort_unstable();
            let want: Vec<u32> = items
                .iter()
                .filter(|(b, l, _)| b.overlaps(&region) && l.intersects(mask))
                .map(|(_, _, v)| *v)
                .collect();
            assert_eq!(got, want);
        }
    }

    #[test]
    fn ray_queries_find_the_nearest_exact_hit_like_brute_force() {
        let mut rng = Lcg(3);
        let items = random_boxes(&mut rng, 500);
        let tree = Bvh::build(items.clone());
        let (mut hits, mut misses) = (0, 0);
        for _ in 0..600 {
            let origin = rng.vec(25.0);
            let Some(ray) = Ray::new(origin, rng.vec(20.0) - origin) else {
                continue;
            };
            let max_t = rng.range(5.0, 60.0);
            let mask = Layers(1 + rng.below(15));
            // The exact test of an item is the ray against its box.
            let exact = |idx: u32| items[idx as usize].0.raycast(&ray, max_t).map(|h| h.t);
            let got = tree.query_ray(&ray, max_t, mask, 0.0, &mut |v, best| {
                exact(*v).filter(|t| *t <= best)
            });
            let want = items
                .iter()
                .filter(|(_, l, _)| l.intersects(mask))
                .filter_map(|(_, _, v)| exact(*v).map(|t| (t, *v)))
                .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            match (got, want) {
                (Some((t, v)), Some((wt, wv))) => {
                    hits += 1;
                    assert!((t - wt).abs() < 1e-4, "t {t} vs {wt}");
                    if (t - wt).abs() < 1e-6 && v != &wv {
                        // Two boxes at the same distance: either is a correct answer.
                        assert!(exact(*v).is_some_and(|x| (x - wt).abs() < 1e-4));
                    }
                }
                (None, None) => misses += 1,
                other => panic!("tree and brute force disagree: {other:?}"),
            }
        }
        assert!(hits > 100 && misses > 50, "hits {hits}, misses {misses}");
    }

    #[test]
    fn a_ray_query_runs_the_exact_test_on_few_items_visiting_the_nearest_first() {
        let mut rng = Lcg(8);
        let items = random_boxes(&mut rng, 3000);
        let tree = Bvh::build(items.clone());
        let (mut calls, mut rays) = (0_usize, 0_usize);
        for _ in 0..200 {
            let origin = rng.vec(25.0);
            let Some(ray) = Ray::new(origin, rng.vec(20.0) - origin) else {
                continue;
            };
            rays += 1;
            tree.query_ray(&ray, 80.0, Layers::ALL, 0.0, &mut |v, best| {
                calls += 1;
                items[*v as usize].0.raycast(&ray, best).map(|h| h.t)
            });
        }
        let average = calls as f32 / rays as f32;
        assert!(
            average < 3.0, // measured: 1.2 nearest-first, 5.8 when the farther child is visited first
            "{average} exact tests per ray out of 3000 items"
        );
    }

    #[test]
    fn a_ray_through_the_seam_between_two_triangles_still_hits() {
        let b = Aabb::new(Vec3::new(-1.0, -2.0, -3.0), Vec3::new(2.0, 1.0, 1.5));
        let mesh = box_mesh(&b);
        // The bottom face is split along its diagonal from (-1, -2, -3) to (2, -2, 1.5): rays
        // from below through points on that diagonal pass exactly through the seam.
        let (from, to) = (Vec3::new(-1.0, -2.0, -3.0), Vec3::new(2.0, -2.0, 1.5));
        for k in 1..400 {
            let p = from + (to - from) * (k as f32 / 400.0);
            let ray = Ray::new(Vec3::new(p.x, -9.0, p.z), Vec3::Y).unwrap(); // the bottom face is met first
            let hit = mesh.raycast(&ray, 100.0, Layers::ALL);
            assert!(
                hit.is_some_and(|h| (h.t - 7.0).abs() < 1e-3),
                "a seam is not a hole (point {k} of 400: {p:?})"
            );
        }
    }

    #[test]
    fn inflating_the_ray_finds_what_a_sphere_sweep_could_touch() {
        // A single box far to one side: a ray misses it, a swept sphere of radius 2 does not.
        let tree = Bvh::build(vec![(
            Aabb::new(Vec3::new(5.0, 1.5, -1.0), Vec3::new(7.0, 3.0, 1.0)),
            Layers::ALL,
            7_u32,
        )]);
        let ray = Ray::new(Vec3::ZERO, Vec3::X).unwrap();
        let mut candidates = 0;
        tree.query_ray(&ray, 20.0, Layers::ALL, 0.0, &mut |_, _| {
            candidates += 1;
            None
        });
        assert_eq!(candidates, 0, "a plain ray passes below the box");
        tree.query_ray(&ray, 20.0, Layers::ALL, 2.0, &mut |_, _| {
            candidates += 1;
            None
        });
        assert_eq!(
            candidates, 1,
            "a ray fattened by the sphere's radius reaches it"
        );
    }

    #[test]
    fn the_same_input_gives_the_same_order_of_visits_every_time() {
        let mut rng = Lcg(4);
        let items = random_boxes(&mut rng, 300);
        let order = |items: &[(Aabb, Layers, u32)]| {
            let tree = Bvh::build(items.to_vec());
            let mut visited = Vec::new();
            tree.query_aabb(
                &Aabb::new(Vec3::splat(-30.0), Vec3::splat(30.0)),
                Layers::ALL,
                &mut |v| visited.push(*v),
            );
            visited
        };
        assert_eq!(order(&items), order(&items));
        let mut shuffled = items.clone();
        shuffled.reverse();
        let (mut a, mut b) = (order(&items), order(&shuffled));
        a.sort_unstable();
        b.sort_unstable();
        assert_eq!(
            a, b,
            "a different input order changes the visiting order but never the set"
        );
    }

    #[test]
    fn boxes_with_identical_bounds_do_not_break_the_build() {
        let same = Aabb::new(Vec3::ZERO, Vec3::ONE);
        let items: Vec<_> = (0..200_u32).map(|i| (same, Layers::ALL, i)).collect();
        let tree = Bvh::build(items);
        tree.check().unwrap();
        assert!(tree.depth() < 20, "{}", tree.depth());
        let mut seen = 0;
        tree.query_aabb(&same, Layers::ALL, &mut |_| seen += 1);
        assert_eq!(seen, 200);
    }

    // ---- triangles and meshes ---------------------------------------------------------------

    #[test]
    fn a_triangle_is_hit_inside_and_missed_outside_from_both_sides() {
        let t = Triangle::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(4.0, 0.0, 0.0),
            Vec3::new(0.0, 4.0, 0.0),
        );
        let down = Ray::new(Vec3::new(1.0, 1.0, 5.0), Vec3::NEG_Z).unwrap();
        let h = t.raycast(&down, 100.0).unwrap();
        assert!((h.t - 5.0).abs() < 1e-5 && h.normal == Vec3::Z, "{h:?}");
        let up = Ray::new(Vec3::new(1.0, 1.0, -5.0), Vec3::Z).unwrap();
        let h = t.raycast(&up, 100.0).unwrap();
        assert!(
            (h.t - 5.0).abs() < 1e-5 && h.normal == Vec3::NEG_Z,
            "the normal faces the ray: {h:?}"
        );
        assert!(
            t.raycast(
                &Ray::new(Vec3::new(3.0, 3.0, 5.0), Vec3::NEG_Z).unwrap(),
                100.0
            )
            .is_none(),
            "outside the hypotenuse"
        );
        assert!(
            t.raycast(&Ray::new(Vec3::new(1.0, 1.0, 5.0), Vec3::X).unwrap(), 100.0)
                .is_none(),
            "parallel to it"
        );
        assert!(t.raycast(&down, 4.9).is_none(), "too short");
        assert!(
            t.raycast(
                &Ray::new(Vec3::new(1.0, 1.0, -5.0), Vec3::NEG_Z).unwrap(),
                100.0
            )
            .is_none(),
            "pointing away"
        );
        assert!(
            Triangle::new(Vec3::ZERO, Vec3::X, Vec3::X * 2.0).is_degenerate(),
            "three points on a line"
        );
    }

    #[test]
    fn the_closest_point_of_a_triangle_is_on_it_and_nearest() {
        let t = Triangle::new(
            Vec3::ZERO,
            Vec3::new(4.0, 0.0, 0.0),
            Vec3::new(0.0, 4.0, 0.0),
        );
        assert!(
            (t.closest_point(Vec3::new(1.0, 1.0, 3.0)) - Vec3::new(1.0, 1.0, 0.0)).length() < 1e-5,
            "above the face"
        );
        assert!(
            (t.closest_point(Vec3::new(-2.0, -2.0, 1.0)) - Vec3::ZERO).length() < 1e-5,
            "beyond a vertex"
        );
        assert!(
            (t.closest_point(Vec3::new(3.0, 3.0, 0.0)) - Vec3::new(2.0, 2.0, 0.0)).length() < 1e-5,
            "beyond the hypotenuse"
        );
        let mut rng = Lcg(9);
        for _ in 0..500 {
            let (a, b, c) = (rng.vec(3.0), rng.vec(3.0), rng.vec(3.0));
            let tri = Triangle::new(a, b, c);
            if tri.is_degenerate() {
                continue;
            }
            let p = rng.vec(6.0);
            let q = tri.closest_point(p);
            // Nothing sampled on the triangle is closer.
            for i in 0..=10 {
                for j in 0..=(10 - i) {
                    let (u, v) = (i as f32 / 10.0, j as f32 / 10.0);
                    let s = a + (b - a) * u + (c - a) * v;
                    assert!((p - q).length() <= (p - s).length() + 1e-3);
                }
            }
        }
    }

    fn box_mesh(b: &Aabb) -> TriMesh {
        let c = b.corners();
        // Corner index bits: x = 1, y = 2, z = 4.
        let quads = [
            [0, 2, 3, 1],
            [4, 5, 7, 6],
            [0, 1, 5, 4],
            [2, 6, 7, 3],
            [0, 4, 6, 2],
            [1, 3, 7, 5],
        ];
        let mut indices = Vec::new();
        for q in quads {
            indices.extend([q[0], q[1], q[2], q[0], q[2], q[3]]);
        }
        TriMesh::new(&c, &indices).unwrap()
    }

    #[test]
    fn a_box_made_of_triangles_agrees_with_the_box_for_every_ray() {
        let b = Aabb::new(Vec3::new(-1.0, -2.0, -3.0), Vec3::new(2.0, 1.0, 1.5));
        let mesh = box_mesh(&b);
        assert_eq!(mesh.triangle_count(), 12);
        let mut rng = Lcg(5);
        let mut compared = 0;
        for _ in 0..3000 {
            let origin = rng.vec(8.0);
            let Some(ray) = Ray::new(origin, rng.vec(1.5) - origin) else {
                continue;
            };
            if b.contains_point(origin) {
                continue;
            }
            let want = b.raycast(&ray, 40.0);
            let got = mesh.raycast(&ray, 40.0, Layers::ALL);
            match (want, got) {
                (Some(w), Some(g)) => {
                    compared += 1;
                    assert!((w.t - g.t).abs() < 1e-3, "{} vs {}", w.t, g.t);
                    assert!(g.normal.dot(ray.dir) <= 1e-4, "faces the ray");
                    assert!((g.point - ray.point_at(g.t)).length() < 1e-3);
                }
                (None, None) => {}
                (Some(w), None) if w.t < 1e-3 => {} // grazing an edge exactly
                (None, Some(g)) if g.t > 39.99 => {}
                other => panic!("box and mesh disagree on {ray:?}: {other:?}"),
            }
        }
        assert!(compared > 300, "{compared}");
    }

    #[test]
    fn a_random_mesh_ray_query_matches_brute_force_over_the_triangles() {
        let mut rng = Lcg(6);
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for _ in 0..400 {
            let centre = rng.vec(15.0);
            let base = vertices.len() as u32;
            for _ in 0..3 {
                vertices.push(centre + rng.vec(2.0));
            }
            indices.extend([base, base + 1, base + 2]);
        }
        let mesh = TriMesh::new(&vertices, &indices).unwrap();
        let (mut hits, mut misses) = (0, 0);
        for _ in 0..800 {
            let origin = rng.vec(20.0);
            let Some(ray) = Ray::new(origin, rng.vec(10.0) - origin) else {
                continue;
            };
            let want = (0..mesh.triangle_count() as u32)
                .filter_map(|i| mesh.triangle(i).raycast(&ray, 50.0).map(|h| (h.t, i)))
                .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            let got = mesh.raycast(&ray, 50.0, Layers::ALL);
            match (want, got) {
                (Some((t, _)), Some(g)) => {
                    hits += 1;
                    assert!((g.t - t).abs() < 1e-4);
                }
                (None, None) => misses += 1,
                other => panic!("{other:?}"),
            }
        }
        assert!(hits > 80 && misses > 80, "hits {hits}, misses {misses}");
    }

    #[test]
    fn sphere_overlap_with_a_mesh_matches_brute_force() {
        let mut rng = Lcg(7);
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for _ in 0..300 {
            let centre = rng.vec(12.0);
            let base = vertices.len() as u32;
            for _ in 0..3 {
                vertices.push(centre + rng.vec(1.5));
            }
            indices.extend([base, base + 1, base + 2]);
        }
        let mesh = TriMesh::new(&vertices, &indices).unwrap();
        for _ in 0..300 {
            let (centre, radius) = (rng.vec(14.0), rng.range(0.3, 3.0));
            let mut got = Vec::new();
            mesh.query_sphere(centre, radius, Layers::ALL, &mut |i| got.push(i));
            got.sort_unstable();
            let want: Vec<u32> = (0..mesh.triangle_count() as u32)
                .filter(|i| (mesh.triangle(*i).closest_point(centre) - centre).length() <= radius)
                .collect();
            assert_eq!(got, want);
        }
    }

    /// A sphere swept along `delta` against one triangle, checked against sampling: at the hit the
    /// sphere touches the triangle, and earlier it did not overlap it.
    #[test]
    fn a_sphere_sweep_against_a_triangle_matches_sampled_distance() {
        let mut rng = Lcg(12);
        let (mut hits, mut misses) = (0, 0);
        for _ in 0..3000 {
            let tri = Triangle::new(rng.vec(3.0), rng.vec(3.0), rng.vec(3.0));
            if tri.is_degenerate() {
                continue;
            }
            let radius = rng.range(0.1, 1.0);
            let center = rng.vec(6.0);
            // Aimed near the triangle, so that hits and misses are both common.
            let aim = (tri.a + tri.b + tri.c) / 3.0 + rng.vec(1.5);
            let delta = (aim - center) * rng.range(0.6, 1.6);
            let distance = |p: Vec3| (tri.closest_point(p) - p).length();
            match tri.sweep_sphere(center, radius, delta) {
                Some(h) if !h.overlapping_at_start => {
                    hits += 1;
                    assert!((0.0..=1.0).contains(&h.t));
                    let at = center + delta * h.t;
                    assert!(
                        (distance(at) - radius).abs() < 3e-3,
                        "touching at the hit: {}",
                        distance(at) - radius
                    );
                    assert!((h.normal.length() - 1.0).abs() < 1e-3);
                    assert!(h.normal.dot(delta) <= 1e-3, "the normal opposes the motion");
                    for i in 0..64 {
                        assert!(
                            distance(center + delta * (h.t * i as f32 / 64.0)) > radius - 3e-3,
                            "overlapping before the hit"
                        );
                    }
                }
                Some(h) => {
                    hits += 1;
                    assert!(h.t == 0.0 && distance(center) < radius + 1e-3);
                }
                None => {
                    misses += 1;
                    for i in 0..=300 {
                        assert!(
                            distance(center + delta * (i as f32 / 300.0)) > radius - 3e-3,
                            "a miss, yet overlapping"
                        );
                    }
                }
            }
        }
        assert!(hits > 150 && misses > 150, "hits {hits}, misses {misses}");
    }

    #[test]
    fn a_sphere_swept_at_a_mesh_stops_on_the_floor_and_slides_past_an_edge() {
        let floor = [
            Vec3::new(-5.0, 0.0, -5.0),
            Vec3::new(5.0, 0.0, -5.0),
            Vec3::new(5.0, 0.0, 5.0),
            Vec3::new(-5.0, 0.0, 5.0),
        ];
        let mesh = TriMesh::new(&floor, &[0, 2, 1, 0, 3, 2]).unwrap();
        let hit = mesh
            .sweep_sphere(
                Vec3::new(1.0, 4.0, 1.0),
                0.5,
                Vec3::new(0.0, -10.0, 0.0),
                Layers::ALL,
            )
            .unwrap();
        assert!(
            (hit.0.t - 0.35).abs() < 1e-4 && hit.0.normal == Vec3::Y,
            "{hit:?}"
        );
        assert!(
            (hit.0.position.y - 0.5).abs() < 1e-4,
            "the centre stops one radius above the floor"
        );
        // Falling just past the floor's edge (more than a radius outside) misses it.
        assert!(
            mesh.sweep_sphere(
                Vec3::new(6.0, 4.0, 0.0),
                0.5,
                Vec3::new(0.0, -10.0, 0.0),
                Layers::ALL
            )
            .is_none()
        );
        // Just inside the radius of the edge it touches the rounded edge, and the normal leans outward.
        let edge = mesh
            .sweep_sphere(
                Vec3::new(5.3, 4.0, 0.0),
                0.5,
                Vec3::new(0.0, -10.0, 0.0),
                Layers::ALL,
            )
            .unwrap();
        assert!(edge.0.normal.x > 0.3 && edge.0.normal.y > 0.3, "{edge:?}");
        let mut mesh = mesh;
        mesh.set_all_layers(Layers::bit(3));
        let down = Vec3::new(0.0, -10.0, 0.0);
        assert!(
            mesh.sweep_sphere(Vec3::new(1.0, 4.0, 1.0), 0.5, down, Layers::bit(2))
                .is_none(),
            "another layer is not seen"
        );
        assert!(
            mesh.sweep_sphere(Vec3::new(1.0, 4.0, 1.0), 0.5, down, Layers::bit(3))
                .is_some()
        );
    }

    #[test]
    fn meshes_refuse_bad_data_and_drop_degenerate_triangles() {
        let v = [Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::new(2.0, 0.0, 0.0)];
        assert!(matches!(
            TriMesh::new(&v, &[0, 1]),
            Err(MeshError::IndexCountNotMultipleOfThree(2))
        ));
        assert!(matches!(
            TriMesh::new(&v, &[0, 1, 9]),
            Err(MeshError::IndexOutOfRange {
                index: 9,
                vertices: 4
            })
        ));
        let bad = [Vec3::ZERO, Vec3::X, Vec3::new(f32::NAN, 0.0, 0.0)];
        assert!(matches!(
            TriMesh::new(&bad, &[0, 1, 2]),
            Err(MeshError::NonFiniteVertex(2))
        ));
        // 0, 1, 3 lie on a line: dropped and counted; the good triangle stays.
        let mesh = TriMesh::new(&v, &[0, 1, 2, 0, 1, 3]).unwrap();
        assert_eq!((mesh.triangle_count(), mesh.dropped_degenerate()), (1, 1));
        let empty = TriMesh::new(&[], &[]).unwrap();
        assert_eq!(empty.triangle_count(), 0);
        assert!(
            empty
                .raycast(&Ray::new(Vec3::ZERO, Vec3::X).unwrap(), 10.0, Layers::ALL)
                .is_none()
        );
    }

    #[test]
    fn triangles_carry_layers_and_the_mask_filters_them() {
        let v = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(4.0, 0.0, 0.0),
            Vec3::new(0.0, 4.0, 0.0),
        ];
        let mut mesh = TriMesh::new(&v, &[0, 1, 2]).unwrap();
        mesh.set_all_layers(Layers::bit(3));
        let ray = Ray::new(Vec3::new(1.0, 1.0, 5.0), Vec3::NEG_Z).unwrap();
        assert!(mesh.raycast(&ray, 100.0, Layers::bit(3)).is_some());
        assert!(
            mesh.raycast(&ray, 100.0, Layers::bit(2)).is_none(),
            "another layer is not seen"
        );
        assert!(mesh.raycast(&ray, 100.0, Layers::ALL).is_some());
    }
}
