# M7.4 — Structured cell-associated vector visualization

## Status

Design contract and Explorer implementation merged to `main` through PR #11
on 2026-11-08 (merge commit `082fc5e18db8a7e40e1655dff70950af2fcc1e03`).
User-reported Windows workspace tests passed.

## Objective

Add a separate Explorer demonstration of cell-associated vector fields on a
structured graph surface. This closes the practical demonstration gap left by
M7.3: the core already resolves structured-cell vector anchors, but the
`--vectors` Explorer mode currently demonstrates point-associated vectors
only.

The demonstration must make cell association and source-cell placement
observable without changing the existing point-vector mode or weakening
depth testing.

## Data and spatial semantics

- Use a `StructuredGrid2D` as the source dataset geometry.
- Attach a `VectorField3` with cell association and world basis.
- Generate the displayed graph surface from that same structured grid. Do not
  build the glyph dataset and surface from independently sampled grids.
- Each glyph anchor is the arithmetic mean of the four source graph-surface
  corner positions for its logical structured cell.
- Preserve the source cell ID and original vector components through the
  core glyph result.
- These anchors are deterministic vertex-average centers, not area-weighted
  centroids or interpolation points.
- Use finite, nonzero deterministic vector values with visibly varying
  directions. Use an explicit scale policy and parameter; no automatic
  scaling or normalization.
- Keep the sample grid sparse enough that neighboring glyphs remain
  distinguishable. Choose the uniform glyph length relative to cell spacing,
  not by adding an implicit automatic scale feature.

## Explorer behavior

- Add an optional `--cell-vectors` mode.
- Keep `--vectors`, `--scalar`, `--unstructured-scalar`,
  `--unstructured`, and the default mode unchanged.
- Retain the source dataset and `VectorGlyphSet3` on the CPU.
- Reuse the existing prepared-glyph renderer input path.
- Do not introduce cell/vector interpretation into `svf-render`.
- Preserve existing surface hover behavior where the current host supports it.

## Visual readability and depth

M7.3 visual feedback: the yellow glyphs render correctly, but some are hard to
distinguish against the surface because the glyphs are short and densely
distributed. M7.4 should use a deliberately sparse cell grid and a fixed,
explicit glyph length that makes shaft and arrowhead orientation legible.

Keep glyphs depth-aware. Surface occlusion is correct behavior; do not draw
all glyphs unconditionally on top of the surface or disable depth testing to
make them more visible. If a glyph is hidden by the surface, treat that as a
visibility consequence of its world-space anchor and camera, not as a reason
to falsify its position.

## Implementation in PR #11

- Added `--cell-vectors` as a separate Explorer mode.
- Builds an 9 × 9 structured graph grid (64 logical cells) over [-4, 4]².
- Uses the same `StructuredGrid2D` instance as the dataset geometry and the
  generated display surface.
- Attaches 64 deterministic cell-associated world-basis velocity vectors.
- Resolves the vectors through the existing core source-location pipeline and
  generates 1.0-world-unit glyphs through the existing explicit uniform policy.
- Retains the source dataset and glyph set on the CPU.
- Adds tests for grid/field cardinality, cell source ordering, arithmetic
  four-corner anchors, explicit display length, and CLI mode selection.

Implementation and automated Windows workspace tests are complete.
User-reported visual validation confirmed the structured glyph pattern,
visible arrowheads, grid/axes, and consistent resize/minimize/restore behavior.
Many glyphs are still small/crowded. Depth occlusion could not be conclusively
assessed from the fixed camera because Explorer has no orbit/pan/zoom controls.
The multiple-angle occlusion check is deferred to M7.5.

## Required tests

Core/API integration:
- The demo dataset has the expected structured-grid dimensions and logical
  cell count.
- The attached vector field has cell association and matching cardinality.
- Resolved source entities are cell IDs in stable source order.
- Resolved anchors equal the arithmetic means of the four source graph-surface
  corner positions.
- Every finite nonzero source vector produces one glyph entry with the
  expected explicit display length.
- Source dataset geometry and field values remain unchanged.

Explorer:
- `--cell-vectors` selects the new mode.
- The demo builds the expected dataset and glyph count.
- Existing command-line modes retain their previous selection behavior.

Windows visual validation:
- Glyphs appear at structured-cell centers on the graph surface.
- Direction and arrowheads are distinguishable across the surface.
- [deferred to M7.5] Inspect surface depth occlusion from multiple camera angles.
- Existing grid and colored axes remain visible.
- Resize, minimize, and restore remain consistent.
- Re-run `--vectors` and the default mode to confirm no regressions.

## Explicit non-goals

- Unstructured volume-cell glyph rendering. Arithmetic cell centers for
  tetrahedra/hexahedra lie inside their volume; an opaque boundary surface can
  correctly occlude those glyphs. A useful volume-cell visualization needs a
  separate representation/visibility design, not a depth-test bypass.
- Streamlines, pathlines, particle tracing, or temporal vector fields.
- Automatic glyph scaling, density adaptation, overlap avoidance, or
  interpolation.
- Per-vector color maps, legends, labels, lighting, or material systems.
- New renderer-side vector semantics or GPU-authoritative field data.
