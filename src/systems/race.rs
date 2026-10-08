use bevy::prelude::*;
use crate::{components::{DriveState, Player}, resources::{Countdown, GameState, RaceProgress, RaceTimer}};

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

fn finish_detection(time: Res<Time>, mut progress: ResMut<RaceProgress>, mut timer: ResMut<RaceTimer>, player: Query<(&Transform, &DriveState), With<Player>>, mut next: ResMut<NextState<GameState>>) {
    let Ok((transform, drive)) = player.single() else { return };
    if transform.translation.x > 18.0 { progress.armed = true; }
    let crossed_gate = drive.previous_position.x < 0.0 && transform.translation.x >= 0.0 && transform.translation.z < -17.0;
    if progress.armed && crossed_gate && drive.speed > 0.0 { timer.running = false; next.set(GameState::Finished); }
    let _ = time;
}

pub fn restart_input(keys: Res<ButtonInput<KeyCode>>, state: Res<State<GameState>>, mut next: ResMut<NextState<GameState>>) {
    if keys.just_pressed(KeyCode::KeyR) && *state.get() == GameState::Finished { next.set(GameState::Countdown); }
}
