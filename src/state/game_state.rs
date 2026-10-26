use super::app_state::AppState;
use bevy::prelude::*;
#[derive(SubStates, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Debug)]
#[source(AppState = AppState::InGame)]
pub enum GameState {
    Paused,
    #[default]
    Playing,
}
