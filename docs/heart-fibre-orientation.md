# B1.2 — Ventricular fibre orientation on the boundary surface (design contract)

Status: **design approved (review round 4). Implementation follows the order
in §13: stages 1–3 are merged; stage 4 (diagnostic and verification) is in
progress, with the numeric input correction of §15.**
The measurements in §5 first came from a scratch program run on 2026-11-10.
Stage 2 reproduced them with the committed independent audit
(`tools/rodero-fibre-audit/`), whose reference output is
`baseline/case01.json`. They are measurements, not SVF features.

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
that of the LV points with Z < 0.02). An LV point is one with V = −1
exactly. A bare V < 0 test is wrong, because the −10 sentinel (V = Z = −10)
would admit 138,056 non-ventricular points as apex points. Then

|α| = arccos( |t̂ · ĉ| ),  |α| ∈ [0°, 90°]

0° is circumferential and 90° is longitudinal. This is a **magnitude only**.
The signed helix angle (+80° vs -60°) needs consistent outward orientation and
a handedness convention, which SVF does not have. A global axis is also only
an approximation of each point's own long-axis direction. It is a coarse
sanity check, not an acceptance criterion for SVF's rendering. The RV is
excluded: the LV axis is not a valid RV reference, and a second
coordinate-frame calculation would add an assumption without answering the
questions in §1.

|α| is computed **only** by the diagnostic example (§7.3) and the independent
audit (§7.2). It is not part of the svf-core library API, and the Explorer
does not use it.

### 4.6 Degenerate cases

Each case below is detected explicitly, counted, and excluded from the
quantity it would corrupt. No fabricated angle, normal or glyph is produced
for it. The audit and the SVF diagnostic apply the same rules and report the
same counters. These are plain counters in the operation's result and in the
diagnostic output, not a general validation subsystem.

All tests use unit inputs, so ε = 1e-6 is an absolute threshold on a
dimensionless quantity: f̂ = f/‖f‖, n̂ = n_g/‖n_g‖, L̂ = L/‖L‖.

| Case | Test | Excluded from | Where counted |
|---|---|---|---|
| Zero-area or non-finite triangle | ‖(b − a) × (c − a)‖ = 0, or not finite | Normal, θ, glyph, binning | svf-core result + diagnostic |
| Zero or non-finite source fibre | ‖f‖ = 0, or any component not finite | θ, glyph, binning | svf-core result + diagnostic |
| Zero fibre projection (fibre along the normal) | ‖t‖ ≤ ε, with t = f̂ − (f̂·n̂) n̂ | \|α\| statistics only (θ = 0° is still valid and reported) | Diagnostic only |
| Degenerate circumferential reference (face perpendicular to the LV axis) | ‖L̂ × n̂‖ ≤ ε | \|α\| statistics only | Diagnostic only |

The two helix cases exist only where |α| exists (§4.5), so svf-core never sees
them. In case 01, every one of these counts is **0** (§5). The audit will
confirm this.

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
- Percentiles use the nearest-rank index ⌊(n − 1)·p + 0.5⌋ on the sorted
  values, which rounds halves up (not Python's round-half-to-even).
- The stage-2 independent audit reproduced every number in this section
  exactly (`tools/rodero-fibre-audit/baseline/case01.json`). The stage-4
  declared-type correction (§7.2, §15) moves the unrounded statistics by at
  most 4e-5 (|α| p25 at the LV epicardium). Every count, and every value at
  the precision quoted here, is unchanged.

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
2. **Anchor:** the triangle centroid, computed as
   p_c = ((a + b) + c) / 3.0 for each coordinate in f64, with a, b, c in the
   triangle's stored vertex order. This exact arithmetic is a numerical
   contract (§7.2), because the centroid decides bin assignment. It is a
   display location only. The fibre is constant over the source tetrahedron, so the
   anchor does not claim where the fibre was measured.
3. **Glyph:** a centred axial segment along the **exact source fibre
   direction**:

   p_start = p_c − (ℓ/2) f̂,  p_end = p_c + (ℓ/2) f̂,  with f̂ = f / ‖f‖.

   This keeps the source fibre direction. It does **not** mathematically
   constrain the segment to the triangle plane. The fibre is **not**
   projected onto that plane, because projecting would change the scientific
   direction being shown and would conflict with A2. The glyph is therefore
   only **approximately tangent** to the surface. Its departure from the plane
   is 90° − θ, using the θ of §4.4 (median θ is 88–89° in case 01). ℓ is a uniform display length in
   mm and does not encode magnitude (all fibres are unit length). The segment
   has no arrowhead.
4. **Thinning by deterministic spatial binning:**
   - Use world-space cubic bins of edge h mm, with the grid origin at the
     world origin O = (0, 0, 0) of the source coordinates.
   - A selected triangle's bin index is k = (⌊(p_c,x − O_x)/h⌋,
     ⌊(p_c,y − O_y)/h⌋, ⌊(p_c,z − O_z)/h⌋). Floor is used so that negative
     coordinates bin consistently. The floor is taken in f64 and then
     converted to an integer index.
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
proposed separately. That proposal must be based on evidence: screenshots
and an identified cause.

### Probe consolidation: `crates/svf-core/examples/vtk_inspect.rs`

This replaces the untracked `parse_probe.rs` and `boundary_probe.rs`. It is a
small diagnostic example, not a second Explorer: there is no rendering, no
field mapping and no fibre logic.
- Usage: `cargo run -p svf-core --release --example vtk_inspect -- <file.vtk>`.
- Reports the point and cell counts, then one line per field: name,
  association (point/cell), type (scalar/`Vector3`), and value count.
- Runs the exterior boundary extraction and reports the boundary triangle
  count and the extraction time.
- Exits with a non-zero status, and the parser or extraction error, on
  failure.

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

This is maintained Python (standard library only), kept as reproducibility
evidence beside `tools/analytical-heat-diffusion/`. It never reads SVF output
or SVF's extracted boundary as an input. SVF output appears only as the thing
being checked (`compare.py`, `verify_glyphs.py`).

`audit.py <01.vtk> --tags 1,2 --bin 4 --out <stats.json>`:
1. reads the **original, unmodified** `01.vtk` with its own parser;
2. identifies tetrahedral faces by sorted source point-ID triples;
3. counts face incidences and keeps the faces that occur exactly once;
4. recovers each boundary face's source cell index, `ID` tag and fibre
   vector;
5. computes geometric normals, θ, the ρ wall groups, the §4.6 exclusions, the
   LV axis and |α|, and the occupied spatial bins, exactly as defined in
   §4–§6;
6. writes the statistics set (below) as JSON.

**Statistics set** (one JSON schema, written by both the audit and the SVF
diagnostic):
- total boundary triangle count; eligible count per tag; per-tag count of
  boundary triangles from cells outside the tag set (must be 0 glyphs);
- per wall group (LV/RV × endo/epi/other): count, |cos| p50/p90/p99, LV |α|
  p25/p50/p75;
- over all eligible triangles: the fractions with |cos| < 0.1 and < 0.2;
- every §4.6 exclusion counter;
- LV axis length and the apex/base point counts;
- thinning, at the configured h:
  - the occupied-bin count (= total glyph count);
  - the thinned glyph count per tag, i.e. the tag of each bin's winning
    triangle under the lowest-surface-index rule. The audit determines each
    winner independently (§7.2, expected surface index), so a mixed LV/RV bin
    is not ambiguous and these counts are compared exactly;
  - the count of eligible centroids within 1e-9 mm of a bin face.
- provenance (§7.4): `source_sha256`, `input_sha256`, and the parameters
  used.
- run record (not compared): wall-clock runtime and peak memory.

`compare.py <audit.json> <svf.json>` applies the matching rule and exits
non-zero on any mismatch, printing every differing key. A key that is missing
from either side is a mismatch. The only keys it does not compare are
`input_sha256` and the run record. It prints both of these, and their names
are fixed in the script, so nothing is skipped silently.

**Matching rule:** all counts must match exactly. Fractions are ratios of
exactly matching counts, so they match exactly too. Percentiles, angles and
lengths must match within 1e-9 absolute.

**Numeric input contract (corrected in stage 4, §15).** Both sides must
convert every array by its *declared VTK type* before any calculation:
`float` is parsed as IEEE-754 binary32 (correctly rounded) and then promoted
to f64; `double` is parsed as binary64; integer types are exact. This is
SVF's existing parser contract (`crates/svf-core/src/vtk_legacy.rs`). It is
not enough for both sides to read the same decimal text: parsing `float`
text directly into f64 gives different input values. Every array in
`01.vtk` is declared `float`, and its text is the printed binary32 data.
With the same converted inputs, only the order of floating-point operations
can differ. The audit records the conversion it used as
`parameters.numeric_conversion`, and `compare.py` compares it like any other
parameter, so neither side can silently revert to direct f64 parsing.

The thinning counts (occupied bins and per-tag glyphs) must match exactly
as well. The centroid arithmetic is pinned (below), so bin assignment is
bitwise reproducible. The near-face count is reported and compared like any
other count, but it grants no allowance.

#### Glyph-level verification: `verify_glyphs.py <01.vtk> <glyphs.csv>`

The verifier establishes both fidelity and deterministic selection. It does
this from the original file alone, without trusting which glyphs SVF chose to
export.

**Expected surface index.** svf-core's triangulated-surface order is fixed by
existing contracts. Boundary faces are ordered lexicographically by their
sorted point-ID key, independent of cell order
(`docs/volume-boundary-extraction.md`, `docs/boundary-surface.md`).
Triangles follow face order, one per triangular face
(`docs/surface-triangulation.md`). For an all-tetrahedron mesh, a triangle's
surface index is therefore the rank of its sorted point-ID key among all
boundary faces. The verifier checks that every cell is a tetrahedron
(VTK type 10) and stops if any is not. It then computes this rank itself.

**Expected glyph set.** Using its own boundary, eligibility (tag set and the
§4.6 exclusions), centroids and floor binning (§6.4), the verifier picks for
every occupied bin the eligible triangle with the lowest expected surface
index. This set of (bin, winner) pairs is the expected glyph set. It also
records every eligible centroid that lies within 1e-9 mm of a bin face (the
near-face set).

**CSV contents.** The export has a header line with `source_sha256`,
`input_sha256`, h, ℓ and the glyph count, then one row per glyph. Each row
holds the surface index, the sorted point-ID triple, the source cell index,
the tag, the bin index, the source fibre, θ, p_start and p_end. f64 values
are written in shortest round-trip form.

**Set comparison.** Matching the exported glyphs to the expected set by bin,
the verifier reports and counts each of these:
- **missing:** an expected bin with no exported glyph;
- **unexpected:** an exported glyph in a bin that is not expected;
- **wrong winner:** an exported glyph whose triangle is not the expected
  winner of its bin;
- **duplicate bin:** two or more exported glyphs in one bin;
- **header mismatch:** the header glyph count differs from the number of rows.

Any discrepancy of any of these kinds fails. There is no near-face
allowance. For diagnosis, the verifier lists every discrepancy with its
triangles and bins, and flags any that involve a near-face centroid.

**Centroid contract (accepted in round 5).** svf-core computes
p_c = ((a + b) + c) / 3.0 independently for each coordinate in f64, with
a, b, c in the triangle's stored vertex order. For a tetrahedral boundary
face, that order is the source cell's Tetrahedron4 template cycle
(`docs/volume-boundary-extraction.md`, passed through unchanged by
`docs/surface-triangulation.md`). The audit reproduces this contract
independently from the original file. With it pinned, case 01 must match the
audit's bin assignments and winners exactly. Synthetic tests on both sides
protect it: the audit's `test_audit.py`, and svf-core's in stage 3. Each
uses a triangle whose bin changes if the order or the formula changes.

**Case 01 has 9 near-face centroids** (8 in stage 2, which parsed the text
directly into f64; see §15). These are recorded and reported, not excused.
Each lies *exactly* on a bin face, because the input coordinates of those
triangles sum to an exact multiple of 3h. Whether f64 then lands on or just
below the face depends on the centroid formula and the summation order.
None of the 9 is its bin's winner. Two of them (ranks 11530 and 94448) would
win the neighbouring bin if rounded below the face. The bin count would be
unchanged, but the winners of those two bins would not. The ninth (rank
61792) appears with binary32 input: its x values sum to 312.000001 in
decimal but to exactly 312.0 as binary32, so its centroid lies on the face
x = 104. It stays in its bin either way, because floor binning puts a point on
a face in the upper bin.

This is why the centroid contract above is pinned and not left to
implementation choice.

**Per-row checks**, for every exported glyph:
- the triple is an audit boundary face of that source cell, and its surface
  index equals the expected rank;
- the tag is in the tag set;
- the fibre equals the original value bit for bit;
- the segment midpoint is the centroid within 1e-9 mm;
- (p_end − p_start) equals ℓ f̂ within 1e-12 relative;
- θ matches the verifier's own value within 1e-9.

Together these verify A2 and the thinning rule independently, which aggregate
statistics cannot do. The checks also confirm that `01.vtk` and
`01-svf-order.vtk` have identical cell indices and point IDs, instead of
assuming it.

**Baseline:** `tools/rodero-fibre-audit/baseline/case01.json` is the committed
output of `audit.py` on `01.vtk`. The dataset itself is not committed
(91 MB). The README documents:
- the download (Zenodo 4590294, `01.tar.gz`);
- the expected `source_sha256`;
- the exact commands and the Python version used;
- the measured runtime and peak memory of each script.

Peak memory is the OS process peak, still through the standard library:
`ctypes` and `K32GetProcessMemoryInfo` on Windows, `resource.getrusage` on
POSIX. This replaces the earlier `tracemalloc` plan, which counted only
Python allocations. Case 01 took 45–49 s with an 884 MB peak in stage 2, and
64 s with an 884.6 MB peak after the correctly rounded binary32 conversion of
stage 4. Performance
work, such as NumPy, is
considered only if these measurements show a real problem. If the audit
disagrees with the scratch numbers in §5, the audit is the reference and §5
is corrected.

### 7.3 SVF diagnostic path

`crates/svf-core/examples/fibre_diagnostic.rs` (separate from `vtk_inspect`)
runs the actual svf-core implementation on `01-svf-order.vtk`. It computes the
diagnostic-only quantities (ρ groups, LV axis, |α|) and writes the same JSON
schema, plus `glyphs.csv`:

```
cargo run -p svf-core --release --example fibre_diagnostic -- \
  <01-svf-order.vtk> --source <01.vtk> --tags 1,2 --bin 4 --length 3 \
  --out svf-case01.json --glyphs glyphs.csv
python -I tools/rodero-fibre-audit/compare.py \
  tools/rodero-fibre-audit/baseline/case01.json svf-case01.json
python -I tools/rodero-fibre-audit/verify_glyphs.py <01.vtk> glyphs.csv
```

### 7.4 Input provenance

The audit reads the original `01.vtk`. The SVF diagnostic reads the reordered
working copy `01-svf-order.vtk`. The two files differ byte for byte, so a
single "input hash" would always mismatch. Two hashes are therefore kept:

| Key | Meaning | Audit | SVF diagnostic | Rule |
|---|---|---|---|---|
| `source_sha256` | SHA-256 of the original source dataset `01.vtk` | Hash of the file it reads | Hash of the file passed with `--source`, which it hashes but does not parse **Must be equal**; see the automated checks below |
| `input_sha256` | SHA-256 of the file actually parsed | Equal to `source_sha256` | Hash of `01-svf-order.vtk` | Not compared. Printed by `compare.py` and kept as diagnostic provenance |

The automated checks, each of which fails the run on a mismatch, are:
1. `compare.py` checks that the baseline's `source_sha256` equals the SVF
   diagnostic's `source_sha256`.
2. The committed baseline `baseline/case01.json` pins the expected
   original-dataset hash. Check 1 therefore ties every SVF run to that pinned
   hash.
3. `verify_glyphs.py` hashes the original `01.vtk` it reads and checks it
   against the `source_sha256` in the CSV header.

Separately, the README documents the same expected hash for people
downloading the dataset. That is a documentation-consistency requirement,
kept in step by hand when the baseline is regenerated. No script reads the
README.

`--source` is required. The diagnostic refuses to run without it, so no output
can lack source provenance. The hashes alone do not prove that the working
copy is a faithful reordering of the source. That is established by the
results: matching statistics in `compare.py`, plus `verify_glyphs.py`
confirming every exported glyph's cell index, point IDs and fibre against the
original file.

## 8. Acceptance criteria

- **A1 Selection:** on case 01, SVF's eligible counts per tag equal the audit
  baseline exactly (the scratch measurement gives 86,241 LV + 85,232 RV =
  171,473). Zero glyphs come from tags 3–24 (`verify_glyphs.py`).
- **A2 Fidelity:** every glyph lies along its source cell's exact fibre
  direction in f64 (up to sign, since the glyph is axial). Nothing is
  projected or smoothed, and provenance resolves to the right triangle and
  cell. Thinning selects exactly the expected winner in every occupied bin,
  with no missing, unexpected or duplicate glyphs. Both are checked
  independently by `verify_glyphs.py` (§7.2), and source provenance agrees
  (§7.4).
- **A3 Dataset agreement:** the §7.1 synthetic tests pass, and `compare.py`
  reports no mismatch between the SVF diagnostic and the audit baseline across
  the full statistics set (§7.2).
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

## 11. Review decisions (round 2: direction approved, revisions required)

| Topic | Decision |
|---|---|
| Fibre direction | Exact f̂, centred on p_c; not constrained to the plane; "approximately tangent" (§6.3) |
| Core tests | Synthetic and general only; no Rodero numbers in svf-core (§7.1) |
| Dataset comparison | Shared JSON statistics set, `compare.py` matching rule, glyph-level `verify_glyphs.py` (§7.2–7.3) |
| Degenerate geometry | Four explicit counters; helix cases are diagnostic-only; no new subsystem (§4.6) |
| Thinning | World-origin cubic bins, floor index, lowest-surface-index tie-break (not a ranking) (§6.4) |
| Audit | `tools/rodero-fibre-audit/`, own boundary from sorted point IDs, committed baseline, documented run (§7.2) |
| Depth | Existing line path first; no normal offsets; changes only on evidence (§6) |
| `vtk_inspect` | Fields with association/type/count, plus a boundary check; small (§6) |
| h, ℓ | 4 mm and 3 mm, provisional and configurable; tuned after validation |
| RV | Tangency only; no RV frame and no RV helix diagnostic |

## 12. Review decisions (round 3)

| Topic | Decision |
|---|---|
| Glyph verification | Kept, and strengthened. It computes the expected winner per bin from its own boundary and the surface-index rank, then reports missing, unexpected, wrong-winner and duplicate-bin glyphs (§7.2) |
| Near-face tolerance | 1e-9 mm, with the count reported. Only individually attributed discrepancies are allowed; it is not a general allowance (§7.2). Superseded in round 5: the count is reported only |
| Audit implementation | Standard-library Python. Runtime and peak memory are recorded; no NumPy unless the measurements show a need (§7.2) |
| Input provenance | `source_sha256` must match; `input_sha256` is kept but not compared; the diagnostic requires `--source` (§7.4) |

## 13. Review decisions (round 4: approved pending these edits)

| Topic | Decision |
|---|---|
| Per-tag glyph counts | Added to the statistics and compared exactly. The near-face exception still applies only to individually attributed discrepancies (§7.2). Superseded in round 5: there is no exception |
| Provenance wording | Automated checks (compare.py, the pinned baseline, verify_glyphs.py) are kept separate from the README's documentation consistency (§7.4) |
| Implementation order | vtk_inspect; audit plus baseline; svf-core operation plus synthetic tests; fibre_diagnostic, compare and verify_glyphs; renderer/Explorer plus visual validation. Each stage is reviewed separately |

## 14. Review decisions (round 5: stage 2 accepted)

| Topic | Decision |
|---|---|
| Centroid | Pinned: ((a + b) + c) / 3.0 for each coordinate in f64, in stored vertex order. This is part of SVF's geometry contract, and the audit reproduces it independently (§6, §7.2) |
| Near-face centroids | Case 01 has 8. They are reported but grant no allowance. Thinning counts, bin assignments and winners must match the audit exactly (§7.2) |
| Protection | A synthetic test whose bin flips if the order or the formula changes, in `test_audit.py` now and in svf-core in stage 3 |
| Audit | Unchanged as the reference |

## 15. Review decisions (stage 4: numeric input contract)

| Topic | Decision |
|---|---|
| Finding | SVF parses `float` arrays as binary32 (VTK semantics); the stage-2 audit parsed the same text as f64. All counts agreed, but 25 floating-point statistics differed by up to 4e-5, and the near-face count was 8 against 9. A scratch audit with binary32 input matched SVF on every key |
| Decision | Option A: the audit converts by declared type (binary32 for `float`, binary64 for `double`, exact integers), and the baseline is regenerated and becomes authoritative. SVF's parser is not changed, and tolerances are not loosened |
| Safeguards | Correctly rounded decimal-to-binary32 conversion, including the case where the f64 lands on a binary32 midpoint; regression tests for `float` and `double`; the conversion recorded in `parameters.numeric_conversion`; `boundary_axial_glyphs.rs` unchanged |
| Evidence | `tools/rodero-fibre-audit/baseline/case01-numeric-conversion.md`: old-versus-new values, with identical eligible sets, bin assignments and winners for all 171,473 eligible triangles |
