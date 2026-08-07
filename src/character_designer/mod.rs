use crate::AppState;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

pub struct CharacterDesignerPlugin;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Gender {
    #[default]
    Male,
    Female,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum HairStyle {
    #[default]
    None,
    Short,
    Ponytail,
    Spiky,
    Curly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum OutfitStyle {
    #[default]
    SciFiSuit,
    TacticalArmor,
    StylizedHero,
    SkeletonExoFrame,
    ClassicMannequin,
}

#[allow(dead_code)]
#[derive(Resource)]
pub struct CharacterSettings {
    pub gender: Gender,
    pub outfit_style: OutfitStyle,
    pub height: f32, // 1.2 to 2.2
    pub weight: f32, // 0.5 to 1.5
    pub skin_color: Color,
    pub eye_color: Color,
    pub hair_style: HairStyle,
    pub hair_color: Color,
    pub head_scale: f32,     // 0.8 to 1.3
    pub muscle_mass: f32,    // 0.0 to 1.5
    pub shoulder_width: f32, // 0.7 to 1.4
    pub leg_length: f32,     // 0.7 to 1.4
    pub waist_width: f32,    // 0.7 to 1.4
    pub custom_name: String,
    pub is_ragdoll_active: bool,
    pub is_sprite_rendered: bool,
    pub render_angle: f32, // angle to capture
    pub show_xray: bool,
}

impl Default for CharacterSettings {
    fn default() -> Self {
        Self {
            gender: Gender::Male,
            outfit_style: OutfitStyle::SciFiSuit,
            height: 1.75,
            weight: 1.0,
            skin_color: Color::srgb(0.9, 0.72, 0.62),
            eye_color: Color::srgb(0.2, 0.4, 0.8),
            hair_style: HairStyle::Short,
            hair_color: Color::srgb(0.4, 0.25, 0.15),
            head_scale: 1.2,
            muscle_mass: 0.2,
            shoulder_width: 1.0,
            leg_length: 1.0,
            waist_width: 1.0,
            custom_name: "Tempest".to_string(),
            is_ragdoll_active: false,
            is_sprite_rendered: false,
            render_angle: 0.0,
            show_xray: true,
        }
    }
}

// Ragdoll Node representing each bone in Verlet physics
#[allow(dead_code)]
#[derive(Component, Debug, Clone)]
pub struct RagdollNode {
    pub name: String,
    pub position: Vec3,
    pub old_position: Vec3,
    pub radius: f32,
    pub offset_from_parent: Vec3,
    pub parent_name: String,
}

// Constraint connecting two RagdollNodes
#[derive(Debug, Clone)]
pub struct RagdollConstraint {
    pub node_a: String,
    pub node_b: String,
    pub rest_length: f32,
}

#[derive(Resource, Default)]
pub struct RagdollPhysics {
    pub nodes: Vec<RagdollNode>,
    pub constraints: Vec<RagdollConstraint>,
}

#[derive(Component)]
pub struct CharacterModelPart;

#[derive(Component)]
pub struct CharacterVisualEntity;

#[derive(Component)]
pub struct BoneVisual {
    pub name: String,
}

#[derive(Component)]
pub struct LimbVisual {
    pub node_a: String,
    pub node_b: String,
    pub radius: f32,
}

#[derive(Component)]
pub struct XraySkinVisual;

#[derive(Component)]
pub struct XraySkeletonVisual;

impl Plugin for CharacterDesignerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CharacterSettings>()
            .init_resource::<RagdollPhysics>()
            .add_systems(
                OnEnter(AppState::CharacterDesigner),
                setup_character_designer,
            )
            .add_systems(
                OnExit(AppState::CharacterDesigner),
                cleanup_character_designer,
            )
            .add_systems(
                Update,
                (
                    character_mesh_sync_system.run_if(in_state(AppState::CharacterDesigner)),
                    ragdoll_physics_system.run_if(in_state(AppState::CharacterDesigner)),
                    character_xray_system.run_if(in_state(AppState::CharacterDesigner)),
                ),
            )
            .add_systems(
                EguiPrimaryContextPass,
                character_designer_ui.run_if(in_state(AppState::CharacterDesigner)),
            );
    }
}

fn setup_character_designer(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    settings: Res<CharacterSettings>,
    mut physics: ResMut<RagdollPhysics>,
) {
    // 3D Camera with Ambient Light
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 1.8, 4.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
        AmbientLight {
            color: Color::WHITE,
            brightness: 350.0,
            ..default()
        },
        CharacterVisualEntity,
    ));

    // Directional Light for premium shadows/highlights
    commands.spawn((
        DirectionalLight {
            illuminance: 4500.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(5.0, 10.0, 5.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
        CharacterVisualEntity,
    ));

    // Simple grid / floor plane so the ragdoll has something to land on!
    let floor_mesh = meshes.add(Plane3d::default().mesh().size(15.0, 15.0));
    commands.spawn((
        Mesh3d(floor_mesh),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.22, 0.25),
            perceptual_roughness: 0.9,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
        CharacterVisualEntity,
    ));

    initialize_ragdoll_skeleton(&settings, &mut physics);
    spawn_character_visuals(
        &mut commands,
        &mut meshes,
        &mut materials,
        &settings,
        &physics,
    );
}

fn cleanup_character_designer(
    mut commands: Commands,
    query: Query<Entity, With<CharacterVisualEntity>>,
    visuals: Query<Entity, With<BoneVisual>>,
) {
    for entity in query.iter() {
        if let Ok(mut cmd) = commands.get_entity(entity) {
            cmd.despawn();
        }
    }
    for entity in visuals.iter() {
        if let Ok(mut cmd) = commands.get_entity(entity) {
            cmd.despawn();
        }
    }
}

fn initialize_ragdoll_skeleton(settings: &CharacterSettings, physics: &mut RagdollPhysics) {
    let h = settings.height;
    let w_thick = settings.weight;
    let muscle = settings.muscle_mass;
    let sh_w = settings.shoulder_width;
    let leg_len = settings.leg_length;
    let waist = settings.waist_width;

    // Realistic proportional heights
    let pelvis_y = h * 0.45 * (2.0 - leg_len);
    let spine_y = pelvis_y + (h * 0.15);
    let chest_y = pelvis_y + (h * 0.3);
    let head_y = chest_y + (h * 0.18);

    let knee_y = pelvis_y * 0.5;

    // Muscle and waist adjustments on joint radii
    let pelvis_rad = 0.15 * w_thick * waist;
    let spine_rad = 0.15 * w_thick * (0.8 + waist * 0.2);
    let chest_rad = 0.17 * w_thick * (1.0 + muscle * 0.15);
    let head_rad = 0.14 * settings.head_scale;

    let arm_rad = 0.08 * w_thick * (1.0 + muscle * 0.18);
    let leg_rad = 0.09 * w_thick * (1.0 + muscle * 0.15);

    let nodes = vec![
        RagdollNode {
            name: "Pelvis".to_string(),
            position: Vec3::new(0.0, pelvis_y, 0.0),
            old_position: Vec3::new(0.0, pelvis_y, 0.0),
            radius: pelvis_rad,
            offset_from_parent: Vec3::ZERO,
            parent_name: "".to_string(),
        },
        RagdollNode {
            name: "Spine".to_string(),
            position: Vec3::new(0.0, spine_y, 0.0),
            old_position: Vec3::new(0.0, spine_y, 0.0),
            radius: spine_rad,
            offset_from_parent: Vec3::new(0.0, spine_y - pelvis_y, 0.0),
            parent_name: "Pelvis".to_string(),
        },
        RagdollNode {
            name: "Chest".to_string(),
            position: Vec3::new(0.0, chest_y, 0.0),
            old_position: Vec3::new(0.0, chest_y, 0.0),
            radius: chest_rad,
            offset_from_parent: Vec3::new(0.0, chest_y - spine_y, 0.0),
            parent_name: "Spine".to_string(),
        },
        RagdollNode {
            name: "Head".to_string(),
            position: Vec3::new(0.0, head_y, 0.0),
            old_position: Vec3::new(0.0, head_y, 0.0),
            radius: head_rad,
            offset_from_parent: Vec3::new(0.0, head_y - chest_y, 0.0),
            parent_name: "Chest".to_string(),
        },
        // Arms
        RagdollNode {
            name: "L_Shoulder".to_string(),
            position: Vec3::new(-0.25 * w_thick * sh_w, chest_y, 0.0),
            old_position: Vec3::new(-0.25 * w_thick * sh_w, chest_y, 0.0),
            radius: arm_rad,
            offset_from_parent: Vec3::new(-0.25 * w_thick * sh_w, 0.0, 0.0),
            parent_name: "Chest".to_string(),
        },
        RagdollNode {
            name: "L_Elbow".to_string(),
            position: Vec3::new(-0.5 * w_thick * sh_w, chest_y, 0.0),
            old_position: Vec3::new(-0.5 * w_thick * sh_w, chest_y, 0.0),
            radius: arm_rad * 0.9,
            offset_from_parent: Vec3::new(-0.25 * w_thick * sh_w, 0.0, 0.0),
            parent_name: "L_Shoulder".to_string(),
        },
        RagdollNode {
            name: "R_Shoulder".to_string(),
            position: Vec3::new(0.25 * w_thick * sh_w, chest_y, 0.0),
            old_position: Vec3::new(0.25 * w_thick * sh_w, chest_y, 0.0),
            radius: arm_rad,
            offset_from_parent: Vec3::new(0.25 * w_thick * sh_w, 0.0, 0.0),
            parent_name: "Chest".to_string(),
        },
        RagdollNode {
            name: "R_Elbow".to_string(),
            position: Vec3::new(0.5 * w_thick * sh_w, chest_y, 0.0),
            old_position: Vec3::new(0.5 * w_thick * sh_w, chest_y, 0.0),
            radius: arm_rad * 0.9,
            offset_from_parent: Vec3::new(0.25 * w_thick * sh_w, 0.0, 0.0),
            parent_name: "R_Shoulder".to_string(),
        },
        // Legs
        RagdollNode {
            name: "L_Hip".to_string(),
            position: Vec3::new(-0.16 * w_thick * waist, pelvis_y, 0.0),
            old_position: Vec3::new(-0.16 * w_thick * waist, pelvis_y, 0.0),
            radius: leg_rad,
            offset_from_parent: Vec3::new(-0.16 * w_thick * waist, 0.0, 0.0),
            parent_name: "Pelvis".to_string(),
        },
        RagdollNode {
            name: "L_Knee".to_string(),
            position: Vec3::new(-0.16 * w_thick * waist, knee_y, 0.0),
            old_position: Vec3::new(-0.16 * w_thick * waist, knee_y, 0.0),
            radius: leg_rad * 0.9,
            offset_from_parent: Vec3::new(0.0, -knee_y, 0.0),
            parent_name: "L_Hip".to_string(),
        },
        RagdollNode {
            name: "L_Foot".to_string(),
            position: Vec3::new(-0.16 * w_thick * waist, 0.0, 0.0),
            old_position: Vec3::new(-0.16 * w_thick * waist, 0.0, 0.0),
            radius: leg_rad * 0.8,
            offset_from_parent: Vec3::new(0.0, -knee_y, 0.0),
            parent_name: "L_Knee".to_string(),
        },
        RagdollNode {
            name: "R_Hip".to_string(),
            position: Vec3::new(0.16 * w_thick * waist, pelvis_y, 0.0),
            old_position: Vec3::new(0.16 * w_thick * waist, pelvis_y, 0.0),
            radius: leg_rad,
            offset_from_parent: Vec3::new(0.16 * w_thick * waist, 0.0, 0.0),
            parent_name: "Pelvis".to_string(),
        },
        RagdollNode {
            name: "R_Knee".to_string(),
            position: Vec3::new(0.16 * w_thick * waist, knee_y, 0.0),
            old_position: Vec3::new(0.16 * w_thick * waist, knee_y, 0.0),
            radius: leg_rad * 0.9,
            offset_from_parent: Vec3::new(0.0, -knee_y, 0.0),
            parent_name: "R_Hip".to_string(),
        },
        RagdollNode {
            name: "R_Foot".to_string(),
            position: Vec3::new(0.16 * w_thick * waist, 0.0, 0.0),
            old_position: Vec3::new(0.16 * w_thick * waist, 0.0, 0.0),
            radius: leg_rad * 0.8,
            offset_from_parent: Vec3::new(0.0, -knee_y, 0.0),
            parent_name: "R_Knee".to_string(),
        },
    ];

    let mut constraints = Vec::new();
    let node_map: rustc_hash::FxHashMap<String, Vec3> =
        nodes.iter().map(|n| (n.name.clone(), n.position)).collect();

    let connections = vec![
        ("Pelvis", "Spine"),
        ("Spine", "Chest"),
        ("Chest", "Head"),
        ("Chest", "L_Shoulder"),
        ("L_Shoulder", "L_Elbow"),
        ("Chest", "R_Shoulder"),
        ("R_Shoulder", "R_Elbow"),
        ("Pelvis", "L_Hip"),
        ("L_Hip", "L_Knee"),
        ("L_Knee", "L_Foot"),
        ("Pelvis", "R_Hip"),
        ("R_Hip", "R_Knee"),
        ("R_Knee", "R_Foot"),
        ("L_Shoulder", "R_Shoulder"),
        ("L_Hip", "R_Hip"),
    ];

    for (a, b) in connections {
        let len = node_map[a].distance(node_map[b]);
        constraints.push(RagdollConstraint {
            node_a: a.to_string(),
            node_b: b.to_string(),
            rest_length: len,
        });
    }

    physics.nodes = nodes;
    physics.constraints = constraints;
}

pub fn build_stylized_bone_mesh(name: &str, radius: f32) -> Mesh {
    // Generate a subdivided lat-long sphere grid
    let sectors = 12;
    let stacks = 10;

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    let mut raw_vertices = Vec::new();

    for i in 0..=stacks {
        let phi = std::f32::consts::PI * (i as f32 / stacks as f32);
        for j in 0..=sectors {
            let theta = std::f32::consts::TAU * (j as f32 / sectors as f32);

            let sin_phi = phi.sin();
            let cos_phi = phi.cos();
            let sin_theta = theta.sin();
            let cos_theta = theta.cos();

            let base_pos = Vec3::new(sin_phi * cos_theta, cos_phi, sin_phi * sin_theta);
            let mut pos = base_pos * radius;

            // Stylize based on node name
            if name == "Head" {
                // Skull: taper jaw area (y < 0.0)
                if pos.y < 0.0 {
                    pos.x *= 0.78;
                    pos.z *= 0.85;
                }
                // Broaden forehead (y > 0.1)
                if pos.y > 0.1 {
                    pos.x *= 1.08;
                    pos.z *= 1.05;
                }
                // Flatten back of skull (z < 0.0)
                if pos.z < 0.0 {
                    pos.z *= 0.85;
                }
            } else if name == "Chest" {
                // Ribcage shape: ringed horizontal bands (ribs)
                let scale = 1.0 + (pos.y * 18.0).sin().abs() * 0.07;
                pos.x *= scale * 1.15; // slightly wider
                pos.z *= scale * 0.95;
            } else if name == "Pelvis" {
                // Pelvis bone: wider, flared hips (x), thin profile (z)
                pos.x *= 1.35;
                if pos.y > 0.0 {
                    pos.x *= 1.15; // flared wings
                }
                pos.z *= 0.65; // flattened pelvic girdle
            } else {
                // Knobby joint: bulge the center, squeeze the attachments slightly
                let bulge = 1.0 + (pos.y * std::f32::consts::PI).cos().abs() * 0.12;
                pos *= bulge;
            }

            raw_vertices.push(pos);
        }
    }

    // Build triangles with flat shading & vertex coloring (ivory bone color)
    let bone_color = [0.92, 0.90, 0.82, 1.0];
    let socket_color = [0.15, 0.15, 0.15, 1.0];

    for i in 0..stacks {
        for j in 0..sectors {
            let p00 = raw_vertices[i * (sectors + 1) + j];
            let p01 = raw_vertices[i * (sectors + 1) + j + 1];
            let p10 = raw_vertices[(i + 1) * (sectors + 1) + j];
            let p11 = raw_vertices[(i + 1) * (sectors + 1) + j + 1];

            // Winding order for outward facing triangles
            // Check if vertex lies inside skull eye sockets
            let is_socket = |p: Vec3| -> bool {
                name == "Head"
                    && p.z > 0.3 * radius
                    && p.y > 0.0
                    && p.y < 0.3 * radius
                    && p.x.abs() > 0.15 * radius
                    && p.x.abs() < 0.55 * radius
            };

            let color0 = if is_socket(p00) {
                socket_color
            } else {
                bone_color
            };
            let color1 = if is_socket(p01) {
                socket_color
            } else {
                bone_color
            };
            let _color2 = if is_socket(p10) {
                socket_color
            } else {
                bone_color
            };
            let _color3 = if is_socket(p11) {
                socket_color
            } else {
                bone_color
            };

            // Triangle 1
            add_skeletal_triangle(
                p00,
                p01,
                p10,
                color0,
                &mut positions,
                &mut normals,
                &mut colors,
                &mut indices,
            );
            // Triangle 2
            add_skeletal_triangle(
                p01,
                p11,
                p10,
                color1,
                &mut positions,
                &mut normals,
                &mut colors,
                &mut indices,
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

pub fn build_skeletal_limb_mesh() -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    // Bone colors
    let bone_color = [0.92, 0.90, 0.82, 1.0];

    // 1. Generate the Bone Shaft (central flared bone)
    let sectors = 8;
    let stacks = 5;

    let mut bone_vertices = Vec::new();
    for i in 0..=stacks {
        let y = -0.5 + (i as f32 / stacks as f32);
        // Flare out at the ends: radius is slender (0.35) in the middle, and thick (0.75) at the joint endings
        let rad = 0.35 + 0.40 * (y * 2.0).powi(2);

        for j in 0..=sectors {
            let theta = std::f32::consts::TAU * (j as f32 / sectors as f32);
            let pos = Vec3::new(theta.cos() * rad, y, theta.sin() * rad);
            bone_vertices.push(pos);
        }
    }

    // Add bone triangles
    for i in 0..stacks {
        for j in 0..sectors {
            let p00 = bone_vertices[i * (sectors + 1) + j];
            let p01 = bone_vertices[i * (sectors + 1) + j + 1];
            let p10 = bone_vertices[(i + 1) * (sectors + 1) + j];
            let p11 = bone_vertices[(i + 1) * (sectors + 1) + j + 1];

            add_skeletal_triangle(
                p00,
                p01,
                p10,
                bone_color,
                &mut positions,
                &mut normals,
                &mut colors,
                &mut indices,
            );
            add_skeletal_triangle(
                p01,
                p11,
                p10,
                bone_color,
                &mut positions,
                &mut normals,
                &mut colors,
                &mut indices,
            );
        }
    }

    // 2. Generate 3 crimson/pink muscle strands with white tendons
    let muscle_color = [0.75, 0.15, 0.20, 1.0];
    let tendon_color = [0.88, 0.88, 0.90, 1.0];

    for m in 0..3 {
        let angle = std::f32::consts::TAU * (m as f32 / 3.0);
        let cos_a = angle.cos();
        let sin_a = angle.sin();

        let tube_sectors = 6;
        let tube_stacks = 6;

        let mut tube_vertices = Vec::new();

        for i in 0..=tube_stacks {
            let t = i as f32 / tube_stacks as f32;
            let y = -0.45 + t * 0.90; // spans most of the limb length

            // Muscle bulges in the center (t = 0.5), and attaches at the ends (t = 0 or 1)
            let bulge = (t * std::f32::consts::PI).sin();
            let rad = 0.08 + bulge * 0.14; // slender near attachment, bulging in center

            // Shift the center of the tube outward radially from the bone shaft
            let radial_offset = 0.45 + bulge * 0.15;
            let center = Vec3::new(cos_a * radial_offset, y, sin_a * radial_offset);

            for j in 0..=tube_sectors {
                let u = std::f32::consts::TAU * (j as f32 / tube_sectors as f32);
                let offset = Vec3::new(u.cos() * rad, 0.0, u.sin() * rad);
                tube_vertices.push(center + offset);
            }
        }

        // Add tube triangles
        for i in 0..tube_stacks {
            let t_mid = (i as f32 + 0.5) / tube_stacks as f32;
            let col = if !(0.22..=0.78).contains(&t_mid) {
                tendon_color
            } else {
                muscle_color
            };

            for j in 0..tube_sectors {
                let p00 = tube_vertices[i * (tube_sectors + 1) + j];
                let p01 = tube_vertices[i * (tube_sectors + 1) + j + 1];
                let p10 = tube_vertices[(i + 1) * (tube_sectors + 1) + j];
                let p11 = tube_vertices[(i + 1) * (tube_sectors + 1) + j + 1];

                add_skeletal_triangle(
                    p00,
                    p01,
                    p10,
                    col,
                    &mut positions,
                    &mut normals,
                    &mut colors,
                    &mut indices,
                );
                add_skeletal_triangle(
                    p01,
                    p11,
                    p10,
                    col,
                    &mut positions,
                    &mut normals,
                    &mut colors,
                    &mut indices,
                );
            }
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
fn add_skeletal_triangle(
    p0: Vec3,
    p1: Vec3,
    p2: Vec3,
    color: [f32; 4],
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
) {
    let edge1 = p1 - p0;
    let edge2 = p2 - p0;
    let normal = edge1.cross(edge2).normalize_or_zero();

    let start_idx = positions.len() as u32;
    positions.push(p0.to_array());
    positions.push(p1.to_array());
    positions.push(p2.to_array());

    normals.push(normal.to_array());
    normals.push(normal.to_array());
    normals.push(normal.to_array());

    colors.push(color);
    colors.push(color);
    colors.push(color);

    indices.push(start_idx);
    indices.push(start_idx + 1);
    indices.push(start_idx + 2);
}

pub fn spawn_character_visuals(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    settings: &CharacterSettings,
    physics: &RagdollPhysics,
) {
    // Skin & Basic Materials
    let skin_mat = materials.add(StandardMaterial {
        base_color: settings.skin_color,
        perceptual_roughness: 0.65,
        ..default()
    });

    // 1. Sci-Fi Suit Materials
    let scifi_suit_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.16, 0.24), // Slate suit
        metallic: 0.75,
        perceptual_roughness: 0.25,
        ..default()
    });
    let scifi_visor_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 0.9, 1.0),
        emissive: LinearRgba::new(0.0, 6.0, 12.0, 1.0),
        unlit: true,
        ..default()
    });
    let scifi_core_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.45, 0.0),
        emissive: LinearRgba::new(12.0, 5.0, 0.0, 1.0),
        unlit: true,
        ..default()
    });
    let scifi_trim_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.65, 0.15), // Gold trim
        metallic: 0.9,
        perceptual_roughness: 0.2,
        ..default()
    });

    // 2. Tactical Armor Materials
    let tac_vest_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.16, 0.20, 0.16), // Military olive
        perceptual_roughness: 0.8,
        ..default()
    });
    let tac_camo_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.10, 0.12, 0.15), // Tactical dark grey
        perceptual_roughness: 0.85,
        ..default()
    });
    let tac_nvg_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 1.0, 0.4),
        emissive: LinearRgba::new(0.0, 8.0, 2.0, 1.0),
        unlit: true,
        ..default()
    });
    let tac_plate_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.06, 0.06, 0.07),
        metallic: 0.3,
        perceptual_roughness: 0.4,
        ..default()
    });

    // 3. Stylized Hero Materials
    let hero_jacket_mat = materials.add(StandardMaterial {
        base_color: if settings.gender == Gender::Male {
            Color::srgb(0.75, 0.18, 0.12)
        } else {
            Color::srgb(0.12, 0.52, 0.75)
        },
        perceptual_roughness: 0.5,
        ..default()
    });
    let hero_pants_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.08, 0.12, 0.22),
        perceptual_roughness: 0.7,
        ..default()
    });
    let hero_boots_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.10, 0.07),
        perceptual_roughness: 0.6,
        ..default()
    });
    let eye_white_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.1,
        ..default()
    });
    let eye_pupil_mat = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        perceptual_roughness: 0.1,
        ..default()
    });

    // 4. Skeleton Exo Frame Materials
    let exo_glass_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.1, 0.65, 0.95, 0.35),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.15,
        ..default()
    });

    // 5. Classic Materials
    let shirt_mat = materials.add(StandardMaterial {
        base_color: if settings.gender == Gender::Male {
            Color::srgb(0.15, 0.45, 0.75)
        } else {
            Color::srgb(0.85, 0.25, 0.55)
        },
        perceptual_roughness: 0.55,
        ..default()
    });
    let pants_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.15, 0.22),
        perceptual_roughness: 0.75,
        ..default()
    });
    let eye_mat = materials.add(StandardMaterial {
        base_color: settings.eye_color,
        perceptual_roughness: 0.1,
        ..default()
    });
    let hair_mat = materials.add(StandardMaterial {
        base_color: settings.hair_color,
        perceptual_roughness: 0.85,
        ..default()
    });
    let bone_mat = materials.add(StandardMaterial {
        base_color: match settings.outfit_style {
            OutfitStyle::SkeletonExoFrame => Color::srgb(0.2, 0.9, 1.0),
            _ => Color::WHITE,
        },
        emissive: match settings.outfit_style {
            OutfitStyle::SkeletonExoFrame => LinearRgba::new(2.5, 7.0, 10.0, 1.0),
            _ => LinearRgba::BLACK,
        },
        unlit: settings.outfit_style == OutfitStyle::SkeletonExoFrame,
        perceptual_roughness: 0.85,
        ..default()
    });

    for node in physics.nodes.iter() {
        let is_head = node.name == "Head";
        let is_torso = node.name == "Pelvis" || node.name == "Spine" || node.name == "Chest";
        let is_pants_area = node.name == "Pelvis" || node.name == "L_Hip" || node.name == "R_Hip";
        let is_foot = node.name == "L_Foot" || node.name == "R_Foot";

        let skin_mat_to_use = match settings.outfit_style {
            OutfitStyle::SciFiSuit => {
                if is_head || is_torso || is_pants_area {
                    scifi_suit_mat.clone()
                } else if is_foot {
                    scifi_trim_mat.clone()
                } else {
                    scifi_suit_mat.clone()
                }
            }
            OutfitStyle::TacticalArmor => {
                if is_torso || is_pants_area {
                    tac_vest_mat.clone()
                } else if is_foot {
                    tac_plate_mat.clone()
                } else {
                    tac_camo_mat.clone()
                }
            }
            OutfitStyle::StylizedHero => {
                if is_head {
                    skin_mat.clone()
                } else if is_pants_area {
                    hero_pants_mat.clone()
                } else if is_torso {
                    hero_jacket_mat.clone()
                } else if is_foot {
                    hero_boots_mat.clone()
                } else {
                    skin_mat.clone()
                }
            }
            OutfitStyle::SkeletonExoFrame => exo_glass_mat.clone(),
            OutfitStyle::ClassicMannequin => {
                if is_head {
                    skin_mat.clone()
                } else if is_pants_area {
                    pants_mat.clone()
                } else if is_torso {
                    shirt_mat.clone()
                } else {
                    skin_mat.clone()
                }
            }
        };

        let mesh_radius = node.radius;

        // 1. Spawn Solid Outer Skin/Clothing Joint Sphere
        let outer_sphere = meshes.add(Sphere::new(mesh_radius).mesh().ico(4).unwrap());
        let outer_node_entity = commands
            .spawn((
                Mesh3d(outer_sphere),
                MeshMaterial3d(skin_mat_to_use),
                Transform::from_translation(node.position),
                BoneVisual {
                    name: node.name.clone(),
                },
                XraySkinVisual,
                CharacterVisualEntity,
                CharacterModelPart,
            ))
            .id();

        // 2. Spawn Inner Skeleton Joint
        let bone_mesh = build_stylized_bone_mesh(&node.name, mesh_radius);
        commands.spawn((
            Mesh3d(meshes.add(bone_mesh)),
            MeshMaterial3d(bone_mat.clone()),
            Transform::from_translation(node.position),
            BoneVisual {
                name: node.name.clone(),
            },
            XraySkeletonVisual,
            CharacterVisualEntity,
            CharacterModelPart,
        ));

        // Node accessories based on OutfitStyle & Node Name
        match settings.outfit_style {
            OutfitStyle::SciFiSuit => {
                if is_head {
                    // Cyber Visor Bar across face
                    let visor_mesh = meshes.add(Cuboid::new(
                        mesh_radius * 1.5,
                        mesh_radius * 0.35,
                        mesh_radius * 0.45,
                    ));
                    let visor = commands
                        .spawn((
                            Mesh3d(visor_mesh),
                            MeshMaterial3d(scifi_visor_mat.clone()),
                            Transform::from_translation(Vec3::new(
                                0.0,
                                mesh_radius * 0.1,
                                mesh_radius * 0.75,
                            )),
                            CharacterVisualEntity,
                            CharacterModelPart,
                        ))
                        .id();
                    commands.entity(outer_node_entity).add_child(visor);

                    // Helmet Gold Ear Comm Pads
                    let comm_mesh =
                        meshes.add(Cylinder::new(mesh_radius * 0.22, mesh_radius * 0.15));
                    for dir in &[-1.0f32, 1.0f32] {
                        let comm = commands
                            .spawn((
                                Mesh3d(comm_mesh.clone()),
                                MeshMaterial3d(scifi_trim_mat.clone()),
                                Transform::from_translation(Vec3::new(
                                    dir * mesh_radius * 0.95,
                                    mesh_radius * 0.1,
                                    0.0,
                                ))
                                .with_rotation(Quat::from_rotation_z(dir * 1.57)),
                                CharacterVisualEntity,
                                CharacterModelPart,
                            ))
                            .id();
                        commands.entity(outer_node_entity).add_child(comm);
                    }
                } else if node.name == "Chest" {
                    // Arc Reactor Chest Core
                    let core_mesh =
                        meshes.add(Sphere::new(mesh_radius * 0.42).mesh().ico(3).unwrap());
                    let core = commands
                        .spawn((
                            Mesh3d(core_mesh),
                            MeshMaterial3d(scifi_core_mat.clone()),
                            Transform::from_translation(Vec3::new(
                                0.0,
                                mesh_radius * 0.2,
                                mesh_radius * 0.85,
                            )),
                            CharacterVisualEntity,
                            CharacterModelPart,
                        ))
                        .id();
                    commands.entity(outer_node_entity).add_child(core);

                    // Gold Chest Armor Collar
                    let armor_plate = meshes.add(Cuboid::new(
                        mesh_radius * 1.8,
                        mesh_radius * 0.4,
                        mesh_radius * 0.6,
                    ));
                    let plate = commands
                        .spawn((
                            Mesh3d(armor_plate),
                            MeshMaterial3d(scifi_trim_mat.clone()),
                            Transform::from_translation(Vec3::new(
                                0.0,
                                mesh_radius * 0.6,
                                mesh_radius * 0.5,
                            )),
                            CharacterVisualEntity,
                            CharacterModelPart,
                        ))
                        .id();
                    commands.entity(outer_node_entity).add_child(plate);
                } else if node.name == "L_Shoulder" || node.name == "R_Shoulder" {
                    // Shoulder Pauldron Guard
                    let pauldron_mesh =
                        meshes.add(Sphere::new(mesh_radius * 1.25).mesh().ico(3).unwrap());
                    let pauldron = commands
                        .spawn((
                            Mesh3d(pauldron_mesh),
                            MeshMaterial3d(scifi_trim_mat.clone()),
                            Transform::from_translation(Vec3::new(0.0, mesh_radius * 0.2, 0.0)),
                            CharacterVisualEntity,
                            CharacterModelPart,
                        ))
                        .id();
                    commands.entity(outer_node_entity).add_child(pauldron);
                } else if is_foot {
                    // Magnetized Boots
                    let boot_mesh = meshes.add(Cuboid::new(
                        mesh_radius * 1.1,
                        mesh_radius * 0.6,
                        mesh_radius * 1.6,
                    ));
                    let boot = commands
                        .spawn((
                            Mesh3d(boot_mesh),
                            MeshMaterial3d(scifi_trim_mat.clone()),
                            Transform::from_translation(Vec3::new(
                                0.0,
                                -mesh_radius * 0.2,
                                mesh_radius * 0.3,
                            )),
                            CharacterVisualEntity,
                            CharacterModelPart,
                        ))
                        .id();
                    commands.entity(outer_node_entity).add_child(boot);
                }
            }
            OutfitStyle::TacticalArmor => {
                if is_head {
                    // Military Helmet Brim
                    let brim_mesh = meshes.add(Cuboid::new(
                        mesh_radius * 1.8,
                        mesh_radius * 0.15,
                        mesh_radius * 1.8,
                    ));
                    let brim = commands
                        .spawn((
                            Mesh3d(brim_mesh),
                            MeshMaterial3d(tac_vest_mat.clone()),
                            Transform::from_translation(Vec3::new(0.0, mesh_radius * 0.45, 0.0)),
                            CharacterVisualEntity,
                            CharacterModelPart,
                        ))
                        .id();
                    commands.entity(outer_node_entity).add_child(brim);

                    // Dual NVG Night Vision Goggles
                    let nvg_mesh = meshes.add(Cylinder::new(mesh_radius * 0.18, mesh_radius * 0.4));
                    for dir in &[-0.32f32, 0.32f32] {
                        let nvg = commands
                            .spawn((
                                Mesh3d(nvg_mesh.clone()),
                                MeshMaterial3d(tac_nvg_mat.clone()),
                                Transform::from_translation(Vec3::new(
                                    dir * mesh_radius,
                                    mesh_radius * 0.25,
                                    mesh_radius * 0.85,
                                ))
                                .with_rotation(Quat::from_rotation_x(1.57)),
                                CharacterVisualEntity,
                                CharacterModelPart,
                            ))
                            .id();
                        commands.entity(outer_node_entity).add_child(nvg);
                    }

                    // Respirator Face Mask
                    let mask_mesh = meshes.add(Cuboid::new(
                        mesh_radius * 0.95,
                        mesh_radius * 0.45,
                        mesh_radius * 0.55,
                    ));
                    let mask = commands
                        .spawn((
                            Mesh3d(mask_mesh),
                            MeshMaterial3d(tac_plate_mat.clone()),
                            Transform::from_translation(Vec3::new(
                                0.0,
                                -mesh_radius * 0.2,
                                mesh_radius * 0.7,
                            )),
                            CharacterVisualEntity,
                            CharacterModelPart,
                        ))
                        .id();
                    commands.entity(outer_node_entity).add_child(mask);
                } else if node.name == "Chest" {
                    // Tactical Kevlar Vest with pouches
                    let vest_mesh = meshes.add(Cuboid::new(
                        mesh_radius * 1.8,
                        mesh_radius * 1.4,
                        mesh_radius * 0.6,
                    ));
                    let vest = commands
                        .spawn((
                            Mesh3d(vest_mesh),
                            MeshMaterial3d(tac_vest_mat.clone()),
                            Transform::from_translation(Vec3::new(0.0, 0.0, mesh_radius * 0.2)),
                            CharacterVisualEntity,
                            CharacterModelPart,
                        ))
                        .id();
                    commands.entity(outer_node_entity).add_child(vest);

                    // Ammo Mag Pouches
                    let pouch_mesh = meshes.add(Cuboid::new(
                        mesh_radius * 0.35,
                        mesh_radius * 0.5,
                        mesh_radius * 0.25,
                    ));
                    for px in &[-0.5f32, 0.0f32, 0.5f32] {
                        let pouch = commands
                            .spawn((
                                Mesh3d(pouch_mesh.clone()),
                                MeshMaterial3d(tac_plate_mat.clone()),
                                Transform::from_translation(Vec3::new(
                                    px * mesh_radius,
                                    -mesh_radius * 0.2,
                                    mesh_radius * 0.8,
                                )),
                                CharacterVisualEntity,
                                CharacterModelPart,
                            ))
                            .id();
                        commands.entity(outer_node_entity).add_child(pouch);
                    }
                } else if node.name == "L_Knee"
                    || node.name == "R_Knee"
                    || node.name == "L_Elbow"
                    || node.name == "R_Elbow"
                {
                    // Molded Hard-Shell Joint Guards
                    let pad_mesh =
                        meshes.add(Sphere::new(mesh_radius * 1.15).mesh().ico(3).unwrap());
                    let pad = commands
                        .spawn((
                            Mesh3d(pad_mesh),
                            MeshMaterial3d(tac_plate_mat.clone()),
                            Transform::from_translation(Vec3::new(0.0, 0.0, mesh_radius * 0.2)),
                            CharacterVisualEntity,
                            CharacterModelPart,
                        ))
                        .id();
                    commands.entity(outer_node_entity).add_child(pad);
                } else if is_foot {
                    // Tactical Combat Boots
                    let boot_mesh = meshes.add(Cuboid::new(
                        mesh_radius * 1.1,
                        mesh_radius * 0.7,
                        mesh_radius * 1.6,
                    ));
                    let boot = commands
                        .spawn((
                            Mesh3d(boot_mesh),
                            MeshMaterial3d(tac_plate_mat.clone()),
                            Transform::from_translation(Vec3::new(
                                0.0,
                                -mesh_radius * 0.2,
                                mesh_radius * 0.3,
                            )),
                            CharacterVisualEntity,
                            CharacterModelPart,
                        ))
                        .id();
                    commands.entity(outer_node_entity).add_child(boot);
                }
            }
            OutfitStyle::StylizedHero => {
                if is_head {
                    // Eye sockets + White Sclera + Iris + Pupil
                    let eye_white_mesh =
                        meshes.add(Sphere::new(mesh_radius * 0.22).mesh().ico(3).unwrap());
                    let eye_iris_mesh =
                        meshes.add(Sphere::new(mesh_radius * 0.13).mesh().ico(3).unwrap());
                    let pupil_mesh =
                        meshes.add(Sphere::new(mesh_radius * 0.06).mesh().ico(3).unwrap());
                    let eyebrow_mesh = meshes.add(Cuboid::new(
                        mesh_radius * 0.35,
                        mesh_radius * 0.05,
                        mesh_radius * 0.08,
                    ));

                    for (side, offset_x) in
                        [(-1.0f32, -mesh_radius * 0.35), (1.0f32, mesh_radius * 0.35)]
                    {
                        let ew = commands
                            .spawn((
                                Mesh3d(eye_white_mesh.clone()),
                                MeshMaterial3d(eye_white_mat.clone()),
                                Transform::from_translation(Vec3::new(
                                    offset_x,
                                    mesh_radius * 0.15,
                                    mesh_radius * 0.85,
                                )),
                                CharacterVisualEntity,
                                CharacterModelPart,
                            ))
                            .id();
                        commands.entity(outer_node_entity).add_child(ew);

                        let ei = commands
                            .spawn((
                                Mesh3d(eye_iris_mesh.clone()),
                                MeshMaterial3d(eye_mat.clone()),
                                Transform::from_translation(Vec3::new(
                                    offset_x,
                                    mesh_radius * 0.15,
                                    mesh_radius * 0.96,
                                )),
                                CharacterVisualEntity,
                                CharacterModelPart,
                            ))
                            .id();
                        commands.entity(outer_node_entity).add_child(ei);

                        let ep = commands
                            .spawn((
                                Mesh3d(pupil_mesh.clone()),
                                MeshMaterial3d(eye_pupil_mat.clone()),
                                Transform::from_translation(Vec3::new(
                                    offset_x,
                                    mesh_radius * 0.15,
                                    mesh_radius * 1.02,
                                )),
                                CharacterVisualEntity,
                                CharacterModelPart,
                            ))
                            .id();
                        commands.entity(outer_node_entity).add_child(ep);

                        let eb = commands
                            .spawn((
                                Mesh3d(eyebrow_mesh.clone()),
                                MeshMaterial3d(hair_mat.clone()),
                                Transform::from_translation(Vec3::new(
                                    offset_x,
                                    mesh_radius * 0.38,
                                    mesh_radius * 0.85,
                                ))
                                .with_rotation(Quat::from_rotation_z(side * -0.15)),
                                CharacterVisualEntity,
                                CharacterModelPart,
                            ))
                            .id();
                        commands.entity(outer_node_entity).add_child(eb);
                    }

                    // Nose / Jaw structure
                    let nose_mesh = meshes.add(Cuboid::new(
                        mesh_radius * 0.12,
                        mesh_radius * 0.25,
                        mesh_radius * 0.25,
                    ));
                    let nose = commands
                        .spawn((
                            Mesh3d(nose_mesh),
                            MeshMaterial3d(skin_mat.clone()),
                            Transform::from_translation(Vec3::new(
                                0.0,
                                -mesh_radius * 0.05,
                                mesh_radius * 0.95,
                            )),
                            CharacterVisualEntity,
                            CharacterModelPart,
                        ))
                        .id();
                    commands.entity(outer_node_entity).add_child(nose);

                    // Hair mesh
                    match settings.hair_style {
                        HairStyle::None => {}
                        HairStyle::Short => {
                            let cap_mesh =
                                meshes.add(Sphere::new(mesh_radius * 1.04).mesh().ico(4).unwrap());
                            let cap = commands
                                .spawn((
                                    Mesh3d(cap_mesh),
                                    MeshMaterial3d(hair_mat.clone()),
                                    Transform::from_translation(Vec3::new(
                                        0.0,
                                        mesh_radius * 0.25,
                                        -mesh_radius * 0.1,
                                    )),
                                    CharacterVisualEntity,
                                    CharacterModelPart,
                                ))
                                .id();
                            commands.entity(outer_node_entity).add_child(cap);
                        }
                        HairStyle::Ponytail => {
                            let cap_mesh =
                                meshes.add(Sphere::new(mesh_radius * 1.03).mesh().ico(4).unwrap());
                            let cap = commands
                                .spawn((
                                    Mesh3d(cap_mesh),
                                    MeshMaterial3d(hair_mat.clone()),
                                    Transform::from_translation(Vec3::new(
                                        0.0,
                                        mesh_radius * 0.33,
                                        -mesh_radius * 0.15,
                                    )),
                                    CharacterVisualEntity,
                                    CharacterModelPart,
                                ))
                                .id();
                            commands.entity(outer_node_entity).add_child(cap);
                        }
                        HairStyle::Spiky => {
                            let cap_mesh =
                                meshes.add(Sphere::new(mesh_radius * 1.02).mesh().ico(4).unwrap());
                            let cap = commands
                                .spawn((
                                    Mesh3d(cap_mesh),
                                    MeshMaterial3d(hair_mat.clone()),
                                    Transform::from_translation(Vec3::new(
                                        0.0,
                                        mesh_radius * 0.33,
                                        -mesh_radius * 0.15,
                                    )),
                                    CharacterVisualEntity,
                                    CharacterModelPart,
                                ))
                                .id();
                            commands.entity(outer_node_entity).add_child(cap);
                        }
                        HairStyle::Curly => {
                            let cap_mesh =
                                meshes.add(Sphere::new(mesh_radius * 1.02).mesh().ico(4).unwrap());
                            let cap = commands
                                .spawn((
                                    Mesh3d(cap_mesh),
                                    MeshMaterial3d(hair_mat.clone()),
                                    Transform::from_translation(Vec3::new(
                                        0.0,
                                        mesh_radius * 0.33,
                                        -mesh_radius * 0.15,
                                    )),
                                    CharacterVisualEntity,
                                    CharacterModelPart,
                                ))
                                .id();
                            commands.entity(outer_node_entity).add_child(cap);
                        }
                    }
                } else if node.name == "Chest" {
                    // Hero Jacket Lapel Collar
                    let lapel_mesh = meshes.add(Cuboid::new(
                        mesh_radius * 1.6,
                        mesh_radius * 0.4,
                        mesh_radius * 0.7,
                    ));
                    let lapel = commands
                        .spawn((
                            Mesh3d(lapel_mesh),
                            MeshMaterial3d(hero_jacket_mat.clone()),
                            Transform::from_translation(Vec3::new(
                                0.0,
                                mesh_radius * 0.5,
                                mesh_radius * 0.3,
                            )),
                            CharacterVisualEntity,
                            CharacterModelPart,
                        ))
                        .id();
                    commands.entity(outer_node_entity).add_child(lapel);
                } else if is_foot {
                    // Leather Boots
                    let boot_mesh = meshes.add(Cuboid::new(
                        mesh_radius * 1.0,
                        mesh_radius * 0.6,
                        mesh_radius * 1.5,
                    ));
                    let boot = commands
                        .spawn((
                            Mesh3d(boot_mesh),
                            MeshMaterial3d(hero_boots_mat.clone()),
                            Transform::from_translation(Vec3::new(
                                0.0,
                                -mesh_radius * 0.2,
                                mesh_radius * 0.3,
                            )),
                            CharacterVisualEntity,
                            CharacterModelPart,
                        ))
                        .id();
                    commands.entity(outer_node_entity).add_child(boot);
                }
            }
            OutfitStyle::SkeletonExoFrame => {
                // Outer skin is translucent glass shell, skeleton is glowing neon cyan
            }
            OutfitStyle::ClassicMannequin => {
                if is_head {
                    let eye_mesh =
                        meshes.add(Sphere::new(mesh_radius * 0.2).mesh().ico(3).unwrap());
                    for offset_x in [-mesh_radius * 0.35, mesh_radius * 0.35] {
                        let e = commands
                            .spawn((
                                Mesh3d(eye_mesh.clone()),
                                MeshMaterial3d(eye_mat.clone()),
                                Transform::from_translation(Vec3::new(
                                    offset_x,
                                    mesh_radius * 0.15,
                                    mesh_radius * 0.85,
                                )),
                                CharacterVisualEntity,
                                CharacterModelPart,
                            ))
                            .id();
                        commands.entity(outer_node_entity).add_child(e);
                    }
                    if settings.hair_style != HairStyle::None {
                        let cap_mesh =
                            meshes.add(Sphere::new(mesh_radius * 1.03).mesh().ico(4).unwrap());
                        let cap = commands
                            .spawn((
                                Mesh3d(cap_mesh),
                                MeshMaterial3d(hair_mat.clone()),
                                Transform::from_translation(Vec3::new(
                                    0.0,
                                    mesh_radius * 0.25,
                                    -mesh_radius * 0.1,
                                )),
                                CharacterVisualEntity,
                                CharacterModelPart,
                            ))
                            .id();
                        commands.entity(outer_node_entity).add_child(cap);
                    }
                }
            }
        }
    }

    // Outer Cylinder Limbs
    let cylinder_mesh = meshes.add(Cylinder::new(1.0, 1.0));
    let skin_connections = match settings.outfit_style {
        OutfitStyle::SciFiSuit => vec![
            ("Pelvis", "Spine", scifi_suit_mat.clone(), 0.18),
            ("Spine", "Chest", scifi_suit_mat.clone(), 0.18),
            ("Chest", "Head", scifi_suit_mat.clone(), 0.12),
            ("Chest", "L_Shoulder", scifi_suit_mat.clone(), 0.12),
            ("L_Shoulder", "L_Elbow", scifi_suit_mat.clone(), 0.08),
            ("Chest", "R_Shoulder", scifi_suit_mat.clone(), 0.12),
            ("R_Shoulder", "R_Elbow", scifi_suit_mat.clone(), 0.08),
            ("Pelvis", "L_Hip", scifi_suit_mat.clone(), 0.14),
            ("L_Hip", "L_Knee", scifi_suit_mat.clone(), 0.10),
            ("L_Knee", "L_Foot", scifi_suit_mat.clone(), 0.08),
            ("Pelvis", "R_Hip", scifi_suit_mat.clone(), 0.14),
            ("R_Hip", "R_Knee", scifi_suit_mat.clone(), 0.10),
            ("R_Knee", "R_Foot", scifi_suit_mat.clone(), 0.08),
        ],
        OutfitStyle::TacticalArmor => vec![
            ("Pelvis", "Spine", tac_vest_mat.clone(), 0.18),
            ("Spine", "Chest", tac_vest_mat.clone(), 0.18),
            ("Chest", "Head", tac_camo_mat.clone(), 0.12),
            ("Chest", "L_Shoulder", tac_camo_mat.clone(), 0.12),
            ("L_Shoulder", "L_Elbow", tac_camo_mat.clone(), 0.08),
            ("Chest", "R_Shoulder", tac_camo_mat.clone(), 0.12),
            ("R_Shoulder", "R_Elbow", tac_camo_mat.clone(), 0.08),
            ("Pelvis", "L_Hip", tac_camo_mat.clone(), 0.14),
            ("L_Hip", "L_Knee", tac_camo_mat.clone(), 0.10),
            ("L_Knee", "L_Foot", tac_camo_mat.clone(), 0.08),
            ("Pelvis", "R_Hip", tac_camo_mat.clone(), 0.14),
            ("R_Hip", "R_Knee", tac_camo_mat.clone(), 0.10),
            ("R_Knee", "R_Foot", tac_camo_mat.clone(), 0.08),
        ],
        OutfitStyle::StylizedHero => vec![
            ("Pelvis", "Spine", hero_pants_mat.clone(), 0.18),
            ("Spine", "Chest", hero_jacket_mat.clone(), 0.18),
            ("Chest", "Head", skin_mat.clone(), 0.12),
            ("Chest", "L_Shoulder", hero_jacket_mat.clone(), 0.12),
            ("L_Shoulder", "L_Elbow", hero_jacket_mat.clone(), 0.08),
            ("Chest", "R_Shoulder", hero_jacket_mat.clone(), 0.12),
            ("R_Shoulder", "R_Elbow", hero_jacket_mat.clone(), 0.08),
            ("Pelvis", "L_Hip", hero_pants_mat.clone(), 0.14),
            ("L_Hip", "L_Knee", hero_pants_mat.clone(), 0.10),
            ("L_Knee", "L_Foot", hero_pants_mat.clone(), 0.08),
            ("Pelvis", "R_Hip", hero_pants_mat.clone(), 0.14),
            ("R_Hip", "R_Knee", hero_pants_mat.clone(), 0.10),
            ("R_Knee", "R_Foot", hero_pants_mat.clone(), 0.08),
        ],
        OutfitStyle::SkeletonExoFrame => vec![
            ("Pelvis", "Spine", exo_glass_mat.clone(), 0.18),
            ("Spine", "Chest", exo_glass_mat.clone(), 0.18),
            ("Chest", "Head", exo_glass_mat.clone(), 0.12),
            ("Chest", "L_Shoulder", exo_glass_mat.clone(), 0.12),
            ("L_Shoulder", "L_Elbow", exo_glass_mat.clone(), 0.08),
            ("Chest", "R_Shoulder", exo_glass_mat.clone(), 0.12),
            ("R_Shoulder", "R_Elbow", exo_glass_mat.clone(), 0.08),
            ("Pelvis", "L_Hip", exo_glass_mat.clone(), 0.14),
            ("L_Hip", "L_Knee", exo_glass_mat.clone(), 0.10),
            ("L_Knee", "L_Foot", exo_glass_mat.clone(), 0.08),
            ("Pelvis", "R_Hip", exo_glass_mat.clone(), 0.14),
            ("R_Hip", "R_Knee", exo_glass_mat.clone(), 0.10),
            ("R_Knee", "R_Foot", exo_glass_mat.clone(), 0.08),
        ],
        OutfitStyle::ClassicMannequin => vec![
            ("Pelvis", "Spine", pants_mat.clone(), 0.18),
            ("Spine", "Chest", shirt_mat.clone(), 0.18),
            ("Chest", "Head", skin_mat.clone(), 0.12),
            ("Chest", "L_Shoulder", shirt_mat.clone(), 0.12),
            ("L_Shoulder", "L_Elbow", skin_mat.clone(), 0.08),
            ("Chest", "R_Shoulder", shirt_mat.clone(), 0.12),
            ("R_Shoulder", "R_Elbow", skin_mat.clone(), 0.08),
            ("Pelvis", "L_Hip", pants_mat.clone(), 0.14),
            ("L_Hip", "L_Knee", pants_mat.clone(), 0.10),
            ("L_Knee", "L_Foot", skin_mat.clone(), 0.08),
            ("Pelvis", "R_Hip", pants_mat.clone(), 0.14),
            ("R_Hip", "R_Knee", pants_mat.clone(), 0.10),
            ("R_Knee", "R_Foot", skin_mat.clone(), 0.08),
        ],
    };

    for (na, nb, mat, rad) in skin_connections {
        commands.spawn((
            Mesh3d(cylinder_mesh.clone()),
            MeshMaterial3d(mat),
            Transform::default(),
            LimbVisual {
                node_a: na.to_string(),
                node_b: nb.to_string(),
                radius: rad,
            },
            XraySkinVisual,
            CharacterVisualEntity,
            CharacterModelPart,
        ));
    }

    // Inner Skeletal Limbs
    let skeletal_limb = build_skeletal_limb_mesh();
    let limb_connections = vec![
        ("Pelvis", "Spine", 0.16),
        ("Spine", "Chest", 0.16),
        ("Chest", "Head", 0.10),
        ("Chest", "L_Shoulder", 0.10),
        ("L_Shoulder", "L_Elbow", 0.07),
        ("Chest", "R_Shoulder", 0.10),
        ("R_Shoulder", "R_Elbow", 0.07),
        ("Pelvis", "L_Hip", 0.12),
        ("L_Hip", "L_Knee", 0.09),
        ("L_Knee", "L_Foot", 0.07),
        ("Pelvis", "R_Hip", 0.12),
        ("R_Hip", "R_Knee", 0.09),
        ("R_Knee", "R_Foot", 0.07),
    ];

    for (na, nb, rad) in limb_connections {
        commands.spawn((
            Mesh3d(meshes.add(skeletal_limb.clone())),
            MeshMaterial3d(bone_mat.clone()),
            Transform::default(),
            LimbVisual {
                node_a: na.to_string(),
                node_b: nb.to_string(),
                radius: rad,
            },
            XraySkeletonVisual,
            CharacterVisualEntity,
            CharacterModelPart,
        ));
    }
}

fn character_mesh_sync_system(
    physics: Res<RagdollPhysics>,
    mut joint_query: Query<(&mut Transform, &BoneVisual), Without<LimbVisual>>,
    mut limb_query: Query<(&mut Transform, &LimbVisual), Without<BoneVisual>>,
) {
    // Sync joint spheres
    for (mut transform, visual) in joint_query.iter_mut() {
        if let Some(node) = physics.nodes.iter().find(|n| n.name == visual.name) {
            transform.translation = node.position;
        }
    }

    // Sync connecting limb cylinders
    for (mut transform, limb) in limb_query.iter_mut() {
        let pos_a = physics
            .nodes
            .iter()
            .find(|n| n.name == limb.node_a)
            .map(|n| n.position);
        let pos_b = physics
            .nodes
            .iter()
            .find(|n| n.name == limb.node_b)
            .map(|n| n.position);

        if let (Some(a), Some(b)) = (pos_a, pos_b) {
            let delta = b - a;
            let dist = delta.length();
            if dist > 0.001 {
                let midpoint = a + delta * 0.5;
                let rotation = Quat::from_rotation_arc(Vec3::Y, delta.normalize());

                transform.translation = midpoint;
                transform.rotation = rotation;
                // Scale Y by cylinder height, X/Z by the thickness of the limb
                transform.scale = Vec3::new(limb.radius, dist, limb.radius);
            }
        }
    }
}

fn character_xray_system(
    settings: Res<CharacterSettings>,
    mut skin_query: Query<&mut Visibility, (With<XraySkinVisual>, Without<XraySkeletonVisual>)>,
    mut skeleton_query: Query<&mut Visibility, (With<XraySkeletonVisual>, Without<XraySkinVisual>)>,
) {
    if settings.show_xray {
        // Show skeleton, hide outer skin
        for mut vis in skin_query.iter_mut() {
            *vis = Visibility::Hidden;
        }
        for mut vis in skeleton_query.iter_mut() {
            *vis = Visibility::Visible;
        }
    } else {
        // Show solid outer skin, hide skeleton
        for mut vis in skin_query.iter_mut() {
            *vis = Visibility::Visible;
        }
        for mut vis in skeleton_query.iter_mut() {
            *vis = Visibility::Hidden;
        }
    }
}

fn ragdoll_physics_system(
    settings: Res<CharacterSettings>,
    mut physics: ResMut<RagdollPhysics>,
    time: Res<Time>,
) {
    if !settings.is_ragdoll_active {
        let h = settings.height;
        let w_thick = settings.weight;
        let sh_w = settings.shoulder_width;
        let leg_len = settings.leg_length;
        let waist = settings.waist_width;

        let pelvis_y = h * 0.45 * (2.0 - leg_len);
        let spine_y = pelvis_y + (h * 0.15);
        let chest_y = pelvis_y + (h * 0.3);
        let head_y = chest_y + (h * 0.18);
        let knee_y = pelvis_y * 0.5;

        for node in physics.nodes.iter_mut() {
            let target_pos = match node.name.as_str() {
                "Pelvis" => Vec3::new(0.0, pelvis_y, 0.0),
                "Spine" => Vec3::new(0.0, spine_y, 0.0),
                "Chest" => Vec3::new(0.0, chest_y, 0.0),
                "Head" => Vec3::new(0.0, head_y, 0.0),
                "L_Shoulder" => Vec3::new(-0.25 * w_thick * sh_w, chest_y, 0.0),
                "L_Elbow" => Vec3::new(-0.5 * w_thick * sh_w, chest_y, 0.0),
                "R_Shoulder" => Vec3::new(0.25 * w_thick * sh_w, chest_y, 0.0),
                "R_Elbow" => Vec3::new(0.5 * w_thick * sh_w, chest_y, 0.0),
                "L_Hip" => Vec3::new(-0.16 * w_thick * waist, pelvis_y, 0.0),
                "L_Knee" => Vec3::new(-0.16 * w_thick * waist, knee_y, 0.0),
                "L_Foot" => Vec3::new(-0.16 * w_thick * waist, 0.0, 0.0),
                "R_Hip" => Vec3::new(0.16 * w_thick * waist, pelvis_y, 0.0),
                "R_Knee" => Vec3::new(0.16 * w_thick * waist, knee_y, 0.0),
                "R_Foot" => Vec3::new(0.16 * w_thick * waist, 0.0, 0.0),
                _ => Vec3::ZERO,
            };
            node.position = target_pos;
            node.old_position = target_pos;
        }
        return;
    }

    let dt = time.delta_secs().min(0.016);
    let gravity = Vec3::new(0.0, -9.8, 0.0);

    for node in physics.nodes.iter_mut() {
        let temp = node.position;
        let vel = node.position - node.old_position;
        let drag = 0.985;
        node.position = node.position + vel * drag + gravity * dt * dt;
        node.old_position = temp;

        let ground_y = node.radius;
        if node.position.y < ground_y {
            node.position.y = ground_y;
            let mut vel_x = node.position.x - node.old_position.x;
            let mut vel_z = node.position.z - node.old_position.z;
            vel_x *= 0.5;
            vel_z *= 0.5;
            node.old_position.x = node.position.x - vel_x;
            node.old_position.z = node.position.z - vel_z;
        }
    }

    for _ in 0..10 {
        let mut changes = vec![Vec3::ZERO; physics.nodes.len()];
        let mut counts = vec![0; physics.nodes.len()];

        for constraint in physics.constraints.iter() {
            let idx_a = physics
                .nodes
                .iter()
                .position(|n| n.name == constraint.node_a)
                .unwrap();
            let idx_b = physics
                .nodes
                .iter()
                .position(|n| n.name == constraint.node_b)
                .unwrap();

            let pos_a = physics.nodes[idx_a].position;
            let pos_b = physics.nodes[idx_b].position;

            let delta = pos_b - pos_a;
            let dist = delta.length().max(0.001);
            let diff = constraint.rest_length - dist;
            let percent = (diff / dist) * 0.5;
            let offset = delta * percent;

            changes[idx_a] -= offset;
            changes[idx_b] += offset;

            counts[idx_a] += 1;
            counts[idx_b] += 1;
        }

        for i in 0..physics.nodes.len() {
            if counts[i] > 0 {
                physics.nodes[i].position += changes[i] / counts[i] as f32;

                let ground_y = physics.nodes[i].radius;
                if physics.nodes[i].position.y < ground_y {
                    physics.nodes[i].position.y = ground_y;
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn character_designer_ui(
    mut commands: Commands,
    mut contexts: EguiContexts,
    mut next_state: ResMut<NextState<AppState>>,
    mut settings: ResMut<CharacterSettings>,
    mut physics: ResMut<RagdollPhysics>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    model_part_query: Query<Entity, With<CharacterModelPart>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    egui::Window::new("Character Designer & Ragdoll Simulator")
        .default_width(320.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .max_height(ui.available_height())
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.heading("Customization Menu");
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut settings.custom_name);
                    });

                    ui.add_space(5.0);
                    ui.heading("👕 Outfit & Armor Style");
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(
                                settings.outfit_style == OutfitStyle::SciFiSuit,
                                "🚀 Sci-Fi Suit",
                            )
                            .clicked()
                        {
                            settings.outfit_style = OutfitStyle::SciFiSuit;
                            trigger_rebuild(
                                &mut commands,
                                &mut meshes,
                                &mut materials,
                                &settings,
                                &mut physics,
                                &model_part_query,
                            );
                        }
                        if ui
                            .selectable_label(
                                settings.outfit_style == OutfitStyle::TacticalArmor,
                                "🛡️ Tactical",
                            )
                            .clicked()
                        {
                            settings.outfit_style = OutfitStyle::TacticalArmor;
                            trigger_rebuild(
                                &mut commands,
                                &mut meshes,
                                &mut materials,
                                &settings,
                                &mut physics,
                                &model_part_query,
                            );
                        }
                    });

                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(
                                settings.outfit_style == OutfitStyle::StylizedHero,
                                "🦸 Stylized Hero",
                            )
                            .clicked()
                        {
                            settings.outfit_style = OutfitStyle::StylizedHero;
                            trigger_rebuild(
                                &mut commands,
                                &mut meshes,
                                &mut materials,
                                &settings,
                                &mut physics,
                                &model_part_query,
                            );
                        }
                        if ui
                            .selectable_label(
                                settings.outfit_style == OutfitStyle::SkeletonExoFrame,
                                "💀 Exo-Skeleton",
                            )
                            .clicked()
                        {
                            settings.outfit_style = OutfitStyle::SkeletonExoFrame;
                            trigger_rebuild(
                                &mut commands,
                                &mut meshes,
                                &mut materials,
                                &settings,
                                &mut physics,
                                &model_part_query,
                            );
                        }
                        if ui
                            .selectable_label(
                                settings.outfit_style == OutfitStyle::ClassicMannequin,
                                "🪵 Classic",
                            )
                            .clicked()
                        {
                            settings.outfit_style = OutfitStyle::ClassicMannequin;
                            trigger_rebuild(
                                &mut commands,
                                &mut meshes,
                                &mut materials,
                                &settings,
                                &mut physics,
                                &model_part_query,
                            );
                        }
                    });

                    ui.add_space(5.0);
                    ui.label("Gender Style:");
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(settings.gender == Gender::Male, "♂ Male Body")
                            .clicked()
                        {
                            settings.gender = Gender::Male;
                            trigger_rebuild(
                                &mut commands,
                                &mut meshes,
                                &mut materials,
                                &settings,
                                &mut physics,
                                &model_part_query,
                            );
                        }
                        if ui
                            .selectable_label(settings.gender == Gender::Female, "♀ Female Body")
                            .clicked()
                        {
                            settings.gender = Gender::Female;
                            trigger_rebuild(
                                &mut commands,
                                &mut meshes,
                                &mut materials,
                                &settings,
                                &mut physics,
                                &model_part_query,
                            );
                        }
                    });

                    ui.add_space(5.0);
                    let prev_h = settings.height;
                    ui.add(
                        egui::Slider::new(&mut settings.height, 1.2..=2.2).text("Height Adjuster"),
                    );
                    if settings.height != prev_h {
                        trigger_rebuild(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            &settings,
                            &mut physics,
                            &model_part_query,
                        );
                    }

                    let prev_w = settings.weight;
                    ui.add(
                        egui::Slider::new(&mut settings.weight, 0.5..=1.5).text("Weight Adjuster"),
                    );
                    if settings.weight != prev_w {
                        trigger_rebuild(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            &settings,
                            &mut physics,
                            &model_part_query,
                        );
                    }

                    let prev_head = settings.head_scale;
                    ui.add(
                        egui::Slider::new(&mut settings.head_scale, 0.7..=1.4).text("Head Scale"),
                    );
                    if settings.head_scale != prev_head {
                        trigger_rebuild(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            &settings,
                            &mut physics,
                            &model_part_query,
                        );
                    }

                    ui.add_space(5.0);
                    egui::CollapsingHeader::new("🧬 Realistic Proportions")
                        .default_open(true)
                        .show(ui, |ui| {
                            let prev_muscle = settings.muscle_mass;
                            ui.add(
                                egui::Slider::new(&mut settings.muscle_mass, 0.0..=1.5)
                                    .text("Muscle Mass"),
                            );
                            if settings.muscle_mass != prev_muscle {
                                trigger_rebuild(
                                    &mut commands,
                                    &mut meshes,
                                    &mut materials,
                                    &settings,
                                    &mut physics,
                                    &model_part_query,
                                );
                            }

                            let prev_sh = settings.shoulder_width;
                            ui.add(
                                egui::Slider::new(&mut settings.shoulder_width, 0.7..=1.4)
                                    .text("Shoulder Width"),
                            );
                            if settings.shoulder_width != prev_sh {
                                trigger_rebuild(
                                    &mut commands,
                                    &mut meshes,
                                    &mut materials,
                                    &settings,
                                    &mut physics,
                                    &model_part_query,
                                );
                            }

                            let prev_leg = settings.leg_length;
                            ui.add(
                                egui::Slider::new(&mut settings.leg_length, 0.7..=1.4)
                                    .text("Leg Length"),
                            );
                            if settings.leg_length != prev_leg {
                                trigger_rebuild(
                                    &mut commands,
                                    &mut meshes,
                                    &mut materials,
                                    &settings,
                                    &mut physics,
                                    &model_part_query,
                                );
                            }

                            let prev_waist = settings.waist_width;
                            ui.add(
                                egui::Slider::new(&mut settings.waist_width, 0.7..=1.4)
                                    .text("Waist Width"),
                            );
                            if settings.waist_width != prev_waist {
                                trigger_rebuild(
                                    &mut commands,
                                    &mut meshes,
                                    &mut materials,
                                    &settings,
                                    &mut physics,
                                    &model_part_query,
                                );
                            }
                        });

                    ui.add_space(8.0);
                    ui.heading("Aesthetics");
                    ui.separator();

                    let prev_style = settings.hair_style;
                    ui.label("Hair Style:");
                    egui::ComboBox::from_label("")
                        .selected_text(format!("{:?}", settings.hair_style))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut settings.hair_style,
                                HairStyle::None,
                                "None (Plain Head)",
                            );
                            ui.selectable_value(
                                &mut settings.hair_style,
                                HairStyle::Short,
                                "Short Cut",
                            );
                            ui.selectable_value(
                                &mut settings.hair_style,
                                HairStyle::Ponytail,
                                "Ponytail Style",
                            );
                            ui.selectable_value(
                                &mut settings.hair_style,
                                HairStyle::Spiky,
                                "Spiky Hair",
                            );
                            ui.selectable_value(
                                &mut settings.hair_style,
                                HairStyle::Curly,
                                "Curly Locks",
                            );
                        });
                    if settings.hair_style != prev_style {
                        trigger_rebuild(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            &settings,
                            &mut physics,
                            &model_part_query,
                        );
                    }

                    ui.add_space(5.0);
                    ui.label("Hair Color:");
                    ui.horizontal(|ui| {
                        let mut c = [
                            (settings.hair_color.to_srgba().red * 255.0) as u8,
                            (settings.hair_color.to_srgba().green * 255.0) as u8,
                            (settings.hair_color.to_srgba().blue * 255.0) as u8,
                        ];
                        if ui.color_edit_button_srgb(&mut c).changed() {
                            settings.hair_color = Color::srgb(
                                c[0] as f32 / 255.0,
                                c[1] as f32 / 255.0,
                                c[2] as f32 / 255.0,
                            );
                            trigger_rebuild(
                                &mut commands,
                                &mut meshes,
                                &mut materials,
                                &settings,
                                &mut physics,
                                &model_part_query,
                            );
                        }
                        ui.label(format!("RGB: {}, {}, {}", c[0], c[1], c[2]));
                    });

                    ui.label("Skin Tone:");
                    ui.horizontal(|ui| {
                        let mut c = [
                            (settings.skin_color.to_srgba().red * 255.0) as u8,
                            (settings.skin_color.to_srgba().green * 255.0) as u8,
                            (settings.skin_color.to_srgba().blue * 255.0) as u8,
                        ];
                        if ui.color_edit_button_srgb(&mut c).changed() {
                            settings.skin_color = Color::srgb(
                                c[0] as f32 / 255.0,
                                c[1] as f32 / 255.0,
                                c[2] as f32 / 255.0,
                            );
                            trigger_rebuild(
                                &mut commands,
                                &mut meshes,
                                &mut materials,
                                &settings,
                                &mut physics,
                                &model_part_query,
                            );
                        }
                        ui.label("Pick skin tone color");
                    });

                    ui.add_space(5.0);
                    ui.checkbox(&mut settings.show_xray, "💀 Show Skeleton (X-Ray View)");

                    ui.add_space(10.0);
                    ui.heading("Physics Sim");
                    ui.separator();

                    let ragdoll_label = if settings.is_ragdoll_active {
                        "⏸ Lock Skeleton (Design Mode)"
                    } else {
                        "💥 Launch Ragdoll Physics!"
                    };
                    if ui.button(ragdoll_label).clicked() {
                        settings.is_ragdoll_active = !settings.is_ragdoll_active;
                        if !settings.is_ragdoll_active {
                            trigger_rebuild(
                                &mut commands,
                                &mut meshes,
                                &mut materials,
                                &settings,
                                &mut physics,
                                &model_part_query,
                            );
                        }
                    }

                    if settings.is_ragdoll_active
                        && ui.button("⚡ Give Impulse / Push Ragdoll").clicked()
                    {
                        for node in physics.nodes.iter_mut() {
                            let force = Vec3::new(
                                (rand::random::<f32>() - 0.5) * 5.0,
                                rand::random::<f32>() * 8.0 + 2.0,
                                (rand::random::<f32>() - 0.5) * 5.0,
                            );
                            node.position += force * 0.016;
                        }
                    }

                    ui.add_space(10.0);
                    ui.heading("Sprite Renderer");
                    ui.separator();
                    ui.label("Capture customized 3D character as 2D sprite sheet!");
                    if ui.button("📸 Capture Character Sprite").clicked() {
                        settings.is_sprite_rendered = true;
                    }

                    if settings.is_sprite_rendered {
                        ui.add(egui::Label::new(
                            egui::RichText::new("Character captured as 2D Sprite frame!")
                                .color(egui::Color32::from_rgb(100, 255, 100))
                                .strong(),
                        ));
                    }

                    ui.add_space(20.0);
                    if ui
                        .add(
                            egui::Button::new("🎮 Design & Enter Play Mode")
                                .fill(egui::Color32::from_rgb(50, 150, 50)),
                        )
                        .clicked()
                    {
                        next_state.set(AppState::PlayMode);
                    }
                    if ui.button("🚪 Exit to Launcher").clicked() {
                        next_state.set(AppState::MainMenu);
                    }
                });
        });
}

fn trigger_rebuild(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    settings: &CharacterSettings,
    physics: &mut RagdollPhysics,
    model_part_query: &Query<Entity, With<CharacterModelPart>>,
) {
    for entity in model_part_query.iter() {
        if let Ok(mut cmd) = commands.get_entity(entity) {
            cmd.despawn();
        }
    }

    initialize_ragdoll_skeleton(settings, physics);
    spawn_character_visuals(commands, meshes, materials, settings, physics);
}
