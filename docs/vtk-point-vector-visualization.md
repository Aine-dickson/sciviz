# M10.2 — Imported VTK point-vector visualization

## Status

Design contract. Implementation is authorized only after this design PR is
reviewed and merged.

## Objective

Allow a user to select a point-associated, world-basis 3-component vector
field from an imported VTK legacy ASCII unstructured grid and see its
direction represented by scientific arrow glyphs over the imported volume
boundary.

The source Dataset and vector values remain authoritative CPU-side data.
Glyphs are a derived visualization representation and must preserve the
source point identity for every vector entry.

## Rationale

The framework already has the complete reusable vector pipeline:

`Dataset + VectorField3 -> resolve_vector_sources -> analyze_vector_sources
-> build_vector_glyphs -> VectorGlyphSet3 -> svf-render`.

M7.3/M7.4 established point- and cell-associated vector semantics, explicit
scale policies, f64 arrow geometry, and depth-aware rendering. M9.1 imports
VTK point/cell vector arrays, but M9.2's Explorer loading path currently
renders only surfaces and scalar fields. M10.2 connects those existing
capabilities so imported scientific vector data can be visualized without
adding another vector algorithm or a VTK-specific core API.

This increment deliberately focuses on point-associated vectors. The
existing cell-associated anchor is the arithmetic mean of a cell's vertices;
for a volume cell this lies inside the opaque boundary and is therefore
occluded by the current depth-tested surface. A useful unstructured
volume-cell glyph representation requires a separate visibility policy and
is not silently introduced here.

## User workflow and CLI

Add one optional argument to the existing imported boundary workflow:

```powershell
cargo run -p svf-explorer -- --vtk <path> --vtk-vectors <field-name>
```

It may be combined with the existing boundary scalar coloring option:

```powershell
cargo run -p svf-explorer -- --vtk <path> --vtk-scalar <scalar-field> --vtk-vectors <vector-field>
```

Rules:

- `--vtk-vectors` requires `--vtk <path>`.
- The field name is exact and case-sensitive, matching existing field selection.
- The selected field must exist, be a Vector3 field, have Point association,
  and use `VectorBasis3::World`.
- The option applies only to the imported volume-boundary representation.
  It cannot be combined with `--vtk-isosurface`,
  `--vtk-isovalue`, or `--vtk-isosurface-color`. Source-point glyphs on a
  derived isosurface would require a separate visibility and relevance
  contract; do not attach them implicitly.
- `--vtk-vectors` may be combined with `--vtk-scalar`. Scalar surface
  coloring and vector glyph rendering are independent derived
  representations of the same retained Dataset.
- Duplicate options, missing values, unknown options in VTK mode, missing
  fields, scalar fields selected as vectors, cell/dataset association, and
  unsupported vector bases are clear pre-window startup errors.
- Existing demo invocations and all existing VTK invocations retain behavior
  when `--vtk-vectors` is absent.

No new interactive field picker, command mode, dependency, or file format is
introduced.

## Scientific semantics

### Source resolution

- Reuse `resolve_vector_sources` with the exact selected field name.
- Accept Point association only. Each source vector is anchored at its exact
  original VTK point coordinate and retains `VectorEntity3::Point(point_id)`.
- Do not average, resample, reorder, or relocate point vectors.
- Preserve original vector components and field metadata in the retained
  Dataset. Do not normalize or rewrite source values.
- Require `VectorBasis3::World`. The VTK legacy importer currently assigns
  this basis to parsed VECTORS arrays; reject any other basis rather than
  guessing a transform.
- Analyze with `analyze_vector_sources`, then construct geometry with
  `build_vector_glyphs`. Keep every source entry, including zero and
  non-finite vectors that have no displayable arrow geometry.

### Display scale

Vector magnitude units and coordinate units are not necessarily comparable.
Do not infer a magnitude-to-world scale or silently use raw magnitudes as
lengths.

Use `VectorScalePolicy3::UniformLength` for this first imported workflow:
all finite, non-zero directions receive the same display length, preserving
direction without implying a quantitative magnitude scale. Derive that length
deterministically as 5% of the imported boundary mesh's bounding-box diagonal,
with a documented small positive floor for degenerate bounds. Record the
actual policy and length in `VectorGlyphSet3`; do not modify source vectors.

Magnitude-scaled glyphs, user-specified scale factors, and legends are
deferred. The UI/terminal summary must state that arrows use uniform display
length so users do not mistake arrow length for vector magnitude.

### Visibility and rendering

- Render glyphs with the existing `SceneRenderer::new_with_vector_glyphs`
  path when the boundary is uncolored.
- When `--vtk-scalar` is also supplied, use the existing
  `SceneRenderer::new_with_mapped_surface_and_vectors` path.
- Reuse the renderer's existing vector line pipeline, color, and depth test.
  Do not force glyphs in front of the surface or disable occlusion.
- Glyph anchors are source points on the volume boundary. Arrows directed
  into the volume may be naturally occluded; this is correct depth behavior,
  not a reason to bypass the depth buffer.
- Keep the source Dataset and `VectorGlyphSet3` alive in Explorer for the
  scene lifetime. Do not reconstruct vectors from GPU buffers.
- The current imported surface camera fit remains authoritative. The chosen
  glyph length is bounded relative to the mesh diagonal and must fit within
  the existing camera margin; if testing demonstrates clipping, adjust the
  fit deliberately and test it.

The current gold glyph color is retained. Per-vector color mapping, vector
legends, labels, glyph picking, and a new renderer pipeline are out of scope.

## Explorer state and diagnostics

- Extend `VtkLoadConfig` with an optional vector field name.
- Extend `ImportedScene` with the prepared `VectorGlyphSet3` and selected
  field name, while retaining the source Dataset.
- Extend the startup summary to identify the selected vector field, source
  vector count, number of displayable arrows, and uniform display-length
  policy/length. Do not imply that zero or invalid vectors were drawn.
- Keep the existing imported-surface inspection behavior. Inspection remains
  surface-only; vector glyphs are not selectable scientific entities in this
  increment.
- Keep hover/click inspection, camera orbit/pan/zoom, resize, minimize, and
  restore behavior unchanged.

## Tests and acceptance

### CLI tests

- Parse `--vtk path --vtk-vectors velocity`.
- Parse combined `--vtk path --vtk-scalar pressure --vtk-vectors velocity`.
- Reject missing path/field value, duplicate vector options, vector option
  without `--vtk`, unknown options, and conflicts with isosurface options.
- Preserve all existing CLI parsing behavior when the new option is absent.

### Core-pipeline and scene-preparation tests

- Load a fixture containing a point-associated VTK VECTORS array and verify
  its exact name, World basis, point association, cardinality, and values.
- Verify resolved source anchors match original VTK point coordinates and
  source entities preserve original point IDs and order.
- Verify finite non-zero vectors produce glyphs with the documented uniform
  length and preserve direction.
- Verify zero and non-finite vectors retain source entries but produce no
  arrow geometry.
- Reject a missing field, scalar field, cell-associated vector field,
  and dataset-associated vector field with distinct useful errors.
- Reject invalid field cardinality/basis if constructible through the public
  Dataset API; never silently omit or mislabel source vectors.
- Verify simultaneous scalar surface mapping and vector glyph preparation
  retains both independent results and does not change the source Dataset.
- Verify summary counts distinguish total source vectors from displayable
  arrows and report the uniform-length policy.

### User Windows acceptance

- `cargo test -p svf-explorer`
- `cargo test --workspace`
- `cargo check --workspace`
- Run an imported VTK fixture with point-vector glyphs.
- Run the same fixture with both scalar surface coloring and vector glyphs.
- Confirm glyphs originate at the expected source points and point in the
  expected directions; verify the uniform-length policy is visible in the
  summary and does not imply magnitude.
- Confirm natural surface occlusion remains enabled.
- Verify camera framing, orbit, pan, zoom, hover/click surface inspection,
  resize, minimize, and restore.
- Verify existing default/demo modes and all existing VTK workflows remain
  unchanged.

Do not claim Windows acceptance until the user reports the commands and
runtime checks. Do not run `cargo fmt` unless separately requested.

## Explicit non-goals

- Cell-associated volume glyphs and their visibility/placement policy.
- Dataset-associated vectors or implicit anchors.
- Vector glyphs on extracted isosurfaces or standalone lower-dimensional
  cells.
- Magnitude-scaled display, user-controlled scale, legends, color mapping,
  labels, or interactive field selection.
- Vector glyph picking/inspection, selection, editing, or annotations.
- Streamlines, pathlines, integration, divergence, curl, gradients, tensors,
  or other derived vector analysis.
- Changes to VTK parser semantics, new cell types, formats, dependencies,
  GPU readback, or new rendering pipelines.

## Dependencies

- M7.3 point/cell vector source resolution, analysis, and glyph construction.
- M7.4 structured cell-vector semantics (with volume-cell visibility
  intentionally deferred).
- M9.1 VTK legacy ASCII vector-field import.
- M9.2 Explorer imported-scene loading and scalar surface coloring.
- M10.1 imported surface inspection, which remains surface-only.
