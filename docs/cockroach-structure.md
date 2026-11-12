# B2 step 4 — cockroach: new blocks, copies, relations and views

Implements step 4 of `explorable-structures.md`. Proves the structure format
generalises to a second organism: a learner sees the external features of a
cockroach, removes layers in dorsal-dissection order, and names the internal
organs. Split into **4a** core blocks (no fixture), **4b** relations, views and
the cockroach fixture, **4c** Explorer and live check.

Lesson content (confirmed by the user, 2026-11-12): external baseline (head,
thorax, abdomen, wings, antennae, compound eyes, cerci) with segmented legs
(coxa, femur, tibia, tarsus), three thoracic segments, abdominal segments,
tegmina and hind wings, mouthparts as one part; internal digestive, nervous,
heart and fat body, reproductive and tracheal systems; **dorsal** dissection.

## Scope

In: two new geometry blocks, part copies (repeat, mirror), relations, views,
a cockroach fixture, Explorer view switching.
Out: labels (text rendering), animation, measured proportions, fine tracheal
branching, texture, and any code guarantee that parts do not overlap (see
Risks). Sections, caps and peeling are unchanged: the section code already
handles any closed, outward-wound part (spike, 2026-11-12).

## 1. Orientation

Anterior +X, dorsal +Y, the animal's left/right along Z, bilateral plane
z = 0. So the Explorer's existing section axes give a sagittal cut (`z`), a
transverse cut (`x`) and a frontal cut (`y`).

## 2. New blocks

Both are closed, outward-wound triangle meshes, one block per part. Anatomy of
several pieces (a leg's four segments, the gut regions) uses child parts, so
each piece is pickable and peelable.

### `Tube`

```
Tube(path: [(x, y, z), ...], half_widths: [(side, up), ...],
     up: (0, 1, 0), steps_per_span: 8, wall: None, arc_deg: None)
```

- A Catmull-Rom curve through the control points. At each control point the
  cross-section is an ellipse of semi-axes `side` (along the frame's side
  direction) and `up` (along the frame's up direction), interpolated
  linearly between control points.
- Frame by parallel transport, started from `up` (a hint, projected
  perpendicular to the first tangent). Circles use equal `side` and `up`.
- Flat end caps, wound outward (the spike's first failure was two end fans
  wound the same way).
- `wall: Some(t)` makes a hollow shell of wall thickness `t` (inner surface
  wound inward, as in the spike's hollow ellipsoid). End caps become rings.
- `arc_deg: Some((start, end))` keeps only that angular part of the
  cross-section (angle measured from the frame's up direction), closed by
  flat side walls: a trough such as the dorsal tergites or ventral sternites.
  Needs `wall`.
- Wings are very flat tubes (`up` tiny); no separate wing block.

Validation (typed errors naming the part): at least two distinct path points;
`half_widths` length equals path length; every semi-axis positive and finite
(a tip tapers to a small radius, not zero); `wall` less than the smallest
semi-axis; `arc_deg` width in (0, 360) and `start < end`; `up` not parallel to
the first tangent; **bend radius greater than the largest semi-axis at every
sample, else `TubeTooTightlyBent`** (a tighter bend self-intersects and
silently breaks closure).

### `Ellipsoid`

```
Ellipsoid(centre: (x, y, z), radii: (a, b, c), rotate_deg: (0, 0, 0), wall: None)
```

Latitude/longitude sphere (poles on local +Y) scaled by the radii, then rotated
about its centre about world X, then Y, then Z. `wall: Some(t)` adds an
inward-wound inner surface (every radius shrinks by `t`). Used for the head
capsule, eyes, brain, ganglia, gizzard, ovaries and fat-body lobes. Poles are
single vertices; `circle_segments / 2` (at least 4) latitude bands.

Segment counts reuse `circle_segments` for the cross-section ring and
`steps_per_span` for the length, so one file-level knob controls detail.

## 3. Copies

Optional fields on a part, applied to its geometry in this order:

```
repeat: Some(Linear(count: 5, step: (0.0, 0.0, 0.0)))   // or Radial(...)
mirror: Some(Z)
```

- `Linear { count, step }` places `count` copies, each offset by `step`.
  `Radial { count, axis_point, axis_dir }` rotates copies evenly about an axis.
- `mirror: Some(X | Y | Z)` adds the reflection across the plane through the
  origin normal to that axis, **reversing triangle winding** so the copy stays
  outward-wound (spike lesson). A mirrored pair is two closed bodies.
- All copies belong to the one part, as `Sector.count` already does:
  picking returns the part, not the copy. A pair of hind legs is therefore one
  pickable "Hind leg", which is acceptable for the lesson. Parts that must be
  picked separately are authored separately.
- No per-copy scale. Tapering abdominal segments are authored explicitly.

`Extrusion` parts accept the same fields (they were already repeatable through
`Sector.count`; both keep working).

## 4. Relations

```
relations: [Relation(from: "crop", kind: OpensInto, to: "gizzard"), ...]
```

`kind` is `Contains`, `ConnectsTo` or `OpensInto`. Ids must exist; a relation
may not join a part to itself. Relations are descriptive and not used for
geometry. The part tree still means "is part of". The Explorer prints the
relations of the clicked part with its description.

## 5. Views

```
views: [View(id: "dorsal-gut", name: "Gut from above",
             camera: Some((yaw_deg: 90.0, pitch_deg: 80.0, zoom: 1.2)),
             show: [], peel: 3, section: None)]
```

- `show`: part ids to display; empty means all, grouping nodes expand to
  their descendants. `peel`: number of outermost layers removed, at most the
  layer count. `section`: `Some((axis, offset, flip))` like `--section`.
- Validated against parts and layers; unknown ids are errors.
- Explorer: `V` cycles views, `--view <id>` starts in one, and an explicit
  `--camera-*`, `--section` or `--peel` flag overrides the view's value.
  A view that would show nothing is refused, the same rule as for `P` and `[`.

## 6. File version and compatibility

`version` stays 1. All additions are optional with defaults (the rule used for
`layers`), so `dicot-stem.ron` loads unchanged and `deny_unknown_fields` still
catches misspelt fields. The stem fixture is a regression test for this.

## 7. Authoring rules

- Body wall is hollow: a `Tube` (or `Ellipsoid`) with `wall`, split dorsal and
  ventral by `arc_deg`. A solid body would have its cap hide the organs. The
  spike shows the cap of a shell is a ring and organs inside stay visible.
- Organs lie inside the cavity, not through the wall.
- Gut regions meet end to end with equal semi-axes at the junction, so the gut
  reads as continuous; each region is its own child part.
- Appendages may sink into the wall by at most the wall thickness. In that
  overlap two surfaces are inside each other, and caps may show a seam; this is
  a known, accepted limit.

## 8. Dissection layers (outermost first)

1. Wings (tegmina and hind wings).
2. Dorsal body wall (pronotum, mesonotum, metanotum, abdominal tergites).
3. Heart and fat body.
4. Digestive system, with salivary glands and Malpighian tubules.
5. Reproductive organs.
6. Tracheal trunks and spiracles.
7. Nervous system (brain, ventral nerve cord and ganglia).

The heart is its own layer, with the fat body, and not part of the wall's: a
layer that is peeled hides all its parts, so a heart in layer 2 could never be
seen. (The first draft of this note put it there; corrected in 4b.)

Parts in no layer are never peeled (existing `visible_parts` rule): ventral
body wall, head capsule, eyes, antennae, legs, cerci, mouthparts. Peeling the
dorsal wall exposes the heart and fat body, as the real dissection does.
Removing the gut then shows what lay beneath it.

## 9. Part tree

Cockroach (grouping) → External (grouping: head and its parts, thorax by
segment, abdomen by segment, wings, legs by pair and segment, cerci) and
Internal (grouping: digestive, circulatory, reproductive, respiratory,
nervous). Digestive: salivary glands, crop, gizzard, gastric caeca, midgut,
Malpighian tubules, hindgut, rectum. Reproductive organs are of **one sex**.
Female (ovaries, oviducts, spermatheca, colleterial glands), confirmed by the user
(2026-11-12). Proportions are schematic. As with the stem, the
`reference` citation is the user's to confirm, and the note records it as
unconfirmed until then.

## 10. Risks and checks

- **Overlap is not detected by code** (spike). 4b adds a fixture test with a
  point-in-body (ray parity) check on sample points of each organ against its
  neighbours and the wall cavity. It is a test helper, not public API.
- **Cap cost** is O(n^2) in loop length: 3 ms at 500 points, 450 ms at 8000.
  Organism-scale loops are well under 100 points, so fine. A tube cut along
  its length gives long loops; the 4a sweep test measures the worst cut and
  fails over 100 ms (release).
- **Size**: about 80 parts. Triangles per part are bounded by `circle_segments`
  and `steps_per_span`; 4b records the total and the time of a section
  rebuild, and reduces segment counts if a keypress is visibly slow.
- **Camera**: structure mode caps pitch near 59 degrees, but a dorsal view
  needs about 90. 4c checks this and raises the cap if the camera allows it.
- **Fixture size** will be tens of kilobytes, not the stem's few. Accepted.
- Imported or adapted meshes are still unsolved (step 5).

## 11. Tests

4a (core, no fixture): each new block is closed (every edge used twice) and
outward-wound (positive signed volume); volume of a straight circular tube,
ellipsoid and shell matches the analytic value within tessellation tolerance;
a mirrored copy has equal volume and stays closed; each validation error class;
a bent tube section sweep over three axes and offsets -4 to 4 (cap succeeds,
result closed); the bend-radius check rejects a tight bend; `dicot-stem.ron`
loads unchanged.
4b: relations and views validation; the cockroach fixture parses, every part
is closed, the sweep passes, the overlap check passes, layer order matches
section 8.
4c: Explorer argument tests for `--view`; screenshots of the external view,
each peel stage and one section, plus the user's live check.

## 12. Order of work

1. 4a on `b2-4a-blocks`: `Tube`, `Ellipsoid`, copies, validation, tests;
   delete `structure_spike.rs` and the test-only `from_bodies`.
2. 4b on `b2-4b-fixture`: relations, views, `fixtures/structures/cockroach.ron`.
3. 4c on `b2-4c-explorer`: `V`, `--view`, relations on pick, camera check,
   evidence images, user's live check.

## 13. Acceptance

An O-level learner, from the Explorer alone, can name the cockroach's external
features, remove layers in the dorsal order of section 8, and name each
internal organ by picking it. No new project or Explorer code is needed for the
cockroach beyond the fixture and the generic `V` view switching.

## 14. Explorer, as built (4c)

- `V` applies the next named view (camera, parts shown, peel, section) and wraps
  after the last. `--view <id>` starts in one; an unknown id lists the choices.
  `--camera-yaw`, `--camera-pitch`, `--camera-zoom`, `--section` and `--peel`
  override the same setting of the view, field by field.
- A view's `show` list restricts the display (grouping nodes expand); `P`, `R`,
  `S` and `I` still work inside a view. `A` leaves the view: everything shown,
  nothing peeled, no view named. A view that cannot be applied (for example a
  pitch beyond the allowed elevation) is refused and the scene is unchanged.
- Clicking a part prints its path, description and relations ("Crop opens into
  Gizzard"). The title names the current view.
- Camera values in a view are relative to the camera fitted to the whole file,
  like the `--camera-*` flags, so they belong to the file they were tuned for.
  For the cockroach, yaw 40 puts the animal square to the screen; pitch 58 is a
  straight-down dorsal view (the limit is the fitted elevation plus pitch below
  about 89 degrees, so no change to the pitch cap was needed), pitch -18 a
  lateral view.
