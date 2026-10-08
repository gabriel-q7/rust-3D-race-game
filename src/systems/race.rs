use bevy::prelude::*;
use crate::{components::{DriveState, Player}, resources::{Countdown, GameState, RaceProgress, RaceTimer, TrackPath}};

pub struct RacePlugin;
impl Plugin for RacePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Countdown), start_countdown)
            .add_systems(Update, countdown.run_if(in_state(GameState::Countdown)))
            .add_systems(OnEnter(GameState::Racing), start_race)
            .add_systems(Update, race_clock.run_if(in_state(GameState::Racing)))
            .add_systems(Update, finish_detection.run_if(in_state(GameState::Racing)));
    }
}

pub fn leave_loading(mut next: ResMut<NextState<GameState>>) { next.set(GameState::Countdown); }

fn start_countdown(mut countdown: ResMut<Countdown>, mut progress: ResMut<RaceProgress>) { countdown.timer.reset(); countdown.step = 3; progress.armed = false; }

fn countdown(time: Res<Time>, mut countdown: ResMut<Countdown>, mut next: ResMut<NextState<GameState>>) {
    countdown.timer.tick(time.delta());
    if countdown.timer.just_finished() { countdown.step -= 1; if countdown.step < 0 { next.set(GameState::Racing); } }
}

fn start_race(mut timer: ResMut<RaceTimer>) { timer.elapsed = 0.0; timer.running = true; }

fn race_clock(time: Res<Time>, mut timer: ResMut<RaceTimer>) { if timer.running { timer.elapsed += time.delta_secs(); } }

fn finish_detection(time: Res<Time>, path: Res<TrackPath>, mut progress: ResMut<RaceProgress>, mut timer: ResMut<RaceTimer>, player: Query<(&Transform, &DriveState), With<Player>>, mut next: ResMut<NextState<GameState>>) {
    let Ok((transform, drive)) = player.single() else { return };
    let checkpoint = path.points[10];
    if transform.translation.distance(checkpoint) < path.width * 1.5 { progress.armed = true; }
    let start = path.points[0];
    let tangent = (path.points[1] - start).normalize();
    let previous_side = (drive.previous_position - start).dot(tangent);
    let current_side = (transform.translation - start).dot(tangent);
    let lateral = (transform.translation - start - tangent * current_side).length();
    let crossed_gate = previous_side <= 0.0 && current_side > 0.0 && lateral < path.width * 0.8;
    if progress.armed && crossed_gate && drive.speed > 0.0 { timer.running = false; next.set(GameState::Finished); }
    let _ = time;
}

pub fn restart_input(keys: Res<ButtonInput<KeyCode>>, state: Res<State<GameState>>, mut next: ResMut<NextState<GameState>>) {
    if keys.just_pressed(KeyCode::KeyR) && *state.get() == GameState::Finished { next.set(GameState::Countdown); }
}
