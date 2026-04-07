use chrono::Local;
use std::f32::consts::TAU;

use bytemuck::{Pod, Zeroable};
use glam::{Affine3A, Quat, Vec3};
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use vzglyd_slide::params_buf;
#[cfg(target_arch = "wasm32")]
use vzglyd_slide::{trace_scope, trace_scope_with_attrs};
use vzglyd_slide::{
    CameraKeyframe, CameraPath, DrawSource, DrawSpec, FilterMode, Limits, PipelineKind, SceneSpace,
    ShaderSources, SlideSpec, StaticMesh, TextureDesc, TextureFormat, WrapMode,
};

// Default midnight-blue sky background colour (RGB, no alpha).
static mut BG_COLOR: [f32; 3] = [0.03, 0.07, 0.14];

#[cfg(target_arch = "wasm32")]
params_buf!(256);

#[cfg(target_arch = "wasm32")]
#[derive(serde::Deserialize)]
struct ClockParams {
    background: Option<[f32; 3]>,
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn vzglyd_configure(len: i32) -> i32 {
    let bytes = len.max(0);
    let bytes_str = bytes.to_string();
    let mut trace = trace_scope_with_attrs("vzglyd_configure", &[("bytes", bytes_str.as_str())]);
    if len <= 0 {
        trace.set_status("ok");
        return 0;
    }
    let bytes = unsafe { &VZGLYD_PARAMS_BUF[..len as usize] };
    if let Ok(params) = serde_json::from_slice::<ClockParams>(bytes) {
        if let Some(bg) = params.background {
            if bg.iter().all(|value| value.is_finite()) {
                unsafe {
                    BG_COLOR = bg;
                    SPEC_BYTES = None;
                }
            }
        }
    }
    trace.set_status("ok");
    0
}

const WIRE_VERSION: u8 = 1;
const CLOCK_SEGMENTS: u8 = 64;

const CAMERA_EYE_Z: f32 = 5.0;
const CAMERA_FOV_Y_DEG: f32 = 45.0;

const MODE_BODY: f32 = 0.0;
const MODE_SKY: f32 = 1.0;
const MODE_HALO: f32 = 2.0;
const MODE_GLASS: f32 = 3.0;
const MODE_HOUR_HAND: f32 = 4.0;
const MODE_MINUTE_HAND: f32 = 5.0;
const MODE_SECOND_HAND: f32 = 6.0;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, Serialize, Deserialize)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
    pub mode: f32,
}

fn build_noise_texture() -> Vec<u8> {
    let width = 96usize;
    let height = 96usize;
    let mut data = vec![0u8; width * height * 4];
    for y in 0..height {
        for x in 0..width {
            let seed = ((x as u32) << 16) ^ y as u32 ^ 0xA3C5_19D7;
            let grain = (hash01(seed) * 255.0) as u8;
            let pulse = (hash01(seed ^ 0x59D2_F41B) * 255.0) as u8;
            let offset = (y * width + x) * 4;
            data[offset] = grain;
            data[offset + 1] = pulse;
            data[offset + 2] = grain.saturating_add(18);
            data[offset + 3] = 255;
        }
    }
    data
}

fn clock_slide_spec() -> SlideSpec<Vertex> {
    let sky_mesh = build_sky_mesh();
    let body_mesh = build_clock_body_mesh();
    let effect_mesh = build_clock_effect_mesh();

    SlideSpec {
        name: "clock_world".into(),
        limits: Limits::pi4(),
        scene_space: SceneSpace::World3D,
        camera_path: Some(clock_camera_path()),
        shaders: Some(ShaderSources {
            vertex_wgsl: None,
            fragment_wgsl: Some(include_str!("clock_shader.wgsl").to_string()),
        }),
        overlay: None,
        font: None,
        textures_used: 2,
        textures: vec![
            TextureDesc {
                label: "blank".into(),
                width: 1,
                height: 1,
                format: TextureFormat::Rgba8Unorm,
                wrap_u: WrapMode::ClampToEdge,
                wrap_v: WrapMode::ClampToEdge,
                wrap_w: WrapMode::ClampToEdge,
                mag_filter: FilterMode::Nearest,
                min_filter: FilterMode::Nearest,
                mip_filter: FilterMode::Nearest,
                data: vec![255, 255, 255, 255],
            },
            TextureDesc {
                label: "noise_tex".into(),
                width: 96,
                height: 96,
                format: TextureFormat::Rgba8Unorm,
                wrap_u: WrapMode::Repeat,
                wrap_v: WrapMode::Repeat,
                wrap_w: WrapMode::Repeat,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                mip_filter: FilterMode::Nearest,
                data: build_noise_texture(),
            },
        ],
        static_meshes: vec![sky_mesh.clone(), body_mesh.clone(), effect_mesh.clone()],
        dynamic_meshes: vec![],
        draws: vec![
            DrawSpec {
                label: "clock_sky".into(),
                source: DrawSource::Static(0),
                pipeline: PipelineKind::Opaque,
                index_range: 0..sky_mesh.indices.len() as u32,
            },
            DrawSpec {
                label: "clock_body".into(),
                source: DrawSource::Static(1),
                pipeline: PipelineKind::Opaque,
                index_range: 0..body_mesh.indices.len() as u32,
            },
            DrawSpec {
                label: "clock_effects".into(),
                source: DrawSource::Static(2),
                pipeline: PipelineKind::Transparent,
                index_range: 0..effect_mesh.indices.len() as u32,
            },
        ],
        lighting: Some(vzglyd_slide::WorldLighting::new(
            [0.90, 0.94, 1.0],
            0.32,
            Some(vzglyd_slide::DirectionalLight::new(
                [0.18, 0.95, 0.24],
                [1.0, 0.96, 0.90],
                1.0,
            )),
        )),
    }
}

fn clock_camera_path() -> CameraPath {
    CameraPath {
        looped: true,
        keyframes: vec![
            CameraKeyframe {
                time: 0.0,
                position: [0.0, 0.0, CAMERA_EYE_Z],
                target: [0.0, 0.0, 0.0],
                up: [0.0, 1.0, 0.0],
                fov_y_deg: CAMERA_FOV_Y_DEG,
            },
            CameraKeyframe {
                time: 12.0,
                position: [0.0, 0.0, CAMERA_EYE_Z],
                target: [0.0, 0.0, 0.0],
                up: [0.0, 1.0, 0.0],
                fov_y_deg: CAMERA_FOV_Y_DEG,
            },
        ],
    }
}

fn build_sky_mesh() -> StaticMesh<Vertex> {
    let [r, g, b] = unsafe { BG_COLOR };
    let sky_color = [r, g, b, 1.0];
    StaticMesh {
        label: "clock_sky".into(),
        vertices: vec![
            Vertex {
                position: [-1.0, -1.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                color: sky_color,
                mode: MODE_SKY,
            },
            Vertex {
                position: [1.0, -1.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                color: sky_color,
                mode: MODE_SKY,
            },
            Vertex {
                position: [1.0, 1.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                color: sky_color,
                mode: MODE_SKY,
            },
            Vertex {
                position: [-1.0, 1.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                color: sky_color,
                mode: MODE_SKY,
            },
        ],
        indices: vec![0, 1, 2, 0, 2, 3],
    }
}

fn build_clock_body_mesh() -> StaticMesh<Vertex> {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let bezel = [0.82, 0.87, 0.94, 1.0];
    let inner_rim = [0.30, 0.70, 0.98, 1.0];
    let face = [0.10, 0.15, 0.22, 1.0];
    let back = [0.07, 0.11, 0.17, 1.0];
    let marker = [0.94, 0.97, 1.0, 1.0];
    let hour_hand = [0.96, 0.98, 1.0, 1.0];
    let minute_hand = [0.90, 0.96, 1.0, 1.0];
    let second_hand = [1.0, 0.44, 0.34, 1.0];
    let center_cap = [0.32, 0.76, 1.0, 1.0];

    push_annulus_prism(
        &mut vertices,
        &mut indices,
        Affine3A::IDENTITY,
        1.48,
        1.74,
        -0.22,
        0.22,
        bezel,
        MODE_BODY,
    );
    push_annulus_prism(
        &mut vertices,
        &mut indices,
        Affine3A::IDENTITY,
        1.30,
        1.38,
        0.04,
        0.08,
        inner_rim,
        MODE_BODY,
    );
    push_disc(
        &mut vertices,
        &mut indices,
        Affine3A::IDENTITY,
        1.40,
        0.06,
        face,
        MODE_BODY,
        true,
    );
    push_disc(
        &mut vertices,
        &mut indices,
        Affine3A::IDENTITY,
        1.48,
        -0.18,
        back,
        MODE_BODY,
        false,
    );
    push_box_transformed(
        &mut vertices,
        &mut indices,
        Affine3A::from_translation(Vec3::new(1.87, 0.0, 0.0)),
        Vec3::new(-0.08, -0.18, -0.11),
        Vec3::new(0.14, 0.18, 0.11),
        bezel,
        MODE_BODY,
    );

    for marker_index in 0..12 {
        let is_major = marker_index % 3 == 0;
        let phi = marker_index as f32 / 12.0 * TAU;
        let rotation = Quat::from_rotation_z(-phi);
        let transform = Affine3A::from_rotation_translation(rotation, Vec3::new(0.0, 0.0, 0.11));
        let radius = if is_major { 1.05 } else { 1.08 };
        let half_width = if is_major { 0.06 } else { 0.03 };
        let half_depth = if is_major { 0.04 } else { 0.03 };
        let height = if is_major { 0.30 } else { 0.18 };
        push_box_transformed(
            &mut vertices,
            &mut indices,
            transform,
            Vec3::new(-half_width, radius, -half_depth),
            Vec3::new(half_width, radius + height, half_depth),
            marker,
            MODE_BODY,
        );
    }

    push_clock_hand(
        &mut vertices,
        &mut indices,
        0.78,
        0.085,
        0.16,
        0.060,
        0.12,
        hour_hand,
        MODE_HOUR_HAND,
    );
    push_clock_hand(
        &mut vertices,
        &mut indices,
        1.12,
        0.055,
        0.20,
        0.048,
        0.15,
        minute_hand,
        MODE_MINUTE_HAND,
    );
    push_clock_hand(
        &mut vertices,
        &mut indices,
        1.24,
        0.018,
        0.28,
        0.022,
        0.18,
        second_hand,
        MODE_SECOND_HAND,
    );

    push_annulus_prism(
        &mut vertices,
        &mut indices,
        Affine3A::IDENTITY,
        0.05,
        0.13,
        0.11,
        0.19,
        center_cap,
        MODE_BODY,
    );

    StaticMesh {
        label: "analog_clock_body".into(),
        vertices,
        indices,
    }
}

fn build_clock_effect_mesh() -> StaticMesh<Vertex> {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    push_effect_disc(
        &mut vertices,
        &mut indices,
        Affine3A::IDENTITY,
        2.34,
        -0.12,
        [0.20, 0.58, 1.0, 0.55],
        MODE_HALO,
    );
    push_effect_disc(
        &mut vertices,
        &mut indices,
        Affine3A::IDENTITY,
        1.96,
        -0.02,
        [0.12, 0.38, 0.78, 0.24],
        MODE_HALO,
    );
    push_effect_disc(
        &mut vertices,
        &mut indices,
        Affine3A::IDENTITY,
        1.43,
        0.24,
        [1.0, 1.0, 1.0, 0.20],
        MODE_GLASS,
    );

    StaticMesh {
        label: "analog_clock_effects".into(),
        vertices,
        indices,
    }
}

fn push_clock_hand(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u16>,
    length: f32,
    half_width: f32,
    tail: f32,
    half_depth: f32,
    z: f32,
    color: [f32; 4],
    mode: f32,
) {
    let transform = Affine3A::from_translation(Vec3::new(0.0, 0.0, z));
    push_box_transformed(
        vertices,
        indices,
        transform,
        Vec3::new(-half_width, -tail, -half_depth),
        Vec3::new(half_width, length, half_depth),
        color,
        mode,
    );
}

fn push_disc(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u16>,
    model: Affine3A,
    radius: f32,
    z: f32,
    color: [f32; 4],
    mode: f32,
    front_facing: bool,
) {
    let center = model.transform_point3(Vec3::new(0.0, 0.0, z));
    let normal = model
        .transform_vector3(if front_facing { Vec3::Z } else { -Vec3::Z })
        .normalize_or_zero();
    let center_index = push_vertex(vertices, center, normal, color, mode);

    for segment in 0..CLOCK_SEGMENTS {
        let t0 = segment as f32 / CLOCK_SEGMENTS as f32 * TAU;
        let t1 = (segment + 1) as f32 / CLOCK_SEGMENTS as f32 * TAU;
        let p0 = model.transform_point3(circle_point(radius, t0, z));
        let p1 = model.transform_point3(circle_point(radius, t1, z));
        let i0 = push_vertex(vertices, p0, normal, color, mode);
        let i1 = push_vertex(vertices, p1, normal, color, mode);
        if front_facing {
            indices.extend_from_slice(&[center_index, i0, i1]);
        } else {
            indices.extend_from_slice(&[center_index, i1, i0]);
        }
    }
}

fn push_annulus_prism(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u16>,
    model: Affine3A,
    inner_radius: f32,
    outer_radius: f32,
    back_z: f32,
    front_z: f32,
    color: [f32; 4],
    mode: f32,
) {
    for segment in 0..CLOCK_SEGMENTS {
        let t0 = segment as f32 / CLOCK_SEGMENTS as f32 * TAU;
        let t1 = (segment + 1) as f32 / CLOCK_SEGMENTS as f32 * TAU;

        let outer0_back = model.transform_point3(circle_point(outer_radius, t0, back_z));
        let outer0_front = model.transform_point3(circle_point(outer_radius, t0, front_z));
        let outer1_back = model.transform_point3(circle_point(outer_radius, t1, back_z));
        let outer1_front = model.transform_point3(circle_point(outer_radius, t1, front_z));

        let inner0_back = model.transform_point3(circle_point(inner_radius, t0, back_z));
        let inner0_front = model.transform_point3(circle_point(inner_radius, t0, front_z));
        let inner1_back = model.transform_point3(circle_point(inner_radius, t1, back_z));
        let inner1_front = model.transform_point3(circle_point(inner_radius, t1, front_z));

        let outer_normal = model
            .transform_vector3(radial_direction(t0, t1))
            .normalize_or_zero();
        let inner_normal = -outer_normal;
        let front_normal = model.transform_vector3(Vec3::Z).normalize_or_zero();
        let back_normal = model.transform_vector3(-Vec3::Z).normalize_or_zero();

        push_quad(
            vertices,
            indices,
            [outer0_back, outer1_back, outer1_front, outer0_front],
            outer_normal,
            color,
            mode,
        );
        push_quad(
            vertices,
            indices,
            [inner0_back, inner0_front, inner1_front, inner1_back],
            inner_normal,
            color,
            mode,
        );
        push_quad(
            vertices,
            indices,
            [inner0_front, outer0_front, outer1_front, inner1_front],
            front_normal,
            color,
            mode,
        );
        push_quad(
            vertices,
            indices,
            [inner0_back, inner1_back, outer1_back, outer0_back],
            back_normal,
            color,
            mode,
        );
    }
}

fn push_effect_disc(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u16>,
    model: Affine3A,
    radius: f32,
    z: f32,
    color: [f32; 4],
    mode: f32,
) {
    let center = model.transform_point3(Vec3::new(0.0, 0.0, z));
    let center_index = push_effect_vertex(vertices, center, [0.0, 0.0, 1.0], color, mode);

    for segment in 0..CLOCK_SEGMENTS {
        let t0 = segment as f32 / CLOCK_SEGMENTS as f32 * TAU;
        let t1 = (segment + 1) as f32 / CLOCK_SEGMENTS as f32 * TAU;
        let local0 = circle_point(radius, t0, z);
        let local1 = circle_point(radius, t1, z);
        let p0 = model.transform_point3(local0);
        let p1 = model.transform_point3(local1);
        let uv0 = [local0.x / radius, local0.y / radius, 1.0];
        let uv1 = [local1.x / radius, local1.y / radius, 1.0];
        let i0 = push_effect_vertex(vertices, p0, uv0, color, mode);
        let i1 = push_effect_vertex(vertices, p1, uv1, color, mode);
        indices.extend_from_slice(&[center_index, i0, i1]);
    }
}

fn push_box_transformed(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u16>,
    transform: Affine3A,
    min: Vec3,
    max: Vec3,
    color: [f32; 4],
    mode: f32,
) {
    let corners = [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::new(min.x, max.y, max.z),
    ]
    .map(|corner| transform.transform_point3(corner));

    let nx = transform.transform_vector3(Vec3::X).normalize_or_zero();
    let ny = transform.transform_vector3(Vec3::Y).normalize_or_zero();
    let nz = transform.transform_vector3(Vec3::Z).normalize_or_zero();

    push_quad(
        vertices,
        indices,
        [corners[4], corners[5], corners[6], corners[7]],
        nz,
        color,
        mode,
    );
    push_quad(
        vertices,
        indices,
        [corners[1], corners[0], corners[3], corners[2]],
        -nz,
        color,
        mode,
    );
    push_quad(
        vertices,
        indices,
        [corners[0], corners[4], corners[7], corners[3]],
        -nx,
        color,
        mode,
    );
    push_quad(
        vertices,
        indices,
        [corners[5], corners[1], corners[2], corners[6]],
        nx,
        color,
        mode,
    );
    push_quad(
        vertices,
        indices,
        [corners[3], corners[7], corners[6], corners[2]],
        ny,
        color,
        mode,
    );
    push_quad(
        vertices,
        indices,
        [corners[0], corners[1], corners[5], corners[4]],
        -ny,
        color,
        mode,
    );
}

fn push_quad(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u16>,
    points: [Vec3; 4],
    normal: Vec3,
    color: [f32; 4],
    mode: f32,
) {
    let base = vertices.len() as u16;
    for point in points {
        vertices.push(Vertex {
            position: point.to_array(),
            normal: normal.to_array(),
            color,
            mode,
        });
    }
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn push_vertex(
    vertices: &mut Vec<Vertex>,
    position: Vec3,
    normal: Vec3,
    color: [f32; 4],
    mode: f32,
) -> u16 {
    let index = vertices.len() as u16;
    vertices.push(Vertex {
        position: position.to_array(),
        normal: normal.to_array(),
        color,
        mode,
    });
    index
}

fn push_effect_vertex(
    vertices: &mut Vec<Vertex>,
    position: Vec3,
    encoded_normal: [f32; 3],
    color: [f32; 4],
    mode: f32,
) -> u16 {
    let index = vertices.len() as u16;
    vertices.push(Vertex {
        position: position.to_array(),
        normal: encoded_normal,
        color,
        mode,
    });
    index
}

fn circle_point(radius: f32, theta: f32, z: f32) -> Vec3 {
    Vec3::new(theta.cos() * radius, theta.sin() * radius, z)
}

fn radial_direction(theta0: f32, theta1: f32) -> Vec3 {
    let theta = (theta0 + theta1) * 0.5;
    Vec3::new(theta.cos(), theta.sin(), 0.0)
}

fn hash01(seed: u32) -> f32 {
    let mut x = seed;
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^= x >> 16;
    x as f32 / u32::MAX as f32
}

static mut SPEC_BYTES: Option<Vec<u8>> = None;

fn build_spec_bytes() -> Vec<u8> {
    let mut bytes = vec![WIRE_VERSION];
    bytes.extend(postcard::to_stdvec(&clock_slide_spec()).expect("serialize clock slide spec"));
    bytes
}

fn get_spec_bytes() -> &'static [u8] {
    unsafe {
        let slot = std::ptr::addr_of_mut!(SPEC_BYTES);
        if (*slot).is_none() {
            *slot = Some(build_spec_bytes());
        }
        (*slot).as_deref().unwrap()
    }
}

pub fn serialized_spec() -> &'static [u8] {
    get_spec_bytes()
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn vzglyd_spec_ptr() -> *const u8 {
    get_spec_bytes().as_ptr()
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn vzglyd_spec_len() -> u32 {
    get_spec_bytes().len() as u32
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn vzglyd_abi_version() -> u32 {
    1
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn vzglyd_init() -> i32 {
    let mut trace = trace_scope("vzglyd_init");
    trace.set_status("ok");
    0
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
pub extern "C" fn vzglyd_update(_dt: f32) -> i32 {
    let dt_ms = format!("{:.3}", _dt * 1000.0);
    let mut trace = trace_scope_with_attrs("vzglyd_update", &[("dt_ms", dt_ms.as_str())]);
    trace.set_status("ok");
    0
}
