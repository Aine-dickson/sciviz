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
| `baseline/case01.json` | Committed output for case 01 (the reference) |

`compare.py` and `verify_glyphs.py` are added in stage 4.

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

`audit_checks` holds audit-only sanity results (the face-incidence histogram
and the sentinel-ρ count). SVF does not reproduce them.

## Case 01 results (baseline)

- 329,952 boundary triangles. Incidence histogram {1: 329,952,
  2: 4,790,756}: there are no non-manifold faces.
- 171,473 eligible ventricular triangles (LV 86,241, RV 85,232), with 0
  exclusions of any kind.
- All the scratch-measurement numbers in design §5 are reproduced exactly.
- Thinning at h = 4 mm: 5,183 occupied bins; 2,836 LV and 2,347 RV glyphs.
- **8 near-face centroids, not 0.** Each lies *exactly* on a bin face (for
  example x = 132.0). The input has 6 decimals, and these triples sum to an
  exact multiple of 3h (e.g. 132.139999 + 131.949997 + 131.910004 =
  396.000000). None of them is its bin's winner. Two of them (ranks 11530
  and 94448) would win the neighbouring lower bin if their centroid were
  rounded below the face. The bin count would not change, but those two bins'
  winners would. This is why the centroid arithmetic is pinned (see
  Definitions). The 8 are reported, but they grant no allowance: SVF must
  match the audit's bin assignments and winners exactly.

## Runtime and peak memory

Measured on Windows 11 with Python 3.14.3, for `audit.py` on case 01: 45–49 s
wall clock, with a peak working set of 884 MB.

Peak memory is the OS process peak, read with `ctypes` from
`K32GetProcessMemoryInfo` (`PeakWorkingSetSize`). On POSIX, the audit uses
`resource.getrusage`. Both are standard library. This was chosen over
`tracemalloc`, which counts only Python allocations and slows the run. The
values are stored in `run`, which is never compared. Neither measurement
indicates a need for NumPy.
