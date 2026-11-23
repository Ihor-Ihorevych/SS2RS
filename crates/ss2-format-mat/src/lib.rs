//! ss2-format-mat — physical materials and schemes (roadmap T02-019; spec `docs/re/mat.md`).
//!
//! Three small file types describe what a surface is *made of* and what happens on it:
//!
//! - **`.mat`** ([`PhysicalMaterial`]): density and friction-like numbers, a movement-speed
//!   multiplier, damage over time, and two schemes (particles and sounds) for events such as
//!   `BulletHit` or `Step`;
//! - **`.scs`** ([`SoundScheme`]): events with weighted alternatives of sounds;
//! - **`.scp`** ([`ParticleScheme`]): events with weighted alternatives of particle effects.
//!
//! An event is named by a string; a scheme may name a **base scheme** that is searched when the
//! event is not found ([`SchemeRef`]). Files are returned as data: the referenced `.wav` and `.pfx`
//! files are paths for the asset manager to load. The animation-side schemes (`.sca`, `.mas`) are
//! not specified yet.
//!
//! ```ignore
//! let m = ss2_format_mat::PhysicalMaterial::from_bytes(&bytes)?;
//! if let ss2_format_mat::SchemeRef::Embedded(sounds) = &m.sound_scheme {
//!     for alt in sounds.event("BulletHit").into_iter().flat_map(|e| &e.alternatives) {
//!         // alt.weight, alt.sound (a .wav path), alt.volume, alt.pitch ...
//!     }
//! }
//! ```

use ss2_assets::{AssetLoader, LoadContext};
use ss2_core::{Error, Result};
use ss2_meta::{Container, Decoded, MetaDeserialize, ObjectRef};

const WHAT: &str = "scheme or material file";
/// Deepest chain of embedded base schemes followed (the real files have one level).
const MAX_DEPTH: usize = 8;

/// A reference from a scheme or a material to another scheme.
#[derive(Clone, Debug, PartialEq)]
pub enum SchemeRef<T> {
    /// None.
    None,
    /// An external scheme file (the path as listed in the file's `RFIL`).
    External(String),
    /// A scheme stored inside the same file.
    Embedded(Box<T>),
}

impl<T> SchemeRef<T> {
    /// The embedded scheme, if any.
    pub fn embedded(&self) -> Option<&T> {
        match self {
            Self::Embedded(t) => Some(t),
            _ => None,
        }
    }
}

/// One alternative of a sound event.
#[derive(Clone, Debug, PartialEq)]
pub struct SoundAlternative {
    /// Relative probability (1 in nearly all of the retail alternatives).
    pub weight: i64,
    /// The `.wav` file (the path as listed in the file's `RFIL`); `None` for an empty slot.
    pub sound: Option<String>,
    /// `SndFlags` bit set (bit `0x2` is set in all retail alternatives).
    pub flags: u32,
    /// Member `.4`: looks like a pitch or speed factor (1.0 = normal).
    pub pitch: f32,
    /// Member `.5`: looks like a random variation (0.05 or 0.1 mostly).
    pub variation: f32,
    /// Member `.6`: looks like a volume (1.0 = full).
    pub volume: f32,
    /// `EPlatform` mask (bit 0 PC, bit 1 Xbox).
    pub platform_mask: u32,
}

/// A named sound event with its alternatives.
#[derive(Clone, Debug, PartialEq)]
pub struct SoundEvent {
    /// The event name (`BulletHit`, `Step`, `Sight`, …).
    pub name: String,
    /// The weighted alternatives (1 to 14 in the retail data).
    pub alternatives: Vec<SoundAlternative>,
    /// `EPlatform` mask of the event.
    pub platform_mask: u32,
}

/// A `.scs` sound scheme.
#[derive(Clone, Debug, PartialEq)]
pub struct SoundScheme {
    /// The events in file order.
    pub events: Vec<SoundEvent>,
    /// The scheme searched when an event is missing here.
    pub base: SchemeRef<SoundScheme>,
}

impl SoundScheme {
    /// The event called `name` (compared case-insensitively) in this scheme only.
    pub fn event(&self, name: &str) -> Option<&SoundEvent> {
        self.events
            .iter()
            .find(|e| e.name.eq_ignore_ascii_case(name))
    }

    /// Read a `.scs` file.
    ///
    /// # Errors
    /// [`Error::Invalid`] naming what is wrong.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let (container, decoded) = decode(bytes)?;
        let root = root_id(&decoded, "CSoundScheme")?;
        Self::from_object(&container, &decoded, root, 0)
    }

    fn from_object(c: &Container<'_>, d: &Decoded, id: u32, depth: usize) -> Result<Self> {
        if depth > MAX_DEPTH {
            return Err(bad("base schemes are nested too deeply"));
        }
        let raw: RawSoundScheme = d.typed(id)?;
        let mut events = Vec::with_capacity(raw.events.len());
        for e in &raw.events {
            let alternatives = e
                .alternatives
                .iter()
                .map(|a| {
                    Ok(SoundAlternative {
                        weight: a.weight,
                        sound: a.sound.0.map(|id| external(c, id, "a sound")).transpose()?,
                        flags: a.flags,
                        pitch: a.pitch,
                        variation: a.variation,
                        volume: a.volume,
                        platform_mask: a.platform,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            events.push(SoundEvent {
                name: ident(c, e.event)?,
                alternatives,
                platform_mask: e.platform,
            });
        }
        let base = scheme_ref(c, d, raw.base, |c, d, id| {
            Self::from_object(c, d, id, depth + 1)
        })?;
        Ok(Self { events, base })
    }
}

/// One alternative of a particle event.
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleAlternative {
    /// Relative probability.
    pub weight: i64,
    /// The `.pfx` file; `None` for an empty slot.
    pub effect: Option<String>,
    /// Member `.3` (0 in the sampled files, meaning unknown).
    pub extra: i64,
}

/// A named particle event with its alternatives.
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleEvent {
    /// The event name.
    pub name: String,
    /// The weighted alternatives.
    pub alternatives: Vec<ParticleAlternative>,
}

/// A `.scp` particle scheme.
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleScheme {
    /// The events in file order.
    pub events: Vec<ParticleEvent>,
    /// The scheme searched when an event is missing here (null in all retail files).
    pub base: SchemeRef<ParticleScheme>,
}

impl ParticleScheme {
    /// The event called `name` (compared case-insensitively) in this scheme only.
    pub fn event(&self, name: &str) -> Option<&ParticleEvent> {
        self.events
            .iter()
            .find(|e| e.name.eq_ignore_ascii_case(name))
    }

    /// Read a `.scp` file.
    ///
    /// # Errors
    /// [`Error::Invalid`] naming what is wrong.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let (container, decoded) = decode(bytes)?;
        let root = root_id(&decoded, "CParticleScheme")?;
        Self::from_object(&container, &decoded, root, 0)
    }

    fn from_object(c: &Container<'_>, d: &Decoded, id: u32, depth: usize) -> Result<Self> {
        if depth > MAX_DEPTH {
            return Err(bad("base schemes are nested too deeply"));
        }
        let raw: RawParticleScheme = d.typed(id)?;
        let mut events = Vec::with_capacity(raw.events.len());
        for e in &raw.events {
            let alternatives = e
                .alternatives
                .iter()
                .map(|a| {
                    Ok(ParticleAlternative {
                        weight: a.weight,
                        effect: a
                            .effect
                            .0
                            .map(|id| external(c, id, "an effect"))
                            .transpose()?,
                        extra: a.extra,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            events.push(ParticleEvent {
                name: ident(c, e.event)?,
                alternatives,
            });
        }
        let base = scheme_ref(c, d, raw.base, |c, d, id| {
            Self::from_object(c, d, id, depth + 1)
        })?;
        Ok(Self { events, base })
    }
}

/// A `.mat` physical material.
#[derive(Clone, Debug, PartialEq)]
pub struct PhysicalMaterial {
    /// `CMaterialAttributes.1`: behaves as a density in kg/m³ (550 to 8,500).
    pub density: f32,
    /// `CMaterialAttributes.2`: friction-like (0.173 ice … 0.983 high friction).
    pub friction: f32,
    /// `CMaterialAttributes.3` (0.448 to 0.772; meaning not identified).
    pub attribute3: f32,
    /// `CMaterialAttributes.4` (0.01 to 0.548; meaning not identified).
    pub attribute4: f32,
    /// Movement speed factor on this surface (1.0 normal; 0.6 to 0.35 slow).
    pub speed_multiplier: f32,
    /// The speed factor applies to players only.
    pub speed_players_only: bool,
    /// Seconds between damage ticks.
    pub damage_period: f32,
    /// Damage per tick (0 in all but `lava` 50 and `swampdeadly` 5).
    pub damage_amount: i64,
    /// The damage applies to players only.
    pub damage_players_only: bool,
    /// Particle effects per event (`BulletHit`, `ChainSaw`, `BeamHit`, …).
    pub particle_scheme: SchemeRef<ParticleScheme>,
    /// Sounds per event.
    pub sound_scheme: SchemeRef<SoundScheme>,
}

impl PhysicalMaterial {
    /// Read a `.mat` file.
    ///
    /// # Errors
    /// [`Error::Invalid`] naming what is wrong.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let (container, decoded) = decode(bytes)?;
        let root = root_id(&decoded, "CSamMaterial")?;
        let raw: RawMaterial = decoded.typed(root)?;
        Ok(Self {
            density: raw.density,
            friction: raw.friction,
            attribute3: raw.attribute3,
            attribute4: raw.attribute4,
            speed_multiplier: raw.speed_multiplier,
            speed_players_only: raw.speed_players_only,
            damage_period: raw.damage_period,
            damage_amount: raw.damage_amount,
            damage_players_only: raw.damage_players_only,
            particle_scheme: scheme_ref(&container, &decoded, raw.particles, |c, d, id| {
                ParticleScheme::from_object(c, d, id, 0)
            })?,
            sound_scheme: scheme_ref(&container, &decoded, raw.sounds, |c, d, id| {
                SoundScheme::from_object(c, d, id, 0)
            })?,
        })
    }
}

#[derive(MetaDeserialize)]
#[meta(class = "CSamMaterial")]
struct RawMaterial {
    #[meta(owner = "CMaterialAttributes", name = "1")]
    density: f32,
    #[meta(owner = "CMaterialAttributes", name = "2")]
    friction: f32,
    #[meta(owner = "CMaterialAttributes", name = "3")]
    attribute3: f32,
    #[meta(owner = "CMaterialAttributes", name = "4")]
    attribute4: f32,
    #[meta(owner = "CSamMaterial", name = "sm_fMovementSpeedMultiplier")]
    speed_multiplier: f32,
    #[meta(owner = "CSamMaterial", name = "sm_bApplySpeedMultiplierOnPlayersOnly")]
    speed_players_only: bool,
    #[meta(owner = "CSamMaterial", name = "sm_fDamageApplyPeriod")]
    damage_period: f32,
    #[meta(owner = "CSamMaterial", name = "sm_slApplyDamageAmount")]
    damage_amount: i64,
    #[meta(owner = "CSamMaterial", name = "sm_bApplyDamageOnPlayersOnly")]
    damage_players_only: bool,
    #[meta(owner = "CSamMaterial", name = "sm_ppsParticleScheme")]
    particles: ObjectRef,
    #[meta(owner = "CSamMaterial", name = "sm_pssSoundScheme")]
    sounds: ObjectRef,
}

#[derive(MetaDeserialize)]
#[meta(class = "CSoundScheme")]
struct RawSoundScheme {
    #[meta(name = "1")]
    events: Vec<RawSoundEvent>,
    #[meta(name = "2")]
    base: ObjectRef,
}

#[derive(MetaDeserialize)]
#[meta(class = "CSoundInScheme")]
struct RawSoundEvent {
    #[meta(name = "1")]
    event: u32,
    #[meta(name = "2")]
    alternatives: Vec<RawSoundAlternative>,
    #[meta(name = "3")]
    platform: u32,
}

#[derive(MetaDeserialize)]
#[meta(class = "CWeightedSoundInScheme")]
struct RawSoundAlternative {
    #[meta(name = "1")]
    weight: i64,
    #[meta(name = "2")]
    sound: ObjectRef,
    #[meta(name = "3")]
    flags: u32,
    #[meta(name = "4")]
    pitch: f32,
    #[meta(name = "5")]
    variation: f32,
    #[meta(name = "6")]
    volume: f32,
    #[meta(name = "7")]
    platform: u32,
}

#[derive(MetaDeserialize)]
#[meta(class = "CParticleScheme")]
struct RawParticleScheme {
    #[meta(name = "1")]
    events: Vec<RawParticleEvent>,
    #[meta(name = "2")]
    base: ObjectRef,
}

#[derive(MetaDeserialize)]
#[meta(class = "CPFXInScheme")]
struct RawParticleEvent {
    #[meta(name = "1")]
    event: u32,
    #[meta(name = "2")]
    alternatives: Vec<RawParticleAlternative>,
}

#[derive(MetaDeserialize)]
#[meta(class = "CWeightedPFXInScheme")]
struct RawParticleAlternative {
    #[meta(name = "1")]
    weight: i64,
    #[meta(name = "2")]
    effect: ObjectRef,
    #[meta(name = "3")]
    extra: i64,
}

fn bad(detail: impl Into<String>) -> Error {
    Error::invalid(WHAT, detail)
}

fn decode(bytes: &[u8]) -> Result<(Container<'_>, Decoded)> {
    let container = Container::parse(bytes)?;
    if container.header.format_version != 8 {
        return Err(bad(format!(
            "format version {}",
            container.header.format_version
        )));
    }
    let decoded = container.decode_objects()?;
    Ok((container, decoded))
}

/// The object id of the first object of class `class` (the file's root).
fn root_id(d: &Decoded, class: &str) -> Result<u32> {
    d.objects
        .iter()
        .find(|o| d.types.type_name(o.type_id) == Some(class))
        .map(|o| o.id)
        .ok_or_else(|| bad(format!("the file has no {class} object")))
}

fn ident(c: &Container<'_>, id: u32) -> Result<String> {
    c.ident(id)
        .map(str::to_string)
        .ok_or_else(|| bad(format!("event name {id} is not in the identifier table")))
}

fn external(c: &Container<'_>, id: u32, what: &str) -> Result<String> {
    c.external_objects
        .iter()
        .find(|e| e.object_id == id)
        .and_then(|e| c.files.get(e.file_ref as usize))
        .map(|f| f.path.to_string())
        .ok_or_else(|| {
            bad(format!(
                "{what} refers to object {id}, which is not an external file"
            ))
        })
}

/// Resolve a reference to a scheme: none, an external file, or an object of the file.
fn scheme_ref<T>(
    c: &Container<'_>,
    d: &Decoded,
    reference: ObjectRef,
    embedded: impl FnOnce(&Container<'_>, &Decoded, u32) -> Result<T>,
) -> Result<SchemeRef<T>> {
    Ok(match reference.0 {
        None => SchemeRef::None,
        Some(id) if d.object(id).is_some() => SchemeRef::Embedded(Box::new(embedded(c, d, id)?)),
        Some(id) => SchemeRef::External(external(c, id, "a scheme")?),
    })
}

macro_rules! loader {
    ($name:ident, $asset:ty, $ext:literal, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Debug, Default)]
        pub struct $name;

        impl AssetLoader for $name {
            type Asset = $asset;

            fn extensions(&self) -> &[&str] {
                &[$ext]
            }

            fn load(&self, ctx: &mut LoadContext<'_>) -> Result<$asset> {
                <$asset>::from_bytes(&ctx.read()?)
            }
        }
    };
}

loader!(
    MatLoader,
    PhysicalMaterial,
    "mat",
    "The `.mat` loader (asset type [`PhysicalMaterial`])."
);
loader!(
    ScsLoader,
    SoundScheme,
    "scs",
    "The `.scs` loader (asset type [`SoundScheme`])."
);
loader!(
    ScpLoader,
    ParticleScheme,
    "scp",
    "The `.scp` loader (asset type [`ParticleScheme`])."
);

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ss2_assets::{AssetManager, LoadState};
    use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};
    use ss2_vfs::MemoryVfs;

    use super::*;

    /// Type table shared by the synthetic files (ids 0 to 40).
    fn types(b: &mut MetaBuilder) {
        b.type_block(TypeBlock::primitive(0, "SLONG", 4, 4));
        b.type_block(TypeBlock::alias(1, "INDEX", 0));
        b.type_block(TypeBlock::primitive(2, "ULONG", 4, 4));
        b.type_block(TypeBlock::primitive(3, "FLOAT", 4, 4));
        b.type_block(TypeBlock::primitive(4, "IDENT", 4, 4));
        b.type_block(TypeBlock::enumeration(5, "SndFlags", 4));
        b.type_block(TypeBlock::enumeration(6, "EPlatform", 4));
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
        b.type_block(TypeBlock::pointer(10, "CStaticSound*", 100));
        b.type_block(TypeBlock::pointer(11, "CEffect*", 101));
        b.type_block(TypeBlock::alias(12, "BOOL", 2));
        // Sound scheme.
        b.type_block(TypeBlock::class(
            13,
            "CWeightedSoundInScheme",
            1,
            5,
            None,
            &[
                ("1", 0),
                ("2", 10),
                ("3", 5),
                ("4", 3),
                ("5", 3),
                ("6", 3),
                ("7", 6),
            ],
        ));
        b.type_block(TypeBlock::short(
            14,
            "CStaticArray<CWeightedSoundInScheme>",
            0,
            6,
            13,
        ));
        b.type_block(TypeBlock::class(
            15,
            "CSoundInScheme",
            1,
            5,
            None,
            &[("1", 4), ("2", 14), ("3", 6)],
        ));
        b.type_block(TypeBlock::short(
            16,
            "CStaticArray<CSoundInScheme>",
            0,
            6,
            15,
        ));
        b.type_block(TypeBlock::pointer(17, "CSoundScheme*", 18));
        b.type_block(TypeBlock::class(
            18,
            "CSoundScheme",
            1,
            5,
            Some(9),
            &[("1", 16), ("2", 17)],
        ));
        // Particle scheme.
        b.type_block(TypeBlock::class(
            19,
            "CWeightedPFXInScheme",
            1,
            5,
            None,
            &[("1", 0), ("2", 11), ("3", 0)],
        ));
        b.type_block(TypeBlock::short(
            20,
            "CStaticArray<CWeightedPFXInScheme>",
            0,
            6,
            19,
        ));
        b.type_block(TypeBlock::class(
            21,
            "CPFXInScheme",
            1,
            5,
            None,
            &[("1", 4), ("2", 20)],
        ));
        b.type_block(TypeBlock::short(22, "CStaticArray<CPFXInScheme>", 0, 6, 21));
        b.type_block(TypeBlock::pointer(23, "CParticleScheme*", 24));
        b.type_block(TypeBlock::class(
            24,
            "CParticleScheme",
            1,
            5,
            Some(9),
            &[("1", 22), ("2", 23)],
        ));
        // Material.
        b.type_block(TypeBlock::class(
            25,
            "CMaterialAttributes",
            1,
            5,
            Some(9),
            &[("1", 3), ("2", 3), ("4", 3), ("3", 3)],
        ));
        b.type_block(TypeBlock::class(
            26,
            "CSamMaterial",
            1,
            5,
            Some(25),
            &[
                ("sm_fMovementSpeedMultiplier", 3),
                ("sm_bApplySpeedMultiplierOnPlayersOnly", 12),
                ("sm_fDamageApplyPeriod", 3),
                ("sm_slApplyDamageAmount", 0),
                ("sm_bApplyDamageOnPlayersOnly", 12),
                ("sm_ppsParticleScheme", 23),
                ("sm_pssSoundScheme", 17),
            ],
        ));
    }

    fn alt(weight: i32, sound: Option<u32>, flags: u32, pitch: f32, var: f32, vol: f32) -> Vec<u8> {
        ObjectBytes::new()
            .i32(weight)
            .object_ref(sound)
            .u32(flags)
            .f32(pitch)
            .f32(var)
            .f32(vol)
            .u32(0x3)
            .finish()
    }

    /// A `.scs`: events `Fire` (two alternatives) and `Step` (one), base scheme external.
    fn sound_scheme_file() -> Vec<u8> {
        let mut b = MetaBuilder::new();
        let f1 = b.file_ref("Content/Test/Sounds/fire1.wav", 0);
        let f2 = b.file_ref("Content/Test/Sounds/fire2.wav", 0);
        let base = b.file_ref("Content/Test/Sounds/Default.scs", 0);
        b.external_type(100, "CStaticSound");
        b.external_type(102, "CSoundScheme");
        types(&mut b);
        let fire = b.identifier("Fire");
        let step = b.identifier("Step");
        b.external_object(10, f1, 1, 100);
        b.external_object(11, f2, 1, 100);
        b.external_object(12, base, 1, 102);
        b.object(0, 7, ObjectBytes::new().i32(1).finish());
        b.object(
            1,
            18,
            ObjectBytes::new()
                .i32(1)
                .object_ref(Some(0))
                .array_header(b"STAR", 2)
                .u32(fire)
                .array_header(b"STAR", 2)
                .raw(&alt(1, Some(10), 2, 1.0, 0.05, 1.0))
                .raw(&alt(3, Some(11), 6, 1.2, 0.1, 0.8))
                .u32(0x3)
                .u32(step)
                .array_header(b"STAR", 1)
                .raw(&alt(1, None, 3, 1.0, 0.0, 1.0))
                .u32(0x1)
                .object_ref(Some(12))
                .finish(),
        );
        b.build().unwrap()
    }

    #[test]
    fn reads_a_sound_scheme_with_weighted_alternatives_and_a_base() {
        let s = SoundScheme::from_bytes(&sound_scheme_file()).unwrap();
        assert_eq!(s.events.len(), 2);
        let fire = s.event("fire").unwrap();
        assert_eq!(fire.name, "Fire");
        assert_eq!(fire.alternatives.len(), 2);
        assert_eq!(
            fire.alternatives[0].sound.as_deref(),
            Some("Content/Test/Sounds/fire1.wav")
        );
        assert_eq!(
            (fire.alternatives[1].weight, fire.alternatives[1].flags),
            (3, 6)
        );
        assert_eq!(
            (
                fire.alternatives[1].pitch,
                fire.alternatives[1].variation,
                fire.alternatives[1].volume
            ),
            (1.2, 0.1, 0.8)
        );
        assert_eq!(fire.platform_mask, 3);
        assert_eq!(s.event("Step").unwrap().alternatives[0].sound, None);
        assert!(s.event("Nope").is_none());
        assert_eq!(
            s.base,
            SchemeRef::External("Content/Test/Sounds/Default.scs".into())
        );
    }

    /// A `.mat` with an embedded particle scheme and an embedded sound scheme.
    fn material_file() -> Vec<u8> {
        let mut b = MetaBuilder::new();
        let pfx = b.file_ref("Content/Test/Effects/hit.pfx", 0);
        let wav = b.file_ref("Content/Test/Sounds/hit.wav", 0);
        b.external_type(100, "CStaticSound");
        b.external_type(101, "CEffect");
        types(&mut b);
        let hit = b.identifier("BulletHit");
        b.external_object(10, pfx, 1, 101);
        b.external_object(11, wav, 1, 100);
        b.object(0, 7, ObjectBytes::new().i32(1).finish());
        b.object(
            1,
            26,
            ObjectBytes::new()
                .i32(1)
                .object_ref(Some(0))
                .f32(50.0)
                .f32(0.25)
                .f32(0.5)
                .f32(0.125) // attributes .1 .2 .4 .3
                .f32(0.75)
                .u32(1)
                .f32(1.0)
                .i32(50)
                .u32(0)
                .object_ref(Some(2))
                .object_ref(Some(3))
                .finish(),
        );
        b.object(
            2,
            24,
            ObjectBytes::new()
                .i32(-1)
                .object_ref(None)
                .array_header(b"STAR", 1)
                .u32(hit)
                .array_header(b"STAR", 1)
                .i32(1)
                .object_ref(Some(10))
                .i32(0)
                .object_ref(None)
                .finish(),
        );
        b.object(
            3,
            18,
            ObjectBytes::new()
                .i32(-1)
                .object_ref(None)
                .array_header(b"STAR", 1)
                .u32(hit)
                .array_header(b"STAR", 1)
                .raw(&alt(1, Some(11), 2, 1.0, 0.05, 1.0))
                .u32(0xFFFF_FFFF)
                .object_ref(None)
                .finish(),
        );
        b.build().unwrap()
    }

    #[test]
    fn reads_a_material_with_embedded_schemes() {
        let m = PhysicalMaterial::from_bytes(&material_file()).unwrap();
        assert_eq!(
            (m.density, m.friction, m.attribute3, m.attribute4),
            (50.0, 0.25, 0.125, 0.5)
        );
        assert_eq!((m.speed_multiplier, m.speed_players_only), (0.75, true));
        assert_eq!(
            (m.damage_period, m.damage_amount, m.damage_players_only),
            (1.0, 50, false)
        );
        let p = m.particle_scheme.embedded().unwrap();
        assert_eq!(
            p.event("BulletHit").unwrap().alternatives[0]
                .effect
                .as_deref(),
            Some("Content/Test/Effects/hit.pfx")
        );
        assert_eq!(p.base, SchemeRef::None);
        let s = m.sound_scheme.embedded().unwrap();
        assert_eq!(
            s.event("BulletHit").unwrap().alternatives[0]
                .sound
                .as_deref(),
            Some("Content/Test/Sounds/hit.wav")
        );
    }

    #[test]
    fn wrong_roots_and_junk_are_refused() {
        assert!(
            ParticleScheme::from_bytes(&sound_scheme_file())
                .unwrap_err()
                .to_string()
                .contains("no CParticleScheme")
        );
        assert!(
            PhysicalMaterial::from_bytes(&sound_scheme_file())
                .unwrap_err()
                .to_string()
                .contains("no CSamMaterial")
        );
        assert!(ParticleScheme::from_bytes(b"junk").is_err());
    }

    #[test]
    fn nothing_panics_on_damaged_files() {
        for good in [material_file(), sound_scheme_file()] {
            for len in (0..good.len()).step_by(11) {
                let _ = PhysicalMaterial::from_bytes(&good[..len]);
                let _ = SoundScheme::from_bytes(&good[..len]);
            }
            for i in (0..good.len()).step_by(5) {
                let mut bad = good.clone();
                bad[i] ^= 0x77;
                let _ = PhysicalMaterial::from_bytes(&bad);
                let _ = SoundScheme::from_bytes(&bad);
            }
        }
    }

    #[test]
    fn the_loaders_plug_into_the_asset_manager() {
        let (mat, scs) = (material_file(), sound_scheme_file());
        let vfs = MemoryVfs::from_files([("m/a.mat", mat.as_slice()), ("m/b.scs", scs.as_slice())])
            .unwrap();
        let assets = AssetManager::new(Arc::new(vfs));
        assets.register_loader(MatLoader).unwrap();
        assets.register_loader(ScsLoader).unwrap();
        assets.register_loader(ScpLoader).unwrap();
        let m = assets.load::<PhysicalMaterial>("M/A.MAT").unwrap();
        assert_eq!(m.wait(), LoadState::Ready);
        let s = assets.load::<SoundScheme>("m/b.scs").unwrap();
        assert_eq!(s.wait(), LoadState::Ready);
        assert_eq!(s.get().unwrap().events.len(), 2);
    }

    /// Data-gated: every `.mat`, `.scs` and `.scp` of the retail install loads (35, 37 and 7
    /// files), the named materials have the values of `mat.md`, and every sound and effect
    /// reference is a path. Skips without `SS2_DATA_DIR`.
    #[test]
    fn every_scheme_and_material_of_the_real_install_loads() {
        use ss2_vfs::{EntryKind, Vfs, VfsPath};
        use std::collections::BTreeMap;
        let dir = ss2_testkit::require_game_data!();
        let vfs = ss2_vfs::mount_install(&dir).unwrap();
        let mut stack = vec![VfsPath::root()];
        let (mut mats, mut scs, mut scp) = (BTreeMap::new(), 0usize, 0usize);
        let (mut alternatives, mut events) = (0usize, 0usize);
        let mut failures = Vec::new();
        while let Some(d) = stack.pop() {
            for e in vfs.list(&d).unwrap() {
                let path = d.join(&e.name).unwrap();
                if e.kind == EntryKind::Dir {
                    stack.push(path);
                    continue;
                }
                let bytes = match path.extension() {
                    Some("mat" | "scs" | "scp") => vfs.read(&path).unwrap(),
                    _ => continue,
                };
                let stem = path
                    .file_name()
                    .unwrap()
                    .rsplit_once('.')
                    .unwrap()
                    .0
                    .to_string();
                match path.extension() {
                    Some("mat") => match PhysicalMaterial::from_bytes(&bytes) {
                        Ok(m) => {
                            if let Some(s) = m.sound_scheme.embedded() {
                                events += s.events.len();
                                alternatives +=
                                    s.events.iter().map(|e| e.alternatives.len()).sum::<usize>();
                            }
                            mats.insert(stem, m);
                        }
                        Err(e) => failures.push(format!("{}: {e}", path.as_str())),
                    },
                    Some("scs") => match SoundScheme::from_bytes(&bytes) {
                        Ok(s) => {
                            scs += 1;
                            events += s.events.len();
                            alternatives +=
                                s.events.iter().map(|e| e.alternatives.len()).sum::<usize>();
                        }
                        Err(e) => failures.push(format!("{}: {e}", path.as_str())),
                    },
                    _ => match ParticleScheme::from_bytes(&bytes) {
                        Ok(_) => scp += 1,
                        Err(e) => failures.push(format!("{}: {e}", path.as_str())),
                    },
                }
            }
        }
        eprintln!(
            "{} .mat, {scs} .scs, {scp} .scp; {events} sound events, {alternatives} sound alternatives; {} failures",
            mats.len(),
            failures.len()
        );
        assert!(
            failures.is_empty(),
            "first failures: {:?}",
            &failures[..failures.len().min(10)]
        );
        assert_eq!((mats.len(), scs, scp), (35, 37, 7));
        let lava = &mats["lava"];
        assert_eq!(
            (
                lava.damage_amount,
                lava.damage_period,
                lava.speed_multiplier
            ),
            (50, 1.0, 0.75)
        );
        let deadly = &mats["swampdeadly"];
        assert_eq!(
            (
                deadly.damage_amount,
                deadly.damage_period,
                deadly.speed_multiplier
            ),
            (5, 0.5, 0.35)
        );
        assert_eq!(mats["ice"].density, 900.0);
        assert!((mats["ice"].friction - 0.173).abs() < 0.001);
        assert_eq!(mats["acid"].density, 1200.0);
        assert!(
            mats["acid"]
                .sound_scheme
                .embedded()
                .unwrap()
                .event("BulletHit")
                .is_some()
        );
    }
}
