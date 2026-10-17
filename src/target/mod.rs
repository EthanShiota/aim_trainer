use super::Hovered;
use std::{
    f32::consts::PI,
    sync::{Arc, Mutex},
};

use bevy::{
    camera::visibility::RenderLayers, input::common_conditions::input_just_pressed, prelude::*,
};
mod components;
pub mod events;
pub mod messages;
mod target_material;
pub use components::*;
pub use target_material::TargetMaterial;

#[derive(Resource, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct DebugMode(pub bool);

mod target_resource;
pub use target_resource::TargetResource;

use crate::{AppState, input::InputMessage};

pub struct TargetPlugin;

#[derive(States, Debug, Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Default)]
#[allow(unused)]
pub enum SpawnerState {
    #[default]
    Active,
    Inactive,
}

#[derive(SystemSet, Hash, Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub struct TargetSchedule;

impl Plugin for TargetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_plugin)
            .insert_resource(DebugMode(false))
            .init_state::<SpawnerState>()
            // INFO: Target Material
            .add_plugins(MaterialPlugin::<TargetMaterial>::default())
            .add_plugins(MarkerPlugin)
            .add_plugins(TimingRingPlugin)
            // Order matters because cull_active removes if the input is not held which is true when
            // active is inserted
            .add_systems(Update, (cull_active, add_effect, sync_effect).chain())
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
            .add_message::<messages::TargetHitDelta>()
            .add_message::<messages::FireWeapon>()
            .add_message::<messages::FireWeaponHeld>()
            // INFO: Handle target hit
            .add_systems(
                Update,
                (components::target::tick, target_material::tick)
                    .in_set(TargetSchedule)
                    .run_if(in_state(AppState::InGame)),
            )
            // INFO: Spawner Debug
            .add_systems(
                Update,
                (draw_spawners).run_if(resource_equals(DebugMode(true))),
            )
            .add_systems(Update, spawner_loop.run_if(in_state(SpawnerState::Active)))
            // INFO: Target Events
            .add_observer(on_target_destroyed)
            .add_observer(on_target_hit)
            .world_mut()
            .register_component_hooks::<TargetSpawner>()
            .on_add(|mut world, context| {
                let ent = world.entity(context.entity);

                // Takes mesh attached to spawner and processes into a spawner volume
                if let Ok(mesh) = ent.get_components::<&Mesh3d>() {
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

fn setup_plugin(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let color_curve =
        Arc::new(Mutex::new((0..).map(|i| {
            Oklcha::new(0.7507, 0.1372, (i as f32 * 22.5) % 360., 1.).into()
        })));
    commands.insert_resource(TargetResource {
        mesh: meshes.add(Sphere::new(2.)),
        ring_start: 0.,
        ring_end: PI / 2.,
        easing: EasingCurve::new(0., 1., EaseFunction::Linear),
        color_curve,
    });
}

#[derive(Component)]
pub struct Active;

fn add_effect(
    hovered: Query<
        (
            Entity,
            &Marker,
            &MeshMaterial3d<TargetMaterial>,
            &mut Transform,
        ),
        With<Hovered>,
    >,
    q_curve: Query<(), With<CurveMarker>>,
    mut commands: Commands,
    mut reader: MessageReader<InputMessage>,
) {
    let mut hovered = hovered.into_iter().sort_by_key::<&Marker, _>(|m| m.time());

    for msg in reader.read() {
        let Some(mut target) = hovered.next() else {
            continue;
        };
        if let InputMessage::FireWeapon = msg
            && q_curve.contains(target.0)
        {
            target.3.scale = Vec3::splat(1.5);

            commands.entity(target.0).insert(Active);
        }
    }
}

// Sync material properties to active status
fn sync_effect(
    q_active: Query<(&MeshMaterial3d<TargetMaterial>, Entity), With<Active>>,
    q_inactive: Query<(&MeshMaterial3d<TargetMaterial>, &mut Transform, Entity), Without<Active>>,
    mut target_material: ResMut<Assets<TargetMaterial>>,
) {
    for active in q_active.into_iter() {
        if let Some(mut material) = target_material.get_mut(active.0.id()) {
            material.hovered = 1;
        }
    }

    for mut inactive in q_inactive.into_iter() {
        if let Some(mut material) = target_material.get_mut(inactive.0.id()) {
            material.hovered = 0;
        }
        inactive.1.scale = Vec3::splat(1.);
    }
}

fn cull_active(
    mut reader: MessageReader<InputMessage>,
    mut commands: Commands,
    q_active: Query<Entity, With<Active>>,
) {
    let pressed = reader
        .read()
        .any(|msg| matches!(msg, InputMessage::FireWeaponHeld));

    // Remove active from all active
    if !pressed {
        for active in q_active.into_iter() {
            commands.entity(active).remove::<Active>();
        }
    }
}
