use bevy::prelude::*;

#[derive(EntityEvent)]
pub struct TargetDestroyed(pub Entity);

#[derive(EntityEvent)]
pub struct TargetHit(pub Entity);

#[derive(EntityEvent)]
pub struct SpawnTarget(pub Entity);

#[derive(EntityEvent)]
pub struct SpawnHint(pub Entity);
