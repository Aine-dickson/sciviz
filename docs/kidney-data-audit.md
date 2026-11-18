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

Important licence discrepancy:
- The current TCIA C4KC-KiTS record lists the harmonized CT+SEG collection as
  CC BY 3.0 and describes commercial, scientific and educational use under
  TCIA's data-use policy.
- The original KiTS19 challenge data page states CC BY-NC-SA terms. These
  statements refer to different distribution/record contexts and must not be
  silently treated as interchangeable. We must identify the exact files and
  their applicable rights before redistribution or use in distributable
  lessons. For planning purposes, treat the original challenge data as
  non-commercial/share-alike unless the exact TCIA package's rights and
  provenance are confirmed.
- The original challenge describes image and ground-truth label volumes in
  anonymized NIfTI format, with labels for background, kidney and tumour.

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
- https://kits-challenge.org/kits23/
- https://github.com/neheller/kits23
- https://kits19.grand-challenge.org/data/
- https://wiki.cancerimagingarchive.net/pages/viewpage.action?pageId=80969923

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

1. Select one case from KiTS19/TCIA and obtain only the necessary image and
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
