use bevy::prelude::*;
use std::time::Duration;

use crate::{
    AudioBuffer,
    target_plugin::{Target, TargetResource},
};
#[derive(Component, DerefMut, Deref, PartialEq, PartialOrd, Clone)]
// Marks when to spawn target
pub struct TargetMarker(pub Duration);

pub fn tick_target_marker(
    mut commands: Commands,
    q_markers: Query<(
        Entity,
        &TargetMarker,
        &Transform,
        &MeshMaterial3d<StandardMaterial>,
    )>,
    q_sink: Single<&AudioSink, With<AudioPlayer<AudioBuffer>>>,
    _time: Res<Time<Real>>,
    target: Res<TargetResource>,
) {
    let window = Duration::from_secs_f32(1.);

    for (ent, marker, transform, _mat) in q_markers.iter() {
        let approach_marker = marker.saturating_sub(window);
        let current_time = q_sink.position();

        if approach_marker <= current_time {
            // show target marker
            commands.entity(ent).insert(Visibility::Visible);
        }
        if **marker <= current_time {
            log::info!("spawned {:?}", **marker);
            // spawn target
            commands.spawn((
                Mesh3d(target.mesh.clone()),
                MeshMaterial3d(target.material.clone()),
                transform.clone(),
                Target,
            ));
            commands.entity(ent).despawn();
        }
    }
}
