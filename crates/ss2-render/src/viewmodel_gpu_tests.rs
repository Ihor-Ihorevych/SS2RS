//! GPU tests of the viewmodel pass (T04-030, `crate::viewmodel`): a viewmodel draw lands fully on top of the
//! world even where it would be behind a near wall under a shared depth test; the world's own depth test is
//! unaffected by the viewmodel pass; and the pass really uses its own field of view, not the world's. Every
//! picture is made in code (no game content). They need a GPU adapter (a software one is fine) and skip with
//! a message when there is none; each takes `gpu_test_guard`.

use ss2_core::math::{Mat4, Quat, Vec3};
use ss2_render_api::scene_data::{MeshData, MeshVertex};
use ss2_render_api::skinning::{
    BoneTransform, SkeletonBone, SkeletonData, SkinnedMeshData, SkinnedVertex,
};
use ss2_render_api::{Aabb, Camera};
use ss2_ui::Color;

use crate::gpu::gpu_test_guard;
use crate::{
    ImageRgba8, RenderError, Renderer, SkinnedDraw, SkinnedMeshHandle, ViewModel, WorldBatch,
};

const SKY: Color = Color([0.2, 0.4, 0.6, 1.0]);
const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const RED_PX: [u8; 4] = [255, 0, 0, 255];
const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
const BLUE_PX: [u8; 4] = [0, 0, 255, 255];
const SIZE: (u32, u32) = (128, 128);

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

fn px(img: &ImageRgba8, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * img.width + x) * 4) as usize;
    img.pixels[i..i + 4].try_into().unwrap()
}

fn close(a: [u8; 4], b: [u8; 4]) -> bool {
    a.iter().zip(b).all(|(x, y)| x.abs_diff(y) <= 2)
}

/// The pixel a world point `p` projects to under the default world camera.
fn at(img: &ImageRgba8, p: Vec3) -> [u8; 4] {
    pixel_of(
        img,
        Camera::default().view_projection(img.width as f32 / img.height as f32),
        p,
    )
}

/// The pixel a camera-space point `p` projects to under `vm`'s own camera, at `img`'s aspect ratio.
fn viewmodel_at(img: &ImageRgba8, vm: &ViewModel, p: Vec3) -> [u8; 4] {
    let aspect = img.width as f32 / img.height as f32;
    let camera = vm.camera(aspect).expect("a valid viewmodel camera");
    pixel_of(img, camera.view_projection(aspect), p)
}

fn pixel_of(img: &ImageRgba8, view_projection: Mat4, p: Vec3) -> [u8; 4] {
    let clip = view_projection * p.extend(1.0);
    let ndc = clip.truncate() / clip.w;
    let x = (((ndc.x + 1.0) / 2.0 * img.width as f32) as u32).min(img.width - 1);
    let y = (((1.0 - ndc.y) / 2.0 * img.height as f32) as u32).min(img.height - 1);
    px(img, x, y)
}

/// A one-bone skeleton at the identity pose: enough to drive a skinned viewmodel draw.
fn one_bone() -> SkeletonData {
    SkeletonData::new(vec![SkeletonBone::at_rest(
        1,
        None,
        BoneTransform::new(Quat::IDENTITY, Vec3::ZERO),
        Mat4::IDENTITY,
    )])
    .unwrap()
}

/// A flat square of `half_extent` (in its own, model space) facing +Z (towards a camera looking down -Z),
/// bound entirely to `skeleton`'s one bone.
fn quad(skeleton: &SkeletonData, half_extent: f32) -> SkinnedMeshData {
    let h = half_extent;
    let v = |x: f32, y: f32| SkinnedVertex {
        position: [x, y, 0.0],
        normal: [0.0, 0.0, 1.0],
        uv: [0.5, 0.5],
        joints: [0, 0, 0, 0],
        weights: [1.0, 0.0, 0.0, 0.0],
    };
    SkinnedMeshData {
        vertices: vec![v(-h, -h), v(h, -h), v(h, h), v(-h, h)],
        indices: vec![0, 1, 2, 0, 2, 3],
        inverse_bind: skeleton.inverse_binds(),
        lightmap_uvs: None,
    }
}

/// A square world mesh facing +Z, `half_extent` wide.
fn world_quad(half_extent: f32) -> MeshData {
    let h = half_extent;
    let v = |x: f32, y: f32| MeshVertex {
        position: [x, y, 0.0],
        normal: [0.0, 0.0, 1.0],
        uv: [0.5, 0.5],
    };
    MeshData {
        vertices: vec![v(-h, -h), v(h, -h), v(h, h), v(-h, h)],
        indices: vec![0, 1, 2, 0, 2, 3],
        lightmap_uvs: None,
    }
}

/// One viewmodel draw: `mesh`, posed by `skeleton`'s rest pose, placed at `transform`.
fn viewmodel_draw(
    mesh: SkinnedMeshHandle,
    skeleton: &SkeletonData,
    transform: Mat4,
) -> SkinnedDraw {
    SkinnedDraw {
        mesh,
        texture: None,
        tint: RED,
        alpha_cutoff: None,
        transform,
        palette: skeleton.model_transforms(&skeleton.rest_pose()).unwrap(),
        bounds: Aabb::new(Vec3::splat(-1.0), Vec3::splat(1.0)),
        lightmap: None,
    }
}

#[test]
fn a_viewmodel_draw_lands_fully_on_top_even_behind_a_near_wall() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = offscreen() else {
        return;
    };

    // A wall just past the world's near plane (0.1), big enough to cover the whole screen.
    let wall_mesh = r.create_mesh(&world_quad(5.0)).unwrap();
    r.set_world(vec![WorldBatch {
        mesh: wall_mesh,
        texture: None,
        tint: BLUE,
        alpha_cutoff: None,
        instances: vec![Mat4::from_translation(Vec3::new(0.0, 0.0, -0.3))],
        lightmap: None,
    }])
    .unwrap();
    let before = r
        .render_world_to_image(&Camera::default(), None, SKY)
        .unwrap();
    assert!(
        close(at(&before, Vec3::new(0.0, 0.0, -0.3)), BLUE_PX),
        "the wall covers the centre"
    );
    assert!(close(px(&before, 2, 2), BLUE_PX), "and the corners");

    // A small viewmodel draw placed deep behind the wall: if the two passes shared one depth test, the wall
    // at -0.3 would occlude anything at -50.
    let skeleton = one_bone();
    let mesh = r.create_skinned_mesh(&quad(&skeleton, 0.3)).unwrap();
    let vm = ViewModel {
        draws: vec![viewmodel_draw(
            mesh,
            &skeleton,
            Mat4::from_translation(Vec3::new(0.0, 0.0, -50.0)),
        )],
        fov_x_at_4_3: 1.0,
        near: 0.1,
    };
    r.set_viewmodel(Some(vm.clone())).unwrap();
    let after = r
        .render_world_to_image(&Camera::default(), None, SKY)
        .unwrap();
    assert!(
        close(
            viewmodel_at(&after, &vm, Vec3::new(0.0, 0.0, -50.0)),
            RED_PX
        ),
        "the viewmodel draw is on top, not hidden behind the nearer wall"
    );
    assert!(
        close(px(&after, 2, 2), BLUE_PX),
        "the wall still shows where the viewmodel does not draw"
    );
}

#[test]
fn the_viewmodel_pass_does_not_disturb_the_worlds_own_depth_test() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = offscreen() else {
        return;
    };
    let mesh = r.create_mesh(&world_quad(1.0)).unwrap();
    let batch = |tint: [f32; 4], z: f32| WorldBatch {
        mesh,
        texture: None,
        tint,
        alpha_cutoff: None,
        instances: vec![Mat4::from_translation(Vec3::new(0.0, 0.0, z))],
        lightmap: None,
    };
    // The nearer, red quad must win the depth test at the centre, whichever order the two are drawn in.
    r.set_world(vec![batch(GREEN, -6.0), batch(RED, -3.0)])
        .unwrap();
    let before = r
        .render_world_to_image(&Camera::default(), None, SKY)
        .unwrap();
    assert!(
        close(at(&before, Vec3::new(0.0, 0.0, -3.0)), RED_PX),
        "red wins before any viewmodel is set"
    );

    // A viewmodel drawn off to one side of its own view, nowhere near the centre.
    let skeleton = one_bone();
    let vm_mesh = r.create_skinned_mesh(&quad(&skeleton, 0.2)).unwrap();
    let vm = ViewModel {
        draws: vec![viewmodel_draw(
            vm_mesh,
            &skeleton,
            Mat4::from_translation(Vec3::new(0.5, 0.5, -2.0)),
        )],
        fov_x_at_4_3: 0.8,
        near: 0.1,
    };
    r.set_viewmodel(Some(vm)).unwrap();
    let after = r
        .render_world_to_image(&Camera::default(), None, SKY)
        .unwrap();
    assert!(
        close(at(&after, Vec3::new(0.0, 0.0, -3.0)), RED_PX),
        "the world's own depth order at the centre is unaffected by the viewmodel pass"
    );
}

#[test]
fn the_viewmodel_pass_uses_its_own_field_of_view_not_the_worlds() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = offscreen() else {
        return;
    };
    // A viewmodel field of view at 4:3 wide enough that, once widened for this test's square aspect ratio
    // (`ViewModel::camera`), it is clearly wider than the world's 60 degree default.
    let fov_x_at_4_3 = 2.4f32;
    let vm_probe = ViewModel {
        draws: Vec::new(),
        fov_x_at_4_3,
        near: 0.1,
    };
    // tan(half the viewmodel's own horizontal fov at this image's aspect ratio).
    let proj = vm_probe.camera(1.0).unwrap().projection_matrix(1.0);
    let tan_half_viewmodel = 1.0 / proj.x_axis.x;
    let tan_half_world = (60f32.to_radians() * 0.5).tan();
    assert!(
        tan_half_viewmodel > tan_half_world * 1.5,
        "the viewmodel's own fov must be clearly wider than the world's: {tan_half_viewmodel} vs {tan_half_world}"
    );

    // A marker just inside the viewmodel's own view but outside the world's: if the pass drew with the
    // world's projection instead of its own, it would be clipped away.
    let marker = Vec3::new(0.9 * tan_half_viewmodel, 0.0, -1.0);
    let world_clip = Camera::default().view_projection(1.0) * marker.extend(1.0);
    assert!(
        world_clip.x.abs() > world_clip.w,
        "sanity: the marker is outside the world's own field of view"
    );

    let skeleton = one_bone();
    let mesh = r.create_skinned_mesh(&quad(&skeleton, 0.03)).unwrap();
    let vm = ViewModel {
        draws: vec![viewmodel_draw(
            mesh,
            &skeleton,
            Mat4::from_translation(marker),
        )],
        fov_x_at_4_3,
        near: 0.1,
    };
    r.set_viewmodel(Some(vm.clone())).unwrap();
    let img = r
        .render_world_to_image(&Camera::default(), None, SKY)
        .unwrap();
    assert!(
        close(viewmodel_at(&img, &vm, marker), RED_PX),
        "drawn at the pixel its own, wider field of view puts it at, not clipped away by the world's"
    );
}

#[test]
fn an_invalid_viewmodel_is_refused_and_nothing_changes() {
    let _gpu = gpu_test_guard();
    let Some(mut r) = offscreen() else {
        return;
    };
    let skeleton = one_bone();
    let mesh = r.create_skinned_mesh(&quad(&skeleton, 0.3)).unwrap();
    let good = viewmodel_draw(
        mesh,
        &skeleton,
        Mat4::from_translation(Vec3::new(0.0, 0.0, -2.0)),
    );

    assert!(
        r.set_viewmodel(Some(ViewModel {
            draws: vec![good.clone()],
            fov_x_at_4_3: f32::NAN,
            near: 0.1,
        }))
        .is_err(),
        "a NaN field of view"
    );
    assert!(
        r.set_viewmodel(Some(ViewModel {
            draws: vec![good.clone()],
            fov_x_at_4_3: 0.8,
            near: 0.0,
        }))
        .is_err(),
        "a zero near plane"
    );
    let mut short = good.clone();
    short.palette = Vec::new();
    assert!(
        r.set_viewmodel(Some(ViewModel {
            draws: vec![short],
            fov_x_at_4_3: 0.8,
            near: 0.1,
        }))
        .is_err(),
        "a palette of the wrong length"
    );

    // None of the refused attempts drew anything.
    let img = r
        .render_world_to_image(&Camera::default(), None, SKY)
        .unwrap();
    assert!(!img.pixels.chunks(4).any(|p| p == RED_PX));

    // A good one now draws.
    r.set_viewmodel(Some(ViewModel {
        draws: vec![good],
        fov_x_at_4_3: 0.8,
        near: 0.1,
    }))
    .unwrap();
    let img = r
        .render_world_to_image(&Camera::default(), None, SKY)
        .unwrap();
    assert!(img.pixels.chunks(4).any(|p| p == RED_PX));

    // And `None` clears it again.
    r.set_viewmodel(None).unwrap();
    let img = r
        .render_world_to_image(&Camera::default(), None, SKY)
        .unwrap();
    assert!(!img.pixels.chunks(4).any(|p| p == RED_PX));
}
