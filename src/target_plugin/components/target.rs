use std::time::Duration;

use crate::AppState;
use crate::scoreing::Score;
use crate::target_plugin::events::TargetHit;
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
                commands.entity(entity).remove::<FadeIn>();
            }
        }
    }
}

pub fn on_target_hit(
    e: On<TargetHit>,
    mut commands: Commands,
    mut q_target: Query<(Entity, &mut Target)>,
    mut score: ResMut<Score>,
    time: Res<Time<Virtual>>,
) {
    if let Ok((ent, mut target)) = q_target.get_mut(e.event_target()) {
        // Update State
        let should_destroy = match target.as_mut() {
            Target::Counter(count) => {
                *count -= 1;
                score.points += 1;
                *count == 0
            }
            Target::Duration(duration) => {
                // *duration = duration.saturating_sub(time.delta());
                // score.points += 1;
                duration.is_zero()
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
    q_transform: Query<&Transform>,
) {
    if let Ok(&t) = q_transform.get(e.event_target()) {
        commands.spawn_scene(bsn! {
            AudioPlayer("audio/Creams.ogg")
            template_value(t)
            DespawnOnExit::<AppState>(AppState::InGame)
        });
    }
    commands.entity(e.event_target()).despawn();
}
