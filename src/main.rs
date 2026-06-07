mod materials;
mod target_spawner;

use std::marker::PhantomData;

use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{
    Exposure, RenderTarget,
};
use bevy::camera_controller::free_camera::{FreeCamera, FreeCameraPlugin};
use bevy::color::palettes::css::PINK;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::common_conditions::input_just_pressed;
use bevy::light::atmosphere::ScatteringMedium;
use bevy::light::light_consts::lux;
use bevy::light::{Atmosphere, AtmosphereEnvironmentMapLight, VolumetricLight};

use bevy::pbr::{AtmosphereSettings, DefaultOpaqueRendererMethod, ScreenSpaceReflections};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::text::TextSection;
use bevy::window::{PrimaryWindow, WindowMode};
use materials::ground_material;

use target_spawner::FireWeapon;

use crate::target_spawner::TargetDestroyed;

#[derive(Resource, Deref, DerefMut)]
struct Score(usize);

#[derive(Event)]
struct PropagateChange<T>(PhantomData<T>)
where
    T: Resource;

impl<T: Resource> Default for PropagateChange<T> {
    fn default() -> Self {
        Self(PhantomData::<T>)
    }
}

fn main() {
    App::new()
        .insert_resource(DefaultOpaqueRendererMethod::deferred())
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(GlobalAmbientLight::NONE)
        .insert_resource(Score(0))
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
            FreeCameraPlugin,
        ))
        // .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        // .add_plugins(RapierDebugRenderPlugin::default())
        .insert_state(GameState::Inactive)
        .add_plugins(target_spawner::TargetPlugin)
        .add_systems(Startup, (startup, setup_ui).chain())
        .add_systems(
            Update,
            (
                fire_weapon.run_if(input_just_pressed(MouseButton::Left)),
                toggle_fullscreen.run_if(input_just_pressed(KeyCode::KeyF)),
                update_ui,
                bevy::ui::widget::update_viewport_render_target_size,
            ),
        )
        .add_observer(update_score)
        .run();
}

#[derive(Debug, Hash, Resource, Eq, PartialEq, PartialOrd, States, Clone)]
enum GameState {
    Active,
    Inactive,
}

#[derive(Component, Default, Reflect, Debug)]
#[reflect(Component, Default)]
#[type_path = "api"]
struct Character {
    name: String,
}

#[derive(Component, Default, Reflect, Debug)]
#[reflect(Component, Default)]
#[type_path = "api"]
struct Ground();

#[derive(Component, Default, Clone)]
struct ScoreUI;

#[derive(Component)]
struct PlayerCamera;

fn update_ui(score: Res<Score>, text: Query<&mut Text, With<ScoreUI>>) {
    if score.is_changed() {
        for mut text in text {
            *text.get_text_mut() = format!("score: {}", **score);
        }
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
) {
    // Spawn crosshair
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(8.))),
        MeshMaterial2d(meterials.add(Color::WHITE)),
        Transform::default(),
    ));

    commands.spawn_scene(score_ui());
}

fn startup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut meterials: ResMut<Assets<StandardMaterial>>,
    mut render_image: ResMut<Assets<Image>>,
    mut scattering_mediums: ResMut<Assets<ScatteringMedium>>,
    window: Single<&Window, With<PrimaryWindow>>,
    asset_server: Res<AssetServer>,
) {
    // commands.spawn((
    //     target_spawner::TargetSpawner(Timer::new(
    //         std::time::Duration::from_millis(100),
    //         TimerMode::Repeating,
    //     )),
    //     target_spawner::SpawnerVolumeMode::SampleBoundary,
    //     target_spawner::SpawnerVolume::from(Sphere::new(100.)),
    //     Transform::from_xyz(20., 10., 0.),
    // ));

    commands.spawn((
        Mesh3d(
            meshes.add(
                Circle::new(100.)
                    .mesh()
                    .resolution(40)
                    .build()
                    .with_generated_tangents()
                    .unwrap(),
            ),
        ),
        MeshMaterial3d(meterials.add(ground_material(asset_server))),
        Transform::from_xyz(0., 0., 0.)
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));

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
        RenderLayers::from_layers(&[0, 1]),
        Camera {
            is_active: true,
            ..default()
        },
        (
            Transform::from_xyz(0., 5., 0.),
            PlayerCamera,
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
                clear_color: ClearColorConfig::Custom(PINK.with_alpha(0.).into()),
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
    if *maximized {
        window.mode = WindowMode::BorderlessFullscreen(MonitorSelection::Current);
    } else {
        window.mode = WindowMode::Windowed;
    }
    *maximized = !*maximized;
}

fn update_score(
    _: On<TargetDestroyed>,
    mut score: ResMut<Score>,
    mut score_ui: Single<&mut Text, With<ScoreUI>>,
) {
    **score += 1;
    *score_ui.get_text_mut() = format!("score: {}", **score);
}
