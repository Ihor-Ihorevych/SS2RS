//! The player's view-model weapon (T04-049; `docs/re/weapon-view.md`, `docs/re/weapon-classes.md`,
//! `docs/re/weapon-pickups.md`): one weapon's first-person `.mdl`, loaded standalone (never placed in a `.wld`,
//! `ss2_world::load::load_standalone_model`), uploaded as a skinned mesh, posed with its idle animation every
//! frame and handed to the viewmodel pass ([`ss2_render::Renderer::set_viewmodel`], T04-030) at the weapon's own
//! field of view.
//!
//! **What this builds.** The weapon's static placement (`docs/re/weapon-view.md` section 3 point 1: the
//! rendering file's own position and angle — every weapon here is drawn right-handed, so the left-hand mirroring
//! of that point does not apply) and field of view and near plane (section 4). The model path and the placement
//! numbers both come from the weapon's own `.ep` files (`EntityParams/<Name>Weapon.ep`,
//! `EntityParams/<Name>WeaponRendering.ep`, `docs/re/weapon-pickups.md` section 1), read with
//! [`ss2_format_ep::EntityParams`] and picked out by position rather than by name
//! (`docs/re/weapon-classes.md` section 4.0's member-order rule: most of these members have no real name in the
//! data, only a position).
//!
//! **What is a fixed, simplified choice for now** (module docs of the roadmap task): which weapon is shown. A
//! level starts with [`DEFAULT_WEAPON`] (the Colt, one of the three weapons `docs/re/weapon-pickups.md` section 2
//! says a new player starts with); the digit keys 1 to 9 ([`digit_weapon`]) swap in another weapon's view-model
//! to show that more than one loads, with **no claim** that this matches the original's own weapon-select
//! bindings (`docs/re/weapon-pickups.md` open question 1, T01-114, left open). There is no inventory, no ammo and
//! no firing: every weapon is always "held", whether or not a player could ever carry it.
//!
//! **Split off** (follow-up roadmap task): walk bob, the strafe lean, view damping and the charge/recoil
//! displacement (`docs/re/weapon-view.md` section 3 points 2 to 4). The spec gives simple formulas for all four,
//! but every one of them is driven by the player's own runtime movement state (the relative-move vector, the
//! sprint factor, a damped vertical-velocity spring) that `ss2-app`'s walk/fly controller does not expose in a
//! reusable form yet; wiring that up is player-movement game logic, not this task's model-loading and placement
//! wiring. Also split off: a real inventory, weapon switching and firing (`docs/re/weapon-pickups.md`,
//! `docs/re/hitscan-shot.md`) — the gap T04-030 and T04-046 already found in `ss2-world`/`ss2-app`.

use ss2_core::math::{Mat4, Quat, Vec3};
use ss2_format_ep::{EntityParams, ParamObject, ParamValue};
use ss2_platform_api::KeyCode;
use ss2_render::{Renderer, SkinnedDraw, SkinnedMeshHandle, TextureHandle, ViewModel};
use ss2_render_api::Aabb;
use ss2_render_api::scene_data::{MaterialData, TextureData};
use ss2_vfs::{Vfs, VfsPath};
use ss2_world::load::{LoadOptions, load_standalone_model};
use ss2_world::scene::{AnimationPlayer, SkinnedModelData};

use crate::level_session::alpha_cutoff;

/// `docs/re/weapon-pickups.md` section 1: `WeaponIndex` to the stem of its two `.ep` files
/// (`EntityParams/<stem>Weapon.ep`, `EntityParams/<stem>WeaponRendering.ep`). Index 14 (combo weapons) and -1
/// have no files and are left out, as the circular saw's blades are (no weapon index of their own).
const WEAPON_STEMS: &[(u16, &str)] = &[
    (0, "CircularSaw"),
    (1, "ZapGun"),
    (2, "AutoShotgun"),
    (3, "DoubleShotgun"),
    (4, "Uzi"),
    (5, "MiniGun"),
    (6, "RocketLauncher"),
    (7, "GrenadeLauncher"),
    (8, "PlasmaRifle"),
    (9, "KlodovikGun"),
    (10, "Cannon"),
    (11, "SeriousBomb"),
    (12, "Colt"),
    (13, "Sniper"),
    (15, "BeamGun"),
    (16, "Flamer"),
];

/// The weapon a fresh level shows (module docs: a fixed, simplified choice): the Colt, one of the three default
/// weapons (`docs/re/weapon-pickups.md` section 2: circular saw, zap gun, Colt) and the only one of the three
/// that is a plain, one-handed, never-charging model.
pub(crate) const DEFAULT_WEAPON: u16 = 12;

/// The weapon a digit key swaps in (module docs: our own simplified cycle, 1 to 9, not a claim about the
/// original's weapon-select bindings); `None` for any other key.
pub(crate) fn digit_weapon(key: KeyCode) -> Option<u16> {
    Some(match key {
        KeyCode::Digit1 => 1,
        KeyCode::Digit2 => 2,
        KeyCode::Digit3 => 3,
        KeyCode::Digit4 => 4,
        KeyCode::Digit5 => 5,
        KeyCode::Digit6 => 6,
        KeyCode::Digit7 => 7,
        KeyCode::Digit8 => 8,
        KeyCode::Digit9 => 9,
        _ => return None,
    })
}

/// The database path of weapon `index`'s own parameters file and of its rendering parameters file; `None` for an
/// index with no files.
fn weapon_paths(index: u16) -> Option<(VfsPath, VfsPath)> {
    let (_, stem) = WEAPON_STEMS.iter().find(|(i, _)| *i == index)?;
    let base = "Content/SeriousSam2/Databases/EntityParams";
    Some((
        VfsPath::new(&format!("{base}/{stem}Weapon.ep")).ok()?,
        VfsPath::new(&format!("{base}/{stem}WeaponRendering.ep")).ok()?,
    ))
}

/// The value at position `pos` among the members `owner` declares, base class first
/// (`docs/re/weapon-classes.md` section 4.0's member-order rule: a member with no real name in the data is still
/// at the position its binary declaration gives it, so position, not name, is how this module reads one).
fn nth_member<'a>(obj: &'a ParamObject, owner: &str, pos: usize) -> Option<&'a ParamValue> {
    obj.members
        .iter()
        .filter(|m| m.owner == owner)
        .nth(pos)
        .map(|m| &m.value)
}

/// A `Vector3f` member's three floats, in order (`docs/re/ep-schema.md` `Vector3f`: three `FLOAT` members, so it
/// decodes as an owned object of three numbers).
fn vec3_member(value: &ParamValue) -> Option<Vec3> {
    let obj = value.as_object()?;
    let mut floats = obj.members.iter().filter_map(|m| m.value.as_f64());
    Some(Vec3::new(
        floats.next()? as f32,
        floats.next()? as f32,
        floats.next()? as f32,
    ))
}

/// The weapon-view numbers this module draws with (`docs/re/weapon-view.md` section 3 point 1 and section 4):
/// the static placement (position, and the heading/pitch angle in degrees — the file's own bank is always 0) and
/// the field of view at 4:3 and near plane [`ViewModel`] needs.
#[derive(Clone, Copy, Debug, PartialEq)]
struct WeaponView {
    position: Vec3,
    /// Heading then pitch, degrees (`docs/re/weapon-view.md` section 3 point 1: "angle (3 floats, degrees: the
    /// first two are the turns, the third is 0 in all files)"; which of the two is heading and which is pitch is
    /// not pinned down by that wording, so this reads them in that order as a plain, documented assumption).
    heading_pitch_deg: (f32, f32),
    fov_x_at_4_3: f32,
    near: f32,
}

impl WeaponView {
    /// The placement matrix in the viewmodel pass's own camera space (`ss2_render::viewmodel` module docs): no
    /// bob, lean, damping or charge displacement (split off, this module's docs).
    fn transform(&self) -> Mat4 {
        let (heading, pitch) = self.heading_pitch_deg;
        let rotation =
            Quat::from_rotation_y(heading.to_radians()) * Quat::from_rotation_x(pitch.to_radians());
        Mat4::from_rotation_translation(rotation, self.position)
    }
}

/// Read weapon `index`'s first-person model path and view numbers from its two `.ep` files.
///
/// # Errors
/// When either file cannot be found, read or parsed, or a needed member is missing or not the shape expected.
fn read_weapon_view(vfs: &dyn Vfs, index: u16) -> Result<(String, WeaponView), String> {
    let (params_path, rendering_path) = weapon_paths(index)
        .ok_or_else(|| format!("weapon index {index} has no parameter files"))?;
    let read_ep = |path: &VfsPath| -> Result<EntityParams, String> {
        let bytes = vfs
            .read(path)
            .map_err(|e| format!("{}: {e}", path.as_str()))?;
        EntityParams::from_bytes(&bytes).map_err(|e| format!("{}: {e}", path.as_str()))
    };
    let params = read_ep(&params_path)?;
    let rendering = read_ep(&rendering_path)?;
    let model = nth_member(&params.root, "SBaseWeaponParams", 0)
        .and_then(ParamValue::as_file)
        .ok_or_else(|| format!("{}: no first-person model", params_path.as_str()))?
        .to_string();
    let position = nth_member(&rendering.root, "SBaseWeaponRenderingParams", 0)
        .and_then(vec3_member)
        .ok_or_else(|| format!("{}: no position", rendering_path.as_str()))?;
    let angle = nth_member(&rendering.root, "SBaseWeaponRenderingParams", 1)
        .and_then(vec3_member)
        .ok_or_else(|| format!("{}: no angle", rendering_path.as_str()))?;
    let fov_x_at_4_3 = nth_member(&rendering.root, "SBaseWeaponRenderingParams", 2)
        .and_then(ParamValue::as_f64)
        .ok_or_else(|| format!("{}: no field of view", rendering_path.as_str()))?
        as f32;
    let near = nth_member(&rendering.root, "SBaseWeaponRenderingParams", 7)
        .and_then(ParamValue::as_f64)
        .ok_or_else(|| format!("{}: no near plane", rendering_path.as_str()))?
        as f32;
    Ok((
        model,
        WeaponView {
            position,
            heading_pitch_deg: (angle.x, angle.y),
            fov_x_at_4_3,
            near,
        },
    ))
}

/// One weapon's view-model on the GPU (T04-049): its skinned model (posed every frame from its idle animation
/// player with [`pose`]), the meshes and textures [`load_weapon_viewmodel`] uploaded for it, and the placement
/// and field-of-view numbers [`ViewModel`] needs.
pub(crate) struct WeaponViewModel {
    skinned: SkinnedModelData,
    meshes: Vec<SkinnedMeshHandle>,
    textures: Vec<TextureHandle>,
    materials: Vec<MaterialData>,
    view: WeaponView,
    player: AnimationPlayer,
}

/// Load weapon `index`'s view-model from `vfs` and upload it to `renderer`. `None` (with a logged reason) when
/// the weapon has no files, unreadable view numbers, or no animated build to draw — the frame simply shows no
/// weapon rather than failing.
pub(crate) fn load_weapon_viewmodel(
    renderer: &mut Renderer,
    vfs: &dyn Vfs,
    index: u16,
) -> Option<WeaponViewModel> {
    let (model_path, view) = match read_weapon_view(vfs, index) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(weapon = index, error = %e, "view-model weapon's placement not read");
            return None;
        }
    };
    let path = match VfsPath::new(&model_path) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(weapon = index, model = %model_path, error = %e, "bad model path");
            return None;
        }
    };
    let built = match load_standalone_model(vfs, &path, &LoadOptions::default()) {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(weapon = index, model = %model_path, error = %e, "model not loaded");
            return None;
        }
    };
    for line in &built.skipped {
        tracing::debug!(weapon = index, "skipped: {line}");
    }
    let Some(skinned) = built.skinned else {
        tracing::warn!(
            weapon = index,
            model = %model_path,
            "the model has no animated build, so no idle pose (not drawn)"
        );
        return None;
    };
    let white = TextureData::solid([255; 4]);
    let mut textures = Vec::with_capacity(built.textures.len());
    for t in &built.textures {
        let handle = if renderer.supports(t.format) {
            renderer.create_texture(t).ok()
        } else {
            None
        };
        let handle = match handle {
            Some(h) => h,
            None => match renderer.create_texture(&white) {
                Ok(h) => h,
                Err(e) => {
                    tracing::warn!(weapon = index, error = %e, "texture not uploaded");
                    return None;
                }
            },
        };
        textures.push(handle);
    }
    let mut meshes = Vec::with_capacity(skinned.parts.len());
    for part in &skinned.parts {
        match renderer.create_skinned_mesh(&part.mesh) {
            Ok(h) => meshes.push(h),
            Err(e) => {
                tracing::warn!(weapon = index, error = %e, "mesh not uploaded");
                return None;
            }
        }
    }
    let player = skinned.default_player();
    Some(WeaponViewModel {
        skinned,
        meshes,
        textures,
        materials: built.materials,
        view,
        player,
    })
}

/// `upload`'s draws at `time` seconds, posed with its idle animation player, and its [`ViewModel`] numbers, for
/// [`Renderer::set_viewmodel`].
pub(crate) fn pose(upload: &WeaponViewModel, time: f32) -> ViewModel {
    let palette = upload.skinned.palette_of(&upload.player, time);
    let transform = upload.view.transform();
    let bounds = upload
        .skinned
        .culling_bounds()
        .unwrap_or_else(|| Aabb::new(Vec3::ZERO, Vec3::ZERO));
    let draws = upload
        .skinned
        .parts
        .iter()
        .zip(&upload.meshes)
        .map(|(part, mesh)| {
            let material = upload.materials[part.material as usize];
            SkinnedDraw {
                mesh: *mesh,
                texture: material.base_texture.map(|t| upload.textures[t as usize]),
                tint: material.base_color,
                alpha_cutoff: alpha_cutoff(&material),
                transform,
                palette: palette.clone(),
                bounds,
                lightmap: None,
            }
        })
        .collect();
    ViewModel {
        draws,
        fov_x_at_4_3: upload.view.fov_x_at_4_3,
        near: upload.view.near,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ss2_testkit::fixtures::ctsemeta::{MetaBuilder, ObjectBytes, TypeBlock};
    use ss2_vfs::MemoryVfs;

    #[test]
    fn digit_keys_one_to_nine_map_to_weapon_indices_one_to_nine() {
        assert_eq!(digit_weapon(KeyCode::Digit1), Some(1));
        assert_eq!(digit_weapon(KeyCode::Digit9), Some(9));
        assert_eq!(digit_weapon(KeyCode::Digit0), None);
        assert_eq!(digit_weapon(KeyCode::A), None);
    }

    #[test]
    fn weapon_paths_name_the_ep_files_and_leave_out_indices_without_one() {
        let (params, rendering) = weapon_paths(12).unwrap();
        // `VfsPath` is normalized to lower case (`docs/re/...`; `ss2_vfs::VfsPath` module docs).
        assert_eq!(
            params.as_str(),
            "content/serioussam2/databases/entityparams/coltweapon.ep"
        );
        assert_eq!(
            rendering.as_str(),
            "content/serioussam2/databases/entityparams/coltweaponrendering.ep"
        );
        assert!(weapon_paths(14).is_none(), "combo weapons: no files");
        assert!(weapon_paths(17).is_none());
    }

    /// A minimal weapon params file: `SBaseWeaponParams`-shaped (model at position 0, by the class's real member
    /// order) and a minimal rendering file (`SBaseWeaponRenderingParams`-shaped, position, angle, FOV, bobbing
    /// multiplier, double-weapon triple, front clip distance).
    fn weapon_ep_files(model_path: &str) -> (Vec<u8>, Vec<u8>) {
        let params = {
            let mut b = MetaBuilder::new();
            let model = b.file_ref(model_path, 0x4_0000);
            b.external_type(100, "CModelConfiguration");
            b.type_block(TypeBlock::primitive(0, "SLONG", 4, 4));
            b.type_block(TypeBlock::pointer(1, "CModelConfiguration*", 100));
            b.type_block(TypeBlock::short(2, "CDynamicContainer<CResource>", 0, 8, 3));
            b.type_block(TypeBlock::class(3, "CResource", 4, 5, None, &[("1", 0)]));
            b.type_block(TypeBlock::class(
                4,
                "CResourceFile",
                3,
                5,
                None,
                &[("rf_crsShared", 2)],
            ));
            b.type_block(TypeBlock::class(5, "CEntityParams", 1, 5, None, &[]));
            b.type_block(TypeBlock::class(
                6,
                "SBaseWeaponParams",
                1,
                5,
                Some(5),
                &[("bwp_pmcModel", 1)],
            ));
            b.external_object(5, model, 1, 100);
            b.object(0, 4, ObjectBytes::new().dynamic_container(&[1]).finish());
            b.object(1, 6, ObjectBytes::new().object_ref(Some(5)).finish());
            b.build().unwrap()
        };
        let rendering = {
            let mut b = MetaBuilder::new();
            b.type_block(TypeBlock::primitive(0, "FLOAT", 4, 4));
            b.type_block(TypeBlock::class(
                1,
                "Vector3f",
                1,
                5,
                None,
                &[(".1", 0), (".2", 0), (".3", 0)],
            ));
            b.type_block(TypeBlock::short(2, "CDynamicContainer<CResource>", 0, 8, 3));
            b.type_block(TypeBlock::class(3, "CResource", 4, 5, None, &[]));
            b.type_block(TypeBlock::class(
                4,
                "CResourceFile",
                3,
                5,
                None,
                &[("rf_crsShared", 2)],
            ));
            b.type_block(TypeBlock::class(5, "CEntityParams", 1, 5, None, &[]));
            b.type_block(TypeBlock::class(
                6,
                "SBaseWeaponRenderingParams",
                1,
                5,
                Some(5),
                &[
                    (".1", 1),
                    (".2", 1),
                    (".3", 0),
                    (".5", 0),
                    ("bwrp_vPositionDoubleWeapon", 1),
                    ("bwrp_vAngleDoubleWeapon", 1),
                    ("bwrp_fFOVDoubleWeapon", 0),
                    (".4", 0),
                ],
            ));
            b.object(0, 4, ObjectBytes::new().dynamic_container(&[1]).finish());
            let position = ObjectBytes::new().f32(0.2).f32(-0.3).f32(-0.5).finish();
            let angle = ObjectBytes::new().f32(6.0).f32(-4.0).f32(0.0).finish();
            let double_pos = ObjectBytes::new().f32(0.0).f32(0.0).f32(0.0).finish();
            let double_angle = ObjectBytes::new().f32(0.0).f32(0.0).f32(0.0).finish();
            let mut members = ObjectBytes::new();
            members = members.raw(&position).raw(&angle).f32(0.8262).f32(1.0);
            members = members.raw(&double_pos).raw(&double_angle).f32(0.8262);
            members = members.f32(0.1);
            b.object(1, 6, members.finish());
            b.build().unwrap()
        };
        (params, rendering)
    }

    #[test]
    fn the_weapon_s_view_numbers_are_read_by_position_not_by_name() {
        let (params, rendering) =
            weapon_ep_files("Content/SeriousSam2/Models/Weapons/Colt/Colt.mdl");
        let vfs = MemoryVfs::from_files([
            (
                "content/serioussam2/databases/entityparams/coltweapon.ep",
                params.as_slice(),
            ),
            (
                "content/serioussam2/databases/entityparams/coltweaponrendering.ep",
                rendering.as_slice(),
            ),
        ])
        .unwrap();
        let (model, view) = read_weapon_view(&vfs, 12).unwrap();
        assert_eq!(model, "Content/SeriousSam2/Models/Weapons/Colt/Colt.mdl");
        assert!((view.position.x - 0.2).abs() < 1e-5);
        assert!((view.position.y - -0.3).abs() < 1e-5);
        assert!((view.position.z - -0.5).abs() < 1e-5);
        assert_eq!(view.heading_pitch_deg, (6.0, -4.0));
        assert!((view.fov_x_at_4_3 - 0.8262).abs() < 1e-5);
        assert!((view.near - 0.1).abs() < 1e-5);
        // A weapon with no files at all does not read.
        assert!(read_weapon_view(&vfs, 14).is_err());
    }

    /// Data-gated: [`DEFAULT_WEAPON`]'s own `.mdl` (its first-person model, found through its `.ep` files)
    /// loads an animated build with an idle animation among its animations (not necessarily the one chosen as
    /// [`ss2_world::scene::SkinnedModelData::default_player`], but present, as every weapon's own build must
    /// have for a believable view-model). Skips without `SS2_DATA_DIR`.
    #[test]
    fn the_default_weapon_s_model_loads_with_an_idle_animation() {
        let dir = ss2_testkit::require_game_data!();
        let vfs = ss2_vfs::mount_install(&dir).unwrap();
        let (model_path, _view) = read_weapon_view(&vfs, DEFAULT_WEAPON).unwrap();
        let path = VfsPath::new(&model_path).unwrap();
        let built = load_standalone_model(&vfs, &path, &LoadOptions::default()).unwrap();
        let skinned = built
            .skinned
            .unwrap_or_else(|| panic!("{model_path}: no animated build ({:?})", built.skipped));
        assert!(
            skinned
                .animations
                .iter()
                .any(|a| a.name.to_ascii_lowercase().contains("idle")),
            "{model_path}: no idle animation among {:?}",
            skinned
                .animations
                .iter()
                .map(|a| &a.name)
                .collect::<Vec<_>>()
        );
    }

    /// Data-gated: loading and posing [`DEFAULT_WEAPON`]'s view-model the way [`crate::app`]'s `redraw` does
    /// every frame — [`load_weapon_viewmodel`] then [`pose`] — gives a non-empty, finite [`ViewModel`] that the
    /// renderer accepts (`Renderer::set_viewmodel`). Skips without `SS2_DATA_DIR` or a GPU adapter.
    #[test]
    fn the_app_builds_a_viewmodel_for_the_default_weapon() {
        let dir = ss2_testkit::require_game_data!();
        let vfs = ss2_vfs::mount_install(&dir).unwrap();
        let mut renderer = match Renderer::new_offscreen((160, 90)) {
            Ok(r) => r,
            Err(ss2_render::RenderError::NoAdapter(reason)) => {
                eprintln!("SKIPPED (no GPU adapter): {reason}");
                return;
            }
            Err(e) => panic!("{e}"),
        };
        let upload = load_weapon_viewmodel(&mut renderer, &vfs, DEFAULT_WEAPON)
            .expect("the default weapon's view-model loads from the real install");
        let vm = pose(&upload, 0.0);
        assert!(!vm.draws.is_empty(), "at least one draw per model part");
        assert!(vm.fov_x_at_4_3.is_finite() && vm.fov_x_at_4_3 > 0.0);
        assert!(vm.near.is_finite() && vm.near > 0.0);
        renderer
            .set_viewmodel(Some(vm))
            .expect("the renderer accepts the posed view-model");
        // Posing again later (a later frame) still works and still draws something.
        let vm_later = pose(&upload, 1.5);
        assert!(!vm_later.draws.is_empty());
    }
}
