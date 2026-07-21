use bevy::{
    camera::visibility::RenderLayers, input::common_conditions::input_just_pressed, prelude::*,
};
mod components;
mod target_material;
pub use components::*;
use target_material::TargetMaterial;

#[derive(Resource, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct DebugMode(pub bool);

#[derive(Resource)]
pub struct TargetResource {
    pub mesh: Handle<Mesh>,
    pub material: Handle<TargetMaterial>,
}

use crate::{AppState, GameState};

pub struct TargetPlugin;

#[derive(States, Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Default)]
pub enum SpawnerState {
    #[default]
    Active,
    Inactive,
}

impl Plugin for TargetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_plugin)
            .insert_resource(DebugMode(false))
            .init_state::<SpawnerState>()
            // INFO: Target Material
            .add_plugins(MaterialPlugin::<TargetMaterial>::default())
            .add_plugins(CurvePlugin)
            // INFO: Gizmo
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
            // INFO: Messages
            .add_message::<TargetHit>()
            .add_message::<FireWeapon>()
            // INFO: Handle target hit
            .add_systems(
                Update,
                (handle_fire_weapon, destroy_hit_targets).run_if(in_state(AppState::InGame)),
            )
            // INFO: Spawner Debug
            .add_systems(
                Update,
                (draw_spawners).run_if(resource_equals(DebugMode(true))),
            )
            .add_systems(
                Update,
                tick_target_marker.run_if(in_state(GameState::Playing)),
            )
            .add_systems(Update, spawner_loop.run_if(in_state(SpawnerState::Active)))
            .world_mut()
            .register_component_hooks::<TargetSpawner>()
            .on_add(|mut world, context| {
                let ent = world.entity(context.entity);

                if let Ok(mesh) = ent.get_components::<&Mesh3d>() {
                    log::info!("mesh!");
                    let triangles = world
                        .get_resource::<Assets<Mesh>>()
                        .and_then(|a| a.get(mesh.0.id()))
                        .and_then(|m| m.triangles().ok())
                        .unwrap()
                        .collect();
                    let entity = ent.entity();
                    world
                        .commands()
                        .entity(entity)
                        .insert(SpawnerVolume::Mesh(triangles));
                    world.commands().entity(entity).remove::<Mesh3d>();
                }
            });
    }
}

fn setup_plugin(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<TargetMaterial>>,
) {
    commands.insert_resource(TargetResource {
        mesh: meshes.add(Sphere::new(2.)),
        material: materials.add(TargetMaterial {
            color: LinearRgba::RED * 10.,
        }),
    });
}
