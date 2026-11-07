# Scientific Visualization Framework — Handoff

Last updated: 2026-11-08

## Project objective

Build a Rust-first, reusable scientific visualization framework for accurate,
interactive visualization of mathematical models and scientific datasets,
with 2D and 3D support. This is not a general-purpose 3D modelling package,
game engine, or scene editor. Scientific correctness, explicit data semantics,
and educational communication take priority over decorative rendering.

## Repository and workflow

- Repository: https://github.com/Aine-dickson/sciviz
- Default branch: `main`
- Initial platform: Windows
- Delivery: Rust library first, with `apps/explorer` as a thin native host.
- The user develops and validates locally on Windows and shares exact output.
- Repository documentation is the source of truth across sessions.
- Never claim local compilation, tests, commits, or runtime validation without
  evidence. Separate implementation from user-reported validation.
- Keep changes focused and do not change architectural decisions silently.

## Workspace

- `svf-core`: graphics-independent scientific and mathematical foundations.
- `svf-render`: wgpu rendering infrastructure and native surface integration.
- `svf-explorer`: thin winit desktop host.

Workspace dependencies: wgpu 30.0.1, winit 0.30.13, pollster 0.4,
tracing 0.1, glam =0.34.2. Glam enables `std` and `float-types`; svf-core
inherits it using `glam.workspace = true`.

## Milestone status

### M0 — Project foundation

Completed.

### M1 — Graphics foundation

Implementation and user-reported local Windows validation completed:
- `cargo check -p svf-render` passed.
- `cargo check --workspace` passed.
- Explorer window opened and displayed the dark blue-gray clear color.
- Resize, minimize/restore, and clean close worked.

The wgpu 30 compatibility adjustment uses
`InstanceDescriptor::new_without_display_handle()` and sets
`apply_limit_buckets: false` in `RequestAdapterOptions`.

### M2 — Spatial foundation

Implementation and user-reported local Windows validation completed for M2.1–M2.4. The final renderer/workspace automated test output should still be confirmed if not already captured.

#### M2.1 — Coordinate semantics and math foundation

Implementation and user-reported local validation completed:
- Glam =0.34.2 pinned centrally and inherited by svf-core.
- Right-handed world axes, +Y up.
- `WorldPoint3` and `WorldVector3` preserve point/displacement semantics.
- Tests cover handedness, point/vector arithmetic and f64 operations.

#### M2.2 — Transform semantics

Initial convention implementation and user-reported local validation completed:
- Column-vector multiplication and column-major storage documented.
- Matrix composition-order test passes.
- Additional typed transform APIs may be added when required by geometry.

User-reported Windows results:
- `cargo check --workspace`: passed.
- `cargo test -p svf-core`: passed.
- `cargo test --workspace`: passed.

#### M2.3 — Camera and projection

Implementation and user-reported local Windows verification completed.

New file: `crates/svf-core/src/camera.rs`, exposed through
`pub mod camera` in `svf-core/src/lib.rs`.

Public API:
- `CameraError`: explicit validation failures.
- `LookAtCamera`: validated world-space position, target and up direction;
  derives a right-handed world-to-view matrix.
- `PerspectiveProjection`: validated vertical FOV, aspect ratio, near and
  far distances; uses right-handed WebGPU [0, 1] NDC projection.
- `OrthographicProjection`: validated view-space bounds and positive near/far
  distances; uses right-handed WebGPU [0, 1] NDC projection.
- `Projection3`: perspective/orthographic projection enum.
- `Camera3`: combines pose and projection; returns view, projection, and
  projection * view matrices.

Validation covers non-finite and degenerate camera inputs, zero/parallel up
vectors, invalid FOV/aspect/near/far/bounds, expected NDC depth endpoints,
orthographic bounds, aspect-ratio update, and transform composition.

Conventions:
- Camera looks from position toward target.
- Camera view space is right-handed, +Y up, -Z forward.
- Perspective FOV is vertical and expressed in radians.
- Near and far are positive distances, with 0 < near < far.
- Orthographic bounds are explicit camera/view-space extents.
- Projection depth is [0, 1], suitable for WebGPU.
- The camera math is f64 and has no wgpu dependency.
- The renderer performs explicit f64-to-f32 conversion when preparing the GPU camera uniform.

Important API behavior:
- `Projection3::with_aspect_ratio` updates perspective projection only.
  It returns `InvalidAspectRatio` for orthographic projection because
  orthographic extents are explicit; callers must choose how to recompute them.

Potential follow-up review:
- Consider whether the relative parallel-vector threshold should be exposed
  or adjusted after edge-case testing with extreme coordinate magnitudes.
- Consider a separate orientation/quaternion camera representation when
  interactive orbit/free-flight controls are designed; current look-at state
  is intentionally simple and explicit.

#### M2.4 — Renderer integration

Implementation committed; user-reported Windows runtime and visual verification completed.

Implemented in `svf-render`:
- `SceneRenderer` owns the wgpu surface, device, queue, camera uniform, line pipeline, and reference geometry.
- `CameraUniform` converts the core `DMat4` to a column-major f32 matrix at the GPU boundary.
- A minimal WGSL line-list pipeline draws a world-space XZ grid (Y=0) and RGB X/Y/Z axes.
- A perspective reference camera looks toward the origin from (14, 12, 18).
- Non-zero surface resize updates the camera's perspective aspect ratio and uniform, then reconfigures the surface. Zero-sized resize is ignored.
- `apps/svf-explorer` now instantiates `SceneRenderer`.
- Unit tests cover reference geometry layout and finite camera-uniform conversion.

User-reported Windows visual/runtime verification:
- Red X, green vertical Y, and blue Z axes meet at the origin.
- The grid lies on the XZ plane, with Y perpendicular to it.
- Axis orientation and perspective remain consistent across window sizes; narrower viewports crop the grid horizontally without obvious stretching.
- Minimize, restore, and close work.

The user successfully launched the Explorer after the wgpu 30 pipeline-layout correction. Keep explicit final automated workspace/render test results as pending unless confirmed in the conversation.

### M3 — Scientific geometry

#### M3.1 — Point and line primitives

Implementation committed; local Windows validation completed by user: formatting check and workspace check passed; `cargo test -p svf-core` passed 24 tests; `cargo test --workspace` passed 26 tests.

New file: `crates/svf-core/src/geometry.rs`, exported through
`pub mod geometry`.

Public API:
- `GeometryError`: non-finite point with offending index, or a polyline with fewer than two points.
- `PointSet3`: ordered collection of independent `WorldPoint3` values. Empty sets are valid; no connectivity is implied.
- `LineSegment3`: two finite endpoints, displacement, and Euclidean length. Coincident endpoints are allowed and produce zero length.
- `Polyline3`: ordered sequence of at least two finite points. Consecutive points define segments; repeated points are retained. The polyline is open unless the first point is explicitly repeated at the end.

All geometry remains in f64 world coordinates. No wgpu types or GPU buffers are exposed by svf-core. Tests cover empty point sets, coordinate validation, endpoint semantics, length, minimum polyline size, ordered segment iteration, repeated points, and explicit closure.

M3.1 local Windows validation (user-reported): formatting check passed; workspace check passed; 24 core tests passed; 26 workspace tests passed.

#### M3.2 — Indexed triangle meshes

Implementation committed; user-reported local Windows validation completed on 2026-11-08.

Updated file: `crates/svf-core/src/geometry.rs`.

New public API:
- `TriangleMesh3`: indexed triangle mesh with ordered `WorldPoint3` vertices and flattened `u32` indices (three indices per triangle).
- `GeometryError` additions: incomplete triangle index group, out-of-range vertex index (with triangle and corner), and repeated vertex index within a triangle.

Contract:
- Vertex and triangle order, including winding, are preserved.
- Empty mesh, vertices without triangles, and unreferenced vertices are valid.
- All vertex coordinates must be finite.
- Index count must be divisible by three.
- Every index must reference an existing vertex.
- A triangle cannot repeat a vertex index.
- Geometrically degenerate triangles (e.g. collinear vertex positions) are preserved. Geometric quality validation is separate from index-topology validity.
- No global manifold, watertightness, connected-component, or adjacent-face orientation checks are implemented yet.
- All data remains CPU-side f64 geometry; no renderer or GPU dependency is introduced.

Eight M3.2 tests cover empty/vertex-only meshes, index and winding preservation, unused vertices, incomplete groups, out-of-range indices, repeated indices, non-finite positions, and geometrically degenerate triangles.

M3.2 local Windows validation (user-reported, 2026-11-08): `cargo check --workspace` passed; `cargo test -p svf-core` passed 32 tests; `cargo test --workspace` passed 34 tests (32 core, 2 renderer), with no failures. The pasted output did not include the formatting-check result, so formatting status is not newly recorded here.

M3.2 is complete. M3.3 adds a graphics-independent, derived topology query layer. Unique undirected edges are canonicalized and lexicographically ordered by vertex indices; face-edge incidence follows local triangle edge order; edge-face incidence preserves source face order. Boundary-edge and greater-than-two-face-incidence queries are provided without claiming global manifoldness or watertightness.

M3.3 user-reported Windows validation (2026-11-08): `cargo fmt --all -- --check` and `cargo check --workspace` passed; `cargo test -p svf-core` passed 38 tests; `cargo test --workspace` passed 40 tests (38 core, 2 renderer), with no failures.

## Accepted coordinate conventions

- Right-handed world coordinates, +Y up.
- Camera view space: right-handed, +Y up, -Z forward.
- Column vectors; transforms multiply on the left.
- Matrices use column-major storage.
- WebGPU NDC depth range [0, 1].
- f64 in core; explicit f32 conversion at renderer boundary.
- World, view, clip/NDC, and viewport/pixel spaces remain distinct.

## Architecture constraints

- Scientific data, coordinates, topology and simulation state must not live
  only in GPU buffers.
- Public svf-core APIs must not expose wgpu types.
- Preserve f64 where scientific calculations require it.
- Renderer consumes prepared visualization data; it does not own scientific
  algorithms.
- Picking should eventually map screen locations back to scientific entities
  and source values.
- Add crate boundaries only when they represent stable responsibilities.

## Repository documents

- `README.md`
- `docs/ARCHITECTURE.md`
- `docs/MILESTONES.md`
- `docs/decisions/0001-windows-first.md`
- `docs/decisions/0002-coordinate-conventions.md`

## Local validation commands

From the repository root:

```powershell
cargo check --workspace
cargo test -p svf-core
cargo test --workspace
```

## M4 — Mathematical sampling (next)

Begin M4.1 design: define bounded mathematical domains and validated sampling specifications without coupling them to function evaluation, mesh generation, or rendering. Agree on domain semantics and test cases before implementing the API. M3.3 is complete and user-validated on Windows (38 core tests; 40 workspace tests).
