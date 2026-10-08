# Getting started

## Requirements

- Rust stable with Cargo.
- A graphics-capable desktop environment supported by Bevy 0.19.
- System libraries required by Bevy's windowing and audio backends for your operating system.

The repository pins Bevy to the 0.19 line in `Cargo.toml`. `Cargo.lock` is committed because this is an executable application and reproducible dependency resolution is useful.

## Run and verify

From the project root:

```bash
cargo check
cargo run
```

The first command validates the project without opening a window. The second launches the game.

## Controls

| Input | Action |
| --- | --- |
| `W` / `Arrow Up` | Accelerate |
| `S` / `Arrow Down` | Brake or reverse |
| `A` / `Arrow Left` | Steer left |
| `D` / `Arrow Right` | Steer right |
| `R` | Restart after finishing |
| `Escape` | Close the window using the platform window control |

## Troubleshooting

If Cargo reports a missing system dependency, install the Bevy prerequisites for your operating system and run `cargo check` again. Procedural geometry is used for the initial milestone, so no files are required in `assets/` to launch the game.
