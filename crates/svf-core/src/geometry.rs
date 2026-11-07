//! Graphics-independent geometric primitives in world coordinates.
//!
//! These types describe scientific geometry, not GPU vertex layouts. They
//! retain f64 coordinates and can be validated and inspected without a
//! renderer. Conversion to renderer-specific formats belongs downstream.

use std::{error::Error, fmt};

use crate::spatial::WorldPoint3;

/// Errors produced when constructing scientific geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeometryError {
    /// A point contains one or more non-finite coordinates.
    NonFinitePoint { index: usize },
    /// A polyline must contain at least two points.
    PolylineRequiresTwoPoints { actual: usize },
}

impl fmt::Display for GeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinitePoint { index } => {
                write!(f, "point at index {index} contains a non-finite coordinate")
            }
            Self::PolylineRequiresTwoPoints { actual } => {
                write!(f, "a polyline requires at least two points; received {actual}")
            }
        }
    }
}

impl Error for GeometryError {}

/// A collection of independent world-space points.
///
/// An empty point set is valid. Point order is preserved, and no connectivity
/// between points is implied.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PointSet3 {
    points: Vec<WorldPoint3>,
}

impl PointSet3 {
    /// Creates a point set after validating every coordinate.
    pub fn new(points: Vec<WorldPoint3>) -> Result<Self, GeometryError> {
        validate_points(&points)?;
        Ok(Self { points })
    }

    /// Returns the points in their original order.
    #[must_use]
    pub fn points(&self) -> &[WorldPoint3] {
        &self.points
    }

    /// Returns the number of points.
    #[must_use]
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Returns whether the point set contains no points.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Consumes the point set and returns its points.
    #[must_use]
    pub fn into_points(self) -> Vec<WorldPoint3> {
        self.points
    }
}

/// A straight line segment between two world-space endpoints.
///
/// Coincident endpoints are allowed and represent a zero-length segment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineSegment3 {
    start: WorldPoint3,
    end: WorldPoint3,
}

impl LineSegment3 {
    /// Creates a segment after validating both endpoints.
    pub fn new(start: WorldPoint3, end: WorldPoint3) -> Result<Self, GeometryError> {
        validate_points(&[start, end])?;
        Ok(Self { start, end })
    }

    #[must_use]
    pub const fn start(&self) -> WorldPoint3 {
        self.start
    }

    #[must_use]
    pub const fn end(&self) -> WorldPoint3 {
        self.end
    }

    /// Returns the displacement from the start point to the end point.
    #[must_use]
    pub fn displacement(&self) -> crate::spatial::WorldVector3 {
        self.end - self.start
    }

    /// Returns the Euclidean length in world-coordinate units.
    #[must_use]
    pub fn length(&self) -> f64 {
        self.displacement().length()
    }
}

/// An ordered, open sequence of world-space points.
///
/// Consecutive points define line segments. The sequence is not implicitly
/// closed; to represent a closed path, repeat the first point at the end.
/// Repeated consecutive points are preserved and produce zero-length segments.
#[derive(Clone, Debug, PartialEq)]
pub struct Polyline3 {
    points: Vec<WorldPoint3>,
}

impl Polyline3 {
    /// Creates a polyline containing at least two finite points.
    pub fn new(points: Vec<WorldPoint3>) -> Result<Self, GeometryError> {
        if points.len() < 2 {
            return Err(GeometryError::PolylineRequiresTwoPoints {
                actual: points.len(),
            });
        }
        validate_points(&points)?;
        Ok(Self { points })
    }

    /// Returns the ordered points defining this polyline.
    #[must_use]
    pub fn points(&self) -> &[WorldPoint3] {
        &self.points
    }

    /// Returns the number of points.
    #[must_use]
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Returns the number of consecutive line segments.
    #[must_use]
    pub fn segment_count(&self) -> usize {
        self.points.len() - 1
    }

    /// Iterates over consecutive segments without copying the point storage.
    pub fn segments(&self) -> impl ExactSizeIterator<Item = LineSegment3> + '_ {
        self.points.windows(2).map(|pair| LineSegment3 {
            start: pair[0],
            end: pair[1],
        })
    }

    /// Consumes the polyline and returns its points.
    #[must_use]
    pub fn into_points(self) -> Vec<WorldPoint3> {
        self.points
    }
}

fn validate_points(points: &[WorldPoint3]) -> Result<(), GeometryError> {
    for (index, point) in points.iter().enumerate() {
        if !point.to_vector().is_finite() {
            return Err(GeometryError::NonFinitePoint { index });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(x: f64, y: f64, z: f64) -> WorldPoint3 {
        WorldPoint3::new(x, y, z)
    }

    #[test]
    fn point_set_preserves_order_and_allows_empty_data() {
        let empty = PointSet3::new(Vec::new()).unwrap();
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);

        let points = vec![point(1.0, 2.0, 3.0), point(-4.0, 5.0, -6.0)];
        let set = PointSet3::new(points.clone()).unwrap();
        assert_eq!(set.len(), 2);
        assert_eq!(set.points(), points);
        assert_eq!(set.into_points(), points);
    }

    #[test]
    fn point_set_rejects_non_finite_coordinates_with_index() {
        let result = PointSet3::new(vec![point(0.0, 0.0, 0.0), point(f64::NAN, 1.0, 2.0)]);
        assert_eq!(result, Err(GeometryError::NonFinitePoint { index: 1 }));
    }

    #[test]
    fn line_segment_preserves_endpoints_and_computes_length() {
        let segment = LineSegment3::new(point(1.0, 2.0, 3.0), point(4.0, 6.0, 3.0)).unwrap();
        assert_eq!(segment.start(), point(1.0, 2.0, 3.0));
        assert_eq!(segment.end(), point(4.0, 6.0, 3.0));
        assert_eq!(segment.displacement().to_vector().to_array(), [3.0, 4.0, 0.0]);
        assert_eq!(segment.length(), 5.0);
    }

    #[test]
    fn line_segment_allows_coincident_endpoints() {
        let segment = LineSegment3::new(point(2.0, 3.0, 4.0), point(2.0, 3.0, 4.0)).unwrap();
        assert_eq!(segment.length(), 0.0);
    }

    #[test]
    fn line_segment_rejects_non_finite_endpoint() {
        let result = LineSegment3::new(point(0.0, 0.0, 0.0), point(0.0, f64::INFINITY, 0.0));
        assert_eq!(result, Err(GeometryError::NonFinitePoint { index: 1 }));
    }

    #[test]
    fn polyline_requires_at_least_two_points() {
        assert_eq!(
            Polyline3::new(Vec::new()),
            Err(GeometryError::PolylineRequiresTwoPoints { actual: 0 })
        );
        assert_eq!(
            Polyline3::new(vec![point(0.0, 0.0, 0.0)]),
            Err(GeometryError::PolylineRequiresTwoPoints { actual: 1 })
        );
    }

    #[test]
    fn polyline_segments_follow_point_order() {
        let polyline = Polyline3::new(vec![
            point(0.0, 0.0, 0.0),
            point(3.0, 0.0, 0.0),
            point(3.0, 4.0, 0.0),
        ])
        .unwrap();

        let segments: Vec<_> = polyline.segments().collect();
        assert_eq!(polyline.len(), 3);
        assert_eq!(polyline.segment_count(), 2);
        assert_eq!(segments[0].start(), point(0.0, 0.0, 0.0));
        assert_eq!(segments[0].end(), point(3.0, 0.0, 0.0));
        assert_eq!(segments[1].start(), point(3.0, 0.0, 0.0));
        assert_eq!(segments[1].end(), point(3.0, 4.0, 0.0));
        assert_eq!(segments.len(), 2);
    }

    #[test]
    fn polyline_preserves_repeated_points() {
        let repeated = point(1.0, 1.0, 1.0);
        let polyline = Polyline3::new(vec![repeated, repeated, point(2.0, 1.0, 1.0)]).unwrap();

        let segments: Vec<_> = polyline.segments().collect();
        assert_eq!(segments[0].length(), 0.0);
        assert_eq!(segments[1].length(), 1.0);
    }

    #[test]
    fn polyline_rejects_non_finite_coordinates_with_index() {
        let result = Polyline3::new(vec![point(0.0, 0.0, 0.0), point(1.0, 0.0, f64::NEG_INFINITY)]);
        assert_eq!(result, Err(GeometryError::NonFinitePoint { index: 1 }));
    }

    #[test]
    fn closed_polyline_is_explicitly_closed_by_repeating_first_point() {
        let first = point(0.0, 0.0, 0.0);
        let polyline = Polyline3::new(vec![
            first,
            point(1.0, 0.0, 0.0),
            point(0.0, 1.0, 0.0),
            first,
        ])
        .unwrap();

        assert_eq!(polyline.segment_count(), 3);
        assert_eq!(polyline.points().first(), polyline.points().last());
    }
}
