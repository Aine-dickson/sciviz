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
- [ ] Compile on Windows MSVC
- [ ] Run on a Windows machine with a supported GPU
- [ ] Verify resize/minimize/restore behavior on real hardware

M1 is implementation-complete in source, but not validated until the unchecked
Windows build/runtime checks pass.

## M2 — Spatial foundation

Proposed acceptance criteria:

- define coordinate conventions and handedness
- define world, view, and clip-space boundaries
- implement camera state and projection
- render 3D axes and a reference grid
- implement orbit, pan, and zoom
- unit-test camera transforms and projection parameters

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
