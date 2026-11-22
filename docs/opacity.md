# Opacity (translucent parts)

Status: built 2026-11-22 (slices O1 to O4). Design decided in the plan for that session; this note records what exists.

## What an author can do
- Give a part `opacity: 0.3` (above 0, at most 1, default 1). Below 1 the part is drawn translucent.
- Fade a part, and its descendants, in a timeline: `Fade(part: "petals", to: 0.0, for_s: 4.0, ease: Smooth)`. It starts from the
  part's authored opacity, chains with later `Fade`s on the same part (overlaps are refused), and works on a grouping part for all
  its descendants.
- A part that has faded to below 0.005 is **left out**: not built, not drawn, not pickable. Fading back in rebuilds it.

## How it is drawn (`crates/svf-render`)
Sorted alpha blending. Opaque triangles are drawn first (depth write, as before), then the reference grid and glyph lines, then the
translucent triangles, farthest first, with standard alpha blending, depth test on and depth write off.
- A triangle is translucent when its first vertex has alpha below 1 (all vertices of a part share one value).
- The translucent triangles are sorted by centroid distance from the eye on the CPU, whenever the mesh is replaced or the camera moves
  (`set_camera`). Equal distances keep source order, so the result is deterministic.
- Chosen over order-independent transparency (more code, approximate colours, extra render targets) and depth peeling (heavy).

## Limits (known, accepted)
- Triangles of different translucent parts that intersect each other, or a very large triangle spanning smaller ones, can sort wrongly.
  Nested closed shells (glass around water around particles) sort correctly.
- Opaque things seen through several translucent layers look washed out: three layers (glass front, water, glass back) tint the
  dye particles noticeably. Keep opacities low and colours saturated.
- Picking hits the nearest triangle whether translucent or not. There is no "click through glass".
- Only structure parts have opacity. VTK/dataset scenes, vector glyphs and the reference lines are unchanged.
- `update_mesh` (fixed-topology buffer write) refuses a scene that has translucent triangles; structures use `replace_coloured_mesh_rgba`.

## Where it lives
- Renderer: `new_with_coloured_mesh_rgba`, `replace_coloured_mesh_rgba`, `partition_triangles`, `sort_translucent_indices`.
- Core: `Part::opacity`, `StructureGeometry::rgba_with(opacity_of_part)`, `PartPose::opacity`, action `Fade`.
  (Opacity is a per-part value looked up through the geometry's part ranges, not a per-vertex array stored in the geometry.)
- Explorer: `StructureScene::rgba()`.

## Cost
Sorting 2,000 translucent triangles: 0.09 ms; 80,000: 4.8 ms (scratch probe, release, CPU only). So even a translucent heart is
feasible if it is ever wanted; upload and drawing were not timed.

## Evidence
`docs/evidence/opacity/` (beaker at 4 s and 28 s with glass and water; flower at 14 s with the petals fading as they fall).
