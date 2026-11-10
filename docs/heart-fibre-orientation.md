# B1.2 — Ventricular fibre orientation on the boundary surface (design contract)

Status: **design, revised after review round 1. Nothing here is implemented.**
The measurements in §5 came from a scratch program run on 2026-11-10. They are
not SVF features, and nothing has been committed for them.

SVF-only evaluation. Mava and Prismal are excluded.

## 1. Scientific question

Can SVF show the ventricular myofibre orientation of a real four-chamber heart
mesh faithfully enough that a viewer can check it against the source's stated
fibre rule? Fibres must not be shown where the source has none, and no
direction the data lacks may be implied.

The experiment must establish:
1. The correct ventricular fibres are selected.
2. Their directions are represented faithfully.
3. They lie along the ventricular wall.
4. The visualisation reveals where the unexplained right-ventricle triangles
   (§5) are.

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
`01.vtk` except that the CELLS and CELL_TYPES records were moved within the
file for the current parser. The order of cells inside those records is
unchanged, so cell indices are the same in both files. The independent audit
(§7) confirms this rather than assuming it. The file has 493,151 points and
2,477,866 tetrahedra. SVF imports `ID` as a cell scalar, `fibres` and `sheets`
as cell `Vector3`, and the four UVCs as point scalars.

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

Three things are kept distinct throughout:
1. the **source fibre direction** of a cell (scientific data);
2. the **geometric orientation** of a boundary triangle (derived geometry);
3. the **glyph**, the line segment used to communicate (1) (presentation).

### 4.1 Fibres are axial

A fibre direction f and -f describe the same fibre. Glyphs are therefore
**centred, unoriented line segments with no arrowhead**. Every angle involving
a fibre uses the absolute dot product, so the result is independent of f's
sign.

### 4.2 Geometric triangle normal (used)

For a boundary triangle with vertices a, b, c, the geometric normal is
n_g = (b − a) × (c − a), normalised. It is exact for that face. It needs no
neighbour information and no orientation decision: its sign depends on the
stored vertex cycle, which SVF deliberately does not certify as outward
(see `docs/boundary-surface.md` and `docs/surface-triangulation.md`). No step
in this design depends on the sign of n_g.

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
an approximation of each point's own long-axis direction. It is a coarse
sanity check, not an acceptance criterion for SVF's rendering. The RV is
excluded: the LV axis is not a valid RV reference, and a second
coordinate-frame calculation would add an assumption without answering the
questions in §1.

### 4.6 Degenerate cases

These cases are reported by count and excluded. They are never given a
fabricated value. ε = 1e-6, applied to quantities derived from unit vectors.

| Case | Effect |
|---|---|
| Triangle with ‖(b − a) × (c − a)‖ = 0 (zero area) | No normal, no θ, no glyph; counted |
| Source fibre zero or non-finite | No glyph, no θ; counted (none present in case 01) |
| Helix: ‖t‖ ≤ ε (fibre parallel to the normal) | Excluded from the \|α\| statistics only; counted |
| Helix: ‖L × n_g‖ ≤ ε (face perpendicular to the LV axis) | Excluded from the \|α\| statistics only; counted |

In case 01, every one of these counts is **0** (§5).

## 5. Baseline measurement (case 01)

Method: the svf-core parser, boundary extraction and triangulation provenance
in a scratch program. A triangle's wall group uses the mean ρ of its three
source points: endocardial ρ < 0.2, epicardial ρ > 0.8, otherwise "other".

- Boundary triangles: 329,952 in total. Of these, 171,473 come from ventricular
  cells (tag 1: 86,241; tag 2: 85,232).
- Degenerate exclusions (§4.6): 0 zero-area ventricular triangles; 0 helix
  exclusions of either kind.
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

- No LV boundary triangle falls in the 0.2 ≤ ρ ≤ 0.8 group.
- LV long axis: 88.5 mm, from 147 apex points and 1,552 base points.
- Percentiles use the nearest-rank index round((n − 1)·p) on the sorted values.

Interpretation:
- Ventricular fibres lie close to the wall surface almost everywhere: the
  median θ is 88–89° in every wall group. They are close to tangent, not
  exactly tangent.
- The LV helix magnitudes order as the source rule predicts: steeper
  (closer to longitudinal) at the endocardium (median 75.8° vs stated 80°) and
  shallower at the epicardium (median 62.5° vs stated 60°). This holds
  despite the coarse global-axis approximation.
- Not yet explained: the p99 tails (|cos| ≈ 0.3) and the 732 "other" RV
  triangles. They are probably at the base cut, the valve rings or the apex.
  Their spatial location must be shown before they are interpreted (A4).

## 6. Proposed experiment

1. **Selection:** use the boundary triangles whose source cell's `ID` value is
   in an explicitly supplied tag set ({1, 2} here), via the existing
   `TriangulatedSurface3::source_cell_index` provenance. There is no
   Rodero-specific hardcoding.
2. **Anchor:** the triangle centroid p_c = (a + b + c)/3. It is a display
   location only. The fibre is constant over the source tetrahedron, so the
   anchor does not claim where the fibre was measured.
3. **Glyph:** a centred axial segment along the **exact source fibre
   direction**:

   p_start = p_c − (ℓ/2) f̂,  p_end = p_c + (ℓ/2) f̂,  with f̂ = f / ‖f‖.

   The fibre is **not** projected onto the triangle plane. Projecting would
   change the scientific direction being shown and would conflict with A2. The
   glyph is therefore only approximately tangent to the surface, by exactly
   the θ of §4.4 (median 88–89° in case 01). ℓ is a uniform display length in
   mm and does not encode magnitude (all fibres are unit length). The segment
   has no arrowhead.
4. **Thinning by deterministic spatial binning:**
   - Use world-space cubic bins of edge h mm, with the grid origin at the
     world origin O = (0, 0, 0) of the source coordinates.
   - A selected triangle's bin index is k = (⌊(p_c,x − O_x)/h⌋,
     ⌊(p_c,y − O_y)/h⌋, ⌊(p_c,z − O_z)/h⌋). Floor is used so that negative
     coordinates bin consistently.
   - In each occupied bin, keep the eligible triangle with the **lowest
     triangulated-surface index**.
   - This index is a deterministic tie-breaker only, not a scientific
     ranking: it says nothing about which fibre is more representative.
   - Excluded triangles (§4.6) are never eligible.
5. **Parameters:** h and ℓ are configurable, with provisional defaults
   h = 4 mm and ℓ = 3 mm (relative to the 1 mm average edge length). They
   will be adjusted after visual inspection.
6. **Provenance retained:** each glyph keeps its triangle index, source cell
   index, tag, unmodified f64 source fibre, and θ.
7. **Surface colour:** optional. The existing `--vtk-scalar` path can colour
   the surface by `RHO.dat` with `--vtk-scalar-exclude -10`, so the viewer can
   relate glyph orientation to endocardium versus epicardium. This needs no
   new mapping feature.

### Placement of responsibilities (accepted in review round 1)

| Layer | Responsibility |
|---|---|
| svf-core | Select boundary triangles by cell tag; resolve cell-associated fibre vectors; compute centroids and geometric normals; thin deterministically; construct axial segments; retain provenance; report exclusion counts. |
| svf-render | Convert f64 segments to renderer coordinates and draw them with the existing line path. |
| Explorer | Parse CLI options, configure the operation, and include selection, glyph and exclusion counts in the scene summary. |

Glyph construction and selection are scientific-data operations, not graphics
operations. The implementation stays specific to this experiment; it is not a
general glyph framework. The proposed CLI, to be finalised in implementation
review, is
`--vtk-cell-axial <field> --vtk-cell-tags <field>=<v1,v2,…>
[--vtk-axial-bin <mm>] [--vtk-axial-length <mm>]`.
`--vtk-vectors` stays unchanged, for point arrows only.

### Depth handling

The first attempt uses the **existing line path unchanged**:
`CompareFunction::LessEqual`, depth writes disabled, zero depth bias
(`crates/svf-render/src/lib.rs`). Glyph positions are never altered to
compensate for rasterisation, and anchors are never offset along n_g, whose
sign is not certified outward. If the near-coplanar glyphs disappear or show
depth artefacts in screenshots, a renderer-side remedy is investigated and
proposed separately.

## 7. Verification

Synthetic tests and real-dataset validation are separate. Rodero counts and
statistics are **not** hardcoded into svf-core tests.

### 7.1 svf-core unit tests (small synthetic meshes)

- Geometric normal computation and zero-area detection.
- Axial semantics: θ is identical for f and -f, and for both triangle
  windings. The segment is centred on p_c, has length ℓ, and lies along ±f̂
  exactly.
- Tag filtering: cells outside the tag set produce no glyphs. A cell holding
  (1, 0, 0) inside the tag set does produce a glyph (selection is by tag,
  not by value).
- Binning: floor behaviour across negative coordinates, lowest-index
  tie-break, determinism across runs.
- Provenance: each glyph resolves to the correct triangle and source cell.
- Degenerate cases from §4.6 are counted and excluded.

### 7.2 Independent dataset audit (`tools/rodero-fibre-audit/`)

This is a maintained Python script, kept as reproducibility evidence beside
`tools/analytical-heat-diffusion/`. It must not consume SVF output. It:
1. reads the **original, unmodified** `01.vtk`;
2. identifies tetrahedral faces by sorted source point IDs;
3. counts face incidences and keeps faces that occur exactly once;
4. recovers each boundary face's source cell index, `ID` and fibre;
5. computes geometric normals, θ, the ρ wall groups and the LV |α| exactly as
   defined in §4–§5;
6. reports the full statistics set: boundary face count; ventricular counts
   per tag; per-group counts; |cos| p50/p90/p99; the < 0.1 and < 0.2
   fractions; LV |α| p25/p50/p75; and all §4.6 exclusion counts.

Its boundary faces are matched to SVF's by sorted point-ID triple and source
cell index, which also confirms that the two files have identical cell
indices.

### 7.3 SVF diagnostic path

A small `svf-core` example (separate from `vtk_inspect`) runs the actual
implementation on the reordered file and prints the same statistics set in
the same format.

**Matching rule:** all counts must match exactly. Fractions are ratios of
exactly matching counts, so they match exactly too. Percentiles and angles
must match within 1e-9 absolute. The audit and SVF parse the same decimal
text into f64, so only the order of floating-point operations can differ.

## 8. Acceptance criteria

- **A1 Selection:** on case 01, exactly 171,473 ventricular boundary triangles
  are eligible (86,241 LV + 85,232 RV). Zero glyphs come from tags 3–24.
- **A2 Fidelity:** every glyph lies along its source cell's exact fibre
  direction in f64 (up to sign, since the glyph is axial). Nothing is
  projected or smoothed, and provenance resolves to the right triangle and
  cell.
- **A3 Tangency:** the §7.1 synthetic tests pass, and the §7.3 SVF diagnostic
  output matches the §7.2 independent audit under the §7.3 matching rule,
  across the full statistics set.
- **A4 Visual (screenshots inspected directly):**
  - No arrowheads.
  - A spatially coherent pattern, oblique across the LV epicardium and
    steeper on the endocardium where it is visible.
  - No glyphs on the atria or vessels.
  - Locations of the high-|cos| tail triangles and the 732 RV "other"
    triangles identified.
  - Glyphs visible through the unchanged line depth path, or any artefact
    documented.
  - Rotation and zoom unaffected.
- **A5 Non-regression:** existing tests pass, and the existing VTK modes
  (`--vtk-scalar`, exclusions, `--vtk-vectors`, isosurfaces) behave as before.

## 9. Non-goals

- Sheet visualisation (a later step that reuses the same path).
- Signed helix angles, and the RV helix check (deferred, §4.5).
- Per-point local cardiac frames, consistent outward orientation, and smoothed
  normals.
- Projecting fibres onto the surface.
- Interior or volume glyphs, and fibre tracing/streamlines.
- Atrial fibres (the source has none).
- A general glyph framework.
- Mava/Prismal integration.

## 10. Review decisions (round 1)

| Topic | Decision |
|---|---|
| Core / render / Explorer split | Accepted (§6 table) |
| Depth handling | Try the existing line path first; no data offsets (§6) |
| h = 4 mm, ℓ = 3 mm | Accepted as provisional, configurable defaults |
| RV helix check | Deferred; tangency is enough for the RV |
| Independent audit | Kept in `tools/rodero-fibre-audit/`, with its own boundary extraction (§7.2) |
| Fibre direction | Exact source direction; no projection; "approximately tangent" wording (§6.3) |
| A3 | Split into synthetic unit tests, independent dataset audit, and SVF diagnostic (§7) |
| Degenerate geometry | Explicit counted exclusions (§4.6) |
| Probe files | Consolidate into `crates/svf-core/examples/vtk_inspect.rs`, a small diagnostic only (approved; implementation follows the final design review) |
