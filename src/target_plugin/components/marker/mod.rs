use std::{fmt::Debug, time::Duration};

use bevy::prelude::*;
use tracing::instrument;

use crate::{
    AppState, AudioBuffer,
    target_plugin::{
        components::beat_map::BeatMapResource,
        events::{SpawnHint, SpawnTarget},
    },
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
    spawn_time: Duration,
    // Offset where hint should be displayed
    preempt: Duration,
}

impl Marker {
    pub fn new(spawn_time: Duration, preempt: Duration) -> Self {
        let Some(preempt_timer) = spawn_time.checked_sub(preempt) else {
            warn!("Preempt begins before start of map!");
            return Self {
                spawn_time: spawn_time,
                preempt: Duration::ZERO,
            };
        };

        Self {
            spawn_time: spawn_time,
            preempt: preempt_timer,
        }
    }
    pub fn time(&self) -> Duration {
        self.spawn_time
    }
}

impl Ord for Marker {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.spawn_time.cmp(&other.spawn_time)
    }
}

impl PartialOrd for Marker {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Marker {
    fn eq(&self, other: &Self) -> bool {
        self.spawn_time.eq(&other.spawn_time)
    }
}

impl Eq for Marker {}

impl Debug for Marker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Marker: {:?}", self.spawn_time)
    }
}

pub fn tick(
    mut q_marker: Query<(Entity, &mut Marker)>,
    q_sink: Query<&AudioSink, With<AudioPlayer<AudioBuffer>>>,
    mut beat_map: If<ResMut<BeatMapResource>>,
    mut commands: Commands,
) {
    let Ok(sink) = q_sink.get(beat_map.music_player) else {
        return;
    };
    for (entity, marker) in q_marker.iter_mut() {
        if (beat_map.scene_timer..=sink.position()).contains(&marker.preempt) {
            commands.entity(entity).trigger(SpawnHint);
        }
        if (beat_map.scene_timer..=sink.position()).contains(&marker.spawn_time) {
            commands.entity(entity).trigger(SpawnTarget);
        }
    }
    beat_map.scene_timer = sink.position();
}
