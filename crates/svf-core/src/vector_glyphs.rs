//! Explicitly scaled, graphics-independent 3D vector arrow geometry.
//!
//! This module consumes analyzed vector sources. It preserves every source
//! entry and generates f64 line segments only for vectors that can be shown
//! under the selected scale policy.

use std::{error::Error, fmt};

use glam::DVec3;

use crate::{
    spatial::{WorldPoint3, WorldVector3},
    vector_visualization::{VectorAnalysis3, VectorEntity3, VectorStatus3},
};

/// Selects an explicit mapping from scientific vector magnitude to display length.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VectorScalePolicy3 {
    /// Display length is magnitude multiplied by this finite, non-negative factor.
    MagnitudeScaled { factor: f64 },
    /// Every finite non-zero vector receives this finite, positive display length.
    UniformLength { length: f64 },
}

/// One world-space line segment belonging to a vector arrow.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VectorGlyphSegment3 {
    start: WorldPoint3,
    end: WorldPoint3,
}

impl VectorGlyphSegment3 {
    /// Returns the segment's start point.
    #[must_use]
    pub const fn start(&self) -> WorldPoint3 {
        self.start
    }

    /// Returns the segment's end point.
    #[must_use]
    pub const fn end(&self) -> WorldPoint3 {
        self.end
    }
}

/// One source vector and its optional three-segment arrow representation.
///
/// Geometry is absent for zero/invalid vectors, magnitude overflow under the
/// magnitude-scaled policy, or a valid zero display length. The original
/// analysis remains available in every case.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VectorGlyphEntry3 {
    analysis: VectorAnalysis3,
    segments: Option<[VectorGlyphSegment3; 3]>,
    display_length: Option<f64>,
}

impl VectorGlyphEntry3 {
    /// Returns the original source analysis and provenance.
    #[must_use]
    pub const fn analysis(&self) -> VectorAnalysis3 {
        self.analysis
    }

    /// Returns shaft and two arrowhead segments when geometry was generated.
    #[must_use]
    pub const fn segments(&self) -> Option<[VectorGlyphSegment3; 3]> {
        self.segments
    }

    /// Returns the applied display length, or None when no arrow was generated.
    #[must_use]
    pub const fn display_length(&self) -> Option<f64> {
        self.display_length
    }
}

/// Ordered CPU-side vector glyph output and the policy used to produce it.
#[derive(Clone, Debug, PartialEq)]
pub struct VectorGlyphSet3 {
    policy: VectorScalePolicy3,
    entries: Vec<VectorGlyphEntry3>,
}

impl VectorGlyphSet3 {
    /// Returns the exact scale policy and parameter used for this result.
    #[must_use]
    pub const fn policy(&self) -> VectorScalePolicy3 {
        self.policy
    }

    /// Returns entries in the same order as their source analyses.
    #[must_use]
    pub fn entries(&self) -> &[VectorGlyphEntry3] {
        &self.entries
    }
}

/// Errors produced while validating scale policies or constructing arrow geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VectorGlyphError {
    /// Magnitude scale factors must be finite and non-negative.
    InvalidMagnitudeScaleFactor { factor: f64 },
    /// Uniform display lengths must be finite and strictly positive.
    InvalidUniformLength { length: f64 },
    /// A finite magnitude and scale factor produced an unrepresentable length.
    DisplayLengthOverflow { entity: VectorEntity3 },
    /// A generated arrow endpoint is not finite in f64 world coordinates.
    NonFiniteEndpoint { entity: VectorEntity3 },
    /// The requested length is too small to represent a non-degenerate arrow at its anchor.
    UnrepresentableArrow { entity: VectorEntity3 },
    /// Storage for glyph entries could not be reserved.
    AllocationFailed { requested_items: usize },
}

impl fmt::Display for VectorGlyphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagnitudeScaleFactor { factor } => write!(
                f,
                "magnitude scale factor must be finite and non-negative; received {factor}"
            ),
            Self::InvalidUniformLength { length } => write!(
                f,
                "uniform vector display length must be finite and positive; received {length}"
            ),
            Self::DisplayLengthOverflow { entity } => {
                write!(f, "display length overflow for source entity {entity:?}")
            }
            Self::NonFiniteEndpoint { entity } => {
                write!(f, "arrow endpoint is not finite for source entity {entity:?}")
            }
            Self::UnrepresentableArrow { entity } => write!(
                f,
                "display length cannot represent a non-degenerate arrow at source entity {entity:?}"
            ),
            Self::AllocationFailed { requested_items } => write!(
                f,
                "could not allocate storage for {requested_items} vector glyph entries"
            ),
        }
    }
}

impl Error for VectorGlyphError {}

/// Converts analyzed vectors into source-ordered, f64 arrow geometry.
///
/// Each arrow contains a shaft and two head segments. The head length is 20%
/// of total display length and its half-width is 8%. A deterministic
/// perpendicular is formed by crossing the direction with the least-aligned
/// world axis, avoiding unstable choices for axis-parallel directions.
///
/// Zero and non-finite vectors retain entries without geometry. A vector
/// whose magnitude overflows f64 can still produce a uniform-length arrow,
/// because its finite direction remains available. Under magnitude scaling,
/// that vector retains its entry but has no displayable length.
pub fn build_vector_glyphs(
    analyses: &[VectorAnalysis3],
    policy: VectorScalePolicy3,
) -> Result<VectorGlyphSet3, VectorGlyphError> {
    validate_policy(policy)?;

    let mut entries = Vec::new();
    entries
        .try_reserve_exact(analyses.len())
        .map_err(|_| VectorGlyphError::AllocationFailed {
            requested_items: analyses.len(),
        })?;

    for analysis in analyses.iter().copied() {
        let Some(direction) = analysis.direction() else {
            entries.push(VectorGlyphEntry3 {
                analysis,
                segments: None,
                display_length: None,
            });
            continue;
        };

        let display_length = match policy {
            VectorScalePolicy3::MagnitudeScaled { factor } => {
                let Some(magnitude) = analysis.magnitude() else {
                    entries.push(VectorGlyphEntry3 {
                        analysis,
                        segments: None,
                        display_length: None,
                    });
                    continue;
                };
                let length = magnitude * factor;
                if !length.is_finite() {
                    return Err(VectorGlyphError::DisplayLengthOverflow {
                        entity: analysis.source().entity(),
                    });
                }
                length
            }
            VectorScalePolicy3::UniformLength { length } => length,
        };

        if display_length == 0.0 {
            entries.push(VectorGlyphEntry3 {
                analysis,
                segments: None,
                display_length: Some(0.0),
            });
            continue;
        }

        let segments = build_arrow_segments(
            analysis.source().anchor(),
            direction,
            display_length,
            analysis.source().entity(),
        )?;
        entries.push(VectorGlyphEntry3 {
            analysis,
            segments: Some(segments),
            display_length: Some(display_length),
        });
    }

    Ok(VectorGlyphSet3 { policy, entries })
}

fn validate_policy(policy: VectorScalePolicy3) -> Result<(), VectorGlyphError> {
    match policy {
        VectorScalePolicy3::MagnitudeScaled { factor }
            if !factor.is_finite() || factor < 0.0 =>
        {
            Err(VectorGlyphError::InvalidMagnitudeScaleFactor { factor })
        }
        VectorScalePolicy3::UniformLength { length }
            if !length.is_finite() || length <= 0.0 =>
        {
            Err(VectorGlyphError::InvalidUniformLength { length })
        }
        _ => Ok(()),
    }
}

fn build_arrow_segments(
    anchor: WorldPoint3,
    direction: WorldVector3,
    length: f64,
    entity: VectorEntity3,
) -> Result<[VectorGlyphSegment3; 3], VectorGlyphError> {
    const HEAD_LENGTH_FRACTION: f64 = 0.20;
    const HEAD_HALF_WIDTH_FRACTION: f64 = 0.08;

    let direction = direction.to_vector();
    let reference = least_aligned_axis(direction);
    let perpendicular = direction.cross(reference).normalize();

    let tip = anchor.to_vector() + direction * length;
    let head_base = tip - direction * (length * HEAD_LENGTH_FRACTION);
    let side = perpendicular * (length * HEAD_HALF_WIDTH_FRACTION);
    let head_left = head_base + side;
    let head_right = head_base - side;

    let endpoints = [tip, head_base, head_left, head_right];
    if endpoints.iter().any(|point| !point.is_finite()) {
        return Err(VectorGlyphError::NonFiniteEndpoint { entity });
    }

    if tip == anchor.to_vector() || head_base == tip || head_left == head_base || head_right == head_base {
        return Err(VectorGlyphError::UnrepresentableArrow { entity });
    }

    let point = |value: DVec3| WorldPoint3::from_vector(value);
    Ok([
        VectorGlyphSegment3 {
            start: anchor,
            end: point(tip),
        },
        VectorGlyphSegment3 {
            start: point(tip),
            end: point(head_left),
        },
        VectorGlyphSegment3 {
            start: point(tip),
            end: point(head_right),
        },
    ])
}

fn least_aligned_axis(direction: DVec3) -> DVec3 {
    let absolute = direction.abs();
    if absolute.x <= absolute.y && absolute.x <= absolute.z {
        DVec3::X
    } else if absolute.y <= absolute.z {
        DVec3::Y
    } else {
        DVec3::Z
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        fields::VectorValue3,
        spatial::WorldPoint3,
        vector_visualization::{analyze_vector_sources, VectorSource3},
    };

    fn analysis(
        entity: VectorEntity3,
        anchor: WorldPoint3,
        components: [f64; 3],
    ) -> VectorAnalysis3 {
        let source = VectorSource3 {
            entity,
            anchor,
            value: VectorValue3::from_f64(components[0], components[1], components[2]),
        };
        analyze_vector_sources(&[source]).unwrap()[0]
    }

    fn segment_length(segment: VectorGlyphSegment3) -> f64 {
        (segment.end() - segment.start()).length()
    }

    #[test]
    fn magnitude_scaled_policy_preserves_relative_lengths_and_metadata() {
        let analyses = [
            analysis(VectorEntity3::Point(0), WorldPoint3::ORIGIN, [3.0, 0.0, 0.0]),
            analysis(VectorEntity3::Point(1), WorldPoint3::ORIGIN, [0.0, 4.0, 0.0]),
        ];
        let policy = VectorScalePolicy3::MagnitudeScaled { factor: 0.5 };
        let glyphs = build_vector_glyphs(&analyses, policy).unwrap();

        assert_eq!(glyphs.policy(), policy);
        assert_eq!(glyphs.entries().len(), 2);
        assert_eq!(glyphs.entries()[0].display_length(), Some(1.5));
        assert_eq!(glyphs.entries()[1].display_length(), Some(2.0));
        assert_eq!(segment_length(glyphs.entries()[0].segments().unwrap()[0]), 1.5);
        assert_eq!(segment_length(glyphs.entries()[1].segments().unwrap()[0]), 2.0);
    }

    #[test]
    fn uniform_policy_supports_magnitude_overflow_and_equalizes_lengths() {
        let analyses = [
            analysis(VectorEntity3::Point(0), WorldPoint3::ORIGIN, [3.0, 0.0, 0.0]),
            analysis(VectorEntity3::Point(1), WorldPoint3::ORIGIN, [f64::MAX, f64::MAX, 0.0]),
        ];
        let policy = VectorScalePolicy3::UniformLength { length: 2.0 };
        let glyphs = build_vector_glyphs(&analyses, policy).unwrap();

        assert_eq!(glyphs.entries()[0].display_length(), Some(2.0));
        assert_eq!(glyphs.entries()[1].display_length(), Some(2.0));
        assert_eq!(segment_length(glyphs.entries()[0].segments().unwrap()[0]), 2.0);
        assert!((segment_length(glyphs.entries()[1].segments().unwrap()[0]) - 2.0).abs() < 1.0e-12);
    }

    #[test]
    fn zero_invalid_and_zero_scaled_entries_preserve_analysis_and_order() {
        let analyses = [
            analysis(VectorEntity3::Point(0), WorldPoint3::ORIGIN, [0.0, 0.0, 0.0]),
            analysis(VectorEntity3::Cell(1), WorldPoint3::ORIGIN, [f64::NAN, 1.0, 0.0]),
            analysis(VectorEntity3::Point(2), WorldPoint3::ORIGIN, [1.0, 0.0, 0.0]),
        ];
        let glyphs = build_vector_glyphs(
            &analyses,
            VectorScalePolicy3::MagnitudeScaled { factor: 0.0 },
        )
        .unwrap();

        assert_eq!(glyphs.entries().len(), 3);
        assert_eq!(glyphs.entries()[0].analysis().status(), VectorStatus3::Zero);
        assert_eq!(
            glyphs.entries()[1].analysis().status(),
            VectorStatus3::NonFiniteComponent
        );
        assert_eq!(glyphs.entries()[2].analysis().source().entity(), VectorEntity3::Point(2));
        assert_eq!(glyphs.entries()[2].display_length(), Some(0.0));
        assert!(glyphs.entries().iter().all(|entry| entry.segments().is_none()));
    }

    #[test]
    fn invalid_scale_parameters_fail_explicitly() {
        assert_eq!(
            build_vector_glyphs(
                &[],
                VectorScalePolicy3::MagnitudeScaled { factor: -1.0 }
            ),
            Err(VectorGlyphError::InvalidMagnitudeScaleFactor { factor: -1.0 })
        );
        assert_eq!(
            build_vector_glyphs(
                &[],
                VectorScalePolicy3::UniformLength { length: 0.0 }
            ),
            Err(VectorGlyphError::InvalidUniformLength { length: 0.0 })
        );
        assert!(matches!(
            build_vector_glyphs(
                &[],
                VectorScalePolicy3::UniformLength { length: f64::INFINITY }
            ),
            Err(VectorGlyphError::InvalidUniformLength { .. })
        ));
    }

    #[test]
    fn magnitude_scaled_length_overflow_is_reported() {
        let analyses = [analysis(
            VectorEntity3::Cell(3),
            WorldPoint3::ORIGIN,
            [f64::MAX / 2.0, 0.0, 0.0],
        )];
        assert_eq!(
            build_vector_glyphs(
                &analyses,
                VectorScalePolicy3::MagnitudeScaled { factor: 4.0 }
            ),
            Err(VectorGlyphError::DisplayLengthOverflow {
                entity: VectorEntity3::Cell(3)
            })
        );
    }

    #[test]
    fn arrowheads_are_finite_and_stable_for_axis_aligned_directions() {
        let directions = [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [1.0e-14, 1.0, 0.0],
            [0.0, 1.0e-14, 1.0],
        ];
        for (index, direction) in directions.into_iter().enumerate() {
            let glyphs = build_vector_glyphs(
                &[analysis(
                    VectorEntity3::Point(index),
                    WorldPoint3::ORIGIN,
                    direction,
                )],
                VectorScalePolicy3::UniformLength { length: 3.0 },
            )
            .unwrap();
            let segments = glyphs.entries()[0].segments().unwrap();
            assert!(segments.iter().all(|segment| {
                segment.start().to_array().iter().all(|value| value.is_finite())
                    && segment.end().to_array().iter().all(|value| value.is_finite())
            }));
            assert!((segment_length(segments[0]) - 3.0).abs() < 1.0e-12);
            assert!(segment_length(segments[1]) > 0.0);
            assert!(segment_length(segments[2]) > 0.0);
        }
    }

    #[test]
    fn arrow_endpoint_overflow_and_unrepresentable_lengths_are_reported() {
        let overflow = [analysis(
            VectorEntity3::Point(0),
            WorldPoint3::new(f64::MAX, 0.0, 0.0),
            [1.0, 0.0, 0.0],
        )];
        assert_eq!(
            build_vector_glyphs(
                &overflow,
                VectorScalePolicy3::UniformLength { length: f64::MAX }
            ),
            Err(VectorGlyphError::NonFiniteEndpoint {
                entity: VectorEntity3::Point(0)
            })
        );

        let too_small = [analysis(
            VectorEntity3::Point(1),
            WorldPoint3::new(1.0e100, 0.0, 0.0),
            [1.0, 0.0, 0.0],
        )];
        assert_eq!(
            build_vector_glyphs(
                &too_small,
                VectorScalePolicy3::UniformLength { length: 1.0 }
            ),
            Err(VectorGlyphError::UnrepresentableArrow {
                entity: VectorEntity3::Point(1)
            })
        );
    }
}
