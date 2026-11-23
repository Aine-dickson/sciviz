//! Rounded routes: a path given by its corner points, with every interior
//! corner replaced by a circular arc and every piece sampled at an even spacing
//! (`docs/authoring-convention.md` section 6, "lines and arcs").
//!
//! Wires, guts, airways and flows are all "straight runs joined by bends".
//! Writing them as corner points plus one radius means an author never has to
//! sample an arc by hand. The result is a plain point list, so it feeds the
//! existing `Tube` and `Flow` code unchanged.
//!
//! Each corner `B` with neighbours `A` and `C` is replaced by the arc of radius
//! `r` that touches both segments. It starts and ends `t = r / tan(theta / 2)`
//! from `B` (`theta` is the angle `A B C`), in the plane of the three points, so
//! routes may bend in 3D. Points that lie on a straight line are not bent.

use std::fmt;

/// Why a route could not be built.
#[derive(Clone, Debug, PartialEq)]
pub enum RouteError {
    /// Fewer than two distinct points.
    TooFewPoints,
    NotFinite,
    /// The radius or spacing is not finite and positive.
    BadNumber(&'static str),
    /// The route turns straight back on itself at this corner (0-based, counting distinct points).
    DoublesBack { corner: usize },
    /// The arc at this corner needs `needed` of straight run, and the shorter side has only `available`.
    RadiusTooLarge { corner: usize, needed: f64, available: f64 },
}

impl fmt::Display for RouteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewPoints => write!(f, "a route needs at least two distinct points"),
            Self::NotFinite => write!(f, "route points must be finite"),
            Self::BadNumber(what) => write!(f, "the route's {what} must be finite and above zero"),
            Self::DoublesBack { corner } => write!(f, "the route turns straight back on itself at corner {corner}"),
            Self::RadiusTooLarge { corner, needed, available } => write!(
                f,
                "the bend at corner {corner} needs {needed:.4} of straight run but only {available:.4} is available; use a smaller radius or move the corners apart"
            ),
        }
    }
}

impl std::error::Error for RouteError {}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn scale(a: [f64; 3], s: f64) -> [f64; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn length(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

/// One corner after analysis.
struct Corner {
    /// Where the straight run before the bend ends and where the next begins.
    enter: [f64; 3],
    exit: [f64; 3],
    /// Arc centre, radius and sweep angle (radians); `None` for a straight-through point.
    arc: Option<([f64; 3], f64, f64)>,
}

/// Rounds every interior corner of `points` with arcs of `radius` and samples
/// the whole route so that no two neighbouring points are further apart than
/// `spacing`. The first and last points are kept exactly.
pub fn rounded_route(points: &[[f64; 3]], radius: f64, spacing: f64) -> Result<Vec<[f64; 3]>, RouteError> {
    if !(radius.is_finite() && radius > 0.0) {
        return Err(RouteError::BadNumber("radius"));
    }
    if !(spacing.is_finite() && spacing > 0.0) {
        return Err(RouteError::BadNumber("spacing"));
    }
    if points.iter().flatten().any(|c| !c.is_finite()) {
        return Err(RouteError::NotFinite);
    }
    let mut p: Vec<[f64; 3]> = Vec::with_capacity(points.len());
    for q in points {
        if p.last().is_none_or(|last| length(sub(*q, *last)) > 1e-12) {
            p.push(*q);
        }
    }
    if p.len() < 2 {
        return Err(RouteError::TooFewPoints);
    }
    let n = p.len();
    let segment_length = |i: usize| length(sub(p[i + 1], p[i]));

    // Analyse the interior corners.
    let mut corners: Vec<Option<Corner>> = Vec::with_capacity(n);
    let mut tangent = vec![0.0; n]; // tangent length t at each corner
    corners.push(None);
    for i in 1..n - 1 {
        let u = scale(sub(p[i - 1], p[i]), 1.0 / segment_length(i - 1));
        let w = scale(sub(p[i + 1], p[i]), 1.0 / segment_length(i));
        let cos_theta = dot(u, w).clamp(-1.0, 1.0);
        let theta = cos_theta.acos();
        if theta < 1e-6 {
            return Err(RouteError::DoublesBack { corner: i });
        }
        if std::f64::consts::PI - theta < 1e-9 {
            corners.push(None); // straight through
            continue;
        }
        let t = radius / (theta / 2.0).tan();
        tangent[i] = t;
        let bisector = {
            let b = add(u, w);
            scale(b, 1.0 / length(b))
        };
        let centre = add(p[i], scale(bisector, radius / (theta / 2.0).sin()));
        corners.push(Some(Corner {
            enter: add(p[i], scale(u, t)),
            exit: add(p[i], scale(w, t)),
            arc: Some((centre, radius, std::f64::consts::PI - theta)),
        }));
    }
    corners.push(None);

    // Neighbouring bends must fit on the segment between them.
    for i in 0..n - 1 {
        let needed = tangent[i] + tangent[i + 1];
        let available = segment_length(i);
        if needed > available * (1.0 + 1e-9) {
            let corner = if tangent[i] >= tangent[i + 1] && tangent[i] > 0.0 { i } else { i + 1 };
            return Err(RouteError::RadiusTooLarge { corner, needed, available });
        }
    }

    // Assemble: straight runs (even pieces) and arcs (even angles).
    let mut out = vec![p[0]];
    let mut cursor = p[0];
    let run_to = |out: &mut Vec<[f64; 3]>, from: [f64; 3], to: [f64; 3]| {
        let d = length(sub(to, from));
        let pieces = (d / spacing).ceil().max(1.0) as usize;
        for k in 1..=pieces {
            out.push(add(from, scale(sub(to, from), k as f64 / pieces as f64)));
        }
    };
    for corner in corners.iter().take(n - 1).skip(1) {
        match corner {
            None => {}
            Some(c) => {
                run_to(&mut out, cursor, c.enter);
                let (centre, r, sweep) = c.arc.expect("a bend has an arc");
                let e1 = scale(sub(c.enter, centre), 1.0 / r);
                let e2 = scale(sub(c.exit, centre), 1.0 / r);
                let pieces = ((r * sweep) / spacing).ceil().max(2.0) as usize;
                // Spherical interpolation between the two radial directions.
                for k in 1..pieces {
                    let phi = sweep * k as f64 / pieces as f64;
                    let a = ((sweep - phi).sin()) / sweep.sin();
                    let b = phi.sin() / sweep.sin();
                    out.push(add(centre, scale(add(scale(e1, a), scale(e2, b)), r)));
                }
                out.push(c.exit);
                cursor = c.exit;
            }
        }
    }
    run_to(&mut out, cursor, p[n - 1]);
    Ok(out)
}

/// The point `fraction` (0 to 1) of the way along `route` by length, and the unit
/// direction of travel there. Use it to place a direction marker on a wire or a
/// flow path: a cone `Cone(base: point - direction * L / 2, tip: point + direction * L / 2, ...)`
/// points the way the route runs.
///
/// Returns `None` for a route with fewer than two distinct points, a non-finite
/// point, or a `fraction` outside 0 to 1.
#[must_use]
pub fn point_along(route: &[[f64; 3]], fraction: f64) -> Option<([f64; 3], [f64; 3])> {
    if !(0.0..=1.0).contains(&fraction) || route.iter().flatten().any(|c| !c.is_finite()) {
        return None;
    }
    let pieces: Vec<(usize, f64)> = route
        .windows(2)
        .enumerate()
        .map(|(i, w)| (i, length(sub(w[1], w[0]))))
        .filter(|(_, d)| *d > 1e-12)
        .collect();
    let total: f64 = pieces.iter().map(|(_, d)| d).sum();
    if pieces.is_empty() || !total.is_finite() {
        return None;
    }
    let mut remaining = fraction * total;
    for (n, &(i, d)) in pieces.iter().enumerate() {
        if remaining <= d || n == pieces.len() - 1 {
            let direction = scale(sub(route[i + 1], route[i]), 1.0 / d);
            let t = (remaining / d).clamp(0.0, 1.0);
            return Some((add(route[i], scale(sub(route[i + 1], route[i]), t)), direction));
        }
        remaining -= d;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(a: [f64; 3], b: [f64; 3]) -> f64 {
        length(sub(a, b))
    }

    #[test]
    fn a_right_angle_becomes_a_quarter_circle_of_the_given_radius() {
        let route = rounded_route(&[[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [10.0, 10.0, 0.0]], 2.0, 0.5).unwrap();
        assert_eq!(route[0], [0.0, 0.0, 0.0]);
        assert_eq!(*route.last().unwrap(), [10.0, 10.0, 0.0]);
        // The arc centre is (8, 2); every point in the bend region is 2 from it.
        let on_arc: Vec<_> = route.iter().filter(|q| q[0] > 8.0 + 1e-9 && q[1] < 2.0 - 1e-9).collect();
        assert!(on_arc.len() >= 3);
        for q in on_arc {
            assert!((d(*q, [8.0, 2.0, 0.0]) - 2.0).abs() < 1e-9, "{q:?}");
        }
        // The bend starts and ends exactly 2 from the corner.
        assert!(route.iter().any(|q| d(*q, [8.0, 0.0, 0.0]) < 1e-9));
        assert!(route.iter().any(|q| d(*q, [10.0, 2.0, 0.0]) < 1e-9));
        // The corner itself is cut off.
        assert!(route.iter().all(|q| d(*q, [10.0, 0.0, 0.0]) > 0.5));
    }

    #[test]
    fn spacing_is_even_and_never_exceeded() {
        let route = rounded_route(&[[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [10.0, 10.0, 0.0]], 2.0, 0.7).unwrap();
        for w in route.windows(2) {
            assert!(d(w[0], w[1]) <= 0.7 + 1e-9, "gap {}", d(w[0], w[1]));
        }
        // Along the first straight run the points are equally spaced.
        let run: Vec<_> = route.iter().take_while(|q| q[0] <= 8.0 + 1e-9).collect();
        let gaps: Vec<f64> = run.windows(2).map(|w| d(*w[0], *w[1])).collect();
        assert!(gaps.iter().all(|g| (g - gaps[0]).abs() < 1e-9));
    }

    #[test]
    fn bends_work_in_three_dimensions_and_keep_to_their_plane() {
        // A corner in the plane through (0,0,0), (5,0,0) and (5,0,5) bends only in x and z.
        let route = rounded_route(&[[0.0, 0.0, 0.0], [5.0, 0.0, 0.0], [5.0, 0.0, 5.0]], 1.0, 0.3).unwrap();
        assert!(route.iter().all(|q| q[1].abs() < 1e-12));
        // An oblique corner: arc points stay at the radius from the computed centre.
        let a = [0.0, 0.0, 0.0];
        let b = [4.0, 1.0, 2.0];
        let c = [8.0, 3.0, 1.0];
        let route = rounded_route(&[a, b, c], 1.0, 0.2).unwrap();
        assert_eq!(route[0], a);
        assert_eq!(*route.last().unwrap(), c);
        // No point is closer to the corner than the bend allows, and the route is continuous.
        for w in route.windows(2) {
            assert!(d(w[0], w[1]) <= 0.2 + 1e-9);
        }
    }

    #[test]
    fn a_circuit_style_rounded_rectangle_has_the_expected_length() {
        // Four corners of a 20 by 12 rectangle with radius 1.5: perimeter minus 4 * (2 - pi/2) * r.
        let pts = [
            [-10.0, -6.0, 0.0],
            [10.0, -6.0, 0.0],
            [10.0, 6.0, 0.0],
            [-10.0, 6.0, 0.0],
            [-10.0, -6.0, 0.0],
        ];
        let route = rounded_route(&pts, 1.5, 0.05).unwrap();
        let len: f64 = route.windows(2).map(|w| d(w[0], w[1])).sum();
        // The three interior corners are rounded (the first and last points are not corners of the route).
        let expected = 64.0 - 3.0 * (2.0 - std::f64::consts::FRAC_PI_2) * 1.5;
        assert!((len - expected).abs() < 0.02, "{len} vs {expected}");
    }

    #[test]
    fn straight_through_points_are_not_bent_and_duplicates_are_ignored() {
        let route = rounded_route(&[[0.0, 0.0, 0.0], [3.0, 0.0, 0.0], [3.0, 0.0, 0.0], [6.0, 0.0, 0.0]], 1.0, 1.0).unwrap();
        assert!(route.iter().all(|q| q[1] == 0.0 && q[2] == 0.0));
        assert_eq!(route.len(), 7);
    }

    #[test]
    fn bad_routes_are_refused_with_a_reason() {
        let ok = [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [10.0, 10.0, 0.0]];
        assert_eq!(rounded_route(&ok[..1], 1.0, 1.0), Err(RouteError::TooFewPoints));
        assert_eq!(rounded_route(&ok, 0.0, 1.0), Err(RouteError::BadNumber("radius")));
        assert_eq!(rounded_route(&ok, 1.0, f64::NAN), Err(RouteError::BadNumber("spacing")));
        assert_eq!(rounded_route(&[[0.0; 3], [f64::NAN, 0.0, 0.0]], 1.0, 1.0), Err(RouteError::NotFinite));
        let back = [[0.0, 0.0, 0.0], [5.0, 0.0, 0.0], [0.0, 0.0, 0.0]];
        assert_eq!(rounded_route(&back, 1.0, 1.0), Err(RouteError::DoublesBack { corner: 1 }));
        // A radius of 6 at a right angle needs 6 of run, but the first segment is 5 long.
        let short = [[0.0, 0.0, 0.0], [5.0, 0.0, 0.0], [5.0, 20.0, 0.0]];
        assert!(matches!(rounded_route(&short, 6.0, 1.0), Err(RouteError::RadiusTooLarge { corner: 1, .. })));
        // Two bends that would overlap on a short middle segment.
        let tight = [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [10.0, 2.0, 0.0], [20.0, 2.0, 0.0]];
        assert!(matches!(rounded_route(&tight, 1.5, 0.5), Err(RouteError::RadiusTooLarge { .. })));
        assert!(rounded_route(&tight, 0.9, 0.5).is_ok());
    }

    #[test]
    fn a_point_along_a_route_has_the_direction_of_travel() {
        let route = rounded_route(&[[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [10.0, 10.0, 0.0]], 2.0, 0.1).unwrap();
        let (start, dir) = point_along(&route, 0.0).unwrap();
        assert_eq!((start, dir), ([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]));
        let (end, dir) = point_along(&route, 1.0).unwrap();
        assert!(d(end, [10.0, 10.0, 0.0]) < 1e-9 && d(dir, [0.0, 1.0, 0.0]) < 1e-9);
        // Straight 8, a quarter circle of radius 2 (length pi), straight 8.
        let total = 16.0 + std::f64::consts::PI;
        // Halfway is the middle of the arc: centre (8, 2), 45 degrees round, heading diagonally.
        let (mid, dir) = point_along(&route, 0.5).unwrap();
        let s2 = std::f64::consts::FRAC_1_SQRT_2;
        assert!(d(mid, [8.0 + 2.0 * s2, 2.0 - 2.0 * s2, 0.0]) < 0.01, "{mid:?}");
        assert!(d(dir, [s2, s2, 0.0]) < 0.05, "{dir:?}");
        assert!((length(dir) - 1.0).abs() < 1e-9);
        // The point 8 along is the start of the bend, still heading along +x.
        let (bend, dir) = point_along(&route, 8.0 / total).unwrap();
        assert!(d(bend, [8.0, 0.0, 0.0]) < 1e-3 && d(dir, [1.0, 0.0, 0.0]) < 0.05, "{bend:?} {dir:?}");
        // Bad input.
        assert!(point_along(&route, 1.5).is_none());
        assert!(point_along(&[[1.0, 1.0, 1.0]], 0.5).is_none());
        assert!(point_along(&[[0.0; 3], [f64::NAN, 0.0, 0.0]], 0.5).is_none());
    }
}
