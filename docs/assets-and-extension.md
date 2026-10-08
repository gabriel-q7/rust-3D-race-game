# Assets and future extension

## Replacing the procedural car

1. Put a `.glb` or `.gltf` file under `assets/`.
2. Load it with Bevy's `AssetServer` and request its scene handle.
3. Spawn the scene as a child of the existing `Player` root entity, or replace the primitive children inside `spawn_player`.
4. Keep `Player` and `DriveState` on the root entity. Movement and camera code should continue to target that root rather than individual mesh parts.
5. Adjust the model's local forward direction and scale if the imported asset does not face Bevy's forward axis.

## Replacing the procedural track

The gameplay code only needs a player root, track bounds, and a finish trigger. A GLB track can therefore replace the meshes created by `spawn_track` while retaining:

- `TrackBounds` or a future collection of checkpoint colliders for gameplay constraints.
- A visible `FinishLine` entity or marker at the finish location.
- `TrackMarker` on track-owned entities if later systems need to find them.

For a non-rectangular track, replace the ring test with a centerline, waypoint, or collider-based validity test without changing the race state machine.

## Safe extension points

- Add `Checkpoint` entities and ordered progress to support laps.
- Add an `InputState` resource if multiple control schemes are introduced.
- Add an `AudioPlugin` for engine, UI, and music events.
- Add an `Opponent` component and a separate AI movement system.
- Add a `BestTime` resource and persistence only after the core loop is stable.

Keep visual scene entities separate from gameplay marker entities. That separation makes it possible to change Blender models or materials without rewriting movement and race logic.
