//! Input-independent camera navigation operations.
//!
//! All navigation is performed in f64 world coordinates. This module has no
//! window-system or renderer dependencies.

use std::{error::Error, fmt};

use glam::DVec3;

use crate::{
    camera::{Camera3, CameraError, LookAtCamera, Projection3},
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

    /// Creates navigation limits relative to the camera's initial distance.
    ///
    /// The limits remain fixed for the lifetime of a navigation session, so
    /// repeated wheel events cannot move the limits along with the camera.
    pub fn for_camera(camera: &Camera3) -> Result<Self, CameraNavigationError> {
        let pose = camera.look_at();
        let distance = pose.position().to_vector().distance(pose.target().to_vector());
        if !distance.is_finite() || distance <= 0.0 {
            return Err(CameraNavigationError::InvalidCamera(
                CameraError::CameraTargetCoincidentWithPosition,
            ));
        }
        Self::new(distance * 0.01, distance * 100.0, 0.02, 0.006, 0.12)
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
            min_distance: 1.0e-6,
            max_distance: 1.0e12,
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
    let projection = match camera.projection() {
        Projection3::Perspective(projection) => {
            // Keep the scene inside the depth interval as the camera moves.
            // The near plane follows close zooms; the far plane expands for
            // distant views. Both remain positive and ordered.
            let near = if new_distance < distance {
                projection.near().min(new_distance * 0.01).max(f64::MIN_POSITIVE)
            } else {
                projection.near()
            };
            let far = if new_distance > distance {
                projection.far().max(new_distance * 1.5).max(near * 2.0)
            } else {
                projection.far()
            };
            Projection3::Perspective(
                crate::camera::PerspectiveProjection::new(
                    projection.vertical_fov_radians(),
                    projection.aspect_ratio(),
                    near,
                    far,
                ).map_err(CameraNavigationError::InvalidCamera)?,
            )
        }
        Projection3::Orthographic(_) => return Err(CameraNavigationError::UnsupportedProjection),
    };
    build_camera(
        WorldPoint3::from_vector(new_position),
        pose.target(),
        projection,
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
    use crate::camera::PerspectiveProjection;

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
    fn wheel_zoom_is_gradual_and_scale_aware_for_millimetre_geometry() {
        let pose = LookAtCamera::new(
            WorldPoint3::new(140.0, 120.0, 180.0),
            WorldPoint3::ORIGIN,
            WorldVector3::new(0.0, 1.0, 0.0),
        ).unwrap();
        let projection = PerspectiveProjection::new(
            45.0_f64.to_radians(), 1.6, 10.0, 400.0,
        ).unwrap();
        let original = Camera3::new(pose, Projection3::Perspective(projection));
        let config = CameraNavigationConfig::for_camera(&original).unwrap();
        let original_distance = distance(&original);

        let zoomed_in = zoom(&original, 1.0, config).unwrap();
        let zoomed_out = zoom(&original, -1.0, config).unwrap();

        assert!((distance(&zoomed_in) / original_distance - (-0.12_f64).exp()).abs() < 1.0e-12);
        assert!((distance(&zoomed_out) / original_distance - 0.12_f64.exp()).abs() < 1.0e-12);
        assert_eq!(zoomed_in.look_at().target(), original.look_at().target());
        assert_eq!(zoomed_out.look_at().target(), original.look_at().target());

        let Projection3::Perspective(zoomed_in_projection) = zoomed_in.projection() else {
            panic!("zoom must preserve perspective projection");
        };
        let Projection3::Perspective(zoomed_out_projection) = zoomed_out.projection() else {
            panic!("zoom must preserve perspective projection");
        };
        assert!(zoomed_in_projection.near() < projection.near());
        assert_eq!(zoomed_in_projection.far(), projection.far());
        assert_eq!(zoomed_out_projection.near(), projection.near());
        assert!(zoomed_out_projection.far() > projection.far());
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
