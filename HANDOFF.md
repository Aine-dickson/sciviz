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

## M4 — Mathematical sampling

### M4.1 — Bounded domains and sampling specifications

Implemented, committed, and user-validated on Windows (2026-11-08):
- `Domain1D`: finite, strictly increasing inclusive bounds; rejects a
  non-finite representable extent.
- `Domain2D`: independent validated X and Y ranges.
- `SamplingSpec1D` and `SamplingSpec2D`: point counts, not interval counts;
  at least two points per axis. Interval count is point count minus one.
- `DomainError`, `SamplingError`, and `SamplingAxis` expose validation
  failures.

User-reported validation: formatting check and workspace check passed;
50 core tests and 52 workspace tests (50 core, 2 renderer) passed.

### M4.2 — Scalar function sampling

Implementation committed and user-validated on Windows (2026-11-08). User reports `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test -p svf-core`, and `cargo test --workspace` all passed: 58 core tests and 60 workspace tests (58 core, 2 renderer).

Public API is exposed from `svf_core::sampling`:
- `sample_scalar_1d(domain, spec, function)` evaluates a caller-provided
  `FnMut(f64) -> f64`.
- `sample_scalar_2d(domain, spec, function)` evaluates a caller-provided
  `FnMut(f64, f64) -> f64`.
- `ScalarSample1D` and `ScalarSample2D` retain sample indices, source
  coordinates, and values.
- `ScalarSamples1D` and `ScalarSamples2D` retain every requested sample;
  the 2D collection supports row/column lookup.
- `SampleValue` distinguishes finite results from `NonFiniteValue::NaN`,
  `PositiveInfinity`, and `NegativeInfinity`.
- 1D endpoints are evaluated at the exact domain bounds. 2D samples are
  stored row-major, with X varying fastest and Y selecting rows.
- 2D sample-count multiplication is checked for overflow. Result storage is
  reserved fallibly before invoking the function, reporting allocation failure
  through `SamplingError`.
- Function evaluation is CPU-side and independent of geometry/rendering.
- Eight tests cover endpoints, finite values, non-finite classification,
  deterministic 2D ordering, coordinate/index lookup, retained invalid
  samples, and sample-count overflow.

User validation commands:
```powershell
cargo fmt --all -- --check
cargo check --workspace
cargo test -p svf-core
cargo test --workspace
```



### M4.3 — Structured grids and surface generation

Implementation committed and user-validated on Windows (2026-11-08).

Public API is exposed from `svf_core::sampling`:
- `StructuredGrid2D::from_samples(ScalarSamples2D)` owns the original
  row-major scalar sample collection without duplicating coordinates/values.
- Grid dimensions and indexed sample access preserve original coordinates
  and finite/non-finite value classification.
- `to_surface_mesh()` creates a CPU-side `TriangleMesh3` graph surface.
  The 2D domain maps to world XZ and scalar values map to world Y, so vertices
  are `(x, scalar_value, y)` under the framework's +Y-up convention.
- Each finite sample becomes one mesh vertex, including finite vertices
  unreferenced because neighboring cells contain invalid samples.
- A cell emits both triangles only when all four corner samples are finite;
  otherwise the entire cell is omitted. No hole is silently bridged.
- The lower-left to upper-right diagonal gives +Y-facing winding.
- `SampledSurface3` maps each mesh vertex to its source grid sample and each
  triangle to its source grid cell for later inspection and picking.
- Output allocations are reserved fallibly and vertex indices are checked
  against the u32 mesh index format.
- Nine focused tests cover dimensions/access, mesh generation, winding,
  invalid-cell omission, source identity, all-invalid grids, and f64 values.

User-reported Windows validation completed on 2026-11-08:
- `cargo fmt --all -- --check` passed.
- `cargo check --workspace` passed.
- `cargo test -p svf-core` passed (67 tests).
- `cargo test --workspace` passed (69 tests: 67 core, 2 renderer).

### M4.4 — Sampling quality

Documentation committed in `docs/sampling-quality.md` (2026-11-08).
Explains point counts versus intervals, uniform spacing, 2D storage and
triangle growth, piecewise-linear surface approximation, factors affecting
error, conditional interpolation-error assumptions, limits of Nyquist-style
reasoning for arbitrary functions, a practical resolution workflow, and
current limitations (no adaptive sampling, error estimation, or anti-alias
filtering).



### M5 prerequisite — sampled-surface coordinate correction

During the M5 renderer audit, the M4.3 graph-surface mapping was found to
conflict with the accepted +Y-up world convention: the original implementation
placed scalar values on world Z and generated +Z-facing triangles. Corrected
the mapping to world coordinates `(x, scalar_value, y)`, placing the sampled
domain on XZ and scalar values on Y. Triangle winding now faces +Y. Source
sample and source-cell mappings remain unchanged. This is a coordinate
correction to M4.3, not a change to the mathematical sampling domain.

The correction is committed and user-validated on Windows (2026-11-08):
formatting and workspace check passed; 67 core tests and 69 workspace tests
(67 core, 2 renderer) passed.


## M5.1 — Indexed triangle rendering and depth testing

Implementation committed and user-validated on Windows (2026-11-08).

Changes:
- `svf-render` accepts a graphics-independent `TriangleMesh3` when
  constructing `SceneRenderer`.
- Mesh positions are converted from f64 world coordinates to f32 at the
  renderer boundary. Conversion rejects coordinates that become non-finite
  after narrowing. Vertex order and u32 triangle indices are preserved.
- Added a dedicated indexed TriangleList pipeline and WGSL mesh shader.
- Added a Depth32Float target. It is cleared per frame, recreated on
  non-zero resize, and used by depth-aware line and mesh pipelines.
- The mesh pipeline currently uses a fixed unlit color. Generic
  `TriangleMesh3` does not yet carry normals or a surface-shading contract;
  lighting/material work is intentionally deferred.
- Explorer generates a small flat sampled graph surface through svf-core and
  passes its mesh to the renderer. This is an integration fixture; M5.2
  replaces it with the adjustable parametric surface.

Renderer unit tests cover f64-to-f32 conversion, preservation of indices, and
rejection of values outside the representable f32 range.

User-reported Windows validation (2026-11-08):
- `cargo fmt --all -- --check` passed.
- `cargo check --workspace` passed.
- `cargo test -p svf-render` passed (4 renderer tests).
- `cargo test --workspace` passed (67 core tests and 4 renderer tests;
  71 total, no failures).
- Explorer displayed a flat blue indexed surface on the XZ plane. The
  reference grid remained visible across and beyond the surface; red X and
  blue Z lay along the surface while green Y rose vertically at the origin.
- Window resize and restore preserved the projection without noticeable
  distortion.
- The user reported the expected projection and visible geometry. No separate
  explicit occlusion/depth-intersection stress test was reported.

Files changed:
- `crates/svf-render/src/lib.rs`
- `crates/svf-render/src/mesh.wgsl` (new)
- `apps/explorer/src/main.rs`

GitHub commits:
- `d8d089c` — indexed triangle mesh pipeline and depth target
- `4bd5dfd` — mesh WGSL shader
- `4925824` — sampled flat surface in Explorer
- `ad73763` — render opaque mesh before depth-tested reference lines
- `1fc79c5` — renderer documentation and conversion-test cleanup
- `45d1383` — keep generic mesh shader unlit until normals are defined
- `d826410` — use wgpu 30 optional depth-state fields and align test formatting

Initial Windows validation exposed four compile errors: wgpu 30 defines
`DepthStencilState.depth_write_enabled` and `depth_compare` as optional
fields. The implementation has been corrected to use `Some(...)`. The same
correction commit also aligns the reported rustfmt differences in the renderer
tests.

M5.1 is closed. The next increment is M5.2 — replace the flat integration
fixture with the adjustable parametric graph surface. Preserve the existing
renderer/core boundary: sample and generate geometry in svf-core, then pass
the resulting TriangleMesh3 to the renderer.


## M5.2 — Parametric surface and live amplitude adjustment

Implementation committed; awaiting user-reported Windows validation.

Explorer now samples the demonstration function

`f(x, y) = A * sin(x) * cos(y)`

over the square domain [-5, 5] × [-5, 5], with 41 sample points per axis.
The first domain coordinate maps to world X, the second to world Z, and
the scalar result maps to world Y. The surface remains an ordinary
`SampledSurface3` generated through the existing svf-core sampling and
structured-grid APIs; no special-case parametric mesh representation was
added to the core.

Interactive controls:
- Up Arrow increases amplitude A by 0.25.
- Down Arrow decreases amplitude A by 0.25.
- A is clamped to [0, 3].
- The window title displays the current amplitude.

On each adjustment, Explorer reevaluates the function and regenerates the
CPU-side scientific surface. `SceneRenderer::update_mesh` converts the new
mesh and writes its vertex/index data into existing GPU buffers. It rejects
updates that change vertex or index counts, avoiding buffer reallocation for
this fixed-topology demonstration. Pipelines, camera, reference grid/axes,
and depth target are retained.

A renderer unit-level test is not yet added for GPU buffer updates because
that operation requires a live device; the Explorer test verifies that
amplitude scales all surface heights while preserving topology and produces
both positive and negative heights.

Files changed:
- `crates/svf-render/src/lib.rs`
- `apps/explorer/src/main.rs`

GitHub commits:
- `36fe2ee` — add fixed-topology scientific mesh buffer updates
- `6517312` — add adjustable parametric surface demonstration
- `9cae655` — handle an uninitialized renderer safely during amplitude updates

Next local validation:
```powershell
cargo fmt --all -- --check
cargo check --workspace
cargo test -p svf-render
cargo test --workspace
cargo run -p svf-explorer
```

Visual verification should confirm a smooth undulating surface and that Up/Down
changes its height while preserving the reference axes/grid and projection.
