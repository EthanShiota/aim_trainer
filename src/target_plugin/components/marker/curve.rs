#![allow(unused)]
use bevy_procedural_meshes::*;
use std::{f32, ops::Range, time::Duration};

use bevy::{
    animation::{AnimatedBy, animate_targets, animated_field},
    asset::{RenderAssetUsages, uuid::Uuid},
    color::palettes::tailwind::{BLUE_300, GREEN_400, RED_100, RED_800},
    math::curve,
    prelude::*,
};
use bevy_inspector_egui::egui::epaint::color;

use crate::{
    AppState, GameState,
    scoreing::Lifetime,
    target_plugin::{
        DebugMode, Target, TargetResource,
        events::{SpawnHint, SpawnTarget, TargetHit},
        target::FadeIn,
        target_material::TargetMaterial,
    },
};

pub struct CurvePlugin;
impl Plugin for CurvePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (curve_marker_gizmos.run_if(resource_equals(DebugMode(true))),),
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
    pub slides: usize,
    // duration of curve
    pub duration: f32,
}

#[derive(GizmoConfigGroup, Default, Reflect)]
pub struct CurveGizmo;

fn curve_marker_gizmos(
    q_curve_marker: Query<(Entity, (&Visibility, &mut CurveMarker))>,
    mut gizmos: Gizmos<CurveGizmo>,
) {
    for (_, (vis, curve)) in q_curve_marker {
        let color = if Visibility::Visible == *vis {
            BLUE_300
        } else {
            GREEN_400
        };

        gizmos.curve_3d(
            &curve.curve,
            (0..=400).map(|i| (i as f32 / 400.) * curve.curve.domain().end()),
            color,
        );
    }
}

fn on_spawn_hint(
    e: On<SpawnHint>,
    q_curve_marker: Query<(Entity, &mut CurveMarker)>,
    mut commands: Commands,
    q_marker: Query<&super::Marker>,
    mut animation_clips: ResMut<Assets<AnimationClip>>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    mut target_resource: Res<TargetResource>,
    time: Res<Time<Virtual>>,
) {
    let marker = q_marker.get(e.event_target()).unwrap();
    let Ok((entity, curve_marker)) = q_curve_marker.get(e.event_target()) else {
        return;
    };

    let preempt = marker.spawn_time.remaining();
    // show curve

    // TODO: new line material
    let mesh_curve = curve_marker.curve.clone().reparametrize(
        Interval::new(
            0.,
            (curve_marker.curve.domain().end() / curve_marker.slides as f32),
        )
        .unwrap(),
        |f| f,
    );
    let mesh = create_curve_hint(mesh_curve);
    info!("spawn hint");
    commands.spawn_scene(bsn! {
                Mesh3d(asset_value(mesh))
                Transform {
                    translation: vec3(0.,0.,-50.)
                }
                DespawnOnExit::<AppState>(AppState::InGame)
                MeshMaterial3d<StandardMaterial>(asset_value(StandardMaterial{unlit: true, ..StandardMaterial::from_color(GREEN_400)}))
                template_value(Lifetime::duration(preempt + Duration::from_secs_f32(curve_marker.duration)))
            });

    // INFO: Spawns curve
    let mut clip = AnimationClip::default();
    let curve = AnimatableCurve::new(
        animated_field!(Transform::translation),
        curve_marker.curve.clone(),
    );

    let anim_id = bevy::animation::AnimationTargetId(Uuid::from_u128(entity.index_u32() as u128));
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
                    MeshMaterial3d::<TargetMaterial>(asset_value(TargetMaterial {color: GREEN_400.into(), ring: 1., ..default()}))
                    Transform {
                        translation: {curve_marker.curve.sample_unchecked(0.)}
                    }
                    FadeIn({Timer::new(preempt, TimerMode::Once)})
                    DespawnOnExit::<AppState>(AppState::InGame)
                    template_value(Target::Duration(Duration::from_secs_f32(curve_marker.duration)))
                    template_value(player)
                    AnimationGraphHandle(asset_value(animation_graph))
                    template_value(Lifetime::duration(preempt + Duration::from_secs_f32(curve_marker.duration)))
                }).observe(move |e: On<SpawnTarget>, mut q_player: Query<&mut AnimationPlayer>, mut commands: Commands, time: Res<Time<Virtual>>, q_target_material: Query<&mut MeshMaterial3d<TargetMaterial>>, mut target_material: ResMut<Assets<TargetMaterial>>| {
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

    trace!("Spawned Curve");
}

fn create_curve_hint(curve: impl Curve<Vec3> + Clone) -> Mesh {
    // TODO: We need to take the curve and compute the tangent
    // -> Then at each sample point add to vertex to the positive and negative normal at half_width
    // -> Finally add end caps and tessellate the shape
    let half_width: f32 = 2.;

    let start = curve.domain().start();
    let domain = curve.domain();

    // Curve parameter
    let mut t = start;

    let step_size = curve.domain().length() / 200.;

    let mut vertices_top = vec![];
    let mut vertices_bottom = vec![];

    // Iterate over curve domain
    while domain.contains(t + step_size) {
        // Compute tangent at each point

        // SAFETY: t is always in the domain
        let sample = curve.sample_unchecked(t);
        let next_sample = curve.sample_unchecked(t + step_size);

        // Approximate tangent
        let tangent = next_sample - sample;

        // Get normalized orthogonal vector
        let v = tangent.rotate_z(f32::consts::FRAC_PI_2).normalize();
        assert_eq!(v.z, 0.);

        vertices_top.push((sample + (v * half_width), v));
        vertices_bottom.push(sample + (-v * half_width));

        // Advance by stepsize
        t += step_size;
    }

    // INFO: Loop slides is causing issues
    let mut mesh = PMesh::<u32>::new();
    mesh.fill(0.01, |builder| {
        builder.begin(vertices_top[0].0.xy());
        for vert in vertices_top {
            // builder.add_circle(vert.0.xy(), 0.1, Winding::Positive);
            builder.line_to(vert.0.xy());
        }
        //builder.add_circle(vec2(0., 0.), 1., Winding::Positive);

        // for vert in vertices_bottom.iter().rev() {
        //     builder.line_to(vert.xy());
        // }
        builder.close();
    });

    mesh.add_backfaces().to_bevy(RenderAssetUsages::all())
}

#[test]
fn curve_mesh() {
    let linear_curve = FunctionCurve::new(Interval::UNIT, |t| vec3(t, t, 0.));
    create_curve_hint(linear_curve);
}
