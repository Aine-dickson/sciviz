# Unstructured topology foundations

## Purpose

Introduce a graphics-independent representation of unstructured scientific
cells in `svf-core`. Source connectivity is authoritative. Any later
triangulation, tessellation, or GPU conversion is derived data and must retain
a mapping back to source cell identity.

This increment extends the existing `Dataset` abstraction. It does not
replace `TriangleMesh3`, `Polyline3`, or `StructuredGrid2D`; those remain
useful specialized geometry types.

## Core representation

Add an `UnstructuredGrid3` containing:

- an ordered array of finite world-space points (`WorldPoint3`);
- one `CellType3` value per source cell;
- a CSR-style `cell_offsets` array of length `cell_count + 1`;
- a flattened `connectivity` array of `u32` point indices.

For source cell `i`, its point indices are
`connectivity[cell_offsets[i]..cell_offsets[i + 1]]`.
The cell's stable identity within this immutable grid is its original array
index `i`. Point identity is likewise the original point-array index.

The constructor accepts the arrays without reordering them. Public accessors
expose read-only slices and a bounds-checked cell connectivity lookup.

## Initial linear cell types

| Cell type | Nodes | Topological dimension | Local ordering |
|---|---:|---:|---|
| `Line2` | 2 | 1 | start, end |
| `Triangle3` | 3 | 2 | cyclic perimeter order |
| `Quad4` | 4 | 2 | cyclic perimeter order |
| `Tetrahedron4` | 4 | 3 | caller-supplied simplex vertex order; orientation is preserved, not validated |
| `Hexahedron8` | 8 | 3 | cyclic order around one face, then corresponding cyclic order around the opposite face; indices 0–3 correspond to 4–7 |

These are linear cells only. Higher-order cells, polygons with variable
arity, wedges, pyramids, and vertex cells are deferred. Input node ordering
is preserved exactly; the constructor does not certify orientation. For a
hexahedron, corresponding vertical pairs are (0,4), (1,5), (2,6), and (3,7).
Independent point data remains represented by `PointSet3`.

Mixed cell types and mixed topological dimensions are permitted in one
unstructured grid. This supports, for example, embedded line or surface
entities alongside volume cells. Cell-field values always follow the
source-cell array order, regardless of cell type or dimension.

## Validation invariants

Construction must reject:

1. non-finite point coordinates;
2. an offsets array whose length is not `cell_types.len() + 1`;
3. offsets that do not start at zero;
4. offsets that decrease;
5. a final offset that differs from `connectivity.len()`;
6. a cell whose connectivity span does not match its fixed node count;
7. a connectivity index outside the point array;
8. repeated point indices within one cell.

Empty grids are valid when offsets is exactly `[0]` and connectivity and cell
types are empty. Points without cells are valid; unreferenced points are
preserved.

Structural validity is deliberately distinct from geometric quality.
Distinct point indices may refer to coincident coordinates. Collinear
triangles, zero-area quads, zero-volume tetrahedra, and other geometrically
degenerate but structurally valid cells are retained. The constructor does
not normalize orientation, reorder nodes, infer missing cells, or repair
connectivity.

## Dataset integration

Add `DatasetGeometry::UnstructuredGrid3` and its `From` conversion.

- Dataset point count is the grid's point count.
- Dataset cell count is the source cell count.
- Existing point- and cell-associated scalar/vector fields use the existing
  cardinality validation without special cases.
- Cell-associated values correspond to source cell IDs, not any later
  triangles or other render primitives.

No rendering or triangulation is included in this milestone. When a future
renderer derives triangles from a quad or volume cell, it must retain source
cell identity for picking, field lookup, and diagnostics.

## Explicit exclusions

- GPU/render integration and surface extraction;
- file-format import/export;
- high-order finite elements;
- polygonal variable-arity cells;
- mesh repair, orientation correction, and quality metrics;
- adjacency/manifold analysis for general unstructured cells;
- interpolation, field remapping, and time-varying datasets.

## Verification status

Unit tests now cover valid mixed cells, stable point/cell ordering, empty and
points-only grids, default construction, validation failures, preservation of
geometrically degenerate cells, and Dataset field attachment using source
cell counts. These tests are committed on the M6.3 branch but have not yet
been run on the user's Windows environment.
