use bevy::prelude::*;
use crate::{
    components::{DriveState, Player},
    resources::{Countdown, GameState, RaceProgress, RaceTimer, TrackPath},
    systems::{player::DriveSystems, ship::HOVER_HEIGHT},
};

/// Half-height of the finish trigger volume around the ship's hover height.
/// Stacked sections are ≥ 6.5 m apart (see the track headroom test), so 3 m
/// accepts any ride-height wobble but rejects the road above or below.
const FINISH_VERTICAL_TOLERANCE: f32 = 3.0;

/// The finish line as a bounded, oriented trigger: a rectangle in the plane
/// through the start sample, facing the racing direction.
pub struct FinishGate {
    origin: Vec3,
    forward: Vec3,
    side: Vec3,
    up: Vec3,
    half_width: f32,
    half_height: f32,
}

impl FinishGate {
    pub fn from_track(path: &TrackPath) -> Self {
        let frame = path.frames[0];
        Self { origin: frame.position + frame.normal * HOVER_HEIGHT, forward: frame.tangent, side: frame.side, up: frame.normal, half_width: path.width * 0.5, half_height: FINISH_VERTICAL_TOLERANCE }
    }

    /// True only if the segment `previous → current` passes through the gate
    /// rectangle going forwards: behind the plane before, on or ahead of it
    /// after, and the crossing point lies within the road width and height band.
    pub fn crossed(&self, previous: Vec3, current: Vec3) -> bool {
        let (before, after) = (previous - self.origin, current - self.origin);
        let (d0, d1) = (before.dot(self.forward), after.dot(self.forward));
        if !(d0 < 0.0 && d1 >= 0.0) { return false; }
        let hit = before.lerp(after, d0 / (d0 - d1));
        hit.dot(self.side).abs() <= self.half_width && hit.dot(self.up).abs() <= self.half_height
    }
}

pub struct RacePlugin;
impl Plugin for RacePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Countdown), start_countdown)
            .add_systems(Update, countdown.run_if(in_state(GameState::Countdown)))
            .add_systems(OnEnter(GameState::Racing), start_race)
            .add_systems(Update, race_clock.run_if(in_state(GameState::Racing)))
            // Runs after movement so it sees this frame's previous → current step exactly once.
            .add_systems(Update, finish_detection.after(DriveSystems).run_if(in_state(GameState::Racing)));
    }
}

pub fn leave_loading(mut next: ResMut<NextState<GameState>>) { next.set(GameState::Countdown); }

fn start_countdown(mut countdown: ResMut<Countdown>, mut progress: ResMut<RaceProgress>) {
    countdown.timer.reset();
    countdown.step = 3;
    *progress = RaceProgress::default();
}

fn countdown(time: Res<Time>, mut countdown: ResMut<Countdown>, mut next: ResMut<NextState<GameState>>) {
    countdown.timer.tick(time.delta());
    if countdown.timer.just_finished() { countdown.step -= 1; if countdown.step < 0 { next.set(GameState::Racing); } }
}

fn start_race(mut timer: ResMut<RaceTimer>) { timer.elapsed = 0.0; timer.running = true; }

fn race_clock(time: Res<Time>, mut timer: ResMut<RaceTimer>) { if timer.running { timer.elapsed += time.delta_secs(); } }

fn finish_detection(path: Res<TrackPath>, mut progress: ResMut<RaceProgress>, mut timer: ResMut<RaceTimer>, player: Query<(&Transform, &DriveState), With<Player>>, mut next: ResMut<NextState<GameState>>) {
    let Ok((transform, drive)) = player.single() else { return };
    let Some(index) = drive.track_index else { return };
    if progress.finished { return; }
    progress.track(path.frames.len(), index);
    if progress.all_checkpoints_passed() && FinishGate::from_track(&path).crossed(drive.previous_position, transform.translation) {
        progress.finished = true;
        timer.running = false;
        next.set(GameState::Finished);
    }
}

pub fn restart_input(keys: Res<ButtonInput<KeyCode>>, state: Res<State<GameState>>, mut next: ResMut<NextState<GameState>>) {
    if keys.just_pressed(KeyCode::KeyR) && *state.get() == GameState::Finished { next.set(GameState::Countdown); }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{resources::CHECKPOINT_COUNT, systems::player::{advance, start_pose}};

    fn gate_and_path() -> (FinishGate, TrackPath) {
        let path = TrackPath::default();
        (FinishGate::from_track(&path), path)
    }

    /// A point on the start straight at `along` metres from the line.
    fn on_start(path: &TrackPath, along: f32, lateral: f32, height: f32) -> Vec3 {
        let f = path.frames[0];
        f.position + f.tangent * along + f.side * lateral + f.normal * (HOVER_HEIGHT + height)
    }

    #[test]
    fn forward_crossing_on_the_road_counts() {
        let (gate, path) = gate_and_path();
        assert!(gate.crossed(on_start(&path, -0.5, 0.0, 0.0), on_start(&path, 0.5, 0.0, 0.0)));
        assert!(gate.crossed(on_start(&path, -0.5, 11.0, 0.3), on_start(&path, 0.5, 11.0, 0.3)), "near the rail still counts");
    }

    #[test]
    fn wrong_direction_does_not_count() {
        let (gate, path) = gate_and_path();
        assert!(!gate.crossed(on_start(&path, 0.5, 0.0, 0.0), on_start(&path, -0.5, 0.0, 0.0)));
    }

    #[test]
    fn approaching_without_crossing_does_not_count() {
        let (gate, path) = gate_and_path();
        assert!(!gate.crossed(on_start(&path, -2.0, 0.0, 0.0), on_start(&path, -0.01, 0.0, 0.0)));
        assert!(!gate.crossed(on_start(&path, 0.2, 0.0, 0.0), on_start(&path, 1.0, 0.0, 0.0)), "already past the line");
    }

    #[test]
    fn crossing_outside_the_road_or_height_band_does_not_count() {
        let (gate, path) = gate_and_path();
        assert!(!gate.crossed(on_start(&path, -0.5, 16.0, 0.0), on_start(&path, 0.5, 16.0, 0.0)), "beside the road");
        assert!(!gate.crossed(on_start(&path, -0.5, 0.0, -6.5), on_start(&path, 0.5, 0.0, -6.5)), "under the line");
        assert!(!gate.crossed(on_start(&path, -0.5, 0.0, 6.5), on_start(&path, 0.5, 0.0, 6.5)), "over the line");
    }

    #[test]
    fn checkpoints_only_count_in_order_and_forwards() {
        let len = 1000;
        let mut progress = RaceProgress::default();
        for index in (0..len).step_by(5) { progress.track(len, index); }
        assert_eq!(progress.checkpoints_passed, CHECKPOINT_COUNT);

        // Reversing from the start never passes checkpoint 4 "backwards".
        let mut progress = RaceProgress::default();
        for index in (0..200).map(|k| (len - k * 2) % len) { progress.track(len, index); }
        assert_eq!(progress.checkpoints_passed, 0);
    }

    /// The reported bug: on the section under the start, the ship drifts
    /// forward across the start plane (e.g. changing lanes on the wide road).
    /// The old rule counted height difference as "lateral" distance, so it fired.
    #[test]
    fn passing_under_the_start_does_not_finish() {
        let (gate, path) = gate_and_path();
        let start = path.frames[0];
        let below = path.frames.iter().find(|f| {
            let d = f.position - start.position;
            Vec3::new(d.x, 0.0, d.z).length() < path.width * 0.5 && d.y < -4.0
        }).expect("a section passes under the start");
        // Same ride height as the ship on that lower road, stepping forward across the start plane.
        let level = below.position + below.normal * HOVER_HEIGHT;
        let on_plane = level - start.tangent * (level - start.position).dot(start.tangent);
        let (previous, current) = (on_plane - start.tangent * 0.5, on_plane + start.tangent * 0.5);

        let legacy = {
            let (before, after) = ((previous - start.position).dot(start.tangent), (current - start.position).dot(start.tangent));
            let lateral = (current - start.position - start.tangent * after).length();
            before <= 0.0 && after > 0.0 && lateral < path.width * 0.8
        };
        assert!(legacy, "old rule should reproduce the false finish");
        assert!(!gate.crossed(previous, current), "new gate must ignore the road below");
    }

    /// Full headless lap with the real movement code; the lap passes under the
    /// start area and must finish only at the real line.
    #[test]
    fn lap_finishes_only_at_the_real_finish_line() {
        let (gate, path) = gate_and_path();
        let count = path.frames.len();
        let (start, rotation) = start_pose(&path);
        let mut transform = Transform { translation: start, rotation, ..default() };
        let mut drive = DriveState { previous_position: start, track_index: Some(0), ..default() };
        let mut progress = RaceProgress::default();
        let start_pos = path.frames[0].position;
        let (mut went_under_start, mut travelled) = (false, 0usize);
        for step in 0..60 * 150 {
            let index = drive.track_index.unwrap();
            let target = path.frames[(index + 25) % count].position;
            let to_target = (target - transform.translation).reject_from_normalized(*transform.up()).normalize_or_zero();
            let side = to_target.dot(*transform.right());
            let steer = if side > 0.05 { 1.0 } else if side < -0.05 { -1.0 } else { 0.0 };
            let throttle = if to_target.dot(*transform.forward()) < 0.8 && drive.speed > 25.0 { -1.0 } else { 1.0 };
            advance(&path, &mut transform, &mut drive, throttle, steer, 1.0 / 60.0);
            let new_index = drive.track_index.unwrap();
            travelled += (new_index + count - index) % count;
            progress.track(count, new_index);

            let offset = transform.translation - start_pos;
            if Vec3::new(offset.x, 0.0, offset.z).length() < path.width && offset.y < -4.0 { went_under_start = true; }

            if progress.all_checkpoints_passed() && gate.crossed(drive.previous_position, transform.translation) {
                assert!(travelled >= count - 5, "finished early at step {step}, after {travelled} of {count} samples");
                assert!(went_under_start, "the lap should pass under the start area");
                return;
            }
            assert!(travelled < count + 50, "drove past the finish line without it registering");
        }
        panic!("lap not completed; travelled {travelled} of {count}");
    }
}
