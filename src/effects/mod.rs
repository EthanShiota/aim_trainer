use crate::{
    AppState, SoundSettings, effects,
    scoreing::SliderScorer,
    target::{self, Active, Marker, TargetMaterial, events::*},
};

use crate::scoreing::Score;
use bevy::{audio::PlaybackMode::Remove, prelude::*};
use tracing::{Level, event, instrument};

pub struct EffectPlugin;

impl Plugin for EffectPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_slider_head)
            .add_observer(on_slider_tail)
            .add_observer(on_slider_tick)
            .add_observer(on_slider_repeat);
    }
}

pub fn hit_effect(points: f64, transform: &Transform) -> impl Scene {}

pub fn hit_sound(sound_settings: Res<SoundSettings>) -> impl Scene {
    bsn! {
        AudioPlayer(
            {
                if let Some(sfx) = sound_settings.sfx_override.clone() {sfx} else {"audio/Creams.ogg".to_string()}
            }
        )
        PlaybackSettings {
            volume: bevy::audio::Volume::Linear({sound_settings.effects_volume}),
            mode: bevy::audio::PlaybackMode::Remove
        }
        DespawnOnExit::<AppState>(AppState::InGame)
    }
}

#[instrument(skip_all)]
pub fn on_slider_tick(
    e: On<SliderTick>,
    mut score: ResMut<Score>,
    mut active: Query<&mut SliderScorer, With<Active>>,
) {
    if let Ok(mut scorer) = active.get_mut(e.0) {
        scorer.hits += 1;
        score.points += 10.;
        score.combo += 1;
    }
}

#[instrument(skip_all)]
pub fn on_slider_head(
    e: On<SliderHead>,
    mut active: Query<&mut SliderScorer, With<Active>>,
    sound_settings: Res<SoundSettings>,
    mut commands: Commands,
) {
    if let Ok(mut scorer) = active.get_mut(e.0) {
        scorer.hits += 1;
        commands.spawn_scene(hit_sound(sound_settings));
    }
}

#[instrument(skip_all)]
pub fn on_slider_tail(
    e: On<SliderTail>,
    mut score: ResMut<Score>,
    active: Query<(), With<Active>>,
    mut scorer: Query<&mut SliderScorer>,
    sound_settings: Res<SoundSettings>,
    mut commands: Commands,
) {
    let mut scoerer = scorer.get_mut(e.0).unwrap();

    if active.contains(e.0) {
        scoerer.hits += 1;
        score.points += 30.;
        commands.spawn_scene(hit_sound(sound_settings));
    }
    score.score_hit(scoerer.score());
    commands.entity(e.0).trigger(RemoveCurve);
}

#[instrument(skip_all)]
pub fn on_slider_repeat(
    e: On<SliderRepeat>,
    mut score: ResMut<Score>,
    mut active: Query<&mut SliderScorer, With<Active>>,
    sound_settings: Res<SoundSettings>,
    mut commands: Commands,
) {
    if let Ok(mut scorer) = active.get_mut(e.0) {
        scorer.hits += 1;
        score.points += 30.;
        commands.spawn_scene(hit_sound(sound_settings));
    }
}
