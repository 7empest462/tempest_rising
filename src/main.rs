use bevy::diagnostic::FrameCount;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};

use bevy::window::WindowResolution;

use avian3d::PhysicsPlugins;
use bevy_panorbit_camera::PanOrbitCameraPlugin;
use bevy_tnua::builtins::{TnuaBuiltinCrouch, TnuaBuiltinJump, TnuaBuiltinWalk};
use bevy_tnua::prelude::{TnuaControllerPlugin, TnuaScheme};
use bevy_tnua_avian3d::TnuaAvian3dPlugin;

#[derive(TnuaScheme)]
#[scheme(basis = TnuaBuiltinWalk)]
pub enum ControlScheme {
    Jump(TnuaBuiltinJump),
    Crouch(TnuaBuiltinCrouch),
}

#[derive(Resource)]
pub struct TokioRuntime(pub tokio::runtime::Handle);

mod character_designer;
mod grass;
mod map_editor;
mod play_mode;
mod procedural_walls;
mod sprite_editor;
mod water;
mod water_gpu;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    MainMenu,
    MapEditor,
    SpriteEditor,
    CharacterDesigner,
    PlayMode,
}

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to build Tokio runtime");
    let handle = rt.handle().clone();
    let _guard = rt.enter();

    App::new()
        .insert_resource(TokioRuntime(handle))
        .add_systems(Startup, setup_cluster_settings)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Tempest Rising".into(),
                resolution: WindowResolution::new(1280, 720),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .init_state::<AppState>()
        .add_plugins((
            grass::GrassPlugin,
            map_editor::MapEditorPlugin,
            sprite_editor::SpriteEditorPlugin,
            character_designer::CharacterDesignerPlugin,
            play_mode::PlayModePlugin,
            procedural_walls::ProceduralWallsPlugin,
            water::WaterPlugin,
        ))
        .add_plugins((
            PhysicsPlugins::default(),
            TnuaControllerPlugin::<ControlScheme>::new(Update),
            TnuaAvian3dPlugin::new(Update),
            PanOrbitCameraPlugin,
        ))
        .add_systems(Startup, setup_main_menu)
        .add_systems(
            EguiPrimaryContextPass,
            main_menu_ui.run_if(in_state(AppState::MainMenu)),
        )
        .run();
}

fn setup_main_menu(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            ..default()
        },
    ));
}

fn main_menu_ui(
    mut contexts: EguiContexts,
    mut next_state: ResMut<NextState<AppState>>,
    frame_count: Res<FrameCount>,
) {
    if frame_count.0 < 2 {
        return;
    }
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    egui::Window::new("Tempest Rising - Launcher")
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.heading("Tempest Rising - Development Tools");
            ui.separator();

            if ui.button("🗺 Open Map Editor").clicked() {
                println!("Open Map Editor clicked!");
                next_state.set(AppState::MapEditor);
            }

            if ui
                .button("🎨 Open Custom Asset Studio (Flags & Reticles)")
                .clicked()
            {
                println!("Open Custom Asset Studio clicked!");
                next_state.set(AppState::SpriteEditor);
            }

            if ui
                .button("🕴 Open Character Designer & Ragdoll Sim")
                .clicked()
            {
                println!("Open Character Designer clicked!");
                next_state.set(AppState::CharacterDesigner);
            }

            if ui.button("🎮 Enter Play Mode").clicked() {
                println!("Enter Play Mode clicked!");
                next_state.set(AppState::PlayMode);
            }
        });
}

fn setup_cluster_settings(
    cluster_settings: Option<ResMut<bevy::light::cluster::GlobalClusterSettings>>,
) {
    if let Some(mut cs) = cluster_settings {
        // Disable GPU clustering on Metal/wgpu to prevent GPU staging buffer DeviceLost panics with many light sources
        cs.gpu_clustering = None;
    }
}
