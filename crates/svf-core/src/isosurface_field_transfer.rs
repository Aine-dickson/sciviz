//! Provenance-based transfer of point scalars onto derived isosurfaces.
//!
//! Transferred values remain associated with the derived mesh vertices. They
//! are never attached to, or represented as values of, source dataset points.

use std::{error::Error, fmt};

use crate::{
    dataset::{Dataset, DatasetGeometry},
    fields::{FieldAssociation, FieldError, FieldMetadata},
    isosurface::{Isosurface3, IsosurfaceVertexProvenance3},
    sampling::{NonFiniteValue, SampleValue},
    scalar_visualization::{
        ColorMap, ColorRgba, ScalarRange, ScalarRangeError, ScalarRangeMode,
    },
};

/// Scalar values transferred into the exact vertex order of an isosurface.
#[derive(Clone, Debug, PartialEq)]
pub struct TransferredScalarField3 {
    isosurface: Isosurface3,
    metadata: FieldMetadata,
    values: Vec<SampleValue>,
}

impl TransferredScalarField3 {
    /// Returns the owned derived isosurface to which these values belong.
    #[must_use]
    pub fn isosurface(&self) -> &Isosurface3 { &self.isosurface }

    /// Returns the source field's descriptive metadata.
    #[must_use]
    pub const fn metadata(&self) -> &FieldMetadata { &self.metadata }

    /// Returns values in exact isosurface mesh vertex order.
    #[must_use]
    pub fn values(&self) -> &[SampleValue] { &self.values }

    /// Returns the number of transferred vertex values.
    #[must_use]
    pub fn len(&self) -> usize { self.values.len() }

    /// Returns whether the derived surface has no vertices.
    #[must_use]
    pub fn is_empty(&self) -> bool { self.values.is_empty() }
}

/// A uniquely indexed isosurface with one mapped color per derived vertex.
#[derive(Clone, Debug, PartialEq)]
pub struct MappedIsosurface3 {
    transferred: TransferredScalarField3,
    colors: Vec<ColorRgba>,
    scalar_range: ScalarRange,
}

impl MappedIsosurface3 {
    /// Returns the transferred field and its owned isosurface/provenance.
    #[must_use]
    pub fn transferred(&self) -> &TransferredScalarField3 { &self.transferred }

    /// Returns the derived isosurface.
    #[must_use]
    pub fn isosurface(&self) -> &Isosurface3 { self.transferred.isosurface() }

    /// Returns one color for each unique isosurface vertex.
    #[must_use]
    pub fn colors(&self) -> &[ColorRgba] { &self.colors }

    /// Returns the resolved scalar range used for color mapping.
    #[must_use]
    pub const fn scalar_range(&self) -> ScalarRange { self.scalar_range }
}

/// Transfers a point-associated scalar field through M8.1 vertex provenance.
///
/// The caller must provide the same source dataset used for extraction.
/// M8.1 provenance does not carry a dataset identity, so this function
/// validates topology cardinalities and every referenced ID but cannot
/// distinguish two different datasets with identical cardinalities.
pub fn transfer_point_scalar_field(
    dataset: &Dataset,
    isosurface: &Isosurface3,
    field_name: &str,
) -> Result<TransferredScalarField3, ScalarTransferError3> {
    let grid = match dataset.geometry() {
        DatasetGeometry::UnstructuredGrid3(grid) => grid,
        _ => return Err(ScalarTransferError3::DatasetGeometryMustBeUnstructured),
    };
    let field = dataset.scalar_field(field_name)
        .ok_or_else(|| ScalarTransferError3::ScalarFieldNotFound { name: field_name.to_owned() })?;
    if field.association() != FieldAssociation::Point {
        return Err(ScalarTransferError3::ScalarFieldMustBePointAssociated {
            actual: field.association(),
        });
    }
    field.validate_cardinality(grid.point_count(), grid.cell_count())
        .map_err(ScalarTransferError3::InvalidFieldCardinality)?;

    if isosurface.vertex_provenance().len() != isosurface.mesh().vertex_count() {
        return Err(ScalarTransferError3::VertexProvenanceCountMismatch {
            expected: isosurface.mesh().vertex_count(),
            actual: isosurface.vertex_provenance().len(),
        });
    }
    if isosurface.triangle_source_cells().len() != isosurface.mesh().triangle_count() {
        return Err(ScalarTransferError3::TriangleSourceCellCountMismatch {
            expected: isosurface.mesh().triangle_count(),
            actual: isosurface.triangle_source_cells().len(),
        });
    }
    for (triangle, &cell_id) in isosurface.triangle_source_cells().iter().enumerate() {
        if cell_id >= grid.cell_count() {
            return Err(ScalarTransferError3::SourceCellOutOfBounds {
                triangle, cell_id, source_cell_count: grid.cell_count(),
            });
        }
    }

    let mut values = Vec::with_capacity(isosurface.mesh().vertex_count());
    for (vertex, provenance) in isosurface.vertex_provenance().iter().copied().enumerate() {
        let value = match provenance {
            IsosurfaceVertexProvenance3::SourcePoint { point_id } => {
                let point = point_id as usize;
                if point >= grid.point_count() {
                    return Err(ScalarTransferError3::SourcePointOutOfBounds {
                        vertex, point_id, source_point_count: grid.point_count(),
                    });
                }
                field.values()[point]
            }
            IsosurfaceVertexProvenance3::SourceEdge { point_ids, t } => {
                let [low, high] = point_ids;
                if low >= high {
                    return Err(ScalarTransferError3::NonCanonicalEdge {
                        vertex, point_ids,
                    });
                }
                if !t.is_finite() || !(0.0..=1.0).contains(&t) {
                    return Err(ScalarTransferError3::InvalidInterpolationParameter {
                        vertex, t,
                    });
                }
                if high as usize >= grid.point_count() {
                    return Err(ScalarTransferError3::SourcePointOutOfBounds {
                        vertex, point_id: high, source_point_count: grid.point_count(),
                    });
                }
                interpolate(field.values()[low as usize], field.values()[high as usize], t)
                    .map_err(|_| ScalarTransferError3::NumericalInterpolation { vertex })?
            }
        };
        values.push(value);
    }

    Ok(TransferredScalarField3 {
        isosurface: isosurface.clone(),
        metadata: field.metadata().clone(),
        values,
    })
}

/// Maps transferred values to colors while retaining unique indexed topology.
pub fn map_transferred_scalar_field(
    transferred: TransferredScalarField3,
    range_mode: ScalarRangeMode,
    color_map: &ColorMap,
    invalid_color: ColorRgba,
) -> Result<MappedIsosurface3, ScalarTransferError3> {
    if transferred.values.len() != transferred.isosurface.mesh().vertex_count() {
        return Err(ScalarTransferError3::TransferredValueCountMismatch {
            expected: transferred.isosurface.mesh().vertex_count(),
            actual: transferred.values.len(),
        });
    }
    let range = match range_mode {
        ScalarRangeMode::Automatic => ScalarRange::from_values(&transferred.values)
            .map_err(ScalarTransferError3::InvalidRange)?,
        ScalarRangeMode::Explicit(range) => range,
    };
    let colors = transferred.values.iter().copied().map(|sample| {
        match sample.as_finite().filter(|value| value.is_finite()) {
            Some(value) => color_map.sample(range.normalize(value)),
            None => invalid_color,
        }
    }).collect();

    Ok(MappedIsosurface3 { transferred, colors, scalar_range: range })
}

fn interpolate(a: SampleValue, b: SampleValue, t: f64) -> Result<SampleValue, ()> {
    if t == 0.0 { return Ok(a); }
    if t == 1.0 { return Ok(b); }

    let result = match (a, b) {
        (SampleValue::Finite(a), SampleValue::Finite(b)) => {
            let value = if a.is_sign_positive() == b.is_sign_positive() {
                a + (b - a) * t
            } else {
                a * (1.0 - t) + b * t
            };
            if !value.is_finite() { return Err(()); }
            SampleValue::Finite(value)
        }
        (SampleValue::NonFinite(NonFiniteValue::NaN), _)
        | (_, SampleValue::NonFinite(NonFiniteValue::NaN)) => {
            SampleValue::NonFinite(NonFiniteValue::NaN)
        }
        (SampleValue::NonFinite(NonFiniteValue::PositiveInfinity),
         SampleValue::NonFinite(NonFiniteValue::NegativeInfinity))
        | (SampleValue::NonFinite(NonFiniteValue::NegativeInfinity),
           SampleValue::NonFinite(NonFiniteValue::PositiveInfinity)) => {
            SampleValue::NonFinite(NonFiniteValue::NaN)
        }
        (SampleValue::NonFinite(value), SampleValue::Finite(_))
        | (SampleValue::Finite(_), SampleValue::NonFinite(value))
        | (SampleValue::NonFinite(value), SampleValue::NonFinite(_)) => {
            SampleValue::NonFinite(value)
        }
    };
    Ok(result)
}

/// Errors produced while transferring or mapping scalar data on an isosurface.
#[derive(Clone, Debug, PartialEq)]
pub enum ScalarTransferError3 {
    DatasetGeometryMustBeUnstructured,
    ScalarFieldNotFound { name: String },
    ScalarFieldMustBePointAssociated { actual: FieldAssociation },
    InvalidFieldCardinality(FieldError),
    VertexProvenanceCountMismatch { expected: usize, actual: usize },
    TriangleSourceCellCountMismatch { expected: usize, actual: usize },
    SourcePointOutOfBounds { vertex: usize, point_id: u32, source_point_count: usize },
    SourceCellOutOfBounds { triangle: usize, cell_id: usize, source_cell_count: usize },
    NonCanonicalEdge { vertex: usize, point_ids: [u32; 2] },
    InvalidInterpolationParameter { vertex: usize, t: f64 },
    NumericalInterpolation { vertex: usize },
    TransferredValueCountMismatch { expected: usize, actual: usize },
    InvalidRange(ScalarRangeError),
}

impl fmt::Display for ScalarTransferError3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DatasetGeometryMustBeUnstructured => write!(f, "scalar transfer requires UnstructuredGrid3 geometry"),
            Self::ScalarFieldNotFound { name } => write!(f, "dataset has no scalar field named '{name}'"),
            Self::ScalarFieldMustBePointAssociated { actual } => write!(f, "scalar transfer requires a point-associated field; received {actual:?}"),
            Self::InvalidFieldCardinality(error) => write!(f, "invalid source scalar field: {error}"),
            Self::VertexProvenanceCountMismatch { expected, actual } => write!(f, "isosurface has {actual} provenance entries for {expected} vertices"),
            Self::TriangleSourceCellCountMismatch { expected, actual } => write!(f, "isosurface has {actual} source-cell entries for {expected} triangles"),
            Self::SourcePointOutOfBounds { vertex, point_id, source_point_count } => write!(f, "isosurface vertex {vertex} references source point {point_id}, but dataset has {source_point_count} points"),
            Self::SourceCellOutOfBounds { triangle, cell_id, source_cell_count } => write!(f, "isosurface triangle {triangle} references source cell {cell_id}, but dataset has {source_cell_count} cells"),
            Self::NonCanonicalEdge { vertex, point_ids } => write!(f, "isosurface vertex {vertex} has noncanonical source edge [{}, {}]", point_ids[0], point_ids[1]),
            Self::InvalidInterpolationParameter { vertex, t } => write!(f, "isosurface vertex {vertex} has invalid edge parameter {t}"),
            Self::NumericalInterpolation { vertex } => write!(f, "finite endpoint interpolation became non-finite at isosurface vertex {vertex}"),
            Self::TransferredValueCountMismatch { expected, actual } => write!(f, "transferred field has {actual} values for {expected} isosurface vertices"),
            Self::InvalidRange(error) => write!(f, "cannot resolve transferred scalar range: {error}"),
        }
    }
}

impl Error for ScalarTransferError3 {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidFieldCardinality(error) => Some(error),
            Self::InvalidRange(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dataset::Dataset,
        fields::{FieldMetadata, ScalarField},
        spatial::WorldPoint3,
        isosurface::extract_tetrahedral_isosurface,
        sampling::SampleValue,
        unstructured::{CellType3, UnstructuredGrid3},
    };

    fn fixture(values: Vec<SampleValue>) -> (Dataset, Isosurface3) {
        let grid = UnstructuredGrid3::new(
            vec![
                WorldPoint3::new(0.0, 0.0, 0.0),
                WorldPoint3::new(1.0, 0.0, 0.0),
                WorldPoint3::new(0.0, 1.0, 0.0),
                WorldPoint3::new(0.0, 0.0, 1.0),
            ],
            vec![CellType3::Tetrahedron4],
            vec![0, 4],
            vec![0, 1, 2, 3],
        ).unwrap();
        let mut dataset = Dataset::new(grid);
        dataset.add_scalar_field(ScalarField::new(
            FieldMetadata::new("extract", None).unwrap(),
            FieldAssociation::Point,
            vec![SampleValue::Finite(-1.0), SampleValue::Finite(1.0),
                 SampleValue::Finite(-1.0), SampleValue::Finite(1.0)],
        ).unwrap()).unwrap();
        dataset.add_scalar_field(ScalarField::new(
            FieldMetadata::new("transferred", Some("units".to_owned())).unwrap(),
            FieldAssociation::Point,
            values,
        ).unwrap()).unwrap();
        let surface = extract_tetrahedral_isosurface(&dataset, "extract", 0.0).unwrap();
        (dataset, surface)
    }

    #[test]
    fn transfers_source_points_and_interpolates_source_edges_in_mesh_order() {
        let source = vec![
            SampleValue::Finite(2.0), SampleValue::Finite(6.0),
            SampleValue::Finite(10.0), SampleValue::Finite(14.0),
        ];
        let (dataset, surface) = fixture(source.clone());
        let transferred = transfer_point_scalar_field(&dataset, &surface, "transferred").unwrap();
        assert_eq!(transferred.values().len(), surface.mesh().vertex_count());
        for (value, provenance) in transferred.values().iter().zip(surface.vertex_provenance()) {
            match provenance {
                IsosurfaceVertexProvenance3::SourcePoint { point_id } =>
                    assert_eq!(*value, source[*point_id as usize]),
                IsosurfaceVertexProvenance3::SourceEdge { point_ids, t } => {
                    let a = source[point_ids[0] as usize].as_finite().unwrap();
                    let b = source[point_ids[1] as usize].as_finite().unwrap();
                    assert_eq!(*value, SampleValue::Finite(a + (b - a) * t));
                }
            }
        }
        assert_eq!(transferred.metadata().name(), "transferred");
        assert_eq!(transferred.metadata().units(), Some("units"));
    }

    #[test]
    fn interpolation_preserves_endpoints_and_non_finite_classification() {
        use NonFiniteValue::{NaN, NegativeInfinity, PositiveInfinity};
        assert_eq!(interpolate(SampleValue::Finite(3.0), SampleValue::NonFinite(NaN), 0.0).unwrap(), SampleValue::Finite(3.0));
        assert_eq!(interpolate(SampleValue::NonFinite(PositiveInfinity), SampleValue::Finite(3.0), 1.0).unwrap(), SampleValue::Finite(3.0));
        assert_eq!(interpolate(SampleValue::Finite(1.0), SampleValue::Finite(5.0), 0.5).unwrap(), SampleValue::Finite(3.0));
        assert_eq!(interpolate(SampleValue::Finite(-f64::MAX), SampleValue::Finite(f64::MAX), 0.5).unwrap(), SampleValue::Finite(0.0));
        assert_eq!(interpolate(SampleValue::NonFinite(PositiveInfinity), SampleValue::Finite(1.0), 0.5).unwrap(), SampleValue::NonFinite(PositiveInfinity));
        assert_eq!(interpolate(SampleValue::NonFinite(PositiveInfinity), SampleValue::NonFinite(PositiveInfinity), 0.5).unwrap(), SampleValue::NonFinite(PositiveInfinity));
        assert_eq!(interpolate(SampleValue::NonFinite(PositiveInfinity), SampleValue::NonFinite(NegativeInfinity), 0.5).unwrap(), SampleValue::NonFinite(NaN));
    }

    #[test]
    fn mapped_isosurface_uses_unique_vertices_and_retains_topology_and_provenance() {
        let (dataset, surface) = fixture(vec![
            SampleValue::Finite(0.0), SampleValue::Finite(2.0),
            SampleValue::Finite(4.0), SampleValue::Finite(6.0),
        ]);
        let transferred = transfer_point_scalar_field(&dataset, &surface, "transferred").unwrap();
        let mapped = map_transferred_scalar_field(
            transferred, ScalarRangeMode::Automatic, &ColorMap::viridis(),
            ColorRgba::new(1.0, 0.0, 1.0, 1.0).unwrap(),
        ).unwrap();
        assert_eq!(mapped.colors().len(), surface.mesh().vertex_count());
        assert_eq!(mapped.isosurface().mesh().indices(), surface.mesh().indices());
        assert_eq!(mapped.isosurface().vertex_provenance(), surface.vertex_provenance());
        assert_eq!(mapped.scalar_range().minimum(), 1.0);
        assert_eq!(mapped.scalar_range().maximum(), 5.0);
    }

    #[test]
    fn automatic_range_rejects_all_invalid_but_explicit_range_maps_invalid_color() {
        let invalid = SampleValue::NonFinite(NonFiniteValue::NaN);
        let (dataset, surface) = fixture(vec![invalid; 4]);
        let transferred = transfer_point_scalar_field(&dataset, &surface, "transferred").unwrap();
        assert!(matches!(
            map_transferred_scalar_field(transferred.clone(), ScalarRangeMode::Automatic, &ColorMap::viridis(), ColorRgba::new(1.0, 0.0, 1.0, 1.0).unwrap()),
            Err(ScalarTransferError3::InvalidRange(ScalarRangeError::NoFiniteValues))
        ));
        let range = ScalarRange::new(0.0, 1.0).unwrap();
        let invalid_color = ColorRgba::new(1.0, 0.0, 1.0, 1.0).unwrap();
        let mapped = map_transferred_scalar_field(
            transferred, ScalarRangeMode::Explicit(range), &ColorMap::viridis(), invalid_color,
        ).unwrap();
        assert!(mapped.colors().iter().all(|color| *color == invalid_color));
    }
}
