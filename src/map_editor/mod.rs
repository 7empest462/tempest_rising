use crate::AppState;
use crate::play_mode::get_bilinear_height;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
pub type HashMap<K, V> = hashbrown::HashMap<K, V>;
use std::fs::File;
use std::io::{Read, Write};

pub mod data;
pub mod geometry;
pub mod noise;
pub mod tree_generator;
use data::{Biome, EditableMesh, PlacedPrefab, TempestMap};
use noise::PerlinNoise;

#[derive(Component)]
pub struct MapEditorEntity;

#[derive(Component)]
pub struct TerrainMesh;

#[derive(Component)]
pub struct EditorBridge;

#[derive(Component)]
pub struct EditorCamera {
    pub orbit: Vec3,
    pub radius: f32,
    pub pitch: f32,
    pub yaw: f32,
}

impl Default for EditorCamera {
    fn default() -> Self {
        Self {
            orbit: Vec3::ZERO,
            radius: 50.0,
            pitch: 0.5,
            yaw: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SculptTool {
    Raise,
    Lower,
    Smooth,
    Disturb,
    Rocky,
    PlaceTreeOak,
    PlaceTreePine,
    PlaceTreeBirch,
    PlaceShrub,
    PlaceCactus,
    PlaceRock,
    PlaceCaveEntrance,
    PlaceSpawnPoint,
    PlaceHouse,
    PlaceOreCopper,
    PlaceOreIron,
    PlaceOreGold,
    PlaceOreSilver,
    PlaceOrePlatinum,
    PlaceOreSteel,
    PlaceOreGranite,
    PlaceProceduralWall,
    SelectObject,
    PlaceModularWall,
    PlaceModularCorner,
    PlaceModularFloor,
    PlaceModularRoof,
    PlaceModularRoofGable,
    PlaceModularDoorFrame,
    PlaceModularWindowFrame,
    PlaceWallTJunction,
    PlaceWallCross,
    PlaceCeilingTile,
    PlaceFluorescentLight,
    PlaceHallwaySegment,
    PlaceRoomPillar,
    PlaceChest,
    PlaceWorkbench,
    PlaceFurnace,
    PlaceBed,
    PlaceTorch,
    PlaceChair,
    PlaceDesk,
    PlaceHealthPack,
    PlaceCrate,
    PlaceCustomAsset,
    PlaceCustomMesh,
    DeletePrefab,
}

#[derive(Resource)]
pub struct BrushSettings {
    pub size: f32,
    pub strength: f32,
    pub tool: SculptTool,
}

impl Default for BrushSettings {
    fn default() -> Self {
        Self {
            size: 5.0,
            strength: 5.0,
            tool: SculptTool::Raise,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CustomMeshPrimitive {
    #[default]
    Cube,
    Sphere,
}

#[allow(dead_code)]
#[derive(Resource)]
pub struct SelectionState {
    pub selected_idx: Option<usize>,
    pub snap_to_grid: bool,
    pub snap_grid_size: f32,
    pub snap_to_objects: bool,
    pub active_drag_axis: Option<usize>, // None, Some(0)=X, Some(1)=Y, Some(2)=Z
    pub drag_scale: bool,                // true = scale/stretch, false = translate
    pub drag_start_offset: Vec3,
    pub drag_start_value: Vec3,
    pub drag_start_mouse_proj: f32,
    pub selected_texture: String,
    pub preview_entity: Option<Entity>,
    pub preview_tool: Option<SculptTool>,
    pub placement_rotation_angle: f32,
    pub placement_flipped: bool,
    pub custom_mesh_primitive: CustomMeshPrimitive,
}

impl Default for SelectionState {
    fn default() -> Self {
        Self {
            selected_idx: None,
            snap_to_grid: true,
            snap_grid_size: 1.0,
            snap_to_objects: true,
            active_drag_axis: None,
            drag_scale: false,
            drag_start_offset: Vec3::ZERO,
            drag_start_value: Vec3::ZERO,
            drag_start_mouse_proj: 0.0,
            selected_texture: "Default".to_string(),
            preview_entity: None,
            preview_tool: None,
            placement_rotation_angle: 0.0,
            placement_flipped: false,
            custom_mesh_primitive: CustomMeshPrimitive::Cube,
        }
    }
}

#[derive(Resource, Default)]
pub struct CustomAssetLibrary {
    pub assets: Vec<CustomAssetEntry>,
    pub import_path: String,
    pub selected_asset_idx: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct CustomAssetEntry {
    pub name: String,
    pub file_path: String,
    pub asset_type: CustomAssetType,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CustomAssetType {
    Gltf,
    Obj,
    Image,
}

#[derive(Resource)]
pub struct GeometryEditorSettings {
    pub selected_face_idx: usize,
    pub extrude_dist: f32,
    pub inset_factor: f32,
    pub bevel_amount: f32,
    pub knife_origin: Vec3,
    pub knife_normal: Vec3,
    pub bridge_face_b: usize,
    pub bool_op: String,
    pub bool_target_idx: Option<usize>,
}

impl Default for GeometryEditorSettings {
    fn default() -> Self {
        Self {
            selected_face_idx: 0,
            extrude_dist: 1.0,
            inset_factor: 0.25,
            bevel_amount: 0.1,
            knife_origin: Vec3::ZERO,
            knife_normal: Vec3::Y,
            bridge_face_b: 0,
            bool_op: "Union".to_string(),
            bool_target_idx: None,
        }
    }
}

#[derive(Resource)]
pub struct MapIOState {
    pub filename: String,
    pub status_message: String,
}

impl Default for MapIOState {
    fn default() -> Self {
        Self {
            filename: "map.json".to_string(),
            status_message: "".to_string(),
        }
    }
}

#[derive(Resource, Clone, Copy)]
pub struct SplatmapSettings {
    pub sand_height: f32,
    pub snow_height: f32,
    pub cliff_steepness: f32,
    pub biome: Biome,
}

impl Default for SplatmapSettings {
    fn default() -> Self {
        Self {
            sand_height: 1.5,
            snow_height: 24.0,
            cliff_steepness: 0.75,
            biome: Biome::Temperate,
        }
    }
}

#[derive(Resource)]
pub struct WaterSettings {
    pub height: f32,
}

impl Default for WaterSettings {
    fn default() -> Self {
        Self { height: 1.2 }
    }
}

#[derive(Resource)]
pub struct NoiseSettings {
    pub seed: u32,
    pub frequency: f32,
    pub octaves: u32,
    pub amplitude: f32,
    pub ridge_exponent: f32,
    pub height_offset: f32,
}

impl Default for NoiseSettings {
    fn default() -> Self {
        Self {
            seed: 1337,
            frequency: 0.03,
            octaves: 4,
            amplitude: 8.0,
            ridge_exponent: 1.0,
            height_offset: 0.0,
        }
    }
}

#[derive(Resource)]
pub struct MapResizeSettings {
    pub width: u32,
    pub height: u32,
}

impl Default for MapResizeSettings {
    fn default() -> Self {
        Self {
            width: 1200,
            height: 1200,
        }
    }
}

#[derive(Resource)]
pub struct BiomeSelection {
    pub temperate: bool,
    pub arid: bool,
    pub tundra: bool,
    pub arctic: bool,
    pub make_island: bool,
    pub generate_caves: bool,
}

impl Default for BiomeSelection {
    fn default() -> Self {
        Self {
            temperate: true,
            arid: true,
            tundra: true,
            arctic: true,
            make_island: true,
            generate_caves: true,
        }
    }
}

#[derive(Message)]
pub struct WaterImpulseEvent {
    pub position: Vec3,
    pub force: f32,
    pub radius: f32,
}

#[derive(Component)]
pub struct WaterMesh;

#[derive(Component)]
pub struct PlacedPrefabMarker {
    pub prefab_type: String,
    pub position: Vec3,
    pub index: usize,
}

#[derive(Component)]
pub struct WaterSimData {
    pub width: u32,
    pub height: u32,
    pub heights: Vec<f32>,
    pub flow_x: Vec<f32>,
    pub flow_y: Vec<f32>,
    pub wall_mask: Vec<bool>,
}

impl WaterSimData {
    pub fn new(w: u32, h: u32) -> Self {
        let size = (w * h) as usize;
        Self {
            width: w,
            height: h,
            heights: vec![1.0; size],
            flow_x: vec![0.0; size],
            flow_y: vec![0.0; size],
            wall_mask: vec![false; size],
        }
    }

    #[inline]
    pub fn idx(&self, x: u32, z: u32) -> usize {
        (z * self.width + x) as usize
    }

    #[inline]
    pub fn get_height(&self, x: u32, z: u32) -> f32 {
        let i = self.idx(x, z);
        if i < self.heights.len() {
            self.heights[i]
        } else {
            1.0
        }
    }

    #[inline]
    pub fn set_height(&mut self, x: u32, z: u32, val: f32) {
        let i = self.idx(x, z);
        if i < self.heights.len() {
            self.heights[i] = val;
        }
    }

    #[inline]
    pub fn get_flow_x(&self, x: u32, z: u32) -> f32 {
        let i = self.idx(x, z);
        if i < self.flow_x.len() {
            self.flow_x[i]
        } else {
            0.0
        }
    }

    #[inline]
    pub fn set_flow_x(&mut self, x: u32, z: u32, val: f32) {
        let i = self.idx(x, z);
        if i < self.flow_x.len() {
            self.flow_x[i] = val;
        }
    }

    #[inline]
    pub fn get_flow_y(&self, x: u32, z: u32) -> f32 {
        let i = self.idx(x, z);
        if i < self.flow_y.len() {
            self.flow_y[i]
        } else {
            0.0
        }
    }

    #[inline]
    pub fn set_flow_y(&mut self, x: u32, z: u32, val: f32) {
        let i = self.idx(x, z);
        if i < self.flow_y.len() {
            self.flow_y[i] = val;
        }
    }

    #[inline]
    pub fn is_wall(&self, x: u32, z: u32) -> bool {
        let i = self.idx(x, z);
        if i < self.wall_mask.len() {
            self.wall_mask[i]
        } else {
            false
        }
    }

    #[inline]
    pub fn set_wall(&mut self, x: u32, z: u32, val: bool) {
        let i = self.idx(x, z);
        if i < self.wall_mask.len() {
            self.wall_mask[i] = val;
        }
    }
}

pub struct MapEditorPlugin;

impl Plugin for MapEditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TempestMap>()
            .init_resource::<BrushSettings>()
            .init_resource::<MapIOState>()
            .init_resource::<SplatmapSettings>()
            .init_resource::<WaterSettings>()
            .init_resource::<NoiseSettings>()
            .init_resource::<MapResizeSettings>()
            .init_resource::<BiomeSelection>()
            .init_resource::<SelectionState>()
            .init_resource::<CustomAssetLibrary>()
            .init_resource::<GeometryEditorSettings>()
            .add_message::<WaterImpulseEvent>()
            .add_systems(
                OnEnter(AppState::MapEditor),
                (setup_map_editor, disable_ui_camera_clear),
            )
            .add_systems(
                OnExit(AppState::MapEditor),
                (cleanup_map_editor, enable_ui_camera_clear),
            )
            .add_systems(
                Update,
                (
                    camera_controller.run_if(in_state(AppState::MapEditor)),
                    terrain_sculpting_system.run_if(in_state(AppState::MapEditor)),
                    water_simulation_system.run_if(in_state(AppState::MapEditor)),
                    configure_terrain_sampler_system.run_if(in_state(AppState::MapEditor)),
                ),
            )
            .add_systems(
                EguiPrimaryContextPass,
                map_editor_ui.run_if(in_state(AppState::MapEditor)),
            )
            .add_systems(
                Update,
                sync_prefab_transforms.run_if(in_state(AppState::MapEditor)),
            );
    }
}

/// Syncs visual entity transforms with map.prefabs data when user edits properties via UI.
fn sync_prefab_transforms(
    map: Res<TempestMap>,
    selection_state: Res<SelectionState>,
    mut query: Query<(&PlacedPrefabMarker, &mut Transform)>,
) {
    let Some(sel_idx) = selection_state.selected_idx else {
        return;
    };
    if sel_idx >= map.prefabs.len() {
        return;
    }
    let prefab = &map.prefabs[sel_idx];
    let target_pos = Vec3::from_array(prefab.position);
    let target_rot = Quat::from_array(prefab.rotation);
    let target_scale = Vec3::from_array(prefab.scale);

    for (marker, mut transform) in query.iter_mut() {
        if marker.index == sel_idx {
            transform.translation = target_pos;
            transform.rotation = target_rot;
            transform.scale = target_scale;
        }
    }
}

fn compute_vertex_color(
    vx: f32,
    vy: f32,
    vz: f32,
    normal_y: f32,
    settings: &SplatmapSettings,
    biome: Biome,
    road: u8,
) -> [f32; 4] {
    let (sand_color, grass_color, rock_color, snow_color) = match biome {
        Biome::Temperate => (
            [0.92, 0.85, 0.62, 1.0], // Sand
            [0.25, 0.60, 0.25, 1.0], // Lush Grass
            [0.45, 0.45, 0.48, 1.0], // Grey Cliffs
            [0.95, 0.95, 0.95, 1.0], // Snowy Peaks
        ),
        Biome::Arid => (
            [0.94, 0.78, 0.45, 1.0], // Golden Sand
            [0.72, 0.58, 0.32, 1.0], // Dry Grass / Clay
            [0.78, 0.35, 0.18, 1.0], // Orange/Red Canyon Rock
            [0.70, 0.65, 0.60, 1.0], // Rocky Heights
        ),
        Biome::Tundra => (
            [0.55, 0.52, 0.45, 1.0], // Muddy Shore
            [0.45, 0.55, 0.35, 1.0], // Olive Mossy Grass
            [0.35, 0.35, 0.38, 1.0], // Charcoal Rock
            [0.85, 0.88, 0.90, 1.0], // Snowy/Ice patches
        ),
        Biome::Arctic => (
            [0.85, 0.92, 0.95, 1.0], // Pale Glacial Ice
            [0.90, 0.95, 0.98, 1.0], // Pale Snow
            [0.15, 0.35, 0.55, 1.0], // Deep Blue Ice Cliffs
            [0.98, 0.98, 0.98, 1.0], // Pure White Snow Peaks
        ),
    };

    let lerp = |a: [f32; 4], b: [f32; 4], t: f32| -> [f32; 4] {
        let t = t.clamp(0.0, 1.0);
        [
            a[0] * (1.0 - t) + b[0] * t,
            a[1] * (1.0 - t) + b[1] * t,
            a[2] * (1.0 - t) + b[2] * t,
            a[3] * (1.0 - t) + b[3] * t,
        ]
    };

    let base_color = if vy < settings.sand_height {
        let t = (vy / settings.sand_height.max(0.01)).clamp(0.0, 1.0);
        lerp(sand_color, grass_color, t)
    } else {
        grass_color
    };

    let slope_factor = if normal_y < settings.cliff_steepness {
        let delta = settings.cliff_steepness - 0.15;
        ((settings.cliff_steepness - normal_y) / (settings.cliff_steepness - delta).max(0.01))
            .clamp(0.0, 1.0)
    } else {
        0.0
    };

    // Calculate layered geological rock striations and granite micro-grain
    let mut actual_rock = rock_color;
    let striation = (vy * 4.5).sin().abs() * 0.14;
    let grain = ((vx * 15.0).cos() * (vz * 15.0).sin()).abs() * 0.08;
    actual_rock[0] = (actual_rock[0] * (0.86 + striation - grain)).clamp(0.0, 1.0);
    actual_rock[1] = (actual_rock[1] * (0.86 + striation - grain)).clamp(0.0, 1.0);
    actual_rock[2] = (actual_rock[2] * (0.86 + striation - grain)).clamp(0.0, 1.0);

    let color_with_cliffs = lerp(base_color, actual_rock, slope_factor);

    let base_color_with_snow = if vy > settings.snow_height {
        let t = ((vy - settings.snow_height) / 3.0).clamp(0.0, 1.0);
        lerp(color_with_cliffs, snow_color, t)
    } else {
        color_with_cliffs
    };

    if road == 1 {
        // Asphalt Paved Road: dark charcoal grey with subtle texture grain
        let grain = ((vx * 8.0).cos() * (vz * 8.0).sin()).abs() * 0.03;
        let mut r_color = [0.18 - grain, 0.18 - grain, 0.20 - grain, 1.0];

        // Dashed center lines
        let is_center_line = (vx - vx.round()).abs() < 0.08 || (vz - vz.round()).abs() < 0.08;
        let is_dash = (vx * 1.5).floor() as i32 % 2 == 0 && (vz * 1.5).floor() as i32 % 2 == 0;
        if is_center_line && is_dash {
            r_color = [0.85, 0.72, 0.15, 1.0]; // yellow dashes!
        }
        r_color
    } else if road == 2 {
        // Dirt Road: warm sandy brown
        let grain = ((vx * 6.0).cos() * (vz * 6.0).sin()).abs() * 0.05;
        [0.38 - grain, 0.32 - grain, 0.24 - grain, 1.0]
    } else {
        base_color_with_snow
    }
}

pub fn rebuild_terrain_mesh(
    entity: Entity,
    commands: &mut Commands,
    map: &TempestMap,
    settings: &SplatmapSettings,
    meshes: &mut Assets<Mesh>,
    mesh_handle: Option<&Mesh3d>,
) {
    if let Some(h) = mesh_handle
        && let Some(mut mesh) = meshes.get_mut(&h.0)
    {
        update_terrain_mesh_in_place(&mut mesh, map, settings);
        return;
    }
    let new_mesh = generate_terrain_mesh(map, settings);
    let new_handle = meshes.add(new_mesh);
    commands.entity(entity).insert(Mesh3d(new_handle));
}

pub fn update_terrain_mesh_in_place(
    mesh: &mut Mesh,
    map: &TempestMap,
    settings: &SplatmapSettings,
) {
    let w = map.width;
    let h = map.height;
    let offset_x = -(w as f32) / 2.0;
    let offset_z = -(h as f32) / 2.0;

    let total = (w * h) as usize;
    let mut positions = Vec::with_capacity(total);
    let mut normals = vec![[0.0, 1.0, 0.0]; total];
    let mut colors = Vec::with_capacity(total);
    let mut uvs = Vec::with_capacity(total);

    for z in 0..h {
        for x in 0..w {
            let y = map.get_height(x, z);
            positions.push([x as f32 + offset_x, y, z as f32 + offset_z]);
            uvs.push([x as f32 * 0.25, z as f32 * 0.25]);
        }
    }

    for z in 0..h {
        for x in 0..w {
            let y = map.get_height(x, z);
            let y_l = if x > 0 { map.get_height(x - 1, z) } else { y };
            let y_r = if x < w - 1 {
                map.get_height(x + 1, z)
            } else {
                y
            };
            let y_u = if z > 0 { map.get_height(x, z - 1) } else { y };
            let y_d = if z < h - 1 {
                map.get_height(x, z + 1)
            } else {
                y
            };

            let normal = Vec3::new(y_l - y_r, 2.0, y_u - y_d).normalize();
            let idx = (z * w + x) as usize;
            normals[idx] = [normal.x, normal.y, normal.z];

            let vx = x as f32 + offset_x;
            let vz = z as f32 + offset_z;
            let color = compute_vertex_color(
                vx,
                y,
                vz,
                normal.y,
                settings,
                map.get_biome(x, z),
                map.get_road(x, z),
            );
            colors.push(color);
        }
    }

    let mut indices = Vec::with_capacity(((w - 1) * (h - 1) * 6) as usize);
    for z in 0..(h - 1) {
        for x in 0..(w - 1) {
            let i0 = z * w + x;
            let i1 = z * w + (x + 1);
            let i2 = (z + 1) * w + x;
            let i3 = (z + 1) * w + (x + 1);

            indices.push(i0);
            indices.push(i2);
            indices.push(i1);

            indices.push(i1);
            indices.push(i2);
            indices.push(i3);
        }
    }

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
}

pub fn generate_terrain_mesh(map: &TempestMap, settings: &SplatmapSettings) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    let w = map.width;
    let h = map.height;

    let offset_x = -(w as f32) / 2.0;
    let offset_z = -(h as f32) / 2.0;

    for z in 0..h {
        for x in 0..w {
            let y = map.get_height(x, z);
            positions.push([x as f32 + offset_x, y, z as f32 + offset_z]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([x as f32 * 0.25, z as f32 * 0.25]);
        }
    }

    for z in 0..h {
        for x in 0..w {
            let y = map.get_height(x, z);
            let y_l = if x > 0 { map.get_height(x - 1, z) } else { y };
            let y_r = if x < w - 1 {
                map.get_height(x + 1, z)
            } else {
                y
            };
            let y_u = if z > 0 { map.get_height(x, z - 1) } else { y };
            let y_d = if z < h - 1 {
                map.get_height(x, z + 1)
            } else {
                y
            };

            let normal = Vec3::new(y_l - y_r, 2.0, y_u - y_d).normalize();
            let idx = (z * w + x) as usize;
            normals[idx] = [normal.x, normal.y, normal.z];

            let vx = x as f32 + offset_x;
            let vz = z as f32 + offset_z;
            let color = compute_vertex_color(
                vx,
                y,
                vz,
                normal.y,
                settings,
                map.get_biome(x, z),
                map.get_road(x, z),
            );
            colors.push(color);
        }
    }

    for z in 0..(h - 1) {
        for x in 0..(w - 1) {
            let i0 = z * w + x;
            let i1 = z * w + (x + 1);
            let i2 = (z + 1) * w + x;
            let i3 = (z + 1) * w + (x + 1);

            indices.push(i0);
            indices.push(i2);
            indices.push(i1);

            indices.push(i1);
            indices.push(i2);
            indices.push(i3);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

pub fn generate_water_mesh(w: u32, h: u32) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    let offset_x = -(w as f32) / 2.0;
    let offset_z = -(h as f32) / 2.0;

    for z in 0..h {
        for x in 0..w {
            positions.push([x as f32 + offset_x, 0.0, z as f32 + offset_z]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([x as f32 / w as f32, z as f32 / h as f32]);
        }
    }

    for z in 0..(h - 1) {
        for x in 0..(w - 1) {
            let i0 = z * w + x;
            let i1 = z * w + (x + 1);
            let i2 = (z + 1) * w + x;
            let i3 = (z + 1) * w + (x + 1);

            indices.push(i0);
            indices.push(i2);
            indices.push(i1);

            indices.push(i1);
            indices.push(i2);
            indices.push(i3);
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

pub fn generate_roof_gable_mesh() -> Mesh {
    let positions = vec![
        // Front Face
        [-2.0, 0.0, 0.1],
        [2.0, 0.0, 0.1],
        [2.0, 2.35, 0.1],
        // Back Face
        [-2.0, 0.0, -0.1],
        [2.0, 0.0, -0.1],
        [2.0, 2.35, -0.1],
        // Bottom Face
        [-2.0, 0.0, -0.1],
        [2.0, 0.0, -0.1],
        [-2.0, 0.0, 0.1],
        [2.0, 0.0, 0.1],
        // Left Slanted Face
        [-2.0, 0.0, -0.1],
        [-2.0, 0.0, 0.1],
        [2.0, 2.35, -0.1],
        [2.0, 2.35, 0.1],
        // Right Vertical Face
        [2.0, 0.0, -0.1],
        [2.0, 0.0, 0.1],
        [2.0, 2.35, -0.1],
        [2.0, 2.35, 0.1],
    ];

    let normals = vec![
        // Front Face
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 1.0],
        // Back Face
        [0.0, 0.0, -1.0],
        [0.0, 0.0, -1.0],
        [0.0, 0.0, -1.0],
        // Bottom Face
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, -1.0, 0.0],
        // Left Slanted Face
        [-0.5, 0.86, 0.0],
        [-0.5, 0.86, 0.0],
        [-0.5, 0.86, 0.0],
        [-0.5, 0.86, 0.0],
        // Right Vertical Face
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
    ];

    let uvs = vec![
        // Front Face
        [0.0, 0.0],
        [1.0, 0.0],
        [1.0, 1.0],
        // Back Face
        [1.0, 0.0],
        [0.0, 0.0],
        [1.0, 1.0],
        // Bottom Face
        [0.0, 0.0],
        [1.0, 0.0],
        [0.0, 1.0],
        [1.0, 1.0],
        // Left Slanted Face
        [0.0, 0.0],
        [1.0, 0.0],
        [0.0, 1.0],
        [1.0, 1.0],
        // Right Vertical Face
        [0.0, 0.0],
        [1.0, 0.0],
        [0.0, 1.0],
        [1.0, 1.0],
    ];

    let indices = vec![
        // Front Face
        0, 1, 2, // Back Face
        5, 4, 3, // Bottom Face
        6, 8, 9, 6, 9, 7, // Left Slanted Face
        10, 11, 13, 10, 13, 12, // Right Vertical Face
        14, 17, 15, 14, 16, 17,
    ];

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

fn get_material_for_texture(
    texture_name: Option<&str>,
    asset_server: &AssetServer,
    materials: &mut Assets<StandardMaterial>,
    default_color: Color,
) -> Handle<StandardMaterial> {
    if let Some(name) = texture_name {
        let path = match name {
            "Wood Planks" => Some("textures/wood_planks.png"),
            "Limestone" => Some("textures/solid_limestone.png"),
            "Stone Brick" => Some("textures/solid_stone.png"),
            "Medieval Brick" => Some("textures/medieval_brick.png"),
            "Roof Shingles" => Some("textures/roof_shingles.png"),
            "Red Shingles" => Some("textures/red_roof_shingles.png"),
            "Rock Wall" => Some("textures/rock_wall.png"),
            "Solid Brick" => Some("textures/solid_brick.png"),
            "Wooden Door" => Some("textures/wooden_door.png"),
            "Cyber Door" => Some("textures/cyber_door.png"),
            _ => None,
        };
        if let Some(p) = path {
            return materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(asset_server.load(p)),
                perceptual_roughness: 0.8,
                ..default()
            });
        }
    }
    materials.add(StandardMaterial {
        base_color: default_color,
        perceptual_roughness: 0.85,
        ..default()
    })
}

#[allow(clippy::too_many_arguments)]
fn spawn_prefab_visuals(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    prefab_type: &str,
    position: Vec3,
    rotation: Quat,
    scale: Vec3,
    texture_override: Option<&str>,
    mansion_settings: &crate::play_mode::house::MansionSettings,
    index: usize,
    asset_server: &AssetServer,
    custom_mesh: Option<&EditableMesh>,
) -> Entity {
    let parent = commands
        .spawn((
            Transform::from_translation(position)
                .with_rotation(rotation)
                .with_scale(scale),
            Visibility::Visible,
            InheritedVisibility::default(),
            PlacedPrefabMarker {
                prefab_type: prefab_type.to_string(),
                position,
                index,
            },
            MapEditorEntity,
        ))
        .id();

    spawn_prefab_visuals_children(
        commands,
        meshes,
        materials,
        prefab_type,
        position,
        texture_override,
        mansion_settings,
        parent,
        asset_server,
        custom_mesh,
    );

    parent
}

use parking_lot::Mutex;
use std::sync::OnceLock;

#[allow(clippy::type_complexity)]
static PROCEDURAL_MESH_CACHE: OnceLock<Mutex<HashMap<(String, u32), Handle<Mesh>>>> =
    OnceLock::new();

#[allow(clippy::too_many_arguments)]
pub fn spawn_prefab_visuals_children(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    prefab_type: &str,
    position: Vec3,
    texture_override: Option<&str>,
    mansion_settings: &crate::play_mode::house::MansionSettings,
    parent: Entity,
    asset_server: &AssetServer,
    custom_mesh: Option<&EditableMesh>,
) {
    let cache = PROCEDURAL_MESH_CACHE.get_or_init(|| Mutex::new(HashMap::default()));
    let mut cache_guard = cache.lock();

    match prefab_type {
        s if s.starts_with("tree") || s == "shrub" || s == "cactus" => {
            let seed =
                ((position.x.abs() * 1000.0) as u32 ^ (position.z.abs() * 1000.0) as u32) | 1;
            let cache_seed = (seed % 16) | 1;
            let cache_key_trunk = (format!("{}_trunk", s), cache_seed);
            let cache_key_leaves = (format!("{}_leaves", s), cache_seed);

            let trunk_handle = if let Some(handle) = cache_guard.get(&cache_key_trunk) {
                handle.clone()
            } else {
                let (trunk_mesh, leaves_mesh) = tree_generator::build_tree_meshes(cache_seed, s);
                let t_handle = meshes.add(trunk_mesh);
                let l_handle = meshes.add(leaves_mesh);
                cache_guard.insert(cache_key_trunk.clone(), t_handle.clone());
                cache_guard.insert(cache_key_leaves.clone(), l_handle.clone());
                t_handle
            };

            let leaves_handle = cache_guard.get(&cache_key_leaves).unwrap().clone();

            let trunk = commands
                .spawn((
                    Mesh3d(trunk_handle),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::WHITE,
                        perceptual_roughness: 0.85,
                        ..default()
                    })),
                    Transform::default(),
                    MapEditorEntity,
                ))
                .id();

            let leaves = commands
                .spawn((
                    Mesh3d(leaves_handle),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::WHITE,
                        perceptual_roughness: 0.75,
                        ..default()
                    })),
                    Transform::default(),
                    MapEditorEntity,
                ))
                .id();

            commands.entity(parent).add_child(trunk).add_child(leaves);
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

            let cache_seed = (seed % 16) | 1;
            let cache_key = ("rock".to_string(), cache_seed);

            let rock_handle = if let Some(handle) = cache_guard.get(&cache_key) {
                handle.clone()
            } else {
                let rock_mesh = tree_generator::build_rock_mesh(cache_seed);
                let handle = meshes.add(rock_mesh);
                cache_guard.insert(cache_key, handle.clone());
                handle
            };

            let rock = commands
                .spawn((
                    Mesh3d(rock_handle),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::WHITE,
                        perceptual_roughness: 0.95,
                        metallic: 0.05,
                        ..default()
                    })),
                    Transform::from_scale(Vec3::new(scale_x, scale_y, scale_z)),
                    MapEditorEntity,
                ))
                .id();

            commands.entity(parent).add_child(rock);
        }
        s if s.starts_with("ore_") => {
            let seed = ((position.x.abs() * 500.0) as u32 ^ (position.z.abs() * 500.0) as u32) | 1;
            let mut lcg_s = seed;
            let mut next_rand = move || {
                lcg_s = lcg_s.wrapping_mul(1103515245).wrapping_add(12345);
                (lcg_s as f32) / (u32::MAX as f32)
            };

            let cache_seed = (seed % 16) | 1;
            let cache_key = (s.to_string(), cache_seed);

            let rock_handle = if let Some(handle) = cache_guard.get(&cache_key) {
                handle.clone()
            } else {
                let rock_mesh = tree_generator::build_rock_mesh(cache_seed);
                let handle = meshes.add(rock_mesh);
                cache_guard.insert(cache_key, handle.clone());
                handle
            };

            let base_color = if s == "ore_granite" {
                Color::srgb(0.2, 0.2, 0.22)
            } else {
                Color::srgb(0.35, 0.35, 0.37)
            };

            let base_rock = commands
                .spawn((
                    Mesh3d(rock_handle),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color,
                        perceptual_roughness: 0.9,
                        metallic: 0.05,
                        ..default()
                    })),
                    Transform::from_scale(Vec3::new(1.0, 0.6, 1.0)),
                    MapEditorEntity,
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

                let shard_seed = ((seed + i + 1) % 16) | 1;
                let shard_key = ("crystal_shard".to_string(), shard_seed);
                let shard_handle = if let Some(handle) = cache_guard.get(&shard_key) {
                    handle.clone()
                } else {
                    let mesh = tree_generator::build_rock_mesh(shard_seed);
                    let handle = meshes.add(mesh);
                    cache_guard.insert(shard_key, handle.clone());
                    handle
                };

                let shard = commands
                    .spawn((
                        Mesh3d(shard_handle),
                        MeshMaterial3d(crystal_mat.clone()),
                        Transform::from_translation(Vec3::new(offset_x, offset_y, offset_z))
                            .with_rotation(Quat::from_euler(EulerRot::YXZ, ry, rx, rz))
                            .with_scale(c_scale),
                        MapEditorEntity,
                    ))
                    .id();

                commands.entity(parent).add_child(shard);
            }
        }
        "spawn_point" => {
            let marker = commands
                .spawn((
                    Mesh3d(meshes.add(Sphere::new(0.4))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb(0.9, 0.1, 0.15),
                        emissive: LinearRgba::from(Color::srgb(0.8, 0.0, 0.15)),
                        perceptual_roughness: 0.15,
                        ..default()
                    })),
                    Transform::from_xyz(0.0, 0.4, 0.0),
                    MapEditorEntity,
                ))
                .id();

            commands.entity(parent).add_child(marker);
        }
        "cave_entrance" => {
            let rock_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.22, 0.25, 0.22),
                base_color_texture: Some(asset_server.load("textures/rock.png")),
                perceptual_roughness: 0.92,
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
                emissive: LinearRgba::new(0.5, 2.5, 4.0, 1.0),
                alpha_mode: AlphaMode::Blend,
                ..default()
            });

            let boulder_mesh = meshes.add(Sphere::new(1.0).mesh().ico(3).unwrap());

            // Dark cavern interior backdrop
            let interior = commands
                .spawn((
                    Mesh3d(boulder_mesh.clone()),
                    MeshMaterial3d(dark_interior_mat),
                    Transform::from_xyz(0.0, 1.6, -0.4).with_scale(Vec3::new(1.6, 1.4, 1.0)),
                    MapEditorEntity,
                ))
                .id();

            // Surrounding natural rock formations
            let r1 = commands
                .spawn((
                    Mesh3d(boulder_mesh.clone()),
                    MeshMaterial3d(rock_mat.clone()),
                    Transform::from_xyz(-1.6, 1.2, -0.2)
                        .with_scale(Vec3::new(1.4, 1.8, 1.3))
                        .with_rotation(Quat::from_rotation_y(0.4)),
                    MapEditorEntity,
                ))
                .id();
            let r2 = commands
                .spawn((
                    Mesh3d(boulder_mesh.clone()),
                    MeshMaterial3d(rock_mat.clone()),
                    Transform::from_xyz(1.6, 1.3, -0.2)
                        .with_scale(Vec3::new(1.5, 1.9, 1.4))
                        .with_rotation(Quat::from_rotation_y(-0.5)),
                    MapEditorEntity,
                ))
                .id();
            let r3 = commands
                .spawn((
                    Mesh3d(boulder_mesh.clone()),
                    MeshMaterial3d(rock_mat.clone()),
                    Transform::from_xyz(0.0, 3.1, 0.1)
                        .with_scale(Vec3::new(2.2, 1.3, 1.5))
                        .with_rotation(Quat::from_rotation_z(0.1)),
                    MapEditorEntity,
                ))
                .id();
            let r4 = commands
                .spawn((
                    Mesh3d(boulder_mesh.clone()),
                    MeshMaterial3d(rock_mat.clone()),
                    Transform::from_xyz(-2.2, 0.6, 0.4).with_scale(Vec3::new(1.0, 0.9, 1.1)),
                    MapEditorEntity,
                ))
                .id();
            let r5 = commands
                .spawn((
                    Mesh3d(boulder_mesh.clone()),
                    MeshMaterial3d(rock_mat),
                    Transform::from_xyz(2.1, 0.7, 0.5).with_scale(Vec3::new(1.1, 1.0, 1.0)),
                    MapEditorEntity,
                ))
                .id();

            // Glowing Cave Portal Ring
            let ring = commands
                .spawn((
                    Mesh3d(meshes.add(Torus::new(0.15, 1.3).mesh())),
                    MeshMaterial3d(portal_mat),
                    Transform::from_xyz(0.0, 1.6, 0.1)
                        .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
                    MapEditorEntity,
                ))
                .id();

            commands
                .entity(parent)
                .add_child(interior)
                .add_child(r1)
                .add_child(r2)
                .add_child(r3)
                .add_child(r4)
                .add_child(r5)
                .add_child(ring);
        }
        "house" => {
            let width = mansion_settings.cols as f32 * mansion_settings.cell_size;
            let depth = mansion_settings.rows as f32 * mansion_settings.cell_size;
            let height = 7.0;

            let marker = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(width, height, depth))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgba(0.2, 0.6, 1.0, 0.4),
                        alpha_mode: AlphaMode::Blend,
                        unlit: true,
                        ..default()
                    })),
                    Transform::from_xyz(0.0, height * 0.5, 0.0),
                    MapEditorEntity,
                ))
                .id();
            commands.entity(parent).add_child(marker);
        }
        "wall_straight" => {
            let mat = get_material_for_texture(
                texture_override,
                asset_server,
                materials,
                Color::srgb(0.55, 0.4, 0.28),
            );
            let wall = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(4.0, 3.5, 0.2))),
                    MeshMaterial3d(mat),
                    Transform::from_xyz(0.0, 1.75, 0.0),
                    MapEditorEntity,
                ))
                .id();
            commands.entity(parent).add_child(wall);
        }
        "wall_corner" => {
            let mat = get_material_for_texture(
                texture_override,
                asset_server,
                materials,
                Color::srgb(0.55, 0.4, 0.28),
            );
            let wall_a = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(4.0, 3.5, 0.2))),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_xyz(0.0, 1.75, -0.1),
                    MapEditorEntity,
                ))
                .id();
            let wall_b = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.2, 3.5, 4.0))),
                    MeshMaterial3d(mat),
                    Transform::from_xyz(-0.1, 1.75, 0.0),
                    MapEditorEntity,
                ))
                .id();
            commands.entity(parent).add_child(wall_a).add_child(wall_b);
        }
        "floor_tile" => {
            let mat = get_material_for_texture(
                texture_override,
                asset_server,
                materials,
                Color::srgb(0.45, 0.45, 0.48),
            );
            let floor = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(4.0, 0.2, 4.0))),
                    MeshMaterial3d(mat),
                    Transform::from_xyz(0.0, 0.1, 0.0),
                    MapEditorEntity,
                ))
                .id();
            // Concrete foundation base extending downward to fill hillside gaps
            let foundation_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.55, 0.55, 0.52),
                perceptual_roughness: 0.95,
                ..default()
            });
            let foundation = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(4.0, 2.0, 4.0))),
                    MeshMaterial3d(foundation_mat),
                    Transform::from_xyz(0.0, -1.0, 0.0),
                    MapEditorEntity,
                ))
                .id();
            commands
                .entity(parent)
                .add_child(floor)
                .add_child(foundation);
        }
        "roof_tile" => {
            let mat = get_material_for_texture(
                texture_override,
                asset_server,
                materials,
                Color::srgb(0.65, 0.2, 0.15),
            );
            let roof = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(4.0, 0.15, 4.0))),
                    MeshMaterial3d(mat),
                    Transform::from_xyz(0.0, 1.2, 0.0)
                        .with_rotation(Quat::from_rotation_x(35.0f32.to_radians())),
                    MapEditorEntity,
                ))
                .id();
            commands.entity(parent).add_child(roof);
        }
        "roof_gable" => {
            let mat = get_material_for_texture(
                texture_override,
                asset_server,
                materials,
                Color::srgb(0.85, 0.85, 0.82),
            );
            let mesh_handle = meshes.add(generate_roof_gable_mesh());
            let visual = commands
                .spawn((
                    Mesh3d(mesh_handle),
                    MeshMaterial3d(mat),
                    Transform::from_xyz(0.0, 0.0, 0.0),
                    MapEditorEntity,
                ))
                .id();
            commands.entity(parent).add_child(visual);
        }
        "door_frame" => {
            let mat = get_material_for_texture(
                texture_override,
                asset_server,
                materials,
                Color::srgb(0.55, 0.4, 0.28),
            );
            let pillar_l = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(1.4, 3.5, 0.2))),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_xyz(-1.3, 1.75, 0.0),
                    MapEditorEntity,
                ))
                .id();
            let pillar_r = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(1.4, 3.5, 0.2))),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_xyz(1.3, 1.75, 0.0),
                    MapEditorEntity,
                ))
                .id();
            let lintel = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(1.2, 1.1, 0.2))),
                    MeshMaterial3d(mat),
                    Transform::from_xyz(0.0, 2.95, 0.0),
                    MapEditorEntity,
                ))
                .id();
            commands
                .entity(parent)
                .add_child(pillar_l)
                .add_child(pillar_r)
                .add_child(lintel);
        }
        "window_frame" => {
            let mat = get_material_for_texture(
                texture_override,
                asset_server,
                materials,
                Color::srgb(0.55, 0.4, 0.28),
            );
            let metal_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.2, 0.2, 0.25),
                metallic: 0.8,
                perceptual_roughness: 0.4,
                ..default()
            });
            let bottom = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(4.0, 1.0, 0.2))),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_xyz(0.0, 0.5, 0.0),
                    MapEditorEntity,
                ))
                .id();
            let top = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(4.0, 0.9, 0.2))),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_xyz(0.0, 3.05, 0.0),
                    MapEditorEntity,
                ))
                .id();
            let side_l = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(1.2, 1.6, 0.2))),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_xyz(-1.4, 1.8, 0.0),
                    MapEditorEntity,
                ))
                .id();
            let side_r = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(1.2, 1.6, 0.2))),
                    MeshMaterial3d(mat),
                    Transform::from_xyz(1.4, 1.8, 0.0),
                    MapEditorEntity,
                ))
                .id();
            commands
                .entity(parent)
                .add_child(bottom)
                .add_child(top)
                .add_child(side_l)
                .add_child(side_r);

            // Steel window bars
            for i in 0..3 {
                let bx = -0.6 + (i as f32) * 0.6;
                let bar = commands
                    .spawn((
                        Mesh3d(meshes.add(Cylinder::new(0.02, 1.6))),
                        MeshMaterial3d(metal_mat.clone()),
                        Transform::from_xyz(bx, 1.8, 0.0),
                        MapEditorEntity,
                    ))
                    .id();
                commands.entity(parent).add_child(bar);
            }
        }
        "chest" => {
            let child = commands
                .spawn((
                    WorldAssetRoot(asset_server.load("Prop_Chest.gltf#Scene0")),
                    Transform::default(),
                    Visibility::Visible,
                    InheritedVisibility::default(),
                    MapEditorEntity,
                ))
                .id();
            commands.entity(parent).add_child(child);
        }
        "prop_chair" => {
            let child = commands
                .spawn((
                    WorldAssetRoot(asset_server.load("Prop_Chair.gltf#Scene0")),
                    Transform::default(),
                    Visibility::Visible,
                    InheritedVisibility::default(),
                    MapEditorEntity,
                ))
                .id();
            commands.entity(parent).add_child(child);
        }
        "prop_desk" => {
            let child = commands
                .spawn((
                    WorldAssetRoot(asset_server.load("Prop_Desk_L.gltf#Scene0")),
                    Transform::default(),
                    Visibility::Visible,
                    InheritedVisibility::default(),
                    MapEditorEntity,
                ))
                .id();
            commands.entity(parent).add_child(child);
        }
        "prop_health_pack" => {
            let child = commands
                .spawn((
                    WorldAssetRoot(asset_server.load("Prop_HealthPack.gltf#Scene0")),
                    Transform::default(),
                    Visibility::Visible,
                    InheritedVisibility::default(),
                    MapEditorEntity,
                ))
                .id();
            commands.entity(parent).add_child(child);
        }
        "prop_crate" => {
            let child = commands
                .spawn((
                    WorldAssetRoot(asset_server.load("Prop_Crate_Large.gltf#Scene0")),
                    Transform::default(),
                    Visibility::Visible,
                    InheritedVisibility::default(),
                    MapEditorEntity,
                ))
                .id();
            commands.entity(parent).add_child(child);
        }
        "workbench" => {
            let wood_mat = get_material_for_texture(
                texture_override,
                asset_server,
                materials,
                Color::srgb(0.5, 0.38, 0.28),
            );
            let top = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(2.0, 0.1, 1.0))),
                    MeshMaterial3d(wood_mat.clone()),
                    Transform::from_xyz(0.0, 0.85, 0.0),
                    MapEditorEntity,
                ))
                .id();
            commands.entity(parent).add_child(top);

            for dx in &[-0.9, 0.9] {
                for dz in &[-0.4, 0.4] {
                    let leg = commands
                        .spawn((
                            Mesh3d(meshes.add(Cylinder::new(0.05, 0.8))),
                            MeshMaterial3d(wood_mat.clone()),
                            Transform::from_xyz(*dx, 0.4, *dz),
                            MapEditorEntity,
                        ))
                        .id();
                    commands.entity(parent).add_child(leg);
                }
            }
        }
        "furnace" => {
            let stone_mat = get_material_for_texture(
                texture_override,
                asset_server,
                materials,
                Color::srgb(0.3, 0.3, 0.32),
            );
            let core_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.4, 0.0),
                emissive: LinearRgba::from(Color::srgb(1.0, 0.3, 0.0)) * 5.0,
                perceptual_roughness: 0.1,
                ..default()
            });
            let body = commands
                .spawn((
                    Mesh3d(meshes.add(Cylinder::new(0.6, 1.4))),
                    MeshMaterial3d(stone_mat),
                    Transform::from_xyz(0.0, 0.7, 0.0),
                    MapEditorEntity,
                ))
                .id();
            let core = commands
                .spawn((
                    Mesh3d(meshes.add(Sphere::new(0.24))),
                    MeshMaterial3d(core_mat),
                    Transform::from_xyz(0.0, 0.4, 0.5),
                    MapEditorEntity,
                ))
                .id();
            commands.entity(parent).add_child(body).add_child(core);
        }
        "bed" => {
            let wood_mat = get_material_for_texture(
                texture_override,
                asset_server,
                materials,
                Color::srgb(0.55, 0.4, 0.28),
            );
            let mattress_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.95, 0.95, 0.95),
                perceptual_roughness: 0.9,
                ..default()
            });
            let pillow_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.4, 0.6, 0.8),
                perceptual_roughness: 0.8,
                ..default()
            });
            let frame = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(1.2, 0.2, 2.0))),
                    MeshMaterial3d(wood_mat),
                    Transform::from_xyz(0.0, 0.1, 0.0),
                    MapEditorEntity,
                ))
                .id();
            let mattress = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(1.1, 0.2, 1.8))),
                    MeshMaterial3d(mattress_mat),
                    Transform::from_xyz(0.0, 0.3, 0.1),
                    MapEditorEntity,
                ))
                .id();
            let pillow = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.9, 0.1, 0.4))),
                    MeshMaterial3d(pillow_mat),
                    Transform::from_xyz(0.0, 0.45, -0.7),
                    MapEditorEntity,
                ))
                .id();
            commands
                .entity(parent)
                .add_child(frame)
                .add_child(mattress)
                .add_child(pillow);
        }
        "torch" => {
            let wood_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.45, 0.3, 0.18),
                perceptual_roughness: 0.8,
                ..default()
            });
            let flame_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.5, 0.0),
                emissive: LinearRgba::from(Color::srgb(1.0, 0.4, 0.0)) * 6.0,
                perceptual_roughness: 0.05,
                ..default()
            });

            // Torch handle tilted slightly forward
            let handle = commands
                .spawn((
                    Mesh3d(meshes.add(Cylinder::new(0.03, 0.6))),
                    MeshMaterial3d(wood_mat),
                    Transform::from_xyz(0.0, 0.3, 0.0)
                        .with_rotation(Quat::from_rotation_x(15.0f32.to_radians())),
                    MapEditorEntity,
                ))
                .id();

            // Torch flame
            let flame = commands
                .spawn((
                    Mesh3d(meshes.add(Sphere::new(0.08))),
                    MeshMaterial3d(flame_mat),
                    Transform::from_xyz(0.0, 0.62, 0.08),
                    MapEditorEntity,
                ))
                .id();

            commands.entity(parent).add_child(handle).add_child(flame);

            // Real dynamic point light component to illuminate surroundings in editor/play mode!
            let light = commands
                .spawn((
                    PointLight {
                        color: Color::srgb(1.0, 0.65, 0.2), // Warm torchlight glow
                        intensity: 1200.0,
                        range: 12.0,
                        shadow_maps_enabled: true,
                        ..default()
                    },
                    Transform::from_xyz(0.0, 0.8, 0.1),
                    MapEditorEntity,
                ))
                .id();
            commands.entity(parent).add_child(light);
        }
        "wall_t_junction" => {
            let mat = get_material_for_texture(
                texture_override,
                asset_server,
                materials,
                Color::srgb(0.55, 0.4, 0.28),
            );
            let wall_main = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(4.0, 3.5, 0.2))),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_xyz(0.0, 1.75, 0.0),
                    MapEditorEntity,
                ))
                .id();
            let wall_branch = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.2, 3.5, 2.0))),
                    MeshMaterial3d(mat),
                    Transform::from_xyz(0.0, 1.75, 1.0),
                    MapEditorEntity,
                ))
                .id();
            commands
                .entity(parent)
                .add_child(wall_main)
                .add_child(wall_branch);
        }
        "wall_cross" => {
            let mat = get_material_for_texture(
                texture_override,
                asset_server,
                materials,
                Color::srgb(0.55, 0.4, 0.28),
            );
            let wall_x = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(4.0, 3.5, 0.2))),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_xyz(0.0, 1.75, 0.0),
                    MapEditorEntity,
                ))
                .id();
            let wall_z = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.2, 3.5, 4.0))),
                    MeshMaterial3d(mat),
                    Transform::from_xyz(0.0, 1.75, 0.0),
                    MapEditorEntity,
                ))
                .id();
            commands.entity(parent).add_child(wall_x).add_child(wall_z);
        }
        "ceiling_tile" => {
            let mat = get_material_for_texture(
                texture_override,
                asset_server,
                materials,
                Color::srgb(0.85, 0.85, 0.82),
            );
            let tile = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(4.0, 0.15, 4.0))),
                    MeshMaterial3d(mat),
                    Transform::from_xyz(0.0, 0.0, 0.0),
                    MapEditorEntity,
                ))
                .id();
            commands.entity(parent).add_child(tile);
        }
        "fluorescent_light" => {
            let housing_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.7, 0.7, 0.72),
                metallic: 0.6,
                perceptual_roughness: 0.3,
                ..default()
            });
            let tube_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.95, 0.98, 1.0),
                emissive: LinearRgba::from(Color::srgb(0.9, 0.95, 1.0)) * 8.0,
                perceptual_roughness: 0.05,
                ..default()
            });
            let housing = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(1.2, 0.06, 0.18))),
                    MeshMaterial3d(housing_mat),
                    Transform::from_xyz(0.0, 3.45, 0.0),
                    MapEditorEntity,
                ))
                .id();
            let tube = commands
                .spawn((
                    Mesh3d(meshes.add(Cylinder::new(0.025, 1.0))),
                    MeshMaterial3d(tube_mat),
                    Transform::from_xyz(0.0, 3.4, 0.0)
                        .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                    MapEditorEntity,
                ))
                .id();
            let light = commands
                .spawn((
                    PointLight {
                        color: Color::srgb(0.95, 0.98, 1.0),
                        intensity: 2500.0,
                        range: 15.0,
                        shadow_maps_enabled: true,
                        ..default()
                    },
                    Transform::from_xyz(0.0, 3.3, 0.0),
                    MapEditorEntity,
                ))
                .id();
            commands
                .entity(parent)
                .add_child(housing)
                .add_child(tube)
                .add_child(light);
        }
        "hallway_segment" => {
            let mat = get_material_for_texture(
                texture_override,
                asset_server,
                materials,
                Color::srgb(0.55, 0.4, 0.28),
            );
            let floor_mat = get_material_for_texture(
                texture_override,
                asset_server,
                materials,
                Color::srgb(0.45, 0.45, 0.48),
            );
            let ceiling_mat = get_material_for_texture(
                texture_override,
                asset_server,
                materials,
                Color::srgb(0.85, 0.85, 0.82),
            );
            let floor = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(4.0, 0.2, 8.0))),
                    MeshMaterial3d(floor_mat),
                    Transform::from_xyz(0.0, 0.1, 0.0),
                    MapEditorEntity,
                ))
                .id();
            let wall_l = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.2, 3.5, 8.0))),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_xyz(-2.0, 1.75, 0.0),
                    MapEditorEntity,
                ))
                .id();
            let wall_r = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.2, 3.5, 8.0))),
                    MeshMaterial3d(mat),
                    Transform::from_xyz(2.0, 1.75, 0.0),
                    MapEditorEntity,
                ))
                .id();
            let ceiling = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(4.0, 0.15, 8.0))),
                    MeshMaterial3d(ceiling_mat),
                    Transform::from_xyz(0.0, 3.5, 0.0),
                    MapEditorEntity,
                ))
                .id();
            // Concrete foundation extending downward
            let found_mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.55, 0.55, 0.52),
                perceptual_roughness: 0.95,
                ..default()
            });
            let foundation = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(4.0, 2.0, 8.0))),
                    MeshMaterial3d(found_mat),
                    Transform::from_xyz(0.0, -1.0, 0.0),
                    MapEditorEntity,
                ))
                .id();
            commands
                .entity(parent)
                .add_child(floor)
                .add_child(wall_l)
                .add_child(wall_r)
                .add_child(ceiling)
                .add_child(foundation);
        }
        "room_pillar" => {
            let mat = get_material_for_texture(
                texture_override,
                asset_server,
                materials,
                Color::srgb(0.6, 0.58, 0.55),
            );
            let column = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.5, 3.5, 0.5))),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_xyz(0.0, 1.75, 0.0),
                    MapEditorEntity,
                ))
                .id();
            let base = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.7, 0.15, 0.7))),
                    MeshMaterial3d(mat.clone()),
                    Transform::from_xyz(0.0, 0.075, 0.0),
                    MapEditorEntity,
                ))
                .id();
            let capital = commands
                .spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.7, 0.15, 0.7))),
                    MeshMaterial3d(mat),
                    Transform::from_xyz(0.0, 3.425, 0.0),
                    MapEditorEntity,
                ))
                .id();
            commands
                .entity(parent)
                .add_child(column)
                .add_child(base)
                .add_child(capital);
        }
        s if s == "custom_asset" || s.starts_with("custom:") => {
            if let Some(asset_path) = texture_override {
                if asset_path.ends_with(".glb") || asset_path.ends_with(".gltf") {
                    let scene_path = format!("{}#Scene0", asset_path);
                    let child = commands
                        .spawn((
                            WorldAssetRoot(asset_server.load(&scene_path)),
                            Transform::default(),
                            Visibility::Visible,
                            InheritedVisibility::default(),
                            MapEditorEntity,
                        ))
                        .id();
                    commands.entity(parent).add_child(child);
                } else {
                    let placeholder = commands
                        .spawn((
                            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
                            MeshMaterial3d(materials.add(StandardMaterial {
                                base_color: Color::srgb(0.8, 0.3, 0.8),
                                ..default()
                            })),
                            Transform::from_xyz(0.0, 0.5, 0.0),
                            MapEditorEntity,
                        ))
                        .id();
                    commands.entity(parent).add_child(placeholder);
                }
            } else {
                let placeholder = commands
                    .spawn((
                        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(0.8, 0.3, 0.8),
                            ..default()
                        })),
                        Transform::from_xyz(0.0, 0.5, 0.0),
                        MapEditorEntity,
                    ))
                    .id();
                commands.entity(parent).add_child(placeholder);
            }
        }
        "custom_mesh" => {
            let mat = get_material_for_texture(
                texture_override,
                asset_server,
                materials,
                Color::srgb(0.65, 0.65, 0.62),
            );
            let mesh_data = if let Some(m) = custom_mesh {
                m.to_bevy_mesh()
            } else {
                EditableMesh::new_cube(1.0).to_bevy_mesh()
            };
            let child = commands
                .spawn((
                    Mesh3d(meshes.add(mesh_data)),
                    MeshMaterial3d(mat),
                    Transform::default(),
                    MapEditorEntity,
                ))
                .id();
            commands.entity(parent).add_child(child);
        }
        _ => {}
    }
}
#[allow(clippy::too_many_arguments)]
fn setup_map_editor(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    map: Res<TempestMap>,
    settings: Res<SplatmapSettings>,
    water_settings: Res<WaterSettings>,
    mansion_settings: Res<crate::play_mode::house::MansionSettings>,
) {
    // Camera with Ambient Light component
    commands.spawn((
        Camera3d::default(),
        Transform::default(),
        EditorCamera::default(),
        AmbientLight {
            color: Color::WHITE,
            brightness: 200.0,
            ..default()
        },
        MapEditorEntity,
    ));

    // Directional Light
    commands.spawn((
        DirectionalLight {
            illuminance: 3000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(20.0, 40.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        MapEditorEntity,
    ));

    // Terrain Mesh
    let mesh = generate_terrain_mesh(&map, &settings);
    let mesh_handle = meshes.add(mesh);

    commands.spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(asset_server.load("textures/ground_grass.png")),
            perceptual_roughness: 0.8,
            ..default()
        })),
        Transform::default(),
        TerrainMesh,
        MapEditorEntity,
    ));

    // Simulated Water Mesh (Translucent, tessellated wave mesh for small maps, flat plane for large maps)
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
            base_color: Color::srgba(0.02, 0.32, 0.78, 0.78), // Translucent deep sapphire blue
            alpha_mode: AlphaMode::Blend,
            perceptual_roughness: 0.08,
            metallic: 0.1,
            ..default()
        })),
        Transform::from_xyz(0.0, water_settings.height, 0.0),
        WaterSimData::new(map.width, map.height),
        WaterMesh,
        MapEditorEntity,
    ));

    // Spawn Placed Prefabs loaded from data
    for (idx, prefab) in map.prefabs.iter().enumerate() {
        let pos = Vec3::from_array(prefab.position);
        let rot = Quat::from_array(prefab.rotation);
        let scale = Vec3::from_array(prefab.scale);
        spawn_prefab_visuals(
            &mut commands,
            &mut meshes,
            &mut materials,
            &prefab.prefab_type,
            pos,
            rot,
            scale,
            prefab.texture_override.as_deref(),
            &mansion_settings,
            idx,
            &asset_server,
            prefab.custom_mesh.as_ref(),
        );
    }

    // Spawn Editor Bridges
    spawn_editor_bridges(
        &mut commands,
        &mut meshes,
        &mut materials,
        &map,
        &asset_server,
    );
}

fn cleanup_map_editor(
    mut commands: Commands,
    query: Query<Entity, (With<MapEditorEntity>, Without<ChildOf>)>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

#[derive(bevy::ecs::system::SystemParam)]
struct MapEditorUiParams<'w, 's> {
    commands: Commands<'w, 's>,
    contexts: EguiContexts<'w, 's>,
    next_state: ResMut<'w, NextState<AppState>>,
    brush: ResMut<'w, BrushSettings>,
    io_state: ResMut<'w, MapIOState>,
    splat_settings: ResMut<'w, SplatmapSettings>,
    water_settings: ResMut<'w, WaterSettings>,
    noise_settings: ResMut<'w, NoiseSettings>,
    resize_settings: ResMut<'w, MapResizeSettings>,
    biome_selection: ResMut<'w, BiomeSelection>,
    map: ResMut<'w, TempestMap>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    mansion_settings: ResMut<'w, crate::play_mode::house::MansionSettings>,
    selection_state: ResMut<'w, SelectionState>,
    custom_assets: ResMut<'w, CustomAssetLibrary>,
    geom_settings: ResMut<'w, GeometryEditorSettings>,
}

#[allow(
    clippy::too_many_arguments,
    clippy::collapsible_if,
    clippy::type_complexity,
    clippy::needless_range_loop
)]
fn map_editor_ui(
    params: MapEditorUiParams,
    terrain_query: Query<(Entity, &Mesh3d), With<TerrainMesh>>,
    mut prefab_query: Query<(Entity, &mut PlacedPrefabMarker)>,
    children_query: Query<&Children>,
    water_query: Query<(Entity, &Mesh3d), With<WaterMesh>>,
    bridge_query: Query<Entity, With<EditorBridge>>,
    asset_server: Res<AssetServer>,
) {
    let MapEditorUiParams {
        mut commands,
        mut contexts,
        mut next_state,
        mut brush,
        mut io_state,
        mut splat_settings,
        mut water_settings,
        mut noise_settings,
        mut resize_settings,
        mut biome_selection,
        mut map,
        mut meshes,
        mut materials,
        mut mansion_settings,
        mut selection_state,
        mut custom_assets,
        mut geom_settings,
    } = params;

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    // Apply a premium, modern dark theme template
    let mut visuals = egui::Visuals::dark();
    visuals.window_corner_radius = egui::CornerRadius::same(10);
    visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(6);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(6);

    // Sleek background & panel colors
    visuals.window_fill = egui::Color32::from_rgba_unmultiplied(20, 20, 25, 235); // semi-transparent charcoal
    visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(28, 28, 33);
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(38, 38, 45);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(55, 55, 66);
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(70, 70, 85);

    // Curated Indigo selection accent
    visuals.selection.bg_fill = egui::Color32::from_rgb(99, 102, 241);

    ctx.set_visuals(visuals);

    egui::Window::new("Map Editor Controls")
        .default_width(340.0)
        .default_height(600.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // 1. Top actions (Exit & Play)
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new("🚪 Exit to Main Menu")
                                .fill(egui::Color32::from_rgb(180, 50, 50))
                                .min_size(egui::vec2(120.0, 24.0)),
                        )
                        .clicked()
                    {
                        next_state.set(AppState::MainMenu);
                    }
                    if ui
                        .add(
                            egui::Button::new("🎮 Play Map")
                                .fill(egui::Color32::from_rgb(50, 150, 50))
                                .min_size(egui::vec2(100.0, 24.0)),
                        )
                        .clicked()
                    {
                        next_state.set(AppState::PlayMode);
                    }
                });
                ui.separator();

                // 2. Snapping controls (Always visible at top)
                ui.label("Snapping:");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut selection_state.snap_to_grid, "📎 Grid Snap");
                    ui.checkbox(&mut selection_state.snap_to_objects, "🧲 Object Snap");
                });
                if selection_state.snap_to_grid {
                    ui.add(
                        egui::Slider::new(&mut selection_state.snap_grid_size, 0.25..=4.0)
                            .text("Grid Size"),
                    );
                }
                ui.checkbox(
                    &mut selection_state.placement_flipped,
                    "Flip/Mirror placing prefab (Key: F)",
                );
                ui.separator();

                // 3. Selected Object Properties (Always visible if selection exists, in collapsible header)
                if let Some(sel_idx) = selection_state.selected_idx {
                    if sel_idx < map.prefabs.len() {
                        egui::CollapsingHeader::new("📋 Selected Object Properties")
                            .default_open(true)
                            .show(ui, |ui| {
        if let Some(sel_idx) = selection_state.selected_idx {
            if sel_idx < map.prefabs.len() {
                ui.separator();
                ui.heading("📋 Selected Object Properties");
                let prefab_type_label = map.prefabs[sel_idx].prefab_type.clone();
                ui.label(format!("Type: {}", prefab_type_label));

                let mut pos = map.prefabs[sel_idx].position;
                let mut changed = false;
                ui.horizontal(|ui| {
                    ui.label("Pos X:");
                    if ui
                        .add(egui::DragValue::new(&mut pos[0]).speed(0.1))
                        .changed()
                    {
                        changed = true;
                    }
                    ui.label("Y:");
                    if ui
                        .add(egui::DragValue::new(&mut pos[1]).speed(0.1))
                        .changed()
                    {
                        changed = true;
                    }
                    ui.label("Z:");
                    if ui
                        .add(egui::DragValue::new(&mut pos[2]).speed(0.1))
                        .changed()
                    {
                        changed = true;
                    }
                });
                if changed {
                    map.prefabs[sel_idx].position = pos;
                    // Update visual entity transform
                    let new_pos = Vec3::from_array(pos);
                    for (_, mut marker) in prefab_query.iter_mut() {
                        if marker.index == sel_idx {
                            marker.position = new_pos;
                        }
                    }
                }

                // Rotation (Euler Y/Pitch/Roll)
                let rot_quat = Quat::from_array(map.prefabs[sel_idx].rotation);
                let (mut yaw, mut pitch, mut roll) = rot_quat.to_euler(EulerRot::YXZ);
                yaw = yaw.to_degrees();
                pitch = pitch.to_degrees();
                roll = roll.to_degrees();
                let mut rot_changed = false;
                ui.horizontal(|ui| {
                    ui.label("Yaw:");
                    if ui
                        .add(egui::DragValue::new(&mut yaw).speed(1.0).suffix("°"))
                        .changed()
                    {
                        rot_changed = true;
                    }
                    ui.label("Pitch:");
                    if ui
                        .add(egui::DragValue::new(&mut pitch).speed(1.0).suffix("°"))
                        .changed()
                    {
                        rot_changed = true;
                    }
                    ui.label("Roll:");
                    if ui
                        .add(egui::DragValue::new(&mut roll).speed(1.0).suffix("°"))
                        .changed()
                    {
                        rot_changed = true;
                    }
                });
                if rot_changed {
                    let new_rot = Quat::from_euler(
                        EulerRot::YXZ,
                        yaw.to_radians(),
                        pitch.to_radians(),
                        roll.to_radians(),
                    );
                    map.prefabs[sel_idx].rotation = new_rot.to_array();
                }

                // Scale
                let mut scl = map.prefabs[sel_idx].scale;
                let mut scl_changed = false;
                ui.horizontal(|ui| {
                    ui.label("Scale X:");
                    if ui
                        .add(
                            egui::DragValue::new(&mut scl[0])
                                .speed(0.05)
                                .range(0.1..=10.0),
                        )
                        .changed()
                    {
                        scl_changed = true;
                    }
                    ui.label("Y:");
                    if ui
                        .add(
                            egui::DragValue::new(&mut scl[1])
                                .speed(0.05)
                                .range(0.1..=10.0),
                        )
                        .changed()
                    {
                        scl_changed = true;
                    }
                    ui.label("Z:");
                    if ui
                        .add(
                            egui::DragValue::new(&mut scl[2])
                                .speed(0.05)
                                .range(0.1..=10.0),
                        )
                        .changed()
                    {
                        scl_changed = true;
                    }
                });
                if scl_changed {
                    map.prefabs[sel_idx].scale = scl;
                }

                // Texture override dropdown
                let textures = [
                    "Default",
                    "Wood Planks",
                    "Limestone",
                    "Stone Brick",
                    "Medieval Brick",
                    "Roof Shingles",
                    "Red Shingles",
                    "Rock Wall",
                    "Solid Brick",
                    "Wooden Door",
                    "Cyber Door",
                ];
                let current_tex = map.prefabs[sel_idx]
                    .texture_override
                    .clone()
                    .unwrap_or("Default".to_string());
                egui::ComboBox::from_label("Texture")
                    .selected_text(&current_tex)
                    .show_ui(ui, |ui| {
                        for tex in &textures {
                            if ui.selectable_label(*tex == current_tex, *tex).clicked() {
                                if *tex == "Default" {
                                    map.prefabs[sel_idx].texture_override = None;
                                } else {
                                    map.prefabs[sel_idx].texture_override = Some(tex.to_string());
                                }
                            }
                        }
                    });

                // --- Geometry Mesh Editor Panel ---
                if map.prefabs[sel_idx].prefab_type == "custom_mesh" {
                    let (num_faces, num_vertices) =
                        if let Some(ref custom) = map.prefabs[sel_idx].custom_mesh {
                            (custom.faces.len(), custom.vertices.len())
                        } else {
                            (0, 0)
                        };

                    ui.separator();
                    ui.heading("📐 Geometry / Mesh Editor");
                    ui.label(format!("Faces: {} | Vertices: {}", num_faces, num_vertices));

                    // Selected face index slider
                    if num_faces > 0 {
                        ui.horizontal(|ui| {
                            ui.label("Selected Face:");
                            ui.add(
                                egui::Slider::new(
                                    &mut geom_settings.selected_face_idx,
                                    0..=(num_faces - 1),
                                )
                                .text("Face ID"),
                            );
                        });
                    }

                    ui.separator();
                    ui.label("Face Editing Operations:");

                    // Extrude Face
                    ui.horizontal(|ui| {
                        ui.label("Extrude Distance:");
                        ui.add(egui::Slider::new(
                            &mut geom_settings.extrude_dist,
                            -10.0..=10.0,
                        ));
                    });
                    if ui.button("🛠 Extrude Selected Face").clicked() {
                        let mut ok = false;
                        if let Some(ref mut custom) = map.prefabs[sel_idx].custom_mesh {
                            if geom_settings.selected_face_idx < custom.faces.len() {
                                custom.extrude(
                                    geom_settings.selected_face_idx,
                                    geom_settings.extrude_dist,
                                );
                                ok = true;
                            }
                        }
                        if ok {
                            respawn_selected_prefab_mesh(
                                &mut commands,
                                &mut meshes,
                                &mut materials,
                                &selection_state,
                                &map,
                                &mansion_settings,
                                &asset_server,
                                &prefab_query,
                                &children_query,
                            );
                        }
                    }

                    // Inset Face
                    ui.horizontal(|ui| {
                        ui.label("Inset Factor:");
                        ui.add(egui::Slider::new(
                            &mut geom_settings.inset_factor,
                            0.0..=0.99,
                        ));
                    });
                    if ui.button("🛠 Inset Selected Face").clicked() {
                        let mut ok = false;
                        if let Some(ref mut custom) = map.prefabs[sel_idx].custom_mesh {
                            if geom_settings.selected_face_idx < custom.faces.len() {
                                custom.inset(
                                    geom_settings.selected_face_idx,
                                    geom_settings.inset_factor,
                                );
                                ok = true;
                            }
                        }
                        if ok {
                            respawn_selected_prefab_mesh(
                                &mut commands,
                                &mut meshes,
                                &mut materials,
                                &selection_state,
                                &map,
                                &mansion_settings,
                                &asset_server,
                                &prefab_query,
                                &children_query,
                            );
                        }
                    }

                    // Bevel Edges
                    ui.horizontal(|ui| {
                        ui.label("Bevel Amount:");
                        ui.add(egui::Slider::new(
                            &mut geom_settings.bevel_amount,
                            0.0..=2.0,
                        ));
                    });
                    if ui.button("🛠 Bevel All Edges").clicked() {
                        let mut ok = false;
                        if let Some(ref mut custom) = map.prefabs[sel_idx].custom_mesh {
                            custom.bevel(geom_settings.bevel_amount);
                            ok = true;
                        }
                        if ok {
                            respawn_selected_prefab_mesh(
                                &mut commands,
                                &mut meshes,
                                &mut materials,
                                &selection_state,
                                &map,
                                &mansion_settings,
                                &asset_server,
                                &prefab_query,
                                &children_query,
                            );
                        }
                    }

                    // Subdivide Mesh
                    if ui.button("🛠 Subdivide Mesh").clicked() {
                        let mut ok = false;
                        if let Some(ref mut custom) = map.prefabs[sel_idx].custom_mesh {
                            custom.subdivide();
                            ok = true;
                        }
                        if ok {
                            respawn_selected_prefab_mesh(
                                &mut commands,
                                &mut meshes,
                                &mut materials,
                                &selection_state,
                                &map,
                                &mansion_settings,
                                &asset_server,
                                &prefab_query,
                                &children_query,
                            );
                        }
                    }

                    // Knife Cut Tool
                    ui.separator();
                    ui.label("🔪 Knife Cut Tool:");
                    ui.horizontal(|ui| {
                        ui.label("Cut Origin X:");
                        ui.add(egui::DragValue::new(&mut geom_settings.knife_origin.x).speed(0.1));
                        ui.label("Y:");
                        ui.add(egui::DragValue::new(&mut geom_settings.knife_origin.y).speed(0.1));
                        ui.label("Z:");
                        ui.add(egui::DragValue::new(&mut geom_settings.knife_origin.z).speed(0.1));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Cut Normal X:");
                        ui.add(egui::DragValue::new(&mut geom_settings.knife_normal.x).speed(0.1));
                        ui.label("Y:");
                        ui.add(egui::DragValue::new(&mut geom_settings.knife_normal.y).speed(0.1));
                        ui.label("Z:");
                        ui.add(egui::DragValue::new(&mut geom_settings.knife_normal.z).speed(0.1));
                    });
                    if ui.button("🔪 Apply Knife Cut").clicked() {
                        let normal = geom_settings.knife_normal.normalize_or_zero();
                        if normal.length_squared() > 0.001 {
                            let mut ok = false;
                            if let Some(ref mut custom) = map.prefabs[sel_idx].custom_mesh {
                                custom.knife_cut(geom_settings.knife_origin, normal);
                                ok = true;
                            }
                            if ok {
                                respawn_selected_prefab_mesh(
                                    &mut commands,
                                    &mut meshes,
                                    &mut materials,
                                    &selection_state,
                                    &map,
                                    &mansion_settings,
                                    &asset_server,
                                    &prefab_query,
                                    &children_query,
                                );
                            }
                        }
                    }

                    // Bridging Faces
                    ui.separator();
                    ui.label("🌉 Face Bridging:");
                    ui.horizontal(|ui| {
                        ui.label("Bridge to Face:");
                        ui.add(
                            egui::Slider::new(
                                &mut geom_settings.bridge_face_b,
                                0..=(num_faces - 1),
                            )
                            .text("Face ID B"),
                        );
                    });
                    if ui.button("🌉 Bridge Face A to B").clicked() {
                        let mut ok = false;
                        if let Some(ref mut custom) = map.prefabs[sel_idx].custom_mesh {
                            if geom_settings.selected_face_idx < custom.faces.len()
                                && geom_settings.bridge_face_b < custom.faces.len()
                                && geom_settings.selected_face_idx != geom_settings.bridge_face_b
                            {
                                custom.bridge(
                                    geom_settings.selected_face_idx,
                                    geom_settings.bridge_face_b,
                                );
                                ok = true;
                            }
                        }
                        if ok {
                            respawn_selected_prefab_mesh(
                                &mut commands,
                                &mut meshes,
                                &mut materials,
                                &selection_state,
                                &map,
                                &mansion_settings,
                                &asset_server,
                                &prefab_query,
                                &children_query,
                            );
                        }
                    }

                    // Boolean Operations
                    ui.separator();
                    ui.label("🔴 Boolean operations:");

                    let mut other_custom_meshes = Vec::new();
                    for (idx, p) in map.prefabs.iter().enumerate() {
                        if idx != sel_idx
                            && p.prefab_type == "custom_mesh"
                            && p.custom_mesh.is_some()
                        {
                            other_custom_meshes
                                .push((idx, format!("Mesh #{} (pos: {:?})", idx, p.position)));
                        }
                    }

                    if other_custom_meshes.is_empty() {
                        ui.label("Create another Custom Mesh to perform Boolean operations.");
                    } else {
                        // Dropdown for target selection
                        let mut selected_target_idx = geom_settings.bool_target_idx;
                        let combo_label = if let Some(t_idx) = selected_target_idx {
                            format!("Target: #{}", t_idx)
                        } else {
                            "Select Target Mesh".to_string()
                        };
                        egui::ComboBox::from_id_salt("bool_target_dropdown")
                            .selected_text(&combo_label)
                            .show_ui(ui, |ui| {
                                for &(idx, ref label) in &other_custom_meshes {
                                    if ui
                                        .selectable_label(selected_target_idx == Some(idx), label)
                                        .clicked()
                                    {
                                        selected_target_idx = Some(idx);
                                    }
                                }
                            });
                        geom_settings.bool_target_idx = selected_target_idx;

                        // Operation type buttons
                        ui.horizontal(|ui| {
                            ui.selectable_value(
                                &mut geom_settings.bool_op,
                                "Union".to_string(),
                                "Union ➕",
                            );
                            ui.selectable_value(
                                &mut geom_settings.bool_op,
                                "Subtraction".to_string(),
                                "Subtract ➖",
                            );
                            ui.selectable_value(
                                &mut geom_settings.bool_op,
                                "Intersection".to_string(),
                                "Intersect ✖",
                            );
                        });

                        if let Some(target_idx) = geom_settings.bool_target_idx {
                            if ui.button("🔴 Apply Boolean Operation").clicked() {
                                if target_idx < map.prefabs.len() {
                                    let sel_pos = Vec3::from_array(map.prefabs[sel_idx].position);
                                    let sel_rot = Quat::from_array(map.prefabs[sel_idx].rotation);

                                    let target_pos =
                                        Vec3::from_array(map.prefabs[target_idx].position);
                                    let target_rot =
                                        Quat::from_array(map.prefabs[target_idx].rotation);

                                    let rel_pos = target_pos - sel_pos;
                                    let rel_rot = sel_rot.inverse() * target_rot;

                                    let mut ok = false;
                                    let target_mesh_opt =
                                        map.prefabs[target_idx].custom_mesh.clone();
                                    if let Some(target_mesh) = target_mesh_opt {
                                        if let Some(ref mut custom) =
                                            map.prefabs[sel_idx].custom_mesh
                                        {
                                            custom.boolean_operation(
                                                &target_mesh,
                                                &geom_settings.bool_op.to_lowercase(),
                                                rel_pos,
                                                rel_rot,
                                            );
                                            ok = true;
                                        }
                                    }

                                    if ok {
                                        respawn_selected_prefab_mesh(
                                            &mut commands,
                                            &mut meshes,
                                            &mut materials,
                                            &selection_state,
                                            &map,
                                            &mansion_settings,
                                            &asset_server,
                                            &prefab_query,
                                            &children_query,
                                        );

                                        // Despawn target visual entity
                                        for (entity, marker) in prefab_query.iter() {
                                            if marker.index == target_idx {
                                                commands.entity(entity).despawn();
                                                break;
                                            }
                                        }

                                        // Remove target from prefabs array
                                        map.prefabs.remove(target_idx);

                                        // Reindex prefab markers
                                        reindex_prefab_markers(&map.prefabs, &mut prefab_query);

                                        // Adjust selected index
                                        selection_state.selected_idx =
                                            Some(if target_idx < sel_idx {
                                                sel_idx - 1
                                            } else {
                                                sel_idx
                                            });
                                        geom_settings.bool_target_idx = None;
                                    }
                                }
                            }
                        }
                    }
                }

                if ui.button("🗑️ Delete Selected").clicked() {
                    let p_pos = Vec3::from_array(map.prefabs[sel_idx].position);
                    for (entity, marker) in prefab_query.iter() {
                        if marker.position.distance(p_pos) < 0.05 {
                            commands.entity(entity).despawn();
                            break;
                        }
                    }
                    map.prefabs.remove(sel_idx);
                    reindex_prefab_markers(&map.prefabs, &mut prefab_query);
                    selection_state.selected_idx = None;
                }
            } else {
                selection_state.selected_idx = None;
            }
        }

        ui.separator();
                            });
                        ui.separator();
                    }
                }

                // 4. Terrain & Biome Sculpting Collapsible
                egui::CollapsingHeader::new("🌋 Terrain Sculpting & Biomes")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.heading("Sculpting Tools");
                        ui.separator();

                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut brush.tool, SculptTool::Raise, "Raise");
                            ui.selectable_value(&mut brush.tool, SculptTool::Lower, "Lower");
                            ui.selectable_value(&mut brush.tool, SculptTool::Smooth, "Smooth");
                            ui.selectable_value(&mut brush.tool, SculptTool::Disturb, "Disturb");
                            ui.selectable_value(&mut brush.tool, SculptTool::Rocky, "Rocky ⛰");
                        });

                        ui.label("Prefab Brushes:");
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut brush.tool, SculptTool::PlaceTreeOak, "Oak");
                            ui.selectable_value(&mut brush.tool, SculptTool::PlaceTreePine, "Pine");
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceTreeBirch,
                                "Birch",
                            );
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceShrub,
                                "Shrub 🌿",
                            );
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceCactus,
                                "Cactus 🌵",
                            );
                            ui.selectable_value(&mut brush.tool, SculptTool::PlaceRock, "Rock");
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceCaveEntrance,
                                "Cave 🕳️",
                            );
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceSpawnPoint,
                                "Spawn",
                            );
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceHouse,
                                "House 🏠",
                            );
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::DeletePrefab,
                                "Delete",
                            );
                        });

                        if brush.tool == SculptTool::PlaceHouse {
                            ui.separator();
                            ui.heading("Mansion Settings");
                            ui.add(
                                egui::Slider::new(&mut mansion_settings.cols, 4..=12)
                                    .text("Columns"),
                            );
                            ui.add(
                                egui::Slider::new(&mut mansion_settings.rows, 2..=6).text("Rows"),
                            );
                            ui.add(
                                egui::Slider::new(&mut mansion_settings.cell_size, 4.0..=8.0)
                                    .text("Cell Size (m)"),
                            );

                            let w = mansion_settings.cols as f32 * mansion_settings.cell_size;
                            let d = mansion_settings.rows as f32 * mansion_settings.cell_size;
                            ui.label(format!(
                                "Mansion Footprint: {:.1}m x {:.1}m ({} Bedrooms)",
                                w,
                                d,
                                (mansion_settings.cols * (mansion_settings.rows - 2) * 2) + 12
                            ));
                        }

                        ui.label("Crafting Ore Brushes:");
                        ui.horizontal(|ui| {
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceOreCopper,
                                "Copper 🔸",
                            );
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceOreIron,
                                "Iron 🟫",
                            );
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceOreGold,
                                "Gold 🟡",
                            );
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceOreSilver,
                                "Silver ◽",
                            );
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceOrePlatinum,
                                "Platinum 💎",
                            );
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceOreSteel,
                                "Steel 🔗",
                            );
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceOreGranite,
                                "Granite ◼",
                            );
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceProceduralWall,
                                "Procedural Wall 🧱",
                            );
                        });

                        ui.separator();

                        ui.separator();
                        ui.add(egui::Slider::new(&mut brush.size, 1.0..=20.0).text("Brush Size"));
                        ui.add(
                            egui::Slider::new(&mut brush.strength, 0.5..=25.0)
                                .text("Brush Strength"),
                        );

                        ui.separator();

                        ui.separator();
                        ui.heading("Splatmap & Biome Settings");

                        let mut splat_changed = false;

                        ui.horizontal(|ui| {
                            ui.label("Active Biome:");
                            egui::ComboBox::from_label("")
                                .selected_text(format!("{:?}", splat_settings.biome))
                                .show_ui(ui, |ui| {
                                    if ui
                                        .selectable_value(
                                            &mut splat_settings.biome,
                                            Biome::Temperate,
                                            "Temperate",
                                        )
                                        .clicked()
                                    {
                                        splat_changed = true;
                                    }
                                    if ui
                                        .selectable_value(
                                            &mut splat_settings.biome,
                                            Biome::Arid,
                                            "Arid",
                                        )
                                        .clicked()
                                    {
                                        splat_changed = true;
                                    }
                                    if ui
                                        .selectable_value(
                                            &mut splat_settings.biome,
                                            Biome::Tundra,
                                            "Tundra",
                                        )
                                        .clicked()
                                    {
                                        splat_changed = true;
                                    }
                                    if ui
                                        .selectable_value(
                                            &mut splat_settings.biome,
                                            Biome::Arctic,
                                            "Arctic",
                                        )
                                        .clicked()
                                    {
                                        splat_changed = true;
                                    }
                                });
                        });

                        if ui
                            .add(
                                egui::Slider::new(&mut splat_settings.sand_height, 0.0..=5.0)
                                    .text("Beach Level"),
                            )
                            .changed()
                        {
                            splat_changed = true;
                        }
                        if ui
                            .add(
                                egui::Slider::new(&mut splat_settings.snow_height, 2.0..=20.0)
                                    .text("Snow Level"),
                            )
                            .changed()
                        {
                            splat_changed = true;
                        }
                        if ui
                            .add(
                                egui::Slider::new(&mut splat_settings.cliff_steepness, 0.3..=0.95)
                                    .text("Cliff Limit"),
                            )
                            .changed()
                        {
                            splat_changed = true;
                        }

                        if splat_changed {
                            for (entity, mesh_3d) in terrain_query.iter() {
                                rebuild_terrain_mesh(
                                    entity,
                                    &mut commands,
                                    &map,
                                    &splat_settings,
                                    &mut meshes,
                                    Some(mesh_3d),
                                );
                            }
                        }

                        ui.separator();
                    });
                ui.separator();

                // 5. Prefab & Modular Placement Collapsible
                egui::CollapsingHeader::new("🏗️ Prefab & Building Blocks")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.heading("🏗️ Object & Building Mode");
                        ui.label("Modular Building Blocks:");
                        ui.horizontal(|ui| {
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceModularFloor,
                                "Floor 🟫",
                            );
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceModularWall,
                                "Wall 🧱",
                            );
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceModularCorner,
                                "Corner 📐",
                            );
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceModularRoof,
                                "Roof 🏠",
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceModularDoorFrame,
                                "Door 🚪",
                            );
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceModularWindowFrame,
                                "Window 🪟",
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceWallTJunction,
                                "T-Junction",
                            );
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceWallCross,
                                "Cross ✚",
                            );
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceCeilingTile,
                                "Ceiling ⬜",
                            );
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceModularRoofGable,
                                "Roof Gable 📐",
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceFluorescentLight,
                                "Light 💡",
                            );
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceHallwaySegment,
                                "Hallway ▬",
                            );
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceRoomPillar,
                                "Pillar ▮",
                            );
                        });
                        ui.label("Functional Structures:");
                        ui.horizontal(|ui| {
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceChest,
                                "Chest 🧰",
                            );
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceWorkbench,
                                "Workbench 🔨",
                            );
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceFurnace,
                                "Furnace 🔥",
                            );
                            ui.selectable_value(&mut brush.tool, SculptTool::PlaceBed, "Bed 🛏️");
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceTorch,
                                "Torch 🔦",
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceChair,
                                "Chair 🪑",
                            );
                            ui.selectable_value(&mut brush.tool, SculptTool::PlaceDesk, "Desk 🗄️");
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceHealthPack,
                                "Health 🏥",
                            );
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceCrate,
                                "Crate 📦",
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::SelectObject,
                                "✋ Select Object",
                            );
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::PlaceCustomMesh,
                                "🔷 Custom Mesh",
                            );
                            ui.selectable_value(
                                &mut brush.tool,
                                SculptTool::DeletePrefab,
                                "🗑️ Delete Object",
                            );
                        });

                        if brush.tool == SculptTool::PlaceCustomMesh {
                            ui.horizontal(|ui| {
                                ui.label("Shape Primitive:");
                                ui.selectable_value(
                                    &mut selection_state.custom_mesh_primitive,
                                    CustomMeshPrimitive::Cube,
                                    "Cube 🟥",
                                );
                                ui.selectable_value(
                                    &mut selection_state.custom_mesh_primitive,
                                    CustomMeshPrimitive::Sphere,
                                    "Sphere 🔮",
                                );
                            });
                        }

                        // Snapping controls
                        ui.separator();

                        ui.separator();
                        ui.heading("\u{1f4c1} Custom Asset Import");
                        ui.horizontal(|ui| {
                            ui.label("Asset path:");
                            ui.text_edit_singleline(&mut custom_assets.import_path);
                        });
                        ui.label("Place .glb/.gltf/.obj files in the assets/ folder.");
                        ui.label("Enter the filename (e.g. my_building.glb)");
                        if ui.button("\u{2795} Import Asset").clicked() {
                            let path = custom_assets.import_path.trim().to_string();
                            if !path.is_empty() {
                                let asset_type =
                                    if path.ends_with(".glb") || path.ends_with(".gltf") {
                                        CustomAssetType::Gltf
                                    } else if path.ends_with(".obj") {
                                        CustomAssetType::Obj
                                    } else if path.ends_with(".png")
                                        || path.ends_with(".jpg")
                                        || path.ends_with(".jpeg")
                                    {
                                        CustomAssetType::Image
                                    } else {
                                        CustomAssetType::Gltf
                                    };
                                let name = path
                                    .split('/')
                                    .next_back()
                                    .unwrap_or(&path)
                                    .split('.')
                                    .next()
                                    .unwrap_or(&path)
                                    .to_string();
                                custom_assets.assets.push(CustomAssetEntry {
                                    name: name.clone(),
                                    file_path: path.clone(),
                                    asset_type,
                                });
                                custom_assets.import_path.clear();
                            }
                        }

                        if !custom_assets.assets.is_empty() {
                            ui.label("Imported Assets:");
                            let mut to_select = custom_assets.selected_asset_idx;
                            for (i, entry) in custom_assets.assets.iter().enumerate() {
                                let label = format!(
                                    "{} ({})",
                                    entry.name,
                                    match entry.asset_type {
                                        CustomAssetType::Gltf => "GLTF",
                                        CustomAssetType::Obj => "OBJ",
                                        CustomAssetType::Image => "Texture",
                                    }
                                );
                                let selected = to_select == Some(i);
                                if ui.selectable_label(selected, &label).clicked() {
                                    to_select = Some(i);
                                    brush.tool = SculptTool::PlaceCustomAsset;
                                }
                            }
                            custom_assets.selected_asset_idx = to_select;
                        }

                        ui.separator();
                    });
                ui.separator();

                // 6. Environment & Generation settings
                egui::CollapsingHeader::new("⚙️ Environment & Generation")
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.heading("Water Plane Settings");
                        ui.add(
                            egui::Slider::new(&mut water_settings.height, -5.0..=15.0)
                                .text("Water Level"),
                        );

                        ui.separator();
                        ui.heading("Procedural Generator");
                        ui.add(egui::Slider::new(&mut noise_settings.seed, 0..=9999).text("Seed"));
                        ui.add(
                            egui::Slider::new(&mut noise_settings.frequency, 0.005..=0.15)
                                .text("Scale / Freq"),
                        );
                        ui.add(
                            egui::Slider::new(&mut noise_settings.octaves, 1..=8).text("Octaves"),
                        );
                        ui.add(
                            egui::Slider::new(&mut noise_settings.amplitude, 1.0..=25.0)
                                .text("Max Height / Amp"),
                        );
                        ui.add(
                            egui::Slider::new(&mut noise_settings.ridge_exponent, 0.5..=4.0)
                                .text("Ridge Exponent"),
                        );
                        ui.add(
                            egui::Slider::new(&mut noise_settings.height_offset, -5.0..=15.0)
                                .text("Sea/Height Offset"),
                        );

                        ui.add_space(5.0);
                        ui.label("Biomes to Include in Generation:");
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut biome_selection.temperate, "Temperate");
                            ui.checkbox(&mut biome_selection.arid, "Arid");
                            ui.checkbox(&mut biome_selection.tundra, "Tundra");
                            ui.checkbox(&mut biome_selection.arctic, "Arctic");
                        });

                        ui.add_space(3.0);
                        ui.checkbox(
                            &mut biome_selection.make_island,
                            "🏝️ Generate as Island (Ocean Borders)",
                        );
                        ui.add_space(5.0);
                        ui.separator();
                        ui.heading("🕳️ Underground Cave System");
                        ui.checkbox(
                            &mut biome_selection.generate_caves,
                            "🕳️ Include Underground Cave Maze & Surface Entrances",
                        );
                        ui.label(
                            "Generates a 3D underground cave maze level at Y = -150m with natural grotto entrances scattered across hills and mountains.",
                        );
                        ui.add_space(5.0);

                        if ui.button("Generate Procedural Terrain").clicked() {
                            // 1. Despawn existing visual prefabs and their children, then clear map.prefabs
                            for (entity, _) in prefab_query.iter() {
                                commands.entity(entity).despawn();
                            }
                            map.prefabs.clear();

                            // 2. Generate new heightmap using fractal Perlin noise
                            let perlin = PerlinNoise::new(noise_settings.seed);
                            let w = map.width;
                            let h = map.height;

                            let mut active_biomes = Vec::new();
                            if biome_selection.arctic {
                                active_biomes.push(Biome::Arctic);
                            }
                            if biome_selection.tundra {
                                active_biomes.push(Biome::Tundra);
                            }
                            if biome_selection.temperate {
                                active_biomes.push(Biome::Temperate);
                            }
                            if biome_selection.arid {
                                active_biomes.push(Biome::Arid);
                            }
                            if active_biomes.is_empty() {
                                active_biomes.push(Biome::Temperate); // fallback
                            }

                            let lake_perlin = PerlinNoise::new(noise_settings.seed + 888);

                            for z in 0..h {
                                for x in 0..w {
                                    let nx = x as f32 * noise_settings.frequency;
                                    let nz = z as f32 * noise_settings.frequency;

                                    // Primary land elevation: fractal Perlin noise in [-1.0, 1.0]
                                    let noise_val =
                                        perlin.fbm(nx, nz, noise_settings.octaves, 2.0, 0.5);

                                    let normalized = (noise_val + 1.0) * 0.5;
                                    // Gentle rolling dry land base (+1.8m elevation above water level 1.2m)
                                    let base_land_height = 1.8 + (normalized.powf(noise_settings.ridge_exponent) * noise_settings.amplitude);

                                    // Secondary low-frequency lake noise for deep lakes & ponds
                                    let lake_val = lake_perlin.fbm(nx * 0.35, nz * 0.35, 2, 2.0, 0.5);
                                    let mut final_height = base_land_height;

                                    // Carve out deep, substantial lakes and ponds where lake_val < -0.22
                                    if lake_val < -0.22 {
                                        let lake_factor = ((-0.22 - lake_val) / 0.78).clamp(0.0, 1.0);
                                        let carved_l_y = -1.5 - lake_factor.powf(1.3) * 4.5; // -1.5m to -6.0m deep!
                                        final_height = base_land_height * (1.0 - lake_factor) + carved_l_y * lake_factor;
                                    }

                                    final_height -= noise_settings.height_offset;

                                    if biome_selection.make_island {
                                        let dx = (x as f32 - (w as f32 / 2.0)) / (w as f32 / 2.0);
                                        let dz = (z as f32 - (h as f32 / 2.0)) / (h as f32 / 2.0);
                                        let d = (dx * dx + dz * dz).sqrt();
                                        let mask = (1.0 - d.powf(3.2)).clamp(0.0, 1.0);
                                        let ocean_depth = -8.0;
                                        final_height =
                                            final_height * mask + ocean_depth * (1.0 - mask);
                                    }

                                    map.set_height(x, z, final_height);

                                    // Determine vertex biome procedurally based on active checklist selection
                                    // Form organic north-to-south bands: Arctic (North) -> Tundra -> Temperate -> Arid/Desert (South)
                                    let z_frac = z as f32 / h as f32;
                                    let wobble =
                                        perlin.fbm(x as f32 * 0.012, z as f32 * 0.012, 2, 2.0, 0.5)
                                            * 0.07;
                                    let val = (z_frac + wobble).clamp(0.0, 0.999);
                                    let biome_idx =
                                        (val * active_biomes.len() as f32).floor() as usize;
                                    let vertex_biome = active_biomes[biome_idx];
                                    map.set_biome(x, z, vertex_biome);
                                }
                            }

                            // 2b. Clean up & initialize house and spawn point prefabs
                            let house_pos = Vec3::new(-35.0, 1.5, -35.0);
                            let spawn_pos = Vec3::new(0.0, 2.0, 5.0);

                            map.prefabs.push(PlacedPrefab {
                                prefab_type: "house".to_string(),
                                position: house_pos.to_array(),
                                rotation: [0.0, 0.0, 0.0, 1.0],
                                scale: [1.0, 1.0, 1.0],
                                texture_override: None,
                                rotation_y: Some(0.0),
                                custom_mesh: None,
                            });
                            map.prefabs.push(PlacedPrefab {
                                prefab_type: "spawn_point".to_string(),
                                position: spawn_pos.to_array(),
                                rotation: [0.0, 0.0, 0.0, 1.0],
                                scale: [1.0, 1.0, 1.0],
                                texture_override: None,
                                rotation_y: Some(0.0),
                                custom_mesh: None,
                            });

                            // Flatten the terrain under the house footprint to height 1.5
                            let half_w =
                                (mansion_settings.cols as f32 * mansion_settings.cell_size) / 2.0;
                            let half_d =
                                (mansion_settings.rows as f32 * mansion_settings.cell_size) / 2.0;
                            let half_map_w = w as f32 / 2.0;
                            let half_map_h = h as f32 / 2.0;

                            let min_x_idx =
                                ((house_pos.x - half_w - 2.0) + half_map_w).max(0.0) as u32;
                            let max_x_idx =
                                ((house_pos.x + half_w + 2.0) + half_map_w).min(w as f32) as u32;
                            let min_z_idx =
                                ((house_pos.z - half_d - 2.0) + half_map_h).max(0.0) as u32;
                            let max_z_idx =
                                ((house_pos.z + half_d + 2.0) + half_map_h).min(h as f32) as u32;

                            let natural_h = map.get_height(house_pos.x as u32, house_pos.z as u32);
                            let house_ground_y = natural_h.clamp(1.5, 45.0);

                            for mz in min_z_idx..max_z_idx {
                                for mx in min_x_idx..max_x_idx {
                                    map.set_height(mx, mz, house_ground_y);
                                    map.set_biome(mx, mz, Biome::Temperate);
                                }
                            }

                            // Spawn visual entities for house and spawn point in the editor
                            spawn_prefab_visuals(
                                &mut commands,
                                &mut meshes,
                                &mut materials,
                                "house",
                                house_pos,
                                Quat::IDENTITY,
                                Vec3::ONE,
                                None,
                                &mansion_settings,
                                0,
                                &asset_server,
                                None,
                            );
                            spawn_prefab_visuals(
                                &mut commands,
                                &mut meshes,
                                &mut materials,
                                "spawn_point",
                                spawn_pos,
                                Quat::IDENTITY,
                                Vec3::ONE,
                                None,
                                &mansion_settings,
                                1,
                                &asset_server,
                                None,
                            );

                            // 2c. Spawn Natural Cave Entrances inland on dry land if enabled
                            if biome_selection.generate_caves {
                                let cave_coords = [
                                    Vec3::new(-150.0, 0.0, -140.0),
                                    Vec3::new(160.0, 0.0, -130.0),
                                    Vec3::new(-140.0, 0.0, 150.0),
                                    Vec3::new(170.0, 0.0, 140.0),
                                ];

                                for mut c_pos in cave_coords {
                                    let mut cx_idx = ((c_pos.x + half_map_w).round() as i32)
                                        .clamp(1, w as i32 - 2) as u32;
                                    let mut cz_idx = ((c_pos.z + half_map_h).round() as i32)
                                        .clamp(1, h as i32 - 2) as u32;
                                    let mut h_ground = map.get_height(cx_idx, cz_idx);

                                    // Step inward toward center if selected spot is in ocean (< 1.6m)
                                    let mut step = 0;
                                    while h_ground < 1.6 && step < 15 {
                                        c_pos.x *= 0.85;
                                        c_pos.z *= 0.85;
                                        cx_idx = ((c_pos.x + half_map_w).round() as i32)
                                            .clamp(1, w as i32 - 2) as u32;
                                        cz_idx = ((c_pos.z + half_map_h).round() as i32)
                                            .clamp(1, h as i32 - 2) as u32;
                                        h_ground = map.get_height(cx_idx, cz_idx);
                                        step += 1;
                                    }
                                    c_pos.y = h_ground;

                                    let prefab_idx = map.prefabs.len();
                                    map.prefabs.push(PlacedPrefab {
                                        prefab_type: "cave_entrance".to_string(),
                                        position: c_pos.to_array(),
                                        rotation: [0.0, 0.0, 0.0, 1.0],
                                        scale: [1.0, 1.0, 1.0],
                                        texture_override: None,
                                        rotation_y: Some(0.0),
                                        custom_mesh: None,
                                    });

                                    spawn_prefab_visuals(
                                        &mut commands,
                                        &mut meshes,
                                        &mut materials,
                                        "cave_entrance",
                                        c_pos,
                                        Quat::IDENTITY,
                                        Vec3::ONE,
                                        None,
                                        &mansion_settings,
                                        prefab_idx,
                                        &asset_server,
                                        None,
                                    );
                                }
                            }

                            // Generate roads and smooth the terrain underneath them
                            generate_roads_on_map(&mut map);

                            // 3. Dynamic splatmap topology analyzer & spawner matching the active biome
                            let density_factor = 0.04; // 4% chance to spawn trees/rocks in flat, fertile regions
                            let offset_x = -(w as f32) / 2.0;
                            let offset_z = -(h as f32) / 2.0;
                            for z in 2..(h - 2) {
                                for x in 2..(w - 2) {
                                    let y = map.get_height(x, z);

                                    // Skip placing obstacles (trees, rocks, ores) directly on top of roads!
                                    if map.get_road(x, z) > 0 {
                                        continue;
                                    }

                                    // Below beach or above snow level = no trees
                                    if y <= splat_settings.sand_height
                                        || y >= splat_settings.snow_height
                                    {
                                        continue;
                                    }

                                    // Compute slope normal at this vertex
                                    let y_l = map.get_height(x - 1, z);
                                    let y_r = map.get_height(x + 1, z);
                                    let y_u = map.get_height(x, z - 1);
                                    let y_d = map.get_height(x, z + 1);
                                    let normal = Vec3::new(y_l - y_r, 2.0, y_u - y_d).normalize();

                                    // If the slope is flat (meaning normal.y is above steepness threshold) - perfect grass area!
                                    if normal.y >= splat_settings.cliff_steepness {
                                        // Generate deterministic pseudo-random scatter hash
                                        let hash = (((x * 123 + z * 4567) as f32).sin()
                                            * 43_758.547)
                                            .fract()
                                            .abs();
                                        if hash < density_factor {
                                            // Assign prefab species based on the actual cell biome
                                            let cell_biome = map.get_biome(x, z);
                                            let prefab_type = match cell_biome {
                                                Biome::Temperate => {
                                                    if hash < density_factor * 0.4 {
                                                        "tree_oak"
                                                    } else if hash < density_factor * 0.75 {
                                                        "tree_birch"
                                                    } else if hash < density_factor * 0.93 {
                                                        "shrub"
                                                    } else {
                                                        "rock"
                                                    }
                                                }
                                                Biome::Arid => {
                                                    if hash < density_factor * 0.5 {
                                                        "cactus"
                                                    } else {
                                                        "rock"
                                                    }
                                                }
                                                Biome::Tundra => {
                                                    if hash < density_factor * 0.7 {
                                                        "tree_pine"
                                                    } else {
                                                        "rock"
                                                    }
                                                }
                                                Biome::Arctic => {
                                                    if hash < density_factor * 0.4 {
                                                        "tree_pine"
                                                    } else {
                                                        "rock"
                                                    }
                                                }
                                            };

                                            let pos = Vec3::new(
                                                x as f32 + offset_x,
                                                y,
                                                z as f32 + offset_z,
                                            );
                                            let rot_y = hash * std::f32::consts::TAU;

                                            let rot = Quat::from_rotation_y(rot_y);
                                            let idx = map.prefabs.len();

                                            spawn_prefab_visuals(
                                                &mut commands,
                                                &mut meshes,
                                                &mut materials,
                                                prefab_type,
                                                pos,
                                                rot,
                                                Vec3::ONE,
                                                None,
                                                &mansion_settings,
                                                idx,
                                                &asset_server,
                                                None,
                                            );

                                            map.prefabs.push(PlacedPrefab {
                                                prefab_type: prefab_type.to_string(),
                                                position: pos.to_array(),
                                                rotation: rot.to_array(),
                                                scale: [1.0, 1.0, 1.0],
                                                texture_override: None,
                                                rotation_y: Some(rot_y),
                                                custom_mesh: None,
                                            });
                                        }
                                    }
                                }
                            }

                            // 4. Rebuild the 3D terrain mesh instantly with the biome's splatmap color scheme
                            for (entity, mesh_3d) in terrain_query.iter() {
                                rebuild_terrain_mesh(
                                    entity,
                                    &mut commands,
                                    &map,
                                    &splat_settings,
                                    &mut meshes,
                                    Some(mesh_3d),
                                );
                            }

                            // Despawn old bridges and spawn new ones
                            for bridge_entity in bridge_query.iter() {
                                commands.entity(bridge_entity).despawn();
                            }
                            spawn_editor_bridges(
                                &mut commands,
                                &mut meshes,
                                &mut materials,
                                &map,
                                &asset_server,
                            );

                            // 5. Fire event to regenerate procedural grass across the new terrain
                            commands.run_system_cached(
                                |mut ev_grass: MessageWriter<crate::grass::GenerateGrassEvent>| {
                                    ev_grass.write(crate::grass::GenerateGrassEvent);
                                },
                            );
                        }

                        ui.add_space(3.0);
                        if ui.button("Generate Road Network").clicked() {
                            generate_roads_on_map(&mut map);
                            for (entity, mesh_3d) in terrain_query.iter() {
                                rebuild_terrain_mesh(
                                    entity,
                                    &mut commands,
                                    &map,
                                    &splat_settings,
                                    &mut meshes,
                                    Some(mesh_3d),
                                );
                            }

                            // Despawn old bridges and spawn new ones
                            for bridge_entity in bridge_query.iter() {
                                commands.entity(bridge_entity).despawn();
                            }
                            spawn_editor_bridges(
                                &mut commands,
                                &mut meshes,
                                &mut materials,
                                &map,
                                &asset_server,
                            );
                        }

                        ui.separator();
                    });
                ui.separator();

                // 7. Save & Load Map File operations
                egui::CollapsingHeader::new("💾 Save & Load Map")
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.heading("File Operations");
                        ui.horizontal(|ui| {
                            ui.label("Path:");
                            ui.text_edit_singleline(&mut io_state.filename);
                        });

                        ui.horizontal(|ui| {
                            if ui.button("Export JSON").clicked() {
                                match File::create(&io_state.filename) {
                                    Ok(mut file) => match serde_json::to_string_pretty(&*map) {
                                        Ok(json_str) => {
                                            if let Err(e) = file.write_all(json_str.as_bytes()) {
                                                io_state.status_message =
                                                    format!("Error writing file: {}", e);
                                            } else {
                                                io_state.status_message =
                                                    "Exported map successfully!".to_string();
                                            }
                                        }
                                        Err(e) => {
                                            io_state.status_message =
                                                format!("Error serialising: {}", e);
                                        }
                                    },
                                    Err(e) => {
                                        io_state.status_message =
                                            format!("Error creating file: {}", e);
                                    }
                                }
                            }

                            if ui.button("Import JSON").clicked() {
                                match File::open(&io_state.filename) {
                                    Ok(mut file) => {
                                        let mut contents = String::new();
                                        if let Err(e) = file.read_to_string(&mut contents) {
                                            io_state.status_message =
                                                format!("Error reading file: {}", e);
                                        } else {
                                            match serde_json::from_str::<TempestMap>(&contents) {
                                                Ok(imported_map) => {
                                                    // Despawn existing visual prefabs first
                                                    for (entity, _) in prefab_query.iter() {
                                                        commands.entity(entity).despawn();
                                                    }

                                                    let mut imported_map = imported_map;
                                                    for p in imported_map.prefabs.iter_mut() {
                                                        if p.rotation == [0.0, 0.0, 0.0, 1.0]
                                                            && let Some(ry) = p.rotation_y
                                                        {
                                                            let half = ry * 0.5;
                                                            p.rotation =
                                                                [0.0, half.sin(), 0.0, half.cos()];
                                                        }
                                                    }
                                                    *map = imported_map;
                                                    io_state.status_message =
                                                        "Imported map successfully!".to_string();

                                                    // Re-spawn loaded prefabs visually in the editor viewport!
                                                    for (idx, prefab) in
                                                        map.prefabs.iter().enumerate()
                                                    {
                                                        let pos = Vec3::from_array(prefab.position);
                                                        let rot = Quat::from_array(prefab.rotation);
                                                        let scale = Vec3::from_array(prefab.scale);
                                                        spawn_prefab_visuals(
                                                            &mut commands,
                                                            &mut meshes,
                                                            &mut materials,
                                                            &prefab.prefab_type,
                                                            pos,
                                                            rot,
                                                            scale,
                                                            prefab.texture_override.as_deref(),
                                                            &mansion_settings,
                                                            idx,
                                                            &asset_server,
                                                            prefab.custom_mesh.as_ref(),
                                                        );
                                                    }

                                                    for (entity, mesh_3d) in terrain_query.iter() {
                                                        rebuild_terrain_mesh(
                                                            entity,
                                                            &mut commands,
                                                            &map,
                                                            &splat_settings,
                                                            &mut meshes,
                                                            Some(mesh_3d),
                                                        );
                                                    }
                                                }
                                                Err(e) => {
                                                    io_state.status_message =
                                                        format!("Error deserialising: {}", e);
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        io_state.status_message =
                                            format!("Error opening file: {}", e);
                                    }
                                }
                            }
                        });
                    });
                ui.separator();

                // 8. Resize Map settings
                egui::CollapsingHeader::new("🔧 Resize Map Dimensions")
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.heading("Resize Map");
                        ui.label(format!("Current Size: {} x {}", map.width, map.height));
                        ui.add(
                            egui::Slider::new(&mut resize_settings.width, 32..=1200)
                                .step_by(16.0)
                                .text("New Width"),
                        );
                        ui.add(
                            egui::Slider::new(&mut resize_settings.height, 32..=1200)
                                .step_by(16.0)
                                .text("New Height"),
                        );
                        if ui.button("🔧 Resize Map & Rebuild").clicked() {
                            let new_w = resize_settings.width;
                            let new_h = resize_settings.height;
                            map.resize(new_w, new_h);

                            // Rebuild terrain mesh fully
                            for (entity, mesh_3d) in terrain_query.iter() {
                                if let Some(mut mesh) = meshes.get_mut(&mesh_3d.0) {
                                    update_terrain_mesh_in_place(&mut mesh, &map, &splat_settings);
                                } else {
                                    let new_mesh = generate_terrain_mesh(&map, &splat_settings);
                                    let new_handle = meshes.add(new_mesh);
                                    commands.entity(entity).insert(Mesh3d(new_handle));
                                }
                            }

                            // Rebuild water mesh fully & replace simulation component
                            for (water_entity, _) in water_query.iter() {
                                let new_handle = meshes.add(generate_water_mesh(new_w, new_h));
                                commands.entity(water_entity).insert(Mesh3d(new_handle));
                                commands
                                    .entity(water_entity)
                                    .insert(WaterSimData::new(new_w, new_h));
                            }
                            io_state.status_message =
                                format!("Resized map to {}x{} successfully!", new_w, new_h);
                        }

                        ui.separator();
                    });
                ui.separator();

                // 9. Keyboard Controls & Help
                egui::CollapsingHeader::new("❓ Keyboard Controls & Help")
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.label("Controls:");
                        ui.label("- Left Mouse Button to sculpt");
                        ui.label("- Hold Right Mouse Button + move to Orbit");
                        ui.label("- Scroll wheel to Zoom");
                        ui.label("- Hold Middle Mouse Button + move to Pan");
                        ui.separator();
                    });

                // Status message (if any)
                ui.separator();
                if !io_state.status_message.is_empty() {
                    ui.add(egui::Label::new(
                        egui::RichText::new(&io_state.status_message)
                            .color(egui::Color32::from_rgb(100, 255, 100))
                            .strong(),
                    ));
                }

                ui.separator();

                ui.separator();
                if ui.button("Back to Main Menu").clicked() {
                    next_state.set(AppState::MainMenu);
                }
            });
        });
}

fn camera_controller(
    mut query: Query<(&mut Transform, &mut EditorCamera)>,
    mut mouse_motion: MessageReader<bevy::input::mouse::MouseMotion>,
    mut mouse_wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let Ok((mut transform, mut cam)) = query.single_mut() else {
        return;
    };
    // Zoom
    let mut zoom = 0.0;
    for ev in mouse_wheel.read() {
        zoom += ev.y;
    }
    cam.radius = (cam.radius - zoom * 2.0).clamp(5.0, 200.0);

    // Orbit (Right Mouse Button)
    let mut rotation_move = Vec2::ZERO;
    if mouse_button.pressed(MouseButton::Right) {
        for ev in mouse_motion.read() {
            rotation_move += ev.delta;
        }
    } else {
        mouse_motion.clear();
    }

    cam.yaw -= rotation_move.x * 0.005;
    cam.pitch = (cam.pitch - rotation_move.y * 0.005).clamp(-1.4, 1.4);

    // Pan (Middle Mouse Button)
    let mut pan_move = Vec2::ZERO;
    if mouse_button.pressed(MouseButton::Middle) {
        for ev in mouse_motion.read() {
            pan_move += ev.delta;
        }
    }

    let rotation = Quat::from_euler(EulerRot::YXZ, cam.yaw, cam.pitch, 0.0);
    let forward = rotation * -Vec3::Z;
    let right = rotation * Vec3::X;
    let forward_xz = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
    let right_xz = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();

    // WASD keyboard panning
    let mut keyboard_move = Vec2::ZERO;
    if keyboard_input.pressed(KeyCode::KeyW) {
        keyboard_move.y += 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        keyboard_move.y -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyA) {
        keyboard_move.x -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        keyboard_move.x += 1.0;
    }

    if keyboard_move.length_squared() > 0.0 {
        let speed = 35.0 * time.delta_secs(); // WASD speed
        cam.orbit += right_xz * keyboard_move.x * speed;
        cam.orbit += forward_xz * keyboard_move.y * speed;
    }

    if pan_move.length_squared() > 0.0 {
        let radius = cam.radius;
        cam.orbit -= right_xz * pan_move.x * radius * 0.001;
        cam.orbit += forward_xz * pan_move.y * radius * 0.001;
    }

    let translation = cam.orbit + rotation * Vec3::new(0.0, 0.0, cam.radius);

    transform.translation = translation;
    transform.look_at(cam.orbit, Vec3::Y);
}

fn raycast_terrain(ray: &Ray3d, map: &TempestMap) -> Option<Vec3> {
    let dir = *ray.direction;
    let origin = ray.origin;

    if dir.y >= 0.0 {
        return None;
    }

    let w = map.width as f32;
    let h = map.height as f32;
    let offset_x = -w / 2.0;
    let offset_z = -h / 2.0;

    let mut t = 0.0;
    let step = 0.2;
    let max_dist = 300.0;

    while t < max_dist {
        let pos = origin + dir * t;
        let grid_x = pos.x - offset_x;
        let grid_z = pos.z - offset_z;

        if grid_x >= 0.0 && grid_x < w && grid_z >= 0.0 && grid_z < h {
            let map_x = grid_x as u32;
            let map_z = grid_z as u32;
            let terrain_y = map.get_height(map_x, map_z);

            if pos.y <= terrain_y {
                return Some(pos);
            }
        }
        t += step;
    }
    None
}

#[allow(dead_code)]
fn project_ray_onto_axis(
    ray_origin: Vec3,
    ray_dir: Vec3,
    axis_origin: Vec3,
    axis_dir: Vec3,
) -> Vec3 {
    let diff = ray_origin - axis_origin;
    let b = axis_dir.dot(ray_dir);
    let d = axis_dir.dot(diff);
    let e = ray_dir.dot(diff);
    let denom = 1.0 - b * b;
    let s = if denom.abs() > 1e-5 {
        (b * e - d) / denom
    } else {
        0.0
    };
    axis_origin + axis_dir * s
}

fn get_building_sockets(prefab_type: &str, pos: Vec3, rot: Quat) -> Vec<(Vec3, Vec3)> {
    let local_sockets = match prefab_type {
        "floor_tile" => vec![
            (Vec3::new(0.0, 0.0, -2.0), Vec3::new(0.0, 0.0, -1.0)), // North
            (Vec3::new(0.0, 0.0, 2.0), Vec3::new(0.0, 0.0, 1.0)),   // South
            (Vec3::new(2.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)),   // East
            (Vec3::new(-2.0, 0.0, 0.0), Vec3::new(-1.0, 0.0, 0.0)), // West
        ],
        "ceiling_tile" => vec![
            (Vec3::new(0.0, 0.0, -2.0), Vec3::new(0.0, 0.0, -1.0)), // North
            (Vec3::new(0.0, 0.0, 2.0), Vec3::new(0.0, 0.0, 1.0)),   // South
            (Vec3::new(2.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)),   // East
            (Vec3::new(-2.0, 0.0, 0.0), Vec3::new(-1.0, 0.0, 0.0)), // West
        ],
        "wall_straight" | "door_frame" | "window_frame" => vec![
            (Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0)), // Bottom
            (Vec3::new(0.0, 3.5, 0.0), Vec3::new(0.0, 1.0, 0.0)),  // Top
            (Vec3::new(-2.0, 1.75, 0.0), Vec3::new(-1.0, 0.0, 0.0)), // Left
            (Vec3::new(2.0, 1.75, 0.0), Vec3::new(1.0, 0.0, 0.0)), // Right
        ],
        "wall_corner" => vec![
            (Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0)), // Bottom
            (Vec3::new(0.0, 3.5, 0.0), Vec3::new(0.0, 1.0, 0.0)),  // Top
            (Vec3::new(2.0, 1.75, -0.1), Vec3::new(1.0, 0.0, 0.0)), // Right edge
            (Vec3::new(-0.1, 1.75, 2.0), Vec3::new(0.0, 0.0, 1.0)), // Forward edge
        ],
        "wall_t_junction" => vec![
            (Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0)), // Bottom
            (Vec3::new(0.0, 3.5, 0.0), Vec3::new(0.0, 1.0, 0.0)),  // Top
            (Vec3::new(-2.0, 1.75, 0.0), Vec3::new(-1.0, 0.0, 0.0)), // Left
            (Vec3::new(2.0, 1.75, 0.0), Vec3::new(1.0, 0.0, 0.0)), // Right
            (Vec3::new(0.0, 1.75, 2.0), Vec3::new(0.0, 0.0, 1.0)), // T branch forward
        ],
        "wall_cross" => vec![
            (Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0)), // Bottom
            (Vec3::new(0.0, 3.5, 0.0), Vec3::new(0.0, 1.0, 0.0)),  // Top
            (Vec3::new(-2.0, 1.75, 0.0), Vec3::new(-1.0, 0.0, 0.0)), // Left
            (Vec3::new(2.0, 1.75, 0.0), Vec3::new(1.0, 0.0, 0.0)), // Right
            (Vec3::new(0.0, 1.75, 2.0), Vec3::new(0.0, 0.0, 1.0)), // Forward
            (Vec3::new(0.0, 1.75, -2.0), Vec3::new(0.0, 0.0, -1.0)), // Back
        ],
        "hallway_segment" => vec![
            (Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0)), // Bottom
            (Vec3::new(0.0, 3.5, 0.0), Vec3::new(0.0, 1.0, 0.0)),  // Top
            (Vec3::new(0.0, 0.0, -4.0), Vec3::new(0.0, 0.0, -1.0)), // Back
            (Vec3::new(0.0, 0.0, 4.0), Vec3::new(0.0, 0.0, 1.0)),  // Front
        ],
        "room_pillar" => vec![
            (Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0)), // Bottom
            (Vec3::new(0.0, 3.5, 0.0), Vec3::new(0.0, 1.0, 0.0)),  // Top
        ],
        "roof_tile" => vec![
            (Vec3::new(0.0, 0.05, 1.64), Vec3::new(0.0, -0.5, 0.866)), // Bottom edge
            (Vec3::new(0.0, 2.35, -1.64), Vec3::new(0.0, 0.5, -0.866)), // Top edge
        ],
        "roof_gable" => vec![
            (Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0)), // Bottom
        ],
        _ => vec![],
    };

    local_sockets
        .into_iter()
        .map(|(p, n)| (pos + rot * p, rot * n))
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn calculate_snap(
    prefab_type: &str,
    cursor_pos: Vec3,
    ray_origin: Vec3,
    ray_dir: Vec3,
    map: &TempestMap,
    snap_grid: bool,
    grid_size: f32,
    snap_objects: bool,
) -> (Vec3, Quat) {
    let default_rot = Quat::IDENTITY;

    if snap_objects {
        let mut best_dist = f32::MAX;
        let mut best_pos = None;
        let mut best_rot = None;

        for other in map.prefabs.iter() {
            let other_pos = Vec3::from_array(other.position);
            let other_rot = Quat::from_array(other.rotation);

            let camera_dist = other_pos.distance(ray_origin);
            if camera_dist > 150.0 {
                continue;
            }
            let threshold = (camera_dist * 0.06).clamp(1.5, 4.0);

            let other_sockets = get_building_sockets(&other.prefab_type, other_pos, other_rot);
            for (w_pos, w_norm) in other_sockets {
                // Calculate distance from the camera ray to w_pos
                let v = w_pos - ray_origin;
                let proj = v.dot(ray_dir);
                if proj > 0.0 {
                    let closest_point_on_ray = ray_origin + ray_dir * proj;
                    let dist = closest_point_on_ray.distance(w_pos);
                    if dist < threshold && dist < best_dist {
                        best_dist = dist;
                        match prefab_type {
                            "floor_tile" | "ceiling_tile" | "hallway_segment" => {
                                if other.prefab_type == "floor_tile"
                                    || other.prefab_type == "ceiling_tile"
                                    || other.prefab_type == "hallway_segment"
                                {
                                    let half_dim = if prefab_type == "hallway_segment" {
                                        4.0
                                    } else {
                                        2.0
                                    };
                                    let mut target_pos = w_pos + w_norm * half_dim;

                                    // Height lock: align target height exactly with parent tile
                                    if prefab_type == "ceiling_tile"
                                        && other.prefab_type == "floor_tile"
                                    {
                                        target_pos.y = other_pos.y + 3.5;
                                    } else if prefab_type == "floor_tile"
                                        && other.prefab_type == "ceiling_tile"
                                    {
                                        target_pos.y = other_pos.y - 3.5;
                                    } else {
                                        target_pos.y = other_pos.y;
                                    }
                                    best_pos = Some(target_pos);
                                } else if (other.prefab_type == "wall_straight"
                                    || other.prefab_type == "door_frame"
                                    || other.prefab_type == "window_frame"
                                    || other.prefab_type == "wall_corner"
                                    || other.prefab_type == "wall_t_junction"
                                    || other.prefab_type == "wall_cross")
                                    && w_norm.y.abs() > 0.9
                                {
                                    // Snapping to the top/bottom of a wall.
                                    // Offset by half-dimension along the wall's local Z-axis towards the cursor.
                                    let local_cursor = other_rot.inverse() * (cursor_pos - w_pos);
                                    let half_dim = if prefab_type == "hallway_segment" {
                                        4.0
                                    } else {
                                        2.0
                                    };
                                    let z_offset = if local_cursor.z >= 0.0 {
                                        half_dim
                                    } else {
                                        -half_dim
                                    };
                                    let offset_vec = other_rot * Vec3::new(0.0, 0.0, z_offset);
                                    let mut target_pos = w_pos + offset_vec;

                                    // Align Y with the socket Y height exactly
                                    target_pos.y = w_pos.y;
                                    best_pos = Some(target_pos);
                                } else {
                                    best_pos = Some(w_pos);
                                }
                                best_rot = Some(other_rot);
                            }
                            "wall_straight" | "door_frame" | "window_frame" | "wall_corner"
                            | "wall_t_junction" | "wall_cross" | "room_pillar" => {
                                best_pos = Some(w_pos);
                                if other.prefab_type == "floor_tile"
                                    || other.prefab_type == "ceiling_tile"
                                {
                                    let forward = w_norm.normalize();
                                    best_rot = Some(Quat::from_rotation_arc(Vec3::Z, forward));
                                } else {
                                    best_rot = Some(other_rot);
                                }
                            }
                            "roof_tile" => {
                                if other.prefab_type == "roof_tile" {
                                    // Check local Z of normal to determine top/bottom snap
                                    let local_norm = other_rot.inverse() * w_norm;
                                    if local_norm.z < 0.0 {
                                        // Snap bottom of new roof to top of old roof
                                        best_pos =
                                            Some(w_pos - other_rot * Vec3::new(0.0, 0.05, 1.64));
                                    } else {
                                        // Snap top of new roof to bottom of old roof
                                        best_pos =
                                            Some(w_pos - other_rot * Vec3::new(0.0, 2.35, -1.64));
                                    }
                                } else if other.prefab_type == "wall_straight"
                                    || other.prefab_type == "door_frame"
                                    || other.prefab_type == "window_frame"
                                    || other.prefab_type == "wall_corner"
                                    || other.prefab_type == "wall_t_junction"
                                    || other.prefab_type == "wall_cross"
                                {
                                    let local_norm = other_rot.inverse() * w_norm;
                                    if local_norm.y > 0.0 {
                                        // Snap bottom of roof to top of wall
                                        best_pos =
                                            Some(w_pos - other_rot * Vec3::new(0.0, 0.05, 1.64));
                                    } else {
                                        best_pos = Some(w_pos);
                                    }
                                } else {
                                    best_pos = Some(w_pos);
                                }
                                best_rot = Some(other_rot);
                            }
                            "roof_gable" => {
                                best_pos = Some(w_pos);
                                best_rot = Some(other_rot);
                            }
                            _ => {
                                best_pos = Some(w_pos);
                                best_rot = Some(other_rot);
                            }
                        }
                    }
                }
            }
        }

        if let (Some(pos), Some(rot)) = (best_pos, best_rot) {
            return (pos, rot);
        }
    }

    if snap_grid {
        let snapped_x = (cursor_pos.x / grid_size).round() * grid_size;
        let snapped_z = (cursor_pos.z / grid_size).round() * grid_size;
        let mut base_y = get_bilinear_height(snapped_x, snapped_z, map);
        for p in &map.prefabs {
            let p_pos = Vec3::from_array(p.position);
            let dx = (snapped_x - p_pos.x).abs();
            let dz = (snapped_z - p_pos.z).abs();
            let (w, d) = if p.prefab_type == "hallway_segment" {
                (2.0f32, 4.0f32)
            } else if p.prefab_type == "floor_tile" || p.prefab_type == "ceiling_tile" {
                (2.0f32, 2.0f32)
            } else if p.prefab_type == "wall_straight"
                || p.prefab_type == "door_frame"
                || p.prefab_type == "window_frame"
            {
                (2.0f32, 0.2f32)
            } else if p.prefab_type == "wall_corner"
                || p.prefab_type == "wall_t_junction"
                || p.prefab_type == "wall_cross"
            {
                (2.0f32, 2.0f32)
            } else if p.prefab_type == "room_pillar" {
                (0.5f32, 0.5f32)
            } else {
                (0.0f32, 0.0f32)
            };
            if w > 0.0 && dx < w && dz < d {
                let top_y = if p.prefab_type == "floor_tile" {
                    p_pos.y + 0.0
                } else if p.prefab_type == "ceiling_tile"
                    || p.prefab_type == "hallway_segment"
                    || p.prefab_type == "wall_straight"
                    || p.prefab_type == "door_frame"
                    || p.prefab_type == "window_frame"
                    || p.prefab_type == "wall_corner"
                    || p.prefab_type == "wall_t_junction"
                    || p.prefab_type == "wall_cross"
                    || p.prefab_type == "room_pillar"
                {
                    p_pos.y + 3.5
                } else {
                    p_pos.y
                };
                if top_y > base_y {
                    base_y = top_y;
                }
            }
        }
        let mut snapped_y = (base_y / 0.5).round() * 0.5;
        if (prefab_type == "ceiling_tile" || prefab_type == "roof_gable") && snapped_y < 3.0 {
            snapped_y = 3.5;
        }
        return (Vec3::new(snapped_x, snapped_y, snapped_z), default_rot);
    }

    (cursor_pos, default_rot)
}

#[allow(clippy::too_many_arguments)]
fn respawn_selected_prefab_mesh(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    selection_state: &SelectionState,
    map: &TempestMap,
    mansion_settings: &crate::play_mode::house::MansionSettings,
    asset_server: &AssetServer,
    prefab_query: &Query<(Entity, &mut PlacedPrefabMarker)>,
    children_query: &Query<&Children>,
) {
    let Some(sel_idx) = selection_state.selected_idx else {
        return;
    };
    if sel_idx >= map.prefabs.len() {
        return;
    }
    let prefab = &map.prefabs[sel_idx];

    for (entity, marker) in prefab_query.iter() {
        if marker.index == sel_idx {
            if let Ok(children) = children_query.get(entity) {
                for child in children.iter() {
                    commands.entity(child).despawn();
                }
            }
            spawn_prefab_visuals_children(
                commands,
                meshes,
                materials,
                &prefab.prefab_type,
                Vec3::from_array(prefab.position),
                prefab.texture_override.as_deref(),
                mansion_settings,
                entity,
                asset_server,
                prefab.custom_mesh.as_ref(),
            );
            break;
        }
    }
}

fn reindex_prefab_markers(
    prefabs: &[PlacedPrefab],
    query: &mut Query<(Entity, &mut PlacedPrefabMarker)>,
) {
    for (_, mut marker) in query.iter_mut() {
        let mut best_idx = None;
        let mut min_dist = 0.1f32;
        for (i, p) in prefabs.iter().enumerate() {
            if p.prefab_type == marker.prefab_type {
                let p_pos = Vec3::from_array(p.position);
                let dist = marker.position.distance(p_pos);
                if dist < min_dist {
                    min_dist = dist;
                    best_idx = Some(i);
                }
            }
        }
        if let Some(idx) = best_idx {
            marker.index = idx;
        }
    }
}

fn despawn_preview_entity(
    commands: &mut Commands,
    selection_state: &mut SelectionState,
    children_query: &Query<&Children>,
) {
    if let Some(prev_ent) = selection_state.preview_entity {
        if let Ok(children) = children_query.get(prev_ent) {
            for child in children.iter() {
                commands.entity(child).despawn();
            }
        }
        commands.entity(prev_ent).despawn();
        selection_state.preview_entity = None;
        selection_state.preview_tool = None;
    }
}

fn apply_preview_material_recursive(
    commands: &mut Commands,
    entity: Entity,
    preview_mat: Handle<StandardMaterial>,
    children_query: &Query<&Children>,
) {
    commands
        .entity(entity)
        .insert(MeshMaterial3d(preview_mat.clone()))
        .remove::<PointLight>();
    if let Ok(children) = children_query.get(entity) {
        for child in children.iter() {
            apply_preview_material_recursive(commands, child, preview_mat.clone(), children_query);
        }
    }
}

#[derive(bevy::ecs::system::SystemParam)]
struct TerrainSculptParams<'w, 's> {
    commands: Commands<'w, 's>,
    map: ResMut<'w, TempestMap>,
    settings: Res<'w, SplatmapSettings>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    mansion_settings: Res<'w, crate::play_mode::house::MansionSettings>,
    selection_state: ResMut<'w, SelectionState>,
    asset_server: Res<'w, AssetServer>,
    custom_assets: Res<'w, CustomAssetLibrary>,
}

#[allow(
    clippy::too_many_arguments,
    clippy::collapsible_if,
    clippy::type_complexity,
    clippy::needless_range_loop
)]
fn terrain_sculpting_system(
    params: TerrainSculptParams,
    camera_query: Query<(&Camera, &GlobalTransform), With<EditorCamera>>,
    window_query: Query<&Window>,
    mut contexts: EguiContexts,
    terrain_query: Query<(Entity, &Mesh3d), With<TerrainMesh>>,
    brush: ResMut<BrushSettings>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut gizmos: Gizmos,
    mut impulse_writer: MessageWriter<WaterImpulseEvent>,
    mut grass_writer: MessageWriter<crate::grass::GenerateGrassEvent>,
    mut prefab_query: Query<(Entity, &mut PlacedPrefabMarker)>,
    children_query: Query<&Children>,
    mut preview_transform_query: Query<
        &mut Transform,
        (
            Without<EditorCamera>,
            Without<TerrainMesh>,
            Without<WaterMesh>,
        ),
    >,
) {
    let TerrainSculptParams {
        mut commands,
        mut map,
        settings,
        mut meshes,
        mut materials,
        mansion_settings,
        mut selection_state,
        asset_server,
        custom_assets,
    } = params;

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    if ctx.egui_wants_pointer_input() || ctx.is_pointer_over_egui() {
        despawn_preview_entity(&mut commands, &mut selection_state, &children_query);
        return;
    }

    let Ok(window) = window_query.single() else {
        despawn_preview_entity(&mut commands, &mut selection_state, &children_query);
        return;
    };
    let Ok((camera, camera_transform)) = camera_query.single() else {
        despawn_preview_entity(&mut commands, &mut selection_state, &children_query);
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        despawn_preview_entity(&mut commands, &mut selection_state, &children_query);
        return;
    };
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) else {
        despawn_preview_entity(&mut commands, &mut selection_state, &children_query);
        return;
    };

    if mouse_button.just_released(MouseButton::Left) {
        selection_state.active_drag_axis = None;
    }

    if selection_state.active_drag_axis.is_some() && mouse_button.pressed(MouseButton::Left) {
        let i = selection_state.active_drag_axis.unwrap();
        if let Some(sel_idx) = selection_state.selected_idx {
            if sel_idx < map.prefabs.len() {
                let prefab = &map.prefabs[sel_idx];
                let pos = Vec3::from_array(prefab.position);
                let rot = Quat::from_array(prefab.rotation);

                let axes = [Vec3::X, Vec3::Y, Vec3::Z];
                let axis_dir = rot * axes[i];
                let ray_dir = *ray.direction;

                let axis_proj = project_ray_onto_axis(ray.origin, ray_dir, pos, axis_dir);
                let current_mouse_proj = (axis_proj - pos).dot(axis_dir);
                let delta = current_mouse_proj - selection_state.drag_start_mouse_proj;

                if selection_state.drag_scale {
                    let mut new_scale = selection_state.drag_start_value;
                    let start_val = selection_state.drag_start_value[i];
                    let ratio = if selection_state.drag_start_mouse_proj.abs() > 0.05 {
                        current_mouse_proj / selection_state.drag_start_mouse_proj
                    } else {
                        1.0
                    };
                    let mut axis_scale = start_val * ratio;
                    if selection_state.snap_to_grid {
                        let snap = selection_state.snap_grid_size;
                        axis_scale = (axis_scale / snap).round() * snap;
                    }
                    new_scale[i] = axis_scale.max(0.1);
                    map.prefabs[sel_idx].scale = new_scale.to_array();
                } else {
                    let mut new_pos = selection_state.drag_start_value + axis_dir * delta;
                    if selection_state.snap_to_grid {
                        let snap = selection_state.snap_grid_size;
                        let start_pos = selection_state.drag_start_value;
                        let local_delta = (new_pos - start_pos).dot(axis_dir);
                        let snapped_delta = (local_delta / snap).round() * snap;
                        new_pos = start_pos + axis_dir * snapped_delta;
                    }
                    map.prefabs[sel_idx].position = new_pos.to_array();
                }
            }
        }

        despawn_preview_entity(&mut commands, &mut selection_state, &children_query);
        draw_gizmo_handles(&mut gizmos, &selection_state, &map);
        return;
    }

    let Some(intersection) = raycast_terrain(&ray, &map) else {
        despawn_preview_entity(&mut commands, &mut selection_state, &children_query);
        return;
    };

    gizmos.circle(
        Isometry3d::new(
            intersection + Vec3::Y * 0.05,
            Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
        ),
        brush.size,
        Color::srgb(0.0, 1.0, 1.0),
    );

    // --- Update Ghost Preview Outline ---
    let is_placement = matches!(
        brush.tool,
        SculptTool::PlaceTreeOak
            | SculptTool::PlaceTreePine
            | SculptTool::PlaceTreeBirch
            | SculptTool::PlaceShrub
            | SculptTool::PlaceCactus
            | SculptTool::PlaceRock
            | SculptTool::PlaceCaveEntrance
            | SculptTool::PlaceSpawnPoint
            | SculptTool::PlaceHouse
            | SculptTool::PlaceOreCopper
            | SculptTool::PlaceOreIron
            | SculptTool::PlaceOreGold
            | SculptTool::PlaceOreSilver
            | SculptTool::PlaceOrePlatinum
            | SculptTool::PlaceOreSteel
            | SculptTool::PlaceOreGranite
            | SculptTool::PlaceModularWall
            | SculptTool::PlaceModularCorner
            | SculptTool::PlaceModularFloor
            | SculptTool::PlaceModularRoof
            | SculptTool::PlaceModularRoofGable
            | SculptTool::PlaceModularDoorFrame
            | SculptTool::PlaceModularWindowFrame
            | SculptTool::PlaceWallTJunction
            | SculptTool::PlaceWallCross
            | SculptTool::PlaceCeilingTile
            | SculptTool::PlaceFluorescentLight
            | SculptTool::PlaceHallwaySegment
            | SculptTool::PlaceRoomPillar
            | SculptTool::PlaceChest
            | SculptTool::PlaceWorkbench
            | SculptTool::PlaceFurnace
            | SculptTool::PlaceBed
            | SculptTool::PlaceTorch
            | SculptTool::PlaceChair
            | SculptTool::PlaceDesk
            | SculptTool::PlaceHealthPack
            | SculptTool::PlaceCrate
            | SculptTool::PlaceCustomAsset
            | SculptTool::PlaceCustomMesh
    );

    if is_placement {
        let prefab_type = match brush.tool {
            SculptTool::PlaceTreeOak => "tree_oak",
            SculptTool::PlaceTreePine => "tree_pine",
            SculptTool::PlaceTreeBirch => "tree_birch",
            SculptTool::PlaceShrub => "shrub",
            SculptTool::PlaceCactus => "cactus",
            SculptTool::PlaceRock => "rock",
            SculptTool::PlaceCaveEntrance => "cave_entrance",
            SculptTool::PlaceSpawnPoint => "spawn_point",
            SculptTool::PlaceHouse => "house",
            SculptTool::PlaceOreCopper => "ore_copper",
            SculptTool::PlaceOreIron => "ore_iron",
            SculptTool::PlaceOreGold => "ore_gold",
            SculptTool::PlaceOreSilver => "ore_silver",
            SculptTool::PlaceOrePlatinum => "ore_platinum",
            SculptTool::PlaceOreSteel => "ore_steel",
            SculptTool::PlaceOreGranite => "ore_granite",
            SculptTool::PlaceModularWall => "wall_straight",
            SculptTool::PlaceModularCorner => "wall_corner",
            SculptTool::PlaceModularFloor => "floor_tile",
            SculptTool::PlaceModularRoof => "roof_tile",
            SculptTool::PlaceModularRoofGable => "roof_gable",
            SculptTool::PlaceModularDoorFrame => "door_frame",
            SculptTool::PlaceModularWindowFrame => "window_frame",
            SculptTool::PlaceChest => "chest",
            SculptTool::PlaceWorkbench => "workbench",
            SculptTool::PlaceFurnace => "furnace",
            SculptTool::PlaceBed => "bed",
            SculptTool::PlaceTorch => "torch",
            SculptTool::PlaceChair => "prop_chair",
            SculptTool::PlaceDesk => "prop_desk",
            SculptTool::PlaceHealthPack => "prop_health_pack",
            SculptTool::PlaceCrate => "prop_crate",
            SculptTool::PlaceWallTJunction => "wall_t_junction",
            SculptTool::PlaceWallCross => "wall_cross",
            SculptTool::PlaceCeilingTile => "ceiling_tile",
            SculptTool::PlaceFluorescentLight => "fluorescent_light",
            SculptTool::PlaceHallwaySegment => "hallway_segment",
            SculptTool::PlaceRoomPillar => "room_pillar",
            SculptTool::PlaceCustomAsset => "custom_asset",
            SculptTool::PlaceCustomMesh => "custom_mesh",
            _ => unreachable!(),
        };

        let is_modular = matches!(
            prefab_type,
            "wall_straight"
                | "wall_corner"
                | "floor_tile"
                | "roof_tile"
                | "roof_gable"
                | "door_frame"
                | "window_frame"
                | "wall_t_junction"
                | "wall_cross"
                | "ceiling_tile"
                | "hallway_segment"
        );

        // Listen to R key for prefab rotation
        if keyboard_input.just_pressed(KeyCode::KeyR) {
            selection_state.placement_rotation_angle = (selection_state.placement_rotation_angle
                + std::f32::consts::FRAC_PI_2)
                % std::f32::consts::TAU;
        }

        // Listen to Arrow keys for smooth rotation
        if keyboard_input.pressed(KeyCode::ArrowLeft) {
            selection_state.placement_rotation_angle = (selection_state.placement_rotation_angle
                + 1.8 * time.delta_secs())
                % std::f32::consts::TAU;
        }
        if keyboard_input.pressed(KeyCode::ArrowRight) {
            selection_state.placement_rotation_angle = (selection_state.placement_rotation_angle
                - 1.8 * time.delta_secs())
                % std::f32::consts::TAU;
        }

        // Listen to F key to flip/mirror placing prefab
        if keyboard_input.just_pressed(KeyCode::KeyF) {
            selection_state.placement_flipped = !selection_state.placement_flipped;
        }

        let placement_scale = if selection_state.placement_flipped {
            Vec3::new(-1.0, 1.0, 1.0)
        } else {
            Vec3::ONE
        };

        let (place_pos, rotation, scale) = if is_modular {
            let (snapped_pos, snapped_rot) = calculate_snap(
                prefab_type,
                intersection,
                ray.origin,
                *ray.direction,
                &map,
                selection_state.snap_to_grid,
                selection_state.snap_grid_size,
                selection_state.snap_to_objects,
            );
            let final_rot =
                snapped_rot * Quat::from_rotation_y(selection_state.placement_rotation_angle);
            (snapped_pos, final_rot, placement_scale)
        } else {
            let is_prop = matches!(
                prefab_type,
                "chest"
                    | "workbench"
                    | "furnace"
                    | "bed"
                    | "torch"
                    | "fluorescent_light"
                    | "prop_chair"
                    | "prop_desk"
                    | "prop_health_pack"
                    | "prop_crate"
            );
            let rot_y = if prefab_type == "house"
                || prefab_type == "custom_mesh"
                || prefab_type == "custom_asset"
                || is_prop
            {
                0.0
            } else {
                let seed = (intersection.x * 12.9898 + intersection.z * 78.233).sin() * 43758.547;
                seed.fract() * std::f32::consts::TAU
            };
            let final_rot = Quat::from_rotation_y(rot_y + selection_state.placement_rotation_angle);
            let mut final_pos = intersection;
            if prefab_type == "custom_mesh" {
                final_pos.y += 1.0;
            }
            (final_pos, final_rot, placement_scale)
        };

        let tex_override: Option<&str> = if prefab_type == "custom_asset" {
            custom_assets
                .selected_asset_idx
                .and_then(|i| custom_assets.assets.get(i))
                .map(|entry| entry.file_path.as_str())
        } else {
            None
        };

        let mut spawn_new = false;
        if let Some(_prev_ent) = selection_state.preview_entity {
            if selection_state.preview_tool != Some(brush.tool) {
                despawn_preview_entity(&mut commands, &mut selection_state, &children_query);
                selection_state.placement_rotation_angle = 0.0;
                selection_state.placement_flipped = false;
                spawn_new = true;
            }
        } else {
            spawn_new = true;
        }

        if spawn_new {
            let preview_mat = materials.add(StandardMaterial {
                base_color: Color::srgba(0.2, 0.8, 0.3, 0.45),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            });

            let parent = commands
                .spawn((
                    Transform::from_translation(place_pos)
                        .with_rotation(rotation)
                        .with_scale(scale),
                    Visibility::Visible,
                    InheritedVisibility::default(),
                    MapEditorEntity,
                ))
                .id();

            let preview_custom_mesh = if prefab_type == "custom_mesh" {
                match selection_state.custom_mesh_primitive {
                    CustomMeshPrimitive::Cube => Some(EditableMesh::new_cube(2.0)),
                    CustomMeshPrimitive::Sphere => Some(EditableMesh::new_sphere(1.0)),
                }
            } else {
                None
            };

            spawn_prefab_visuals_children(
                &mut commands,
                &mut meshes,
                &mut materials,
                prefab_type,
                place_pos,
                tex_override,
                &mansion_settings,
                parent,
                &asset_server,
                preview_custom_mesh.as_ref(),
            );

            apply_preview_material_recursive(&mut commands, parent, preview_mat, &children_query);

            selection_state.preview_entity = Some(parent);
            selection_state.preview_tool = Some(brush.tool);
        } else if let Some(prev_ent) = selection_state.preview_entity {
            if let Ok(mut trans) = preview_transform_query.get_mut(prev_ent) {
                trans.translation = place_pos;
                trans.rotation = rotation;
                trans.scale = scale;
            }
        }
    } else {
        despawn_preview_entity(&mut commands, &mut selection_state, &children_query);
    }

    let is_sculpt_tool = matches!(
        brush.tool,
        SculptTool::Raise
            | SculptTool::Lower
            | SculptTool::Smooth
            | SculptTool::Disturb
            | SculptTool::Rocky
    );
    if is_sculpt_tool && mouse_button.pressed(MouseButton::Left) {
        let w = map.width;
        let h = map.height;
        let offset_x = -(w as f32) / 2.0;
        let offset_z = -(h as f32) / 2.0;

        let radius_sq = brush.size * brush.size;
        let dt = time.delta_secs();

        let min_x = ((intersection.x - brush.size - offset_x).floor() as i32).max(0) as u32;
        let max_x =
            ((intersection.x + brush.size - offset_x).ceil() as i32).min(w as i32 - 1) as u32;
        let min_z = ((intersection.z - brush.size - offset_z).floor() as i32).max(0) as u32;
        let max_z =
            ((intersection.z + brush.size - offset_z).ceil() as i32).min(h as i32 - 1) as u32;

        let mut modified = false;

        if brush.tool == SculptTool::Disturb {
            // Simply trigger a wave impulse at the cursor intersection
            impulse_writer.write(WaterImpulseEvent {
                position: intersection,
                force: brush.strength * 0.25 * dt,
                radius: brush.size,
            });
        } else {
            // Normal sculpting tools: Raise, Lower, Smooth
            for z in min_z..=max_z {
                for x in min_x..=max_x {
                    let vx = x as f32 + offset_x;
                    let vz = z as f32 + offset_z;
                    let dx = vx - intersection.x;
                    let dz = vz - intersection.z;
                    let dist_sq = dx * dx + dz * dz;

                    if dist_sq <= radius_sq {
                        let dist = dist_sq.sqrt();
                        let falloff = (1.0 - dist / brush.size).clamp(0.0, 1.0);
                        let current_height = map.get_height(x, z);

                        match brush.tool {
                            SculptTool::Raise => {
                                let delta = brush.strength * falloff * dt;
                                map.set_height(x, z, current_height + delta);
                                modified = true;
                            }
                            SculptTool::Lower => {
                                let delta = brush.strength * falloff * dt;
                                map.set_height(x, z, current_height - delta);
                                modified = true;
                            }
                            SculptTool::Smooth => {
                                let mut sum = 0.0;
                                let mut count = 0;
                                for nz in (z as i32 - 1)..=(z as i32 + 1) {
                                    for nx in (x as i32 - 1)..=(x as i32 + 1) {
                                        if nx >= 0 && nx < w as i32 && nz >= 0 && nz < h as i32 {
                                            sum += map.get_height(nx as u32, nz as u32);
                                            count += 1;
                                        }
                                    }
                                }
                                let target = sum / count as f32;
                                let delta = (target - current_height) * brush.strength * 0.1 * dt;
                                map.set_height(x, z, current_height + delta);
                                modified = true;
                            }
                            SculptTool::Rocky => {
                                // Stratified/terraced rocky noise: combines high-frequency jagged noise with step-quantized terraces
                                let noise_val = (vx * 1.5).sin() * (vz * 1.5).cos() * 0.6
                                    + (vx * 5.0).cos() * (vz * 5.0).sin() * 0.15;
                                let raw_delta = brush.strength * noise_val * falloff * dt * 5.0;
                                let target_height = current_height + raw_delta;

                                // Quantize to steps of 0.75 meters to create beautiful horizontal geological strata/shelves!
                                let step_size = 0.75;
                                let terraced_height =
                                    (target_height / step_size).round() * step_size;
                                let blended_height = target_height.lerp(terraced_height, 0.45); // blend 45% terraced shelves, 55% raw rugged cuts

                                map.set_height(x, z, blended_height);
                                modified = true;
                            }
                            _ => {}
                        }
                    }
                }
            }

            if modified {
                for (_, mesh_handle) in terrain_query.iter() {
                    if let Some(mut mesh) = meshes.get_mut(&mesh_handle.0) {
                        update_terrain_mesh_in_place(&mut mesh, &map, &settings);
                    }
                }

                // Check prefabs within brush radius: update ground height or despawn if lowered into water/beach
                let w = map.width;
                let h = map.height;
                let offset_x = -(w as f32) / 2.0;
                let offset_z = -(h as f32) / 2.0;

                let mut to_remove_indices = Vec::new();

                for idx in 0..map.prefabs.len() {
                    let px = map.prefabs[idx].position[0];
                    let pz = map.prefabs[idx].position[2];

                    let dx = px - intersection.x;
                    let dz = pz - intersection.z;

                    // If prefab is inside sculpt brush region
                    if dx * dx + dz * dz <= radius_sq {
                        let hx = ((px - offset_x).round() as i32).clamp(0, w as i32 - 1) as u32;
                        let hz = ((pz - offset_z).round() as i32).clamp(0, h as i32 - 1) as u32;
                        let new_h = map.get_height(hx, hz);

                        // If ground is lowered into sand/water, schedule for deletion
                        if new_h < settings.sand_height + 0.3 {
                            to_remove_indices.push(idx);
                        } else {
                            // Update prefab stored height to follow terrain
                            map.prefabs[idx].position[1] = new_h;

                            // Update active entity visual position & marker
                            for (entity, mut marker) in prefab_query.iter_mut() {
                                if (marker.position.x - px).abs() < 0.2
                                    && (marker.position.z - pz).abs() < 0.2
                                {
                                    marker.position.y = new_h;
                                    if let Ok(mut trans) = preview_transform_query.get_mut(entity) {
                                        trans.translation.y = new_h;
                                    }
                                }
                            }
                        }
                    }
                }

                if !to_remove_indices.is_empty() {
                    to_remove_indices.sort_by(|a, b| b.cmp(a));
                    for &idx in to_remove_indices.iter() {
                        let removed = map.prefabs.remove(idx);
                        let removed_pos = Vec3::from_array(removed.position);
                        for (entity, marker) in prefab_query.iter() {
                            if (marker.position.x - removed_pos.x).abs() < 0.2
                                && (marker.position.z - removed_pos.z).abs() < 0.2
                            {
                                commands.entity(entity).despawn();
                            }
                        }
                    }
                }

                // Add a gentle ripple to the water as you reshape shorelines
                impulse_writer.write(WaterImpulseEvent {
                    position: intersection,
                    force: brush.strength * 0.04 * dt,
                    radius: brush.size * 0.6,
                });
            }
        }

        if mouse_button.just_released(MouseButton::Left) {
            // Regenerate grass once when sculpting stroke finishes
            grass_writer.write(crate::grass::GenerateGrassEvent);
        }
    } else if !is_sculpt_tool && mouse_button.just_pressed(MouseButton::Left) {
        let mut handle_clicked = false;
        if brush.tool == SculptTool::SelectObject {
            if let Some(sel_idx) = selection_state.selected_idx {
                if sel_idx < map.prefabs.len() {
                    let prefab = &map.prefabs[sel_idx];
                    let pos = Vec3::from_array(prefab.position);
                    let rot = Quat::from_array(prefab.rotation);
                    let scale = Vec3::from_array(prefab.scale);

                    let axes = [Vec3::X, Vec3::Y, Vec3::Z];
                    let ray_dir = *ray.direction;

                    for i in 0..3 {
                        let axis_dir = rot * axes[i];
                        let axis_len = 2.0;
                        let scale_handle_pos = pos + axis_dir * axis_len;

                        // 1. Scale handle detection
                        let ray_to_handle = scale_handle_pos - ray.origin;
                        let proj = ray_to_handle.dot(ray_dir);
                        if proj > 0.0 {
                            let closest_point_on_ray = ray.origin + ray_dir * proj;
                            let dist_to_handle = closest_point_on_ray.distance(scale_handle_pos);
                            if dist_to_handle < 0.35 {
                                selection_state.active_drag_axis = Some(i);
                                selection_state.drag_scale = true;
                                selection_state.drag_start_value = scale;
                                let axis_proj =
                                    project_ray_onto_axis(ray.origin, ray_dir, pos, axis_dir);
                                selection_state.drag_start_mouse_proj =
                                    (axis_proj - pos).dot(axis_dir);
                                handle_clicked = true;
                                break;
                            }
                        }

                        // 2. Translation handle detection
                        let axis_proj = project_ray_onto_axis(ray.origin, ray_dir, pos, axis_dir);
                        let s = (axis_proj - pos).dot(axis_dir);
                        if s >= 0.0 && s <= axis_len {
                            let closest_point_on_ray =
                                ray.origin + ray_dir * (axis_proj - ray.origin).dot(ray_dir);
                            let dist_to_axis = closest_point_on_ray.distance(axis_proj);
                            if dist_to_axis < 0.25 {
                                selection_state.active_drag_axis = Some(i);
                                selection_state.drag_scale = false;
                                selection_state.drag_start_value = pos;
                                selection_state.drag_start_mouse_proj = s;
                                handle_clicked = true;
                                break;
                            }
                        }
                    }
                }
            }
        }

        if !handle_clicked {
            // Place or Delete Prefab
            let is_placement = matches!(
                brush.tool,
                SculptTool::PlaceTreeOak
                    | SculptTool::PlaceTreePine
                    | SculptTool::PlaceTreeBirch
                    | SculptTool::PlaceShrub
                    | SculptTool::PlaceCactus
                    | SculptTool::PlaceRock
                    | SculptTool::PlaceCaveEntrance
                    | SculptTool::PlaceSpawnPoint
                    | SculptTool::PlaceHouse
                    | SculptTool::PlaceOreCopper
                    | SculptTool::PlaceOreIron
                    | SculptTool::PlaceOreGold
                    | SculptTool::PlaceOreSilver
                    | SculptTool::PlaceOrePlatinum
                    | SculptTool::PlaceOreSteel
                    | SculptTool::PlaceOreGranite
                    | SculptTool::PlaceModularWall
                    | SculptTool::PlaceModularCorner
                    | SculptTool::PlaceModularFloor
                    | SculptTool::PlaceModularRoof
                    | SculptTool::PlaceModularRoofGable
                    | SculptTool::PlaceModularDoorFrame
                    | SculptTool::PlaceModularWindowFrame
                    | SculptTool::PlaceWallTJunction
                    | SculptTool::PlaceWallCross
                    | SculptTool::PlaceCeilingTile
                    | SculptTool::PlaceFluorescentLight
                    | SculptTool::PlaceHallwaySegment
                    | SculptTool::PlaceRoomPillar
                    | SculptTool::PlaceChest
                    | SculptTool::PlaceWorkbench
                    | SculptTool::PlaceFurnace
                    | SculptTool::PlaceBed
                    | SculptTool::PlaceTorch
                    | SculptTool::PlaceChair
                    | SculptTool::PlaceDesk
                    | SculptTool::PlaceHealthPack
                    | SculptTool::PlaceCrate
                    | SculptTool::PlaceCustomAsset
                    | SculptTool::PlaceCustomMesh
            );

            if is_placement {
                let prefab_type = match brush.tool {
                    SculptTool::PlaceTreeOak => "tree_oak",
                    SculptTool::PlaceTreePine => "tree_pine",
                    SculptTool::PlaceTreeBirch => "tree_birch",
                    SculptTool::PlaceShrub => "shrub",
                    SculptTool::PlaceCactus => "cactus",
                    SculptTool::PlaceRock => "rock",
                    SculptTool::PlaceCaveEntrance => "cave_entrance",
                    SculptTool::PlaceSpawnPoint => "spawn_point",
                    SculptTool::PlaceHouse => "house",
                    SculptTool::PlaceOreCopper => "ore_copper",
                    SculptTool::PlaceOreIron => "ore_iron",
                    SculptTool::PlaceOreGold => "ore_gold",
                    SculptTool::PlaceOreSilver => "ore_silver",
                    SculptTool::PlaceOrePlatinum => "ore_platinum",
                    SculptTool::PlaceOreSteel => "ore_steel",
                    SculptTool::PlaceOreGranite => "ore_granite",
                    SculptTool::PlaceModularWall => "wall_straight",
                    SculptTool::PlaceModularCorner => "wall_corner",
                    SculptTool::PlaceModularFloor => "floor_tile",
                    SculptTool::PlaceModularRoof => "roof_tile",
                    SculptTool::PlaceModularRoofGable => "roof_gable",
                    SculptTool::PlaceModularDoorFrame => "door_frame",
                    SculptTool::PlaceModularWindowFrame => "window_frame",
                    SculptTool::PlaceChest => "chest",
                    SculptTool::PlaceWorkbench => "workbench",
                    SculptTool::PlaceFurnace => "furnace",
                    SculptTool::PlaceBed => "bed",
                    SculptTool::PlaceTorch => "torch",
                    SculptTool::PlaceChair => "prop_chair",
                    SculptTool::PlaceDesk => "prop_desk",
                    SculptTool::PlaceHealthPack => "prop_health_pack",
                    SculptTool::PlaceCrate => "prop_crate",
                    SculptTool::PlaceWallTJunction => "wall_t_junction",
                    SculptTool::PlaceWallCross => "wall_cross",
                    SculptTool::PlaceCeilingTile => "ceiling_tile",
                    SculptTool::PlaceFluorescentLight => "fluorescent_light",
                    SculptTool::PlaceHallwaySegment => "hallway_segment",
                    SculptTool::PlaceRoomPillar => "room_pillar",
                    SculptTool::PlaceCustomAsset => "custom_asset",
                    SculptTool::PlaceCustomMesh => "custom_mesh",
                    _ => unreachable!(),
                };

                // For modular building blocks, use snapping system
                let is_modular = matches!(
                    prefab_type,
                    "wall_straight"
                        | "wall_corner"
                        | "floor_tile"
                        | "roof_tile"
                        | "roof_gable"
                        | "door_frame"
                        | "window_frame"
                        | "wall_t_junction"
                        | "wall_cross"
                        | "ceiling_tile"
                        | "hallway_segment"
                );

                // Single-house constraint: remove any previously placed house
                if prefab_type == "house" {
                    map.prefabs.retain(|p| p.prefab_type != "house");
                    for (entity, marker) in prefab_query.iter() {
                        if marker.prefab_type == "house" {
                            commands.entity(entity).despawn();
                        }
                    }
                }

                let placement_scale = if selection_state.placement_flipped {
                    Vec3::new(-1.0, 1.0, 1.0)
                } else {
                    Vec3::ONE
                };

                // Calculate position and rotation based on snapping or random rotation
                let (place_pos, rotation, scale) = if is_modular {
                    let (snapped_pos, snapped_rot) = calculate_snap(
                        prefab_type,
                        intersection,
                        ray.origin,
                        *ray.direction,
                        &map,
                        selection_state.snap_to_grid,
                        selection_state.snap_grid_size,
                        selection_state.snap_to_objects,
                    );
                    let final_rot = snapped_rot
                        * Quat::from_rotation_y(selection_state.placement_rotation_angle);
                    (snapped_pos, final_rot, placement_scale)
                } else {
                    let is_prop = matches!(
                        prefab_type,
                        "chest"
                            | "workbench"
                            | "furnace"
                            | "bed"
                            | "torch"
                            | "fluorescent_light"
                            | "prop_chair"
                            | "prop_desk"
                            | "prop_health_pack"
                            | "prop_crate"
                    );
                    let rot_y = if prefab_type == "house"
                        || prefab_type == "custom_mesh"
                        || prefab_type == "custom_asset"
                        || is_prop
                    {
                        0.0
                    } else {
                        let seed =
                            (intersection.x * 12.9898 + intersection.z * 78.233).sin() * 43758.547;
                        seed.fract() * std::f32::consts::TAU
                    };
                    let final_rot =
                        Quat::from_rotation_y(rot_y + selection_state.placement_rotation_angle);
                    let mut final_pos = intersection;
                    if prefab_type == "custom_mesh" {
                        final_pos.y += 1.0;
                    }
                    (final_pos, final_rot, placement_scale)
                };

                // For custom assets, store the file path in texture_override
                let tex_override: Option<&str> = if prefab_type == "custom_asset" {
                    custom_assets
                        .selected_asset_idx
                        .and_then(|i| custom_assets.assets.get(i))
                        .map(|entry| entry.file_path.as_str())
                } else {
                    None
                };

                let custom_mesh_data = if prefab_type == "custom_mesh" {
                    match selection_state.custom_mesh_primitive {
                        CustomMeshPrimitive::Cube => Some(EditableMesh::new_cube(2.0)),
                        CustomMeshPrimitive::Sphere => Some(EditableMesh::new_sphere(1.0)),
                    }
                } else {
                    None
                };

                let new_index = map.prefabs.len();
                spawn_prefab_visuals(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    prefab_type,
                    place_pos,
                    rotation,
                    scale,
                    tex_override,
                    &mansion_settings,
                    new_index,
                    &asset_server,
                    custom_mesh_data.as_ref(),
                );

                map.prefabs.push(PlacedPrefab {
                    prefab_type: prefab_type.to_string(),
                    position: place_pos.to_array(),
                    rotation: rotation.to_array(),
                    scale: scale.to_array(),
                    texture_override: tex_override.map(|s| s.to_string()),
                    rotation_y: None,
                    custom_mesh: custom_mesh_data,
                });

                // If placing a modular piece, clear overlapping natural obstacles (trees, rocks, shrubs, cacti, ores)
                if is_modular {
                    let clear_radius = match prefab_type {
                        "hallway_segment" => 4.5f32,
                        _ => 2.2f32,
                    };

                    let mut idx_to_remove = Vec::new();
                    for (idx, p) in map.prefabs.iter().enumerate() {
                        if idx == map.prefabs.len() - 1 {
                            continue;
                        }
                        let p_type = &p.prefab_type;
                        if p_type.starts_with("tree")
                            || p_type == "rock"
                            || p_type == "shrub"
                            || p_type == "cactus"
                            || p_type.starts_with("ore_")
                        {
                            let p_pos = Vec3::from_array(p.position);
                            if place_pos.distance(p_pos) < clear_radius {
                                idx_to_remove.push(idx);
                            }
                        }
                    }

                    if !idx_to_remove.is_empty() {
                        idx_to_remove.sort_by(|a, b| b.cmp(a));
                        for &idx in idx_to_remove.iter() {
                            let removed = map.prefabs.remove(idx);
                            let removed_pos = Vec3::from_array(removed.position);
                            for (entity, marker) in prefab_query.iter() {
                                if marker.position.distance(removed_pos) < 0.1 {
                                    commands.entity(entity).despawn();
                                    break;
                                }
                            }
                        }
                    }
                }

                // If placing a house, clear overlapping vegetation and flatten terrain
                if prefab_type == "house" {
                    let half_w = (mansion_settings.cols as f32 * mansion_settings.cell_size) / 2.0;
                    let half_d = (mansion_settings.rows as f32 * mansion_settings.cell_size) / 2.0;

                    // 1. Remove overlapping prefabs from map data
                    map.prefabs.retain(|p| {
                        if p.prefab_type == "house" || p.prefab_type == "spawn_point" {
                            return true;
                        }
                        let p_pos = Vec3::from_array(p.position);
                        let inside = (p_pos.x - intersection.x).abs() < half_w + 1.0
                            && (p_pos.z - intersection.z).abs() < half_d + 1.0;
                        !inside
                    });

                    // 2. Despawn overlapping visual entities in editor
                    for (entity, marker) in prefab_query.iter() {
                        if marker.prefab_type == "house" || marker.prefab_type == "spawn_point" {
                            continue;
                        }
                        let inside = (marker.position.x - intersection.x).abs() < half_w + 1.0
                            && (marker.position.z - intersection.z).abs() < half_d + 1.0;
                        if inside {
                            commands.entity(entity).despawn();
                        }
                    }

                    // 3. Flatten terrain under the house footprint
                    let half_map_w = map.width as f32 / 2.0;
                    let half_map_h = map.height as f32 / 2.0;

                    let min_x_idx = ((intersection.x - half_w - 2.0) + half_map_w).max(0.0) as u32;
                    let max_x_idx =
                        ((intersection.x + half_w + 2.0) + half_map_w).min(map.width as f32) as u32;
                    let min_z_idx = ((intersection.z - half_d - 2.0) + half_map_h).max(0.0) as u32;
                    let max_z_idx = ((intersection.z + half_d + 2.0) + half_map_h)
                        .min(map.height as f32) as u32;

                    let natural_h = map.get_height(intersection.x as u32, intersection.z as u32);
                    let house_ground_y = natural_h.clamp(1.5, 45.0);

                    for mz in min_z_idx..max_z_idx {
                        for mx in min_x_idx..max_x_idx {
                            map.set_height(mx, mz, house_ground_y);
                            map.set_biome(mx, mz, Biome::Temperate);
                        }
                    }

                    // Rebuild terrain mesh in-place
                    for (terrain_entity, mesh_3d) in terrain_query.iter() {
                        rebuild_terrain_mesh(
                            terrain_entity,
                            &mut commands,
                            &map,
                            &settings,
                            &mut meshes,
                            Some(mesh_3d),
                        );
                    }
                    grass_writer.write(crate::grass::GenerateGrassEvent);
                }

                // Flatten terrain under floor tiles and hallway segments
                if prefab_type == "floor_tile" || prefab_type == "hallway_segment" {
                    let (fw, fd) = if prefab_type == "hallway_segment" {
                        (4.0f32, 8.0f32)
                    } else {
                        (4.0f32, 4.0f32)
                    };
                    let half_fw = fw / 2.0;
                    let half_fd = fd / 2.0;
                    let floor_y = place_pos.y;
                    let half_map_w = map.width as f32 / 2.0;
                    let half_map_h = map.height as f32 / 2.0;

                    let min_x_idx = ((place_pos.x - half_fw - 1.0) + half_map_w).max(0.0) as u32;
                    let max_x_idx =
                        ((place_pos.x + half_fw + 1.0) + half_map_w).min(map.width as f32) as u32;
                    let min_z_idx = ((place_pos.z - half_fd - 1.0) + half_map_h).max(0.0) as u32;
                    let max_z_idx =
                        ((place_pos.z + half_fd + 1.0) + half_map_h).min(map.height as f32) as u32;

                    for mz in min_z_idx..max_z_idx {
                        for mx in min_x_idx..max_x_idx {
                            let current_h = map.get_height(mx, mz);
                            if current_h > floor_y {
                                map.set_height(mx, mz, floor_y);
                            }
                        }
                    }

                    for (terrain_entity, mesh_3d) in terrain_query.iter() {
                        rebuild_terrain_mesh(
                            terrain_entity,
                            &mut commands,
                            &map,
                            &settings,
                            &mut meshes,
                            Some(mesh_3d),
                        );
                    }
                }
                reindex_prefab_markers(&map.prefabs, &mut prefab_query);
            } else if brush.tool == SculptTool::DeletePrefab {
                // Find closest prefab within 2.0 meters
                let mut closest_idx: Option<usize> = None;
                let mut closest_dist = 2.0;

                for (idx, p) in map.prefabs.iter().enumerate() {
                    let p_pos = Vec3::from_array(p.position);
                    let dist = intersection.distance(p_pos);
                    if dist < closest_dist {
                        closest_dist = dist;
                        closest_idx = Some(idx);
                    }
                }

                if let Some(idx) = closest_idx {
                    let p_pos = Vec3::from_array(map.prefabs[idx].position);
                    // Despawn 3D parent entity and its child meshes
                    for (entity, marker) in prefab_query.iter() {
                        if marker.position.distance(p_pos) < 0.05 {
                            commands.entity(entity).despawn();
                            break;
                        }
                    }
                    map.prefabs.remove(idx);
                }
                reindex_prefab_markers(&map.prefabs, &mut prefab_query);
            } else if brush.tool == SculptTool::SelectObject {
                // Find closest prefab within 3.0 meters and select it
                let mut closest_idx: Option<usize> = None;
                let mut closest_dist = 3.0f32;

                for (idx, p) in map.prefabs.iter().enumerate() {
                    let p_pos = Vec3::from_array(p.position);
                    let dist = intersection.distance(p_pos);
                    if dist < closest_dist {
                        closest_dist = dist;
                        closest_idx = Some(idx);
                    }
                }

                selection_state.selected_idx = closest_idx;
            }
        } // close of if !handle_clicked
    }

    // Draw gizmo axes around selected object
    draw_gizmo_handles(&mut gizmos, &selection_state, &map);
}

fn draw_gizmo_handles(gizmos: &mut Gizmos, selection_state: &SelectionState, map: &TempestMap) {
    let Some(sel_idx) = selection_state.selected_idx else {
        return;
    };
    if sel_idx >= map.prefabs.len() {
        return;
    }

    let sel = &map.prefabs[sel_idx];
    let pos = Vec3::from_array(sel.position);
    let rot = Quat::from_array(sel.rotation);
    let scale = Vec3::from_array(sel.scale);

    let axis_len = 2.0;
    let axes = [Vec3::X, Vec3::Y, Vec3::Z];
    let colors = [
        Color::srgb(1.0, 0.0, 0.0), // X = Red
        Color::srgb(0.0, 1.0, 0.0), // Y = Green
        Color::srgb(0.0, 0.0, 1.0), // Z = Blue
    ];

    // Draw central white pivot sphere
    gizmos.sphere(pos, 0.1, Color::srgb(1.0, 1.0, 1.0));

    for i in 0..3 {
        let axis_dir = rot * axes[i];
        let line_end = pos + axis_dir * axis_len;
        let color = colors[i];

        // 1. Draw the axis line
        gizmos.line(pos, line_end, color);

        // 2. Draw Translation handle (a small sphere along the line)
        let trans_pos = pos + axis_dir * (axis_len * 0.65);
        gizmos.sphere(trans_pos, 0.08, color);

        // 3. Draw Scale handle (a small cube at the end of the line)
        let scale_pos = line_end;
        let right_dir = rot * axes[(i + 1) % 3];
        let up_dir = rot * axes[(i + 2) % 3];
        let half_size = 0.08;

        let c000 = scale_pos - axis_dir * half_size - right_dir * half_size - up_dir * half_size;
        let c100 = scale_pos + axis_dir * half_size - right_dir * half_size - up_dir * half_size;
        let c010 = scale_pos - axis_dir * half_size + right_dir * half_size - up_dir * half_size;
        let c110 = scale_pos + axis_dir * half_size + right_dir * half_size - up_dir * half_size;
        let c001 = scale_pos - axis_dir * half_size - right_dir * half_size + up_dir * half_size;
        let c101 = scale_pos + axis_dir * half_size - right_dir * half_size + up_dir * half_size;
        let c011 = scale_pos - axis_dir * half_size + right_dir * half_size + up_dir * half_size;
        let c111 = scale_pos + axis_dir * half_size + right_dir * half_size + up_dir * half_size;

        gizmos.line(c000, c100, color);
        gizmos.line(c010, c110, color);
        gizmos.line(c001, c101, color);
        gizmos.line(c011, c111, color);

        gizmos.line(c000, c010, color);
        gizmos.line(c100, c110, color);
        gizmos.line(c001, c011, color);
        gizmos.line(c101, c111, color);

        gizmos.line(c000, c001, color);
        gizmos.line(c100, c101, color);
        gizmos.line(c010, c011, color);
        gizmos.line(c110, c111, color);
    }

    // Selection wireframe box
    let half = scale * 0.5;
    let corners = [
        pos + rot * Vec3::new(-half.x, 0.0, -half.z),
        pos + rot * Vec3::new(half.x, 0.0, -half.z),
        pos + rot * Vec3::new(half.x, 0.0, half.z),
        pos + rot * Vec3::new(-half.x, 0.0, half.z),
        pos + rot * Vec3::new(-half.x, half.y * 2.0, -half.z),
        pos + rot * Vec3::new(half.x, half.y * 2.0, -half.z),
        pos + rot * Vec3::new(half.x, half.y * 2.0, half.z),
        pos + rot * Vec3::new(-half.x, half.y * 2.0, half.z),
    ];
    let sel_color = Color::srgb(1.0, 1.0, 0.0);
    // Bottom face
    for i in 0..4 {
        gizmos.line(corners[i], corners[(i + 1) % 4], sel_color);
    }
    // Top face
    for i in 4..8 {
        gizmos.line(corners[i], corners[4 + (i - 4 + 1) % 4], sel_color);
    }
    // Verticals
    for i in 0..4 {
        gizmos.line(corners[i], corners[i + 4], sel_color);
    }
}

fn disable_ui_camera_clear(mut query: Query<&mut Camera, With<Camera2d>>) {
    for mut camera in query.iter_mut() {
        camera.clear_color = ClearColorConfig::None;
    }
}

fn enable_ui_camera_clear(mut query: Query<&mut Camera, With<Camera2d>>) {
    for mut camera in query.iter_mut() {
        camera.clear_color = ClearColorConfig::Default;
    }
}

pub fn water_simulation_system(
    time: Res<Time>,
    water_settings: Res<WaterSettings>,
    map: Res<TempestMap>,
    mut query: Query<(&mut WaterSimData, &mut Transform), With<WaterMesh>>,
    mut impulse_events: MessageReader<WaterImpulseEvent>,
) {
    let delta_time = time.delta_secs().min(0.09); // Cap dt to prevent wave explosions
    let gravity: f32 = 12.0;
    let friction: f32 = 0.94; // Realistic decay of waves over distance

    for (mut water_data, mut transform) in query.iter_mut() {
        // Sync transform Y to water settings height
        transform.translation.y = water_settings.height;

        let w = water_data.width;
        let h = water_data.height;

        if w > 256 || h > 256 {
            continue;
        }

        // Dynamic shoreline/wall updates based on current terrain and water height
        for z in 0..h {
            for x in 0..w {
                // Border limits are always walls
                if x == 0 || x == w - 1 || z == 0 || z == h - 1 {
                    water_data.set_wall(x, z, true);
                    continue;
                }

                // If terrain is above water settings height, it's solid shoreline
                let terrain_h = map.get_height(x, z);
                let is_solid = terrain_h >= water_settings.height;
                water_data.set_wall(x, z, is_solid);
            }
        }

        // --- SHALLOW WATER SIMULATION ---
        // Clear border flows
        for i in 0..w {
            water_data.set_flow_x(0, i, 0.0);
            water_data.set_flow_x(w - 1, i, 0.0);
            water_data.set_flow_y(i, 0, 0.0);
            water_data.set_flow_y(i, h - 1, 0.0);
        }

        // Calculate flow based on height difference
        for z in 0..h {
            for x in 0..w {
                if x > 0 {
                    let source_has_wall = water_data.is_wall(x - 1, z);
                    let dest_has_wall = water_data.is_wall(x, z);
                    let height_diff = water_data.get_height(x - 1, z) - water_data.get_height(x, z);

                    if !source_has_wall && !dest_has_wall {
                        let current_flow = water_data.get_flow_x(x, z);
                        let new_flow = current_flow * friction.powf(delta_time)
                            + height_diff * gravity * delta_time;
                        water_data.set_flow_x(x, z, new_flow);
                    } else {
                        water_data.set_flow_x(x, z, 0.0);
                    }
                } else {
                    water_data.set_flow_x(x, z, 0.0);
                }

                if z > 0 {
                    let source_has_wall = water_data.is_wall(x, z - 1);
                    let dest_has_wall = water_data.is_wall(x, z);
                    let height_diff = water_data.get_height(x, z - 1) - water_data.get_height(x, z);

                    if !source_has_wall && !dest_has_wall {
                        let current_flow = water_data.get_flow_y(x, z);
                        let new_flow = current_flow * friction.powf(delta_time)
                            + height_diff * gravity * delta_time;
                        water_data.set_flow_y(x, z, new_flow);
                    } else {
                        water_data.set_flow_y(x, z, 0.0);
                    }
                } else {
                    water_data.set_flow_y(x, z, 0.0);
                }
            }
        }

        // Outflow scaling to prevent grid cells draining below zero height
        for z in 0..h {
            for x in 0..w {
                if water_data.is_wall(x, z) {
                    continue;
                }

                let mut total_outflow = 0.0;
                total_outflow += 0.0f32.max(-water_data.get_flow_x(x, z));
                total_outflow += 0.0f32.max(-water_data.get_flow_y(x, z));

                if x < w - 1 {
                    total_outflow += 0.0f32.max(water_data.get_flow_x(x + 1, z));
                }
                if z < h - 1 {
                    total_outflow += 0.0f32.max(water_data.get_flow_y(x, z + 1));
                }

                let max_outflow = water_data.get_height(x, z) / delta_time;

                if total_outflow > 0.0 {
                    let scale = 1.0f32.min(max_outflow / total_outflow);
                    if water_data.get_flow_x(x, z) < 0.0 {
                        let val = water_data.get_flow_x(x, z) * scale;
                        water_data.set_flow_x(x, z, val);
                    }
                    if water_data.get_flow_y(x, z) < 0.0 {
                        let val = water_data.get_flow_y(x, z) * scale;
                        water_data.set_flow_y(x, z, val);
                    }
                    if x < w - 1 && water_data.get_flow_x(x + 1, z) > 0.0 {
                        let val = water_data.get_flow_x(x + 1, z) * scale;
                        water_data.set_flow_x(x + 1, z, val);
                    }
                    if z < h - 1 && water_data.get_flow_y(x, z + 1) > 0.0 {
                        let val = water_data.get_flow_y(x, z + 1) * scale;
                        water_data.set_flow_y(x, z + 1, val);
                    }
                }
            }
        }

        // Apply flows and update heights
        for z in 0..h {
            for x in 0..w {
                let mut height_change = 0.0;

                let can_receive_from_left =
                    x > 0 && !water_data.is_wall(x - 1, z) && !water_data.is_wall(x, z);
                if can_receive_from_left {
                    height_change += water_data.get_flow_x(x, z);
                }

                let can_receive_from_top =
                    z > 0 && !water_data.is_wall(x, z - 1) && !water_data.is_wall(x, z);
                if can_receive_from_top {
                    height_change += water_data.get_flow_y(x, z);
                }

                let can_flow_right = x < w - 1 && !water_data.is_wall(x + 1, z);
                if can_flow_right {
                    height_change -= water_data.get_flow_x(x + 1, z);
                }

                let can_flow_bottom = z < h - 1 && !water_data.is_wall(x, z + 1);
                if can_flow_bottom {
                    height_change -= water_data.get_flow_y(x, z + 1);
                }

                let current_height = water_data.get_height(x, z);
                let mut new_height = current_height + height_change * delta_time;

                // Restorative damping returning height back to equilibrium (1.0)
                new_height = new_height + (1.0 - new_height) * 0.035 * delta_time * 60.0;
                new_height = new_height.clamp(0.85, 1.15);

                if water_data.is_wall(x, z) {
                    // Smoothly inherit height from neighboring open-water cells so shoreline moves in unison
                    let mut sum_h = 0.0;
                    let mut count = 0.0;
                    if x > 0 && !water_data.is_wall(x - 1, z) {
                        sum_h += water_data.get_height(x - 1, z);
                        count += 1.0;
                    }
                    if x < w - 1 && !water_data.is_wall(x + 1, z) {
                        sum_h += water_data.get_height(x + 1, z);
                        count += 1.0;
                    }
                    if z > 0 && !water_data.is_wall(x, z - 1) {
                        sum_h += water_data.get_height(x, z - 1);
                        count += 1.0;
                    }
                    if z < h - 1 && !water_data.is_wall(x, z + 1) {
                        sum_h += water_data.get_height(x, z + 1);
                        count += 1.0;
                    }
                    if count > 0.0 {
                        new_height = sum_h / count;
                    } else {
                        new_height = 1.0;
                    }
                }

                water_data.set_height(x, z, new_height);
            }
        }

        // Process any impulses
        for event in impulse_events.read() {
            let offset_x = -(w as f32) / 2.0;
            let offset_z = -(h as f32) / 2.0;

            let grid_x_f = event.position.x - offset_x;
            let grid_z_f = event.position.z - offset_z;

            let grid_x = grid_x_f.round() as i32;
            let grid_z = grid_z_f.round() as i32;

            let radius = event.radius.round() as i32;

            for rz in -radius..=radius {
                for rx in -radius..=radius {
                    let px = grid_x + rx;
                    let pz = grid_z + rz;

                    if px >= 0 && px < w as i32 && pz >= 0 && pz < h as i32 {
                        let dist_sq = rx * rx + rz * rz;
                        let radius_sq = radius * radius;
                        if dist_sq <= radius_sq {
                            if water_data.is_wall(px as u32, pz as u32) {
                                continue;
                            }
                            let dist = (dist_sq as f32).sqrt();
                            let falloff = 1.0 - (dist / event.radius.max(0.1));
                            let current = water_data.get_height(px as u32, pz as u32);
                            water_data.set_height(
                                px as u32,
                                pz as u32,
                                current + event.force * falloff,
                            );
                        }
                    }
                }
            }
        }
    }
}

pub fn configure_terrain_sampler_system(
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
    mut is_done: Local<bool>,
) {
    if *is_done {
        return;
    }

    let handle = asset_server.load("textures/ground_grass.png");

    // Check if loaded first without triggering change detection
    if images.get(&handle).is_none() {
        return;
    }

    if let Some(mut image) = images.get_mut(&handle) {
        let mut needs_sampler_update =
            !matches!(image.sampler, bevy::image::ImageSampler::Descriptor(_));

        if let Some(ref mut data) = image.data {
            let mut is_desaturated = true;
            for chunk in data.chunks_exact(4) {
                if chunk[0] != chunk[1] || chunk[1] != chunk[2] {
                    is_desaturated = false;
                    break;
                }
            }

            if !is_desaturated {
                for chunk in data.chunks_exact_mut(4) {
                    let r = chunk[0] as f32 / 255.0;
                    let g = chunk[1] as f32 / 255.0;
                    let b = chunk[2] as f32 / 255.0;
                    let l = 0.299 * r + 0.587 * g + 0.114 * b;
                    let detail = 0.8 + l * 0.2;
                    let final_val = (detail * 255.0f32).clamp(0.0f32, 255.0f32) as u8;
                    chunk[0] = final_val;
                    chunk[1] = final_val;
                    chunk[2] = final_val;
                }
                needs_sampler_update = true;
            }
        }

        if needs_sampler_update {
            image.sampler =
                bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
                    address_mode_u: bevy::image::ImageAddressMode::Repeat,
                    address_mode_v: bevy::image::ImageAddressMode::Repeat,
                    ..default()
                });
        }

        *is_done = true;
    }
}

pub fn generate_roads_on_map(map: &mut TempestMap) {
    // 1. Initialize/reset road map vector to match the current grid dimensions
    map.road_map = vec![0; (map.width * map.height) as usize];

    let w = map.width as i32;
    let h = map.height as i32;

    let center_x = w / 2;
    let center_z = h / 2;

    // 2. Define locations (nodes) to connect - scaled proportionally to map size
    let span_x = (w as f32 * 0.21) as i32;
    let span_z = (h as f32 * 0.21) as i32;

    let nodes = vec![
        (center_x, center_z),                   // Mansion (Center)
        (center_x + span_x, center_z - span_z), // Alien Settlement (North-East)
        (center_x - span_x, center_z + span_z), // Outpost Alpha (South-West)
        (center_x + span_x, center_z + span_z), // Outpost Beta (South-East)
        (center_x - span_x, center_z - span_z), // Outpost Gamma (North-West)
        (center_x, center_z - span_z),          // Outpost Delta (North)
        (center_x, center_z + span_z),          // Outpost Epsilon (South)
        (center_x - span_x, center_z),          // Outpost Zeta (West)
        (center_x + span_x, center_z),          // Outpost Eta (East)
    ];

    // Connect nodes to form an intricate highway network loop
    let connections = vec![
        // Central Cross (Spokes)
        (0, 5), // Mansion -> North
        (0, 6), // Mansion -> South
        (0, 7), // Mansion -> West
        (0, 8), // Mansion -> East
        // Diagonal Spokes
        (0, 1), // Mansion -> North-East
        (0, 2), // Mansion -> South-West
        (0, 3), // Mansion -> South-East
        (0, 4), // Mansion -> North-West
        // Outer Ring
        (5, 1), // North -> North-East
        (1, 8), // North-East -> East
        (8, 3), // East -> South-East
        (3, 6), // South-East -> South
        (6, 2), // South -> South-West
        (2, 7), // South-West -> West
        (7, 4), // West -> North-West
        (4, 5), // North-West -> North
    ];

    for &(start_idx, end_idx) in &connections {
        if start_idx < nodes.len() && end_idx < nodes.len() {
            let start = nodes[start_idx];
            let end = nodes[end_idx];

            // Pathfind using the optimized A* algorithm
            if let Some(path) = pathfind_road(map, start, end) {
                // Paint the road and grade the terrain along it
                for &(px, pz) in path.iter() {
                    let local_h = map.get_height(px, pz);
                    let road_h = local_h.max(1.35); // Follow natural terrain contours while keeping land roads dry!

                    // Set road cell type (1 = Asphalt Paved, 2 = Alien Dirt/Gravel, 3 = Bridge)
                    let road_type = if start_idx == 1 || end_idx == 1 { 2 } else { 1 };

                    // Inner 3x3 core (flat road surface)
                    for rz in -1..=1 {
                        for rx in -1..=1 {
                            let nx = px as i32 + rx;
                            let nz = pz as i32 + rz;
                            if nx >= 0 && nx < w && nz >= 0 && nz < h {
                                let neighbor_original_h = map.get_height(nx as u32, nz as u32);
                                if neighbor_original_h <= 1.2 {
                                    // Water: paint a bridge and leave terrain height natural (deep)
                                    map.set_road(nx as u32, nz as u32, 3);
                                } else {
                                    // Land: paint standard road and set smooth graded height following natural contours
                                    map.set_road(nx as u32, nz as u32, road_type);
                                    let cell_road_h = neighbor_original_h * 0.7 + road_h * 0.3;
                                    map.set_height(nx as u32, nz as u32, cell_road_h.max(1.35));
                                }
                            }
                        }
                    }

                    // Outer 5x5 shoulders (smooth transition to terrain)
                    for rz in -2_i32..=2_i32 {
                        for rx in -2_i32..=2_i32 {
                            if rx.abs() == 2 || rz.abs() == 2 {
                                let nx = px as i32 + rx;
                                let nz = pz as i32 + rz;
                                if nx >= 0 && nx < w && nz >= 0 && nz < h {
                                    let curr_h = map.get_height(nx as u32, nz as u32);
                                    // Only smooth land cells, do not fill in waterways
                                    if curr_h > 1.2 {
                                        let blend_h = curr_h * 0.85 + road_h * 0.15;
                                        map.set_height(nx as u32, nz as u32, blend_h.max(1.35));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn pathfind_road(map: &TempestMap, start: (i32, i32), end: (i32, i32)) -> Option<Vec<(u32, u32)>> {
    use std::cmp::Ordering;
    use std::collections::BinaryHeap;

    #[derive(Copy, Clone, Eq, PartialEq)]
    struct State {
        cost: i32,
        idx: usize,
    }

    impl Ord for State {
        fn cmp(&self, other: &Self) -> Ordering {
            other.cost.cmp(&self.cost) // Min-heap
        }
    }

    impl PartialOrd for State {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    let w = map.width as i32;
    let h = map.height as i32;
    let size = (w * h) as usize;

    let mut to_visit = BinaryHeap::new();
    let mut g_score = vec![i32::MAX; size];
    let mut parent = vec![usize::MAX; size];

    let start_idx = (start.1 * w + start.0) as usize;
    let end_idx = (end.1 * w + end.0) as usize;

    g_score[start_idx] = 0;
    to_visit.push(State {
        cost: 0,
        idx: start_idx,
    });

    while let Some(State { cost, idx }) = to_visit.pop() {
        if idx == end_idx {
            // Reconstruct path
            let mut path = Vec::new();
            let mut curr = end_idx;
            while curr != start_idx {
                let cx = (curr as i32) % w;
                let cz = (curr as i32) / w;
                path.push((cx as u32, cz as u32));
                curr = parent[curr];
                if curr == usize::MAX {
                    break;
                }
            }
            path.push((start.0 as u32, start.1 as u32));
            path.reverse();
            return Some(path);
        }

        if cost
            > g_score[idx]
                + ((idx as i32 % w - end.0).abs() * 10 + (idx as i32 / w - end.1).abs() * 10)
        {
            continue;
        }

        let curr_g = g_score[idx];
        let cx = (idx as i32) % w;
        let cz = (idx as i32) / w;

        // 8-directional movement
        let dirs = [
            (0, 1),
            (1, 0),
            (0, -1),
            (-1, 0),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ];

        for &(dx, dz) in &dirs {
            let nx = cx + dx;
            let nz = cz + dz;
            if nx < 0 || nx >= w || nz < 0 || nz >= h {
                continue;
            }

            let next_idx = (nz * w + nx) as usize;
            let next_h = map.get_height(nx as u32, nz as u32);
            let curr_h = map.get_height(cx as u32, cz as u32);

            let height_diff = (next_h - curr_h).abs();

            // base distance cost
            let base_cost = if dx != 0 && dz != 0 { 14 } else { 10 };

            // cost multiplier for slopes (heavily avoid steep mountains)
            let slope_penalty = (height_diff * 45.0) as i32;

            // cost multiplier for deep water (avoid routing through water)
            let water_penalty = if next_h <= 1.2 {
                if next_h <= 0.0 { 1200 } else { 250 }
            } else {
                0
            };

            // cost reduction for existing roads (encourages roads to merge)
            let existing_road_discount = if map.get_road(nx as u32, nz as u32) > 0 {
                -6
            } else {
                0
            };

            let step_cost =
                (base_cost + slope_penalty + water_penalty + existing_road_discount).max(1);
            let new_g = curr_g + step_cost;

            if new_g < g_score[next_idx] {
                g_score[next_idx] = new_g;
                parent[next_idx] = idx;

                let h_score = (nx - end.0).abs() * 10 + (nz - end.1).abs() * 10;
                to_visit.push(State {
                    cost: new_g + h_score,
                    idx: next_idx,
                });
            }
        }
    }

    None
}

pub fn spawn_editor_bridges(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    map: &TempestMap,
    asset_server: &Res<AssetServer>,
) {
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
            if map.get_road(x, z) == 3 && map.get_height(x, z) <= 1.2 {
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
                    Transform::from_xyz(vx, 1.35, vz).with_rotation(rot),
                    EditorBridge,
                    MapEditorEntity,
                ));
            }
        }
    }
}
