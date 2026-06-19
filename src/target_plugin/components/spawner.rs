use crate::target_plugin::TargetResource;
use std::time::Duration;

use bevy::prelude::*;

use crate::PlayerCamera;

use super::*;

#[derive(GizmoConfigGroup, Default, Reflect)]
pub struct SpawnerGizmo;

#[derive(Component, Clone, FromTemplate, Reflect)]
#[require(Visibility)]
#[reflect(Component)]
#[type_path = "api"]
pub struct TargetSpawner {
    pub timer: Timer,
    pub limit: Option<usize>,
}

impl TargetSpawnerTemplate {
    pub fn new(period: Duration, limit: Option<usize>) -> Self {
        Self {
            timer: Timer::new(period, TimerMode::Repeating),
            limit,
        }
    }
}

pub fn draw_spawners(
    targets: Query<(&Transform, &SpawnerVolume)>,
    mut gizmos: Gizmos<SpawnerGizmo>,
) {
    for (transform, spawner_volume) in targets.into_iter() {
        let color = Color::BLACK;
        let isometry = Isometry3d::from_translation(transform.translation);
        match spawner_volume {
            SpawnerVolume::Sphere(sphere) => {
                gizmos.primitive_3d(sphere, isometry, color);
            }
            SpawnerVolume::Cuboid(cuboid) => {
                gizmos.primitive_3d(cuboid, isometry, color);
            }
            SpawnerVolume::Torus(torus) => {
                gizmos.primitive_3d(torus, isometry, color);
            }
            SpawnerVolume::Mesh(_mesh) => {
                gizmos.sphere(isometry, 3., color.lighter(0.1));
            }
        }
    }
}

pub fn debug_spawn_target(
    mut commands: Commands,
    target_res: Res<TargetResource>,
    player_camera_transform: Single<&Transform, With<PlayerCamera>>,
) {
    commands.spawn((
        Mesh3d(target_res.mesh.clone()),
        MeshMaterial3d(target_res.material.clone()),
        Target,
        player_camera_transform.with_translation(
            player_camera_transform.translation + *player_camera_transform.forward() * 40.,
        ),
    ));
}

pub fn spawner_loop(
    mut commands: Commands,
    target_res: Res<TargetResource>,
    spawners: Query<(
        Entity,
        &Transform,
        &mut TargetSpawner,
        &SpawnerVolume,
        &SpawnerVolumeMode,
        Option<&Children>,
    )>,
    time: Res<Time>,
) {
    for (entity, transform, mut spawner, vol, mode, targets) in spawners {
        if !spawner
            .limit
            .is_none_or(|limit| targets.map(|c| c.len()).unwrap_or_default() < limit)
        {
            continue;
        }
        if spawner.timer.tick(time.delta()).just_finished() {
            let target_translation = match mode {
                SpawnerVolumeMode::SampleInterior => vol.sample_interior(&mut rand::rng()),
                SpawnerVolumeMode::SampleBoundary => vol.sample_boundary(&mut rand::rng()),
            } + transform.translation;

            commands.entity(entity).with_child((
                Target,
                Visibility::Visible,
                Transform::from_translation(target_translation),
                Mesh3d(target_res.mesh.clone()),
                MeshMaterial3d(target_res.material.clone()),
            ));
        }
    }
}
