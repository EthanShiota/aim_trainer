mod crosshair;
mod edit_mode;
mod effects;
mod input;
mod menus;
mod scenarios;
mod scoreing;
mod state;
mod target;

use bevy::audio::AddAudioSource;
use bevy::camera::Exposure;
use bevy::camera::Projection::Perspective;
use bevy::core_pipeline::tonemapping::Tonemapping;
use state::*;

use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::platform::collections::HashMap;
use bevy::post_process::bloom::Bloom;
use bevy::settings::{ReflectSettingsGroup, SaveSettingsSync, SettingsGroup, SettingsPlugin};
use bevy_egui::egui::Widget;
use bevy_egui::input::EguiWantsInput;
use rodio::buffer::SamplesBuffer;
use std::hash::Hash;
use tracing::instrument;

use bevy::light::Skybox;
use bevy::picking::PickingSettings;
use bevy_egui::prelude::*;

use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowMode};

use crate::crosshair::CrosshairMaterial;
use crate::input::{Hovered, InputMessage};
use crate::scoreing::Score;
use crate::target::events::TargetHit;
use crate::target::{
    CurveMarker, CurveMarkerMaterial, Marker, Target, TargetMarker, TargetResource,
};
use fps_camera::{FPSCamera, FPSCameraPlugin, GrabMouse};

#[derive(Resource, Clone, SettingsGroup, Reflect)]
#[reflect(Resource, SettingsGroup, Default)]
#[settings_group(group = "general")]
pub struct GameSettings {
    // in / 360
    pub mouse_sensitivity: f32,
    pub dpi: usize,
    // Vertical Camera fov
    pub fov: f32,
    pub keybinds: HashMap<GameAction, Vec<KeyCode>>,
    pub mousebinds: HashMap<GameAction, Vec<MouseButton>>,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            mouse_sensitivity: 16.351,
            dpi: 800,
            fov: 1.0112001,
            keybinds: [(GameAction::FireWeapon, vec![KeyCode::KeyX, KeyCode::KeyZ])]
                .into_iter()
                .collect(),
            mousebinds: [(GameAction::FireWeapon, vec![MouseButton::Left])]
                .into_iter()
                .collect(),
        }
    }
}

#[derive(Resource, Clone, SettingsGroup, Reflect)]
#[reflect(Resource, SettingsGroup, Default)]
#[settings_group(group = "volume")]
pub struct SoundSettings {
    pub music_volume: f32,
    pub effects_volume: f32,
    // Path to audio asset
    pub sfx_override: Option<String>,
}

impl Default for SoundSettings {
    fn default() -> Self {
        Self {
            music_volume: 1.0,
            effects_volume: 1.0,
            sfx_override: None,
        }
    }
}

#[derive(Resource, Clone, SettingsGroup, Reflect)]
#[reflect(Resource, SettingsGroup, Default)]
#[settings_group(group = "crosshair")]
pub struct CrosshairSettings {
    crosshair: CrosshairMaterial,
}

impl Default for CrosshairSettings {
    fn default() -> Self {
        Self {
            crosshair: default(),
        }
    }
}

#[derive(Resource, Clone, SettingsGroup, Reflect)]
#[reflect(Resource, SettingsGroup, Default)]
#[settings_group(group = "gameplay")]
pub struct GameplaySettings {
    target_scale: f32,
    target_distance: f32,
}

impl Default for GameplaySettings {
    fn default() -> Self {
        Self {
            target_scale: 1.0,
            target_distance: 50.,
        }
    }
}

#[derive(Hash, Eq, PartialEq, Clone, Reflect)]
pub enum GameAction {
    FireWeapon,
}

fn main() {
    App::new()
        .insert_resource(GrabMouse(true))
        .insert_resource(PickingSettings {
            is_enabled: true,
            is_input_enabled: true,
            is_hover_enabled: true,
            is_window_picking_enabled: false,
            ..default()
        })
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Window {
                        title: "Aim Game".to_string(),
                        resizable: true,
                        present_mode: bevy::window::PresentMode::Mailbox,
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: true,
                        ..default()
                    }
                    .into(),
                    ..default()
                })
                .set(AssetPlugin {
                    watch_for_changes_override: Some(true),
                    unapproved_path_mode: bevy::asset::UnapprovedPathMode::Deny,
                    ..default()
                }),
            FrameTimeDiagnosticsPlugin::default(),
        ))
        .add_plugins((
            FPSCameraPlugin,
            EguiPlugin::default(),
            edit_mode::EditPlugin,
            menus::MenuPlugin,
            scoreing::ScoringPlugin,
            input::GameInputPlugin,
            effects::EffectPlugin,
            target::TargetPlugin,
            state::StatePlugin,
        ))
        .add_plugins(SettingsPlugin::new("com.github.EthanShiota.aim_trainer"))
        .init_resource::<GameSettings>()
        .init_resource::<GameplaySettings>()
        .add_plugins(UiMaterialPlugin::<CrosshairMaterial>::default())
        // INFO: Audio buffer for quick scrubbing
        .insert_resource(Assets::<AudioBuffer>::default())
        .add_audio_source::<AudioBuffer>()
        // INFO: Setup world when entering game
        // lights + camera + ui
        .add_systems(OnEnter(AppState::InGame), (setup_ui, terrian))
        .add_systems(Update, bevy::ui::widget::update_viewport_render_target_size)
        // INFO: Egui context systems
        .add_systems(
            EguiPrimaryContextPass,
            (debug_window.run_if(resource_equals(target::DebugMode(true))))
                .run_if(in_state(AppState::InGame)),
        )
        // INFO: Sync settings to game systems
        .add_systems(OnExit(AppState::Menu), sync_game_settings)
        .add_systems(Startup, light_and_cameras)
        .run();
}

#[derive(Component, Default, Clone)]
struct PlayerCamera;

fn debug_window(
    mut contexts: EguiContexts,
    target_resource: Option<ResMut<TargetResource>>,
    q_targets: Query<(
        Entity,
        &Marker,
        Option<&CurveMarker>,
        Option<&TargetMarker>,
        Option<&ChildOf>,
    )>,
    curve_marker_mat: Res<Assets<CurveMarkerMaterial>>,
    q_mat: Query<&MeshMaterial3d<CurveMarkerMaterial>>,
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    score: Option<Res<Score>>,
) -> Result {
    if let Some(mut target_resource) = target_resource {
        egui::Window::new("Target Resource").show(contexts.ctx_mut()?, |ui| {
            let TargetResource {
                ring_start,
                ring_end,
                mesh: _,
                easing: _,
                color_curve: _,
            } = target_resource.as_mut();

            egui::Slider::new(ring_start, 0.0..=3.0)
                .text("Ring Start")
                .ui(ui);
            egui::Slider::new(ring_end, 0.0..=3.0)
                .text("Ring End")
                .ui(ui);
        });
    }

    egui::Window::new("Game State")
        .scroll([false, true])
        .max_height(400.)
        .show(contexts.ctx_mut()?, |ui| -> Result {
            ui.label(format!("Time: {:?}", time.elapsed()));
            for (entity, marker, curve, target, _parent) in q_targets.into_iter().sort::<&Marker>()
            {
                let marker_type = match (curve, target) {
                    (Some(_), Some(_)) => return Err("Marker with both types".into()),
                    (Some(_), _) => "Curve",
                    (_, Some(_)) => "Point",
                    (None, None) => return Err("Marker without marker type".into()),
                };
                if ui
                    .button(format!("{marker_type} Marker: {:?}", marker.time()))
                    .clicked()
                {
                    if let Some(p) = _parent {
                        let mat = q_mat.get(p.parent()).unwrap();
                        let mat = curve_marker_mat.get(mat.id()).unwrap();
                        debug!(?mat);
                    }
                    commands.entity(entity).log_components();
                }
            }
            Ok(())
        });
    Ok(())
}

#[derive(Resource)]
pub struct Crosshair(pub Handle<CrosshairMaterial>);

fn setup_ui(
    mut commands: Commands,
    mut crosshair_materials: ResMut<Assets<CrosshairMaterial>>,
    crosshair_settings: Res<CrosshairSettings>,
) -> Result {
    let crosshair = crosshair_materials.add(crosshair_settings.crosshair.clone());
    commands.spawn_scene(bsn! {
        MaterialNode<CrosshairMaterial>({crosshair.clone()})
        Node {
            width: percent(100.),
            height: percent(100.)
        }
    });
    commands.insert_resource(Crosshair(crosshair));

    // commands.spawn((
    //     DiagnosticsOverlay {
    //         title: "Fps".into(),
    //         items: vec![
    //             DiagnosticPath::new("fps").into(),
    //             DiagnosticPath::new("frame_time").into(),
    //         ],
    //     },
    //     DespawnOnExit(AppState::InGame),
    // ));

    Ok(())
}

#[derive(Asset, Resource, Deref, DerefMut, TypePath)]
struct AudioBuffer(SamplesBuffer);

impl Decodable for AudioBuffer {
    type Decoder = SamplesBuffer;
    fn decoder(&self) -> Self::Decoder {
        self.0.clone()
    }
}

fn terrian(asset_server: ResMut<AssetServer>, mut commands: Commands) {
    let gltf_scene: Handle<WorldAsset> =
        asset_server.load(GltfAssetLabel::Scene(0).from_asset("terrain/terrian.glb"));
    commands.spawn_scene(bsn! {
        #Terrain
        WorldAssetRoot(gltf_scene)
        Transform {
            scale: Vec3::splat(10.)
        }
        DespawnOnExit::<AppState>(AppState::InGame)
    });
}
fn light_and_cameras(
    mut commands: Commands,
    game_settings: Res<GameSettings>,
    asset_server: ResMut<AssetServer>,
) {
    let skybox: Handle<Image> =
        asset_server.load("textures/skybox/NightSky008/NightSkyHDRI008_8K_HDR_skybox.ktx2");
    let diffuse: Handle<Image> =
        asset_server.load("textures/skybox/NightSky008/NightSkyHDRI008_8K_HDR_diffuse.ktx2");
    let specular: Handle<Image> =
        asset_server.load("textures/skybox/NightSky008/NightSkyHDRI008_8K_HDR_specular.ktx2");

    commands.spawn((
        Camera3d::default(),
        Camera {
            is_active: true,
            ..default()
        },
        Perspective(PerspectiveProjection {
            fov: game_settings.fov,
            ..default()
        }),
        (
            Skybox {
                image: Some(skybox),
                brightness: 40.,
                ..default()
            },
            EnvironmentMapLight {
                diffuse_map: diffuse,
                specular_map: specular,
                intensity: 5.,
                ..default()
            },
            Bloom::NATURAL,
            Tonemapping::Linear,
            Exposure { ev100: 8. },
            Msaa::Off,
        ),
        Transform::from_xyz(0., 5., 0.),
        PlayerCamera,
        FPSCamera::default(),
        DisableOnExit(AppState::InGame),
        EnableOnEnter(AppState::InGame),
    ));
}

/// Syncs Game Settings from resource to wherever they need to be applied
/// Also saves to settings file
fn sync_game_settings(
    settings: Res<GameSettings>,
    sound_settings: Res<SoundSettings>,
    mut fps_config: ResMut<fps_camera::FPSCameraConfig>,
    mut q_sink: Query<&mut AudioSink, With<AudioPlayer<AudioBuffer>>>,
    mut commands: Commands,
) {
    let new_sens = fps_camera::FPSCameraConfig::with_sens(settings.mouse_sensitivity, settings.dpi);
    fps_config.sensitivity = new_sens.sensitivity;

    for mut sink in q_sink.iter_mut() {
        sink.set_volume(bevy::audio::Volume::Linear(sound_settings.music_volume));
    }
    commands.queue(SaveSettingsSync::IfChanged);
}
