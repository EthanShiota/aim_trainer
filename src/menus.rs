use std::time;

use bevy::{
    camera::{CameraOutputMode, visibility::RenderLayers},
    color::palettes::tailwind::*,
    prelude::*,
    render::render_resource::BlendState,
};
use bevy_egui::{
    egui::{Color32, Ui, UiBuilder},
    prelude::*,
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;

use crate::{AppState, GameSettings, scenarios};
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

fn settings_window(mut contexts: EguiContexts, mut settings: ResMut<GameSettings>) -> Result {
    egui::Window::new("Settings").show(contexts.ctx_mut()?, |ui| {
        ui.add(egui::Slider::new(&mut settings.volume, 0.0..=1.0).text("Volume"));
        ui.add(
            egui::Slider::new(&mut settings.mouse_sensitivity, 0.5..=30.0)
                .text("Mouse Sensitivity"),
        );
        ui.add(egui::Slider::new(&mut settings.dpi, 400..=3200).text("DPI"));
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

fn main_menu(
    mut contexts: EguiContexts,
    mut commands: Commands,
    mut beat_maps: Local<Vec<Vec<BeatMapOsu>>>,
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
    if beat_maps.is_empty() {
        let now = time::Instant::now();
        let mut count = 0;
        warn!("begin serialization");
        let beatmap_dir = "osu_beatmaps";
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
        warn!(
            "end serialization of {count} maps in {} secs",
            time::Instant::now().duration_since(now).as_secs_f64()
        );
    }

    egui::CentralPanel::default().show(&mut viewport_ui, |ui| {
        for versions in beat_maps.iter() {
            if !versions.is_empty() {
                egui::CollapsingHeader::new(&versions[0].metadata.title).show(ui, |ui| {
                    for beat_map in versions.iter() {
                        let button = ui.button((beat_map.metadata.version).to_string());
                        if button.clicked() {
                            commands.set_state(AppState::InGame);
                            commands.run_system_cached_with(scenarios::osu, beat_map.clone());
                        }
                        if button.secondary_clicked() {
                            commands.set_state(AppState::InGame);
                            commands
                                .run_system_cached_with(scenarios::debug_scene, beat_map.clone());
                        }
                    }
                });
            }
        }
    });

    Ok(())
}
