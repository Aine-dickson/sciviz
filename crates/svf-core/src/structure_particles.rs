//! Particle generators for animation (docs/animation-design.md section 5).
//!
//! These **show** a process; they do not compute one. Every position is a
//! pure function of the specification and the time: the same seed gives the
//! same picture on every platform, and nothing depends on frame rate. Only
//! `+ - * /` and `sqrt` are used on floating-point numbers (all exactly
//! rounded), and the random numbers come from a small generator written here,
//! so results do not depend on the platform's maths library.
//!
//! - [`Spread`]: particles that wander inside a closed container (diffusion).
//!   The walk is replayed once, on a fixed time step, when the spread is
//!   made; asking for a time then only reads and interpolates.
//! - [`Flow`]: particles that travel along a path and wrap round (water in a
//!   stem, blood in a vessel). Evaluated directly from the time.

use std::{error::Error, fmt};

use crate::geometry::TriangleMesh3;

/// Most particles in one generator (the first cut draws them as merged low-poly spheres).
pub const MAX_PARTICLES: usize = 2000;
/// Most stored positions (particles times steps) in one [`Spread`].
pub const MAX_STORED_POSITIONS: usize = 20_000_000;

/// Why a generator could not be made.
#[derive(Clone, Debug, PartialEq)]
pub enum ParticleError {
    BadCount(usize),
    /// A number that must be finite and positive is not.
    NotPositive(&'static str),
    NotFinite(&'static str),
    /// `count * steps` would be too large to store.
    TooManySteps,
    /// No point of the start region could be found inside the container.
    StartOutsideContainer,
    /// A flow path needs at least two distinct points.
    BadPath,
}

impl fmt::Display for ParticleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadCount(n) => write!(f, "particle count {n} is not between 1 and {MAX_PARTICLES}"),
            Self::NotPositive(what) => write!(f, "{what} must be finite and greater than zero"),
            Self::NotFinite(what) => write!(f, "{what} must be finite"),
            Self::TooManySteps => write!(f, "too many particles times steps to store"),
            Self::StartOutsideContainer => write!(f, "the start region does not lie inside the container"),
            Self::BadPath => write!(f, "a flow path needs at least two distinct finite points"),
        }
    }
}

impl Error for ParticleError {}

/// A closed region particles stay inside.
pub trait Container {
    fn contains(&self, point: [f64; 3]) -> bool;
}

/// A cylinder standing on the +Y axis direction: base centre, radius, height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cylinder {
    pub base_centre: [f64; 3],
    pub radius: f64,
    pub height: f64,
}

impl Container for Cylinder {
    fn contains(&self, p: [f64; 3]) -> bool {
        let (dx, dz) = (p[0] - self.base_centre[0], p[2] - self.base_centre[2]);
        let dy = p[1] - self.base_centre[1];
        dx * dx + dz * dz <= self.radius * self.radius && dy >= 0.0 && dy <= self.height
    }
}

/// An axis-aligned box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Box3 {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

impl Container for Box3 {
    fn contains(&self, p: [f64; 3]) -> bool {
        (0..3).all(|a| p[a] >= self.min[a] && p[a] <= self.max[a])
    }
}

/// A ball.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ball {
    pub centre: [f64; 3],
    pub radius: f64,
}

impl Container for Ball {
    fn contains(&self, p: [f64; 3]) -> bool {
        let d: f64 = (0..3).map(|a| (p[a] - self.centre[a]).powi(2)).sum();
        d <= self.radius * self.radius
    }
}

/// The inside of a closed triangle mesh, by ray parity along a fixed
/// oblique direction. Cost is one pass over the triangles per question, so it
/// suits low-polygon containers. Meant for a **closed** mesh; a point exactly
/// on the surface may go either way.
pub struct MeshContainer<'a> {
    mesh: &'a TriangleMesh3,
    low: [f64; 3],
    high: [f64; 3],
}

impl<'a> MeshContainer<'a> {
    #[must_use]
    pub fn new(mesh: &'a TriangleMesh3) -> Self {
        let mut low = [f64::INFINITY; 3];
        let mut high = [f64::NEG_INFINITY; 3];
        for v in mesh.vertices() {
            let p = v.to_array();
            for a in 0..3 {
                low[a] = low[a].min(p[a]);
                high[a] = high[a].max(p[a]);
            }
        }
        Self { mesh, low, high }
    }
}

impl Container for MeshContainer<'_> {
    fn contains(&self, p: [f64; 3]) -> bool {
        if (0..3).any(|a| p[a] < self.low[a] || p[a] > self.high[a]) {
            return false;
        }
        // Chosen so that it is very unlikely to graze an edge of a regular shape.
        let d = [0.371_283_9, 0.842_167_1, 0.390_551_3];
        let v = self.mesh.vertices();
        let mut crossings = 0usize;
        for t in self.mesh.indices().chunks_exact(3) {
            let a = v[t[0] as usize].to_array();
            let b = v[t[1] as usize].to_array();
            let c = v[t[2] as usize].to_array();
            let (e1, e2) = (sub(b, a), sub(c, a));
            let h = cross(d, e2);
            let det = dot(e1, h);
            if det.abs() < 1e-14 {
                continue;
            }
            let s = sub(p, a);
            let u = dot(s, h) / det;
            if !(0.0..=1.0).contains(&u) {
                continue;
            }
            let q = cross(s, e1);
            let w = dot(d, q) / det;
            if w < 0.0 || u + w > 1.0 {
                continue;
            }
            if dot(e2, q) / det > 0.0 {
                crossings += 1;
            }
        }
        crossings % 2 == 1
    }
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

/// SplitMix64: tiny, fast, and identical everywhere.
#[derive(Clone, Debug)]
struct Rng(u64);

impl Rng {
    fn new(seed: u64, stream: u64) -> Self {
        // Separate streams for separate particles from one seed.
        let mut r = Self(seed ^ stream.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        r.next_u64();
        r
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[0, 1)`.
    #[allow(clippy::cast_precision_loss)]
    fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / 9_007_199_254_740_992.0)
    }

    /// Uniform in `[-1, 1)`.
    fn signed(&mut self) -> f64 {
        self.unit() * 2.0 - 1.0
    }

    /// A uniformly random direction (rejection from the unit ball).
    fn direction(&mut self) -> [f64; 3] {
        loop {
            let v = [self.signed(), self.signed(), self.signed()];
            let s = dot(v, v);
            if s > 1e-12 && s <= 1.0 {
                let r = s.sqrt();
                return [v[0] / r, v[1] / r, v[2] / r];
            }
        }
    }

    /// A uniformly random point of the unit ball.
    fn in_ball(&mut self) -> [f64; 3] {
        loop {
            let v = [self.signed(), self.signed(), self.signed()];
            if dot(v, v) <= 1.0 {
                return v;
            }
        }
    }
}

fn positive(what: &'static str, value: f64) -> Result<(), ParticleError> {
    if value.is_finite() && value > 0.0 { Ok(()) } else { Err(ParticleError::NotPositive(what)) }
}

fn finite3(what: &'static str, p: [f64; 3]) -> Result<(), ParticleError> {
    if p.iter().all(|c| c.is_finite()) { Ok(()) } else { Err(ParticleError::NotFinite(what)) }
}

/// What to spread.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpreadSpec {
    pub count: usize,
    /// Centre of the region the particles start in, and its radius.
    pub start: [f64; 3],
    pub start_radius: f64,
    /// Distance moved at each time step.
    pub step_length: f64,
    /// Time steps per second of presentation time.
    pub steps_per_second: f64,
    /// Length of the walk in seconds; later times show the last position.
    pub duration_s: f64,
    pub seed: u64,
}

/// Particles wandering inside a container, from a small start region.
///
/// Each step a particle tries to move `step_length` in a random direction. If
/// that lands outside the container it tries the mirror direction (a bounce
/// off the wall) and, failing that, stays where it is. So no particle ever
/// leaves, and the count never changes.
#[derive(Clone, Debug, PartialEq)]
pub struct Spread {
    count: usize,
    steps: usize,
    steps_per_second: f64,
    /// `(steps + 1)` rows of `count` positions; row 0 is the start.
    positions: Vec<[f64; 3]>,
}

impl Spread {
    pub fn new(spec: &SpreadSpec, container: &dyn Container) -> Result<Self, ParticleError> {
        if spec.count == 0 || spec.count > MAX_PARTICLES {
            return Err(ParticleError::BadCount(spec.count));
        }
        positive("start radius", spec.start_radius)?;
        positive("step length", spec.step_length)?;
        positive("steps per second", spec.steps_per_second)?;
        positive("duration", spec.duration_s)?;
        finite3("start", spec.start)?;
        let steps_f = (spec.duration_s * spec.steps_per_second).ceil();
        if !steps_f.is_finite() || steps_f > 1.0e7 {
            return Err(ParticleError::TooManySteps);
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let steps = steps_f.max(1.0) as usize;
        if spec.count.checked_mul(steps + 1).is_none_or(|n| n > MAX_STORED_POSITIONS) {
            return Err(ParticleError::TooManySteps);
        }

        let mut positions = Vec::with_capacity(spec.count * (steps + 1));
        let mut rngs: Vec<Rng> = (0..spec.count).map(|i| Rng::new(spec.seed, i as u64)).collect();
        for rng in &mut rngs {
            let mut found = None;
            for _ in 0..200 {
                let o = rng.in_ball();
                let p = [
                    spec.start[0] + o[0] * spec.start_radius,
                    spec.start[1] + o[1] * spec.start_radius,
                    spec.start[2] + o[2] * spec.start_radius,
                ];
                if container.contains(p) {
                    found = Some(p);
                    break;
                }
            }
            positions.push(found.ok_or(ParticleError::StartOutsideContainer)?);
        }
        for step in 0..steps {
            for (i, rng) in rngs.iter_mut().enumerate() {
                let here = positions[step * spec.count + i];
                let d = rng.direction();
                let try_to = |sign: f64| {
                    [
                        here[0] + sign * d[0] * spec.step_length,
                        here[1] + sign * d[1] * spec.step_length,
                        here[2] + sign * d[2] * spec.step_length,
                    ]
                };
                let forward = try_to(1.0);
                let next = if container.contains(forward) {
                    forward
                } else {
                    let back = try_to(-1.0);
                    if container.contains(back) { back } else { here }
                };
                positions.push(next);
            }
        }
        Ok(Self { count: spec.count, steps, steps_per_second: spec.steps_per_second, positions })
    }

    #[must_use]
    pub const fn count(&self) -> usize {
        self.count
    }

    /// Length of the stored walk in seconds.
    #[must_use]
    pub fn duration(&self) -> f64 {
        self.steps as f64 / self.steps_per_second
    }

    /// Position of particle `i` at time `t` (clamped to the walk), linearly
    /// interpolated between stored steps. `None` if `i` is out of range.
    #[must_use]
    pub fn position(&self, i: usize, t: f64) -> Option<[f64; 3]> {
        if i >= self.count {
            return None;
        }
        let t = if t.is_nan() { 0.0 } else { t.clamp(0.0, self.duration()) };
        let x = t * self.steps_per_second;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let row = (x.floor() as usize).min(self.steps.saturating_sub(1));
        #[allow(clippy::cast_precision_loss)]
        let f = (x - row as f64).clamp(0.0, 1.0);
        let a = self.positions[row * self.count + i];
        let b = self.positions[(row + 1) * self.count + i];
        Some([a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f, a[2] + (b[2] - a[2]) * f])
    }

    /// Every particle at time `t`, in particle order.
    #[must_use]
    pub fn positions_at(&self, t: f64) -> Vec<[f64; 3]> {
        (0..self.count).filter_map(|i| self.position(i, t)).collect()
    }
}

/// What to make flow.
#[derive(Clone, Debug, PartialEq)]
pub struct FlowSpec {
    pub count: usize,
    /// Distance along the path per second.
    pub speed: f64,
    /// The centre line, at least two distinct points.
    pub path: Vec<[f64; 3]>,
    /// How far from the centre line a particle may sit.
    pub lateral_radius: f64,
    pub seed: u64,
}

/// Particles travelling along a path and wrapping from the end to the start.
#[derive(Clone, Debug, PartialEq)]
pub struct Flow {
    speed: f64,
    path: Vec<[f64; 3]>,
    /// Cumulative length at each path point.
    cumulative: Vec<f64>,
    /// Each particle: start distance along the path, and two offsets in `[-1, 1]` across it.
    particles: Vec<(f64, f64, f64)>,
    lateral_radius: f64,
}

impl Flow {
    pub fn new(spec: &FlowSpec) -> Result<Self, ParticleError> {
        if spec.count == 0 || spec.count > MAX_PARTICLES {
            return Err(ParticleError::BadCount(spec.count));
        }
        positive("speed", spec.speed)?;
        if !spec.lateral_radius.is_finite() || spec.lateral_radius < 0.0 {
            return Err(ParticleError::NotFinite("lateral radius"));
        }
        let mut path: Vec<[f64; 3]> = Vec::with_capacity(spec.path.len());
        for p in &spec.path {
            finite3("path point", *p)?;
            if path.last().is_none_or(|last| sub(*p, *last) != [0.0; 3]) {
                path.push(*p);
            }
        }
        if path.len() < 2 {
            return Err(ParticleError::BadPath);
        }
        let mut cumulative = vec![0.0];
        for w in path.windows(2) {
            let d = sub(w[1], w[0]);
            cumulative.push(cumulative[cumulative.len() - 1] + dot(d, d).sqrt());
        }
        let length = cumulative[cumulative.len() - 1];
        // Evenly spread along the path, with a small random shift, so the flow looks continuous.
        let particles = (0..spec.count)
            .map(|i| {
                let mut rng = Rng::new(spec.seed, i as u64);
                #[allow(clippy::cast_precision_loss)]
                let base = (i as f64 + rng.unit() * 0.8) / spec.count as f64;
                (base * length, rng.signed(), rng.signed())
            })
            .collect();
        Ok(Self { speed: spec.speed, path, cumulative, particles, lateral_radius: spec.lateral_radius })
    }

    #[must_use]
    pub const fn count(&self) -> usize {
        self.particles.len()
    }

    /// Total path length: the particles come back to their start after `length / speed` seconds.
    #[must_use]
    pub fn length(&self) -> f64 {
        self.cumulative[self.cumulative.len() - 1]
    }

    /// Position of particle `i` at time `t` (not clamped: the flow goes on and wraps).
    #[must_use]
    pub fn position(&self, i: usize, t: f64) -> Option<[f64; 3]> {
        let &(start, u, v) = self.particles.get(i)?;
        let t = if t.is_finite() { t } else { 0.0 };
        let length = self.length();
        let s = (start + self.speed * t).rem_euclid(length);
        let seg = self
            .cumulative
            .windows(2)
            .position(|w| s >= w[0] && s < w[1])
            .unwrap_or(self.path.len() - 2);
        let (a, b) = (self.path[seg], self.path[seg + 1]);
        let along = (s - self.cumulative[seg]) / (self.cumulative[seg + 1] - self.cumulative[seg]);
        let centre = [a[0] + (b[0] - a[0]) * along, a[1] + (b[1] - a[1]) * along, a[2] + (b[2] - a[2]) * along];

        // Two directions across the segment, from the axis least aligned with it.
        let d = sub(b, a);
        let len = dot(d, d).sqrt();
        let tangent = [d[0] / len, d[1] / len, d[2] / len];
        let helper = if tangent[1].abs() < 0.9 { [0.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0] };
        let n1 = cross(tangent, helper);
        let l1 = dot(n1, n1).sqrt();
        let n1 = [n1[0] / l1, n1[1] / l1, n1[2] / l1];
        let n2 = cross(tangent, n1);
        // Keep the offset inside a disc: scale a square's point back if it is outside.
        let r = (u * u + v * v).sqrt().max(1.0);
        let (u, v) = (u / r * self.lateral_radius, v / r * self.lateral_radius);
        Some([
            centre[0] + n1[0] * u + n2[0] * v,
            centre[1] + n1[1] * u + n2[1] * v,
            centre[2] + n1[2] * u + n2[2] * v,
        ])
    }

    #[must_use]
    pub fn positions_at(&self, t: f64) -> Vec<[f64; 3]> {
        (0..self.count()).filter_map(|i| self.position(i, t)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spatial::WorldPoint3;

    fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
        let d = sub(a, b);
        dot(d, d).sqrt()
    }

    fn beaker() -> Cylinder {
        Cylinder { base_centre: [0.0, 0.0, 0.0], radius: 3.0, height: 8.0 }
    }

    fn spec(seed: u64) -> SpreadSpec {
        SpreadSpec {
            count: 60,
            start: [0.0, 6.0, 0.0],
            start_radius: 0.5,
            step_length: 0.3,
            steps_per_second: 20.0,
            duration_s: 10.0,
            seed,
        }
    }

    fn cube_mesh(low: f64, high: f64) -> TriangleMesh3 {
        let p = |x, y, z| WorldPoint3::new(x, y, z);
        let vertices = vec![
            p(low, low, low), p(high, low, low), p(high, high, low), p(low, high, low),
            p(low, low, high), p(high, low, high), p(high, high, high), p(low, high, high),
        ];
        // Outward winding.
        let indices = vec![
            0, 2, 1, 0, 3, 2, // z = low
            4, 5, 6, 4, 6, 7, // z = high
            0, 1, 5, 0, 5, 4, // y = low
            3, 7, 6, 3, 6, 2, // y = high
            0, 4, 7, 0, 7, 3, // x = low
            1, 2, 6, 1, 6, 5, // x = high
        ];
        TriangleMesh3::new(vertices, indices).unwrap()
    }

    #[test]
    fn simple_containers_answer_inside_and_outside() {
        let c = beaker();
        assert!(c.contains([0.0, 4.0, 0.0]) && c.contains([2.9, 0.1, 0.0]));
        assert!(!c.contains([3.1, 4.0, 0.0]) && !c.contains([0.0, 8.1, 0.0]) && !c.contains([0.0, -0.1, 0.0]));
        let b = Box3 { min: [0.0; 3], max: [1.0, 2.0, 3.0] };
        assert!(b.contains([0.5, 1.0, 2.9]) && !b.contains([0.5, 2.1, 1.0]));
        let ball = Ball { centre: [1.0, 1.0, 1.0], radius: 1.0 };
        assert!(ball.contains([1.0, 1.5, 1.0]) && !ball.contains([2.1, 1.0, 1.0]));
    }

    #[test]
    fn a_closed_mesh_container_matches_the_cube_it_encloses() {
        let mesh = cube_mesh(-1.0, 1.0);
        let inside = MeshContainer::new(&mesh);
        let reference = Box3 { min: [-1.0; 3], max: [1.0; 3] };
        let mut rng = Rng::new(7, 0);
        for _ in 0..2000 {
            let p = [rng.signed() * 1.5, rng.signed() * 1.5, rng.signed() * 1.5];
            assert_eq!(inside.contains(p), reference.contains(p), "{p:?}");
        }
    }

    #[test]
    fn the_generator_is_fixed_and_streams_differ() {
        // A known value pins the algorithm: it must never change between platforms or versions.
        assert_eq!(Rng(0).next_u64(), 0xE220_A839_7B1D_CDAF);
        let mut a = Rng::new(1, 0);
        let mut b = Rng::new(1, 1);
        assert_ne!(a.next_u64(), b.next_u64());
        let u = Rng::new(5, 5).unit();
        assert!((0.0..1.0).contains(&u));
        let d = Rng::new(9, 9).direction();
        assert!((dot(d, d) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn a_spread_is_deterministic_and_depends_on_the_seed() {
        let c = beaker();
        let a = Spread::new(&spec(1), &c).unwrap();
        assert_eq!(a, Spread::new(&spec(1), &c).unwrap());
        assert_ne!(a, Spread::new(&spec(2), &c).unwrap());
        assert_eq!(a.positions_at(3.3), a.positions_at(3.3));
    }

    #[test]
    fn particles_stay_inside_and_the_count_never_changes() {
        let c = beaker();
        let s = Spread::new(&spec(3), &c).unwrap();
        assert_eq!(s.count(), 60);
        assert!((s.duration() - 10.0).abs() < 1e-12);
        for step in 0..=200 {
            let t = f64::from(step) * 0.05;
            let all = s.positions_at(t);
            assert_eq!(all.len(), 60);
            assert!(all.iter().all(|&p| c.contains(p)), "a particle left the beaker at t={t}");
        }
        // Even between steps (a segment between two inside points of a convex container stays inside).
        assert!(s.positions_at(4.37).iter().all(|&p| c.contains(p)));
    }

    #[test]
    fn particles_start_in_the_start_region_and_spread_out() {
        let c = beaker();
        let s = Spread::new(&spec(4), &c).unwrap();
        let start = [0.0, 6.0, 0.0];
        assert!(s.positions_at(0.0).iter().all(|&p| dist(p, start) <= 0.5 + 1e-12));
        let mean = |t: f64| s.positions_at(t).iter().map(|&p| dist(p, start)).sum::<f64>() / 60.0;
        // Spreading from a small start, then levelling off once the beaker is filled
        // (so later times need not keep growing).
        assert!(mean(0.0) < 0.5 && mean(0.0) < mean(1.0) && mean(1.0) < mean(5.0), "{} {} {}", mean(0.0), mean(1.0), mean(5.0));
        assert!(mean(10.0) > 3.0 * mean(0.0));
    }

    #[test]
    fn times_are_clamped_and_positions_interpolate() {
        let c = beaker();
        let s = Spread::new(&spec(5), &c).unwrap();
        assert_eq!(s.positions_at(-4.0), s.positions_at(0.0));
        assert_eq!(s.positions_at(99.0), s.positions_at(10.0));
        assert_eq!(s.positions_at(f64::NAN), s.positions_at(0.0));
        // Halfway between two steps is the mean of the two.
        let (a, b, m) = (s.position(7, 1.0).unwrap(), s.position(7, 1.05).unwrap(), s.position(7, 1.025).unwrap());
        for axis in 0..3 {
            assert!((m[axis] - (a[axis] + b[axis]) / 2.0).abs() < 1e-9);
        }
        assert_eq!(s.position(60, 0.0), None);
    }

    #[test]
    fn a_spread_in_a_mesh_container_stays_inside_it() {
        let mesh = cube_mesh(-2.0, 2.0);
        let c = MeshContainer::new(&mesh);
        let s = Spread::new(
            &SpreadSpec { count: 20, start: [0.0; 3], start_radius: 0.3, step_length: 0.4,
                steps_per_second: 10.0, duration_s: 5.0, seed: 8 },
            &c,
        )
        .unwrap();
        for step in 0..=50 {
            assert!(s.positions_at(f64::from(step) * 0.1).iter().all(|&p| c.contains(p)));
        }
    }

    #[test]
    fn bad_spreads_are_refused() {
        let c = beaker();
        let bad = |change: &dyn Fn(&mut SpreadSpec)| {
            let mut s = spec(1);
            change(&mut s);
            Spread::new(&s, &c).unwrap_err()
        };
        assert_eq!(bad(&|s| s.count = 0), ParticleError::BadCount(0));
        assert_eq!(bad(&|s| s.count = MAX_PARTICLES + 1), ParticleError::BadCount(MAX_PARTICLES + 1));
        assert_eq!(bad(&|s| s.step_length = 0.0), ParticleError::NotPositive("step length"));
        assert_eq!(bad(&|s| s.duration_s = f64::NAN), ParticleError::NotPositive("duration"));
        assert_eq!(bad(&|s| s.start = [50.0, 0.0, 0.0]), ParticleError::StartOutsideContainer);
        assert_eq!(bad(&|s| s.start = [f64::NAN, 0.0, 0.0]), ParticleError::NotFinite("start"));
        assert_eq!(bad(&|s| { s.count = 2000; s.duration_s = 1.0e5; }), ParticleError::TooManySteps);
    }

    fn stem() -> FlowSpec {
        FlowSpec {
            count: 40,
            speed: 2.0,
            path: vec![[0.0, 0.0, 0.0], [0.0, 5.0, 0.0], [1.0, 10.0, 0.0]],
            lateral_radius: 0.4,
            seed: 11,
        }
    }

    fn distance_to_path(path: &[[f64; 3]], p: [f64; 3]) -> f64 {
        path.windows(2)
            .map(|w| {
                let d = sub(w[1], w[0]);
                let t = (dot(sub(p, w[0]), d) / dot(d, d)).clamp(0.0, 1.0);
                dist(p, [w[0][0] + d[0] * t, w[0][1] + d[1] * t, w[0][2] + d[2] * t])
            })
            .fold(f64::INFINITY, f64::min)
    }

    #[test]
    fn a_flow_keeps_every_particle_near_the_path_and_the_count_fixed() {
        let spec = stem();
        let f = Flow::new(&spec).unwrap();
        assert_eq!(f.count(), 40);
        for step in 0..60 {
            let all = f.positions_at(f64::from(step) * 0.37);
            assert_eq!(all.len(), 40);
            for p in all {
                assert!(distance_to_path(&spec.path, p) <= 0.4 + 1e-9, "{p:?}");
            }
        }
    }

    #[test]
    fn a_flow_moves_along_the_path_at_its_speed_and_repeats() {
        let f = Flow::new(&stem()).unwrap();
        let length = f.length();
        assert!((length - (5.0 + 26.0_f64.sqrt())).abs() < 1e-12);
        // The start is not the same a moment later, and it repeats after one lap.
        assert_ne!(f.positions_at(0.0), f.positions_at(0.5));
        let lap = length / 2.0;
        for (a, b) in f.positions_at(0.0).into_iter().zip(f.positions_at(lap)) {
            assert!(dist(a, b) < 1e-9);
        }
        // Positions along the path are evenly spread (no clumping): the sorted
        // distances along y never leave a gap bigger than a few mean spacings.
        let mut ys: Vec<f64> = f.positions_at(0.0).iter().map(|p| p[1]).collect();
        ys.sort_by(f64::total_cmp);
        assert!(ys.windows(2).all(|w| w[1] - w[0] < 1.0), "{ys:?}");
    }

    #[test]
    fn a_flow_is_deterministic_and_bad_ones_are_refused() {
        assert_eq!(Flow::new(&stem()).unwrap(), Flow::new(&stem()).unwrap());
        assert_ne!(Flow::new(&stem()).unwrap(), Flow::new(&FlowSpec { seed: 12, ..stem() }).unwrap());
        let bad = |change: &dyn Fn(&mut FlowSpec)| {
            let mut s = stem();
            change(&mut s);
            Flow::new(&s).unwrap_err()
        };
        assert_eq!(bad(&|s| s.count = 0), ParticleError::BadCount(0));
        assert_eq!(bad(&|s| s.speed = -1.0), ParticleError::NotPositive("speed"));
        assert_eq!(bad(&|s| s.path = vec![[1.0; 3], [1.0; 3]]), ParticleError::BadPath);
        assert_eq!(bad(&|s| s.path = vec![[0.0; 3]]), ParticleError::BadPath);
        assert_eq!(bad(&|s| s.path[1] = [f64::INFINITY, 0.0, 0.0]), ParticleError::NotFinite("path point"));
        assert_eq!(bad(&|s| s.lateral_radius = -1.0), ParticleError::NotFinite("lateral radius"));
        assert_eq!(Flow::new(&stem()).unwrap().position(40, 0.0), None);
    }
}
