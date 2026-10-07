mod crosshair;
mod edit_mode;
mod effects;
mod input;
mod menus;
mod scenarios;
mod scoreing;
mod target;

use bevy::anti_alias::smaa::Smaa;
use bevy::audio::AddAudioSource;
use bevy::camera::Exposure;
use bevy::camera::Projection::Perspective;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::dev_tools::diagnostics_overlay::{DiagnosticsOverlay, DiagnosticsOverlayPlugin};
use bevy::diagnostic::{DiagnosticPath, FrameTimeDiagnosticsPlugin};
use bevy::platform::collections::HashMap;
use bevy::post_process::bloom::Bloom;
use bevy::settings::{ReflectSettingsGroup, SaveSettingsSync, SettingsGroup, SettingsPlugin};
use bevy_egui::egui::Widget;
use rodio::buffer::SamplesBuffer;
use std::hash::Hash;
use std::time::Duration;
use tracing::instrument;

use bevy::camera::visibility::RenderLayers;
use bevy::light::Skybox;
use bevy::picking::PickingSettings;
use bevy_egui::prelude::*;

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow, WindowMode};

use crate::crosshair::CrosshairMaterial;
use crate::input::{Hovered, InputMessage};
use crate::scoreing::Score;
use crate::target::events::{TargetDestroyed, TargetHit};
use crate::target::{Marker, Target, TargetResource};
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
}

impl Default for SoundSettings {
    fn default() -> Self {
        Self {
            music_volume: 1.0,
            effects_volume: 1.0,
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
        .insert_resource(GlobalUiDebugOptions {
            enabled: false,
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
            FPSCameraPlugin,
            FrameTimeDiagnosticsPlugin::default(),
            DiagnosticsOverlayPlugin,
            EguiPlugin::default(),
            edit_mode::EditPlugin,
            menus::MenuPlugin,
            scoreing::ScoringPlugin,
            input::GameInputPlugin,
            effects::EffectPlugin,
        ))
        .add_plugins(SettingsPlugin::new("com.github.EthanShiota.aim_trainer"))
        .init_resource::<GameSettings>()
        .add_plugins(UiMaterialPlugin::<CrosshairMaterial>::default())
        // INFO: State
        .insert_state(AppState::Menu)
        .add_sub_state::<GameState>()
        .add_sub_state::<EditMode>()
        // INFO: Audio buffer for quick scrubbing
        .insert_resource(Assets::<AudioBuffer>::default())
        .add_audio_source::<AudioBuffer>()
        // INFO: Target plugin
        .add_plugins(target::TargetPlugin)
        // INFO: Setup world when entering game
        // lights + camera + ui
        .add_systems(OnEnter(AppState::InGame), (setup_ui, terrian))
        // INFO: Setup main menu
        // .add_systems(OnEnter(AppState::Menu), main_menu.spawn())
        .add_systems(OnEnter(GameState::Paused), transition::pause)
        .add_systems(OnEnter(GameState::Playing), transition::play)
        .add_systems(Update, bevy::ui::widget::update_viewport_render_target_size)
        // INFO: Game Logic loops
        .add_systems(
            Update,
            (
                global_bindings,
                game_loop.run_if(in_state(AppState::InGame)),
                playing_binds.run_if(in_state(GameState::Playing)),
            ),
        )
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

#[derive(Debug, Hash, Resource, Eq, PartialEq, PartialOrd, States, Clone, Default)]
enum AppState {
    InGame,
    #[default]
    Menu,
}

#[derive(SubStates, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Debug)]
#[source(AppState = AppState::InGame)]
enum GameState {
    Paused,
    #[default]
    Playing,
}

#[derive(SubStates, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Debug)]
#[source(AppState = AppState::InGame)]
enum EditMode {
    Editing,
    #[default]
    Normal,
}

#[derive(Component, Default, Clone)]
struct PlayerCamera;

fn playing_binds(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    mut q_curve: Query<(Entity, &mut Target), (With<Hovered>, With<target::Active>)>,
    q_point: Query<(Entity, &Marker), (With<Target>, With<Hovered>)>,
    mut reader: PopulatedMessageReader<InputMessage>,
) {
    if reader.len() > 1 {
        warn!("more messages then expected!");
    }
    for msg in reader.read() {
        match msg {
            InputMessage::FireWeapon => fire_weapon(q_point, &mut commands),
            InputMessage::FireWeaponHeld => {
                fire_weapon_held(time.delta(), &mut q_curve, &mut commands)
            }
        }
    }
}

#[instrument(skip_all)]
fn fire_weapon(
    q_hit: Query<(Entity, &Marker), (With<Target>, With<Hovered>)>,
    commands: &mut Commands,
) {
    let mut hits: Vec<_> = q_hit.into_iter().collect();
    hits.sort_by_key(|elm| elm.1);
    debug!(hits = ?hits);
    if let Some((t, _)) = hits.first() {
        commands.entity(*t).trigger(TargetHit);
    }
}

fn fire_weapon_held(
    delta: Duration,
    q_hit: &mut Query<(Entity, &mut Target), (With<Hovered>, With<target::Active>)>,
    commands: &mut Commands,
) {
    for (entity, mut target) in q_hit.iter_mut() {
        if let Target::Duration(dur) = target.as_mut() {
            *dur = dur.saturating_sub(delta);
            if dur.is_zero() {
                commands.entity(entity).trigger(TargetDestroyed);
            }
        }
    }
}

fn global_bindings(
    key_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut ui_debug: ResMut<GlobalUiDebugOptions>,
) {
    // Global
    if key_input.just_pressed(KeyCode::KeyF) {
        commands.run_system_cached(toggle_fullscreen);
    }

    if key_input.just_pressed(KeyCode::Equal) {
        ui_debug.enabled.toggle();
    }
}

fn game_loop(
    mut commands: Commands,
    key_input: Res<ButtonInput<KeyCode>>,
    mut debug_mode: ResMut<target::DebugMode>,
    game_state: Res<State<GameState>>,
) {
    let game_state = game_state.get();

    if key_input.just_pressed(KeyCode::Slash) {
        debug_mode.0 = !debug_mode.0;
    }
    if key_input.just_pressed(KeyCode::Escape) {
        match game_state {
            GameState::Playing => {
                commands.set_state(GameState::Paused);
            }
            GameState::Paused => commands.set_state(GameState::Playing),
        }
    }
}

fn debug_window(
    mut contexts: EguiContexts,
    target_resource: Option<ResMut<TargetResource>>,
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

    if let Some(score) = score {
        egui::Window::new("score").show(contexts.ctx_mut()?, |ui| {
            ui.label(format!("points: {}", score.points));
        });
    }
    Ok(())
}
pub mod transition {

    use super::*;

    pub(crate) fn play(
        mut cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
        mut grab_mode: ResMut<GrabMouse>,
        mut time: ResMut<Time<Virtual>>,
        q_sink: Query<(&mut AudioSink, &AudioPlayer<AudioBuffer>)>,
    ) {
        q_sink.into_iter().for_each(|sink| sink.0.play());
        time.unpause();
        cursor_options.grab_mode = CursorGrabMode::Locked;
        cursor_options.visible = false;
        **grab_mode = true;
    }
    pub(crate) fn pause(
        mut commands: Commands,
        mut cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
        mut grab_mode: ResMut<GrabMouse>,
        mut time: ResMut<Time<Virtual>>,
        q_sink: Query<(&mut AudioSink, &AudioPlayer<AudioBuffer>)>,
    ) {
        q_sink.into_iter().for_each(|sink| sink.0.pause());
        time.pause();
        **grab_mode = false;
        cursor_options.grab_mode = CursorGrabMode::None;
        cursor_options.visible = true;
        commands.spawn_scene(menus::pause_menu());
    }
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

    commands.spawn((
        DiagnosticsOverlay {
            title: "Fps".into(),
            items: vec![
                DiagnosticPath::new("fps").into(),
                DiagnosticPath::new("frame_time").into(),
            ],
        },
        DespawnOnExit(AppState::InGame),
    ));

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
        RenderLayers::from_layers(&[0, 1]),
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
            Tonemapping::AgX,
            Exposure { ev100: 8. },
            Msaa::Off,
            Smaa::default(),
        ),
        Transform::from_xyz(0., 5., 0.),
        PlayerCamera,
        FPSCamera::default(),
        DisableOnExit(AppState::InGame),
        EnableOnEnter(AppState::InGame),
    ));
}

fn toggle_fullscreen(
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    mut maximized: Local<bool>,
) {
    *maximized = !*maximized;
    if *maximized {
        window.mode = WindowMode::BorderlessFullscreen(MonitorSelection::Current);
    } else {
        window.mode = WindowMode::Windowed;
    }
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
