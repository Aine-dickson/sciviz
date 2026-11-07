# Scientific Visualization Framework — Handoff

Last updated: 2026-11-07

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

In progress.

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

## Immediate next action

Confirm the final automated checks (`cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test -p svf-core`, `cargo test -p svf-render`, and `cargo test --workspace`) if their complete results are not already recorded. Visual/runtime behavior for the reference axes/grid, resize, minimize/restore, and clean exit has been user-verified.
