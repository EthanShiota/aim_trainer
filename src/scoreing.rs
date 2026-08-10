use bevy::prelude::*;

use crate::AppState;
pub struct ScoringPlugin;

#[derive(Resource, Default)]
struct Score {
    points: usize,
}

impl Plugin for ScoringPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::InGame), setup);
    }
}

fn setup(mut commands: Commands) {
    commands.insert_resource(Score::default());
}
