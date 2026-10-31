use bevy::prelude::*;

use crate::{
    components::{DriveState, Player},
    resources::{GameState, TrackPath},
    systems::ship::{self, EngineState, HOVER_HEIGHT, SHIP_WIDTH},
};

// Arcade tuning for the wide track. 1 unit ≈ 1 m, so 55 m/s ≈ 198 km/h.
pub const MAX_SPEED: f32 = 55.0;
const REVERSE_SPEED: f32 = 14.0;
const ACCELERATION: f32 = 26.0;
const BRAKING: f32 = 45.0;
/// Deceleration while coasting.
const DRAG: f32 = 12.0;
/// Yaw rate (rad/s) at full steering and speed; at top speed this gives a
/// turning radius of MAX_SPEED / STEER_RATE ≈ 32 m, inside the track's corners.
const STEER_RATE: f32 = 1.7;
/// Fraction of steering kept when nearly stopped, so the ship can still turn in place-ish.
const MIN_STEER_FACTOR: f32 = 0.25;
/// How fast steering input ramps (per second); smooths keyboard on/off.
const STEER_RESPONSE: f32 = 7.0;
/// Extra gap kept between the wingtips and the rails.
const RAIL_MARGIN: f32 = 0.4;

pub struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player)
            .add_systems(Update, drive_player.run_if(in_state(GameState::Racing)))
            .add_systems(OnEnter(GameState::Countdown), reset_player)
            .add_systems(OnExit(GameState::Racing), release_controls);
    }
}

fn spawn_player(mut commands: Commands, path: Res<TrackPath>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let (start, rotation) = start_pose(&path);
    let entity = commands
        .spawn((Player, DriveState { previous_position: start, track_index: Some(0), ..default() }, EngineState::default(), Transform { translation: start, rotation, ..default() }, Visibility::default()))
        .id();
    ship::spawn_ship(&mut commands, entity, &mut meshes, &mut materials);
}

fn reset_player(path: Res<TrackPath>, mut query: Query<(&mut Transform, &mut DriveState), With<Player>>) {
    if let Ok((mut transform, mut drive)) = query.single_mut() {
        let (start, rotation) = start_pose(&path);
        transform.translation = start;
        transform.rotation = rotation;
        *drive = DriveState { previous_position: start, track_index: Some(0), ..default() };
    }
}

/// Drops inputs when the race ends so engines idle and the ship levels out.
fn release_controls(mut query: Query<&mut DriveState, With<Player>>) {
    if let Ok(mut drive) = query.single_mut() {
        drive.throttle = 0.0;
        drive.steer = 0.0;
    }
}

fn start_pose(path: &TrackPath) -> (Vec3, Quat) {
    let frame = path.frames[0];
    (frame.position + frame.normal * HOVER_HEIGHT, Transform::IDENTITY.looking_to(frame.tangent, frame.normal).rotation)
}

fn drive_player(time: Res<Time>, keys: Res<ButtonInput<KeyCode>>, path: Res<TrackPath>, mut query: Query<(&mut Transform, &mut DriveState), With<Player>>) {
    let Ok((mut transform, mut drive)) = query.single_mut() else { return };
    let pressed = |a: KeyCode, b: KeyCode| keys.pressed(a) || keys.pressed(b);
    let throttle = if pressed(KeyCode::KeyW, KeyCode::ArrowUp) { 1.0 } else if pressed(KeyCode::KeyS, KeyCode::ArrowDown) { -1.0 } else { 0.0 };
    let steer_input = pressed(KeyCode::KeyD, KeyCode::ArrowRight) as i8 as f32 - pressed(KeyCode::KeyA, KeyCode::ArrowLeft) as i8 as f32;
    advance(&path, &mut transform, &mut drive, throttle, steer_input, time.delta_secs());
}

/// One movement step from raw inputs; pure so tests can drive the ship headless.
fn advance(path: &TrackPath, transform: &mut Transform, drive: &mut DriveState, throttle: f32, steer_input: f32, dt: f32) {
    drive.throttle = throttle;
    drive.steer = move_towards(drive.steer, steer_input, STEER_RESPONSE * dt);

    if throttle > 0.0 { drive.speed = (drive.speed + ACCELERATION * dt).min(MAX_SPEED); }
    else if throttle < 0.0 { drive.speed = (drive.speed - BRAKING * dt).max(-REVERSE_SPEED); }
    else { drive.speed = move_towards(drive.speed, 0.0, DRAG * dt); }

    let steering_strength = (drive.speed.abs() / MAX_SPEED).clamp(MIN_STEER_FACTOR, 1.0);
    // Steer around the ship's own up axis so turning stays correct on slopes.
    let up = transform.up();
    transform.rotate_axis(up, -drive.steer * STEER_RATE * steering_strength * dt * drive.speed.signum());
    drive.previous_position = transform.translation;
    let forward = *transform.forward();
    transform.translation += forward * drive.speed * dt;
    keep_ship_on_track(path, transform, drive);
}

/// Glues the ship to the road surface: clamps it between the rails, sets its
/// hover height from the spline and tilts it to follow slopes.
fn keep_ship_on_track(path: &TrackPath, transform: &mut Transform, drive: &mut DriveState) {
    let hit = path.closest(transform.translation, drive.track_index);
    drive.track_index = Some(hit.index);
    let frame = hit.frame;
    let lateral = (transform.translation - frame.position).dot(frame.side);
    let safe_half_width = path.width * 0.5 - SHIP_WIDTH * 0.5 - RAIL_MARGIN;
    let clamped_lateral = lateral.clamp(-safe_half_width, safe_half_width);
    transform.translation = frame.position + frame.side * clamped_lateral + frame.normal * HOVER_HEIGHT;
    let heading = transform.forward().reject_from_normalized(frame.normal).normalize_or(frame.tangent);
    transform.rotation = Transform::IDENTITY.looking_to(heading, frame.normal).rotation;
}

fn move_towards(value: f32, target: f32, amount: f32) -> f32 {
    if (value - target).abs() <= amount { target } else { value + (target - value).signum() * amount }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Headless lap with a simple autopilot that only uses the real controls
    /// (full throttle, digital steering, braking when badly misaligned).
    /// Proves the circuit is completable with the current tuning and width.
    #[test]
    fn autopilot_completes_a_lap() {
        let path = TrackPath::default();
        let count = path.frames.len();
        let (start, rotation) = start_pose(&path);
        let mut transform = Transform { translation: start, rotation, ..default() };
        let mut drive = DriveState { previous_position: start, track_index: Some(0), ..default() };
        let dt = 1.0 / 60.0;
        let mut progressed = 0usize;
        let mut last_index = 0usize;
        let mut rail_frames = 0usize;
        let safe_half_width = path.width * 0.5 - SHIP_WIDTH * 0.5 - RAIL_MARGIN;
        for step in 0..60 * 120 {
            let index = drive.track_index.unwrap();
            // Aim at a point ahead on the centerline.
            let target = path.frames[(index + 25) % count].position;
            let to_target = (target - transform.translation).reject_from_normalized(*transform.up()).normalize_or_zero();
            let side = to_target.dot(*transform.right());
            let alignment = to_target.dot(*transform.forward());
            let steer = if side > 0.05 { 1.0 } else if side < -0.05 { -1.0 } else { 0.0 };
            let throttle = if alignment < 0.8 && drive.speed > 25.0 { -1.0 } else { 1.0 };
            advance(&path, &mut transform, &mut drive, throttle, steer, dt);

            let new_index = drive.track_index.unwrap();
            let delta = (new_index + count - last_index) % count;
            assert!(delta < count / 2, "ship went backwards or jumped at step {step}: {last_index} -> {new_index}");
            progressed += delta;
            last_index = new_index;
            let lateral = (transform.translation - path.frames[new_index].position).dot(path.frames[new_index].side).abs();
            if lateral > safe_half_width - 0.05 { rail_frames += 1; }
            if progressed >= count {
                // Scraping the rails briefly is fine; riding them for most of the lap is not.
                assert!(rail_frames < step / 5, "ship rode the rails for {rail_frames} of {step} frames");
                println!("lap in {:.1}s, {rail_frames} frames on the rails", step as f32 * dt);
                return;
            }
        }
        panic!("lap not finished in 120 s; reached {progressed} of {count} samples");
    }
}
