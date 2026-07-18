use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeSpecies {
    Oak,
    Pine,
    Birch,
    Shrub,
    Cactus,
}

impl TreeSpecies {
    pub fn from_str(s: &str) -> Self {
        match s {
            "tree_pine" | "pine" | "tree" if s.contains("pine") => TreeSpecies::Pine,
            "tree_birch" | "birch" | "tree" if s.contains("birch") => TreeSpecies::Birch,
            "shrub" => TreeSpecies::Shrub,
            "cactus" => TreeSpecies::Cactus,
            _ => TreeSpecies::Oak,
        }
    }
}

#[derive(Clone)]
struct Branch {
    start: Vec3,
    end: Vec3,
    r_start: f32,
    r_end: f32,
}

// Deterministic LCG helper
struct Lcg {
    s: u32,
}
impl Lcg {
    fn new(seed: u32) -> Self {
        Self { s: seed }
    }
    fn next(&mut self) -> f32 {
        self.s = self.s.wrapping_mul(1103515245).wrapping_add(12345);
        (self.s as f32) / (u32::MAX as f32)
    }
}

#[allow(clippy::too_many_arguments)]
fn add_subdivided_branch(
    start: Vec3,
    end: Vec3,
    r_start: f32,
    r_end: f32,
    subdivisions: u32,
    wiggle: f32,
    species: TreeSpecies,
    lcg: &mut Lcg,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
) {
    let dir = (end - start).normalize();
    let length = start.distance(end);
    let sub_len = length / subdivisions as f32;

    let mut joints = Vec::new();
    joints.push(start);

    // Find axes perpendicular to dir
    let right = if dir.x.abs() < 0.9 {
        dir.cross(Vec3::X).normalize()
    } else {
        dir.cross(Vec3::Z).normalize()
    };
    let up = right.cross(dir).normalize();

    for i in 1..subdivisions {
        let t = i as f32 / subdivisions as f32;
        let mut next_pos = start + dir * (t * length);

        // Add wiggle offset perpendicular to branch direction
        let angle1 = lcg.next() * std::f32::consts::TAU;
        let r_wiggle = wiggle * sub_len * (lcg.next() * 0.8 + 0.2);
        let offset = (right * angle1.cos() + up * angle1.sin()) * r_wiggle;

        // Damp wiggle near start
        let damp = if t < 0.25 { t * 4.0 } else { 1.0 };
        next_pos += offset * damp;
        joints.push(next_pos);
    }
    joints.push(end);

    // Build cylinders between joints
    for i in 0..subdivisions as usize {
        let j_start = joints[i];
        let j_end = joints[i + 1];
        let t0 = i as f32 / subdivisions as f32;
        let t1 = (i + 1) as f32 / subdivisions as f32;
        let radius_start = r_start * (1.0 - t0) + r_end * t0;
        let radius_end = r_start * (1.0 - t1) + r_end * t1;

        let seg_branch = Branch {
            start: j_start,
            end: j_end,
            r_start: radius_start,
            r_end: radius_end,
        };
        add_branch_segment_mesh(
            &seg_branch,
            species,
            lcg,
            positions,
            normals,
            colors,
            indices,
        );
    }
}

fn add_branch_segment_mesh(
    branch: &Branch,
    species: TreeSpecies,
    lcg: &mut Lcg,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
) {
    let dir = (branch.end - branch.start).normalize();
    let right = if dir.x.abs() < 0.9 {
        dir.cross(Vec3::X).normalize()
    } else {
        dir.cross(Vec3::Z).normalize()
    };
    let up = right.cross(dir).normalize();

    let segments = 8;
    let base_idx = positions.len() as u32;

    for i in 0..=segments {
        let theta = (i as f32 * 2.0 * std::f32::consts::PI) / segments as f32;
        let cos_t = theta.cos();
        let sin_t = theta.sin();

        let n = (right * cos_t + up * sin_t).normalize();
        let p_start = branch.start + n * branch.r_start;
        let p_end = branch.end + n * branch.r_end;

        // Start vertex
        positions.push([p_start.x, p_start.y, p_start.z]);
        normals.push([n.x, n.y, n.z]);
        let start_color = compute_bark_color(p_start, theta, species, lcg);
        colors.push(start_color);

        // End vertex
        positions.push([p_end.x, p_end.y, p_end.z]);
        normals.push([n.x, n.y, n.z]);
        let end_color = compute_bark_color(p_end, theta, species, lcg);
        colors.push(end_color);
    }

    for i in 0..segments {
        let i0 = base_idx + i * 2;
        let i1 = i0 + 1;
        let i2 = i0 + 2;
        let i3 = i0 + 3;

        indices.push(i0);
        indices.push(i1);
        indices.push(i2);

        indices.push(i2);
        indices.push(i1);
        indices.push(i3);
    }
}

fn compute_bark_color(p: Vec3, theta: f32, species: TreeSpecies, lcg: &mut Lcg) -> [f32; 4] {
    let rng_val = lcg.next();
    match species {
        TreeSpecies::Oak => {
            // Oak tree bark: Rich deep brown with dark vertical furrows
            let furrow = (theta * 10.0).cos();
            if furrow > 0.15 {
                [
                    0.26 + rng_val * 0.04,
                    0.16 + rng_val * 0.02,
                    0.08 + rng_val * 0.02,
                    1.0,
                ]
            } else {
                [
                    0.36 + rng_val * 0.05,
                    0.24 + rng_val * 0.03,
                    0.12 + rng_val * 0.02,
                    1.0,
                ]
            }
        }
        TreeSpecies::Pine => {
            // Pine tree bark: Reddish-brown with flaky variations
            let flake = (theta * 6.0).sin() * (p.y * 8.0).sin() + rng_val * 0.3;
            if flake > 0.45 {
                [0.40 + rng_val * 0.04, 0.22 + rng_val * 0.02, 0.14, 1.0]
            } else if flake < -0.45 {
                [0.24 + rng_val * 0.03, 0.16 + rng_val * 0.02, 0.12, 1.0]
            } else {
                [0.32 + rng_val * 0.04, 0.20 + rng_val * 0.03, 0.13, 1.0]
            }
        }
        TreeSpecies::Birch => {
            // Birch tree bark: White with black spots and bands
            let spot_pattern = (p.y * 15.0).cos() * (theta * 4.0).sin();
            let band_pattern = (p.y * 3.5).cos() * (theta * 1.5).cos();
            let val = spot_pattern.max(band_pattern) + rng_val * 0.2;
            if val > 0.65 {
                [
                    0.10 + rng_val * 0.03,
                    0.10 + rng_val * 0.03,
                    0.10 + rng_val * 0.03,
                    1.0,
                ]
            } else {
                let wh = 0.88 + rng_val * 0.08;
                [wh, wh, wh - 0.02, 1.0]
            }
        }
        TreeSpecies::Cactus => {
            // Cactus skin: saturated cactus green with vertical rib highlights
            let rib = (theta * 8.0).cos();
            if rib > 0.1 {
                [
                    0.12 + rng_val * 0.04,
                    0.46 + rng_val * 0.04,
                    0.14 + rng_val * 0.02,
                    1.0,
                ] // bright ribs
            } else {
                [
                    0.08 + rng_val * 0.03,
                    0.36 + rng_val * 0.03,
                    0.09 + rng_val * 0.02,
                    1.0,
                ] // darker ribs
            }
        }
        TreeSpecies::Shrub => {
            // Shrub wood: light woody brown/tan
            [0.34 + rng_val * 0.04, 0.24 + rng_val * 0.03, 0.14, 1.0]
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn add_sphere_mesh_with_color(
    center: Vec3,
    radius: f32,
    species: TreeSpecies,
    lcg: &mut Lcg,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
) {
    let rings = 7;
    let sectors = 9;
    let base_idx = positions.len() as u32;

    let sun_dir = Vec3::new(0.35, 0.85, 0.4).normalize();

    for r in 0..=rings {
        let phi = std::f32::consts::PI * (r as f32 / rings as f32);
        let sin_phi = phi.sin();
        let cos_phi = phi.cos();

        for s in 0..=sectors {
            let theta = 2.0 * std::f32::consts::PI * (s as f32 / sectors as f32);
            let sin_theta = theta.sin();
            let cos_theta = theta.cos();

            let nx = cos_theta * sin_phi;
            let ny = cos_phi;
            let nz = sin_theta * sin_phi;

            let px = center.x + nx * radius;
            let py = center.y + ny * radius;
            let pz = center.z + nz * radius;

            positions.push([px, py, pz]);
            normals.push([nx, ny, nz]);

            // Volumetric leaf shading based on alignment with sun
            let v_pos = Vec3::new(px, py, pz);
            let dir_from_center = (v_pos - center).normalize();
            let sun_alignment = dir_from_center.dot(sun_dir);
            let factor = (sun_alignment * 0.5 + 0.5).clamp(0.0, 1.0);

            let mut col = match species {
                TreeSpecies::Oak => {
                    let shadow_col = [0.06, 0.18, 0.06, 1.0];
                    let sun_col = [0.32, 0.62, 0.14, 1.0];
                    lerp_color(shadow_col, sun_col, factor)
                }
                TreeSpecies::Pine => {
                    let shadow_col = [0.03, 0.13, 0.10, 1.0];
                    let sun_col = [0.10, 0.38, 0.22, 1.0];
                    lerp_color(shadow_col, sun_col, factor)
                }
                TreeSpecies::Birch => {
                    let shadow_col = [0.16, 0.28, 0.06, 1.0];
                    let sun_col = [0.48, 0.70, 0.12, 1.0];
                    lerp_color(shadow_col, sun_col, factor)
                }
                TreeSpecies::Shrub => {
                    let shadow_col = [0.14, 0.32, 0.18, 1.0];
                    let sun_col = [0.38, 0.72, 0.24, 1.0];
                    lerp_color(shadow_col, sun_col, factor)
                }
                TreeSpecies::Cactus => {
                    // Cacti do not have leaves, but we output a fallback green just in case
                    [0.12, 0.44, 0.15, 1.0]
                }
            };

            let jitter = (lcg.next() - 0.5) * 0.04;
            col[0] = (col[0] + jitter).clamp(0.0, 1.0);
            col[1] = (col[1] + jitter).clamp(0.0, 1.0);
            col[2] = (col[2] + jitter).clamp(0.0, 1.0);

            colors.push(col);
        }
    }

    for r in 0..rings {
        for s in 0..sectors {
            let i0 = base_idx + r * (sectors + 1) + s;
            let i1 = i0 + (sectors + 1);
            let i2 = i0 + 1;
            let i3 = i1 + 1;

            indices.push(i0);
            indices.push(i1);
            indices.push(i2);

            indices.push(i2);
            indices.push(i1);
            indices.push(i3);
        }
    }
}

fn lerp_color(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] * (1.0 - t) + b[0] * t,
        a[1] * (1.0 - t) + b[1] * t,
        a[2] * (1.0 - t) + b[2] * t,
        a[3] * (1.0 - t) + b[3] * t,
    ]
}

#[allow(clippy::too_many_arguments)]
fn generate_oak_branches(
    start: Vec3,
    dir: Vec3,
    length: f32,
    r: f32,
    depth: u32,
    lcg: &mut Lcg,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
    leaf_positions: &mut Vec<(Vec3, f32)>,
) {
    let end = start + dir * length;
    let r_end = r * 0.72;

    let subdivisions = if depth >= 2 { 4 } else { 2 };
    let wiggle = 0.15;

    add_subdivided_branch(
        start,
        end,
        r,
        r_end,
        subdivisions,
        wiggle,
        TreeSpecies::Oak,
        lcg,
        positions,
        normals,
        colors,
        indices,
    );

    if depth == 0 {
        leaf_positions.push((end, length * 1.1));
        return;
    }

    let splits = if depth == 3 { 3 } else { 2 };
    for i in 0..splits {
        let angle = 0.32 + lcg.next() * 0.28;
        let angle_rot = (i as f32 * 2.0 * std::f32::consts::PI / splits as f32) + lcg.next() * 0.5;
        let right = if dir.x.abs() < 0.9 {
            dir.cross(Vec3::X).normalize()
        } else {
            dir.cross(Vec3::Z).normalize()
        };
        let up = right.cross(dir).normalize();
        let rot_axis = (right * angle_rot.cos() + up * angle_rot.sin()).normalize();

        let q = Quat::from_axis_angle(rot_axis, angle);
        let mut new_dir = q * dir;
        new_dir.y = (new_dir.y + 0.3).max(0.2);
        new_dir = new_dir.normalize();

        let new_len = length * (0.64 + lcg.next() * 0.14);
        let new_r = r_end * (0.65 + lcg.next() * 0.1);

        generate_oak_branches(
            end,
            new_dir,
            new_len,
            new_r,
            depth - 1,
            lcg,
            positions,
            normals,
            colors,
            indices,
            leaf_positions,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn generate_birch_branches(
    start: Vec3,
    dir: Vec3,
    length: f32,
    r: f32,
    depth: u32,
    lcg: &mut Lcg,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
    leaf_positions: &mut Vec<(Vec3, f32)>,
) {
    let end = start + dir * length;
    let r_end = r * 0.68;

    let subdivisions = if depth >= 2 { 3 } else { 2 };
    let wiggle = 0.05;

    add_subdivided_branch(
        start,
        end,
        r,
        r_end,
        subdivisions,
        wiggle,
        TreeSpecies::Birch,
        lcg,
        positions,
        normals,
        colors,
        indices,
    );

    if depth == 0 {
        leaf_positions.push((end, length * 1.0));
        return;
    }

    let splits = 2;
    for i in 0..splits {
        let angle = 0.38 + lcg.next() * 0.2;
        let angle_rot = (i as f32 * std::f32::consts::PI) + lcg.next() * 0.4;
        let right = if dir.x.abs() < 0.9 {
            dir.cross(Vec3::X).normalize()
        } else {
            dir.cross(Vec3::Z).normalize()
        };
        let up = right.cross(dir).normalize();
        let rot_axis = (right * angle_rot.cos() + up * angle_rot.sin()).normalize();

        let q = Quat::from_axis_angle(rot_axis, angle);
        let mut new_dir = q * dir;

        if depth <= 2 {
            new_dir.y = (new_dir.y - 0.2).clamp(-0.4, 0.4);
        } else {
            new_dir.y = (new_dir.y + 0.2).max(0.1);
        }
        new_dir = new_dir.normalize();

        let new_len = length * (0.68 + lcg.next() * 0.1);
        let new_r = r_end * (0.62 + lcg.next() * 0.08);

        generate_birch_branches(
            end,
            new_dir,
            new_len,
            new_r,
            depth - 1,
            lcg,
            positions,
            normals,
            colors,
            indices,
            leaf_positions,
        );
    }
}

fn generate_pine_structure(
    lcg: &mut Lcg,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
    leaf_positions: &mut Vec<(Vec3, f32)>,
) {
    let trunk_height = 2.4 + lcg.next() * 1.4;
    let base_radius = 0.16 + lcg.next() * 0.04;

    let subdivisions = 6;
    let wiggle = 0.02;

    add_subdivided_branch(
        Vec3::ZERO,
        Vec3::Y * trunk_height,
        base_radius,
        0.015,
        subdivisions,
        wiggle,
        TreeSpecies::Pine,
        lcg,
        positions,
        normals,
        colors,
        indices,
    );

    let layers = 6;
    let start_y = trunk_height * 0.25;
    let y_step = (trunk_height - start_y) / (layers as f32);

    for l in 0..layers {
        let t_height = start_y + (l as f32) * y_step;
        let height_pct = t_height / trunk_height;
        let max_branch_len = (1.5 - height_pct * 1.1) * (0.8 + lcg.next() * 0.4);
        let num_branches = 5 - (height_pct * 2.0) as u32;
        let num_branches = num_branches.max(3);

        for b in 0..num_branches {
            let angle_rad =
                (b as f32 * 2.0 * std::f32::consts::PI / num_branches as f32) + lcg.next() * 0.2;
            let b_dir =
                Vec3::new(angle_rad.cos(), 0.05 + lcg.next() * 0.08, angle_rad.sin()).normalize();

            let b_start = Vec3::new(0.0, t_height, 0.0);
            let b_end = b_start + b_dir * max_branch_len;
            let b_r_start = base_radius * (1.0 - height_pct) * 0.4;
            let b_r_end = b_r_start * 0.5;

            add_subdivided_branch(
                b_start,
                b_end,
                b_r_start,
                b_r_end,
                2,
                0.04,
                TreeSpecies::Pine,
                lcg,
                positions,
                normals,
                colors,
                indices,
            );

            leaf_positions.push((b_end, max_branch_len * 0.38));
            leaf_positions.push((
                b_start + b_dir * (max_branch_len * 0.6),
                max_branch_len * 0.32,
            ));
        }
    }
}

pub fn build_tree_meshes(seed: u32, species_str: &str) -> (Mesh, Mesh) {
    let mut lcg = Lcg::new(seed);
    let species = TreeSpecies::from_str(species_str);

    let mut trunk_positions = Vec::new();
    let mut trunk_normals = Vec::new();
    let mut trunk_colors = Vec::new();
    let mut trunk_indices = Vec::new();

    let mut leaf_positions = Vec::new();

    match species {
        TreeSpecies::Oak => {
            let length = 1.3 + lcg.next() * 0.5;
            let radius = 0.22 + lcg.next() * 0.05;
            generate_oak_branches(
                Vec3::ZERO,
                Vec3::Y,
                length,
                radius,
                3,
                &mut lcg,
                &mut trunk_positions,
                &mut trunk_normals,
                &mut trunk_colors,
                &mut trunk_indices,
                &mut leaf_positions,
            );
        }
        TreeSpecies::Birch => {
            let length = 1.8 + lcg.next() * 0.5;
            let radius = 0.12 + lcg.next() * 0.02;
            generate_birch_branches(
                Vec3::ZERO,
                Vec3::Y,
                length,
                radius,
                3,
                &mut lcg,
                &mut trunk_positions,
                &mut trunk_normals,
                &mut trunk_colors,
                &mut trunk_indices,
                &mut leaf_positions,
            );
        }
        TreeSpecies::Pine => {
            generate_pine_structure(
                &mut lcg,
                &mut trunk_positions,
                &mut trunk_normals,
                &mut trunk_colors,
                &mut trunk_indices,
                &mut leaf_positions,
            );
        }
        TreeSpecies::Shrub => {
            // Shrub has a tiny trunk and a massive cluster of leaves close to the ground
            let radius = 0.08;
            add_subdivided_branch(
                Vec3::ZERO,
                Vec3::new(0.0, 0.15, 0.0),
                radius,
                radius * 0.8,
                2,
                0.01,
                TreeSpecies::Shrub,
                &mut lcg,
                &mut trunk_positions,
                &mut trunk_normals,
                &mut trunk_colors,
                &mut trunk_indices,
            );
            // Cluster of leaves
            leaf_positions.push((Vec3::new(0.0, 0.5, 0.0), 0.75));
        }
        TreeSpecies::Cactus => {
            // Main vertical stem
            add_subdivided_branch(
                Vec3::ZERO,
                Vec3::new(0.0, 1.6, 0.0),
                0.14,
                0.12,
                4,
                0.02,
                TreeSpecies::Cactus,
                &mut lcg,
                &mut trunk_positions,
                &mut trunk_normals,
                &mut trunk_colors,
                &mut trunk_indices,
            );

            // Left arm (out then up)
            add_subdivided_branch(
                Vec3::new(0.0, 0.7, 0.0),
                Vec3::new(-0.35, 0.7, 0.0),
                0.10,
                0.09,
                2,
                0.0,
                TreeSpecies::Cactus,
                &mut lcg,
                &mut trunk_positions,
                &mut trunk_normals,
                &mut trunk_colors,
                &mut trunk_indices,
            );
            add_subdivided_branch(
                Vec3::new(-0.35, 0.7, 0.0),
                Vec3::new(-0.35, 1.3, 0.0),
                0.09,
                0.08,
                2,
                0.0,
                TreeSpecies::Cactus,
                &mut lcg,
                &mut trunk_positions,
                &mut trunk_normals,
                &mut trunk_colors,
                &mut trunk_indices,
            );

            // Right arm (out then up)
            add_subdivided_branch(
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(0.35, 1.0, 0.0),
                0.10,
                0.09,
                2,
                0.0,
                TreeSpecies::Cactus,
                &mut lcg,
                &mut trunk_positions,
                &mut trunk_normals,
                &mut trunk_colors,
                &mut trunk_indices,
            );
            add_subdivided_branch(
                Vec3::new(0.35, 1.0, 0.0),
                Vec3::new(0.35, 1.6, 0.0),
                0.09,
                0.08,
                2,
                0.0,
                TreeSpecies::Cactus,
                &mut lcg,
                &mut trunk_positions,
                &mut trunk_normals,
                &mut trunk_colors,
                &mut trunk_indices,
            );
        }
    }

    let mut trunk_mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    trunk_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, trunk_positions);
    trunk_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, trunk_normals);
    trunk_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, trunk_colors);
    trunk_mesh.insert_indices(Indices::U32(trunk_indices));

    let mut leaves_positions = Vec::new();
    let mut leaves_normals = Vec::new();
    let mut leaves_colors = Vec::new();
    let mut leaves_indices = Vec::new();

    for (pos, rad) in &leaf_positions {
        let r = *rad * 0.95;
        add_sphere_mesh_with_color(
            *pos,
            r,
            species,
            &mut lcg,
            &mut leaves_positions,
            &mut leaves_normals,
            &mut leaves_colors,
            &mut leaves_indices,
        );

        if species != TreeSpecies::Pine {
            add_sphere_mesh_with_color(
                *pos + Vec3::new(r * 0.5, r * 0.25, -r * 0.25),
                r * 0.75,
                species,
                &mut lcg,
                &mut leaves_positions,
                &mut leaves_normals,
                &mut leaves_colors,
                &mut leaves_indices,
            );
            add_sphere_mesh_with_color(
                *pos + Vec3::new(-r * 0.4, -r * 0.15, r * 0.5),
                r * 0.65,
                species,
                &mut lcg,
                &mut leaves_positions,
                &mut leaves_normals,
                &mut leaves_colors,
                &mut leaves_indices,
            );
        }
    }

    let mut leaves_mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    leaves_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, leaves_positions);
    leaves_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, leaves_normals);
    leaves_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, leaves_colors);
    leaves_mesh.insert_indices(Indices::U32(leaves_indices));

    (trunk_mesh, leaves_mesh)
}

pub fn build_rock_mesh(seed: u32) -> Mesh {
    let mut lcg = Lcg::new(seed);

    // Generate a basic sphere grid (stacks & sectors)
    let sectors = 8;
    let stacks = 6;

    let mut raw_vertices = Vec::new();

    // Generate deformed coordinates using LCG random offsets
    for i in 0..=stacks {
        let phi = std::f32::consts::PI * (i as f32 / stacks as f32);
        for j in 0..sectors {
            let theta = std::f32::consts::TAU * (j as f32 / sectors as f32);

            let sin_phi = phi.sin();
            let cos_phi = phi.cos();
            let sin_theta = theta.sin();
            let cos_theta = theta.cos();

            let base_pos = Vec3::new(sin_phi * cos_theta, cos_phi, sin_phi * sin_theta);

            // Random displacement along the vertex normal to make it jagged/rugged
            // Add a combination of large scale deformation and small scale bumps
            let mut displacement = 0.8 + lcg.next() * 0.4;
            // Introduce sharp ridges/facets
            if lcg.next() < 0.3 {
                displacement *= 0.7; // sharp inward cut
            } else if lcg.next() < 0.2 {
                displacement *= 1.3; // sharp outward facet
            }

            raw_vertices.push(base_pos * displacement);
        }
    }

    // Construct triangles and build a flat-shaded mesh
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    // Gray rock colors with slight variations based on lcg
    let base_color = Color::srgb(
        0.38 + lcg.next() * 0.08,
        0.38 + lcg.next() * 0.08,
        0.40 + lcg.next() * 0.08,
    );

    for i in 0..stacks {
        for j in 0..sectors {
            let next_j = (j + 1) % sectors;

            let p00 = raw_vertices[i * sectors + j];
            let p01 = raw_vertices[i * sectors + next_j];
            let p10 = raw_vertices[(i + 1) * sectors + j];
            let p11 = raw_vertices[(i + 1) * sectors + next_j];

            // Triangle 1
            add_flat_triangle(
                p00,
                p01,
                p10,
                base_color,
                &mut positions,
                &mut normals,
                &mut colors,
                &mut indices,
                &mut lcg,
            );

            // Triangle 2
            add_flat_triangle(
                p01,
                p11,
                p10,
                base_color,
                &mut positions,
                &mut normals,
                &mut colors,
                &mut indices,
                &mut lcg,
            );
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

#[allow(clippy::too_many_arguments)]
fn add_flat_triangle(
    p0: Vec3,
    p1: Vec3,
    p2: Vec3,
    base_color: Color,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
    lcg: &mut Lcg,
) {
    let edge1 = p1 - p0;
    let edge2 = p2 - p0;
    let normal = edge1.cross(edge2).normalize_or_zero();

    let start_idx = positions.len() as u32;

    positions.push(p0.to_array());
    positions.push(p1.to_array());
    positions.push(p2.to_array());

    // Add flat face normal to all 3 vertices
    normals.push(normal.to_array());
    normals.push(normal.to_array());
    normals.push(normal.to_array());

    // Slight shadow variation on each face based on its angle and LCG jitter
    let shade_factor = 0.75 + lcg.next() * 0.3;
    let face_color = [
        (base_color.to_linear().red * shade_factor).clamp(0.0, 1.0),
        (base_color.to_linear().green * shade_factor).clamp(0.0, 1.0),
        (base_color.to_linear().blue * shade_factor).clamp(0.0, 1.0),
        1.0,
    ];

    colors.push(face_color);
    colors.push(face_color);
    colors.push(face_color);

    indices.push(start_idx);
    indices.push(start_idx + 1);
    indices.push(start_idx + 2);
}
