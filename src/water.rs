use super::water_gpu::{WATER_GRID_SIZE, WaterComputePlugin, WaterGpuHandles, WaterSimParams};
use crate::AppState;
use bevy::asset::RenderAssetUsages;
use bevy::camera::{RenderTarget, visibility::RenderLayers};
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::*;
use bevy::render::storage::ShaderBuffer;
use bevy::shader::ShaderRef;

pub struct WaterPlugin;

#[derive(Component)]
pub struct ReflectionCamera;

#[derive(Resource)]
pub struct ReflectionTarget {
    pub image: Handle<Image>,
}

impl Plugin for WaterPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<WaterImpulseEvent>();
        app.add_plugins((
            MaterialPlugin::<WaterMaterial>::default(),
            WaterComputePlugin,
        ))
        .add_systems(OnEnter(AppState::PlayMode), setup_water)
        .add_systems(OnEnter(AppState::MapEditor), setup_water)
        .add_systems(OnExit(AppState::PlayMode), cleanup_water)
        .add_systems(OnExit(AppState::MapEditor), cleanup_water)
        .add_systems(
            Update,
            (
                center_water_on_player,
                update_water_material,
                entity_water_interaction,
                process_water_impulses,
                sync_reflection_camera,
            )
                .run_if(in_state(AppState::PlayMode).or_else(in_state(AppState::MapEditor))),
        );
    }
}

#[derive(Message)]
pub struct WaterImpulseEvent {
    pub position: Vec3,
    pub force: f32,
    pub radius: f32,
}

#[derive(Component)]
#[allow(dead_code)]
pub struct WaterMesh {
    pub handle: Handle<Mesh>,
}

#[derive(Component)]
#[allow(dead_code)]
pub struct WaterSimData {
    pub height: Vec<f32>,
    pub flow_x: Vec<f32>,
    pub flow_y: Vec<f32>,
    pub wall_mask: Vec<bool>,
    pub last_disturbed_pos: Option<(usize, usize)>,
    pub size: f32, // World width/depth of the simulation plane
    pub grid_len: usize,
    pub dirty: bool,
}

#[allow(dead_code)]
impl WaterSimData {
    pub fn new(grid_len: usize, size: f32) -> Self {
        let count = grid_len * grid_len;
        let mut sim = Self {
            height: vec![1.0; count],
            flow_x: vec![0.0; count],
            flow_y: vec![0.0; count],
            wall_mask: vec![false; count],
            last_disturbed_pos: None,
            size,
            grid_len,
            dirty: true,
        };

        // Setup default borders as walls
        for i in 0..grid_len {
            sim.set_wall(i, 0, true);
            sim.set_wall(i, grid_len - 1, true);
            sim.set_wall(0, i, true);
            sim.set_wall(grid_len - 1, i, true);
        }

        sim
    }

    #[inline]
    pub fn idx(&self, x: usize, y: usize) -> usize {
        x * self.grid_len + y
    }

    #[inline]
    pub fn get_height(&self, x: usize, y: usize) -> f32 {
        self.height[self.idx(x, y)]
    }

    #[inline]
    pub fn set_height(&mut self, x: usize, y: usize, val: f32) {
        let idx = self.idx(x, y);
        self.height[idx] = val;
    }

    #[inline]
    pub fn is_wall(&self, x: usize, y: usize) -> bool {
        self.wall_mask[self.idx(x, y)]
    }

    #[inline]
    pub fn set_wall(&mut self, x: usize, y: usize, val: bool) {
        let idx = self.idx(x, y);
        self.wall_mask[idx] = val;
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct WaterMaterial {
    #[uniform(0)]
    pub color: Vec4,
    #[uniform(0)]
    pub time: f32,
    #[uniform(0)]
    pub camera_position: Vec3,
    #[uniform(0)]
    pub resolution: Vec2,
    #[uniform(0)]
    pub water_level: f32,
    #[uniform(0)]
    pub grid_scale: f32,
    #[uniform(0)]
    pub cloudiness: f32,

    #[texture(1)]
    #[sampler(2)]
    pub reflection_texture: Option<Handle<Image>>,

    #[storage(3, read_only, visibility(vertex, fragment))]
    pub height_buffer: Handle<ShaderBuffer>,
}

#[allow(dead_code)]
impl WaterMaterial {
    pub fn new(color: Color) -> Self {
        let c = color.to_linear();
        Self {
            color: Vec4::new(c.red, c.green, c.blue, c.alpha),
            time: 0.0,
            camera_position: Vec3::ZERO,
            resolution: Vec2::new(1920.0, 1080.0),
            water_level: 0.0,
            grid_scale: 512.0 / WATER_GRID_SIZE as f32,
            cloudiness: 0.0,
            reflection_texture: None,
            height_buffer: Handle::default(),
        }
    }
}

impl Material for WaterMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/water_material.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "shaders/water_material.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

pub fn create_water_mesh(size: f32, grid_size: usize) -> Mesh {
    let vertices_per_side = grid_size + 1;
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    let step = size / grid_size as f32;
    let half_size = size * 0.5;

    // Generate vertices
    for y in 0..vertices_per_side {
        for x in 0..vertices_per_side {
            let pos_x = -half_size + (x as f32 * step);
            let pos_z = -half_size + (y as f32 * step);

            positions.push([pos_x, 0.0, pos_z]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([x as f32 / grid_size as f32, y as f32 / grid_size as f32]);
        }
    }

    // Generate indices
    for y in 0..grid_size {
        for x in 0..grid_size {
            let base = (y * vertices_per_side + x) as u32;

            // First triangle
            indices.push(base);
            indices.push(base + vertices_per_side as u32);
            indices.push(base + 1);

            // Second triangle
            indices.push(base + 1);
            indices.push(base + vertices_per_side as u32);
            indices.push(base + vertices_per_side as u32 + 1);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn setup_water(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut water_materials: ResMut<Assets<WaterMaterial>>,
    mut images: ResMut<Assets<Image>>,
    refl_target: Option<Res<ReflectionTarget>>,
) {
    if refl_target.is_some() {
        return;
    }

    let size = 512.0;
    let grid_len = WATER_GRID_SIZE as usize;

    // Create half-resolution reflection render target
    let extent = Extent3d {
        width: 960,
        height: 540,
        depth_or_array_layers: 1,
    };
    let mut reflection_image = Image {
        texture_descriptor: TextureDescriptor {
            label: Some("water_reflection"),
            size: extent,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };
    reflection_image.resize(extent);
    let reflection_handle = images.add(reflection_image);
    commands.insert_resource(ReflectionTarget {
        image: reflection_handle.clone(),
    });

    // Spawn reflection camera that renders only non-water layer (0) into the image
    commands.spawn((
        Name::new("WaterReflectionCamera"),
        ReflectionCamera,
        Camera3d::default(),
        Camera {
            order: -1, // Render BEFORE main camera
            invert_culling: true,
            clear_color: ClearColorConfig::Default,
            ..default()
        },
        RenderTarget::Image(reflection_handle.clone().into()),
        Projection::Perspective(PerspectiveProjection {
            fov: 90.0f32.to_radians(),
            far: 2000.0,
            near: 0.1,
            ..default()
        }),
        Transform::default(),
        RenderLayers::layer(0),
    ));

    // Create high-fidelity water mesh
    let mesh_handle = meshes.add(create_water_mesh(size, grid_len));

    // Setup custom translucent WaterMaterial with reflection texture
    let material_handle = water_materials.add(WaterMaterial {
        color: Vec4::new(0.04, 0.38, 0.78, 0.32),
        time: 0.0,
        camera_position: Vec3::ZERO,
        resolution: Vec2::new(1920.0, 1080.0),
        water_level: 0.0,
        grid_scale: 512.0 / WATER_GRID_SIZE as f32,
        cloudiness: 0.0,
        reflection_texture: Some(reflection_handle),
        height_buffer: Handle::default(),
    });

    // Spawn the simulated water plane entity
    commands.spawn((
        Name::new("SimulatedWaterPlane"),
        Mesh3d(mesh_handle.clone()),
        MeshMaterial3d(material_handle),
        Transform::from_xyz(0.0, 0.0, 0.0),
        WaterSimData::new(grid_len, size),
        WaterMesh {
            handle: mesh_handle,
        },
        RenderLayers::layer(1),
    ));
}

#[allow(clippy::type_complexity)]
fn cleanup_water(
    mut commands: Commands,
    water_query: Query<Entity, Or<(With<WaterSimData>, With<ReflectionCamera>)>>,
) {
    for entity in water_query.iter() {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<ReflectionTarget>();
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn update_water_material(
    time: Res<Time>,
    camera_q: Query<
        &Transform,
        (
            With<crate::play_mode::PlayModeCamera>,
            Without<WaterSimData>,
        ),
    >,
    water_query: Query<
        (&Transform, &MeshMaterial3d<WaterMaterial>),
        (
            With<WaterSimData>,
            Without<crate::play_mode::PlayModeCamera>,
        ),
    >,
    water_settings: Option<Res<crate::map_editor::WaterSettings>>,
    windows: Query<&Window>,
    mut water_materials: ResMut<Assets<WaterMaterial>>,
    gpu_handles: Option<Res<WaterGpuHandles>>,
) {
    let camera_position = camera_q
        .single()
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    let resolution = windows
        .single()
        .map(|w| Vec2::new(w.physical_width() as f32, w.physical_height() as f32))
        .unwrap_or(Vec2::new(1920.0, 1080.0));

    let water_level = water_settings.map(|s| s.height).unwrap_or(0.0);

    if let Some(handles) = &gpu_handles {
        let height_buf_handle = handles.height_current.clone();
        for (water_transform, mat_handle) in water_query.iter() {
            if let Some(mut material) = water_materials.get_mut(&mat_handle.0) {
                material.time = time.elapsed_secs();
                material.camera_position = camera_position;
                material.resolution = resolution;
                material.water_level = water_transform.translation.y.max(water_level);
                material.height_buffer = height_buf_handle.clone();
            }
        }
    } else {
        for (water_transform, mat_handle) in water_query.iter() {
            if let Some(mut material) = water_materials.get_mut(&mat_handle.0) {
                material.time = time.elapsed_secs();
                material.camera_position = camera_position;
                material.resolution = resolution;
                material.water_level = water_transform.translation.y.max(water_level);
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn sync_reflection_camera(
    main_camera_q: Query<
        (&Transform, &Projection),
        (
            With<crate::play_mode::PlayModeCamera>,
            Without<ReflectionCamera>,
            Without<WaterSimData>,
        ),
    >,
    mut refl_camera_q: Query<
        (&mut Transform, &mut Projection),
        (
            With<ReflectionCamera>,
            Without<crate::play_mode::PlayModeCamera>,
            Without<WaterSimData>,
        ),
    >,
    water_query: Query<
        &Transform,
        (
            With<WaterSimData>,
            Without<crate::play_mode::PlayModeCamera>,
            Without<ReflectionCamera>,
        ),
    >,
    water_settings: Option<Res<crate::map_editor::WaterSettings>>,
    windows: Query<&Window>,
    reflection_target: Res<ReflectionTarget>,
    mut images: ResMut<Assets<Image>>,
) {
    let Ok((main_transform, main_proj)) = main_camera_q.single() else {
        return;
    };
    let Ok((mut refl_transform, mut refl_proj)) = refl_camera_q.single_mut() else {
        return;
    };
    let Ok(window) = windows.single() else { return };

    // Dynamically resize reflection image to half resolution (capped at 960x540 for optimal performance)
    let max_width = 960;
    let max_height = 540;
    let width = ((window.physical_width() / 2).min(max_width)).max(1);
    let height = ((window.physical_height() / 2).min(max_height)).max(1);
    if let Some(mut image) = images.get_mut(&reflection_target.image)
        && (image.texture_descriptor.size.width != width
            || image.texture_descriptor.size.height != height)
    {
        let new_extent = Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        image.resize(new_extent);
    }

    let water_y = water_query
        .iter()
        .next()
        .map(|t| t.translation.y)
        .unwrap_or_else(|| water_settings.map(|s| s.height).unwrap_or(0.0));

    let main_global_rotation = main_transform.rotation;
    let main_global_translation = main_transform.translation;

    // Mirrored camera Y position
    let refl_y = 2.0 * water_y - main_global_translation.y;
    refl_transform.translation =
        Vec3::new(main_global_translation.x, refl_y, main_global_translation.z);
    refl_transform.scale = Vec3::ONE;

    // Exact quaternion mirroring across XZ plane
    refl_transform.rotation = Quat::from_xyzw(
        -main_global_rotation.x,
        main_global_rotation.y,
        -main_global_rotation.z,
        main_global_rotation.w,
    );

    *refl_proj = main_proj.clone();
    if let Projection::Perspective(ref mut persp) = *refl_proj {
        let main_y = main_global_translation.y;
        if main_y > water_y {
            let d = water_y - refl_y;
            persp.near = (d - 0.15).max(0.1);
        } else {
            persp.near = 0.1;
        }
    }
}

#[allow(clippy::type_complexity)]
fn center_water_on_player(
    player_query: Query<&Transform, (With<crate::play_mode::PlayModePlayer>, Without<WaterMesh>)>,
    mut water_query: Query<
        (&mut Transform, &mut WaterSimData),
        (With<WaterMesh>, Without<crate::play_mode::PlayModePlayer>),
    >,
    water_settings: Option<Res<crate::map_editor::WaterSettings>>,
    gpu_handles: Option<Res<WaterGpuHandles>>,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };
    let Ok((mut water_transform, mut water_data)) = water_query.single_mut() else {
        return;
    };

    let grid_len = water_data.grid_len;
    let size = water_data.size;
    let cell_size = size / grid_len as f32;

    let target_x = (player_transform.translation.x / cell_size).round() * cell_size;
    let target_z = (player_transform.translation.z / cell_size).round() * cell_size;

    let dx = target_x - water_transform.translation.x;
    let dz = target_z - water_transform.translation.z;

    let shift_x = (dx / cell_size).round() as i32;
    let shift_z = (dz / cell_size).round() as i32;

    let target_y = water_settings.map(|s| s.height).unwrap_or(0.0);
    water_transform.translation.y = target_y;

    if shift_x != 0 || shift_z != 0 {
        let mut new_height = vec![1.0; grid_len * grid_len];
        let mut new_flow_x = vec![0.0; grid_len * grid_len];
        let mut new_flow_y = vec![0.0; grid_len * grid_len];
        let mut new_wall_mask = vec![false; grid_len * grid_len];

        for x in 0..grid_len {
            for y in 0..grid_len {
                let old_x = x as i32 + shift_x;
                let old_y = y as i32 + shift_z;

                let new_idx = x * grid_len + y;

                if old_x >= 0 && old_x < grid_len as i32 && old_y >= 0 && old_y < grid_len as i32 {
                    let old_idx = (old_x as usize) * grid_len + (old_y as usize);
                    new_height[new_idx] = water_data.height[old_idx];
                    new_flow_x[new_idx] = water_data.flow_x[old_idx];
                    new_flow_y[new_idx] = water_data.flow_y[old_idx];
                    new_wall_mask[new_idx] = water_data.wall_mask[old_idx];
                }
            }
        }

        water_data.height = new_height;
        water_data.flow_x = new_flow_x;
        water_data.flow_y = new_flow_y;
        water_data.wall_mask = new_wall_mask;
        water_data.dirty = true;

        water_transform.translation.x = target_x;
        water_transform.translation.z = target_z;

        if let Some(handles) = &gpu_handles {
            if let Some(mut buf) = buffers.get_mut(&handles.height_current) {
                buf.data = Some(bytemuck::cast_slice::<f32, u8>(&water_data.height).to_vec());
            }
            if let Some(mut buf) = buffers.get_mut(&handles.height_next) {
                buf.data = Some(bytemuck::cast_slice::<f32, u8>(&water_data.height).to_vec());
            }
        }
    }
}

#[derive(Component)]
pub struct WaterInteractor {
    pub last_position: Vec3,
    pub mass: f32,
}

impl Default for WaterInteractor {
    fn default() -> Self {
        Self {
            last_position: Vec3::ZERO,
            mass: 1.0,
        }
    }
}

fn entity_water_interaction(
    time: Res<Time>,
    mut interactors_query: Query<(Entity, &Transform, &mut WaterInteractor)>,
    water_query: Query<(&WaterSimData, &Transform), Without<WaterInteractor>>,
    water_settings: Option<Res<crate::map_editor::WaterSettings>>,
    map: Option<Res<crate::map_editor::data::TempestMap>>,
    mut params: ResMut<WaterSimParams>,
    mut impulse_writer: MessageWriter<WaterImpulseEvent>,
) {
    let water_level = water_settings.map(|s| s.height).unwrap_or(0.0);
    params.delta_time = time.delta_secs().min(0.03);
    params.gravity = 28.0;
    params.friction = 0.985;
    params.interactor_count = 0;

    let Ok((water_data, water_transform)) = water_query.single() else {
        return;
    };

    let grid_len = water_data.grid_len;
    let size = water_data.size;
    let cell_size = size / grid_len as f32;
    let half_size = size * 0.5;

    for (_entity, entity_transform, mut interactor) in interactors_query.iter_mut() {
        if params.interactor_count >= 16 {
            break;
        }

        let pos = entity_transform.translation;
        let last_pos = if interactor.last_position == Vec3::ZERO {
            pos
        } else {
            interactor.last_position
        };
        interactor.last_position = pos;

        let velocity = if time.delta_secs() > 0.0 {
            (pos - last_pos) / time.delta_secs()
        } else {
            Vec3::ZERO
        };

        // Check actual terrain height at entity's location
        let ground_y = if let Some(map) = &map {
            let offset_x = -(map.width as f32) / 2.0;
            let offset_z = -(map.height as f32) / 2.0;
            let hx = ((pos.x - offset_x).round() as i32).clamp(0, map.width as i32 - 1) as u32;
            let hz = ((pos.z - offset_z).round() as i32).clamp(0, map.height as i32 - 1) as u32;
            map.get_height(hx, hz)
        } else {
            pos.y
        };

        // Entity is ONLY in water if on surface world (Y > -20.0), ground is submerged AND entity Y is in water
        let in_water = pos.y > -20.0 && ground_y <= water_level + 0.2 && pos.y <= water_level + 0.5;
        let was_in_water = last_pos.y > -20.0 && last_pos.y <= water_level + 0.5;
        let crossed_surface = in_water != was_in_water;

        let rel_x = pos.x - (water_transform.translation.x - half_size);
        let rel_z = pos.z - (water_transform.translation.z - half_size);

        let grid_x = rel_x / cell_size;
        let grid_z = rel_z / cell_size;

        if grid_x >= 1.0
            && grid_x < (grid_len - 1) as f32
            && grid_z >= 1.0
            && grid_z < (grid_len - 1) as f32
        {
            let speed = velocity.length();

            if crossed_surface && speed > 0.5 {
                impulse_writer.write(WaterImpulseEvent {
                    position: pos,
                    force: (speed * 0.35 * interactor.mass).clamp(0.8, 5.0),
                    radius: 2.5,
                });
            }

            if in_water {
                let slot = params.interactor_count as usize;
                params.interactors[slot] = super::water_gpu::WaterInteractorData {
                    grid_x,
                    grid_z,
                    push_force: (speed.max(0.6) * 14.0 * interactor.mass).clamp(3.0, 50.0),
                    push_radius: 1.8,
                    swim_add_height: 0.3,
                    swim_radius: 1.5,
                    _pad1: 0.0,
                    _pad2: 0.0,
                };
                params.interactor_count += 1;
            }
        }
    }
}

fn process_water_impulses(
    mut events: MessageReader<WaterImpulseEvent>,
    water_query: Query<(&WaterSimData, &Transform)>,
    mut params: ResMut<WaterSimParams>,
) {
    let Ok((water_data, water_transform)) = water_query.single() else {
        return;
    };

    let grid_len = water_data.grid_len;
    let size = water_data.size;
    let cell_size = size / grid_len as f32;
    let half_size = size * 0.5;

    params.impulse_count = 0;

    for event in events.read() {
        if params.impulse_count >= 8 {
            break;
        }

        let rel_x = event.position.x - (water_transform.translation.x - half_size);
        let rel_z = event.position.z - (water_transform.translation.z - half_size);

        let grid_x = rel_x / cell_size;
        let grid_z = rel_z / cell_size;

        if grid_x >= 1.0
            && grid_x < (grid_len - 1) as f32
            && grid_z >= 1.0
            && grid_z < (grid_len - 1) as f32
        {
            let slot = params.impulse_count as usize;
            params.impulses[slot] = super::water_gpu::WaterImpulseData {
                grid_x,
                grid_z,
                force: event.force,
                radius: event.radius / cell_size,
            };
            params.impulse_count += 1;
        }
    }
}
