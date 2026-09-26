use bevy::{
    color::palettes::tailwind,
    feathers::{
        theme::{ThemeBackgroundColor, ThemeProps, ThemeTextColor, UiTheme},
        tokens::{self, BUTTON_BG, TEXT_MAIN},
    },
    platform::collections::HashMap,
    prelude::*,
};
use parser::BeatMapOsu;

pub fn theme() -> UiTheme {
    UiTheme(ThemeProps {
        color: HashMap::from([
            (tokens::WINDOW_BG, Color::Srgba(tailwind::SLATE_700)),
            (tokens::BUTTON_BG, Color::Srgba(tailwind::PINK_300)),
            (tokens::TEXT_MAIN, Color::Srgba(tailwind::SLATE_700)),
            (tokens::TEXT_DIM, Color::Srgba(tailwind::SLATE_400)),
        ]),
    })
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
