//! Validated unstructured cell connectivity in three-dimensional world space.
//!
//! This module stores source cells, not render primitives. Cell and point
//! order are preserved exactly as supplied by the caller.

use std::{error::Error, fmt};

use crate::spatial::WorldPoint3;

/// A supported linear cell embedded in three-dimensional world space.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CellType3 {
    /// Two endpoints; topological dimension 1.
    Line2,
    /// Three vertices in cyclic perimeter order; dimension 2.
    Triangle3,
    /// Four vertices in cyclic perimeter order; dimension 2.
    Quad4,
    /// Four vertices in caller-supplied simplex order; dimension 3.
    Tetrahedron4,
    /// Eight vertices: cyclic order around one face, then corresponding
    /// cyclic order around the opposite face; dimension 3.
    Hexahedron8,
}

impl CellType3 {
    /// Returns the fixed number of point indices required by this cell type.
    #[must_use]
    pub const fn node_count(self) -> usize {
        match self {
            Self::Line2 => 2,
            Self::Triangle3 => 3,
            Self::Quad4 | Self::Tetrahedron4 => 4,
            Self::Hexahedron8 => 8,
        }
    }

    /// Returns the cell's topological dimension (1, 2, or 3).
    #[must_use]
    pub const fn topological_dimension(self) -> u8 {
        match self {
            Self::Line2 => 1,
            Self::Triangle3 | Self::Quad4 => 2,
            Self::Tetrahedron4 | Self::Hexahedron8 => 3,
        }
    }
}

/// Errors produced when constructing an unstructured grid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnstructuredGridError {
    /// A point contains one or more non-finite coordinates.
    NonFinitePoint { point_index: usize },
    /// Offsets must contain one entry per cell plus the initial zero.
    CellOffsetCountMismatch { expected: usize, actual: usize },
    /// The first cell offset must be zero.
    CellOffsetsMustStartAtZero { actual: usize },
    /// Cell offsets must be monotonically non-decreasing.
    CellOffsetsNotMonotonic {
        offset_index: usize,
        previous: usize,
        actual: usize,
    },
    /// The final cell offset must equal the connectivity length.
    CellOffsetsEndMismatch { expected: usize, actual: usize },
    /// A cell's connectivity span does not match its cell type.
    CellNodeCountMismatch {
        cell_index: usize,
        cell_type: CellType3,
        expected: usize,
        actual: usize,
    },
    /// A cell references a point outside the point array.
    PointIndexOutOfBounds {
        cell_index: usize,
        local_node_index: usize,
        point_index: u32,
        point_count: usize,
    },
    /// A cell contains the same point index more than once.
    RepeatedPointIndex {
        cell_index: usize,
        point_index: u32,
    },
}

impl fmt::Display for UnstructuredGridError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinitePoint { point_index } => {
                write!(f, "point {point_index} contains a non-finite coordinate")
            }
            Self::CellOffsetCountMismatch { expected, actual } => write!(
                f,
                "cell offset count must be {expected}; received {actual}"
            ),
            Self::CellOffsetsMustStartAtZero { actual } => {
                write!(f, "the first cell offset must be zero; received {actual}")
            }
            Self::CellOffsetsNotMonotonic {
                offset_index,
                previous,
                actual,
            } => write!(
                f,
                "cell offset {offset_index} decreased from {previous} to {actual}"
            ),
            Self::CellOffsetsEndMismatch { expected, actual } => write!(
                f,
                "the final cell offset must equal connectivity length {expected}; received {actual}"
            ),
            Self::CellNodeCountMismatch {
                cell_index,
                cell_type,
                expected,
                actual,
            } => write!(
                f,
                "cell {cell_index} of type {cell_type:?} requires {expected} point indices; received {actual}"
            ),
            Self::PointIndexOutOfBounds {
                cell_index,
                local_node_index,
                point_index,
                point_count,
            } => write!(
                f,
                "cell {cell_index} local node {local_node_index} references point {point_index}, but only {point_count} points exist"
            ),
            Self::RepeatedPointIndex {
                cell_index,
                point_index,
            } => write!(
                f,
                "cell {cell_index} references point {point_index} more than once"
            ),
        }
    }
}

impl Error for UnstructuredGridError {}

/// An immutable collection of unstructured source cells and their points.
///
/// Connectivity uses compressed sparse row (CSR) storage. For cell i, its
/// point indices occupy connectivity[cell_offsets[i]..cell_offsets[i + 1]].
/// Cell IDs are their positions in cell_types; point IDs are positions in
/// points. Both orders are preserved exactly.
///
/// Mixed cell types and topological dimensions are allowed. Structural
/// connectivity is validated, while geometric degeneracy is retained.
#[derive(Clone, Debug, PartialEq)]
pub struct UnstructuredGrid3 {
    points: Vec<WorldPoint3>,
    cell_types: Vec<CellType3>,
    cell_offsets: Vec<usize>,
    connectivity: Vec<u32>,
}

impl Default for UnstructuredGrid3 {
    fn default() -> Self {
        Self {
            points: Vec::new(),
            cell_types: Vec::new(),
            cell_offsets: vec![0],
            connectivity: Vec::new(),
        }
    }
}

impl UnstructuredGrid3 {
    /// Creates a grid after validating points, offsets, cell arity, and indices.
    ///
    /// An empty grid uses empty points/types/connectivity and offsets [0].
    /// Points without cells are also valid.
    pub fn new(
        points: Vec<WorldPoint3>,
        cell_types: Vec<CellType3>,
        cell_offsets: Vec<usize>,
        connectivity: Vec<u32>,
    ) -> Result<Self, UnstructuredGridError> {
        for (point_index, point) in points.iter().enumerate() {
            if !point.to_vector().is_finite() {
                return Err(UnstructuredGridError::NonFinitePoint { point_index });
            }
        }

        let expected_offsets = cell_types.len() + 1;
        if cell_offsets.len() != expected_offsets {
            return Err(UnstructuredGridError::CellOffsetCountMismatch {
                expected: expected_offsets,
                actual: cell_offsets.len(),
            });
        }

        if cell_offsets[0] != 0 {
            return Err(UnstructuredGridError::CellOffsetsMustStartAtZero {
                actual: cell_offsets[0],
            });
        }

        for (offset_index, pair) in cell_offsets.windows(2).enumerate() {
            if pair[1] < pair[0] {
                return Err(UnstructuredGridError::CellOffsetsNotMonotonic {
                    offset_index: offset_index + 1,
                    previous: pair[0],
                    actual: pair[1],
                });
            }
        }

        let final_offset = *cell_offsets.last().expect("offset count is non-zero");
        if final_offset != connectivity.len() {
            return Err(UnstructuredGridError::CellOffsetsEndMismatch {
                expected: connectivity.len(),
                actual: final_offset,
            });
        }

        for (cell_index, cell_type) in cell_types.iter().copied().enumerate() {
            let start = cell_offsets[cell_index];
            let end = cell_offsets[cell_index + 1];
            let actual = end - start;
            let expected = cell_type.node_count();
            if actual != expected {
                return Err(UnstructuredGridError::CellNodeCountMismatch {
                    cell_index,
                    cell_type,
                    expected,
                    actual,
                });
            }

            let cell_indices = &connectivity[start..end];
            for (local_node_index, &point_index) in cell_indices.iter().enumerate() {
                if point_index as usize >= points.len() {
                    return Err(UnstructuredGridError::PointIndexOutOfBounds {
                        cell_index,
                        local_node_index,
                        point_index,
                        point_count: points.len(),
                    });
                }
                if cell_indices[..local_node_index].contains(&point_index) {
                    return Err(UnstructuredGridError::RepeatedPointIndex {
                        cell_index,
                        point_index,
                    });
                }
            }
        }

        Ok(Self {
            points,
            cell_types,
            cell_offsets,
            connectivity,
        })
    }

    /// Returns source points in their original order.
    #[must_use]
    pub fn points(&self) -> &[WorldPoint3] {
        &self.points
    }

    /// Returns source cell types in cell-ID order.
    #[must_use]
    pub fn cell_types(&self) -> &[CellType3] {
        &self.cell_types
    }

    /// Returns CSR offsets, including the terminal connectivity offset.
    #[must_use]
    pub fn cell_offsets(&self) -> &[usize] {
        &self.cell_offsets
    }

    /// Returns flattened point indices in their original order.
    #[must_use]
    pub fn connectivity(&self) -> &[u32] {
        &self.connectivity
    }

    /// Returns the point indices for a source cell, or None if out of range.
    #[must_use]
    pub fn cell_point_indices(&self, cell_index: usize) -> Option<&[u32]> {
        let start = *self.cell_offsets.get(cell_index)?;
        let end = *self.cell_offsets.get(cell_index + 1)?;
        Some(&self.connectivity[start..end])
    }

    /// Returns the cell type for a source cell, or None if out of range.
    #[must_use]
    pub fn cell_type(&self, cell_index: usize) -> Option<CellType3> {
        self.cell_types.get(cell_index).copied()
    }

    /// Returns the number of stored points, including unreferenced points.
    #[must_use]
    pub fn point_count(&self) -> usize {
        self.points.len()
    }

    /// Returns the number of source cells.
    #[must_use]
    pub fn cell_count(&self) -> usize {
        self.cell_types.len()
    }

    /// Returns whether the grid contains neither points nor cells.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.points.is_empty() && self.cell_types.is_empty()
    }

    /// Consumes the grid and returns its point and connectivity arrays.
    #[must_use]
    pub fn into_parts(self) -> (Vec<WorldPoint3>, Vec<CellType3>, Vec<usize>, Vec<u32>) {
        (self.points, self.cell_types, self.cell_offsets, self.connectivity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(x: f64, y: f64, z: f64) -> WorldPoint3 {
        WorldPoint3::new(x, y, z)
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

    #[test]
    fn accepts_mixed_cell_types_and_preserves_source_order() {
        let points = cube_points();
        let cell_types = vec![
            CellType3::Line2,
            CellType3::Triangle3,
            CellType3::Quad4,
            CellType3::Tetrahedron4,
            CellType3::Hexahedron8,
        ];
        let offsets = vec![0, 2, 5, 9, 13, 21];
        let connectivity = vec![
            0, 1, // line
            0, 1, 2, // triangle
            0, 1, 2, 3, // quad
            0, 1, 2, 4, // tetrahedron
            0, 1, 2, 3, 4, 5, 6, 7, // hexahedron
        ];
        let grid = UnstructuredGrid3::new(
            points.clone(),
            cell_types.clone(),
            offsets.clone(),
            connectivity.clone(),
        )
        .unwrap();

        assert_eq!(grid.points(), points);
        assert_eq!(grid.cell_types(), cell_types);
        assert_eq!(grid.cell_offsets(), offsets);
        assert_eq!(grid.connectivity(), connectivity);
        assert_eq!(grid.point_count(), 8);
        assert_eq!(grid.cell_count(), 5);
        assert_eq!(grid.cell_point_indices(0), Some(&[0, 1][..]));
        assert_eq!(grid.cell_point_indices(4), Some(&[0, 1, 2, 3, 4, 5, 6, 7][..]));
        assert_eq!(grid.cell_type(3), Some(CellType3::Tetrahedron4));
        assert_eq!(grid.cell_point_indices(5), None);
        assert_eq!(grid.cell_type(5), None);
    }

    #[test]
    fn reports_node_counts_and_dimensions_for_each_cell_type() {
        assert_eq!(CellType3::Line2.node_count(), 2);
        assert_eq!(CellType3::Triangle3.node_count(), 3);
        assert_eq!(CellType3::Quad4.node_count(), 4);
        assert_eq!(CellType3::Tetrahedron4.node_count(), 4);
        assert_eq!(CellType3::Hexahedron8.node_count(), 8);
        assert_eq!(CellType3::Line2.topological_dimension(), 1);
        assert_eq!(CellType3::Quad4.topological_dimension(), 2);
        assert_eq!(CellType3::Tetrahedron4.topological_dimension(), 3);
    }

    #[test]
    fn default_grid_has_valid_empty_connectivity() {
        let grid = UnstructuredGrid3::default();
        assert!(grid.is_empty());
        assert_eq!(grid.cell_offsets(), &[0]);
        assert_eq!(grid.cell_point_indices(0), None);
    }

    #[test]
    fn accepts_empty_grid_and_points_without_cells() {
        let empty = UnstructuredGrid3::new(Vec::new(), Vec::new(), vec![0], Vec::new()).unwrap();
        assert!(empty.is_empty());
        assert_eq!(empty.cell_count(), 0);

        let points_only = UnstructuredGrid3::new(
            vec![point(0.0, 0.0, 0.0), point(1.0, 0.0, 0.0)],
            Vec::new(),
            vec![0],
            Vec::new(),
        )
        .unwrap();
        assert_eq!(points_only.point_count(), 2);
        assert_eq!(points_only.cell_count(), 0);
        assert!(!points_only.is_empty());
    }

    #[test]
    fn rejects_offsets_with_wrong_count() {
        assert_eq!(
            UnstructuredGrid3::new(vec![], vec![CellType3::Line2], vec![0], vec![]),
            Err(UnstructuredGridError::CellOffsetCountMismatch {
                expected: 2,
                actual: 1,
            })
        );
    }

    #[test]
    fn rejects_offsets_that_do_not_start_at_zero() {
        assert_eq!(
            UnstructuredGrid3::new(vec![], vec![], vec![1], vec![]),
            Err(UnstructuredGridError::CellOffsetsMustStartAtZero { actual: 1 })
        );
    }

    #[test]
    fn rejects_decreasing_offsets() {
        assert_eq!(
            UnstructuredGrid3::new(
                vec![],
                vec![CellType3::Line2, CellType3::Line2],
                vec![0, 2, 1],
                vec![0, 1],
            ),
            Err(UnstructuredGridError::CellOffsetsNotMonotonic {
                offset_index: 2,
                previous: 2,
                actual: 1,
            })
        );
    }

    #[test]
    fn rejects_terminal_offset_that_differs_from_connectivity_length() {
        assert_eq!(
            UnstructuredGrid3::new(vec![], vec![], vec![0], vec![0]),
            Err(UnstructuredGridError::CellOffsetsEndMismatch {
                expected: 1,
                actual: 0,
            })
        );
    }

    #[test]
    fn rejects_cell_with_wrong_node_count() {
        assert_eq!(
            UnstructuredGrid3::new(
                vec![point(0.0, 0.0, 0.0); 3],
                vec![CellType3::Triangle3],
                vec![0, 2],
                vec![0, 1],
            ),
            Err(UnstructuredGridError::CellNodeCountMismatch {
                cell_index: 0,
                cell_type: CellType3::Triangle3,
                expected: 3,
                actual: 2,
            })
        );
    }

    #[test]
    fn rejects_out_of_bounds_point_index_with_cell_location() {
        assert_eq!(
            UnstructuredGrid3::new(
                vec![point(0.0, 0.0, 0.0); 2],
                vec![CellType3::Line2],
                vec![0, 2],
                vec![0, 2],
            ),
            Err(UnstructuredGridError::PointIndexOutOfBounds {
                cell_index: 0,
                local_node_index: 1,
                point_index: 2,
                point_count: 2,
            })
        );
    }

    #[test]
    fn rejects_repeated_point_indices_within_a_cell() {
        assert_eq!(
            UnstructuredGrid3::new(
                vec![point(0.0, 0.0, 0.0); 2],
                vec![CellType3::Line2],
                vec![0, 2],
                vec![0, 0],
            ),
            Err(UnstructuredGridError::RepeatedPointIndex {
                cell_index: 0,
                point_index: 0,
            })
        );
    }

    #[test]
    fn rejects_non_finite_points() {
        assert_eq!(
            UnstructuredGrid3::new(
                vec![point(0.0, 0.0, 0.0), point(f64::NAN, 0.0, 0.0)],
                vec![],
                vec![0],
                vec![],
            ),
            Err(UnstructuredGridError::NonFinitePoint { point_index: 1 })
        );
    }

    #[test]
    fn preserves_geometrically_degenerate_cells_with_distinct_indices() {
        let grid = UnstructuredGrid3::new(
            vec![
                point(0.0, 0.0, 0.0),
                point(1.0, 0.0, 0.0),
                point(2.0, 0.0, 0.0),
            ],
            vec![CellType3::Triangle3],
            vec![0, 3],
            vec![0, 1, 2],
        )
        .unwrap();
        assert_eq!(grid.cell_count(), 1);
    }
}
