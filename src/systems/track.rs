use bevy::{math::primitives::Cuboid, prelude::*};

use crate::{components::{FinishLine, TrackMarker}, resources::TrackPath};

pub struct TrackPlugin;

impl Plugin for TrackPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TrackPath::default()).add_systems(Startup, spawn_track);
    }
}

fn spawn_track(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let path = TrackPath::default();
    let road = materials.add(Color::srgb(0.055, 0.035, 0.12));
    let cyan = materials.add(StandardMaterial { base_color: Color::srgb(0.02, 0.8, 1.0), emissive: LinearRgba::new(0.0, 1.0, 1.0, 1.0), ..default() });
    let pink = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.03, 0.45), emissive: LinearRgba::new(1.0, 0.0, 0.25, 1.0), ..default() });
    let smooth_points = path.smooth_points();
    for i in 0..smooth_points.len() {
        let a = smooth_points[i];
        let b = smooth_points[(i + 1) % smooth_points.len()];
        spawn_road_segment(&mut commands, &mut meshes, road.clone(), cyan.clone(), pink.clone(), a, b, path.width);
    }

    let line_material = materials.add(StandardMaterial { base_color: Color::WHITE, emissive: LinearRgba::new(1.0, 0.3, 0.8, 1.0), ..default() });
    let start = path.points[0];
    let tangent = (path.points[1] - start).normalize();
    let gate_rotation = Quat::from_rotation_arc(Vec3::Z, tangent);
    let line_mesh = meshes.add(Cuboid::new(path.width, 0.4, 0.7));
    commands.spawn((Mesh3d(line_mesh), MeshMaterial3d(line_material), Transform { translation: start + Vec3::Y * 0.25, rotation: gate_rotation, ..default() }, FinishLine));

    // Small start markers make the gate legible from the chase camera.
    let marker = meshes.add(Cuboid::new(0.35, 4.0, 0.35));
    for side in [-1.0, 1.0] {
        commands.spawn((Mesh3d(marker.clone()), MeshMaterial3d(cyan.clone()), Transform { translation: start + tangent.cross(Vec3::Y).normalize() * side * path.width * 0.5 + Vec3::Y * 2.0, rotation: gate_rotation, ..default() }, TrackMarker));
    }
}

fn spawn_road_segment(commands: &mut Commands, meshes: &mut Assets<Mesh>, road: Handle<StandardMaterial>, cyan: Handle<StandardMaterial>, pink: Handle<StandardMaterial>, a: Vec3, b: Vec3, width: f32) {
    let delta = b - a;
    let length = delta.length();
    let tangent = delta / length;
    let rotation = Quat::from_rotation_arc(Vec3::Z, tangent);
    let center = (a + b) * 0.5;
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(width, 0.35, length + 0.5))), MeshMaterial3d(road), Transform { translation: center, rotation, ..default() }, TrackMarker));
    let side = tangent.cross(Vec3::Y).normalize();
    for (offset, material) in [(-1.0, cyan.clone()), (1.0, pink)] {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.22, 0.55, length + 0.8))), MeshMaterial3d(material), Transform { translation: center + side * offset * width * 0.5 + Vec3::Y * 0.35, rotation, ..default() }, TrackMarker));
    }
    // Repeated overhead frames make the high sections read as bridges/tunnels.
    if center.y > 8.0 {
        for t in [-0.35, 0.35] {
            commands.spawn((Mesh3d(meshes.add(Cuboid::new(width + 1.0, 0.25, 0.25))), MeshMaterial3d(cyan.clone()), Transform { translation: center + tangent * t * length + Vec3::Y * 4.0, rotation, ..default() }, TrackMarker));
        }
    }
}
