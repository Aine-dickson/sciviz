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

Workspace dependencies currently include wgpu 30.0.1, winit 0.30.13,
pollster 0.4, and tracing 0.1. `svf-core` currently has no external
dependencies and contains only project identity constants and a basic test.

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

Started. Current work is design definition; no camera/math implementation is
claimed yet.

Objective: define coordinate semantics and camera mathematics in
`svf-core`, test them independently, then connect them to the renderer.

Initial design direction (to be confirmed/refined before implementation):
- right-handed world coordinates, +Y up
- camera looks along local -Z
- column-vector transform convention
- WebGPU clip-space depth range [0, 1]
- world/scientific values remain f64 in core
- explicit conversion to f32 at the rendering boundary
- world, view, clip/NDC, and viewport/pixel coordinates remain distinct

Next M2 tasks:
1. Review and agree the coordinate convention document.
2. Specify transform semantics, matrix layout, and projection parameter
   validation.
3. Choose the smallest suitable math implementation strategy; avoid a
   dependency until the required API and precision needs are clear.
4. Implement core coordinate/transform primitives with unit tests.
5. Implement camera view and perspective/orthographic projection.
6. Integrate matrices into the renderer and draw reference axes/grid.

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
- `docs/decisions/0002-coordinate-conventions.md`: initial M2 coordinate
  convention proposal.

## Local validation commands

From the repository root:

```powershell
cargo check --workspace
cargo test --workspace
cargo run -p svf-explorer
```

Run tests after each core math change. For renderer integration, also verify
window launch, visible output, resize, minimize/restore, and clean exit.

## Immediate next action

Review `docs/decisions/0002-coordinate-conventions.md` and settle the
coordinate, matrix, and projection conventions before writing M2 math code.
