//! Reusable surface representations with explicit source-dataset provenance.
//!
//! Surface vertices and triangles may be reordered, compacted, or expanded for
//! rendering. These mappings preserve the identity of the source point and
//! logical cell represented by each surface vertex and triangle.

use std::{error::Error, fmt};

use crate::{
    fields::{FieldAssociation, FieldError, ScalarField},
    sampling::SampleValue,
    geometry::{GeometryError, TriangleMesh3},
    sampling::{GridCellIndex2D, GridSampleIndex2D, SampledSurface3, StructuredGrid2D},
    unstructured::UnstructuredGrid3,
    surface_triangulation::TriangulatedSurface3,
};


/// An RGBA color in normalized linear-light channel space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorRgba {
    channels: [f64; 4],
}

impl ColorRgba {
    /// Creates a color whose four channels must be finite and within [0, 1].
    pub fn new(r: f64, g: f64, b: f64, a: f64) -> Result<Self, ColorMapError> {
        let channels = [r, g, b, a];
        for (channel, value) in channels.iter().copied().enumerate() {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(ColorMapError::InvalidColorChannel { channel, value });
            }
        }
        Ok(Self { channels })
    }

    /// Returns red, green, blue, and alpha channels in linear-light space.
    #[must_use]
    pub const fn channels(self) -> [f64; 4] {
        self.channels
    }
}

/// One normalized color-map stop.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorStop {
    position: f64,
    color: ColorRgba,
}

impl ColorStop {
    /// Creates a stop at a finite normalized position in [0, 1].
    pub fn new(position: f64, color: ColorRgba) -> Result<Self, ColorMapError> {
        if !position.is_finite() || !(0.0..=1.0).contains(&position) {
            return Err(ColorMapError::InvalidStopPosition { position });
        }
        Ok(Self { position, color })
    }

    /// Returns the normalized stop position.
    #[must_use]
    pub const fn position(self) -> f64 {
        self.position
    }

    /// Returns the stop color.
    #[must_use]
    pub const fn color(self) -> ColorRgba {
        self.color
    }
}

/// A validated continuous color map with ordered normalized stops.
#[derive(Clone, Debug, PartialEq)]
pub struct ColorMap {
    stops: Vec<ColorStop>,
}

impl ColorMap {
    /// Creates a color map with strictly increasing stops including 0 and 1.
    pub fn new(stops: Vec<ColorStop>) -> Result<Self, ColorMapError> {
        if stops.len() < 2 {
            return Err(ColorMapError::TooFewStops { actual: stops.len() });
        }
        if stops[0].position != 0.0 {
            return Err(ColorMapError::MissingStartStop);
        }
        if stops[stops.len() - 1].position != 1.0 {
            return Err(ColorMapError::MissingEndStop);
        }
        for (index, pair) in stops.windows(2).enumerate() {
            if pair[1].position <= pair[0].position {
                return Err(ColorMapError::StopsNotStrictlyIncreasing { index: index + 1 });
            }
        }
        Ok(Self { stops })
    }

    /// Creates the built-in Viridis-style sequential palette.
    ///
    /// The stop channels are linear-light values; interpolation is performed
    /// in that same space.
    #[must_use]
    pub fn viridis() -> Self {
        let stops = [
            (0.0, [0.004, 0.016, 0.090, 1.0]),
            (0.25, [0.012, 0.185, 0.280, 1.0]),
            (0.5, [0.080, 0.390, 0.290, 1.0]),
            (0.75, [0.480, 0.610, 0.130, 1.0]),
            (1.0, [0.900, 0.820, 0.045, 1.0]),
        ];
        let stops = stops
            .into_iter()
            .map(|(position, channels)| ColorStop {
                position,
                color: ColorRgba { channels },
            })
            .collect();
        Self { stops }
    }

    /// Returns the ordered palette stops.
    #[must_use]
    pub fn stops(&self) -> &[ColorStop] {
        &self.stops
    }

    /// Samples the palette at a normalized position, clamping to [0, 1].
    #[must_use]
    pub fn sample(&self, position: f64) -> ColorRgba {
        let position = position.clamp(0.0, 1.0);
        if position <= 0.0 {
            return self.stops[0].color;
        }
        if position >= 1.0 {
            return self.stops[self.stops.len() - 1].color;
        }
        let upper = self.stops.partition_point(|stop| stop.position < position);
        let left = self.stops[upper - 1];
        let right = self.stops[upper];
        let t = (position - left.position) / (right.position - left.position);
        let a = left.color.channels;
        let b = right.color.channels;
        ColorRgba {
            channels: [
                a[0] + (b[0] - a[0]) * t,
                a[1] + (b[1] - a[1]) * t,
                a[2] + (b[2] - a[2]) * t,
                a[3] + (b[3] - a[3]) * t,
            ],
        }
    }
}

/// Errors produced while constructing or sampling a color map.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColorMapError {
    /// A channel must be finite and in [0, 1].
    InvalidColorChannel { channel: usize, value: f64 },
    /// A stop position must be finite and in [0, 1].
    InvalidStopPosition { position: f64 },
    /// At least two stops are required.
    TooFewStops { actual: usize },
    /// The first stop must be at position 0.
    MissingStartStop,
    /// The final stop must be at position 1.
    MissingEndStop,
    /// Stop positions must be strictly increasing.
    StopsNotStrictlyIncreasing { index: usize },
}

impl fmt::Display for ColorMapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidColorChannel { channel, value } => write!(
                f,
                "color channel {channel} must be finite and in [0, 1]; received {value}"
            ),
            Self::InvalidStopPosition { position } => write!(
                f,
                "color stop position must be finite and in [0, 1]; received {position}"
            ),
            Self::TooFewStops { actual } => {
                write!(f, "color map requires at least two stops; received {actual}")
            }
            Self::MissingStartStop => write!(f, "color map must include a stop at 0"),
            Self::MissingEndStop => write!(f, "color map must include a stop at 1"),
            Self::StopsNotStrictlyIncreasing { index } => {
                write!(f, "color map stop {index} is not strictly increasing")
            }
        }
    }
}

impl Error for ColorMapError {}

/// A resolved finite scalar range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScalarRange {
    minimum: f64,
    maximum: f64,
}

impl ScalarRange {
    /// Creates an explicit range with finite, strictly increasing endpoints.
    pub fn new(minimum: f64, maximum: f64) -> Result<Self, ScalarRangeError> {
        if !minimum.is_finite() {
            return Err(ScalarRangeError::NonFiniteMinimum { value: minimum });
        }
        if !maximum.is_finite() {
            return Err(ScalarRangeError::NonFiniteMaximum { value: maximum });
        }
        if minimum >= maximum {
            return Err(ScalarRangeError::InvalidOrder { minimum, maximum });
        }
        Ok(Self { minimum, maximum })
    }

    /// Computes the range from every finite value in a scalar field.
    ///
    /// A constant field is valid and retains equal minimum/maximum endpoints.
    pub fn from_field(field: &ScalarField) -> Result<Self, ScalarRangeError> {
        let mut finite = field.values()
            .iter()
            .filter_map(|value| value.as_finite())
            .filter(|value| value.is_finite());
        let first = finite.next().ok_or(ScalarRangeError::NoFiniteValues)?;
        let (minimum, maximum) =
            finite.fold((first, first), |(min, max), value| (min.min(value), max.max(value)));
        Ok(Self { minimum, maximum })
    }

    /// Returns the minimum finite scalar value.
    #[must_use]
    pub const fn minimum(self) -> f64 {
        self.minimum
    }

    /// Returns the maximum finite scalar value.
    #[must_use]
    pub const fn maximum(self) -> f64 {
        self.maximum
    }

    /// Normalizes a finite value, clamping it to the range.
    ///
    /// A constant range maps every finite value to the midpoint.
    #[must_use]
    pub fn normalize(self, value: f64) -> f64 {
        if self.minimum == self.maximum {
            0.5
        } else {
            ((value - self.minimum) / (self.maximum - self.minimum)).clamp(0.0, 1.0)
        }
    }
}

/// Errors produced while resolving scalar ranges.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScalarRangeError {
    /// Automatic range resolution requires at least one finite value.
    NoFiniteValues,
    /// The explicit minimum must be finite.
    NonFiniteMinimum { value: f64 },
    /// The explicit maximum must be finite.
    NonFiniteMaximum { value: f64 },
    /// An explicit range requires minimum < maximum.
    InvalidOrder { minimum: f64, maximum: f64 },
}

impl fmt::Display for ScalarRangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoFiniteValues => write!(f, "scalar field contains no finite values"),
            Self::NonFiniteMinimum { value } => {
                write!(f, "scalar range minimum must be finite; received {value}")
            }
            Self::NonFiniteMaximum { value } => {
                write!(f, "scalar range maximum must be finite; received {value}")
            }
            Self::InvalidOrder { minimum, maximum } => write!(
                f,
                "scalar range requires minimum < maximum; received {minimum} and {maximum}"
            ),
        }
    }
}

impl Error for ScalarRangeError {}

/// Selects automatic field range resolution or a caller-supplied range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScalarRangeMode {
    /// Resolve min/max from all finite values in the source field.
    Automatic,
    /// Use this explicit finite range and clamp out-of-range values.
    Explicit(ScalarRange),
}

/// A triangle surface with per-corner colors and retained source provenance.
///
/// Vertices are expanded in triangle order: exactly three output vertices per
/// triangle. This permits discontinuous cell-associated colors at shared edges.
#[derive(Clone, Debug, PartialEq)]
pub struct MappedSurface3 {
    positions: Vec<WorldPoint3>,
    indices: Vec<u32>,
    colors: Vec<ColorRgba>,
    source_point_ids: Vec<usize>,
    source_cell_ids: Vec<usize>,
    scalar_range: ScalarRange,
}

impl MappedSurface3 {
    /// Returns expanded positions in triangle-corner order.
    #[must_use]
    pub fn positions(&self) -> &[WorldPoint3] {
        &self.positions
    }

    /// Returns sequential triangle indices.
    #[must_use]
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    /// Returns one color for each expanded triangle corner.
    #[must_use]
    pub fn colors(&self) -> &[ColorRgba] {
        &self.colors
    }

    /// Returns the source point ID represented by each expanded corner.
    #[must_use]
    pub fn source_point_ids(&self) -> &[usize] {
        &self.source_point_ids
    }

    /// Returns the source cell ID for each output triangle.
    #[must_use]
    pub fn source_cell_ids(&self) -> &[usize] {
        &self.source_cell_ids
    }

    /// Returns the resolved scalar range used for mapping.
    #[must_use]
    pub const fn scalar_range(&self) -> ScalarRange {
        self.scalar_range
    }

    /// Returns the number of output triangles.
    #[must_use]
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}

/// Maps a source scalar field onto a provenance-aware surface.
///
/// The source field is never modified. Automatic range resolution considers
/// all finite source field values, including values not represented on this
/// surface. Invalid samples receive `invalid_color`.
pub fn map_scalar_field(
    surface: &SurfaceProvenance3,
    field: &ScalarField,
    source_point_count: usize,
    source_cell_count: usize,
    range_mode: ScalarRangeMode,
    color_map: &ColorMap,
    invalid_color: ColorRgba,
) -> Result<MappedSurface3, ScalarMappingError> {
    field
        .validate_cardinality(source_point_count, source_cell_count)
        .map_err(ScalarMappingError::InvalidField)?;
    for (surface_vertex, &source_point) in surface.source_point_ids.iter().enumerate() {
        if source_point >= source_point_count {
            return Err(ScalarMappingError::InvalidProvenance(
                SurfaceProvenanceError::SourcePointOutOfBounds {
                    surface_vertex,
                    source_point,
                    source_point_count,
                },
            ));
        }
    }
    for (surface_triangle, &source_cell) in surface.source_cell_ids.iter().enumerate() {
        if source_cell >= source_cell_count {
            return Err(ScalarMappingError::InvalidProvenance(
                SurfaceProvenanceError::SourceCellOutOfBounds {
                    surface_triangle,
                    source_cell,
                    source_cell_count,
                },
            ));
        }
    }

    let range = match range_mode {
        ScalarRangeMode::Automatic => ScalarRange::from_field(field)
            .map_err(ScalarMappingError::InvalidRange)?,
        ScalarRangeMode::Explicit(range) => range,
    };

    let mut positions = Vec::with_capacity(surface.mesh.triangle_count() * 3);
    let mut indices = Vec::with_capacity(surface.mesh.indices().len());
    let mut colors = Vec::with_capacity(surface.mesh.indices().len());
    let mut source_point_ids = Vec::with_capacity(surface.mesh.indices().len());
    let mut source_cell_ids = Vec::with_capacity(surface.mesh.triangle_count());

    for (triangle_index, triangle) in surface.mesh.triangles().enumerate() {
        let source_cell = surface.source_cell_ids[triangle_index];
        for source_surface_vertex in triangle {
            let surface_vertex = source_surface_vertex as usize;
            let source_point = surface.source_point_ids[surface_vertex];
            let value_index = match field.association() {
                FieldAssociation::Point => source_point,
                FieldAssociation::Cell => source_cell,
                FieldAssociation::Dataset => 0,
            };
            let value = field.values()[value_index];
            let color = match value.as_finite().filter(|value| value.is_finite()) {
                Some(value) => color_map.sample(range.normalize(value)),
                None => invalid_color,
            };

            positions.push(surface.mesh.vertices()[surface_vertex]);
            colors.push(color);
            source_point_ids.push(source_point);
            indices.push(u32::try_from(indices.len()).map_err(|_| {
                ScalarMappingError::OutputIndexOverflow
            })?);
        }
        source_cell_ids.push(source_cell);
    }

    Ok(MappedSurface3 {
        positions,
        indices,
        colors,
        source_point_ids,
        source_cell_ids,
        scalar_range: range,
    })
}

/// Errors produced while mapping scalar fields onto surfaces.
#[derive(Clone, Debug, PartialEq)]
pub enum ScalarMappingError {
    /// Source field cardinality does not match the dataset.
    InvalidField(FieldError),
    /// Source provenance contains an ID outside the dataset.
    InvalidProvenance(SurfaceProvenanceError),
    /// Automatic range resolution found no finite values.
    InvalidRange(ScalarRangeError),
    /// Expanded output vertex indices exceed u32.
    OutputIndexOverflow,
}

impl fmt::Display for ScalarMappingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidField(error) => write!(f, "cannot map scalar field: {error}"),
            Self::InvalidProvenance(error) => write!(f, "invalid surface provenance: {error}"),
            Self::InvalidRange(error) => write!(f, "cannot resolve scalar range: {error}"),
            Self::OutputIndexOverflow => write!(f, "mapped surface exceeds u32 vertex indexing"),
        }
    }
}

impl Error for ScalarMappingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidField(error) => Some(error),
            Self::InvalidProvenance(error) => Some(error),
            Self::InvalidRange(error) => Some(error),
            Self::OutputIndexOverflow => None,
        }
    }
}

/// A triangle surface whose vertices and triangles retain source identities.
///
/// `source_point_ids[i]` identifies the source dataset point represented by
/// surface vertex `i`. `source_cell_ids[i]` identifies the source logical
/// cell represented by surface triangle `i`. IDs are zero-based and refer to
/// the original dataset ordering.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceProvenance3 {
    mesh: TriangleMesh3,
    source_point_ids: Vec<usize>,
    source_cell_ids: Vec<usize>,
}

impl SurfaceProvenance3 {
    /// Creates a provenance mapping after validating cardinality and IDs.
    pub fn new(
        mesh: TriangleMesh3,
        source_point_ids: Vec<usize>,
        source_cell_ids: Vec<usize>,
        source_point_count: usize,
        source_cell_count: usize,
    ) -> Result<Self, SurfaceProvenanceError> {
        if source_point_ids.len() != mesh.vertex_count() {
            return Err(SurfaceProvenanceError::PointMappingCountMismatch {
                expected: mesh.vertex_count(),
                actual: source_point_ids.len(),
            });
        }
        if source_cell_ids.len() != mesh.triangle_count() {
            return Err(SurfaceProvenanceError::CellMappingCountMismatch {
                expected: mesh.triangle_count(),
                actual: source_cell_ids.len(),
            });
        }

        for (surface_vertex, &source_point) in source_point_ids.iter().enumerate() {
            if source_point >= source_point_count {
                return Err(SurfaceProvenanceError::SourcePointOutOfBounds {
                    surface_vertex,
                    source_point,
                    source_point_count,
                });
            }
        }
        for (surface_triangle, &source_cell) in source_cell_ids.iter().enumerate() {
            if source_cell >= source_cell_count {
                return Err(SurfaceProvenanceError::SourceCellOutOfBounds {
                    surface_triangle,
                    source_cell,
                    source_cell_count,
                });
            }
        }

        Ok(Self {
            mesh,
            source_point_ids,
            source_cell_ids,
        })
    }

    /// Adapts a structured-grid sampled surface to source point/cell IDs.
    ///
    /// Structured samples and cells are row-major: X varies fastest within
    /// each Y row. Both generated triangles from one grid rectangle retain
    /// the same source logical cell ID.
    pub fn from_structured(
        surface: &SampledSurface3,
        grid: &StructuredGrid2D,
    ) -> Result<Self, SurfaceProvenanceError> {
        let nx = grid.x_sample_count();
        let ny = grid.y_sample_count();
        let source_point_ids = surface
            .vertex_sources()
            .iter()
            .copied()
            .map(|source| structured_point_id(source, nx, ny))
            .collect::<Result<Vec<_>, _>>()?;
        let source_cell_ids = surface
            .triangle_sources()
            .iter()
            .copied()
            .map(|source| structured_cell_id(source, nx, ny))
            .collect::<Result<Vec<_>, _>>()?;

        Self::new(
            surface.mesh().clone(),
            source_point_ids,
            source_cell_ids,
            grid.sample_count(),
            (nx - 1) * (ny - 1),
        )
    }

    /// Adapts an unstructured volume boundary to its original source IDs.
    pub fn from_unstructured(
        surface: &TriangulatedSurface3,
        grid: &UnstructuredGrid3,
    ) -> Result<Self, SurfaceProvenanceError> {
        let mut indices = Vec::with_capacity(surface.triangle_count() * 3);
        let mut source_cell_ids = Vec::with_capacity(surface.triangle_count());
        for triangle in surface.triangles() {
            indices.extend_from_slice(&triangle.vertices());
            source_cell_ids.push(triangle.source_cell_index());
        }
        let mesh = TriangleMesh3::new(surface.points().to_vec(), indices)
            .map_err(SurfaceProvenanceError::InvalidSurfaceMesh)?;

        Self::new(
            mesh,
            surface
                .source_point_ids()
                .iter()
                .map(|&id| id as usize)
                .collect(),
            source_cell_ids,
            grid.point_count(),
            grid.cell_count(),
        )
    }

    /// Adapts a triangle-mesh dataset using identity point and cell IDs.
    pub fn from_triangle_mesh(
        mesh: &TriangleMesh3,
    ) -> Result<Self, SurfaceProvenanceError> {
        Self::new(
            mesh.clone(),
            (0..mesh.vertex_count()).collect(),
            (0..mesh.triangle_count()).collect(),
            mesh.vertex_count(),
            mesh.triangle_count(),
        )
    }

    /// Returns the triangle mesh in surface-vertex order.
    #[must_use]
    pub const fn mesh(&self) -> &TriangleMesh3 {
        &self.mesh
    }

    /// Returns the source dataset point ID for each surface vertex.
    #[must_use]
    pub fn source_point_ids(&self) -> &[usize] {
        &self.source_point_ids
    }

    /// Returns the source dataset logical cell ID for each surface triangle.
    #[must_use]
    pub fn source_cell_ids(&self) -> &[usize] {
        &self.source_cell_ids
    }
}

fn structured_point_id(
    source: GridSampleIndex2D,
    nx: usize,
    ny: usize,
) -> Result<usize, SurfaceProvenanceError> {
    if source.x() >= nx || source.y() >= ny {
        return Err(SurfaceProvenanceError::StructuredPointIndexOutOfBounds {
            x: source.x(),
            y: source.y(),
            x_count: nx,
            y_count: ny,
        });
    }
    source
        .y()
        .checked_mul(nx)
        .and_then(|row| row.checked_add(source.x()))
        .ok_or(SurfaceProvenanceError::StructuredIndexOverflow)
}

fn structured_cell_id(
    source: GridCellIndex2D,
    nx: usize,
    ny: usize,
) -> Result<usize, SurfaceProvenanceError> {
    if source.x() >= nx - 1 || source.y() >= ny - 1 {
        return Err(SurfaceProvenanceError::StructuredCellIndexOutOfBounds {
            x: source.x(),
            y: source.y(),
            x_cell_count: nx - 1,
            y_cell_count: ny - 1,
        });
    }
    source
        .y()
        .checked_mul(nx - 1)
        .and_then(|row| row.checked_add(source.x()))
        .ok_or(SurfaceProvenanceError::StructuredIndexOverflow)
}

/// Errors produced when constructing a source-provenance surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceProvenanceError {
    /// There must be one source point ID per surface vertex.
    PointMappingCountMismatch { expected: usize, actual: usize },
    /// There must be one source cell ID per surface triangle.
    CellMappingCountMismatch { expected: usize, actual: usize },
    /// A surface vertex refers to a point outside the source dataset.
    SourcePointOutOfBounds {
        surface_vertex: usize,
        source_point: usize,
        source_point_count: usize,
    },
    /// A surface triangle refers to a cell outside the source dataset.
    SourceCellOutOfBounds {
        surface_triangle: usize,
        source_cell: usize,
        source_cell_count: usize,
    },
    /// A structured sample index is outside the source grid.
    StructuredPointIndexOutOfBounds {
        x: usize,
        y: usize,
        x_count: usize,
        y_count: usize,
    },
    /// A structured cell index is outside the source grid.
    StructuredCellIndexOutOfBounds {
        x: usize,
        y: usize,
        x_cell_count: usize,
        y_cell_count: usize,
    },
    /// Flattening a structured source index overflowed usize.
    StructuredIndexOverflow,
    /// The derived unstructured surface did not form a valid triangle mesh.
    InvalidSurfaceMesh(GeometryError),
}

impl fmt::Display for SurfaceProvenanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PointMappingCountMismatch { expected, actual } => write!(
                f,
                "surface requires {expected} source point IDs; received {actual}"
            ),
            Self::CellMappingCountMismatch { expected, actual } => write!(
                f,
                "surface requires {expected} source cell IDs; received {actual}"
            ),
            Self::SourcePointOutOfBounds {
                surface_vertex,
                source_point,
                source_point_count,
            } => write!(
                f,
                "surface vertex {surface_vertex} references source point {source_point}, but only {source_point_count} points exist"
            ),
            Self::SourceCellOutOfBounds {
                surface_triangle,
                source_cell,
                source_cell_count,
            } => write!(
                f,
                "surface triangle {surface_triangle} references source cell {source_cell}, but only {source_cell_count} cells exist"
            ),
            Self::StructuredPointIndexOutOfBounds {
                x,
                y,
                x_count,
                y_count,
            } => write!(
                f,
                "structured sample ({x}, {y}) is outside grid dimensions ({x_count}, {y_count})"
            ),
            Self::StructuredCellIndexOutOfBounds {
                x,
                y,
                x_cell_count,
                y_cell_count,
            } => write!(
                f,
                "structured cell ({x}, {y}) is outside cell dimensions ({x_cell_count}, {y_cell_count})"
            ),
            Self::StructuredIndexOverflow => {
                write!(f, "structured source index overflowed usize")
            }
            Self::InvalidSurfaceMesh(error) => {
                write!(f, "derived unstructured surface is invalid: {error}")
            }
        }
    }
}

impl Error for SurfaceProvenanceError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        sampling::{
            Domain1D, Domain2D, SamplingSpec2D, sample_scalar_2d,
        },
        spatial::WorldPoint3,
        unstructured::CellType3,
    };


    fn rgba(r: f64, g: f64, b: f64) -> ColorRgba {
        ColorRgba::new(r, g, b, 1.0).unwrap()
    }

    fn test_color_map() -> ColorMap {
        ColorMap::new(vec![
            ColorStop::new(0.0, rgba(0.0, 0.0, 0.0)).unwrap(),
            ColorStop::new(1.0, rgba(1.0, 1.0, 1.0)).unwrap(),
        ])
        .unwrap()
    }

    fn scalar_field(association: FieldAssociation, values: Vec<SampleValue>) -> ScalarField {
        ScalarField::new(
            FieldMetadata::new("test", None).unwrap(),
            association,
            values,
        )
        .unwrap()
    }

    #[test]
    fn automatic_range_ignores_non_finite_values() {
        let field = scalar_field(
            FieldAssociation::Point,
            vec![
                SampleValue::Finite(2.0),
                SampleValue::NonFinite(crate::sampling::NonFiniteValue::NaN),
                SampleValue::Finite(8.0),
                SampleValue::NonFinite(crate::sampling::NonFiniteValue::PositiveInfinity),
            ],
        );
        assert_eq!(ScalarRange::from_field(&field).unwrap(), ScalarRange { minimum: 2.0, maximum: 8.0 });
    }

    #[test]
    fn automatic_range_accepts_constant_field_and_normalizes_to_midpoint() {
        let field = scalar_field(FieldAssociation::Point, vec![SampleValue::Finite(4.0); 2]);
        let range = ScalarRange::from_field(&field).unwrap();
        assert_eq!(range.minimum(), 4.0);
        assert_eq!(range.maximum(), 4.0);
        assert_eq!(range.normalize(4.0), 0.5);
    }

    #[test]
    fn automatic_range_rejects_field_without_finite_values() {
        let field = scalar_field(
            FieldAssociation::Point,
            vec![SampleValue::NonFinite(crate::sampling::NonFiniteValue::NaN)],
        );
        assert_eq!(ScalarRange::from_field(&field), Err(ScalarRangeError::NoFiniteValues));
    }

    #[test]
    fn explicit_range_validates_endpoints_and_clamps_values() {
        let range = ScalarRange::new(2.0, 8.0).unwrap();
        assert_eq!(range.normalize(-10.0), 0.0);
        assert_eq!(range.normalize(10.0), 1.0);
        assert_eq!(ScalarRange::new(2.0, 2.0), Err(ScalarRangeError::InvalidOrder { minimum: 2.0, maximum: 2.0 }));
        assert!(matches!(ScalarRange::new(f64::NAN, 2.0), Err(ScalarRangeError::NonFiniteMinimum { .. })));
    }

    #[test]
    fn color_map_interpolates_and_clamps() {
        let map = test_color_map();
        assert_eq!(map.sample(-1.0), rgba(0.0, 0.0, 0.0));
        assert_eq!(map.sample(0.5), rgba(0.5, 0.5, 0.5));
        assert_eq!(map.sample(2.0), rgba(1.0, 1.0, 1.0));
    }

    #[test]
    fn color_map_rejects_missing_endpoints_and_duplicate_stops() {
        assert_eq!(
            ColorMap::new(vec![ColorStop::new(0.2, rgba(0.0, 0.0, 0.0)).unwrap(), ColorStop::new(1.0, rgba(1.0, 1.0, 1.0)).unwrap()]),
            Err(ColorMapError::MissingStartStop)
        );
        assert_eq!(
            ColorMap::new(vec![ColorStop::new(0.0, rgba(0.0, 0.0, 0.0)).unwrap(), ColorStop::new(0.0, rgba(0.5, 0.5, 0.5)).unwrap(), ColorStop::new(1.0, rgba(1.0, 1.0, 1.0)).unwrap()]),
            Err(ColorMapError::StopsNotStrictlyIncreasing { index: 1 })
        );
    }

    #[test]
    fn point_mapping_expands_triangles_and_preserves_source_ids() {
        let mesh = TriangleMesh3::new(
            vec![point(0.0, 0.0, 0.0), point(1.0, 0.0, 0.0), point(0.0, 1.0, 0.0), point(1.0, 1.0, 0.0)],
            vec![0, 1, 2, 2, 1, 3],
        ).unwrap();
        let surface = SurfaceProvenance3::from_triangle_mesh(&mesh).unwrap();
        let field = scalar_field(FieldAssociation::Point, vec![SampleValue::Finite(0.0), SampleValue::Finite(1.0), SampleValue::Finite(0.5), SampleValue::Finite(0.25)]);
        let original = field.clone();
        let mapped = map_scalar_field(&surface, &field, 4, 2, ScalarRangeMode::Automatic, &test_color_map(), rgba(1.0, 0.0, 1.0)).unwrap();

        assert_eq!(mapped.positions().len(), 6);
        assert_eq!(mapped.indices(), &[0, 1, 2, 3, 4, 5]);
        assert_eq!(mapped.source_point_ids(), &[0, 1, 2, 2, 1, 3]);
        assert_eq!(mapped.source_cell_ids(), &[0, 1]);
        assert_eq!(mapped.colors()[0], rgba(0.0, 0.0, 0.0));
        assert_eq!(mapped.colors()[1], rgba(1.0, 1.0, 1.0));
        assert_eq!(mapped.colors()[2], rgba(0.5, 0.5, 0.5));
        assert_eq!(field, original);
    }

    #[test]
    fn cell_mapping_colors_each_triangle_corner_without_adjacent_cell_bleeding() {
        let mesh = TriangleMesh3::new(
            vec![point(0.0, 0.0, 0.0), point(1.0, 0.0, 0.0), point(0.0, 1.0, 0.0), point(1.0, 1.0, 0.0)],
            vec![0, 1, 2, 2, 1, 3],
        ).unwrap();
        let surface = SurfaceProvenance3::from_triangle_mesh(&mesh).unwrap();
        let field = scalar_field(FieldAssociation::Cell, vec![SampleValue::Finite(0.0), SampleValue::Finite(1.0)]);
        let mapped = map_scalar_field(&surface, &field, 4, 2, ScalarRangeMode::Automatic, &test_color_map(), rgba(1.0, 0.0, 1.0)).unwrap();

        assert_eq!(&mapped.colors()[0..3], &[rgba(0.0, 0.0, 0.0); 3]);
        assert_eq!(&mapped.colors()[3..6], &[rgba(1.0, 1.0, 1.0); 3]);
    }

    #[test]
    fn dataset_mapping_uses_one_value_and_invalid_color() {
        let mesh = TriangleMesh3::new(
            vec![point(0.0, 0.0, 0.0), point(1.0, 0.0, 0.0), point(0.0, 1.0, 0.0)],
            vec![0, 1, 2],
        ).unwrap();
        let surface = SurfaceProvenance3::from_triangle_mesh(&mesh).unwrap();
        let field = scalar_field(FieldAssociation::Dataset, vec![SampleValue::NonFinite(crate::sampling::NonFiniteValue::NaN)]);
        let invalid = rgba(1.0, 0.0, 1.0);
        let mapped = map_scalar_field(&surface, &field, 3, 1, ScalarRangeMode::Explicit(ScalarRange::new(0.0, 1.0).unwrap()), &test_color_map(), invalid).unwrap();
        assert_eq!(mapped.colors(), &[invalid; 3]);
    }

    #[test]
    fn mapping_uses_full_field_range_not_only_visible_surface_values() {
        let mesh = TriangleMesh3::new(
            vec![point(0.0, 0.0, 0.0), point(1.0, 0.0, 0.0), point(0.0, 1.0, 0.0)],
            vec![0, 1, 2],
        ).unwrap();
        let surface = SurfaceProvenance3::from_triangle_mesh(&mesh).unwrap();
        let field = scalar_field(FieldAssociation::Point, vec![SampleValue::Finite(0.0), SampleValue::Finite(1.0), SampleValue::Finite(2.0), SampleValue::Finite(100.0)]);
        let mapped = map_scalar_field(&surface, &field, 4, 1, ScalarRangeMode::Automatic, &test_color_map(), rgba(1.0, 0.0, 1.0)).unwrap();
        assert_eq!(mapped.scalar_range().maximum(), 100.0);
        assert_eq!(mapped.colors()[2], rgba(0.02, 0.02, 0.02));
    }

    fn point(x: f64, y: f64, z: f64) -> WorldPoint3 {
        WorldPoint3::new(x, y, z)
    }

    fn structured_grid(nx: usize, ny: usize) -> StructuredGrid2D {
        let domain = Domain2D::new(
            Domain1D::new(0.0, (nx - 1) as f64).unwrap(),
            Domain1D::new(0.0, (ny - 1) as f64).unwrap(),
        );
        StructuredGrid2D::from_samples(
            sample_scalar_2d(domain, SamplingSpec2D::new(nx, ny).unwrap(), |x, y| x + y)
                .unwrap(),
        )
    }

    #[test]
    fn structured_surface_maps_row_major_source_points_and_cells() {
        let grid = structured_grid(3, 2);
        let surface = grid.to_surface_mesh().unwrap();
        let mapped = SurfaceProvenance3::from_structured(&surface, &grid).unwrap();

        assert_eq!(mapped.source_point_ids(), &[0, 1, 2, 3, 4, 5]);
        assert_eq!(mapped.source_cell_ids(), &[0, 0, 1, 1]);
    }

    #[test]
    fn triangle_mesh_uses_identity_source_mapping() {
        let mesh = TriangleMesh3::new(
            vec![point(0.0, 0.0, 0.0), point(1.0, 0.0, 0.0), point(0.0, 1.0, 0.0)],
            vec![0, 1, 2],
        )
        .unwrap();
        let mapped = SurfaceProvenance3::from_triangle_mesh(&mesh).unwrap();
        assert_eq!(mapped.source_point_ids(), &[0, 1, 2]);
        assert_eq!(mapped.source_cell_ids(), &[0]);
    }

    #[test]
    fn unstructured_surface_maps_compact_points_and_source_cells() {
        let grid = UnstructuredGrid3::new(
            vec![
                point(0.0, 0.0, 0.0),
                point(1.0, 0.0, 0.0),
                point(0.0, 1.0, 0.0),
                point(0.0, 0.0, 1.0),
            ],
            vec![CellType3::Tetrahedron4],
            vec![0, 4],
            vec![0, 1, 2, 3],
        )
        .unwrap();
        let triangulated = grid.boundary_surface().unwrap().triangulate();
        let mapped = SurfaceProvenance3::from_unstructured(&triangulated, &grid).unwrap();

        assert_eq!(mapped.source_point_ids(), &[0, 1, 2, 3]);
        assert_eq!(mapped.source_cell_ids(), &[0, 0, 0, 0]);
        assert_eq!(mapped.mesh().triangle_count(), 4);
    }

    #[test]
    fn rejects_point_mapping_cardinality_mismatch() {
        let mesh = TriangleMesh3::new(
            vec![point(0.0, 0.0, 0.0), point(1.0, 0.0, 0.0), point(0.0, 1.0, 0.0)],
            vec![0, 1, 2],
        )
        .unwrap();
        assert_eq!(
            SurfaceProvenance3::new(mesh, vec![0, 1], vec![0], 3, 1),
            Err(SurfaceProvenanceError::PointMappingCountMismatch {
                expected: 3,
                actual: 2,
            })
        );
    }

    #[test]
    fn rejects_source_ids_outside_dataset_cardinality() {
        let mesh = TriangleMesh3::new(
            vec![point(0.0, 0.0, 0.0), point(1.0, 0.0, 0.0), point(0.0, 1.0, 0.0)],
            vec![0, 1, 2],
        )
        .unwrap();
        assert_eq!(
            SurfaceProvenance3::new(mesh, vec![0, 1, 3], vec![0], 3, 1),
            Err(SurfaceProvenanceError::SourcePointOutOfBounds {
                surface_vertex: 2,
                source_point: 3,
                source_point_count: 3,
            })
        );
    }
}
