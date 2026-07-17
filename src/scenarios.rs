use bevy::prelude::*;
use rodio::Source;
use std::{fs::File, path::Path, time::Duration};

use crate::{
    AudioBuffer, EditMode, GameStats, SceneTimer,
    osu_parser::BeatMapOsu,
    target_plugin::{BeatMap, TargetMarker},
};

pub fn basic(
    mut commands: Commands,
    mut audio: ResMut<Assets<AudioBuffer>>,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
) {
    let audio_file = File::open("samples/Ian Asher & Phantogram- Black Out Days.wav").unwrap();
    let decoder = rodio::decoder::Decoder::try_from(audio_file).unwrap();

    let audio_buffer = rodio::buffer::SamplesBuffer::new(
        decoder.channels(),
        decoder.sample_rate(),
        decoder.collect::<Vec<_>>(),
    );
    log::info!("audio buffer size: {}", audio_buffer.len());

    // commands.spawn((
    //     AudioPlayer(audio.add(AudioBuffer(audio_buffer))),
    //     PlaybackSettings {
    //         volume: bevy::audio::Volume::Linear(0.5),
    //         spatial: false,
    //         ..default()
    //     },
    // ));
    commands.set_state(EditMode::Editing);
    commands.insert_resource(GameStats::default());
    let beat_map = BeatMap {
        song: audio.add(AudioBuffer(audio_buffer)),
        hit_targets: Vec::new(),
    };
    beat_map.spawn(commands.reborrow(), meshes, materials);
    commands.insert_resource(beat_map);
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
    let hit_targets = osu_beat_map
        .hit_objects
        .iter()
        .filter_map(|hit_obj| {
            // if hit_obj.type_bitmask & 1 == 1 {
            //     return None;
            // }
            let (max_x, max_y) = (512f32, 384f32);
            let x = (hit_obj.position.x as f32 / max_x) * width - width / 2.;
            let y = (1. - (hit_obj.position.y as f32 / max_y)) * height - height / 2.;
            let t = hit_obj.time;

            Some((
                TargetMarker(Duration::from_millis(t as u64)),
                Transform::from_xyz(x, y, -50.),
            ))
        })
        .collect();
    let beat_map = BeatMap {
        song: audio.add(AudioBuffer(audio_buffer)),
        hit_targets,
    };
    stopwatch.reset();
    beat_map.spawn(commands.reborrow(), meshes, materials);
    commands.insert_resource(beat_map);
}
