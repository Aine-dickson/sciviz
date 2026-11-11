# B2 step 2 — structure model and the dicot stem recipe

Implements step 2 of `explorable-structures.md`. Proves: a structure with no
dataset can be authored as a small text file, shown, picked and isolated.
Split into two PRs: **2a** core model, blocks and tests (no GUI); **2b**
Explorer loading, picking, isolating, plus the renderer coloured-mesh API.

## Scope

In: parts tree, recipe geometry, RON file, validation, dicot stem example.
Deferred (the format rejects them for now, so they cannot be silently lost):
relations, layers, views, labels (later steps), adapted and imported geometry
(step 5). Sections and peeling are step 3; **the transverse layout is already
visible in step 2 by looking down the stem axis**, because the end caps of the
tissues tile the cross-section.

## File format (RON, versioned)

```ron
Structure(
    version: 1,
    name: "Dicot stem (transverse layout)",
    units: "mm",
    reference: "<textbook / syllabus the layout follows>",
    circle_segments: 48,
    parts: [
        Part(id: "epidermis", name: "Epidermis", parent: None,
             description: "...", colour: (0.35, 0.65, 0.30),
             geometry: Extrusion(axis_length: 20.0,
                 profile: Annulus(inner: 4.6, outer: 5.0))),
        Part(id: "phloem", name: "Phloem", parent: Some("vascular-bundles"),
             ..., geometry: Extrusion(axis_length: 20.0,
                 profile: Repeat(count: 8, first_angle_deg: 0.0, of: Sector(
                     inner: 3.0, outer: 3.4, width_deg: 28.0)))),
        ...
    ],
)
```

All types use `deny_unknown_fields`. `parent` references another part id
(tree: organism, then organ, then tissue). A part with no geometry
(`geometry: None`) is a grouping node such as "Vascular bundles".

## Recipe blocks needed for the stem

One block, `Extrusion`: a 2D profile swept along +Y from 0 to `axis_length`,
centred on the Y axis. Profiles:

| Profile | Meaning |
|---|---|
| `Disc(radius)` | pith |
| `Annulus(inner, outer)` | epidermis, cortex, ring zones |
| `Sector(inner, outer, width_deg)` | one bundle tissue or ray, centred on `first_angle_deg` |
| `Repeat(count, first_angle_deg, of: Sector)` | N sectors evenly spaced round the axis |

Each extrusion is a **closed** triangle mesh: outer wall, inner wall (if any),
two end caps, consistent outward winding. A repeated profile is several closed
bodies belonging to one part (picking returns the part, not the copy).
Tissues must partition the cross-section (no overlap) so coincident faces
never z-fight; the example fills the vascular ring with xylem, phloem and
medullary-ray sectors for that reason.

## Core API (svf-core, graphics-free, f64)

`structure` module:

- `StructureDocument` — serde types matching the file; `from_ron_str`.
- `Structure` — validated: unique non-empty ids, parents exist and form a
  tree, colours in [0,1], positive finite dimensions, `inner < outer`,
  `0 < width_deg < 360/count`, `count >= 1`, `circle_segments >= 8`.
- `Structure::build_geometry() -> StructureGeometry`: one merged
  `TriangleMesh3`, a per-vertex colour list, and `parts: Vec<PartRange>`
  (triangle range per part). Visible subsets: `geometry_for(&[PartId])`.
- Errors are typed enums with the offending part id; no panics.

New dependencies in `svf-core`: `serde` (derive) and `ron`, pinned exactly and
added to `docs/DEPENDENCIES.md`. This is the RON decision already accepted.

## Explorer (2b)

- `--structure <file.ron>`; camera fitted to the mesh bounds by the existing
  fit; headlight on (colour is categorical, not a measured value).
- Renderer: `new_with_coloured_mesh(window, mesh, colours)` and
  `replace_coloured_mesh(...)` (reallocates buffers; the existing
  `update_mesh` requires equal counts). The mapped-surface path is not reused
  because it is scalar-specific.
- Pick: click casts a ray (`intersect_mesh`), maps the triangle to a part via
  the ranges, and shows the part name and description in the window title and
  stderr, as the other modes do.
- Isolate: `I` isolates the picked part, `A` shows all. Isolation rebuilds the
  visible mesh; picking then runs on the visible mesh.

## Example and provenance

`apps/explorer/examples/dicot-stem.ron`, hand-authored, a few KB. Its
`reference` field must name the source the layout follows. The layout is the
standard transverse arrangement (epidermis, cortex, a ring of vascular
bundles with phloem outside xylem, pith); **the precise citation is to be
confirmed by the user, and the proportions are schematic, not measured.**

## Tests (2a)

- validation rejects each error class (duplicate id, unknown parent, cycle,
  bad colour, non-positive size, `inner >= outer`, overlapping repeats,
  unknown field, wrong version);
- every generated body is closed: each undirected edge is used by exactly two
  triangles;
- volume of an annulus prism matches `pi (R^2 - r^2) L` within polygon
  tolerance; a disc likewise; sector volume likewise;
- part triangle ranges are contiguous and cover the mesh exactly;
- the shipped example parses, validates and has the expected parts.

## Acceptance for step 2

An O-level learner, from the Explorer view down the axis and an oblique view,
can name epidermis, cortex, vascular bundles (phloem vs xylem by colour and
name on pick), medullary rays and pith, and see that the bundles form a ring.
Transverse and longitudinal sections as filled regions are step 3.
