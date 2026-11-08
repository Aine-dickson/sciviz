//! Compact, provenance-preserving surfaces extracted from unstructured volumes.

use std::{collections::BTreeMap, error::Error, fmt};

use crate::{
    spatial::WorldPoint3,
    unstructured::UnstructuredGrid3,
    unstructured_topology::{VolumeFaceClassification, VolumeFaceVertices3},
};

/// Polygon connectivity into a `BoundarySurface3` compact point array.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundaryFaceVertices3 {
    /// A triangular boundary face.
    Triangle([u32; 3]),
    /// A quadrilateral boundary face; no triangulation is implied.
    Quad([u32; 4]),
}

/// One extracted boundary polygon and its source-topology provenance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundarySurfaceFace3 {
    vertices: BoundaryFaceVertices3,
    source_face_index: usize,
    source_cell_index: usize,
    source_local_face_index: usize,
}

impl BoundarySurfaceFace3 {
    /// Returns compact point indices in the source incidence's local cycle.
    #[must_use]
    pub const fn vertices(&self) -> BoundaryFaceVertices3 {
        self.vertices
    }

    /// Returns the face's index in the deterministic M6.4 topology face array.
    #[must_use]
    pub const fn source_face_index(&self) -> usize {
        self.source_face_index
    }

    /// Returns the source volume cell index.
    #[must_use]
    pub const fn source_cell_index(&self) -> usize {
        self.source_cell_index
    }

    /// Returns the source cell's local face-template index.
    #[must_use]
    pub const fn source_local_face_index(&self) -> usize {
        self.source_local_face_index
    }
}

/// A compact surface derived from an unstructured volume grid.
///
/// Points are stored once in ascending source point-ID order. Face indices
/// address this compact point array; `source_point_ids` maps each compact
/// point back to the original grid. Faces retain triangles/quads and source
/// local cycles. No outward-orientation or geometric-validity claim is made.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BoundarySurface3 {
    points: Vec<WorldPoint3>,
    source_point_ids: Vec<u32>,
    faces: Vec<BoundarySurfaceFace3>,
}

impl BoundarySurface3 {
    /// Returns compact f64 world-space points.
    #[must_use]
    pub fn points(&self) -> &[WorldPoint3] {
        &self.points
    }

    /// Maps each compact point index to its original source point ID.
    #[must_use]
    pub fn source_point_ids(&self) -> &[u32] {
        &self.source_point_ids
    }

    /// Returns extracted boundary polygons in deterministic source-topology order.
    #[must_use]
    pub fn faces(&self) -> &[BoundarySurfaceFace3] {
        &self.faces
    }

    /// Returns the number of compact points referenced by boundary faces.
    #[must_use]
    pub fn point_count(&self) -> usize {
        self.points.len()
    }

    /// Returns the number of extracted boundary polygons.
    #[must_use]
    pub fn face_count(&self) -> usize {
        self.faces.len()
    }
}

/// Errors that can occur while building compact boundary-surface connectivity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundarySurfaceError {
    /// A compact point index cannot be represented by the u32 connectivity format.
    PointIndexOverflow,
}

impl fmt::Display for BoundarySurfaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PointIndexOverflow => {
                write!(f, "boundary surface point index exceeds the u32 connectivity range")
            }
        }
    }
}

impl Error for BoundarySurfaceError {}

impl UnstructuredGrid3 {
    /// Extracts a compact surface from faces incident to exactly one volume cell.
    ///
    /// Interior and non-manifold faces are excluded. Referenced source points
    /// are compacted once in ascending source point-ID order. Face ordering
    /// follows `volume_topology()`; polygon vertex order follows the sole
    /// source incidence. Quads remain quads.
    pub fn boundary_surface(&self) -> Result<BoundarySurface3, BoundarySurfaceError> {
        let topology = self.volume_topology();
        let boundary_faces: Vec<_> = topology
            .faces()
            .iter()
            .enumerate()
            .filter(|(_, face)| face.classification() == VolumeFaceClassification::Boundary)
            .collect();

        let mut used_source_ids = BTreeMap::<u32, ()>::new();
        for (_, face) in &boundary_faces {
            for &source_id in face.vertices().as_slice() {
                used_source_ids.insert(source_id, ());
            }
        }

        let mut source_point_ids = Vec::with_capacity(used_source_ids.len());
        let mut points = Vec::with_capacity(used_source_ids.len());
        let mut compact_ids = BTreeMap::<u32, u32>::new();

        for source_id in used_source_ids.keys().copied() {
            let compact_index = u32::try_from(source_point_ids.len())
                .map_err(|_| BoundarySurfaceError::PointIndexOverflow)?;
            compact_ids.insert(source_id, compact_index);
            source_point_ids.push(source_id);
            points.push(self.points()[source_id as usize]);
        }

        let mut faces = Vec::with_capacity(boundary_faces.len());
        for (source_face_index, face) in boundary_faces {
            let incidence = &face.incidences()[0];
            let remap = |source_id: u32| {
                *compact_ids
                    .get(&source_id)
                    .expect("all boundary face points were collected")
            };
            let vertices = match incidence.vertices() {
                VolumeFaceVertices3::Triangle(ids) => BoundaryFaceVertices3::Triangle([
                    remap(ids[0]), remap(ids[1]), remap(ids[2]),
                ]),
                VolumeFaceVertices3::Quad(ids) => BoundaryFaceVertices3::Quad([
                    remap(ids[0]), remap(ids[1]), remap(ids[2]), remap(ids[3]),
                ]),
            };
            faces.push(BoundarySurfaceFace3 {
                vertices,
                source_face_index,
                source_cell_index: incidence.source_cell_index(),
                source_local_face_index: incidence.local_face_index(),
            });
        }

        Ok(BoundarySurface3 {
            points,
            source_point_ids,
            faces,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unstructured::CellType3;

    fn p(x: f64, y: f64, z: f64) -> WorldPoint3 {
        WorldPoint3::new(x, y, z)
    }

    fn tetra_points() -> Vec<WorldPoint3> {
        vec![p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(0.0, 1.0, 0.0), p(0.0, 0.0, 1.0)]
    }

    fn cube_points() -> Vec<WorldPoint3> {
        vec![
            p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0), p(1.0, 1.0, 0.0), p(0.0, 1.0, 0.0),
            p(0.0, 0.0, 1.0), p(1.0, 0.0, 1.0), p(1.0, 1.0, 1.0), p(0.0, 1.0, 1.0),
        ]
    }

    fn grid(points: Vec<WorldPoint3>, cells: &[(CellType3, &[u32])]) -> UnstructuredGrid3 {
        let mut types = Vec::new();
        let mut offsets = vec![0];
        let mut connectivity = Vec::new();
        for (kind, ids) in cells {
            types.push(*kind);
            connectivity.extend_from_slice(ids);
            offsets.push(connectivity.len());
        }
        UnstructuredGrid3::new(points, types, offsets, connectivity).unwrap()
    }

    fn compact_ids(face: &BoundarySurfaceFace3) -> Vec<u32> {
        match face.vertices() {
            BoundaryFaceVertices3::Triangle(ids) => ids.to_vec(),
            BoundaryFaceVertices3::Quad(ids) => ids.to_vec(),
        }
    }

    #[test]
    fn tetrahedron_extracts_four_triangles_and_compact_points() {
        let grid = grid(tetra_points(), &[(CellType3::Tetrahedron4, &[0, 1, 2, 3])]);
        let surface = grid.boundary_surface().unwrap();
        assert_eq!(surface.face_count(), 4);
        assert_eq!(surface.point_count(), 4);
        assert!(surface.faces().iter().all(|f| matches!(f.vertices(), BoundaryFaceVertices3::Triangle(_))));
        assert_eq!(surface.source_point_ids(), &[0, 1, 2, 3]);
    }

    #[test]
    fn hexahedron_extracts_six_quads_without_triangulation() {
        let grid = grid(cube_points(), &[(CellType3::Hexahedron8, &[0, 1, 2, 3, 4, 5, 6, 7])]);
        let surface = grid.boundary_surface().unwrap();
        assert_eq!(surface.face_count(), 6);
        assert_eq!(surface.point_count(), 8);
        assert!(surface.faces().iter().all(|f| matches!(f.vertices(), BoundaryFaceVertices3::Quad(_))));
    }

    #[test]
    fn shared_tetrahedron_face_is_excluded() {
        let mut points = tetra_points();
        points.push(p(0.0, 0.0, -1.0));
        let grid = grid(points, &[
            (CellType3::Tetrahedron4, &[0, 1, 2, 3]),
            (CellType3::Tetrahedron4, &[0, 2, 1, 4]),
        ]);
        let surface = grid.boundary_surface().unwrap();
        assert_eq!(surface.face_count(), 6);
        assert!(surface.faces().iter().all(|f| f.source_face_index() < 7));
        assert_eq!(surface.faces().iter().filter(|f| compact_ids(f).len() == 3).count(), 6);
    }

    #[test]
    fn adjacent_hexahedra_extract_ten_quads() {
        let mut points = cube_points();
        points.extend([p(2.0, 0.0, 0.0), p(2.0, 1.0, 0.0), p(2.0, 0.0, 1.0), p(2.0, 1.0, 1.0)]);
        let grid = grid(points, &[
            (CellType3::Hexahedron8, &[0, 1, 2, 3, 4, 5, 6, 7]),
            (CellType3::Hexahedron8, &[1, 8, 9, 2, 5, 10, 11, 6]),
        ]);
        let surface = grid.boundary_surface().unwrap();
        assert_eq!(surface.face_count(), 10);
        assert_eq!(surface.point_count(), 12);
    }

    #[test]
    fn mixed_volume_cells_preserve_polygon_arities() {
        let mut points = cube_points();
        points.extend([p(3.0, 0.0, 0.0), p(4.0, 0.0, 0.0), p(3.0, 1.0, 0.0), p(3.0, 0.0, 1.0)]);
        let grid = grid(points, &[
            (CellType3::Tetrahedron4, &[8, 9, 10, 11]),
            (CellType3::Hexahedron8, &[0, 1, 2, 3, 4, 5, 6, 7]),
        ]);
        let surface = grid.boundary_surface().unwrap();
        assert_eq!(surface.face_count(), 10);
        assert_eq!(surface.faces().iter().filter(|f| matches!(f.vertices(), BoundaryFaceVertices3::Triangle(_))).count(), 4);
        assert_eq!(surface.faces().iter().filter(|f| matches!(f.vertices(), BoundaryFaceVertices3::Quad(_))).count(), 6);
    }

    #[test]
    fn non_manifold_faces_are_excluded() {
        let mut points = tetra_points();
        points.extend([p(0.0, 0.0, -1.0), p(0.0, 0.0, 2.0)]);
        let grid = grid(points, &[
            (CellType3::Tetrahedron4, &[0, 1, 2, 3]),
            (CellType3::Tetrahedron4, &[0, 2, 1, 4]),
            (CellType3::Tetrahedron4, &[0, 1, 2, 5]),
        ]);
        let surface = grid.boundary_surface().unwrap();
        assert_eq!(surface.face_count(), 9);
        assert!(surface.faces().iter().all(|f| f.source_cell_index() < 3));
    }

    #[test]
    fn lower_dimensional_cells_are_ignored() {
        let grid = grid(tetra_points(), &[
            (CellType3::Line2, &[0, 1]),
            (CellType3::Triangle3, &[0, 1, 2]),
            (CellType3::Quad4, &[0, 1, 2, 3]),
        ]);
        let surface = grid.boundary_surface().unwrap();
        assert_eq!(surface.point_count(), 0);
        assert_eq!(surface.face_count(), 0);
    }

    #[test]
    fn compact_points_are_unique_and_sorted_by_source_id() {
        let mut points = tetra_points();
        points.push(p(9.0, 9.0, 9.0)); // unreferenced
        let grid = grid(points, &[(CellType3::Tetrahedron4, &[3, 1, 0, 2])]);
        let surface = grid.boundary_surface().unwrap();
        assert_eq!(surface.source_point_ids(), &[0, 1, 2, 3]);
        assert_eq!(surface.points(), &[p(0.0,0.0,0.0), p(1.0,0.0,0.0), p(0.0,1.0,0.0), p(0.0,0.0,1.0)]);
        let mut used = Vec::new();
        for face in surface.faces() { used.extend(compact_ids(face)); }
        used.sort_unstable();
        used.dedup();
        assert_eq!(used, vec![0, 1, 2, 3]);
    }

    #[test]
    fn face_provenance_and_local_cycle_are_preserved() {
        let grid = grid(tetra_points(), &[(CellType3::Tetrahedron4, &[0, 1, 2, 3])]);
        let topology = grid.volume_topology();
        let surface = grid.boundary_surface().unwrap();
        for face in surface.faces() {
            let source = &topology.faces()[face.source_face_index()];
            let incidence = &source.incidences()[0];
            assert_eq!(face.source_cell_index(), incidence.source_cell_index());
            assert_eq!(face.source_local_face_index(), incidence.local_face_index());
            let expected: Vec<_> = incidence.vertices().as_slice().iter()
                .map(|id| surface.source_point_ids().iter().position(|source_id| source_id == id).unwrap() as u32)
                .collect();
            assert_eq!(compact_ids(face), expected);
        }
    }

    #[test]
    fn repeated_extraction_is_deterministic_and_does_not_mutate_grid() {
        let grid = grid(tetra_points(), &[(CellType3::Tetrahedron4, &[0, 1, 2, 3])]);
        let before = grid.clone();
        assert_eq!(grid.boundary_surface().unwrap(), grid.boundary_surface().unwrap());
        assert_eq!(grid, before);
    }

    #[test]
    fn coincident_coordinates_with_distinct_ids_remain_distinct() {
        let mut points = tetra_points();
        points.extend([p(0.0,0.0,0.0), p(1.0,0.0,0.0), p(0.0,1.0,0.0), p(0.0,0.0,-1.0)]);
        let grid = grid(points, &[
            (CellType3::Tetrahedron4, &[0, 1, 2, 3]),
            (CellType3::Tetrahedron4, &[4, 5, 6, 7]),
        ]);
        let surface = grid.boundary_surface().unwrap();
        assert_eq!(surface.point_count(), 8);
        assert_eq!(surface.source_point_ids(), &[0, 1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn empty_and_non_volume_grids_produce_empty_surface() {
        assert_eq!(UnstructuredGrid3::default().boundary_surface().unwrap(), BoundarySurface3::default());
        let points_only = UnstructuredGrid3::new(tetra_points(), vec![], vec![0], vec![]).unwrap();
        assert_eq!(points_only.boundary_surface().unwrap(), BoundarySurface3::default());
    }

    #[test]
    fn geometrically_degenerate_volume_cells_are_retained() {
        let points = vec![p(0.0,0.0,0.0), p(1.0,0.0,0.0), p(2.0,0.0,0.0), p(3.0,0.0,0.0)];
        let grid = grid(points, &[(CellType3::Tetrahedron4, &[0, 1, 2, 3])]);
        let surface = grid.boundary_surface().unwrap();
        assert_eq!(surface.face_count(), 4);
        assert_eq!(surface.point_count(), 4);
    }
}
