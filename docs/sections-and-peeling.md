# B2 step 3 — section plane with capping, and layer peeling

Implements step 3 of `explorable-structures.md`. Proves: a learner can look
*inside* a structure (a cut that shows tissues as filled regions) and remove
outer layers in a set order, for any structure made of closed parts. Split into
**3a** core (sectioning, capping, layers in the file format, tests, no GUI) and
**3b** Explorer (keys, command line, picking on caps, screenshots).

## Scope

In: one plane, capped cut faces, ordered layers, peel and restore.
Out: curved or multiple cuts, labels (step 3 follow-up), sectioning lines,
non-closed parts (reported as an error, never silently left open), animation.

## 1. Section plane

`SectionPlane { point: WorldPoint3, normal: WorldVector3 }` (normal non-zero,
finite; stored normalised). The side where `normal . (p - point) > 0` is
**removed**; the kept solid's cap faces `+normal`. Flip the kept side by
negating the normal.

```
section_geometry(&StructureGeometry, &SectionPlane) -> Result<StructureGeometry, SectionError>
```

The result is an ordinary `StructureGeometry`: same colours, **one contiguous
triangle range per part** (clipped body then its cap), so `part_of_triangle`
still names the tissue when the learner clicks the cap. Parts wholly removed
get no range; parts wholly kept are unchanged.

### Algorithm (per part, on its closed bodies)

1. **Degenerate plane position.** If any vertex is within `1e-9 * extent` of the
   plane (extent = bounding-box diagonal), the plane is nudged by `2e-9 * extent`
   along its normal. This is deterministic, invisible, and means no vertex is
   ever exactly on the plane.
2. **Clip.** Each triangle is classified by the sign of its three vertices.
   Wholly kept: copied. Wholly removed: dropped. Cut: clipped, producing one or
   two triangles on the kept side and one **cut segment** between two new
   vertices. New vertices are keyed by the undirected source edge, so the two
   triangles sharing an edge share the new vertex (watertight, no cracks).
3. **Loops.** Cut segments, directed by the triangle winding, chain into closed
   loops. A part that is a closed manifold gives every new vertex exactly two
   segments; anything else is `SectionError::NotClosed { part }`.
4. **Cap.** Loops are projected to the plane (2D basis `u, v` with
   `u x v = normal`). A loop that is counter-clockwise about `+normal` is an
   outer boundary, clockwise is a hole (this follows from outward winding and
   is why bodies must be outward-wound, which 2a already tests). Each hole is
   attached to its smallest enclosing outer loop (nested islands work), then
   each outer loop with its holes is triangulated by **ear clipping with hole
   bridging**, written in `svf-core` (no new dependency). Cap triangles face
   `+normal`.
5. Cap vertices are the loop's own new vertices, so the cap meets the wall
   exactly.

**Retry (found by live use).** A plane lying along an edge of a body (for the stem, x = +-1.8 or +-1.1) makes a cap sliver about 1e-7 wide, which the area tolerances read as zero. If a cap fails, the whole section is retried with the nudge raised from 1e-9 to 1e-6 and then 1e-4 of the extent (below anything visible). `NotClosed` is never retried.

**Tiny caps far from the origin (found with the cockroach fixture).** A plane 1e-7 above an ellipsoid's pole at x = 37 leaves a cut loop about 6e-4 wide. The cap's area check summed the shoelace formula about the plane origin, so cancellation (terms near 100, area near 1e-6) made the total disagree with the triangulated area and the cap failed. In a dense scene every larger nudge then fails too (some vertex always lies within it), so the whole cut returned `DegeneratePlane`. The area is now measured from the loop's first vertex. A unit test calls the smallest nudge directly.

Everything is f64 and graphics-free.

## 2. Layers and peeling

Format: an optional `layers` list in the structure file (version stays 1: the
field is new and defaults to empty, so existing files still load).

```ron
layers: [
    Layer(id: "skin",     name: "Epidermis",      parts: ["epidermis"]),
    Layer(id: "cortex",   name: "Cortex",         parts: ["cortex"]),
    Layer(id: "vascular", name: "Vascular ring",  parts: ["xylem", "phloem", "vascular-cambium", "medullary-rays"]),
    Layer(id: "pith",     name: "Pith",           parts: ["pith"]),
],
```

Order is outermost first, which for dissection is the order of removal.
Validation: unique layer ids, every part id exists, a part is in at most one
layer, a layer lists at least one part, a grouping node expands to its
descendants that have geometry. Parts in no layer are never peeled.

API: `Structure::layers()`, and a pure `visible_parts(isolated: Option<usize>,
peeled: usize) -> Vec<usize>` that returns the parts of every layer not yet
peeled, intersected with the isolated part if any. "Peeled" is a count of
layers removed from the outside, so restore is `peeled - 1`.

The stem is then: peel 1 removes the epidermis, peel 2 the cortex, peel 3 the
vascular ring, leaving the pith; with a transverse section this reads as the
usual concentric diagram.

## 3. Explorer (3b)

Composition: visible set (isolate and peel) -> `build_geometry_for` ->
`section_geometry` -> `replace_coloured_mesh`. Picking works on the final mesh
so a cap names its tissue.

Keys: `S` cycles section off, transverse (normal along the structure axis,
world +Y), longitudinal (normal world +X), off. `[` and `]` move the plane by
1/50 of the extent along its normal. `F` flips the kept side. `P` peels the
outermost remaining layer, `R` restores the last peeled layer. The plane starts
at the middle of the visible bounding box. Existing keys are unchanged. The
window title shows section and peel state; stderr prints the layer name.

Command line (also for screenshots): `--section <x|y|z>=<offset>`,
`--section-flip`, `--peel <n>`.

The Explorer presets are axis-aligned only; the core accepts any plane.

## Tests

3a (core):
- section of a closed box and of a cylinder: the clipped body plus cap is
  closed (each undirected edge used by exactly two triangles);
- volume on the kept side matches the analytic value (cylinder cut at height
  `h` keeps `pi r^2 h`; annulus likewise), within polygon tolerance;
- an annulus cut transversally gives a cap with a hole: cap area equals
  `pi (R^2 - r^2)` within tolerance, and the hole is empty;
- the stem cut transversally: cap areas of all parts sum to the cross-section
  area, tissues do not overlap, and each part's cap area matches its profile;
- longitudinal cut through the stem axis: every cut part has a non-empty cap and
  `part_of_triangle` names the right part for cap triangles;
- plane through a vertex (nudge) and plane entirely outside: the first still
  closes, the second returns the geometry unchanged or empty parts removed;
- a non-closed part returns `NotClosed`;
- flip keeps the other side, and both sides together tile the body;
- layers: validation rejects each error class; the shipped stem loads with its
  four layers; `visible_parts` for peel 0..4 and with isolation.

3b (Explorer): argument parsing and rejections; the composition chain picks a
cap triangle as the right part; screenshots of transverse and longitudinal
sections, and of peels 1 to 3.

## Acceptance for step 3

From the Explorer on the dicot stem, an O-level learner can cut the stem across
and along, see epidermis, cortex, bundles (xylem, phloem, cambium), rays and
pith as filled regions, click any region for its name and description, and peel
the layers from the outside in and back. This completes the stem acceptance
criteria of `explorable-structures.md` section 8.

## Risks

Capping is the stated risk. Mitigations: recipe bodies are closed and
outward-wound by test; the nudge removes the exact-hit case; loops are checked
rather than assumed; area and volume tests are analytic, not visual.
Ear clipping with holes is the most intricate code, hence the area tests on
annuli and on the whole stem.
