//! Axial line glyphs for a cell-associated direction field on a boundary surface.
//!
//! This is the B1.2 operation (`docs/heart-fibre-orientation.md`). It selects
//! triangulated boundary triangles by an explicit set of integer cell tags,
//! never by the direction values themselves, and builds one centred,
//! unoriented segment per occupied spatial bin along the exact source
//! direction of the triangle's source cell.
//!
//! Numerical contract (design §4, §6, §7.2), reproduced independently by
//! `tools/rodero-fibre-audit/audit.py`:
//! - geometric normal n = (b − a) × (c − a) in stored vertex order; its sign
//!   is not certified outward and nothing here depends on it;
//! - centroid p_c = ((a + b) + c) / 3.0 per coordinate in f64, stored order;
//! - |cos| = |f̂ · n̂| with f̂ = f · (1/‖f‖) and n̂ = n · (1/‖n‖). Where
//!   squaring would overflow or underflow, the vector (or, for n, each edge)
//!   is first divided by its largest component; elsewhere the result is
//!   bitwise identical to the audit's √(u·u) form;
//! - bin index ⌊p_c / h⌋ per axis, with the grid origin at the world origin;
//! - the winner of a bin is its eligible triangle with the lowest surface
//!   index (a deterministic tie-break, not a ranking);
//! - glyph p_c ± (ℓ/2) f̂, not projected onto the triangle plane.
//!
//! It is specific to this experiment and is not a general glyph framework.

use std::{collections::BTreeMap, error::Error, fmt};

use crate::{
    dataset::{Dataset, DatasetField, DatasetGeometry},
    fields::FieldAssociation,
    sampling::SampleValue,
    spatial::{WorldPoint3, WorldVector3},
    surface_triangulation::TriangulatedSurface3,
};

/// Largest magnitude below which every integer is exactly representable in f64.
const MAX_EXACT_INTEGER: f64 = 9_007_199_254_740_992.0;

/// Thinning and display parameters for [`build_boundary_axial_glyphs`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundaryAxialGlyphOptions3 {
    /// Edge length h of the world-origin cubic bins, in source length units.
    pub bin_edge: f64,
    /// Uniform glyph length ℓ, in source length units. It encodes no magnitude.
    pub length: f64,
}

impl Default for BoundaryAxialGlyphOptions3 {
    /// Provisional defaults from design §6.5: h = 4 and ℓ = 3 (mm in case 01).
    fn default() -> Self {
        Self { bin_edge: 4.0, length: 3.0 }
    }
}

/// One eligible boundary triangle: in the tag set, non-degenerate, valid direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundaryAxialSample3 {
    triangle_index: usize,
    source_cell_index: usize,
    tag: i64,
    direction: WorldVector3,
    unit_direction: WorldVector3,
    centroid: WorldPoint3,
    unit_normal: WorldVector3,
    abs_cos: f64,
    bin: [i64; 3],
}

impl BoundaryAxialSample3 {
    /// Returns the triangle's index in the triangulated surface.
    #[must_use]
    pub const fn triangle_index(&self) -> usize { self.triangle_index }

    /// Returns the source volume cell index.
    #[must_use]
    pub const fn source_cell_index(&self) -> usize { self.source_cell_index }

    /// Returns the source cell's integer tag.
    #[must_use]
    pub const fn tag(&self) -> i64 { self.tag }

    /// Returns the unmodified f64 source direction of the cell.
    #[must_use]
    pub const fn direction(&self) -> WorldVector3 { self.direction }

    /// Returns f̂, the unit source direction used for θ and the glyph.
    #[must_use]
    pub const fn unit_direction(&self) -> WorldVector3 { self.unit_direction }

    /// Returns the pinned f64 centroid ((a + b) + c) / 3.0.
    #[must_use]
    pub const fn centroid(&self) -> WorldPoint3 { self.centroid }

    /// Returns the unit geometric normal. Its sign follows the stored vertex
    /// cycle and is not certified outward.
    #[must_use]
    pub const fn unit_normal(&self) -> WorldVector3 { self.unit_normal }

    /// Returns |f̂ · n̂|, independent of the signs of f and n.
    #[must_use]
    pub const fn abs_cos(&self) -> f64 { self.abs_cos }

    /// Returns the surface-tangency angle θ = arccos(|f̂ · n̂|) in degrees, in
    /// [0, 90]. 90° means the direction lies in the triangle's plane.
    #[must_use]
    pub fn tangency_angle_degrees(&self) -> f64 { self.abs_cos.min(1.0).acos().to_degrees() }

    /// Returns the world-origin floor bin index of the centroid.
    #[must_use]
    pub const fn bin(&self) -> [i64; 3] { self.bin }
}

/// One thinned glyph: a bin's winning sample and its centred axial segment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundaryAxialGlyph3 {
    sample: BoundaryAxialSample3,
    start: WorldPoint3,
    end: WorldPoint3,
}

impl BoundaryAxialGlyph3 {
    /// Returns the winning sample and its provenance.
    #[must_use]
    pub const fn sample(&self) -> BoundaryAxialSample3 { self.sample }

    /// Returns p_c − (ℓ/2) f̂.
    #[must_use]
    pub const fn start(&self) -> WorldPoint3 { self.start }

    /// Returns p_c + (ℓ/2) f̂.
    #[must_use]
    pub const fn end(&self) -> WorldPoint3 { self.end }
}

/// The operation's result: eligible samples, thinned glyphs and counters.
#[derive(Clone, Debug, PartialEq)]
pub struct BoundaryAxialGlyphSet3 {
    options: BoundaryAxialGlyphOptions3,
    tags: Vec<i64>,
    boundary_by_tag: BTreeMap<i64, usize>,
    selected_by_tag: BTreeMap<i64, usize>,
    zero_area_count: usize,
    invalid_direction_count: usize,
    samples: Vec<BoundaryAxialSample3>,
    glyphs: Vec<BoundaryAxialGlyph3>,
}

impl BoundaryAxialGlyphSet3 {
    /// Returns the parameters used.
    #[must_use]
    pub const fn options(&self) -> BoundaryAxialGlyphOptions3 { self.options }

    /// Returns the requested tag set, sorted and without duplicates.
    #[must_use]
    pub fn tags(&self) -> &[i64] { &self.tags }

    /// Returns the boundary triangle count for every tag present, selected or not.
    #[must_use]
    pub const fn boundary_by_tag(&self) -> &BTreeMap<i64, usize> { &self.boundary_by_tag }

    /// Returns the boundary triangle count for each requested tag present,
    /// before the degenerate exclusions.
    #[must_use]
    pub const fn selected_by_tag(&self) -> &BTreeMap<i64, usize> { &self.selected_by_tag }

    /// Returns the eligible triangle count for each tag with at least one.
    #[must_use]
    pub fn eligible_by_tag(&self) -> BTreeMap<i64, usize> {
        count_by_tag(self.samples.iter().map(BoundaryAxialSample3::tag))
    }

    /// Returns the thinned glyph count for each tag with at least one winner.
    #[must_use]
    pub fn glyphs_by_tag(&self) -> BTreeMap<i64, usize> {
        count_by_tag(self.glyphs.iter().map(|glyph| glyph.sample.tag))
    }

    /// Returns the selected triangles excluded for zero or non-finite area
    /// (design §4.6). They receive no normal, angle, bin or glyph.
    #[must_use]
    pub const fn zero_area_count(&self) -> usize { self.zero_area_count }

    /// Returns the selected, non-degenerate triangles excluded because the
    /// source direction is zero or non-finite (design §4.6).
    #[must_use]
    pub const fn invalid_direction_count(&self) -> usize { self.invalid_direction_count }

    /// Returns every eligible triangle in ascending surface-index order.
    #[must_use]
    pub fn samples(&self) -> &[BoundaryAxialSample3] { &self.samples }

    /// Returns one glyph per occupied bin, in ascending surface-index order.
    #[must_use]
    pub fn glyphs(&self) -> &[BoundaryAxialGlyph3] { &self.glyphs }
}

/// Errors produced while validating inputs or constructing glyphs.
#[derive(Clone, Debug, PartialEq)]
pub enum BoundaryAxialGlyphError3 {
    DatasetGeometryMustBeUnstructured,
    InvalidBinEdge { bin_edge: f64 },
    InvalidLength { length: f64 },
    EmptyTagSet,
    TagNotExactlyRepresentable { tag: i64 },
    MissingField { name: String },
    FieldMustBeCellScalar { name: String },
    FieldMustBeCellVector { name: String },
    InvalidFieldCardinality { name: String, message: String },
    SourceCellOutOfBounds { triangle: usize, cell: usize, cell_count: usize },
    /// A boundary triangle's source cell tag is not a finite integer.
    NonIntegerTag { triangle: usize, cell: usize, value: SampleValue },
    /// A centroid divided by the bin edge has no representable integer bin index.
    BinIndexOutOfRange { triangle: usize },
}

impl fmt::Display for BoundaryAxialGlyphError3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DatasetGeometryMustBeUnstructured => write!(f, "boundary axial glyphs require UnstructuredGrid3 source geometry"),
            Self::InvalidBinEdge { bin_edge } => write!(f, "bin edge must be finite and positive; received {bin_edge}"),
            Self::InvalidLength { length } => write!(f, "glyph length must be finite and positive; received {length}"),
            Self::EmptyTagSet => write!(f, "the tag set must contain at least one tag"),
            Self::TagNotExactlyRepresentable { tag } => write!(f, "tag {tag} is not exactly representable as an f64 field value"),
            Self::MissingField { name } => write!(f, "field '{name}' does not exist"),
            Self::FieldMustBeCellScalar { name } => write!(f, "tag field '{name}' must be a cell-associated scalar field"),
            Self::FieldMustBeCellVector { name } => write!(f, "direction field '{name}' must be a cell-associated Vector3 field"),
            Self::InvalidFieldCardinality { name, message } => write!(f, "field '{name}' has invalid cardinality: {message}"),
            Self::SourceCellOutOfBounds { triangle, cell, cell_count } => write!(f, "triangle {triangle} references source cell {cell}, but dataset has {cell_count} cells"),
            Self::NonIntegerTag { triangle, cell, value } => write!(f, "triangle {triangle} has source cell {cell} with non-integer tag {value:?}"),
            Self::BinIndexOutOfRange { triangle } => write!(f, "triangle {triangle} has a centroid whose bin index is not representable"),
        }
    }
}
impl Error for BoundaryAxialGlyphError3 {}

/// Builds thinned axial glyphs for the boundary triangles whose source cell's
/// `tag_field` value is in `tags`.
///
/// `surface` must be the triangulated boundary of `dataset`'s unstructured
/// grid. `direction_field` is a cell `Vector3` field whose values are used
/// exactly, up to sign. Cells are selected by tag only, never by direction
/// value. Every boundary triangle's tag must be a finite integer.
pub fn build_boundary_axial_glyphs(
    dataset: &Dataset,
    surface: &TriangulatedSurface3,
    direction_field: &str,
    tag_field: &str,
    tags: &[i64],
    options: BoundaryAxialGlyphOptions3,
) -> Result<BoundaryAxialGlyphSet3, BoundaryAxialGlyphError3> {
    let DatasetGeometry::UnstructuredGrid3(grid) = dataset.geometry() else {
        return Err(BoundaryAxialGlyphError3::DatasetGeometryMustBeUnstructured);
    };
    let BoundaryAxialGlyphOptions3 { bin_edge, length } = options;
    if !(bin_edge.is_finite() && bin_edge > 0.0) {
        return Err(BoundaryAxialGlyphError3::InvalidBinEdge { bin_edge });
    }
    if !(length.is_finite() && length > 0.0) {
        return Err(BoundaryAxialGlyphError3::InvalidLength { length });
    }
    let mut tags = tags.to_vec();
    tags.sort_unstable();
    tags.dedup();
    if tags.is_empty() {
        return Err(BoundaryAxialGlyphError3::EmptyTagSet);
    }
    if let Some(&tag) = tags.iter().find(|tag| tag.unsigned_abs() > MAX_EXACT_INTEGER as u64) {
        return Err(BoundaryAxialGlyphError3::TagNotExactlyRepresentable { tag });
    }

    let cell_count = grid.cell_count();
    let point_count = grid.point_count();
    let tag_values = match dataset.field(tag_field) {
        None => return Err(BoundaryAxialGlyphError3::MissingField { name: tag_field.to_owned() }),
        Some(DatasetField::Scalar(field)) if field.association() == FieldAssociation::Cell => {
            field.validate_cardinality(point_count, cell_count).map_err(|error| {
                BoundaryAxialGlyphError3::InvalidFieldCardinality { name: tag_field.to_owned(), message: error.to_string() }
            })?;
            field.values()
        }
        Some(_) => return Err(BoundaryAxialGlyphError3::FieldMustBeCellScalar { name: tag_field.to_owned() }),
    };
    let directions = match dataset.field(direction_field) {
        None => return Err(BoundaryAxialGlyphError3::MissingField { name: direction_field.to_owned() }),
        Some(DatasetField::Vector3(field)) if field.association() == FieldAssociation::Cell => {
            field.validate_cardinality(point_count, cell_count).map_err(|error| {
                BoundaryAxialGlyphError3::InvalidFieldCardinality { name: direction_field.to_owned(), message: error.to_string() }
            })?;
            field.values()
        }
        Some(_) => return Err(BoundaryAxialGlyphError3::FieldMustBeCellVector { name: direction_field.to_owned() }),
    };

    let points = surface.points();
    let half_length = length / 2.0;
    let mut boundary_by_tag = BTreeMap::new();
    let mut selected_by_tag = BTreeMap::new();
    let mut zero_area_count = 0;
    let mut invalid_direction_count = 0;
    let mut samples = Vec::new();
    // Triangles are visited in ascending surface index, so the first eligible
    // triangle seen in a bin is that bin's winner.
    let mut winners: BTreeMap<[i64; 3], usize> = BTreeMap::new();

    for (triangle_index, triangle) in surface.triangles().iter().enumerate() {
        let cell = triangle.source_cell_index();
        if cell >= cell_count {
            return Err(BoundaryAxialGlyphError3::SourceCellOutOfBounds { triangle: triangle_index, cell, cell_count });
        }
        let tag = integer_tag(tag_values[cell])
            .ok_or(BoundaryAxialGlyphError3::NonIntegerTag { triangle: triangle_index, cell, value: tag_values[cell] })?;
        *boundary_by_tag.entry(tag).or_insert(0) += 1;
        if tags.binary_search(&tag).is_err() {
            continue;
        }
        *selected_by_tag.entry(tag).or_insert(0) += 1;

        let [a, b, c] = triangle.vertices().map(|vertex| points[vertex as usize].to_array());
        let Some(n_hat) = unit_normal(sub(b, a), sub(c, a)) else {
            zero_area_count += 1;
            continue;
        };
        let Some(direction) = directions[cell].as_finite_world_vector() else {
            invalid_direction_count += 1;
            continue;
        };
        let Some(f_hat) = unit_vector(direction.to_vector().to_array()) else {
            invalid_direction_count += 1;
            continue;
        };
        let abs_cos = dot(f_hat, n_hat).abs();
        let centroid = [0, 1, 2].map(|axis| ((a[axis] + b[axis]) + c[axis]) / 3.0);
        let bin = bin_index(centroid, bin_edge)
            .ok_or(BoundaryAxialGlyphError3::BinIndexOutOfRange { triangle: triangle_index })?;

        winners.entry(bin).or_insert(samples.len());
        samples.push(BoundaryAxialSample3 {
            triangle_index,
            source_cell_index: cell,
            tag,
            direction,
            unit_direction: WorldVector3::new(f_hat[0], f_hat[1], f_hat[2]),
            centroid: WorldPoint3::new(centroid[0], centroid[1], centroid[2]),
            unit_normal: WorldVector3::new(n_hat[0], n_hat[1], n_hat[2]),
            abs_cos,
            bin,
        });
    }

    let mut winner_indices: Vec<usize> = winners.into_values().collect();
    winner_indices.sort_unstable();
    let glyphs = winner_indices
        .into_iter()
        .map(|index| {
            let sample = samples[index];
            let p = sample.centroid.to_array();
            let offset = scale(sample.unit_direction.to_vector().to_array(), half_length);
            let start = sub(p, offset);
            let end = add(p, offset);
            // Always finite: the centroid sum ((a + b) + c) is finite, so
            // |p_c| ≤ f64::MAX / 3, and |(ℓ/2) f̂_i| ≤ f64::MAX / 2 (up to rounding).
            debug_assert!(start.iter().chain(end.iter()).all(|value| value.is_finite()));
            BoundaryAxialGlyph3 {
                sample,
                start: WorldPoint3::new(start[0], start[1], start[2]),
                end: WorldPoint3::new(end[0], end[1], end[2]),
            }
        })
        .collect();

    Ok(BoundaryAxialGlyphSet3 {
        options,
        tags,
        boundary_by_tag,
        selected_by_tag,
        zero_area_count,
        invalid_direction_count,
        samples,
        glyphs,
    })
}

fn integer_tag(value: SampleValue) -> Option<i64> {
    let value = value.as_finite()?;
    // The range check also guarantees that the cast below is exact.
    (value == value.floor() && value.abs() <= MAX_EXACT_INTEGER).then_some(value as i64)
}

fn bin_index(centroid: [f64; 3], bin_edge: f64) -> Option<[i64; 3]> {
    let mut bin = [0_i64; 3];
    for (slot, coordinate) in bin.iter_mut().zip(centroid) {
        // The grid origin is the world origin, so (x − 0) / h is x / h.
        let index = (coordinate / bin_edge).floor();
        if !(index.is_finite() && index.abs() <= MAX_EXACT_INTEGER) {
            return None;
        }
        *slot = index as i64;
    }
    Some(bin)
}

fn count_by_tag(tags: impl Iterator<Item = i64>) -> BTreeMap<i64, usize> {
    let mut counts = BTreeMap::new();
    for tag in tags {
        *counts.entry(tag).or_insert(0) += 1;
    }
    counts
}

// Explicit component arithmetic: the operation order is part of the contract
// shared with the independent audit, so it is not left to a vector library.
fn sub(u: [f64; 3], v: [f64; 3]) -> [f64; 3] { [u[0] - v[0], u[1] - v[1], u[2] - v[2]] }

fn add(u: [f64; 3], v: [f64; 3]) -> [f64; 3] { [u[0] + v[0], u[1] + v[1], u[2] + v[2]] }

fn cross(u: [f64; 3], v: [f64; 3]) -> [f64; 3] {
    [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]]
}

fn dot(u: [f64; 3], v: [f64; 3]) -> f64 { u[0] * v[0] + u[1] * v[1] + u[2] * v[2] }

/// Returns u / ‖u‖, or None when u is zero or has a non-finite component.
///
/// Where the squared length is a finite normal f64, this is exactly the
/// audit's u · (1 / √(u·u)), so results are bitwise identical in that range.
/// Otherwise squaring would overflow or underflow, and u is first divided by
/// its largest absolute component, which changes no direction.
fn unit_vector(u: [f64; 3]) -> Option<[f64; 3]> {
    if !u.iter().all(|value| value.is_finite()) {
        return None;
    }
    let squared = dot(u, u);
    if squared.is_finite() && squared >= f64::MIN_POSITIVE {
        return Some(scale(u, 1.0 / squared.sqrt()));
    }
    let largest = u.iter().fold(0.0_f64, |m, value| m.max(value.abs()));
    if largest == 0.0 {
        return None;
    }
    let reduced = u.map(|value| value / largest);
    Some(scale(reduced, 1.0 / dot(reduced, reduced).sqrt()))
}

/// Returns the unit geometric normal of edges e1 = b − a and e2 = c − a, or
/// None for zero or non-finite area.
///
/// The cross product is used as is where it is safe, matching the audit. If
/// it overflows or underflows, each edge is first divided by its largest
/// absolute component; positive scale factors do not change the normal.
fn unit_normal(e1: [f64; 3], e2: [f64; 3]) -> Option<[f64; 3]> {
    if !e1.iter().chain(&e2).all(|value| value.is_finite()) {
        return None;
    }
    let normal = cross(e1, e2);
    let squared = dot(normal, normal);
    if squared.is_finite() && squared >= f64::MIN_POSITIVE {
        return Some(scale(normal, 1.0 / squared.sqrt()));
    }
    let reduce = |e: [f64; 3]| {
        let largest = e.iter().fold(0.0_f64, |m, value| m.max(value.abs()));
        (largest > 0.0).then(|| e.map(|value| value / largest))
    };
    unit_vector(cross(reduce(e1)?, reduce(e2)?))
}

fn scale(u: [f64; 3], s: f64) -> [f64; 3] { [u[0] * s, u[1] * s, u[2] * s] }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        fields::{FieldMetadata, ScalarField, VectorBasis3, VectorField3, VectorValue3},
        geometry::PointSet3,
        unstructured::{CellType3, UnstructuredGrid3},
    };

    // Two tetrahedra sharing face (1, 2, 3): three boundary triangles each.
    const POINTS: [[f64; 3]; 5] = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 1.0, 1.0]];
    const CELLS: [[u32; 4]; 2] = [[0, 1, 2, 3], [1, 2, 3, 4]];

    fn dataset(points: &[[f64; 3]], cells: &[[u32; 4]], tags: &[f64], fibres: &[[f64; 3]]) -> Dataset {
        let grid = UnstructuredGrid3::new(
            points.iter().map(|p| WorldPoint3::new(p[0], p[1], p[2])).collect(),
            vec![CellType3::Tetrahedron4; cells.len()],
            (0..=cells.len()).map(|i| 4 * i).collect(),
            cells.iter().flatten().copied().collect(),
        ).unwrap();
        let mut dataset = Dataset::new(grid);
        dataset.add_scalar_field(ScalarField::new(
            FieldMetadata::new("ID", None).unwrap(),
            FieldAssociation::Cell,
            tags.iter().map(|&t| SampleValue::from_f64(t)).collect(),
        ).unwrap()).unwrap();
        dataset.add_vector_field3(VectorField3::new(
            FieldMetadata::new("fibres", None).unwrap(),
            FieldAssociation::Cell,
            VectorBasis3::World,
            fibres.iter().map(|f| VectorValue3::from_f64(f[0], f[1], f[2])).collect(),
        ).unwrap()).unwrap();
        dataset
    }

    fn surface(dataset: &Dataset) -> TriangulatedSurface3 {
        let DatasetGeometry::UnstructuredGrid3(grid) = dataset.geometry() else { unreachable!() };
        grid.boundary_surface().unwrap().triangulate()
    }

    fn build(dataset: &Dataset, tags: &[i64], bin_edge: f64, length: f64) -> Result<BoundaryAxialGlyphSet3, BoundaryAxialGlyphError3> {
        build_boundary_axial_glyphs(dataset, &surface(dataset), "fibres", "ID", tags, BoundaryAxialGlyphOptions3 { bin_edge, length })
    }

    fn sorted_ids(surface: &TriangulatedSurface3, triangle: usize) -> [u32; 3] {
        let mut ids = surface.triangles()[triangle].vertices().map(|v| surface.source_point_ids()[v as usize]);
        ids.sort_unstable();
        ids
    }

    fn norm(u: [f64; 3]) -> f64 { dot(u, u).sqrt() }

    fn close(u: [f64; 3], v: [f64; 3], tolerance: f64) -> bool {
        u.iter().zip(v).all(|(a, b)| (a - b).abs() <= tolerance)
    }

    #[test]
    fn default_options_are_the_provisional_design_values() {
        assert_eq!(BoundaryAxialGlyphOptions3::default(), BoundaryAxialGlyphOptions3 { bin_edge: 4.0, length: 3.0 });
    }

    #[test]
    fn selection_is_by_tag_not_by_value() {
        // Cell 1 is outside the tag set; cell 0 holds the placeholder-like (1, 0, 0).
        let data = dataset(&POINTS, &CELLS, &[1.0, 5.0], &[[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]);
        let result = build(&data, &[1], 0.25, 0.5).unwrap();
        assert_eq!(result.tags(), &[1]);
        assert_eq!(result.boundary_by_tag(), &BTreeMap::from([(1, 3), (5, 3)]));
        assert_eq!(result.selected_by_tag(), &BTreeMap::from([(1, 3)]));
        assert_eq!(result.eligible_by_tag(), BTreeMap::from([(1, 3)]));
        assert!(!result.glyphs().is_empty());
        assert!(result.samples().iter().all(|s| s.source_cell_index() == 0 && s.tag() == 1));
        assert!(result.glyphs().iter().all(|g| g.sample().source_cell_index() == 0));
        // The same tag requested twice, plus an absent tag, changes nothing.
        let repeated = build(&data, &[1, 9, 1], 0.25, 0.5).unwrap();
        assert_eq!(repeated.tags(), &[1, 9]);
        assert_eq!(repeated.samples(), result.samples());
        assert_eq!(repeated.glyphs(), result.glyphs());
        // Nothing is selected when no requested tag is present.
        let none = build(&data, &[2], 0.25, 0.5).unwrap();
        assert!(none.samples().is_empty() && none.glyphs().is_empty() && none.selected_by_tag().is_empty());
    }

    #[test]
    fn geometric_normals_and_tangency_angles() {
        let data = dataset(&POINTS[..4], &CELLS[..1], &[1.0], &[[1.0, 0.0, 0.0]]);
        let surface = surface(&data);
        let result = build(&data, &[1], 4.0, 1.0).unwrap();
        assert_eq!(result.samples().len(), 4);
        let s3 = 1.0 / 3.0_f64.sqrt();
        for sample in result.samples() {
            let n = sample.unit_normal().to_vector().to_array();
            assert!((norm(n) - 1.0).abs() <= 1e-15);
            let (axis, abs_cos, theta) = match sorted_ids(&surface, sample.triangle_index()) {
                [0, 2, 3] => ([1.0, 0.0, 0.0], 1.0, 0.0),
                [0, 1, 3] => ([0.0, 1.0, 0.0], 0.0, 90.0),
                [0, 1, 2] => ([0.0, 0.0, 1.0], 0.0, 90.0),
                [1, 2, 3] => ([s3, s3, s3], s3, s3.acos().to_degrees()),
                other => panic!("unexpected face {other:?}"),
            };
            // The sign of n follows the stored cycle and is not certified.
            assert!(close(n, axis, 1e-15) || close(n, scale(axis, -1.0), 1e-15), "{n:?}");
            assert!((sample.abs_cos() - abs_cos).abs() <= 1e-15);
            assert!((sample.tangency_angle_degrees() - theta).abs() <= 1e-12);
        }
    }

    #[test]
    fn axial_quantities_ignore_direction_sign_and_winding() {
        let plus = dataset(&POINTS[..4], &CELLS[..1], &[1.0], &[[0.3, -0.5, 0.7]]);
        let minus = dataset(&POINTS[..4], &CELLS[..1], &[1.0], &[[-0.3, 0.5, -0.7]]);
        let plus = build(&plus, &[1], 0.25, 1.5).unwrap();
        let minus = build(&minus, &[1], 0.25, 1.5).unwrap();
        assert_eq!(plus.samples().len(), minus.samples().len());
        for (p, m) in plus.samples().iter().zip(minus.samples()) {
            assert_eq!(p.abs_cos().to_bits(), m.abs_cos().to_bits());
            assert_eq!((p.centroid(), p.bin(), p.unit_normal()), (m.centroid(), m.bin(), m.unit_normal()));
        }
        for (p, m) in plus.glyphs().iter().zip(minus.glyphs()) {
            // Negating f̂ exactly swaps the endpoints of the centred segment.
            assert_eq!((p.start(), p.end()), (m.end(), m.start()));
        }

        // Reversed winding: the same tetrahedron with two vertices swapped.
        let forward = dataset(&POINTS[..4], &[[0, 1, 2, 3]], &[1.0], &[[0.3, -0.5, 0.7]]);
        let reversed = dataset(&POINTS[..4], &[[0, 2, 1, 3]], &[1.0], &[[0.3, -0.5, 0.7]]);
        let (fs, rs) = (surface(&forward), surface(&reversed));
        let forward = build(&forward, &[1], 0.25, 1.5).unwrap();
        let reversed = build(&reversed, &[1], 0.25, 1.5).unwrap();
        assert_eq!(forward.samples().len(), 4);
        for f in forward.samples() {
            let key = sorted_ids(&fs, f.triangle_index());
            let r = reversed.samples().iter().find(|r| sorted_ids(&rs, r.triangle_index()) == key).unwrap();
            assert!((f.abs_cos() - r.abs_cos()).abs() <= 1e-15);
            assert!((f.tangency_angle_degrees() - r.tangency_angle_degrees()).abs() <= 1e-12);
            let (fn_, rn) = (f.unit_normal().to_vector().to_array(), r.unit_normal().to_vector().to_array());
            assert!(close(fn_, scale(rn, -1.0), 1e-15), "winding flips the geometric normal");
        }
    }

    #[test]
    fn segment_is_centred_has_length_l_and_follows_the_exact_direction() {
        let data = dataset(&POINTS[..4], &CELLS[..1], &[1.0], &[[0.0, 3.0, 4.0]]);
        let result = build(&data, &[1], 0.25, 2.0).unwrap();
        assert!(!result.glyphs().is_empty());
        for glyph in result.glyphs() {
            let sample = glyph.sample();
            assert_eq!(sample.direction(), WorldVector3::new(0.0, 3.0, 4.0), "unmodified source value");
            let (start, end) = (glyph.start().to_array(), glyph.end().to_array());
            let midpoint = scale(add(start, end), 0.5);
            assert!(close(midpoint, sample.centroid().to_array(), 1e-15));
            let displacement = sub(end, start);
            assert!((norm(displacement) - 2.0).abs() <= 1e-15);
            assert!(close(displacement, [0.0, 1.2, 1.6], 1e-15), "along f̂, not projected");
        }
    }

    #[test]
    fn floor_binning_is_consistent_across_negative_coordinates() {
        let shifted: Vec<[f64; 3]> = POINTS[..4].iter().map(|p| p.map(|x| x - 10.0)).collect();
        let data = dataset(&shifted, &CELLS[..1], &[1.0], &[[1.0, 0.0, 0.0]]);
        let result = build(&data, &[1], 4.0, 1.0).unwrap();
        // Centroids lie in [-10, -9.6]: floor(x / 4) = -3, whereas truncation would give -2.
        assert!(result.samples().iter().all(|s| s.bin() == [-3, -3, -3]));
        assert_eq!(result.glyphs().len(), 1);
        assert_eq!(bin_index([-0.5, 0.0, -0.0], 4.0), Some([-1, 0, 0]));
        assert_eq!(bin_index([-4.0, 3.999, 4.0], 4.0), Some([-1, 0, 1]));
    }

    #[test]
    fn lowest_surface_index_wins_each_bin_deterministically() {
        let data = dataset(&POINTS, &CELLS, &[1.0, 2.0], &[[1.0, 0.0, 0.0], [0.0, 0.0, 1.0]]);
        let one_bin = build(&data, &[1, 2], 100.0, 1.0).unwrap();
        assert_eq!(one_bin.samples().len(), 6);
        assert_eq!(one_bin.glyphs().len(), 1);
        assert_eq!(one_bin.glyphs()[0].sample().triangle_index(), 0);
        assert_eq!(one_bin.glyphs_by_tag().values().sum::<usize>(), 1);

        let mut multi_bin_cases = 0;
        for bin_edge in [0.2, 0.3, 0.5] {
            let result = build(&data, &[1, 2], bin_edge, 0.1).unwrap();
            assert_eq!(result, build(&data, &[1, 2], bin_edge, 0.1).unwrap(), "deterministic");
            let mut expected: BTreeMap<[i64; 3], usize> = BTreeMap::new();
            for sample in result.samples() {
                let entry = expected.entry(sample.bin()).or_insert(usize::MAX);
                *entry = (*entry).min(sample.triangle_index());
            }
            let mut expected: Vec<usize> = expected.into_values().collect();
            expected.sort_unstable();
            let actual: Vec<usize> = result.glyphs().iter().map(|g| g.sample().triangle_index()).collect();
            assert_eq!(actual, expected, "bin edge {bin_edge}");
            assert_eq!(result.glyphs_by_tag().values().sum::<usize>(), result.glyphs().len());
            if result.glyphs().len() > 1 && result.glyphs().len() < result.samples().len() {
                multi_bin_cases += 1;
            }
        }
        assert!(multi_bin_cases > 0, "at least one case must share a bin and occupy several");
    }

    #[test]
    fn provenance_resolves_to_the_triangle_cell_tag_and_direction() {
        let fibres = [[1.0, 0.0, 0.0], [0.0, 0.6, 0.8]];
        let data = dataset(&POINTS, &CELLS, &[1.0, 2.0], &fibres);
        let surface = surface(&data);
        let result = build(&data, &[1, 2], 0.25, 1.0).unwrap();
        assert_eq!(result.samples().len(), surface.triangle_count());
        for sample in result.samples() {
            let triangle = surface.triangles()[sample.triangle_index()];
            let cell = triangle.source_cell_index();
            assert_eq!(sample.source_cell_index(), cell);
            assert_eq!(sample.tag(), [1, 2][cell]);
            assert_eq!(sample.direction().to_vector().to_array(), fibres[cell]);
            let ids = sorted_ids(&surface, sample.triangle_index());
            assert!(ids.iter().all(|id| CELLS[cell].contains(id)));
        }
        let indices: Vec<usize> = result.samples().iter().map(BoundaryAxialSample3::triangle_index).collect();
        assert!(indices.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn pinned_centroid_arithmetic_and_stored_vertex_order_decide_the_bin() {
        // Same case as tools/rodero-fibre-audit/test_audit.py. Local face 1 of
        // the Tetrahedron4 template is stored as [0, 3, 2]. Its x values sum
        // to exactly 396 in that order, but not reversed or as a/3 + b/3 + c/3.
        let (xa, xb, xc) = (132.211618, 131.696712, 132.09167);
        assert_eq!(((xa + xb) + xc) / 3.0, 132.0);
        assert!(((xc + xb) + xa) / 3.0 < 132.0);
        assert!(xa / 3.0 + xb / 3.0 + xc / 3.0 < 132.0);
        let points = [[xa, 1.0, 1.0], [131.0, 2.0, 1.0], [xc, 1.0, 2.0], [xb, 2.0, 2.0]];
        let data = dataset(&points, &CELLS[..1], &[1.0], &[[0.0, 0.0, 1.0]]);
        let surface = surface(&data);
        let result = build(&data, &[1], 4.0, 1.0).unwrap();
        assert_eq!(result.zero_area_count(), 0);
        assert_eq!(result.samples().len(), 4);
        let mut checked_face = false;
        for sample in result.samples() {
            let triangle = surface.triangles()[sample.triangle_index()];
            if triangle.source_local_face_index() == 1 {
                assert_eq!(triangle.vertices().map(|v| surface.source_point_ids()[v as usize]), [0, 3, 2]);
                assert_eq!(sample.centroid().to_array()[0], 132.0);
                assert_eq!(sample.bin()[0], 33);
                checked_face = true;
            } else {
                assert_eq!(sample.bin()[0], 32);
            }
        }
        assert!(checked_face);
        // Any other order or formula would put all four faces in one bin.
        assert_eq!(result.glyphs().len(), 2);
    }

    #[test]
    fn degenerate_cases_are_counted_and_excluded() {
        // Points 0, 1, 2 are collinear: local face 3 (0, 2, 1) has zero area.
        let flat = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
        let result = build(&dataset(&flat, &CELLS[..1], &[1.0], &[[0.0, 1.0, 0.0]]), &[1], 4.0, 1.0).unwrap();
        assert_eq!((result.zero_area_count(), result.invalid_direction_count()), (1, 0));
        assert_eq!(result.selected_by_tag(), &BTreeMap::from([(1, 4)]));
        assert_eq!(result.eligible_by_tag(), BTreeMap::from([(1, 3)]));

        // A zero-area triangle is counted once, as zero area, even with an invalid direction.
        let both = build(&dataset(&flat, &CELLS[..1], &[1.0], &[[0.0, 0.0, 0.0]]), &[1], 4.0, 1.0).unwrap();
        assert_eq!((both.zero_area_count(), both.invalid_direction_count()), (1, 3));

        for fibre in [[0.0, 0.0, 0.0], [-0.0, 0.0, -0.0], [f64::NAN, 0.0, 1.0], [0.0, f64::INFINITY, 0.0]] {
            let data = dataset(&POINTS, &CELLS, &[1.0, 2.0], &[fibre, [0.0, 0.0, 1.0]]);
            let result = build(&data, &[1, 2], 4.0, 1.0).unwrap();
            assert_eq!((result.zero_area_count(), result.invalid_direction_count()), (0, 3), "{fibre:?}");
            assert_eq!(result.eligible_by_tag(), BTreeMap::from([(2, 3)]));
            assert!(result.glyphs().iter().all(|g| g.sample().tag() == 2));
            // Outside the tag set, the invalid direction is neither used nor counted.
            let outside = build(&data, &[2], 4.0, 1.0).unwrap();
            assert_eq!(outside.invalid_direction_count(), 0);
        }
    }

    #[test]
    fn invalid_inputs_fail_explicitly() {
        type E = BoundaryAxialGlyphError3;
        let data = dataset(&POINTS, &CELLS, &[1.0, 2.0], &[[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]);
        let surf = surface(&data);
        let run = |dataset: &Dataset, direction: &str, tag: &str, tags: &[i64], bin_edge: f64, length: f64| {
            build_boundary_axial_glyphs(dataset, &surf, direction, tag, tags, BoundaryAxialGlyphOptions3 { bin_edge, length })
        };
        for bin_edge in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(matches!(run(&data, "fibres", "ID", &[1], bin_edge, 1.0), Err(E::InvalidBinEdge { .. })));
        }
        for length in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(matches!(run(&data, "fibres", "ID", &[1], 4.0, length), Err(E::InvalidLength { .. })));
        }
        assert_eq!(run(&data, "fibres", "ID", &[], 4.0, 1.0), Err(E::EmptyTagSet));
        let huge = (1_i64 << 53) + 1;
        assert_eq!(run(&data, "fibres", "ID", &[1, huge], 4.0, 1.0), Err(E::TagNotExactlyRepresentable { tag: huge }));
        assert_eq!(run(&data, "missing", "ID", &[1], 4.0, 1.0), Err(E::MissingField { name: "missing".into() }));
        assert_eq!(run(&data, "fibres", "missing", &[1], 4.0, 1.0), Err(E::MissingField { name: "missing".into() }));
        assert_eq!(run(&data, "fibres", "fibres", &[1], 4.0, 1.0), Err(E::FieldMustBeCellScalar { name: "fibres".into() }));
        assert_eq!(run(&data, "ID", "ID", &[1], 4.0, 1.0), Err(E::FieldMustBeCellVector { name: "ID".into() }));

        let mut point_tags = data.clone();
        point_tags.add_scalar_field(ScalarField::new(
            FieldMetadata::new("point_id", None).unwrap(),
            FieldAssociation::Point,
            vec![SampleValue::Finite(1.0); POINTS.len()],
        ).unwrap()).unwrap();
        assert_eq!(run(&point_tags, "fibres", "point_id", &[1], 4.0, 1.0), Err(E::FieldMustBeCellScalar { name: "point_id".into() }));

        for bad in [1.5, f64::NAN, 1e300] {
            let data = dataset(&POINTS, &CELLS, &[1.0, bad], &[[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]);
            assert!(matches!(run(&data, "fibres", "ID", &[1], 4.0, 1.0), Err(E::NonIntegerTag { cell: 1, .. })), "{bad}");
        }

        let points = Dataset::new(PointSet3::new(vec![WorldPoint3::ORIGIN]).unwrap());
        assert_eq!(run(&points, "fibres", "ID", &[1], 4.0, 1.0), Err(E::DatasetGeometryMustBeUnstructured));

        // A surface that does not belong to the dataset.
        let small = dataset(&POINTS[..4], &CELLS[..1], &[1.0], &[[1.0, 0.0, 0.0]]);
        assert!(matches!(run(&small, "fibres", "ID", &[1], 4.0, 1.0), Err(E::SourceCellOutOfBounds { cell: 1, cell_count: 1, .. })));

        // A bin index beyond the exactly representable integer range.
        assert!(matches!(run(&data, "fibres", "ID", &[1], f64::MIN_POSITIVE, 1.0), Err(E::BinIndexOutOfRange { .. })));
    }

    #[test]
    fn unit_vectors_match_the_audit_formula_bitwise_in_the_safe_range() {
        // The audit computes u · (1 / √(u·u)); stage 4 compares against it exactly.
        for u in [[0.3, -0.5, 0.7], [1.0, 0.0, 0.0], [1e150, -2e150, 3e150], [1e-150, 2e-150, 0.0], [-7.25, 1e-200, 3.0]] {
            let audit = scale(u, 1.0 / norm(u));
            assert_eq!(unit_vector(u).unwrap().map(f64::to_bits), audit.map(f64::to_bits), "{u:?}");
        }
        let (e1, e2) = ([1.0, 0.25, -3.0], [0.5, 2.0, 0.125]);
        let n = cross(e1, e2);
        assert_eq!(unit_normal(e1, e2).unwrap().map(f64::to_bits), scale(n, 1.0 / norm(n)).map(f64::to_bits));
    }

    #[test]
    fn large_and_small_finite_directions_are_valid() {
        // Squaring these overflows or underflows f64; the directions themselves are fine.
        let reference = build(&dataset(&POINTS[..4], &CELLS[..1], &[1.0], &[[1.0, 2.0, 0.0]]), &[1], 0.25, 1.0).unwrap();
        for fibre in [[1e300, 2e300, 0.0], [1e-300, 2e-300, 0.0], [f64::MAX / 2.0, f64::MAX, 0.0], [5e-324, 1e-323, 0.0]] {
            let result = build(&dataset(&POINTS[..4], &CELLS[..1], &[1.0], &[fibre]), &[1], 0.25, 1.0).unwrap();
            assert_eq!(result.invalid_direction_count(), 0, "{fibre:?}");
            assert_eq!(result.samples().len(), 4);
            assert_eq!(result.glyphs().len(), reference.glyphs().len());
            for (sample, expected) in result.samples().iter().zip(reference.samples()) {
                assert_eq!(sample.direction().to_vector().to_array(), fibre, "unmodified source value");
                let f_hat = sample.unit_direction().to_vector().to_array();
                assert!((norm(f_hat) - 1.0).abs() <= 1e-15);
                assert!(close(f_hat, expected.unit_direction().to_vector().to_array(), 1e-15), "{fibre:?}");
                assert!((sample.abs_cos() - expected.abs_cos()).abs() <= 1e-15);
            }
            for (glyph, expected) in result.glyphs().iter().zip(reference.glyphs()) {
                assert!(close(glyph.start().to_array(), expected.start().to_array(), 1e-15));
                assert!(close(glyph.end().to_array(), expected.end().to_array(), 1e-15));
            }
        }
    }

    #[test]
    fn large_and_small_finite_triangles_have_correct_normals() {
        // Edge products overflow (1e200²) or underflow (1e-200²) in the cross product.
        for factor in [1e200, 1e-200, 1e-160] {
            let points: Vec<[f64; 3]> = POINTS[..4].iter().map(|p| p.map(|x| x * factor)).collect();
            let data = dataset(&points, &CELLS[..1], &[1.0], &[[1.0, 0.0, 0.0]]);
            let surface = surface(&data);
            let result = build(&data, &[1], 4.0 * factor, 1.0).unwrap();
            assert_eq!(result.zero_area_count(), 0, "factor {factor}");
            assert_eq!(result.samples().len(), 4);
            let s3 = 1.0 / 3.0_f64.sqrt();
            for sample in result.samples() {
                let n = sample.unit_normal().to_vector().to_array();
                let axis = match sorted_ids(&surface, sample.triangle_index()) {
                    [0, 2, 3] => [1.0, 0.0, 0.0],
                    [0, 1, 3] => [0.0, 1.0, 0.0],
                    [0, 1, 2] => [0.0, 0.0, 1.0],
                    _ => [s3, s3, s3],
                };
                assert!(close(n, axis, 1e-15) || close(n, scale(axis, -1.0), 1e-15), "factor {factor}: {n:?}");
            }
        }
        // Truly collinear points remain zero area at any scale.
        let flat: Vec<[f64; 3]> = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 0.0, 1.0]]
            .iter().map(|p| p.map(|x| x * 1e-200)).collect();
        let result = build(&dataset(&flat, &CELLS[..1], &[1.0], &[[0.0, 1.0, 0.0]]), &[1], 1.0, 1.0).unwrap();
        assert_eq!(result.zero_area_count(), 1);
        // An edge that overflows f64 is a non-finite triangle.
        assert_eq!(unit_normal([f64::INFINITY, 0.0, 0.0], [0.0, 1.0, 0.0]), None);
    }

    #[test]
    fn extreme_finite_inputs_give_finite_endpoints() {
        // A centroid near the finite-sum limit, with the longest length.
        let x = f64::MAX / 4.0;
        let far = [[x, 0.0, 0.0], [x, 1.0, 0.0], [x, 0.0, 1.0], [x * 0.5, 0.0, 0.0]];
        let data = dataset(&far, &CELLS[..1], &[1.0], &[[1.0, 0.0, 0.0]]);
        let options = BoundaryAxialGlyphOptions3 { bin_edge: f64::MAX, length: f64::MAX };
        let result = build_boundary_axial_glyphs(&data, &surface(&data), "fibres", "ID", &[1], options).unwrap();
        assert!(!result.glyphs().is_empty());
        for glyph in result.glyphs() {
            assert!(glyph.start().to_array().iter().chain(&glyph.end().to_array()).all(|v| v.is_finite()));
        }
    }
}
