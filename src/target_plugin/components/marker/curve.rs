#![allow(unused)]
use std::time::Duration;

use bevy::{
    animation::{AnimatedBy, animate_targets, animated_field},
    asset::uuid::Uuid,
    color::palettes::tailwind::{BLUE_300, GREEN_400, RED_100, RED_800},
    math::curve,
    prelude::*,
};
use bevy_inspector_egui::egui::epaint::color;

use crate::{
    AppState, GameState, scoreing::Lifetime, target_plugin::{DebugMode, Target, TargetResource, events::{SpawnHint, SpawnTarget, TargetHit}, target::FadeIn, target_material::TargetMaterial},
};

pub struct CurvePlugin;
impl Plugin for CurvePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                curve_marker_gizmos.run_if(resource_equals(DebugMode(true))),
            ),
        )
            .insert_gizmo_config(
                CurveGizmo,
                GizmoConfig {
                    render_layers: bevy::camera::visibility::RenderLayers::layer(1),
                    ..default()
                },
            )
            .add_observer(on_spawn_hint);
    }
}

/// Display preview of curve and then spawn curve in
#[derive(Component, Clone)]
#[require(super::Marker)]
pub struct CurveMarker {
    pub curve: bevy::math::curve::SampleAutoCurve<Vec3>,
    // duration of curve
    pub duration: f32,
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

fn on_spawn_hint(e: On<SpawnHint>, q_curve_marker: Query<(Entity, &mut CurveMarker)>, mut commands: Commands, q_marker: Query<&super::Marker>,
    mut animation_clips: ResMut<Assets<AnimationClip>>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    mut target_resource: Res<TargetResource>,
    time: Res<Time<Virtual>>,
) {
            let Ok(marker) = q_marker.get(e.event_target()) else {return};
            let Ok((entity,curve_marker)) = q_curve_marker.get(e.event_target()) else {return};

            let preempt = marker.spawn_time.remaining();
            // show curve

            // Construct polyline3d using samples
            let samples = curve_marker.curve.samples(600).unwrap();
            let polyline = Polyline3d::new(samples);

            let mesh = polyline.mesh().build();
            // TODO: new line material
            commands.spawn_scene(bsn! {
                Mesh3d(asset_value(mesh))
                MeshMaterial3d<StandardMaterial>(asset_value(StandardMaterial{unlit: true, ..StandardMaterial::from_color(GREEN_400)}))
                template_value(Lifetime::duration(preempt + Duration::from_secs_f32(curve_marker.duration)))
            });

            // INFO: Spawns curve
            let mut clip = AnimationClip::default();
            let curve = AnimatableCurve::new(
                animated_field!(Transform::translation),
                curve_marker.curve.clone(),
            );

            let anim_id =
                bevy::animation::AnimationTargetId(Uuid::from_u128(entity.index_u32() as u128));
            clip.add_curve_to_target(anim_id, curve);
            clip.set_duration(curve_marker.duration);

            let (animation_graph, animation_node_index) =
                AnimationGraph::from_clip(animation_clips.add(clip));

            let mut player = AnimationPlayer::default();

            let anim_duration = Duration::from_secs_f32(curve_marker.duration);
            let end_time = time.elapsed() + preempt + anim_duration;
            let mut slider = commands.entity(entity)
                .apply_scene(bsn! {
                    Mesh3d({target_resource.mesh.clone()})
                    MeshMaterial3d::<TargetMaterial>(asset_value(TargetMaterial {color: GREEN_400.into(), ring: 1., ring_width: 0.1}))
                    Transform {
                        translation: {curve_marker.curve.sample_unchecked(0.)}
                    }
                    FadeIn({Timer::new(preempt, TimerMode::Once)})
                    DespawnOnExit::<AppState>(AppState::InGame)
                    template_value(Target::Duration(Duration::from_secs_f32(curve_marker.duration)))
                    template_value(player)
                    AnimationGraphHandle(asset_value(animation_graph))
                    template_value(Lifetime::duration(preempt + Duration::from_secs_f32(curve_marker.duration)))
                }).observe(move |e: On<SpawnTarget>, mut q_player: Query<&mut AnimationPlayer>, mut commands: Commands, time: Res<Time<Virtual>>| {
                    let speedup = anim_duration.div_duration_f32(time.elapsed().abs_diff(end_time));
                    if let Ok(mut p) = q_player.get_mut(e.event_target()) {
                        p.start(animation_node_index).set_speed(speedup);
                        
                    }
                    commands.entity(e.observer()).despawn();
                }).observe(|e: On<TargetHit>, mut commands:  Commands| {
                    commands.entity(e.event_target()).trigger(SpawnTarget);
                    commands.entity(e.observer()).despawn();

        })
                .id();

            commands
                .entity(slider)
                .insert((anim_id, AnimatedBy(slider)));
            let mut delay = commands.delayed();


            trace!("Spawned Curve");

}
