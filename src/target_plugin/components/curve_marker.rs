#![allow(unused)]
use std::time::Duration;

use bevy::{
    animation::{AnimatedBy, animate_targets, animated_field},
    asset::uuid::Uuid,
    color::palettes::tailwind::{BLUE_300, GREEN_400, RED_100, RED_800},
    math::curve,
    prelude::*,
};

use crate::{
    AppState, GameState, target_plugin::{DebugMode, Target, target::FadeIn, target_material::TargetMaterial},
};

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
    // duration of curve
    pub duration: f32,
    // Spawns curve when duration is zero
    pub lifetime: Timer,
    pub preempt: Duration,
}

#[derive(GizmoConfigGroup, Default, Reflect)]
pub struct CurveGizmo;

fn curve_marker_gizmos(
    q_curve_marker: Query<(Entity, &mut CurveMarker)>,
    mut gizmos: Gizmos<CurveGizmo>,
) {
    for (_, curve) in q_curve_marker {
        gizmos.curve_3d(
            &curve.curve,
            (0..100).map(|i| (i as f32 / 100.) * curve.curve.domain().end()),
            BLUE_300,
        );
    }
}
fn tick_curve_marker(
    mut q_curve_marker: Query<(Entity, &mut CurveMarker)>,
    mut animation_clips: ResMut<Assets<AnimationClip>>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    time: Res<Time<Virtual>>,
    mut commands: Commands,
) {
    for (ent, mut curve_marker) in q_curve_marker.iter_mut() {
        curve_marker.lifetime.tick(time.delta());

        let approach_window = curve_marker.preempt;
        if (curve_marker.lifetime.remaining().abs_diff(approach_window)) <= time.delta() {
            // show curve

            // Construct polyline3d using samples
            let samples = curve_marker.curve.samples(600).unwrap();
            let polyline = Polyline3d::new(samples);

            let mesh = polyline.mesh().build();
            // TODO: new line material
            commands.entity(ent).apply_scene(bsn! {
                Mesh3d(asset_value(mesh))
                MeshMaterial3d<StandardMaterial>(asset_value(StandardMaterial{unlit: true, ..StandardMaterial::from_color(GREEN_400)}))
                // Transform {
                //     translation: {curve_marker.curve.sample(0.).unwrap()}
                // }
            });
            let hint = commands.spawn_scene(
                bsn! {
                    Mesh3d(asset_value(Sphere::new(2.)))
                    MeshMaterial3d::<TargetMaterial>(asset_value(TargetMaterial {color: GREEN_400.into(), ring: 1., ring_width: 0.3}))
                    
                    FadeIn({Timer::new(curve_marker.preempt, TimerMode::Once)})
                    Transform {
                        translation: {curve_marker.curve.sample_unchecked(0.)}
                    }
                }
            ).id();
            commands.delayed().duration(curve_marker.preempt).entity(hint).despawn();
            trace!("Curve Marker Spawned!");
        }

        // INFO: Spawns curve
        if curve_marker.lifetime.just_finished() {
            let mut clip = AnimationClip::default();
            let curve = AnimatableCurve::new(
                animated_field!(Transform::translation),
                curve_marker.curve.clone(),
            );

            let anim_id =
                bevy::animation::AnimationTargetId(Uuid::from_u128(ent.index_u32() as u128));
            clip.add_curve_to_target(anim_id, curve);
            clip.set_duration(curve_marker.duration);

            let (animation_graph, animation_node_index) =
                AnimationGraph::from_clip(animation_clips.add(clip));

            let mut player = AnimationPlayer::default();
            player.start(animation_node_index);

            let mut slider = commands
                .spawn_scene(bsn! {
                    Mesh3d(asset_value(Sphere::new(2.)))
                    MeshMaterial3d::<TargetMaterial>(asset_value(TargetMaterial {color: GREEN_400.into(), ring: 1., ring_width: 0.1}))
                    Transform {
                        translation: {curve_marker.curve.sample_unchecked(0.)}
                    }
                    DespawnOnExit::<AppState>(AppState::InGame)
                    template_value(Target::Duration(Duration::from_secs_f32(curve_marker.duration)))
                    template_value(player)
                    AnimationGraphHandle(asset_value(animation_graph))
                })
                .id();

            commands
                .entity(slider)
                .insert((anim_id, AnimatedBy(slider)));
            let mut delay = commands.delayed();
            delay.secs(curve_marker.duration).entity(slider).despawn();
            delay.secs(curve_marker.duration).entity(ent).try_despawn();

            trace!("Spawned Curve");
        }
    }
}
