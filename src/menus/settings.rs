use std::{f32::consts::PI, ops::RangeInclusive};

use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    feathers::{
        controls::{
            FeathersButton, FeathersNumberInput, NumberInputPrecision, NumberInputStep, SoftLimit,
        },
        theme::{InheritableThemeTextColor, ThemeBorderColor, ThemedText},
        tokens::{self},
    },
    input_focus::InputFocus,
    prelude::*,
    settings::SaveSettings,
    text::EditableText,
    ui_widgets::Activate,
};
use ui::{space, spacing};

use crate::{GameSettings, GameplaySettings, SoundSettings};

// Marker
#[derive(Component, Clone, Copy, Default)]
struct SettingsItem;

pub fn settings_number_input(label: &'static str, range: RangeInclusive<f32>) -> impl Scene {
    bsn! {
        SettingsItem
        SoftLimit(bevy::feathers::controls::NumberInputRange::F32(range))

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
                commands.entity(e.event_target()).insert(::bevy::feathers::controls::NumberInputValue::F32(val as _));
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
#[component(on_insert)]
#[component(on_remove)]
struct ConfirmReset;

impl ConfirmReset {
    fn on_insert(mut world: DeferredWorld, ctx: HookContext) {
        let children = world.entity(ctx.entity).get_components::<&Children>();
        if let Ok(children) = children {
            let children = children.collection().clone();
            for text in children.into_iter() {
                if let Some(mut text) = world.get_mut::<Text>(text) {
                    text.replace_range(.., "Confirm?");
                }
            }
        }
    }
    fn on_remove(mut world: DeferredWorld, ctx: HookContext) {
        let children = world.entity(ctx.entity).get_components::<&Children>();
        if let Ok(children) = children {
            let children = children.collection().clone();
            for text in children.into_iter() {
                if let Some(mut text) = world.get_mut::<Text>(text) {
                    text.replace_range(.., "Reset Settings");
                }
            }
        }
    }
}
pub fn settings_item_submit(mut focus: ResMut<InputFocus>, settings_item: Query<&EditableText>) {
    if let Some(focus_ent) = focus.get()
        && settings_item.contains(focus_ent)
    {
        focus.clear();
    }
}

pub fn settings_tab() -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Column,
            width: space(80.),
            row_gap: spacing::SM,
        }
        ThemeBorderColor(tokens::MENU_BORDER)
        @ui::card(true)
        InheritableThemeTextColor(tokens::TEXT_MAIN)

        Children [
            @{settings_sync!(SoundSettings => music_volume)}
            @settings_number_input("Music Volume", 0.0..=1.0)
            --
            @{settings_sync!(SoundSettings => effects_volume)}
            @settings_number_input("Effects Volume", 0.0..=1.0)
            --
            @{settings_sync!(GameSettings => mouse_sensitivity)}
            @settings_number_input("Mouse Sensitivity (in/360)", 0.0..=100.0)
            --
            @{settings_sync!(GameSettings => dpi)}
            NumberInputStep(400.)
            NumberInputPrecision(0)
            @settings_number_input("DPI", 400.0..=8000.0)
            --
            @{settings_sync!(GameSettings => fov)}
            @settings_number_input("Vertical FOV in radians", 0.0..=PI)
            --
            @{settings_sync!(GameplaySettings => target_scale)}
            @settings_number_input("Target Scale", 0.0..=2.0)
            --
            @FeathersButton
            on(|e: On<Activate>,
                mut commands: Commands,
                q: Query<Has<ConfirmReset>>,
                q_parent: Query<&ChildOf>,
                q_children: Query<&Children>,
                q_settings_item: Query<(),
                With<SettingsItem>>| -> Result {
                if q.get(e.event_target())? {

                    commands.insert_resource(GameSettings::default());
                    commands.insert_resource(SoundSettings::default());
                    commands.insert_resource(GameplaySettings::default());
                    commands.queue(SaveSettings::IfChanged);

                    // trigger reload for all children of settings tab
                    let p = q_parent.get(e.event_target())?.parent();
                    let children = q_children.get(p)?;
                    for child in children {
                        if q_settings_item.contains(*child) {
                            commands.entity(*child).trigger(|entity| ReloadSettings{entity, reset: false});
                        }
                    }
                    commands.entity(e.event_target()).remove::<ConfirmReset>();

                } else {
                    commands.entity(e.event_target()).insert(ConfirmReset);
                }
                Ok(())

            })
            on(|e: On<PointerLeave>, mut commands: Commands| {
                commands.entity(e.event_target()).try_remove::<ConfirmReset>();
            })
            Children [
                ThemedText
                Text("Reset Settings")
            ]
        ]
    }
}
