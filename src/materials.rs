use bevy::prelude::*;
pub fn ground_material(asset_server: Res<AssetServer>) -> impl Into<StandardMaterial> {
    let basepath = "textures/Ground080_1K-PNG/Ground080_1K-PNG_";
    let res_path = |s| format!("{}{}.png", basepath, s);
    StandardMaterial {
        base_color_texture: Some(asset_server.load(res_path("Color"))),
        normal_map_texture: Some(asset_server.load(res_path("NormalGL"))),
        metallic_roughness_texture: Some(asset_server.load(res_path("Roughness"))),
        metallic: 1.,
        perceptual_roughness: 1.,
        ..default()
    }

    // "Ground080.png"
    // "Ground080_1K-PNG.blend"
    // "Ground080_1K-PNG.mtlx"
    // "Ground080_1K-PNG.tres"
    // "Ground080_1K-PNG.usdc"
    // "Ground080_1K-PNG_Displacement.png"
    // "Ground080_1K-PNG_NormalDX.png"
    // "Ground080_1K-PNG_NormalGL.png"
    // "Ground080_1K-PNG_Roughness.png"
}
