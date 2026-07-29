#import bevy_pbr::forward_io::{Vertex, VertexOutput}
#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::mesh_functions as mesh_functions
#import "shaders/sky_common.wgsl"::get_sky_color

@group(3) @binding(3) var<storage, read> height_buffer: array<f32>;

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    let gx = clamp(u32(vertex.uv.x * 255.0), 0u, 255u);
    let gy = clamp(u32(vertex.uv.y * 255.0), 0u, 255u);
    let idx = gx * 256u + gy;
    let h = height_buffer[idx];
    var local_position = vertex.position;
    local_position.y = (h - 1.0) * 0.25;

    // Calculate local normal from adjacent heights
    let h_left = height_buffer[clamp(gx - 1u, 0u, 255u) * 256u + gy];
    let h_right = height_buffer[clamp(gx + 1u, 0u, 255u) * 256u + gy];
    let h_up = height_buffer[gx * 256u + clamp(gy - 1u, 0u, 255u)];
    let h_down = height_buffer[gx * 256u + clamp(gy + 1u, 0u, 255u)];

    let dx = (h_right - h_left) * 0.25 / 8.0;
    let dy = (h_down - h_up) * 0.25 / 8.0;
    let local_normal = normalize(vec3<f32>(-dx, 1.0, -dy));

    let model = mesh_functions::get_world_from_local(vertex.instance_index);
    
    out.world_position = mesh_functions::mesh_position_local_to_world(
        model,
        vec4<f32>(local_position, 1.0)
    );
    
    out.position = mesh_functions::mesh_position_local_to_clip(
        model,
        vec4<f32>(local_position, 1.0)
    );

    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        local_normal,
        vertex.instance_index
    );

    out.uv = vertex.uv;

    return out;
}

const LIGHT_POSITION: vec3<f32> = vec3<f32>(20.0, 35.0, -25.0);
const LIGHT_COLOR: vec3<f32> = vec3<f32>(1.0, 0.94, 0.82);
const SEA_BASE: vec3<f32> = vec3<f32>(0.015, 0.09, 0.18);
const SEA_WATER_COLOR: vec3<f32> = vec3<f32>(0.10, 0.48, 0.85);
const SEA_SPEED: f32 = 0.15;
const SEA_FREQ: f32 = 0.22;
const PI: f32 = 3.14159265359;

// Wavelength-dependent absorption coefficients (per meter of depth)
const ABSORPTION: vec3<f32> = vec3<f32>(0.45, 0.08, 0.04);

const F0_WATER: f32 = 0.04;

const OCTAVE_M: mat2x2<f32> = mat2x2<f32>(
    vec2<f32>(1.6, 1.2),
    vec2<f32>(-1.2, 1.6)
);

struct WaterMaterial {
    color: vec4<f32>,
    time: f32,
    camera_position: vec3<f32>,
    resolution: vec2<f32>,
    water_level: f32,
    grid_scale: f32,
    cloudiness: f32,
};

@group(3) @binding(0) var<uniform> material: WaterMaterial;
@group(3) @binding(1) var reflection_texture: texture_2d<f32>;
@group(3) @binding(2) var reflection_sampler: sampler;

// --- Noise ---

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453123);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash(i), hash(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y
    ) * 2.0 - 1.0;
}

// --- Enhanced Normal Map Perturbation ---

fn get_wave_normal(world_pos: vec3<f32>, time: f32) -> vec3<f32> {
    let base_uv = world_pos.xz * SEA_FREQ;
    let t = time * SEA_SPEED;
    var wave = vec2<f32>(0.0);
    
    // Paired opposing direction vectors to cancel out net flow
    let dirs = array<vec2<f32>, 5>(
        vec2<f32>(0.4, 0.3),
        vec2<f32>(-0.4, -0.3),
        vec2<f32>(-0.35, 0.35),
        vec2<f32>(0.35, -0.35),
        vec2<f32>(0.1, -0.1)
    );

    var uv = base_uv;
    var freq = 1.0;
    var amp = 0.20;

    for (var i = 0; i < 5; i++) {
        let n1 = noise(uv * freq + t * dirs[i]);
        let n2 = noise(uv * freq * 1.38 - t * dirs[i]);
        wave += vec2<f32>(n1, n2) * amp;
        uv = OCTAVE_M * uv;
        freq *= 1.80;
        amp *= 0.35;
    }
    return normalize(vec3<f32>(wave.x * 0.4, 1.0, wave.y * 0.4));
}

// --- Caustics ---

fn get_caustics(world_pos: vec3<f32>, time: f32) -> vec3<f32> {
    let uv = world_pos.xz * 0.75;
    let t = time * 0.45;

    var intensity = 0.0;
    intensity += noise(uv + t * vec2<f32>(1.2, 0.8)) * 0.7;
    intensity += noise(uv * 2.3 - t * vec2<f32>(0.9, 1.4)) * 0.45;
    intensity += noise(uv * 4.7 + t * vec2<f32>(1.6, -0.9)) * 0.25;

    intensity = pow(max(0.0, intensity * 0.6 + 0.45), 3.6);

    let depth_factor = smoothstep(6.0, 0.0, world_pos.y - material.water_level);
    
    return vec3<f32>(0.5, 0.9, 1.3) * intensity * depth_factor * 1.35;
}

// --- Lighting ---

fn diffuse(n: vec3<f32>, l: vec3<f32>, p: f32) -> f32 {
    return pow(max(dot(n, l) * 0.5 + 0.5, 0.0), p);
}

fn specular(n: vec3<f32>, l: vec3<f32>, e: vec3<f32>, s: f32) -> f32 {
    let nrm = (s + 8.0) / (PI * 8.0);
    return pow(max(dot(reflect(e, n), l), 0.0), s) * nrm;
}

// --- Core water color ---

fn get_water_color(
    p: vec3<f32>,
    n: vec3<f32>,
    l: vec3<f32>,
    eye: vec3<f32>,
    dist: vec3<f32>,
    water_level: f32
) -> vec3<f32> {

    let cos_theta = max(dot(n, eye), 0.0);
    let fresnel = F0_WATER + (1.0 - F0_WATER) * pow(1.0 - cos_theta, 5.0);

    var reflected_color = get_sky_color(reflect(-eye, n));

    let height_factor = clamp((p.y - water_level + 0.4) / 0.8, 0.0, 1.0);
    let water_deep = vec3<f32>(0.05, 0.15, 0.4);
    let water_shallow = vec3<f32>(0.3, 0.8, 1.0);
    let wave_base_color = mix(water_deep, water_shallow, height_factor);

    let ndotl = max(dot(n, l), 0.0);
    let lit_water = wave_base_color * (0.4 + 0.8 * ndotl);

    let depth_darkening = smoothstep(0.0, 0.5, 1.0 - height_factor);
    let flo_shaded_water = mix(lit_water, lit_water * 0.3, depth_darkening);

    let depth = max(0.0, water_level - p.y + 1.0);
    let absorbed = exp(-ABSORPTION * depth);
    let refracted = flo_shaded_water * absorbed;

    var color = refracted;

    let wave_height = p.y - water_level;
    let sss_dot = pow(max(0.0, dot(l, -eye)), 4.0);
    let sss_height = smoothstep(-0.2, 1.5, wave_height);
    let sss_thin = smoothstep(2.0, 0.0, max(0.0, wave_height));
    let sss_intensity = sss_dot * sss_height * sss_thin * 0.6;
    let sss_color = vec3<f32>(0.08, 0.6, 0.55);
    color += sss_color * sss_intensity;

    let caustics = get_caustics(p, material.time);
    color += caustics * (1.0 - fresnel) * 0.6;

    let atten = max(1.0 - length(dist) * 0.000065, 0.0);
    color *= atten;

    let spec = specular(n, l, eye, 160.0);
    color += LIGHT_COLOR * (spec * 1.25);

    let shore = smoothstep(0.0, 1.6, p.y - water_level + 0.8);
    color = mix(color, vec3<f32>(0.96, 0.98, 1.0), shore * 0.18);

    let view_dist = length(dist);
    let fog = 1.0 - exp(-view_dist * 0.0006);
    let horizon_color = get_sky_color(vec3<f32>(0.0, 0.02, 0.0));
    color = mix(color, horizon_color, fog);

    return color;
}

fn get_normal_from_mesh(world_normal: vec3<f32>) -> vec3<f32> {
    var n = normalize(world_normal);
    if (n.y < 0.0) { n = -n; }
    return n;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let world_pos = in.world_position.xyz;
    let eye_dir = normalize(material.camera_position - world_pos);
    let light_dir = normalize(LIGHT_POSITION);

    var normal = get_normal_from_mesh(in.world_normal);
    let wave_normal = get_wave_normal(world_pos, material.time);

    let blend = 0.6 + 0.4 * (1.0 - abs(dot(eye_dir, vec3<f32>(0.0, 1.0, 0.0))));
    normal = normalize(mix(normal, wave_normal, blend));

    let mirrored_pos = vec4<f32>(world_pos.x, 2.0 * material.water_level - world_pos.y, world_pos.z, 1.0);
    let clip_refl = view.clip_from_world * mirrored_pos;
    let ndc_refl = clip_refl.xy / clip_refl.w;
    
    var proj_uv = ndc_refl * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    
    let view_normal = (view.view_from_world * vec4<f32>(normal, 0.0)).xyz;
    let distortion = view_normal.xy * 0.007;
    
    let refl_uv = clamp(
        vec2<f32>(proj_uv.x + distortion.x, proj_uv.y + distortion.y),
        vec2<f32>(0.001, 0.001),
        vec2<f32>(0.999, 0.999)
    );
    let scene_refl_sample = textureSample(reflection_texture, reflection_sampler, refl_uv);
    let scene_reflection = scene_refl_sample.rgb;
    let scene_alpha = scene_refl_sample.a;
    
    let reflect_dir = reflect(-eye_dir, normal);
    let base_sky = get_sky_color(reflect_dir);
    let storm_sky = vec3<f32>(0.12, 0.14, 0.18) * (reflect_dir.y * 0.4 + 0.6);
    var sky_reflection = mix(base_sky, storm_sky, material.cloudiness);

    if (reflect_dir.y > 0.0) {
        let sky_uv = reflect_dir.xz / (reflect_dir.y + 0.01);
        let cloud_noise = noise(sky_uv * 1.5 + material.time * vec2<f32>(0.05, 0.03));
        let cloud_mask = smoothstep(-0.15, 0.45, cloud_noise) * smoothstep(0.0, 0.08, reflect_dir.y) * material.cloudiness;
        let c_color = vec3<f32>(0.85, 0.88, 0.92) * (1.0 - material.cloudiness * 0.48);
        sky_reflection = mix(sky_reflection, c_color, cloud_mask * 0.72);
    }

    let reflected = mix(sky_reflection, scene_reflection, scene_alpha * 0.95);

    let dist = material.camera_position - world_pos;

    var water_color = get_water_color(
        world_pos, normal, light_dir, eye_dir, dist, material.water_level
    );

    let cos_theta = max(dot(normal, eye_dir), 0.0);
    let fresnel = F0_WATER + (1.0 - F0_WATER) * pow(1.0 - cos_theta, 5.0);
    water_color = mix(water_color * 0.65, reflected, fresnel);

    let glance = pow(1.0 - cos_theta, 3.0);
    let final_alpha = clamp(mix(material.color.a, 0.88, glance), 0.28, 0.88);

    return vec4<f32>(water_color, final_alpha);
}
