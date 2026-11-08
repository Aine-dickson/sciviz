//! Reusable surface representations with explicit source-dataset provenance.
//!
//! Surface vertices and triangles may be reordered, compacted, or expanded for
//! rendering. These mappings preserve the identity of the source point and
//! logical cell represented by each surface vertex and triangle.

use std::{error::Error, fmt};

use crate::{
    geometry::{GeometryError, TriangleMesh3},
    sampling::{GridCellIndex2D, GridSampleIndex2D, SampledSurface3, StructuredGrid2D},
    spatial::WorldPoint3,
    unstructured::UnstructuredGrid3,
    surface_triangulation::TriangulatedSurface3,
};

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
