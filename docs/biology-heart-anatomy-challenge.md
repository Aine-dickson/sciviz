# B1 — Human heart anatomy and spatial relationships

## Status

**Design proposal for review.** The source catalogue has been inspected online, including its anatomical concept inventory, mesh representation identifiers, and licence. The archive itself has not yet been downloaded or independently audited locally. No fixture, conversion, importer, or SVF production code is included in this design increment.

## Objective

Evaluate standalone SVF's ability to represent and inspect a multi-structure, anatomically meaningful 3D model of the human heart.

The evaluation asks whether SVF can preserve and communicate:
- the spatial arrangement of the four chambers;
- the positions and connections of major vessels;
- the locations and distinctions of the four principal valves;
- anatomical orientation and left/right relationships; and
- the identity and provenance of individual structures.

This is an anatomy representation and visualization challenge. It is not a cardiovascular simulation, diagnostic tool, or claim that SVF can infer anatomy from geometry.

## Evaluation boundary

- Evaluate SVF independently through `svf-core`, `svf-render`, and the standalone Explorer.
- Do not include Mava or Prismal, integration requirements, or assumptions about their data models.
- Do not add a heart solver, physiology simulation, blood-flow computation, electrical conduction, or animation.
- Do not implement a general scene graph or generic anatomical editor as a side effect of this challenge.
- A missing capability is recorded as an SVF limitation. Do not convert or simplify the data merely to make the existing importer accept it if that changes the evaluation question.
- The first task is source-data inspection and compatibility assessment. Implementation is a later, separately reviewed decision.

## Source-data audit

### Primary candidate: BodyParts3D 4.0, PART-OF tree

Official download page:
https://dbarchive.biosciencedbc.jp/en/bodyparts3d/download.html

The official catalogue exposes:
- `partof_parts_list_e.txt`: concept ID, representation ID, and English anatomical name;
- `partof_inclusion_relation_list.txt`: PART-OF inclusion relationships;
- `partof_element_parts.txt`: compound-organ element-file mappings; and
- `partof_BP3D_4.0_obj_99.zip`: polygon mesh data for the PART-OF tree (listed as 62 MB).

The catalogue's 99% reduction mesh archive is the proposed initial geometry source because it is a collection of polygon meshes and is accompanied by explicit anatomical concept and inclusion tables. The exact archive contents, OBJ naming, units, coordinate axes, mesh quality, and structure coverage still require local verification.

### Confirmed catalogue entries

The official PART-OF concept list contains the following entries. The BP identifiers below are the catalogue's representation IDs; they are not yet a claim that every corresponding mesh has passed archive-level verification.

| Structure | FMA concept ID | BodyParts3D representation ID |
|---|---|---|
| Heart | FMA7088 | BP9305 |
| Right atrium | FMA7096 | BP9831 |
| Left atrium | FMA7097 | BP9454 |
| Right ventricle | FMA7098 | BP9598 |
| Left ventricle | FMA7101 | BP9448 |
| Aorta | FMA3734 | BP10374 |
| Superior vena cava | FMA4720 | BP10386 |
| Inferior vena cava | FMA10951 | BP10397 |
| Pulmonary trunk | FMA8612 | BP9911 |
| Right pulmonary artery | FMA50872 | BP9427 |
| Left pulmonary artery | FMA50873 | BP9414 |
| Right superior pulmonary vein | FMA49914 | BP9365 |
| Right inferior pulmonary vein | FMA49911 | BP9407 |
| Left superior pulmonary vein | FMA49916 | BP9499 |
| Left inferior pulmonary vein | FMA49913 | BP9712 |
| Tricuspid valve | FMA7234 | BP9830 |
| Pulmonary valve | FMA7246 | BP9868 |
| Mitral valve | FMA7235 | BP9455 |
| Aortic valve | FMA7236 | BP9447 |

The four valves are present as catalogue concepts. Their individual mesh availability and geometric adequacy must still be checked in the archive; catalogue presence alone is not sufficient.

The concept and relationship tables must be used together. Do not assume a single OBJ corresponds one-to-one with every named compound anatomical concept. Some concepts may be composed from multiple element files. Preserve source IDs and mappings in the fixture manifest.

### Independent reference candidate: HuBMAP CCF / Human Reference Atlas

Official library:
https://hubmapconsortium.github.io/ccf/pages/ccf-3d-reference-library.html

The CCF 3D Reference Object Library provides expert-developed and organ-expert-approved reference objects. The published library includes male and female heart objects in GLB format and publishes release downloads and model mappings. Its reference objects are a useful independent comparison for overall heart form and spatial presentation.

Use HuBMAP as a secondary visual/structural reference, not as a substitute for BodyParts3D's independently named structure inventory unless the source audit demonstrates that its hierarchy is more suitable. Record the exact release, file identity, attribution, and licence if used.

### Licensing and attribution

BodyParts3D's official licence page states CC BY 4.0 and requires this attribution when redistributing the data or adapted material:

> BodyParts3D, © The Database Center for Life Science licensed under CC Attribution 4.0 International.

Official licence:
https://dbarchive.biosciencedbc.jp/en/bodyparts3d/lic.html

HuBMAP CCF 3D reference objects are also listed under CC BY 4.0. Record separate attribution for each source actually used. Include the original source URLs, release/version, access date, and any transformations in the fixture documentation. Do not imply that SVF created the anatomy.

## Anatomical scope

### Required structures

The initial required inventory is:
- four chambers: right atrium, left atrium, right ventricle, left ventricle;
- major systemic vessels: superior vena cava, inferior vena cava, aorta;
- pulmonary outflow and inflow: pulmonary trunk, right and left pulmonary arteries, and four pulmonary veins;
- four principal valves: tricuspid, pulmonary, mitral, and aortic valves.

The heart as a whole is retained as a source concept/reference where available. It must not be rendered as an opaque shell that hides the internal structures during the internal-structure inspection.

### Optional structures

Coronary arteries and veins, coronary sinus, septa, chordae tendineae, papillary muscles, and detailed valve leaflets may be recorded if source geometry is available and suitable. They are not required for B1 acceptance. Do not synthesize missing anatomy.

### Biological interpretation guardrails

- Anatomical right and left are the subject's right and left, not the viewer's.
- A front/anterior view must be labelled as such; never silently swap anatomical left/right to match screen position.
- The right and left pulmonary arteries are named relative to the lungs they supply.
- Pulmonary veins return oxygenated blood to the left atrium; pulmonary arteries carry blood away from the right ventricle through the pulmonary trunk. These flow relationships are reference annotations, not inferred from mesh proximity.
- Geometric intersection, contact, or overlap is not automatically proof of anatomical connection.
- Do not imply the model is a patient-specific scan or a universal representation of all human variation.

## Data contract for the eventual fixture

The source audit must establish before fixture generation:
1. exact archive filename and version;
2. exact source OBJ/element file(s) used for each selected FMA concept;
3. original units, axis directions, handedness, and coordinate origin;
4. whether meshes are already in a common coordinate frame;
5. vertex/face counts and bounding boxes per structure;
6. disconnected components, degenerate faces, duplicate vertices, normals, and any non-manifold/open boundaries;
7. whether each structure is independently represented or assembled from multiple source elements;
8. attribution and any modifications required for a distributable fixture.

Keep the source meshes immutable. If conversion is needed, preserve source-to-output mapping and document every transformation. Do not apply per-structure centering, scaling, mirroring, or rotation that destroys the original relative positions. Any coordinate conversion must be a single documented transform applied consistently to all structures.

Do not require every mesh to be watertight or manifold without first establishing that property from the source and its anatomical meaning. Report quality metrics per structure; do not silently repair geometry.

## B1 acceptance criteria

| ID | Acceptance condition | Evidence |
|---|---|---|
| B1-01 | Source and licence are verifiable | Source manifest, exact release and attribution |
| B1-02 | Required anatomical inventory is accounted for | FMA/BP/source-file mapping; explicit missing-item report |
| B1-03 | Mesh data is structurally valid for the chosen representation | Independent counts, finite coordinates, valid indices, face-quality report |
| B1-04 | Units and coordinate orientation are established | Source documentation and measured bounds/orientation checks |
| B1-05 | Relative positions are preserved | Shared-frame checks and comparison against source/reference views |
| B1-06 | Complete selected assembly can be represented | Standalone Explorer render with no missing required structure |
| B1-07 | Individual structures remain identifiable | Structure-by-structure visibility/inspection evidence |
| B1-08 | Anatomical orientation is interpretable | Anterior, posterior, superior, and oblique views with correct left/right |
| B1-09 | Internal chambers, vessels, and valves can be examined | Views that reveal internal structures without misleading occlusion |
| B1-10 | Provenance and reproduction are complete | Fixture manifest, attribution, exact commands, and audit report |

For B1-06 through B1-09, record both successful behavior and limitations of the current SVF architecture. No acceptance criterion presumes a particular new importer, scene abstraction, or interaction framework.

## Evaluation views

At minimum, prepare reference views for:
1. anterior (front);
2. posterior (back);
3. superior (top);
4. right and left oblique views;
5. an internal/chamber-focused view that makes the four chambers and valves inspectable.

These are evaluation viewpoints, not a requirement to build a medical atlas UI. Record camera/view definitions so results can be reproduced.

## Planned workflow

### B1.1 — Source-data audit

- Download the official BodyParts3D PART-OF archive and matching metadata tables.
- Inspect archive structure and verify the required concepts map to actual mesh element files.
- Inspect a HuBMAP heart reference object as an independent comparison.
- Record formats, coordinate conventions, units, geometry statistics, source hierarchy, licences, and any gaps.
- Do not add files to the repository until redistribution, fixture size, and attribution are understood.

### B1.2 — Finalize the challenge contract

- Confirm the exact required structure set based on the archive audit.
- Select and freeze the source release and exact files.
- Define fixture packaging and independent geometry checks.
- Decide whether current SVF can represent the dataset without production changes.

### B1.3 — Fixture preparation (separate approval)

Prepare a reproducible standalone fixture and verifier only after B1.2 is accepted. Keep raw upstream downloads out of Git unless their size and licence make that appropriate; document reproducible retrieval and retain only the required derived test data.

### B1.4 — SVF evaluation (separate approval)

Evaluate import/representation, rendering, orientation, structure visibility, and inspection. Report pass/fail/blocked outcomes with screenshots and exact command output. Any new SVF capability requires its own bounded design proposal after the limitation is demonstrated.

## Explicit non-goals

- Heartbeat or cardiac-cycle animation.
- Electrical conduction or electrophysiology.
- Pressure, flow, velocity, or computational fluid dynamics.
- Tissue mechanics or contraction simulation.
- Pathology, diagnosis, treatment, or patient-specific interpretation.
- Whole-body anatomy or a general medical atlas.
- Automatic segmentation, mesh reconstruction, or invented missing structures.
- Mava/Prismal integration.
- A general-purpose scene graph, editor, or anatomical authoring system.
