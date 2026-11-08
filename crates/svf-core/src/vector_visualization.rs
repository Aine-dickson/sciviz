//! Association-aware source locations for three-component vector fields.
//!
//! This module resolves each vector measurement to its original dataset
//! entity and a world-space anchor. It does not compute vector magnitudes,
//! directions, glyph geometry, or renderer-specific data.

use std::{error::Error, fmt};

use crate::{
    dataset::{Dataset, DatasetField, DatasetGeometry},
    fields::{FieldAssociation, VectorBasis3, VectorValue3},
    sampling::SampleValue,
    spatial::WorldPoint3,
};

/// Identifies the source dataset entity associated with one vector value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VectorEntity3 {
    /// A source point or structured-grid sample ID.
    Point(usize),
    /// A logical source cell ID.
    Cell(usize),
}

/// One vector measurement paired with its exact source identity and anchor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VectorSource3 {
    entity: VectorEntity3,
    anchor: WorldPoint3,
    value: VectorValue3,
}

impl VectorSource3 {
    /// Returns the source point or cell identity.
    #[must_use]
    pub const fn entity(&self) -> VectorEntity3 {
        self.entity
    }

    /// Returns the world-space location assigned by the association contract.
    #[must_use]
    pub const fn anchor(&self) -> WorldPoint3 {
        self.anchor
    }

    /// Returns the original classified vector components without modification.
    #[must_use]
    pub const fn value(&self) -> VectorValue3 {
        self.value
    }
}

/// Errors produced while resolving vector-field source locations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VectorLocationError {
    /// The dataset does not contain a field with the requested name.
    FieldNotFound { name: String },
    /// A field with the requested name exists but is not a vector field.
    FieldIsNotVector { name: String },
    /// Dataset-associated vectors have no implicit spatial anchor.
    DatasetAssociationHasNoAnchor,
    /// Only world-coordinate vectors are currently supported.
    UnsupportedBasis { basis: VectorBasis3 },
    /// A structured-grid source sample has no finite world-space height.
    NonFiniteSourceLocation {
        entity: VectorEntity3,
    },
    /// Storage for resolved source records could not be reserved.
    AllocationFailed { requested_items: usize },
}

impl fmt::Display for VectorLocationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FieldNotFound { name } => {
                write!(f, "dataset has no field named '{name}'")
            }
            Self::FieldIsNotVector { name } => {
                write!(f, "field '{name}' is not a three-component vector field")
            }
            Self::DatasetAssociationHasNoAnchor => write!(
                f,
                "dataset-associated vector fields require an explicit spatial anchor"
            ),
            Self::UnsupportedBasis { basis } => {
                write!(f, "vector basis {basis:?} is not supported")
            }
            Self::NonFiniteSourceLocation { entity } => {
                write!(f, "source entity {entity:?} has no finite world-space location")
            }
            Self::AllocationFailed { requested_items } => write!(
                f,
                "could not allocate storage for {requested_items} vector source locations"
            ),
        }
    }
}

impl Error for VectorLocationError {}

/// Resolves a named dataset vector field into source-ordered world anchors.
///
/// Point-associated values use exact source point coordinates. Cell-associated
/// values use the arithmetic mean of the source cell's vertices. For
/// structured graph grids, world coordinates are (sample x, scalar value,
/// sample y); a non-finite scalar sample therefore has no world-space anchor.
///
/// Dataset-associated fields are rejected because a dataset-level vector has
/// no intrinsic spatial location. No origin or other implicit anchor is used.
pub fn resolve_vector_sources(
    dataset: &Dataset,
    field_name: &str,
) -> Result<Vec<VectorSource3>, VectorLocationError> {
    let field = match dataset.field(field_name) {
        None => {
            return Err(VectorLocationError::FieldNotFound {
                name: field_name.to_owned(),
            });
        }
        Some(DatasetField::Scalar(_)) => {
            return Err(VectorLocationError::FieldIsNotVector {
                name: field_name.to_owned(),
            });
        }
        Some(DatasetField::Vector3(field)) => field,
    };

    if field.basis() != VectorBasis3::World {
        return Err(VectorLocationError::UnsupportedBasis {
            basis: field.basis(),
        });
    }

    let association = field.association();
    if association == FieldAssociation::Dataset {
        return Err(VectorLocationError::DatasetAssociationHasNoAnchor);
    }

    let count = field.len();
    let mut sources = Vec::new();
    sources
        .try_reserve_exact(count)
        .map_err(|_| VectorLocationError::AllocationFailed {
            requested_items: count,
        })?;

    match association {
        FieldAssociation::Point => {
            for (index, value) in field.values().iter().copied().enumerate() {
                let anchor = point_anchor(dataset.geometry(), index)?;
                sources.push(VectorSource3 {
                    entity: VectorEntity3::Point(index),
                    anchor,
                    value,
                });
            }
        }
        FieldAssociation::Cell => {
            for (index, value) in field.values().iter().copied().enumerate() {
                let anchor = cell_anchor(dataset.geometry(), index)?;
                sources.push(VectorSource3 {
                    entity: VectorEntity3::Cell(index),
                    anchor,
                    value,
                });
            }
        }
        FieldAssociation::Dataset => unreachable!("dataset association rejected above"),
    }

    Ok(sources)
}

fn point_anchor(
    geometry: &DatasetGeometry,
    index: usize,
) -> Result<WorldPoint3, VectorLocationError> {
    let anchor = match geometry {
        DatasetGeometry::PointSet3(points) => points.points()[index],
        DatasetGeometry::Polyline3(line) => line.points()[index],
        DatasetGeometry::TriangleMesh3(mesh) => mesh.vertices()[index],
        DatasetGeometry::StructuredGrid2D(grid) => {
            let x_count = grid.x_sample_count();
            let sample = grid
                .sample(index % x_count, index / x_count)
                .expect("dataset field cardinality matches structured grid");
            let SampleValue::Finite(height) = sample.value() else {
                return Err(VectorLocationError::NonFiniteSourceLocation {
                    entity: VectorEntity3::Point(index),
                });
            };
            WorldPoint3::new(sample.x(), height, sample.y())
        }
        DatasetGeometry::UnstructuredGrid3(grid) => grid.points()[index],
    };
    Ok(anchor)
}

fn cell_anchor(
    geometry: &DatasetGeometry,
    index: usize,
) -> Result<WorldPoint3, VectorLocationError> {
    match geometry {
        DatasetGeometry::PointSet3(_) => {
            unreachable!("point sets have no cells and cannot own a cell field")
        }
        DatasetGeometry::Polyline3(line) => {
            let points = line.points();
            Ok(mean_point(&points[index..=index + 1]))
        }
        DatasetGeometry::TriangleMesh3(mesh) => {
            let triangle = mesh
                .triangles()
                .nth(index)
                .expect("dataset field cardinality matches triangle mesh");
            let vertices = mesh.vertices();
            Ok(mean_point(&[
                vertices[triangle[0] as usize],
                vertices[triangle[1] as usize],
                vertices[triangle[2] as usize],
            ]))
        }
        DatasetGeometry::StructuredGrid2D(grid) => {
            let x_count = grid.x_sample_count() - 1;
            let x = index % x_count;
            let y = index / x_count;
            let corners = [
                structured_point(grid, x, y, index)?,
                structured_point(grid, x + 1, y, index)?,
                structured_point(grid, x, y + 1, index)?,
                structured_point(grid, x + 1, y + 1, index)?,
            ];
            Ok(mean_point(&corners))
        }
        DatasetGeometry::UnstructuredGrid3(grid) => {
            let indices = grid
                .cell_point_indices(index)
                .expect("dataset field cardinality matches unstructured grid");
            let points: Vec<_> = indices
                .iter()
                .map(|point_index| grid.points()[*point_index as usize])
                .collect();
            Ok(mean_point(&points))
        }
    }
}

fn structured_point(
    grid: &crate::sampling::StructuredGrid2D,
    x: usize,
    y: usize,
    cell_index: usize,
) -> Result<WorldPoint3, VectorLocationError> {
    let sample = grid
        .sample(x, y)
        .expect("structured cell corner is within grid bounds");
    let SampleValue::Finite(height) = sample.value() else {
        return Err(VectorLocationError::NonFiniteSourceLocation {
            entity: VectorEntity3::Cell(cell_index),
        });
    };
    Ok(WorldPoint3::new(sample.x(), height, sample.y()))
}

/// Computes a vertex average without overflowing merely because the
/// coordinates are large. All source geometry constructors guarantee finite
/// coordinates, and scaling keeps each accumulated component bounded.
fn mean_point(points: &[WorldPoint3]) -> WorldPoint3 {
    let scale = points
        .iter()
        .flat_map(|point| point.to_array())
        .map(f64::abs)
        .fold(0.0_f64, f64::max);

    if scale == 0.0 {
        return WorldPoint3::ORIGIN;
    }

    let divisor = points.len() as f64;
    let mut mean = [0.0; 3];
    for point in points {
        let coordinates = point.to_array();
        for axis in 0..3 {
            mean[axis] += (coordinates[axis] / scale) / divisor;
        }
    }

    WorldPoint3::new(mean[0] * scale, mean[1] * scale, mean[2] * scale)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        fields::{FieldMetadata, VectorBasis3},
        geometry::{PointSet3, Polyline3, TriangleMesh3},
        sampling::{
            Domain1D, Domain2D, SamplingSpec2D, sample_scalar_2d, StructuredGrid2D,
        },
        spatial::WorldPoint3,
        unstructured::{CellType3, UnstructuredGrid3},
    };

    fn metadata(name: &str) -> FieldMetadata {
        FieldMetadata::new(name, None).unwrap()
    }

    fn vector(x: f64, y: f64, z: f64) -> VectorValue3 {
        VectorValue3::from_f64(x, y, z)
    }

    fn add_vectors(
        dataset: &mut Dataset,
        association: FieldAssociation,
        values: Vec<VectorValue3>,
    ) {
        dataset
            .add_vector_field3(
                crate::fields::VectorField3::new(
                    metadata("velocity"),
                    association,
                    VectorBasis3::World,
                    values,
                )
                .unwrap(),
            )
            .unwrap();
    }

    fn grid(nx: usize, ny: usize) -> StructuredGrid2D {
        let domain = Domain2D::new(
            Domain1D::new(0.0, (nx - 1) as f64).unwrap(),
            Domain1D::new(0.0, (ny - 1) as f64).unwrap(),
        );
        StructuredGrid2D::from_samples(
            sample_scalar_2d(domain, SamplingSpec2D::new(nx, ny).unwrap(), |x, y| {
                x + 2.0 * y
            })
            .unwrap(),
        )
    }

    #[test]
    fn point_association_uses_exact_source_point_order_and_identity() {
        let points = vec![
            WorldPoint3::new(1.0, 2.0, 3.0),
            WorldPoint3::new(-4.0, 5.0, 6.0),
        ];
        let mut dataset = Dataset::new(PointSet3::new(points.clone()).unwrap());
        add_vectors(&mut dataset, FieldAssociation::Point, vec![vector(1.0, 0.0, 0.0), vector(0.0, 2.0, 0.0)]);

        let sources = resolve_vector_sources(&dataset, "velocity").unwrap();
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].entity(), VectorEntity3::Point(0));
        assert_eq!(sources[0].anchor(), points[0]);
        assert_eq!(sources[0].value(), vector(1.0, 0.0, 0.0));
        assert_eq!(sources[1].entity(), VectorEntity3::Point(1));
        assert_eq!(sources[1].anchor(), points[1]);
    }

    #[test]
    fn point_association_supports_each_geometry_kind() {
        let line = Polyline3::new(vec![
            WorldPoint3::new(0.0, 1.0, 2.0),
            WorldPoint3::new(3.0, 4.0, 5.0),
        ])
        .unwrap();
        let mut line_dataset = Dataset::new(line);
        add_vectors(&mut line_dataset, FieldAssociation::Point, vec![vector(1.0, 0.0, 0.0); 2]);
        assert_eq!(resolve_vector_sources(&line_dataset, "velocity").unwrap()[1].anchor(), WorldPoint3::new(3.0, 4.0, 5.0));

        let mesh = TriangleMesh3::new(
            vec![WorldPoint3::new(0.0, 0.0, 0.0), WorldPoint3::new(1.0, 2.0, 3.0), WorldPoint3::new(4.0, 5.0, 6.0)],
            vec![0, 1, 2],
        ).unwrap();
        let mut mesh_dataset = Dataset::new(mesh);
        add_vectors(&mut mesh_dataset, FieldAssociation::Point, vec![vector(1.0, 0.0, 0.0); 3]);
        assert_eq!(resolve_vector_sources(&mesh_dataset, "velocity").unwrap()[2].anchor(), WorldPoint3::new(4.0, 5.0, 6.0));

        let unstructured = UnstructuredGrid3::new(
            vec![WorldPoint3::new(0.0, 0.0, 0.0), WorldPoint3::new(2.0, 0.0, 0.0)],
            vec![CellType3::Line2],
            vec![0, 2],
            vec![0, 1],
        ).unwrap();
        let mut unstructured_dataset = Dataset::new(unstructured);
        add_vectors(&mut unstructured_dataset, FieldAssociation::Point, vec![vector(1.0, 0.0, 0.0); 2]);
        assert_eq!(resolve_vector_sources(&unstructured_dataset, "velocity").unwrap()[1].anchor(), WorldPoint3::new(2.0, 0.0, 0.0));
    }

    #[test]
    fn structured_point_association_uses_graph_surface_world_coordinates() {
        let mut dataset = Dataset::new(grid(2, 2));
        add_vectors(&mut dataset, FieldAssociation::Point, vec![vector(1.0, 0.0, 0.0); 4]);
        let sources = resolve_vector_sources(&dataset, "velocity").unwrap();
        assert_eq!(sources[0].anchor(), WorldPoint3::new(0.0, 0.0, 0.0));
        assert_eq!(sources[3].anchor(), WorldPoint3::new(1.0, 3.0, 1.0));
    }

    #[test]
    fn cell_association_uses_vertex_average_for_supported_geometry() {
        let line = Polyline3::new(vec![
            WorldPoint3::new(0.0, 0.0, 0.0),
            WorldPoint3::new(2.0, 4.0, 6.0),
            WorldPoint3::new(4.0, 4.0, 4.0),
        ]).unwrap();
        let mut line_dataset = Dataset::new(line);
        add_vectors(&mut line_dataset, FieldAssociation::Cell, vec![vector(1.0, 0.0, 0.0); 2]);
        let line_sources = resolve_vector_sources(&line_dataset, "velocity").unwrap();
        assert_eq!(line_sources[0].entity(), VectorEntity3::Cell(0));
        assert_eq!(line_sources[0].anchor(), WorldPoint3::new(1.0, 2.0, 3.0));
        assert_eq!(line_sources[1].anchor(), WorldPoint3::new(3.0, 4.0, 5.0));

        let mesh = TriangleMesh3::new(
            vec![WorldPoint3::new(0.0, 0.0, 0.0), WorldPoint3::new(3.0, 0.0, 0.0), WorldPoint3::new(0.0, 3.0, 0.0)],
            vec![0, 1, 2],
        ).unwrap();
        let mut mesh_dataset = Dataset::new(mesh);
        add_vectors(&mut mesh_dataset, FieldAssociation::Cell, vec![vector(1.0, 0.0, 0.0)]);
        assert_eq!(resolve_vector_sources(&mesh_dataset, "velocity").unwrap()[0].anchor(), WorldPoint3::new(1.0, 1.0, 0.0));

        let structured = grid(2, 2);
        let mut structured_dataset = Dataset::new(structured);
        add_vectors(&mut structured_dataset, FieldAssociation::Cell, vec![vector(1.0, 0.0, 0.0)]);
        assert_eq!(resolve_vector_sources(&structured_dataset, "velocity").unwrap()[0].anchor(), WorldPoint3::new(0.5, 1.5, 0.5));

        let unstructured = UnstructuredGrid3::new(
            vec![
                WorldPoint3::new(0.0, 0.0, 0.0),
                WorldPoint3::new(2.0, 0.0, 0.0),
                WorldPoint3::new(0.0, 2.0, 0.0),
                WorldPoint3::new(0.0, 0.0, 2.0),
            ],
            vec![CellType3::Tetrahedron4],
            vec![0, 4],
            vec![0, 1, 2, 3],
        ).unwrap();
        let mut unstructured_dataset = Dataset::new(unstructured);
        add_vectors(&mut unstructured_dataset, FieldAssociation::Cell, vec![vector(1.0, 0.0, 0.0)]);
        assert_eq!(resolve_vector_sources(&unstructured_dataset, "velocity").unwrap()[0].anchor(), WorldPoint3::new(0.5, 0.5, 0.5));
    }

    #[test]
    fn rejects_dataset_association_instead_of_inventing_an_anchor() {
        let mut dataset = Dataset::new(PointSet3::new(vec![]).unwrap());
        dataset.add_vector_field3(
            crate::fields::VectorField3::new(
                metadata("velocity"),
                FieldAssociation::Dataset,
                VectorBasis3::World,
                vec![vector(1.0, 0.0, 0.0)],
            ).unwrap(),
        ).unwrap();
        assert_eq!(
            resolve_vector_sources(&dataset, "velocity"),
            Err(VectorLocationError::DatasetAssociationHasNoAnchor)
        );
    }

    #[test]
    fn preserves_non_finite_vector_components_for_later_status_classification() {
        let mut dataset = Dataset::new(PointSet3::new(vec![WorldPoint3::ORIGIN]).unwrap());
        add_vectors(&mut dataset, FieldAssociation::Point, vec![vector(f64::NAN, 2.0, 3.0)]);
        let sources = resolve_vector_sources(&dataset, "velocity").unwrap();
        assert_eq!(
            sources[0].value().components(),
            [
                SampleValue::NonFinite(crate::sampling::NonFiniteValue::NaN),
                SampleValue::Finite(2.0),
                SampleValue::Finite(3.0),
            ]
        );
    }

    #[test]
    fn reports_non_finite_structured_sample_as_missing_world_anchor() {
        let domain = Domain2D::new(
            Domain1D::new(0.0, 1.0).unwrap(),
            Domain1D::new(0.0, 1.0).unwrap(),
        );
        let structured = StructuredGrid2D::from_samples(
            sample_scalar_2d(domain, SamplingSpec2D::new(2, 2).unwrap(), |x, y| {
                if x == 1.0 && y == 1.0 { f64::NAN } else { x + y }
            }).unwrap(),
        );
        let mut dataset = Dataset::new(structured);
        add_vectors(&mut dataset, FieldAssociation::Point, vec![vector(1.0, 0.0, 0.0); 4]);
        assert_eq!(
            resolve_vector_sources(&dataset, "velocity"),
            Err(VectorLocationError::NonFiniteSourceLocation { entity: VectorEntity3::Point(3) })
        );
    }

    #[test]
    fn stable_vertex_average_handles_extreme_finite_coordinates() {
        let mesh = TriangleMesh3::new(
            vec![
                WorldPoint3::new(-f64::MAX, 0.0, 0.0),
                WorldPoint3::new(f64::MAX, 0.0, 0.0),
                WorldPoint3::new(0.0, 3.0, 0.0),
            ],
            vec![0, 1, 2],
        ).unwrap();
        let mut dataset = Dataset::new(mesh);
        add_vectors(&mut dataset, FieldAssociation::Cell, vec![vector(1.0, 0.0, 0.0)]);
        let anchor = resolve_vector_sources(&dataset, "velocity").unwrap()[0].anchor();
        assert_eq!(anchor, WorldPoint3::new(0.0, 1.0, 0.0));
    }

    #[test]
    fn reports_missing_or_non_vector_fields() {
        let mut dataset = Dataset::new(PointSet3::new(vec![]).unwrap());
        dataset.add_scalar_field(
            crate::fields::ScalarField::new(
                metadata("temperature"),
                FieldAssociation::Dataset,
                vec![SampleValue::Finite(1.0)],
            ).unwrap(),
        ).unwrap();
        assert_eq!(
            resolve_vector_sources(&dataset, "missing"),
            Err(VectorLocationError::FieldNotFound { name: "missing".to_owned() })
        );
        assert_eq!(
            resolve_vector_sources(&dataset, "temperature"),
            Err(VectorLocationError::FieldIsNotVector { name: "temperature".to_owned() })
        );
    }
}
