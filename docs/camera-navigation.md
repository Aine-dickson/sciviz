# M7.5 — Interactive 3D camera navigation

## Status

Implemented and merged to `main` through PR #13 on 2026-11-08.
Merge commit: `32002a3cbce2a3c03755cbda67dc8b759de1892d`.

User-reported Windows validation: workspace tests and check passed; runtime
navigation passed in all six Explorer modes. The workspace check retains one
pre-existing `VectorSource3::new` dead-code warning.

## Motivation

The Explorer previously rendered every visualization from a fixed reference
camera. M7.4 validation confirmed structured cell-vector glyph placement and
arrowhead visibility, but a single view did not allow a conclusive visual
inspection of depth occlusion. M7.5 adds interactive orbit, pan, and zoom so
the same scientific scene can be inspected from multiple viewpoints.

Add camera navigation as a reusable inspection capability, not as a
scientific-data or geometry transformation.

## Objective

Allow users to inspect existing scientific scenes from different viewpoints
using three independent operations:

- **Orbit:** rotate the camera around its look-at target.
- **Pan:** translate the camera position and target together in the camera
  plane.
- **Zoom:** move toward or away from the target while enforcing configured
  minimum and maximum viewing distances.

The initial implementation remains a perspective-camera feature. Orthographic
camera navigation is outside this increment unless implementation review
shows it is already naturally supported without broadening scope.

## Architectural responsibilities

### `svf-core`

- Provide reusable camera-navigation mathematics independent of winit and wgpu.
- Operate on the existing f64 `Camera3` / `LookAtCamera` model.
- Validate finite inputs and preserve the framework's right-handed,
  +Y-up conventions.
- Do not receive mouse, keyboard, window, or GPU types.
- Do not modify scientific geometry, datasets, fields, or glyph positions.

A small input-independent controller or explicit camera-operation API may be
introduced if it provides a clear reusable contract. Avoid a generic input
framework or a large camera abstraction.

### `svf-render`

- Expose a validated camera update method accepting the camera state produced
  by the core navigation API.
- Update the renderer's f64 camera state and corresponding GPU uniform as one
  operation.
- Ensure `camera()` and `ray_from_viewport` use the same current camera state.
- Preserve the current projection parameters except for aspect-ratio changes
  during resize.
- Do not own mouse gesture interpretation.

### `svf-explorer`

- Track pointer position and active gesture state.
- Convert pointer deltas and wheel input into orbit, pan, and zoom requests.
- Request redraw after successful camera updates.
- Keep existing click-to-inspect behavior; a drag must not also trigger a
  click inspection.
- Keep camera controls available in every current visualization mode.

## Default interaction mapping

Implemented Explorer mapping:

- Left mouse drag: orbit.
- Middle mouse drag: pan.
- Mouse wheel: zoom.
- Right mouse drag: reserved initially; do not assign a second pan gesture
  unless platform testing shows a strong usability reason.

Use physical-pixel pointer deltas consistently with the current viewport and
picking APIs. Gesture start/end must be tracked explicitly so a completed drag
does not generate a click action.

## Camera semantics and numerical safeguards

- Orbit around the current look-at target; do not rotate scientific objects.
- Use a stable world-up reference of +Y.
- Prevent the view direction from reaching the up-axis singularity; clamp
  elevation to a documented margin from the poles.
- Preserve camera-target distance during orbit.
- Pan translates position and target by the same finite world-space offset.
- Pan scale is derived from camera distance and vertical field of view so the
  same pointer motion has a predictable screen-space effect.
- Zoom changes camera-target distance, not the target itself.
- Define explicit positive minimum and maximum camera distances.
- Reject or safely ignore non-finite deltas and invalid configurations.
- Avoid sudden flips, inverted controls, and unbounded movement.
- Keep all camera calculations in f64. Conversion to f32 remains exclusively
  at the existing renderer/GPU boundary.

The initial reference camera looks from (14, 12, 18) toward the origin, with a
45-degree vertical field of view and a 0.1–100 clipping range. Initial
navigation limits and sensitivity constants should be selected and documented
during implementation, with tests around their boundaries.

## Picking and rendering consistency

After any camera update:

- The renderer's camera uniform must reflect the new camera.
- The public camera accessor must return that same camera.
- Explorer surface inspection must build its ray from that updated camera.
- No stale camera copy may be used for picking.
- Resize must update aspect ratio without resetting the user's orbit, pan, or
  zoom state.

Depth testing remains enabled. Camera navigation must not change glyph depth,
force glyphs in front of surfaces, or bypass occlusion.

## Required automated tests

Core:
- Orbit preserves target and camera-target distance.
- Orbit remains finite and valid near both elevation limits.
- Pan translates position and target equally and preserves orientation.
- Pan scale responds consistently to camera distance and field of view.
- Zoom preserves target and clamps to minimum/maximum distance.
- Non-finite deltas and invalid limits are rejected or safely ignored.
- Repeated operations do not create a degenerate look-at pose.

Renderer:
- Camera update changes the stored camera and writes a finite GPU uniform.
- Camera update rejects invalid camera states without partially changing state.
- Resize preserves pose and updates only projection aspect ratio.
- Picking rays from the updated camera differ appropriately from the original.

Explorer:
- Gesture state distinguishes click from drag.
- Orbit, pan, and zoom input mapping is deterministic.
- Existing click inspection still works without a drag.
- Camera gestures do not alter scientific data or mesh buffers.

## Windows acceptance

- [x] Left drag orbits around the scene target.
- [x] Middle drag pans the scene without changing camera orientation.
- [x] Wheel zooms and respects configured distance limits.
- [x] Orbit elevation is clamped away from the world-up singularity.
- [x] Click inspection remains functional; a completed drag does not also
      trigger click inspection.
- [x] Picking follows the updated camera.
- [x] M7.4 `--cell-vectors` can be inspected from multiple angles.
- [x] `--vectors`, `--scalar`, `--unstructured-scalar`,
      `--unstructured`, and default mode remain functional.
- [x] Resize and minimize/restore preserve usable camera behavior.
- [x] `cargo check --workspace` passes; one pre-existing dead-code warning
      remains.
- [x] `cargo test --workspace` passes.

## Explicit non-goals

- Camera animation, keyframe paths, bookmarks, or saved views.
- Orthographic navigation unless already supported without broadening scope.
- Multiple cameras, viewports, or split-screen layouts.
- Object selection/manipulation or scene editing.
- Transforming scientific datasets or glyph geometry.
- Lighting, materials, labels, legends, or other rendering enhancements.
- General-purpose input binding/configuration framework.
- Touch, gamepad, or multi-touch navigation.


## Implementation record

The implementation follows the responsibilities above:

- `svf-core/src/camera_navigation.rs` provides
  `CameraNavigationConfig`, `CameraNavigationError`, and public
  `orbit`, `pan`, and `zoom` operations. It has no winit or wgpu
  dependency.
- Orbit preserves target and distance, uses +Y world up, and clamps elevation
  using the configured pole margin.
- Pan is perspective-only and translates camera position and target by the
  same camera-plane offset, scaled using distance, vertical FOV, and viewport
  height.
- Zoom changes distance exponentially and clamps it to configured limits.
  Positive wheel delta zooms in.
- The default configuration is min distance 1.0, max distance 80.0,
  pole margin 0.02 radians, orbit sensitivity 0.006 radians/pixel, and
  zoom exponent sensitivity 0.12 per wheel unit.
- `SceneRenderer::set_camera` validates the f32 GPU uniform before writing
  it and replacing the renderer's stored f64 camera, so rendering and
  `ray_from_viewport` use the same camera.
- Explorer uses left drag for orbit, middle drag for pan, and wheel input for
  zoom. A drag suppresses the corresponding click inspection.

User-reported Windows validation (2026-11-08):

- `cargo test --workspace`: passed (exact final count not recorded).
- `cargo check --workspace`: passed with the existing
  `VectorSource3::new` dead-code warning.
- Runtime navigation passed in default, `--unstructured`, `--scalar`,
  `--unstructured-scalar`, `--vectors`, and `--cell-vectors` modes.
- In `--cell-vectors`, orbit, pan, zoom, picking synchronization, and
  resize/minimize/restore were explicitly reported working as expected.
