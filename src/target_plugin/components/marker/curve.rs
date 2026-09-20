#![allow(unused)]
use bevy::color::palettes::css::{LIGHT_BLUE, RED, TURQUOISE};
use bevy::color::palettes::tailwind::{RED_900, VIOLET_400};
use bevy::gltf::{self, GltfMesh, GltfPrimitive};
use bevy::math::{Affine3A, DAffine3, DQuat, DVec3};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::pbr::wireframe::{Wireframe, WireframePlugin};
use bevy::render::render_resource::AsBindGroup;
use bevy_egui::egui::color_picker::show_color;

use std::{f32, ops::Range, time::Duration};

use bevy::{
    animation::{AnimatedBy, animate_targets, animated_field},
    asset::{RenderAssetUsages, uuid::Uuid},
    color::palettes::tailwind::{BLUE_300, GREEN_400, RED_100, RED_800},
    math::curve,
    prelude::*,
};
use bevy_inspector_egui::egui::epaint::color;

use crate::target_plugin::components::beat_map::BeatMapResource;
use crate::target_plugin::components::marker::curve::math_helpers::{circle, sample_circle};
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
        for (idx, segment) in curve_info.iter().enumerate() {
            gizmos.arrow(
                segment.position,
                segment.position + segment.normal.normalize() * 2.,
                RED_900,
            );
            let samples: Vec<_> = math_helpers::sample_circle(
                10,
                Transform::from_rotation(Quat::from_rotation_arc(
                    Vec3::NEG_X,
                    segment.normal.normalize(),
                ))
                .with_translation(segment.position)
                .compute_affine(),
                2.,
            );
            for sample in samples {
                gizmos.line(sample.position, segment.position, VIOLET_400);
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
    q_curve_marker: Query<(Entity, &super::Marker, &mut CurveMarker)>,
    mut commands: Commands,
    mut animation_clips: ResMut<Assets<AnimationClip>>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    mut target_resource: Res<TargetResource>,
    asset_server: ResMut<AssetServer>,
    time: Res<Time<Virtual>>,
    beat_map: If<Res<BeatMapResource>>,
) {
    let Ok((entity, marker, curve_marker)) = q_curve_marker.get(e.event_target()) else {
        return;
    };

    let preempt = marker.spawn_time - marker.preempt;
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
    let mesh = create_curve_mesh(mesh_curve);
    debug!("spawn hint");
    // TODO: Make the curve despawn when the animation is done
    commands.spawn_scene(bsn! {
        Name("Curve Path")
        Mesh3d(asset_value(mesh))
        // Transform {
        //     translation: vec3(0.,0.,-50.)
        // }
        DespawnOnExit::<AppState>(AppState::InGame)
        // MeshMaterial3d<CurveMarkerMaterial>(asset_value(CurveMarkerMaterial {color: GREEN_400.into()}))

        MeshMaterial3d<StandardMaterial>(asset_value(StandardMaterial::from_color(LIGHT_BLUE)))
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
    for slide in 1..=curve_marker.slides {
        let last = slide == curve_marker.slides;
        clip.add_event_to_target(
            anim_id,
            segment_duration * slide as f32,
            CurveSoundEvent { entity, last },
        );
    }

    let (animation_graph, animation_node_index) =
        AnimationGraph::from_clip(animation_clips.add(clip));

    let mut player = AnimationPlayer::default();

    let anim_duration = Duration::from_secs_f32(curve_marker.duration);
    let end_time = time.elapsed() + preempt + anim_duration;
    let mut slider = commands
        .entity(entity)
        .apply_scene(bsn! {
            {target_resource.target_scene()}
            Transform {
                translation: {curve_marker.curve.sample_unchecked(0.)}
            }
            FadeIn({Timer::new(preempt, TimerMode::Once)})
            template_value(Target::Duration(Duration::from_secs_f32(curve_marker.duration)))
            template_value(player)
            AnimationGraphHandle(asset_value(animation_graph))
        })
        .observe(
            move |e: On<SpawnTarget>,
                  mut q_player: Query<&mut AnimationPlayer>,
                  mut commands: Commands,
                  time: Res<Time<Virtual>>| {
                if let Ok(mut p) = q_player.get_mut(e.event_target()) {
                    p.start(animation_node_index);
                }
                commands
                    .entity(e.event_target())
                    .insert(Target::Duration(anim_duration))
                    .insert(Lifetime::duration(anim_duration));

                commands.entity(e.observer()).despawn();
            },
        )
        .observe(
            |e: On<TargetHit>, mut commands: Commands, asset_server: ResMut<AssetServer>| {
                commands.entity(e.event_target()).trigger(SpawnTarget);
                commands.entity(e.event_target()).insert((
                    AudioPlayer::new(asset_server.load("audio/Creams.ogg")),
                    PlaybackSettings::REMOVE,
                ));

                commands.entity(e.observer()).despawn();
            },
        )
        .id();

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

/// Takes curve and generates a position and derivative at some number of sample points
fn generate_curve_info(curve: impl Curve<Vec3> + Clone) -> Vec<Segment> {
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

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Vertex {
        pub position: Vec3,
        pub normal: Vec3,
    }
    pub fn yz_circle(i: f32, r: f32) -> Vertex {
        let pos = vec3(0., f32::sin(i) * r, f32::cos(i) * r);
        Vertex {
            position: pos,
            // Normal is position because we are at the origin
            normal: pos,
        }
    }

    pub fn circle(i: f32, t: Affine3A, r: f32) -> Vertex {
        let circ = yz_circle(i * TAU, r);
        Vertex {
            position: t.transform_point3(circ.position),
            normal: t.transform_vector3(circ.normal),
        }
    }

    pub fn sample_circle(n: usize, t: Affine3A, r: f32) -> Vec<Vertex> {
        (0..n)
            .map(|i| circle(i as f32 / n as f32, t, r))
            .collect::<Vec<_>>()
    }

    mod test {
        use super::*;
        use std::f32::consts::*;

        fn is_close(a: Vec3, b: Vec3, tol: f32) -> bool {
            dbg!((dbg!(a) - dbg!(b)).abs().length() < tol)
        }

        #[test]
        fn circle_test() {
            assert!(is_close(yz_circle(0., 1.).position, vec3(0., 0., 1.), 0.01));
            assert!(is_close(
                yz_circle(PI, 1.).position,
                vec3(0., 0., -1.),
                0.01
            ));

            assert!(is_close(
                yz_circle(FRAC_PI_2, 1.).position,
                vec3(0., f32::sin(FRAC_PI_2), f32::cos(FRAC_PI_2)),
                0.06
            ));
        }

        #[test]
        fn circle_transformed() {
            let transform = Affine3A::from_quat(Quat::from_rotation_arc(Vec3::X, Vec3::Z));
            assert!(is_close(
                circle(0., transform, 1.).position,
                vec3(-1., 0., 0.),
                0.01
            ));

            assert!(is_close(
                circle(0., transform, 1.).normal,
                vec3(-1., 0., 0.),
                0.01
            ));
        }
    }
}

macro_rules! debug_mesh {
    ($edge:expr, $mesh:expr) => {
        debug!("{} Primary: {:?}", line!(), $mesh.edge($edge));
        debug!("Twin: {:?}", $mesh.edge($edge).twin(&$mesh));
    };
}

fn create_curve_mesh(curve: impl Curve<Vec3> + Clone) -> Mesh {
    let resolution = 32;
    let radius = 1.;

    let segments = generate_curve_info(curve);

    let vertex_count = resolution * segments.len();
    let segment_count = segments.len();
    // 0..resolution are ring 0

    // Should be fine
    let caps = [
        segments.first().expect("Curve has zero segments"),
        segments.last().expect("Curve has zero segments"),
    ];
    let cap_vertices = [
        math_helpers::Vertex {
            position: caps[0].position,
            normal: -caps[0].normal.normalize(),
        },
        math_helpers::Vertex {
            position: caps[1].position,
            normal: caps[1].normal.normalize(),
        },
    ];

    let vertices = segments
        .into_iter()
        .flat_map(|segment: Segment| {
            // Transform to apply to ring at the segment
            let transform = Affine3A::from_quat(
                DQuat::from_rotation_arc(DVec3::NEG_X, segment.normal.normalize().as_dvec3())
                    .as_quat(),
            );
            let mut circ = sample_circle(resolution, transform, radius);
            circ.iter_mut().for_each(|v| v.position += segment.position);
            circ
        })
        .chain(cap_vertices)
        .collect::<Vec<_>>();

    let diffs: Vec<_> = vertices
        .array_windows::<2>()
        .map(|[a, b]| a.normal.angle_between(b.normal))
        .collect();

    // Two vertices for end caps
    assert!(vertices.len() - 2 == vertex_count);

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vertices
            .iter()
            .map(|v| v.position.to_array())
            .collect::<Vec<_>>(),
    )
    // Assign a UV coordinate to each vertex.
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_UV_0,
        (0..vertex_count)
            .into_iter()
            .map(|i| {
                [
                    ((i % resolution) as f32 / resolution as f32),
                    (segment_count as f32 - (i / resolution) as f32) / segment_count as f32,
                ]
            })
            .chain(
                // bottom, top
                [[0.0, 1.], [0.0, 0.0]],
            )
            .collect::<Vec<_>>(),
    )
    // Assign normals (everything points outwards)
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        vertices.into_iter().map(|v| v.normal).collect::<Vec<_>>(),
    )
    // After defining all the vertices and their attributes, build each triangle using the
    // indices of the vertices that make it up in a counter-clockwise order.
    .with_inserted_indices(Indices::U32(
        // Iterate all but last row
        // each vertex gets 2 faces
        (0..vertex_count - resolution)
            .flat_map(|i| {
                // INFO: Wrap shape with faces
                let ring_base = (i / resolution) * resolution;
                let next_ring_base = ring_base + resolution;
                let local_index = |i| i % resolution;

                [
                    ring_base + local_index(i),
                    ring_base + local_index(i + 1),
                    next_ring_base + local_index(i),
                    next_ring_base + local_index(i),
                    ring_base + local_index(i + 1),
                    next_ring_base + local_index(i + 1),
                ]
            })
            .chain(
                // Bottom and top caps
                (0..=1).flat_map(|b| {
                    // 0 -> bottom
                    // 1 -> top
                    let base = b * (vertex_count - resolution);
                    let cap_vertex_index = vertex_count + b;

                    // walk edge and make triangles which connect to the cap vertex
                    (0..resolution).flat_map(move |i| {
                        let local_index = |i| i % resolution;
                        [
                            base + local_index(i + (1 - b)),
                            base + local_index(i + b),
                            cap_vertex_index,
                        ]
                    })
                }),
            )
            .map(|val| val as u32)
            .collect::<Vec<u32>>(),
    ))
}
