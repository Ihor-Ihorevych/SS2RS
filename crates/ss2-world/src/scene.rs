//! One loaded level as plain data: what to draw, where, what collides and where players start
//! (RFC 0013).
//!
//! A [`LevelScene`] names no file format. The loader (`load`, T07-028) builds it from the game's
//! files; the renderer, the collision builder and the app consume it. [`LevelScene::test_room`]
//! is a small procedural level that every consumer can be tested against without game data.
//!
//! ```
//! use ss2_world::scene::LevelScene;
//!
//! let room = LevelScene::test_room();
//! room.validate().unwrap();
//! let start = room.start();
//! assert!(room.bounds().unwrap().max.y > start.position.y);
//! ```

use ss2_core::math::{Quat, Vec3};
use ss2_core::{Error, Result};
use ss2_render_api::scene_data::{
    AlphaMode, MaterialData, MeshData, MeshVertex, TexelFormat, TextureData,
};
pub use ss2_render_api::scene_data::{CubeFace, FaceBasis};
use ss2_render_api::{Aabb, Transform};

/// One drawable piece of a model: a mesh with one material.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelPart {
    /// The triangles, in the model's own space.
    pub mesh: MeshData,
    /// Index into [`LevelScene::materials`].
    pub material: u32,
}

/// Triangles that collide, in the model's own space.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CollisionData {
    /// Vertex positions.
    pub positions: Vec<Vec3>,
    /// Three indices per triangle, into `positions`.
    pub indices: Vec<u32>,
    /// `true` when these are the model's own collision mesh (`bmf.md`, "Collision mesh"); `false`
    /// for the fallback (the coarsest PC render level of detail of a model without one) and for
    /// procedural geometry.
    pub from_collision_mesh: bool,
}

/// A model that instances place: its drawable parts and, optionally, its collision.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelData {
    /// Where it came from (a file path for a loaded model), for messages.
    pub name: String,
    /// What to draw.
    pub parts: Vec<ModelPart>,
    /// What collides; `None` means the model does not collide. The model's own collision mesh
    /// where it has a non-empty one, else its coarsest PC render level of detail (T07-036).
    pub collision: Option<CollisionData>,
}

/// A model placed in the level.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Instance {
    /// Index into [`LevelScene::models`].
    pub model: u32,
    /// Model space to world space. The scale may be negative on any axis (a mirrored placement);
    /// mesh winding stays as authored and a consumer that culls faces must account for it.
    pub transform: Transform,
    /// Whether this placement takes part in collision.
    pub collidable: bool,
    /// This placement's own lightmap: an index into [`LevelScene::textures`], drawn with the lightmap UVs of the
    /// model's parts ([`MeshData::lightmap_uvs`]); `None`: not lightmapped (RFC 0013 amendment 2).
    pub lightmap: Option<u32>,
}

/// A place a player can start.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpawnPoint {
    /// Feet position in world space.
    pub position: Vec3,
    /// Heading in radians around +Y; 0 looks down -Z.
    pub yaw: f32,
}

/// The level's sky: up to six face images drawn around the viewer behind everything (RFC 0013 amendment 1).
///
/// Each face image follows the orientation convention of [`CubeFace::basis`]: seen from inside the cube, texel
/// (0, 0) is the top-left corner and the image's right and down point along `right` and `down`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkyData {
    /// Texture index (into [`LevelScene::textures`]) per face in [`CubeFace::ALL`] order; `None` shows `tint` alone.
    pub faces: [Option<u32>; 6],
    /// Linear RGBA multiplier on every face.
    pub tint: [f32; 4],
    /// How the cube is turned in the world (the sky entity's placement rotation, applied in full; its position is
    /// never used: the cube is centred on the viewer). `docs/re/environment.md`, T01-409.
    pub rotation: Quat,
    /// The cube's edge length in world units (64 in most levels).
    pub size: f32,
    /// The viewer's height inside the cube, relative to its centre, in the same units as `size` (the sign is the
    /// spec's open question: positive puts the viewer above the centre, so the horizon drops).
    pub viewer_height: f32,
}

impl SkyData {
    /// The placement of a sky without one: not turned, size 64, the viewer at the centre.
    pub const UNPLACED_SIZE: f32 = 64.0;
}

/// A whole level, ready to draw and collide with.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelScene {
    /// The level's name, for messages and the window title.
    pub name: String,
    /// Acceleration of falling bodies, metres per second squared.
    pub gravity: Vec3,
    /// Models the instances refer to.
    pub models: Vec<ModelData>,
    /// Materials the model parts refer to.
    pub materials: Vec<MaterialData>,
    /// Textures the materials refer to.
    pub textures: Vec<TextureData>,
    /// Placed models.
    pub instances: Vec<Instance>,
    /// Start points, in the level's order.
    pub spawns: Vec<SpawnPoint>,
    /// What the loader left out or replaced, one line each.
    pub skipped: Vec<String>,
    /// The sky, if the level has one.
    pub sky: Option<SkyData>,
}

/// What a level load is busy with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadStage {
    /// Reading the world file.
    World,
    /// Reading models.
    Models,
    /// Reading textures.
    Textures,
    /// Building collision.
    Collision,
}

/// How far a level load has come within one stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoadProgress {
    /// The stage.
    pub stage: LoadStage,
    /// Items finished in this stage.
    pub done: u32,
    /// Items in this stage.
    pub total: u32,
}

impl LevelScene {
    /// Check every index and value: the renderer and the collision builder may then trust it.
    ///
    /// # Errors
    /// [`Error::Invalid`] describing the first problem found.
    pub fn validate(&self) -> Result<()> {
        let bad = |detail: String| Err(Error::invalid("level scene", detail));
        let wrap = |what: String, e: Error| Error::invalid("level scene", format!("{what}: {e}"));
        if !self.gravity.is_finite() {
            return bad(format!("gravity {}", self.gravity));
        }
        for (i, t) in self.textures.iter().enumerate() {
            t.validate().map_err(|e| wrap(format!("texture {i}"), e))?;
        }
        for (i, m) in self.materials.iter().enumerate() {
            m.validate().map_err(|e| wrap(format!("material {i}"), e))?;
            if let Some(t) = m.base_texture
                && t as usize >= self.textures.len()
            {
                return bad(format!(
                    "material {i} names texture {t} of {}",
                    self.textures.len()
                ));
            }
        }
        for (i, model) in self.models.iter().enumerate() {
            for (j, part) in model.parts.iter().enumerate() {
                part.mesh
                    .validate()
                    .map_err(|e| wrap(format!("model {i} ({}) part {j}", model.name), e))?;
                if part.material as usize >= self.materials.len() {
                    return bad(format!(
                        "model {i} ({}) part {j} names material {} of {}",
                        model.name,
                        part.material,
                        self.materials.len()
                    ));
                }
            }
            if let Some(c) = &model.collision {
                let n = c.positions.len();
                if !c.indices.len().is_multiple_of(3)
                    || c.indices.iter().any(|x| *x as usize >= n)
                    || c.positions.iter().any(|p| !p.is_finite())
                {
                    return bad(format!("model {i} ({}) has broken collision", model.name));
                }
            }
        }
        for (i, inst) in self.instances.iter().enumerate() {
            if inst.model as usize >= self.models.len() {
                return bad(format!(
                    "instance {i} names model {} of {}",
                    inst.model,
                    self.models.len()
                ));
            }
            let t = &inst.transform;
            if let Some(l) = inst.lightmap
                && l as usize >= self.textures.len()
            {
                return bad(format!(
                    "instance {i} names lightmap {l} of {}",
                    self.textures.len()
                ));
            }
            let finite = t.translation.is_finite() && t.rotation.is_finite() && t.scale.is_finite();
            if !finite
                || (t.rotation.length_squared() - 1.0).abs() > 1e-3
                || t.scale.abs().min_element() == 0.0
            {
                return bad(format!("instance {i} has a bad transform {t:?}"));
            }
        }
        if let Some(sky) = &self.sky {
            if !sky.tint.iter().all(|c| c.is_finite()) {
                return bad(format!("sky tint {:?}", sky.tint));
            }
            if !sky.rotation.is_finite() || (sky.rotation.length_squared() - 1.0).abs() > 1e-3 {
                return bad(format!("sky rotation {:?}", sky.rotation));
            }
            if !(sky.size.is_finite() && sky.size > 0.0) || !sky.viewer_height.is_finite() {
                return bad(format!(
                    "sky size {} and viewer height {}",
                    sky.size, sky.viewer_height
                ));
            }
            if let Some(t) = sky
                .faces
                .iter()
                .flatten()
                .find(|t| **t as usize >= self.textures.len())
            {
                return bad(format!(
                    "the sky names texture {t} of {}",
                    self.textures.len()
                ));
            }
        }
        if let Some(i) = self
            .spawns
            .iter()
            .position(|s| !(s.position.is_finite() && s.yaw.is_finite()))
        {
            return bad(format!("spawn {i} is not finite"));
        }
        Ok(())
    }

    /// The world-space box around every instance; `None` without instances or meshes.
    pub fn bounds(&self) -> Option<Aabb> {
        let mut out: Option<Aabb> = None;
        for inst in &self.instances {
            let Some(model) = self.models.get(inst.model as usize) else {
                continue;
            };
            for part in &model.parts {
                if let Some(b) = part.mesh.bounds() {
                    let w = b.transformed(inst.transform.matrix());
                    out = Some(match out {
                        Some(o) => Aabb::new(o.min.min(w.min), o.max.max(w.max)),
                        None => w,
                    });
                }
            }
        }
        out
    }

    /// Where a player starts: the first spawn point; without one, above the middle of the
    /// level's bounds; without bounds, the origin.
    pub fn start(&self) -> SpawnPoint {
        if let Some(s) = self.spawns.first() {
            return *s;
        }
        let position = self.bounds().map_or(Vec3::ZERO, |b| {
            Vec3::new(b.center().x, b.max.y, b.center().z)
        });
        SpawnPoint { position, yaw: 0.0 }
    }

    /// A small procedural level for tests and as a stand-in without game data: a 20 m square
    /// floor (top at y = 0) inside 3 m walls, a 0.4 m and a 1.0 m step, a 20 degree ramp and a
    /// mirrored pillar, all placements of one unit cube (see [`TEST_ROOM_LABELS`]); a checkered
    /// texture; gravity (0, -30, 0); one spawn looking down -Z at the steps; a sky whose faces show which way they
    /// face (see [`test_sky_face`]).
    pub fn test_room() -> Self {
        let place = |translation: Vec3, rotation: Quat, scale: Vec3| Instance {
            model: 0,
            transform: Transform {
                translation,
                rotation,
                scale,
            },
            collidable: true,
            lightmap: None,
        };
        let id = Quat::IDENTITY;
        let ramp = 20f32.to_radians();
        // A 6 m slab tilted about Z rises 6 sin 20 = 2.05 m from its west end to its east end.
        let instances = vec![
            place(Vec3::new(0.0, -0.1, 0.0), id, Vec3::new(20.0, 0.2, 20.0)),
            place(Vec3::new(0.0, 1.5, -10.0), id, Vec3::new(20.2, 3.0, 0.2)),
            place(Vec3::new(0.0, 1.5, 10.0), id, Vec3::new(20.2, 3.0, 0.2)),
            place(Vec3::new(-10.0, 1.5, 0.0), id, Vec3::new(0.2, 3.0, 20.2)),
            place(Vec3::new(10.0, 1.5, 0.0), id, Vec3::new(0.2, 3.0, 20.2)),
            place(Vec3::new(3.0, 0.2, -3.0), id, Vec3::new(2.0, 0.4, 2.0)),
            place(Vec3::new(-3.0, 0.5, -3.0), id, Vec3::new(2.0, 1.0, 2.0)),
            place(
                Vec3::new(6.0, 3.0 * ramp.sin() - 0.1, 4.0),
                Quat::from_rotation_z(ramp),
                Vec3::new(6.0, 0.2, 2.0),
            ),
            place(
                Vec3::new(-6.0, 1.0, 5.0),
                Quat::from_rotation_y(30f32.to_radians()),
                Vec3::new(-0.5, 2.0, 0.5),
            ),
        ];
        let mut instances = instances;
        // The floor is lightmapped: a procedural lightmap with a shadow in its middle (see `test_lightmap`).
        instances[0].lightmap = Some(TEST_LIGHTMAP_TEXTURE);
        let (vertices, indices) = unit_cube();
        // Each face of the cube maps the whole lightmap (0..1): fine for the one lightmapped placement, the floor,
        // whose top face shows it.
        let lightmap_uvs = Some(vertices.iter().map(|v| v.uv).collect());
        let collision = CollisionData {
            positions: vertices.iter().map(|v| Vec3::from(v.position)).collect(),
            indices: indices.clone(),
            from_collision_mesh: false,
        };
        Self {
            name: "test room".to_string(),
            gravity: Vec3::new(0.0, -30.0, 0.0),
            models: vec![ModelData {
                name: "unit cube".to_string(),
                parts: vec![ModelPart {
                    mesh: MeshData {
                        vertices,
                        indices,
                        lightmap_uvs,
                    },
                    material: 0,
                }],
                collision: Some(collision),
            }],
            materials: vec![MaterialData {
                base_color: [1.0, 1.0, 1.0, 1.0],
                base_texture: Some(0),
                alpha: AlphaMode::Opaque,
                double_sided: false,
            }],
            textures: std::iter::once(checker_texture())
                .chain(CubeFace::ALL.map(test_sky_face))
                .chain(std::iter::once(test_lightmap()))
                .collect(),
            instances,
            spawns: vec![SpawnPoint {
                position: Vec3::new(0.0, 0.0, 6.0),
                yaw: 0.0,
            }],
            skipped: Vec::new(),
            sky: Some(SkyData {
                faces: [1, 2, 3, 4, 5, 6].map(Some),
                tint: [1.0, 1.0, 1.0, 1.0],
                // A quarter turn to the left, so a renderer that ignores the rotation shows the wrong colour ahead.
                rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
                size: SkyData::UNPLACED_SIZE,
                viewer_height: 0.0,
            }),
        }
    }
}

/// Where [`LevelScene::test_room`] keeps the floor's lightmap in its textures.
pub const TEST_LIGHTMAP_TEXTURE: u32 = 7;

/// The test room floor's lightmap: 16 x 16 RGBA8, light grey (200) with a dark (60) 8 x 8 square in its middle, so a
/// renderer that applies it draws a shadow in the middle of the floor and one that ignores it does not.
pub fn test_lightmap() -> TextureData {
    let mut texels = Vec::with_capacity(16 * 16 * 4);
    for y in 0..16 {
        for x in 0..16 {
            let shadow = (4..12).contains(&x) && (4..12).contains(&y);
            let c = if shadow { 60 } else { 200 };
            texels.extend([c, c, c, 255]);
        }
    }
    TextureData {
        width: 16,
        height: 16,
        format: TexelFormat::Rgba8,
        mips: vec![texels],
        repeat_u: false,
        repeat_v: false,
    }
}

/// The colour of `face` in the test room's sky: each face has its own, so a face drawn in the wrong place shows.
pub fn test_sky_colour(face: CubeFace) -> [u8; 4] {
    match face {
        CubeFace::PosX => [200, 60, 60, 255],
        CubeFace::NegX => [60, 200, 200, 255],
        CubeFace::PosY => [70, 110, 230, 255],
        CubeFace::NegY => [120, 90, 50, 255],
        CubeFace::PosZ => [60, 190, 70, 255],
        CubeFace::NegZ => [220, 200, 60, 255],
    }
}

/// A 16 x 16 face image of the test room's sky in the convention of [`CubeFace::basis`]: the face's colour, a white
/// band over its top 4 rows and a black 4 x 4 square in its top-left corner, so a face that is upside down, turned or
/// mirrored shows it.
pub fn test_sky_face(face: CubeFace) -> TextureData {
    let colour = test_sky_colour(face);
    let mut texels = Vec::with_capacity(16 * 16 * 4);
    for y in 0..16 {
        for x in 0..16 {
            let texel = if x < 4 && y < 4 {
                [0, 0, 0, 255]
            } else if y < 4 {
                [255, 255, 255, 255]
            } else {
                colour
            };
            texels.extend(texel);
        }
    }
    TextureData {
        width: 16,
        height: 16,
        format: TexelFormat::Rgba8,
        mips: vec![texels],
        repeat_u: false,
        repeat_v: false,
    }
}

/// What each instance of [`LevelScene::test_room`] is, in order.
pub const TEST_ROOM_LABELS: [&str; 9] = [
    "floor",
    "north wall",
    "south wall",
    "west wall",
    "east wall",
    "low step",
    "high step",
    "ramp",
    "mirrored pillar",
];

/// A cube from -0.5 to 0.5 on every axis: 24 vertices (flat normals, 0..1 UVs per face),
/// 12 counter-clockwise triangles seen from outside.
fn unit_cube() -> (Vec<MeshVertex>, Vec<u32>) {
    // (normal, u direction, v direction) with u x v = normal, so the corners below run
    // counter-clockwise seen from outside.
    let faces: [(Vec3, Vec3, Vec3); 6] = [
        (Vec3::X, Vec3::NEG_Z, Vec3::Y),
        (Vec3::NEG_X, Vec3::Z, Vec3::Y),
        (Vec3::Y, Vec3::X, Vec3::NEG_Z),
        (Vec3::NEG_Y, Vec3::X, Vec3::Z),
        (Vec3::Z, Vec3::X, Vec3::Y),
        (Vec3::NEG_Z, Vec3::NEG_X, Vec3::Y),
    ];
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for (n, u, v) in faces {
        let base = u32::try_from(vertices.len()).unwrap_or(u32::MAX);
        for (su, sv) in [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)] {
            let p = n * 0.5 + u * su + v * sv;
            vertices.push(MeshVertex {
                position: p.to_array(),
                normal: n.to_array(),
                uv: [su + 0.5, 0.5 - sv],
            });
        }
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (vertices, indices)
}

/// An 8 x 8 two-tone checker (RGBA8) with its full mip chain, repeating.
fn checker_texture() -> TextureData {
    let mut mips = Vec::new();
    let mut size = 8u32;
    loop {
        let mut level = Vec::new();
        for y in 0..size {
            for x in 0..size {
                let light = (x * 8 / size + y * 8 / size).is_multiple_of(2);
                let c = match (size, light) {
                    (1, _) => 160,
                    (_, true) => 200,
                    (_, false) => 120,
                };
                level.extend([c, c, c, 255]);
            }
        }
        mips.push(level);
        if size == 1 {
            break;
        }
        size /= 2;
    }
    TextureData {
        width: 8,
        height: 8,
        format: TexelFormat::Rgba8,
        mips,
        repeat_u: true,
        repeat_v: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_test_room_is_valid_and_has_what_the_slices_test_against() {
        let room = LevelScene::test_room();
        room.validate().unwrap();
        assert_eq!(room.gravity, Vec3::new(0.0, -30.0, 0.0));
        assert!(!room.spawns.is_empty());
        assert!(room.instances.iter().all(|i| i.collidable));
        assert!(
            room.instances
                .iter()
                .any(|i| i.transform.scale.min_element() < 0.0),
            "a mirrored placement"
        );
        assert!(room.models.iter().all(|m| m.collision.is_some()));
        assert!(room.materials.iter().any(|m| m.base_texture.is_some()));
        assert!(room.skipped.is_empty());
        // The floor's top is at y = 0 and the room is 20 m across.
        let b = room.bounds().unwrap();
        assert!(
            (b.min.x + 10.1).abs() < 1e-4 && (b.max.x - 10.1).abs() < 1e-4,
            "{b:?}"
        );
        assert!(b.max.y >= 3.0);
        // A spawn stands on the floor.
        assert_eq!(room.start().position.y, 0.0);
    }

    #[test]
    fn the_test_room_has_a_low_and_a_high_step_and_a_ramp() {
        let room = LevelScene::test_room();
        let top = |name: &str| {
            let i = room
                .instances
                .iter()
                .zip(TEST_ROOM_LABELS)
                .find(|(_, l)| *l == name)
                .unwrap()
                .0;
            room.models[i.model as usize].parts[0]
                .mesh
                .bounds()
                .unwrap()
                .transformed(i.transform.matrix())
                .max
                .y
        };
        assert!((top("low step") - 0.4).abs() < 1e-5);
        assert!((top("high step") - 1.0).abs() < 1e-5);
        assert!(top("ramp") > 1.0);
    }

    #[test]
    fn the_cube_winds_counter_clockwise_seen_from_outside() {
        let (v, idx) = unit_cube();
        for t in idx.as_chunks::<3>().0 {
            let [a, b, c] = t.map(|i| Vec3::from(v[i as usize].position));
            let n = Vec3::from(v[t[0] as usize].normal);
            assert!((b - a).cross(c - a).dot(n) > 0.0, "{t:?}");
            assert!(
                a.dot(n) > 0.0,
                "the face is on the side its normal points to"
            );
        }
    }

    #[test]
    fn the_test_room_has_a_sky_with_six_distinct_faces() {
        let room = LevelScene::test_room();
        let sky = room.sky.unwrap();
        let mut colours = Vec::new();
        for (face, index) in CubeFace::ALL.into_iter().zip(sky.faces) {
            let t = &room.textures[index.unwrap() as usize];
            assert_eq!((t.width, t.height), (16, 16));
            let texel = |x: usize, y: usize| -> [u8; 4] {
                t.mips[0][(y * 16 + x) * 4..][..4].try_into().unwrap()
            };
            assert_eq!(texel(0, 0), [0, 0, 0, 255], "the corner mark is top-left");
            assert_eq!(texel(8, 1), [255, 255, 255, 255], "the band is at the top");
            assert_eq!(texel(8, 12), test_sky_colour(face));
            colours.push(test_sky_colour(face));
        }
        colours.sort_unstable();
        colours.dedup();
        assert_eq!(colours.len(), 6);
    }

    #[test]
    fn the_test_room_floor_is_lightmapped() {
        let room = LevelScene::test_room();
        let lit: Vec<usize> = (0..room.instances.len())
            .filter(|i| room.instances[*i].lightmap.is_some())
            .collect();
        assert_eq!(lit, [0], "only the floor");
        assert_eq!(TEST_ROOM_LABELS[0], "floor");
        let lightmap = &room.textures[TEST_LIGHTMAP_TEXTURE as usize];
        assert_eq!(lightmap, &test_lightmap());
        let mesh = &room.models[0].parts[0].mesh;
        assert_eq!(
            mesh.lightmap_uvs.as_ref().map(Vec::len),
            Some(mesh.vertices.len())
        );
        let mut bad = LevelScene::test_room();
        bad.instances[0].lightmap = Some(99);
        assert!(bad.validate().is_err());
        let mut bad = LevelScene::test_room();
        bad.models[0].parts[0]
            .mesh
            .lightmap_uvs
            .as_mut()
            .unwrap()
            .pop();
        assert!(bad.validate().is_err());
    }

    #[test]
    fn a_bad_sky_is_rejected() {
        let check = |f: &dyn Fn(&mut SkyData)| {
            let mut s = LevelScene::test_room();
            f(s.sky.as_mut().unwrap());
            s.validate().is_err()
        };
        assert!(check(&|s| s.faces[2] = Some(99)));
        assert!(check(&|s| s.tint[1] = f32::NAN));
        assert!(check(&|s| s.rotation = Quat::from_xyzw(0.0, 0.0, 0.0, 3.0)));
        assert!(check(&|s| s.size = 0.0));
        assert!(check(&|s| s.size = f32::INFINITY));
        assert!(check(&|s| s.viewer_height = f32::NAN));
        let mut s = LevelScene::test_room();
        s.sky.as_mut().unwrap().faces[3] = None;
        s.validate().unwrap();
        s.sky = None;
        s.validate().unwrap();
    }

    #[test]
    fn bounds_cover_every_instance_and_an_empty_level_has_none() {
        let mut room = LevelScene::test_room();
        let before = room.bounds().unwrap();
        room.instances.push(Instance {
            model: 0,
            transform: Transform::from_translation(Vec3::new(100.0, 5.0, 0.0)),
            collidable: false,
            lightmap: None,
        });
        let after = room.bounds().unwrap();
        assert!(after.max.x >= 100.5 && after.min == before.min);
        room.instances.clear();
        assert_eq!(room.bounds(), None);
    }

    #[test]
    fn start_falls_back_above_the_middle_and_then_to_the_origin() {
        let mut room = LevelScene::test_room();
        room.spawns.clear();
        let b = room.bounds().unwrap();
        let s = room.start();
        assert_eq!(s.position, Vec3::new(b.center().x, b.max.y, b.center().z));
        room.instances.clear();
        assert_eq!(room.start().position, Vec3::ZERO);
    }

    #[test]
    fn broken_references_and_values_are_rejected() {
        let check = |f: &dyn Fn(&mut LevelScene)| {
            let mut s = LevelScene::test_room();
            f(&mut s);
            s.validate().is_err()
        };
        assert!(check(&|s| s.instances[0].model = 99));
        assert!(check(&|s| s.models[0].parts[0].material = 99));
        assert!(check(&|s| s.materials[0].base_texture = Some(99)));
        assert!(check(&|s| s.models[0].parts[0].mesh.indices.push(0)));
        assert!(check(&|s| s.textures[0].mips.clear()));
        assert!(check(&|s| s.materials[0].base_color[0] = f32::NAN));
        assert!(check(
            &|s| s.instances[0].transform.translation.x = f32::INFINITY
        ));
        assert!(check(
            &|s| s.instances[0].transform.rotation = Quat::from_xyzw(0.0, 0.0, 0.0, 2.0)
        ));
        assert!(check(&|s| s.instances[0].transform.scale.y = 0.0));
        assert!(check(&|s| s.models[0]
            .collision
            .as_mut()
            .unwrap()
            .indices
            .push(500)));
        assert!(check(&|s| s.gravity.y = f32::NAN));
        assert!(check(&|s| s.spawns[0].yaw = f32::NAN));
    }
}
