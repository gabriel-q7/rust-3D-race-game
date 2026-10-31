use bevy::{camera::Hdr, pbr::{DistanceFog, FogFalloff}, post_process::bloom::Bloom, prelude::*};
use crate::{components::{FollowCamera, Player}, world::backdrop::{FADE_END, FADE_START, HORIZON}};

pub const CAMERA_DISTANCE: f32 = 15.0;
pub const CAMERA_HEIGHT: f32 = 6.0;
/// Point the camera aims at, ahead of and slightly above the ship.
const LOOK_AHEAD: f32 = 10.0;
const LOOK_HEIGHT: f32 = 1.5;

pub struct CameraPlugin;
impl Plugin for CameraPlugin { fn build(&self, app: &mut App) { app.add_systems(Startup, spawn_camera).add_systems(Update, follow_player); } }

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        // HDR + bloom turn the emissive neon and the sun into real glow.
        Hdr,
        Bloom::NATURAL,
        // Same color and range as the ground fade, so props melt into the horizon.
        DistanceFog { color: HORIZON, falloff: FogFalloff::Linear { start: FADE_START, end: FADE_END }, ..default() },
        Transform::from_xyz(-100.0, 24.0, -100.0).looking_at(Vec3::ZERO, Vec3::Y),
        FollowCamera,
    ));
}

fn follow_player(time: Res<Time>, player: Query<&GlobalTransform, With<Player>>, mut camera: Query<&mut Transform, (With<FollowCamera>, Without<Player>)>) {
    let (Ok(player), Ok(mut camera)) = (player.single(), camera.single_mut()) else { return };
    let target = player.translation() - player.forward() * CAMERA_DISTANCE + Vec3::Y * CAMERA_HEIGHT;
    camera.translation = camera.translation.lerp(target, (time.delta_secs() * 5.0).min(1.0));
    camera.look_at(player.translation() + player.forward() * LOOK_AHEAD + Vec3::Y * LOOK_HEIGHT, Vec3::Y);
}
