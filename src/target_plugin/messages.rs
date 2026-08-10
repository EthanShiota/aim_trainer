use bevy::prelude::*;
use std::time::Duration;

#[derive(Message)]
pub struct TargetHit(pub Entity);

#[derive(Message)]
pub struct TargetHitDelta(pub (Entity, Duration));

#[derive(Message)]
pub struct FireWeapon;

#[derive(Message)]
pub struct FireWeaponHeld {
    pub delta: Duration,
}
