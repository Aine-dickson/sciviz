//! Structured two-dimensional scalar grids and sampled graph-surface generation.
use std::{error::Error, fmt};
use crate::{
    geometry::{GeometryError, TriangleMesh3},
    ray::{intersect_mesh, Ray3},
    spatial::WorldPoint3,
};
use super::{SampleValue, ScalarSamples2D};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GridSampleIndex2D { x: usize, y: usize }
impl GridSampleIndex2D {
    #[must_use] pub const fn new(x: usize, y: usize) -> Self { Self { x, y } }
    #[must_use] pub const fn x(self) -> usize { self.x }
    #[must_use] pub const fn y(self) -> usize { self.y }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GridCellIndex2D { x: usize, y: usize }
impl GridCellIndex2D {
    #[must_use] pub const fn new(x: usize, y: usize) -> Self { Self { x, y } }
    #[must_use] pub const fn x(self) -> usize { self.x }
    #[must_use] pub const fn y(self) -> usize { self.y }
}

/// Structured grid owning the original samples without duplicating values.
#[derive(Clone, Debug, PartialEq)]
pub struct StructuredGrid2D { samples: ScalarSamples2D }
impl StructuredGrid2D {
    #[must_use] pub const fn from_samples(samples: ScalarSamples2D) -> Self { Self { samples } }
    #[must_use] pub const fn x_sample_count(&self) -> usize { self.samples.x_sample_count() }
    #[must_use] pub const fn y_sample_count(&self) -> usize { self.samples.y_sample_count() }
    #[must_use] pub fn sample_count(&self) -> usize { self.samples.len() }
    #[must_use] pub fn sample(&self, x: usize, y: usize) -> Option<&super::ScalarSample2D> { self.samples.get(x, y) }
    #[must_use] pub const fn samples(&self) -> &ScalarSamples2D { &self.samples }
    #[must_use] pub fn into_samples(self) -> ScalarSamples2D { self.samples }

    /// Creates a graph-surface mesh at world coordinates (x, scalar_value, y).
    /// The sampled domain maps to the world XZ plane; scalar values map to +Y.
    /// A cell emits both triangles only when all four corners are finite.
    /// Finite vertices are retained even when not referenced by a triangle.
    /// The lower-left to upper-right diagonal gives +Y-facing winding.
    pub fn to_surface_mesh(&self) -> Result<SampledSurface3, SurfaceGenerationError> {
        let count = self.sample_count();
        let mut source_to_vertex = Vec::new();
        reserve(&mut source_to_vertex, count)?;
        source_to_vertex.resize(count, None);

        let finite_count = self.samples.as_slice().iter().filter(|s| s.value().is_finite()).count();
        let mut vertices = Vec::new();
        reserve(&mut vertices, finite_count)?;
        let mut vertex_sources = Vec::new();
        reserve(&mut vertex_sources, finite_count)?;

        for sample in self.samples.as_slice() {
            let SampleValue::Finite(height) = sample.value() else { continue };
            let id = u32::try_from(vertices.len()).map_err(|_| SurfaceGenerationError::VertexIndexOverflow {
                source: GridSampleIndex2D::new(sample.x_index(), sample.y_index()),
            })?;
            let source = sample.y_index() * self.x_sample_count() + sample.x_index();
            source_to_vertex[source] = Some(id);
            vertices.push(WorldPoint3::new(sample.x(), height, sample.y()));
            vertex_sources.push(GridSampleIndex2D::new(sample.x_index(), sample.y_index()));
        }

        let cells = (self.x_sample_count() - 1).checked_mul(self.y_sample_count() - 1)
            .ok_or(SurfaceGenerationError::AllocationFailed { requested_items: usize::MAX })?;
        let index_capacity = cells.checked_mul(6)
            .ok_or(SurfaceGenerationError::AllocationFailed { requested_items: usize::MAX })?;
        let mut indices = Vec::new();
        reserve(&mut indices, index_capacity)?;
        let mut triangle_sources = Vec::new();
        reserve(&mut triangle_sources, cells.checked_mul(2)
            .ok_or(SurfaceGenerationError::AllocationFailed { requested_items: usize::MAX })?)?;

        let nx = self.x_sample_count();
        for y in 0..self.y_sample_count() - 1 {
            for x in 0..nx - 1 {
                let corners = [
                    source_to_vertex[y * nx + x],
                    source_to_vertex[y * nx + x + 1],
                    source_to_vertex[(y + 1) * nx + x],
                    source_to_vertex[(y + 1) * nx + x + 1],
                ];
                let [Some(ll), Some(lr), Some(ul), Some(ur)] = corners else { continue };
                indices.extend_from_slice(&[ll, ur, lr, ll, ul, ur]);
                triangle_sources.push(GridCellIndex2D::new(x, y));
                triangle_sources.push(GridCellIndex2D::new(x, y));
            }
        }

        let mesh = TriangleMesh3::new(vertices, indices)
            .map_err(SurfaceGenerationError::InvalidGeneratedMesh)?;
        Ok(SampledSurface3 { mesh, vertex_sources, triangle_sources })
    }
}
fn reserve<T>(items: &mut Vec<T>, count: usize) -> Result<(), SurfaceGenerationError> {
    items.try_reserve_exact(count)
        .map_err(|_| SurfaceGenerationError::AllocationFailed { requested_items: count })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceGenerationError {
    VertexIndexOverflow { source: GridSampleIndex2D },
    AllocationFailed { requested_items: usize },
    InvalidGeneratedMesh(GeometryError),
}
impl fmt::Display for SurfaceGenerationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VertexIndexOverflow { source } => write!(f, "grid sample ({}, {}) exceeds u32 mesh index capacity", source.x(), source.y()),
            Self::AllocationFailed { requested_items } => write!(f, "could not allocate surface storage for {requested_items} items"),
            Self::InvalidGeneratedMesh(error) => write!(f, "generated surface mesh is invalid: {error}"),
        }
    }
}
impl Error for SurfaceGenerationError {}

#[derive(Clone, Debug, PartialEq)]
pub struct SampledSurface3 {
    mesh: TriangleMesh3,
    vertex_sources: Vec<GridSampleIndex2D>,
    triangle_sources: Vec<GridCellIndex2D>,
}
impl SampledSurface3 {
    #[must_use] pub const fn mesh(&self) -> &TriangleMesh3 { &self.mesh }
    #[must_use] pub fn vertex_sources(&self) -> &[GridSampleIndex2D] { &self.vertex_sources }
    #[must_use] pub fn triangle_sources(&self) -> &[GridCellIndex2D] { &self.triangle_sources }
    #[must_use] pub fn source_for_vertex(&self, i: usize) -> Option<GridSampleIndex2D> { self.vertex_sources.get(i).copied() }
    #[must_use] pub fn source_for_triangle(&self, i: usize) -> Option<GridCellIndex2D> { self.triangle_sources.get(i).copied() }
    #[must_use] pub fn vertex_count(&self) -> usize { self.mesh.vertex_count() }
    #[must_use] pub fn triangle_count(&self) -> usize { self.mesh.triangle_count() }

    /// Intersects a world-space ray and maps the nearest hit back to the
    /// generated surface's source cell and continuous structured-grid space.
    ///
    /// Domain coordinates and scalar value are interpolated over the hit
    /// triangle. This is the piecewise-linear surface represented by the mesh,
    /// not a fresh evaluation of the original mathematical function.
    #[must_use]
    pub fn inspect_ray(&self, ray: &Ray3) -> Option<SurfaceInspection3> {
        let hit = intersect_mesh(ray, &self.mesh)?;
        let triangle_index = hit.triangle_index();
        let source_cell = self.source_for_triangle(triangle_index)?;
        let triangle = self.mesh.triangles().nth(triangle_index)?;
        let weights = hit.barycentric();
        let sources = [
            self.source_for_vertex(triangle[0] as usize)?,
            self.source_for_vertex(triangle[1] as usize)?,
            self.source_for_vertex(triangle[2] as usize)?,
        ];
        let grid_coordinates = [
            weights[0] * sources[0].x() as f64
                + weights[1] * sources[1].x() as f64
                + weights[2] * sources[2].x() as f64,
            weights[0] * sources[0].y() as f64
                + weights[1] * sources[1].y() as f64
                + weights[2] * sources[2].y() as f64,
        ];
        let position = hit.position();
        let world = position.to_array();
        Some(SurfaceInspection3 {
            distance: hit.distance(),
            triangle_index,
            source_cell,
            barycentric: weights,
            world_position: position,
            domain_coordinates: [world[0], world[2]],
            grid_coordinates,
            scalar_value: world[1],
        })
    }
}

/// Scientific interpretation of a ray hit on a sampled graph surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceInspection3 {
    distance: f64,
    triangle_index: usize,
    source_cell: GridCellIndex2D,
    barycentric: [f64; 3],
    world_position: WorldPoint3,
    domain_coordinates: [f64; 2],
    grid_coordinates: [f64; 2],
    scalar_value: f64,
}
impl SurfaceInspection3 {
    #[must_use] pub const fn distance(&self) -> f64 { self.distance }
    #[must_use] pub const fn triangle_index(&self) -> usize { self.triangle_index }
    #[must_use] pub const fn source_cell(&self) -> GridCellIndex2D { self.source_cell }
    #[must_use] pub const fn barycentric(&self) -> [f64; 3] { self.barycentric }
    #[must_use] pub const fn world_position(&self) -> WorldPoint3 { self.world_position }
    /// Continuous source-domain coordinates in the original X/Y domain order.
    #[must_use] pub const fn domain_coordinates(&self) -> [f64; 2] { self.domain_coordinates }
    /// Continuous source-grid sample coordinates, where integer pairs are samples.
    #[must_use] pub const fn grid_coordinates(&self) -> [f64; 2] { self.grid_coordinates }
    /// Piecewise-linear scalar value represented by the intersected triangle.
    #[must_use] pub const fn scalar_value(&self) -> f64 { self.scalar_value }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sampling::{sample_scalar_2d, Domain1D, Domain2D, SamplingSpec2D};

    fn grid<F: FnMut(f64, f64) -> f64>(nx: usize, ny: usize, f: F) -> StructuredGrid2D {
        let domain = Domain2D::new(Domain1D::new(0.0, (nx - 1) as f64).unwrap(),
                                   Domain1D::new(0.0, (ny - 1) as f64).unwrap());
        StructuredGrid2D::from_samples(sample_scalar_2d(domain, SamplingSpec2D::new(nx, ny).unwrap(), f).unwrap())
    }

    #[test]
    fn grid_preserves_dimensions_and_samples() {
        let g = grid(2, 2, |x, y| x + y);
        assert_eq!((g.x_sample_count(), g.y_sample_count(), g.sample_count()), (2, 2, 4));
        assert_eq!(g.sample(1, 1).unwrap().value(), SampleValue::Finite(2.0));
        assert!(g.sample(2, 1).is_none());
    }

    #[test]
    fn finite_grid_generates_two_triangles_per_cell() {
        let surface = grid(3, 2, |x, y| x + y).to_surface_mesh().unwrap();
        assert_eq!((surface.vertex_count(), surface.triangle_count()), (6, 4));
        assert_eq!(surface.mesh().indices(), &[0, 4, 1, 0, 3, 4, 1, 5, 2, 1, 4, 5]);
        assert_eq!(surface.mesh().vertices()[5].to_array(), [2.0, 3.0, 1.0]);
    }

    #[test]
    fn triangle_winding_faces_positive_world_y() {
        let surface = grid(2, 2, |_, _| 0.0).to_surface_mesh().unwrap();
        let v = surface.mesh().vertices();
        for t in surface.mesh().triangles() {
            let a = v[t[0] as usize].to_vector();
            let b = v[t[1] as usize].to_vector();
            let c = v[t[2] as usize].to_vector();
            assert!((b - a).cross(c - a).y > 0.0);
        }
    }

    #[test]
    fn non_finite_corner_omits_whole_cell_and_keeps_finite_vertices() {
        let surface = grid(2, 2, |x, y| if x == 1.0 && y == 1.0 { f64::NAN } else { x + y })
            .to_surface_mesh().unwrap();
        assert_eq!((surface.vertex_count(), surface.triangle_count()), (3, 0));
    }

    #[test]
    fn invalid_sample_prevents_adjacent_cells_from_bridging_hole() {
        let surface = grid(3, 2, |x, y| if x == 1.0 && y == 0.0 { f64::INFINITY } else { x + y })
            .to_surface_mesh().unwrap();
        assert_eq!((surface.vertex_count(), surface.triangle_count()), (5, 0));
    }

    #[test]
    fn vertex_and_triangle_indices_map_back_to_source_grid() {
        let surface = grid(2, 2, |x, y| x + y).to_surface_mesh().unwrap();
        assert_eq!(surface.source_for_vertex(3), Some(GridSampleIndex2D::new(1, 1)));
        assert_eq!(surface.source_for_triangle(0), Some(GridCellIndex2D::new(0, 0)));
        assert_eq!(surface.source_for_triangle(2), None);
    }

    #[test]
    fn all_non_finite_grid_generates_empty_mesh() {
        let surface = grid(2, 2, |_, _| f64::NEG_INFINITY).to_surface_mesh().unwrap();
        assert_eq!((surface.vertex_count(), surface.triangle_count()), (0, 0));
    }

    #[test]
    fn every_cell_triangle_retains_its_source_cell() {
        let surface = grid(3, 3, |x, y| x * x + y * y).to_surface_mesh().unwrap();
        assert_eq!(surface.triangle_count(), 8);
        assert_eq!(surface.triangle_sources()[0], GridCellIndex2D::new(0, 0));
        assert_eq!(surface.triangle_sources()[7], GridCellIndex2D::new(1, 1));
    }


    #[test]
    fn inspection_maps_ray_hit_to_domain_grid_and_scalar_data() {
        use crate::ray::Ray3;
        let surface = grid(2, 2, |x, y| x + 2.0 * y).to_surface_mesh().unwrap();
        let ray = Ray3::new(
            WorldPoint3::new(0.75, 5.0, 0.25),
            crate::spatial::WorldVector3::new(0.0, -1.0, 0.0),
        ).unwrap();
        let hit = surface.inspect_ray(&ray).unwrap();
        assert_eq!(hit.triangle_index(), 0);
        assert_eq!(hit.source_cell(), GridCellIndex2D::new(0, 0));
        assert!((hit.domain_coordinates()[0] - 0.75).abs() < 1.0e-12);
        assert!((hit.domain_coordinates()[1] - 0.25).abs() < 1.0e-12);
        assert!((hit.grid_coordinates()[0] - 0.75).abs() < 1.0e-12);
        assert!((hit.grid_coordinates()[1] - 0.25).abs() < 1.0e-12);
        assert!((hit.scalar_value() - 1.25).abs() < 1.0e-12);
        assert!((hit.world_position().to_array()[1] - hit.scalar_value()).abs() < 1.0e-12);
    }

    #[test]
    fn inspection_reports_no_hit_for_rays_outside_the_surface() {
        use crate::ray::Ray3;
        let surface = grid(2, 2, |x, y| x + y).to_surface_mesh().unwrap();
        let ray = Ray3::new(
            WorldPoint3::new(5.0, 5.0, 5.0),
            crate::spatial::WorldVector3::new(0.0, -1.0, 0.0),
        ).unwrap();
        assert!(surface.inspect_ray(&ray).is_none());
    }

    #[test]
    fn generated_vertices_preserve_f64_values() {
        let surface = grid(2, 2, |_, _| 1.0e100).to_surface_mesh().unwrap();
        assert_eq!(surface.mesh().vertices()[0].to_array()[1], 1.0e100);
    }
}
