use bevy::prelude::*;

use crate::{resources::GameState, systems, world};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .init_resource::<crate::resources::RaceTimer>()
            .init_resource::<crate::resources::Countdown>()
            .init_resource::<crate::resources::RaceProgress>()
            .add_plugins((
                world::EnvironmentPlugin,
                systems::track::TrackPlugin,
                systems::player::PlayerPlugin,
                systems::camera::CameraPlugin,
                systems::race::RacePlugin,
                systems::ui::UiPlugin,
            ))
            .add_systems(Update, systems::race::leave_loading.run_if(in_state(GameState::Loading)))
            .add_systems(Update, systems::race::restart_input);
    }
}
