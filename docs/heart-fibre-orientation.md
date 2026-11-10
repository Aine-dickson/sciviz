# B1.2 — Ventricular fibre orientation on the boundary surface (design contract)

Status: **design proposal for review. Nothing here is implemented.** The
measurements below came from scratch scripts run on 2026-11-10. They are not
SVF features and nothing has been committed for them.

SVF-only evaluation. Mava and Prismal are excluded.

## 1. Scientific question

Can SVF show the ventricular myofibre orientation of a real four-chamber heart
mesh faithfully enough that a viewer can check it against the source's stated
fibre rule? Fibres must not be shown where the source has none, and no
direction the data lacks may be implied.

## 2. Source facts

Dataset: Rodero et al. (2021) healthy four-chamber cohort, case 01. Zenodo
record https://zenodo.org/records/4590294 (CC BY 4.0). The associated paper is
Rodero et al., *Linking statistical shape models and simulated function in the
healthy adult human heart*, PLOS Comput Biol 2021,
doi:10.1371/journal.pcbi.1008851.

The record states:
- "Ventricular fibres were generated using a rule-based method … with a fibre
  orientation varying transmurally from endocardium to epicardium from 80˚ to
  -60˚, respectively."
- ρ (`RHO.dat`) is "a transmural coordinate … from 0 at the endocardium to 1 at
  the epicardium". Z runs from 0 at the apex to 1 at the base. V is -1 in the
  LV and +1 in the RV. Non-ventricular labels use -10 for all UVCs.
- Coordinates are in mm, with an average edge length of 1 mm.
- The record does **not** describe the sheet vectors and does **not** describe
  fibres outside the ventricles.

Working file: `~/Downloads/rodero2021/01-svf-order.vtk`. It is identical to
`01.vtk` except that the CELLS and CELL_TYPES records were reordered for the
current parser. It has 493,151 points and 2,477,866 tetrahedra. SVF imports
`ID` as a cell scalar, `fibres` and `sheets` as cell `Vector3`, and the four
UVCs as point scalars.

## 3. Data hazard: placeholder vectors

An independent Python audit (not using SVF's parser) of all 2,477,866 cells
found:

| Tags | Cells | `fibres` | `sheets` |
|---|---|---|---|
| 1 LV myocardium | 1,340,385 | 1,195,444 distinct unit vectors | 1,214,019 distinct |
| 2 RV myocardium | 508,707 | 480,352 distinct unit vectors | 486,640 distinct |
| 3–24 (atria, great-vessel walls, valve planes, inlets, borders) | 628,774 | **all exactly (1, 0, 0)** | **all exactly (0, 1, 0)** |

Every vector in every tag is unit length, and every fibre is perpendicular to
its sheet. The placeholders therefore look like valid data. Unlike the `-10`
UVC sentinel, nothing in the value marks them as invalid.

**Decision:** select cells by the anatomical `ID` tag (1 and 2). Do not
exclude cells by matching the value (1,0,0): a real fibre could legitimately
have that direction, and the source defines fibres per region, not per value.

## 4. Definitions

### 4.1 Fibres are axial

A fibre direction f and -f describe the same fibre. Glyphs must therefore be
**centred, unoriented line segments with no arrowhead**. Every angle involving
a fibre uses the absolute dot product, so the result is independent of f's
sign.

### 4.2 Geometric triangle normal (used)

For a boundary triangle with vertices a, b, c, the geometric normal is
n_g = (b − a) × (c − a), normalised. It is exact for that face. It needs no
neighbour information and no orientation decision: its sign depends on the
stored vertex cycle, which SVF deliberately does not certify as outward
(see `docs/boundary-surface.md` and `docs/surface-triangulation.md`).
Triangles with ‖n_g‖ = 0 are reported and skipped, never assigned a normal.

### 4.3 Smoothed surface normal (not used in this increment)

A smoothed normal is a weighted average of the geometric normals of the
triangles around a vertex or region. It differs from n_g, especially on a
tetrahedral mesh, where the boundary is faceted at roughly the 1 mm edge scale
and dihedral angles between neighbouring faces vary. It also needs a
**consistent orientation**: averaging normals with mixed signs can cancel them
out. SVF currently has neither consistent orientation nor smoothing.

The two normals are not interchangeable. This increment uses only n_g, which
also pairs naturally with the data: each boundary triangle is a face of
exactly one source tetrahedron, and the fibre is constant over that
tetrahedron. A smoothed-normal comparison is out of scope until a use case
requires it. It would first need an orientation-consistency design.

### 4.4 Surface-tangency angle

For a selected boundary triangle with source cell fibre f:

θ = arccos( |f · n_g| / (‖f‖ ‖n_g‖) ),  θ ∈ [0°, 90°]

θ = 90° means the fibre lies in the triangle's plane. The absolute value makes
θ invariant under f → -f and under n_g → -n_g.

### 4.5 Helix-angle magnitude (diagnostic only, LV only)

Project the fibre onto the face plane, t = f − (f·n_g) n_g. Take the
circumferential direction as c = L × n_g, where L is a single global LV
long-axis direction (the mean position of the LV points with Z > 0.98 minus
that of the LV points with Z < 0.02, using V < 0). Then

|α| = arccos( |t̂ · ĉ| ),  |α| ∈ [0°, 90°]

0° is circumferential and 90° is longitudinal. This is a **magnitude only**.
The signed helix angle (+80° vs -60°) needs consistent outward orientation and
a handedness convention, which SVF does not have. A global axis is also only
an approximation of each point's own long-axis direction. It is used only as a
coarse sanity check. It is not an acceptance criterion for SVF's rendering.

## 5. Baseline measurement (case 01)

Method: the svf-core parser, boundary extraction and triangulation provenance
in a scratch program. A triangle's wall group uses the mean ρ of its three
source points: endocardial ρ < 0.2, epicardial ρ > 0.8, otherwise "other".

- Boundary triangles: 329,952 in total. Of these, 171,473 come from ventricular
  cells (tag 1: 86,241; tag 2: 85,232).
- Over all ventricular boundary triangles, the fraction with |cos| < 0.1
  (θ > 84.26°) is **0.9344**, and the fraction with |cos| < 0.2 (θ > 78.46°)
  is **0.9774**.

| Group | Triangles | \|cos\| p50 | p90 | p99 | \|α\| p25 | p50 | p75 |
|---|---|---|---|---|---|---|---|
| LV endo (ρ<0.2) | 35,339 | 0.0350 | 0.1056 | 0.2829 | 68.3° | 75.8° | 82.9° |
| LV epi (ρ>0.8) | 50,902 | 0.0170 | 0.0623 | 0.3567 | 54.1° | 62.5° | 72.7° |
| RV endo (ρ<0.2) | 40,904 | 0.0213 | 0.0695 | 0.3066 | — | — | — |
| RV epi (ρ>0.8) | 43,596 | 0.0225 | 0.0578 | 0.2989 | — | — | — |
| RV other | 732 | 0.1379 | 0.5501 | 0.9092 | — | — | — |

- LV long axis: 88.5 mm, from 147 apex points and 1,552 base points.
- RV helix magnitudes are not computed, because the LV axis is not a valid RV
  reference.

Interpretation:
- Ventricular fibres lie in the wall surface almost everywhere: the median θ
  is 88–89° in every wall group.
- The LV helix magnitudes order as the source rule predicts: steeper
  (closer to longitudinal) at the endocardium (median 75.8° vs stated 80°) and
  shallower at the epicardium (median 62.5° vs stated 60°). This holds
  despite the coarse global-axis approximation.
- Not yet explained: the p99 tails (|cos| ≈ 0.3) and the 732 "other" RV
  triangles. They are probably at the base cut, the valve rings or the apex.
  Their spatial location must be shown before they are interpreted (see
  acceptance criterion A4).

## 6. Proposed experiment

1. **Selection:** use the boundary triangles whose source cell has `ID` 1 or 2,
   via the existing `TriangulatedSurface3::source_cell_index` provenance. The
   tag set is supplied explicitly; there is no Rodero-specific hardcoding.
2. **Anchor:** the triangle centroid, with the glyph drawn in the triangle's
   plane. The anchor is a display location. It does not claim where the fibre
   is measured, because the fibre is constant over the source tetrahedron.
3. **Glyph:** a centred axial segment, `anchor ± (ℓ/2) f̂`, with no arrowhead.
   ℓ is a uniform display length in mm and does not encode magnitude (all
   fibres are unit length).
4. **Thinning:** deterministic spatial binning. Use cubic bins of edge h mm in
   world space. In each bin, keep the selected triangle with the smallest
   triangulated-surface index. Unlike source-order striding, this gives
   roughly uniform density and stays reproducible. h and ℓ are explicit
   parameters, with proposed defaults h = 4 mm and ℓ = 3 mm, to be adjusted
   after visual review.
5. **Provenance retained:** each glyph keeps its triangle index, source cell
   index, tag and unmodified f64 source fibre.
6. **Surface colour:** optional. The existing `--vtk-scalar` path can colour
   the surface by `RHO.dat` with `--vtk-scalar-exclude -10`, so the viewer can
   relate glyph orientation to endocardium versus epicardium. This needs no
   new mapping feature.

### Placement of responsibilities (to be confirmed in review)

- **svf-core:**
  - Axial glyph construction: centred segment, no head, f64, with validation
    of zero and non-finite vectors. This is scientific semantics, so it does
    not belong in the renderer.
  - Boundary-face sampling of cell-associated vectors, with tag selection and
    deterministic binning. It returns per-glyph provenance.
- **svf-render:** no new pipeline, because the existing line path draws
  segments. Lines lying exactly in an opaque face's plane may z-fight. Open
  question: either use a renderer-side depth bias for glyph lines, or offset
  the anchor along n_g. The n_g sign is not certified outward, so an offset
  could go into the wall; a depth bias is preferred because it changes no
  data.
- **Explorer:** one new imported-scene option, for example
  `--vtk-cell-axial <field> --vtk-cell-tags <field>=<v1,v2,…>
  [--vtk-axial-bin <mm>] [--vtk-axial-length <mm>]`. The exact CLI is for
  review. `--vtk-vectors` stays unchanged, for point arrows only.

## 7. Acceptance criteria

- **A1 Selection:** on case 01, exactly 171,473 ventricular boundary triangles
  are eligible (86,241 LV + 85,232 RV). Zero glyphs come from tags 3–24.
- **A2 Fidelity:** every glyph direction equals its source cell's fibre in f64
  (up to sign, since the glyph is axial). Its provenance resolves to the right
  triangle and cell.
- **A3 Tangency, reproduced independently:** the θ statistics in §5 are
  recomputed by an SVF unit/integration check **and** by an independent
  script with its own boundary extraction. The θ > 84.26° fraction matches
  0.9344 in both, within 1e-4.
- **A4 Visual (screenshots inspected directly):**
  - No arrowheads.
  - A spatially coherent pattern, oblique across the LV epicardium and
    steeper on the endocardium where it is visible.
  - No glyphs on the atria or vessels.
  - Locations of the high-|cos| tail triangles identified.
  - Rotation and zoom unaffected.
- **A5 Non-regression:** existing tests pass, and the existing VTK modes
  (`--vtk-scalar`, exclusions, `--vtk-vectors`, isosurfaces) behave as before.

## 8. Non-goals

Sheet visualisation (a later step that reuses the same path), signed helix
angles, per-point local cardiac frames, consistent outward orientation,
smoothed normals, interior or volume glyphs, fibre tracing/streamlines,
atrial fibres (the source has none), generalisation beyond what this dataset
needs, and Mava/Prismal integration.

## 9. Open questions for review

1. Is the core/Explorer split in §6 acceptable?
2. Should glyph lines get a depth bias, or should another way of avoiding
   z-fighting be used?
3. Defaults for h (bin size) and ℓ (glyph length).
4. Is an RV-specific helix check wanted, or is tangency enough for the RV?
5. Should the independent A3 script live in the repository (for example
   `tools/rodero-fibre-audit/`) beside the heat-diffusion verifier?
