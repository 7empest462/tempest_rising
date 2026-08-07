use crate::AppState;
use crate::map_editor::data::TempestMap;
use crate::play_mode::{
    PlayModeEntity, PlayModePlayer, PlayResourceNode, PlayerInventory, WallCollider, inventory_log,
};
use avian3d::prelude::*;
use bevy::prelude::*;

#[derive(Component)]
pub struct TorchFlameEffect {
    pub base_intensity: f32,
    pub base_y: f32,
    pub seed: f32,
}

#[derive(Component)]
pub struct CaveEntranceMarker {
    pub _cave_id: u32,
    pub target_pos: Vec3,
}

#[derive(Component)]
pub struct CaveExitMarker {
    pub _cave_id: u32,
    pub target_pos: Vec3,
}

#[derive(Component)]
pub struct CaveChest {
    pub is_opened: bool,
}

#[derive(Component)]
pub struct CaveHeadlamp;

#[derive(Resource, Default)]
pub struct CaveSystemData {
    pub surface_entrances: Vec<Vec3>,
    pub cave_spawns: Vec<Vec3>,
    pub grid_cols: usize,
    pub grid_rows: usize,
    pub grid: Vec<Vec<u8>>,
}

pub struct CavePlugin;

impl Plugin for CavePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CaveSystemData>().add_systems(
            Update,
            (
                cave_interaction_system,
                cave_headlamp_system,
                cave_torch_flame_animation_system,
            )
                .run_if(in_state(AppState::PlayMode)),
        );
    }
}

pub const CAVE_FLOOR_Y: f32 = -150.0;
pub const CAVE_CEILING_Y: f32 = -142.0; // Vaulted 8.0m height
pub const _CAVE_GRID_COLS: usize = 36;
pub const _CAVE_GRID_ROWS: usize = 36;
pub const CAVE_CELL_SIZE: f32 = 4.0;

/// Generates and spawns the 3D underground cave maze level at Y = -150.0
/// and places corresponding surface cave entrances on the main map terrain.
pub fn setup_underground_cave_system(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    map: &TempestMap,
    cave_data: &mut CaveSystemData,
) {
    cave_data.surface_entrances.clear();
    cave_data.cave_spawns.clear();

    // Dynamically scale cave grid dimensions to match the surface map width and depth
    let grid_cols = ((map.width as f32 / CAVE_CELL_SIZE) as usize).clamp(36, 120);
    let grid_rows = ((map.height as f32 / CAVE_CELL_SIZE) as usize).clamp(36, 120);

    // 1. Generate Maze Grid using Depth-First-Search / Recursive Backtracker
    // 0 = Solid Wall, 1 = Open Passage, 2 = Chamber/Hub
    let mut grid = vec![vec![0u8; grid_rows]; grid_cols];
    let mut rng = fastrand::Rng::with_seed(12345);

    // Carve maze paths
    let mut stack = Vec::new();
    let start_c = 1;
    let start_r = 1;
    grid[start_c][start_r] = 1;
    stack.push((start_c, start_r));

    while let Some((c, r)) = stack.pop() {
        let mut neighbors = Vec::new();

        let dirs = [(0, -2), (0, 2), (-2, 0), (2, 0)];
        for (dc, dr) in dirs {
            let nc = c as i32 + dc;
            let nr = r as i32 + dr;
            if nc > 0
                && nc < (grid_cols as i32 - 1)
                && nr > 0
                && nr < (grid_rows as i32 - 1)
                && grid[nc as usize][nr as usize] == 0
            {
                neighbors.push((
                    nc as usize,
                    nr as usize,
                    (c as i32 + dc / 2) as usize,
                    (r as i32 + dr / 2) as usize,
                ));
            }
        }

        if !neighbors.is_empty() {
            stack.push((c, r));
            let idx = rng.usize(0..neighbors.len());
            let (nc, nr, mc, mr) = neighbors[idx];
            grid[mc][mr] = 1;
            grid[nc][nr] = 1;
            stack.push((nc, nr));
        }
    }

    // Carve 4 Spacious Entrance Hubs (5x5 open halls) distributed across the ENTIRE map
    let c1 = (grid_cols / 6).clamp(3, grid_cols - 4);
    let c2 = (5 * grid_cols / 6).clamp(3, grid_cols - 4);
    let r1 = (grid_rows / 6).clamp(3, grid_rows - 4);
    let r2 = (5 * grid_rows / 6).clamp(3, grid_rows - 4);

    let entrance_chamber_indices = [(c1, r1), (c2, r1), (c1, r2), (c2, r2)];
    for &(ec, er) in &entrance_chamber_indices {
        for dc in -2..=2 {
            for dr in -2..=2 {
                let nc = (ec as i32 + dc).clamp(1, grid_cols as i32 - 2) as usize;
                let nr = (er as i32 + dr).clamp(1, grid_rows as i32 - 2) as usize;
                grid[nc][nr] = 2; // Spacious Entrance Hub
            }
        }
        // Connect Entrance Hubs with cardinal corridors into the main maze
        for i in 1..=6 {
            grid[(ec as i32 + i).clamp(1, grid_cols as i32 - 2) as usize][er] = 1;
            grid[(ec as i32 - i).clamp(1, grid_cols as i32 - 2) as usize][er] = 1;
            grid[ec][(er as i32 + i).clamp(1, grid_rows as i32 - 2) as usize] = 1;
            grid[ec][(er as i32 - i).clamp(1, grid_rows as i32 - 2) as usize] = 1;
        }
    }

    // Carve 9 Large Subterranean Caverns across the full grid
    let cm = grid_cols / 2;
    let rm = grid_rows / 2;
    let chamber_centers = [
        (grid_cols / 4, grid_rows / 4),
        (3 * grid_cols / 4, grid_rows / 4),
        (grid_cols / 4, 3 * grid_rows / 4),
        (3 * grid_cols / 4, 3 * grid_rows / 4),
        (cm, rm),
        (cm, grid_rows / 4),
        (cm, 3 * grid_rows / 4),
        (grid_cols / 4, rm),
        (3 * grid_cols / 4, rm),
    ];
    for (cc, cr) in chamber_centers {
        for dc in -3..=3 {
            for dr in -3..=3 {
                let nc = (cc as i32 + dc).clamp(1, grid_cols as i32 - 2) as usize;
                let nr = (cr as i32 + dr).clamp(1, grid_rows as i32 - 2) as usize;
                grid[nc][nr] = 2; // Cavern
            }
        }
    }

    // 2. Materials & Meshes for Underground World
    cave_data.grid_cols = grid_cols;
    cave_data.grid_rows = grid_rows;
    cave_data.grid = grid.clone();

    let cave_rock_texture = asset_server
        .load_builder()
        .with_settings(|settings: &mut bevy::image::ImageLoaderSettings| {
            settings.sampler =
                bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
                    address_mode_u: bevy::image::ImageAddressMode::Repeat,
                    address_mode_v: bevy::image::ImageAddressMode::Repeat,
                    ..default()
                });
        })
        .load("textures/rock.png");

    let wall_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.55, 0.58),
        base_color_texture: Some(cave_rock_texture.clone()),
        perceptual_roughness: 0.88,
        metallic: 0.05,
        ..default()
    });

    let floor_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.50, 0.48, 0.52),
        base_color_texture: Some(cave_rock_texture.clone()),
        perceptual_roughness: 0.85,
        metallic: 0.05,
        ..default()
    });

    let ceiling_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.45, 0.48),
        base_color_texture: Some(cave_rock_texture),
        perceptual_roughness: 0.90,
        metallic: 0.05,
        ..default()
    });

    let crystal_cyan_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.9, 1.0),
        emissive: LinearRgba::new(0.6, 3.5, 4.5, 1.0),
        perceptual_roughness: 0.1,
        metallic: 0.8,
        ..default()
    });

    let crystal_purple_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.2, 1.0),
        emissive: LinearRgba::new(3.8, 0.6, 4.2, 1.0),
        perceptual_roughness: 0.1,
        metallic: 0.8,
        ..default()
    });

    let crystal_emerald_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 1.0, 0.5),
        emissive: LinearRgba::new(0.6, 4.0, 1.5, 1.0),
        perceptual_roughness: 0.1,
        metallic: 0.8,
        ..default()
    });

    let crystal_gold_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.8, 0.2),
        emissive: LinearRgba::new(4.2, 3.0, 0.5, 1.0),
        perceptual_roughness: 0.1,
        metallic: 0.8,
        ..default()
    });

    let torch_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.3, 0.18),
        perceptual_roughness: 0.8,
        ..default()
    });

    let fire_emissive_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.65, 0.1),
        emissive: LinearRgba::new(5.0, 2.5, 0.3, 1.0),
        ..default()
    });

    let stalagmite_mesh = meshes.add(Cone::new(0.4, 2.5).mesh());
    let stalactite_mesh = meshes.add(Cone::new(0.35, 2.2).mesh());
    let crystal_mesh = meshes.add(Cuboid::new(0.28, 1.4, 0.28));
    // 6. Calculate Dynamic Cave World Boundaries & Transforms
    let cave_offset_x = -(grid_cols as f32 * CAVE_CELL_SIZE) * 0.5;
    let cave_offset_z = -(grid_rows as f32 * CAVE_CELL_SIZE) * 0.5;

    let total_w = grid_cols as f32 * CAVE_CELL_SIZE;
    let total_d = grid_rows as f32 * CAVE_CELL_SIZE;

    // Cave Floor at Y = -150.0
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(total_w, 0.2, total_d))),
        MeshMaterial3d(floor_mat.clone()),
        Transform::from_xyz(0.0, CAVE_FLOOR_Y - 0.1, 0.0),
        RigidBody::Static,
        Collider::cuboid(total_w, 0.2, total_d),
        PlayModeEntity,
    ));

    // Cave Ceiling at Y = -142.0 (8m height)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(total_w, 0.2, total_d))),
        MeshMaterial3d(ceiling_mat.clone()),
        Transform::from_xyz(0.0, CAVE_CEILING_Y + 0.1, 0.0),
        RigidBody::Static,
        Collider::cuboid(total_w, 0.2, total_d),
        PlayModeEntity,
    ));

    // Ambient Subterranean Vault Lighting across ceiling grid
    for cx in (0..grid_cols).step_by(4) {
        for cz in (0..grid_rows).step_by(4) {
            let lx = cave_offset_x + (cx as f32 + 0.5) * CAVE_CELL_SIZE;
            let lz = cave_offset_z + (cz as f32 + 0.5) * CAVE_CELL_SIZE;
            commands.spawn((
                PointLight {
                    color: Color::srgb(0.35, 0.45, 0.65),
                    intensity: 2500.0,
                    range: 45.0,
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::from_xyz(lx, CAVE_CEILING_Y - 0.8, lz),
                PlayModeEntity,
            ));
        }
    }

    // 3. Build Cave Entrances and Exit Hubs
    let map_w = map.width as f32;
    let map_h = map.height as f32;
    let map_offset_x = -map_w * 0.5;
    let map_offset_z = -map_h * 0.5;

    let mut custom_cave_entrances: Vec<Vec3> = map
        .prefabs
        .iter()
        .filter(|p| p.prefab_type == "cave_entrance")
        .map(|p| Vec3::from_array(p.position))
        .collect();

    if custom_cave_entrances.is_empty() {
        // Find house or spawn point position to place cave entrances safely inland on island ground
        let mut base_pos = Vec3::new(-35.0, 0.0, -35.0);
        for p in map.prefabs.iter() {
            if p.prefab_type == "house" || p.prefab_type == "spawn_point" {
                base_pos = Vec3::from_array(p.position);
                break;
            }
        }

        let candidates = [
            Vec3::new(base_pos.x - 18.0, 0.0, base_pos.z - 15.0),
            Vec3::new(base_pos.x + 22.0, 0.0, base_pos.z - 12.0),
            Vec3::new(base_pos.x - 15.0, 0.0, base_pos.z + 20.0),
            Vec3::new(base_pos.x + 18.0, 0.0, base_pos.z + 18.0),
        ];

        for mut pos in candidates {
            // Ensure entrance position is on solid dry land (terrain height >= 1.5)
            let mut x_idx =
                ((pos.x - map_offset_x).round() as i32).clamp(1, map.width as i32 - 2) as u32;
            let mut z_idx =
                ((pos.z - map_offset_z).round() as i32).clamp(1, map.height as i32 - 2) as u32;
            let mut h = map.get_height(x_idx, z_idx);

            let mut step = 0;
            while h < 1.5 && step < 10 {
                pos.x += (base_pos.x - pos.x) * 0.25;
                pos.z += (base_pos.z - pos.z) * 0.25;
                x_idx =
                    ((pos.x - map_offset_x).round() as i32).clamp(1, map.width as i32 - 2) as u32;
                z_idx =
                    ((pos.z - map_offset_z).round() as i32).clamp(1, map.height as i32 - 2) as u32;
                h = map.get_height(x_idx, z_idx);
                step += 1;
            }

            custom_cave_entrances.push(pos);
        }
    }

    for (cave_id, mut surf_pos) in custom_cave_entrances.into_iter().enumerate() {
        let (c, r) = entrance_chamber_indices[cave_id % entrance_chamber_indices.len()];
        let cell_x = cave_offset_x + (c as f32 + 0.5) * CAVE_CELL_SIZE;
        let cell_z = cave_offset_z + (r as f32 + 0.5) * CAVE_CELL_SIZE;
        let cave_spawn = Vec3::new(cell_x, CAVE_FLOOR_Y + 0.2, cell_z);
        cave_data.cave_spawns.push(cave_spawn);

        let surf_x_idx =
            ((surf_pos.x - map_offset_x).round() as i32).clamp(0, map.width as i32 - 1) as u32;
        let surf_z_idx =
            ((surf_pos.z - map_offset_z).round() as i32).clamp(0, map.height as i32 - 1) as u32;
        surf_pos.y = map.get_height(surf_x_idx, surf_z_idx);

        cave_data.surface_entrances.push(surf_pos);

        // Spawn Surface Entrance Grotto Portal
        spawn_surface_cave_entrance(
            commands,
            meshes,
            materials,
            asset_server,
            surf_pos,
            cave_id as u32,
            cave_spawn,
        );

        // Spawn Underground Exit Hub with 4 Wall Torches & Bright Illumination
        spawn_underground_cave_exit_hub(
            commands,
            meshes,
            materials,
            cave_spawn,
            cave_id as u32,
            surf_pos + Vec3::new(0.0, 0.1, 2.0),
            torch_mat.clone(),
            fire_emissive_mat.clone(),
        );
    }

    // 7. Render 3D Cave Blocks, Passages, Chests & Environmental Features
    for (c, row) in grid.iter().enumerate().take(grid_cols) {
        for (r, &cell) in row.iter().enumerate().take(grid_rows) {
            let cell_x = cave_offset_x + (c as f32 + 0.5) * CAVE_CELL_SIZE;
            let cell_z = cave_offset_z + (r as f32 + 0.5) * CAVE_CELL_SIZE;
            let cell_center = Vec3::new(cell_x, CAVE_FLOOR_Y + 4.0, cell_z);

            if cell == 0 {
                // Solid Stone Wall Block (8m tall)
                commands.spawn((
                    Mesh3d(meshes.add(Cuboid::new(CAVE_CELL_SIZE, 8.0, CAVE_CELL_SIZE))),
                    MeshMaterial3d(wall_mat.clone()),
                    Transform::from_translation(cell_center),
                    RigidBody::Static,
                    Collider::cuboid(CAVE_CELL_SIZE, 8.0, CAVE_CELL_SIZE),
                    WallCollider {
                        half_extents: Vec3::new(CAVE_CELL_SIZE * 0.5, 4.0, CAVE_CELL_SIZE * 0.5),
                    },
                    PlayModeEntity,
                ));
            } else {
                // Open Passage or Chamber — populate with rich illumination & decorations
                let seed = (c * 73 + r * 101) % 100;

                // Bioluminescent Crystal Formations (Cyan, Purple, Emerald, Gold)
                if seed < 35 {
                    let (crystal_mat, light_color) = match seed % 4 {
                        0 => (crystal_cyan_mat.clone(), Color::srgb(0.2, 0.9, 1.0)),
                        1 => (crystal_purple_mat.clone(), Color::srgb(0.9, 0.2, 1.0)),
                        2 => (crystal_emerald_mat.clone(), Color::srgb(0.2, 1.0, 0.5)),
                        _ => (crystal_gold_mat.clone(), Color::srgb(1.0, 0.8, 0.2)),
                    };

                    let cry_pos = Vec3::new(
                        cell_x + (rand::random::<f32>() - 0.5) * 2.0,
                        CAVE_FLOOR_Y + 0.7,
                        cell_z + (rand::random::<f32>() - 0.5) * 2.0,
                    );

                    // Cluster of 4 mineable crystal spikes
                    for i in 0..4 {
                        let rot_x = (i as f32 * 0.25) - 0.3;
                        let rot_z = (i as f32 * 0.35) - 0.4;
                        let pos = cry_pos + Vec3::new(i as f32 * 0.18, 0.0, (i % 2) as f32 * 0.12);
                        commands.spawn((
                            Mesh3d(crystal_mesh.clone()),
                            MeshMaterial3d(crystal_mat.clone()),
                            Transform::from_translation(pos).with_rotation(Quat::from_euler(
                                EulerRot::XYZ,
                                rot_x,
                                i as f32 * 1.2,
                                rot_z,
                            )),
                            PlayResourceNode {
                                index: 0,
                                prefab_type: "crystal".to_string(),
                                health: 4,
                                position: pos,
                            },
                            PlayModeEntity,
                        ));
                    }

                    // Crystal Point Light with Shadow Mapping
                    commands.spawn((
                        PointLight {
                            color: light_color,
                            intensity: 3500.0,
                            range: 30.0,
                            shadow_maps_enabled: false,
                            ..default()
                        },
                        Transform::from_translation(cry_pos + Vec3::Y * 1.2),
                        PlayModeEntity,
                    ));
                }
                // Warm Floor-Grounded Torch Posts with Animated Fire
                else if (35..65).contains(&seed) {
                    let torch_base = Vec3::new(cell_x, CAVE_FLOOR_Y + 1.25, cell_z);

                    // Wooden Torch Pillar rooted to the cave floor
                    commands.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.12, 2.5, 0.12))),
                        MeshMaterial3d(torch_mat.clone()),
                        Transform::from_translation(torch_base),
                        PlayModeEntity,
                    ));

                    // Iron Sconce Bracket Cup
                    commands.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.24, 0.15, 0.24))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(0.2, 0.2, 0.22),
                            metallic: 0.8,
                            ..default()
                        })),
                        Transform::from_translation(torch_base + Vec3::Y * 1.25),
                        PlayModeEntity,
                    ));

                    // Animated Flame Mesh Top with Dynamic Flickering Light
                    let flame_pos = torch_base + Vec3::Y * 1.5;
                    commands.spawn((
                        Mesh3d(meshes.add(Sphere::new(0.22).mesh())),
                        MeshMaterial3d(fire_emissive_mat.clone()),
                        Transform::from_translation(flame_pos),
                        TorchFlameEffect {
                            base_intensity: 4500.0,
                            base_y: flame_pos.y,
                            seed: (c as f32 * 13.0 + r as f32 * 29.0),
                        },
                        PointLight {
                            color: Color::srgb(1.0, 0.68, 0.2),
                            intensity: 4500.0,
                            range: 38.0,
                            shadow_maps_enabled: false,
                            ..default()
                        },
                        PlayModeEntity,
                    ));
                }
                // Stalagmite Formations
                else if (65..82).contains(&seed) {
                    let st_pos = Vec3::new(
                        cell_x + (rand::random::<f32>() - 0.5) * 2.2,
                        CAVE_FLOOR_Y + 1.25,
                        cell_z + (rand::random::<f32>() - 0.5) * 2.2,
                    );
                    commands.spawn((
                        Mesh3d(stalagmite_mesh.clone()),
                        MeshMaterial3d(wall_mat.clone()),
                        Transform::from_translation(st_pos),
                        PlayModeEntity,
                    ));
                }
                // Stalactite Hanging Formations
                else if (82..92).contains(&seed) {
                    let st_pos = Vec3::new(
                        cell_x + (rand::random::<f32>() - 0.5) * 2.2,
                        CAVE_CEILING_Y - 1.1,
                        cell_z + (rand::random::<f32>() - 0.5) * 2.2,
                    );
                    commands.spawn((
                        Mesh3d(stalactite_mesh.clone()),
                        MeshMaterial3d(wall_mat.clone()),
                        Transform::from_translation(st_pos)
                            .with_rotation(Quat::from_rotation_x(std::f32::consts::PI)),
                        PlayModeEntity,
                    ));
                }
                // Ancient Treasure Coffers
                else if seed >= 92 {
                    let chest_pos = Vec3::new(cell_x, CAVE_FLOOR_Y + 0.35, cell_z);
                    commands.spawn((
                        Mesh3d(meshes.add(Cuboid::new(1.0, 0.65, 0.65))),
                        MeshMaterial3d(fire_emissive_mat.clone()),
                        Transform::from_translation(chest_pos),
                        CaveChest { is_opened: false },
                        PlayModeEntity,
                    ));

                    // Chest Ambient Spotlight
                    commands.spawn((
                        PointLight {
                            color: Color::srgb(1.0, 0.85, 0.3),
                            intensity: 3500.0,
                            range: 28.0,
                            shadow_maps_enabled: false,
                            ..default()
                        },
                        Transform::from_translation(chest_pos + Vec3::Y * 1.5),
                        PlayModeEntity,
                    ));
                }
            }
        }
    }
}

/// Spawns a rocky surface cave entrance structure with glowing portal marker.
fn spawn_surface_cave_entrance(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    pos: Vec3,
    cave_id: u32,
    target_cave_pos: Vec3,
) {
    let rock_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.38, 0.35),
        base_color_texture: Some(asset_server.load("textures/rock.png")),
        perceptual_roughness: 0.88,
        metallic: 0.05,
        ..default()
    });

    let dark_interior_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.04, 0.04, 0.06),
        perceptual_roughness: 0.99,
        ..default()
    });

    let portal_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.2, 0.8, 1.0, 0.6),
        emissive: LinearRgba::new(0.8, 3.5, 5.0, 1.0),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    let boulder_mesh = meshes.add(Sphere::new(1.0).mesh().ico(3).unwrap());

    // Dark cavern interior backdrop & solid back wall
    commands.spawn((
        Mesh3d(boulder_mesh.clone()),
        MeshMaterial3d(dark_interior_mat),
        Transform::from_translation(pos + Vec3::new(0.0, 1.6, -0.4))
            .with_scale(Vec3::new(1.6, 1.4, 1.0)),
        avian3d::prelude::RigidBody::Static,
        avian3d::prelude::Collider::cuboid(3.2, 2.8, 1.0),
        crate::play_mode::WallCollider {
            half_extents: Vec3::new(1.6, 1.4, 0.5),
        },
        PlayModeEntity,
    ));

    // Surrounding natural rock formations (Left, Right, Top Arch)
    commands.spawn((
        Mesh3d(boulder_mesh.clone()),
        MeshMaterial3d(rock_mat.clone()),
        Transform::from_translation(pos + Vec3::new(-1.6, 1.2, -0.2))
            .with_scale(Vec3::new(1.4, 1.8, 1.3))
            .with_rotation(Quat::from_rotation_y(0.4)),
        avian3d::prelude::RigidBody::Static,
        avian3d::prelude::Collider::cuboid(1.4, 1.8, 1.3),
        crate::play_mode::WallCollider {
            half_extents: Vec3::new(0.7, 0.9, 0.65),
        },
        PlayModeEntity,
    ));
    commands.spawn((
        Mesh3d(boulder_mesh.clone()),
        MeshMaterial3d(rock_mat.clone()),
        Transform::from_translation(pos + Vec3::new(1.6, 1.3, -0.2))
            .with_scale(Vec3::new(1.5, 1.9, 1.4))
            .with_rotation(Quat::from_rotation_y(-0.5)),
        avian3d::prelude::RigidBody::Static,
        avian3d::prelude::Collider::cuboid(1.5, 1.9, 1.4),
        crate::play_mode::WallCollider {
            half_extents: Vec3::new(0.75, 0.95, 0.7),
        },
        PlayModeEntity,
    ));
    commands.spawn((
        Mesh3d(boulder_mesh.clone()),
        MeshMaterial3d(rock_mat.clone()),
        Transform::from_translation(pos + Vec3::new(0.0, 3.1, 0.1))
            .with_scale(Vec3::new(2.2, 1.3, 1.5))
            .with_rotation(Quat::from_rotation_z(0.1)),
        avian3d::prelude::RigidBody::Static,
        avian3d::prelude::Collider::cuboid(2.2, 1.3, 1.5),
        crate::play_mode::WallCollider {
            half_extents: Vec3::new(1.1, 0.65, 0.75),
        },
        PlayModeEntity,
    ));
    commands.spawn((
        Mesh3d(boulder_mesh.clone()),
        MeshMaterial3d(rock_mat.clone()),
        Transform::from_translation(pos + Vec3::new(-2.2, 0.6, 0.4))
            .with_scale(Vec3::new(1.0, 0.9, 1.1)),
        PlayModeEntity,
    ));
    commands.spawn((
        Mesh3d(boulder_mesh.clone()),
        MeshMaterial3d(rock_mat),
        Transform::from_translation(pos + Vec3::new(2.1, 0.7, 0.5))
            .with_scale(Vec3::new(1.1, 1.0, 1.0)),
        PlayModeEntity,
    ));

    // Glowing Cave Portal Ring
    commands.spawn((
        Mesh3d(meshes.add(Torus::new(0.15, 1.3).mesh())),
        MeshMaterial3d(portal_mat),
        Transform::from_translation(pos + Vec3::new(0.0, 1.6, 0.1))
            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
        CaveEntranceMarker {
            _cave_id: cave_id,
            target_pos: target_cave_pos,
        },
        PlayModeEntity,
    ));

    // Portal Glow Light
    commands.spawn((
        PointLight {
            color: Color::srgb(0.2, 0.8, 1.0),
            intensity: 1800.0,
            range: 14.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_translation(pos + Vec3::new(0.0, 1.6, 0.1)),
        PlayModeEntity,
    ));
}

/// Spawns an underground cave exit (ladder/portal ring) with standing torches & bright hub lighting.
#[allow(clippy::too_many_arguments)]
fn spawn_underground_cave_exit_hub(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    cave_pos: Vec3,
    cave_id: u32,
    target_surf_pos: Vec3,
    torch_mat: Handle<StandardMaterial>,
    fire_mat: Handle<StandardMaterial>,
) {
    let ladder_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.35, 0.18),
        perceptual_roughness: 0.75,
        ..default()
    });

    let portal_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(1.0, 0.6, 0.2, 0.7),
        emissive: LinearRgba::new(5.0, 2.2, 0.5, 1.0),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    // Ladder vertical beams (7.5m high)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.12, 7.5, 0.12))),
        MeshMaterial3d(ladder_mat.clone()),
        Transform::from_translation(cave_pos + Vec3::new(-0.5, 3.75, 0.0)),
        PlayModeEntity,
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.12, 7.5, 0.12))),
        MeshMaterial3d(ladder_mat.clone()),
        Transform::from_translation(cave_pos + Vec3::new(0.5, 3.75, 0.0)),
        PlayModeEntity,
    ));

    // Ladder rungs
    for r in 0..14 {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(1.0, 0.08, 0.08))),
            MeshMaterial3d(ladder_mat.clone()),
            Transform::from_translation(cave_pos + Vec3::new(0.0, 0.5 + r as f32 * 0.5, 0.0)),
            PlayModeEntity,
        ));
    }

    // Glowing Exit Portal Ring
    commands.spawn((
        Mesh3d(meshes.add(Torus::new(0.15, 1.3).mesh())),
        MeshMaterial3d(portal_mat),
        Transform::from_translation(cave_pos + Vec3::new(0.0, 1.8, 0.0))
            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
        CaveExitMarker {
            _cave_id: cave_id,
            target_pos: target_surf_pos,
        },
        PlayModeEntity,
    ));

    // 4 Standing Torches around the Entrance Hub
    let offsets = [
        Vec3::new(-2.5, 0.0, -2.5),
        Vec3::new(2.5, 0.0, -2.5),
        Vec3::new(-2.5, 0.0, 2.5),
        Vec3::new(2.5, 0.0, 2.5),
    ];
    for off in offsets {
        let t_pos = cave_pos + off + Vec3::Y * 1.5;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.15, 2.5, 0.15))),
            MeshMaterial3d(torch_mat.clone()),
            Transform::from_translation(t_pos),
            PlayModeEntity,
        ));
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.25).mesh())),
            MeshMaterial3d(fire_mat.clone()),
            Transform::from_translation(t_pos + Vec3::Y * 1.3),
            PlayModeEntity,
        ));
        commands.spawn((
            PointLight {
                color: Color::srgb(1.0, 0.7, 0.3),
                intensity: 1800.0,
                range: 25.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(t_pos + Vec3::Y * 1.5),
            PlayModeEntity,
        ));
    }
}

/// System managing player headlamp / chest flashlight when exploring underground caves.
#[allow(clippy::type_complexity)]
pub fn cave_headlamp_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    player_query: Query<(&Transform, &PlayModePlayer)>,
    joint_query: Query<(Entity, &crate::play_mode::PlayJointVisual)>,
    headlamp_query: Query<Entity, With<CaveHeadlamp>>,
) {
    let Ok((player_transform, player)) = player_query.single() else {
        return;
    };

    let is_underground = player_transform.translation.y < -30.0;
    let should_illuminate = is_underground || player.is_headlamp_on;

    if should_illuminate {
        if headlamp_query.is_empty() {
            let mut head_entity = None;
            for (entity, joint) in joint_query.iter() {
                if joint.name == "Head" {
                    head_entity = Some(entity);
                    break;
                }
            }
            let Some(head_ent) = head_entity else {
                return;
            };

            // Spawn parent headlamp entity attached to player's Head joint node
            let headlamp_entity = commands
                .spawn((
                    PointLight {
                        color: Color::srgb(0.98, 0.96, 1.0),
                        intensity: 24000.0,
                        range: 50.0,
                        shadow_maps_enabled: false,
                        ..default()
                    },
                    Transform::from_xyz(0.0, 0.08, 0.16),
                    CaveHeadlamp,
                    PlayModeEntity,
                    Visibility::Visible,
                    InheritedVisibility::default(),
                ))
                .id();

            // Focused SpotLight child facing straight forward out from forehead
            let spot_child = commands
                .spawn((
                    SpotLight {
                        color: Color::srgb(0.98, 0.96, 1.0),
                        intensity: 95000.0,
                        range: 110.0,
                        inner_angle: 0.38,
                        outer_angle: 1.15, // Wide 130-degree beam scope
                        shadow_maps_enabled: true,
                        shadow_depth_bias: 0.02,
                        ..default()
                    },
                    Transform::from_xyz(0.0, 0.0, 0.05)
                        .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
                    PlayModeEntity,
                ))
                .id();
            commands.entity(headlamp_entity).add_child(spot_child);

            // Sleek Metallic Compact Headlamp Housing Unit
            let casing_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.22, 0.25, 0.28),
                perceptual_roughness: 0.3,
                metallic: 0.9,
                ..default()
            });
            let casing = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.12, 0.04, 0.05))),
                    MeshMaterial3d(casing_mat),
                    Transform::from_xyz(0.0, 0.0, 0.0),
                    PlayModeEntity,
                ))
                .id();
            commands.entity(headlamp_entity).add_child(casing);

            // Dual Ultra-Bright Glowing Xenon Lenses (Left and Right Lenses)
            let lens_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.98, 0.85),
                emissive: LinearRgba::new(12.0, 11.0, 7.0, 1.0),
                unlit: true,
                ..default()
            });

            for offset_x in [-0.035, 0.035] {
                let lens = commands
                    .spawn((
                        Mesh3d(meshes.add(Cylinder::new(0.018, 0.025))),
                        MeshMaterial3d(lens_mat.clone()),
                        Transform::from_xyz(offset_x, 0.0, 0.03)
                            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                        PlayModeEntity,
                    ))
                    .id();
                commands.entity(headlamp_entity).add_child(lens);
            }

            commands.entity(head_ent).add_child(headlamp_entity);
        }
    } else {
        // Despawn headlamp and its children cleanly when turned off
        for hl_entity in headlamp_query.iter() {
            commands.entity(hl_entity).despawn();
        }
    }
}

/// System animating cave torch flames with realistic chaotic flickering and bobbing
pub fn cave_torch_flame_animation_system(
    time: Res<Time>,
    mut flame_query: Query<(&TorchFlameEffect, &mut Transform, Option<&mut PointLight>)>,
) {
    let t = time.elapsed_secs();
    for (flame, mut transform, light_opt) in flame_query.iter_mut() {
        let flicker = (t * 16.0 + flame.seed).sin() * 0.35 + (t * 29.0).cos() * 0.25;
        transform.translation.y = flame.base_y + (t * 6.0 + flame.seed).sin() * 0.04;
        let scale_mod = 1.0 + flicker * 0.15;
        transform.scale = Vec3::splat(scale_mod);

        if let Some(mut light) = light_opt {
            light.intensity = (flame.base_intensity + flicker * 800.0).max(1000.0);
        }
    }
}

/// System handling [E] key interactions for surface cave entrances, cave exits, and cave chests.
#[allow(clippy::type_complexity)]
pub fn cave_interaction_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<(Entity, &mut PlayModePlayer, &mut Transform, &mut Position)>,
    mut inventory: ResMut<PlayerInventory>,
    entrance_query: Query<(&Transform, &CaveEntranceMarker), Without<PlayModePlayer>>,
    exit_query: Query<(&Transform, &CaveExitMarker), Without<PlayModePlayer>>,
    mut chest_query: Query<(&Transform, &mut CaveChest), Without<PlayModePlayer>>,
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

    // 1. Check Surface Cave Entrances
    for (ent_transform, entrance) in entrance_query.iter() {
        let d = player_pos.xz().distance(ent_transform.translation.xz());
        let dy = (player_pos.y - ent_transform.translation.y).abs();
        if d < 2.8 && dy < 3.5 {
            player.position = entrance.target_pos;
            let float_height = player.height * 0.5 + 0.08;
            let new_phys_pos = entrance.target_pos + Vec3::Y * float_height;
            player_transform.translation = new_phys_pos;
            phys_pos.0 = new_phys_pos;

            for n in player.nodes.iter_mut() {
                let diff = entrance.target_pos - player_pos;
                n.position += diff;
                n.old_position += diff;
            }

            inventory_log("🕳️ Entering underground cave maze system!");
            return;
        }
    }

    // 2. Check Underground Cave Exits
    for (exit_transform, exit) in exit_query.iter() {
        let d = player_pos.xz().distance(exit_transform.translation.xz());
        let dy = (player_pos.y - exit_transform.translation.y).abs();
        if d < 2.8 && dy < 3.5 {
            player.position = exit.target_pos;
            let float_height = player.height * 0.5 + 0.08;
            let new_phys_pos = exit.target_pos + Vec3::Y * float_height;
            player_transform.translation = new_phys_pos;
            phys_pos.0 = new_phys_pos;

            for n in player.nodes.iter_mut() {
                let diff = exit.target_pos - player_pos;
                n.position += diff;
                n.old_position += diff;
            }

            inventory_log("🪜 Climbing ladder back up to surface world!");
            return;
        }
    }

    // 3. Check Cave Treasure Chests
    for (chest_transform, mut chest) in chest_query.iter_mut() {
        let d = player_pos.xz().distance(chest_transform.translation.xz());
        let dy = (player_pos.y - chest_transform.translation.y).abs();
        if d < 2.5 && dy < 2.8 {
            if !chest.is_opened {
                chest.is_opened = true;
                inventory.gold += 15;
                inventory.silver += 20;
                inventory.platinum += 5;
                inventory.steel += 10;
                inventory.copper += 12;
                player.health_packs += 1;
                inventory_log(
                    "💎 Opened Ancient Cave Coffer! Received +15 Gold, +20 Silver, +5 Platinum, +10 Steel, +12 Copper, and +1 Health Pack!",
                );
            } else {
                inventory_log("📭 The cave coffer is empty.");
            }
            return;
        }
    }
}
