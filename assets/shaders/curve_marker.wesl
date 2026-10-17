import bevy_pbr::render::{
    mesh_view_bindings::{view,globals},
    forward_io::VertexOutput,
    utils::coords_to_viewport_uv,
    view_transformations::{uv_to_ndc, frag_coord_to_ndc}
};
import bevy_render::view::direction_view_to_world;
import bevy_render::maths;

const TAU: f32 = 6.28318530718;




@group(constants::MATERIAL_BIND_GROUP) @binding(0) var<uniform> before_color: vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1) var<uniform> t0: f32;
@group(constants::MATERIAL_BIND_GROUP) @binding(2) var<uniform> slides: u32;
@group(constants::MATERIAL_BIND_GROUP) @binding(3) var<uniform> after_color: vec4<f32>;

// (We assume the `VertexOutput` struct is defined and received as `in`)
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    

    let slide = t0 * f32(slides);
    let t = fract(slide);
    let even = (u32(slide + 1) & u32(1));
    

    if step(in.uv.y, abs(f32(even) - t)) != f32(even) {
      return after_color;
    } else {
      return before_color;
    }

}
