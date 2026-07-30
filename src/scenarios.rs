#![allow(unused)]
use bevy::{
    math::{
        bounding::Bounded2d,
        cubic_splines::{self, LinearSpline},
    },
    prelude::*,
};
use rodio::Source;
use std::{f32::consts::TAU, fs::File, iter::Peekable, path::Path, time::Duration};

use crate::{
    AudioBuffer, EditMode, GameStats, SceneTimer,
    target_plugin::{BeatMap, CurveMarker, TargetMarker},
};
use parser::{BeatMapOsu, Point, SliderParams, TimingPoint};
struct BeatMapDecoderState {
    target_points: Vec<(TargetMarker, Transform)>,
    target_curves: Vec<CurveMarker>,
    beat_length: f32,
    slider_velocity: f32,
    previous_timing_point: f32,
    timing_point_iter: Peekable<vec::IntoIter<TimingPoint>>,
}

impl BeatMapDecoderState {
    fn new(timing_points: Vec<TimingPoint>) -> Self {
        let mut iter = timing_points.into_iter().peekable();
        let timing_point = iter.next().unwrap();
        assert!(timing_point.uninherited);
        Self {
            target_points: default(),
            target_curves: default(),
            beat_length: timing_point.beat_length,
            slider_velocity: 1.,
            previous_timing_point: timing_point.time,
            timing_point_iter: iter,
        }
    }
}
pub fn osu(
    In(osu_beat_map): In<BeatMapOsu>,
    mut commands: Commands,
    mut audio: ResMut<Assets<AudioBuffer>>,
    // beat_map_path: Res<BeatMapPath>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
    mut stopwatch: ResMut<SceneTimer>,
) {
    // let f = File::open(&beat_map_path.0).unwrap();
    // let osu_beat_map = osu_parser::BeatMapOsu::new(f).unwrap();

    let audio_file = osu_beat_map
        .beat_map_path
        .parent()
        .unwrap()
        .join(Path::new(&osu_beat_map.general.audio_filename));
    let audio_file = File::open(audio_file).unwrap();
    let decoder = rodio::decoder::Decoder::try_from(audio_file).unwrap();

    let audio_buffer = rodio::buffer::SamplesBuffer::new(
        decoder.channels(),
        decoder.sample_rate(),
        decoder.collect::<Vec<_>>(),
    );

    commands.set_state(EditMode::Normal);
    commands.insert_resource(GameStats::default());
    // let leadin = Duration::from_millis(osu_beat_map.general.audio_lead_in as u64);
    // stopwatch.pause();
    // commands
    //     .delayed()
    //     .duration(leadin)
    //     .insert_resource(SceneTimer::default());

    let width = 50.;
    let height = 20.;
    // TODO: Fix length, add cycles (slides)
    let BeatMapDecoderState {
        target_points: target_markers,
        target_curves,
        ..
    } = osu_beat_map.hit_objects.iter().fold(
        BeatMapDecoderState::new(osu_beat_map.timing_points),
        |mut state, hit_obj| {
            let (max_x, max_y) = (512f32, 384f32);
            let map_point = |point: Point| {
                let x = (point.x as f32 / max_x) * width - width / 2.;
                let y = (1. - (point.y as f32 / max_y)) * height - height / 2.;
                (x, y)
            };
            let (x, y) = map_point(hit_obj.position);
            let t = hit_obj.time;

            if t > (state.previous_timing_point as usize) {
                if let Some(next_timing_point) = state.timing_point_iter.peek() {
                    if t >= next_timing_point.time as usize {
                        // apply next timing point
                        if next_timing_point.uninherited {
                            state.beat_length = next_timing_point.beat_length;
                        } else {
                            // slider velocity
                            state.slider_velocity = -(1. / (next_timing_point.beat_length / 100.));
                        }
                        state.previous_timing_point = next_timing_point.time;
                        state.timing_point_iter.next();
                    }
                }
            }

            if let Some(curve_params) = hit_obj.object_params.clone() {
                let SliderParams {
                    curve_points,
                    length,
                    slides,
                    curve_type,
                } = curve_params;
                let points: Vec<_> = std::iter::once(vec2(x, y))
                    .chain(curve_points.iter().map(|p| map_point(*p).into()))
                    .collect();

                let slider_multiplier = osu_beat_map.difficulty.slider_multiplier;
                // INFO: milliseconds it takes to complete one slide of the slider
                let curve_duration = ((length
                    / (slider_multiplier * 100. * state.slider_velocity))
                    * state.beat_length)
                    / 1000.
                    * slides as f32;
                info!("{:?}", curve_duration);

                // TODO: Implement slides
                match curve_type {
                    parser::CurveType::Bezier => {
                        state.target_curves.push(CurveMarker {
                            curve: points_to_bezier(points)
                                .reparametrize_linear(Interval::new(0., curve_duration).unwrap())
                                .unwrap()
                                .resample_auto(100)
                                .unwrap(),
                            // truncated to nearest millisecond
                            duration: curve_duration,
                            lifetime: Timer::new(Duration::from_millis(t as u64), TimerMode::Once),
                        });
                    }
                    parser::CurveType::CentripetalCatmullRom => todo!(),
                    parser::CurveType::Linear => {
                        // Linear path between all points
                        // curve_points
                        let linear_spline = LinearSpline::new(points);
                        let curve = linear_spline
                            .to_curve()
                            .unwrap()
                            // TODO: Map 3d better
                            .map(|p| p.extend(-50.))
                            .resample_auto(100)
                            .unwrap();
                        // target_curves.push(CurveMarker {
                        //     curve,
                        //     lifetime: Timer::new(
                        //         Duration::from_millis(t as u64),
                        //         TimerMode::Once,
                        //     ),
                        // });
                    }
                    parser::CurveType::PerfectCircle => {
                        // TODO: PerfectCircle
                        if points.len() != 3 {
                            // TODO: default to bezier for PerfectCircle with 3+ points
                            todo!()
                        }

                        let bounding_circle = bevy::math::primitives::Triangle2d::new(
                            points[0], points[1], points[2],
                        )
                        .bounding_circle(Isometry2d::IDENTITY);
                        let r = bounding_circle.radius();
                        let p = bounding_circle.center;
                        let curve = bevy::math::curve::FunctionCurve::new(Interval::UNIT, |i| {
                            // TODO: Map 3d better
                            vec3(
                                f32::sin(i * TAU) * r + p.x,
                                f32::cos(i * TAU) * r + p.y,
                                -50.,
                            )
                        })
                        .resample_auto(100)
                        .unwrap();
                        // target_curves.push(CurveMarker {
                        //     curve,
                        //     lifetime: Timer::new(
                        //         Duration::from_millis(t as u64),
                        //         TimerMode::Once,
                        //     ),
                        // });
                    }
                }
            }

            state.target_points.push((
                TargetMarker(Duration::from_millis(t as u64)),
                Transform::from_xyz(x, y, -50.),
            ));
            state
        },
    );
    let beat_map = BeatMap {
        song: audio.add(AudioBuffer(audio_buffer)),
        target_curves,
        target_markers,
    };
    stopwatch.reset();
    beat_map.spawn(commands.reborrow(), meshes, materials);
    commands.insert_resource(beat_map);
}

fn points_to_bezier(points: Vec<Vec2>) -> SampleAutoCurve<Vec3> {
    // set of nary bezier curves
    let mut curves = vec![];
    let mut curr_curve = vec![];
    for [curr, next] in points.array_windows::<2>() {
        curr_curve.push(*curr);
        if curr == next {
            curves.push(curr_curve.clone());
            curr_curve.clear();
        }
    }
    curr_curve.push(*points.last().unwrap());
    // FIX: Bezier Curves are broken I think
    curves.push(curr_curve);

    let curve = bevy::math::curve::FunctionCurve::new(Interval::UNIT, |i| {
        let n = curves.len();
        let curve_n = (i * (n - 1) as f32).floor();
        let curve = &curves[curve_n as usize];

        let n = curve.len();
        let mut beta = curve.clone();
        for j in 1..n {
            for k in 0..(n - j) {
                beta[k] = beta[k] * (1. - i) + beta[k + 1] * i;
            }
        }
        // TODO: Map 3d better
        beta[0].extend(-50.)
    })
    .resample_auto(100)
    .unwrap();
    curve
}

#[test]
fn bezier() {
    let points = vec![[vec2(0., 0.), vec2(0., 1.), vec2(1., 1.), vec2(2., 1.)]];
    let spline = cubic_splines::CubicBezier::new(points.clone())
        .to_curve()
        .unwrap();
    let true_curve = spline.resample_auto(100).unwrap();
    let my_curve = points_to_bezier(points[0].to_vec())
        .map(|p| p.xy())
        .resample_auto(100)
        .unwrap();
    let lhs: Vec<_> = true_curve.samples(10).unwrap().collect();
    let rhs: Vec<_> = my_curve.samples(10).unwrap().collect();
    for (r, l) in rhs.iter().zip(lhs.iter()) {
        println!("{l:?} = {r:?}");
    }
    assert!(
        lhs.iter()
            .zip(rhs.iter())
            .all(|(a, b)| { (a - b).length().abs() < 0.001 })
    );
}
