#![allow(unused)]
use bevy::{
    math::{
        NormedVectorSpace, VectorSpace,
        bounding::Bounded2d,
        cubic_splines::{self, LinearSpline},
    },
    prelude::*,
};
use rodio::Source;
use std::{
    f32::consts::TAU,
    fmt::Debug,
    fs::File,
    iter::Peekable,
    ops::{Neg, Sub},
    path::Path,
    time::Duration,
};

use crate::{
    AudioBuffer, EditMode, SceneTimer,
    target_plugin::{BeatMap, CurveMarker, TargetMarker, TargetResource},
};
use parser::{BeatMapOsu, Point, SliderParams, TimingPoint};

fn linear_curve(points: Vec<Vec2>, curve_duration: f32, slides: usize) -> SampleAutoCurve<Vec3> {
    points
        .array_windows::<2>()
        .map(|&s| {
            let c = LinearSpline::new(s)
                .to_curve()
                .unwrap()
                .map(|p| convert_osu_to_world(p))
                .reparametrize_linear(interval(0., s[0].distance(s[1])).unwrap())
                .unwrap();
            FunctionCurve::new(
                c.domain(),
                Box::new(move |t| c.sample_unchecked(t)) as Box<dyn Fn(f32) -> Vec3>,
            )
        })
        .reduce(|acc, elm| {
            let c = acc.chain(elm).unwrap();
            FunctionCurve::new(c.domain(), Box::new(move |t| c.sample_unchecked(t)))
        })
        .unwrap()
        .reparametrize_linear(Interval::new(0., curve_duration / slides as f32).unwrap())
        .unwrap()
        .ping_pong()
        .unwrap()
        .repeat(slides)
        .unwrap()
        .resample_auto(100 * slides)
        .unwrap()
}
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

fn to_vec2(p: Point) -> Vec2 {
    Vec2 {
        x: p.x as f32,
        y: p.y as f32,
    }
}
fn convert_osu_to_world(point: Vec2) -> Vec3 {
    let width = 30.;
    let height = 30.;
    let (max_x, max_y) = (512f32, 384f32);
    let x = (point.x as f32 / max_x) * width - width / 2.;
    let y = (1. - (point.y as f32 / max_y)) * height - height / 2.;
    vec3(x, y, -50.)
}

fn circle_from_3_points(a: Vec2, b: Vec2, c: Vec2) -> Vec2 {
    let m1 = (a + b) * 0.5;
    let m2 = (a + c) * 0.5;
    let ab = b - a;
    let ac = c - a;
    let perp_ab = Vec2::new(-ab.y, ab.x);
    let perp_ac = Vec2::new(-ac.y, ac.x);

    let denom = perp_ac.dot(perp_ab);
    if denom.abs() < 1e-6 {
        return m1;
    }
    let t = (m2 - m1).dot(perp_ac) / denom;
    m1 + t * perp_ab
}
pub fn osu(
    In(osu_beat_map): In<BeatMapOsu>,
    mut target_resource: ResMut<TargetResource>,
    mut commands: Commands,
    mut audio: ResMut<Assets<AudioBuffer>>,
    mut meshes: ResMut<Assets<Mesh>>,
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

    let BeatMapDecoderState {
        target_points: target_markers,
        target_curves,
        ..
    } = osu_beat_map.hit_objects.iter().fold(
        BeatMapDecoderState::new(osu_beat_map.timing_points),
        |mut state, hit_obj| {
            let t = hit_obj.time;

            if t > (state.previous_timing_point as usize)
                && let Some(next_timing_point) = state.timing_point_iter.peek()
                && t >= next_timing_point.time as usize
            {
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
            let ar = osu_beat_map.difficulty.approach_rate;

            // INFO: preempt calculation
            // AR < 5: preempt = 1200ms + 120ms * (5 - AR)
            // AR = 5: preempt = 1200ms
            // AR > 5: preempt = 1200ms - 150ms * (AR - 5)
            let preempt_ms = match ar {
                ar if (0. ..5.).contains(&ar) => 1200. + 120. * (5. - ar),
                ar if ar > 5. && ar <= 10. => 1200. - 150. * (ar - 5.),
                5. => 1200.,
                err => {
                    error!("approach rate must be [0,10] -> {ar}");
                    1200.
                }
            };

            if let Some(curve_params) = hit_obj.object_params.clone() {
                let SliderParams {
                    curve_points,
                    length,
                    slides,
                    curve_type,
                } = curve_params;

                assert!(slides >= 1);
                let points: Vec<_> = std::iter::once(hit_obj.position)
                    .chain(curve_points)
                    .map(|p| vec2(p.x as f32, p.y as f32))
                    .collect();

                let slider_multiplier = osu_beat_map.difficulty.slider_multiplier;
                // INFO: milliseconds it takes to complete one slide of the slider
                let curve_duration = (((length
                    / (slider_multiplier * 100. * state.slider_velocity))
                    * state.beat_length)
                    / 1000.)
                    * slides as f32;
                // info!("{:?}", curve_duration);

                match curve_type {
                    parser::CurveType::Bezier => {
                        let curve = points_to_bezier(points)
                            .reparametrize_linear(
                                Interval::new(0., curve_duration / slides as f32).unwrap(),
                            )
                            .unwrap()
                            .map(convert_osu_to_world)
                            .ping_pong()
                            .unwrap()
                            .repeat(slides)
                            .unwrap()
                            .resample_auto(100 * slides)
                            .unwrap();

                        state.target_curves.push(CurveMarker {
                            curve,
                            // truncated to nearest millisecond
                            duration: curve_duration,
                            lifetime: Timer::new(Duration::from_millis(t as u64), TimerMode::Once),
                            preempt: Duration::from_millis(preempt_ms as u64),
                        });
                    }
                    parser::CurveType::CentripetalCatmullRom => todo!(),
                    parser::CurveType::Linear => {
                        let curve = linear_curve(points, curve_duration, slides);
                        state.target_curves.push(CurveMarker {
                            curve,
                            duration: curve_duration,
                            lifetime: Timer::new(Duration::from_millis(t as u64), TimerMode::Once),

                            preempt: Duration::from_millis(preempt_ms as u64),
                        });
                    }
                    parser::CurveType::PerfectCircle => {
                        if points.len() != 3 {
                            // TODO: default to bezier for PerfectCircle with 3+ points
                            todo!()
                        }

                        let points: Vec<Vec2> = points
                            .clone()
                            .into_iter()
                            .map(|p| convert_osu_to_world(p).xy())
                            .collect();

                        let z = convert_osu_to_world(Vec2::ZERO).z;
                        let center = circle_from_3_points(points[0], points[1], points[2]);
                        let radius = center.distance(points[0]);

                        let curve = bevy::math::curve::FunctionCurve::new(Interval::UNIT, |i| {
                            let angle = i * TAU;
                            let xy = vec2(
                                center.x + radius * f32::cos(angle),
                                center.y + radius * f32::sin(angle),
                            );
                            xy.extend(z)
                        })
                        .reparametrize_linear(
                            Interval::new(0., curve_duration / slides as f32).unwrap(),
                        )
                        .unwrap()
                        .ping_pong()
                        .unwrap()
                        .repeat(slides)
                        .unwrap()
                        .resample_auto(100 * slides)
                        .unwrap();
                        state.target_curves.push(CurveMarker {
                            curve,
                            duration: curve_duration,
                            lifetime: Timer::new(Duration::from_millis(t as u64), TimerMode::Once),
                            preempt: Duration::from_millis(preempt_ms as u64),
                        });
                    }
                }
            } else {
                // Push target into beat_map
                state.target_points.push((
                    TargetMarker {
                        time: Timer::new(Duration::from_millis(t as u64), TimerMode::Once),
                        // TODO: Calculate preempt
                        preempt: Timer::new(
                            Duration::from_millis(preempt_ms as u64),
                            TimerMode::Once,
                        ),
                    },
                    Transform::from_translation(convert_osu_to_world(to_vec2(hit_obj.position))),
                ));
            }

            state
        },
    );

    commands.set_state(EditMode::Normal);
    // TODO: Leadin
    let leadin = Duration::from_millis(osu_beat_map.general.audio_lead_in as u64);

    target_resource.mesh = meshes.add(Sphere::new(osu_beat_map.difficulty.circle_size));
    let beat_map = BeatMap {
        song: audio.add(AudioBuffer(audio_buffer)),
        target_curves,
        target_markers,
    };
    stopwatch.reset();
    beat_map.spawn(commands.reborrow());
    commands.insert_resource(beat_map);
}

fn extract_nary_bezier(points: Vec<Vec2>) -> Vec<Vec<Vec2>> {
    let mut curves = vec![];
    let mut curr_curve = vec![];
    for [curr, next] in points.array_windows::<2>() {
        curr_curve.push(*curr);
        if curr == next {
            curves.push(curr_curve.clone());
            curr_curve.clear();
        }
    }
    // HACK:when curves have a n=1 section add it to previous instead of creating a 1 point bezier
    if curr_curve.is_empty() {
        curves.last_mut().unwrap().push(*points.last().unwrap());
    } else {
        curr_curve.push(*points.last().unwrap());
        curves.push(curr_curve);
    }
    curves
}
fn points_to_bezier(points: Vec<Vec2>) -> FunctionCurve<Vec2, impl Fn(f32) -> Vec2> {
    // set of nary bezier curves
    let curves = extract_nary_bezier(points);

    assert!(curves.iter().all(|curve| curve.len() >= 2));
    let curve = curves
        .into_iter()
        .map(|curve| {
            let c = FunctionCurve::new(Interval::UNIT, move |i| {
                let n = curve.len();
                let mut beta = curve.clone();
                for j in 1..n {
                    for k in 0..(n - j) {
                        beta[k] = beta[k] * (1. - i) + beta[k + 1] * i;
                    }
                }
                beta[0]
            });

            // Approx distance with linear segments
            let samples = c.samples(100).unwrap().collect::<Vec<_>>();
            let length = samples
                .clone()
                .into_iter()
                .collect::<Vec<_>>()
                .array_windows::<2>()
                .map(|[a, b]| a.distance(*b))
                .sum::<f32>();
            let c = c
                .reparametrize_linear(Interval::new(0., length).unwrap())
                .unwrap();
            FunctionCurve::new(
                c.domain(),
                Box::new(move |t| c.sample_unchecked(t)) as Box<dyn Fn(f32) -> Vec2>,
            )
        })
        .reduce(|acc, c| {
            let c = acc.chain(c).unwrap();
            FunctionCurve::new(c.domain(), Box::new(move |t| c.sample_unchecked(t)))
        })
        .unwrap();

    // let curve = bevy::math::curve::FunctionCurve::new(Interval::UNIT, move |i| {
    //     let n = curves.len();
    //     let curve_n = (i * (n - 1) as f32).ceil();
    //     let curve = &curves[curve_n as usize];
    //
    //     let n = curve.len();
    //     let mut beta = curve.clone();
    //     for j in 1..n {
    //         for k in 0..(n - j) {
    //             beta[k] = beta[k] * (1. - i) + beta[k + 1] * i;
    //         }
    //     }
    //     beta[0]
    // });
    curve
}

fn compare_curves<'l, I, T>(a: I, b: I) -> bool
where
    I: Curve<T> + CurveExt<T>,
    T: Debug + NormedVectorSpace<Scalar = f32>,
{
    let lhs: Vec<_> = a.samples(10).unwrap().collect();
    let rhs: Vec<_> = b.samples(10).unwrap().collect();
    for (r, l) in rhs.iter().zip(lhs.iter()) {
        println!("{l:?} = {r:?}");
    }
    lhs.into_iter()
        .zip(rhs.into_iter())
        .all(|(a, b)| (a - b).norm() < 0.001)
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
    assert!(compare_curves(true_curve, my_curve));
}

#[test]
fn bezier_2() {
    let points = vec![
        vec2(277.0, 184.0),
        vec2(208.0, 240.0),
        vec2(224.0, 352.0),
        vec2(224.0, 352.0),
        vec2(160.0, 256.0),
        vec2(64.0, 272.0),
        vec2(64.0, 272.0),
    ];
    dbg!(extract_nary_bezier(points.clone()));
    let curve_samples = points_to_bezier(points.clone())
        .samples(10)
        .unwrap()
        .collect::<Vec<_>>();
    info!("{curve_samples:?}");
    panic!()
}

#[test]
fn test_circle_from_3_points() {
    let a = vec2(1., 0.);
    let b = vec2(0., 1.);
    let c = vec2(-1., 0.);
    let center = circle_from_3_points(a, b, c);
    assert!(center.norm() < 0.001);

    let a = vec2(2., 0.);
    let b = vec2(0., 2.);
    let c = vec2(-2., 0.);
    let center = circle_from_3_points(a, b, c);
    assert!(center.norm() < 0.001);

    let a = vec2(5., 0.);
    let b = vec2(5., 5.);
    let c = vec2(0., 5.);
    let center = circle_from_3_points(a, b, c);
    assert!((center.x - 2.5).abs() < 0.001);
    assert!((center.y - 2.5).abs() < 0.001);
}
