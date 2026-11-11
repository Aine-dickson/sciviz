//! Deterministic camera setup from the command line (stage 5b).
//!
//! The options are applied once, to the fitted camera, before the window is
//! created. They reuse the interactive `orbit` and `zoom` operations, so a
//! scripted view and a dragged view obey the same geometry.
//!
//! Definitions (also in the Explorer usage text and the evidence README):
//! * The camera orbits the fitted target; the target and the world-up axis
//!   (+Y) never change.
//! * Azimuth is `atan2(x, z)` of the camera offset from the target, about
//!   world +Y. Positive yaw DEcreases the azimuth, the same as dragging the
//!   pointer to the right by that many degrees.
//! * Elevation is the angle of the offset above the world XZ plane. Positive
//!   pitch raises the camera. Fitted elevation plus pitch must stay within
//!   +-(90 degrees - 0.02 rad); it is rejected, never clamped.
//! * Yaw and pitch are applied together in one orbit, then the zoom. Zoom is
//!   a factor on the fitted camera-target distance: the new distance is
//!   `fitted distance / factor`, so a factor above 1 moves closer.

use svf_core::{
    camera::Camera3,
    camera_navigation::{self, CameraNavigationConfig},
};

/// Pole margin used by interactive navigation (`CameraNavigationConfig::for_camera`).
const POLE_MARGIN_RADIANS: f64 = 0.02;
/// Interactive zoom sensitivity; the exponent is `-wheel * this`.
const ZOOM_EXPONENT_PER_UNIT: f64 = 0.12;

/// Requested view relative to the fitted camera. `None` leaves that part of
/// the fitted camera untouched.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CameraSetup {
    pub yaw_degrees: Option<f64>,
    pub pitch_degrees: Option<f64>,
    pub zoom_factor: Option<f64>,
}

impl CameraSetup {
    pub fn is_requested(&self) -> bool {
        self.yaw_degrees.is_some() || self.pitch_degrees.is_some() || self.zoom_factor.is_some()
    }
}

/// Apply the requested view to the fitted camera. With nothing requested the
/// fitted camera is returned bit for bit.
pub fn apply_camera_setup(fitted: &Camera3, setup: &CameraSetup) -> Result<Camera3, String> {
    if !setup.is_requested() {
        return Ok(*fitted);
    }
    let limits = CameraNavigationConfig::for_camera(fitted)
        .map_err(|error| format!("camera setup: {error}"))?;
    let (minimum, maximum) = (limits.min_distance(), limits.max_distance());
    let mut camera = *fitted;

    if setup.yaw_degrees.is_some() || setup.pitch_degrees.is_some() {
        let yaw = setup.yaw_degrees.unwrap_or(0.0);
        let pitch = setup.pitch_degrees.unwrap_or(0.0);

        let pose = fitted.look_at();
        let offset = pose.position().to_vector() - pose.target().to_vector();
        let fitted_elevation = offset.y.atan2(offset.x.hypot(offset.z));
        let limit = std::f64::consts::FRAC_PI_2 - POLE_MARGIN_RADIANS;
        let requested = fitted_elevation + pitch.to_radians();
        if requested.abs() > limit {
            let low = (-limit - fitted_elevation).to_degrees();
            let high = (limit - fitted_elevation).to_degrees();
            return Err(format!(
                "--camera-pitch {pitch} puts the camera outside the allowed elevation range; \
                 for this scene the allowed pitch is {low:.2} to {high:.2} degrees"
            ));
        }

        // One "pixel" of the interactive orbit is one degree here.
        let orbit_config = CameraNavigationConfig::new(
            minimum,
            maximum,
            POLE_MARGIN_RADIANS,
            1.0_f64.to_radians(),
            ZOOM_EXPONENT_PER_UNIT,
        ).map_err(|error| format!("camera setup: {error}"))?;
        camera = camera_navigation::orbit(&camera, yaw, pitch, orbit_config)
            .map_err(|error| format!("camera setup: {error}"))?;
    }

    if let Some(factor) = setup.zoom_factor {
        // Distance may range over [minimum, maximum], i.e. the factor over
        // [fitted distance / maximum, fitted distance / minimum].
        let fitted_distance = (fitted.look_at().position().to_vector()
            - fitted.look_at().target().to_vector()).length();
        let (low, high) = (fitted_distance / maximum, fitted_distance / minimum);
        if !(factor >= low * (1.0 - 1.0e-12) && factor <= high * (1.0 + 1.0e-12)) {
            return Err(format!(
                "--camera-zoom {factor} is outside the allowed range {low} to {high} \
                 (camera distance may change by at most a factor of {high} either way)"
            ));
        }
        let zoom_config = CameraNavigationConfig::new(
            minimum,
            maximum,
            POLE_MARGIN_RADIANS,
            1.0_f64.to_radians(),
            ZOOM_EXPONENT_PER_UNIT,
        ).map_err(|error| format!("camera setup: {error}"))?;
        camera = camera_navigation::zoom(&camera, factor.ln() / ZOOM_EXPONENT_PER_UNIT, zoom_config)
            .map_err(|error| format!("camera setup: {error}"))?;
    }
    Ok(camera)
}

#[cfg(test)]
mod tests {
    use super::*;
    use svf_core::{
        camera::{LookAtCamera, PerspectiveProjection, Projection3},
        spatial::{WorldPoint3, WorldVector3},
    };

    /// Camera on +Z looking at the origin (azimuth 0, elevation 0).
    fn camera_on_z(distance: f64) -> Camera3 {
        let pose = LookAtCamera::new(
            WorldPoint3::new(0.0, 0.0, distance),
            WorldPoint3::new(0.0, 0.0, 0.0),
            WorldVector3::new(0.0, 1.0, 0.0),
        ).unwrap();
        let projection = PerspectiveProjection::new(0.8, 4.0 / 3.0, 0.1, 100.0).unwrap();
        Camera3::new(pose, Projection3::Perspective(projection))
    }

    /// Camera offset from its target as (x, y, z).
    fn offset(camera: &Camera3) -> (f64, f64, f64) {
        let v = camera.look_at().position().to_vector() - camera.look_at().target().to_vector();
        (v.x, v.y, v.z)
    }

    fn distance(camera: &Camera3) -> f64 {
        let (x, y, z) = offset(camera);
        x.hypot(y).hypot(z)
    }

    #[test]
    fn omitted_options_return_the_fitted_camera_unchanged() {
        let fitted = camera_on_z(10.0);
        let applied = apply_camera_setup(&fitted, &CameraSetup::default()).unwrap();
        assert_eq!(applied, fitted);
    }

    #[test]
    fn positive_yaw_from_plus_z_moves_the_camera_to_minus_x() {
        let moved = apply_camera_setup(
            &camera_on_z(10.0),
            &CameraSetup { yaw_degrees: Some(90.0), ..CameraSetup::default() },
        ).unwrap();
        let (x, y, z) = offset(&moved);
        assert!((x + 10.0).abs() < 1.0e-9 && y.abs() < 1.0e-9 && z.abs() < 1.0e-9, "{x} {y} {z}");
    }

    #[test]
    fn positive_pitch_raises_the_camera_and_keeps_distance_and_target() {
        let fitted = camera_on_z(10.0);
        let moved = apply_camera_setup(
            &fitted,
            &CameraSetup { pitch_degrees: Some(30.0), ..CameraSetup::default() },
        ).unwrap();
        let (_, y, _) = offset(&moved);
        assert!((y - 5.0).abs() < 1.0e-9, "{y}");
        assert!((distance(&moved) - 10.0).abs() < 1.0e-9);
        assert_eq!(moved.look_at().target(), fitted.look_at().target());
    }

    #[test]
    fn zoom_factor_divides_the_distance() {
        let fitted = camera_on_z(10.0);
        let closer = apply_camera_setup(
            &fitted,
            &CameraSetup { zoom_factor: Some(2.0), ..CameraSetup::default() },
        ).unwrap();
        assert!((distance(&closer) - 5.0).abs() < 1.0e-9);
        let farther = apply_camera_setup(
            &fitted,
            &CameraSetup { zoom_factor: Some(0.5), ..CameraSetup::default() },
        ).unwrap();
        assert!((distance(&farther) - 20.0).abs() < 1.0e-9);
        assert_eq!(closer.look_at().target(), fitted.look_at().target());
    }

    #[test]
    fn yaw_pitch_and_zoom_combine_deterministically() {
        let setup = CameraSetup {
            yaw_degrees: Some(-40.0),
            pitch_degrees: Some(-15.0),
            zoom_factor: Some(3.0),
        };
        let first = apply_camera_setup(&camera_on_z(10.0), &setup).unwrap();
        let second = apply_camera_setup(&camera_on_z(10.0), &setup).unwrap();
        assert_eq!(first, second);
        assert!((distance(&first) - 10.0 / 3.0).abs() < 1.0e-9);
    }

    #[test]
    fn out_of_range_pitch_and_zoom_are_rejected_not_clamped() {
        let fitted = camera_on_z(10.0);
        let pitch = apply_camera_setup(
            &fitted,
            &CameraSetup { pitch_degrees: Some(89.5), ..CameraSetup::default() },
        ).unwrap_err();
        assert!(pitch.contains("allowed pitch"), "{pitch}");
        for factor in [100.1, 0.0099] {
            let error = apply_camera_setup(
                &fitted,
                &CameraSetup { zoom_factor: Some(factor), ..CameraSetup::default() },
            ).unwrap_err();
            assert!(error.contains("--camera-zoom"), "{error}");
        }
        // The documented limits themselves are accepted.
        for factor in [100.0, 0.01] {
            apply_camera_setup(
                &fitted,
                &CameraSetup { zoom_factor: Some(factor), ..CameraSetup::default() },
            ).unwrap();
        }
    }
}
