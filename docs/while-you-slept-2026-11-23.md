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
- Not measured: cost on heart-sized meshes (the function is O(triangles); the heart structure was not timed).
