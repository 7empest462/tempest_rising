use crate::AppState;
use crate::map_editor::data::{Biome, TempestMap};
use crate::play_mode::{PlayModeEntity, PlayModePlayer, PlayerInventory, WallCollider};
use avian3d::prelude::{Collider, Position, RigidBody};
use bevy::prelude::*;

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
                )
                    .run_if(in_state(AppState::PlayMode)),
            );
    }
}

#[derive(Resource, Default)]
pub struct HousePuzzleState {
    pub bookcase_opened: bool,
    pub basement_solved: bool,
    pub artifact_collected: bool,
}

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
pub fn flatten_terrain(mut map: ResMut<TempestMap>, mansion_settings: Res<MansionSettings>) {
    let mut house_pos = Vec3::new(0.0, 1.5, 0.0);
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

    let min_x_idx = ((house_pos.x - half_w - 2.0) + half_map_w).max(0.0) as u32;
    let max_x_idx = ((house_pos.x + half_w + 2.0) + half_map_w).min(map.width as f32) as u32;
    let min_z_idx = ((house_pos.z - half_d - 2.0) + half_map_h).max(0.0) as u32;
    let max_z_idx = ((house_pos.z + half_d + 2.0) + half_map_h).min(map.height as f32) as u32;

    for mz in min_z_idx..max_z_idx {
        for mx in min_x_idx..max_x_idx {
            map.set_height(mx, mz, 1.5);
            map.set_biome(mx, mz, Biome::Temperate);
        }
    }
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

fn spawn_window_wall(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    wall_material: &Handle<StandardMaterial>,
    pos: Vec3,
    is_horizontal: bool,
) {
    let iron_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.2, 0.22), // dark iron
        metallic: 0.8,
        perceptual_roughness: 0.4,
        ..default()
    });

    if is_horizontal {
        // Left post (width 1.8)
        let lp_size = Vec3::new(1.8, 3.5, 0.2);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(lp_size))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_translation(pos + Vec3::new(-1.6, 0.0, 0.0)),
            WallCollider {
                half_extents: lp_size * 0.5,
            },
            HouseMarker,
            PlayModeEntity,
        ));
        // Right post (width 1.8)
        let rp_size = Vec3::new(1.8, 3.5, 0.2);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(rp_size))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_translation(pos + Vec3::new(1.6, 0.0, 0.0)),
            WallCollider {
                half_extents: rp_size * 0.5,
            },
            HouseMarker,
            PlayModeEntity,
        ));
        // Bottom post (width 1.4, height 1.0)
        let bp_size = Vec3::new(1.4, 1.0, 0.2);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(bp_size))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_translation(pos + Vec3::new(0.0, -1.25, 0.0)),
            WallCollider {
                half_extents: bp_size * 0.5,
            },
            HouseMarker,
            PlayModeEntity,
        ));
        // Top post (width 1.4, height 1.0)
        let tp_size = Vec3::new(1.4, 1.0, 0.2);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(tp_size))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_translation(pos + Vec3::new(0.0, 1.25, 0.0)),
            WallCollider {
                half_extents: tp_size * 0.5,
            },
            HouseMarker,
            PlayModeEntity,
        ));
        // 3 Vertical iron bars (height 1.5, in the opening)
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
        // Vertical window wall (along Z)
        // Left post (width 1.8 along Z)
        let lp_size = Vec3::new(0.2, 3.5, 1.8);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(lp_size))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_translation(pos + Vec3::new(0.0, 0.0, -1.6)),
            WallCollider {
                half_extents: lp_size * 0.5,
            },
            HouseMarker,
            PlayModeEntity,
        ));
        // Right post (width 1.8 along Z)
        let rp_size = Vec3::new(0.2, 3.5, 1.8);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(rp_size))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_translation(pos + Vec3::new(0.0, 0.0, 1.6)),
            WallCollider {
                half_extents: rp_size * 0.5,
            },
            HouseMarker,
            PlayModeEntity,
        ));
        // Bottom post (height 1.0)
        let bp_size = Vec3::new(0.2, 1.0, 1.4);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(bp_size))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_translation(pos + Vec3::new(0.0, -1.25, 0.0)),
            WallCollider {
                half_extents: bp_size * 0.5,
            },
            HouseMarker,
            PlayModeEntity,
        ));
        // Top post (height 1.0)
        let tp_size = Vec3::new(0.2, 1.0, 1.4);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(tp_size))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_translation(pos + Vec3::new(0.0, 1.25, 0.0)),
            WallCollider {
                half_extents: tp_size * 0.5,
            },
            HouseMarker,
            PlayModeEntity,
        ));
        // 3 Vertical iron bars (height 1.5, in the opening)
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

fn spawn_solid_wall(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    material: &Handle<StandardMaterial>,
    pos: Vec3,
    is_horizontal: bool,
) {
    let size = if is_horizontal {
        Vec3::new(5.0, 3.5, 0.2)
    } else {
        Vec3::new(0.2, 3.5, 5.0)
    };
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(size))),
        MeshMaterial3d(material.clone()),
        Transform::from_translation(pos),
        WallCollider {
            half_extents: size * 0.5,
        },
        HouseMarker,
        PlayModeEntity,
    ));
}

fn spawn_door_wall(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    wall_material: &Handle<StandardMaterial>,
    pos: Vec3,
    is_horizontal: bool,
    asset_server: &Res<AssetServer>,
) {
    let door_width = 1.6;
    let door_height = 2.2;

    let door_mat = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/wooden_door.png")),
        perceptual_roughness: 0.8,
        ..default()
    });

    if is_horizontal {
        // Left post (length 1.7m)
        let lp_size = Vec3::new(1.7, 3.5, 0.2);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(lp_size))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_translation(pos + Vec3::new(-1.65, 0.0, 0.0)),
            WallCollider {
                half_extents: lp_size * 0.5,
            },
            HouseMarker,
            PlayModeEntity,
        ));

        // Right post (length 1.7m)
        let rp_size = Vec3::new(1.7, 3.5, 0.2);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(rp_size))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_translation(pos + Vec3::new(1.65, 0.0, 0.0)),
            WallCollider {
                half_extents: rp_size * 0.5,
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
            WallCollider {
                half_extents: l_size * 0.5,
            },
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
                WallCollider {
                    half_extents: Vec3::new(1.6, 1.1, 0.05), // centered at -0.8 hinge, spans from -2.4 to +0.8, covering doorway
                },
                HouseMarker,
                PlayModeEntity,
                Visibility::Visible,
                InheritedVisibility::default(),
            ))
            .id();

        // Child visual mesh offset to the right by half door width
        let child_id = commands
            .spawn((
                Mesh3d(meshes.add(Cuboid::from_size(door_size))),
                MeshMaterial3d(door_mat),
                Transform::from_xyz(0.8, 0.0, 0.0),
                HouseMarker,
                PlayModeEntity,
            ))
            .id();

        commands.entity(parent_id).add_child(child_id);
    } else {
        // Vertical door wall (along Z)
        let lp_size = Vec3::new(0.2, 3.5, 1.7);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(lp_size))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_translation(pos + Vec3::new(0.0, 0.0, -1.65)),
            WallCollider {
                half_extents: lp_size * 0.5,
            },
            HouseMarker,
            PlayModeEntity,
        ));

        let rp_size = Vec3::new(0.2, 3.5, 1.7);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(rp_size))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_translation(pos + Vec3::new(0.0, 0.0, 1.65)),
            WallCollider {
                half_extents: rp_size * 0.5,
            },
            HouseMarker,
            PlayModeEntity,
        ));

        let l_size = Vec3::new(0.2, 1.3, door_width);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(l_size))),
            MeshMaterial3d(wall_material.clone()),
            Transform::from_translation(pos + Vec3::new(0.0, 1.1, 0.0)),
            WallCollider {
                half_extents: l_size * 0.5,
            },
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
                WallCollider {
                    half_extents: Vec3::new(0.05, 1.1, 1.6), // centered at -0.8 hinge, spans Z from -2.4 to +0.8, covering doorway
                },
                HouseMarker,
                PlayModeEntity,
                Visibility::Visible,
                InheritedVisibility::default(),
            ))
            .id();

        // Child visual mesh offset along Z by half door width
        let child_id = commands
            .spawn((
                Mesh3d(meshes.add(Cuboid::from_size(door_size))),
                MeshMaterial3d(door_mat),
                Transform::from_xyz(0.0, 0.0, 0.8),
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

    let mut house_pos = Vec3::new(0.0, 1.5, 0.0);
    for p in map.prefabs.iter() {
        if p.prefab_type == "house" {
            house_pos = Vec3::from_array(p.position);
            break;
        }
    }

    // Materials
    let wall_mat = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/solid_brick.png")),
        perceptual_roughness: 0.9,
        ..default()
    });

    let floor_mat = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/solid_limestone.png")),
        perceptual_roughness: 0.85,
        ..default()
    });

    let basement_stone_mat = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/solid_stone.png")),
        perceptual_roughness: 0.95,
        ..default()
    });

    let sub_basement_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.4, 0.6), // dark tinted
        base_color_texture: Some(asset_server.load("textures/solid_stone.png")),
        perceptual_roughness: 0.95,
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
                    commands.spawn((
                        Mesh3d(meshes.add(Cuboid::new(cell_size, 0.1, cell_size))),
                        MeshMaterial3d(floor_mat.clone()),
                        Transform::from_xyz(x_center, y_base, z_center),
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
                        );
                    } else {
                        spawn_solid_wall(
                            &mut commands,
                            &mut meshes,
                            &wall_mat,
                            Vec3::new(x_center, y_base + 1.75, house_pos.z - half_d),
                            true,
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
                        );
                    } else {
                        spawn_solid_wall(
                            &mut commands,
                            &mut meshes,
                            &wall_mat,
                            Vec3::new(house_pos.x - half_w, y_base + 1.75, z_center),
                            false,
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
                        );
                    } else if cell_type == CellType::Bedroom {
                        spawn_window_wall(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            &wall_mat,
                            Vec3::new(x_center, y_base + 1.75, house_pos.z + half_d),
                            true,
                        );
                    } else {
                        spawn_solid_wall(
                            &mut commands,
                            &mut meshes,
                            &wall_mat,
                            Vec3::new(x_center, y_base + 1.75, house_pos.z + half_d),
                            true,
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
                        );
                    } else {
                        spawn_solid_wall(
                            &mut commands,
                            &mut meshes,
                            &wall_mat,
                            Vec3::new(house_pos.x + half_w, y_base + 1.75, z_center),
                            false,
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

    // Basement descent portal (Ground Floor Foyer -> Basement)
    let foyer_portal_x = house_pos.x + 3.5;
    let foyer_portal_z = house_pos.z + 2.5;
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.4).mesh().ico(3).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.1, 0.8, 0.5, 0.6),
            alpha_mode: AlphaMode::Blend,
            emissive: LinearRgba::from(Color::srgb(0.05, 0.4, 0.2)),
            ..default()
        })),
        Transform::from_xyz(foyer_portal_x, house_pos.y + 0.7, foyer_portal_z),
        Teleporter {
            target_pos: Vec3::new(foyer_portal_x, -48.2, foyer_portal_z),
            message: "🕳 Entering Basement Cellar...".to_string(),
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
        Transform::from_xyz(se_x, house_pos.y + 4.15, se_z),
        PuzzleChest { is_locked: true },
        HouseMarker,
        PlayModeEntity,
    ));

    // -----------------------------------------------------------------
    // FLOOR 3: BASEMENT (spawns underground at Y = -50.0)
    // -----------------------------------------------------------------
    let basement_w = (grid_cols as f32 * cell_size) + 4.0;
    let basement_d = (grid_rows as f32 * cell_size) + 4.0;

    // Floor
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(basement_w, 0.1, basement_d))),
        MeshMaterial3d(basement_stone_mat.clone()),
        Transform::from_xyz(house_pos.x, -50.0, house_pos.z),
        RigidBody::Static,
        Collider::cuboid(basement_w, 0.1, basement_d),
        HouseMarker,
        PlayModeEntity,
    ));

    // Ceiling
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(basement_w, 0.1, basement_d))),
        MeshMaterial3d(basement_stone_mat.clone()),
        Transform::from_xyz(house_pos.x, -46.0, house_pos.z),
        RigidBody::Static,
        Collider::cuboid(basement_w, 0.1, basement_d),
        HouseMarker,
        PlayModeEntity,
    ));

    // Outer Walls
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(basement_w, 4.0, 0.2))),
        MeshMaterial3d(basement_stone_mat.clone()),
        Transform::from_xyz(house_pos.x, -48.0, house_pos.z - (basement_d * 0.5)),
        WallCollider {
            half_extents: Vec3::new(basement_w * 0.5, 2.0, 0.1),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(basement_w, 4.0, 0.2))),
        MeshMaterial3d(basement_stone_mat.clone()),
        Transform::from_xyz(house_pos.x, -48.0, house_pos.z + (basement_d * 0.5)),
        WallCollider {
            half_extents: Vec3::new(basement_w * 0.5, 2.0, 0.1),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.2, 4.0, basement_d))),
        MeshMaterial3d(basement_stone_mat.clone()),
        Transform::from_xyz(house_pos.x + (basement_w * 0.5), -48.0, house_pos.z),
        WallCollider {
            half_extents: Vec3::new(0.1, 2.0, basement_d * 0.5),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.2, 4.0, basement_d))),
        MeshMaterial3d(basement_stone_mat.clone()),
        Transform::from_xyz(house_pos.x - (basement_w * 0.5), -48.0, house_pos.z),
        WallCollider {
            half_extents: Vec3::new(0.1, 2.0, basement_d * 0.5),
        },
        HouseMarker,
        PlayModeEntity,
    ));

    // Basement ascent portal
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.4).mesh().ico(3).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.1, 0.8, 0.5, 0.6),
            alpha_mode: AlphaMode::Blend,
            emissive: LinearRgba::from(Color::srgb(0.05, 0.4, 0.2)),
            ..default()
        })),
        Transform::from_xyz(foyer_portal_x, -48.2, foyer_portal_z),
        Teleporter {
            target_pos: Vec3::new(foyer_portal_x, house_pos.y + 0.7, foyer_portal_z),
            message: "🏠 Climbing back to Ground Floor foyer...".to_string(),
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
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(8.0, 4.0, 0.2))),
        MeshMaterial3d(basement_stone_mat.clone()),
        Transform::from_xyz(house_pos.x - 6.0, -48.0, house_pos.z),
        WallCollider {
            half_extents: Vec3::new(4.0, 2.0, 0.1),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(8.0, 4.0, 0.2))),
        MeshMaterial3d(basement_stone_mat.clone()),
        Transform::from_xyz(house_pos.x + 6.0, -48.0, house_pos.z),
        WallCollider {
            half_extents: Vec3::new(4.0, 2.0, 0.1),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0, 1.2, 0.2))),
        MeshMaterial3d(basement_stone_mat.clone()),
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
            target_pos: Vec3::new(house_pos.x, -98.2, house_pos.z),
            message: "🕯️ Descending into the Ancient Crypt...".to_string(),
        },
        HouseMarker,
        PlayModeEntity,
    ));

    // -----------------------------------------------------------------
    // FLOOR 4: SUB-BASEMENT (spawns deep underground at Y = -100.0)
    // -----------------------------------------------------------------

    // Floor
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(10.0, 0.1, 10.0))),
        MeshMaterial3d(sub_basement_mat.clone()),
        Transform::from_xyz(house_pos.x, -100.0, house_pos.z),
        RigidBody::Static,
        Collider::cuboid(10.0, 0.1, 10.0),
        HouseMarker,
        PlayModeEntity,
    ));

    // Ceiling
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(10.0, 0.1, 10.0))),
        MeshMaterial3d(sub_basement_mat.clone()),
        Transform::from_xyz(house_pos.x, -96.0, house_pos.z),
        RigidBody::Static,
        Collider::cuboid(10.0, 0.1, 10.0),
        HouseMarker,
        PlayModeEntity,
    ));

    // Outer Walls
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(10.0, 4.0, 0.2))),
        MeshMaterial3d(sub_basement_mat.clone()),
        Transform::from_xyz(house_pos.x, -98.0, house_pos.z - 5.0),
        WallCollider {
            half_extents: Vec3::new(5.0, 2.0, 0.1),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(10.0, 4.0, 0.2))),
        MeshMaterial3d(sub_basement_mat.clone()),
        Transform::from_xyz(house_pos.x, -98.0, house_pos.z + 5.0),
        WallCollider {
            half_extents: Vec3::new(5.0, 2.0, 0.1),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.2, 4.0, 10.0))),
        MeshMaterial3d(sub_basement_mat.clone()),
        Transform::from_xyz(house_pos.x + 5.0, -98.0, house_pos.z),
        WallCollider {
            half_extents: Vec3::new(0.1, 2.0, 5.0),
        },
        HouseMarker,
        PlayModeEntity,
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.2, 4.0, 10.0))),
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
            target_pos: Vec3::new(house_pos.x, -48.2, ladder_z),
            message: "🪜 Climbing back up to basement cellar...".to_string(),
        },
        HouseMarker,
        PlayModeEntity,
    ));

    // Pedestal
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.35, 1.2))),
        MeshMaterial3d(basement_stone_mat.clone()),
        Transform::from_xyz(house_pos.x, -99.4, house_pos.z),
        ArtifactPedestal,
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

    // Spawn visual steps (no individual physics colliders to prevent getting stuck)
    for step_idx in 0..num_steps {
        let step_y = house_pos.y + (step_idx as f32) * step_height + step_height * 0.5;
        // Start from house_pos.z + 5.0 (bottom, South foyer) and go up to house_pos.z - 5.0 (top, North bedroom/hallway boundary)
        let step_z = (house_pos.z + 5.0) - (step_idx as f32) * step_depth - step_depth * 0.5;

        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(step_width, step_height, step_depth))),
            MeshMaterial3d(floor_mat.clone()),
            Transform::from_xyz(staircase_x, step_y, step_z),
            HouseMarker,
            PlayModeEntity,
        ));
    }

    // Spawn a single smooth tilted ramp collider underneath the steps (thick and wide to prevent clipping/sliding off)
    let pitch = (3.5f32 / total_depth).atan();
    let slope_len = (total_depth.powi(2) + 3.5f32.powi(2)).sqrt();
    let local_y = Quat::from_rotation_x(pitch) * Vec3::Y;
    let center_pos = Vec3::new(staircase_x, house_pos.y + 1.75, house_pos.z) - local_y * 1.485;

    commands.spawn((
        Transform::from_translation(center_pos).with_rotation(Quat::from_rotation_x(pitch)),
        RigidBody::Static,
        Collider::cuboid(4.8, 3.0, slope_len),
        HouseMarker,
        PlayModeEntity,
    ));

    // -----------------------------------------------------------------
    // MANSION PITCHED ROOF (using roof_shingles.png)
    // -----------------------------------------------------------------
    let mansion_roof_mesh = meshes.add(Cuboid::new(41.0, 0.1, 11.0));
    let mansion_roof_mat = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/roof_shingles.png")),
        perceptual_roughness: 0.9,
        ..default()
    });

    // North slope
    commands.spawn((
        Mesh3d(mansion_roof_mesh.clone()),
        MeshMaterial3d(mansion_roof_mat.clone()),
        Transform::from_xyz(house_pos.x, house_pos.y + 7.0 + 1.0, house_pos.z - 5.0)
            .with_rotation(Quat::from_rotation_x(-0.20)), // pitch up towards center
        RigidBody::Static,
        Collider::cuboid(41.0, 0.1, 11.0),
        HouseMarker,
        PlayModeEntity,
    ));

    // South slope
    commands.spawn((
        Mesh3d(mansion_roof_mesh),
        MeshMaterial3d(mansion_roof_mat),
        Transform::from_xyz(house_pos.x, house_pos.y + 7.0 + 1.0, house_pos.z + 5.0)
            .with_rotation(Quat::from_rotation_x(0.20)), // pitch up towards center
        RigidBody::Static,
        Collider::cuboid(41.0, 0.1, 11.0),
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

fn door_swing_system(time: Res<Time>, mut query: Query<(&mut Transform, &HouseDoor)>) {
    let dt = time.delta_secs();
    for (mut transform, door) in query.iter_mut() {
        let target_rot = if door.is_open {
            door.open_rot
        } else {
            door.closed_rot
        };
        transform.rotation = transform.rotation.slerp(target_rot, 5.0 * dt);
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
#[allow(clippy::too_many_arguments)]
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
    map: Res<TempestMap>,
    mansion_settings: Res<MansionSettings>,
) {
    if !keyboard_input.just_pressed(KeyCode::KeyE) {
        return;
    }
    let Ok((_player_entity, mut player, mut player_transform, mut phys_pos)) =
        player_query.single_mut()
    else {
        return;
    };
    let player_pos = player.position;

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
