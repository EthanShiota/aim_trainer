#![allow(unused)]
use std::f32::consts::PI;

use bevy::{
    input::mouse::AccumulatedMouseMotion,
    prelude::*,
    window::{CursorOptions, PrimaryWindow},
};
pub struct FPSCameraPlugin;

#[derive(Resource)]
pub struct FPSCameraConfig {
    pub sensitivity: f32,
}
const RADIANS_PER_DOT: f32 = 1.0 / 180.0;
impl Default for FPSCameraConfig {
    fn default() -> Self {
        Self { sensitivity: 0.2 }
    }
}

/// Marker trait for fps camera
#[derive(Component, Default)]
pub struct FPSCamera {
    pub pitch: f32,
    pub yaw: f32,
}
impl Plugin for FPSCameraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(FPSCameraConfig::default()).add_systems(
            RunFixedMainLoop,
            update.in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop),
        );
    }
}
fn setup(mut q_player_transform: Query<(&Transform, &mut FPSCamera)>) {
    for (player_transform, mut fps_camera) in q_player_transform.iter_mut() {
        let (z, y, x) = player_transform.rotation.to_euler(EulerRot::ZYX);
        fps_camera.pitch = y;
        fps_camera.yaw = x;
    }
}

fn update(
    mut q_fps_camera: Query<(&mut FPSCamera, &mut Transform)>,
    key_input: Res<ButtonInput<KeyCode>>,
    mut grab_mouse: Local<bool>,
    mut q_primary_window: Query<(&mut Window, &mut CursorOptions), With<PrimaryWindow>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    config: Res<FPSCameraConfig>,
    time: Res<Time<Real>>,
) {
    let (mut primary_window, mut cursor_options) = q_primary_window.single_mut().unwrap();
    if key_input.just_pressed(KeyCode::KeyM) && primary_window.focused {
        *grab_mouse = !*grab_mouse;
        if *grab_mouse {
            cursor_options.grab_mode = bevy::window::CursorGrabMode::Locked;
            cursor_options.visible = false;
        } else {
            cursor_options.grab_mode = bevy::window::CursorGrabMode::None;
            cursor_options.visible = true;
        }
    }
    if !*grab_mouse {
        return;
    }

    let (mut state, mut transform) = q_fps_camera
        .single_mut()
        .expect("Only on fps camera supported");

    if mouse_motion.delta != Vec2::ZERO {
        let (dx, dy) = mouse_motion.delta.into();
        state.pitch = (state.pitch - mouse_motion.delta.y * RADIANS_PER_DOT * config.sensitivity)
            .clamp(-PI / 2., PI / 2.);
        state.yaw -= mouse_motion.delta.x * RADIANS_PER_DOT * config.sensitivity;
        transform.rotation = Quat::from_euler(EulerRot::ZYX, 0.0, state.yaw, state.pitch);
    }
}
