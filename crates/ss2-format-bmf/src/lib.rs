//! ss2-format-bmf — the `.bmf` mesh loader (roadmap T02-014; spec `docs/re/bmf.md`).
//!
//! A `.bmf` holds one `CMesh`: a render mesh with 1 to 4 LODs, each with surfaces, and sometimes a
//! collision mesh. The vertex and index data sit in raw byte buffers; every surface says where its
//! streams start inside them (an *element path*: element code, buffer number, byte offset).
//! [`Mesh::from_bytes`] cuts those streams out into typed arrays: indices, positions, normals,
//! optional tangents, UV sets, and optional bone weights and indices with the bone palette.
//!
//! What is **not** done here: materials are only identified ([`MaterialRef`]; the material loader
//! is T02-018), morph maps and the collision mesh are not read (their layout is open, T01-055), and
//! the editor-side records after `EDOB` are skipped.
//!
//! ```ignore
//! let mesh = ss2_format_bmf::Mesh::from_bytes(&bytes)?;
//! for lod in mesh.lods.iter().filter(|l| l.is_for_pc()) {
//!     for surface in &lod.surfaces {
//!         // surface.indices, surface.positions, surface.uv_sets[0], ...
//!     }
//! }
//! ```

use ss2_assets::{AssetLoader, LoadContext};
use ss2_core::{Error, Result};
use ss2_meta::{Container, Decoded, MetaDeserialize, ObjectRef};

const WHAT: &str = ".bmf mesh";

/// Element codes of the stream element paths (`bmf.md`).
mod code {
    pub const INDEX: i64 = 135;
    pub const FLOAT3: i64 = 133;
    pub const FLOAT2: i64 = 132;
    pub const FLOAT4: i64 = 134;
    pub const BYTE4: i64 = 128;
}

/// Bit 0 of a LOD's platform mask: the LOD is drawn on a PC.
pub const PLATFORM_PC: u32 = 0x1;

/// An axis-aligned box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    /// Smallest corner.
    pub min: [f32; 3],
    /// Largest corner.
    pub max: [f32; 3],
}

/// What a surface uses as its material.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MaterialRef {
    /// None (88 of 353 surfaces in the retail meshes; what the engine draws is unknown).
    None,
    /// A `CShaderPreset` object inside this file (the object id).
    Embedded(u32),
    /// A `.mtr` file (the path as listed in the file's `RFIL`).
    External(String),
}

/// One draw call: a triangle list with its vertex streams.
#[derive(Clone, Debug, PartialEq)]
pub struct Surface {
    /// Triangle list: three indices per triangle, local to this surface (all below
    /// `positions.len()`).
    pub indices: Vec<u16>,
    /// One position per vertex, in the mesh's object space.
    pub positions: Vec<[f32; 3]>,
    /// One unit normal per vertex.
    pub normals: Vec<[f32; 3]>,
    /// Tangents (xyz and the sign of the bitangent in `w`), for the surfaces that have them.
    pub tangents: Option<Vec<[f32; 4]>>,
    /// The UV streams (1 to 5), each one `[u, v]` per vertex, in slot order.
    pub uv_sets: Vec<Vec<[f32; 2]>>,
    /// The UV map id of each stream of [`Surface::uv_sets`].
    pub uv_map_ids: Vec<u32>,
    /// Four bone weights per vertex (sum about 255; all zero = not skinned), skinned surfaces only.
    pub bone_weights: Option<Vec<[u8; 4]>>,
    /// Four bone indices per vertex into [`Surface::bone_palette`], skinned surfaces only.
    pub bone_indices: Option<Vec<[u8; 4]>>,
    /// The bone palette: presumably skeleton bone numbers (needs the `.skl` loader to confirm).
    pub bone_palette: Vec<i64>,
    /// The material.
    pub material: MaterialRef,
}

impl Surface {
    /// Number of triangles.
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// `true` if the surface carries bone weights.
    pub fn is_skinned(&self) -> bool {
        self.bone_weights.is_some()
    }
}

/// One level of detail.
#[derive(Clone, Debug, PartialEq)]
pub struct Lod {
    /// Bounding box of all positions of the LOD.
    pub bounds: Aabb,
    /// Distance from which this LOD is used (ascending with the LOD number).
    pub switch_distance: f32,
    /// `EPlatform` mask: bit 0 PC, bit 1 Xbox (`0xFFFFFFFF` all platforms).
    pub platform_mask: u32,
    /// The surfaces (the files have one section per LOD, so these are its surfaces).
    pub surfaces: Vec<Surface>,
}

impl Lod {
    /// `true` if a PC draws this LOD (bit 0 of the platform mask).
    pub fn is_for_pc(&self) -> bool {
        self.platform_mask & PLATFORM_PC != 0
    }

    /// Total triangles of the LOD.
    pub fn triangle_count(&self) -> usize {
        self.surfaces.iter().map(Surface::triangle_count).sum()
    }
}

/// A decoded `.bmf` file.
#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    /// The LODs, nearest first.
    pub lods: Vec<Lod>,
    /// `CRenderMesh.7`, `.8`, `.9`: three distances (probably fade and view distances; meaning
    /// unconfirmed, `bmf.md`).
    pub distances: [f32; 3],
    /// `true` if the file has a `CCollisionMesh` (not read).
    pub has_collision_mesh: bool,
}

#[derive(MetaDeserialize)]
#[meta(class = "CMesh")]
struct RawMesh {
    #[meta(owner = "CMesh", name = "1")]
    render: ObjectRef,
    #[meta(owner = "CMesh", name = "2")]
    collision: ObjectRef,
}

#[derive(MetaDeserialize)]
#[meta(class = "CRenderMesh")]
struct RawRenderMesh {
    #[meta(owner = "CRenderMesh", name = "5")]
    lods: Vec<RawLod>,
    #[meta(owner = "CRenderMesh", name = "12")]
    index_buffers: Vec<RawHolder>,
    #[meta(owner = "CRenderMesh", name = "13")]
    vertex_buffers: Vec<RawHolder>,
    #[meta(owner = "CRenderMesh", name = "7")]
    d1: f32,
    #[meta(owner = "CRenderMesh", name = "8")]
    d2: f32,
    #[meta(owner = "CRenderMesh", name = "9")]
    d3: f32,
}

#[derive(MetaDeserialize)]
#[meta(class = "VBufferHolder")]
struct RawHolder {
    #[meta(name = "1")]
    buffer: ObjectRef,
}

#[derive(MetaDeserialize)]
#[meta(class = "GFXHANDLE")]
struct RawBuffer {
    #[meta(name = "2")]
    bytes: ss2_meta::Bytes,
}

#[derive(MetaDeserialize)]
#[meta(class = "Box3f")]
struct RawBox {
    #[meta(name = "1")]
    x0: f32,
    #[meta(name = "2")]
    y0: f32,
    #[meta(name = "3")]
    z0: f32,
    #[meta(name = "4")]
    x1: f32,
    #[meta(name = "5")]
    y1: f32,
    #[meta(name = "6")]
    z1: f32,
}

#[derive(MetaDeserialize)]
#[meta(class = "CRenderMeshLOD")]
struct RawLod {
    #[meta(name = "3")]
    bounds: RawBox,
    #[meta(name = "5")]
    sections: Vec<RawSection>,
    #[meta(name = "6")]
    switch_distance: f32,
    #[meta(name = "7")]
    platform_mask: u32,
}

#[derive(MetaDeserialize)]
#[meta(class = "CRenderMeshSection")]
struct RawSection {
    #[meta(name = "1")]
    surfaces: Vec<RawSurface>,
}

#[derive(MetaDeserialize)]
#[meta(class = "CRenderMeshElementPath")]
struct RawPath {
    #[meta(name = "1")]
    code: i64,
    #[meta(name = "2")]
    buffer: i64,
    #[meta(name = "3")]
    offset: i64,
}

#[derive(MetaDeserialize)]
#[meta(class = "CRenderMeshSurface")]
struct RawSurface {
    #[meta(name = "2")]
    triangles: i64,
    #[meta(name = "3")]
    vertices: i64,
    #[meta(name = "4")]
    uv_count: i64,
    #[meta(name = "6")]
    index_path: RawPath,
    #[meta(name = "7")]
    position_path: RawPath,
    #[meta(name = "8")]
    normal_path: RawPath,
    #[meta(name = "9")]
    tangent_path: RawPath,
    #[meta(name = "10")]
    weight_path: RawPath,
    #[meta(name = "11")]
    bone_index_path: RawPath,
    #[meta(name = "12")]
    uv_map_ids: Vec<i64>,
    #[meta(name = "14")]
    uv_paths: Vec<RawPath>,
    #[meta(name = "16")]
    material: ObjectRef,
    #[meta(name = "18")]
    bone_palette: Vec<i64>,
}

fn bad(detail: impl Into<String>) -> Error {
    Error::invalid(WHAT, detail)
}

/// The bytes of buffer `number` of a holder list, or an error if the slot is unused or missing.
struct Buffers<'a> {
    list: Vec<Option<ss2_meta::Bytes>>,
    what: &'a str,
}

impl Buffers<'_> {
    fn new<'a>(decoded: &Decoded, holders: &[RawHolder], what: &'a str) -> Result<Buffers<'a>> {
        let mut list = Vec::with_capacity(holders.len());
        for h in holders {
            list.push(match h.buffer.0 {
                None => None,
                Some(id) => {
                    let raw: RawBuffer = decoded.typed(id)?;
                    Some(raw.bytes)
                }
            });
        }
        Ok(Buffers { list, what })
    }

    /// `count` elements of `size` bytes at `path`; `None` if the path says "absent".
    fn stream(&self, path: &RawPath, size: usize, count: usize) -> Result<Option<&[u8]>> {
        if path.code == 0 || path.buffer == 255 || path.offset == 0xFFFF_FFFF {
            return Ok(None);
        }
        let buffer = usize::try_from(path.buffer)
            .ok()
            .and_then(|n| self.list.get(n))
            .and_then(Option::as_ref)
            .ok_or_else(|| {
                bad(format!(
                    "a stream refers to {} buffer {} which does not exist",
                    self.what, path.buffer
                ))
            })?;
        let start = usize::try_from(path.offset)
            .map_err(|_| bad(format!("stream offset {}", path.offset)))?;
        let len = count
            .checked_mul(size)
            .ok_or_else(|| bad("a stream is too long"))?;
        buffer
            .0
            .get(start..start.checked_add(len).ok_or_else(|| bad("a stream is too long"))?)
            .map(Some)
            .ok_or_else(|| bad(format!("a stream of {len} bytes at offset {start} does not fit in the {} bytes of buffer {}", buffer.0.len(), path.buffer)))
    }
}

fn floats<const N: usize>(bytes: &[u8]) -> Vec<[f32; N]> {
    bytes
        .chunks_exact(4 * N)
        .map(|c| {
            let mut out = [0.0f32; N];
            for (i, o) in out.iter_mut().enumerate() {
                *o = f32::from_le_bytes([c[4 * i], c[4 * i + 1], c[4 * i + 2], c[4 * i + 3]]);
            }
            out
        })
        .collect()
}

fn bytes4(bytes: &[u8]) -> Vec<[u8; 4]> {
    bytes.as_chunks::<4>().0.to_vec()
}

fn expect_code(path: &RawPath, expected: i64, what: &str) -> Result<()> {
    if path.code != 0 && path.buffer != 255 && path.code != expected {
        return Err(bad(format!(
            "the {what} stream has element code {}, {expected} expected",
            path.code
        )));
    }
    Ok(())
}

impl Mesh {
    /// Read a `.bmf` file.
    ///
    /// Checks the rules of `bmf.md`: every stream and the index list fit inside their buffers, the
    /// element codes are the known ones, every index is smaller than the vertex count, the number
    /// of UV streams matches, bone indices stay inside the palette. Counts are bounded by the
    /// buffer sizes before anything is allocated.
    ///
    /// # Errors
    /// [`Error::Invalid`] naming what is wrong.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let container = Container::parse(bytes)?;
        if container.header.format_version != 8 {
            return Err(bad(format!(
                "format version {}",
                container.header.format_version
            )));
        }
        let decoded = container.decode_objects()?;
        let mesh_object = decoded
            .objects
            .iter()
            .find(|o| decoded.types.type_name(o.type_id) == Some("CMesh"))
            .ok_or_else(|| bad("the file has no CMesh object"))?;
        let raw_mesh: RawMesh =
            ss2_meta::FromValue::from_value(&mesh_object.value, &decoded.types)?;
        let render_id = raw_mesh
            .render
            .0
            .ok_or_else(|| bad("the mesh has no render mesh"))?;
        let render: RawRenderMesh = decoded.typed(render_id)?;
        let index_buffers = Buffers::new(&decoded, &render.index_buffers, "index")?;
        let vertex_buffers = Buffers::new(&decoded, &render.vertex_buffers, "vertex")?;

        let mut lods = Vec::with_capacity(render.lods.len());
        for raw_lod in &render.lods {
            let mut surfaces = Vec::new();
            for section in &raw_lod.sections {
                for s in &section.surfaces {
                    surfaces.push(Self::surface(
                        &container,
                        &decoded,
                        &index_buffers,
                        &vertex_buffers,
                        s,
                    )?);
                }
            }
            let b = &raw_lod.bounds;
            lods.push(Lod {
                bounds: Aabb {
                    min: [b.x0, b.y0, b.z0],
                    max: [b.x1, b.y1, b.z1],
                },
                switch_distance: raw_lod.switch_distance,
                platform_mask: raw_lod.platform_mask,
                surfaces,
            });
        }
        Ok(Self {
            lods,
            distances: [render.d1, render.d2, render.d3],
            has_collision_mesh: raw_mesh.collision.0.is_some(),
        })
    }

    fn surface(
        container: &Container<'_>,
        decoded: &Decoded,
        index_buffers: &Buffers<'_>,
        vertex_buffers: &Buffers<'_>,
        s: &RawSurface,
    ) -> Result<Surface> {
        let triangles = usize::try_from(s.triangles).map_err(|_| bad("negative triangle count"))?;
        let vertices = usize::try_from(s.vertices).map_err(|_| bad("negative vertex count"))?;
        // The element codes are checked before the sizes (an unknown code means unknown sizes).
        expect_code(&s.index_path, code::INDEX, "index")?;
        expect_code(&s.position_path, code::FLOAT3, "position")?;
        expect_code(&s.normal_path, code::FLOAT3, "normal")?;
        expect_code(&s.tangent_path, code::FLOAT4, "tangent")?;
        expect_code(&s.weight_path, code::BYTE4, "bone weight")?;
        expect_code(&s.bone_index_path, code::BYTE4, "bone index")?;

        let index_count = triangles
            .checked_mul(3)
            .ok_or_else(|| bad("too many triangles"))?;
        let index_bytes = index_buffers
            .stream(&s.index_path, 2, index_count)?
            .ok_or_else(|| bad("a surface has no index list"))?;
        let indices: Vec<u16> = index_bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c))
            .collect();
        if let Some(i) = indices.iter().find(|&&i| usize::from(i) >= vertices) {
            return Err(bad(format!(
                "index {i} is not below the vertex count {vertices}"
            )));
        }
        let positions = floats::<3>(
            vertex_buffers
                .stream(&s.position_path, 12, vertices)?
                .ok_or_else(|| bad("a surface has no positions"))?,
        );
        let normals = floats::<3>(
            vertex_buffers
                .stream(&s.normal_path, 12, vertices)?
                .ok_or_else(|| bad("a surface has no normals"))?,
        );
        let tangents = vertex_buffers
            .stream(&s.tangent_path, 16, vertices)?
            .map(floats::<4>);

        let uv_count = usize::try_from(s.uv_count).map_err(|_| bad("negative UV count"))?;
        let mut uv_sets = Vec::new();
        let mut uv_map_ids = Vec::new();
        for (slot, path) in s.uv_paths.iter().enumerate() {
            expect_code(path, code::FLOAT2, "UV")?;
            if let Some(bytes) = vertex_buffers.stream(path, 8, vertices)? {
                uv_sets.push(floats::<2>(bytes));
                uv_map_ids
                    .push(u32::try_from(s.uv_map_ids.get(slot).copied().unwrap_or(0)).unwrap_or(0));
            }
        }
        if uv_sets.len() != uv_count {
            return Err(bad(format!(
                "the surface declares {uv_count} UV streams and has {}",
                uv_sets.len()
            )));
        }

        let bone_weights = vertex_buffers
            .stream(&s.weight_path, 4, vertices)?
            .map(bytes4);
        let bone_indices = vertex_buffers
            .stream(&s.bone_index_path, 4, vertices)?
            .map(bytes4);
        if bone_weights.is_some() != bone_indices.is_some() {
            return Err(bad("bone weights and bone indices must come together"));
        }
        if let Some(bi) = &bone_indices
            && let Some(max) = bi.iter().flatten().max()
            && usize::from(*max) >= s.bone_palette.len()
        {
            return Err(bad(format!(
                "bone index {max} is outside the palette of {} bones",
                s.bone_palette.len()
            )));
        }

        let material = match s.material.0 {
            None => MaterialRef::None,
            Some(id) if decoded.object(id).is_some() => MaterialRef::Embedded(id),
            Some(id) => container
                .external_objects
                .iter()
                .find(|e| e.object_id == id)
                .and_then(|e| container.files.get(e.file_ref as usize))
                .map_or(MaterialRef::None, |f| {
                    MaterialRef::External(f.path.to_string())
                }),
        };
        Ok(Surface {
            indices,
            positions,
            normals,
            tangents,
            uv_sets,
            uv_map_ids,
            bone_weights,
            bone_indices,
            bone_palette: s.bone_palette.clone(),
            material,
        })
    }
}

/// The `.bmf` loader: register it with `AssetManager::register_loader` to load [`Mesh`]es.
#[derive(Clone, Copy, Debug, Default)]
pub struct BmfLoader;

impl AssetLoader for BmfLoader {
    type Asset = Mesh;

    fn extensions(&self) -> &[&str] {
        &["bmf"]
    }

    fn load(&self, ctx: &mut LoadContext<'_>) -> Result<Mesh> {
        Mesh::from_bytes(&ctx.read()?)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ss2_assets::{AssetManager, LoadState};
    use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};
    use ss2_vfs::MemoryVfs;

    use super::*;

    /// A surface description for the synthetic file.
    struct Spec {
        triangles: u32,
        vertices: u32,
        index_offset: u32,
        position_offset: u32,
        normal_offset: u32,
        uv_offset: u32,
        tangent_offset: Option<u32>,
        skin: Option<(u32, u32, Vec<i32>)>,
        material: Option<u32>,
    }

    fn path(code: u32, buffer: u32, offset: u32) -> Vec<u8> {
        ObjectBytes::new()
            .u32(code)
            .u32(buffer)
            .u32(offset)
            .finish()
    }

    fn absent() -> Vec<u8> {
        path(0, 255, 0xFFFF_FFFF)
    }

    /// One LOD with one surface; `index_bytes` and `vertex_bytes` are the two buffers.
    fn mesh_file(spec: &Spec, platform: u32, index_bytes: &[u8], vertex_bytes: &[u8]) -> Vec<u8> {
        let mut b = MetaBuilder::new();
        b.type_block(TypeBlock::primitive(0, "SLONG", 4, 4));
        b.type_block(TypeBlock::alias(1, "INDEX", 0));
        b.type_block(TypeBlock::primitive(2, "ULONG", 4, 4));
        b.type_block(TypeBlock::primitive(3, "FLOAT", 4, 4));
        b.type_block(TypeBlock::primitive(4, "UBYTE", 1, 1));
        b.type_block(TypeBlock::enumeration(5, "GfxBufferType", 4));
        b.type_block(TypeBlock::short(6, "CStaticArray<UBYTE>", 0, 6, 4));
        b.type_block(TypeBlock::class(
            7,
            "CResourceFile",
            3,
            5,
            None,
            &[("1", 1)],
        ));
        b.type_block(TypeBlock::pointer(8, "CResourceFile*", 7));
        b.type_block(TypeBlock::class(
            9,
            "CResource",
            4,
            5,
            None,
            &[("1", 1), ("2", 8)],
        ));
        b.type_block(TypeBlock::class(
            10,
            "GFXHANDLE",
            1,
            5,
            Some(9),
            &[("1", 5), ("2", 6)],
        ));
        b.type_block(TypeBlock::pointer(11, "GFXHANDLE*", 10));
        b.type_block(TypeBlock::class(
            12,
            "VBufferHolder",
            1,
            5,
            None,
            &[("1", 11)],
        ));
        b.type_block(TypeBlock::class(
            13,
            "Box3f",
            1,
            5,
            None,
            &[("1", 3), ("2", 3), ("3", 3), ("4", 3), ("5", 3), ("6", 3)],
        ));
        b.type_block(TypeBlock::class(
            14,
            "CRenderMeshElementPath",
            1,
            5,
            None,
            &[("1", 2), ("2", 2), ("3", 2)],
        ));
        b.type_block(TypeBlock::short(15, "ULONG[8]", 0, 4, 2));
        b.type_block(TypeBlock::short(16, "CRenderMeshElementPath[8]", 0, 4, 14));
        b.type_block(TypeBlock::short(17, "CStaticArray<INDEX>", 0, 6, 1));
        b.type_block(TypeBlock::pointer(19, "CResource*", 9));
        b.type_block(TypeBlock::class(
            18,
            "CRenderMeshSurface",
            1,
            5,
            None,
            &[
                ("2", 2),
                ("3", 2),
                ("4", 2),
                ("6", 14),
                ("7", 14),
                ("8", 14),
                ("9", 14),
                ("10", 14),
                ("11", 14),
                ("12", 15),
                ("14", 16),
                ("16", 19),
                ("18", 17),
            ],
        ));
        b.type_block(TypeBlock::short(
            20,
            "CStaticArray<CRenderMeshSurface>",
            0,
            6,
            18,
        ));
        b.type_block(TypeBlock::class(
            21,
            "CRenderMeshSection",
            1,
            5,
            None,
            &[("1", 20)],
        ));
        b.type_block(TypeBlock::short(
            22,
            "CStaticArray<CRenderMeshSection>",
            0,
            6,
            21,
        ));
        b.type_block(TypeBlock::class(
            23,
            "CRenderMeshLOD",
            1,
            5,
            None,
            &[("3", 13), ("5", 22), ("6", 3), ("7", 2)],
        ));
        b.type_block(TypeBlock::short(
            24,
            "CStaticArray<CRenderMeshLOD>",
            0,
            6,
            23,
        ));
        b.type_block(TypeBlock::short(
            25,
            "CStaticArray<VBufferHolder>",
            0,
            6,
            12,
        ));
        b.type_block(TypeBlock::class(
            26,
            "CRenderMesh",
            1,
            5,
            None,
            &[
                ("5", 24),
                ("12", 25),
                ("13", 25),
                ("7", 3),
                ("8", 3),
                ("9", 3),
            ],
        ));
        b.type_block(TypeBlock::pointer(27, "CRenderMesh*", 26));
        b.type_block(TypeBlock::class(
            28,
            "CMesh",
            1,
            5,
            Some(9),
            &[("1", 27), ("2", 27)],
        ));
        // The material is a second, empty resource object (a stand-in for a CShaderPreset).
        b.type_block(TypeBlock::class(29, "CShaderPreset", 1, 5, Some(9), &[]));

        b.object(0, 7, ObjectBytes::new().i32(1).finish());
        b.object(
            1,
            28,
            ObjectBytes::new()
                .i32(1)
                .object_ref(Some(0))
                .object_ref(Some(2))
                .object_ref(None)
                .finish(),
        );
        // The render mesh, its LOD with one section and one surface (inline).
        let mut surface = ObjectBytes::new()
            .u32(spec.triangles)
            .u32(spec.vertices)
            .u32(1)
            .raw(&path(135, 0, spec.index_offset))
            .raw(&path(133, 0, spec.position_offset))
            .raw(&path(133, 0, spec.normal_offset));
        surface = surface.raw(&spec.tangent_offset.map_or_else(absent, |o| path(134, 0, o)));
        match &spec.skin {
            Some((w, i, _)) => surface = surface.raw(&path(128, 0, *w)).raw(&path(128, 0, *i)),
            None => surface = surface.raw(&absent()).raw(&absent()),
        }
        for slot in 0..8u32 {
            surface = surface.u32(if slot == 0 { 3 } else { 0 });
        }
        surface = surface.raw(&path(132, 0, spec.uv_offset));
        for _ in 1..8 {
            surface = surface.raw(&absent());
        }
        surface = surface.object_ref(spec.material);
        let palette = spec.skin.as_ref().map_or(&[][..], |s| s.2.as_slice());
        surface = surface.array_header(b"STAR", palette.len() as u32);
        for p in palette {
            surface = surface.i32(*p);
        }
        let lod = ObjectBytes::new()
            .f32(-1.0)
            .f32(-1.0)
            .f32(-1.0)
            .f32(1.0)
            .f32(1.0)
            .f32(1.0)
            .array_header(b"STAR", 1)
            .array_header(b"STAR", 1)
            .raw(&surface.finish())
            .f32(150.0)
            .u32(platform);
        let holder = |id: u32| ObjectBytes::new().object_ref(Some(id)).finish();
        b.object(
            2,
            26,
            ObjectBytes::new()
                .array_header(b"STAR", 1)
                .raw(&lod.finish())
                .array_header(b"STAR", 1)
                .raw(&holder(3))
                .array_header(b"STAR", 1)
                .raw(&holder(4))
                .f32(0.0)
                .f32(500.0)
                .f32(500.0)
                .finish(),
        );
        let buffer = |kind: u32, data: &[u8]| {
            ObjectBytes::new()
                .i32(-1)
                .object_ref(None)
                .u32(kind)
                .array_header(b"STAR", data.len() as u32)
                .raw(data)
                .finish()
        };
        b.object(3, 10, buffer(0x5F, index_bytes));
        b.object(4, 10, buffer(0x1F, vertex_bytes));
        b.object(5, 29, ObjectBytes::new().i32(-1).object_ref(None).finish());
        b.build().unwrap()
    }

    fn f32s(values: &[f32]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    /// A quad: 4 vertices, 2 triangles, positions + normals + one UV set.
    fn quad() -> (Spec, Vec<u8>, Vec<u8>) {
        let idx: Vec<u8> = [0u16, 1, 2, 2, 1, 3]
            .iter()
            .flat_map(|i| i.to_le_bytes())
            .collect();
        let mut vtx = f32s(&[
            -1.0, 0.0, -1.0, 1.0, 0.0, -1.0, -1.0, 0.0, 1.0, 1.0, 0.0, 1.0,
        ]);
        vtx.extend(f32s(&[0.0, 1.0, 0.0].repeat(4)));
        vtx.extend(f32s(&[0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0]));
        (
            Spec {
                triangles: 2,
                vertices: 4,
                index_offset: 0,
                position_offset: 0,
                normal_offset: 48,
                uv_offset: 96,
                tangent_offset: None,
                skin: None,
                material: Some(5),
            },
            idx,
            vtx,
        )
    }

    #[test]
    fn reads_a_quad_with_all_its_streams() {
        let (spec, idx, vtx) = quad();
        let mesh = Mesh::from_bytes(&mesh_file(&spec, 0xFFFF_FFFF, &idx, &vtx)).unwrap();
        assert_eq!(mesh.lods.len(), 1);
        assert_eq!(mesh.distances, [0.0, 500.0, 500.0]);
        assert!(!mesh.has_collision_mesh);
        let lod = &mesh.lods[0];
        assert!(lod.is_for_pc());
        assert_eq!((lod.switch_distance, lod.triangle_count()), (150.0, 2));
        assert_eq!(
            lod.bounds,
            Aabb {
                min: [-1.0; 3],
                max: [1.0; 3]
            }
        );
        let s = &lod.surfaces[0];
        assert_eq!(s.indices, [0, 1, 2, 2, 1, 3]);
        assert_eq!(s.triangle_count(), 2);
        assert_eq!(s.positions[3], [1.0, 0.0, 1.0]);
        assert_eq!(s.normals, vec![[0.0, 1.0, 0.0]; 4]);
        assert_eq!(s.uv_sets.len(), 1);
        assert_eq!(s.uv_sets[0][3], [1.0, 1.0]);
        assert_eq!(s.uv_map_ids, [3]);
        assert!(s.tangents.is_none() && !s.is_skinned());
        assert_eq!(s.material, MaterialRef::Embedded(5));
    }

    #[test]
    fn the_platform_mask_decides_whether_a_pc_draws_the_lod() {
        let (spec, idx, vtx) = quad();
        for (mask, pc) in [
            (0xFFFF_FFFF, true),
            (0xFFFF_FFFD, true),
            (0x3, true),
            (0x1, true),
            (0xFFFF_FFFE, false),
        ] {
            let mesh = Mesh::from_bytes(&mesh_file(&spec, mask, &idx, &vtx)).unwrap();
            assert_eq!(mesh.lods[0].is_for_pc(), pc, "{mask:#x}");
        }
    }

    #[test]
    fn tangents_and_skinning_are_read() {
        let (mut spec, idx, mut vtx) = quad();
        spec.tangent_offset = Some(128);
        vtx.extend(f32s(&[1.0, 0.0, 0.0, 1.0].repeat(4)));
        spec.skin = Some((192, 208, vec![4, 5, 6, 7]));
        vtx.extend([255u8, 0, 0, 0, 200, 55, 0, 0, 0, 0, 0, 0, 128, 127, 0, 0]);
        vtx.extend([0u8, 0, 0, 0, 1, 2, 0, 0, 3, 0, 0, 0, 0, 1, 0, 0]);
        let mesh = Mesh::from_bytes(&mesh_file(&spec, 0xFFFF_FFFF, &idx, &vtx)).unwrap();
        let s = &mesh.lods[0].surfaces[0];
        assert_eq!(s.tangents.as_ref().unwrap()[0], [1.0, 0.0, 0.0, 1.0]);
        assert!(s.is_skinned());
        assert_eq!(s.bone_palette, [4, 5, 6, 7]);
        assert_eq!(s.bone_weights.as_ref().unwrap()[1], [200, 55, 0, 0]);
        assert_eq!(
            s.bone_weights.as_ref().unwrap()[2],
            [0, 0, 0, 0],
            "all zero is allowed"
        );
        assert_eq!(s.bone_indices.as_ref().unwrap()[1], [1, 2, 0, 0]);
    }

    #[test]
    fn broken_meshes_are_refused_with_a_reason() {
        let (spec, idx, vtx) = quad();
        let err = |spec: &Spec, idx: &[u8], vtx: &[u8]| {
            Mesh::from_bytes(&mesh_file(spec, 0xFFFF_FFFF, idx, vtx))
                .unwrap_err()
                .to_string()
        };
        // An index past the vertex count.
        let mut bad_idx = idx.clone();
        bad_idx[0] = 9;
        assert!(err(&spec, &bad_idx, &vtx).contains("index 9"));
        // A stream that does not fit its buffer.
        assert!(err(&spec, &idx, &vtx[..100]).contains("does not fit"));
        let mut too_many = quad().0;
        too_many.triangles = 3;
        assert!(err(&too_many, &idx, &vtx).contains("does not fit"));
        // A bone index outside the palette.
        let mut skinned = quad().0;
        skinned.skin = Some((128, 144, vec![1]));
        let mut v = vtx.clone();
        v.extend([255u8; 16]);
        v.extend([5u8; 16]);
        assert!(err(&skinned, &idx, &v).contains("outside the palette"));
        // More vertices declared than streams hold.
        let mut huge = quad().0;
        huge.vertices = 1_000_000;
        assert!(
            err(&huge, &idx, &vtx).contains("index")
                || err(&huge, &idx, &vtx).contains("does not fit")
        );
        assert!(Mesh::from_bytes(b"junk").is_err());
    }

    #[test]
    fn nothing_panics_on_damaged_files() {
        let (spec, idx, vtx) = quad();
        let good = mesh_file(&spec, 0xFFFF_FFFF, &idx, &vtx);
        for len in (0..good.len()).step_by(7) {
            let _ = Mesh::from_bytes(&good[..len]);
        }
        for i in (0..good.len()).step_by(3) {
            let mut bad = good.clone();
            bad[i] ^= 0xA5;
            let _ = Mesh::from_bytes(&bad);
        }
    }

    #[test]
    fn the_loader_plugs_into_the_asset_manager() {
        let (spec, idx, vtx) = quad();
        let file = mesh_file(&spec, 0xFFFF_FFFF, &idx, &vtx);
        let vfs = MemoryVfs::from_files([("models/q.bmf", file.as_slice())]).unwrap();
        let assets = AssetManager::new(Arc::new(vfs));
        assets.register_loader(BmfLoader).unwrap();
        let h = assets.load::<Mesh>("Models/Q.BMF").unwrap();
        assert_eq!(h.wait(), LoadState::Ready);
        assert_eq!(h.get().unwrap().lods[0].triangle_count(), 2);
    }

    /// Data-gated: every `.bmf` of the retail install loads and obeys the invariants of the spec
    /// (indices below the vertex count, positions inside the LOD box, unit normals). Skips without
    /// `SS2_DATA_DIR`.
    #[test]
    fn every_mesh_of_the_real_install_loads() {
        use ss2_vfs::{EntryKind, Vfs, VfsPath};
        let dir = ss2_testkit::require_game_data!();
        let vfs = ss2_vfs::mount_install(&dir).unwrap();
        let mut stack = vec![VfsPath::root()];
        let (mut files, mut lods, mut surfaces, mut triangles, mut vertices) = (0, 0, 0, 0, 0usize);
        let (mut skinned, mut pc_lods, mut collision) = (0, 0, 0);
        let mut failures = Vec::new();
        while let Some(d) = stack.pop() {
            for e in vfs.list(&d).unwrap() {
                let path = d.join(&e.name).unwrap();
                if e.kind == EntryKind::Dir {
                    stack.push(path);
                    continue;
                }
                if path.extension() != Some("bmf") {
                    continue;
                }
                match Mesh::from_bytes(&vfs.read(&path).unwrap()) {
                    Ok(m) => {
                        files += 1;
                        collision += usize::from(m.has_collision_mesh);
                        for lod in &m.lods {
                            lods += 1;
                            pc_lods += usize::from(lod.is_for_pc());
                            for s in &lod.surfaces {
                                surfaces += 1;
                                triangles += s.triangle_count();
                                vertices += s.positions.len();
                                skinned += usize::from(s.is_skinned());
                                for p in &s.positions {
                                    for (k, &c) in p.iter().enumerate() {
                                        if c < lod.bounds.min[k] - 0.01
                                            || c > lod.bounds.max[k] + 0.01
                                        {
                                            failures.push(format!(
                                                "{}: position outside the LOD box",
                                                path.as_str()
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => failures.push(format!("{}: {e}", path.as_str())),
                }
            }
        }
        eprintln!(
            "{files} meshes, {lods} LODs ({pc_lods} for PC), {surfaces} surfaces ({skinned} skinned), {triangles} triangles, {vertices} vertices, {collision} with a collision mesh; {} failures",
            failures.len()
        );
        assert!(
            failures.is_empty(),
            "first failures: {:?}",
            &failures[..failures.len().min(5)]
        );
        assert_eq!(files, 76, "bmf.md counts 76 meshes");
    }
}
