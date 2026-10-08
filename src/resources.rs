use bevy::prelude::*;

#[derive(States, Default, Debug, Clone, Eq, PartialEq, Hash)]
pub enum GameState {
    #[default]
    Loading,
    Countdown,
    Racing,
    Finished,
}

#[derive(Resource)]
pub struct Countdown {
    pub timer: Timer,
    pub step: i8,
}

impl Default for Countdown {
    fn default() -> Self {
        Self { timer: Timer::from_seconds(1.0, TimerMode::Repeating), step: 3 }
    }
}

#[derive(Resource, Default)]
pub struct RaceTimer {
    pub elapsed: f32,
    pub running: bool,
}

#[derive(Resource, Default)]
pub struct RaceProgress {
    pub armed: bool,
}

#[derive(Resource, Clone)]
pub struct TrackPath {
    pub points: Vec<Vec3>,
    pub width: f32,
}

impl TrackPath {
    pub const SAMPLES_PER_SECTION: usize = 6;

    /// Catmull-Rom samples are shared by rendering and driving, so the road
    /// and the car always agree on the same smooth centerline.
    pub fn smooth_points(&self) -> Vec<Vec3> {
        let count = self.points.len();
        let mut samples = Vec::with_capacity(count * Self::SAMPLES_PER_SECTION);
        for i in 0..count {
            let p0 = self.points[(i + count - 1) % count];
            let p1 = self.points[i];
            let p2 = self.points[(i + 1) % count];
            let p3 = self.points[(i + 2) % count];
            for step in 0..Self::SAMPLES_PER_SECTION {
                let t = step as f32 / Self::SAMPLES_PER_SECTION as f32;
                let t2 = t * t;
                let t3 = t2 * t;
                samples.push(0.5 * ((2.0 * p1)
                    + (-p0 + p2) * t
                    + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
                    + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3));
            }
        }
        samples
    }

    /// Returns the closest point and tangent on the smoothed 3D centerline.
    pub fn closest_point(&self, position: Vec3) -> (Vec3, Vec3) {
        let samples = self.smooth_points();
        let mut best_point = samples[0];
        let mut best_tangent = (samples[1] - samples[0]).normalize();
        let mut best_distance = f32::MAX;
        for i in 0..samples.len() {
            let a = samples[i];
            let b = samples[(i + 1) % samples.len()];
            let segment = b - a;
            let fraction = ((position - a).dot(segment) / segment.length_squared()).clamp(0.0, 1.0);
            let point = a + segment * fraction;
            let distance = position.distance_squared(point);
            if distance < best_distance {
                best_distance = distance;
                best_point = point;
                best_tangent = segment.normalize();
            }
        }
        (best_point, best_tangent)
    }
}

impl Default for TrackPath {
    fn default() -> Self {
        // The route deliberately revisits similar X/Z positions at different
        // heights, creating bridges, underpasses, and genuine 3D crossings.
        Self {
            width: 9.0,
            points: vec![
                Vec3::new(0.0, 1.0, -62.0), Vec3::new(0.0, 1.0, -35.0),
                Vec3::new(18.0, 4.0, -20.0), Vec3::new(44.0, 4.0, -20.0),
                Vec3::new(56.0, 11.0, -4.0), Vec3::new(56.0, 11.0, 28.0),
                Vec3::new(38.0, 16.0, 47.0), Vec3::new(4.0, 16.0, 47.0),
                Vec3::new(-14.0, 12.0, 28.0), Vec3::new(-43.0, 9.0, 10.0),
                Vec3::new(16.0, 9.0, 10.0), Vec3::new(35.0, 3.0, -4.0),
                Vec3::new(35.0, 3.0, -34.0), Vec3::new(12.0, -3.0, -49.0),
                Vec3::new(-20.0, -3.0, -49.0), Vec3::new(-34.0, -3.0, -29.0),
                Vec3::new(16.0, -3.0, 10.0), Vec3::new(-19.0, -3.0, 10.0),
                Vec3::new(-42.0, 1.0, -8.0), Vec3::new(-42.0, 1.0, -43.0),
                Vec3::new(-20.0, 1.0, -62.0),
            ],
        }
    }
}
