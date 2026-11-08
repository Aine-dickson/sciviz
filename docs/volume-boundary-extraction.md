# M6.4 — Unstructured volume boundary extraction

## Objective

Derive the face topology of supported three-dimensional unstructured cells and
classify each unique face by its source-cell incidence. The output remains
graphics-independent and preserves source-cell identity for later rendering,
field lookup, and picking.

This increment is a topology query, not a mesh repair, geometric validation,
surface triangulation, or renderer feature.

## Scope

Only volume cells contribute faces:

- `Tetrahedron4`: four triangular faces.
- `Hexahedron8`: six quadrilateral faces.

The existing lower-dimensional `Line2`, `Triangle3`, and `Quad4` cells
remain in the source grid and are ignored by this volume-topology query. This
avoids inventing mixed-dimensional boundary semantics in this increment.

## Public API proposal

Add `crates/svf-core/src/unstructured_topology.rs`, exported from
`svf-core`.

The query is exposed from `UnstructuredGrid3`:

```rust
impl UnstructuredGrid3 {
    pub fn volume_topology(&self) -> UnstructuredVolumeTopology3;
}
```

The derived result contains every unique face and its incident volume cells.
Suggested public types:

- `UnstructuredVolumeTopology3`: owns the deterministic unique-face list;
  exposes `faces()`, `boundary_faces()`, `interior_faces()`,
  `non_manifold_faces()`, and `face_count()`.
- `VolumeFace3`: exposes its classification, representative ordered point
  indices, and all `VolumeFaceIncidence3` records.
- `VolumeFaceIncidence3`: exposes the source cell ID, local face index, and
  that cell's original ordered point indices for this face.
- `VolumeFaceVertices3`: distinguishes `Triangle([u32; 3])` from
  `Quad([u32; 4])`.
- `VolumeFaceClassification`: `Boundary`, `Interior`, or
  `NonManifold`.

The representative ordering is the ordering from the first source-cell
incidence. Each incidence separately retains its own ordering, so no local
winding information is lost.

The result is derived from an already validated immutable grid; no additional
validation error is expected from the query.

## Face identity and classification

Face identity is topological and uses point IDs, not coordinates.

- A triangular face key is its three point IDs sorted in ascending order.
- A quadrilateral face key is its four point IDs sorted in ascending order.
- Face arity is part of the key, so triangles and quads cannot match.
- Different point IDs remain distinct even when their world coordinates are
  coincident.
- Original local point order is retained separately in each incidence.

Classification is based only on the number of incident volume cells:

| Incidence count | Classification |
| ---: | --- |
| 1 | Boundary |
| 2 | Interior |
| 3 or more | Non-manifold |

This is a local incidence classification. It does not certify geometric
watertightness, valid cell orientation, non-overlap, or global manifoldness.

## Local face templates

Templates refer to local connectivity positions in each source cell. The
point IDs are obtained by indexing the cell's CSR connectivity with these
positions.

### Tetrahedron4

For local nodes `0, 1, 2, 3`, use these four reference face cycles:

```text
local face 0: [1, 2, 3]
local face 1: [0, 3, 2]
local face 2: [0, 1, 3]
local face 3: [0, 2, 1]
```

These cycles are outward for the conventional positively oriented simplex
ordering. M6.3 does not certify simplex orientation, so M6.4 treats them as
deterministic reference cycles only. It must not claim they are outward for
arbitrary input.

### Hexahedron8

M6.3 defines local nodes 0–3 as one cyclic face, 4–7 as the opposite cyclic
face, and corresponding vertical pairs (0,4), (1,5), (2,6), (3,7).

Use these reference face cycles:

```text
local face 0: [0, 3, 2, 1]
local face 1: [4, 5, 6, 7]
local face 2: [0, 1, 5, 4]
local face 3: [1, 2, 6, 5]
local face 4: [2, 3, 7, 6]
local face 5: [3, 0, 4, 7]
```

These are the conventional reference cycles for a positively ordered
hexahedron. As with tetrahedra, they are not an orientation certificate for
arbitrary input.

M6.4 does not reorder or normalize any face cycle. Adjacent incidences retain
their own local cycles, allowing a future orientation-consistency query to
compare them explicitly.

## Determinism

- Unique faces are ordered lexicographically by their canonical point-ID key.
- Incidences within each face are ordered by source cell ID.
- Local face indices follow the templates above.
- Repeated calls on an unchanged grid return equal results.
- Reordering source cells may change source cell IDs, as those IDs are source
  array positions, but must not change the canonical face-key ordering or
  classification when the same cells remain present.

A sorted map keyed by canonical face identity is a suitable implementation;
do not expose hash-map iteration order as public ordering.

## Mixed cell types

A tetrahedron has triangular faces; a hexahedron has quadrilateral faces.
Therefore a linear tetrahedron and linear hexahedron cannot share a complete
face in this model. A mixed-grid test should verify that both cell types
contribute their respective boundary faces, not invent a tetrahedron/hex
shared-face case.

## Explicit exclusions

- No surface triangulation; quads remain quads.
- No geometric coordinate welding or tolerance-based face matching.
- No outward-orientation certification or automatic winding repair.
- No degenerate-cell rejection beyond M6.3 structural validation.
- No volume, area, normal, or quality calculations.
- No connected-component analysis.
- No boundary extraction from lower-dimensional cells.
- No renderer or GPU changes.
- No mutation of the source grid or attached dataset fields.

## Acceptance tests

1. One tetrahedron produces four boundary triangles.
2. One hexahedron produces six boundary quads.
3. Two tetrahedra sharing one triangular face classify that face as interior.
4. Two hexahedra sharing one quadrilateral face classify that face as interior.
5. Three tetrahedra sharing one triangular face classify it as non-manifold.
6. A mixed grid retains independent tetrahedral and hexahedral boundary faces.
7. Surface cells in a mixed-dimensional grid do not contribute to volume
   topology.
8. Each face incidence reports the correct source cell ID and local face index.
9. Each incidence preserves its local ordered point IDs.
10. Repeated queries are deterministic.
11. Reordering source cells preserves canonical face ordering and
    classification while updating source cell IDs to the new source positions.
12. Geometrically coincident faces with different point IDs remain distinct.
13. Empty grids and grids containing only lower-dimensional cells produce an
    empty volume topology.
14. Structurally valid geometrically degenerate cells remain represented;
    no geometric validity claim is introduced.

## Completion checks

- Add focused unit tests for every acceptance condition.
- Keep all implementation in `svf-core`; no wgpu types.
- Preserve f64 source geometry and all source IDs.
- Verify `cargo check --workspace` on Windows.
- Verify `cargo test -p svf-core` on Windows.
- Verify `cargo test --workspace` on Windows.
- Do not run formatting routines as a routine cleanup step. Inspect and avoid
  unrelated formatting churn.
