use bevy::prelude::*;
use std::{
    f32::{self},
    time::Duration,
};

use crate::{
    AppState,
    target_plugin::{Target, TargetResource, components::timing_ring::TimingRing},
};
#[derive(Component, DerefMut, Deref, Clone)]
// Marks when to spawn target
pub struct TargetMarker(pub Timer);

pub fn tick_target_marker(
    mut commands: Commands,
    mut q_target_markers: Query<(Entity, &mut TargetMarker, &Transform)>,
    time: Res<Time<Virtual>>,
    target: Res<TargetResource>,
) {
    // HACK:Approach window
    let window = Duration::from_secs_f32(1.);

    for (ent, mut marker, transform) in q_target_markers.iter_mut() {
        marker.as_mut().tick(time.delta());
        let approach_marker = marker.0.remaining();

        // TODO: Use correct window
        if approach_marker <= window && (approach_marker + time.delta()) > window {
            // show target marker
            commands.entity(ent).insert(Visibility::Visible);
            commands.entity(ent).with_children(|c| {
                c.spawn(TimingRing {
                    lifetime: Timer::new(window, TimerMode::Once),
                    end_scale: (1. / 3.),
                });
            });
        }
        if marker.just_finished() {
            log::info!("spawned {:?}", **marker);
            // spawn target
            let target = commands
                .spawn((
                    Mesh3d(target.mesh.clone()),
                    MeshMaterial3d(target.material.clone()),
                    *transform,
                    DespawnOnExit::<AppState>(AppState::InGame),
                    Target::Counter(1),
                ))
                .id();
            commands.delayed().secs(1.).entity(target).try_despawn();
            commands.entity(ent).despawn();
        }
    }
}
