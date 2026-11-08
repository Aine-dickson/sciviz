# M6.6 — Provenance-preserving boundary surface triangulation

## Objective

Derive a triangle-only surface representation from the M6.5
`BoundarySurface3` without changing the source boundary surface or losing
source identity. This is a graphics-independent topology conversion in
`svf-core`, not a rendering feature.

## Input and output

Input: an immutable `BoundarySurface3`.

Output: a `TriangulatedSurface3` containing:
- The same compact f64 world-space point array.
- The same compact-to-original-source point-ID mapping.
- A deterministic list of triangles.
- Per-triangle provenance identifying the source boundary-face index,
  source volume-cell index, source local-face index, and triangle ordinal
  within that source face.

The output should own or clone the point and source-ID arrays rather than
borrow from the input, keeping the result independently usable. It must not
mutate the input.

## Triangulation rules

- A source triangle produces exactly one output triangle with its three
  compact point indices in exactly the original order.
- A source quad with local cycle `[v0, v1, v2, v3]` produces exactly two
  triangles.
- Select the diagonal by comparing the two endpoint pairs using **original
  source point IDs**, each pair sorted ascending. Choose the lexicographically
  smaller pair:
  - diagonal 0–2: compare `(min(id(v0), id(v2)), max(...))`
  - diagonal 1–3: compare `(min(id(v1), id(v3)), max(...))`
- This policy is deterministic, independent of coordinates, and stable
  under changes to point positions that preserve source IDs.
- If diagonal 0–2 is selected, emit `[v0, v1, v2]` then
  `[v0, v2, v3]`.
- If diagonal 1–3 is selected, emit `[v0, v1, v3]` then
  `[v1, v2, v3]`.
- Output triangle order follows source boundary-face order; triangles from
  each source face use ordinal 0 then 1.

Both split patterns preserve the source quad's local cyclic order. They do
not certify outward orientation: M6.3 does not certify source volume-cell
orientation, and M6.5 explicitly preserves reference cycles only.

## Provenance

Every output triangle retains:
- `boundary_face_index`: index into the input `BoundarySurface3::faces()`,
  allowing direct lookup of the M6.5 boundary-face record.
- `source_face_index`: original M6.4 topology face index retained by the
  M6.5 boundary-face record.
- `source_cell_index`: original volume cell index from M6.5.
- `source_local_face_index`: local face-template index in that cell.
- `source_triangle_index`: 0 for the first triangle emitted from that
  source face, 1 for the second; source triangles have index 0.

Triangle connectivity indexes the output's unchanged compact point array.
The parallel source-point-ID mapping remains unchanged, so callers can
resolve each triangle vertex to its original `UnstructuredGrid3` point ID.

## Error and allocation policy

No new connectivity index space is introduced: all output indices are
copied from the already validated M6.5 compact surface indices. Avoid
inventing generic error abstractions. If implementation requires fallible
allocation handling, define only the narrow error needed and document it.

## Required tests

1. Empty surface yields empty triangulated surface.
2. Triangle faces pass through unchanged with exact vertex order and
   provenance; triangle ordinal is zero.
3. Quad emits exactly two triangles for the 0–2 diagonal.
4. Quad emits exactly two triangles for the 1–3 diagonal.
5. The diagonal policy reads the explicit original source point-ID mapping
   and chooses the lexicographically smaller canonical endpoint pair.
6. Both split patterns preserve the source local cycle and deterministic
   triangle order.
7. Mixed triangle/quad surfaces preserve source face order and produce
   one/two triangles respectively.
8. Every output triangle preserves both the M6.5 boundary-face array index
   and original M6.4 topology face index, plus cell, local-face, and
   per-face triangle ordinal.
9. Point coordinates and source-point-ID mapping are preserved exactly.
10. Repeated triangulation is deterministic.
11. Input surface remains unchanged.
12. Geometrically degenerate quads are triangulated structurally; no
    geometric quality filter is implied.

## Explicit exclusions

- Renderer, GPU buffers, shaders, normals, lighting, and Explorer changes.
- Outward orientation certification or winding repair.
- Shortest-diagonal selection, coordinate-dependent diagonal choice,
  geometric quality optimization, or degenerate-triangle filtering.
- Field transfer/interpolation or field association changes.
- Surface welding, simplification, decimation, repair, or file I/O.

## Acceptance

- Public API is exported from `svf-core` and documented.
- Every source polygon is represented by one or two output triangles under
  the rules above, with complete provenance.
- The required focused tests pass.
- User validates `cargo check --workspace`, `cargo test -p svf-core`,
  and `cargo test --workspace` on Windows.
- Do not run formatting routines as routine cleanup; preserve unrelated
  local changes and stashes.
