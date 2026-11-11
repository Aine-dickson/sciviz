# Scientific Visualization Framework — Handoff

Last updated: 2026-11-11

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

Public API is exposed from `svf_core::sampling`:- `sample_scalar_1d(domain, spec, function)` evaluates a caller-provided  `FnMut(f64) -> f64`.
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
  triangle to its source grid cell for later inspection and picking.- Output allocations are reserved fallibly and vertex indices are checked
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
The camera exposes`Camera3::ray_from_viewport(pixel_x, pixel_y, width, height)`. It accepts
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
validation completed: 89 core, 1 Explorer, and 4 renderer tests passed.Formatting status was not newly confirmed for this increment.


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
- `cargo check --workspace`: passed.- `cargo test -p svf-core`: 125 passed, 0 failed.
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


## M6.6 — Provenance-preserving boundary surface triangulation (complete; merged to main 2026-11-08)

Branch: `m6-6-surface-triangulation`, based on current `main` after M6.5.

Design contract: `docs/surface-triangulation.md`.

M6.6 derives an independent triangle-only surface from M6.5
`BoundarySurface3`. Triangles pass through unchanged. Quads split into two
triangles using a deterministic diagonal selected by lexicographically
comparing sorted endpoint pairs of original source point IDs. Both split
patterns preserve the source local cycle. Every output triangle retains
source boundary-face, source volume-cell, source local-face, and per-face
triangle ordinal provenance. The compact points and source-point mapping
remain unchanged.

No outward orientation certification, geometry-dependent diagonal choice,
quality filtering, field transfer, or renderer integration is in scope.


M6.6 implementation is now present on the feature branch:
- New `crates/svf-core/src/surface_triangulation.rs`, exported from
  `svf-core`.
- Public API: `TriangulatedSurface3`,
  `TriangulatedSurfaceTriangle3`, and
  `BoundarySurface3::triangulate()`.
- Source triangles pass through unchanged. Quads split using the
  lexicographically smaller sorted pair of original source point IDs.
- The output preserves compact f64 points and source point IDs.
- Each triangle records both `boundary_face_index` (index into the M6.5
  surface face array) and `source_face_index` (original M6.4 topology face
  index), plus source cell, local face, and per-face triangle ordinal.
- Ten focused unit tests use public UnstructuredGrid3 construction and
  boundary extraction to exercise tetrahedra, hexahedra, both diagonal
  choices, mixed cell types, provenance, determinism, immutability, and
  geometric degeneracy.
- User validated on Windows: `cargo check --workspace` passed;
  `cargo test -p svf-core` passed (148 tests); `cargo test --workspace`
  passed (153 tests: 148 core, 1 Explorer, 4 renderer). Doc-tests passed.
- No formatter was run.
- Code/API review completed; PR #5 merged to `main` on 2026-11-08. User synchronized local `main` at `957d50e` with a clean working tree.


## M7.1 — Unstructured boundary-surface rendering (complete; merged to main 2026-11-08)

Branch: `m7-1-unstructured-rendering`.

Design contract: `docs/unstructured-rendering.md`.

PR #6 merged to `main` on 2026-11-08:
https://github.com/Aine-dickson/sciviz/pull/6

Merge commit: `1d34ee114191e8a98bcfb3d1611812f66520ef83`
(feature head: `29a713088d4e611d5c1753e4a7903fbf18ac6258`).

The approved pipeline is
`UnstructuredGrid3 → BoundarySurface3 → TriangulatedSurface3 → TriangleMesh3 → SceneRenderer`.
The mixed fixture contains one disconnected `Hexahedron8` and one
`Tetrahedron4`: 2 source cells, 10 boundary polygons (6 quads and 4
triangles), 12 compact boundary points, and 16 render triangles. The original
grid and triangulated provenance remain CPU-side; only the derived mesh is
sent to the renderer.

Windows validation was user-reported on 2026-11-08:
- `cargo check --workspace` passed without warnings.
- `cargo test --workspace` passed: 159 tests (148 core, 7 Explorer,
  4 renderer); doc-tests passed.
- `cargo run -p svf-explorer -- --unstructured` displayed both shapes.
- Default `cargo run -p svf-explorer` retained the parametric surface,
  click/hover inspection, and amplitude interaction.
- Both modes handled resize and minimize/restore correctly.
- No formatting routine was run.

M7.1 is complete. Preserve its scope boundaries: no field coloring,
unstructured picking UI, orientation repair, lighting, camera controls,
volume rendering, or file I/O was added in M7.1.

## M7.2 — Reusable scalar visualization (complete)

Merged to `main` through PR #7 on 2026-11-08.

Merge commit: `09c99da4eb4f36d09fad20facb987fde76d2e588`.

Feature branch: `m7-2-scalar-visualization`.

Design contract: `docs/scalar-visualization.md`.

The M7.2 core pipeline is implemented, merged, and user-validated on Windows.
Core implementation:
- `crates/svf-core/src/scalar_visualization.rs` provides
  `SurfaceProvenance3` adapters for structured sampled surfaces,
  unstructured triangulated surfaces, and identity-mapped triangle meshes.
- `MappedSurface3` expands each triangle into three corners and retains
  per-corner source point IDs plus per-triangle source cell IDs.
- Point, cell, and dataset scalar associations are mapped without mutating
  source geometry or fields.
- Automatic and explicit ranges, finite/non-finite handling, constant-field
  midpoint behavior, clamping, validated RGBA colors, and continuous color
  maps are implemented.
- User-reported Windows validation: `cargo test -p svf-core` passed
  (163 tests); `cargo check --workspace` passed without warnings.

Current renderer integration is implemented and user-validated on
Windows (2026-11-08):
- `svf-render` provides `SceneRenderer::new_with_mapped_surface`.
- Mapped positions and linear-light RGBA values convert to f32 at the GPU
  boundary; the mesh vertex layout and WGSL shader accept RGBA.
- The existing `SceneRenderer::new(window, &TriangleMesh3)` path remains,
  using its existing flat color with alpha 1.0.
- User-reported `cargo test --workspace`: 175 passed (163 core,
  7 Explorer, 5 renderer); doc-tests passed.
- User-reported `cargo check --workspace`: passed without warnings.
- User visually confirmed the existing unstructured flat-color mode still
  renders correctly and handles resize/minimize/restore.

Explorer scalar demonstrations are implemented on this branch:
- `--scalar`: structured grid with a point-associated height field.
- `--unstructured-scalar`: existing hexahedron/tetrahedron fixture with a
  cell-associated field.
- Both modes construct a source `Dataset`, create a provenance-aware surface,
  call the same `map_scalar_field` API, and pass the resulting
  `MappedSurface3` to the renderer.
- Source dataset, provenance, and mapped surface are retained CPU-side.
- Existing default parametric and `--unstructured` flat-color modes remain.
- Explorer unit tests cover mode selection and both mapped demo datasets.

Windows validation completed (user-reported, 2026-11-08):
- Pulled branch through `b104bf1`, including the extreme-range
  normalization fix.
- `cargo test -p svf-core scalar_visualization`: 16 passed.
- `cargo test --workspace`: 178 passed (164 core, 9 Explorer,
  5 renderer); doc-tests passed.
- `cargo check --workspace`: passed without warnings.
- `--scalar`: structured surface displayed a smooth visible gradient;
  yellow peaks and dark-blue troughs aligned with height variation.
  Hover inspection returned a finite sample; click inspection produced
  valid hits and expected misses outside the sampled surface.
- `--unstructured-scalar`: hexahedron and tetrahedron both rendered with
  distinct dark-blue and bright-yellow cell colors. Each cell was uniform
  across its visible triangles, confirming no within-cell color variation.
- Default parametric and original `--unstructured` flat-color modes still
  rendered and behaved as before.
- All four modes handled resize and minimize/restore consistently.
- Regression test confirms normalization remains finite and correctly
  maps endpoints and midpoint for extreme finite ranges, including
  [-f64::MAX, f64::MAX].
- No formatting routine was run.

M7.2 implementation, automated Windows validation, numerical edge-case
validation, and visual acceptance are complete. PR #7 is merged to `main`
(commit `09c99da`). The feature branch is no longer the active work branch.

Do not add implicit interpolation for generated points without explicit
source weights. Keep scalar range calculation, palette interpolation, and
field association lookup in `svf-core`; the renderer must remain a consumer
of mapped geometry and colors.


## M7.3 — Vector field visualization (complete; merged to main 2026-11-08)

Design contract: `docs/vector-field-visualization.md`.

PR #9 merged to `main`; merge commit:
`23c33fb855ffa2d52749baf66c1fb23eed53d19c`.

Implemented:
- Graphics-independent point/cell source resolution with provenance.
- Robust vector magnitude, direction, and explicit validity status.
- f64 arrow glyph construction with explicit magnitude-scaled and
  uniform-length policies.
- Renderer conversion at the f64-to-f32 boundary and depth-aware drawing.
- Explorer `--vectors` point-associated demonstration with 49 vectors.
- Source dataset and glyph results retained CPU-side.

User-reported Windows validation (2026-11-08):
- `cargo test --workspace`: all tests passed.
- `--vectors` displayed the undulating surface and yellow glyphs; hover
  sample, grid, and colored axes remained functional.
- Resize, minimize, and restore were consistent with the other modes.
- Some glyphs were difficult to distinguish because they are short and
  distributed fairly densely. This feedback informs M7.4; do not alter the
  validated M7.3 behavior without a focused change.

Coverage note: M7.3 core supports point- and cell-associated vectors.
The Explorer demonstrates point association through `--vectors`; M7.4
adds the structured cell-associated demonstration through `--cell-vectors`.

## M7.4 — Structured cell-associated vector visualization (complete; merged 2026-11-08)

PR #11 merged to `main`; merge commit:
`082fc5e18db8a7e40e1655dff70950af2fcc1e03`.

Design contract: `docs/cell-vector-visualization.md`.
Implemented:
- `--cell-vectors` uses a 9 × 9 structured graph grid (64 logical cells).
- Dataset geometry and displayed surface come from the same  `StructuredGrid2D` instance.
- A cell-associated, world-basis `VectorField3` has 64 deterministic values.
- Each glyph anchor is the arithmetic mean of its source cell's four
  graph-surface corner positions; this is not an area-weighted centroid.
- Glyphs use an explicit 1.0-world-unit uniform length.
- Source dataset and glyph result remain CPU-side; the existing
  depth-aware prepared-glyph renderer path is reused.
- Tests cover field/grid cardinality, stable source-cell IDs, arithmetic
  anchors, display length, and CLI mode selection.

User-reported Windows validation (2026-11-08):
- Workspace tests passed.
- Glyph placement appeared structured over the surface; grid and colored axes
  remained visible.
- Yellow glyphs and arrow tips were visible, though many were small/crowded
  and their directions were not consistently easy to distinguish.
- Depth occlusion could not be conclusively assessed from the fixed camera.
- Resize/minimize/restore behavior remained consistent.
- Explorer does not yet implement camera orbit, pan, or zoom. Mouse dragging
  has no camera effect; do not assume those controls exist.

Validation limitation: the automated tests establish exact source-cell
anchors and glyph geometry, but the visual occlusion check remains open until
camera navigation is available. M7.5 addresses that Explorer inspection
limitation without changing scientific geometry or glyph placement.

Unstructured volume-cell glyphs remain excluded: arithmetic cell centers lie
inside opaque volume boundaries and can be correctly occluded. A separate
visibility/representation design is needed for useful volume-cell glyphs.

## M7.5 — Interactive 3D camera navigation (complete; merged 2026-11-08)

Design and implementation contract: `docs/camera-navigation.md`.

PR #13 merged to `main`; merge commit:
`32002a3cbce2a3c03755cbda67dc8b759de1892d`.

Implemented:
- `svf-core` exposes input-independent f64 orbit, pan, and zoom operations
  through `camera_navigation`, with validated configuration and explicit  numerical limits.
- Orbit rotates around the current target, preserves camera-target distance,
  uses +Y world up, and clamps elevation away from the poles.
- Pan translates camera position and target together in the camera plane;
  the operation is perspective-camera only.
- Zoom changes camera-target distance exponentially and clamps it to the
  configured minimum and maximum.
- Default navigation configuration: minimum distance 1.0, maximum distance
  80.0, pole margin 0.02 radians, orbit sensitivity 0.006 radians/pixel,
  and zoom exponent sensitivity 0.12 per wheel unit.
- `svf-render` exposes validated `SceneRenderer::set_camera`; it validates
  the GPU uniform before updating the stored camera and uniform, keeping
  rendering and picking camera state synchronized.
- Explorer maps left drag to orbit, middle drag to pan, and wheel input to
  zoom. A drag does not also trigger click inspection.
- Camera controls work across all six existing modes; scientific geometry,
  datasets, and glyph positions are not transformed.

User-reported Windows validation (2026-11-08):
- `cargo test --workspace`: passed.
- `cargo check --workspace`: passed; the only warning was the pre-existing
  `VectorSource3::new` dead-code warning in `svf-core`.
- Orbit, pan, zoom, click inspection/picking synchronization, and
  resize/minimize/restore worked in `--cell-vectors`.
- Navigation also worked in default, `--vectors`, `--scalar`,
  `--unstructured`, and `--unstructured-scalar` modes.
- The user reported all remaining mode checks passed.
- Multi-angle inspection of the M7.4 cell-vector visualization is now
  available; no camera-driven geometry or glyph-depth override was introduced.

M7.5 implementation, automated workspace tests, compilation, and user-reported
Windows runtime validation are complete. PR #13 is merged to `main`.


## M8.1 — Tetrahedral isosurface extraction (complete; merged 2026-11-08)

Design and implementation contract: `docs/tetrahedral-isosurface.md`.

- Core PR #16 merged to `main`; merge commit
  `08c69cee2aa132b12e4d79e52f71f8b69b410858`.
- Explorer PR #17 merged to `main`; merge commit
  `f4d7ade5d45c109e40580b095ae6af9724621b56`.

Implemented:
- `svf-core::isosurface::extract_tetrahedral_isosurface` extracts a
  deterministic triangle surface from a point-associated scalar field on
  `Tetrahedron4` cells in an `UnstructuredGrid3`.
- Uses the documented half-open classification: values strictly below the
  isovalue are low; equal or above are high.
- Reuses canonical source-edge intersections and exact source-point
  intersections; interpolation and output positions remain f64.
- Preserves one source-point/source-edge provenance record per output vertex
  and one source-cell ID per output triangle.
- Reports unsupported cell types and skipped cells with explicit reasons.
- Orients triangles toward increasing scalar values and selects quad splits
  deterministically.
- Resolves exact-isovalue shared faces through tetrahedral face adjacency,
  avoiding duplicate coincident triangles.
- Explorer adds deterministic `--isosurface` mode using two adjacent
  tetrahedra and retains both source dataset and extraction result CPU-side.
- Existing Explorer modes and renderer mesh path remain unchanged.

User-reported Windows validation (2026-11-08):
- Focused Explorer isosurface test passed after correcting fixture assertions.
- `cargo test --workspace`: all tests passed (200 core, 15 Explorer,
  7 renderer; doc-tests passed).
- `cargo check --workspace`: passed.
- `cargo run -p svf-explorer -- --isosurface`: extracted surface rendered
  correctly; behavior remained consistent, including camera navigation and
  window interactions.
- The only reported compiler warning remains the pre-existing
  `VectorSource3::new` dead-code warning in `svf-core`.

M8.1 core extraction, Explorer integration, automated Windows validation,
and visual acceptance are complete.

## M8.2 — Provenance-based scalar transfer onto derived isosurfaces (complete; merged 2026-11-09)

Design contract: `docs/isosurface-field-transfer.md`.

- Core PR #20 merged to `main`; merge commit
  `531bd64da3f57dbe20c37dad8b591874bc625043`.
- Renderer/Explorer PR #21 merged to `main`; merge commit
  `ccc84e0b72fb1cedbaa6a159ecc2d5c384b2a917`.

Implemented:
- `svf-core::isosurface_field_transfer` transfers point-associated scalar
  values to isosurface vertices using source-point/source-edge provenance.
- `TransferredScalarField3` owns the exact `Isosurface3`, source metadata,
  and transferred values in mesh-vertex order.
- `MappedIsosurface3` owns the transferred field, one RGBA color per unique
  derived vertex, and the resolved scalar range. Existing `MappedSurface3`
  remains unchanged.
- Transfer validates the unstructured source dataset, point association,
  field cardinality, provenance count and bounds, canonical edge endpoint
  ordering, finite interpolation parameter in [0, 1], and triangle source
  cell IDs.
- Source-point values and edge endpoints are copied exactly. Interior
  interpolation preserves NaN and signed-infinity semantics and uses
  sign-aware f64 arithmetic for finite values.
- `ScalarRange::from_values` supports automatic ranges over transferred
  values; explicit range and invalid-color behavior reuse scalar
  visualization semantics.
- Renderer exposes
  `SceneRenderer::new_with_mapped_isosurface`. It consumes prepared mesh
  positions, indices, and colors only; f64-to-f32 conversion remains at the
  renderer boundary.
- Explorer adds `--colored-isosurface` with a distinct point-associated
  `temperature` field and Viridis mapping. The original `--isosurface`
  mode remains unchanged. Source dataset, extracted isosurface, and mapped
  result are retained CPU-side.

User-reported Windows validation (2026-11-09):
- PR #20 core validation: 226 tests passed (204 core, 15 Explorer,
  7 renderer).
- After renderer/Explorer integration, PR #21 validation: 228 tests passed
  (204 core, 17 Explorer, 7 renderer).
- The user visually confirmed that `--colored-isosurface` output matched
  the expected mapped-color rendering.
- The user explicitly authorized merging PR #21 after validation.
- A `cargo check --workspace` result was not included in the final PR #21
  validation report and is therefore not recorded as passed here.

M8.2 implementation, user-reported Windows test validation, visual
acceptance, and merge are complete.

## Current project state and next planning step

M0–M10.1 are implemented and merged to `main`. The next milestone has not
yet been selected. Before implementation, review the current architecture,
remaining design constraints, and desired scientific workflows; then define
one focused increment with an explicit design contract and testable acceptance
criteria. Do not infer or silently begin an M8.3 scope.


## M9.1 — VTK legacy ASCII unstructured-grid import (complete; merged 2026-11-09)

Design contract: `docs/vtk-legacy-ascii-import.md`.

- PR #22 design contract and PR #23 implementation are merged to `main`.
- Implementation PR: https://github.com/Aine-dickson/sciviz/pull/23
- Merge commit: `efa1cdbd59c09ca97a67a7e674f5c5483cc948c8`.
- Feature head: `fd48c4b10900f19558eca59d681c7fde6cb3068f`.

Implemented in graphics-independent `svf-core`:
- Public API: `parse_legacy_ascii_unstructured_grid(&str) -> Result<Dataset, VtkLegacyError>`.
- Parses the accepted VTK legacy ASCII `UNSTRUCTURED_GRID` subset and preserves supported mixed-cell topology, point/cell order, and field associations.
- Supports VTK LINE, TRIANGLE, QUAD, TETRA, and HEXAHEDRON cells.
- Supports point/cell scalar and 3-component vector arrays, declared numeric conversions, non-finite field values, optional scalar metadata, and inline lookup-table definitions.
- Validates the physical file envelope, record order, declared counts, connectivity, indices, numeric types (including empty arrays), duplicate sections/field names, and unsupported records; returns categorized errors without partial datasets.
- Binary files, unsupported cell types/attributes, dataset-associated fields, and Explorer file loading remain out of scope. Explorer loading is deferred to M9.2.

User-reported Windows validation on the final feature head:
- `cargo test -p svf-core vtk_legacy`: 12 passed.
- `cargo test --workspace`: 240 passed (216 core, 17 Explorer, 7 renderer); doc-tests passed.
- `cargo check --workspace`: passed.
- The only warning was the pre-existing `VectorSource3::new` dead-code warning.
- No Explorer runtime changes were introduced or required.

M9.1 implementation, review, final Windows validation, and merge are complete. The next increment is M9.2: design and implement user-facing VTK file loading in Explorer, without moving parsing or scientific data ownership out of `svf-core`.


## M9.2 — Explorer VTK dataset loading (complete; merged 2026-11-09)

Design contract: `docs/explorer-vtk-loading.md`.

- PR #25: https://github.com/Aine-dickson/sciviz/pull/25
- Merge commit: `e9d57463a11dc89a6c43b8ba2677fdb79517a174`.
- Implementation branch: `m9-2-explorer-vtk-loading`.

Implemented:
- Explorer accepts `--vtk <path>` and optional `--vtk-scalar <field-name>`.
- Parses CLI arguments and loads UTF-8 VTK files before creating the event
  loop/window; delegates parsing to the unchanged `svf-core` M9.1 parser.
- Extracts volume-cell boundaries, triangulates them, and renders the resulting
  surface. Reports lower-dimensional source cells omitted from this initial
  view and rejects inputs without a renderable volume boundary.
- Optional point- or cell-associated scalar mapping reuses the existing
  provenance-aware scalar visualization pipeline, automatic finite range,
  and Viridis color map.
- Computes a perspective camera from imported mesh bounds and applies it
  through the existing renderer camera API.
- Retains the source Dataset, triangulated surface, source provenance,
  TriangleMesh3, and optional mapped surface on the CPU.
- Existing demo modes and default launch remain unchanged.
- Includes a committed tetrahedron-plus-line fixture at
  `apps/explorer/examples/tetra-with-fields.vtk`.

User-reported Windows validation (2026-11-09):
- `cargo test -p svf-explorer`: 26 passed.
- `cargo test --workspace`: 249 passed (216 core, 26 Explorer, 7 renderer);
  doc-tests passed.
- `cargo check --workspace`: passed.
- Uncolored, point-associated `temperature`, and cell-associated `density`
  rendering were visually inspected. Point colors interpolate across the
  surface; cell colors remain uniform per source cell.
- Immediate startup camera framing was confirmed visible and centered before
  user interaction. Zooming fully out can move the small fixture out of view.
- Missing-file handling was verified to report a contextual error and exit
  nonzero before opening a window.
- The only reported compiler warning remains the pre-existing
  `VectorSource3::new` dead-code warning.

M9.2 implementation, Windows automated validation, visual acceptance, and
merge are complete. The next milestone has not been selected; do not infer
or silently begin an M9.3 scope.
## M9.3 — Isosurface extraction from imported VTK fields (complete; merged 2026-11-09)

Design contract: M9.3 Explorer isosurface design PR #26 (merged).

- PR #27: https://github.com/Aine-dickson/sciviz/pull/27- Merge commit: `b05f59371a7023460ac60b22f6787839ef23a4e4`.
- Feature head: `93e348d1ea300b41e8666ff74fd2eb0273aa28b4`.

Implemented:
- Add `--vtk-isosurface <field-name>` and `--vtk-isovalue <finite-f64>`
  for tetrahedral isosurface extraction from an imported VTK dataset.
- Add optional `--vtk-isosurface-color <point-scalar-field>` to color the
  extracted surface using an independent point-associated scalar field.
- Reuse M8.1 extraction/provenance and M8.2 scalar-transfer/mapping APIs.
- Retain source Dataset, Isosurface3 provenance, and optional
  MappedIsosurface3 CPU-side.
- Render a depth-tested overlay of unique undirected triangle edges.
- Fit the initial camera to the extracted surface; reject empty extraction
  before window creation and report unsupported/skipped cells.
- Preserve M9.2 VTK loading and existing demo workflows.
- Extend the committed VTK fixture with an independent point scalar
  `pressure` field.

User-reported Windows validation (2026-11-09):
- `cargo test -p svf-explorer`: 35 passed.
- `cargo test --workspace`: 258 passed (216 core, 35 Explorer,
  7 renderer); doc-tests passed.
- `cargo check --workspace`: passed.
- Runtime command:
  ```powershell
  cargo run -p svf-explorer -- --vtk apps/explorer/examples/tetra-with-fields.vtk --vtk-isosurface temperature --vtk-isovalue 25 --vtk-isosurface-color pressure
  ```
- User visually confirmed both extracted triangles, the contrasting shared
  edge/mesh overlay, and color variation from the independent `pressure`
  field.
- The only reported compiler warning remains the pre-existing
  `VectorSource3::new` dead-code warning.

M9.3 implementation, Windows automated validation, visual acceptance, and
merge are complete. No next milestone is selected; define its scope and
design contract before implementation.

## M10.1 — Scientific inspection of imported surfaces (complete; merged 2026-11-09)

Design contract: `docs/imported-surface-inspection.md` (PR #28).

- Implementation PR #29: https://github.com/Aine-dickson/sciviz/pull/29
- Merge commit: `5baadb4d80da0ec1ad1810f8e1ca77015c610f5f`.

Implemented:
- Add graphics-independent `svf-core::surface_inspection` ray-hit APIs for
  imported boundary surfaces and tetrahedral isosurfaces.
- Resolve boundary hits to source cell, source face, local face, and source
  triangle identity; resolve isosurface hits to their original source cell.
- Evaluate point-associated scalar values barycentrically, cell-associated
  values from the exact source cell, and dataset-associated values directly.
- Evaluate isosurface fields through source-point/source-edge provenance before
  barycentric interpolation; retain field names, associations, units, and
  explicit non-finite classifications.
- Explorer hover updates the window title with representation, source cell,
  and world position. Click diagnostics report entity identity, triangle,
  ray distance, world position, barycentric weights, and scalar values.
- Preserve existing parametric inspection, camera gestures, and
  click-versus-drag behavior.

User-reported Windows validation (2026-11-09):
- `cargo test -p svf-core surface_inspection`: 4 passed.
- `cargo test -p svf-explorer`: 35 passed.
- `cargo test --workspace`: 262 passed (220 core, 35 Explorer,
  7 renderer); doc-tests passed.
- `cargo check --workspace`: passed.
- Boundary-surface runtime inspection resolved source cell 0 and its boundary
  faces; point-associated `temperature` and `pressure` values were
  barycentrically evaluated, and cell-associated `density` resolved to 7.0.
- Colored-isosurface runtime inspection resolved both triangles to source
  cell 0; `temperature` evaluated to approximately 25 at the isosurface,
  `pressure` was interpolated, and `density` remained 7.0.
- Camera orbit, pan, zoom, and click-versus-drag interactions remained correct.
- The only compiler warning was the pre-existing
  `VectorSource3::new` dead-code warning.

M10.1 implementation, automated Windows validation, runtime inspection,
interaction checks, and merge are complete. No next milestone is selected;
agree on the next scientific workflow and write its design contract before
implementation.


## M10.2 — Imported VTK point-vector visualization (complete; merged 2026-11-09)

Design contract: `docs/vtk-point-vector-visualization.md` (PR #30).

- Implementation PR #31: https://github.com/Aine-dickson/sciviz/pull/31
- Merge commit: `1fe8fa051dbeb1f61d374a56b9be8c1570938d66`.

Implemented:
- Add `--vtk-vectors <field-name>` for point-associated, world-basis VTK
  Vector3 fields on imported volume-boundary scenes.
- Reuse the existing core vector-source resolution, analysis, uniform-length
  glyph construction, and renderer paths; no new vector algorithm or renderer
  pipeline was introduced.
- Preserve original source point anchors, vector order, values, and the source
  Dataset. Zero/non-finite entries remain represented in the glyph set even
  when they do not produce displayable arrows.
- Derive uniform display length as 5% of the imported boundary bounding-box
  diagonal, with a positive floor. This communicates direction without
  implying that arrow length represents physical magnitude.
- Support simultaneous `--vtk-scalar` surface coloring and vector glyphs;
  reject unsupported associations and incompatible isosurface options.
- Extend the VTK fixture and Explorer tests for CLI, source identity,
  zero-vector behavior, scalar-plus-vector preparation, and invalid fields.

User-reported Windows validation (2026-11-09):
- `cargo test -p svf-explorer`: passed.
- `cargo test --workspace`: passed; exact test count was not included in the
  report.
- `cargo check --workspace`: passed.
- User confirmed the vector visualization rendered. The implementation was
  merged only after that local validation and visual confirmation.

M10.2 implementation, user-reported automated validation, runtime rendering,
and merge are complete. The M10.2 implementation is the end of the currently
selected general-purpose sequence. **Do not automatically select another
framework-wide milestone.** Future work must begin with the user's own
scientific, engineering, educational, or research use case: the concrete
problem, dataset/model, desired analysis, and required output. Build reusable
capabilities where they serve that use case, not to pursue feature parity or
hypothetical general-user demand. Preserve the framework's scientific
correctness and architecture constraints.


## E1 — Chemistry composition core (implementation complete; merged 2026-11-09)

Research direction and initial API proposal: PR #32 (`docs/experimental-chemistry-composition.md`, `docs/chemistry-composition-api.md`). Implementation: [PR #33](https://github.com/Aine-dickson/sciviz/pull/33).

- Merge commit: `46b134c2755b5ee5a815774ab1e035bafc602f70`.
- Add graphics-independent `svf_core::chemistry` with immutable element definitions for Hydrogen, Carbon, and Oxygen, plus validated extension definitions.
- Add distinct `AtomId`/`BondId`, atom instances, semantic single/double/triple bond orders, and finite `f64` molecular coordinates explicitly measured in ångströms.
- Add validated `Molecule::new`, preserving atom/bond order and rejecting duplicate identities, unknown endpoints, self-bonds, and duplicate undirected atom pairs.
- Provide read-only lookup and incident-bond accessors. Empty molecules and isolated atoms are structurally valid; chemical valence, charge, stereochemistry, and reaction simulation are intentionally not validated.
- Water and methane are the first domain examples. No renderer, Explorer, simulation, or generic user-defined component system was added.

User-reported Windows validation (2026-11-09): chemistry tests 16 passed; `svf-core` 236 passed; workspace 283 passed (236 core, 40 Explorer, 7 renderer); `cargo check --workspace` passed. Existing `VectorSource3::new` dead-code warning remains; `prepare_imported_scene_with_options` also emitted a dead-code warning. Neither blocked validation.

The initial chemistry structural model is complete. Next work should test whether this representation can produce useful scientific views and inspection for actual chemistry learning/teaching. Keep domain-defined examples first; only extract generic assembly mechanisms when multiple real structures demonstrate the need. Biology is a later generality check, not an automatic next feature.


## E2 — Chemistry molecular views (implementation complete; merged 2026-11-09)

- Design PR #34: https://github.com/Aine-dickson/sciviz/pull/34; design document: `docs/chemistry-molecular-views.md`; merge commit: `0b5b96ee4c99a323348a10d47464f572b8796e89`.
- Implementation PR #35: https://github.com/Aine-dickson/sciviz/pull/35; merge commit: `e7381a9c82bef10f625b57592da2f99f5ccf3555`.
- Adds graphics-independent `svf_core::chemistry_view` with `BallAndStickStyle`, `AtomGlyph3`, `BondAxis3`, `BondGlyph3`, and `MolecularView3`.
- `build_ball_and_stick_view` derives an ordered representation from an immutable E1 `Molecule`, preserving atom/bond identities, endpoints, and semantic bond order.
- Atom display radii, bond radius, and multiple-bond separation are explicit positive finite style parameters in ångströms. The built-in H/C/O style is illustrative, not a statement of empirical atomic radii.
- Single, double, and triple bonds derive one, two, and three deterministic parallel axes. Missing styles, invalid style values, coincident bonded positions, and non-finite derived geometry are rejected.
- Water and methane are covered, along with multiple-bond geometry, deterministic offsets, empty/isolated structures, invalid styles, overflow, provenance, and source immutability.
- Core remains graphics-independent. No renderer, Explorer, GPU, tessellation, simulation, or generic component authoring was introduced.

User-reported Windows validation (2026-11-09):
- Focused `chemistry_view` tests: 10 passed.
- `cargo test -p svf-core`: 246 passed.
- `cargo test --workspace`: 293 passed (246 core, 40 Explorer, 7 renderer).
- `cargo check --workspace`: passed.
- Existing dead-code warnings remain for `VectorSource3::new` and `prepare_imported_scene_with_options`; neither blocked validation.

E2 implementation, automated validation, and merge are complete. Renderer integration is not implied. The next step must be selected from an actual chemistry teaching/learning need and should establish what learners or teachers need to see, manipulate, inspect, and understand before adding presentation-layer functionality.

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

- Implementation: [PR #39](https://github.com/Aine-dickson/sciviz/pull/39). Squash merge commit: `849e11236cd85f0454f4f85dcf0d7a599ffa8830`.
- Added `svf_core::vsepr` with stable `ElectronDomainId`, explicit bonding-pair/lone-pair classifications, and a validated `VseprCentre` tied to an existing `Molecule`.
- Bonding domains preserve exact source `BondId` and neighbouring `AtomId`; lone-pair domains have no bond or second-atom identity.
- Validation rejects missing central atoms, duplicate domain IDs, unknown/non-incident bonds, incorrect neighbours, duplicate or omitted incident-bond mappings, unsupported domain patterns, and contradictory shape classifications.
- Only the approved four-domain patterns are supported: methane (4 bonding, 0 lone pairs → tetrahedral) and water (2 bonding, 2 lone pairs → bent). Double/triple bonds each count as one domain.
- Caller domain order is preserved. The semantic model does not infer geometry from molecular coordinates or claim measured bond angles.
- User-reported Windows validation: focused VSEPR tests passed; `cargo test -p svf-core` passed 259 tests; `cargo test --workspace` passed 306 tests (259 core, 40 Explorer, 7 renderer); `cargo check --workspace` passed. No failures.
- No schematic adapter, renderer, Explorer UI, Prismal/Mava integration, or general VSEPR prediction engine was added.

E3.2 implementation, user-reported automated validation, and merge are complete. The next step is not automatically a visualization feature: evaluate the learning case and decide whether the missing schematic electron-domain representation is actually needed for the intended teaching interaction.


## Current SVF-only chemistry evaluation — VSEPR schematic

- The user explicitly limited evaluation to SVF independently. Mava and Prismal are not part of this evaluation; no integration assumptions or requirements should be introduced.
- Design contract PR #40 (`docs/vsepr-schematic-view.md`) was accepted and merged; merge commit: `5dc9c28e087bec165f6ed9bd8d9369c22731e2e4`.
- The graphics-independent schematic adapter is implemented in `svf-core` as `vsepr_view`. It exposes `VseprSchematicPoint3`, `VseprDomainGlyph3`, and `VseprSchematicView3::from_centre`.
- It maps four validated tetrahedral electron domains to deterministic normalized dimensionless directions, preserving central atom, domain identity/classification, source bond/neighbour provenance, input order, and molecular-shape metadata.
- Scientific safeguards: schematic anchors are not molecular/world coordinates or ångström measurements; lone-pair markers are classifications, not electron-density surfaces; the schematic does not position neighbouring atoms or depict molecular bond angles.
- Implementation source is on `main`: `crates/svf-core/src/vsepr_view.rs`, exported from `svf-core/src/lib.rs`. PR #41 was closed as redundant because its implementation file had independently landed on `main`; the missing module export was added directly in commit `368f1bbf1f7f8503fa0cf7de4f47da53ce39d8fb`.
- User-reported Windows validation: focused `vsepr_view` tests 5 passed; `cargo test -p svf-core` 264 passed; `cargo test --workspace` 311 passed (264 core, 40 Explorer, 7 renderer); `cargo check --workspace` passed. Existing dead-code warnings remain for `VectorSource3::new` and `prepare_imported_scene_with_options`; no failures.
- **Next bounded task:** add a minimal standalone Explorer sample for methane and water, constructed directly from SVF's chemistry/VSEPR APIs. Show bonding-pair and lone-pair markers with a legend, and state molecular shape separately (methane tetrahedral; water bent). Do not place hydrogen atoms at schematic domain anchors or imply a tetrahedral H–O–H angle. Reuse the existing Explorer rendering path; avoid disproportionate UI/picking infrastructure.
- After the sample is locally validated, evaluate what SVF itself demonstrates and what remains missing. Do not scope Mava/Prismal integration or automatically expand VSEPR beyond the four-domain cases.


## E3.4 — Standalone VSEPR Explorer sample (complete; merged 2026-11-09)

- PR #42: https://github.com/Aine-dickson/sciviz/pull/42. Squash merge commit: `9deda0dd6aa6f54c2aa50d029b23302c4411efa2`.
- Adds `apps/explorer/src/vsepr_demo.rs`, constructing methane and water independently from SVF chemistry and VSEPR APIs, then adapting their `VseprSchematicView3` values into one static display mesh.
- Explorer flag: `--vsepr`.
- Left cluster is methane (CH₄): four bonding-pair domain markers; molecular shape labelled tetrahedral.
- Right cluster is water (H₂O): two bonding-pair and two lone-pair markers; molecular shape labelled bent.
- Marker convention: octahedral markers indicate bonding-pair domains; tetrahedral markers indicate lone-pair domains. A larger octahedral marker indicates each central atom. The renderer currently uses a single mesh color, so marker geometry rather than color distinguishes the domain classes; the legend is printed to the terminal and the window title identifies left/right molecules.
- No lines connect domain markers. They are not atom positions, molecular bonds, or molecular geometry. The schematic anchors are converted only into display positions for the sample; no Å interpretation is made. No H–O–H angle is depicted.
- This is a static, standalone SVF sample; no Mava/Prismal integration, picking, or lesson-player behavior.
- Focused Explorer VSEPR tests: 3 passed.
- User-reported Windows validation: `svf-core` 265 passed; Explorer 43 passed; renderer 7 passed; workspace total 315 passed; `cargo check --workspace` passed. No test failures.
- Runtime visual validation: user inspected five camera angles. Both groups remained visible and separate without obvious clipping or overlap. The larger central markers and surrounding domain markers were visible; water's lone-pair markers had distinct angular geometry, most apparent in angled views.
- Visual limitation recorded by the user: the sample does not show hydrogen atoms or molecular bonds, so molecular shapes are conveyed by the title/labels rather than independently apparent from the marker geometry. There are no connecting spokes or bond-angle annotations.
- Non-blocking compiler warnings: existing `VectorSource3::new` and `prepare_imported_scene_with_options` dead-code warnings, plus unused `methane` and `water` fields in `VseprDemo`.
- **Evaluation outcome:** the sample demonstrates the four-domain electron arrangement and distinguishes bonding-pair from lone-pair classifications, but it is not by itself a complete visual explanation of molecular shape. Do not add more geometry or expand VSEPR automatically; select the next SVF task only from a concrete scientific learning/use case.


## Next evaluation proposal — analytical heat-diffusion challenge (design PR pending review)

- Design document: `docs/analytical-heat-diffusion-challenge.md`.
- Branch: `analytical-heat-diffusion-challenge-design`.
- Scope is an SVF-only evaluation of existing capabilities using a manufactured
  3D heat-diffusion solution on coarse and fine tetrahedral meshes. It is not a
  heat-equation solver project.
- Unit-cube domain; exact temperature
  `T = exp(-3*pi^2*alpha*t) sin(pi*x) sin(pi*y) sin(pi*z)`,
  with `alpha = 1 m^2/s), `k = 1 W/(m K)`, and snapshots at zero and
  `ln(2)/(3*pi^2*alpha)` seconds. The later snapshot is exactly half the
  earlier one.
- Exact Fourier heat-flux vectors are supplied as independent point fields;
  SVF is not asked to compute gradients. A separate linear scalar probe
  `P=x+2y+3z` checks interpolation/transfer provenance.
- Coarse mesh: 5 points/axis, 125 points, 384 tetrahedra. Fine mesh:
  9 points/axis, 729 points, 3,072 tetrahedra. Both use a conforming
  six-tetrahedra-per-cube Freudenthal decomposition.
- Five point arrays: `temperature_t0`, `temperature_t1`,
  `probe_linear`, `heat_flux_t0`, `heat_flux_t1`.
- Evaluation exercises existing VTK import, tetrahedral boundary extraction,
  constant boundary scalar mapping, interior isosurface extraction, scalar
  transfer, point-vector glyphs, combined scalar/vector rendering, and
  source-aware inspection.
- The design document states exact counts, formulas, invariants, tolerances,
  CLI workflows, topology and convergence checks, scientific limitations,
  failure classification, and explicit non-goals.
- No fixture or implementation has been created yet. Do not claim local
  validation. Await user review of the challenge contract before generating
  fixtures.


## Analytical heat-diffusion challenge — fixture implementation in progress

- Design PR #43 accepted and merged; design merge commit
  `0999f67058fe332a067667933cb30419e39910b8`.
- Fixture implementation branch: `analytical-heat-diffusion-fixtures`.
- Added deterministic generator: `tools/analytical-heat-diffusion/generate.py`.
- Added independent VTK parser/verifier:
  `tools/analytical-heat-diffusion/verify.py`. It checks mesh topology,
  point ordering, all five analytical fields, snapshot scaling, boundary
  conditions, heat-flux direction, and coarse/fine isosurface residual.
- Added coarse and fine VTK fixtures under
  `fixtures/analytical-heat-diffusion/`, plus a README with units, formulas,
  reproduction instructions, and SVF commands.
- Coarse: 125 points, 384 tetrahedra, 192 exterior triangles.
- Fine: 729 points, 3,072 tetrahedra, 768 exterior triangles.
- Preliminary maximum continuous isosurface residuals at 0.2 K:
  coarse 0.150410743745 K; fine 0.0333587000791 K.
- The probe scalar is dimensionless using coordinates normalized by one metre.
  The boundary temperature is constant [0,0]; test the existing constant-field
  mapping behavior rather than expecting a boundary gradient.
- **Not yet locally executed or validated.** The next step is to pull this
  branch and run the generator/verifier before any SVF runtime evaluation.


## Next SVF evaluation — B1 human heart anatomy (design proposal)

- The user approved evaluating human heart anatomy as the next biology use case.
- Design contract: [B1 — Human heart anatomy and spatial relationships](docs/biology-heart-anatomy-challenge.md), branch `biology-heart-anatomy-design`.
- This is an SVF-only evaluation. Mava and Prismal are explicitly excluded.
- Primary source candidate: BodyParts3D 4.0 PART-OF tree. The official catalogue exposes concept/representation IDs, inclusion relationships, compound-organ element mappings, and a 99%-reduced OBJ polygon-mesh archive (listed as 62 MB). Required heart structures are present in the online concept list; exact archive membership, file mapping, units, axes, and mesh quality are not yet verified locally.
- Secondary reference: HuBMAP CCF / Human Reference Atlas heart reference objects in GLB format; expert-developed and organ-expert-approved. Use as independent reference, not a substitute for the separate structure inventory.
- BodyParts3D and HuBMAP CCF data are CC BY 4.0; preserve the exact required attribution and source release details for every dataset actually used.
- The design document includes the required inventory with FMA and BodyParts3D representation IDs, data-contract requirements, anatomical interpretation guardrails, acceptance criteria, reproducible evaluation views, and explicit non-goals.
- **Current state:** design proposal only. The archive has not been downloaded/audited locally; no fixture, conversion, importer, or SVF production code has been created. Do not claim source geometry availability solely from catalogue entries.
- Next step after design review: B1.1 source-data audit. Do not start fixture generation or implementation until the archive audit is complete and the final challenge contract is accepted.


## B1 — Human heart anatomy source audit (in progress)

- B1 design approved and merged through PR #45; revised primary source selection is under review in PR #46.
- The initial BodyParts3D 4.0 PART-OF candidate was not selected as primary: its four valve concepts map to FJ element meshes that overlap the chamber mappings, so it does not supply four separately selectable valve surfaces.
- Preferred primary candidate: Rodero et al. (2021), Virtual cohort of adult healthy four-chamber heart meshes from CT images. Zenodo DOI: https://doi.org/10.5281/zenodo.4590294.
- The cohort contains 20 CT-derived four-chamber meshes, approximately 1 mm average edge length, ASCII legacy VTK UNSTRUCTURED_GRID, coordinates in millimetres, cell-associated anatomical tags, point-associated UVC fields, and supplied ventricular fibre vectors. Case archives are 54.9–103.2 MB; proposed first case 01.tar.gz is 91.1 MB.
- Licence CC BY 4.0 is confirmed by the CEMRG dataset catalogue and PyAnsys Heart acknowledgements; preserve dataset authorship and cite the associated paper.
- The 24 cell labels include four chamber myocardium regions, aorta/pulmonary artery walls, four valve planes, and vessel/appendage inlets and borders. Valve planes are not leaflets/cusps; vessel inlets are not complete vessel tubes. Fibre directions are rule-based, and UVCs are mathematical coordinates.
- The cohort is described as healthy/asymptomatic based on clinical follow-up, not a general-population canonical model.
- BodyParts3D remains a secondary whole-body reference; its official coordinate system is mm, anatomical left X>0/right X<0, posterior Y>0/anterior Y<0, superior +Z. Correction: pulmonary trunk BP9912, not BP9911.
- HuBMAP CCF male/female whole-heart GLB models are optional whole-organ visual references. Layered Human's 119-part procedural heart (MIT original model assets, according to its repo) is an optional valve morphology reference only; it is not patient-derived and is a GLB surface model.
- The single-case Rodero archive has not yet been downloaded or locally inspected. No fixture or SVF production code has been created.
- Next: after accepting the revised source choice, download only case 01, audit VTK topology/fields/units/orientation, and test the existing SVF importer. Do not download the full cohort or implement missing capabilities before that audit.
- SVF-only evaluation. Mava and Prismal remain excluded.



## Camera zoom fix (PR #47) and scalar sentinel exclusions (PR #48)

- PR #47 (scale-aware camera zoom and clipping) merged at `fb17f4b`; the user
  confirmed navigation on the real Rodero heart.
- PR #48 adds `svf_core::scalar_visualization::map_scalar_field_with_exclusions`
  and repeatable Explorer `--vtk-scalar-exclude <finite f64>` (requires
  `--vtk-scalar`). Excluded values match by exact f64 equality, are omitted
  from the automatic range, and are drawn in the invalid-data color. Source
  fields are unchanged; `map_scalar_field` behaves as before.
- Use case: Rodero et al. (2021) case 01, working copy
  `~/Downloads/rodero2021/01-svf-order.vtk` (only CELLS/CELL_TYPES reordered;
  original `01.vtk` untouched). 493,151 points, 2,477,866 tetrahedra.
  `Z.dat` (ventricular apico-basal UVC) uses -10 for the 138,056 points outside
  the ventricular domain; valid range is [0, 1].
- User-reported tests on the branch: `svf-core` 269 passed, `svf-explorer`
  44 passed.
- Runtime validation (2026-11-10, release build, AMD Radeon/Vulkan, captured
  with PrintWindow and inspected directly): 329,952 boundary triangles in both
  runs. Without exclusion the ventricles were near-uniform yellow and the
  sentinel region dark blue (it read like a real low value). With `-10`
  excluded the ventricles span the full Viridis range (dark at the apex,
  yellow at the base), and the atria and great vessels show the invalid color.
  Geometry and framing are identical. Orbit across four angles and a
  zoom in, out and back sequence (reversible, with no disappearance or
  clipping) all behaved correctly.
- Known, non-blocking: the valid/excluded border is a one-triangle-wide blend
  band, because point colors are interpolated per vertex on the GPU (same
  behavior as for NaN). The blended colors there carry no data value.


## B1.2 — Ventricular fibre orientation (design approved; stage 1 in review)

- Design contract: `docs/heart-fibre-orientation.md`, PR #49 (branch
  `b1-2-ventricular-fibre-design`). Round 2 approved the direction. The
  revisions from rounds 2–4 are in the document (§11–§13). Round 4 approved
  the design. **Do not implement until
  the user authorises it after reviewing the diff.**
- Data hazard found in Rodero case 01: only tags 1 (LV) and 2 (RV) have real
  rule-based fibres and sheets. Tags 3–24 (628,774 cells) carry placeholders
  fibre=(1,0,0) and sheet=(0,1,0), which are unit length and orthogonal like
  the real data. Select by `ID` tag, never by value.
- Source rule (Zenodo 4590294): ventricular fibres rotate from 80° at the
  endocardium to -60° at the epicardium. Sheets are undocumented.
- Decisions not to revisit:
  - Glyph p_c ± (ℓ/2) f̂, with the exact source direction. It is not projected
    and only approximately tangent. Axial: |f·n|, no arrowheads.
  - The geometric triangle normal only. No smoothed normals, no
    orientation-dependent steps.
  - Thinning: world-origin cubic bins with a floor index; keep the lowest
    surface index (a tie-break, not a ranking). h = 4 mm and ℓ = 3 mm are
    provisional and configurable.
  - Four explicit degenerate counters (§4.6). The helix ones are
    diagnostic-only.
  - Use the existing line depth path first. No anchor offsets along normals.
  - RV: tangency only. No RV frame and no RV helix diagnostic.
  - svf-core tests are synthetic only, with no Rodero numbers.
- Verification plan (§7): `tools/rodero-fibre-audit/` (stdlib Python:
  `audit.py`, `compare.py`, `verify_glyphs.py`, committed
  `baseline/case01.json`). It derives its own boundary from the original
  `01.vtk` using sorted point-ID face incidence. The SVF example
  `fibre_diagnostic.rs` writes the same JSON schema plus `glyphs.csv`.
  - `verify_glyphs.py` recomputes the expected winner of every bin. The
    surface index is the rank of the sorted point-ID key, from the existing
    boundary/triangulation ordering contracts, and holds for all-tet meshes
    only. It fails on any missing, unexpected, wrong-winner or duplicate
    glyph, unless a near-face centroid (within 1e-9 mm) explains that
    specific discrepancy. Per-tag thinned glyph counts are compared exactly.
  - Provenance: `source_sha256` (original `01.vtk`; the diagnostic takes it
    through a required `--source`) must match. `input_sha256` (the file
    actually parsed) is kept but not compared. The baseline pins the hash.
    The README value is documentation only; no script reads it.
  - Record the audit's runtime and peak memory (`tracemalloc`). No NumPy
    unless those measurements show a need.
- Scratch baseline (not yet reproduced by the audit): 171,473 ventricular
  boundary triangles; 93.44% have θ > 84.26°; LV |α| median 75.8° (endo) and
  62.5° (epi). The audit becomes the reference once it exists.
- Stage 1 (branch `b1-2-vtk-inspect`): `crates/svf-core/examples/vtk_inspect.rs`
  replaces the deleted probes. Verified locally on 2026-11-11:
  - `01-svf-order.vtk`: 7 fields reported, with association, type and count.
    329,952 boundary faces, all triangles, 329,952 triangulated triangles,
    matching the scratch baseline. Parse took 7.4 s and boundary 8.8 s
    (release).
  - Original `01.vtk`: exits 1 with "expected CELLS record" (the known parser
    limitation).
  - Missing file: exits 1. No argument: exits 2.
  - `cargo test -p svf-core`: 269 passed.
- Next action: the user reviews the stage-1 PR. Then implement stage 2. The
  stages, one per reviewable PR (PR #49 merged as 198e568):
  in this order:
  1. vtk_inspect;
  2. audit plus committed baseline;
  3. svf-core operation plus synthetic tests;
  4. fibre_diagnostic, compare and verify_glyphs;
  5. renderer/Explorer plus visual validation.

  Establish fidelity (stages 2 and 4) before judging visuals.
