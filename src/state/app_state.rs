use bevy::prelude::*;
#[derive(Debug, Hash, Resource, Eq, PartialEq, PartialOrd, States, Clone, Default)]
pub enum AppState {
    InGame,
    #[default]
    Menu,
    UiDebug,
    ScoreSummery,
}
