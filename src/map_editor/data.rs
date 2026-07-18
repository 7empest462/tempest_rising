use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Biome {
    #[default]
    Temperate,
    Arid,
    Tundra,
    Arctic,
}

fn default_quaternion() -> [f32; 4] {
    [0.0, 0.0, 0.0, 1.0]
}

fn default_scale() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct EditableMesh {
    pub vertices: Vec<[f32; 3]>,
    pub faces: Vec<Vec<u32>>, // Indices of vertices forming each face (supports arbitrary polygons, e.g. quads/triangles)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlacedPrefab {
    pub prefab_type: String, // "tree_oak", "tree_pine", "tree_birch", "rock", "spawn_point", "shrub", "cactus", "ore_copper", "ore_iron", "ore_gold", "ore_silver", "ore_platinum", "ore_steel", "ore_granite"
    pub position: [f32; 3],  // Serializable array for JSON compatibility
    #[serde(default = "default_quaternion")]
    pub rotation: [f32; 4], // Quaternion [x, y, z, w]
    #[serde(default = "default_scale")]
    pub scale: [f32; 3], // Scale multiplier [x, y, z]
    #[serde(default)]
    pub texture_override: Option<String>,
    #[serde(default)]
    pub rotation_y: Option<f32>, // Kept for backward compatibility
    #[serde(default)]
    pub custom_mesh: Option<EditableMesh>,
}

#[derive(Serialize, Deserialize, Debug, Clone, bevy::prelude::Resource)]
pub struct TempestMap {
    pub width: u32,
    pub height: u32,
    pub terrain: Vec<f32>,
    #[serde(default)]
    pub biome_map: Vec<Biome>,
    #[serde(default)]
    pub prefabs: Vec<PlacedPrefab>,
    #[serde(default)]
    pub road_map: Vec<u8>,
}

impl Default for TempestMap {
    fn default() -> Self {
        let width = 1200;
        let height = 1200;
        Self {
            width,
            height,
            terrain: vec![0.0; (width * height) as usize],
            biome_map: vec![Biome::Temperate; (width * height) as usize],
            prefabs: Vec::new(),
            road_map: vec![0; (width * height) as usize],
        }
    }
}

impl TempestMap {
    pub fn get_height(&self, x: u32, z: u32) -> f32 {
        if x < self.width && z < self.height {
            self.terrain[(z * self.width + x) as usize]
        } else {
            0.0
        }
    }

    pub fn set_height(&mut self, x: u32, z: u32, height: f32) {
        if x < self.width && z < self.height {
            self.terrain[(z * self.width + x) as usize] = height;
        }
    }

    pub fn get_biome(&self, x: u32, z: u32) -> Biome {
        let idx = (z * self.width + x) as usize;
        if x < self.width && z < self.height && idx < self.biome_map.len() {
            self.biome_map[idx]
        } else {
            Biome::Temperate
        }
    }

    pub fn set_biome(&mut self, x: u32, z: u32, biome: Biome) {
        if x < self.width && z < self.height {
            let idx = (z * self.width + x) as usize;
            if idx < self.biome_map.len() {
                self.biome_map[idx] = biome;
            }
        }
    }

    pub fn get_road(&self, x: u32, z: u32) -> u8 {
        let idx = (z * self.width + x) as usize;
        if x < self.width && z < self.height && idx < self.road_map.len() {
            self.road_map[idx]
        } else {
            0
        }
    }

    pub fn set_road(&mut self, x: u32, z: u32, val: u8) {
        if x < self.width && z < self.height {
            let idx = (z * self.width + x) as usize;
            if idx < self.road_map.len() {
                self.road_map[idx] = val;
            }
        }
    }

    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        let mut new_terrain = vec![0.0; (new_width * new_height) as usize];
        let mut new_biome_map = vec![Biome::Temperate; (new_width * new_height) as usize];
        let mut new_road_map = vec![0; (new_width * new_height) as usize];
        let min_w = self.width.min(new_width);
        let min_h = self.height.min(new_height);
        for z in 0..min_h {
            for x in 0..min_w {
                let old_idx = (z * self.width + x) as usize;
                let new_idx = (z * new_width + x) as usize;
                new_terrain[new_idx] = self.terrain[old_idx];
                if old_idx < self.biome_map.len() {
                    new_biome_map[new_idx] = self.biome_map[old_idx];
                }
                if old_idx < self.road_map.len() {
                    new_road_map[new_idx] = self.road_map[old_idx];
                }
            }
        }
        self.width = new_width;
        self.height = new_height;
        self.terrain = new_terrain;
        self.biome_map = new_biome_map;
        self.road_map = new_road_map;

        // Retain prefabs that are within the new map bounds (offset from center)
        let half_w = new_width as f32 / 2.0;
        let half_h = new_height as f32 / 2.0;
        self.prefabs.retain(|p| {
            let px = p.position[0];
            let pz = p.position[2];
            px >= -half_w && px <= half_w && pz >= -half_h && pz <= half_h
        });
    }
}
