# Milestones

## M0 — Project foundation

- [x] Establish Rust workspace
- [x] Separate scientific core from renderer
- [x] Add a thin native Explorer host
- [x] Document architecture and boundaries

## M1 — Graphics foundation

- [x] Create a native window using winit
- [x] Initialize wgpu instance, adapter, device, and queue
- [x] Create and configure a presentation surface
- [x] Clear the surface through a render pass
- [x] Handle window resizing and zero-sized surfaces
- [x] Handle close requests and surface acquisition states
- [x] Compile on Windows MSVC
- [x] Run on a Windows machine with a supported GPU
- [x] Verify resize/minimize/restore behavior on real hardware
- [x] Verify clean window exit

M1 was locally validated on the user's Windows environment. User-reported
checks passed, the Explorer window opened, and resizing/minimize/restore/close
behaved as expected. The visible clear color is intentionally a very dark
blue-gray. These results have not been independently reproduced here.

## M2 — Spatial foundation

Objective: establish a graphics-independent mathematical model for spatial
coordinates and camera transforms before introducing scientific geometry.

### M2.1 — Coordinate semantics and math foundation

- [x] Select glam for f64 vector/matrix/quaternion primitives
- [x] Declare glam centrally and inherit it in svf-core
- [x] Define right-handed world axes (+Y up)
- [x] Distinguish world points from displacement vectors
- [x] Verify implementation with local Windows workspace checks and tests

### M2.2 — Transform semantics

- [x] Document column-vector multiplication and column-major storage
- [x] Add a composition-order unit test
- [x] Verify transform convention tests locally
- [ ] Add further typed world/view transform APIs if required by later geometry

User-reported Windows validation:
- `cargo check --workspace`: passed.
- `cargo test -p svf-core`: passed.
- `cargo test --workspace`: passed.

### M2.3 — Camera and projection

- [x] Camera and projection implementation
- [x] Glam 0.34 camera API compatibility
- [x] Local Windows `cargo test -p svf-core`: 14 passed, 0 failed (user-reported)
- [x] Local Windows `cargo check --workspace`: passed (user-reported)
- [x] Local Windows `cargo test --workspace`: passed (user-reported)

The combined camera test expects NDC depth 0.88 for its chosen world target at view distance 5 with near=1 and far=11. Camera constructors use `glam::dcamera::rh` APIs.

- [x] Define validated look-at camera pose
- [x] Derive right-handed world-to-view matrix
- [x] Implement validated perspective projection with [0, 1] NDC depth
- [x] Implement validated orthographic projection with [0, 1] NDC depth
- [x] Add perspective aspect-ratio update without mutating camera/world state
- [x] Combine view and projection matrices in documented order
- [x] Add tests for degenerate camera configurations
- [x] Add tests for projection bounds and invalid parameters
- [x] Verify implementation with local Windows workspace checks and tests (user-reported)

### M2.4 — Renderer integration

Implementation committed; Windows runtime behavior visually validated by the user.

- [x] Convert camera matrices to renderer representation explicitly (f64 to f32 at GPU uniform boundary)
- [x] Render reference axes and an XZ-plane world grid with a minimal line pipeline
- [x] Update perspective aspect ratio and camera uniform on non-zero resize without changing world geometry
- [x] Verify visible red X, green vertical Y, and blue Z axes meeting at the origin
- [x] Verify XZ grid orientation and consistent perspective across window sizes
- [x] Verify minimize, restore, and close behavior
- [x] Record final workspace check and test results (user-reported 2026-11-08: workspace check passed; 32 core tests and 2 renderer tests passed in the full workspace run)

## M3 — Scientific geometry

### M3.1 — Point and line primitives

Implementation committed; local Windows validation completed (24 core tests; 26 workspace tests passed).

- [x] Add graphics-independent `PointSet3` with ordered f64 world points
- [x] Add validated `LineSegment3` with endpoint, displacement, and length accessors
- [x] Add validated open `Polyline3` with consecutive segment iteration
- [x] Reject non-finite coordinates with the offending point index
- [x] Preserve repeated points and zero-length segments explicitly
- [x] Require at least two points for a polyline; closed paths repeat the first point explicitly
- [x] Add CPU unit tests for construction, validation, ordering, and segment semantics
- [x] Verify core tests and workspace tests on Windows (24 core; 26 workspace tests passed)

### M3.2 — Indexed triangle meshes

Implementation committed; user-reported local Windows validation completed on 2026-11-08 (32 core tests; 34 workspace tests passed).

- [x] Add graphics-independent `TriangleMesh3` with f64 world-space vertices and flattened `u32` triangle indices
- [x] Preserve vertex order, triangle order, and winding
- [x] Validate that the index count is divisible by three
- [x] Validate every index is in range and report triangle/corner on failure
- [x] Reject repeated vertex indices within a triangle
- [x] Validate all vertex coordinates are finite
- [x] Allow empty meshes, vertices without triangles, and unreferenced vertices
- [x] Preserve geometrically degenerate triangles; geometric quality checks are a separate concern
- [x] Add CPU tests for valid meshes and each validation rule
- [x] Verify `cargo check --workspace` on Windows
- [x] Verify `cargo test -p svf-core` on Windows (32 passed)
- [x] Verify `cargo test --workspace` on Windows (34 passed: 32 core, 2 renderer)
- [ ] Record formatting-check result for this increment

Scope note: this increment validates per-triangle index structure, not global manifoldness, watertightness, connected components, or consistent orientation across adjacent triangles. Those require explicit future topology APIs.

## M4 — Mathematical sampling

Objective: represent mathematical domains and sampled values explicitly before converting them into renderable geometry.

### M4.1 — Bounded domains and sampling specifications

Implementation committed; user-reported Windows validation completed on 2026-11-08.

- [x] Add `Domain1D` with finite, strictly increasing inclusive bounds
- [x] Add `Domain2D` with independent X and Y ranges
- [x] Reject non-finite bounds, non-positive extents, and extents that overflow f64
- [x] Add `SamplingSpec1D` and `SamplingSpec2D` with independent axis counts
- [x] Define sample counts as points, with interval count equal to points minus one
- [x] Require at least two points per sampled axis and report the offending axis
- [x] Keep domain and sampling-specification types independent of function evaluation and rendering
- [x] Test valid bounds, non-finite/equal/reversed/overflowing bounds, independent axes, and invalid sample counts
- [x] Verify formatting and workspace check on Windows
- [x] Verify core tests on Windows (50 passed)
- [x] Verify workspace tests on Windows (52 passed: 50 core, 2 renderer)

### M4.2 — Scalar function sampling

Implementation committed; user-reported Windows validation completed on 2026-11-08.

- [x] Add 1D scalar sampling using caller-supplied `FnMut(f64) -> f64`
- [x] Add 2D scalar sampling using caller-supplied `FnMut(f64, f64) -> f64`
- [x] Preserve exact domain endpoints in 1D and along both 2D axes
- [x] Preserve source coordinates and sample indices in every record
- [x] Define deterministic row-major 2D order with X varying fastest
- [x] Represent finite, NaN, positive-infinity, and negative-infinity results explicitly
- [x] Retain non-finite results without dropping samples or shifting indices- [x] Add checked 2D sample-count multiplication and fallible result allocation
- [x] Test endpoint inclusion, coordinate mapping, finite values, non-finite classification, random access, and overflow
- [x] Verify formatting and workspace check on Windows
- [x] Verify core tests on Windows (58 passed)
- [x] Verify workspace tests on Windows (60 passed: 58 core, 2 renderer)

### M4.3 — Structured grids and surface generation

Implementation committed; user-reported Windows validation completed on 2026-11-08.

- [x] Wrap existing 2D scalar samples in `StructuredGrid2D` without duplicating sample data
- [x] Preserve dimensions, row-major ordering, source coordinates, and finite/non-finite values
- [x] Generate a CPU-side `TriangleMesh3` graph surface from finite samples
- [x] Map the 2D domain to world XZ and scalar values to world Y (+Y up)
- [x] Retain finite vertices even when adjacent cells are omitted
- [x] Omit both triangles of any cell containing a non-finite corner; do not bridge holes
- [x] Define deterministic cell diagonal and +Y-facing triangle winding
- [x] Map generated mesh vertices to source grid samples and triangles to source cells
- [x] Check u32 vertex-index capacity and reserve output allocations fallibly
- [x] Test dimensions/access, triangulation, winding, invalid cells, source mapping, and f64 values
- [x] Verify formatting and workspace check on Windows
- [x] Verify core tests on Windows (67 passed)
- [x] Verify workspace tests on Windows (69 passed: 67 core, 2 renderer)

### M4.4 — Sampling quality

Documentation committed in `docs/sampling-quality.md`.

- [x] Explain sample point counts versus interval counts and endpoint inclusion
- [x] Explain uniform spacing and independent 2D axis resolution
- [x] Describe sample, cell, and triangle growth as resolution increases
- [x] Explain piecewise-linear approximation and limits of uniform sampling
- [x] Describe error factors and conditional interpolation-error assumptions
- [x] Clarify that Nyquist-style guarantees require band-limited assumptions
- [x] Provide a practical resolution-selection and comparison workflow
- [x] Document current absence of adaptive sampling, error estimation, and anti-alias filtering

## M5 — Interactive surface demonstrator

### M5.1 — Indexed triangle rendering and depth testing

Implementation committed and user-validated on Windows (2026-11-08).

- [x] Convert `TriangleMesh3` positions from f64 to f32 at the renderer boundary and reject non-finite narrowed values
- [x] Preserve source vertex order and u32 indices in GPU buffers
- [x] Add an indexed TriangleList pipeline using the existing camera uniform
- [x] Add a Depth32Float target, clear it each frame, and enable mesh depth testing/writes
- [x] Recreate the depth target on non-zero window resize
- [x] Render opaque mesh before reference lines; lines depth-test without writing depth
- [x] Add CPU tests for mesh conversion, index preservation, and f32 range rejection
- [x] Feed a sampled flat graph surface from svf-core into Explorer as an integration fixture
- [x] Verify `cargo fmt --all -- --check` on Windows
- [x] Verify `cargo check --workspace` on Windows
- [x] Verify `cargo test -p svf-render` on Windows (4 passed)
- [x] Verify `cargo test --workspace` on Windows (71 passed: 67 core, 4 renderer)
- [x] Visually verify the indexed surface, grid/axes, and resize/restore behavior on Windows
- [ ] Perform a dedicated visual occlusion/depth-intersection stress test

### M5.2 — Parametric surface

Implementation committed and user-validated on Windows (2026-11-08).

- [x] Generate `f(x, y) = A * sin(x) * cos(y)` on world XZ with scalar values on world Y
- [x] Use the existing svf-core scalar sampling and structured-grid pipeline
- [x] Add Up/Down amplitude controls with a bounded range and visible current value
- [x] Regenerate CPU-side scientific geometry when amplitude changes
- [x] Update existing GPU mesh buffers without recreating pipelines, camera, or reference geometry
- [x] Reject mesh updates that change the allocated vertex/index counts
- [x] Test amplitude scaling and preservation of mesh topology
- [ ] Confirm formatting, workspace compilation, and tests for the M5.2 commit on Windows
- [x] Visually verify the parametric surface and live amplitude adjustment on Windows, including increasing and decreasing A and returning to A=0.00

### M5.3 — Scientific inspection

Core ray intersection, sampled-surface source mapping, viewport unprojection,
and Explorer hover/click integration are implemented and user-validated on
Windows (2026-11-08).

- [x] Add validated f64 world-space rays with normalized directions
- [x] Intersect rays with indexed triangle meshes without back-face culling
- [x] Return nearest non-negative hit distance, world position, triangle index, and barycentric weights
- [x] Skip degenerate and near-parallel triangles; accept hits at ray origin
- [x] Map sampled-surface hits to their source grid cell
- [x] Recover continuous domain coordinates, continuous grid coordinates, and piecewise-linear scalar value
- [x] Test intersection edge cases and source-data recovery
- [x] Verify `cargo fmt --all -- --check` on Windows (passed after `cargo fmt --all`)
- [x] Verify `cargo check --workspace` on Windows (passed)
- [x] Verify `cargo test --workspace` on Windows (84 passed: 79 core, 1 Explorer, 4 renderer)
- [x] Convert physical viewport pixels to world rays using inverse view-projection unprojection and WebGPU NDC depth [0, 1]
- [x] Support perspective and orthographic ray origins
- [x] Expose the renderer's active camera to CPU-side picking
- [x] Connect Explorer cursor tracking and left-click scientific surface inspection
- [x] Add live hover feedback in the window title for surface hits and misses
- [x] Add explicit click diagnostics with pixel/viewport coordinates and HIT/MISS/ray-error status
- [x] Visually verify hover feedback and picking at multiple surface locations and outside the surface on Windows
- [x] Verify picking remains correct after resizing, minimizing, and maximizing the window on Windows
- [x] Verify picking remains correct after changing amplitude, including returning to the flat A=0.00 surface, on Windows

User-reported Windows validation is complete. The user ran `cargo fmt --all`,
then confirmed formatting check, workspace compilation, and all 84 workspace
tests passed. The formatting-only cleanup was committed locally as`d545466` (`style: format workspace`). Copilot's read-only diff inspectionfound no semantic changes. The pre-sync stash remains untouched and must not
be blindly restored; it is an older pre-picking snapshot.

### M3.3 — Mesh topology queries

Implementation committed; user-reported Windows validation completed on 2026-11-08.

- [x] Derive topology independently from the existing `TriangleMesh3` storage
- [x] Extract unique undirected edges with canonical ascending vertex indices
- [x] Assign deterministic edge IDs using lexicographic endpoint ordering
- [x] Expose face-to-edge incidence in local triangle edge order
- [x] Expose edge-to-face incidence in source face order
- [x] Query boundary edges (exactly one incident face)
- [x] Query edges incident to more than two faces without making a global manifoldness claim
- [x] Cover shared edges, boundary edges, disconnected components, unreferenced vertices, non-manifold edge incidence, deterministic IDs, and empty-face meshes
- [x] Verify `cargo fmt --all -- --check` on Windows
- [x] Verify `cargo check --workspace` on Windows
- [x] Verify `cargo test -p svf-core` on Windows (38 passed)
- [x] Verify `cargo test --workspace` on Windows (40 passed: 38 core, 2 renderer)

Scope note: edge incidence is descriptive topology data. It does not certify consistent adjacent-face orientation, manifold vertices, watertightness, or global surface validity.

### M5 prerequisite — sampled-surface coordinate correction

- [x] Correct graph-surface vertices to `(x, scalar_value, y)` in world space
- [x] Correct triangle winding to face +Y
- [x] Preserve source sample and source-cell mappings
- [x] Verify formatting and workspace compilation on Windows
- [x] Verify core tests on Windows (67 passed)
- [x] Verify workspace tests on Windows (69 passed: 67 core, 2 renderer)


## M6 — Scientific datasets and field semantics

Objective: introduce a reusable, graphics-independent representation of
scientific quantities and their relationships to dataset geometry/topology.
Scientific data remains authoritative on the CPU; rendering consumes derived
representations.

### M6.1 — Field types and association semantics

Implementation is committed to `svf-core`; user-reported Windows workspace
validation completed (94 tests passed: 89 core, 1 Explorer, 4 renderer).

- [x] Add explicit point, cell, and dataset field associations
- [x] Add field metadata with a non-empty name and optional descriptive units
- [x] Add scalar fields retaining finite and non-finite `SampleValue` entries
- [x] Add three-component vector fields with independently classified components- [x] Declare vector coordinate basis explicitly (world basis in this increment)
- [x] Enforce exactly one value for dataset-associated fields
- [x] Provide point/cell cardinality validation against supplied dataset counts
- [x] Preserve field value order and avoid implicit association conversion
- [x] Add unit tests for metadata, values, non-finite handling, basis, and cardinality
- [ ] Verify `cargo fmt --all -- --check` on Windows (not run for this increment)
- [x] Verify `cargo check --workspace` on Windows (user-reported)
- [x] Verify `cargo test --workspace` on Windows (94 passed: 89 core, 1 Explorer, 4 renderer)
- [x] Record user-reported local results in the handoff

Design details: `docs/fields.md`.

M6.1 established field semantics independently; M6.2 subsequently added
dataset ownership and geometry-aware attachment, including the distinction
between logical source cells and generated render triangles.


### M6.2 — Dataset containers and attribute attachment

Implementation merged and user-validated on Windows (2026-11-08).

Merged to `main` through PR #1:
https://github.com/Aine-dickson/sciviz/pull/1

Merge commit: `9b4fc41` (feature head `dfb27c9`).

- [x] Add a source-geometry enum for point sets, polylines, triangle meshes,
  and structured 2D grids
- [x] Define source point/sample counts and logical cell counts per geometry
- [x] Add a dataset container owning immutable source geometry
- [x] Attach scalar and three-component vector fields with association-aware
  cardinality validation
- [x] Enforce unique field names across scalar and vector field kinds
- [x] Preserve field insertion order and provide typed name lookup
- [x] Ensure failed attachment leaves the dataset unchanged
- [x] Keep structured-grid rectangle counts independent of generated surface
  triangle counts
- [x] Add focused unit tests for dataset geometry and field attachment
- [x] Review implementation and public API
- [x] Verify `cargo check --workspace` on Windows (user-reported)
- [x] Verify `cargo test --workspace` on Windows (103 passed: 98 core,
  1 Explorer, 4 renderer; doc tests passed)

No formatter was run for this increment. M6.1 formatting-only changes remain
separate in the user's local stash and must not be blindly restored.

Design boundary: geometry is immutable through `Dataset`; changing source
geometry creates a new dataset. Field transfer/remapping, mutable topology,
unstructured grids, time-varying datasets, and automatic interpolation are
outside M6.2.

### M6.3 — Unstructured topology foundations

Implementation committed; user-reported Windows validation completed on 2026-11-08.

Design decisions are recorded in `docs/unstructured-topology.md`.

- [x] Define CSR connectivity with stable source point/cell array ordering
- [x] Define initial linear cell types: Line2, Triangle3, Quad4, Tetrahedron4, and Hexahedron8
- [x] Permit mixed cell types and topological dimensions
- [x] Validate finite points, offsets, fixed cell arity, point-index bounds, and repeated point indices
- [x] Preserve structurally valid geometric degeneracy and input ordering
- [x] Add `UnstructuredGrid3` to `DatasetGeometry` point/cell counts
- [x] Add a dataset field-cardinality integration test
- [x] Add unit tests for mixed cells, ordering, empty/default grids, and connectivity validation
- [x] Review the complete implementation and public API
- [x] Verify `cargo check --workspace` on Windows (passed)
- [x] Verify `cargo test --workspace` on Windows (117 passed: 112 core, 1 Explorer, 4 renderer)

User-reported validation completed on Windows (2026-11-08). No formatting command was run for this increment.

Scope excludes rendering, file importers, higher-order cells, variable-arity polygons, mesh repair, quality metrics, and general unstructured adjacency.


### M6.4 — Unstructured volume boundary extraction

Implementation committed, user-validated on Windows, and merged through
PR #3 on 2026-11-08.

- PR: https://github.com/Aine-dickson/sciviz/pull/3
- Merge commit: `d4e78c1bb73c44945ee4258d87e5f69e477a7a94`.

- [x] Define deterministic face identity from canonical point IDs
- [x] Extract triangular faces from `Tetrahedron4`
- [x] Extract quadrilateral faces from `Hexahedron8`
- [x] Preserve source cell IDs, local face indices, and local point ordering
- [x] Classify faces by volume-cell incidence: boundary, interior, non-manifold
- [x] Ignore lower-dimensional cells in volume topology queries
- [x] Keep topology graphics-independent and preserve source data
- [x] Add tests for single cells, shared faces, non-manifold incidence,
  mixed volume types, source mapping, determinism, coordinate identity,
  empty input, and geometric degeneracy
- [x] Review implementation and public API
- [x] Verify `cargo check --workspace` on Windows (passed)
- [x] Verify `cargo test -p svf-core` on Windows (125 passed)
- [x] Verify `cargo test --workspace` on Windows (130 passed: 125 core,
  1 Explorer, 4 renderer)

User-reported validation completed on Windows (2026-11-08). No formatting
routine was run; unrelated formatting changes and existing stashes remain
untouched.


### M6.5 — Unstructured boundary surface representation

Design contract: `docs/boundary-surface.md`.

- [x] Extract only one-incidence boundary faces from M6.4 volume topology
- [x] Preserve triangle and quadrilateral face types without triangulation
- [x] Build a compact deterministic point array with source-point mapping
- [x] Preserve source topology face, source cell, and local face provenance
- [x] Exclude interior and non-manifold faces from the extracted boundary surface
- [x] Preserve source local face cycles without claiming outward orientation
- [x] Keep extraction graphics-independent and leave source grid/fields unchanged
- [x] Add 13 focused tests for tetrahedra, hexahedra, mixed cells, shared faces,
  non-manifold faces, compact indexing, provenance, deterministic ordering,
  empty input, and geometric degeneracy
- [x] Verify `cargo check --workspace` on Windows
- [x] Verify `cargo test -p svf-core` on Windows
- [x] Verify `cargo test --workspace` on Windows

Scope excludes quad triangulation, renderer integration, field transfer,
orientation repair, geometric quality, and file import/export.


### M6.6 — Provenance-preserving boundary surface triangulation

Design contract: `docs/surface-triangulation.md`.

- [x] Pass source triangle faces through unchanged
- [x] Split each source quad into two triangles
- [x] Select quad diagonals deterministically from sorted original source
  point-ID endpoint pairs (lexicographically smaller pair wins)
- [x] Preserve source local-cycle winding without claiming outward orientation
- [x] Preserve boundary-face array index, original topology face index,
  volume-cell, local-face, and per-face triangle ordinal provenance
- [x] Preserve compact f64 points and source-point-ID mapping
- [x] Add ten focused tests for both diagonal choices, explicit source-ID
  selection, mixed face arities, provenance, determinism, immutability,
  empty input, and geometric degeneracy
- [x] Review public API and implementation
- [x] Verify `cargo check --workspace` on Windows
- [x] Verify `cargo test -p svf-core` on Windows (148 passed)
- [x] Verify `cargo test --workspace` on Windows (153 passed: 148 core, 1 Explorer, 4 renderer)

Scope excludes renderer integration, field transfer, orientation repair,
geometric quality optimization, and file import/export.


## M7 — Unstructured volume visualization

Objective: take the graphics-independent unstructured volume geometry and
provenance pipeline through to a visible native representation without making
GPU buffers authoritative.

### M7.1 — Unstructured boundary-surface rendering

Design contract: `docs/unstructured-rendering.md`.

Status: complete; PR #6 merged to main on 2026-11-08. Windows validation user-reported.

- [x] Add an optional `--unstructured` Explorer mode; keep the existing
  parametric-surface mode as the default
- [x] Build a deterministic fixture containing one `Hexahedron8` and one
  disconnected `Tetrahedron4`
- [x] Derive its `BoundarySurface3` and `TriangulatedSurface3` using
  existing `svf-core` APIs
- [x] Adapt triangulated f64 points/connectivity to `TriangleMesh3` without
  reordering points or triangles
- [x] Retain `TriangulatedSurface3` and source provenance on the CPU
- [x] Reuse the existing indexed renderer, depth testing, and no-cull policy
- [x] Add tests for fixture counts, polygon arities, triangle counts,
  per-cell provenance, render-mesh conversion, and index validity
- [x] Verify `cargo check --workspace` on Windows (passed without warnings)
- [x] Verify core tests as part of `cargo test --workspace` (148 passed)
- [x] Verify `cargo test --workspace` on Windows (159 passed: 148 core, 7 Explorer, 4 renderer; doc-tests passed)
- [x] Visually verify both volume-cell surfaces, depth behavior, and
  resize/minimize/restore in unstructured mode
- [x] Re-run the default Explorer and verify parametric amplitude controls
  and sampled-surface picking remain functional

Scope excludes field coloring/transfer, unstructured picking UI, normals,
lighting, materials, orientation repair, volume rendering, import/export,
and new cell types.


M7.1 Windows validation and merge completed (2026-11-08):
- PR #6 merged to `main`; merge commit `1d34ee1`.
- User confirmed both visual modes, existing interactions, and resize/minimize/restore behavior.
- `cargo check --workspace` passed without warnings.
- `cargo test --workspace` passed (159 tests: 148 core, 7 Explorer, 4 renderer); doc-tests passed.
- No formatting routine was run.


### M7.2 — Reusable scalar visualization

Design contract: `docs/scalar-visualization.md`.

Status: complete and merged to `main` through PR #7 on 2026-11-08
(merge commit `09c99da`). Implementation, Windows automated validation,
numerical edge-case validation, and all four Explorer-mode visual checks
are complete.
- [x] Define a common surface representation with source point IDs per render  vertex and source logical cell IDs per render triangle
- [x] Adapt structured `SampledSurface3`, unstructured
  `TriangulatedSurface3`, and identity-mapped `TriangleMesh3`
- [x] Implement reusable scalar range resolution and continuous color maps in
  graphics-independent `svf-core`
- [x] Support point-, cell-, and dataset-associated scalar fields
- [x] Define finite/non-finite, constant-field, explicit-range, and clamping
  semantics
- [x] Expand triangle corners for mapped output so cell colors remain constant
  per triangle and do not bleed across shared indexed vertices
- [x] Preserve source point/cell provenance and never mutate source fields or
  geometry
- [x] Add a colored-surface renderer input path while preserving the existing
  flat-color mesh path
- [x] Demonstrate the same mapping API on a structured point field and an
  unstructured cell field through optional Explorer modes
- [x] Add focused core, renderer conversion, and Explorer demo/mode tests
- [x] Verify `cargo check --workspace` on Windows after renderer integration
- [x] Verify `cargo test --workspace` on Windows after renderer integration
  (175 passed: 163 core, 7 Explorer, 5 renderer; doc-tests passed)
- [x] Compile and test the Explorer scalar demonstration implementation on
  Windows (`cargo check --workspace` passed; `cargo test --workspace`
  passed: 177 tests — 163 core, 9 Explorer, 5 renderer; doc-tests passed)
- [x] Visually verify both scalar demonstrations and resize/minimize/restore
- [x] Re-verify default and `--unstructured` modes after Explorer integration
- [x] Fix and regression-test normalization across extreme finite scalar
  ranges (including [-f64::MAX, f64::MAX])
- [x] Verify the final core scalar tests on Windows (16 passed)
- [x] Verify the final workspace on Windows (178 passed: 164 core,
  9 Explorer, 5 renderer; doc-tests passed)
- [x] Verify the final workspace check on Windows (passed without warnings)

Scope excludes vector glyphs, contours, isosurfaces, volume rendering,
time-varying fields, implicit field transfer/interpolation, GPU-side scalar
mapping, logarithmic/percentile ranges, materials/lighting, on-screen text
legends, file import/export, and mesh repair.


## M7.3 — Vector field visualization

Design contract: `docs/vector-field-visualization.md`.

Status: complete; PR #9 merged to `main` on 2026-11-08
(merge commit `23c33fb855ffa2d52749baf66c1fb23eed53d19c`). User-reported
Windows workspace tests and visual/runtime validation passed.

Objective: convert point- and cell-associated `VectorField3` data into
graphics-independent, provenance-preserving 3D arrow glyphs in `svf-core`,
then render prepared glyph geometry in `svf-render`.

Initial semantics:
- Point glyphs use exact source point positions.
- Cell glyphs use deterministic arithmetic-mean source-vertex centers;
  these are not mass or volume centroids.
- Dataset-associated vectors are deferred because they have no intrinsic
  spatial anchor; the API must not invent one.
- World-basis vectors only.
- Explicit magnitude-scaled and uniform-length policies; no automatic scaling.
- Zero and non-finite vectors retain explicit source status.
- Core generates arrow line segments in f64; renderer only converts and draws.

Implemented:
- [x] Resolve point-associated vectors to exact source point positions and IDs
- [x] Resolve cell-associated vectors to documented vertex-average anchors
  for polylines, triangle meshes, structured grids, and unstructured cells
- [x] Preserve source vector components, source identity, and source ordering
- [x] Classify finite nonzero, zero, non-finite-component, and magnitude-overflow
  vectors without fabricating directions
- [x] Generate f64 arrow geometry with explicit magnitude-scaled and
  uniform-length policies
- [x] Convert and render prepared glyph segments through the depth-aware
  renderer path
- [x] Add a deterministic point-associated `--vectors` Explorer mode
  (49 point vectors, uniform display length 0.8)
- [x] Verify `cargo test --workspace` on Windows (user-reported: all tests pass)
- [x] Visually verify vector glyph rendering, hover sampling, grid/axes, and
  resize/minimize/restore on Windows
- [x] Merge PR #9 to `main`

Implementation coverage note: the core supports cell-associated vectors, but
the Explorer demonstration currently exercises point association only.
A separate structured-cell demonstration is the next increment. Some
short/densely distributed glyphs in `--vectors` are difficult to distinguish;
M7.4 will use a sparser structured-cell fixture and an explicit display length.

### M7.4 — Structured cell-associated vector visualization

Design contract: `docs/cell-vector-visualization.md`.

Status: complete and merged to `main` through PR #11 on 2026-11-08
(merge commit `082fc5e18db8a7e40e1655dff70950af2fcc1e03`).
User-reported Windows workspace tests passed.

Objective: demonstrate cell-associated vector fields on a structured graph
surface, using source-cell vertex-average anchors and the existing preparedglyph renderer path.

- [x] Build a dataset whose geometry is the same `StructuredGrid2D` used to
  generate the displayed graph surface
- [x] Attach a cell-associated, world-basis `VectorField3` with matching
  logical-cell cardinality
- [x] Resolve each vector to its source cell ID and arithmetic-mean four-corner
  graph-surface anchor
- [x] Generate deterministic finite, nonzero vectors with varying directions
- [x] Use a sparse 8 × 8 cell fixture and explicit 1.0-world-unit uniform
  glyph length for legibility
- [x] Add Explorer fixture, source-anchor, cardinality, glyph, and mode-selection tests
- [x] Add optional `--cell-vectors` without changing existing modes
- [x] Retain source dataset and glyph result on the CPU
- [x] Verify workspace tests on Windows (user-reported: all passed)
- [x] Verify glyph placement, grid/axes, and resize/minimize/restore visually
- [x] Inspect the cell-vector glyphs from multiple camera angles using M7.5 navigation

Visual feedback: the glyph pattern is structured and arrowheads are visible,
but many glyphs remain small/crowded. At M7.4 completion, the fixed camera
prevented a conclusive visual depth-occlusion check. M7.5 now supplies
multi-angle camera inspection, and the user reports that the cell-vector mode
passes the navigation checks.

Unstructured volume-cell glyphs remain deliberately excluded: their arithmetic
cell centers lie inside opaque volume boundaries and are not usefully visible
without a separate visibility/representation design.

## M7.5 — Interactive 3D camera navigation

Design and implementation contract: `docs/camera-navigation.md`.

Status: complete; merged to `main` through PR #13 on 2026-11-08.

- Merge commit: `32002a3cbce2a3c03755cbda67dc8b759de1892d`.
- [x] Define reusable, input-independent f64 camera navigation semantics in
  `svf-core`
- [x] Implement validated orbit, pan, and zoom with explicit distance,
  elevation, and sensitivity limits
- [x] Add validated camera replacement in `svf-render`, synchronizing stored
  camera state and the GPU uniform
- [x] Map left drag to orbit, middle drag to pan, and wheel to zoom in Explorer
- [x] Preserve click inspection and keep picking rays synchronized with the
  currently rendered camera
- [x] Cover navigation math, invalid inputs, distance limits, and camera
  stability with core tests
- [x] Verify `cargo test --workspace` on Windows (user-reported: passed)
- [x] Verify `cargo check --workspace` on Windows (passed; one pre-existing
  `VectorSource3::new` dead-code warning remains)
- [x] Verify navigation, picking, resize, and minimize/restore in
  `--cell-vectors` on Windows
- [x] Verify navigation in default, `--vectors`, `--scalar`,
  `--unstructured`, and `--unstructured-scalar` modes on Windows
- [x] Inspect M7.4 cell-associated glyphs from multiple camera angles

User-reported validation completed on Windows (2026-11-08). The workspace
test and check commands passed, and runtime navigation passed in all six
Explorer modes. Exact test counts were not recorded in the final validation
report. The only reported check warning is the pre-existing
`VectorSource3::new` dead-code warning.



## M8.1 — Tetrahedral isosurface extraction

Design and implementation contract: `docs/tetrahedral-isosurface.md`.

Status: complete; core PR #16 and Explorer PR #17 merged to `main`
on 2026-11-08.

- Core merge commit: `08c69cee2aa132b12e4d79e52f71f8b69b410858`.
- Explorer merge commit: `f4d7ade5d45c109e40580b095ae6af9724621b56`.

Implemented:
- [x] Extract point-associated scalar isosurfaces from `Tetrahedron4`
  cells in `UnstructuredGrid3` using f64 marching-tetrahedra calculations
- [x] Define half-open low/high classification and deterministic handling
  for exact-isovalue source vertices and shared faces
- [x] Reuse canonical shared-edge intersections and preserve interpolation
  provenance measured from the lower source point ID to the higher ID
- [x] Preserve per-vertex source provenance and per-triangle source cell IDs
- [x] Orient triangles toward increasing scalar values and choose quad
  diagonals deterministically
- [x] Report unsupported cell types and skipped invalid/degenerate cells
- [x] Add deterministic Explorer `--isosurface` mode and retain source
  dataset plus derived result CPU-side
- [x] Verify focused isosurface test on Windows (user-reported: passed)
- [x] Verify `cargo test --workspace` on Windows (user-reported: all passed;
  200 core, 15 Explorer, 7 renderer; doc-tests passed)
- [x] Verify `cargo check --workspace` on Windows (user-reported: passed)
- [x] Visually verify extracted surface rendering, camera behavior, and
  window interactions on Windows
- [x] Merge core implementation PR #16 and Explorer PR #17 to `main`

The user reported that the extracted surface works correctly and behavior
remains consistent. The only reported compiler warning is the pre-existing
`VectorSource3::new` dead-code warning.

## M8.2 — Provenance-based scalar transfer onto derived isosurfaces

Design contract: `docs/isosurface-field-transfer.md`.

Status: complete; core PR #20 and renderer/Explorer PR #21 merged to
`main` on 2026-11-09.

- Core transfer PR #20 merge commit:
  `531bd64da3f57dbe20c37dad8b591874bc625043`.
- Renderer/Explorer PR #21 merge commit:
  `ccc84e0b72fb1cedbaa6a159ecc2d5c384b2a917`.

Implemented:
- [x] Transfer point-associated scalar values onto derived isosurface
  vertices using exact source-point/source-edge provenance
- [x] Validate source association, field cardinality, provenance bounds,
  canonical edge ordering, and interpolation parameter range
- [x] Preserve exact source-point values and exact edge endpoint values
- [x] Define NaN and signed-infinity propagation for interior edge
  interpolation
- [x] Use sign-aware f64 interpolation for finite endpoint values and report
  unexpected non-finite results
- [x] Retain the exact `Isosurface3`, source field metadata, and transferred
  values in `TransferredScalarField3`
- [x] Add `ScalarRange::from_values` and `MappedIsosurface3`, with one
  mapped color per unique derived vertex
- [x] Keep `MappedSurface3` unchanged and avoid fabricating source point IDs
- [x] Add renderer support for prepared mapped isosurface geometry; convert
  f64 positions and RGBA colors to f32 only at the renderer boundary
- [x] Add separate Explorer `--colored-isosurface` mode using a distinct
  point-associated scalar field; preserve `--isosurface` unchanged
- [x] Retain source dataset, extracted isosurface, and mapped derived
  representation CPU-side
- [x] Verify Windows workspace tests (user-reported: 228 passed —
  204 core, 17 Explorer, 7 renderer)
- [x] Visually verify `--colored-isosurface`; user reports the output
  matches the expected mapped-color rendering
- [x] Confirm the original `--isosurface` mode remains unchanged

User-reported Windows validation (2026-11-09): all 228 workspace tests
passed, and the colored isosurface output matched the expected appearance.
The user authorized merging PR #21 after validation. No claim is made here
that `cargo check --workspace` was run for this increment; the latest
reported validation explicitly confirms the test suite and visual output.

Explicit non-goals remain: cell/dataset/vector/tensor transfer, structured
or hexahedral extraction, general resampling, volume rendering, GPU-side
interpolation, and general material/legend systems.


## M9 — Scientific dataset import

### M9.1 — VTK legacy ASCII unstructured-grid import (complete; merged 2026-11-09)

Design contract: `docs/vtk-legacy-ascii-import.md`.
PR #23: https://github.com/Aine-dickson/sciviz/pull/23  
Merge commit: `efa1cdbd59c09ca97a67a7e674f5c5483cc948c8`  
Feature head: `fd48c4b10900f19558eca59d681c7fde6cb3068f`.
- [x] Add a pure, graphics-independent VTK legacy ASCII `UNSTRUCTURED_GRID` parser to `svf-core`
- [x] Preserve supported mixed-cell topology, point/cell ordering, and point/cell scalar/vector field associations
- [x] Support LINE, TRIANGLE, QUAD, TETRA, and HEXAHEDRON cells
- [x] Parse supported numeric types, float32 widening, non-finite field values, optional scalar metadata, and inline lookup-table definitions
- [x] Validate the positional file envelope, counts, connectivity, indices, numeric types (including empty arrays), duplicate records/fields, and unsupported attributes
- [x] Return categorized errors and never expose a partially constructed dataset
- [x] Keep binary VTK, unsupported cell/attribute types, dataset fields, and Explorer file loading out of scope
- [x] Review implementation and public API
- [x] Verify focused importer tests on Windows (12 passed)
- [x] Verify `cargo test --workspace` on Windows (240 passed: 216 core, 17 Explorer, 7 renderer; doc-tests passed)
- [x] Verify `cargo check --workspace` on Windows
- [x] Merge PR #23

The only reported compiler warning is the pre-existing `VectorSource3::new` dead-code warning. M9.1 is complete. M9.2 will address user-facing Explorer file loading while keeping parsing and scientific data ownership in `svf-core`.


## M9.2 — Explorer VTK dataset loading (complete; merged 2026-11-09)

Design contract: `docs/explorer-vtk-loading.md`.

PR #25: https://github.com/Aine-dickson/sciviz/pull/25  
Merge commit: `e9d57463a11dc89a6c43b8ba2677fdb79517a174`.

- [x] Add `--vtk <path>` and optional `--vtk-scalar <field-name>`
- [x] Parse arguments and load UTF-8 input before creating the event loop/window
- [x] Reuse the unchanged M9.1 `svf-core` VTK parser
- [x] Extract and triangulate volume boundaries for the existing triangle renderer
- [x] Report omitted lower-dimensional cells and reject files without a renderable volume boundary
- [x] Map selected point- or cell-associated scalar fields through the existing provenance-aware Viridis pipeline
- [x] Fit and apply a perspective camera to imported mesh bounds
- [x] Retain source Dataset, derived surface, provenance, and mapped representation CPU-side
- [x] Preserve default launch and all existing Explorer demo modes
- [x] Add a committed tetrahedron-plus-line VTK smoke fixture
- [x] Verify `cargo test -p svf-explorer` on Windows (26 passed)
- [x] Verify `cargo test --workspace` on Windows (249 passed: 216 core, 26 Explorer, 7 renderer; doc-tests passed)
- [x] Verify `cargo check --workspace` on Windows
- [x] Visually verify uncolored, point-associated scalar, and cell-associated scalar rendering
- [x] Verify initial camera framing, zoom interaction, and missing-file failure before window creation
- [x] Merge PR #25

User-reported Windows validation (2026-11-09): all 249 workspace tests
passed, doc-tests passed, and `cargo check --workspace` passed. The imported
tetrahedral surface was visible and centered immediately at startup. Point
scalar colors varied across the surface; cell scalar color remained uniform
for the source cell. A missing path produced a contextual error and nonzero
exit before a window opened. The only compiler warning was the pre-existing
`VectorSource3::new` dead-code warning.

M9.2 is complete. Binary VTK, native file dialogs, standalone rendering of
lower-dimensional-only datasets, vector-field controls, legends, and
additional cell types remain outside this increment.

### M9.3 — Isosurface extraction from imported VTK fields (complete; merged 2026-11-09)

Design contract: M9.3 Explorer isosurface design PR #26 (merged).

- PR #27: https://github.com/Aine-dickson/sciviz/pull/27
- Merge commit: `b05f59371a7023460ac60b22f6787839ef23a4e4`.
- [x] Add `--vtk-isosurface <field-name>` and finite `--vtk-isovalue <value>`
- [x] Reuse M8.1 tetrahedral extraction and preserve isosurface provenance
- [x] Add optional `--vtk-isosurface-color <point-scalar-field>` using M8.2
  provenance-based scalar transfer and Viridis mapping
- [x] Add a depth-tested overlay of unique undirected triangle edges
- [x] Retain the source Dataset, extracted Isosurface3, and optional mapped
  result on the CPU
- [x] Fit the initial camera to the extracted surface and reject empty results
  before window creation
- [x] Preserve existing VTK loading and demo modes
- [x] Verify `cargo test -p svf-explorer` on Windows (35 passed)
- [x] Verify `cargo test --workspace` on Windows (258 passed: 216 core,
  35 Explorer, 7 renderer; doc-tests passed)
- [x] Verify `cargo check --workspace` on Windows
- [x] Visually verify both triangles, independent `pressure` coloring, and
  the contrasting shared-edge overlay
- [x] Merge PR #27

User-reported Windows validation (2026-11-09): all 258 workspace tests
passed, doc-tests passed, and `cargo check --workspace` passed. The
`--vtk-isosurface temperature --vtk-isovalue 25 --vtk-isosurface-color
pressure` runtime displayed both triangular facets with a visible shared
edge and a gradient from the independent `pressure` field. The only
reported compiler warning remains the pre-existing
`VectorSource3::new` dead-code warning.

M9.3 implementation, automated validation, visual acceptance, and merge are
complete. Future work must begin with an explicitly selected scope and design
contract; do not infer the next milestone.


## M10.1 — Scientific inspection of imported surfaces (complete; merged 2026-11-09)

Design contract: PR #28. Implementation: [PR #29](https://github.com/Aine-dickson/sciviz/pull/29).

- Merge commit: `5baadb4d80da0ec1ad1810f8e1ca77015c610f5f`.
- [x] Add graphics-independent ray-based inspection APIs in `svf-core`
- [x] Resolve boundary hits to source cell, face, local-face, and triangle identity
- [x] Resolve isosurface hits through source-cell provenance
- [x] Evaluate point-associated fields barycentrically and cell-associated
  fields from the exact source cell; preserve dataset-associated values
- [x] Transfer isosurface samples through source-point/source-edge provenance
- [x] Preserve field names, associations, units, and non-finite classifications
- [x] Add Explorer hover feedback and detailed click diagnostics
- [x] Preserve parametric inspection and camera/click-versus-drag interactions
- [x] Verify focused core inspection tests on Windows (4 passed)
- [x] Verify Explorer tests on Windows (35 passed)
- [x] Verify workspace tests on Windows (262 passed: 220 core, 35 Explorer,
  7 renderer; doc-tests passed)
- [x] Verify `cargo check --workspace` on Windows
- [x] Visually/runtime verify boundary and colored-isosurface inspection
- [x] Verify orbit, pan, zoom, and click-versus-drag behavior
- [x] Merge PR #29

User-reported Windows runtime validation (2026-11-09): boundary hits resolved
the correct source cell and boundary faces, with barycentric point-field
values and the exact cell-associated density value. Isosurface hits resolved
to source cell 0; the extraction field evaluated to approximately the requested
isovalue (25), while the independent pressure field was interpolated through
the source provenance. Camera and click-versus-drag interactions remained
correct. The only reported compiler warning was the pre-existing
`VectorSource3::new` dead-code warning.

M10.1 implementation, automated validation, runtime acceptance, and merge are
complete. The next milestone is intentionally unselected.


## M10.2 — Imported VTK point-vector visualization (complete; merged 2026-11-09)

Design contract: `docs/vtk-point-vector-visualization.md` (PR #30).  
Implementation: [PR #31](https://github.com/Aine-dickson/sciviz/pull/31).  
Merge commit: `1fe8fa051dbeb1f61d374a56b9be8c1570938d66`.

- [x] Add `--vtk-vectors <field-name>` for point-associated world-basis vectors
- [x] Reuse existing source resolution, vector analysis, glyph construction,
  and renderer paths
- [x] Preserve source point anchors, vector order, values, and CPU-side Dataset
- [x] Apply deterministic uniform display length from 5% of boundary bounds
  diagonal with a positive floor; do not encode magnitude
- [x] Support combination with `--vtk-scalar`
- [x] Reject missing/wrong field types, unsupported associations, and
  incompatible isosurface combinations
- [x] Extend the committed VTK fixture and Explorer tests
- [x] User-reported Windows `cargo test -p svf-explorer` passed
- [x] User-reported Windows `cargo test --workspace` passed
- [x] User-reported Windows `cargo check --workspace` passed
- [x] User visually confirmed vector glyph rendering
- [x] Merge PR #31

The user confirmed all requested test commands passed and the vector figure
rendered. Exact test counts were not included in the report. M10.2 is complete.

### Roadmap direction after M10.2

Do not automatically choose another general-purpose framework milestone.
The next work must be selected from the user's own scientific, engineering,
educational, or research use cases. Start by understanding the concrete
problem, data/model, analysis or experiment, and desired output. General-purpose
abstractions are justified when they directly support those needs; do not
chase feature parity or hypothetical user demand. Maintain scientific
correctness and the established architecture constraints.


### E1 — Chemistry composition core (complete; merged 2026-11-09)

Research/API proposal: PR #32. Implementation: [PR #33](https://github.com/Aine-dickson/sciviz/pull/33). Merge commit: `46b134c2755b5ee5a815774ab1e035bafc602f70`.

- [x] Add graphics-independent `svf_core::chemistry` module
- [x] Define reusable Hydrogen, Carbon, and Oxygen element identities
- [x] Define distinct atom and bond identities and semantic bond orders
- [x] Represent molecular coordinates as finite `f64` ångström values
- [x] Validate molecule identity, endpoints, self-bonds, and duplicate undirected bonds
- [x] Add water and methane composition/connectivity tests
- [x] Verify focused chemistry tests on Windows (16 passed)
- [x] Verify `svf-core` tests on Windows (236 passed)
- [x] Verify workspace tests on Windows (283 passed: 236 core, 40 Explorer, 7 renderer)
- [x] Verify `cargo check --workspace` on Windows
- [x] Merge PR #33

The core represents structural composition and semantic connectivity, not full chemical validity or behaviour. Geometry/rendering adapters and interactive educational views remain future work and must be driven by the learning objectives and actual examples.


### E2 — Chemistry molecular views (complete; merged 2026-11-09)

Design contract: [PR #34](https://github.com/Aine-dickson/sciviz/pull/34), document `docs/chemistry-molecular-views.md`. Implementation: [PR #35](https://github.com/Aine-dickson/sciviz/pull/35). Implementation merge commit: `e7381a9c82bef10f625b57592da2f99f5ccf3555`.

- [x] Define graphics-independent ball-and-stick views over the E1 molecule model
- [x] Preserve atom and bond identities, endpoints, and semantic bond order
- [x] Keep representation dimensions explicitly in ångströms and distinguish visual radii from empirical atomic boundaries
- [x] Derive deterministic single-, double-, and triple-bond axis geometry
- [x] Validate style entries, missing element styles, coincident bonded endpoints, and non-finite derived geometry
- [x] Cover water, methane, empty/isolated molecules, deterministic offsets, provenance, overflow, and source immutability
- [x] Verify focused molecular-view tests on Windows (10 passed)
- [x] Verify `cargo test -p svf-core` on Windows (246 passed)
- [x] Verify `cargo test --workspace` on Windows (293 passed: 246 core, 40 Explorer, 7 renderer)
- [x] Verify `cargo check --workspace` on Windows
- [x] Merge PR #35

The representation is a graphics-independent adapter, not a renderer or interactive lesson. Renderer integration and further chemistry capabilities remain unselected until a concrete teaching or learning workflow establishes their value. The existing `VectorSource3::new` and `prepare_imported_scene_with_options` dead-code warnings were non-blocking.

## E3 — Chemistry learning workflow: upper-secondary VSEPR (design complete; merged 2026-11-09)

- PR #36 established the chemistry molecular learning workflow: teacher-led demonstration, guided exploration, and independent investigation using water and methane. Merge commit: `34fc271dc6b784c877896afba88207bdbf18242b`.
- The user selected upper secondary as the first target, with later downward adaptation/testing as needed.
- PR #37 defines the focused learning case `docs/upper-secondary-chemistry-vsepr-learning-case.md`. Merge commit: `9c2fdb8702ec700aa9cbb557a0a03fadddde3f65`.
- Curriculum anchor: Uganda NCDC 2025 Advanced Secondary Chemistry, Senior Five Term One, Topic 3: Bonding and Structure, Sub-topic 3.2: Covalent Bonds and Molecular Structures (18 periods; official syllabus pp. 9, 18–19).
- Learning question: why is methane tetrahedral while water is bent? Both have four electron domains around the central atom; methane has four bonding pairs, while water has two bonding pairs and two lone pairs. The lesson distinguishes electron-domain arrangement from molecular shape and compares approximate bond angles (CH₄ 109.5°, H₂O 104.5°).
- The design includes prerequisites, inquiry sequence, model-guided exploration, formative assessment through observation/conversation/learner product, and explicit representation limits.
- Critical E1/E2 gap: the current chemistry model has no lone-pair/electron-domain entities, VSEPR arrangement/shape semantics, or validated bond-angle provenance. Ball-and-stick atoms and bonds alone cannot explain the central learning outcome.
- SVF is the likely owner of scientific electron-domain semantics and inspection; Mava owns lesson sequencing, prompts, reveal and assessment; Prismal is not required for this static structural lesson.
- No implementation is authorized by the design alone. Next work must design the smallest scientifically correct electron-domain representation and validate it against the CH₄/H₂O learning case before renderer/Explorer or lesson integration is scoped. Do not build a general VSEPR engine or molecule editor by default.

## E3.1 — VSEPR electron-domain model design (complete; merged 2026-11-09)

- Design contract: `docs/vsepr-electron-domain-design.md`, PR #38. Merge commit: `89b9aaa273e9ac38b643c227367dea8268dc9c76`.
- Defines a separate VSEPR semantic layer over the E1 molecule model, with stable electron-domain identities and explicit bonding-pair/lone-pair classification.
- Bonding domains retain exact source `BondId` and neighbouring `AtomId`; lone-pair domains belong only to the central atom and are not represented as atoms or bonds.
- Initial scope is only the four-domain tetrahedral cases: methane (4 bonding, 0 lone pairs; tetrahedral molecular shape) and water (2 bonding, 2 lone pairs; bent molecular shape).
- Separates molecular coordinates from an idealized electron-domain arrangement schematic. It does not claim schematic lone-pair positions are measured electron-density maxima.
- Does not hardcode measured bond angles into the semantic model. Angle values require explicit coordinate/reference provenance.
- No implementation, renderer/Explorer integration, Prismal/Mava integration, or general VSEPR engine has been started. Next increment should implement and test the bounded semantic model in `svf-core`; representation adapter is a separate decision after semantic review.

### E3.2 — VSEPR electron-domain core implementation (complete; merged 2026-11-09)

Implementation: [PR #39](https://github.com/Aine-dickson/sciviz/pull/39). Squash merge commit: `849e11236cd85f0454f4f85dcf0d7a599ffa8830`.

- [x] Add stable `ElectronDomainId` identities and explicit bonding-pair/lone-pair classifications
- [x] Add a validated `VseprCentre` tied to the source `Molecule`
- [x] Preserve exact source bond and neighbouring-atom provenance for bonding domains
- [x] Validate central atom, domain identities, bond incidence, neighbour identity, complete incident-bond coverage, and supported domain patterns
- [x] Support methane (4 bonding / 0 lone pairs → tetrahedral) and water (2 bonding / 2 lone pairs → bent)
- [x] Verify focused VSEPR tests on Windows
- [x] Verify `cargo test -p svf-core` on Windows (259 passed)
- [x] Verify `cargo test --workspace` on Windows (306 passed: 259 core, 40 Explorer, 7 renderer)
- [x] Verify `cargo check --workspace` on Windows
- [x] Merge PR #39

The user reported no failures. The semantic core is complete. A schematic electron-domain adapter, renderer/Explorer integration, and lesson integration remain separate decisions; do not assume these are the next tasks without validating the concrete educational interaction.
