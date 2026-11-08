# M6.5 — Unstructured boundary surface representation

Status: design approved for implementation planning; no implementation or local validation is claimed.

## Objective

Turn the incidence information produced by M6.4 into a compact, reusable,
graphics-independent surface representation. The result is a derived view of
an `UnstructuredGrid3`; the original volume grid remains authoritative.

This increment stops before renderer integration. In particular, it does not
silently convert quadrilaterals into triangles or transfer source fields.

## Public API contract

Add a sibling module, `crates/svf-core/src/boundary_surface.rs`, exported from
`svf_core`.

The exact API should be kept small:

- `UnstructuredGrid3::boundary_surface() -> Result<BoundarySurface3, BoundarySurfaceError>`
- `BoundarySurface3::points() -> &[WorldPoint3]`
- `BoundarySurface3::source_point_ids() -> &[u32]`
- `BoundarySurface3::faces() -> &[BoundarySurfaceFace3]`
- `BoundarySurface3::point_count() -> usize`
- `BoundarySurface3::face_count() -> usize`
- `BoundarySurfaceFace3::vertices() -> BoundaryFaceVertices3`
- `BoundarySurfaceFace3::source_face_index() -> usize`
- `BoundarySurfaceFace3::source_cell_index() -> usize`
- `BoundarySurfaceFace3::source_local_face_index() -> usize`
- `BoundaryFaceVertices3::{Triangle([u32; 3]), Quad([u32; 4])}`
- `BoundarySurfaceError::PointIndexOverflow`

Surface face indices address the compact `BoundarySurface3::points()` array.
The parallel `source_point_ids()` array maps each compact point index back to
the corresponding point ID in the original `UnstructuredGrid3`. The point
array is compact and deterministic: include only points referenced by
extracted boundary faces, ordered by ascending source point ID.

Faces are emitted in the deterministic order already provided by
`UnstructuredVolumeTopology3::faces()`. Only faces classified as
`VolumeFaceClassification::Boundary` are included. A face with more than two
incident volume cells is non-manifold and is deliberately excluded; callers
can still inspect it through `volume_topology().non_manifold_faces()`.

For each included face, use the sole incidence's local vertex cycle and retain
its source topology face index, source cell index, and local face index.
Triangle and quad arity remain explicit. Do not triangulate quads in core.

## Identity, ordering, and orientation

- Source point identity is the original u32 point ID, never coordinate welding.
- Compact point IDs are assigned in ascending source point-ID order.
- Face order follows the M6.4 canonical face-key order.
- Face vertex order follows the sole source-cell incidence's local cycle.
- This cycle is a deterministic reference order, not a certified outward
  orientation. M6.3 does not certify volume-cell orientation.
- No point duplication per face: adjacent boundary faces share compact point
  indices when they share source point IDs.

## Scientific semantics

- The output owns copied f64 world positions and explicit source-point mapping.
- It does not mutate, consume, or replace the source grid.
- It does not own or copy dataset fields. Point/cell field transfer and
  association semantics remain a later, separately designed increment.
- It does not claim geometric watertightness, manifoldness, valid cell
  orientation, normals, area, volume, or element quality.
- Geometrically degenerate but structurally valid source cells are preserved;
  this increment does not perform geometric filtering.
- Lower-dimensional cells remain ignored because M6.4 volume topology ignores
  them.
- Empty input and volume grids with no boundary faces produce empty point and
  face arrays.

## Validation and errors

The only planned domain error is compact-index overflow when converting a
compact point index to u32. Preserve the existing source point IDs unchanged.
Do not add generic allocation/error abstractions unless implementation
demonstrates a concrete need.

Tests must cover:

1. One tetrahedron: four triangle faces and four compact points.
2. One hexahedron: six quad faces and eight compact points.
3. Two adjacent tetrahedra: only the six exterior triangles are emitted;
   the shared interior face is absent.
4. Two adjacent hexahedra: ten exterior quads are emitted.
5. Mixed tetrahedral/hexahedral grids retain both face arities.
6. Three cells sharing a face: the non-manifold face is excluded.
7. Lower-dimensional cells do not contribute surface faces.
8. Compact point IDs map back to ascending original source point IDs.
9. Shared source points are represented once and reused by multiple faces.
10. Face source IDs and local ordering match M6.4 incidence data.
11. Face and point ordering is deterministic across repeated extraction.
12. Coincident coordinates with different source point IDs remain distinct.
13. Empty and non-volume grids return empty surfaces.
14. Structurally valid geometric degeneracy is retained.
15. Source grid and its topology are unchanged by extraction.

## Explicit exclusions

- wgpu, renderer, Explorer, GPU buffers, shaders, or visualization styling.
- Quad triangulation or diagonal selection.
- Normals, orientation repair, geometric quality, watertightness checks.
- Field copying, interpolation, remapping, or source association conversion.
- Surface editing, welding, simplification, decimation, or mesh repair.
- File import/export and higher-order cells.

## Acceptance criteria

- The API is exported from `svf-core` and documented.
- The output is deterministic, compact, and provenance-preserving.
- All tests above pass.
- `cargo check --workspace`, `cargo test -p svf-core`, and
  `cargo test --workspace` pass on the user's Windows environment.
- No formatter is run as routine cleanup; preserve unrelated local changes and
  stashes.
