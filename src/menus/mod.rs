use std::{
    error::Error,
    fs, io,
    path::{Path, PathBuf},
    time,
};

use bevy::{
    camera::{CameraOutputMode, visibility::RenderLayers},
    ecs::template::OptionTemplate,
    feathers::{
        FeathersCorePlugin,
        controls::{FeathersButton, FeathersListRow, FeathersListView},
        font_styles::InheritableFont,
        theme::{
            InheritableThemeTextColor, ThemeBackgroundColor, ThemeBorderColor, ThemeTextColor,
            ThemedText,
        },
        tokens::{self, MENU_BG, MENU_BORDER},
    },
    input_focus::tab_navigation::{TabGroup, TabIndex},
    picking::hover::Hovered,
    prelude::*,
    render::render_resource::BlendState,
    tasks::{Task, futures::check_ready},
    text::{EditableText, TextCursorStyle, TextEditChange},
    ui::Selected,
    ui_widgets::{
        Activate, Button, ControlOrientation, ListItem, SelectedTab, Tab, TabActivation, TabList,
        TextInput, ValueChange, listbox_update_selection, tablist_self_update,
    },
};
use bevy_egui::prelude::*;
use rfd::FileDialog;
use ui::spacing;
use zip::{ZipArchive, result::ZipError};

use crate::{
    AppState, GameState, PlayerCamera, menus::components::ShownTab, scenarios, scoreing::Score,
};

pub use ui::{DisplayInState, apply_display_in_state};

use components::crosshair_settings;
use parser::BeatMapOsu;

mod components;
mod settings;
use settings::*;

pub struct MenuPlugin;
impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OsuBeatMaps>()
            .insert_resource(components::theme())
            .add_systems(Startup, (setup_egui_camera, main_menu_spawner))
            .add_systems(Update, load_selected_file_async)
            .add_plugins(FeathersCorePlugin)
            .init_state::<ShownTab>()
            .add_systems(StateTransition, apply_display_in_state::<ShownTab>)
            .add_systems(StateTransition, apply_display_in_state::<AppState>)
            .add_systems(Update, (style_tab_header, settings::reset_on_keypress))
            // TODO: move to debug
            .add_observer(
                |e: On<PointerClick>,
                 q: Query<(Option<&Name>, &ComputedNode)>,
                 children: Query<&ChildOf>,
                 node_ref: Query<EntityRef>,
                 debug: Res<GlobalUiDebugOptions>|
                 -> Result {
                    if !debug.enabled {
                        return Ok(());
                    }
                    if let Ok((name, node)) = q.get(e.entity) {
                        debug!(?name, size = ?&node.size, content_size = ?&node.content_size);
                        for node in children.iter_ancestors(e.entity) {
                            if let Ok(name) = node_ref.get(node)?.get_components::<&Name>() {
                                debug!(?name);
                            }
                        }
                    }
                    Ok(())
                },
            )
            .add_observer(
                |_: On<Insert<OsuBeatMaps>>,
                 mut commands: Commands,
                 selector: Query<Entity, With<BeatmapSelector>>,
                 beatmaps: Res<OsuBeatMaps>| {
                    // Should only be one selector
                    let selector = selector.single().unwrap();

                    let rows = beatmap_selector_list(&beatmaps);
                    commands.entity(selector).despawn_children();
                    commands.entity(selector).apply_scene(bsn! {
                        @FeathersListView {
                            @rows: bsn_list!({rows})
                        }
                    });
                },
            );
    }
}

fn setup_egui_camera(
    mut commands: Commands,
    mut egui_global_settings: ResMut<EguiGlobalSettings>,
    _q_cam: Query<Entity, With<PlayerCamera>>,
) {
    egui_global_settings.auto_create_primary_context = false;

    // Egui Camera
    commands.spawn((
        // The `PrimaryEguiContext` component requires everything needed to render a primary context.
        PrimaryEguiContext,
        Camera2d,
        // Setting RenderLayers to none makes sure we won't render anything apart from the UI.
        RenderLayers::none(),
        Msaa::Off,
        Camera {
            order: 2,
            output_mode: CameraOutputMode::Write {
                blend_state: Some(BlendState::ALPHA_BLENDING),
                clear_color: ClearColorConfig::None,
            },
            clear_color: ClearColorConfig::Custom(Color::NONE),
            ..default()
        },
    ));
}

#[derive(Component)]
struct SelectedFile(Task<Option<PathBuf>>);

fn load_selected_file_async(
    mut selected_file: Query<(&mut SelectedFile, Entity)>,
    mut commands: Commands,
) {
    // TODO: use res
    let beatmap_dir = "osu_beatmaps";
    for (mut file, entity) in selected_file.iter_mut() {
        if let Some(filepath) = check_ready(&mut file.0) {
            // Dialog can return None
            if let Some(path) = filepath {
                debug!("path: {path:?}");
                unzip_beatmap(Path::new(beatmap_dir), &path).unwrap();
                commands.insert_resource(OsuBeatMaps(serialize_beatmaps(beatmap_dir)));
            }
            commands.entity(entity).remove::<SelectedFile>();
        }
    }
}

fn unzip_beatmap(base_dir: &Path, beatmap_path: &Path) -> Result<(), Box<dyn Error>> {
    // Validate the path without requiring the file to exist
    // let out_root = candidate_path.components().collect::<std::path::PathBuf>();
    // if !out_root.starts_with(&base_dir) {
    //     error!(
    //         "Error: path {:?} escapes the allowed directory.",
    //         candidate_path.display()
    //     );
    //     return Err("Invalid path".into());
    // }

    let mut archive = std::fs::File::open(beatmap_path)
        .map_err(ZipError::from)
        .and_then(ZipArchive::new)?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let comment = file.comment();
        if !comment.is_empty() {
            trace!("{comment}");
        }
        if let Some(out_path) = file.enclosed_name() {
            let out_path = base_dir
                .join(beatmap_path.file_name().unwrap())
                .join(out_path)
                .components()
                .collect::<std::path::PathBuf>();
            if !out_path.starts_with(base_dir) {
                error!("error path escapes base dir: {out_path:?}");
                return Err("Path escapes base dir".into());
            }

            if file.is_dir() {
                if let Err(e) = fs::create_dir_all(&out_path) {
                    error!("error creating directory: {e:?}");
                }
            } else {
                if let Some(p) = out_path.parent()
                    && !p.exists()
                    && let Err(e) = fs::create_dir_all(p)
                {
                    error!(
                        "Error: unable to create parent directory {p:?} of file {}: {e}",
                        p.display()
                    );
                    continue;
                }
                match fs::File::create(&out_path)
                    .and_then(|mut outfile| io::copy(&mut file, &mut outfile))
                {
                    Ok(bytes_extracted) => {
                        println!(
                            "File {} extracted to {:?} ({bytes_extracted} bytes)",
                            i,
                            out_path.display(),
                        );
                    }
                    Err(e) => {
                        error!(
                            "Error: unable to extract file {i} to {:?}: {e}",
                            out_path.display()
                        );
                        continue;
                    }
                }
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                if let Some(mode) = file.unix_mode()
                    && let Err(e) = fs::set_permissions(&out_path, fs::Permissions::from_mode(mode))
                {
                    error!(
                        "Error: unable to change permissions of file {i} ({:?}): {e}",
                        out_path.display()
                    );
                }
            }
        } else {
            info!("skipping file with invalid path: {}", file.name()?);
        }
    }
    Ok(())
}

// TODO: Async
fn serialize_beatmaps(beatmap_dir: &str) -> Vec<BeatMapGroup> {
    let now = time::Instant::now();
    let mut count = 0;
    debug!("begin serialization");
    let mut beat_maps = vec![];
    if let Ok(exists) = std::fs::exists(beatmap_dir)
        && !exists
    {
        // create beatmap_dir if it doesn't exist
        std::fs::create_dir(beatmap_dir).expect("could not create {beatmap_dir}!");
    }
    for file in std::fs::read_dir(beatmap_dir)
        .expect("could not read beatmap dir")
        .flat_map(|w| w.ok())
    {
        let mut versions = vec![];
        for ent in file.path().read_dir().unwrap() {
            if let Some(map) = ent.ok().and_then(|dir| BeatMapOsu::new(dir.path()).ok()) {
                count += 1;
                versions.push(Arc::new(map));
            }
        }
        if let Some(map) = versions.first() {
            let title = map.metadata.title.to_lowercase();
            let display_title = map.metadata.title.clone();
            // Sort versions
            versions.sort_by_key(|entry| entry.hit_objects.len());
            let beat_map_group = BeatMapGroup {
                title,
                display_title,
                versions,
            };

            beat_maps.push(beat_map_group);
        }
    }
    debug!(
        "end serialization of {count} maps in {} secs",
        time::Instant::now().duration_since(now).as_secs_f64()
    );
    beat_maps
}

pub fn pause_menu() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            width: percent(100.),
            height: percent(100.)
        }
        DespawnOnExit::<GameState>(GameState::Paused)
        Children [
            @crosshair_settings()
            --
            Node {
                 flex_direction: FlexDirection::Column,
                 width: ui::space(64.),
                 height: ui::space(48.),
                 align_items: AlignItems::Center,
                 justify_content: JustifyContent::SpaceBetween,
            }
            @ui::card(true)
            ThemeBorderColor(MENU_BORDER)
            ThemeBackgroundColor(MENU_BG)
            Children [
                    @pause_menu_button("Exit")
                    on(|_: On<Activate>, mut commands: Commands| {
                        commands.set_state(GameState::Playing);
                    })
                    --
                    @pause_menu_button("Menu")
                    on(|_: On<Activate>, mut commands: Commands| {
                        commands.set_state(AppState::Menu);
                    })
            ]
        ]
    }
}

fn pause_menu_button(label: &'static str) -> impl Scene {
    bsn! {
        @FeathersButton {
            @caption: bsn_list! {
                ThemedText
                Text(label)
            }
        }
        InheritableFont {
            font_size: rem(2.),
            weight: FontWeight(800)
        }
        Node {
            width: percent(100.),
            flex_grow: 1.
        }
        @ui::card(true)
    }
}

#[derive(Clone, Component)]
#[component(immutable)]
struct BeatMapGroup {
    // Lowercase title for search
    title: String,
    display_title: String,
    versions: Vec<Arc<BeatMapOsu>>,
}

// After game is over
pub fn score_summery_menu(score: &Score) -> impl Scene {
    let acc = score.hit_counts.accuracy();
    let map_name = &score.metadata.metadata.title;
    bsn! {
        DespawnOnExit::<AppState>(AppState::ScoreSummery)
        Node {
            width: percent(100.),
            height: percent(100.),
            display: Display::Flex,
            flex_direction: FlexDirection::Column
        }
        ThemeBorderColor(tokens::MENU_BORDER)
        ThemeBackgroundColor(tokens::WINDOW_BG)
        InheritableThemeTextColor(tokens::TEXT_MAIN)
        Children [
            Node {
                display: Display::Flex,
                width: vw(100.),
                padding: px(16),
                min_height: px(64)
            }
            ThemedText
            TextFont {
                font_size: px(32)
            }
            ThemeBorderColor(tokens::MENU_BORDER)
            Text(map_name)
            --
            Node {
                width: percent(100.),
                flex_grow: 1.,
            }
            ThemedText
            Children [
                Node {
                    height: percent(100.),
                    aspect_ratio: 1f32,
                    border: px(3),
                    padding: px(20)
                }
                ThemeBorderColor(tokens::MENU_BORDER)
                @ui::card(true)
                --
                Node {
                    display: Display::Flex,
                    flex_grow: 1.,
                    padding: px(20)
                }
                ThemedText
                TextFont {
                    font_size: px(32)
                }
                Text({format!("Score {}\nAccuracy {acc:.4}", score.points)})
                Children [
                    ThemeBorderColor(tokens::TEXT_MAIN)
                    @FeathersButton {
                        @caption: bsn_list! {
                            ThemedText
                            Node {
                                align_self: AlignSelf::Center,
                                justify_self: JustifySelf::Center
                            }
                            ThemedText
                            Text("Menu")
                            TextLayout {
                                justify: Justify::Center,
                            }
                        }
                    }
                    Node {
                        width: px(300),
                        height: px(64),
                        border: px(4),
                        display: Display::Grid,
                        align_self: AlignSelf::End,
                    }
                    @ui::card(true)

                    on(|_: On<Activate>, mut commands: Commands|{
                        commands.set_state(AppState::Menu);
                    })
                ]
            ]
        ]
    }
}

// System to spawn the main menu, loads beatmaps from files
fn main_menu_spawner(mut beatmaps: ResMut<OsuBeatMaps>, mut commands: Commands) {
    once!({
        if beatmaps.is_empty() {
            let beatmap_dir = "osu_beatmaps";
            let beat_maps = serialize_beatmaps(beatmap_dir);
            **beatmaps = beat_maps;
        }
    });

    commands.spawn_scene(main_menu_scene(beatmaps));
}

#[derive(Component, Clone, Copy, Default)]
struct MainMenu;

fn main_menu_scene(beatmaps: ResMut<OsuBeatMaps>) -> impl Scene {
    bsn! {
        #MainMenu
        DisplayInState::<AppState>(AppState::Menu)

        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: spacing::SM,
            justify_items: JustifyItems::Start,
            overflow: Overflow::hidden(),
            width: vw(100.),
            height: vh(100.),
        }

        ThemeBackgroundColor(tokens::WINDOW_BG)
        MainMenu
        Children [
            @header()
            --
            Node {
                flex_direction: FlexDirection::Column,
                justify_items: JustifyItems::Start,
                align_items: AlignItems::Start,
                overflow: Overflow::hidden(),
                width: percent(100.),
                height: percent(100.),
            }
            Children [
                DisplayInState::<ShownTab>(ShownTab::Main)
                @main_tab(&*beatmaps)
                --
                DisplayInState::<ShownTab>(ShownTab::Settings)
                @settings_tab()
            ]
        ]
    }
}

fn main_tab(beatmaps: &OsuBeatMaps) -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: spacing::SM,
            justify_items: JustifyItems::Start,
            overflow: Overflow::hidden(),
            width: percent(100.),
            height: percent(100.),
        }
        ThemeBorderColor(tokens::MENU_BORDER)
        @ui::card(true)

        Children [
            @search_bar()
            --
            #SelectorGrid

            Node {
                display: Display::Flex,
                column_gap: spacing::SM,
                overflow: Overflow::hidden(),
            }

            Children [
                @beatmap_selector(beatmaps)
                --
                @beatmap_version_selector()
            ]
        ]
    }
}

fn beatmap_version_selector() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            width: spacing::space(80.),
        }
        on(|e: On<BeatmapGroupSelected>, beatmap_group: Query<&BeatMapGroup>, mut commands: Commands| -> Result {
            let group = beatmap_group.get(e.beatmap_group)?;
            // TODO: Better difficulty
            let group: Vec<_> = group.versions.iter().cloned().map(|ver| {
                    bsn! {
                        #VersionOption
                        Button
                        @FeathersListRow
                        ThemeBackgroundColor({tokens::LISTROW_BG})
                        ThemeBorderColor({tokens::GROUP_BORDER})
                        Node {
                            padding: spacing::MD,
                            border: spacing::space(0.5),
                            align_items: AlignItems::Center,
                            margin: UiRect::bottom(spacing::SM),

                            width: spacing::space(80.),
                            height: spacing::LG,
                            overflow: Overflow::hidden(),
                        }
                        on(move |_: On<Activate>, mut commands: Commands| {
                            commands.run_system_cached_with(scenarios::osu, ver.clone());
                            commands.set_state(AppState::InGame);
                        })

                        Children [
                            Text({&ver.metadata.version})
                            ThemedText
                            ThemeTextColor(tokens::TEXT_MAIN)
                            TextFont {
                                font_size: px(20.)
                            }
                        ]
                    }
            }).collect();
            commands.entity(e.event_target()).despawn_children();
            commands.entity(e.event_target()).apply_scene(bsn! {
                Children [
                    {group}
                ]

            });
            Ok(())

        })
        BeatmapVersionSelector

    }
}

#[derive(Component, Clone, Copy, Default)]
struct BeatmapSelector;

#[derive(Component, Clone, Copy, Default)]
struct BeatmapVersionSelector;

#[derive(EntityEvent)]
struct BeatmapGroupSelected {
    entity: Entity,
    beatmap_group: Entity,
}

use std::sync::Arc;
#[derive(Resource, Default, Clone, Deref, DerefMut)]
struct OsuBeatMaps(Vec<BeatMapGroup>);

fn beatmap_selector_list(maps: &OsuBeatMaps) -> Vec<impl Scene> {
    maps.iter()
        .cloned()
        .map(|beatmap_group| {
            bsn! {
                #BeatmapGroup
                @FeathersListRow
                Node {
                    padding: spacing::MD,
                    border: spacing::space(0.5),
                    align_items: AlignItems::Center,
                    margin: UiRect::bottom(spacing::SM),

                    width: spacing::space(80.),
                    height: spacing::LG,
                }
                // Button
                ListItem
                ThemeBackgroundColor({tokens::LISTROW_BG})
                ThemeBorderColor({tokens::GROUP_BORDER})
                TabIndex(-1)
                beatmap_group
                Children [
                    ThemedText
                    ThemeTextColor(tokens::TEXT_MAIN)

                    Text({&beatmap_group.display_title})

                    TextFont {
                        font_size: px(20.)
                    }
                ]
            }
        })
        .collect()
}
fn beatmap_selector(maps: &OsuBeatMaps) -> impl Scene {
    let map_list: Vec<_> = beatmap_selector_list(maps);
    bsn! {
            @FeathersListView {
                @rows: bsn_list!{{map_list}}
            }

            #BeatmapSelector
            BeatmapSelector

            on(|event: On<ValueChange<Entity>>, mut commands: Commands, version_selector: Query<Entity, With<BeatmapVersionSelector>>| -> Result {
                let selector =  version_selector.single()?;
                commands.trigger(BeatmapGroupSelected {entity: selector, beatmap_group: event.value});
                Ok(())
            })
            on(listbox_update_selection)

    }
}

fn style_tab_header(
    q_header: Query<(Entity, Has<Selected>, &Hovered, &ThemeBorderColor), With<TabHeader>>,
    mut commands: Commands,
) {
    for (entity, selected, hovered, border) in q_header.iter() {
        let border_token = match (selected, hovered.0) {
            (true, _) => tokens::FOCUS_RING,
            (_, true) => tokens::FOCUS_RING,
            (false, false) => tokens::MENU_BORDER,
        };
        if border.0 != border_token {
            commands
                .entity(entity)
                .insert(ThemeBorderColor(border_token));
        }
    }
}

#[derive(Component, Clone, Copy, Default)]
struct TabHeader;
fn tab_header(label: &'static str, s: ShownTab) -> impl Scene {
    bsn! {
        on(move |_: On<Add<Selected>>, mut commands: Commands| {
            commands.set_state(s);
        })
        Tab
        TabHeader
        Hovered
        ThemeBackgroundColor(tokens::WINDOW_BG)
        ThemeBorderColor(tokens::MENU_BORDER)
        Node {
            align_items: AlignItems::Center,
            height: percent(100.),

            width: spacing::space(80.),
            justify_content: JustifyContent::Center,
            border: px(2.)
        }
        Children [
            Text(label)
            ThemedText
            TextFont {
                font_size: px(28.)
            }

        ]

    }
}

fn header() -> impl Scene {
    bsn! {
        #MenuHeader
        Node {
            display: Display::Flex,
            width: vw(100.),
            min_height: px(64)
        }
        ThemedText
        ThemeBackgroundColor(tokens::MENU_BG)

        InheritableThemeTextColor(tokens::BUTTON_TEXT)

        TabGroup
        Children [
            Node {
                flex_grow: 1.,
                height: percent(100.)
            }
            TabList {
                orientation: ControlOrientation::Horizontal,
                activation: TabActivation::Manual,
            }
            SelectedTab(OptionTemplate::Some(#Main))
            on(tablist_self_update)
            Children [
                #Main
                @tab_header("Main", ShownTab::Main)
                --
                #Settings
                @tab_header("Settings", ShownTab::Settings)
            ]
        ]
    }
}

fn import_beatmap() -> impl Scene {
    bsn! {
        @FeathersButton
        Node {
            padding: spacing::MD,
            border: spacing::space(0.5),
            align_items: AlignItems::Center,

            width: spacing::space(80.),
            height: spacing::LG,
        }
        on(|_: On<Activate>, mut commands: Commands| {
            let thread_pool = bevy::tasks::AsyncComputeTaskPool::get();
            let task = thread_pool.spawn(async move { FileDialog::new().pick_file() });
            commands.spawn(SelectedFile(task));
        })
        Children [
            ThemedText
            Text("Import Beatmap")
        ]
    }
}

fn reload_beatmaps() -> impl Scene {
    bsn! {
        @FeathersButton

        Node {
            padding: spacing::MD,
            border: spacing::space(0.5),
            align_items: AlignItems::Center,

            width: spacing::space(80.),
            height: spacing::LG,
        }
        on(|_: On<Activate>, mut commands: Commands| {
            // TODO: use res
            let beatmap_dir = "osu_beatmaps";
            commands.insert_resource(OsuBeatMaps(serialize_beatmaps(beatmap_dir)));

        })
        Children [
            ThemedText
            Text("Reload Beatmaps")
        ]
    }
}

fn search_bar() -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
        }
        Children [
            Node {
                display: Display::Flex,
                align_content: AlignContent::Center,
                flex_wrap: FlexWrap::NoWrap,
                width: em(50),
                padding: em(0.5)
            }
            ThemeBorderColor(tokens::MENU_BORDER)
            @ui::card(true)
            TextCursorStyle
            ThemeBackgroundColor(tokens::TEXT_INPUT_BG)
            ThemeTextColor(tokens::TEXT_INPUT_TEXT)
            ThemedText
            EditableText {
                allow_newlines: false
            }
            TextFont {
                font_size: px(24.)
            }
            TextInput
            TabIndex(-1)
            on(|e: On<TextEditChange>, mut prev: Local<String>, text_edit: Query<&EditableText>, mut beatmaps: Query<(Entity, &BeatMapGroup, &mut Node)>, mut _commands: Commands| -> Result {
                let text_edit = text_edit.get(e.event_target())?;
                if text_edit.value() == &**prev {
                    // no change
                    return Ok(());
                } else {
                    *prev = text_edit.value().to_string().to_lowercase();
                }

                for (_ent, map, mut node) in beatmaps.iter_mut() {
                    if map.title.contains(&*prev) {
                        node.display = Display::Flex;
                    } else {
                        node.display = Display::None;
                    }
                }

                Ok(())

            })
        ]
    }
}
