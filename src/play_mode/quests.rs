use bevy::prelude::WorldAssetRoot;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::map_editor::data::TempestMap;
use crate::play_mode::{PlayModeEntity, PlayModePlayer, inventory_log};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum QuestId {
    #[default]
    None,
    RebuildShrine,
    ThreatAtHatchery,
    AstralCompass,
    HarmonyBeacons,
}

impl QuestId {
    pub fn title(&self) -> &'static str {
        match self {
            QuestId::None => "No Active Quest",
            QuestId::RebuildShrine => "🛠️ Quest 1: Restoring the Hydro-Shrine",
            QuestId::ThreatAtHatchery => "⚔️ Quest 2: Threat at the Hatchery",
            QuestId::AstralCompass => "🗝️ Quest 3: The Astral Compass",
            QuestId::HarmonyBeacons => "📡 Quest 4: Harmony Beacon Network",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            QuestId::None => "Speak with Chieftain Zylar at the Zolyrian Outpost to begin.",
            QuestId::RebuildShrine => {
                "Gather 10 Wood, 10 Granite, & 5 Crystal Shards from caves to rebuild the Alien Hydro-Shrine."
            }
            QuestId::ThreatAtHatchery => {
                "Defeat 5 Corrupted Hostile Beasts in the surrounding hills to protect the alien village."
            }
            QuestId::AstralCompass => {
                "Retrieve the sacred Astral Compass relic from the subterranean basement vault."
            }
            QuestId::HarmonyBeacons => {
                "Activate 3 ancient Alien Harmony Beacons atop the high mountain peaks."
            }
        }
    }

    #[allow(dead_code)]
    pub fn npc_name(&self) -> &'static str {
        match self {
            QuestId::None => "Chieftain Zylar",
            QuestId::RebuildShrine => "Chieftain Zylar",
            QuestId::ThreatAtHatchery => "Scout Kael",
            QuestId::AstralCompass => "Elder Veyla",
            QuestId::HarmonyBeacons => "Chieftain Zylar",
        }
    }
}

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct QuestTracker {
    pub active_quest: QuestId,
    pub quest_progress: u32,
    pub quest_target: u32,
    pub completed_shrine: bool,
    pub completed_hatchery: bool,
    pub completed_compass: bool,
    pub completed_beacons: bool,
    pub beacons_activated: u32,
    pub alien_reputation: u32,
    pub has_energy_shield: bool,
}

impl Default for QuestTracker {
    fn default() -> Self {
        Self {
            active_quest: QuestId::None,
            quest_progress: 0,
            quest_target: 0,
            completed_shrine: false,
            completed_hatchery: false,
            completed_compass: false,
            completed_beacons: false,
            beacons_activated: 0,
            alien_reputation: 0,
            has_energy_shield: false,
        }
    }
}

#[derive(Component)]
pub struct NativeAlienNPC {
    pub name: String,
    pub role: String,
    pub quest_id: QuestId,
    pub initial_pos: Vec3,
}

#[derive(Component)]
pub struct NativeAlienNPCHead;

#[allow(dead_code)]
#[derive(Component)]
pub struct NativeAlienNPCLeftArm;

#[allow(dead_code)]
#[derive(Component)]
pub struct NativeAlienNPCRightArm;

#[derive(Component)]
pub struct NativeAlienNPCStaff;

#[derive(Component)]
pub struct NativeAlienNPCStaffOrb;

#[derive(Component)]
pub struct HydroShrine {
    pub is_rebuilt: bool,
}

#[derive(Component)]
pub struct HarmonyBeacon {
    #[allow(dead_code)]
    pub id: u32,
    pub is_active: bool,
}

#[allow(dead_code)]
#[derive(Component)]
pub struct QuestMarkerUI;

#[derive(Resource, Default)]
pub struct ActiveDialogueState {
    pub is_open: bool,
    pub npc_name: String,
    #[allow(dead_code)]
    pub dialogue_text: String,
    pub quest_offer: QuestId,
}

// Scans natural terrain height across a circular area to find the highest terrain elevation point
pub fn get_max_terrain_in_radius(cx: f32, cz: f32, radius: f32, map: &TempestMap) -> f32 {
    let mut max_y = crate::play_mode::get_bilinear_height(cx, cz, map);

    let step_size = 2.0_f32;
    let r_steps = (radius / step_size) as i32;
    for dx in -r_steps..=r_steps {
        for dz in -r_steps..=r_steps {
            let wx = cx + dx as f32 * step_size;
            let wz = cz + dz as f32 * step_size;
            if (wx - cx).hypot(wz - cz) <= radius {
                let hy = crate::play_mode::get_bilinear_height(wx, wz, map);
                if hy > max_y {
                    max_y = hy;
                }
            }
        }
    }
    max_y
}

// Flattens the natural terrain surrounding the alien outpost platform gently
pub fn flatten_outpost_terrain(map: &mut TempestMap, outpost_pos: Vec3) {
    let half_map_w = map.width as f32 / 2.0;
    let half_map_h = map.height as f32 / 2.0;

    let target_h = outpost_pos.y - 1.5;
    let platform_radius = 14.0_f32;
    let blend_dist = 8.0_f32;
    let total_r = platform_radius + blend_dist;

    let min_x_idx = ((outpost_pos.x - total_r) + half_map_w).max(0.0) as u32;
    let max_x_idx = ((outpost_pos.x + total_r) + half_map_w).min(map.width as f32) as u32;
    let min_z_idx = ((outpost_pos.z - total_r) + half_map_h).max(0.0) as u32;
    let max_z_idx = ((outpost_pos.z + total_r) + half_map_h).min(map.height as f32) as u32;

    for mz in min_z_idx..max_z_idx {
        for mx in min_x_idx..max_x_idx {
            let wx = mx as f32 - half_map_w;
            let wz = mz as f32 - half_map_h;

            let dist = (wx - outpost_pos.x).hypot(wz - outpost_pos.z);
            if dist <= platform_radius {
                map.set_height(mx, mz, target_h);
            } else if dist <= total_r {
                let blend_t = ((dist - platform_radius) / blend_dist).clamp(0.0, 1.0);
                let orig_h = map.get_height(mx, mz);
                let blended = target_h * (1.0 - blend_t) + orig_h * blend_t;
                map.set_height(mx, mz, blended);
            }
        }
    }
}

// Spawns the Zolyrian Native Alien Outpost, NPCs, and Hydro-Shrine
pub fn spawn_alien_outpost(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    outpost_pos: Vec3,
    tracker: &QuestTracker,
) {
    let repeat_tex = asset_server
        .load_builder()
        .with_settings(|settings: &mut bevy::image::ImageLoaderSettings| {
            settings.sampler =
                bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
                    address_mode_u: bevy::image::ImageAddressMode::Repeat,
                    address_mode_v: bevy::image::ImageAddressMode::Repeat,
                    ..default()
                });
        })
        .load("textures/rock_wall.png");

    let alien_struct_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.28, 0.35),
        base_color_texture: Some(repeat_tex),
        metallic: 0.8,
        perceptual_roughness: 0.4,
        emissive: LinearRgba::new(0.05, 0.4, 0.6, 1.0),
        cull_mode: None,
        double_sided: true,
        ..default()
    });

    let glow_cyan_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.1, 1.0, 0.85),
        emissive: LinearRgba::new(3.0, 15.0, 20.0, 1.0),
        unlit: true,
        ..default()
    });

    let alien_skin_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.6, 0.75),
        metallic: 0.2,
        perceptual_roughness: 0.6,
        emissive: LinearRgba::new(0.05, 0.2, 0.3, 1.0),
        ..default()
    });

    let alien_robe_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.2, 0.75),
        metallic: 0.4,
        perceptual_roughness: 0.5,
        emissive: LinearRgba::new(0.2, 0.05, 0.3, 1.0),
        ..default()
    });

    // --- HIGH ELEVATED OUTPOST PLATFORM (2.0m thickness with 6m solid foundation skirt) ---
    let platform_height = 2.0;
    let platform_center = outpost_pos + Vec3::Y * (platform_height * 0.5);

    // Main top platform surface (visual mesh & physical collider — perfectly aligned with get_floor_and_ceiling)
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(12.0, platform_height))),
        MeshMaterial3d(alien_struct_mat.clone()),
        Transform::from_translation(platform_center),
        avian3d::prelude::RigidBody::Static,
        avian3d::prelude::Collider::cylinder(12.0, platform_height),
        PlayModeEntity,
    ));

    // Deep solid foundation skirt extending 6.0m into the earth so terrain hills NEVER poke through top platform
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(12.5, 6.0))),
        MeshMaterial3d(alien_struct_mat.clone()),
        Transform::from_translation(outpost_pos - Vec3::Y * 2.5),
        PlayModeEntity,
    ));

    // --- ACCESSIBLE HIGH-PLATFORM ENTRANCE RAMPS (North, South, East, West) ---
    // 4 ultra-long angled ramp slabs extending from platform edge (11.5m) deep down into terrain (29.5m)
    let ramp_length = 18.0_f32;
    let ramp_thickness = 0.8_f32;
    let drop_h = 5.0_f32; // 5.0m height drop over 18.0m run
    let pitch_angle = (drop_h / ramp_length).atan(); // positive pitch tilts outer end DOWNWARD into ground

    for i in 0..4 {
        let angle = (i as f32) * std::f32::consts::FRAC_PI_2;
        let dir = Vec3::new(angle.sin(), 0.0, angle.cos());
        let yaw_rot = Quat::from_rotation_y(angle);

        let ramp_center_dist = 20.5_f32; // spans radius 11.5m to 29.5m
        let ramp_center_y = outpost_pos.y - 0.6_f32; // top edge connects at outpost_pos.y + 1.9m (0.1m flush below top platform floor 2.0m)
        let ramp_pos =
            outpost_pos + dir * ramp_center_dist + Vec3::Y * (ramp_center_y - outpost_pos.y);

        let pitch_rot = Quat::from_rotation_x(pitch_angle);
        let ramp_rot = yaw_rot * pitch_rot;

        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(5.2, ramp_thickness, ramp_length))),
            MeshMaterial3d(alien_struct_mat.clone()),
            Transform::from_translation(ramp_pos).with_rotation(ramp_rot),
            avian3d::prelude::RigidBody::Static,
            avian3d::prelude::Collider::cuboid(5.2, ramp_thickness, ramp_length),
            PlayModeEntity,
        ));
    }

    // Energy Totems around perimeter
    for i in 0..4 {
        let angle = (i as f32) * std::f32::consts::FRAC_PI_2 + 0.4;
        let totem_pos =
            outpost_pos + Vec3::new(angle.cos() * 9.5, 1.5 + platform_height, angle.sin() * 9.5);

        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.35, 3.0))),
            MeshMaterial3d(alien_struct_mat.clone()),
            Transform::from_translation(totem_pos),
            avian3d::prelude::RigidBody::Static,
            avian3d::prelude::Collider::cylinder(0.35, 3.0),
            PlayModeEntity,
        ));

        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.4).mesh().ico(4).unwrap())),
            MeshMaterial3d(glow_cyan_mat.clone()),
            Transform::from_translation(totem_pos + Vec3::Y * 1.8),
            PointLight {
                color: Color::srgb(0.1, 1.0, 0.85),
                intensity: 450.0,
                range: 8.0,
                ..default()
            },
            PlayModeEntity,
        ));
    }

    // --- HYDRO-CRYSTAL SHRINE ---
    let shrine_pos = outpost_pos + Vec3::new(0.0, platform_height, -4.0);
    spawn_hydro_shrine(
        commands,
        meshes,
        materials,
        shrine_pos,
        tracker.completed_shrine,
    );

    // --- NATIVE ALIEN NPCs (Tall Elders & Ranger standing ON TOP of platform surface) ---
    // 1. Chieftain Zylar (Tall Outpost Leader Elder — Human Height 2.35x)
    spawn_alien_npc(
        commands,
        meshes,
        asset_server,
        alien_skin_mat.clone(),
        alien_robe_mat.clone(),
        glow_cyan_mat.clone(),
        outpost_pos + Vec3::new(-2.5, platform_height, 1.0),
        "Chieftain Zylar",
        "Outpost Leader",
        QuestId::RebuildShrine,
        2.35,
    );

    // 2. Scout Kael (Tactical Ranger — 2.05x)
    spawn_alien_npc(
        commands,
        meshes,
        asset_server,
        alien_skin_mat.clone(),
        alien_robe_mat.clone(),
        glow_cyan_mat.clone(),
        outpost_pos + Vec3::new(3.0, platform_height, 2.0),
        "Scout Kael",
        "Tactical Ranger",
        QuestId::ThreatAtHatchery,
        2.05,
    );

    // 3. Elder Veyla (Tall Mystic Elder — Human Height 2.35x)
    spawn_alien_npc(
        commands,
        meshes,
        asset_server,
        alien_skin_mat,
        alien_robe_mat,
        glow_cyan_mat,
        outpost_pos + Vec3::new(0.0, platform_height, 3.5),
        "Elder Veyla",
        "Alien Mystic",
        QuestId::AstralCompass,
        2.35,
    );

    inventory_log("👽 Discovered Zolyrian Native Alien Outpost!");
}

// Spawns a 3D Native Alien NPC using official alien.glb 3D character asset + ceremonial staff & accents
#[allow(clippy::too_many_arguments)]
fn spawn_alien_npc(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    asset_server: &Res<AssetServer>,
    skin_mat: Handle<StandardMaterial>,
    _robe_mat: Handle<StandardMaterial>,
    glow_mat: Handle<StandardMaterial>,
    pos: Vec3,
    name: &str,
    role: &str,
    quest_id: QuestId,
    scale: f32,
) {
    let root = commands
        .spawn((
            WorldAssetRoot(asset_server.load("alien.glb#Scene0")),
            Transform::from_translation(pos).with_scale(Vec3::splat(scale)),
            Visibility::Visible,
            InheritedVisibility::default(),
            NativeAlienNPC {
                name: name.to_string(),
                role: role.to_string(),
                quest_id,
                initial_pos: pos,
            },
            PlayModeEntity,
        ))
        .id();

    // Ceremonial Head Crest Fin
    let head = commands
        .spawn((
            Transform::from_xyz(0.0, 1.55, 0.0),
            NativeAlienNPCHead,
            Visibility::Visible,
            InheritedVisibility::default(),
            PlayModeEntity,
        ))
        .id();
    commands.entity(root).add_child(head);

    let crest = commands
        .spawn((
            Mesh3d(meshes.add(Cone {
                radius: 0.1,
                height: 0.35,
            })),
            MeshMaterial3d(glow_mat.clone()),
            Transform::from_xyz(0.0, 0.22, -0.05).with_rotation(Quat::from_rotation_x(-0.4)),
            PlayModeEntity,
        ))
        .id();
    commands.entity(head).add_child(crest);

    // Ceremonial Glowing Staff (attached to Right Hand position of Elder root)
    let staff = commands
        .spawn((
            Mesh3d(meshes.add(Cylinder::new(0.04, 1.8))),
            MeshMaterial3d(skin_mat),
            Transform::from_xyz(0.35, 0.75, 0.15).with_rotation(Quat::from_rotation_x(0.2)),
            NativeAlienNPCStaff,
            PlayModeEntity,
        ))
        .id();
    let orb = commands
        .spawn((
            Mesh3d(meshes.add(Sphere::new(0.13).mesh().ico(4).unwrap())),
            MeshMaterial3d(glow_mat),
            Transform::from_xyz(0.0, 0.95, 0.0),
            PointLight {
                color: Color::srgb(0.2, 1.0, 0.85),
                intensity: 250.0,
                range: 4.5,
                ..default()
            },
            NativeAlienNPCStaffOrb,
            PlayModeEntity,
        ))
        .id();
    commands.entity(staff).add_child(orb);
    commands.entity(root).add_child(staff);
}

// Spawns the Hydro-Crystal Shrine (Ruined or Restored)
pub fn spawn_hydro_shrine(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    shrine_pos: Vec3,
    is_rebuilt: bool,
) {
    let stone_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.35, 0.4),
        metallic: 0.4,
        perceptual_roughness: 0.7,
        ..default()
    });

    let crystal_mat = materials.add(StandardMaterial {
        base_color: if is_rebuilt {
            Color::srgb(0.1, 0.9, 1.0)
        } else {
            Color::srgb(0.4, 0.4, 0.5)
        },
        emissive: if is_rebuilt {
            LinearRgba::new(4.0, 18.0, 24.0, 1.0)
        } else {
            LinearRgba::new(0.1, 0.2, 0.3, 1.0)
        },
        unlit: is_rebuilt,
        ..default()
    });

    let shrine_root = commands
        .spawn((
            Transform::from_translation(shrine_pos),
            Visibility::Visible,
            InheritedVisibility::default(),
            HydroShrine { is_rebuilt },
            PlayModeEntity,
        ))
        .id();

    // Shrine Altar Base
    let altar = commands
        .spawn((
            Mesh3d(meshes.add(Cylinder::new(2.5, 0.6))),
            MeshMaterial3d(stone_mat),
            Transform::from_xyz(0.0, 0.3, 0.0),
            avian3d::prelude::RigidBody::Static,
            avian3d::prelude::Collider::cylinder(2.5, 0.6),
            PlayModeEntity,
        ))
        .id();
    commands.entity(shrine_root).add_child(altar);

    // Central Hydro-Crystal Monument
    let crystal_height = if is_rebuilt { 2.8 } else { 0.8 };
    let crystal = commands
        .spawn((
            Mesh3d(meshes.add(Cone {
                radius: 0.8,
                height: crystal_height,
            })),
            MeshMaterial3d(crystal_mat),
            Transform::from_xyz(0.0, 0.6 + crystal_height * 0.5, 0.0),
            PlayModeEntity,
        ))
        .id();
    commands.entity(shrine_root).add_child(crystal);

    if is_rebuilt {
        let light = commands
            .spawn((
                PointLight {
                    color: Color::srgb(0.1, 0.95, 1.0),
                    intensity: 1200.0,
                    range: 12.0,
                    ..default()
                },
                Transform::from_xyz(0.0, 2.5, 0.0),
                PlayModeEntity,
            ))
            .id();
        commands.entity(shrine_root).add_child(light);
    }
}

// Spawns Harmony Beacons on mountain peaks & cliffs
pub fn spawn_harmony_beacons(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    beacon_locations: &[(u32, Vec3)],
    tracker: &QuestTracker,
) {
    let beacon_base_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.25, 0.35),
        metallic: 0.9,
        perceptual_roughness: 0.3,
        ..default()
    });

    for (id, pos) in beacon_locations.iter() {
        let is_active = (tracker.beacons_activated & (1 << id)) != 0;

        let glow_mat = materials.add(StandardMaterial {
            base_color: if is_active {
                Color::srgb(0.2, 1.0, 0.6)
            } else {
                Color::srgb(0.8, 0.3, 0.1)
            },
            emissive: if is_active {
                LinearRgba::new(3.0, 16.0, 8.0, 1.0)
            } else {
                LinearRgba::new(1.0, 0.2, 0.05, 1.0)
            },
            unlit: true,
            ..default()
        });

        let beacon_root = commands
            .spawn((
                Transform::from_translation(*pos),
                Visibility::Visible,
                InheritedVisibility::default(),
                HarmonyBeacon { id: *id, is_active },
                PlayModeEntity,
            ))
            .id();

        // Pillar Spire
        let spire = commands
            .spawn((
                Mesh3d(meshes.add(Cylinder::new(0.4, 4.0))),
                MeshMaterial3d(beacon_base_mat.clone()),
                Transform::from_xyz(0.0, 2.0, 0.0),
                avian3d::prelude::RigidBody::Static,
                avian3d::prelude::Collider::cylinder(0.4, 4.0),
                PlayModeEntity,
            ))
            .id();
        commands.entity(beacon_root).add_child(spire);

        // Beacon Crystal Crown
        let crown = commands
            .spawn((
                Mesh3d(meshes.add(Sphere::new(0.5).mesh().ico(4).unwrap())),
                MeshMaterial3d(glow_mat),
                Transform::from_xyz(0.0, 4.3, 0.0),
                PointLight {
                    color: if is_active {
                        Color::srgb(0.2, 1.0, 0.6)
                    } else {
                        Color::srgb(1.0, 0.4, 0.1)
                    },
                    intensity: if is_active { 800.0 } else { 200.0 },
                    range: 10.0,
                    ..default()
                },
                PlayModeEntity,
            ))
            .id();
        commands.entity(beacon_root).add_child(crown);
    }
}

pub fn check_and_increment_kill_counter(tracker: &mut QuestTracker) {
    if tracker.active_quest == QuestId::ThreatAtHatchery && tracker.quest_progress < 5 {
        tracker.quest_progress += 1;
        inventory_log(&format!(
            "⚔️ Quest Progress: Defeated Corrupted Beast [{}/5]!",
            tracker.quest_progress
        ));
    }
}

// System to animate Zolyrian Native Alien Elders (idle scanning, facing player when speaking & staff motion)
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn alien_npc_animation_system(
    time: Res<Time>,
    dialogue_state: Res<ActiveDialogueState>,
    player_query: Query<&PlayModePlayer>,
    mut npc_query: Query<
        (Entity, &mut Transform, &NativeAlienNPC, &Children),
        With<NativeAlienNPC>,
    >,
    mut child_transform_query: Query<
        (
            &mut Transform,
            Option<&NativeAlienNPCHead>,
            Option<&NativeAlienNPCStaff>,
        ),
        Without<NativeAlienNPC>,
    >,
    mut light_query: Query<&mut PointLight, With<NativeAlienNPCStaffOrb>>,
) {
    let t = time.elapsed_secs();
    let player_pos = player_query
        .single()
        .map(|p| p.position)
        .unwrap_or(Vec3::ZERO);

    for (_entity, mut npc_trans, npc, children) in npc_query.iter_mut() {
        let is_speaking = dialogue_state.is_open && dialogue_state.npc_name == npc.name;

        // Keep NPC tethered to their assigned position on the platform
        npc_trans.translation.x = npc.initial_pos.x;
        npc_trans.translation.z = npc.initial_pos.z;

        if is_speaking {
            // Smoothly turn body directly face-to-face toward player (+PI 180 deg mesh alignment fix)
            let dir = (player_pos - npc_trans.translation).normalize_or_zero();
            if dir.length_squared() > 1e-4 {
                let target_yaw = dir.x.atan2(dir.z) + std::f32::consts::PI;
                let current_rot = npc_trans.rotation;
                let target_rot = Quat::from_rotation_y(target_yaw);
                npc_trans.rotation =
                    current_rot.slerp(target_rot, (5.0 * time.delta_secs()).min(1.0));
            }

            // Conversational breathing & posture bobbing
            let speak_bob = (t * 3.0).sin() * 0.03;
            npc_trans.translation.y = npc.initial_pos.y + speak_bob;
        } else {
            // Subtle idle posture oscillation
            let idle_bob = (t * 1.2 + (npc.name.len() as f32)).sin() * 0.015;
            npc_trans.translation.y = npc.initial_pos.y + idle_bob;
        }

        // Animate head crest and ceremonial staff using disjoint child query
        for child in children.iter() {
            if let Ok((mut trans, is_head, is_staff)) = child_transform_query.get_mut(child) {
                if is_head.is_some() {
                    if is_speaking {
                        let head_nod = (t * 3.5).sin() * 0.08;
                        let head_tilt = (t * 2.1).cos() * 0.06;
                        trans.rotation = Quat::from_euler(EulerRot::YXZ, head_tilt, head_nod, 0.0);
                    } else {
                        let head_yaw = (t * 0.7 + (npc.name.len() as f32)).sin() * 0.15;
                        trans.rotation = Quat::from_rotation_y(head_yaw);
                    }
                } else if is_staff.is_some() {
                    if is_speaking {
                        let g = (t * 3.0).sin() * 0.08;
                        trans.translation.y = 0.75 + g;
                        trans.rotation = Quat::from_euler(EulerRot::XYZ, 0.2 + g * 0.4, 0.0, 0.0);
                    } else {
                        let idle = (t * 1.5).sin() * 0.03;
                        trans.translation.y = 0.75 + idle;
                        trans.rotation = Quat::from_rotation_x(0.2);
                    }
                }
            }
        }
    }

    for mut light in light_query.iter_mut() {
        if dialogue_state.is_open {
            light.intensity = 450.0 + (t * 5.0).sin() * 150.0;
        } else {
            light.intensity = 250.0 + (t * 2.5).sin() * 50.0;
        }
    }
}
