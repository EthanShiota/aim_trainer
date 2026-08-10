use std::time::Duration;

use crate::target_plugin::messages::*;
use crate::{fps_camera::Hovered, target_plugin::events::TargetDestroyed};
use bevy::{
    color::palettes::{css::BLUE_VIOLET, tailwind::RED_800},
    prelude::*,
};

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

pub fn tick(
    mut mats: ResMut<Assets<TargetMaterial>>,
    mut q_target: Query<(Entity, &mut FadeIn, &MeshMaterial3d<TargetMaterial>)>,
    target_resource: Res<TargetResource>,
    time: Res<Time<Virtual>>,
) {
    let dt = time.delta();
    for (_entity, mut fade_in, mat) in q_target.iter_mut() {
        fade_in.0.tick(dt);
        if let Some(mut m) = mats.get_mut(mat) {
            m.ring = target_resource
                .easing
                .sample_unchecked(fade_in.0.fraction());
        }
    }
}

pub fn handle_fire_weapon(
    mut target_hit: MessageWriter<TargetHit>,
    mut target_hit_delta: MessageWriter<TargetHitDelta>,
    mut fire_weapon: MessageReader<FireWeapon>,
    mut fire_weapon_held: MessageReader<FireWeaponHeld>,
    q_hits: Query<(Entity, &Hovered), With<Target>>,
) {
    let hits = q_hits
        .into_iter()
        .sort_by_key::<&Hovered, _>(|hover| hover.0)
        .collect::<Vec<_>>();
    for _ in fire_weapon.read() {
        for (entity, Hovered(_)) in hits.iter().take(1) {
            target_hit.write(TargetHit(*entity));
        }
    }

    for FireWeaponHeld { delta } in fire_weapon_held.read() {
        for (entity, Hovered(_)) in hits.iter().take(1) {
            target_hit_delta.write(TargetHitDelta((*entity, *delta)));
        }
    }
}

pub fn destroy_hit_targets(
    mut target_hit: PopulatedMessageReader<TargetHit>,
    mut target_hit_delta: MessageReader<TargetHitDelta>,
    mut q_target: Query<&mut Target>,
    mut commands: Commands,
    mut target_material: ResMut<Assets<TargetMaterial>>,
    q_target_material: Query<&MeshMaterial3d<TargetMaterial>>,
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
                e.try_despawn();
            });
        commands.trigger(TargetDestroyed);
    }

    for TargetHitDelta((entity, delta)) in target_hit_delta.read() {
        let ent = commands.entity(*entity);
        let Ok(mut target) = q_target.get_mut(ent.id()) else {
            // skip case where target is despawned before commands queue
            continue;
        };
        match target.as_mut() {
            Target::Counter(count) => {
                // *count -= 1;
                if *count != 0 {
                    continue;
                }
            }
            Target::Duration(duration) => {
                let time_on_target = *delta;

                *duration = duration.saturating_sub(time_on_target);

                if let Ok(h_mat) = q_target_material.get(*entity)
                    && let Some(mut mat) = target_material.get_mut(h_mat.id())
                {
                    mat.color = BLUE_VIOLET.into();
                }

                if !duration.is_zero() {
                    continue;
                }
            }
        }
    }
}
