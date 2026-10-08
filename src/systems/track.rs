use bevy::{color::palettes::css::*, math::primitives::Cuboid, prelude::*};

use crate::{components::{FinishLine, TrackMarker}, resources::TrackBounds};

pub struct TrackPlugin;

impl Plugin for TrackPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TrackBounds::default()).add_systems(Startup, spawn_track);
    }
}

fn spawn_track(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let bounds = TrackBounds::default();
    let road = materials.add(Color::srgb(0.055, 0.035, 0.12));
    let cyan = materials.add(StandardMaterial { base_color: Color::srgb(0.02, 0.8, 1.0), emissive: LinearRgba::new(0.0, 1.0, 1.0, 1.0), ..default() });
    let pink = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.03, 0.45), emissive: LinearRgba::new(1.0, 0.0, 0.25, 1.0), ..default() });
    let road_mesh = meshes.add(Cuboid::new(90.0, 0.25, 15.0));
    let side_mesh = meshes.add(Cuboid::new(15.0, 0.25, 64.0));
    let edge_mesh = meshes.add(Cuboid::new(90.0, 0.35, 0.22));
    let side_edge_mesh = meshes.add(Cuboid::new(0.22, 0.35, 64.0));

    for (mesh, position) in [
        (road_mesh.clone(), Vec3::new(0.0, 0.0, -24.5)),
        (road_mesh.clone(), Vec3::new(0.0, 0.0, 24.5)),
        (side_mesh.clone(), Vec3::new(-37.5, 0.0, 0.0)),
        (side_mesh, Vec3::new(37.5, 0.0, 0.0)),
    ] {
        commands.spawn((Mesh3d(mesh), MeshMaterial3d(road.clone()), Transform::from_translation(position), TrackMarker));
    }
    for (mesh, position, material) in [
        (edge_mesh.clone(), Vec3::new(0.0, 0.25, -32.0), cyan.clone()),
        (edge_mesh, Vec3::new(0.0, 0.25, 32.0), pink.clone()),
        (side_edge_mesh.clone(), Vec3::new(-45.0, 0.25, 0.0), pink.clone()),
        (side_edge_mesh, Vec3::new(45.0, 0.25, 0.0), cyan.clone()),
    ] {
        commands.spawn((Mesh3d(mesh), MeshMaterial3d(material), Transform::from_translation(position), TrackMarker));
    }

    let line_material = materials.add(StandardMaterial { base_color: Color::WHITE, emissive: LinearRgba::new(1.0, 0.3, 0.8, 1.0), ..default() });
    let line_mesh = meshes.add(Cuboid::new(0.5, 0.4, 15.0));
    commands.spawn((Mesh3d(line_mesh), MeshMaterial3d(line_material), Transform::from_xyz(0.0, 0.3, -24.5), FinishLine));

    // Small start markers make the gate legible from the chase camera.
    let marker = meshes.add(Cuboid::new(0.3, 3.0, 0.3));
    for x in [-7.5, 7.5] {
        commands.spawn((Mesh3d(marker.clone()), MeshMaterial3d(cyan.clone()), Transform::from_xyz(x, 1.5, -32.0), TrackMarker));
    }
    let _ = bounds;
}
