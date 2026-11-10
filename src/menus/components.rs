use bevy::{
    color::palettes::tailwind,
    ecs::{
        lifecycle::HookContext,
        system::IntoObserverSystem,
        template::{FnTemplate, TemplateContext},
        world::DeferredWorld,
    },
    feathers::{
        controls::{FeathersNumberInput, NumberInputValue},
        dark_theme::create_dark_theme,
        theme::{
            InheritableThemeTextColor, ThemeBackgroundColor, ThemeBorderColor, ThemeTextColor,
            ThemedText, UiTheme,
        },
        tokens::{self},
    },
    input_focus::tab_navigation::TabIndex,
    prelude::*,
    text::{EditableText, TextCursorStyle},
    ui_widgets::TextInput,
};

use bevy::settings::SaveSettingsDeferred;

use bevy::text::TextEditChange;
use ui::{space, spacing};

use crate::{Crosshair, crosshair::CrosshairMaterial};

pub fn theme() -> UiTheme {
    let mut base = create_dark_theme();
    base.semantic_base.insert(
        tokens::semantic::SURFACE_WINDOW,
        Color::Srgba(tailwind::SLATE_800),
    );

    base.semantic_base.insert(
        tokens::semantic::TEXT_DEFAULT,
        Color::Srgba(tailwind::GREEN_200),
    );
    base.semantic_base.insert(
        tokens::semantic::TEXT_ON_ACCENT,
        Color::Srgba(tailwind::GREEN_300),
    );

    base.semantic_base.insert(
        tokens::semantic::SURFACE_MENU,
        Color::Srgba(tailwind::SLATE_800),
    );

    UiTheme(base)
}

// settings_item_crosshair!("label", u32 => accesser)
macro_rules! settings_item_crosshair {
    ($label:expr, $parse_type:ty => $accesser:ident) => {
        settings_item_crosshair(
            $label,
            template(|ctx| {
                let crosshair_handle = ctx.resource::<Crosshair>().0.id();
                let crosshair_assets = ctx.resource_mut::<Assets<CrosshairMaterial>>();
                Ok(::bevy::feathers::controls::NumberInputValue::F32(
                    crosshair_assets
                        .get(crosshair_handle)
                        .map(|mat| mat.$accesser)
                        .unwrap_or_default() as _,
                ))
            }),
            |e: On<::bevy::ui_widgets::ValueChange<f32>>,
             mut mat: ResMut<Assets<CrosshairMaterial>>,
             mut commands: Commands,
             mut settings: ResMut<crate::CrosshairSettings>| {
                let value = e.value;
                for (_, m) in mat.iter_mut() {
                    m.$accesser = value as _;
                }
                if e.is_final {
                    settings.crosshair.$accesser = value as _;
                    commands.queue(SaveSettingsDeferred::default());
                }
                commands
                    .entity(e.event_target())
                    .insert(::bevy::feathers::controls::NumberInputValue::F32(e.value));
            },
        )
    };
}

pub fn settings_item_crosshair<
    I: IntoObserverSystem<E, M> + Clone + Sync + Send,
    E: EntityEvent,
    M: 'static,
    F: Fn(&mut TemplateContext) -> Result<NumberInputValue> + Clone + Send + Sync + 'static,
>(
    label: &str,
    default_value: FnTemplate<F, NumberInputValue>,
    update_function: I,
) -> impl Scene {
    bsn! {
        Node {
            border: UiRect::bottom(space(0.5)),
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Start,
            width: percent(100.),
            overflow: Overflow::hidden(),
        }
        ThemeBorderColor(tokens::MENU_BORDER)
        InheritableThemeTextColor(tokens::BUTTON_TEXT)
        Children [
            Node {
                width: space(60.),
            }
            Text(label)
            ThemedText
            --
            ThemedText
            @FeathersNumberInput
            @default_value

            on(update_function)
        ]

    }
}

pub fn crosshair_settings() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            top: px(100.),
            right: px(100.),
            width: px(600.),
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: spacing::SM,
        }
        @ui::card(true)
        ThemeBackgroundColor(tokens::MENU_BG)
        ThemeTextColor(tokens::MENUITEM_TEXT)
        ThemeBorderColor(tokens::MENU_BORDER)
        Children [
            @{settings_item_crosshair!("Dot Size: ", f32 => dot_radius)}
            --
            @{settings_item_crosshair!("Crosshair Hairs: ", u32 => num_hairs)}
            --
            @{settings_item_crosshair!("Gap: ", f32 => gap)}
            --
            @{settings_item_crosshair!("Hair Rotation: ", f32 => hair_rotation)}
            --
            @{settings_item_crosshair!("Hair Length: ", f32 => hair_length)}
            --
            @{settings_item_crosshair!("Hair Width: ", f32 => hair_width)}
            --
            Node {
                width: percent(100.),
                height: px(300.),
                display: Display::Flex,
                padding: px(0.),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center
            }
            ThemeBorderColor(tokens::MENU_BORDER)
            @ui::card(true)
            template(|ctx| {
                Ok(MaterialNode(ctx.resource::<Crosshair>().0.clone()))
            })
        ]
    }
}

#[derive(States, Clone, Copy, Debug, Hash, PartialEq, PartialOrd, Ord, Eq, Default)]
pub enum ShownTab {
    #[default]
    Main,
    Settings,
}
