use std::time::Duration;

use bevy::prelude::*;

use crate::target_plugin::{TargetMaterial, TargetResource};

// Marker component for targets
#[derive(Component, Copy, Clone)]
pub enum Target {
    Counter(usize),
    Duration(Duration),
}

#[derive(Component, Default, Clone)]
pub struct FadeIn(pub Timer);

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

pub fn tick(
    mut mats: ResMut<Assets<TargetMaterial>>,
    mut q_target: Query<(&mut FadeIn, &MeshMaterial3d<TargetMaterial>)>,
    target_resource: Res<TargetResource>,
    time: Res<Time<Virtual>>,
) {
    let dt = time.delta();
    for (mut fade_in, mat) in q_target.iter_mut() {
        fade_in.0.tick(dt);
        if let Some(mut m) = mats.get_mut(mat) {
            m.ring = target_resource
                .easing
                .sample_unchecked(fade_in.0.fraction());
        }
    }
}
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
        let Ok(mut target) = q_target.get_mut(ent.id()) else {
            // skip case where target is despawned before commands queue
            continue;
        };
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
            .map(|mut e| {
                e.despawn();
            });
        commands.trigger(TargetDestroyed);
    }
}
