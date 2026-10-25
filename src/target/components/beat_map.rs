use std::time::Duration;

use bevy::prelude::*;

use crate::{
    AppState, AudioBuffer,
    target::{CurveMarker, Marker, TargetMarker},
};

pub struct BeatMap {
    pub song: Handle<AudioBuffer>,
    pub target_markers: Vec<(TargetMarker, Marker, Transform)>,
    pub target_curves: Vec<(CurveMarker, Marker)>,
}

#[derive(Resource)]
pub struct BeatMapResource {
    // Source of truth
    pub scene_timer: Duration,
    pub music_player: Entity,
}

// Spawns markers and song in
impl BeatMap {
    pub fn spawn(self, mut commands: Commands) -> BeatMapResource {
        for [(_, curr, _), (_, next, _)] in self.target_markers.array_windows::<2>() {
            if curr.time() == next.time() {
                error!("marker spawned in at same time");
            }
        }
        for [(curr_marker, curr), (_, next)] in self.target_curves.array_windows::<2>() {
            // if we spawn next curve while current is alive
            if Duration::from_secs_f32(curr_marker.duration) + curr.time() > next.time() {
                error!(curve = ?curr,"curve spawned in at same time");
            }
        }

        let music_player = commands
            .spawn((AudioPlayer(self.song), DespawnOnExit(AppState::InGame)))
            .id();
        for (target_marker, marker, transform) in self.target_markers {
            commands.spawn((
                target_marker,
                marker,
                transform,
                DespawnOnExit(AppState::InGame),
            ));
        }

        for (curve_marker, marker) in self.target_curves {
            commands.spawn((curve_marker, marker, DespawnOnExit(AppState::InGame)));
        }
        BeatMapResource {
            scene_timer: Duration::ZERO,
            music_player,
        }
    }
}
