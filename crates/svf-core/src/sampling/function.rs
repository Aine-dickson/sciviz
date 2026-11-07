//! CPU-side evaluation of scalar functions over validated sampling domains.
//!
//! Results preserve every requested sample and its source coordinates. A
//! non-finite function result is represented explicitly rather than omitted.
//! Two-dimensional samples are stored in row-major order: Y selects the row
//! and X varies fastest within each row.

use super::{Domain1D, Domain2D, SamplingError, SamplingSpec1D, SamplingSpec2D};

/// The classification of a non-finite scalar function result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NonFiniteValue {
    /// The function returned NaN.
    NaN,
    /// The function returned positive infinity.
    PositiveInfinity,
    /// The function returned negative infinity.
    NegativeInfinity,
}

/// A scalar function result that distinguishes finite values from non-finite
/// results without losing the sample that produced them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SampleValue {
    /// A finite scalar result.
    Finite(f64),
    /// A non-finite scalar result.
    NonFinite(NonFiniteValue),
}

impl SampleValue {
    fn from_f64(value: f64) -> Self {
        if value.is_nan() {
            Self::NonFinite(NonFiniteValue::NaN)
        } else if value == f64::INFINITY {
            Self::NonFinite(NonFiniteValue::PositiveInfinity)
        } else if value == f64::NEG_INFINITY {
            Self::NonFinite(NonFiniteValue::NegativeInfinity)
        } else {
            Self::Finite(value)
        }
    }

    /// Returns the finite value, or None for NaN and infinities.
    #[must_use]
    pub const fn as_finite(self) -> Option<f64> {
        match self {
            Self::Finite(value) => Some(value),
            Self::NonFinite(_) => None,
        }
    }

    /// Returns whether this result is finite.
    #[must_use]
    pub const fn is_finite(self) -> bool {
        matches!(self, Self::Finite(_))
    }
}

/// One evaluated sample of a one-dimensional scalar function.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScalarSample1D {
    index: usize,
    coordinate: f64,
    value: SampleValue,
}

impl ScalarSample1D {
    /// Returns this sample's zero-based position in the requested sequence.
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the coordinate at which the function was evaluated.
    #[must_use]
    pub const fn coordinate(&self) -> f64 {
        self.coordinate
    }

    /// Returns the function result, including its finite/non-finite status.
    #[must_use]
    pub const fn value(&self) -> SampleValue {
        self.value
    }
}

/// The complete ordered result of a one-dimensional scalar sampling run.
#[derive(Clone, Debug, PartialEq)]
pub struct ScalarSamples1D {
    samples: Vec<ScalarSample1D>,
}

impl ScalarSamples1D {
    /// Returns the number of requested samples.
    #[must_use]
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Returns whether the result contains no samples.
    ///
    /// A valid sampling specification always requests at least two samples.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Returns all samples in increasing coordinate order.
    #[must_use]
    pub fn as_slice(&self) -> &[ScalarSample1D] {
        &self.samples
    }

    /// Returns a sample by its zero-based sequence index.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&ScalarSample1D> {
        self.samples.get(index)
    }
}

/// One evaluated sample of a two-dimensional scalar function.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScalarSample2D {
    x_index: usize,
    y_index: usize,
    x: f64,
    y: f64,
    value: SampleValue,
}

impl ScalarSample2D {
    /// Returns this sample's zero-based column index.
    #[must_use]
    pub const fn x_index(&self) -> usize {
        self.x_index
    }

    /// Returns this sample's zero-based row index.
    #[must_use]
    pub const fn y_index(&self) -> usize {
        self.y_index
    }

    /// Returns the X coordinate at which the function was evaluated.
    #[must_use]
    pub const fn x(&self) -> f64 {
        self.x
    }

    /// Returns the Y coordinate at which the function was evaluated.
    #[must_use]
    pub const fn y(&self) -> f64 {
        self.y
    }

    /// Returns the function result, including its finite/non-finite status.
    #[must_use]
    pub const fn value(&self) -> SampleValue {
        self.value
    }
}

/// The complete result of a two-dimensional scalar sampling run.
///
/// Samples are stored in row-major order: all X positions for y_index == 0
/// come first, followed by the next Y row. X varies fastest.
#[derive(Clone, Debug, PartialEq)]
pub struct ScalarSamples2D {
    x_sample_count: usize,
    y_sample_count: usize,
    samples: Vec<ScalarSample2D>,
}

impl ScalarSamples2D {
    /// Returns the number of sample columns along X.
    #[must_use]
    pub const fn x_sample_count(&self) -> usize {
        self.x_sample_count
    }

    /// Returns the number of sample rows along Y.
    #[must_use]
    pub const fn y_sample_count(&self) -> usize {
        self.y_sample_count
    }

    /// Returns the total number of evaluated samples.
    #[must_use]
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Returns whether the result contains no samples.
    ///
    /// A valid two-dimensional sampling specification always requests at
    /// least two samples on each axis.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Returns all samples in row-major order.
    #[must_use]
    pub fn as_slice(&self) -> &[ScalarSample2D] {
        &self.samples
    }

    /// Returns the sample at a zero-based column and row, if in bounds.
    #[must_use]
    pub fn get(&self, x_index: usize, y_index: usize) -> Option<&ScalarSample2D> {
        if x_index >= self.x_sample_count || y_index >= self.y_sample_count {
            return None;
        }
        self.samples.get(y_index * self.x_sample_count + x_index)
    }
}

/// Evaluates a scalar function at every point in a one-dimensional domain.
///
/// The first and last evaluations use the exact domain bounds. Intermediate
/// coordinates are evenly distributed over the domain. Every requested
/// sample is retained, including samples whose function result is non-finite.
pub fn sample_scalar_1d<F>(
    domain: Domain1D,
    spec: SamplingSpec1D,
    mut function: F,
) -> Result<ScalarSamples1D, SamplingError>
where
    F: FnMut(f64) -> f64,
{
    let sample_count = spec.sample_count();
    let mut samples = Vec::new();
    samples
        .try_reserve_exact(sample_count)
        .map_err(|_| SamplingError::AllocationFailed {
            requested_samples: sample_count,
        })?;

    for index in 0..sample_count {
        let coordinate = coordinate_at(domain, index, sample_count);
        let value = SampleValue::from_f64(function(coordinate));
        samples.push(ScalarSample1D {
            index,
            coordinate,
            value,
        });
    }

    Ok(ScalarSamples1D { samples })
}

/// Evaluates a scalar function at every point in a two-dimensional domain.
///
/// Rows are traversed in increasing Y order and X varies fastest within each
/// row. The returned collection retains both grid indices and source
/// coordinates for every result. Non-finite function outputs remain present
/// as explicit non-finite SampleValue entries.
pub fn sample_scalar_2d<F>(
    domain: Domain2D,
    spec: SamplingSpec2D,
    mut function: F,
) -> Result<ScalarSamples2D, SamplingError>
where
    F: FnMut(f64, f64) -> f64,
{
    let x_sample_count = spec.x_sample_count();
    let y_sample_count = spec.y_sample_count();
    let total_samples =
        x_sample_count
            .checked_mul(y_sample_count)
            .ok_or(SamplingError::SampleCountOverflow {
                x_sample_count,
                y_sample_count,
            })?;

    let mut samples = Vec::new();
    samples
        .try_reserve_exact(total_samples)
        .map_err(|_| SamplingError::AllocationFailed {
            requested_samples: total_samples,
        })?;

    for y_index in 0..y_sample_count {
        let y = coordinate_at(domain.y(), y_index, y_sample_count);
        for x_index in 0..x_sample_count {
            let x = coordinate_at(domain.x(), x_index, x_sample_count);
            let value = SampleValue::from_f64(function(x, y));
            samples.push(ScalarSample2D {
                x_index,
                y_index,
                x,
                y,
                value,
            });
        }
    }

    Ok(ScalarSamples2D {
        x_sample_count,
        y_sample_count,
        samples,
    })
}

fn coordinate_at(domain: Domain1D, index: usize, sample_count: usize) -> f64 {
    if index == 0 {
        return domain.start();
    }
    if index + 1 == sample_count {
        return domain.end();
    }

    let interval_count = sample_count - 1;
    let fraction = index as f64 / interval_count as f64;
    domain.start() + domain.extent() * fraction
}

#[cfg(test)]
mod tests {
    use super::*;

    fn domain_1d() -> Domain1D {
        Domain1D::new(-1.0, 1.0).unwrap()
    }

    fn domain_2d() -> Domain2D {
        Domain2D::new(
            Domain1D::new(-1.0, 1.0).unwrap(),
            Domain1D::new(10.0, 20.0).unwrap(),
        )
    }

    #[test]
    fn samples_1d_include_exact_endpoints_and_ordered_coordinates() {
        let samples =
            sample_scalar_1d(domain_1d(), SamplingSpec1D::new(5).unwrap(), |x| x).unwrap();
        assert_eq!(samples.len(), 5);
        assert_eq!(samples.get(0).unwrap().coordinate(), -1.0);
        assert_eq!(samples.get(1).unwrap().coordinate(), -0.5);
        assert_eq!(samples.get(2).unwrap().coordinate(), 0.0);
        assert_eq!(samples.get(3).unwrap().coordinate(), 0.5);
        assert_eq!(samples.get(4).unwrap().coordinate(), 1.0);
        assert_eq!(samples.get(4).unwrap().index(), 4);
    }

    #[test]
    fn samples_1d_preserve_finite_function_values() {
        let samples = sample_scalar_1d(domain_1d(), SamplingSpec1D::new(3).unwrap(), |x| {
            x * x + 2.0
        })
        .unwrap();
        assert_eq!(samples.as_slice()[0].value(), SampleValue::Finite(3.0));
        assert_eq!(samples.as_slice()[1].value(), SampleValue::Finite(2.0));
        assert_eq!(samples.as_slice()[2].value(), SampleValue::Finite(3.0));
    }

    #[test]
    fn samples_1d_retain_and_classify_non_finite_results() {
        let samples = sample_scalar_1d(domain_1d(), SamplingSpec1D::new(3).unwrap(), |x| {
            if x < 0.0 {
                f64::NAN
            } else if x == 0.0 {
                f64::INFINITY
            } else {
                f64::NEG_INFINITY
            }
        })
        .unwrap();
        assert_eq!(samples.len(), 3);
        assert_eq!(
            samples.as_slice()[0].value(),
            SampleValue::NonFinite(NonFiniteValue::NaN)
        );
        assert_eq!(
            samples.as_slice()[1].value(),
            SampleValue::NonFinite(NonFiniteValue::PositiveInfinity)
        );
        assert_eq!(
            samples.as_slice()[2].value(),
            SampleValue::NonFinite(NonFiniteValue::NegativeInfinity)
        );
        assert!(
            samples
                .as_slice()
                .iter()
                .all(|sample| !sample.value().is_finite())
        );
    }

    #[test]
    fn samples_2d_are_row_major_with_x_varying_fastest() {
        let samples = sample_scalar_2d(domain_2d(), SamplingSpec2D::new(3, 2).unwrap(), |x, y| {
            x + y
        })
        .unwrap();
        assert_eq!(samples.len(), 6);
        assert_eq!((samples.x_sample_count(), samples.y_sample_count()), (3, 2));
        let ordered: Vec<_> = samples
            .as_slice()
            .iter()
            .map(|s| (s.x_index(), s.y_index(), s.x(), s.y()))
            .collect();
        assert_eq!(
            ordered,
            vec![
                (0, 0, -1.0, 10.0),
                (1, 0, 0.0, 10.0),
                (2, 0, 1.0, 10.0),
                (0, 1, -1.0, 20.0),
                (1, 1, 0.0, 20.0),
                (2, 1, 1.0, 20.0),
            ]
        );
    }

    #[test]
    fn samples_2d_allow_random_access_by_grid_indices() {
        let samples = sample_scalar_2d(domain_2d(), SamplingSpec2D::new(3, 2).unwrap(), |x, y| {
            x * y
        })
        .unwrap();
        let sample = samples.get(2, 1).unwrap();
        assert_eq!(sample.x(), 1.0);
        assert_eq!(sample.y(), 20.0);
        assert_eq!(sample.value(), SampleValue::Finite(20.0));
        assert!(samples.get(3, 1).is_none());
        assert!(samples.get(0, 2).is_none());
    }

    #[test]
    fn samples_2d_retain_non_finite_results_without_shifting_indices() {
        let samples = sample_scalar_2d(domain_2d(), SamplingSpec2D::new(2, 2).unwrap(), |x, y| {
            if x == 1.0 && y == 20.0 {
                f64::NAN
            } else {
                x + y
            }
        })
        .unwrap();
        assert_eq!(samples.len(), 4);
        assert_eq!(
            samples.get(1, 1).unwrap().value(),
            SampleValue::NonFinite(NonFiniteValue::NaN)
        );
        assert_eq!(
            samples.get(0, 1).unwrap().value(),
            SampleValue::Finite(19.0)
        );
    }

    #[test]
    fn sample_value_exposes_finite_values_without_coercing_non_finite_results() {
        assert_eq!(SampleValue::Finite(4.5).as_finite(), Some(4.5));
        assert_eq!(
            SampleValue::NonFinite(NonFiniteValue::PositiveInfinity).as_finite(),
            None
        );
        assert!(SampleValue::Finite(0.0).is_finite());
        assert!(!SampleValue::NonFinite(NonFiniteValue::NaN).is_finite());
    }

    #[test]
    fn two_dimensional_sample_count_overflow_is_reported() {
        let spec = SamplingSpec2D::new(usize::MAX, 2).unwrap();
        let result = sample_scalar_2d(domain_2d(), spec, |x, y| x + y);
        assert_eq!(
            result,
            Err(SamplingError::SampleCountOverflow {
                x_sample_count: usize::MAX,
                y_sample_count: 2,
            })
        );
    }
}
