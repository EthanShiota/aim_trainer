use std::{
    error::Error,
    f32::consts::{PI, TAU},
    fs, io,
    path::{Path, PathBuf},
    sync::Arc,
    time,
};

use bevy::{
    app::on_r_inserted,
    camera::{CameraOutputMode, visibility::RenderLayers},
    color::palettes::{
        css::{self, BLACK, GREEN, RED, WHITE, YELLOW},
        tailwind,
    },
    ecs::entity_disabling::Disabled,
    feathers::{
        FeathersCorePlugin,
        controls::{
            FeathersButton, FeathersListRow, FeathersListView, FeathersMenu, FeathersMenuItem,
        },
        font_styles,
        theme::{
            self, InheritableThemeTextColor, ThemeBackgroundColor, ThemeBorderColor,
            ThemeTextColor, ThemedText,
        },
        tokens::{self, BUTTON_TEXT, MENU_BG, MENU_BORDER},
    },
    input::{common_conditions::input_just_pressed, mouse::MouseScrollPixelsPerLine},
    input_focus::{
        AutoFocus, InputFocus, directional_navigation::FocusableArea,
        pointer_focus::PointerFocusPlugin, tab_navigation::TabIndex,
    },
    math::AspectRatio,
    picking::hover::Hovered,
    prelude::*,
    render::render_resource::BlendState,
    scene::{Ready, SceneScope},
    settings::SaveSettings,
    tasks::{Task, futures::check_ready},
    text::{EditableText, TextCursorStyle, TextEdit, TextEditChange},
    ui::{Checked, InteractionDisabled, Pressed, Selectable, Selected},
    ui_widgets::{
        self, Activate, Button, ListBox, ListItem, ScrollArea, ScrollAreaPlugin, TextInput,
        TextInputPlugin, ValueChange, listbox_update_selection,
    },
    window::PrimaryWindow,
};
use bevy_egui::egui::Ui;
use bevy_egui::{egui::UiBuilder, prelude::*};
use rfd::FileDialog;
use ui::spacing;
use zip::{ZipArchive, result::ZipError};

use crate::{
    AppState, GameSettings, GameState, GameplaySettings, PlayerCamera, SoundSettings,
    menus::components::container, scenarios, scoreing::Score,
};

use components::crosshair_settings;
use parser::BeatMapOsu;

mod components;
pub struct MenuPlugin;
impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            EguiPrimaryContextPass,
            (settings_window).run_if(in_state(AppState::Menu)),
        )
        .add_systems(Startup, setup)
        .add_systems(OnEnter(AppState::Menu), main_menu.spawn())
        .add_plugins(FeathersCorePlugin)
        .add_systems(
            Update,
            settings_item_submit.run_if(input_just_pressed(KeyCode::Enter)),
        )
        .add_observer(
            |e: On<PointerClick>,
             q: Query<(Option<&Name>, &ComputedNode)>,
             children: Query<&ChildOf>,
             node_ref: Query<EntityRef>|
             -> Result {
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
        );
    }
}

fn settings_item_submit(mut focus: ResMut<InputFocus>, settings_item: Query<&EditableText>) {
    if let Some(focus_ent) = focus.get()
        && settings_item.contains(focus_ent)
    {
        focus.clear();
    }
}
fn button(text: &str) -> impl Scene {
    bsn! {
        @ui::card(true)
        Node {
            width: percent(100.),
            height: percent(50.),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            justify_items: JustifyItems::Center
        }
        ThemeBackgroundColor(tokens::BUTTON_BG)
        on(|e: On<PointerOver>, mut commands: Commands| {
            commands.entity(e.event_target()).insert(ThemeBackgroundColor(tokens::BUTTON_BG_HOVER));
        })
        on(|e: On<PointerOut>, mut commands: Commands| {
            commands.entity(e.event_target()).insert(ThemeBackgroundColor(tokens::BUTTON_BG));
        })
        Children [
            Node {justify_content: JustifyContent::Center}
            ThemedText
            Text(text)
        ]
    }
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
                 border: ui::space(2.),
            }
            ThemeBorderColor(MENU_BORDER)
            ThemeBackgroundColor(MENU_BG)
            Children [
                    @button("Exit")
                    on(|_: On<PointerPress>, mut commands: Commands| {
                        commands.set_state(GameState::Playing);
                    })
                    --
                    @button("Menu")
                    on(|_: On<PointerPress>, mut commands: Commands| {
                        commands.set_state(AppState::Menu);
                    })
            ]
        ]
    }
}

fn settings_window(
    mut contexts: EguiContexts,
    mut settings: If<ResMut<GameSettings>>,
    mut gameplay_settings: If<ResMut<GameplaySettings>>,
    mut sound_settings: If<ResMut<SoundSettings>>,
    mut confirm: Local<bool>,
    //mut sfx_override_buffer: Local<String>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut commands: Commands,
) -> Result {
    egui::Window::new("Settings")
        .default_pos((1000., 0.))
        .default_open(false)
        .show(contexts.ctx_mut()?, |ui| {
            if *confirm {
                if ui.button("confirm").clicked() {
                    *settings.as_mut() = GameSettings::default();
                    commands.queue(SaveSettings::IfChanged);
                    *confirm = false;
                }
                if ui.button("cancel").clicked() {
                    *confirm = false;
                }
            } else {
                ui.add(
                    egui::Slider::new(&mut sound_settings.music_volume, 0.0..=1.0)
                        .text("Music Volume"),
                );
                ui.add(
                    egui::Slider::new(&mut sound_settings.effects_volume, 0.0..=1.0)
                        .text("Effects Volume"),
                );
                // let sfx_response = ui.add(egui::TextEdit::singleline(&mut *sfx_override_buffer));
                // ui.label("Sfx Override, must be in assets directory");
                //
                // if sfx_response.changed()
                //     && std::fs::exists(format!("assets/{}", *sfx_override_buffer)).unwrap_or(false)
                // {
                //     sound_settings.sfx_override = Some(sfx_override_buffer.clone());
                // }

                ui.add(
                    egui::Slider::new(&mut settings.mouse_sensitivity, 0.5..=30.0)
                        .text("Mouse Sensitivity"),
                );
                ui.add(egui::Slider::new(&mut settings.dpi, 400..=3200).text("DPI"));
                ui.add(
                    egui::Slider::new(&mut settings.fov, 0.0..=PI)
                        .text("Vertical FOV in radians")
                        .step_by(0.0),
                );

                ui.add(
                    egui::Slider::new(&mut gameplay_settings.target_scale, 0.0..=2.0)
                        .text("Target Scale"),
                );

                let a: AspectRatio =
                    AspectRatio::try_from(window.size()).unwrap_or(AspectRatio::SIXTEEN_NINE);
                ui.label(format!(
                    "Vertical FOV: {} degrees\nHorizontal FOV: {}",
                    settings.fov * (360. / TAU),
                    settings.fov * (360. / TAU) * a.ratio()
                ));
                if ui.button("reset").clicked() {
                    *confirm = true;
                }
            }
        });
    Ok(())
}

fn setup(
    mut commands: Commands,
    mut egui_global_settings: ResMut<EguiGlobalSettings>,
    q_cam: Query<Entity, With<PlayerCamera>>,
) {
    egui_global_settings.auto_create_primary_context = false;

    commands.insert_resource(components::theme());

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

#[derive(Clone, Component)]
#[component(immutable)]
struct BeatMapGroup {
    // Lowercase title for search
    title: String,
    display_title: String,
    versions: Vec<BeatMapOsu>,
}

fn main_menu_egui(
    mut contexts: EguiContexts,
    mut commands: Commands,
    mut beat_maps: Local<Vec<BeatMapGroup>>,
    mut selected_file: Query<(&mut SelectedFile, Entity)>,
    mut search: Local<String>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let mut viewport_ui = Ui::new(
        ctx.clone(),
        "viewport".into(),
        UiBuilder::new().max_rect(ctx.viewport_rect()),
    );

    let beatmap_dir = "osu_beatmaps";
    // don't spin when beatmap_dir is empty
    once!(if beat_maps.is_empty() {
        *beat_maps = serialize_beatmaps(beatmap_dir);
    });

    egui::CentralPanel::default().show(&mut viewport_ui, |ui| {
        // Import new beatmap
        if ui.button("Import Beatmap").clicked() && selected_file.is_empty() {
            let thread_pool = bevy::tasks::AsyncComputeTaskPool::get();
            let task = thread_pool.spawn(async move { FileDialog::new().pick_file() });
            commands.spawn(SelectedFile(task));
        }
        if ui
            .button("Reload beatmaps")
            .on_hover_text("This reloads beatmaps from the osu_beatmaps directory.")
            .clicked()
        {
            *beat_maps = serialize_beatmaps(beatmap_dir);
            // let mut elements = vec![];
            // for map in beat_maps.iter().flatten().step_by(3) {
            //     elements.push(commands.spawn_scene(components::beatmap_version(map)).id());
            // }
            // commands
            //     .spawn_scene(components::container())
            //     .add_children(elements.as_slice());
        }

        ui.horizontal(|ui| {
            ui.label("Search");
            ui.text_edit_singleline(&mut *search);
        });

        for (mut file, entity) in selected_file.iter_mut() {
            if let Some(filepath) = check_ready(&mut file.0) {
                // Dialog can return None
                if let Some(path) = filepath {
                    debug!("path: {path:?}");
                    unzip_beatmap(Path::new(beatmap_dir), &path).unwrap();
                    *beat_maps = serialize_beatmaps(beatmap_dir);
                }
                commands.entity(entity).remove::<SelectedFile>();
            }
        }
        // Beatmap select
        for group in beat_maps
            .iter_mut()
            .filter(|map| map.title.contains(&*search.to_lowercase()))
        {
            egui::CollapsingHeader::new(&group.title).show(ui, |ui| {
                // use hit_object count as proxy for difficulty
                group.versions.sort_by_key(|entry| entry.hit_objects.len());
                for beat_map in group.versions.iter() {
                    let button = ui.button((beat_map.metadata.version).to_string());
                    if button.clicked() {
                        commands.set_state(AppState::InGame);
                        commands.run_system_cached_with(scenarios::osu, beat_map.clone());
                    }
                }
            });
        }
    });

    Ok(())
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
                versions.push(map);
            }
        }
        if let Some(map) = versions.first() {
            let title = map.metadata.title.to_lowercase();
            let display_title = map.metadata.title.clone();
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

pub fn score_summery(score: &Score) -> impl Scene {
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
        ThemeBackgroundColor(tokens::WINDOW_BG)
        InheritableThemeTextColor(tokens::TEXT_MAIN)
        Children [
            @header()
            ThemedText
            TextFont {
                font_size: px(32)
            }
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
                BorderColor::all(tailwind::GRAY_100)
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
                    Node {
                        width: px(300),
                        height: px(64),
                        border: px(4),
                        display: Display::Grid,
                        align_self: AlignSelf::End,
                    }
                    ThemeBorderColor(tokens::TEXT_MAIN)
                    ui_widgets::Button
                    ThemedText
                    Children [
                        Node {
                            align_self: AlignSelf::Center,
                            justify_self: JustifySelf::Center
                        }
                        ThemedText
                        Text("Menu")
                        TextLayout {
                            justify: Justify::Center,
                        }
                    ]
                    on(|_: On<PointerPress>, mut commands: Commands|{
                        commands.set_state(AppState::Menu);
                    })
                ]
            ]
        ]
    }
}

pub fn main_menu() -> impl Scene {
    let beatmap_dir = "osu_beatmaps";
    let beat_maps = serialize_beatmaps(beatmap_dir);

    bsn! {
        #MainMenu
        DespawnOnExit::<AppState>(AppState::Menu)

        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: spacing::SM,
            justify_items: JustifyItems::Start,
            width: vw(100.),
            height: vh(100.),

        }

        ThemeBackgroundColor(tokens::WINDOW_BG)
        Children [
            @header()
            --
            @search_bar()
            --
            #SelectorGrid
            Node {
                display: Display::Flex,
                column_gap: spacing::SM,
                overflow: Overflow::hidden(),
            }

            Children [
                @beatmap_selector(beat_maps)
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
            row_gap: spacing::SM
        }
        on(|e: On<BeatmapGroupSelected>, beatmap_group: Query<&BeatMapGroup>, mut commands: Commands| -> Result {
            let mut group = beatmap_group.get(e.beatmap_group)?.clone();
            // TODO: Better difficulty
            group.versions.sort_by_key(|map| map.hit_objects.len());
            let group: Vec<_> = group.versions.into_iter().map(|ver| {
                    bsn! {
                        #VersionOption
                        Button
                        @FeathersListRow
                        ThemeBackgroundColor({tokens::LISTROW_BG})
                        ThemeBorderColor({tokens::GROUP_BORDER})
                        Node {
                            padding: spacing::MD,
                            border: spacing::space(0.5),
                            align_items: AlignItems::Center

                            width: spacing::space(80.),
                            height: spacing::LG,
                        }
                        on(move |_: On<Activate>, mut commands: Commands| {
                            commands.set_state(AppState::InGame);
                            commands.run_system_cached_with(scenarios::osu, ver.clone());
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
            commands.entity(e.event_target()).apply_scene(
                bsn! {
                    ThemeBackgroundColor(tokens::MENUITEM_BG)
                    Children [
                        {group}
                    ]
                }
            );
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

fn beatmap_selector(maps: Vec<BeatMapGroup>) -> impl Scene {
    let map_list: Vec<_> = maps
        .into_iter()
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
        .collect();
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

fn header() -> impl Scene {
    bsn! {
        #MenuHeader
        Node {
            display: Display::Flex,
            width: vw(100.),
            padding: px(16),
            min_height: px(64)
        }
        ThemedText
        ThemeBackgroundColor(tokens::MENU_BG)
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
            on(|e: On<TextEditChange>, mut prev: Local<String>, text_edit: Query<&EditableText>, mut beatmaps: Query<(Entity, &BeatMapGroup, &mut Node)>, mut commands: Commands| -> Result {
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
