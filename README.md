# rust-3D-race-game
# Neon Horizon // Vaporwave Racer

A small procedural 3D arcade racing prototype built with Rust stable and Bevy 0.19.

Project documentation lives in [`docs/`](docs/README.md).

## Run

```bash
cargo check
cargo run
```

The project uses `1 Bevy world unit ≈ 1 meter`. Drive with `WASD` or the arrow keys. Press `R` after finishing to restart.

## Architecture

`GamePlugin` owns the state machine (`Loading → Countdown → Racing → Finished`). `PlayerPlugin` builds the low-poly car and applies arcade movement. `TrackPlugin` creates the road, neon boundaries, and finish gate. `RacePlugin` owns the countdown, clock, restart input, and one-lap gate logic. `CameraPlugin` smoothly follows the player. `UiPlugin` renders the HUD and result panel. `EnvironmentPlugin` creates the moon, lighting, grid, pillars, and buildings.

The car accelerates toward a capped forward speed, brakes/reverses at a lower cap, and loses speed when the throttle is released. Steering rotates the car more strongly as speed increases. A simple rectangular ring test keeps the car on the road and bounces it back from invalid space.

The finish system arms after the car reaches the far side of the first straight, then detects the next left-to-right crossing of the start gate. This is intentionally a one-lap-friendly foundation for later checkpoints and lap counters.

## Replacing procedural content with GLB assets

Add a GLB to `assets/`, load it with `AssetServer`, and replace the relevant procedural spawn function with a scene instance (`SceneRoot`/`Scene` workflow in Bevy). Keep the `Player`, `TrackMarker`, and `FinishLine` gameplay entities as separate markers or child entities so movement and race logic do not depend on mesh details.
