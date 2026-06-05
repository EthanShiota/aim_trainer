use bevy::prelude::*;

use super::target_material::TargetMaterial;
pub mod spawner;
pub mod spawner_volume;
mod target;
pub use target::*;

#[derive(Resource)]
pub struct TargetResource {
    pub mesh: Handle<Mesh>,
    pub material: Handle<TargetMaterial>,
}

#[derive(Component, Deref, DerefMut)]
#[require(Visibility)]
pub struct TargetSpawner(pub Timer);

#[derive(Component)]
#[require(SpawnerVolumeMode)]
pub enum SpawnerVolume {
    Sphere(bevy::math::primitives::Sphere),
    Cuboid(bevy::math::primitives::Cuboid),
    Torus(bevy::math::primitives::Torus),
}

#[derive(Component, Default)]
pub enum SpawnerVolumeMode {
    #[default]
    SampleInterior,
    SampleBoundary,
}

#[derive(Resource, PartialEq, Eq, PartialOrd, Ord)]
pub struct DebugMode(pub bool);
