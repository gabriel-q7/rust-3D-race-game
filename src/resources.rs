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

/// One sample of the road centerline with its local orientation frame.
#[derive(Clone, Copy)]
pub struct TrackFrame {
    pub position: Vec3,
    pub tangent: Vec3,
    /// Points to the driver's right, always horizontal so the road never banks.
    pub side: Vec3,
    /// Road surface normal (up for the car).
    pub normal: Vec3,
}

impl TrackFrame {
    fn new(position: Vec3, velocity: Vec3) -> Self {
        let tangent = velocity.normalize();
        let side = tangent.cross(Vec3::Y).normalize();
        let normal = side.cross(tangent).normalize();
        Self { position, tangent, side, normal }
    }

    fn lerp(&self, other: &Self, t: f32) -> Self {
        let position = self.position.lerp(other.position, t);
        Self::new(position, self.tangent.lerp(other.tangent, t))
    }
}

/// Result of projecting a world position onto the track.
pub struct TrackHit {
    pub frame: TrackFrame,
    /// Index of the sample that starts the closest segment.
    pub index: usize,
}

#[derive(Resource, Clone)]
pub struct TrackPath {
    pub width: f32,
    /// Dense frames sampled once from the spline; shared by rendering and
    /// driving so the road mesh and the car agree on the same surface.
    pub frames: Vec<TrackFrame>,
}

impl TrackPath {
    /// Height of the ground plane's top face; no part of the road may dip below it.
    pub const GROUND_TOP: f32 = -0.25;
    /// Minimum gap between the road's underside and the ground.
    const GROUND_CLEARANCE: f32 = 0.5;
    pub const ROAD_THICKNESS: f32 = 0.35;
    /// Road width; wide enough for side-by-side manoeuvring with the ship.
    pub const WIDTH: f32 = 27.0;
    /// The hand-written layout is authored small and scaled up: XZ spreads the
    /// circuit out, a smaller height scale keeps slopes gentle while still
    /// leaving generous clearance where sections cross.
    const LAYOUT_SCALE: f32 = 3.0;
    const HEIGHT_SCALE: f32 = 1.6;
    /// Target radius for rounded corners, measured on the centerline.
    pub const CORNER_RADIUS: f32 = 36.0;
    /// Distance between spline control points after resampling.
    const CONTROL_SPACING: f32 = 3.0;
    /// Spline samples per control segment; frames end up ~1 unit apart.
    const SAMPLES_PER_SEGMENT: usize = 3;
    pub const FRAME_SPACING: f32 = Self::CONTROL_SPACING / Self::SAMPLES_PER_SEGMENT as f32;
    /// How many samples around the previous hit are searched each frame.
    const SEARCH_WINDOW: usize = 16;

    /// Builds the drivable loop from rough layout points:
    /// 1. every corner is replaced by a circular arc (fillet),
    /// 2. the result is resampled at even spacing,
    /// 3. a cyclic Catmull-Rom spline through those samples gives the final frames,
    /// 4. the whole loop is lifted, if needed, so it never sinks into the ground.
    /// Even spacing is what keeps Catmull-Rom from overshooting into kinks.
    pub fn new(points: Vec<Vec3>, width: f32) -> Self {
        let control = resample_loop(&fillet_corners(&points, Self::CORNER_RADIUS), Self::CONTROL_SPACING);
        let curve = CubicCardinalSpline::new_catmull_rom(control.iter().copied())
            .to_curve_cyclic()
            .expect("track needs at least two control points");
        let count = control.len() * Self::SAMPLES_PER_SEGMENT;
        let mut frames: Vec<TrackFrame> = (0..count)
            .map(|i| {
                let t = i as f32 / Self::SAMPLES_PER_SEGMENT as f32;
                TrackFrame::new(curve.position(t), curve.velocity(t))
            })
            .collect();
        // Lifting the whole loop keeps every crossing's relative headroom intact.
        let lowest = frames.iter().map(|frame| frame.position.y).fold(f32::MAX, f32::min);
        let lift = (Self::GROUND_TOP + Self::GROUND_CLEARANCE + Self::ROAD_THICKNESS - lowest).max(0.0);
        for frame in &mut frames {
            frame.position.y += lift;
        }
        Self { width, frames }
    }

    /// Horizontal distance from `point` to the nearest centerline sample. Used at
    /// startup to keep decorations off the road; not meant for per-frame use.
    pub fn horizontal_clearance(&self, point: Vec3) -> f32 {
        self.frames.iter().map(|frame| horizontal(frame.position - point).length()).fold(f32::MAX, f32::min)
    }

    /// Projects `position` onto the centerline. With a `hint` (the previous
    /// hit index) only nearby samples are searched, so stacked sections such
    /// as bridges and underpasses never steal the car.
    pub fn closest(&self, position: Vec3, hint: Option<usize>) -> TrackHit {
        let count = self.frames.len();
        let candidates: Box<dyn Iterator<Item = usize>> = match hint {
            Some(hint) => Box::new((0..=Self::SEARCH_WINDOW * 2).map(move |o| (hint + count + o - Self::SEARCH_WINDOW) % count)),
            None => Box::new(0..count),
        };
        let mut best = TrackHit { frame: self.frames[0], index: 0 };
        let mut best_distance = f32::MAX;
        for i in candidates {
            let a = &self.frames[i];
            let b = &self.frames[(i + 1) % count];
            let segment = b.position - a.position;
            let fraction = ((position - a.position).dot(segment) / segment.length_squared()).clamp(0.0, 1.0);
            let frame = a.lerp(b, fraction);
            let distance = position.distance_squared(frame.position);
            if distance < best_distance {
                best_distance = distance;
                best = TrackHit { frame, index: i };
            }
        }
        best
    }
}

impl Default for TrackPath {
    fn default() -> Self {
        // The route deliberately revisits similar X/Z positions at different
        // heights, creating bridges, underpasses, and genuine 3D crossings.
        let layout = vec![
            Vec3::new(0.0, 1.0, -62.0), Vec3::new(0.0, 1.0, -35.0),
            Vec3::new(18.0, 8.0, -20.0), Vec3::new(44.0, 8.0, -20.0),
            Vec3::new(56.0, 11.0, -4.0), Vec3::new(56.0, 11.0, 28.0),
            Vec3::new(38.0, 16.0, 47.0), Vec3::new(4.0, 16.0, 47.0),
            Vec3::new(-14.0, 12.0, 28.0), Vec3::new(-36.0, 11.0, 33.0),
            Vec3::new(-60.0, 10.0, 32.0), Vec3::new(-62.0, 10.0, 10.0),
            Vec3::new(-38.0, 9.0, 10.0),
            Vec3::new(16.0, 9.0, 10.0), Vec3::new(35.0, 3.0, -4.0),
            Vec3::new(35.0, 3.0, -34.0), Vec3::new(12.0, -4.0, -49.0),
            Vec3::new(-16.0, -4.0, -52.0), Vec3::new(-34.0, -3.0, -29.0),
            Vec3::new(4.0, -3.0, -14.0), Vec3::new(28.0, -3.0, -10.0),
            Vec3::new(30.0, -3.0, 10.0), Vec3::new(10.0, -3.0, 13.0),
            Vec3::new(-19.0, -3.0, 10.0),
            Vec3::new(-42.0, 1.0, -8.0), Vec3::new(-42.0, 1.0, -43.0),
            Vec3::new(-20.0, 1.0, -62.0),
        ];
        let scale = Vec3::new(Self::LAYOUT_SCALE, Self::HEIGHT_SCALE, Self::LAYOUT_SCALE);
        Self::new(layout.into_iter().map(|point| point * scale).collect(), Self::WIDTH)
    }
}

fn horizontal(v: Vec3) -> Vec3 { Vec3::new(v.x, 0.0, v.z) }

/// Replaces each layout corner with a circular arc in the XZ plane, blending
/// height linearly. The returned loop starts where the first corner's arc ends,
/// so the start line sits on a straight.
fn fillet_corners(points: &[Vec3], radius: f32) -> Vec<Vec3> {
    let count = points.len();
    let mut out = Vec::new();
    let mut first_arc_end = 0;
    for i in 0..count {
        if i == 1 { first_arc_end = out.len() - 1; }
        let (a, b, c) = (points[(i + count - 1) % count], points[i], points[(i + 1) % count]);
        let (in_leg, out_leg) = (horizontal(b - a), horizontal(c - b));
        let (d1, d2) = (in_leg.normalize(), out_leg.normalize());
        let turn = d1.angle_between(d2);
        if turn < 1f32.to_radians() {
            out.push(b);
            continue;
        }
        // Each corner may use at most half of each leg, so neighbours never overlap.
        let tangent_length = (radius * (turn * 0.5).tan()).min(in_leg.length().min(out_leg.length()) * 0.5);
        let arc_radius = tangent_length / (turn * 0.5).tan();
        let start = b - d1 * tangent_length;
        let start_y = b.y.lerp(a.y, tangent_length / in_leg.length());
        let end_y = b.y.lerp(c.y, tangent_length / out_leg.length());
        let towards_center = (d2 - d1 * d1.dot(d2)).normalize();
        let center = horizontal(start) + towards_center * arc_radius;
        let steps = (turn.to_degrees() / 5.0).ceil().max(1.0) as usize;
        for k in 0..=steps {
            let t = k as f32 / steps as f32;
            let angle = turn * t;
            let point = center - towards_center * arc_radius * angle.cos() + d1 * arc_radius * angle.sin();
            out.push(Vec3::new(point.x, start_y.lerp(end_y, t), point.z));
        }
    }
    // Rotate so the loop begins at the end of corner 0's arc.
    out.rotate_left(first_arc_end);
    out
}

/// Walks the closed polyline and emits points every `spacing` units of length.
fn resample_loop(polyline: &[Vec3], spacing: f32) -> Vec<Vec3> {
    let count = polyline.len();
    let total: f32 = (0..count).map(|i| polyline[i].distance(polyline[(i + 1) % count])).sum();
    let samples = (total / spacing).round().max(4.0) as usize;
    let step = total / samples as f32;
    let mut out = Vec::with_capacity(samples);
    let (mut segment, mut walked) = (0, 0.0);
    for n in 0..samples {
        let target = n as f32 * step;
        loop {
            let (a, b) = (polyline[segment], polyline[(segment + 1) % count]);
            let length = a.distance(b);
            if walked + length >= target || segment == count - 1 {
                out.push(a.lerp(b, ((target - walked) / length.max(f32::EPSILON)).clamp(0.0, 1.0)));
                break;
            }
            walked += length;
            segment += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimum vertical gap between stacked road sections (surface to surface):
    /// hover height + ship height + road thickness, with room to spare.
    const MIN_HEADROOM: f32 = 6.5;

    #[test]
    fn corners_are_wide_enough_for_the_road() {
        let path = TrackPath::default();
        let count = path.frames.len();
        for i in 0..count {
            // Measure over a few samples so float noise on tiny steps doesn't dominate.
            let (a, b) = (path.frames[i], path.frames[(i + 4) % count]);
            let length: f32 = (i..i + 4).map(|j| horizontal(path.frames[j % count].position - path.frames[(j + 1) % count].position).length()).sum();
            let turn = horizontal(a.tangent).angle_between(horizontal(b.tangent)).max(1e-6);
            let radius = length / turn;
            assert!(radius > path.width, "turn radius {radius:.2} at sample {i} ({:?}) is tighter than the road", a.position);
        }
    }

    #[test]
    fn stacked_sections_leave_headroom() {
        let path = TrackPath::default();
        let count = path.frames.len();
        let mut tightest = f32::MAX;
        for i in 0..count {
            for j in i + 1..count {
                // Skip neighbours along the loop; only true crossings matter.
                let gap = (j - i).min(count - (j - i));
                if (gap as f32) * TrackPath::FRAME_SPACING < path.width * 2.0 { continue; }
                let (a, b) = (path.frames[i].position, path.frames[j].position);
                if horizontal(a - b).length() < path.width {
                    tightest = tightest.min((a.y - b.y).abs());
                }
            }
        }
        assert!(tightest > MIN_HEADROOM, "only {tightest:.2} units between stacked road sections");
    }

    #[test]
    fn road_stays_above_the_ground() {
        let path = TrackPath::default();
        for (i, frame) in path.frames.iter().enumerate() {
            let underside = frame.position.y - TrackPath::ROAD_THICKNESS;
            assert!(underside > TrackPath::GROUND_TOP, "road dips into the ground at sample {i} ({:?})", frame.position);
        }
    }

    #[test]
    fn track_has_no_sharp_kinks() {
        let path = TrackPath::default();
        let count = path.frames.len();
        for i in 0..count {
            let a = path.frames[i].tangent;
            let b = path.frames[(i + 1) % count].tangent;
            let angle = a.angle_between(b).to_degrees();
            assert!(angle < 8.0, "kink of {angle:.1} degrees at sample {i}");
        }
    }

    #[test]
    fn local_search_follows_the_car_along_the_loop() {
        let path = TrackPath::default();
        let mut hint = Some(0);
        for (i, frame) in path.frames.iter().enumerate() {
            let hit = path.closest(frame.position + frame.normal * 0.9, hint);
            assert!(hit.index.abs_diff(i) <= 1 || hit.index.abs_diff(i) == path.frames.len() - 1, "jumped from {i} to {}", hit.index);
            hint = Some(hit.index);
        }
    }
}
