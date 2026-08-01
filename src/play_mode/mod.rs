use crate::AppState;
use crate::character_designer::{
    CharacterSettings, build_skeletal_limb_mesh, build_stylized_bone_mesh,
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

pub mod cave;
pub mod creatures;
pub mod house;
pub mod structures;

pub struct PlayModePlugin;

impl Plugin for PlayModePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerInventory>()
            .init_resource::<creatures::CreatureRespawnTimer>()
            .init_resource::<structures::BuildingPlacementState>()
            .add_plugins(house::HousePlugin)
            .add_plugins(cave::CavePlugin)
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
                    sync_mansion_global_bounds_system,
                    player_movement_and_ragdoll_system,
                    axe_swing_system,
                    play_visual_sync_system,
                    play_weapon_sync_system,
                    player_armor_sync_system,
                    weapon_attachment_system,
                    particle_update_system,
                )
                    .run_if(in_state(AppState::PlayMode)),
            )
            .add_systems(
                Update,
                (
                    sync_logs,
                    creatures::creature_ai_system,
                    creatures::creature_animation_sync_system,
                    creatures::creature_skeletal_animation_system,
                    gun_fire_and_bullet_system,
                    play_particle_update_system,
                    crate::map_editor::configure_terrain_sampler_system,
                    update_drops_system,
                )
                    .run_if(in_state(AppState::PlayMode)),
            )
            .add_systems(
                Update,
                (
                    poll_terrain_load_system,
                    creatures::attach_fox_animation_player,
                    creatures::drive_fox_animations,
                    creatures::attach_trilobite_animation_player,
                    creatures::drive_trilobite_animations,
                    creatures::spawn_defender_trilobite,
                    creatures::trilobite_combat_system,
                    creatures::tamed_fox_combat_system,
                    creatures::fox_taming_interaction_system,
                    creatures::spawn_saved_tamed_foxes,
                    creatures::creature_respawn_system,
                    add_physics_to_wall_colliders,
                    play_mode_mouse_grab_system,
                    cloud_drift_system,
                    play_sky_cycle_system,
                    starship_plasma_bolt_system,
                    starship_visual_sync_system,
                    crate::map_editor::water_simulation_system,
                    building_placement_system,
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

#[derive(Component, Default)]
pub struct AmmoDrop {
    pub ammo_pistol: u32,
    pub ammo_revolver: u32,
    pub ammo_rifle: u32,
    pub ammo_sniper: u32,
    pub wood: u32,
    pub copper: u32,
    pub iron: u32,
    pub health_heal: f32,
    pub fox_pelt: u32,
    pub alien_pelt: u32,
    pub kangaroo_fur: u32,
    pub alien_feather: u32,
    pub monster_core: u32,
    pub alien_tech: u32,
    pub robot_parts: u32,
}

#[derive(Component)]
pub struct SpinDrop;

#[derive(Component)]
pub struct CrashedStarshipConsoleMarker;

#[derive(Component)]
pub struct StarshipDebris;

#[derive(Component)]
pub struct StarshipBrokenWing;

#[derive(Component)]
pub struct StarshipRepairedWing;

#[derive(Component)]
#[allow(dead_code)]
pub struct CrashedStarship {
    pub is_repaired: bool,
    pub flight_speed: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
}

#[derive(Component)]
pub struct StarshipPlasmaBolt {
    pub velocity: Vec3,
    pub lifetime: f32,
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AxeTier {
    #[default]
    Wood,
    Copper,
    Steel,
    Gold,
    Platinum,
}

impl AxeTier {
    pub fn name(&self) -> &'static str {
        match self {
            AxeTier::Wood => "🪓 Wood Axe",
            AxeTier::Copper => "🟧 Copper Axe",
            AxeTier::Steel => "⚙ Steel Battleaxe",
            AxeTier::Gold => "👑 Golden Waraxe",
            AxeTier::Platinum => "💎 Platinum Excalibur Axe",
        }
    }
    pub fn damage_multiplier(&self) -> u32 {
        match self {
            AxeTier::Wood => 1,
            AxeTier::Copper => 2,
            AxeTier::Steel => 3,
            AxeTier::Gold => 4,
            AxeTier::Platinum => 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ArmorTier {
    #[default]
    None,
    Leather,
    Copper,
    Steel,
    Platinum,
    FlightSuit,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SaveData {
    pub wood: u32,
    pub rock: u32,
    pub copper: u32,
    pub iron: u32,
    pub gold: u32,
    pub silver: u32,
    pub platinum: u32,
    pub granite: u32,
    pub steel: u32,
    pub fox_pelt: u32,
    pub alien_pelt: u32,
    pub kangaroo_fur: u32,
    pub alien_feather: u32,
    pub monster_core: u32,
    pub alien_tech: u32,
    pub robot_parts: u32,
    pub crystal_shard: u32,
    pub ship_repair_steel: u32,
    pub ship_repair_platinum: u32,
    pub ship_repair_crystals: u32,
    pub ship_repair_robot_parts: u32,
    pub ship_repair_alien_tech: u32,
    pub starship_repaired: bool,
    pub equipped_axe: AxeTier,
    pub equipped_armor: ArmorTier,
    pub has_flight_suit: bool,
    pub wooden_shelter_parts: u32,
    pub metal_shelter_parts: u32,
    pub has_sword: bool,
    pub has_leather_armor: bool,
    pub has_recall_beacon: bool,
    #[serde(default)]
    pub tamed_fox_count: u32,
    pub player_pos: [f32; 3],
    pub health: f32,
    pub max_health: f32,
    pub ammo_pistol: u32,
    pub ammo_revolver: u32,
    pub ammo_rifle: u32,
    pub ammo_sniper: u32,
    pub clip_pistol: u32,
    pub clip_revolver: u32,
    pub clip_rifle: u32,
    pub clip_sniper: u32,
    pub health_packs: u32,
    pub outfit_style: crate::character_designer::OutfitStyle,
    pub gender: crate::character_designer::Gender,
    pub height: f32,
    pub weight: f32,
    pub hair_style: crate::character_designer::HairStyle,
}

pub fn save_progress(
    inventory: &PlayerInventory,
    player: &PlayModePlayer,
    char_settings: &CharacterSettings,
) -> Result<(), String> {
    let data = SaveData {
        wood: inventory.wood,
        rock: inventory.rock,
        copper: inventory.copper,
        iron: inventory.iron,
        gold: inventory.gold,
        silver: inventory.silver,
        platinum: inventory.platinum,
        granite: inventory.granite,
        steel: inventory.steel,
        fox_pelt: inventory.fox_pelt,
        alien_pelt: inventory.alien_pelt,
        kangaroo_fur: inventory.kangaroo_fur,
        alien_feather: inventory.alien_feather,
        monster_core: inventory.monster_core,
        alien_tech: inventory.alien_tech,
        robot_parts: inventory.robot_parts,
        crystal_shard: inventory.crystal_shard,
        ship_repair_steel: inventory.ship_repair_steel,
        ship_repair_platinum: inventory.ship_repair_platinum,
        ship_repair_crystals: inventory.ship_repair_crystals,
        ship_repair_robot_parts: inventory.ship_repair_robot_parts,
        ship_repair_alien_tech: inventory.ship_repair_alien_tech,
        starship_repaired: inventory.starship_repaired,
        equipped_axe: inventory.equipped_axe,
        equipped_armor: inventory.equipped_armor,
        has_flight_suit: inventory.has_flight_suit,
        wooden_shelter_parts: inventory.wooden_shelter_parts,
        metal_shelter_parts: inventory.metal_shelter_parts,
        has_sword: inventory.has_sword,
        has_leather_armor: inventory.has_leather_armor,
        has_recall_beacon: inventory.has_recall_beacon,
        tamed_fox_count: inventory.tamed_fox_count,
        player_pos: [player.position.x, player.position.y, player.position.z],
        health: player.health,
        max_health: player.max_health,
        ammo_pistol: player.ammo_pistol,
        ammo_revolver: player.ammo_revolver,
        ammo_rifle: player.ammo_rifle,
        ammo_sniper: player.ammo_sniper,
        clip_pistol: player.clip_pistol,
        clip_revolver: player.clip_revolver,
        clip_rifle: player.clip_rifle,
        clip_sniper: player.clip_sniper,
        health_packs: player.health_packs,
        outfit_style: char_settings.outfit_style,
        gender: char_settings.gender,
        height: char_settings.height,
        weight: char_settings.weight,
        hair_style: char_settings.hair_style,
    };

    let json = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
    std::fs::write("save_game.json", json).map_err(|e| e.to_string())?;
    inventory_log("💾 Game Progress Saved to 'save_game.json'!");
    Ok(())
}

pub fn load_progress(
    inventory: &mut PlayerInventory,
    player: &mut PlayModePlayer,
    char_settings: &mut CharacterSettings,
) -> Result<(), String> {
    let json = std::fs::read_to_string("save_game.json").map_err(|e| e.to_string())?;
    let data: SaveData = serde_json::from_str(&json).map_err(|e| e.to_string())?;

    inventory.wood = data.wood;
    inventory.rock = data.rock;
    inventory.copper = data.copper;
    inventory.iron = data.iron;
    inventory.gold = data.gold;
    inventory.silver = data.silver;
    inventory.platinum = data.platinum;
    inventory.granite = data.granite;
    inventory.steel = data.steel;
    inventory.fox_pelt = data.fox_pelt;
    inventory.alien_pelt = data.alien_pelt;
    inventory.kangaroo_fur = data.kangaroo_fur;
    inventory.alien_feather = data.alien_feather;
    inventory.monster_core = data.monster_core;
    inventory.alien_tech = data.alien_tech;
    inventory.robot_parts = data.robot_parts;
    inventory.crystal_shard = data.crystal_shard;
    inventory.ship_repair_steel = data.ship_repair_steel;
    inventory.ship_repair_platinum = data.ship_repair_platinum;
    inventory.ship_repair_crystals = data.ship_repair_crystals;
    inventory.ship_repair_robot_parts = data.ship_repair_robot_parts;
    inventory.ship_repair_alien_tech = data.ship_repair_alien_tech;
    inventory.starship_repaired = data.starship_repaired;
    inventory.tamed_fox_count = data.tamed_fox_count;
    inventory.equipped_axe = data.equipped_axe;
    inventory.equipped_armor = data.equipped_armor;
    inventory.has_flight_suit = data.has_flight_suit;
    inventory.wooden_shelter_parts = data.wooden_shelter_parts;
    inventory.metal_shelter_parts = data.metal_shelter_parts;
    inventory.has_sword = data.has_sword;
    inventory.has_leather_armor = data.has_leather_armor;
    inventory.has_recall_beacon = data.has_recall_beacon;

    player.position = Vec3::from_array(data.player_pos);
    player.health = data.health;
    player.max_health = data.max_health;
    player.ammo_pistol = data.ammo_pistol;
    player.ammo_revolver = data.ammo_revolver;
    player.ammo_rifle = data.ammo_rifle;
    player.ammo_sniper = data.ammo_sniper;
    player.clip_pistol = data.clip_pistol;
    player.clip_revolver = data.clip_revolver;
    player.clip_rifle = data.clip_rifle;
    player.clip_sniper = data.clip_sniper;
    player.health_packs = data.health_packs;

    char_settings.outfit_style = data.outfit_style;
    char_settings.gender = data.gender;
    char_settings.height = data.height;
    char_settings.weight = data.weight;
    char_settings.hair_style = data.hair_style;

    inventory_log("📂 Game Progress Loaded successfully!");
    Ok(())
}

impl ArmorTier {
    pub fn name(&self) -> &'static str {
        match self {
            ArmorTier::None => "No Armor",
            ArmorTier::Leather => "🛡️ Leather Armor (-25%)",
            ArmorTier::Copper => "🟧 Copper Plated Armor (-35%)",
            ArmorTier::Steel => "⚙ Steel Plate Armor (-50%)",
            ArmorTier::Platinum => "💎 Platinum Mesh Armor (-65%)",
            ArmorTier::FlightSuit => "🚀 Cyber Flight Suit (-80% + Flight)",
        }
    }
    pub fn damage_multiplier(&self) -> f32 {
        match self {
            ArmorTier::None => 1.0,
            ArmorTier::Leather => 0.75,
            ArmorTier::Copper => 0.65,
            ArmorTier::Steel => 0.50,
            ArmorTier::Platinum => 0.35,
            ArmorTier::FlightSuit => 0.20,
        }
    }
}

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
    // Creature Drops
    pub fox_pelt: u32,
    pub alien_pelt: u32,
    pub kangaroo_fur: u32,
    pub alien_feather: u32,
    pub monster_core: u32,
    pub alien_tech: u32,
    pub robot_parts: u32,
    pub crystal_shard: u32,
    // Starship Repair Subsystems & Status
    pub show_ship_repair_window: bool,
    pub ship_repair_steel: u32,
    pub ship_repair_platinum: u32,
    pub ship_repair_crystals: u32,
    pub ship_repair_robot_parts: u32,
    pub ship_repair_alien_tech: u32,
    pub starship_repaired: bool,
    // Equipment & Tiers
    pub equipped_axe: AxeTier,
    pub equipped_armor: ArmorTier,
    pub has_flight_suit: bool,
    // Crafting outputs & buffs
    pub wooden_shelter_parts: u32,
    pub metal_shelter_parts: u32,
    pub has_sword: bool,
    pub has_leather_armor: bool,
    pub has_recall_beacon: bool,
    pub shield_timer: f32,
    pub show_alien_store: bool,
    pub tamed_fox_count: u32,
    pub loot_log: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerState {
    Active,
    Ragdoll,
    Swimming,
    Flying,
    PilotingStarship,
    Climbing,
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
    pub health_packs: u32,
    pub is_headlamp_on: bool,
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

#[derive(Component)]
pub struct PlayArmorVisual {
    pub armor_tier: ArmorTier,
}

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
pub struct PlayNightPlanet;

#[derive(Component)]
pub struct PlayPlanetRings;

#[derive(Component)]
pub struct PlayBlackHoleMoon;

#[derive(Component)]
pub struct PlayBlackHoleDiskHoriz;

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

fn spawn_modular_block_colliders(
    commands: &mut Commands,
    prefab_type: &str,
    custom_mesh: Option<&crate::map_editor::data::EditableMesh>,
    parent: Entity,
) {
    match prefab_type {
        "floor_tile" => {
            let child = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(4.0, 0.2, 4.0),
                    Transform::from_xyz(0.0, 0.1, 0.0),
                    PlayModeEntity,
                ))
                .id();
            commands.entity(parent).add_child(child);
        }
        "ceiling_tile" => {
            let child = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(4.0, 0.15, 4.0),
                    Transform::from_xyz(0.0, 0.0, 0.0),
                    PlayModeEntity,
                ))
                .id();
            commands.entity(parent).add_child(child);
        }
        "wall_straight" => {
            let child = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(4.0, 3.5, 0.2),
                    Transform::from_xyz(0.0, 1.75, 0.0),
                    PlayModeEntity,
                ))
                .id();
            commands.entity(parent).add_child(child);
        }
        "wall_corner" => {
            let child1 = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(4.0, 3.5, 0.2),
                    Transform::from_xyz(0.0, 1.75, -0.1),
                    PlayModeEntity,
                ))
                .id();
            let child2 = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(0.2, 3.5, 4.0),
                    Transform::from_xyz(-0.1, 1.75, 0.0),
                    PlayModeEntity,
                ))
                .id();
            commands.entity(parent).add_child(child1).add_child(child2);
        }
        "wall_t_junction" => {
            let child1 = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(4.0, 3.5, 0.2),
                    Transform::from_xyz(0.0, 1.75, 0.0),
                    PlayModeEntity,
                ))
                .id();
            let child2 = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(0.2, 3.5, 2.0),
                    Transform::from_xyz(0.0, 1.75, 1.0),
                    PlayModeEntity,
                ))
                .id();
            commands.entity(parent).add_child(child1).add_child(child2);
        }
        "wall_cross" => {
            let child1 = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(4.0, 3.5, 0.2),
                    Transform::from_xyz(0.0, 1.75, 0.0),
                    PlayModeEntity,
                ))
                .id();
            let child2 = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(0.2, 3.5, 4.0),
                    Transform::from_xyz(0.0, 1.75, 0.0),
                    PlayModeEntity,
                ))
                .id();
            commands.entity(parent).add_child(child1).add_child(child2);
        }
        "door_tile" | "door_frame" => {
            let child1 = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(1.2, 3.5, 0.2),
                    Transform::from_xyz(-1.4, 1.75, 0.0),
                    PlayModeEntity,
                ))
                .id();
            let child2 = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(1.2, 3.5, 0.2),
                    Transform::from_xyz(1.4, 1.75, 0.0),
                    PlayModeEntity,
                ))
                .id();
            let child3 = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(1.6, 1.1, 0.2),
                    Transform::from_xyz(0.0, 2.95, 0.0),
                    PlayModeEntity,
                ))
                .id();
            let door_panel = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(1.6, 2.4, 0.1),
                    Transform::from_xyz(0.0, 1.2, 0.0),
                    PlayModeEntity,
                ))
                .id();
            commands
                .entity(parent)
                .add_child(child1)
                .add_child(child2)
                .add_child(child3)
                .add_child(door_panel);
        }
        "window_tile" | "window_frame" => {
            let child1 = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(4.0, 1.0, 0.2),
                    Transform::from_xyz(0.0, 0.5, 0.0),
                    PlayModeEntity,
                ))
                .id();
            let child2 = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(4.0, 1.0, 0.2),
                    Transform::from_xyz(0.0, 3.0, 0.0),
                    PlayModeEntity,
                ))
                .id();
            let child3 = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(0.4, 1.5, 0.2),
                    Transform::from_xyz(-1.8, 1.75, 0.0),
                    PlayModeEntity,
                ))
                .id();
            let child4 = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(0.4, 1.5, 0.2),
                    Transform::from_xyz(1.8, 1.75, 0.0),
                    PlayModeEntity,
                ))
                .id();
            commands
                .entity(parent)
                .add_child(child1)
                .add_child(child2)
                .add_child(child3)
                .add_child(child4);
        }
        "roof_tile" => {
            let child = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(4.0, 0.2, 4.3),
                    Transform::from_xyz(0.0, 1.2, 0.0)
                        .with_rotation(Quat::from_rotation_x(35.0f32.to_radians())),
                    PlayModeEntity,
                ))
                .id();
            commands.entity(parent).add_child(child);
        }
        "roof_gable" => {
            let child = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(4.0, 2.35, 0.2),
                    Transform::from_xyz(0.0, 1.175, 0.0),
                    PlayModeEntity,
                ))
                .id();
            commands.entity(parent).add_child(child);
        }
        "hallway_segment" => {
            let child1 = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(4.0, 0.2, 8.0),
                    Transform::from_xyz(0.0, 0.1, 0.0),
                    PlayModeEntity,
                ))
                .id();
            let child2 = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(0.2, 3.5, 8.0),
                    Transform::from_xyz(-2.0, 1.75, 0.0),
                    PlayModeEntity,
                ))
                .id();
            let child3 = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(0.2, 3.5, 8.0),
                    Transform::from_xyz(2.0, 1.75, 0.0),
                    PlayModeEntity,
                ))
                .id();
            let child4 = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(4.0, 0.15, 8.0),
                    Transform::from_xyz(0.0, 3.5, 0.0),
                    PlayModeEntity,
                ))
                .id();
            commands
                .entity(parent)
                .add_child(child1)
                .add_child(child2)
                .add_child(child3)
                .add_child(child4);
        }
        "room_pillar" => {
            let child = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(0.5, 3.5, 0.5),
                    Transform::from_xyz(0.0, 1.75, 0.0),
                    PlayModeEntity,
                ))
                .id();
            commands.entity(parent).add_child(child);
        }
        "chest" => {
            let child = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(0.8, 0.6, 0.5),
                    Transform::from_xyz(0.0, 0.3, 0.0),
                    PlayModeEntity,
                ))
                .id();
            commands.entity(parent).add_child(child);
        }
        "workbench" => {
            let child = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(1.2, 0.8, 0.9),
                    Transform::from_xyz(0.0, 0.45, 0.0),
                    PlayModeEntity,
                ))
                .id();
            commands.entity(parent).add_child(child);
        }
        "furnace" => {
            let child = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(0.8, 0.8, 0.9),
                    Transform::from_xyz(0.0, 0.45, 0.0),
                    PlayModeEntity,
                ))
                .id();
            commands.entity(parent).add_child(child);
        }
        "bed" => {
            let child = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(1.2, 0.5, 2.0),
                    Transform::from_xyz(0.0, 0.25, 0.0),
                    PlayModeEntity,
                ))
                .id();
            commands.entity(parent).add_child(child);
        }
        "prop_chair" => {
            let child = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(0.6, 0.9, 0.6),
                    Transform::from_xyz(0.0, 0.45, 0.0),
                    PlayModeEntity,
                ))
                .id();
            commands.entity(parent).add_child(child);
        }
        "prop_desk" => {
            let child = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(1.6, 0.85, 0.8),
                    Transform::from_xyz(0.0, 0.425, 0.0),
                    PlayModeEntity,
                ))
                .id();
            commands.entity(parent).add_child(child);
        }
        "prop_health_pack" => {
            let child = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(0.5, 0.25, 0.4),
                    Transform::from_xyz(0.0, 0.125, 0.0),
                    PlayModeEntity,
                ))
                .id();
            commands.entity(parent).add_child(child);
        }
        "prop_crate" => {
            let child = commands
                .spawn((
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(1.2, 1.2, 1.2),
                    Transform::from_xyz(0.0, 0.6, 0.0),
                    PlayModeEntity,
                ))
                .id();
            commands.entity(parent).add_child(child);
        }
        "custom_mesh" => {
            if let Some(m) = custom_mesh {
                let bevy_mesh = m.to_bevy_mesh();
                if let Some(col) = avian3d::prelude::Collider::trimesh_from_mesh(&bevy_mesh) {
                    commands
                        .entity(parent)
                        .insert((avian3d::prelude::RigidBody::Static, col));
                }
            }
        }
        _ => {}
    }
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
    mut cave_data: ResMut<cave::CaveSystemData>,
) {
    let h_scale = char_settings.height;
    let config_handle = control_configs.add(ControlSchemeConfig {
        basis: bevy_tnua::builtins::TnuaBuiltinWalkConfig {
            speed: 12.0,
            float_height: h_scale * 0.5 + 0.08,
            max_slope: 65.0f32.to_radians(),
            cling_distance: 1.8,
            acceleration: 90.0,
            air_acceleration: 45.0,
            turning_angvel: 16.0,
            ..default()
        },
        jump: bevy_tnua::builtins::TnuaBuiltinJumpConfig {
            height: 3.5,
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
    use bevy::light::*;
    use bevy::pbr::*;
    commands.insert_resource(ClearColor(Color::srgb(0.08, 0.05, 0.14)));

    // 2. Golden Sun Sphere & Light with PlaySun component (Distant skybox position)
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(120.0).mesh().ico(3).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.8, 0.6),
            emissive: LinearRgba::from(Color::srgb(10.0, 8.0, 6.0)),
            unlit: true,
            fog_enabled: false,
            ..default()
        })),
        Transform::from_xyz(2500.0, 2000.0, 1500.0),
        NotShadowCaster,
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
            shadow_depth_bias: 0.02,
            shadow_normal_bias: 1.8,
            illuminance: 9500.0,
            color: Color::srgb(1.0, 0.85, 0.65),
            shadow_maps_enabled: true,
            ..default()
        },
        CascadeShadowConfigBuilder {
            maximum_distance: 2500.0,
            ..default()
        }
        .build(),
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

    // 3. Cyan Sun Sphere & Light with PlaySun component (Distant skybox position, close binary partner)
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(80.0).mesh().ico(3).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.4, 0.9, 1.0),
            emissive: LinearRgba::from(Color::srgb(4.0, 9.0, 10.0)),
            unlit: true,
            fog_enabled: false,
            ..default()
        })),
        Transform::from_xyz(-2500.0, 1800.0, -1800.0),
        NotShadowCaster,
        PlaySun {
            id: 1,
            angle_offset: 0.25,
            orbit_speed: 1.0,
            base_color: Color::srgb(0.4, 0.9, 1.0),
            day_intensity: 6500.0,
        },
        PlayModeEntity,
    ));

    commands.spawn((
        DirectionalLight {
            shadow_depth_bias: 0.02,
            shadow_normal_bias: 1.8,
            illuminance: 6500.0,
            color: Color::srgb(0.45, 0.92, 1.0),
            shadow_maps_enabled: true,
            ..default()
        },
        CascadeShadowConfigBuilder {
            maximum_distance: 2500.0,
            ..default()
        }
        .build(),
        Transform::from_xyz(-70.0, 45.0, -50.0).looking_at(Vec3::ZERO, Vec3::Y),
        PlaySun {
            id: 1,
            angle_offset: 0.25,
            orbit_speed: 1.0,
            base_color: Color::srgb(0.4, 0.9, 1.0),
            day_intensity: 6500.0,
        },
        PlayModeEntity,
    ));

    // 4. Large night sky gas giant planet (Neptune/Uranus Cyan-Blue)
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(100.0).mesh().ico(4).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.15, 0.45, 0.90), // Neptune deep cyan-blue
            emissive: LinearRgba::new(0.6, 1.8, 3.8, 1.0), // vibrant atmospheric glow
            unlit: true,
            cull_mode: None,
            fog_enabled: false,
            ..default()
        })),
        Transform::from_xyz(0.0, -3500.0, 0.0),
        NotShadowCaster,
        PlayNightPlanet,
        PlayModeEntity,
    ));

    // Gas Giant Rings (Flat double-sided annulus)
    commands.spawn((
        Mesh3d(meshes.add(Annulus::new(125.0, 195.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.55, 0.72, 0.95, 0.65),
            emissive: LinearRgba::new(1.2, 1.8, 2.5, 1.0),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            cull_mode: None,
            fog_enabled: false,
            ..default()
        })),
        Transform::from_xyz(0.0, -3500.0, 0.0),
        NotShadowCaster,
        PlayPlanetRings,
        PlayModeEntity,
    ));

    // 5. Gravitational Lensing Black Hole Moon (Event Horizon, Accretion Disks, Thin Einstein Ring Halo)
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(100.0).mesh().ico(4).unwrap())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::BLACK,
            unlit: true,
            fog_enabled: false,
            ..default()
        })),
        Transform::from_xyz(0.0, -3500.0, 0.0),
        NotShadowCaster,
        PlayBlackHoleMoon,
        PlayModeEntity,
    ));

    // Accretion Disk - Horizontal (Swirling relativistic particle stream accretion disk)
    let perlin = crate::map_editor::noise::PerlinNoise::new(12345);
    let accretion_image = generate_accretion_disk_texture(&perlin);
    let accretion_handle = images.add(accretion_image);
    commands.spawn((
        Mesh3d(meshes.add(create_accretion_disk_mesh(140.0, 360.0, 128))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(accretion_handle.clone()),
            emissive: LinearRgba::new(16.0, 5.0, 0.5, 1.0),
            emissive_texture: Some(accretion_handle),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            cull_mode: None,
            fog_enabled: false,
            ..default()
        })),
        Transform::from_xyz(0.0, -3500.0, 0.0),
        NotShadowCaster,
        PlayBlackHoleDiskHoriz,
        PlayModeEntity,
    ));

    // 4. Spawn 3D Terrain & Grass (Asynchronously in Background)
    let (tx, rx) = std::sync::mpsc::channel();
    let map_clone = map.clone();
    let tokio_handle = rt.0.clone();

    tokio_handle.spawn(async move {
        let splat_settings = SplatmapSettings::default();
        let terrain_mesh = crate::map_editor::generate_terrain_mesh(&map_clone, &splat_settings);
        let grass_chunks =
            crate::grass::generate_grass_chunks(&map_clone, Some(&splat_settings), None);
        let _ = tx.send((terrain_mesh, grass_chunks));
    });

    commands.insert_resource(TerrainLoadChannel {
        rx: std::sync::Mutex::new(rx),
    });

    // Spawn 3D Terrain (Physical Heightfield Collider)
    let mut heights = vec![vec![0.0; map.width as usize]; map.height as usize];
    for z in 0..map.height {
        for x in 0..map.width {
            heights[z as usize][x as usize] = map.get_height(x, z);
        }
    }
    let heightfield_scale = Vec3::new(map.width as f32 - 1.0, 1.0, map.height as f32 - 1.0);
    commands.spawn((
        avian3d::prelude::RigidBody::Static,
        avian3d::prelude::Collider::heightfield(heights, heightfield_scale),
        Transform::from_xyz(
            -(map.width as f32 - 1.0) * 0.5,
            0.0,
            -(map.height as f32 - 1.0) * 0.5,
        ),
        PlayModeEntity,
    ));

    // 5. Spawn Underground Cave Maze System
    cave::setup_underground_cave_system(
        &mut commands,
        &mut meshes,
        &mut materials,
        &asset_server,
        &map,
        &mut cave_data,
    );

    // 6. Spawn Translucent Interactive Water
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
        base_color: Color::srgb(0.85, 0.85, 0.85),
        perceptual_roughness: 0.95,
        metallic: 0.0,
        reflectance: 0.05,
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

                let terrain_h = map.get_height(x, z);
                let bridge_y = (terrain_h + 0.08).max(1.30);

                commands.spawn((
                    Mesh3d(bridge_mesh.clone()),
                    MeshMaterial3d(bridge_mat.clone()),
                    Transform::from_xyz(vx, bridge_y, vz).with_rotation(rot),
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(2.4, 0.15, 1.05),
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
        NotShadowCaster,
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
    let mut house_pos = Vec3::new(-35.0, 1.5, -35.0);
    for p in map.prefabs.iter() {
        if p.prefab_type == "house" {
            house_pos = Vec3::from_array(p.position);
            break;
        }
    }

    let half_w = (mansion_settings.cols as f32 * mansion_settings.cell_size) / 2.0;
    let half_d = (mansion_settings.rows as f32 * mansion_settings.cell_size) / 2.0;

    // 5. Spawn all Placed Prefabs (Resource Nodes and Modular/Custom blocks)
    for (idx, p) in map.prefabs.iter().enumerate() {
        if p.prefab_type == "spawn_point"
            || p.prefab_type == "house"
            || p.prefab_type == "cave_entrance"
        {
            continue;
        }

        let p_pos = Vec3::from_array(p.position);

        // Skip spawning if it overlaps the house footprint
        let inside = (p_pos.x - house_pos.x).abs() < half_w + 1.0
            && (p_pos.z - house_pos.z).abs() < half_d + 1.0;
        if inside {
            continue;
        }

        let is_mod_or_custom = matches!(
            p.prefab_type.as_str(),
            "floor_tile"
                | "wall_straight"
                | "wall_corner"
                | "door_tile"
                | "door_frame"
                | "window_tile"
                | "window_frame"
                | "roof_tile"
                | "roof_gable"
                | "wall_t_junction"
                | "wall_cross"
                | "ceiling_tile"
                | "hallway_segment"
                | "room_pillar"
                | "custom_mesh"
                | "custom_asset"
                | "chest"
                | "workbench"
                | "furnace"
                | "bed"
                | "torch"
                | "fluorescent_light"
                | "prop_chair"
                | "prop_desk"
                | "prop_health_pack"
                | "prop_crate"
        ) || p.prefab_type.starts_with("custom:");

        if is_mod_or_custom {
            // Spawn modular building block / custom mesh / custom asset with full 3D rotation and scale
            let rot = Quat::from_array(p.rotation);
            let scale = Vec3::from_array(p.scale);
            let parent = commands
                .spawn((
                    Transform::from_translation(p_pos)
                        .with_rotation(rot)
                        .with_scale(scale),
                    Visibility::Visible,
                    InheritedVisibility::default(),
                    PlayModeEntity,
                ))
                .id();

            // Spawn visual children using the map editor's public function
            crate::map_editor::spawn_prefab_visuals_children(
                &mut commands,
                &mut meshes,
                &mut materials,
                &p.prefab_type,
                p_pos,
                p.texture_override.as_deref(),
                &mansion_settings,
                parent,
                &asset_server,
                p.custom_mesh.as_ref(),
            );

            // Spawn physical colliders
            spawn_modular_block_colliders(
                &mut commands,
                &p.prefab_type,
                p.custom_mesh.as_ref(),
                parent,
            );
        } else {
            // Natural resource node
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
    }

    // 6. Build customized Player model
    let h_scale = char_settings.height;
    let w_thick = char_settings.weight;
    let head_scale = char_settings.head_scale;
    let sh_w = char_settings.shoulder_width;
    let leg_len = char_settings.leg_length;
    let waist = char_settings.waist_width;

    let pelvis_y = h_scale * 0.45 * (2.0 - leg_len);
    let spine_y = pelvis_y + (h_scale * 0.15);
    let chest_y = pelvis_y + (h_scale * 0.3);
    let head_y = chest_y + (h_scale * 0.18);
    let knee_y = pelvis_y * 0.5;

    // Build Verlet node list similar to character designer structure
    let nodes = vec![
        PlayVerletNode {
            name: "Pelvis".to_string(),
            position: spawn_pos + Vec3::new(0.0, pelvis_y, 0.0),
            old_position: spawn_pos + Vec3::new(0.0, pelvis_y, 0.0),
            radius: 0.15 * w_thick,
            start_local: Vec3::new(0.0, pelvis_y, 0.0),
        },
        PlayVerletNode {
            name: "Spine".to_string(),
            position: spawn_pos + Vec3::new(0.0, spine_y, 0.0),
            old_position: spawn_pos + Vec3::new(0.0, spine_y, 0.0),
            radius: 0.16 * w_thick,
            start_local: Vec3::new(0.0, spine_y, 0.0),
        },
        PlayVerletNode {
            name: "Chest".to_string(),
            position: spawn_pos + Vec3::new(0.0, chest_y, 0.0),
            old_position: spawn_pos + Vec3::new(0.0, chest_y, 0.0),
            radius: 0.18 * w_thick,
            start_local: Vec3::new(0.0, chest_y, 0.0),
        },
        PlayVerletNode {
            name: "Head".to_string(),
            position: spawn_pos + Vec3::new(0.0, head_y, 0.0),
            old_position: spawn_pos + Vec3::new(0.0, head_y, 0.0),
            radius: 0.14 * head_scale,
            start_local: Vec3::new(0.0, head_y, 0.0),
        },
        PlayVerletNode {
            name: "L_Shoulder".to_string(),
            position: spawn_pos + Vec3::new(-0.25 * w_thick * sh_w, chest_y, 0.0),
            old_position: spawn_pos + Vec3::new(-0.25 * w_thick * sh_w, chest_y, 0.0),
            radius: 0.08 * w_thick,
            start_local: Vec3::new(-0.25 * w_thick * sh_w, chest_y, 0.0),
        },
        PlayVerletNode {
            name: "L_Elbow".to_string(),
            position: spawn_pos + Vec3::new(-0.5 * w_thick * sh_w, chest_y, 0.0),
            old_position: spawn_pos + Vec3::new(-0.5 * w_thick * sh_w, chest_y, 0.0),
            radius: 0.07 * w_thick,
            start_local: Vec3::new(-0.5 * w_thick * sh_w, chest_y, 0.0),
        },
        PlayVerletNode {
            name: "R_Shoulder".to_string(),
            position: spawn_pos + Vec3::new(0.25 * w_thick * sh_w, chest_y, 0.0),
            old_position: spawn_pos + Vec3::new(0.25 * w_thick * sh_w, chest_y, 0.0),
            radius: 0.08 * w_thick,
            start_local: Vec3::new(0.25 * w_thick * sh_w, chest_y, 0.0),
        },
        PlayVerletNode {
            name: "R_Elbow".to_string(),
            position: spawn_pos + Vec3::new(0.5 * w_thick * sh_w, chest_y, 0.0),
            old_position: spawn_pos + Vec3::new(0.5 * w_thick * sh_w, chest_y, 0.0),
            radius: 0.07 * w_thick,
            start_local: Vec3::new(0.5 * w_thick * sh_w, chest_y, 0.0),
        },
        PlayVerletNode {
            name: "L_Hip".to_string(),
            position: spawn_pos + Vec3::new(-0.16 * w_thick * waist, pelvis_y, 0.0),
            old_position: spawn_pos + Vec3::new(-0.16 * w_thick * waist, pelvis_y, 0.0),
            radius: 0.1 * w_thick,
            start_local: Vec3::new(-0.16 * w_thick * waist, pelvis_y, 0.0),
        },
        PlayVerletNode {
            name: "L_Knee".to_string(),
            position: spawn_pos + Vec3::new(-0.16 * w_thick * waist, knee_y, 0.0),
            old_position: spawn_pos + Vec3::new(-0.16 * w_thick * waist, knee_y, 0.0),
            radius: 0.09 * w_thick,
            start_local: Vec3::new(-0.16 * w_thick * waist, knee_y, 0.0),
        },
        PlayVerletNode {
            name: "L_Foot".to_string(),
            position: spawn_pos + Vec3::new(-0.16 * w_thick * waist, 0.0, 0.0),
            old_position: spawn_pos + Vec3::new(-0.16 * w_thick * waist, 0.0, 0.0),
            radius: 0.08 * w_thick,
            start_local: Vec3::new(-0.16 * w_thick * waist, 0.0, 0.0),
        },
        PlayVerletNode {
            name: "R_Hip".to_string(),
            position: spawn_pos + Vec3::new(0.16 * w_thick * waist, pelvis_y, 0.0),
            old_position: spawn_pos + Vec3::new(0.16 * w_thick * waist, pelvis_y, 0.0),
            radius: 0.1 * w_thick,
            start_local: Vec3::new(0.16 * w_thick * waist, pelvis_y, 0.0),
        },
        PlayVerletNode {
            name: "R_Knee".to_string(),
            position: spawn_pos + Vec3::new(0.16 * w_thick * waist, knee_y, 0.0),
            old_position: spawn_pos + Vec3::new(0.16 * w_thick * waist, knee_y, 0.0),
            radius: 0.09 * w_thick,
            start_local: Vec3::new(0.16 * w_thick * waist, knee_y, 0.0),
        },
        PlayVerletNode {
            name: "R_Foot".to_string(),
            position: spawn_pos + Vec3::new(0.16 * w_thick * waist, 0.0, 0.0),
            old_position: spawn_pos + Vec3::new(0.16 * w_thick * waist, 0.0, 0.0),
            radius: 0.08 * w_thick,
            start_local: Vec3::new(0.16 * w_thick * waist, 0.0, 0.0),
        },
        PlayVerletNode {
            name: "L_Hand".to_string(),
            position: spawn_pos + Vec3::new(-0.7 * w_thick, chest_y, 0.0),
            old_position: spawn_pos + Vec3::new(-0.7 * w_thick, chest_y, 0.0),
            radius: 0.06 * w_thick,
            start_local: Vec3::new(-0.7 * w_thick, chest_y, 0.0),
        },
        PlayVerletNode {
            name: "R_Hand".to_string(),
            position: spawn_pos + Vec3::new(0.7 * w_thick, chest_y, 0.0),
            old_position: spawn_pos + Vec3::new(0.7 * w_thick, chest_y, 0.0),
            radius: 0.06 * w_thick,
            start_local: Vec3::new(0.7 * w_thick, chest_y, 0.0),
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

    let muscle_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.72, 0.1, 0.12),
        perceptual_roughness: 0.6,
        metallic: 0.1,
        ..default()
    });

    // 1. Sci-Fi Suit Materials
    let scifi_suit_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.16, 0.24),
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
        base_color: Color::srgb(0.85, 0.65, 0.15),
        metallic: 0.9,
        perceptual_roughness: 0.2,
        ..default()
    });

    // 2. Tactical Armor Materials
    let tac_vest_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.16, 0.20, 0.16),
        perceptual_roughness: 0.8,
        ..default()
    });
    let tac_camo_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.10, 0.12, 0.15),
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
        base_color: if char_settings.gender == crate::character_designer::Gender::Male {
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
        base_color: if char_settings.gender == crate::character_designer::Gender::Male {
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
        base_color: match char_settings.outfit_style {
            crate::character_designer::OutfitStyle::SkeletonExoFrame => Color::srgb(0.2, 0.9, 1.0),
            _ => Color::WHITE,
        },
        emissive: match char_settings.outfit_style {
            crate::character_designer::OutfitStyle::SkeletonExoFrame => {
                LinearRgba::new(2.5, 7.0, 10.0, 1.0)
            }
            _ => LinearRgba::BLACK,
        },
        unlit: char_settings.outfit_style
            == crate::character_designer::OutfitStyle::SkeletonExoFrame,
        perceptual_roughness: 0.85,
        ..default()
    });

    // We hold reference to spawned nodes to parent them
    let mut visual_nodes = std::collections::HashMap::new();

    // Loop nodes and spawn either bone shapes or skin spheres
    for node in nodes.iter() {
        let is_head = node.name == "Head";
        let is_torso = node.name == "Pelvis" || node.name == "Spine" || node.name == "Chest";
        let is_pants_area = node.name == "Pelvis" || node.name == "L_Hip" || node.name == "R_Hip";
        let is_foot = node.name == "L_Foot" || node.name == "R_Foot";

        let skin_mat_to_use = match char_settings.outfit_style {
            crate::character_designer::OutfitStyle::SciFiSuit => {
                if is_head || is_torso || is_pants_area {
                    scifi_suit_mat.clone()
                } else if is_foot {
                    scifi_trim_mat.clone()
                } else {
                    scifi_suit_mat.clone()
                }
            }
            crate::character_designer::OutfitStyle::TacticalArmor => {
                if is_torso || is_pants_area {
                    tac_vest_mat.clone()
                } else if is_foot {
                    tac_plate_mat.clone()
                } else {
                    tac_camo_mat.clone()
                }
            }
            crate::character_designer::OutfitStyle::StylizedHero => {
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
            crate::character_designer::OutfitStyle::SkeletonExoFrame => exo_glass_mat.clone(),
            crate::character_designer::OutfitStyle::ClassicMannequin => {
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

        // Node accessories based on OutfitStyle & Node Name
        match char_settings.outfit_style {
            crate::character_designer::OutfitStyle::SciFiSuit => {
                if is_head {
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
                            PlayModeEntity,
                        ))
                        .id();
                    commands.entity(node_id).add_child(visor);

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
                                PlayModeEntity,
                            ))
                            .id();
                        commands.entity(node_id).add_child(comm);
                    }
                } else if node.name == "Chest" {
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
                            PlayModeEntity,
                        ))
                        .id();
                    commands.entity(node_id).add_child(core);
                }
            }
            crate::character_designer::OutfitStyle::TacticalArmor => {
                if is_head {
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
                            PlayModeEntity,
                        ))
                        .id();
                    commands.entity(node_id).add_child(brim);

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
                                PlayModeEntity,
                            ))
                            .id();
                        commands.entity(node_id).add_child(nvg);
                    }
                }
            }
            crate::character_designer::OutfitStyle::StylizedHero => {
                if is_head {
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
                                PlayModeEntity,
                            ))
                            .id();
                        commands.entity(node_id).add_child(ew);

                        let ei = commands
                            .spawn((
                                Mesh3d(eye_iris_mesh.clone()),
                                MeshMaterial3d(eye_mat.clone()),
                                Transform::from_translation(Vec3::new(
                                    offset_x,
                                    mesh_radius * 0.15,
                                    mesh_radius * 0.96,
                                )),
                                PlayModeEntity,
                            ))
                            .id();
                        commands.entity(node_id).add_child(ei);

                        let ep = commands
                            .spawn((
                                Mesh3d(pupil_mesh.clone()),
                                MeshMaterial3d(eye_pupil_mat.clone()),
                                Transform::from_translation(Vec3::new(
                                    offset_x,
                                    mesh_radius * 0.15,
                                    mesh_radius * 1.02,
                                )),
                                PlayModeEntity,
                            ))
                            .id();
                        commands.entity(node_id).add_child(ep);

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
                                PlayModeEntity,
                            ))
                            .id();
                        commands.entity(node_id).add_child(eb);
                    }
                }
            }
            crate::character_designer::OutfitStyle::SkeletonExoFrame
            | crate::character_designer::OutfitStyle::ClassicMannequin => {
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
                                PlayModeEntity,
                            ))
                            .id();
                        commands.entity(node_id).add_child(e);
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
                health_packs: 0,
                is_headlamp_on: true,
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
            crate::water::WaterInteractor {
                mass: 1.0,
                ..default()
            },
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
        } else {
            match char_settings.outfit_style {
                crate::character_designer::OutfitStyle::SciFiSuit => scifi_suit_mat.clone(),
                crate::character_designer::OutfitStyle::TacticalArmor => {
                    if is_pants || is_torso {
                        tac_vest_mat.clone()
                    } else {
                        tac_camo_mat.clone()
                    }
                }
                crate::character_designer::OutfitStyle::StylizedHero => {
                    if is_pants {
                        hero_pants_mat.clone()
                    } else if is_torso {
                        hero_jacket_mat.clone()
                    } else {
                        skin_mat.clone()
                    }
                }
                crate::character_designer::OutfitStyle::SkeletonExoFrame => exo_glass_mat.clone(),
                crate::character_designer::OutfitStyle::ClassicMannequin => {
                    if is_pants {
                        pants_mat.clone()
                    } else if is_torso {
                        shirt_mat.clone()
                    } else {
                        skin_mat.clone()
                    }
                }
            }
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
            far: 3000.0,
            ..default()
        }),
        Transform::from_xyz(spawn_pos.x, spawn_pos.y + 4.0, spawn_pos.z - 6.0)
            .looking_at(spawn_pos, Vec3::Y),
        PlayModeCamera {
            target_distance: 3.2,
            yaw: 0.0,
            pitch: -0.2,
            view_mode: ViewMode::ThirdPerson,
        },
        DistanceFog {
            color: Color::srgb(0.18, 0.22, 0.45),
            falloff: FogFalloff::Linear {
                start: 1000.0,
                end: 6500.0,
            },
            ..default()
        },
        bevy::camera::visibility::RenderLayers::layer(0).with(1),
        PlayModeEntity,
    ));

    // 10. Spawn 3D Crashed Starship Wreckage near spawn location
    spawn_crashed_starship(&mut commands, &mut meshes, &mut materials, spawn_pos, &map);
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
        let extents = wall_collider.half_extents;
        commands.entity(entity).insert((
            avian3d::prelude::RigidBody::Static,
            avian3d::prelude::Collider::cuboid(extents.x * 2.0, extents.y * 2.0, extents.z * 2.0),
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

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

static MANSION_COLS: AtomicU32 = AtomicU32::new(8);
static MANSION_ROWS: AtomicU32 = AtomicU32::new(4);
static MANSION_CELL_SIZE: AtomicU32 = AtomicU32::new(1084227584); // 5.0f32.to_bits()
static HOUSE_POS_X: AtomicU32 = AtomicU32::new(0);
static HOUSE_POS_Z: AtomicU32 = AtomicU32::new(0);
static HOUSE_PLACED: AtomicBool = AtomicBool::new(false);

/// Returns (floor_y, ceiling_y) for the given position.
/// Used for grounding the player/creatures AND preventing them from passing through ceilings.
fn get_floor_and_ceiling(pos: Vec3, terrain_y: f32) -> (f32, f32) {
    if pos.y < -120.0 {
        return (cave::CAVE_FLOOR_Y, cave::CAVE_CEILING_Y);
    }
    if pos.y < -75.0 {
        return (-100.0, -96.0); // Sub-basement
    }
    if pos.y < -30.0 {
        return (-50.0, -46.0); // Basement
    }
    if HOUSE_PLACED.load(Ordering::Relaxed) {
        let cols = MANSION_COLS.load(Ordering::Relaxed);
        let rows = MANSION_ROWS.load(Ordering::Relaxed);
        let cell_size = f32::from_bits(MANSION_CELL_SIZE.load(Ordering::Relaxed));
        let house_pos_x = f32::from_bits(HOUSE_POS_X.load(Ordering::Relaxed));
        let house_pos_z = f32::from_bits(HOUSE_POS_Z.load(Ordering::Relaxed));

        let half_w = (cols as f32 * cell_size) * 0.5;
        let half_d = (rows as f32 * cell_size) * 0.5;
        let inside_mansion =
            (pos.x - house_pos_x).abs() < half_w && (pos.z - house_pos_z).abs() < half_d;
        if inside_mansion {
            let rel_x = pos.x - (house_pos_x - half_w);
            let rel_z = pos.z - (house_pos_z - half_d);
            let c = (rel_x / cell_size).floor() as i32;
            let r = (rel_z / cell_size).floor() as i32;

            let is_foyer_hole = (c == 3 || c == 4) && (r == 1 || r == 2);
            let staircase_x = house_pos_x - half_w + cell_size * 3.5;
            let near_staircase = (pos.x - staircase_x).abs() < (cell_size * 0.7);

            if is_foyer_hole {
                if near_staircase && pos.y > 1.4 && pos.y < 5.1 {
                    (pos.y, 8.5)
                } else {
                    (1.5, 8.5)
                }
            } else if pos.y > 3.25 {
                // On the second floor (floor 2)
                (5.0, 8.5) // floor at 5.0, ceiling at 8.5 (5.0 + 3.5)
            } else {
                // On the ground floor (floor 1)
                (1.5, 5.0) // floor at 1.5, ceiling at 5.0
            }
        } else {
            (terrain_y, f32::MAX) // Outdoors — no ceiling
        }
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
    player_config_query: Query<'w, 's, &'static bevy_tnua::prelude::TnuaConfig<ControlScheme>>,
    control_configs: ResMut<'w, Assets<ControlSchemeConfig>>,
    inventory: ResMut<'w, PlayerInventory>,
    mouse_input: Res<'w, ButtonInput<MouseButton>>,
    puzzle_state: Res<'w, crate::play_mode::house::HousePuzzleState>,
    ladder_query: Query<'w, 's, &'static GlobalTransform, With<structures::WatchtowerLadder>>,
}

fn sync_mansion_global_bounds_system(
    map: Res<TempestMap>,
    mansion_settings: Res<crate::play_mode::house::MansionSettings>,
) {
    let mut house_pos = Vec3::new(-35.0, 1.5, -35.0);
    for p in map.prefabs.iter() {
        if p.prefab_type == "house" {
            house_pos = Vec3::from_array(p.position);
            break;
        }
    }
    HOUSE_PLACED.store(true, Ordering::Relaxed);
    MANSION_COLS.store(mansion_settings.cols, Ordering::Relaxed);
    MANSION_ROWS.store(mansion_settings.rows, Ordering::Relaxed);
    MANSION_CELL_SIZE.store(mansion_settings.cell_size.to_bits(), Ordering::Relaxed);
    HOUSE_POS_X.store(house_pos.x.to_bits(), Ordering::Relaxed);
    HOUSE_POS_Z.store(house_pos.z.to_bits(), Ordering::Relaxed);
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
        (Entity, &WallCollider, &GlobalTransform),
        (Without<PlayModePlayer>, Without<PlayModeCamera>),
    >,
    door_query: Query<&crate::play_mode::house::HouseDoor>,
    mut tnua_query: Query<&mut bevy_tnua::prelude::TnuaController<ControlScheme>>,
    mut velocity_query: Query<&mut avian3d::prelude::LinearVelocity>,
    mut physics_pos_query: Query<&mut avian3d::prelude::Position>,
    mut settings: ResMut<CharacterSettings>,
) {
    let Ok((_player_entity, mut player, mut player_transform)) = player_query.single_mut() else {
        return;
    };

    if keyboard_input.just_pressed(KeyCode::F5) {
        let _ = save_progress(&params.inventory, &player, &settings);
    }
    if keyboard_input.just_pressed(KeyCode::F9) {
        let _ = load_progress(&mut params.inventory, &mut player, &mut settings);
    }
    let ui_active = params.inventory.show_ship_repair_window
        || params.inventory.show_alien_store
        || params.puzzle_state.active_terminal_log.is_some()
        || params.puzzle_state.show_security_keypad
        || params.puzzle_state.show_synthesizer_ui;
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

    if !ui_active && !egui_hovered {
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

    // Switch weapons with Key1..=Key5 (Only when UI is NOT active)
    let mut switched = false;
    if !ui_active {
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
        if params.inventory.has_flight_suit {
            if player.state == PlayerState::Flying {
                player.state = PlayerState::Active;
                inventory_log("🚀 Cyber Flight Suit deactivated — returning to ground controls");
            } else {
                player.state = PlayerState::Flying;
                player.velocity_y = 0.0;
                inventory_log("🚀 High-Tech Cyber Flight Suit engaged! Flying activated!");
            }
        } else {
            inventory_log(
                "❌ High-Tech Cyber Flight Suit required to fly! Craft it at the Workbench using 5 Platinum, 5 Steel, & 3 Alien Tech.",
            );
        }
    }

    if keyboard_input.just_pressed(KeyCode::KeyH) {
        player.is_headlamp_on = !player.is_headlamp_on;
        if player.is_headlamp_on {
            inventory_log("🔦 Tactical Headlamp switched ON [H]");
        } else {
            inventory_log("🔦 Tactical Headlamp switched OFF [H]");
        }
    }

    if keyboard_input.just_pressed(KeyCode::KeyQ) {
        if player.health_packs > 0 {
            if player.health >= player.max_health {
                inventory_log("❤️ Health is already full!");
            } else {
                player.health_packs -= 1;
                player.health = (player.health + 35.0).min(player.max_health);
                params.commands.spawn((
                    AudioPlayer::new(params.asset_server.load("chest_open.wav")),
                    PlaybackSettings::DESPAWN,
                ));
                inventory_log(&format!(
                    "❤️ Used Health Pack! Healed +35 HP (Current HP: {}/{}) - {} left",
                    player.health as u32, player.max_health as u32, player.health_packs
                ));
            }
        } else {
            inventory_log("❌ No Health Packs left! Find or place some on the map.");
        }
    }

    // Handle active Shield Timer countdown
    if params.inventory.shield_timer > 0.0 {
        params.inventory.shield_timer = (params.inventory.shield_timer - dt).max(0.0);
    }

    // Handle Surface Recall Teleporter Beacon key [B]
    if keyboard_input.just_pressed(KeyCode::KeyB) {
        if params.inventory.has_recall_beacon {
            let terrain_y = get_bilinear_height(0.0, 0.0, &map);
            let target_pos = Vec3::new(0.0, terrain_y + player.height * 0.5 + 0.08, 0.0);
            player.position = target_pos;
            if let Ok(mut phys_pos) = physics_pos_query.get_mut(_player_entity) {
                phys_pos.0 = target_pos;
            }
            player_transform.translation = target_pos;
            for n in player.nodes.iter_mut() {
                n.position = target_pos;
                n.old_position = target_pos;
            }
            params.commands.spawn((
                AudioPlayer::new(params.asset_server.load("chest_open.wav")),
                PlaybackSettings::DESPAWN,
            ));
            inventory_log("✨ Surface Recall Beacon activated! Teleported back to surface!");
        } else {
            inventory_log(
                "❌ No Recall Beacon crafted! Craft one at the Workbench using Alien Tech.",
            );
        }
    }

    // Auto-consume Health Pack as emergency rescue on fatal damage
    if player.health <= 0.0 && player.health_packs > 0 {
        player.health_packs -= 1;
        player.health = 35.0;
        params.commands.spawn((
            AudioPlayer::new(params.asset_server.load("chest_open.wav")),
            PlaybackSettings::DESPAWN,
        ));
        inventory_log(&format!(
            "🚨 EMERGENCY RESCUE! Auto-consumed Health Pack on fatal damage! Revived with 35 HP ({} Health Packs remaining)",
            player.health_packs
        ));
    }

    let p_state = player.state;

    // 2. Active Mode / Swimming Mode / Flying Mode / Piloting Controls / Climbing
    if p_state == PlayerState::Active
        || p_state == PlayerState::Swimming
        || p_state == PlayerState::Flying
        || p_state == PlayerState::PilotingStarship
        || p_state == PlayerState::Climbing
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
            let water_depth = if player_transform.translation.y > -20.0 && ground_y > -20.0 {
                (water_settings.height - ground_y).max(0.0)
            } else {
                0.0
            };

            let float_h = player.height * 0.5 + 0.08;
            let mut speed = 1.25 * player.height;
            if keyboard_input.pressed(KeyCode::ShiftLeft)
                || keyboard_input.pressed(KeyCode::ShiftRight)
            {
                speed *= 2.244; // Running speed (keeps overall sprint speed at 2.8 * height)
            }

            // Wade speed reduction in shallow water (only when feet are actually in the water)
            if water_depth > 0.1
                && player_transform.translation.y <= water_settings.height + float_h + 0.05
            {
                let wade_factor = (1.0 - (water_depth / 1.0) * 0.45).max(0.55);
                speed *= wade_factor;
            }

            // Check Watchtower Ladder climbing proximity
            let mut near_ladder = None;
            for ladder_trans in params.ladder_query.iter() {
                let l_pos = ladder_trans.translation();
                let horizontal_dist = Vec2::new(
                    player_transform.translation.x - l_pos.x,
                    player_transform.translation.z - l_pos.z,
                )
                .length();
                if horizontal_dist < 1.4
                    && player_transform.translation.y >= l_pos.y - 1.8
                    && player_transform.translation.y <= l_pos.y + 4.2
                {
                    near_ladder = Some((*ladder_trans, l_pos));
                    break;
                }
            }

            if let Some((_ladder_trans, _l_pos)) = near_ladder
                && (keyboard_input.pressed(KeyCode::KeyW)
                    || keyboard_input.pressed(KeyCode::Space)
                    || keyboard_input.pressed(KeyCode::KeyS)
                    || keyboard_input.pressed(KeyCode::ControlLeft)
                    || keyboard_input.pressed(KeyCode::KeyC))
            {
                player.state = PlayerState::Climbing;
                player.position = player_transform.translation;
                inventory_log("🪜 Engaged watchtower ladder — climbing mode active.");
            }

            // Update Tnua walk basis speed dynamically
            if let Ok(tnua_config) = params.player_config_query.get(_player_entity)
                && let Some(mut config) = params.control_configs.get_mut(&tnua_config.0)
            {
                config.basis.speed = speed;
            }

            let horizontal_vel = if let Ok(vel) = velocity_query.get(_player_entity) {
                Vec3::new(vel.x, 0.0, vel.z)
            } else {
                Vec3::ZERO
            };
            let current_speed = horizontal_vel.length();

            if player.is_walking {
                player.walk_timer += dt * current_speed * 5.0;
            } else {
                player.walk_timer = 0.0;
            }

            let float_height = player.height * 0.5 + 0.08;
            let terrain_y = get_bilinear_height(
                player_transform.translation.x,
                player_transform.translation.z,
                &map,
            );
            let ground_y = get_effective_floor_height(player_transform.translation, terrain_y);

            // Calculate black hole gravitational anomaly boost (higher jump height at night & floaty landing)
            let bh_boost = get_black_hole_gravity_boost(time.elapsed_secs());
            if let Ok(tnua_config) = params.player_config_query.get(_player_entity)
                && let Some(mut config) = params.control_configs.get_mut(&tnua_config.0)
            {
                config.jump.height = 3.5 + bh_boost * 24.5;
            }

            let current_y = player_transform.translation.y - float_height;
            let is_in_air = (current_y - ground_y) > 0.35;

            // Apply floaty low-gravity anti-gravitational pull during nighttime jump (both ascent and descent)
            if is_in_air
                && bh_boost > 0.0
                && let Ok(mut vel) = velocity_query.get_mut(_player_entity)
            {
                if vel.y > 0.0 {
                    // Ascending: counteract 45% of gravity so upward momentum soars high into the sky!
                    vel.y += 9.8 * 0.45 * bh_boost * dt;
                } else {
                    // Descending: counteract 82% of gravity for slow, graceful moon-landing float!
                    vel.y += 9.8 * 0.82 * bh_boost * dt;
                }
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

                if keyboard_input.just_pressed(KeyCode::Space)
                    || keyboard_input.pressed(KeyCode::Space)
                {
                    tnua.action(crate::ControlScheme::Jump(
                        bevy_tnua::builtins::TnuaBuiltinJump::default(),
                    ));
                    if (current_y - ground_y).abs() < 0.35
                        && let Ok(mut vel) = velocity_query.get_mut(_player_entity)
                        && vel.y <= 0.5
                    {
                        vel.y = 10.0 + bh_boost * 22.0;
                    }
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
            let is_deep_enough_to_swim = water_depth >= 1.0;
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

            let terrain_y = get_bilinear_height(
                player_transform.translation.x,
                player_transform.translation.z,
                &map,
            );
            player.position.x = player_transform.translation.x;
            player.position.z = player_transform.translation.z;
            player_transform.rotation = Quat::from_rotation_y(-player.rotation_yaw);
            let ground_y = get_effective_floor_height(player.position, terrain_y);
            let target_y = player_transform.translation.y - float_height;

            // Penetration recovery safety check: if physics body penetrates deeply below ground (> 0.40m), restore position to ground_y + float_height so Tnua ground check immediately succeeds
            if target_y < ground_y - 0.40 {
                let corrected_y = ground_y + float_height;
                player_transform.translation.y = corrected_y;
                if let Ok(mut phys_pos) = physics_pos_query.get_mut(_player_entity) {
                    phys_pos.0.y = corrected_y;
                }
                if let Ok(mut vel) = velocity_query.get_mut(_player_entity) {
                    vel.y = vel.y.max(0.0);
                }
            }

            let updated_target_y = player_transform.translation.y - float_height;
            let target_visual_y = updated_target_y.max(ground_y);
            let is_airborne = if let Ok(vel) = velocity_query.get(_player_entity) {
                vel.y.abs() > 0.1 || (updated_target_y - ground_y) > 0.03
            } else {
                (updated_target_y - ground_y) > 0.03
            };
            if !player.is_walking && !is_airborne && (target_visual_y - ground_y).abs() < 0.04 {
                player.position.y = ground_y;
            } else {
                player.position.y = target_visual_y;
            }
        } else {
            let mut target_pos = player.position;

            let terrain_y = get_bilinear_height(player.position.x, player.position.z, &map);
            let (ground_y, _ceiling_y) = get_floor_and_ceiling(player.position, terrain_y);
            let water_depth = if player.position.y > -20.0 && ground_y > -20.0 {
                (water_settings.height - player.position.y).max(0.0)
            } else {
                0.0
            };

            if p_state == PlayerState::Climbing {
                // Find closest ladder to snap to
                let mut closest_ladder = None;
                let mut min_dist = f32::MAX;
                for ladder_trans in params.ladder_query.iter() {
                    let l_pos = ladder_trans.translation();
                    let dist = Vec2::new(player.position.x - l_pos.x, player.position.z - l_pos.z)
                        .length();
                    if dist < min_dist {
                        min_dist = dist;
                        closest_ladder = Some((*ladder_trans, l_pos));
                    }
                }

                if let Some((ladder_trans, l_pos)) = closest_ladder
                    && min_dist < 2.0
                {
                    // Snap XZ to the ladder position
                    target_pos.x = l_pos.x;
                    target_pos.z = l_pos.z;

                    let target_top_y = l_pos.y + 3.2;

                    // Manual vertical movement
                    let mut climb_dir = 0.0;
                    if keyboard_input.pressed(KeyCode::KeyW)
                        || keyboard_input.pressed(KeyCode::Space)
                    {
                        climb_dir += 1.0;
                    }
                    if keyboard_input.pressed(KeyCode::KeyS)
                        || keyboard_input.pressed(KeyCode::ControlLeft)
                        || keyboard_input.pressed(KeyCode::KeyC)
                    {
                        climb_dir -= 1.0;
                    }

                    target_pos.y += climb_dir * 3.8 * dt;
                    player.is_walking = climb_dir != 0.0;

                    // Check boundaries
                    if target_pos.y >= target_top_y {
                        // Reached the top! Step onto the deck towards the center
                        let step_dir = -ladder_trans.right();
                        target_pos.x += step_dir.x * 1.2;
                        target_pos.z += step_dir.z * 1.2;
                        target_pos.y = target_top_y;
                        player.state = PlayerState::Active;
                        inventory_log("🪜 Climbed up onto the Watchtower observation deck!");
                    } else if target_pos.y <= l_pos.y - 1.7 {
                        // Reached the bottom! Transition back to Active
                        target_pos.y = l_pos.y - 1.7;
                        player.state = PlayerState::Active;
                        inventory_log("🪜 Safely climbed down to the ground.");
                    }
                } else {
                    // Lost ladder connection, drop back to Active state
                    player.state = PlayerState::Active;
                }
            } else {
                let mut move_dir = Vec3::ZERO;

                let cam_forward =
                    Vec3::new(cam_transform.forward().x, 0.0, cam_transform.forward().z)
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

                let mut speed = if p_state == PlayerState::Swimming {
                    1.5 * player.height
                } else if p_state == PlayerState::Flying {
                    8.0 * player.height
                } else {
                    1.25 * player.height
                };
                if keyboard_input.pressed(KeyCode::ShiftLeft)
                    || keyboard_input.pressed(KeyCode::ShiftRight)
                {
                    if p_state == PlayerState::Flying {
                        speed *= 2.5;
                    } else {
                        speed *= 2.244;
                    }
                }

                // Wade speed reduction in shallow water
                if p_state == PlayerState::Active && water_depth > 0.0 {
                    let wade_factor = (1.0 - (water_depth / 1.0) * 0.45).max(0.55);
                    speed *= wade_factor;
                }

                if player.is_walking {
                    move_dir = move_dir.normalize();
                    target_pos += move_dir * speed * dt;
                    player.walk_timer += dt * speed * 5.0;
                } else {
                    if p_state == PlayerState::Swimming {
                        player.walk_timer += dt * 2.0; // Slow gentle floating motion
                    } else if p_state == PlayerState::Flying {
                        player.walk_timer += dt * 1.5; // Hover motion
                    } else {
                        player.walk_timer = 0.0;
                    }
                }
            }

            // Manual 2D AABB-vs-Sphere horizontal wall collision & sliding solver (only for manual non-Tnua movement states)
            if p_state != PlayerState::Active {
                let player_radius = 0.32 * player.weight;
                let player_half_height = (player.height * 1.8) * 0.5;

                for (entity, collider, col_transform) in collider_query.iter() {
                    if let Ok(door) = door_query.get(entity)
                        && door.is_open
                    {
                        continue;
                    }

                    let center = col_transform.translation();
                    let extents = collider.half_extents;

                    // 1. Vertical Y Overlap Check: Skip if player's height range is completely above or below collider box
                    let player_y_min = target_pos.y;
                    let player_y_max = target_pos.y + player_half_height * 2.0;
                    let box_y_min = center.y - extents.y;
                    let box_y_max = center.y + extents.y;

                    if player_y_max < box_y_min || player_y_min > box_y_max {
                        continue; // Player is on a different floor height level, no collision
                    }

                    // 2. 2D Horizontal XZ Closest Point Computation
                    let closest_x = target_pos
                        .x
                        .clamp(center.x - extents.x, center.x + extents.x);
                    let closest_z = target_pos
                        .z
                        .clamp(center.z - extents.z, center.z + extents.z);

                    let dx = target_pos.x - closest_x;
                    let dz = target_pos.z - closest_z;
                    let dist_sq = dx * dx + dz * dz;

                    if dist_sq > 0.000001 {
                        // Player is outside the 2D bounding box
                        let dist = dist_sq.sqrt();
                        if dist < player_radius {
                            let penetration = player_radius - dist;
                            target_pos.x += (dx / dist) * penetration;
                            target_pos.z += (dz / dist) * penetration;
                        }
                    } else {
                        // Player center is inside the 2D bounding box - calculate minimum 2D escape vector
                        let overlap_left = target_pos.x - (center.x - extents.x);
                        let overlap_right = (center.x + extents.x) - target_pos.x;
                        let overlap_back = target_pos.z - (center.z - extents.z);
                        let overlap_front = (center.z + extents.z) - target_pos.z;

                        let min_overlap = overlap_left
                            .min(overlap_right)
                            .min(overlap_back)
                            .min(overlap_front);

                        if (min_overlap - overlap_left).abs() < 0.0001 {
                            target_pos.x = (center.x - extents.x) - player_radius;
                        } else if (min_overlap - overlap_right).abs() < 0.0001 {
                            target_pos.x = (center.x + extents.x) + player_radius;
                        } else if (min_overlap - overlap_back).abs() < 0.0001 {
                            target_pos.z = (center.z - extents.z) - player_radius;
                        } else {
                            target_pos.z = (center.z + extents.z) + player_radius;
                        }
                    }
                }
            }
            player.position = target_pos;

            let hw = map.width as f32 / 2.0;
            let hh = map.height as f32 / 2.0;
            player.position.x = player.position.x.clamp(-hw + 1.0, hw - 1.0);
            player.position.z = player.position.z.clamp(-hh + 1.0, hh - 1.0);

            let water_level = water_settings.height;
            let _is_deep_enough_to_swim = water_depth >= 1.0;

            if p_state == PlayerState::Swimming {
                let mut swim_y_dir = 0.0;
                if keyboard_input.pressed(KeyCode::Space) {
                    swim_y_dir += 1.0;
                }
                if keyboard_input.pressed(KeyCode::KeyC)
                    || keyboard_input.pressed(KeyCode::ControlLeft)
                    || keyboard_input.pressed(KeyCode::ControlRight)
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

                // Check for bridge vault / climb up onto bridge deck from water
                let offset_x = -(map.width as f32) / 2.0;
                let offset_z = -(map.height as f32) / 2.0;
                let mut near_bridge = false;
                let mut target_bridge_y = 1.35;

                for z_off in [-1, 0, 1] {
                    for x_off in [-1, 0, 1] {
                        let check_x =
                            ((player.position.x - offset_x) + x_off as f32).round() as i32;
                        let check_z =
                            ((player.position.z - offset_z) + z_off as f32).round() as i32;
                        if check_x >= 0
                            && check_x < map.width as i32
                            && check_z >= 0
                            && check_z < map.height as i32
                            && map.get_road(check_x as u32, check_z as u32) == 3
                        {
                            near_bridge = true;
                            let terrain_h = map.get_height(check_x as u32, check_z as u32);
                            target_bridge_y = (terrain_h + 0.08).max(1.30);
                            break;
                        }
                    }
                    if near_bridge {
                        break;
                    }
                }

                if near_bridge
                    && (keyboard_input.just_pressed(KeyCode::Space)
                        || (keyboard_input.pressed(KeyCode::Space)
                            && player.position.y >= water_level - 0.25))
                {
                    player.position.y = target_bridge_y + 0.45;
                    player.velocity_y = 2.0;
                    player.state = PlayerState::Active;
                    inventory_log("🧗 Climbed up onto the bridge deck!");
                    params.commands.spawn((
                        AudioPlayer::new(params.asset_server.load("chest_open.wav")),
                        PlaybackSettings::DESPAWN,
                    ));
                } else {
                    let lake_depth = (water_level - ground_y).max(0.0);
                    if lake_depth < 0.8 || player.position.y < -20.0 {
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
            } else if p_state == PlayerState::PilotingStarship {
                player.rotation_yaw = camera.yaw + std::f32::consts::PI;

                let mut move_dir = Vec3::ZERO;
                let cam_forward =
                    Vec3::new(cam_transform.forward().x, 0.0, cam_transform.forward().z)
                        .normalize_or_zero();
                let cam_right = Vec3::new(cam_transform.right().x, 0.0, cam_transform.right().z)
                    .normalize_or_zero();

                let speed = if keyboard_input.pressed(KeyCode::ShiftLeft)
                    || keyboard_input.pressed(KeyCode::ShiftRight)
                {
                    55.0
                } else {
                    30.0
                };

                if keyboard_input.pressed(KeyCode::KeyW) {
                    move_dir += cam_forward * speed;
                }
                if keyboard_input.pressed(KeyCode::KeyS) {
                    move_dir -= cam_forward * (speed * 0.5);
                }
                if keyboard_input.pressed(KeyCode::KeyA) {
                    move_dir -= cam_right * (speed * 0.7);
                }
                if keyboard_input.pressed(KeyCode::KeyD) {
                    move_dir += cam_right * (speed * 0.7);
                }

                if keyboard_input.pressed(KeyCode::Space) {
                    move_dir += Vec3::Y * 20.0;
                }
                if keyboard_input.pressed(KeyCode::ControlLeft)
                    || keyboard_input.pressed(KeyCode::KeyC)
                {
                    move_dir -= Vec3::Y * 18.0;
                }

                player.position += move_dir * dt;
                let min_y = get_bilinear_height(player.position.x, player.position.z, &map) + 1.5;
                player.position.y = player.position.y.max(min_y);

                // Dual Plasma Cannon firing [Left Click]
                if params.mouse_input.just_pressed(MouseButton::Left) {
                    let forward = cam_transform.forward();
                    let right = cam_transform.right();
                    let p1 = player.position + *right * -3.6 + *forward * 3.0;
                    let p2 = player.position + *right * 3.6 + *forward * 3.0;

                    let bolt_mat = params.materials.add(StandardMaterial {
                        base_color: Color::srgb(0.1, 0.9, 1.0),
                        emissive: LinearRgba::new(0.5, 6.0, 10.0, 1.0),
                        unlit: true,
                        ..default()
                    });

                    for origin in [p1, p2] {
                        params.commands.spawn((
                            Mesh3d(params.meshes.add(Sphere::new(0.35))),
                            MeshMaterial3d(bolt_mat.clone()),
                            Transform::from_translation(origin)
                                .looking_at(origin + *forward, Vec3::Y),
                            StarshipPlasmaBolt {
                                velocity: *forward * 90.0,
                                lifetime: 2.5,
                            },
                            PlayModeEntity,
                        ));
                    }

                    params.commands.spawn((
                        AudioPlayer::new(params.asset_server.load("pistol_shoot.wav")),
                        PlaybackSettings::DESPAWN,
                    ));
                    inventory_log("⚡ Starfighter Dual Plasma Cannons Fired!");
                }

                // Exit Starfighter vehicle using [KeyE]
                if keyboard_input.just_pressed(KeyCode::KeyE) {
                    player.state = PlayerState::Active;
                    let terrain_y = get_bilinear_height(player.position.x, player.position.z, &map);
                    player.position.y = terrain_y;
                    inventory_log("🚀 Exited Starfighter. Vehicle parked.");
                }
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

            // Apply manual positions ONLY in manual non-Tnua movement states
            if p_state != PlayerState::Active {
                player_transform.translation = player.position;
                if let Ok(mut phys_pos) = physics_pos_query.get_mut(_player_entity) {
                    phys_pos.0 = player.position;
                }
            }
        }

        let terrain_y = get_bilinear_height(player.position.x, player.position.z, &map);
        let (ground_y, _ceiling_y) = get_floor_and_ceiling(player.position, terrain_y);
        let water_level = water_settings.height;
        let water_depth = if player.position.y > -20.0 && ground_y > -20.0 {
            (water_level - ground_y).max(0.0)
        } else {
            0.0
        };

        let p_height = player.height;
        let p_weight = player.weight;
        let p_axe_swing_timer = player.axe_swing_timer;
        let p_rotation_yaw = player.rotation_yaw;
        let p_pos = player.position;
        let p_walk_timer = player.walk_timer;
        let p_is_walking = player.is_walking;
        let p_active_weapon = player.active_weapon;

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
        } else if p_state == PlayerState::Flying {
            // CYBER FLIGHT SUIT SOARING & LEGS DANGLING ANIMATION
            let sh_w = settings.shoulder_width;
            let waist = settings.waist_width;

            let fly_cycle = time.elapsed_secs() * 3.0;
            let hover_bob = (fly_cycle * 0.8).sin() * 0.08;
            let leg_sway = (fly_cycle * 1.2).sin() * 0.05;
            let leg_sway_alt = (fly_cycle * 1.2 + std::f32::consts::PI).sin() * 0.05;

            // Body inclination forward into wind
            let pelvis_pos = p_pos + Vec3::Y * (p_height * 0.45 + hover_bob);
            let spine_pos = pelvis_pos + Vec3::Y * (p_height * 0.16) + forward * 0.08;
            let chest_pos = pelvis_pos + Vec3::Y * (p_height * 0.32) + forward * 0.16;
            let head_pos = chest_pos + Vec3::Y * (p_height * 0.18) + forward * 0.08;

            nodes[0].position = pelvis_pos; // Pelvis
            nodes[1].position = spine_pos; // Spine
            nodes[2].position = chest_pos; // Chest
            nodes[3].position = head_pos; // Head

            let is_first_person = camera.view_mode == ViewMode::FirstPerson;

            if is_first_person {
                // First-Person arms holding active weapon
                let cam_forward = cam_transform.forward().as_vec3();
                let cam_right = cam_transform.right().as_vec3();
                let cam_up = cam_transform.up().as_vec3();

                nodes[4].position = chest_pos - right * 0.25 * p_weight * sh_w; // L_Shoulder
                nodes[6].position = chest_pos + right * 0.25 * p_weight * sh_w; // R_Shoulder

                let r_pos = cam_transform.translation + cam_forward * 0.5 + cam_right * 0.15
                    - cam_up * 0.15;
                let l_pos = cam_transform.translation + cam_forward * 0.7 + cam_right * 0.05
                    - cam_up * 0.12;

                nodes[14].position = l_pos;
                nodes[15].position = r_pos;

                nodes[5].position = nodes[4].position
                    + (nodes[14].position - nodes[4].position) * 0.5
                    - right * 0.08
                    - Vec3::Y * 0.05;
                nodes[7].position = nodes[6].position
                    + (nodes[15].position - nodes[6].position) * 0.5
                    + right * 0.08
                    - Vec3::Y * 0.05;
            } else {
                // Third-Person arms stabilizing thruster flight
                nodes[4].position = chest_pos - right * 0.25 * p_weight * sh_w; // L_Shoulder
                nodes[6].position = chest_pos + right * 0.25 * p_weight * sh_w; // R_Shoulder

                let is_rifle = p_active_weapon == ActiveWeapon::Rifle
                    || p_active_weapon == ActiveWeapon::Sniper;
                let is_pistol = p_active_weapon == ActiveWeapon::Pistol
                    || p_active_weapon == ActiveWeapon::Revolver;

                if is_rifle {
                    let r_pos = chest_pos + forward * 0.38 + right * 0.04 * p_weight
                        - Vec3::Y * 0.02 * p_height;
                    let l_pos = chest_pos + forward * 0.58 - right * 0.02 * p_weight
                        + Vec3::Y * 0.01 * p_height;
                    nodes[14].position = l_pos;
                    nodes[15].position = r_pos;

                    nodes[5].position = nodes[4].position
                        + (nodes[14].position - nodes[4].position) * 0.5
                        - right * 0.08
                        - forward * 0.08;
                    nodes[7].position = nodes[6].position
                        + (nodes[15].position - nodes[6].position) * 0.5
                        + right * 0.08
                        - forward * 0.06;
                } else if is_pistol {
                    let r_pos = chest_pos + forward * 0.40 + right * 0.02 * p_weight
                        - Vec3::Y * 0.12 * p_height;
                    let l_pos = r_pos - right * 0.03 * p_weight - forward * 0.02 + Vec3::Y * 0.01;
                    nodes[14].position = l_pos;
                    nodes[15].position = r_pos;

                    nodes[5].position = nodes[4].position
                        + (nodes[14].position - nodes[4].position) * 0.5
                        - right * 0.08
                        - forward * 0.08;
                    nodes[7].position = nodes[6].position
                        + (nodes[15].position - nodes[6].position) * 0.5
                        + right * 0.08
                        - forward * 0.06;
                } else {
                    // Flying flight control posture: hands angled down & back
                    let l_hand = nodes[4].position
                        - right * 0.18 * p_weight * sh_w
                        - forward * 0.22
                        - Vec3::Y * 0.35 * p_height;
                    let r_hand = nodes[6].position + right * 0.18 * p_weight * sh_w
                        - forward * 0.22
                        - Vec3::Y * 0.35 * p_height;
                    nodes[14].position = l_hand;
                    nodes[15].position = r_hand;

                    nodes[5].position = nodes[4].position
                        - right * 0.2 * p_weight * sh_w
                        - forward * 0.1
                        - Vec3::Y * 0.18 * p_height;
                    nodes[7].position = nodes[6].position + right * 0.2 * p_weight * sh_w
                        - forward * 0.1
                        - Vec3::Y * 0.18 * p_height;
                }
            }

            // DANGLING & SWAYING LEGS (Jetpack flight posture)
            let l_hip = pelvis_pos - right * 0.16 * p_weight * waist;
            let r_hip = pelvis_pos + right * 0.16 * p_weight * waist;
            nodes[8].position = l_hip;
            nodes[11].position = r_hip;

            // Knees dangle downward and slightly back with airflow sway
            let drag = if p_is_walking { 0.28 } else { 0.08 };
            nodes[9].position =
                l_hip - Vec3::Y * (p_height * 0.20) - forward * drag + right * (leg_sway * 0.04);
            nodes[12].position = r_hip - Vec3::Y * (p_height * 0.20) - forward * drag
                + right * (leg_sway_alt * 0.04);

            // Feet dangle downward behind knees with airflow sway
            nodes[10].position =
                nodes[9].position - Vec3::Y * (p_height * 0.18) - forward * (drag * 1.5)
                    + right * (leg_sway * 0.06);
            nodes[13].position =
                nodes[12].position - Vec3::Y * (p_height * 0.18) - forward * (drag * 1.5)
                    + right * (leg_sway_alt * 0.06);
        } else if p_state == PlayerState::Climbing {
            // LADDER CLIMBING SKELETON ANIMATION
            let sh_w = settings.shoulder_width;
            let waist = settings.waist_width;

            let climb_cycle = p_pos.y * 5.0; // Animates based on actual vertical position!
            let left_hand_up = (climb_cycle).sin() > 0.0;
            let right_hand_up = !left_hand_up;

            // Body flat against the ladder
            let pelvis_pos = p_pos + Vec3::Y * (p_height * 0.45);
            let spine_pos = pelvis_pos + Vec3::Y * (p_height * 0.16);
            let chest_pos = pelvis_pos + Vec3::Y * (p_height * 0.32);
            let head_pos = chest_pos + Vec3::Y * (p_height * 0.18);

            nodes[0].position = pelvis_pos;
            nodes[1].position = spine_pos;
            nodes[2].position = chest_pos;
            nodes[3].position = head_pos;

            // Shoulders
            nodes[4].position = chest_pos - right * 0.25 * p_weight * sh_w; // L_Shoulder
            nodes[6].position = chest_pos + right * 0.25 * p_weight * sh_w; // R_Shoulder

            // Hands grabbing rungs in front of the chest
            let l_hand_y = chest_pos.y + if left_hand_up { 0.25 } else { -0.15 };
            let r_hand_y = chest_pos.y + if right_hand_up { 0.25 } else { -0.15 };

            nodes[14].position = chest_pos - right * 0.15 + forward * 0.25;
            nodes[14].position.y = l_hand_y; // L_Hand
            nodes[15].position = chest_pos + right * 0.15 + forward * 0.25;
            nodes[15].position.y = r_hand_y; // R_Hand

            // Elbows bent outwards
            nodes[5].position =
                nodes[4].position + (nodes[14].position - nodes[4].position) * 0.5 - right * 0.08;
            nodes[7].position =
                nodes[6].position + (nodes[15].position - nodes[6].position) * 0.5 + right * 0.08;

            // Hips
            let l_hip = pelvis_pos - right * 0.16 * p_weight * waist;
            let r_hip = pelvis_pos + right * 0.16 * p_weight * waist;
            nodes[8].position = l_hip;
            nodes[11].position = r_hip;

            // Feet stepping on rungs
            let left_foot_up = !left_hand_up;
            let l_foot_y = pelvis_pos.y - 0.45 + if left_foot_up { 0.15 } else { -0.15 };
            let r_foot_y = pelvis_pos.y - 0.45 + if !left_foot_up { 0.15 } else { -0.15 };

            nodes[9].position = l_hip - Vec3::Y * 0.22 + forward * 0.08; // L_Knee
            nodes[12].position = r_hip - Vec3::Y * 0.22 + forward * 0.08; // R_Knee

            nodes[10].position = nodes[9].position - Vec3::Y * 0.2 + forward * 0.08;
            nodes[10].position.y = l_foot_y; // L_Foot
            nodes[13].position = nodes[12].position - Vec3::Y * 0.2 + forward * 0.08;
            nodes[13].position.y = r_foot_y; // R_Foot
        } else {
            // STANDING / UPRIGHT WALKING ALIGNMENT
            let sh_w = settings.shoulder_width;
            let leg_len = settings.leg_length;
            let waist = settings.waist_width;

            let pelvis_y = p_height * 0.45 * (2.0 - leg_len);
            let spine_y = pelvis_y + (p_height * 0.15);
            let chest_y = pelvis_y + (p_height * 0.3);
            let head_y = chest_y + (p_height * 0.18);
            let knee_y = pelvis_y * 0.5;

            nodes[0].position = p_pos + Vec3::Y * pelvis_y; // Pelvis
            nodes[1].position = p_pos + Vec3::Y * spine_y; // Spine
            nodes[2].position = p_pos + Vec3::Y * chest_y; // Chest
            nodes[3].position = p_pos + Vec3::Y * head_y; // Head

            let is_first_person = camera.view_mode == ViewMode::FirstPerson;

            if is_first_person {
                // FIRST PERSON ARM ALIGNMENT - Raise hands holding the weapon in front of the camera
                let mut arm_swing = p_walk_timer.sin() * 0.05;
                if p_axe_swing_timer.is_some() {
                    arm_swing = 0.0;
                }

                // Left shoulder & Right shoulder locked
                nodes[4].position = p_pos + Vec3::Y * chest_y - right * 0.25 * p_weight * sh_w; // L_Shoulder
                nodes[6].position = p_pos + Vec3::Y * chest_y + right * 0.25 * p_weight * sh_w; // R_Shoulder

                let cam_forward = cam_transform.forward().as_vec3();
                let cam_right = cam_transform.right().as_vec3();
                let cam_up = cam_transform.up().as_vec3();

                let is_rifle = p_active_weapon == ActiveWeapon::Rifle
                    || p_active_weapon == ActiveWeapon::Sniper;
                let is_pistol = p_active_weapon == ActiveWeapon::Pistol
                    || p_active_weapon == ActiveWeapon::Revolver;

                let (l_hand_pos, r_hand_pos) = if is_rifle {
                    // Rifle two-handed hold: Right hand on trigger, Left hand supporting barrel forward
                    let r_pos = cam_transform.translation + cam_forward * 0.5 + cam_right * 0.15
                        - cam_up * 0.15;
                    let l_pos = cam_transform.translation + cam_forward * 0.7 + cam_right * 0.05
                        - cam_up * 0.12;
                    (l_pos, r_pos)
                } else if is_pistol {
                    // Pistol two-handed hold: Both hands near each other in center
                    let r_pos = cam_transform.translation + cam_forward * 0.5 + cam_right * 0.1
                        - cam_up * 0.18;
                    let l_pos = r_pos - cam_right * 0.05 - cam_forward * 0.03 + cam_up * 0.02;
                    (l_pos, r_pos)
                } else {
                    // Melee / Default first-person hold
                    let l_pos = nodes[4].position - right * 0.1 * p_weight * sh_w
                        + forward * (0.4 + arm_swing)
                        - Vec3::Y * 0.15;

                    let mut r_pos =
                        cam_transform.translation + cam_forward * 0.55 + cam_right * 0.2
                            - cam_up * 0.15;

                    if let Some(t) = p_axe_swing_timer {
                        let offset = if t < 0.1 {
                            let factor = t / 0.1;
                            cam_up * (factor * 0.18)
                                - cam_forward * (factor * 0.15)
                                - cam_right * (factor * 0.08)
                        } else if t < 0.2 {
                            let factor = (t - 0.1) / 0.1;
                            let wind_up_offset =
                                cam_up * 0.18 - cam_forward * 0.15 - cam_right * 0.08;
                            let strike_offset =
                                -cam_up * 0.25 + cam_forward * 0.35 + cam_right * 0.05;
                            let t_smooth = factor * factor;
                            Vec3::lerp(wind_up_offset, strike_offset, t_smooth)
                        } else {
                            let factor = (t - 0.2) / 0.1;
                            let strike_offset =
                                -cam_up * 0.25 + cam_forward * 0.35 + cam_right * 0.05;
                            let t_smooth = factor * (2.0 - factor);
                            Vec3::lerp(strike_offset, Vec3::ZERO, t_smooth)
                        };
                        r_pos += offset;
                    }
                    (l_pos, r_pos)
                };

                nodes[14].position = l_hand_pos; // L_Hand
                nodes[15].position = r_hand_pos; // R_Hand

                nodes[5].position = nodes[4].position
                    + (nodes[14].position - nodes[4].position) * 0.5
                    - right * 0.08
                    - Vec3::Y * 0.05; // L_Elbow
                nodes[7].position = nodes[6].position
                    + (nodes[15].position - nodes[6].position) * 0.5
                    + right * 0.08
                    - Vec3::Y * 0.05; // R_Elbow
            } else {
                // THIRD PERSON ARM ALIGNMENT
                let walk_swing = if p_is_walking {
                    p_walk_timer.sin() * 0.22
                } else {
                    0.0
                };

                nodes[4].position = p_pos + Vec3::Y * chest_y - right * 0.25 * p_weight * sh_w; // L_Shoulder
                nodes[6].position = p_pos + Vec3::Y * chest_y + right * 0.25 * p_weight * sh_w; // R_Shoulder

                let is_rifle = p_active_weapon == ActiveWeapon::Rifle
                    || p_active_weapon == ActiveWeapon::Sniper;
                let is_pistol = p_active_weapon == ActiveWeapon::Pistol
                    || p_active_weapon == ActiveWeapon::Revolver;

                let cam_forward = cam_transform.forward().as_vec3();
                let pitch_offset = Vec3::Y * (cam_forward.y * 0.35);
                let (l_hand_pos, r_hand_pos) = if is_rifle {
                    // Rifle & Sniper two-handed hold: Raised up near chest/shoulder height
                    let r_pos = nodes[2].position + forward * 0.38 + right * 0.04 * p_weight
                        - Vec3::Y * 0.02 * p_height
                        + pitch_offset;
                    let l_pos = nodes[2].position + forward * 0.58 - right * 0.02 * p_weight
                        + Vec3::Y * 0.01 * p_height
                        + pitch_offset;
                    (l_pos, r_pos)
                } else if is_pistol {
                    // Pistol two-handed hold: Both hands together in front of chest
                    let r_pos = nodes[2].position + forward * 0.40 + right * 0.02 * p_weight
                        - Vec3::Y * 0.12 * p_height
                        + pitch_offset;
                    let l_pos = r_pos - right * 0.03 * p_weight - forward * 0.02 + Vec3::Y * 0.01;
                    (l_pos, r_pos)
                } else {
                    // Melee / Default behavior
                    let l_pos = nodes[4].position - right * 0.15 * p_weight * sh_w
                        + forward * (0.1 + walk_swing)
                        - Vec3::Y * 0.38 * p_height;

                    let mut r_pos = nodes[6].position
                        + right * 0.15 * p_weight * sh_w
                        + forward * (0.2 - walk_swing * 0.5)
                        - Vec3::Y * 0.25 * p_height;

                    if let Some(t) = p_axe_swing_timer {
                        if t < 0.1 {
                            let factor = t / 0.1;
                            let wind_up_pos = nodes[6].position
                                + Vec3::Y * 0.25 * p_height
                                + right * 0.08 * p_weight * sh_w
                                - forward * 0.15;
                            r_pos = Vec3::lerp(r_pos, wind_up_pos, factor);
                        } else if t < 0.2 {
                            let factor = (t - 0.1) / 0.1;
                            let wind_up_pos = nodes[6].position
                                + Vec3::Y * 0.25 * p_height
                                + right * 0.08 * p_weight * sh_w
                                - forward * 0.15;
                            let strike_pos = nodes[6].position - Vec3::Y * 0.45 * p_height
                                + right * 0.18 * p_weight * sh_w
                                + forward * 0.65;
                            let t_smooth = factor * factor;
                            r_pos = Vec3::lerp(wind_up_pos, strike_pos, t_smooth);
                        } else {
                            let factor = (t - 0.2) / 0.1;
                            let strike_pos = nodes[6].position - Vec3::Y * 0.45 * p_height
                                + right * 0.18 * p_weight * sh_w
                                + forward * 0.65;
                            let target_idle = nodes[6].position
                                + right * 0.15 * p_weight * sh_w
                                + forward * (0.2 - walk_swing * 0.5)
                                - Vec3::Y * 0.25 * p_height;
                            let t_smooth = factor * (2.0 - factor);
                            r_pos = Vec3::lerp(strike_pos, target_idle, t_smooth);
                        }
                    }
                    (l_pos, r_pos)
                };

                nodes[14].position = l_hand_pos; // L_Hand
                nodes[15].position = r_hand_pos; // R_Hand

                nodes[5].position = nodes[4].position
                    + (nodes[14].position - nodes[4].position) * 0.5
                    - right * 0.08 * p_weight * sh_w
                    - forward * 0.08; // L_Elbow
                nodes[7].position = nodes[6].position
                    + (nodes[15].position - nodes[6].position) * 0.5
                    + right * 0.08 * p_weight * sh_w
                    - forward * 0.06; // R_Elbow
            }

            let l_leg_swing = p_walk_timer.sin() * 0.35 * p_height;
            let r_leg_swing = -p_walk_timer.sin() * 0.35 * p_height;

            nodes[8].position = p_pos + Vec3::Y * pelvis_y - right * 0.16 * p_weight * waist; // L_Hip
            nodes[11].position = p_pos + Vec3::Y * pelvis_y + right * 0.16 * p_weight * waist; // R_Hip

            nodes[9].position = p_pos + Vec3::Y * knee_y - right * 0.16 * p_weight * waist
                + forward * l_leg_swing.max(0.0); // L_Knee
            nodes[12].position = p_pos
                + Vec3::Y * knee_y
                + right * 0.16 * p_weight * waist
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
                let ft_terrain = get_bilinear_height(
                    p_pos.x - right.x * 0.16 * waist,
                    p_pos.z - right.z * 0.16 * waist,
                    &map,
                );
                let ft_ground = get_effective_floor_height(p_pos, ft_terrain);
                ft_ground + l_foot_lift
            } else {
                p_pos.y + l_foot_lift
            };
            let r_foot_y = if is_grounded {
                let ft_terrain = get_bilinear_height(
                    p_pos.x + right.x * 0.16 * waist,
                    p_pos.z + right.z * 0.16 * waist,
                    &map,
                );
                let ft_ground = get_effective_floor_height(p_pos, ft_terrain);
                ft_ground + r_foot_lift
            } else {
                p_pos.y + r_foot_lift
            };

            nodes[10].position = Vec3::new(
                p_pos.x - right.x * 0.16 * waist + forward.x * l_leg_swing,
                l_foot_y,
                p_pos.z - right.z * 0.16 * waist + forward.z * l_leg_swing,
            ); // L_Foot
            nodes[13].position = Vec3::new(
                p_pos.x + right.x * 0.16 * waist + forward.x * r_leg_swing,
                r_foot_y,
                p_pos.z + right.z * 0.16 * waist + forward.z * r_leg_swing,
            ); // R_Foot
        }

        // Wade stepping interaction in shallow water (only when feet are actually in the water)
        if p_is_walking
            && water_depth > 0.10
            && p_pos.y <= water_level + 0.05
            && p_state == PlayerState::Active
        {
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
        let terrain_y = get_bilinear_height(player.position.x, player.position.z, &map);
        let (ground_y, _) = get_floor_and_ceiling(player.position, terrain_y);
        let water_depth = if player.position.y > -20.0 && ground_y > -20.0 {
            (water_level - player.position.y).max(0.0)
        } else {
            0.0
        };
        let is_deep_enough_to_swim = water_depth >= 1.0;

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
        let target_distance = if player.state == PlayerState::PilotingStarship {
            8.0
        } else {
            camera.target_distance
        };

        let camera_offset = Vec3::new(
            camera.yaw.cos() * target_distance * camera.pitch.cos(),
            target_distance * -camera.pitch.sin() + 1.2,
            camera.yaw.sin() * target_distance * camera.pitch.cos(),
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
                node.health -= inventory.equipped_axe.damage_multiplier() as i32;

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
                    "crystal" | "crystal_cluster" | "bioluminescent_crystal" => {
                        (Color::srgb(0.2, 0.9, 1.0), "Bioluminescent Crystal")
                    }
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
                        "crystal" | "crystal_cluster" | "bioluminescent_crystal" => {
                            inventory.crystal_shard += 3;
                            inventory_log("+3 Bioluminescent Crystal Shards added!");
                        }
                        _ => {}
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
                        "crystal" | "crystal_cluster" | "bioluminescent_crystal" => {
                            inventory.crystal_shard += 1;
                            inventory_log("+1 Crystal Shard collected");
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
        if player.state == PlayerState::PilotingStarship {
            *visibility = Visibility::Hidden;
            continue;
        }

        if let Some(node) = player.nodes.iter().find(|n| n.name == visual.name) {
            transform.translation = node.position;
            // Orient joint nodes to face the player's body movement direction
            if visual.name == "Head" || visual.name == "Chest" {
                transform.rotation =
                    Quat::from_rotation_y(std::f32::consts::FRAC_PI_2 - player.rotation_yaw);
            } else {
                transform.rotation = Quat::from_rotation_y(-player.rotation_yaw);
            }

            // Hide the head in first person to prevent clipping/view blockage
            if visual.name == "Head" && is_first_person {
                *visibility = Visibility::Hidden;
            } else {
                *visibility = Visibility::Inherited;
            }
        }
    }

    // Sync connecting limbs
    for (mut transform, limb) in limb_query.iter_mut() {
        if player.state == PlayerState::PilotingStarship {
            transform.scale = Vec3::ZERO;
            continue;
        }

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
    mut player_query: Query<&mut PlayModePlayer>,
    map: Res<TempestMap>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    keyboard: Res<ButtonInput<KeyCode>>,
    creature_query: Query<(
        Entity,
        &Transform,
        &creatures::PlayCreature,
        Option<&creatures::AggroState>,
    )>,
    starship_query: Query<(&Transform, &CrashedStarship)>,
    mut building_state: ResMut<structures::BuildingPlacementState>,
    mut char_settings: ResMut<CharacterSettings>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let Ok(mut player) = player_query.single_mut() else {
        return;
    };

    if keyboard.just_pressed(KeyCode::F5) {
        let _ = save_progress(&inventory, &player, &char_settings);
    }
    if keyboard.just_pressed(KeyCode::F9) {
        let _ = load_progress(&mut inventory, &mut player, &mut char_settings);
    }

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
                            if inventory.has_sword { "⚔ Broadsword (Infinite)".to_string() } else { format!("{} (Infinite)", inventory.equipped_axe.name()) }
                        }
                        ActiveWeapon::Pistol => format!("Pistol [{} / {}]", player.clip_pistol, player.ammo_pistol),
                        ActiveWeapon::Revolver => format!("Revolver [{} / {}]", player.clip_revolver, player.ammo_revolver),
                        ActiveWeapon::Rifle => format!("Rifle [{} / {}]", player.clip_rifle, player.ammo_rifle),
                        ActiveWeapon::Sniper => format!("Sniper [{} / {}]", player.clip_sniper, player.ammo_sniper),
                    }
                };
                ui.label(egui::RichText::new(ammo_text).strong().color(egui::Color32::from_rgb(90, 220, 255)));
            });
            ui.horizontal(|ui| {
                ui.label("🛡️ Armor:");
                ui.label(egui::RichText::new(inventory.equipped_armor.name()).strong().color(egui::Color32::from_rgb(180, 230, 255)));
            });
            ui.horizontal(|ui| {
                ui.label("🔦 Headlamp:");
                let hl_status = if player.is_headlamp_on {
                    egui::RichText::new("ON [H]").strong().color(egui::Color32::YELLOW)
                } else {
                    egui::RichText::new("OFF [H]").strong().color(egui::Color32::GRAY)
                };
                ui.label(hl_status);
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
                    ui.label("🔮 Crystal Shards:"); ui.label(egui::RichText::new(inventory.crystal_shard.to_string()).strong().color(egui::Color32::from_rgb(100, 240, 255))); ui.end_row();
                    ui.label("🦊 Fox / Alien Pelts:"); ui.label(egui::RichText::new((inventory.fox_pelt + inventory.alien_pelt).to_string()).strong().color(egui::Color32::from_rgb(200, 140, 90))); ui.end_row();
                    ui.label("🦘 Kangaroo Fur:"); ui.label(egui::RichText::new(inventory.kangaroo_fur.to_string()).strong().color(egui::Color32::from_rgb(220, 180, 110))); ui.end_row();
                    ui.label("🪶 Alien Feathers:"); ui.label(egui::RichText::new(inventory.alien_feather.to_string()).strong().color(egui::Color32::from_rgb(180, 220, 255))); ui.end_row();
                    ui.label("🔮 Monster Core:"); ui.label(egui::RichText::new(inventory.monster_core.to_string()).strong().color(egui::Color32::from_rgb(255, 180, 0))); ui.end_row();
                    ui.label("🛸 Alien Tech:"); ui.label(egui::RichText::new(inventory.alien_tech.to_string()).strong().color(egui::Color32::from_rgb(50, 230, 150))); ui.end_row();
                    ui.label("🤖 Robot Parts:"); ui.label(egui::RichText::new(inventory.robot_parts.to_string()).strong().color(egui::Color32::from_rgb(140, 150, 170))); ui.end_row();
                });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("💾 Save (F5)")
                                .strong()
                                .color(egui::Color32::BLACK),
                        )
                        .fill(egui::Color32::from_rgb(100, 240, 150)),
                    )
                    .clicked()
                {
                    let _ = save_progress(&inventory, &player, &char_settings);
                }
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new("📂 Load (F9)")
                                .strong()
                                .color(egui::Color32::BLACK),
                        )
                        .fill(egui::Color32::from_rgb(100, 200, 255)),
                    )
                    .clicked()
                {
                    let _ = load_progress(&mut inventory, &mut player, &mut char_settings);
                }
            });
            ui.add_space(10.0);
            ui.separator();

            ui.label(egui::RichText::new("Controls:").strong().underline());
            ui.label("• W, A, S, D to move / strafe\n• Shift to Run / Turbo Swim Sprint\n• Space to Jump, Swim Up, or Climb Bridge Deck\n• Ctrl or C to Dive Down (Water) or Crouch\n• Mouse to look and aim\n• Left-Click to shoot / swing melee\n• Press 1..=5 to switch weapon slot\n• Press [R] to reload current gun\n• Press [F5] to Quick Save / [F9] to Quick Load\n• Press [H] to toggle Tactical Headlamp ON/OFF\n• Press [Q] to use Health Pack (+35 HP)\n• Press [X] to dismantle Trilobite defender\n• Press [B] to activate Surface Recall Beacon\n• Press [G] to collapse into ragdoll!");

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
        .default_width(310.0)
        .anchor(egui::Align2::RIGHT_TOP, egui::Vec2::new(-10.0, 10.0))
        .collapsible(true)
        .resizable(true)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .max_height(560.0)
                .auto_shrink([false; 2])
                .show(ui, |ui| {
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
                            shadow_maps_enabled: false,
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
            ui.separator();

            // SECTION 2: METAL AXES & HARVESTING TOOLS
            ui.label(egui::RichText::new("🪓 METAL AXES & HARVESTING TOOLS").strong().underline().color(egui::Color32::from_rgb(255, 180, 80)));
            ui.add_space(4.0);

            // Copper Axe
            ui.label(egui::RichText::new("🟧 Copper Axe").strong().color(egui::Color32::from_rgb(220, 100, 40)));
            ui.label("Cost: 10 Copper Ore, 5 Wood (2x Chop Damage)");
            let can_craft_copper_axe = inventory.copper >= 10 && inventory.wood >= 5;
            ui.horizontal(|ui| {
                if inventory.equipped_axe == AxeTier::Copper {
                    ui.label("✔ Equipped");
                } else if ui.add_enabled(can_craft_copper_axe, egui::Button::new("🪓 Craft Copper Axe")).clicked() {
                    inventory.copper -= 10;
                    inventory.wood -= 5;
                    inventory.equipped_axe = AxeTier::Copper;
                    inventory_log("🟧 Crafted Copper Axe! Chop damage increased to 2x!");
                }
            });
            ui.separator();

            // Steel Battleaxe
            ui.label(egui::RichText::new("⚙ Steel Battleaxe").strong().color(egui::Color32::from_rgb(160, 170, 185)));
            ui.label("Cost: 10 Steel Chunks, 5 Wood (3x Chop Damage)");
            let can_craft_steel_axe = inventory.steel >= 10 && inventory.wood >= 5;
            ui.horizontal(|ui| {
                if inventory.equipped_axe == AxeTier::Steel {
                    ui.label("✔ Equipped");
                } else if ui.add_enabled(can_craft_steel_axe, egui::Button::new("🪓 Craft Steel Battleaxe")).clicked() {
                    inventory.steel -= 10;
                    inventory.wood -= 5;
                    inventory.equipped_axe = AxeTier::Steel;
                    inventory_log("⚙ Crafted Steel Battleaxe! Chop damage increased to 3x!");
                }
            });
            ui.separator();

            // Golden Waraxe
            ui.label(egui::RichText::new("👑 Golden Waraxe").strong().color(egui::Color32::from_rgb(255, 215, 0)));
            ui.label("Cost: 10 Gold Ore, 5 Wood (4x Chop Damage)");
            let can_craft_gold_axe = inventory.gold >= 10 && inventory.wood >= 5;
            ui.horizontal(|ui| {
                if inventory.equipped_axe == AxeTier::Gold {
                    ui.label("✔ Equipped");
                } else if ui.add_enabled(can_craft_gold_axe, egui::Button::new("🪓 Craft Golden Waraxe")).clicked() {
                    inventory.gold -= 10;
                    inventory.wood -= 5;
                    inventory.equipped_axe = AxeTier::Gold;
                    inventory_log("👑 Crafted Golden Waraxe! Chop damage increased to 4x!");
                }
            });
            ui.separator();

            // Platinum Excalibur Axe
            ui.label(egui::RichText::new("💎 Platinum Excalibur Axe").strong().color(egui::Color32::from_rgb(160, 220, 255)));
            ui.label("Cost: 10 Platinum Ore, 5 Wood (5x Instant Chop)");
            let can_craft_plat_axe = inventory.platinum >= 10 && inventory.wood >= 5;
            ui.horizontal(|ui| {
                if inventory.equipped_axe == AxeTier::Platinum {
                    ui.label("✔ Equipped");
                } else if ui.add_enabled(can_craft_plat_axe, egui::Button::new("🪓 Craft Platinum Excalibur Axe")).clicked() {
                    inventory.platinum -= 10;
                    inventory.wood -= 5;
                    inventory.equipped_axe = AxeTier::Platinum;
                    inventory_log("💎 Crafted Platinum Excalibur Axe! Instant harvesting unlocked (5x Chop)!");
                }
            });
            ui.add_space(8.0);
            ui.separator();

            // SECTION 3: PROTECTIVE SUITS & ARMORS
            ui.label(egui::RichText::new("🛡️ PROTECTIVE SUITS & ARMORS").strong().underline().color(egui::Color32::from_rgb(100, 200, 255)));
            ui.add_space(4.0);

            // Leather Armor
            ui.label(
                egui::RichText::new("🛡️ Reinforced Leather Armor")
                    .strong()
                    .color(egui::Color32::from_rgb(220, 150, 90)),
            );
            ui.label("Cost: 3 Pelts/Fur (-25% Damage Taken)");
            let total_pelts = inventory.fox_pelt + inventory.alien_pelt + inventory.kangaroo_fur;
            let can_craft_armor = total_pelts >= 3;
            ui.horizontal(|ui| {
                if inventory.equipped_armor == ArmorTier::Leather {
                    ui.label("🛡️ Equipped (-25% Dmg)");
                } else if ui
                    .add_enabled(can_craft_armor, egui::Button::new("🛡️ Equip Leather Armor"))
                    .clicked()
                {
                    let mut remaining = 3u32;
                    let take_fox = remaining.min(inventory.fox_pelt);
                    inventory.fox_pelt -= take_fox;
                    remaining -= take_fox;

                    let take_alien = remaining.min(inventory.alien_pelt);
                    inventory.alien_pelt -= take_alien;
                    remaining -= take_alien;

                    let take_roo = remaining.min(inventory.kangaroo_fur);
                    inventory.kangaroo_fur -= take_roo;

                    inventory.equipped_armor = ArmorTier::Leather;
                    inventory.has_leather_armor = true;
                    inventory_log("🛡️ Equipped Leather Armor! Granted -25% Damage Reduction!");
                }
            });
            ui.separator();

            // Copper Armor
            ui.label(egui::RichText::new("🟧 Copper Plated Armor").strong().color(egui::Color32::from_rgb(220, 100, 40)));
            ui.label("Cost: 10 Copper Ore (-35% Damage Taken)");
            let can_craft_copper_armor = inventory.copper >= 10;
            ui.horizontal(|ui| {
                if inventory.equipped_armor == ArmorTier::Copper {
                    ui.label("🛡️ Equipped (-35% Dmg)");
                } else if ui.add_enabled(can_craft_copper_armor, egui::Button::new("🛡️ Equip Copper Armor"))
                    .clicked()
                {
                    inventory.copper -= 10;
                    inventory.equipped_armor = ArmorTier::Copper;
                    inventory_log("🟧 Equipped Copper Plated Armor! Granted -35% Damage Reduction!");
                }
            });
            ui.separator();

            // Steel Heavy Plate Armor
            ui.label(egui::RichText::new("⚙ Steel Heavy Plate Armor").strong().color(egui::Color32::from_rgb(160, 175, 190)));
            ui.label("Cost: 10 Steel Chunks (-50% Damage Taken)");
            let can_craft_steel_armor = inventory.steel >= 10;
            ui.horizontal(|ui| {
                if inventory.equipped_armor == ArmorTier::Steel {
                    ui.label("🛡️ Equipped (-50% Dmg)");
                } else if ui.add_enabled(can_craft_steel_armor, egui::Button::new("🛡️ Equip Steel Heavy Armor"))
                    .clicked()
                {
                    inventory.steel -= 10;
                    inventory.equipped_armor = ArmorTier::Steel;
                    inventory_log("⚙ Equipped Steel Heavy Plate Armor! Granted -50% Damage Reduction!");
                }
            });
            ui.separator();

            // Platinum Mesh Armor
            ui.label(egui::RichText::new("💎 Platinum Mesh Armor").strong().color(egui::Color32::from_rgb(160, 220, 255)));
            ui.label("Cost: 8 Platinum Ore (-65% Damage Taken)");
            let can_craft_plat_armor = inventory.platinum >= 8;
            ui.horizontal(|ui| {
                if inventory.equipped_armor == ArmorTier::Platinum {
                    ui.label("🛡️ Equipped (-65% Dmg)");
                } else if ui.add_enabled(can_craft_plat_armor, egui::Button::new("🛡️ Equip Platinum Armor"))
                    .clicked()
                {
                    inventory.platinum -= 8;
                    inventory.equipped_armor = ArmorTier::Platinum;
                    inventory_log("💎 Equipped Platinum Mesh Armor! Granted -65% Damage Reduction!");
                }
            });
            ui.separator();

            // High-Tech Cyber Flight Suit
            ui.label(egui::RichText::new("🚀 High-Tech Cyber Flight Suit").strong().color(egui::Color32::from_rgb(50, 240, 180)));
            ui.label("Cost: 5 Platinum, 5 Steel, 3 Alien Tech (-80% Dmg & Unlocks Flight!)");
            let can_craft_flight_suit = inventory.platinum >= 5 && inventory.steel >= 5 && inventory.alien_tech >= 3;
            ui.horizontal(|ui| {
                if inventory.has_flight_suit {
                    ui.label("🚀 Equipped! Press [F] to Fly!");
                } else if ui.add_enabled(can_craft_flight_suit, egui::Button::new("🚀 Build Cyber Flight Suit"))
                    .clicked()
                {
                    inventory.platinum -= 5;
                    inventory.steel -= 5;
                    inventory.alien_tech -= 3;
                    inventory.equipped_armor = ArmorTier::FlightSuit;
                    inventory.has_flight_suit = true;
                    inventory_log("🚀 Built & Equipped High-Tech Cyber Flight Suit! Granted -80% Damage & Unlocked Flight [F]!");
                }
            });
            ui.add_space(8.0);
            ui.separator();

            // SECTION 4: SURVIVAL GEAR & EQUIPMENT
            ui.label(egui::RichText::new("⚡ SURVIVAL GEAR & EQUIPMENT").strong().underline().color(egui::Color32::from_rgb(220, 220, 100)));
            ui.add_space(4.0);

            // Survival Recipe 7: Surface Recall Teleporter Beacon
            ui.label(
                egui::RichText::new("✨ Surface Recall Teleporter Beacon")
                    .strong()
                    .color(egui::Color32::from_rgb(80, 220, 255)),
            );
            ui.label("Cost: 2 Alien Tech, 4 Robot Parts (Press [B] to Recall)");
            let can_craft_beacon = inventory.alien_tech >= 2 && inventory.robot_parts >= 4 && !inventory.has_recall_beacon;
            ui.horizontal(|ui| {
                if inventory.has_recall_beacon {
                    ui.label("✨ Unlocked (Press [B] anytime to Recall)");
                } else if ui
                    .add_enabled(can_craft_beacon, egui::Button::new("✨ Craft Recall Beacon"))
                    .clicked()
                {
                    inventory.alien_tech -= 2;
                    inventory.robot_parts -= 4;
                    inventory.has_recall_beacon = true;
                    inventory_log("✨ Crafted Recall Beacon! Press [B] anytime to teleport back to the surface!");
                }
            });
            ui.separator();

            // Survival Recipe 8: Plasma Energy Shield
            ui.label(
                egui::RichText::new("🔮 Plasma Energy Shield")
                    .strong()
                    .color(egui::Color32::from_rgb(255, 200, 50)),
            );
            ui.label(format!("Cost: 1 Monster Core (Active Shield: {:.1}s)", inventory.shield_timer));
            let can_activate_shield = inventory.monster_core >= 1;
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(can_activate_shield, egui::Button::new("🔮 Activate Energy Shield"))
                    .clicked()
                {
                    inventory.monster_core -= 1;
                    inventory.shield_timer = 20.0;
                    inventory_log("🔮 Activated Plasma Energy Shield! Invincible for 20 seconds!");
                }
            });
            ui.add_space(8.0);
            ui.separator();

            // SECTION 5: ENHANCED METAL AMMUNITION
            ui.label(egui::RichText::new("🔫 ENHANCED METAL AMMUNITION").strong().underline().color(egui::Color32::from_rgb(255, 100, 100)));
            ui.add_space(4.0);

            // Copper Pistol Ammo
            ui.label(egui::RichText::new("🟧 Copper High-Velocity Pistol Ammo").strong().color(egui::Color32::from_rgb(220, 100, 40)));
            ui.label("Cost: 3 Copper Ore (+20 Pistol Ammo)");
            let can_craft_copper_ammo = inventory.copper >= 3;
            ui.horizontal(|ui| {
                if ui.add_enabled(can_craft_copper_ammo, egui::Button::new("🔫 Craft Pistol Ammo"))
                    .clicked()
                {
                    inventory.copper -= 3;
                    player.ammo_pistol += 20;
                    inventory_log("🟧 Crafted +20 High-Velocity Copper Pistol Ammo!");
                }
            });
            ui.separator();

            // Steel Rifle Ammo
            ui.label(egui::RichText::new("⚙ Steel Armor-Piercing Rifle Ammo").strong().color(egui::Color32::from_rgb(160, 175, 190)));
            ui.label("Cost: 3 Steel Chunks (+40 Rifle Ammo)");
            let can_craft_steel_ammo = inventory.steel >= 3;
            ui.horizontal(|ui| {
                if ui.add_enabled(can_craft_steel_ammo, egui::Button::new("🔫 Craft Rifle Ammo"))
                    .clicked()
                {
                    inventory.steel -= 3;
                    player.ammo_rifle += 40;
                    inventory_log("⚙ Crafted +40 Steel Armor-Piercing Rifle Ammo!");
                }
            });
            ui.separator();

            // Platinum Revolver Ammo
            ui.label(egui::RichText::new("💎 Platinum Heavy Mag Revolver Ammo").strong().color(egui::Color32::from_rgb(160, 220, 255)));
            ui.label("Cost: 3 Platinum Ore (+15 Revolver Ammo)");
            let can_craft_plat_ammo = inventory.platinum >= 3;
            ui.horizontal(|ui| {
                if ui.add_enabled(can_craft_plat_ammo, egui::Button::new("🔫 Craft Revolver Ammo"))
                    .clicked()
                {
                    inventory.platinum -= 3;
                    player.ammo_revolver += 15;
                    inventory_log("💎 Crafted +15 Platinum Heavy Mag Revolver Ammo!");
                }
            });
            ui.separator();

            // Survival Recipe 9: Robot Ammo Synthesizer
            ui.label(
                egui::RichText::new("🤖 Robot Ammo Synthesizer")
                    .strong()
                    .color(egui::Color32::from_rgb(150, 170, 200)),
            );
            ui.label("Cost: 2 Robot Parts (+30 Rifle, +15 Pistol Ammo)");
            let can_synth_ammo = inventory.robot_parts >= 2;
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(can_synth_ammo, egui::Button::new("🤖 Synthesize Ammo"))
                    .clicked()
                {
                    inventory.robot_parts -= 2;
                    player.ammo_rifle += 30;
                    player.ammo_pistol += 15;
                    inventory_log("🤖 Synthesized +30 Rifle & +15 Pistol Ammo!");
                }
            });
            ui.separator();

            // Survival Recipe 10: High-Velocity Feathered Ammo
            ui.label(
                egui::RichText::new("🪶 Feathered High-Velocity Ammo")
                    .strong()
                    .color(egui::Color32::from_rgb(180, 220, 255)),
            );
            ui.label("Cost: 3 Alien Feathers (+10 Sniper Ammo)");
            let can_craft_sniper_ammo = inventory.alien_feather >= 3;
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(can_craft_sniper_ammo, egui::Button::new("🪶 Craft Sniper Ammo"))
                    .clicked()
                {
                    inventory.alien_feather -= 3;
                    player.ammo_sniper += 10;
                    inventory_log("🪶 Crafted +10 High-Velocity Sniper Ammo!");
                }
            });
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

    // 4. Check Proximity to Neutral Alien NPCs, Defender Trilobites, & Crashed Starship Terminal
    let mut near_alien_npc = false;
    let mut near_trilobite_entity = None;
    for (c_entity, c_transform, creature, aggro_opt) in creature_query.iter() {
        if creature.creature_type == creatures::CreatureType::Alien
            && creature.state != creatures::CreatureState::Dead
        {
            let is_provoked = aggro_opt.as_ref().is_some_and(|a| a.is_provoked);
            if !is_provoked && player.position.distance(c_transform.translation) < 4.5 {
                near_alien_npc = true;
            }
        } else if creature.creature_type == creatures::CreatureType::RobotTrilobite
            && creature.state != creatures::CreatureState::Dead
            && player.position.distance(c_transform.translation) < 4.5
        {
            near_trilobite_entity = Some(c_entity);
        }
    }

    if near_alien_npc {
        egui::Area::new(egui::Id::new("alien_barter_prompt"))
            .anchor(egui::Align2::CENTER_BOTTOM, egui::Vec2::new(0.0, -110.0))
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("💬 Press [E] to Barter & Trade with Alien NPC")
                        .size(18.0)
                        .strong()
                        .color(egui::Color32::from_rgb(100, 255, 180)),
                );
            });

        if keyboard.just_pressed(KeyCode::KeyE) {
            inventory.show_alien_store = !inventory.show_alien_store;
        }
    } else {
        inventory.show_alien_store = false;
    }

    if let Some(trilobite_entity) = near_trilobite_entity {
        egui::Area::new(egui::Id::new("trilobite_salvage_prompt"))
                .anchor(egui::Align2::CENTER_BOTTOM, egui::Vec2::new(0.0, -75.0))
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new("🤖 Press [X] to Dismantle & Recycle Defender Trilobite (+1 Robot Parts, +1 Steel)")
                            .size(18.0)
                            .strong()
                            .color(egui::Color32::from_rgb(140, 220, 255)),
                    );
                });

        if keyboard.just_pressed(KeyCode::KeyX) {
            commands.entity(trilobite_entity).despawn();
            inventory.robot_parts += 1;
            inventory.steel += 1;
            inventory_log("🤖 Dismantled Defender Trilobite into +1 Robot Parts & +1 Steel!");
        }
    }

    // Proximity to Crashed/Repaired Starship Wreckage
    let crash_site_pos = if let Ok((ship_trans, _)) = starship_query.single() {
        ship_trans.translation
    } else {
        Vec3::new(8.0, get_bilinear_height(8.0, 10.0, &map), 10.0)
    };
    let near_starship = player.position.distance(crash_site_pos) < 7.0;

    if near_starship && player.state != PlayerState::PilotingStarship {
        egui::Area::new(egui::Id::new("starship_prompt"))
            .anchor(egui::Align2::CENTER_BOTTOM, egui::Vec2::new(0.0, -145.0))
            .show(ctx, |ui| {
                if !inventory.starship_repaired {
                    ui.label(
                        egui::RichText::new(
                            "💬 Press [E] to Access Crashed Starship Repair Terminal",
                        )
                        .size(18.0)
                        .strong()
                        .color(egui::Color32::from_rgb(255, 200, 80)),
                    );
                } else {
                    ui.label(
                        egui::RichText::new("🚀 Press [E] to Board & Pilot Starfighter!")
                            .size(19.0)
                            .strong()
                            .color(egui::Color32::from_rgb(80, 230, 255)),
                    );
                }
            });

        if keyboard.just_pressed(KeyCode::KeyE) {
            if !inventory.starship_repaired {
                inventory.show_ship_repair_window = !inventory.show_ship_repair_window;
            } else {
                player.state = PlayerState::PilotingStarship;
                inventory_log(
                    "🚀 Boarded Starfighter! [WASD] Throttle/Steer, [Space] Lift, [Shift] Nitro Boost, [Left Click] Plasma Cannons!",
                );
            }
        }
    } else {
        inventory.show_ship_repair_window = false;
    }

    // Crashed Starship Repair Console Window
    if inventory.show_ship_repair_window {
        egui::Window::new("🛠️ Crashed Starship Restoration Project")
            .default_width(420.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("🚀 STARSHIP EMERGENCY REPAIR CONSOLE");
                ui.label(
                    egui::RichText::new("\"System Diagnostic: Critical hull breach, thruster offline, avionics fried. Supply raw materials to restore starship functionality!\"")
                        .italics()
                        .color(egui::Color32::from_rgb(200, 220, 255)),
                );
                ui.separator();

                // Subsystem 1: Hull Integrity & Wings
                ui.label(egui::RichText::new("🛠️ Subsystem 1: Hull Integrity & Wings").strong().color(egui::Color32::from_rgb(255, 180, 100)));
                ui.label(format!("Steel Progress: {}/15  |  Platinum Progress: {}/10", inventory.ship_repair_steel, inventory.ship_repair_platinum));
                let can_add_hull = (inventory.steel > 0 && inventory.ship_repair_steel < 15) || (inventory.platinum > 0 && inventory.ship_repair_platinum < 10);
                if ui.add_enabled(can_add_hull, egui::Button::new("🛠️ Contribute Hull Plating (Steel / Platinum)")).clicked() {
                    let steel_needed = 15usize.saturating_sub(inventory.ship_repair_steel as usize) as u32;
                    let steel_add = inventory.steel.min(steel_needed);
                    inventory.steel -= steel_add;
                    inventory.ship_repair_steel += steel_add;

                    let plat_needed = 10usize.saturating_sub(inventory.ship_repair_platinum as usize) as u32;
                    let plat_add = inventory.platinum.min(plat_needed);
                    inventory.platinum -= plat_add;
                    inventory.ship_repair_platinum += plat_add;
                    inventory_log("🛠️ Contributed materials to Starship Hull Repair!");
                }
                ui.separator();

                // Subsystem 2: Plasma Thrusters & Energy Core
                ui.label(egui::RichText::new("⚡ Subsystem 2: Plasma Thrusters & Energy Core").strong().color(egui::Color32::from_rgb(100, 230, 255)));
                ui.label(format!("Crystal Progress: {}/5", inventory.ship_repair_crystals));
                let can_add_thrusters = inventory.crystal_shard > 0 && inventory.ship_repair_crystals < 5;
                if ui.add_enabled(can_add_thrusters, egui::Button::new("⚡ Contribute Energy Crystals")).clicked() {
                    let cry_needed = 5usize.saturating_sub(inventory.ship_repair_crystals as usize) as u32;
                    let cry_add = inventory.crystal_shard.min(cry_needed);
                    inventory.crystal_shard -= cry_add;
                    inventory.ship_repair_crystals += cry_add;
                    inventory_log("⚡ Contributed Crystal Shards to Starship Plasma Thrusters!");
                }
                ui.separator();

                // Subsystem 3: Avionics & Flight Guidance
                ui.label(egui::RichText::new("🤖 Subsystem 3: Avionics & Navigation Flight Core").strong().color(egui::Color32::from_rgb(180, 150, 255)));
                ui.label(format!("Robot Parts Progress: {}/5  |  Alien Tech Progress: {}/3", inventory.ship_repair_robot_parts, inventory.ship_repair_alien_tech));
                let can_add_avionics = (inventory.robot_parts > 0 && inventory.ship_repair_robot_parts < 5) || (inventory.alien_tech > 0 && inventory.ship_repair_alien_tech < 3);
                if ui.add_enabled(can_add_avionics, egui::Button::new("🤖 Contribute Avionics Components")).clicked() {
                    let rob_needed = 5usize.saturating_sub(inventory.ship_repair_robot_parts as usize) as u32;
                    let rob_add = inventory.robot_parts.min(rob_needed);
                    inventory.robot_parts -= rob_add;
                    inventory.ship_repair_robot_parts += rob_add;

                    let tech_needed = 3usize.saturating_sub(inventory.ship_repair_alien_tech as usize) as u32;
                    let tech_add = inventory.alien_tech.min(tech_needed);
                    inventory.alien_tech -= tech_add;
                    inventory.ship_repair_alien_tech += tech_add;
                    inventory_log("🤖 Contributed Avionics & Alien Tech to Starship Navigation!");
                }
                ui.separator();

                // Check Completion
                let is_complete = inventory.ship_repair_steel >= 15
                    && inventory.ship_repair_platinum >= 10
                    && inventory.ship_repair_crystals >= 5
                    && inventory.ship_repair_robot_parts >= 5
                    && inventory.ship_repair_alien_tech >= 3;

                if is_complete {
                    ui.label(egui::RichText::new("🎉 ALL SUBSYSTEMS FULLY RESTORED!").strong().size(16.0).color(egui::Color32::from_rgb(80, 255, 180)));
                    if ui.button("🚀 INITIATE ALL SYSTEMS & LAUNCH STARFIGHTER").clicked() {
                        inventory.starship_repaired = true;
                        inventory.show_ship_repair_window = false;
                        inventory_log("🚀 STARSHIP FULLY RESTORED & OPERATIONAL! Approach the cockpit & press [E] to fly!");
                    }
                }
            });
    }

    // 6. Starfighter Piloting Flight HUD Overlay
    if player.state == PlayerState::PilotingStarship {
        egui::Area::new(egui::Id::new("starfighter_flight_hud"))
            .anchor(egui::Align2::LEFT_TOP, egui::Vec2::new(20.0, 80.0))
            .show(ctx, |ui| {
                egui::Frame::group(ui.style())
                    .fill(egui::Color32::from_rgba_unmultiplied(10, 20, 35, 220))
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.heading(
                            egui::RichText::new("🚀 STARFIGHTER FLIGHT DASHBOARD")
                                .color(egui::Color32::from_rgb(100, 240, 255)),
                        );
                        ui.separator();
                        ui.label(
                            egui::RichText::new("STATUS: ALL SYSTEMS OPERATIONAL")
                                .strong()
                                .color(egui::Color32::from_rgb(80, 255, 180)),
                        );
                        ui.label("⚡ Dual Plasma Cannons: ONLINE [Left Click]");
                        ui.label("🚀 Afterburner Nitro Boost: READY [Shift]");
                        ui.separator();
                        ui.label(
                            egui::RichText::new("🎮 CONTROLS GUIDE:")
                                .strong()
                                .color(egui::Color32::YELLOW),
                        );
                        ui.label("• [W / S] — Accelerate / Brake");
                        ui.label("• [A / D] — Pitch / Yaw Steering & Banking Roll");
                        ui.label("• [Space / Ctrl] — Ascend / Descend Altitude");
                        ui.label("• [Left Click] — Fire Plasma Cannon Lasers");
                        ui.label("• [E] — Land & Disembark Starfighter");
                    });
            });
    }

    // 5. Alien NPC Barter & Trade Station Window
    if inventory.show_alien_store {
        egui::Window::new("🛸 Alien NPC Barter & Trade Station")
            .default_width(380.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("👽 GREETINGS SURVIVOR OF THE FALLEN STAR-VESSEL");
                ui.label(
                    egui::RichText::new("\"Ah, traveler! We saw your starship burn through our atmosphere and crash-land upon our world. We collect raw surface minerals & crash site salvage. Trade with us for Alien Tech to repair your gear!\"")
                        .italics()
                        .color(egui::Color32::from_rgb(150, 230, 255)),
                );
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("🛸 Your Current Alien Tech:");
                    ui.label(
                        egui::RichText::new(format!("{} Alien Tech", inventory.alien_tech))
                            .strong()
                            .color(egui::Color32::from_rgb(80, 255, 180)),
                    );
                });
                ui.separator();

                egui::ScrollArea::vertical()
                    .max_height(340.0)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("🤝 AVAILABLE BARTER DEALS").strong().color(egui::Color32::from_rgb(255, 200, 100)));
                        ui.add_space(4.0);

                        // Deal 1: Wood & Stone Bundle
                        ui.label(egui::RichText::new("🪵 Timber & Stone Pack").strong().color(egui::Color32::from_rgb(220, 160, 100)));
                        ui.label("Cost: 15 Wood, 10 Stone ➔ +1 Alien Tech");
                        let can_barter_1 = inventory.wood >= 15 && inventory.rock >= 10;
                        ui.horizontal(|ui| {
                            if ui.add_enabled(can_barter_1, egui::Button::new("🤝 Trade Timber & Stone")).clicked() {
                                inventory.wood -= 15;
                                inventory.rock -= 10;
                                inventory.alien_tech += 1;
                                inventory_log("🛸 Alien Trader: May your star-vessel rise again! +1 Alien Tech received!");
                            }
                        });
                        ui.separator();

                        // Deal 2: Copper & Iron Ore Bundle
                        ui.label(egui::RichText::new("🟧 Copper & Iron Ore Pack").strong().color(egui::Color32::from_rgb(230, 120, 50)));
                        ui.label("Cost: 5 Copper Ore, 5 Iron Ore ➔ +1 Alien Tech");
                        let can_barter_2 = inventory.copper >= 5 && inventory.iron >= 5;
                        ui.horizontal(|ui| {
                            if ui.add_enabled(can_barter_2, egui::Button::new("🤝 Trade Copper & Iron")).clicked() {
                                inventory.copper -= 5;
                                inventory.iron -= 5;
                                inventory.alien_tech += 1;
                                inventory_log("🛸 Alien Trader: Excellent minerals! +1 Alien Tech received!");
                            }
                        });
                        ui.separator();

                        // Deal 3: Gold Ore Offer
                        ui.label(egui::RichText::new("👑 Gold Ingot Specimen").strong().color(egui::Color32::from_rgb(255, 215, 0)));
                        ui.label("Cost: 3 Gold Ore ➔ +1 Alien Tech");
                        let can_barter_3 = inventory.gold >= 3;
                        ui.horizontal(|ui| {
                            if ui.add_enabled(can_barter_3, egui::Button::new("🤝 Trade Gold Ore")).clicked() {
                                inventory.gold -= 3;
                                inventory.alien_tech += 1;
                                inventory_log("🛸 Alien Trader: Pure Gold! Very valuable! +1 Alien Tech received!");
                            }
                        });
                        ui.separator();

                        // Deal 4: Platinum Ore Offer
                        ui.label(egui::RichText::new("💎 Platinum Ore Specimen").strong().color(egui::Color32::from_rgb(160, 220, 255)));
                        ui.label("Cost: 2 Platinum Ore ➔ +1 Alien Tech");
                        let can_barter_4 = inventory.platinum >= 2;
                        ui.horizontal(|ui| {
                            if ui.add_enabled(can_barter_4, egui::Button::new("🤝 Trade Platinum Ore")).clicked() {
                                inventory.platinum -= 2;
                                inventory.alien_tech += 1;
                                inventory_log("🛸 Alien Trader: Rare Platinum! +1 Alien Tech received!");
                            }
                        });
                        ui.separator();

                        // Deal 5: Monster Core Specimen
                        ui.label(egui::RichText::new("🔮 Subterranean Monster Core").strong().color(egui::Color32::from_rgb(255, 200, 50)));
                        ui.label("Cost: 1 Monster Core ➔ +1 Alien Tech");
                        let can_barter_5 = inventory.monster_core >= 1;
                        ui.horizontal(|ui| {
                            if ui.add_enabled(can_barter_5, egui::Button::new("🤝 Trade Monster Core")).clicked() {
                                inventory.monster_core -= 1;
                                inventory.alien_tech += 1;
                                inventory_log("🛸 Alien Trader: Fascinating biological power core! +1 Alien Tech received!");
                            }
                        });
                        ui.separator();

                        // Deal 6: Alien Feathers Collection
                        ui.label(egui::RichText::new("🪶 Alien Feathers Collection").strong().color(egui::Color32::from_rgb(180, 220, 255)));
                        ui.label("Cost: 5 Alien Feathers ➔ +1 Alien Tech");
                        let can_barter_6 = inventory.alien_feather >= 5;
                        ui.horizontal(|ui| {
                            if ui.add_enabled(can_barter_6, egui::Button::new("🤝 Trade Feathers")).clicked() {
                                inventory.alien_feather -= 5;
                                inventory.alien_tech += 1;
                                inventory_log("🛸 Alien Trader: Soft flight plumage! +1 Alien Tech received!");
                            }
                        });
                        ui.separator();

                        // Deal 7: Hides & Pelts Assortment
                        ui.label(egui::RichText::new("🦊 Pelts & Fur Assortment").strong().color(egui::Color32::from_rgb(220, 150, 90)));
                        ui.label("Cost: 3 Pelts/Fur ➔ +1 Alien Tech");
                        let total_pelts = inventory.fox_pelt + inventory.alien_pelt + inventory.kangaroo_fur;
                        let can_barter_7 = total_pelts >= 3;
                        ui.horizontal(|ui| {
                            if ui.add_enabled(can_barter_7, egui::Button::new("🤝 Trade Animal Pelts")).clicked() {
                                let mut remaining = 3u32;
                                let take_fox = remaining.min(inventory.fox_pelt);
                                inventory.fox_pelt -= take_fox;
                                remaining -= take_fox;
                                let take_alien = remaining.min(inventory.alien_pelt);
                                inventory.alien_pelt -= take_alien;
                                remaining -= take_alien;
                                let take_roo = remaining.min(inventory.kangaroo_fur);
                                inventory.kangaroo_fur -= take_roo;

                                inventory.alien_tech += 1;
                                inventory_log("🛸 Alien Trader: Warm pelts for alien winter! +1 Alien Tech received!");
                            }
                        });
                        ui.separator();

                        // Deal 8: Salvaged Robot Parts
                        ui.label(egui::RichText::new("🤖 Salvaged Robot Parts").strong().color(egui::Color32::from_rgb(150, 170, 200)));
                        ui.label("Cost: 3 Robot Parts ➔ +1 Alien Tech");
                        let can_barter_8 = inventory.robot_parts >= 3;
                        ui.horizontal(|ui| {
                            if ui.add_enabled(can_barter_8, egui::Button::new("🤝 Trade Robot Parts")).clicked() {
                                inventory.robot_parts -= 3;
                                inventory.alien_tech += 1;
                                inventory_log("🛸 Alien Trader: High grade machine components! +1 Alien Tech received!");
                            }
                        });
                        ui.separator();

                        // Deal 9: Bioluminescent Crystal Shards
                        ui.label(egui::RichText::new("🔮 Crystal Shards Specimen").strong().color(egui::Color32::from_rgb(100, 240, 255)));
                        ui.label("Cost: 3 Crystal Shards ➔ +1 Alien Tech");
                        let can_barter_9 = inventory.crystal_shard >= 3;
                        ui.horizontal(|ui| {
                            if ui.add_enabled(can_barter_9, egui::Button::new("🤝 Trade Crystal Shards")).clicked() {
                                inventory.crystal_shard -= 3;
                                inventory.alien_tech += 1;
                                inventory_log("🛸 Alien Trader: Luminescent energy crystals! +1 Alien Tech received!");
                            }
                        });
                    });

                ui.separator();
                if ui.button("❌ Close Alien Store [E]").clicked() {
                    inventory.show_alien_store = false;
                }
            });
    }

    // 7. Building Placement Selection Station Window
    if building_state.is_active {
        egui::Window::new("🏗️ Procedural Building Placement Mode")
            .default_width(330.0)
            .anchor(egui::Align2::RIGHT_CENTER, egui::Vec2::new(-10.0, 0.0))
            .collapsible(false)
            .show(ctx, |ui| {
                ui.heading("Select Structure to Construct [Keys 1..=9]:");
                ui.separator();

                let structures_list = [
                    (structures::StructureType::ClassicBrickWall, "1"),
                    (structures::StructureType::Watchtower, "2"),
                    (structures::StructureType::Staircase, "3"),
                    (structures::StructureType::Ramp, "4"),
                    (structures::StructureType::WoodenBridge, "5"),
                    (structures::StructureType::PalisadeFence, "6"),
                    (structures::StructureType::GraniteFortressWall, "7"),
                    (structures::StructureType::LogTimberWall, "8"),
                    (structures::StructureType::CyberMetalWall, "9"),
                ];

                for (st, key_str) in structures_list {
                    let is_selected = building_state.selected_structure == st;
                    let btn_text = format!("[{}] {}", key_str, st.name());
                    let btn = if is_selected {
                        egui::Button::new(
                            egui::RichText::new(&btn_text)
                                .strong()
                                .color(egui::Color32::YELLOW),
                        )
                        .fill(egui::Color32::from_rgb(40, 90, 140))
                    } else {
                        egui::Button::new(btn_text)
                    };
                    if ui.add(btn).clicked() {
                        building_state.selected_structure = st;
                    }
                    ui.label(egui::RichText::new(st.description()).small().weak());
                    ui.add_space(2.0);
                }

                ui.separator();
                let is_brick_wall = building_state.selected_structure == structures::StructureType::ClassicBrickWall;
                if is_brick_wall {
                    ui.label(
                        egui::RichText::new("🧱 Multi-Point Wall Builder:\n• Left-Click ground to place points 1, 2, 3...\n• Arrow Up / Down to adjust Wall Height\n• Press [ENTER] to construct complete wall\n• Press [B] to Exit Building Mode")
                            .strong()
                            .color(egui::Color32::from_rgb(100, 255, 180)),
                    );
                } else {
                    ui.label(
                        egui::RichText::new("🎯 Left-Click in world to place structure\n🔄 Press [R] / [Q] / [E] to ROTATE structure 360°\n⌨ Press [B] to Exit Building Mode")
                            .strong()
                            .color(egui::Color32::from_rgb(100, 220, 255)),
                    );
                }
            });
    }
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
                    let handle_mesh = meshes.add(Cuboid::new(0.04, 0.8, 0.04));
                    let handle_mat = materials.add(StandardMaterial {
                        base_color: Color::srgb(0.42, 0.25, 0.15),
                        perceptual_roughness: 0.9,
                        ..default()
                    });
                    let handle = commands
                        .spawn((
                            Mesh3d(handle_mesh),
                            MeshMaterial3d(handle_mat),
                            Transform::from_xyz(0.0, 0.30, 0.0)
                                .with_rotation(Quat::from_rotation_x(-0.25)),
                            PlayWeaponVisual {
                                weapon_type: ActiveWeapon::Melee,
                                is_sword: false,
                            },
                            PlayModeEntity,
                        ))
                        .id();

                    let (axe_color, metallic_val, emissive_val) = match inventory.equipped_axe {
                        AxeTier::Wood => (Color::srgb(0.72, 0.75, 0.78), 0.95, LinearRgba::BLACK),
                        AxeTier::Copper => (
                            Color::srgb(0.95, 0.45, 0.22),
                            0.9,
                            LinearRgba::new(0.3, 0.1, 0.0, 1.0),
                        ),
                        AxeTier::Steel => (Color::srgb(0.65, 0.7, 0.75), 0.95, LinearRgba::BLACK),
                        AxeTier::Gold => (
                            Color::srgb(1.0, 0.82, 0.1),
                            0.9,
                            LinearRgba::new(0.5, 0.4, 0.0, 1.0),
                        ),
                        AxeTier::Platinum => (
                            Color::srgb(0.7, 0.9, 1.0),
                            1.0,
                            LinearRgba::new(1.0, 3.0, 4.0, 1.0),
                        ),
                    };

                    // 1. Central Metallic Socket Collar
                    let socket_mesh = meshes.add(Cuboid::new(0.07, 0.12, 0.07));
                    let socket_mat = materials.add(StandardMaterial {
                        base_color: Color::srgb(0.2, 0.22, 0.25),
                        metallic: 0.95,
                        perceptual_roughness: 0.2,
                        ..default()
                    });
                    let socket = commands
                        .spawn((
                            Mesh3d(socket_mesh),
                            MeshMaterial3d(socket_mat),
                            Transform::from_xyz(0.0, 0.35, 0.0),
                            PlayModeEntity,
                        ))
                        .id();
                    commands.entity(handle).add_child(socket);

                    // 2. Double-Sided Battleaxe Blades & Sharp Beveled Edges
                    let blade_mat = materials.add(StandardMaterial {
                        base_color: axe_color,
                        metallic: metallic_val,
                        perceptual_roughness: 0.15,
                        emissive: emissive_val,
                        ..default()
                    });
                    let edge_mat = materials.add(StandardMaterial {
                        base_color: Color::srgb(0.9, 0.95, 1.0),
                        metallic: 1.0,
                        perceptual_roughness: 0.05,
                        emissive: LinearRgba::new(0.5, 0.8, 1.0, 1.0),
                        ..default()
                    });

                    let crescent_mesh = meshes.add(Cuboid::new(0.18, 0.26, 0.03));
                    let edge_mesh = meshes.add(Cuboid::new(0.06, 0.28, 0.015));

                    // Left Blade & Sharp Edge
                    let left_blade = commands
                        .spawn((
                            Mesh3d(crescent_mesh.clone()),
                            MeshMaterial3d(blade_mat.clone()),
                            Transform::from_xyz(0.11, 0.35, 0.0),
                            PlayModeEntity,
                        ))
                        .id();
                    let left_edge = commands
                        .spawn((
                            Mesh3d(edge_mesh.clone()),
                            MeshMaterial3d(edge_mat.clone()),
                            Transform::from_xyz(0.21, 0.35, 0.0),
                            PlayModeEntity,
                        ))
                        .id();
                    commands.entity(handle).add_child(left_blade);
                    commands.entity(handle).add_child(left_edge);

                    // Right Symmetrical Blade & Sharp Edge (Double-Sided Battleaxe)
                    let right_blade = commands
                        .spawn((
                            Mesh3d(crescent_mesh),
                            MeshMaterial3d(blade_mat),
                            Transform::from_xyz(-0.11, 0.35, 0.0),
                            PlayModeEntity,
                        ))
                        .id();
                    let right_edge = commands
                        .spawn((
                            Mesh3d(edge_mesh),
                            MeshMaterial3d(edge_mat),
                            Transform::from_xyz(-0.21, 0.35, 0.0),
                            PlayModeEntity,
                        ))
                        .id();
                    commands.entity(handle).add_child(right_blade);
                    commands.entity(handle).add_child(right_edge);

                    handle
                }
            }
            ActiveWeapon::Pistol => commands
                .spawn((
                    WorldAssetRoot(asset_server.load("Gun_Pistol.gltf#Scene0")),
                    Transform::from_xyz(0.0, -0.04, 0.05)
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
                    Transform::from_xyz(0.0, -0.04, 0.05)
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
                    Transform::from_xyz(-0.02, -0.05, 0.08)
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
                    Transform::from_xyz(-0.02, -0.06, 0.10)
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

// System to dynamically render 3D Armor, Helmet, & Cyber Flight Suit overlays on the player character model
fn player_armor_sync_system(
    mut commands: Commands,
    inventory: Res<PlayerInventory>,
    joint_query: Query<(Entity, &PlayJointVisual)>,
    armor_query: Query<(Entity, &PlayArmorVisual)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let current_equipped = inventory.equipped_armor;

    // Check if existing armor visuals match current_equipped
    let mut already_matches = false;
    let mut to_despawn = Vec::new();
    for (entity, armor_vis) in armor_query.iter() {
        if armor_vis.armor_tier == current_equipped {
            already_matches = true;
        } else {
            to_despawn.push(entity);
        }
    }

    for entity in to_despawn {
        commands.entity(entity).despawn();
    }

    if already_matches || current_equipped == ArmorTier::None {
        return;
    }

    // Find the player's "Chest" joint node to attach the armor model
    let mut chest_entity = None;
    let mut head_entity = None;
    for (entity, joint) in joint_query.iter() {
        if joint.name == "Chest" {
            chest_entity = Some(entity);
        } else if joint.name == "Head" {
            head_entity = Some(entity);
        }
    }
    let Some(chest_ent) = chest_entity else {
        return;
    };

    // Spawn 3D armor overlay mesh based on equipped ArmorTier
    let armor_entity = match current_equipped {
        ArmorTier::None => return,
        ArmorTier::Leather => {
            let armor_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.48, 0.28, 0.15),
                perceptual_roughness: 0.85,
                ..default()
            });
            let chestplate = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.36, 0.42, 0.26))),
                    MeshMaterial3d(armor_mat.clone()),
                    Transform::from_xyz(0.0, -0.12, 0.0),
                    PlayArmorVisual {
                        armor_tier: ArmorTier::Leather,
                    },
                    PlayModeEntity,
                ))
                .id();

            // Shoulder pads
            for side in [-0.22f32, 0.22f32] {
                let pad = commands
                    .spawn((
                        Mesh3d(meshes.add(Sphere::new(0.1).mesh().ico(3).unwrap())),
                        MeshMaterial3d(armor_mat.clone()),
                        Transform::from_xyz(side, 0.06, 0.0),
                        PlayModeEntity,
                    ))
                    .id();
                commands.entity(chestplate).add_child(pad);
            }
            chestplate
        }
        ArmorTier::Copper => {
            let copper_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.92, 0.46, 0.2),
                metallic: 0.85,
                perceptual_roughness: 0.25,
                emissive: LinearRgba::new(0.3, 0.1, 0.0, 1.0),
                ..default()
            });
            let chestplate = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.38, 0.44, 0.28))),
                    MeshMaterial3d(copper_mat.clone()),
                    Transform::from_xyz(0.0, -0.12, 0.0),
                    PlayArmorVisual {
                        armor_tier: ArmorTier::Copper,
                    },
                    PlayModeEntity,
                ))
                .id();

            // Copper shoulder pauldrons
            for side in [-0.23f32, 0.23f32] {
                let pauldron = commands
                    .spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.14, 0.12, 0.18))),
                        MeshMaterial3d(copper_mat.clone()),
                        Transform::from_xyz(side, 0.06, 0.0),
                        PlayModeEntity,
                    ))
                    .id();
                commands.entity(chestplate).add_child(pauldron);
            }
            chestplate
        }
        ArmorTier::Steel => {
            let steel_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.6, 0.65, 0.7),
                metallic: 0.95,
                perceptual_roughness: 0.2,
                ..default()
            });
            let chestplate = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.4, 0.46, 0.3))),
                    MeshMaterial3d(steel_mat.clone()),
                    Transform::from_xyz(0.0, -0.12, 0.0),
                    PlayArmorVisual {
                        armor_tier: ArmorTier::Steel,
                    },
                    PlayModeEntity,
                ))
                .id();

            // Steel heavy pauldrons
            for side in [-0.24f32, 0.24f32] {
                let pauldron = commands
                    .spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.16, 0.14, 0.2))),
                        MeshMaterial3d(steel_mat.clone()),
                        Transform::from_xyz(side, 0.06, 0.0),
                        PlayModeEntity,
                    ))
                    .id();
                commands.entity(chestplate).add_child(pauldron);
            }
            chestplate
        }
        ArmorTier::Platinum => {
            let plat_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.75, 0.9, 1.0),
                metallic: 1.0,
                perceptual_roughness: 0.1,
                emissive: LinearRgba::new(0.8, 1.8, 2.5, 1.0),
                ..default()
            });
            let chestplate = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.42, 0.48, 0.32))),
                    MeshMaterial3d(plat_mat.clone()),
                    Transform::from_xyz(0.0, -0.12, 0.0),
                    PlayArmorVisual {
                        armor_tier: ArmorTier::Platinum,
                    },
                    PlayModeEntity,
                ))
                .id();

            // Platinum pauldrons with cyan trim
            for side in [-0.25f32, 0.25f32] {
                let pauldron = commands
                    .spawn((
                        Mesh3d(meshes.add(Sphere::new(0.12).mesh().ico(4).unwrap())),
                        MeshMaterial3d(plat_mat.clone()),
                        Transform::from_xyz(side, 0.06, 0.0),
                        PlayModeEntity,
                    ))
                    .id();
                commands.entity(chestplate).add_child(pauldron);
            }
            chestplate
        }
        ArmorTier::FlightSuit => {
            // Sleek High-Tech Cyber Suit body
            let cyber_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.08, 0.16, 0.24),
                metallic: 0.9,
                perceptual_roughness: 0.15,
                emissive: LinearRgba::new(0.1, 0.8, 1.5, 1.0),
                ..default()
            });
            let glow_core_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.2, 1.0, 0.8),
                emissive: LinearRgba::new(2.0, 12.0, 15.0, 1.0),
                unlit: true,
                ..default()
            });

            let suit_root = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.44, 0.5, 0.34))),
                    MeshMaterial3d(cyber_mat.clone()),
                    Transform::from_xyz(0.0, -0.12, 0.0),
                    PlayArmorVisual {
                        armor_tier: ArmorTier::FlightSuit,
                    },
                    PlayModeEntity,
                ))
                .id();

            // Glowing Arc Core Reactor on Chest
            let core = commands
                .spawn((
                    Mesh3d(meshes.add(Sphere::new(0.09).mesh().ico(4).unwrap())),
                    MeshMaterial3d(glow_core_mat.clone()),
                    Transform::from_xyz(0.0, 0.05, 0.18),
                    PlayModeEntity,
                ))
                .id();
            commands.entity(suit_root).add_child(core);

            // Cyber Shoulder Pauldrons
            for side in [-0.26f32, 0.26f32] {
                let pauldron = commands
                    .spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.16, 0.15, 0.22))),
                        MeshMaterial3d(cyber_mat.clone()),
                        Transform::from_xyz(side, 0.06, 0.0),
                        PlayModeEntity,
                    ))
                    .id();
                commands.entity(suit_root).add_child(pauldron);
            }

            // Twin Jetpack Thrusters on Back
            for side in [-0.14f32, 0.14f32] {
                let thruster = commands
                    .spawn((
                        Mesh3d(meshes.add(Cylinder::new(0.07, 0.45))),
                        MeshMaterial3d(cyber_mat.clone()),
                        Transform::from_xyz(side, 0.0, -0.22),
                        PlayModeEntity,
                    ))
                    .id();
                let nozzle = commands
                    .spawn((
                        Mesh3d(meshes.add(Sphere::new(0.05))),
                        MeshMaterial3d(glow_core_mat.clone()),
                        Transform::from_xyz(0.0, -0.24, 0.0),
                        PlayModeEntity,
                    ))
                    .id();
                commands.entity(thruster).add_child(nozzle);
                commands.entity(suit_root).add_child(thruster);
            }

            // High-Tech Cyber Flight Helmet attached to Head joint!
            if let Some(head_ent) = head_entity {
                let visor_glow_mat = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.2, 1.0, 0.85),
                    emissive: LinearRgba::new(3.0, 15.0, 18.0, 1.0),
                    unlit: true,
                    ..default()
                });

                // Cyber Helmet Dome Shell
                let helmet = commands
                    .spawn((
                        Mesh3d(meshes.add(Sphere::new(0.22).mesh().ico(4).unwrap())),
                        MeshMaterial3d(cyber_mat.clone()),
                        Transform::from_xyz(0.0, 0.04, 0.02),
                        PlayArmorVisual {
                            armor_tier: ArmorTier::FlightSuit,
                        },
                        PlayModeEntity,
                    ))
                    .id();

                // Glowing Cyber Visor
                let visor = commands
                    .spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.22, 0.08, 0.12))),
                        MeshMaterial3d(visor_glow_mat),
                        Transform::from_xyz(0.0, 0.02, 0.14),
                        PlayModeEntity,
                    ))
                    .id();
                commands.entity(helmet).add_child(visor);

                // Side Communication Fins
                for side in [-0.21f32, 0.21f32] {
                    let fin = commands
                        .spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.04, 0.14, 0.06))),
                            MeshMaterial3d(cyber_mat.clone()),
                            Transform::from_xyz(side, 0.02, -0.02),
                            PlayModeEntity,
                        ))
                        .id();
                    commands.entity(helmet).add_child(fin);
                }

                commands.entity(head_ent).add_child(helmet);
            }

            suit_root
        }
    };

    commands.entity(chest_ent).add_child(armor_entity);
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
            let need_reparent = parent.parent() != hand_entity;
            if need_reparent {
                commands.entity(hand_entity).add_child(weapon_entity);
            }
            // Restore default hand transforms
            let (offset, rot) = match visual.weapon_type {
                ActiveWeapon::Melee => {
                    if visual.is_sword {
                        (
                            Vec3::new(0.0, -0.05, 0.05),
                            Quat::from_rotation_x(std::f32::consts::FRAC_PI_2 - 0.1),
                        )
                    } else {
                        (Vec3::new(0.0, 0.30, 0.0), Quat::from_rotation_x(-0.25))
                    }
                }
                ActiveWeapon::Pistol | ActiveWeapon::Revolver => (
                    Vec3::new(0.0, -0.04, 0.05),
                    Quat::from_rotation_y(std::f32::consts::PI),
                ),
                ActiveWeapon::Rifle => (
                    Vec3::new(-0.02, -0.05, 0.08),
                    Quat::from_rotation_y(std::f32::consts::PI),
                ),
                ActiveWeapon::Sniper => (
                    Vec3::new(-0.02, -0.06, 0.10),
                    Quat::from_rotation_y(std::f32::consts::PI),
                ),
            };
            if need_reparent {
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
    inventory: Res<PlayerInventory>,
    puzzle_state: Res<crate::play_mode::house::HousePuzzleState>,
) {
    let Ok(mut cursor_options) = window_query.single_mut() else {
        return;
    };

    let ui_active = inventory.show_ship_repair_window
        || inventory.show_alien_store
        || puzzle_state.active_terminal_log.is_some()
        || puzzle_state.show_security_keypad
        || puzzle_state.show_synthesizer_ui;

    if ui_active {
        cursor_options.visible = true;
        cursor_options.grab_mode = CursorGrabMode::None;
        return;
    }

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

pub fn get_black_hole_gravity_boost(elapsed: f32) -> f32 {
    let day_speed = 0.0168;
    let master_phase = elapsed * day_speed;
    let bh_phase = master_phase + std::f32::consts::PI;
    let bh_y = bh_phase.sin() * 2800.0;
    if bh_y > 0.0 {
        (bh_y / 2800.0).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn create_accretion_disk_mesh(inner_radius: f32, outer_radius: f32, segments: usize) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    for i in 0..=segments {
        let t = i as f32 / segments as f32;
        let angle = t * std::f32::consts::TAU;
        let cos = angle.cos();
        let sin = angle.sin();

        // Inner vertex
        let ix = inner_radius * cos;
        let iz = inner_radius * sin;
        positions.push([ix, 0.0, iz]);
        normals.push([0.0, 1.0, 0.0]);
        // Map to the square texture (Cartesian)
        uvs.push([
            (ix / outer_radius) * 0.5 + 0.5,
            (iz / outer_radius) * 0.5 + 0.5,
        ]);

        // Outer vertex
        let ox = outer_radius * cos;
        let oz = outer_radius * sin;
        positions.push([ox, 0.0, oz]);
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([
            (ox / outer_radius) * 0.5 + 0.5,
            (oz / outer_radius) * 0.5 + 0.5,
        ]);
    }

    for i in 0..segments {
        let i0 = (i * 2) as u32;
        let i1 = i0 + 1;
        let i2 = i0 + 2;
        let i3 = i0 + 3;

        indices.extend_from_slice(&[i0, i1, i2]);
        indices.extend_from_slice(&[i1, i3, i2]);
    }

    let mut mesh = Mesh::new(
        bevy::render::render_resource::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::mesh::Indices::U32(indices));
    mesh
}

fn generate_accretion_disk_texture(perlin: &crate::map_editor::noise::PerlinNoise) -> Image {
    let size = 256;
    let mut data = vec![0u8; size * size * 4];
    let center = size as f32 / 2.0;

    for y in 0..size {
        for x in 0..size {
            let idx = (y * size + x) * 4;
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let radius = (dx * dx + dy * dy).sqrt() / center;
            let angle = dy.atan2(dx);

            // Only draw the ring
            if !(0.18..=0.95).contains(&radius) {
                data[idx] = 0;
                data[idx + 1] = 0;
                data[idx + 2] = 0;
                data[idx + 3] = 0;
                continue;
            }

            // Smooth radial falloff (soft edges)
            let inner = ((radius - 0.18) / 0.10).clamp(0.0, 1.0);
            let outer = ((0.95 - radius) / 0.12).clamp(0.0, 1.0);
            let radial_falloff = (inner * outer).powf(0.7);

            // Mild spiral arms (much less aggressive so it stays full)
            let spiral = angle * 2.5 + (1.0 / (radius + 0.08)) * 1.8;
            let noise1 = perlin.noise(spiral.cos() * 3.0, spiral.sin() * 3.0);
            let noise2 = perlin.noise(radius * 9.0, angle * 2.0);
            let noise_val = (noise1 * 0.55 + noise2 * 0.45) * 0.5 + 0.5;

            // Concentric energy bands
            let ring_band = ((radius * 22.0).sin() * 0.5 + 0.5).powf(1.3);

            // Final intensity – keeps the ring more uniform
            let intensity =
                (radial_falloff * (0.45 + noise_val * 0.35 + ring_band * 0.25)).clamp(0.0, 1.0);

            // Color gradient: hot white-yellow core → orange → deep red
            let (r, g, b) = if intensity > 0.72 {
                (255, 240, 190)
            } else if intensity > 0.40 {
                (
                    255,
                    (120.0 + intensity * 110.0) as u8,
                    (20.0 + intensity * 40.0) as u8,
                )
            } else {
                (
                    (intensity * 380.0).min(255.0) as u8,
                    (intensity * 90.0) as u8,
                    8,
                )
            };

            let alpha = (intensity * 255.0) as u8;

            data[idx] = r;
            data[idx + 1] = g;
            data[idx + 2] = b;
            data[idx + 3] = alpha;
        }
    }

    Image::new(
        bevy::render::render_resource::Extent3d {
            width: size as u32,
            height: size as u32,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::default(),
    )
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn play_sky_cycle_system(
    time: Res<Time>,
    player_query: Query<&Transform, With<PlayModePlayer>>,
    mut sun_query: Query<
        (
            &mut Transform,
            &PlaySun,
            &mut Visibility,
            Option<&mut DirectionalLight>,
        ),
        (
            Without<PlayModePlayer>,
            Without<PlayNightPlanet>,
            Without<PlayPlanetRings>,
            Without<PlayBlackHoleMoon>,
            Without<PlayBlackHoleDiskHoriz>,
        ),
    >,
    mut cloud_query: Query<
        &mut Visibility,
        (
            With<PlayModeCloud>,
            Without<PlaySun>,
            Without<PlayModePlayer>,
            Without<PlayNightPlanet>,
            Without<PlayPlanetRings>,
            Without<PlayBlackHoleMoon>,
            Without<PlayBlackHoleDiskHoriz>,
        ),
    >,
    mut clear_color: ResMut<ClearColor>,
    mut camera_query: Query<&mut DistanceFog, With<PlayModeCamera>>,
    mut night_sky_query: Query<
        (
            &mut Transform,
            &mut Visibility,
            Option<&PlayNightPlanet>,
            Option<&PlayPlanetRings>,
            Option<&PlayBlackHoleMoon>,
            Option<&PlayBlackHoleDiskHoriz>,
        ),
        (
            Or<(
                With<PlayNightPlanet>,
                With<PlayPlanetRings>,
                With<PlayBlackHoleMoon>,
                With<PlayBlackHoleDiskHoriz>,
            )>,
            Without<PlayModePlayer>,
            Without<PlaySun>,
            Without<PlayModeCloud>,
        ),
    >,
) {
    let elapsed = time.elapsed_secs();
    let player_center = player_query
        .single()
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);
    let is_underground = player_center.y < -30.0;

    // Day cycle speed (one full cycle takes 374 seconds ~ 6.2 minutes)
    let day_speed = 0.0168;
    let master_phase = elapsed * day_speed;

    let mut highest_sun_y = -999.0;

    for (mut transform, sun, mut visibility, opt_light) in sun_query.iter_mut() {
        let phase = master_phase * sun.orbit_speed + sun.angle_offset;
        let radius = 2500.0;

        let x = phase.cos() * radius;
        let y = phase.sin() * radius;
        let z = (phase * 0.5).cos() * radius * 0.4;

        transform.translation = player_center + Vec3::new(x, y, z);

        if is_underground || y < -200.0 {
            *visibility = Visibility::Hidden;
        } else {
            *visibility = Visibility::Inherited;
        }

        if y > highest_sun_y {
            highest_sun_y = y;
        }

        if let Some(mut light) = opt_light {
            let target_dir = (player_center - transform.translation).normalize_or_zero();
            let up = if target_dir.y.abs() > 0.99 {
                Vec3::Z
            } else {
                Vec3::Y
            };
            transform.look_at(player_center, up);

            let day_factor = (y / 200.0).clamp(0.0, 1.0); // smooth 0.0 to 1.0 day transition
            if is_underground || day_factor <= 0.001 {
                light.illuminance = 0.0;
                light.shadow_maps_enabled = false;
            } else {
                light.illuminance = day_factor * sun.day_intensity;
                light.shadow_maps_enabled = true;
            }
        }
    }

    // Black Hole centered in the night sky (opposite to Sun, i.e. phase offset is PI)
    let bh_phase = master_phase * 1.0 + std::f32::consts::PI;
    let bh_orbit_radius = 2800.0;
    let bh_x = bh_phase.cos() * bh_orbit_radius;
    let bh_y = bh_phase.sin() * bh_orbit_radius;
    let bh_z = (bh_phase * 0.5).cos() * bh_orbit_radius * 0.3;
    let bh_center = player_center + Vec3::new(bh_x, bh_y, bh_z);

    // Gas Giant Planet orbits the Black Hole!
    let planet_orbit_phase = elapsed * 0.20; // slow orbit around black hole
    let planet_orbit_radius = 520.0; // wide orbit to prevent overlap and show transit/lensing
    let pox = planet_orbit_phase.cos() * planet_orbit_radius;
    let poy = planet_orbit_phase.sin() * planet_orbit_radius;
    let poz = (planet_orbit_phase * 0.7).cos() * planet_orbit_radius * 0.45;
    let planet_pos = bh_center + Vec3::new(pox, poy, poz);

    // Compute gravitational lensing alignment (magnification/shear when passing behind black hole)
    let dir_bh = (bh_center - player_center).normalize_or_zero();
    let dir_planet = (planet_pos - player_center).normalize_or_zero();
    let alignment = dir_bh.dot(dir_planet);
    let lens_factor = if alignment > 0.88 {
        1.0 + ((alignment - 0.88) / 0.12).clamp(0.0, 1.0) * 0.45
    } else {
        1.0
    };

    // Update Planet, Rings, Black Hole Moon, Accretion Disk
    for (mut transform, mut visibility, opt_planet, opt_rings, opt_bh, opt_disk_horiz) in
        night_sky_query.iter_mut()
    {
        if is_underground || bh_y < -300.0 {
            *visibility = Visibility::Hidden;
        } else {
            *visibility = Visibility::Inherited;
        }

        if opt_planet.is_some() {
            transform.translation = planet_pos;
            transform.rotation = Quat::from_rotation_x(0.2) * Quat::from_rotation_y(elapsed * 0.15);
            transform.scale = Vec3::splat(lens_factor);
        } else if opt_rings.is_some() {
            transform.translation = planet_pos;
            transform.rotation =
                Quat::from_rotation_x(0.45) * Quat::from_rotation_y(elapsed * 0.08);
            transform.scale = Vec3::splat(lens_factor);
        } else if opt_bh.is_some() {
            transform.translation = bh_center;
            transform.scale = Vec3::ONE;
        } else if opt_disk_horiz.is_some() {
            let dir_to_player = (player_center - bh_center).normalize_or_zero();
            transform.translation = bh_center + dir_to_player * 25.0;
            transform.rotation =
                Quat::from_rotation_x(0.35) * Quat::from_rotation_y(-elapsed * 0.15);
            transform.scale = Vec3::ONE;
        }
    }

    for mut cloud_vis in cloud_query.iter_mut() {
        if is_underground {
            *cloud_vis = Visibility::Hidden;
        } else {
            *cloud_vis = Visibility::Inherited;
        }
    }

    // Set clear color based on highest sun height (day/night transition)
    let sky_factor = (highest_sun_y / 2500.0).clamp(-0.5, 1.0);
    let night_linear = Color::srgb(0.04, 0.03, 0.08).to_linear();
    let twilight_linear = Color::srgb(0.35, 0.12, 0.28).to_linear();
    let day_linear = Color::srgb(0.18, 0.22, 0.45).to_linear();

    let current_linear = if is_underground {
        night_linear
    } else if sky_factor < 0.0 {
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
                let hand_pos = player.nodes[15].position;
                let aim_target = cam_transform.translation + cam_transform.forward() * 100.0;
                let shoot_dir = (aim_target - hand_pos).normalize_or_zero();
                let start_pos = hand_pos + shoot_dir * 0.4;
                let forward = shoot_dir;

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
                creatures::CreatureType::Monster => (1.6, 2.4),
                creatures::CreatureType::Bird => (0.0, 0.7),
                creatures::CreatureType::BigBird => (0.0, 1.5),
                creatures::CreatureType::Triangaroo => (0.6, 1.0),
                creatures::CreatureType::Polypug => (0.4, 0.8),
                creatures::CreatureType::Fox => (0.0, 1.4),
                creatures::CreatureType::Alien => (0.3, 1.1),
                creatures::CreatureType::RobotTrilobite => (0.3, 0.9),
            };

            let center = c_transform.translation + Vec3::Y * center_offset;
            let seg = new_pos - old_pos;
            let seg_len_sq = seg.length_squared();
            let dist = if seg_len_sq < 1e-6 {
                new_pos.distance(center)
            } else {
                let t = ((center - old_pos).dot(seg) / seg_len_sq).clamp(0.0, 1.0);
                let proj = old_pos + seg * t;
                proj.distance(center)
            };

            if dist < radius
                || old_pos.distance(center) < radius
                || new_pos.distance(center) < radius
            {
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

                // Spawn a physical ammo and loot drop box or a health pack!
                let drop_pos = Vec3::new(c_pos.x, c_pos.y + 0.3, c_pos.z);

                let wood_loot = 1 + (rand::random::<f32>() * 3.0) as u32; // 1 to 3
                let copper_loot = (rand::random::<f32>() * 3.0) as u32; // 0 to 2
                let iron_loot = (rand::random::<f32>() * 2.0) as u32; // 0 to 1

                if rand::random::<f32>() < 0.4 {
                    // Spawn a Health Pack!
                    commands.spawn((
                        WorldAssetRoot(asset_server.load("Prop_HealthPack.gltf#Scene0")),
                        Transform::from_translation(drop_pos).with_scale(Vec3::splat(1.5)),
                        AmmoDrop {
                            health_heal: 35.0,
                            ..default()
                        },
                        SpinDrop,
                        PlayModeEntity,
                    ));
                    inventory_log("💀 Struck creature down! Health Pack Drop Spawned!");
                } else {
                    // Spawn a standard Ammo/Loot Drop!
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
                            health_heal: 0.0,
                            ..default()
                        },
                        SpinDrop,
                        PlayModeEntity,
                    ));
                    inventory_log("💀 Struck creature down! Ammo Drop Spawned!");
                }
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
            inventory.fox_pelt += drop.fox_pelt;
            inventory.alien_pelt += drop.alien_pelt;
            inventory.kangaroo_fur += drop.kangaroo_fur;
            inventory.alien_feather += drop.alien_feather;
            inventory.monster_core += drop.monster_core;
            inventory.alien_tech += drop.alien_tech;
            inventory.robot_parts += drop.robot_parts;

            if drop.health_heal > 0.0 {
                player.health = (player.health + drop.health_heal).min(player.max_health);
            }

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
            if drop.fox_pelt > 0 {
                items.push(format!("+{} Fox Pelt", drop.fox_pelt));
            }
            if drop.alien_pelt > 0 {
                items.push(format!("+{} Alien Pelt", drop.alien_pelt));
            }
            if drop.kangaroo_fur > 0 {
                items.push(format!("+{} Kangaroo Fur", drop.kangaroo_fur));
            }
            if drop.alien_feather > 0 {
                items.push(format!("+{} Alien Feather", drop.alien_feather));
            }
            if drop.monster_core > 0 {
                items.push(format!("+{} Monster Core", drop.monster_core));
            }
            if drop.alien_tech > 0 {
                items.push(format!("+{} Alien Tech", drop.alien_tech));
            }
            if drop.robot_parts > 0 {
                items.push(format!("+{} Robot Parts", drop.robot_parts));
            }
            if drop.health_heal > 0.0 {
                items.push(format!("+{} HP", drop.health_heal as u32));
            }

            inventory_log(&format!("🎒 Acquired Loot Drop: {}", items.join(", ")));

            // Play satisfying pickup sound
            commands.spawn((
                AudioPlayer::new(asset_server.load("chest_open.wav")),
                PlaybackSettings::DESPAWN,
            ));

            commands.entity(entity).despawn();
        }
    }
}

// 3D Crashed Starship Wreckage at Player Surface Spawn Location
fn spawn_crashed_starship(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    spawn_pos: Vec3,
    map: &TempestMap,
) {
    let crash_x = spawn_pos.x + 8.0;
    let crash_z = spawn_pos.z + 10.0;
    let terrain_y = get_bilinear_height(crash_x, crash_z, map);
    let crash_pos = Vec3::new(crash_x, terrain_y - 0.5, crash_z);

    // 1. Scorched Crash Site Crater Base
    let crater_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.1, 0.08),
        perceptual_roughness: 0.95,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(4.8).mesh().uv(16, 8))),
        MeshMaterial3d(crater_mat),
        Transform::from_translation(crash_pos + Vec3::new(0.0, -4.2, 0.0))
            .with_scale(Vec3::new(1.4, 0.25, 1.8)),
        StarshipDebris,
        PlayModeEntity,
    ));

    // 2. Tilted Crashed Starship Hull Root Entity
    let ship_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.14, 0.18, 0.25), // Dark titanium alloy
        metallic: 0.9,
        perceptual_roughness: 0.25,
        ..default()
    });
    let glow_trim_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 0.8, 1.0),
        emissive: LinearRgba::new(0.5, 3.5, 5.0, 1.0),
        unlit: true,
        ..default()
    });

    let ship_root =
        commands
            .spawn((
                Mesh3d(meshes.add(Cuboid::new(3.6, 2.2, 8.5))),
                MeshMaterial3d(ship_mat.clone()),
                Transform::from_translation(crash_pos + Vec3::Y * 0.8)
                    .with_rotation(Quat::from_euler(EulerRot::XYZ, 0.22, 0.45, -0.15)),
                CrashedStarship {
                    is_repaired: false,
                    flight_speed: 0.0,
                    yaw: 0.0,
                    pitch: 0.0,
                    roll: 0.0,
                },
                PlayModeEntity,
            ))
            .id();

    // 3. Translucent Cockpit Visor Canopy
    let canopy_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.2, 0.8, 1.0, 0.45),
        alpha_mode: AlphaMode::Blend,
        emissive: LinearRgba::new(0.2, 1.5, 2.0, 1.0),
        ..default()
    });
    let canopy = commands
        .spawn((
            Mesh3d(meshes.add(Sphere::new(1.3).mesh().ico(4).unwrap())),
            MeshMaterial3d(canopy_mat),
            Transform::from_xyz(0.0, 0.6, 2.8),
            PlayModeEntity,
        ))
        .id();
    commands.entity(ship_root).add_child(canopy);

    // 4. Intact Swept Left Wing
    let wing_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.16, 0.2, 0.28),
        metallic: 0.95,
        perceptual_roughness: 0.3,
        ..default()
    });
    let left_wing = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(4.8, 0.16, 2.4))),
            MeshMaterial3d(wing_mat.clone()),
            Transform::from_xyz(-3.6, 0.3, -0.4).with_rotation(Quat::from_rotation_z(0.2)),
            PlayModeEntity,
        ))
        .id();
    commands.entity(ship_root).add_child(left_wing);

    // 5. Damaged/Broken Right Wing
    let broken_wing = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(2.2, 0.16, 2.0))),
            MeshMaterial3d(wing_mat),
            Transform::from_xyz(2.2, -0.4, 0.2).with_rotation(Quat::from_rotation_z(-0.35)),
            StarshipBrokenWing,
            PlayModeEntity,
        ))
        .id();
    commands.entity(ship_root).add_child(broken_wing);

    // 6. Dual Damaged Rear Plasma Engines
    for side in [-1.1f32, 1.1f32] {
        let engine = commands
            .spawn((
                Mesh3d(meshes.add(Cylinder::new(0.65, 2.2))),
                MeshMaterial3d(ship_mat.clone()),
                Transform::from_xyz(side, 0.2, -4.2)
                    .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                PlayModeEntity,
            ))
            .id();
        let nozzle = commands
            .spawn((
                Mesh3d(meshes.add(Sphere::new(0.48))),
                MeshMaterial3d(glow_trim_mat.clone()),
                Transform::from_xyz(0.0, -1.1, 0.0),
                PlayModeEntity,
            ))
            .id();
        commands.entity(engine).add_child(nozzle);
        commands.entity(ship_root).add_child(engine);
    }

    // 7. Scattered Crash Debris
    let panel_mesh = meshes.add(Cuboid::new(0.8, 0.05, 1.0));
    for i in 0..6 {
        let dx = (i as f32 * 1.5 - 4.5) * 1.2;
        let dz = (i as f32 * 2.1 - 5.0) * 1.1;
        let d_terrain_y = get_bilinear_height(crash_x + dx, crash_z + dz, map);
        commands.spawn((
            Mesh3d(panel_mesh.clone()),
            MeshMaterial3d(ship_mat.clone()),
            Transform::from_xyz(crash_x + dx, d_terrain_y + 0.05, crash_z + dz)
                .with_rotation(Quat::from_rotation_y(i as f32 * 1.1)),
            StarshipDebris,
            PlayModeEntity,
        ));
    }

    // 8. Emergency Crash Terminal Console
    let console_pos = Vec3::new(crash_x - 3.2, terrain_y, crash_z + 2.0);
    let console_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.25, 0.3),
        metallic: 0.8,
        ..default()
    });
    let console_screen = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 0.9, 0.7),
        emissive: LinearRgba::new(0.0, 5.0, 6.0, 1.0),
        unlit: true,
        ..default()
    });

    let console_root = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.9, 1.2, 0.6))),
            MeshMaterial3d(console_mat),
            Transform::from_translation(console_pos),
            CrashedStarshipConsoleMarker,
            PlayModeEntity,
        ))
        .id();

    let screen = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.7, 0.45, 0.05))),
            MeshMaterial3d(console_screen),
            Transform::from_xyz(0.0, 0.4, 0.3).with_rotation(Quat::from_rotation_x(-0.3)),
            PlayModeEntity,
        ))
        .id();
    commands.entity(console_root).add_child(screen);
}

/// System managing Starfighter visual restoration upon repair and movement sync during flight
#[allow(clippy::too_many_arguments)]
pub fn starship_visual_sync_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    inventory: Res<PlayerInventory>,
    player_query: Query<&PlayModePlayer>,
    mut starship_query: Query<
        (Entity, &mut Transform, &mut CrashedStarship),
        Without<PlayModeCamera>,
    >,
    debris_query: Query<Entity, With<StarshipDebris>>,
    broken_wing_query: Query<Entity, With<StarshipBrokenWing>>,
    map: Res<TempestMap>,
) {
    let Ok(player) = player_query.single() else {
        return;
    };

    for (ship_entity, mut ship_transform, mut starship) in starship_query.iter_mut() {
        // If starship restoration is complete in inventory, update the visual model!
        if !starship.is_repaired && inventory.starship_repaired {
            starship.is_repaired = true;

            for entity in debris_query.iter() {
                commands.entity(entity).despawn();
            }
            for entity in broken_wing_query.iter() {
                commands.entity(entity).despawn();
            }

            // Spawn intact right wing matching left wing
            let wing_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.16, 0.2, 0.28),
                metallic: 0.95,
                perceptual_roughness: 0.3,
                ..default()
            });
            let right_wing = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(4.8, 0.16, 2.4))),
                    MeshMaterial3d(wing_mat),
                    Transform::from_xyz(3.6, 0.3, -0.4).with_rotation(Quat::from_rotation_z(-0.2)),
                    StarshipRepairedWing,
                    PlayModeEntity,
                ))
                .id();
            commands.entity(ship_entity).add_child(right_wing);

            // Spawn Dual Wingtip Plasma Cannon Barrels
            let cannon_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.1, 0.8, 1.0),
                metallic: 0.9,
                emissive: LinearRgba::new(0.5, 4.0, 6.0, 1.0),
                ..default()
            });
            let cannon_l = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.2, 0.2, 1.2))),
                    MeshMaterial3d(cannon_mat.clone()),
                    Transform::from_xyz(-5.8, 0.3, -0.4),
                    PlayModeEntity,
                ))
                .id();
            let cannon_r = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.2, 0.2, 1.2))),
                    MeshMaterial3d(cannon_mat),
                    Transform::from_xyz(5.8, 0.3, -0.4),
                    PlayModeEntity,
                ))
                .id();
            commands
                .entity(ship_entity)
                .add_child(cannon_l)
                .add_child(cannon_r);

            // Re-orient starship root to level upright posture on terrain
            let ground_y = get_bilinear_height(
                ship_transform.translation.x,
                ship_transform.translation.z,
                &map,
            );
            ship_transform.rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
            ship_transform.translation.y = ground_y + 1.0;
        }

        if player.state == PlayerState::PilotingStarship {
            ship_transform.translation = player.position;

            let yaw_rot =
                Quat::from_rotation_y(std::f32::consts::FRAC_PI_2 - player.rotation_yaw);
            let pitch_angle = if keyboard_input.pressed(KeyCode::Space) {
                0.18
            } else if keyboard_input.pressed(KeyCode::ControlLeft)
                || keyboard_input.pressed(KeyCode::KeyC)
            {
                -0.18
            } else {
                0.0
            };
            let bank_angle = if keyboard_input.pressed(KeyCode::KeyA) {
                0.22
            } else if keyboard_input.pressed(KeyCode::KeyD) {
                -0.22
            } else {
                0.0
            };
            let flight_tilt = Quat::from_euler(EulerRot::ZXY, bank_angle, 0.0, pitch_angle);
            ship_transform.rotation = yaw_rot * flight_tilt;

            // Spawn nitro boost exhaust particles when Shift is held
            if keyboard_input.pressed(KeyCode::ShiftLeft)
                || keyboard_input.pressed(KeyCode::ShiftRight)
            {
                let forward = Vec3::new(player.rotation_yaw.cos(), 0.0, player.rotation_yaw.sin());
                let tail_pos = player.position - forward * 4.2 + Vec3::Y * 0.2;

                let p_mesh = meshes.add(Sphere::new(0.25));
                let p_mat = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.1, 0.9, 1.0),
                    emissive: LinearRgba::new(1.0, 8.0, 12.0, 1.0),
                    unlit: true,
                    ..default()
                });
                commands.spawn((
                    Mesh3d(p_mesh),
                    MeshMaterial3d(p_mat),
                    Transform::from_translation(tail_pos),
                    PlayParticle {
                        velocity: -forward * 25.0
                            + Vec3::new(
                                (rand::random::<f32>() - 0.5) * 2.0,
                                (rand::random::<f32>() - 0.5) * 2.0,
                                (rand::random::<f32>() - 0.5) * 2.0,
                            ),
                        lifetime: 0.0,
                        max_lifetime: 0.3,
                        color: Color::srgb(0.1, 0.9, 1.0),
                    },
                    PlayModeEntity,
                ));
            }
        }
    }
}

/// System managing Starfighter Dual Plasma Cannon bolts and collision impact damage
pub fn starship_plasma_bolt_system(
    mut commands: Commands,
    time: Res<Time>,
    mut bolt_query: Query<(Entity, &mut Transform, &mut StarshipPlasmaBolt)>,
    mut creature_query: Query<
        (&mut creatures::PlayCreature, &Transform),
        Without<StarshipPlasmaBolt>,
    >,
    map: Res<TempestMap>,
) {
    let dt = time.delta_secs();
    for (entity, mut transform, mut bolt) in bolt_query.iter_mut() {
        bolt.lifetime -= dt;
        if bolt.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        transform.translation += bolt.velocity * dt;
        let pos = transform.translation;
        let ground_y = get_bilinear_height(pos.x, pos.z, &map);

        // Ground Impact
        if pos.y <= ground_y + 0.2 {
            commands.entity(entity).despawn();
            continue;
        }

        // Creature Hit Detection
        for (mut creature, c_trans) in creature_query.iter_mut() {
            if creature.state != creatures::CreatureState::Dead
                && pos.distance(c_trans.translation) < 2.5
            {
                creature.health -= 75.0;
                if creature.health <= 0.0 {
                    creature.state = creatures::CreatureState::Dead;
                }
                commands.entity(entity).despawn();
                break;
            }
        }
    }
}

/// Building placement system for procedural structures
#[allow(clippy::too_many_arguments)]
fn building_placement_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut building_state: ResMut<structures::BuildingPlacementState>,
    player_query: Query<(&Transform, &PlayModePlayer)>,
    camera_query: Query<&Transform, With<PlayModeCamera>>,
    ghost_query: Query<Entity, With<structures::PlacementPreviewGhost>>,
    mut wall_builder: ResMut<crate::procedural_walls::ProceduralWallBuilder>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyB) {
        building_state.is_active = !building_state.is_active;
        if building_state.is_active {
            inventory_log("🏗️ Building Mode Activated! Press [1..=9] to select structure.");
        } else {
            inventory_log("🏗️ Building Mode Deactivated.");
            wall_builder.active = false;
            wall_builder.points.clear();
            for ghost in ghost_query.iter() {
                commands.entity(ghost).despawn();
            }
        }
    }

    if !building_state.is_active {
        wall_builder.active = false;
        for ghost in ghost_query.iter() {
            commands.entity(ghost).despawn();
        }
        return;
    }

    let prev_structure = building_state.selected_structure;

    // Number keys 1..=9 select structure
    if keyboard_input.just_pressed(KeyCode::Digit1) {
        building_state.selected_structure = structures::StructureType::ClassicBrickWall;
        inventory_log(
            "🏗️ Selected: 🧱 Multi-Point Procedural Brick Wall [Click points, Enter to build]",
        );
    } else if keyboard_input.just_pressed(KeyCode::Digit2) {
        building_state.selected_structure = structures::StructureType::Watchtower;
        inventory_log("🏗️ Selected: 🗼 Fortified Watchtower [R / Q / E to rotate]");
    } else if keyboard_input.just_pressed(KeyCode::Digit3) {
        building_state.selected_structure = structures::StructureType::Staircase;
        inventory_log("🏗️ Selected: 🪜 Modular Staircase [R / Q / E to rotate]");
    } else if keyboard_input.just_pressed(KeyCode::Digit4) {
        building_state.selected_structure = structures::StructureType::Ramp;
        inventory_log("🏗️ Selected: 📐 Inclined Ramp [R / Q / E to rotate]");
    } else if keyboard_input.just_pressed(KeyCode::Digit5) {
        building_state.selected_structure = structures::StructureType::WoodenBridge;
        inventory_log("🏗️ Selected: 🌉 Wooden Plank Bridge [R / Q / E to rotate]");
    } else if keyboard_input.just_pressed(KeyCode::Digit6) {
        building_state.selected_structure = structures::StructureType::PalisadeFence;
        inventory_log(
            "🏗️ Selected: 🪵 Multi-Point Palisade Stake Fence [Click points, Enter to build]",
        );
    } else if keyboard_input.just_pressed(KeyCode::Digit7) {
        building_state.selected_structure = structures::StructureType::GraniteFortressWall;
        inventory_log(
            "🏗️ Selected: 🧱 Multi-Point Granite Fortress Wall [Click points, Enter to build]",
        );
    } else if keyboard_input.just_pressed(KeyCode::Digit8) {
        building_state.selected_structure = structures::StructureType::LogTimberWall;
        inventory_log("🏗️ Selected: 🪵 Multi-Point Log Cabin Wall [Click points, Enter to build]");
    } else if keyboard_input.just_pressed(KeyCode::Digit9) {
        building_state.selected_structure = structures::StructureType::CyberMetalWall;
        inventory_log(
            "🏗️ Selected: ⚡ Multi-Point Cyber Metal Wall [Click points, Enter to build]",
        );
    }

    // 1. Rotation Key Controls: R, Q, E, Left / Right Arrows
    let shift_held =
        keyboard_input.pressed(KeyCode::ShiftLeft) || keyboard_input.pressed(KeyCode::ShiftRight);
    let mut rotation_changed = false;
    if keyboard_input.just_pressed(KeyCode::KeyR) {
        if shift_held {
            building_state.rotation_yaw -= std::f32::consts::FRAC_PI_4;
        } else {
            building_state.rotation_yaw += std::f32::consts::FRAC_PI_4;
        }
        rotation_changed = true;
    } else if keyboard_input.just_pressed(KeyCode::KeyQ) {
        building_state.rotation_yaw -= std::f32::consts::FRAC_PI_4;
        rotation_changed = true;
    } else if keyboard_input.just_pressed(KeyCode::KeyE) {
        building_state.rotation_yaw += std::f32::consts::FRAC_PI_4;
        rotation_changed = true;
    }

    // 2. Handle Multi-Point Procedural Wall Builder Mode for all 5 Wall Styles
    let procedural_style = match building_state.selected_structure {
        structures::StructureType::ClassicBrickWall => {
            Some(crate::procedural_walls::WallStyle::ClassicBrick)
        }
        structures::StructureType::PalisadeFence => {
            Some(crate::procedural_walls::WallStyle::PalisadeFence)
        }
        structures::StructureType::GraniteFortressWall => {
            Some(crate::procedural_walls::WallStyle::GraniteFortress)
        }
        structures::StructureType::LogTimberWall => {
            Some(crate::procedural_walls::WallStyle::LogTimber)
        }
        structures::StructureType::CyberMetalWall => {
            Some(crate::procedural_walls::WallStyle::CyberMetal)
        }
        _ => None,
    };

    if let Some(style) = procedural_style {
        wall_builder.active = true;
        if wall_builder.style != style {
            wall_builder.style = style;
            wall_builder.points.clear();
        }
        for ghost in ghost_query.iter() {
            commands.entity(ghost).despawn();
        }
        return;
    } else {
        wall_builder.active = false;
        wall_builder.points.clear();
    }

    let Ok((_player_transform, player)) = player_query.single() else {
        return;
    };
    let Ok(cam_transform) = camera_query.single() else {
        return;
    };

    let cam_forward =
        Vec3::new(cam_transform.forward().x, 0.0, cam_transform.forward().z).normalize_or_zero();
    let place_dist = 4.0;
    let place_pos = player.position + cam_forward * place_dist;
    let place_rot = Quat::from_rotation_y(-player.rotation_yaw + building_state.rotation_yaw);

    // Re-spawn or update ghost preview model when selection or rotation changes
    if ghost_query.is_empty()
        || prev_structure != building_state.selected_structure
        || rotation_changed
    {
        for ghost in ghost_query.iter() {
            commands.entity(ghost).despawn();
        }
        structures::spawn_preview_ghost_model(
            &mut commands,
            &mut meshes,
            &mut materials,
            building_state.selected_structure,
            place_pos,
            place_rot,
        );
    } else {
        for ghost in ghost_query.iter() {
            commands
                .entity(ghost)
                .insert(Transform::from_translation(place_pos).with_rotation(place_rot));
        }
    }

    // Place modular structure on Left-Click
    if mouse_button.just_pressed(MouseButton::Left) {
        structures::spawn_procedural_structure(
            &mut commands,
            &mut meshes,
            &mut materials,
            &asset_server,
            building_state.selected_structure,
            place_pos,
            place_rot,
        );
        inventory_log(&format!(
            "🏗️ Constructed {}!",
            building_state.selected_structure.name()
        ));
        commands.spawn((
            AudioPlayer::new(asset_server.load("chest_open.wav")),
            PlaybackSettings::DESPAWN,
        ));
    }
}
