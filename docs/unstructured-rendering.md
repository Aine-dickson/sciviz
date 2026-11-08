# M7.1 — Unstructured boundary-surface rendering

Status: design contract

## Objective

Demonstrate the existing unstructured-volume pipeline end to end by rendering
a boundary surface derived from `UnstructuredGrid3` in the native Explorer.

The source grid remains the authoritative scientific geometry. Boundary
extraction and triangulation remain graphics-independent operations in
`svf-core`. The renderer receives only a derived `TriangleMesh3`.

## Pipeline

`UnstructuredGrid3`
→ `volume_topology()`
→ `boundary_surface()`
→ `triangulate()`
→ `TriangleMesh3` render adapter
→ `SceneRenderer`

The adapter copies the triangulated surface's compact f64 points and flattens
its triangle vertex triplets into the existing indexed-mesh representation.
It must not reorder points or triangles. The `TriangulatedSurface3` remains
available on the CPU in Explorer, including its source-point and
source-face/cell provenance. Provenance is not uploaded as GPU data.

## Demonstration fixture

Add an optional unstructured-volume mode to the existing Explorer, selected
with:

```powershell
cargo run -p svf-explorer -- --unstructured
```

Keep the existing parametric-surface mode as the default, preserving its
amplitude controls and sampled-surface inspection behavior.

The fixture contains two disconnected, non-degenerate linear volume cells:
- one `Hexahedron8` with eight points;
- one `Tetrahedron4` with four points.

Use coordinates of a few world units around the origin so both components
are visible with the existing reference camera. No shared points or faces are
required for this first rendering integration.

Expected derived counts:
- 2 source volume cells;
- 10 boundary polygons (6 quads and 4 triangles);
- 12 compact boundary points;
- 16 output triangles (12 from the hexahedron and 4 from the tetrahedron).

The fixture and expected counts are deterministic and covered by tests.

## Rendering contract

- Reuse the existing `SceneRenderer` indexed triangle pipeline.
- Reuse the existing f64-to-f32 conversion boundary and depth target.
- Keep the current unlit fixed-color mesh presentation.
- Do not enable back-face culling. Existing no-cull behavior is required
  because source cell orientation and extracted local face cycles do not
  certify outward orientation.
- Do not add normals, lighting, materials, camera controls, or a new renderer
  abstraction in this increment.
- Keep source geometry, triangulated geometry, and provenance on the CPU.
  GPU buffers are disposable render representations only.
- The optional mode must not remove or regress the default parametric demo,
  amplitude adjustment, or sampled-surface picking.

## Explorer behavior

- Parse the exact `--unstructured` argument to select the new mode.
- Build the source grid, boundary surface, and triangulated surface before
  renderer initialization.
- Retain the `TriangulatedSurface3` in Explorer for source identity and
  future scientific picking.
- Construct a `TriangleMesh3` for the renderer without losing or rewriting
  the retained provenance.
- Give the window a mode-appropriate title.
- Do not interpret amplitude keys as meaningful in unstructured mode.
- Do not claim unstructured scientific picking in this increment.

## Tests

Add focused Explorer-side tests for:
1. Fixture construction and source cell ordering/types.
2. Boundary extraction counts and polygon arities.
3. Triangulation counts and per-source-cell triangle counts.
4. Complete provenance: each generated triangle maps to a valid boundary
   face, original topology face, source cell, local face, and triangle ordinal.
5. Render-mesh conversion preserves f64 point order and flattened triangle
   connectivity exactly.
6. All generated indices are in range and all fixture coordinates are finite.

Existing `svf-core` tests remain the authority for topology, extraction, and
triangulation semantics. Existing `svf-render` tests remain the authority
for f64-to-f32 conversion and GPU-facing mesh representation.

## Manual Windows verification

- Run `cargo run -p svf-explorer -- --unstructured`.
- Confirm both the hexahedron and tetrahedron exterior surfaces are visible.
- Confirm the reference axes/grid remain visible and depth testing behaves
  correctly.
- Resize, minimize, restore, and close the window.
- Run the default `cargo run -p svf-explorer` and confirm the existing
  parametric surface, amplitude controls, and sampled-surface inspection
  still work.

Automated validation:
- `cargo check --workspace`
- `cargo test -p svf-core`
- `cargo test --workspace`

Do not run formatting routines as part of this increment. Report formatting
status as not run.

## Explicit exclusions

- Scalar/vector field coloring or transfer to derived triangles.
- Unstructured surface picking and source-cell inspection UI.
- Normals, lighting, materials, transparency, or advanced shading.
- Orientation repair or outward-normal certification.
- Geometric quality filtering, mesh repair, or volume rendering.
- File import/export, adaptive tessellation, and new cell types.
- General scene graphs, renderer replacement, or camera-control systems.

## Completion criteria

M7.1 is complete when the optional mode renders both fixture cell types on
Windows, the default Explorer workflow remains functional, the focused tests
and workspace checks pass, and the handoff/milestone documentation records
the exact user-reported results.
