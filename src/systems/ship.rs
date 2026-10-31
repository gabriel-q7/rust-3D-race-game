//! Procedural anti-gravity racing ship: faceted low-poly hull, swept wings,
//! twin engines, and throttle-driven exhaust effects.
//!
//! Ship convention (same as the old car): width X, height Y, length Z, nose at local -Z.
//! The gameplay root (`Player`) carries movement; all meshes hang under a
//! `ShipVisual` child that banks and bobs without touching gameplay.

use bevy::{
    asset::RenderAssetUsages,
    math::primitives::{Cone, Cuboid, Cylinder, Sphere},
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

use crate::{
    components::{DriveState, Player},
    world::{animation::{Floating, Pulsating}, hash01},
};

/// Wingtip to wingtip; the track clamp keeps this inside the rails.
pub const SHIP_WIDTH: f32 = 5.0;
/// Distance from the road surface to the ship origin.
pub const HOVER_HEIGHT: f32 = 1.1;
/// Maximum visual roll (radians) when steering fully.
const BANK_ANGLE: f32 = 0.35;
/// Engine exhaust centres in ship space.
const EXHAUSTS: [Vec3; 2] = [Vec3::new(-0.85, 0.0, 3.05), Vec3::new(0.85, 0.0, 3.05)];
/// Particle pool size; particles are recycled, never spawned per frame.
const PARTICLE_POOL: usize = 64;
/// Particles emitted per second at full power.
const PARTICLE_RATE: f32 = 70.0;

pub struct ShipPlugin;

impl Plugin for ShipPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (bank_ship, engine_effects, update_particles));
    }
}

/// Child entity holding every ship mesh; receives roll and hover bob.
#[derive(Component)]
pub struct ShipVisual;

/// Exhaust flame cone; its length follows engine power.
#[derive(Component)]
struct ExhaustFlame;

/// Smoothed engine output in `0..=1` plus particle emitter bookkeeping.
#[derive(Component, Default)]
pub struct EngineState {
    pub power: f32,
    emit_accumulator: f32,
    next_particle: usize,
}

#[derive(Component)]
struct EngineParticle {
    age: f32,
    lifetime: f32,
    velocity: Vec3,
}

#[derive(Resource)]
struct EngineFx {
    /// Shared by exhaust nozzles and flames; its emissive tracks engine power.
    glow: Handle<StandardMaterial>,
    particles: Vec<Entity>,
}

/// Builds the ship meshes under `root`. Called once by the player spawner.
pub fn spawn_ship(commands: &mut Commands, root: Entity, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) {
    let hull = materials.add(StandardMaterial { base_color: Color::srgb(0.2, 0.05, 0.42), metallic: 0.6, perceptual_roughness: 0.35, ..default() });
    let wing = materials.add(StandardMaterial { base_color: Color::srgb(0.08, 0.02, 0.2), metallic: 0.5, perceptual_roughness: 0.4, ..default() });
    let metal = materials.add(StandardMaterial { base_color: Color::srgb(0.05, 0.04, 0.08), metallic: 0.8, perceptual_roughness: 0.3, ..default() });
    let glass = materials.add(StandardMaterial { base_color: Color::srgb(0.05, 0.5, 0.75), emissive: LinearRgba::rgb(0.0, 0.25, 0.4), perceptual_roughness: 0.05, ..default() });
    let pink = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.05, 0.5), emissive: LinearRgba::rgb(3.0, 0.1, 1.2), ..default() });
    let cyan = materials.add(StandardMaterial { base_color: Color::srgb(0.1, 0.9, 1.0), emissive: LinearRgba::rgb(0.2, 2.4, 3.0), ..default() });
    let glow = materials.add(StandardMaterial { base_color: Color::srgb(0.6, 0.9, 1.0), emissive: LinearRgba::rgb(0.5, 1.5, 3.0), unlit: true, ..default() });
    // Particles live in world space (no parent) so they trail behind the ship.
    let particle_mesh = meshes.add(Cuboid::new(0.22, 0.22, 0.22));
    let particle_material = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.4, 0.9), emissive: LinearRgba::rgb(3.0, 0.6, 2.4), unlit: true, ..default() });
    let particles = (0..PARTICLE_POOL)
        .map(|_| commands.spawn((Mesh3d(particle_mesh.clone()), MeshMaterial3d(particle_material.clone()), Transform::default(), Visibility::Hidden, EngineParticle { age: 0.0, lifetime: 0.0, velocity: Vec3::ZERO })).id())
        .collect();
    commands.insert_resource(EngineFx { glow: glow.clone(), particles });

    let visual = commands.spawn((ShipVisual, Transform::default(), Visibility::default(), Floating::new(0.08, 0.6, 0.0), ChildOf(root))).id();

    part(commands, visual, meshes.add(hull_mesh()), &hull, Transform::default());
    part(commands, visual, meshes.add(wings_mesh()), &wing, Transform::default());
    part(commands, visual, meshes.add(fins_mesh()), &wing, Transform::default());
    // Canopy: a stretched sphere reads as a sleek cockpit bubble.
    part(commands, visual, meshes.add(Sphere::new(1.0).mesh().uv(16, 10)), &glass, Transform::from_xyz(0.0, 0.42, -0.6).with_scale(Vec3::new(0.48, 0.3, 1.15)));

    // Neon accents: spine stripe, wing leading edges, wingtip lights, under-glow.
    part(commands, visual, meshes.add(Cuboid::new(0.12, 0.06, 3.2)), &cyan, Transform::from_xyz(0.0, 0.47, 1.0));
    for side in [-1.0, 1.0] {
        let (from, to) = (Vec3::new(side * 0.75, -0.02, -0.7), Vec3::new(side * 2.45, -0.02, 1.85));
        part(commands, visual, meshes.add(Cuboid::new(0.1, 0.1, from.distance(to))), &pink, Transform::from_translation((from + to) * 0.5).looking_to(to - from, Vec3::Y));
        // Each wingtip gets its own material so the two can pulse out of phase.
        let tip_light = materials.add(StandardMaterial { base_color: Color::srgb(0.1, 0.9, 1.0), emissive: LinearRgba::rgb(0.2, 2.4, 3.0), ..default() });
        let phase = if side < 0.0 { 0.0 } else { std::f32::consts::PI };
        part(commands, visual, meshes.add(Cuboid::new(0.14, 0.14, 0.7)), &tip_light, Transform::from_xyz(side * 2.5, 0.0, 2.25)).insert(Pulsating::new(0.5, 2.2, phase));
    }
    let underglow = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.05, 0.5), emissive: LinearRgba::rgb(3.0, 0.1, 1.2), ..default() });
    part(commands, visual, meshes.add(Cuboid::new(1.4, 0.04, 4.2)), &underglow, Transform::from_xyz(0.0, -0.36, 0.4)).insert(Pulsating::new(0.25, 1.1, 0.5));

    // Twin engines with glowing nozzles and throttle-driven flames.
    let pod = meshes.add(Cylinder::new(0.4, 1.7).mesh().resolution(10));
    let nozzle = meshes.add(Cylinder::new(0.3, 0.06).mesh().resolution(10));
    let flame = meshes.add(Cone { radius: 0.28, height: 1.0 }.mesh().resolution(10));
    let along_z = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
    for exhaust in EXHAUSTS {
        part(commands, visual, pod.clone(), &metal, Transform { translation: exhaust - Vec3::Z * 0.85, rotation: along_z, ..default() });
        part(commands, visual, nozzle.clone(), &glow, Transform { translation: exhaust + Vec3::Z * 0.01, rotation: along_z, ..default() });
        part(commands, visual, flame.clone(), &glow, Transform { translation: exhaust + Vec3::Z * 0.5, rotation: along_z, ..default() }).insert(ExhaustFlame);
    }
}

fn part<'a>(commands: &'a mut Commands, parent: Entity, mesh: Handle<Mesh>, material: &Handle<StandardMaterial>, transform: Transform) -> EntityCommands<'a> {
    commands.spawn((Mesh3d(mesh), MeshMaterial3d(material.clone()), transform, ChildOf(parent)))
}

/// Rolls the visual into turns; gameplay orientation stays untouched.
fn bank_ship(time: Res<Time>, player: Query<&DriveState, With<Player>>, mut visual: Query<&mut Transform, With<ShipVisual>>) {
    let (Ok(drive), Ok(mut transform)) = (player.single(), visual.single_mut()) else { return };
    let target = Quat::from_rotation_z(drive.steer * BANK_ANGLE) * Quat::from_rotation_x(drive.throttle.max(0.0) * 0.04);
    transform.rotation = transform.rotation.slerp(target, (time.delta_secs() * 8.0).min(1.0));
}

/// Smooths engine power from throttle and speed, then drives glow, flames and particle emission.
fn engine_effects(
    time: Res<Time>,
    fx: Option<Res<EngineFx>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut player: Query<(&Transform, &DriveState, &mut EngineState), With<Player>>,
    mut flames: Query<&mut Transform, (With<ExhaustFlame>, Without<Player>)>,
    mut particles: Query<(&mut Transform, &mut Visibility, &mut EngineParticle), (Without<Player>, Without<ExhaustFlame>)>,
) {
    let (Some(fx), Ok((ship, drive, mut engine))) = (fx, player.single_mut()) else { return };
    let dt = time.delta_secs();
    // Idle glow when parked, stronger with speed, full burn while accelerating.
    let speed_ratio = (drive.speed.abs() / crate::systems::player::MAX_SPEED).clamp(0.0, 1.0);
    let target = (0.12 + 0.45 * speed_ratio).max(drive.throttle.max(0.0));
    engine.power += (target - engine.power) * (dt * 5.0).min(1.0);
    let power = engine.power;

    if let Some(mut material) = materials.get_mut(&fx.glow) {
        material.emissive = LinearRgba::rgb(0.4, 1.2, 3.0) * (0.4 + 3.5 * power);
    }
    for mut flame in &mut flames {
        let length = 0.25 + 2.2 * power;
        flame.scale = Vec3::new(0.7 + 0.5 * power, length, 0.7 + 0.5 * power);
        flame.translation.z = EXHAUSTS[0].z + length * 0.5;
    }

    engine.emit_accumulator += PARTICLE_RATE * power * power * dt;
    while engine.emit_accumulator >= 1.0 {
        engine.emit_accumulator -= 1.0;
        let slot = engine.next_particle;
        engine.next_particle = (slot + 1) % fx.particles.len().max(1);
        let Some(&entity) = fx.particles.get(slot) else { break };
        let Ok((mut transform, mut visibility, mut particle)) = particles.get_mut(entity) else { continue };
        let jitter = Vec3::new(hash01(slot as u32 * 3) - 0.5, hash01(slot as u32 * 3 + 1) - 0.5, 0.0) * 0.6;
        let exhaust = EXHAUSTS[slot % EXHAUSTS.len()] + Vec3::Z * 0.3 + jitter;
        transform.translation = ship.transform_point(exhaust);
        transform.scale = Vec3::ONE;
        *visibility = Visibility::Visible;
        // Particles lag behind the ship and drift backwards, leaving a short trail.
        particle.velocity = *ship.back() * (3.0 + 6.0 * power) + *ship.forward() * drive.speed * 0.35 + ship.rotation * jitter * 2.0;
        particle.age = 0.0;
        particle.lifetime = 0.3 + 0.3 * power;
    }
}

fn update_particles(time: Res<Time>, mut particles: Query<(&mut Transform, &mut Visibility, &mut EngineParticle)>) {
    let dt = time.delta_secs();
    for (mut transform, mut visibility, mut particle) in &mut particles {
        if *visibility == Visibility::Hidden { continue; }
        particle.age += dt;
        if particle.age >= particle.lifetime {
            *visibility = Visibility::Hidden;
            continue;
        }
        transform.translation += particle.velocity * dt;
        transform.scale = Vec3::splat(1.0 - particle.age / particle.lifetime);
    }
}

/// Flat-shaded triangle soup; every triangle is flipped to face away from a
/// reference point inside the (convex) part it belongs to.
#[derive(Default)]
struct Faceted {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
}

impl Faceted {
    fn tri(&mut self, a: Vec3, b: Vec3, c: Vec3, inside: Vec3) {
        let mut normal = (b - a).cross(c - a).normalize_or_zero();
        let (b, c) = if normal.dot(a - inside) < 0.0 { normal = -normal; (c, b) } else { (b, c) };
        self.positions.extend([a, b, c].map(|v| v.to_array()));
        self.normals.extend([normal.to_array(); 3]);
    }

    /// Convex polygon in the plane spanned by `u`/`v`, extruded ± `half` along `u × v`.
    fn prism(&mut self, polygon: &[Vec2], origin: Vec3, u: Vec3, v: Vec3, half: f32) {
        let n = u.cross(v).normalize() * half;
        let point = |p: Vec2| origin + u * p.x + v * p.y;
        let top: Vec<Vec3> = polygon.iter().map(|&p| point(p) + n).collect();
        let bottom: Vec<Vec3> = polygon.iter().map(|&p| point(p) - n).collect();
        let inside = top.iter().chain(&bottom).copied().sum::<Vec3>() / (2 * polygon.len()) as f32;
        for i in 1..polygon.len() - 1 {
            self.tri(top[0], top[i], top[i + 1], inside);
            self.tri(bottom[0], bottom[i], bottom[i + 1], inside);
        }
        for i in 0..polygon.len() {
            let j = (i + 1) % polygon.len();
            self.tri(top[i], top[j], bottom[j], inside);
            self.tri(top[i], bottom[j], bottom[i], inside);
        }
    }

    fn build(self) -> Mesh {
        let count = self.positions.len() as u32;
        Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, self.positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals)
            .with_inserted_indices(Indices::U32((0..count).collect()))
    }
}

/// Lofted hull: pointed nose, widening to a broad, flat rear.
fn hull_mesh() -> Mesh {
    // (z, half width, bottom, side height, top, top half width)
    const SECTIONS: [(f32, f32, f32, f32, f32, f32); 4] = [
        (-1.7, 0.55, -0.22, -0.05, 0.22, 0.28),
        (0.3, 0.95, -0.32, 0.0, 0.42, 0.5),
        (1.9, 1.2, -0.3, 0.05, 0.45, 0.75),
        (3.0, 1.15, -0.25, 0.05, 0.38, 0.7),
    ];
    let nose = Vec3::new(0.0, -0.04, -3.4);
    let rings: Vec<[Vec3; 5]> = SECTIONS
        .iter()
        .map(|&(z, w, b, m, t, tw)| [Vec3::new(0.0, b, z), Vec3::new(w, m, z), Vec3::new(tw, t, z), Vec3::new(-tw, t, z), Vec3::new(-w, m, z)])
        .collect();
    let axis = |z: f32| Vec3::new(0.0, 0.05, z);
    let mut mesh = Faceted::default();
    for i in 0..5 {
        let j = (i + 1) % 5;
        mesh.tri(nose, rings[0][i], rings[0][j], axis(-1.5));
        for r in 0..rings.len() - 1 {
            let (a, b) = (&rings[r], &rings[r + 1]);
            let inside = axis((a[0].z + b[0].z) * 0.5);
            mesh.tri(a[i], a[j], b[j], inside);
            mesh.tri(a[i], b[j], b[i], inside);
        }
        let last = &rings[rings.len() - 1];
        mesh.tri(last[i], last[j], axis(last[0].z), axis(last[0].z - 0.5));
    }
    mesh.build()
}

/// Swept delta wings; planform given as (x, z) for the right side and mirrored.
fn wings_mesh() -> Mesh {
    let mut mesh = Faceted::default();
    for side in [-1.0, 1.0] {
        let planform = [Vec2::new(0.7, -0.8), Vec2::new(2.5, 1.85), Vec2::new(2.55, 2.65), Vec2::new(0.9, 2.5)];
        let polygon: Vec<Vec2> = planform.iter().map(|p| Vec2::new(p.x * side, p.y)).collect();
        mesh.prism(&polygon, Vec3::new(0.0, -0.05, 0.0), Vec3::X, Vec3::Z, 0.06);
    }
    mesh.build()
}

/// Twin vertical stabilisers on the rear deck; polygon given as (z, y).
fn fins_mesh() -> Mesh {
    let mut mesh = Faceted::default();
    for side in [-0.65, 0.65] {
        let polygon = [Vec2::new(1.2, 0.35), Vec2::new(2.9, 0.38), Vec2::new(3.05, 1.25), Vec2::new(2.45, 1.25)];
        mesh.prism(&polygon, Vec3::new(side, 0.0, 0.0), Vec3::Z, Vec3::Y, 0.05);
    }
    mesh.build()
}
