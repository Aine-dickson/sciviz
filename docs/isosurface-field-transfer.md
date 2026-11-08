# M8.2 — Provenance-based scalar transfer onto derived isosurfaces

## Status

Design contract finalized; implementation has not started. This increment
builds on the merged M8.1 tetrahedral isosurface extractor.

## Objective

Transfer a selected point-associated scalar field from the unchanged source
dataset onto the vertices of an extracted `Isosurface3`, using the
isosurface's source-point and source-edge interpolation provenance. Map the
transferred values to colors and render the derived surface without inventing
source point IDs or mutating source data.

M8.2 is a vertical slice: core transfer and mapping, renderer adaptation, and
a separate Explorer demonstration.

## Scope and input contract

- Input geometry is `DatasetGeometry::UnstructuredGrid3`, and the selected
  field is a scalar field with `FieldAssociation::Point`.
- The caller must pass the exact source `Dataset` from which the
  `Isosurface3` was extracted. M8.1 does not carry a globally unique dataset
  identity, so the transfer API can validate cardinalities and provenance
  bounds but cannot prove that two same-sized datasets are identical.
- Validate source field cardinality against the source dataset point and cell
  counts before reading values.
- Validate that the isosurface has one provenance entry per mesh vertex and
  one source-cell ID per mesh triangle.
- Validate every source point ID, both source edge endpoint IDs, canonical
  edge ordering, and interpolation parameter before indexing.
- Transfer and mapping never mutate the source dataset, source field, or
  original `Isosurface3`.

Initial support is limited to point-associated scalar fields. Cell-associated
transfer is deliberately excluded: an edge vertex shared by multiple source
cells does not have a unique cell value without an explicit discontinuity
policy.

## Transfer semantics

Produce one value for each isosurface mesh vertex, in exact mesh vertex order.

- `SourcePoint { point_id }`: copy the source field's `SampleValue`
  exactly, including NaN and signed infinity classifications.
- `SourceEdge { point_ids: [low_id, high_id], t }`: interpolate the values
  at those endpoint IDs. M8.1 defines `t` from the lower source point ID
  toward the higher ID.
- Require `low_id < high_id`; require finite `t` in the closed interval
  [0, 1]. Reject malformed provenance; never reorder endpoints or clamp
  `t`.
- At `t == 0` or `t == 1`, return the corresponding endpoint
  `SampleValue` exactly. This avoids contaminating an exact endpoint with
  the other endpoint's invalid value.

### Non-finite endpoint policy

For interior interpolation (`0 < t < 1`), preserve IEEE-style convex
combination behavior in the existing `SampleValue` model:

- If either endpoint is NaN, the result is NaN.
- One infinite endpoint and one finite endpoint produce that infinity.
- Two infinities with the same sign produce that infinity.
- Opposing positive and negative infinities produce NaN.
- Finite endpoints produce a finite interpolated value.

A non-finite transferred value invalidates only that output vertex's scalar
sample. It does not fail the whole transfer operation and is not replaced
with zero. The color-mapping stage assigns the configured invalid color to
that vertex.

### Stable finite interpolation

For finite endpoints, use a convex interpolation that avoids avoidable
overflow:

- If both endpoints have the same sign, compute
  `a + (b - a) * t`. The endpoint difference is bounded for same-sign
  finite values.
- If endpoints have opposite signs, compute
  `a * (1 - t) + b * t`. The weighted terms are finite and oppose rather
  than add in magnitude.
- Handle `t == 0` and `t == 1` before either formula.
- Classify the result through `SampleValue::from_f64`. If finite inputs
  nevertheless produce a non-finite result, return an explicit numerical
  error; do not silently turn arithmetic failure into a data-invalid sample.

A convex combination of finite endpoints is mathematically bounded by those
endpoints, so overflow should not occur with the prescribed branch-specific
formulas. Underflow and signed zero remain valid finite results.

## Core API and ownership

Add a focused `svf-core::isosurface_field_transfer` module.

- `transfer_point_scalar_field(dataset, isosurface, field_name)`
  returns a `TransferredScalarField3`.
- The transferred object owns a clone of the source `Isosurface3`, the
  source field's `FieldMetadata`, and one `SampleValue` per isosurface
  mesh vertex. Owning the isosurface ties values to the exact mesh order and
  retains vertex provenance, triangle source-cell IDs, and extraction
  metadata together.
- Expose read-only accessors. Do not expose a constructor that can create
  mismatched mesh/value/provenance cardinalities.
- A separate `map_transferred_scalar_field` operation consumes the
  transferred object and returns `MappedIsosurface3`, retaining the
  transferred object plus one RGBA color per unique mesh vertex and the
  resolved scalar range.

The derived field is not attached to the source `Dataset`. It must never
claim that an edge-interpolated value belongs to one source point.

## Color mapping contract

Reuse `ColorMap`, `ColorRgba`, `ScalarRange`, and
`ScalarRangeMode` from `scalar_visualization`.

- Automatic range is computed from finite transferred values on the
  extracted isosurface, not all values in the source dataset. Add
  `ScalarRange::from_values` for this purpose and have
  `ScalarRange::from_field` delegate to the shared implementation.
- Explicit ranges retain existing finite-endpoint validation and clamping
  behavior.
- A constant finite transferred range maps finite values to the existing
  midpoint normalization (0.5).
- NaN and infinities receive the caller-supplied invalid color.
- If automatic range has no finite transferred values, return the existing
  `ScalarRangeError::NoFiniteValues`. An explicit range can still map an
  all-invalid or empty result, assigning invalid colors where vertices exist.
- Colors are stored once per unique isosurface vertex. Keep the original
  indexed triangle topology; do not expand vertices or fabricate source
  point IDs.
- Preserve the full `Isosurface3` inside the mapped result so source vertex
  provenance, per-triangle source cell IDs, and extraction metadata remain
  available.

The existing `MappedSurface3` remains unchanged. Its
`source_point_ids` represent actual source points and must not be overloaded
with interpolated edge vertices.

## Renderer and Explorer integration

- Add a renderer entry point accepting `MappedIsosurface3`.
- Renderer conversion reads prepared f64 mesh positions and RGBA colors,
  converts them to f32 at the GPU boundary, and reuses the mesh indices.
- Validate position/color cardinality, index bounds, and finite f32
  representability. The renderer does not inspect the source dataset,
  interpolate fields, or interpret provenance.
- Add a separate `--colored-isosurface` Explorer mode. Keep the existing
  `--isosurface` behavior and all earlier modes unchanged.
- Extend the deterministic two-tetrahedron fixture with a second
  point-associated scalar field distinct from the extraction field. Extract
  using the existing signed-distance field, transfer the second field, map
  it to colors, and render it through the new renderer entry point.
- Keep source dataset, isosurface, transferred values, and provenance alive
  on the CPU side in Explorer.

## Required tests

Core transfer:
- Source-point values copy exactly for finite, NaN, positive infinity, and
  negative infinity samples.
- Edge interpolation covers t=0, t=1, and interior values, including
  same-sign, opposite-sign, zero, and extreme finite endpoints.
- Interior non-finite combinations cover NaN, one infinity, same-sign
  infinities, and opposing infinities.
- Invalid field type/name, non-point association, and cardinality mismatch
  return explicit errors.
- Invalid source IDs, noncanonical edge IDs, invalid t, provenance count
  mismatch, and invalid source-cell IDs are rejected before indexing.
- Empty isosurfaces produce valid empty transferred fields.
- The source dataset and input isosurface remain unchanged.

Color mapping:
- Automatic range uses finite transferred values only.
- Constant and explicit ranges retain existing semantics.
- Invalid values receive the exact supplied invalid color.
- All-invalid automatic range returns `NoFiniteValues`; explicit range
  supports all-invalid and empty results.
- One color is retained per unique isosurface vertex and triangle indices
  remain unchanged.
- Mapped output retains the complete isosurface and all provenance.

Renderer/Explorer:
- Renderer conversion preserves positions, colors, and indices.
- Reject f64 positions or colors that become non-finite in f32.
- Verify `--colored-isosurface` selects the new mode and renders the
  transferred field.
- Existing `--isosurface`, all prior Explorer modes, camera navigation,
  picking/hover, resize, minimize, and restore remain unchanged.

## Explicit non-goals

- Structured-grid or hexahedral isosurface extraction.
- Cell-, dataset-, vector-, or tensor-associated transfer.
- General dataset-to-dataset interpolation or resampling.
- Nearest-neighbor transfer or spatial search.
- Contour lines, volume rendering, GPU-side interpolation, or time-varying
  field transfer.
- General materials, lighting, legends, or a general rendering framework.

## Acceptance criteria

M8.2 is complete when point-associated scalar values transfer deterministically
through validated M8.1 provenance, including explicit non-finite and
numerical behavior; mapped colors preserve the original derived mesh and
provenance without fabricated source point IDs; the new renderer and
`--colored-isosurface` path work; and Windows workspace tests plus the
agreed visual and regression checks pass.
