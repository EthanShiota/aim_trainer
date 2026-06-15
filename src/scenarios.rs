use bevy::prelude::*;
use std::time::Duration;

use crate::target_spawner::{SpawnerVolume, TargetSpawner};

pub fn basic() -> impl Scene {
    bsn! {
        TargetSpawner::new(Duration::from_secs_f64(0.5), Some(10))
    }
}
