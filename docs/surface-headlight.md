# Surface headlight (B2 step 1)

Implements section 6 of `explorable-structures.md`. Nothing more.

## Behaviour

- One light fixed to the camera. Each fragment is scaled by
  `0.25 + 0.75 * |n . v|`, where `n` is the triangle's own facing and `v` the
  unit vector from the fragment to the eye. The surface is two-sided, so the
  absolute value is used and winding does not matter.
- `n` comes from `cross(dpdx(world), dpdy(world))` in the fragment shader. No
  vertex normals and no new mesh data. The result is flat-shaded (faceted),
  which is honest about the data: the surface is a triangle mesh.
- Degenerate derivatives (zero-length cross product) leave the colour unchanged.
- Lines (grid, axes, glyphs, edges) are not shaded.

## Switch

- `SceneRenderer` holds one `shaded` flag, written into the camera uniform
  (`eye_shading.w`: 1.0 on, 0.0 off). The camera uniform grows from 64 to 80
  bytes: `view_projection` plus `eye_shading` (eye xyz, shading flag).
- Defaults: on for plain meshes (`new`, `new_with_vector_glyphs`,
  `new_with_axial_glyphs` without a mapped surface); off for every constructor
  that takes a scalar-mapped surface or isosurface, because shading changes
  brightness and distorts colour-encoded values.
- `set_shaded(bool)` and `shaded()` change and read it. The Explorer binds the
  `L` key to toggle it at run time.

## Verification

- Unit tests: uniform carries the eye and flag; mapped constructors default off.
- Running Explorer on the Rodero heart without RHO (shape readable) and with
  RHO (shading off by default, colours identical to before), via the existing
  `--screenshot` capture.

## Not in scope

Light objects, materials, shadows, specular, smooth normals, a light editor.
