use bevy::prelude::*;

mod app_state;
mod edit_mode;
mod game_state;
pub use app_state::AppState;
pub use edit_mode::EditMode;
pub use game_state::GameState;

pub struct StatePlugin;

impl Plugin for StatePlugin {
    fn build(&self, app: &mut App) {
        app.insert_state(AppState::Menu)
            .add_sub_state::<GameState>()
            .add_sub_state::<EditMode>()
            .add_systems(OnEnter(AppState::InGame), transition::game)
            .add_systems(OnEnter(GameState::Paused), transition::pause)
            .add_systems(OnEnter(GameState::Playing), transition::play)
            .add_systems(OnEnter(AppState::UiDebug), transition::ui_debug)
            .add_systems(OnEnter(AppState::ScoreSummery), transition::score_summery);
    }
}

pub mod transition {

    use bevy::{
        ecs::entity_disabling::Disabled,
        prelude::*,
        window::{CursorGrabMode, CursorOptions, PrimaryWindow},
    };
    use fps_camera::{FPSCamera, GrabMouse};

    use crate::{AudioBuffer, menus, scoreing::Score};

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

    pub(crate) fn ui_debug(mut commands: Commands) {
        // INFO: Hook for debug window
    }

    pub(crate) fn score_summery(
        mut commands: Commands,
        score: Res<Score>,
        mut grab_mode: ResMut<GrabMouse>,
        mut cursor_options: Single<&mut CursorOptions, With<PrimaryWindow>>,
    ) {
        **grab_mode = false;
        cursor_options.grab_mode = CursorGrabMode::None;
        cursor_options.visible = true;
        commands.spawn_scene(menus::score_summery(&score));
    }

    pub(crate) fn game(mut fps_camera: Single<(&mut FPSCamera, &mut Transform)>) {
        let (mut cam, mut transform) = fps_camera.into_inner();
        cam.pitch = 0.0;
        cam.yaw = 0.0;
        transform.rotation = Quat::from_euler(EulerRot::ZYX, 0.0, cam.yaw, cam.pitch);
    }
}
