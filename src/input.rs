use crate::state::AppState;
use crate::target::events::TargetHit;
use crate::target::{DebugMode, Marker, Target};
use crate::{GameAction, GameState};
use bevy::window::{PrimaryWindow, WindowMode};
use bevy_egui::input::EguiWantsInput;
use fps_camera::FPSCamera;

use super::GameSettings;
use bevy::app::AnimationSystems;
use bevy::picking::mesh_picking::ray_cast::RayCastVisibility::Visible;
use bevy::prelude::*;

#[derive(Message, Debug)]
pub enum InputMessage {
    FireWeapon,
    FireWeaponHeld,
}

#[derive(SystemSet, Hash, Debug, PartialEq, PartialOrd, Ord, Eq, Clone, Copy)]
pub struct InputSet;

pub struct GameInputPlugin;
impl Plugin for GameInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<InputMessage>()
            .add_systems(
                Update,
                (
                    global_binds,
                    game_binds.run_if(in_state(AppState::InGame)),
                    playing_binds.run_if(in_state(GameState::Playing)),
                ),
            )
            .add_systems(
                PreUpdate,
                (update_raycast, handle_fire_input)
                    .in_set(InputSet)
                    .distributive_run_if(in_state(GameState::Playing)),
            )
            .add_systems(PostUpdate, remove_hovered.after(AnimationSystems));
    }
}

fn playing_binds(
    mut commands: Commands,
    q_target: Query<(Entity, &Marker), (With<Target>, With<Hovered>)>,
    mut reader: PopulatedMessageReader<InputMessage>,
) {
    if reader.len() > 1 {
        warn!("more messages then expected!");
    }
    for msg in reader.read() {
        if let InputMessage::FireWeapon = msg {
            fire_weapon(q_target, &mut commands)
        }
    }
}

fn fire_weapon(
    q_hit: Query<(Entity, &Marker), (With<Target>, With<Hovered>)>,
    commands: &mut Commands,
) {
    let mut hits = q_hit.iter().sort::<&Marker>();
    if let Some((t, _)) = hits.next() {
        commands.entity(t).trigger(TargetHit);
    }
}

fn global_binds(
    key_input: Res<ButtonInput<KeyCode>>,
    egui_focus: Res<EguiWantsInput>,
    mut commands: Commands,
    mut ui_debug: ResMut<GlobalUiDebugOptions>,
) {
    // don't trigger if something is focused i.e. text input
    if egui_focus.wants_any_keyboard_input() {
        return;
    }

    // Global
    if key_input.just_pressed(KeyCode::KeyF) {
        commands.run_system_cached(toggle_fullscreen);
    }

    if key_input.just_pressed(KeyCode::Equal) {
        ui_debug.enabled.toggle();
    }

    if key_input.just_pressed(KeyCode::Comma) {
        commands.set_state(AppState::UiDebug);
    }
}

fn game_binds(
    mut commands: Commands,
    key_input: Res<ButtonInput<KeyCode>>,
    mut debug_mode: ResMut<DebugMode>,
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

    if key_input.just_pressed(KeyCode::Period) {
        commands.set_state(AppState::ScoreSummery);
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

fn handle_fire_input(
    mut event_writer: MessageWriter<InputMessage>,
    game_settings: Res<GameSettings>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    let fire_button = game_settings.keybinds.get(&GameAction::FireWeapon).unwrap();
    let fire_button_mouse = game_settings
        .mousebinds
        .get(&GameAction::FireWeapon)
        .unwrap();
    if mouse.any_just_pressed(fire_button_mouse.iter().copied())
        || keyboard.any_just_pressed(fire_button.iter().copied())
    {
        event_writer.write(InputMessage::FireWeapon);
    } else if mouse.any_pressed(fire_button_mouse.iter().copied())
        || keyboard.any_pressed(fire_button.iter().copied())
    {
        event_writer.write(InputMessage::FireWeaponHeld);
    }
}

#[derive(Component, Deref)]
pub struct Hovered(pub usize);
fn update_raycast(
    mut ray_cast: MeshRayCast,
    q_cam: Query<&Transform, With<FPSCamera>>,
    q_target: Query<&Target>,
    mut commands: Commands,
) {
    for cam in q_cam.iter() {
        // Cast hovered ray from fps camera look direction
        let ray = Ray3d::new(cam.translation, cam.forward());

        // Filter out non target meshes
        let filter = |entity| q_target.contains(entity);
        let settings = MeshRayCastSettings::default()
            .with_filter(&filter)
            .with_visibility(Visible)
            .never_early_exit();

        let results = ray_cast.cast_ray(ray, &settings);

        for (idx, (entity, _hit)) in results.iter().enumerate() {
            // Hovered idx is likely non-deterministic as targets can be at identical distances from the camera
            // or it is incorrect way of ordering hit priority
            commands.entity(*entity).insert(Hovered(idx));
        }
    }
}

fn remove_hovered(mut commands: Commands, q_hovered: Query<Entity, With<Hovered>>) {
    for hovered in q_hovered.into_iter() {
        commands.entity(hovered).remove::<Hovered>();
    }
}
