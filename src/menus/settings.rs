use std::{f32::consts::PI, ops::RangeInclusive};

use bevy::{
    ecs::{component::Mutable, lifecycle::HookContext, world::DeferredWorld},
    feathers::{
        controls::{
            FeathersButton, FeathersNumberInput, NumberInputPrecision, NumberInputStep, SoftLimit,
        },
        theme::{InheritableThemeTextColor, ThemeBorderColor, ThemedText},
        tokens::{self},
    },
    picking::hover::Hovered,
    prelude::*,
    settings::SaveSettings,
    ui_widgets::Activate,
};
use ui::{space, spacing};

use crate::{
    GameSettings, GameplaySettings, SoundSettings,
    menus::{import_beatmap, reload_beatmaps},
};

// Marker
#[derive(Component, Clone, Copy, Default)]
#[require(Hovered)]
pub struct SettingsItem;

pub fn settings_number_input(label: &'static str, range: RangeInclusive<f32>) -> impl Scene {
    bsn! {
        SettingsItem
        SoftLimit(bevy::feathers::controls::NumberInputRange::F32(range))

        on(|e: On<PointerLeave>, mut commands: Commands| {
            commands.entity(e.event_target()).try_remove::<ConfirmReset>();
        })
        @FeathersNumberInput {
            @label_text: label,
        }

    }
}

macro_rules! settings_sync {
    ($res:ty => $access:ident) => {
        bsn! {
            template(|ctx| {
                let val = ctx.resource::<$res>().$access;
                Ok(::bevy::feathers::controls::NumberInputValue::F32(val as _))
            })
            on(|e: On<ReloadSettings>, ctx: Res<$res>, mut commands: Commands| {
                let val = if e.reset {
                    <$res>::default().$access
                } else {
                    ctx.$access
                };
                commands.trigger(::bevy::ui_widgets::ValueChange {source: e.event_target(), value: val as f32 , is_final: true});
            })

            on(|val: On<::bevy::ui_widgets::ValueChange<f32>>, mut commands: Commands, mut res: ResMut<$res>| {
                if val.is_final {
                    res.$access = val.value as _;
                    commands.queue(::bevy::settings::SaveSettingsDeferred::default());
                }
                commands.entity(val.event_target()).insert(::bevy::feathers::controls::NumberInputValue::F32(val.value as _));

            })
        }
    };
}

#[derive(EntityEvent, Clone, Copy)]
struct ReloadSettings {
    entity: Entity,
    reset: bool,
}

#[derive(Component, Default, Clone, Copy)]
pub struct ConfirmReset;

pub fn settings_tab() -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: spacing::SM,
        }
        ThemeBorderColor(tokens::MENU_BORDER)
        @ui::card(true)
        InheritableThemeTextColor(tokens::TEXT_MAIN)

        Children [
            @sound_settings()
            --
            @game_settings()
            --
            @gameplay_settings()
            --
            @FeathersButton
            on(reset_settings)
            on(|e: On<Insert<ConfirmReset>>, mut text: Query<(&ChildOf, &mut Text)>| {
                if let Some((_, mut text)) = text.iter_mut().find(|(c, _)| c.parent() == e.event_target()) {
                    text.replace_range(.., "Confirm?");
                }
            })
            on(|e: On<Remove<ConfirmReset>>, mut text: Query<(&ChildOf, &mut Text)>| {
                if let Some((_, mut text)) = text.iter_mut().find(|(c, _)| c.parent() == e.event_target()) {
                    text.replace_range(.., "Reset Settings");
                }
            })

            on(|e: On<PointerLeave>, mut commands: Commands| {
                commands.entity(e.event_target()).try_remove::<ConfirmReset>();
            })
            Children [
                ThemedText
                Text("Reset Settings")
            ]
            --
            ThemeBorderColor(tokens::MENU_BORDER)
            @ui::card(true)
            Node {
                flex_direction: FlexDirection::Column
            }
            Children [
                @import_beatmap()
                --
                @reload_beatmaps()
            ]
        ]
    }
}

fn settings_group(label: &'static str) -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Column,
            width: space(80.),
            row_gap: spacing::SM,
        }
        @ui::card(true)
        Children [
            ThemedText
            Text(label)
        ]
    }
}

fn sound_settings() -> impl Scene {
    bsn! {
        @settings_group("Sound")
        Children [
            @{settings_sync!(SoundSettings => music_volume)}
            @settings_number_input("Music Volume", 0.0..=1.0)
            --
            @{settings_sync!(SoundSettings => effects_volume)}
            @settings_number_input("Effects Volume", 0.0..=1.0)
        ]
    }
}

fn game_settings() -> impl Scene {
    bsn! {
        @settings_group("Game")
        Children [
            @{settings_sync!(GameSettings => mouse_sensitivity)}
            @settings_number_input("Mouse Sensitivity (in/360)", 0.0..=100.0)
            --
            NumberInputStep(400.)
            NumberInputPrecision(-2)
            @{settings_sync!(GameSettings => dpi)}
            @settings_number_input("DPI", 400.0..=8000.0)
            --
            @{settings_sync!(GameSettings => fov)}
            @settings_number_input("Vertical FOV in radians", 0.0..=PI)
        ]
    }
}

fn gameplay_settings() -> impl Scene {
    bsn! {
        @settings_group("Gameplay")
        Children [
            @{settings_sync!(GameplaySettings => target_scale)}
            @settings_number_input("Target Scale", 0.0..=2.0)
            --
            @settings_input::<GameplaySettings, f32>(|s| {s.rectangular_transform.width}, |mut s, v| {s.rectangular_transform.width = v;})
            @settings_number_input("Width", 10.0..=100.0)
            --

            @settings_input::<GameplaySettings, f32>(|s| {s.rectangular_transform.height}, |mut s, v| {s.rectangular_transform.height = v;})
            @settings_number_input("Height", 10.0..=100.0)
            --
            @settings_input::<GameplaySettings, f32>(|s| {s.rectangular_transform.distance}, |mut s, v| {s.rectangular_transform.distance = v;})
            @settings_number_input("Distance", 10.0..=100.0)
        ]
    }
}

fn settings_input<T, O>(
    get: impl Fn(&T) -> O + Send + Sync + Copy + 'static,
    set: impl Fn(ResMut<T>, O) + Send + Sync + Copy + 'static,
) -> impl Scene
where
    T: Default + Resource<Mutability = Mutable> + Clone + Sized,
    O: Into<f32> + From<f32>,
{
    bsn! {
        template(move |ctx| {
            let val = get(ctx.resource::<T>());
            Ok(::bevy::feathers::controls::NumberInputValue::F32(val.into()))
        })
        on(move |e: On<ReloadSettings>, ctx: Res<T>, mut commands: Commands| {
                let val = if e.reset {
                    get(&T::default())
                } else {
                    get(ctx.as_ref())
                };
                commands.trigger(::bevy::ui_widgets::ValueChange::<f32> {source: e.event_target(), value: val.into() , is_final: true});
        })
        on(move |val: On<::bevy::ui_widgets::ValueChange<f32>>, mut commands: Commands, res: ResMut<T>| {
            if val.is_final {
                set(res, O::from(val.value));
                commands.queue(::bevy::settings::SaveSettingsDeferred::default());
            }
            commands.entity(val.event_target()).insert(::bevy::feathers::controls::NumberInputValue::F32(val.value as _));
        })
    }
}

fn reset_settings(
    e: On<Activate>,
    mut commands: Commands,
    q: Query<Has<ConfirmReset>>,
    q_settings_item: Query<Entity, With<SettingsItem>>,
) -> Result {
    if q.get(e.event_target())? {
        for child in q_settings_item {
            commands.entity(child).trigger(|entity| ReloadSettings {
                entity,
                reset: true,
            });
        }
        commands.entity(e.event_target()).remove::<ConfirmReset>();
        commands.queue(SaveSettings::IfChanged);
    } else {
        commands.entity(e.event_target()).insert(ConfirmReset);
    }
    Ok(())
}

pub fn reset_on_keypress(
    mut commands: Commands,
    settings_items: Query<(Entity, &Hovered, Has<ConfirmReset>), With<SettingsItem>>,
    key: Res<ButtonInput<KeyCode>>,
) {
    if key.just_pressed(KeyCode::KeyR) {
        for (entity, &Hovered(hover), confirm) in settings_items {
            match (hover, confirm) {
                (true, true) => {
                    commands.entity(entity).trigger(|entity| ReloadSettings {
                        entity,
                        reset: true,
                    });
                    commands.entity(entity).remove::<ConfirmReset>();
                }
                (true, false) => {
                    commands.entity(entity).insert(ConfirmReset);
                }
                _ => (),
            }
        }
    }
}
