pub mod animation;
pub mod backdrop;
pub mod environment;
pub mod lighting;
pub use animation::AnimationPlugin;
pub use backdrop::BackdropPlugin;
pub use environment::EnvironmentPlugin;
pub use lighting::TrackLightingPlugin;

/// Cheap deterministic pseudo-random value in `0..1`, so procedural placement
/// and effects look varied but identical on every run.
pub fn hash01(n: u32) -> f32 {
    let mut x = n.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    (x & 0x00FF_FFFF) as f32 / 0x0100_0000 as f32
}
