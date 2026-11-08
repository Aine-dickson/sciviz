# M8.2 — Provenance-based scalar transfer onto derived isosurfaces

## Status

Design proposal; implementation has not started. This proposal follows the
completed M8.1 tetrahedral isosurface extractor.

## Objective

Transfer a selected point-associated scalar field from an unchanged source
`Dataset` onto the vertices of an `Isosurface3`, using the isosurface's
source-point and source-edge interpolation provenance.

This enables a derived isosurface to be colored or inspected by a scalar
field other than the field used to extract it. It also establishes a reusable
rule for transferring point data onto this specific derived geometry without
pretending that interpolated vertices are original source points.

## Input contract

- The source dataset is the same immutable dataset used to produce the
  `Isosurface3`.
- The selected field exists on that dataset and is point-associated.
- Its cardinality matches the source dataset point count.
- The `Isosurface3` has one provenance entry per output mesh vertex.
- The transfer operation does not mutate the source dataset, source field,
  isosurface mesh, or isosurface provenance.
- Initial scope is scalar fields only. Vector/tensor transfer is deferred.

The API should make the relationship between source dataset, isosurface,
and selected field explicit. It must validate source cardinality and every
provenance point ID before indexing source values.

## Transfer semantics

For each output vertex, in exact isosurface mesh vertex order:

- `SourcePoint { point_id }`: copy the selected source field value at
  `point_id` without numerical transformation.
- `SourceEdge { point_ids: [low_id, high_id], t }`: interpolate the source
  field values at `low_id` and `high_id` using `t`, where `t` is
  measured from the lower source point ID toward the higher source point ID.

The edge parameter must be finite and within [0, 1]. Invalid provenance is
an explicit error; it must not be silently clamped or reinterpreted.

Transfer is based on source provenance, not nearest-point lookup, geometric
proximity, output vertex order assumptions, or triangle source-cell IDs.

## Numerical and invalid-value policy to settle

The design review must define and test:

- How NaN and positive/negative infinity at source endpoints propagate to
  transferred values.
- Whether a non-finite endpoint invalidates only its corresponding output
  vertex or causes the entire transfer operation to fail.
- A stable interpolation formula for extreme finite endpoint values that
  avoids avoidable intermediate overflow.
- How any genuinely unrepresentable result is classified or reported.
- Whether the result stores `SampleValue` values or a dedicated transferred
  value/status type.
- Whether an empty isosurface produces a valid empty transferred field.

Do not silently replace invalid values with zero, extrapolate, or clamp the
interpolation parameter. The policy must be deterministic and documented.

## Output contract

The result is a derived scalar field aligned one-to-one with the isosurface
mesh vertices. It should retain:

- The selected source field's name and optional units.
- The output values in mesh vertex order.
- The source isosurface vertex provenance, either by a stable reference to
  the `Isosurface3` or by an explicitly documented ownership model.
- Enough identity to associate each transferred value with the corresponding
  derived mesh vertex.

The result is not attached to the source `Dataset` and must not claim that
edge-interpolated values belong to a single source point.

## Color mapping and renderer boundary

The existing scalar visualization pipeline already defines scalar ranges,
color maps, invalid colors, and prepared colored surfaces for source-aware
surface representations. M8.2 should reuse those semantics where possible.

However, the existing `MappedSurface3` source-point mapping must not be
populated with a fabricated point ID for an isosurface edge vertex. The
design must choose an explicit derived-surface representation, such as a
dedicated mapped-isosurface result carrying:
- f64 output positions and triangle connectivity,
- per-corner or per-vertex colors as required by the renderer,
- isosurface vertex provenance,
- per-triangle source cell IDs.

The renderer must consume prepared positions, indices, and colors. It must
not perform field interpolation, inspect datasets, or interpret provenance.
Any f64-to-f32 conversion remains at the renderer boundary.

Whether the first M8.2 implementation includes the renderer adapter and an
Explorer colored-isosurface mode should be decided during design review.
Avoid expanding the increment into a generic rendering/material framework.

## Required tests

- Exact source-point values are copied unchanged.
- Source-edge values interpolate correctly at t=0, t=0.5, and t=1.
- Canonical endpoint ordering is respected.
- Shared isosurface vertices are transferred exactly once and remain aligned
  with mesh vertex order.
- Invalid provenance IDs and invalid t values are rejected.
- Non-finite endpoint policy is tested for NaN and both infinities.
- Extreme finite endpoint values do not create avoidable overflow.
- Empty isosurface behavior is deterministic.
- Source dataset and isosurface remain unchanged.
- If color mapping is included, constant ranges, explicit/automatic ranges,
  invalid colors, and per-triangle provenance are covered.
- Existing M7 scalar mapping and all Explorer modes remain unchanged.

## Explicit non-goals

- Structured-grid or hexahedral isosurface extraction.
- Cell-associated, dataset-associated, vector, or tensor field transfer.
- General dataset-to-dataset interpolation or resampling.
- Nearest-neighbor transfer or spatial search.
- Contour lines, volume rendering, GPU-side interpolation, or time-varying
  field transfer.
- General materials, lighting, or UI legend systems.

## Acceptance criteria

M8.2 is complete when a selected point-associated scalar field can be
transferred deterministically from the source dataset to every derived
isosurface vertex using validated source provenance; numerical and invalid
value behavior is explicit and tested; source data remains unchanged; and
any colored output preserves derived-geometry provenance without fabricating
source point identity. Windows workspace tests and the agreed visual
validation must pass if renderer/Explorer integration is included.
