use bevy::{color::palettes::tailwind::RED_800, prelude::*};
use std::time::Duration;

use crate::target_plugin::{
    Target, TargetMaterial, TargetResource, components::target, events::SpawnHint,
};
use crate::{AppState, scoreing::Lifetime};

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
    mut target_resource: ResMut<TargetResource>,
) {
    let Ok((entity, marker, spawn_location)) = q_target_markers.get(e.event_target()) else {
        return;
    };
    let preempt = marker.spawn_time - marker.preempt;
    // Start preempt time i.e. fade in or whatever
    trace!("Begin Preempt");
    // spawn target

    let miss_duration = Duration::from_millis(400);

    // Spawn Target
    let _target = commands.entity(entity).apply_scene(bsn! {
        target::FadeIn({Timer::new(preempt,TimerMode::Once)})
        {target_resource.target_scene()}
        // Mesh3d({target_resource.mesh.clone()})
        // MeshMaterial3d::<TargetMaterial>(asset_value(mat))
        template_value(*spawn_location)
        DespawnOnExit::<AppState>(AppState::InGame)
        template_value(Target::Counter(1))
        Lifetime::duration(preempt + miss_duration)
    });
}
