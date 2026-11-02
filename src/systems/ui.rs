//! Racing HUD built with Bevy UI: translucent dark panels with neon borders.
//!
//! Layout: race status (top-left), timer (top-centre), speed with a bar
//! (bottom-right), a centred countdown panel that pops on every step, and a
//! centred result panel when the race is finished.
//!
//! Updates use `set_if_neq`, so text is only re-laid out when its value
//! actually changes (e.g. the speed digits), not every frame.

use bevy::prelude::*;

use crate::{
    components::{DriveState, Player},
    resources::{Countdown, GameState, RaceProgress, RaceTimer, TrackPath, CHECKPOINT_COUNT},
    systems::player::MAX_SPEED,
};

// --- Palette ---------------------------------------------------------------
/// Panel fill: deep purple, translucent so the track stays visible behind it.
const PANEL_BG: Color = Color::srgba(0.05, 0.01, 0.11, 0.62);
/// Result panel is more opaque; it is the focus once the race ends.
const RESULT_BG: Color = Color::srgba(0.04, 0.0, 0.09, 0.86);
/// Full-screen dim behind the result panel.
const SCRIM: Color = Color::srgba(0.0, 0.0, 0.0, 0.35);
const CYAN: Color = Color::srgb(0.35, 0.95, 1.0);
const MAGENTA: Color = Color::srgb(1.0, 0.3, 0.8);
const CYAN_BORDER: Color = Color::srgba(0.1, 0.9, 1.0, 0.75);
const MAGENTA_BORDER: Color = Color::srgba(1.0, 0.2, 0.75, 0.75);
const LABEL: Color = Color::srgba(0.78, 0.72, 0.95, 0.85);
const VALUE: Color = Color::srgb(0.96, 0.94, 1.0);
const BAR_TRACK: Color = Color::srgba(1.0, 1.0, 1.0, 0.08);

// --- Dimensions & spacing (logical px) ------------------------------------
const HUD_MARGIN: f32 = 24.0;
const PANEL_PADDING: UiRect = UiRect::axes(Val::Px(18.0), Val::Px(10.0));
const BORDER: f32 = 2.0;
/// Angular "cut" look: two opposite corners sharp, two rounded.
const PANEL_RADIUS: BorderRadius = BorderRadius::new(Val::Px(14.0), Val::Px(2.0), Val::Px(14.0), Val::Px(2.0));
const RACE_PANEL_WIDTH: f32 = 230.0;
const TIMER_PANEL_WIDTH: f32 = 240.0;
const SPEED_PANEL_WIDTH: f32 = 250.0;
const COUNTDOWN_PANEL_SIZE: f32 = 220.0;
const RESULT_PANEL_WIDTH: f32 = 520.0;
const SPEED_BAR_HEIGHT: f32 = 8.0;

// --- Typography -------------------------------------------------------------
const LABEL_SIZE: f32 = 14.0;
const BODY_SIZE: f32 = 20.0;
const TIMER_SIZE: f32 = 40.0;
const SPEED_SIZE: f32 = 56.0;
const COUNTDOWN_SIZE: f32 = 120.0;

pub struct UiPlugin;
impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_ui)
            .add_systems(Update, (update_race_panel, update_timer, update_speed, update_countdown, update_result));
    }
}

#[derive(Component)]
struct StatusText;
#[derive(Component)]
struct CheckpointText;
#[derive(Component)]
struct ProgressText;
#[derive(Component)]
struct TimerText;
#[derive(Component)]
struct SpeedText;
#[derive(Component)]
struct SpeedBarFill;
/// Panels that only make sense while driving; hidden on the result screen.
#[derive(Component)]
struct GameplayPanel;
#[derive(Component)]
struct CountdownOverlay;
#[derive(Component)]
struct CountdownPanel;
#[derive(Component)]
struct CountdownText;
#[derive(Component)]
struct ResultOverlay;
#[derive(Component)]
struct ResultTimeText;
#[derive(Component)]
struct RestartHint;

fn text(value: &str, size: f32, color: Color) -> (Text, TextFont, TextColor) {
    (Text::new(value), TextFont { font_size: FontSize::Px(size), ..default() }, TextColor(color))
}

/// Translucent neon-bordered panel laid out as a vertical stack.
fn panel(width: f32, border: Color, align: AlignItems) -> (Node, BackgroundColor, BorderColor) {
    (
        Node {
            width: Val::Px(width),
            flex_direction: FlexDirection::Column,
            align_items: align,
            row_gap: Val::Px(2.0),
            padding: PANEL_PADDING,
            border: UiRect::all(Val::Px(BORDER)),
            border_radius: PANEL_RADIUS,
            ..default()
        },
        BackgroundColor(PANEL_BG),
        BorderColor::all(border),
    )
}

/// Full-screen layer that centres its single child.
fn overlay() -> Node {
    Node { position_type: PositionType::Absolute, width: Val::Percent(100.0), height: Val::Percent(100.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, display: Display::None, ..default() }
}

fn spawn_ui(mut commands: Commands) {
    let root = commands.spawn(Node { width: Val::Percent(100.0), height: Val::Percent(100.0), position_type: PositionType::Absolute, ..default() }).id();

    // Top-left: race status and progress.
    let mut race_panel = panel(RACE_PANEL_WIDTH, CYAN_BORDER, AlignItems::Start);
    race_panel.0 = Node { position_type: PositionType::Absolute, top: Val::Px(HUD_MARGIN), left: Val::Px(HUD_MARGIN), ..race_panel.0 };
    let race = commands.spawn((race_panel, GameplayPanel, ChildOf(root))).id();
    commands.spawn((text("RACE", LABEL_SIZE, LABEL), ChildOf(race)));
    commands.spawn((text("READY", BODY_SIZE, CYAN), StatusText, ChildOf(race)));
    commands.spawn((text("CHECKPOINT 0/4", BODY_SIZE, VALUE), CheckpointText, ChildOf(race)));
    commands.spawn((text("TRACK 0%", BODY_SIZE, VALUE), ProgressText, ChildOf(race)));

    // Top-centre: the timer, the most important number.
    let top = commands.spawn((Node { position_type: PositionType::Absolute, top: Val::Px(HUD_MARGIN), width: Val::Percent(100.0), justify_content: JustifyContent::Center, ..default() }, ChildOf(root))).id();
    let timer = commands.spawn((panel(TIMER_PANEL_WIDTH, MAGENTA_BORDER, AlignItems::Center), ChildOf(top))).id();
    commands.spawn((text("TIME", LABEL_SIZE, LABEL), ChildOf(timer)));
    commands.spawn((text("00:00.00", TIMER_SIZE, VALUE), TimerText, ChildOf(timer)));

    // Bottom-right: speed, unit and a fill bar.
    let mut speed_panel = panel(SPEED_PANEL_WIDTH, CYAN_BORDER, AlignItems::End);
    speed_panel.0 = Node { position_type: PositionType::Absolute, bottom: Val::Px(HUD_MARGIN), right: Val::Px(HUD_MARGIN), ..speed_panel.0 };
    let speed = commands.spawn((speed_panel, GameplayPanel, ChildOf(root))).id();
    commands.spawn((text("SPEED", LABEL_SIZE, LABEL), ChildOf(speed)));
    let readout = commands.spawn((Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Baseline, column_gap: Val::Px(8.0), ..default() }, ChildOf(speed))).id();
    commands.spawn((text("000", SPEED_SIZE, VALUE), SpeedText, ChildOf(readout)));
    commands.spawn((text("KM/H", BODY_SIZE, MAGENTA), ChildOf(readout)));
    let bar = commands.spawn((Node { width: Val::Percent(100.0), height: Val::Px(SPEED_BAR_HEIGHT), margin: UiRect::top(Val::Px(6.0)), border_radius: BorderRadius::all(Val::Px(4.0)), ..default() }, BackgroundColor(BAR_TRACK), ChildOf(speed))).id();
    commands.spawn((Node { width: Val::Percent(0.0), height: Val::Percent(100.0), border_radius: BorderRadius::all(Val::Px(4.0)), ..default() }, BackgroundColor(CYAN), SpeedBarFill, ChildOf(bar)));

    // Centre: countdown panel that pops on each step.
    let countdown = commands.spawn((overlay(), CountdownOverlay, ChildOf(root))).id();
    let countdown_panel = commands
        .spawn((
            Node { width: Val::Px(COUNTDOWN_PANEL_SIZE), height: Val::Px(COUNTDOWN_PANEL_SIZE), flex_direction: FlexDirection::Column, justify_content: JustifyContent::Center, align_items: AlignItems::Center, border: UiRect::all(Val::Px(BORDER + 1.0)), border_radius: PANEL_RADIUS, ..default() },
            BackgroundColor(PANEL_BG),
            BorderColor::all(MAGENTA_BORDER),
            BoxShadow::new(MAGENTA.with_alpha(0.35), Val::ZERO, Val::ZERO, Val::Px(4.0), Val::Px(24.0)),
            UiTransform::default(),
            CountdownPanel,
            ChildOf(countdown),
        ))
        .id();
    commands.spawn((text("GET READY", LABEL_SIZE, LABEL), ChildOf(countdown_panel)));
    commands.spawn((text("3", COUNTDOWN_SIZE, MAGENTA), CountdownText, ChildOf(countdown_panel)));

    // Centre: result panel over a light scrim.
    let result = commands.spawn((overlay(), BackgroundColor(SCRIM), ResultOverlay, ChildOf(root))).id();
    let result_panel = commands
        .spawn((
            Node { width: Val::Px(RESULT_PANEL_WIDTH), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(10.0), padding: UiRect::axes(Val::Px(32.0), Val::Px(28.0)), border: UiRect::all(Val::Px(BORDER)), border_radius: PANEL_RADIUS, ..default() },
            BackgroundColor(RESULT_BG),
            BorderColor::all(CYAN_BORDER),
            BoxShadow::new(CYAN.with_alpha(0.3), Val::ZERO, Val::ZERO, Val::Px(4.0), Val::Px(30.0)),
            ChildOf(result),
        ))
        .id();
    commands.spawn((text("RACE COMPLETE", 44.0, MAGENTA), ChildOf(result_panel)));
    commands.spawn((Node { width: Val::Percent(80.0), height: Val::Px(2.0), margin: UiRect::vertical(Val::Px(4.0)), ..default() }, BackgroundColor(CYAN_BORDER), ChildOf(result_panel)));
    commands.spawn((text("FINAL TIME", LABEL_SIZE, LABEL), ChildOf(result_panel)));
    commands.spawn((text("00:00.00", SPEED_SIZE, CYAN), ResultTimeText, ChildOf(result_panel)));
    let hint = commands
        .spawn((
            Node { margin: UiRect::top(Val::Px(12.0)), padding: UiRect::axes(Val::Px(20.0), Val::Px(8.0)), border: UiRect::all(Val::Px(BORDER)), border_radius: BorderRadius::all(Val::Px(6.0)), ..default() },
            BackgroundColor(MAGENTA.with_alpha(0.15)),
            BorderColor::all(MAGENTA_BORDER),
            RestartHint,
            ChildOf(result_panel),
        ))
        .id();
    commands.spawn((text("PRESS  R  TO RESTART", BODY_SIZE, VALUE), ChildOf(hint)));
}

fn update_race_panel(
    state: Res<State<GameState>>,
    progress: Res<RaceProgress>,
    path: Res<TrackPath>,
    player: Query<&DriveState, With<Player>>,
    mut texts: ParamSet<(Query<(&mut Text, &mut TextColor), With<StatusText>>, Query<&mut Text, With<CheckpointText>>, Query<&mut Text, With<ProgressText>>)>,
) {
    let (status, color) = match state.get() {
        GameState::Loading | GameState::Countdown => ("GET READY", MAGENTA),
        GameState::Racing => ("RACING", CYAN),
        GameState::Finished => ("FINISHED", MAGENTA),
    };
    if let Ok((mut text, mut text_color)) = texts.p0().single_mut() {
        text.set_if_neq(Text::new(status));
        text_color.set_if_neq(TextColor(color));
    }
    if let Ok(mut text) = texts.p1().single_mut() {
        text.set_if_neq(Text(format!("CHECKPOINT {}/{CHECKPOINT_COUNT}", progress.checkpoints_passed)));
    }
    // Progress along the loop from the track sample the ship is on.
    let percent = player.single().ok().and_then(|drive| drive.track_index).map_or(0, |index| index * 100 / path.frames.len().max(1));
    if let Ok(mut text) = texts.p2().single_mut() {
        text.set_if_neq(Text(format!("TRACK {percent:>3}%")));
    }
}

fn update_timer(timer: Res<RaceTimer>, mut query: Query<&mut Text, With<TimerText>>) {
    if let Ok(mut text) = query.single_mut() {
        text.set_if_neq(Text(format_time(timer.elapsed)));
    }
}

fn update_speed(
    state: Res<State<GameState>>,
    player: Query<&DriveState, With<Player>>,
    mut speed_text: Query<&mut Text, With<SpeedText>>,
    mut bar: Query<(&mut Node, &mut BackgroundColor), With<SpeedBarFill>>,
    mut panels: Query<&mut Visibility, With<GameplayPanel>>,
) {
    let speed = player.single().map_or(0.0, |drive| drive.speed.abs());
    // Track units are metres, so m/s * 3.6 gives km/h.
    if let Ok(mut text) = speed_text.single_mut() {
        text.set_if_neq(Text(format!("{:03}", (speed * 3.6).round() as u32)));
    }
    let ratio = (speed / MAX_SPEED).clamp(0.0, 1.0);
    if let Ok((mut node, mut color)) = bar.single_mut() {
        let width = Val::Percent((ratio * 100.0).round());
        if node.width != width { node.width = width; }
        // Cyan when cruising, shifting to magenta near top speed.
        color.set_if_neq(BackgroundColor(CYAN.mix(&MAGENTA, ratio * ratio)));
    }
    let visible = if *state.get() == GameState::Finished { Visibility::Hidden } else { Visibility::Inherited };
    for mut visibility in &mut panels {
        visibility.set_if_neq(visible);
    }
}

fn update_countdown(
    state: Res<State<GameState>>,
    countdown: Res<Countdown>,
    mut overlay: Query<&mut Node, With<CountdownOverlay>>,
    mut panel: Query<&mut UiTransform, With<CountdownPanel>>,
    mut label: Query<(&mut Text, &mut TextColor), With<CountdownText>>,
) {
    let active = *state.get() == GameState::Countdown;
    if let Ok(mut node) = overlay.single_mut() {
        let display = if active { Display::Flex } else { Display::None };
        if node.display != display { node.display = display; }
    }
    if !active { return; }
    // Each step starts large and settles: an ease-out "pop".
    let settle = 1.0 - (1.0 - countdown.timer.fraction()).powi(3);
    if let Ok(mut transform) = panel.single_mut() {
        transform.scale = Vec2::splat(1.35 - 0.35 * settle.min(1.0));
    }
    if let Ok((mut text, mut color)) = label.single_mut() {
        let go = countdown.step <= 0;
        text.set_if_neq(Text(if go { "GO!".into() } else { countdown.step.to_string() }));
        color.set_if_neq(TextColor(if go { CYAN } else { MAGENTA }));
    }
}

fn update_result(
    time: Res<Time>,
    state: Res<State<GameState>>,
    timer: Res<RaceTimer>,
    mut overlay: Query<&mut Node, With<ResultOverlay>>,
    mut result: Query<&mut Text, With<ResultTimeText>>,
    mut hint: Query<&mut BackgroundColor, With<RestartHint>>,
) {
    let finished = *state.get() == GameState::Finished;
    if let Ok(mut node) = overlay.single_mut() {
        let display = if finished { Display::Flex } else { Display::None };
        if node.display != display { node.display = display; }
    }
    if !finished { return; }
    if let Ok(mut text) = result.single_mut() {
        text.set_if_neq(Text(format_time(timer.elapsed)));
    }
    // The restart chip breathes gently to draw the eye.
    if let Ok(mut color) = hint.single_mut() {
        color.0 = MAGENTA.with_alpha(0.12 + 0.12 * (time.elapsed_secs() * 3.0).sin().abs());
    }
}

fn format_time(seconds: f32) -> String { format!("{:02}:{:05.2}", (seconds as u32) / 60, seconds % 60.0) }
