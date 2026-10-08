use bevy::prelude::*;

#[derive(Component)]
pub struct Player;

#[derive(Component, Default)]
pub struct CarBody;

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
}

impl Default for DriveState {
    fn default() -> Self {
        Self { speed: 0.0, previous_position: Vec3::ZERO }
    }
}
