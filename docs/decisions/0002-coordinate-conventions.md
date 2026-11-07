# 0002 — Coordinate conventions and camera mathematics

- Status: Accepted for M2
- Date: 2026-11-07

## Context

SVF needs stable spatial semantics before it introduces scientific geometry.
The conventions must be understandable independently of wgpu and must not
couple scientific coordinates to screen pixels or a particular renderer.

## Accepted conventions

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
- Matrices are stored column-major. Storage layout and multiplication
  convention are documented separately.
- Angles are expressed in radians at the mathematical API boundary.

### Projection and clip space

- Use the WebGPU/DirectX NDC depth range [0, 1].
- View space is right-handed, Y-up, with -Z forward.
- Keep perspective and orthographic projection as distinct camera projection
  modes.
- Validate projection parameters before producing a matrix; do not rely on
  optional glam assertions for user-input validation.
- Keep clip/NDC coordinates separate from viewport/pixel coordinates. The
  renderer owns the final mapping to the surface, including its pixel-space
  Y direction.

### Precision and API boundary

- Core spatial calculations use f64 where appropriate.
- Conversion to f32 is explicit at the renderer boundary.
- No wgpu types appear in `svf-core` public APIs.

## Math dependency

Use `glam` as the underlying linear algebra implementation. Pin the initial
workspace dependency to `=0.34.2` while the library is pre-1.0. Enable only
`std` and `float-types`; SVF needs f64 vectors, matrices, and quaternions.

SVF's public domain API should add semantic types where they prevent mistakes
(for example, distinguishing a world point from a displacement vector).
Do not blindly expose every glam type as an SVF domain concept. Glam remains
the mathematical implementation dependency and can be wrapped where stable
domain semantics require it.

Glam uses column vectors and column-major matrix storage. For WebGPU, select
right-handed projection functions that produce NDC depth [0, 1].

## Open decisions before camera implementation

1. Camera state representation: look-at target/up versus orientation/quaternion.
2. Exact projection parameter types and error model.
3. Exact NDC-to-viewport mapping and depth handling at the renderer boundary.
4. Whether 2D coordinates use a separate explicit coordinate type or share
   spatial primitives with a constrained dimensional model.
5. Whether world points/vectors should later carry coordinate-frame identity
   for multi-frame scientific datasets.

## Initial M2 validation expectations

- Test identity and composition order.
- Test known world-to-view transformations.
- Test perspective and orthographic projection bounds and invalid parameters.
- Test aspect-ratio changes without mutating world/camera scientific data.
- Verify numerical behavior around degenerate camera configurations.
