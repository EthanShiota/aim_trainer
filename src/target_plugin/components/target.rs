use std::time::Duration;

use bevy::prelude::*;

// Marker component for targets
#[derive(Component, Copy, Clone)]
pub enum Target {
    Counter(usize),
    Duration(Duration),
}

impl Default for Target {
    fn default() -> Self {
        Target::Counter(1)
    }
}

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
    mut target_hit: PopulatedMessageReader<TargetHit>,
    mut q_target: Query<&mut Target>,
    mut commands: Commands,
    asset_server: ResMut<AssetServer>,
    time: Res<Time<Real>>,
) {
    for TargetHit(entity) in target_hit.read() {
        let mut ent = commands.entity(*entity);
        let mut target = q_target.get_mut(ent.id()).unwrap();
        match target.as_mut() {
            Target::Counter(count) => {
                *count -= 1;
                if *count != 0 {
                    continue;
                }
            }
            Target::Duration(duration) => {
                // HACK: use system delta to approximate time on target
                let time_on_target = time.delta();

                info!("hit registered {:?} {:?}", time_on_target, duration);
                *duration = duration.saturating_sub(time_on_target);
                if !duration.is_zero() {
                    continue;
                }
            }
        }

        // Destroy hit target

        ent.remove::<Mesh3d>();
        // TODO: Sound effect handling
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
