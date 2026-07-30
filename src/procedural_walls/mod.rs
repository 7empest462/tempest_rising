//! Procedural wall generation module for creating dynamic brick walls.

pub mod arch;
pub mod brick;
pub mod curve;
pub mod wall_constructor;

pub use arch::{
    ArchBrick, ArchOpening, MAX_ARCH_SPAN, MIN_ARCH_SPAN, WallEndpoint, find_arch_openings,
    generate_arch, voussoir_spawn_delay,
};
pub use brick::Brick;
pub use curve::Curve;
pub use wall_constructor::WallConstructor;

use crate::map_editor::data::TempestMap;
use crate::play_mode::get_bilinear_height;
use bevy::prelude::*;
use rand::RngExt;
use std::collections::HashMap;

/// Plugin for procedural brick wall construction and destruction.
pub struct ProceduralWallsPlugin;

impl Plugin for ProceduralWallsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProceduralWallBuilder>()
            .init_resource::<ArchRegistry>()
            .init_resource::<ProceduralWallPreviewCache>()
            .init_resource::<ProceduralWallAssets>()
            .add_systems(
                Update,
                (
                    update_wall_builder,
                    draw_wall_preview,
                    animate_brick_spawns,
                    carve_gateways,
                    detect_and_spawn_arches,
                    particle_despawn_system,
                ),
            );
    }
}
/// Selectable visual and material style for procedural wall curve construction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WallStyle {
    #[default]
    ClassicBrick,
    PalisadeFence,
    GraniteFortress,
    LogTimber,
    CyberMetal,
}

/// Active builder state for placing procedural wall curves
#[derive(Resource)]
pub struct ProceduralWallBuilder {
    /// Placed control points
    pub points: Vec<Vec3>,
    /// Selected height for the wall (adjustable dynamically!)
    pub height: f32,
    /// Selected wall style
    pub style: WallStyle,
    /// Whether build mode is toggled active (play mode only)
    pub active: bool,
    /// Current hovered target point under cursor
    pub hover_point: Option<Vec3>,
}

impl Default for ProceduralWallBuilder {
    fn default() -> Self {
        Self {
            points: Vec::new(),
            height: 2.4, // Default wall height
            style: WallStyle::ClassicBrick,
            active: false,
            hover_point: None,
        }
    }
}

/// Cache resource for real-time holographic brick/curve preview.
/// Saves CPU cycles by avoiding expensive voxel searches and curve resampling
/// when the control points and height are static.
#[derive(Resource, Default)]
pub struct ProceduralWallPreviewCache {
    pub points: Vec<Vec3>,
    pub height: f32,
    pub style: WallStyle,
    pub cached_bricks: Vec<Brick>,
    pub cached_voussoirs: Vec<ArchBrick>,
}

/// Animates a brick scaling and dropping down upon spawning
#[derive(Component)]
pub struct BrickSpawnAnimation {
    pub target_translation: Vec3,
    pub target_scale: Vec3,
    pub delay: f32,
    pub elapsed: f32,
    pub duration: f32,
}

/// Marker component for each individual generated wall brick entity
#[derive(Component)]
pub struct ProceduralBrick;

/// Marker component for clay red brick masonry walls (enables arch spawning & gateway carving)
#[derive(Component)]
pub struct ProceduralMasonryBrick;

/// Marker component for voussoir (arch) bricks — sub-type of ProceduralBrick.
/// Both components are present on arch bricks, so mining/health work automatically.
#[derive(Component)]
#[allow(dead_code)]
pub struct ProceduralArchBrick {
    /// ID linking all voussoirs in the same arch together.
    pub arch_id: u64,
}

#[derive(Component)]
pub struct Particle {
    pub velocity: Vec3,
    pub lifetime: Timer,
}

#[derive(Component)]
pub struct Hittable;

#[derive(Component)]
#[allow(dead_code)]
pub struct Health(pub f32);

impl Health {
    pub fn new(val: f32) -> Self {
        Self(val)
    }
}

#[derive(Component)]
#[allow(dead_code)]
pub struct Door {
    pub open: bool,
    pub hinge_side: f32,
    pub is_open: bool,
    pub original_rotation: Quat,
}

pub fn particle_despawn_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Particle, &mut Transform)>,
) {
    for (entity, mut particle, mut transform) in query.iter_mut() {
        particle.lifetime.tick(time.delta());
        if particle.lifetime.fraction() >= 1.0 {
            commands.entity(entity).despawn();
        } else {
            transform.translation += particle.velocity * time.delta_secs();
        }
    }
}

/// Tracks all currently-live auto-detected arch openings so we don't re-spawn
/// them every frame.
#[derive(Resource, Default)]
pub struct ArchRegistry {
    /// Each entry is (left_foot_xz, right_foot_xz, root_entity).
    pub arches: Vec<(bevy::math::Vec2, bevy::math::Vec2, Entity)>,
}

/// Cached mesh and material assets for procedural walls and their particles
#[derive(Resource)]
#[allow(dead_code)]
pub struct ProceduralWallAssets {
    pub unit_cube: Handle<Mesh>,
    pub spark_mesh: Handle<Mesh>,
    pub spark_material: Handle<StandardMaterial>,
    pub dust_mesh: Handle<Mesh>,
    pub dust_material: Handle<StandardMaterial>,
    pub splinter_mesh: Handle<Mesh>,
    pub splinter_material: Handle<StandardMaterial>,
}

impl FromWorld for ProceduralWallAssets {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let unit_cube = meshes.add(Cuboid::from_size(Vec3::ONE));
        let spark_mesh = meshes.add(Cuboid::from_size(Vec3::splat(0.08)));
        let dust_mesh = meshes.add(Cuboid::from_size(Vec3::splat(0.12)));
        let splinter_mesh = meshes.add(Cuboid::from_size(Vec3::splat(0.1)));

        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        let spark_material = materials.add(StandardMaterial {
            base_color: Color::from(bevy::color::palettes::css::GOLD),
            emissive: LinearRgba::from(Color::srgb(1.0, 0.8, 0.0)),
            ..default()
        });
        let dust_material = materials.add(StandardMaterial {
            base_color: Color::srgba(0.8, 0.75, 0.7, 0.45),
            alpha_mode: AlphaMode::Blend,
            ..default()
        });
        let splinter_material = materials.add(StandardMaterial {
            base_color: Color::srgba(0.6, 0.4, 0.35, 1.0),
            ..default()
        });

        Self {
            unit_cube,
            spark_mesh,
            spark_material,
            dust_mesh,
            dust_material,
            splinter_mesh,
            splinter_material,
        }
    }
}

#[derive(Clone, Copy)]
struct BrickAdjacency {
    is_left_edge: bool,
    is_right_edge: bool,
    is_top_edge: bool,
    is_bottom_edge: bool,
}

fn compute_brick_adjacency(bricks: &[Brick]) -> Vec<BrickAdjacency> {
    let mut left_edges: HashMap<(i32, i32), Vec<usize>> = HashMap::with_capacity(bricks.len());
    let mut right_edges: HashMap<(i32, i32), Vec<usize>> = HashMap::with_capacity(bricks.len());
    let mut top_edges: HashMap<i32, Vec<usize>> = HashMap::new();
    let mut bottom_edges: HashMap<i32, Vec<usize>> = HashMap::new();

    for (idx, brick) in bricks.iter().enumerate() {
        let half_bounds = brick.bounds_uv * 0.5;
        let left = brick.pivot_uv.x - half_bounds.x;
        let right = brick.pivot_uv.x + half_bounds.x;
        let bottom = brick.pivot_uv.y - half_bounds.y;
        let top = brick.pivot_uv.y + half_bounds.y;
        let row_key = adjacency_key(brick.pivot_uv.y);

        left_edges
            .entry((adjacency_key(left), row_key))
            .or_default()
            .push(idx);
        right_edges
            .entry((adjacency_key(right), row_key))
            .or_default()
            .push(idx);
        bottom_edges
            .entry(adjacency_key(bottom))
            .or_default()
            .push(idx);
        top_edges.entry(adjacency_key(top)).or_default().push(idx);
    }

    bricks
        .iter()
        .enumerate()
        .map(|(idx, brick)| {
            let half_bounds = brick.bounds_uv * 0.5;
            let left = brick.pivot_uv.x - half_bounds.x;
            let right = brick.pivot_uv.x + half_bounds.x;
            let bottom = brick.pivot_uv.y - half_bounds.y;
            let top = brick.pivot_uv.y + half_bounds.y;

            let has_left_neighbor = has_side_neighbor(&right_edges, left, brick.pivot_uv.y, idx);
            let has_right_neighbor = has_side_neighbor(&left_edges, right, brick.pivot_uv.y, idx);
            let has_top_neighbor = has_vertical_neighbor(&bottom_edges, top, |other_idx| {
                let other = &bricks[other_idx];
                other.pivot_uv.y > brick.pivot_uv.y
                    && spans_overlap(
                        brick.pivot_uv.x,
                        brick.bounds_uv.x,
                        other.pivot_uv.x,
                        other.bounds_uv.x,
                    )
            });
            let has_bottom_neighbor = has_vertical_neighbor(&top_edges, bottom, |other_idx| {
                let other = &bricks[other_idx];
                other.pivot_uv.y < brick.pivot_uv.y
                    && spans_overlap(
                        brick.pivot_uv.x,
                        brick.bounds_uv.x,
                        other.pivot_uv.x,
                        other.bounds_uv.x,
                    )
            });

            BrickAdjacency {
                is_left_edge: !has_left_neighbor,
                is_right_edge: !has_right_neighbor,
                is_top_edge: !has_top_neighbor,
                is_bottom_edge: !has_bottom_neighbor,
            }
        })
        .collect()
}

fn has_side_neighbor(
    edge_map: &HashMap<(i32, i32), Vec<usize>>,
    edge: f32,
    row: f32,
    current_idx: usize,
) -> bool {
    let edge_key = adjacency_key(edge);
    let row_key = adjacency_key(row);

    for nearby_edge in (edge_key - 1)..=(edge_key + 1) {
        for nearby_row in (row_key - 1)..=(row_key + 1) {
            if edge_map
                .get(&(nearby_edge, nearby_row))
                .is_some_and(|neighbors| neighbors.iter().any(|&idx| idx != current_idx))
            {
                return true;
            }
        }
    }

    false
}

fn has_vertical_neighbor(
    edge_map: &HashMap<i32, Vec<usize>>,
    edge: f32,
    mut predicate: impl FnMut(usize) -> bool,
) -> bool {
    let edge_key = adjacency_key(edge);

    ((edge_key - 1)..=(edge_key + 1)).any(|nearby_edge| {
        edge_map
            .get(&nearby_edge)
            .is_some_and(|neighbors| neighbors.iter().any(|&idx| predicate(idx)))
    })
}

fn adjacency_key(value: f32) -> i32 {
    (value / 0.02).round() as i32
}

fn spans_overlap(a_center: f32, a_width: f32, b_center: f32, b_width: f32) -> bool {
    (b_center - a_center).abs() < (a_width + b_width) * 0.5 - 0.01
}

#[allow(clippy::too_many_arguments)]
fn update_wall_builder(
    mut builder: ResMut<ProceduralWallBuilder>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    map: Res<TempestMap>,
    gamepads: Query<&Gamepad>,
    procedural_wall_assets: Res<ProceduralWallAssets>,
    camera_query: Query<(
        &Camera,
        &GlobalTransform,
        Option<&crate::play_mode::PlayModeCamera>,
        Option<&crate::map_editor::EditorCamera>,
    )>,
    window: Query<&Window>,
    brush_settings: Option<Res<crate::map_editor::BrushSettings>>,
    state: Res<State<crate::AppState>>,
) {
    let is_editor = *state.get() == crate::AppState::MapEditor;
    let is_play = *state.get() == crate::AppState::PlayMode;

    if is_play && keys.just_pressed(KeyCode::KeyB) {
        builder.active = !builder.active;
        if builder.active {
            crate::play_mode::inventory_log(
                "🔨 Entering Build Mode. Left-Click to place wall points, Enter to build, B to exit.",
            );
        } else {
            builder.points.clear();
            builder.hover_point = None;
            crate::play_mode::inventory_log("⚔️ Exiting Build Mode. Weapon active.");
        }
    }

    let is_building = if is_editor {
        brush_settings
            .as_ref()
            .map(|s| s.tool == crate::map_editor::SculptTool::PlaceProceduralWall)
            .unwrap_or(false)
    } else if is_play {
        builder.active
    } else {
        false
    };

    if !is_building {
        return;
    }

    let Ok(win) = window.single() else {
        return;
    };

    // In play mode, the cursor is grabbed, so we raycast from the center of the screen
    let cursor_pos = if is_play {
        Vec2::new(win.width() / 2.0, win.height() / 2.0)
    } else {
        let Some(c) = win.cursor_position() else {
            return;
        };
        c
    };

    let mut ray_opt = None;
    for (camera, camera_transform, play_cam, edit_cam) in camera_query.iter() {
        if camera.is_active
            && ((is_play && play_cam.is_some()) || (is_editor && edit_cam.is_some()))
            && let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos)
        {
            ray_opt = Some(ray);
            break;
        }
    }
    let Some(ray) = ray_opt else {
        return;
    };

    let mut hover_point = None;

    // Raymarch against terrain height
    let mut d = 0.0;
    let step = 0.4;
    let max_dist = 60.0;
    let mut hit = false;

    while d < max_dist {
        let p = ray.origin + ray.direction * d;
        let half_w = map.width as f32 / 2.0;
        let half_h = map.height as f32 / 2.0;
        if p.x.abs() > half_w || p.z.abs() > half_h {
            break;
        }
        let ground_y = get_bilinear_height(p.x, p.z, &map);
        if p.y <= ground_y {
            // Precise intersection using interpolation with previous step
            let prev_p = ray.origin + ray.direction * (d - step);
            let prev_ground_y = get_bilinear_height(prev_p.x, prev_p.z, &map);
            let t_factor = if ((p.y - ground_y) - (prev_p.y - prev_ground_y)).abs() > 0.001 {
                (prev_p.y - prev_ground_y) / ((prev_p.y - prev_ground_y) - (p.y - ground_y))
            } else {
                0.5
            };
            let hit_pos = prev_p + (p - prev_p) * t_factor.clamp(0.0, 1.0);
            hover_point = Some(hit_pos);
            hit = true;
            break;
        }
        d += step;
    }

    // Fallback to Y=0 plane if raymarch did not hit
    if !hit && ray.direction.y < -0.01 {
        let t = -ray.origin.y / ray.direction.y;
        let hover_pos_flat = ray.origin + ray.direction * t;
        let ground_y = get_bilinear_height(hover_pos_flat.x, hover_pos_flat.z, &map);
        hover_point = Some(Vec3::new(hover_pos_flat.x, ground_y, hover_pos_flat.z));
    }

    builder.hover_point = hover_point;

    let mut gamepad_place = false;
    let mut gamepad_undo = false;
    let mut gamepad_cancel = false;
    let mut gamepad_height_up = false;
    let mut gamepad_height_down = false;
    let mut gamepad_build = false;

    for gamepad in gamepads.iter() {
        if gamepad.just_pressed(GamepadButton::LeftTrigger2) {
            gamepad_place = true;
        }
        if gamepad.just_pressed(GamepadButton::LeftTrigger) {
            gamepad_undo = true;
        }
        if gamepad.just_pressed(GamepadButton::East) {
            gamepad_cancel = true;
        }
        if gamepad.pressed(GamepadButton::DPadRight) {
            gamepad_height_up = true;
        }
        if gamepad.pressed(GamepadButton::DPadLeft) {
            gamepad_height_down = true;
        }
        if gamepad.just_pressed(GamepadButton::RightTrigger2) {
            gamepad_build = true;
        }
    }

    // 1. Place curve control point (Left-Click in play mode, Right-Click in editor mode, or LT)
    let place_triggered = if is_play {
        mouse_input.just_pressed(MouseButton::Left) || gamepad_place
    } else {
        mouse_input.just_pressed(MouseButton::Right) || gamepad_place
    };

    if place_triggered && let Some(pt) = hover_point {
        builder.points.push(pt);

        // Visual feedback spark
        let mut rng = rand::rng();
        for _ in 0..4 {
            commands.spawn((
                Mesh3d(procedural_wall_assets.spark_mesh.clone()),
                MeshMaterial3d(procedural_wall_assets.spark_material.clone()),
                Transform::from_translation(pt),
                Particle {
                    velocity: Vec3::new(
                        rng.random_range(-1.5..1.5),
                        rng.random_range(1.5..3.5),
                        rng.random_range(-1.5..1.5),
                    ),
                    lifetime: Timer::from_seconds(0.4, TimerMode::Once),
                },
            ));
        }
    }

    // 2. Undo last point (Backspace, Delete, or LB)
    if keys.just_pressed(KeyCode::Backspace) || keys.just_pressed(KeyCode::Delete) || gamepad_undo {
        builder.points.pop();
    }

    // 3. Cancel build (Escape or B Button)
    if keys.just_pressed(KeyCode::Escape) || gamepad_cancel {
        builder.points.clear();
        builder.hover_point = None;
        if is_play {
            builder.active = false;
            crate::play_mode::inventory_log("⚔️ Exiting Build Mode. Weapon active.");
        }
    }

    // 4. Dynamic height adjustments (Up/Down arrow keys or D-Pad Right/Left)
    if keys.pressed(KeyCode::ArrowUp) || gamepad_height_up {
        builder.height = (builder.height + 0.04).min(6.0); // Maximum 6.0m high
    }
    if keys.pressed(KeyCode::ArrowDown) || gamepad_height_down {
        builder.height = (builder.height - 0.04).max(0.4); // Minimum 0.4m high (at least 1 row)
    }

    // 5. Confirm and build wall (Enter/Return or RT)
    if (keys.just_pressed(KeyCode::Enter)
        || keys.just_pressed(KeyCode::NumpadEnter)
        || gamepad_build)
        && builder.points.len() >= 2
    {
        let style = builder.style;
        let raw_curve = Curve::from(builder.points.clone()).smooth(2);

        match style {
            WallStyle::PalisadeFence => {
                let resampled_curve = raw_curve.resample(1.2);
                spawn_procedural_palisade_fence(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &asset_server,
                    &resampled_curve,
                    builder.height,
                    |pos| get_bilinear_height(pos.x, pos.z, &map),
                );
            }
            WallStyle::GraniteFortress => {
                let resampled_curve = raw_curve.resample(2.4);
                spawn_procedural_granite_fortress(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &asset_server,
                    &resampled_curve,
                    builder.height,
                    |pos| get_bilinear_height(pos.x, pos.z, &map),
                );
            }
            WallStyle::LogTimber => {
                let resampled_curve = raw_curve.resample(2.4);
                spawn_procedural_log_timber(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &asset_server,
                    &resampled_curve,
                    builder.height,
                    |pos| get_bilinear_height(pos.x, pos.z, &map),
                );
            }
            WallStyle::CyberMetal => {
                let resampled_curve = raw_curve.resample(2.4);
                spawn_procedural_cyber_metal(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    &asset_server,
                    &resampled_curve,
                    builder.height,
                    |pos| get_bilinear_height(pos.x, pos.z, &map),
                );
            }
            WallStyle::ClassicBrick => {
                let resampled_curve = raw_curve.resample(0.8);
                let bricks = WallConstructor::from_curve_with_style(
                    &resampled_curve,
                    builder.height,
                    style,
                    |pos| get_bilinear_height(pos.x, pos.z, &map),
                );
                let adjacency = compute_brick_adjacency(&bricks);
                let active_texture_handle = Some(asset_server.load("textures/solid_brick.png"));
                let mortar_material = materials.add(StandardMaterial {
                    base_color: Color::srgb(0.88, 0.86, 0.82),
                    perceptual_roughness: 0.85,
                    metallic: 0.0,
                    ..default()
                });

                let mut rng = rand::rng();

                for (idx, brick) in bricks.iter().enumerate() {
                    let r_off: f32 = (rng.random::<f32>() - 0.5) * 0.12;
                    let g_off: f32 = (rng.random::<f32>() - 0.5) * 0.10;
                    let b_off: f32 = (rng.random::<f32>() - 0.5) * 0.10;
                    let brick_color = Color::srgba(
                        (0.76 + r_off).clamp(0.0_f32, 1.0_f32),
                        (0.44 + g_off).clamp(0.0_f32, 1.0_f32),
                        (0.30 + b_off).clamp(0.0_f32, 1.0_f32),
                        1.0,
                    );

                    let brick_pos = brick.transform.translation;
                    let stagger_delay = brick.pivot_uv.y * 0.35 + brick.pivot_uv.x * 0.15;
                    commands
                        .spawn((
                            ProceduralBrick,
                            ProceduralMasonryBrick,
                            Hittable,
                            Health::new(35.0),
                            brick.transform.with_scale(brick.transform.scale * 0.01),
                            BrickSpawnAnimation {
                                target_translation: brick.transform.translation,
                                target_scale: brick.transform.scale,
                                delay: stagger_delay,
                                elapsed: 0.0,
                                duration: 0.42,
                            },
                            Visibility::default(),
                            InheritedVisibility::default(),
                        ))
                        .with_children(|parent| {
                            let brick_adjacency = adjacency[idx];
                            let is_left = brick_adjacency.is_left_edge;
                            let is_right = brick_adjacency.is_right_edge;
                            let is_top = brick_adjacency.is_top_edge;
                            let is_bottom = brick_adjacency.is_bottom_edge;

                            let left_shrink = if is_left { 0.0 } else { 0.02 };
                            let right_shrink = if is_right { 0.0 } else { 0.02 };
                            let bottom_shrink = if is_bottom { 0.0 } else { 0.02 };
                            let top_shrink = if is_top { 0.0 } else { 0.02 };

                            let rel_x = ((brick.transform.scale.x - (left_shrink + right_shrink))
                                / brick.transform.scale.x)
                                .max(0.1);
                            let rel_y = ((brick.transform.scale.y - (bottom_shrink + top_shrink))
                                / brick.transform.scale.y)
                                .max(0.1);

                            let trans_x =
                                (left_shrink - right_shrink) / (2.0 * brick.transform.scale.x);
                            let trans_y =
                                (bottom_shrink - top_shrink) / (2.0 * brick.transform.scale.y);

                            parent.spawn((
                                Mesh3d(procedural_wall_assets.unit_cube.clone()),
                                MeshMaterial3d(materials.add(StandardMaterial {
                                    base_color: brick_color,
                                    base_color_texture: active_texture_handle.clone(),
                                    perceptual_roughness: 0.85,
                                    metallic: 0.0,
                                    ..default()
                                })),
                                Transform {
                                    translation: Vec3::new(trans_x, trans_y, 0.0),
                                    scale: Vec3::new(rel_x, rel_y, 1.05),
                                    ..default()
                                },
                            ));

                            let mortar_left_inset = if is_left { 0.04 } else { 0.0 };
                            let mortar_right_inset = if is_right { 0.04 } else { 0.0 };
                            let mortar_bottom_inset = if is_bottom { 0.04 } else { 0.0 };
                            let mortar_top_inset = if is_top { 0.04 } else { 0.0 };

                            let mortar_rel_x = 1.02
                                - (mortar_left_inset + mortar_right_inset)
                                    / brick.transform.scale.x;
                            let mortar_rel_y = 1.02
                                - (mortar_top_inset + mortar_bottom_inset)
                                    / brick.transform.scale.y;

                            let mortar_trans_x = (mortar_left_inset - mortar_right_inset)
                                / (2.0 * brick.transform.scale.x);
                            let mortar_trans_y = (mortar_bottom_inset - mortar_top_inset)
                                / (2.0 * brick.transform.scale.y);

                            parent.spawn((
                                Mesh3d(procedural_wall_assets.unit_cube.clone()),
                                MeshMaterial3d(mortar_material.clone()),
                                Transform {
                                    translation: Vec3::new(mortar_trans_x, mortar_trans_y, 0.0),
                                    scale: Vec3::new(
                                        mortar_rel_x.max(0.1),
                                        mortar_rel_y.max(0.1),
                                        0.80,
                                    ),
                                    ..default()
                                },
                            ));
                        });

                    if rng.random::<f32>() < 0.25 {
                        commands.spawn((
                            Mesh3d(procedural_wall_assets.dust_mesh.clone()),
                            MeshMaterial3d(procedural_wall_assets.dust_material.clone()),
                            Transform::from_translation(brick_pos),
                            Particle {
                                velocity: Vec3::new(
                                    rng.random::<f32>() - 0.5,
                                    rng.random::<f32>() * 1.5,
                                    rng.random::<f32>() - 0.5,
                                ),
                                lifetime: Timer::from_seconds(0.7, TimerMode::Once),
                            },
                        ));
                    }
                }
            }
        }

        builder.points.clear();
        builder.hover_point = None;
        if is_play {
            builder.active = false;
            crate::play_mode::inventory_log("🧱 Wall constructed! Exiting Build Mode.");
        }
    }
}

// ---------------------------------------------------------------------------
// Arch voussoir spawn helper (shared by carve_gateways and detect_and_spawn_arches)
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn spawn_arch_voussoirs(
    opening: &ArchOpening,
    arch_id: u64,
    active_texture: &'static str,
    parent: Option<Entity>,
    commands: &mut Commands,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    procedural_wall_assets: &Res<ProceduralWallAssets>,
) {
    let voussoirs = generate_arch(opening);
    if voussoirs.is_empty() {
        return;
    }

    let mut rng = rand::rng();

    // Slightly darker than regular wall bricks to visually differentiate the arch
    let (base_r, base_g, base_b) = match active_texture {
        "textures/solid_stone.png" => (0.54, 0.54, 0.56),
        "textures/solid_brick.png" => (0.67, 0.39, 0.26),
        "textures/solid_limestone.png" => (0.75, 0.72, 0.65),
        _ => (0.54, 0.42, 0.37),
    };
    let mortar_color = match active_texture {
        "textures/solid_stone.png" => Color::srgb(0.78, 0.78, 0.76),
        "textures/solid_brick.png" => Color::srgb(0.88, 0.86, 0.82),
        "textures/solid_limestone.png" => Color::srgb(0.68, 0.66, 0.62),
        _ => Color::srgb(0.80, 0.80, 0.80),
    };
    let active_texture_handle = asset_server.load(active_texture);
    let mortar_material = materials.add(StandardMaterial {
        base_color: mortar_color,
        perceptual_roughness: 0.95,
        metallic: 0.0,
        ..default()
    });

    for voussoir in &voussoirs {
        let r_off: f32 = (rng.random::<f32>() - 0.5) * 0.10;
        let g_off: f32 = (rng.random::<f32>() - 0.5) * 0.08;
        let b_off: f32 = (rng.random::<f32>() - 0.5) * 0.08;
        let brick_color = Color::srgba(
            (base_r + r_off).clamp(0.0_f32, 1.0_f32),
            (base_g + g_off).clamp(0.0_f32, 1.0_f32),
            (base_b + b_off).clamp(0.0_f32, 1.0_f32),
            1.0,
        );

        let delay = voussoir_spawn_delay(voussoir.arc_t);
        let target_translation = voussoir.transform.translation;
        let target_scale = voussoir.transform.scale;

        let child = commands
            .spawn((
                ProceduralBrick,
                ProceduralArchBrick { arch_id },
                Hittable,
                Health::new(45.0), // arch bricks are slightly tougher
                voussoir.transform.with_scale(Vec3::splat(0.01)),
                // Collider is added after the spawn animation completes to avoid heavy BVH refitting/stuttering every frame!
                BrickSpawnAnimation {
                    target_translation,
                    target_scale,
                    delay,
                    elapsed: 0.0,
                    duration: 0.42,
                },
                Visibility::default(),
                InheritedVisibility::default(),
            ))
            .with_children(|parent| {
                // Stone face
                parent.spawn((
                    Mesh3d(procedural_wall_assets.unit_cube.clone()),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: brick_color,
                        base_color_texture: Some(active_texture_handle.clone()),
                        perceptual_roughness: 0.90,
                        metallic: 0.01,
                        ..default()
                    })),
                    Transform {
                        translation: Vec3::ZERO,
                        scale: Vec3::new(1.02, 1.02, 1.05),
                        ..default()
                    },
                    crate::play_mode::WallCollider {
                        half_extents: Vec3::new(
                            voussoir.transform.scale.x * 0.5,
                            voussoir.transform.scale.y * 0.5,
                            voussoir.transform.scale.z * 0.5,
                        ),
                    },
                ));
                // Mortar backing
                parent.spawn((
                    Mesh3d(procedural_wall_assets.unit_cube.clone()),
                    MeshMaterial3d(mortar_material.clone()),
                    Transform {
                        translation: Vec3::ZERO,
                        scale: Vec3::new(0.96, 0.96, 0.80),
                        ..default()
                    },
                ));
            })
            .id();

        if let Some(p) = parent {
            commands.entity(p).add_child(child);
        }
    }
}

// ---------------------------------------------------------------------------
// Procedural Palisade Fence Spawner (Vertical Cylindrical Wooden Stakes & Iron Straps)
// ---------------------------------------------------------------------------
fn spawn_procedural_palisade_fence(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    curve: &Curve,
    wall_height: f32,
    get_ground_y: impl Fn(Vec3) -> f32,
) {
    let wood_mat = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/wood_planks.png")),
        perceptual_roughness: 0.90,
        ..default()
    });
    let strap_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.22, 0.25),
        metallic: 0.85,
        perceptual_roughness: 0.35,
        ..default()
    });

    let stake_radius = 0.09;
    let stake_mesh = meshes.add(Cylinder::new(stake_radius, wall_height));

    for i in 0..curve.points.len().saturating_sub(1) {
        let p0 = curve.points[i];
        let p1 = curve.points[i + 1];
        let seg_vec = p1 - p0;
        let seg_len = seg_vec.length();
        if seg_len < 0.05 {
            continue;
        }

        let dir = seg_vec / seg_len;
        let yaw = dir.z.atan2(dir.x);
        let rot = Quat::from_rotation_y(-yaw);

        // Spawn vertical wooden stakes spaced 0.25m apart along segment
        let stake_spacing = 0.25;
        let stake_count = (seg_len / stake_spacing).round().max(1.0) as usize;
        for s in 0..stake_count {
            let t = (s as f32 + 0.5) / (stake_count as f32);
            let pos = p0 + seg_vec * t;
            let gy = get_ground_y(pos);
            let stake_pos = Vec3::new(pos.x, gy + wall_height * 0.5, pos.z);

            commands.spawn((
                ProceduralBrick,
                Hittable,
                Health::new(35.0),
                Mesh3d(stake_mesh.clone()),
                MeshMaterial3d(wood_mat.clone()),
                Transform::from_translation(stake_pos),
                avian3d::prelude::RigidBody::Static,
                avian3d::prelude::Collider::cylinder(stake_radius, wall_height),
                crate::play_mode::PlayModeEntity,
            ));
        }

        // Horizontal Iron Binding Straps along segment
        let midpoint = (p0 + p1) * 0.5;
        let gy = get_ground_y(midpoint);
        let strap_mesh = meshes.add(Cuboid::new(0.22, 0.06, seg_len));

        for sy_offset in [0.35 * wall_height, 0.75 * wall_height] {
            let strap_pos = Vec3::new(midpoint.x, gy + sy_offset, midpoint.z);
            commands.spawn((
                Mesh3d(strap_mesh.clone()),
                MeshMaterial3d(strap_mat.clone()),
                Transform::from_translation(strap_pos)
                    .with_rotation(rot * Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
                crate::play_mode::PlayModeEntity,
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Procedural Granite Fortress Wall Spawner (Solid Granite Blocks + Battlements)
// ---------------------------------------------------------------------------
fn spawn_procedural_granite_fortress(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    curve: &Curve,
    wall_height: f32,
    get_ground_y: impl Fn(Vec3) -> f32,
) {
    let stone_mat = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/solid_stone.png")),
        perceptual_roughness: 0.95,
        metallic: 0.0,
        reflectance: 0.05,
        ..default()
    });

    let wall_thickness = 0.6;
    let merlon_height = 0.6;

    for i in 0..curve.points.len().saturating_sub(1) {
        let p0 = curve.points[i];
        let p1 = curve.points[i + 1];
        let seg_vec = p1 - p0;
        let seg_len = seg_vec.length();
        if seg_len < 0.05 {
            continue;
        }

        let dir = seg_vec / seg_len;
        let yaw = dir.z.atan2(dir.x);
        let rot = Quat::from_rotation_y(-yaw) * Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);

        let midpoint = (p0 + p1) * 0.5;
        let gy = get_ground_y(midpoint);
        let wall_pos = Vec3::new(midpoint.x, gy + wall_height * 0.5, midpoint.z);

        // Solid granite main wall block
        let wall_mesh = meshes.add(Cuboid::new(wall_thickness, wall_height, seg_len));
        commands.spawn((
            ProceduralBrick,
            Hittable,
            Health::new(60.0),
            Mesh3d(wall_mesh),
            MeshMaterial3d(stone_mat.clone()),
            Transform::from_translation(wall_pos).with_rotation(rot),
            avian3d::prelude::RigidBody::Static,
            avian3d::prelude::Collider::cuboid(wall_thickness, wall_height, seg_len),
            crate::play_mode::PlayModeEntity,
        ));

        // Battlements / Merlon Crenellations on top edge
        let merlon_spacing = 0.9;
        let merlon_count = (seg_len / merlon_spacing).round().max(1.0) as usize;
        let merlon_mesh = meshes.add(Cuboid::new(wall_thickness, merlon_height, 0.5));

        for m in 0..merlon_count {
            if m % 2 == 0 {
                let t = (m as f32 + 0.5) / (merlon_count as f32);
                let m_pos_xz = p0 + seg_vec * t;
                let m_pos = Vec3::new(
                    m_pos_xz.x,
                    gy + wall_height + merlon_height * 0.5,
                    m_pos_xz.z,
                );
                commands.spawn((
                    Mesh3d(merlon_mesh.clone()),
                    MeshMaterial3d(stone_mat.clone()),
                    Transform::from_translation(m_pos).with_rotation(rot),
                    avian3d::prelude::RigidBody::Static,
                    avian3d::prelude::Collider::cuboid(wall_thickness, merlon_height, 0.5),
                    crate::play_mode::PlayModeEntity,
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Procedural Log Cabin Timber Wall Spawner (Horizontal Stacked Round Logs & Corner Posts)
// ---------------------------------------------------------------------------
fn spawn_procedural_log_timber(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    curve: &Curve,
    wall_height: f32,
    get_ground_y: impl Fn(Vec3) -> f32,
) {
    let wood_mat = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/wood_planks.png")),
        perceptual_roughness: 0.85,
        ..default()
    });

    let log_radius = 0.14;
    let log_diameter = log_radius * 2.0;
    let log_count = (wall_height / (log_diameter * 0.95)).ceil().max(1.0) as usize;

    for i in 0..curve.points.len().saturating_sub(1) {
        let p0 = curve.points[i];
        let p1 = curve.points[i + 1];
        let seg_vec = p1 - p0;
        let seg_len = seg_vec.length();
        if seg_len < 0.05 {
            continue;
        }

        let dir = seg_vec / seg_len;
        let yaw = dir.z.atan2(dir.x);
        let rot = Quat::from_rotation_y(-yaw);

        let midpoint = (p0 + p1) * 0.5;
        let gy = get_ground_y(midpoint);

        let log_mesh = meshes.add(Cylinder::new(log_radius, seg_len));

        // Horizontally stacked round logs along segment
        for r in 0..log_count {
            let ly = gy + (r as f32 * log_diameter * 0.92) + log_radius;
            let log_pos = Vec3::new(midpoint.x, ly, midpoint.z);

            commands.spawn((
                ProceduralBrick,
                Hittable,
                Health::new(45.0),
                Mesh3d(log_mesh.clone()),
                MeshMaterial3d(wood_mat.clone()),
                Transform::from_translation(log_pos)
                    .with_rotation(rot * Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                avian3d::prelude::RigidBody::Static,
                avian3d::prelude::Collider::cylinder(log_radius, seg_len),
                crate::play_mode::PlayModeEntity,
            ));
        }

        // Vertical corner post at segment joints
        let corner_mesh = meshes.add(Cuboid::new(0.3, wall_height, 0.3));
        for pt in [p0, p1] {
            let p_gy = get_ground_y(pt);
            let corner_pos = Vec3::new(pt.x, p_gy + wall_height * 0.5, pt.z);
            commands.spawn((
                Mesh3d(corner_mesh.clone()),
                MeshMaterial3d(wood_mat.clone()),
                Transform::from_translation(corner_pos),
                avian3d::prelude::RigidBody::Static,
                avian3d::prelude::Collider::cuboid(0.3, wall_height, 0.3),
                crate::play_mode::PlayModeEntity,
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Procedural Cyber Metal Wall Spawner (Solid Alloy Panel + Cyan Glowing Conduit Lines)
// ---------------------------------------------------------------------------
fn spawn_procedural_cyber_metal(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    curve: &Curve,
    wall_height: f32,
    get_ground_y: impl Fn(Vec3) -> f32,
) {
    let metal_mat = materials.add(StandardMaterial {
        base_color_texture: Some(asset_server.load("textures/cyber_door.png")),
        metallic: 0.9,
        perceptual_roughness: 0.25,
        ..default()
    });
    let cyan_emissive_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 0.85, 1.0),
        emissive: LinearRgba::new(0.5, 6.0, 10.0, 1.0),
        unlit: true,
        ..default()
    });

    let panel_thickness = 0.3;

    for i in 0..curve.points.len().saturating_sub(1) {
        let p0 = curve.points[i];
        let p1 = curve.points[i + 1];
        let seg_vec = p1 - p0;
        let seg_len = seg_vec.length();
        if seg_len < 0.05 {
            continue;
        }

        let dir = seg_vec / seg_len;
        let yaw = dir.z.atan2(dir.x);
        let rot = Quat::from_rotation_y(-yaw) * Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);

        let midpoint = (p0 + p1) * 0.5;
        let gy = get_ground_y(midpoint);
        let wall_pos = Vec3::new(midpoint.x, gy + wall_height * 0.5, midpoint.z);

        // Solid cyber metal alloy panel
        let panel_mesh = meshes.add(Cuboid::new(panel_thickness, wall_height, seg_len));
        commands.spawn((
            ProceduralBrick,
            Hittable,
            Health::new(50.0),
            Mesh3d(panel_mesh),
            MeshMaterial3d(metal_mat.clone()),
            Transform::from_translation(wall_pos).with_rotation(rot),
            avian3d::prelude::RigidBody::Static,
            avian3d::prelude::Collider::cuboid(panel_thickness, wall_height, seg_len),
            crate::play_mode::PlayModeEntity,
        ));

        // Horizontal Cyan Emissive Light Conduit Lines along panel
        let conduit_mesh = meshes.add(Cuboid::new(panel_thickness + 0.04, 0.06, seg_len));
        for cy_offset in [0.3 * wall_height, 0.75 * wall_height] {
            let conduit_pos = Vec3::new(midpoint.x, gy + cy_offset, midpoint.z);
            commands.spawn((
                Mesh3d(conduit_mesh.clone()),
                MeshMaterial3d(cyan_emissive_mat.clone()),
                Transform::from_translation(conduit_pos).with_rotation(rot),
                crate::play_mode::PlayModeEntity,
            ));
        }
    }
}

/// Renders a real-time holographic brick/curve blueprint projection in the game world
fn draw_wall_preview(
    mut gizmos: Gizmos,
    builder: Res<ProceduralWallBuilder>,
    map: Res<TempestMap>,
    mut cache: ResMut<ProceduralWallPreviewCache>,
    brush_settings: Option<Res<crate::map_editor::BrushSettings>>,
) {
    let is_procedural_wall_selected = brush_settings
        .map(|s| s.tool == crate::map_editor::SculptTool::PlaceProceduralWall)
        .unwrap_or(false);

    if !(is_procedural_wall_selected || builder.active) {
        return;
    }

    // Draw active aiming indicator (hover dot under crosshair)
    if let Some(next_pt) = builder.hover_point {
        gizmos.sphere(next_pt, 0.12, Color::srgba(1.0, 0.82, 0.0, 0.7));
        if let Some(&last_pt) = builder.points.last() {
            gizmos.line(last_pt, next_pt, Color::srgba(1.0, 0.82, 0.0, 0.45));
        }
    }

    if builder.points.is_empty() {
        return;
    }

    // Connect placed control points with vivid gold indicators
    for (i, &pt) in builder.points.iter().enumerate() {
        gizmos.sphere(pt, 0.15, Color::srgb(1.0, 0.82, 0.0));

        if i > 0 {
            gizmos.line(builder.points[i - 1], pt, Color::srgb(1.0, 0.82, 0.0));
        }
    }

    // Invalidate/rebuild cache if builder points, height, or style changed
    let cache_valid = cache.points == builder.points
        && (cache.height - builder.height).abs() < 0.001
        && cache.style == builder.style;
    if !cache_valid {
        cache.points = builder.points.clone();
        cache.height = builder.height;
        cache.style = builder.style;
        cache.cached_bricks.clear();
        cache.cached_voussoirs.clear();

        if builder.points.len() >= 2 {
            let raw_curve = Curve::from(builder.points.clone()).smooth(2);
            let resampled_curve = raw_curve.resample(0.8);
            cache.cached_bricks = WallConstructor::from_curve_with_style(
                &resampled_curve,
                builder.height,
                builder.style,
                |pos| get_bilinear_height(pos.x, pos.z, &map),
            );

            // Draw holographic arch preview arcs over any detected gap between the
            // preview wall endpoints and nearby existing bricks / the wall itself (ClassicBrick only).
            if builder.style == WallStyle::ClassicBrick
                && let (Some(&first_pt), Some(&last_pt)) = (
                    resampled_curve.points.first(),
                    resampled_curve.points.last(),
                )
            {
                let span_xz = Vec2::new(last_pt.x - first_pt.x, last_pt.z - first_pt.z);
                let span = span_xz.length();
                if (MIN_ARCH_SPAN..=MAX_ARCH_SPAN).contains(&span) {
                    let left_y = get_bilinear_height(first_pt.x, first_pt.z, &map);
                    let left_y_final = left_y + builder.height;
                    let right_y = get_bilinear_height(last_pt.x, last_pt.z, &map);
                    let right_y_final = right_y + builder.height;
                    let opening = ArchOpening {
                        left_foot: first_pt.with_y(left_y_final),
                        right_foot: last_pt.with_y(right_y_final),
                    };
                    cache.cached_voussoirs = generate_arch(&opening);
                }
            }
        }
    }

    let gizmo_color = match builder.style {
        WallStyle::ClassicBrick => Color::srgba(0.9, 0.65, 0.1, 0.45),
        WallStyle::PalisadeFence => Color::srgba(0.2, 0.85, 0.35, 0.50),
        WallStyle::GraniteFortress => Color::srgba(0.5, 0.6, 0.8, 0.50),
        WallStyle::LogTimber => Color::srgba(0.85, 0.5, 0.2, 0.50),
        WallStyle::CyberMetal => Color::srgba(0.0, 0.9, 1.0, 0.55),
    };

    // Render holographic translucent brick layout projection from cache
    for brick in &cache.cached_bricks {
        gizmos.primitive_3d(
            &Cuboid::new(
                (brick.transform.scale.x - 0.04).max(0.1),
                (brick.transform.scale.y - 0.04).max(0.1),
                (brick.transform.scale.z - 0.04).max(0.1),
            ),
            Isometry3d::new(brick.transform.translation, brick.transform.rotation),
            gizmo_color,
        );
    }

    // Render holographic arch preview voussoirs from cache
    for v in &cache.cached_voussoirs {
        gizmos.primitive_3d(
            &Cuboid::new(
                (v.transform.scale.x - 0.02).max(0.05),
                (v.transform.scale.y - 0.02).max(0.05),
                (v.transform.scale.z - 0.02).max(0.05),
            ),
            Isometry3d::new(v.transform.translation, v.transform.rotation),
            Color::srgba(0.4, 0.8, 1.0, 0.35), // cyan-ish arch ghost
        );
    }
}

/// Smoothly animates bricks dropping from above and scaling up with spring bounce.
fn animate_brick_spawns(
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut BrickSpawnAnimation)>,
    mut commands: Commands,
    procedural_wall_assets: Res<ProceduralWallAssets>,
) {
    let dt = time.delta_secs();
    for (entity, mut transform, mut anim) in query.iter_mut() {
        if anim.delay > 0.0 {
            anim.delay -= dt;
            continue;
        }

        anim.elapsed += dt;
        let progress = (anim.elapsed / anim.duration).clamp(0.0_f32, 1.0_f32);

        // Beautiful elastic spring overshoot/bounce landing formula!
        let bounce = 1.0 - (1.0 - progress).powi(3) * (1.0 - progress * 2.8);

        // Animate translation dropping down from 2.0 meters above
        let height_offset = (1.0 - progress).powi(2) * 2.0;
        transform.translation = anim.target_translation + Vec3::Y * height_offset;

        // Animate scale using the spring bounce
        transform.scale = anim.target_scale * bounce.max(0.01);

        if progress >= 1.0 {
            transform.translation = anim.target_translation;
            transform.scale = anim.target_scale;
            commands.entity(entity).remove::<BrickSpawnAnimation>();

            // Insert collider only after the scaling/translation animation finishes to ensure static BVH updates
            commands
                .entity(entity)
                .insert(crate::play_mode::WallCollider {
                    half_extents: anim.target_scale * 0.5,
                });

            // Satisfying dust/landing particles!
            let mut rng = rand::rng();
            for _ in 0..2 {
                commands.spawn((
                    Mesh3d(procedural_wall_assets.dust_mesh.clone()),
                    MeshMaterial3d(procedural_wall_assets.dust_material.clone()),
                    Transform::from_translation(anim.target_translation),
                    Particle {
                        velocity: Vec3::new(
                            rng.random_range(-1.0..1.0),
                            rng.random_range(0.2..1.2),
                            rng.random_range(-1.0..1.0),
                        ),
                        lifetime: Timer::from_seconds(0.5, TimerMode::Once),
                    },
                ));
            }
        }
    }
}

/// Dynamically carves castle doors/gateways into existing procedural brick walls.
#[allow(clippy::too_many_arguments)]
fn carve_gateways(
    mut commands: Commands,
    _mouse_input: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    camera_query: Query<(
        &Camera,
        &GlobalTransform,
        Option<&crate::play_mode::PlayModeCamera>,
        Option<&crate::map_editor::EditorCamera>,
    )>,
    brick_query: Query<
        (Entity, &GlobalTransform, &Health, &Transform),
        With<ProceduralMasonryBrick>,
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    procedural_wall_assets: Res<ProceduralWallAssets>,
    window: Query<&Window>,
) {
    // Press 'G' key while looking at a brick to carve a gateway
    if !keys.just_pressed(KeyCode::KeyG) {
        return;
    }

    let Ok(win) = window.single() else {
        return;
    };
    let Some(cursor) = win.cursor_position() else {
        return;
    };

    let mut ray_opt = None;
    for (camera, camera_transform, play_cam, edit_cam) in camera_query.iter() {
        if camera.is_active
            && (play_cam.is_some() || edit_cam.is_some())
            && let Ok(ray) = camera.viewport_to_world(camera_transform, cursor)
        {
            ray_opt = Some(ray);
            break;
        }
    }
    let Some(ray) = ray_opt else {
        return;
    };

    // Raycast: find targeted brick within 6.0 meters
    let mut targeted_brick = None;
    let mut closest_t = 6.0;

    for (entity, global_transform, _, transform) in brick_query.iter() {
        let pos = global_transform.translation();
        let to_brick = pos - ray.origin;
        let forward_vec = Vec3::from(ray.direction);
        let t = to_brick.dot(forward_vec);

        if t > 0.0 && t < closest_t {
            let closest_point = ray.origin + forward_vec * t;
            let dist = closest_point.distance(pos);
            let bound_radius = transform.scale.max_element() * 0.72;

            if dist < bound_radius {
                closest_t = t;
                targeted_brick = Some((entity, pos, transform.rotation));
            }
        }
    }

    if let Some((_entity, carve_pos, carve_rot)) = targeted_brick {
        let active_texture = "textures/solid_stone.png";
        // Calculate the actual ground and top boundaries of the brick column
        let mut lowest_y = carve_pos.y;
        let mut highest_y = carve_pos.y;
        let mut lowest_scale_y = 0.4;
        let mut highest_scale_y = 0.4;

        for (_, global_transform, _, transform) in brick_query.iter() {
            let pos = global_transform.translation();
            let horizontal_dist =
                Vec2::new(pos.x, pos.z).distance(Vec2::new(carve_pos.x, carve_pos.z));
            if horizontal_dist < 1.4 {
                if pos.y < lowest_y {
                    lowest_y = pos.y;
                    lowest_scale_y = transform.scale.y;
                }
                if pos.y > highest_y {
                    highest_y = pos.y;
                    highest_scale_y = transform.scale.y;
                }
            }
        }

        let ground_y = lowest_y - lowest_scale_y / 2.0;
        let top_y = highest_y + highest_scale_y / 2.0;
        let door_height = (top_y - ground_y).clamp(1.8, 6.0);

        // Despawn all bricks within 1.2m horizontally and up to highest_y vertically to carve a gateway
        let mut rng = rand::rng();
        for (entity, global_transform, _, _) in brick_query.iter() {
            let pos = global_transform.translation();
            let horizontal_dist =
                Vec2::new(pos.x, pos.z).distance(Vec2::new(carve_pos.x, carve_pos.z));

            if horizontal_dist < 1.2 && pos.y <= top_y + 0.1 {
                commands.entity(entity).despawn();

                // Spark dust particles at each cleared brick
                for _ in 0..2 {
                    commands.spawn((
                        Mesh3d(procedural_wall_assets.dust_mesh.clone()),
                        MeshMaterial3d(procedural_wall_assets.dust_material.clone()),
                        Transform::from_translation(pos),
                        Particle {
                            velocity: Vec3::new(
                                rng.random_range(-2.0..2.0),
                                rng.random_range(1.5..4.0),
                                rng.random_range(-2.0..2.0),
                            ),
                            lifetime: Timer::from_seconds(0.6, TimerMode::Once),
                        },
                    ));
                }
            }
        }

        // Spawn a beautiful, double-hinged medieval wooden castle gate centered in the archway!
        let mut gate_pos = carve_pos;
        gate_pos.y = ground_y;

        commands
            .spawn((
                Transform::from_translation(gate_pos).with_rotation(carve_rot),
                Visibility::default(),
                InheritedVisibility::default(),
            ))
            .with_children(|gate| {
                // Left Door Hinge (offset at left end of opening: -1.2m local X)
                gate.spawn((
                    Door {
                        open: false,
                        hinge_side: -1.0,
                        is_open: false,
                        original_rotation: Quat::IDENTITY,
                    },
                    Transform::from_xyz(-1.2, 0.0, 0.0),
                    Visibility::default(),
                    InheritedVisibility::default(),
                ))
                .with_children(|hinge| {
                    hinge.spawn((
                        Mesh3d(procedural_wall_assets.unit_cube.clone()),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(0.38, 0.22, 0.12), // Medieval dark oak wood
                            perceptual_roughness: 0.85,
                            ..default()
                        })),
                        Transform::from_xyz(0.6, door_height / 2.0, 0.0).with_scale(Vec3::new(
                            1.2,
                            door_height,
                            0.12,
                        )), // Center door panel between hinge and opening center
                    ));
                });

                // Right Door Hinge (offset at right end of opening: 1.2m local X)
                gate.spawn((
                    Door {
                        open: false,
                        hinge_side: 1.0,
                        is_open: false,
                        original_rotation: Quat::IDENTITY,
                    },
                    Transform::from_xyz(1.2, 0.0, 0.0),
                    Visibility::default(),
                    InheritedVisibility::default(),
                ))
                .with_children(|hinge| {
                    hinge.spawn((
                        Mesh3d(procedural_wall_assets.unit_cube.clone()),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(0.38, 0.22, 0.12), // Medieval dark oak wood
                            perceptual_roughness: 0.85,
                            ..default()
                        })),
                        Transform::from_xyz(-0.6, door_height / 2.0, 0.0).with_scale(Vec3::new(
                            1.2,
                            door_height,
                            0.12,
                        )), // Center door panel between hinge and opening center
                    ));
                });
            });

        // -----------------------------------------------------------------------
        // Spawn a semicircular arch above the carved opening
        // -----------------------------------------------------------------------
        // Compute impost positions accounting for wall rotation
        let rot_mat = bevy::math::Mat3::from_quat(carve_rot);
        let left_offset = rot_mat * Vec3::new(-1.2, 0.0, 0.0);
        let right_offset = rot_mat * Vec3::new(1.2, 0.0, 0.0);
        let arch_opening = ArchOpening {
            left_foot: carve_pos.with_y(top_y) + left_offset,
            right_foot: carve_pos.with_y(top_y) + right_offset,
        };

        // Unique ID for this arch (use top_y + position hash as rough unique key)
        let arch_id = (carve_pos.x.to_bits() as u64)
            .wrapping_add((carve_pos.z.to_bits() as u64) << 32)
            .wrapping_add(top_y.to_bits() as u64);

        spawn_arch_voussoirs(
            &arch_opening,
            arch_id,
            active_texture,
            None,
            &mut commands,
            &mut materials,
            &asset_server,
            &procedural_wall_assets,
        );

        println!("Carved Gate/Archway in Procedural Wall — arch spawned above!");
    }
}

// ---------------------------------------------------------------------------
// Auto-arch: detect close wall endpoints and bridge with an arch
// ---------------------------------------------------------------------------

/// Scans all `ProceduralBrick` entities each frame, finds wall endpoints that
/// are close to each other but unconnected, and spawns bridging arches.
#[allow(clippy::too_many_arguments, clippy::type_complexity)] // System param to avoid "too many arguments" error in Bevy 0.19
fn detect_and_spawn_arches(
    mut commands: Commands,
    mut arch_registry: ResMut<ArchRegistry>,
    brick_query: Query<
        (
            Entity,
            &GlobalTransform,
            &Transform,
            Option<&BrickSpawnAnimation>,
        ),
        (With<ProceduralMasonryBrick>, Without<ProceduralArchBrick>),
    >,
    root_query: Query<Entity>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    procedural_wall_assets: Res<ProceduralWallAssets>,
    mut local_state: Local<(f32, usize)>, // (timer, last_brick_count)
) {
    // ... rest of the original function body stays exactly the same
    let current_brick_count = brick_query.iter().count();
    let count_changed = current_brick_count != local_state.1;

    local_state.0 += time.delta_secs();

    // Throttle checks to twice a second (0.5s) unless the brick count has changed
    if !count_changed && local_state.0 < 0.5 {
        return;
    }

    local_state.0 = 0.0;
    local_state.1 = current_brick_count;

    #[derive(Clone, Copy)]
    struct BrickInfo {
        pos: Vec3,
        transform: Transform,
    }

    // Gather all brick positions and transforms
    let mut bricks = Vec::with_capacity(current_brick_count);
    for (_entity, _gt, transform, opt_anim) in brick_query.iter() {
        let pos = if let Some(anim) = opt_anim {
            anim.target_translation
        } else {
            transform.translation
        };
        bricks.push(BrickInfo {
            pos,
            transform: *transform,
        });
    }

    if bricks.len() < 2 {
        return;
    }

    // 1. Group bricks into stable vertical columns by horizontal position (tolerance: 0.1m)
    // Optimized grouping: Loop in reverse and use distance_squared to avoid sqrt calls
    let mut columns: Vec<Vec<BrickInfo>> = Vec::new();
    for brick in bricks {
        let brick_xz = Vec2::new(brick.pos.x, brick.pos.z);
        let mut found = false;
        for col in columns.iter_mut().rev() {
            let col_xz = Vec2::new(col[0].pos.x, col[0].pos.z);
            if col_xz.distance_squared(brick_xz) < 0.01 {
                // 0.1m * 0.1m = 0.01
                col.push(brick);
                found = true;
                break;
            }
        }
        if !found {
            columns.push(vec![brick]);
        }
    }

    // 2. Find the highest brick of each column to represent the column tops
    let mut column_tops: Vec<(Vec3, Transform)> = Vec::new();
    for col in &columns {
        if let Some(highest_brick) = col.iter().max_by(|a, b| {
            a.pos
                .y
                .partial_cmp(&b.pos.y)
                .unwrap_or(std::cmp::Ordering::Equal)
        }) {
            column_tops.push((highest_brick.pos, highest_brick.transform));
        }
    }

    // 3. Detect true wall endpoints using stable wall-tangent projection.
    // A column is in the middle of a wall if it has neighbor columns in both directions
    // along the wall's local tangent vector. If it lacks a neighbor on either side, it is an endpoint.
    let mut endpoints: Vec<WallEndpoint> = Vec::new();
    for &(ct_pos, ct_transform) in &column_tops {
        let ct_xz = Vec2::new(ct_pos.x, ct_pos.z);
        let tangent = ct_transform.rotation * Vec3::X;

        let mut has_forward = false;
        let mut has_backward = false;

        for &(other_pos, _) in &column_tops {
            if other_pos == ct_pos {
                continue;
            }
            let other_xz = Vec2::new(other_pos.x, other_pos.z);
            // Optimized distance check using distance_squared (1.2m * 1.2m = 1.44)
            if ct_xz.distance_squared(other_xz) < 1.44 {
                let disp = other_pos - ct_pos;
                let dot = disp.dot(tangent);

                if dot > 0.15 {
                    has_forward = true;
                } else if dot < -0.15 {
                    has_backward = true;
                }
            }
        }

        if !(has_forward && has_backward) {
            endpoints.push(WallEndpoint {
                top_center: ct_pos,
                bottom_center: Vec3::new(ct_pos.x, 0.0, ct_pos.z),
                is_right_end: false,
            });
        }
    }

    // 4. Prune existing arches that are no longer supported by current endpoints.
    // An arch is valid only if there is still an endpoint near its left foot AND an endpoint near its right foot.
    let mut active_arches = Vec::new();
    for (left_xz, right_xz, root) in arch_registry.arches.drain(..) {
        let has_left = endpoints
            .iter()
            .any(|ep| Vec2::new(ep.top_center.x, ep.top_center.z).distance_squared(left_xz) < 0.36); // 0.6m * 0.6m = 0.36
        let has_right = endpoints.iter().any(|ep| {
            Vec2::new(ep.top_center.x, ep.top_center.z).distance_squared(right_xz) < 0.36
        });

        if has_left && has_right {
            active_arches.push((left_xz, right_xz, root));
        } else {
            // Despawn the orphaned arch and all of its voussoir children!
            if root_query.contains(root) {
                commands.entity(root).despawn();
            }
        }
    }
    arch_registry.arches = active_arches;

    // 5. Find candidate arch openings from active endpoints.
    let openings = find_arch_openings(&endpoints);

    let active_texture = "textures/solid_stone.png";

    for opening in openings {
        let left_xz = Vec2::new(opening.left_foot.x, opening.left_foot.z);
        let right_xz = Vec2::new(opening.right_foot.x, opening.right_foot.z);

        // Check if an arch for this opening already exists in the registry.
        let already_registered = arch_registry.arches.iter().any(|(l, r, _)| {
            l.distance_squared(left_xz) < 0.36 && r.distance_squared(right_xz) < 0.36
        });
        if already_registered {
            continue;
        }

        // Unique ID from foot positions.
        let arch_id = (opening.left_foot.x.to_bits() as u64)
            .wrapping_add((opening.left_foot.z.to_bits() as u64) << 16)
            .wrapping_add((opening.right_foot.x.to_bits() as u64) << 32)
            .wrapping_add((opening.right_foot.z.to_bits() as u64) << 48);

        // Spawn a root entity at Identity so children world translations are correct!
        let root = commands
            .spawn((
                Transform::default(),
                Visibility::default(),
                InheritedVisibility::default(),
            ))
            .id();

        spawn_arch_voussoirs(
            &opening,
            arch_id,
            active_texture,
            Some(root),
            &mut commands,
            &mut materials,
            &asset_server,
            &procedural_wall_assets,
        );

        arch_registry.arches.push((left_xz, right_xz, root));
    }
}
