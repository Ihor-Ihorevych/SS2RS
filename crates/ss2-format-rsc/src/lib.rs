//! ss2-format-rsc — the `.rsc` resource databases and the `.ctl` character tools (roadmap
//! T02-023; spec `docs/re/rsc-ctl.md`).
//!
//! Both extensions use the container of the `.ep` files (a `CResourceFile` with exactly one shared
//! root object), so the generic tree decoder of `ss2-format-ep` ([`ParamTree`]) does the reading
//! and this crate adds what the two formats mean:
//!
//! - **`.rsc`** ([`ResourceDatabase`]) is a *generic* container: the class of the root object says
//!   what the file is ([`RscKind`]): one of 99 menu screen layouts, a skeleton, or one of ten
//!   global databases (cameras, game parameters, HUD, crosshairs, power-ups, menu fonts,
//!   spawning). The full tree is always available ([`ResourceDatabase::root`]); menu screens
//!   ([`MenuScreen`]) and the camera database ([`Camera`], [`Graph`]) also have typed views.
//! - **`.ctl`** ([`CharacterTool`]) is one kind: a held, picked-up or thrown object described by
//!   four animations, an attachment, launch velocities and an optional entity creator.
//!
//! References to other files (textures, fonts, sounds, effects, models, other `.rsc` files) are
//! paths in the file's own table; [`ResourceDatabase::referenced_files`] and
//! [`CharacterTool::referenced_files`] list them and the loaders record them as dependencies.
//! A root class the spec does not know fails loudly instead of being guessed.
//!
//! ```ignore
//! let db = ss2_format_rsc::ResourceDatabase::from_bytes(&bytes)?;
//! if let Some(screen) = db.menu_screen() {
//!     println!("{} gadgets for {}", screen.gadgets.len(), screen.screen_class);
//! }
//! let tool = ss2_format_rsc::CharacterTool::from_bytes(&ctl_bytes)?;
//! println!("throw animation: {}", tool.throw.name);
//! ```

use ss2_assets::{AssetLoader, LoadContext};
use ss2_core::{Error, Result};
use ss2_format_ep::{ParamObject, ParamTree, ParamValue};

const WHAT_RSC: &str = ".rsc resource database";
const WHAT_CTL: &str = ".ctl character tool";

fn rsc_err(detail: impl Into<String>) -> Error {
    Error::invalid(WHAT_RSC, detail)
}

fn ctl_err(detail: impl Into<String>) -> Error {
    Error::invalid(WHAT_CTL, detail)
}

// ---------------------------------------------------------------------------------------------
// .rsc
// ---------------------------------------------------------------------------------------------

/// What a `.rsc` file is, decided by the class of its root object (the 12 roots of the spec).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RscKind {
    /// `CMenuScreenLayout`: the layout of one menu screen.
    MenuScreen,
    /// `CSkeleton`: an enemy skeleton stored as `.rsc` (the format of `skl.md`).
    Skeleton,
    /// `CCameraDatabase`: the named cameras.
    CameraDatabase,
    /// `CGlobalGameParams`: game-wide parameters.
    GlobalGameParams,
    /// `CHudElementsParamsDatabase`: HUD element parameters.
    HudElements,
    /// `CSimpleHudBarParams`: HUD bar parameters.
    SimpleHudBar,
    /// `CPlayerPuppetCrosshairs`: crosshair parameters.
    PlayerCrosshairs,
    /// `CPowerUpEffectDatabase`: power-up effect parameters.
    PowerUpEffects,
    /// `CMenuLayoutDatabase`: menu fonts and parameters.
    MenuLayout,
    /// `CGlobalSpawningParams`: enemy spawning parameters.
    SpawningParams,
    /// `CGlobalSpawnerTweakParams`: spawner tweaks.
    SpawnerTweaks,
    /// `CGlobalSpawnerBootTweaks`: spawner boot tweaks.
    SpawnerBootTweaks,
}

impl RscKind {
    /// Every kind, in the order of the spec's table.
    pub const ALL: [Self; 12] = [
        Self::MenuScreen,
        Self::Skeleton,
        Self::CameraDatabase,
        Self::GlobalGameParams,
        Self::HudElements,
        Self::SimpleHudBar,
        Self::PlayerCrosshairs,
        Self::PowerUpEffects,
        Self::MenuLayout,
        Self::SpawningParams,
        Self::SpawnerTweaks,
        Self::SpawnerBootTweaks,
    ];

    /// The class name of the root object of this kind.
    pub fn root_class(self) -> &'static str {
        match self {
            Self::MenuScreen => "CMenuScreenLayout",
            Self::Skeleton => "CSkeleton",
            Self::CameraDatabase => "CCameraDatabase",
            Self::GlobalGameParams => "CGlobalGameParams",
            Self::HudElements => "CHudElementsParamsDatabase",
            Self::SimpleHudBar => "CSimpleHudBarParams",
            Self::PlayerCrosshairs => "CPlayerPuppetCrosshairs",
            Self::PowerUpEffects => "CPowerUpEffectDatabase",
            Self::MenuLayout => "CMenuLayoutDatabase",
            Self::SpawningParams => "CGlobalSpawningParams",
            Self::SpawnerTweaks => "CGlobalSpawnerTweakParams",
            Self::SpawnerBootTweaks => "CGlobalSpawnerBootTweaks",
        }
    }

    /// The kind whose root class is `class`.
    pub fn from_root_class(class: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.root_class() == class)
    }
}

/// A decoded `.rsc` file: its [`RscKind`] and the full parameter tree.
#[derive(Clone, Debug, PartialEq)]
pub struct ResourceDatabase {
    /// What the file is.
    pub kind: RscKind,
    /// The class names from the root class up to its oldest base.
    pub class_chain: Vec<String>,
    /// The root object with every member, base class first (see `ss2-format-ep`).
    pub root: ParamObject,
}

impl ResourceDatabase {
    /// Read a `.rsc` file.
    ///
    /// # Errors
    /// [`Error::Invalid`] when the file is not a decodable resource file with one shared root, or
    /// when the root class is not one of the 12 of the spec.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let ParamTree { class_chain, root } = ParamTree::from_bytes(bytes, WHAT_RSC)?;
        let kind = RscKind::from_root_class(&root.class)
            .ok_or_else(|| rsc_err(format!("unknown root class `{}`", root.class)))?;
        Ok(Self {
            kind,
            class_chain,
            root,
        })
    }

    /// The root class name.
    pub fn root_class(&self) -> &str {
        &self.root.class
    }

    /// Every file the database refers to (textures, fonts, sounds, effects, other databases), in
    /// order of appearance without repeats (compared without regard to case).
    pub fn referenced_files(&self) -> Vec<&str> {
        self.root.referenced_files()
    }

    /// The menu screen view, for [`RscKind::MenuScreen`] files.
    pub fn menu_screen(&self) -> Option<MenuScreen<'_>> {
        if self.kind != RscKind::MenuScreen {
            return None;
        }
        let r = &self.root;
        let own = "CMenuScreenLayout";
        let text = |name: &str| match r.get(own, name) {
            Some(ParamValue::Text(t)) => Some(t.as_str()),
            _ => None,
        };
        let screen_class = match r.get(own, "1") {
            Some(ParamValue::Ident(i)) => i.as_str(),
            _ => return None,
        };
        let gadgets = r
            .get(own, "2")?
            .as_list()?
            .iter()
            .map(ParamValue::as_object)
            .collect::<Option<Vec<_>>>()?;
        Some(MenuScreen {
            screen_class,
            underlay: text("3")?,
            overlay: text("4")?,
            gadgets,
        })
    }

    /// The cameras of a [`RscKind::CameraDatabase`] file, in file order (empty for other kinds).
    pub fn cameras(&self) -> Vec<Camera> {
        if self.kind != RscKind::CameraDatabase {
            return Vec::new();
        }
        let Some(holders) = self
            .root
            .get("CCameraDatabase", "cd_cchCameraHolders")
            .and_then(ParamValue::as_list)
        else {
            return Vec::new();
        };
        holders
            .iter()
            .filter_map(ParamValue::as_object)
            .filter_map(|h| h.find("ch_pccCameraControl")?.as_object())
            .filter_map(Camera::from_object)
            .collect()
    }
}

/// The view of a menu screen layout (`CMenuScreenLayout`).
#[derive(Clone, Debug, PartialEq)]
pub struct MenuScreen<'a> {
    /// The name of the screen's class in the game's menu code (`SCreateMatchMenuScreen`).
    pub screen_class: &'a str,
    /// The name of the underlay class (`CMSUnderlay`, `CMSPopupMenuUnderlay`, …; may be empty).
    pub underlay: &'a str,
    /// The name of the overlay class (`CMSOverlay`; may be empty).
    pub overlay: &'a str,
    /// The gadget layouts, each an object of a `CMenuGadget…Layout`-style class.
    pub gadgets: Vec<&'a ParamObject>,
}

impl MenuScreen<'_> {
    /// The gadgets whose class is `class`.
    pub fn gadgets_of_class<'s>(
        &'s self,
        class: &'s str,
    ) -> impl Iterator<Item = &'s ParamObject> + 's {
        self.gadgets
            .iter()
            .copied()
            .filter(move |g| g.class == class)
    }
}

/// A camera of the camera database (`CCameraControl`).
#[derive(Clone, Debug, PartialEq)]
pub struct Camera {
    /// The camera name puppets refer to (`3rdPersonCamera`, `HoverBikeCamera`, …).
    pub name: String,
    /// How far behind the target the camera sits, over the pitch angle in degrees.
    pub behind_distance_over_pitch: Graph,
    /// How far above the target the camera sits, over the pitch angle in degrees.
    pub above_distance_over_pitch: Graph,
    /// The lowest pitch angle in degrees.
    pub min_pitch: f32,
    /// The highest pitch angle in degrees.
    pub max_pitch: f32,
}

impl Camera {
    fn from_object(o: &ParamObject) -> Option<Self> {
        let own = "CCameraControl";
        let Some(ParamValue::Ident(name)) = o.get(own, "cc_idName") else {
            return None;
        };
        Some(Self {
            name: name.clone(),
            behind_distance_over_pitch: Graph::from_value(
                o.get(own, "cc_pgrBehindDistanceOverPitch")?,
            )?,
            above_distance_over_pitch: Graph::from_value(
                o.get(own, "cc_pgrAboveDistanceOverPitch")?,
            )?,
            min_pitch: float(o.get(own, "cc_fMinPitch")?)?,
            max_pitch: float(o.get(own, "cc_fMaxPitch")?)?,
        })
    }
}

/// A graph object (`CLinearGraph`, `CValueGraph`, `CStepUpGraph`, `CStepDownGraph`): the key
/// data only. How values between the points are interpolated is not specified yet.
#[derive(Clone, Debug, PartialEq)]
pub struct Graph {
    /// The graph class.
    pub class: String,
    /// The control points `(x, y)` in file order; a `CValueGraph` (a constant) has the one point
    /// `(0, value)`. (The unused second member of a control point is editor garbage and not
    /// kept.)
    pub points: Vec<[f32; 2]>,
}

impl Graph {
    /// Read a graph from a member value (an owned graph object).
    pub fn from_value(v: &ParamValue) -> Option<Self> {
        let o = v.as_object()?;
        let mut points = Vec::new();
        for m in &o.members {
            let Some(list) = m.value.as_list() else {
                continue;
            };
            for p in list {
                let Some(p) = p.as_object().filter(|p| p.class == "CControlPoint") else {
                    continue;
                };
                let v = p.get("CControlPoint", "3")?.as_object()?;
                let x = float(v.get("Vector2f", "1")?)?;
                let y = float(v.get("Vector2f", "2")?)?;
                points.push([x, y]);
            }
        }
        // A constant graph stores no control points: its value is `CGraph.12` (spec: "one point
        // (0, value)").
        if points.is_empty() && o.class == "CValueGraph" {
            points.push([0.0, float(o.get("CGraph", "12")?)?]);
        }
        Some(Self {
            class: o.class.clone(),
            points,
        })
    }
}

// ---------------------------------------------------------------------------------------------
// .ctl
// ---------------------------------------------------------------------------------------------

/// One of the four animations of a tool: the animation term and whether to blend into it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolAnimation {
    /// The animation name (`Draw`, `Throw`, `Body_Tool_ThrowChest`, …); empty means none.
    pub name: String,
    /// Blend into the animation.
    pub blend: bool,
}

/// A placement: a rotation quaternion and a position (`QuatVect`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuatVect {
    /// The quaternion, as stored (four floats).
    pub rotation: [f32; 4],
    /// The position (three floats).
    pub position: [f32; 3],
}

/// The launch velocities and angle ranges of a thrown tool (`CLaunchVelocities`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LaunchVelocities {
    /// The angles are relative to the attachment.
    pub relative_to_attachment: bool,
    /// Lowest launch speed.
    pub min_velocity: f32,
    /// Highest launch speed.
    pub max_velocity: f32,
    /// Lowest heading angle.
    pub min_heading: f32,
    /// Highest heading angle.
    pub max_heading: f32,
    /// Lowest pitch angle.
    pub min_pitch: f32,
    /// Highest pitch angle.
    pub max_pitch: f32,
    /// Lowest heading rotation speed.
    pub min_rotation_h: f32,
    /// Highest heading rotation speed.
    pub max_rotation_h: f32,
    /// Lowest pitch rotation speed.
    pub min_rotation_p: f32,
    /// Highest pitch rotation speed.
    pub max_rotation_p: f32,
    /// Lowest bank rotation speed.
    pub min_rotation_b: f32,
    /// Highest bank rotation speed.
    pub max_rotation_b: f32,
}

/// The class of a tool's entity creator (what the tool becomes when thrown or dropped).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolEntityCreatorKind {
    /// `CBaseItemToolEntityCreator`: an item entity (its property record is in the tree).
    BaseItem,
    /// `CDebrisToolEntityCreator`: debris with a life time and a trail.
    Debris,
    /// `CGenericProjectileToolEntityCreator`: a projectile parameter file.
    GenericProjectile,
}

/// A tool's entity creator: its kind and the full object.
#[derive(Clone, Debug, PartialEq)]
pub struct ToolEntityCreator {
    /// The class of the creator.
    pub kind: ToolEntityCreatorKind,
    /// The object with all members (item properties, life time, projectile file, …).
    pub object: ParamObject,
}

/// A decoded `.ctl` file: a character tool (`CCharacterTool`).
#[derive(Clone, Debug, PartialEq)]
pub struct CharacterTool {
    /// The animation played to draw the tool.
    pub draw: ToolAnimation,
    /// The animation played to store the tool.
    pub store: ToolAnimation,
    /// The animation played to pick the tool up.
    pub pick: ToolAnimation,
    /// The animation played to throw the tool.
    pub throw: ToolAnimation,
    /// The tool's model, a `.mdl` path (`None` for the 6 tools without one).
    pub model: Option<String>,
    /// The tool's health (−1 or 40 in the retail data).
    pub health: i32,
    /// The limb it attaches to (`RightHand`, `LeftHand`; may be empty).
    pub limb: String,
    /// The attachment name (27 distinct in the retail data; may be empty).
    pub attachment: String,
    /// The bone it attaches to (`L_Hand`, `R_Hand`; may be empty).
    pub target_bone: String,
    /// The placement relative to the target.
    pub target_offset: QuatVect,
    /// The scale.
    pub stretch: f32,
    /// The scale follows the owner's scale.
    pub stretch_with_owner: bool,
    /// The tool is thrown when its owner dies.
    pub thrown_when_owner_dies: bool,
    /// The tool is straightened up while it is thrown.
    pub straighten_up_when_throwing: bool,
    /// Launch velocities and angle ranges.
    pub launch: LaunchVelocities,
    /// What the tool becomes when thrown or dropped (`None` for 51 of the 68 retail tools).
    pub entity_creator: Option<ToolEntityCreator>,
    /// The tool's file references, in order of appearance without repeats.
    references: Vec<String>,
}

/// Typed access to the members one class declares in an object.
struct Members<'a> {
    object: &'a ParamObject,
    owner: &'static str,
}

impl<'a> Members<'a> {
    fn value(&self, name: &str) -> Result<&'a ParamValue> {
        self.object
            .get(self.owner, name)
            .ok_or_else(|| ctl_err(format!("`{}.{name}` is missing", self.owner)))
    }

    fn wrong(&self, name: &str, wanted: &str) -> Error {
        ctl_err(format!("`{}.{name}` is not {wanted}", self.owner))
    }

    fn ident(&self, name: &str) -> Result<String> {
        match self.value(name)? {
            ParamValue::Ident(s) => Ok(s.clone()),
            _ => Err(self.wrong(name, "an identifier")),
        }
    }

    fn boolean(&self, name: &str) -> Result<bool> {
        self.value(name)?
            .as_i64()
            .map(|v| v != 0)
            .ok_or_else(|| self.wrong(name, "a boolean"))
    }

    fn float(&self, name: &str) -> Result<f32> {
        float(self.value(name)?).ok_or_else(|| self.wrong(name, "a number"))
    }

    fn object(&self, name: &str) -> Result<&'a ParamObject> {
        self.value(name)?
            .as_object()
            .ok_or_else(|| self.wrong(name, "an object"))
    }

    fn animation(&self, term: &str, lerp: &str) -> Result<ToolAnimation> {
        Ok(ToolAnimation {
            name: self.ident(term)?,
            blend: self.boolean(lerp)?,
        })
    }
}

/// A number as `f32` (the files store single precision; the tree widened it losslessly).
#[allow(clippy::cast_possible_truncation)]
fn float(v: &ParamValue) -> Option<f32> {
    v.as_f64().map(|f| f as f32)
}

/// Every float of a (nested) value, in order.
#[allow(clippy::cast_possible_truncation)]
fn floats(v: &ParamValue, out: &mut Vec<f32>) {
    match v {
        ParamValue::Float(f) => out.push(*f as f32),
        ParamValue::Object(o) => o.members.iter().for_each(|m| floats(&m.value, out)),
        ParamValue::List(l) => l.iter().for_each(|x| floats(x, out)),
        _ => {}
    }
}

fn quat_vect(v: &ParamValue) -> Option<QuatVect> {
    let mut f = Vec::new();
    floats(v, &mut f);
    let f: [f32; 7] = f.try_into().ok()?;
    Some(QuatVect {
        rotation: [f[0], f[1], f[2], f[3]],
        position: [f[4], f[5], f[6]],
    })
}

impl CharacterTool {
    /// Read a `.ctl` file.
    ///
    /// # Errors
    /// [`Error::Invalid`] when the file is not a decodable resource file with one shared root,
    /// the root is not a `CCharacterTool`, a member is missing or has the wrong type, or the
    /// entity creator is of an unknown class.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let tree = ParamTree::from_bytes(bytes, WHAT_CTL)?;
        if tree.root.class != "CCharacterTool" {
            return Err(ctl_err(format!(
                "the root class is `{}`, not CCharacterTool",
                tree.root.class
            )));
        }
        let references = tree
            .referenced_files()
            .into_iter()
            .map(str::to_owned)
            .collect();
        let m = Members {
            object: &tree.root,
            owner: "CCharacterTool",
        };
        let model = match m.value("ct_pmcModel")? {
            ParamValue::Null => None,
            ParamValue::File(p) => Some(p.clone()),
            _ => return Err(m.wrong("ct_pmcModel", "a file or null")),
        };
        let health = m
            .value("ct_slHealth")?
            .as_i64()
            .and_then(|h| i32::try_from(h).ok())
            .ok_or_else(|| m.wrong("ct_slHealth", "a 32-bit integer"))?;
        let target_offset = quat_vect(m.value("ct_qvTargetOffset")?)
            .ok_or_else(|| m.wrong("ct_qvTargetOffset", "a QuatVect (7 floats)"))?;
        let lv = Members {
            object: m.object("ct_lvLaunchVelocities")?,
            owner: "CLaunchVelocities",
        };
        let launch = LaunchVelocities {
            relative_to_attachment: lv.boolean("lv_bRelativeToAttachment")?,
            min_velocity: lv.float("lv_fMinVelocity")?,
            max_velocity: lv.float("lv_fMaxVelocity")?,
            min_heading: lv.float("lv_fMinHeading")?,
            max_heading: lv.float("lv_fMaxHeading")?,
            min_pitch: lv.float("lv_fMinPitch")?,
            max_pitch: lv.float("lv_fMaxPitch")?,
            min_rotation_h: lv.float("lv_fMinRotationH")?,
            max_rotation_h: lv.float("lv_fMaxRotationH")?,
            min_rotation_p: lv.float("lv_fMinRotationP")?,
            max_rotation_p: lv.float("lv_fMaxRotationP")?,
            min_rotation_b: lv.float("lv_fMinRotationB")?,
            max_rotation_b: lv.float("lv_fMaxRotationB")?,
        };
        let entity_creator = match m.value("ct_ptecToolEntityCreator")? {
            ParamValue::Null => None,
            ParamValue::Object(o) => {
                let kind = match o.class.as_str() {
                    "CBaseItemToolEntityCreator" => ToolEntityCreatorKind::BaseItem,
                    "CDebrisToolEntityCreator" => ToolEntityCreatorKind::Debris,
                    "CGenericProjectileToolEntityCreator" => {
                        ToolEntityCreatorKind::GenericProjectile
                    }
                    other => {
                        return Err(ctl_err(format!("unknown entity creator class `{other}`")));
                    }
                };
                Some(ToolEntityCreator {
                    kind,
                    object: (**o).clone(),
                })
            }
            _ => return Err(m.wrong("ct_ptecToolEntityCreator", "an object or null")),
        };
        Ok(Self {
            draw: m.animation("ct_idDrawAnimationTerm", "ct_bLerpDrawAnimation")?,
            store: m.animation("ct_idStoreAnimationTerm", "ct_bLerpStoreAnimation")?,
            pick: m.animation("ct_idPickAnimationTerm", "ct_bLerpPickAnimation")?,
            throw: m.animation("ct_idThrowAnimationTerm", "ct_bLerpThrowAnimation")?,
            model,
            health,
            limb: m.ident("ct_idLimb")?,
            attachment: m.ident("ct_idAttachment")?,
            target_bone: m.ident("ct_idTargetBone")?,
            target_offset,
            stretch: m.float("ct_fStretch")?,
            stretch_with_owner: m.boolean("ct_bStretchWithOwner")?,
            thrown_when_owner_dies: m.boolean("ct_bThrownWhenOwnerDies")?,
            straighten_up_when_throwing: m.boolean("ct_bStraightenUpWhenThrowing")?,
            launch,
            entity_creator,
            references,
        })
    }

    /// Every file the tool refers to (its `.mdl`, and whatever its entity creator names), in
    /// order of appearance without repeats.
    pub fn referenced_files(&self) -> Vec<&str> {
        self.references.iter().map(String::as_str).collect()
    }
}

// ---------------------------------------------------------------------------------------------
// Loaders
// ---------------------------------------------------------------------------------------------

/// The `.rsc` loader (asset type [`ResourceDatabase`]); the referenced files are recorded as
/// dependencies when a loader for their format is registered.
#[derive(Clone, Copy, Debug, Default)]
pub struct RscLoader;

impl AssetLoader for RscLoader {
    type Asset = ResourceDatabase;

    fn extensions(&self) -> &[&str] {
        &["rsc"]
    }

    fn load(&self, ctx: &mut LoadContext<'_>) -> Result<ResourceDatabase> {
        let db = ResourceDatabase::from_bytes(&ctx.read()?)?;
        for path in db.referenced_files() {
            // A reference whose format has no loader registered yet is simply not recorded.
            let _ = ctx.load_any(path);
        }
        Ok(db)
    }
}

/// The `.ctl` loader (asset type [`CharacterTool`]); the referenced files are recorded as
/// dependencies when a loader for their format is registered.
#[derive(Clone, Copy, Debug, Default)]
pub struct CtlLoader;

impl AssetLoader for CtlLoader {
    type Asset = CharacterTool;

    fn extensions(&self) -> &[&str] {
        &["ctl"]
    }

    fn load(&self, ctx: &mut LoadContext<'_>) -> Result<CharacterTool> {
        let tool = CharacterTool::from_bytes(&ctx.read()?)?;
        for path in tool.referenced_files() {
            let _ = ctx.load_any(path);
        }
        Ok(tool)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use ss2_assets::{AssetManager, LoadState};
    use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};
    use ss2_vfs::MemoryVfs;

    use super::*;

    /// Types 0 to 7 shared by the fixtures: `SLONG`, `FLOAT`, `IDENT`, `BOOL`, `CString`, a
    /// `CResource` base with one member, its container and the `CResourceFile` that lists the
    /// shared roots.
    fn base_types(b: &mut MetaBuilder) {
        b.type_block(TypeBlock::primitive(0, "SLONG", 4, 4));
        b.type_block(TypeBlock::primitive(1, "FLOAT", 4, 4));
        b.type_block(TypeBlock::primitive(2, "IDENT", 4, 4));
        b.type_block(TypeBlock::primitive(3, "BOOL", 4, 4));
        b.type_block(TypeBlock::primitive(4, "CString", 4, 4));
        b.type_block(TypeBlock::class(5, "CResource", 4, 5, None, &[("1", 0)]));
        b.type_block(TypeBlock::short(6, "CDynamicContainer<CResource>", 0, 8, 5));
        b.type_block(TypeBlock::class(
            7,
            "CResourceFile",
            3,
            5,
            None,
            &[("rf_crsShared", 6)],
        ));
    }

    /// A one-object menu screen whose root class is `root_class`, with two gadgets.
    fn menu_screen_file(root_class: &str) -> Vec<u8> {
        let mut b = MetaBuilder::new();
        let screen = b.identifier("SCreateMatchMenuScreen");
        base_types(&mut b);
        b.type_block(TypeBlock::class(
            8,
            "CMenuGadgetLayout",
            1,
            5,
            None,
            &[("1", 1)],
        ));
        b.type_block(TypeBlock::short(
            9,
            "CDynamicContainer<CMenuGadgetLayout>",
            0,
            8,
            8,
        ));
        b.type_block(TypeBlock::class(
            10,
            root_class,
            1,
            5,
            Some(5),
            &[("1", 2), ("3", 4), ("4", 4), ("2", 9)],
        ));
        b.object(0, 7, ObjectBytes::new().dynamic_container(&[1]).finish());
        b.object(
            1,
            10,
            ObjectBytes::new()
                .i32(1)
                .u32(screen)
                .cstring("CMSUnderlay")
                .cstring("")
                .dynamic_container(&[2, 3])
                .finish(),
        );
        b.object(2, 8, ObjectBytes::new().f32(0.5).finish());
        b.object(3, 8, ObjectBytes::new().f32(0.25).finish());
        b.build().unwrap()
    }

    /// A tool; `creator` is the class of its entity creator (`None` = null).
    fn tool_file(root_class: &str, with_model: bool, creator: Option<&str>) -> Vec<u8> {
        let mut b = MetaBuilder::new();
        let throw = b.identifier("Throw");
        let hand = b.identifier("RightHand");
        let empty = b.identifier("");
        let model = b.file_ref("Content/Test/Models/axe.mdl", 0x4_0000);
        b.external_type(100, "CModelConfiguration");
        base_types(&mut b);
        b.type_block(TypeBlock::class(
            8,
            "Quaternion4f",
            1,
            5,
            None,
            &[("1", 1), ("2", 1), ("3", 1), ("4", 1)],
        ));
        b.type_block(TypeBlock::class(
            9,
            "Vector3f",
            1,
            5,
            None,
            &[("1", 1), ("2", 1), ("3", 1)],
        ));
        b.type_block(TypeBlock::class(
            10,
            "QuatVect",
            1,
            5,
            None,
            &[("1", 8), ("2", 9)],
        ));
        let mut lv: Vec<(&str, u32)> = vec![("lv_bRelativeToAttachment", 3)];
        lv.extend(
            [
                "lv_fMinVelocity",
                "lv_fMaxVelocity",
                "lv_fMinHeading",
                "lv_fMaxHeading",
                "lv_fMinPitch",
                "lv_fMaxPitch",
                "lv_fMinRotationH",
                "lv_fMaxRotationH",
                "lv_fMinRotationP",
                "lv_fMaxRotationP",
                "lv_fMinRotationB",
                "lv_fMaxRotationB",
            ]
            .map(|n| (n, 1)),
        );
        b.type_block(TypeBlock::class(11, "CLaunchVelocities", 1, 5, None, &lv));
        b.type_block(TypeBlock::pointer(12, "Ptr<CModelConfiguration>", 100));
        b.type_block(TypeBlock::class(13, "CToolEntityCreator", 1, 5, None, &[]));
        b.type_block(TypeBlock::class(
            14,
            "CDebrisToolEntityCreator",
            1,
            5,
            Some(13),
            &[("dtec_fLifeTime", 1)],
        ));
        b.type_block(TypeBlock::class(
            15,
            "CMysteryToolEntityCreator",
            1,
            5,
            Some(13),
            &[],
        ));
        b.type_block(TypeBlock::pointer(16, "CToolEntityCreator*", 13));
        b.type_block(TypeBlock::class(
            17,
            root_class,
            1,
            5,
            Some(5),
            &[
                ("ct_idDrawAnimationTerm", 2),
                ("ct_bLerpDrawAnimation", 3),
                ("ct_idStoreAnimationTerm", 2),
                ("ct_bLerpStoreAnimation", 3),
                ("ct_idPickAnimationTerm", 2),
                ("ct_bLerpPickAnimation", 3),
                ("ct_idThrowAnimationTerm", 2),
                ("ct_bLerpThrowAnimation", 3),
                ("ct_pmcModel", 12),
                ("ct_slHealth", 0),
                ("ct_idLimb", 2),
                ("ct_idAttachment", 2),
                ("ct_idTargetBone", 2),
                ("ct_qvTargetOffset", 10),
                ("ct_fStretch", 1),
                ("ct_bStretchWithOwner", 3),
                ("ct_bThrownWhenOwnerDies", 3),
                ("ct_lvLaunchVelocities", 11),
                ("ct_ptecToolEntityCreator", 16),
                ("ct_bStraightenUpWhenThrowing", 3),
            ],
        ));
        if with_model {
            b.external_object(5, model, 1, 100);
        }
        b.object(0, 7, ObjectBytes::new().dynamic_container(&[1]).finish());
        let mut o = ObjectBytes::new()
            .i32(1)
            .u32(empty)
            .u32(1)
            .u32(empty)
            .u32(1)
            .u32(empty)
            .u32(0)
            .u32(throw)
            .u32(1)
            .object_ref(with_model.then_some(5))
            .i32(40)
            .u32(empty)
            .u32(hand)
            .u32(empty);
        // QuatVect: identity rotation, position (1, 2, 3).
        for f in [0.0, 0.0, 0.0, 1.0, 1.0, 2.0, 3.0] {
            o = o.f32(f);
        }
        // Stretch, stretch with owner, thrown when the owner dies, relative to the attachment.
        o = o.f32(2.0).u32(1).u32(0).u32(1);
        for f in [
            5.0, 20.0, 0.0, 10.0, 30.0, 60.0, -90.0, 90.0, -45.0, 45.0, 0.0, 1.0,
        ] {
            o = o.f32(f);
        }
        o = o.object_ref(creator.map(|_| 2)).u32(1);
        b.object(1, 17, o.finish());
        match creator {
            Some("CDebrisToolEntityCreator") => {
                b.object(2, 14, ObjectBytes::new().f32(5.0).finish());
            }
            Some(_) => {
                b.object(2, 15, Vec::new());
            }
            None => {}
        }
        b.build().unwrap()
    }

    #[test]
    fn a_menu_screen_reads_its_class_overlays_and_gadgets() {
        let db = ResourceDatabase::from_bytes(&menu_screen_file("CMenuScreenLayout")).unwrap();
        assert_eq!(db.kind, RscKind::MenuScreen);
        assert_eq!(db.root_class(), "CMenuScreenLayout");
        assert_eq!(db.class_chain, ["CMenuScreenLayout", "CResource"]);
        let screen = db.menu_screen().unwrap();
        assert_eq!(screen.screen_class, "SCreateMatchMenuScreen");
        assert_eq!(screen.underlay, "CMSUnderlay");
        assert_eq!(screen.overlay, "");
        assert_eq!(screen.gadgets.len(), 2);
        assert_eq!(screen.gadgets_of_class("CMenuGadgetLayout").count(), 2);
        assert_eq!(screen.gadgets_of_class("CListColumnInfo").count(), 0);
        assert!(db.cameras().is_empty());
        assert!(db.referenced_files().is_empty());
    }

    #[test]
    fn every_root_class_of_the_spec_maps_to_one_kind() {
        for kind in RscKind::ALL {
            assert_eq!(RscKind::from_root_class(kind.root_class()), Some(kind));
        }
        assert_eq!(RscKind::from_root_class("CCharacterTool"), None);
    }

    #[test]
    fn a_database_with_an_unknown_root_class_is_refused() {
        let err = ResourceDatabase::from_bytes(&menu_screen_file("CMysteryDatabase"))
            .unwrap_err()
            .to_string();
        assert!(err.contains("unknown root class"), "{err}");
        assert!(ResourceDatabase::from_bytes(b"junk").is_err());
    }

    #[test]
    fn a_character_tool_reads_animations_placement_launch_and_model() {
        let tool = CharacterTool::from_bytes(&tool_file("CCharacterTool", true, None)).unwrap();
        assert_eq!(
            tool.throw,
            ToolAnimation {
                name: "Throw".into(),
                blend: true
            }
        );
        assert_eq!(tool.draw.name, "");
        assert!(tool.draw.blend);
        assert!(!tool.pick.blend);
        assert_eq!(tool.model.as_deref(), Some("Content/Test/Models/axe.mdl"));
        assert_eq!(tool.health, 40);
        assert_eq!(tool.attachment, "RightHand");
        assert_eq!(tool.target_offset.rotation, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(tool.target_offset.position, [1.0, 2.0, 3.0]);
        assert!((tool.stretch - 2.0).abs() < f32::EPSILON);
        assert!(tool.stretch_with_owner);
        assert!(!tool.thrown_when_owner_dies);
        assert!(tool.straighten_up_when_throwing);
        assert!(tool.launch.relative_to_attachment);
        assert!((tool.launch.min_velocity - 5.0).abs() < f32::EPSILON);
        assert!((tool.launch.max_velocity - 20.0).abs() < f32::EPSILON);
        assert!((tool.launch.min_rotation_p + 45.0).abs() < f32::EPSILON);
        assert!((tool.launch.max_rotation_b - 1.0).abs() < f32::EPSILON);
        assert!(tool.entity_creator.is_none());
        assert_eq!(tool.referenced_files(), ["Content/Test/Models/axe.mdl"]);
    }

    #[test]
    fn a_tool_without_a_model_or_with_an_entity_creator() {
        let tool = CharacterTool::from_bytes(&tool_file("CCharacterTool", false, None)).unwrap();
        assert_eq!(tool.model, None);
        assert!(tool.referenced_files().is_empty());
        let tool = CharacterTool::from_bytes(&tool_file(
            "CCharacterTool",
            true,
            Some("CDebrisToolEntityCreator"),
        ))
        .unwrap();
        let creator = tool.entity_creator.unwrap();
        assert_eq!(creator.kind, ToolEntityCreatorKind::Debris);
        assert_eq!(
            creator.object.find("dtec_fLifeTime").unwrap().as_f64(),
            Some(5.0)
        );
    }

    #[test]
    fn a_tool_with_an_unknown_creator_or_root_class_is_refused() {
        let err = CharacterTool::from_bytes(&tool_file(
            "CCharacterTool",
            true,
            Some("CMysteryToolEntityCreator"),
        ))
        .unwrap_err()
        .to_string();
        assert!(err.contains("unknown entity creator class"), "{err}");
        let err = CharacterTool::from_bytes(&tool_file("CNotATool", true, None))
            .unwrap_err()
            .to_string();
        assert!(err.contains("not CCharacterTool"), "{err}");
        assert!(CharacterTool::from_bytes(b"junk").is_err());
        // A menu screen is not a tool and a tool is not a database.
        assert!(CharacterTool::from_bytes(&menu_screen_file("CMenuScreenLayout")).is_err());
        assert!(ResourceDatabase::from_bytes(&tool_file("CCharacterTool", true, None)).is_err());
    }

    #[test]
    fn nothing_panics_on_damaged_files() {
        for good in [
            menu_screen_file("CMenuScreenLayout"),
            tool_file("CCharacterTool", true, Some("CDebrisToolEntityCreator")),
        ] {
            for len in (0..good.len()).step_by(7) {
                let _ = ResourceDatabase::from_bytes(&good[..len]);
                let _ = CharacterTool::from_bytes(&good[..len]);
            }
            for i in (0..good.len()).step_by(3) {
                let mut bad = good.clone();
                bad[i] ^= 0x33;
                let _ = ResourceDatabase::from_bytes(&bad);
                let _ = CharacterTool::from_bytes(&bad);
            }
        }
    }

    #[test]
    fn the_loaders_plug_into_the_asset_manager() {
        let rsc = menu_screen_file("CMenuScreenLayout");
        let ctl = tool_file("CCharacterTool", true, None);
        let vfs = MemoryVfs::from_files([
            ("menus/screen.rsc", rsc.as_slice()),
            ("databases/tools/axe.ctl", ctl.as_slice()),
        ])
        .unwrap();
        let assets = AssetManager::new(Arc::new(vfs));
        assets.register_loader(RscLoader).unwrap();
        assets.register_loader(CtlLoader).unwrap();
        let h = assets.load::<ResourceDatabase>("Menus/Screen.RSC").unwrap();
        assert_eq!(h.wait(), LoadState::Ready);
        assert_eq!(h.get().unwrap().kind, RscKind::MenuScreen);
        let h = assets
            .load::<CharacterTool>("Databases/Tools/Axe.CTL")
            .unwrap();
        assert_eq!(h.wait(), LoadState::Ready);
        assert_eq!(h.get().unwrap().health, 40);
        // No `.mdl` loader is registered here, so the model is not a recorded dependency.
        assert!(h.dependencies().is_empty());
    }

    /// Every file of the retail install with the given extension, read through the mounted
    /// install: `(path, bytes)`.
    fn install_files(dir: &std::path::Path, ext: &str) -> Vec<(String, Vec<u8>)> {
        use ss2_vfs::{EntryKind, Vfs, VfsPath};
        let vfs = ss2_vfs::mount_install(dir).unwrap();
        let mut stack = vec![VfsPath::root()];
        let mut out = Vec::new();
        while let Some(d) = stack.pop() {
            for e in vfs.list(&d).unwrap() {
                let path = d.join(&e.name).unwrap();
                if e.kind == EntryKind::Dir {
                    stack.push(path);
                } else if path.extension() == Some(ext) {
                    let bytes = vfs.read(&path).unwrap();
                    out.push((path.as_str().to_owned(), bytes.to_vec()));
                }
            }
        }
        out
    }

    fn extension_counts<'a>(refs: impl Iterator<Item = &'a str>) -> BTreeMap<String, usize> {
        let mut by_ext = BTreeMap::new();
        for r in refs {
            *by_ext
                .entry(r.rsplit('.').next().unwrap_or("").to_lowercase())
                .or_default() += 1;
        }
        by_ext
    }

    /// Data-gated: all 112 `.rsc` files of the retail install load with the root classes and the
    /// 270 references of `rsc-ctl.md`. Skips without `SS2_DATA_DIR`.
    #[test]
    fn every_rsc_file_of_the_real_install_loads() {
        let dir = ss2_testkit::require_game_data!();
        let files = install_files(&dir, "rsc");
        let mut kinds: BTreeMap<RscKind, usize> = BTreeMap::new();
        let mut refs_per_file: Vec<String> = Vec::new();
        let (mut cameras, mut screens, mut gadgets) = (0usize, 0usize, 0usize);
        let mut camera_names = std::collections::BTreeSet::new();
        let mut camera_graphs_have_points = true;
        let mut failures = Vec::new();
        for (path, bytes) in &files {
            match ResourceDatabase::from_bytes(bytes) {
                Ok(db) => {
                    *kinds.entry(db.kind).or_default() += 1;
                    refs_per_file.extend(db.referenced_files().into_iter().map(str::to_owned));
                    for c in db.cameras() {
                        cameras += 1;
                        camera_names.insert(c.name.clone());
                        camera_graphs_have_points &=
                            !c.behind_distance_over_pitch.points.is_empty()
                                && !c.above_distance_over_pitch.points.is_empty()
                                && c.min_pitch < c.max_pitch;
                    }
                    if let Some(screen) = db.menu_screen() {
                        screens += 1;
                        gadgets += screen.gadgets.len();
                    } else if db.kind == RscKind::MenuScreen {
                        failures.push(format!("{path}: no menu screen view"));
                    }
                }
                Err(e) => failures.push(format!("{path}: {e}")),
            }
        }
        let by_ext = extension_counts(refs_per_file.iter().map(String::as_str));
        eprintln!(
            "{} files; kinds {kinds:?}; {} references {by_ext:?}; {screens} screens with {gadgets} gadgets; {cameras} cameras; {} failures",
            files.len(),
            refs_per_file.len(),
            failures.len()
        );
        assert!(
            failures.is_empty(),
            "failures: {:?}",
            &failures[..failures.len().min(3)]
        );
        assert_eq!(files.len(), 112);
        assert_eq!(kinds[&RscKind::MenuScreen], 99);
        assert_eq!(kinds[&RscKind::Skeleton], 3);
        for kind in RscKind::ALL {
            if !matches!(kind, RscKind::MenuScreen | RscKind::Skeleton) {
                assert_eq!(kinds[&kind], 1, "{kind:?}");
            }
        }
        assert_eq!(screens, 99);
        assert_eq!(cameras, 18, "the camera database has 18 cameras");
        assert_eq!(camera_names.len(), 18, "the names are distinct");
        assert!(camera_names.contains("3rdPersonCamera"));
        assert!(
            camera_graphs_have_points,
            "every camera graph has control points"
        );
        assert_eq!(refs_per_file.len(), 270, "the references of rsc-ctl.md");
        assert_eq!(by_ext["tex"], 236);
        assert_eq!(by_ext["fnt"], 8);
        assert_eq!(by_ext["tfx"], 8);
        assert_eq!(by_ext["wav"], 5);
        assert_eq!(by_ext["rsc"], 4);
        assert_eq!(by_ext["pfx"], 4);
        assert_eq!(by_ext["mtr"], 3);
        assert_eq!(by_ext["ctl"], 1);
        assert_eq!(by_ext["ogg"], 1);
    }

    /// Data-gated: all 68 `.ctl` files load; 62 have a model and 17 an entity creator, and the
    /// references are the 68 of `rsc-ctl.md`. Skips without `SS2_DATA_DIR`.
    #[test]
    fn every_ctl_file_of_the_real_install_loads() {
        let dir = ss2_testkit::require_game_data!();
        let files = install_files(&dir, "ctl");
        let mut refs: Vec<String> = Vec::new();
        let (mut models, mut creators) = (0usize, BTreeMap::new());
        let mut failures = Vec::new();
        for (path, bytes) in &files {
            match CharacterTool::from_bytes(bytes) {
                Ok(tool) => {
                    refs.extend(tool.referenced_files().into_iter().map(str::to_owned));
                    models += usize::from(tool.model.is_some());
                    if let Some(c) = &tool.entity_creator {
                        *creators.entry(format!("{:?}", c.kind)).or_default() += 1usize;
                    }
                }
                Err(e) => failures.push(format!("{path}: {e}")),
            }
        }
        let by_ext = extension_counts(refs.iter().map(String::as_str));
        eprintln!(
            "{} files; {models} models; creators {creators:?}; {} references {by_ext:?}; {} failures",
            files.len(),
            refs.len(),
            failures.len()
        );
        assert!(
            failures.is_empty(),
            "failures: {:?}",
            &failures[..failures.len().min(3)]
        );
        assert_eq!(files.len(), 68);
        assert_eq!(models, 62);
        assert_eq!(creators.values().sum::<usize>(), 17);
        assert_eq!(creators["BaseItem"], 8);
        assert_eq!(creators["Debris"], 7);
        assert_eq!(creators["GenericProjectile"], 2);
        assert_eq!(refs.len(), 68, "the references of rsc-ctl.md");
        assert_eq!(by_ext["mdl"], 62);
        assert_eq!(by_ext["ep"], 2);
    }
}
