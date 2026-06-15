use bevy::prelude::*;

use super::target_material::TargetMaterial;
mod spawner;
mod spawner_volume;
mod target;
pub use spawner::*;
pub use spawner_volume::*;
pub use target::*;

#[derive(Resource)]
pub struct TargetResource {
    pub mesh: Handle<Mesh>,
    pub material: Handle<TargetMaterial>,
}

#[derive(Resource, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct DebugMode(pub bool);
