#import bevy_ui::ui_vertex_output::UiVertexOutput

// struct UiVertexOutput {
//     @location(0) uv: vec2<f32>,
//     /// The size of the borders in UV space. Order is Left, Right, Top, Bottom.
//     @location(1) border_widths: vec4<f32>,
//     /// The border radius x value in pixels. Order is top left, top right, bottom right, bottom left.
//     @location(2) border_radius_x: vec4<f32>,
//     /// The border radius y value in pixels. Order is top left, top right, bottom right, bottom left.
//     @location(3) border_radius_y: vec4<f32>,
//     /// The size of the node in pixels. Order is width, height.
//     @location(4) @interpolate(flat) size: vec2<f32>,
//     @builtin(position) position: vec4<f32>,
// };
//

const TAU: f32 = 6.283185307;
const PI: f32 = 3.1415927;
@group(1) @binding(0) var<uniform> color: vec4<f32>;
@group(1) @binding(1) var<uniform> gap: f32;
@group(1) @binding(2) var<uniform> dot_radius: f32;
@group(1) @binding(3) var<uniform> hair_rotation: f32;
@group(1) @binding(4) var<uniform> hair_length: f32;
@group(1) @binding(5) var<uniform> hair_width: f32;
@group(1) @binding(6) var<uniform> num_hairs: u32;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {


  let crosshair_position = vec2f(0.5,0.5);

  var point = ((in.uv.xy*in.size.xy) - (crosshair_position * in.size.xy));

  let dot = sdfCircle(point, dot_radius);
  var signed_distance: f32 = 0.0;
  signed_distance = dot;

  // Hairs
  let halfwidth = vec2f(hair_length, hair_width);
  for (var i: u32 = 0; i < num_hairs;i++) {
    let angle = (6.283185307 / f32(num_hairs)) * f32(i) + hair_rotation;
    let hair = sdfBox(rotate2D(point, angle) + vec2(0.,gap), halfwidth);
    signed_distance = min(signed_distance, hair);
  }
  // Could add outline or smoothstep this
  let alpha = 1. - smoothstep(0., 1., signed_distance);

  let frag_color = vec3f(color.xyz * alpha);

  return vec4<f32>(frag_color,alpha);
}

fn sdfCircle(p: vec2<f32>, r: f32) -> f32 {
  return length(p) - r;
}

fn sdfBox(p: vec2<f32>, b: vec2<f32>) -> f32 {
  let d = abs(p) - b;
  return length(max(d, vec2(0.0))) + min(max(d.x, d.y), 0.0);
}

fn sdfToSmoothSolid(signedDistance: f32, threshold: f32, smoothing: f32) -> f32 {
  return 1.0 - smoothstep(threshold - smoothing, threshold + smoothing, signedDistance);
}

fn rotate2D(v: vec2<f32>, angle: f32) -> vec2<f32> {
  let c = cos(angle);
  let s = sin(angle);
  return vec2(v.x * c - v.y * s, v.x * s + v.y * c);
}
