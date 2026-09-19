use crate::GameAction;

use super::GameSettings;
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
            .add_systems(PreUpdate, handle_fire_input);
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
