use crate::AppState;
use crate::map_editor::data::{Biome, TempestMap};
use crate::play_mode::{
    PlayModeEntity, PlayModePlayer, PlayerInventory, WallCollider, get_bilinear_height,
};
use avian3d::prelude::{Collider, Position, RigidBody};
use bevy::prelude::*;
use bevy_egui::egui;

pub struct HousePlugin;

impl Plugin for HousePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HousePuzzleState>()
            .init_resource::<MansionSettings>()
            .add_systems(OnEnter(AppState::PlayMode), spawn_house)
            .add_systems(
                Update,
                (
                    house_interaction_system,
                    bookcase_slide_system,
                    door_swing_system,
                    crate_movement_system,
                    pressure_plate_system,
                    gate_slide_system,
                    pedestal_glow_system,
                    vault_door_unlock_system,
                    research_complex_ui_system,
                )
                    .chain()
                    .run_if(in_state(AppState::PlayMode)),
            );
    }
}

#[derive(Resource, Default)]
pub struct HousePuzzleState {
    pub bookcase_opened: bool,
    pub basement_solved: bool,
    pub artifact_collected: bool,
    pub vault_unlocked: bool,
    pub active_terminal_log: Option<u32>,
    pub show_security_keypad: bool,
    pub keypad_input: String,
    pub show_synthesizer_ui: bool,
}

#[allow(dead_code)]
#[derive(Component)]
pub struct ResearchTerminal {
    pub terminal_id: u32,
    pub title: String,
    pub log_text: String,
}

#[derive(Component)]
pub struct BasementSecurityConsole;

#[derive(Component)]
pub struct BasementVaultDoor;

#[derive(Component)]
pub struct PlasmaSynthesizerConsole;

#[derive(Resource, Clone, Copy)]
pub struct MansionSettings {
    pub cols: u32,
    pub rows: u32,
    pub cell_size: f32,
}

impl Default for MansionSettings {
    fn default() -> Self {
        Self {
            cols: 8,
            rows: 4,
            cell_size: 5.0,
        }
    }
}

// Marker components for puzzle entities
#[derive(Component)]
pub struct HouseMarker;

#[derive(Component)]
pub struct BookcaseDoor {
    pub start_pos: Vec3,
    pub target_pos: Vec3,
}

#[derive(Component)]
pub struct InteractiveBookcase;

#[derive(Component)]
pub struct Teleporter {
    pub target_pos: Vec3,
    pub message: String,
}

#[derive(Component)]
pub struct PushableCrate {
    pub target_pos: Vec3,
}

#[allow(dead_code)]
#[derive(Component)]
pub struct PressurePlate {
    pub id: u32,
    pub triggered: bool,
}

#[derive(Component)]
pub struct CellGate {
    pub start_pos: Vec3,
    pub target_pos: Vec3,
}

#[derive(Component)]
pub struct PuzzleChest {
    pub is_locked: bool,
}

#[derive(Component)]
pub struct ArtifactPedestal;

#[derive(Component)]
pub struct RotatingArtifact;

#[derive(Component)]
pub struct HouseDoor {
    pub is_open: bool,
    pub closed_rot: Quat,
    pub open_rot: Quat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CellType {
    Foyer,
    Hallway,
    Bedroom,
    Empty,
}

// Flatten terrain dynamically at the placed house footprint center
pub fn flatten_terrain(
    mut map: ResMut<TempestMap>,
    mansion_settings: Res<MansionSettings>,
    mut ev_grass: MessageWriter<crate::grass::GenerateGrassEvent>,
) {
    let mut house_pos = Vec3::new(-35.0, 1.5, -35.0);
    for p in map.prefabs.iter() {
        if p.prefab_type == "house" {
            house_pos = Vec3::from_array(p.position);
            break;
        }
    }

    let half_map_w = map.width as f32 / 2.0;
    let half_map_h = map.height as f32 / 2.0;

    let half_w = (mansion_settings.cols as f32 * mansion_settings.cell_size) / 2.0;
    let half_d = (mansion_settings.rows as f32 * mansion_settings.cell_size) / 2.0;

    // Sample natural terrain height around the house center to avoid digging deep ruts down to water level
    let natural_h = get_bilinear_height(house_pos.x, house_pos.z, &map);
    let house_ground_y = natural_h.clamp(1.5, 45.0);

    let yard_size = 14.0_f32;
    let border = yard_size + 10.0_f32;
    let min_x_idx = ((house_pos.x - half_w - border) + half_map_w).max(0.0) as u32;
    let max_x_idx = ((house_pos.x + half_w + border) + half_map_w).min(map.width as f32) as u32;
    let min_z_idx = ((house_pos.z - half_d - border) + half_map_h).max(0.0) as u32;
    let max_z_idx = ((house_pos.z + half_d + border) + half_map_h).min(map.height as f32) as u32;

    for mz in min_z_idx..max_z_idx {
        for mx in min_x_idx..max_x_idx {
            let wx = mx as f32 - half_map_w;
            let wz = mz as f32 - half_map_h;

            let dx = ((wx - house_pos.x).abs() - half_w).max(0.0);
            let dz = ((wz - house_pos.z).abs() - half_d).max(0.0);
            let dist_edge = dx.max(dz);

            if dist_edge <= yard_size {
                map.set_height(mx, mz, house_ground_y);
            } else {
                let blend_t = ((dist_edge - yard_size) / 10.0).clamp(0.0, 1.0);
                let orig_h = map.get_height(mx, mz);
                let blended = house_ground_y * (1.0 - blend_t) + orig_h * blend_t;
                map.set_height(mx, mz, blended);
            }
            map.set_biome(mx, mz, Biome::Temperate);
        }
    }
    ev_grass.write(crate::grass::GenerateGrassEvent);
}

fn get_cell_type(floor: u32, c: i32, r: i32) -> CellType {
    match floor {
        1 => get_ground_floor_cell(c, r),
        _ => get_first_floor_cell(c, r),
    }
}

// Ground Floor
fn get_ground_floor_cell(c: i32, r: i32) -> CellType {
    if (c == 3 || c == 4) && (r == 1 || r == 2 || r == 3) {
        CellType::Foyer
    } else if r == 2 {
        CellType::Hallway
    } else if (r == 0 || r == 3) && ((0..=2).contains(&c) || (5..=7).contains(&c)) {
        CellType::Bedroom
    } else {
        CellType::Hallway
    }
}

// First Floor
fn get_first_floor_cell(c: i32, r: i32) -> CellType {
    if (c == 3 || c == 4) && (r == 1 || r == 2) {
        CellType::Empty
    } else if r == 2 {
        CellType::Hallway
    } else if (r == 0) || ((r == 1 || r == 3) && ((0..=2).contains(&c) || (5..=7).contains(&c))) {
        CellType::Bedroom
    } else {
        CellType::Hallway
    }
}
fn spawn_lantern(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
) {
    let casing_mesh = meshes.add(Cylinder::new(0.12, 0.4));
    let glass_mesh = meshes.add(Sphere::new(0.08));

    let iron_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.15, 0.15),
        metallic: 0.8,
        perceptual_roughness: 0.5,
        ..default()
    });

    let glow_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.65, 0.15), // warm yellow-orange lantern glow
        emissive: LinearRgba::from(Color::srgb(1.0, 0.65, 0.15)) * 4.0,
        perceptual_roughness: 0.1,
        ..default()
    });

    // Spawn casing
    commands.spawn((
        Mesh3d(casing_mesh),
        MeshMaterial3d(iron_mat),
        Transform::from_translation(pos),
        HouseMarker,
        PlayModeEntity,
    ));

    // Spawn glow core
    commands.spawn((
        Mesh3d(glass_mesh),
        MeshMaterial3d(glow_mat),
        Transform::from_translation(pos + Vec3::new(0.0, -0.05, 0.0)),
        HouseMarker,
        PlayModeEntity,
    ));

    // Spawn PointLight
    commands.spawn((
        PointLight {
            color: Color::srgb(1.0, 0.65, 0.15),
            intensity: 900.0,
            range: 16.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_translation(pos + Vec3::new(0.0, -0.1, 0.0)),
        HouseMarker,
        PlayModeEntity,
    ));
}

pub fn create_world_uv_cuboid(size: Vec3, tile_scale: f32) -> Mesh {
    let hx = size.x * 0.5;
    let hy = size.y * 0.5;
    let hz = size.z * 0.5;

    let sx = size.x / tile_scale;
    let sy = size.y / tile_scale;
    let sz = size.z / tile_scale;

    let positions = vec![
        // Front (+Z)
        [-hx, -hy, hz],
        [hx, -hy, hz],
        [hx, hy, hz],
        [-hx, hy, hz],
        // Back (-Z)
        [hx, -hy, -hz],
        [-hx, -hy, -hz],
        [-hx, hy, -hz],
        [hx, hy, -hz],
        // Top (+Y)
        [-hx, hy, hz],
        [hx, hy, hz],
        [hx, hy, -hz],
        [-hx, hy, -hz],
        // Bottom (-Y)
        [-hx, -hy, -hz],
        [hx, -hy, -hz],
        [hx, -hy, hz],
        [-hx, -hy, hz],
        // Right (+X)
        [hx, -hy, hz],
        [hx, -hy, -hz],
        [hx, hy, -hz],
        [hx, hy, hz],
        // Left (-X)
        [-hx, -hy, -hz],
        [-hx, -hy, hz],
        [-hx, hy, hz],
        [-hx, hy, -hz],
    ];

    let normals = vec![
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, -1.0],
        [0.0, 0.0, -1.0],
        [0.0, 0.0, -1.0],
        [0.0, 0.0, -1.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
    ];

    let uvs = vec![
        // Front (+Z)
        [0.0, sy],
        [sx, sy],
        [sx, 0.0],
        [0.0, 0.0],
        // Back (-Z)
        [0.0, sy],
        [sx, sy],
        [sx, 0.0],
        [0.0, 0.0],
        // Top (+Y)
        [0.0, 0.0],
        [sx, 0.0],
        [sx, sz],
        [0.0, sz],
        // Bottom (-Y)
        [0.0, 0.0],
        [sx, 0.0],
        [sx, sz],
        [0.0, sz],
        // Right (+X)
        [0.0, sy],
        [sz, sy],
        [sz, 0.0],
        [0.0, 0.0],
        // Left (-X)
        [0.0, sy],
        [sz, sy],
        [sz, 0.0],
        [0.0, 0.0],
    ];

    let indices = vec![
        0, 1, 2, 2, 3, 0, 4, 5, 6, 6, 7, 4, 8, 9, 10, 10, 11, 8, 12, 13, 14, 14, 15, 12, 16, 17,
        18, 18, 19, 16, 20, 21, 22, 22, 23, 20,
    ];

    let mut mesh = Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
    mesh
}

pub fn create_door_mesh(size: Vec3) -> Mesh {
    let hx = size.x * 0.5;
    let hy = size.y * 0.5;
    let hz = size.z * 0.5;

    let positions = vec![
        // Front (+Z)
        [-hx, -hy, hz],
        [hx, -hy, hz],
        [hx, hy, hz],
        [-hx, hy, hz],
        // Back (-Z)
        [hx, -hy, -hz],
        [-hx, -hy, -hz],
        [-hx, hy, -hz],
        [hx, hy, -hz],
        // Top (+Y)
        [-hx, hy, hz],
        [hx, hy, hz],
        [hx, hy, -hz],
        [-hx, hy, -hz],
        // Bottom (-Y)
        [-hx, -hy, -hz],
        [hx, -hy, -hz],
        [hx, -hy, hz],
        [-hx, -hy, hz],
        // Right (+X)
        [hx, -hy, hz],
        [hx, -hy, -hz],
        [hx, hy, -hz],
        [hx, hy, hz],
        // Left (-X)
        [-hx, -hy, -hz],
        [-hx, -hy, hz],
        [-hx, hy, hz],
        [-hx, hy, -hz],
    ];

    let normals = vec![
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, -1.0],
        [0.0, 0.0, -1.0],
        [0.0, 0.0, -1.0],
        [0.0, 0.0, -1.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
    ];

    let uvs = vec![
        // Front (+Z): map 0..1 texture right-side up
        [0.0, 1.0],
        [1.0, 1.0],
        [1.0, 0.0],
        [0.0, 0.0],
        // Back (-Z): map 0..1 texture right-side up
        [0.0, 1.0],
        [1.0, 1.0],
        [1.0, 0.0],
        [0.0, 0.0],
        // Top (+Y)
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 0.1],
        [0.0, 0.1],
        // Bottom (-Y)
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 0.1],
        [0.0, 0.1],
        // Right (+X)
        [0.0, 1.0],
        [0.1, 1.0],
        [0.1, 0.0],
        [0.0, 0.0],
        // Left (-X)
        [0.0, 1.0],
        [0.1, 1.0],
        [0.1, 0.0],
        [0.0, 0.0],
    ];

    let indices = vec![
        0, 1, 2, 2, 3, 0, 4, 5, 6, 6, 7, 4, 8, 9, 10, 10, 11, 8, 12, 13, 14, 14, 15, 12, 16, 17,
        18, 18, 19, 16, 20, 21, 22, 22, 23, 20,
    ];

    let mut mesh = Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));
    mesh
}

fn spawn_solid_wall(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    material: &Handle<StandardMaterial>,
    pos: Vec3,
    is_horizontal: bool,
    cell_size: f32,
) {
    let size = if is_horizontal {
        Vec3::new(cell_size + 0.2, 3.5, 0.2)
    } else {
        Vec3::new(0.2, 3.5, cell_size + 0.2)
    };
    let col_half = if is_horizontal {
        Vec3::new((cell_size + 0.2) * 0.5, 1.75, 0.35)
    } else {
        Vec3::new(0.35, 1.75, (cell_size + 0.2) * 0.5)
    };

    commands.spawn((
        Mesh3d(meshes.add(create_world_uv_cuboid(size, 3.0))),
        MeshMaterial3d(material.clone()),
        Transform::from_translation(pos),
        WallCollider {
            half_extents: col_half,
        },
        HouseMarker,
        PlayModeEntity,
    ));
}

#[allow(clippy::too_many_arguments)]
fn spawn_window_wall(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    wall_material: &Handle<StandardMaterial>,
    pos: Vec3,
    is_horizontal: bool,
    cell_size: f32,
) {
    let window_width = 1.6;
    let post_width = (cell_size - window_width) / 2.0;

    let iron_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.1, 0.1),
        metallic: 0.9,
        perceptual_roughness: 0.2,
        ..default()
    });

    if is_horizontal {
        // Left post
        let lp_size = Vec3::new(post_width + 0.1, 3.5, 0.2);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(lp_size))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_translation(
                pos + Vec3::new(-(cell_size * 0.5 - post_width * 0.5), 0.0, 0.0),
            ),
            WallCollider {
                half_extents: Vec3::new((post_width + 0.1) * 0.5, 1.75, 0.35),
            },
            HouseMarker,
            PlayModeEntity,
        ));
        // Right post
        let rp_size = Vec3::new(post_width + 0.1, 3.5, 0.2);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(rp_size))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_translation(
                pos + Vec3::new(cell_size * 0.5 - post_width * 0.5, 0.0, 0.0),
            ),
            WallCollider {
                half_extents: Vec3::new((post_width + 0.1) * 0.5, 1.75, 0.35),
            },
            HouseMarker,
            PlayModeEntity,
        ));
        // Bottom post
        let bp_size = Vec3::new(window_width, 1.0, 0.2);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(bp_size))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_translation(pos + Vec3::new(0.0, -1.25, 0.0)),
            HouseMarker,
            PlayModeEntity,
        ));
        // Top post
        let tp_size = Vec3::new(window_width, 1.0, 0.2);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(tp_size))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_translation(pos + Vec3::new(0.0, 1.25, 0.0)),
            HouseMarker,
            PlayModeEntity,
        ));
        // Window opening barrier collider (prevents walking through window)
        commands.spawn((
            Transform::from_translation(pos),
            WallCollider {
                half_extents: Vec3::new(window_width * 0.5, 1.75, 0.35),
            },
            HouseMarker,
            PlayModeEntity,
        ));

        // 3 Vertical iron bars
        let bar_mesh = meshes.add(Cylinder::new(0.02, 1.5));
        for &offset_x in &[-0.35, 0.0, 0.35] {
            commands.spawn((
                Mesh3d(bar_mesh.clone()),
                MeshMaterial3d(iron_mat.clone()),
                Transform::from_translation(pos + Vec3::new(offset_x, 0.0, 0.0)),
                HouseMarker,
                PlayModeEntity,
            ));
        }
    } else {
        // Left post (along negative Z)
        let lp_size = Vec3::new(0.2, 3.5, post_width + 0.1);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(lp_size))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_translation(
                pos + Vec3::new(0.0, 0.0, -(cell_size * 0.5 - post_width * 0.5)),
            ),
            WallCollider {
                half_extents: Vec3::new(0.35, 1.75, (post_width + 0.1) * 0.5),
            },
            HouseMarker,
            PlayModeEntity,
        ));
        // Right post (along positive Z)
        let rp_size = Vec3::new(0.2, 3.5, post_width + 0.1);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(rp_size))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_translation(
                pos + Vec3::new(0.0, 0.0, cell_size * 0.5 - post_width * 0.5),
            ),
            WallCollider {
                half_extents: Vec3::new(0.35, 1.75, (post_width + 0.1) * 0.5),
            },
            HouseMarker,
            PlayModeEntity,
        ));
        // Bottom post
        let bp_size = Vec3::new(0.2, 1.0, window_width);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(bp_size))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_translation(pos + Vec3::new(0.0, -1.25, 0.0)),
            HouseMarker,
            PlayModeEntity,
        ));
        // Top post
        let tp_size = Vec3::new(0.2, 1.0, window_width);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(tp_size))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_translation(pos + Vec3::new(0.0, 1.25, 0.0)),
            HouseMarker,
            PlayModeEntity,
        ));
        // Window opening barrier collider (prevents walking through window)
        commands.spawn((
            Transform::from_translation(pos),
            WallCollider {
                half_extents: Vec3::new(0.35, 1.75, window_width * 0.5),
            },
            HouseMarker,
            PlayModeEntity,
        ));

        // 3 Vertical iron bars
        let bar_mesh = meshes.add(Cylinder::new(0.02, 1.5));
        for &offset_z in &[-0.35, 0.0, 0.35] {
            commands.spawn((
                Mesh3d(bar_mesh.clone()),
                MeshMaterial3d(iron_mat.clone()),
                Transform::from_translation(pos + Vec3::new(0.0, 0.0, offset_z)),
                HouseMarker,
                PlayModeEntity,
            ));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_door_wall(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    wall_material: &Handle<StandardMaterial>,
    pos: Vec3,
    is_horizontal: bool,
    asset_server: &Res<AssetServer>,
    cell_size: f32,
) {
    let door_width = 1.6;
    let door_height = 2.2;

    let door_mat = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/wooden_door.png")),
        perceptual_roughness: 0.8,
        ..default()
    });

    let post_width = (cell_size - door_width) / 2.0;

    if is_horizontal {
        // Left post
        let lp_size = Vec3::new(post_width + 0.1, 3.5, 0.2);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(lp_size))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_translation(
                pos + Vec3::new(-(cell_size * 0.5 - post_width * 0.5), 0.0, 0.0),
            ),
            WallCollider {
                half_extents: Vec3::new((post_width + 0.1) * 0.5, 1.75, 0.35),
            },
            HouseMarker,
            PlayModeEntity,
        ));

        // Right post
        let rp_size = Vec3::new(post_width + 0.1, 3.5, 0.2);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(rp_size))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_translation(
                pos + Vec3::new(cell_size * 0.5 - post_width * 0.5, 0.0, 0.0),
            ),
            WallCollider {
                half_extents: Vec3::new((post_width + 0.1) * 0.5, 1.75, 0.35),
            },
            HouseMarker,
            PlayModeEntity,
        ));

        // Top lintel
        let l_size = Vec3::new(door_width, 1.3, 0.2);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(l_size))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_translation(pos + Vec3::new(0.0, 1.1, 0.0)),
            HouseMarker,
            PlayModeEntity,
        ));

        // Swinging wooden door with hinge parent
        let door_size = Vec3::new(door_width - 0.05, door_height, 0.08);
        let closed_rot = Quat::IDENTITY;
        let open_rot = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);

        // Hinge position at the left edge of the opening
        let hinge_pos = pos + Vec3::new(-0.8, -0.65, 0.0);

        let parent_id = commands
            .spawn((
                Transform::from_translation(hinge_pos).with_rotation(closed_rot),
                HouseDoor {
                    is_open: false,
                    closed_rot,
                    open_rot,
                },
                HouseMarker,
                PlayModeEntity,
                Visibility::Visible,
                InheritedVisibility::default(),
            ))
            .id();

        // Child visual mesh offset to the right by half door width, holding door collider
        let child_id = commands
            .spawn((
                Mesh3d(meshes.add(create_door_mesh(door_size))),
                MeshMaterial3d(door_mat.clone()),
                Transform::from_xyz(0.8, 0.0, 0.0),
                HouseDoor {
                    is_open: false,
                    closed_rot,
                    open_rot,
                },
                WallCollider {
                    half_extents: Vec3::new(0.85, 1.1, 0.35), // Centered on doorway opening, seamlessly overlaps posts
                },
                HouseMarker,
                PlayModeEntity,
            ))
            .id();

        commands.entity(parent_id).add_child(child_id);
    } else {
        // Vertical door wall (along Z)
        let lp_size = Vec3::new(0.2, 3.5, post_width + 0.1);
        commands.spawn((
            Mesh3d(meshes.add(create_world_uv_cuboid(lp_size, 3.0))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_translation(
                pos + Vec3::new(0.0, 0.0, -(cell_size * 0.5 - post_width * 0.5)),
            ),
            WallCollider {
                half_extents: Vec3::new(0.35, 1.75, (post_width + 0.1) * 0.5),
            },
            HouseMarker,
            PlayModeEntity,
        ));

        let rp_size = Vec3::new(0.2, 3.5, post_width + 0.1);
        commands.spawn((
            Mesh3d(meshes.add(create_world_uv_cuboid(rp_size, 3.0))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_translation(
                pos + Vec3::new(0.0, 0.0, cell_size * 0.5 - post_width * 0.5),
            ),
            WallCollider {
                half_extents: Vec3::new(0.35, 1.75, (post_width + 0.1) * 0.5),
            },
            HouseMarker,
            PlayModeEntity,
        ));

        let l_size = Vec3::new(0.2, 1.3, door_width);
        commands.spawn((
            Mesh3d(meshes.add(create_world_uv_cuboid(l_size, 3.0))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_translation(pos + Vec3::new(0.0, 1.1, 0.0)),
            HouseMarker,
            PlayModeEntity,
        ));

        // Swinging wooden door with hinge parent
        let door_size = Vec3::new(0.08, door_height, door_width - 0.05);
        let closed_rot = Quat::IDENTITY;
        let open_rot = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);

        // Hinge position at the top edge of the opening (negative Z)
        let hinge_pos = pos + Vec3::new(0.0, -0.65, -0.8);

        let parent_id = commands
            .spawn((
                Transform::from_translation(hinge_pos).with_rotation(closed_rot),
                HouseDoor {
                    is_open: false,
                    closed_rot,
                    open_rot,
                },
                HouseMarker,
                PlayModeEntity,
                Visibility::Visible,
                InheritedVisibility::default(),
            ))
            .id();

        // Child visual mesh offset along Z by half door width, holding door collider
        let child_id = commands
            .spawn((
                Mesh3d(meshes.add(create_door_mesh(door_size))),
                MeshMaterial3d(door_mat),
                Transform::from_xyz(0.0, 0.0, 0.8),
                HouseDoor {
                    is_open: false,
                    closed_rot,
                    open_rot,
                },
                WallCollider {
                    half_extents: Vec3::new(0.35, 1.1, 0.85), // Centered on doorway opening, seamlessly overlaps posts
                },
                HouseMarker,
                PlayModeEntity,
            ))
            .id();

        commands.entity(parent_id).add_child(child_id);
    }
}

fn spawn_house(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut puzzle_state: ResMut<HousePuzzleState>,
    map: Res<TempestMap>,
    mansion_settings: Res<MansionSettings>,
    asset_server: Res<AssetServer>,
) {
    *puzzle_state = HousePuzzleState::default();

    let mut house_pos = Vec3::new(-35.0, 1.5, -35.0);
    for p in map.prefabs.iter() {
        if p.prefab_type == "house" {
            house_pos = Vec3::from_array(p.position);
            break;
        }
    }
    house_pos.y = get_bilinear_height(house_pos.x, house_pos.z, &map);

    let load_repeat_tex = |path: &'static str| -> Handle<Image> {
        asset_server
            .load_builder()
            .with_settings(|settings: &mut bevy::image::ImageLoaderSettings| {
                settings.sampler =
                    bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
                        address_mode_u: bevy::image::ImageAddressMode::Repeat,
                        address_mode_v: bevy::image::ImageAddressMode::Repeat,
                        ..default()
                    });
            })
            .load(path)
    };

    // Materials
    let wall_mat = materials.add(StandardMaterial {
        base_color_texture: Some(load_repeat_tex("textures/solid_brick.png")),
        perceptual_roughness: 0.9,
        cull_mode: None,
        double_sided: true,
        ..default()
    });

    let floor_mat = materials.add(StandardMaterial {
        base_color_texture: Some(load_repeat_tex("textures/solid_limestone.png")),
        perceptual_roughness: 0.85,
        cull_mode: None,
        double_sided: true,
        ..default()
    });

    let basement_floor_mat = materials.add(StandardMaterial {
        base_color_texture: Some(load_repeat_tex("textures/solid_limestone.png")),
        perceptual_roughness: 0.85,
        cull_mode: None,
        double_sided: true,
        ..default()
    });

    let basement_wall_mat = materials.add(StandardMaterial {
        base_color_texture: Some(load_repeat_tex("textures/rock_wall.png")),
        perceptual_roughness: 0.9,
        cull_mode: None,
        double_sided: true,
        ..default()
    });

    let sub_basement_mat = materials.add(StandardMaterial {
        base_color_texture: Some(load_repeat_tex("textures/rock_wall.png")),
        perceptual_roughness: 0.9,
        cull_mode: None,
        double_sided: true,
        ..default()
    });

    let gold_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.82, 0.1),
        metallic: 0.9,
        perceptual_roughness: 0.1,
        ..default()
    });

    let red_glow_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.1, 0.15),
        emissive: LinearRgba::from(Color::srgb(0.8, 0.05, 0.05)),
        perceptual_roughness: 0.5,
        ..default()
    });

    // -----------------------------------------------------------------
    // FLOORS 1 & 2: GROUND FLOOR & FIRST FLOOR PROCEDURAL GRID
    // -----------------------------------------------------------------
    let grid_cols = mansion_settings.cols as i32;
    let grid_rows = mansion_settings.rows as i32;
    let cell_size = mansion_settings.cell_size;

    let half_w = (grid_cols as f32 * cell_size) * 0.5;
    let half_d = (grid_rows as f32 * cell_size) * 0.5;

    for floor in 1..=2 {
        let y_base = if floor == 1 {
            house_pos.y
        } else {
            house_pos.y + 3.5
        };

        for r in 0..grid_rows {
            for c in 0..grid_cols {
                let cell_type = get_cell_type(floor, c, r);
                let x_center = house_pos.x - half_w + (cell_size * 0.5) + (c as f32) * cell_size;
                let z_center = house_pos.z - half_d + (cell_size * 0.5) + (r as f32) * cell_size;

                // 1. Spawn Floor mesh
                if cell_type != CellType::Empty {
                    let y_floor = if floor == 1 { y_base + 0.02 } else { y_base };
                    commands.spawn((
                        Mesh3d(meshes.add(Cuboid::new(cell_size, 0.1, cell_size))),
                        MeshMaterial3d(floor_mat.clone()),
                        Transform::from_xyz(x_center, y_floor, z_center),
                        RigidBody::Static,
                        Collider::cuboid(cell_size, 0.1, cell_size),
                        HouseMarker,
                        PlayModeEntity,
                    ));
                }

                // 2. Spawn Ceiling mesh
                if !(floor == 1 && cell_type == CellType::Foyer) {
                    commands.spawn((
                        Mesh3d(meshes.add(Cuboid::new(cell_size, 0.1, cell_size))),
                        MeshMaterial3d(floor_mat.clone()),
                        Transform::from_xyz(x_center, y_base + 3.5, z_center),
                        RigidBody::Static,
                        Collider::cuboid(cell_size, 0.1, cell_size),
                        HouseMarker,
                        PlayModeEntity,
                    ));
                }

                // 3. Spawn boundaries
                // NORTH BOUNDARY
                if r == 0 {
                    if cell_type == CellType::Bedroom {
                        spawn_window_wall(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            &wall_mat,
                            Vec3::new(x_center, y_base + 1.75, house_pos.z - half_d),
                            true,
                            cell_size,
                        );
                    } else {
                        spawn_solid_wall(
                            &mut commands,
                            &mut meshes,
                            &wall_mat,
                            Vec3::new(x_center, y_base + 1.75, house_pos.z - half_d),
                            true,
                            cell_size,
                        );
                    }
                } else {
                    let n_type = get_cell_type(floor, c, r - 1);
                    if cell_type != n_type
                        && cell_type != CellType::Empty
                        && n_type != CellType::Empty
                    {
                        if (cell_type == CellType::Bedroom || n_type == CellType::Bedroom)
                            && (cell_type == CellType::Hallway
                                || n_type == CellType::Hallway
                                || cell_type == CellType::Foyer
                                || n_type == CellType::Foyer)
                        {
                            // Secret library puzzle bookcase door goes between northwest cell (0,0) and (0,1)
                            if floor == 1 && c == 0 && r == 1 {
                                let bc_closed = Vec3::new(
                                    x_center,
                                    y_base + 1.3,
                                    house_pos.z - half_d + cell_size,
                                );
                                commands.spawn((
                                    Mesh3d(meshes.add(Cuboid::new(cell_size - 0.2, 2.6, 0.3))),
                                    MeshMaterial3d(materials.add(StandardMaterial {
                                        base_color: Color::srgb(0.48, 0.3, 0.15),
                                        perceptual_roughness: 0.8,
                                        ..default()
                                    })),
                                    Transform::from_translation(bc_closed),
                                    BookcaseDoor {
                                        start_pos: bc_closed,
                                        target_pos: bc_closed,
                                    },
                                    WallCollider {
                                        half_extents: Vec3::new((cell_size - 0.2) * 0.5, 1.3, 0.15),
                                    },
                                    HouseMarker,
                                    PlayModeEntity,
                                ));
                            } else {
                                spawn_door_wall(
                                    &mut commands,
                                    &mut meshes,
                                    &mut materials,
                                    &wall_mat,
                                    Vec3::new(
                                        x_center,
                                        y_base + 1.75,
                                        house_pos.z - half_d + (r as f32) * cell_size,
                                    ),
                                    true,
                                    &asset_server,
                                    cell_size,
                                );
                            }
                        } else if cell_type == CellType::Bedroom && n_type == CellType::Bedroom {
                            spawn_solid_wall(
                                &mut commands,
                                &mut meshes,
                                &wall_mat,
                                Vec3::new(
                                    x_center,
                                    y_base + 1.75,
                                    house_pos.z - half_d + (r as f32) * cell_size,
                                ),
                                true,
                                cell_size,
                            );
                        }
                    }
                }

                // WEST BOUNDARY
                if c == 0 {
                    if cell_type == CellType::Bedroom {
                        spawn_window_wall(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            &wall_mat,
                            Vec3::new(house_pos.x - half_w, y_base + 1.75, z_center),
                            false,
                            cell_size,
                        );
                    } else {
                        spawn_solid_wall(
                            &mut commands,
                            &mut meshes,
                            &wall_mat,
                            Vec3::new(house_pos.x - half_w, y_base + 1.75, z_center),
                            false,
                            cell_size,
                        );
                    }
                } else {
                    let w_type = get_cell_type(floor, c - 1, r);
                    if cell_type != w_type
                        && cell_type != CellType::Empty
                        && w_type != CellType::Empty
                    {
                        if (cell_type == CellType::Bedroom || w_type == CellType::Bedroom)
                            && (cell_type == CellType::Hallway
                                || w_type == CellType::Hallway
                                || cell_type == CellType::Foyer
                                || w_type == CellType::Foyer)
                        {
                            spawn_door_wall(
                                &mut commands,
                                &mut meshes,
                                &mut materials,
                                &wall_mat,
                                Vec3::new(
                                    house_pos.x - half_w + (c as f32) * cell_size,
                                    y_base + 1.75,
                                    z_center,
                                ),
                                false,
                                &asset_server,
                                cell_size,
                            );
                        } else if cell_type == CellType::Bedroom && w_type == CellType::Bedroom {
                            spawn_solid_wall(
                                &mut commands,
                                &mut meshes,
                                &wall_mat,
                                Vec3::new(
                                    house_pos.x - half_w + (c as f32) * cell_size,
                                    y_base + 1.75,
                                    z_center,
                                ),
                                false,
                                cell_size,
                            );
                        }
                    }
                }

                // SOUTH BOUNDARY (outer boundary loop for row 3)
                if r == grid_rows - 1 {
                    if floor == 1 && c == 4 {
                        spawn_door_wall(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            &wall_mat,
                            Vec3::new(x_center, y_base + 1.75, house_pos.z + half_d),
                            true,
                            &asset_server,
                            cell_size,
                        );

                        // Front Entrance Patio Apron (flush with terrain and house floor)
                        let z_door = house_pos.z + half_d;
                        commands.spawn((
                            Mesh3d(meshes.add(Cuboid::new(5.0, 0.1, 3.0))),
                            MeshMaterial3d(floor_mat.clone()),
                            Transform::from_xyz(x_center, house_pos.y + 0.02, z_door + 1.5),
                            RigidBody::Static,
                            Collider::cuboid(5.0, 0.1, 3.0),
                            HouseMarker,
                            PlayModeEntity,
                        ));
                    } else if cell_type == CellType::Bedroom {
                        spawn_window_wall(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            &wall_mat,
                            Vec3::new(x_center, y_base + 1.75, house_pos.z + half_d),
                            true,
                            cell_size,
                        );
                    } else {
                        spawn_solid_wall(
                            &mut commands,
                            &mut meshes,
                            &wall_mat,
                            Vec3::new(x_center, y_base + 1.75, house_pos.z + half_d),
                            true,
                            cell_size,
                        );
                    }
                }

                // EAST BOUNDARY (outer boundary loop for col 7)
                if c == grid_cols - 1 {
                    if cell_type == CellType::Bedroom {
                        spawn_window_wall(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            &wall_mat,
                            Vec3::new(house_pos.x + half_w, y_base + 1.75, z_center),
                            false,
                            cell_size,
                        );
                    } else {
                        spawn_solid_wall(
                            &mut commands,
                            &mut meshes,
                            &wall_mat,
                            Vec3::new(house_pos.x + half_w, y_base + 1.75, z_center),
                            false,
                            cell_size,
                        );
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------
    // PUZZLE TRIGGERS, CHESTS, AND STAIRCASE TELEPORTERS
    // -----------------------------------------------------------------

    // Relocated Crimson puzzle trigger book inside northwest Ground Floor bedroom (cell 0,0)
    let nw_x = house_pos.x - half_w + cell_size * 0.5 - 1.0;
    let nw_z = house_pos.z - half_d + cell_size * 0.5 - 1.5;
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.15, 0.25, 0.35))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.75, 0.15, 0.20),
            perceptual_roughness: 0.7,
            emissive: LinearRgba::from(Color::srgb(0.2, 0.05, 0.05)),
            ..default()
        })),
        Transform::from_xyz(nw_x, house_pos.y + 1.0, nw_z),
        InteractiveBookcase,
        WallCollider {
            half_extents: Vec3::new(0.1, 0.15, 0.2),
        },
        HouseMarker,
        PlayModeEntity,
    ));

    // Stairs-up portal (Ground Floor -> First Floor) - placed at Col 0, Row 2 center
    let step_x = house_pos.x - half_w + cell_size * 0.5;
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.4).mesh().ico(3).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.1, 0.5, 0.8, 0.6),
            alpha_mode: AlphaMode::Blend,
            emissive: LinearRgba::from(Color::srgb(0.05, 0.2, 0.4)),
            ..default()
        })),
        Transform::from_xyz(step_x, house_pos.y + 0.7, house_pos.z),
        Teleporter {
            target_pos: Vec3::new(step_x, house_pos.y + 4.0, house_pos.z),
            message: "✨ Ascending to the gallery floor...".to_string(),
        },
        HouseMarker,
        PlayModeEntity,
    ));

    // Stairs-down portal (First Floor -> Ground Floor)
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.4).mesh().ico(3).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.1, 0.5, 0.8, 0.6),
            alpha_mode: AlphaMode::Blend,
            emissive: LinearRgba::from(Color::srgb(0.05, 0.2, 0.4)),
            ..default()
        })),
        Transform::from_xyz(step_x, house_pos.y + 4.2, house_pos.z),
        Teleporter {
            target_pos: Vec3::new(step_x, house_pos.y + 0.7, house_pos.z),
            message: "✨ Descending back to Ground Floor...".to_string(),
        },
        HouseMarker,
        PlayModeEntity,
    ));

    // -----------------------------------------------------------------
    // INDUSTRIAL ELEVATOR LIFT (Ground Floor Foyer -> Basement)
    // -----------------------------------------------------------------
    let portal_c = 4;
    let portal_r = 1;
    let foyer_portal_x = house_pos.x - half_w + (portal_c as f32 + 0.5) * cell_size;
    let foyer_portal_z = house_pos.z - half_d + (portal_r as f32 + 0.5) * cell_size;

    // Elevator Steel Deck Floor
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.4, 0.1, 2.4))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.25, 0.28, 0.32),
            metallic: 0.9,
            perceptual_roughness: 0.3,
            ..default()
        })),
        Transform::from_xyz(foyer_portal_x, house_pos.y + 0.05, foyer_portal_z),
        HouseMarker,
        PlayModeEntity,
    ));

    // Elevator Control Panel Console
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.3, 0.5, 0.1))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.1, 0.8, 0.4),
            emissive: LinearRgba::from(Color::srgb(0.2, 2.0, 1.0)),
            ..default()
        })),
        Transform::from_xyz(foyer_portal_x, house_pos.y + 0.9, foyer_portal_z),
        Teleporter {
            target_pos: Vec3::new(house_pos.x - 14.0, -49.0, house_pos.z + 5.0),
            message: "🛗 Elevator descending to Research Complex Basement...".to_string(),
        },
        HouseMarker,
        PlayModeEntity,
    ));

    // Gold Chest on First Floor
    let se_x = house_pos.x + half_w - 1.5;
    let se_z = house_pos.z + half_d - 2.5;
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.8, 0.6, 0.6))),
        MeshMaterial3d(gold_mat.clone()),
        Transform::from_xyz(se_x, house_pos.y + 3.65, se_z),
        PuzzleChest { is_locked: true },
        WallCollider {
            half_extents: Vec3::new(0.4, 0.3, 0.3),
        },
        HouseMarker,
        PlayModeEntity,
    ));

    // -----------------------------------------------------------------
    // FLOOR 3: BASEMENT (spawns underground at Y = -50.0)
    // -----------------------------------------------------------------
    let basement_w = (grid_cols as f32 * cell_size) + 4.0;
    let basement_d = (grid_rows as f32 * cell_size) + 4.0;
    let bf_size = Vec3::new(basement_w, 0.1, basement_d);
    commands.spawn((
        Mesh3d(meshes.add(create_world_uv_cuboid(bf_size, 3.0))),
        MeshMaterial3d(basement_floor_mat.clone()),
        Transform::from_xyz(house_pos.x, -50.0, house_pos.z),
        RigidBody::Static,
        Collider::cuboid(basement_w, 0.1, basement_d),
        HouseMarker,
        PlayModeEntity,
    ));

    // Ceiling
    commands.spawn((
        Mesh3d(meshes.add(create_world_uv_cuboid(bf_size, 3.0))),
        MeshMaterial3d(basement_floor_mat.clone()),
        Transform::from_xyz(house_pos.x, -46.0, house_pos.z),
        RigidBody::Static,
        Collider::cuboid(basement_w, 0.1, basement_d),
        HouseMarker,
        PlayModeEntity,
    ));

    // Outer Walls
    let bw_size = Vec3::new(basement_w, 4.0, 0.2);
    let bd_size = Vec3::new(0.2, 4.0, basement_d);

    commands.spawn((
        Mesh3d(meshes.add(create_world_uv_cuboid(bw_size, 3.0))),
        MeshMaterial3d(basement_wall_mat.clone()),
        Transform::from_xyz(house_pos.x, -48.0, house_pos.z - (basement_d * 0.5)),
        WallCollider {
            half_extents: Vec3::new(basement_w * 0.5, 2.0, 0.1),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    commands.spawn((
        Mesh3d(meshes.add(create_world_uv_cuboid(bw_size, 3.0))),
        MeshMaterial3d(basement_wall_mat.clone()),
        Transform::from_xyz(house_pos.x, -48.0, house_pos.z + (basement_d * 0.5)),
        WallCollider {
            half_extents: Vec3::new(basement_w * 0.5, 2.0, 0.1),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    commands.spawn((
        Mesh3d(meshes.add(create_world_uv_cuboid(bd_size, 3.0))),
        MeshMaterial3d(basement_wall_mat.clone()),
        Transform::from_xyz(house_pos.x + (basement_w * 0.5), -48.0, house_pos.z),
        WallCollider {
            half_extents: Vec3::new(0.1, 2.0, basement_d * 0.5),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    commands.spawn((
        Mesh3d(meshes.add(create_world_uv_cuboid(bd_size, 3.0))),
        MeshMaterial3d(basement_wall_mat.clone()),
        Transform::from_xyz(house_pos.x - (basement_w * 0.5), -48.0, house_pos.z),
        WallCollider {
            half_extents: Vec3::new(0.1, 2.0, basement_d * 0.5),
        },
        HouseMarker,
        PlayModeEntity,
    ));

    // -----------------------------------------------------------------
    // BASEMENT ELEVATOR LIFT (Spacious SW Bay -> Ground Floor Foyer)
    // -----------------------------------------------------------------
    let basement_lift_x = house_pos.x - 14.0;
    let basement_lift_z = house_pos.z + 5.0;

    // High-Tech Industrial Elevator Shaft Wall Frame
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(3.0, 4.0, 0.2))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.22, 0.26),
            metallic: 0.95,
            perceptual_roughness: 0.2,
            ..default()
        })),
        Transform::from_xyz(basement_lift_x, -48.0, basement_lift_z + 1.2),
        HouseMarker,
        PlayModeEntity,
    ));

    // Steel Lift Deck
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.4, 0.1, 2.4))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.25, 0.28, 0.32),
            metallic: 0.9,
            perceptual_roughness: 0.3,
            ..default()
        })),
        Transform::from_xyz(basement_lift_x, -49.95, basement_lift_z),
        HouseMarker,
        PlayModeEntity,
    ));

    // Elevator Control Panel Console
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.3, 0.5, 0.1))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.1, 0.8, 0.4),
            emissive: LinearRgba::from(Color::srgb(0.2, 2.0, 1.0)),
            ..default()
        })),
        Transform::from_xyz(basement_lift_x, -49.1, basement_lift_z),
        Teleporter {
            target_pos: Vec3::new(foyer_portal_x, house_pos.y + 0.1, foyer_portal_z + 3.0),
            message: "🛗 Elevator ascending to Ground Floor Foyer...".to_string(),
        },
        HouseMarker,
        PlayModeEntity,
    ));

    // Three pressure plates
    let plate_positions = [
        Vec3::new(house_pos.x - 6.0, -49.9, house_pos.z - 5.0),
        Vec3::new(house_pos.x, -49.9, house_pos.z - 7.0),
        Vec3::new(house_pos.x + 6.0, -49.9, house_pos.z - 5.0),
    ];
    for (i, pos) in plate_positions.iter().enumerate() {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(1.2, 0.08, 1.2))),
            MeshMaterial3d(red_glow_mat.clone()),
            Transform::from_translation(*pos),
            PressurePlate {
                id: i as u32,
                triggered: false,
            },
            HouseMarker,
            PlayModeEntity,
        ));
    }

    // Three metal crates
    let crate_positions = [
        Vec3::new(house_pos.x - 4.0, -49.5, house_pos.z + 4.0),
        Vec3::new(house_pos.x, -49.5, house_pos.z + 5.0),
        Vec3::new(house_pos.x + 4.0, -49.5, house_pos.z + 4.0),
    ];
    for pos in crate_positions.iter() {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.45, 0.48, 0.52),
                metallic: 0.85,
                perceptual_roughness: 0.35,
                ..default()
            })),
            Transform::from_translation(*pos),
            PushableCrate { target_pos: *pos },
            WallCollider {
                half_extents: Vec3::new(0.5, 0.5, 0.5),
            },
            HouseMarker,
            PlayModeEntity,
        ));
    }

    // Vault partition wall separating cellar and Crypt ladder room
    let vp1_size = Vec3::new(8.0, 4.0, 0.2);
    commands.spawn((
        Mesh3d(meshes.add(create_world_uv_cuboid(vp1_size, 3.0))),
        MeshMaterial3d(basement_wall_mat.clone()),
        Transform::from_xyz(house_pos.x - 6.0, -48.0, house_pos.z),
        WallCollider {
            half_extents: Vec3::new(4.0, 2.0, 0.1),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    commands.spawn((
        Mesh3d(meshes.add(create_world_uv_cuboid(vp1_size, 3.0))),
        MeshMaterial3d(basement_wall_mat.clone()),
        Transform::from_xyz(house_pos.x + 6.0, -48.0, house_pos.z),
        WallCollider {
            half_extents: Vec3::new(4.0, 2.0, 0.1),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    let vp2_size = Vec3::new(4.0, 1.2, 0.2);
    commands.spawn((
        Mesh3d(meshes.add(create_world_uv_cuboid(vp2_size, 3.0))),
        MeshMaterial3d(basement_wall_mat.clone()),
        Transform::from_xyz(house_pos.x, -46.6, house_pos.z),
        WallCollider {
            half_extents: Vec3::new(2.0, 0.6, 0.1),
        },
        HouseMarker,
        PlayModeEntity,
    ));

    // Sliding metal gate
    let gate_closed = Vec3::new(house_pos.x, -48.6, house_pos.z);
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.8, 2.8, 0.1))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.5, 0.5, 0.5),
            metallic: 0.9,
            perceptual_roughness: 0.1,
            ..default()
        })),
        Transform::from_translation(gate_closed),
        CellGate {
            start_pos: gate_closed,
            target_pos: gate_closed,
        },
        WallCollider {
            half_extents: Vec3::new(0.9, 1.4, 0.05),
        },
        HouseMarker,
        PlayModeEntity,
    ));

    // Ladder Teleporter down to sub-basement
    let ladder_z = house_pos.z - 5.0;
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.4).mesh().ico(3).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.8, 0.1, 0.8, 0.6),
            alpha_mode: AlphaMode::Blend,
            emissive: LinearRgba::from(Color::srgb(0.4, 0.05, 0.4)),
            ..default()
        })),
        Transform::from_xyz(house_pos.x, -48.2, ladder_z),
        Teleporter {
            target_pos: Vec3::new(house_pos.x, -100.0, house_pos.z + 2.0),
            message: "🕯️ Descending into the Ancient Crypt...".to_string(),
        },
        HouseMarker,
        PlayModeEntity,
    ));

    // -----------------------------------------------------------------
    // FLOOR 4: SUB-BASEMENT (spawns deep underground at Y = -100.0)
    // -----------------------------------------------------------------

    // Floor
    let sb_f_size = Vec3::new(10.0, 0.1, 10.0);
    commands.spawn((
        Mesh3d(meshes.add(create_world_uv_cuboid(sb_f_size, 3.0))),
        MeshMaterial3d(basement_floor_mat.clone()),
        Transform::from_xyz(house_pos.x, -100.0, house_pos.z),
        RigidBody::Static,
        Collider::cuboid(10.0, 0.1, 10.0),
        HouseMarker,
        PlayModeEntity,
    ));

    // Ceiling
    commands.spawn((
        Mesh3d(meshes.add(create_world_uv_cuboid(sb_f_size, 3.0))),
        MeshMaterial3d(basement_floor_mat.clone()),
        Transform::from_xyz(house_pos.x, -96.0, house_pos.z),
        RigidBody::Static,
        Collider::cuboid(10.0, 0.1, 10.0),
        HouseMarker,
        PlayModeEntity,
    ));

    // Outer Walls
    let sb_w1_size = Vec3::new(10.0, 4.0, 0.2);
    let sb_w2_size = Vec3::new(0.2, 4.0, 10.0);

    commands.spawn((
        Mesh3d(meshes.add(create_world_uv_cuboid(sb_w1_size, 3.0))),
        MeshMaterial3d(sub_basement_mat.clone()),
        Transform::from_xyz(house_pos.x, -98.0, house_pos.z - 5.0),
        WallCollider {
            half_extents: Vec3::new(5.0, 2.0, 0.1),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    commands.spawn((
        Mesh3d(meshes.add(create_world_uv_cuboid(sb_w1_size, 3.0))),
        MeshMaterial3d(sub_basement_mat.clone()),
        Transform::from_xyz(house_pos.x, -98.0, house_pos.z + 5.0),
        WallCollider {
            half_extents: Vec3::new(5.0, 2.0, 0.1),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    commands.spawn((
        Mesh3d(meshes.add(create_world_uv_cuboid(sb_w2_size, 3.0))),
        MeshMaterial3d(sub_basement_mat.clone()),
        Transform::from_xyz(house_pos.x + 5.0, -98.0, house_pos.z),
        WallCollider {
            half_extents: Vec3::new(0.1, 2.0, 5.0),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    commands.spawn((
        Mesh3d(meshes.add(create_world_uv_cuboid(sb_w2_size, 3.0))),
        MeshMaterial3d(sub_basement_mat.clone()),
        Transform::from_xyz(house_pos.x - 5.0, -98.0, house_pos.z),
        WallCollider {
            half_extents: Vec3::new(0.1, 2.0, 5.0),
        },
        HouseMarker,
        PlayModeEntity,
    ));

    // Ladder Teleporter back up
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.4).mesh().ico(3).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.8, 0.1, 0.8, 0.6),
            alpha_mode: AlphaMode::Blend,
            emissive: LinearRgba::from(Color::srgb(0.4, 0.05, 0.4)),
            ..default()
        })),
        Transform::from_xyz(house_pos.x, -98.2, house_pos.z - 3.5),
        Teleporter {
            target_pos: Vec3::new(house_pos.x, -50.0, ladder_z + 2.0),
            message: "🪜 Climbing back up to basement cellar...".to_string(),
        },
        HouseMarker,
        PlayModeEntity,
    ));

    // Pedestal
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.35, 1.2))),
        MeshMaterial3d(basement_wall_mat.clone()),
        Transform::from_xyz(house_pos.x, -99.4, house_pos.z),
        ArtifactPedestal,
        WallCollider {
            half_extents: Vec3::new(0.35, 0.6, 0.35),
        },
        HouseMarker,
        PlayModeEntity,
    ));

    // Ancient Artifact
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.3).mesh().ico(3).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.8, 1.0),
            emissive: LinearRgba::from(Color::srgb(1.0, 5.0, 8.0)),
            perceptual_roughness: 0.1,
            ..default()
        })),
        Transform::from_xyz(house_pos.x, -98.4, house_pos.z),
        RotatingArtifact,
        HouseMarker,
        PlayModeEntity,
    ));

    // -----------------------------------------------------------------
    // GRAND LIMESTONE STAIRCASE IN FOYER (Col 3, Row 2 & Row 1)
    // -----------------------------------------------------------------
    let staircase_x = house_pos.x - half_w + cell_size * 3.5;
    let num_steps = 20;
    let total_depth = cell_size * 2.0; // Spans two grid cells (10.0m) to reach second floor walkway
    let step_height = 3.5 / num_steps as f32; // 0.175m per step
    let step_depth = total_depth / num_steps as f32; // 0.5m depth per step
    let step_width = 1.8;

    let start_z = house_pos.z - half_d + 3.0 * cell_size;

    // Spawn visual steps (no individual physics colliders to prevent getting stuck)
    for step_idx in 0..num_steps {
        let step_y = house_pos.y + (step_idx as f32) * step_height + step_height * 0.5;
        // Start from start_z (bottom, South foyer) and go up to start_z - total_depth (top, North bedroom/hallway boundary)
        let step_z = start_z - (step_idx as f32) * step_depth - step_depth * 0.5;

        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(step_width, step_height, step_depth))),
            MeshMaterial3d(floor_mat.clone()),
            Transform::from_xyz(staircase_x, step_y, step_z),
            HouseMarker,
            PlayModeEntity,
        ));
    }

    // Spawn a single smooth tilted ramp collider matching step width (thickness 0.2m)
    let pitch = (3.5f32 / total_depth).atan();
    let slope_len = (total_depth.powi(2) + 3.5f32.powi(2)).sqrt();
    let local_y = Quat::from_rotation_x(pitch) * Vec3::Y;
    let center_z = start_z - total_depth * 0.5;
    let center_pos = Vec3::new(staircase_x, house_pos.y + 1.75, center_z) - local_y * 0.1;

    commands.spawn((
        Transform::from_translation(center_pos).with_rotation(Quat::from_rotation_x(pitch)),
        RigidBody::Static,
        Collider::cuboid(step_width, 0.2, slope_len),
        HouseMarker,
        PlayModeEntity,
    ));

    // -----------------------------------------------------------------
    // MANSION PITCHED ROOF (using roof_shingles.png)
    // -----------------------------------------------------------------
    let roof_w = grid_cols as f32 * cell_size + 1.0;
    let roof_slope_len = half_d / 0.20_f32.cos() + 1.0;
    let mansion_roof_mesh = meshes.add(Cuboid::new(roof_w, 0.1, roof_slope_len));
    let mansion_roof_mat = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/roof_shingles.png")),
        perceptual_roughness: 0.9,
        ..default()
    });
    let roof_y = house_pos.y + 7.0 + (half_d * 0.5) * 0.20_f32.tan();

    // North slope
    commands.spawn((
        Mesh3d(mansion_roof_mesh.clone()),
        MeshMaterial3d(mansion_roof_mat.clone()),
        Transform::from_xyz(house_pos.x, roof_y, house_pos.z - half_d * 0.5)
            .with_rotation(Quat::from_rotation_x(-0.20)), // pitch up towards center
        RigidBody::Static,
        Collider::cuboid(roof_w, 0.1, roof_slope_len),
        HouseMarker,
        PlayModeEntity,
    ));

    // South slope
    commands.spawn((
        Mesh3d(mansion_roof_mesh),
        MeshMaterial3d(mansion_roof_mat),
        Transform::from_xyz(house_pos.x, roof_y, house_pos.z + half_d * 0.5)
            .with_rotation(Quat::from_rotation_x(0.20)), // pitch up towards center
        RigidBody::Static,
        Collider::cuboid(roof_w, 0.1, roof_slope_len),
        HouseMarker,
        PlayModeEntity,
    ));

    // -----------------------------------------------------------------
    // BASEMENT LIGHTING: 4 CORNER LANTERNS PER LEVEL
    // -----------------------------------------------------------------
    let basement_w = (grid_cols as f32 * cell_size) + 4.0;
    let basement_d = (grid_rows as f32 * cell_size) + 4.0;

    // Floor 3 (Basement at Y = -50.0, ceiling at Y = -46.0)
    spawn_lantern(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(
            house_pos.x - basement_w * 0.4,
            -47.2,
            house_pos.z - basement_d * 0.4,
        ),
    );
    spawn_lantern(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(
            house_pos.x + basement_w * 0.4,
            -47.2,
            house_pos.z - basement_d * 0.4,
        ),
    );
    spawn_lantern(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(
            house_pos.x - basement_w * 0.4,
            -47.2,
            house_pos.z + basement_d * 0.4,
        ),
    );
    spawn_lantern(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(
            house_pos.x + basement_w * 0.4,
            -47.2,
            house_pos.z + basement_d * 0.4,
        ),
    );

    // Floor 4 (Sub-basement at Y = -100.0, ceiling at Y = -96.0)
    spawn_lantern(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(
            house_pos.x - basement_w * 0.4,
            -97.2,
            house_pos.z - basement_d * 0.4,
        ),
    );
    spawn_lantern(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(
            house_pos.x + basement_w * 0.4,
            -97.2,
            house_pos.z - basement_d * 0.4,
        ),
    );
    spawn_lantern(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(
            house_pos.x - basement_w * 0.4,
            -97.2,
            house_pos.z + basement_d * 0.4,
        ),
    );
    spawn_lantern(
        &mut commands,
        &mut meshes,
        &mut materials,
        Vec3::new(
            house_pos.x + basement_w * 0.4,
            -97.2,
            house_pos.z + basement_d * 0.4,
        ),
    );

    spawn_research_complex_decorations(
        &mut commands,
        &mut meshes,
        &mut materials,
        &asset_server,
        house_pos,
        half_w,
        half_d,
        cell_size,
    );
}

fn bookcase_slide_system(
    time: Res<Time>,
    puzzle_state: Res<HousePuzzleState>,
    mut query: Query<(&mut Transform, &mut BookcaseDoor)>,
) {
    let dt = time.delta_secs();
    for (mut transform, mut bookcase) in query.iter_mut() {
        let target = if puzzle_state.bookcase_opened {
            bookcase.start_pos + Vec3::new(0.0, 0.0, -2.2)
        } else {
            bookcase.start_pos
        };
        bookcase.target_pos = target;
        transform.translation = transform.translation.lerp(bookcase.target_pos, 4.0 * dt);
    }
}

fn door_swing_system(
    time: Res<Time>,
    player_query: Query<&Transform, With<PlayModePlayer>>,
    mut door_query: Query<
        (Entity, &mut Transform, &mut HouseDoor, Option<&Children>),
        Without<PlayModePlayer>,
    >,
    children_query: Query<&Children>,
) {
    let dt = time.delta_secs();
    let player_pos = player_query
        .iter()
        .next()
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    // Pre-pass: collect auto-open triggers for parent hinges near player
    let mut auto_open_parents = Vec::new();
    for (parent_entity, transform, door, children_opt) in door_query.iter() {
        if children_opt.is_some() {
            let dist = player_pos.xz().distance(transform.translation.xz());
            let dy = (player_pos.y - transform.translation.y).abs();
            if dist < 2.8 && dy < 2.5 && !door.is_open {
                auto_open_parents.push(parent_entity);
            }
        }
    }

    for parent_entity in auto_open_parents {
        if let Ok((_, _, mut door, _)) = door_query.get_mut(parent_entity) {
            door.is_open = true;
        }
    }

    // Pass 1: Rotate parent hinge transforms (With<Children>)
    let mut parent_states = Vec::new();
    for (entity, mut transform, door, children_opt) in door_query.iter_mut() {
        if children_opt.is_some() {
            let target_rot = if door.is_open {
                door.open_rot
            } else {
                door.closed_rot
            };
            transform.rotation = transform.rotation.slerp(target_rot, 5.0 * dt);
            parent_states.push((entity, door.is_open));
        }
    }

    // Pass 2: Sync is_open state to child door components
    for (parent_entity, is_open) in parent_states {
        if let Ok(children) = children_query.get(parent_entity) {
            for child_entity in children.iter() {
                if let Ok((_, _, mut child_door, _)) = door_query.get_mut(child_entity) {
                    child_door.is_open = is_open;
                }
            }
        }
    }
}

fn gate_slide_system(
    time: Res<Time>,
    puzzle_state: Res<HousePuzzleState>,
    mut query: Query<(&mut Transform, &mut CellGate)>,
) {
    let dt = time.delta_secs();
    for (mut transform, mut gate) in query.iter_mut() {
        let target = if puzzle_state.basement_solved {
            gate.start_pos + Vec3::new(0.0, 3.6, 0.0)
        } else {
            gate.start_pos
        };
        gate.target_pos = target;
        transform.translation = transform.translation.lerp(gate.target_pos, 4.0 * dt);
    }
}

fn crate_movement_system(time: Res<Time>, mut query: Query<(&mut Transform, &PushableCrate)>) {
    let dt = time.delta_secs();
    for (mut transform, krate) in query.iter_mut() {
        transform.translation = transform.translation.lerp(krate.target_pos, 8.0 * dt);
    }
}

fn pressure_plate_system(
    mut query: Query<(&Transform, &mut PressurePlate)>,
    crate_query: Query<&Transform, With<PushableCrate>>,
    player_query: Query<&Transform, With<PlayModePlayer>>,
    mut puzzle_state: ResMut<HousePuzzleState>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let mut current_solved = true;

    for (plate_transform, mut plate) in query.iter_mut() {
        let mut is_occupied = false;

        let d_player = plate_transform
            .translation
            .xz()
            .distance(player_transform.translation.xz());
        let dy_player = (plate_transform.translation.y - player_transform.translation.y).abs();
        if d_player < 0.85 && dy_player < 1.0 {
            is_occupied = true;
        }

        for crate_transform in crate_query.iter() {
            let d_crate = plate_transform
                .translation
                .xz()
                .distance(crate_transform.translation.xz());
            let dy_crate = (plate_transform.translation.y - crate_transform.translation.y).abs();
            if d_crate < 0.65 && dy_crate < 1.0 {
                is_occupied = true;
            }
        }

        plate.triggered = is_occupied;

        if !plate.triggered {
            current_solved = false;
        }
    }

    if current_solved && !puzzle_state.basement_solved {
        puzzle_state.basement_solved = true;
        crate::play_mode::inventory_log("🔓 Click! The vault gate slides open!");
    }
}

fn pedestal_glow_system(time: Res<Time>, mut query: Query<&mut Transform, With<RotatingArtifact>>) {
    let elapsed = time.elapsed_secs();
    for mut transform in query.iter_mut() {
        transform.rotation =
            Quat::from_rotation_y(elapsed * 1.5) * Quat::from_rotation_x(elapsed * 0.5);
    }
}
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn house_interaction_system(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<(Entity, &mut PlayModePlayer, &mut Transform, &mut Position)>,
    mut puzzle_state: ResMut<HousePuzzleState>,
    mut inventory: ResMut<PlayerInventory>,
    mut door_query: Query<(Entity, &mut HouseDoor, &Transform), Without<PlayModePlayer>>,
    bookcase_query: Query<&Transform, (With<InteractiveBookcase>, Without<PlayModePlayer>)>,
    teleporter_query: Query<(&Transform, &Teleporter), Without<PlayModePlayer>>,
    mut crate_query: Query<(Entity, &mut PushableCrate)>,
    mut chest_query: Query<(Entity, &mut PuzzleChest, &Transform), Without<PlayModePlayer>>,
    artifact_query: Query<(Entity, &Transform), (With<RotatingArtifact>, Without<PlayModePlayer>)>,
    terminal_query: Query<(&Transform, &ResearchTerminal), Without<PlayModePlayer>>,
    security_query: Query<&Transform, (With<BasementSecurityConsole>, Without<PlayModePlayer>)>,
    synth_query: Query<&Transform, (With<PlasmaSynthesizerConsole>, Without<PlayModePlayer>)>,
    map: Res<TempestMap>,
    mansion_settings: Res<MansionSettings>,
) {
    if !keyboard_input.just_pressed(KeyCode::KeyE) {
        return;
    }

    // If a modal or terminal log window is open, pressing KeyE closes it!
    if puzzle_state.active_terminal_log.is_some()
        || puzzle_state.show_security_keypad
        || puzzle_state.show_synthesizer_ui
    {
        puzzle_state.active_terminal_log = None;
        puzzle_state.show_security_keypad = false;
        puzzle_state.show_synthesizer_ui = false;
        return;
    }
    let Ok((_player_entity, mut player, mut player_transform, mut phys_pos)) =
        player_query.single_mut()
    else {
        return;
    };
    let player_pos = player.position;

    // 0a. Interacting with Research Expedition Terminals
    for (t_transform, terminal) in terminal_query.iter() {
        let d = player_pos.xz().distance(t_transform.translation.xz());
        let dy = (player_pos.y - t_transform.translation.y).abs();
        if d < 2.5 && dy < 2.5 {
            puzzle_state.active_terminal_log = Some(terminal.terminal_id);
            crate::play_mode::inventory_log(&format!("💻 Accessing Terminal: {}", terminal.title));
            return;
        }
    }

    // 0b. Interacting with Basement Security Keypad Console
    for s_transform in security_query.iter() {
        let d = player_pos.xz().distance(s_transform.translation.xz());
        let dy = (player_pos.y - s_transform.translation.y).abs();
        if d < 2.5 && dy < 2.5 {
            if puzzle_state.vault_unlocked {
                crate::play_mode::inventory_log("🔓 Basement Vault Security is disengaged.");
            } else {
                puzzle_state.show_security_keypad = true;
                puzzle_state.keypad_input.clear();
                crate::play_mode::inventory_log(
                    "🔒 Accessing Basement Security Override Keypad...",
                );
            }
            return;
        }
    }

    // 0c. Interacting with Plasma Synthesizer Station
    for synth_transform in synth_query.iter() {
        let d = player_pos.xz().distance(synth_transform.translation.xz());
        let dy = (player_pos.y - synth_transform.translation.y).abs();
        if d < 2.5 && dy < 2.5 {
            if puzzle_state.vault_unlocked {
                puzzle_state.show_synthesizer_ui = true;
                crate::play_mode::inventory_log("🔬 Accessing Plasma Synthesizer Station...");
            } else {
                crate::play_mode::inventory_log(
                    "🔒 Plasma Synthesizer offline — Vault door is locked!",
                );
            }
            return;
        }
    }

    // 1. Interacting with closest door within 2.2m horizontal radius and 2.5m height tolerance
    let mut closest_door = None;
    let mut min_d = 2.2;
    for (_entity, door, transform) in door_query.iter_mut() {
        let d = player_pos.xz().distance(transform.translation.xz());
        let dy = (player_pos.y - transform.translation.y).abs();
        if d < min_d && dy < 2.5 {
            min_d = d;
            closest_door = Some(door);
        }
    }
    if let Some(mut door) = closest_door {
        door.is_open = !door.is_open;
        if door.is_open {
            crate::play_mode::inventory_log("🚪 Door opened!");
        } else {
            crate::play_mode::inventory_log("🚪 Door closed!");
        }
        return;
    }

    // 2. Interacting with hidden bookcase book
    for bc_transform in bookcase_query.iter() {
        let d = player_pos.xz().distance(bc_transform.translation.xz());
        let dy = (player_pos.y - bc_transform.translation.y).abs();
        if d < 2.0 && dy < 2.5 {
            puzzle_state.bookcase_opened = !puzzle_state.bookcase_opened;
            if puzzle_state.bookcase_opened {
                crate::play_mode::inventory_log(
                    "📚 Grabbing crimson book... A secret bookcase slides open!",
                );
            } else {
                crate::play_mode::inventory_log(
                    "📚 Pushing book back... The bookcase door closes.",
                );
            }
            return;
        }
    }

    // 3. Interacting with Gold Chest
    let mut closest_chest = None;
    let mut min_chest_d = 2.2;
    for (_entity, chest, transform) in chest_query.iter_mut() {
        let d = player_pos.xz().distance(transform.translation.xz());
        let dy = (player_pos.y - transform.translation.y).abs();
        if d < min_chest_d && dy < 2.5 {
            min_chest_d = d;
            closest_chest = Some(chest);
        }
    }
    if let Some(mut chest) = closest_chest {
        if chest.is_locked {
            chest.is_locked = false;
            inventory.wood += 5;
            inventory.rock += 5;
            crate::play_mode::inventory_log("🔑 Opened Gold Chest! Found 5 Wood and 5 Stone!");
        } else {
            crate::play_mode::inventory_log("📭 The chest is empty.");
        }
        return;
    }

    // 4. Teleporting (stairs/ladders)
    for (tel_transform, tel) in teleporter_query.iter() {
        let d = player_pos.xz().distance(tel_transform.translation.xz());
        let dy = (player_pos.y - tel_transform.translation.y).abs();
        if d < 1.8 && dy < 2.5 {
            player.position = tel.target_pos;
            let float_height = player.height * 0.5 + 0.08;
            let new_phys_pos = tel.target_pos + Vec3::Y * float_height;
            player_transform.translation = new_phys_pos;
            phys_pos.0 = new_phys_pos;

            for n in player.nodes.iter_mut() {
                let diff = tel.target_pos - player_pos;
                n.position += diff;
                n.old_position += diff;
            }
            crate::play_mode::inventory_log(&tel.message);
            return;
        }
    }

    // 5. Pushing/pulling metal crates
    let mut closest_crate = None;
    let mut min_crate_d = 2.0;
    for (entity, krate) in crate_query.iter_mut() {
        let d = player_pos.xz().distance(krate.target_pos.xz());
        let dy = (player_pos.y - krate.target_pos.y).abs();
        if d < min_crate_d && dy < 2.5 {
            min_crate_d = d;
            closest_crate = Some((entity, krate));
        }
    }
    if let Some((_, mut krate)) = closest_crate {
        let yaw = player.rotation_yaw;
        let forward = Vec3::new(yaw.cos(), 0.0, yaw.sin());

        let push_dir = if forward.x.abs() > forward.z.abs() {
            Vec3::new(forward.x.signum() * 1.5, 0.0, 0.0)
        } else {
            Vec3::new(0.0, 0.0, forward.z.signum() * 1.5)
        };

        let new_pos = krate.target_pos + push_dir;

        // Heavy metal crate limits matching dynamic mansion settings
        let mut house_pos = Vec3::new(0.0, 1.5, 0.0);
        for p in map.prefabs.iter() {
            if p.prefab_type == "house" {
                house_pos = Vec3::from_array(p.position);
                break;
            }
        }

        let border_limit_x =
            (mansion_settings.cols as f32 * mansion_settings.cell_size) * 0.5 - 1.0;
        let border_limit_z =
            (mansion_settings.rows as f32 * mansion_settings.cell_size) * 0.5 - 1.0;

        let relative_pos = new_pos - Vec3::new(house_pos.x, -50.0, house_pos.z);
        if relative_pos.x.abs() < border_limit_x && relative_pos.z.abs() < border_limit_z {
            krate.target_pos = new_pos;
            crate::play_mode::inventory_log("🔩 Pushed heavy metal crate!");
        } else {
            crate::play_mode::inventory_log("🚧 Can't push crate further!");
        }
        return;
    }

    // 6. Collecting the ancient artifact
    for (entity, art_transform) in artifact_query.iter() {
        let d = player_pos.xz().distance(art_transform.translation.xz());
        let dy = (player_pos.y - art_transform.translation.y).abs();
        if d < 2.0 && dy < 2.5 {
            commands.entity(entity).despawn();
            puzzle_state.artifact_collected = true;
            crate::play_mode::inventory_log(
                "🏆 YOU FOUND THE ANCIENT ARTIFACT! EXPLORATION COMPLETE!",
            );
            return;
        }
    }
}

pub fn vault_door_unlock_system(
    mut commands: Commands,
    puzzle_state: Res<HousePuzzleState>,
    vault_door_query: Query<Entity, With<BasementVaultDoor>>,
) {
    if puzzle_state.vault_unlocked {
        for entity in vault_door_query.iter() {
            commands.entity(entity).despawn();
        }
    }
}

pub fn research_complex_ui_system(
    mut contexts: bevy_egui::EguiContexts,
    mut puzzle_state: ResMut<HousePuzzleState>,
    mut inventory: ResMut<PlayerInventory>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape)
        && (puzzle_state.active_terminal_log.is_some()
            || puzzle_state.show_security_keypad
            || puzzle_state.show_synthesizer_ui)
    {
        puzzle_state.active_terminal_log = None;
        puzzle_state.show_security_keypad = false;
        puzzle_state.show_synthesizer_ui = false;
        return;
    }

    // Handle physical keyboard digit entry when Security Keypad is active
    if puzzle_state.show_security_keypad && !puzzle_state.vault_unlocked {
        let digits = [
            (KeyCode::Digit0, '0'),
            (KeyCode::Numpad0, '0'),
            (KeyCode::Digit1, '1'),
            (KeyCode::Numpad1, '1'),
            (KeyCode::Digit2, '2'),
            (KeyCode::Numpad2, '2'),
            (KeyCode::Digit3, '3'),
            (KeyCode::Numpad3, '3'),
            (KeyCode::Digit4, '4'),
            (KeyCode::Numpad4, '4'),
            (KeyCode::Digit5, '5'),
            (KeyCode::Numpad5, '5'),
            (KeyCode::Digit6, '6'),
            (KeyCode::Numpad6, '6'),
            (KeyCode::Digit7, '7'),
            (KeyCode::Numpad7, '7'),
            (KeyCode::Digit8, '8'),
            (KeyCode::Numpad8, '8'),
            (KeyCode::Digit9, '9'),
            (KeyCode::Numpad9, '9'),
        ];

        for (key, ch) in digits {
            if keyboard_input.just_pressed(key) && puzzle_state.keypad_input.len() < 3 {
                puzzle_state.keypad_input.push(ch);
            }
        }

        if keyboard_input.just_pressed(KeyCode::Backspace) {
            puzzle_state.keypad_input.pop();
        }

        if keyboard_input.just_pressed(KeyCode::Enter)
            || keyboard_input.just_pressed(KeyCode::NumpadEnter)
        {
            if puzzle_state.keypad_input == "371" {
                puzzle_state.vault_unlocked = true;
                puzzle_state.show_security_keypad = false;
                crate::play_mode::inventory_log(
                    "🔓 ACCESS GRANTED! Passcode 371 Accepted! Security Vault Blast Doors disengaged!",
                );
            } else if !puzzle_state.keypad_input.is_empty() {
                crate::play_mode::inventory_log("❌ ACCESS DENIED: Invalid Passcode!");
                puzzle_state.keypad_input.clear();
            }
        }
    }

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    // 1. Terminal Log Window
    if let Some(log_id) = puzzle_state.active_terminal_log {
        let (title, text) = match log_id {
            1 => (
                "🔬 EXPEDITION LOG #01: ATMOSPHERIC INTERFERENCE",
                "Survey Team Alpha - Log 104:\nAtmospheric density on this world is 4x higher than expected. Electromagnetic storms disrupt orbital communication. We have established Research Complex Alpha at grid coordinate West-Edge. Deep mineral scans indicate bioluminescent crystal deposits in the lower cave system.",
            ),
            2 => (
                "🧪 RESEARCH LOG #02: FAUNA & CRYSTAL SYNTHESIS",
                "Dr. Vance Notes:\nThe native alien species is non-hostile when unprovoked. They possess advanced trade tech. We built a Plasma Synthesizer in the Basement Vault to refine raw surface minerals into tech modules. Security Override Code is recorded in the Basement Server.",
            ),
            _ => (
                "🔒 BASEMENT SECURITY OVERRIDE PASSCODE",
                "SECURITY ALERT:\nBasement Vault locked due to grid overload. Emergency Manual Keypad Override Passcode: [ 3 - 7 - 1 ]. Enter passcode on the Security Keypad to disengage magnetic locks and access the Plasma Synthesizer.",
            ),
        };

        let mut is_open = true;
        egui::Window::new(title)
            .default_width(420.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .collapsible(false)
            .resizable(false)
            .open(&mut is_open)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(text)
                        .size(15.0)
                        .color(egui::Color32::from_rgb(180, 230, 255)),
                );
                ui.add_space(14.0);
                if ui
                    .add_sized(
                        [ui.available_width(), 32.0],
                        egui::Button::new(
                            egui::RichText::new("✖ Close Terminal Log (ESC)")
                                .size(15.0)
                                .strong()
                                .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(180, 40, 40)),
                    )
                    .clicked()
                {
                    puzzle_state.active_terminal_log = None;
                }
            });
        if !is_open {
            puzzle_state.active_terminal_log = None;
        }
    }

    // 2. Security Keypad Window
    if puzzle_state.show_security_keypad && !puzzle_state.vault_unlocked {
        let mut is_open = true;
        egui::Window::new("🔒 BASEMENT SECURITY KEYPAD OVERRIDE")
            .default_width(320.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .collapsible(false)
            .resizable(false)
            .open(&mut is_open)
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Type or Press 3-Digit Passcode (Clue: Terminal #03)").strong().color(egui::Color32::from_rgb(200, 220, 255)));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "Input: [ {:^3} ]",
                            if puzzle_state.keypad_input.is_empty() {
                                "___"
                            } else {
                                &puzzle_state.keypad_input
                            }
                        ))
                        .size(24.0)
                        .strong()
                        .color(egui::Color32::from_rgb(255, 200, 80)),
                    );
                });
                ui.add_space(10.0);

                // 3x4 On-Screen Keypad Buttons Grid
                egui::Grid::new("keypad_num_grid").spacing([10.0, 8.0]).show(ui, |ui| {
                    for r in 0..3 {
                        for c in 0..3 {
                            let digit_num = r * 3 + c + 1;
                            let digit_str = digit_num.to_string();
                            if ui.add_sized([75.0, 36.0], egui::Button::new(egui::RichText::new(&digit_str).size(18.0).strong())).clicked()
                                 && puzzle_state.keypad_input.len() < 3
                            {
                                puzzle_state.keypad_input.push_str(&digit_str);
                            }
                        }
                        ui.end_row();
                    }
                    if ui.add_sized([75.0, 36.0], egui::Button::new("⌫ Clear")).clicked() {
                        puzzle_state.keypad_input.clear();
                    }
                    if ui.add_sized([75.0, 36.0], egui::Button::new(egui::RichText::new("0").size(18.0).strong())).clicked()
                        && puzzle_state.keypad_input.len() < 3
                    {
                        puzzle_state.keypad_input.push('0');
                    }
                    if ui.add_sized([75.0, 36.0], egui::Button::new(egui::RichText::new("↵ Enter").size(14.0).strong().color(egui::Color32::GREEN))).clicked() {
                        if puzzle_state.keypad_input == "371" {
                            puzzle_state.vault_unlocked = true;
                            puzzle_state.show_security_keypad = false;
                            crate::play_mode::inventory_log("🔓 ACCESS GRANTED! Passcode 371 Accepted! Security Vault Blast Doors disengaged!");
                        } else {
                            crate::play_mode::inventory_log("❌ ACCESS DENIED: Invalid Passcode!");
                            puzzle_state.keypad_input.clear();
                        }
                    }
                    ui.end_row();
                });

                ui.add_space(8.0);

                // Auto-submit check if 3 digits entered
                if puzzle_state.keypad_input.len() == 3 {
                    if puzzle_state.keypad_input == "371" {
                        puzzle_state.vault_unlocked = true;
                        puzzle_state.show_security_keypad = false;
                        crate::play_mode::inventory_log("🔓 ACCESS GRANTED! Passcode 371 Accepted! Security Vault Blast Doors disengaged!");
                    } else {
                        crate::play_mode::inventory_log("❌ ACCESS DENIED: Invalid Passcode!");
                        puzzle_state.keypad_input.clear();
                    }
                }

                if ui
                    .add_sized(
                        [ui.available_width(), 28.0],
                        egui::Button::new(
                            egui::RichText::new("✖ Cancel Keypad (ESC)")
                                .size(14.0)
                                .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(180, 40, 40)),
                    )
                    .clicked()
                {
                    puzzle_state.show_security_keypad = false;
                }
            });
        if !is_open {
            puzzle_state.show_security_keypad = false;
        }
    }

    // 3. Plasma Synthesizer Window
    if puzzle_state.show_synthesizer_ui {
        let mut is_open = true;
        egui::Window::new("🔬 PLASMA SYNTHESIZER STATION")
            .default_width(400.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .collapsible(false)
            .resizable(false)
            .open(&mut is_open)
            .show(ctx, |ui| {
                ui.heading("⚡ HIGH-TECH MATERIAL SYNTHESIZER");
                ui.label("Refine raw planet minerals into advanced components:");
                ui.separator();

                // Recipe 1
                ui.label("🤖 Synthesize Robot Parts (Cost: 2 Stone, 1 Copper)");
                let can_synth_1 = inventory.rock >= 2 && inventory.copper >= 1;
                if ui
                    .add_enabled(
                        can_synth_1,
                        egui::Button::new("⚡ Synthesize +1 Robot Parts"),
                    )
                    .clicked()
                {
                    inventory.rock -= 2;
                    inventory.copper -= 1;
                    inventory.robot_parts += 1;
                    crate::play_mode::inventory_log("⚡ Synthesized +1 Robot Parts!");
                }
                ui.separator();

                // Recipe 2
                ui.label("👽 Synthesize Alien Tech (Cost: 2 Iron, 1 Crystal Shard)");
                let can_synth_2 = inventory.iron >= 2 && inventory.crystal_shard >= 1;
                if ui
                    .add_enabled(
                        can_synth_2,
                        egui::Button::new("⚡ Synthesize +1 Alien Tech"),
                    )
                    .clicked()
                {
                    inventory.iron -= 2;
                    inventory.crystal_shard -= 1;
                    inventory.alien_tech += 1;
                    crate::play_mode::inventory_log("⚡ Synthesized +1 Alien Tech!");
                }
                ui.separator();

                if ui
                    .add_sized(
                        [ui.available_width(), 30.0],
                        egui::Button::new(
                            egui::RichText::new("✖ Close Synthesizer (E / ESC)")
                                .size(15.0)
                                .strong()
                                .color(egui::Color32::WHITE),
                        )
                        .fill(egui::Color32::from_rgb(180, 40, 40)),
                    )
                    .clicked()
                {
                    puzzle_state.show_synthesizer_ui = false;
                }
            });
        if !is_open {
            puzzle_state.show_synthesizer_ui = false;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_research_complex_decorations(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    house_pos: Vec3,
    _half_w: f32,
    _half_d: f32,
    _cell_size: f32,
) {
    // Colors & Materials
    let _metal_dark = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.22, 0.25),
        metallic: 0.8,
        perceptual_roughness: 0.4,
        ..default()
    });
    let screen_cyan = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.9, 1.0),
        emissive: LinearRgba::from(Color::srgb(0.2, 2.5, 3.5)),
        ..default()
    });
    let screen_red = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.2, 0.2),
        emissive: LinearRgba::from(Color::srgb(3.0, 0.4, 0.4)),
        ..default()
    });
    let lab_green = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 1.0, 0.4),
        emissive: LinearRgba::from(Color::srgb(0.4, 2.5, 0.8)),
        ..default()
    });

    // Asset scenes
    let desk_scene = asset_server.load("Prop_Desk_L.gltf#Scene0");
    let chair_scene = asset_server.load("Prop_Chair.gltf#Scene0");
    let chest_scene = asset_server.load("Prop_Chest.gltf#Scene0");
    let crate_scene = asset_server.load("Prop_Crate_Large.gltf#Scene0");
    let health_scene = asset_server.load("Prop_HealthPack.gltf#Scene0");

    // -----------------------------------------------------------------
    // 1. COMMAND & CONTROL FOYER (Ground Floor Center)
    // -----------------------------------------------------------------
    let desk_pos = house_pos + Vec3::new(-3.5, 0.05, -2.5);
    // 3D L-Shaped Executive Desk
    commands.spawn((
        WorldAssetRoot(desk_scene.clone()),
        Transform::from_translation(desk_pos)
            .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
        WallCollider {
            half_extents: Vec3::new(0.9, 0.45, 0.6),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    // 3D Office Swivel Chair
    commands.spawn((
        WorldAssetRoot(chair_scene.clone()),
        Transform::from_translation(desk_pos + Vec3::new(0.0, 0.0, 1.2))
            .with_rotation(Quat::from_rotation_y(0.0)),
        WallCollider {
            half_extents: Vec3::new(0.35, 0.45, 0.35),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    // Holographic Monitor Screen Console (Elevated to sit cleanly on top of desk)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.9, 0.4, 0.05))),
        MeshMaterial3d(screen_cyan.clone()),
        Transform::from_translation(desk_pos + Vec3::new(0.0, 1.15, -0.2)),
        HouseMarker,
        PlayModeEntity,
    ));
    // Terminal 1 (Expedition Log #01)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.4, 0.3, 0.4))),
        MeshMaterial3d(screen_cyan.clone()),
        Transform::from_translation(desk_pos + Vec3::new(0.7, 1.05, -0.1)),
        ResearchTerminal {
            terminal_id: 1,
            title: "🔬 EXPEDITION LOG #01: ATMOSPHERIC INTERFERENCE".to_string(),
            log_text: "Survey Team Alpha - Log 104:\nAtmospheric density on this world is 4x higher than expected. Electromagnetic storms disrupt orbital communication. We have established Research Complex Alpha at grid coordinate West-Edge. Deep mineral scans indicate bioluminescent crystal deposits in the lower cave system.".to_string(),
        },
        WallCollider {
            half_extents: Vec3::new(0.2, 0.15, 0.2),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    // Foyer Supply Crates (Positioned deep inside East & West rooms against outer walls)
    commands.spawn((
        WorldAssetRoot(crate_scene.clone()),
        Transform::from_translation(house_pos + Vec3::new(12.5, 0.05, 6.0)),
        WallCollider {
            half_extents: Vec3::new(0.5, 0.45, 0.5),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    commands.spawn((
        WorldAssetRoot(crate_scene.clone()),
        Transform::from_translation(house_pos + Vec3::new(-12.5, 0.05, 6.0)),
        WallCollider {
            half_extents: Vec3::new(0.5, 0.45, 0.5),
        },
        HouseMarker,
        PlayModeEntity,
    ));

    // -----------------------------------------------------------------
    // 2. BIOCHEMICAL LABORATORY ROOM (Ground Floor West Room next to Stairs)
    // -----------------------------------------------------------------
    let lab_pos = house_pos + Vec3::new(-13.5, 0.05, 2.0);
    // Lab Workbench Desk inside West room
    commands.spawn((
        WorldAssetRoot(desk_scene.clone()),
        Transform::from_translation(lab_pos)
            .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
        WallCollider {
            half_extents: Vec3::new(0.6, 0.45, 0.9),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    // Lab Swivel Chair inside West room
    commands.spawn((
        WorldAssetRoot(chair_scene.clone()),
        Transform::from_translation(lab_pos + Vec3::new(-1.2, 0.0, 0.0))
            .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
        WallCollider {
            half_extents: Vec3::new(0.35, 0.45, 0.35),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    // Emergency Medical Health Pack on Workbench
    commands.spawn((
        WorldAssetRoot(health_scene.clone()),
        Transform::from_translation(lab_pos + Vec3::new(-0.2, 0.95, 0.2)),
        HouseMarker,
        PlayModeEntity,
    ));
    // Glowing Liquid Canister on Workbench
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.15, 0.4))),
        MeshMaterial3d(lab_green.clone()),
        Transform::from_translation(lab_pos + Vec3::new(-0.2, 1.10, -0.4)),
        HouseMarker,
        PlayModeEntity,
    ));
    // Terminal 2 (Expedition Log #02) on Lab Workbench
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.4, 0.3, 0.4))),
        MeshMaterial3d(screen_cyan.clone()),
        Transform::from_translation(lab_pos + Vec3::new(0.2, 1.05, 0.4)),
        ResearchTerminal {
            terminal_id: 2,
            title: "🧪 RESEARCH LOG #02: FAUNA & CRYSTAL SYNTHESIS".to_string(),
            log_text: "Dr. Vance Notes:\nThe native alien species is non-hostile when unprovoked. They possess advanced trade tech. We built a Plasma Synthesizer in the Basement Vault to refine raw surface minerals into tech modules. Security Override Code is recorded in the Basement Server.".to_string(),
        },
        WallCollider {
            half_extents: Vec3::new(0.2, 0.15, 0.2),
        },
        HouseMarker,
        PlayModeEntity,
    ));

    // -----------------------------------------------------------------
    // 3. NORTH-WEST OFFICE ROOM (Ground Floor NW Corner)
    // -----------------------------------------------------------------
    let office_pos = house_pos + Vec3::new(-15.0, 0.05, -6.0);
    // Executive Desk
    commands.spawn((
        WorldAssetRoot(desk_scene.clone()),
        Transform::from_translation(office_pos).with_rotation(Quat::from_rotation_y(0.0)),
        WallCollider {
            half_extents: Vec3::new(0.9, 0.45, 0.6),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    // Executive Swivel Chair
    commands.spawn((
        WorldAssetRoot(chair_scene.clone()),
        Transform::from_translation(office_pos + Vec3::new(0.0, 0.0, 1.2))
            .with_rotation(Quat::from_rotation_y(0.0)),
        WallCollider {
            half_extents: Vec3::new(0.35, 0.45, 0.35),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    // Storage Chest in Office
    commands.spawn((
        WorldAssetRoot(chest_scene.clone()),
        Transform::from_translation(office_pos + Vec3::new(2.5, 0.0, -0.5))
            .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        WallCollider {
            half_extents: Vec3::new(0.35, 0.35, 0.5),
        },
        HouseMarker,
        PlayModeEntity,
    ));

    // -----------------------------------------------------------------
    // 3b. EAST ARMORY & WORKSHOP ROOM (Ground Floor East Room)
    // -----------------------------------------------------------------
    let armory_pos = house_pos + Vec3::new(11.5, 0.05, 2.0);
    commands.spawn((
        WorldAssetRoot(desk_scene.clone()),
        Transform::from_translation(armory_pos)
            .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        WallCollider {
            half_extents: Vec3::new(0.6, 0.45, 0.9),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    commands.spawn((
        WorldAssetRoot(chair_scene.clone()),
        Transform::from_translation(armory_pos + Vec3::new(-1.2, 0.0, 0.0))
            .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
        WallCollider {
            half_extents: Vec3::new(0.35, 0.45, 0.35),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    commands.spawn((
        WorldAssetRoot(chest_scene.clone()),
        Transform::from_translation(armory_pos + Vec3::new(0.0, 0.0, -2.2))
            .with_rotation(Quat::from_rotation_y(0.0)),
        WallCollider {
            half_extents: Vec3::new(0.5, 0.35, 0.35),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    commands.spawn((
        WorldAssetRoot(crate_scene.clone()),
        Transform::from_translation(armory_pos + Vec3::new(1.8, 0.05, 1.8)),
        WallCollider {
            half_extents: Vec3::new(0.5, 0.45, 0.5),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    commands.spawn((
        WorldAssetRoot(health_scene.clone()),
        Transform::from_translation(armory_pos + Vec3::new(0.2, 0.95, -0.2)),
        HouseMarker,
        PlayModeEntity,
    ));

    // -----------------------------------------------------------------
    // 3c. NORTH-EAST CONTROL SUITE (Ground Floor NE Corner)
    // -----------------------------------------------------------------
    let ne_control_pos = house_pos + Vec3::new(14.0, 0.05, -7.5);
    commands.spawn((
        WorldAssetRoot(desk_scene.clone()),
        Transform::from_translation(ne_control_pos + Vec3::new(0.0, 0.0, -0.8))
            .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
        WallCollider {
            half_extents: Vec3::new(0.9, 0.45, 0.6),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    commands.spawn((
        WorldAssetRoot(chair_scene.clone()),
        Transform::from_translation(ne_control_pos + Vec3::new(0.0, 0.0, 0.5))
            .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
        WallCollider {
            half_extents: Vec3::new(0.35, 0.45, 0.35),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    commands.spawn((
        WorldAssetRoot(crate_scene.clone()),
        Transform::from_translation(ne_control_pos + Vec3::new(3.2, 0.05, -1.0)),
        WallCollider {
            half_extents: Vec3::new(0.5, 0.45, 0.5),
        },
        HouseMarker,
        PlayModeEntity,
    ));

    // -----------------------------------------------------------------
    // 4. LIVING QUARTERS BARRACKS (First Floor West)
    // -----------------------------------------------------------------
    let bed_pos = house_pos + Vec3::new(-5.0, 3.55, -2.0);
    // Equipment Storage Chest at bed foot
    commands.spawn((
        WorldAssetRoot(chest_scene.clone()),
        Transform::from_translation(bed_pos + Vec3::new(0.0, 0.0, 1.8))
            .with_rotation(Quat::from_rotation_y(0.0)),
        WallCollider {
            half_extents: Vec3::new(0.5, 0.35, 0.35),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    // Barracks Rest Chair
    commands.spawn((
        WorldAssetRoot(chair_scene.clone()),
        Transform::from_translation(bed_pos + Vec3::new(-2.0, 0.0, 0.0))
            .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        WallCollider {
            half_extents: Vec3::new(0.35, 0.45, 0.35),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    // Barracks Health Pack
    commands.spawn((
        WorldAssetRoot(health_scene.clone()),
        Transform::from_translation(bed_pos + Vec3::new(-1.8, 0.1, 1.2)),
        HouseMarker,
        PlayModeEntity,
    ));

    // -----------------------------------------------------------------
    // 4b. MASTER EXECUTIVE SUITE (First Floor East)
    // -----------------------------------------------------------------
    let east_suite_pos = house_pos + Vec3::new(8.5, 3.55, -2.0);
    commands.spawn((
        WorldAssetRoot(desk_scene.clone()),
        Transform::from_translation(east_suite_pos)
            .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
        WallCollider {
            half_extents: Vec3::new(0.9, 0.45, 0.6),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    commands.spawn((
        WorldAssetRoot(chair_scene.clone()),
        Transform::from_translation(east_suite_pos + Vec3::new(0.0, 0.0, -1.2))
            .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
        WallCollider {
            half_extents: Vec3::new(0.35, 0.45, 0.35),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    commands.spawn((
        WorldAssetRoot(chest_scene.clone()),
        Transform::from_translation(east_suite_pos + Vec3::new(2.2, 0.0, 0.0))
            .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
        WallCollider {
            half_extents: Vec3::new(0.35, 0.35, 0.5),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    commands.spawn((
        WorldAssetRoot(health_scene.clone()),
        Transform::from_translation(east_suite_pos + Vec3::new(-0.4, 0.95, 0.1)),
        HouseMarker,
        PlayModeEntity,
    ));

    // -----------------------------------------------------------------
    // 4c. OBSERVATORY BALCONY (First Floor North)
    // -----------------------------------------------------------------
    let balcony_pos = house_pos + Vec3::new(0.0, 3.55, -8.5);
    commands.spawn((
        WorldAssetRoot(desk_scene.clone()),
        Transform::from_translation(balcony_pos).with_rotation(Quat::from_rotation_y(0.0)),
        WallCollider {
            half_extents: Vec3::new(0.9, 0.45, 0.6),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    commands.spawn((
        WorldAssetRoot(chair_scene.clone()),
        Transform::from_translation(balcony_pos + Vec3::new(-1.2, 0.0, 0.0))
            .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
        WallCollider {
            half_extents: Vec3::new(0.35, 0.45, 0.35),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    commands.spawn((
        WorldAssetRoot(chair_scene.clone()),
        Transform::from_translation(balcony_pos + Vec3::new(1.2, 0.0, 0.0))
            .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
        WallCollider {
            half_extents: Vec3::new(0.35, 0.45, 0.35),
        },
        HouseMarker,
        PlayModeEntity,
    ));

    // -----------------------------------------------------------------
    // 5. BASEMENT VAULT & GENERATOR ROOM (Y = -50.0)
    // -----------------------------------------------------------------
    let vault_center = Vec3::new(house_pos.x + 6.0, -50.0, house_pos.z - 4.0);

    // Heavy Metal Cargo Crates Stack inside Inner Vault Chamber (Far East corner)
    commands.spawn((
        WorldAssetRoot(crate_scene.clone()),
        Transform::from_translation(vault_center + Vec3::new(4.2, 0.05, 3.2)),
        WallCollider {
            half_extents: Vec3::new(0.5, 0.45, 0.5),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    commands.spawn((
        WorldAssetRoot(crate_scene.clone()),
        Transform::from_translation(vault_center + Vec3::new(4.2, 0.85, 3.2)),
        WallCollider {
            half_extents: Vec3::new(0.5, 0.45, 0.5),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    commands.spawn((
        WorldAssetRoot(crate_scene.clone()),
        Transform::from_translation(vault_center + Vec3::new(4.2, 0.05, -3.2)),
        WallCollider {
            half_extents: Vec3::new(0.5, 0.45, 0.5),
        },
        HouseMarker,
        PlayModeEntity,
    ));

    // Heavy Vault Equipment Chest inside Basement
    commands.spawn((
        WorldAssetRoot(chest_scene.clone()),
        Transform::from_translation(vault_center + Vec3::new(4.2, 0.05, -1.5))
            .with_rotation(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)),
        WallCollider {
            half_extents: Vec3::new(0.35, 0.35, 0.5),
        },
        HouseMarker,
        PlayModeEntity,
    ));

    // Locked Security Blast Door
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.3, 3.5, 3.2))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.35, 0.38, 0.42),
            metallic: 0.95,
            perceptual_roughness: 0.2,
            ..default()
        })),
        Transform::from_translation(vault_center + Vec3::new(-2.0, 2.0, 0.0)),
        BasementVaultDoor,
        WallCollider {
            half_extents: Vec3::new(0.15, 1.75, 1.6),
        },
        HouseMarker,
        PlayModeEntity,
    ));

    // Security Keypad Terminal Console (Uncluttered, mounted on wall pillar next to Blast Door)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.4, 0.6, 0.4))),
        MeshMaterial3d(screen_red.clone()),
        Transform::from_translation(vault_center + Vec3::new(-2.6, 1.5, -2.5)),
        BasementSecurityConsole,
        WallCollider {
            half_extents: Vec3::new(0.2, 0.3, 0.2),
        },
        HouseMarker,
        PlayModeEntity,
    ));

    // Terminal 3 (Log #03 - Code Clue)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.4, 0.5, 0.4))),
        MeshMaterial3d(screen_cyan.clone()),
        Transform::from_translation(vault_center + Vec3::new(-2.6, 1.5, 2.5)),
        ResearchTerminal {
            terminal_id: 3,
            title: "🔒 BASEMENT SECURITY OVERRIDE PASSCODE".to_string(),
            log_text: "SECURITY ALERT:\nBasement Vault locked due to grid overload. Emergency Manual Keypad Override Passcode: [ 3 - 7 - 1 ]. Enter passcode on the Security Keypad to disengage magnetic locks and access the Plasma Synthesizer.".to_string(),
        },
        WallCollider {
            half_extents: Vec3::new(0.2, 0.25, 0.2),
        },
        HouseMarker,
        PlayModeEntity,
    ));

    // Plasma Synthesizer Station inside Vault Chamber
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.4, 1.2, 1.4))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.1, 0.7, 0.9),
            emissive: LinearRgba::from(Color::srgb(0.5, 3.0, 4.0)),
            ..default()
        })),
        Transform::from_translation(vault_center + Vec3::new(2.5, 1.2, 0.0)),
        PlasmaSynthesizerConsole,
        WallCollider {
            half_extents: Vec3::new(0.7, 0.6, 0.7),
        },
        HouseMarker,
        PlayModeEntity,
    ));
}
