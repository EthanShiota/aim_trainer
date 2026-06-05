use bevy::{
    camera::visibility::RenderLayers, input::common_conditions::input_just_pressed, prelude::*,
};
mod components;
mod target_material;
use components::spawner::*;
pub use components::{
    DebugMode, FireWeapon, SpawnerVolume, SpawnerVolumeMode, Target, TargetDestroyed, TargetHit,
    TargetResource, TargetSpawner,
};
use target_material::TargetMaterial;

use components::{destroy_hit_targets, handle_fire_weapon, move_targets, spawner::SpawnerGizmo};

pub struct TargetPlugin;

impl Plugin for TargetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_plugin)
            .insert_resource(DebugMode(true))
            .add_plugins(MaterialPlugin::<TargetMaterial>::default())
            .insert_gizmo_config(
                SpawnerGizmo,
                GizmoConfig {
                    render_layers: RenderLayers::layer(1),
                    ..default()
                },
            )
            .add_systems(
                Update,
                debug_spawn_target.run_if(
                    input_just_pressed(MouseButton::Right)
                        .and_then(resource_equals(DebugMode(true))),
                ),
            )
            .add_message::<TargetHit>()
            .add_message::<FireWeapon>()
            .add_systems(Update, (handle_fire_weapon, destroy_hit_targets))
            .add_systems(
                Update,
                (draw_spawners).run_if(resource_equals(DebugMode(true))),
            )
            .add_systems(Update, (spawner_loop, move_targets));
    }
}

fn setup_plugin(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<TargetMaterial>>,
) {
    commands.insert_resource(TargetResource {
        mesh: meshes.add(Sphere::new(3.)),
        material: materials.add(TargetMaterial {
            color: LinearRgba::RED * 10.,
        }),
    });
}
