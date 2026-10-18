use bevy::{math::primitives::{Cuboid, Cylinder, Sphere}, prelude::*};
use crate::resources::TrackPath;

pub struct EnvironmentPlugin;
impl Plugin for EnvironmentPlugin { fn build(&self, app: &mut App) { app.add_systems(Startup, spawn_environment); } }

fn spawn_environment(mut commands: Commands, path: Res<TrackPath>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    commands.spawn((DirectionalLight { illuminance: 7000.0, color: Color::srgb(0.35, 0.2, 0.7), shadow_maps_enabled: true, ..default() }, Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, -0.8, 0.0))));
    commands.insert_resource(GlobalAmbientLight { color: Color::srgb(0.18, 0.06, 0.25), brightness: 500.0, ..default() });
    // Ground, grid and sun live in the backdrop module.
    let cyan = materials.add(StandardMaterial { base_color: Color::srgb(0.02, 0.6, 0.8), emissive: LinearRgba::new(0.0, 0.5, 0.8, 1.0), ..default() });
    let purple = materials.add(Color::srgb(0.14, 0.02, 0.28));
    let pink = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.03, 0.45), emissive: LinearRgba::new(1.0, 0.0, 0.25, 1.0), ..default() });
    for z in [-70.0, -55.0, 55.0, 70.0] { for x in [-70.0, -55.0, 55.0, 70.0] {
        spawn_pillar(&mut commands, &mut meshes, cyan.clone(), Vec3::new(x, 3.0, z));
    }}
    for (x, z, h) in [(-78.0, 10.0, 8.0), (65.0, -8.0, 12.0), (-60.0, -45.0, 16.0), (60.0, 45.0, 10.0)] {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(8.0, h, 8.0))), MeshMaterial3d(purple.clone()), Transform::from_xyz(x, h / 2.0, z)));
    }
    // Arches sit on the generated road, evenly spread around the loop.
    for frame in path.frames.iter().skip(path.frames.len() / 8).step_by(path.frames.len() / 4) {
        spawn_arch(&mut commands, &mut meshes, cyan.clone(), frame.position, frame.tangent, path.width);
    }
    for (x, y, z) in [(-76.0, 22.0, 32.0), (70.0, 18.0, -25.0), (20.0, 26.0, 58.0), (-20.0, 29.0, 38.0)] {
        commands.spawn((Mesh3d(meshes.add(Sphere::new(2.0).mesh().uv(8, 5))), MeshMaterial3d(pink.clone()), Transform::from_xyz(x, y, z)));
    }
}

fn spawn_pillar(commands: &mut Commands, meshes: &mut Assets<Mesh>, material: Handle<StandardMaterial>, position: Vec3) { commands.spawn((Mesh3d(meshes.add(Cylinder::new(0.45, 6.0))), MeshMaterial3d(material), Transform::from_translation(position))); }

fn spawn_arch(commands: &mut Commands, meshes: &mut Assets<Mesh>, material: Handle<StandardMaterial>, position: Vec3, tangent: Vec3, width: f32) {
    let rotation = Quat::from_rotation_arc(Vec3::Z, tangent);
    let side = tangent.cross(Vec3::Y).normalize();
    for offset in [-1.0, 1.0] {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.3, 5.0, 0.3))), MeshMaterial3d(material.clone()), Transform { translation: position + side * offset * width * 0.45 + Vec3::Y * 2.5, rotation, ..default() }));
    }
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(width * 0.95, 0.3, 0.3))), MeshMaterial3d(material), Transform { translation: position + Vec3::Y * 5.0, rotation, ..default() }));
}
