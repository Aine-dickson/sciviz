# B1.1 — Preliminary heart anatomy source audit

## Status

**Online source audit complete; single-case download and local mesh audit pending.**

The initial BodyParts3D candidate was not selected as the primary dataset because its compound-concept mappings overlap chamber and valve element meshes. A better fit for SVF's current VTK import path has been identified: the Rodero et al. (2021) virtual cohort of adult healthy four-chamber heart meshes.

No dataset archive has yet been downloaded or locally inspected. No fixture or SVF production code has been created.

## Primary candidate — Rodero et al. (2021) healthy four-chamber cohort

- Official record: https://zenodo.org/records/4590294
- DOI: https://doi.org/10.5281/zenodo.4590294
- Paper: https://doi.org/10.1371/journal.pcbi.1008851
- CEMRG model catalogue: https://cemrg.co.uk/models/
- Format: ASCII legacy VTK UNSTRUCTURED_GRID with tetrahedral cells.
- Geometry: 20 end-diastolic CT-derived four-chamber heart meshes, approximately 1 mm average edge length.
- Units: point coordinates in millimetres.
- Fields: CELL_DATA anatomical meshing tags; supplied ventricular fibre VECTORS; POINT_DATA universal ventricular coordinates rho, phi, Z, and V.
- Packaging: one tar.gz archive per case. Case 01 is 91.1 MB; the 20 case archives total about 1.5 GB.
- Licence: CC BY 4.0, confirmed by the CEMRG catalogue and PyAnsys Heart acknowledgements. The Zenodo record's Rights field is not populated, so retain the cross-source licence evidence and the authors' required attribution in the fixture manifest.

The authors describe the cohort as healthy/asymptomatic because the source patients attended emergency departments with acute chest pain but no cardiac condition was detected during follow-up. It is a CT-derived cohort of individual anatomies, not a canonical population-average heart.

### Published cell-label inventory

The dataset describes 24 anatomical element labels:
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

The labels are tetrahedral cell tags. The valves are represented as **planes**, not leaflets or cusps. The pulmonary veins and venae cavae are represented by inlets/borders, not complete vessel tubes. The dataset does not provide coronary vessels, chordae, papillary muscles, or detailed valve apparatus.

The UVCs are mathematical coordinates, not millimetre measurements. Fibre directions are supplied rule-based model fields, not directly measured histological fibre orientations.

### Why this is the primary candidate

The source is research-derived from CT segmentation and supplies tetrahedral meshes with named cell tags. Its ASCII VTK UNSTRUCTURED_GRID format aligns with the current SVF import route, avoiding an immediate need for an OBJ/GLB importer or surface-mesh conversion. A single case archive is small enough for an initial bounded audit. The cell tags, UVC fields, and fibre vectors also provide meaningful scientific data to test beyond surface rendering.

### Known caveats

- The cohort is asymptomatic/healthy by clinical follow-up, not a random sample of the general population.
- The mesh is a static end-diastolic anatomical representation.
- The four valve labels are planes only.
- The pulmonary veins and venae cavae are represented by inlet/border regions, not full vessel geometry.
- The source's fibre directions are rule-based.
- Internal material interfaces may not be visible through SVF's current exterior-boundary extraction. This is an evaluation question, not a reason to preprocess away the interfaces.

## Rejected as primary — BodyParts3D 4.0 PART-OF

Official sources:
- https://dbarchive.biosciencedbc.jp/en/bodyparts3d/download.html
- https://dbarchive.biosciencedbc.jp/data/bodyparts3d/LATEST/README_e.html
- https://dbarchive.biosciencedbc.jp/data/bodyparts3d/LATEST/partof_parts_list_e.txt
- https://dbarchive.biosciencedbc.jp/data/bodyparts3d/LATEST/partof_element_parts.txt
- https://dbarchive.biosciencedbc.jp/data/bodyparts3d/20130619/release_4.0_e.html
- https://dbarchive.biosciencedbc.jp/archive/bodyparts3d/images/coordinate_system.png

The online metadata confirms all 19 concepts originally proposed, but the archive maps compound concepts to shared FJ element meshes. The four chambers map as follows:
- Right atrium: FJ2421, FJ2424, FJ2433, FJ2436, FJ2439.
- Left atrium: FJ2420, FJ2425, FJ2426, FJ2431, FJ2432, FJ2438.
- Right ventricle: FJ2417, FJ2419, FJ2421, FJ2423, FJ2427, FJ2430, FJ2433, FJ2434, FJ2436, FJ2437.
- Left ventricle: FJ2418, FJ2420, FJ2422, FJ2426, FJ2429, FJ2431, FJ2432, FJ2435.

Valve mappings:
- Tricuspid: FJ2421, FJ2433, FJ2436.
- Pulmonary: FJ2417, FJ2427, FJ2434.
- Mitral: FJ2420, FJ2426, FJ2431, FJ2432.
- Aortic: FJ2426, FJ2431, FJ2435.

Every valve mapping overlaps chamber mappings. It therefore does not supply four independent valve meshes separate from the chamber meshes. BodyParts3D remains useful as a whole-body reference but is not the selected B1 fixture source.

Its official coordinate diagram specifies millimetres; anatomical left X>0/right X<0; posterior Y>0/anterior Y<0; and superior increasing +Z. The official release note warns that the model may contain errors if used as a canonical model of common human anatomy and identifies it as an adult male reference.

Correction to the first B1 design draft: pulmonary trunk is BP9912, not BP9911 (BP9911 is right conus artery).

## Other secondary references

### HuBMAP CCF / Human Reference Atlas

https://hubmapconsortium.github.io/ccf/pages/ccf-3d-reference-library.html

Provides male and female whole-heart GLB reference objects, useful for independent whole-organ comparison. The public library does not establish separate meshes for all chambers and valves.

### Layered Human detailed teaching-heart model

https://github.com/ethanprince4/AnatomyExplorer

The project README describes a 119-part procedural heart built from published measurements: 15 chamber wall panels, five atrioventricular leaflets, six semilunar cusps, chordae, papillary muscles, coronary arteries, cardiac veins, and great vessels. Its own original model assets are stated to be MIT-licensed. It is a potentially useful visual reference for valve morphology, but it is a GLB surface model, not a patient-derived tetrahedral dataset, and it has not been independently expert-reviewed for this challenge. Do not merge it into the Rodero cohort.

## Proposed next action

After accepting the revised source selection:
1. Download only case 01 from the Rodero Zenodo record (91.1 MB).
2. Inspect the tar archive and locate the ASCII VTK file.
3. Audit point/cell counts, tetrahedral connectivity, cell tag IDs, UVC fields, fibre vectors, units, bounds, and coordinate orientation.
4. Run the existing SVF import path and record what it renders and which fields it preserves.
5. Decide whether the original B1 scope is supportable or should be narrowed to the exact anatomical regions present in the cohort.

No full-cohort download, fixture generation, or SVF production changes before the single-case audit and a separate review of the final fixture contract.
