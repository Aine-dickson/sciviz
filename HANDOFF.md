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

Implementation committed and user-validated on Windows (2026-11-08).

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

User-reported Windows validation (2026-11-08): the surface changed from flat at A=0.00 to increasingly pronounced peaks and troughs across A=0.25–3.00 as Up Arrow increased amplitude. Decreasing amplitude back to zero returned it to a flat surface. The reference grid, colored axes, and camera view remained in place throughout. This confirms both directions of interactive adjustment and the zero-amplitude boundary. The user had already reported the 67 core and 4 renderer tests passing for the preceding M5.1 validation; no new M5.2 command output was included in this visual report, so M5.2's latest compile/test status remains tied to the previous validation run unless separately confirmed.


## M5.3 — Scientific inspection (core intersection layer)

Initial core implementation committed on 2026-11-08; Windows validation is
pending.

New file: `crates/svf-core/src/ray.rs`, exposed as `svf_core::ray`.

Public API:
- `Ray3::new(origin, direction)` validates finite inputs and a non-zero,
  finite direction length, then normalizes the direction. Ray distances are
  consequently measured in world units.
- `Ray3::point_at(distance)` evaluates a point along the half-line.
- `intersect_mesh(&ray, &TriangleMesh3)` performs two-sided Möller–Trumbore
  intersection and returns the nearest non-negative hit.
- `RayHit3` retains distance, world hit position, source triangle index, and
  barycentric weights in the triangle's stored index order.
- Degenerate and near-parallel triangles are skipped. A hit at the ray origin
  is valid.

`SampledSurface3::inspect_ray` now maps a hit back to
`SurfaceInspection3`, including triangle index, source grid cell, barycentric
weights, world position, continuous domain coordinates (X/Y), continuous
grid-sample coordinates, and the piecewise-linear scalar value (world Y).
The result is interpolated over the generated triangle; it is not a fresh
evaluation of the original function. The existing mesh vertex/triangle source
mappings remain the authoritative source identity.

Tests added for ray validation/normalization, two-sided intersection,
barycentric weights, nearest-hit selection, behind-ray rejection, misses,
parallel/degenerate triangles, origin hits, scientific coordinate/value
recovery, and inspection misses.

Files changed:
- `crates/svf-core/src/ray.rs` (new)
- `crates/svf-core/src/lib.rs`
- `crates/svf-core/src/sampling.rs`
- `crates/svf-core/src/sampling/grid.rs`

Implementation commits:
- `3d6970f` — add graphics-independent ray and mesh intersection
- `db0afc3` — expose ray intersection foundations
- `10109bd` — map sampled surface ray hits to scientific source data
- `cc3aecf` — export sampled surface inspection results

User-reported Windows test results (2026-11-08):
- `svf-core`: 79 passed.
- Explorer: 1 passed.
- `svf-render`: 4 passed.
- Workspace total: 84 passed, 0 failed.
The user supplied the workspace test output after syncing to commit
`87f1d59`. Formatting and `cargo check --workspace` results were not
included and remain unconfirmed.

### M5.3 — Viewport unprojection and Explorer interaction validation

Implementation is committed and the user has now validated the interaction
on Windows (2026-11-08).

The camera exposes
`Camera3::ray_from_viewport(pixel_x, pixel_y, width, height)`. It accepts
physical-pixel coordinates from a top-left-origin viewport, unprojects NDC
depth 0 and 1 through the inverse view-projection matrix, and returns a
validated f64 `Ray3`. Perspective rays originate at the camera position;
orthographic rays originate on the near plane and remain parallel. Invalid
pixels, zero-sized viewports, singular transforms, and invalid homogeneous
coordinates are reported explicitly.

The renderer exposes its current `Camera3` by reference so picking uses the
same camera and projection as rendering. Explorer retains the current
`SampledSurface3`, tracks the cursor's physical-pixel position, and performs
hover inspection. The window title displays scientific coordinates/scalar
information over the surface and indicates when no surface is under the
cursor. Left-click emits pixel and viewport dimensions followed by explicit
HIT/MISS/ray-error diagnostics. Successful hits include world position,
continuous domain and grid coordinates, scalar value, source cell, triangle,
and barycentric weights. Hover does not flood stderr. Amplitude changes
replace the retained scientific surface after updating renderer buffers.

User-reported Windows runtime validation (2026-11-08):
- Explorer launched on AMD Radeon(TM) Graphics using Vulkan.
- Moving over the blue surface showed coordinates/scalar information in the
  window title; moving off the surface showed the no-surface state.
- Two clicks outside the visible surface returned MISS.
- Three clicks at different visible surface locations returned HIT and
  reported plausible world/domain/grid coordinates, scalar values, source
  cells, triangle indices, and barycentric weights.
- The observed interaction confirms the complete physical-pixel -> camera
  ray -> mesh intersection -> sampled scientific data -> UI/diagnostic path.

User-reported automated validation on Windows (2026-11-08):
- `cargo fmt --all -- --check`: passed after running `cargo fmt --all`.
- `cargo check --workspace`: passed.
- `cargo test --workspace`: 84 passed, 0 failed (79 core, 1 Explorer,
  4 renderer).

Additional user-reported runtime validation:
- Picking remained correct after changing amplitude, including returning to
  the flat A=0.00 surface; hits then reported scalar values of zero.
- Picking remained correct after resizing, minimizing, and maximizing the
  Explorer window.

The formatter changed ten tracked Rust files. Copilot inspected the complete
working-tree diff and reported formatting-only changes: no changes to
calculations, conditions, public API behavior, coordinate conventions,
error handling, or tests. `git diff --check` reported no whitespace errors.
The user committed this cleanup locally as `d545466`
(`style: format workspace`), leaving the working tree clean. This local
commit was one commit ahead of `origin/main` at the time of reporting.

The pre-sync stash `stash@{0}` remains untouched. Inspection found it to be
an older pre-picking snapshot; it omits `crates/svf-core/src/ray.rs`, has an
older Explorer without picking integration, and contains stale milestone
documentation. No stash content should be restored over the validated M5.3
implementation. Keep the stash until its eventual disposal is an explicit,
separate decision.


## M6.1 — Scientific field semantics

Implementation committed to `main`; user-reported Windows workspace validation completed (94 tests passed).

New module: `crates/svf-core/src/fields.rs`, exposed as
`svf_core::fields`.

Public API:
- `FieldAssociation::{Point, Cell, Dataset}` distinguishes values attached
  to points, logical cells, or the dataset as a whole.
- `FieldMetadata` carries a non-empty field name and optional descriptive
  units. Units are labels only; no conversion or dimensional analysis occurs.
- `ScalarField` stores ordered `SampleValue` entries and preserves
  finite/non-finite classifications.
- `VectorField3` stores ordered `VectorValue3` entries with explicit
  `VectorBasis3::World` basis.
- `VectorValue3` classifies each component independently and converts to a
  `WorldVector3` only when all components are finite.
- `FieldError` reports invalid metadata and cardinality failures.
- `SampleValue::from_f64` is now public for consistent scalar classification.

Association rules:
- Point fields are validated against dataset point count.
- Cell fields are validated against logical dataset cell count, not generated
  render-triangle count.
- Dataset-associated fields contain exactly one value.
- Point/cell fields can be constructed before attachment; their cardinality
  is checked when dataset counts are available.
- No implicit interpolation or conversion between associations occurs.

The increment deliberately does not add a dataset container, field registry,
unit conversion, local vector coordinate frames, visualization mapping, or
renderer changes. Design details and exclusions are documented in
`docs/fields.md`.

Tests are included in `fields.rs`. User-reported Windows workspace
validation completed: 89 core, 1 Explorer, and 4 renderer tests passed.
Formatting status was not newly confirmed for this increment.


## M6.1 — Scientific field semantics: local validation update

The user reported a successful Windows workspace test run after pulling M6.1:
- `svf-core`: 89 passed.
- `svf-explorer`: 1 passed.
- `svf-render`: 4 passed.
- Total: 94 passed, 0 failed.

The working tree contains formatting-only changes in
`crates/svf-core/src/fields.rs` and `crates/svf-core/src/lib.rs`. Inspection
confirmed no behavior/API/test changes and no whitespace errors. The user
requested that formatting not become a recurring cleanup task. Preserve these
changes; do not run a formatter or create a formatting-only commit. The
`LF will be replaced by CRLF` messages are Git normalization warnings.

## M6.2 — Dataset containers and attribute attachment

Implementation was developed on branch `m6-2-dataset-attachment` from
M6.1 commit `475bb0f` and has since been merged into `main` through PR #1
(merge commit `9b4fc41`). The user's local `main` branch and its
formatting-only stash remain separate until the user syncs the merge.

New module: `crates/svf-core/src/dataset.rs`, exposed as
`svf_core::dataset`.

Public API:
- `DatasetGeometry` variants: `PointSet3`, `Polyline3`,
  `TriangleMesh3`, and `StructuredGrid2D`.
- `point_count()` returns source point/sample count.
- `cell_count()` returns logical source-cell count: zero for point sets,
  consecutive segments for polylines, source triangles for triangle meshes,
  and grid rectangles for structured grids.
- `DatasetField::{Scalar, Vector3}` stores supported field families.
- `Dataset` owns immutable source geometry and insertion-ordered fields.
- `add_scalar_field` and `add_vector_field3` validate association
  cardinality before insertion.
- Field names are unique across scalar and vector fields.
- `field`, `scalar_field`, and `vector_field3` provide name lookup.
- A failed attachment does not mutate the dataset.

Important source/render distinction: each structured-grid rectangle is one
logical cell, even though the current surface generator may emit two triangles
for it. Dataset cell-field cardinality is checked against rectangles, not
render triangles.

Geometry is immutable through the dataset API. Geometry changes create a new
dataset; field remapping/transfer is deferred. Unstructured grids, time
series, field interpolation, and renderer integration are not part of M6.2.

Nine focused unit tests are included in `dataset.rs`; all nine passed in the
user-reported Windows workspace run. No formatter was run for M6.2. The
feature branch included the M6.1 implementation from remote main. The user's
local M6.1 formatting-only changes remain preserved in a separate stash.


## M6.2 — Dataset containers and attribute attachment: merged

M6.2 is implemented, reviewed, user-validated on Windows, and merged into
`main` through PR #1 on 2026-11-08.

- PR: https://github.com/Aine-dickson/sciviz/pull/1
- Merge commit: `9b4fc41`
- Feature head: `dfb27c9`
- Dataset API: `crates/svf-core/src/dataset.rs`, exposed as
  `svf_core::dataset`.
- The branch was 9 commits ahead and 0 behind `main` at review time.
- The PR became mergeable and GitHub confirmed successful merge.
- No GitHub Actions/check-run results were configured for the feature commit;
  automated validation is the user's local Windows report.

User-reported Windows validation (2026-11-08):
- `cargo check --workspace`: passed.
- `cargo test --workspace`: 103 passed, 0 failed
  (98 core, 1 Explorer, 4 renderer; doc tests passed).
- All nine dataset-specific tests passed.
- No formatter was run for M6.2.

Review confirmed the dataset API preserves source topology semantics,
association cardinality, unique field names, insertion order, and
failure-atomic field attachment. The geometry accessors expose immutable
references, so topology cannot be mutated through `Dataset`.

The user subsequently reported completing the local switch to `main` and
fast-forward pull after the merge. Exact command output was not included.
Do not overwrite or blindly pop the preserved formatting stash. The M6.1
formatting-only changes remain separate and must be handled deliberately if
they are ever reapplied.

## M6.3 — Unstructured topology foundations: merged and validated

M6.3 was implemented on `m6-3-unstructured-topology`, reviewed, validated
on Windows by the user, and merged through PR #2.

- PR: https://github.com/Aine-dickson/sciviz/pull/2
- Merge commit: `fbe8a7bbb630b69fd04c1817ee706f3ea73886dc`.
- Feature head: `d7870124aac4f57a92186ecb3c2b114a846251e5`.
- Design decisions: `docs/unstructured-topology.md`.
- Implementation: `crates/svf-core/src/unstructured.rs`.
- `UnstructuredGrid3` stores ordered points and CSR connectivity with
  explicit `CellType3` values.
- Supported linear cells: `Line2`, `Triangle3`, `Quad4`,
  `Tetrahedron4`, and `Hexahedron8`.
- Dataset geometry and point/cell field cardinality integrate with the
  unstructured source counts.
- Structural connectivity is validated; geometrically degenerate cells are
  retained and cell orientation is not certified.

User-reported Windows validation (2026-11-08):
- `cargo check --workspace`: passed.
- `cargo test --workspace`: 117 passed, 0 failed
  (112 core, 1 Explorer, 4 renderer).

The user synchronized local `main` successfully at merge commit
`fbe8a7b`; the reported working tree was clean and matched `origin/main`.
M6.1 formatting-only stashes remain untouched. Do not run formatting
routines or restore either stash blindly.

## M6.4 — Unstructured volume boundary extraction: locally validated

M6.4 was implemented on `m6-4-volume-boundary` from merged M6.3
commit `fbe8a7bbb630b69fd04c1817ee706f3ea73886dc`, validated on Windows,
and merged through PR #3 on 2026-11-08.

- PR: https://github.com/Aine-dickson/sciviz/pull/3
- Merge commit: `d4e78c1bb73c44945ee4258d87e5f69e477a7a94`.

Design: `docs/volume-boundary-extraction.md`.

Implementation:
- `crates/svf-core/src/unstructured_topology.rs` adds
  `UnstructuredVolumeTopology3`, `VolumeFace3`,
  `VolumeFaceIncidence3`, `VolumeFaceVertices3`, and
  `VolumeFaceClassification`.
- `UnstructuredGrid3::volume_topology()` derives unique faces from
  `Tetrahedron4` and `Hexahedron8`; lower-dimensional cells are ignored.
- Face identity uses sorted point IDs, with triangle/quad arity kept distinct.
  Coordinate-coincident faces with different point IDs do not merge.
- Original local face order, source cell IDs, and local face indices are
  retained per incidence.
- Unique faces use deterministic canonical-key ordering; incidences retain
  source cell/local-face order.
- Incidence counts classify faces as boundary (1), interior (2), or
  non-manifold (>2). No geometric watertightness or outward-orientation
  claim is made.

User-reported Windows validation (2026-11-08):
- `cargo check --workspace`: passed.
- `cargo test -p svf-core`: 125 passed, 0 failed.
- `cargo test --workspace`: 130 passed, 0 failed
  (125 core, 1 Explorer, 4 renderer).
- The user merged PR #3 on GitHub. Local `main` synchronization after the
  merge is the next step.

No formatter was run. Continue preserving the user's formatting-only stash;
do not run formatting routines as cleanup.



## M6.5 — Unstructured boundary surface representation (in progress)

Branch: `m6-5-boundary-surface`, based on synchronized `main` at
`295c53b`.

The design contract is recorded in `docs/boundary-surface.md`. M6.5 will
convert M6.4 boundary-face incidence into a compact graphics-independent
surface while keeping the source volume grid authoritative.

Design decisions:
- Include only faces classified as boundary (one incident volume cell).
  Interior faces are omitted; non-manifold faces remain queryable through
  M6.4 topology but are excluded from the extracted boundary surface.
- Preserve triangle and quad faces as distinct polygon types. No core
  triangulation or diagonal choice.
- Compact referenced source points once, in ascending source point-ID order.
  Store f64 world positions plus a parallel compact-to-source point-ID map.
- Preserve deterministic M6.4 face order and each boundary face's sole
  incidence local vertex cycle.
- Preserve source topology face index, source cell index, and local face index.
- Treat local face cycles as reference order only; no outward orientation is
  certified because source cell orientation is not certified.
- Do not copy/transfer fields, mutate the grid, or integrate the renderer.

No M6.5 implementation or local validation is claimed yet. Next: implement the
contract in a new `svf-core` module, add focused tests, then ask the user to
pull and validate on Windows. Do not run formatting routines or disturb
existing stashes/local formatting changes.


M6.5 implementation is now present on the branch:
- New `crates/svf-core/src/boundary_surface.rs`, exported by `lib.rs`.
- Public API: `BoundarySurface3`, `BoundarySurfaceFace3`,
  `BoundaryFaceVertices3`, and `BoundarySurfaceError`.
- `UnstructuredGrid3::boundary_surface()` derives compact boundary geometry.
- Thirteen unit tests cover tetrahedra, hexahedra, adjacent cells, mixed
  arities, non-manifold exclusion, lower-dimensional cells, compact point
  mapping, provenance/local cycles, determinism, source immutability,
  coordinate identity, empty input, and geometric degeneracy.
- Compact source point IDs are ascending and unique; face order follows M6.4.
  Quads remain quads. Only one-incidence faces are included.
- No formatter, local build, or local test has been run for M6.5. Windows
  validation remains pending from the user.


M6.5 validation and merge completed (2026-11-08):
- PR #4 merged by squash; merge commit `e2571e649b601e7ae4e1d489e5ce1049cc77dcd0`.
- User confirmed every test passes on their Windows environment, including the M6.5 validation run.
- No formatter was run.
- Next action: synchronize local `main`; M6.5 is complete.
