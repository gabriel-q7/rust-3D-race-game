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
/// Distance from the road surface to the car origin (wheels rest on the road).
const RIDE_HEIGHT: f32 = 0.9;

pub struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player)
            .add_systems(Update, (drive_player, animate_wheels).chain().run_if(in_state(GameState::Racing)))
            .add_systems(OnEnter(GameState::Countdown), (reset_player, reset_wheels));
    }
}

fn spawn_player(mut commands: Commands, path: Res<TrackPath>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let (start, rotation) = start_pose(&path);
    let body = materials.add(Color::srgb(0.25, 0.04, 0.55));
    let glass = materials.add(Color::srgb(0.03, 0.4, 0.65));
    let neon = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.05, 0.5), emissive: LinearRgba::new(1.0, 0.0, 0.3, 1.0), ..default() });
    let headlight = materials.add(StandardMaterial { base_color: Color::srgb(0.7, 0.95, 1.0), emissive: LinearRgba::new(0.3, 0.9, 1.0, 1.0), ..default() });
    let taillight = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.02, 0.03), emissive: LinearRgba::new(1.0, 0.0, 0.0, 1.0), ..default() });
    let wheel = materials.add(Color::srgb(0.015, 0.01, 0.03));
    let entity = commands.spawn((Player, CarBody, DriveState { speed: 0.0, previous_position: start, track_index: Some(0) }, Transform { translation: start, rotation, ..default() }, GlobalTransform::default())).id();
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
        let (start, rotation) = start_pose(&path);
        transform.translation = start;
        transform.rotation = rotation;
        drive.speed = 0.0;
        drive.previous_position = start;
        drive.track_index = Some(0);
    }
}

fn start_pose(path: &TrackPath) -> (Vec3, Quat) {
    let frame = path.frames[0];
    (frame.position + frame.normal * RIDE_HEIGHT, Transform::IDENTITY.looking_to(frame.tangent, frame.normal).rotation)
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
    // Steer around the car's own up axis so turning stays correct on slopes.
    let up = transform.up();
    transform.rotate_axis(up, -(steering as f32) * 1.8 * steering_strength * dt * drive.speed.signum());
    drive.previous_position = transform.translation;
    let forward = *transform.forward();
    transform.translation += forward * drive.speed * dt;
    keep_car_on_track(&path, &mut transform, &mut drive);
}

/// Glues the car to the road surface: clamps it between the rails, sets its
/// height from the spline and tilts it to follow slopes.
fn keep_car_on_track(path: &TrackPath, transform: &mut Transform, drive: &mut DriveState) {
    let hit = path.closest(transform.translation, drive.track_index);
    drive.track_index = Some(hit.index);
    let frame = hit.frame;
    let lateral = (transform.translation - frame.position).dot(frame.side);
    let safe_half_width = path.width * 0.5 - CAR_WIDTH * 0.55;
    let clamped_lateral = lateral.clamp(-safe_half_width, safe_half_width);
    transform.translation = frame.position + frame.side * clamped_lateral + frame.normal * RIDE_HEIGHT;
    let heading = transform.forward().reject_from_normalized(frame.normal).normalize_or(frame.tangent);
    transform.rotation = Transform::IDENTITY.looking_to(heading, frame.normal).rotation;
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
