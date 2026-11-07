//! Camera state and projection mathematics.
//!
//! Camera and projection types are graphics-independent and use f64
//! coordinates. Matrices use right-handed view space (-Z forward) and the
//! WebGPU NDC depth interval [0, 1].

use std::{error::Error, fmt};

use glam::{DMat4, DVec3};

use crate::spatial::{WorldPoint3, WorldVector3};

const PARALLEL_EPSILON_SQUARED: f64 = 1.0e-24;

/// Invalid camera or projection configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraError {
    NonFiniteCameraInput,
    CameraTargetCoincidentWithPosition,
    ZeroLengthUpDirection,
    UpDirectionParallelToViewDirection,
    InvalidVerticalFieldOfView,
    InvalidAspectRatio,
    InvalidNearFarPlanes,
    InvalidOrthographicBounds,
}

impl fmt::Display for CameraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::NonFiniteCameraInput => "camera inputs must be finite",
            Self::CameraTargetCoincidentWithPosition => "camera position and target must differ",
            Self::ZeroLengthUpDirection => "camera up direction must be non-zero",
            Self::UpDirectionParallelToViewDirection => "camera up must not be parallel to view",
            Self::InvalidVerticalFieldOfView => "vertical field of view must be between 0 and pi",
            Self::InvalidAspectRatio => "aspect ratio must be finite and greater than zero",
            Self::InvalidNearFarPlanes => "near and far must be finite, with 0 < near < far",
            Self::InvalidOrthographicBounds => "orthographic bounds must be finite with positive extents",
        };
        f.write_str(message)
    }
}

impl Error for CameraError {}

/// A camera pose represented by a world-space position and look-at target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LookAtCamera {
    position: WorldPoint3,
    target: WorldPoint3,
    up: WorldVector3,
}

impl LookAtCamera {
    /// Creates a validated right-handed look-at camera.
    ///
    /// The camera looks from position toward target. The world-space up
    /// direction need not be normalized.
    pub fn new(
        position: WorldPoint3,
        target: WorldPoint3,
        up: WorldVector3,
    ) -> Result<Self, CameraError> {
        let eye = position.to_vector();
        let target_vector = target.to_vector();
        let up_vector = up.to_vector();

        if !eye.is_finite() || !target_vector.is_finite() || !up_vector.is_finite() {
            return Err(CameraError::NonFiniteCameraInput);
        }

        let direction = target_vector - eye;
        let direction_length_squared = direction.length_squared();
        if direction_length_squared == 0.0 {
            return Err(CameraError::CameraTargetCoincidentWithPosition);
        }

        let up_length_squared = up_vector.length_squared();
        if up_length_squared == 0.0 {
            return Err(CameraError::ZeroLengthUpDirection);
        }

        let cross_length_squared = direction.cross(up_vector).length_squared();
        if cross_length_squared
            <= direction_length_squared * up_length_squared * PARALLEL_EPSILON_SQUARED
        {
            return Err(CameraError::UpDirectionParallelToViewDirection);
        }

        Ok(Self { position, target, up })
    }

    #[must_use]
    pub const fn position(&self) -> WorldPoint3 { self.position }

    #[must_use]
    pub const fn target(&self) -> WorldPoint3 { self.target }

    #[must_use]
    pub const fn up(&self) -> WorldVector3 { self.up }

    /// Derives the world-to-view transformation.
    #[must_use]
    pub fn view_matrix(&self) -> DMat4 {
        DMat4::look_at_rh(
            self.position.to_vector(),
            self.target.to_vector(),
            self.up.to_vector(),
        )
    }
}

/// A validated perspective projection using vertical field of view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PerspectiveProjection {
    vertical_fov_radians: f64,
    aspect_ratio: f64,
    near: f64,
    far: f64,
}

impl PerspectiveProjection {
    /// Creates a perspective projection with WebGPU [0, 1] NDC depth.
    pub fn new(
        vertical_fov_radians: f64,
        aspect_ratio: f64,
        near: f64,
        far: f64,
    ) -> Result<Self, CameraError> {
        if !vertical_fov_radians.is_finite()
            || vertical_fov_radians <= 0.0
            || vertical_fov_radians >= std::f64::consts::PI
        {
            return Err(CameraError::InvalidVerticalFieldOfView);
        }
        if !aspect_ratio.is_finite() || aspect_ratio <= 0.0 {
            return Err(CameraError::InvalidAspectRatio);
        }
        validate_near_far(near, far)?;
        Ok(Self { vertical_fov_radians, aspect_ratio, near, far })
    }

    #[must_use]
    pub const fn vertical_fov_radians(&self) -> f64 { self.vertical_fov_radians }

    #[must_use]
    pub const fn aspect_ratio(&self) -> f64 { self.aspect_ratio }

    #[must_use]
    pub const fn near(&self) -> f64 { self.near }

    #[must_use]
    pub const fn far(&self) -> f64 { self.far }

    /// Returns a copy with a newly validated aspect ratio.
    pub fn with_aspect_ratio(self, aspect_ratio: f64) -> Result<Self, CameraError> {
        Self::new(self.vertical_fov_radians, aspect_ratio, self.near, self.far)
    }

    /// Builds a right-handed perspective matrix with [0, 1] NDC depth.
    #[must_use]
    pub fn matrix(&self) -> DMat4 {
        DMat4::perspective_rh(
            self.vertical_fov_radians,
            self.aspect_ratio,
            self.near,
            self.far,
        )
    }
}

/// A validated orthographic projection defined by view-space bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrthographicProjection {
    left: f64,
    right: f64,
    bottom: f64,
    top: f64,
    near: f64,
    far: f64,
}

impl OrthographicProjection {
    /// Creates an orthographic projection with WebGPU [0, 1] NDC depth.
    ///
    /// Bounds are view-space extents. Near and far are positive distances
    /// from the camera, not signed view-space Z coordinates.
    pub fn new(
        left: f64,
        right: f64,
        bottom: f64,
        top: f64,
        near: f64,
        far: f64,
    ) -> Result<Self, CameraError> {
        if ![left, right, bottom, top].into_iter().all(f64::is_finite)
            || left >= right
            || bottom >= top
        {
            return Err(CameraError::InvalidOrthographicBounds);
        }
        validate_near_far(near, far)?;
        Ok(Self { left, right, bottom, top, near, far })
    }

    /// Builds a right-handed orthographic matrix with [0, 1] NDC depth.
    #[must_use]
    pub fn matrix(&self) -> DMat4 {
        DMat4::orthographic_rh(
            self.left, self.right, self.bottom, self.top, self.near, self.far,
        )
    }
}

/// A camera projection mode.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Projection3 {
    Perspective(PerspectiveProjection),
    Orthographic(OrthographicProjection),
}

impl Projection3 {
    #[must_use]
    pub fn matrix(&self) -> DMat4 {
        match self {
            Self::Perspective(projection) => projection.matrix(),
            Self::Orthographic(projection) => projection.matrix(),
        }
    }

    /// Updates perspective aspect ratio. Orthographic bounds remain explicit.
    pub fn with_aspect_ratio(self, aspect_ratio: f64) -> Result<Self, CameraError> {
        match self {
            Self::Perspective(projection) => {
                Ok(Self::Perspective(projection.with_aspect_ratio(aspect_ratio)?))
            }
            Self::Orthographic(_) => Err(CameraError::InvalidAspectRatio),
        }
    }
}

/// A validated 3D camera pose paired with a projection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera3 {
    look_at: LookAtCamera,
    projection: Projection3,
}

impl Camera3 {
    #[must_use]
    pub const fn new(look_at: LookAtCamera, projection: Projection3) -> Self {
        Self { look_at, projection }
    }

    #[must_use]
    pub const fn look_at(&self) -> &LookAtCamera { &self.look_at }

    #[must_use]
    pub const fn projection(&self) -> &Projection3 { &self.projection }

    #[must_use]
    pub fn view_matrix(&self) -> DMat4 { self.look_at.view_matrix() }

    #[must_use]
    pub fn projection_matrix(&self) -> DMat4 { self.projection.matrix() }

    /// Returns the world-to-clip transform: projection multiplied by view.
    #[must_use]
    pub fn view_projection_matrix(&self) -> DMat4 {
        self.projection_matrix() * self.view_matrix()
    }
}

fn validate_near_far(near: f64, far: f64) -> Result<(), CameraError> {
    if !near.is_finite() || !far.is_finite() || near <= 0.0 || far <= near {
        return Err(CameraError::InvalidNearFarPlanes);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1.0e-10;

    fn approx_eq(actual: f64, expected: f64) {
        assert!((actual - expected).abs() <= EPSILON, "expected {expected}, got {actual}");
    }

    fn point(x: f64, y: f64, z: f64) -> WorldPoint3 { WorldPoint3::new(x, y, z) }
    fn vector(x: f64, y: f64, z: f64) -> WorldVector3 { WorldVector3::new(x, y, z) }

    fn ndc(matrix: DMat4, point: DVec3) -> DVec3 {
        let clip = matrix * point.extend(1.0);
        clip.truncate() / clip.w
    }

    fn test_camera() -> LookAtCamera {
        LookAtCamera::new(
            point(0.0, 0.0, 5.0),
            point(0.0, 0.0, 0.0),
            vector(0.0, 1.0, 0.0),
        ).expect("valid test camera")
    }

    #[test]
    fn look_at_maps_eye_to_origin_and_target_to_negative_z() {
        let view = test_camera().view_matrix();
        assert!(view.transform_point3(DVec3::new(0.0, 0.0, 5.0))
            .abs_diff_eq(DVec3::ZERO, EPSILON));
        assert!(view.transform_point3(DVec3::ZERO)
            .abs_diff_eq(DVec3::new(0.0, 0.0, -5.0), EPSILON));
    }

    #[test]
    fn rejects_degenerate_camera_configurations() {
        assert_eq!(
            LookAtCamera::new(point(0.0, 0.0, 0.0), point(0.0, 0.0, 0.0), vector(0.0, 1.0, 0.0)),
            Err(CameraError::CameraTargetCoincidentWithPosition)
        );
        assert_eq!(
            LookAtCamera::new(point(0.0, 0.0, 1.0), point(0.0, 0.0, 0.0), WorldVector3::ZERO),
            Err(CameraError::ZeroLengthUpDirection)
        );
        assert_eq!(
            LookAtCamera::new(point(0.0, 0.0, 1.0), point(0.0, 0.0, 0.0), vector(0.0, 0.0, 2.0)),
            Err(CameraError::UpDirectionParallelToViewDirection)
        );
    }

    #[test]
    fn rejects_non_finite_camera_inputs() {
        assert_eq!(
            LookAtCamera::new(point(f64::NAN, 0.0, 1.0), point(0.0, 0.0, 0.0), vector(0.0, 1.0, 0.0)),
            Err(CameraError::NonFiniteCameraInput)
        );
    }

    #[test]
    fn perspective_maps_near_and_far_to_zero_and_one() {
        let projection = PerspectiveProjection::new(std::f64::consts::FRAC_PI_2, 1.0, 1.0, 11.0)
            .expect("valid perspective");
        let matrix = projection.matrix();
        approx_eq(ndc(matrix, DVec3::new(0.0, 0.0, -1.0)).z, 0.0);
        approx_eq(ndc(matrix, DVec3::new(0.0, 0.0, -11.0)).z, 1.0);
        approx_eq(ndc(matrix, DVec3::new(1.0, 0.0, -1.0)).x, 1.0);
    }

    #[test]
    fn perspective_aspect_update_preserves_other_parameters() {
        let projection = PerspectiveProjection::new(1.0, 1.0, 0.1, 100.0).unwrap();
        let resized = projection.with_aspect_ratio(2.0).unwrap();
        assert_eq!(resized.aspect_ratio(), 2.0);
        assert_eq!(resized.vertical_fov_radians(), projection.vertical_fov_radians());
        assert_eq!(resized.near(), projection.near());
        assert_eq!(resized.far(), projection.far());
    }

    #[test]
    fn rejects_invalid_perspective_parameters() {
        assert_eq!(PerspectiveProjection::new(0.0, 1.0, 0.1, 100.0), Err(CameraError::InvalidVerticalFieldOfView));
        assert_eq!(PerspectiveProjection::new(std::f64::consts::PI, 1.0, 0.1, 100.0), Err(CameraError::InvalidVerticalFieldOfView));
        assert_eq!(PerspectiveProjection::new(1.0, 0.0, 0.1, 100.0), Err(CameraError::InvalidAspectRatio));
        assert_eq!(PerspectiveProjection::new(1.0, 1.0, 0.0, 100.0), Err(CameraError::InvalidNearFarPlanes));
        assert_eq!(PerspectiveProjection::new(1.0, 1.0, 10.0, 1.0), Err(CameraError::InvalidNearFarPlanes));
    }

    #[test]
    fn orthographic_maps_bounds_and_depth_to_expected_ndc() {
        let projection = OrthographicProjection::new(-2.0, 2.0, -1.0, 1.0, 1.0, 11.0)
            .expect("valid orthographic");
        let matrix = projection.matrix();
        assert!(ndc(matrix, DVec3::new(2.0, 1.0, -1.0))
            .abs_diff_eq(DVec3::new(1.0, 1.0, 0.0), EPSILON));
        assert!(ndc(matrix, DVec3::new(-2.0, -1.0, -11.0))
            .abs_diff_eq(DVec3::new(-1.0, -1.0, 1.0), EPSILON));
    }

    #[test]
    fn rejects_invalid_orthographic_parameters() {
        assert_eq!(OrthographicProjection::new(1.0, 1.0, -1.0, 1.0, 0.1, 10.0), Err(CameraError::InvalidOrthographicBounds));
        assert_eq!(OrthographicProjection::new(-1.0, 1.0, 2.0, -2.0, 0.1, 10.0), Err(CameraError::InvalidOrthographicBounds));
        assert_eq!(OrthographicProjection::new(-1.0, 1.0, -1.0, 1.0, 10.0, 10.0), Err(CameraError::InvalidNearFarPlanes));
    }

    #[test]
    fn combined_matrix_applies_view_then_projection() {
        let pose = test_camera();
        let projection = PerspectiveProjection::new(std::f64::consts::FRAC_PI_2, 1.0, 1.0, 11.0).unwrap();
        let camera = Camera3::new(pose, Projection3::Perspective(projection));
        approx_eq(ndc(camera.view_projection_matrix(), DVec3::ZERO).z, 0.8);
    }
}
