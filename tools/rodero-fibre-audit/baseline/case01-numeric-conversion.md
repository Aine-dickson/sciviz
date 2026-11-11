# Case 01 baseline: declared-type numeric conversion (stage 4)

This records why `case01.json` was regenerated in stage 4 and exactly what
changed. The design decision is `docs/heart-fibre-orientation.md` §15, and
the corrected contract is in §7.2.

## Why

Every array in `01.vtk` is declared `float`, for example `POINTS 493151 float`
and `VECTORS fibres float`. SVF's parser reads a `float` value as IEEE-754
binary32 (correctly rounded) and promotes it to f64. That is the VTK meaning
of `float`, and an existing svf-core test enforces it. The stage-2 audit
parsed the same decimal text directly into f64, so the two sides started
from different input values.

The text is printed binary32 data. In a sample of 60,000 coordinates, every
one prints back to its original 6-decimal text after rounding to binary32.
Near 100 mm, adjacent binary32 values are about 7.6e-6 apart, so arbitrary
6-decimal text would rarely survive that round trip. Reading the text as
binary32 therefore recovers the stored values. Reading it as f64 gives a
decimal approximation of them.

Before the correction, comparing the SVF diagnostic with the stage-2 baseline
matched every count but differed in 25 floating-point statistics and in the
near-face count. A scratch audit that rounded `float` input to binary32
matched the SVF diagnostic on every compared key. The input conversion was
therefore the only cause.

## What changed in `audit.py`

- `float` arrays: correctly rounded decimal-to-binary32 conversion
  (`binary32()`), then promotion to f64. The result goes through f64 first
  and is resolved exactly, with `fractions.Fraction`, in the one case where
  that can be wrong: when the f64 lands exactly on a binary32 midpoint.
- `double` arrays: decimal-to-binary64, as before.
- Integer types (such as `ID`): exact integers within the declared range
  (i16, u16, i32, u32, u8), as SVF parses them (review of PR #53). Text that
  SVF's parser rejects is an error. An unknown declared type is an error.
  Case 01 is unaffected: a fresh audit run still matches this baseline.
- The baseline records the conversion as `parameters.numeric_conversion`:
  `vtk-declared-type/1: float=binary32 promoted to f64; double=binary64; integer types exact`.
  `compare.py` compares it like any other parameter, and `test_audit.py`
  checks that the committed baseline carries it.

## Old versus new values

Old: stage-2 baseline (commit 5a0ef74). New: the regenerated `case01.json`.
`run` is excluded. 77 values are unchanged, and the 27 changed ones are listed below.

| Key | Old (direct f64) | New (binary32) | New − old |
|---|---|---|---|
| `parameters.numeric_conversion` | (absent) | `vtk-declared-type/1: …` | |
| `groups.1.endo.abs_cos_p50` | 0.03497678925013399 | 0.03497725836512887 | +4.691e-07 |
| `groups.1.endo.abs_cos_p90` | 0.10557991497066152 | 0.10557967486338676 | −2.401e-07 |
| `groups.1.endo.abs_cos_p99` | 0.28286217317939 | 0.28286257462581155 | +4.014e-07 |
| `groups.1.endo.alpha_deg_p25` | 68.30728812035083 | 68.307287531503 | −5.888e-07 |
| `groups.1.endo.alpha_deg_p50` | 75.75185845806715 | 75.75187195640542 | +1.350e-05 |
| `groups.1.endo.alpha_deg_p75` | 82.92043224657384 | 82.92042989847012 | −2.348e-06 |
| `groups.1.epi.abs_cos_p50` | 0.0169957618892784 | 0.016996669779228646 | +9.079e-07 |
| `groups.1.epi.abs_cos_p90` | 0.06231078185159622 | 0.06231062199399662 | −1.599e-07 |
| `groups.1.epi.abs_cos_p99` | 0.3566700321036724 | 0.3566697882831257 | −2.438e-07 |
| `groups.1.epi.alpha_deg_p25` | 54.08673153809052 | 54.08669193187403 | −3.961e-05 |
| `groups.1.epi.alpha_deg_p50` | 62.51768174519754 | 62.517690581849685 | +8.837e-06 |
| `groups.1.epi.alpha_deg_p75` | 72.70225461753303 | 72.7022491058486 | −5.512e-06 |
| `groups.2.endo.abs_cos_p50` | 0.021255088506141384 | 0.02125518900738997 | +1.005e-07 |
| `groups.2.endo.abs_cos_p90` | 0.06949084980889148 | 0.0694914598174502 | +6.100e-07 |
| `groups.2.endo.abs_cos_p99` | 0.3066268655153014 | 0.3066272906343705 | +4.251e-07 |
| `groups.2.epi.abs_cos_p50` | 0.022545294993684017 | 0.022545038343110613 | −2.567e-07 |
| `groups.2.epi.abs_cos_p90` | 0.05782430973181882 | 0.057824912358007324 | +6.026e-07 |
| `groups.2.epi.abs_cos_p99` | 0.2988867212414074 | 0.2988866141377179 | −1.071e-07 |
| `groups.2.other.abs_cos_p50` | 0.1379271199884837 | 0.1379280460778315 | +9.261e-07 |
| `groups.2.other.abs_cos_p90` | 0.5501484466481944 | 0.5501485470530235 | +1.004e-07 |
| `groups.2.other.abs_cos_p99` | 0.9091985632271642 | 0.9091988613559288 | +2.981e-07 |
| `lv_axis.vector_mm.0` | −38.78325849553791 | −38.783258536611214 | −4.107e-08 |
| `lv_axis.vector_mm.1` | 57.028950566243104 | 57.028950563277355 | −2.966e-09 |
| `lv_axis.vector_mm.2` | 55.53346511379647 | 55.53346507468525 | −3.911e-08 |
| `lv_axis.length_mm` | 88.54607890677012 | 88.54607889832077 | −8.449e-09 |
| `thinning.near_face_centroids` | 8 | 9 | +1 |

Unchanged values:
- every other count: boundary triangles (329,952), boundary, selected and
  eligible counts per tag, all exclusions (0), every group count, the
  |cos| < 0.1 and < 0.2 counts and fractions, the apex and base point
  counts, 5,183 occupied bins, and 2,836 LV and 2,347 RV glyphs;
- the face-incidence histogram and the sentinel-ρ count;
- `source_sha256` and `input_sha256`.

The largest change is 3.96e-5 (`groups.1.epi.alpha_deg_p25`). At the
precision quoted in design §5 (|cos| to 4 decimals, |α| to 0.1°, axis
88.5 mm), every number is unchanged.

## Bins and winners, per triangle

Both audits were run on `01.vtk` and compared triangle by triangle, not only
by count:

- The eligible set is identical: the same 171,473 triangles (by rank).
- Bin assignments are identical: no eligible triangle changed bin.
- The occupied bins are identical: the same 5,183 bin indices.
- The winners are identical: no bin's winning triangle changed.

The near-face set gains one centroid, rank 61792 (point IDs 67166, 343781,
67167, in stored cycle order). Its x coordinates are 103.511002, 104.261002
and 104.227997 in decimal, which sum to 312.000001: a centroid 3.3e-7 mm above
the bin face x = 104. As binary32 values they sum to exactly 312.0, so the
centroid lies exactly on the face. It is in bin (26, 9, 17) under both
readings (floor binning puts a point on a face in the upper bin), and it
is not that bin's winner. The other 8 near-face centroids (ranks 11530,
31106, 51510, 94448, 175010, 190573, 240591, 309425) are unchanged.

## Run

`python -I tools/rodero-fibre-audit/audit.py 01.vtk --tags 1,2 --lv-tag 1 --bin 4 --out baseline/case01.json`,
with Python 3.14.3 on Windows 11: 64.3 s and a peak working set of 884.6 MB.
Stage 2 took 45–49 s; the correctly rounded binary32 conversion of about
12 million values accounts for the difference.
