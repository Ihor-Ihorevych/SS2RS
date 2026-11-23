//! `.pfx` particle effects (spec `docs/re/pfx.md`, member reference `docs/re/pfx-schema.md`).
//!
//! A `.pfx` file is one [`Effect`] (class `CEffect`) that holds a list of [`ParticleEffect`]s, each
//! one of three kinds ([`ParticleKind`]): a **machine** emitter (the general case, with exactly one
//! emitter shape), a **clouds** effect or an **atomic** (trailing) effect. Nearly every number is a
//! [`GraphHolder`]: a constant that a graph may replace.
//!
//! The enumerations the spec leaves unnamed (`EffectLifeType`, `ParticleShape`, orientation,
//! distance flags, render order) are returned as raw numbers. The blend type is the material
//! enumeration of `mtr.md`: 501 alpha blending, 503, 504 and 506 additive variants, 500 opaque
//! (constants in [`blend`]).

use ss2_core::Result;
use ss2_meta::{Container, Decoded};

use crate::graph::{GraphHolder, MinMax};
use crate::read::{Ctx, Obj};

const WHAT: &str = ".pfx particle effect";

/// The values of `CParticlesRendering.1` (`GfuBlendType`) seen in particle files.
pub mod blend {
    /// Opaque or default (4 uses).
    pub const OPAQUE: u32 = 500;
    /// Alpha blending (983 uses): the textures have an alpha channel.
    pub const ALPHA: u32 = 501;
    /// Additive (495 uses).
    pub const ADDITIVE: u32 = 503;
    /// An additive-type variant (366 uses); the difference to [`ADDITIVE`] is not determined.
    pub const ADDITIVE_B: u32 = 504;
    /// An additive-type variant (19 uses).
    pub const ADDITIVE_C: u32 = 506;
}

/// An axis-aligned box (`Box3f`): six floats, minimum corner then maximum corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundingBox {
    /// The corner with the smallest coordinates.
    pub min: [f32; 3],
    /// The corner with the largest coordinates.
    pub max: [f32; 3],
}

/// The ten graph holders of a `CRelativePlacement`: the placement relative to the owner.
#[derive(Clone, Debug, PartialEq)]
pub struct RelativePlacement {
    /// `rp_ghOffsetX/Y/Z`.
    pub offset: [GraphHolder; 3],
    /// `rp_ghVelocityX/Y/Z`.
    pub velocity: [GraphHolder; 3],
    /// `rp_ghHeading`.
    pub heading: GraphHolder,
    /// `rp_ghPitch`.
    pub pitch: GraphHolder,
    /// `rp_ghBanking`.
    pub banking: GraphHolder,
    /// `rp_ghWeight`.
    pub weight: GraphHolder,
}

/// The texture of a particle effect (`CParticleEffectTexture`).
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleTexture {
    /// The `.tex` file (the path as listed in the file's `RFIL`); `None` for a null pointer (2 of
    /// 1,887 in the retail files).
    pub texture: Option<String>,
    /// Atlas columns (1, 2, 4 or 8).
    pub columns: u32,
    /// Atlas rows (1, 2 or 4).
    pub rows: u32,
    /// `.4`: an animation rate (0 to 5).
    pub animation_rate: f32,
    /// `.5`: a flag.
    pub flag: bool,
}

/// How a particle effect is drawn (`CParticlesRendering`).
#[derive(Clone, Debug, PartialEq)]
pub struct Rendering {
    /// `GfuBlendType`, see [`blend`].
    pub blend: u32,
    /// `.3`: a flag.
    pub member_3: bool,
    /// `.4`: a float (0, 0.413, 0.5, 1).
    pub member_4: f32,
    /// `RenderOrder` (0, 1, 2 or 3; names not in the data).
    pub render_order: u32,
}

/// One key of a gradient: a position over life and a colour as ARGB (alpha in the top byte).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorKey {
    /// Position (0 to 1).
    pub position: f32,
    /// The colour, for example `0xFF858585`.
    pub color: u32,
}

/// A colour over life (`CGradient`): 1 to 7 keys.
#[derive(Clone, Debug, PartialEq)]
pub struct Gradient {
    /// The keys in file order.
    pub keys: Vec<ColorKey>,
}

/// The shape particles are emitted from (`CEmitterShape` and its three derived classes).
#[derive(Clone, Debug, PartialEq)]
pub struct Emitter {
    /// `es_ghPivotOffsetX/Y/Z`.
    pub pivot_offset: [GraphHolder; 3],
    /// `LaunchDirOverride` (1 to 3).
    pub launch_dir_override: u32,
    /// `es_ghOverrideRatio`.
    pub override_ratio: GraphHolder,
    /// The class-specific part.
    pub shape: EmitterShape,
}

/// The three emitter shapes.
#[derive(Clone, Debug, PartialEq)]
pub enum EmitterShape {
    /// `CSphereEmitterShape` (1,744): positions by heading, pitch and radius ranges.
    Sphere {
        /// `ses_ghEmitPosHMin/Max`.
        heading: MinMax,
        /// `ses_ghEmitPosPMin/Max`.
        pitch: MinMax,
        /// `ses_ghEmitPosRMin/Max`.
        radius: MinMax,
    },
    /// `CBoxEmitterShape` (76).
    Box {
        /// `bes_ghBoxW`.
        width: GraphHolder,
        /// `bes_ghBoxH`.
        height: GraphHolder,
        /// `bes_ghBoxL`.
        length: GraphHolder,
    },
    /// `CCylinderEmitterShape` (22).
    Cylinder {
        /// `ces_ghHeight`.
        height: GraphHolder,
        /// `ces_ghTopRMin/Max`.
        top_radius: MinMax,
        /// `ces_ghBottomRMin/Max`.
        bottom_radius: MinMax,
        /// `ces_ghHMin/Max`.
        h: MinMax,
    },
}

/// The emission of a machine effect (`CMachineParticleEffectLife`).
#[derive(Clone, Debug, PartialEq)]
pub struct Emission {
    /// `.21`: the emitter shape (exactly one).
    pub emitter: Emitter,
    /// `mpel_ghEmittingFreq`: emission frequency.
    pub emitting_freq: GraphHolder,
    /// `mpel_ghEmittedParticleLifeMin/Max`: life span of an emitted particle.
    pub particle_life: MinMax,
    /// `mpel_ghEmitVelocityMin/Max`: launch velocity.
    pub velocity: MinMax,
    /// `mpel_ghLaunchSpinMin/Max`.
    pub spin: MinMax,
    /// `mpel_ghBirthSizeMin/Max`: size at birth.
    pub birth_size: MinMax,
    /// `mpel_ghEmitDirHMin/Max`: emission heading range.
    pub dir_heading: MinMax,
    /// `mpel_ghEmitDirPMin/Max`: emission pitch range.
    pub dir_pitch: MinMax,
    /// `.20`: a flag.
    pub member_20: bool,
}

/// What happens to a particle during its life (`CParticleLife`).
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleLife {
    /// `.1`: the first colour gradient.
    pub gradient_1: Gradient,
    /// `.3`: the second colour gradient.
    pub gradient_3: Gradient,
    /// `.14` (-1 to 50).
    pub member_14: f32,
    /// `.2`: a flag.
    pub member_2: bool,
    /// `pl_ghSizeMultiplier`.
    pub size_multiplier: GraphHolder,
    /// `pl_ghVelocityMultiplier`.
    pub velocity_multiplier: GraphHolder,
    /// `pl_ghVelocityDamping`.
    pub velocity_damping: GraphHolder,
    /// `pl_ghWeightMin/Max`.
    pub weight: MinMax,
    /// `pl_ghSpinMultiplier`.
    pub spin_multiplier: GraphHolder,
    /// `pl_ghSpinDamping`.
    pub spin_damping: GraphHolder,
    /// `pl_ghHModifier`.
    pub h_modifier: GraphHolder,
    /// `pl_ghPModifier`.
    pub p_modifier: GraphHolder,
    /// `pl_ghBModifier`.
    pub b_modifier: GraphHolder,
    /// `pl_ghRising`.
    pub rising: GraphHolder,
}

/// Particle collision settings (`CParticlesCollision`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Collision {
    /// `.1`: particles collide.
    pub enabled: bool,
    /// `.2` (0 to 1.4; probably a bounce factor).
    pub member_2: f32,
    /// `.3` (0.2 to 3.241).
    pub member_3: f32,
}

/// The general emitter, 1,842 of the retail particle effects.
#[derive(Clone, Debug, PartialEq)]
pub struct MachineEffect {
    /// `.1`: the emission.
    pub emission: Emission,
    /// `.2`: the life of a particle.
    pub life: ParticleLife,
    /// `.3`: collision.
    pub collision: Collision,
    /// `.4` (0 to 30).
    pub member_4: f32,
    /// `ParticleShape` (0, 1, 2 or 3; names not in the data).
    pub particle_shape: u32,
    /// `.6`: a flag.
    pub member_6: bool,
}

/// `CCloudsParticleEffect` (22): numbered members with value ranges only; their meaning is not
/// worked out (spec: "not worked out beyond the value ranges").
#[derive(Clone, Debug, PartialEq)]
pub struct CloudsEffect {
    /// `.1` (8 to 60): probably a count.
    pub member_1: i32,
    /// `.2` to `.10`.
    pub floats_2_to_10: [f32; 9],
    /// `.11` and `.12`: colours.
    pub colors_11_12: [u32; 2],
    /// `.13`.
    pub member_13: bool,
    /// `.26`.
    pub member_26: bool,
    /// `.14`: a vector.
    pub member_14: [f32; 3],
    /// `.28` and `.29`: colours.
    pub colors_28_29: [u32; 2],
    /// `.16` to `.19`.
    pub floats_16_to_19: [f32; 4],
    /// `.20` (1 to 5).
    pub member_20: i32,
    /// `.21` and `.22`.
    pub floats_21_22: [f32; 2],
    /// `.23`: the clouds' own texture.
    pub texture: ParticleTexture,
    /// `.24`: the clouds' own rendering.
    pub rendering: Rendering,
    /// `.25` (-12 or 0).
    pub member_25: i32,
    /// `.30` (-160.1 or 0).
    pub member_30: f32,
}

/// `CAtomicParticleEffect` (3): trailing particles (the flies of `Flies_Rabbit.pfx`).
#[derive(Clone, Debug, PartialEq)]
pub struct AtomicEffect {
    /// `.1` (3 or 6).
    pub member_1: i32,
    /// `.2` (2, 13 or 30).
    pub member_2: i32,
    /// `.3` (0.63 to 1.311).
    pub member_3: f32,
    /// `.5`: the head colour gradient.
    pub head_gradient: Gradient,
    /// `.6`: the trail colour gradient.
    pub trail_gradient: Gradient,
    /// `ape_ghTrailWidth`.
    pub trail_width: GraphHolder,
    /// `.8` to `.11`.
    pub floats_8_to_11: [f32; 4],
}

/// The three kinds of particle effect.
#[derive(Clone, Debug, PartialEq)]
pub enum ParticleKind {
    /// `CMachineParticleEffect`.
    Machine(Box<MachineEffect>),
    /// `CCloudsParticleEffect`.
    Clouds(Box<CloudsEffect>),
    /// `CAtomicParticleEffect`.
    Atomic(Box<AtomicEffect>),
}

/// One particle effect: the members of the base `CParticleEffect` and the kind-specific part.
#[derive(Clone, Debug, PartialEq)]
pub struct ParticleEffect {
    /// `.1`: a name (mostly empty).
    pub name: String,
    /// `.2`: a flag.
    pub member_2: bool,
    /// `ParticleEffectOrientation` (0, 1, 3 or 4; names not in the data): how particles face.
    pub orientation: u32,
    /// `.8` (-0.3 to 2.1).
    pub member_8: f32,
    /// The texture and its atlas.
    pub texture: ParticleTexture,
    /// Blending and render order.
    pub rendering: Rendering,
    /// `.5` (1 to 2,000,000): probably the maximum number of particles.
    pub max_particles: u32,
    /// `.6`: placement relative to the owner.
    pub placement: RelativePlacement,
    /// `DistanceAcceptanceFlags` (8, 24, 4 or 16): a level-of-detail bit set.
    pub distance_flags: u32,
    /// `.9`: a flag.
    pub member_9: bool,
    /// `.10`: a flag (0 in all retail files).
    pub member_10: bool,
    /// The kind-specific part.
    pub kind: ParticleKind,
}

/// A decoded `.pfx` file: a `CEffect`.
#[derive(Clone, Debug, PartialEq)]
pub struct Effect {
    /// `EffectLifeType` (0, 1 or 2; names not in the data): how the effect lives.
    pub life_type: u32,
    /// `.1` (0 to 500): probably the effect's duration in seconds.
    pub member_1: f32,
    /// `.3` (4 to 1,000,000): a limit.
    pub member_3: i32,
    /// `.10` (0 to 50,000): a distance.
    pub member_10: f32,
    /// `.11` (5 to 1,000,000): a distance.
    pub member_11: f32,
    /// `.21` (-1000 to 1000).
    pub member_21: f32,
    /// `ef_boxInitialBoundingBox`.
    pub bounding_box: BoundingBox,
    /// `ef_ghStretchOverLife`.
    pub stretch_over_life: GraphHolder,
    /// `ef_ghStretchOverDistance`.
    pub stretch_over_distance: GraphHolder,
    /// `.6`: placement relative to the owner.
    pub placement: RelativePlacement,
    /// `.5`: the particle effects (1 to 14).
    pub particle_effects: Vec<ParticleEffect>,
}

impl Effect {
    /// Read a `.pfx` file.
    ///
    /// Checks the reader invariants of `pfx.md`: one `CEffect` root, every particle effect of one
    /// of the three kinds, every machine effect with one emitter of a known shape, every graph of a
    /// known class.
    ///
    /// # Errors
    /// [`ss2_core::Error::Invalid`] naming what is wrong.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let (container, decoded) = Ctx::decode(bytes, WHAT)?;
        Self::from_decoded(&container, &decoded)
    }

    fn from_decoded(container: &Container<'_>, decoded: &Decoded) -> Result<Self> {
        let ctx = Ctx {
            container,
            decoded,
            what: WHAT,
        };
        let root = ctx.root("CEffect")?;
        let b = root.sub("CEffect", "ef_boxInitialBoundingBox")?;
        let corner = |x: &str, y: &str, z: &str| -> Result<[f32; 3]> {
            Ok([b.get("Box3f", x)?, b.get("Box3f", y)?, b.get("Box3f", z)?])
        };
        Ok(Self {
            life_type: root.get("CEffect", "12")?,
            member_1: root.get("CEffect", "1")?,
            member_3: root.get("CEffect", "3")?,
            member_10: root.get("CEffect", "10")?,
            member_11: root.get("CEffect", "11")?,
            member_21: root.get("CEffect", "21")?,
            bounding_box: BoundingBox {
                min: corner("1", "2", "3")?,
                max: corner("4", "5", "6")?,
            },
            stretch_over_life: GraphHolder::read(&root, "CEffect", "ef_ghStretchOverLife")?,
            stretch_over_distance: GraphHolder::read(&root, "CEffect", "ef_ghStretchOverDistance")?,
            placement: read_placement(&root.sub("CEffect", "6")?)?,
            particle_effects: root
                .objects("CEffect", "5")?
                .iter()
                .map(read_particle_effect)
                .collect::<Result<_>>()?,
        })
    }

    /// Every texture the effect refers to, in order of appearance without repeats.
    pub fn referenced_files(&self) -> Vec<&str> {
        let mut seen = std::collections::BTreeSet::new();
        let mut out = Vec::new();
        for p in &self.particle_effects {
            let clouds = match &p.kind {
                ParticleKind::Clouds(c) => Some(&c.texture),
                _ => None,
            };
            for t in std::iter::once(&p.texture).chain(clouds) {
                if let Some(path) = t.texture.as_deref()
                    && seen.insert(path.to_lowercase())
                {
                    out.push(path);
                }
            }
        }
        out
    }

    /// Every graph holder of the effect (the constants and the graphs alike), in file order.
    pub fn graph_holders(&self) -> Vec<&GraphHolder> {
        let mut out = vec![&self.stretch_over_life, &self.stretch_over_distance];
        self.placement.collect(&mut out);
        for p in &self.particle_effects {
            p.placement.collect(&mut out);
            match &p.kind {
                ParticleKind::Machine(m) => m.collect(&mut out),
                ParticleKind::Clouds(_) => {}
                ParticleKind::Atomic(a) => out.push(&a.trail_width),
            }
        }
        out
    }
}

impl RelativePlacement {
    fn collect<'a>(&'a self, out: &mut Vec<&'a GraphHolder>) {
        out.extend(&self.offset);
        out.extend(&self.velocity);
        out.extend([&self.heading, &self.pitch, &self.banking, &self.weight]);
    }
}

impl MachineEffect {
    fn collect<'a>(&'a self, out: &mut Vec<&'a GraphHolder>) {
        let e = &self.emission;
        out.extend(&e.emitter.pivot_offset);
        out.push(&e.emitter.override_ratio);
        match &e.emitter.shape {
            EmitterShape::Sphere {
                heading,
                pitch,
                radius,
            } => {
                for m in [heading, pitch, radius] {
                    out.extend([&m.min, &m.max]);
                }
            }
            EmitterShape::Box {
                width,
                height,
                length,
            } => out.extend([width, height, length]),
            EmitterShape::Cylinder {
                height,
                top_radius,
                bottom_radius,
                h,
            } => {
                out.push(height);
                for m in [top_radius, bottom_radius, h] {
                    out.extend([&m.min, &m.max]);
                }
            }
        }
        out.push(&e.emitting_freq);
        for m in [
            &e.particle_life,
            &e.velocity,
            &e.spin,
            &e.birth_size,
            &e.dir_heading,
            &e.dir_pitch,
        ] {
            out.extend([&m.min, &m.max]);
        }
        let l = &self.life;
        out.extend([
            &l.size_multiplier,
            &l.velocity_multiplier,
            &l.velocity_damping,
            &l.weight.min,
            &l.weight.max,
            &l.spin_multiplier,
            &l.spin_damping,
            &l.h_modifier,
            &l.p_modifier,
            &l.b_modifier,
            &l.rising,
        ]);
    }
}

fn read_placement(p: &Obj<'_>) -> Result<RelativePlacement> {
    let h = |name: &str| GraphHolder::read(p, "CRelativePlacement", name);
    Ok(RelativePlacement {
        offset: [h("rp_ghOffsetX")?, h("rp_ghOffsetY")?, h("rp_ghOffsetZ")?],
        velocity: [
            h("rp_ghVelocityX")?,
            h("rp_ghVelocityY")?,
            h("rp_ghVelocityZ")?,
        ],
        heading: h("rp_ghHeading")?,
        pitch: h("rp_ghPitch")?,
        banking: h("rp_ghBanking")?,
        weight: h("rp_ghWeight")?,
    })
}

fn read_texture(t: &Obj<'_>) -> Result<ParticleTexture> {
    const T: &str = "CParticleEffectTexture";
    Ok(ParticleTexture {
        texture: t.file(T, "1")?,
        columns: t.get(T, "2")?,
        rows: t.get(T, "3")?,
        animation_rate: t.get(T, "4")?,
        flag: t.get(T, "5")?,
    })
}

fn read_rendering(r: &Obj<'_>) -> Result<Rendering> {
    const R: &str = "CParticlesRendering";
    Ok(Rendering {
        blend: r.get(R, "1")?,
        member_3: r.get(R, "3")?,
        member_4: r.get(R, "4")?,
        render_order: r.get(R, "5")?,
    })
}

fn read_gradient(o: &Obj<'_>, owner: &str, name: &str) -> Result<Gradient> {
    let g = o.sub(owner, name)?;
    let keys = g
        .subs("CGradient", "2")?
        .iter()
        .map(|k| {
            Ok(ColorKey {
                position: k.get("CColorKeyFrame", "1")?,
                color: k.get("CColorKeyFrame", "2")?,
            })
        })
        .collect::<Result<_>>()?;
    Ok(Gradient { keys })
}

fn read_particle_effect(o: &Obj<'_>) -> Result<ParticleEffect> {
    const B: &str = "CParticleEffect";
    let kind = match o.class() {
        "CMachineParticleEffect" => ParticleKind::Machine(Box::new(read_machine(o)?)),
        "CCloudsParticleEffect" => ParticleKind::Clouds(Box::new(read_clouds(o)?)),
        "CAtomicParticleEffect" => ParticleKind::Atomic(Box::new(read_atomic(o)?)),
        other => {
            return Err(o
                .ctx
                .bad(format!("`{other}` is not a known particle effect class")));
        }
    };
    Ok(ParticleEffect {
        name: o.get(B, "1")?,
        member_2: o.get(B, "2")?,
        orientation: o.get(B, "11")?,
        member_8: o.get(B, "8")?,
        texture: read_texture(&o.sub(B, "3")?)?,
        rendering: read_rendering(&o.sub(B, "4")?)?,
        max_particles: o.get(B, "5")?,
        placement: read_placement(&o.sub(B, "6")?)?,
        distance_flags: o.get(B, "7")?,
        member_9: o.get(B, "9")?,
        member_10: o.get(B, "10")?,
        kind,
    })
}

fn read_emitter(o: &Obj<'_>) -> Result<Emitter> {
    const B: &str = "CEmitterShape";
    let h = |n: &str| GraphHolder::read(o, B, n);
    let shape = match o.class() {
        "CSphereEmitterShape" => {
            let c = "CSphereEmitterShape";
            EmitterShape::Sphere {
                heading: MinMax::read(o, c, "ses_ghEmitPosHMin", "ses_ghEmitPosHMax")?,
                pitch: MinMax::read(o, c, "ses_ghEmitPosPMin", "ses_ghEmitPosPMax")?,
                radius: MinMax::read(o, c, "ses_ghEmitPosRMin", "ses_ghEmitPosRMax")?,
            }
        }
        "CBoxEmitterShape" => {
            let c = "CBoxEmitterShape";
            EmitterShape::Box {
                width: GraphHolder::read(o, c, "bes_ghBoxW")?,
                height: GraphHolder::read(o, c, "bes_ghBoxH")?,
                length: GraphHolder::read(o, c, "bes_ghBoxL")?,
            }
        }
        "CCylinderEmitterShape" => {
            let c = "CCylinderEmitterShape";
            EmitterShape::Cylinder {
                height: GraphHolder::read(o, c, "ces_ghHeight")?,
                top_radius: MinMax::read(o, c, "ces_ghTopRMin", "ces_ghTopRMax")?,
                bottom_radius: MinMax::read(o, c, "ces_ghBottomRMin", "ces_ghBottomRMax")?,
                h: MinMax::read(o, c, "ces_ghHMin", "ces_ghHMax")?,
            }
        }
        other => {
            return Err(o
                .ctx
                .bad(format!("`{other}` is not a known emitter shape class")));
        }
    };
    Ok(Emitter {
        pivot_offset: [
            h("es_ghPivotOffsetX")?,
            h("es_ghPivotOffsetY")?,
            h("es_ghPivotOffsetZ")?,
        ],
        launch_dir_override: o.get(B, "4")?,
        override_ratio: h("es_ghOverrideRatio")?,
        shape,
    })
}

fn read_machine(o: &Obj<'_>) -> Result<MachineEffect> {
    const C: &str = "CMachineParticleEffect";
    const L: &str = "CMachineParticleEffectLife";
    const P: &str = "CParticleLife";
    let e = o.sub(C, "1")?;
    let emitter = e
        .pointee(L, "21")?
        .ok_or_else(|| o.ctx.bad("a machine effect has no emitter shape"))
        .and_then(|s| read_emitter(&s))?;
    let emission = Emission {
        emitter,
        emitting_freq: GraphHolder::read(&e, L, "mpel_ghEmittingFreq")?,
        particle_life: MinMax::read(
            &e,
            L,
            "mpel_ghEmittedParticleLifeMin",
            "mpel_ghEmittedParticleLifeMax",
        )?,
        velocity: MinMax::read(&e, L, "mpel_ghEmitVelocityMin", "mpel_ghEmitVelocityMax")?,
        spin: MinMax::read(&e, L, "mpel_ghLaunchSpinMin", "mpel_ghLaunchSpinMax")?,
        birth_size: MinMax::read(&e, L, "mpel_ghBirthSizeMin", "mpel_ghBirthSizeMax")?,
        dir_heading: MinMax::read(&e, L, "mpel_ghEmitDirHMin", "mpel_ghEmitDirHMax")?,
        dir_pitch: MinMax::read(&e, L, "mpel_ghEmitDirPMin", "mpel_ghEmitDirPMax")?,
        member_20: e.get(L, "20")?,
    };
    let l = o.sub(C, "2")?;
    let lh = |n: &str| GraphHolder::read(&l, P, n);
    let life = ParticleLife {
        gradient_1: read_gradient(&l, P, "1")?,
        gradient_3: read_gradient(&l, P, "3")?,
        member_14: l.get(P, "14")?,
        member_2: l.get(P, "2")?,
        size_multiplier: lh("pl_ghSizeMultiplier")?,
        velocity_multiplier: lh("pl_ghVelocityMultiplier")?,
        velocity_damping: lh("pl_ghVelocityDamping")?,
        weight: MinMax::read(&l, P, "pl_ghWeightMin", "pl_ghWeightMax")?,
        spin_multiplier: lh("pl_ghSpinMultiplier")?,
        spin_damping: lh("pl_ghSpinDamping")?,
        h_modifier: lh("pl_ghHModifier")?,
        p_modifier: lh("pl_ghPModifier")?,
        b_modifier: lh("pl_ghBModifier")?,
        rising: lh("pl_ghRising")?,
    };
    let c = o.sub(C, "3")?;
    Ok(MachineEffect {
        emission,
        life,
        collision: Collision {
            enabled: c.get("CParticlesCollision", "1")?,
            member_2: c.get("CParticlesCollision", "2")?,
            member_3: c.get("CParticlesCollision", "3")?,
        },
        member_4: o.get(C, "4")?,
        particle_shape: o.get(C, "5")?,
        member_6: o.get(C, "6")?,
    })
}

fn read_clouds(o: &Obj<'_>) -> Result<CloudsEffect> {
    const C: &str = "CCloudsParticleEffect";
    let f = |n: &str| o.get::<f32>(C, n);
    let col = |n: &str| o.get::<u32>(C, n);
    let v = o.sub(C, "14")?;
    Ok(CloudsEffect {
        member_1: o.get(C, "1")?,
        floats_2_to_10: [
            f("2")?,
            f("3")?,
            f("4")?,
            f("5")?,
            f("6")?,
            f("7")?,
            f("8")?,
            f("9")?,
            f("10")?,
        ],
        colors_11_12: [col("11")?, col("12")?],
        member_13: o.get(C, "13")?,
        member_26: o.get(C, "26")?,
        member_14: [
            v.get("Vector3f", "1")?,
            v.get("Vector3f", "2")?,
            v.get("Vector3f", "3")?,
        ],
        colors_28_29: [col("28")?, col("29")?],
        floats_16_to_19: [f("16")?, f("17")?, f("18")?, f("19")?],
        member_20: o.get(C, "20")?,
        floats_21_22: [f("21")?, f("22")?],
        texture: read_texture(&o.sub(C, "23")?)?,
        rendering: read_rendering(&o.sub(C, "24")?)?,
        member_25: o.get(C, "25")?,
        member_30: f("30")?,
    })
}

fn read_atomic(o: &Obj<'_>) -> Result<AtomicEffect> {
    const C: &str = "CAtomicParticleEffect";
    let f = |n: &str| o.get::<f32>(C, n);
    Ok(AtomicEffect {
        member_1: o.get(C, "1")?,
        member_2: o.get(C, "2")?,
        member_3: f("3")?,
        head_gradient: read_gradient(o, C, "5")?,
        trail_gradient: read_gradient(o, C, "6")?,
        trail_width: GraphHolder::read(o, C, "ape_ghTrailWidth")?,
        floats_8_to_11: [f("8")?, f("9")?, f("10")?, f("11")?],
    })
}

#[cfg(test)]
mod tests;
