#import bevy_pbr::{
    mesh_view_bindings::{view,globals},
    forward_io::VertexOutput,
    utils::coords_to_viewport_uv,
    view_transformations::{uv_to_ndc, frag_coord_to_ndc}
}
#import bevy_render::view::direction_view_to_world;
#import bevy_render::maths

const TAU: f32 = 6.28318530718;

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> color: vec4<f32>;

// (We assume the `VertexOutput` struct is defined and received as `in`)
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var final_color = vec4(in.world_normal.xy,0.,0.);
    return final_color;
}

