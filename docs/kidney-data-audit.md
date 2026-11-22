# Kidney dataset and capability audit

Last reviewed: 2026-11-18

## Purpose

Select a manageable kidney dataset for an SVF educational-visualization
investigation. This is a source/capability audit, not an implementation
authorization. The immediate target is lesson building beginning January 2027.

The candidate must be chosen against a specific learning objective. A dataset
for kidney/tumour segmentation is not automatically a comprehensive kidney
anatomy dataset.

## Existing SVF capabilities relevant to this work

Repository inspection on 2026-11-18 found:

### Present

- `Dataset` owns immutable source geometry and named scalar/vector fields
  with explicit point, cell, and dataset association semantics.
- `UnstructuredGrid3` represents mixed linear cells, including tetrahedra and
  hexahedra, with CSR connectivity and source cell ordering.
- The VTK legacy reader accepts **ASCII `UNSTRUCTURED_GRID` only**. Its
  supported cells are LINE, TRIANGLE, QUAD, TETRA and HEXAHEDRON. It imports
  supported point/cell scalar and 3-component vector fields.
- The Explorer can load these VTK files, extract volume-cell boundaries,
  triangulate them, and optionally map scalar fields to the rendered surface.
- Tetrahedral point-scalar isosurfaces are implemented with source-edge /
  source-point and source-cell provenance.
- Surface meshes can be loaded as semantic `Structure` parts with labels,
  relations, layers, named views, picking, isolation, sectioning and peeling.
- Structure sectioning clips closed outward-wound mesh parts and caps the
  resulting cuts.

### Not present / explicit gaps

- No VTK `STRUCTURED_POINTS`, `STRUCTURED_GRID`, `IMAGE_DATA`, or XML
  image-data volume reader.
- No NIfTI reader and no DICOM series reader.
- No authoritative 3D voxel/image-volume type carrying dimensions, spacing,
  origin, direction/orientation, scalar storage and source metadata.
- No segmentation-label-map type with a declared label table and voxel
  association semantics.
- No general structured-volume isosurface extraction (e.g. marching cubes);
  current isosurface extraction is tetrahedra-only on point-associated
  scalar data.
- No volume rendering or slice-stack viewer for source image volumes.
- Existing structure sectioning operates on reconstructed closed surface
  meshes; it does not yet sample or display the corresponding source CT slice.
- Existing field/mesh support must not be mistaken for image-volume support.
  A tetrahedral mesh and a voxel image volume are different source geometries.

The current VTK import contract is documented in
`docs/vtk-legacy-ascii-import.md`; current isosurface limits are in
`docs/tetrahedral-isosurface.md`; Explorer import behavior is in
`docs/explorer-vtk-loading.md`.

## Public kidney dataset candidates

### Candidate A — KiTS19 / C4KC-KiTS via The Cancer Imaging Archive

TCIA's collection record describes 210 patients/studies, CT and SEG modalities,
621 series, 71,423 images and approximately 40.7 GB for the full collection.
The collection is listed under CC BY 3.0 and described as available for
commercial, scientific and educational use subject to TCIA's data-use policy.

Source: https://wiki.cancerimagingarchive.net/pages/viewpage.action?pageId=64684840

Licence and distribution distinction:
- The original KiTS19 challenge distribution states CC BY-NC-SA.
- The harmonized C4KC-KiTS collection published by TCIA lists the CT and
  segmentation DICOM data under CC BY 3.0 and says it may be used for
  commercial, scientific and educational purposes subject to TCIA's data-use
  policy.
- These are different distributions/representations. Do not apply the TCIA
  licence to files downloaded from the original challenge GitHub, or the
  challenge licence to the harmonized TCIA package without checking the exact
  package. For a distributable SVF lesson asset, prefer the TCIA package and
  preserve its attribution and required citation.
- TCIA's current harmonized record lists 210 participants, 210 studies, 621
  series, 71,423 images and approximately 40.7 GB total. It provides CT and
  DICOM SEG modalities. Its note says the segmentation corresponds to the
  arterial phase in every case; do not pair it with a different CT phase.
- The original challenge describes anonymized NIfTI image and label volumes,
  with labels for background, kidney and tumour. That is a different
  acquisition/conversion route from the TCIA DICOM representation.

Advantages:
- The original challenge provides paired CT and segmentation volumes.
- TCIA supports browsing/filtering and downloading subsets rather than
  requiring the full collection.
- Its volume and segmentation representation is directly relevant to an
  initial NIfTI/label-map ingestion experiment.

Still to verify on one selected case:
- Which exact distribution/version is being downloaded and which licence
  applies to those exact files.
- Exact segmentation objects and semantic label meanings in that package.
- Whether the case contains all required image slices and spatial metadata.
- Practical per-case download size and conversion workflow.
- Whether the selected segmentation is suitable for the intended lesson.

### Candidate B — KiTS23

The official challenge page describes 599 cases: 489 training and 110 test.
Each case has a contrast-enhanced preoperative CT and segmentation classes:
1 = kidney, 2 = tumour, 3 = cyst; 0 is background. The kidney class includes
parenchyma and non-adipose tissue within the hilum. The page says image and
segmentation data are CC BY-NC-SA 4.0. Imaging is downloaded from the
organisers' linked imaging repository; segmentations are stored in the
challenge GitHub repository.

Sources:
- https://wiki.cancerimagingarchive.net/pages/viewpage.action?pageId=133072129
- https://wiki.cancerimagingarchive.net/pages/viewpage.action?pageId=64684840
- https://wiki.cancerimagingarchive.net/pages/viewpage.action?pageId=80969923
- https://kits19.grand-challenge.org/data/
- https://kits-challenge.org/kits23/
- https://github.com/neheller/kits23
- https://github.com/TCIA/data-retriever
- https://wiki.cancerimagingarchive.net/pages/viewpage.action?pageId=4555089

Advantages:
- Clear documented label semantics and paired image/segmentation data.
- More cases and an explicit label-map workflow.
- Useful for testing volumetric image plus segmentation ingestion.

Limitations:
- CC BY-NC-SA 4.0 is non-commercial and share-alike. Do not assume this is
  suitable for SVF lesson assets or any future commercial distribution.
- The labels are kidney, tumour and cyst, not cortex, medulla, renal pyramids,
  calyces, renal pelvis, vessels or ureter. It is therefore not a complete
  normal-kidney anatomy model.
- A tumour dataset supports a kidney/tumour/cyst learning objective; it does
  not by itself support a detailed nephron or internal kidney anatomy lesson.

## Preliminary recommendation

For the first volumetric ingestion experiment, audit one manageable
KiTS19/TCIA case first because the TCIA listing states CC BY 3.0 and CT+SEG
availability. Confirm the exact case contents, segmentation semantics,
download size and licence/attribution details before adopting it.

Keep KiTS23 as a technically attractive alternative/reference for its
explicit kidney/tumour/cyst label map, but do not adopt it for distributable
lesson assets without resolving the non-commercial/share-alike licence
implications.

Neither candidate should be described as a complete anatomical kidney model.
For a lesson about cortex, medulla, renal pyramids, calyces, pelvis, renal
vessels or ureter, we will likely need a separate appropriately licensed
anatomical model, an SVF-authored schematic, or multiple complementary
sources. Any authored model must be validated and labelled as schematic where
appropriate.

## Proposed audit sequence (no implementation yet)

1. Open the [TCIA C4KC-KiTS collection record](https://wiki.cancerimagingarchive.net/pages/viewpage.action?pageId=133072129)
   and use its Search/Data Portal workflow. Select one participant and add
   the arterial-phase CT series plus its corresponding DICOM SEG series to
   the cart. Do not download all 210 cases.
2. Download the selected-series manifest and use the official
   [TCIA Data Retriever](https://github.com/TCIA/data-retriever) to fetch only
   that case's selected series. Preserve the manifest and collection record
   alongside the files.
3. Check that the SEG references the selected arterial-phase CT series before
   proceeding; do not assume that a similarly named CT series is the match.
4. Record the exact patient/study/series identifiers and sizes in the local
   audit notes. Avoid publishing any patient-level metadata beyond what the
   anonymized dataset and its terms permit.
5. Select one case from KiTS19/TCIA and obtain only the necessary image and
   segmentation objects.
2. Record its source identifiers, citation, licence, file formats, file sizes,
   image dimensions, spacing, origin/orientation, scalar type, and segmentation
   labels.
3. Inspect a few orthogonal source slices and the segmentation overlay in an
   established medical-image viewer. Confirm the mask aligns with the CT.
4. Decide the first educational question. A bounded initial case could be:
   identify the kidney in CT, distinguish kidney from tumour/cyst labels, and
   explore its 3D extent. Do not silently expand this to detailed internal
   anatomy.
5. Compare the actual files with SVF's current types. Identify the smallest
   reusable volume and label-map representation required to preserve source
   coordinates, calibration, values, labels and provenance.
6. Only then write a reviewed design contract for volume loading, segmentation
   inspection, surface reconstruction and source-consistent sectioning.

## Decision boundary

No new volume type, NIfTI/DICOM reader, marching-cubes implementation, or
kidney-specific production API is authorized by this audit. The selected
dataset and learning objective must be confirmed first. Synthetic volume
fixtures with exact dimensions, spacing, labels and expected measurements
should accompany any eventual implementation tests.


## Case-level audit — KiTS-00005 (2026-11-18)

### Scope and provenance

The first local case audit used the harmonized TCIA C4KC-KiTS DICOM
distribution, not the original KiTS19 challenge NIfTI distribution.

- Collection: [TCIA C4KC-KiTS](https://wiki.cancerimagingarchive.net/pages/viewpage.action?pageId=133072129)
- Participant/case: KiTS-00005
- Selected imaging: arterial-phase CT only, plus its corresponding DICOM SEG.
- Local case root:
  datasets/kidney_kits19/downloaded/c4kc_kits/KiTS-00005
- Study directory: 2003-11-27-three_phase__abdomen-21232
- CT series directory: 6-arterial-78517
- SEG series directory: 300-Segmentation-25690
- Downloaded files: 834 CT DICOM instances and one multi-frame SEG DICOM object
  (835 files total; approximately 474 MB).
- No late-phase or non-contrast CT series was downloaded.
- Keep the imaging files local; do not add the approximately 474 MB case data
  to the source repository or commit patient-level imaging data.

The CT series identifiers recorded during inspection:

- Study Instance UID:
  1.3.6.1.4.1.14519.5.2.1.6919.4624.184137344550130978153760721232
- CT Series Instance UID:
  1.3.6.1.4.1.14519.5.2.1.6919.4624.306271435885747871637212478517
- SEG Series Instance UID:
  1.2.276.0.7230010.3.1.3.0.74474.1588583289.825690

The SEG references the arterial CT series above. Its two Segment Sequence
entries are segment 1, Kidney, and segment 2, Mass; both are marked
SEMIAUTOMATIC.

### CT volume geometry and intensity

| Property | Observed value |
|---|---|
| Rows × columns | 512 × 512 |
| Number of CT instances | 834 |
| Pixel spacing | 0.9765625 × 0.9765625 mm |
| Slice-centre spacing | 0.5 mm |
| Slice-centre Z range | -737.4 to -320.9 mm |
| Distance between first and last slice centres | 416.5 mm |
| Image orientation | [1, 0, 0, 0, 1, 0] |
| Rescale slope | 1 |
| Rescale intercept | -1024 |
| Stored pixel range (sampled middle slice) | 0 to 2957 |
| HU range (sampled middle slice) | -1024 to 1933 |
| HU percentiles [1, 25, 50, 75, 99] | [-1024, -1013, -144, -79, 176] |

The CT header reports SliceThickness = 1 mm, while the measured distance
between consecutive ImagePositionPatient Z coordinates is consistently
0.5 mm (833 intervals). These are distinct DICOM properties; use the measured
slice-centre spacing for the volume grid, not SliceThickness.

A sampled middle slice was used for pixel/HU range inspection. The percentile
values are not claimed to be whole-volume statistics.

### SEG geometry and CT correspondence

- Modality: SEG.
- SegmentationType: BINARY.
- BitsAllocated / BitsStored / HighBit: 1 / 1 / 0.
- Dimensions per frame: 512 × 512.
- NumberOfFrames: 1,668 (834 Kidney frames and 834 Mass frames).
- Shared SEG PixelSpacing: 0.9765625 × 0.9765625 mm.
- Shared SEG SpacingBetweenSlices: 0.5 mm.
- Every frame references one of the 834 arterial CT SOP instances.
- Distinct referenced CT instances per segment: 834.
- Missing CT references: 0.
- Position mismatches: 0 in the frame-to-instance comparison.
- A separate coordinate comparison reported a maximum position difference
  of approximately 0.00001875 mm, consistent with DICOM decimal precision.
- Orientation mismatches: 0.

The SEG is a multi-frame binary object, not a single integer label-map image.
A reader must use per-frame segment identification and source-image
references; it must not assume frame index alone is a globally meaningful
slice index or that all segments are mutually exclusive in every dataset.

### Segment voxel audit

| Measurement | Kidney (segment 1) | Mass (segment 2) |
|---|---:|---:|
| Frames | 834 | 834 |
| Non-empty frames | 283 | 119 |
| Empty frames | 551 | 715 |
| Foreground voxels | 902,167 | 126,444 |
| Maximum foreground pixels in one frame | 5,883 | 1,828 |
| Non-empty Z range | -551.4 to -410.4 mm | -504.4 to -445.4 mm |
| Image row-index bounds | 256–347 | 265–315 |
| Image column-index bounds | 256–420 | 383–431 |

The row/column bounds are pixel-index bounds from the decoded masks (zero-based
array indices). They are 2D bounding limits aggregated over all non-empty
frames, not a 3D physical bounding box.

Using the SEG voxel spacing (0.9765625 × 0.9765625 × 0.5 mm), the foreground
counts correspond to approximate binary-mask voxel volumes of 430.2 cm³
(Kidney) and 60.3 cm³ (Mass). These are mask-derived volumes, not independently
validated anatomical organ/tumour volume measurements. Do not present them as
clinical measurements.

### Connected components and mask relationship

Connected components were counted using 26-connectivity in the voxel grid.

- Kidney: 4 components. The largest contains 902,158 voxels; the remaining
  components contain 5, 3 and 1 voxels (9 isolated voxels total).
- Mass: 1 component containing all 126,444 voxels.
- Exact voxel overlap between Kidney and Mass: 0.
- All 126,444 Mass voxels are outside the Kidney binary mask.
- Mass voxels with a face-neighbour (6-connectivity) in Kidney: 3,652.
- Mass voxels with any face/edge/corner neighbour (26-connectivity) in Kidney:
  6,203.
- Z range of 26-neighbour contact: -504.4 to -451.9 mm.

A Euclidean distance transform was also computed from the Kidney voxel mask,
using sampling (Z, row, column) = (0.5, 0.9765625, 0.9765625) mm. Distances
are from Mass voxel centres to the nearest Kidney voxel centre, not exact
distances between reconstructed triangle surfaces.

| Distance threshold | Mass voxels within threshold | Percentage |
|---|---:|---:|
| ≤ 0.5 mm | 1,275 | 1.01% |
| ≤ 1 mm | 4,154 | 3.29% |
| ≤ 2 mm | 9,084 | 7.18% |
| ≤ 3 mm | 15,172 | 12.00% |
| ≤ 5 mm | 26,641 | 21.07% |
| ≤ 10 mm | 55,542 | 43.93% |

Additional distance statistics:

- Minimum: 0.5 mm.
- Mean: 12.1359 mm.
- Median: 11.3732 mm.
- 25th / 75th percentiles: 5.8807 / 17.6123 mm.
- 90th / 95th / 99th percentiles: 22.8978 / 25.5786 / 29.2934 mm.
- Maximum: 33.4526 mm.

### Interpretation and limitations

1. The case provides a spatially matched arterial CT volume and two named
   binary segments suitable for a first DICOM CT + SEG ingestion investigation.
2. Kidney is one dominant connected region with nine isolated foreground
   voxels; Mass is one connected region under 26-connectivity.
3. The two binary masks are disjoint, but some Mass voxels directly neighbour
   Kidney voxels. Preserve them as separate source segments. Do not merge them,
   fill the space between them, or force Mass into the Kidney mask.
4. The measurements do not establish whether the tumour is anatomically
   enclosed by the kidney's outer envelope. The relatively large centre-to-centre
   distances must not be interpreted as proof that the tumour is anatomically
   detached from the kidney.
5. The sampled CT intensity percentiles are from one middle slice only.
6. Connectedness and voxel adjacency do not establish surface manifoldness,
   mesh quality, or anatomical correctness.
7. No independent medical-image viewer overlay review or reconstructed-surface
   comparison has yet been recorded. Do not claim those validations.
8. The dataset represents kidney and mass segmentation, not detailed normal
   kidney anatomy (cortex, medulla, pyramids, calyces, pelvis, vessels or ureter).

### Reproducibility and local inspection scripts

The following diagnostic scripts were created and run locally during this
audit:

- inspect_kidney.py — initial CT/SEG DICOM metadata.
- inspect_seg_frames.py — per-frame segment counts and positions.
- compare_kidney_geometry.py — CT/SEG geometry and reference comparison.
- inspect_kidney_voxels.py — sampled CT pixel/HU data and SEG foreground counts.
- inspect_kidney_alignment.py — frame references, bounds and non-empty
  slice ranges.
- inspect_kidney_mask_relationship.py — voxel overlap and disjoint counts.
- inspect_kidney_components.py — 26-connected component counts.
- inspect_kidney_mask_adjacency.py — 6- and 26-neighbour contact.
- inspect_kidney_mass_distance.py — anisotropic voxel-centre distance transform.

These scripts were run from the repository root against the local dataset.
Their outputs are summarized above. The scripts themselves have not yet been
moved into a versioned tools directory; do not assume they are committed.
The local Python environment used pydicom 3.0.2, NumPy, and SciPy for the
distance-transform check.

### Status and next boundary

The single-case source/geometry/voxel audit is complete for the checks listed
above. The dataset is a candidate engineering fixture for the first volume
and segmentation pipeline, not yet an approved distributable lesson asset.

Next work should:
1. Preserve the diagnostic scripts in a versioned
   tools/dataset-audits/kits-00005/ directory and make their outputs
   reproducible.
2. Inspect the existing core API and tests before proposing new volume types.
3. Write and review a design contract for the smallest reusable image-volume
   and segmentation representation.
4. Add synthetic fixtures with known geometry and values before relying on
   this one real-world case for regression tests.
5. Separately review exact licence/attribution requirements before distributing
   any case-derived lesson assets.

No volume type, DICOM reader, NIfTI reader, marching-cubes implementation,
volume renderer, or kidney-specific production API is authorized by this
audit.
