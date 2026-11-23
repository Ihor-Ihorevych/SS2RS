//! ss2-format-ans — `.ans` animation sets and a pose sampling API (roadmap T02-016; spec
//! `docs/re/ans.md`).
//!
//! An [`AnimSet`] is a list of [`Animation`]s. An animation has a frame range, a frame rate,
//! **tracks** and optional **events**:
//!
//! - a [`TranslationTrack`] and a [`RotationTrack`] per animated bone (parent-relative local
//!   transforms, the same space as the skeleton's rest pose),
//! - [`ScalarTrack`]s for extra float channels (they address ids that are not bones),
//! - [`AnimEvent`]s: sounds, effects, tool and attachment actions at a frame.
//!
//! A track is a **sparse** list of keys (`frame`, `value`). Tracks are addressed by the
//! **identifier name** stored in the file (`"Kleerofski"`, `"Hammer_Top"`, …; also the raw
//! identifier-table index as [`TranslationTrack::target_id`]): this crate does not depend on the
//! skeleton loader, the caller binds track names to bones.
//!
//! ```ignore
//! let set = ss2_format_ans::AnimSet::from_bytes(&bytes)?;
//! let anim = set.animation("Boss_Kleerofski_Alive").unwrap();
//! // Pose at 0.4 s of a looping animation, then the events passed in the last tick.
//! let pose = anim.sample(0.4, LoopMode::Loop);
//! for (track, t) in anim.translations.iter().zip(&pose.translations) { /* track.target, t */ }
//! for fired in anim.events_between(0.3, 0.4, LoopMode::Loop) { /* fired.event.class */ }
//! ```
//!
//! # What the spec leaves open (see `docs/ANIMATION_SAMPLING.md`)
//!
//! `docs/re/ans.md` does **not** determine how to interpolate between keys, how an animation
//! loops, or what the root-motion bone is. The sampling code makes the documented choices
//! (linear translation and scalars, shortest-arc spherical rotation, see [`sampling`]) and says so
//! in the item documentation; follow-up T01-058 settles them from the game's behaviour.

mod sampling;

pub use sampling::{FiredEvent, LoopMode, MAX_LOOPS_PER_QUERY, Pose};

use std::collections::HashMap;

use ss2_assets::{AssetLoader, LoadContext};
use ss2_core::{Error, Result};
use ss2_meta::{Container, Decoded, FromValue, MetaDeserialize, ObjectRef, Value};

const WHAT: &str = "animation set file";

fn bad(detail: impl Into<String>) -> Error {
    Error::invalid(WHAT, detail)
}

/// One keyframe: a frame number on the animation's integer frame grid and a value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Key<T> {
    /// Frame number (inside the animation's frame range, strictly ascending within a track).
    pub frame: i32,
    /// The value at that frame.
    pub value: T,
}

/// The translation of one bone over time.
#[derive(Clone, Debug, PartialEq)]
pub struct TranslationTrack {
    /// The target's name (identifier of the file; a bone name).
    pub target: String,
    /// The target's index in the file's identifier table (the "bone id" of `ans.md`; only
    /// meaningful together with the file it comes from, use [`Self::target`] across files).
    pub target_id: u32,
    /// `CVec3LenAnimEnvelope.1`: the reciprocal of the skeleton's bone length of the target
    /// (retargeting hint; not used by sampling).
    pub inverse_bone_length: f32,
    /// The keys: parent-relative local translation, strictly ascending in frame.
    pub keys: Vec<Key<[f32; 3]>>,
}

/// The rotation of one bone over time.
#[derive(Clone, Debug, PartialEq)]
pub struct RotationTrack {
    /// The target's name.
    pub target: String,
    /// The target's identifier-table index.
    pub target_id: u32,
    /// The keys: unit quaternions `[x, y, z, w]` (w last), normalised on load.
    pub keys: Vec<Key<[f32; 4]>>,
}

/// A float channel (the spec does not say what the 743 target ids animate).
#[derive(Clone, Debug, PartialEq)]
pub struct ScalarTrack {
    /// The target's name.
    pub target: String,
    /// The target's identifier-table index.
    pub target_id: u32,
    /// The keys.
    pub keys: Vec<Key<f32>>,
}

/// A value of an event member (see [`EventParam`]).
#[derive(Clone, Debug, PartialEq)]
pub enum EventValue {
    /// An integer, an `BOOL` or an enumeration (the raw number).
    Int(i64),
    /// A float.
    Float(f32),
    /// An identifier, resolved to its text (attachment, sound, effect and blast names).
    Name(String),
    /// A string.
    Text(String),
    /// A reference to a file outside this one (a sound or effect), as listed in the file.
    File(String),
    /// A reference to an object of this file (not resolved).
    Object(u32),
    /// A null reference.
    Null,
    /// A class instance stored by value (for example the shaking parameters).
    Struct(Vec<EventParam>),
    /// A list or array.
    List(Vec<EventValue>),
    /// Raw bytes.
    Bytes(Vec<u8>),
}

/// One class-specific member of an [`AnimEvent`].
#[derive(Clone, Debug, PartialEq)]
pub struct EventParam {
    /// The class that declares the member (`CSamSoundAnimEvent`).
    pub owner: String,
    /// The member name (`ssae_idSound`).
    pub name: String,
    /// The value.
    pub value: EventValue,
}

/// An event of an animation (a "notify"): something the game does at a frame.
///
/// What each class does is gameplay and not specified by `ans.md`; the class-specific members are
/// kept as generic [`EventParam`]s, in file order, and nothing is guessed.
#[derive(Clone, Debug, PartialEq)]
pub struct AnimEvent {
    /// The class name (`SPuppetSoundAnimEvent`, `CSamEffectAnimEvent`, …).
    pub class: String,
    /// `CAnimEvent.ae_fFrame`: the float frame at which the event fires (in the animation's frame
    /// numbering; inside the range for all but 4 of the 1,313 events of the install).
    pub frame: f32,
    /// `CAnimEvent.ae_ulFlags` (`AnimEventFlags`; 0x0, 0x1 and 0x4 seen).
    pub flags: u32,
    /// `CAnimEvent.ae_bOnlyInFirstLoop`: do not fire again when the animation loops.
    pub only_in_first_loop: bool,
    /// The members of the derived classes.
    pub params: Vec<EventParam>,
}

impl AnimEvent {
    /// The class-specific member called `name` (`ssae_idSound`).
    pub fn param(&self, name: &str) -> Option<&EventValue> {
        self.params
            .iter()
            .find(|p| p.name == name)
            .map(|p| &p.value)
    }

    /// The identifier text of the member `name`, if it is one.
    pub fn name_param(&self, name: &str) -> Option<&str> {
        match self.param(name)? {
            EventValue::Name(s) => Some(s),
            _ => None,
        }
    }
}

/// One animation of a set.
#[derive(Clone, Debug, PartialEq)]
pub struct Animation {
    /// `CAnimation.1`: the animation's name (an identifier, `Boss_Kleerofski_Alive`).
    pub name: String,
    /// First frame number.
    pub first_frame: i32,
    /// Last frame number (inclusive, `first_frame <= last_frame`).
    pub last_frame: i32,
    /// Seconds per frame (`CAnimation.4`, equals `1 / fps`).
    pub seconds_per_frame: f32,
    /// Frames per second (`CAnimation.5`, finite and positive).
    pub fps: f32,
    /// Translation tracks, in file order.
    pub translations: Vec<TranslationTrack>,
    /// Rotation tracks, in file order (the same bone order as the translations in the files).
    pub rotations: Vec<RotationTrack>,
    /// Float channels, in file order.
    pub scalars: Vec<ScalarTrack>,
    /// Events, in file order (not necessarily sorted by frame).
    pub events: Vec<AnimEvent>,
}

impl Animation {
    /// Index of the translation track of `target` (compared case-insensitively).
    pub fn translation_index(&self, target: &str) -> Option<usize> {
        self.translations
            .iter()
            .position(|t| t.target.eq_ignore_ascii_case(target))
    }

    /// Index of the rotation track of `target` (compared case-insensitively).
    pub fn rotation_index(&self, target: &str) -> Option<usize> {
        self.rotations
            .iter()
            .position(|t| t.target.eq_ignore_ascii_case(target))
    }

    /// Index of the scalar track of `target` (compared case-insensitively).
    pub fn scalar_index(&self, target: &str) -> Option<usize> {
        self.scalars
            .iter()
            .position(|t| t.target.eq_ignore_ascii_case(target))
    }
}

/// A `.ans` animation set.
#[derive(Clone, Debug, PartialEq)]
pub struct AnimSet {
    /// `CAnimSet.3`: an identifier (`Animations` in most files; the spec does not say whether it
    /// is a name or a group).
    pub name: String,
    /// The animations in file order.
    pub animations: Vec<Animation>,
}

impl AnimSet {
    /// The animation called `name` (compared case-insensitively).
    pub fn animation(&self, name: &str) -> Option<&Animation> {
        self.animations
            .iter()
            .find(|a| a.name.eq_ignore_ascii_case(name))
    }

    /// Read a `.ans` file.
    ///
    /// # Errors
    /// [`Error::Invalid`] naming what is wrong (not a format-8 container, no `CAnimSet`, key
    /// counts that do not match, frames out of order or outside the range, …). Never panics.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let container = Container::parse(bytes)?;
        if container.header.format_version != 8 {
            return Err(bad(format!(
                "format version {}",
                container.header.format_version
            )));
        }
        let decoded = container.decode_objects()?;
        let root = decoded
            .objects
            .iter()
            .find(|o| decoded.types.type_name(o.type_id) == Some("CAnimSet"))
            .map(|o| o.id)
            .ok_or_else(|| bad("the file has no CAnimSet object"))?;
        Self::from_object(&container, &decoded, root)
    }

    /// Read the `CAnimSet` object `id` of an already decoded container (also the sets embedded in
    /// `.mdl` files).
    ///
    /// # Errors
    /// [`Error::Invalid`] as for [`Self::from_bytes`].
    pub fn from_object(container: &Container<'_>, decoded: &Decoded, id: u32) -> Result<Self> {
        let cx = Cx::new(container, decoded);
        let raw: RawSet = cx.typed(id)?;
        let mut animations = Vec::with_capacity(raw.animations.len());
        for r in &raw.animations {
            animations.push(cx.animation(cx.id(*r, "an animation")?)?);
        }
        Ok(Self {
            name: cx.ident(raw.name)?,
            animations,
        })
    }
}

// ---------------------------------------------------------------------------------------------
// Reading

#[derive(MetaDeserialize)]
#[meta(class = "CAnimSet")]
struct RawSet {
    #[meta(name = "1")]
    animations: Vec<ObjectRef>,
    #[meta(name = "3")]
    name: u32,
}

#[derive(MetaDeserialize)]
#[meta(class = "CAnimation")]
struct RawAnimation {
    #[meta(name = "1")]
    name: u32,
    #[meta(name = "2")]
    first: i32,
    #[meta(name = "3")]
    last: i32,
    #[meta(name = "4")]
    seconds_per_frame: f32,
    #[meta(name = "5")]
    fps: f32,
    #[meta(name = "6")]
    envelopes: Vec<ObjectRef>,
    #[meta(name = "8")]
    events: ObjectRef,
}

#[derive(MetaDeserialize)]
struct RawEnvelope {
    #[meta(owner = "CAnimEnvelope", name = "1")]
    target: u32,
    #[meta(owner = "CAnimEnvelope", name = "4")]
    count: i32,
}

#[derive(MetaDeserialize)]
struct RawVec3Envelope {
    #[meta(owner = "CVec3LenAnimEnvelope", name = "1")]
    inverse_bone_length: f32,
    #[meta(owner = "CVec3LenAnimEnvelope", name = "2")]
    keys: Vec<RawVec3Key>,
}

#[derive(MetaDeserialize)]
struct RawVec3Key {
    #[meta(owner = "CAnimFrame", name = "1")]
    frame: i32,
    #[meta(owner = "CVec3AnimFrame", name = "1")]
    value: RawVec3,
}

#[derive(MetaDeserialize)]
struct RawVec3 {
    #[meta(owner = "Vector3f", name = "1")]
    x: f32,
    #[meta(owner = "Vector3f", name = "2")]
    y: f32,
    #[meta(owner = "Vector3f", name = "3")]
    z: f32,
}

#[derive(MetaDeserialize)]
struct RawQuatEnvelope {
    #[meta(owner = "CQuatAnimEnvelope", name = "1")]
    keys: Vec<RawQuatKey>,
}

#[derive(MetaDeserialize)]
struct RawQuatKey {
    #[meta(owner = "CAnimFrame", name = "1")]
    frame: i32,
    #[meta(owner = "CQuatAnimFrame", name = "1")]
    value: RawQuat,
}

#[derive(MetaDeserialize)]
struct RawQuat {
    #[meta(owner = "Quaternion4f", name = "1")]
    x: f32,
    #[meta(owner = "Quaternion4f", name = "2")]
    y: f32,
    #[meta(owner = "Quaternion4f", name = "3")]
    z: f32,
    #[meta(owner = "Quaternion4f", name = "4")]
    w: f32,
}

#[derive(MetaDeserialize)]
struct RawFloatEnvelope {
    #[meta(owner = "CFloatAnimEnvelope", name = "1")]
    keys: Vec<RawFloatKey>,
}

#[derive(MetaDeserialize)]
struct RawFloatKey {
    #[meta(owner = "CAnimFrame", name = "1")]
    frame: i32,
    #[meta(owner = "CFloatAnimFrame", name = "1")]
    value: f32,
}

#[derive(MetaDeserialize)]
struct RawEventHolder {
    #[meta(owner = "CAnimEventHolder", name = "aeh_caeEvents")]
    events: Vec<ObjectRef>,
}

/// Decoding context: the container, the decoded objects and an id index.
struct Cx<'a, 'b> {
    c: &'a Container<'b>,
    d: &'a Decoded,
    /// object id -> index into `d.objects` (then `d.extra`).
    index: HashMap<u32, usize>,
}

impl<'a, 'b> Cx<'a, 'b> {
    fn new(c: &'a Container<'b>, d: &'a Decoded) -> Self {
        let index = d
            .objects
            .iter()
            .chain(&d.extra)
            .enumerate()
            .map(|(i, o)| (o.id, i))
            .collect();
        Self { c, d, index }
    }

    fn object(&self, id: u32) -> Option<&'a ss2_meta::Object> {
        let i = *self.index.get(&id)?;
        self.d.objects.get(i).or_else(|| {
            self.d
                .extra
                .get(i.checked_sub(self.d.objects.len()).unwrap_or(usize::MAX))
        })
    }

    fn typed<T: FromValue>(&self, id: u32) -> Result<T> {
        let o = self
            .object(id)
            .ok_or_else(|| bad(format!("object {id} does not exist")))?;
        T::from_value(&o.value, &self.d.types)
    }

    fn class(&self, id: u32) -> Option<&'a str> {
        self.d.types.type_name(self.object(id)?.type_id)
    }

    fn id(&self, r: ObjectRef, what: &str) -> Result<u32> {
        match r.0 {
            Some(id) if self.object(id).is_some() => Ok(id),
            Some(id) => Err(bad(format!(
                "{what} refers to object {id}, which is not in the file"
            ))),
            None => Err(bad(format!("{what} is a null reference"))),
        }
    }

    fn ident(&self, id: u32) -> Result<String> {
        self.c
            .ident(id)
            .map(str::to_string)
            .ok_or_else(|| bad(format!("identifier {id} is not in the identifier table")))
    }

    fn animation(&self, id: u32) -> Result<Animation> {
        let raw: RawAnimation = self.typed(id)?;
        let name = self.ident(raw.name)?;
        let ctx = |what: &str| format!("animation `{name}`: {what}");
        if raw.first > raw.last {
            return Err(bad(ctx(&format!(
                "first frame {} is after last frame {}",
                raw.first, raw.last
            ))));
        }
        if !(raw.fps.is_finite() && raw.fps > 0.0) {
            return Err(bad(ctx(&format!("frame rate {} is not positive", raw.fps))));
        }
        let (first, last) = (raw.first, raw.last);
        let mut a = Animation {
            name: name.clone(),
            first_frame: first,
            last_frame: last,
            seconds_per_frame: raw.seconds_per_frame,
            fps: raw.fps,
            translations: Vec::new(),
            rotations: Vec::new(),
            scalars: Vec::new(),
            events: Vec::new(),
        };
        for r in &raw.envelopes {
            let eid = self.id(*r, "an envelope")?;
            let class = self.class(eid).unwrap_or("?");
            let head: RawEnvelope = self.typed(eid)?;
            let target = self.ident(head.target)?;
            let what =
                |detail: &str| bad(format!("animation `{name}`, track `{target}`: {detail}"));
            let target_id = head.target;
            match class {
                "CVec3LenAnimEnvelope" => {
                    let raw: RawVec3Envelope = self.typed(eid)?;
                    let keys = keys(&raw.keys, head.count, first, last, |k| {
                        (k.frame, [k.value.x, k.value.y, k.value.z])
                    })
                    .map_err(|e| what(&e))?;
                    a.translations.push(TranslationTrack {
                        target,
                        target_id,
                        inverse_bone_length: raw.inverse_bone_length,
                        keys,
                    });
                }
                "CQuatAnimEnvelope" => {
                    let raw: RawQuatEnvelope = self.typed(eid)?;
                    let mut keys = keys(&raw.keys, head.count, first, last, |k| {
                        (k.frame, [k.value.x, k.value.y, k.value.z, k.value.w])
                    })
                    .map_err(|e| what(&e))?;
                    for k in &mut keys {
                        k.value = normalise(k.value).ok_or_else(|| {
                            what("a rotation key is not a finite non-zero quaternion")
                        })?;
                    }
                    a.rotations.push(RotationTrack {
                        target,
                        target_id,
                        keys,
                    });
                }
                "CFloatAnimEnvelope" => {
                    let raw: RawFloatEnvelope = self.typed(eid)?;
                    let keys = keys(&raw.keys, head.count, first, last, |k| (k.frame, k.value))
                        .map_err(|e| what(&e))?;
                    a.scalars.push(ScalarTrack {
                        target,
                        target_id,
                        keys,
                    });
                }
                other => {
                    return Err(bad(format!(
                        "animation `{name}`: unknown envelope class `{other}`"
                    )));
                }
            }
        }
        if let Some(holder) = raw.events.0 {
            let hid = self.id(ObjectRef(Some(holder)), "the event holder")?;
            let holder: RawEventHolder = self.typed(hid)?;
            for r in &holder.events {
                let eid = self.id(*r, "an event")?;
                a.events.push(self.event(eid).map_err(|e| match e {
                    Error::Invalid { what, detail } => Error::Invalid {
                        what,
                        detail: format!("animation `{name}`: {detail}"),
                    },
                    other => other,
                })?);
            }
        }
        Ok(a)
    }

    fn event(&self, id: u32) -> Result<AnimEvent> {
        let o = self
            .object(id)
            .ok_or_else(|| bad(format!("event {id} does not exist")))?;
        let class = self.d.types.type_name(o.type_id).unwrap_or("?").to_string();
        let s = o
            .value
            .as_struct()
            .ok_or_else(|| bad(format!("event {id} is not a class instance")))?;
        let members = self
            .d
            .types
            .members(o.type_id)
            .ok_or_else(|| bad(format!("event class `{class}` has no member list")))?;
        let (mut frame, mut flags, mut only_first) = (None, None, None);
        let mut params = Vec::new();
        for (m, v) in members.iter().zip(&s.values) {
            if m.owner_name == "CAnimEvent" {
                match m.name.as_str() {
                    "ae_fFrame" => frame = v.as_f64().map(|f| f as f32),
                    "ae_ulFlags" => flags = v.as_i64().and_then(|f| u32::try_from(f).ok()),
                    "ae_bOnlyInFirstLoop" => only_first = v.as_i64().map(|b| b != 0),
                    _ => {}
                }
            } else {
                params.push(EventParam {
                    owner: m.owner_name.clone(),
                    name: m.name.clone(),
                    value: self.event_value(v, 0)?,
                });
            }
        }
        let (Some(frame), Some(flags), Some(only_in_first_loop)) = (frame, flags, only_first)
        else {
            return Err(bad(format!(
                "event class `{class}` lacks the CAnimEvent members (frame, flags, first-loop flag)"
            )));
        };
        if !frame.is_finite() {
            return Err(bad(format!(
                "an event of class `{class}` has frame {frame}"
            )));
        }
        Ok(AnimEvent {
            class,
            frame,
            flags,
            only_in_first_loop,
            params,
        })
    }

    fn event_value(&self, v: &Value, depth: usize) -> Result<EventValue> {
        const MAX_DEPTH: usize = 8;
        if depth > MAX_DEPTH {
            return Err(bad("event members are nested too deeply"));
        }
        Ok(match v {
            Value::F32(f) => EventValue::Float(*f),
            Value::F64(f) => EventValue::Float(*f as f32),
            Value::Ident(i) => EventValue::Name(self.ident(*i)?),
            Value::String(s) => EventValue::Text(s.clone()),
            Value::Ref(None) => EventValue::Null,
            Value::Ref(Some(id)) => {
                match self.c.external_objects.iter().find(|e| e.object_id == *id) {
                    Some(e) => EventValue::File(
                        self.c
                            .files
                            .get(e.file_ref as usize)
                            .map(|f| f.path.to_string())
                            .ok_or_else(|| bad("an event refers to a file that is not listed"))?,
                    ),
                    None => EventValue::Object(*id),
                }
            }
            Value::Bytes(b) => EventValue::Bytes(b.clone()),
            Value::List(items) => EventValue::List(
                items
                    .iter()
                    .map(|i| self.event_value(i, depth + 1))
                    .collect::<Result<_>>()?,
            ),
            Value::Struct(s) => {
                let members = self
                    .d
                    .types
                    .members(s.type_id)
                    .ok_or_else(|| bad("an event member has no member list"))?;
                EventValue::Struct(
                    members
                        .iter()
                        .zip(&s.values)
                        .map(|(m, v)| {
                            Ok(EventParam {
                                owner: m.owner_name.clone(),
                                name: m.name.clone(),
                                value: self.event_value(v, depth + 1)?,
                            })
                        })
                        .collect::<Result<_>>()?,
                )
            }
            other => EventValue::Int(
                other
                    .as_i64()
                    .or_else(|| match other {
                        Value::U64(v) => i64::try_from(*v).ok(),
                        _ => None,
                    })
                    .ok_or_else(|| bad("an event member is a number out of range"))?,
            ),
        })
    }
}

/// Check a key list against the invariants of `ans.md` and convert it.
fn keys<R, T>(
    raw: &[R],
    count: i32,
    first: i32,
    last: i32,
    f: impl Fn(&R) -> (i32, T),
) -> std::result::Result<Vec<Key<T>>, String> {
    if usize::try_from(count).ok() != Some(raw.len()) {
        return Err(format!(
            "the envelope says {count} keys but lists {}",
            raw.len()
        ));
    }
    if raw.is_empty() {
        return Err("the track has no keys".to_string());
    }
    let mut out: Vec<Key<T>> = Vec::with_capacity(raw.len());
    for r in raw {
        let (frame, value) = f(r);
        if frame < first || frame > last {
            return Err(format!(
                "key frame {frame} is outside the animation's frames {first}..={last}"
            ));
        }
        if out.last().is_some_and(|p| p.frame >= frame) {
            return Err(format!(
                "key frames do not strictly ascend at frame {frame}"
            ));
        }
        out.push(Key { frame, value });
    }
    Ok(out)
}

/// Normalise a quaternion; `None` for a zero or non-finite one.
fn normalise(q: [f32; 4]) -> Option<[f32; 4]> {
    let n = q.iter().map(|c| f64::from(*c).powi(2)).sum::<f64>().sqrt();
    if !n.is_finite() || n < 1e-9 {
        return None;
    }
    Some(q.map(|c| (f64::from(c) / n) as f32))
}

/// The `.ans` loader (asset type [`AnimSet`]).
#[derive(Clone, Copy, Debug, Default)]
pub struct AnsLoader;

impl AssetLoader for AnsLoader {
    type Asset = AnimSet;

    fn extensions(&self) -> &[&str] {
        &["ans"]
    }

    fn load(&self, ctx: &mut LoadContext<'_>) -> Result<AnimSet> {
        AnimSet::from_bytes(&ctx.read()?)
    }
}

#[cfg(test)]
mod tests;
