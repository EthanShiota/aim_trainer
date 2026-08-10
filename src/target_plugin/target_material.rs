use bevy::{
    color::palettes::{css::BLUE_VIOLET, tailwind::GREEN_300},
    prelude::*,
    render::render_resource::AsBindGroup,
};

use crate::{fps_camera::Hovered, target_plugin::Target};

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

pub fn tick(
    q_hovered: Query<&Hovered>,
    q_mat: Query<(Entity, &MeshMaterial3d<TargetMaterial>), With<Target>>,
    mut target_mats: ResMut<Assets<TargetMaterial>>,
) {
    for (ent, mat) in q_mat.into_iter() {
        if let Some(mut mat) = target_mats.get_mut(mat.id()) {
            match q_hovered.contains(ent) {
                true => mat.color = BLUE_VIOLET.into(),
                false => mat.color = GREEN_300.into(),
            }
        }
    }
}
