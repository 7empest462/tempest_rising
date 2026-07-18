use crate::map_editor::data::TempestMap;
use crate::play_mode::{
    PlayModeEntity, PlayModePlayer, get_bilinear_height, get_effective_floor_height,
};
use bevy::animation::prelude::*;
use bevy::gltf::GltfAssetLabel;
use bevy::prelude::WorldAssetRoot;
use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Reflect)]
pub enum CreatureType {
    Triangaroo,
    Polypug,
    Bird,
    Monster,
    Fox,
    Alien,
    RobotTrilobite,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Reflect)]
pub enum CreatureState {
    Idle,
    Wandering,
    Chasing,
    Attacking,
    Dead,
}

#[derive(Component)]
pub struct PlayCreature {
    pub creature_type: CreatureType,
    pub state: CreatureState,
    pub health: f32,
    pub max_health: f32,
    pub position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub wander_timer: f32,
    pub hop_cooldown: f32,
    pub is_grounded: bool,
    pub death_timer: f32,
    pub attack_cooldown: f32,
}

#[derive(Component)]
pub struct ProceduralWing {
    pub is_left: bool,
}

#[derive(Component)]
pub struct RestPose(pub Quat);

/// Marks a creature as allied to the player (won't be damaged by player bullets).
#[derive(Component)]
pub struct PlayerDefender;

/// Tracks aggro state for neutral NPCs (Aliens).
#[derive(Component)]
pub struct AggroState {
    pub aggro_timer: f32,
    pub is_provoked: bool,
}

// ──────────────────────────────────────────────
// Animation resources for GLTF-animated creatures
// ──────────────────────────────────────────────

#[derive(Resource)]
pub struct FoxAnimations {
    pub graph: Handle<AnimationGraph>,
    pub survey: AnimationNodeIndex,
    pub walk: AnimationNodeIndex,
    pub run: AnimationNodeIndex,
}

#[derive(Resource)]
pub struct TrilobiteAnimations {
    pub graph: Handle<AnimationGraph>,
    pub idle: AnimationNodeIndex,
    pub walk: AnimationNodeIndex,
    pub run: AnimationNodeIndex,
    pub attack: AnimationNodeIndex,
}

// ──────────────────────────────────────────────
// Animation setup systems (OnEnter PlayMode)
// ──────────────────────────────────────────────

pub fn setup_fox_animations(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let mut graph = AnimationGraph::new();
    let survey = graph.add_clip(
        asset_server.load(GltfAssetLabel::Animation(0).from_asset("Fox.glb")),
        1.0,
        graph.root,
    );
    let walk = graph.add_clip(
        asset_server.load(GltfAssetLabel::Animation(1).from_asset("Fox.glb")),
        1.0,
        graph.root,
    );
    let run = graph.add_clip(
        asset_server.load(GltfAssetLabel::Animation(2).from_asset("Fox.glb")),
        1.0,
        graph.root,
    );
    let graph_handle = graphs.add(graph);
    commands.insert_resource(FoxAnimations {
        graph: graph_handle,
        survey,
        walk,
        run,
    });
}

pub fn setup_trilobite_animations(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let mut graph = AnimationGraph::new();
    let attack = graph.add_clip(
        asset_server.load(GltfAssetLabel::Animation(0).from_asset("Enemy_Trilobite.gltf")),
        1.0,
        graph.root,
    );
    let idle = graph.add_clip(
        asset_server.load(GltfAssetLabel::Animation(4).from_asset("Enemy_Trilobite.gltf")),
        1.0,
        graph.root,
    );
    let run = graph.add_clip(
        asset_server.load(GltfAssetLabel::Animation(6).from_asset("Enemy_Trilobite.gltf")),
        1.0,
        graph.root,
    );
    let walk = graph.add_clip(
        asset_server.load(GltfAssetLabel::Animation(8).from_asset("Enemy_Trilobite.gltf")),
        1.0,
        graph.root,
    );
    let graph_handle = graphs.add(graph);
    commands.insert_resource(TrilobiteAnimations {
        graph: graph_handle,
        idle,
        walk,
        run,
        attack,
    });
}

// ──────────────────────────────────────────────
// Animation attachment systems (catch newly-loaded AnimationPlayers)
// ──────────────────────────────────────────────

pub fn attach_fox_animation_player(
    mut commands: Commands,
    fox_anims: Option<Res<FoxAnimations>>,
    mut new_players: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>,
    parent_query: Query<&ChildOf>,
    creature_query: Query<&PlayCreature>,
) {
    let Some(anims) = fox_anims else { return };
    for (entity, mut player) in new_players.iter_mut() {
        let mut curr = entity;
        let mut is_fox = false;
        loop {
            if let Ok(c) = creature_query.get(curr) {
                if c.creature_type == CreatureType::Fox {
                    is_fox = true;
                }
                break;
            }
            if let Ok(child_of) = parent_query.get(curr) {
                curr = child_of.parent();
            } else {
                break;
            }
        }
        if is_fox {
            commands
                .entity(entity)
                .insert(AnimationGraphHandle(anims.graph.clone()));
            player.play(anims.survey).repeat();
        }
    }
}

pub fn attach_trilobite_animation_player(
    mut commands: Commands,
    trilo_anims: Option<Res<TrilobiteAnimations>>,
    mut new_players: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>,
    parent_query: Query<&ChildOf>,
    creature_query: Query<&PlayCreature>,
) {
    let Some(anims) = trilo_anims else { return };
    for (entity, mut player) in new_players.iter_mut() {
        let mut curr = entity;
        let mut is_trilobite = false;
        loop {
            if let Ok(c) = creature_query.get(curr) {
                if c.creature_type == CreatureType::RobotTrilobite {
                    is_trilobite = true;
                }
                break;
            }
            if let Ok(child_of) = parent_query.get(curr) {
                curr = child_of.parent();
            } else {
                break;
            }
        }
        if is_trilobite {
            commands
                .entity(entity)
                .insert(AnimationGraphHandle(anims.graph.clone()));
            player.play(anims.idle).repeat();
        }
    }
}

// ──────────────────────────────────────────────
// Animation driving systems (switch clips based on creature state)
// ──────────────────────────────────────────────

pub fn drive_fox_animations(
    fox_anims: Option<Res<FoxAnimations>>,
    mut players: Query<(Entity, &mut AnimationPlayer)>,
    parent_query: Query<&ChildOf>,
    creature_query: Query<&PlayCreature>,
) {
    let Some(anims) = fox_anims else { return };
    for (entity, mut player) in players.iter_mut() {
        let mut curr = entity;
        let mut creature_opt = None;
        loop {
            if let Ok(c) = creature_query.get(curr) {
                creature_opt = Some(c);
                break;
            }
            if let Ok(child_of) = parent_query.get(curr) {
                curr = child_of.parent();
            } else {
                break;
            }
        }
        let Some(creature) = creature_opt else {
            continue;
        };
        if creature.creature_type != CreatureType::Fox {
            continue;
        }

        if creature.state == CreatureState::Dead {
            player.stop_all();
            continue;
        }

        let speed = creature.velocity.length();
        let target_node = if speed < 0.1 {
            anims.survey
        } else if speed < 2.5 {
            anims.walk
        } else {
            anims.run
        };
        // Only switch animation when the target clip changes to avoid restarting it every frame
        if !player.is_playing_animation(target_node) {
            player.play(target_node).repeat();
        }
    }
}

pub fn drive_trilobite_animations(
    trilo_anims: Option<Res<TrilobiteAnimations>>,
    mut players: Query<(Entity, &mut AnimationPlayer)>,
    parent_query: Query<&ChildOf>,
    creature_query: Query<&PlayCreature>,
) {
    let Some(anims) = trilo_anims else { return };
    for (entity, mut player) in players.iter_mut() {
        let mut curr = entity;
        let mut creature_opt = None;
        loop {
            if let Ok(c) = creature_query.get(curr) {
                creature_opt = Some(c);
                break;
            }
            if let Ok(child_of) = parent_query.get(curr) {
                curr = child_of.parent();
            } else {
                break;
            }
        }
        let Some(creature) = creature_opt else {
            continue;
        };
        if creature.creature_type != CreatureType::RobotTrilobite {
            continue;
        }

        if creature.state == CreatureState::Dead {
            player.stop_all();
            continue;
        }

        let speed = creature.velocity.length();
        let target_node = if creature.state == CreatureState::Attacking {
            anims.attack
        } else if creature.state == CreatureState::Chasing {
            anims.run
        } else if speed > 0.5 {
            anims.walk
        } else {
            anims.idle
        };
        if !player.is_playing_animation(target_node) {
            player.play(target_node).repeat();
        }
    }
}

// ──────────────────────────────────────────────
// Trilobite defender spawn (T key)
// ──────────────────────────────────────────────

pub fn spawn_defender_trilobite(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    asset_server: Res<AssetServer>,
    player_query: Query<&PlayModePlayer>,
) {
    if !keys.just_pressed(KeyCode::KeyT) {
        return;
    }
    let Ok(player) = player_query.single() else {
        return;
    };
    let spawn_pos = player.position + Vec3::new(2.0, 0.0, 2.0);

    commands.spawn((
        WorldAssetRoot(asset_server.load("Enemy_Trilobite.gltf#Scene0")),
        Transform::from_translation(spawn_pos).with_scale(Vec3::splat(0.5)),
        PlayCreature {
            creature_type: CreatureType::RobotTrilobite,
            state: CreatureState::Idle,
            health: 100.0,
            max_health: 100.0,
            position: spawn_pos,
            velocity: Vec3::ZERO,
            yaw: 0.0,
            wander_timer: 0.0,
            hop_cooldown: 0.0,
            is_grounded: true,
            death_timer: 0.0,
            attack_cooldown: 0.0,
        },
        PlayerDefender,
        PlayModeEntity,
        Visibility::Visible,
        InheritedVisibility::default(),
        avian3d::prelude::RigidBody::Dynamic,
        avian3d::prelude::Collider::capsule(0.6, 0.4),
        avian3d::prelude::LockedAxes::ROTATION_LOCKED,
        avian3d::prelude::Friction::ZERO,
    ));
    crate::play_mode::inventory_log("🤖 Robot Trilobite defender deployed!");
}

// ──────────────────────────────────────────────
// Creature Spawning
// ──────────────────────────────────────────────

// Spawns a list of creatures spread out on flat areas of the map
#[allow(clippy::needless_range_loop)]
pub fn spawn_creatures_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    map: Res<TempestMap>,
) {
    let w = map.width;
    let h = map.height;
    let offset_x = -(w as f32) / 2.0;
    let offset_z = -(h as f32) / 2.0;

    let mut spawn_points = Vec::new();

    // Scan for flat areas away from spawn
    for z in (10..(h - 10)).step_by(8) {
        for x in (10..(w - 10)).step_by(8) {
            let y = map.get_height(x, z);
            if y > 0.5 {
                // Above water
                let pos = Vec3::new(x as f32 + offset_x, y, z as f32 + offset_z);
                if pos.length() > 15.0 {
                    // away from spawning point at (0,0)
                    spawn_points.push(pos);
                }
            }
        }
    }

    // Shuffle/choose points deterministically
    let count = spawn_points.len();
    if count == 0 {
        return;
    }

    // Deterministically shuffle spawn points so creatures are spread evenly across the entire map/island rather than clustered in the far corner
    spawn_points.sort_by_cached_key(|pos| {
        (((pos.x * 12.9898 + pos.z * 78.233).sin() * 43_758.547).fract() * 100000.0) as i32
    });

    let mut spawn_idx = 0;
    let get_next_spawn = |idx: &mut usize, points: &[Vec3]| -> Vec3 {
        let p = points[*idx % points.len()];
        *idx += 1;
        p
    };

    // 1. Spawn 4 Triangaroo (Kangaroo GLTF)
    for i in 0..4 {
        let pos = get_next_spawn(&mut spawn_idx, &spawn_points);
        commands.spawn((
            WorldAssetRoot(asset_server.load("059_Triangaroo_Art.glb#Scene0")),
            Transform::from_translation(pos),
            PlayCreature {
                creature_type: CreatureType::Triangaroo,
                state: CreatureState::Wandering,
                health: 30.0,
                max_health: 30.0,
                position: pos,
                velocity: Vec3::ZERO,
                yaw: (i as f32) * 1.57,
                wander_timer: 2.0,
                hop_cooldown: 1.0,
                is_grounded: true,
                death_timer: 0.0,
                attack_cooldown: 0.0,
            },
            PlayModeEntity,
            Visibility::Visible,
            InheritedVisibility::default(),
            avian3d::prelude::RigidBody::Dynamic,
            avian3d::prelude::Collider::capsule(0.4, 1.2),
            avian3d::prelude::LockedAxes::ROTATION_LOCKED,
            avian3d::prelude::Friction::ZERO,
        ));
    }

    // 2. Spawn 4 Polypug (Quadruped GLTF)
    for i in 0..4 {
        let pos = get_next_spawn(&mut spawn_idx, &spawn_points);

        commands.spawn((
            WorldAssetRoot(asset_server.load("060_Polypug_Art.glb#Scene0")),
            Transform::from_translation(pos),
            PlayCreature {
                creature_type: CreatureType::Polypug,
                state: CreatureState::Wandering,
                health: 20.0,
                max_health: 20.0,
                position: pos,
                velocity: Vec3::ZERO,
                yaw: (i as f32) * 1.15,
                wander_timer: 1.0 + (i as f32) * 0.5,
                hop_cooldown: 0.0,
                is_grounded: true,
                death_timer: 0.0,
                attack_cooldown: 0.0,
            },
            PlayModeEntity,
            Visibility::Visible,
            InheritedVisibility::default(),
            avian3d::prelude::RigidBody::Dynamic,
            avian3d::prelude::Collider::capsule(0.3, 0.5),
            avian3d::prelude::LockedAxes::ROTATION_LOCKED,
            avian3d::prelude::Friction::ZERO,
        ));
    }

    // 3. Spawn 4 Fox (Quadruped GLTF — uses embedded GLTF animations)
    for i in 0..4 {
        let pos = get_next_spawn(&mut spawn_idx, &spawn_points);

        commands.spawn((
            WorldAssetRoot(asset_server.load("Fox.glb#Scene0")),
            Transform::from_translation(pos).with_scale(Vec3::splat(0.012)),
            PlayCreature {
                creature_type: CreatureType::Fox,
                state: CreatureState::Wandering,
                health: 20.0,
                max_health: 20.0,
                position: pos,
                velocity: Vec3::ZERO,
                yaw: (i as f32) * 1.15,
                wander_timer: 1.0 + (i as f32) * 0.5,
                hop_cooldown: 0.0,
                is_grounded: true,
                death_timer: 0.0,
                attack_cooldown: 0.0,
            },
            PlayModeEntity,
            Visibility::Visible,
            InheritedVisibility::default(),
            avian3d::prelude::RigidBody::Dynamic,
            avian3d::prelude::Collider::capsule(0.3, 0.6),
            avian3d::prelude::LockedAxes::ROTATION_LOCKED,
            avian3d::prelude::Friction::ZERO,
        ));
    }

    // 4. Spawn 3 Birds (Procedural flying birds circling overhead)
    let bird_mesh = meshes.add(Sphere::new(0.18));
    let bird_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.6, 0.8), // vibrant blue plumage
        perceptual_roughness: 0.5,
        ..default()
    });

    let wing_mesh = meshes.add(Cuboid::new(0.6, 0.02, 0.16));
    let wing_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.9, 0.95), // white flight feathers
        perceptual_roughness: 0.7,
        ..default()
    });

    for i in 0..3 {
        let start_pos = Vec3::new(
            -15.0 + (i as f32) * 15.0,
            12.0 + (i as f32) * 1.5,
            -15.0 + (i as f32) * 15.0,
        );

        let parent_bird = commands
            .spawn((
                Mesh3d(bird_mesh.clone()),
                MeshMaterial3d(bird_mat.clone()),
                Transform::from_translation(start_pos),
                PlayCreature {
                    creature_type: CreatureType::Bird,
                    state: CreatureState::Wandering,
                    health: 10.0,
                    max_health: 10.0,
                    position: start_pos,
                    velocity: Vec3::ZERO,
                    yaw: (i as f32) * 2.09,
                    wander_timer: 0.0,
                    hop_cooldown: 0.0,
                    is_grounded: false,
                    death_timer: 0.0,
                    attack_cooldown: 0.0,
                },
                PlayModeEntity,
            ))
            .id();

        // Spawn left wing
        let left_wing = commands
            .spawn((
                Mesh3d(wing_mesh.clone()),
                MeshMaterial3d(wing_mat.clone()),
                Transform::from_xyz(-0.35, 0.0, 0.0),
                ProceduralWing { is_left: true },
                PlayModeEntity,
            ))
            .id();
        commands.entity(parent_bird).add_child(left_wing);

        // Spawn right wing
        let right_wing = commands
            .spawn((
                Mesh3d(wing_mesh.clone()),
                MeshMaterial3d(wing_mat.clone()),
                Transform::from_xyz(0.35, 0.0, 0.0),
                ProceduralWing { is_left: false },
                PlayModeEntity,
            ))
            .id();
        commands.entity(parent_bird).add_child(right_wing);
    }

    // 5. Spawn 2 Alien Monsters (Procedural Alien Jellyfish)
    let jelly_bell_mesh = meshes.add(Sphere::new(0.6).mesh().uv(32, 16));
    let jelly_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.1, 0.9, 0.4, 0.6), // Translucent glowing green
        alpha_mode: AlphaMode::Blend,
        emissive: LinearRgba::from(Color::srgb(0.1, 0.9, 0.4)) * 2.0,
        ..default()
    });

    let tentacle_mesh = meshes.add(Cylinder::new(0.04, 1.2));
    let tentacle_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.2, 0.8, 0.3, 0.8),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    for i in 0..2 {
        let pos = get_next_spawn(&mut spawn_idx, &spawn_points);
        let monster_entity = commands
            .spawn((
                Mesh3d(jelly_bell_mesh.clone()),
                MeshMaterial3d(jelly_mat.clone()),
                Transform::from_translation(pos).with_scale(Vec3::splat(1.5)),
                PlayCreature {
                    creature_type: CreatureType::Monster,
                    state: CreatureState::Wandering,
                    health: 150.0,
                    max_health: 150.0,
                    position: pos,
                    velocity: Vec3::ZERO,
                    yaw: (i as f32) * std::f32::consts::PI,
                    wander_timer: 1.0,
                    hop_cooldown: 0.0,
                    is_grounded: false, // Make them float!
                    death_timer: 0.0,
                    attack_cooldown: 0.0,
                },
                PlayModeEntity,
                avian3d::prelude::RigidBody::Dynamic,
                avian3d::prelude::Collider::sphere(1.2),
                avian3d::prelude::LockedAxes::ROTATION_LOCKED,
                avian3d::prelude::Friction::ZERO,
            ))
            .id();

        // Spawn a scary green point light inside the jellyfish
        let green_glow = commands
            .spawn((
                PointLight {
                    color: Color::srgb(0.1, 1.0, 0.3),
                    intensity: 12000.0,
                    range: 8.0,
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::from_xyz(0.0, -0.2, 0.0),
                PlayModeEntity,
            ))
            .id();
        commands.entity(monster_entity).add_child(green_glow);

        // Add 4 dangling tentacles
        for t in 0..4 {
            let angle = (t as f32) * std::f32::consts::FRAC_PI_2;
            let offset_x = angle.cos() * 0.4;
            let offset_z = angle.sin() * 0.4;

            let tentacle = commands
                .spawn((
                    Mesh3d(tentacle_mesh.clone()),
                    MeshMaterial3d(tentacle_mat.clone()),
                    Transform::from_xyz(offset_x, -0.6, offset_z).with_rotation(
                        Quat::from_rotation_x(0.2 * angle.cos())
                            * Quat::from_rotation_z(0.2 * angle.sin()),
                    ),
                    PlayModeEntity,
                ))
                .id();
            commands.entity(monster_entity).add_child(tentacle);
        }
    }

    // 6. Spawn Alien Settlement (Monolith + 3 futuristic homes) and 3 Alien NPCs
    let w = map.width as f32;
    let h = map.height as f32;
    let span_x = w * 0.21;
    let span_z = h * 0.21;

    let monolith_pos = Vec3::new(
        span_x,
        crate::play_mode::get_bilinear_height(span_x, -span_z, &map),
        -span_z,
    );
    let h1_pos = Vec3::new(
        span_x - 6.0,
        crate::play_mode::get_bilinear_height(span_x - 6.0, -span_z + 6.0, &map),
        -span_z + 6.0,
    );
    let h2_pos = Vec3::new(
        span_x + 6.0,
        crate::play_mode::get_bilinear_height(span_x + 6.0, -span_z, &map),
        -span_z,
    );
    let h3_pos = Vec3::new(
        span_x - 5.0,
        crate::play_mode::get_bilinear_height(span_x - 5.0, -span_z - 6.0, &map),
        -span_z - 6.0,
    );

    spawn_alien_monolith(&mut commands, &mut meshes, &mut materials, monolith_pos);
    spawn_alien_house(&mut commands, &mut meshes, &mut materials, h1_pos);
    spawn_alien_house(&mut commands, &mut meshes, &mut materials, h2_pos);
    spawn_alien_house(&mut commands, &mut meshes, &mut materials, h3_pos);

    let alien_spawn_positions = [
        h1_pos + Vec3::new(0.0, 0.2, 2.0),
        h2_pos + Vec3::new(2.0, 0.2, 0.0),
        h3_pos + Vec3::new(0.0, 0.2, -2.0),
    ];

    for i in 0..3 {
        let pos = alien_spawn_positions[i];
        commands.spawn((
            WorldAssetRoot(asset_server.load("alien.glb#Scene0")),
            Transform::from_translation(pos),
            PlayCreature {
                creature_type: CreatureType::Alien,
                state: CreatureState::Wandering,
                health: 50.0,
                max_health: 50.0,
                position: pos,
                velocity: Vec3::ZERO,
                yaw: (i as f32) * 2.09,
                wander_timer: 2.0 + (i as f32) * 1.0,
                hop_cooldown: 0.0,
                is_grounded: true,
                death_timer: 0.0,
                attack_cooldown: 0.0,
            },
            AggroState {
                aggro_timer: 0.0,
                is_provoked: false,
            },
            PlayModeEntity,
            Visibility::Visible,
            InheritedVisibility::default(),
            avian3d::prelude::RigidBody::Dynamic,
            avian3d::prelude::Collider::capsule(0.5, 1.4),
            avian3d::prelude::LockedAxes::ROTATION_LOCKED,
            avian3d::prelude::Friction::ZERO,
        ));
    }
}

// ──────────────────────────────────────────────
// Creature AI
// ──────────────────────────────────────────────

// Runs AI movement, gravity, and attacking logic for all creatures
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn creature_ai_system(
    mut commands: Commands,
    time: Res<Time>,
    map: Res<TempestMap>,
    mut player_query: Query<(Entity, &mut PlayModePlayer)>,
    mut creature_query: Query<(
        Entity,
        &mut PlayCreature,
        &mut Transform,
        Option<&mut AggroState>,
        Option<&mut avian3d::prelude::Position>,
    )>,
    collider_query: Query<
        (Entity, &crate::play_mode::WallCollider, &Transform),
        (Without<PlayModePlayer>, Without<PlayCreature>),
    >,
    door_query: Query<&crate::play_mode::house::HouseDoor>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let dt = time.delta_secs();
    let Ok((_player_entity, mut player)) = player_query.single_mut() else {
        return;
    };
    let player_pos = player.position;

    // Pre-collect hostile creature positions so the trilobite can find targets
    // without borrowing creature_query a second time during the mutable iteration.
    let hostile_positions: Vec<(Entity, Vec3)> = creature_query
        .iter()
        .filter(|(_, c, _, _, _)| {
            c.state != CreatureState::Dead
                && matches!(
                    c.creature_type,
                    CreatureType::Monster | CreatureType::Triangaroo | CreatureType::Polypug
                )
        })
        .map(|(e, _, t, _, _)| (e, t.translation))
        .collect();

    for (entity, mut creature, mut transform, mut aggro_opt, phys_pos_opt) in
        creature_query.iter_mut()
    {
        if creature.state == CreatureState::Dead {
            commands
                .entity(entity)
                .remove::<avian3d::prelude::RigidBody>();
            commands
                .entity(entity)
                .remove::<avian3d::prelude::Collider>();
            creature.death_timer += dt;

            // Centralized smooth death falling tilt animation (tilt to 90 degrees / FRAC_PI_2 over 0.5 seconds)
            let tilt_progress = (creature.death_timer / 0.5).min(1.0);
            let tilt_angle = tilt_progress * std::f32::consts::FRAC_PI_2;
            let yaw_offset = match creature.creature_type {
                CreatureType::Triangaroo => 0.0,
                CreatureType::Polypug | CreatureType::Monster => 0.0,
                CreatureType::Fox => 0.0,
                CreatureType::Alien => 0.0,
                CreatureType::RobotTrilobite => 0.0,
                _ => -std::f32::consts::FRAC_PI_2,
            };
            let base_rot = Quat::from_rotation_y(-creature.yaw + yaw_offset);
            transform.rotation = Quat::from_rotation_x(tilt_angle) * base_rot;
            transform.translation = creature.position;

            if creature.death_timer >= 4.0 {
                // Drop elements upon death
                let drop_pos = creature.position + Vec3::Y * 0.5;
                spawn_death_loot_mesh(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    drop_pos,
                    creature.creature_type,
                );
                commands.entity(entity).despawn();
            }
            continue;
        }

        // Ticking attack cooldown
        if creature.attack_cooldown > 0.0 {
            creature.attack_cooldown -= dt;
        }

        let dist_to_player = creature.position.distance(player_pos);

        match creature.creature_type {
            CreatureType::Triangaroo => {
                // Triangaroo Hopping AI
                if creature.is_grounded {
                    // Slow down friction
                    creature.velocity.x *= (0.01f32).powf(dt);
                    creature.velocity.z *= (0.01f32).powf(dt);

                    creature.hop_cooldown -= dt;
                    if creature.hop_cooldown <= 0.0 {
                        // Choose to hop towards player if somewhat near, otherwise wander
                        let hop_dir = if dist_to_player < 10.0 {
                            (player_pos - creature.position).normalize_or_zero()
                        } else {
                            let angle = rand::random::<f32>() * std::f32::consts::TAU;
                            Vec3::new(angle.cos(), 0.0, angle.sin())
                        };

                        creature.yaw = hop_dir.z.atan2(hop_dir.x);
                        creature.velocity.y = 3.8; // jump impulse
                        creature.velocity.x = hop_dir.x * 3.2; // forward speed
                        creature.velocity.z = hop_dir.z * 3.2;
                        creature.is_grounded = false;
                        creature.hop_cooldown = 0.8 + rand::random::<f32>() * 1.2;
                    }
                } else {
                    // In air: apply gravity
                    creature.velocity.y -= 9.8 * dt;
                }

                // Apply velocity
                let vel = creature.velocity;
                creature.position += vel * dt;

                // Collide with terrain
                let terrain_y = get_bilinear_height(creature.position.x, creature.position.z, &map);
                let ground_y = get_effective_floor_height(creature.position, terrain_y);
                if creature.position.y <= ground_y {
                    creature.position.y = ground_y;
                    creature.velocity.y = 0.0;
                    creature.is_grounded = true;
                }
            }
            CreatureType::Polypug => {
                // Quadruped wandering AI
                let terrain_y = get_bilinear_height(creature.position.x, creature.position.z, &map);
                creature.position.y = get_effective_floor_height(creature.position, terrain_y);

                creature.wander_timer -= dt;
                if creature.wander_timer <= 0.0 {
                    creature.wander_timer = 2.0 + rand::random::<f32>() * 4.0;
                    creature.yaw = rand::random::<f32>() * std::f32::consts::TAU;
                    creature.state = if rand::random::<f32>() > 0.4 {
                        CreatureState::Wandering
                    } else {
                        CreatureState::Idle
                    };
                }

                if creature.state == CreatureState::Wandering {
                    let dir = Vec3::new(creature.yaw.cos(), 0.0, creature.yaw.sin());
                    creature.velocity = dir * 1.5;
                } else {
                    creature.velocity = Vec3::ZERO;
                }
                let vel = creature.velocity;
                creature.position += vel * dt;
            }

            CreatureType::Fox => {
                let terrain_y = get_bilinear_height(creature.position.x, creature.position.z, &map);
                creature.position.y = get_effective_floor_height(creature.position, terrain_y);

                creature.wander_timer -= dt;

                // Foxes are more skittish and curious
                if creature.wander_timer <= 0.0 {
                    creature.wander_timer = 0.8 + rand::random::<f32>() * 2.2; // Foxes change direction more often

                    let should_run = rand::random::<f32>() < 0.35; // 35% chance to sprint

                    creature.state = if should_run {
                        CreatureState::Wandering // or add a Running state later
                    } else {
                        CreatureState::Idle
                    };

                    creature.yaw = rand::random::<f32>() * std::f32::consts::TAU;
                }

                if creature.state == CreatureState::Wandering {
                    let dir = Vec3::new(creature.yaw.cos(), 0.0, creature.yaw.sin());
                    let speed = if creature.wander_timer > 1.5 {
                        3.2
                    } else {
                        1.8
                    }; // occasional bursts of speed
                    creature.velocity = dir * speed;
                } else {
                    creature.velocity = Vec3::ZERO;
                }

                // Apply movement
                let vel = creature.velocity;
                creature.position += vel * dt;

                // Gentle bobbing when idle (fox-like curiosity)
                if creature.state == CreatureState::Idle {
                    creature.position.y += (time.elapsed_secs() * 6.0).sin() * 0.025;
                }
            }
            CreatureType::Bird => {
                // Flying Bird AI (circles in sky)
                creature.yaw += 0.6 * dt; // slow turn rate
                let dir = Vec3::new(creature.yaw.cos(), 0.0, creature.yaw.sin());
                creature.position += dir * 5.5 * dt;
                creature.position.y = 12.0 + (time.elapsed_secs() * 0.4).sin() * 1.5; // slow sinus bobbing
            }
            CreatureType::Monster => {
                // Red glowing beast AI (chasing / aggressive)
                let terrain_y = get_bilinear_height(creature.position.x, creature.position.z, &map);
                let floor_y = get_effective_floor_height(creature.position, terrain_y);
                // Hover 2.5 meters above the ground with a slight bob
                let target_y = floor_y + 2.5 + (time.elapsed_secs() * 1.2).sin() * 0.3;
                creature.position.y += (target_y - creature.position.y) * 4.0 * dt;
                creature.velocity.y = 0.0;

                if dist_to_player < 14.0 && player.health > 0.0 {
                    // Chase mode!
                    creature.state = CreatureState::Chasing;
                    let to_player = (player_pos - creature.position).normalize_or_zero();
                    creature.yaw = to_player.z.atan2(to_player.x);
                    creature.velocity = to_player * 3.8;
                    let vel = creature.velocity;
                    creature.position += vel * dt; // aggressive charge speed!

                    // Attack trigger if next to player
                    if dist_to_player < 1.6 && creature.attack_cooldown <= 0.0 {
                        player.health = (player.health - 20.0).max(0.0);
                        creature.attack_cooldown = 1.8;

                        // Push player back
                        let push_back = to_player * 6.0;
                        player.position += push_back;
                        for node in player.nodes.iter_mut() {
                            node.position += push_back;
                        }
                        crate::play_mode::inventory_log(
                            "🚨 OUCH! You were struck by the Alien Beast! Received -20 damage.",
                        );
                    }
                } else {
                    // Wander mode
                    creature.state = CreatureState::Wandering;
                    creature.wander_timer -= dt;
                    if creature.wander_timer <= 0.0 {
                        creature.wander_timer = 3.0 + rand::random::<f32>() * 3.0;
                        creature.yaw = rand::random::<f32>() * std::f32::consts::TAU;
                    }
                    let dir = Vec3::new(creature.yaw.cos(), 0.0, creature.yaw.sin());
                    creature.position += dir * 1.8 * dt;
                }
            }

            CreatureType::Alien => {
                // Neutral NPC villager — only hostile if provoked by player attacks
                let terrain_y = get_bilinear_height(creature.position.x, creature.position.z, &map);
                creature.position.y = get_effective_floor_height(creature.position, terrain_y);

                // Handle aggro cooldown
                if let Some(ref mut aggro) = aggro_opt
                    && aggro.is_provoked
                {
                    aggro.aggro_timer -= dt;
                    if aggro.aggro_timer <= 0.0 {
                        aggro.is_provoked = false;
                        creature.state = CreatureState::Wandering;
                    }
                }

                let is_provoked = aggro_opt.as_ref().is_some_and(|a| a.is_provoked);

                if is_provoked && dist_to_player < 20.0 && player.health > 0.0 {
                    // Chase and attack player when provoked
                    creature.state = CreatureState::Chasing;
                    let to_player = (player_pos - creature.position).normalize_or_zero();
                    creature.yaw = to_player.z.atan2(to_player.x);
                    creature.velocity = to_player * 3.0;
                    let vel = creature.velocity;
                    creature.position += vel * dt;

                    if dist_to_player < 1.8 && creature.attack_cooldown <= 0.0 {
                        player.health = (player.health - 15.0).max(0.0);
                        creature.attack_cooldown = 2.0;
                        crate::play_mode::inventory_log("🛸 An Alien struck you! -15 HP");
                    }
                } else {
                    // Neutral wandering
                    creature.wander_timer -= dt;
                    if creature.wander_timer <= 0.0 {
                        creature.wander_timer = 3.0 + rand::random::<f32>() * 4.0;
                        creature.yaw = rand::random::<f32>() * std::f32::consts::TAU;
                        creature.state = if rand::random::<f32>() > 0.3 {
                            CreatureState::Wandering
                        } else {
                            CreatureState::Idle
                        };
                    }

                    if creature.state == CreatureState::Wandering {
                        let dir = Vec3::new(creature.yaw.cos(), 0.0, creature.yaw.sin());
                        creature.velocity = dir * 1.2;
                    } else {
                        creature.velocity = Vec3::ZERO;
                    }
                    let vel = creature.velocity;
                    creature.position += vel * dt;
                }
            }

            CreatureType::RobotTrilobite => {
                // Player-allied defender — patrols near the player AND attacks hostile creatures
                let terrain_y = get_bilinear_height(creature.position.x, creature.position.z, &map);
                creature.position.y = get_effective_floor_height(creature.position, terrain_y);

                // Find nearest hostile from pre-collected positions
                let my_pos = creature.position;
                let mut nearest_hostile: Option<(Entity, Vec3, f32)> = None;
                for &(hostile_entity, hostile_pos) in &hostile_positions {
                    if hostile_entity == entity {
                        continue;
                    }
                    let dist = my_pos.distance(hostile_pos);
                    if dist < 15.0
                        && (nearest_hostile.is_none() || dist < nearest_hostile.unwrap().2)
                    {
                        nearest_hostile = Some((hostile_entity, hostile_pos, dist));
                    }
                }

                if let Some((_target_entity, target_pos, target_dist)) = nearest_hostile {
                    // Combat mode — chase and attack the hostile
                    let to_target = (target_pos - creature.position).normalize_or_zero();
                    creature.yaw = to_target.z.atan2(to_target.x);

                    if target_dist < 2.0 {
                        // In attack range
                        creature.state = CreatureState::Attacking;
                        creature.velocity = Vec3::ZERO;
                    } else {
                        // Chase toward the hostile
                        creature.state = CreatureState::Chasing;
                        creature.velocity = to_target * 4.5;
                    }
                } else if dist_to_player > 12.0 {
                    // No hostiles nearby — run back to player
                    creature.state = CreatureState::Chasing;
                    let to_player = (player_pos - creature.position).normalize_or_zero();
                    creature.yaw = to_player.z.atan2(to_player.x);
                    creature.velocity = to_player * 4.5;
                } else if dist_to_player > 5.0 {
                    // Walk toward player
                    creature.state = CreatureState::Wandering;
                    let to_player = (player_pos - creature.position).normalize_or_zero();
                    creature.yaw = to_player.z.atan2(to_player.x);
                    creature.velocity = to_player * 2.0;
                } else {
                    // Patrol/idle near player
                    creature.wander_timer -= dt;
                    if creature.wander_timer <= 0.0 {
                        creature.wander_timer = 2.0 + rand::random::<f32>() * 3.0;
                        creature.yaw = rand::random::<f32>() * std::f32::consts::TAU;
                        creature.state = if rand::random::<f32>() > 0.5 {
                            CreatureState::Wandering
                        } else {
                            CreatureState::Idle
                        };
                    }
                    if creature.state == CreatureState::Wandering {
                        let dir = Vec3::new(creature.yaw.cos(), 0.0, creature.yaw.sin());
                        creature.velocity = dir * 1.5;
                    } else {
                        creature.velocity = Vec3::ZERO;
                    }
                }
                let vel = creature.velocity;
                creature.position += vel * dt;
            }
        }

        // Clamp positions to terrain dimensions
        let hw = map.width as f32 / 2.0;
        let hh = map.height as f32 / 2.0;
        creature.position.x = creature.position.x.clamp(-hw + 2.0, hw - 2.0);
        creature.position.z = creature.position.z.clamp(-hh + 2.0, hh - 2.0);

        // Apply Wall Collisions
        let creature_radius = match creature.creature_type {
            CreatureType::Monster => 1.2,
            CreatureType::Triangaroo => 0.4,
            CreatureType::Fox => 0.3,
            CreatureType::Alien => 0.5,
            CreatureType::RobotTrilobite => 0.6,
            _ => 0.3,
        };
        for (entity, collider, col_transform) in collider_query.iter() {
            if let Ok(door) = door_query.get(entity)
                && door.is_open
            {
                continue;
            }

            let center = col_transform.translation;
            let extents = collider.half_extents;

            let closest_point = Vec3::new(
                creature
                    .position
                    .x
                    .clamp(center.x - extents.x, center.x + extents.x),
                creature
                    .position
                    .y
                    .clamp(center.y - extents.y, center.y + extents.y),
                creature
                    .position
                    .z
                    .clamp(center.z - extents.z, center.z + extents.z),
            );

            let dist = creature.position.distance(closest_point);
            if dist < creature_radius {
                let penetration = creature_radius - dist;
                let push_dir = (creature.position - closest_point).normalize_or_zero();
                creature.position += push_dir * penetration;
            }
        }

        // Sync Transform
        transform.translation = creature.position;
        if let Some(mut phys_pos) = phys_pos_opt {
            phys_pos.0 = creature.position;
        }
        // Different creatures have different front-facing orientations in their source GLB.
        let yaw_offset = match creature.creature_type {
            CreatureType::Triangaroo => 0.0, // Face direction of motion instead of sideways
            CreatureType::Polypug | CreatureType::Monster => 0.0,
            CreatureType::Fox => 0.0,
            CreatureType::Alien => 0.0,
            CreatureType::RobotTrilobite => 0.0,
            _ => -std::f32::consts::FRAC_PI_2,
        };
        transform.rotation = Quat::from_rotation_y(-creature.yaw + yaw_offset);
    }
}

// ──────────────────────────────────────────────
// Trilobite Combat (damage dealing)
// ──────────────────────────────────────────────

/// Separate system so the trilobite can mutably access other creatures for damage.
/// Runs after creature_ai_system sets Attacking state + positions.
pub fn trilobite_combat_system(mut creature_query: Query<(Entity, &mut PlayCreature, &Transform)>) {
    // First pass: collect attacking trilobites and their targets
    let mut attacks: Vec<(Entity, Vec3)> = Vec::new();
    for (entity, creature, _transform) in creature_query.iter() {
        if creature.creature_type == CreatureType::RobotTrilobite
            && creature.state == CreatureState::Attacking
            && creature.attack_cooldown <= 0.0
        {
            attacks.push((entity, creature.position));
        }
    }

    // Second pass: for each attacking trilobite, find & damage the nearest hostile
    for (trilo_entity, trilo_pos) in attacks {
        let mut best_target: Option<(Entity, f32)> = None;
        for (other_entity, other_creature, _) in creature_query.iter() {
            if other_entity == trilo_entity {
                continue;
            }
            if other_creature.state == CreatureState::Dead {
                continue;
            }
            match other_creature.creature_type {
                CreatureType::Monster | CreatureType::Triangaroo | CreatureType::Polypug => {}
                _ => continue,
            }
            let dist = trilo_pos.distance(other_creature.position);
            if dist < 2.5 && (best_target.is_none() || dist < best_target.unwrap().1) {
                best_target = Some((other_entity, dist));
            }
        }

        if let Some((target_entity, _)) = best_target {
            // Deal damage to the hostile
            if let Ok((_, mut target, _)) = creature_query.get_mut(target_entity) {
                target.health = (target.health - 12.0).max(0.0);
                let kb_dir = (target.position - trilo_pos).normalize_or_zero();
                target.velocity += kb_dir * 1.5;
                if target.health <= 0.0 {
                    target.state = CreatureState::Dead;
                    target.death_timer = 0.0;
                    crate::play_mode::inventory_log(
                        "🤖 Your Trilobite defender struck down a hostile!",
                    );
                }
            }
            // Reset the trilobite's attack cooldown
            if let Ok((_, mut trilo, _)) = creature_query.get_mut(trilo_entity) {
                trilo.attack_cooldown = 0.8;
            }
        }
    }
}

// ──────────────────────────────────────────────
// Procedural animations
// ──────────────────────────────────────────────

// Controls procedural animations (wings flapping, bodies bobbing during walks)
pub fn creature_animation_sync_system(
    time: Res<Time>,
    creature_query: Query<(&PlayCreature, &Children)>,
    mut wing_query: Query<(&mut Transform, &ProceduralWing)>,
) {
    let t = time.elapsed_secs();
    for (creature, children) in creature_query.iter() {
        if creature.state == CreatureState::Dead {
            continue;
        }

        for child in children.iter() {
            if let Ok((mut wing_transform, wing)) = wing_query.get_mut(child) {
                // Wing flapping animation
                let flap_speed = 18.0;
                let flap_angle = (t * flap_speed).sin() * 0.45;
                if wing.is_left {
                    wing_transform.rotation = Quat::from_rotation_z(-flap_angle);
                } else {
                    wing_transform.rotation = Quat::from_rotation_z(flap_angle);
                }
            }
        }
    }
}

// Procedurally animates named bones of creatures (e.g. kangaroo arms, quadruped walk legs)
#[allow(clippy::type_complexity)]
pub fn creature_skeletal_animation_system(
    mut commands: Commands,
    time: Res<Time>,
    creature_query: Query<&PlayCreature>,
    parent_query: Query<&ChildOf>,
    mut bone_query: Query<
        (Entity, &Name, &mut Transform, Option<&RestPose>),
        (
            Without<PlayCreature>,
            Without<crate::play_mode::PlayModeCamera>,
        ),
    >,
) {
    let t = time.elapsed_secs();

    for (entity, name, mut transform, rest_pose) in bone_query.iter_mut() {
        // Traverse up the hierarchy using ChildOf to find the creature ancestor
        let mut curr = entity;
        let mut creature_opt = None;
        loop {
            if let Ok(c) = creature_query.get(curr) {
                creature_opt = Some(c);
                break;
            }
            if let Ok(child_of) = parent_query.get(curr) {
                curr = child_of.parent();
            } else {
                break;
            }
        }

        let Some(creature) = creature_opt else {
            continue;
        };
        if creature.state == CreatureState::Dead {
            continue;
        }

        let name_str = name.as_str();

        let rest_quat = if let Some(rest) = rest_pose {
            rest.0
        } else {
            commands.entity(entity).insert(RestPose(transform.rotation));
            transform.rotation
        };

        match creature.creature_type {
            CreatureType::Triangaroo => {
                // 1. Kangaroo Arms (Arm.L, Arm.R) hang down and sway slightly instead of T-posing
                if name_str == "Arm.L" {
                    let swing = (t * 2.5).sin() * 0.08;
                    let hop_lift = if !creature.is_grounded { 0.45 } else { 0.0 };
                    transform.rotation = rest_quat
                        * Quat::from_rotation_z(1.2 - hop_lift)
                        * Quat::from_rotation_y(-0.25)
                        * Quat::from_rotation_x(swing);
                } else if name_str == "Arm.R" {
                    let swing = (t * 2.5).sin() * 0.08;
                    let hop_lift = if !creature.is_grounded { 0.45 } else { 0.0 };
                    transform.rotation = rest_quat
                        * Quat::from_rotation_z(-1.2 + hop_lift)
                        * Quat::from_rotation_y(0.25)
                        * Quat::from_rotation_x(swing);
                }
            }

            CreatureType::Fox => {
                // Fox uses embedded GLTF animations via AnimationPlayer — no procedural animation
            }

            CreatureType::RobotTrilobite => {
                // Trilobite uses embedded GLTF animations via AnimationPlayer — no procedural animation
            }

            CreatureType::Alien => {
                // Procedural bone animation for the alien (bones: armLeft, armRight, legLeft, legRight, head, body)
                let speed = creature.velocity.length();
                let walk_cycle = (t * speed * 6.0).sin();

                if name_str == "legLeft" {
                    let swing = walk_cycle * 0.5 * (speed / 2.0).min(1.0);
                    transform.rotation = rest_quat * Quat::from_rotation_x(swing);
                } else if name_str == "legRight" {
                    let swing = -walk_cycle * 0.5 * (speed / 2.0).min(1.0);
                    transform.rotation = rest_quat * Quat::from_rotation_x(swing);
                } else if name_str == "armLeft" {
                    let swing = -walk_cycle * 0.35 * (speed / 2.0).min(1.0);
                    transform.rotation = rest_quat * Quat::from_rotation_x(swing);
                } else if name_str == "armRight" {
                    let swing = walk_cycle * 0.35 * (speed / 2.0).min(1.0);
                    transform.rotation = rest_quat * Quat::from_rotation_x(swing);
                } else if name_str == "head" {
                    let look = (t * 1.5).sin() * 0.2;
                    transform.rotation = rest_quat * Quat::from_rotation_y(look);
                }
            }

            CreatureType::Polypug | CreatureType::Monster => {
                // Quadruped walking legs swing using speed-scaled sine wave
                let speed = creature.velocity.length();
                let walk_amplitude = (speed * 0.15).min(0.45);
                let walk_angle = (t * 11.0).sin() * walk_amplitude;

                if name_str == "front_thigh.L" || name_str == "thigh.R" {
                    transform.rotation = rest_quat * Quat::from_rotation_x(walk_angle);
                } else if name_str == "front_thigh.R" || name_str == "thigh.L" {
                    transform.rotation = rest_quat * Quat::from_rotation_x(-walk_angle);
                }

                // Bend shins slightly to match the walk swing
                let shin_angle =
                    (t * 11.0 + std::f32::consts::FRAC_PI_2).sin() * walk_amplitude * 0.5;
                if name_str == "front_shin.L" || name_str == "shin.R" {
                    transform.rotation = rest_quat * Quat::from_rotation_x(shin_angle + 0.1);
                } else if name_str == "front_shin.R" || name_str == "shin.L" {
                    transform.rotation = rest_quat * Quat::from_rotation_x(-shin_angle + 0.1);
                }
            }
            _ => {}
        }
    }
}

// ──────────────────────────────────────────────
// Loot
// ──────────────────────────────────────────────

// Spawns a physical loot item mesh in the world when a creature dies
fn spawn_death_loot_mesh(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    pos: Vec3,
    c_type: CreatureType,
) {
    let (mesh, color, name) = match c_type {
        CreatureType::Triangaroo => (
            meshes.add(Sphere::new(0.2)),
            Color::srgb(0.9, 0.7, 0.4),
            "kangaroo_fur",
        ),
        CreatureType::Polypug => (
            meshes.add(Sphere::new(0.2)),
            Color::srgb(0.5, 0.4, 0.35),
            "alien_pelt",
        ),

        CreatureType::Fox => (
            meshes.add(Sphere::new(0.2)),
            Color::srgb(0.8, 0.8, 0.9),
            "fox_pelt",
        ),

        CreatureType::Bird => (
            meshes.add(Sphere::new(0.12)),
            Color::srgb(0.8, 0.8, 0.9),
            "alien_feather",
        ),
        CreatureType::Monster => (
            meshes.add(Cuboid::new(0.3, 0.3, 0.3)),
            Color::srgb(1.0, 0.8, 0.0),
            "monster_core",
        ),
        CreatureType::Alien => (
            meshes.add(Sphere::new(0.2)),
            Color::srgb(0.3, 0.9, 0.5),
            "alien_tech",
        ),
        CreatureType::RobotTrilobite => (
            meshes.add(Sphere::new(0.25)),
            Color::srgb(0.5, 0.5, 0.6),
            "robot_parts",
        ),
    };

    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            metallic: 0.4,
            perceptual_roughness: 0.6,
            emissive: if c_type == CreatureType::Monster {
                LinearRgba::from(Color::srgb(0.8, 0.4, 0.0)) * 2.0
            } else {
                LinearRgba::BLACK
            },
            ..default()
        })),
        Transform::from_translation(pos),
        crate::play_mode::PlayModeEntity,
    ));

    crate::play_mode::inventory_log(&format!("🎁 Creature dropped: {}!", name));
}

fn spawn_alien_house(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    center_pos: Vec3,
) {
    let metal_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.28, 0.35), // bluish steel
        metallic: 0.9,
        perceptual_roughness: 0.2,
        ..default()
    });

    let neon_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.1, 0.9), // magenta neon trims
        emissive: LinearRgba::from(Color::srgb(0.9, 0.1, 0.9)) * 4.0,
        ..default()
    });

    let dome_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.1, 0.6, 0.8, 0.45), // translucent dome
        alpha_mode: AlphaMode::Blend,
        emissive: LinearRgba::from(Color::srgb(0.05, 0.3, 0.4)) * 2.0,
        perceptual_roughness: 0.1,
        ..default()
    });

    // 1. Ring base (underground foundation ring)
    let base_mesh = meshes.add(Cylinder::new(1.8, 0.1));
    commands.spawn((
        Mesh3d(base_mesh),
        MeshMaterial3d(metal_mat.clone()),
        Transform::from_translation(center_pos + Vec3::new(0.0, 0.05, 0.0)),
        PlayModeEntity,
    ));

    // 2. Spawn 4 Pillars (corners of the 2.5m room)
    let pillar_mesh = meshes.add(Cuboid::new(0.16, 2.8, 0.16));
    let pillar_offsets = [
        Vec3::new(-1.25, 1.4, -1.25),
        Vec3::new(1.25, 1.4, -1.25),
        Vec3::new(-1.25, 1.4, 1.25),
        Vec3::new(1.25, 1.4, 1.25),
    ];
    for offset in pillar_offsets {
        commands.spawn((
            Mesh3d(pillar_mesh.clone()),
            MeshMaterial3d(metal_mat.clone()),
            Transform::from_translation(center_pos + offset),
            avian3d::prelude::RigidBody::Static,
            avian3d::prelude::Collider::cuboid(0.16, 2.8, 0.16),
            crate::play_mode::WallCollider {
                half_extents: Vec3::new(0.08, 1.4, 0.08),
            },
            PlayModeEntity,
        ));
    }

    // 3. Spawning 3 outer walls (back, left, right)
    let wall_mesh_back = meshes.add(Cuboid::new(2.5, 2.8, 0.08));
    let wall_mesh_side = meshes.add(Cuboid::new(0.08, 2.8, 2.5));

    // Back wall
    commands.spawn((
        Mesh3d(wall_mesh_back),
        MeshMaterial3d(metal_mat.clone()),
        Transform::from_translation(center_pos + Vec3::new(0.0, 1.4, -1.25)),
        avian3d::prelude::RigidBody::Static,
        avian3d::prelude::Collider::cuboid(2.5, 2.8, 0.08),
        crate::play_mode::WallCollider {
            half_extents: Vec3::new(1.25, 1.4, 0.04),
        },
        PlayModeEntity,
    ));
    // Left wall
    commands.spawn((
        Mesh3d(wall_mesh_side.clone()),
        MeshMaterial3d(metal_mat.clone()),
        Transform::from_translation(center_pos + Vec3::new(-1.25, 1.4, 0.0)),
        avian3d::prelude::RigidBody::Static,
        avian3d::prelude::Collider::cuboid(0.08, 2.8, 2.5),
        crate::play_mode::WallCollider {
            half_extents: Vec3::new(0.04, 1.4, 1.25),
        },
        PlayModeEntity,
    ));
    // Right wall
    commands.spawn((
        Mesh3d(wall_mesh_side),
        MeshMaterial3d(metal_mat.clone()),
        Transform::from_translation(center_pos + Vec3::new(1.25, 1.4, 0.0)),
        avian3d::prelude::RigidBody::Static,
        avian3d::prelude::Collider::cuboid(0.08, 2.8, 2.5),
        crate::play_mode::WallCollider {
            half_extents: Vec3::new(0.04, 1.4, 1.25),
        },
        PlayModeEntity,
    ));

    // 4. Dome Roof (Translucent scaled sphere)
    let dome_mesh = meshes.add(Sphere::new(1.8).mesh().ico(3).unwrap());
    commands.spawn((
        Mesh3d(dome_mesh),
        MeshMaterial3d(dome_mat),
        Transform::from_translation(center_pos + Vec3::new(0.0, 2.8, 0.0))
            .with_scale(Vec3::new(1.0, 0.45, 1.0)),
        avian3d::prelude::RigidBody::Static,
        avian3d::prelude::Collider::sphere(1.8),
        PlayModeEntity,
    ));

    // 5. Spire & Glowing Power Orb
    let spire_mesh = meshes.add(Cylinder::new(0.04, 1.5));
    commands.spawn((
        Mesh3d(spire_mesh),
        MeshMaterial3d(metal_mat),
        Transform::from_translation(center_pos + Vec3::new(0.0, 4.0, 0.0)),
        PlayModeEntity,
    ));

    let orb_mesh = meshes.add(Sphere::new(0.18).mesh().ico(3).unwrap());
    commands.spawn((
        Mesh3d(orb_mesh),
        MeshMaterial3d(neon_mat.clone()),
        Transform::from_translation(center_pos + Vec3::new(0.0, 4.8, 0.0)),
        PlayModeEntity,
    ));

    // Spire light
    commands.spawn((
        PointLight {
            color: Color::srgb(0.9, 0.1, 0.9),
            intensity: 700.0,
            range: 12.0,
            ..default()
        },
        Transform::from_translation(center_pos + Vec3::new(0.0, 4.8, 0.0)),
        PlayModeEntity,
    ));

    // 6. Interactive Translucent Energy Shield Door (placed at the front z = 1.25)
    let door_mesh = meshes.add(Cuboid::new(2.4, 2.6, 0.04));
    let energy_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.0, 0.85, 1.0, 0.4), // shimmering cyan energy
        alpha_mode: AlphaMode::Blend,
        emissive: LinearRgba::from(Color::srgb(0.0, 0.85, 1.0)) * 6.0,
        ..default()
    });

    let closed_rot = Quat::IDENTITY;
    let open_rot = Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2);

    let door_parent = commands
        .spawn((
            Transform::from_translation(center_pos + Vec3::new(-1.2, 1.3, 1.25))
                .with_rotation(closed_rot),
            crate::play_mode::house::HouseDoor {
                is_open: false,
                closed_rot,
                open_rot,
            },
            crate::play_mode::WallCollider {
                half_extents: Vec3::new(1.22, 1.3, 0.02), // blocks the entire front entrance when closed
            },
            Visibility::Visible,
            InheritedVisibility::default(),
            PlayModeEntity,
        ))
        .id();

    let door_child = commands
        .spawn((
            Mesh3d(door_mesh),
            MeshMaterial3d(energy_mat),
            Transform::from_xyz(1.2, 0.0, 0.0), // pivot hinge at the left pillar
            Visibility::default(),
            InheritedVisibility::default(),
            PlayModeEntity,
        ))
        .id();

    commands.entity(door_parent).add_child(door_child);
}

fn spawn_alien_monolith(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
) {
    let base_mesh = meshes.add(Cuboid::new(0.6, 3.5, 0.6));
    let tip_mesh = meshes.add(Sphere::new(0.3).mesh().ico(3).unwrap());

    let obsidian_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.08, 0.08, 0.1), // shiny obsidian
        metallic: 0.95,
        perceptual_roughness: 0.05,
        ..default()
    });

    let magenta_glow_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.0, 0.6),
        emissive: LinearRgba::from(Color::srgb(1.0, 0.0, 0.6)) * 6.0,
        perceptual_roughness: 0.1,
        ..default()
    });

    // Base obelisk
    commands.spawn((
        Mesh3d(base_mesh),
        MeshMaterial3d(obsidian_mat),
        Transform::from_translation(pos + Vec3::new(0.0, 1.75, 0.0)),
        avian3d::prelude::RigidBody::Static,
        avian3d::prelude::Collider::cuboid(0.6, 3.5, 0.6),
        crate::play_mode::WallCollider {
            half_extents: Vec3::new(0.3, 1.75, 0.3),
        },
        PlayModeEntity,
    ));

    // Floating glowing tip
    commands.spawn((
        Mesh3d(tip_mesh),
        MeshMaterial3d(magenta_glow_mat),
        Transform::from_translation(pos + Vec3::new(0.0, 4.0, 0.0)),
        PlayModeEntity,
    ));

    // Monolith light source
    commands.spawn((
        PointLight {
            color: Color::srgb(1.0, 0.0, 0.6),
            intensity: 900.0,
            range: 15.0,
            ..default()
        },
        Transform::from_translation(pos + Vec3::new(0.0, 4.0, 0.0)),
        PlayModeEntity,
    ));
}
