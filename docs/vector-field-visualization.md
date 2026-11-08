# M7.3 — Vector field visualization

## Status

Design contract recorded on the feature branch. No implementation or local
validation is claimed yet.

## Objective

Add a reusable, graphics-independent pipeline that turns attached
three-component vector fields into spatially meaningful 3D arrow glyphs.
Preserve source field values, association semantics, and provenance. The
renderer draws prepared glyph geometry; it does not interpret vectors or
decide where dataset entities are located. This is the vector-field
counterpart to M7.2 scalar-to-color mapping.

## Architectural pipeline

```text
Dataset + VectorField3
  -> association-aware source locations + provenance
  -> vector validation, magnitude/direction, explicit scale policy
  -> VectorGlyphSet3 (CPU-side arrow geometry + source identity + status)
  -> svf-render (GPU conversion and drawing)
```

- `svf-core` resolves vector values and source locations, validates
  association/cardinality, computes magnitude/direction, applies explicit
  scaling, and generates graphics-independent arrow geometry.
- `svf-render` converts prepared glyph geometry to GPU formats and draws it.
  It does not inspect datasets, fields, associations, or scale policies.
- `svf-explorer` builds deterministic demonstration data and remains a thin
  host.
- Source components remain f64 / `SampleValue`; narrowing to f32 occurs only
  at the renderer boundary. No wgpu types enter `svf-core`.

## Association and source-location semantics

Initial support:

- **Point association:** anchor at each exact source point coordinate and
  preserve its source point ID.
- **Cell association:** anchor at each logical source cell's arithmetic mean
  of source vertices and preserve its source cell ID. Structured grid cells
  are source rectangles; unstructured cells are original cells, not generated
  boundary triangles.

Cell-center rules:
- Point sets have no cells.
- Polyline cells use the midpoint of their endpoints.
- Triangle-mesh cells use the arithmetic mean of their three vertices.
- Structured-grid cells use the arithmetic mean of their four corners.
- Unstructured cells use the arithmetic mean of their source cell vertices.

These are deterministic vertex-average centers, not mass/volume centroids or
interpolation points. This limitation must be explicit.

**Dataset association is deferred:** one dataset-level vector has no intrinsic
spatial anchor. Do not invent an origin or silently place it at world zero.
A later API may accept an explicit caller-provided anchor.

Only `VectorBasis3::World` is supported. Other frames and basis transforms
require a separate design.

## Vector validity and derived representation

For each source vector:
- Preserve the original three classified components in the source field.
- It is renderable only when all components are finite.
- Compute Euclidean magnitude with an overflow-resistant method, not a naïve
  sum of squares.
- Derive direction only for non-zero magnitude.
- A zero vector is valid but directionless; preserve its source identity and
  report a zero-vector status rather than inventing a direction.
- A vector with any non-finite component is not rendered; retain an explicit
  invalid status and source identity.

The CPU-side result preserves per source entity: world anchor, source entity
ID and association, original vector components (or exact f64 equivalent),
magnitude when finite, display status, and generated arrow geometry for
renderable non-zero vectors.

## Arrow geometry and scale policies

The initial glyph is a 3D line arrow: one shaft and two head segments. The
core generates all endpoints in f64 world coordinates. Construct the arrow
head from a deterministic perpendicular basis that remains stable when the
vector is parallel or nearly parallel to any world axis.

Two explicit policies:
1. **Magnitude-scaled:** display length = vector magnitude × caller-supplied
   finite, non-negative scale factor. Relative magnitudes are preserved.
2. **Uniform-length:** every non-zero vector receives a caller-supplied finite,
   positive display length. Direction is emphasized while magnitude
   differences are intentionally suppressed.

Retain policy and parameters as result metadata. No automatic scaling,
normalization, clamping, or maximum-length cap. A zero magnitude-scale factor
is valid but yields no visible arrow while retaining the source entry.
Invalid scale parameters fail explicitly. Arrow-head proportions are a fixed,
documented fraction of display length, tested rather than exposed as numerous
tuning parameters.

## Renderer integration

Add a dedicated prepared-vector-glyph input path without breaking existing
mesh, mapped-surface, or flat-color APIs. The renderer consumes core-generated
line-segment endpoints and a simple fixed display color, preserves order,
converts f64 endpoints to f32 and rejects non-finite narrowed values, and
draws glyphs depth-aware so surfaces correctly occlude them. Vector lookup,
source-location resolution, magnitude/direction, arrow construction, and
scaling remain in `svf-core`.

## Explorer demonstration

Add optional `--vectors`; keep all existing modes unchanged. Use deterministic
point- and cell-associated vector fields, exercising structured and
unstructured geometry where practical. Make direction and relative magnitude
visually understandable. Retain source dataset and derived glyphs on CPU.

## Required tests

Core:
- Exact point anchors and source IDs.
- Correct cell IDs and vertex-average centers for polyline, triangle mesh,
  structured grid, and supported unstructured cells.
- Dataset association rejected clearly; no invented anchor.
- Correct finite magnitude and normalized direction.
- Extreme finite components handled without avoidable overflow; explicitly
  handle a mathematically unrepresentable magnitude.
- Zero and non-finite vectors retain source identity and explicit status.
- Both scale policies produce documented lengths; invalid scales fail.
- Arrow-head orientation stable for axis-parallel and nearly parallel vectors.
- Source geometry, fields, and ordering remain unchanged.

Renderer/Explorer:
- Endpoint conversion preserves segment order and rejects unrepresentable
  coordinates.
- Glyphs render with depth testing without breaking existing paths.
- Verify point/cell glyphs, direction, scale, resize, minimize, and restore
  on Windows.

## Explicit non-goals

Streamlines/pathlines, particle tracing, time-dependent fields, tensor glyphs,
dataset vectors without explicit anchors, non-world basis transforms,
automatic scaling/normalization, adaptive density or decimation, overlap
avoidance, scalar glyph coloring, legends/labels, lighting/materials,
file import/export, and field interpolation/transfer.
