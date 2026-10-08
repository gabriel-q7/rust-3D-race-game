use bevy::{math::primitives::{Cuboid, Cylinder}, prelude::*};

use crate::{components::{CarBody, DriveState, Player, Wheel}, resources::{GameState, TrackPath}};

// Car convention: width X, height Y, length Z, visual front is local -Z.
pub const CAR_LENGTH: f32 = 3.6;
pub const CAR_WIDTH: f32 = 1.8;
pub const WHEEL_RADIUS: f32 = 0.42;
pub const WHEEL_X: f32 = 1.08;
pub const WHEEL_Z: f32 = 1.12;
pub const MAX_STEERING_ANGLE: f32 = 0.48;
const MAX_SPEED: f32 = 24.0;
const REVERSE_SPEED: f32 = 8.0;

pub struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player)
            .add_systems(Update, (drive_player, animate_wheels).chain().run_if(in_state(GameState::Racing)))
            .add_systems(OnEnter(GameState::Countdown), (reset_player, reset_wheels));
    }
}

fn spawn_player(mut commands: Commands, path: Res<TrackPath>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let start = path.points[0] + Vec3::Y * 0.9;
    let heading = (path.points[1] - path.points[0]).normalize();
    let rotation = Quat::from_rotation_arc(Vec3::NEG_Z, heading);
    let body = materials.add(Color::srgb(0.25, 0.04, 0.55));
    let glass = materials.add(Color::srgb(0.03, 0.4, 0.65));
    let neon = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.05, 0.5), emissive: LinearRgba::new(1.0, 0.0, 0.3, 1.0), ..default() });
    let headlight = materials.add(StandardMaterial { base_color: Color::srgb(0.7, 0.95, 1.0), emissive: LinearRgba::new(0.3, 0.9, 1.0, 1.0), ..default() });
    let taillight = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.02, 0.03), emissive: LinearRgba::new(1.0, 0.0, 0.0, 1.0), ..default() });
    let wheel = materials.add(Color::srgb(0.015, 0.01, 0.03));
    let entity = commands.spawn((Player, CarBody, DriveState { speed: 0.0, previous_position: start }, Transform { translation: start, rotation, ..default() }, GlobalTransform::default())).id();
    commands.entity(entity).with_children(|parent| {
        parent.spawn((Mesh3d(meshes.add(Cuboid::new(CAR_WIDTH, 0.65, CAR_LENGTH))), MeshMaterial3d(body), Transform::from_xyz(0.0, 0.0, 0.0)));
        parent.spawn((Mesh3d(meshes.add(Cuboid::new(1.35, 0.62, 1.45))), MeshMaterial3d(glass), Transform::from_xyz(0.0, 0.52, 0.15)));
        parent.spawn((Mesh3d(meshes.add(Cuboid::new(CAR_WIDTH + 0.12, 0.1, CAR_LENGTH + 0.1))), MeshMaterial3d(neon), Transform::from_xyz(0.0, -0.33, 0.0)));
        for x in [-0.58, 0.58] {
            parent.spawn((Mesh3d(meshes.add(Cuboid::new(0.28, 0.16, 0.12))), MeshMaterial3d(headlight.clone()), Transform::from_xyz(x, 0.02, -CAR_LENGTH * 0.5 - 0.02)));
            parent.spawn((Mesh3d(meshes.add(Cuboid::new(0.28, 0.16, 0.12))), MeshMaterial3d(taillight.clone()), Transform::from_xyz(x, 0.02, CAR_LENGTH * 0.5 + 0.02)));
        }
        for (front, z) in [(true, -WHEEL_Z), (false, WHEEL_Z)] {
            for x in [-WHEEL_X, WHEEL_X] {
                parent.spawn((Wheel { front, spin: 0.0 }, Mesh3d(meshes.add(Cylinder::new(WHEEL_RADIUS, 0.28))), MeshMaterial3d(wheel.clone()), Transform { translation: Vec3::new(x, -0.43, z), rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2), ..default() }));
            }
        }
    });
}

fn reset_player(path: Res<TrackPath>, mut query: Query<(&mut Transform, &mut DriveState), With<Player>>) {
    if let Ok((mut transform, mut drive)) = query.single_mut() {
        let start = path.points[0] + Vec3::Y * 0.9;
        transform.translation = start;
        transform.rotation = Quat::from_rotation_arc(Vec3::NEG_Z, (path.points[1] - path.points[0]).normalize());
        drive.speed = 0.0;
        drive.previous_position = start;
    }
}

fn drive_player(time: Res<Time>, keys: Res<ButtonInput<KeyCode>>, path: Res<TrackPath>, mut query: Query<(&mut Transform, &mut DriveState), With<Player>>) {
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
    keep_car_on_track(&path, &mut transform);
}

fn keep_car_on_track(path: &TrackPath, transform: &mut Transform) {
    let (center, tangent) = path.closest_point(transform.translation);
    let horizontal_tangent = Vec3::new(tangent.x, 0.0, tangent.z).normalize_or_zero();
    if horizontal_tangent == Vec3::ZERO { return; }
    let side = Vec3::new(-horizontal_tangent.z, 0.0, horizontal_tangent.x);
    let lateral = (transform.translation - center).dot(side);
    let safe_half_width = path.width * 0.5 - CAR_WIDTH * 0.55;
    let clamped_lateral = lateral.clamp(-safe_half_width, safe_half_width);
    transform.translation = center + side * clamped_lateral + Vec3::Y * 0.9;
}

fn animate_wheels(time: Res<Time>, keys: Res<ButtonInput<KeyCode>>, player: Query<(&DriveState, &Transform), With<Player>>, mut wheels: Query<(&mut Transform, &mut Wheel), Without<Player>>) {
    let Ok((drive, car)) = player.single() else { return };
    let steering = (keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight)) as i8 - (keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft)) as i8;
    let steer_angle = -(steering as f32) * MAX_STEERING_ANGLE;
    let signed_distance = (car.translation - drive.previous_position).dot(*car.forward());
    for (mut transform, mut wheel) in &mut wheels {
        wheel.spin += signed_distance / WHEEL_RADIUS;
        let steer = if wheel.front { Quat::from_rotation_y(steer_angle) } else { Quat::IDENTITY };
        transform.rotation = steer * Quat::from_rotation_z(std::f32::consts::FRAC_PI_2) * Quat::from_rotation_x(wheel.spin);
    }
    let _ = time;
}

fn reset_wheels(mut wheels: Query<(&mut Transform, &mut Wheel)>) {
    for (mut transform, mut wheel) in &mut wheels {
        wheel.spin = 0.0;
        transform.rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
    }
}

fn move_towards(value: f32, target: f32, amount: f32) -> f32 {
    if (value - target).abs() <= amount { target } else { value + (target - value).signum() * amount }
}
