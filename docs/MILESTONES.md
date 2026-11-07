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

- bounded domains and sampling strategies
- function sampling with invalid-value handling
- surface mesh generation
- sampling density and error considerations

## M5 — Interactive surface demonstrator

- render `z = A * sin(x) * cos(y)`
- adjust A and regenerate only affected geometry
- inspect a selected point and recover its source coordinates/value

### M3.3 — Mesh topology queries (planned)

- Define stable, deterministic unique-undirected-edge extraction
- Define face-to-edge and edge-to-face incidence semantics
- Preserve source vertex/face identity and triangle winding
- Test boundary edges, shared edges, disconnected components, and non-manifold edge incidence without silently classifying meshes as manifold or watertight
- Keep all topology data and algorithms in `svf-core`, independent of the renderer
