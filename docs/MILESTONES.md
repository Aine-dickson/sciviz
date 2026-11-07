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
- [x] Generate a CPU-side `TriangleMesh3` height field from finite samples
- [x] Retain finite vertices even when adjacent cells are omitted
- [x] Omit both triangles of any cell containing a non-finite corner; do not bridge holes
- [x] Define deterministic cell diagonal and +Z-facing triangle winding
- [x] Map generated mesh vertices to source grid samples and triangles to source cells
- [x] Check u32 vertex-index capacity and reserve output allocations fallibly
- [x] Test dimensions/access, triangulation, winding, invalid cells, source mapping, and f64 values
- [x] Verify formatting and workspace check on Windows
- [x] Verify core tests on Windows (67 passed)
- [x] Verify workspace tests on Windows (69 passed: 67 core, 2 renderer)

### M4.4 — Sampling quality

- [ ] Document resolution, density, and numerical error considerations
- [ ] Avoid implying that denser uniform sampling guarantees accuracy for arbitrary functions

## M5 — Interactive surface demonstrator

- render `z = A * sin(x) * cos(y)`
- adjust A and regenerate only affected geometry
- inspect a selected point and recover its source coordinates/value

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