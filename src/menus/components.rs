use bevy::{
    color::palettes::tailwind,
    ecs::{system::IntoObserverSystem, template::TemplateContext},
    feathers::{
        dark_theme::create_dark_theme,
        theme::{
            InheritableThemeTextColor, ThemeBackgroundColor, ThemeBorderColor, ThemeProps,
            ThemeTextColor, ThemedText, UiTheme,
        },
        tokens::{self, BUTTON_BG, TEXT_MAIN},
    },
    input_focus::{
        AcquireFocus, AutoFocus, InputFocus, directional_navigation::FocusableArea,
        tab_navigation::TabIndex,
    },
    platform::collections::HashMap,
    prelude::*,
    text::{EditableText, EditableTextFilter, TextCursorStyle},
    ui::FocusPolicy,
    ui_widgets::TextInput,
};

use bevy::settings::SaveSettingsDeferred;
use parser::BeatMapOsu;

use bevy::text::TextEditChange;
use ui::space;

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
pub fn container() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            width: percent(50.),
            height: percent(100.),
        }
    }
}

// settings_item_crosshair!("label", u32 => accesser)
macro_rules! settings_item_crosshair {
    ($label:expr, $parse_type:ty => $accesser:ident) => {
        settings_item(
            $label,
            |ctx| {
                let crosshair_handle = ctx.resource::<Crosshair>().0.id();
                let crosshair_assets = ctx.resource_mut::<Assets<CrosshairMaterial>>();
                crosshair_assets
                    .get(crosshair_handle)
                    .map(|mat| mat.$accesser.to_string())
            },
            |e: On<TextEditChange>,
             mut q_text: Query<&mut EditableText>,
             mut mat: ResMut<Assets<CrosshairMaterial>>,
             mut commands: Commands,
             mut settings: ResMut<crate::CrosshairSettings>| {
                let Ok(text) = q_text.get_mut(e.event_target()) else {
                    return;
                };
                let value = text.value().to_string().parse::<$parse_type>();
                if let Ok(val) = value {
                    for (_, m) in mat.iter_mut() {
                        m.$accesser = val;
                    }
                    settings.crosshair.$accesser = val;
                    commands.queue(SaveSettingsDeferred::default());
                }
            },
        )
    };
}

pub fn settings_item<
    I: IntoObserverSystem<E, M> + Clone + Sync + Send,
    E: EntityEvent,
    M: 'static,
>(
    label: &str,
    default_value: impl Fn(&mut TemplateContext) -> Option<String> + Send + Sync + 'static + Clone,
    update_function: I,
) -> impl Scene {
    bsn! {
        Node {
            border: UiRect::bottom(space(0.5)),
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            width: percent(100.),
        }
        ThemeBorderColor(tokens::MENU_BORDER)
        InheritableThemeTextColor(tokens::BUTTON_TEXT)
        Children [
            Text(label)
            ThemedText
            --
            Node {
                flex_grow: 1.,
            }
            ThemedText
            template(move |ctx| {
                Ok(EditableText::new(default_value(ctx).unwrap_or_default()))
            })
            TextInput
            on(update_function)
            // HACK: Allows for pointer focus
            TabIndex(0)
            TextCursorStyle
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
            flex_direction: FlexDirection::Column
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

pub fn beatmap_accordian(beatmap: &BeatMapOsu, asset_server: &mut AssetServer) -> impl Scene {
    let bg_image = asset_server
        .load_builder()
        .override_unapproved()
        .load(beatmap.background().unwrap());
    bsn! {
        ImageNode {
            image: bg_image,
            image_mode: NodeImageMode::Auto
        }
        ThemeBackgroundColor(BUTTON_BG)
        ThemeTextColor(TEXT_MAIN)
        Text({&beatmap.metadata.title})
    }
}

/// Used for a single version of a beatmap
pub fn beatmap_version(beatmap: &BeatMapOsu) -> impl Scene {
    bsn! {
        ThemeBackgroundColor(BUTTON_BG)
        ThemeTextColor(TEXT_MAIN)
        Text({&beatmap.metadata.version})
    }
}
