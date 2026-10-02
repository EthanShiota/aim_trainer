use bevy::{prelude::*, render::render_resource::AsBindGroup};

#[derive(AsBindGroup, Clone, Asset, Reflect)]
pub struct CrosshairMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
    #[uniform(1)]
    pub gap: f32,
    #[uniform(2)]
    pub dot_radius: f32,
    #[uniform(3)]
    pub hair_rotation: f32,
    #[uniform(4)]
    pub hair_length: f32,
    #[uniform(5)]
    pub hair_width: f32,
    #[uniform(6)]
    pub num_hairs: u32,
}

impl Default for CrosshairMaterial {
    fn default() -> Self {
        Self {
            num_hairs: 0,
            color: LinearRgba::WHITE,
            gap: 0.0,
            dot_radius: 10.,
            hair_rotation: 0.0,
            hair_length: 0.0,
            hair_width: 0.0,
        }
    }
}

impl UiMaterial for CrosshairMaterial {
    fn fragment_shader() -> bevy::shader::ShaderRef {
        "shaders/crosshair.wgsl".into()
    }
}
