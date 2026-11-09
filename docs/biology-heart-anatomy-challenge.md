# B1 — Human heart anatomy and spatial relationships

## Status

**Design accepted; primary dataset selection revised for review.** The initial BodyParts3D audit found that its PART-OF element mappings do not provide independent valve geometry. A better-aligned, research-derived tetrahedral dataset has been identified: the Rodero et al. (2021) virtual cohort of adult healthy four-chamber heart meshes. The user should accept this revised source choice before any download, fixture generation, or SVF code change.

## Objective

Evaluate standalone SVF's ability to import, represent, render, and inspect a multi-region tetrahedral model of the human heart while preserving the anatomical meaning and provenance of the source fields.

This first bounded evaluation focuses on:
- the spatial arrangement of the four chamber myocardium regions;
- the aorta and pulmonary artery walls;
- the four valve-plane regions;
- the major vessel and appendage inlets/borders;
- the source's point-associated universal ventricular coordinates; and
- the source's supplied myocardial fibre-direction vectors.

This is a static anatomy and scientific-data visualization challenge. It does not ask SVF to simulate cardiac electrophysiology, mechanics, or blood flow.

## Primary dataset candidate

**Virtual cohort of adult healthy four-chamber heart meshes from CT images**  
Rodero et al., 2021. Zenodo DOI: https://doi.org/10.5281/zenodo.4590294

Official record:
https://zenodo.org/records/4590294

The cohort contains 20 four-chamber heart meshes segmented from end-diastolic CT. The patients initially attended an emergency department with acute chest pain; no cardiac condition was detected during follow-up, so the authors describe them as representative of healthy/asymptomatic hearts. The meshes were used for electromechanical simulations and a statistical shape model.

Dataset properties documented by the authors:
- 20 patient-derived four-chamber heart models.
- Tetrahedral meshes with approximately 1 mm average edge length.
- ASCII legacy VTK UNSTRUCTURED_GRID files.
- Coordinates in millimetres.
- Cell-associated anatomical meshing tags.
- Point-associated universal ventricular coordinates (UVCs): rho, phi, Z, and V.
- Supplied ventricular fibre-direction vectors.
- One compressed archive per case; the 20 case archives range from approximately 54.9 MB to 103.2 MB, with about 1.5 GB total.

The data are shared under CC BY 4.0, as confirmed by the CEMRG dataset catalogue and PyAnsys Heart acknowledgements. Preserve attribution to the dataset authors and cite the accompanying paper:
Strocchi et al. (2021), “Linking statistical shape models and simulated function in the healthy adult human heart,” PLOS Computational Biology, DOI: https://doi.org/10.1371/journal.pcbi.1008851.

For the first audit, use one case only (proposed default: 01.tar.gz). Do not download the full cohort until the single-case workflow is established.

### Source anatomical labels

The dataset description lists 24 element labels:

1. Left ventricle myocardium
2. Right ventricle myocardium
3. Left atrium myocardium
4. Right atrium myocardium
5. Aorta wall
6. Pulmonary artery wall
7. Mitral valve plane
8. Tricuspid valve plane
9. Aortic valve plane
10. Pulmonary valve plane
11. Left atrium appendage inlet
12. Left superior pulmonary vein inlet
13. Left inferior pulmonary vein inlet
14. Right inferior pulmonary vein inlet
15. Right superior pulmonary vein inlet
16. Superior vena cava inlet
17. Inferior vena cava inlet
18. Left atrial appendage border
19. Right inferior pulmonary vein border
20. Left inferior pulmonary vein border
21. Left superior pulmonary vein border
22. Right superior pulmonary vein border
23. Superior vena cava border
24. Inferior vena cava border

The labels are cell tags in the tetrahedral mesh. The four valves are represented as **valve planes**, not detailed leaflets/cusps. The pulmonary veins and venae cavae are represented by inlet/border regions, not complete vessel tubes. Do not describe these tags as complete valve or vessel geometry.

### Scientific limitations of the source

- The authors call the cohort healthy/asymptomatic based on follow-up; it is not a random sample of the entire healthy population.
- It is a cohort of individual CT-derived anatomies, not a single canonical heart.
- The model provides chamber myocardium and selected vessel walls, valve planes, and inlet/border regions. It does not provide detailed valve leaflets, chordae, papillary muscles, coronary vessels, or complete pulmonary vein/SVC/IVC geometry.
- The fibre directions are supplied rule-based model fields, not directly measured histological fibre orientations.
- The UVCs are mathematical coordinates defined by the authors; they are not physical measurements in millimetres.
- No cardiac solver or simulation output is required for B1.

## Secondary anatomical references

### BodyParts3D 4.0

Official download page:
https://dbarchive.biosciencedbc.jp/en/bodyparts3d/download.html

BodyParts3D is useful as a whole-body coordinate/anatomical reference, but it is not selected as the primary B1 mesh. Its PART-OF data maps compound concepts to shared FJ element meshes. The four valve concepts overlap chamber element mappings, so it does not provide independent valve surfaces. Its official coordinate diagram specifies millimetres, anatomical left X>0/right X<0, posterior Y>0/anterior Y<0, and superior increasing +Z. Its release note also cautions that the data may contain errors if used as a canonical model.

See the companion audit: biology-heart-anatomy-source-audit.md.

### HuBMAP CCF / Human Reference Atlas

Official library:
https://hubmapconsortium.github.io/ccf/pages/ccf-3d-reference-library.html

The library provides male and female whole-heart GLB reference objects developed by specialist 3D medical illustrators and approved by organ experts. Use these as independent whole-organ visual references, not as the primary multi-region volumetric dataset.

### Detailed teaching-heart reference (optional)

The Layered Human project publishes a 119-part procedural teaching-heart model with separate chamber wall panels, valve leaflets/cusps, chordae, papillary muscles, coronary arteries, cardiac veins, and great vessels. Its project README describes the model as built from published measurements and its own original model assets as MIT-licensed:
https://github.com/ethanprince4/AnatomyExplorer

This is an optional visual comparison only. It is not the primary B1 dataset, is a surface/GLB model rather than the tetrahedral VTK cohort, and has not undergone independent expert validation for this challenge. Do not merge its geometry into the patient-derived cohort.

## Evaluation boundary

- Evaluate SVF independently through svf-core, svf-render, and the standalone Explorer.
- Mava and Prismal are excluded entirely.
- No heart solver, electrophysiology, contraction, blood-flow computation, time integration, or animation.
- No generic scene graph or anatomical editor.
- No new importer, surface-mesh system, material-interface extractor, or interaction framework is presumed by the acceptance criteria.
- If current SVF cannot reveal an internal material interface or distinguish structures from cell tags, record that as a limitation. Do not pre-process the dataset in a way that conceals the limitation.
- Any new SVF capability requires a separate, bounded design proposal after the evaluation documents the gap.

## Data contract for the selected case

Before fixture preparation, establish:
1. exact case archive filename, Zenodo record/version, and checksum;
2. archive member names and exact VTK file selected;
3. point, tetrahedron, and cell-type counts;
4. cell-tag names/IDs and their mapping to the 24 anatomical labels;
5. point-associated UVC arrays and fibre-vector association/tuple counts;
6. coordinate bounds, millimetre units, and anatomical orientation;
7. topology integrity: finite coordinates, valid connectivity, positive tetrahedron volumes, and face-incidence statistics;
8. source attribution, CC BY 4.0 notice, and every transformation applied.

Keep the original source mesh immutable. Any format conversion or derived fixture must preserve cell tags, point fields, fibre vectors, and source-to-output identity. Do not apply per-region transforms. If coordinate conversion is needed for SVF, apply one documented transform to all points and preserve the original coordinates in the audit report.

## B1 acceptance criteria

| ID | Acceptance condition | Evidence |
|---|---|---|
| B1-01 | Source provenance and licence verified | Zenodo record, paper citation, CC BY 4.0 attribution, archive checksum |
| B1-02 | Dataset structure is independently audited | Counts, connectivity, cell types, bounds, topology report |
| B1-03 | All 24 anatomical cell labels are mapped correctly | Tag-ID/name table and per-label cell counts |
| B1-04 | Coordinate units and orientation are established | Millimetre bounds and anatomical reference checks |
| B1-05 | Existing SVF VTK import handles the selected case | Exact import counts and field inventory |
| B1-06 | Outer boundary is extracted and rendered correctly | Explorer output and reproducible views |
| B1-07 | Cell-associated anatomical tags remain inspectable/mappable | Field inspection and rendered evidence |
| B1-08 | Point-associated UVC fields remain available | Field inventory and selected-value inspection |
| B1-09 | Supplied fibre vectors can be resolved and visualized | Vector source counts, direction checks, visual evidence |
| B1-10 | Internal anatomical interfaces are meaningfully inspectable | Explicit pass/fail/blocked result; no concealed preprocessing |
| B1-11 | Reproduction and attribution are complete | Fixture manifest, exact commands, and audit instructions |

A failure or unsupported capability is a valid evaluation result. Do not implement new capabilities merely to turn a failed criterion into a pass.

## Evaluation views

At minimum, record:
1. anterior;
2. posterior;
3. superior;
4. right and left oblique;
5. any internal/material-interface view that the existing SVF pipeline can expose.

The source coordinate axes must be verified before naming the camera views. Camera presets are evidence artifacts, not a requirement to build a medical atlas UI.

## Planned workflow

### B1.1 — Single-case source audit (next)

- Download only 01.tar.gz from the official Zenodo record.
- Inspect archive contents and identify the ASCII VTK unstructured-grid file.
- Verify counts, cell tags, point fields, vectors, coordinates, and topology.
- Record the exact source archive checksum and the attribution.
- Decide whether the existing SVF importer and boundary pipeline can evaluate the case without data-altering preprocessing.

### B1.2 — Finalize fixture contract (separate review)

Freeze the case, tag mapping, coordinate transform (if any), independent verifier, and fixture packaging. Do not include the full 20-case cohort unless the single-case evaluation demonstrates a concrete need.

### B1.3 — Fixture preparation (separate approval)

Prepare a reproducible, source-preserving fixture and verifier. Keep large raw upstream downloads out of Git unless size and licence justify it.

### B1.4 — SVF evaluation (separate approval)

Run import, boundary extraction, categorical field mapping, UVC inspection, and fibre-vector visualization. Record pass/fail/blocked results with exact terminal output and screenshots.

## Explicit non-goals

- Valve leaflet/cusp geometry or detailed valve mechanics.
- Complete pulmonary vein, vena cava, or coronary vessel trees.
- Heartbeat or cardiac-cycle animation.
- Electrical conduction or electrophysiology simulation.
- Pressure, blood-flow, velocity, or computational fluid dynamics.
- Tissue mechanics or contraction simulation.
- Pathology, diagnosis, treatment, or patient-specific clinical interpretation.
- Whole-body anatomy or a general medical atlas.
- Automatic segmentation, mesh reconstruction, or invented missing structures.
- Mava/Prismal integration.
- A general-purpose scene graph, editor, or anatomical authoring system.
