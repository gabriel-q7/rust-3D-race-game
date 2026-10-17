use bevy::{asset::RenderAssetUsages, math::primitives::Cuboid, mesh::{Indices, PrimitiveTopology}, prelude::*};

use crate::{components::{FinishLine, TrackMarker}, resources::{TrackFrame, TrackPath}};

pub const ROAD_THICKNESS: f32 = 0.35;
const RAIL_WIDTH: f32 = 0.22;
const RAIL_HEIGHT: f32 = 0.55;

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
    commands.spawn((Mesh3d(meshes.add(extrude(&path.frames, &rect(-half, half, -ROAD_THICKNESS, 0.0)))), MeshMaterial3d(road), TrackMarker));
    for (offset, material) in [(-1.0, cyan.clone()), (1.0, pink)] {
        let x = offset * half;
        let profile = rect(x - RAIL_WIDTH * 0.5, x + RAIL_WIDTH * 0.5, 0.0, RAIL_HEIGHT);
        commands.spawn((Mesh3d(meshes.add(extrude(&path.frames, &profile))), MeshMaterial3d(material), TrackMarker));
    }

    // Repeated overhead frames make the high sections read as bridges/tunnels.
    let frame_mesh = meshes.add(Cuboid::new(path.width + 1.0, 0.25, 0.25));
    for frame in path.frames.iter().step_by((8.0 / TrackPath::FRAME_SPACING) as usize).filter(|frame| frame.position.y > 8.0) {
        commands.spawn((Mesh3d(frame_mesh.clone()), MeshMaterial3d(cyan.clone()), Transform { translation: frame.position + frame.normal * 4.0, rotation: frame_rotation(frame), ..default() }, TrackMarker));
    }

    let line_material = materials.add(StandardMaterial { base_color: Color::WHITE, emissive: LinearRgba::new(1.0, 0.3, 0.8, 1.0), ..default() });
    let start = path.frames[0];
    let gate_rotation = frame_rotation(&start);
    let line_mesh = meshes.add(Cuboid::new(path.width, 0.04, 0.7));
    commands.spawn((Mesh3d(line_mesh), MeshMaterial3d(line_material), Transform { translation: start.position + start.normal * 0.02, rotation: gate_rotation, ..default() }, FinishLine));

    // Small start markers make the gate legible from the chase camera.
    let marker = meshes.add(Cuboid::new(0.35, 4.0, 0.35));
    for side in [-1.0, 1.0] {
        commands.spawn((Mesh3d(marker.clone()), MeshMaterial3d(cyan.clone()), Transform { translation: start.position + start.side * side * path.width * 0.5 + Vec3::Y * 2.0, rotation: gate_rotation, ..default() }, TrackMarker));
    }
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
            let v = ring as f32 * TrackPath::FRAME_SPACING / 8.0;
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
