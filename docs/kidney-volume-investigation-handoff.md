# Kidney volume investigation — handoff

Last updated: 2026-11-18
Working branch: `b2-5e-heart-tool`

## Purpose

Continue the SVF kidney investigation from a completed first-case source,
geometry and voxel audit. The immediate objective is to determine the smallest
reusable volume and segmentation capabilities SVF needs for educational
scientific visualization. This is an investigation, not authorization to
implement a DICOM reader or kidney-specific API.

## Durable project records

- Full case audit: [`docs/kidney-data-audit.md`](kidney-data-audit.md)
- Milestone record: [`docs/MILESTONES.md`](MILESTONES.md)
- Architecture and current dataset model: [`docs/ARCHITECTURE.md`](ARCHITECTURE.md)
- Educational product direction: [`docs/EDUCATIONAL-DIRECTION.md`](EDUCATIONAL-DIRECTION.md)

The audit report contains the case identifiers, exact local paths, CT/SEG
geometry, pixel findings, segment statistics, connectivity, adjacency,
distance-transform results, interpretations, limitations and licence cautions.
Read it before repeating any dataset inspection.

## Dataset currently on the user's machine

Repository root: `~/sci_viz`

Case root (note the directory is named `downloaded`, not `downloads`):

`datasets/kidney_kits19/downloaded/c4kc_kits/KiTS-00005`

- Study: `2003-11-27-three_phase__abdomen-21232`
- Arterial CT: `6-arterial-78517` — 834 DICOM files
- DICOM SEG: `300-Segmentation-25690` — one multi-frame DICOM file
- Total: 835 files, approximately 474 MB
- Only arterial CT and matching SEG were downloaded.
- Do not ask the user to upload the dataset and do not add it to Git.

## Key validated findings

- CT: 512 × 512 × 834; pixel spacing 0.9765625 × 0.9765625 mm;
  measured slice-centre spacing 0.5 mm.
- CT SliceThickness header says 1 mm; do not confuse it with the measured
  0.5 mm slice-centre spacing.
- CT rescale slope/intercept: 1 / -1024. Sampled middle-slice HU range:
  -1024 to 1933. Its percentiles are slice-only, not whole-volume.
- SEG: BINARY, 1 bit, 1,668 frames: 834 Kidney + 834 Mass.
- All frames reference the correct arterial CT series. No missing CT instance
  references, no position mismatches, no orientation mismatches. A separate
  coordinate comparison found a maximum difference of about 0.00001875 mm.
- Kidney: 902,167 foreground voxels, 283 non-empty frames, one dominant
  26-connected component of 902,158 voxels plus tiny components of 5, 3 and 1.
- Mass: 126,444 foreground voxels, 119 non-empty frames, one
  26-connected component.
- Kidney/Mass overlap is zero. Mass is a separate binary mask, not an
  overlapping tumour label in this case.
- 3,652 Mass voxels have 6-neighbour Kidney contact; 6,203 have
  26-neighbour contact.
- An anisotropic voxel-centre distance transform was computed. Median distance
  is 11.3732 mm; maximum 33.4526 mm. This is not exact mesh-surface distance.
- Preserve the two source segments independently. Do not merge masks, fill
  intervening space, or infer that the tumour is anatomically outside the
  kidney based only on mask distance.

## Existing SVF capability boundary

Current core has datasets, point/cell fields, unstructured grids, ASCII legacy
VTK UNSTRUCTURED_GRID import, tetrahedral point-scalar isosurfaces, and
semantic surface structures with sectioning/peeling.

It does not yet have an authoritative 3D image-volume type, DICOM/NIfTI reader,
segmentation volume/label semantics, general structured-volume surface
extraction, volume rendering, or source-CT slice-stack display. Consult the
full audit and current source before proposing architecture.

## Local diagnostic scripts

The user created and ran these scripts locally from the repository root:

- `inspect_kidney.py`
- `inspect_seg_frames.py`
- `compare_kidney_geometry.py`
- `inspect_kidney_voxels.py`
- `inspect_kidney_alignment.py`
- `inspect_kidney_mask_relationship.py`
- `inspect_kidney_components.py`
- `inspect_kidney_mask_adjacency.py`
- `inspect_kidney_mass_distance.py`

Their measured outputs are preserved in `docs/kidney-data-audit.md`, but the
scripts themselves have not yet been committed. Do not claim that they are
versioned or assume their exact source contents are available in a future
session. Next, ask the user to preserve/copy the local scripts into
`tools/dataset-audits/kits-00005/`, or create a reviewed consolidated
reproducibility script and compare its output against the recorded results.

Python environment used locally included pydicom 3.0.2, NumPy and SciPy.

## Immediate next steps

1. Preserve the local diagnostic scripts in
   `tools/dataset-audits/kits-00005/` and add a README with dependencies,
   invocation, expected dataset layout and privacy/licence cautions.
2. Inspect current `crates/svf-core/src` APIs and tests on the user's checked
   out branch. Do not infer source details from an old file listing.
3. Map current geometry/dataset/field types against a volume model carrying
   dimensions, physical origin, spacing, direction, scalar values and metadata.
4. Decide how independent binary segmentation masks and per-segment metadata
   should be represented without assuming mutual exclusivity.
5. Write and review a design contract before implementation.
6. Use synthetic volumes with known geometry and ground truth for initial
   automated tests; use KiTS-00005 as a real-world integration/validation case
   only after licensing and attribution are checked for the exact distribution.

## Workflow constraints

- Work incrementally: give one focused command/action, wait for exact output,
  then choose the next step.
- User works on Windows via Git Bash.
- Do not run `cargo fmt` unless explicitly requested.
- Never claim tests passed unless the user ran them and reported the output.
- Do not spend time on kidney-specific surface polishing before volume geometry,
  segmentation semantics and source correspondence are designed.
- Preserve scientific provenance and clearly separate measured facts from
  anatomical interpretation.
