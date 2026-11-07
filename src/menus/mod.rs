use std::{
    error::Error,
    f32::consts::{PI, TAU},
    fs, io,
    ops::{DerefMut, RangeInclusive},
    path::{Path, PathBuf},
    time,
};

use bevy::{
    camera::{CameraOutputMode, visibility::RenderLayers},
    color::palettes::tailwind,
    ecs::{
        lifecycle::HookContext,
        template::{EntityTemplate, OptionTemplate, TemplateContext},
        world::DeferredWorld,
    },
    feathers::{
        FeathersCorePlugin,
        controls::{
            FeathersButton, FeathersListRow, FeathersListView, FeathersNumberInput, NumberFormat,
            NumberInputPrecision, NumberInputStep, NumberInputUnits, NumberInputValue, SoftLimit,
        },
        theme::{
            InheritableThemeTextColor, ThemeBackgroundColor, ThemeBorderColor, ThemeTextColor,
            ThemedText,
        },
        tokens::{self, MENU_BG, MENU_BORDER},
    },
    input::common_conditions::input_just_pressed,
    input_focus::{
        AutoFocus, InputFocus,
        tab_navigation::{TabGroup, TabIndex},
    },
    math::AspectRatio,
    picking::hover::Hovered,
    prelude::*,
    render::render_resource::BlendState,
    scene::Ready,
    settings::{SaveSettings, SaveSettingsDeferred},
    tasks::{Task, futures::check_ready},
    text::{EditableText, TextCursorStyle, TextEditChange, TextSection},
    ui::Selected,
    ui_widgets::{
        self, Activate, Button, ControlOrientation, ListItem, SelectedTab, Tab, TabActivation,
        TabList, TextInput, ValueChange, listbox_update_selection, tablist_self_update,
    },
    window::PrimaryWindow,
};
use bevy_egui::egui::Ui;
use bevy_egui::{egui::UiBuilder, prelude::*};
use rand::seq::IndexedRandom;
use rfd::FileDialog;
use ui::{space, spacing};
use zip::{ZipArchive, result::ZipError};

use crate::{
    AppState, GameSettings, GameState, GameplaySettings, PlayerCamera, SoundSettings, scenarios,
    scoreing::Score,
};

use components::crosshair_settings;
use parser::BeatMapOsu;

mod components;
pub struct MenuPlugin;
impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, poll_async)
            .add_systems(Startup, setup)
            .init_resource::<OsuBeatMaps>()
            .insert_resource(components::theme())
            .add_systems(Startup, main_menu)
            .add_plugins(FeathersCorePlugin)
            .add_systems(
                Update,
                settings_item_submit.run_if(input_just_pressed(KeyCode::Enter)),
            )
            .init_state::<ShownTab>()
            .add_systems(StateTransition, apply_display_in_state::<ShownTab>)
            .add_systems(StateTransition, apply_display_in_state::<AppState>)
            .add_systems(Update, style_tab_header)
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

                    let rows = beatmap_selector_list(&*beatmaps);
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

fn poll_async(mut selected_file: Query<(&mut SelectedFile, Entity)>, mut commands: Commands) {
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

#[derive(Component, Clone, Copy, Default)]
struct SettingsItem;
fn settings_item(label: &'static str, range: RangeInclusive<f32>) -> impl Scene {
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
                Ok(NumberInputValue::F32(val as _))
            })
            on(|e: On<ReloadSettings>, ctx: Res<$res>, mut commands: Commands| {
                let val = if e.reset {
                    <$res>::default().$access
                } else {
                    ctx.$access
                };
                commands.entity(e.event_target()).insert(NumberInputValue::F32(val as _));
            })

            on(|val: On<ValueChange<f32>>, mut commands: Commands, mut res: ResMut<$res>| {
                if val.is_final {
                    res.$access = val.value as _;
                    commands.queue(::bevy::settings::SaveSettingsDeferred::default());
                }
                commands.entity(val.event_target()).insert(NumberInputValue::F32(val.value as _));

            })
        }
    };
}

#[derive(EntityEvent, Clone, Copy)]
struct ReloadSettings {
    entity: Entity,
    reset: bool,
}

fn settings_window() -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Column,
            width: space(80.),
            row_gap: spacing::SM,
        }
        InheritableThemeTextColor(tokens::TEXT_MAIN)

        Children [
            @{settings_sync!(SoundSettings => music_volume)}
            @settings_item("Music Volume", 0.0..=1.0)
            --
            @{settings_sync!(SoundSettings => effects_volume)}
            @settings_item("Effects Volume", 0.0..=1.0)
            --
            @{settings_sync!(GameSettings => mouse_sensitivity)}
            @settings_item("Mouse Sensitivity (in/360)", 0.0..=100.0)
            --
            @{settings_sync!(GameSettings => dpi)}
            NumberInputStep(400.)
            NumberInputPrecision(0)
            @settings_item("DPI", 400.0..=8000.0)
            --
            @{settings_sync!(GameSettings => fov)}
            @settings_item("Vertical FOV in radians", 0.0..=PI)
            --
            @{settings_sync!(GameplaySettings => target_scale)}
            @settings_item("Target Scale", 0.0..=2.0)
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

fn settings_window_egui(
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

#[derive(Clone, Component)]
#[component(immutable)]
struct BeatMapGroup {
    // Lowercase title for search
    title: String,
    display_title: String,
    versions: Vec<Arc<BeatMapOsu>>,
}

fn main_menu_egui(
    mut contexts: EguiContexts,
    mut commands: Commands,
    mut beat_maps: ResMut<OsuBeatMaps>,
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
        **beat_maps = serialize_beatmaps(beatmap_dir);
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
            **beat_maps = serialize_beatmaps(beatmap_dir);
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
                    **beat_maps = serialize_beatmaps(beatmap_dir);
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

// System to spawn the main menu
fn main_menu(mut beatmaps: ResMut<OsuBeatMaps>, mut commands: Commands) {
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
        //DespawnOnExit::<AppState>(AppState::Menu)
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
            DisplayInState::<ShownTab>(ShownTab::Main)
            @main_tab(&*beatmaps)
            --
            DisplayInState::<ShownTab>(ShownTab::Settings)
            @settings_window()
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
            width: vw(100.),
            height: vh(100.),
        }

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
                    @FeathersListView {
                        @rows: bsn_list!{{group}}
                    }
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

#[derive(States, Clone, Copy, Debug, Hash, PartialEq, PartialOrd, Ord, Eq, Default)]
enum ShownTab {
    #[default]
    Main,
    Settings,
}

#[derive(Component, Clone, Debug, PartialEq, Eq, Default)]
#[require(Node)]
#[component(on_add)]
pub struct DisplayInState<S: States>(pub S);

impl<S: States> DisplayInState<S> {
    fn on_add(mut world: DeferredWorld, ctx: HookContext) {
        let state = world.get_resource::<State<S>>().unwrap().get().clone();

        if let Ok((mut node, display_state, prior_display)) = world
            .entity_mut(ctx.entity)
            .get_components_mut::<(&mut Node, &DisplayInState<S>, Option<&PriorDisplay>)>()
        {
            if state == display_state.0 {
                if let Some(prior_display) = prior_display {
                    node.display = prior_display.0;
                } else {
                    node.display = Display::Flex;
                }
            } else {
                let prior = node.display;
                node.display = Display::None;
                world
                    .commands()
                    .entity(ctx.entity)
                    .insert(PriorDisplay(prior));
            }
        }
    }
}

#[derive(Component, Clone, Debug)]
pub struct PriorDisplay(Display);

pub fn apply_display_in_state<S: States>(
    mut event: MessageReader<StateTransitionEvent<S>>,
    mut commands: Commands,
    mut q_display_in_state: Query<(Entity, &mut Node, &DisplayInState<S>, Option<&PriorDisplay>)>,
) {
    let Some(msg) = event.read().last() else {
        return;
    };

    let StateTransitionEvent {
        entered,
        exited,
        allow_same_state_transitions,
    } = msg;
    if entered == exited && !allow_same_state_transitions {
        return;
    }
    let Some(entered) = entered else {
        return;
    };

    for (entity, mut node, display_in_state, prior_display) in q_display_in_state.iter_mut() {
        if &display_in_state.0 == entered {
            if let Some(display) = prior_display {
                node.display = display.0;
            } else {
                node.display = Display::Flex;
            }
        } else if node.display != Display::None {
            commands.entity(entity).insert(PriorDisplay(node.display));
            node.display = Display::None;
        }
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
            --
            @import_beatmap()
            --
            @reload_beatmaps()
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
