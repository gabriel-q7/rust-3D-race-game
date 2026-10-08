use bevy::{math::primitives::{Cuboid, Cylinder}, prelude::*};

use crate::{components::{CarBody, DriveState, Player}, resources::{GameState, TrackBounds}};

const START: Vec3 = Vec3::new(-28.0, 0.65, -24.5);
const MAX_SPEED: f32 = 24.0;
const REVERSE_SPEED: f32 = 8.0;

pub struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player)
            .add_systems(Update, drive_player.run_if(in_state(GameState::Racing)))
            .add_systems(OnEnter(GameState::Countdown), reset_player);
    }
}

fn spawn_player(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let body = materials.add(Color::srgb(0.25, 0.04, 0.55));
    let glass = materials.add(Color::srgb(0.03, 0.4, 0.65));
    let neon = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.05, 0.5), emissive: LinearRgba::new(1.0, 0.0, 0.3, 1.0), ..default() });
    let wheel = materials.add(Color::srgb(0.015, 0.01, 0.03));
    let entity = commands.spawn((
        Player, CarBody, DriveState { speed: 0.0, previous_position: START }, Transform::from_translation(START), GlobalTransform::default(),
    )).id();
    commands.entity(entity).with_children(|parent| {
        parent.spawn((Mesh3d(meshes.add(Cuboid::new(3.2, 0.65, 1.7))), MeshMaterial3d(body.clone()), Transform::from_xyz(0.0, 0.0, 0.0)));
        parent.spawn((Mesh3d(meshes.add(Cuboid::new(1.45, 0.6, 1.3))), MeshMaterial3d(glass), Transform::from_xyz(-0.15, 0.55, 0.0)));
        parent.spawn((Mesh3d(meshes.add(Cuboid::new(3.0, 0.1, 1.8))), MeshMaterial3d(neon.clone()), Transform::from_xyz(0.0, -0.3, 0.0)));
        for x in [-1.05, 1.05] { for z in [-0.88, 0.88] {
            parent.spawn((Mesh3d(meshes.add(Cylinder::new(0.38, 0.22))), MeshMaterial3d(wheel.clone()), Transform { translation: Vec3::new(x, -0.35, z), rotation: Quat::from_rotation_x(std::f32::consts::FRAC_PI_2), ..default() }));
        }}
    });
}

fn reset_player(mut query: Query<(&mut Transform, &mut DriveState), With<Player>>) {
    if let Ok((mut transform, mut drive)) = query.single_mut() { transform.translation = START; transform.rotation = Quat::IDENTITY; drive.speed = 0.0; drive.previous_position = START; }
}

fn drive_player(time: Res<Time>, keys: Res<ButtonInput<KeyCode>>, bounds: Res<TrackBounds>, mut query: Query<(&mut Transform, &mut DriveState), With<Player>>) {
    let Ok((mut transform, mut drive)) = query.single_mut() else { return };
    let throttle = if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) { 1.0 } else if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) { -1.0 } else { 0.0 };
    let steering = (keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight)) as i8 - (keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft)) as i8;
    let dt = time.delta_secs();
    if throttle > 0.0 { drive.speed = (drive.speed + 18.0 * dt).min(MAX_SPEED); }
    else if throttle < 0.0 { drive.speed = (drive.speed - 22.0 * dt).max(-REVERSE_SPEED); }
    else { drive.speed = move_towards(drive.speed, 0.0, 10.0 * dt); }
    let steering_strength = (drive.speed.abs() / MAX_SPEED).clamp(0.15, 1.0);
    transform.rotate_y(-(steering as f32) * 1.8 * steering_strength * dt * drive.speed.signum());
    drive.previous_position = transform.translation;
    let forward = *transform.forward();
    transform.translation += forward * drive.speed * dt;

    let p = transform.translation;
    let in_outer = p.x.abs() <= bounds.outer_half_x && p.z.abs() <= bounds.outer_half_z;
    let in_inner = p.x.abs() < bounds.inner_half_x && p.z.abs() < bounds.inner_half_z;
    if !in_outer || in_inner {
        transform.translation = drive.previous_position;
        drive.speed *= -0.15;
    }
}

fn move_towards(value: f32, target: f32, amount: f32) -> f32 {
    if (value - target).abs() <= amount { target } else { value + (target - value).signum() * amount }
}
