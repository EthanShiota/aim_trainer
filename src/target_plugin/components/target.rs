use std::time::Duration;

use bevy::{color::palettes::css::WHITE, prelude::*};

use crate::{AudioBuffer, target_plugin::TargetSpawner};
// Marker component for targets
#[derive(Component, Copy, Clone)]
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

        for (entity, _ray_mesh_hit) in hits.iter().take(1) {
            target_hit.write(TargetHit(*entity));
        }
    }
}

pub fn destroy_hit_targets(
    mut targets: PopulatedMessageReader<TargetHit>,
    mut commands: Commands,
    asset_server: ResMut<AssetServer>,
) {
    for TargetHit(entity) in targets.read() {
        commands.entity(*entity).remove::<Mesh3d>();
        commands
            .entity(*entity)
            .insert(AudioPlayer::new(asset_server.load("audio/Creams.ogg")));
        _ = commands
            .delayed()
            .secs(2.)
            .get_entity(*entity)
            .and_then(|mut e| {
                e.despawn();
                Ok(())
            });
        commands.trigger(TargetDestroyed);
    }
}
