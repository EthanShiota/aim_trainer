use std::{
    error::Error,
    fs, io,
    path::{Path, PathBuf},
    time,
};

use bevy::{
    camera::{CameraOutputMode, visibility::RenderLayers},
    color::palettes::css,
    prelude::*,
    render::render_resource::BlendState,
    settings::SaveSettings,
    tasks::{Task, futures::check_ready},
};
use bevy_egui::{
    egui::{Color32, Ui, UiBuilder},
    prelude::*,
};
use rfd::FileDialog;
use zip::{ZipArchive, result::ZipError};

use crate::{AppState, GameSettings, GameState, scenarios};
use parser::BeatMapOsu;
pub struct MenuPlugin;
impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            EguiPrimaryContextPass,
            (main_menu, settings_window).run_if(in_state(AppState::Menu)),
        )
        .add_systems(Startup, setup);
    }
}

pub fn pause_menu() -> impl Scene {
    bsn! {
        Node {
            display: Display::Flex, justify_content: JustifyContent::Center, align_items: AlignItems::Center, width: percent(100.), height: percent(100.)
        }
        DespawnOnExit::<GameState>(GameState::Paused)
        Children [
            Node { flex_direction: FlexDirection::Column, width: percent(20.), height: percent(20.), align_items: AlignItems::Center, justify_content: JustifyContent::SpaceBetween, border: px(4.)}
            BorderColor::all(css::BLACK)
            Children [
                (
                    Node {width: percent(100.), height: percent(50.), align_items: AlignItems::Center, justify_content: JustifyContent::Center, justify_items: JustifyItems::Center}
                    BackgroundColor(css::BLACK)
                    on(|_: On<Pointer<Press>>, mut commands: Commands| {
                        commands.set_state(GameState::Playing);
                    })
                    Children [
                        Node {justify_content: JustifyContent::Center}
                        Text::new("Exit")
                    ]
                ),
                (
                    Node {width: percent(100.), height: percent(50.), align_items: AlignItems::Center, justify_content: JustifyContent::Center}
                    BackgroundColor(css::BLACK)
                    on(|_: On<Pointer<Press>>, mut commands: Commands| {
                        commands.set_state(AppState::Menu);
                    })
                    Children [
                        Node {justify_content: JustifyContent::Center}
                        Text::new("Main Menu")
                    ]
                )
            ]
        ]
    }
}

fn settings_window(
    mut contexts: EguiContexts,
    mut settings: If<ResMut<GameSettings>>,
    mut confirm: Local<bool>,
    mut commands: Commands,
) -> Result {
    egui::Window::new("Settings").show(contexts.ctx_mut()?, |ui| {
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
            ui.add(egui::Slider::new(&mut settings.volume, 0.0..=1.0).text("Volume"));
            ui.add(
                egui::Slider::new(&mut settings.mouse_sensitivity, 0.5..=30.0)
                    .text("Mouse Sensitivity"),
            );
            ui.add(egui::Slider::new(&mut settings.dpi, 400..=3200).text("DPI"));
            if ui.button("reset").clicked() {
                *confirm = true;
            }
        }
    });
    Ok(())
}

fn setup(mut commands: Commands, mut egui_global_settings: ResMut<EguiGlobalSettings>) {
    egui_global_settings.auto_create_primary_context = false;
    // Egui Camera
    commands.spawn((
        // The `PrimaryEguiContext` component requires everything needed to render a primary context.
        PrimaryEguiContext,
        Camera2d,
        // Setting RenderLayers to none makes sure we won't render anything apart from the UI.
        RenderLayers::none(),
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

fn color(c: Srgba) -> Color32 {
    let [r, g, b, a] = c.to_u8_array();
    Color32::from_rgba_unmultiplied_const(r, g, b, a)
}

#[derive(Component)]
struct SelectedFile(Task<Option<PathBuf>>);

fn main_menu(
    mut contexts: EguiContexts,
    mut commands: Commands,
    mut beat_maps: Local<Vec<Vec<BeatMapOsu>>>,
    mut selected_file: Query<(&mut SelectedFile, Entity)>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    // ctx.global_style_mut(|style| {
    //     style.visuals.extreme_bg_color = color(RED_700);
    // });
    let mut viewport_ui = Ui::new(
        ctx.clone(),
        "viewport".into(),
        UiBuilder::new().max_rect(ctx.viewport_rect()),
    );

    // let mut vis = viewport_ui.visuals().clone();
    // egui::Window::new("style").show(&mut viewport_ui, |ui| vis.ui(ui));
    // ctx.global_style_mut(|style| {
    //     style.visuals = vis;
    // });
    let beatmap_dir = "osu_beatmaps";
    if beat_maps.is_empty() {
        *beat_maps = serialize_beatmaps(beatmap_dir);
    }

    egui::CentralPanel::default().show_inside(&mut viewport_ui, |ui| {
        // Import new beatmap
        if ui.button("Import Beatmap").clicked() && selected_file.is_empty() {
            let thread_pool = bevy::tasks::AsyncComputeTaskPool::get();
            let task = thread_pool.spawn(async move { FileDialog::new().pick_file() });
            commands.spawn(SelectedFile(task));
        }

        for (mut file, entity) in selected_file.iter_mut() {
            if let Some(filepath) = check_ready(&mut file.0) {
                // Dialog can return None
                if let Some(path) = filepath {
                    debug!("path: {path:?}");
                    unzip_beatmap(&Path::new(beatmap_dir), &path).unwrap();
                    *beat_maps = serialize_beatmaps(beatmap_dir);
                }
                commands.entity(entity).remove::<SelectedFile>();
            }
        }
        // Beatmap select
        for versions in beat_maps.iter() {
            if !versions.is_empty() {
                egui::CollapsingHeader::new(&versions[0].metadata.title).show(ui, |ui| {
                    for beat_map in versions.iter() {
                        let button = ui.button((beat_map.metadata.version).to_string());
                        if button.clicked() {
                            commands.set_state(AppState::InGame);
                            commands.run_system_cached_with(scenarios::osu, beat_map.clone());
                        }
                    }
                });
            }
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
            info!("skipping file with invalid path: {}", file.name());
        }
    }
    Ok(())
}

// TODO: Remove unwraps
fn serialize_beatmaps(beatmap_dir: &str) -> Vec<Vec<BeatMapOsu>> {
    let now = time::Instant::now();
    let mut count = 0;
    debug!("begin serialization");
    let mut beat_maps = vec![];
    for file in std::fs::read_dir(beatmap_dir).unwrap().flat_map(|w| w.ok()) {
        let mut versions = vec![];
        for ent in file.path().read_dir().unwrap() {
            if let Some(map) = ent.ok().and_then(|dir| BeatMapOsu::new(dir.path()).ok()) {
                count += 1;
                versions.push(map);
            }
        }
        beat_maps.push(versions);
    }
    debug!(
        "end serialization of {count} maps in {} secs",
        time::Instant::now().duration_since(now).as_secs_f64()
    );
    beat_maps
}
