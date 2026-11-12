# B2 step 5 — the heart, adapted from Rodero case 01

Implements step 5 of `explorable-structures.md`: the **adapted** geometry route,
a light structure derived from a dataset. Proves that a real patient-derived
tetrahedral heart can be turned into the same kind of structure as the dicot
stem and the cockroach: named parts, layers, views, sections, picking. Split
into **5a** audit, **5b** region surfaces, **5c** simplifier, **5d** mesh blocks
and provenance in the format, **5e** the adaptation tool, the fixture and the
live check.

Decisions made with the user (2026-11-12): chamber **cavities** are built as
parts; the heart is **re-oriented** to a heart frame; the derived mesh is
**committed** to the repository with attribution.

## 1. Scope

The learner can name each part of the heart, turn it in 3D, remove the walls to
see the four chambers as shapes with the valve positions between them, and cut
a four-chamber and a short-axis view.

Out: valve leaflets, chordae, papillary muscles, coronary vessels (none are in
the data), the ventricular fibres (B1.2, separate), the lumens of the aorta and
pulmonary artery (their ends are open), animation, and the other 19 cases. The
heart is one individual's end-diastolic anatomy, not a canonical heart; the
structure says so.

## 2. Source facts this design relies on

From `docs/biology-heart-anatomy-source-audit.md` and
`tools/rodero-fibre-audit/baseline/case01.json`:

- Case 01: 493,151 points, 2,477,866 tetrahedra, millimetres, CC BY 4.0
  (Rodero et al. 2021, DOI 10.5281/zenodo.4590294; Strocchi et al. 2021, DOI
  10.1371/journal.pcbi.1008851). Source SHA-256 of `01.vtk` is pinned in the
  baseline.
- Cell tags: 1 to 4 LV, RV, LA, RA myocardium; 5 aorta wall; 6 pulmonary artery
  wall; 7 to 10 mitral, tricuspid, aortic, pulmonary valve **planes**; 11 to 17
  appendage and vein inlets; 18 to 24 their borders.
- Exposed boundary triangles per tag run from 86,241 (LV) down to 492 (left atrial
  appendage inlet); 329,952 in all. Every face is used once (boundary) or twice
  (interior) by tetrahedra, so the mesh is conforming.
- There is no blood-pool mesh: the chambers are empty space between the walls.
  The valve planes and the inlets are one-element-thick slabs that close the
  orifices and vein ends, so the chambers should be enclosed (checked in 5a).
- Valves are planes, not cusps. Every part description says what the part is in
  the data, not what the organ is in life.

## 3. Heart frame

The scanner frame has the left ventricle's long axis oblique to every axis, and
section planes are axis-aligned. The adaptation tool therefore applies one rigid
transform (rotation and translation, no scaling, millimetres kept), computed
from the tags and recorded in the provenance:

- **Y**: the left ventricle's long axis, apex to base. Base = the mean of the
  mitral and aortic plane centroids. Apex = the left ventricle point farthest
  from that base.
- **X**: the part of the vector from the LV centroid to the RV centroid that is
  perpendicular to Y, normalised.
- **Z**: X cross Y, so the frame is right-handed.
- **Origin**: the centroid of the four valve planes.

A plane normal to Z through the origin is then a four-chamber cut, a plane
normal to Y a short-axis cut, and the existing `S`, `[`, `]` and `F` keys give
them with no format change. The Explorer's section axes are world axes, as for
the other structures.

## 4. Parts (21 with geometry)

| Group | Parts |
|---|---|
| Walls | left ventricle, right ventricle, left atrium, right atrium, aorta, pulmonary artery (tags 1 to 6) |
| Valve planes | mitral, tricuspid, aortic, pulmonary (tags 7 to 10) |
| Vessel ends | four pulmonary veins, superior and inferior vena cava, left atrial appendage (each an inlet tag with its border tag) |
| Cavities | left ventricle, right ventricle, left atrium, right atrium (derived, section 5) |

Names of vessel ends say "inlet segment" where that is all the data holds. The
cavities are coloured by the usual convention (blue right side, red left side),
stated in their descriptions. The aorta and pulmonary artery lumens are not
built.

## 5. Cavities

A chamber cavity is the empty space the valve planes and inlet slabs enclose.
Its surface is a **closed boundary component** of the union of the tetrahedra
(an inner component, separate from the outer surface). Reversing the winding of
that component gives an outward-wound body, because the tetrahedra's outward
direction points into the cavity.

Gate in 5a: exactly four inner components, each with a plausible volume (a few
hundred millilitres at most). If a slab does not seal, the opening is capped
with a planar fan and the fix is recorded in the provenance; if more than one
opening is unsealed, stop and report.

## 6. Simplify patches, not parts

The source surface is about 330,000 triangles. The labelled complex is every
face whose two sides differ in tag or in void. It is split into **patches**:
maximal sets of faces with the same pair of side labels (for example "LV wall
against LV cavity"). Patch boundary curves are pinned. Each patch interior is
simplified once by edge collapse with quadric error (`mesh_simplify` in
`svf-core`): no collapse may flip a triangle, no collapse may break the link
condition (so no pinches), and no collapse may exceed the error bound
(`epsilon`, default 0.4 mm). Each part is then the union of its patches, wound
by side, so neighbouring parts share the very same triangles.

Why not the alternatives:

- **Vertex clustering** collapses the 1 mm valve slabs and the 2 mm atrial
  walls.
- **Per-part decimation** leaves hairline cracks and overlaps wherever two parts
  meet, visible in a cut.

Consequence: every part is closed, and gaps or overlaps between parts cannot
happen, by construction.

## 7. Orientation and closure

Each tag region's boundary comes from the face incidence of the existing
`volume_topology()` (cells carry their tag). The outward direction of a boundary
face is decided **geometrically** (against the vertex of the tetrahedron
opposite the face), never from file order, because the source does not certify
orientation.

Where two parts of a region touch along an edge or at a vertex the boundary is
not a 2-manifold and cannot be capped. 5a found 97 such edges and 18 such
vertices in case 01 (section 15), so the repair is part of the design: at a
non-manifold edge the faces are sorted by angle round the edge and paired along
the solid wedges, so each pair becomes a manifold sheet; a non-manifold vertex
is duplicated once per connected face fan. The repair runs per part when the
part is assembled, on the part's own vertex list, so the shared patch triangles
stay geometrically identical. Single-cell islands (a tag region of one
tetrahedron touching its region only along an edge) are relabelled to the
neighbouring tag with the largest shared face area before extraction.

## 8. Format additions

All optional, so `version` stays 1 and existing files load unchanged.

- `Geometry::Mesh(file: "heart-case01.svfmesh", body: "left-ventricle")`: a body
  from a mesh container. It is the adapted route now and the imported route later.
- **`.svfmesh` container**: magic, body count, then per body a name, f32
  positions and u32 triangle indices, and a FNV-1a 64-bit checksum of the file
  (integrity, not security; no new dependency). Positions are f32 on disk and
  f64 in memory.
- **Loading** is `Structure::from_ron_with(text, resolver)`, where `resolver`
  turns a file name into bytes. svf-core stays free of file IO and
  `from_ron_str` is unchanged for recipe-only files. The Explorer resolves paths
  next to the structure file.
- A mesh body must be closed and outward-wound, otherwise loading fails with an
  error that names the part.
- `provenance: Some(Provenance(...))`: dataset name, DOI, licence, required
  attribution, source SHA-256, tool name and commit, parameters (`epsilon`, tag
  groups), the frame transform, per-body source volume in mm3, and "modified:
  yes". The Explorer prints provenance and attribution at startup.
- `copies` (`repeat`, `mirror`) are accepted on a mesh part like any other.

## 9. Layers (outermost first)

1. Great vessels and vein ends (aorta, pulmonary artery, pulmonary veins, venae
   cavae, left atrial appendage).
2. Atria (left and right atrium walls).
3. Right ventricle wall (it lies in front).
4. Left ventricle wall.

Valve planes and cavities are in no layer. Peeling everything therefore leaves
the four chambers as casts with the valve planes between them, which is the
view that teaches where the valves sit.

## 10. Views

Anterior; four-chamber section; short-axis section; walls peeled (chambers and
valves); valve planes only; great vessels. Camera values are relative to the
fitted camera, tuned in the Explorer and recorded, as for the cockroach.

## 11. Acceptance and checks

- Every part is closed and outward-wound.
- Per-part volume within 1 percent of the exact source volume: the sum of the
  tetrahedra of that tag for walls, planes and vessel ends; the enclosed volume of
  the boundary component for cavities.
- Patch error below `epsilon`, by sampling source points against the simplified
  patch and the reverse.
- Interface edges are shared exactly: every edge of a part lying on a patch
  boundary is used by the same triangles in the neighbouring part.
- About 60,000 triangles in all and under 2 MB; the tool reports the real
  numbers and, if over budget, says so and raises `epsilon` explicitly.
- Fixture overlap test (ray parity, inset 1e-3 mm) passes with an **empty**
  allow-list.
- Plane sweeps on all three axes: every cut succeeds and is closed; no cut over
  100 ms in a release build.
- Screenshots of each view, and the user's live check.

## 12. Risks

- A cavity that is not sealed, or tag regions that touch at an edge (gate in
  5a).
- Edge-collapse quality with pinned curves: thin triangles near them. If the
  budget is missed the answer is a larger `epsilon`, reported, not a hidden
  loosening.
- Parsing 260 MB and about 900 MB of memory: seen before (about 8 s to read and
  parse); the tool runs offline, never in tests.
- The CC BY 4.0 attribution wording; the structure carries the required text and
  says the mesh is modified.
- The data is one individual's heart; descriptions say what the data holds.

## 13. Work, one PR each

1. **5a** an audit example on case 01 (no production API): non-manifold edges and
   vertices per tag, inner boundary components and their volumes, patch count
   and pinned-curve length, an estimate of the triangle budget. Its results settle
   the gates in sections 5 to 7 and are written into the handoff.
2. **5b** region surfaces, patches and cavity extraction in `svf-core`, with
   synthetic tetrahedral tests.
3. **5c** `mesh_simplify`: quadric edge collapse with pinned boundaries, tested on
   synthetic closed shapes and cut patches (closure, volume, no flips, error).
4. **5d** `Geometry::Mesh`, the container, the resolver, the provenance block, and
   Explorer file loading.
5. **5e** the adaptation tool, its run on case 01, the committed fixture and mesh,
   fixture tests, screenshots, and the user's live check.

## 14. Acceptance

An A-level learner, from the Explorer alone, can name the four chambers, the great
vessels and the valve planes; remove the walls in order to see the chambers as
casts; and cut a four-chamber and a short-axis view in which walls and chambers
read as filled regions. No heart-specific Explorer code is needed beyond the
generic mesh loading and the attribution line.

## 15. 5a results on case 01

Run of `crates/svf-core/examples/heart_audit.rs` on `01-svf-order.vtk` (report in
`docs/evidence/b2-5a-heart-audit/case01-audit.txt`; reading and parsing 6 to 7
s, face map 4 s, the whole run well under a minute).

- **Conforming, no degenerates.** 5,120,708 unique faces; none is used by more
  than two tetrahedra; no zero-volume tetrahedron. The labelled complex has
  369,926 faces: 329,952 exterior and 39,974 between tags.
- **Orientation rule confirmed.** Orienting each face against the opposite
  vertex of its tetrahedron makes every tag region's boundary consistently
  wound, and its divergence-theorem volume equals the exact sum of tetrahedron
  volumes to better than 1e-13 relative for all 24 tags. No face is ambiguous.
- **Cavities: the gate passes.** The exterior surface has five components:
  the outer surface (785 mL enclosed) and exactly four inner ones, each sealed
  by valve planes and inlet slabs:

  | Cavity | Volume | Bounded by tags |
  |---|---|---|
  | Right ventricle | 181.7 mL | 1, 2, 8 (tricuspid), 10 (pulmonary) |
  | Left ventricle | 120.1 mL | 1, 7 (mitral), 9 (aortic) |
  | Right atrium | 112.4 mL | 4, 8, 16 and 17 (venae cavae inlets) |
  | Left atrium | 68.5 mL | 3, 7, 11 to 15 (appendage and pulmonary vein inlets) |

  The volumes are plausible for end-diastole. No capping fallback is needed.
- **Non-manifold: the gate fails as feared, and is repairable.** 97 edges and
  18 vertices over all regions (tag 2 has 22 and 10, the most; 29 exterior
  edges). Every non-manifold edge is balanced (as many forward as backward
  uses), so pairing is always possible. Nine regions-of-one-tetrahedron islands
  (tag 2 four, tag 8 five) account for the extra components; they are relabelled.
- **Patches.** 65 side-tag pairs, 286 patches; 197 of those have fewer than ten
  faces (496 faces together, pinches and islands), 28 hold 346,550 of the
  369,926 faces. Pinned curves: 6,503 edges, 5,582 mm, 6,341 vertices (a floor of
  about 12,700 triangles). Patches under ten faces are kept as they are.
- **Budget.** Mean source edge 0.92 mm. An area-only estimate for the labelled
  complex is 35,000 triangles at a 3 mm mean edge and 79,000 at 2 mm. Each
  complex face belongs to one part (outer surface, 163,704 faces) or two (every
  cavity surface and every tag interface), so the parts add up to about 1.56
  times the complex: 55,000 triangles at 3 mm, 123,000 at 2 mm. The target of
  about 60,000 is therefore realistic at roughly 3 mm; the final number is
  decided by `epsilon` in 5c and reported.

## 16. 5b as built and its results on case 01

`svf-core::region_surface` (checked on the real mesh by
`crates/svf-core/examples/heart_regions.rs`; report in
`docs/evidence/b2-5b-heart-regions/case01-regions.txt`).

- **`relabel_islands(grid, tags, max_island_cells)`**: components of one tag joined
  through shared faces; each tag keeps its largest; any other component under the
  limit goes to the neighbouring tag with the most shared face area. With the
  limit at 8, case 01 has **46** such strays (the audit's 9 counted only the
  ones separate by edges): single cells, a few of 2 or 4. They are relabelled
  before extraction.
- **`LabelledSurface::build(grid, tags)`**: 369,806 faces, each wound from its
  `front` label to its `back` label (`front < back`), void = 0. Enclosed voids
  get the labels `CAVITY_BASE + k`. Cavities come out as in the audit (181.7,
  120.1, 112.4 and 68.5 mL), now relabelled so a cavity is just another region.
  `patches()` gives 197 patches after relabelling; `region_triangles(labels)` is
  the closed outward-wound surface of any label set.
- **`repair_pinches(points, triangles)`**: pairs the faces round a non-manifold
  edge along the solid wedges, then duplicates vertices per connected fan.
  One edge in tag 2 (the right ventricle) is a **handle through the pinch**:
  the fans round both ends are connected anyway, so the wedge pairing leaves it
  non-manifold, and the crossing pairing separates it. The repair tries the next
  pairing for any edge left unresolved (up to four), and reports how many edges
  needed it; a synthetic test builds the same shape.
- **Result.** All 21 parts (10 wall and plane tags, 7 vessel ends as inlet tag
  plus border tag, 4 cavities) come out as closed, consistently wound
  2-manifolds whose volume equals the exact tetrahedron sum to better than 1e-13;
  repair takes about a second. The parts add up to 572,362 triangles before
  simplification (the estimate in section 15 was about 577,000).
- **Vessel-end pairing is by adjacency, not by name.** The inlet and border
  tags that touch are 11 and 18, 12 and 19, 13 and 20, 14 and 21, 15 and 22,
  16 and 23, 17 and 24. The source's published list of border names is not in
  the same order as the inlet names, so 5e names the pulmonary vein parts by
  their position in the heart frame (left or right, superior or inferior), not
  from the tag number.

## 17. 5c as built (`svf-core::mesh_simplify`)

- **`simplify(points, triangles, pinned, options)`**: quadric edge collapse of a
  consistently wound 2-manifold surface, open or closed. Open-boundary vertices
  and any caller-pinned vertex keep their exact coordinates, and a collapse never
  joins two pinned vertices, so patches simplified apart still share their
  boundary exactly. `Simplified::vertex_map` gives each output vertex's source
  vertex, which is how 5e merges patches into parts.
- **Refused collapses**: link condition (the endpoints share exactly the two
  apexes of the edge, so no topology change and no pinch; tested on a torus, genus
  kept), flips (a surviving triangle's normal must keep cosine at least
  `min_normal_cos`, default 0.1, and not degenerate), and the error limit. A closed
  surface is never taken below four triangles.
- **The error limit is a heuristic.** Each vertex sums squared distances to the
  unit-normal planes of the source triangles it stands for; the limit is applied to
  the **root mean square** of that. The plain sum would be a true bound on the
  distance to those planes but grows with the number of planes and stalls
  simplification early (a radius-10 sphere stopped at half its triangles for 0.3).
  With the RMS rule the measured deviation on that sphere was 0.515 for
  `epsilon` 0.3, so the limit is not a bound on the deviation.
- **`sampled_deviation(a, b)`** measures it: the larger one-way distance between
  sample points (vertices, edge midpoints, centroids) of one surface and the
  triangles of the other, grid-accelerated.
- **`simplify_within(points, triangles, pinned, max_deviation, rounds, options)`**
  makes the limit real: it searches `epsilon` (doubling, then bisecting) for the
  smallest result whose measured deviation is within `max_deviation`, recording
  `measured_deviation` and `epsilon_used`; if nothing qualifies it returns the
  source unchanged. 5e uses this per patch with 0.4 mm, as section 11 requires.
- Deterministic (ties by vertex index). Not done: any use on the heart (5e), the
  file format (5d). Budget is still decided on the real data in 5e.

## 18. 5d as built (`svf-core::structure_mesh`, `Geometry::Mesh`)

- **`.svfmesh`** (little endian): `SVFMESH1`, body count, then per body name,
  point count, triangle count, `f32` points, `u32` triangles; last an FNV-1a 64-bit
  checksum of everything before it. `MeshContainer::to_bytes` and `from_bytes` both
  validate: finite points, indices in range, every body a closed 2-manifold with
  positive volume (outward), unique body names; the reader verifies the checksum
  first and checks counts against the remaining length before allocating.
  Bytes are deterministic.
- **`Geometry::Mesh { file, body }`** is an ordinary geometry: `repeat`, `mirror`,
  layers, views, relations, sections and picking all apply unchanged. Points are
  used as stored (no welding).
- **Loading.** `Structure::from_ron_with(text, resolver)` reads each distinct file
  once through `resolver: &dyn Fn(&str) -> Result<Vec<u8>, String>`; svf-core does
  no file IO. `from_ron_str` is unchanged for recipe files and refuses a mesh part
  (`MeshNeedsResolver`). Errors name the file or the part and body.
- **Provenance** is an optional `provenance: Some(Provenance(...))` block: dataset,
  DOI, licence, required attribution, source SHA-256 (64 hex), tool, tool commit,
  parameters, optional frame transform (3x4), per-body source volumes, `modified`.
  `Structure::provenance()` returns it; `Provenance::summary()` gives the startup lines.
- **Explorer** resolves files next to the structure file (plain relative names only,
  no `..` or absolute paths) and prints the provenance summary to stderr at load.
- Not done: the container writer's use on real data (5e), any capability to read
  the container elsewhere (imports are a later route).
