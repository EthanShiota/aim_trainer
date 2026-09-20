use std::sync::{Arc, Mutex};

use bevy::{color::palettes::tailwind::GREEN_300, prelude::*};

use crate::{AppState, target_plugin::TargetMaterial};
#[derive(Resource)]
pub struct TargetResource {
    // base target mesh
    pub mesh: Handle<Mesh>,
    pub ring_start: f32,
    pub ring_end: f32,
    pub easing: EasingCurve<f32>,
    pub color_curve: Arc<Mutex<dyn Iterator<Item = LinearRgba> + Send + Sync>>,
}

impl TargetResource {
    pub fn target_scene(&mut self) -> impl Scene {
        bsn! {
            DespawnOnExit<AppState>(AppState::InGame)
            Mesh3d({self.mesh.clone()})
            MeshMaterial3d<TargetMaterial>(asset_value(TargetMaterial {
                color: self.color_curve.lock().unwrap().next().unwrap(),
                ring: self.ring_start,
                ..default()
            }))
        }
    }
}
