//! Reusable, purely cosmetic animation components.
//!
//! Every animation is a deterministic function of elapsed time plus a phase,
//! so there are no per-object timers and no per-frame allocations. Each
//! component captures its entity's starting pose the first time it runs, so
//! spawners only describe the motion, not the rest pose.
//!
//! Combining: `Floating` writes translation and `Rotating`/`Swaying` write
//! rotation, so `Floating` pairs with either. Don't put `Rotating` and
//! `Swaying` on the same entity.

use bevy::prelude::*;

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (rotate, float, sway, pulse_materials, pulse_lights));
    }
}

/// Continuous rotation around a local axis.
#[derive(Component)]
pub struct Rotating {
    pub axis: Vec3,
    /// Radians per second.
    pub speed: f32,
    pub phase: f32,
    base: Option<Quat>,
}

impl Rotating {
    pub fn new(axis: Vec3, speed: f32, phase: f32) -> Self {
        Self { axis: axis.normalize(), speed, phase, base: None }
    }
}

/// Smooth vertical sine bob around the starting position.
#[derive(Component)]
pub struct Floating {
    pub amplitude: f32,
    /// Cycles per second.
    pub frequency: f32,
    pub phase: f32,
    base: Option<Vec3>,
}

impl Floating {
    pub fn new(amplitude: f32, frequency: f32, phase: f32) -> Self {
        Self { amplitude, frequency, phase, base: None }
    }
}

/// Back-and-forth tilt around a local axis, e.g. palm trees in the wind.
#[derive(Component)]
pub struct Swaying {
    pub axis: Vec3,
    /// Maximum tilt in radians.
    pub amplitude: f32,
    pub frequency: f32,
    pub phase: f32,
    base: Option<Quat>,
}

impl Swaying {
    pub fn new(axis: Vec3, amplitude: f32, frequency: f32, phase: f32) -> Self {
        Self { axis: axis.normalize(), amplitude, frequency, phase, base: None }
    }
}

/// Emissive (or point-light) intensity that breathes around its starting value.
///
/// Applies to the entity's `StandardMaterial` and/or `PointLight`. Entities that
/// share one material share its pulse, so give an entity its own material (or
/// a per-phase material) when it needs an independent phase.
#[derive(Component)]
pub struct Pulsating {
    /// Relative swing, e.g. 0.4 = ±40 %.
    pub intensity: f32,
    pub frequency: f32,
    pub phase: f32,
    base_emissive: Option<LinearRgba>,
    base_light: Option<f32>,
}

impl Pulsating {
    pub fn new(intensity: f32, frequency: f32, phase: f32) -> Self {
        Self { intensity, frequency, phase, base_emissive: None, base_light: None }
    }

    /// Main sine plus a faster, smaller overtone for occasional irregular flickers.
    fn factor(&self, t: f32) -> f32 {
        let w = std::f32::consts::TAU * self.frequency;
        let wave = (w * t + self.phase).sin() + 0.35 * (w * 2.7 * t + self.phase * 1.9).sin();
        (1.0 + self.intensity * wave / 1.35).max(0.0)
    }
}

fn wave(t: f32, frequency: f32, phase: f32) -> f32 {
    (std::f32::consts::TAU * frequency * t + phase).sin()
}

fn rotate(time: Res<Time>, mut query: Query<(&mut Transform, &mut Rotating)>) {
    let t = time.elapsed_secs();
    for (mut transform, mut rotating) in &mut query {
        let base = *rotating.base.get_or_insert(transform.rotation);
        transform.rotation = base * Quat::from_axis_angle(rotating.axis, rotating.speed * t + rotating.phase);
    }
}

fn float(time: Res<Time>, mut query: Query<(&mut Transform, &mut Floating)>) {
    let t = time.elapsed_secs();
    for (mut transform, mut floating) in &mut query {
        let base = *floating.base.get_or_insert(transform.translation);
        transform.translation = base + Vec3::Y * floating.amplitude * wave(t, floating.frequency, floating.phase);
    }
}

fn sway(time: Res<Time>, mut query: Query<(&mut Transform, &mut Swaying)>) {
    let t = time.elapsed_secs();
    for (mut transform, mut swaying) in &mut query {
        let base = *swaying.base.get_or_insert(transform.rotation);
        transform.rotation = base * Quat::from_axis_angle(swaying.axis, swaying.amplitude * wave(t, swaying.frequency, swaying.phase));
    }
}

fn pulse_materials(time: Res<Time>, mut materials: ResMut<Assets<StandardMaterial>>, mut query: Query<(&MeshMaterial3d<StandardMaterial>, &mut Pulsating)>) {
    let t = time.elapsed_secs();
    // Capture every new base before writing anything, so entities sharing a
    // material all record its untouched emissive.
    for (handle, mut pulsating) in &mut query {
        if pulsating.base_emissive.is_none() {
            pulsating.base_emissive = materials.get(&handle.0).map(|material| material.emissive);
        }
    }
    for (handle, pulsating) in &query {
        let (Some(base), Some(mut material)) = (pulsating.base_emissive, materials.get_mut(&handle.0)) else { continue };
        material.emissive = base * pulsating.factor(t);
    }
}

fn pulse_lights(time: Res<Time>, mut query: Query<(&mut PointLight, &mut Pulsating)>) {
    let t = time.elapsed_secs();
    for (mut light, mut pulsating) in &mut query {
        let base = *pulsating.base_light.get_or_insert(light.intensity);
        light.intensity = base * pulsating.factor(t);
    }
}
