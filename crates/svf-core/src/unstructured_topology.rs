//! Derived face topology for unstructured volume cells.
//!
//! This module reports topological face incidence without modifying source
//! connectivity or making geometric validity and orientation claims.

use std::collections::BTreeMap;

use crate::unstructured::{CellType3, UnstructuredGrid3};

/// The vertex IDs of a volume face, retaining its local cyclic ordering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VolumeFaceVertices3 {
    /// A triangular face in local cyclic order.
    Triangle([u32; 3]),
    /// A quadrilateral face in local cyclic order.
    Quad([u32; 4]),
}

impl VolumeFaceVertices3 {
    /// Returns the point IDs in this face's retained local order.
    #[must_use]
    pub const fn as_slice(&self) -> &[u32] {
        match self {
            Self::Triangle(vertices) => vertices,
            Self::Quad(vertices) => vertices,
        }
    }

    /// Returns the number of vertices in this face.
    #[must_use]
    pub const fn vertex_count(&self) -> usize {
        match self {
            Self::Triangle(_) => 3,
            Self::Quad(_) => 4,
        }
    }
}

/// Classification based only on the number of incident volume cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VolumeFaceClassification {
    /// Exactly one volume cell references this face.
    Boundary,
    /// Exactly two volume cells reference this face.
    Interior,
    /// More than two volume cells reference this face.
    NonManifold,
}

/// One source cell's contribution to a unique volume face.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VolumeFaceIncidence3 {
    source_cell_index: usize,
    local_face_index: usize,
    vertices: VolumeFaceVertices3,
}

impl VolumeFaceIncidence3 {
    /// Returns the source cell ID (its index in the original grid).
    #[must_use]
    pub const fn source_cell_index(&self) -> usize {
        self.source_cell_index
    }

    /// Returns the face's index in the source cell's local face template.
    #[must_use]
    pub const fn local_face_index(&self) -> usize {
        self.local_face_index
    }

    /// Returns this incidence's point IDs in the source cell's local order.
    #[must_use]
    pub const fn vertices(&self) -> VolumeFaceVertices3 {
        self.vertices
    }
}

/// A unique face and all source volume cells incident to it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VolumeFace3 {
    vertices: VolumeFaceVertices3,
    incidences: Vec<VolumeFaceIncidence3>,
    classification: VolumeFaceClassification,
}

impl VolumeFace3 {
    /// Returns the representative point order from the first incidence.
    #[must_use]
    pub const fn vertices(&self) -> VolumeFaceVertices3 {
        self.vertices
    }

    /// Returns all source-cell incidences in deterministic source order.
    #[must_use]
    pub fn incidences(&self) -> &[VolumeFaceIncidence3] {
        &self.incidences
    }

    /// Returns the incidence-based face classification.
    #[must_use]
    pub const fn classification(&self) -> VolumeFaceClassification {
        self.classification
    }

    /// Returns the number of incident volume cells.
    #[must_use]
    pub fn incidence_count(&self) -> usize {
        self.incidences.len()
    }
}

/// Deterministic face topology derived from an unstructured volume grid.
///
/// Faces are ordered lexicographically by canonical point IDs. Incidences
/// within a face are ordered by source cell ID and then local face index.
/// This structure reports connectivity incidence only; it does not certify
/// watertightness, geometric manifoldness, cell orientation, or cell quality.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnstructuredVolumeTopology3 {
    faces: Vec<VolumeFace3>,
}

impl UnstructuredVolumeTopology3 {
    /// Returns all unique volume faces in deterministic canonical-key order.
    #[must_use]
    pub fn faces(&self) -> &[VolumeFace3] {
        &self.faces
    }

    /// Iterates over faces incident to exactly one volume cell.
    pub fn boundary_faces(&self) -> impl Iterator<Item = &VolumeFace3> {
        self.faces
            .iter()
            .filter(|face| face.classification == VolumeFaceClassification::Boundary)
    }

    /// Iterates over faces incident to exactly two volume cells.
    pub fn interior_faces(&self) -> impl Iterator<Item = &VolumeFace3> {
        self.faces
            .iter()
            .filter(|face| face.classification == VolumeFaceClassification::Interior)
    }

    /// Iterates over faces incident to more than two volume cells.
    pub fn non_manifold_faces(&self) -> impl Iterator<Item = &VolumeFace3> {
        self.faces
            .iter()
            .filter(|face| face.classification == VolumeFaceClassification::NonManifold)
    }

    /// Returns the number of unique faces across all supported volume cells.
    #[must_use]
    pub fn face_count(&self) -> usize {
        self.faces.len()
    }

    /// Returns the number of faces incident to exactly one volume cell.
    #[must_use]
    pub fn boundary_face_count(&self) -> usize {
        self.boundary_faces().count()
    }

    /// Returns the number of faces incident to exactly two volume cells.
    #[must_use]
    pub fn interior_face_count(&self) -> usize {
        self.interior_faces().count()
    }

    /// Returns the number of faces incident to more than two volume cells.
    #[must_use]
    pub fn non_manifold_face_count(&self) -> usize {
        self.non_manifold_faces().count()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum FaceKey {
    Triangle([u32; 3]),
    Quad([u32; 4]),
}

impl FaceKey {
    fn from_vertices(vertices: VolumeFaceVertices3) -> Self {
        match vertices {
            VolumeFaceVertices3::Triangle(mut ids) => {
                ids.sort_unstable();
                Self::Triangle(ids)
            }
            VolumeFaceVertices3::Quad(mut ids) => {
                ids.sort_unstable();
                Self::Quad(ids)
            }
        }
    }
}

const TETRAHEDRON_FACES: [[usize; 3]; 4] = [
    [1, 2, 3],
    [0, 3, 2],
    [0, 1, 3],
    [0, 2, 1],
];

const HEXAHEDRON_FACES: [[usize; 4]; 6] = [
    [0, 3, 2, 1],
    [4, 5, 6, 7],
    [0, 1, 5, 4],
    [1, 2, 6, 5],
    [2, 3, 7, 6],
    [3, 0, 4, 7],
];

impl UnstructuredGrid3 {
    /// Derives unique face incidence for all tetrahedral and hexahedral cells.
    ///
    /// Lower-dimensional cells are ignored. Face identity is based on point
    /// IDs, not coordinates. Original local face ordering and source cell IDs
    /// are retained in every incidence.
    #[must_use]
    pub fn volume_topology(&self) -> UnstructuredVolumeTopology3 {
        let mut faces: BTreeMap<FaceKey, Vec<VolumeFaceIncidence3>> = BTreeMap::new();

        for (source_cell_index, cell_type) in self.cell_types().iter().copied().enumerate() {
            let Some(point_ids) = self.cell_point_indices(source_cell_index) else {
                continue;
            };

            match cell_type {
                CellType3::Tetrahedron4 => {
                    for (local_face_index, template) in TETRAHEDRON_FACES.iter().enumerate() {
                        let vertices = VolumeFaceVertices3::Triangle([
                            point_ids[template[0]],
                            point_ids[template[1]],
                            point_ids[template[2]],
                        ]);
                        faces.entry(FaceKey::from_vertices(vertices)).or_default().push(
                            VolumeFaceIncidence3 {
                                source_cell_index,
                                local_face_index,
                                vertices,
                            },
                        );
                    }
                }
                CellType3::Hexahedron8 => {
                    for (local_face_index, template) in HEXAHEDRON_FACES.iter().enumerate() {
                        let vertices = VolumeFaceVertices3::Quad([
                            point_ids[template[0]],
                            point_ids[template[1]],
                            point_ids[template[2]],
                            point_ids[template[3]],
                        ]);
                        faces.entry(FaceKey::from_vertices(vertices)).or_default().push(
                            VolumeFaceIncidence3 {
                                source_cell_index,
                                local_face_index,
                                vertices,
                            },
                        );
                    }
                }
                CellType3::Line2 | CellType3::Triangle3 | CellType3::Quad4 => {}
            }
        }

        let faces = faces
            .into_values()
            .map(|incidences| {
                let classification = match incidences.len() {
                    1 => VolumeFaceClassification::Boundary,
                    2 => VolumeFaceClassification::Interior,
                    _ => VolumeFaceClassification::NonManifold,
                };
                VolumeFace3 {
                    vertices: incidences[0].vertices,
                    incidences,
                    classification,
                }
            })
            .collect();

        UnstructuredVolumeTopology3 { faces }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spatial::WorldPoint3;

    fn point(x: f64, y: f64, z: f64) -> WorldPoint3 {
        WorldPoint3::new(x, y, z)
    }

    fn tetra_points() -> Vec<WorldPoint3> {
        vec![
            point(0.0, 0.0, 0.0),
            point(1.0, 0.0, 0.0),
            point(0.0, 1.0, 0.0),
            point(0.0, 0.0, 1.0),
        ]
    }

    fn cube_points() -> Vec<WorldPoint3> {
        vec![
            point(0.0, 0.0, 0.0),
            point(1.0, 0.0, 0.0),
            point(1.0, 1.0, 0.0),
            point(0.0, 1.0, 0.0),
            point(0.0, 0.0, 1.0),
            point(1.0, 0.0, 1.0),
            point(1.0, 1.0, 1.0),
            point(0.0, 1.0, 1.0),
        ]
    }

    fn grid(points: Vec<WorldPoint3>, cells: &[(CellType3, &[u32])]) -> UnstructuredGrid3 {
        let mut offsets = vec![0];
        let mut types = Vec::with_capacity(cells.len());
        let mut connectivity = Vec::new();
        for (cell_type, ids) in cells {
            types.push(*cell_type);
            connectivity.extend_from_slice(ids);
            offsets.push(connectivity.len());
        }
        UnstructuredGrid3::new(points, types, offsets, connectivity).unwrap()
    }

    fn key(face: &VolumeFace3) -> Vec<u32> {
        let mut ids = face.vertices().as_slice().to_vec();
        ids.sort_unstable();
        ids
    }

    #[test]
    fn one_tetrahedron_produces_four_boundary_triangles() {
        let grid = grid(
            tetra_points(),
            &[(CellType3::Tetrahedron4, &[0, 1, 2, 3])],
        );
        let topology = grid.volume_topology();
        assert_eq!(topology.face_count(), 4);
        assert_eq!(topology.boundary_face_count(), 4);
        assert_eq!(topology.interior_face_count(), 0);
        assert_eq!(topology.non_manifold_face_count(), 0);
        assert!(topology
            .faces()
            .iter()
            .all(|face| matches!(face.vertices(), VolumeFaceVertices3::Triangle(_))));
    }

    #[test]
    fn one_hexahedron_produces_six_boundary_quads() {
        let grid = grid(
            cube_points(),
            &[(CellType3::Hexahedron8, &[0, 1, 2, 3, 4, 5, 6, 7])],
        );
        let topology = grid.volume_topology();
        assert_eq!(topology.face_count(), 6);
        assert_eq!(topology.boundary_face_count(), 6);
        assert!(topology
            .faces()
            .iter()
            .all(|face| matches!(face.vertices(), VolumeFaceVertices3::Quad(_))));
    }

    #[test]
    fn two_tetrahedra_sharing_a_face_produce_one_interior_face() {
        let mut points = tetra_points();
        points.push(point(0.0, 0.0, -1.0));
        let grid = grid(
            points,
            &[
                (CellType3::Tetrahedron4, &[0, 1, 2, 3]),
                (CellType3::Tetrahedron4, &[0, 2, 1, 4]),
            ],
        );
        let topology = grid.volume_topology();
        assert_eq!(topology.face_count(), 7);
        assert_eq!(topology.boundary_face_count(), 6);
        assert_eq!(topology.interior_face_count(), 1);
        let shared = topology.interior_faces().next().unwrap();
        assert_eq!(key(shared), vec![0, 1, 2]);
        assert_eq!(
            shared
                .incidences()
                .iter()
                .map(VolumeFaceIncidence3::source_cell_index)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
    }

    #[test]
    fn two_adjacent_hexahedra_produce_one_interior_face() {
        let mut points = cube_points();
        points.extend([
            point(2.0, 0.0, 0.0),
            point(2.0, 1.0, 0.0),
            point(2.0, 0.0, 1.0),
            point(2.0, 1.0, 1.0),
        ]);
        let grid = grid(
            points,
            &[
                (CellType3::Hexahedron8, &[0, 1, 2, 3, 4, 5, 6, 7]),
                (CellType3::Hexahedron8, &[1, 8, 9, 2, 5, 10, 11, 6]),
            ],
        );
        let topology = grid.volume_topology();
        assert_eq!(topology.face_count(), 11);
        assert_eq!(topology.boundary_face_count(), 10);
        assert_eq!(topology.interior_face_count(), 1);
        assert_eq!(
            key(topology.interior_faces().next().unwrap()),
            vec![1, 2, 5, 6]
        );
    }

    #[test]
    fn three_tetrahedra_sharing_a_face_produce_non_manifold_incidence() {
        let mut points = tetra_points();
        points.extend([
            point(0.0, 0.0, -1.0),
            point(0.0, 0.0, 2.0),
        ]);
        let grid = grid(
            points,
            &[
                (CellType3::Tetrahedron4, &[0, 1, 2, 3]),
                (CellType3::Tetrahedron4, &[0, 2, 1, 4]),
                (CellType3::Tetrahedron4, &[0, 1, 2, 5]),
            ],
        );
        let topology = grid.volume_topology();
        assert_eq!(topology.non_manifold_face_count(), 1);
        let shared = topology.non_manifold_faces().next().unwrap();
        assert_eq!(key(shared), vec![0, 1, 2]);
        assert_eq!(shared.incidence_count(), 3);
    }

    #[test]
    fn mixed_volume_cells_contribute_their_own_face_types() {
        let mut points = cube_points();
        points.extend([
            point(3.0, 0.0, 0.0),
            point(4.0, 0.0, 0.0),
            point(3.0, 1.0, 0.0),
            point(3.0, 0.0, 1.0),
        ]);
        let grid = grid(
            points,
            &[
                (CellType3::Tetrahedron4, &[8, 9, 10, 11]),
                (CellType3::Hexahedron8, &[0, 1, 2, 3, 4, 5, 6, 7]),
            ],
        );
        let topology = grid.volume_topology();
        assert_eq!(topology.face_count(), 10);
        assert_eq!(
            topology
                .faces()
                .iter()
                .filter(|face| matches!(face.vertices(), VolumeFaceVertices3::Triangle(_)))
                .count(),
            4
        );
        assert_eq!(
            topology
                .faces()
                .iter()
                .filter(|face| matches!(face.vertices(), VolumeFaceVertices3::Quad(_)))
                .count(),
            6
        );
    }

    #[test]
    fn lower_dimensional_cells_do_not_contribute_volume_faces() {
        let grid = grid(
            tetra_points(),
            &[
                (CellType3::Line2, &[0, 1]),
                (CellType3::Triangle3, &[0, 1, 2]),
                (CellType3::Quad4, &[0, 1, 2, 3]),
            ],
        );
        assert_eq!(grid.volume_topology().face_count(), 0);
    }

    #[test]
    fn incidences_preserve_source_cell_and_local_face_order() {
        let grid = grid(
            tetra_points(),
            &[(CellType3::Tetrahedron4, &[10.min(3), 1, 2, 3])],
        );
        let topology = grid.volume_topology();
        let face = topology
            .faces()
            .iter()
            .find(|face| key(face) == vec![1, 2, 3])
            .unwrap();
        assert_eq!(face.incidences()[0].source_cell_index(), 0);
        assert_eq!(face.incidences()[0].local_face_index(), 0);
        assert_eq!(
            face.incidences()[0].vertices(),
            VolumeFaceVertices3::Triangle([1, 2, 3])
        );
    }

    #[test]
    fn face_and_incidence_order_is_deterministic() {
        let grid = grid(
            tetra_points(),
            &[(CellType3::Tetrahedron4, &[0, 1, 2, 3])],
        );
        assert_eq!(grid.volume_topology(), grid.volume_topology());
        let keys: Vec<_> = grid.volume_topology().faces().iter().map(key).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted);
    }

    #[test]
    fn reordering_cells_preserves_face_keys_and_classification() {
        let mut points = tetra_points();
        points.push(point(0.0, 0.0, -1.0));
        let first = grid(
            points.clone(),
            &[
                (CellType3::Tetrahedron4, &[0, 1, 2, 3]),
                (CellType3::Tetrahedron4, &[0, 2, 1, 4]),
            ],
        );
        let reordered = grid(
            points,
            &[
                (CellType3::Tetrahedron4, &[0, 2, 1, 4]),
                (CellType3::Tetrahedron4, &[0, 1, 2, 3]),
            ],
        );
        let first_faces = first.volume_topology();
        let reordered_faces = reordered.volume_topology();
        assert_eq!(
            first_faces.faces().iter().map(key).collect::<Vec<_>>(),
            reordered_faces.faces().iter().map(key).collect::<Vec<_>>()
        );
        assert_eq!(
            first_faces
                .faces()
                .iter()
                .map(VolumeFace3::classification)
                .collect::<Vec<_>>(),
            reordered_faces
                .faces()
                .iter()
                .map(VolumeFace3::classification)
                .collect::<Vec<_>>()
        );
        let shared = reordered_faces.interior_faces().next().unwrap();
        assert_eq!(
            shared
                .incidences()
                .iter()
                .map(VolumeFaceIncidence3::source_cell_index)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
    }

    #[test]
    fn coincident_coordinates_with_different_point_ids_do_not_merge_faces() {
        let mut points = tetra_points();
        points.extend([
            point(0.0, 0.0, 0.0),
            point(1.0, 0.0, 0.0),
            point(0.0, 1.0, 0.0),
            point(0.0, 0.0, -1.0),
        ]);
        let grid = grid(
            points,
            &[
                (CellType3::Tetrahedron4, &[0, 1, 2, 3]),
                (CellType3::Tetrahedron4, &[4, 5, 6, 7]),
            ],
        );
        let topology = grid.volume_topology();
        assert_eq!(topology.face_count(), 8);
        assert_eq!(topology.interior_face_count(), 0);
        assert_eq!(topology.boundary_face_count(), 8);
    }

    #[test]
    fn empty_and_non_volume_grids_have_empty_topology() {
        assert_eq!(UnstructuredGrid3::default().volume_topology().face_count(), 0);
        let points_only = UnstructuredGrid3::new(
            tetra_points(),
            Vec::new(),
            vec![0],
            Vec::new(),
        )
        .unwrap();
        assert_eq!(points_only.volume_topology().face_count(), 0);
    }

    #[test]
    fn geometrically_degenerate_volume_cells_are_retained() {
        let points = vec![
            point(0.0, 0.0, 0.0),
            point(1.0, 0.0, 0.0),
            point(2.0, 0.0, 0.0),
            point(3.0, 0.0, 0.0),
        ];
        let grid = grid(
            points,
            &[(CellType3::Tetrahedron4, &[0, 1, 2, 3])],
        );
        assert_eq!(grid.volume_topology().boundary_face_count(), 4);
    }
}
