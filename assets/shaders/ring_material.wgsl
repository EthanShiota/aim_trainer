#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::{
    mesh_view_bindings::{view,globals},
}
#import bevy_render::view::direction_view_to_world;

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> color: vec4<f32>;

// =================================================================================
// Helper Functions
// =================================================================================

// A basic function to create turbulent distortion
fn turbulent_distortion(p: vec2<f32>, t: f32) -> f32 {
    // Use sine/cosine waves oscillating at different frequencies and phases
    let f1 = sin(p.x * 3.0 + t * 2.0) * 0.5;
    let f2 = cos(p.y * 2.5 - t * 1.5) * 0.5;
    let f3 = sin((p.x + p.y) * 1.5 + t * 3.0) * 0.5;
    let f4 = cos(p.x * 3. - t) * 0.5;

    // Combine them
    return (f1 + f2 + f3 + f4) * 0.5;
}

// Smoothstep implementation
fn smoothstep_custom(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

// =================================================================================
// Fragment Shader Entry Point
// =================================================================================

@fragment
fn fragement(in: VertexOutput) -> @location(0) vec4<f32> {

    let uv = ((in.uv / 2.) - 1.);
    let p = in.uv * view.viewport.zw;
    let t = globals.time;

    // 1. Generate Turbulence
    let noise_val = turbulent_distortion(p / 30., t);

    let normal = normalize(in.world_normal);

    let look_vector = direction_view_to_world(vec3<f32>(0.0, 0.0, 1.0), view.world_from_view);

    let obj_vector = view.world_position.xyz - in.world_position.xyz;

    let d = distance(0.5, abs(uv.y));
    // 2. Define Core and Edge (Fading out at the edges)
    // let intent = max(0.0, 1.0 - dot(normal, normalize(obj_vector)));
    let center_factor = 1.0 - smoothstep_custom(0.2, 0.4, d);

    let fire_intensity = noise_val * center_factor;

    // 3. Color Mapping (The Fire Ramp)
    let normalized_intensity = fire_intensity;

    // Mix 1: Dark/Smoke to Orange/Red
    let smoke_to_red = mix(vec4<f32>(0.0, 0.0, 0.0, 0.0), vec4<f32>(0.8, 0.2, 0.0, 1.0), smoothstep_custom(0.0, 0.5, normalized_intensity));

    // Mix 2: Orange/Red to Yellow/White (the core)
    var final_mix = mix(smoke_to_red, vec4<f32>(1.0, 0.6, 0.1, 1.0), // Bright Yellow/White
        smoothstep_custom(0.4, 1.0, normalized_intensity));

    let hard_edge = step(0.1, d);
    final_mix.a *= hard_edge;

    return final_mix;
}
