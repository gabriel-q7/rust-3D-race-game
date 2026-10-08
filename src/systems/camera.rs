use bevy::prelude::*;
use crate::components::{FollowCamera, Player};

pub struct CameraPlugin;
impl Plugin for CameraPlugin { fn build(&self, app: &mut App) { app.add_systems(Startup, spawn_camera).add_systems(Update, follow_player); } }

fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera3d::default(), Transform::from_xyz(-34.0, 8.0, -34.0).looking_at(Vec3::ZERO, Vec3::Y), FollowCamera));
}

fn follow_player(time: Res<Time>, player: Query<&GlobalTransform, With<Player>>, mut camera: Query<&mut Transform, (With<FollowCamera>, Without<Player>)>) {
    let (Ok(player), Ok(mut camera)) = (player.single(), camera.single_mut()) else { return };
    let target = player.translation() - player.forward() * 9.0 + Vec3::Y * 5.5;
    camera.translation = camera.translation.lerp(target, (time.delta_secs() * 5.0).min(1.0));
    camera.look_at(player.translation() + player.forward() * 5.0 + Vec3::Y * 0.8, Vec3::Y);
}
