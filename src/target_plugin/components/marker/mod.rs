use std::{fmt::Debug, time::Duration};

use bevy::prelude::*;

use crate::{
    AppState,
    target_plugin::events::{SpawnHint, SpawnTarget},
};

mod curve;
mod point;

pub use curve::CurveMarker;
pub use point::TargetMarker;

pub struct MarkerPlugin;
impl Plugin for MarkerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((curve::CurvePlugin, point::TargetMarkerPlugin))
            .add_systems(Update, tick.run_if(in_state(AppState::InGame)));
    }
}

#[derive(Component, Clone, Default)]
pub struct Marker {
    // Location in song where marker should be clicked
    spawn_time: Timer,
    // Offset where hint should be displayed
    preempt: Timer,
}

impl Marker {
    pub fn new(spawn_time: Duration, preempt: Duration) -> Self {
        let Some(preempt_timer) = spawn_time.checked_sub(preempt) else {
            warn!("Preempt begins before start of map!");
            return Self {
                spawn_time: Timer::new(spawn_time, TimerMode::Once),
                preempt: Timer::new(Duration::ZERO, TimerMode::Once),
            };
        };

        Self {
            spawn_time: Timer::new(spawn_time, TimerMode::Once),
            preempt: Timer::new(preempt_timer, TimerMode::Once),
        }
    }
    pub fn time(&self) -> Duration {
        self.spawn_time.duration()
    }
}

impl Ord for Marker {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.spawn_time.duration().cmp(&other.spawn_time.duration())
    }
}

impl PartialOrd for Marker {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Marker {
    fn eq(&self, other: &Self) -> bool {
        self.spawn_time.duration().eq(&other.spawn_time.duration())
    }
}

impl Eq for Marker {}

impl Debug for Marker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Marker: {:?}", self.spawn_time.duration())
    }
}

pub fn tick(
    time: Res<Time<Virtual>>,
    mut q_marker: Query<(Entity, &mut Marker)>,
    mut commands: Commands,
) {
    let delta = time.delta();
    for (entity, mut marker) in q_marker.iter_mut() {
        marker.spawn_time.tick(delta);
        marker.preempt.tick(delta);
        if marker.preempt.just_finished() {
            commands.entity(entity).trigger(SpawnHint);
        }
        if marker.spawn_time.just_finished() {
            commands.entity(entity).trigger(SpawnTarget);
        }
    }
}
