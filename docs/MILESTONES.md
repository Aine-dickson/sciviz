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

Implementation is committed to `svf-core`; local Windows validation is
pending.

- [x] Add explicit point, cell, and dataset field associations
- [x] Add field metadata with a non-empty name and optional descriptive units
- [x] Add scalar fields retaining finite and non-finite `SampleValue` entries
- [x] Add three-component vector fields with independently classified components
- [x] Declare vector coordinate basis explicitly (world basis in this increment)
- [x] Enforce exactly one value for dataset-associated fields
- [x] Provide point/cell cardinality validation against supplied dataset counts
- [x] Preserve field value order and avoid implicit association conversion
- [x] Add unit tests for metadata, values, non-finite handling, basis, and cardinality
- [ ] Verify `cargo fmt --all -- --check` on Windows
- [ ] Verify `cargo check --workspace` on Windows
- [ ] Verify `cargo test --workspace` on Windows
- [ ] Record user-reported local results in the handoff

Design details: `docs/fields.md`.

M6.1 intentionally does not add a dataset container or geometry attachment.
M6.2 will define those contracts against actual dataset topology, including
the distinction between logical source cells and generated render triangles.


### M6.2 — Dataset containers and attribute attachment

Implementation is on branch `m6-2-dataset-attachment`; local Windows
validation is pending.

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
- [x] Add unit tests for geometry counts, valid attachment, mismatch rejection,
  duplicate names, and structured-grid source-cell semantics
- [ ] Review implementation diff and public API on Windows
- [ ] Verify `cargo check --workspace` on Windows
- [ ] Verify `cargo test --workspace` on Windows
- [ ] Record user-reported local validation in the handoff

Design note: geometry is immutable through `Dataset`; changing source
geometry creates a new dataset. Field transfer/remapping, mutable topology,
unstructured grids, time-varying datasets, and automatic interpolation are
outside M6.2.


M6.2 user-reported Windows validation (2026-11-08):
- `cargo check --workspace`: passed.
- `cargo test --workspace`: 103 passed, 0 failed (98 core, 1 Explorer,
  4 renderer; doc tests also passed).
- The nine dataset-specific unit tests all passed.
- No formatter was run; formatting-only changes from M6.1 remain in a
  separate stash on the user's local repository.
