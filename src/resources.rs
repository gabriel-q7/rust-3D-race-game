use bevy::prelude::*;

#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
pub enum GameState {
    #[default]
    Loading,
    Countdown,
    Racing,
    Finished,
}

#[derive(Resource)]
pub struct Countdown {
    pub timer: Timer,
    pub step: i8,
}

impl Default for Countdown {
    fn default() -> Self {
        Self { timer: Timer::from_seconds(1.0, TimerMode::Repeating), step: 3 }
    }
}

#[derive(Resource, Default)]
pub struct RaceTimer {
    pub elapsed: f32,
    pub running: bool,
}

#[derive(Resource, Default)]
pub struct RaceProgress {
    pub armed: bool,
}

#[derive(Resource, Clone, Copy)]
pub struct TrackBounds {
    pub outer_half_x: f32,
    pub outer_half_z: f32,
    pub inner_half_x: f32,
    pub inner_half_z: f32,
}

impl Default for TrackBounds {
    fn default() -> Self {
        Self { outer_half_x: 45.0, outer_half_z: 32.0, inner_half_x: 30.0, inner_half_z: 17.0 }
    }
}
