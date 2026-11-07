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

M1 was locally validated on the user's Windows environment:
- `cargo check -p svf-render` passed.
- `cargo check --workspace` passed.
- The Explorer window opened and displayed the clear background.
- Resizing, minimizing, restoring, and closing behaved as expected.

The visible clear color is intentionally a very dark blue-gray, not pure black.
These results are user-reported local validation; they have not been independently
reproduced in this repository environment.

## M2 — Spatial foundation

Objective: establish a graphics-independent mathematical model for spatial
coordinates and camera transforms before introducing scientific geometry.

Accepted conventions are recorded in
`docs/decisions/0002-coordinate-conventions.md`.

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
- [ ] Add validated world/view transform APIs and tests

User-reported Windows validation:
- `cargo check --workspace`: passed.
- `cargo test -p svf-core`: passed.
- `cargo test --workspace`: passed.

### M2.3 — Camera and projection

- [ ] Define camera state and view transformation
- [ ] Implement validated perspective projection
- [ ] Implement validated orthographic projection
- [ ] Define aspect-ratio and viewport mapping behavior
- [ ] Test camera and projection edge cases

### M2.4 — Renderer integration

- [ ] Convert camera matrices to renderer representation explicitly
- [ ] Render reference axes and a grid
- [ ] Verify resize updates projection without mutating world data

## M3 — Scientific geometry

- points, line segments, polylines
- indexed triangle meshes
- explicit topology validation
- CPU-side geometry tests

## M4 — Mathematical sampling

- bounded domains and sampling strategies
- function sampling with invalid-value handling
- surface mesh generation
- sampling density and error considerations

## M5 — Interactive surface demonstrator

- render `z = A * sin(x) * cos(y)`
- adjust A and regenerate only affected geometry
- inspect a selected point and recover its source coordinates/value
