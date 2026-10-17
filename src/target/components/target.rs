use std::time::Duration;

use crate::scoreing::{Lifetime, Score};
use crate::target::events::TargetDestroyed;
use crate::target::events::TargetHit;
use crate::{AppState, SoundSettings};
use bevy::prelude::*;
use tracing::instrument;

use crate::target::{Active, Marker, TargetMaterial, TargetResource};

// Marker component for targets
#[derive(Component, Copy, Clone)]
pub enum Target {
    Counter(usize),
    Duration(Duration),
    Marker,
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
    mut q_target: Query<(
        Entity,
        Option<&mut FadeIn>,
        &MeshMaterial3d<TargetMaterial>,
        &Marker,
    )>,
    target_resource: Res<TargetResource>,
    time: Res<Time<Virtual>>,
    mut commands: Commands,
) {
    let dt = time.delta();
    for (order, (entity, fade_in, mat, _)) in q_target.iter_mut().sort::<&Marker>().enumerate() {
        // lowest order needs to be clicked first

        if let Some(mut m) = mats.get_mut(mat) {
            if let Some(mut fade_in) = fade_in {
                let ease = target_resource
                    .easing
                    .sample_unchecked(fade_in.0.fraction());
                fade_in.0.tick(dt);
                if fade_in.0.is_finished() {
                    m.ring_width = 2.;
                    commands.entity(entity).try_remove::<FadeIn>();
                }
                m.ring = (1. - ease) * target_resource.ring_start + ease * target_resource.ring_end;
            }

            m.priority = order as u32;
        }
    }
}

pub fn on_target_hit(
    e: On<TargetHit>,
    mut commands: Commands,
    mut q_target: Query<(Entity, &mut Target)>,
    mut score: ResMut<Score>,
    q_lifetime: Query<&Lifetime>,
) {
    if let Ok((ent, mut target)) = q_target.get_mut(e.event_target()) {
        // Update State
        let should_destroy = match target.as_mut() {
            Target::Counter(count) => {
                *count -= 1;
                if let Ok(lifetime) = q_lifetime.get(e.event_target()) {
                    debug_span!("target_hit").in_scope(|| {
                        let points = score.score_lifetime(lifetime) as f64;
                        debug!(points = points);
                        score.points += points;
                    });
                }
                *count == 0
            }
            Target::Duration(_duration) => {
                // *duration = duration.saturating_sub(time.delta());
                // score.points += 1;
                // duration.is_zero()
                // TODO: Handle; linked to curve and scoring
                false
            }
            Target::Marker => false,
        };

        // Destroy target
        if should_destroy {
            commands.entity(ent).trigger(TargetDestroyed);
        }
    }
}

#[instrument(skip(e, commands, sound_settings, q_transform))]
pub fn on_target_destroyed(
    e: On<TargetDestroyed>,
    mut commands: Commands,
    sound_settings: Res<SoundSettings>,
    q_transform: Query<&Transform>,
) {
    if let Ok(&t) = q_transform.get(e.event_target()) {
        commands.spawn_scene(bsn! {
            @crate::effects::hit_sound(sound_settings.effects_volume)
            t
            DespawnOnExit::<AppState>(AppState::InGame)
        });
    }
    commands.entity(e.event_target()).despawn();
}
