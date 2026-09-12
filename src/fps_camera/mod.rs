#![allow(unused)]
use std::f32::{
    self,
    consts::{PI, TAU},
};

use bevy::{
    input::mouse::AccumulatedMouseMotion,
    prelude::*,
    window::{CursorOptions, PrimaryWindow},
};

use crate::target_plugin::Target;

#[derive(Resource, Reflect)]
pub struct FPSCameraConfig {
    pub sensitivity: f32,
}
const RADIANS_PER_DOT: f32 = 2. * PI;

impl Default for FPSCameraConfig {
    fn default() -> Self {
        // Self {
        //     sensitivity: 0.0275212525,
        // }
        FPSCameraConfig::with_sens(16.351, 800)
    }
}

impl FPSCameraConfig {
    // in / 360
    pub fn with_sens(sens: f32, dpi: usize) -> Self {
        // in / revolution * dot / in -> dot / revolution
        // * revolution / TAU -> dot / rad
        // ^ -1 -> rad / dot
        let sensitivity = 1. / (sens * (dpi as f32));
        println!("sensitivity: {:?}", sensitivity);
        Self { sensitivity }
    }
}

/// Marker trait for fps camera
#[derive(Component, Default, Clone)]
pub struct FPSCamera {
    pub pitch: f32,
    pub yaw: f32,
}

use super::effects::EffectSchedule;
pub struct FPSCameraPlugin;
impl Plugin for FPSCameraPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(FPSCameraConfig::default())
            .add_systems(
                RunFixedMainLoop,
                (update, update_raycast)
                    .chain()
                    .in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop)
                    .before(EffectSchedule),
            )
            .add_systems(PostUpdate, remove_hovered);
    }
}
fn setup(mut q_player_transform: Query<(&Transform, &mut FPSCamera)>) {
    for (player_transform, mut fps_camera) in q_player_transform.iter_mut() {
        let (z, y, x) = player_transform.rotation.to_euler(EulerRot::ZYX);
        fps_camera.pitch = y;
        fps_camera.yaw = x;
    }
}

#[derive(Resource, Clone, Copy, DerefMut, Deref)]
pub struct GrabMouse(pub bool);
fn update(
    mut q_fps_camera: Single<(&mut FPSCamera, &mut Transform)>,
    key_input: Res<ButtonInput<KeyCode>>,
    mut grab_mouse: ResMut<GrabMouse>,
    mut q_primary_window: Query<(&mut Window, &mut CursorOptions), With<PrimaryWindow>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    config: Res<FPSCameraConfig>,
    time: Res<Time<Real>>,
) {
    if !**grab_mouse {
        return;
    }

    let (mut state, mut transform) = q_fps_camera.into_inner();

    if mouse_motion.delta != Vec2::ZERO {
        let (dx, dy) = mouse_motion.delta.into();
        state.pitch = (state.pitch - (mouse_motion.delta.y * config.sensitivity * RADIANS_PER_DOT))
            .clamp(-PI / 2., PI / 2.);
        state.yaw -= (mouse_motion.delta.x * config.sensitivity * RADIANS_PER_DOT);
        transform.rotation = Quat::from_euler(EulerRot::ZYX, 0.0, state.yaw, state.pitch);
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
        let mut settings = MeshRayCastSettings::default().with_filter(&filter);

        let results = ray_cast.cast_ray(ray, &settings);

        for (idx, (entity, _hit)) in results.iter().enumerate() {
            // Hovered idx is likely non-deterministic as targets can be at identical distances from the camera
            // or it is incorrect way of ordering hit priority
            commands.entity(*entity).insert(Hovered(idx));
        }
    }
}

fn remove_hovered(mut commands: Commands, mut q_hovered: Query<Entity, With<Hovered>>) {
    for hovered in q_hovered.iter_mut() {
        commands.entity(hovered).remove::<Hovered>();
    }
}
