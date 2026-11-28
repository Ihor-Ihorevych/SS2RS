//! GPU tests of the skinned path of the world pass (T04-006): a procedural two-bone bar bent by its palette, the bind
//! pose against the static path, and refused palettes. Every picture is made in code. They need a GPU adapter (a
//! software one is fine) and skip with a message when there is none; each takes `gpu_test_guard`.

use std::f32::consts::FRAC_PI_2;

use ss2_core::math::{Mat4, Quat, Vec3};
use ss2_render_api::Camera;
use ss2_render_api::scene_data::{MeshData, MeshVertex};
use ss2_render_api::skinning::{
    BoneTransform, SkeletonBone, SkeletonData, SkinnedMeshData, SkinnedVertex,
};
use ss2_ui::Color;

use crate::gpu::gpu_test_guard;
use crate::{ImageRgba8, RenderError, Renderer, SkinnedDraw, SkinnedMeshHandle, WorldBatch};

const SKY: Color = Color([0.2, 0.4, 0.6, 1.0]);
const SKY_PX: [u8; 4] = [51, 102, 153, 255];
const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const RED_PX: [u8; 4] = [255, 0, 0, 255];
const SIZE: (u32, u32) = (128, 128);
/// How far in front of the camera the bar stands.
const DEPTH: f32 = -3.0;

fn offscreen() -> Option<Renderer> {
    match Renderer::new_offscreen(SIZE) {
        Ok(r) => Some(r),
        Err(RenderError::NoAdapter(reason)) => {
            eprintln!("SKIPPED (no GPU adapter): {reason}");
            None
        }
        Err(e) => panic!("renderer creation failed: {e}"),
    }
}

/// The pixel of `img` that the world point `p` projects to with the default camera.
fn at(img: &ImageRgba8, p: Vec3) -> [u8; 4] {
    let clip =
        Camera::default().view_projection(img.width as f32 / img.height as f32) * p.extend(1.0);
    let ndc = clip.truncate() / clip.w;
    let x = ((ndc.x + 1.0) / 2.0 * img.width as f32) as u32;
    let y = ((1.0 - ndc.y) / 2.0 * img.height as f32) as u32;
    let i = ((y * img.width + x) * 4) as usize;
    img.pixels[i..i + 4].try_into().unwrap()
}

fn close(a: [u8; 4], b: [u8; 4]) -> bool {
    a.iter().zip(b).all(|(x, y)| x.abs_diff(y) <= 2)
}

/// Two bones: a root at y = -1 and a child (the joint) at the origin; inverse binds are the rest pose's inverses.
fn skeleton() -> SkeletonData {
    SkeletonData::new(vec![
        SkeletonBone::at_rest(
            1,
            None,
            BoneTransform::new(Quat::IDENTITY, -Vec3::Y),
            Mat4::from_translation(Vec3::Y),
        ),
        SkeletonBone::at_rest(
            2,
            Some(1),
            BoneTransform::new(Quat::IDENTITY, Vec3::Y),
            Mat4::IDENTITY,
        ),
    ])
    .unwrap()
}

/// A bar 0.4 wide facing +Z: the lower square (y -1..0) on the root, the upper one (y 0..1) on the child.
fn bar() -> (Vec<SkinnedVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for (bone, y0) in [(0u16, -1.0f32), (1, 0.0)] {
        let base = u32::try_from(vertices.len()).unwrap();
        for (x, y) in [(-0.2, y0), (0.2, y0), (0.2, y0 + 1.0), (-0.2, y0 + 1.0)] {
            vertices.push(SkinnedVertex {
                position: [x, y, 0.0],
                normal: [0.0, 0.0, 1.0],
                uv: [0.5, 0.5],
                joints: [bone, 0, 0, 0],
                weights: [1.0, 0.0, 0.0, 0.0],
            });
        }
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (vertices, indices)
}

fn skinned_bar(s: &SkeletonData) -> SkinnedMeshData {
    let (vertices, indices) = bar();
    SkinnedMeshData {
        vertices,
        indices,
        inverse_bind: s.inverse_binds(),
    }
}

fn draw(mesh: SkinnedMeshHandle, palette: Vec<Mat4>) -> SkinnedDraw {
    SkinnedDraw {
        mesh,
        texture: None,
        tint: RED,
        alpha_cutoff: None,
        transform: Mat4::from_translation(Vec3::new(0.0, 0.0, DEPTH)),
        palette,
    }
}

fn point(x: f32, y: f32) -> Vec3 {
    Vec3::new(x, y, DEPTH)
}

#[test]
fn a_two_bone_bar_bends_where_its_palette_says() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = offscreen() else {
        return;
    };
    let s = skeleton();
    let mesh = r.create_skinned_mesh(&skinned_bar(&s)).unwrap();
    let rest = s.model_transforms(&s.rest_pose()).unwrap();
    r.set_skinned(vec![draw(mesh, rest)]).unwrap();
    assert_eq!(r.skinned_count(), 1);
    let img = r
        .render_world_to_image(&Camera::default(), None, SKY)
        .unwrap();
    assert!(close(at(&img, point(0.0, -0.6)), RED_PX), "the lower half");
    assert!(
        close(at(&img, point(0.0, 0.6)), RED_PX),
        "the upper half, upright"
    );
    assert_eq!(at(&img, point(-0.6, 0.1)), SKY_PX);

    // Bend the child 90 degrees about +Z: the upper half now points along -X; the lower half stays.
    let bent = s.pose_from_tracks([], [(2, Quat::from_rotation_z(FRAC_PI_2).to_array())]);
    r.set_bone_palette(0, &s.model_transforms(&bent).unwrap())
        .unwrap();
    let img = r
        .render_world_to_image(&Camera::default(), None, SKY)
        .unwrap();
    assert!(
        close(at(&img, point(0.0, -0.6)), RED_PX),
        "the lower half stays"
    );
    assert_eq!(at(&img, point(0.0, 0.6)), SKY_PX, "the upper half has left");
    assert!(
        close(at(&img, point(-0.6, 0.1)), RED_PX),
        "{:?}",
        at(&img, point(-0.6, 0.1))
    );

    // Moving the instance moves everything.
    r.set_skinned_transform(0, Mat4::from_translation(Vec3::new(1.0, 0.0, DEPTH)))
        .unwrap();
    let img = r
        .render_world_to_image(&Camera::default(), None, SKY)
        .unwrap();
    assert!(close(at(&img, point(1.0, -0.6)), RED_PX));
    assert_eq!(at(&img, point(0.0, -0.6)), SKY_PX);
}

#[test]
fn the_bind_pose_matches_the_static_path() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = offscreen() else {
        return;
    };
    let (vertices, indices) = bar();
    let static_mesh = r
        .create_mesh(&MeshData {
            vertices: vertices
                .iter()
                .map(|v| MeshVertex {
                    position: v.position,
                    normal: v.normal,
                    uv: v.uv,
                })
                .collect(),
            indices,
            lightmap_uvs: None,
        })
        .unwrap();
    let transform =
        Mat4::from_rotation_translation(Quat::from_rotation_y(0.4), Vec3::new(0.2, 0.1, DEPTH));
    r.set_world(vec![WorldBatch {
        mesh: static_mesh,
        texture: None,
        tint: RED,
        alpha_cutoff: None,
        instances: vec![transform],
        lightmap: None,
    }])
    .unwrap();
    let static_img = r
        .render_world_to_image(&Camera::default(), None, SKY)
        .unwrap();
    r.set_world(Vec::new()).unwrap();

    let s = skeleton();
    let mesh = r.create_skinned_mesh(&skinned_bar(&s)).unwrap();
    let rest = s.model_transforms(&s.rest_pose()).unwrap();
    r.set_skinned(vec![SkinnedDraw {
        transform,
        ..draw(mesh, rest)
    }])
    .unwrap();
    let skinned_img = r
        .render_world_to_image(&Camera::default(), None, SKY)
        .unwrap();
    let differing = static_img
        .pixels
        .chunks(4)
        .zip(skinned_img.pixels.chunks(4))
        .filter(|(a, b)| a.iter().zip(*b).any(|(x, y)| x.abs_diff(*y) > 1))
        .count();
    assert_eq!(differing, 0, "pixels that differ from the static path");
    assert!(
        static_img
            .pixels
            .chunks(4)
            .any(|p| p[0] > 200 && p[1] < 10 && p[2] < 10),
        "the bar is drawn at all (turned, so lit a little less than fully)"
    );
}

#[test]
fn bad_palettes_and_stale_handles_are_refused() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = offscreen() else {
        return;
    };
    let s = skeleton();
    let mesh = r.create_skinned_mesh(&skinned_bar(&s)).unwrap();
    let rest = s.model_transforms(&s.rest_pose()).unwrap();
    assert!(
        r.set_skinned(vec![draw(mesh, rest[..1].to_vec())]).is_err(),
        "short"
    );
    assert!(
        r.set_skinned(vec![draw(mesh, vec![Mat4::IDENTITY; 3])])
            .is_err(),
        "long"
    );
    let mut nan = rest.clone();
    nan[1] = Mat4::from_cols_array(&[f32::NAN; 16]);
    assert!(r.set_skinned(vec![draw(mesh, nan.clone())]).is_err(), "NaN");
    assert_eq!(r.skinned_count(), 0, "nothing changed");

    r.set_skinned(vec![draw(mesh, rest.clone())]).unwrap();
    assert!(r.set_bone_palette(0, &rest[..1]).is_err());
    assert!(r.set_bone_palette(0, &nan).is_err());
    assert!(r.set_bone_palette(1, &rest).is_err(), "no instance 1");
    assert!(
        r.set_skinned_transform(0, Mat4::from_cols_array(&[f32::INFINITY; 16]))
            .is_err()
    );
    r.set_bone_palette(0, &rest).unwrap();

    let mut bad = skinned_bar(&s);
    bad.vertices[0].joints[0] = 5;
    assert!(
        r.create_skinned_mesh(&bad).is_err(),
        "a joint outside the bones"
    );
    let empty = SkinnedMeshData {
        indices: Vec::new(),
        ..skinned_bar(&s)
    };
    assert!(r.create_skinned_mesh(&empty).is_err(), "no triangles");

    r.clear_world();
    assert_eq!(r.skinned_count(), 0);
    assert!(
        r.set_bone_palette(0, &rest).is_err(),
        "dropped with the world"
    );
    assert!(
        r.set_skinned(vec![draw(mesh, rest)]).is_err(),
        "a stale mesh"
    );
}
