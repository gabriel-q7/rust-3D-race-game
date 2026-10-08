mod components;
mod game;
mod resources;
mod systems;
mod world;

use bevy::prelude::*;
use game::GamePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Neon Horizon // Vaporwave Racer".into(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(GamePlugin)
        .run();
}
