use super::*;
use crate::PlayerCamera;

#[derive(GizmoConfigGroup, Default, Reflect)]
pub struct SpawnerGizmo;

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
        &mut TargetSpawner,
        &SpawnerVolume,
        &SpawnerVolumeMode,
    )>,
    time: Res<Time>,
) {
    for (entity, mut spawner, vol, mode) in spawners {
        if spawner.tick(time.delta()).just_finished() {
            let target_translation = match mode {
                SpawnerVolumeMode::SampleInterior => vol.sample_interior(&mut rand::rng()),
                SpawnerVolumeMode::SampleBoundary => vol.sample_boundary(&mut rand::rng()),
            };
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
