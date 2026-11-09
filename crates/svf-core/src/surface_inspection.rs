//! Scientific inspection of provenance-preserving imported triangle surfaces.
//!
//! This module resolves ray hits against the CPU-side f64 geometry and maps
//! them back to authoritative source dataset entities and scalar fields.

use std::{error::Error, fmt};

use crate::{
    dataset::{Dataset, DatasetField, DatasetGeometry},
    fields::FieldAssociation,
    geometry::TriangleMesh3,
    isosurface::Isosurface3,
    isosurface_field_transfer::transfer_point_scalar_field,
    ray::{intersect_mesh, Ray3},
    sampling::SampleValue,
    spatial::WorldPoint3,
    surface_triangulation::TriangulatedSurface3,
};

/// The source entity represented by an inspected rendered triangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InspectedEntity3 {
    /// A triangle derived from a boundary polygon of a source volume cell.
    BoundaryFace {
        source_cell_id: usize,
        source_face_id: usize,
        source_local_face_id: usize,
        source_triangle_index: usize,
    },
    /// A triangle generated from a source tetrahedral cell's isosurface.
    IsosurfaceTriangle { source_cell_id: usize },
}

/// One scalar field value evaluated at an inspection hit.
#[derive(Clone, Debug, PartialEq)]
pub struct InspectedScalarValue3 {
    name: String,
    units: Option<String>,
    association: FieldAssociation,
    value: SampleValue,
}

impl InspectedScalarValue3 {
    #[must_use]
    pub fn name(&self) -> &str { &self.name }
    #[must_use]
    pub fn units(&self) -> Option<&str> { self.units.as_deref() }
    #[must_use]
    pub const fn association(&self) -> FieldAssociation { self.association }
    #[must_use]
    pub const fn value(&self) -> SampleValue { self.value }
}

/// A complete scientific hit, including source identity and field values.
#[derive(Clone, Debug, PartialEq)]
pub struct ScientificSurfaceHit3 {
    world_position: WorldPoint3,
    distance: f64,
    triangle_index: usize,
    barycentric: [f64; 3],
    entity: InspectedEntity3,
    scalar_values: Vec<InspectedScalarValue3>,
}

impl ScientificSurfaceHit3 {
    #[must_use]
    pub const fn world_position(&self) -> WorldPoint3 { self.world_position }
    #[must_use]
    pub const fn distance(&self) -> f64 { self.distance }
    #[must_use]
    pub const fn triangle_index(&self) -> usize { self.triangle_index }
    #[must_use]
    pub const fn barycentric(&self) -> [f64; 3] { self.barycentric }
    #[must_use]
    pub const fn entity(&self) -> InspectedEntity3 { self.entity }
    #[must_use]
    pub fn scalar_values(&self) -> &[InspectedScalarValue3] { &self.scalar_values }
}

/// Errors produced while resolving a rendered hit to scientific source data.
#[derive(Clone, Debug, PartialEq)]
pub enum SurfaceInspectionError3 {
    DatasetGeometryMustBeUnstructured,
    InvalidRenderMesh(String),
    TriangleCountMismatch { expected: usize, actual: usize },
    SourcePointCountMismatch { expected: usize, actual: usize },
    SourcePointOutOfBounds { triangle: usize, point_id: u32, point_count: usize },
    SourceCellOutOfBounds { triangle: usize, cell_id: usize, cell_count: usize },
    InvalidFieldCardinality { name: String, message: String },
    InvalidPointInterpolation { field: String, triangle: usize },
    InvalidIsosurfaceTransfer { field: String, message: String },
}

impl fmt::Display for SurfaceInspectionError3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DatasetGeometryMustBeUnstructured => write!(f, "scientific surface inspection requires UnstructuredGrid3 source geometry"),
            Self::InvalidRenderMesh(message) => write!(f, "invalid inspection mesh: {message}"),
            Self::TriangleCountMismatch { expected, actual } => write!(f, "inspection representation has {actual} triangle records for {expected} mesh triangles"),
            Self::SourcePointCountMismatch { expected, actual } => write!(f, "inspection representation has {actual} source point IDs for {expected} compact points"),
            Self::SourcePointOutOfBounds { triangle, point_id, point_count } => write!(f, "triangle {triangle} references source point {point_id}, but dataset has {point_count} points"),
            Self::SourceCellOutOfBounds { triangle, cell_id, cell_count } => write!(f, "triangle {triangle} references source cell {cell_id}, but dataset has {cell_count} cells"),
            Self::InvalidFieldCardinality { name, message } => write!(f, "scalar field '{name}' has invalid cardinality: {message}"),
            Self::InvalidPointInterpolation { field, triangle } => write!(f, "point scalar interpolation for field '{field}' is non-finite at triangle {triangle}"),
            Self::InvalidIsosurfaceTransfer { field, message } => write!(f, "cannot transfer field '{field}' to isosurface: {message}"),
        }
    }
}
impl Error for SurfaceInspectionError3 {}

/// Inspects a ray against a triangulated unstructured-volume boundary.
///
/// A miss is returned as `Ok(None)`; malformed source mappings are errors.
pub fn inspect_boundary_surface(
    dataset: &Dataset,
    surface: &TriangulatedSurface3,
    ray: &Ray3,
) -> Result<Option<ScientificSurfaceHit3>, SurfaceInspectionError3> {
    let grid = match dataset.geometry() {
        DatasetGeometry::UnstructuredGrid3(grid) => grid,
        _ => return Err(SurfaceInspectionError3::DatasetGeometryMustBeUnstructured),
    };
    if surface.source_point_ids().len() != surface.point_count() {
        return Err(SurfaceInspectionError3::SourcePointCountMismatch {
            expected: surface.point_count(), actual: surface.source_point_ids().len(),
        });
    }
    if surface.triangles().len() != surface.triangle_count() {
        return Err(SurfaceInspectionError3::TriangleCountMismatch {
            expected: surface.triangle_count(), actual: surface.triangles().len(),
        });
    }
    for (triangle, record) in surface.triangles().iter().enumerate() {
        if record.source_cell_index() >= grid.cell_count() {
            return Err(SurfaceInspectionError3::SourceCellOutOfBounds {
                triangle, cell_id: record.source_cell_index(), cell_count: grid.cell_count(),
            });
        }
        for compact_id in record.vertices() {
            let Some(&source_id) = surface.source_point_ids().get(compact_id as usize) else {
                return Err(SurfaceInspectionError3::SourcePointOutOfBounds {
                    triangle, point_id: compact_id, point_count: surface.point_count(),
                });
            };
            if source_id as usize >= grid.point_count() {
                return Err(SurfaceInspectionError3::SourcePointOutOfBounds {
                    triangle, point_id: source_id, point_count: grid.point_count(),
                });
            }
        }
    }
    let mesh = mesh_from_parts(surface.points(), surface.triangles().iter().flat_map(|t| t.vertices()).collect())?;
    let Some(hit) = intersect_mesh(ray, &mesh) else { return Ok(None); };
    let record = surface.triangles()[hit.triangle_index()];
    let source_ids = record.vertices().map(|id| surface.source_point_ids()[id as usize]);
    let values = evaluate_boundary_fields(dataset, record.source_cell_index(), source_ids, hit.barycentric(), hit.triangle_index())?;
    Ok(Some(ScientificSurfaceHit3 {
        world_position: hit.position(),
        distance: hit.distance(),
        triangle_index: hit.triangle_index(),
        barycentric: hit.barycentric(),
        entity: InspectedEntity3::BoundaryFace {
            source_cell_id: record.source_cell_index(),
            source_face_id: record.source_face_index(),
            source_local_face_id: record.source_local_face_index(),
            source_triangle_index: record.source_triangle_index(),
        },
        scalar_values: values,
    }))
}

/// Inspects a ray against a tetrahedral isosurface while resolving its source
/// tetrahedron and evaluating every scalar field through its declared
/// association and the isosurface vertex provenance.
pub fn inspect_isosurface(
    dataset: &Dataset,
    isosurface: &Isosurface3,
    ray: &Ray3,
) -> Result<Option<ScientificSurfaceHit3>, SurfaceInspectionError3> {
    let grid = match dataset.geometry() {
        DatasetGeometry::UnstructuredGrid3(grid) => grid,
        _ => return Err(SurfaceInspectionError3::DatasetGeometryMustBeUnstructured),
    };
    if isosurface.triangle_source_cells().len() != isosurface.mesh().triangle_count() {
        return Err(SurfaceInspectionError3::TriangleCountMismatch {
            expected: isosurface.mesh().triangle_count(),
            actual: isosurface.triangle_source_cells().len(),
        });
    }
    for (triangle, &cell_id) in isosurface.triangle_source_cells().iter().enumerate() {
        if cell_id >= grid.cell_count() {
            return Err(SurfaceInspectionError3::SourceCellOutOfBounds {
                triangle, cell_id, cell_count: grid.cell_count(),
            });
        }
    }
    let Some(hit) = intersect_mesh(ray, isosurface.mesh()) else { return Ok(None); };
    let cell_id = isosurface.triangle_source_cells()[hit.triangle_index()];
    let values = evaluate_isosurface_fields(dataset, isosurface, cell_id, hit.barycentric(), hit.triangle_index())?;
    Ok(Some(ScientificSurfaceHit3 {
        world_position: hit.position(),
        distance: hit.distance(),
        triangle_index: hit.triangle_index(),
        barycentric: hit.barycentric(),
        entity: InspectedEntity3::IsosurfaceTriangle { source_cell_id: cell_id },
        scalar_values: values,
    }))
}

fn mesh_from_parts(
    points: &[WorldPoint3],
    indices: Vec<u32>,
) -> Result<TriangleMesh3, SurfaceInspectionError3> {
    TriangleMesh3::new(points.to_vec(), indices)
        .map_err(|error| SurfaceInspectionError3::InvalidRenderMesh(error.to_string()))
}

fn evaluate_boundary_fields(
    dataset: &Dataset,
    cell_id: usize,
    source_ids: [u32; 3],
    weights: [f64; 3],
    triangle: usize,
) -> Result<Vec<InspectedScalarValue3>, SurfaceInspectionError3> {
    let mut result = Vec::new();
    for entry in dataset.fields() {
        let DatasetField::Scalar(field) = entry else { continue; };
        field.validate_cardinality(dataset.point_count(), dataset.cell_count())
            .map_err(|error| SurfaceInspectionError3::InvalidFieldCardinality {
                name: field.metadata().name().to_owned(), message: error.to_string(),
            })?;
        let value = match field.association() {
            FieldAssociation::Point => barycentric_value(source_ids.map(|id| field.values()[id as usize]), weights)
                .map_err(|_| SurfaceInspectionError3::InvalidPointInterpolation {
                    field: field.metadata().name().to_owned(), triangle,
                })?,
            FieldAssociation::Cell => field.values()[cell_id],
            FieldAssociation::Dataset => field.values()[0],
        };
        result.push(InspectedScalarValue3 {
            name: field.metadata().name().to_owned(),
            units: field.metadata().units().map(str::to_owned),
            association: field.association(),
            value,
        });
    }
    Ok(result)
}

fn evaluate_isosurface_fields(
    dataset: &Dataset,
    isosurface: &Isosurface3,
    cell_id: usize,
    weights: [f64; 3],
    triangle: usize,
) -> Result<Vec<InspectedScalarValue3>, SurfaceInspectionError3> {
    let mut result = Vec::new();
    for entry in dataset.fields() {
        let DatasetField::Scalar(field) = entry else { continue; };
        field.validate_cardinality(dataset.point_count(), dataset.cell_count())
            .map_err(|error| SurfaceInspectionError3::InvalidFieldCardinality {
                name: field.metadata().name().to_owned(), message: error.to_string(),
            })?;
        let value = match field.association() {
            FieldAssociation::Point => {
                let transferred = transfer_point_scalar_field(dataset, isosurface, field.metadata().name())
                    .map_err(|error| SurfaceInspectionError3::InvalidIsosurfaceTransfer {
                        field: field.metadata().name().to_owned(), message: error.to_string(),
                    })?;
                let mesh_indices = isosurface.mesh().triangles().nth(triangle)
                    .ok_or_else(|| SurfaceInspectionError3::InvalidRenderMesh("hit triangle index is out of bounds".to_owned()))?;
                barycentric_value(mesh_indices.map(|id| transferred.values()[id as usize]), weights)
                    .map_err(|_| SurfaceInspectionError3::InvalidPointInterpolation {
                        field: field.metadata().name().to_owned(), triangle,
                    })?
            }
            FieldAssociation::Cell => field.values()[cell_id],
            FieldAssociation::Dataset => field.values()[0],
        };
        result.push(InspectedScalarValue3 {
            name: field.metadata().name().to_owned(),
            units: field.metadata().units().map(str::to_owned),
            association: field.association(),
            value,
        });
    }
    Ok(result)
}

fn barycentric_value(values: [SampleValue; 3], weights: [f64; 3]) -> Result<SampleValue, ()> {
    let mut total = 0.0;
    for (value, weight) in values.into_iter().zip(weights) {
        if !weight.is_finite() || weight < -1.0e-12 || weight > 1.0 + 1.0e-12 {
            return Err(());
        }
        if weight.abs() <= f64::EPSILON { continue; }
        match value {
            SampleValue::Finite(value) => total += value * weight,
            SampleValue::NonFinite(kind) => return Ok(SampleValue::NonFinite(kind)),
        }
        if !total.is_finite() { return Err(()); }
    }
    Ok(SampleValue::Finite(total))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        fields::{FieldMetadata, ScalarField},
        isosurface::extract_tetrahedral_isosurface,
        ray::Ray3,
        sampling::{NonFiniteValue, SampleValue},
        spatial::{WorldPoint3, WorldVector3},
        unstructured::{CellType3, UnstructuredGrid3},
    };

    fn fixture() -> Dataset {
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
            FieldMetadata::new("point_value", Some("K".to_owned())).unwrap(),
            FieldAssociation::Point,
            vec![10.0, 20.0, 30.0, 40.0].into_iter().map(SampleValue::Finite).collect(),
        ).unwrap()).unwrap();
        dataset.add_scalar_field(ScalarField::new(
            FieldMetadata::new("cell_value", None).unwrap(),
            FieldAssociation::Cell,
            vec![SampleValue::Finite(7.0)],
        ).unwrap()).unwrap();
        dataset.add_scalar_field(ScalarField::new(
            FieldMetadata::new("ambient", None).unwrap(),
            FieldAssociation::Dataset,
            vec![SampleValue::Finite(293.15)],
        ).unwrap()).unwrap();
        dataset
    }

    fn ray(origin: WorldPoint3, direction: WorldVector3) -> Ray3 {
        Ray3::new(origin, direction).unwrap()
    }

    fn triangle_ray(mesh: &TriangleMesh3) -> Ray3 {
        let [a, b, c] = mesh.triangles().next().unwrap()
            .map(|id| mesh.vertices()[id as usize].to_array());
        let center = WorldPoint3::new(
            (a[0] + b[0] + c[0]) / 3.0,
            (a[1] + b[1] + c[1]) / 3.0,
            (a[2] + b[2] + c[2]) / 3.0,
        );
        let edge1 = WorldVector3::new(b[0]-a[0], b[1]-a[1], b[2]-a[2]);
        let edge2 = WorldVector3::new(c[0]-a[0], c[1]-a[1], c[2]-a[2]);
        let normal = edge1.cross(edge2);
        ray(center + normal, normal * -1.0)
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 1.0e-10, "expected {expected}, got {actual}");
    }

    #[test]
    fn boundary_hit_resolves_source_face_and_associated_scalar_values() {
        let dataset = fixture();
        let grid = match dataset.geometry() {
            DatasetGeometry::UnstructuredGrid3(grid) => grid,
            _ => unreachable!(),
        };
        let surface = grid.boundary_surface().unwrap().triangulate();
        let hit = inspect_boundary_surface(&dataset, &surface, &triangle_ray(&mesh_from_parts(
            surface.points(), surface.triangles().iter().flat_map(|t| t.vertices()).collect(),
        ).unwrap())).unwrap().unwrap();
        assert_eq!(hit.triangle_index(), 0);
        for weight in hit.barycentric() { assert_close(weight, 1.0 / 3.0); }
        assert!(matches!(hit.entity(), InspectedEntity3::BoundaryFace {
            source_cell_id: 0, source_face_id: _, source_local_face_id: _, source_triangle_index: 0,
        }));
        assert_close(hit.scalar_values()[0].value().as_finite().unwrap(), 20.0);
        assert_eq!(hit.scalar_values()[0].units(), Some("K"));
        assert_eq!(hit.scalar_values()[0].association(), FieldAssociation::Point);
        assert_eq!(hit.scalar_values()[1].value(), SampleValue::Finite(7.0));
        assert_eq!(hit.scalar_values()[2].value(), SampleValue::Finite(293.15));
    }

    #[test]
    fn boundary_inspection_returns_miss_without_fabricating_a_hit() {
        let dataset = fixture();
        let grid = match dataset.geometry() { DatasetGeometry::UnstructuredGrid3(grid) => grid, _ => unreachable!() };
        let surface = grid.boundary_surface().unwrap().triangulate();
        let miss = inspect_boundary_surface(
            &dataset, &surface,
            &ray(WorldPoint3::new(20.0, 20.0, 20.0), WorldVector3::new(1.0, 0.0, 0.0)),
        ).unwrap();
        assert!(miss.is_none());
    }

    #[test]
    fn isosurface_inspection_uses_provenance_for_all_point_fields() {
        let mut dataset = fixture();
        dataset.add_scalar_field(ScalarField::new(
            FieldMetadata::new("extract", None).unwrap(),
            FieldAssociation::Point,
            vec![0.0, 1.0, 0.0, 1.0].into_iter().map(SampleValue::Finite).collect(),
        ).unwrap()).unwrap();
        dataset.add_scalar_field(ScalarField::new(
            FieldMetadata::new("pressure", Some("kPa".to_owned())).unwrap(),
            FieldAssociation::Point,
            vec![0.0, 100.0, 50.0, 150.0].into_iter().map(SampleValue::Finite).collect(),
        ).unwrap()).unwrap();
        let iso = extract_tetrahedral_isosurface(&dataset, "extract", 0.5).unwrap();
        let hit = inspect_isosurface(&dataset, &iso, &triangle_ray(iso.mesh())).unwrap().unwrap();
        assert!(matches!(hit.entity(), InspectedEntity3::IsosurfaceTriangle { source_cell_id: 0 }));
        assert_eq!(hit.scalar_values().len(), 5);
        assert_close(hit.scalar_values()[0].value().as_finite().unwrap(), 25.0);
        assert_eq!(hit.scalar_values()[1].value(), SampleValue::Finite(7.0));
        assert_eq!(hit.scalar_values()[2].value(), SampleValue::Finite(293.15));
        assert_eq!(hit.scalar_values()[3].value(), SampleValue::Finite(0.5));
        assert!(hit.scalar_values()[4].value().as_finite().is_some());
        assert_eq!(hit.scalar_values()[4].units(), Some("kPa"));
    }

    #[test]
    fn non_finite_source_values_remain_classified() {
        let grid = UnstructuredGrid3::new(
            vec![
                WorldPoint3::new(0.0, 0.0, 0.0),
                WorldPoint3::new(1.0, 0.0, 0.0),
                WorldPoint3::new(0.0, 1.0, 0.0),
                WorldPoint3::new(0.0, 0.0, 1.0),
            ],
            vec![CellType3::Tetrahedron4], vec![0, 4], vec![0, 1, 2, 3],
        ).unwrap();
        let mut dataset = Dataset::new(grid);
        dataset.add_scalar_field(ScalarField::new(
            FieldMetadata::new("invalid", None).unwrap(),
            FieldAssociation::Point,
            vec![SampleValue::Finite(1.0), SampleValue::NonFinite(NonFiniteValue::NaN),
                 SampleValue::Finite(2.0), SampleValue::Finite(3.0)],
        ).unwrap()).unwrap();
        let surface = match dataset.geometry() { DatasetGeometry::UnstructuredGrid3(grid) => grid.boundary_surface().unwrap().triangulate(), _ => unreachable!() };
        let hit = inspect_boundary_surface(&dataset, &surface, &triangle_ray(
            &mesh_from_parts(surface.points(), surface.triangles().iter().flat_map(|t| t.vertices()).collect()).unwrap(),
        )).unwrap().unwrap();
        assert!(matches!(hit.scalar_values()[0].value(), SampleValue::NonFinite(NonFiniteValue::NaN)));
    }
}
