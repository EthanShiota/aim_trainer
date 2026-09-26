use std::time::Duration;

use crate::scoreing::{Lifetime, Score};
use crate::target_plugin::events::TargetDestroyed;
use crate::target_plugin::events::TargetHit;
use crate::{AppState, SoundSettings};
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

pub fn tick(
    mut mats: ResMut<Assets<TargetMaterial>>,
    mut q_target: Query<(Entity, &mut FadeIn, &MeshMaterial3d<TargetMaterial>)>,
    target_resource: Res<TargetResource>,
    time: Res<Time<Virtual>>,
    mut commands: Commands,
) {
    let dt = time.delta();
    for (entity, mut fade_in, mat) in q_target.iter_mut() {
        fade_in.0.tick(dt);
        if let Some(mut m) = mats.get_mut(mat) {
            let ease = target_resource
                .easing
                .sample_unchecked(fade_in.0.fraction());
            m.ring = (1. - ease) * target_resource.ring_start + ease * target_resource.ring_end;

            if fade_in.0.is_finished() {
                m.ring_width = 2.;
                commands.entity(entity).try_remove::<FadeIn>();
            }
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
                    let points = lifetime.judge_hit(score.overall_difficulty);
                    score.points += points;
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
        };

        // Destroy target
        if should_destroy {
            commands.entity(ent).trigger(TargetDestroyed);
        }
    }
}

pub fn on_target_destroyed(
    e: On<TargetDestroyed>,
    mut commands: Commands,
    sound_settings: Res<SoundSettings>,
    q_transform: Query<&Transform>,
) {
    if let Ok(&t) = q_transform.get(e.event_target()) {
        commands.spawn_scene(bsn! {
            crate::effects::hit_sound(sound_settings.effects_volume)
            template_value(t)
            DespawnOnExit::<AppState>(AppState::InGame)
        });
    }
    commands.entity(e.event_target()).despawn();
}
