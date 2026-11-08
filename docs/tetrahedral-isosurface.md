# M8.1 — Tetrahedral isosurface extraction

## Status

Implemented and merged to `main` on 2026-11-08 through core PR #16 and Explorer PR #17.

This increment is intended to be the first M8 derived-geometry pipeline.
It builds on the M6 dataset/field model, M7.2 scalar semantics, and the
provenance requirements established by surface triangulation and mapping.

## Implementation status

The implementation follows this contract. The core extraction API is
`extract_tetrahedral_isosurface`; the optional Explorer mode is
`--isosurface`. User-reported Windows workspace tests, workspace check,
and visual validation passed. See `HANDOFF.md` and `docs/MILESTONES.md`
for merge commits and validation details.

## Objective

Extract a deterministic triangle surface from a point-associated scalar
field on tetrahedral cells at one caller-supplied isovalue.

The result is derived scientific geometry. It must retain enough provenance
to identify the source tetrahedron and the source edge or vertex that
generated every output vertex. The source dataset and scalar field remain
unchanged.

## Why tetrahedra first

The current `UnstructuredGrid3` supports mixed cell types, including
`Tetrahedron4` and `Hexahedron8`. Supporting all volume cells at once would
introduce multiple interpolation and ambiguity rules.

Tetrahedra provide a bounded first case: a linear scalar field within each
tetrahedron intersects its six edges in a deterministic way. Hexahedron
decomposition, structured-grid isosurfaces, and other cell types are deferred
until their topology, shared-face consistency, and interpolation semantics
are designed explicitly.

## Input contract

- Input is an immutable `Dataset` whose geometry is
  `DatasetGeometry::UnstructuredGrid3`.
- The selected field is a `ScalarField` with
  `FieldAssociation::Point`.
- The field cardinality must match the dataset point count, as enforced by
  the existing dataset/field model.
- The isovalue must be finite.
- Only `CellType3::Tetrahedron4` cells are processed in this increment.
- Other cell types in a mixed dataset are not interpreted as tetrahedra.
  Their cell IDs must be reported as skipped/unsupported in the result
  metadata rather than silently treated as supported volume cells.
- Scalar values remain `SampleValue`; non-finite measurements are not
  rewritten or converted to zero.

Cell-associated and dataset-associated scalar fields are out of scope.
A cell-associated value is constant per source cell and does not define the
within-cell linear interpolation needed by this algorithm.

## Algorithm and classification

Use a marching-tetrahedra case analysis over each supported tetrahedron.

For this increment, classify finite scalar values using a half-open rule:
values strictly below the isovalue are on the low side; values equal to or
above it are on the high side. This makes exact-isovalue classification
deterministic and must be documented in the public API.

- Zero or four low-side vertices: no polygon is emitted.
- One or three low-side vertices: one triangle is emitted from the three
  crossing edges (or their exact-isovalue endpoint vertices).
- Two low-side vertices: the four crossings form a quadrilateral, split into
  two triangles using a deterministic diagonal rule based on canonical
  source point IDs.

For every crossing edge with endpoint scalar values `s0` and `s1`, compute
the linear interpolation parameter

`t = (iso - s0) / (s1 - s0)`

and interpolate the world-space position along that source edge. The
implementation must use numerically careful arithmetic, validate the
interpolation parameter and generated position, and report an explicit
error or skipped-cell reason if finite inputs cannot produce a finite result.
Do not silently clamp a mathematically invalid result.

When a crossing is exactly at a source vertex, represent it as a source-point
provenance entry rather than creating multiple coincident edge vertices.

## Shared vertices and deterministic output

Adjacent tetrahedra commonly share source edges. The extractor must reuse
one output vertex for each unique crossing source edge so the resulting
surface is connected and does not contain cracks caused by independently
computed duplicate positions.

Use canonical undirected source-edge identity (the ordered pair of source
point IDs) as the edge-cache key. Exact-isovalue source vertices use their
source point ID as their identity. Output order must be deterministic for
the same dataset, field, and isovalue; it must not depend on hash-map iteration
order.

A source edge's interpolation parameter in provenance is measured from the
lower source point ID to the higher source point ID. The resulting position
must be computed consistently with that canonical orientation.

## Output contract

Return a graphics-independent result, tentatively named `Isosurface3`,
containing:

- A validated `TriangleMesh3` with f64 world-space positions.
- One vertex-provenance entry per output mesh vertex:
  - `SourcePoint { point_id }` for an exact source-vertex intersection.
  - `SourceEdge { point_ids: [low_id, high_id], t }` for an interpolated
    edge intersection, with `t` measured from `low_id` to `high_id`.
- One source cell ID per output triangle, preserving the tetrahedron that
  generated it.
- The requested isovalue.
- Deterministic extraction metadata, including counts/IDs of unsupported
  cells and tetrahedra skipped because required scalar values are non-finite
  or geometry/interpolation is invalid.

The output must not be attached back to the source `Dataset` as if it were
source topology. It is a derived representation with explicit provenance.

A later field-transfer or scalar-mapping increment may use the edge weights
to interpolate source point fields onto generated vertices. M8.1 must not
pretend that an interpolated vertex has one source point ID.

## Winding and degeneracy

Triangles should have consistent orientation relative to the local scalar
gradient, using a documented convention (the high-scalar side is the
positive side of the oriented normal). The implementation must not infer
orientation from the tetrahedron's input ordering alone.

Define deterministic behavior for:
- Exact-isovalue vertices and edges.
- A tetrahedron whose scalar values are all equal to the isovalue.
- Repeated geometric positions with distinct source point IDs.
- Zero-area output triangles.
- Geometrically degenerate tetrahedra.
- Non-finite scalar values.
- Finite scalar values whose subtraction/interpolation would overflow.

Do not emit repeated-index or zero-area triangles. Any skipped source cells
must remain visible in result metadata with a reason. The implementation
must avoid duplicate coincident triangles when the isosurface passes exactly
through a shared tetrahedral face.

These edge cases should be resolved in the implementation design review
before coding begins; the public contract must not leave them to incidental
floating-point comparisons or container iteration order.

## Architecture

- `svf-core` owns classification, interpolation, topology, edge reuse,
  winding, validation, and provenance.
- `svf-render` receives the resulting triangle mesh through the existing
  mesh rendering path. No marching-tetrahedra logic or dataset interpretation
  enters the renderer.
- `svf-explorer` adds a deterministic `--isosurface` demonstration using a
  tetrahedral mesh and a point-associated scalar field. It should render the
  extracted surface and retain the source dataset and extraction result on
  the CPU for inspection.
- No wgpu types enter `svf-core`; scientific coordinates remain f64 until
  the renderer boundary.

## Required core tests

- All 16 tetrahedron low/high classification patterns, including complements.
- No intersection for all-low and all-high cases.
- One-triangle and two-triangle cases with expected geometry.
- Exact-isovalue vertex handling and deterministic half-open classification.
- Shared-edge vertex reuse across adjacent tetrahedra.
- Deterministic quadrilateral diagonal selection and output ordering.
- Consistent triangle winding relative to scalar gradient.
- Correct source cell IDs for every output triangle.
- Correct source point/edge provenance and canonical interpolation parameter.
- Non-finite point scalar handling without changing the source field.
- Unsupported mixed cell types are reported, not misinterpreted.
- Degenerate tetrahedra, zero-area triangles, and coincident source positions
  follow the documented skip/error policy.
- Extreme finite scalar values and coordinates do not silently create
  non-finite output.
- Empty result behavior is valid and deterministic.
- Source dataset, geometry, and scalar field remain unchanged.

## Explorer and Windows validation

- Add `--isosurface` without changing any existing mode.
- Use a deterministic tetrahedral fixture whose isosurface is analytically
  understandable and contains multiple adjacent tetrahedra.
- Verify the surface is continuous across shared tetrahedral edges/faces.
- Verify orbit, pan, zoom, picking, resize, and minimize/restore continue to
  work with the derived surface.
- Verify all previous Explorer modes remain unchanged.
- Run `cargo test --workspace` and `cargo check --workspace` on Windows.

## Explicit non-goals

- Hexahedron or general polyhedron isosurface extraction.
- Structured-grid marching cubes or structured-grid isosurfaces.
- Cell-associated or dataset-associated scalar interpolation.
- General field transfer/remapping between datasets.
- Contour lines, streamlines, volume rendering, or time-varying data.
- Mesh repair, smoothing, simplification, or topology mutation.
- GPU-side isosurface extraction.
- On-screen scalar legends or general UI overlays.

## Acceptance criteria

M8.1 is complete when `svf-core` produces deterministic, validated,
provenance-preserving isosurfaces for tetrahedral cells using point-associated
scalar data; shared edges are reused; exact-isovalue and degenerate cases have
documented deterministic behavior; the renderer consumes only the prepared
mesh; the Explorer demonstration works without regressing existing modes;
and Windows workspace tests and runtime validation pass.
