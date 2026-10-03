use bevy::{animation::AnimationEvent, prelude::*};

#[derive(EntityEvent)]
pub struct TargetDestroyed(pub Entity);

#[derive(EntityEvent)]
pub struct TargetHit(pub Entity);

#[derive(EntityEvent)]
pub struct SpawnTarget(pub Entity);

#[derive(EntityEvent)]
pub struct SpawnHint(pub Entity);

/// Triggered at the start, end, and when changing direction
#[derive(AnimationEvent, Clone)]
pub struct CurveSoundEvent {
    pub entity: Entity,
    pub last: bool,
}
