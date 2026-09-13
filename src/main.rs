#![recursion_limit = "256"]
mod edit_mode;
mod effects;
mod fps_camera;
mod materials;
mod menus;
mod scenarios;
mod scoreing;
mod target_plugin;

use bevy::audio::AddAudioSource;
use bevy::color::palettes::tailwind::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::time::Stopwatch;
use bevy_egui::egui::Widget;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use rodio::Source;
use rodio::buffer::SamplesBuffer;
use std::hash::Hash;
use std::path::PathBuf;
use std::time::Duration;

use bevy::camera::visibility::RenderLayers;
use bevy::color::palettes::css::{self};
use bevy::light::VolumetricLight;
use bevy::light::atmosphere::ScatteringMedium;
use bevy::light::light_consts::lux;
use bevy::picking::PickingSettings;
use bevy_egui::prelude::*;
use bevy_skein::SkeinPlugin;

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow, WindowMode};

use crate::fps_camera::{FPSCamera, FPSCameraPlugin, GrabMouse, Hovered};
use crate::target_plugin::events::{TargetDestroyed, TargetHit};
use crate::target_plugin::{BeatMap, DebugMode, Marker, Target, TargetResource};

#[derive(Resource, Clone, Copy)]
pub struct GameSettings {
    pub volume: f32,
    pub mouse_sensitivity: f32,
    pub dpi: usize,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            volume: 1.0,
            mouse_sensitivity: 16.351,
            dpi: 800,
        }
    }
}

#[derive(Resource, PartialEq, Debug, Default, DerefMut, Deref)]
pub struct SceneTimer(Stopwatch);

#[derive(Resource, Deref)]
pub struct BeatMapPath(PathBuf);

fn main() {
    App::new()
        .insert_resource(SceneTimer::default())
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
        .insert_resource(GameSettings::default())
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Window {
                        title: "Aim Game".to_string(),
                        resizable: true,
                        present_mode: bevy::window::PresentMode::Immediate,
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: true,
                        ..default()
                    }
                    .into(),
                    ..default()
                })
                .set(AssetPlugin {
                    watch_for_changes_override: Some(true),
                    ..default()
                }),
            FPSCameraPlugin,
            SkeinPlugin::default(),
            EguiPlugin::default(),
            edit_mode::EditPlugin,
            menus::MenuPlugin,
            scoreing::ScoringPlugin,
            WorldInspectorPlugin::default().run_if(resource_equals(DebugMode(true))),
        ))
        .add_plugins(MaterialPlugin::<SkyMaterial>::default())
        // INFO: State
        .insert_state(AppState::Menu)
        .add_sub_state::<GameState>()
        .add_sub_state::<EditMode>()
        // INFO: Audio buffer for quick scrubbing
        .insert_resource(Assets::<AudioBuffer>::default())
        .add_audio_source::<AudioBuffer>()
        // INFO: Target plugin
        .add_plugins(target_plugin::TargetPlugin)
        // INFO: Setup world when entering game
        // lights + camera + ui
        .add_systems(
            OnEnter(AppState::InGame),
            (light_and_cameras, setup_ui).chain(),
        )
        // INFO: Setup main menu
        // .add_systems(OnEnter(AppState::Menu), main_menu.spawn())
        .add_systems(OnEnter(GameState::Paused), transition::pause)
        .add_systems(OnEnter(GameState::Playing), transition::play)
        .add_systems(
            Update,
            (bevy::ui::widget::update_viewport_render_target_size,),
        )
        // INFO: Game Logic loops
        .add_systems(
            Update,
            (
                global_bindings,
                game_loop.run_if(in_state(AppState::InGame)),
                playing.run_if(in_state(GameState::Playing)),
            ),
        )
        // INFO: Egui context systems
        .add_systems(
            EguiPrimaryContextPass,
            (debug_window.run_if(resource_equals(target_plugin::DebugMode(true))))
                .run_if(in_state(AppState::InGame)),
        )
        // INFO: Sync settings to game systems
        .add_systems(OnExit(AppState::Menu), sync_game_settings)
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

fn main_menu() -> impl Scene {
    bsn! {
        DespawnOnExit::<AppState>(AppState::Menu)
        Camera2d
        Node {
            width: percent(100.),
            height: percent(100.),
            display: Display::Flex,
            align_content: AlignContent::Center,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            flex_direction: FlexDirection::Column,
        }
        BackgroundColor(RED_100)
        Children [
            (
                menu_button("Play")
                on(|_e: On<Pointer<Press>>, _commands: Commands, _path: Option<Res<BeatMapPath>>| {
                    // let f = FileDialog::default().set_directory("/").pick_file().unwrap();
                    // commands.insert_resource(BeatMapPath(f));
                    // commands.set_state(AppState::InGame);
                    // commands.run_system_cached(scenarios::osu);
                })
            ),
            (
                menu_button("Exit")
            )

        ]
    }
}

fn menu_button(text: &'static str) -> impl Scene {
    bsn! {
        Node {
            min_width: px(200.),
            min_height: px(100.),
            width: percent(40.),
            height: percent(20.),
            align_content: AlignContent::Center,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            display: Display::Flex,
            margin: px(30.),
            border: px(4.)
        }
        BorderColor::all(FUCHSIA_300)
        // on(|e: On<Pointer<Press>>, mut commands: Commands|{
        //     commands.entity(e.entity).insert(BackgroundColor(GREEN_300.into()));
        // })
        // on(|e: On<Pointer<Release>>, mut commands: Commands|{
        //     commands.entity(e.entity).insert(BackgroundColor(VIOLET_500.into()));
        // })
        Button
        BackgroundColor(VIOLET_500)
        Children [
            Text(text)
        ]
    }
}

fn music_controls(
    mut contexts: EguiContexts,
    mut q_sink: Query<(&mut AudioSink, &AudioPlayer<AudioBuffer>)>,
    sources: Res<Assets<AudioBuffer>>,
    mut should_resume: Local<bool>,
    mut stopwatch: ResMut<SceneTimer>,
) -> Result {
    egui::Window::new("Controls").show(contexts.ctx_mut()?, |ui| {
        for (sink, audio_player) in q_sink.iter_mut() {
            let mut value = sink.position().as_secs_f64();
            let max = sources
                .get(&audio_player.0)
                .unwrap()
                .total_duration()
                .unwrap();
            let slider = ui.add(egui::Slider::new(&mut value, 0.0..=max.as_secs_f64()));
            if slider.changed() {
                _ = sink
                    .try_seek(Duration::from_secs_f64(value))
                    .inspect_err(|err| println!("{err:?}"));
                stopwatch.set_elapsed(sink.position());
            }
            if slider.drag_started() {
                *should_resume = !sink.is_paused();
                sink.pause();
            }
            if slider.drag_stopped() && *should_resume {
                sink.play();
            }
        }
    });
    Ok(())
}

fn playing(
    mut commands: Commands,
    mouse_input: Res<ButtonInput<MouseButton>>,
    time: Res<Time<Virtual>>,
    _q_audio: Query<&AudioSink, With<AudioPlayer<AudioBuffer>>>,
    _stopwatch: ResMut<SceneTimer>,
) {
    if mouse_input.just_pressed(MouseButton::Left) {
        commands.run_system_cached(fire_weapon);
    } else if mouse_input.pressed(MouseButton::Left) {
        commands.run_system_cached_with(fire_weapon_held, time.delta());
    }
}

fn global_bindings(key_input: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    // Global
    if key_input.just_pressed(KeyCode::KeyF) {
        commands.run_system_cached(toggle_fullscreen);
    }
}

fn game_loop(
    mut commands: Commands,
    key_input: Res<ButtonInput<KeyCode>>,
    mut debug_mode: ResMut<target_plugin::DebugMode>,
    game_state: Res<State<GameState>>,
    q_sink: Query<(&mut AudioSink, &AudioPlayer<AudioBuffer>)>,
) {
    let game_state = game_state.get();
    for (sink, _) in q_sink {
        if *game_state == GameState::Paused {
            sink.pause();
        } else {
            sink.play();
        }
    }

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
    beat_map: Option<Res<BeatMap>>,
    target_resource: Option<ResMut<TargetResource>>,
) -> Result {
    egui::Window::new("Beatmap Debug Inspector").show(contexts.ctx_mut()?, |ui| {
        if let Some(beat_map) = beat_map {
            let hit_markers = beat_map.target_markers.clone();
            egui::ScrollArea::new([false, true])
                .max_height(400.)
                .show(ui, |ui| {
                    for (_target_marker, marker, transform) in hit_markers {
                        ui.label(format!("{:?} at {:?}", marker, transform.translation));
                    }
                });
        }
    });

    if let Some(mut target_resource) = target_resource {
        egui::Window::new("Target Resource").show(contexts.ctx_mut()?, |ui| {
            let TargetResource {
                ring_start,
                ring_end,
                mesh: _,
                easing: _,
            } = target_resource.as_mut();

            egui::Slider::new(ring_start, 0.0..=3.0)
                .text("Ring Start")
                .ui(ui);
            egui::Slider::new(ring_end, 0.0..=3.0)
                .text("Ring End")
                .ui(ui);
        });
    }
    Ok(())
}
pub mod transition {
    use super::*;

    pub fn play(
        mut cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
        mut grab_mode: ResMut<GrabMouse>,
        mut time: ResMut<Time<Virtual>>,
    ) {
        time.unpause();
        cursor_options.grab_mode = CursorGrabMode::Locked;
        cursor_options.visible = false;
        **grab_mode = true;
    }
    pub fn pause(
        mut commands: Commands,
        mut cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
        mut grab_mode: ResMut<GrabMouse>,
        mut time: ResMut<Time<Virtual>>,
    ) {
        time.pause();
        **grab_mode = false;
        cursor_options.grab_mode = CursorGrabMode::None;
        cursor_options.visible = true;
        commands.spawn_scene(bsn! {
            Node {
                display: Display::Flex, justify_content: JustifyContent::Center, align_items: AlignItems::Center, width: percent(100.), height: percent(100.)
            }
            DespawnOnExit::<GameState>(GameState::Paused)
                Children [
                Node { flex_direction: FlexDirection::Column, width: percent(20.), height: percent(20.), align_items: AlignItems::Center, justify_content: JustifyContent::Center}
            BorderColor::all(css::BLACK)
                Children [
                (
                    Node {width: percent(100.), align_items: AlignItems::Center}
                    on(|_: On<Pointer<Press>>, mut commands: Commands| {
                        commands.set_state(GameState::Playing);
                    })
                    Text::new("Exit")
                ),
                (
                    Node {width: percent(100.), align_items: AlignItems::Center}
                    on(|_: On<Pointer<Press>>, mut commands: Commands| {
                        commands.set_state(AppState::Menu);
                    })
                    Text::new("Main Menu")
                )
                ]
                ]
        });
    }
}

fn setup_ui(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut meterials: ResMut<Assets<ColorMaterial>>,
) -> Result {
    // Spawn crosshair
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(3.))),
        MeshMaterial2d(meterials.add(Color::WHITE)),
        Transform::default(),
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

#[derive(AsBindGroup, Debug, Clone, Asset, Reflect)]
pub struct SkyMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
}

impl Material for SkyMaterial {
    fn fragment_shader() -> bevy::shader::ShaderRef {
        "shaders/SkyMaterial.wgsl".into()
    }
}

fn light_and_cameras(
    mut commands: Commands,
    _scattering_mediums: ResMut<Assets<ScatteringMedium>>,
    _images: ResMut<Assets<Image>>,
    _asset_server: ResMut<AssetServer>,
) {
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            // lux::RAW_SUNLIGHT is recommended for use with this feature, since
            // other values approximate sunlight *post-scattering* in various
            // conditions. RAW_SUNLIGHT in comparison is the illuminance of the
            // sun unfiltered by the atmosphere, so it is the proper input for
            // sunlight to be filtered by the atmosphere.
            illuminance: lux::RAW_SUNLIGHT,
            ..default()
        },
        Transform::from_xyz(1.0, 0.4, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
        VolumetricLight,
    ));

    commands.spawn_scene(bsn! {
        #Sky
        Mesh3d(asset_value(Sphere::new(1000.).mesh().ico(7).unwrap().with_inverted_winding().unwrap()))
        MeshMaterial3d::<SkyMaterial>(asset_value(SkyMaterial {
            color: RED_100.into()
        }))
    });

    commands.spawn((
        Camera3d::default(),
        RenderLayers::from_layers(&[0, 1]),
        Camera {
            is_active: true,
            ..default()
        },
        Transform::from_xyz(0., 5., 0.),
        PlayerCamera,
        FPSCamera::default(),
        DespawnOnExit(AppState::InGame),
    ));

    commands.spawn((
        Name::new("UI Camera"),
        DespawnOnExit(AppState::InGame),
        Camera2d,
        Camera {
            order: 1,
            is_active: true,
            clear_color: ClearColorConfig::None,
            ..default()
        },
    ));
}

fn fire_weapon(
    q_hit: Query<(Entity, &Marker), (With<Target>, With<Hovered>)>,
    mut commands: Commands,
) {
    let mut hits: Vec<_> = q_hit.into_iter().collect();
    hits.sort_by_key(|elm| elm.1);
    if let Some((t, _)) = hits.first() {
        commands.entity(*t).trigger(TargetHit);
    }
}

fn fire_weapon_held(
    In(delta): In<Duration>,
    mut q_hit: Query<(Entity, &mut Target), (With<Hovered>, With<target_plugin::Active>)>,
    mut commands: Commands,
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

fn sync_game_settings(
    settings: Res<GameSettings>,
    mut fps_config: ResMut<crate::fps_camera::FPSCameraConfig>,
    mut q_sink: Query<&mut AudioSink, With<AudioPlayer<AudioBuffer>>>,
) {
    let new_sens =
        crate::fps_camera::FPSCameraConfig::with_sens(settings.mouse_sensitivity, settings.dpi);
    fps_config.sensitivity = new_sens.sensitivity;

    for mut sink in q_sink.iter_mut() {
        sink.set_volume(bevy::audio::Volume::Linear(settings.volume));
    }
}
