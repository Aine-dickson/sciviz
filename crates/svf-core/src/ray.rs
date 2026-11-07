//! Graphics-independent ray construction and indexed triangle-mesh intersection.
use crate::{
    geometry::TriangleMesh3,
    spatial::{WorldPoint3, WorldVector3},
};
use glam::DVec3;
use std::{error::Error, fmt};

const PARALLEL_EPSILON: f64 = 32.0 * f64::EPSILON;
const BARYCENTRIC_EPSILON: f64 = 1.0e-12;

/// Errors produced when constructing a world-space ray.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RayError {
    NonFiniteOrigin,
    NonFiniteDirection,
    ZeroDirection,
    NonFiniteDirectionLength,
}
impl fmt::Display for RayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::NonFiniteOrigin => "ray origin must be finite",
            Self::NonFiniteDirection => "ray direction must be finite",
            Self::ZeroDirection => "ray direction must be non-zero",
            Self::NonFiniteDirectionLength => "ray direction length must be finite",
        })
    }
}
impl Error for RayError {}

/// A half-line in world space with a normalized direction.
///
/// Hit distance is therefore measured in world units. Rays include their
/// origin and extend infinitely in the positive direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray3 {
    origin: WorldPoint3,
    direction: WorldVector3,
}
impl Ray3 {
    pub fn new(origin: WorldPoint3, direction: WorldVector3) -> Result<Self, RayError> {
        let origin_vector = origin.to_vector();
        let direction_vector = direction.to_vector();
        if !origin_vector.is_finite() { return Err(RayError::NonFiniteOrigin); }
        if !direction_vector.is_finite() { return Err(RayError::NonFiniteDirection); }
        let length = direction_vector.length();
        if length == 0.0 { return Err(RayError::ZeroDirection); }
        if !length.is_finite() { return Err(RayError::NonFiniteDirectionLength); }
        Ok(Self {
            origin,
            direction: WorldVector3::from_vector(direction_vector / length),
        })
    }
    #[must_use] pub const fn origin(&self) -> WorldPoint3 { self.origin }
    #[must_use] pub const fn direction(&self) -> WorldVector3 { self.direction }
    #[must_use]
    pub fn point_at(&self, distance: f64) -> WorldPoint3 {
        self.origin + self.direction * distance
    }
}

/// The nearest non-negative intersection of a ray with an indexed triangle mesh.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHit3 {
    distance: f64,
    position: WorldPoint3,
    triangle_index: usize,
    barycentric: [f64; 3],
}
impl RayHit3 {
    #[must_use] pub const fn distance(&self) -> f64 { self.distance }
    #[must_use] pub const fn position(&self) -> WorldPoint3 { self.position }
    #[must_use] pub const fn triangle_index(&self) -> usize { self.triangle_index }
    /// Weights correspond to the triangle's stored index order and sum to one.
    #[must_use] pub const fn barycentric(&self) -> [f64; 3] { self.barycentric }
}

/// Finds the nearest two-sided ray/triangle intersection in a mesh.
///
/// Degenerate and near-parallel triangles are skipped. Triangle winding does
/// not affect hit detection. A hit at the ray origin is valid.
#[must_use]
pub fn intersect_mesh(ray: &Ray3, mesh: &TriangleMesh3) -> Option<RayHit3> {
    let origin = ray.origin().to_vector();
    let direction = ray.direction().to_vector();
    let vertices = mesh.vertices();
    let mut nearest: Option<RayHit3> = None;

    for (triangle_index, [ia, ib, ic]) in mesh.triangles().enumerate() {
        let a = vertices[ia as usize].to_vector();
        let b = vertices[ib as usize].to_vector();
        let c = vertices[ic as usize].to_vector();
        let edge1 = b - a;
        let edge2 = c - a;
        let p = direction.cross(edge2);
        let determinant = edge1.dot(p);
        let scale = edge1.length() * edge2.length();
        if !determinant.is_finite() || !scale.is_finite()
            || scale == 0.0 || determinant.abs() <= PARALLEL_EPSILON * scale
        {
            continue;
        }

        let inverse_determinant = determinant.recip();
        let from_a = origin - a;
        let u = from_a.dot(p) * inverse_determinant;
        if !u.is_finite() || u < -BARYCENTRIC_EPSILON || u > 1.0 + BARYCENTRIC_EPSILON {
            continue;
        }

        let q = from_a.cross(edge1);
        let v = direction.dot(q) * inverse_determinant;
        if !v.is_finite() || v < -BARYCENTRIC_EPSILON
            || u + v > 1.0 + BARYCENTRIC_EPSILON
        {
            continue;
        }

        let distance = edge2.dot(q) * inverse_determinant;
        if !distance.is_finite() || distance < 0.0
            || nearest.is_some_and(|hit| distance >= hit.distance)
        {
            continue;
        }

        let position = ray.point_at(distance);
        if !position.to_vector().is_finite() { continue; }
        let weights = [1.0 - u - v, u, v];
        nearest = Some(RayHit3 {
            distance,
            position,
            triangle_index,
            barycentric: weights,
        });
    }
    nearest
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64, z: f64) -> WorldPoint3 { WorldPoint3::new(x, y, z) }
    fn v(x: f64, y: f64, z: f64) -> WorldVector3 { WorldVector3::new(x, y, z) }
    fn ray(origin: WorldPoint3, direction: WorldVector3) -> Ray3 {
        Ray3::new(origin, direction).unwrap()
    }

    #[test]
    fn ray_normalizes_direction_and_uses_world_distance() {
        let r = ray(p(0.0, 0.0, 0.0), v(0.0, -4.0, 0.0));
        assert_eq!(r.direction(), v(0.0, -1.0, 0.0));
        assert_eq!(r.point_at(3.0), p(0.0, -3.0, 0.0));
    }

    #[test]
    fn ray_rejects_invalid_origin_and_direction() {
        assert_eq!(Ray3::new(p(f64::NAN, 0.0, 0.0), v(1.0, 0.0, 0.0)), Err(RayError::NonFiniteOrigin));
        assert_eq!(Ray3::new(p(0.0, 0.0, 0.0), v(f64::INFINITY, 0.0, 0.0)), Err(RayError::NonFiniteDirection));
        assert_eq!(Ray3::new(p(0.0, 0.0, 0.0), v(0.0, 0.0, 0.0)), Err(RayError::ZeroDirection));
        assert_eq!(Ray3::new(p(0.0, 0.0, 0.0), v(f64::MAX, f64::MAX, 0.0)), Err(RayError::NonFiniteDirectionLength));
    }

    #[test]
    fn intersects_triangle_from_both_sides_and_returns_barycentric_weights() {
        let mesh = TriangleMesh3::new(vec![p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.0, 0.0, 1.0)], vec![0, 1, 2]).unwrap();
        let hit = intersect_mesh(&ray(p(0.25, 2.0, 0.25), v(0.0, -1.0, 0.0)), &mesh).unwrap();
        assert_eq!(hit.triangle_index(), 0);
        assert!((hit.distance() - 2.0).abs() < 1.0e-12);
        assert!((hit.position().to_vector() - glam::DVec3::new(0.25, 0.0, 0.25)).length() < 1.0e-12);
        let weights = hit.barycentric();
        assert!((weights[0] - 0.5).abs() < 1.0e-12);
        assert!((weights[1] - 0.25).abs() < 1.0e-12);
        assert!((weights[2] - 0.25).abs() < 1.0e-12);

        let reverse = intersect_mesh(&ray(p(0.25, -2.0, 0.25), v(0.0, 1.0, 0.0)), &mesh).unwrap();
        assert_eq!(reverse.position(), hit.position());
    }

    #[test]
    fn returns_nearest_hit_and_ignores_triangles_behind_ray() {
        let mesh = TriangleMesh3::new(
            vec![p(0.0, 1.0, 0.0), p(1.0, 1.0, 0.0), p(0.0, 1.0, 1.0),
                 p(0.0, 3.0, 0.0), p(1.0, 3.0, 0.0), p(0.0, 3.0, 1.0)],
            vec![0, 1, 2, 3, 4, 5],
        ).unwrap();
        let hit = intersect_mesh(&ray(p(0.2, 0.0, 0.2), v(0.0, 1.0, 0.0)), &mesh).unwrap();
        assert_eq!(hit.triangle_index(), 0);
        assert_eq!(hit.distance(), 1.0);
        assert!(intersect_mesh(&ray(p(0.2, 4.0, 0.2), v(0.0, 1.0, 0.0)), &mesh).is_none());
    }

    #[test]
    fn misses_outside_parallel_and_degenerate_triangles() {
        let mesh = TriangleMesh3::new(
            vec![p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(2.0, 0.0, 0.0)],
            vec![0, 1, 2],
        ).unwrap();
        assert!(intersect_mesh(&ray(p(0.2, 1.0, 0.2), v(0.0, -1.0, 0.0)), &mesh).is_none());
        assert!(intersect_mesh(&ray(p(0.2, 1.0, 0.0), v(1.0, 0.0, 0.0)), &mesh).is_none());
    }

    #[test]
    fn accepts_a_hit_at_the_ray_origin() {
        let mesh = TriangleMesh3::new(vec![p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.0, 0.0, 1.0)], vec![0, 1, 2]).unwrap();
        let hit = intersect_mesh(&ray(p(0.25, 0.0, 0.25), v(0.0, 1.0, 0.0)), &mesh).unwrap();
        assert_eq!(hit.distance(), 0.0);
    }
}
