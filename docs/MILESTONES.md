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
- [x] Retain non-finite results without dropping samples or shifting indices
- [x] Add checked 2D sample-count multiplication and fallible result allocation
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
tests passed. The formatting-only cleanup was committed locally as
`d545466` (`style: format workspace`). Copilot's read-only diff inspection
found no semantic changes. The pre-sync stash remains untouched and must not
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
- [x] Add three-component vector fields with independently classified components
- [x] Declare vector coordinate basis explicitly (world basis in this increment)
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

- [x] Define a common surface representation with source point IDs per render
  vertex and source logical cell IDs per render triangle
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

Status: design prepared on branch `m7-4-cell-vector-visualization`;
implementation has not started.

Objective: demonstrate cell-associated vector fields on a structured graph
surface, using source-cell vertex-average anchors and the existing prepared
glyph renderer path.

- [ ] Build a dataset whose geometry is the same `StructuredGrid2D` used to
  generate the displayed graph surface
- [ ] Attach a cell-associated, world-basis `VectorField3` with matching
  logical-cell cardinality
- [ ] Resolve each vector to its source cell ID and arithmetic-mean four-corner
  graph-surface anchor
- [ ] Generate deterministic finite, nonzero vectors with varying directions
- [ ] Use a sparse fixture and explicit uniform glyph length for legibility
- [ ] Add core integration and Explorer mode-selection/demo tests
- [ ] Add optional `--cell-vectors` without changing existing modes
- [ ] Retain source dataset and glyph result on the CPU
- [ ] Verify workspace tests and checks on Windows
- [ ] Visually verify arrow direction, arrowheads, correct depth occlusion,
  and resize/minimize/restore
- [ ] Re-run `--vectors` and default mode for regression coverage

Unstructured volume-cell glyphs are deliberately excluded: their arithmetic
cell centers lie inside opaque volume boundaries and are not usefully visible
without a separate volume/visibility representation design.
