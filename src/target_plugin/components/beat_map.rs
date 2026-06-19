use bevy::{color::palettes::css::WHITE, prelude::*};

use crate::{AudioBuffer, target_plugin::TargetMarker};
#[derive(Resource)]
pub struct BeatMap {
    pub song: Handle<AudioBuffer>,
    pub hit_targets: Vec<(TargetMarker, Transform)>,
}

impl BeatMap {
    pub fn spawn(
        &self,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        commands.spawn(AudioPlayer(self.song.clone()));
        // TODO: Change from placeholder material
        let mesh = meshes.add(Sphere::new(1.));
        let mat = materials.add(StandardMaterial::from_color(WHITE));
        for (target, transform) in self.hit_targets.iter().cloned() {
            commands.spawn((
                target,
                transform,
                Mesh3d(mesh.clone()),
                MeshMaterial3d(mat.clone()),
                Visibility::Hidden,
            ));
        }
    }

    pub fn save(&mut self, targets: Query<(&TargetMarker, &Transform)>) {
        if targets.is_empty() {
            log::warn!("empty save!");
            return;
        }
        self.hit_targets = targets
            .iter()
            .map(|(a, b)| (a.clone(), b.clone()))
            .collect();
    }
}
