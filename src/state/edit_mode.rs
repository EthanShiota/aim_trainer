use super::AppState;
use bevy::prelude::*;
#[derive(SubStates, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Debug)]
#[source(AppState = AppState::InGame)]
pub enum EditMode {
    Editing,
    #[default]
    Normal,
}
