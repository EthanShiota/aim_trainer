#![allow(unused)]
use bevy::color::palettes::css::{RED, TURQUOISE};
use bevy::color::palettes::tailwind::{RED_900, VIOLET_400};
use bevy::gltf::{self, GltfMesh, GltfPrimitive};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::pbr::wireframe::{Wireframe, WireframePlugin};
use bevy::render::render_resource::AsBindGroup;
use bevy_egui::egui::color_picker::show_color;
use procedural_modelling::extensions::bevy::show_faces;
use procedural_modelling::extensions::bevy::text::Text3dGizmo;
use procedural_modelling::mesh::{
    EmptyEdgePayload, EmptyFacePayload, FaceBasics, HalfEdge, HalfEdgeMesh, MeshBasics,
    MeshHalfEdgeBuilder,
};
use procedural_modelling::{
    extensions::bevy::{BevyMesh3d, BevyMeshType3d32, BevyVertexPayload3d},
    halfedge::*,
    math::*,
    mesh::MeshBuilder,
    operations::*,
};

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
        events::{CurveSoundEvent, SpawnHint, SpawnTarget, TargetHit},
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
        .add_plugins(MaterialPlugin::<CurveMarkerMaterial>::default())
        // .add_plugins(WireframePlugin::default())
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

#[derive(AsBindGroup, Clone, Asset, TypePath)]
pub struct CurveMarkerMaterial {
    #[uniform(100)]
    color: LinearRgba,
}

impl Material for CurveMarkerMaterial {
    fn fragment_shader() -> bevy::shader::ShaderRef {
        "shaders/curve_marker.wgsl".into()
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

        let curve_info = generate_curve_info(&curve.curve);
        for (idx, segment) in curve_info.iter().step_by(10).enumerate() {
            gizmos.arrow(
                segment.position,
                segment.position + segment.normal.normalize() * 2.,
                RED_900,
            );
            let samples: Vec<_> = math_helpers::sample_circle(
                10,
                Transform::from_rotation_arc(Vec3::NEG_X, segment.normal.normalize())
                    .with_translation(segment.position)
                    .compute_affine(),
                2.,
            );
            for sample in samples {
                gizmos.line(*sample.pos(), segment.position, VIOLET_400);
            }
        }

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
    asset_server: ResMut<AssetServer>,
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
    debug!("spawn hint");
    commands.spawn_scene(bsn! {
                Name("Curve Path")
                Mesh3d(asset_value(mesh))
                // Transform {
                //     translation: vec3(0.,0.,-50.)
                // }
                DespawnOnExit::<AppState>(AppState::InGame)
                MeshMaterial3d<CurveMarkerMaterial>(asset_value(CurveMarkerMaterial {color: GREEN_400.into()}))
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

    // INFO: add sound events
    let segment_duration = curve_marker.duration / curve_marker.slides as f32;
    for slide in 0..=curve_marker.slides {
        clip.add_event_to_target(
            anim_id,
            segment_duration * slide as f32,
            CurveSoundEvent(entity),
        );
    }

    let (animation_graph, animation_node_index) =
        AnimationGraph::from_clip(animation_clips.add(clip));

    let mut player = AnimationPlayer::default();

    let anim_duration = Duration::from_secs_f32(curve_marker.duration);
    let end_time = time.elapsed() + preempt + anim_duration;
    let mut slider = commands.entity(entity)
                .apply_scene(bsn! {
                    {target_resource.target_scene()}
                    Transform {
                        translation: {curve_marker.curve.sample_unchecked(0.)}
                    }
                    FadeIn({Timer::new(preempt, TimerMode::Once)})
                    template_value(Target::Duration(Duration::from_secs_f32(curve_marker.duration)))
                    template_value(player)
                    AnimationGraphHandle(asset_value(animation_graph))
                    template_value(Lifetime::duration(preempt + Duration::from_secs_f32(curve_marker.duration)))
                }).observe(move |e: On<SpawnTarget>, mut q_player: Query<&mut AnimationPlayer>, mut commands: Commands, time: Res<Time<Virtual>>, q_target_material: Query<&mut MeshMaterial3d<TargetMaterial>>, mut target_material: ResMut<Assets<TargetMaterial>>| {
                    let speedup = anim_duration.div_duration_f32(time.elapsed().abs_diff(end_time));
                    if let Ok(mut p) = q_player.get_mut(e.event_target()) {
                        debug!("begin curve: {} animation with speedup: {speedup}", e.event_target());
                        p.start(animation_node_index).set_speed(speedup);
                    }
                    commands.entity(e.event_target()).insert(Target::Duration(time.elapsed().abs_diff(end_time)));

                    commands.entity(e.observer()).despawn();
                }).observe(|e: On<TargetHit>, mut commands:  Commands| {
                    commands.entity(e.event_target()).trigger(SpawnTarget);
                    commands.entity(e.observer()).despawn();

                }).id();

    commands
        .entity(slider)
        .insert((anim_id, AnimatedBy(slider)));

    debug!("spawn curve {slider:?}");
}

struct Segment {
    position: Vec3,
    normal: Vec3,
    v: Vec3,
}

fn generate_curve_info(curve: impl Curve<Vec3> + Clone) -> Vec<Segment> {
    // TODO: We need to take the curve and compute the tangent
    // -> Then at each sample point add to vertex to the positive and negative normal at half_width
    // -> Finally add end caps and tessellate the shape
    let half_width: f32 = 2.;

    let start = curve.domain().start();
    let domain = curve.domain();

    // Curve parameter
    let mut t = start;

    let step_size = curve.domain().length() / 200.;

    let mut segements: Vec<Segment> = vec![];

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

        segements.push(Segment {
            position: sample,
            normal: tangent,
            v,
        });

        // Advance by stepsize
        t += step_size;
    }

    segements
}
mod math_helpers {
    use std::f32::consts::TAU;

    use bevy::{math::Affine3A, prelude::*};
    use procedural_modelling::{extensions::bevy::*, math::HasPosition};

    pub fn vp(vec: Vec3) -> BevyVertexPayload3d {
        BevyVertexPayload3d::from_pos(vec)
    }
    pub fn yz_circle(i: f32, r: f32) -> Vec3 {
        vec3(0., f32::sin(i) * r, f32::cos(i) * r)
    }

    pub fn circle(i: f32, t: Affine3A, r: f32) -> Vec3 {
        t.transform_point(yz_circle(i * TAU, r))
    }

    pub fn sample_circle(n: usize, t: Affine3A, r: f32) -> Vec<BevyVertexPayload3d> {
        (0..n)
            .map(|i| circle(i as f32 / n as f32, t, r))
            .map(vp)
            .collect::<Vec<_>>()
    }
}

macro_rules! debug_mesh {
    ($edge:expr, $mesh:expr) => {
        debug!("{} Primary: {:?}", line!(), $mesh.edge($edge));
        debug!("Twin: {:?}", $mesh.edge($edge).twin(&$mesh));
    };
}

fn create_curve_mesh(curve: impl Curve<Vec3> + Clone) -> Mesh {
    let segments = generate_curve_info(curve);
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    // Add 4 vertices, each with its own position attribute (coordinate in
    // 3D space), for each of the corners of the parallelogram.
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 2.0, 0.0],
            [2.0, 2.0, 0.0],
            [1.0, 0.0, 0.0],
        ],
    )
    // Assign a UV coordinate to each vertex.
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0, 1.0], [0.5, 0.0], [1.0, 0.0], [0.5, 1.0]],
    )
    // Assign normals (everything points outwards)
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        vec![
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ],
    )
    // After defining all the vertices and their attributes, build each triangle using the
    // indices of the vertices that make it up in a counter-clockwise order.
    .with_inserted_indices(Indices::U32(vec![
        // First triangle
        0, 3, 1, // Second triangle
        1, 3, 2,
    ]))
}

fn create_curve_hint(curve: impl Curve<Vec3> + Clone) -> Mesh {
    // TODO: We need to take the curve and compute the tangent
    // -> Then at each sample point add to vertex to the positive and negative normal at half_width
    // -> Finally add end caps and tessellate the shape
    let mut mesh = BevyMesh3d::default();
    let segments = generate_curve_info(curve);

    let mut prior_edge = None;
    for [segment, next_segment] in segments.array_windows::<2>() {
        let transform = Transform::from_rotation_arc(Vec3::NEG_X, segment.normal.normalize())
            .with_translation(segment.position);

        let next_transform =
            Transform::from_rotation_arc(Vec3::NEG_X, next_segment.normal.normalize())
                .with_translation(next_segment.position);

        let points: Vec<_> = math_helpers::sample_circle(22, transform.compute_affine(), 2.)
            .into_iter()
            .rev()
            .collect();

        prior_edge = if let Some(prior_edge) = prior_edge {
            let twin = mesh.loft_tri_closed(prior_edge, points);
            Some(twin)
        } else {
            // insert current edge
            let edge = mesh.insert_loop(points);
            // close first edge
            mesh.close_hole_default(mesh.edge(edge).twin_id());

            Some(edge)
        };
    }

    mesh.to_bevy_ex(
        RenderAssetUsages::all(),
        procedural_modelling::tesselate::TriangulationAlgorithm::Delaunay,
        true,
    )
}

#[test]
fn curve_mesh() {
    let linear_curve = FunctionCurve::new(Interval::UNIT, |t| vec3(t, t, 0.));
    create_curve_hint(linear_curve);
}
