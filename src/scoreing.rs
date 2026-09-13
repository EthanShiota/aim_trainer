use std::time::Duration;

use bevy::{color::palettes::tailwind::BLUE_300, prelude::*, text::TextSection};

use crate::AppState;
pub struct ScoringPlugin;

#[derive(Resource, Default)]
pub struct Score {
    pub points: usize,
}

// Marker for score display
#[derive(Component, Clone, Default)]
struct ScoreDisplay;

impl Plugin for ScoringPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::InGame), setup)
            .add_systems(
                Update,
                (tick, update_score).run_if(in_state(AppState::InGame)),
            );
    }
}

#[derive(Component, Clone, Default)]
pub struct Lifetime(Timer);

impl Lifetime {
    pub fn duration(duration: Duration) -> Self {
        Self(Timer::new(duration, TimerMode::Once))
    }
}

fn tick(
    mut q_lifetime: Query<(Entity, &mut Lifetime)>,
    time: Res<Time<Virtual>>,
    mut commands: Commands,
) {
    for (ent, mut lifetime) in q_lifetime.iter_mut() {
        lifetime.0.tick(time.delta());
        if lifetime.0.is_finished() {
            commands.entity(ent).despawn();
        }
    }
}

fn setup(mut commands: Commands) {
    commands.insert_resource(Score::default());
    commands.spawn_scene(bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row
            align_self: AlignSelf::Start,
            justify_content: JustifyContent::Center,
            width: vw(100.),
        }
        Children [
            ScoreDisplay
            Text("hello")
            TextFont {
                font_size: px(44.),
            }
                TextColor(BLUE_300)

        ]
        DespawnOnExit::<AppState>(AppState::InGame)
    });
}

fn update_score(score: If<Res<Score>>, mut q_display: Query<&mut Text, With<ScoreDisplay>>) {
    if let Ok(mut text) = q_display.single_mut() {
        *text.get_text_mut() = format!("Score {}", score.points);
    }
}
