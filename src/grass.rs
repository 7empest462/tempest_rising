use crate::map_editor::{
    SplatmapSettings,
    data::{Biome, TempestMap},
};
use bevy::{
    asset::RenderAssetUsages,
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology},
        render_resource::AsBindGroup,
    },
    shader::ShaderRef,
};

pub struct GrassPlugin;

#[derive(Message)]
pub struct GenerateGrassEvent;

#[derive(Component)]
pub struct ProceduralGrass;

#[derive(Asset, AsBindGroup, Debug, Clone, Reflect)]
pub struct GrassWindExtension {}

impl MaterialExtension for GrassWindExtension {
    fn vertex_shader() -> ShaderRef {
        "shaders/grass_wind.wgsl".into()
    }

    fn prepass_vertex_shader() -> ShaderRef {
        "shaders/grass_wind_prepass.wgsl".into()
    }

    fn deferred_vertex_shader() -> ShaderRef {
        "shaders/grass_wind_prepass.wgsl".into()
    }
}

pub type GrassMaterial = ExtendedMaterial<StandardMaterial, GrassWindExtension>;

impl Plugin for GrassPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<GrassMaterial>::default());
        app.add_message::<GenerateGrassEvent>();
        app.add_systems(Update, spawn_grass_system);
    }
}

fn append_grass_blade(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
    pos: Vec3,
    rot: f32,
    scale: f32,
) {
    let start_idx = positions.len() as u32;

    let width = 0.5 * scale;
    let height = 0.8 * scale;
    let half_width = width / 2.0;

    let base_positions = [
        Vec3::new(-half_width, height, 0.0),
        Vec3::new(half_width, height, 0.0),
        Vec3::new(-half_width, 0.0, 0.0),
        Vec3::new(half_width, 0.0, 0.0),
        Vec3::new(0.0, height, -half_width),
        Vec3::new(0.0, height, half_width),
        Vec3::new(0.0, 0.0, -half_width),
        Vec3::new(0.0, 0.0, half_width),
    ];

    let base_normals = [
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
    ];

    let base_uvs = [
        [0.0, 0.0],
        [1.0, 0.0],
        [0.0, 1.0],
        [1.0, 1.0],
        [0.0, 0.0],
        [1.0, 0.0],
        [0.0, 1.0],
        [1.0, 1.0],
    ];

    let quat = Quat::from_rotation_y(rot);

    for i in 0..8 {
        let transformed_pos = quat * base_positions[i] + pos;
        let transformed_normal = quat * base_normals[i];

        positions.push(transformed_pos.to_array());
        normals.push(transformed_normal.to_array());
        uvs.push(base_uvs[i]);
    }

    let base_indices = [
        0, 2, 1, 1, 2, 3, 1, 3, 0, 0, 3, 2, 4, 6, 5, 5, 6, 7, 5, 7, 4, 4, 7, 6,
    ];

    for &idx in &base_indices {
        indices.push(start_idx + idx);
    }
}

fn build_mesh(
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

pub struct GrassChunkData {
    pub patch_mesh: Option<Mesh>,
    pub single_mesh: Option<Mesh>,
}

pub fn generate_grass_chunks(map: &TempestMap) -> Vec<GrassChunkData> {
    let w = map.width;
    let h = map.height;
    let offset_x = -(w as f32) / 2.0;
    let offset_z = -(h as f32) / 2.0;

    let splat = SplatmapSettings::default();

    let chunk_size = 32;
    let chunks_x = w.div_ceil(chunk_size);
    let chunks_z = h.div_ceil(chunk_size);

    let mut result = Vec::with_capacity((chunks_x * chunks_z) as usize);

    for cz in 0..chunks_z {
        for cx in 0..chunks_x {
            let mut patch_positions = Vec::new();
            let mut patch_normals = Vec::new();
            let mut patch_uvs = Vec::new();
            let mut patch_indices = Vec::new();

            let mut single_positions = Vec::new();
            let mut single_normals = Vec::new();
            let mut single_uvs = Vec::new();
            let mut single_indices = Vec::new();

            let start_x = (cx * chunk_size).max(2);
            let start_z = (cz * chunk_size).max(2);
            let end_x = ((cx + 1) * chunk_size).min(w - 2);
            let end_z = ((cz + 1) * chunk_size).min(h - 2);

            for z in start_z..end_z {
                for x in start_x..end_x {
                    let y = map.get_height(x, z);

                    if y <= splat.sand_height || y >= splat.snow_height {
                        continue;
                    }

                    // Skip grass on roads (paved/dirt/bridges)
                    if map.get_road(x, z) > 0 {
                        continue;
                    }

                    // Skip grass inside the mansion footprint (X: [-22, 22], Z: [-12, 12])
                    let world_x = x as f32 + offset_x;
                    let world_z = z as f32 + offset_z;
                    if world_x.abs() <= 22.0 && world_z.abs() <= 12.0 {
                        continue;
                    }

                    let y_l = map.get_height(x - 1, z);
                    let y_r = map.get_height(x + 1, z);
                    let y_u = map.get_height(x, z - 1);
                    let y_d = map.get_height(x, z + 1);
                    let normal = Vec3::new(y_l - y_r, 2.0, y_u - y_d).normalize();

                    if normal.y >= splat.cliff_steepness {
                        let cell_biome = map.get_biome(x, z);
                        let is_temperate = cell_biome == Biome::Temperate;
                        let is_tundra = cell_biome == Biome::Tundra;

                        if is_temperate || is_tundra {
                            // Only spawn grass on 4% of eligible tiles to avoid geometry explosion
                            let spawn_hash =
                                (((x * 53 + z * 97) as f32).sin() * 43758.54).fract().abs();
                            if spawn_hash < 0.04 {
                                let mut density = 3.0
                                    + ((((x * 77 + z * 88) as f32).sin() * 1234.0).fract().abs()
                                        * 3.0);
                                if is_tundra {
                                    density *= 0.20; // Tundra is sparse
                                }
                                let cluster_size = density as usize;
                                for i in 0..cluster_size {
                                    let r1 = (((x * 123 + z * 456 + (i as u32)) as f32).sin()
                                        * 43_758.547)
                                        .fract();
                                    let r2 = (((x * 321 + z * 654 + (i as u32)) as f32).cos()
                                        * 54_321.125)
                                        .fract();

                                    let local_x = r1;
                                    let local_z = r2;

                                    let px = (x as f32 + local_x) + offset_x;
                                    let pz = (z as f32 + local_z) + offset_z;

                                    // Skip grass on roads (paved/dirt/bridges)
                                    let gx = (px - offset_x).round() as u32;
                                    let gz = (pz - offset_z).round() as u32;
                                    if gx < w && gz < h && map.get_road(gx, gz) > 0 {
                                        continue;
                                    }

                                    // Skip grass inside the mansion footprint
                                    if px.abs() <= 22.0 && pz.abs() <= 12.0 {
                                        continue;
                                    }

                                    let py = crate::play_mode::get_bilinear_height(px, pz, map);

                                    let rotation = r2.abs() * std::f32::consts::TAU;
                                    let rand_val = (r1.abs() * 100.0).fract();

                                    if is_temperate && rand_val < 0.6 {
                                        // 60% chance of a lush grass patch (only in Temperate)
                                        let scale = 0.2 + r1.abs() * 0.3;
                                        append_grass_blade(
                                            &mut patch_positions,
                                            &mut patch_normals,
                                            &mut patch_uvs,
                                            &mut patch_indices,
                                            Vec3::new(px, py, pz),
                                            rotation,
                                            scale,
                                        );
                                    } else {
                                        // Tundra gets 100% single blades, Temperate gets 40%
                                        let scale = 0.3 + r1.abs() * 0.4;
                                        append_grass_blade(
                                            &mut single_positions,
                                            &mut single_normals,
                                            &mut single_uvs,
                                            &mut single_indices,
                                            Vec3::new(px, py, pz),
                                            rotation,
                                            scale,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let patch_mesh = if !patch_positions.is_empty() {
                Some(build_mesh(
                    patch_positions,
                    patch_normals,
                    patch_uvs,
                    patch_indices,
                ))
            } else {
                None
            };
            let single_mesh = if !single_positions.is_empty() {
                Some(build_mesh(
                    single_positions,
                    single_normals,
                    single_uvs,
                    single_indices,
                ))
            } else {
                None
            };

            if patch_mesh.is_some() || single_mesh.is_some() {
                result.push(GrassChunkData {
                    patch_mesh,
                    single_mesh,
                });
            }
        }
    }

    result
}

fn spawn_grass_system(
    mut commands: Commands,
    mut ev_generate: MessageReader<GenerateGrassEvent>,
    grass_query: Query<Entity, With<ProceduralGrass>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GrassMaterial>>,
    asset_server: Res<AssetServer>,
    map: Option<Res<TempestMap>>,
) {
    if ev_generate.read().next().is_none() {
        return;
    }

    let Some(map) = map else { return };

    for entity in grass_query.iter() {
        commands.entity(entity).despawn();
    }

    let grass_material = materials.add(ExtendedMaterial {
        base: StandardMaterial {
            base_color_texture: Some(asset_server.load("textures/grass.png")),
            alpha_mode: AlphaMode::Mask(0.5),
            cull_mode: None,
            perceptual_roughness: 0.9,
            reflectance: 0.1,
            ..default()
        },
        extension: GrassWindExtension {},
    });

    let grass_single_material = materials.add(ExtendedMaterial {
        base: StandardMaterial {
            base_color_texture: Some(asset_server.load("textures/grass_single.png")),
            alpha_mode: AlphaMode::Mask(0.5),
            cull_mode: None,
            perceptual_roughness: 0.9,
            reflectance: 0.1,
            ..default()
        },
        extension: GrassWindExtension {},
    });

    let chunks = generate_grass_chunks(&map);
    for chunk in chunks {
        if let Some(mesh) = chunk.patch_mesh {
            commands.spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(grass_material.clone()),
                Transform::default(),
                ProceduralGrass,
            ));
        }
        if let Some(mesh) = chunk.single_mesh {
            commands.spawn((
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(grass_single_material.clone()),
                Transform::default(),
                ProceduralGrass,
            ));
        }
    }
}
