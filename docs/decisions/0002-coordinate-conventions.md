# 0002 — Coordinate conventions and camera mathematics

- Status: Proposed for M2; confirm before implementation
- Date: 2026-11-07

## Context

SVF needs stable spatial semantics before it introduces scientific geometry.
The conventions must be understandable independently of wgpu and must not
couple scientific coordinates to screen pixels or a particular renderer.

## Proposed conventions

### World coordinates

- Right-handed coordinate system.
- +Y is world up.
- +X is the world horizontal axis.
- +Z completes the right-handed basis.
- The initial reference view may place the camera on +Z looking toward the
  origin; this is a viewing example, not a restriction on camera placement.

### Camera/view coordinates

- Camera-local -Z is forward (the viewing direction).
- Camera-local +Y is up.
- The view transform maps world-space positions into camera/view space.
- Camera state is expressed in world-space terms; view matrices are derived
  presentation transforms, not a mutation of scientific data.

### Transform convention

- Use column vectors.
- A point is transformed by `p_clip = P * V * M * p_local`.
- The rightmost transform is applied first.
- Matrix storage layout must be specified separately from mathematical
  multiplication convention; do not infer one from the other.

### Projection and clip space

- Use the WebGPU clip-space depth range [0, 1].
- Keep perspective and orthographic projection as distinct camera projection
  modes.
- Validate projection parameters (for example, positive aspect ratio,
  near > 0, far > near, and valid field of view) before producing a matrix.
- Keep clip/NDC coordinates separate from viewport/pixel coordinates. The
  renderer owns the final mapping to the surface, including its pixel-space
  Y direction.

### Precision and API boundary

- Core spatial calculations use f64 where appropriate.
- Conversion to f32 is explicit at the renderer boundary.
- No wgpu types appear in `svf-core` public APIs.

## Open decisions before implementation

1. Whether to implement minimal in-house f64 vector/matrix primitives or adopt
   a focused math crate. Decide based on required operations, correctness,
   testability, and maintenance—not convenience alone.
2. Exact matrix storage/API representation.
3. Camera API shape: look-at target/up representation versus orientation
   representation, and how orbit controls will build on it.
4. Exact NDC-to-viewport mapping and depth handling at the renderer boundary.
5. Whether 2D coordinates use a separate explicit coordinate type or share
   spatial primitives with a constrained dimensional model.

## Initial M2 validation expectations

- Test identity and composition order.
- Test known world-to-view transformations.
- Test perspective and orthographic projection bounds and invalid parameters.
- Test aspect-ratio changes without mutating world/camera scientific data.
- Verify numerical behavior around degenerate camera configurations.
