//! Deterministic triangulation of provenance-preserving boundary surfaces.

use crate::boundary_surface::{BoundaryFaceVertices3, BoundarySurface3};

/// One triangle derived from a source boundary polygon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriangulatedSurfaceTriangle3 {
    vertices: [u32; 3],
    boundary_face_index: usize,
    source_face_index: usize,
    source_cell_index: usize,
    source_local_face_index: usize,
    source_triangle_index: usize,
}

impl TriangulatedSurfaceTriangle3 {
    /// Returns indices into the owning triangulated surface's compact points.
    #[must_use]
    pub const fn vertices(&self) -> [u32; 3] { self.vertices }

    /// Returns the face's index in the input BoundarySurface3 face array.
    #[must_use]
    pub const fn boundary_face_index(&self) -> usize { self.boundary_face_index }

    /// Returns the original M6.4 topology face index retained by M6.5.
    #[must_use]
    pub const fn source_face_index(&self) -> usize { self.source_face_index }

    /// Returns the original unstructured volume cell index.
    #[must_use]
    pub const fn source_cell_index(&self) -> usize { self.source_cell_index }

    /// Returns the source cell's local face-template index.
    #[must_use]
    pub const fn source_local_face_index(&self) -> usize { self.source_local_face_index }

    /// Returns this triangle's ordinal within its source polygon (0 or 1).
    #[must_use]
    pub const fn source_triangle_index(&self) -> usize { self.source_triangle_index }
}

/// A triangle-only surface derived from a BoundarySurface3.
///
/// Point coordinates and source point IDs are copied without reordering.
/// Triangles retain source polygon and volume-cell provenance. Quad diagonals
/// are selected from original source point IDs, not geometric coordinates.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TriangulatedSurface3 {
    points: Vec<crate::spatial::WorldPoint3>,
    source_point_ids: Vec<u32>,
    triangles: Vec<TriangulatedSurfaceTriangle3>,
}

impl TriangulatedSurface3 {
    /// Returns the unchanged compact f64 world-space points.
    #[must_use]
    pub fn points(&self) -> &[crate::spatial::WorldPoint3] { &self.points }

    /// Maps each compact point index to its original unstructured-grid point ID.
    #[must_use]
    pub fn source_point_ids(&self) -> &[u32] { &self.source_point_ids }

    /// Returns triangles in deterministic source-face order.
    #[must_use]
    pub fn triangles(&self) -> &[TriangulatedSurfaceTriangle3] { &self.triangles }

    /// Returns the number of compact points.
    #[must_use]
    pub fn point_count(&self) -> usize { self.points.len() }

    /// Returns the number of generated triangles.
    #[must_use]
    pub fn triangle_count(&self) -> usize { self.triangles.len() }
}

impl BoundarySurface3 {
    /// Triangulates each polygon while preserving source identity and ordering.
    ///
    /// Source triangles pass through unchanged. A quad with local cycle
    /// [v0, v1, v2, v3] is split along whichever diagonal has the
    /// lexicographically smaller pair of sorted original source point IDs.
    /// The generated triangles preserve the source local cycle, but no
    /// outward orientation is certified.
    #[must_use]
    pub fn triangulate(&self) -> TriangulatedSurface3 {
        let triangle_capacity: usize = self.faces().iter().map(|face| match face.vertices() {
            BoundaryFaceVertices3::Triangle(_) => 1,
            BoundaryFaceVertices3::Quad(_) => 2,
        }).sum();
        let mut triangles = Vec::with_capacity(triangle_capacity);

        for (source_face_index, face) in self.faces().iter().enumerate() {
            let source_cell_index = face.source_cell_index();
            let source_local_face_index = face.source_local_face_index();

            let mut push_triangle = |vertices, source_triangle_index| {
                triangles.push(TriangulatedSurfaceTriangle3 {
                    vertices,
                    boundary_face_index: source_face_index,
                    source_face_index: face.source_face_index(),
                    source_cell_index,
                    source_local_face_index,
                    source_triangle_index,
                });
            };

            match face.vertices() {
                BoundaryFaceVertices3::Triangle(vertices) => push_triangle(vertices, 0),
                BoundaryFaceVertices3::Quad([v0, v1, v2, v3]) => {
                    let id02 = canonical_pair(
                        self.source_point_ids()[v0 as usize],
                        self.source_point_ids()[v2 as usize],
                    );
                    let id13 = canonical_pair(
                        self.source_point_ids()[v1 as usize],
                        self.source_point_ids()[v3 as usize],
                    );

                    if id02 < id13 {
                        push_triangle([v0, v1, v2], 0);
                        push_triangle([v0, v2, v3], 1);
                    } else {
                        push_triangle([v0, v1, v3], 0);
                        push_triangle([v1, v2, v3], 1);
                    }
                }
            }
        }

        TriangulatedSurface3 {
            points: self.points().to_vec(),
            source_point_ids: self.source_point_ids().to_vec(),
            triangles,
        }
    }
}

fn canonical_pair(a: u32, b: u32) -> (u32, u32) {
    if a < b { (a, b) } else { (b, a) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        spatial::WorldPoint3,
        unstructured::{CellType3, UnstructuredGrid3},
    };

    fn p(x: f64, y: f64, z: f64) -> WorldPoint3 {
        WorldPoint3::new(x, y, z)
    }

    fn cube_surface() -> BoundarySurface3 {
        let points = vec![
            p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0),
            p(1.0, 1.0, 0.0), p(0.0, 1.0, 0.0),
            p(0.0, 0.0, 1.0), p(1.0, 0.0, 1.0),
            p(1.0, 1.0, 1.0), p(0.0, 1.0, 1.0),
        ];
        let grid = UnstructuredGrid3::new(
            points,
            vec![CellType3::Hexahedron8],
            vec![0, 8],
            vec![0, 1, 2, 3, 4, 5, 6, 7],
        ).unwrap();
        grid.boundary_surface().unwrap()
    }

    #[test]
    fn empty_surface_triangulates_to_empty_output() {
        assert_eq!(BoundarySurface3::default().triangulate(), TriangulatedSurface3::default());
    }

    #[test]
    fn source_triangle_passes_through_with_exact_order_and_provenance() {
        let grid = UnstructuredGrid3::new(
            vec![p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0),
                 p(0.0, 1.0, 0.0), p(0.0, 0.0, 1.0)],
            vec![CellType3::Tetrahedron4],
            vec![0, 4],
            vec![0, 1, 2, 3],
        ).unwrap();
        let source = grid.boundary_surface().unwrap();
        let result = source.triangulate();
        assert_eq!(result.triangle_count(), 4);

        for triangle in result.triangles() {
            let source_face = &source.faces()[triangle.boundary_face_index()];
            assert_eq!(triangle.vertices(), match source_face.vertices() {
                BoundaryFaceVertices3::Triangle(ids) => ids,
                BoundaryFaceVertices3::Quad(_) => panic!("tetrahedron face must be triangular"),
            });
            assert_eq!(triangle.source_cell_index(), source_face.source_cell_index());
            assert_eq!(triangle.source_local_face_index(), source_face.source_local_face_index());
            assert_eq!(triangle.source_triangle_index(), 0);
        }
    }

    #[test]
    fn quad_selects_diagonal_02_for_canonical_low_pair() {
        let source = cube_surface();
        let face = source.faces().iter().find(|face| {
            face.source_local_face_index() == 0
        }).unwrap();
        let BoundaryFaceVertices3::Quad([v0, v1, v2, v3]) = face.vertices() else {
            panic!("hexahedron face must be a quad");
        };
        let result = source.triangulate();
        let first = result.triangles().iter().find(|t| t.source_local_face_index() == 0
            && t.source_triangle_index() == 0).unwrap();
        let second = result.triangles().iter().find(|t| t.source_local_face_index() == 0
            && t.source_triangle_index() == 1).unwrap();
        assert_eq!(first.vertices(), [v0, v1, v2]);
        assert_eq!(second.vertices(), [v0, v2, v3]);
    }

    #[test]
    fn quad_selects_diagonal_13_when_canonical_pair_is_smaller() {
        let source = cube_surface();
        let face = source.faces().iter().find(|face| {
            face.source_local_face_index() == 5
        }).unwrap();
        let BoundaryFaceVertices3::Quad([v0, v1, v2, v3]) = face.vertices() else {
            panic!("hexahedron face must be a quad");
        };
        let result = source.triangulate();
        let first = result.triangles().iter().find(|t| t.source_local_face_index() == 5
            && t.source_triangle_index() == 0).unwrap();
        let second = result.triangles().iter().find(|t| t.source_local_face_index() == 5
            && t.source_triangle_index() == 1).unwrap();
        assert_eq!(first.vertices(), [v0, v1, v3]);
        assert_eq!(second.vertices(), [v1, v2, v3]);
    }

    #[test]
    fn diagonal_policy_reads_original_source_point_ids() {
        let source = cube_surface();
        for face in source.faces() {
            if let BoundaryFaceVertices3::Quad(ids) = face.vertices() {
                let source_ids = source.source_point_ids();
                let pair02 = canonical_pair(source_ids[ids[0] as usize], source_ids[ids[2] as usize]);
                let pair13 = canonical_pair(source_ids[ids[1] as usize], source_ids[ids[3] as usize]);
                let expected = if pair02 < pair13 {
                    [[ids[0], ids[1], ids[2]], [ids[0], ids[2], ids[3]]]
                } else {
                    [[ids[0], ids[1], ids[3]], [ids[1], ids[2], ids[3]]]
                };
                let generated: Vec<_> = source.triangulate().triangles().iter()
                    .filter(|t| t.boundary_face_index() == source.faces().iter()
                        .position(|candidate| std::ptr::eq(candidate, face)).unwrap())
                    .map(TriangulatedSurfaceTriangle3::vertices)
                    .collect();
                assert_eq!(generated, expected);
            }
        }
    }

    #[test]
    fn mixed_tetrahedron_and_hexahedron_faces_preserve_face_order_and_arity() {
        let mut points = vec![
            p(0.0, 0.0, 0.0), p(1.0, 0.0, 0.0),
            p(1.0, 1.0, 0.0), p(0.0, 1.0, 0.0),
            p(0.0, 0.0, 1.0), p(1.0, 0.0, 1.0),
            p(1.0, 1.0, 1.0), p(0.0, 1.0, 1.0),
        ];
        points.extend([p(3.0, 0.0, 0.0), p(4.0, 0.0, 0.0),
                       p(3.0, 1.0, 0.0), p(3.0, 0.0, 1.0)]);
        let grid = UnstructuredGrid3::new(
            points,
            vec![CellType3::Hexahedron8, CellType3::Tetrahedron4],
            vec![0, 8, 12],
            vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        ).unwrap();
        let source = grid.boundary_surface().unwrap();
        let result = source.triangulate();
        let expected_count: usize = source.faces().iter().map(|face| match face.vertices() {
            BoundaryFaceVertices3::Triangle(_) => 1,
            BoundaryFaceVertices3::Quad(_) => 2,
        }).sum();
        assert_eq!(result.triangle_count(), expected_count);

        let mut previous_face = 0;
        for triangle in result.triangles() {
            assert!(triangle.boundary_face_index() >= previous_face);
            previous_face = triangle.boundary_face_index();
            let source_face = &source.faces()[triangle.boundary_face_index()];
            assert_eq!(triangle.source_face_index(), source_face.source_face_index());
            assert_eq!(triangle.source_cell_index(), source_face.source_cell_index());
        }
    }

    #[test]
    fn every_triangle_retains_complete_source_provenance_and_ordinal() {
        let source = cube_surface();
        let result = source.triangulate();
        for triangle in result.triangles() {
            let source_face = &source.faces()[triangle.boundary_face_index()];
            assert_eq!(triangle.source_face_index(), source_face.source_face_index());
            assert_eq!(triangle.source_cell_index(), source_face.source_cell_index());
            assert_eq!(triangle.source_local_face_index(), source_face.source_local_face_index());
            match source_face.vertices() {
                BoundaryFaceVertices3::Triangle(_) => assert_eq!(triangle.source_triangle_index(), 0),
                BoundaryFaceVertices3::Quad(_) => assert!(triangle.source_triangle_index() < 2),
            }
        }
    }

    #[test]
    fn points_and_source_point_mapping_are_preserved_exactly() {
        let source = cube_surface();
        let result = source.triangulate();
        assert_eq!(result.points(), source.points());
        assert_eq!(result.source_point_ids(), source.source_point_ids());
        assert_eq!(result.point_count(), source.point_count());
    }

    #[test]
    fn repeated_triangulation_is_deterministic_and_does_not_mutate_source() {
        let source = cube_surface();
        let before = source.clone();
        assert_eq!(source.triangulate(), source.triangulate());
        assert_eq!(source, before);
    }

    #[test]
    fn degenerate_quad_is_triangulated_without_geometric_filtering() {
        let grid = UnstructuredGrid3::new(
            vec![p(0.0, 0.0, 0.0); 8],
            vec![CellType3::Hexahedron8],
            vec![0, 8],
            vec![0, 1, 2, 3, 4, 5, 6, 7],
        ).unwrap();
        let source = grid.boundary_surface().unwrap();
        assert_eq!(source.triangulate().triangle_count(), 12);
    }
}
