//! Tetrahedral isosurface extraction for point-associated scalar fields.
//! All scientific calculations and output coordinates remain f64.

use std::{collections::BTreeMap, error::Error, fmt};

use crate::{
    dataset::{Dataset, DatasetGeometry},
    fields::{FieldAssociation, FieldError, ScalarField},
    geometry::{GeometryError, TriangleMesh3},
    spatial::WorldPoint3,
    unstructured::{CellType3, UnstructuredGrid3},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IsosurfaceVertexProvenance3 {
    SourcePoint { point_id: u32 },
    SourceEdge { point_ids: [u32; 2], t: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnsupportedIsosurfaceCell3 {
    pub cell_id: usize,
    pub cell_type: CellType3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IsosurfaceSkipReason3 {
    NonFiniteScalar,
    DegenerateTetrahedron,
    InvalidInterpolation,
    DegenerateOutputTriangle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkippedIsosurfaceCell3 {
    pub cell_id: usize,
    pub reason: IsosurfaceSkipReason3,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IsosurfaceMetadata3 {
    unsupported_cells: Vec<UnsupportedIsosurfaceCell3>,
    skipped_cells: Vec<SkippedIsosurfaceCell3>,
}

impl IsosurfaceMetadata3 {
    #[must_use]
    pub fn unsupported_cells(&self) -> &[UnsupportedIsosurfaceCell3] {
        &self.unsupported_cells
    }

    #[must_use]
    pub fn skipped_cells(&self) -> &[SkippedIsosurfaceCell3] {
        &self.skipped_cells
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Isosurface3 {
    mesh: TriangleMesh3,
    vertex_provenance: Vec<IsosurfaceVertexProvenance3>,
    triangle_source_cells: Vec<usize>,
    isovalue: f64,
    metadata: IsosurfaceMetadata3,
}

impl Isosurface3 {
    #[must_use]
    pub fn mesh(&self) -> &TriangleMesh3 { &self.mesh }

    #[must_use]
    pub fn vertex_provenance(&self) -> &[IsosurfaceVertexProvenance3] {
        &self.vertex_provenance
    }

    #[must_use]
    pub fn triangle_source_cells(&self) -> &[usize] { &self.triangle_source_cells }

    #[must_use]
    pub const fn isovalue(&self) -> f64 { self.isovalue }

    #[must_use]
    pub fn metadata(&self) -> &IsosurfaceMetadata3 { &self.metadata }
}

#[derive(Clone, Debug, PartialEq)]
pub enum IsosurfaceError3 {
    NonFiniteIsovalue,
    DatasetGeometryMustBeUnstructured,
    ScalarFieldNotFound { name: String },
    ScalarFieldMustBePointAssociated { actual: FieldAssociation },
    InvalidFieldCardinality(FieldError),
    InvalidOutputGeometry(GeometryError),
}

impl fmt::Display for IsosurfaceError3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteIsovalue => write!(f, "isosurface value must be finite"),
            Self::DatasetGeometryMustBeUnstructured =>
                write!(f, "isosurface extraction requires UnstructuredGrid3 geometry"),
            Self::ScalarFieldNotFound { name } =>
                write!(f, "dataset has no scalar field named '{name}'"),
            Self::ScalarFieldMustBePointAssociated { actual } =>
                write!(f, "isosurface extraction requires a point-associated field; received {actual:?}"),
            Self::InvalidFieldCardinality(error) => write!(f, "invalid scalar field: {error}"),
            Self::InvalidOutputGeometry(error) => write!(f, "invalid extracted mesh: {error}"),
        }
    }
}
impl Error for IsosurfaceError3 {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum VertexKey {
    Point(u32),
    Edge(u32, u32),
}

#[derive(Clone, Copy)]
struct Crossing {
    key: VertexKey,
    position: WorldPoint3,
    provenance: IsosurfaceVertexProvenance3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct FaceKey([u32; 3]);

impl FaceKey {
    fn new(mut ids: [u32; 3]) -> Self {
        ids.sort_unstable();
        Self(ids)
    }
}

#[derive(Clone, Copy)]
struct FaceSide {
    cell_id: usize,
    side: i8,
}

const EDGES: [(usize, usize); 6] = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
const FACES: [([usize; 3], usize); 4] = [
    ([0, 1, 2], 3), ([0, 1, 3], 2), ([0, 2, 3], 1), ([1, 2, 3], 0),
];

/// Extracts an isosurface from a point-associated scalar field on tetrahedra.
///
/// Values strictly below the isovalue are low; values equal to or above it
/// are high. An exact-isovalue face shared by tetrahedra is emitted only when
/// incident finite tetrahedra occupy opposite scalar sides. For a
/// non-manifold face, the lowest-ID low-side tetrahedron owns the face.
pub fn extract_tetrahedral_isosurface(
    dataset: &Dataset,
    field_name: &str,
    isovalue: f64,
) -> Result<Isosurface3, IsosurfaceError3> {
    if !isovalue.is_finite() {
        return Err(IsosurfaceError3::NonFiniteIsovalue);
    }
    let grid = match dataset.geometry() {
        DatasetGeometry::UnstructuredGrid3(grid) => grid,
        _ => return Err(IsosurfaceError3::DatasetGeometryMustBeUnstructured),
    };
    let field = dataset.scalar_field(field_name)
        .ok_or_else(|| IsosurfaceError3::ScalarFieldNotFound { name: field_name.to_owned() })?;
    if field.association() != FieldAssociation::Point {
        return Err(IsosurfaceError3::ScalarFieldMustBePointAssociated {
            actual: field.association(),
        });
    }
    field.validate_cardinality(grid.point_count(), grid.cell_count())
        .map_err(IsosurfaceError3::InvalidFieldCardinality)?;

    let mut metadata = IsosurfaceMetadata3::default();
    for (cell_id, cell_type) in grid.cell_types().iter().copied().enumerate() {
        if cell_type != CellType3::Tetrahedron4 {
            metadata.unsupported_cells.push(UnsupportedIsosurfaceCell3 { cell_id, cell_type });
        }
    }
    let face_owners = exact_face_owners(grid, field, isovalue);
    let mut positions = Vec::new();
    let mut indices = Vec::new();
    let mut provenance = Vec::new();
    let mut triangle_cells = Vec::new();
    let mut vertex_cache = BTreeMap::<VertexKey, u32>::new();

    for cell_id in 0..grid.cell_count() {
        if grid.cell_type(cell_id) != Some(CellType3::Tetrahedron4) { continue; }
        let source_ids = grid.cell_point_indices(cell_id).expect("validated cell");
        let ids = [source_ids[0], source_ids[1], source_ids[2], source_ids[3]];
        let samples = ids.map(|id| field.values()[id as usize].as_finite());
        if samples.iter().any(Option::is_none) {
            metadata.skipped_cells.push(SkippedIsosurfaceCell3 {
                cell_id, reason: IsosurfaceSkipReason3::NonFiniteScalar,
            });
            continue;
        }
        let scalars = samples.map(|value| value.expect("finite values checked"));
        let points = ids.map(|id| grid.points()[id as usize]);

        let mut crossings = BTreeMap::<VertexKey, Crossing>::new();
        let mut interpolation_failed = false;
        for (a, b) in EDGES {
            if (scalars[a] < isovalue) == (scalars[b] < isovalue) { continue; }
            match crossing(ids[a], points[a], scalars[a], ids[b], points[b], scalars[b], isovalue) {
                Some(vertex) => { crossings.insert(vertex.key, vertex); }
                None => { interpolation_failed = true; break; }
            }
        }
        if interpolation_failed {
            metadata.skipped_cells.push(SkippedIsosurfaceCell3 {
                cell_id, reason: IsosurfaceSkipReason3::InvalidInterpolation,
            });
            continue;
        }
        if crossings.len() < 3 { continue; }

        let Some(gradient) = scalar_gradient(points, scalars) else {
            metadata.skipped_cells.push(SkippedIsosurfaceCell3 {
                cell_id, reason: IsosurfaceSkipReason3::DegenerateTetrahedron,
            });
            continue;
        };
        let mut polygon = crossings.into_values().collect::<Vec<_>>();
        if polygon.len() == 4 && !sort_quad(&mut polygon, gradient) {
            metadata.skipped_cells.push(SkippedIsosurfaceCell3 {
                cell_id, reason: IsosurfaceSkipReason3::DegenerateTetrahedron,
            });
            continue;
        }
        if polygon.len() != 3 && polygon.len() != 4 { continue; }

        // A polygon consisting of three exact source points is an
        // isovalue-coincident tetrahedron face; consult its adjacency owner.
        if polygon.len() == 3 && polygon.iter().all(|v| matches!(v.key, VertexKey::Point(_))) {
            let face = FaceKey::new([
                point_key(polygon[0].key), point_key(polygon[1].key), point_key(polygon[2].key),
            ]);
            if face_owners.get(&face).copied().flatten() != Some(cell_id) { continue; }
        }

        let candidates = triangulate(&polygon);
        let mut accepted = Vec::new();
        let mut degenerate = false;
        for candidate in candidates {
            if let Some(triangle) = orient_triangle(candidate, gradient) {
                accepted.push(triangle);
            } else {
                degenerate = true;
            }
        }
        if degenerate {
            metadata.skipped_cells.push(SkippedIsosurfaceCell3 {
                cell_id, reason: IsosurfaceSkipReason3::DegenerateOutputTriangle,
            });
            continue;
        }

        for triangle in accepted {
            let mut output = [0_u32; 3];
            for (corner, vertex) in triangle.into_iter().enumerate() {
                let index = if let Some(index) = vertex_cache.get(&vertex.key) {
                    *index
                } else {
                    let index = u32::try_from(positions.len()).map_err(|_| {
                        IsosurfaceError3::InvalidOutputGeometry(
                            GeometryError::TriangleIndexOutOfBounds {
                                triangle_index: indices.len() / 3,
                                corner,
                                index: u32::MAX,
                                vertex_count: positions.len(),
                            },
                        )
                    })?;
                    positions.push(vertex.position);
                    provenance.push(vertex.provenance);
                    vertex_cache.insert(vertex.key, index);
                    index
                };
                output[corner] = index;
            }
            if output[0] != output[1] && output[0] != output[2] && output[1] != output[2] {
                indices.extend_from_slice(&output);
                triangle_cells.push(cell_id);
            }
        }
    }

    let mesh = TriangleMesh3::new(positions, indices)
        .map_err(IsosurfaceError3::InvalidOutputGeometry)?;
    Ok(Isosurface3 {
        mesh, vertex_provenance: provenance, triangle_source_cells: triangle_cells,
        isovalue, metadata,
    })
}

fn point_key(key: VertexKey) -> u32 {
    match key { VertexKey::Point(id) => id, VertexKey::Edge(_, _) => unreachable!() }
}

fn exact_face_owners(
    grid: &UnstructuredGrid3,
    field: &ScalarField,
    iso: f64,
) -> BTreeMap<FaceKey, Option<usize>> {
    let mut faces = BTreeMap::<FaceKey, Vec<FaceSide>>::new();
    for cell_id in 0..grid.cell_count() {
        if grid.cell_type(cell_id) != Some(CellType3::Tetrahedron4) { continue; }
        let ids = grid.cell_point_indices(cell_id).expect("validated cell");
        for (face, opposite) in FACES {
            let face_ids = [ids[face[0]], ids[face[1]], ids[face[2]]];
            if !face_ids.iter().all(|id| field.values()[*id as usize].as_finite() == Some(iso)) {
                continue;
            }
            let Some(value) = field.values()[ids[opposite] as usize].as_finite() else { continue; };
            let side = if value < iso { -1 } else if value > iso { 1 } else { 0 };
            faces.entry(FaceKey::new(face_ids)).or_default().push(FaceSide { cell_id, side });
        }
    }
    faces.into_iter().map(|(face, entries)| {
        let low = entries.iter().any(|entry| entry.side < 0);
        let high = entries.iter().any(|entry| entry.side >= 0);
        let owner = if entries.len() == 1 && low {
            entries.first().map(|entry| entry.cell_id)
        } else if low && high {
            entries.iter().filter(|entry| entry.side < 0)
                .map(|entry| entry.cell_id).min()
        } else { None };
        (face, owner)
    }).collect()
}

fn crossing(
    id_a: u32, point_a: WorldPoint3, value_a: f64,
    id_b: u32, point_b: WorldPoint3, value_b: f64, iso: f64,
) -> Option<Crossing> {
    if value_a == iso {
        return Some(Crossing { key: VertexKey::Point(id_a), position: point_a,
            provenance: IsosurfaceVertexProvenance3::SourcePoint { point_id: id_a } });
    }
    if value_b == iso {
        return Some(Crossing { key: VertexKey::Point(id_b), position: point_b,
            provenance: IsosurfaceVertexProvenance3::SourcePoint { point_id: id_b } });
    }
    let (lo_id, lo_point, lo_value, hi_id, hi_point, hi_value) = if id_a < id_b {
        (id_a, point_a, value_a, id_b, point_b, value_b)
    } else { (id_b, point_b, value_b, id_a, point_a, value_a) };
    let scale_value = lo_value.abs().max(hi_value.abs()).max(iso.abs());
    if scale_value == 0.0 || !scale_value.is_finite() { return None; }
    let lo = lo_value / scale_value;
    let hi = hi_value / scale_value;
    let level = iso / scale_value;
    let denominator = hi - lo;
    if denominator == 0.0 || !denominator.is_finite() { return None; }
    let t = (level - lo) / denominator;
    if !t.is_finite() || !(0.0..=1.0).contains(&t) { return None; }
    let a = lo_point.to_array();
    let b = hi_point.to_array();
    let position = WorldPoint3::new(
        a[0] * (1.0 - t) + b[0] * t,
        a[1] * (1.0 - t) + b[1] * t,
        a[2] * (1.0 - t) + b[2] * t,
    );
    if !position.to_vector().is_finite() { return None; }
    Some(Crossing {
        key: VertexKey::Edge(lo_id, hi_id), position,
        provenance: IsosurfaceVertexProvenance3::SourceEdge { point_ids: [lo_id, hi_id], t },
    })
}

fn scalar_gradient(points: [WorldPoint3; 4], values: [f64; 4]) -> Option<[f64; 3]> {
    let p = points.map(WorldPoint3::to_array);
    let a = subtract(p[1], p[0])?;
    let b = subtract(p[2], p[0])?;
    let c = subtract(p[3], p[0])?;
    let coordinate_scale = a.into_iter().chain(b).chain(c).map(f64::abs).fold(0.0, f64::max);
    if coordinate_scale == 0.0 { return None; }
    let a = divide(a, coordinate_scale);
    let b = divide(b, coordinate_scale);
    let c = divide(c, coordinate_scale);
    let determinant = dot(a, cross(b, c));
    if determinant == 0.0 || !determinant.is_finite() { return None; }
    let value_scale = values.iter().map(|v| v.abs()).fold(0.0, f64::max);
    if value_scale == 0.0 { return None; }
    let d1 = values[1] / value_scale - values[0] / value_scale;
    let d2 = values[2] / value_scale - values[0] / value_scale;
    let d3 = values[3] / value_scale - values[0] / value_scale;
    let numerator = add(add(multiply(cross(b, c), d1), multiply(cross(c, a), d2)),
        multiply(cross(a, b), d3));
    normalize(divide(numerator, determinant))
}

fn sort_quad(poly: &mut [Crossing], normal: [f64; 3]) -> bool {
    let reference = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
        .into_iter().min_by(|a, b| dot(*a, normal).abs().total_cmp(&dot(*b, normal).abs()))
        .expect("three reference axes");
    let Some(u) = normalize(cross(normal, reference)) else { return false; };
    let v = cross(normal, u);
    let Some(center) = centroid(&poly.iter().map(|v| v.position).collect::<Vec<_>>()) else {
        return false;
    };
    let mut order = Vec::new();
    for (index, vertex) in poly.iter().enumerate() {
        let Some(delta) = subtract(vertex.position.to_array(), center) else { return false; };
        order.push((dot(delta, v).atan2(dot(delta, u)), index));
    }
    order.sort_by(|a, b| a.0.total_cmp(&b.0));
    let original = poly.to_vec();
    for (destination, (_, source)) in order.into_iter().enumerate() {
        poly[destination] = original[source];
    }
    true
}

fn triangulate(poly: &[Crossing]) -> Vec<[Crossing; 3]> {
    if poly.len() == 3 { return vec![[poly[0], poly[1], poly[2]]]; }
    let d02 = ordered_pair(poly[0].key, poly[2].key);
    let d13 = ordered_pair(poly[1].key, poly[3].key);
    if d02 <= d13 {
        vec![[poly[0], poly[1], poly[2]], [poly[0], poly[2], poly[3]]]
    } else {
        vec![[poly[0], poly[1], poly[3]], [poly[1], poly[2], poly[3]]]
    }
}

fn ordered_pair(a: VertexKey, b: VertexKey) -> (VertexKey, VertexKey) {
    if a <= b { (a, b) } else { (b, a) }
}

fn orient_triangle(mut tri: [Crossing; 3], gradient: [f64; 3]) -> Option<[Crossing; 3]> {
    let a = tri[0].position.to_array();
    let b = tri[1].position.to_array();
    let c = tri[2].position.to_array();
    let ab = subtract(b, a)?;
    let ac = subtract(c, a)?;
    let factor = ab.into_iter().chain(ac).map(f64::abs).fold(0.0, f64::max);
    if factor == 0.0 { return None; }
    let direction = dot(cross(divide(ab, factor), divide(ac, factor)), gradient);
    if !direction.is_finite() || direction == 0.0 { return None; }
    if direction < 0.0 { tri.swap(1, 2); }
    Some(tri)
}

fn centroid(points: &[WorldPoint3]) -> Option<[f64; 3]> {
    let arrays = points.iter().map(|p| p.to_array()).collect::<Vec<_>>();
    let mut result = [0.0; 3];
    for axis in 0..3 {
        let factor = arrays.iter().map(|p| p[axis].abs()).fold(0.0, f64::max);
        if factor == 0.0 { continue; }
        let mean = arrays.iter().map(|p| p[axis] / factor).sum::<f64>() / arrays.len() as f64;
        result[axis] = mean * factor;
    }
    result.iter().all(|v| v.is_finite()).then_some(result)
}

fn subtract(a: [f64; 3], b: [f64; 3]) -> Option<[f64; 3]> {
    let r = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    r.iter().all(|v| v.is_finite()).then_some(r)
}
fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn multiply(a: [f64; 3], s: f64) -> [f64; 3] { [a[0] * s, a[1] * s, a[2] * s] }
fn divide(a: [f64; 3], s: f64) -> [f64; 3] { [a[0] / s, a[1] / s, a[2] / s] }
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 { a[0]*b[0] + a[1]*b[1] + a[2]*b[2] }
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]]
}
fn normalize(a: [f64; 3]) -> Option<[f64; 3]> {
    let factor = a.iter().map(|v| v.abs()).fold(0.0, f64::max);
    if factor == 0.0 || !factor.is_finite() { return None; }
    let scaled = divide(a, factor);
    let length = dot(scaled, scaled).sqrt();
    let result = divide(scaled, length);
    result.iter().all(|v| v.is_finite()).then_some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{fields::FieldMetadata, sampling::SampleValue};

    fn point(x: f64, y: f64, z: f64) -> WorldPoint3 { WorldPoint3::new(x, y, z) }

    fn dataset(values: [f64; 4]) -> Dataset {
        let grid = UnstructuredGrid3::new(
            vec![point(0.0,0.0,0.0), point(1.0,0.0,0.0),
                 point(0.0,1.0,0.0), point(0.0,0.0,1.0)],
            vec![CellType3::Tetrahedron4], vec![0,4], vec![0,1,2,3],
        ).unwrap();
        let mut dataset = Dataset::new(grid);
        dataset.add_scalar_field(ScalarField::new(
            FieldMetadata::new("value", None).unwrap(), FieldAssociation::Point,
            values.map(SampleValue::from_f64).to_vec(),
        ).unwrap()).unwrap();
        dataset
    }

    #[test]
    fn all_sixteen_tetrahedron_patterns_produce_expected_triangle_counts() {
        for pattern in 0_u8..16 {
            let values = std::array::from_fn(|i| if pattern & (1 << i) != 0 { 0.0 } else { 1.0 });
            let result = extract_tetrahedral_isosurface(&dataset(values), "value", 0.5).unwrap();
            let expected = match pattern.count_ones() { 0 | 4 => 0, 1 | 3 => 1, 2 => 2, _ => unreachable!() };
            assert_eq!(result.mesh().triangle_count(), expected, "pattern {pattern:04b}");
        }
    }

    #[test]
    fn edge_provenance_and_source_cell_ids_are_preserved() {
        let result = extract_tetrahedral_isosurface(&dataset([0.0,1.0,1.0,1.0]), "value", 0.5).unwrap();
        assert_eq!(result.mesh().triangle_count(), 1);
        assert_eq!(result.triangle_source_cells(), &[0]);
        assert_eq!(result.vertex_provenance().len(), 3);
        assert!(result.vertex_provenance().iter().all(|entry| matches!(
            entry, IsosurfaceVertexProvenance3::SourceEdge { t, .. } if (*t - 0.5).abs() < 1e-12
        )));
    }

    #[test]
    fn exact_isovalue_vertices_are_reused_as_source_points() {
        let result = extract_tetrahedral_isosurface(&dataset([0.0,0.5,0.5,0.5]), "value", 0.5).unwrap();
        assert_eq!(result.mesh().triangle_count(), 1);
        assert_eq!(result.mesh().vertex_count(), 3);
        assert!(result.vertex_provenance().iter().all(|entry|
            matches!(entry, IsosurfaceVertexProvenance3::SourcePoint { .. })));
    }

    #[test]
    fn triangle_winding_points_toward_increasing_scalar_values() {
        let result = extract_tetrahedral_isosurface(&dataset([0.0,1.0,1.0,1.0]), "value", 0.5).unwrap();
        let [a,b,c] = result.mesh().triangles().next().unwrap();
        let vertices = result.mesh().vertices();
        let ab = subtract(vertices[b as usize].to_array(), vertices[a as usize].to_array()).unwrap();
        let ac = subtract(vertices[c as usize].to_array(), vertices[a as usize].to_array()).unwrap();
        assert!(dot(cross(ab, ac), [1.0,1.0,1.0]) > 0.0);
    }

    #[test]
    fn non_finite_isovalue_is_rejected() {
        assert_eq!(extract_tetrahedral_isosurface(&dataset([0.0;4]), "value", f64::NAN),
            Err(IsosurfaceError3::NonFiniteIsovalue));
    }

    #[test]
    fn non_finite_source_scalar_skips_cell_without_mutating_dataset() {
        let source = dataset([0.0,f64::NAN,1.0,1.0]);
        let before = source.clone();
        let result = extract_tetrahedral_isosurface(&source, "value", 0.5).unwrap();
        assert_eq!(source, before);
        assert_eq!(result.metadata().skipped_cells(), &[SkippedIsosurfaceCell3 {
            cell_id: 0, reason: IsosurfaceSkipReason3::NonFiniteScalar,
        }]);
        assert!(result.mesh().is_empty());
    }

    #[test]
    fn exact_shared_face_is_emitted_once_for_opposite_scalar_sides() {
        let grid = UnstructuredGrid3::new(
            vec![point(0.0,0.0,0.0), point(1.0,0.0,0.0), point(0.0,1.0,0.0),
                 point(0.0,0.0,1.0), point(0.0,0.0,-1.0)],
            vec![CellType3::Tetrahedron4, CellType3::Tetrahedron4],
            vec![0,4,8], vec![0,1,2,3, 0,2,1,4],
        ).unwrap();
        let mut source = Dataset::new(grid);
        source.add_scalar_field(ScalarField::new(
            FieldMetadata::new("value", None).unwrap(), FieldAssociation::Point,
            vec![0.5,0.5,0.5,0.0,1.0].into_iter().map(SampleValue::Finite).collect(),
        ).unwrap()).unwrap();
        let result = extract_tetrahedral_isosurface(&source, "value", 0.5).unwrap();
        assert_eq!(result.mesh().triangle_count(), 1);
        assert_eq!(result.triangle_source_cells(), &[0]);
    }

    #[test]
    fn exact_shared_face_between_low_and_equal_tetrahedra_is_emitted_once() {
        let grid = UnstructuredGrid3::new(
            vec![point(0.0,0.0,0.0), point(1.0,0.0,0.0), point(0.0,1.0,0.0),
                 point(0.0,0.0,1.0), point(0.0,0.0,-1.0)],
            vec![CellType3::Tetrahedron4, CellType3::Tetrahedron4],
            vec![0,4,8], vec![0,1,2,3, 0,2,1,4],
        ).unwrap();
        let mut source = Dataset::new(grid);
        source.add_scalar_field(ScalarField::new(
            FieldMetadata::new("value", None).unwrap(), FieldAssociation::Point,
            vec![0.5,0.5,0.5,0.0,0.5].into_iter().map(SampleValue::Finite).collect(),
        ).unwrap()).unwrap();
        let result = extract_tetrahedral_isosurface(&source, "value", 0.5).unwrap();
        assert_eq!(result.mesh().triangle_count(), 1);
        assert_eq!(result.triangle_source_cells(), &[0]);
    }

    #[test]
    fn exact_shared_face_with_both_tetrahedra_on_low_side_is_suppressed() {
        let grid = UnstructuredGrid3::new(
            vec![point(0.0,0.0,0.0), point(1.0,0.0,0.0), point(0.0,1.0,0.0),
                 point(0.0,0.0,1.0), point(0.0,0.0,-1.0)],
            vec![CellType3::Tetrahedron4, CellType3::Tetrahedron4],
            vec![0,4,8], vec![0,1,2,3, 0,2,1,4],
        ).unwrap();
        let mut source = Dataset::new(grid);
        source.add_scalar_field(ScalarField::new(
            FieldMetadata::new("value", None).unwrap(), FieldAssociation::Point,
            vec![0.5,0.5,0.5,0.0,0.0].into_iter().map(SampleValue::Finite).collect(),
        ).unwrap()).unwrap();
        let result = extract_tetrahedral_isosurface(&source, "value", 0.5).unwrap();
        assert!(result.mesh().is_empty());
    }

    #[test]
    fn non_tetrahedral_cells_are_reported_as_unsupported() {
        let grid = UnstructuredGrid3::new(
            vec![point(0.0,0.0,0.0),point(1.0,0.0,0.0),point(0.0,1.0,0.0)],
            vec![CellType3::Triangle3], vec![0,3], vec![0,1,2],
        ).unwrap();
        let mut source = Dataset::new(grid);
        source.add_scalar_field(ScalarField::new(
            FieldMetadata::new("value", None).unwrap(), FieldAssociation::Point,
            vec![SampleValue::Finite(0.0);3],
        ).unwrap()).unwrap();
        let result = extract_tetrahedral_isosurface(&source, "value", 0.5).unwrap();
        assert_eq!(result.metadata().unsupported_cells(), &[UnsupportedIsosurfaceCell3 {
            cell_id: 0, cell_type: CellType3::Triangle3,
        }]);
        assert!(result.mesh().is_empty());
    }
}
