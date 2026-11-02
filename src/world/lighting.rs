//! Localized trackside lighting. Lights are placed from the track data itself
//! (sharp turns, crossings, the start gate, arches, gantries), so they follow
//! any layout change. Counts are capped per category to keep the clustered
//! forward renderer cheap; no light casts shadows.

use bevy::{light::NotShadowCaster, math::primitives::{Cuboid, Cylinder, Sphere}, prelude::*};

use crate::{
    resources::{TrackFrame, TrackPath},
    systems::track::gantry_frames,
    world::{animation::Pulsating, environment::arch_frames},
};

/// Color, intensity (lumens) and range (m) shared by every light of one kind.
#[derive(Clone, Copy)]
pub struct LightPreset {
    pub color: Color,
    pub intensity: f32,
    pub range: f32,
}

impl LightPreset {
    pub fn light(self) -> PointLight {
        PointLight { color: self.color, intensity: self.intensity, range: self.range, radius: 0.6, shadow_maps_enabled: false, ..default() }
    }

    /// HDR emissive for the fixture that visibly "is" this light.
    fn emissive(self) -> LinearRgba {
        LinearRgba::from(self.color) * 4.0
    }
}

pub const CYAN: LightPreset = LightPreset { color: Color::srgb(0.1, 0.9, 1.0), intensity: 4_500_000.0, range: 44.0 };
pub const MAGENTA: LightPreset = LightPreset { color: Color::srgb(1.0, 0.1, 0.75), intensity: 4_500_000.0, range: 44.0 };
pub const PURPLE: LightPreset = LightPreset { color: Color::srgb(0.6, 0.2, 1.0), intensity: 3_000_000.0, range: 32.0 };
pub const PINK: LightPreset = LightPreset { color: Color::srgb(1.0, 0.35, 0.6), intensity: 3_500_000.0, range: 36.0 };

/// Caps per placement category.
const MAX_TURN_LIGHTS: usize = 8;
const MAX_CROSSING_LIGHTS: usize = 5;
const MAX_GANTRY_LIGHTS: usize = 4;
/// Minimum distance along/between lights of the same category, in metres.
const TURN_SPACING: f32 = 90.0;
const CROSSING_SPACING: f32 = 60.0;
/// Heading change over `TURN_WINDOW` metres either side that counts as a sharp turn.
const TURN_THRESHOLD: f32 = 0.55;
const TURN_WINDOW: f32 = 18.0;
/// Turn masts stand this far outside the road edge and carry the lamp at this height.
const MAST_OFFSET: f32 = 3.0;
const MAST_HEIGHT: f32 = 9.0;

pub struct TrackLightingPlugin;

impl Plugin for TrackLightingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_track_lights)
            // Only looks at newly spawned meshes, so it costs nothing once the scene is built.
            .add_systems(Update, emissive_surfaces_cast_no_shadows);
    }
}

/// Shared fixture meshes and one emissive material per preset.
struct Fixtures {
    mast: Handle<Mesh>,
    lamp: Handle<Mesh>,
    plate: Handle<Mesh>,
    dark: Handle<StandardMaterial>,
    glow: [(LightPreset, Handle<StandardMaterial>); 4],
}

impl Fixtures {
    fn glow(&self, preset: LightPreset) -> Handle<StandardMaterial> {
        self.glow.iter().find(|(p, _)| p.color == preset.color).map(|(_, h)| h.clone()).unwrap_or_else(|| self.glow[0].1.clone())
    }
}

fn spawn_track_lights(mut commands: Commands, path: Res<TrackPath>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let mut glow = |preset: LightPreset| (preset, materials.add(StandardMaterial { base_color: preset.color, emissive: preset.emissive(), unlit: true, ..default() }));
    let glow = [glow(CYAN), glow(MAGENTA), glow(PURPLE), glow(PINK)];
    let fixtures = Fixtures {
        mast: meshes.add(Cylinder::new(0.22, MAST_HEIGHT).mesh().resolution(8)),
        lamp: meshes.add(Sphere::new(0.6).mesh().ico(1).expect("icosphere")),
        plate: meshes.add(Cuboid::new(3.0, 0.15, 0.8)),
        dark: materials.add(StandardMaterial { base_color: Color::srgb(0.05, 0.02, 0.1), metallic: 0.6, ..default() }),
        glow,
    };

    start_gate(&mut commands, &path, &fixtures);
    sharp_turns(&mut commands, &path, &fixtures);
    crossings(&mut commands, &path, &fixtures);

    // Arches carry alternating cyan/magenta lights that breathe slowly.
    for (i, frame) in arch_frames(&path).enumerate() {
        let preset = if i % 2 == 0 { CYAN } else { MAGENTA };
        commands.spawn((preset.light(), Transform::from_translation(frame.position + Vec3::Y * 10.0), Pulsating::new(0.25, 0.3, i as f32)));
    }
    // A few gantries over the elevated sections get a pink downlight.
    for frame in gantry_frames(&path).step_by(3).take(MAX_GANTRY_LIGHTS) {
        commands.spawn((PINK.light(), Transform::from_translation(frame.position + frame.normal * 7.0)));
    }
}

/// Light-emitting surfaces (neon, lamps, glows, particles) would not cast a
/// believable shadow, yet each caster is drawn once per shadow cascade. Skip them.
fn emissive_surfaces_cast_no_shadows(mut commands: Commands, materials: Res<Assets<StandardMaterial>>, added: Query<(Entity, &MeshMaterial3d<StandardMaterial>), Added<MeshMaterial3d<StandardMaterial>>>) {
    for (entity, handle) in &added {
        let Some(material) = materials.get(&handle.0) else { continue };
        let emissive = material.emissive;
        if material.unlit || emissive.red + emissive.green + emissive.blue > 0.3 {
            commands.entity(entity).insert(NotShadowCaster);
        }
    }
}

/// Cyan and magenta lights on the gate posts plus a pulsing pink overhead light.
fn start_gate(commands: &mut Commands, path: &TrackPath, fixtures: &Fixtures) {
    let start = path.frames[0];
    for (side, preset) in [(-1.0, CYAN), (1.0, MAGENTA)] {
        let position = start.position + start.side * side * (path.width * 0.5 + 1.0) + Vec3::Y * 13.0;
        commands.spawn((Mesh3d(fixtures.lamp.clone()), MeshMaterial3d(fixtures.glow(preset)), Transform::from_translation(position)));
        commands.spawn((preset.light(), Transform::from_translation(position - Vec3::Y * 0.8)));
    }
    let overhead = LightPreset { range: 48.0, intensity: 6_000_000.0, ..PINK };
    commands.spawn((overhead.light(), Transform::from_translation(start.position + Vec3::Y * 12.0), Pulsating::new(0.35, 0.6, 0.0)));
}

/// Lamp masts on the outside of the sharpest turns, alternating cyan/magenta.
fn sharp_turns(commands: &mut Commands, path: &TrackPath, fixtures: &Fixtures) {
    let count = path.frames.len();
    let window = (TURN_WINDOW / TrackPath::FRAME_SPACING) as usize;
    let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z).normalize_or_zero();
    // Signed heading change around each sample (positive = turning left).
    let turn: Vec<f32> = (0..count)
        .map(|i| {
            let (a, b) = (flat(path.frames[(i + count - window) % count].tangent), flat(path.frames[(i + window) % count].tangent));
            a.angle_between(b) * a.cross(b).y.signum()
        })
        .collect();
    let mut candidates: Vec<usize> = (0..count).filter(|&i| turn[i].abs() > TURN_THRESHOLD && turn[i].abs() >= turn[(i + 1) % count].abs() && turn[i].abs() >= turn[(i + count - 1) % count].abs()).collect();
    candidates.sort_by(|&a, &b| turn[b].abs().total_cmp(&turn[a].abs()));

    let mut placed: Vec<Vec3> = Vec::new();
    for i in candidates {
        let frame = &path.frames[i];
        if placed.iter().any(|p| p.distance(frame.position) < TURN_SPACING) { continue; }
        // The outside of a left turn is the right-hand side, and vice versa.
        let outside = if turn[i] > 0.0 { 1.0 } else { -1.0 };
        let base = frame.position + frame.side * outside * (path.width * 0.5 + MAST_OFFSET);
        let preset = if placed.len() % 2 == 0 { CYAN } else { MAGENTA };
        mast(commands, fixtures, base, preset);
        placed.push(frame.position);
        if placed.len() >= MAX_TURN_LIGHTS { break; }
    }
}

fn mast(commands: &mut Commands, fixtures: &Fixtures, base: Vec3, preset: LightPreset) {
    commands.spawn((Mesh3d(fixtures.mast.clone()), MeshMaterial3d(fixtures.dark.clone()), Transform::from_translation(base + Vec3::Y * MAST_HEIGHT * 0.5)));
    let head = base + Vec3::Y * MAST_HEIGHT;
    commands.spawn((Mesh3d(fixtures.lamp.clone()), MeshMaterial3d(fixtures.glow(preset)), Transform::from_translation(head)));
    commands.spawn((preset.light(), Transform::from_translation(head - Vec3::Y * 0.8)));
}

/// Purple light slung under each bridge where the track crosses itself, with a
/// glowing plate on the deck underside, lighting both the underpass and the bridge.
fn crossings(commands: &mut Commands, path: &TrackPath, fixtures: &Fixtures) {
    let count = path.frames.len();
    let stride = (4.0 / TrackPath::FRAME_SPACING).max(1.0) as usize;
    let mut placed: Vec<Vec3> = Vec::new();
    for i in (0..count).step_by(stride) {
        for j in (i + 1..count).step_by(stride) {
            let gap = (j - i).min(count - (j - i)) as f32 * TrackPath::FRAME_SPACING;
            if gap < path.width * 2.0 { continue; }
            let (a, b): (&TrackFrame, &TrackFrame) = (&path.frames[i], &path.frames[j]);
            let horizontal = Vec3::new(a.position.x - b.position.x, 0.0, a.position.z - b.position.z).length();
            let vertical = (a.position.y - b.position.y).abs();
            if horizontal > path.width * 0.6 || vertical < 4.0 { continue; }
            let (lower, upper) = if a.position.y < b.position.y { (a, b) } else { (b, a) };
            if placed.iter().any(|p| p.distance(lower.position) < CROSSING_SPACING) { continue; }
            let underside = upper.position - Vec3::Y * (TrackPath::ROAD_THICKNESS + 0.1);
            let rotation = Transform::IDENTITY.looking_to(upper.tangent, Vec3::Y).rotation;
            commands.spawn((Mesh3d(fixtures.plate.clone()), MeshMaterial3d(fixtures.glow(PURPLE)), Transform { translation: underside, rotation, ..default() }));
            commands.spawn((PURPLE.light(), Transform::from_translation(underside - Vec3::Y * 1.0), Pulsating::new(0.2, 0.25, placed.len() as f32)));
            placed.push(lower.position);
            if placed.len() >= MAX_CROSSING_LIGHTS { return; }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs the startup system headless and checks every category actually
    /// finds spots on the default layout, while the total stays within budget.
    #[test]
    fn lights_cover_turns_and_crossings_within_budget() {
        let mut app = App::new();
        app.insert_resource(TrackPath::default())
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_systems(Update, spawn_track_lights);
        app.update();
        let world = app.world_mut();
        let lights: Vec<PointLight> = world.query::<&PointLight>().iter(world).cloned().collect();
        let count = |preset: LightPreset| lights.iter().filter(|l| l.color == preset.color).count();
        println!("lights: total {}, cyan {}, magenta {}, purple {}, pink {}", lights.len(), count(CYAN), count(MAGENTA), count(PURPLE), count(PINK));
        assert!(count(PURPLE) >= 2, "expected underpass lights at crossings");
        assert!(count(CYAN) + count(MAGENTA) >= 2 + 6 + 4, "expected gate, arch and turn lights");
        assert!(lights.len() <= 40, "too many lights: {}", lights.len());
    }
}
