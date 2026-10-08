use std::time::Duration;

use bevy::{
    color::palettes::tailwind::BLUE_300, feathers::theme::ThemeBackgroundColor, prelude::*,
    text::TextSection,
};
use parser::BeatMapOsu;
use tracing::instrument;

use crate::{AppState, target::events::TargetHit};
pub struct ScoringPlugin;

#[derive(Resource)]
pub struct Score {
    pub points: f64,
    pub overall_difficulty: f32,
    pub difficulty_multiplier: f32,
    pub hit_counts: HitCounts,
    pub combo: usize,
    pub metadata: Box<BeatMapOsu>,
}

#[derive(Default)]
pub struct HitCounts {
    n300: u32,
    n100: u32,
    n50: u32,
    miss: u32,
}

impl HitCounts {
    pub fn accuracy(&self) -> f64 {
        if self.n300 + self.n100 + self.n50 + self.miss == 0 {
            return 1.;
        }
        (300 * self.n300 + 100 * self.n100 + 50 * self.n50) as f64
            / (300 * (self.n300 + self.n100 + self.n50 + self.miss)) as f64
    }
}

impl Score {
    pub fn new(
        points: f64,
        overall_difficulty: f32,
        difficulty_multiplier: f32,
        metadata: BeatMapOsu,
    ) -> Self {
        Self {
            points,
            overall_difficulty,
            difficulty_multiplier,
            combo: 0,
            hit_counts: default(),
            metadata: Box::new(metadata),
        }
    }

    #[instrument(skip(self))]
    pub fn score_hit(&mut self, value: usize) -> f32 {
        match value {
            0 => self.hit_counts.miss += 1,
            50 => self.hit_counts.n50 += 1,
            100 => self.hit_counts.n100 += 1,
            300 => self.hit_counts.n300 += 1,
            _ => (),
        }

        if value == 0 {
            self.combo = 0;
        }
        debug!(value);
        value as f32
            * (1.
                + (
                    self.combo.saturating_sub(2) as f32 * self.difficulty_multiplier
                    // * mod_multiplier
                    // / 25.
                ))
    }

    #[instrument(skip_all)]
    pub fn score_lifetime(&mut self, lifetime: &Lifetime) -> f32 {
        // Score = Hit value * (1 + (Combo multiplier * Difficulty multiplier * Mod multiplier / 25))
        // TODO: Mod Multiplier
        let mod_multiplier = 1.;

        self.combo += 1;
        debug!("combo: {}", self.combo);
        let hit_value = lifetime.judge_hit(self.overall_difficulty);
        self.score_hit(hit_value)
    }
}

// Marker for score display
#[derive(Component, Clone, Default)]
struct ScoreDisplay;

impl Plugin for ScoringPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::InGame), setup)
            .add_systems(Update, (update_score).run_if(in_state(AppState::InGame)))
            .add_systems(Update, tick.run_if(in_state(AppState::InGame)));
    }
}

#[derive(Component, Clone, Default)]
pub struct SliderScorer {
    pub hits: u32,
    pub total: u32,
}

impl SliderScorer {
    pub fn score(&mut self) -> usize {
        let span = debug_span!("score");
        let _guard = span.enter();
        let proportion = self.hits as f64 / self.total as f64;
        debug!(proportion = proportion);
        match proportion {
            // GREAT 	100%
            prop if prop >= 1. => 300,
            // OK 	50%
            prop if prop >= 0.5 => 100,
            // MEH 	At least one slider part
            prop if prop > 0. => 50,
            // MISS 	0%
            _ => 0,
        }
    }
}

#[derive(Component, Clone, Default)]
pub struct Lifetime(Timer);

#[derive(EntityEvent, Clone, Copy)]
pub struct LifetimeEvent(pub Entity);

impl Lifetime {
    pub fn duration(duration: Duration) -> Self {
        Self(Timer::new(duration, TimerMode::Once))
    }

    pub fn remaining(&self) -> Duration {
        self.0.remaining()
    }

    #[instrument(skip(self))]
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
        let hit_error = self.remaining().as_millis() as i32 - 400;
        debug!("hit error: {}", hit_error);
        hit_error
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
            // TODO: Calculate score
            commands.entity(ent).trigger(LifetimeEvent);
        }
    }
}

fn setup(mut commands: Commands) {
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
        *text.get_text_mut() = format!(
            "Score {} -> Accuracy {}",
            score.points,
            score.hit_counts.accuracy()
        );
    }
}
