# Rodero fibre audit (B1.2)

An independent reference for the B1.2 ventricular fibre experiment
(`docs/heart-fibre-orientation.md`, §7.2). It is standard-library Python and
does not use SVF in any way. It reads the **original, unmodified** `01.vtk`
with its own parser. It derives the exterior boundary itself: every
tetrahedral face is keyed by its sorted point-ID triple, and faces with
exactly one incidence are boundary faces. It then writes the statistics set
that later stages compare SVF against.

| File | Purpose |
|---|---|
| `audit.py` | Computes the statistics set and the thinning baseline as JSON |
| `test_audit.py` | Synthetic self-tests of the audit's own logic |
| `compare.py` | Compares an SVF diagnostic statistics file with the baseline (§7.2 matching rule) |
| `verify_glyphs.py` | Recomputes every bin's expected winner from `01.vtk` and checks SVF's glyph export row by row |
| `test_tools.py` | Synthetic self-tests of `compare.py` and `verify_glyphs.py` |
| `baseline/case01.json` | Committed output for case 01 (the reference) |
| `baseline/case01-numeric-conversion.md` | Why the baseline was regenerated in stage 4, and every value that changed |

SVF's side is `crates/svf-core/examples/fibre_diagnostic.rs` (design §7.3).

## Dataset

Rodero et al. (2021), Zenodo record https://zenodo.org/records/4590294
(CC BY 4.0), archive `01.tar.gz`, file `01.vtk`. The dataset is not committed.

Expected SHA-256 of `01.vtk`:

```
20bf29b8ec86454de4cfde2a0afc19992715107b6ef8a7dabe61f52e8f78f440
```

This value is documentation for people downloading the file. The automated
pin is `provenance.source_sha256` in `baseline/case01.json`, and no script
reads this README (§7.4). Update both together if the baseline is ever
regenerated from a different file.

## Running

```
python -I tools/rodero-fibre-audit/test_audit.py
python -I tools/rodero-fibre-audit/audit.py <path>/01.vtk \
    --tags 1,2 --lv-tag 1 --bin 4 --out tools/rodero-fibre-audit/baseline/case01.json
```

To check that a fresh run reproduces the committed baseline, write to a
scratch path instead and compare every key except `run`. Two runs on
2026-11-11 were identical apart from `run`.

## Definitions the audit fixes

These follow the design. Where the design text was ambiguous, the audit's
choice is stated here and in the design:
- **Numeric input (corrected in stage 4):** every array is converted by its
  declared VTK type before any calculation, as SVF's parser does. `float`
  becomes the correctly rounded IEEE-754 binary32 value, promoted to f64;
  `double` becomes binary64; integer types are exact and must lie in the
  declared range (i16, u16, i32, u32, u8), as in SVF. Text that SVF's
  parser rejects (underscores, non-ASCII digits, a `-` on an unsigned type)
  is an error here too; `test_audit.py` and `fibre_diagnostic.rs` share one
  table of these cases. Every array in `01.vtk` is `float`. The baseline records this as `parameters.numeric_conversion`.
  The stage-2 audit parsed `float` text directly into f64, which is not the
  same input; see `baseline/case01-numeric-conversion.md`.
- **Tag selection:** by the cell `ID` value, never by fibre value.
- **LV points for the axis:** `V.dat == -1` exactly. The source writes -10 for
  every UVC outside the ventricles, so a bare `V < 0` test would admit 138,056
  sentinel points as "apex" points (Z = -10 < 0.02).
- **Wall group:** the mean `RHO.dat` of the triangle's three vertices; < 0.2
  is endocardial, > 0.8 epicardial, otherwise other. The audit also reports
  how many eligible triangles have a sentinel ρ vertex (case 01: 0).
- **Percentiles:** nearest rank, index `floor((n - 1) p + 0.5)` on the sorted
  values, which rounds halves up. This is not Python's `round` (which rounds
  halves to even). The SVF diagnostic must use the same expression.
- **Centroid (pinned contract, design §7.2):** `((a + b) + c) / 3.0` for each
  coordinate in f64, with a, b, c in the boundary face's stored cycle order
  (the source cell's Tetrahedron4 template cycle, from
  `docs/volume-boundary-extraction.md`). `test_audit.py` protects it with a
  triangle whose bin changes under reversed order or `a/3 + b/3 + c/3`.
- **Thinning:** floor bins of edge h from the world origin. The winner of each
  bin is the eligible triangle with the lowest sorted-key rank, which is the
  expected triangulated-surface index for an all-tetrahedron mesh. The audit
  refuses any non-tetrahedral cell.
- **Near-face centroids:** eligible centroids within 1e-9 mm of a bin face.

`audit_checks` holds sanity results: the face-incidence histogram and the
sentinel-ρ count. The SVF diagnostic reproduces both from its own topology,
so `compare.py` compares them like every other key.

## Case 01 results (baseline)

- 329,952 boundary triangles. Incidence histogram {1: 329,952,
  2: 4,790,756}: there are no non-manifold faces.
- 171,473 eligible ventricular triangles (LV 86,241, RV 85,232), with 0
  exclusions of any kind.
- All the scratch-measurement numbers in design §5 are reproduced at the
  precision quoted there (exactly, in stage 2's f64 reading).
- Thinning at h = 4 mm: 5,183 occupied bins; 2,836 LV and 2,347 RV glyphs.
- **9 near-face centroids, not 0** (8 in stage 2's f64 reading; the ninth,
  rank 61792, is explained in `baseline/case01-numeric-conversion.md`). Each
  lies *exactly* on a bin face (for example x = 132.0), because the
  triangle's coordinates sum to an exact multiple of 3h (e.g. 132.139999 +
  131.949997 + 131.910004 = 396.000000). None of them is its bin's winner. Two of them (ranks 11530
  and 94448) would win the neighbouring lower bin if their centroid were
  rounded below the face. The bin count would not change, but those two bins'
  winners would. This is why the centroid arithmetic is pinned (see
  Definitions). The 9 are reported, but they grant no allowance: SVF must
  match the audit's bin assignments and winners exactly.

## Checking SVF against the baseline (stage 4)

```
python -I tools/rodero-fibre-audit/test_tools.py
cargo run -p svf-core --release --example fibre_diagnostic -- <path>/01-svf-order.vtk     --source <path>/01.vtk --tags 1,2 --lv-tag 1 --bin 4 --length 3     --out svf-case01.json --glyphs glyphs.csv
python -I tools/rodero-fibre-audit/compare.py tools/rodero-fibre-audit/baseline/case01.json svf-case01.json
python -I tools/rodero-fibre-audit/verify_glyphs.py <path>/01.vtk glyphs.csv
```

`compare.py` compares every key except `provenance.input_sha256` and `run`,
which it prints. Counts, strings, `parameters` and the |cos| fractions must
be equal; other floats must agree within 1e-9. A missing key, or a type
difference, is a mismatch. `verify_glyphs.py` needs only the original file and
the export. It checks `source_sha256`, then reports missing, unexpected,
wrong-winner and duplicate-bin glyphs, a header count mismatch, and per-row
problems: face and rank, cell, tag, bin, fibre bit for bit, midpoint within
1e-9 mm, end − start = ℓ f̂ within 1e-12 relative, and θ within 1e-9°. Near-face
centroids are listed but grant no allowance. Each script exits 1 on any
discrepancy.

Case 01, 2026-11-11 (Windows 11, Python 3.14.3, release build): the
diagnostic took 34 s; `compare.py` reported MATCH; and `verify_glyphs.py`
reported PASS (5,183 expected bins, 5,183 rows, 0 discrepancies of every
kind) in 80 s with an 887 MB peak working set.

## Runtime and peak memory

Measured on Windows 11 with Python 3.14.3, for `audit.py` on case 01: 64.3 s
wall clock, with a peak working set of 884.6 MB (stage 4, binary32 input).
Stage 2's direct f64 parsing took 45–49 s at 884 MB; correctly rounded
binary32 conversion of about 12 million values accounts for the difference.

Peak memory is the OS process peak, read with `ctypes` from
`K32GetProcessMemoryInfo` (`PeakWorkingSetSize`). On POSIX, the audit uses
`resource.getrusage`. Both are standard library. This was chosen over
`tracemalloc`, which counts only Python allocations and slows the run. The
values are stored in `run`, which is never compared. Neither measurement
indicates a need for NumPy.
