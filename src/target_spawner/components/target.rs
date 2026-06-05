use bevy::{ecs::relationship::DescendantIter, prelude::*};

use crate::target_spawner::TargetSpawner;
// Marker component for targets
#[derive(Component)]
pub struct Target;

#[derive(Message)]
pub struct TargetHit(pub Entity);

#[derive(Event)]
pub struct TargetDestroyed;

#[derive(Message)]
pub struct FireWeapon(pub Transform);

pub fn handle_fire_weapon(
    mut ray_cast: MeshRayCast,
    mut target_hit: MessageWriter<TargetHit>,
    mut fire_weapon: PopulatedMessageReader<FireWeapon>,
    targets: Query<Entity, With<Target>>,
) {
    for FireWeapon(transform) in fire_weapon.read() {
        let ray = Ray3d::new(transform.translation, transform.forward());
        let filter = |entity| targets.contains(entity);
        let settings = MeshRayCastSettings::default().with_filter(&filter);
        let hits = ray_cast.cast_ray(ray, &settings);

        for (entity, _ray_mesh_hit) in hits {
            target_hit.write(TargetHit(*entity));
        }
    }
}

pub fn destroy_hit_targets(mut targets: PopulatedMessageReader<TargetHit>, mut commands: Commands) {
    for TargetHit(entity) in targets.read() {
        commands.entity(*entity).despawn();
        commands.trigger(TargetDestroyed);
    }
}

pub fn move_targets(
    spawners: Query<Entity, With<TargetSpawner>>,
    time: ResMut<Time>,
    mut transforms: Query<&mut Transform>,
    child_query: Query<&Children>,
) {
    for spawner in spawners {
        for target in child_query.iter_descendants(spawner) {
            if let Ok(mut transform) = transforms.get_mut(target) {
                transform.translate_around(
                    Vec3::ZERO,
                    Quat::from_euler(EulerRot::XYZ, 0.0, 4.0 * time.delta().as_secs_f32(), 0.0),
                );
            }
        }
    }
}
