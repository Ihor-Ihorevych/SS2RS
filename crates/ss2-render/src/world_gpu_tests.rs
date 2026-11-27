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
