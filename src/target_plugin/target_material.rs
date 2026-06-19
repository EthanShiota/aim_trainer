use bevy::{prelude::*, render::render_resource::AsBindGroup};

#[derive(AsBindGroup, Debug, Clone, Asset, TypePath)]
pub struct TargetMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
}

impl Material for TargetMaterial {
    fn fragment_shader() -> bevy::shader::ShaderRef {
        "shaders/target_material.wgsl".into()
    }
}
