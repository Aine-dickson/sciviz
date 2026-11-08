//! Input-independent camera navigation operations.
//!
//! All navigation is performed in f64 world coordinates. This module has no
//! window-system or renderer dependencies.

use std::{error::Error, fmt};

use glam::DVec3;

use crate::{
    camera::{Camera3, CameraError, LookAtCamera, PerspectiveProjection, Projection3},
    spatial::{WorldPoint3, WorldVector3},
};

/// Validated limits and sensitivities for perspective-camera navigation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraNavigationConfig {
    min_distance: f64,
    max_distance: f64,
    pole_margin_radians: f64,
    orbit_radians_per_pixel: f64,
    zoom_exponent_per_wheel_unit: f64,
}

impl CameraNavigationConfig {
    /// Creates navigation settings with explicit distance and sensitivity limits.
    pub fn new(
        min_distance: f64,
        max_distance: f64,
        pole_margin_radians: f64,
        orbit_radians_per_pixel: f64,
        zoom_exponent_per_wheel_unit: f64,
    ) -> Result<Self, CameraNavigationError> {
        if ![
            min_distance,
            max_distance,
            pole_margin_radians,
            orbit_radians_per_pixel,
            zoom_exponent_per_wheel_unit,
        ].into_iter().all(f64::is_finite)
            || min_distance <= 0.0
            || max_distance <= min_distance
            || pole_margin_radians <= 0.0
            || pole_margin_radians >= std::f64::consts::FRAC_PI_2
            || orbit_radians_per_pixel <= 0.0
            || zoom_exponent_per_wheel_unit <= 0.0
        {
            return Err(CameraNavigationError::InvalidConfiguration);
        }
        Ok(Self {
            min_distance,
            max_distance,
            pole_margin_radians,
            orbit_radians_per_pixel,
            zoom_exponent_per_wheel_unit,
        })
    }

    /// Minimum camera-target distance.
    #[must_use]
    pub const fn min_distance(self) -> f64 { self.min_distance }

    /// Maximum camera-target distance.
    #[must_use]
    pub const fn max_distance(self) -> f64 { self.max_distance }
}

impl Default for CameraNavigationConfig {
    fn default() -> Self {
        Self {
            min_distance: 1.0,
            max_distance: 80.0,
            pole_margin_radians: 0.02,
            orbit_radians_per_pixel: 0.006,
            zoom_exponent_per_wheel_unit: 0.12,
        }
    }
}

/// Errors raised by camera navigation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraNavigationError {
    InvalidConfiguration,
    NonFiniteInput,
    InvalidViewport,
    UnsupportedProjection,
    InvalidCamera(CameraError),
}

impl fmt::Display for CameraNavigationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfiguration => f.write_str("camera navigation configuration is invalid"),
            Self::NonFiniteInput => f.write_str("camera navigation input must be finite"),
            Self::InvalidViewport => f.write_str("camera navigation viewport height must be non-zero"),
            Self::UnsupportedProjection => f.write_str("camera navigation currently requires a perspective projection"),
            Self::InvalidCamera(error) => write!(f, "camera navigation produced an invalid camera: {error}"),
        }
    }
}

impl Error for CameraNavigationError {}

/// Rotates the camera around its current target.
///
/// Pointer deltas are physical pixels. Horizontal movement changes azimuth;
/// vertical movement changes elevation. Elevation is clamped away from the
/// world-up singularities.
pub fn orbit(
    camera: &Camera3,
    delta_x_pixels: f64,
    delta_y_pixels: f64,
    config: CameraNavigationConfig,
) -> Result<Camera3, CameraNavigationError> {
    if !delta_x_pixels.is_finite() || !delta_y_pixels.is_finite() {
        return Err(CameraNavigationError::NonFiniteInput);
    }
    let pose = camera.look_at();
    let target = pose.target().to_vector();
    let offset = pose.position().to_vector() - target;
    let distance = offset.length();
    if !distance.is_finite() || distance == 0.0 {
        return Err(CameraNavigationError::InvalidCamera(CameraError::CameraTargetCoincidentWithPosition));
    }

    let horizontal = offset.x.hypot(offset.z);
    let mut azimuth = offset.x.atan2(offset.z);
    let mut elevation = offset.y.atan2(horizontal);
    azimuth -= delta_x_pixels * config.orbit_radians_per_pixel;
    elevation = (elevation + delta_y_pixels * config.orbit_radians_per_pixel)
        .clamp(
            -std::f64::consts::FRAC_PI_2 + config.pole_margin_radians,
            std::f64::consts::FRAC_PI_2 - config.pole_margin_radians,
        );

    let horizontal_distance = distance * elevation.cos();
    let new_offset = DVec3::new(
        horizontal_distance * azimuth.sin(),
        distance * elevation.sin(),
        horizontal_distance * azimuth.cos(),
    );
    build_camera(
        WorldPoint3::from_vector(target + new_offset),
        pose.target(),
        camera.projection().to_owned(),
    )
}

/// Pans camera position and target together in the camera plane.
///
/// Pointer deltas are physical pixels. The vertical field of view and viewport
/// height determine the world-space distance represented by each pixel.
pub fn pan(
    camera: &Camera3,
    delta_x_pixels: f64,
    delta_y_pixels: f64,
    viewport_height: u32,
) -> Result<Camera3, CameraNavigationError> {
    if !delta_x_pixels.is_finite() || !delta_y_pixels.is_finite() {
        return Err(CameraNavigationError::NonFiniteInput);
    }
    if viewport_height == 0 {
        return Err(CameraNavigationError::InvalidViewport);
    }
    let Projection3::Perspective(projection) = camera.projection() else {
        return Err(CameraNavigationError::UnsupportedProjection);
    };
    let pose = camera.look_at();
    let forward = (pose.target().to_vector() - pose.position().to_vector()).normalize();
    let right = forward.cross(DVec3::Y).normalize();
    let camera_up = right.cross(forward).normalize();
    let distance = pose.position().to_vector().distance(pose.target().to_vector());
    let world_per_pixel =
        2.0 * distance * (projection.vertical_fov_radians() * 0.5).tan()
            / f64::from(viewport_height);
    let translation = right * (-delta_x_pixels * world_per_pixel)
        + camera_up * (delta_y_pixels * world_per_pixel);
    if !translation.is_finite() {
        return Err(CameraNavigationError::NonFiniteInput);
    }
    build_camera(
        WorldPoint3::from_vector(pose.position().to_vector() + translation),
        WorldPoint3::from_vector(pose.target().to_vector() + translation),
        *camera.projection(),
    )
}

/// Zooms by an exponential wheel delta and clamps camera-target distance.
///
/// Positive wheel deltas zoom in; negative values zoom out.
pub fn zoom(
    camera: &Camera3,
    wheel_delta: f64,
    config: CameraNavigationConfig,
) -> Result<Camera3, CameraNavigationError> {
    if !wheel_delta.is_finite() {
        return Err(CameraNavigationError::NonFiniteInput);
    }
    let pose = camera.look_at();
    let target = pose.target().to_vector();
    let offset = pose.position().to_vector() - target;
    let distance = offset.length();
    if !distance.is_finite() || distance == 0.0 {
        return Err(CameraNavigationError::InvalidCamera(CameraError::CameraTargetCoincidentWithPosition));
    }
    let exponent = -wheel_delta * config.zoom_exponent_per_wheel_unit;
    let requested_distance = distance * exponent.exp();
    let new_distance = if requested_distance.is_finite() {
        requested_distance.clamp(config.min_distance, config.max_distance)
    } else if exponent.is_sign_positive() {
        config.max_distance
    } else {
        config.min_distance
    };
    let new_position = target + offset * (new_distance / distance);
    build_camera(
        WorldPoint3::from_vector(new_position),
        pose.target(),
        *camera.projection(),
    )
}

fn build_camera(
    position: WorldPoint3,
    target: WorldPoint3,
    projection: Projection3,
) -> Result<Camera3, CameraNavigationError> {
    let pose = LookAtCamera::new(position, target, WorldVector3::new(0.0, 1.0, 0.0))
        .map_err(CameraNavigationError::InvalidCamera)?;
    Ok(Camera3::new(pose, projection))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn camera() -> Camera3 {
        let pose = LookAtCamera::new(
            WorldPoint3::new(14.0, 12.0, 18.0),
            WorldPoint3::ORIGIN,
            WorldVector3::new(0.0, 1.0, 0.0),
        ).unwrap();
        let projection = PerspectiveProjection::new(45.0_f64.to_radians(), 1.6, 0.1, 100.0).unwrap();
        Camera3::new(pose, Projection3::Perspective(projection))
    }

    fn distance(camera: &Camera3) -> f64 {
        camera.look_at().position().to_vector().distance(camera.look_at().target().to_vector())
    }

    #[test]
    fn orbit_preserves_target_and_distance_and_remains_valid_at_large_deltas() {
        let original = camera();
        let result = orbit(&original, 120.0, 1.0e9, CameraNavigationConfig::default()).unwrap();
        assert_eq!(result.look_at().target(), original.look_at().target());
        assert!((distance(&result) - distance(&original)).abs() < 1.0e-10);
        assert!(result.look_at().position().to_vector().is_finite());
        assert!(result.look_at().up().to_vector().abs_diff_eq(DVec3::Y, 1.0e-12));
    }

    #[test]
    fn pan_translates_camera_and_target_equally() {
        let original = camera();
        let result = pan(&original, 30.0, -12.0, 800).unwrap();
        let position_delta = result.look_at().position() - original.look_at().position();
        let target_delta = result.look_at().target() - original.look_at().target();
        assert!(position_delta.to_vector().abs_diff_eq(target_delta.to_vector(), 1.0e-12));
        assert!((distance(&result) - distance(&original)).abs() < 1.0e-10);
    }

    #[test]
    fn zoom_clamps_distance_and_preserves_target() {
        let original = camera();
        let config = CameraNavigationConfig::default();
        let near = zoom(&original, 1.0e9, config).unwrap();
        let far = zoom(&original, -1.0e9, config).unwrap();
        assert!((distance(&near) - config.min_distance()).abs() < 1.0e-10);
        assert!((distance(&far) - config.max_distance()).abs() < 1.0e-10);
        assert_eq!(near.look_at().target(), original.look_at().target());
        assert_eq!(far.look_at().target(), original.look_at().target());
    }

    #[test]
    fn navigation_rejects_non_finite_input_and_zero_viewport() {
        let original = camera();
        assert_eq!(orbit(&original, f64::NAN, 0.0, CameraNavigationConfig::default()), Err(CameraNavigationError::NonFiniteInput));
        assert_eq!(pan(&original, 0.0, 0.0, 0), Err(CameraNavigationError::InvalidViewport));
        assert_eq!(zoom(&original, f64::INFINITY, CameraNavigationConfig::default()), Err(CameraNavigationError::NonFiniteInput));
    }

    #[test]
    fn invalid_navigation_configuration_is_rejected() {
        assert_eq!(
            CameraNavigationConfig::new(0.0, 10.0, 0.02, 0.006, 0.12),
            Err(CameraNavigationError::InvalidConfiguration)
        );
        assert_eq!(
            CameraNavigationConfig::new(1.0, 10.0, 0.0, 0.006, 0.12),
            Err(CameraNavigationError::InvalidConfiguration)
        );
    }
}
