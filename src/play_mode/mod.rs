use crate::AppState;
use crate::character_designer::{
    CharacterSettings, Gender, HairStyle, build_skeletal_limb_mesh, build_stylized_bone_mesh,
};
use crate::map_editor::data::TempestMap;
use crate::map_editor::{SplatmapSettings, WaterImpulseEvent, WaterSettings, generate_water_mesh};
use crate::{ControlScheme, ControlSchemeConfig};
use bevy::asset::RenderAssetUsages;
use bevy::ecs::relationship::Relationship;
use bevy::prelude::WorldAssetRoot;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::{CursorGrabMode, CursorOptions};
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

pub mod creatures;
pub mod house;

pub struct PlayModePlugin;

impl Plugin for PlayModePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerInventory>()
            .add_plugins(house::HousePlugin)
            .add_systems(
                OnEnter(AppState::PlayMode),
                (
                    house::flatten_terrain,
                    setup_play_mode,
                    creatures::spawn_creatures_system,
                    creatures::setup_fox_animations,
                    creatures::setup_trilobite_animations,
                )
                    .chain(),
            )
            .add_systems(
                OnExit(AppState::PlayMode),
                (cleanup_play_mode, release_mouse_on_exit),
            )
            .add_systems(
                EguiPrimaryContextPass,
                play_mode_hud_ui.run_if(in_state(AppState::PlayMode)),
            )
            .add_systems(
                Update,
                (
                    player_movement_and_ragdoll_system,
                    axe_swing_system,
                    play_visual_sync_system,
                    play_weapon_sync_system,
                    weapon_attachment_system,
                    particle_update_system,
                    sync_logs,
                    creatures::creature_ai_system,
                    creatures::creature_animation_sync_system,
                    creatures::creature_skeletal_animation_system,
                    gun_fire_and_bullet_system,
                    play_particle_update_system,
                    crate::map_editor::configure_terrain_sampler_system,
                    update_drops_system,
                    poll_terrain_load_system,
                )
                    .run_if(in_state(AppState::PlayMode)),
            )
            .add_systems(
                Update,
                (
                    creatures::attach_fox_animation_player,
                    creatures::drive_fox_animations,
                    creatures::attach_trilobite_animation_player,
                    creatures::drive_trilobite_animations,
                    creatures::spawn_defender_trilobite,
                    creatures::trilobite_combat_system,
                    add_physics_to_wall_colliders,
                    play_mode_mouse_grab_system,
                    cloud_drift_system,
                    play_sky_cycle_system,
                    crate::map_editor::water_simulation_system,
                    crate::map_editor::animate_water_mesh_system,
                )
                    .run_if(in_state(AppState::PlayMode)),
            )
            .add_systems(
                PostUpdate,
                camera_follow_system
                    .before(bevy::transform::TransformSystems::Propagate)
                    .run_if(in_state(AppState::PlayMode)),
            );
    }
}

// Marker component for despawning Play Mode entities
#[derive(Component)]
pub struct PlayModeEntity;

#[derive(Component)]
pub struct AmmoDrop {
    pub ammo_pistol: u32,
    pub ammo_revolver: u32,
    pub ammo_rifle: u32,
    pub ammo_sniper: u32,
    pub wood: u32,
    pub copper: u32,
    pub iron: u32,
}

#[derive(Component)]
pub struct SpinDrop;

// Resource tracking harvested items
#[derive(Resource, Default, Debug)]
pub struct PlayerInventory {
    pub wood: u32,
    pub rock: u32,
    pub copper: u32,
    pub iron: u32,
    pub gold: u32,
    pub silver: u32,
    pub platinum: u32,
    pub granite: u32,
    pub steel: u32,
    // Crafting outputs
    pub wooden_shelter_parts: u32,
    pub metal_shelter_parts: u32,
    pub has_sword: bool,
    pub loot_log: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerState {
    Active,
    Ragdoll,
    Swimming,
    Flying,
}

// Verlet physics node matching the design parameters
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PlayVerletNode {
    pub name: String,
    pub position: Vec3,
    pub old_position: Vec3,
    pub radius: f32,
    pub start_local: Vec3, // relative to Pelvis in design pose
}

#[derive(Debug, Clone)]
pub struct PlayVerletConstraint {
    pub node_a: usize,
    pub node_b: usize,
    pub target_length: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Reflect)]
pub enum ActiveWeapon {
    #[default]
    Melee,
    Pistol,
    Revolver,
    Rifle,
    Sniper,
}

#[derive(Component)]
pub struct PlayWeaponVisual {
    pub weapon_type: ActiveWeapon,
    pub is_sword: bool,
}

#[derive(Component)]
pub struct Bullet {
    pub velocity: Vec3,
    pub gravity: f32,
    pub lifetime: f32,
    pub damage: f32,
}

#[allow(dead_code)]
#[derive(Component)]
pub struct PlayModePlayer {
    pub state: PlayerState,
    pub position: Vec3,
    pub rotation_yaw: f32,
    pub walk_timer: f32,
    pub is_walking: bool,
    pub nodes: Vec<PlayVerletNode>,
    pub constraints: Vec<PlayVerletConstraint>,
    pub height: f32,
    pub weight: f32,
    pub head_scale: f32,
    pub axe_swing_timer: Option<f32>, // Some(time_elapsed)
    pub axe_has_struck: bool,
    pub stand_up_timer: f32,
    pub velocity_y: f32,
    // New Weapon/Health System Fields
    pub health: f32,
    pub max_health: f32,
    pub active_weapon: ActiveWeapon,
    pub ammo_pistol: u32,
    pub ammo_revolver: u32,
    pub ammo_rifle: u32,
    pub ammo_sniper: u32,
    pub clip_pistol: u32,
    pub clip_revolver: u32,
    pub clip_rifle: u32,
    pub clip_sniper: u32,
    pub reload_timer: Option<f32>,
    pub automatic_fire_timer: f32,
    pub swim_sound_entity: Option<Entity>,
    pub wade_sound_timer: f32,
}

// Parent tag for sync
#[derive(Component)]
pub struct PlayJointVisual {
    pub name: String,
}

#[allow(dead_code)]
#[derive(Resource)]
pub struct PlayWeaponAssets {
    pub pistol: Handle<WorldAsset>,
    pub revolver: Handle<WorldAsset>,
    pub rifle: Handle<WorldAsset>,
    pub sniper: Handle<WorldAsset>,
}

#[derive(Resource)]
pub struct TerrainLoadChannel {
    pub rx: std::sync::Mutex<std::sync::mpsc::Receiver<(Mesh, Vec<crate::grass::GrassChunkData>)>>,
}

#[derive(Component)]
pub struct PlayLimbVisual {
    pub node_a: String,
    pub node_b: String,
    pub radius: f32,
}

#[allow(dead_code)]
#[derive(Component)]
pub struct PlayAxeVisual;

#[allow(dead_code)]
#[derive(Component)]
pub struct PlayWeaponBase {
    pub is_sword: bool,
}

// Marker tag for resource nodes spawned in Play Mode
#[allow(dead_code)]
#[derive(Component)]
pub struct PlayResourceNode {
    pub index: usize,
    pub prefab_type: String,
    pub position: Vec3,
    pub health: i32,
}

#[allow(dead_code)]
#[derive(Component)]
pub struct PlayParticle {
    pub velocity: Vec3,
    pub lifetime: f32,
    pub max_lifetime: f32,
    pub color: Color,
}

#[allow(dead_code)]
#[derive(Component)]
pub struct PlaySun {
    pub id: usize,
    pub angle_offset: f32,
    pub orbit_speed: f32,
    pub base_color: Color,
    pub day_intensity: f32,
}

#[derive(Component)]
pub struct WallCollider {
    pub half_extents: Vec3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    ThirdPerson,
    FirstPerson,
    Orbit,
}

#[derive(Component)]
pub struct PlayModeCamera {
    pub target_distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub view_mode: ViewMode,
}

// Setup system to initialize play world dynamically
#[allow(clippy::too_many_arguments)]
fn setup_play_mode(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    map: Res<TempestMap>,
    char_settings: Res<CharacterSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut inventory: ResMut<PlayerInventory>,
    mansion_settings: Res<crate::play_mode::house::MansionSettings>,
    mut control_configs: ResMut<Assets<ControlSchemeConfig>>,
    rt: Res<crate::TokioRuntime>,
) {
    let h_scale = char_settings.height;
    let config_handle = control_configs.add(ControlSchemeConfig {
        basis: bevy_tnua::builtins::TnuaBuiltinWalkConfig {
            speed: 12.0,
            float_height: h_scale * 0.5 + 0.08,
            max_slope: std::f32::consts::FRAC_PI_4,
            cling_distance: 1.5,
            acceleration: 80.0,
            air_acceleration: 40.0,
            turning_angvel: 16.0,
            ..default()
        },
        jump: bevy_tnua::builtins::TnuaBuiltinJumpConfig {
            height: 2.2,
            ..default()
        },
        crouch: bevy_tnua::builtins::TnuaBuiltinCrouchConfig {
            float_offset: -h_scale * 0.25,
            ..default()
        },
    });

    inventory.loot_log.clear();
    inventory.loot_log.push(
        "🎮 Welcome to Play Mode! Press [E] to interact. Hold axe to harvest elements.".to_string(),
    );

    // Cache weapon assets to prevent unload use-after-free
    let weapon_assets = PlayWeaponAssets {
        pistol: asset_server.load("Gun_Pistol.gltf#Scene0"),
        revolver: asset_server.load("Gun_Revolver.gltf#Scene0"),
        rifle: asset_server.load("Gun_Rifle.gltf#Scene0"),
        sniper: asset_server.load("Gun_Sniper.gltf#Scene0"),
    };
    commands.insert_resource(weapon_assets);

    // 1. Set ambient light & clear color for the alien twilight sky
    commands.insert_resource(bevy::light::GlobalAmbientLight {
        color: Color::srgb(0.35, 0.25, 0.45),
        brightness: 450.0,
        ..default()
    });
    commands.insert_resource(ClearColor(Color::srgb(0.08, 0.05, 0.14)));

    // 2. Golden Sun Sphere & Light with PlaySun component
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(4.0).mesh().ico(3).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.8, 0.6),
            emissive: LinearRgba::from(Color::srgb(10.0, 8.0, 6.0)),
            unlit: true,
            ..default()
        })),
        Transform::from_xyz(60.0, 50.0, 40.0),
        PlaySun {
            id: 0,
            angle_offset: 0.0,
            orbit_speed: 1.0,
            base_color: Color::srgb(1.0, 0.8, 0.6),
            day_intensity: 9500.0,
        },
        PlayModeEntity,
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 9500.0,
            color: Color::srgb(1.0, 0.85, 0.65),
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(60.0, 50.0, 40.0).looking_at(Vec3::ZERO, Vec3::Y),
        PlaySun {
            id: 0,
            angle_offset: 0.0,
            orbit_speed: 1.0,
            base_color: Color::srgb(1.0, 0.8, 0.6),
            day_intensity: 9500.0,
        },
        PlayModeEntity,
    ));

    // 3. Cyan Sun Sphere & Light with PlaySun component
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(2.5).mesh().ico(3).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.4, 0.9, 1.0),
            emissive: LinearRgba::from(Color::srgb(4.0, 9.0, 10.0)),
            unlit: true,
            ..default()
        })),
        Transform::from_xyz(-70.0, 45.0, -50.0),
        PlaySun {
            id: 1,
            angle_offset: 2.2,
            orbit_speed: 1.3,
            base_color: Color::srgb(0.4, 0.9, 1.0),
            day_intensity: 6500.0,
        },
        PlayModeEntity,
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 6500.0,
            color: Color::srgb(0.45, 0.92, 1.0),
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-70.0, 45.0, -50.0).looking_at(Vec3::ZERO, Vec3::Y),
        PlaySun {
            id: 1,
            angle_offset: 2.2,
            orbit_speed: 1.3,
            base_color: Color::srgb(0.4, 0.9, 1.0),
            day_intensity: 6500.0,
        },
        PlayModeEntity,
    ));

    // 4. Spawn 3D Terrain & Grass (Asynchronously in Background)
    let (tx, rx) = std::sync::mpsc::channel();
    let map_clone = map.clone();
    let tokio_handle = rt.0.clone();

    tokio_handle.spawn(async move {
        let splat_settings = SplatmapSettings::default();
        let terrain_mesh = crate::map_editor::generate_terrain_mesh(&map_clone, &splat_settings);
        let grass_chunks = crate::grass::generate_grass_chunks(&map_clone);
        let _ = tx.send((terrain_mesh, grass_chunks));
    });

    commands.insert_resource(TerrainLoadChannel {
        rx: std::sync::Mutex::new(rx),
    });

    // Spawn 3D Terrain (Physical Heightfield Collider)
    let mut heights = vec![vec![0.0; map.height as usize]; map.width as usize];
    for z in 0..map.height {
        for x in 0..map.width {
            heights[x as usize][z as usize] = map.get_height(x, z);
        }
    }
    let heightfield_scale = Vec3::new(map.width as f32 - 1.0, 1.0, map.height as f32 - 1.0);
    commands.spawn((
        avian3d::prelude::RigidBody::Static,
        avian3d::prelude::Collider::heightfield(heights, heightfield_scale),
        Transform::from_xyz(-0.5, 0.0, -0.5),
        PlayModeEntity,
    ));

    // 5. Spawn Translucent Interactive Water
    let water_mesh_handle = if map.width > 256 || map.height > 256 {
        meshes.add(
            Plane3d::default()
                .mesh()
                .size(map.width as f32, map.height as f32),
        )
    } else {
        let water_mesh = generate_water_mesh(map.width, map.height);
        meshes.add(water_mesh)
    };
    commands.spawn((
        Mesh3d(water_mesh_handle),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.02, 0.32, 0.78, 0.78),
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 0.08,
            metallic: 0.1,
            ..default()
        })),
        Transform::from_xyz(0.0, 1.2, 0.0), // match standard water height
        crate::map_editor::WaterMesh,
        crate::map_editor::WaterSimData::new(map.width, map.height),
        PlayModeEntity,
    ));

    // Spawn Play Mode Bridges (road == 3)
    let bridge_mat = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/solid_stone.png")),
        perceptual_roughness: 0.9,
        ..default()
    });
    let bridge_mesh = meshes.add(Cuboid::new(2.4, 0.3, 1.05));

    let offset_x = -(map.width as f32) / 2.0;
    let offset_z = -(map.height as f32) / 2.0;

    for z in 0..map.height {
        for x in 0..map.width {
            if map.get_road(x, z) == 3 {
                let vx = x as f32 + offset_x;
                let vz = z as f32 + offset_z;

                let is_east_west = (x > 0 && map.get_road(x - 1, z) > 0)
                    || (x < map.width - 1 && map.get_road(x + 1, z) > 0);
                let rot = if is_east_west {
                    Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)
                } else {
                    Quat::IDENTITY
                };

                commands.spawn((
                    Mesh3d(bridge_mesh.clone()),
                    MeshMaterial3d(bridge_mat.clone()),
                    Transform::from_xyz(vx, 1.35, vz).with_rotation(rot), // 1.35m height is just above water (1.2m)
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(2.4, 0.3, 1.05),
                    PlayModeEntity,
                ));
            }
        }
    }

    // 6. Spawn Procedural Cloud Plane
    let perlin = crate::map_editor::noise::PerlinNoise::new(12345);
    let cloud_image = generate_cloud_texture(&perlin);
    let cloud_image_handle = images.add(cloud_image);
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(800.0, 800.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 1.0, 1.0, 0.8),
            base_color_texture: Some(cloud_image_handle),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            cull_mode: None, // Double-sided rendering so it is visible from below!
            ..default()
        })),
        Transform::from_xyz(0.0, 25.0, 0.0),
        PlayModeCloud,
        PlayModeEntity,
    ));

    // 4. Find Spawn Position
    let mut spawn_pos = Vec3::new(0.0, 2.0, 0.0);
    for p in map.prefabs.iter() {
        if p.prefab_type == "spawn_point" {
            spawn_pos = Vec3::from_array(p.position);
            break;
        }
    }
    // Clamp to terrain height
    // Clamp to terrain height (offsetting to capsule hover height plus a 0.5 unit drop cushion to prevent spawning inside the ground)
    let terrain_y = get_bilinear_height(spawn_pos.x, spawn_pos.z, &map);
    spawn_pos.y = terrain_y + (char_settings.height * 0.5 + 0.08) + 0.5;

    // Find the house position first
    let mut house_pos = Vec3::new(0.0, 1.5, 0.0);
    let mut house_placed = false;
    for p in map.prefabs.iter() {
        if p.prefab_type == "house" {
            house_pos = Vec3::from_array(p.position);
            house_placed = true;
            break;
        }
    }

    let half_w = (mansion_settings.cols as f32 * mansion_settings.cell_size) / 2.0;
    let half_d = (mansion_settings.rows as f32 * mansion_settings.cell_size) / 2.0;

    // 5. Spawn all Placed Prefabs (Resource Nodes)
    for (idx, p) in map.prefabs.iter().enumerate() {
        if p.prefab_type == "spawn_point" || p.prefab_type == "house" {
            continue;
        }

        let p_pos = Vec3::from_array(p.position);

        // Skip spawning if it overlaps the house footprint
        if house_placed {
            let inside = (p_pos.x - house_pos.x).abs() < half_w + 1.0
                && (p_pos.z - house_pos.z).abs() < half_d + 1.0;
            if inside {
                continue;
            }
        }

        let node_parent = spawn_play_prefab(
            &mut commands,
            &mut meshes,
            &mut materials,
            &p.prefab_type,
            p_pos,
            p.rotation_y.unwrap_or(0.0),
        );

        // Spawn a resource tracker entity attached to it
        commands.entity(node_parent).insert((
            PlayResourceNode {
                index: idx,
                prefab_type: p.prefab_type.clone(),
                position: p_pos,
                health: if p.prefab_type.starts_with("tree") {
                    3
                } else {
                    4
                },
            },
            PlayModeEntity,
        ));
    }

    // 6. Build customized Player model
    let h_scale = char_settings.height;
    let w_thick = char_settings.weight;
    let head_scale = char_settings.head_scale;

    // Build Verlet node list similar to character designer structure
    let nodes = vec![
        PlayVerletNode {
            name: "Pelvis".to_string(),
            position: spawn_pos + Vec3::new(0.0, h_scale * 0.5, 0.0),
            old_position: spawn_pos + Vec3::new(0.0, h_scale * 0.5, 0.0),
            radius: 0.15 * w_thick,
            start_local: Vec3::new(0.0, h_scale * 0.5, 0.0),
        },
        PlayVerletNode {
            name: "Spine".to_string(),
            position: spawn_pos + Vec3::new(0.0, h_scale * 0.65, 0.0),
            old_position: spawn_pos + Vec3::new(0.0, h_scale * 0.65, 0.0),
            radius: 0.16 * w_thick,
            start_local: Vec3::new(0.0, h_scale * 0.65, 0.0),
        },
        PlayVerletNode {
            name: "Chest".to_string(),
            position: spawn_pos + Vec3::new(0.0, h_scale * 0.8, 0.0),
            old_position: spawn_pos + Vec3::new(0.0, h_scale * 0.8, 0.0),
            radius: 0.18 * w_thick,
            start_local: Vec3::new(0.0, h_scale * 0.8, 0.0),
        },
        PlayVerletNode {
            name: "Head".to_string(),
            position: spawn_pos + Vec3::new(0.0, h_scale * 0.98, 0.0),
            old_position: spawn_pos + Vec3::new(0.0, h_scale * 0.98, 0.0),
            radius: 0.14 * head_scale,
            start_local: Vec3::new(0.0, h_scale * 0.98, 0.0),
        },
        PlayVerletNode {
            name: "L_Shoulder".to_string(),
            position: spawn_pos + Vec3::new(-0.25 * w_thick, h_scale * 0.8, 0.0),
            old_position: spawn_pos + Vec3::new(-0.25 * w_thick, h_scale * 0.8, 0.0),
            radius: 0.08 * w_thick,
            start_local: Vec3::new(-0.25 * w_thick, h_scale * 0.8, 0.0),
        },
        PlayVerletNode {
            name: "L_Elbow".to_string(),
            position: spawn_pos + Vec3::new(-0.5 * w_thick, h_scale * 0.8, 0.0),
            old_position: spawn_pos + Vec3::new(-0.5 * w_thick, h_scale * 0.8, 0.0),
            radius: 0.07 * w_thick,
            start_local: Vec3::new(-0.5 * w_thick, h_scale * 0.8, 0.0),
        },
        PlayVerletNode {
            name: "R_Shoulder".to_string(),
            position: spawn_pos + Vec3::new(0.25 * w_thick, h_scale * 0.8, 0.0),
            old_position: spawn_pos + Vec3::new(0.25 * w_thick, h_scale * 0.8, 0.0),
            radius: 0.08 * w_thick,
            start_local: Vec3::new(0.25 * w_thick, h_scale * 0.8, 0.0),
        },
        PlayVerletNode {
            name: "R_Elbow".to_string(),
            position: spawn_pos + Vec3::new(0.5 * w_thick, h_scale * 0.8, 0.0),
            old_position: spawn_pos + Vec3::new(0.5 * w_thick, h_scale * 0.8, 0.0),
            radius: 0.07 * w_thick,
            start_local: Vec3::new(0.5 * w_thick, h_scale * 0.8, 0.0),
        },
        PlayVerletNode {
            name: "L_Hip".to_string(),
            position: spawn_pos + Vec3::new(-0.16 * w_thick, h_scale * 0.45, 0.0),
            old_position: spawn_pos + Vec3::new(-0.16 * w_thick, h_scale * 0.45, 0.0),
            radius: 0.1 * w_thick,
            start_local: Vec3::new(-0.16 * w_thick, h_scale * 0.45, 0.0),
        },
        PlayVerletNode {
            name: "L_Knee".to_string(),
            position: spawn_pos + Vec3::new(-0.16 * w_thick, h_scale * 0.22, 0.0),
            old_position: spawn_pos + Vec3::new(-0.16 * w_thick, h_scale * 0.22, 0.0),
            radius: 0.09 * w_thick,
            start_local: Vec3::new(-0.16 * w_thick, h_scale * 0.22, 0.0),
        },
        PlayVerletNode {
            name: "L_Foot".to_string(),
            position: spawn_pos + Vec3::new(-0.16 * w_thick, 0.0, 0.0),
            old_position: spawn_pos + Vec3::new(-0.16 * w_thick, 0.0, 0.0),
            radius: 0.08 * w_thick,
            start_local: Vec3::new(-0.16 * w_thick, 0.0, 0.0),
        },
        PlayVerletNode {
            name: "R_Hip".to_string(),
            position: spawn_pos + Vec3::new(0.16 * w_thick, h_scale * 0.45, 0.0),
            old_position: spawn_pos + Vec3::new(0.16 * w_thick, h_scale * 0.45, 0.0),
            radius: 0.1 * w_thick,
            start_local: Vec3::new(0.16 * w_thick, h_scale * 0.45, 0.0),
        },
        PlayVerletNode {
            name: "R_Knee".to_string(),
            position: spawn_pos + Vec3::new(0.16 * w_thick, h_scale * 0.22, 0.0),
            old_position: spawn_pos + Vec3::new(0.16 * w_thick, h_scale * 0.22, 0.0),
            radius: 0.09 * w_thick,
            start_local: Vec3::new(0.16 * w_thick, h_scale * 0.22, 0.0),
        },
        PlayVerletNode {
            name: "R_Foot".to_string(),
            position: spawn_pos + Vec3::new(0.16 * w_thick, 0.0, 0.0),
            old_position: spawn_pos + Vec3::new(0.16 * w_thick, 0.0, 0.0),
            radius: 0.08 * w_thick,
            start_local: Vec3::new(0.16 * w_thick, 0.0, 0.0),
        },
        PlayVerletNode {
            name: "L_Hand".to_string(),
            position: spawn_pos + Vec3::new(-0.7 * w_thick, h_scale * 0.8, 0.0),
            old_position: spawn_pos + Vec3::new(-0.7 * w_thick, h_scale * 0.8, 0.0),
            radius: 0.06 * w_thick,
            start_local: Vec3::new(-0.7 * w_thick, h_scale * 0.8, 0.0),
        },
        PlayVerletNode {
            name: "R_Hand".to_string(),
            position: spawn_pos + Vec3::new(0.7 * w_thick, h_scale * 0.8, 0.0),
            old_position: spawn_pos + Vec3::new(0.7 * w_thick, h_scale * 0.8, 0.0),
            radius: 0.06 * w_thick,
            start_local: Vec3::new(0.7 * w_thick, h_scale * 0.8, 0.0),
        },
    ];

    // Establish skeleton connection lengths
    let mut constraints = Vec::new();
    let connections = vec![
        ("Pelvis", "Spine"),
        ("Spine", "Chest"),
        ("Chest", "Head"),
        ("Chest", "L_Shoulder"),
        ("L_Shoulder", "L_Elbow"),
        ("L_Elbow", "L_Hand"),
        ("Chest", "R_Shoulder"),
        ("R_Shoulder", "R_Elbow"),
        ("R_Elbow", "R_Hand"),
        ("Pelvis", "L_Hip"),
        ("L_Hip", "L_Knee"),
        ("L_Knee", "L_Foot"),
        ("Pelvis", "R_Hip"),
        ("R_Hip", "R_Knee"),
        ("R_Knee", "R_Foot"),
    ];

    for (a_name, b_name) in connections {
        let idx_a = nodes.iter().position(|n| n.name == a_name).unwrap();
        let idx_b = nodes.iter().position(|n| n.name == b_name).unwrap();
        let dist = nodes[idx_a].position.distance(nodes[idx_b].position);
        constraints.push(PlayVerletConstraint {
            node_a: idx_a,
            node_b: idx_b,
            target_length: dist,
        });
    }

    // Add extra stabilization constraints to keep torso rigid
    let cross_links = vec![
        ("L_Shoulder", "R_Shoulder"),
        ("L_Hip", "R_Hip"),
        ("Chest", "L_Hip"),
        ("Chest", "R_Hip"),
    ];
    for (a_name, b_name) in cross_links {
        if let (Some(idx_a), Some(idx_b)) = (
            nodes.iter().position(|n| n.name == a_name),
            nodes.iter().position(|n| n.name == b_name),
        ) {
            let dist = nodes[idx_a].position.distance(nodes[idx_b].position);
            constraints.push(PlayVerletConstraint {
                node_a: idx_a,
                node_b: idx_b,
                target_length: dist,
            });
        }
    }

    // 7. Spawn Player Meshes (Bones/Muscles if show_xray is true, otherwise Skin spheres)
    let skin_mat = materials.add(StandardMaterial {
        base_color: char_settings.skin_color,
        perceptual_roughness: 0.7,
        ..default()
    });

    let shirt_mat = materials.add(StandardMaterial {
        base_color: if char_settings.gender == Gender::Male {
            Color::srgb(0.1, 0.5, 0.8)
        } else {
            Color::srgb(0.8, 0.15, 0.45)
        },
        perceptual_roughness: 0.6,
        ..default()
    });

    let pants_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.18, 0.22),
        perceptual_roughness: 0.8,
        ..default()
    });

    let eye_mat = materials.add(StandardMaterial {
        base_color: char_settings.eye_color,
        perceptual_roughness: 0.1,
        ..default()
    });

    let hair_mat = materials.add(StandardMaterial {
        base_color: char_settings.hair_color,
        perceptual_roughness: 0.85,
        ..default()
    });

    let bone_mat = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.85,
        ..default()
    });

    let muscle_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.72, 0.1, 0.12),
        perceptual_roughness: 0.6,
        metallic: 0.1,
        ..default()
    });

    // We hold reference to spawned nodes to parent them
    let mut visual_nodes = std::collections::HashMap::new();

    // Loop nodes and spawn either bone shapes or skin spheres
    for node in nodes.iter() {
        let is_head = node.name == "Head";
        let is_torso = node.name == "Pelvis" || node.name == "Spine" || node.name == "Chest";
        let is_pants_area = node.name == "Pelvis" || node.name == "L_Hip" || node.name == "R_Hip";

        let skin_mat_to_use = if is_head {
            skin_mat.clone()
        } else if is_pants_area {
            pants_mat.clone()
        } else if is_torso {
            shirt_mat.clone()
        } else {
            skin_mat.clone()
        };

        let mesh_radius = node.radius;

        let node_id = if char_settings.show_xray {
            // Spawn stylized ivory bone
            let bone_mesh = build_stylized_bone_mesh(&node.name, mesh_radius);
            commands
                .spawn((
                    Mesh3d(meshes.add(bone_mesh)),
                    MeshMaterial3d(bone_mat.clone()),
                    Transform::from_translation(node.position),
                    PlayJointVisual {
                        name: node.name.clone(),
                    },
                    PlayModeEntity,
                ))
                .id()
        } else {
            // Spawn solid skin sphere
            let sphere_mesh = meshes.add(Sphere::new(mesh_radius).mesh().ico(4).unwrap());
            commands
                .spawn((
                    Mesh3d(sphere_mesh),
                    MeshMaterial3d(skin_mat_to_use),
                    Transform::from_translation(node.position),
                    PlayJointVisual {
                        name: node.name.clone(),
                    },
                    PlayModeEntity,
                ))
                .id()
        };

        visual_nodes.insert(node.name.clone(), node_id);

        // Spawn hair & eyes under the Head node
        if is_head {
            let eye_mesh = meshes.add(Sphere::new(mesh_radius * 0.2).mesh().ico(3).unwrap());

            let le = commands
                .spawn((
                    Mesh3d(eye_mesh.clone()),
                    MeshMaterial3d(eye_mat.clone()),
                    Transform::from_translation(Vec3::new(
                        -mesh_radius * 0.35,
                        mesh_radius * 0.15,
                        mesh_radius * 0.85,
                    )),
                    PlayModeEntity,
                ))
                .id();
            commands.entity(node_id).add_child(le);

            let re = commands
                .spawn((
                    Mesh3d(eye_mesh),
                    MeshMaterial3d(eye_mat.clone()),
                    Transform::from_translation(Vec3::new(
                        mesh_radius * 0.35,
                        mesh_radius * 0.15,
                        mesh_radius * 0.85,
                    )),
                    PlayModeEntity,
                ))
                .id();
            commands.entity(node_id).add_child(re);

            match char_settings.hair_style {
                HairStyle::None => {}
                HairStyle::Short => {
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
                            PlayModeEntity,
                        ))
                        .id();
                    commands.entity(node_id).add_child(cap);
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
                                mesh_radius * 0.25,
                                -mesh_radius * 0.1,
                            )),
                            PlayModeEntity,
                        ))
                        .id();
                    commands.entity(node_id).add_child(cap);

                    let tail_mesh =
                        meshes.add(Sphere::new(mesh_radius * 0.35).mesh().ico(3).unwrap());
                    let tail = commands
                        .spawn((
                            Mesh3d(tail_mesh),
                            MeshMaterial3d(hair_mat.clone()),
                            Transform::from_translation(Vec3::new(
                                0.0,
                                -mesh_radius * 0.25,
                                -mesh_radius * 1.15,
                            )),
                            PlayModeEntity,
                        ))
                        .id();
                    commands.entity(node_id).add_child(tail);
                }
                HairStyle::Spiky => {
                    let cap_mesh =
                        meshes.add(Sphere::new(mesh_radius * 1.02).mesh().ico(4).unwrap());
                    let cap = commands
                        .spawn((
                            Mesh3d(cap_mesh),
                            MeshMaterial3d(hair_mat.clone()),
                            Transform::from_translation(Vec3::new(0.0, mesh_radius * 0.22, 0.0)),
                            PlayModeEntity,
                        ))
                        .id();
                    commands.entity(node_id).add_child(cap);

                    // Create multiple layers of spikes for a proper spiky look
                    for layer in 0..3 {
                        let layer_radius = mesh_radius * (0.95 - layer as f32 * 0.22);
                        let spike_count = if layer == 0 { 14 } else { 9 - layer * 2 };

                        for i in 0..spike_count {
                            let angle = (i as f32 / spike_count as f32) * std::f32::consts::TAU
                                + (layer as f32 * 0.8);
                            let radial_offset = if layer == 0 {
                                0.0
                            } else {
                                (i as f32 * 0.4).sin() * 0.15
                            };

                            let x = angle.cos() * layer_radius + radial_offset;
                            let z = angle.sin() * layer_radius;

                            // Lower spikes and vary height for natural look
                            let y_offset = mesh_radius * (0.75 + layer as f32 * 0.35);
                            let height_scale = 1.6 + (i as f32 * 0.7).sin() * 0.6;

                            let spike = commands
                                .spawn((
                                    Mesh3d(meshes.add(
                                        Sphere::new(mesh_radius * 0.19).mesh().ico(3).unwrap(),
                                    )),
                                    MeshMaterial3d(hair_mat.clone()),
                                    Transform::from_translation(Vec3::new(x, y_offset, z))
                                        .with_scale(Vec3::new(0.75, height_scale, 0.75)),
                                    PlayModeEntity,
                                ))
                                .id();

                            commands.entity(node_id).add_child(spike);
                        }
                    }
                }
                HairStyle::Curly => {
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
                            PlayModeEntity,
                        ))
                        .id();
                    commands.entity(node_id).add_child(cap);

                    for curl_idx in 0..12 {
                        let cx = (curl_idx as f32).sin() * mesh_radius * 0.9;
                        let cy = (curl_idx as f32).cos() * mesh_radius * 0.5 + mesh_radius * 0.4;
                        let cz = -mesh_radius * 0.5;
                        let curl =
                            commands
                                .spawn((
                                    Mesh3d(meshes.add(
                                        Sphere::new(mesh_radius * 0.28).mesh().ico(3).unwrap(),
                                    )),
                                    MeshMaterial3d(hair_mat.clone()),
                                    Transform::from_translation(Vec3::new(cx, cy, cz)),
                                    PlayModeEntity,
                                ))
                                .id();
                        commands.entity(node_id).add_child(curl);
                    }
                }
            }
        }
    }

    // Spawn player container
    let _player_entity = commands
        .spawn((
            PlayModePlayer {
                state: PlayerState::Active,
                position: spawn_pos - Vec3::Y * (h_scale * 0.5 + 0.08),
                rotation_yaw: 0.0,
                walk_timer: 0.0,
                is_walking: false,
                nodes: nodes.clone(),
                constraints,
                height: h_scale,
                weight: w_thick,
                head_scale,
                axe_swing_timer: None,
                axe_has_struck: false,
                stand_up_timer: 0.0,
                velocity_y: 0.0,
                health: 100.0,
                max_health: 100.0,
                active_weapon: ActiveWeapon::Melee,
                ammo_pistol: 40,
                ammo_revolver: 24,
                ammo_rifle: 120,
                ammo_sniper: 15,
                clip_pistol: 8,
                clip_revolver: 6,
                clip_rifle: 30,
                clip_sniper: 5,
                reload_timer: None,
                automatic_fire_timer: 0.0,
                swim_sound_entity: None,
                wade_sound_timer: 0.0,
            },
            Transform::from_translation(spawn_pos),
            Visibility::Visible,
            InheritedVisibility::default(),
            PlayModeEntity,
            avian3d::prelude::RigidBody::Dynamic,
            avian3d::prelude::Collider::capsule(0.3 * w_thick, 0.9),
            avian3d::prelude::LockedAxes::ROTATION_LOCKED,
            bevy_tnua::prelude::TnuaController::<ControlScheme>::default(),
            bevy_tnua::prelude::TnuaConfig::<ControlScheme>(config_handle),
            avian3d::prelude::Friction::new(0.0),
        ))
        .id();

    // Now parent the visual nodes
    // Decoupled: We do not parent them so they render directly in absolute global coordinates.

    // Spawn connecting limb meshes (Muscles if show_xray is true, Clothing cylinders otherwise)
    let connections_list = vec![
        ("Pelvis", "Spine", 0.14 * w_thick),
        ("Spine", "Chest", 0.14 * w_thick),
        ("Chest", "L_Shoulder", 0.08 * w_thick),
        ("L_Shoulder", "L_Elbow", 0.07 * w_thick),
        ("L_Elbow", "L_Hand", 0.06 * w_thick),
        ("Chest", "R_Shoulder", 0.08 * w_thick),
        ("R_Shoulder", "R_Elbow", 0.07 * w_thick),
        ("R_Elbow", "R_Hand", 0.06 * w_thick),
        ("Pelvis", "L_Hip", 0.09 * w_thick),
        ("L_Hip", "L_Knee", 0.08 * w_thick),
        ("L_Knee", "L_Foot", 0.07 * w_thick),
        ("Pelvis", "R_Hip", 0.09 * w_thick),
        ("R_Hip", "R_Knee", 0.08 * w_thick),
        ("R_Knee", "R_Foot", 0.07 * w_thick),
    ];

    let cylinder_mesh = meshes.add(Cylinder::new(1.0, 1.0));

    for (a_name, b_name, radius) in connections_list {
        let is_torso = a_name == "Pelvis" || a_name == "Spine" || a_name == "Chest";
        let is_pants = a_name == "Pelvis" || a_name == "L_Hip" || a_name == "R_Hip";

        let limb_mat = if char_settings.show_xray {
            muscle_mat.clone()
        } else if is_pants {
            pants_mat.clone()
        } else if is_torso {
            shirt_mat.clone()
        } else {
            skin_mat.clone()
        };

        if char_settings.show_xray {
            let muscle_mesh = build_skeletal_limb_mesh();
            let _limb_entity = commands
                .spawn((
                    Mesh3d(meshes.add(muscle_mesh)),
                    MeshMaterial3d(muscle_mat.clone()),
                    Transform::default(),
                    PlayLimbVisual {
                        node_a: a_name.to_string(),
                        node_b: b_name.to_string(),
                        radius,
                    },
                    PlayModeEntity,
                ))
                .id();
        } else {
            let _limb_entity = commands
                .spawn((
                    Mesh3d(cylinder_mesh.clone()),
                    MeshMaterial3d(limb_mat),
                    Transform::default(),
                    PlayLimbVisual {
                        node_a: a_name.to_string(),
                        node_b: b_name.to_string(),
                        radius,
                    },
                    PlayModeEntity,
                ))
                .id();
        }
    }

    // 8. Weapons will be spawned dynamically by play_weapon_sync_system on the first frame!

    // 9. Spawn Follow Camera
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            far: 500.0,
            ..default()
        }),
        Transform::from_xyz(spawn_pos.x, spawn_pos.y + 4.0, spawn_pos.z - 6.0)
            .looking_at(spawn_pos, Vec3::Y),
        PlayModeCamera {
            target_distance: 3.2,
            yaw: 0.0,
            pitch: -0.3,
            view_mode: ViewMode::ThirdPerson,
        },
        DistanceFog {
            color: Color::srgb(0.18, 0.22, 0.45),
            falloff: FogFalloff::Linear {
                start: 300.0,
                end: 500.0,
            },
            ..default()
        },
        PlayModeEntity,
    ));
}

fn poll_terrain_load_system(
    mut commands: Commands,
    channel: Option<Res<TerrainLoadChannel>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut grass_materials: ResMut<Assets<crate::grass::GrassMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let Some(chan) = channel else { return };
    if let Ok((terrain_mesh, grass_chunks)) = chan.rx.lock().unwrap().try_recv() {
        // Spawn terrain visual mesh
        commands.spawn((
            Mesh3d(meshes.add(terrain_mesh)),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(asset_server.load("textures/ground_grass.png")),
                perceptual_roughness: 0.9,
                ..default()
            })),
            Transform::from_xyz(0.0, 0.0, 0.0),
            PlayModeEntity,
        ));

        // Spawn grass chunks
        let grass_material = grass_materials.add(bevy::pbr::ExtendedMaterial {
            base: StandardMaterial {
                base_color_texture: Some(asset_server.load("textures/grass.png")),
                alpha_mode: AlphaMode::Mask(0.5),
                cull_mode: None,
                perceptual_roughness: 0.9,
                reflectance: 0.1,
                ..default()
            },
            extension: crate::grass::GrassWindExtension {},
        });

        let grass_single_material = grass_materials.add(bevy::pbr::ExtendedMaterial {
            base: StandardMaterial {
                base_color_texture: Some(asset_server.load("textures/grass_single.png")),
                alpha_mode: AlphaMode::Mask(0.5),
                cull_mode: None,
                perceptual_roughness: 0.9,
                reflectance: 0.1,
                ..default()
            },
            extension: crate::grass::GrassWindExtension {},
        });

        for chunk in grass_chunks {
            if let Some(mesh) = chunk.patch_mesh {
                commands.spawn((
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(grass_material.clone()),
                    Transform::default(),
                    crate::grass::ProceduralGrass,
                    PlayModeEntity,
                ));
            }
            if let Some(mesh) = chunk.single_mesh {
                commands.spawn((
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(grass_single_material.clone()),
                    Transform::default(),
                    crate::grass::ProceduralGrass,
                    PlayModeEntity,
                ));
            }
        }

        // Remove channel to stop polling
        commands.remove_resource::<TerrainLoadChannel>();
    }
}

// Cleanup system to sweep all Play Mode assets
fn cleanup_play_mode(
    mut commands: Commands,
    query: Query<Entity, (With<PlayModeEntity>, Without<ChildOf>)>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<PlayWeaponAssets>();
}

fn add_physics_to_wall_colliders(
    mut commands: Commands,
    query: Query<(Entity, &WallCollider), Added<WallCollider>>,
) {
    for (entity, wall_collider) in query.iter() {
        let size = wall_collider.half_extents * 2.0;
        commands.entity(entity).insert((
            avian3d::prelude::RigidBody::Static,
            avian3d::prelude::Collider::cuboid(size.x, size.y, size.z),
        ));
    }
}

// Particle wake helper for swimming
fn spawn_swim_wake(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    origin: Vec3,
) {
    let wake_mesh = meshes.add(Sphere::new(0.08).mesh().ico(3).unwrap());
    let wake_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.5, 0.85, 1.0, 0.4),
        emissive: LinearRgba::from(Color::srgba(0.15, 0.35, 0.55, 0.3)),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    for _ in 0..2 {
        let velocity = Vec3::new(
            (rand::random::<f32>() - 0.5) * 1.2,
            0.3 + rand::random::<f32>() * 0.8,
            (rand::random::<f32>() - 0.5) * 1.2,
        );

        commands.spawn((
            Mesh3d(wake_mesh.clone()),
            MeshMaterial3d(wake_mat.clone()),
            Transform::from_translation(origin),
            PlayParticle {
                velocity,
                lifetime: 0.0,
                max_lifetime: 0.3 + rand::random::<f32>() * 0.3,
                color: Color::srgba(0.5, 0.85, 1.0, 0.4),
            },
            PlayModeEntity,
        ));
    }
}

/// Returns (floor_y, ceiling_y) for the given position.
/// Used for grounding the player/creatures AND preventing them from passing through ceilings.
fn get_floor_and_ceiling(pos: Vec3, terrain_y: f32) -> (f32, f32) {
    // Mansion grid bounds: X inside -20.0..20.0, Z inside -10.0..10.0
    let inside_mansion = pos.x.abs() < 20.0 && pos.z.abs() < 10.0;
    if inside_mansion {
        if pos.y > 5.0 {
            // On the second floor (floor 2)
            (5.0, 8.5) // floor at 5.0, ceiling at 8.5 (5.0 + 3.5)
        } else {
            // On the ground floor (floor 1)
            (1.5, 5.0) // floor at 1.5, ceiling at 5.0
        }
    } else if pos.y < -75.0 {
        (-100.0, -50.0) // Sub-basement
    } else if pos.y < -30.0 {
        (-50.0, f32::MAX) // Basement (no ceiling constraint going up — teleporter handles it)
    } else {
        (terrain_y, f32::MAX) // Outdoors — no ceiling
    }
}

/// Convenience wrapper that returns only the floor height (backwards-compatible).
fn get_effective_floor_height(pos: Vec3, terrain_y: f32) -> f32 {
    get_floor_and_ceiling(pos, terrain_y).0
}

#[derive(bevy::ecs::system::SystemParam)]
struct PlayerMovementParams<'w, 's> {
    commands: Commands<'w, 's>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    asset_server: Res<'w, AssetServer>,
}

// System governing movements, slope locking, mouse-look, swimming, and physics-based Verlet cliff tumbling
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn player_movement_and_ragdoll_system(
    mut params: PlayerMovementParams,
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut mouse_motion: MessageReader<bevy::input::mouse::MouseMotion>,
    mut contexts: EguiContexts,
    mut player_query: Query<(Entity, &mut PlayModePlayer, &mut Transform)>,
    mut camera_query: Query<(Entity, &mut Transform, &mut PlayModeCamera), Without<PlayModePlayer>>,
    map: Res<TempestMap>,
    water_settings: Res<WaterSettings>,
    mut impulse_writer: MessageWriter<WaterImpulseEvent>,
    collider_query: Query<
        (Entity, &WallCollider, &Transform),
        (Without<PlayModePlayer>, Without<PlayModeCamera>),
    >,
    door_query: Query<&crate::play_mode::house::HouseDoor>,
    mut tnua_query: Query<&mut bevy_tnua::prelude::TnuaController<ControlScheme>>,
    mut velocity_query: Query<&mut avian3d::prelude::LinearVelocity>,
    mut physics_pos_query: Query<&mut avian3d::prelude::Position>,
) {
    let Ok((_player_entity, mut player, mut player_transform)) = player_query.single_mut() else {
        return;
    };
    if let Ok(mut tnua) = tnua_query.get_mut(_player_entity) {
        tnua.initiate_action_feeding();
    }
    // Cancel Tnua controller & velocity if not in Active state to prevent conflict with manual modes
    if player.state != PlayerState::Active {
        if let Ok(mut tnua) = tnua_query.get_mut(_player_entity) {
            tnua.basis = bevy_tnua::builtins::TnuaBuiltinWalk {
                desired_motion: Vec3::ZERO,
                desired_forward: None,
            };
        }
        if let Ok(mut vel) = velocity_query.get_mut(_player_entity) {
            vel.0 = Vec3::ZERO;
        }
    }
    let Ok((camera_entity, cam_transform, mut camera)) = camera_query.single_mut() else {
        return;
    };

    let dt = time.delta_secs().min(0.03); // clamp to prevent explosive integration steps

    if player.wade_sound_timer > 0.0 {
        player.wade_sound_timer -= dt;
    }

    // 1. Mouse-Look Camera Rotation (only when not hovering over egui context)
    let mut mouse_delta = Vec2::ZERO;
    let egui_hovered = if let Ok(ctx) = contexts.ctx_mut() {
        ctx.is_pointer_over_egui()
    } else {
        false
    };

    if !egui_hovered {
        for event in mouse_motion.read() {
            mouse_delta += event.delta;
        }
    }

    if camera.view_mode != ViewMode::Orbit && mouse_delta.length_squared() > 0.0001 {
        let sensitivity = 0.0025;
        camera.yaw += mouse_delta.x * sensitivity;
        camera.pitch = (camera.pitch - mouse_delta.y * sensitivity).clamp(-1.0, 0.7);
    }

    // Toggle camera view mode with key V
    if keyboard_input.just_pressed(KeyCode::KeyV) {
        camera.view_mode = match camera.view_mode {
            ViewMode::ThirdPerson => {
                inventory_log("📷 First-Person View active! Press [V] for Orbit Mode.");
                ViewMode::FirstPerson
            }
            ViewMode::FirstPerson => {
                inventory_log(
                    "📷 Orbit/Spectator View active! Use mouse drag to pan/orbit. Press [V] for Third-Person.",
                );
                params
                    .commands
                    .entity(camera_entity)
                    .insert(bevy_panorbit_camera::PanOrbitCamera::default());
                ViewMode::Orbit
            }
            ViewMode::Orbit => {
                inventory_log("📷 Third-Person View active! Press [V] for First-Person.");
                params
                    .commands
                    .entity(camera_entity)
                    .remove::<bevy_panorbit_camera::PanOrbitCamera>();
                ViewMode::ThirdPerson
            }
        };
    }

    // Switch weapons with Key1..=Key5
    let mut switched = false;
    if keyboard_input.just_pressed(KeyCode::Digit1) {
        player.active_weapon = ActiveWeapon::Melee;
        switched = true;
    } else if keyboard_input.just_pressed(KeyCode::Digit2) {
        player.active_weapon = ActiveWeapon::Pistol;
        switched = true;
    } else if keyboard_input.just_pressed(KeyCode::Digit3) {
        player.active_weapon = ActiveWeapon::Revolver;
        switched = true;
    } else if keyboard_input.just_pressed(KeyCode::Digit4) {
        player.active_weapon = ActiveWeapon::Rifle;
        switched = true;
    } else if keyboard_input.just_pressed(KeyCode::Digit5) {
        player.active_weapon = ActiveWeapon::Sniper;
        switched = true;
    }

    if switched {
        player.reload_timer = None; // Cancel active reload
        let reload_sound = match player.active_weapon {
            ActiveWeapon::Sniper => "sniper_reload.wav",
            _ => "gun_reload.wav",
        };
        params.commands.spawn((
            AudioPlayer::new(params.asset_server.load(reload_sound)),
            PlaybackSettings::DESPAWN,
        ));
        inventory_log(&format!("🔫 Switched to slot: {:?}", player.active_weapon));
    }

    if keyboard_input.just_pressed(KeyCode::KeyF) {
        if player.state == PlayerState::Flying {
            player.state = PlayerState::Active;
            inventory_log("🦅 Flying mode deactivated");
        } else {
            player.state = PlayerState::Flying;
            player.velocity_y = 0.0;
            inventory_log("🦅 Flying mode activated!");
        }
    }

    let p_state = player.state;

    // 2. Active Mode / Swimming Mode / Flying Mode Controls
    if p_state == PlayerState::Active
        || p_state == PlayerState::Swimming
        || p_state == PlayerState::Flying
    {
        if p_state == PlayerState::Active {
            let mut move_dir = Vec3::ZERO;

            let cam_forward = Vec3::new(cam_transform.forward().x, 0.0, cam_transform.forward().z)
                .normalize_or_zero();
            let cam_right = Vec3::new(cam_transform.right().x, 0.0, cam_transform.right().z)
                .normalize_or_zero();

            if keyboard_input.pressed(KeyCode::KeyW) {
                move_dir += cam_forward;
            }
            if keyboard_input.pressed(KeyCode::KeyS) {
                move_dir -= cam_forward;
            }
            if keyboard_input.pressed(KeyCode::KeyA) {
                move_dir -= cam_right; // Strafe Left
            }
            if keyboard_input.pressed(KeyCode::KeyD) {
                move_dir += cam_right; // Strafe Right
            }

            player.is_walking = move_dir.length_squared() > 0.001;

            let is_first_person = camera.view_mode == ViewMode::FirstPerson;
            if is_first_person {
                player.rotation_yaw = camera.yaw + std::f32::consts::PI;
            } else {
                if player.is_walking {
                    player.rotation_yaw = move_dir.z.atan2(move_dir.x);
                }
            }

            let terrain_y = get_bilinear_height(
                player_transform.translation.x,
                player_transform.translation.z,
                &map,
            );
            let (ground_y, _) = get_floor_and_ceiling(player_transform.translation, terrain_y);
            let water_depth = (water_settings.height - ground_y).max(0.0);

            let mut speed = 4.0 * player.height;
            if keyboard_input.pressed(KeyCode::ShiftLeft)
                || keyboard_input.pressed(KeyCode::ShiftRight)
            {
                speed *= 2.0; // Running speed
            }

            // Wade speed reduction in shallow water
            if water_depth > 0.0 {
                let wade_factor = (1.0 - (water_depth / 1.3) * 0.45).max(0.55);
                speed *= wade_factor;
            }

            let horizontal_vel = if let Ok(vel) = velocity_query.get(_player_entity) {
                Vec3::new(vel.x, 0.0, vel.z)
            } else {
                Vec3::ZERO
            };
            let current_speed = horizontal_vel.length();

            if player.is_walking {
                player.walk_timer += dt * current_speed * 2.5;
            } else {
                player.walk_timer = 0.0;
            }

            // Feed walk, jump, and crouch inputs to Tnua controller
            if let Ok(mut tnua) = tnua_query.get_mut(_player_entity) {
                let walk_vel = move_dir.normalize_or_zero() * speed;
                let desired_facing = if camera.view_mode == ViewMode::FirstPerson {
                    let fwd = cam_transform.forward();
                    Dir3::new(Vec3::new(fwd.x, 0.0, fwd.z)).ok()
                } else {
                    Dir3::new(walk_vel).ok()
                };
                tnua.basis = bevy_tnua::builtins::TnuaBuiltinWalk {
                    desired_motion: walk_vel,
                    desired_forward: desired_facing,
                };

                if keyboard_input.pressed(KeyCode::Space) {
                    tnua.action(crate::ControlScheme::Jump(
                        bevy_tnua::builtins::TnuaBuiltinJump::default(),
                    ));
                }
                if keyboard_input.pressed(KeyCode::KeyC)
                    || keyboard_input.pressed(KeyCode::ControlLeft)
                {
                    tnua.action(crate::ControlScheme::Crouch(
                        bevy_tnua::builtins::TnuaBuiltinCrouch,
                    ));
                }
            }

            // Fluid transition: if deep water, change to Swimming state
            let is_deep_enough_to_swim = water_depth >= 1.3;
            if player_transform.translation.y <= water_settings.height
                && is_deep_enough_to_swim
                && player_transform.translation.y > -20.0
            {
                player.state = PlayerState::Swimming;
                inventory_log("🏊 Entered deep water! Transitioning to swimming float.");
                if let Ok(mut tnua) = tnua_query.get_mut(_player_entity) {
                    tnua.basis = bevy_tnua::builtins::TnuaBuiltinWalk {
                        desired_motion: Vec3::ZERO,
                        desired_forward: None,
                    };
                }
                if let Ok(mut vel) = velocity_query.get_mut(_player_entity) {
                    vel.0 = Vec3::ZERO;
                }
            }

            // Clamp player container translation to map boundaries to prevent walking off the edge of the map
            let hw = map.width as f32 / 2.0;
            let hh = map.height as f32 / 2.0;
            let clamped_x = player_transform.translation.x.clamp(-hw + 1.0, hw - 1.0);
            let clamped_z = player_transform.translation.z.clamp(-hh + 1.0, hh - 1.0);
            if clamped_x != player_transform.translation.x
                || clamped_z != player_transform.translation.z
            {
                player_transform.translation.x = clamped_x;
                player_transform.translation.z = clamped_z;
                if let Ok(mut phys_pos) = physics_pos_query.get_mut(_player_entity) {
                    phys_pos.0.x = clamped_x;
                    phys_pos.0.z = clamped_z;
                }
                if let Ok(mut vel) = velocity_query.get_mut(_player_entity) {
                    vel.x = 0.0;
                    vel.z = 0.0;
                }
            }

            // Sync player's logical position with solved physics translation (offsetting by float_height so player.position.y is at the ground/feet, smoothed to eliminate physics spring jitter)
            let float_height = player.height * 0.5 + 0.08;

            // Ground clamp player physics to prevent clipping/sinking into hills
            let terrain_y = get_bilinear_height(player_transform.translation.x, player_transform.translation.z, &map);
            let min_phys_y = terrain_y + float_height;
            if player_transform.translation.y < min_phys_y {
                player_transform.translation.y = min_phys_y;
                if let Ok(mut phys_pos) = physics_pos_query.get_mut(_player_entity) {
                    phys_pos.0.y = min_phys_y;
                }
                if let Ok(mut vel) = velocity_query.get_mut(_player_entity) {
                    if vel.y < 0.0 {
                        vel.y = 0.0;
                    }
                }
            }

            let target_y = player_transform.translation.y - float_height;
            player.position.x = player_transform.translation.x;
            player.position.z = player_transform.translation.z;
            // Instant vertical response when grounded to eliminate sinking/clipping through hills
            player.position.y = target_y;
        } else {
            let mut move_dir = Vec3::ZERO;

            let cam_forward = Vec3::new(cam_transform.forward().x, 0.0, cam_transform.forward().z)
                .normalize_or_zero();
            let cam_right = Vec3::new(cam_transform.right().x, 0.0, cam_transform.right().z)
                .normalize_or_zero();

            if keyboard_input.pressed(KeyCode::KeyW) {
                move_dir += cam_forward;
            }
            if keyboard_input.pressed(KeyCode::KeyS) {
                move_dir -= cam_forward;
            }
            if keyboard_input.pressed(KeyCode::KeyA) {
                move_dir -= cam_right; // Strafe Left
            }
            if keyboard_input.pressed(KeyCode::KeyD) {
                move_dir += cam_right; // Strafe Right
            }

            player.is_walking = move_dir.length_squared() > 0.001;

            let is_first_person = camera.view_mode == ViewMode::FirstPerson;
            if is_first_person {
                player.rotation_yaw = camera.yaw + std::f32::consts::PI;
            } else {
                if player.is_walking {
                    player.rotation_yaw = move_dir.z.atan2(move_dir.x);
                }
            }

            let terrain_y = get_bilinear_height(player.position.x, player.position.z, &map);
            let (ground_y, _ceiling_y) = get_floor_and_ceiling(player.position, terrain_y);
            let water_depth = (water_settings.height - ground_y).max(0.0);

            let mut speed = if p_state == PlayerState::Swimming {
                2.5 * player.height
            } else if p_state == PlayerState::Flying {
                12.0 * player.height
            } else {
                4.0 * player.height
            };
            if keyboard_input.pressed(KeyCode::ShiftLeft)
                || keyboard_input.pressed(KeyCode::ShiftRight)
            {
                if p_state == PlayerState::Flying {
                    speed *= 3.0;
                } else {
                    speed *= 2.0;
                }
            }

            // Wade speed reduction in shallow water
            if p_state == PlayerState::Active && water_depth > 0.0 {
                let wade_factor = (1.0 - (water_depth / 1.3) * 0.45).max(0.55);
                speed *= wade_factor;
            }

            let mut target_pos = player.position;
            if player.is_walking {
                move_dir = move_dir.normalize();
                target_pos += move_dir * speed * dt;
                player.walk_timer += dt * 10.0;
            } else {
                if p_state == PlayerState::Swimming {
                    player.walk_timer += dt * 2.0; // Slow gentle floating motion
                } else if p_state == PlayerState::Flying {
                    player.walk_timer += dt * 1.5; // Hover motion
                } else {
                    player.walk_timer = 0.0;
                }
            }

            // Apply 3D AABB-vs-Sphere wall collisions and sliding response for manual modes
            let player_radius = 0.32 * player.weight;
            for (entity, collider, col_transform) in collider_query.iter() {
                if let Ok(door) = door_query.get(entity)
                    && door.is_open
                {
                    continue;
                }

                let center = col_transform.translation;
                let extents = collider.half_extents;

                let closest_point = Vec3::new(
                    target_pos
                        .x
                        .clamp(center.x - extents.x, center.x + extents.x),
                    target_pos
                        .y
                        .clamp(center.y - extents.y, center.y + extents.y),
                    target_pos
                        .z
                        .clamp(center.z - extents.z, center.z + extents.z),
                );

                let dist = target_pos.distance(closest_point);
                if dist < player_radius {
                    let penetration = player_radius - dist;
                    let push_dir = (target_pos - closest_point).normalize_or_zero();
                    target_pos += push_dir * penetration;
                }
            }
            player.position = target_pos;

            let hw = map.width as f32 / 2.0;
            let hh = map.height as f32 / 2.0;
            player.position.x = player.position.x.clamp(-hw + 1.0, hw - 1.0);
            player.position.z = player.position.z.clamp(-hh + 1.0, hh - 1.0);

            let water_level = water_settings.height;
            let _is_deep_enough_to_swim = water_depth >= 1.3;

            if p_state == PlayerState::Swimming {
                let mut swim_y_dir = 0.0;
                if keyboard_input.pressed(KeyCode::Space) {
                    swim_y_dir += 1.0;
                }
                if keyboard_input.pressed(KeyCode::KeyC)
                    || keyboard_input.pressed(KeyCode::ShiftLeft)
                {
                    swim_y_dir -= 1.0;
                }

                if swim_y_dir > 0.0 {
                    player.position.y = (player.position.y + 2.5 * dt).min(water_level);
                } else if swim_y_dir < 0.0 {
                    player.position.y = (player.position.y - 2.5 * dt).max(ground_y + 0.3);
                } else {
                    if player.position.y < water_level {
                        player.position.y = (player.position.y + 0.8 * dt).min(water_level);
                    } else {
                        if !player.is_walking {
                            let bobbing = (time.elapsed_secs() * 2.0).sin() * 0.06;
                            player.position.y = (water_level - 0.1) + bobbing;
                        } else {
                            player.position.y = water_level;
                        }
                    }
                }
                player.velocity_y = 0.0;

                if water_depth < 1.2 || player.position.y < -20.0 {
                    player.state = PlayerState::Active;
                    inventory_log("🚶 Walked out of water onto land.");
                } else if player.position.y >= water_level - 0.05
                    && keyboard_input.just_pressed(KeyCode::Space)
                {
                    player.velocity_y = 4.5;
                    player.position.y = water_level + 0.05;
                    player.state = PlayerState::Active;

                    impulse_writer.write(WaterImpulseEvent {
                        position: player.position,
                        force: 0.8,
                        radius: 3.5,
                    });
                    spawn_water_splash(
                        &mut params.commands,
                        &mut params.meshes,
                        &mut params.materials,
                        Vec3::new(player.position.x, water_level, player.position.z),
                    );
                    inventory_log("🏊 Splashed out of water!");
                }
            } else if p_state == PlayerState::Flying {
                let mut fly_y_dir = 0.0;
                if keyboard_input.pressed(KeyCode::Space) {
                    fly_y_dir += 1.0;
                }
                if keyboard_input.pressed(KeyCode::KeyC)
                    || keyboard_input.pressed(KeyCode::ControlLeft)
                {
                    fly_y_dir -= 1.0;
                }

                let mut fly_speed_mult = 1.0;
                if keyboard_input.pressed(KeyCode::ShiftLeft)
                    || keyboard_input.pressed(KeyCode::ShiftRight)
                {
                    fly_speed_mult = 3.0;
                }

                if fly_y_dir > 0.0 {
                    player.position.y += 15.0 * fly_speed_mult * dt;
                } else if fly_y_dir < 0.0 {
                    player.position.y =
                        (player.position.y - 15.0 * fly_speed_mult * dt).max(ground_y);
                    if player.position.y <= ground_y + 0.05 {
                        player.state = PlayerState::Active;
                        inventory_log("🦅 Landed on ground. Flying mode deactivated.");
                    }
                }
                player.velocity_y = 0.0;
            }

            // Universal ceiling clamp — applies to all movement modes.
            let final_terrain = get_bilinear_height(player.position.x, player.position.z, &map);
            let (_, final_ceiling) = get_floor_and_ceiling(player.position, final_terrain);
            if final_ceiling < f32::MAX {
                let head_clearance = player.height;
                if player.position.y + head_clearance > final_ceiling {
                    player.position.y = final_ceiling - head_clearance;
                    player.velocity_y = player.velocity_y.min(0.0);
                }
            }

            // Apply manual positions to transform AND physics engine translation
            player_transform.translation = player.position;
            if let Ok(mut phys_pos) = physics_pos_query.get_mut(_player_entity) {
                phys_pos.0 = player.position;
            }
        }

        let terrain_y = get_bilinear_height(player.position.x, player.position.z, &map);
        let (ground_y, _ceiling_y) = get_floor_and_ceiling(player.position, terrain_y);
        let water_level = water_settings.height;
        let water_depth = (water_level - ground_y).max(0.0);

        let p_height = player.height;
        let p_weight = player.weight;
        let p_axe_swing_timer = player.axe_swing_timer;
        let p_rotation_yaw = player.rotation_yaw;
        let p_pos = player.position;
        let p_walk_timer = player.walk_timer;
        let p_is_walking = player.is_walking;

        let cos_yaw = p_rotation_yaw.cos();
        let sin_yaw = p_rotation_yaw.sin();
        let forward = Vec3::new(cos_yaw, 0.0, sin_yaw);
        let right = Vec3::new(-sin_yaw, 0.0, cos_yaw);

        let mut should_play_wade_sound = false;
        let nodes = &mut player.nodes;

        if p_state == PlayerState::Swimming {
            if p_is_walking {
                // SWIMMING FLATTENED JOINT ALIGNMENT
                nodes[0].position = p_pos; // Pelvis
                nodes[1].position = p_pos + forward * p_height * 0.18; // Spine
                nodes[2].position = p_pos + forward * p_height * 0.36; // Chest
                nodes[3].position = p_pos + forward * p_height * 0.55; // Head

                let swim_cycle = p_walk_timer * 0.4;
                let sweep = (swim_cycle.sin() + 1.0) * 0.5;

                let l_shoulder = nodes[2].position - right * 0.25 * p_weight;
                let r_shoulder = nodes[2].position + right * 0.25 * p_weight;
                nodes[4].position = l_shoulder;
                nodes[6].position = r_shoulder;

                // Breaststroke arm sweeps
                let l_elbow_offset =
                    forward * (swim_cycle.cos() * 0.35 + 0.1) - right * (sweep * 0.35 + 0.1);
                let r_elbow_offset =
                    forward * (swim_cycle.cos() * 0.35 + 0.1) + right * (sweep * 0.35 + 0.1);
                nodes[5].position = l_shoulder + l_elbow_offset;
                nodes[7].position = r_shoulder + r_elbow_offset;

                // Hands extending forward
                nodes[14].position = nodes[5].position + forward * 0.25 - right * 0.05 * p_weight; // L_Hand
                nodes[15].position = nodes[7].position + forward * 0.25 + right * 0.05 * p_weight; // R_Hand

                // Hips
                let l_hip = nodes[0].position - right * 0.16 * p_weight;
                let r_hip = nodes[0].position + right * 0.16 * p_weight;
                nodes[8].position = l_hip;
                nodes[11].position = r_hip;

                // Flutter kicked legs
                let kick_l = swim_cycle.sin() * 0.15;
                let kick_r = (swim_cycle + std::f32::consts::PI).sin() * 0.15;

                nodes[9].position = l_hip - forward * p_height * 0.22 + Vec3::Y * kick_l; // L_Knee
                nodes[10].position =
                    nodes[9].position - forward * p_height * 0.22 + Vec3::Y * kick_l * 1.5; // L_Foot

                nodes[12].position = r_hip - forward * p_height * 0.22 + Vec3::Y * kick_r; // R_Knee
                nodes[13].position =
                    nodes[12].position - forward * p_height * 0.22 + Vec3::Y * kick_r * 1.5; // R_Foot
            } else {
                // UPRIGHT FLOATING BOBBING JOINT ALIGNMENT
                nodes[0].position = p_pos - Vec3::Y * p_height * 0.1; // Pelvis
                nodes[1].position = p_pos + Vec3::Y * p_height * 0.1; // Spine
                nodes[2].position = p_pos + Vec3::Y * p_height * 0.3; // Chest
                nodes[3].position = p_pos + Vec3::Y * p_height * 0.52; // Head

                let float_cycle = time.elapsed_secs() * 2.0;
                let arm_float_l = float_cycle.cos() * 0.05;
                let arm_float_r = -float_cycle.cos() * 0.05;

                nodes[4].position = nodes[2].position - right * 0.28 * p_weight; // L_Shoulder
                nodes[5].position = nodes[4].position - right * 0.22 * p_weight
                    + Vec3::Y * (0.05 + arm_float_l)
                    + forward * 0.1; // L_Elbow
                nodes[14].position =
                    nodes[5].position - right * 0.15 * p_weight + Vec3::Y * arm_float_l; // L_Hand

                nodes[6].position = nodes[2].position + right * 0.28 * p_weight; // R_Shoulder
                nodes[7].position = nodes[6].position
                    + right * 0.22 * p_weight
                    + Vec3::Y * (0.05 + arm_float_r)
                    + forward * 0.1; // R_Elbow
                nodes[15].position =
                    nodes[7].position + right * 0.15 * p_weight + Vec3::Y * arm_float_r; // R_Hand

                let l_hip = nodes[0].position - right * 0.16 * p_weight;
                let r_hip = nodes[0].position + right * 0.16 * p_weight;
                nodes[8].position = l_hip;
                nodes[11].position = r_hip;

                nodes[9].position = l_hip - Vec3::Y * p_height * 0.25; // L_Knee
                nodes[10].position = nodes[9].position - Vec3::Y * p_height * 0.25; // L_Foot

                nodes[12].position = r_hip - Vec3::Y * p_height * 0.25; // R_Knee
                nodes[13].position = nodes[12].position - Vec3::Y * p_height * 0.25; // R_Foot
            }

            // Wake interaction
            if p_is_walking {
                let wake_pos = p_pos - forward * 0.8;
                spawn_swim_wake(
                    &mut params.commands,
                    &mut params.meshes,
                    &mut params.materials,
                    Vec3::new(wake_pos.x, water_level, wake_pos.z),
                );

                // Dipole wave generation: push crest in front, pull trough behind!
                impulse_writer.write(WaterImpulseEvent {
                    position: p_pos + forward * 0.5,
                    force: 0.22,
                    radius: 2.0,
                });
                impulse_writer.write(WaterImpulseEvent {
                    position: p_pos - forward * 0.8,
                    force: -0.18,
                    radius: 2.0,
                });
            }
        } else {
            // STANDING / UPRIGHT WALKING ALIGNMENT
            nodes[0].position = p_pos + Vec3::Y * p_height * 0.5; // Pelvis
            nodes[1].position = p_pos + Vec3::Y * p_height * 0.65; // Spine
            nodes[2].position = p_pos + Vec3::Y * p_height * 0.8; // Chest
            nodes[3].position = p_pos + Vec3::Y * p_height * 0.98; // Head

            let is_first_person = camera.view_mode == ViewMode::FirstPerson;

            if is_first_person {
                // FIRST PERSON ARM ALIGNMENT - Raise hands holding the weapon in front of the camera
                let mut arm_swing = p_walk_timer.sin() * 0.05;
                if p_axe_swing_timer.is_some() {
                    arm_swing = 0.0;
                }

                // Left shoulder & hand held out forward-left
                nodes[4].position = p_pos + Vec3::Y * p_height * 0.85 - right * 0.25 * p_weight; // L_Shoulder
                nodes[14].position = nodes[4].position - right * 0.1 * p_weight
                    + forward * (0.4 + arm_swing)
                    - Vec3::Y * 0.15; // L_Hand
                nodes[5].position = nodes[4].position
                    + (nodes[14].position - nodes[4].position) * 0.5
                    - right * 0.08
                    - Vec3::Y * 0.05; // L_Elbow

                // Right shoulder & hand held out forward-right holding weapon
                let cam_forward = cam_transform.forward().as_vec3();
                let cam_right = cam_transform.right().as_vec3();
                let cam_up = cam_transform.up().as_vec3();

                // Bring the hand higher and more centered so the gun is clearly visible in first person!
                let mut r_hand_pos =
                    cam_transform.translation + cam_forward * 0.55 + cam_right * 0.2
                        - cam_up * 0.15;

                if let Some(t) = p_axe_swing_timer {
                    let chop_factor = (t * std::f32::consts::PI / 0.3).sin();
                    r_hand_pos +=
                        cam_forward * (chop_factor * 0.25) - cam_up * (chop_factor * 0.28);
                }

                nodes[6].position = p_pos + Vec3::Y * p_height * 0.85 + right * 0.25 * p_weight; // R_Shoulder
                nodes[15].position = r_hand_pos; // R_Hand
                nodes[7].position = nodes[6].position
                    + (nodes[15].position - nodes[6].position) * 0.5
                    + right * 0.08
                    - Vec3::Y * 0.05; // R_Elbow
            } else {
                // THIRD PERSON ARM ALIGNMENT
                let mut arm_swing = p_walk_timer.sin() * 0.25;
                if p_axe_swing_timer.is_some() {
                    arm_swing = 0.0;
                }

                nodes[4].position = p_pos + Vec3::Y * p_height * 0.8
                    - right * 0.25 * p_weight
                    - forward * arm_swing; // L_Shoulder
                nodes[14].position = nodes[4].position - right * 0.15 * p_weight
                    + forward * (0.2 + arm_swing)
                    - Vec3::Y * 0.15; // L_Hand
                nodes[5].position = nodes[4].position
                    + (nodes[14].position - nodes[4].position) * 0.5
                    - right * 0.12
                    - Vec3::Y * 0.08; // L_Elbow

                let mut r_hand_pos =
                    p_pos + Vec3::Y * p_height * 0.65 + right * 0.3 * p_weight + forward * 0.3;

                if let Some(t) = p_axe_swing_timer {
                    let chop_factor = (t * std::f32::consts::PI / 0.3).sin();
                    r_hand_pos = p_pos
                        + Vec3::Y * p_height * (0.65 - chop_factor * 0.4)
                        + right * 0.2 * p_weight
                        + forward * (0.3 + chop_factor * 0.5);
                }

                nodes[6].position = p_pos + Vec3::Y * p_height * 0.8 + right * 0.25 * p_weight; // R_Shoulder
                nodes[15].position = r_hand_pos; // R_Hand
                nodes[7].position = nodes[6].position
                    + (nodes[15].position - nodes[6].position) * 0.5
                    + right * 0.12
                    - Vec3::Y * 0.08; // R_Elbow
            }

            let l_leg_swing = p_walk_timer.sin() * 0.35 * p_height;
            let r_leg_swing = -p_walk_timer.sin() * 0.35 * p_height;

            nodes[8].position = p_pos + Vec3::Y * p_height * 0.45 - right * 0.16 * p_weight; // L_Hip
            nodes[11].position = p_pos + Vec3::Y * p_height * 0.45 + right * 0.16 * p_weight; // R_Hip

            nodes[9].position = p_pos + Vec3::Y * p_height * 0.22 - right * 0.16 * p_weight
                + forward * l_leg_swing.max(0.0); // L_Knee
            nodes[12].position = p_pos
                + Vec3::Y * p_height * 0.22
                + right * 0.16 * p_weight
                + forward * r_leg_swing.max(0.0); // R_Knee

            let l_foot_lift = if l_leg_swing > 0.0 {
                l_leg_swing * 0.4
            } else {
                0.0
            };
            let r_foot_lift = if r_leg_swing > 0.0 {
                r_leg_swing * 0.4
            } else {
                0.0
            };

            let is_grounded = p_pos.y <= ground_y + 0.05;
            let l_foot_y = if is_grounded {
                // Use the effective floor height so feet stay correct in houses/basements
                let ft_terrain =
                    get_bilinear_height(p_pos.x - right.x * 0.16, p_pos.z - right.z * 0.16, &map);
                let ft_ground = get_effective_floor_height(p_pos, ft_terrain);
                ft_ground + l_foot_lift
            } else {
                p_pos.y + l_foot_lift
            };
            let r_foot_y = if is_grounded {
                let ft_terrain =
                    get_bilinear_height(p_pos.x + right.x * 0.16, p_pos.z + right.z * 0.16, &map);
                let ft_ground = get_effective_floor_height(p_pos, ft_terrain);
                ft_ground + r_foot_lift
            } else {
                p_pos.y + r_foot_lift
            };

            nodes[10].position = Vec3::new(
                p_pos.x - right.x * 0.16 + forward.x * l_leg_swing,
                l_foot_y,
                p_pos.z - right.z * 0.16 + forward.z * l_leg_swing,
            ); // L_Foot
            nodes[13].position = Vec3::new(
                p_pos.x + right.x * 0.16 + forward.x * r_leg_swing,
                r_foot_y,
                p_pos.z + right.z * 0.16 + forward.z * r_leg_swing,
            ); // R_Foot
        }

        // Wade stepping interaction in shallow water
        if p_is_walking && water_depth > 0.0 && p_state == PlayerState::Active {
            let step_timer = time.elapsed_secs() * 10.0;
            if step_timer.sin().abs() > 0.95 {
                impulse_writer.write(WaterImpulseEvent {
                    position: p_pos,
                    force: 0.15,
                    radius: 1.5,
                });
                spawn_swim_wake(
                    &mut params.commands,
                    &mut params.meshes,
                    &mut params.materials,
                    Vec3::new(p_pos.x, water_level, p_pos.z),
                );
                should_play_wade_sound = true;
            }
        }

        for node in nodes.iter_mut() {
            node.old_position = node.position;
        }

        // Play puddle stepping sound if triggered and cooldown is up (borrow of nodes has ended here)
        if should_play_wade_sound && player.wade_sound_timer <= 0.0 {
            params.commands.spawn((
                AudioPlayer::new(params.asset_server.load("puddle_stepping.wav")),
                PlaybackSettings {
                    volume: bevy::audio::Volume::Linear(0.55),
                    ..PlaybackSettings::DESPAWN
                },
            ));
            player.wade_sound_timer = 0.35; // 350ms cooldown (about half of walk step frequency)
        }

        // Trigger manual ragdoll tumbling
        if keyboard_input.just_pressed(KeyCode::KeyG) {
            player.state = PlayerState::Ragdoll;
            inventory_log("💥 Collapsed into floppy ragdoll!");
            for n in player.nodes.iter_mut() {
                n.old_position -= forward * 0.2 + Vec3::Y * 0.1;
            }
        }

        // Transition to ragdoll automatically when walking off high precipices
        let lowest_foot = player.nodes[10].position.y.min(player.nodes[13].position.y);
        if player.position.y - lowest_foot > 1.3 * p_height && p_state != PlayerState::Swimming {
            player.state = PlayerState::Ragdoll;
            inventory_log("⚠️ Walked off cliff! Humanoid slipping into ragdoll tumble!");
        }
    } else {
        // 3. FULL VERLET PHYSICS RAGDOLL MODE
        let gravity = Vec3::new(0.0, -9.8, 0.0);
        for node in player.nodes.iter_mut() {
            let temp = node.position;
            let velocity = (node.position - node.old_position) * 0.98;
            node.position += velocity + gravity * dt * dt;
            node.old_position = temp;
        }

        // Solve distance constraints safely
        let constraints = player.constraints.clone();
        for _ in 0..4 {
            for c in constraints.iter() {
                let p_a = player.nodes[c.node_a].position;
                let p_b = player.nodes[c.node_b].position;

                let delta = p_b - p_a;
                let dist = delta.length();
                if dist > 0.001 {
                    let diff = c.target_length - dist;
                    let percent = diff / dist * 0.5;
                    let offset = delta * percent;

                    player.nodes[c.node_a].position -= offset;
                    player.nodes[c.node_b].position += offset;
                }
            }
        }

        // Terrain height bounds and friction collisions
        let mut total_velocity = 0.0;
        for node in player.nodes.iter_mut() {
            let terrain_y = get_bilinear_height(node.position.x, node.position.z, &map);
            let eff_floor = get_effective_floor_height(node.position, terrain_y);
            if node.position.y < eff_floor + node.radius {
                node.position.y = eff_floor + node.radius;

                let vx = node.position.x - node.old_position.x;
                let vz = node.position.z - node.old_position.z;
                node.old_position.x = node.position.x - vx * 0.4;
                node.old_position.z = node.position.z - vz * 0.4;
            }
            total_velocity += (node.position - node.old_position).length();
        }

        let root_pos = player.nodes[0].position;
        player.position = root_pos;
        player_transform.translation = root_pos;
        if let Ok(mut phys_pos) = physics_pos_query.get_mut(_player_entity) {
            phys_pos.0 = root_pos;
        }

        // Dynamic water entry check: if we are in deep water, transition to swimming float instead of tumbling!
        let water_level = water_settings.height;
        let ground_y = get_bilinear_height(player.position.x, player.position.z, &map);
        let water_depth = (water_level - ground_y).max(0.0);
        let is_deep_enough_to_swim = water_depth >= 1.3;

        if is_deep_enough_to_swim && player.position.y <= water_level {
            player.state = PlayerState::Swimming;
            player.stand_up_timer = 0.0;
            let fall_speed =
                ((player.nodes[0].position.y - player.nodes[0].old_position.y) / dt).abs();
            let vel_factor = (fall_speed / 3.0).max(1.0);
            impulse_writer.write(WaterImpulseEvent {
                position: player.position,
                force: -1.4 * vel_factor,
                radius: 4.0 * vel_factor,
            });
            spawn_water_splash(
                &mut params.commands,
                &mut params.meshes,
                &mut params.materials,
                Vec3::new(player.position.x, water_level, player.position.z),
            );
            inventory_log("🏊 Entered deep water! Transitioning to swimming float.");
            for n in player.nodes.iter_mut() {
                n.old_position = n.position;
            }
        }

        if total_velocity < 0.15 {
            player.stand_up_timer += dt;
        } else {
            player.stand_up_timer = 0.0;
        }

        if keyboard_input.just_pressed(KeyCode::Space) {
            player.state = PlayerState::Active;
            player.stand_up_timer = 0.0;
            inventory_log("🛡️ Stood back up! Rebalancing active skeleton.");
            for n in player.nodes.iter_mut() {
                n.old_position = n.position;
            }
        }
    }

    // Swimming Sound Management
    if player.state == PlayerState::Swimming && player.is_walking {
        if player.swim_sound_entity.is_none() {
            let swim_entity = params
                .commands
                .spawn((
                    AudioPlayer::new(params.asset_server.load("water_swim.ogg")),
                    PlaybackSettings {
                        mode: bevy::audio::PlaybackMode::Loop,
                        volume: bevy::audio::Volume::Linear(0.55),
                        ..default()
                    },
                ))
                .id();
            player.swim_sound_entity = Some(swim_entity);
        }
    } else {
        if let Some(swim_entity) = player.swim_sound_entity {
            params.commands.entity(swim_entity).despawn();
            player.swim_sound_entity = None;
        }
    }
}

// Separate follow camera positioning system to execute after physics solver in PostUpdate, preventing camera jitter
fn camera_follow_system(
    time: Res<Time>,
    player_query: Query<&PlayModePlayer>,
    mut camera_query: Query<(&mut Transform, &PlayModeCamera), Without<PlayModePlayer>>,
    mut smoothed_focus: Local<Option<Vec3>>,
) {
    let Ok(player) = player_query.single() else {
        return;
    };
    let Ok((mut cam_transform, camera)) = camera_query.single_mut() else {
        return;
    };

    let dt = time.delta_secs().min(0.03);

    // Position camera based on view mode (First-Person at player Head, Third-Person over-the-shoulder follow)
    if camera.view_mode == ViewMode::FirstPerson && player.state != PlayerState::Ragdoll {
        let head_pos = player.nodes[3].position;
        let look_dir = Vec3::new(
            -camera.yaw.cos() * camera.pitch.cos(),
            camera.pitch.sin(),
            -camera.yaw.sin() * camera.pitch.cos(),
        )
        .normalize_or_zero();

        // Put camera exactly at head, offset forward by 0.18 to prevent rendering inside the head sphere
        let target_cam_pos = head_pos + look_dir * 0.18;
        cam_transform.translation = target_cam_pos;
        cam_transform.look_at(target_cam_pos + look_dir * 5.0, Vec3::Y);
    } else if camera.view_mode == ViewMode::ThirdPerson || player.state == PlayerState::Ragdoll {
        // Third Person Camera Lerping
        let camera_offset = Vec3::new(
            camera.yaw.cos() * camera.target_distance * camera.pitch.cos(),
            camera.target_distance * -camera.pitch.sin() + 1.2,
            camera.yaw.sin() * camera.target_distance * camera.pitch.cos(),
        );

        let player_y = if player.state == PlayerState::Swimming {
            1.2
        } else {
            player.position.y
        };
        let mut stable_player_pos = player.position;
        stable_player_pos.y = player_y;

        let target_cam_pos = stable_player_pos + camera_offset;
        cam_transform.translation = cam_transform.translation.lerp(target_cam_pos, 8.0 * dt);

        // Smoothly lerp look-at focus point to completely eliminate micro-jitters
        let target_focus = stable_player_pos + Vec3::Y * player.height * 0.65;
        let current_focus = smoothed_focus.get_or_insert(target_focus);
        *current_focus = current_focus.lerp(target_focus, 12.0 * dt);
        cam_transform.look_at(*current_focus, Vec3::Y);
    }
}

// Axe chopping swing trigger and ore harvesting checks
#[allow(clippy::too_many_arguments)]
fn axe_swing_system(
    mut commands: Commands,
    time: Res<Time>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut player_query: Query<&mut PlayModePlayer>,
    mut resource_query: Query<(Entity, &mut PlayResourceNode)>,
    mut visual_query: Query<&mut Transform, Without<PlayModePlayer>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut inventory: ResMut<PlayerInventory>,
    children_query: Query<&Children>,
    builder: Res<crate::procedural_walls::ProceduralWallBuilder>,
    window_query: Query<&CursorOptions, With<Window>>,
) {
    let Ok(mut player) = player_query.single_mut() else {
        return;
    };
    let Ok(cursor_options) = window_query.single() else {
        return;
    };
    if cursor_options.grab_mode != CursorGrabMode::Locked {
        return;
    }
    let dt = time.delta_secs();

    // Trigger Left Click swing (only when melee weapon is active and not building)
    if player.state == PlayerState::Active
        && player.active_weapon == ActiveWeapon::Melee
        && player.axe_swing_timer.is_none()
        && mouse_button.just_pressed(MouseButton::Left)
        && !builder.active
    {
        player.axe_swing_timer = Some(0.0);
        player.axe_has_struck = false;
    }

    if let Some(mut t) = player.axe_swing_timer {
        t += dt;
        player.axe_swing_timer = Some(t);

        if t >= 0.15 && !player.axe_has_struck {
            player.axe_has_struck = true;

            let player_pos = player.position;
            let yaw = player.rotation_yaw;
            let forward = Vec3::new(yaw.cos(), 0.0, yaw.sin());

            let strike_center = player_pos + forward * 1.2 + Vec3::Y * player.height * 0.5;

            let mut closest_node: Option<(Entity, Mut<PlayResourceNode>)> = None;
            let mut closest_dist = 2.5;

            for (entity, node) in resource_query.iter_mut() {
                let dist = strike_center.distance(node.position);
                if dist < closest_dist {
                    closest_dist = dist;
                    closest_node = Some((entity, node));
                }
            }

            if let Some((entity, mut node)) = closest_node {
                node.health -= 1;

                let (spark_color, name) = match node.prefab_type.as_str() {
                    "tree_oak" => (Color::srgb(0.55, 0.4, 0.25), "Oak Wood"),
                    "tree_pine" => (Color::srgb(0.5, 0.35, 0.2), "Pine Wood"),
                    "tree_birch" => (Color::srgb(0.85, 0.8, 0.75), "Birch Wood"),
                    "rock" => (Color::srgb(0.5, 0.5, 0.5), "Stone"),
                    "ore_copper" => (Color::srgb(0.9, 0.4, 0.2), "Copper Ore"),
                    "ore_iron" => (Color::srgb(0.7, 0.3, 0.15), "Iron Ore"),
                    "ore_gold" => (Color::srgb(1.0, 0.85, 0.1), "Gold Ore"),
                    "ore_silver" => (Color::srgb(0.9, 0.9, 0.92), "Silver Ore"),
                    "ore_platinum" => (Color::srgb(0.85, 0.9, 1.0), "Platinum Ore"),
                    "ore_steel" => (Color::srgb(0.5, 0.52, 0.55), "Steel chunk"),
                    "ore_granite" => (Color::srgb(0.3, 0.3, 0.32), "Granite"),
                    _ => (Color::WHITE, "Material"),
                };

                if let Ok(mut transform) = visual_query.get_mut(entity) {
                    transform.scale *= 0.92;
                }

                spawn_sparks(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    node.position + Vec3::Y * 0.8,
                    spark_color,
                );

                if node.health <= 0 {
                    inventory_log(&format!("🎉 Fully harvested resource: {}!", name));
                    match node.prefab_type.as_str() {
                        "tree_oak" | "tree_pine" | "tree_birch" => {
                            inventory.wood += 3;
                            inventory_log("+3 Wood added to inventory!");
                        }
                        "rock" => {
                            inventory.rock += 3;
                            inventory_log("+3 Stone added to inventory!");
                        }
                        "ore_copper" => {
                            inventory.copper += 3;
                            inventory_log("+3 Copper added!");
                        }
                        "ore_iron" => {
                            inventory.iron += 3;
                            inventory_log("+3 Iron added!");
                        }
                        "ore_gold" => {
                            inventory.gold += 3;
                            inventory_log("+3 Gold added!");
                        }
                        "ore_silver" => {
                            inventory.silver += 3;
                            inventory_log("+3 Silver added!");
                        }
                        "ore_platinum" => {
                            inventory.platinum += 3;
                            inventory_log("+3 Platinum added!");
                        }
                        "ore_steel" => {
                            inventory.steel += 3;
                            inventory_log("+3 Steel added!");
                        }
                        "ore_granite" => {
                            inventory.granite += 3;
                            inventory_log("+3 Granite added!");
                        }
                        _ => {}
                    }

                    if let Ok(children) = children_query.get(entity) {
                        for child in children.iter() {
                            commands.entity(child).despawn();
                        }
                    }
                    commands.entity(entity).despawn();
                } else {
                    inventory_log(&format!("⛏ Struck resource node: {}!", name));
                    match node.prefab_type.as_str() {
                        "tree_oak" | "tree_pine" | "tree_birch" => {
                            inventory.wood += 1;
                            inventory_log("+1 Wood collected");
                        }
                        "rock" => {
                            inventory.rock += 1;
                            inventory_log("+1 Stone collected");
                        }
                        "ore_copper" => {
                            inventory.copper += 1;
                            inventory_log("+1 Copper collected");
                        }
                        "ore_iron" => {
                            inventory.iron += 1;
                            inventory_log("+1 Iron collected");
                        }
                        "ore_gold" => {
                            inventory.gold += 1;
                            inventory_log("+1 Gold collected");
                        }
                        "ore_silver" => {
                            inventory.silver += 1;
                            inventory_log("+1 Silver collected");
                        }
                        "ore_platinum" => {
                            inventory.platinum += 1;
                            inventory_log("+1 Platinum collected");
                        }
                        "ore_steel" => {
                            inventory.steel += 1;
                            inventory_log("+1 Steel collected");
                        }
                        "ore_granite" => {
                            inventory.granite += 1;
                            inventory_log("+1 Granite collected");
                        }
                        _ => {}
                    }
                }
            }
        }

        if t >= 0.3 {
            player.axe_swing_timer = None;
        }
    }
}

// Spark emission utility
fn spawn_sparks(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    origin: Vec3,
    color: Color,
) {
    let spark_mesh = meshes.add(Sphere::new(0.08).mesh().ico(3).unwrap());
    let spark_mat = materials.add(StandardMaterial {
        base_color: color,
        emissive: LinearRgba::from(color) * 1.5,
        ..default()
    });

    for i in 0..6 {
        let angle = (i as f32) * std::f32::consts::TAU / 6.0;
        let velocity = Vec3::new(
            angle.cos() * (1.2 + rand::random::<f32>() * 0.8),
            2.0 + rand::random::<f32>() * 2.0,
            angle.sin() * (1.2 + rand::random::<f32>() * 0.8),
        );

        commands.spawn((
            Mesh3d(spark_mesh.clone()),
            MeshMaterial3d(spark_mat.clone()),
            Transform::from_translation(origin),
            PlayParticle {
                velocity,
                lifetime: 0.0,
                max_lifetime: 0.6 + rand::random::<f32>() * 0.4,
                color,
            },
            PlayModeEntity,
        ));
    }
}

// Particle movement system
fn particle_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut particle_query: Query<(Entity, &mut Transform, &mut PlayParticle)>,
) {
    let dt = time.delta_secs();
    let gravity = Vec3::new(0.0, -8.0, 0.0);

    for (entity, mut transform, mut particle) in particle_query.iter_mut() {
        particle.lifetime += dt;
        if particle.lifetime >= particle.max_lifetime {
            commands.entity(entity).despawn();
            continue;
        }

        particle.velocity += gravity * dt;
        transform.translation += particle.velocity * dt;

        let scale = 1.0 - (particle.lifetime / particle.max_lifetime);
        transform.scale = Vec3::splat(scale);
    }
}

// Sync visual joint spheres and limb deforming cylinders in real-time
#[allow(clippy::type_complexity)]
fn play_visual_sync_system(
    player_query: Query<&PlayModePlayer>,
    camera_query: Query<(&Transform, &PlayModeCamera)>,
    mut joint_query: Query<
        (&mut Transform, &mut Visibility, &PlayJointVisual),
        (Without<PlayLimbVisual>, Without<PlayModeCamera>),
    >,
    mut limb_query: Query<
        (&mut Transform, &PlayLimbVisual),
        (Without<PlayJointVisual>, Without<PlayModeCamera>),
    >,
) {
    let Ok(player) = player_query.single() else {
        return;
    };
    let Ok((_camera_transform, camera)) = camera_query.single() else {
        return;
    };
    let is_first_person = camera.view_mode == ViewMode::FirstPerson;

    // Sync joints
    for (mut transform, mut visibility, visual) in joint_query.iter_mut() {
        if let Some(node) = player.nodes.iter().find(|n| n.name == visual.name) {
            transform.translation = node.position;

            // Hide the head in first person to prevent clipping/view blockage
            if visual.name == "Head" && is_first_person {
                *visibility = Visibility::Hidden;
            } else {
                *visibility = Visibility::Inherited;
            }

            // Orient the head to face the player's body movement direction
            if visual.name == "Head" {
                transform.rotation = Quat::from_rotation_y(-player.rotation_yaw);
            }

            // Orient the hand (and any equipped weapons) to match the player body yaw
            if visual.name == "R_Hand" && !is_first_person {
                transform.rotation = Quat::from_rotation_y(-player.rotation_yaw);
            }
        }
    }

    // Sync connecting limbs
    for (mut transform, limb) in limb_query.iter_mut() {
        let pos_a = player
            .nodes
            .iter()
            .find(|n| n.name == limb.node_a)
            .map(|n| n.position);
        let pos_b = player
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
                transform.scale = Vec3::new(limb.radius, dist, limb.radius);
            }
        }
    }
}

// Egui HUD inventory panel overlay, crafting sidebar, and alert log rendering
#[allow(clippy::too_many_arguments)]
fn play_mode_hud_ui(
    mut commands: Commands,
    mut contexts: EguiContexts,
    mut next_state: ResMut<NextState<AppState>>,
    mut inventory: ResMut<PlayerInventory>,
    player_query: Query<&PlayModePlayer>,
    map: Res<TempestMap>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let Ok(player) = player_query.single() else {
        return;
    };

    // 1. Play Mode Inventory & Controls Panel
    egui::Window::new("🎮 Play Mode HUD & Inventory")
        .default_width(280.0)
        .anchor(egui::Align2::LEFT_TOP, egui::Vec2::new(10.0, 10.0))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            // Player Health Bar
            ui.horizontal(|ui| {
                ui.label("❤️ Health:");
                let hp_text = format!("{:.0} / {:.0}", player.health, player.max_health);
                ui.add(egui::ProgressBar::new(player.health / player.max_health)
                    .text(egui::RichText::new(hp_text).strong().color(egui::Color32::WHITE))
                    .fill(egui::Color32::from_rgb(225, 45, 45)));
            });
            ui.separator();

            // Active Weapon & Ammo Clip
            ui.horizontal(|ui| {
                ui.label("🔫 Equipped:");
                let ammo_text = if let Some(reload_time) = player.reload_timer {
                    format!("Reloading ({:.1}s)", reload_time)
                } else {
                    match player.active_weapon {
                        ActiveWeapon::Melee => {
                            if inventory.has_sword { "⚔ Broadsword (Infinite)".to_string() } else { "🪓 Wood Axe (Infinite)".to_string() }
                        }
                        ActiveWeapon::Pistol => format!("Pistol [{} / {}]", player.clip_pistol, player.ammo_pistol),
                        ActiveWeapon::Revolver => format!("Revolver [{} / {}]", player.clip_revolver, player.ammo_revolver),
                        ActiveWeapon::Rifle => format!("Rifle [{} / {}]", player.clip_rifle, player.ammo_rifle),
                        ActiveWeapon::Sniper => format!("Sniper [{} / {}]", player.clip_sniper, player.ammo_sniper),
                    }
                };
                ui.label(egui::RichText::new(ammo_text).strong().color(egui::Color32::from_rgb(90, 220, 255)));
            });
            ui.separator();

            ui.heading("Crafting Elements Looted");
            ui.separator();

            egui::Grid::new("loot_grid")
                .num_columns(2)
                .spacing([40.0, 6.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label("🪵 Wood Logs:"); ui.label(egui::RichText::new(inventory.wood.to_string()).strong().color(egui::Color32::from_rgb(170, 120, 80))); ui.end_row();
                    ui.label("🪨 Stone Cobbles:"); ui.label(egui::RichText::new(inventory.rock.to_string()).strong().color(egui::Color32::LIGHT_GRAY)); ui.end_row();
                    ui.label("🔸 Copper Ore:"); ui.label(egui::RichText::new(inventory.copper.to_string()).strong().color(egui::Color32::from_rgb(220, 100, 40))); ui.end_row();
                    ui.label("🟫 Iron Ore:"); ui.label(egui::RichText::new(inventory.iron.to_string()).strong().color(egui::Color32::from_rgb(180, 80, 50))); ui.end_row();
                    ui.label("🟡 Gold Ore:"); ui.label(egui::RichText::new(inventory.gold.to_string()).strong().color(egui::Color32::from_rgb(255, 215, 0))); ui.end_row();
                    ui.label("◽ Silver Ore:"); ui.label(egui::RichText::new(inventory.silver.to_string()).strong().color(egui::Color32::from_rgb(200, 200, 220))); ui.end_row();
                    ui.label("💎 Platinum Ore:"); ui.label(egui::RichText::new(inventory.platinum.to_string()).strong().color(egui::Color32::from_rgb(160, 210, 255))); ui.end_row();
                    ui.label("🔗 Steel chunk:"); ui.label(egui::RichText::new(inventory.steel.to_string()).strong().color(egui::Color32::from_rgb(120, 130, 140))); ui.end_row();
                    ui.label("◼ Granite Block:"); ui.label(egui::RichText::new(inventory.granite.to_string()).strong().color(egui::Color32::from_rgb(80, 80, 85))); ui.end_row();
                });

            ui.add_space(10.0);
            ui.separator();

            ui.label(egui::RichText::new("Controls:").strong().underline());
            ui.label("• W, A, S, D to move / strafe\n• Space to jump (Active) or swim up (Water)\n• Shift or C to dive down (Water)\n• Mouse to look and aim\n• Left-Click to shoot / swing melee\n• Press 1..=5 to switch weapon slot\n• Press [R] to reload current gun\n• Press [G] to collapse into ragdoll\n• Press [Space] to stand back up!");

            ui.add_space(15.0);
            if ui.add(egui::Button::new("🚪 Exit to Launcher Menu").fill(egui::Color32::from_rgb(160, 40, 40))).clicked() {
                next_state.set(AppState::MainMenu);
            }
        });

    // 1.5 Draw center screen crosshair
    egui::Area::new(egui::Id::new("crosshair_marker"))
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new("+")
                    .size(24.0)
                    .strong()
                    .color(egui::Color32::from_rgb(100, 255, 100)),
            );
        });

    // 2. Crafting & Fabrication sidebar Station Panel
    egui::Window::new("🛠️ Crafting & Fabrication Station")
        .default_width(280.0)
        .anchor(egui::Align2::RIGHT_TOP, egui::Vec2::new(-10.0, 10.0))
        .collapsible(true)
        .resizable(false)
        .show(ctx, |ui| {
            ui.heading("Crafting Station Recipes");
            ui.separator();

            // Recipe 1: Wooden Shelter Parts
            ui.label(
                egui::RichText::new("🪵 Wooden Shelter Parts")
                    .strong()
                    .color(egui::Color32::from_rgb(220, 160, 100)),
            );
            ui.label("Cost: 15 Wood, 5 Stone");
            let can_craft_wood_parts = inventory.wood >= 15 && inventory.rock >= 5;
            ui.horizontal(|ui| {
                ui.label(format!("Owned: {}", inventory.wooden_shelter_parts));
                if ui
                    .add_enabled(can_craft_wood_parts, egui::Button::new("⚒ Craft Parts"))
                    .clicked()
                {
                    inventory.wood -= 15;
                    inventory.rock -= 5;
                    inventory.wooden_shelter_parts += 1;
                    inventory_log("⚒ Crafted: 1x Wooden Shelter Parts!");
                }
            });
            ui.separator();

            // Recipe 2: Wooden Shelter
            ui.label(
                egui::RichText::new("🏡 Wooden Shelter Structure")
                    .strong()
                    .color(egui::Color32::from_rgb(180, 120, 70)),
            );
            ui.label("Cost: 1 Wooden Parts, 10 Wood");
            let can_craft_wood_shelter =
                inventory.wooden_shelter_parts >= 1 && inventory.wood >= 10;
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        can_craft_wood_shelter,
                        egui::Button::new("🏗 Build Wooden Shelter"),
                    )
                    .clicked()
                {
                    inventory.wooden_shelter_parts -= 1;
                    inventory.wood -= 10;

                    // Spawning 3D wooden shelter in front of the player
                    let yaw = player.rotation_yaw;
                    let forward = Vec3::new(yaw.cos(), 0.0, yaw.sin());
                    let spawn_pos = player.position + forward * 2.5;
                    let terrain_y = get_bilinear_height(spawn_pos.x, spawn_pos.z, &map);
                    let shelter_pos = Vec3::new(spawn_pos.x, terrain_y, spawn_pos.z);

                    let rotation = Quat::from_rotation_y(-yaw);

                    let pillar_mesh = meshes.add(Cuboid::new(0.2, 3.6, 0.2));
                    let wood_mat = materials.add(StandardMaterial {
                        base_color_texture: Some(asset_server.load("textures/wood_planks.png")),
                        perceptual_roughness: 0.9,
                        ..default()
                    });

                    // 4 corners (Pillars)
                    commands.spawn((
                        Mesh3d(pillar_mesh.clone()),
                        MeshMaterial3d(wood_mat.clone()),
                        Transform::from_translation(
                            shelter_pos + rotation * Vec3::new(-2.4, 1.8, -2.4),
                        )
                        .with_rotation(rotation),
                        avian3d::prelude::RigidBody::Static,
                        avian3d::prelude::Collider::cuboid(0.2, 3.6, 0.2),
                        crate::play_mode::WallCollider {
                            half_extents: Vec3::new(0.1, 1.8, 0.1),
                        },
                        PlayModeEntity,
                    ));
                    commands.spawn((
                        Mesh3d(pillar_mesh.clone()),
                        MeshMaterial3d(wood_mat.clone()),
                        Transform::from_translation(
                            shelter_pos + rotation * Vec3::new(2.4, 1.8, -2.4),
                        )
                        .with_rotation(rotation),
                        avian3d::prelude::RigidBody::Static,
                        avian3d::prelude::Collider::cuboid(0.2, 3.6, 0.2),
                        crate::play_mode::WallCollider {
                            half_extents: Vec3::new(0.1, 1.8, 0.1),
                        },
                        PlayModeEntity,
                    ));
                    commands.spawn((
                        Mesh3d(pillar_mesh.clone()),
                        MeshMaterial3d(wood_mat.clone()),
                        Transform::from_translation(
                            shelter_pos + rotation * Vec3::new(-2.4, 1.8, 2.4),
                        )
                        .with_rotation(rotation),
                        avian3d::prelude::RigidBody::Static,
                        avian3d::prelude::Collider::cuboid(0.2, 3.6, 0.2),
                        crate::play_mode::WallCollider {
                            half_extents: Vec3::new(0.1, 1.8, 0.1),
                        },
                        PlayModeEntity,
                    ));
                    commands.spawn((
                        Mesh3d(pillar_mesh),
                        MeshMaterial3d(wood_mat.clone()),
                        Transform::from_translation(
                            shelter_pos + rotation * Vec3::new(2.4, 1.8, 2.4),
                        )
                        .with_rotation(rotation),
                        avian3d::prelude::RigidBody::Static,
                        avian3d::prelude::Collider::cuboid(0.2, 3.6, 0.2),
                        crate::play_mode::WallCollider {
                            half_extents: Vec3::new(0.1, 1.8, 0.1),
                        },
                        PlayModeEntity,
                    ));

                    let wall_mesh_back = meshes.add(Cuboid::new(4.8, 3.6, 0.08));
                    let wall_mesh_side = meshes.add(Cuboid::new(0.08, 3.6, 4.8));
                    let wall_mesh_front = meshes.add(Cuboid::new(1.8, 3.6, 0.08));

                    // Back Wall
                    commands.spawn((
                        Mesh3d(wall_mesh_back),
                        MeshMaterial3d(wood_mat.clone()),
                        Transform::from_translation(
                            shelter_pos + rotation * Vec3::new(0.0, 1.8, -2.4),
                        )
                        .with_rotation(rotation),
                        avian3d::prelude::RigidBody::Static,
                        avian3d::prelude::Collider::cuboid(4.8, 3.6, 0.08),
                        crate::play_mode::WallCollider {
                            half_extents: Vec3::new(2.4, 1.8, 0.04),
                        },
                        PlayModeEntity,
                    ));
                    // Left Wall
                    commands.spawn((
                        Mesh3d(wall_mesh_side.clone()),
                        MeshMaterial3d(wood_mat.clone()),
                        Transform::from_translation(
                            shelter_pos + rotation * Vec3::new(-2.4, 1.8, 0.0),
                        )
                        .with_rotation(rotation),
                        avian3d::prelude::RigidBody::Static,
                        avian3d::prelude::Collider::cuboid(0.08, 3.6, 4.8),
                        crate::play_mode::WallCollider {
                            half_extents: Vec3::new(0.04, 1.8, 2.4),
                        },
                        PlayModeEntity,
                    ));
                    // Right Wall
                    commands.spawn((
                        Mesh3d(wall_mesh_side),
                        MeshMaterial3d(wood_mat.clone()),
                        Transform::from_translation(
                            shelter_pos + rotation * Vec3::new(2.4, 1.8, 0.0),
                        )
                        .with_rotation(rotation),
                        avian3d::prelude::RigidBody::Static,
                        avian3d::prelude::Collider::cuboid(0.08, 3.6, 4.8),
                        crate::play_mode::WallCollider {
                            half_extents: Vec3::new(0.04, 1.8, 2.4),
                        },
                        PlayModeEntity,
                    ));

                    // Two front walls leaving a 1.2m wide doorway in the center
                    commands.spawn((
                        Mesh3d(wall_mesh_front.clone()),
                        MeshMaterial3d(wood_mat.clone()),
                        Transform::from_translation(
                            shelter_pos + rotation * Vec3::new(-1.5, 1.8, 2.4),
                        )
                        .with_rotation(rotation),
                        avian3d::prelude::RigidBody::Static,
                        avian3d::prelude::Collider::cuboid(1.8, 3.6, 0.08),
                        crate::play_mode::WallCollider {
                            half_extents: Vec3::new(0.9, 1.8, 0.04),
                        },
                        PlayModeEntity,
                    ));
                    commands.spawn((
                        Mesh3d(wall_mesh_front),
                        MeshMaterial3d(wood_mat),
                        Transform::from_translation(
                            shelter_pos + rotation * Vec3::new(1.5, 1.8, 2.4),
                        )
                        .with_rotation(rotation),
                        avian3d::prelude::RigidBody::Static,
                        avian3d::prelude::Collider::cuboid(1.8, 3.6, 0.08),
                        crate::play_mode::WallCollider {
                            half_extents: Vec3::new(0.9, 1.8, 0.04),
                        },
                        PlayModeEntity,
                    ));

                    let roof_mesh = meshes.add(Cuboid::new(2.7, 0.08, 5.2));
                    let roof_mat = materials.add(StandardMaterial {
                        base_color_texture: Some(
                            asset_server.load("textures/red_roof_shingles.png"),
                        ),
                        perceptual_roughness: 0.9,
                        ..default()
                    });

                    // A-frame roof sloped upwards (theta = 0.41 radians)
                    commands.spawn((
                        Mesh3d(roof_mesh.clone()),
                        MeshMaterial3d(roof_mat.clone()),
                        Transform::from_translation(
                            shelter_pos + rotation * Vec3::new(-1.2, 4.0, 0.0),
                        )
                        .with_rotation(rotation * Quat::from_rotation_z(0.41)),
                        avian3d::prelude::RigidBody::Static,
                        avian3d::prelude::Collider::cuboid(2.7, 0.08, 5.2),
                        PlayModeEntity,
                    ));
                    commands.spawn((
                        Mesh3d(roof_mesh),
                        MeshMaterial3d(roof_mat),
                        Transform::from_translation(
                            shelter_pos + rotation * Vec3::new(1.2, 4.0, 0.0),
                        )
                        .with_rotation(rotation * Quat::from_rotation_z(-0.41)),
                        avian3d::prelude::RigidBody::Static,
                        avian3d::prelude::Collider::cuboid(2.7, 0.08, 5.2),
                        PlayModeEntity,
                    ));

                    // Interactive Wooden Door Hinge Parent (placed at the left side of the doorway)
                    let door_hinge_pos = shelter_pos + rotation * Vec3::new(-0.58, 1.7, 2.4);
                    let closed_rot = rotation;
                    let open_rot = rotation * Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2);

                    let door_parent = commands
                        .spawn((
                            Transform::from_translation(door_hinge_pos).with_rotation(closed_rot),
                            crate::play_mode::house::HouseDoor {
                                is_open: false,
                                closed_rot,
                                open_rot,
                            },
                            crate::play_mode::WallCollider {
                                half_extents: Vec3::new(1.18, 1.7, 0.04), // covers entire doorway from -0.6 to 0.6 when closed
                            },
                            Visibility::Visible,
                            InheritedVisibility::default(),
                            PlayModeEntity,
                        ))
                        .id();

                    let door_mesh = meshes.add(Cuboid::new(1.16, 3.4, 0.08));
                    let door_mat = materials.add(StandardMaterial {
                        base_color_texture: Some(asset_server.load("textures/wooden_door.png")),
                        perceptual_roughness: 0.85,
                        ..default()
                    });

                    let door_child = commands
                        .spawn((
                            Mesh3d(door_mesh),
                            MeshMaterial3d(door_mat),
                            Transform::from_xyz(0.58, 0.0, 0.0), // offset so the hinge is at the edge
                            Visibility::default(),
                            InheritedVisibility::default(),
                            PlayModeEntity,
                        ))
                        .id();

                    commands.entity(door_parent).add_child(door_child);
                    inventory_log("🏡 Spacious 3D Wooden Shelter constructed!");
                }
            });
            ui.separator();

            // Recipe 3: Steel Broadsword
            ui.label(
                egui::RichText::new("⚔ Steel Broadsword")
                    .strong()
                    .color(egui::Color32::from_rgb(120, 200, 255)),
            );
            ui.label("Cost: 15 Iron, 5 Steel");
            let can_craft_sword =
                inventory.iron >= 15 && inventory.steel >= 5 && !inventory.has_sword;
            ui.horizontal(|ui| {
                if inventory.has_sword {
                    ui.label("⚔ Equipped (Glowing Blade!)");
                } else if ui
                    .add_enabled(can_craft_sword, egui::Button::new("Forge Steel Broadsword"))
                    .clicked()
                {
                    inventory.iron -= 15;
                    inventory.steel -= 5;
                    inventory.has_sword = true;
                    inventory_log("🎉 Forged glowing Steel Broadsword! Axe replaced.");
                }
            });
            ui.separator();

            // Recipe 4: Enhanced Metal Shelter Parts
            ui.label(
                egui::RichText::new("🔗 Enhanced Metal Shelter Parts")
                    .strong()
                    .color(egui::Color32::from_rgb(180, 180, 190)),
            );
            ui.label("Cost: 15 Iron, 10 Steel, 5 Copper");
            let can_craft_metal_parts =
                inventory.iron >= 15 && inventory.steel >= 10 && inventory.copper >= 5;
            ui.horizontal(|ui| {
                ui.label(format!("Owned: {}", inventory.metal_shelter_parts));
                if ui
                    .add_enabled(can_craft_metal_parts, egui::Button::new("⚒ Craft Parts"))
                    .clicked()
                {
                    inventory.iron -= 15;
                    inventory.steel -= 10;
                    inventory.copper -= 5;
                    inventory.metal_shelter_parts += 1;
                    inventory_log("⚒ Crafted: 1x Enhanced Metal Shelter Parts!");
                }
            });
            ui.separator();

            // Recipe 5: Enhanced Metal Shelter
            ui.label(
                egui::RichText::new("🛰 Enhanced Metal Shelter")
                    .strong()
                    .color(egui::Color32::from_rgb(100, 255, 180)),
            );
            ui.label("Cost: 1 Enhanced Parts, 20 Iron");
            let can_craft_metal_shelter =
                inventory.metal_shelter_parts >= 1 && inventory.iron >= 20;
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        can_craft_metal_shelter,
                        egui::Button::new("🏗 Build Metal Shelter"),
                    )
                    .clicked()
                {
                    inventory.metal_shelter_parts -= 1;
                    inventory.iron -= 20;

                    // Spawning 3D cyber metallic shelter in front of the player
                    let yaw = player.rotation_yaw;
                    let forward = Vec3::new(yaw.cos(), 0.0, yaw.sin());
                    let spawn_pos = player.position + forward * 3.0;
                    let terrain_y = get_bilinear_height(spawn_pos.x, spawn_pos.z, &map);
                    let shelter_pos = Vec3::new(spawn_pos.x, terrain_y, spawn_pos.z);

                    let rotation = Quat::from_rotation_y(-yaw);

                    let pillar_mesh = meshes.add(Cuboid::new(0.2, 3.0, 0.2));
                    let metal_mat = materials.add(StandardMaterial {
                        base_color: Color::srgb(0.65, 0.68, 0.72), // bright steel
                        metallic: 0.95,
                        perceptual_roughness: 0.15,
                        ..default()
                    });

                    // 4 corners (Pillars)
                    commands.spawn((
                        Mesh3d(pillar_mesh.clone()),
                        MeshMaterial3d(metal_mat.clone()),
                        Transform::from_translation(
                            shelter_pos + rotation * Vec3::new(-2.0, 1.5, -2.0),
                        )
                        .with_rotation(rotation),
                        avian3d::prelude::RigidBody::Static,
                        avian3d::prelude::Collider::cuboid(0.2, 3.0, 0.2),
                        crate::play_mode::WallCollider {
                            half_extents: Vec3::new(0.1, 1.5, 0.1),
                        },
                        PlayModeEntity,
                    ));
                    commands.spawn((
                        Mesh3d(pillar_mesh.clone()),
                        MeshMaterial3d(metal_mat.clone()),
                        Transform::from_translation(
                            shelter_pos + rotation * Vec3::new(2.0, 1.5, -2.0),
                        )
                        .with_rotation(rotation),
                        avian3d::prelude::RigidBody::Static,
                        avian3d::prelude::Collider::cuboid(0.2, 3.0, 0.2),
                        crate::play_mode::WallCollider {
                            half_extents: Vec3::new(0.1, 1.5, 0.1),
                        },
                        PlayModeEntity,
                    ));
                    commands.spawn((
                        Mesh3d(pillar_mesh.clone()),
                        MeshMaterial3d(metal_mat.clone()),
                        Transform::from_translation(
                            shelter_pos + rotation * Vec3::new(-2.0, 1.5, 2.0),
                        )
                        .with_rotation(rotation),
                        avian3d::prelude::RigidBody::Static,
                        avian3d::prelude::Collider::cuboid(0.2, 3.0, 0.2),
                        crate::play_mode::WallCollider {
                            half_extents: Vec3::new(0.1, 1.5, 0.1),
                        },
                        PlayModeEntity,
                    ));
                    commands.spawn((
                        Mesh3d(pillar_mesh),
                        MeshMaterial3d(metal_mat.clone()),
                        Transform::from_translation(
                            shelter_pos + rotation * Vec3::new(2.0, 1.5, 2.0),
                        )
                        .with_rotation(rotation),
                        avian3d::prelude::RigidBody::Static,
                        avian3d::prelude::Collider::cuboid(0.2, 3.0, 0.2),
                        crate::play_mode::WallCollider {
                            half_extents: Vec3::new(0.1, 1.5, 0.1),
                        },
                        PlayModeEntity,
                    ));

                    let wall_mesh_z = meshes.add(Cuboid::new(4.0, 3.0, 0.1));
                    let wall_mesh_x = meshes.add(Cuboid::new(0.1, 3.0, 4.0));
                    let wall_mesh_front = meshes.add(Cuboid::new(1.4, 3.0, 0.1));
                    let dark_panel_mat = materials.add(StandardMaterial {
                        base_color: Color::srgb(0.15, 0.17, 0.2), // sleek dark carbon gray
                        metallic: 0.8,
                        perceptual_roughness: 0.35,
                        ..default()
                    });

                    // Back Wall
                    commands.spawn((
                        Mesh3d(wall_mesh_z),
                        MeshMaterial3d(dark_panel_mat.clone()),
                        Transform::from_translation(
                            shelter_pos + rotation * Vec3::new(0.0, 1.5, -2.0),
                        )
                        .with_rotation(rotation),
                        avian3d::prelude::RigidBody::Static,
                        avian3d::prelude::Collider::cuboid(4.0, 3.0, 0.1),
                        crate::play_mode::WallCollider {
                            half_extents: Vec3::new(2.0, 1.5, 0.05),
                        },
                        PlayModeEntity,
                    ));
                    // Left Wall
                    commands.spawn((
                        Mesh3d(wall_mesh_x.clone()),
                        MeshMaterial3d(dark_panel_mat.clone()),
                        Transform::from_translation(
                            shelter_pos + rotation * Vec3::new(-2.0, 1.5, 0.0),
                        )
                        .with_rotation(rotation),
                        avian3d::prelude::RigidBody::Static,
                        avian3d::prelude::Collider::cuboid(0.1, 3.0, 4.0),
                        crate::play_mode::WallCollider {
                            half_extents: Vec3::new(0.05, 1.5, 2.0),
                        },
                        PlayModeEntity,
                    ));
                    // Right Wall
                    commands.spawn((
                        Mesh3d(wall_mesh_x),
                        MeshMaterial3d(dark_panel_mat.clone()),
                        Transform::from_translation(
                            shelter_pos + rotation * Vec3::new(2.0, 1.5, 0.0),
                        )
                        .with_rotation(rotation),
                        avian3d::prelude::RigidBody::Static,
                        avian3d::prelude::Collider::cuboid(0.1, 3.0, 4.0),
                        crate::play_mode::WallCollider {
                            half_extents: Vec3::new(0.05, 1.5, 2.0),
                        },
                        PlayModeEntity,
                    ));
                    // Two Front Walls leaving a 1.2m wide doorway in the center
                    commands.spawn((
                        Mesh3d(wall_mesh_front.clone()),
                        MeshMaterial3d(dark_panel_mat.clone()),
                        Transform::from_translation(
                            shelter_pos + rotation * Vec3::new(-1.3, 1.5, 2.0),
                        )
                        .with_rotation(rotation),
                        avian3d::prelude::RigidBody::Static,
                        avian3d::prelude::Collider::cuboid(1.4, 3.0, 0.1),
                        crate::play_mode::WallCollider {
                            half_extents: Vec3::new(0.7, 1.5, 0.05),
                        },
                        PlayModeEntity,
                    ));
                    commands.spawn((
                        Mesh3d(wall_mesh_front),
                        MeshMaterial3d(dark_panel_mat),
                        Transform::from_translation(
                            shelter_pos + rotation * Vec3::new(1.3, 1.5, 2.0),
                        )
                        .with_rotation(rotation),
                        avian3d::prelude::RigidBody::Static,
                        avian3d::prelude::Collider::cuboid(1.4, 3.0, 0.1),
                        crate::play_mode::WallCollider {
                            half_extents: Vec3::new(0.7, 1.5, 0.05),
                        },
                        PlayModeEntity,
                    ));

                    // Glowing neon bands
                    let neon_mat = materials.add(StandardMaterial {
                        base_color: Color::srgb(0.0, 0.85, 1.0),
                        emissive: LinearRgba::from(Color::srgb(0.0, 0.85, 1.0)) * 3.5,
                        ..default()
                    });
                    let strip_mesh_z = meshes.add(Cuboid::new(3.8, 0.05, 0.11));
                    let strip_mesh_x = meshes.add(Cuboid::new(0.11, 0.05, 3.8));

                    commands.spawn((
                        Mesh3d(strip_mesh_z),
                        MeshMaterial3d(neon_mat.clone()),
                        Transform::from_translation(
                            shelter_pos + rotation * Vec3::new(0.0, 1.5, -2.0),
                        )
                        .with_rotation(rotation),
                        PlayModeEntity,
                    ));
                    commands.spawn((
                        Mesh3d(strip_mesh_x.clone()),
                        MeshMaterial3d(neon_mat.clone()),
                        Transform::from_translation(
                            shelter_pos + rotation * Vec3::new(-2.0, 1.5, 0.0),
                        )
                        .with_rotation(rotation),
                        PlayModeEntity,
                    ));
                    commands.spawn((
                        Mesh3d(strip_mesh_x),
                        MeshMaterial3d(neon_mat),
                        Transform::from_translation(
                            shelter_pos + rotation * Vec3::new(2.0, 1.5, 0.0),
                        )
                        .with_rotation(rotation),
                        PlayModeEntity,
                    ));

                    let roof_mesh = meshes.add(Cuboid::new(4.4, 0.12, 4.4));
                    commands.spawn((
                        Mesh3d(roof_mesh),
                        MeshMaterial3d(metal_mat.clone()),
                        Transform::from_translation(
                            shelter_pos + rotation * Vec3::new(0.0, 3.06, 0.0),
                        )
                        .with_rotation(rotation),
                        avian3d::prelude::RigidBody::Static,
                        avian3d::prelude::Collider::cuboid(4.4, 0.12, 4.4),
                        PlayModeEntity,
                    ));

                    let beacon_base_mesh = meshes.add(Cylinder::new(0.08, 0.4));
                    commands.spawn((
                        Mesh3d(beacon_base_mesh),
                        MeshMaterial3d(metal_mat),
                        Transform::from_translation(
                            shelter_pos + rotation * Vec3::new(0.0, 3.26, 0.0),
                        )
                        .with_rotation(rotation),
                        PlayModeEntity,
                    ));

                    let beacon_glow_mesh = meshes.add(Sphere::new(0.16).mesh().ico(3).unwrap());
                    let beacon_glow_mat = materials.add(StandardMaterial {
                        base_color: Color::srgb(1.0, 0.0, 0.0),
                        emissive: LinearRgba::from(Color::srgb(1.0, 0.0, 0.0)) * 5.0,
                        ..default()
                    });
                    commands.spawn((
                        Mesh3d(beacon_glow_mesh),
                        MeshMaterial3d(beacon_glow_mat),
                        Transform::from_translation(
                            shelter_pos + rotation * Vec3::new(0.0, 3.5, 0.0),
                        )
                        .with_rotation(rotation),
                        PlayModeEntity,
                    ));

                    // Red Point Light
                    commands.spawn((
                        PointLight {
                            color: Color::srgb(1.0, 0.0, 0.0),
                            intensity: 800.0,
                            range: 12.0,
                            shadow_maps_enabled: true,
                            ..default()
                        },
                        Transform::from_translation(
                            shelter_pos + rotation * Vec3::new(0.0, 3.5, 0.0),
                        ),
                        PlayModeEntity,
                    ));

                    // Interactive Cyber Metal Door Hinge Parent (placed at the left side of the doorway)
                    let door_hinge_pos = shelter_pos + rotation * Vec3::new(-0.58, 1.4, 2.0);
                    let closed_rot = rotation;
                    let open_rot = rotation * Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2);

                    let door_parent = commands
                        .spawn((
                            Transform::from_translation(door_hinge_pos).with_rotation(closed_rot),
                            crate::play_mode::house::HouseDoor {
                                is_open: false,
                                closed_rot,
                                open_rot,
                            },
                            crate::play_mode::WallCollider {
                                half_extents: Vec3::new(1.18, 1.4, 0.04), // covers entire doorway from -0.6 to 0.6 when closed
                            },
                            Visibility::Visible,
                            InheritedVisibility::default(),
                            PlayModeEntity,
                        ))
                        .id();

                    let door_mesh = meshes.add(Cuboid::new(1.16, 2.8, 0.08));
                    let door_mat = materials.add(StandardMaterial {
                        base_color_texture: Some(asset_server.load("textures/cyber_door.png")),
                        perceptual_roughness: 0.85,
                        ..default()
                    });

                    let door_child = commands
                        .spawn((
                            Mesh3d(door_mesh),
                            MeshMaterial3d(door_mat),
                            Transform::from_xyz(0.58, 0.0, 0.0), // offset so the hinge is at the edge
                            Visibility::default(),
                            InheritedVisibility::default(),
                            PlayModeEntity,
                        ))
                        .id();

                    commands.entity(door_parent).add_child(door_child);

                    inventory_log(
                        "🛰 Enhanced Cyber Shelter with red beacon and security door spawned!",
                    );
                }
            });
        });

    // 3. Mining Activity Log Panel
    egui::Window::new("📜 Mining Activity Log")
        .default_width(320.0)
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::Vec2::new(-10.0, -10.0))
        .collapsible(true)
        .resizable(false)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .max_height(140.0)
                .show(ui, |ui| {
                    for entry in inventory.loot_log.iter().rev() {
                        let text_color = if entry.starts_with("+") {
                            egui::Color32::from_rgb(100, 255, 100)
                        } else if entry.starts_with("🎉") {
                            egui::Color32::from_rgb(255, 215, 0)
                        } else {
                            egui::Color32::WHITE
                        };
                        ui.label(egui::RichText::new(entry).color(text_color));
                    }
                });

            if player.state == PlayerState::Ragdoll {
                ui.add_space(5.0);
                ui.add(egui::Label::new(
                    egui::RichText::new("🤸 SKELETON TUMBLING! Press [Space] to Get Up!")
                        .strong()
                        .color(egui::Color32::from_rgb(255, 120, 0)),
                ));
            }
        });
}

// System governing visual weapon replacement dynamically
#[allow(clippy::too_many_arguments)]
fn play_weapon_sync_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    inventory: Res<PlayerInventory>,
    player_query: Query<&PlayModePlayer>,
    weapon_query: Query<(Entity, &PlayWeaponVisual, &ChildOf)>,
    joint_query: Query<(Entity, &PlayJointVisual)>,
    camera_query: Query<(Entity, &PlayModeCamera)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(player) = player_query.single() else {
        return;
    };

    // Find right hand entity
    let mut right_hand_entity = None;
    for (entity, visual) in joint_query.iter() {
        if visual.name == "R_Hand" {
            right_hand_entity = Some(entity);
            break;
        }
    }
    let Some(hand_entity) = right_hand_entity else {
        return;
    };

    // Also find the camera entity in case the weapon is attached to the FPS camera
    let cam_entity_opt = camera_query.iter().next().map(|(e, _)| e);

    // Check currently spawned weapon visual
    let mut current_weapon = None;
    for (entity, visual, parent) in weapon_query.iter() {
        if parent.get() == hand_entity || Some(parent.get()) == cam_entity_opt {
            current_weapon = Some((entity, visual));
            break;
        }
    }

    let target_weapon = player.active_weapon;
    let target_is_sword = target_weapon == ActiveWeapon::Melee && inventory.has_sword;

    let needs_respawn = match current_weapon {
        None => true,
        Some((_, visual)) => {
            visual.weapon_type != target_weapon
                || (target_weapon == ActiveWeapon::Melee && visual.is_sword != target_is_sword)
        }
    };

    if needs_respawn {
        if let Some((entity, _)) = current_weapon {
            commands.entity(entity).despawn();
        }

        let weapon_entity = match target_weapon {
            ActiveWeapon::Melee => {
                if target_is_sword {
                    let handle_mesh = meshes.add(Cylinder::new(0.025, 0.3));
                    let handle_mat = materials.add(StandardMaterial {
                        base_color: Color::srgb(0.2, 0.22, 0.25),
                        metallic: 0.9,
                        perceptual_roughness: 0.2,
                        ..default()
                    });
                    let handle = commands
                        .spawn((
                            Mesh3d(handle_mesh),
                            MeshMaterial3d(handle_mat),
                            Transform::from_xyz(0.0, -0.15, 0.15).with_rotation(
                                Quat::from_rotation_x(std::f32::consts::FRAC_PI_2 - 0.1),
                            ),
                            PlayWeaponVisual {
                                weapon_type: ActiveWeapon::Melee,
                                is_sword: true,
                            },
                            PlayModeEntity,
                        ))
                        .id();

                    let guard_mesh = meshes.add(Cuboid::new(0.28, 0.04, 0.06));
                    let guard_mat = materials.add(StandardMaterial {
                        base_color: Color::srgb(0.85, 0.65, 0.15),
                        metallic: 0.95,
                        perceptual_roughness: 0.1,
                        ..default()
                    });
                    let guard = commands
                        .spawn((
                            Mesh3d(guard_mesh),
                            MeshMaterial3d(guard_mat),
                            Transform::from_xyz(0.0, 0.16, 0.0),
                            PlayModeEntity,
                        ))
                        .id();
                    commands.entity(handle).add_child(guard);

                    let blade_mesh = meshes.add(Cuboid::new(0.08, 1.2, 0.02));
                    let blade_mat = materials.add(StandardMaterial {
                        base_color: Color::srgb(0.3, 0.7, 1.0),
                        metallic: 0.98,
                        perceptual_roughness: 0.05,
                        emissive: LinearRgba::from(Color::srgb(0.3, 0.7, 1.0)) * 2.5,
                        ..default()
                    });
                    let blade = commands
                        .spawn((
                            Mesh3d(blade_mesh),
                            MeshMaterial3d(blade_mat),
                            Transform::from_xyz(0.0, 0.76, 0.0),
                            PlayModeEntity,
                        ))
                        .id();
                    commands.entity(handle).add_child(blade);
                    handle
                } else {
                    let handle_mesh = meshes.add(Cuboid::new(0.04, 1.0, 0.04));
                    let handle_mat = materials.add(StandardMaterial {
                        base_color: Color::srgb(0.42, 0.25, 0.15),
                        perceptual_roughness: 0.9,
                        ..default()
                    });
                    let handle = commands
                        .spawn((
                            Mesh3d(handle_mesh),
                            MeshMaterial3d(handle_mat),
                            Transform::from_xyz(0.0, -0.22, 0.18).with_rotation(
                                Quat::from_rotation_x(std::f32::consts::FRAC_PI_2 - 0.1),
                            ),
                            PlayWeaponVisual {
                                weapon_type: ActiveWeapon::Melee,
                                is_sword: false,
                            },
                            PlayModeEntity,
                        ))
                        .id();

                    let blade_mesh = meshes.add(Cuboid::new(0.06, 0.28, 0.25));
                    let blade_mat = materials.add(StandardMaterial {
                        base_color: Color::srgb(0.72, 0.75, 0.78),
                        metallic: 0.95,
                        perceptual_roughness: 0.2,
                        ..default()
                    });
                    let blade = commands
                        .spawn((
                            Mesh3d(blade_mesh),
                            MeshMaterial3d(blade_mat),
                            Transform::from_xyz(0.0, 0.48, 0.08),
                            PlayModeEntity,
                        ))
                        .id();
                    commands.entity(handle).add_child(blade);
                    handle
                }
            }
            ActiveWeapon::Pistol => commands
                .spawn((
                    WorldAssetRoot(asset_server.load("Gun_Pistol.gltf#Scene0")),
                    Transform::from_xyz(0.0, -0.05, 0.18)
                        .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
                    PlayWeaponVisual {
                        weapon_type: ActiveWeapon::Pistol,
                        is_sword: false,
                    },
                    PlayModeEntity,
                    Visibility::Visible,
                    InheritedVisibility::default(),
                ))
                .id(),
            ActiveWeapon::Revolver => commands
                .spawn((
                    WorldAssetRoot(asset_server.load("Gun_Revolver.gltf#Scene0")),
                    Transform::from_xyz(0.0, -0.05, 0.18)
                        .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
                    PlayWeaponVisual {
                        weapon_type: ActiveWeapon::Revolver,
                        is_sword: false,
                    },
                    PlayModeEntity,
                    Visibility::Visible,
                    InheritedVisibility::default(),
                ))
                .id(),
            ActiveWeapon::Rifle => commands
                .spawn((
                    WorldAssetRoot(asset_server.load("Gun_Rifle.gltf#Scene0")),
                    Transform::from_xyz(0.0, -0.08, 0.22)
                        .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
                    PlayWeaponVisual {
                        weapon_type: ActiveWeapon::Rifle,
                        is_sword: false,
                    },
                    PlayModeEntity,
                    Visibility::Visible,
                    InheritedVisibility::default(),
                ))
                .id(),
            ActiveWeapon::Sniper => commands
                .spawn((
                    WorldAssetRoot(asset_server.load("Gun_Sniper.gltf#Scene0")),
                    Transform::from_xyz(0.0, -0.1, 0.25)
                        .with_rotation(Quat::from_rotation_y(std::f32::consts::PI)),
                    PlayWeaponVisual {
                        weapon_type: ActiveWeapon::Sniper,
                        is_sword: false,
                    },
                    PlayModeEntity,
                    Visibility::Visible,
                    InheritedVisibility::default(),
                ))
                .id(),
        };

        commands.entity(hand_entity).add_child(weapon_entity);
        inventory_log(&format!(
            "✨ Visual weapon synced: {:?} active!",
            target_weapon
        ));
    }
}

// System to dynamically detach the weapon and attach it to the camera in first person mode
fn weapon_attachment_system(
    mut commands: Commands,
    camera_query: Query<(Entity, &PlayModeCamera)>,
    weapon_query: Query<(Entity, &PlayWeaponVisual, &ChildOf)>,
    joint_query: Query<(Entity, &PlayJointVisual)>,
) {
    let Ok((cam_entity, camera)) = camera_query.single() else {
        return;
    };

    let mut right_hand_entity = None;
    for (entity, visual) in joint_query.iter() {
        if visual.name == "R_Hand" {
            right_hand_entity = Some(entity);
            break;
        }
    }
    let Some(hand_entity) = right_hand_entity else {
        return;
    };

    let is_first_person = camera.view_mode == ViewMode::FirstPerson;

    for (weapon_entity, visual, parent) in weapon_query.iter() {
        if is_first_person {
            // If it's not already on the camera, move it to the camera
            if parent.parent() != cam_entity {
                commands.entity(cam_entity).add_child(weapon_entity);
                // Set the local transform for a perfect FPS viewmodel
                let (offset, rot) = match visual.weapon_type {
                    ActiveWeapon::Melee => (
                        Vec3::new(0.3, -0.3, -0.5),
                        Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
                    ),
                    ActiveWeapon::Pistol | ActiveWeapon::Revolver => {
                        // Point straight ahead relative to camera (undo the 180 deg rotation that caused it to be sideways/backwards)
                        (
                            Vec3::new(0.2, -0.2, -0.4),
                            Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
                        )
                    }
                    ActiveWeapon::Rifle | ActiveWeapon::Sniper => (
                        Vec3::new(0.2, -0.2, -0.5),
                        Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2),
                    ),
                };
                commands
                    .entity(weapon_entity)
                    .insert(Transform::from_translation(offset).with_rotation(rot));
            }
        } else {
            // If it's not already on the hand, move it to the hand
            if parent.parent() != hand_entity {
                commands.entity(hand_entity).add_child(weapon_entity);
                // Restore default hand transforms
                let (offset, rot) = match visual.weapon_type {
                    ActiveWeapon::Melee => {
                        if visual.is_sword {
                            (
                                Vec3::new(0.0, -0.15, 0.15),
                                Quat::from_rotation_x(std::f32::consts::FRAC_PI_2 - 0.1),
                            )
                        } else {
                            (
                                Vec3::new(0.0, -0.22, 0.18),
                                Quat::from_rotation_x(std::f32::consts::FRAC_PI_2 - 0.1),
                            )
                        }
                    }
                    ActiveWeapon::Pistol | ActiveWeapon::Revolver => (
                        Vec3::new(0.0, -0.05, 0.18),
                        Quat::from_rotation_y(std::f32::consts::PI),
                    ),
                    ActiveWeapon::Rifle => (
                        Vec3::new(0.0, -0.08, 0.22),
                        Quat::from_rotation_y(std::f32::consts::PI),
                    ),
                    ActiveWeapon::Sniper => (
                        Vec3::new(0.0, -0.1, 0.25),
                        Quat::from_rotation_y(std::f32::consts::PI),
                    ),
                };
                commands
                    .entity(weapon_entity)
                    .insert(Transform::from_translation(offset).with_rotation(rot));
            }
        }
    }
}

use std::sync::Mutex;
static LOG_QUEUE: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn inventory_log(msg: &str) {
    println!("[PlayMode Log] {}", msg);
    if let Ok(mut queue) = LOG_QUEUE.lock() {
        queue.push(msg.to_string());
    }
}

fn sync_logs(mut inventory: ResMut<PlayerInventory>) {
    if let Ok(mut queue) = LOG_QUEUE.lock() {
        while !queue.is_empty() {
            let msg = queue.remove(0);
            inventory.loot_log.push(msg);
            if inventory.loot_log.len() > 25 {
                inventory.loot_log.remove(0);
            }
        }
    }
}

// Bilinear interpolation for heightmap heights
pub fn get_bilinear_height(x: f32, z: f32, map: &TempestMap) -> f32 {
    let w = map.width as f32;
    let h = map.height as f32;

    let grid_x = x + w / 2.0;
    let grid_z = z + h / 2.0;

    let x0 = (grid_x.floor() as u32).min(map.width - 1);
    let x1 = (x0 + 1).min(map.width - 1);
    let z0 = (grid_z.floor() as u32).min(map.height - 1);
    let z1 = (z0 + 1).min(map.height - 1);

    let tx = grid_x - grid_x.floor();
    let tz = grid_z - grid_z.floor();

    let h00 = map.get_height(x0, z0);
    let h10 = map.get_height(x1, z0);
    let h01 = map.get_height(x0, z1);
    let h11 = map.get_height(x1, z1);

    let h0 = h00 * (1.0 - tx) + h10 * tx;
    let h1 = h01 * (1.0 - tx) + h11 * tx;

    h0 * (1.0 - tz) + h1 * tz
}

// Spawn prefab visuals helper for Play Mode
fn spawn_play_prefab(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    prefab_type: &str,
    position: Vec3,
    rotation_y: f32,
) -> Entity {
    let parent = commands
        .spawn((
            Transform::from_translation(position).with_rotation(Quat::from_rotation_y(rotation_y)),
            Visibility::Visible,
            InheritedVisibility::default(),
            PlayModeEntity,
        ))
        .id();

    match prefab_type {
        s if s.starts_with("tree") || s == "shrub" || s == "cactus" => {
            let seed =
                ((position.x.abs() * 1000.0) as u32 ^ (position.z.abs() * 1000.0) as u32) | 1;
            let (trunk_mesh, leaves_mesh) =
                crate::map_editor::tree_generator::build_tree_meshes(seed, s);

            let trunk = commands
                .spawn((
                    Mesh3d(meshes.add(trunk_mesh)),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::WHITE,
                        perceptual_roughness: 0.85,
                        ..default()
                    })),
                    Transform::default(),
                    PlayModeEntity,
                ))
                .id();

            let leaves = commands
                .spawn((
                    Mesh3d(meshes.add(leaves_mesh)),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::WHITE,
                        perceptual_roughness: 0.75,
                        ..default()
                    })),
                    Transform::default(),
                    PlayModeEntity,
                ))
                .id();

            commands.entity(parent).add_child(trunk).add_child(leaves);

            // Add static physics collider
            let col = if s.starts_with("tree") {
                Some((
                    avian3d::prelude::Collider::cylinder(0.24, 3.8),
                    Vec3::new(0.0, 1.9, 0.0),
                ))
            } else if s == "cactus" {
                Some((
                    avian3d::prelude::Collider::cylinder(0.15, 2.0),
                    Vec3::new(0.0, 1.0, 0.0),
                ))
            } else if s == "shrub" {
                Some((
                    avian3d::prelude::Collider::sphere(0.4),
                    Vec3::new(0.0, 0.3, 0.0),
                ))
            } else {
                None
            };
            if let Some((collider, offset)) = col {
                let col_child = commands
                    .spawn((
                        avian3d::prelude::RigidBody::Static,
                        collider,
                        Transform::from_translation(offset),
                        PlayModeEntity,
                    ))
                    .id();
                commands.entity(parent).add_child(col_child);
            }
        }
        "rock" => {
            let seed = ((position.x.abs() * 500.0) as u32 ^ (position.z.abs() * 500.0) as u32) | 1;
            let mut lcg_s = seed;
            let mut next_rand = move || {
                lcg_s = lcg_s.wrapping_mul(1103515245).wrapping_add(12345);
                (lcg_s as f32) / (u32::MAX as f32)
            };

            let scale_x = 0.8 + next_rand() * 0.6;
            let scale_y = 0.6 + next_rand() * 0.4;
            let scale_z = 0.8 + next_rand() * 0.6;

            let rock_mesh = crate::map_editor::tree_generator::build_rock_mesh(seed);

            let rock = commands
                .spawn((
                    Mesh3d(meshes.add(rock_mesh)),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::WHITE,
                        perceptual_roughness: 0.95,
                        metallic: 0.05,
                        ..default()
                    })),
                    Transform::from_scale(Vec3::new(scale_x, scale_y, scale_z)),
                    PlayModeEntity,
                ))
                .id();

            commands.entity(parent).add_child(rock);

            // Add static physics collider
            let col_child = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::sphere(0.6 * scale_x.max(scale_z)),
                    Transform::from_translation(Vec3::new(0.0, 0.4, 0.0)),
                    PlayModeEntity,
                ))
                .id();
            commands.entity(parent).add_child(col_child);
        }
        s if s.starts_with("ore_") => {
            let seed = ((position.x.abs() * 500.0) as u32 ^ (position.z.abs() * 500.0) as u32) | 1;
            let mut lcg_s = seed;
            let mut next_rand = move || {
                lcg_s = lcg_s.wrapping_mul(1103515245).wrapping_add(12345);
                (lcg_s as f32) / (u32::MAX as f32)
            };

            let rock_mesh = crate::map_editor::tree_generator::build_rock_mesh(seed);

            let base_color = if s == "ore_granite" {
                Color::srgb(0.2, 0.2, 0.22)
            } else {
                Color::srgb(0.35, 0.35, 0.37)
            };

            let base_rock = commands
                .spawn((
                    Mesh3d(meshes.add(rock_mesh)),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color,
                        perceptual_roughness: 0.9,
                        metallic: 0.05,
                        ..default()
                    })),
                    Transform::from_scale(Vec3::new(1.0, 0.6, 1.0)),
                    PlayModeEntity,
                ))
                .id();

            commands.entity(parent).add_child(base_rock);

            let (ore_color, crystal_metal, crystal_rough, crystal_emissive) = match s {
                "ore_copper" => (
                    Color::srgb(0.9, 0.4, 0.25),
                    0.95,
                    0.2,
                    Color::srgb(0.3, 0.1, 0.0),
                ),
                "ore_iron" => (Color::srgb(0.65, 0.25, 0.1), 0.8, 0.5, Color::BLACK),
                "ore_gold" => (
                    Color::srgb(1.0, 0.82, 0.1),
                    1.0,
                    0.1,
                    Color::srgb(0.4, 0.3, 0.0),
                ),
                "ore_silver" => (
                    Color::srgb(0.95, 0.95, 0.98),
                    1.0,
                    0.15,
                    Color::srgb(0.2, 0.2, 0.2),
                ),
                "ore_platinum" => (
                    Color::srgb(0.85, 0.9, 1.0),
                    1.0,
                    0.05,
                    Color::srgb(0.3, 0.35, 0.5),
                ),
                "ore_steel" => (Color::srgb(0.5, 0.52, 0.55), 0.9, 0.3, Color::BLACK),
                "ore_granite" => (Color::srgb(0.4, 0.4, 0.45), 0.0, 0.95, Color::BLACK),
                _ => (Color::WHITE, 0.0, 1.0, Color::BLACK),
            };

            let crystal_mat = materials.add(StandardMaterial {
                base_color: ore_color,
                metallic: crystal_metal,
                perceptual_roughness: crystal_rough,
                emissive: LinearRgba::from(crystal_emissive),
                ..default()
            });

            let crystal_count = 3 + (seed % 3);
            for i in 0..crystal_count {
                let rx = (next_rand() - 0.5) * 0.8;
                let ry = next_rand() * std::f32::consts::TAU;
                let rz = (next_rand() - 0.5) * 0.8;

                let c_scale = Vec3::new(
                    0.15 + next_rand() * 0.15,
                    0.4 + next_rand() * 0.6,
                    0.15 + next_rand() * 0.15,
                );

                let offset_x = (next_rand() - 0.5) * 0.5;
                let offset_y = 0.2 + next_rand() * 0.2;
                let offset_z = (next_rand() - 0.5) * 0.5;

                let shard_mesh = crate::map_editor::tree_generator::build_rock_mesh(seed + i + 1);

                let shard = commands
                    .spawn((
                        Mesh3d(meshes.add(shard_mesh)),
                        MeshMaterial3d(crystal_mat.clone()),
                        Transform::from_translation(Vec3::new(offset_x, offset_y, offset_z))
                            .with_rotation(Quat::from_euler(EulerRot::YXZ, ry, rx, rz))
                            .with_scale(c_scale),
                        PlayModeEntity,
                    ))
                    .id();

                commands.entity(parent).add_child(shard);
            }

            // Add static physics collider
            let col_child = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::sphere(0.8),
                    Transform::from_translation(Vec3::new(0.0, 0.3, 0.0)),
                    PlayModeEntity,
                ))
                .id();
            commands.entity(parent).add_child(col_child);
        }
        _ => {}
    }

    parent
}

// Spawns splash particles in water
fn spawn_water_splash(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    origin: Vec3,
) {
    let splash_mesh = meshes.add(Sphere::new(0.12).mesh().ico(3).unwrap());
    let splash_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.4, 0.7, 1.0, 0.6),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    for i in 0..12 {
        let angle = (i as f32) * std::f32::consts::TAU / 12.0;
        let speed = 1.2 + rand::random::<f32>() * 2.2;
        let velocity = Vec3::new(
            angle.cos() * speed,
            3.0 + rand::random::<f32>() * 3.5,
            angle.sin() * speed,
        );

        commands.spawn((
            Mesh3d(splash_mesh.clone()),
            MeshMaterial3d(splash_mat.clone()),
            Transform::from_translation(origin),
            PlayParticle {
                velocity,
                lifetime: 0.0,
                max_lifetime: 0.4 + rand::random::<f32>() * 0.5,
                color: Color::srgba(0.45, 0.75, 1.0, 0.6),
            },
            PlayModeEntity,
        ));
    }
}

#[derive(Component)]
pub struct PlayModeCloud;

fn cloud_drift_system(
    player_query: Query<&PlayModePlayer>,
    mut query: Query<&mut Transform, With<PlayModeCloud>>,
) {
    let Ok(player) = player_query.single() else {
        return;
    };
    for mut transform in query.iter_mut() {
        transform.translation.x = player.position.x;
        transform.translation.z = player.position.z;
    }
}

fn play_mode_mouse_grab_system(
    mut window_query: Query<&mut CursorOptions, With<Window>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    key_input: Res<ButtonInput<KeyCode>>,
    mut contexts: EguiContexts,
) {
    let Ok(mut cursor_options) = window_query.single_mut() else {
        return;
    };
    if mouse_button.just_pressed(MouseButton::Left) {
        if let Ok(ctx) = contexts.ctx_mut()
            && ctx.egui_wants_pointer_input()
        {
            return;
        }
        cursor_options.visible = false;
        cursor_options.grab_mode = CursorGrabMode::Locked;
    }
    if key_input.just_pressed(KeyCode::Escape) {
        cursor_options.visible = true;
        cursor_options.grab_mode = CursorGrabMode::None;
    }
}

fn release_mouse_on_exit(mut window_query: Query<&mut CursorOptions, With<Window>>) {
    if let Ok(mut cursor_options) = window_query.single_mut() {
        cursor_options.visible = true;
        cursor_options.grab_mode = CursorGrabMode::None;
    }
}

fn generate_cloud_texture(perlin: &crate::map_editor::noise::PerlinNoise) -> Image {
    let size = 128;
    let mut data = vec![0u8; size * size * 4];

    for y in 0..size {
        for x in 0..size {
            let idx = (y * size + x) * 4;
            let nx = x as f32 / 12.0;
            let ny = y as f32 / 12.0;
            let val = perlin.noise(nx, ny) * 0.5
                + perlin.noise(nx * 2.0, ny * 2.0) * 0.25
                + perlin.noise(nx * 4.0, ny * 4.0) * 0.125;

            let intensity = ((val + 0.35).clamp(0.0, 1.0) * 255.0) as u8;
            let alpha = if intensity < 90 {
                0
            } else {
                ((intensity as f32 - 90.0) / 165.0 * 210.0) as u8
            };

            data[idx] = 255;
            data[idx + 1] = 255;
            data[idx + 2] = 255;
            data[idx + 3] = alpha;
        }
    }

    Image::new(
        Extent3d {
            width: size as u32,
            height: size as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

fn play_sky_cycle_system(
    time: Res<Time>,
    mut sun_query: Query<(&mut Transform, &PlaySun, Option<&mut DirectionalLight>)>,
    mut clear_color: ResMut<ClearColor>,
    mut camera_query: Query<&mut DistanceFog, With<PlayModeCamera>>,
) {
    let elapsed = time.elapsed_secs();

    // Day cycle speed (one full cycle takes 374 seconds ~ 6.2 minutes)
    let day_speed = 0.0168;
    let master_phase = elapsed * day_speed;

    let mut highest_sun_y = -999.0;

    for (mut transform, sun, opt_light) in sun_query.iter_mut() {
        let phase = master_phase * sun.orbit_speed + sun.angle_offset;
        let radius = 150.0;

        let x = phase.cos() * radius;
        let y = phase.sin() * radius;
        let z = (phase * 0.5).cos() * radius * 0.4;

        transform.translation = Vec3::new(x, y, z);

        if y > highest_sun_y {
            highest_sun_y = y;
        }

        if let Some(mut light) = opt_light {
            transform.look_at(Vec3::ZERO, Vec3::Y);

            let t = (y / radius).clamp(-0.2, 0.2) * 2.5 + 0.5; // smooth twilight/day fade
            let intensity = t.clamp(0.0, 1.0) * sun.day_intensity;
            light.illuminance = intensity;
        }
    }

    // Set clear color based on highest sun height (day/night transition)
    let sky_factor = (highest_sun_y / 150.0).clamp(-0.5, 1.0);
    let night_linear = Color::srgb(0.04, 0.03, 0.08).to_linear();
    let twilight_linear = Color::srgb(0.35, 0.12, 0.28).to_linear();
    let day_linear = Color::srgb(0.18, 0.22, 0.45).to_linear();

    let current_linear = if sky_factor < 0.0 {
        let t = (sky_factor + 0.5) / 0.5;
        let r = night_linear.red + (twilight_linear.red - night_linear.red) * t;
        let g = night_linear.green + (twilight_linear.green - night_linear.green) * t;
        let b = night_linear.blue + (twilight_linear.blue - night_linear.blue) * t;
        LinearRgba::new(r, g, b, 1.0)
    } else {
        let r = twilight_linear.red + (day_linear.red - twilight_linear.red) * sky_factor;
        let g = twilight_linear.green + (day_linear.green - twilight_linear.green) * sky_factor;
        let b = twilight_linear.blue + (day_linear.blue - twilight_linear.blue) * sky_factor;
        LinearRgba::new(r, g, b, 1.0)
    };

    clear_color.0 = Color::from(current_linear);
    for mut fog in camera_query.iter_mut() {
        fog.color = Color::from(current_linear);
    }
}

use crate::play_mode::creatures::PlayCreature;
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn gun_fire_and_bullet_system(
    mut commands: Commands,
    mut player_query: Query<&mut PlayModePlayer>,
    camera_query: Query<
        (&Transform, &PlayModeCamera),
        (
            Without<PlayModePlayer>,
            Without<Bullet>,
            Without<PlayCreature>,
        ),
    >,
    mut bullet_query: Query<
        (Entity, &mut Bullet, &mut Transform),
        (Without<PlayModeCamera>, Without<PlayCreature>),
    >,
    mut creature_query: Query<
        (
            Entity,
            &mut PlayCreature,
            &mut Transform,
            Option<&mut creatures::AggroState>,
        ),
        (Without<PlayModeCamera>, Without<Bullet>),
    >,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    map: Res<TempestMap>,
    water_settings: Res<WaterSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut impulse_writer: MessageWriter<WaterImpulseEvent>,
    builder: Res<crate::procedural_walls::ProceduralWallBuilder>,
    window_query: Query<&CursorOptions, With<Window>>,
) {
    let dt = time.delta_secs();

    // 1. Tick Player Reload and Automatic Firing Timers
    let Ok(mut player) = player_query.single_mut() else {
        return;
    };

    if player.automatic_fire_timer > 0.0 {
        player.automatic_fire_timer -= dt;
    }

    // Tick reload timer
    if let Some(mut timer) = player.reload_timer {
        timer -= dt;
        if timer <= 0.0 {
            // Perform ammo replenishment!
            match player.active_weapon {
                ActiveWeapon::Pistol => {
                    let needed = 8 - player.clip_pistol;
                    let loaded = needed.min(player.ammo_pistol);
                    player.clip_pistol += loaded;
                    player.ammo_pistol -= loaded;
                }
                ActiveWeapon::Revolver => {
                    let needed = 6 - player.clip_revolver;
                    let loaded = needed.min(player.ammo_revolver);
                    player.clip_revolver += loaded;
                    player.ammo_revolver -= loaded;
                }
                ActiveWeapon::Rifle => {
                    let needed = 30 - player.clip_rifle;
                    let loaded = needed.min(player.ammo_rifle);
                    player.clip_rifle += loaded;
                    player.ammo_rifle -= loaded;
                }
                ActiveWeapon::Sniper => {
                    let needed = 5 - player.clip_sniper;
                    let loaded = needed.min(player.ammo_sniper);
                    player.clip_sniper += loaded;
                    player.ammo_sniper -= loaded;
                }
                _ => {}
            }
            player.reload_timer = None;
            inventory_log("✅ Reload complete!");
        } else {
            player.reload_timer = Some(timer);
        }
    }

    // Keyboard trigger for manual reloading
    if keyboard_input.just_pressed(KeyCode::KeyR)
        && player.reload_timer.is_none()
        && !builder.active
    {
        let mut reload_time = 0.0;
        let mut should_reload = false;
        let mut reload_sound = "gun_reload.wav";

        match player.active_weapon {
            ActiveWeapon::Pistol => {
                if player.clip_pistol < 8 && player.ammo_pistol > 0 {
                    reload_time = 1.5;
                    should_reload = true;
                }
            }
            ActiveWeapon::Revolver => {
                if player.clip_revolver < 6 && player.ammo_revolver > 0 {
                    reload_time = 2.0;
                    should_reload = true;
                }
            }
            ActiveWeapon::Rifle => {
                if player.clip_rifle < 30 && player.ammo_rifle > 0 {
                    reload_time = 2.2;
                    should_reload = true;
                }
            }
            ActiveWeapon::Sniper if player.clip_sniper < 5 && player.ammo_sniper > 0 => {
                reload_time = 2.8;
                should_reload = true;
                reload_sound = "sniper_reload.wav";
            }
            _ => {}
        }

        if should_reload {
            player.reload_timer = Some(reload_time);
            commands.spawn((
                AudioPlayer::new(asset_server.load(reload_sound)),
                PlaybackSettings::DESPAWN,
            ));
            inventory_log("⏳ Reloading weapon...");
        }
    }

    // 2. Weapon Firing Logic
    let is_gun = player.active_weapon != ActiveWeapon::Melee;
    let is_cursor_locked = window_query
        .single()
        .is_ok_and(|c| c.grab_mode == CursorGrabMode::Locked);

    if is_gun && is_cursor_locked && player.reload_timer.is_none() && !builder.active {
        let mut try_shoot = false;

        if player.active_weapon == ActiveWeapon::Rifle {
            if mouse_button.pressed(MouseButton::Left) && player.automatic_fire_timer <= 0.0 {
                try_shoot = true;
                player.automatic_fire_timer = 0.12; // 500 RPM
            }
        } else {
            if mouse_button.just_pressed(MouseButton::Left) {
                try_shoot = true;
            }
        }

        if try_shoot {
            let mut has_ammo = false;
            let mut clip_ref = 0;

            match player.active_weapon {
                ActiveWeapon::Pistol => {
                    if player.clip_pistol > 0 {
                        player.clip_pistol -= 1;
                        has_ammo = true;
                    }
                    clip_ref = player.clip_pistol;
                }
                ActiveWeapon::Revolver => {
                    if player.clip_revolver > 0 {
                        player.clip_revolver -= 1;
                        has_ammo = true;
                    }
                    clip_ref = player.clip_revolver;
                }
                ActiveWeapon::Rifle => {
                    if player.clip_rifle > 0 {
                        player.clip_rifle -= 1;
                        has_ammo = true;
                    }
                    clip_ref = player.clip_rifle;
                }
                ActiveWeapon::Sniper => {
                    if player.clip_sniper > 0 {
                        player.clip_sniper -= 1;
                        has_ammo = true;
                    }
                    clip_ref = player.clip_sniper;
                }
                _ => {}
            }

            if has_ammo {
                let Ok((cam_transform, _camera)) = camera_query.single() else {
                    return;
                };
                let start_pos = cam_transform.translation + cam_transform.forward() * 0.5;
                let forward = cam_transform.forward();

                let (bullet_speed, gravity, damage, sound_file) = match player.active_weapon {
                    ActiveWeapon::Pistol => (75.0, 9.8, 12.0, "pistol_shoot.wav"),
                    ActiveWeapon::Revolver => (85.0, 9.8, 25.0, "revolver_shoot.wav"),
                    ActiveWeapon::Rifle => (120.0, 6.0, 18.0, "rifle_shoot.wav"),
                    ActiveWeapon::Sniper => (220.0, 2.5, 95.0, "sniper_shoot.wav"),
                    _ => (0.0, 0.0, 0.0, ""),
                };

                let bullet_vel = forward * bullet_speed;

                let tracer_mesh = meshes.add(Sphere::new(0.06));
                let tracer_mat = materials.add(StandardMaterial {
                    base_color: Color::srgb(1.0, 0.9, 0.2),
                    emissive: LinearRgba::from(Color::srgb(1.0, 0.9, 0.2)) * 6.0,
                    ..default()
                });

                commands.spawn((
                    Mesh3d(tracer_mesh),
                    MeshMaterial3d(tracer_mat),
                    Transform::from_translation(start_pos),
                    Bullet {
                        velocity: bullet_vel,
                        gravity,
                        lifetime: 3.0,
                        damage,
                    },
                    PlayModeEntity,
                ));

                commands.spawn((
                    AudioPlayer::new(asset_server.load(sound_file)),
                    PlaybackSettings::DESPAWN,
                ));

                if clip_ref == 0 {
                    let mut reload_time = 0.0;
                    let mut reload_sound = "gun_reload.wav";
                    let mut do_auto = false;
                    match player.active_weapon {
                        ActiveWeapon::Pistol => {
                            if player.ammo_pistol > 0 {
                                reload_time = 1.5;
                                do_auto = true;
                            }
                        }
                        ActiveWeapon::Revolver => {
                            if player.ammo_revolver > 0 {
                                reload_time = 2.0;
                                do_auto = true;
                            }
                        }
                        ActiveWeapon::Rifle => {
                            if player.ammo_rifle > 0 {
                                reload_time = 2.2;
                                do_auto = true;
                            }
                        }
                        ActiveWeapon::Sniper if player.ammo_sniper > 0 => {
                            reload_time = 2.8;
                            do_auto = true;
                            reload_sound = "sniper_reload.wav";
                        }
                        _ => {}
                    }
                    if do_auto {
                        player.reload_timer = Some(reload_time);
                        commands.spawn((
                            AudioPlayer::new(asset_server.load(reload_sound)),
                            PlaybackSettings::DESPAWN,
                        ));
                    }
                }
            } else {
                commands.spawn((
                    AudioPlayer::new(asset_server.load("gun_reload.wav")),
                    PlaybackSettings {
                        speed: 2.2,
                        ..PlaybackSettings::DESPAWN
                    },
                ));
            }
        }
    }

    // 3. Bullet Physics and Collision Checks
    for (bullet_entity, mut bullet, mut transform) in bullet_query.iter_mut() {
        bullet.lifetime -= dt;
        if bullet.lifetime <= 0.0 {
            commands.entity(bullet_entity).despawn();
            continue;
        }

        let old_pos = transform.translation;
        bullet.velocity.y -= bullet.gravity * dt;
        let new_pos = old_pos + bullet.velocity * dt;
        transform.translation = new_pos;

        let terrain_y = get_bilinear_height(new_pos.x, new_pos.z, &map);

        if new_pos.y <= terrain_y {
            spawn_bullet_impact_particles(
                &mut commands,
                &mut meshes,
                &mut materials,
                new_pos,
                Color::srgb(0.5, 0.45, 0.4),
            );
            commands.entity(bullet_entity).despawn();
            continue;
        }

        let water_level = water_settings.height;
        if old_pos.y > water_level && new_pos.y <= water_level {
            impulse_writer.write(WaterImpulseEvent {
                position: Vec3::new(new_pos.x, water_level, new_pos.z),
                force: -0.22,
                radius: 1.5,
            });
            spawn_bullet_impact_particles(
                &mut commands,
                &mut meshes,
                &mut materials,
                Vec3::new(new_pos.x, water_level, new_pos.z),
                Color::srgb(0.4, 0.6, 0.95),
            );
            commands.spawn((
                AudioPlayer::new(asset_server.load("water_splash.ogg")),
                PlaybackSettings {
                    volume: bevy::audio::Volume::Linear(0.6),
                    ..PlaybackSettings::DESPAWN
                },
            ));
            commands.entity(bullet_entity).despawn();
            continue;
        }

        let mut hit_info: Option<(Entity, Vec3)> = None;
        for (creature_entity, creature, c_transform, _aggro) in creature_query.iter_mut() {
            if creature.state == creatures::CreatureState::Dead {
                continue;
            }
            // Don't damage the player's own robot defenders
            if creature.creature_type == creatures::CreatureType::RobotTrilobite {
                continue;
            }

            let (center_offset, radius) = match creature.creature_type {
                creatures::CreatureType::Monster => (0.0, 1.4),
                creatures::CreatureType::Bird => (0.0, 0.5),
                creatures::CreatureType::Triangaroo => (0.6, 0.9),
                creatures::CreatureType::Polypug => (0.4, 0.7),
                creatures::CreatureType::Fox => (0.0, 1.4),
                creatures::CreatureType::Alien => (0.3, 1.0),
                creatures::CreatureType::RobotTrilobite => (0.3, 0.8),
            };

            let dist = new_pos.distance(c_transform.translation + Vec3::Y * center_offset);
            if dist < radius {
                hit_info = Some((creature_entity, c_transform.translation));
                break;
            }
        }

        if let Some((c_entity, c_pos)) = hit_info
            && let Ok((_, mut creature, _, mut aggro_opt)) = creature_query.get_mut(c_entity)
        {
            creature.health = (creature.health - bullet.damage).max(0.0);
            let bullet_dir = bullet.velocity.normalize_or_zero();
            creature.velocity += bullet_dir * (bullet.damage * 0.4);

            // Provoke aliens when shot
            if creature.creature_type == creatures::CreatureType::Alien
                && let Some(ref mut aggro) = aggro_opt
            {
                aggro.is_provoked = true;
                aggro.aggro_timer = 10.0;
            }

            spawn_bullet_impact_particles(
                &mut commands,
                &mut meshes,
                &mut materials,
                new_pos,
                Color::srgb(1.0, 0.15, 0.15),
            );

            if creature.health <= 0.0 {
                creature.state = creatures::CreatureState::Dead;
                creature.death_timer = 0.0;

                // Spawn a physical ammo and loot drop box!
                let drop_pos = Vec3::new(c_pos.x, c_pos.y + 0.3, c_pos.z);

                let wood_loot = 1 + (rand::random::<f32>() * 3.0) as u32; // 1 to 3
                let copper_loot = (rand::random::<f32>() * 3.0) as u32; // 0 to 2
                let iron_loot = (rand::random::<f32>() * 2.0) as u32; // 0 to 1

                commands.spawn((
                    WorldAssetRoot(asset_server.load("Gun_Sniper_Ammo.gltf#Scene0")),
                    Transform::from_translation(drop_pos).with_scale(Vec3::splat(1.8)),
                    AmmoDrop {
                        ammo_pistol: 12,
                        ammo_revolver: 6,
                        ammo_rifle: 30,
                        ammo_sniper: 5,
                        wood: wood_loot,
                        copper: copper_loot,
                        iron: iron_loot,
                    },
                    SpinDrop,
                    PlayModeEntity,
                ));

                inventory_log("💀 Struck creature down! Ammo Drop Spawned!");
            } else {
                inventory_log(&format!(
                    "💥 Hit creature! HP remaining: {}/{}",
                    creature.health, creature.max_health
                ));
            }

            commands.entity(bullet_entity).despawn();
        }
    }
}

fn spawn_bullet_impact_particles(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    pos: Vec3,
    color: Color,
) {
    let p_mesh = meshes.add(Sphere::new(0.04));
    let p_mat = materials.add(StandardMaterial {
        base_color: color,
        emissive: if color.to_linear().red > 0.8 {
            LinearRgba::from(Color::srgb(1.0, 0.2, 0.2)) * 1.5
        } else {
            LinearRgba::BLACK
        },
        ..default()
    });

    for i in 0..6 {
        let angle = (i as f32) * 1.04;
        let p_vel = Vec3::new(
            angle.cos() * 2.0,
            3.0 + rand::random::<f32>() * 2.0,
            angle.sin() * 2.0,
        );
        commands.spawn((
            Mesh3d(p_mesh.clone()),
            MeshMaterial3d(p_mat.clone()),
            Transform::from_translation(pos),
            PlayModeEntity,
            PlayModeParticle {
                velocity: p_vel,
                lifetime: 0.45,
            },
        ));
    }
}

#[derive(Component)]
pub struct PlayModeParticle {
    pub velocity: Vec3,
    pub lifetime: f32,
}

fn play_particle_update_system(
    mut commands: Commands,
    time: Res<Time>,
    mut particle_query: Query<(Entity, &mut PlayModeParticle, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (entity, mut particle, mut transform) in particle_query.iter_mut() {
        particle.lifetime -= dt;
        if particle.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        particle.velocity.y -= 9.8 * dt;
        transform.translation += particle.velocity * dt;
    }
}

fn update_drops_system(
    mut commands: Commands,
    time: Res<Time>,
    mut drop_query: Query<(Entity, &AmmoDrop, &mut Transform), With<SpinDrop>>,
    mut player_query: Query<(&mut PlayModePlayer, &Transform), Without<SpinDrop>>,
    mut inventory: ResMut<PlayerInventory>,
    asset_server: Res<AssetServer>,
    children_query: Query<&Children>,
) {
    let Ok((mut player, player_transform)) = player_query.single_mut() else {
        return;
    };
    let player_pos = player_transform.translation;
    let dt = time.delta_secs();

    for (entity, drop, mut transform) in drop_query.iter_mut() {
        // Spin the drop visually
        transform.rotation = Quat::from_rotation_y(dt * 1.5) * transform.rotation;
        // Bob up and down slightly using a sine wave
        transform.translation.y += (time.elapsed_secs() * 3.0).sin() * 0.003;

        let dist = player_pos.distance(transform.translation);
        if dist < 1.8 {
            // Automatically pick up!
            player.ammo_pistol += drop.ammo_pistol;
            player.ammo_revolver += drop.ammo_revolver;
            player.ammo_rifle += drop.ammo_rifle;
            player.ammo_sniper += drop.ammo_sniper;

            inventory.wood += drop.wood;
            inventory.copper += drop.copper;
            inventory.iron += drop.iron;

            // Log pickup to HUD
            let mut items = vec![];
            if drop.ammo_pistol > 0 {
                items.push(format!("+{} Pistol Ammo", drop.ammo_pistol));
            }
            if drop.ammo_revolver > 0 {
                items.push(format!("+{} Revolver Ammo", drop.ammo_revolver));
            }
            if drop.ammo_rifle > 0 {
                items.push(format!("+{} Rifle Ammo", drop.ammo_rifle));
            }
            if drop.ammo_sniper > 0 {
                items.push(format!("+{} Sniper Ammo", drop.ammo_sniper));
            }
            if drop.wood > 0 {
                items.push(format!("+{} Wood", drop.wood));
            }
            if drop.copper > 0 {
                items.push(format!("+{} Copper", drop.copper));
            }
            if drop.iron > 0 {
                items.push(format!("+{} Iron", drop.iron));
            }

            inventory_log(&format!("🎒 Acquired Loot Drop: {}", items.join(", ")));

            // Play satisfying pickup sound
            commands.spawn((
                AudioPlayer::new(asset_server.load("chest_open.wav")),
                PlaybackSettings::DESPAWN,
            ));

            if let Ok(children) = children_query.get(entity) {
                for child in children.iter() {
                    commands.entity(child).despawn();
                }
            }
            commands.entity(entity).despawn();
        }
    }
}
