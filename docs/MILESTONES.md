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

Initial design direction:
- use a right-handed world coordinate system with +Y up
- define camera forward as local -Z
- use column vectors and explicit transform composition
- preserve f64 in the scientific/core layer
- make conversion to renderer-friendly f32 an explicit boundary
- use WebGPU clip-space depth range [0, 1]
- keep viewport/pixel coordinates distinct from world and clip coordinates

Planned acceptance criteria:
- document coordinate spaces and transformation direction
- implement tested vector/matrix or equivalent transform primitives in
  `svf-core`, without exposing wgpu types
- implement camera view transformation
- implement perspective and orthographic projection with validated parameters
- define aspect-ratio and viewport mapping behavior
- add unit tests for camera transforms, projection, and edge cases
- integrate camera matrices into `svf-render` only after core math is tested
- render reference axes/grid as the first spatial integration test

M2 has started with coordinate-convention design. No camera or transform
implementation is claimed yet.

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
