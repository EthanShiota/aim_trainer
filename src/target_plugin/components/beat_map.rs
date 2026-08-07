use bevy::{color::palettes::css::WHITE, prelude::*};

use crate::{
    AppState, AudioBuffer,
    target_plugin::{CurveMarker, TargetMarker, components::timing_ring::TimingRing},
};
#[derive(Resource)]
pub struct BeatMap {
    pub song: Handle<AudioBuffer>,
    pub target_markers: Vec<(TargetMarker, Transform)>,
    pub target_curves: Vec<CurveMarker>,
}

impl BeatMap {
    pub fn spawn(
        &self,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        commands.spawn((
            AudioPlayer(self.song.clone()),
            DespawnOnExit(AppState::InGame),
        ));
        // TODO: Change from placeholder material
        let mesh = meshes.add(Sphere::new(1.));
        let mat = materials.add(StandardMaterial::from_color(WHITE));
        for (target, transform) in self.target_markers.iter().cloned() {
            commands.spawn((
                target,
                transform,
                // Mesh3d(mesh.clone()),
                // MeshMaterial3d(mat.clone()),
                Visibility::Hidden,
                DespawnOnExit(AppState::InGame),
            ));
        }

        for curve_marker in self.target_curves.iter().cloned() {
            commands.spawn((curve_marker, DespawnOnExit(AppState::InGame)));
        }
    }
}
