#import bevy_pbr::{
    mesh_view_bindings::{view,globals},
    forward_io::VertexOutput,
    utils::coords_to_viewport_uv,
    view_transformations::{uv_to_ndc, frag_coord_to_ndc}
}
#import bevy_render::view::direction_view_to_world;
#import bevy_render::maths

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> color: vec4<f32>;

// (We assume the `VertexOutput` struct is defined and received as `in`)
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // 1. Prepare Your Input Data
    // The data arriving in the `in` struct has been smoothly interpolated
    // across the triangle's surface by the rasterizer. However, this
    // interpolation process means that a normalized vector (like our normal)
    // is no longer guaranteed to have a length of 1.0.
    // For any calculation that relies on direction, especially lighting,
    // the first step is ALWAYS to re-normalize the normal vector.
    let normal = normalize(in.world_normal);

    let viewport_uv = uv_to_ndc(coords_to_viewport_uv(in.position.xy, view.viewport));

    // let look_vector = direction_view_to_world(vec3<f32>(0.0,0.0,1.0), view.world_from_view);
    let look_vector = direction_view_to_world(vec3<f32>(0.0, 0.0, 1.0), view.world_from_view);
    let obj_vector = view.world_position.xyz - in.world_position.xyz;
    let intent = max(0.0, 1.0 - dot(normal, normalize(obj_vector)));
    let rimlight = pow(intent, 9.0);

    let dist = distance(viewport_uv * normalize(view.viewport.zw), vec2(0.0, 0.0));
    let angle = acos(dot(normalize(look_vector), normalize(obj_vector)));
    // var flash = vec4<f32>(0.0);
    // if dist < 0.1 {
    //   flash = vec4(sin(floor(2. - 2. * log(10. * dist)) + 10. *globals.time), 0.0, 0.0,0.0);
    // }

    // 3. Return the Final Color
    // The final output of a fragment shader must be a `vec4<f32>` representing
    // an RGBA color. We construct our output by taking our calculated RGB `vec3`
    // and adding a fixed alpha component of 1.0 for full opacity.
    // return vec4<f32>(color, 1.0) 
    return (vec4<f32>(0.0, 0.0, 0.0, 1.0) + rimlight * color);
}

