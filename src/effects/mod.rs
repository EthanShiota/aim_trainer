use crate::{
    AppState, SoundSettings, effects,
    target::{self, Marker, TargetMaterial, events::CurveSoundEvent},
};
use bevy::prelude::*;

use super::input::Hovered;
pub struct EffectPlugin;

pub fn hit_sound(effects_volume: f32) -> impl Scene {
    bsn! {
        AudioPlayer("audio/Creams.ogg")
        PlaybackSettings {
            volume: bevy::audio::Volume::Linear(effects_volume),
            mode: bevy::audio::PlaybackMode::Remove
        }
        DespawnOnExit::<AppState>(AppState::InGame)
    }
}

pub fn on_sound_event(
    e: On<CurveSoundEvent>,
    hovered: Query<(), (With<Hovered>, With<target::Active>)>,
    sound_settings: Res<SoundSettings>,
    mut commands: Commands,
) {
    if hovered.contains(e.entity) {
        commands.spawn_scene(effects::hit_sound(sound_settings.effects_volume));
    }
    if e.last {
        commands.entity(e.entity).try_despawn();
    }
}
