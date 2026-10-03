use crate::GameAction;
use crate::target::Target;
use fps_camera::FPSCamera;

use super::GameSettings;
use bevy::app::AnimationSystems;
use bevy::picking::mesh_picking::ray_cast::RayCastVisibility::Visible;
use bevy::prelude::*;

#[derive(Message)]
pub enum InputMessage {
    FireWeapon,
    FireWeaponHeld,
}

pub struct GameInputPlugin;
impl Plugin for GameInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<InputMessage>()
            .add_systems(PreUpdate, handle_fire_input)
            .add_systems(PreUpdate, update_raycast)
            .add_systems(PostUpdate, remove_hovered.after(AnimationSystems));
    }
}

fn handle_fire_input(
    mut event_writer: MessageWriter<InputMessage>,
    game_settings: Res<GameSettings>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    let fire_button = game_settings.keybinds.get(&GameAction::FireWeapon).unwrap();
    let fire_button_mouse = game_settings
        .mousebinds
        .get(&GameAction::FireWeapon)
        .unwrap();
    if mouse.any_just_pressed(fire_button_mouse.iter().copied())
        || keyboard.any_just_pressed(fire_button.iter().copied())
    {
        event_writer.write(InputMessage::FireWeapon);
    } else if mouse.any_pressed(fire_button_mouse.iter().copied())
        || keyboard.any_pressed(fire_button.iter().copied())
    {
        event_writer.write(InputMessage::FireWeaponHeld);
    }
}

#[derive(Component, Deref)]
pub struct Hovered(pub usize);
fn update_raycast(
    mut ray_cast: MeshRayCast,
    q_cam: Query<&Transform, With<FPSCamera>>,
    q_target: Query<&Target>,
    mut commands: Commands,
) {
    for cam in q_cam.iter() {
        // Cast hovered ray from fps camera look direction
        let ray = Ray3d::new(cam.translation, cam.forward());

        // Filter out non target meshes
        let filter = |entity| q_target.contains(entity);
        let settings = MeshRayCastSettings::default()
            .with_filter(&filter)
            .with_visibility(Visible);

        let results = ray_cast.cast_ray(ray, &settings);

        for (idx, (entity, _hit)) in results.iter().enumerate() {
            // Hovered idx is likely non-deterministic as targets can be at identical distances from the camera
            // or it is incorrect way of ordering hit priority
            commands.entity(*entity).insert(Hovered(idx));
        }
    }
}

fn remove_hovered(mut commands: Commands, q_hovered: Query<Entity, With<Hovered>>) {
    for hovered in q_hovered.into_iter() {
        commands.entity(hovered).remove::<Hovered>();
    }
}
