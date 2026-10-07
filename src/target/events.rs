use bevy::{animation::AnimationEvent, prelude::*};

#[derive(EntityEvent)]
pub struct TargetDestroyed(pub Entity);

#[derive(EntityEvent)]
pub struct TargetHit(pub Entity);

#[derive(EntityEvent)]
pub struct SpawnTarget(pub Entity);

#[derive(EntityEvent)]
pub struct SpawnHint(pub Entity);

#[derive(AnimationEvent, Clone)]
pub struct SliderTick(pub Entity);

#[derive(AnimationEvent, Clone)]
pub struct SliderHead(pub Entity);

#[derive(AnimationEvent, Clone)]
pub struct SliderTail(pub Entity);

#[derive(AnimationEvent, Clone)]
pub struct SliderRepeat(pub Entity);

#[derive(EntityEvent)]
pub struct RemoveCurve(pub Entity);
