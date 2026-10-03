#![allow(dead_code)]
use bevy::{math::sampling::UniformMeshSampler, prelude::*};
use rand::distr::Distribution;

#[derive(Component, Clone, Reflect, FromTemplate)]
#[require(SpawnerVolumeMode)]
#[reflect(Component)]
#[type_path = "api"]
// #[component(on_add = on_add_register_mesh)]
pub enum SpawnerVolume {
    #[default]
    Sphere(bevy::math::primitives::Sphere),
    Cuboid(bevy::math::primitives::Cuboid),
    Torus(bevy::math::primitives::Torus),
    Mesh(Vec<Triangle3d>),
}

impl Default for SpawnerVolume {
    fn default() -> Self {
        Self::Sphere(Sphere::new(1.))
    }
}

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

impl From<Mesh3d> for SpawnerVolume {
    fn from(_value: Mesh3d) -> Self {
        log::info!("hit form handle<mesh>");
        todo!();
    }
}

fn sample_mesh<R: rand::RngExt + ?Sized>(mesh: Vec<Triangle3d>, rng: &mut R) -> Vec3 {
    UniformMeshSampler::try_new(mesh.clone())
        .unwrap()
        .sample(rng)
}

impl SpawnerVolume {
    pub fn sample_interior<R: rand::RngExt + ?Sized>(&self, rng: &mut R) -> Vec3 {
        match self {
            SpawnerVolume::Sphere(sphere) => sphere.sample_interior(rng),
            SpawnerVolume::Cuboid(cuboid) => cuboid.sample_interior(rng),
            SpawnerVolume::Torus(_torus) => {
                todo!();
            }
            SpawnerVolume::Mesh(mesh) => sample_mesh(mesh.clone(), rng),
        }
    }
    pub fn sample_boundary<R: rand::RngExt + ?Sized>(&self, rng: &mut R) -> Vec3 {
        match self {
            SpawnerVolume::Sphere(sphere) => sphere.sample_boundary(rng),
            SpawnerVolume::Cuboid(cuboid) => cuboid.sample_boundary(rng),
            SpawnerVolume::Torus(_torus) => {
                todo!();
            }
            SpawnerVolume::Mesh(mesh) => sample_mesh(mesh.clone(), rng),
        }
    }
}

#[derive(Component, Default, Clone)]
pub enum SpawnerVolumeMode {
    #[default]
    SampleInterior,
    SampleBoundary,
}
