//! Graphics-independent geometric primitives in world coordinates.
//!
//! These types describe scientific geometry, not GPU vertex layouts. They
//! retain f64 coordinates and can be validated and inspected without a
//! renderer. Conversion to renderer-specific formats belongs downstream.

use std::{collections::{BTreeMap, BTreeSet}, error::Error, fmt};

use crate::spatial::WorldPoint3;

/// Errors produced when constructing scientific geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeometryError {
    /// A point contains one or more non-finite coordinates.
    NonFinitePoint { index: usize },
    /// A polyline must contain at least two points.
    PolylineRequiresTwoPoints { actual: usize },
    /// An indexed triangle mesh must contain complete groups of three indices.
    TriangleIndexCountNotMultipleOfThree { actual: usize },
    /// A triangle references a vertex that does not exist.
    TriangleIndexOutOfBounds {
        triangle_index: usize,
        corner: usize,
        index: u32,
        vertex_count: usize,
    },
    /// A triangle cannot reference the same vertex more than once.
    RepeatedTriangleVertexIndex { triangle_index: usize, index: u32 },
}

impl fmt::Display for GeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinitePoint { index } => {
                write!(f, "point at index {index} contains a non-finite coordinate")
            }
            Self::PolylineRequiresTwoPoints { actual } => {
                write!(f, "a polyline requires at least two points; received {actual}")
            }
            Self::TriangleIndexCountNotMultipleOfThree { actual } => {
                write!(f, "triangle index count must be divisible by three; received {actual}")
            }
            Self::TriangleIndexOutOfBounds {
                triangle_index,
                corner,
                index,
                vertex_count,
            } => write!(
                f,
                "triangle {triangle_index} corner {corner} references vertex {index}, but only {vertex_count} vertices exist"
            ),
            Self::RepeatedTriangleVertexIndex { triangle_index, index } => write!(
                f,
                "triangle {triangle_index} references vertex {index} more than once"
            ),
        }
    }
}

impl Error for GeometryError {}

/// A collection of independent world-space points.
///
/// An empty point set is valid. Point order is preserved, and no connectivity
/// between points is implied.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PointSet3 {
    points: Vec<WorldPoint3>,
}

impl PointSet3 {
    /// Creates a point set after validating every coordinate.
    pub fn new(points: Vec<WorldPoint3>) -> Result<Self, GeometryError> {
        validate_points(&points)?;
        Ok(Self { points })
    }

    /// Returns the points in their original order.
    #[must_use]
    pub fn points(&self) -> &[WorldPoint3] {
        &self.points
    }

    /// Returns the number of points.
    #[must_use]
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Returns whether the point set contains no points.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Consumes the point set and returns its points.
    #[must_use]
    pub fn into_points(self) -> Vec<WorldPoint3> {
        self.points
    }
}

/// A straight line segment between two world-space endpoints.
///
/// Coincident endpoints are allowed and represent a zero-length segment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineSegment3 {
    start: WorldPoint3,
    end: WorldPoint3,
}

impl LineSegment3 {
    /// Creates a segment after validating both endpoints.
    pub fn new(start: WorldPoint3, end: WorldPoint3) -> Result<Self, GeometryError> {
        validate_points(&[start, end])?;
        Ok(Self { start, end })
    }

    #[must_use]
    pub const fn start(&self) -> WorldPoint3 {
        self.start
    }

    #[must_use]
    pub const fn end(&self) -> WorldPoint3 {
        self.end
    }

    /// Returns the displacement from the start point to the end point.
    #[must_use]
    pub fn displacement(&self) -> crate::spatial::WorldVector3 {
        self.end - self.start
    }

    /// Returns the Euclidean length in world-coordinate units.
    #[must_use]
    pub fn length(&self) -> f64 {
        self.displacement().length()
    }
}

/// An ordered, open sequence of world-space points.
///
/// Consecutive points define line segments. The sequence is not implicitly
/// closed; to represent a closed path, repeat the first point at the end.
/// Repeated consecutive points are preserved and produce zero-length segments.
#[derive(Clone, Debug, PartialEq)]
pub struct Polyline3 {
    points: Vec<WorldPoint3>,
}

impl Polyline3 {
    /// Creates a polyline containing at least two finite points.
    pub fn new(points: Vec<WorldPoint3>) -> Result<Self, GeometryError> {
        if points.len() < 2 {
            return Err(GeometryError::PolylineRequiresTwoPoints {
                actual: points.len(),
            });
        }
        validate_points(&points)?;
        Ok(Self { points })
    }

    /// Returns the ordered points defining this polyline.
    #[must_use]
    pub fn points(&self) -> &[WorldPoint3] {
        &self.points
    }

    /// Returns the number of points.
    #[must_use]
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Returns the number of consecutive line segments.
    #[must_use]
    pub fn segment_count(&self) -> usize {
        self.points.len() - 1
    }

    /// Iterates over consecutive segments without copying the point storage.
    pub fn segments(&self) -> impl ExactSizeIterator<Item = LineSegment3> + '_ {
        self.points.windows(2).map(|pair| LineSegment3 {
            start: pair[0],
            end: pair[1],
        })
    }

    /// Consumes the polyline and returns its points.
    #[must_use]
    pub fn into_points(self) -> Vec<WorldPoint3> {
        self.points
    }
}

/// An indexed triangle mesh in world coordinates.
///
/// Vertices are stored once and triangles refer to them through three
/// `u32` indices per triangle. Triangle winding and input ordering are
/// preserved. Unreferenced vertices are allowed. The mesh does not require
/// manifold edges or a closed surface: open surfaces and disconnected
/// components are valid scientific geometry.
///
/// Repeated vertex indices within one triangle are rejected because they do
/// not define a valid triangle topology. Geometrically collinear or coincident
/// vertex positions are retained; geometric degeneracy is distinct from
/// index-topology validity and may be meaningful in source datasets.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TriangleMesh3 {
    vertices: Vec<WorldPoint3>,
    indices: Vec<u32>,
}

impl TriangleMesh3 {
    /// Creates an indexed triangle mesh after validating coordinates and
    /// triangle index topology.
    pub fn new(
        vertices: Vec<WorldPoint3>,
        indices: Vec<u32>,
    ) -> Result<Self, GeometryError> {
        validate_points(&vertices)?;

        if indices.len() % 3 != 0 {
            return Err(GeometryError::TriangleIndexCountNotMultipleOfThree {
                actual: indices.len(),
            });
        }

        let vertex_count = vertices.len();
        for (triangle_index, triangle) in indices.chunks_exact(3).enumerate() {
            for (corner, &index) in triangle.iter().enumerate() {
                if index as usize >= vertex_count {
                    return Err(GeometryError::TriangleIndexOutOfBounds {
                        triangle_index,
                        corner,
                        index,
                        vertex_count,
                    });
                }
            }

            if triangle[0] == triangle[1] {
                return Err(GeometryError::RepeatedTriangleVertexIndex {
                    triangle_index,
                    index: triangle[0],
                });
            }
            if triangle[0] == triangle[2] {
                return Err(GeometryError::RepeatedTriangleVertexIndex {
                    triangle_index,
                    index: triangle[0],
                });
            }
            if triangle[1] == triangle[2] {
                return Err(GeometryError::RepeatedTriangleVertexIndex {
                    triangle_index,
                    index: triangle[1],
                });
            }
        }

        Ok(Self { vertices, indices })
    }

    /// Returns the vertex positions in their original order.
    #[must_use]
    pub fn vertices(&self) -> &[WorldPoint3] {
        &self.vertices
    }

    /// Returns the flattened triangle indices in their original order.
    #[must_use]
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }

    /// Returns the number of stored vertices, including unreferenced ones.
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Returns the number of indexed triangles.
    #[must_use]
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Returns whether the mesh contains neither vertices nor triangles.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty() && self.indices.is_empty()
    }

    /// Iterates over triangle vertex indices, preserving winding order.
    pub fn triangles(&self) -> impl ExactSizeIterator<Item = [u32; 3]> + '_ {
        self.indices
            .chunks_exact(3)
            .map(|triangle| [triangle[0], triangle[1], triangle[2]])
    }

    /// Derives deterministic edge and face adjacency information.
    ///
    /// Unique undirected edges are ordered lexicographically by their
    /// canonical vertex-index pair. Face-edge slots follow each triangle's
    /// local directed edge order: (v0, v1), (v1, v2), (v2, v0).
    #[must_use]
    pub fn topology(&self) -> MeshTopology3 {
        MeshTopology3::from_mesh(self)
    }
    /// Consumes the mesh and returns its vertices and flattened indices.
    #[must_use]
    pub fn into_parts(self) -> (Vec<WorldPoint3>, Vec<u32>) {
        (self.vertices, self.indices)
    }
}

/// A unique undirected edge in a triangle mesh.
///
/// Endpoints are stored in ascending vertex-index order. This canonical
/// representation identifies an edge independently of triangle winding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MeshEdge {
    vertices: [u32; 2],
}

impl MeshEdge {
    fn new(a: u32, b: u32) -> Self {
        Self { vertices: if a < b { [a, b] } else { [b, a] } }
    }

    /// Returns the canonical endpoint vertex indices in ascending order.
    #[must_use]
    pub const fn vertices(&self) -> [u32; 2] {
        self.vertices
    }
}

/// Derived edge and face incidence for an indexed triangle mesh.
///
/// Edge IDs are stable for an unchanged vertex/index dataset: edges are
/// sorted lexicographically by their canonical endpoint indices. Face indices
/// refer to the original triangle order. This structure reports incidence
/// only; it does not certify manifoldness, watertightness, or surface validity.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MeshTopology3 {
    edges: Vec<MeshEdge>,
    face_edges: Vec<[usize; 3]>,
    edge_faces: Vec<Vec<usize>>,
}

impl MeshTopology3 {
    fn from_mesh(mesh: &TriangleMesh3) -> Self {
        let mut unique_edges = BTreeSet::new();
        for [a, b, c] in mesh.triangles() {
            unique_edges.insert(MeshEdge::new(a, b).vertices);
            unique_edges.insert(MeshEdge::new(b, c).vertices);
            unique_edges.insert(MeshEdge::new(c, a).vertices);
        }

        let edges: Vec<_> = unique_edges.into_iter()
            .map(|vertices| MeshEdge { vertices }).collect();
        let edge_lookup: BTreeMap<_, _> = edges.iter().enumerate()
            .map(|(edge_id, edge)| (edge.vertices, edge_id)).collect();
        let mut face_edges = Vec::with_capacity(mesh.triangle_count());
        let mut edge_faces = vec![Vec::new(); edges.len()];

        for (face_index, [a, b, c]) in mesh.triangles().enumerate() {
            let keys = [MeshEdge::new(a, b).vertices, MeshEdge::new(b, c).vertices, MeshEdge::new(c, a).vertices];
            let edge_ids = keys.map(|key| edge_lookup[&key]);
            for edge_id in edge_ids { edge_faces[edge_id].push(face_index); }
            face_edges.push(edge_ids);
        }
        Self { edges, face_edges, edge_faces }
    }

    /// Returns all unique undirected edges in deterministic order.
    #[must_use]
    pub fn edges(&self) -> &[MeshEdge] { &self.edges }

    /// Returns the three edge IDs for a face in local (v0-v1, v1-v2, v2-v0)
    /// order, or None when the face index is out of range.
    #[must_use]
    pub fn face_edges(&self, face_index: usize) -> Option<[usize; 3]> {
        self.face_edges.get(face_index).copied()
    }

    /// Returns incident face indices in source face order, or None for an
    /// out-of-range edge ID.
    #[must_use]
    pub fn edge_faces(&self, edge_id: usize) -> Option<&[usize]> {
        self.edge_faces.get(edge_id).map(Vec::as_slice)
    }

    /// Iterates over edges incident to exactly one face.
    /// This local incidence query does not validate the complete surface.
    pub fn boundary_edges(&self) -> impl Iterator<Item = (usize, &MeshEdge)> {
        self.edges.iter().enumerate()
            .filter(|(edge_id, _)| self.edge_faces[*edge_id].len() == 1)
    }

    /// Iterates over edges incident to more than two faces. This makes no
    /// global manifoldness assertion.
    pub fn edges_with_more_than_two_faces(&self) -> impl Iterator<Item = (usize, &MeshEdge)> {
        self.edges.iter().enumerate()
            .filter(|(edge_id, _)| self.edge_faces[*edge_id].len() > 2)
    }

    /// Returns the number of unique undirected edges.
    #[must_use]
    pub fn edge_count(&self) -> usize { self.edges.len() }

    /// Returns the number of source faces represented by this topology.
    #[must_use]
    pub fn face_count(&self) -> usize { self.face_edges.len() }
}
fn validate_points(points: &[WorldPoint3]) -> Result<(), GeometryError> {
    for (index, point) in points.iter().enumerate() {
        if !point.to_vector().is_finite() {
            return Err(GeometryError::NonFinitePoint { index });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(x: f64, y: f64, z: f64) -> WorldPoint3 {
        WorldPoint3::new(x, y, z)
    }

    #[test]
    fn triangle_mesh_accepts_empty_mesh_and_vertices_without_triangles() {
        let empty = TriangleMesh3::new(Vec::new(), Vec::new()).unwrap();
        assert!(empty.is_empty());
        assert_eq!(empty.vertex_count(), 0);
        assert_eq!(empty.triangle_count(), 0);

        let vertices = vec![point(0.0, 0.0, 0.0), point(1.0, 0.0, 0.0)];
        let vertices_only = TriangleMesh3::new(vertices.clone(), Vec::new()).unwrap();
        assert!(!vertices_only.is_empty());
        assert_eq!(vertices_only.vertices(), vertices);
        assert_eq!(vertices_only.triangle_count(), 0);
    }

    #[test]
    fn triangle_mesh_preserves_vertex_order_indices_and_winding() {
        let vertices = vec![
            point(0.0, 0.0, 0.0),
            point(1.0, 0.0, 0.0),
            point(0.0, 1.0, 0.0),
            point(1.0, 1.0, 0.0),
        ];
        let indices = vec![0, 1, 2, 2, 1, 3];
        let mesh = TriangleMesh3::new(vertices.clone(), indices.clone()).unwrap();

        assert_eq!(mesh.vertices(), vertices);
        assert_eq!(mesh.indices(), indices);
        assert_eq!(mesh.vertex_count(), 4);
        assert_eq!(mesh.triangle_count(), 2);
        assert_eq!(mesh.triangles().collect::<Vec<_>>(), vec![[0, 1, 2], [2, 1, 3]]);
        assert_eq!(mesh.into_parts(), (vertices, indices));
    }

    #[test]
    fn triangle_mesh_allows_unreferenced_vertices() {
        let vertices = vec![
            point(0.0, 0.0, 0.0),
            point(1.0, 0.0, 0.0),
            point(0.0, 1.0, 0.0),
            point(9.0, 9.0, 9.0),
        ];
        let mesh = TriangleMesh3::new(vertices, vec![0, 1, 2]).unwrap();
        assert_eq!(mesh.vertex_count(), 4);
        assert_eq!(mesh.triangle_count(), 1);
    }

    #[test]
    fn triangle_mesh_rejects_incomplete_index_groups() {
        assert_eq!(
            TriangleMesh3::new(
                vec![point(0.0, 0.0, 0.0), point(1.0, 0.0, 0.0)],
                vec![0, 1]
            ),
            Err(GeometryError::TriangleIndexCountNotMultipleOfThree { actual: 2 })
        );
    }

    #[test]
    fn triangle_mesh_rejects_out_of_range_indices_with_location() {
        let result = TriangleMesh3::new(
            vec![point(0.0, 0.0, 0.0), point(1.0, 0.0, 0.0), point(0.0, 1.0, 0.0)],
            vec![0, 1, 3],
        );
        assert_eq!(
            result,
            Err(GeometryError::TriangleIndexOutOfBounds {
                triangle_index: 0,
                corner: 2,
                index: 3,
                vertex_count: 3,
            })
        );
    }

    #[test]
    fn triangle_mesh_rejects_repeated_vertex_indices() {
        let vertices = vec![
            point(0.0, 0.0, 0.0),
            point(1.0, 0.0, 0.0),
            point(0.0, 1.0, 0.0),
        ];
        for indices in [vec![0, 0, 1], vec![0, 1, 0], vec![1, 0, 1]] {
            assert!(matches!(
                TriangleMesh3::new(vertices.clone(), indices),
                Err(GeometryError::RepeatedTriangleVertexIndex { triangle_index: 0, .. })
            ));
        }
    }

    #[test]
    fn triangle_mesh_rejects_non_finite_vertex_coordinates() {
        let result = TriangleMesh3::new(
            vec![
                point(0.0, 0.0, 0.0),
                point(1.0, 0.0, 0.0),
                point(0.0, f64::NAN, 0.0),
            ],
            vec![0, 1, 2],
        );
        assert_eq!(result, Err(GeometryError::NonFinitePoint { index: 2 }));
    }

    #[test]
    fn triangle_mesh_preserves_geometrically_degenerate_positions() {
        let mesh = TriangleMesh3::new(
            vec![
                point(0.0, 0.0, 0.0),
                point(1.0, 0.0, 0.0),
                point(2.0, 0.0, 0.0),
            ],
            vec![0, 1, 2],
        )
        .unwrap();
        assert_eq!(mesh.triangle_count(), 1);
    }

    #[test]
    fn topology_extracts_unique_edges_and_face_incidence() {
        let mesh = TriangleMesh3::new(
            vec![point(0.0, 0.0, 0.0), point(1.0, 0.0, 0.0), point(0.0, 1.0, 0.0), point(1.0, 1.0, 0.0)],
            vec![0, 1, 2, 2, 1, 3],
        ).unwrap();
        let topology = mesh.topology();
        assert_eq!(topology.edge_count(), 5);
        assert_eq!(topology.face_count(), 2);
        assert_eq!(topology.edges().iter().map(MeshEdge::vertices).collect::<Vec<_>>(),
            vec![[0, 1], [0, 2], [1, 2], [1, 3], [2, 3]]);
        let first = topology.face_edges(0).unwrap();
        let second = topology.face_edges(1).unwrap();
        assert_eq!(first.map(|id| topology.edges()[id].vertices()), [[0, 1], [1, 2], [0, 2]]);
        assert_eq!(second.map(|id| topology.edges()[id].vertices()), [[1, 2], [1, 3], [2, 3]]);
        assert_eq!(topology.edge_faces(first[1]), Some(&[0, 1][..]));
        assert_eq!(topology.edge_faces(first[0]), Some(&[0][..]));
    }

    #[test]
    fn topology_identifies_boundary_edges_by_incidence() {
        let mesh = TriangleMesh3::new(vec![point(0.0, 0.0, 0.0), point(1.0, 0.0, 0.0), point(0.0, 1.0, 0.0)], vec![0, 1, 2]).unwrap();
        let topology = mesh.topology();
        assert_eq!(topology.boundary_edges().count(), 3);
        assert_eq!(topology.edges_with_more_than_two_faces().count(), 0);
        assert!(topology.edge_faces(topology.edge_count()).is_none());
        assert!(topology.face_edges(1).is_none());
    }

    #[test]
    fn topology_handles_disconnected_components_and_unreferenced_vertices() {
        let mesh = TriangleMesh3::new(
            vec![point(0.0, 0.0, 0.0), point(1.0, 0.0, 0.0), point(0.0, 1.0, 0.0), point(3.0, 0.0, 0.0), point(4.0, 0.0, 0.0), point(3.0, 1.0, 0.0), point(9.0, 9.0, 9.0)],
            vec![0, 1, 2, 3, 4, 5],
        ).unwrap();
        let topology = mesh.topology();
        assert_eq!(topology.edge_count(), 6);
        assert_eq!(topology.face_count(), 2);
        assert!(topology.edge_faces(0).unwrap().iter().all(|&face| face == 0));
        assert!(topology.edge_faces(5).unwrap().iter().all(|&face| face == 1));
    }

    #[test]
    fn topology_reports_edges_incident_to_more_than_two_faces() {
        let mesh = TriangleMesh3::new(
            vec![point(0.0, 0.0, 0.0), point(1.0, 0.0, 0.0), point(0.0, 1.0, 0.0), point(0.0, -1.0, 0.0), point(0.0, 0.0, 1.0)],
            vec![0, 1, 2, 1, 0, 3, 0, 1, 4],
        ).unwrap();
        let topology = mesh.topology();
        let over_shared_edge: Vec<_> = topology.edges_with_more_than_two_faces().collect();
        assert_eq!(over_shared_edge.len(), 1);
        let (edge_id, edge) = over_shared_edge[0];
        assert_eq!(edge.vertices(), [0, 1]);
        assert_eq!(topology.edge_faces(edge_id), Some(&[0, 1, 2][..]));
    }

    #[test]
    fn topology_edge_ids_are_deterministic_and_face_order_is_preserved() {
        let mesh = TriangleMesh3::new(vec![point(0.0, 0.0, 0.0), point(1.0, 0.0, 0.0), point(0.0, 1.0, 0.0), point(1.0, 1.0, 0.0)], vec![2, 1, 3, 0, 2, 1]).unwrap();
        let first = mesh.topology();
        let second = mesh.topology();
        assert_eq!(first, second);
        assert_eq!(first.face_edges(0).unwrap().map(|id| first.edges()[id].vertices()), [[1, 2], [1, 3], [2, 3]]);
        assert_eq!(first.edge_faces(first.face_edges(0).unwrap()[0]), Some(&[0, 1][..]));
        assert!(first.face_edges(2).is_none());
    }

    #[test]
    fn topology_of_mesh_without_faces_has_no_edges_or_incidence() {
        let mesh = TriangleMesh3::new(vec![point(0.0, 0.0, 0.0), point(1.0, 0.0, 0.0)], Vec::new()).unwrap();
        let topology = mesh.topology();
        assert_eq!(topology.edge_count(), 0);
        assert_eq!(topology.face_count(), 0);
        assert_eq!(topology.boundary_edges().count(), 0);
        assert_eq!(topology.edges_with_more_than_two_faces().count(), 0);
    }
    #[test]
    fn point_set_preserves_order_and_allows_empty_data() {
        let empty = PointSet3::new(Vec::new()).unwrap();
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);

        let points = vec![point(1.0, 2.0, 3.0), point(-4.0, 5.0, -6.0)];
        let set = PointSet3::new(points.clone()).unwrap();
        assert_eq!(set.len(), 2);
        assert_eq!(set.points(), points);
        assert_eq!(set.into_points(), points);
    }

    #[test]
    fn point_set_rejects_non_finite_coordinates_with_index() {
        let result = PointSet3::new(vec![point(0.0, 0.0, 0.0), point(f64::NAN, 1.0, 2.0)]);
        assert_eq!(result, Err(GeometryError::NonFinitePoint { index: 1 }));
    }

    #[test]
    fn line_segment_preserves_endpoints_and_computes_length() {
        let segment = LineSegment3::new(point(1.0, 2.0, 3.0), point(4.0, 6.0, 3.0)).unwrap();
        assert_eq!(segment.start(), point(1.0, 2.0, 3.0));
        assert_eq!(segment.end(), point(4.0, 6.0, 3.0));
        assert_eq!(segment.displacement().to_vector().to_array(), [3.0, 4.0, 0.0]);
        assert_eq!(segment.length(), 5.0);
    }

    #[test]
    fn line_segment_allows_coincident_endpoints() {
        let segment = LineSegment3::new(point(2.0, 3.0, 4.0), point(2.0, 3.0, 4.0)).unwrap();
        assert_eq!(segment.length(), 0.0);
    }

    #[test]
    fn line_segment_rejects_non_finite_endpoint() {
        let result = LineSegment3::new(point(0.0, 0.0, 0.0), point(0.0, f64::INFINITY, 0.0));
        assert_eq!(result, Err(GeometryError::NonFinitePoint { index: 1 }));
    }

    #[test]
    fn polyline_requires_at_least_two_points() {
        assert_eq!(
            Polyline3::new(Vec::new()),
            Err(GeometryError::PolylineRequiresTwoPoints { actual: 0 })
        );
        assert_eq!(
            Polyline3::new(vec![point(0.0, 0.0, 0.0)]),
            Err(GeometryError::PolylineRequiresTwoPoints { actual: 1 })
        );
    }

    #[test]
    fn polyline_segments_follow_point_order() {
        let polyline = Polyline3::new(vec![
            point(0.0, 0.0, 0.0),
            point(3.0, 0.0, 0.0),
            point(3.0, 4.0, 0.0),
        ])
        .unwrap();

        let segments: Vec<_> = polyline.segments().collect();
        assert_eq!(polyline.len(), 3);
        assert_eq!(polyline.segment_count(), 2);
        assert_eq!(segments[0].start(), point(0.0, 0.0, 0.0));
        assert_eq!(segments[0].end(), point(3.0, 0.0, 0.0));
        assert_eq!(segments[1].start(), point(3.0, 0.0, 0.0));
        assert_eq!(segments[1].end(), point(3.0, 4.0, 0.0));
        assert_eq!(segments.len(), 2);
    }

    #[test]
    fn polyline_preserves_repeated_points() {
        let repeated = point(1.0, 1.0, 1.0);
        let polyline = Polyline3::new(vec![repeated, repeated, point(2.0, 1.0, 1.0)]).unwrap();

        let segments: Vec<_> = polyline.segments().collect();
        assert_eq!(segments[0].length(), 0.0);
        assert_eq!(segments[1].length(), 1.0);
    }

    #[test]
    fn polyline_rejects_non_finite_coordinates_with_index() {
        let result = Polyline3::new(vec![point(0.0, 0.0, 0.0), point(1.0, 0.0, f64::NEG_INFINITY)]);
        assert_eq!(result, Err(GeometryError::NonFinitePoint { index: 1 }));
    }

    #[test]
    fn closed_polyline_is_explicitly_closed_by_repeating_first_point() {
        let first = point(0.0, 0.0, 0.0);
        let polyline = Polyline3::new(vec![
            first,
            point(1.0, 0.0, 0.0),
            point(0.0, 1.0, 0.0),
            first,
        ])
        .unwrap();

        assert_eq!(polyline.segment_count(), 3);
        assert_eq!(polyline.points().first(), polyline.points().last());
    }
}
