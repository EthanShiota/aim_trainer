use crate::{
    AppState,
    target_plugin::{Marker, TargetMaterial},
};
use bevy::prelude::*;

use super::fps_camera::Hovered;
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
