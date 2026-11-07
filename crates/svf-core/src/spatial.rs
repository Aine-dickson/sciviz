//! Graphics-independent spatial primitives and coordinate conventions.
//!
//! SVF uses a right-handed world coordinate system with +Y up. Camera view
//! space uses +X right, +Y up, and -Z forward. Vectors are column vectors;
//! matrices multiply them from the left. Matrix storage is column-major.
//!
//! These are mathematical conventions, not a commitment to a particular
//! renderer or screen-coordinate system.

pub use glam::{DMat3, DMat4, DQuat, DVec2, DVec3, DVec4};

/// Positive world X axis.
pub const WORLD_X: DVec3 = DVec3::X;
/// Positive world Y axis (up).
pub const WORLD_UP: DVec3 = DVec3::Y;
/// Positive world Z axis.
pub const WORLD_Z: DVec3 = DVec3::Z;

/// A position in the SVF world coordinate system.
///
/// A point is distinct from a displacement vector: subtracting two points
/// produces a vector, while translating a point requires a vector.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WorldPoint3(DVec3);

impl WorldPoint3 {
    /// The world-space origin.
    pub const ORIGIN: Self = Self(DVec3::ZERO);

    /// Creates a world-space point from Cartesian coordinates.
    #[must_use]
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self(DVec3::new(x, y, z))
    }

    /// Creates a world-space point from a glam vector.
    #[must_use]
    pub const fn from_vector(value: DVec3) -> Self {
        Self(value)
    }

    /// Returns the underlying f64 coordinates.
    #[must_use]
    pub const fn to_vector(self) -> DVec3 {
        self.0
    }

    /// Returns the coordinates as [x, y, z].
    #[must_use]
    pub const fn to_array(self) -> [f64; 3] {
        self.0.to_array()
    }
}

/// A displacement or free vector expressed in world coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WorldVector3(DVec3);

impl WorldVector3 {
    /// The zero displacement.
    pub const ZERO: Self = Self(DVec3::ZERO);

    /// Creates a world-space vector from Cartesian components.
    #[must_use]
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self(DVec3::new(x, y, z))
    }

    /// Creates a world-space vector from a glam vector.
    #[must_use]
    pub const fn from_vector(value: DVec3) -> Self {
        Self(value)
    }

    /// Returns the underlying f64 components.
    #[must_use]
    pub const fn to_vector(self) -> DVec3 {
        self.0
    }

    /// Returns the vector magnitude.
    #[must_use]
    pub fn length(self) -> f64 {
        self.0.length()
    }

    /// Returns the dot product with another world-space vector.
    #[must_use]
    pub fn dot(self, rhs: Self) -> f64 {
        self.0.dot(rhs.0)
    }

    /// Returns the cross product with another world-space vector.
    #[must_use]
    pub fn cross(self, rhs: Self) -> Self {
        Self(self.0.cross(rhs.0))
    }
}

impl std::ops::Add<WorldVector3> for WorldPoint3 {
    type Output = Self;
    fn add(self, rhs: WorldVector3) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::Sub<WorldVector3> for WorldPoint3 {
    type Output = Self;
    fn sub(self, rhs: WorldVector3) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl std::ops::Sub<WorldPoint3> for WorldPoint3 {
    type Output = WorldVector3;
    fn sub(self, rhs: WorldPoint3) -> Self::Output {
        WorldVector3(self.0 - rhs.0)
    }
}

impl std::ops::Add for WorldVector3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::Sub for WorldVector3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl std::ops::Neg for WorldVector3 {
    type Output = Self;
    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl std::ops::Mul<f64> for WorldVector3 {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self::Output {
        Self(self.0 * rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1.0e-12;

    fn approx_eq(lhs: f64, rhs: f64) {
        assert!((lhs - rhs).abs() <= EPSILON, "{lhs} != {rhs}");
    }

    #[test]
    fn world_axes_follow_the_documented_convention() {
        assert_eq!(WORLD_X, DVec3::new(1.0, 0.0, 0.0));
        assert_eq!(WORLD_UP, DVec3::new(0.0, 1.0, 0.0));
        assert_eq!(WORLD_Z, DVec3::new(0.0, 0.0, 1.0));
        assert!(WORLD_X.cross(WORLD_UP).abs_diff_eq(WORLD_Z, EPSILON));
    }

    #[test]
    fn subtracting_points_produces_a_displacement() {
        let a = WorldPoint3::new(4.0, 7.0, -2.0);
        let b = WorldPoint3::new(1.0, 2.0, -5.0);
        let displacement = a - b;
        assert_eq!(displacement.to_vector(), DVec3::new(3.0, 5.0, 3.0));
        assert_eq!((b + displacement).to_array(), a.to_array());
    }

    #[test]
    fn vector_operations_preserve_f64_math() {
        let x = WorldVector3::new(1.0, 0.0, 0.0);
        let y = WorldVector3::new(0.0, 1.0, 0.0);
        assert_eq!(x.cross(y), WorldVector3::from_vector(WORLD_Z));
        approx_eq(x.dot(y), 0.0);
        approx_eq(WorldVector3::new(3.0, 4.0, 0.0).length(), 5.0);
    }

    #[test]
    fn glam_matrices_use_column_vector_composition() {
        let translation = DMat4::from_translation(DVec3::new(10.0, 0.0, 0.0));
        let scale = DMat4::from_scale(DVec3::splat(2.0));
        let local_point = DVec3::new(1.0, 0.0, 0.0);
        let transformed = (translation * scale).transform_point3(local_point);
        assert_eq!(transformed, DVec3::new(12.0, 0.0, 0.0));
    }
}
