//! Section plane with capped cut faces for structure geometry.
//! Design: `docs/sections-and-peeling.md`.
//!
//! Every part of a [`StructureGeometry`] must be made of closed, outward-wound
//! bodies. The part is clipped by the plane, the cut outline is chained into
//! loops, and the loops are triangulated into a cap that faces the plane
//! normal (the removed side). All in f64 and graphics-free.

use std::{collections::HashMap, error::Error, fmt};

use crate::{
    geometry::{GeometryError, TriangleMesh3},
    spatial::{WorldPoint3, WorldVector3},
    structure::{PartRange, StructureGeometry},
};

type V3 = [f64; 3];
type P2 = [f64; 2];

/// Relative distances from the plane inside which a vertex counts as on it,
/// tried in turn. A plane lying along an edge or face of a body gives a cap
/// sliver too thin to triangulate; the next, larger value moves the plane
/// further off it (at most 1e-4 of the structure's size, far below what is
/// visible).
const ON_PLANE_STEPS: [f64; 3] = [1e-9, 1e-6, 1e-4];

/// A cutting plane. The side where `normal . (p - point) > 0` is removed; the
/// cap faces along `normal`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SectionPlane {
    point: V3,
    normal: V3,
}

impl SectionPlane {
    /// `normal` must be finite and non-zero; it is normalised.
    pub fn new(point: WorldPoint3, normal: WorldVector3) -> Result<Self, SectionError> {
        let point = point.to_array();
        let normal = normal.to_vector().to_array();
        let length = dot(normal, normal).sqrt();
        if !point.iter().chain(&normal).all(|c| c.is_finite()) || !(length > 0.0) || !length.is_finite() {
            return Err(SectionError::InvalidPlane);
        }
        Ok(Self { point, normal: normal.map(|c| c / length) })
    }

    /// The same plane keeping the other side.
    #[must_use]
    pub fn flipped(&self) -> Self {
        Self { point: self.point, normal: self.normal.map(|c| -c) }
    }

    #[must_use]
    pub fn point(&self) -> WorldPoint3 {
        WorldPoint3::new(self.point[0], self.point[1], self.point[2])
    }

    #[must_use]
    pub fn normal(&self) -> WorldVector3 {
        WorldVector3::new(self.normal[0], self.normal[1], self.normal[2])
    }
}

/// Why a section could not be built.
#[derive(Clone, Debug, PartialEq)]
pub enum SectionError {
    InvalidPlane,
    /// The part (index into the structure) is not a closed manifold.
    NotClosed { part: usize },
    /// The cut outline of the part could not be triangulated.
    CapFailed { part: usize },
    /// No plane position could be found away from every vertex.
    DegeneratePlane,
    Geometry(GeometryError),
}

impl fmt::Display for SectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPlane => write!(f, "section plane needs a finite point and a non-zero normal"),
            Self::NotClosed { part } => write!(f, "part {part} is not a closed surface, so it cannot be capped"),
            Self::CapFailed { part } => write!(f, "the cut of part {part} could not be capped"),
            Self::DegeneratePlane => write!(f, "section plane coincides with the geometry"),
            Self::Geometry(error) => write!(f, "sectioned geometry is invalid: {error}"),
        }
    }
}

impl Error for SectionError {}

impl From<GeometryError> for SectionError {
    fn from(error: GeometryError) -> Self {
        Self::Geometry(error)
    }
}

/// Clip every part by the plane and cap the cuts. The result keeps one
/// contiguous triangle range per remaining part (clipped body, then its cap),
/// so a click on a cap still names its tissue. Parts wholly removed have no
/// range; a plane that removes everything gives an empty mesh.
pub fn section_geometry(
    geometry: &StructureGeometry,
    plane: &SectionPlane,
) -> Result<StructureGeometry, SectionError> {
    let mut result = section_with(geometry, plane, ON_PLANE_STEPS[0]);
    for &step in &ON_PLANE_STEPS[1..] {
        if !matches!(result, Err(SectionError::CapFailed { .. })) {
            break;
        }
        result = section_with(geometry, plane, step);
    }
    result
}

fn section_with(
    geometry: &StructureGeometry,
    plane: &SectionPlane,
    relative_eps: f64,
) -> Result<StructureGeometry, SectionError> {
    let vertices: Vec<V3> = geometry.mesh().vertices().iter().map(|p| p.to_array()).collect();
    let indices = geometry.mesh().indices();

    let mut minimum = [f64::INFINITY; 3];
    let mut maximum = [f64::NEG_INFINITY; 3];
    for v in &vertices {
        for axis in 0..3 {
            minimum[axis] = minimum[axis].min(v[axis]);
            maximum[axis] = maximum[axis].max(v[axis]);
        }
    }
    let extent = if vertices.is_empty() {
        1.0
    } else {
        dot(sub(maximum, minimum), sub(maximum, minimum)).sqrt().max(f64::MIN_POSITIVE)
    };
    let eps = relative_eps * extent;

    // Move the plane a hair along its normal until no vertex lies on it.
    let base = dot(plane.normal, plane.point);
    let mut distances = Vec::new();
    let mut found = false;
    for attempt in 0..16 {
        let shift = if attempt == 0 { 0.0 } else { 3.0 * eps * attempt as f64 };
        distances = vertices.iter().map(|v| dot(plane.normal, *v) - (base + shift)).collect();
        if distances.iter().all(|d| d.abs() > eps) {
            found = true;
            break;
        }
    }
    if !found {
        return Err(SectionError::DegeneratePlane);
    }
    let origin = plane.point;
    let colours = geometry.colours();

    let mut out_points: Vec<V3> = Vec::new();
    let mut out_colours: Vec<[f64; 3]> = Vec::new();
    let mut out_indices: Vec<u32> = Vec::new();
    let mut out_ranges: Vec<PartRange> = Vec::new();

    for range in geometry.ranges() {
        let first = out_indices.len() / 3;
        let triangles = &indices[range.first_triangle * 3..(range.first_triangle + range.triangle_count) * 3];
        section_part(
            range.part_index,
            triangles,
            &vertices,
            colours,
            &distances,
            plane.normal,
            origin,
            &mut out_points,
            &mut out_colours,
            &mut out_indices,
        )?;
        let count = out_indices.len() / 3 - first;
        if count > 0 {
            out_ranges.push(PartRange { part_index: range.part_index, first_triangle: first, triangle_count: count });
        }
    }

    let points = out_points.iter().map(|p| WorldPoint3::new(p[0], p[1], p[2])).collect();
    Ok(StructureGeometry::from_parts(TriangleMesh3::new(points, out_indices)?, out_colours, out_ranges))
}

#[allow(clippy::too_many_arguments)]
fn section_part(
    part: usize,
    triangles: &[u32],
    vertices: &[V3],
    colours: &[[f64; 3]],
    distances: &[f64],
    normal: V3,
    origin: V3,
    out_points: &mut Vec<V3>,
    out_colours: &mut Vec<[f64; 3]>,
    out_indices: &mut Vec<u32>,
) -> Result<(), SectionError> {
    let mut kept: HashMap<u32, u32> = HashMap::new();
    let mut cut: HashMap<(u32, u32), u32> = HashMap::new();
    let mut segments: Vec<(u32, u32)> = Vec::new();

    let mut keep = |old: u32, points: &mut Vec<V3>, cols: &mut Vec<[f64; 3]>| -> u32 {
        *kept.entry(old).or_insert_with(|| {
            points.push(vertices[old as usize]);
            cols.push(colours[old as usize]);
            (points.len() - 1) as u32
        })
    };
    let mut cut_vertex = |a: u32, b: u32, points: &mut Vec<V3>, cols: &mut Vec<[f64; 3]>| -> u32 {
        // Sorted so both triangles sharing the edge compute identical bits.
        let (a, b) = if a < b { (a, b) } else { (b, a) };
        *cut.entry((a, b)).or_insert_with(|| {
            let (va, vb) = (vertices[a as usize], vertices[b as usize]);
            let (da, db) = (distances[a as usize], distances[b as usize]);
            let t = da / (da - db);
            points.push([
                va[0] + (vb[0] - va[0]) * t,
                va[1] + (vb[1] - va[1]) * t,
                va[2] + (vb[2] - va[2]) * t,
            ]);
            cols.push(colours[a as usize]);
            (points.len() - 1) as u32
        })
    };

    for triangle in triangles.chunks_exact(3) {
        let outside = [0, 1, 2].map(|i| distances[triangle[i] as usize] > 0.0);
        match outside.iter().filter(|&&o| o).count() {
            3 => {}
            0 => {
                let mapped = [0, 1, 2].map(|i| keep(triangle[i], out_points, out_colours));
                out_indices.extend(mapped);
            }
            _ => {
                let mut polygon: Vec<u32> = Vec::with_capacity(4);
                let (mut exit, mut entry) = (None, None);
                for i in 0..3 {
                    let (p, q) = (triangle[i], triangle[(i + 1) % 3]);
                    match (outside[i], outside[(i + 1) % 3]) {
                        (false, false) => polygon.push(keep(q, out_points, out_colours)),
                        (false, true) => {
                            let x = cut_vertex(p, q, out_points, out_colours);
                            polygon.push(x);
                            exit = Some(x);
                        }
                        (true, false) => {
                            let x = cut_vertex(p, q, out_points, out_colours);
                            polygon.push(x);
                            entry = Some(x);
                            polygon.push(keep(q, out_points, out_colours));
                        }
                        (true, true) => {}
                    }
                }
                for i in 1..polygon.len() - 1 {
                    out_indices.extend([polygon[0], polygon[i], polygon[i + 1]]);
                }
                if let (Some(from), Some(to)) = (exit, entry) {
                    segments.push((from, to));
                }
            }
        }
    }

    if segments.is_empty() {
        return Ok(());
    }

    // Chain the cut segments into closed loops.
    let mut next: HashMap<u32, u32> = HashMap::new();
    let mut targets: HashMap<u32, u32> = HashMap::new();
    for &(from, to) in &segments {
        if next.insert(from, to).is_some() || targets.insert(to, from).is_some() {
            return Err(SectionError::NotClosed { part });
        }
    }
    if segments.iter().any(|(_, to)| !next.contains_key(to)) {
        return Err(SectionError::NotClosed { part });
    }
    let mut visited: HashMap<u32, ()> = HashMap::new();
    let mut loops: Vec<Vec<u32>> = Vec::new();
    for &(start, _) in &segments {
        if visited.contains_key(&start) {
            continue;
        }
        let mut ring = Vec::new();
        let mut current = start;
        loop {
            visited.insert(current, ());
            ring.push(current);
            current = next[&current];
            if current == start {
                break;
            }
        }
        // Wall winding runs the opposite way round the cap's outward normal.
        ring.reverse();
        loops.push(ring);
    }

    // Planar coordinates: u x v = normal.
    let helper = {
        let mut axis = 0;
        for i in 1..3 {
            if normal[i].abs() < normal[axis].abs() {
                axis = i;
            }
        }
        let mut h = [0.0; 3];
        h[axis] = 1.0;
        h
    };
    let u = normalise(cross(helper, normal));
    let v = cross(normal, u);
    let project = |id: u32| -> (u32, P2) {
        let d = sub(out_points[id as usize], origin);
        (id, [dot(d, u), dot(d, v)])
    };
    let rings: Vec<Vec<(u32, P2)>> =
        loops.iter().map(|ring| ring.iter().map(|&id| project(id)).collect()).collect();

    let cap = cap_triangles(&rings).ok_or(SectionError::CapFailed { part })?;
    out_indices.extend(cap.into_iter().flatten());
    Ok(())
}

/// Triangulate loops that are counter-clockwise (outer) or clockwise (hole)
/// about the cap normal. Returns triangles counter-clockwise in the plane.
fn cap_triangles(rings: &[Vec<(u32, P2)>]) -> Option<Vec<[u32; 3]>> {
    let areas: Vec<f64> = rings.iter().map(|r| signed_area(r)).collect();
    let mut low = [f64::INFINITY; 2];
    let mut high = [f64::NEG_INFINITY; 2];
    for (_, p) in rings.iter().flatten() {
        for axis in 0..2 {
            low[axis] = low[axis].min(p[axis]);
            high[axis] = high[axis].max(p[axis]);
        }
    }
    let scale = (high[0] - low[0]).hypot(high[1] - low[1]);
    let eps = 1e-13 * scale * scale;
    if areas.iter().any(|a| a.abs() <= eps) {
        return None;
    }

    let outers: Vec<usize> = (0..rings.len()).filter(|&i| areas[i] > 0.0).collect();
    let mut holes_of: Vec<Vec<usize>> = vec![Vec::new(); rings.len()];
    for hole in (0..rings.len()).filter(|&i| areas[i] < 0.0) {
        let probe = rings[hole][0].1;
        let owner = outers
            .iter()
            .copied()
            .filter(|&o| inside(&rings[o], probe))
            .min_by(|&a, &b| areas[a].total_cmp(&areas[b]))?;
        holes_of[owner].push(hole);
    }

    let mut result = Vec::new();
    for &outer in &outers {
        let holes: Vec<&Vec<(u32, P2)>> = holes_of[outer].iter().map(|&h| &rings[h]).collect();
        let expected = areas[outer] + holes_of[outer].iter().map(|&h| areas[h]).sum::<f64>();
        let polygon = bridge_holes(&rings[outer], &holes, eps)?;
        let triangles = ear_clip(polygon, eps)?;
        let total: f64 = triangles.iter().map(triangle_area).sum();
        if (total - expected).abs() > 1e-9 * expected.abs().max(eps) {
            return None;
        }
        result.extend(triangles.into_iter().map(|t| [t[0].0, t[1].0, t[2].0]));
    }
    Some(result)
}

fn triangle_area(t: &[(u32, P2); 3]) -> f64 {
    cross2(t[0].1, t[1].1, t[2].1) / 2.0
}

/// Merge each hole into the outer polygon with a zero-width bridge between
/// mutually visible vertices, giving one weakly simple polygon.
fn bridge_holes(outer: &[(u32, P2)], holes: &[&Vec<(u32, P2)>], eps: f64) -> Option<Vec<(u32, P2)>> {
    let mut polygon = outer.to_vec();
    let mut order: Vec<usize> = (0..holes.len()).collect();
    let max_x = |h: &Vec<(u32, P2)>| h.iter().map(|(_, p)| p[0]).fold(f64::NEG_INFINITY, f64::max);
    order.sort_by(|&a, &b| max_x(holes[b]).total_cmp(&max_x(holes[a])));

    for (n, &h) in order.iter().enumerate() {
        let hole = holes[h];
        // Rightmost hole vertex.
        let hm = (0..hole.len()).max_by(|&a, &b| hole[a].1[0].total_cmp(&hole[b].1[0]))?;
        let m = hole[hm].1;
        let remaining: Vec<&Vec<(u32, P2)>> = order[n..].iter().map(|&i| holes[i]).collect();

        let mut candidates: Vec<usize> = (0..polygon.len()).collect();
        candidates.sort_by(|&a, &b| dist2(polygon[a].1, m).total_cmp(&dist2(polygon[b].1, m)));
        let pi = candidates.into_iter().find(|&pi| {
            let p = polygon[pi].1;
            let midpoint = [(m[0] + p[0]) / 2.0, (m[1] + p[1]) / 2.0];
            inside(&polygon, midpoint)
                && !inside(hole, midpoint)
                && !crosses_ring(&polygon, m, p, eps)
                && remaining.iter().all(|r| !crosses_ring(r, m, p, eps))
        })?;

        let mut merged = Vec::with_capacity(polygon.len() + hole.len() + 2);
        merged.extend_from_slice(&polygon[..=pi]);
        merged.extend((0..=hole.len()).map(|k| hole[(hm + k) % hole.len()]));
        merged.extend_from_slice(&polygon[pi..]);
        polygon = merged;
    }
    Some(polygon)
}

/// Does the open segment `a-b` touch any edge of `ring` other than at the
/// segment's own end points?
fn crosses_ring(ring: &[(u32, P2)], a: P2, b: P2, eps: f64) -> bool {
    (0..ring.len()).any(|i| {
        let (c, d) = (ring[i].1, ring[(i + 1) % ring.len()].1);
        let shares = |p: P2| same(p, a) || same(p, b);
        if shares(c) || shares(d) {
            return false;
        }
        segments_touch(a, b, c, d, eps)
    })
}

fn segments_touch(a: P2, b: P2, c: P2, d: P2, eps: f64) -> bool {
    let sign = |x: f64| if x.abs() <= eps { 0 } else if x > 0.0 { 1 } else { -1 };
    let o1 = sign(cross2(a, b, c));
    let o2 = sign(cross2(a, b, d));
    let o3 = sign(cross2(c, d, a));
    let o4 = sign(cross2(c, d, b));
    if o1 == 0 && o2 == 0 {
        // Collinear: overlap of the projections on the longer axis.
        let axis = usize::from((b[1] - a[1]).abs() > (b[0] - a[0]).abs());
        let (amin, amax) = (a[axis].min(b[axis]), a[axis].max(b[axis]));
        let (cmin, cmax) = (c[axis].min(d[axis]), c[axis].max(d[axis]));
        return amax >= cmin && cmax >= amin;
    }
    o1 * o2 <= 0 && o3 * o4 <= 0
}

/// Ear clipping. Prefers ears of positive area; collinear vertices stay until
/// last, then close as zero-area triangles so shared edges remain watertight.
fn ear_clip(mut polygon: Vec<(u32, P2)>, eps: f64) -> Option<Vec<[(u32, P2); 3]>> {
    let mut triangles = Vec::new();
    while polygon.len() > 3 {
        let n = polygon.len();
        let find = |minimum_area: f64| {
            (0..n).find(|&i| {
                let (ip, inext) = ((i + n - 1) % n, (i + 1) % n);
                let (prev, cur, next) = (polygon[ip], polygon[i], polygon[inext]);
                if prev.0 == cur.0 || cur.0 == next.0 || prev.0 == next.0 {
                    return false;
                }
                if cross2(prev.1, cur.1, next.1) < minimum_area {
                    return false;
                }
                !polygon.iter().enumerate().any(|(j, q)| {
                    j != ip
                        && j != i
                        && j != inext
                        && !same(q.1, prev.1)
                        && !same(q.1, cur.1)
                        && !same(q.1, next.1)
                        && strictly_inside(prev.1, cur.1, next.1, q.1, eps)
                })
            })
        };
        let i = find(eps).or_else(|| find(-eps))?;
        let (prev, next) = (polygon[(i + n - 1) % n], polygon[(i + 1) % n]);
        triangles.push([prev, polygon[i], next]);
        polygon.remove(i);
    }
    if polygon.len() == 3 && polygon[0].0 != polygon[1].0 && polygon[1].0 != polygon[2].0 && polygon[0].0 != polygon[2].0 {
        triangles.push([polygon[0], polygon[1], polygon[2]]);
    } else if polygon.len() == 3 {
        return None;
    }
    Some(triangles)
}

fn strictly_inside(a: P2, b: P2, c: P2, q: P2, eps: f64) -> bool {
    cross2(a, b, q) > eps && cross2(b, c, q) > eps && cross2(c, a, q) > eps
}

fn inside(ring: &[(u32, P2)], q: P2) -> bool {
    let mut result = false;
    for i in 0..ring.len() {
        let (a, b) = (ring[i].1, ring[(i + 1) % ring.len()].1);
        if (a[1] > q[1]) != (b[1] > q[1]) && q[0] < (b[0] - a[0]) * (q[1] - a[1]) / (b[1] - a[1]) + a[0] {
            result = !result;
        }
    }
    result
}

fn signed_area(ring: &[(u32, P2)]) -> f64 {
    (0..ring.len())
        .map(|i| {
            let (a, b) = (ring[i].1, ring[(i + 1) % ring.len()].1);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        / 2.0
}

/// Twice the signed area of (a, b, c); positive when counter-clockwise.
fn cross2(a: P2, b: P2, c: P2) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

fn same(a: P2, b: P2) -> bool {
    a[0] == b[0] && a[1] == b[1]
}

fn dist2(a: P2, b: P2) -> f64 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)
}

fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn normalise(a: V3) -> V3 {
    let l = dot(a, a).sqrt();
    a.map(|c| c / l)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structure::Structure;
    use std::f64::consts::TAU;

    const STEM: &str = include_str!("../../../fixtures/structures/dicot-stem.ron");

    fn plane(point: V3, normal: V3) -> SectionPlane {
        SectionPlane::new(
            WorldPoint3::new(point[0], point[1], point[2]),
            WorldVector3::new(normal[0], normal[1], normal[2]),
        )
        .unwrap()
    }

    fn stem() -> (Structure, StructureGeometry) {
        let s = Structure::from_ron_str(STEM).unwrap();
        let g = s.build_geometry().unwrap();
        (s, g)
    }

    fn one_part(geometry: &str) -> StructureGeometry {
        let text = format!(
            "Structure(version: 1, name: \"t\", units: \"mm\", reference: \"r\", circle_segments: 48, parts: [\
             Part(id: \"p\", name: \"p\", parent: None, description: \"d\", colour: (0.5, 0.5, 0.5), geometry: {geometry})])"
        );
        Structure::from_ron_str(&text).unwrap().build_geometry().unwrap()
    }

    fn annulus() -> StructureGeometry {
        one_part("Some(Extrusion(axis_length: 10.0, profile: Annulus(inner: 2.0, outer: 3.0)))")
    }

    fn assert_closed(mesh: &TriangleMesh3) {
        let mut directed: HashMap<(u32, u32), usize> = HashMap::new();
        for t in mesh.indices().chunks_exact(3) {
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                *directed.entry((a, b)).or_default() += 1;
            }
        }
        for (&(a, b), &n) in &directed {
            assert_eq!(n, 1, "directed edge ({a},{b}) used {n} times");
            assert_eq!(directed.get(&(b, a)), Some(&1), "edge ({a},{b}) has no opposite edge");
        }
    }

    fn signed_volume(mesh: &TriangleMesh3) -> f64 {
        let v = mesh.vertices();
        mesh.indices()
            .chunks_exact(3)
            .map(|t| {
                let (a, b, c) = (v[t[0] as usize].to_array(), v[t[1] as usize].to_array(), v[t[2] as usize].to_array());
                dot(a, cross(b, c)) / 6.0
            })
            .sum()
    }

    /// Area of the triangles of one range that lie in the plane.
    fn cap_area(g: &StructureGeometry, r: &PartRange, p: &SectionPlane) -> f64 {
        let v = g.mesh().vertices();
        let n = p.normal.map(|c| c);
        g.mesh().indices()[r.first_triangle * 3..(r.first_triangle + r.triangle_count) * 3]
            .chunks_exact(3)
            .filter(|t| {
                t.iter().all(|&i| (dot(n, sub(v[i as usize].to_array(), p.point))).abs() < 1e-6)
            })
            .map(|t| {
                let (a, b, c) = (v[t[0] as usize].to_array(), v[t[1] as usize].to_array(), v[t[2] as usize].to_array());
                let x = cross(sub(b, a), sub(c, a));
                let length = dot(x, x).sqrt();
                let signed = dot(x, n);
                // Wall slivers beside the cut are also near the plane; only
                // triangles lying flat in it are cap triangles.
                if length < 1e-9 || signed.abs() < 0.999 * length {
                    return 0.0;
                }
                assert!(signed > 0.0, "cap triangle faces the wrong way");
                signed / 2.0
            })
            .sum()
    }

    fn range_of(g: &StructureGeometry, part: usize) -> Option<PartRange> {
        g.ranges().iter().copied().find(|r| r.part_index == part)
    }

    fn ring_area(segments: usize, outer: f64, inner: f64, step: f64) -> f64 {
        segments as f64 / 2.0 * (outer * outer - inner * inner) * step.sin()
    }

    #[test]
    fn plane_rejects_bad_input_and_flips() {
        let bad = SectionPlane::new(WorldPoint3::new(0.0, 0.0, 0.0), WorldVector3::new(0.0, 0.0, 0.0));
        assert_eq!(bad.unwrap_err(), SectionError::InvalidPlane);
        let nan = SectionPlane::new(WorldPoint3::new(f64::NAN, 0.0, 0.0), WorldVector3::new(0.0, 1.0, 0.0));
        assert_eq!(nan.unwrap_err(), SectionError::InvalidPlane);
        let p = plane([0.0; 3], [0.0, 3.0, 0.0]);
        assert_eq!(p.normal().to_vector().to_array(), [0.0, 1.0, 0.0]);
        assert_eq!(p.flipped().normal().to_vector().to_array(), [0.0, -1.0, 0.0]);
    }

    #[test]
    fn annulus_cut_across_is_closed_with_the_exact_volume_and_a_ringed_cap() {
        let g = annulus();
        let p = plane([0.0, 4.0, 0.0], [0.0, 1.0, 0.0]);
        let s = section_geometry(&g, &p).unwrap();
        assert_closed(s.mesh());
        let ring = ring_area(48, 3.0, 2.0, TAU / 48.0);
        assert!((signed_volume(s.mesh()) - ring * 4.0).abs() < 1e-9);
        let r = s.ranges()[0];
        assert!((cap_area(&s, &r, &p) - ring).abs() < 1e-9, "hole must stay empty");
    }

    #[test]
    fn cut_exactly_through_vertices_is_nudged_and_still_closed() {
        let g = annulus();
        for y in [0.0, 10.0] {
            let s = section_geometry(&g, &plane([0.0, y, 0.0], [0.0, 1.0, 0.0])).unwrap();
            assert_closed(s.mesh());
        }
    }

    #[test]
    fn both_sides_of_a_cut_tile_the_body() {
        let g = annulus();
        let p = plane([0.0, 3.3, 0.0], [0.0, 1.0, 0.0]);
        let kept = signed_volume(section_geometry(&g, &p).unwrap().mesh());
        let other = signed_volume(section_geometry(&g, &p.flipped()).unwrap().mesh());
        assert!((kept + other - signed_volume(g.mesh())).abs() < 1e-9);
    }

    #[test]
    fn oblique_cut_through_a_disc_is_closed() {
        let g = one_part("Some(Extrusion(axis_length: 6.0, profile: Disc(radius: 2.0)))");
        let s = section_geometry(&g, &plane([0.3, 2.9, -0.2], [0.4, 1.0, 0.2])).unwrap();
        assert_closed(s.mesh());
        assert!(signed_volume(s.mesh()) > 0.0);
    }

    #[test]
    fn planes_along_edges_of_the_stem_still_cap_and_close() {
        // x = +-1.8 and +-1.1 contain vertical edges of the sectors (3.6 and
        // 2.2 times cos 60), which gave an unusable sliver cap before the retry.
        let (_, g) = stem();
        for axis in 0..3 {
            for k in -60..=60 {
                let offset = f64::from(k) * 0.1;
                let mut n = [0.0; 3];
                n[axis] = 1.0;
                let mut point = [0.0; 3];
                point[axis] = offset;
                let cut = section_geometry(&g, &plane(point, n))
                    .unwrap_or_else(|e| panic!("axis {axis} offset {offset}: {e}"));
                assert_closed(cut.mesh());
            }
        }
    }

    #[test]
    fn plane_outside_keeps_or_removes_everything() {
        let g = annulus();
        let all = section_geometry(&g, &plane([0.0, 50.0, 0.0], [0.0, 1.0, 0.0])).unwrap();
        assert_eq!(all.mesh().indices().len(), g.mesh().indices().len());
        let none = section_geometry(&g, &plane([0.0, 50.0, 0.0], [0.0, -1.0, 0.0])).unwrap();
        assert!(none.mesh().indices().is_empty());
        assert!(none.ranges().is_empty());
    }

    #[test]
    fn stem_cut_across_caps_every_tissue_with_its_exact_area() {
        let (s, g) = stem();
        let p = plane([0.0, 6.0, 0.0], [0.0, 1.0, 0.0]);
        let cut = section_geometry(&g, &p).unwrap();
        assert_closed(cut.mesh());
        let area = |id: &str| {
            let r = range_of(&cut, s.part_index(id).unwrap()).unwrap();
            cap_area(&cut, &r, &p)
        };
        let step = TAU / 48.0;
        assert!((area("epidermis") - ring_area(48, 5.0, 4.7, step)).abs() < 1e-9);
        assert!((area("cortex") - ring_area(48, 4.7, 3.6, step)).abs() < 1e-9);
        assert!((area("pith") - ring_area(48, 2.2, 0.0, step)).abs() < 1e-9);
        // Sector tissues use ceil(48 w / 360) arc segments each.
        let sector = |inner: f64, outer: f64, width: f64, count: f64| {
            let segs = (48.0 * width / 360.0).ceil();
            count * ring_area(segs as usize, outer, inner, width.to_radians() / segs)
        };
        assert!((area("phloem") - sector(2.95, 3.6, 30.0, 8.0)).abs() < 1e-9);
        assert!((area("xylem") - sector(2.2, 2.85, 30.0, 8.0)).abs() < 1e-9);
        // The tissues tile the cross-section: no gap, no overlap.
        let total: f64 = cut.ranges().iter().map(|r| cap_area(&cut, r, &p)).sum();
        let disc = ring_area(48, 5.0, 0.0, step);
        assert!(((total - disc) / disc).abs() < 0.01, "{total} vs {disc}");
    }

    #[test]
    fn stem_cut_along_the_axis_caps_the_tissues_it_crosses() {
        let (s, g) = stem();
        // Angle 0 is +X, bundles sit every 45 degrees, so x = 0 crosses the
        // bundles at 90 and 270 degrees and misses every ray.
        let p = plane([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
        let cut = section_geometry(&g, &p).unwrap();
        assert_closed(cut.mesh());
        // The plane is nudged by about 1e-8 off the vertices at 90 degrees, so
        // areas agree to 1e-6, not to rounding.
        let area = |id: &str| {
            range_of(&cut, s.part_index(id).unwrap()).map_or(0.0, |r| cap_area(&cut, &r, &p))
        };
        let near = |a: f64, b: f64| assert!((a - b).abs() < 1e-6, "{a} vs {b}");
        near(area("epidermis"), 2.0 * 0.3 * 12.0);
        near(area("cortex"), 2.0 * 1.1 * 12.0);
        near(area("pith"), 2.0 * 2.2 * 12.0);
        near(area("phloem"), 2.0 * 0.65 * 12.0);
        near(area("vascular-cambium"), 2.0 * 0.1 * 12.0);
        near(area("xylem"), 2.0 * 0.65 * 12.0);
        // Rays are wholly on the kept side: present, but no cap.
        let rays = range_of(&cut, s.part_index("medullary-rays").unwrap()).unwrap();
        assert_eq!(cap_area(&cut, &rays, &p), 0.0);
    }

    #[test]
    fn cap_triangles_are_named_by_their_part() {
        let (s, g) = stem();
        let p = plane([0.0, 6.0, 0.0], [0.0, 1.0, 0.0]);
        let cut = section_geometry(&g, &p).unwrap();
        let v = cut.mesh().vertices();
        let xylem = s.part_index("xylem").unwrap();
        let mut found = 0;
        for (i, t) in cut.mesh().indices().chunks_exact(3).enumerate() {
            let on_plane = t.iter().all(|&k| (v[k as usize].to_array()[1] - 6.0).abs() < 1e-6);
            let c = t.iter().map(|&k| v[k as usize].to_array()).fold([0.0; 3], |a, b| [a[0] + b[0], 0.0, a[2] + b[2]]);
            let radius = (c[0] * c[0] + c[2] * c[2]).sqrt() / 3.0;
            if on_plane && (2.3..2.8).contains(&radius) {
                // Mid-radius of xylem (2.2..2.85) and no other tissue.
                if cut.part_of_triangle(i) == Some(xylem) {
                    found += 1;
                }
            }
        }
        assert!(found > 0);
    }

    #[test]
    fn open_parts_are_reported_not_capped() {
        let mesh = TriangleMesh3::new(
            vec![WorldPoint3::new(0.0, -1.0, 0.0), WorldPoint3::new(1.0, 1.0, 0.0), WorldPoint3::new(-1.0, 1.0, 0.0)],
            vec![0, 1, 2],
        )
        .unwrap();
        let g = StructureGeometry::from_parts(
            mesh,
            vec![[0.5; 3]; 3],
            vec![PartRange { part_index: 7, first_triangle: 0, triangle_count: 1 }],
        );
        let result = section_geometry(&g, &plane([0.0, 0.0, 0.0], [0.0, 1.0, 0.0]));
        assert_eq!(result.unwrap_err(), SectionError::NotClosed { part: 7 });
    }

    fn ring_of(points: &[P2], first: u32) -> Vec<(u32, P2)> {
        points.iter().enumerate().map(|(i, p)| (first + i as u32, *p)).collect()
    }

    #[test]
    fn triangulation_handles_holes_islands_and_collinear_points() {
        // 10x10 square with extra collinear points and two square holes.
        let outer = ring_of(&[[0.0, 0.0], [5.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]], 0);
        let hole1 = ring_of(&[[2.0, 2.0], [2.0, 4.0], [4.0, 4.0], [4.0, 2.0]], 10);
        let hole2 = ring_of(&[[6.0, 6.0], [6.0, 8.0], [8.0, 8.0], [8.0, 6.0]], 20);
        let t = cap_triangles(&[outer.clone(), hole1.clone(), hole2.clone()]).unwrap();
        let coords: HashMap<u32, P2> = outer.iter().chain(&hole1).chain(&hole2).map(|&(i, p)| (i, p)).collect();
        let area: f64 = t.iter().map(|t| cross2(coords[&t[0]], coords[&t[1]], coords[&t[2]]) / 2.0).sum();
        assert!((area - 92.0).abs() < 1e-9, "{area}");
        // Island inside a hole: ring 5x5 hole, 1x1 island in its middle.
        let big = ring_of(&[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]], 0);
        let hole = ring_of(&[[2.0, 2.0], [2.0, 8.0], [8.0, 8.0], [8.0, 2.0]], 10);
        let island = ring_of(&[[4.0, 4.0], [6.0, 4.0], [6.0, 6.0], [4.0, 6.0]], 20);
        let t = cap_triangles(&[big.clone(), hole.clone(), island.clone()]).unwrap();
        let coords: HashMap<u32, P2> = big.iter().chain(&hole).chain(&island).map(|&(i, p)| (i, p)).collect();
        let area: f64 = t.iter().map(|t| cross2(coords[&t[0]], coords[&t[1]], coords[&t[2]]) / 2.0).sum();
        assert!((area - 68.0).abs() < 1e-9, "{area}");
        let _ = (hole1, hole2);
    }

    #[test]
    fn triangulation_area_matches_for_a_square_with_a_hole() {
        let outer = ring_of(&[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]], 0);
        let hole = ring_of(&[[4.0, 4.0], [4.0, 6.0], [6.0, 6.0], [6.0, 4.0]], 10);
        let t = cap_triangles(&[outer.clone(), hole.clone()]).unwrap();
        let coords: HashMap<u32, P2> = outer.iter().chain(&hole).map(|&(i, p)| (i, p)).collect();
        let area: f64 = t.iter().map(|t| cross2(coords[&t[0]], coords[&t[1]], coords[&t[2]]) / 2.0).sum();
        assert!((area - 96.0).abs() < 1e-9, "{area}");
        assert!(t.iter().all(|t| cross2(coords[&t[0]], coords[&t[1]], coords[&t[2]]) >= -1e-12));
    }
}


