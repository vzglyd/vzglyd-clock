const CLOCK_MODE_BODY: i32 = 0;
const CLOCK_MODE_SKY: i32 = 1;
const CLOCK_MODE_HALO: i32 = 2;
const CLOCK_MODE_GLASS: i32 = 3;
const CLOCK_MODE_HOUR_HAND: i32 = 4;
const CLOCK_MODE_MINUTE_HAND: i32 = 5;
const CLOCK_MODE_SECOND_HAND: i32 = 6;

const CLOCK_CYCLE_SECS: f32 = 10.5;
const CLOCK_SAFE_RADIUS: f32 = 1.94;
const CAMERA_EYE_Z: f32 = 9.4;
const CAMERA_FOV_Y_DEG: f32 = 36.0;
const VIEW_ASPECT: f32 = 640.0 / 480.0;

fn clock_mode(mode: f32) -> i32 {
    return i32(floor(mode + 0.5));
}

fn rotation_x(angle: f32) -> mat3x3<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return mat3x3<f32>(
        vec3<f32>(1.0, 0.0, 0.0),
        vec3<f32>(0.0, c, s),
        vec3<f32>(0.0, -s, c),
    );
}

fn rotation_y(angle: f32) -> mat3x3<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return mat3x3<f32>(
        vec3<f32>(c, 0.0, -s),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(s, 0.0, c),
    );
}

fn rotation_z(angle: f32) -> mat3x3<f32> {
    let c = cos(angle);
    let s = sin(angle);
    return mat3x3<f32>(
        vec3<f32>(c, s, 0.0),
        vec3<f32>(-s, c, 0.0),
        vec3<f32>(0.0, 0.0, 1.0),
    );
}

fn clock_cycle_fade() -> f32 {
    let cycle_t = fract(u.time / CLOCK_CYCLE_SECS);
    let fade_in = smoothstep(0.0, 0.18, cycle_t);
    let fade_out = 1.0 - smoothstep(0.72, 1.0, cycle_t);
    return clamp(fade_in * fade_out, 0.0, 1.0);
}

fn clock_alpha_scale(mode_code: i32) -> f32 {
    let fade = clock_cycle_fade();
    if mode_code == CLOCK_MODE_HALO || mode_code == CLOCK_MODE_GLASS {
        return fade * 0.95;
    }
    if mode_code == CLOCK_MODE_SKY {
        return 1.0;
    }
    return fade;
}

fn hash01(seed: u32) -> f32 {
    var x = seed;
    x = x ^ (x >> 16u);
    x = x * 0x7FEB352Du;
    x = x ^ (x >> 15u);
    x = x * 0x846CA68Bu;
    x = x ^ (x >> 16u);
    return f32(x) / f32(0xFFFFFFFFu);
}

fn sample_range(seed: u32, min_value: f32, max_value: f32) -> f32 {
    return mix(min_value, max_value, hash01(seed));
}

fn min_visible_distance(x: f32, y: f32, radius: f32) -> f32 {
    let tan_half_y = tan(radians(CAMERA_FOV_Y_DEG) * 0.5);
    let distance_y = (abs(y) + radius) / tan_half_y;
    let distance_x = (abs(x) + radius) / (tan_half_y * VIEW_ASPECT);
    return max(max(distance_x, distance_y), 4.8);
}

fn clock_pose_translation() -> vec3<f32> {
    let cycle = u32(max(floor(u.time / CLOCK_CYCLE_SECS), 0.0));
    let cycle_t = fract(u.time / CLOCK_CYCLE_SECS);
    let motion_t = smoothstep(0.0, 1.0, cycle_t);

    let start_x = sample_range(cycle ^ 0x15A31C2Du, -5.2, 5.2);
    let start_y = sample_range(cycle ^ 0x77B09A11u, -3.8, 3.8);
    let end_x = sample_range(cycle ^ 0x229D04F1u, -1.7, 1.7) + start_x * 0.22;
    let end_y = sample_range(cycle ^ 0x59ABE120u, -1.2, 1.2) + start_y * 0.16;

    let end_distance_min = min_visible_distance(end_x, end_y, CLOCK_SAFE_RADIUS) + 0.35;
    let end_distance = max(sample_range(cycle ^ 0x9182771Bu, 5.6, 7.8), end_distance_min);
    let start_distance = end_distance + sample_range(cycle ^ 0x4109BD53u, 2.4, 5.1);

    let x = mix(start_x, end_x, motion_t);
    let y = mix(start_y, end_y, motion_t) + sin(u.time * 0.55 + f32(cycle) * 0.23) * 0.10;
    let distance = max(
        mix(start_distance, end_distance, motion_t),
        min_visible_distance(x, y, CLOCK_SAFE_RADIUS) + 0.25,
    );

    return vec3<f32>(x, y, CAMERA_EYE_Z - distance);
}

fn clock_pose_rotation() -> mat3x3<f32> {
    let cycle = u32(max(floor(u.time / CLOCK_CYCLE_SECS), 0.0));
    let yaw = sample_range(cycle ^ 0x0F135ACDu, -1.05, 1.05);
    let pitch = sample_range(cycle ^ 0xD31E33B9u, -0.30, 0.30);
    let roll = sample_range(cycle ^ 0xABCD1021u, -0.16, 0.16);
    return rotation_y(yaw) * rotation_x(pitch) * rotation_z(roll);
}

fn clock_hand_angle(mode_code: i32) -> f32 {
    let total_seconds = floor(u.clock_seconds);
    let total_minutes = floor(total_seconds / 60.0);
    let total_hours = floor(total_seconds / 3600.0);

    let seconds = total_seconds - 60.0 * floor(total_seconds / 60.0);
    let minutes = total_minutes - 60.0 * floor(total_minutes / 60.0);
    let hours = total_hours - 12.0 * floor(total_hours / 12.0);

    if mode_code == CLOCK_MODE_HOUR_HAND {
        return -6.28318530718 * (hours + minutes / 60.0 + seconds / 3600.0) / 12.0;
    }
    if mode_code == CLOCK_MODE_MINUTE_HAND {
        return -6.28318530718 * (minutes + seconds / 60.0) / 60.0;
    }
    return -6.28318530718 * seconds / 60.0;
}

fn clock_is_surface(mode_code: i32) -> bool {
    return mode_code == CLOCK_MODE_BODY
        || mode_code == CLOCK_MODE_HOUR_HAND
        || mode_code == CLOCK_MODE_MINUTE_HAND
        || mode_code == CLOCK_MODE_SECOND_HAND;
}

@vertex
fn vs_main(in: VzglydVertexInput) -> VzglydVertexOutput {
    var out: VzglydVertexOutput;
    let mode_code = clock_mode(in.mode);
    out.color = in.color;
    out.mode = f32(mode_code);

    if mode_code == CLOCK_MODE_SKY {
        out.world_pos = in.position;
        out.normal = in.normal;
        out.clip_pos = vec4<f32>(in.position.xy, 0.999, 1.0);
        return out;
    }

    var local_pos = in.position;
    var local_normal = in.normal;
    if mode_code == CLOCK_MODE_HOUR_HAND
        || mode_code == CLOCK_MODE_MINUTE_HAND
        || mode_code == CLOCK_MODE_SECOND_HAND
    {
        let hand_rotation = rotation_z(clock_hand_angle(mode_code));
        local_pos = hand_rotation * local_pos;
        local_normal = hand_rotation * local_normal;
    }

    let pose_rotation = clock_pose_rotation();
    let pose_translation = clock_pose_translation();
    let world_pos = pose_rotation * local_pos + pose_translation;

    out.world_pos = world_pos;
    out.color.a = in.color.a * clock_alpha_scale(mode_code);
    if clock_is_surface(mode_code) {
        out.normal = normalize(pose_rotation * local_normal);
    } else {
        out.normal = in.normal;
    }
    out.clip_pos = u.view_proj * vec4<f32>(world_pos, 1.0);
    return out;
}

fn fog_rgb(rgb: vec3<f32>, world_pos: vec3<f32>) -> vec3<f32> {
    let dist = length(world_pos - u.cam_pos);
    let t = clamp((dist - u.fog_start) / (u.fog_end - u.fog_start), 0.0, 1.0);
    return mix(rgb, u.fog_color.rgb, t * t * (3.0 - 2.0 * t));
}

fn bayer_threshold(clip_pos: vec4<f32>) -> f32 {
    var m = array<f32, 16>(
         0.0 / 16.0,  8.0 / 16.0,  2.0 / 16.0, 10.0 / 16.0,
        12.0 / 16.0,  4.0 / 16.0, 14.0 / 16.0,  6.0 / 16.0,
         3.0 / 16.0, 11.0 / 16.0,  1.0 / 16.0,  9.0 / 16.0,
        15.0 / 16.0,  7.0 / 16.0, 13.0 / 16.0,  5.0 / 16.0,
    );
    let px = u32(clip_pos.x) % 4u;
    let py = u32(clip_pos.y) % 4u;
    return m[py * 4u + px];
}

fn sky_color(clip_xy: vec2<f32>, bg: vec3<f32>) -> vec3<f32> {
    let uv = clip_xy * 0.5 + vec2<f32>(0.5, 0.5);
    let grad = clamp(0.18 + uv.y * 0.82, 0.0, 1.0);
    let base = mix(bg * 0.7, bg * 1.3, grad);
    let bloom = 1.0 - clamp(length((uv - vec2<f32>(0.5, 0.58)) * vec2<f32>(1.05, 0.82)), 0.0, 1.0);
    let waves = textureSample(t_noise, s_repeat, uv * 2.2 + vec2<f32>(u.time * 0.012, -u.time * 0.017)).r;
    return base + bg * 0.3 * bloom * bloom + vec3<f32>(waves * 0.015);
}

fn lit_clock_surface(base: vec4<f32>, normal: vec3<f32>, world_pos: vec3<f32>) -> vec3<f32> {
    let n = normalize(normal);
    let view_dir = normalize(u.cam_pos - world_pos);
    let light_dir = vzglyd_main_light_dir();
    let half_vec = normalize(light_dir + view_dir);
    let diff = max(dot(n, light_dir), 0.0);
    let spec = pow(max(dot(n, half_vec), 0.0), 42.0);
    let rim = pow(1.0 - max(dot(n, view_dir), 0.0), 2.2);
    let grain = textureSample(t_noise, s_repeat, world_pos.xy * 0.42 + vec2<f32>(2.0, -1.0)).r;
    let band = floor(diff * 3.5 + 0.5) / 3.5;
    let light = vzglyd_ambient_light() + vzglyd_main_light_rgb() * band * vzglyd_direct_light_scale();
    let metal = base.rgb * (0.92 + grain * 0.16);
    return fog_rgb(metal * light + vec3<f32>(spec * 0.24 + rim * 0.10), world_pos);
}

@fragment
fn fs_main(in: VzglydVertexOutput) -> @location(0) vec4<f32> {
    let mode_code = clock_mode(in.mode);

    if mode_code == CLOCK_MODE_SKY {
        return vec4<f32>(sky_color(in.world_pos.xy, in.color.rgb), 1.0);
    }

    if mode_code == CLOCK_MODE_HALO {
        let radial = length(in.normal.xy);
        let ring = smoothstep(1.0, 0.35, radial) * smoothstep(0.12, 0.55, radial);
        let pulse = 0.72 + 0.28 * sin(u.time * 1.45);
        let rgb = fog_rgb(in.color.rgb * (0.55 + pulse * 0.85), in.world_pos);
        return vec4<f32>(rgb, in.color.a * ring);
    }

    if mode_code == CLOCK_MODE_GLASS {
        let radial = clamp(length(in.normal.xy), 0.0, 1.0);
        let rim = pow(radial, 3.0);
        let shimmer = textureSample(
            t_noise,
            s_repeat,
            in.normal.xy * 0.45 + vec2<f32>(u.time * 0.03, -u.time * 0.02),
        ).r;
        let alpha = in.color.a * (0.08 + rim * 0.46);
        let tint = in.color.rgb * (0.82 + shimmer * 0.22) + vec3<f32>(0.08, 0.11, 0.16) * rim;
        return vec4<f32>(fog_rgb(tint, in.world_pos), alpha);
    }

    if in.color.a < 0.999 {
        if in.color.a <= bayer_threshold(in.clip_pos) {
            discard;
        }
    }

    return vec4<f32>(lit_clock_surface(in.color, in.normal, in.world_pos), 1.0);
}
