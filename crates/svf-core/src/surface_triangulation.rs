//! Deterministic triangulation of provenance-preserving boundary surfaces.

use crate::boundary_surface::{BoundaryFaceVertices3, BoundarySurface3};

/// One triangle derived from a source boundary polygon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriangulatedSurfaceTriangle3 {
    vertices: [u32; 3],
    source_face_index: usize,
    source_cell_index: usize,
    source_local_face_index: usize,
    source_triangle_index: usize,
}

impl TriangulatedSurfaceTriangle3 {
    /// Returns indices into the owning triangulated surface's compact points.
    #[must_use]
    pub const fn vertices(&self) -> [u32; 3] { self.vertices }

    /// Returns the source boundary-surface face index.
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
                    source_face_index,
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
        boundary_surface::{BoundarySurfaceFace3, BoundarySurface3},
        spatial::WorldPoint3,
    };

    fn p(x: f64, y: f64, z: f64) -> WorldPoint3 { WorldPoint3::new(x, y, z) }

    fn face(vertices: BoundaryFaceVertices3, source_face: usize) -> BoundarySurfaceFace3 {
        BoundarySurfaceFace3 {
            vertices,
            source_face_index: source_face,
            source_cell_index: source_face + 10,
            source_local_face_index: source_face + 20,
        }
    }

    fn surface(source_point_ids: Vec<u32>, faces: Vec<BoundarySurfaceFace3>) -> BoundarySurface3 {
        let points = source_point_ids.iter()
            .map(|&id| p(f64::from(id), 0.0, 0.0))
            .collect();
        BoundarySurface3 { points, source_point_ids, faces }
    }

    #[test]
    fn empty_surface_triangulates_to_empty_output() {
        assert_eq!(BoundarySurface3::default().triangulate(), TriangulatedSurface3::default());
    }

    #[test]
    fn source_triangle_passes_through_with_exact_order_and_provenance() {
        let source = surface(vec![40, 10, 30],
            vec![face(BoundaryFaceVertices3::Triangle([2, 0, 1]), 7)]);
        let result = source.triangulate();
        assert_eq!(result.triangle_count(), 1);
        let triangle = result.triangles()[0];
        assert_eq!(triangle.vertices(), [2, 0, 1]);
        assert_eq!(triangle.source_face_index(), 0);
        assert_eq!(triangle.source_cell_index(), 10);
        assert_eq!(triangle.source_local_face_index(), 20);
        assert_eq!(triangle.source_triangle_index(), 0);
    }

    #[test]
    fn quad_selects_diagonal_02_when_its_source_id_pair_is_smaller() {
        let source = surface(vec![10, 20, 30, 40],
            vec![face(BoundaryFaceVertices3::Quad([0, 1, 2, 3]), 0)]);
        let result = source.triangulate();
        assert_eq!(result.triangles()[0].vertices(), [0, 1, 2]);
        assert_eq!(result.triangles()[1].vertices(), [0, 2, 3]);
    }

    #[test]
    fn quad_selects_diagonal_13_when_its_source_id_pair_is_smaller() {
        let source = surface(vec![10, 40, 20, 30],
            vec![face(BoundaryFaceVertices3::Quad([0, 1, 2, 3]), 0)]);
        let result = source.triangulate();
        assert_eq!(result.triangles()[0].vertices(), [0, 1, 3]);
        assert_eq!(result.triangles()[1].vertices(), [1, 2, 3]);
    }

    #[test]
    fn diagonal_selection_uses_original_source_ids_not_compact_ids() {
        // Compact pair 0-2 is smaller, but source pair (10, 90) is larger
        // than source pair (20, 30).
        let source = surface(vec![10, 20, 90, 30],
            vec![face(BoundaryFaceVertices3::Quad([0, 1, 2, 3]), 0)]);
        let result = source.triangulate();
        assert_eq!(result.triangles()[0].vertices(), [0, 1, 3]);
        assert_eq!(result.triangles()[1].vertices(), [1, 2, 3]);
    }

    #[test]
    fn both_split_patterns_preserve_quad_local_cycle_order() {
        let diagonal_02 = surface(vec![10, 20, 30, 40],
            vec![face(BoundaryFaceVertices3::Quad([0, 1, 2, 3]), 0)]).triangulate();
        assert_eq!(diagonal_02.triangles()[0].vertices(), [0, 1, 2]);
        assert_eq!(diagonal_02.triangles()[1].vertices(), [0, 2, 3]);

        let diagonal_13 = surface(vec![10, 40, 20, 30],
            vec![face(BoundaryFaceVertices3::Quad([0, 1, 2, 3]), 0)]).triangulate();
        assert_eq!(diagonal_13.triangles()[0].vertices(), [0, 1, 3]);
        assert_eq!(diagonal_13.triangles()[1].vertices(), [1, 2, 3]);
    }

    #[test]
    fn mixed_faces_keep_source_order_and_triangle_ordinals() {
        let source = surface(vec![0, 1, 2, 3, 4], vec![
            face(BoundaryFaceVertices3::Quad([0, 1, 2, 3]), 0),
            face(BoundaryFaceVertices3::Triangle([1, 2, 4]), 1),
        ]);
        let result = source.triangulate();
        assert_eq!(result.triangle_count(), 3);
        assert_eq!(result.triangles()[0].source_face_index(), 0);
        assert_eq!(result.triangles()[0].source_triangle_index(), 0);
        assert_eq!(result.triangles()[1].source_face_index(), 0);
        assert_eq!(result.triangles()[1].source_triangle_index(), 1);
        assert_eq!(result.triangles()[2].source_face_index(), 1);
        assert_eq!(result.triangles()[2].source_triangle_index(), 0);
    }

    #[test]
    fn every_triangle_retains_complete_source_provenance() {
        let source = surface(vec![10, 20, 30, 40],
            vec![face(BoundaryFaceVertices3::Quad([0, 1, 2, 3]), 0)]);
        for (ordinal, triangle) in source.triangulate().triangles().iter().enumerate() {
            assert_eq!(triangle.source_face_index(), 0);
            assert_eq!(triangle.source_cell_index(), 10);
            assert_eq!(triangle.source_local_face_index(), 20);
            assert_eq!(triangle.source_triangle_index(), ordinal);
        }
    }

    #[test]
    fn points_and_source_point_mapping_are_preserved_exactly() {
        let source = surface(vec![50, 10, 40, 20],
            vec![face(BoundaryFaceVertices3::Quad([0, 1, 2, 3]), 0)]);
        let result = source.triangulate();
        assert_eq!(result.points(), source.points());
        assert_eq!(result.source_point_ids(), source.source_point_ids());
        assert_eq!(result.point_count(), source.point_count());
    }

    #[test]
    fn repeated_triangulation_is_deterministic_and_does_not_mutate_source() {
        let source = surface(vec![10, 40, 20, 30],
            vec![face(BoundaryFaceVertices3::Quad([0, 1, 2, 3]), 0)]);
        let before = source.clone();
        assert_eq!(source.triangulate(), source.triangulate());
        assert_eq!(source, before);
    }

    #[test]
    fn degenerate_quad_is_triangulated_without_geometric_filtering() {
        let mut source = surface(vec![10, 20, 30, 40],
            vec![face(BoundaryFaceVertices3::Quad([0, 1, 2, 3]), 0)]);
        source.points = vec![p(0.0, 0.0, 0.0); 4];
        assert_eq!(source.triangulate().triangle_count(), 2);
    }
}
