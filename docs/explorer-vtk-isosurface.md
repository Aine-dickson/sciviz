# M9.3 — Isosurfaces from Imported VTK Datasets

## Status

Design contract only. This document authorizes implementation only after
this design PR is reviewed and merged.

## Objective

Allow users to extract and inspect a tetrahedral isosurface directly from a
VTK legacy ASCII unstructured-grid file loaded by M9.2.

This connects three already implemented capabilities into one practical
scientific workflow:

`VTK file → Dataset + point scalar field → tetrahedral isosurface extraction
→ TriangleMesh3 → existing renderer`.

The source `Dataset` remains authoritative. The existing M8.1 extraction
algorithm and its provenance are reused unchanged. M9.3 is an Explorer
integration increment, not a new contouring algorithm or a new file format.

## Rationale and scope choice

SVF's target workflows include scalar quantities such as temperature,
pressure, and density, with contours/isosurfaces as a central way to inspect
3D fields. M9.1 already imports point-associated scalar fields from VTK;
M8.1 already extracts tetrahedral isosurfaces; M9.2 already loads imported
datasets into Explorer. The missing link is applying the existing extraction
pipeline to an imported field.

This is a more evidence-based next step than adding another file format
without a representative target file or a demonstrated format requirement.
Additional formats (including VTK XML, binary legacy VTK, and HDF5-backed
formats) remain deferred until concrete datasets establish the needed
geometry, field, and encoding semantics.

## User workflow and CLI

Add an explicit isosurface invocation:

```text
cargo run -p svf-explorer -- --vtk <path> --vtk-isosurface <field-name> --vtk-isovalue <value>
```

- `--vtk` supplies the input file, as in M9.2.
- `--vtk-isosurface` selects an exact-name point-associated scalar field
  from the imported Dataset.
- `--vtk-isovalue` supplies one finite f64 isovalue.
- Both isosurface options are required together and are valid only with
  `--vtk`.
- The isosurface invocation is mutually exclusive with `--vtk-scalar`.
  M9.3 renders the extracted surface uncolored; it does not silently color
  the result using a source field whose transfer semantics have not been
  requested.
- Existing `--vtk <path>`, `--vtk-scalar <field-name>`, demo flags, and
  default parametric launch retain their M9.2 behavior.
- Duplicate options, missing values, malformed/non-finite isovalues,
  unknown VTK-mode options, and conflicting modes are errors. Do not infer
  precedence or silently fall back to another representation.

The field name is preserved exactly. The isovalue parser must accept ordinary
Rust f64 numeric syntax supported by the CLI and reject NaN and infinities.

## Scientific semantics

- Resolve the named field from the imported Dataset.
- Require a scalar field with Point association. Missing fields and vector
  fields must produce distinct actionable errors; cell- and dataset-associated
  scalar fields must be rejected with an explicit association diagnostic.
- Invoke the existing
  `svf_core::isosurface::extract_tetrahedral_isosurface` API with the
  imported Dataset, exact field name, and f64 isovalue.
- Do not duplicate marching-tetrahedra logic in Explorer or renderer.
- Preserve the returned `Isosurface3`, including its mesh, source-point /
  source-edge vertex provenance, source-cell IDs, isovalue, and extraction
  metadata, for the lifetime of the imported scene.
- The current extraction algorithm supports tetrahedral cells. For mixed
  datasets, preserve its explicit unsupported-cell reporting; do not
  reinterpret hexahedra or other cell types as tetrahedra.
- If extraction yields no triangles, fail before opening the window with a
  contextual diagnostic. Include useful extraction counts/reasons when
  available rather than displaying an empty scene as success.
- Convert the extracted mesh to the renderer's existing triangle-mesh input.
  Scientific coordinates and extraction remain f64 in core; f64-to-f32
  conversion remains at the renderer boundary.
- The extracted surface is an uncolored geometric representation in M9.3.
  Do not mutate the source Dataset, replace its field, or fabricate new
  source point IDs.

The extraction's established half-open classification, deterministic shared
edge/face handling, winding, and degenerate-cell behavior remain governed by
the M8.1 contract and tests. M9.3 must not change those semantics.

## Camera and interaction

- Fit the initial camera to the extracted isosurface mesh bounds, using the
  existing imported-scene camera fitting behavior.
- Reuse existing orbit, pan, zoom, picking synchronization, resize, and
  minimize/restore behavior.
- Do not change global/default demo cameras.

## Explorer state and architecture

- Extend the existing imported-data startup configuration and scene model;
  do not create a separate hard-coded mode per file.
- Retain the original Dataset and extracted `Isosurface3` on the CPU.
- Keep file I/O and CLI handling in Explorer.
- Keep extraction and provenance in `svf-core`.
- Keep `svf-render` unaware of VTK syntax, file paths, field lookup, and
  extraction policy; it receives only prepared triangle geometry.
- Add no new dependency and do not alter the M9.1 parser contract.

## Tests and acceptance

### CLI/configuration tests

- Parse a valid VTK path, field name, and finite isovalue.
- Reject either isosurface option without the other.
- Reject isosurface options without `--vtk`.
- Reject conflicts with `--vtk-scalar` and existing demo flags.
- Reject duplicate options, missing values, malformed values, NaN, and
  positive/negative infinity.
- Verify existing M9.2 VTK modes, all demo modes, and default launch remain
  unchanged.

### Scene preparation tests

- Extract an isosurface from a point-associated scalar field in the committed
  tetrahedron VTK fixture (extend/add a fixture if needed to produce a
  non-empty, non-degenerate surface).
- Verify exact isovalue, expected triangle/vertex counts, finite positions,
  retained source Dataset, and provenance/source-cell metadata.
- Verify a second point scalar field can be selected by exact name.
- Reject missing fields, vector fields, cell-associated fields, and
  dataset-associated fields distinctly.
- Reject non-finite or invalid isovalues.
- Verify a valid dataset whose selected field produces no surface fails
  clearly before window creation.
- Verify mixed tetrahedral/non-tetrahedral input reports unsupported cells
  without misrepresenting them as extracted.
- Verify camera fit is finite and frames the extracted mesh, including
  degenerate bounds handling where applicable.

### User Windows acceptance

- `cargo test -p svf-explorer`
- `cargo test --workspace`
- `cargo check --workspace`
- Run the committed VTK fixture with an isosurface field and isovalue.
- Visually verify the extracted surface is visible immediately and responds
  to orbit, pan, zoom, resize, minimize, and restore.
- Verify existing M9.2 uncolored and scalar-colored surface loading.
- Verify all existing demo modes and default launch remain unchanged.
- Verify invalid field, association, isovalue, and no-surface cases exit
  nonzero before opening a window.

No `cargo fmt` is required unless separately requested. Do not claim local
Windows acceptance until the user reports the results.

## Explicit non-goals

- New isosurface algorithms or changes to M8.1 extraction semantics.
- Hexahedral, structured-grid, or general-cell isosurface extraction.
- Cell-associated scalar isosurface extraction or implicit resampling.
- Coloring the extracted surface from transferred or secondary fields.
- UI field selection, isovalue controls, legends, or interactive sliders.
- Volume rendering, clipping planes, multiple simultaneous isosurfaces,
  time-varying fields, and animation.
- New file formats, binary/XML VTK, VTKHDF, HDF5, or format auto-detection.
- Native file dialogs, drag/drop, reload, or file watching.

## Dependencies

- M8.1 tetrahedral isosurface extraction.
- M9.1 VTK legacy ASCII Dataset import.
- M9.2 Explorer VTK dataset loading and camera fitting.

References:
- `docs/tetrahedral-isosurface.md`
- `docs/vtk-legacy-ascii-import.md`
- `docs/explorer-vtk-loading.md`
