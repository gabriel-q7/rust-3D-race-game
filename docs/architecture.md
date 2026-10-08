# Architecture and ECS design

## Startup flow

`src/main.rs` creates the Bevy `App`, configures the primary window, and registers `GamePlugin`. `GamePlugin` initializes resources and composes the feature plugins:

| Plugin | Responsibility |
| --- | --- |
| `EnvironmentPlugin` | Moon, lighting, floor, grid, pillars, and buildings. |
| `TrackPlugin` | Procedural road, neon edges, start/finish gate, and track bounds. |
| `PlayerPlugin` | Car hierarchy, input-driven movement, and countdown reset. |
| `CameraPlugin` | Third-person camera spawning and smoothing. |
| `RacePlugin` | Loading, countdown, race clock, finish detection, and restart input. |
| `UiPlugin` | Countdown, timer, speed readout, and result panel. |

## Game states

The `GameState` state machine is deliberately small:

```text
Loading -> Countdown -> Racing -> Finished
                         ^          |
                         +---- R ---+
```

`Loading` lasts one update while the procedural world is already being created during `Startup`. Entering `Countdown` resets the car and countdown resource. Entering `Racing` starts the race timer. Entering `Finished` leaves the timer stopped and exposes the result panel.

## Components and resources

- `Player` identifies the controllable car root entity.
- `DriveState` stores current speed and the previous position used by the finish gate.
- `TrackMarker` and `FinishLine` identify procedural track entities for future replacement or effects.
- `FollowCamera` identifies the chase camera.
- `GameState` is a Bevy `States` resource.
- `TrackBounds` stores the outer and inner dimensions of the rectangular road ring.
- `Countdown`, `RaceTimer`, and `RaceProgress` hold global race flow data.

Systems communicate through these components and resources rather than global mutable variables. Rendering entities are spawned once, while state-gated systems handle the changing gameplay.
