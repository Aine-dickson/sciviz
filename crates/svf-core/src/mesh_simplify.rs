//! Quadric edge-collapse simplification of a triangle surface with pinned
//! boundaries. Design: `docs/heart-structure.md` sections 6 and 17.
//!
//! [`simplify`] reduces a consistently wound surface (open or closed 2-manifold)
//! by repeatedly collapsing the cheapest edge, with these guarantees:
//!
//! * **Boundary pinned.** Every vertex on an open boundary, and every vertex the
//!   caller pins, keeps its exact coordinates and is never merged with another
//!   pinned vertex. Two patches simplified apart therefore still share their
//!   common boundary exactly ([`Simplified::vertex_map`] names the source vertex
//!   each output vertex came from).
//! * **No flips.** A collapse is refused if it turns any surviving triangle by
//!   more than `acos(min_normal_cos)` or makes it degenerate.
//! * **Link condition.** A collapse is refused unless the two endpoints share
//!   exactly the apexes of the triangles on the edge, so the topology (closure,
//!   genus, manifoldness) never changes and no pinch can appear.
//! * **Error bound.** Each vertex carries the sum of squared distances to the
//!   planes of the source triangles it stands for (unit normals, not area
//!   weighted) and how many planes that is. A collapse is refused when the root
//!   mean square distance at the new position exceeds `epsilon`. That is a
//!   heuristic for the deviation from the source, not a bound on it (the sum
//!   itself would be a bound, but it grows with the number of planes and stops
//!   simplification far too early); the real deviation is measured by
//!   [`sampled_deviation`] and is what a caller checks.
//!
//! The result is deterministic: ties break by vertex index.

use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap},
    error::Error,
    fmt,
};

use crate::region_surface::P3;

/// Options for [`simplify`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimplifyOptions {
    /// Largest allowed root mean square plane error of a collapse, in the units of the points.
    pub epsilon: f64,
    /// Stop once this many triangles or fewer remain (0 = go as far as the error allows).
    pub target_triangles: usize,
    /// A surviving triangle's normal must keep at least this cosine with its old
    /// normal. Strictly above -1; 0 forbids turning past a right angle; the default 0.1.
    pub min_normal_cos: f64,
}

impl Default for SimplifyOptions {
    fn default() -> Self {
        Self { epsilon: 0.4, target_triangles: 0, min_normal_cos: 0.1 }
    }
}

/// Why simplification stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopReason {
    TargetReached,
    /// The cheapest legal collapse costs more than `epsilon`.
    ErrorBound,
    /// No collapse is legal (everything pinned, or refused by the link or flip tests).
    NoLegalCollapse,
}

/// What [`simplify`] did.
#[derive(Clone, Debug, PartialEq)]
pub struct SimplifyReport {
    pub triangles_before: usize,
    pub triangles_after: usize,
    pub collapses: usize,
    pub refused_link: usize,
    pub refused_flip: usize,
    /// Largest root mean square plane error of an accepted collapse.
    pub max_collapse_error: f64,
    pub stop: StopReason,
    /// The measured deviation from the source, set by [`simplify_within`] only.
    pub measured_deviation: Option<f64>,
    /// The `epsilon` that produced the result ([`simplify_within`] searches for it).
    pub epsilon_used: f64,
}

/// A simplified surface.
#[derive(Clone, Debug, PartialEq)]
pub struct Simplified {
    pub points: Vec<P3>,
    pub triangles: Vec<[u32; 3]>,
    /// For each output vertex, the source vertex it descends from (the survivor
    /// of its collapses). A pinned vertex has the same coordinates as that source vertex.
    pub vertex_map: Vec<u32>,
    pub report: SimplifyReport,
}

/// Why a surface cannot be simplified.
#[derive(Clone, Debug, PartialEq)]
pub enum SimplifyError {
    BadOptions,
    NonFinitePoint { index: usize },
    IndexOutOfRange { triangle: usize },
    DegenerateTriangle { triangle: usize },
    PinnedLength { pinned: usize, points: usize },
    /// Two triangles use an edge in the same direction (this also covers three or
    /// more triangles on one edge, which must repeat a direction).
    Inconsistent { edges: usize },
}

impl fmt::Display for SimplifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadOptions => write!(f, "epsilon must be finite and positive, min_normal_cos in (-1, 1)"),
            Self::NonFinitePoint { index } => write!(f, "point {index} is not finite"),
            Self::IndexOutOfRange { triangle } => write!(f, "triangle {triangle} names a missing point"),
            Self::DegenerateTriangle { triangle } => write!(f, "triangle {triangle} repeats a vertex"),
            Self::PinnedLength { pinned, points } => write!(f, "{pinned} pin flags for {points} points"),
            Self::Inconsistent { edges } => write!(f, "{edges} edges are used in the same direction twice"),
        }
    }
}

impl Error for SimplifyError {}

fn sub(a: P3, b: P3) -> P3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: P3, b: P3) -> P3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn dot(a: P3, b: P3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn len(a: P3) -> f64 {
    dot(a, a).sqrt()
}
fn normal(a: P3, b: P3, c: P3) -> P3 {
    cross(sub(b, a), sub(c, a))
}

// ---------------------------------------------------------------- quadrics

/// Symmetric 4x4 quadric: a2, ab, ac, ad, b2, bc, bd, c2, cd, d2, then the number
/// of source planes it sums (so the error can be reported as a root mean square).
type Quadric = [f64; 11];

fn plane_quadric(n: P3, d: f64) -> Quadric {
    [n[0] * n[0], n[0] * n[1], n[0] * n[2], n[0] * d, n[1] * n[1], n[1] * n[2], n[1] * d, n[2] * n[2], n[2] * d, d * d, 1.0]
}

fn add(q: &mut Quadric, r: &Quadric) {
    for (a, b) in q.iter_mut().zip(r) {
        *a += b;
    }
}

fn eval(q: &Quadric, p: P3) -> f64 {
    let [x, y, z] = p;
    let v = q[0] * x * x
        + 2.0 * q[1] * x * y
        + 2.0 * q[2] * x * z
        + 2.0 * q[3] * x
        + q[4] * y * y
        + 2.0 * q[5] * y * z
        + 2.0 * q[6] * y
        + q[7] * z * z
        + 2.0 * q[8] * z
        + q[9];
    v.max(0.0)
}

/// Root mean square distance to the summed planes.
fn rms(q: &Quadric, raw: f64) -> f64 {
    if q[10] > 0.0 {
        (raw / q[10]).sqrt()
    } else {
        0.0
    }
}

/// The point minimising the quadric, if its 3x3 part is well conditioned.
fn optimum(q: &Quadric) -> Option<P3> {
    let (a, b, c, d, e, f) = (q[0], q[1], q[2], q[4], q[5], q[7]);
    let rhs = [-q[3], -q[6], -q[8]];
    let det = a * (d * f - e * e) - b * (b * f - c * e) + c * (b * e - c * d);
    let trace = a + d + f;
    if !(trace > 0.0) || det.abs() <= 1e-9 * trace * trace * trace {
        return None;
    }
    let x = (rhs[0] * (d * f - e * e) - b * (rhs[1] * f - c * rhs[2]) + c * (rhs[1] * e - d * rhs[2])) / det;
    let y = (a * (rhs[1] * f - rhs[2] * e) - rhs[0] * (b * f - c * e) + c * (b * rhs[2] - rhs[1] * c)) / det;
    let z = (a * (d * rhs[2] - e * rhs[1]) - b * (b * rhs[2] - rhs[1] * c) + rhs[0] * (b * e - c * d)) / det;
    let p = [x, y, z];
    p.iter().all(|v| v.is_finite()).then_some(p)
}

// ------------------------------------------------------------------ state

struct State {
    pos: Vec<P3>,
    quadric: Vec<Quadric>,
    pinned: Vec<bool>,
    tris: Vec<[u32; 3]>,
    tri_alive: Vec<bool>,
    vfaces: Vec<Vec<u32>>,
    vertex_alive: Vec<bool>,
    version: Vec<u32>,
}

impl State {
    fn neighbours(&self, v: u32) -> Vec<u32> {
        let mut out: Vec<u32> = Vec::new();
        for &t in &self.vfaces[v as usize] {
            for &w in &self.tris[t as usize] {
                if w != v {
                    out.push(w);
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// The cheapest position for collapsing `a` and `b` into one vertex, and its
    /// root mean square plane error. A pinned endpoint's position is kept.
    fn best_position(&self, a: u32, b: u32) -> (P3, f64) {
        let (pa, pb) = (self.pos[a as usize], self.pos[b as usize]);
        let mut q = self.quadric[a as usize];
        add(&mut q, &self.quadric[b as usize]);
        if self.pinned[a as usize] {
            return (pa, rms(&q, eval(&q, pa)));
        }
        if self.pinned[b as usize] {
            return (pb, rms(&q, eval(&q, pb)));
        }
        let mid = [(pa[0] + pb[0]) / 2.0, (pa[1] + pb[1]) / 2.0, (pa[2] + pb[2]) / 2.0];
        let reach = len(sub(pa, pb));
        let mut best = (mid, eval(&q, mid));
        let mut candidates = vec![pa, pb];
        if let Some(p) = optimum(&q) {
            // A far optimum comes from a nearly singular system; do not trust it.
            if len(sub(p, mid)) <= reach {
                candidates.push(p);
            }
        }
        for p in candidates {
            let c = eval(&q, p);
            if c < best.1 {
                best = (p, c);
            }
        }
        (best.0, rms(&q, best.1))
    }

    /// The edge `a`-`b` is on exactly two triangles whose apexes are the only
    /// vertices the endpoints share.
    fn link_ok(&self, a: u32, b: u32) -> bool {
        let apexes: Vec<u32> = self.vfaces[a as usize]
            .iter()
            .filter_map(|&t| {
                let tri = self.tris[t as usize];
                tri.contains(&b).then(|| *tri.iter().find(|&&v| v != a && v != b).unwrap())
            })
            .collect();
        if apexes.len() != 2 {
            return false;
        }
        let (na, nb) = (self.neighbours(a), self.neighbours(b));
        let common: Vec<u32> = na.iter().copied().filter(|v| nb.binary_search(v).is_ok()).collect();
        common.len() == 2 && apexes.iter().all(|v| common.contains(v))
    }

    /// No surviving triangle around the edge turns too far or degenerates when
    /// `a` and `b` move to `p`.
    fn flip_ok(&self, a: u32, b: u32, p: P3, min_cos: f64) -> bool {
        let moved = |v: u32| if v == a || v == b { p } else { self.pos[v as usize] };
        let mut seen: Vec<u32> = self.vfaces[a as usize].iter().chain(&self.vfaces[b as usize]).copied().collect();
        seen.sort_unstable();
        seen.dedup();
        for t in seen {
            let tri = self.tris[t as usize];
            if tri.contains(&a) && tri.contains(&b) {
                continue;
            }
            let old = normal(self.pos[tri[0] as usize], self.pos[tri[1] as usize], self.pos[tri[2] as usize]);
            let new = normal(moved(tri[0]), moved(tri[1]), moved(tri[2]));
            let (lo, ln) = (len(old), len(new));
            if lo == 0.0 {
                continue;
            }
            if ln == 0.0 || dot(old, new) <= min_cos * lo * ln {
                return false;
            }
        }
        true
    }

    /// Merges `b` into `a` at `p`; returns how many triangles vanished.
    fn collapse(&mut self, a: u32, b: u32, p: P3) -> usize {
        let mut gone = 0;
        for t in std::mem::take(&mut self.vfaces[b as usize]) {
            let tri = self.tris[t as usize];
            if tri.contains(&a) {
                self.tri_alive[t as usize] = false;
                gone += 1;
                for &v in &tri {
                    if v != b {
                        self.vfaces[v as usize].retain(|&x| x != t);
                    }
                }
            } else {
                for v in &mut self.tris[t as usize] {
                    if *v == b {
                        *v = a;
                    }
                }
                self.vfaces[a as usize].push(t);
            }
        }
        self.vertex_alive[b as usize] = false;
        self.pos[a as usize] = p;
        let qb = self.quadric[b as usize];
        add(&mut self.quadric[a as usize], &qb);
        gone
    }
}

type Entry = Reverse<(u64, u32, u32, u32, u32)>;

fn push_edge(heap: &mut BinaryHeap<Entry>, s: &State, a: u32, b: u32) {
    if s.pinned[a as usize] && s.pinned[b as usize] {
        return;
    }
    let (lo, hi) = (a.min(b), a.max(b));
    let (_, cost) = s.best_position(lo, hi);
    heap.push(Reverse((cost.to_bits(), lo, hi, s.version[lo as usize], s.version[hi as usize])));
}

// ----------------------------------------------------------------- simplify

/// Simplifies a consistently wound 2-manifold triangle surface. `pinned` marks
/// extra vertices to keep (empty for none); open-boundary vertices are always kept.
pub fn simplify(
    points: &[P3],
    triangles: &[[u32; 3]],
    pinned: &[bool],
    options: &SimplifyOptions,
) -> Result<Simplified, SimplifyError> {
    if !(options.epsilon.is_finite() && options.epsilon > 0.0)
        || !(options.min_normal_cos > -1.0 && options.min_normal_cos < 1.0)
    {
        return Err(SimplifyError::BadOptions);
    }
    if !pinned.is_empty() && pinned.len() != points.len() {
        return Err(SimplifyError::PinnedLength { pinned: pinned.len(), points: points.len() });
    }
    if let Some(index) = points.iter().position(|p| p.iter().any(|v| !v.is_finite())) {
        return Err(SimplifyError::NonFinitePoint { index });
    }
    for (i, t) in triangles.iter().enumerate() {
        if t.iter().any(|&v| v as usize >= points.len()) {
            return Err(SimplifyError::IndexOutOfRange { triangle: i });
        }
        if t[0] == t[1] || t[1] == t[2] || t[0] == t[2] {
            return Err(SimplifyError::DegenerateTriangle { triangle: i });
        }
    }

    // Edge usage: directed counts, to find the boundary and reject bad input.
    let mut directed: HashMap<(u32, u32), u32> = HashMap::new();
    for t in triangles {
        for k in 0..3 {
            *directed.entry((t[k], t[(k + 1) % 3])).or_default() += 1;
        }
    }
    let same_direction = directed.values().filter(|&&n| n > 1).count();
    if same_direction > 0 {
        return Err(SimplifyError::Inconsistent { edges: same_direction });
    }
    let n = points.len();
    let mut pin: Vec<bool> = if pinned.is_empty() { vec![false; n] } else { pinned.to_vec() };
    for (&(a, b), _) in &directed {
        if !directed.contains_key(&(b, a)) {
            pin[a as usize] = true;
            pin[b as usize] = true;
        }
    }

    let mut quadric = vec![[0.0f64; 11]; n];
    let mut vfaces: Vec<Vec<u32>> = vec![Vec::new(); n];
    for (i, t) in triangles.iter().enumerate() {
        let (a, b, c) = (points[t[0] as usize], points[t[1] as usize], points[t[2] as usize]);
        let nrm = normal(a, b, c);
        let l = len(nrm);
        if l > 0.0 {
            let unit = [nrm[0] / l, nrm[1] / l, nrm[2] / l];
            let q = plane_quadric(unit, -dot(unit, a));
            for &v in t {
                add(&mut quadric[v as usize], &q);
            }
        }
        for &v in t {
            vfaces[v as usize].push(i as u32);
        }
    }

    let mut s = State {
        pos: points.to_vec(),
        quadric,
        pinned: pin,
        tris: triangles.to_vec(),
        tri_alive: vec![true; triangles.len()],
        vfaces,
        vertex_alive: triangles_vertices(triangles, n),
        version: vec![0; n],
    };

    let mut heap: BinaryHeap<Entry> = BinaryHeap::new();
    let mut seen_edges: Vec<(u32, u32)> = directed.keys().map(|&(a, b)| (a.min(b), a.max(b))).collect();
    seen_edges.sort_unstable();
    seen_edges.dedup();
    for (a, b) in seen_edges {
        push_edge(&mut heap, &s, a, b);
    }

    let limit = options.epsilon;
    let mut live = triangles.len();
    let mut report = SimplifyReport {
        triangles_before: triangles.len(),
        triangles_after: triangles.len(),
        collapses: 0,
        refused_link: 0,
        refused_flip: 0,
        max_collapse_error: 0.0,
        stop: StopReason::NoLegalCollapse,
        measured_deviation: None,
        epsilon_used: options.epsilon,
    };

    loop {
        if options.target_triangles > 0 && live <= options.target_triangles {
            report.stop = StopReason::TargetReached;
            break;
        }
        // A closed surface cannot go below a tetrahedron.
        if live <= 4 {
            report.stop = StopReason::NoLegalCollapse;
            break;
        }
        let Some(Reverse((_, a, b, va, vb))) = heap.pop() else {
            break;
        };
        if !s.vertex_alive[a as usize]
            || !s.vertex_alive[b as usize]
            || s.version[a as usize] != va
            || s.version[b as usize] != vb
        {
            continue;
        }
        // Keep the pinned endpoint (if any) as the survivor.
        let (keep, drop) = if s.pinned[b as usize] { (b, a) } else { (a, b) };
        let (p, cost) = s.best_position(keep, drop);
        if cost > limit {
            report.stop = StopReason::ErrorBound;
            break;
        }
        if !s.link_ok(keep, drop) {
            report.refused_link += 1;
            continue;
        }
        if !s.flip_ok(keep, drop, p, options.min_normal_cos) {
            report.refused_flip += 1;
            continue;
        }

        live -= s.collapse(keep, drop, p);
        report.collapses += 1;
        report.max_collapse_error = report.max_collapse_error.max(cost);

        // The one-ring changed: refresh every edge touching it.
        let mut touched = s.neighbours(keep);
        touched.push(keep);
        touched.sort_unstable();
        for &v in &touched {
            s.version[v as usize] += 1;
        }
        for &c in &touched {
            for d in s.neighbours(c) {
                if c < d || touched.binary_search(&d).is_err() {
                    push_edge(&mut heap, &s, c, d);
                }
            }
        }
    }

    // Compact the survivors in ascending source order.
    let mut new_index: Vec<u32> = vec![u32::MAX; n];
    let mut out_points: Vec<P3> = Vec::new();
    let mut vertex_map: Vec<u32> = Vec::new();
    let mut out_tris: Vec<[u32; 3]> = Vec::new();
    for (i, t) in s.tris.iter().enumerate() {
        if !s.tri_alive[i] {
            continue;
        }
        out_tris.push(*t);
    }
    let mut used: Vec<u32> = out_tris.iter().flatten().copied().collect();
    used.sort_unstable();
    used.dedup();
    for v in used {
        new_index[v as usize] = out_points.len() as u32;
        out_points.push(s.pos[v as usize]);
        vertex_map.push(v);
    }
    for t in &mut out_tris {
        for v in t.iter_mut() {
            *v = new_index[*v as usize];
        }
    }
    report.triangles_after = out_tris.len();
    Ok(Simplified { points: out_points, triangles: out_tris, vertex_map, report })
}

fn triangles_vertices(triangles: &[[u32; 3]], n: usize) -> Vec<bool> {
    let mut used = vec![false; n];
    for t in triangles {
        for &v in t {
            used[v as usize] = true;
        }
    }
    used
}

// ---------------------------------------------------------------- deviation

fn closest_on_triangle(p: P3, a: P3, b: P3, c: P3) -> P3 {
    // Ericson, Real-Time Collision Detection, 5.1.5.
    let (ab, ac, ap) = (sub(b, a), sub(c, a), sub(p, a));
    let (d1, d2) = (dot(ab, ap), dot(ac, ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = sub(p, b);
    let (d3, d4) = (dot(ab, bp), dot(ac, bp));
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return [a[0] + v * ab[0], a[1] + v * ab[1], a[2] + v * ab[2]];
    }
    let cp = sub(p, c);
    let (d5, d6) = (dot(ab, cp), dot(ac, cp));
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return [a[0] + w * ac[0], a[1] + w * ac[1], a[2] + w * ac[2]];
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        let bc = sub(c, b);
        return [b[0] + w * bc[0], b[1] + w * bc[1], b[2] + w * bc[2]];
    }
    let denom = 1.0 / (va + vb + vc);
    let (v, w) = (vb * denom, vc * denom);
    [a[0] + ab[0] * v + ac[0] * w, a[1] + ab[1] * v + ac[1] * w, a[2] + ab[2] * v + ac[2] * w]
}

struct TriangleGrid<'a> {
    points: &'a [P3],
    triangles: &'a [[u32; 3]],
    origin: P3,
    cell: f64,
    dims: [i64; 3],
    cells: HashMap<[i64; 3], Vec<u32>>,
}

impl<'a> TriangleGrid<'a> {
    fn new(points: &'a [P3], triangles: &'a [[u32; 3]]) -> Self {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        let mut edge_sum = 0.0;
        for t in triangles {
            for k in 0..3 {
                let p = points[t[k] as usize];
                for a in 0..3 {
                    lo[a] = lo[a].min(p[a]);
                    hi[a] = hi[a].max(p[a]);
                }
                edge_sum += len(sub(p, points[t[(k + 1) % 3] as usize]));
            }
        }
        let diag = len(sub(hi, lo)).max(1e-12);
        let mean_edge = if triangles.is_empty() { diag } else { edge_sum / (3.0 * triangles.len() as f64) };
        let cell = (2.0 * mean_edge).max(diag / 64.0);
        let dims = [0, 1, 2].map(|a| (((hi[a] - lo[a]) / cell).floor() as i64) + 1);
        let mut grid = Self { points, triangles, origin: lo, cell, dims, cells: HashMap::new() };
        for (i, t) in triangles.iter().enumerate() {
            let ps = t.map(|v| points[v as usize]);
            let mut min = [i64::MAX; 3];
            let mut max = [i64::MIN; 3];
            for p in ps {
                let c = grid.cell_of(p);
                for a in 0..3 {
                    min[a] = min[a].min(c[a]);
                    max[a] = max[a].max(c[a]);
                }
            }
            for x in min[0]..=max[0] {
                for y in min[1]..=max[1] {
                    for z in min[2]..=max[2] {
                        grid.cells.entry([x, y, z]).or_default().push(i as u32);
                    }
                }
            }
        }
        grid
    }

    fn cell_of(&self, p: P3) -> [i64; 3] {
        [0, 1, 2].map(|a| ((p[a] - self.origin[a]) / self.cell).floor() as i64)
    }

    /// Distance from `p` to the nearest triangle.
    fn distance(&self, p: P3) -> f64 {
        let c = self.cell_of(p);
        let reach = {
            let far = [0, 1, 2].map(|a| (c[a]).abs().max((self.dims[a] - c[a]).abs()));
            far.into_iter().max().unwrap_or(0) + 1
        };
        let mut best = f64::INFINITY;
        for r in 0..=reach {
            for x in c[0] - r..=c[0] + r {
                for y in c[1] - r..=c[1] + r {
                    for z in c[2] - r..=c[2] + r {
                        let ring = (x - c[0]).abs().max((y - c[1]).abs()).max((z - c[2]).abs());
                        if ring != r {
                            continue;
                        }
                        if let Some(list) = self.cells.get(&[x, y, z]) {
                            for &t in list {
                                let tri = self.triangles[t as usize];
                                let q = closest_on_triangle(
                                    p,
                                    self.points[tri[0] as usize],
                                    self.points[tri[1] as usize],
                                    self.points[tri[2] as usize],
                                );
                                best = best.min(len(sub(p, q)));
                            }
                        }
                    }
                }
            }
            // Anything in a farther shell is at least r * cell away.
            if best <= r as f64 * self.cell {
                break;
            }
        }
        best
    }
}

fn samples(points: &[P3], triangles: &[[u32; 3]]) -> Vec<P3> {
    let mut out = points.to_vec();
    for t in triangles {
        let (a, b, c) = (points[t[0] as usize], points[t[1] as usize], points[t[2] as usize]);
        for (p, q) in [(a, b), (b, c), (c, a)] {
            out.push([(p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0, (p[2] + q[2]) / 2.0]);
        }
        out.push([(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0, (a[2] + b[2] + c[2]) / 3.0]);
    }
    out
}

/// The larger of two one-sided distances between two surfaces: from sample
/// points of `a` (vertices, edge midpoints, centroids) to the triangles of `b`,
/// and the reverse. Not an exact Hausdorff distance, but it is a measurement of
/// the real surfaces, unlike the plane error `simplify` bounds. Empty input gives 0.
#[must_use]
pub fn sampled_deviation(a_points: &[P3], a_tris: &[[u32; 3]], b_points: &[P3], b_tris: &[[u32; 3]]) -> f64 {
    if a_tris.is_empty() || b_tris.is_empty() {
        return 0.0;
    }
    let one_way = |from_p: &[P3], from_t: &[[u32; 3]], to_p: &[P3], to_t: &[[u32; 3]]| {
        let grid = TriangleGrid::new(to_p, to_t);
        samples(from_p, from_t).into_iter().map(|p| grid.distance(p)).fold(0.0, f64::max)
    };
    one_way(a_points, a_tris, b_points, b_tris).max(one_way(b_points, b_tris, a_points, a_tris))
}

/// [`simplify`] with a deviation limit that is actually measured. It searches
/// for the largest `epsilon` whose result stays within `max_deviation` of the
/// source by [`sampled_deviation`] (at most `rounds` simplifications, at least
/// one) and returns the smallest such result. If even the first tries miss the
/// limit it falls back to the source surface unchanged, which deviates by zero.
/// `options.epsilon` is the search's starting value; `target_triangles` and
/// `min_normal_cos` are used as given.
pub fn simplify_within(
    points: &[P3],
    triangles: &[[u32; 3]],
    pinned: &[bool],
    max_deviation: f64,
    rounds: usize,
    options: &SimplifyOptions,
) -> Result<Simplified, SimplifyError> {
    if !(max_deviation.is_finite() && max_deviation > 0.0) {
        return Err(SimplifyError::BadOptions);
    }
    let mut best: Option<Simplified> = None;
    let (mut ok_eps, mut bad_eps) = (0.0f64, f64::INFINITY);
    let mut eps = options.epsilon;
    for _ in 0..rounds.max(1) {
        let mut result = simplify(points, triangles, pinned, &SimplifyOptions { epsilon: eps, ..*options })?;
        let d = sampled_deviation(points, triangles, &result.points, &result.triangles);
        result.report.measured_deviation = Some(d);
        if d <= max_deviation {
            ok_eps = ok_eps.max(eps);
            if best.as_ref().map_or(true, |b| result.triangles.len() <= b.triangles.len()) {
                best = Some(result);
            }
        } else {
            bad_eps = bad_eps.min(eps);
        }
        eps = if bad_eps.is_finite() {
            if ok_eps > 0.0 {
                (ok_eps + bad_eps) / 2.0
            } else {
                bad_eps * (max_deviation / d).min(0.9)
            }
        } else {
            ok_eps * 2.0
        };
    }
    Ok(best.unwrap_or_else(|| {
        let mut vertices: Vec<u32> = triangles.iter().flatten().copied().collect();
        vertices.sort_unstable();
        vertices.dedup();
        let mut index = vec![0u32; points.len()];
        for (i, &v) in vertices.iter().enumerate() {
            index[v as usize] = i as u32;
        }
        let mut report = SimplifyReport {
            triangles_before: triangles.len(),
            triangles_after: triangles.len(),
            collapses: 0,
            refused_link: 0,
            refused_flip: 0,
            max_collapse_error: 0.0,
            stop: StopReason::ErrorBound,
            measured_deviation: Some(0.0),
            epsilon_used: 0.0,
        };
        report.epsilon_used = ok_eps;
        Simplified {
            points: vertices.iter().map(|&v| points[v as usize]).collect(),
            triangles: triangles.iter().map(|t| t.map(|v| index[v as usize])).collect(),
            vertex_map: vertices,
            report,
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::region_surface::{is_closed_manifold, signed_volume};
    use std::collections::BTreeMap;

    /// Unit icosphere scaled by `radius`, subdivided `level` times, wound outward.
    fn sphere(radius: f64, level: u32) -> (Vec<P3>, Vec<[u32; 3]>) {
        let t = (1.0 + 5.0f64.sqrt()) / 2.0;
        let mut pts: Vec<P3> = vec![
            [-1.0, t, 0.0],
            [1.0, t, 0.0],
            [-1.0, -t, 0.0],
            [1.0, -t, 0.0],
            [0.0, -1.0, t],
            [0.0, 1.0, t],
            [0.0, -1.0, -t],
            [0.0, 1.0, -t],
            [t, 0.0, -1.0],
            [t, 0.0, 1.0],
            [-t, 0.0, -1.0],
            [-t, 0.0, 1.0],
        ];
        let mut tris: Vec<[u32; 3]> = vec![
            [0, 11, 5],
            [0, 5, 1],
            [0, 1, 7],
            [0, 7, 10],
            [0, 10, 11],
            [1, 5, 9],
            [5, 11, 4],
            [11, 10, 2],
            [10, 7, 6],
            [7, 1, 8],
            [3, 9, 4],
            [3, 4, 2],
            [3, 2, 6],
            [3, 6, 8],
            [3, 8, 9],
            [4, 9, 5],
            [2, 4, 11],
            [6, 2, 10],
            [8, 6, 7],
            [9, 8, 1],
        ];
        for _ in 0..level {
            let mut mid: BTreeMap<(u32, u32), u32> = BTreeMap::new();
            let mut next = Vec::new();
            for tri in &tris {
                let mut m = [0u32; 3];
                for k in 0..3 {
                    let (a, b) = (tri[k], tri[(k + 1) % 3]);
                    let key = (a.min(b), a.max(b));
                    m[k] = *mid.entry(key).or_insert_with(|| {
                        let (p, q) = (pts[a as usize], pts[b as usize]);
                        pts.push([(p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0, (p[2] + q[2]) / 2.0]);
                        pts.len() as u32 - 1
                    });
                }
                next.push([tri[0], m[0], m[2]]);
                next.push([tri[1], m[1], m[0]]);
                next.push([tri[2], m[2], m[1]]);
                next.push([m[0], m[1], m[2]]);
            }
            tris = next;
        }
        for p in &mut pts {
            let l = len(*p);
            *p = [p[0] / l * radius, p[1] / l * radius, p[2] / l * radius];
        }
        (pts, tris)
    }

    /// Torus with `u` by `v` quads, outward wound.
    fn torus(u: u32, v: u32) -> (Vec<P3>, Vec<[u32; 3]>) {
        let (big, small) = (6.0, 2.0);
        let mut pts = Vec::new();
        for i in 0..u {
            for j in 0..v {
                let (a, b) = (std::f64::consts::TAU * i as f64 / u as f64, std::f64::consts::TAU * j as f64 / v as f64);
                let r = big + small * b.cos();
                pts.push([r * a.cos(), small * b.sin(), r * a.sin()]);
            }
        }
        let id = |i: u32, j: u32| (i % u) * v + (j % v);
        let mut tris = Vec::new();
        for i in 0..u {
            for j in 0..v {
                let (p, q, r, s) = (id(i, j), id(i + 1, j), id(i + 1, j + 1), id(i, j + 1));
                tris.push([p, s, q]);
                tris.push([q, s, r]);
            }
        }
        (pts, tris)
    }

    /// A flat `n` by `n` grid of quads in the plane z = 0, normals +z.
    fn flat(n: u32) -> (Vec<P3>, Vec<[u32; 3]>) {
        let mut pts = Vec::new();
        for j in 0..=n {
            for i in 0..=n {
                pts.push([i as f64, j as f64, 0.0]);
            }
        }
        let id = |i: u32, j: u32| j * (n + 1) + i;
        let mut tris = Vec::new();
        for j in 0..n {
            for i in 0..n {
                tris.push([id(i, j), id(i + 1, j), id(i + 1, j + 1)]);
                tris.push([id(i, j), id(i + 1, j + 1), id(i, j + 1)]);
            }
        }
        (pts, tris)
    }

    fn euler(points: usize, triangles: &[[u32; 3]]) -> i64 {
        let mut edges = std::collections::BTreeSet::new();
        for t in triangles {
            for k in 0..3 {
                let (a, b) = (t[k], t[(k + 1) % 3]);
                edges.insert((a.min(b), a.max(b)));
            }
        }
        points as i64 - edges.len() as i64 + triangles.len() as i64
    }

    fn area(points: &[P3], triangles: &[[u32; 3]]) -> f64 {
        triangles
            .iter()
            .map(|t| len(normal(points[t[0] as usize], points[t[1] as usize], points[t[2] as usize])) / 2.0)
            .sum()
    }

    #[test]
    fn sphere_stays_a_closed_outward_sphere() {
        let (p, t) = sphere(10.0, 3);
        let volume = signed_volume(&p, &t);
        let options = SimplifyOptions { epsilon: 0.3, target_triangles: 200, ..Default::default() };
        let s = simplify(&p, &t, &[], &options).unwrap();
        assert!(s.report.triangles_after < t.len() / 3, "{:?}", s.report);
        assert!(is_closed_manifold(&s.triangles));
        assert_eq!(euler(s.points.len(), &s.triangles), 2);
        let v = signed_volume(&s.points, &s.triangles);
        assert!(v > 0.0 && ((v - volume) / volume).abs() < 0.10, "volume {v} vs {volume}");
        for tri in &s.triangles {
            let [a, b, c] = tri.map(|i| s.points[i as usize]);
            let centre = [(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0, (a[2] + b[2] + c[2]) / 3.0];
            assert!(dot(normal(a, b, c), centre) > 0.0, "an inverted triangle");
        }
        assert_eq!(s.vertex_map.len(), s.points.len());
        assert!(s.report.max_collapse_error <= options.epsilon + 1e-12);
    }

    #[test]
    fn the_measured_deviation_search_honours_its_limit() {
        let (p, t) = sphere(10.0, 3);
        let plain = simplify(&p, &t, &[], &SimplifyOptions { epsilon: 0.3, ..Default::default() }).unwrap();
        let d_plain = sampled_deviation(&p, &t, &plain.points, &plain.triangles);
        // The root mean square plane error is a heuristic: it can be exceeded.
        assert!(d_plain > 0.3, "if the heuristic became a bound, say so in the docs: {d_plain}");
        let s = simplify_within(&p, &t, &[], 0.3, 8, &SimplifyOptions { epsilon: 0.3, ..Default::default() }).unwrap();
        let d = sampled_deviation(&p, &t, &s.points, &s.triangles);
        assert!(d <= 0.3, "deviation {d}");
        assert_eq!(s.report.measured_deviation, Some(d));
        assert!(s.triangles.len() < t.len() / 2, "{:?}", s.report);
        assert!(is_closed_manifold(&s.triangles));
    }

    #[test]
    fn an_impossible_limit_falls_back_to_the_source() {
        let (p, t) = sphere(10.0, 1);
        let s = simplify_within(&p, &t, &[], 1e-9, 3, &SimplifyOptions::default()).unwrap();
        assert_eq!(s.triangles.len(), t.len());
        assert!(s.report.measured_deviation.unwrap() < 1e-9);
        assert!(is_closed_manifold(&s.triangles));
    }

    #[test]
    fn a_tiny_epsilon_changes_nothing_on_a_cube() {
        let pts: Vec<P3> = (0..8u32)
            .map(|i| [(i & 1) as f64, ((i >> 1) & 1) as f64, ((i >> 2) & 1) as f64])
            .collect();
        let tris: Vec<[u32; 3]> = vec![
            [0, 2, 1],
            [1, 2, 3],
            [4, 5, 6],
            [5, 7, 6],
            [0, 1, 4],
            [1, 5, 4],
            [2, 6, 3],
            [3, 6, 7],
            [0, 4, 2],
            [2, 4, 6],
            [1, 3, 5],
            [3, 7, 5],
        ];
        assert!(signed_volume(&pts, &tris) > 0.0);
        let s = simplify(&pts, &tris, &[], &SimplifyOptions { epsilon: 0.01, ..Default::default() }).unwrap();
        assert_eq!(s.triangles.len(), 12);
        assert_eq!(s.report.collapses, 0);
        assert_eq!(s.report.stop, StopReason::ErrorBound);
    }

    #[test]
    fn a_flat_patch_loses_its_interior_but_keeps_boundary_area_and_normals() {
        let (p, t) = flat(10);
        let s = simplify(&p, &t, &[], &SimplifyOptions::default()).unwrap();
        assert!(s.triangles.len() < t.len() / 2, "{}", s.triangles.len());
        assert!((area(&s.points, &s.triangles) - 100.0).abs() < 1e-9);
        for tri in &s.triangles {
            let [a, b, c] = tri.map(|i| s.points[i as usize]);
            let n = normal(a, b, c);
            assert!(n[2] > 0.0 && n[0] == 0.0 && n[1] == 0.0);
        }
        // Every boundary vertex survives at its own coordinates.
        let boundary = p.iter().enumerate().filter(|(_, q)| q[0] == 0.0 || q[0] == 10.0 || q[1] == 0.0 || q[1] == 10.0);
        for (i, q) in boundary {
            let at = s.vertex_map.iter().position(|&v| v as usize == i).expect("boundary vertex kept");
            assert_eq!(s.points[at], *q);
        }
        assert!(!is_closed_manifold(&s.triangles));
    }

    #[test]
    fn pinned_vertices_are_kept_exactly() {
        let (p, t) = sphere(10.0, 2);
        let pinned: Vec<bool> = p.iter().map(|q| q[1] > 4.0).collect();
        let s = simplify(&p, &t, &pinned, &SimplifyOptions { epsilon: 1.0, ..Default::default() }).unwrap();
        for (i, q) in p.iter().enumerate() {
            if pinned[i] {
                let at = s.vertex_map.iter().position(|&v| v as usize == i).expect("pinned vertex kept");
                assert_eq!(s.points[at], *q);
            }
        }
        assert!(s.triangles.len() < t.len());
        assert!(is_closed_manifold(&s.triangles));
    }

    #[test]
    fn two_cut_patches_simplified_apart_still_close_along_the_cut() {
        let (p, t) = sphere(10.0, 3);
        // Cut at y = 0: assign each triangle by its centroid, so the cut follows edges.
        let upper = |tri: &[u32; 3]| tri.iter().map(|&v| p[v as usize][1]).sum::<f64>() > 0.0;
        let top: Vec<[u32; 3]> = t.iter().copied().filter(|tri| upper(tri)).collect();
        let bottom: Vec<[u32; 3]> = t.iter().copied().filter(|tri| !upper(tri)).collect();
        let options = SimplifyOptions { epsilon: 0.3, ..Default::default() };
        let (a, b) = (simplify(&p, &top, &[], &options).unwrap(), simplify(&p, &bottom, &[], &options).unwrap());
        assert!(a.triangles.len() < top.len() && b.triangles.len() < bottom.len());
        // Merge by source vertex: pinned boundary vertices coincide exactly.
        let mut merged_points: Vec<P3> = Vec::new();
        let mut index: HashMap<[u64; 3], u32> = HashMap::new();
        let mut merged: Vec<[u32; 3]> = Vec::new();
        for part in [&a, &b] {
            for tri in &part.triangles {
                let mut out = [0u32; 3];
                for k in 0..3 {
                    let q = part.points[tri[k] as usize];
                    let key = q.map(f64::to_bits);
                    out[k] = *index.entry(key).or_insert_with(|| {
                        merged_points.push(q);
                        merged_points.len() as u32 - 1
                    });
                }
                merged.push(out);
            }
        }
        assert!(is_closed_manifold(&merged), "the two patches do not close");
        assert_eq!(euler(merged_points.len(), &merged), 2);
    }

    #[test]
    fn a_torus_keeps_its_genus_even_when_pushed_hard() {
        let (p, t) = torus(24, 12);
        assert!(is_closed_manifold(&t));
        let before = signed_volume(&p, &t);
        assert!(before > 0.0);
        let s = simplify(&p, &t, &[], &SimplifyOptions { epsilon: 100.0, target_triangles: 8, ..Default::default() }).unwrap();
        assert!(s.triangles.len() < t.len() / 4, "{:?}", s.report);
        assert!(is_closed_manifold(&s.triangles));
        assert_eq!(euler(s.points.len(), &s.triangles), 0);
        assert!(s.report.refused_link > 0, "the link condition was never exercised");
    }

    #[test]
    fn the_target_stops_the_run() {
        let (p, t) = sphere(5.0, 3);
        let s = simplify(&p, &t, &[], &SimplifyOptions { epsilon: 10.0, target_triangles: 500, ..Default::default() }).unwrap();
        assert_eq!(s.report.stop, StopReason::TargetReached);
        assert!(s.triangles.len() <= 500 && s.triangles.len() >= 496);
    }

    #[test]
    fn the_result_is_deterministic() {
        let (p, t) = sphere(10.0, 3);
        let o = SimplifyOptions { epsilon: 0.5, ..Default::default() };
        assert_eq!(simplify(&p, &t, &[], &o).unwrap(), simplify(&p, &t, &[], &o).unwrap());
    }

    #[test]
    fn bad_input_is_reported() {
        let (p, t) = flat(2);
        let o = SimplifyOptions::default();
        assert_eq!(simplify(&p, &t, &[], &SimplifyOptions { epsilon: 0.0, ..o }), Err(SimplifyError::BadOptions));
        assert_eq!(
            simplify(&p, &t, &[true], &o),
            Err(SimplifyError::PinnedLength { pinned: 1, points: p.len() })
        );
        assert_eq!(simplify(&p, &[[0, 1, 99]], &[], &o), Err(SimplifyError::IndexOutOfRange { triangle: 0 }));
        assert_eq!(simplify(&p, &[[0, 1, 1]], &[], &o), Err(SimplifyError::DegenerateTriangle { triangle: 0 }));
        let mut bad = p.clone();
        bad[3][0] = f64::NAN;
        assert_eq!(simplify(&bad, &t, &[], &o), Err(SimplifyError::NonFinitePoint { index: 3 }));
        // Two triangles using edge 0->1 the same way.
        assert_eq!(simplify(&p, &[[0, 1, 4], [0, 1, 3]], &[], &o), Err(SimplifyError::Inconsistent { edges: 1 }));
        // Three triangles on one edge must repeat a direction.
        assert_eq!(
            simplify(&p, &[[0, 1, 4], [1, 0, 3], [0, 1, 5]], &[], &o),
            Err(SimplifyError::Inconsistent { edges: 1 })
        );
    }

    #[test]
    fn deviation_of_identical_and_offset_surfaces() {
        let (p, t) = flat(4);
        assert!(sampled_deviation(&p, &t, &p, &t) < 1e-12);
        let lifted: Vec<P3> = p.iter().map(|q| [q[0], q[1], 0.25]).collect();
        let d = sampled_deviation(&p, &t, &lifted, &t);
        assert!((d - 0.25).abs() < 1e-12, "{d}");
        assert_eq!(sampled_deviation(&p, &[], &p, &t), 0.0);
    }
}
