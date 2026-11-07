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
- Delivery strategy: Rust library first, with `apps/explorer` as a thin native
  integration/demo host.
- The user develops and validates locally on Windows, then shares exact
  compiler output, test results, and runtime observations.
- Treat repository documentation as the source of truth across sessions.
  Update this file after each completed feature or milestone.
- Never claim code is compiled, tested, committed, or runtime-verified without
  evidence. Distinguish implementation from user-reported local validation.
- Keep changes focused; do not add abstractions or dependencies prematurely.
- Do not change architectural decisions silently.

## Current workspace

- `svf-core`: graphics-independent scientific and mathematical foundations.
- `svf-render`: wgpu rendering infrastructure and native surface integration.
- `svf-explorer`: thin winit desktop host.

Workspace dependencies include wgpu 30.0.1, winit 0.30.13, pollster 0.4,
tracing 0.1, and glam =0.34.2. Glam is configured with only `std` and
`float-types` enabled. `svf-core` now has a `spatial` module containing
f64 glam primitive re-exports, world-axis constants, and semantic
`WorldPoint3` / `WorldVector3` wrappers.

## Milestone status

### M0 — Project foundation

Completed.

### M1 — Graphics foundation

Implementation and local validation completed.

User-reported Windows results:
- `cargo check -p svf-render`: passed.
- `cargo check --workspace`: passed.
- Explorer window opened and displayed the renderer's dark blue-gray clear
  color.
- Resizing worked.
- Minimize/restore and clean close behaved as expected.

The wgpu 30 compatibility adjustment in `crates/svf-render/src/lib.rs`
replaced the `InstanceDescriptor` default-update expression with
`InstanceDescriptor::new_without_display_handle()` and added
`apply_limit_buckets: false` to `RequestAdapterOptions`. The user reports
that this preserved backend and adapter-selection behavior.

These are user-reported local results, not independently reproduced here.

### M2 — Spatial foundation

In progress. M2.1 and M2.2 source changes have been committed, but local
Windows compilation and tests are pending.

Accepted conventions:
- right-handed world coordinates, +Y up
- camera view space is right-handed, +Y up, -Z forward
- column vectors; transforms multiply on the left
- matrices use column-major storage
- WebGPU NDC depth range [0, 1]
- f64 in the core; explicit f32 conversion at renderer boundary
- world, view, clip/NDC, and viewport/pixel spaces remain distinct

Math dependency:
- `glam =0.34.2`, workspace dependency
- `std` and `float-types` features only
- glam types are mathematical primitives; SVF adds semantic domain types
  rather than treating arbitrary vectors as points.

Implemented in `crates/svf-core/src/spatial.rs`:
- f64 glam vector, matrix, and quaternion primitive re-exports
- world axis constants
- `WorldPoint3` and `WorldVector3` newtypes
- point/vector arithmetic that preserves point-versus-displacement semantics
- unit tests for axis handedness, point subtraction/translation, vector
  operations, and matrix composition order

Not implemented yet:
- camera state and validated view transform
- perspective and orthographic projection wrappers
- viewport/NDC mapping
- renderer camera integration, axes, or grid

## Key architectural constraints

- Scientific data, coordinates, topology, and simulation state must never live
  only in GPU buffers.
- Public scientific/core APIs must not expose wgpu types.
- Preserve f64 where scientific calculations require it; f32 conversion is a
  rendering-boundary concern.
- Renderer consumes prepared visualization data; it does not own scientific
  algorithms.
- Picking should eventually map screen locations back to scientific entities
  and source values.
- Introduce crate boundaries only when they represent stable responsibilities.

## Current repository documents

- `README.md`: project overview and setup.
- `docs/ARCHITECTURE.md`: architectural boundaries and planned data model.
- `docs/MILESTONES.md`: milestone scope and acceptance criteria.
- `docs/decisions/0001-windows-first.md`: initial platform decision.
- `docs/decisions/0002-coordinate-conventions.md`: accepted coordinate
  conventions and glam dependency decision.

## Local validation commands

From the repository root:

```powershell
cargo check --workspace
cargo test -p svf-core
cargo test --workspace
```

For renderer integration later, also verify window launch, visible output,
resize, minimize/restore, and clean exit.

## Immediate next action

Run the workspace check and core tests on Windows. Fix any compiler or test
failures before starting camera implementation. Once M2.1/M2.2 pass locally,
continue with validated camera state and right-handed WebGPU perspective and
orthographic projection.
