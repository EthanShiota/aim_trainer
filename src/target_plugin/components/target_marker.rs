use bevy::{
    animation::{AnimatedBy, AnimationTargetId, animated_field},
    asset::uuid::Uuid,
    color::palettes::tailwind::RED_800,
    prelude::*,
};
use std::{
    f32::{self},
    time::Duration,
};

use crate::{
    AppState,
    target_plugin::{
        Target, TargetMaterial, TargetResource,
        components::{target, timing_ring::TimingRing},
    },
};
#[derive(Component, Clone)]
// Marks when to spawn target
pub struct TargetMarker {
    pub time: Timer,
    pub preempt: Timer,
}

pub fn tick(
    mut commands: Commands,
    mut q_target_markers: Query<(Entity, &mut TargetMarker, &Transform)>,
    time: Res<Time<Virtual>>,
    target: Res<TargetResource>,
) {
    for (ent, mut marker, transform) in q_target_markers.iter_mut() {
        marker.as_mut().time.tick(time.delta());
        let approach_marker = &marker.preempt;

        // Start preempt time i.e. fade in or whatever
        if marker
            .time
            .remaining()
            .saturating_sub(approach_marker.duration())
            .is_zero()
        {
            info!("Begin Preempt");
            // spawn target
            spawn_target(
                &mut commands,
                &target,
                *transform,
                marker.preempt.duration(),
            );
            commands.entity(ent).despawn();
        }
    }
}

fn spawn_target(
    commands: &mut Commands,
    target_resource: &Res<TargetResource>,
    spawn_location: Transform,
    preempt: Duration,
) {
    // Base material for target
    let mat = TargetMaterial {
        // TODO: Target Base Color
        color: RED_800.into(),
        ring: target_resource.ring_start,
        ring_width: 0.2,
    };

    // Spawn Target
    let target = commands.spawn_scene(bsn! {
        target::FadeIn({Timer::new(preempt,TimerMode::Once)})
        Mesh3d({target_resource.mesh.clone()})
        MeshMaterial3d::<TargetMaterial>(asset_value(mat))
        template_value(spawn_location)
        DespawnOnExit::<AppState>(AppState::InGame)
        template_value(Target::Counter(1))
    });
    let target_id = target.id();

    let miss_duration = Duration::from_millis(400);
    // Despawn after miss duration
    commands
        .delayed()
        .duration(preempt + miss_duration)
        .entity(target_id)
        .try_despawn();
}
