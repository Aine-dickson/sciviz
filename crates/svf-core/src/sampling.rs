//! Mathematical domains and sampling-resolution specifications.
//!
//! This module defines bounded domains, sampling resolutions, and CPU-side
//! scalar function evaluation. Domain and resolution types are independent
//! of evaluation; function results preserve every requested sample. This
//! module does not create renderable geometry. Sample counts always mean
//! points, so N points span N - 1 intervals.

use std::{error::Error, fmt};

pub mod function;
pub mod grid;
pub use function::{
    NonFiniteValue, SampleValue, ScalarSample1D, ScalarSample2D, ScalarSamples1D, ScalarSamples2D,
    sample_scalar_1d, sample_scalar_2d,
};
pub use grid::{
    GridCellIndex2D, GridSampleIndex2D, SampledSurface3, StructuredGrid2D, SurfaceGenerationError,
    SurfaceInspection3,
};

/// Identifies an axis when reporting an invalid sampling count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SamplingAxis {
    /// The single axis of a one-dimensional sampling specification.
    Single,
    /// The horizontal X axis of a two-dimensional specification.
    X,
    /// The vertical Y axis of a two-dimensional specification.
    Y,
}

/// Errors produced when constructing a bounded mathematical domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DomainError {
    /// The lower/start bound is not finite.
    NonFiniteStart,
    /// The upper/end bound is not finite.
    NonFiniteEnd,
    /// The end bound must be strictly greater than the start bound.
    NonPositiveExtent,
    /// Both bounds are finite, but subtracting them overflows f64.
    NonFiniteExtent,
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteStart => write!(f, "domain start must be finite"),
            Self::NonFiniteEnd => write!(f, "domain end must be finite"),
            Self::NonPositiveExtent => write!(f, "domain end must be greater than domain start"),
            Self::NonFiniteExtent => write!(f, "domain extent must be finite"),
        }
    }
}

impl Error for DomainError {}

/// Errors produced when constructing a sampling specification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SamplingError {
    /// At least two sample points are required to include both endpoints.
    TooFewSamples {
        axis: SamplingAxis,
        actual: usize,
        minimum: usize,
    },
    /// The requested two-dimensional sample count overflows usize.
    SampleCountOverflow {
        x_sample_count: usize,
        y_sample_count: usize,
    },
    /// Memory could not be reserved for the requested sample records.
    AllocationFailed { requested_samples: usize },
}

impl fmt::Display for SamplingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewSamples {
                axis,
                actual,
                minimum,
            } => write!(
                f,
                "sampling along {axis:?} requires at least {minimum} points; received {actual}"
            ),
            Self::SampleCountOverflow {
                x_sample_count,
                y_sample_count,
            } => write!(
                f,
                "two-dimensional sample count overflows usize ({x_sample_count} × {y_sample_count})"
            ),
            Self::AllocationFailed { requested_samples } => write!(
                f,
                "could not allocate storage for {requested_samples} samples"
            ),
        }
    }
}

impl Error for SamplingError {}

/// A finite, closed, one-dimensional mathematical domain.
///
/// The start bound is inclusive and must be strictly less than the inclusive
/// end bound. The representable extent must also be finite.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Domain1D {
    start: f64,
    end: f64,
    extent: f64,
}

impl Domain1D {
    /// Creates a domain with finite, strictly increasing bounds.
    pub fn new(start: f64, end: f64) -> Result<Self, DomainError> {
        if !start.is_finite() {
            return Err(DomainError::NonFiniteStart);
        }
        if !end.is_finite() {
            return Err(DomainError::NonFiniteEnd);
        }
        if end <= start {
            return Err(DomainError::NonPositiveExtent);
        }

        let extent = end - start;
        if !extent.is_finite() {
            return Err(DomainError::NonFiniteExtent);
        }

        Ok(Self { start, end, extent })
    }

    /// Returns the inclusive lower bound.
    #[must_use]
    pub const fn start(&self) -> f64 {
        self.start
    }

    /// Returns the inclusive upper bound.
    #[must_use]
    pub const fn end(&self) -> f64 {
        self.end
    }

    /// Returns the finite positive distance from start to end.
    #[must_use]
    pub const fn extent(&self) -> f64 {
        self.extent
    }
}

/// A rectangular two-dimensional domain represented by independent X and Y
/// ranges. Both axis ranges are inclusive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Domain2D {
    x: Domain1D,
    y: Domain1D,
}

impl Domain2D {
    /// Creates a 2D domain from already validated X and Y ranges.
    #[must_use]
    pub const fn new(x: Domain1D, y: Domain1D) -> Self {
        Self { x, y }
    }

    /// Returns the inclusive X-axis range.
    #[must_use]
    pub const fn x(&self) -> Domain1D {
        self.x
    }

    /// Returns the inclusive Y-axis range.
    #[must_use]
    pub const fn y(&self) -> Domain1D {
        self.y
    }
}

/// A one-dimensional sampling resolution.
///
/// The count is the number of sample points, not the number of intervals.
/// At least two points are required; N points define N - 1 intervals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SamplingSpec1D {
    sample_count: usize,
}

impl SamplingSpec1D {
    /// Creates a sampling specification containing at least two points.
    pub fn new(sample_count: usize) -> Result<Self, SamplingError> {
        if sample_count < 2 {
            return Err(SamplingError::TooFewSamples {
                axis: SamplingAxis::Single,
                actual: sample_count,
                minimum: 2,
            });
        }
        Ok(Self { sample_count })
    }

    /// Returns the requested number of sample points.
    #[must_use]
    pub const fn sample_count(&self) -> usize {
        self.sample_count
    }

    /// Returns the number of intervals between adjacent sample points.
    #[must_use]
    pub const fn interval_count(&self) -> usize {
        self.sample_count - 1
    }
}

/// A two-dimensional sampling resolution with independent X and Y point
/// counts. Counts are points, not intervals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SamplingSpec2D {
    x_sample_count: usize,
    y_sample_count: usize,
}

impl SamplingSpec2D {
    /// Creates a 2D sampling specification with at least two points per axis.
    pub fn new(x_sample_count: usize, y_sample_count: usize) -> Result<Self, SamplingError> {
        if x_sample_count < 2 {
            return Err(SamplingError::TooFewSamples {
                axis: SamplingAxis::X,
                actual: x_sample_count,
                minimum: 2,
            });
        }
        if y_sample_count < 2 {
            return Err(SamplingError::TooFewSamples {
                axis: SamplingAxis::Y,
                actual: y_sample_count,
                minimum: 2,
            });
        }
        Ok(Self {
            x_sample_count,
            y_sample_count,
        })
    }

    /// Returns the requested number of sample points along X.
    #[must_use]
    pub const fn x_sample_count(&self) -> usize {
        self.x_sample_count
    }

    /// Returns the requested number of sample points along Y.
    #[must_use]
    pub const fn y_sample_count(&self) -> usize {
        self.y_sample_count
    }

    /// Returns the number of intervals along X.
    #[must_use]
    pub const fn x_interval_count(&self) -> usize {
        self.x_sample_count - 1
    }

    /// Returns the number of intervals along Y.
    #[must_use]
    pub const fn y_interval_count(&self) -> usize {
        self.y_sample_count - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_1d_preserves_inclusive_bounds_and_extent() {
        let domain = Domain1D::new(-std::f64::consts::PI, std::f64::consts::PI).unwrap();

        assert_eq!(domain.start(), -std::f64::consts::PI);
        assert_eq!(domain.end(), std::f64::consts::PI);
        assert_eq!(domain.extent(), 2.0 * std::f64::consts::PI);
    }

    #[test]
    fn domain_1d_rejects_non_finite_start() {
        assert_eq!(
            Domain1D::new(f64::NAN, 1.0),
            Err(DomainError::NonFiniteStart)
        );
        assert_eq!(
            Domain1D::new(f64::NEG_INFINITY, 1.0),
            Err(DomainError::NonFiniteStart)
        );
    }

    #[test]
    fn domain_1d_rejects_non_finite_end() {
        assert_eq!(Domain1D::new(0.0, f64::NAN), Err(DomainError::NonFiniteEnd));
        assert_eq!(
            Domain1D::new(0.0, f64::INFINITY),
            Err(DomainError::NonFiniteEnd)
        );
    }

    #[test]
    fn domain_1d_rejects_equal_or_reversed_bounds() {
        assert_eq!(Domain1D::new(2.0, 2.0), Err(DomainError::NonPositiveExtent));
        assert_eq!(Domain1D::new(3.0, 2.0), Err(DomainError::NonPositiveExtent));
    }

    #[test]
    fn domain_1d_rejects_extent_that_overflows_f64() {
        assert_eq!(
            Domain1D::new(-f64::MAX, f64::MAX),
            Err(DomainError::NonFiniteExtent)
        );
    }

    #[test]
    fn domain_2d_keeps_axis_ranges_independent() {
        let x = Domain1D::new(-2.0, 2.0).unwrap();
        let y = Domain1D::new(10.0, 20.0).unwrap();
        let domain = Domain2D::new(x, y);

        assert_eq!(domain.x(), x);
        assert_eq!(domain.y(), y);
    }

    #[test]
    fn sampling_1d_uses_point_count_and_derives_intervals() {
        let spec = SamplingSpec1D::new(101).unwrap();

        assert_eq!(spec.sample_count(), 101);
        assert_eq!(spec.interval_count(), 100);
    }

    #[test]
    fn sampling_1d_accepts_minimum_two_points() {
        let spec = SamplingSpec1D::new(2).unwrap();

        assert_eq!(spec.sample_count(), 2);
        assert_eq!(spec.interval_count(), 1);
    }

    #[test]
    fn sampling_1d_rejects_zero_or_one_point() {
        for count in [0, 1] {
            assert_eq!(
                SamplingSpec1D::new(count),
                Err(SamplingError::TooFewSamples {
                    axis: SamplingAxis::Single,
                    actual: count,
                    minimum: 2,
                })
            );
        }
    }

    #[test]
    fn sampling_2d_supports_independent_axis_resolutions() {
        let spec = SamplingSpec2D::new(101, 51).unwrap();

        assert_eq!(spec.x_sample_count(), 101);
        assert_eq!(spec.y_sample_count(), 51);
        assert_eq!(spec.x_interval_count(), 100);
        assert_eq!(spec.y_interval_count(), 50);
    }

    #[test]
    fn sampling_2d_rejects_invalid_x_count() {
        assert_eq!(
            SamplingSpec2D::new(1, 10),
            Err(SamplingError::TooFewSamples {
                axis: SamplingAxis::X,
                actual: 1,
                minimum: 2,
            })
        );
    }

    #[test]
    fn sampling_2d_rejects_invalid_y_count() {
        assert_eq!(
            SamplingSpec2D::new(10, 0),
            Err(SamplingError::TooFewSamples {
                axis: SamplingAxis::Y,
                actual: 0,
                minimum: 2,
            })
        );
    }
}
