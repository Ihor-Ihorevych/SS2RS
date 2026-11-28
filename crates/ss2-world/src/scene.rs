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

use ss2_core::math::{Mat4, Quat, Vec3};
use ss2_core::{Error, Result};
use ss2_render_api::scene_data::{
    AlphaMode, Bloom, Fog, MaterialData, MeshData, MeshVertex, TexelFormat, TextureData,
};
pub use ss2_render_api::scene_data::{CubeFace, FaceBasis};
use ss2_render_api::skinning::{BoneTransform, SkeletonData, SkinnedMeshData};
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

/// One drawable piece of an animated model: a skinned mesh with one material (RFC 0013 amendment 8).
#[derive(Clone, Debug, PartialEq)]
pub struct SkinnedPart {
    /// The triangles in the bind pose, in the model's own space. Its bone list ([`SkinnedMeshData::inverse_bind`])
    /// is the model's whole skeleton in bone order, so one palette poses every part of the model.
    pub mesh: SkinnedMeshData,
    /// Index into [`LevelScene::materials`].
    pub material: u32,
}

/// One key of a [`BoneTrack`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimKey<T> {
    /// Seconds since the animation's start.
    pub time: f32,
    /// The parent-relative value at that time.
    pub value: T,
}

/// The keys of one bone of a [`SkeletalAnimation`]: parent-relative translations and rotations (`[x, y, z, w]`), in
/// ascending time. An empty list leaves that part of the bone at its rest pose.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BoneTrack {
    /// Index into the skeleton's bones ([`SkeletonData::bones`]).
    pub bone: u32,
    /// Translation keys.
    pub translations: Vec<AnimKey<[f32; 3]>>,
    /// Rotation keys (unit quaternions).
    pub rotations: Vec<AnimKey<[f32; 4]>>,
}

/// An animation already bound to the bones of one skeleton (RFC 0013 amendment 8). It loops: time wraps every
/// [`SkeletalAnimation::duration`] seconds. Between keys translations are interpolated linearly and rotations
/// spherically along the shorter arc (`docs/re/ans.md`, reader invariant 6); before the first and after the last key
/// the nearest key is held.
#[derive(Clone, Debug, PartialEq)]
pub struct SkeletalAnimation {
    /// The animation's name, for messages and for choosing one.
    pub name: String,
    /// The length of one pass, in seconds (above 0).
    pub duration: f32,
    /// The animated bones; a bone without a track keeps its rest pose.
    pub tracks: Vec<BoneTrack>,
}

impl SkeletalAnimation {
    /// The parent-relative pose of `skeleton` `time` seconds after the start (time wraps; a time that is not finite
    /// counts as 0): its rest pose with every track's value at that time. Tracks whose bone is not in `skeleton` are
    /// skipped.
    pub fn local_pose(&self, skeleton: &SkeletonData, time: f32) -> Vec<BoneTransform> {
        let t = if time.is_finite() && self.duration > 0.0 {
            time.rem_euclid(self.duration)
        } else {
            0.0
        };
        self.pose_at(skeleton, t)
    }

    /// Like [`SkeletalAnimation::local_pose`], but does not wrap: `time` is clamped to `[0, duration]` instead of
    /// taken modulo it, so a placement playing this animation once ([`PlayMode::Once`]) holds the pose it ends on
    /// (or its first, before `time` reaches 0) rather than restarting (T04-048).
    pub fn local_pose_once(&self, skeleton: &SkeletonData, time: f32) -> Vec<BoneTransform> {
        let t = if time.is_finite() {
            time.clamp(0.0, self.duration.max(0.0))
        } else {
            0.0
        };
        self.pose_at(skeleton, t)
    }

    /// The parent-relative pose at the already-wrapped-or-clamped local time `t`.
    fn pose_at(&self, skeleton: &SkeletonData, t: f32) -> Vec<BoneTransform> {
        let mut pose = skeleton.rest_pose();
        for track in &self.tracks {
            let Some(bone) = pose.get_mut(track.bone as usize) else {
                continue;
            };
            if let Some(v) = sample_keys(&track.translations, t, |a, b, u| {
                Vec3::from(a).lerp(Vec3::from(b), u).to_array()
            }) && v.iter().all(|c| c.is_finite())
            {
                bone.translation = Vec3::from(v);
            }
            if let Some(v) = sample_keys(&track.rotations, t, |a, b, u| {
                Quat::from_array(a)
                    .normalize()
                    .slerp(Quat::from_array(b).normalize(), u)
                    .to_array()
            }) {
                let q = Quat::from_array(v);
                if q.is_finite() && q.length_squared() > 1e-12 {
                    bone.rotation = q.normalize();
                }
            }
        }
        pose
    }
}

/// Whether an [`AnimationPlayer`] repeats or stops (RFC 0013 amendment 9, T04-048).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayMode {
    /// Time wraps every [`SkeletalAnimation::duration`]: the animation repeats forever.
    Loop,
    /// Plays once from `start` and then holds the pose it ends on, instead of restarting
    /// ([`SkeletalAnimation::local_pose_once`]). A `start` the clock has not reached yet holds the first pose
    /// instead (see [`AnimationPlayer::held`]).
    Once,
}

/// Whether `name` reads as a resting loop rather than an action, ignoring case: it contains `idle` or is exactly
/// `default` (module docs, "Animated models"; T04-048). Doors, lifts and other mechanisms are named for what they
/// do (`open`, `close`, `up`, `down`, `activate`, ...) and are actions by this rule.
pub fn is_idle_or_default(name: &str) -> bool {
    name.to_ascii_lowercase().contains("idle") || name.eq_ignore_ascii_case("default")
}

/// One placement's own animation state (RFC 0013 amendment 9, T04-048): which of its model's animations it plays,
/// since when, and whether it repeats. Sim-safe (no `HashMap`, no wall clock): the app (and, later, game logic)
/// turns its own clock into the `time` seconds given to [`SkinnedModelData::palette_of`]; nothing here reads a
/// clock itself.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnimationPlayer {
    /// Index into the model's [`SkinnedModelData::animations`].
    pub animation: u32,
    /// The clock time (same seconds as `palette_of`'s `time`) at which this pass started; `time - start` is how
    /// far into the animation the placement is. `f32::INFINITY` never starts (see [`AnimationPlayer::held`]).
    pub start: f32,
    /// Loop or hold at the end.
    pub mode: PlayMode,
}

impl AnimationPlayer {
    /// Plays `animation` on a loop from `start`.
    pub fn looping(animation: u32, start: f32) -> Self {
        Self {
            animation,
            start,
            mode: PlayMode::Loop,
        }
    }

    /// Plays `animation` once from `start`, holding the pose it ends on once it finishes.
    pub fn once(animation: u32, start: f32) -> Self {
        Self {
            animation,
            start,
            mode: PlayMode::Once,
        }
    }

    /// Shows `animation`'s first frame and does not advance: `start` is set so far in the future (`+infinity`)
    /// that `time - start` is negative, and so clamped to 0, for every finite `time`. Used for a placement whose
    /// chosen animation is an action (module docs, "Animated models"): it should not play itself until something
    /// (the game, later) gives it a real `start` with [`AnimationPlayer::once`].
    pub fn held(animation: u32) -> Self {
        Self::once(animation, f32::INFINITY)
    }
}

/// The value of `keys` at `time`: held before the first and after the last key, mixed by `mix` in between.
fn sample_keys<T: Copy>(keys: &[AnimKey<T>], time: f32, mix: impl Fn(T, T, f32) -> T) -> Option<T> {
    let (first, last) = (keys.first()?, keys.last()?);
    if time <= first.time {
        return Some(first.value);
    }
    if time >= last.time {
        return Some(last.value);
    }
    let next = keys.partition_point(|k| k.time <= time);
    let (a, b) = (&keys[next - 1], &keys[next]);
    let span = b.time - a.time;
    let u = if span > 0.0 {
        (time - a.time) / span
    } else {
        1.0
    };
    Some(mix(a.value, b.value, u))
}

/// A named attachment slot on an animated model, optionally filled with another model held rigidly on one of its
/// bones (RFC 0013 amendment 11, T04-046; `docs/re/mdl.md`, "Attachment children"). Unlike [`SkinnedPart`], the
/// held model is never itself skinned: it is drawn as plain world batches, moved every frame by
/// [`ss2_render_api::skinning::attachment_transform`] on the placement's own bone palette.
#[derive(Clone, Debug, PartialEq)]
pub struct Attachment {
    /// The slot's name, for messages (`ss2_format_mdl::AttachmentChild::name`).
    pub name: String,
    /// The index of the parent bone in this model's [`SkinnedModelData::skeleton`].
    pub bone: usize,
    /// The slot's placement relative to the bone.
    pub offset: BoneTransform,
    /// The model held in the slot, drawn as static world batches; `None`: an empty slot (the data's majority,
    /// `mdl.md`).
    pub model: Option<ModelData>,
    /// Whether the slot starts hidden: a hidden attachment is not drawn (most do in the retail data, `mdl.md`).
    pub hidden: bool,
}

/// An animated model that skinned instances place (RFC 0013 amendment 8): a skeleton, skinned parts and animations
/// bound to the skeleton.
#[derive(Clone, Debug, PartialEq)]
pub struct SkinnedModelData {
    /// Where it came from (a file path for a loaded model), for messages.
    pub name: String,
    /// The bones; every part's bone list is this skeleton's, in bone order.
    pub skeleton: SkeletonData,
    /// What to draw.
    pub parts: Vec<SkinnedPart>,
    /// The animations, at least one.
    pub animations: Vec<SkeletalAnimation>,
    /// The animation a placement plays: an index into `animations` (the loader's module docs say how it is chosen).
    pub default_animation: u32,
    /// What collides, in the bind pose; `None` means the model does not collide.
    pub collision: Option<CollisionData>,
    /// This model's attachment slots (RFC 0013 amendment 11, T04-046), in `ss2_format_mdl::Model::children` order.
    pub attachments: Vec<Attachment>,
}

impl SkinnedModelData {
    /// The bone palette (one bone-to-model matrix per bone, in bone order: what `Renderer::set_bone_palette` takes)
    /// of animation `animation` at `time` seconds; the rest pose for an index that is out of range.
    pub fn palette(&self, animation: usize, time: f32) -> Vec<Mat4> {
        let local = match self.animations.get(animation) {
            Some(a) => a.local_pose(&self.skeleton, time),
            None => self.skeleton.rest_pose(),
        };
        // One transform per bone by construction, so this cannot fail.
        self.skeleton.model_transforms(&local).unwrap_or_default()
    }

    /// [`SkinnedModelData::palette`] of the default animation.
    pub fn default_palette(&self, time: f32) -> Vec<Mat4> {
        self.palette(self.default_animation as usize, time)
    }

    /// The box around every part in the bind pose, in model space; `None` without vertices.
    pub fn bounds(&self) -> Option<Aabb> {
        self.parts
            .iter()
            .filter_map(|p| p.mesh.bounds())
            .reduce(|a, b| Aabb::new(a.min.min(b.min), a.max.max(b.max)))
    }

    /// The bone palette `player` gives at `time` seconds (T04-048): `player`'s animation, played from `player.start`
    /// and looped or held at the end per `player.mode`; the rest pose for an animation index that is out of range.
    pub fn palette_of(&self, player: &AnimationPlayer, time: f32) -> Vec<Mat4> {
        let local = match self.animations.get(player.animation as usize) {
            Some(a) => {
                let elapsed = time - player.start;
                match player.mode {
                    PlayMode::Loop => a.local_pose(&self.skeleton, elapsed),
                    PlayMode::Once => a.local_pose_once(&self.skeleton, elapsed),
                }
            }
            None => self.skeleton.rest_pose(),
        };
        self.skeleton.model_transforms(&local).unwrap_or_default()
    }

    /// The player a freshly placed instance starts with (module docs, "Animated models"; T04-048): its chosen
    /// [`SkinnedModelData::default_animation`], looping from the scene's start (`0.0`) if the name reads as a
    /// resting loop ([`is_idle_or_default`]), else held at its first frame ([`AnimationPlayer::held`]) until
    /// something gives the placement a real start — an action should not play itself.
    pub fn default_player(&self) -> AnimationPlayer {
        let loops = self
            .animations
            .get(self.default_animation as usize)
            .is_some_and(|a| is_idle_or_default(&a.name));
        if loops {
            AnimationPlayer::looping(self.default_animation, 0.0)
        } else {
            AnimationPlayer::held(self.default_animation)
        }
    }

    /// The box used to cull this model's draws (T04-048): [`SkinnedModelData::bounds`] (the bind pose) grown by
    /// twice its own diagonal plus 10 m on every side, in model space; `None` without vertices. This is the same
    /// margin `ss2-world/tests/load_install.rs`'s `animated_placements_pose_with_finite_bounded_palettes` already
    /// checks every posed vertex of the player's own data stays inside (of `dm_dockingbay` and `6_3_snow_base`), so
    /// using it for culling does not vanish anything those levels animate; a future model whose motion needs more
    /// room would have to be checked again.
    pub fn culling_bounds(&self) -> Option<Aabb> {
        let b = self.bounds()?;
        let grow = Vec3::splat((b.max - b.min).length() * 2.0 + 10.0);
        Some(Aabb::new(b.min - grow, b.max + grow))
    }
}

/// An animated model placed in the level (RFC 0013 amendment 8).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkinnedInstance {
    /// Index into [`LevelScene::skinned_models`].
    pub model: u32,
    /// Model space to world space, with the rules of [`Instance::transform`].
    pub transform: Transform,
    /// Whether this placement takes part in collision (with its model's bind-pose collision).
    pub collidable: bool,
    /// This placement's own animation state (RFC 0013 amendment 9, T04-048): the loader starts it at
    /// [`SkinnedModelData::default_player`]; the app poses the placement from it every frame
    /// ([`SkinnedModelData::palette_of`]) instead of every placement of a model sharing one clock.
    pub animation: AnimationPlayer,
    /// This placement's own lightmap: an index into [`LevelScene::textures`], drawn with the lightmap UVs of the
    /// model's parts ([`SkinnedPart::mesh`]'s `lightmap_uvs`); `None`: not lightmapped, with the rules of
    /// [`Instance::lightmap`] (RFC 0013 amendment 10, T04-047).
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
    /// The cube's half-extent (centre to face) in world units: 64 in most levels (`docs/re/environment.md`).
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
    /// The haze volumes, in the file's list order (RFC 0013 amendment 4); see [`LevelScene::fog_at`].
    pub hazes: Vec<HazeVolume>,
    /// The bloom volumes, in the file's list order (RFC 0013 amendment 6); see [`LevelScene::blooms_at`].
    pub blooms: Vec<BloomVolume>,
    /// Animated models the skinned instances refer to (RFC 0013 amendment 8).
    pub skinned_models: Vec<SkinnedModelData>,
    /// Placed animated models: drawn skinned and posed every frame by the app; they are not in `instances`.
    pub skinned_instances: Vec<SkinnedInstance>,
}

/// One bloom volume (`docs/re/environment.md`, "Bloom"): a box like a haze's, fading in over `fade_margin` as the eye
/// goes in, and the numbers of the original's bloom chain.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BloomVolume {
    /// The box's low corner in the volume's own frame.
    pub min: Vec3,
    /// The box's high corner in the volume's own frame.
    pub max: Vec3,
    /// How the volume's frame is turned about the world origin.
    pub rotation: Quat,
    /// Linear RGBA added at the end of the chain (renderable `.3`).
    pub colour: [f32; 4],
    /// The edge fade distance (`.2`), at least [`HazeVolume::MIN_FADE_MARGIN`].
    pub fade_margin: f32,
    /// The strength multiplier (renderable `.4`, record `.6`).
    pub strength: f32,
    /// The blur spread (renderable `.6`, record `.8`).
    pub blur: f32,
    /// How the two blur taps share the spread (renderable `.12`, record `.13`).
    pub balance: f32,
    /// The high-pass amount (renderable `.13`, record `.16`).
    pub high_pass: f32,
    /// Where on the fixed colour ramp the tint samples (renderable `.18`, the record's second `.21`).
    pub ramp: f32,
}

impl BloomVolume {
    /// How strongly the bloom applies with the eye at `eye`, by the same edge fade as [`HazeVolume::fade`].
    pub fn fade(&self, eye: Vec3) -> f32 {
        box_fade(self.min, self.max, self.rotation, self.fade_margin, eye)
    }
}

/// The edge fade shared by haze and bloom: 1 inside the box shrunk by `margin` on every face (never past its
/// centre), falling linearly to 0 at the box's own faces, 0 outside; measured in the box's turned frame.
fn box_fade(min: Vec3, max: Vec3, rotation: Quat, margin: f32, eye: Vec3) -> f32 {
    let local = rotation.inverse() * eye;
    let margin = margin.max(HazeVolume::MIN_FADE_MARGIN);
    let centre = (min + max) * 0.5;
    let lo = (min + Vec3::splat(margin)).min(centre);
    let hi = (max - Vec3::splat(margin)).max(centre);
    let outside = (lo - local).max(local - hi).max(Vec3::ZERO);
    1.0 - (outside.length() / margin).clamp(0.0, 1.0)
}

/// Bad numbers in a bloom, if any.
fn bloom_problem(b: &BloomVolume) -> Option<String> {
    let numbers = [b.strength, b.blur, b.balance, b.high_pass, b.ramp];
    let finite = b.min.is_finite()
        && b.max.is_finite()
        && b.colour.iter().all(|c| c.is_finite())
        && b.fade_margin.is_finite()
        && numbers.iter().all(|n| n.is_finite());
    if !finite {
        return Some("is not finite".into());
    }
    if b.min.cmpgt(b.max).any() {
        return Some(format!("box {} .. {} is inside out", b.min, b.max));
    }
    if !b.rotation.is_finite() || (b.rotation.length_squared() - 1.0).abs() > 1e-3 {
        return Some(format!("rotation {:?}", b.rotation));
    }
    if b.fade_margin < HazeVolume::MIN_FADE_MARGIN {
        return Some(format!("fade margin {}", b.fade_margin));
    }
    if numbers.iter().any(|n| *n < 0.0) {
        return Some(format!("negative chain numbers {numbers:?}"));
    }
    None
}

/// One haze volume (`docs/re/environment.md`, "Haze"): a box with a colour and a linear fog range, fading in over
/// `fade_margin` as the eye goes in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HazeVolume {
    /// The box's low corner in the volume's own frame.
    pub min: Vec3,
    /// The box's high corner in the volume's own frame.
    pub max: Vec3,
    /// How the volume's frame is turned about the world origin: the haze renderable's world rotation (identity for
    /// most hazes; 13 of the 65 retail hazes are turned, 2 of them through a parent).
    pub rotation: Quat,
    /// Linear RGBA; the alpha is carried, its use is open.
    pub colour: [f32; 4],
    /// The edge fade distance (`.5`), at least [`HazeVolume::MIN_FADE_MARGIN`].
    pub fade_margin: f32,
    /// Where the fog starts (`.6`).
    pub near: f32,
    /// Where the fog is full (`.7`), at least `near`.
    pub far: f32,
}

impl HazeVolume {
    /// The original clamps the edge fade to at least this.
    pub const MIN_FADE_MARGIN: f32 = 0.001;

    /// How strongly the volume applies with the eye at `eye`: 1 inside the box shrunk by `fade_margin` on every face,
    /// falling linearly to 0 at the box's own faces and 0 outside (the original's edge fade).
    pub fn fade(&self, eye: Vec3) -> f32 {
        box_fade(self.min, self.max, self.rotation, self.fade_margin, eye)
    }
}

/// Bad numbers in a haze, if any.
fn haze_problem(h: &HazeVolume) -> Option<String> {
    let finite = h.min.is_finite()
        && h.max.is_finite()
        && h.colour.iter().all(|c| c.is_finite())
        && h.fade_margin.is_finite()
        && h.near.is_finite()
        && h.far.is_finite();
    if !finite {
        return Some("is not finite".into());
    }
    if h.min.cmpgt(h.max).any() {
        return Some(format!("box {} .. {} is inside out", h.min, h.max));
    }
    if !h.rotation.is_finite() || (h.rotation.length_squared() - 1.0).abs() > 1e-3 {
        return Some(format!("rotation {:?}", h.rotation));
    }
    if h.fade_margin < HazeVolume::MIN_FADE_MARGIN {
        return Some(format!("fade margin {}", h.fade_margin));
    }
    if h.far < h.near {
        return Some(format!("far {} before near {}", h.far, h.near));
    }
    None
}

/// Whether a placement's transform is finite, its rotation of unit length and no scale axis zero.
fn transform_ok(t: &Transform) -> bool {
    let finite = t.translation.is_finite() && t.rotation.is_finite() && t.scale.is_finite();
    finite
        && (t.rotation.length_squared() - 1.0).abs() <= 1e-3
        && t.scale.abs().min_element() != 0.0
}

/// What is wrong with an animation of a skeleton of `bones` bones, if anything.
fn animation_problem(a: &SkeletalAnimation, bones: usize) -> Option<String> {
    if !(a.duration.is_finite() && a.duration > 0.0) {
        return Some(format!("lasts {} s", a.duration));
    }
    for t in &a.tracks {
        if t.bone as usize >= bones {
            return Some(format!("animates bone {} of {bones}", t.bone));
        }
        let translations: Vec<(f32, bool)> = t
            .translations
            .iter()
            .map(|k| (k.time, k.value.iter().all(|c| c.is_finite())))
            .collect();
        let rotations: Vec<(f32, bool)> = t
            .rotations
            .iter()
            .map(|k| (k.time, k.value.iter().all(|c| c.is_finite())))
            .collect();
        for keys in [translations, rotations] {
            if keys.iter().any(|(time, ok)| !(time.is_finite() && *ok)) {
                return Some(format!("has a non-finite key on bone {}", t.bone));
            }
            if keys.windows(2).any(|w| w[1].0 < w[0].0) {
                return Some(format!("has keys out of order on bone {}", t.bone));
            }
        }
    }
    None
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
            if !transform_ok(t) {
                return bad(format!("instance {i} has a bad transform {t:?}"));
            }
        }
        self.validate_skinned()?;
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
        for (i, h) in self.hazes.iter().enumerate() {
            if let Some(problem) = haze_problem(h) {
                return bad(format!("haze {i} {problem}"));
            }
        }
        for (i, b) in self.blooms.iter().enumerate() {
            if let Some(problem) = bloom_problem(b) {
                return bad(format!("bloom {i} {problem}"));
            }
        }
        Ok(())
    }

    /// The checks of [`LevelScene::validate`] for the animated models and their placements.
    fn validate_skinned(&self) -> Result<()> {
        let bad = |detail: String| Err(Error::invalid("level scene", detail));
        for (i, model) in self.skinned_models.iter().enumerate() {
            let what = format!("skinned model {i} ({})", model.name);
            let bones = model.skeleton.bones().len();
            for (j, part) in model.parts.iter().enumerate() {
                if let Err(e) = part.mesh.validate() {
                    return bad(format!("{what} part {j}: {e}"));
                }
                if part.mesh.inverse_bind.len() != bones {
                    return bad(format!(
                        "{what} part {j} has {} bones, the skeleton {bones}",
                        part.mesh.inverse_bind.len()
                    ));
                }
                if part.material as usize >= self.materials.len() {
                    return bad(format!(
                        "{what} part {j} names material {} of {}",
                        part.material,
                        self.materials.len()
                    ));
                }
            }
            if model.default_animation as usize >= model.animations.len() {
                return bad(format!(
                    "{what} plays animation {} of {}",
                    model.default_animation,
                    model.animations.len()
                ));
            }
            for a in &model.animations {
                if let Some(problem) = animation_problem(a, bones) {
                    return bad(format!("{what} animation `{}` {problem}", a.name));
                }
            }
            if let Some(c) = &model.collision {
                let n = c.positions.len();
                if !c.indices.len().is_multiple_of(3)
                    || c.indices.iter().any(|x| *x as usize >= n)
                    || c.positions.iter().any(|p| !p.is_finite())
                {
                    return bad(format!("{what} has broken collision"));
                }
            }
            for (j, a) in model.attachments.iter().enumerate() {
                if a.bone >= bones {
                    return bad(format!(
                        "{what} attachment {j} ({}) names bone {} of {bones}",
                        a.name, a.bone
                    ));
                }
                if !a.offset.rotation.is_finite()
                    || (a.offset.rotation.length_squared() - 1.0).abs() > 1e-3
                    || !a.offset.translation.is_finite()
                {
                    return bad(format!(
                        "{what} attachment {j} ({}) has a bad offset {:?}",
                        a.name, a.offset
                    ));
                }
                if let Some(held) = &a.model {
                    for (k, part) in held.parts.iter().enumerate() {
                        if let Err(e) = part.mesh.validate() {
                            return bad(format!(
                                "{what} attachment {j} ({}) model part {k}: {e}",
                                a.name
                            ));
                        }
                        if part.material as usize >= self.materials.len() {
                            return bad(format!(
                                "{what} attachment {j} ({}) model part {k} names material {} of {}",
                                a.name,
                                part.material,
                                self.materials.len()
                            ));
                        }
                    }
                }
            }
        }
        for (i, inst) in self.skinned_instances.iter().enumerate() {
            if inst.model as usize >= self.skinned_models.len() {
                return bad(format!(
                    "skinned instance {i} names model {} of {}",
                    inst.model,
                    self.skinned_models.len()
                ));
            }
            if !transform_ok(&inst.transform) {
                return bad(format!(
                    "skinned instance {i} has a bad transform {:?}",
                    inst.transform
                ));
            }
            if let Some(l) = inst.lightmap
                && l as usize >= self.textures.len()
            {
                return bad(format!(
                    "skinned instance {i} names lightmap {l} of {}",
                    self.textures.len()
                ));
            }
            let animations = self.skinned_models[inst.model as usize].animations.len();
            if inst.animation.animation as usize >= animations {
                return bad(format!(
                    "skinned instance {i} plays animation {} of {animations}",
                    inst.animation.animation
                ));
            }
            if inst.animation.start.is_nan() {
                return bad(format!("skinned instance {i} has a NaN animation start"));
            }
        }
        Ok(())
    }

    /// The blooms active with the eye at `eye` (RFC 0013 amendment 6): every volume whose edge fade is above 0, in
    /// list order (the original runs each one's chain in turn: every bloom has the same sort key, so they run in the
    /// level's own object order, T01-422).
    pub fn blooms_at(&self, eye: Vec3) -> Vec<Bloom> {
        self.blooms
            .iter()
            .filter_map(|b| {
                let fade = b.fade(eye);
                (fade > 0.0).then_some(Bloom {
                    colour: b.colour,
                    fade: fade.clamp(0.0, 1.0),
                    strength: b.strength,
                    blur: b.blur,
                    balance: b.balance,
                    high_pass: b.high_pass,
                    ramp: b.ramp,
                })
            })
            .collect()
    }

    /// The fog with the eye at `eye` (RFC 0013 amendment 4, `docs/re/environment.md` "Combining several hazes",
    /// T01-412 and T01-415): every volume with a [`HazeVolume::fade`] above 0 contributes. Its weight is its fade
    /// times (the largest contributing box diagonal / its own box diagonal), so a small haze outweighs a large one at
    /// the same fade; near, far and the colour are the averages by that weight (alpha is averaged by the original too,
    /// but its effect on the pixels is open, T01-427, and it is not carried). When the plain fades sum to less than 1
    /// the range widens towards no fog: near times that sum, far divided by it. `None` when no volume reaches the eye.
    pub fn fog_at(&self, eye: Vec3) -> Option<Fog> {
        let active: Vec<(f32, &HazeVolume)> = self
            .hazes
            .iter()
            .map(|h| (h.fade(eye), h))
            .filter(|(fade, _)| *fade > 0.0)
            .collect();
        if active.is_empty() {
            return None;
        }
        let diagonal = |h: &HazeVolume| (h.max - h.min).length().max(1e-6);
        let largest = active
            .iter()
            .map(|(_, h)| diagonal(h))
            .fold(0.0f32, f32::max);
        let (mut fades, mut weights) = (0.0, 0.0);
        let (mut near, mut far, mut colour) = (0.0, 0.0, [0.0f32; 3]);
        for (fade, h) in &active {
            let w = fade * largest / diagonal(h);
            fades += fade;
            weights += w;
            near += w * h.near;
            far += w * h.far;
            for (c, v) in colour.iter_mut().zip(h.colour) {
                *c += w * v;
            }
        }
        let (mut near, mut far) = (near / weights, far / weights);
        if fades < 1.0 {
            near *= fades;
            far /= fades;
        }
        Some(Fog {
            colour: colour.map(|c| c / weights),
            near,
            far: far.max(near),
            strength: 1.0,
        })
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
        for inst in &self.skinned_instances {
            let Some(b) = self
                .skinned_models
                .get(inst.model as usize)
                .and_then(SkinnedModelData::bounds)
            else {
                continue;
            };
            let w = b.transformed(inst.transform.matrix());
            out = Some(match out {
                Some(o) => Aabb::new(o.min.min(w.min), o.max.max(w.max)),
                None => w,
            });
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
            hazes: vec![TEST_HAZE],
            blooms: vec![TEST_BLOOM],
            skinned_models: Vec::new(),
            skinned_instances: Vec::new(),
        }
    }

    /// [`LevelScene::test_room`] with [`test_skinned_bar`] standing on the floor 4 m in front of the spawn (at
    /// `(0, 0, 2)`), collidable: an animated placement for tests without game data. Its player loops `Bend` from
    /// `0.0` (not [`SkinnedModelData::default_player`]'s choice, since `Bend` is an action by that rule: tests
    /// that want to see it loop, as callers did before T04-048, still can).
    pub fn test_room_animated() -> Self {
        let mut room = Self::test_room();
        room.skinned_models.push(test_skinned_bar());
        room.skinned_instances.push(SkinnedInstance {
            model: 0,
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
            collidable: true,
            animation: AnimationPlayer::looping(0, 0.0),
            lightmap: None,
        });
        room
    }
}

/// A small animated model for tests: a 0.4 m square bar 2 m tall on two bones (the root at the origin, a child named
/// `Tip` at y = 1), its lower half on the root and its upper half on the child, with collision, and one animation,
/// `Bend`, that turns the child a quarter turn about +Z and back over 2 s (keys at 0, 1 and 2 s). Its part names
/// material 0.
pub fn test_skinned_bar() -> SkinnedModelData {
    use ss2_render_api::skinning::{SkeletonBone, SkinnedVertex};
    let skeleton = SkeletonData::new(vec![
        SkeletonBone::at_rest(1, None, BoneTransform::IDENTITY, Mat4::IDENTITY),
        SkeletonBone::at_rest(
            2,
            Some(1),
            BoneTransform::new(Quat::IDENTITY, Vec3::Y),
            Mat4::from_translation(-Vec3::Y),
        ),
    ])
    .and_then(|s| s.with_bone_names(["Root", "Tip"]))
    .unwrap_or_else(|e| unreachable!("a fixed valid skeleton: {e}"));
    // The unit cube scaled to 0.4 x 2 x 0.4, standing on y = 0; vertices above y = 1 follow the child.
    let (cube, indices) = unit_cube();
    let vertices: Vec<SkinnedVertex> = cube
        .iter()
        .map(|v| {
            let p = Vec3::from(v.position) * Vec3::new(0.4, 2.0, 0.4) + Vec3::Y;
            SkinnedVertex {
                position: p.to_array(),
                normal: v.normal,
                uv: v.uv,
                joints: [u16::from(p.y > 1.0), 0, 0, 0],
                weights: [1.0, 0.0, 0.0, 0.0],
            }
        })
        .collect();
    let collision = CollisionData {
        positions: vertices.iter().map(|v| Vec3::from(v.position)).collect(),
        indices: indices.clone(),
        from_collision_mesh: false,
    };
    let turn = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2).to_array();
    let key = |time: f32, value: [f32; 4]| AnimKey { time, value };
    SkinnedModelData {
        name: "skinned test bar".to_string(),
        parts: vec![SkinnedPart {
            mesh: SkinnedMeshData {
                vertices,
                indices,
                inverse_bind: skeleton.inverse_binds(),
                lightmap_uvs: None,
            },
            material: 0,
        }],
        skeleton,
        animations: vec![SkeletalAnimation {
            name: "Bend".to_string(),
            duration: 2.0,
            tracks: vec![BoneTrack {
                bone: 1,
                translations: Vec::new(),
                rotations: vec![
                    key(0.0, Quat::IDENTITY.to_array()),
                    key(1.0, turn),
                    key(2.0, Quat::IDENTITY.to_array()),
                ],
            }],
        }],
        default_animation: 0,
        collision: Some(collision),
        attachments: Vec::new(),
    }
}

/// [`test_skinned_bar`] with one attachment on its `Tip` bone, half a metre further up, holding a 0.2 m cube (for
/// tests of [`Attachment`] and `attachment_transform`; RFC 0013 amendment 11, T04-046).
pub fn test_skinned_bar_with_attachment() -> SkinnedModelData {
    let mut bar = test_skinned_bar();
    let (cube, indices) = unit_cube();
    let vertices: Vec<MeshVertex> = cube
        .iter()
        .map(|v| MeshVertex {
            position: (Vec3::from(v.position) * 0.2).to_array(),
            normal: v.normal,
            uv: v.uv,
        })
        .collect();
    bar.attachments.push(Attachment {
        name: "Muzzle".to_string(),
        bone: 1, // "Tip"
        offset: BoneTransform::new(Quat::IDENTITY, Vec3::new(0.0, 0.5, 0.0)),
        model: Some(ModelData {
            name: "test attachment cube".to_string(),
            parts: vec![ModelPart {
                mesh: MeshData {
                    vertices,
                    indices,
                    lightmap_uvs: None,
                },
                material: 0,
            }],
            collision: None,
        }),
        hidden: false,
    });
    bar
}

/// [`LevelScene::test_room`]'s haze: the far (-Z) half of the room, fading in over the metre before its south face at
/// z = 0, a blue-grey fog from 2 m to 15 m.
/// [`LevelScene::test_room`]'s bloom: the near (+Z) half of the room, where the spawn stands, fading in over the
/// metre past its north face at z = 0; numbers like `1_3_Village`'s (`docs/re/environment.md`).
pub const TEST_BLOOM: BloomVolume = BloomVolume {
    min: Vec3::new(-10.0, -1.0, 0.0),
    max: Vec3::new(10.0, 4.0, 10.0),
    rotation: Quat::IDENTITY,
    colour: [0.06, 0.045, 0.02, 1.0],
    fade_margin: 1.0,
    strength: 1.2422,
    blur: 1.0,
    balance: 1.0,
    high_pass: 0.15,
    ramp: 1.5,
};

pub const TEST_HAZE: HazeVolume = HazeVolume {
    min: Vec3::new(-10.0, -1.0, -10.0),
    max: Vec3::new(10.0, 4.0, 0.0),
    rotation: Quat::IDENTITY,
    colour: [0.3, 0.35, 0.4, 1.0],
    fade_margin: 1.0,
    near: 2.0,
    far: 15.0,
};

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

#[cfg(test)]
mod haze_tests {
    use super::*;

    #[test]
    fn the_fade_is_full_inside_falls_over_the_margin_and_is_zero_outside() {
        let h = TEST_HAZE;
        assert_eq!(h.fade(Vec3::new(0.0, 1.0, -5.0)), 1.0);
        assert_eq!(h.fade(Vec3::new(0.0, 1.0, -1.0)), 1.0, "on the inset face");
        assert!((h.fade(Vec3::new(0.0, 1.0, -0.5)) - 0.5).abs() < 1e-5);
        assert_eq!(
            h.fade(Vec3::new(0.0, 1.0, 0.0)),
            0.0,
            "on the box's own face"
        );
        assert_eq!(h.fade(Vec3::new(0.0, 1.0, 6.0)), 0.0);
        // A turned volume: a quarter turn about +Y puts the box's -Z half on the -X side.
        let turned = HazeVolume {
            rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            ..h
        };
        assert_eq!(turned.fade(Vec3::new(-5.0, 1.0, 0.0)), 1.0);
        assert_eq!(turned.fade(Vec3::new(5.0, 1.0, 0.0)), 0.0);
        // A box thinner than twice the margin still has a full middle.
        let thin = HazeVolume {
            min: Vec3::new(-0.5, -0.5, -0.5),
            max: Vec3::new(0.5, 0.5, 0.5),
            ..h
        };
        assert_eq!(thin.fade(Vec3::ZERO), 1.0);
    }

    #[test]
    fn several_hazes_are_averaged_by_their_fade_and_a_weak_total_widens_the_range() {
        // The spec's worked example (`1_2_approachingvillage`): A near -150 far 20, B near 0 far 5000.
        let a = HazeVolume {
            min: Vec3::new(-10.0, -10.0, -10.0),
            max: Vec3::new(10.0, 10.0, 10.0),
            near: -150.0,
            far: 20.0,
            fade_margin: 10.0,
            ..TEST_HAZE
        };
        let b = HazeVolume {
            near: 0.0,
            far: 5000.0,
            colour: [1.0, 0.0, 0.0, 1.0],
            ..a
        };
        let mut scene = LevelScene::test_room();
        // Only A's shape, at fade 0.3: the box spans x -20..0 with a 10 m margin (inset face at x = -10), the eye 7 m
        // past it.
        let thirty = HazeVolume {
            min: Vec3::new(-20.0, -20.0, -20.0),
            max: Vec3::new(0.0, 20.0, 20.0),
            ..a
        };
        scene.hazes = vec![thirty];
        let eye = Vec3::new(-3.0, 0.0, 0.0);
        assert!((thirty.fade(eye) - 0.3).abs() < 1e-5);
        let fog = scene.fog_at(eye).unwrap();
        assert!((fog.near - -45.0).abs() < 1e-3, "{fog:?}");
        assert!((fog.far - 20.0 / 0.3).abs() < 1e-2, "{fog:?}");
        // Deep inside both: the plain average, no widening.
        scene.hazes = vec![a, b];
        let fog = scene.fog_at(Vec3::ZERO).unwrap();
        assert_eq!((fog.near, fog.far, fog.strength), (-75.0, 2510.0, 1.0));
        for (got, want) in fog.colour.iter().zip([0.65, 0.175, 0.2]) {
            assert!((got - want).abs() < 1e-5, "{:?}", fog.colour);
        }
        // Outside both: no fog.
        assert_eq!(scene.fog_at(Vec3::splat(100.0)), None);
    }

    #[test]
    fn a_small_haze_outweighs_a_large_one_by_their_box_diagonals() {
        // The spec's corrected worked example (`1_2_approachingvillage`, T01-415): with both hazes full, the result
        // lands near the small haze A's own values. A's box diagonal is 1/27.8 of B's here, which gives the spec's
        // near -144.8 and far 192.
        let a = HazeVolume {
            min: Vec3::splat(-10.0),
            max: Vec3::splat(10.0),
            near: -150.0,
            far: 20.0,
            fade_margin: 1.0,
            ..TEST_HAZE
        };
        let b = HazeVolume {
            min: Vec3::splat(-278.0),
            max: Vec3::splat(278.0),
            near: 0.0,
            far: 5000.0,
            colour: [1.0, 0.0, 0.0, 1.0],
            ..a
        };
        let mut scene = LevelScene::test_room();
        scene.hazes = vec![a, b];
        let fog = scene.fog_at(Vec3::ZERO).unwrap();
        assert!((fog.near - -144.8).abs() < 0.1, "{fog:?}");
        assert!((fog.far - 192.0).abs() < 1.0, "{fog:?}");
        assert!(
            fog.colour[0] < 0.4,
            "the colour stays near the small haze's: {fog:?}"
        );
        // The order does not matter.
        scene.hazes = vec![b, a];
        let swapped = scene.fog_at(Vec3::ZERO).unwrap();
        assert!((swapped.near - fog.near).abs() < 1e-3 && (swapped.far - fog.far).abs() < 1e-2);
    }

    #[test]
    fn the_test_room_fogs_its_far_half() {
        let room = LevelScene::test_room();
        room.validate().unwrap();
        assert_eq!(
            room.fog_at(room.start().position + Vec3::Y),
            None,
            "the spawn is outside"
        );
        let fog = room.fog_at(Vec3::new(0.0, 1.0, -5.0)).unwrap();
        assert_eq!(
            (fog.colour, fog.near, fog.far, fog.strength),
            ([0.3, 0.35, 0.4], 2.0, 15.0, 1.0)
        );
        fog.validate().unwrap();
    }

    #[test]
    fn bad_hazes_are_refused() {
        let check = |f: &dyn Fn(&mut HazeVolume)| {
            let mut room = LevelScene::test_room();
            f(&mut room.hazes[0]);
            room.validate().is_err()
        };
        assert!(!check(&|_| {}));
        assert!(check(&|h| h.min.x = 50.0));
        assert!(check(&|h| h.max.y = f32::NAN));
        assert!(check(&|h| h.colour[3] = f32::INFINITY));
        assert!(check(&|h| h.rotation = Quat::from_xyzw(0.0, 0.0, 0.0, 2.0)));
        assert!(check(&|h| h.fade_margin = 0.0));
        assert!(check(&|h| h.far = 1.0));
    }
}

#[cfg(test)]
mod bloom_tests {
    use super::*;

    #[test]
    fn a_bloom_is_active_inside_its_box_with_the_haze_edge_fade() {
        let room = LevelScene::test_room();
        room.validate().unwrap();
        let at_spawn = room.blooms_at(room.start().position + Vec3::Y);
        assert_eq!(at_spawn.len(), 1, "the spawn stands in the bloom");
        assert_eq!(at_spawn[0].fade, 1.0);
        assert_eq!(at_spawn[0].strength, TEST_BLOOM.strength);
        at_spawn[0].validate().unwrap();
        assert!((TEST_BLOOM.fade(Vec3::new(0.0, 1.0, 0.5)) - 0.5).abs() < 1e-5);
        assert!(
            room.blooms_at(Vec3::new(0.0, 1.0, -5.0)).is_empty(),
            "the far half has none"
        );
        // Same fade rule as haze for the same box.
        let haze = HazeVolume {
            min: TEST_BLOOM.min,
            max: TEST_BLOOM.max,
            rotation: TEST_BLOOM.rotation,
            fade_margin: TEST_BLOOM.fade_margin,
            ..TEST_HAZE
        };
        for z in [-1.0, 0.0, 0.3, 0.7, 1.0, 5.0, 9.5, 11.0] {
            let eye = Vec3::new(0.0, 1.0, z);
            assert_eq!(TEST_BLOOM.fade(eye), haze.fade(eye), "z = {z}");
        }
    }

    #[test]
    fn several_blooms_stay_separate_in_list_order() {
        let mut room = LevelScene::test_room();
        room.blooms.push(BloomVolume {
            ramp: 2.0,
            ..TEST_BLOOM
        });
        let active = room.blooms_at(room.start().position + Vec3::Y);
        assert_eq!(
            active.iter().map(|b| b.ramp).collect::<Vec<_>>(),
            [1.5, 2.0]
        );
    }

    #[test]
    fn bad_blooms_are_refused() {
        let check = |f: &dyn Fn(&mut BloomVolume)| {
            let mut room = LevelScene::test_room();
            f(&mut room.blooms[0]);
            room.validate().is_err()
        };
        assert!(!check(&|_| {}));
        assert!(check(&|b| b.min.z = 50.0));
        assert!(check(&|b| b.strength = f32::NAN));
        assert!(check(&|b| b.blur = -1.0));
        assert!(check(&|b| b.fade_margin = 0.0));
        assert!(check(&|b| b.rotation = Quat::from_xyzw(0.0, 0.0, 0.0, 2.0)));
    }
}

#[cfg(test)]
mod skinned_tests {
    use super::*;

    #[test]
    fn the_animated_test_room_is_valid_and_its_bar_bends() {
        let room = LevelScene::test_room_animated();
        room.validate().unwrap();
        let bar = &room.skinned_models[0];
        assert_eq!(bar.parts[0].mesh.inverse_bind.len(), 2);
        // At rest (and at a whole period) the palette is the rest pose: the skin is the identity.
        let rest = bar
            .skeleton
            .model_transforms(&bar.skeleton.rest_pose())
            .unwrap();
        for t in [0.0, 2.0, 4.0, -2.0, f32::NAN] {
            let p = bar.default_palette(t);
            assert!(
                p.iter().zip(&rest).all(|(a, b)| a.abs_diff_eq(*b, 1e-5)),
                "t = {t}"
            );
        }
        // At 1 s the child is turned a quarter about +Z at its joint: the top (y = 2) moves to x = -1, y = 1.
        let p = bar.default_palette(1.0);
        let top = p[1] * bar.skeleton.bones()[1].inverse_bind;
        assert!(
            top.transform_point3(Vec3::new(0.0, 2.0, 0.0))
                .abs_diff_eq(Vec3::new(-1.0, 1.0, 0.0), 1e-5)
        );
        // Half way to it (0.5 s), an eighth of a turn.
        let half = bar.default_palette(0.5)[1]
            .to_scale_rotation_translation()
            .1;
        assert!(half.abs_diff_eq(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4), 1e-5));
        // An animation index out of range gives the rest pose.
        assert_eq!(bar.palette(5, 1.0), rest);
        // The bar is in the level's bounds: its top at y = 2 near z = 2.
        assert!(room.bounds().unwrap().max.y >= 2.0);
        let mut alone = LevelScene::test_room_animated();
        alone.instances.clear();
        let b = alone.bounds().unwrap();
        assert!(
            b.min.abs_diff_eq(Vec3::new(-0.2, 0.0, 1.8), 1e-5)
                && b.max.abs_diff_eq(Vec3::new(0.2, 2.0, 2.2), 1e-5)
        );
    }

    #[test]
    fn keys_are_held_outside_their_range_and_empty_tracks_keep_the_rest_pose() {
        let bar = test_skinned_bar();
        let anim = SkeletalAnimation {
            name: "Lift".into(),
            duration: 4.0,
            tracks: vec![
                BoneTrack {
                    bone: 1,
                    translations: vec![
                        AnimKey {
                            time: 1.0,
                            value: [0.0, 1.0, 0.0],
                        },
                        AnimKey {
                            time: 2.0,
                            value: [0.0, 3.0, 0.0],
                        },
                    ],
                    rotations: Vec::new(),
                },
                // A bone that is not in the skeleton is skipped.
                BoneTrack {
                    bone: 9,
                    ..BoneTrack::default()
                },
            ],
        };
        let y = |t: f32| anim.local_pose(&bar.skeleton, t)[1].translation.y;
        assert_eq!(y(0.5), 1.0, "before the first key");
        assert!((y(1.5) - 2.0).abs() < 1e-6);
        assert_eq!(y(3.0), 3.0, "after the last key");
        assert!((y(5.5) - 2.0).abs() < 1e-6, "wraps every 4 s");
        assert_eq!(
            anim.local_pose(&bar.skeleton, 1.5)[1].rotation,
            Quat::IDENTITY
        );
    }

    #[test]
    fn broken_animated_models_and_placements_are_rejected() {
        let check = |f: &dyn Fn(&mut LevelScene)| {
            let mut s = LevelScene::test_room_animated();
            f(&mut s);
            s.validate().is_err()
        };
        assert!(!check(&|_| {}));
        assert!(check(&|s| s.skinned_instances[0].model = 3));
        assert!(check(&|s| s.skinned_instances[0].transform.scale.x = 0.0));
        assert!(check(&|s| s.skinned_models[0].parts[0].material = 40));
        assert!(check(&|s| s.skinned_models[0].parts[0]
            .mesh
            .inverse_bind
            .pop()
            .map(drop)
            .unwrap_or(())));
        assert!(check(&|s| s.skinned_models[0].parts[0]
            .mesh
            .indices
            .push(0)));
        assert!(check(&|s| s.skinned_models[0].default_animation = 1));
        assert!(check(&|s| s.skinned_models[0].animations[0].duration = 0.0));
        assert!(check(
            &|s| s.skinned_models[0].animations[0].tracks[0].bone = 2
        ));
        assert!(check(&|s| s.skinned_models[0].animations[0].tracks[0]
            .rotations[1]
            .time = 5.0));
        assert!(check(&|s| s.skinned_models[0].animations[0].tracks[0]
            .rotations[0]
            .value[0] = f32::NAN));
        assert!(check(&|s| s.skinned_models[0]
            .collision
            .as_mut()
            .unwrap()
            .indices
            .push(999)));
        assert!(check(&|s| s.skinned_instances[0].animation.animation = 9));
        assert!(check(&|s| s.skinned_instances[0].animation.start = f32::NAN));
        // Infinity (`AnimationPlayer::held`) is not NaN: not rejected by itself.
        assert!(!check(
            &|s| s.skinned_instances[0].animation = AnimationPlayer::held(0)
        ));
        // A lightmap (T04-047) follows the same rule as a static instance's.
        assert!(check(&|s| s.skinned_instances[0].lightmap = Some(999)));
        assert!(!check(
            &|s| s.skinned_instances[0].lightmap = Some(TEST_LIGHTMAP_TEXTURE)
        ));
    }

    /// T04-048: two placements of the same model, started at different times, play independently instead of
    /// sharing one clock (the bug this task fixes).
    #[test]
    fn two_placements_of_one_model_play_from_their_own_start() {
        let bar = test_skinned_bar();
        let early = AnimationPlayer::looping(0, 0.0);
        let late = AnimationPlayer::looping(0, 1.0);
        let same = |a: &[Mat4], b: &[Mat4]| a.iter().zip(b).all(|(x, y)| x.abs_diff_eq(*y, 1e-4));
        // At the same moment, a placement that started a second later is a second further behind (1.6 and 0.6 s
        // into `Bend`, not the symmetric 0.5/1.5 s pair that happens to bend the same amount both ways).
        assert!(same(
            &bar.palette_of(&early, 1.6),
            &bar.palette_of(&late, 2.6)
        ));
        assert!(!same(
            &bar.palette_of(&early, 1.6),
            &bar.palette_of(&late, 1.6)
        ));
    }

    /// T04-048: a once-only animation holds the pose it ends on instead of looping past it.
    #[test]
    fn a_once_only_animation_holds_its_last_pose() {
        let bar = test_skinned_bar();
        let once = AnimationPlayer::once(0, 0.0);
        // `Bend` lasts 2 s. A looping player at 5 s is 1 s into its third pass (bent); a once player instead holds
        // where its single pass ended, at 2 s (back to rest).
        let held = bar.palette_of(&once, 5.0);
        assert_eq!(held, bar.palette_of(&once, 2.0), "holds where it ended");
        let looped = bar.palette_of(&AnimationPlayer::looping(0, 0.0), 5.0);
        assert_ne!(held, looped, "a loop would be bent at 5 s instead");
        // Before its start, a once player holds its first frame (rest), the same as after T04-048's `held`.
        let not_yet = AnimationPlayer::once(0, 10.0);
        assert_eq!(
            bar.palette_of(&not_yet, 0.0),
            bar.palette_of(&AnimationPlayer::held(0), 0.0)
        );
    }

    /// T04-048: the loader's default player loops an idle/default-named animation from the scene's start, and
    /// holds an action at its first frame until the game drives it (module docs, "Animated models").
    #[test]
    fn default_player_loops_idle_or_default_and_holds_an_action() {
        let mut idle = test_skinned_bar();
        idle.animations[0].name = "Idle_Sway".to_string();
        let player = idle.default_player();
        assert_eq!(player.mode, PlayMode::Loop);
        assert_eq!(player.start, 0.0);

        let action = test_skinned_bar(); // "Bend": no idle/default match.
        let player = action.default_player();
        assert_eq!(player.mode, PlayMode::Once);
        assert!(player.start.is_infinite() && player.start > 0.0);
        // Held at every finite time, including negative ones: it never catches up to `start`.
        let rest = action
            .skeleton
            .model_transforms(&action.skeleton.rest_pose())
            .unwrap();
        for t in [0.0, 1.0, 1_000.0, -5.0] {
            assert_eq!(action.palette_of(&player, t), rest, "t = {t}");
        }
    }

    #[test]
    fn is_idle_or_default_matches_idle_anywhere_and_default_exactly() {
        assert!(is_idle_or_default("Idle"));
        assert!(is_idle_or_default("crouchidle"));
        assert!(is_idle_or_default("DEFAULT"));
        assert!(!is_idle_or_default("Open"));
        assert!(!is_idle_or_default("default_open"), "not an exact match");
    }

    /// T04-048: the box used to cull a model's draws contains its bind pose with room to spare.
    #[test]
    fn culling_bounds_grow_the_bind_pose_box() {
        let bar = test_skinned_bar();
        let bind = bar.bounds().unwrap();
        let culling = bar.culling_bounds().unwrap();
        assert!(culling.min.cmple(bind.min).all() && culling.max.cmpge(bind.max).all());
        assert!(culling.min != bind.min || culling.max != bind.max);
    }
}
