use bevy::prelude::*;
use rodio::Source;
use std::{fs::File, time::Duration};

use crate::{
    AudioBuffer, EditMode, GameState, GameStats,
    target_spawner::{SpawnerState, SpawnerVolume, TargetSpawner},
};

pub fn basic(mut commands: Commands, mut audio: ResMut<Assets<AudioBuffer>>) {
    let audio_file = File::open("samples/Ian Asher & Phantogram- Black Out Days.wav").unwrap();
    let decoder = rodio::decoder::Decoder::try_from(audio_file).unwrap();

    let audio_buffer = rodio::buffer::SamplesBuffer::new(
        decoder.channels(),
        decoder.sample_rate(),
        decoder.collect::<Vec<_>>(),
    );
    log::info!("audio buffer size: {}", audio_buffer.len());

    commands.spawn((
        AudioPlayer(audio.add(AudioBuffer(audio_buffer))),
        PlaybackSettings {
            volume: bevy::audio::Volume::Linear(0.5),
            spatial: false,
            ..default()
        },
    ));
    commands.set_state(EditMode::Editing);
    commands.insert_resource(GameStats::default());
    let scene = bsn! {
        SpawnerVolume
        TargetSpawner::new(Duration::from_secs_f64(0.5), Some(10))
        Transform
    };
    commands.spawn_scene(scene);
}
