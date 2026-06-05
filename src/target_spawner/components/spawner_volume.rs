use bevy::prelude::*;

use super::SpawnerVolume;

impl From<Sphere> for SpawnerVolume {
    fn from(value: Sphere) -> Self {
        Self::Sphere(value)
    }
}

impl From<Cuboid> for SpawnerVolume {
    fn from(value: Cuboid) -> Self {
        Self::Cuboid(value)
    }
}

impl SpawnerVolume {
    pub fn sample_interior<R: rand::RngExt + ?Sized>(&self, rng: &mut R) -> Vec3 {
        match self {
            SpawnerVolume::Sphere(sphere) => sphere.sample_interior(rng),
            SpawnerVolume::Cuboid(cuboid) => cuboid.sample_interior(rng),
            SpawnerVolume::Torus(torus) => {
                todo!();
            }
        }
    }
    pub fn sample_boundary<R: rand::RngExt + ?Sized>(&self, rng: &mut R) -> Vec3 {
        match self {
            SpawnerVolume::Sphere(sphere) => sphere.sample_boundary(rng),
            SpawnerVolume::Cuboid(cuboid) => cuboid.sample_boundary(rng),
            SpawnerVolume::Torus(torus) => {
                todo!();
            }
        }
    }
}
