use std::time::Duration;

use bevy::{
    animation::{animate_targets, animated_field},
    color::palettes::tailwind::{RED_100, RED_800},
    prelude::*,
};

use crate::{AppState, GameState, target_plugin::DebugMode};

pub struct CurvePlugin;
impl Plugin for CurvePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                tick_curve_marker.run_if(in_state(GameState::Playing)),
                curve_marker_gizmos.run_if(resource_equals(DebugMode(true))),
            ),
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

#[derive(GizmoConfigGroup, Default, Reflect)]
pub struct CurveGizmo;

fn curve_marker_gizmos(
    q_curve_marker: Query<(Entity, &mut CurveMarker)>,
    mut gizmos: Gizmos<CurveGizmo>,
) {
    for (_, curve) in q_curve_marker {
        gizmos.curve_3d(&curve.curve, (0..1000).map(|a| a as f32 / 1000.), RED_800);
    }
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

            commands.entity(ent).despawn();
        }
    }
}
