# B1.2 stage 5 evidence: boundary axial glyphs on Rodero case 01

Eight PNGs, four views each shown plain and RHO-coloured. All are 1600 x 1200
RGBA8 PNGs written by `svf-explorer --screenshot`. They regenerate with one
command (see Reproduce).

## What was run

- Binary: `svf-explorer` release build of PR #55 head `e8b0ff8`
  (`cargo build -p svf-explorer --release`), Windows 11, adapter
  AMD Radeon(TM) Graphics (Vulkan, integrated).
- Data: `01-svf-order.vtk`, `input_sha256 5af445a6e73e1640af7ccd8a89229aee0d90740b360ce2e8a9d8442bc2a2bb5f`
  (the reordered copy of Rodero 2021 case 01, source sha256
  `20bf29b8ec86454de4cfde2a0afc19992715107b6ef8a7dabe61f52e8f78f440`).
- Glyph options on every image: `--vtk-cell-axial fibres --vtk-cell-tags ID=1,2`,
  bin 4 mm, length 3 mm (the defaults). The summary line printed on every run:
  tag 1: 86,241 selected / 86,241 eligible / 2,836 glyphs; tag 2: 85,232 / 85,232 /
  2,347; 5,183 glyphs; 0 zero-area and 0 invalid-direction exclusions. This equals
  the established count.
- RHO images add `--vtk-scalar RHO.dat --vtk-scalar-exclude -10`. Excluded
  sentinel vertices take the invalid colour (magenta), which is what the atria and
  vessels show.

## Reproduce

```
cargo build -p svf-explorer --release
docs/evidence/b1-2-stage5/capture.sh [path/to/01-svf-order.vtk] [path/to/svf-explorer]
```

Each image is one command of this form (`capture.sh` holds all eight):

```
svf-explorer --vtk 01-svf-order.vtk --vtk-cell-axial fibres --vtk-cell-tags ID=1,2 \
  [--vtk-scalar RHO.dat --vtk-scalar-exclude -10] \
  --camera-yaw <Y> [--camera-pitch <P>] --camera-zoom <Z> --screenshot <name>.png
```

## Views

The camera orbits the fitted target, the centre of the mesh bounding box. The
fitted camera sits at azimuth 45 degrees (`atan2(x, z)` of its offset), elevation
29.5 degrees above the XZ plane. Positive yaw reduces the azimuth. Pitch is 0 in
every view, so the elevation stays 29.5 degrees.

| Files | Yaw | Pitch | Zoom | Camera azimuth | Purpose |
|---|---|---|---|---|---|
| `v1-yaw270-{plain,rho}.png` | 270 | 0 | 1.5 | 135 | Ventricles with the atria above, the whole-heart view. |
| `v2-yaw180-{plain,rho}.png` | 180 | 0 | 1.5 | 225 | The ventricular surface from the other side, glyphs wrapping the wall. |
| `v3-yaw090-{plain,rho}.png` | 90 | 0 | 1.5 | 315 | From the atrial side; mostly atria, so it checks that they carry no glyphs. |
| `v4-yaw270-zoom3p5-{plain,rho}.png` | 270 | 0 | 3.5 | 135 | Close view for glyph detail and depth behaviour. |

The names carry no anatomical meaning. Which ventricle faces the camera in each
view was not established; the labels are camera parameters only.

## Deviations from the plan

- The plan named anterior and posterior epicardium and a basal view into the
  ventricles. Anterior and posterior were not established for this dataset, so
  the views are named by camera. A basal view looks at the atria, not into the
  ventricles, so v3 (the atrial side) replaces it as the atria check.
- The tail and RV-other triangles are not marked in the images. By the agreed
  decision the Explorer has no option to highlight them; they are located through
  the `fibre_diagnostic --samples` export (case 01: 864 LV tail, 855 RV tail,
  732 RV other, 8 rows in both RV classes).

## Observations (the author's, for the A4 review; not a verdict)

- The glyphs are plain line segments, without arrowheads.
- In all four views the glyphs sit on the ventricular surface and none appear
  on the atria or vessels (v1, v2, v3; the magenta region in the RHO images).
- The pattern is coherent: in the close view (v4) neighbouring glyphs run in
  nearly one direction, and in the wider views the direction turns smoothly
  across the wall (v2 shows it curving round the wall). This says nothing about
  whether the angle matches an expected helix angle; that stays outside this
  evidence.
- Visibility: on the plain surface the cyan glyphs are visible, but cyan on light
  blue is low contrast at the whole-heart scale (v1). On the RHO images the
  epicardium is uniformly yellow (RHO about 1) and the glyphs stand out clearly.
  The Viridis mid-teal that the colour was meant to stay distinct from does not
  occur on this exterior surface, so that comparison was not exercised.
- Depth: the lines draw through the unchanged line depth path. No dashing,
  flicker-like gaps or glyphs showing through the opaque surface were seen at
  these four views. A glyph whose fibre leaves the surface plane would be partly
  hidden; no such case was identified here, and an oblique close-up was not taken.
- Endocardium: not visible from outside, so the "steeper on the endocardium"
  part of A4 cannot be judged from these images.
- Not tested here: rotation and zoom in the interactive window (a manual check).
  The images come from a scripted camera and do not exercise those controls.

## Scripted sweep (in place of the interactive check)

The hands-on rotate/zoom check could not be done. Instead `sweep.sh` renders 18
more views into `sweep/` (RHO-coloured unless named `-plain`). `--camera-yaw`,
`--camera-pitch` and `--camera-zoom` call the same `camera_navigation::orbit` and
`zoom` functions, with the same distance and pole limits, as mouse navigation.
What this does **not** test is live input: drag and wheel event handling, frame
to frame continuity, flicker over time. Every run printed the 5,183-glyph baseline.

| Files | Yaw | Pitch | Zoom | Purpose |
|---|---|---|---|---|
| `s-yaw{000..315}.png` (8) | 0 to 315, step 45 | 0 | 1.5 | Full turn around the heart |
| `p-yaw270-pitch{+55,-60,-110}.png` | 270 | +55, -60, -110 | 1.5 | From above and below |
| `o-yaw{225,315,135}-pitch*-z3p5.png`, one `-plain` | 225, 315, 135 | +30, -30, +20 | 3.5 | Oblique close-ups |
| `z-yaw270-z{0p5,8,20}.png` | 270 | 0 | 0.5, 8, 20 | Zoom extremes |

Observations (the author's; not a verdict):

- Full turn, above and below: at every pose the heart is framed, glyphs lie on
  the yellow ventricular surface and none on the magenta atria/vessels. Framing
  changes smoothly between 45-degree steps; no pose lost the model.
- **Oblique close-ups show partial occlusion.** Many segments are drawn broken
  or dotted: parts of a segment fall behind the surface. This is the case the
  stage 5 review flagged (the exact source direction is not projected to the
  surface and there is no normal offset, so a segment leaving the surface plane
  is partly hidden, and near-coplanar parts can z-fight). Direction stays
  readable, but the segments are visibly fragmented. Seen in all three oblique
  RHO views and the plain one; clearest in `o-yaw315-pitch-30-z3p5.png`.
  Whether this can stay as a documented limitation or needs a depth bias is a
  review decision; nothing was changed.
- Zoom out (0.5): the whole heart stays in view, small, nothing clipped.
- Zoom in (8, 20): the camera ends up past the outer wall. The images show a
  dark (low-RHO) inner surface with glyphs and the magenta region at the edge;
  whether the camera is inside the wall/cavity or the near plane cuts the wall
  cannot be told from the images. Zooming toward the bounding-box centre allows
  this by design (zoom range 0.01 to 100). Glyphs on this inner surface are
  visible, but these views are not a controlled endocardium check.
