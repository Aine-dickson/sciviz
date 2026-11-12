//! Recipe blocks for structures beyond extrusions: `Tube`, `Ellipsoid`, and
//! part copies (`repeat`, `mirror`). Design: `docs/cockroach-structure.md`.
//!
//! Every body is a closed triangle mesh with outward winding. A hollow body
//! (`wall`) is its outer surface plus an inward-wound inner surface.

use std::f64::consts::{PI, TAU};

use serde::Deserialize;

use crate::{
    spatial::{DMat3, DQuat, DVec3},
    structure::MeshBuilder,
};

pub(crate) const MAX_COPIES: usize = 1000;
const MAX_STEPS_PER_SPAN: usize = 256;
const EPS: f64 = 1e-12;

/// Why a block could not be validated or built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BlockError {
    Invalid(&'static str),
    TooTightlyBent,
}

/// Repetition of a part's body.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Repeat {
    /// `count` copies, each `step` further along than the last.
    Linear { count: usize, step: [f64; 3] },
    /// `count` copies spread evenly round an axis through `axis_point`.
    Radial { count: usize, axis_point: [f64; 3], axis_dir: [f64; 3] },
}

/// The plane through the origin, normal to this axis, that a body is mirrored in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum MirrorAxis {
    X,
    Y,
    Z,
}

fn finite3(v: [f64; 3]) -> bool {
    v.iter().all(|c| c.is_finite())
}

fn vec3(v: [f64; 3]) -> DVec3 {
    DVec3::new(v[0], v[1], v[2])
}

/// Checks `repeat` and `mirror` fields.
pub(crate) fn check_copies(repeat: Option<&Repeat>) -> Result<(), &'static str> {
    match repeat {
        None => Ok(()),
        Some(Repeat::Linear { count, step }) => {
            if *count == 0 || *count > MAX_COPIES {
                return Err("repeat count must be between 1 and 1000");
            }
            if !finite3(*step) {
                return Err("repeat step must be finite");
            }
            if *count > 1 && vec3(*step).length() <= EPS {
                return Err("repeat step must be non-zero or the copies coincide");
            }
            Ok(())
        }
        Some(Repeat::Radial { count, axis_point, axis_dir }) => {
            if *count == 0 || *count > MAX_COPIES {
                return Err("repeat count must be between 1 and 1000");
            }
            if !finite3(*axis_point) || !finite3(*axis_dir) {
                return Err("repeat axis must be finite");
            }
            if vec3(*axis_dir).length() <= EPS {
                return Err("repeat axis_dir must be non-zero");
            }
            Ok(())
        }
    }
}

/// Applies `repeat`, then `mirror`, to a body given as raw triangles. Copies
/// are appended as separate bodies (never welded to each other). A mirrored
/// copy has its winding reversed so it stays outward-wound.
pub(crate) fn expand_copies(
    positions: Vec<[f64; 3]>,
    indices: Vec<u32>,
    repeat: Option<&Repeat>,
    mirror: Option<MirrorAxis>,
) -> (Vec<[f64; 3]>, Vec<u32>) {
    if repeat.is_none() && mirror.is_none() {
        return (positions, indices);
    }
    let copies: Vec<Vec<[f64; 3]>> = match repeat {
        None => vec![positions],
        Some(Repeat::Linear { count, step }) => (0..*count)
            .map(|c| {
                let offset = vec3(*step) * c as f64;
                positions
                    .iter()
                    .map(|p| (vec3(*p) + offset).to_array())
                    .collect()
            })
            .collect(),
        Some(Repeat::Radial { count, axis_point, axis_dir }) => {
            let origin = vec3(*axis_point);
            let axis = vec3(*axis_dir).normalize();
            (0..*count)
                .map(|c| {
                    let rotation = DQuat::from_axis_angle(axis, TAU * c as f64 / *count as f64);
                    positions
                        .iter()
                        .map(|p| (origin + rotation * (vec3(*p) - origin)).to_array())
                        .collect()
                })
                .collect()
        }
    };

    let mut out_positions: Vec<[f64; 3]> = Vec::new();
    let mut out_indices: Vec<u32> = Vec::new();
    let mut push = |points: Vec<[f64; 3]>, reverse: bool| {
        let offset = out_positions.len() as u32;
        out_positions.extend(points);
        for triangle in indices.chunks_exact(3) {
            let (a, b, c) = (triangle[0] + offset, triangle[1] + offset, triangle[2] + offset);
            if reverse {
                out_indices.extend([a, c, b]);
            } else {
                out_indices.extend([a, b, c]);
            }
        }
    };
    for copy in &copies {
        push(copy.clone(), false);
    }
    if let Some(axis) = mirror {
        let component = match axis {
            MirrorAxis::X => 0,
            MirrorAxis::Y => 1,
            MirrorAxis::Z => 2,
        };
        for copy in copies {
            let reflected = copy
                .into_iter()
                .map(|mut p| {
                    p[component] = -p[component];
                    p
                })
                .collect();
            push(reflected, true);
        }
    }
    (out_positions, out_indices)
}

// ---------------------------------------------------------------- Ellipsoid

/// Parameters of an ellipsoid block.
pub(crate) struct EllipsoidParams {
    pub centre: [f64; 3],
    pub radii: [f64; 3],
    pub rotate_deg: [f64; 3],
    pub wall: Option<f64>,
}

pub(crate) fn check_ellipsoid(p: &EllipsoidParams) -> Result<(), BlockError> {
    if !finite3(p.centre) {
        return Err(BlockError::Invalid("centre must be finite"));
    }
    if !finite3(p.rotate_deg) {
        return Err(BlockError::Invalid("rotate_deg must be finite"));
    }
    if !p.radii.iter().all(|r| r.is_finite() && *r > 0.0) {
        return Err(BlockError::Invalid("radii must be finite and positive"));
    }
    if let Some(wall) = p.wall {
        let smallest = p.radii.iter().copied().fold(f64::INFINITY, f64::min);
        if !(wall.is_finite() && wall > 0.0 && wall < smallest) {
            return Err(BlockError::Invalid("wall must be positive and less than the smallest radius"));
        }
    }
    Ok(())
}

/// Latitude/longitude ellipsoid about local +Y, rotated about the centre by
/// `rotate_deg` (about X, then Y, then Z, all world axes). Optional inner
/// surface wound inward.
pub(crate) fn build_ellipsoid(
    builder: &mut MeshBuilder,
    p: &EllipsoidParams,
    segments: usize,
) -> Result<(), BlockError> {
    check_ellipsoid(p)?;
    let rotation = DMat3::from_rotation_z(p.rotate_deg[2].to_radians())
        * DMat3::from_rotation_y(p.rotate_deg[1].to_radians())
        * DMat3::from_rotation_x(p.rotate_deg[0].to_radians());
    ellipsoid_surface(builder, p.centre, p.radii, rotation, segments, false);
    if let Some(wall) = p.wall {
        let inner = p.radii.map(|r| r - wall);
        ellipsoid_surface(builder, p.centre, inner, rotation, segments, true);
    }
    Ok(())
}

fn ellipsoid_surface(
    builder: &mut MeshBuilder,
    centre: [f64; 3],
    radii: [f64; 3],
    rotation: DMat3,
    segments: usize,
    inward: bool,
) {
    let lon = segments;
    let lat = (segments / 2).max(4);
    let mut place = |x: f64, y: f64, z: f64| -> u32 {
        let local = DVec3::new(radii[0] * x, radii[1] * y, radii[2] * z);
        builder.vertex((vec3(centre) + rotation * local).to_array())
    };
    let north = place(0.0, 1.0, 0.0);
    let mut rings: Vec<Vec<u32>> = Vec::with_capacity(lat - 1);
    for i in 1..lat {
        let theta = PI * i as f64 / lat as f64;
        let ring = (0..lon)
            .map(|j| {
                let phi = TAU * j as f64 / lon as f64;
                place(theta.sin() * phi.cos(), theta.cos(), theta.sin() * phi.sin())
            })
            .collect();
        rings.push(ring);
    }
    let south = place(0.0, -1.0, 0.0);

    let tri = |builder: &mut MeshBuilder, a: u32, b: u32, c: u32| {
        if inward {
            builder.tri(a, c, b);
        } else {
            builder.tri(a, b, c);
        }
    };
    let last = rings.len() - 1;
    for j in 0..lon {
        let k = (j + 1) % lon;
        tri(builder, north, rings[0][k], rings[0][j]);
        tri(builder, south, rings[last][j], rings[last][k]);
    }
    for i in 0..last {
        for j in 0..lon {
            let k = (j + 1) % lon;
            tri(builder, rings[i][j], rings[i + 1][k], rings[i + 1][j]);
            tri(builder, rings[i][j], rings[i][k], rings[i + 1][k]);
        }
    }
}

// --------------------------------------------------------------------- Tube

/// Parameters of a tube block.
pub(crate) struct TubeParams<'a> {
    pub path: &'a [[f64; 3]],
    pub half_widths: &'a [(f64, f64)],
    pub up: [f64; 3],
    pub steps_per_span: usize,
    pub wall: Option<f64>,
    pub arc_deg: Option<(f64, f64)>,
}

/// One sample along the tube: centre, unit tangent, frame, semi-axes.
struct Sample {
    centre: DVec3,
    tangent: DVec3,
    up: DVec3,
    side: DVec3,
    a: f64,
    b: f64,
}

fn basic_checks(p: &TubeParams<'_>) -> Result<(), BlockError> {
    let invalid = |reason| Err(BlockError::Invalid(reason));
    if p.path.len() < 2 {
        return invalid("path needs at least two points");
    }
    if p.half_widths.len() != p.path.len() {
        return invalid("half_widths must have one entry per path point");
    }
    if !p.path.iter().all(|c| finite3(*c)) {
        return invalid("path points must be finite");
    }
    if p.path.windows(2).any(|w| (vec3(w[1]) - vec3(w[0])).length() <= EPS) {
        return invalid("consecutive path points must differ");
    }
    if !p.half_widths.iter().all(|(a, b)| a.is_finite() && b.is_finite() && *a > 0.0 && *b > 0.0) {
        return invalid("half_widths must be finite and positive");
    }
    if !finite3(p.up) || vec3(p.up).length() <= EPS {
        return invalid("up must be finite and non-zero");
    }
    if p.steps_per_span == 0 || p.steps_per_span > MAX_STEPS_PER_SPAN {
        return invalid("steps_per_span must be between 1 and 256");
    }
    let smallest = p
        .half_widths
        .iter()
        .flat_map(|(a, b)| [*a, *b])
        .fold(f64::INFINITY, f64::min);
    if let Some(wall) = p.wall {
        if !(wall.is_finite() && wall > 0.0 && wall < smallest) {
            return invalid("wall must be positive and less than the smallest half width");
        }
    }
    if let Some((start, end)) = p.arc_deg {
        if p.wall.is_none() {
            return invalid("arc_deg needs a wall (a trough is a hollow shell)");
        }
        if !(start.is_finite() && end.is_finite() && end > start && end - start < 360.0) {
            return invalid("arc_deg must be finite with start < end and a width below 360");
        }
    }
    Ok(())
}

/// Catmull-Rom position and derivative on the span `k` (between points `k`
/// and `k + 1`) at `t`. Ends are extended by reflection.
fn curve(path: &[[f64; 3]], k: usize, t: f64) -> (DVec3, DVec3) {
    let point = |i: isize| -> DVec3 {
        let n = path.len() as isize;
        if i < 0 {
            2.0 * vec3(path[0]) - vec3(path[1])
        } else if i >= n {
            2.0 * vec3(path[path.len() - 1]) - vec3(path[path.len() - 2])
        } else {
            vec3(path[i as usize])
        }
    };
    let k = k as isize;
    let (p0, p1, p2, p3) = (point(k - 1), point(k), point(k + 1), point(k + 2));
    let c1 = p2 - p0;
    let c2 = 2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3;
    let c3 = -p0 + 3.0 * p1 - 3.0 * p2 + p3;
    let position = 0.5 * (2.0 * p1 + c1 * t + c2 * (t * t) + c3 * (t * t * t));
    let derivative = 0.5 * (c1 + 2.0 * c2 * t + 3.0 * c3 * (t * t));
    (position, derivative)
}

/// Samples the centre curve with a parallel-transported frame, and checks
/// that the tube neither turns back on itself nor bends too tightly.
fn sample(p: &TubeParams<'_>) -> Result<Vec<Sample>, BlockError> {
    basic_checks(p)?;
    let spans = p.path.len() - 1;
    let steps = p.steps_per_span;
    let mut samples: Vec<Sample> = Vec::with_capacity(spans * steps + 1);
    let mut previous_up = vec3(p.up);
    for index in 0..=spans * steps {
        let (span, step) = if index == spans * steps { (spans - 1, steps) } else { (index / steps, index % steps) };
        let t = step as f64 / steps as f64;
        let (centre, derivative) = curve(p.path, span, t);
        if derivative.length() <= EPS {
            return Err(BlockError::Invalid("path stops moving at a control point"));
        }
        let tangent = derivative.normalize();
        let flat = previous_up - tangent * previous_up.dot(tangent);
        if flat.length() <= 1e-6 * previous_up.length() {
            return Err(BlockError::Invalid(
                "up is parallel to the path direction (or the path turns back on itself)",
            ));
        }
        let up = flat.normalize();
        previous_up = up;
        let side = up.cross(tangent);
        let (a0, b0) = p.half_widths[span];
        let (a1, b1) = p.half_widths[span + 1];
        samples.push(Sample {
            centre,
            tangent,
            up,
            side,
            a: a0 + (a1 - a0) * t,
            b: b0 + (b1 - b0) * t,
        });
    }
    for pair in samples.windows(2) {
        let distance = (pair[1].centre - pair[0].centre).length();
        let angle = pair[0].tangent.dot(pair[1].tangent).clamp(-1.0, 1.0).acos();
        let widest = pair[0].a.max(pair[0].b).max(pair[1].a).max(pair[1].b);
        if distance <= EPS || angle / distance * widest >= 1.0 {
            return Err(BlockError::TooTightlyBent);
        }
    }
    Ok(samples)
}

pub(crate) fn check_tube(p: &TubeParams<'_>) -> Result<(), BlockError> {
    sample(p).map(|_| ())
}

/// A tube along a Catmull-Rom curve with elliptical sections, flat end caps,
/// optional wall (hollow) and optional lengthwise arc (a trough).
pub(crate) fn build_tube(
    builder: &mut MeshBuilder,
    p: &TubeParams<'_>,
    segments: usize,
) -> Result<(), BlockError> {
    let samples = sample(p)?;
    let closed = p.arc_deg.is_none();
    // Angles are counter-clockwise about the direction of travel, from `up`.
    let (angles, cells): (Vec<f64>, usize) = match p.arc_deg {
        None => ((0..segments).map(|k| TAU * k as f64 / segments as f64).collect(), segments),
        Some((start, end)) => {
            let n = ((segments as f64 * (end - start) / 360.0).ceil() as usize).max(2);
            let (s, e) = (start.to_radians(), end.to_radians());
            ((0..=n).map(|j| s + (e - s) * j as f64 / n as f64).collect(), n)
        }
    };
    let ring = |builder: &mut MeshBuilder, s: &Sample, shrink: f64| -> Vec<u32> {
        let (a, b) = (s.a - shrink, s.b - shrink);
        angles
            .iter()
            .map(|psi| builder.vertex((s.centre + s.up * (b * psi.cos()) - s.side * (a * psi.sin())).to_array()))
            .collect()
    };
    let outer: Vec<Vec<u32>> = samples.iter().map(|s| ring(builder, s, 0.0)).collect();
    let inner: Option<Vec<Vec<u32>>> =
        p.wall.map(|wall| samples.iter().map(|s| ring(builder, s, wall)).collect());
    let next = |k: usize| if closed { (k + 1) % cells } else { k + 1 };
    let last = samples.len() - 1;

    for s in 0..last {
        for k in 0..cells {
            let n = next(k);
            builder.quad(outer[s][k], outer[s][n], outer[s + 1][n], outer[s + 1][k]);
            if let Some(inner) = &inner {
                builder.quad(inner[s][k], inner[s + 1][k], inner[s + 1][n], inner[s][n]);
            }
        }
    }

    match &inner {
        Some(inner) => {
            for k in 0..cells {
                let n = next(k);
                // End cap faces +tangent; start cap faces -tangent.
                builder.quad(outer[last][k], outer[last][n], inner[last][n], inner[last][k]);
                builder.quad(outer[0][k], inner[0][k], inner[0][n], outer[0][n]);
            }
            if !closed {
                for s in 0..last {
                    // Lip at the end angle faces +psi; lip at the start angle faces -psi.
                    builder.quad(outer[s][cells], inner[s][cells], inner[s + 1][cells], outer[s + 1][cells]);
                    builder.quad(outer[s][0], outer[s + 1][0], inner[s + 1][0], inner[s][0]);
                }
            }
        }
        None => {
            let end_centre = builder.vertex(samples[last].centre.to_array());
            let start_centre = builder.vertex(samples[0].centre.to_array());
            for k in 0..cells {
                let n = next(k);
                builder.tri(end_centre, outer[last][k], outer[last][n]);
                builder.tri(start_centre, outer[0][n], outer[0][k]);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::Instant;

    use super::*;
    use crate::{
        geometry::TriangleMesh3,
        spatial::{WorldPoint3, WorldVector3},
        structure::{Structure, StructureError, StructureGeometry},
        structure_section::{section_geometry, SectionPlane},
    };

    const SEGMENTS: usize = 48;

    /// A structure of parts `(id, geometry, extras)`; `extras` is text such as
    /// `, mirror: Some(Z)` appended inside `Part(...)`.
    fn structure(parts: &[(&str, &str, &str)]) -> Result<Structure, StructureError> {
        let body: String = parts
            .iter()
            .map(|(id, geometry, extras)| {
                format!(
                    "Part(id: \"{id}\", name: \"{id}\", parent: None, description: \"d\", \
                     colour: (0.5, 0.5, 0.5), geometry: Some({geometry}){extras}),"
                )
            })
            .collect();
        Structure::from_ron_str(&format!(
            "Structure(version: 1, name: \"t\", units: \"mm\", reference: \"r\", \
             circle_segments: {SEGMENTS}, parts: [{body}])"
        ))
    }

    fn geometry(geometry: &str, extras: &str) -> StructureGeometry {
        structure(&[("p", geometry, extras)]).unwrap().build_geometry().unwrap()
    }

    fn signed_volume(mesh: &TriangleMesh3) -> f64 {
        let v = mesh.vertices();
        mesh.indices()
            .chunks_exact(3)
            .map(|t| {
                let a = v[t[0] as usize].to_array();
                let b = v[t[1] as usize].to_array();
                let c = v[t[2] as usize].to_array();
                (a[0] * (b[1] * c[2] - b[2] * c[1]) + a[1] * (b[2] * c[0] - b[0] * c[2])
                    + a[2] * (b[0] * c[1] - b[1] * c[0]))
                    / 6.0
            })
            .sum()
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

    /// Area of a regular `n`-gon with semi-axes `a` and `b`.
    fn polygon_area(n: usize, a: f64, b: f64) -> f64 {
        0.5 * n as f64 * a * b * (TAU / n as f64).sin()
    }

    fn straight(wall: &str, half: &str, extra: &str) -> String {
        format!(
            "Tube(path: [(0.0, 0.0, 0.0), (3.0, 0.0, 0.0), (10.0, 0.0, 0.0)], \
             half_widths: [{half}, {half}, {half}], wall: {wall}{extra})"
        )
    }

    #[test]
    fn straight_solid_tube_is_closed_outward_with_prism_volume() {
        let g = geometry(&straight("None", "(1.0, 1.0)", ""), "");
        assert_closed(g.mesh());
        let expected = polygon_area(SEGMENTS, 1.0, 1.0) * 10.0;
        assert!((signed_volume(g.mesh()) - expected).abs() < 1e-9);
    }

    #[test]
    fn elliptical_tube_uses_side_and_up_semi_axes() {
        let g = geometry(&straight("None", "(2.0, 0.5)", ""), "");
        assert_closed(g.mesh());
        let expected = polygon_area(SEGMENTS, 2.0, 0.5) * 10.0;
        assert!((signed_volume(g.mesh()) - expected).abs() < 1e-9);
        // Default up is +Y, so the section is 0.5 high and 2 wide (along Z).
        let (mut ymax, mut zmax) = (0.0_f64, 0.0_f64);
        for p in g.mesh().vertices() {
            ymax = ymax.max(p.to_array()[1].abs());
            zmax = zmax.max(p.to_array()[2].abs());
        }
        assert!((ymax - 0.5).abs() < 1e-9 && (zmax - 2.0).abs() < 1e-9, "y {ymax} z {zmax}");
    }

    #[test]
    fn hollow_tube_has_a_wall_volume() {
        let g = geometry(&straight("Some(0.25)", "(1.0, 1.0)", ""), "");
        assert_closed(g.mesh());
        let expected = (polygon_area(SEGMENTS, 1.0, 1.0) - polygon_area(SEGMENTS, 0.75, 0.75)) * 10.0;
        assert!((signed_volume(g.mesh()) - expected).abs() < 1e-9);
    }

    #[test]
    fn arc_trough_is_closed_and_covers_only_its_angles() {
        let g = geometry(&straight("Some(0.25)", "(1.0, 1.0)", ", arc_deg: Some((-90.0, 90.0))"), "");
        assert_closed(g.mesh());
        // 48 * 180 / 360 = 24 cells, each 7.5 degrees.
        let step = 7.5_f64.to_radians();
        let expected = 24.0 * 0.5 * (1.0 - 0.75 * 0.75) * step.sin() * 10.0;
        assert!((signed_volume(g.mesh()) - expected).abs() < 1e-9);
        // Angles are measured from +Y, so the trough is the upper half.
        let lowest = g.mesh().vertices().iter().map(|p| p.to_array()[1]).fold(f64::INFINITY, f64::min);
        assert!(lowest > -1e-9, "lowest y {lowest}");
    }

    #[test]
    fn bent_tube_is_closed_and_close_to_the_swept_volume() {
        // A quarter circle of radius 5 in the XZ plane, tube radius 0.5.
        let points: Vec<String> = (0..=6)
            .map(|i| {
                let t = std::f64::consts::FRAC_PI_2 * f64::from(i) / 6.0;
                format!("({}, 0.0, {})", 5.0 * t.cos(), 5.0 * t.sin())
            })
            .collect();
        let widths = vec!["(0.5, 0.5)"; 7].join(", ");
        let g = geometry(&format!("Tube(path: [{}], half_widths: [{widths}])", points.join(", ")), "");
        assert_closed(g.mesh());
        let expected = polygon_area(SEGMENTS, 0.5, 0.5) * 5.0 * std::f64::consts::FRAC_PI_2;
        let volume = signed_volume(g.mesh());
        assert!(((volume - expected) / expected).abs() < 0.03, "{volume} vs {expected}");
    }

    #[test]
    fn tapering_tube_is_closed_and_positive() {
        let g = geometry(
            "Tube(path: [(0.0, 0.0, 0.0), (4.0, 0.5, 0.0), (8.0, 0.0, 0.5)], \
             half_widths: [(1.0, 1.0), (0.6, 0.6), (0.1, 0.1)])",
            "",
        );
        assert_closed(g.mesh());
        assert!(signed_volume(g.mesh()) > 0.0);
    }

    #[test]
    fn a_tight_bend_is_rejected_by_name() {
        let text = "Tube(path: [(0.0, 0.0, 0.0), (2.0, 0.0, 0.0), (2.0, 0.3, 0.0), (0.0, 0.5, 0.0)], \
                    half_widths: [(0.5, 0.5), (0.5, 0.5), (0.5, 0.5), (0.5, 0.5)])";
        assert_eq!(structure(&[("p", text, "")]).unwrap_err(), StructureError::TubeTooTightlyBent("p".into()));
    }

    fn block_reason(geometry: &str) -> &'static str {
        match structure(&[("p", geometry, "")]).unwrap_err() {
            StructureError::InvalidBlock { reason, .. } => reason,
            other => panic!("expected InvalidBlock for {geometry}, got {other:?}"),
        }
    }

    #[test]
    fn tube_validation_rejects_each_error_class() {
        let two = "path: [(0.0, 0.0, 0.0), (5.0, 0.0, 0.0)]";
        let w = "half_widths: [(1.0, 1.0), (1.0, 1.0)]";
        assert!(block_reason("Tube(path: [(0.0, 0.0, 0.0)], half_widths: [(1.0, 1.0)])").contains("two points"));
        assert!(block_reason(&format!("Tube({two}, half_widths: [(1.0, 1.0)])")).contains("one entry per path point"));
        assert!(block_reason(&format!("Tube({two}, half_widths: [(1.0, 0.0), (1.0, 1.0)])")).contains("positive"));
        assert!(block_reason("Tube(path: [(0.0, 0.0, 0.0), (0.0, 0.0, 0.0)], half_widths: [(1.0, 1.0), (1.0, 1.0)])")
            .contains("differ"));
        assert!(block_reason(&format!("Tube({two}, {w}, up: (1.0, 0.0, 0.0))")).contains("parallel"));
        assert!(block_reason(&format!("Tube({two}, {w}, up: (0.0, 0.0, 0.0))")).contains("non-zero"));
        assert!(block_reason(&format!("Tube({two}, {w}, wall: Some(1.0))")).contains("wall"));
        assert!(block_reason(&format!("Tube({two}, {w}, steps_per_span: 0)")).contains("steps_per_span"));
        assert!(block_reason(&format!("Tube({two}, {w}, arc_deg: Some((0.0, 90.0)))")).contains("needs a wall"));
        assert!(block_reason(&format!("Tube({two}, {w}, wall: Some(0.2), arc_deg: Some((0.0, 360.0)))"))
            .contains("below 360"));
        assert!(block_reason(&format!("Tube({two}, {w}, wall: Some(0.2), arc_deg: Some((90.0, 0.0)))"))
            .contains("start < end"));
    }

    fn ellipsoid(radii: &str, extra: &str) -> String {
        format!("Ellipsoid(centre: (1.0, 2.0, 3.0), radii: {radii}{extra})")
    }

    #[test]
    fn ellipsoid_is_closed_outward_and_near_the_analytic_volume() {
        let g = geometry(&ellipsoid("(3.0, 2.0, 1.0)", ""), "");
        assert_closed(g.mesh());
        let analytic = 4.0 / 3.0 * PI * 6.0;
        let volume = signed_volume(g.mesh());
        assert!(((volume - analytic) / analytic).abs() < 0.02, "{volume} vs {analytic}");
    }

    #[test]
    fn rotation_keeps_volume_and_moves_the_long_axis() {
        let plain = geometry(&ellipsoid("(3.0, 1.0, 1.0)", ""), "");
        let turned = geometry(&ellipsoid("(3.0, 1.0, 1.0)", ", rotate_deg: (0.0, 0.0, 90.0)"), "");
        assert_closed(turned.mesh());
        assert!((signed_volume(plain.mesh()) - signed_volume(turned.mesh())).abs() < 1e-9);
        let extent = |g: &StructureGeometry, axis: usize| {
            let values: Vec<f64> = g.mesh().vertices().iter().map(|p| p.to_array()[axis]).collect();
            values.iter().copied().fold(f64::MIN, f64::max) - values.iter().copied().fold(f64::MAX, f64::min)
        };
        assert!((extent(&plain, 0) - 6.0).abs() < 1e-9);
        assert!((extent(&turned, 1) - 6.0).abs() < 1e-9, "rotating 90 degrees about Z puts the long axis on Y");
    }

    #[test]
    fn hollow_ellipsoid_is_outer_minus_inner() {
        let shell = geometry(&ellipsoid("(3.0, 2.0, 2.0)", ", wall: Some(0.3)"), "");
        let outer = geometry(&ellipsoid("(3.0, 2.0, 2.0)", ""), "");
        let inner = geometry(&ellipsoid("(2.7, 1.7, 1.7)", ""), "");
        assert_closed(shell.mesh());
        let expected = signed_volume(outer.mesh()) - signed_volume(inner.mesh());
        assert!((signed_volume(shell.mesh()) - expected).abs() < 1e-9);
    }

    #[test]
    fn ellipsoid_validation() {
        assert!(block_reason(&ellipsoid("(1.0, 0.0, 1.0)", "")).contains("radii"));
        assert!(block_reason(&ellipsoid("(1.0, 1.0, 1.0)", ", wall: Some(1.0)")).contains("wall"));
        assert!(block_reason("Ellipsoid(centre: (inf, 0.0, 0.0), radii: (1.0, 1.0, 1.0))").contains("centre"));
    }

    #[test]
    fn mirror_reverses_winding_and_doubles_the_body() {
        let one = geometry(&ellipsoid("(1.0, 1.0, 1.0)", ""), "");
        let both = geometry(&ellipsoid("(1.0, 1.0, 1.0)", ""), ", mirror: Some(Z)");
        assert_closed(both.mesh());
        assert!((signed_volume(both.mesh()) - 2.0 * signed_volume(one.mesh())).abs() < 1e-9);
        assert_eq!(both.ranges().len(), 1, "both copies belong to one part");
        let zs: Vec<f64> = both.mesh().vertices().iter().map(|p| p.to_array()[2]).collect();
        assert!(zs.iter().any(|z| *z > 3.5) && zs.iter().any(|z| *z < -3.5));
    }

    #[test]
    fn linear_and_radial_repeats_multiply_the_volume() {
        let base = signed_volume(geometry(&ellipsoid("(1.0, 1.0, 1.0)", ""), "").mesh());
        let linear = geometry(
            &ellipsoid("(1.0, 1.0, 1.0)", ""),
            ", repeat: Some(Linear(count: 4, step: (5.0, 0.0, 0.0)))",
        );
        assert_closed(linear.mesh());
        assert!((signed_volume(linear.mesh()) - 4.0 * base).abs() < 1e-9);
        let radial = geometry(
            &ellipsoid("(1.0, 1.0, 1.0)", ""),
            ", repeat: Some(Radial(count: 3, axis_point: (0.0, 0.0, 0.0), axis_dir: (0.0, 1.0, 0.0)))",
        );
        assert_closed(radial.mesh());
        assert!((signed_volume(radial.mesh()) - 3.0 * base).abs() < 1e-9);
        // Repeat then mirror: 4 x 2 bodies.
        let both = geometry(
            &ellipsoid("(1.0, 1.0, 1.0)", ""),
            ", repeat: Some(Linear(count: 4, step: (0.0, 0.0, 5.0))), mirror: Some(X)",
        );
        assert_closed(both.mesh());
        assert!((signed_volume(both.mesh()) - 8.0 * base).abs() < 1e-9);
    }

    #[test]
    fn copy_validation() {
        let copy = |extras: &str| match structure(&[("p", &ellipsoid("(1.0, 1.0, 1.0)", ""), extras)]).unwrap_err() {
            StructureError::InvalidCopy { reason, .. } => reason,
            other => panic!("expected InvalidCopy, got {other:?}"),
        };
        assert!(copy(", repeat: Some(Linear(count: 0, step: (1.0, 0.0, 0.0)))").contains("count"));
        assert!(copy(", repeat: Some(Linear(count: 2, step: (0.0, 0.0, 0.0)))").contains("coincide"));
        assert!(copy(", repeat: Some(Radial(count: 2, axis_point: (0.0, 0.0, 0.0), axis_dir: (0.0, 0.0, 0.0)))")
            .contains("axis_dir"));
        let group = "Part(id: \"g\", name: \"g\", parent: None, description: \"d\", colour: (0.5, 0.5, 0.5), \
                     geometry: None, mirror: Some(X)),";
        let text = format!(
            "Structure(version: 1, name: \"t\", units: \"mm\", reference: \"r\", circle_segments: 48, parts: [{group}])"
        );
        assert!(matches!(
            Structure::from_ron_str(&text).unwrap_err(),
            StructureError::InvalidCopy { .. }
        ));
    }

    // ------------------------------------------------------------ sections

    fn plane(point: [f64; 3], normal: [f64; 3]) -> SectionPlane {
        SectionPlane::new(
            WorldPoint3::new(point[0], point[1], point[2]),
            WorldVector3::new(normal[0], normal[1], normal[2]),
        )
        .unwrap()
    }

    /// Cuts at 0.1 steps from -4 to 4 on all three axes; every cut must succeed
    /// and be closed. Returns the slowest cut.
    fn sweep(label: &str, g: &StructureGeometry) -> std::time::Duration {
        let mut slowest = std::time::Duration::ZERO;
        for axis in 0..3 {
            for k in -40..=40 {
                let offset = f64::from(k) * 0.1;
                let (mut n, mut p) = ([0.0; 3], [0.0; 3]);
                n[axis] = 1.0;
                p[axis] = offset;
                let start = Instant::now();
                let cut = section_geometry(g, &plane(p, n))
                    .unwrap_or_else(|e| panic!("{label}: axis {axis} offset {offset:.1}: {e}"));
                slowest = slowest.max(start.elapsed());
                assert_closed(cut.mesh());
            }
        }
        slowest
    }

    #[test]
    fn bent_tube_trough_shell_and_mirror_survive_a_plane_sweep() {
        let points: Vec<String> = (0..=8)
            .map(|i| {
                let t = 1.5 * PI * f64::from(i) / 8.0;
                format!("({}, {}, 0.0)", 3.0 * t.cos(), 3.0 * t.sin())
            })
            .collect();
        let widths = vec!["(0.8, 0.8)"; 9].join(", ");
        let bent = format!("Tube(path: [{}], half_widths: [{widths}])", points.join(", "));
        let bent_g = geometry(&bent, "");
        let slowest = sweep("bent tube", &bent_g);

        let trough = geometry(&straight("Some(0.2)", "(1.0, 1.0)", ", arc_deg: Some((-100.0, 100.0))"), "");
        sweep("trough", &trough);

        // A hollow body wall with an organ inside it, and a mirrored pair.
        let wall = "Ellipsoid(centre: (0.0, 0.0, 0.0), radii: (3.0, 2.0, 2.0), wall: Some(0.3))";
        let organ = "Ellipsoid(centre: (0.3, 0.0, 0.0), radii: (1.0, 0.6, 0.6))";
        let pair = "Ellipsoid(centre: (0.0, 0.0, 1.5), radii: (0.8, 0.5, 0.5))";
        let g = structure(&[("wall", wall, ""), ("organ", organ, ""), ("pair", pair, ", mirror: Some(Z)")])
            .unwrap()
            .build_geometry()
            .unwrap();
        sweep("shell + organ + pair", &g);

        // Cap triangulation is O(n^2) in loop length; organism-scale loops
        // must stay quick. Timing is only meaningful in release builds.
        if !cfg!(debug_assertions) {
            assert!(slowest.as_millis() < 100, "slowest cut {slowest:?}");
        }
    }

    #[test]
    fn a_shell_cut_leaves_a_ring_not_a_disc() {
        let wall = "Ellipsoid(centre: (0.0, 0.0, 0.0), radii: (3.0, 2.0, 2.0), wall: Some(0.3))";
        let g = geometry(wall, "");
        let cut = section_geometry(&g, &plane([0.0, 0.0, 0.0], [0.0, 1.0, 0.0])).unwrap();
        let v = cut.mesh().vertices();
        let area: f64 = cut
            .mesh()
            .indices()
            .chunks_exact(3)
            .filter(|t| t.iter().all(|&i| v[i as usize].to_array()[1].abs() < 1e-6))
            .map(|t| {
                let (a, b, c) = (v[t[0] as usize].to_array(), v[t[1] as usize].to_array(), v[t[2] as usize].to_array());
                0.5 * ((b[0] - a[0]) * (c[2] - a[2]) - (b[2] - a[2]) * (c[0] - a[0])).abs()
            })
            .sum();
        let disc = PI * 3.0 * 2.0;
        let ring = PI * (3.0 * 2.0 - 2.7 * 1.7);
        assert!(area < 0.9 * disc && (area - ring).abs() < 0.1 * ring, "area {area}, ring {ring}, disc {disc}");
    }
}
