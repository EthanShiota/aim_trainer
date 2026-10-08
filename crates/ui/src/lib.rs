use bevy::{
    color::palettes::tailwind,
    feathers::{
        FeathersCorePlugin,
        theme::{ThemeProps, ThemeToken, UiTheme},
        tokens,
    },
    input::common_conditions::input_just_pressed,
    input_focus::InputFocus,
    platform::collections::HashMap,
    prelude::*,
    text::EditableText,
};

pub struct UiPlugin;
impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            settings_item_submit.run_if(input_just_pressed(KeyCode::Enter)),
        )
        .add_plugins(FeathersCorePlugin);
    }
}

pub fn spawn_widget_gallery(mut commands: Commands) {
    commands.spawn_scene(bsn! {
        Node {
            width: percent(100.),
            height: percent(100.),
            display: Display::Flex
        }
        BackgroundColor(tailwind::GRAY_900)
        Children [
            ValueSlider(32.)
        ]
    });
}

fn settings_item_submit(mut focus: ResMut<InputFocus>, settings_item: Query<&EditableText>) {
    if let Some(focus_ent) = focus.get()
        && settings_item.contains(focus_ent)
    {
        focus.clear();
    }
}

mod components;
pub use components::*;
