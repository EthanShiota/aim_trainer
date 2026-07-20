use bevy::prelude::*;
use rodio::Source;
use std::{fs::File, path::Path, time::Duration};

use crate::{
    AudioBuffer, EditMode, GameStats, SceneTimer,
    osu_parser::{BeatMapOsu, Point, SliderParams},
    target_plugin::{BeatMap, CurveMarker, TargetMarker},
};

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
    let (target_markers, target_curves) =
        osu_beat_map
            .hit_objects
            .iter()
            .fold((vec![], vec![]), |acc, hit_obj| {
                let (mut target_points, mut target_curves) = acc;
                // if hit_obj.type_bitmask & 1 == 1 {
                //     return None;
                // }
                let (max_x, max_y) = (512f32, 384f32);
                let map_point = |point: Point| {
                    let x = (point.x as f32 / max_x) * width - width / 2.;
                    let y = (1. - (point.y as f32 / max_y)) * height - height / 2.;
                    (x, y)
                };
                let (x, y) = map_point(hit_obj.position);
                let t = hit_obj.time;

                if let Some(curve_params) = hit_obj.object_params.clone() {
                    let SliderParams {
                        curve_points,
                        length,
                        slides,
                        curve_type,
                    } = curve_params;
                    match curve_type {
                        crate::osu_parser::CurveType::Bezier => {
                            let points: Vec<_> = std::iter::once(vec2(x, y))
                                .chain(curve_points.iter().map(|p| map_point(*p).into()))
                                .collect();

                            // set of nary bezier curves
                            let mut curves = vec![];
                            let mut curr_curve = vec![];
                            for window in points.windows(2) {
                                if let [curr, next] = window {
                                    if curr != next {
                                        curr_curve.push(*curr);
                                    } else {
                                        curves.push(curr_curve.clone());
                                        curr_curve.clear();
                                    }
                                } else {
                                    panic!()
                                }
                            }
                            curr_curve.push(*points.last().unwrap());
                            curves.push(curr_curve);

                            let curve = bevy::math::curve::FunctionCurve::new(
                                Interval::new(0., 1.).unwrap(),
                                |i| {
                                    let n = curves.len();
                                    let curve_n = (i * n as f32).floor();
                                    let curve = &curves[curve_n as usize];

                                    let n = curve.len();
                                    let mut beta = curve.clone();
                                    let i = curve_n.fract();
                                    for j in 1..n {
                                        for k in 0..(n - j) {
                                            beta[k] = beta[k] * (1. - i) + beta[k + 1] * i;
                                        }
                                    }
                                    beta[0].extend(0.)
                                },
                            )
                            .resample_auto(100)
                            .unwrap();

                            target_curves.push(CurveMarker {
                                curve,
                                lifetime: Timer::new(Duration::from_secs_f32(32.), TimerMode::Once),
                            });
                        }
                        crate::osu_parser::CurveType::CentripetalCatmullRom => todo!(),
                        crate::osu_parser::CurveType::Linear => todo!(),
                        crate::osu_parser::CurveType::PerfectCircle => todo!(),
                    }
                }

                target_points.push((
                    TargetMarker(Duration::from_millis(t as u64)),
                    Transform::from_xyz(x, y, -50.),
                ));
                (target_points, target_curves)
            });
    let beat_map = BeatMap {
        song: audio.add(AudioBuffer(audio_buffer)),
        target_curves,
        target_markers,
    };
    stopwatch.reset();
    beat_map.spawn(commands.reborrow(), meshes, materials);
    commands.insert_resource(beat_map);
}
