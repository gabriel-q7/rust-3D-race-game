//! Procedural anti-gravity racer: a wide, flat, angular hull with an integrated
//! cockpit, side-mounted engine pods ending in large propulsion units, armor
//! plates, neon strips and glowing hover pads. Exhaust glow, flames and trail
//! particles follow throttle.
//!
//! Ship convention: width X, height Y, length Z, nose at local -Z.
//! The gameplay root (`Player`) carries movement; all meshes hang under a
//! `ShipVisual` child that banks and bobs without touching gameplay.

use bevy::{
    asset::RenderAssetUsages,
    math::primitives::{Cone, Cuboid, Cylinder, Torus},
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

use crate::{
    components::{DriveState, Player},
    world::{animation::Floating, hash01},
};

// --- Proportions (ship space, metres) -------------------------------------
/// Outer edge to outer edge of the engine pods; the track clamp keeps this inside the rails.
pub const SHIP_WIDTH: f32 = 5.4;
/// Distance from the road surface to the ship origin.
pub const HOVER_HEIGHT: f32 = 1.0;
/// Lateral offset of the side engine pods from the centreline.
const POD_X: f32 = 2.05;
/// Pod radius at its rear (largest) section.
const POD_RADIUS: f32 = 0.66;
/// Maximum visual roll (radians) when steering fully; small, the hull is wide.
const BANK_ANGLE: f32 = 0.22;
/// Hover bob while idle: amplitude (m) and frequency (Hz).
const HOVER_BOB: (f32, f32) = (0.07, 0.55);

// --- Engines ---------------------------------------------------------------
/// Exhaust centre and nozzle radius: two big pod units and a smaller centre booster.
const EXHAUSTS: [(Vec3, f32); 3] = [
    (Vec3::new(-POD_X, 0.0, 3.05), 0.5),
    (Vec3::new(POD_X, 0.0, 3.05), 0.5),
    (Vec3::new(0.0, 0.12, 3.05), 0.3),
];
/// Emissive colour of nozzles and flames at power 1 (HDR, bloom picks it up).
const ENGINE_GLOW: LinearRgba = LinearRgba::rgb(0.5, 1.4, 3.4);
/// Emissive multiplier range: idle → full burn.
const GLOW_IDLE: f32 = 0.45;
const GLOW_BOOST: f32 = 3.6;
/// Flame length (m) at idle and the extra length at full power.
const FLAME_IDLE: f32 = 0.3;
const FLAME_BOOST: f32 = 2.6;
/// Particle pool size; particles are recycled, never spawned per frame.
const PARTICLE_POOL: usize = 72;
/// Particles emitted per second at full power.
const PARTICLE_RATE: f32 = 80.0;

pub struct ShipPlugin;

impl Plugin for ShipPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (bank_ship, engine_effects, update_particles));
    }
}

/// Child entity holding every ship mesh; receives roll and hover bob.
#[derive(Component)]
pub struct ShipVisual;

/// Exhaust flame cone; its length follows engine power from `base_z` backwards.
#[derive(Component)]
struct ExhaustFlame {
    base_z: f32,
}

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
    /// Shared by nozzles and flames; emissive tracks engine power.
    glow: Handle<StandardMaterial>,
    /// Hover pads under the hull; a softer glow that breathes while idle.
    hover: Handle<StandardMaterial>,
    particles: Vec<Entity>,
}

/// Builds the ship meshes under `root`. Called once by the player spawner.
pub fn spawn_ship(commands: &mut Commands, root: Entity, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) {
    let hull = materials.add(StandardMaterial { base_color: Color::srgb(0.22, 0.06, 0.45), metallic: 0.7, perceptual_roughness: 0.3, ..default() });
    let armor = materials.add(StandardMaterial { base_color: Color::srgb(0.07, 0.03, 0.16), metallic: 0.6, perceptual_roughness: 0.45, ..default() });
    let metal = materials.add(StandardMaterial { base_color: Color::srgb(0.06, 0.05, 0.1), metallic: 0.9, perceptual_roughness: 0.25, ..default() });
    let glass = materials.add(StandardMaterial { base_color: Color::srgb(0.03, 0.35, 0.55), emissive: LinearRgba::rgb(0.0, 0.35, 0.6), perceptual_roughness: 0.05, metallic: 0.2, ..default() });
    let pink = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.05, 0.5), emissive: LinearRgba::rgb(3.0, 0.1, 1.2), ..default() });
    let cyan = materials.add(StandardMaterial { base_color: Color::srgb(0.1, 0.9, 1.0), emissive: LinearRgba::rgb(0.2, 2.4, 3.0), ..default() });
    let glow = materials.add(StandardMaterial { base_color: Color::srgb(0.6, 0.9, 1.0), emissive: ENGINE_GLOW, unlit: true, ..default() });
    let hover = materials.add(StandardMaterial { base_color: Color::srgb(0.5, 0.2, 1.0), emissive: LinearRgba::rgb(1.2, 0.4, 2.6), unlit: true, ..default() });

    // Particles live in world space (no parent) so they trail behind the ship.
    let particle_mesh = meshes.add(Cuboid::new(0.24, 0.24, 0.24));
    let particle_material = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.4, 0.9), emissive: LinearRgba::rgb(3.0, 0.6, 2.4), unlit: true, ..default() });
    let particles = (0..PARTICLE_POOL)
        .map(|_| commands.spawn((Mesh3d(particle_mesh.clone()), MeshMaterial3d(particle_material.clone()), Transform::default(), Visibility::Hidden, EngineParticle { age: 0.0, lifetime: 0.0, velocity: Vec3::ZERO })).id())
        .collect();
    commands.insert_resource(EngineFx { glow: glow.clone(), hover: hover.clone(), particles });

    let visual = commands.spawn((ShipVisual, Transform::default(), Visibility::default(), Floating::new(HOVER_BOB.0, HOVER_BOB.1, 0.0), ChildOf(root))).id();

    // Body, cockpit, pods and the struts joining them.
    part(commands, visual, meshes.add(hull_mesh()), &hull, Transform::default());
    part(commands, visual, meshes.add(cockpit_mesh()), &glass, Transform::default());
    part(commands, visual, meshes.add(pods_mesh()), &hull, Transform::default());
    part(commands, visual, meshes.add(armor_mesh()), &armor, Transform::default());

    // Neon strips set into the hull: side lines, pod spines and a nose chevron.
    let strip = meshes.add(Cuboid::new(0.07, 0.07, 1.0));
    let neon = |commands: &mut Commands, from: Vec3, to: Vec3, material: &Handle<StandardMaterial>| {
        let transform = Transform::from_translation((from + to) * 0.5).looking_to(to - from, Vec3::Y).with_scale(Vec3::new(1.0, 1.0, from.distance(to)));
        part(commands, visual, strip.clone(), material, transform);
    };
    for side in [-1.0, 1.0] {
        neon(commands, Vec3::new(side * 1.62, 0.02, -2.0), Vec3::new(side * 1.82, 0.06, 2.9), &pink);
        neon(commands, Vec3::new(side * POD_X, 0.5, -0.6), Vec3::new(side * POD_X, 0.62, 2.8), &cyan);
        neon(commands, Vec3::new(side * 0.12, 0.12, -3.3), Vec3::new(side * 0.95, 0.4, -1.6), &pink);
    }

    // Anti-gravity pads: glowing discs under the body and both pods.
    let pad = meshes.add(Cylinder::new(0.45, 0.05).mesh().resolution(12));
    for (x, z) in [(-POD_X, -0.3), (POD_X, -0.3), (-POD_X, 2.0), (POD_X, 2.0), (0.0, -1.6), (0.0, 1.4)] {
        part(commands, visual, pad.clone(), &hover, Transform::from_xyz(x, -0.52, z));
    }

    // Propulsion units: armored ring, glowing nozzle and a throttle-driven flame.
    let along_z = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
    for (exhaust, radius) in EXHAUSTS {
        let ring = meshes.add(Torus::new(radius * 0.85, radius * 1.3).mesh().minor_resolution(6).major_resolution(16));
        let nozzle = meshes.add(Cylinder::new(radius * 0.9, 0.06).mesh().resolution(16));
        let flame = meshes.add(Cone { radius: radius * 0.85, height: 1.0 }.mesh().resolution(12));
        part(commands, visual, ring, &metal, Transform { translation: exhaust, rotation: along_z, ..default() });
        part(commands, visual, nozzle, &glow, Transform { translation: exhaust + Vec3::Z * 0.02, rotation: along_z, ..default() });
        part(commands, visual, flame, &glow, Transform { translation: exhaust + Vec3::Z * 0.5, rotation: along_z, ..default() }).insert(ExhaustFlame { base_z: exhaust.z });
    }
}

fn part<'a>(commands: &'a mut Commands, parent: Entity, mesh: Handle<Mesh>, material: &Handle<StandardMaterial>, transform: Transform) -> EntityCommands<'a> {
    commands.spawn((Mesh3d(mesh), MeshMaterial3d(material.clone()), transform, ChildOf(parent)))
}

/// Rolls the visual into turns; gameplay orientation stays untouched.
fn bank_ship(time: Res<Time>, player: Query<&DriveState, With<Player>>, mut visual: Query<&mut Transform, With<ShipVisual>>) {
    let (Ok(drive), Ok(mut transform)) = (player.single(), visual.single_mut()) else { return };
    // Nose lifts slightly under thrust and dips when braking.
    let target = Quat::from_rotation_z(drive.steer * BANK_ANGLE) * Quat::from_rotation_x(drive.throttle * 0.035);
    transform.rotation = transform.rotation.slerp(target, (time.delta_secs() * 8.0).min(1.0));
}

/// Smooths engine power from throttle and speed, then drives glow, flames and particle emission.
fn engine_effects(
    time: Res<Time>,
    fx: Option<Res<EngineFx>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut player: Query<(&Transform, &DriveState, &mut EngineState), With<Player>>,
    mut flames: Query<(&mut Transform, &ExhaustFlame), Without<Player>>,
    mut particles: Query<(&mut Transform, &mut Visibility, &mut EngineParticle), (Without<Player>, Without<ExhaustFlame>)>,
) {
    let (Some(fx), Ok((ship, drive, mut engine))) = (fx, player.single_mut()) else { return };
    let dt = time.delta_secs();
    // Idle glow when parked, stronger with speed, full burn while accelerating.
    let speed_ratio = (drive.speed.abs() / crate::systems::player::MAX_SPEED).clamp(0.0, 1.0);
    let target = (0.12 + 0.45 * speed_ratio).max(drive.throttle.max(0.0));
    engine.power += (target - engine.power) * (dt * 5.0).min(1.0);
    let power = engine.power;

    // A soft shimmer keeps idle engines alive; it fades out as power rises.
    let t = time.elapsed_secs();
    let shimmer = 1.0 + (1.0 - power) * 0.18 * (t * 5.3).sin() * (t * 1.7).cos();
    if let Some(mut material) = materials.get_mut(&fx.glow) {
        material.emissive = ENGINE_GLOW * ((GLOW_IDLE + GLOW_BOOST * power) * shimmer);
    }
    if let Some(mut material) = materials.get_mut(&fx.hover) {
        material.emissive = LinearRgba::rgb(1.2, 0.4, 2.6) * (0.8 + 0.25 * (t * 2.1).sin() + 0.6 * speed_ratio);
    }
    for (mut flame, exhaust) in &mut flames {
        let length = (FLAME_IDLE + FLAME_BOOST * power) * shimmer;
        let girth = 0.75 + 0.45 * power;
        flame.scale = Vec3::new(girth, length, girth);
        flame.translation.z = exhaust.base_z + length * 0.5;
    }

    engine.emit_accumulator += PARTICLE_RATE * power * power * dt;
    while engine.emit_accumulator >= 1.0 {
        engine.emit_accumulator -= 1.0;
        let slot = engine.next_particle;
        engine.next_particle = (slot + 1) % fx.particles.len().max(1);
        let Some(&entity) = fx.particles.get(slot) else { break };
        let Ok((mut transform, mut visibility, mut particle)) = particles.get_mut(entity) else { continue };
        let jitter = Vec3::new(hash01(slot as u32 * 3) - 0.5, hash01(slot as u32 * 3 + 1) - 0.5, 0.0);
        let (centre, radius) = EXHAUSTS[slot % EXHAUSTS.len()];
        let exhaust = centre + Vec3::Z * 0.3 + jitter * radius * 1.6;
        transform.translation = ship.transform_point(exhaust);
        transform.scale = Vec3::ONE;
        *visibility = Visibility::Visible;
        // Particles lag behind the ship and drift backwards, leaving a short trail.
        particle.velocity = *ship.back() * (3.0 + 6.0 * power) + *ship.forward() * drive.speed * 0.35 + ship.rotation * jitter * 1.2;
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

    /// Joins consecutive rings (same vertex count) with quads and caps both ends.
    fn loft(&mut self, rings: &[Vec<Vec3>]) {
        let centre = |ring: &[Vec3]| ring.iter().copied().sum::<Vec3>() / ring.len() as f32;
        for pair in rings.windows(2) {
            let (a, b) = (&pair[0], &pair[1]);
            let inside = (centre(a) + centre(b)) * 0.5;
            for i in 0..a.len() {
                let j = (i + 1) % a.len();
                self.tri(a[i], a[j], b[j], inside);
                self.tri(a[i], b[j], b[i], inside);
            }
        }
        for (ring, toward) in [(&rings[0], &rings[1]), (&rings[rings.len() - 1], &rings[rings.len() - 2])] {
            let (c, inside) = (centre(ring), centre(toward));
            for i in 0..ring.len() {
                self.tri(ring[i], ring[(i + 1) % ring.len()], c, inside);
            }
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

/// Hexagonal cross-section: flat belly, angled flanks, flat deck.
/// (z, half width at flank, belly half width, deck half width, belly y, flank y, deck y)
type Section = (f32, f32, f32, f32, f32, f32, f32);

fn hex_ring(&(z, w, bw, tw, b, m, t): &Section, x: f32) -> Vec<Vec3> {
    [(bw, b), (w, m), (tw, t), (-tw, t), (-w, m), (-bw, b)].iter().map(|&(px, py)| Vec3::new(x + px, py, z)).collect()
}

/// Wide, flat wedge body with a chisel nose (no point, no fuselage tube).
fn hull_mesh() -> Mesh {
    const SECTIONS: [Section; 5] = [
        (-3.6, 0.7, 0.5, 0.4, -0.12, -0.06, 0.0),
        (-2.0, 1.4, 1.0, 0.8, -0.26, -0.05, 0.28),
        (0.0, 1.75, 1.3, 1.0, -0.32, 0.0, 0.48),
        (2.0, 1.8, 1.35, 1.15, -0.32, 0.05, 0.5),
        (3.0, 1.6, 1.2, 1.0, -0.26, 0.05, 0.42),
    ];
    let mut mesh = Faceted::default();
    mesh.loft(&SECTIONS.iter().map(|s| hex_ring(s, 0.0)).collect::<Vec<_>>());
    mesh.build()
}

/// Low, angular canopy sunk into the deck: a sloped faceted slab, not a bubble.
fn cockpit_mesh() -> Mesh {
    const SECTIONS: [Section; 3] = [
        (-1.9, 0.35, 0.3, 0.12, 0.2, 0.24, 0.32),
        (-0.8, 0.62, 0.55, 0.38, 0.38, 0.46, 0.66),
        (0.4, 0.6, 0.55, 0.42, 0.44, 0.5, 0.64),
    ];
    let mut mesh = Faceted::default();
    mesh.loft(&SECTIONS.iter().map(|s| hex_ring(s, 0.0)).collect::<Vec<_>>());
    mesh.build()
}

/// Side engine pods: octagonal nacelles that flare into the rear propulsion
/// units, joined to the hull by flat struts.
fn pods_mesh() -> Mesh {
    // (z, radius); the vertical radius is squashed so pods stay low.
    const PROFILE: [(f32, f32); 5] = [(-2.0, 0.18), (-1.2, 0.45), (0.5, 0.55), (2.3, 0.6), (3.05, POD_RADIUS)];
    let mut mesh = Faceted::default();
    for side in [-1.0, 1.0] {
        let rings: Vec<Vec<Vec3>> = PROFILE
            .iter()
            .map(|&(z, r)| (0..8).map(|k| {
                let angle = (k as f32 + 0.5) * std::f32::consts::TAU / 8.0;
                Vec3::new(side * POD_X + angle.cos() * r, angle.sin() * r * 0.75, z)
            }).collect())
            .collect();
        mesh.loft(&rings);
        // Strut: a flat swept plate between flank and pod.
        let strut = [Vec2::new(1.55, -0.6), Vec2::new(POD_X, -0.9), Vec2::new(POD_X, 2.2), Vec2::new(1.6, 2.4)];
        let polygon: Vec<Vec2> = strut.iter().map(|p| Vec2::new(p.x * side, p.y)).collect();
        mesh.prism(&polygon, Vec3::new(0.0, 0.05, 0.0), Vec3::X, Vec3::Z, 0.08);
    }
    mesh.build()
}

/// Geometric armor plates layered on the deck and nose.
fn armor_mesh() -> Mesh {
    let mut mesh = Faceted::default();
    for side in [-1.0, 1.0] {
        // Swept intake plates either side of the cockpit.
        let intake = [Vec2::new(0.75, -1.7), Vec2::new(1.45, -1.2), Vec2::new(1.55, 0.9), Vec2::new(0.8, 0.6)];
        let polygon: Vec<Vec2> = intake.iter().map(|p| Vec2::new(p.x * side, p.y)).collect();
        mesh.prism(&polygon, Vec3::new(0.0, 0.5, 0.0), Vec3::X, Vec3::Z, 0.06);
        // Rear deck plates over the engine bay.
        let rear = [Vec2::new(0.2, 1.0), Vec2::new(1.1, 1.2), Vec2::new(1.05, 2.8), Vec2::new(0.25, 2.8)];
        let polygon: Vec<Vec2> = rear.iter().map(|p| Vec2::new(p.x * side, p.y)).collect();
        mesh.prism(&polygon, Vec3::new(0.0, 0.55, 0.0), Vec3::X, Vec3::Z, 0.05);
    }
    // Front splitter: a thin blade under the chisel nose.
    let splitter = [Vec2::new(-1.0, -3.9), Vec2::new(1.0, -3.9), Vec2::new(1.5, -2.6), Vec2::new(-1.5, -2.6)];
    mesh.prism(&splitter, Vec3::new(0.0, -0.18, 0.0), Vec3::X, Vec3::Z, 0.04);
    mesh.build()
}
