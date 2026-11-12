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
not a 2-manifold and cannot be capped. 5a counts these per tag. If there are
any, the pinched vertices are split per face fan before simplification.

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
