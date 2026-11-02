//! Opt-in performance diagnostics: run with `VAPORWAVE_PERF=1` to log FPS,
//! frame time and entity count every two seconds, plus a one-off inventory of
//! what the scene contains. Costs nothing when the variable is unset.

use std::time::Duration;

use bevy::{
    diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    light::NotShadowCaster,
    prelude::*,
};

pub struct PerfPlugin;

impl Plugin for PerfPlugin {
    fn build(&self, app: &mut App) {
        if std::env::var_os("VAPORWAVE_PERF").is_none() { return; }
        app.add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            EntityCountDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin { wait_duration: Duration::from_secs(2), ..default() },
            bevy::render::diagnostic::RenderDiagnosticsPlugin,
        ))
        .add_systems(PostStartup, uncap_frame_rate)
        // A few frames in, once per-entity flags (e.g. NotShadowCaster) are assigned.
        .add_systems(Update, log_inventory.run_if(|mut frame: Local<u32>| { *frame += 1; *frame == 10 }));
    }
}

/// Measures real per-frame cost: with vsync on, frame time is pinned to the
/// display and an idle GPU may down-clock, hiding (or inverting) differences.
fn uncap_frame_rate(mut windows: Query<&mut Window>) {
    for mut window in &mut windows {
        window.present_mode = bevy::window::PresentMode::AutoNoVsync;
    }
}

fn log_inventory(
    entities: Query<()>,
    meshes_in_scene: Query<(), With<Mesh3d>>,
    shadow_casters: Query<(), (With<Mesh3d>, Without<NotShadowCaster>)>,
    point_lights: Query<&PointLight>,
    directional: Query<&DirectionalLight>,
    meshes: Res<Assets<Mesh>>,
    materials: Res<Assets<StandardMaterial>>,
) {
    info!(
        "scene inventory: {} entities, {} mesh entities ({} cast shadows), {} point lights ({} with shadows), {} directional ({} with shadows), {} mesh assets, {} standard materials",
        entities.iter().count(),
        meshes_in_scene.iter().count(),
        shadow_casters.iter().count(),
        point_lights.iter().count(),
        point_lights.iter().filter(|l| l.shadow_maps_enabled).count(),
        directional.iter().count(),
        directional.iter().filter(|l| l.shadow_maps_enabled).count(),
        meshes.len(),
        materials.len(),
    );
}
