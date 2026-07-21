use std::time::Duration;

use bevy::{
    animation::{animate_targets, animated_field},
    color::palettes::tailwind::RED_100,
    prelude::*,
};

use crate::{AppState, GameState};

pub struct CurvePlugin;
impl Plugin for CurvePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            tick_curve_marker.run_if(in_state(GameState::Playing)),
        );
    }
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
            commands.spawn_scene(bsn! {
                Mesh3d(asset_value(Sphere::new(3.)))
                MeshMaterial3d::<StandardMaterial>(asset_value(StandardMaterial{ unlit: true, ..StandardMaterial::from_color(RED_100)}))
                Transform {
                    translation: {curve_marker.curve.sample_unchecked(0.)}
                }
                DespawnOnExit::<AppState>(AppState::InGame)
            });
            info!("Spawned Curve");

            commands.entity(ent).remove::<CurveMarker>();
        }
    }
}
