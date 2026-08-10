use bevy::prelude::*;

use crate::{
    AppState, AudioBuffer,
    target_plugin::{CurveMarker, TargetMarker},
};
#[derive(Resource)]
pub struct BeatMap {
    pub song: Handle<AudioBuffer>,
    pub target_markers: Vec<(TargetMarker, Transform)>,
    pub target_curves: Vec<CurveMarker>,
}

impl BeatMap {
    pub fn spawn(&self, mut commands: Commands) {
        commands.spawn((
            AudioPlayer(self.song.clone()),
            DespawnOnExit(AppState::InGame),
        ));
        for (target_marker, transform) in self.target_markers.iter().cloned() {
            commands.spawn((target_marker, transform, DespawnOnExit(AppState::InGame)));
        }

        for curve_marker in self.target_curves.iter().cloned() {
            commands.spawn((curve_marker, DespawnOnExit(AppState::InGame)));
        }
    }
}
