use std::time::Duration;

use bevy::prelude::*;

use crate::{
    AppState, AudioBuffer,
    scoreing::Lifetime,
    target_plugin::{CurveMarker, Marker, TargetMarker},
};
#[derive(Resource)]
pub struct BeatMap {
    pub song: Handle<AudioBuffer>,
    pub target_markers: Vec<(TargetMarker, Marker, Transform)>,
    pub target_curves: Vec<(CurveMarker, Marker)>,
}

impl BeatMap {
    pub fn spawn(&self, mut commands: Commands) {
        commands.spawn((
            AudioPlayer(self.song.clone()),
            DespawnOnExit(AppState::InGame),
        ));
        for (target_marker, marker, transform) in self.target_markers.iter().cloned() {
            commands.spawn((
                target_marker,
                marker,
                transform,
                DespawnOnExit(AppState::InGame),
            ));
        }

        for (curve_marker, marker) in self.target_curves.iter().cloned() {
            commands.spawn((curve_marker, marker, DespawnOnExit(AppState::InGame)));
        }
    }
}
