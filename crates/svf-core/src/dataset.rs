//! Scientific datasets: source geometry and fields attached to its entities.
//!
//! Dataset geometry remains authoritative. Derived render geometry, such as
//! the triangles generated from a structured grid, is not substituted for
//! the source dataset when determining field cardinality.

use std::{error::Error, fmt};

use crate::{
    unstructured::UnstructuredGrid3,
    fields::{FieldError, FieldMetadata, ScalarField, VectorField3},
    geometry::{PointSet3, Polyline3, TriangleMesh3},
    sampling::StructuredGrid2D,
};

/// Geometry variants currently supported as source datasets.
#[derive(Clone, Debug, PartialEq)]
pub enum DatasetGeometry {
    /// An unordered collection of independent points; it has no cells.
    PointSet3(PointSet3),
    /// An ordered polyline whose cells are its consecutive line segments.
    Polyline3(Polyline3),
    /// An indexed triangle mesh whose cells are its source triangles.
    TriangleMesh3(TriangleMesh3),
    /// A two-dimensional structured grid whose cells are grid intervals.
    StructuredGrid2D(StructuredGrid2D),
    /// An unstructured collection of source cells with explicit connectivity.
    UnstructuredGrid3(UnstructuredGrid3),
}

impl DatasetGeometry {
    /// Returns the number of source points or samples.
    #[must_use]
    pub fn point_count(&self) -> usize {
        match self {
            Self::PointSet3(points) => points.len(),
            Self::Polyline3(line) => line.len(),
            Self::TriangleMesh3(mesh) => mesh.vertex_count(),
            Self::StructuredGrid2D(grid) => grid.sample_count(),
            Self::UnstructuredGrid3(grid) => grid.point_count(),
        }
    }

    /// Returns the number of logical source cells.
    ///
    /// For a structured grid this is the number of grid rectangles, not the
    /// number of triangles that may later be generated for surface rendering.
    #[must_use]
    pub fn cell_count(&self) -> usize {
        match self {
            Self::PointSet3(_) => 0,
            Self::Polyline3(line) => line.segment_count(),
            Self::TriangleMesh3(mesh) => mesh.triangle_count(),
            Self::StructuredGrid2D(grid) => {
                (grid.x_sample_count() - 1) * (grid.y_sample_count() - 1)
            }
            Self::UnstructuredGrid3(grid) => grid.cell_count(),
        }
    }
}

impl From<PointSet3> for DatasetGeometry {
    fn from(value: PointSet3) -> Self {
        Self::PointSet3(value)
    }
}

impl From<Polyline3> for DatasetGeometry {
    fn from(value: Polyline3) -> Self {
        Self::Polyline3(value)
    }
}

impl From<TriangleMesh3> for DatasetGeometry {
    fn from(value: TriangleMesh3) -> Self {
        Self::TriangleMesh3(value)
    }
}

impl From<StructuredGrid2D> for DatasetGeometry {
    fn from(value: StructuredGrid2D) -> Self {
        Self::StructuredGrid2D(value)
    }
}

impl From<UnstructuredGrid3> for DatasetGeometry {
    fn from(value: UnstructuredGrid3) -> Self {
        Self::UnstructuredGrid3(value)
    }
}

/// A field stored by a dataset.
///
/// Names are unique across scalar and vector fields within one dataset.
#[derive(Clone, Debug, PartialEq)]
pub enum DatasetField {
    /// A scalar field.
    Scalar(ScalarField),
    /// A three-component vector field.
    Vector3(VectorField3),
}

impl DatasetField {
    /// Returns the field's metadata.
    #[must_use]
    pub fn metadata(&self) -> &FieldMetadata {
        match self {
            Self::Scalar(field) => field.metadata(),
            Self::Vector3(field) => field.metadata(),
        }
    }
}

/// Errors produced when attaching fields to a dataset.
#[derive(Clone, Debug, PartialEq)]
pub enum DatasetError {
    /// A field with this name is already attached to the dataset.
    DuplicateFieldName { name: String },
    /// The field's association or cardinality does not match the geometry.
    InvalidField(FieldError),
}

impl fmt::Display for DatasetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateFieldName { name } => {
                write!(f, "dataset already contains a field named '{name}'")
            }
            Self::InvalidField(error) => write!(f, "cannot attach field: {error}"),
        }
    }
}

impl Error for DatasetError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::DuplicateFieldName { .. } => None,
            Self::InvalidField(error) => Some(error),
        }
    }
}

/// A scientific dataset containing source geometry and attached fields.
///
/// Geometry is immutable after construction so attached field cardinality
/// cannot become invalid through an in-place topology change. Build a new
/// dataset when the source geometry changes.
#[derive(Clone, Debug, PartialEq)]
pub struct Dataset {
    geometry: DatasetGeometry,
    fields: Vec<DatasetField>,
}

impl Dataset {
    /// Creates an empty dataset from supported source geometry.
    pub fn new(geometry: impl Into<DatasetGeometry>) -> Self {
        Self {
            geometry: geometry.into(),
            fields: Vec::new(),
        }
    }

    /// Returns the source geometry.
    #[must_use]
    pub fn geometry(&self) -> &DatasetGeometry {
        &self.geometry
    }

    /// Returns the number of source points or structured-grid samples.
    #[must_use]
    pub fn point_count(&self) -> usize {
        self.geometry.point_count()
    }

    /// Returns the number of logical source cells.
    #[must_use]
    pub fn cell_count(&self) -> usize {
        self.geometry.cell_count()
    }

    /// Returns attached fields in insertion order.
    #[must_use]
    pub fn fields(&self) -> &[DatasetField] {
        &self.fields
    }

    /// Finds an attached field by its unique name.
    #[must_use]
    pub fn field(&self, name: &str) -> Option<&DatasetField> {
        self.fields
            .iter()
            .find(|field| field.metadata().name() == name)
    }

    /// Finds a scalar field by name.
    #[must_use]
    pub fn scalar_field(&self, name: &str) -> Option<&ScalarField> {
        match self.field(name)? {
            DatasetField::Scalar(field) => Some(field),
            DatasetField::Vector3(_) => None,
        }
    }

    /// Finds a three-component vector field by name.
    #[must_use]
    pub fn vector_field3(&self, name: &str) -> Option<&VectorField3> {
        match self.field(name)? {
            DatasetField::Vector3(field) => Some(field),
            DatasetField::Scalar(_) => None,
        }
    }

    /// Validates and attaches a scalar field.
    ///
    /// On error, the dataset remains unchanged.
    pub fn add_scalar_field(&mut self, field: ScalarField) -> Result<(), DatasetError> {
        self.validate_new_field(field.metadata())?;
        field
            .validate_cardinality(self.point_count(), self.cell_count())
            .map_err(DatasetError::InvalidField)?;
        self.fields.push(DatasetField::Scalar(field));
        Ok(())
    }

    /// Validates and attaches a three-component vector field.
    ///
    /// On error, the dataset remains unchanged.
    pub fn add_vector_field3(&mut self, field: VectorField3) -> Result<(), DatasetError> {
        self.validate_new_field(field.metadata())?;
        field
            .validate_cardinality(self.point_count(), self.cell_count())
            .map_err(DatasetError::InvalidField)?;
        self.fields.push(DatasetField::Vector3(field));
        Ok(())
    }

    fn validate_new_field(&self, metadata: &FieldMetadata) -> Result<(), DatasetError> {
        if self
            .fields
            .iter()
            .any(|existing| existing.metadata().name() == metadata.name())
        {
            return Err(DatasetError::DuplicateFieldName {
                name: metadata.name().to_owned(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        fields::{FieldAssociation, VectorBasis3, VectorValue3},
        geometry::GeometryError,
        sampling::{
            Domain1D, Domain2D, SamplingSpec2D, SampleValue, sample_scalar_2d,
        },
        spatial::WorldPoint3,
    };

    fn metadata(name: &str) -> FieldMetadata {
        FieldMetadata::new(name, None).unwrap()
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

    fn triangle_mesh() -> TriangleMesh3 {
        TriangleMesh3::new(
            vec![
                WorldPoint3::new(0.0, 0.0, 0.0),
                WorldPoint3::new(1.0, 0.0, 0.0),
                WorldPoint3::new(0.0, 1.0, 0.0),
                WorldPoint3::new(1.0, 1.0, 0.0),
            ],
            vec![0, 1, 2, 2, 1, 3],
        )
        .unwrap()
    }

    #[test]
    fn geometry_counts_match_each_source_geometry_type() {
        let points = PointSet3::new(vec![
            WorldPoint3::new(0.0, 0.0, 0.0),
            WorldPoint3::new(1.0, 0.0, 0.0),
        ])
        .unwrap();
        let point_dataset = Dataset::new(points);
        assert_eq!((point_dataset.point_count(), point_dataset.cell_count()), (2, 0));

        let line = Polyline3::new(vec![
            WorldPoint3::new(0.0, 0.0, 0.0),
            WorldPoint3::new(1.0, 0.0, 0.0),
            WorldPoint3::new(2.0, 0.0, 0.0),
        ])
        .unwrap();
        let line_dataset = Dataset::new(line);
        assert_eq!((line_dataset.point_count(), line_dataset.cell_count()), (3, 2));

        let mesh_dataset = Dataset::new(triangle_mesh());
        assert_eq!((mesh_dataset.point_count(), mesh_dataset.cell_count()), (4, 2));
    }

    #[test]
    fn structured_grid_counts_source_cells_not_generated_surface_triangles() {
        let grid = structured_grid(3, 2);
        let dataset = Dataset::new(grid.clone());
        assert_eq!((dataset.point_count(), dataset.cell_count()), (6, 2));

        let surface = grid.to_surface_mesh().unwrap();
        assert_eq!(surface.triangle_count(), 4);
        assert_eq!(dataset.cell_count(), 2);
    }

    #[test]
    fn attaches_point_cell_and_dataset_scalar_fields() {
        let mut dataset = Dataset::new(triangle_mesh());
        let point = ScalarField::new(
            metadata("temperature"),
            FieldAssociation::Point,
            vec![SampleValue::Finite(10.0); 4],
        )
        .unwrap();
        let cell = ScalarField::new(
            metadata("pressure"),
            FieldAssociation::Cell,
            vec![SampleValue::Finite(100.0); 2],
        )
        .unwrap();
        let whole = ScalarField::new(
            metadata("ambient_temperature"),
            FieldAssociation::Dataset,
            vec![SampleValue::Finite(293.15)],
        )
        .unwrap();

        dataset.add_scalar_field(point).unwrap();
        dataset.add_scalar_field(cell).unwrap();
        dataset.add_scalar_field(whole).unwrap();

        assert_eq!(dataset.fields().len(), 3);
        assert_eq!(dataset.scalar_field("temperature").unwrap().len(), 4);
        assert_eq!(dataset.scalar_field("pressure").unwrap().len(), 2);
        assert_eq!(dataset.scalar_field("ambient_temperature").unwrap().len(), 1);
    }

    #[test]
    fn attaches_vector_fields_and_retrieves_them_by_name() {
        let mut dataset = Dataset::new(triangle_mesh());
        let field = VectorField3::new(
            metadata("velocity"),
            FieldAssociation::Point,
            VectorBasis3::World,
            vec![VectorValue3::from_f64(1.0, 2.0, 3.0); 4],
        )
        .unwrap();
        dataset.add_vector_field3(field).unwrap();

        assert_eq!(dataset.vector_field3("velocity").unwrap().len(), 4);
        assert!(dataset.scalar_field("velocity").is_none());
    }

    #[test]
    fn rejects_cardinality_mismatch_without_mutating_dataset() {
        let mut dataset = Dataset::new(triangle_mesh());
        let invalid = ScalarField::new(
            metadata("temperature"),
            FieldAssociation::Point,
            vec![SampleValue::Finite(1.0); 3],
        )
        .unwrap();

        assert_eq!(
            dataset.add_scalar_field(invalid),
            Err(DatasetError::InvalidField(
                FieldError::CardinalityMismatch {
                    association: FieldAssociation::Point,
                    expected: 4,
                    actual: 3,
                }
            ))
        );
        assert!(dataset.fields().is_empty());
    }

    #[test]
    fn rejects_duplicate_names_across_field_kinds() {
        let mut dataset = Dataset::new(triangle_mesh());
        dataset
            .add_scalar_field(
                ScalarField::new(
                    metadata("velocity"),
                    FieldAssociation::Point,
                    vec![SampleValue::Finite(1.0); 4],
                )
                .unwrap(),
            )
            .unwrap();
        let duplicate = VectorField3::new(
            metadata("velocity"),
            FieldAssociation::Point,
            VectorBasis3::World,
            vec![VectorValue3::from_f64(1.0, 0.0, 0.0); 4],
        )
        .unwrap();

        assert_eq!(
            dataset.add_vector_field3(duplicate),
            Err(DatasetError::DuplicateFieldName {
                name: "velocity".to_owned(),
            })
        );
        assert_eq!(dataset.fields().len(), 1);
    }

    #[test]
    fn rejects_field_that_does_not_match_structured_grid_source_cells() {
        let mut dataset = Dataset::new(structured_grid(3, 2));
        let invalid = ScalarField::new(
            metadata("pressure"),
            FieldAssociation::Cell,
            vec![SampleValue::Finite(1.0); 4],
        )
        .unwrap();

        assert_eq!(
            dataset.add_scalar_field(invalid),
            Err(DatasetError::InvalidField(
                FieldError::CardinalityMismatch {
                    association: FieldAssociation::Cell,
                    expected: 2,
                    actual: 4,
                }
            ))
        );
        assert!(dataset.fields().is_empty());
    }

    #[test]
    fn empty_point_set_accepts_empty_point_field() {
        let geometry = PointSet3::new(Vec::new()).unwrap();
        let mut dataset = Dataset::new(geometry);
        let field = ScalarField::new(metadata("empty_measurement"), FieldAssociation::Point, vec![])
            .unwrap();
        dataset.add_scalar_field(field).unwrap();
        assert_eq!(dataset.scalar_field("empty_measurement").unwrap().len(), 0);
    }

    #[test]
    fn geometry_is_validated_before_dataset_creation() {
        let invalid = TriangleMesh3::new(
            vec![WorldPoint3::new(f64::NAN, 0.0, 0.0)],
            vec![],
        );
        assert_eq!(invalid, Err(GeometryError::NonFinitePoint { index: 0 }));
    }
}
