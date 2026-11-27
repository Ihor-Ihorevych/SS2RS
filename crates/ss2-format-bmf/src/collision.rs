//! The collision mesh of a `CMesh` (`bmf.md`, "Collision mesh" and "Collision tree", T01-055).
//!
//! A `CCollisionMesh` is a triangle soup with one collision surface id per triangle and a flat
//! axis-split tree over the triangles. Its vertices usually live in the render mesh's vertex
//! buffers (`.19` set: a holder number, byte offsets of the position and normal streams and a
//! vertex count); otherwise in its own position array (`.12`, empty in all retail files).

use ss2_core::Result;
use ss2_meta::{Container, Decoded, MetaDeserialize, ObjectRef};

use crate::{Aabb, Buffers, MaterialRef, RawBox, RawPath, bad, code, floats, material_ref};

/// The tree's split positions are stored in units of 1/16,384 of the mesh box extent.
const SPLIT_UNITS: f32 = 16_384.0;

/// A node field that says "none" (`0xFFFF`): both split words of a leaf.
const NONE16: u16 = 0xFFFF;

/// One collision surface (`CCollisionMeshSurface`): what a triangle's surface id refers to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollisionSurface {
    /// `.4`: the surface is double-sided.
    pub double_sided: bool,
    /// `.5`: the material attributes (in the retail files an external `.mat` physical material).
    pub material: MaterialRef,
}

/// One node of the flat collision tree (`CCollisionMeshNode`: four `u16`), rooted at index 0.
///
/// A node may hold triangles (a range of [`CollisionMesh::node_triangles`]) and, unless it is a
/// leaf, two children split along one axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollisionNode {
    /// `.1`: the axis in the upper two bits (0 = X, 1 = Y, 2 = Z), the split position in the lower
    /// 14 bits; `0xFFFF` for a leaf.
    pub split: u16,
    /// `.2`: the index of the "below" child; the "above" child follows it. `0xFFFF` for a leaf.
    pub child: u16,
    /// `.3`: the number of triangle references of this node.
    pub triangle_count: u16,
    /// `.4`: the first of them in [`CollisionMesh::node_triangles`].
    pub first_triangle: u16,
}

impl CollisionNode {
    /// `true` for a leaf (both split words `0xFFFF`).
    pub fn is_leaf(&self) -> bool {
        self.split == NONE16 && self.child == NONE16
    }

    /// The split plane of an internal node: the axis (0 = X, 1 = Y, 2 = Z) and the coordinate,
    /// decoded against the full mesh box `bounds` (the quantization domain at every depth).
    pub fn split_plane(&self, bounds: &Aabb) -> Option<(usize, f32)> {
        if self.is_leaf() {
            return None;
        }
        let axis = usize::from(self.split >> 14);
        let units = f32::from(self.split & 0x3FFF);
        let (lo, hi) = (*bounds.min.get(axis)?, *bounds.max.get(axis)?);
        Some((axis, lo + units / SPLIT_UNITS * (hi - lo)))
    }

    /// The node indices of the "below" and "above" children of an internal node.
    pub fn children(&self) -> Option<(usize, usize)> {
        (!self.is_leaf()).then(|| {
            let below = usize::from(self.child);
            (below, below + 1)
        })
    }

    /// The range of [`CollisionMesh::node_triangles`] this node holds.
    pub fn triangle_range(&self) -> std::ops::Range<usize> {
        let start = usize::from(self.first_triangle);
        start..start + usize::from(self.triangle_count)
    }
}

/// A decoded `CCollisionMesh`, in the mesh's own object space (like the render LODs).
#[derive(Clone, Debug, PartialEq)]
pub struct CollisionMesh {
    /// The vertices.
    pub positions: Vec<[f32; 3]>,
    /// One normal per vertex, when the vertices come from the render geometry (empty otherwise).
    pub normals: Vec<[f32; 3]>,
    /// The triangles: three indices into [`CollisionMesh::positions`] each (stored as `u16` in
    /// most meshes, as `u32` in one retail mesh).
    pub triangles: Vec<[u32; 3]>,
    /// One collision surface id per triangle, an index into [`CollisionMesh::surfaces`]. **Not
    /// always in range** (one retail mesh has ids past its table, `bmf.md`): use
    /// [`CollisionMesh::surface_of`].
    pub surface_ids: Vec<u8>,
    /// The collision surface table.
    pub surfaces: Vec<CollisionSurface>,
    /// One byte of packed edge/vertex flags per triangle (the individual bits are not specified),
    /// or empty: some meshes embedded in models carry none.
    pub edge_flags: Vec<u8>,
    /// The mesh box (`.18`): the quantization domain of the tree's split positions.
    pub bounds: Aabb,
    /// The tree nodes, rooted at index 0 (empty when there are no triangles).
    pub nodes: Vec<CollisionNode>,
    /// The triangle references of the nodes: triangle numbers, ranges given by
    /// [`CollisionNode::triangle_range`]. A triangle may be referenced by several nodes.
    pub node_triangles: Vec<u32>,
}

impl CollisionMesh {
    /// Number of triangles.
    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    /// `true` if the collision mesh has no triangles (three retail meshes, `bmf.md`).
    pub fn is_empty(&self) -> bool {
        self.triangles.is_empty()
    }

    /// The collision surface of triangle `triangle`, `None` if the triangle does not exist or its
    /// surface id is outside the table.
    pub fn surface_of(&self, triangle: usize) -> Option<&CollisionSurface> {
        self.surfaces
            .get(usize::from(*self.surface_ids.get(triangle)?))
    }

    /// The triangle numbers that node `node` holds (empty for a node that does not exist).
    pub fn node_triangles_of(&self, node: usize) -> &[u32] {
        self.nodes
            .get(node)
            .and_then(|n| self.node_triangles.get(n.triangle_range()))
            .unwrap_or(&[])
    }
}

#[derive(MetaDeserialize)]
struct RawVector {
    #[meta(owner = "Vector3f", name = "1")]
    x: f32,
    #[meta(owner = "Vector3f", name = "2")]
    y: f32,
    #[meta(owner = "Vector3f", name = "3")]
    z: f32,
}

#[derive(MetaDeserialize)]
#[meta(class = "CCollisionMeshSurface")]
struct RawSurface {
    #[meta(name = "4")]
    double_sided: bool,
    #[meta(name = "5")]
    material: ObjectRef,
}

#[derive(MetaDeserialize)]
#[meta(class = "CCollisionMeshNode")]
struct RawNode {
    #[meta(name = "1")]
    split: u16,
    #[meta(name = "2")]
    child: u16,
    #[meta(name = "3")]
    count: u16,
    #[meta(name = "4")]
    first: u16,
}

#[derive(MetaDeserialize)]
#[meta(class = "CCollisionMesh")]
pub(crate) struct RawCollisionMesh {
    #[meta(name = "12", default)]
    own_positions: Vec<RawVector>,
    #[meta(name = "13", default)]
    indices16: Vec<u16>,
    #[meta(name = "14", default)]
    indices32: Vec<u32>,
    #[meta(name = "9", default)]
    surface_ids: ss2_meta::Bytes,
    #[meta(name = "11", default)]
    surfaces: Vec<RawSurface>,
    #[meta(name = "26", default)]
    edge_flags: ss2_meta::Bytes,
    #[meta(name = "16", default)]
    nodes: Vec<RawNode>,
    #[meta(name = "17", default)]
    node_triangles: Vec<i64>,
    #[meta(name = "18")]
    bounds: RawBox,
    #[meta(name = "19")]
    render_geometry: bool,
    #[meta(name = "20")]
    holder: i64,
    #[meta(name = "21")]
    position_offset: i64,
    #[meta(name = "23")]
    normal_offset: i64,
    #[meta(name = "22")]
    vertex_count: i64,
}

/// Read the `CCollisionMesh` object `id`, taking render-geometry vertices from `vertex_buffers`.
pub(crate) fn read(
    container: &Container<'_>,
    decoded: &Decoded,
    vertex_buffers: &Buffers<'_>,
    id: u32,
) -> Result<CollisionMesh> {
    let raw: RawCollisionMesh = decoded.typed(id)?;

    let (positions, normals) = if raw.render_geometry {
        let count = usize::try_from(raw.vertex_count)
            .map_err(|_| bad(format!("collision vertex count {}", raw.vertex_count)))?;
        let stream = |offset: i64, what: &str| -> Result<Vec<[f32; 3]>> {
            let path = RawPath {
                code: code::FLOAT3,
                buffer: raw.holder,
                offset,
            };
            vertex_buffers
                .stream(&path, 12, count)?
                .map(floats::<3>)
                .ok_or_else(|| bad(format!("the collision mesh has no {what} stream")))
        };
        (
            stream(raw.position_offset, "position")?,
            stream(raw.normal_offset, "normal")?,
        )
    } else {
        let own = raw.own_positions.iter().map(|v| [v.x, v.y, v.z]).collect();
        (own, Vec::new())
    };

    let indices: Vec<u32> = match (raw.indices16.is_empty(), raw.indices32.is_empty()) {
        (_, true) => raw.indices16.iter().map(|&i| u32::from(i)).collect(),
        (true, false) => raw.indices32,
        (false, false) => {
            return Err(bad(
                "the collision mesh has both a 16-bit and a 32-bit index list",
            ));
        }
    };
    if !indices.len().is_multiple_of(3) {
        return Err(bad(format!(
            "the collision index list has {} entries, not whole triangles",
            indices.len()
        )));
    }
    if let Some(i) = indices
        .iter()
        .find(|&&i| usize::try_from(i).map_or(true, |i| i >= positions.len()))
    {
        return Err(bad(format!(
            "collision index {i} is not below the vertex count {}",
            positions.len()
        )));
    }
    let triangles: Vec<[u32; 3]> = indices.as_chunks::<3>().0.to_vec();
    let n = triangles.len();

    // The edge flags may be missing altogether: 12 meshes embedded in retail `.mdl` models have
    // none (an older class layout without `.26`, or an empty array).
    for (what, len, may_be_empty) in [
        ("surface id", raw.surface_ids.0.len(), false),
        ("edge flag", raw.edge_flags.0.len(), true),
    ] {
        if len != n && !(may_be_empty && len == 0) {
            return Err(bad(format!(
                "the collision mesh has {n} triangles and {len} {what} entries"
            )));
        }
    }

    let node_triangles = raw
        .node_triangles
        .iter()
        .map(|&t| {
            u32::try_from(t)
                .ok()
                .filter(|&t| (t as usize) < n)
                .ok_or_else(|| bad(format!("a collision tree reference to triangle {t} of {n}")))
        })
        .collect::<Result<Vec<u32>>>()?;
    let nodes: Vec<CollisionNode> = raw
        .nodes
        .iter()
        .map(|r| CollisionNode {
            split: r.split,
            child: r.child,
            triangle_count: r.count,
            first_triangle: r.first,
        })
        .collect();
    for (index, node) in nodes.iter().enumerate() {
        if node.triangle_range().end > node_triangles.len() {
            return Err(bad(format!(
                "collision tree node {index} refers past the {} triangle references",
                node_triangles.len()
            )));
        }
        if node.is_leaf() {
            continue;
        }
        if node.split >> 14 > 2 {
            return Err(bad(format!("collision tree node {index} splits axis 3")));
        }
        // Children follow their parent: this keeps a traversal from looping.
        let below = usize::from(node.child);
        if below <= index || below + 1 >= nodes.len() {
            return Err(bad(format!(
                "collision tree node {index} has children {below} and {} of {} nodes",
                below + 1,
                nodes.len()
            )));
        }
    }

    let surfaces = raw
        .surfaces
        .iter()
        .map(|s| CollisionSurface {
            double_sided: s.double_sided,
            material: material_ref(container, decoded, s.material),
        })
        .collect();
    let b = &raw.bounds;
    Ok(CollisionMesh {
        positions,
        normals,
        triangles,
        surface_ids: raw.surface_ids.0,
        surfaces,
        edge_flags: raw.edge_flags.0,
        bounds: Aabb {
            min: [b.x0, b.y0, b.z0],
            max: [b.x1, b.y1, b.z1],
        },
        nodes,
        node_triangles,
    })
}
