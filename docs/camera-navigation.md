# M7.5 — Interactive 3D camera navigation

## Status

Design proposed for review. No implementation has started.

## Motivation

The Explorer currently renders every visualization from a fixed reference
camera. M7.4 validation confirmed structured cell-vector glyph placement and
arrowhead visibility, but a single view did not allow a conclusive visual
inspection of depth occlusion. Explorer currently has no orbit, pan, or zoom
input handling.

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

## Proposed default interaction mapping

The following mapping is proposed for review, not yet implemented:

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

- [ ] Left drag orbits around the scene target.
- [ ] Middle drag pans the scene without changing camera orientation.
- [ ] Wheel zooms smoothly and respects distance limits.
- [ ] No camera flip or singularity occurs near vertical views.
- [ ] Click inspection remains functional; dragging does not trigger click inspection.
- [ ] Picking follows the updated camera.
- [ ] M7.4 `--cell-vectors` can be inspected from multiple angles and glyph
      depth occlusion is visibly correct.
- [ ] `--vectors`, `--scalar`, `--unstructured-scalar`,
      `--unstructured`, and default mode remain functional.
- [ ] Resize and minimize/restore preserve the current camera pose.
- [ ] `cargo check --workspace` passes without warnings.
- [ ] `cargo test --workspace` passes.

## Explicit non-goals

- Camera animation, keyframe paths, bookmarks, or saved views.
- Orthographic navigation unless already supported without broadening scope.
- Multiple cameras, viewports, or split-screen layouts.
- Object selection/manipulation or scene editing.
- Transforming scientific datasets or glyph geometry.
- Lighting, materials, labels, legends, or other rendering enhancements.
- General-purpose input binding/configuration framework.
- Touch, gamepad, or multi-touch navigation.
