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
    scoreing::Lifetime,
    target_plugin::{
        Target, TargetMaterial, TargetResource,
        components::{marker::Marker, target, timing_ring::TimingRing},
        events::SpawnHint,
    },
};

pub struct TargetMarkerPlugin;
impl Plugin for TargetMarkerPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_spawn_hint);
    }
}
#[derive(Component, Clone)]
// Marks when to spawn target
pub struct TargetMarker;

pub fn on_spawn_hint(
    e: On<SpawnHint>,
    mut commands: Commands,
    q_target_markers: Query<(Entity, &super::Marker, &Transform), With<TargetMarker>>,
    target_resource: Res<TargetResource>,
) {
    let Ok((entity, marker, spawn_location)) = q_target_markers.get(e.event_target()) else {
        return;
    };
    let preempt = marker.spawn_time.remaining();
    // Start preempt time i.e. fade in or whatever
    trace!("Begin Preempt");
    // spawn target

    // Base material for target
    let mat = TargetMaterial {
        // TODO: Target Base Color
        color: RED_800.into(),
        ring: target_resource.ring_start,
        ring_width: 0.2,
    };

    let miss_duration = Duration::from_millis(400);

    // Spawn Target
    let _target = commands.spawn_scene(bsn! {
        target::FadeIn({Timer::new(preempt,TimerMode::Once)})
        Mesh3d({target_resource.mesh.clone()})
        MeshMaterial3d::<TargetMaterial>(asset_value(mat))
        template_value(*spawn_location)
        DespawnOnExit::<AppState>(AppState::InGame)
        template_value(Target::Counter(1))
        Lifetime::duration(preempt + miss_duration)
    });
}
