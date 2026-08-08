use bevy::{prelude::*, render::render_resource::AsBindGroup};

#[derive(AsBindGroup, Debug, Clone, Asset, Component, Reflect, Default)]
pub struct TargetMaterial {
    #[uniform(100)]
    pub color: LinearRgba,
    #[uniform(101)]
    pub ring: f32,
    #[uniform(102)]
    pub ring_width: f32,
}

impl Material for TargetMaterial {
    fn fragment_shader() -> bevy::shader::ShaderRef {
        "shaders/target_material.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Premultiplied
    }
}
