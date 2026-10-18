// Infinite neon grid floor. The plane follows the camera, but lines are computed
// in world space so they stay still while driving. Distance fades into the
// horizon colour to hide the plane's edge and blend into the sky.
#import bevy_pbr::{forward_io::VertexOutput, mesh_view_bindings::view}

struct Ground {
    base: vec4<f32>,
    line: vec4<f32>,
    horizon: vec4<f32>,
    // x: cell size, y: line width in pixels, z: fade start, w: fade end.
    params: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> ground: Ground;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let world = in.world_position.xz;
    let coord = world / ground.params.x;

    // Anti-aliased lines: distance to the nearest line measured in screen pixels.
    let derivative = fwidth(coord);
    let grid = abs(fract(coord - 0.5) - 0.5) / max(derivative, vec2<f32>(1e-4));
    let line = 1.0 - clamp(min(grid.x, grid.y) / ground.params.y, 0.0, 1.0);
    // Once cells shrink below a few pixels the lines would shimmer; fade them out.
    let lod = 1.0 - clamp(max(derivative.x, derivative.y) * 4.0 - 0.5, 0.0, 1.0);

    var color = mix(ground.base.rgb, ground.line.rgb, line * lod);

    let distance = length(world - view.world_position.xz);
    let fade = smoothstep(ground.params.z, ground.params.w, distance);
    color = mix(color, ground.horizon.rgb, fade);

    return vec4<f32>(color, 1.0);
}
