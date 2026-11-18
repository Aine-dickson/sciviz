# KiTS-00005 DICOM Dataset Audit

Reproducible inspection scripts for the KiTS-00005 arterial-phase CT series
and its corresponding DICOM segmentation (SEG).

These scripts document and investigate the source data used during the
SVF kidney-volume feasibility investigation. They are diagnostic tools,
not a production DICOM ingestion library or a clinical validation system.

## Dataset provenance

- Collection: TCIA C4KC-KiTS
- Collection information:
  https://wiki.cancerimagingarchive.net/pages/viewpage.action?pageId=133072129
- Case: KiTS-00005
- Imaging: arterial-phase CT and matching DICOM SEG
- SEG segments: Kidney and Mass

Check the collection's current attribution and licensing requirements before
redistributing the dataset or derivative assets.

## Expected local dataset layout

The scripts expect the dataset at this path, relative to the repository root:

    datasets/kidney_kits19/downloaded/c4kc_kits/KiTS-00005/

The case contains the study directory, including:

- `6-arterial-78517/` — arterial-phase CT DICOM series
- `300-Segmentation-25690/` — multi-frame DICOM SEG

The scripts use this relative dataset root. Run them from the repository
root unless you have deliberately modified their paths.

## Dependencies

The audit environment used:

- Python
- pydicom 3.0.2
- NumPy
- SciPy

Install the required packages in your Python environment:

    python -m pip install pydicom==3.0.2 numpy scipy

SciPy is required by `inspect_kidney_mass_distance.py`.

## Running the audits

Run individual scripts from the repository root:

    python tools/dataset-audits/kits-00005/inspect_kidney.py
    python tools/dataset-audits/kits-00005/inspect_seg_frames.py
    python tools/dataset-audits/kits-00005/compare_kidney_geometry.py
    python tools/dataset-audits/kits-00005/inspect_kidney_voxels.py
    python tools/dataset-audits/kits-00005/inspect_kidney_alignment.py
    python tools/dataset-audits/kits-00005/inspect_kidney_mask_relationship.py
    python tools/dataset-audits/kits-00005/inspect_kidney_components.py
    python tools/dataset-audits/kits-00005/inspect_kidney_mask_adjacency.py
    python tools/dataset-audits/kits-00005/inspect_kidney_mass_distance.py

Export the audited SEG masks into SVF-compatible local validation inputs:

    python tools/dataset-audits/kits-00005/export_svf_masks.py

The exporter writes `segment-1.raw`, `segment-2.raw`, and `metadata.json`
under `datasets/validation-exports/kits-00005/` by default. Each raw file is
one byte per voxel (0 or 1), stored in NumPy `(k, j, i)` C-order, which is the
same i-fastest linear order used by `ImageVolumeGeometry3`. The metadata
records the affine origin and basis vectors. Use `--dataset-root` and
`--output-dir` to override the defaults.

After exporting, run the opt-in Rust integration test from the workspace root
(Git Bash). Relative `SVF_KITS_MASK_DIR` paths are resolved from the workspace
root even though Cargo runs the integration test from the package directory:

    SVF_KITS_MASK_DIR=datasets/validation-exports/kits-00005 cargo test -p svf-core --test kits_real_masks -- --ignored --nocapture

The test loads both raw masks into `BinaryMask3` and `SegmentationVolume3`,
then checks foreground counts and inclusive voxel-index bounds against the
audited dataset. It is ignored during ordinary test runs because it requires
the external local dataset export.

Surface extraction is a separate opt-in test. Start with the smaller Mass
segment before attempting the much larger Kidney segment:

    SVF_KITS_MASK_DIR=datasets/validation-exports/kits-00005 cargo test -p svf-core --test kits_real_masks extracts_real_kits_mass_surface_and_checks_topology -- --ignored --nocapture

This reports mesh size and timings, checks edge incidence and consistent
orientation, and compares signed mesh volume with the mask-derived physical
volume. The topology checks are diagnostics for this extracted mesh; they are
not a general certification of anatomical or clinical validity.

## Exporting surfaces for independent inspection

Generate legacy VTK PolyData files for both real segments:

    SVF_KITS_MASK_DIR=datasets/validation-exports/kits-00005 cargo test -p svf-core --test kits_real_masks exports_real_kits_surfaces_for_external_inspection -- --ignored --nocapture

The test writes these local files:

- `datasets/validation-exports/kits-00005/surface-inspection/kidney.vtk`
- `datasets/validation-exports/kits-00005/surface-inspection/mass.vtk`
- `datasets/validation-exports/kits-00005/surface-inspection/README.txt`

The VTK files contain the exact extracted triangle meshes, without smoothing
or coordinate perturbation. The audited DICOM geometry is patient LPS; the
exporter converts it to 3D Slicer's RAS convention (negate X and Y, preserve Z)
so the models can be overlaid with the source images.

In 3D Slicer:

1. Load the original arterial CT series and matching DICOM SEG through the
   DICOM module/browser, preserving their source geometry.
2. Load `kidney.vtk` and `mass.vtk` as model files.
3. Show the models alongside the SEG and CT. Adjust model opacity and segment
   visibility to compare boundaries on axial, coronal and sagittal slices.
4. Inspect the five documented Kidney self-contact locations as well as the
   overall renal contour and Mass boundary.

The meshes are geometric derivatives for engineering inspection, not clinical
models or validated anatomical surfaces. Keep the exports and original data
outside Git.

The scripts print diagnostic results to the terminal. Run them individually
when investigating a particular aspect of the dataset.

## Script reference

| Script | Purpose |
|---|---|
| `inspect_kidney.py` | Initial CT and SEG metadata inspection |
| `inspect_seg_frames.py` | SEG frame and segment inspection |
| `compare_kidney_geometry.py` | CT/SEG geometry and source-reference comparison |
| `inspect_kidney_voxels.py` | CT pixel/HU and segmentation voxel inspection |
| `inspect_kidney_alignment.py` | Frame references, spatial alignment and bounds |
| `inspect_kidney_mask_relationship.py` | Kidney/Mass mask overlap and separation |
| `inspect_kidney_components.py` | Connected-component analysis |
| `inspect_kidney_mask_adjacency.py` | Six- and 26-neighbour mask adjacency |
| `inspect_kidney_mass_distance.py` | Anisotropic voxel-centre distance analysis |

## Important interpretation notes

- The SEG is a multi-frame binary segmentation, not a single integer label
  map. Frame-to-segment identification and source-image references matter.
- CT `SliceThickness` and actual slice-centre spacing are distinct values.
  Do not substitute one for the other.
- Distance-transform measurements describe distances between voxel centres;
  they are not mesh-surface distances.
- Kidney and Mass are independent source segments. Do not merge them or
  alter their relationship based solely on these audit results.
- Mask-derived volumes and geometric measurements are dataset observations,
  not clinically validated anatomical measurements.
- The dataset contains Kidney and Mass segments; it does not provide detailed
  kidney substructures such as cortex, medulla, calyces or vessels.

For the measured results, geometry details, limitations and investigation
decisions, see:

- `docs/kidney-data-audit.md`
- `docs/kidney-volume-investigation-handoff.md`

## Data and version control

The original CT and SEG files are not included in this repository. Keep the
downloaded dataset outside Git unless its licensing and repository policies
have been explicitly reviewed.

These scripts preserve the inspection procedures used during the audit.
They do not replace the source DICOM files or the documented audit results.
