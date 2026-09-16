use bevy::{color::palettes::css::WHITE, prelude::*};
use bevy_egui::prelude::*;

use crate::{
    AudioBuffer, EditMode, PlayerCamera,
    target_plugin::{BeatMap, Target, TargetMarker},
};

pub struct EditPlugin;
impl Plugin for EditPlugin {
    fn build(&self, app: &mut App) {
        app
            //.add_systems(Update, edit_mode.run_if(in_state(EditMode::Editing)))
            .add_systems(
                EguiPrimaryContextPass,
                edit_mode_ui.run_if(in_state(EditMode::Editing)),
            );
    }
}

// fn edit_mode(
//     mut commands: Commands,
//     stopwatch: Res<SceneTimer>,
//     player_camera: Single<&Transform, With<PlayerCamera>>,
//     button_input: Res<ButtonInput<KeyCode>>,
//     mut meshes: ResMut<Assets<Mesh>>,
//     mut materials: ResMut<Assets<StandardMaterial>>,
// ) {
//     let mesh = meshes.add(Sphere::new(1.));
//     let mat = materials.add(StandardMaterial::from_color(WHITE));
//     if button_input.just_pressed(KeyCode::KeyV) {
//         let pos = player_camera
//             .with_translation(player_camera.translation + *player_camera.forward() * 40.);
//         commands.spawn((
//             TargetMarker(stopwatch.elapsed()),
//             pos,
//             Mesh3d(mesh.clone()),
//             MeshMaterial3d(mat.clone()),
//         ));
//     }
// }
fn edit_mode_ui(
    mut contexts: EguiContexts,
    _q_audio: Query<&AudioSink, With<AudioPlayer<AudioBuffer>>>,
    mut commands: Commands,
    spawned_targets: Query<(Entity, &Target)>,
) -> Result {
    egui::Window::new("edit").show(contexts.ctx_mut()?, |ui| {
        if ui.button("reset").clicked() {
            for (ent, _) in spawned_targets {
                commands.entity(ent).despawn();
            }
        }
    });
    Ok(())
}
