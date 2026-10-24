use std::sync::{Arc, Mutex};

use bevy::prelude::*;

use crate::{AppState, target::TargetMaterial};
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
    pub fn target_scene(&mut self, color: LinearRgba) -> impl Scene {
        bsn! {
            DespawnOnExit<AppState>(AppState::InGame)
            Mesh3d({self.mesh.clone()})
            MeshMaterial3d<TargetMaterial>(asset_value(TargetMaterial {
                ring: self.ring_start,
                color,
                ..default()
            }))
        }
    }

    // Get a cycling color
    pub fn color(&mut self) -> LinearRgba {
        self.color_curve.lock().unwrap().next().unwrap()
    }
}
