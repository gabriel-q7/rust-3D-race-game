use bevy::{math::primitives::{Cuboid, Cylinder, Sphere}, prelude::*};
use crate::resources::TrackPath;

pub struct EnvironmentPlugin;
impl Plugin for EnvironmentPlugin { fn build(&self, app: &mut App) { app.add_systems(Startup, spawn_environment); } }

fn spawn_environment(mut commands: Commands, path: Res<TrackPath>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    commands.spawn((DirectionalLight { illuminance: 7000.0, color: Color::srgb(0.35, 0.2, 0.7), shadow_maps_enabled: true, ..default() }, Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, -0.8, 0.0))));
    commands.insert_resource(GlobalAmbientLight { color: Color::srgb(0.18, 0.06, 0.25), brightness: 500.0, ..default() });
    let dark = materials.add(Color::srgb(0.015, 0.005, 0.04));
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(220.0, 0.2, 180.0))), MeshMaterial3d(dark), Transform::from_xyz(0.0, -0.35, 0.0)));
    let moon = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.18, 0.65), emissive: LinearRgba::new(1.0, 0.05, 0.4, 1.0), ..default() });
    commands.spawn((Mesh3d(meshes.add(Sphere::new(14.0).mesh().uv(32, 16))), MeshMaterial3d(moon), Transform::from_xyz(0.0, 22.0, 75.0)));
    let cyan = materials.add(StandardMaterial { base_color: Color::srgb(0.02, 0.6, 0.8), emissive: LinearRgba::new(0.0, 0.5, 0.8, 1.0), ..default() });
    let purple = materials.add(Color::srgb(0.14, 0.02, 0.28));
    let pink = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.03, 0.45), emissive: LinearRgba::new(1.0, 0.0, 0.25, 1.0), ..default() });
    for z in [-70.0, -55.0, 55.0, 70.0] { for x in [-70.0, -55.0, 55.0, 70.0] {
        spawn_pillar(&mut commands, &mut meshes, cyan.clone(), Vec3::new(x, 3.0, z));
    }}
    for (x, z, h) in [(-65.0, 10.0, 8.0), (65.0, -8.0, 12.0), (-60.0, -45.0, 16.0), (60.0, 45.0, 10.0)] {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(8.0, h, 8.0))), MeshMaterial3d(purple.clone()), Transform::from_xyz(x, h / 2.0, z)));
    }
    for (index, point) in path.points.iter().enumerate().filter(|(index, _)| *index % 5 == 2) {
        spawn_arch(&mut commands, &mut meshes, cyan.clone(), *point, (path.points[(index + 1) % path.points.len()] - *point).normalize(), path.width);
    }
    for (x, y, z) in [(-68.0, 14.0, 32.0), (70.0, 18.0, -25.0), (20.0, 26.0, 58.0), (-20.0, 21.0, 35.0)] {
        commands.spawn((Mesh3d(meshes.add(Sphere::new(2.0).mesh().uv(8, 5))), MeshMaterial3d(pink.clone()), Transform::from_xyz(x, y, z)));
    }
    // A sparse horizon grid gives depth without hundreds of geometry entities.
    for i in -8..=8 { let p = i as f32 * 10.0; spawn_grid_line(&mut commands, &mut meshes, cyan.clone(), Vec3::new(p, -0.2, 0.0), Vec3::new(0.08, 0.08, 180.0)); spawn_grid_line(&mut commands, &mut meshes, cyan.clone(), Vec3::new(0.0, -0.19, p), Vec3::new(180.0, 0.08, 0.08)); }
}

fn spawn_pillar(commands: &mut Commands, meshes: &mut Assets<Mesh>, material: Handle<StandardMaterial>, position: Vec3) { commands.spawn((Mesh3d(meshes.add(Cylinder::new(0.45, 6.0))), MeshMaterial3d(material), Transform::from_translation(position))); }
fn spawn_grid_line(commands: &mut Commands, meshes: &mut Assets<Mesh>, material: Handle<StandardMaterial>, position: Vec3, size: Vec3) { commands.spawn((Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))), MeshMaterial3d(material), Transform::from_translation(position))); }

fn spawn_arch(commands: &mut Commands, meshes: &mut Assets<Mesh>, material: Handle<StandardMaterial>, position: Vec3, tangent: Vec3, width: f32) {
    let rotation = Quat::from_rotation_arc(Vec3::Z, tangent);
    let side = tangent.cross(Vec3::Y).normalize();
    for offset in [-1.0, 1.0] {
        commands.spawn((Mesh3d(meshes.add(Cuboid::new(0.3, 5.0, 0.3))), MeshMaterial3d(material.clone()), Transform { translation: position + side * offset * width * 0.45 + Vec3::Y * 2.5, rotation, ..default() }));
    }
    commands.spawn((Mesh3d(meshes.add(Cuboid::new(width * 0.95, 0.3, 0.3))), MeshMaterial3d(material), Transform { translation: position + Vec3::Y * 5.0, rotation, ..default() }));
}
