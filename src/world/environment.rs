use bevy::{math::primitives::{Cuboid, Cylinder, Sphere, Tetrahedron, Torus}, prelude::*};

use crate::{
    resources::TrackPath,
    world::{animation::{Floating, Pulsating, Rotating, Swaying}, hash01},
};

/// Props are scattered on a jittered grid covering this half-extent around the origin.
const SCATTER_EXTENT: f32 = 300.0;
const SCATTER_CELL: f32 = 38.0;
/// Extra horizontal gap between a prop's footprint and the road edge.
const ROAD_MARGIN: f32 = 6.0;
/// Pulsing neon is grouped into a few phase-shifted materials instead of one per entity.
const PHASE_GROUPS: usize = 4;

pub struct EnvironmentPlugin;
impl Plugin for EnvironmentPlugin { fn build(&self, app: &mut App) { app.add_systems(Startup, spawn_environment); } }

/// Shared meshes and materials for every prop kind.
struct Palette {
    cyan_pulse: Vec<Handle<StandardMaterial>>,
    pink_pulse: Vec<Handle<StandardMaterial>>,
    pink: Handle<StandardMaterial>,
    cyan: Handle<StandardMaterial>,
    purple: Handle<StandardMaterial>,
    dark: Handle<StandardMaterial>,
    trunk: Handle<StandardMaterial>,
    frond: Handle<StandardMaterial>,
}

impl Palette {
    fn new(materials: &mut Assets<StandardMaterial>) -> Self {
        let neon = |materials: &mut Assets<StandardMaterial>, base: Color, emissive: LinearRgba| materials.add(StandardMaterial { base_color: base, emissive, ..default() });
        Self {
            cyan_pulse: (0..PHASE_GROUPS).map(|_| neon(materials, Color::srgb(0.02, 0.6, 0.8), LinearRgba::rgb(0.0, 0.9, 1.4))).collect(),
            pink_pulse: (0..PHASE_GROUPS).map(|_| neon(materials, Color::srgb(1.0, 0.03, 0.45), LinearRgba::rgb(1.6, 0.0, 0.5))).collect(),
            pink: neon(materials, Color::srgb(1.0, 0.1, 0.6), LinearRgba::rgb(2.0, 0.15, 0.9)),
            cyan: neon(materials, Color::srgb(0.1, 0.8, 1.0), LinearRgba::rgb(0.2, 1.6, 2.2)),
            purple: materials.add(Color::srgb(0.14, 0.02, 0.28)),
            dark: materials.add(StandardMaterial { base_color: Color::srgb(0.06, 0.02, 0.12), metallic: 0.4, ..default() }),
            trunk: materials.add(Color::srgb(0.35, 0.08, 0.35)),
            frond: neon(materials, Color::srgb(0.05, 0.75, 0.6), LinearRgba::rgb(0.0, 0.35, 0.3)),
        }
    }
}

fn spawn_environment(mut commands: Commands, path: Res<TrackPath>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    commands.spawn((DirectionalLight { illuminance: 7000.0, color: Color::srgb(0.35, 0.2, 0.7), shadow_maps_enabled: true, ..default() }, Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, -0.8, 0.0))));
    commands.insert_resource(GlobalAmbientLight { color: Color::srgb(0.18, 0.06, 0.25), brightness: 500.0, ..default() });
    // Ground, grid and sun live in the backdrop module.
    let palette = Palette::new(&mut materials);
    // One Pulsating driver per phase group: each group material breathes on its own phase.
    for (group, handle) in palette.cyan_pulse.iter().chain(&palette.pink_pulse).enumerate() {
        commands.spawn((MeshMaterial3d(handle.clone()), Pulsating::new(0.45, 0.35 + 0.07 * group as f32, group as f32 * 1.3)));
    }

    // Arches sit on the generated road, evenly spread around the loop, outside the rails.
    let post = meshes.add(Cuboid::new(0.6, 11.0, 0.6));
    let beam = meshes.add(Cuboid::new(path.width + 3.4, 0.6, 0.6));
    for (i, frame) in path.frames.iter().skip(path.frames.len() / 8).step_by(path.frames.len() / 6).enumerate() {
        let material = &palette.cyan_pulse[i % PHASE_GROUPS];
        let rotation = Transform::IDENTITY.looking_to(frame.tangent, Vec3::Y).rotation;
        for offset in [-1.0, 1.0] {
            commands.spawn((Mesh3d(post.clone()), MeshMaterial3d(material.clone()), Transform { translation: frame.position + frame.side * offset * (path.width * 0.5 + 1.4) + Vec3::Y * 5.5, rotation, ..default() }));
        }
        commands.spawn((Mesh3d(beam.clone()), MeshMaterial3d(material.clone()), Transform { translation: frame.position + Vec3::Y * 11.0, rotation, ..default() }));
    }

    let props = Props {
        trunk: meshes.add(Cylinder::new(0.45, 3.2).mesh().resolution(7)),
        frond: meshes.add(Cuboid::new(0.9, 0.12, 5.5)),
        pillar: meshes.add(Cylinder::new(0.8, 16.0).mesh().resolution(8)),
        building: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        platform: meshes.add(Cylinder::new(7.0, 0.8).mesh().resolution(6)),
        ring: meshes.add(Torus::new(6.8, 7.6).mesh().minor_resolution(6).major_resolution(24)),
        ico: meshes.add(Sphere::new(3.2).mesh().ico(0).expect("icosphere with 0 subdivisions")),
        tetra: meshes.add(Tetrahedron::default().mesh()),
        torus: meshes.add(Torus::new(2.0, 3.2).mesh().minor_resolution(8).major_resolution(16)),
    };

    // Deterministic scatter: every cell gets a jittered position and a prop kind.
    let half_road = path.width * 0.5;
    let cells = (SCATTER_EXTENT * 2.0 / SCATTER_CELL) as i32;
    let mut lights = 0;
    for cx in 0..cells {
        for cz in 0..cells {
            let id = (cx * cells + cz) as u32 * 7;
            let roll = |k: u32| hash01(id + k);
            let x = -SCATTER_EXTENT + (cx as f32 + 0.2 + 0.6 * roll(0)) * SCATTER_CELL;
            let z = -SCATTER_EXTENT + (cz as f32 + 0.2 + 0.6 * roll(1)) * SCATTER_CELL;
            let ground = Vec3::new(x, TrackPath::GROUND_TOP, z);
            let clearance = path.horizontal_clearance(ground) - half_road - ROAD_MARGIN;
            let phase = roll(2) * std::f32::consts::TAU;
            match roll(3) {
                k if k < 0.38 && clearance > 6.0 => props.palm(&mut commands, &palette, ground, roll(4), phase),
                k if k < 0.52 && clearance > 2.0 => props.pillar(&mut commands, &palette, ground, id as usize),
                k if k < 0.64 && clearance > 12.0 => props.building(&mut commands, &palette, ground, roll(4), id as usize),
                k if k < 0.76 && clearance > 9.0 => {
                    let with_light = lights < 6;
                    lights += with_light as usize;
                    props.platform(&mut commands, &palette, ground + Vec3::Y * (10.0 + 10.0 * roll(4)), phase, with_light);
                }
                k if k < 0.92 && clearance > 5.0 => props.floater(&mut commands, &palette, ground + Vec3::Y * (34.0 + 20.0 * roll(4)), roll(5), phase),
                _ => {}
            }
        }
    }
}

struct Props {
    trunk: Handle<Mesh>,
    frond: Handle<Mesh>,
    pillar: Handle<Mesh>,
    building: Handle<Mesh>,
    platform: Handle<Mesh>,
    ring: Handle<Mesh>,
    ico: Handle<Mesh>,
    tetra: Handle<Mesh>,
    torus: Handle<Mesh>,
}

impl Props {
    /// Low-poly neon palm: a curved trunk and a crown of drooping fronds, swaying from its base.
    fn palm(&self, commands: &mut Commands, palette: &Palette, base: Vec3, size: f32, phase: f32) {
        let scale = 0.9 + 0.5 * size;
        let axis = Vec3::new(phase.cos(), 0.0, phase.sin());
        let root = commands.spawn((Transform::from_translation(base).with_scale(Vec3::splat(scale)), Visibility::default(), Swaying::new(axis, 0.035, 0.22 + 0.1 * size, phase))).id();
        let mut top = Vec3::ZERO;
        for i in 0..4 {
            // Each segment leans a bit more, giving the trunk its curve.
            let lean = Quat::from_rotation_z(-0.06 * (i as f32 + 1.0));
            let center = top + lean * Vec3::Y * 1.6;
            commands.spawn((Mesh3d(self.trunk.clone()), MeshMaterial3d(palette.trunk.clone()), Transform { translation: center, rotation: lean, scale: Vec3::splat(1.0 - 0.1 * i as f32) }, ChildOf(root)));
            top = center + lean * Vec3::Y * 1.6;
        }
        for k in 0..7 {
            let yaw = Quat::from_rotation_y(k as f32 * std::f32::consts::TAU / 7.0 + phase);
            let rotation = yaw * Quat::from_rotation_x(-0.45);
            commands.spawn((Mesh3d(self.frond.clone()), MeshMaterial3d(palette.frond.clone()), Transform { translation: top + rotation * Vec3::new(0.0, 0.0, -2.5), rotation, ..default() }, ChildOf(root)));
        }
    }

    fn pillar(&self, commands: &mut Commands, palette: &Palette, base: Vec3, id: usize) {
        commands.spawn((Mesh3d(self.pillar.clone()), MeshMaterial3d(palette.cyan_pulse[id % PHASE_GROUPS].clone()), Transform::from_translation(base + Vec3::Y * 8.0)));
    }

    /// Dark tower with a pulsing neon roof band.
    fn building(&self, commands: &mut Commands, palette: &Palette, base: Vec3, size: f32, id: usize) {
        let height = 14.0 + 22.0 * size;
        commands.spawn((Mesh3d(self.building.clone()), MeshMaterial3d(palette.purple.clone()), Transform::from_translation(base + Vec3::Y * height * 0.5).with_scale(Vec3::new(14.0, height, 14.0))));
        commands.spawn((Mesh3d(self.building.clone()), MeshMaterial3d(palette.pink_pulse[id % PHASE_GROUPS].clone()), Transform::from_translation(base + Vec3::Y * (height + 0.25)).with_scale(Vec3::new(14.6, 0.5, 14.6))));
    }

    /// Hexagonal hover platform with a neon rim; bobs and turns slowly.
    fn platform(&self, commands: &mut Commands, palette: &Palette, center: Vec3, phase: f32, with_light: bool) {
        let root = commands.spawn((Mesh3d(self.platform.clone()), MeshMaterial3d(palette.dark.clone()), Transform::from_translation(center), Floating::new(1.2, 0.12, phase), Rotating::new(Vec3::Y, 0.08, phase))).id();
        commands.spawn((Mesh3d(self.ring.clone()), MeshMaterial3d(palette.cyan.clone()), Transform::default(), ChildOf(root)));
        if with_light {
            // A handful of real lights; kept few because each costs per-pixel work.
            commands.spawn((PointLight { color: Color::srgb(1.0, 0.2, 0.7), intensity: 2_000_000.0, range: 45.0, shadow_maps_enabled: false, ..default() }, Transform::from_xyz(0.0, -2.0, 0.0), Pulsating::new(0.5, 0.5, phase), ChildOf(root)));
        }
    }

    /// High floating geometric shape, spinning on a tilted axis.
    fn floater(&self, commands: &mut Commands, palette: &Palette, center: Vec3, kind: f32, phase: f32) {
        let (mesh, material, scale) = match (kind * 3.0) as u32 {
            0 => (self.ico.clone(), palette.pink.clone(), 1.0),
            1 => (self.tetra.clone(), palette.cyan.clone(), 5.0),
            _ => (self.torus.clone(), palette.pink.clone(), 1.0),
        };
        let axis = Vec3::new(phase.sin() * 0.5, 1.0, phase.cos() * 0.5);
        commands.spawn((Mesh3d(mesh), MeshMaterial3d(material), Transform::from_translation(center).with_scale(Vec3::splat(scale)), Rotating::new(axis, 0.25 + 0.4 * kind, phase), Floating::new(1.5 + kind, 0.08 + 0.06 * kind, phase)));
    }
}
