mod fps_camera;
mod materials;
mod music;
mod scenarios;
mod target_spawner;

use bevy::audio::AddAudioSource;
use bevy::color::palettes::tailwind::*;
use bevy::ecs::system::ObserverSystem;
use bevy::log::LogPlugin;
use bevy::time::Stopwatch;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use rodio::Source;
use rodio::buffer::SamplesBuffer;
use std::fs::File;
use std::hash::Hash;
use std::time::Duration;
use tracing::Instrument;

use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{Exposure, RenderTarget};
use bevy::camera_controller::free_camera::{FreeCamera, FreeCameraPlugin};
use bevy::color::palettes::css;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::atmosphere::ScatteringMedium;
use bevy::light::light_consts::lux;
use bevy::light::{Atmosphere, AtmosphereEnvironmentMapLight, VolumetricLight};
use bevy::picking::PickingSettings;
use bevy::reflect::DynamicTypePath;
use bevy_egui::prelude::*;
use bevy_skein::SkeinPlugin;

use bevy::pbr::{AtmosphereSettings, DefaultOpaqueRendererMethod, ScreenSpaceReflections};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::text::TextSection;
use bevy::window::{PresentMode, PrimaryWindow, WindowMode, WindowRef, WindowResolution};

use target_spawner::FireWeapon;

use crate::fps_camera::{FPSCamera, FPSCameraPlugin};
use crate::target_spawner::TargetDestroyed;

#[derive(Resource, Default, Clone, Copy)]
struct GameStats {
    score: usize,
    time_left: Option<Duration>,
}

#[derive(Resource, PartialEq, Debug, Default, DerefMut, Deref)]
pub struct SceneTimer(Stopwatch);

fn main() {
    App::new()
        .insert_resource(DefaultOpaqueRendererMethod::deferred())
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(GlobalAmbientLight::NONE)
        .insert_resource(SceneTimer::default())
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
            // FPSCameraPlugin,
            FreeCameraPlugin,
            SkeinPlugin::default(),
            EguiPlugin::default(),
            WorldInspectorPlugin::new().run_if(resource_equals(target_spawner::DebugMode(true))),
        ))
        // INFO: State
        .insert_state(AppState::Menu)
        .add_sub_state::<GameState>()
        .add_sub_state::<EditMode>()
        // INFO: Audio buffer for quick scrubbing
        .insert_resource(Assets::<AudioBuffer>::default())
        .add_audio_source::<AudioBuffer>()
        // INFO: Target plugin
        .add_plugins(target_spawner::TargetPlugin)
        // INFO: Setup world when entering game
        // lights + camera + ui
        .add_systems(
            OnEnter(AppState::InGame),
            (light_and_cameras, setup_ui).chain(),
        )
        // INFO: Setup main menu
        .add_systems(OnEnter(AppState::Menu), main_menu.spawn())
        .add_systems(
            Update,
            (
                // TODO: Make better score ui
                update_ui,
                bevy::ui::widget::update_viewport_render_target_size,
            ),
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
        .add_systems(OnEnter(GameState::Paused), pause_menu.spawn())
        // INFO: Egui context systems
        .add_systems(
            EguiPrimaryContextPass,
            (
                music_controls,
                debug_window.run_if(resource_equals(target_spawner::DebugMode(true))),
            )
                .run_if(in_state(AppState::InGame)),
        )
        // INFO: Update score when target is destroyed
        .add_observer(update_score)
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
struct ScoreUI;

#[derive(Component)]
struct PlayerCamera;

fn main_menu() -> impl Scene {
    bsn! {
        DespawnOnExit::<_>(AppState::Menu)
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
                on(|e: On<Pointer<Press>>, mut commands: Commands| {
                    log::info!("press!");
                    commands.set_state(AppState::InGame);
                    commands.run_system_cached(scenarios::basic);
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

fn score_menu(stats: GameStats) -> impl Scene {
    let score = format!("{}", stats.score);
    bsn! {
        Node {display: Display::Flex, width: percent(100.), height: percent(100.), justify_content: JustifyContent::Center, align_items: AlignItems::Center}
        TextFont {font_size: px(200.)}
        BackgroundColor(VIOLET_100)
        Text(score)
    }
}

fn music_controls(
    mut contexts: EguiContexts,
    mut q_sink: Query<(&mut AudioSink, &AudioPlayer<AudioBuffer>)>,
    sources: Res<Assets<AudioBuffer>>,
    mut should_resume: Local<bool>,
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
            }
            if slider.drag_started() {
                *should_resume = !sink.is_paused();
                sink.pause();
            }
            if slider.drag_stopped() {
                if *should_resume {
                    sink.play();
                }
            }
        }
    });
    Ok(())
}

fn playing(
    mut commands: Commands,
    game_stats: Option<ResMut<GameStats>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    time: Res<Time<Real>>,
    q_audio: Query<&AudioSink, With<AudioPlayer<AudioBuffer>>>,
    mut stopwatch: ResMut<SceneTimer>,
) {
    if let Ok(sink) = q_audio.single() {
        stopwatch.set_elapsed(sink.position());
    }
    if let Some(mut game_stats) = game_stats {
        if game_stats.time_left.is_some_and(|t| t.is_zero()) {
            // destroy world
            // TODO: More robust scenario handling and exit conditions
            commands.set_state(AppState::Menu);
            commands.spawn_scene(score_menu(*game_stats));
        }
        game_stats.time_left = game_stats.time_left.map(|t| t.saturating_sub(time.delta()));
    }

    if mouse_input.just_pressed(MouseButton::Left) {
        commands.run_system_cached(fire_weapon);
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
    mut debug_mode: ResMut<target_spawner::DebugMode>,
    game_state: Res<State<GameState>>,
) {
    let game_state = game_state.get();

    if key_input.just_pressed(KeyCode::Slash) {
        debug_mode.0 = !debug_mode.0;
    }
    if key_input.just_pressed(KeyCode::Escape) {
        match game_state {
            GameState::Playing => commands.set_state(GameState::Paused),
            GameState::Paused => commands.set_state(GameState::Playing),
        }
    }
}

fn update_ui(stats: Option<Res<GameStats>>, text: Populated<&mut Text, With<ScoreUI>>) {
    if let Some(stats) = stats
        && stats.is_changed()
    {
        for mut text in text {
            *text.get_text_mut() = format!(
                "score: {}\ntime left: {:?}",
                stats.score,
                stats.time_left.unwrap_or(Duration::ZERO)
            );
        }
    }
}

fn debug_window(mut contexts: EguiContexts, world_asset: Res<Assets<WorldAsset>>) -> Result {
    egui::Window::new("Debug Inspector").show(contexts.ctx_mut()?, |ui| {
        let worlds = world_asset.iter();
        for (_id, world) in worlds {
            // World fold
            egui::CollapsingHeader::new(world.reflect_short_type_path())
                .id_salt(world.world.id())
                .show(ui, |ui| {
                    // Entity loop
                    world.world.iter_entities().for_each(|ent| {
                        // Entity component
                        if let Ok(component) = world.world.inspect_entity(ent.id()) {
                            // Component name
                            let name = ent
                                .get_components::<&Name>()
                                .map(|n| n.to_string())
                                .unwrap_or(ent.id().to_string());

                            egui::CollapsingHeader::new(&name)
                                .id_salt(ent.id())
                                .show(ui, |ui| {
                                    for info in component {
                                        ui.label(info.name().to_string());
                                    }
                                });
                        }
                    });
                });
        }
    });
    Ok(())
}

fn pause_menu() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex, justify_content: JustifyContent::Center, align_items: AlignItems::Center, width: percent(100.), height: percent(100.)
        }
        DespawnOnExit<_>(GameState::Paused)
        Children [
            Node { flex_direction: FlexDirection::Column, width: percent(20.), height: percent(20.), align_items: AlignItems::Center, justify_content: JustifyContent::Center}
            BackgroundColor(css::RED)
            Children [
                Node {width: percent(100.), align_items: AlignItems::Center}
                on(|_: On<Pointer<Press>>, mut commands: Commands| {
                    commands.set_state(GameState::Playing);
                })
                Text::new("Exit")
            ]
        ]
    }
}

fn score_ui() -> impl Scene {
    bsn! {
        ScoreUI
        Node {
            top: Val::Px(0.),
            left: Val::Px(0.),
        }
        Text
        TextFont {
            font_size: FontSize::Px(50.),
        }
    }
}

fn setup_ui(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut meterials: ResMut<Assets<ColorMaterial>>,
    mut egui_settings: ResMut<EguiGlobalSettings>,
) -> Result {
    egui_settings.auto_create_primary_context = false;
    // Spawn crosshair
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(3.))),
        MeshMaterial2d(meterials.add(Color::WHITE)),
        Transform::default(),
    ));

    commands.spawn_scene(score_ui());
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

fn light_and_cameras(
    mut commands: Commands,
    // mut meshes: ResMut<Assets<Mesh>>,
    // mut materials: ResMut<Assets<StandardMaterial>>,
    mut render_image: ResMut<Assets<Image>>,
    mut scattering_mediums: ResMut<Assets<ScatteringMedium>>,
    // mut audio: ResMut<Assets<AudioBuffer>>,
    window: Single<&Window, With<PrimaryWindow>>,
    // asset_server: Res<AssetServer>,
) {
    // let gltf = WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("Scene.glb")));
    // log::info!("{:?}", gltf);
    // let _root = commands.spawn(gltf);
    //
    // let audio_file = File::open("samples/Ian Asher & Phantogram- Black Out Days.wav").unwrap();
    // let decoder = rodio::decoder::Decoder::try_from(audio_file).unwrap();
    //
    // let audio_buffer = rodio::buffer::SamplesBuffer::new(
    //     decoder.channels(),
    //     decoder.sample_rate(),
    //     decoder.collect::<Vec<_>>(),
    // );
    // log::info!("audio buffer size: {}", audio_buffer.len());
    //
    // commands.spawn((
    //     AudioPlayer(audio.add(AudioBuffer(audio_buffer))),
    //     PlaybackSettings {
    //         volume: bevy::audio::Volume::Linear(0.5),
    //         spatial: false,
    //         ..default()
    //     },
    // ));

    // std::thread::spawn(|| {
    //     let stream = testing();
    //     Box::leak(Box::new(stream));
    // });

    // commands.spawn((
    //     target_spawner::TargetSpawner(Timer::new(
    //         std::time::Duration::from_millis(100),
    //         TimerMode::Repeating,
    //     )),
    //     target_spawner::SpawnerVolumeMode::SampleBoundary,
    //     target_spawner::SpawnerVolume::from(Sphere::new(100.)),
    //     Transform::from_xyz(20., 10., 0.),
    // ));

    // commands.spawn((
    //     Mesh3d(
    //         meshes.add(
    //             Circle::new(100.)
    //                 .mesh()
    //                 .resolution(40)
    //                 .build()
    //                 .with_generated_tangents()
    //                 .unwrap(),
    //         ),
    //     ),
    //     MeshMaterial3d(materials.add(ground_material(asset_server))),
    //     Transform::from_xyz(0., 0., 0.)
    //         .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    // ));

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

    commands.spawn(Atmosphere::earth(
        scattering_mediums.add(ScatteringMedium::earth(256, 256)),
    ));

    commands.spawn((
        Camera3d::default(),
        PrimaryEguiContext,
        RenderLayers::from_layers(&[0, 1]),
        Camera {
            is_active: true,
            ..default()
        },
        (
            Transform::from_xyz(0., 5., 0.),
            PlayerCamera,
            FPSCamera::default(),
            FreeCamera::default(),
            AtmosphereSettings::default(),
            Exposure { ev100: 13.0 },
            Tonemapping::AcesFitted,
            Bloom::NATURAL,
            AtmosphereEnvironmentMapLight::default(),
            Msaa::Off,
            IsDefaultUiCamera,
            TemporalAntiAliasing::default(),
            ScreenSpaceReflections {
                min_perceptual_roughness: 0.0..0.0,
                ..default()
            },
        ),
    ));

    let cam = commands
        .spawn((
            Name::new("UI Camera"),
            AtmosphereSettings::default(),
            Camera2d,
            RenderTarget::Image(
                render_image
                    .add(Image::new_target_texture(
                        window.physical_width(),
                        window.physical_height(),
                        TextureFormat::Rgba8UnormSrgb,
                        None,
                    ))
                    .into(),
            ),
            Camera {
                order: 1,
                is_active: true,
                clear_color: ClearColorConfig::Custom(css::PINK.with_alpha(0.).into()),
                ..default()
            },
        ))
        .id();
    // 2d overlay camera
    commands.spawn((
        Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            align_content: AlignContent::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        ViewportNode::new(cam),
    ));
}

fn fire_weapon(
    mut fire_weapon: MessageWriter<FireWeapon>,
    transform: Single<&Transform, With<PlayerCamera>>,
) {
    fire_weapon.write(FireWeapon(**transform));
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

fn update_score(_: On<TargetDestroyed>, mut stats: If<ResMut<GameStats>>) {
    stats.score += 1;
}
