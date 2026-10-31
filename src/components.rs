use bevy::prelude::*;

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct FollowCamera;

#[derive(Component)]
pub struct TrackMarker;

#[derive(Component)]
pub struct FinishLine;

#[derive(Component)]
pub struct HudRoot;

#[derive(Component)]
pub struct CountdownText;

#[derive(Component)]
pub struct TimerText;

#[derive(Component)]
pub struct SpeedText;

#[derive(Component)]
pub struct ResultPanel;

#[derive(Component)]
pub struct ResultText;

#[derive(Component)]
pub struct DriveState {
    pub speed: f32,
    pub previous_position: Vec3,
    /// Last track sample the car was projected onto; keeps the search local.
    pub track_index: Option<usize>,
    /// Smoothed steering input in `-1..=1` (positive = right).
    pub steer: f32,
    /// Raw throttle input: 1 accelerate, -1 brake/reverse, 0 coast.
    pub throttle: f32,
}

impl Default for DriveState {
    fn default() -> Self {
        Self { speed: 0.0, previous_position: Vec3::ZERO, track_index: None, steer: 0.0, throttle: 0.0 }
    }
}
