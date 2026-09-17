use std::time::Duration;

use bevy::{color::palettes::tailwind::BLUE_300, prelude::*, text::TextSection};

use crate::{AppState, target_plugin::Marker};
pub struct ScoringPlugin;

#[derive(Resource, Default)]
pub struct Score {
    pub points: usize,
    pub overall_difficulty: f32,
}

// Marker for score display
#[derive(Component, Clone, Default)]
struct ScoreDisplay;

impl Plugin for ScoringPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::InGame), setup)
            .add_systems(Update, (update_score).run_if(in_state(AppState::InGame)))
            .add_systems(FixedUpdate, tick.run_if(in_state(AppState::InGame)));
    }
}

#[derive(Component, Clone, Default)]
pub struct Lifetime(Timer);

impl Lifetime {
    pub fn duration(duration: Duration) -> Self {
        Self(Timer::new(duration, TimerMode::Once))
    }

    pub fn remaining(&self) -> Duration {
        self.0.remaining()
    }

    pub fn judge_hit(&self, overall_difficulty: f32) -> usize {
        let od = overall_difficulty;
        match self.hit_error() as f32 {
            e if e < (80. - 6. * od) => 300,
            e if e < (140. - 8. * od) => 100,
            e if e < (200. - 10. * od) => 50,
            _ => 0,
        }
    }

    pub fn hit_error(&self) -> i32 {
        // 400ms is max error before marker will despawn
        let hit_error = (self.remaining().as_millis() as i32 - 400) as i32;
        debug!("hit error: {}", hit_error);
        hit_error
    }
}

fn tick(
    mut q_lifetime: Query<(Entity, &mut Lifetime)>,
    time: Res<Time<Fixed>>,
    mut commands: Commands,
) {
    for (ent, mut lifetime) in q_lifetime.iter_mut() {
        lifetime.0.tick(time.delta());
        if lifetime.0.is_finished() {
            // TODO: Calculate score
            commands.entity(ent).try_despawn();
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
            justify_content: JustifyContent::Start,
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
