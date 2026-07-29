use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum StructureType {
    #[default]
    ClassicBrickWall,
    Watchtower,
    Staircase,
    Ramp,
    WoodenBridge,
    PalisadeFence,
    GraniteFortressWall,
    LogTimberWall,
    CyberMetalWall,
}

impl StructureType {
    pub fn name(&self) -> &'static str {
        match self {
            StructureType::ClassicBrickWall => "🧱 Classic Brick Wall",
            StructureType::Watchtower => "🗼 Fortified Watchtower",
            StructureType::Staircase => "🪜 Modular Staircase",
            StructureType::Ramp => "📐 Inclined Ramp",
            StructureType::WoodenBridge => "🌉 Wooden Plank Bridge",
            StructureType::PalisadeFence => "🪵 Palisade Stake Fence",
            StructureType::GraniteFortressWall => "🧱 Granite Fortress Wall",
            StructureType::LogTimberWall => "🪵 Log Cabin Wall",
            StructureType::CyberMetalWall => "⚡ Cyber Metal Wall",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            StructureType::ClassicBrickWall => "Original classic red brick wall segment",
            StructureType::Watchtower => "2-story lookout tower with ladder, deck, roof & lantern",
            StructureType::Staircase => "Step-by-step modular stairs with side handrails (+2.5m)",
            StructureType::Ramp => "Smooth angled inclined timber ramp (+2.5m)",
            StructureType::WoodenBridge => "Modular plank walkway deck with side safety railings",
            StructureType::PalisadeFence => "High sharpened wooden stake defensive barrier",
            StructureType::GraniteFortressWall => "Heavy stone block wall with battlements",
            StructureType::LogTimberWall => "Stacked wilderness log cabin wall",
            StructureType::CyberMetalWall => "Reinforced alloy wall with emissive cyan seams",
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct BuildingPlacementState {
    pub is_active: bool,
    pub selected_structure: StructureType,
}

impl Default for BuildingPlacementState {
    fn default() -> Self {
        Self {
            is_active: false,
            selected_structure: StructureType::ClassicBrickWall,
        }
    }
}

/// Tag component for placed procedural structures
#[derive(Component)]
pub struct PlacedStructure;

/// Tag component for the ghost placement preview box
#[derive(Component)]
pub struct PlacementPreviewGhost;

/// Spawns a procedural structure based on `StructureType`
pub fn spawn_procedural_structure(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    structure_type: StructureType,
    pos: Vec3,
    rot: Quat,
) -> Entity {
    let parent_entity = commands
        .spawn((
            Transform::from_translation(pos).with_rotation(rot),
            Visibility::Visible,
            InheritedVisibility::default(),
            PlacedStructure,
            crate::play_mode::PlayModeEntity,
        ))
        .id();

    match structure_type {
        StructureType::ClassicBrickWall => {
            spawn_classic_brick_wall(commands, meshes, materials, asset_server, parent_entity);
        }
        StructureType::Watchtower => {
            spawn_watchtower(commands, meshes, materials, asset_server, parent_entity);
        }
        StructureType::Staircase => {
            spawn_staircase(commands, meshes, materials, asset_server, parent_entity);
        }
        StructureType::Ramp => {
            spawn_ramp(commands, meshes, materials, asset_server, parent_entity);
        }
        StructureType::WoodenBridge => {
            spawn_wooden_bridge(commands, meshes, materials, asset_server, parent_entity);
        }
        StructureType::PalisadeFence => {
            spawn_palisade_fence(commands, meshes, materials, asset_server, parent_entity);
        }
        StructureType::GraniteFortressWall => {
            spawn_granite_fortress_wall(commands, meshes, materials, asset_server, parent_entity);
        }
        StructureType::LogTimberWall => {
            spawn_log_timber_wall(commands, meshes, materials, asset_server, parent_entity);
        }
        StructureType::CyberMetalWall => {
            spawn_cyber_metal_wall(commands, meshes, materials, asset_server, parent_entity);
        }
    }

    parent_entity
}

/// 1. Classic Brick Wall (built brick-by-brick via procedural_walls::WallConstructor)
fn spawn_classic_brick_wall(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    parent: Entity,
) {
    let brick_mat = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/solid_brick.png")),
        perceptual_roughness: 0.8,
        ..default()
    });

    let curve = crate::procedural_walls::Curve::from(vec![
        Vec3::new(0.0, 0.0, -1.2),
        Vec3::new(0.0, 0.0, 1.2),
    ]);

    let bricks = crate::procedural_walls::WallConstructor::from_curve(&curve, 2.4, |_| 0.0);

    for brick in bricks {
        let brick_mesh = meshes.add(Cuboid::from_size(brick.transform.scale));
        let brick_child = commands
            .spawn((
                Mesh3d(brick_mesh),
                MeshMaterial3d(brick_mat.clone()),
                brick.transform,
                crate::play_mode::PlayModeEntity,
            ))
            .id();
        commands.entity(parent).add_child(brick_child);
    }

    // Overall static physics collider covering the full wall segment
    let collider_child = commands
        .spawn((
            Transform::from_xyz(0.0, 1.2, 0.0),
            avian3d::prelude::RigidBody::Static,
            avian3d::prelude::Collider::cuboid(0.35, 2.4, 2.4),
            crate::play_mode::PlayModeEntity,
        ))
        .id();
    commands.entity(parent).add_child(collider_child);
}

/// 2. Fortified 2-Story Watchtower
fn spawn_watchtower(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    parent: Entity,
) {
    let wood_mat = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/wood_planks.png")),
        perceptual_roughness: 0.85,
        ..default()
    });
    let post_mesh = meshes.add(Cuboid::new(0.25, 5.0, 0.25));

    // 4 Corner Support Posts (5m tall)
    for x_sign in [-1.0, 1.0] {
        for z_sign in [-1.0, 1.0] {
            let post = commands
                .spawn((
                    Mesh3d(post_mesh.clone()),
                    MeshMaterial3d(wood_mat.clone()),
                    Transform::from_xyz(x_sign * 1.4, 2.5, z_sign * 1.4),
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(0.25, 5.0, 0.25),
                    crate::play_mode::PlayModeEntity,
                ))
                .id();
            commands.entity(parent).add_child(post);
        }
    }

    // Mid-Deck Floor (at Y = 3.2m)
    let deck_mesh = meshes.add(Cuboid::new(3.1, 0.2, 3.1));
    let deck = commands
        .spawn((
            Mesh3d(deck_mesh),
            MeshMaterial3d(wood_mat.clone()),
            Transform::from_xyz(0.0, 3.2, 0.0),
            avian3d::prelude::RigidBody::Static,
            avian3d::prelude::Collider::cuboid(3.1, 0.2, 3.1),
            crate::play_mode::PlayModeEntity,
        ))
        .id();
    commands.entity(parent).add_child(deck);

    // Guardrail Fence around deck (1.0m height)
    let rail_mesh_side = meshes.add(Cuboid::new(3.1, 0.9, 0.1));
    let rail_mesh_back = meshes.add(Cuboid::new(0.1, 0.9, 3.1));

    let r1 = commands
        .spawn((
            Mesh3d(rail_mesh_side.clone()),
            MeshMaterial3d(wood_mat.clone()),
            Transform::from_xyz(0.0, 3.75, 1.5),
            avian3d::prelude::RigidBody::Static,
            avian3d::prelude::Collider::cuboid(3.1, 0.9, 0.1),
            crate::play_mode::PlayModeEntity,
        ))
        .id();
    let r2 = commands
        .spawn((
            Mesh3d(rail_mesh_side),
            MeshMaterial3d(wood_mat.clone()),
            Transform::from_xyz(0.0, 3.75, -1.5),
            avian3d::prelude::RigidBody::Static,
            avian3d::prelude::Collider::cuboid(3.1, 0.9, 0.1),
            crate::play_mode::PlayModeEntity,
        ))
        .id();
    let r3 = commands
        .spawn((
            Mesh3d(rail_mesh_back),
            MeshMaterial3d(wood_mat.clone()),
            Transform::from_xyz(-1.5, 3.75, 0.0),
            avian3d::prelude::RigidBody::Static,
            avian3d::prelude::Collider::cuboid(0.1, 0.9, 3.1),
            crate::play_mode::PlayModeEntity,
        ))
        .id();
    commands.entity(parent).add_child(r1);
    commands.entity(parent).add_child(r2);
    commands.entity(parent).add_child(r3);

    // Ladder Rungs on Right Post
    let rung_mesh = meshes.add(Cuboid::new(0.1, 0.06, 0.8));
    for rung_idx in 0..12 {
        let ry = 0.3 + (rung_idx as f32) * 0.26;
        let rung = commands
            .spawn((
                Mesh3d(rung_mesh.clone()),
                MeshMaterial3d(wood_mat.clone()),
                Transform::from_xyz(1.52, ry, 0.0),
                avian3d::prelude::RigidBody::Static,
                avian3d::prelude::Collider::cuboid(0.1, 0.06, 0.8),
                crate::play_mode::PlayModeEntity,
            ))
            .id();
        commands.entity(parent).add_child(rung);
    }

    // Roof Canopy (at Y = 5.1m)
    let roof_mat = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/red_roof_shingles.png")),
        perceptual_roughness: 0.7,
        ..default()
    });
    let roof_mesh = meshes.add(Cuboid::new(3.4, 0.35, 3.4));
    let roof = commands
        .spawn((
            Mesh3d(roof_mesh),
            MeshMaterial3d(roof_mat),
            Transform::from_xyz(0.0, 5.1, 0.0),
            avian3d::prelude::RigidBody::Static,
            avian3d::prelude::Collider::cuboid(3.4, 0.35, 3.4),
            crate::play_mode::PlayModeEntity,
        ))
        .id();
    commands.entity(parent).add_child(roof);

    // Hanging Lantern PointLight
    let light = commands
        .spawn((
            PointLight {
                color: Color::srgb(1.0, 0.8, 0.4),
                intensity: 1500.0,
                range: 18.0,
                shadow_maps_enabled: true,
                ..default()
            },
            Transform::from_xyz(0.0, 4.8, 0.0),
            crate::play_mode::PlayModeEntity,
        ))
        .id();
    commands.entity(parent).add_child(light);
}

/// 3. Modular Step Staircase (+2.5m elevation)
fn spawn_staircase(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    parent: Entity,
) {
    let wood_mat = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/wood_planks.png")),
        perceptual_roughness: 0.8,
        ..default()
    });
    let step_mesh = meshes.add(Cuboid::new(1.8, 0.22, 0.4));
    let num_steps = 10;
    let step_height = 0.25;
    let step_depth = 0.32;

    for i in 0..num_steps {
        let sy = 0.11 + (i as f32) * step_height;
        let sz = -1.5 + (i as f32) * step_depth;

        let step = commands
            .spawn((
                Mesh3d(step_mesh.clone()),
                MeshMaterial3d(wood_mat.clone()),
                Transform::from_xyz(0.0, sy, sz),
                avian3d::prelude::RigidBody::Static,
                avian3d::prelude::Collider::cuboid(1.8, 0.22, 0.4),
                crate::play_mode::PlayModeEntity,
            ))
            .id();
        commands.entity(parent).add_child(step);
    }

    // Side Safety Handrails
    let handrail_mesh = meshes.add(Cuboid::new(0.08, 0.08, 3.6));
    let handrail_rot = Quat::from_rotation_x(-0.65);

    let hr_left = commands
        .spawn((
            Mesh3d(handrail_mesh.clone()),
            MeshMaterial3d(wood_mat.clone()),
            Transform::from_xyz(-0.95, 1.8, -0.1).with_rotation(handrail_rot),
            avian3d::prelude::RigidBody::Static,
            avian3d::prelude::Collider::cuboid(0.08, 0.08, 3.6),
            crate::play_mode::PlayModeEntity,
        ))
        .id();
    let hr_right = commands
        .spawn((
            Mesh3d(handrail_mesh),
            MeshMaterial3d(wood_mat),
            Transform::from_xyz(0.95, 1.8, -0.1).with_rotation(handrail_rot),
            avian3d::prelude::RigidBody::Static,
            avian3d::prelude::Collider::cuboid(0.08, 0.08, 3.6),
            crate::play_mode::PlayModeEntity,
        ))
        .id();

    commands.entity(parent).add_child(hr_left);
    commands.entity(parent).add_child(hr_right);
}

/// 4. Inclined Ramp (+2.5m elevation)
fn spawn_ramp(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    parent: Entity,
) {
    let wood_mat = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/wood_planks.png")),
        perceptual_roughness: 0.85,
        ..default()
    });
    let ramp_mesh = meshes.add(Cuboid::new(2.2, 0.18, 4.2));
    let ramp_rot = Quat::from_rotation_x(-0.55);

    let ramp = commands
        .spawn((
            Mesh3d(ramp_mesh),
            MeshMaterial3d(wood_mat.clone()),
            Transform::from_xyz(0.0, 1.25, 0.0).with_rotation(ramp_rot),
            avian3d::prelude::RigidBody::Static,
            avian3d::prelude::Collider::cuboid(2.2, 0.18, 4.2),
            crate::play_mode::PlayModeEntity,
        ))
        .id();
    commands.entity(parent).add_child(ramp);

    // Non-slip timber cleats along ramp surface
    let cleat_mesh = meshes.add(Cuboid::new(2.1, 0.05, 0.08));
    for c_idx in 0..8 {
        let cz = -1.6 + (c_idx as f32) * 0.45;
        let cy = 0.1 + (c_idx as f32) * 0.28;
        let cleat = commands
            .spawn((
                Mesh3d(cleat_mesh.clone()),
                MeshMaterial3d(wood_mat.clone()),
                Transform::from_xyz(0.0, cy + 0.1, cz).with_rotation(ramp_rot),
                crate::play_mode::PlayModeEntity,
            ))
            .id();
        commands.entity(parent).add_child(cleat);
    }
}

/// 5. Wooden Plank Bridge
fn spawn_wooden_bridge(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    parent: Entity,
) {
    let wood_mat = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/wood_planks.png")),
        perceptual_roughness: 0.85,
        ..default()
    });

    let deck_mesh = meshes.add(Cuboid::new(2.4, 0.25, 4.0));
    let deck = commands
        .spawn((
            Mesh3d(deck_mesh),
            MeshMaterial3d(wood_mat.clone()),
            Transform::from_xyz(0.0, 0.125, 0.0),
            avian3d::prelude::RigidBody::Static,
            avian3d::prelude::Collider::cuboid(2.4, 0.25, 4.0),
            crate::play_mode::PlayModeEntity,
        ))
        .id();
    commands.entity(parent).add_child(deck);

    // Side Railings
    let rail_mesh = meshes.add(Cuboid::new(0.1, 0.9, 4.0));
    let r_left = commands
        .spawn((
            Mesh3d(rail_mesh.clone()),
            MeshMaterial3d(wood_mat.clone()),
            Transform::from_xyz(-1.2, 0.65, 0.0),
            avian3d::prelude::RigidBody::Static,
            avian3d::prelude::Collider::cuboid(0.1, 0.9, 4.0),
            crate::play_mode::PlayModeEntity,
        ))
        .id();
    let r_right = commands
        .spawn((
            Mesh3d(rail_mesh),
            MeshMaterial3d(wood_mat),
            Transform::from_xyz(1.2, 0.65, 0.0),
            avian3d::prelude::RigidBody::Static,
            avian3d::prelude::Collider::cuboid(0.1, 0.9, 4.0),
            crate::play_mode::PlayModeEntity,
        ))
        .id();

    commands.entity(parent).add_child(r_left);
    commands.entity(parent).add_child(r_right);
}

/// 6. Palisade Stake Fence
fn spawn_palisade_fence(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    parent: Entity,
) {
    let wood_mat = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/wood_planks.png")),
        perceptual_roughness: 0.9,
        ..default()
    });

    // 8 Sharpened wooden stakes bound together
    let stake_mesh = meshes.add(Cylinder::new(0.09, 2.2));
    for idx in 0..8 {
        let sx = -1.05 + (idx as f32) * 0.3;
        let stake = commands
            .spawn((
                Mesh3d(stake_mesh.clone()),
                MeshMaterial3d(wood_mat.clone()),
                Transform::from_xyz(sx, 1.1, 0.0),
                avian3d::prelude::RigidBody::Static,
                avian3d::prelude::Collider::cylinder(0.09, 2.2),
                crate::play_mode::PlayModeEntity,
            ))
            .id();
        commands.entity(parent).add_child(stake);
    }

    // Horizontal Iron Binding Straps
    let strap_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.2, 0.22),
        metallic: 0.8,
        perceptual_roughness: 0.4,
        ..default()
    });
    let strap_mesh = meshes.add(Cuboid::new(2.5, 0.06, 0.22));
    for sy in [0.6, 1.6] {
        let strap = commands
            .spawn((
                Mesh3d(strap_mesh.clone()),
                MeshMaterial3d(strap_mat.clone()),
                Transform::from_xyz(0.0, sy, 0.0),
                crate::play_mode::PlayModeEntity,
            ))
            .id();
        commands.entity(parent).add_child(strap);
    }
}

/// 7. Granite Fortress Wall with Battlements
fn spawn_granite_fortress_wall(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    parent: Entity,
) {
    let stone_mat = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/solid_stone.png")),
        perceptual_roughness: 0.95,
        metallic: 0.0,
        reflectance: 0.05,
        ..default()
    });

    let main_wall_mesh = meshes.add(Cuboid::new(2.5, 2.8, 0.6));
    let wall = commands
        .spawn((
            Mesh3d(main_wall_mesh),
            MeshMaterial3d(stone_mat.clone()),
            Transform::from_xyz(0.0, 1.4, 0.0),
            avian3d::prelude::RigidBody::Static,
            avian3d::prelude::Collider::cuboid(2.5, 2.8, 0.6),
            crate::play_mode::PlayModeEntity,
        ))
        .id();
    commands.entity(parent).add_child(wall);

    // Battlements (Crenellations) on top
    let merlon_mesh = meshes.add(Cuboid::new(0.65, 0.6, 0.6));
    for mx in [-0.9, 0.9] {
        let merlon = commands
            .spawn((
                Mesh3d(merlon_mesh.clone()),
                MeshMaterial3d(stone_mat.clone()),
                Transform::from_xyz(mx, 3.1, 0.0),
                avian3d::prelude::RigidBody::Static,
                avian3d::prelude::Collider::cuboid(0.65, 0.6, 0.6),
                crate::play_mode::PlayModeEntity,
            ))
            .id();
        commands.entity(parent).add_child(merlon);
    }
}

/// 8. Log Cabin Timber Wall
fn spawn_log_timber_wall(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    parent: Entity,
) {
    let wood_mat = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/wood_planks.png")),
        perceptual_roughness: 0.85,
        ..default()
    });

    let log_mesh = meshes.add(Cylinder::new(0.14, 2.6));

    // 8 Horizontally stacked cylindrical logs
    for log_idx in 0..8 {
        let ly = 0.15 + (log_idx as f32) * 0.27;
        let log = commands
            .spawn((
                Mesh3d(log_mesh.clone()),
                MeshMaterial3d(wood_mat.clone()),
                Transform::from_xyz(0.0, ly, 0.0)
                    .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                avian3d::prelude::RigidBody::Static,
                avian3d::prelude::Collider::cylinder(0.14, 2.6),
                crate::play_mode::PlayModeEntity,
            ))
            .id();
        commands.entity(parent).add_child(log);
    }

    // Corner Posts
    let corner_mesh = meshes.add(Cuboid::new(0.3, 2.4, 0.3));
    for cx in [-1.25, 1.25] {
        let c_post = commands
            .spawn((
                Mesh3d(corner_mesh.clone()),
                MeshMaterial3d(wood_mat.clone()),
                Transform::from_xyz(cx, 1.2, 0.0),
                avian3d::prelude::RigidBody::Static,
                avian3d::prelude::Collider::cuboid(0.3, 2.4, 0.3),
                crate::play_mode::PlayModeEntity,
            ))
            .id();
        commands.entity(parent).add_child(c_post);
    }
}

/// 9. Cyber Metal Wall with Cyan Emissive Conduit Lines
fn spawn_cyber_metal_wall(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    parent: Entity,
) {
    let metal_mat = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/cyber_door.png")),
        metallic: 0.9,
        perceptual_roughness: 0.25,
        ..default()
    });

    let main_mesh = meshes.add(Cuboid::new(2.5, 2.6, 0.3));
    let wall = commands
        .spawn((
            Mesh3d(main_mesh),
            MeshMaterial3d(metal_mat),
            Transform::from_xyz(0.0, 1.3, 0.0),
            avian3d::prelude::RigidBody::Static,
            avian3d::prelude::Collider::cuboid(2.5, 2.6, 0.3),
            crate::play_mode::PlayModeEntity,
        ))
        .id();
    commands.entity(parent).add_child(wall);

    // Cyan Emissive Conduit Light Lines
    let cyan_emissive_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 0.85, 1.0),
        emissive: LinearRgba::new(0.5, 6.0, 10.0, 1.0),
        unlit: true,
        ..default()
    });
    let conduit_mesh = meshes.add(Cuboid::new(2.4, 0.06, 0.34));
    for cy in [0.7, 1.9] {
        let conduit = commands
            .spawn((
                Mesh3d(conduit_mesh.clone()),
                MeshMaterial3d(cyan_emissive_mat.clone()),
                Transform::from_xyz(0.0, cy, 0.0),
                crate::play_mode::PlayModeEntity,
            ))
            .id();
        commands.entity(parent).add_child(conduit);
    }
}
