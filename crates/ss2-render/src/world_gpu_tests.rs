//! GPU tests of the 3D world pass: depth order, the overlay on top, textures, handles, device loss and a golden
//! image of the procedural test room. Every picture is made in code (no game content). They need a GPU adapter (a
//! software one is fine) and skip with a message when there is none; each takes `gpu_test_guard`.

use std::path::PathBuf;

use ss2_core::math::{Mat4, Quat, Vec3};
use ss2_render_api::Camera;
use ss2_render_api::scene_data::{MeshData, MeshVertex, TexelFormat, TextureData};
use ss2_ui::{Color, DrawList, Quad, Rect};
use ss2_world::scene::LevelScene;

use crate::golden::{self, Mode, Tolerance};
use crate::gpu::gpu_test_guard;
use crate::{ImageRgba8, RenderError, Renderer, TextureHandle, WorldBatch};

const SKY: Color = Color([0.2, 0.4, 0.6, 1.0]);
const SKY_PX: [u8; 4] = [51, 102, 153, 255];

fn offscreen(size: (u32, u32)) -> Option<Renderer> {
    match Renderer::new_offscreen(size) {
        Ok(r) => Some(r),
        Err(RenderError::NoAdapter(reason)) => {
            eprintln!("SKIPPED (no GPU adapter): {reason}");
            None
        }
        Err(e) => panic!("renderer creation failed: {e}"),
    }
}

fn px(img: &ImageRgba8, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * img.width + x) * 4) as usize;
    img.pixels[i..i + 4].try_into().unwrap()
}

fn close(a: [u8; 4], b: [u8; 4], tolerance: u8) -> bool {
    a.iter().zip(b).all(|(x, y)| x.abs_diff(y) <= tolerance)
}

/// A square facing +Z, 2 m wide, centred on the origin.
fn square() -> MeshData {
    let v = |x: f32, y: f32| MeshVertex {
        position: [x, y, 0.0],
        normal: [0.0, 0.0, 1.0],
        uv: [(x + 1.0) / 2.0, (1.0 - y) / 2.0],
    };
    MeshData {
        vertices: vec![v(-1.0, -1.0), v(1.0, -1.0), v(1.0, 1.0), v(-1.0, 1.0)],
        indices: vec![0, 1, 2, 0, 2, 3],
        lightmap_uvs: None,
    }
}

fn at(z: f32) -> Mat4 {
    Mat4::from_translation(Vec3::new(0.0, 0.0, z))
}

fn batch(mesh: crate::MeshHandle, tint: [f32; 4], z: f32) -> WorldBatch {
    WorldBatch {
        mesh,
        texture: None,
        tint,
        alpha_cutoff: None,
        instances: vec![at(z)],
        lightmap: None,
    }
}

const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];

/// The camera at the origin looking down -Z. The light comes from the camera, so a square facing it is lit at
/// 0.55 + 0.45 = 100 % and shows its colour exactly.
fn camera() -> Camera {
    Camera::default()
}

#[test]
fn the_nearer_surface_wins_whatever_the_draw_order() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = offscreen((64, 64)) else {
        return;
    };
    let mesh = r.create_mesh(&square()).unwrap();
    for order in [[(RED, -3.0), (GREEN, -6.0)], [(GREEN, -6.0), (RED, -3.0)]] {
        r.set_world(order.iter().map(|(c, z)| batch(mesh, *c, *z)).collect())
            .unwrap();
        let img = r.render_world_to_image(&camera(), None, SKY).unwrap();
        assert!(
            close(px(&img, 32, 32), [255, 0, 0, 255], 2),
            "{:?}",
            px(&img, 32, 32)
        );
        assert_eq!(px(&img, 0, 0), SKY_PX, "the clear colour around the square");
    }
}

#[test]
fn the_overlay_is_drawn_over_the_world() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = offscreen((1920, 1080)) else {
        return;
    };
    let mesh = r.create_mesh(&square()).unwrap();
    r.set_world(vec![batch(mesh, RED, -3.0)]).unwrap();
    let mut ui = DrawList::default();
    ui.quads.push(Quad {
        rect: Rect {
            x: 950.0,
            y: 530.0,
            w: 20.0,
            h: 20.0,
        },
        color: Color([0.0, 0.0, 1.0, 1.0]),
    });
    let img = r.render_world_to_image(&camera(), Some(&ui), SKY).unwrap();
    assert_eq!(px(&img, 960, 540), [0, 0, 255, 255]);
    assert!(close(px(&img, 900, 540), [255, 0, 0, 255], 2));
}

#[test]
fn textures_are_sampled_and_tinted_and_cut_out() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = offscreen((64, 64)) else {
        return;
    };
    let mesh = r.create_mesh(&square()).unwrap();
    // Left half opaque yellow, right half transparent; two texels each, so linear filtering inside a half
    // blends equal texels.
    let mut texels = Vec::new();
    for _y in 0..4 {
        texels.extend([255, 255, 0, 255, 255, 255, 0, 255, 0, 0, 0, 0, 0, 0, 0, 0]);
    }
    let tex = r
        .create_texture(&TextureData {
            width: 4,
            height: 4,
            format: TexelFormat::Rgba8,
            mips: vec![texels],
            repeat_u: false,
            repeat_v: false,
        })
        .unwrap();
    let textured = |tint: [f32; 4], cutoff: Option<f32>, texture: TextureHandle| WorldBatch {
        texture: Some(texture),
        alpha_cutoff: cutoff,
        ..batch(mesh, tint, -3.0)
    };
    r.set_world(vec![textured([1.0, 0.5, 1.0, 1.0], Some(0.5), tex)])
        .unwrap();
    let img = r.render_world_to_image(&camera(), None, SKY).unwrap();
    assert!(
        close(px(&img, 24, 32), [255, 128, 0, 255], 3),
        "{:?}",
        px(&img, 24, 32)
    );
    assert_eq!(px(&img, 40, 32), SKY_PX, "cut out");
}

#[test]
fn bc1_textures_upload_when_the_adapter_supports_them() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = offscreen((64, 64)) else {
        return;
    };
    let red_block = {
        // Both end colours pure red in RGB565, every index 0.
        let c = 0xF800u16.to_le_bytes();
        vec![c[0], c[1], c[0], c[1], 0, 0, 0, 0]
    };
    let data = TextureData {
        width: 4,
        height: 4,
        format: TexelFormat::Bc1,
        mips: vec![red_block.clone(), red_block.clone(), red_block],
        repeat_u: true,
        repeat_v: true,
    };
    if !r.supports(TexelFormat::Bc1) {
        assert!(r.create_texture(&data).is_err());
        eprintln!("SKIPPED (no BC texture support on this adapter)");
        return;
    }
    let tex = r.create_texture(&data).unwrap();
    let mesh = r.create_mesh(&square()).unwrap();
    r.set_world(vec![WorldBatch {
        texture: Some(tex),
        ..batch(mesh, [1.0; 4], -3.0)
    }])
    .unwrap();
    let img = r.render_world_to_image(&camera(), None, SKY).unwrap();
    assert!(
        close(px(&img, 32, 32), [255, 0, 0, 255], 3),
        "{:?}",
        px(&img, 32, 32)
    );
}

#[test]
fn bad_data_and_stale_handles_are_errors() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = offscreen((32, 32)) else {
        return;
    };
    let mut broken = square();
    broken.indices.push(7);
    assert!(r.create_mesh(&broken).is_err());
    let mut bad_tex = TextureData::solid([1, 2, 3, 4]);
    bad_tex.mips[0].pop();
    assert!(r.create_texture(&bad_tex).is_err());

    let mesh = r.create_mesh(&square()).unwrap();
    let tex = r
        .create_texture(&TextureData::solid([9, 9, 9, 255]))
        .unwrap();
    r.set_world(vec![batch(mesh, RED, -3.0)]).unwrap();
    let mut bad = batch(mesh, RED, -3.0);
    bad.instances[0] = Mat4::from_cols_array(&[f32::NAN; 16]);
    assert!(r.set_world(vec![bad]).is_err(), "non-finite matrix");
    r.clear_world();
    assert!(
        r.set_world(vec![batch(mesh, RED, -3.0)]).is_err(),
        "mesh from before clear_world"
    );
    let fresh = r.create_mesh(&square()).unwrap();
    let stale_texture = WorldBatch {
        texture: Some(tex),
        ..batch(fresh, RED, -3.0)
    };
    assert!(
        r.set_world(vec![stale_texture]).is_err(),
        "texture from before clear_world"
    );
    // An empty world draws only the clear colour.
    r.set_world(Vec::new()).unwrap();
    let img = r.render_world_to_image(&camera(), None, SKY).unwrap();
    assert_eq!(px(&img, 16, 16), SKY_PX);
}

#[test]
fn a_device_rebuild_drops_the_world_and_changes_the_generation() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = offscreen((32, 32)) else {
        return;
    };
    let mesh = r.create_mesh(&square()).unwrap();
    r.set_world(vec![batch(mesh, RED, -3.0)]).unwrap();
    let generation = r.world_generation();
    r.destroy_device_for_tests();
    let img = r
        .render_world_to_image(&camera(), None, SKY)
        .expect("the frame rebuilds the device and draws");
    assert_eq!(r.stats().device_rebuilds, 1);
    assert_ne!(r.world_generation(), generation);
    assert_eq!(
        px(&img, 16, 16),
        SKY_PX,
        "the world is gone until it is uploaded again"
    );
    assert!(
        r.set_world(vec![batch(mesh, RED, -3.0)]).is_err(),
        "old handles are stale"
    );
    let mesh = r.create_mesh(&square()).unwrap();
    r.set_world(vec![batch(mesh, RED, -3.0)]).unwrap();
    let img = r.render_world_to_image(&camera(), None, SKY).unwrap();
    assert!(close(px(&img, 16, 16), [255, 0, 0, 255], 2));
}

/// Upload a whole [`LevelScene`] the simple way: one mesh per model part, one batch per part with every instance
/// of its model.
fn upload(r: &mut Renderer, scene: &LevelScene) -> Vec<WorldBatch> {
    let textures: Vec<TextureHandle> = scene
        .textures
        .iter()
        .map(|t| r.create_texture(t).unwrap())
        .collect();
    let mut batches = Vec::new();
    for (m, model) in scene.models.iter().enumerate() {
        let instances: Vec<Mat4> = scene
            .instances
            .iter()
            .filter(|i| i.model as usize == m)
            .map(|i| i.transform.matrix())
            .collect();
        for part in &model.parts {
            let material = &scene.materials[part.material as usize];
            batches.push(WorldBatch {
                mesh: r.create_mesh(&part.mesh).unwrap(),
                texture: material.base_texture.map(|t| textures[t as usize]),
                tint: material.base_color,
                alpha_cutoff: None,
                instances: instances.clone(),
                lightmap: None,
            });
        }
    }
    batches
}

#[test]
fn the_test_room_matches_its_golden_image() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = offscreen((640, 360)) else {
        return;
    };
    let room = LevelScene::test_room();
    let batches = upload(&mut r, &room);
    r.set_world(batches).unwrap();
    let start = room.start();
    // From the spawn, a little above eye height, looking slightly down towards the steps.
    let cam = Camera {
        position: start.position + Vec3::new(0.0, 2.2, 0.0),
        rotation: Quat::from_rotation_y(start.yaw) * Quat::from_rotation_x(-0.25),
        ..Camera::default()
    };
    let img = r.render_world_to_image(&cam, None, SKY).unwrap();
    // Without the golden: the floor fills the bottom row, the sky does not show at the bottom.
    assert_ne!(px(&img, 320, 355), SKY_PX);
    let sky_pixels = img.pixels.chunks(4).filter(|p| *p == SKY_PX).count();
    assert!(
        sky_pixels < 640 * 360 / 2,
        "the room fills most of the view: {sky_pixels}"
    );

    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/golden");
    let failures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/golden-failures");
    let mode = Mode::from_env(failures);
    if let Err(e) = golden::check_golden(
        &dir,
        "world_test_room.png",
        &img,
        &Tolerance::default(),
        &mode,
    ) {
        panic!("{e}");
    }
}

// ---- the sky ----------------------------------------------------------------------------------------------------------

use crate::SkyBatch;
use ss2_core::math::{Mat4 as M4, Vec4};
use ss2_render_api::Projection;
use ss2_render_api::scene_data::CubeFace;
use ss2_world::scene::{test_sky_colour, test_sky_face};

/// Upload the test sky's six faces and set them as the renderer's sky.
fn test_sky(r: &mut Renderer) {
    let faces = CubeFace::ALL.map(|f| Some(r.create_texture(&test_sky_face(f)).unwrap()));
    r.set_sky(Some(SkyBatch {
        faces,
        tint: [1.0; 4],
        rotation: Quat::IDENTITY,
        size: 64.0,
        viewer_height: 0.0,
    }))
    .unwrap();
}

/// A camera at `position` looking at `face` with its up along the face image's top.
fn facing(face: CubeFace, position: Vec3, fov_degrees: f32) -> Camera {
    let b = face.basis();
    let up = -b.down;
    let m = M4::from_cols(
        Vec4::from((b.right, 0.0)),
        Vec4::from((up, 0.0)),
        Vec4::from((-b.forward, 0.0)),
        Vec4::W,
    );
    Camera {
        position,
        rotation: Quat::from_mat4(&m),
        projection: Projection::Perspective {
            fov_y: fov_degrees.to_radians(),
            near: 0.1,
        },
    }
}

/// The pixel of a square image of `size` that shows the direction `forward + a right + b down` for a camera made by
/// [`facing`] with a field of view of `fov_degrees`.
fn pixel_towards(img: &ImageRgba8, a: f32, b: f32, fov_degrees: f32) -> [u8; 4] {
    let t = (fov_degrees.to_radians() / 2.0).tan();
    let (ndc_x, ndc_y) = (a / t, -b / t);
    let x = ((ndc_x + 1.0) / 2.0 * img.width as f32) as u32;
    let y = ((1.0 - ndc_y) / 2.0 * img.height as f32) as u32;
    px(img, x.min(img.width - 1), y.min(img.height - 1))
}

#[test]
fn every_sky_face_is_where_the_convention_puts_it_upright_and_not_mirrored() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = offscreen((128, 128)) else {
        return;
    };
    test_sky(&mut r);
    let fov = 100.0;
    for face in CubeFace::ALL {
        let img = r
            .render_world_to_image(&facing(face, Vec3::ZERO, fov), None, SKY)
            .unwrap();
        let at = |a, b| pixel_towards(&img, a, b, fov);
        assert!(
            close(at(0.0, 0.0), test_sky_colour(face), 3),
            "{face:?} centre {:?}",
            at(0.0, 0.0)
        );
        assert!(
            close(at(0.0, -0.75), [255; 4], 3),
            "{face:?} top band {:?}",
            at(0.0, -0.75)
        );
        assert!(
            close(at(-0.8, -0.8), [0, 0, 0, 255], 3),
            "{face:?} top-left {:?}",
            at(-0.8, -0.8)
        );
        assert!(
            close(at(0.8, -0.8), [255; 4], 3),
            "{face:?} top-right (mirrored?) {:?}",
            at(0.8, -0.8)
        );
    }
}

#[test]
fn the_sky_is_behind_the_world_and_never_comes_closer() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = offscreen((64, 64)) else {
        return;
    };
    let mesh = r.create_mesh(&square()).unwrap();
    r.set_world(vec![batch(mesh, RED, -3.0)]).unwrap();
    test_sky(&mut r);
    let img = r.render_world_to_image(&camera(), None, SKY).unwrap();
    assert!(
        close(px(&img, 32, 32), [255, 0, 0, 255], 2),
        "the world in front"
    );
    let left = px(&img, 1, 32);
    assert!(
        close(left, test_sky_colour(CubeFace::NegZ), 3),
        "the sky around it: {left:?}"
    );
    // Far away, the sky looks the same: it moves with the camera.
    let far = Camera {
        position: Vec3::new(500.0, 80.0, -300.0),
        ..camera()
    };
    r.set_world(Vec::new()).unwrap();
    let img = r.render_world_to_image(&far, None, SKY).unwrap();
    assert!(close(px(&img, 1, 32), left, 2));
}

#[test]
fn a_face_without_a_texture_shows_the_tint_and_bad_skies_are_refused() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = offscreen((32, 32)) else {
        return;
    };
    r.set_sky(Some(SkyBatch {
        faces: [None; 6],
        tint: [0.2, 0.4, 0.6, 1.0],
        rotation: Quat::IDENTITY,
        size: 64.0,
        viewer_height: 0.0,
    }))
    .unwrap();
    let img = r
        .render_world_to_image(&camera(), None, Color([0.0, 0.0, 0.0, 1.0]))
        .unwrap();
    assert!(
        close(px(&img, 16, 16), [51, 102, 153, 255], 2),
        "{:?}",
        px(&img, 16, 16)
    );
    assert!(
        r.set_sky(Some(SkyBatch {
            faces: [None; 6],
            tint: [f32::NAN, 0.0, 0.0, 1.0],
            rotation: Quat::IDENTITY,
            size: 64.0,
            viewer_height: 0.0,
        }))
        .is_err()
    );
    let tex = r
        .create_texture(&TextureData::solid([1, 2, 3, 255]))
        .unwrap();
    r.clear_world();
    let img = r.render_world_to_image(&camera(), None, SKY).unwrap();
    assert_eq!(px(&img, 16, 16), SKY_PX, "clear_world drops the sky");
    assert!(
        r.set_sky(Some(SkyBatch {
            faces: [Some(tex), None, None, None, None, None],
            tint: [1.0; 4],
            rotation: Quat::IDENTITY,
            size: 64.0,
            viewer_height: 0.0,
        }))
        .is_err(),
        "a texture from before clear_world"
    );
    r.set_sky(None).unwrap();
}

#[test]
fn the_test_room_under_its_sky_matches_its_golden_image() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = offscreen((640, 360)) else {
        return;
    };
    let room = LevelScene::test_room();
    let batches = upload(&mut r, &room);
    r.set_world(batches).unwrap();
    test_sky(&mut r);
    let start = room.start();
    // From the spawn, looking up over the back wall and a little to the left, so three sky faces show.
    let cam = Camera {
        position: start.position + Vec3::new(0.0, 1.65, 0.0),
        rotation: Quat::from_rotation_y(0.5) * Quat::from_rotation_x(0.45),
        ..Camera::default()
    };
    let img = r.render_world_to_image(&cam, None, SKY).unwrap();
    assert!(
        !img.pixels.chunks(4).any(|p| p == SKY_PX),
        "the sky covers everything the room does not"
    );
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/golden");
    let failures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/golden-failures");
    let mode = Mode::from_env(failures);
    if let Err(e) = golden::check_golden(
        &dir,
        "world_test_room_sky.png",
        &img,
        &Tolerance::default(),
        &mode,
    ) {
        panic!("{e}");
    }
}

// ---- lightmaps --------------------------------------------------------------------------------------------------------

use ss2_world::scene::TEST_LIGHTMAP_TEXTURE;

/// The test room's floor alone, drawn with its lightmap, seen from 12 m straight above (the floor fills the view).
fn lit_floor(r: &mut Renderer) -> ImageRgba8 {
    let room = LevelScene::test_room();
    let floor = &room.instances[0];
    let part = &room.models[floor.model as usize].parts[0];
    let base = r
        .create_texture(
            &room.textures[room.materials[part.material as usize].base_texture.unwrap() as usize],
        )
        .unwrap();
    let lightmap = r
        .create_texture(&room.textures[TEST_LIGHTMAP_TEXTURE as usize])
        .unwrap();
    let mesh = r.create_mesh(&part.mesh).unwrap();
    r.set_world(vec![WorldBatch {
        mesh,
        texture: Some(base),
        tint: [1.0; 4],
        alpha_cutoff: None,
        instances: vec![floor.transform.matrix()],
        lightmap: Some(lightmap),
    }])
    .unwrap();
    let above = Camera {
        position: Vec3::new(0.0, 12.0, 0.0),
        rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),
        ..Camera::default()
    };
    r.render_world_to_image(&above, None, SKY).unwrap()
}

#[test]
fn a_lightmapped_floor_is_full_bright_with_its_shadow_in_the_middle() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = offscreen((128, 128)) else {
        return;
    };
    let img = lit_floor(&mut r);
    // From this high the floor's checker is minified to its average grey; the lightmap's shadow (60) and light (200)
    // multiply it with no light term, so the middle is 60/200 = 0.3 of the edge.
    let centre = px(&img, 64, 64);
    let edge = px(&img, 3, 64);
    let ratio = f32::from(centre[0]) / f32::from(edge[0]);
    assert!(
        (ratio - 0.3).abs() < 0.04,
        "shadow/light ratio {ratio}: {centre:?} vs {edge:?}"
    );
    assert!(
        edge[0] > 100,
        "the lit edge is bright (full bright, not the flat light): {edge:?}"
    );
    assert_eq!(centre[0], centre[1], "grey stays grey");
}

#[test]
fn the_lightmapped_floor_matches_its_golden_image() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = offscreen((256, 256)) else {
        return;
    };
    let img = lit_floor(&mut r);
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testdata/golden");
    let failures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/golden-failures");
    let mode = Mode::from_env(failures);
    if let Err(e) = golden::check_golden(
        &dir,
        "world_lightmapped_floor.png",
        &img,
        &Tolerance::default(),
        &mode,
    ) {
        panic!("{e}");
    }
}

/// The test sky placed with `rotation` and `viewer_height` (size 64).
fn placed_test_sky(r: &mut Renderer, rotation: Quat, viewer_height: f32) {
    let faces = CubeFace::ALL.map(|f| Some(r.create_texture(&test_sky_face(f)).unwrap()));
    r.set_sky(Some(SkyBatch {
        faces,
        tint: [1.0; 4],
        rotation,
        size: 64.0,
        viewer_height,
    }))
    .unwrap();
}

#[test]
fn a_turned_sky_keeps_its_turn_in_the_world() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = offscreen((64, 64)) else {
        return;
    };
    // Turned a quarter to the left (+90 degrees around +Y), the cube's +X face now lies towards -Z.
    placed_test_sky(
        &mut r,
        Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
        0.0,
    );
    let img = r.render_world_to_image(&camera(), None, SKY).unwrap();
    let centre = px(&img, 32, 40);
    assert!(
        close(centre, test_sky_colour(CubeFace::PosX), 3),
        "looking down -Z: {centre:?}"
    );
    // Not turned: the -Z face.
    placed_test_sky(&mut r, Quat::IDENTITY, 0.0);
    let img = r.render_world_to_image(&camera(), None, SKY).unwrap();
    assert!(close(px(&img, 32, 40), test_sky_colour(CubeFace::NegZ), 3));
    // A placement that is not a rotation is refused.
    let bad = SkyBatch {
        faces: [None; 6],
        tint: [1.0; 4],
        rotation: Quat::from_xyzw(0.0, 0.0, 0.0, 2.0),
        size: 64.0,
        viewer_height: 0.0,
    };
    assert!(r.set_sky(Some(bad)).is_err());
}

#[test]
fn a_viewer_high_in_the_cube_sees_the_ceiling_sooner() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = offscreen((64, 64)) else {
        return;
    };
    // Looking 20 degrees up from facing -Z: from the centre the ray meets the -Z face; from 80 % of the way to the
    // ceiling (viewer height 0.8 x half-extent 64 = 51.2) it meets the ceiling first.
    let up = Camera {
        rotation: Quat::from_rotation_x(20f32.to_radians()),
        ..camera()
    };
    placed_test_sky(&mut r, Quat::IDENTITY, 0.0);
    let img = r.render_world_to_image(&up, None, SKY).unwrap();
    assert!(
        close(px(&img, 32, 32), test_sky_colour(CubeFace::NegZ), 3),
        "{:?}",
        px(&img, 32, 32)
    );
    placed_test_sky(&mut r, Quat::IDENTITY, 51.2);
    let img = r.render_world_to_image(&up, None, SKY).unwrap();
    assert!(
        close(px(&img, 32, 32), test_sky_colour(CubeFace::PosY), 3),
        "{:?}",
        px(&img, 32, 32)
    );
}

#[test]
fn fog_greys_out_the_far_surface_and_leaves_the_near_one_and_the_sky_alone() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = offscreen((64, 64)) else {
        return;
    };
    let mesh = r.create_mesh(&square()).unwrap();
    let blue = ss2_render_api::scene_data::Fog {
        colour: [0.0, 0.0, 1.0],
        near: 10.0,
        far: 20.0,
        strength: 1.0,
    };
    // A red square 3 m away is before the fog starts.
    r.set_world(vec![batch(mesh, RED, -3.0)]).unwrap();
    r.set_fog(Some(blue)).unwrap();
    let img = r.render_world_to_image(&camera(), None, SKY).unwrap();
    assert!(
        close(px(&img, 32, 32), [255, 0, 0, 255], 2),
        "{:?}",
        px(&img, 32, 32)
    );
    // 30 m away (scaled to fill the same view) it is past the far distance: all fog.
    let mut far = batch(mesh, RED, -30.0);
    far.instances = vec![Mat4::from_scale_rotation_translation(
        Vec3::splat(10.0),
        Quat::IDENTITY,
        Vec3::new(0.0, 0.0, -30.0),
    )];
    r.set_world(vec![far]).unwrap();
    let img = r.render_world_to_image(&camera(), None, SKY).unwrap();
    assert!(
        close(px(&img, 32, 32), [0, 0, 255, 255], 2),
        "{:?}",
        px(&img, 32, 32)
    );
    // Half strength: half way (the target is sRGB, so compare in linear terms loosely: neither pure colour).
    r.set_fog(Some(ss2_render_api::scene_data::Fog {
        strength: 0.5,
        ..blue
    }))
    .unwrap();
    let mid = px(
        &r.render_world_to_image(&camera(), None, SKY).unwrap(),
        32,
        32,
    );
    assert!(mid[0] > 100 && mid[2] > 100, "{mid:?}");
    // No fog: the far square shows its own colour again.
    r.set_fog(None).unwrap();
    let img = r.render_world_to_image(&camera(), None, SKY).unwrap();
    assert!(
        close(px(&img, 32, 32), [255, 0, 0, 255], 2),
        "{:?}",
        px(&img, 32, 32)
    );
    // The sky is never fogged.
    r.clear_world();
    r.set_fog(Some(blue)).unwrap();
    test_sky(&mut r);
    let fogged = r.render_world_to_image(&camera(), None, SKY).unwrap();
    r.set_fog(None).unwrap();
    let clear = r.render_world_to_image(&camera(), None, SKY).unwrap();
    assert_eq!(fogged.pixels, clear.pixels);
    // A bad fog is refused and the old one kept.
    assert!(
        r.set_fog(Some(ss2_render_api::scene_data::Fog { far: 1.0, ..blue }))
            .is_err()
    );
}

#[test]
fn a_missing_floor_is_left_open_under_a_ceiling_and_tinted_without_one() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = offscreen((32, 32)) else {
        return;
    };
    let tint = [0.2, 0.4, 0.6, 1.0];
    let ceiling = r
        .create_texture(&TextureData::solid([255, 255, 255, 255]))
        .unwrap();
    let down = facing(CubeFace::NegY, Vec3::ZERO, 60.0);
    let black = Color([0.0, 0.0, 0.0, 1.0]);
    // Floor missing, ceiling present: looking straight down shows the clear colour (an open bottom).
    let mut faces = [None; 6];
    faces[CubeFace::PosY.index()] = Some(ceiling);
    r.set_sky(Some(SkyBatch {
        faces,
        tint,
        rotation: Quat::IDENTITY,
        size: 64.0,
        viewer_height: 0.0,
    }))
    .unwrap();
    let img = r.render_world_to_image(&down, None, black).unwrap();
    assert_eq!(px(&img, 16, 16), [0, 0, 0, 255], "open bottom");
    // No ceiling either: the floor is drawn in the tint like every other face.
    r.set_sky(Some(SkyBatch {
        faces: [None; 6],
        tint,
        rotation: Quat::IDENTITY,
        size: 64.0,
        viewer_height: 0.0,
    }))
    .unwrap();
    let img = r.render_world_to_image(&down, None, black).unwrap();
    assert!(
        close(px(&img, 16, 16), [51, 102, 153, 255], 2),
        "{:?}",
        px(&img, 16, 16)
    );
}
