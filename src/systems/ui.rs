use bevy::{color::palettes::css::*, prelude::*};
use crate::{components::{CountdownText, HudRoot, ResultPanel, ResultText, SpeedText, TimerText}, resources::{Countdown, GameState, RaceTimer}};

pub struct UiPlugin;
impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_ui)
            .add_systems(Update, (update_countdown, update_hud, update_result));
    }
}

fn text_style(size: f32, color: Color) -> (TextFont, TextColor) { (TextFont { font_size: size, ..default() }, TextColor(color)) }

fn spawn_ui(mut commands: Commands) {
    let root = commands.spawn((Node { width: Val::Percent(100.0), height: Val::Percent(100.0), position_type: PositionType::Absolute, ..default() }, HudRoot)).id();
    commands.entity(root).with_children(|parent| {
        parent.spawn((Text::new("TIME 00:00.00"), text_style(28.0, Color::srgb(0.4, 1.0, 1.0)), Node { position_type: PositionType::Absolute, top: Val::Px(24.0), left: Val::Px(32.0), ..default() }, TimerText));
        parent.spawn((Text::new("SPEED 000"), text_style(28.0, Color::srgb(1.0, 0.35, 0.8)), Node { position_type: PositionType::Absolute, top: Val::Px(24.0), right: Val::Px(32.0), ..default() }, SpeedText));
        parent.spawn((Text::new("3"), text_style(96.0, Color::srgb(1.0, 0.2, 0.8)), Node { position_type: PositionType::Absolute, top: Val::Percent(38.0), left: Val::Percent(48.0), ..default() }, CountdownText));
        parent.spawn((Node { width: Val::Percent(100.0), height: Val::Percent(100.0), position_type: PositionType::Absolute, display: Display::None, ..default() }, ResultPanel)).with_children(|panel| {
            panel.spawn((Text::new("RACE COMPLETE\n\nTIME 00:00.00\n\nPress R to restart"), text_style(40.0, Color::srgb(1.0, 0.4, 0.85)), Node { position_type: PositionType::Absolute, top: Val::Percent(28.0), left: Val::Percent(33.0), ..default() }, ResultText));
        });
    });
}

fn update_countdown(state: Res<State<GameState>>, countdown: Res<Countdown>, mut query: Query<&mut Text, With<CountdownText>>) {
    let Ok(mut text) = query.single_mut() else { return };
    text.0 = if *state.get() == GameState::Countdown { if countdown.step > 0 { countdown.step.to_string() } else { "GO!".into() } } else { String::new() };
}

fn update_hud(state: Res<State<GameState>>, timer: Res<RaceTimer>, mut timer_text: Query<&mut Text, With<TimerText>>, mut speed_text: Query<&mut Text, With<SpeedText>>) {
    if let Ok(mut text) = timer_text.single_mut() { text.0 = format!("TIME {}", format_time(timer.elapsed)); }
    if let Ok(mut text) = speed_text.single_mut() { text.0 = if *state.get() == GameState::Racing { "SPEED 000".into() } else { String::new() }; }
}

fn update_result(state: Res<State<GameState>>, timer: Res<RaceTimer>, mut panel: Query<&mut Node, With<ResultPanel>>, mut result: Query<&mut Text, With<ResultText>>) {
    let Ok(mut node) = panel.single_mut() else { return };
    node.display = if *state.get() == GameState::Finished { Display::Flex } else { Display::None };
    if let Ok(mut text) = result.single_mut() { text.0 = format!("RACE COMPLETE\n\nTIME {}\n\nPress R to restart", format_time(timer.elapsed)); }
}

fn format_time(seconds: f32) -> String { format!("{:02}:{:05.2}", (seconds as u32) / 60, seconds % 60.0) }
