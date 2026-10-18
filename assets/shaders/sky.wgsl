// Vaporwave sky dome: vertical gradient plus a striped retro sun.
// Drawn on a large sphere centred on the camera, so only the view direction matters.
#import bevy_pbr::{forward_io::VertexOutput, mesh_view_bindings::view}

struct Sky {
    horizon: vec4<f32>,
    zenith: vec4<f32>,
    sun_top: vec4<f32>,
    sun_bottom: vec4<f32>,
    // xyz: direction towards the sun, w: angular radius (radians).
    sun: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> sky: Sky;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let dir = normalize(in.world_position.xyz - view.world_position);
    let height = max(dir.y, 0.0);

    // Hot band at the horizon, quickly cooling into a deep night zenith.
    var color = mix(sky.horizon.rgb, sky.zenith.rgb, pow(height, 0.45));

    // Sun disc: angular distance from the sun direction.
    let sun_dir = normalize(sky.sun.xyz);
    let radius = sky.sun.w;
    let angle = acos(clamp(dot(dir, sun_dir), -1.0, 1.0));
    // Position inside the disc from top (1) to bottom (-1).
    let rel = (dir.y - sun_dir.y) / radius;
    let sun_color = mix(sky.sun_bottom.rgb, sky.sun_top.rgb, clamp(rel * 0.5 + 0.5, 0.0, 1.0));
    // Classic horizontal cut-outs that start above the middle and thicken downwards.
    let band = fract(rel * 6.0);
    let gap = clamp(0.4 - rel, 0.0, 1.4) * 0.45;
    let striped = select(1.0, 0.0, rel < 0.4 && band < gap);
    let edge = 1.0 - smoothstep(radius * 0.97, radius, angle);
    color = mix(color, sun_color, edge * striped);

    // Soft glow around the sun so bloom has something to pick up.
    let glow = exp(-max(angle - radius, 0.0) * 6.0) * 0.35;
    color += sky.sun_bottom.rgb * glow;

    return vec4<f32>(color, 1.0);
}
