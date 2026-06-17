use bevy::{
    asset::io::embedded::GetAssetServer, camera::visibility::RenderLayers,
    input::common_conditions::input_just_pressed, prelude::*,
};
use bevy_egui::prelude::*;
mod components;
mod target_material;
pub use components::*;
use target_material::TargetMaterial;

use crate::{AppState, AudioBuffer, EditMode, GameState, SceneTimer};

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
            .add_systems(Update, edit_mode.run_if(in_state(EditMode::Editing)))
            .add_systems(
                EguiPrimaryContextPass,
                edit_mode_ui.run_if(in_state(EditMode::Editing)),
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

fn tick_target_marker(
    mut commands: Commands,
    q_markers: Query<(&TargetMarker, &Transform)>,
    mut scene_timer: ResMut<SceneTimer>,
    time: Res<Time<Real>>,
    target: Res<TargetResource>,
) {
    scene_timer.tick(time.delta());

    for (marker, transform) in q_markers.iter() {
        if **marker <= scene_timer.elapsed() {
            // spawn target
            commands.spawn((
                Mesh3d(target.mesh.clone()),
                MeshMaterial3d(target.material.clone()),
                transform.clone(),
                Target,
            ));
        }
    }
}

fn edit_mode() {}
fn edit_mode_ui(
    mut contexts: EguiContexts,
    q_audio: Query<&AudioSink, With<AudioPlayer<AudioBuffer>>>,
    mut stopwatch: ResMut<SceneTimer>,
) -> Result {
    egui::Window::new("edit").show(contexts.ctx_mut()?, |ui| {
        if ui.button("Play/Pause").clicked() {
            if let Ok(sink) = q_audio.single() {
                sink.toggle_playback();
            }
            if stopwatch.is_paused() {
                stopwatch.unpause();
            } else {
                stopwatch.pause();
            }
        }
        if let Ok(sink) = q_audio.single() {
            ui.label(format!(
                "sink: {:?}\nstopwatch: {:?}",
                sink.position(),
                stopwatch.elapsed()
            ));
        }
    });
    Ok(())
}

fn setup_plugin(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<TargetMaterial>>,
) {
    commands.insert_resource(TargetResource {
        mesh: meshes.add(Sphere::new(1.)),
        material: materials.add(TargetMaterial {
            color: LinearRgba::RED * 10.,
        }),
    });
}
