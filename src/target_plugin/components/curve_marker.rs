use std::time::Duration;

use bevy::prelude::*;

struct CurvePlugin;
impl Plugin for CurvePlugin {
    fn build(&self, app: &mut App) {}
}

/// Display preview of curve and then spawn curve in
#[derive(Component, Clone)]
pub struct CurveMarker {
    pub curve: bevy::math::curve::SampleAutoCurve<Vec3>,
    // Spawns curve when duration is zero
    pub lifetime: Timer,
}

fn tick_curve_marker(
    mut q_curve_marker: Query<(Entity, &mut CurveMarker)>,
    time: Res<Time<Real>>,
    mut commands: Commands,
) {
    for (ent, mut curve_marker) in q_curve_marker.iter_mut() {
        curve_marker.lifetime.tick(time.delta());
        if curve_marker.lifetime.just_finished() {
            // TODO:! spawn curve

            commands.entity(ent).remove::<CurveMarker>();
        }
    }
}
