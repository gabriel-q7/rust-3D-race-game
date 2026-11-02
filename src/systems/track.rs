use bevy::{asset::RenderAssetUsages, math::primitives::Cuboid, mesh::{Indices, PrimitiveTopology}, prelude::*};

use crate::{components::{FinishLine, TrackMarker}, resources::{TrackFrame, TrackPath}, world::animation::Pulsating};

const RAIL_WIDTH: f32 = 0.5;
const RAIL_HEIGHT: f32 = 0.9;
/// Overhead gantries on high sections: spacing along the road, clearance above it,
/// and the minimum height above ground where they appear.
const GANTRY_SPACING: f32 = 30.0;
const GANTRY_CLEARANCE: f32 = 8.0;
const GANTRY_MIN_HEIGHT: f32 = 24.0;

pub struct TrackPlugin;

impl Plugin for TrackPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TrackPath::default()).add_systems(Startup, spawn_track);
    }
}

fn spawn_track(
    mut commands: Commands,
    path: Res<TrackPath>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let road = materials.add(Color::srgb(0.055, 0.035, 0.12));
    let cyan = materials.add(StandardMaterial { base_color: Color::srgb(0.02, 0.8, 1.0), emissive: LinearRgba::new(0.0, 1.0, 1.0, 1.0), ..default() });
    let pink = materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.03, 0.45), emissive: LinearRgba::new(1.0, 0.0, 0.25, 1.0), ..default() });

    // The whole loop is one continuous extrusion, so there are no seams or
    // steps between segments for the car to bump into.
    let half = path.width * 0.5;
    commands.spawn((Mesh3d(meshes.add(extrude(&path.frames, &rect(-half, half, -TrackPath::ROAD_THICKNESS, 0.0)))), MeshMaterial3d(road), TrackMarker));
    for (offset, material) in [(-1.0, cyan.clone()), (1.0, pink)] {
        let x = offset * half;
        let profile = rect(x - RAIL_WIDTH * 0.5, x + RAIL_WIDTH * 0.5, 0.0, RAIL_HEIGHT);
        commands.spawn((Mesh3d(meshes.add(extrude(&path.frames, &profile))), MeshMaterial3d(material), TrackMarker));
    }

    // Repeated overhead gantries make the high sections read as bridges.
    let gantry = meshes.add(Cuboid::new(path.width + 2.0 * RAIL_WIDTH, 0.4, 0.4));
    for frame in gantry_frames(&path) {
        commands.spawn((Mesh3d(gantry.clone()), MeshMaterial3d(cyan.clone()), Transform { translation: frame.position + frame.normal * GANTRY_CLEARANCE, rotation: frame_rotation(frame), ..default() }, TrackMarker));
    }

    let line_material = materials.add(StandardMaterial { base_color: Color::WHITE, emissive: LinearRgba::new(1.0, 0.3, 0.8, 1.0), ..default() });
    let start = path.frames[0];
    let gate_rotation = frame_rotation(&start);
    let line_mesh = meshes.add(Cuboid::new(path.width, 0.04, 1.4));
    commands.spawn((Mesh3d(line_mesh), MeshMaterial3d(line_material), Transform { translation: start.position + start.normal * 0.02, rotation: gate_rotation, ..default() }, FinishLine));

    // Small start markers make the gate legible from the chase camera.
    let marker = meshes.add(Cuboid::new(0.6, 14.0, 0.6));
    for side in [-1.0, 1.0] {
        commands.spawn((Mesh3d(marker.clone()), MeshMaterial3d(cyan.clone()), Transform { translation: start.position + start.side * side * (path.width * 0.5 + RAIL_WIDTH + 0.4) + Vec3::Y * 7.0, rotation: gate_rotation, ..default() }, TrackMarker));
    }
    let banner = meshes.add(Cuboid::new(path.width + 2.0 * RAIL_WIDTH + 1.4, 0.35, 0.35));
    commands.spawn((Mesh3d(banner), MeshMaterial3d(pink_banner(&mut materials)), Transform { translation: start.position + Vec3::Y * 14.0, rotation: gate_rotation, ..default() }, TrackMarker, Pulsating::new(0.45, 0.8, 0.0)));
}

/// Track samples that carry an overhead gantry (high sections only).
pub fn gantry_frames(path: &TrackPath) -> impl Iterator<Item = &TrackFrame> {
    path.frames.iter().step_by((GANTRY_SPACING / TrackPath::FRAME_SPACING) as usize).filter(|frame| frame.position.y - TrackPath::GROUND_TOP > GANTRY_MIN_HEIGHT)
}

/// Own material so the start banner can pulse without affecting the rails.
fn pink_banner(materials: &mut Assets<StandardMaterial>) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial { base_color: Color::srgb(1.0, 0.03, 0.45), emissive: LinearRgba::new(2.0, 0.0, 0.6, 1.0), ..default() })
}

/// Rotation that maps local X to the frame side, Y to its normal and -Z to its tangent.
fn frame_rotation(frame: &TrackFrame) -> Quat {
    Transform::IDENTITY.looking_to(frame.tangent, frame.normal).rotation
}

/// Closed counter-clockwise rectangle in (side, normal) profile space.
fn rect(left: f32, right: f32, bottom: f32, top: f32) -> [Vec2; 4] {
    [Vec2::new(left, bottom), Vec2::new(right, bottom), Vec2::new(right, top), Vec2::new(left, top)]
}

/// Sweeps a closed 2D profile along the looping frames. Every profile edge gets
/// its own vertices so the sides stay crisp while the length stays smooth.
fn extrude(frames: &[TrackFrame], profile: &[Vec2]) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let rings = frames.len() + 1; // Repeat the first frame to close the loop.

    for edge in 0..profile.len() {
        let (p, q) = (profile[edge], profile[(edge + 1) % profile.len()]);
        let outward = Vec2::new(q.y - p.y, p.x - q.x).normalize();
        let base = positions.len() as u32;
        for ring in 0..rings {
            let frame = &frames[ring % frames.len()];
            let to_world = |v: Vec2| frame.position + frame.side * v.x + frame.normal * v.y;
            let normal = (frame.side * outward.x + frame.normal * outward.y).to_array();
            let v = ring as f32 * TrackPath::FRAME_SPACING / 24.0;
            positions.extend([to_world(p).to_array(), to_world(q).to_array()]);
            normals.extend([normal, normal]);
            uvs.extend([[0.0, v], [1.0, v]]);
        }
        for ring in 0..rings as u32 - 1 {
            let (a, b, c, d) = (base + ring * 2, base + ring * 2 + 1, base + ring * 2 + 2, base + ring * 2 + 3);
            // Pick the winding that faces outward so back-face culling keeps the right side.
            let [pa, pb, pc] = [a, b, c].map(|i| Vec3::from(positions[i as usize]));
            if (pb - pa).cross(pc - pa).dot(Vec3::from(normals[a as usize])) >= 0.0 {
                indices.extend([a, b, c, b, d, c]);
            } else {
                indices.extend([a, c, b, b, c, d]);
            }
        }
    }

    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices))
}
