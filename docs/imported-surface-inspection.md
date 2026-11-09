# M10.1 — Scientific inspection of imported surfaces

## Status

Design contract. Implementation is authorized only after this design PR is
reviewed and merged.

## Objective

Make imported VTK boundary surfaces and extracted tetrahedral isosurfaces
scientifically inspectable by ray-based hover and click interaction.

A user should be able to point at a rendered triangle and recover the
scientific entity it represents, its world-space hit location, and the
applicable source scalar values. The original Dataset remains authoritative;
inspection must use the existing topology and isosurface provenance rather
than infer source identity from render-vertex numbering.

## Rationale

The framework architecture states that interaction should resolve a screen
location to a scientific entity and associated values. M5 already provides
ray construction, triangle intersection, and scientific inspection for the
sampled parametric surface. M6–M9 added provenance-preserving unstructured
boundary surfaces, VTK import, tetrahedral isosurfaces, and scalar transfer.

However, Explorer currently reports that unstructured surface inspection is
not implemented. M10.1 closes this gap and exercises the provenance model
with actual imported scientific data. It is a more valuable next increment
than introducing another input format or rendering mode before users can
inspect the data already supported.

## User workflow

For an imported VTK boundary or isosurface:

- Move the pointer over a visible triangle to receive concise live inspection
  feedback.
- Click to print a stable, detailed inspection record to the terminal.
- Move outside the surface to receive an explicit MISS state.
- Continue to use left-drag orbit, middle-drag pan, wheel zoom, and the
  existing click-versus-drag threshold without accidental click inspection
  after a camera gesture.

The first implementation uses the existing window-title hover feedback and
terminal click diagnostics. A dock, side panel, tooltip framework, field
selector, or other new UI system is not required.

## Scientific semantics

### Common hit data

Inspection is performed against the exact CPU-side f64 triangle mesh used
to prepare the visible surface, with the renderer's active camera and current
physical viewport.

A successful hit reports:

- world-space hit position (f64);
- ray distance in world units;
- rendered triangle index;
- barycentric weights in that triangle's stored vertex order;
- original source volume-cell ID;
- source boundary face ID and local face ID for a boundary-surface hit;
- source triangle ordinal within that boundary polygon, where applicable.

The result must distinguish a miss from an invalid ray or invalid viewport.
The nearest non-negative two-sided triangle hit wins, using the existing
ray/mesh intersection semantics. No GPU readback or GPU-only picking IDs are
introduced.

### Imported volume boundary

For a boundary hit, map the hit triangle's compact surface vertex indices
through `TriangulatedSurface3::source_point_ids()` to original VTK point IDs.
Use the triangle's existing provenance to report its original volume cell,
source face, local face, and polygon-triangle ordinal.

Scalar values:
- Point-associated scalar fields are evaluated piecewise-linearly at the
  hit using the triangle barycentric weights and the three original source
  point values.
- Cell-associated scalar fields are reported exactly from the hit triangle's
  original source volume-cell ID; they are not interpolated between cells.
- Dataset-associated scalar fields, if present in future inputs, are reported
  as the single dataset value.
- Non-finite source values remain explicitly non-finite. Do not silently
  discard them, replace them with zero, or present them as finite.

### Extracted tetrahedral isosurface

For an isosurface hit, report its triangle index, barycentric weights,
world-space position, and source tetrahedron cell ID from
`Isosurface3::triangle_source_cells()`.

For point-associated scalar fields, first resolve each derived isosurface
vertex's source-point/source-edge provenance using the existing M8.2 transfer
semantics, then interpolate the three transferred vertex values using the
hit barycentric weights. This applies both to the field used to extract the
isosurface and to independent point scalar fields such as `pressure`.
The extraction field should therefore evaluate to the requested isovalue
within the documented numerical behavior of the transfer and barycentric
operations; report the computed value rather than substituting the requested
isovalue.

Cell-associated scalar fields are read exactly from the source tetrahedron
cell ID. Dataset-associated scalar fields are reported as the dataset value.
No implicit resampling or interpolation of cell values is allowed.

### Initial field coverage

M10.1 reports scalar fields only. It should include all supported scalar
fields attached to the source Dataset, retaining each field's exact name and
association. Vector fields, tensors, derived gradients, units conversion,
and statistical summaries are deferred.

If a field value is non-finite, report its non-finite category explicitly.
A field whose cardinality or association cannot be reconciled with the source
Dataset is an inspection error, not a reason to return a partial or
mislabelled result.

## Core architecture

- Keep ray construction, triangle intersection, source identity resolution,
  barycentric interpolation, and inspection result types in graphics-
  independent `svf-core`.
- Reuse `Ray3` and `intersect_mesh`; do not duplicate the triangle
  intersection algorithm in Explorer.
- Provide explicit inspection paths for `TriangulatedSurface3` boundary
  representations and `Isosurface3` derived representations. Shared result
  data may be factored into a common type only where it improves clarity.
- Pass the authoritative source `Dataset` to inspection so values are read
  from original fields. Do not copy fields into the renderer or mutate the
  Dataset.
- Preserve f64 calculations and values throughout inspection.
- Keep window events, cursor state, camera-to-ray conversion, title updates,
  and terminal output in Explorer.
- Keep `svf-render` unaware of VTK files, field names, topology provenance,
  and scientific inspection policy.
- Do not add dependencies, a new crate, a general pipeline framework, or a
  GPU picking subsystem.

The API should validate source cardinalities, provenance lengths and bounds,
triangle/source-cell correspondence, and finite barycentric/hit data before
returning a scientific result. Prefer typed errors over panics or silently
omitting invalid source values.

## Explorer behavior

- Enable live hover inspection for imported boundary and isosurface scenes.
- On a hit, show a compact title summary containing representation kind,
  source cell ID, and world position; include a representative scalar field
  value only if doing so does not hide the source identity.
- On a miss, show an explicit miss indication.
- On click, print the complete hit record, including source IDs, triangle
  identity, barycentric weights, world position, and all scalar field values
  with their associations.
- Ray construction errors and missing cursor/zero-size viewport states must
  remain distinct from a geometric miss.
- Keep the existing sampled parametric surface inspection behavior unchanged.
- Keep all existing imported modes, demo modes, camera gestures, resize, and
  minimize/restore behavior unchanged.

## Tests and acceptance

### Core tests

- Ray hit selects the nearest triangle and returns the expected barycentric
  weights and world position.
- Boundary hit resolves compact surface vertices to original VTK point IDs
  and reports the correct source cell, face, local face, and source triangle.
- Point scalar values are interpolated using barycentric weights in the
  triangle's stored vertex order.
- Cell scalar values are read from the exact source cell, including when
  adjacent cells have different values.
- Dataset scalar values are returned without interpolation.
- Non-finite point and cell scalar values remain explicitly classified.
- Isosurface hits resolve source tetrahedron IDs and evaluate point scalar
  fields through source-point/source-edge provenance before barycentric
  interpolation.
- Isosurface extraction-field values are computed from transferred values,
  not hardcoded to the isovalue.
- Independent point fields and cell fields return the correct source values.
- Invalid source IDs, field cardinalities, provenance lengths, and malformed
  hit mappings return typed errors.
- Misses remain misses; no hit is fabricated for rays outside the mesh.

### Explorer tests and Windows acceptance

- Verify imported boundary and isosurface modes enable hover and click
  inspection.
- Verify hover hit/miss title states and click output include correct source
  identity and scalar values.
- Verify camera orbit/pan/zoom still work and camera drags do not trigger
  inspection clicks.
- Verify the existing parametric inspection and all prior demo/VTK modes
  remain unchanged.
- Verify resize, minimize, and restore preserve correct viewport-to-ray
  behavior.
- Run:
  ```powershell
  cargo test -p svf-core
  cargo test -p svf-explorer
  cargo test --workspace
  cargo check --workspace
  ```
- Visually exercise both imported boundary and isosurface workflows using
  `apps/explorer/examples/tetra-with-fields.vtk`. Click known triangle
  locations and compare reported IDs and values with the fixture's declared
  point/cell fields.

Do not claim Windows acceptance until the user reports the commands and
runtime checks. Do not run `cargo fmt` unless separately requested.

## Explicit non-goals

- Vector/tensor inspection, gradients, derivatives, and field statistics.
- Selection highlighting, persistent selected entities, multi-selection,
  annotations, measurement tools, or editing.
- A property inspector panel, field picker, legend, or new UI toolkit.
- GPU ID-buffer picking, asynchronous readback, acceleration structures,
  spatial indexing, or performance benchmarking.
- Picking glyphs, axes, grids, or helper geometry as scientific entities.
- Structured-grid, point-cloud, line, and standalone polygon inspection.
- New file formats, binary/XML VTK, hexahedral isosurface extraction, or
  changes to existing extraction and field-transfer algorithms.

## Dependencies

- M5 ray construction, triangle intersection, and sampled-surface inspection.
- M6.5/M6.6 boundary-surface provenance and triangulation.
- M8.1 tetrahedral isosurface extraction and provenance.
- M8.2 provenance-based scalar transfer.
- M9.1 VTK Dataset import, M9.2 Explorer loading, and M9.3 isosurface mode.
