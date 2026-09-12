use crate::target_plugin::{Marker, TargetMaterial};
use bevy::prelude::*;

use super::fps_camera::Hovered;
pub struct EffectPlugin;

#[derive(SystemSet, Hash, Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub struct EffectSchedule;

impl Plugin for EffectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            RunFixedMainLoop,
            add_effect
                .in_set(EffectSchedule)
                .before(RunFixedMainLoopSystems::FixedMainLoop),
        )
        .add_systems(Update, sync_effect);
    }
}

#[derive(Component)]
struct Active;

fn add_effect(
    hovered: Query<(Entity, &Marker, &MeshMaterial3d<TargetMaterial>), With<Hovered>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut commands: Commands,
) {
    if mouse.just_pressed(MouseButton::Left) {
        if let Some(target) = hovered
            .into_iter()
            .sort_by_key::<&Marker, _>(|m| m.time())
            .next()
        {
            commands.entity(target.0).insert(Active);
        }
    }
}

fn sync_effect(
    active: Query<(&MeshMaterial3d<TargetMaterial>, Entity), (With<Active>, With<Hovered>)>,
    mut target_material: ResMut<Assets<TargetMaterial>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut commands: Commands,
) {
    let active_materials: std::collections::HashMap<AssetId<TargetMaterial>, Entity> =
        active.into_iter().map(|(mat, e)| (mat.id(), e)).collect();
    let pressed = mouse.pressed(MouseButton::Left);
    for (id, material) in target_material.iter_mut() {
        if pressed && active_materials.contains_key(&id) {
            material.hovered = 1;
        } else {
            material.hovered = 0;
            active_materials.get(&id).map(|entity| {
                commands.entity(*entity).remove::<Active>();
            });
        }
    }
}
