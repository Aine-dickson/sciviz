# M9.2 — Explorer VTK Dataset Loading

## Status

Design contract for M9.2. This document authorizes implementation only after
the design PR is reviewed and merged. M9.2 builds on the pure parser delivered
in M9.1; it does not change the accepted VTK grammar.

## Objective

Allow a user to load a VTK legacy ASCII unstructured-grid file into the native
SVF Explorer and inspect its geometry, with an optional scalar field mapped
onto the rendered surface.

The imported `Dataset` remains the authoritative scientific object. Explorer
coordinates file access and user-facing errors; `svf-core` continues to own
parsing, topology, provenance, and scalar mapping; `svf-render` receives only
prepared render geometry.

## User workflow and CLI

Use an explicit command-line path rather than adding a native file-dialog
dependency in this increment:

```text
cargo run -p svf-explorer -- --vtk <path>
cargo run -p svf-explorer -- --vtk <path> --vtk-scalar <field-name>
```

- `--vtk` consumes exactly one following path argument. Paths containing
  spaces are supported when quoted by the invoking shell.
- `--vtk-scalar` consumes exactly one following field name and is valid only
  with `--vtk`.
- With no scalar option, render the imported dataset's uncolored surface.
- With `--vtk-scalar`, require a scalar field with that exact name and map it
  using the existing Viridis color map and automatic finite range.
- Existing demo flags and the default parametric mode retain their behavior.
- A VTK load request combined with any demo-mode flag is an argument error;
  do not silently pick a precedence. Duplicate options, missing values, and
  unknown options in the new VTK invocation are errors.
- Existing invocations that use only current demo flags remain accepted.

A native file picker, interactive reload, and file watching are not part of
M9.2. The CLI is a deliberate low-dependency first user-facing loading path.

## Loading and error boundary

1. Parse the command line into a validated Explorer startup configuration
   before creating the event loop or window.
2. Read the requested path as UTF-8 text using standard filesystem APIs.
   Do not infer format from the extension.
3. Pass the complete text to
   `svf_core::vtk_legacy::parse_legacy_ascii_unstructured_grid`.
4. Preserve the source `Dataset` in Explorer for the full lifetime of the
   loaded scene. Do not reconstruct scientific data from GPU buffers.
5. Convert filesystem, parser, geometry, mapping, and renderer failures into
   contextual startup diagnostics. Include the path for file I/O and import
   errors; preserve the parser's useful error details.
6. On failure, print a clear diagnostic and exit non-zero before creating a
   graphics window. Do not panic, silently fall back to a demo, or create a
   partially initialized scene.

No file I/O, command-line parsing, windowing, or renderer dependency is added
to `svf-core`. No new dependency is expected for this CLI-based increment.

## Initial renderable representation

The current renderer accepts indexed triangle meshes. M9.2 therefore defines
one explicit initial representation for imported unstructured datasets:

- For 3D volume cells (tetrahedra and hexahedra), extract the exterior
  boundary with the existing `boundary_surface()` and triangulate it with
  the existing deterministic triangulation pipeline.
- Preserve the source `Dataset` and derived surface/provenance objects on
  the CPU for the lifetime of the scene.
- If the input contains a mixture of 3D and lower-dimensional cells, render
  the volume boundary and report how many lower-dimensional cells are not
  represented in this initial surface view. Never imply those cells were
  rendered.
- If there are no 3D volume cells, fail with a clear “no renderable volume
  boundary” diagnostic. The importer still accepts line and surface cells;
  their dedicated rendering representations are deferred.
- If the volume boundary is empty (for example, because the dataset has no
  volume cells), do not pass an empty mesh to the renderer as if loading
  succeeded.

This is intentionally not a general mixed-dimensional renderer. Direct
rendering of line cells, surface cells, points, and explicit mixed
representations requires a separate representation design.

## Optional scalar coloring

When `--vtk-scalar FIELD` is supplied:

- Resolve the named scalar field from the retained imported `Dataset`.
  Missing names and vector fields produce distinct, actionable errors.
- Accept the imported point- or cell-associated scalar fields supported by
  M9.1. Reject dataset association if encountered; do not invent a mapping.
- Build `SurfaceProvenance3` from the triangulated volume boundary and
  original `UnstructuredGrid3`.
- Reuse `map_scalar_field` with `ScalarRangeMode::Automatic`,
  `ColorMap::viridis()`, and the existing invalid-value color.
- Preserve source field values and association. Do not mutate the Dataset,
  bake colors into it, or replace it with a derived mesh.
- If the field has no finite values and automatic range construction fails,
  surface that mapping error clearly.

Without `--vtk-scalar`, do not choose a field automatically. Listing fields
in startup diagnostics/window title is useful but not required for this
increment.

Vector glyph display, field selection UI, custom color maps/ranges, isosurface
extraction from imported data, and field transfer to derived isosurfaces are
deferred.

## Camera framing

Imported files may use arbitrary world-space position and scale. Reuse the
existing f64 camera model and `SceneRenderer::set_camera()`; do not change
the global default camera or existing demo camera behavior.

- Compute finite axis-aligned bounds from the actual triangulated render mesh.
- Aim the camera at the bounds center.
- Choose a perspective distance from the bounds' bounding-sphere radius and
  the existing vertical field of view so the entire mesh fits with a modest
  margin.
- Derive positive near/far planes from the fitted distance and radius.
- Handle zero-extent axes and a single-point/degenerate radius defensively;
  do not create non-finite camera inputs.
- Keep the existing +Y up, right-handed camera convention.
- Camera orbit, pan, zoom, resize, and minimize/restore behavior must remain
  unchanged after fitting.

If bounds cannot produce a valid camera configuration, return a contextual
load/scene error rather than displaying a clipped or invalid scene.

## Explorer state and rendering

- Add one imported-data mode/configuration; do not add another hard-coded
  demo mode for each file.
- Keep the imported `Dataset`, `TriangulatedSurface3`, and optional
  `MappedSurface3` alive in Explorer state.
- Uncolored data uses `SceneRenderer::new`; scalar-colored data uses
  `SceneRenderer::new_with_mapped_surface`.
- The existing renderer remains unaware of file paths, VTK syntax, and
  Dataset ownership.
- Existing demo modes, keyboard behavior, camera navigation, resize handling,
  and default parametric launch must remain unchanged.

## Tests and acceptance

### CLI/configuration tests

- Parse `--vtk path` and `--vtk path --vtk-scalar name`.
- Preserve quoted paths containing spaces as a single path.
- Reject missing path/field values, duplicate VTK options, scalar option
  without VTK input, unknown options in VTK mode, and conflicting demo flags.
- Verify all existing demo flags still select the same modes and default
  remains parametric.

### Loading and representation tests

- Load a representative M9.1 fixture through the Explorer loading function.
- Verify source point/cell counts and fields remain available in the retained
  Dataset.
- Verify mixed volume/lower-dimensional input reports omitted lower-dimensional
  cells and renders the volume boundary only.
- Reject valid line/surface-only datasets with the documented diagnostic.
- Verify missing scalar name, vector field name, and all-invalid scalar field
  fail clearly.
- Verify point- and cell-associated scalar fields map to the derived surface
  with correct source provenance and expected color count.
- Verify camera fitting centers the bounds, produces finite valid projection
  parameters, and handles zero-extent dimensions.
- Keep GPU-independent tests separate from Windows runtime validation.

### User Windows acceptance

- `cargo test -p svf-explorer`
- `cargo test --workspace`
- `cargo check --workspace`
- Run one imported file without scalar coloring.
- Run the same file with point-associated scalar coloring.
- Run a file with cell-associated scalar coloring.
- Verify an invalid path and invalid VTK file produce clear non-zero startup
  failures without opening a graphics window.
- Verify camera framing, orbit, pan, zoom, resize, minimize, and restore.
- Verify every existing demo mode and default launch still behave as before.

No `cargo fmt` is required unless separately requested. Do not claim local
Windows acceptance until the user reports the results.

## Explicit non-goals

- Native file dialogs, interactive reload, file watching, and drag/drop.
- Binary legacy VTK, XML VTK, VTKHDF, or format auto-detection.
- New VTK syntax, data types, cell types, or field semantics beyond M9.1.
- Rendering line cells, standalone surface cells, points, or general
  mixed-dimensional datasets.
- Vector glyph selection/rendering, isosurfaces, or derived-field transfer
  from imported datasets.
- GUI field browser, scalar range/color-map controls, legends, and metadata
  inspector.
- Multiple simultaneous datasets, time series, streaming, and parallel files.
- New graphics dependencies or VTK runtime dependencies.

## Dependency on M9.1

M9.2 consumes the merged M9.1 API and its structured error types unchanged.
Any parser behavior change requires a separate core-level justification and
tests; do not weaken the M9.1 parser contract to accommodate Explorer.
