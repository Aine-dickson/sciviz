# Notes left for the morning (2026-11-23 night session)

The user went to sleep and asked Claude to keep working through the plan and leave behind anything they need to know.
Newest entries last. Nothing here is merged; nothing was pushed. Branch: `c1-fidelity-register` (off #83).

## C2.1 Smooth shading (done, committed)

- The renderer now gives meshes smooth vertex normals (`with_smooth_normals` in `crates/svf-render/src/lib.rs`, shader
  `mesh.wgsl`). A vertex is split into one copy per group of faces within a 40 degree crease angle, so balls and tubes are
  smooth while boxes, caps and steps keep sharp edges. Winding is ignored (the shader is two-sided). Colour and opacity
  are copied to every copy of a vertex.
- **Decision (mine, easy to change):** crease angle 40 degrees, constant `CREASE_ANGLE_DEGREES`.
- **Decision:** the plain `SceneRenderer::new(window, mesh)` path stays flat on purpose, because `update_mesh` (the
  parametric demo) needs a fixed vertex count. Structures, coloured meshes and scalar/isosurface scenes are smoothed
  (unshaded scalar surfaces are not touched).
- Evidence: `docs/evidence/c2-smooth-shading/` (salt, water, breathing, circuit); the old flat shots are in the other
  evidence folders for comparison. Looked at by Claude only.
- Tests: 3 new renderer tests (cube stays sharp, sphere smooth and unsplit, mixed winding / zero-area face / colour kept).
  Workspace tests: 0 failures.
- Cost on the heart: the whole Explorer start plus screenshot takes 0.68 s (`heart.png` in the evidence folder), so no cache is needed.

## C2.2 Hinge for Turn (done, committed)

- `Turn(..., about: Some((x, y, z)))` swings a part about that point (RON, optional; old files unchanged). Spec in
  `docs/animation-design.md` section 27. The hinge point travels with `Move`; `Grow` does not shift it.
- **Decision:** `PartPose` gained `turn_pivots` parallel to `turns` instead of changing the `turns` tuple, so existing
  code and tests are untouched.
- The circuit switch and the bar-magnet compass are NOT retrofitted; the circuit is re-authored in C3.

## C2.3 Rounded routes (done, committed)

- New module `structure_route` plus RON `corner_radius` / `point_spacing` on `Tube` and on the `Flow` action. Spec in
  `docs/animation-design.md` section 28.
- **Decision:** routing between *named* points is left to the C3 item-document compiler (it resolves names to coordinates);
  the engine only rounds corners. A tube with `corner_radius` takes exactly one `half_widths` entry (a varying width along a
  routed tube is not supported; say if you want it).
- **Decision:** a radius too big for its straight run is an error, never silently shrunk.

## C2.4 Group transforms (done, committed)

- `Grow` / `Turn` on a group scale or turn the whole group as one body about its combined centre; `each: true` does each
  part about its own centre. Spec: `docs/animation-design.md` section 29.
- **Behaviour change to know about:** `Grow`/`Turn` on a part that has children now takes the children too (before, only the
  part itself). No fixture was affected; one old unit test that asserted the old rule was rewritten.
- **Decision:** one part may not be grown/turned both inside a group and on its own in the same timeline (refused with a
  message), because one part has a single scale and a single centre.
- The salt item still has its 64 separate grows; it is re-authored in C4.
- **Limit:** a parent that owns geometry cannot be scaled without its children (split its own geometry into a child part).
