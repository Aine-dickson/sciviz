# M7.2 — Reusable scalar visualization

## Status

Design contract and core scalar-mapping stages are implemented. The user
validated the core on Windows: `cargo test -p svf-core` passed (163 tests)
and `cargo check --workspace` passed without warnings. Renderer RGBA input
integration is now implemented on the feature branch and awaits Windows
validation. Explorer scalar demonstration modes are still pending.

Branch: `m7-2-scalar-visualization`, based on `main` after M7.1.

## Objective

Add a reusable, graphics-independent scalar-to-color mapping pipeline that
works with surface representations derived from both structured and
unstructured datasets. The same mapping rules and output contract must apply
regardless of how the source geometry is stored.

M7.2 is the first end-to-end use of the dataset and field model introduced in
M6. Scientific values and source associations remain authoritative in
`svf-core`; mapped colors and render meshes are derived outputs.

## Architectural pipeline

```text
Dataset + ScalarField
        |
        v
Surface representation + source provenance
        |
        v
Scalar range + color map
        |
        v
MappedSurface3 (positions + per-corner RGBA + provenance)
        |
        v
svf-render (GPU conversion and drawing)
```

Responsibilities:

- `svf-core` validates field/geometry compatibility, resolves source values
  through provenance, computes scalar ranges, maps values to colors, and
  creates a graphics-independent mapped surface.
- `svf-render` accepts the mapped surface's positions, triangle connectivity,
  and colors. It does not inspect `Dataset`, `ScalarField`, field
  associations, or compute scalar ranges.
- `svf-explorer` constructs demonstration datasets and selects a field and
  mapping configuration. It remains a thin host.

No wgpu types enter `svf-core`. Scientific source values remain f64 /
`SampleValue`; conversion of positions and color attributes to GPU formats
occurs at the renderer boundary.

## Reusable surface provenance contract

A common surface adapter must provide:

- A triangle mesh in render-vertex order.
- One source point ID for each render vertex.
- One source logical cell ID for each render triangle.

The mapping arrays must be validated against the mesh and the source
`Dataset`. Source IDs must be in range. This contract is independent of the
source geometry representation.

Initial adapters:

1. **Structured grid:** adapt `SampledSurface3`. Convert each
   `GridSampleIndex2D(x, y)` to the dataset's row-major point ID
   `y * x_sample_count + x`. Convert each `GridCellIndex2D(x, y)` to the
   row-major source rectangle ID `y * (x_sample_count - 1) + x`. Both
   generated triangles from one rectangle therefore resolve to the same
   logical source cell.
2. **Unstructured volume:** adapt `TriangulatedSurface3`. Its compact points
   map through `source_point_ids()`; each triangle maps through
   `source_cell_index()`.
3. **Triangle mesh dataset:** use identity point IDs and triangle IDs.

This is a surface-representation contract, not a general remapping system.
Future filters that create interpolated points must provide explicit source
weights or another declared transfer rule; M7.2 must not guess interpolation.

## Supported field associations

M7.2 supports all three existing `FieldAssociation` variants:

- **Point:** each surface render vertex looks up the scalar value using its
  source point ID. This works for structured samples and unstructured source
  points without assuming their storage layouts are identical.
- **Cell:** each surface triangle looks up the scalar value using its source
  logical cell ID. Every corner of that triangle receives the same color.
- **Dataset:** the single dataset-associated value colors the entire surface.

The source field's original order and values are never reordered or mutated.
The mapping output retains source point/cell provenance so later inspection
can recover the scientific entity represented by a rendered primitive.

## Cell-associated rendering and vertex expansion

The current indexed renderer stores color per vertex. A shared indexed vertex
cannot carry two different cell colors when adjacent cells have different
values. Therefore the mapped surface must expand triangle corners (three
render vertices per output triangle) or use an equivalently explicit
per-primitive mechanism.

M7.2 will use triangle-corner expansion for a single predictable path:

- Each output triangle has three output vertices and sequential triangle
  indices.
- Point-associated colors are looked up independently for each corner.
- Cell-associated colors are repeated identically at all three corners.
- Dataset-associated color is repeated for every corner.
- Source point ID is retained for each expanded corner; source cell ID is
  retained for each output triangle.
- Source geometry and topology are not modified.

This avoids color bleeding across cell boundaries and makes the same mapped
surface representation valid for all three associations.

## Scalar range semantics

Provide two range modes:

- **Automatic:** compute minimum and maximum from all finite values in the
  source scalar field, not only values that happen to appear on the visible
  surface. This keeps the scale stable when different representations expose
  different subsets of the same dataset.
- **Explicit:** caller supplies a finite minimum and maximum with
  minimum strictly less than maximum.

Rules:

- Automatic range ignores non-finite entries when finding extrema.
- Automatic range fails explicitly if the field contains no finite values.
- A constant finite field (minimum equals maximum) is valid. Its values map
  to the midpoint of the color map; no division by zero is permitted.
- Explicit-range values below or above the range are clamped to the nearest
  endpoint in this increment.
- Non-finite field entries map to a separately configured invalid-value color.
  They are not dropped, reindexed, or silently treated as zero.
- An explicit range may still be used when every field entry is non-finite;
  all rendered values then use the invalid-value color.

Logarithmic, symmetric-logarithmic, percentile, per-visible-surface, and
time-varying range policies are deferred.

## Color-map semantics

Provide a reusable continuous color map defined by ordered normalized stops:

- Stop positions are finite and lie in [0, 1].
- Stops are strictly increasing and include both 0 and 1.
- Each stop contains a validated RGBA color.
- Colors between adjacent stops are interpolated deterministically.
- A built-in Viridis-style preset is the initial default; custom stops remain
  supported so the API is not tied to one palette.
- The color channel/color-space convention must be explicit and consistent
  through interpolation and the renderer output path. Do not leave sRGB versus
  linear-light interpretation implicit.
- Invalid-value color is independent of the finite-value palette.

The mapped result exposes the resolved scalar range and color-map metadata.
That is sufficient for a later legend/color-bar component to use the exact
same scale. On-screen legend labels and typography are not part of M7.2
because the current renderer has no text/overlay subsystem.

## Renderer integration

Add a colored-surface input path without breaking the existing
`TriangleMesh3` API or M7.1 flat-color mode.

The renderer must:

- Preserve triangle order and positions.
- Upload per-vertex RGBA attributes alongside positions.
- Use the existing indexed triangle/depth pipeline where practical.
- Convert f64 positions and normalized colors to f32 at the GPU boundary.
- Reject non-finite or unrepresentable GPU attributes explicitly.
- Keep scalar range calculation, palette interpolation, and field lookup out
  of the renderer.

## Explorer demonstration

Keep the existing default parametric demo unchanged.

Provide two demonstrations of the same scalar-mapping API:

- A structured-grid surface with a point-associated scalar field.
- The existing unstructured hexahedron/tetrahedron surface with a
  cell-associated scalar field (and automated coverage for point and dataset
  association on unstructured data as well).

The existing `--unstructured` flat-color demonstration should remain
available. Scalar visualization is an additional mode/option, not a
replacement for M7.1 behavior.

## Required tests

### Core mapping

- Automatic range from finite values while ignoring NaN and infinities.
- Explicit range validation and endpoint clamping.
- Constant-field midpoint behavior.
- Empty/all-non-finite automatic-range failure.
- Invalid-value color behavior with explicit range.
- Color-stop ordering, endpoint coverage, validation, and interpolation.
- Point, cell, and dataset association lookup.
- Source IDs and triangle provenance remain correct after corner expansion.
- Invalid point/cell source IDs and mapping cardinality mismatches fail
  explicitly.
- Mapping does not mutate source geometry or scalar field values.

### Representation integration

- Structured-grid sample IDs and source rectangle IDs resolve correctly.
- Unstructured triangulated-surface point IDs and source cell IDs resolve
  correctly.
- TriangleMesh3 identity mapping resolves correctly.
- Adjacent cells with different cell values do not interpolate/bleed colors
  across their shared geometric edge.

### Renderer and Explorer

- Colored vertex conversion preserves geometry, indices, and RGBA values.
- Invalid GPU-bound attributes are rejected.
- Existing uncolored rendering and M7.1 behavior remain intact.
- Visually verify a structured point field and unstructured cell field,
  including resize/minimize/restore.

## Explicit non-goals

- Vector glyphs, streamlines, tensor visualization, contours, isosurfaces,
  volume rendering, or time-dependent fields.
- Field interpolation or transfer between unrelated datasets.
- Automatic interpolation for generated points without explicit provenance.
- GPU-side scalar lookup or transfer-function evaluation.
- Logarithmic/percentile range policies.
- Material/lighting systems or multiple simultaneous scalar overlays.
- On-screen text legend, general UI overlays, or a scene editor.
- New source cell types, file import/export, or mesh repair.

## Acceptance criteria

M7.2 is complete when one reusable core mapping path colors both structured
and unstructured surface representations, supports point/cell/dataset
associations with the declared provenance semantics, preserves source data,
renders per-corner colors without cell-boundary bleeding, passes focused
Windows workspace tests, and is visually validated in both demonstrations.
